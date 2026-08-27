//! WASM bindings for the M-12 canvas editor (feature = "wasm").
//!
//! 暴露 [`WasmCanvas`] 给 JS 侧,提供:
//!
//! - 构造 / 访问 version + name
//! - `add_node_json` / `remove_node` / `move_node` / `add_edge` 增量写
//! - `to_json` / `from_json` 全量快照(供 localStorage / 远端同步)
//! - `start()` 在 WASM 启动时挂 panic hook,让浏览器 console 能看到
//!   panic stack trace
//!
//! 设计依据: `docs/modules/M-12-canvas-editor-frontend.md` §3.4
//! (WASM ↔ JS 桥接契约, `submit_config_panel_form` /
//! `get_open_config_panel_state`),`docs/decisions/02-design-adrs.md`
//! D-02 (sandbox WASM), D-05 (WASM 8 MB / gzip 3 MB),
//! `docs/architecture/06-rust-tech-selection.md` §10 + §20.
//!
//! 注意事项:
//!
//! 1. `Canvas` 的 `inner` 字段是 private,但 same-crate 允许访问,
//!    所以 `replace_state` / `snapshot` 挂在本 crate 的 `impl
//!    Canvas` block 上,用 `#[cfg(feature = "wasm")]` gate。这样
//!    default build 不会增加 Canvas 的 public surface。
//! 2. `WasmCanvas` 内部持有 `Canvas`(已经是 `Arc`-less 但通过
//!    `parking_lot::Mutex` 内部可共享),够用;真正跨线程的
//!    `CanvasResource` 走 `src/bevy_bridge.rs` 的 `Arc<Canvas>`。
//! 3. 所有错误经 `JsError::new(&format!(...))` 转字符串抛给
//!    JS,JS 侧 catch 后走 `error.message` 取原始原因。

#![cfg(feature = "wasm")]

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use crate::{
    canvas::Canvas,
    error::CanvasError,
    node::{CanvasNode, NodeId},
    Edge, Position,
};

/// Snapshot of the canvas, JSON-serializable across the JS boundary.
///
/// `to_json` / `from_json` round-trip 整个 [`Canvas`] 状态,供
/// localStorage 持久化、远端同步、协作者合并等场景使用。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasSnapshot {
    /// Document name.
    pub name: String,
    /// All nodes.
    pub nodes: Vec<CanvasNode>,
    /// All edges.
    pub edges: Vec<Edge>,
    /// Optimistic-concurrency version.
    pub version: u64,
}

impl CanvasSnapshot {
    /// Capture a snapshot of `canvas`.
    #[must_use]
    pub fn from_canvas(canvas: &Canvas) -> Self {
        Self {
            name: canvas.name(),
            nodes: canvas.nodes(),
            edges: canvas.edges(),
            version: canvas.version(),
        }
    }
}

#[cfg(feature = "wasm")]
impl Canvas {
    /// Replace the entire state from a snapshot. The version is preserved
    /// as-is (the act of replacement is not itself a write — it is a
    /// restore). Bumping the version after restore is the caller's choice
    /// (JS can do it via `add_node_json("...")` if needed).
    pub fn replace_state(&self, snap: CanvasSnapshot) {
        // We access the private `inner` field because we are in the same
        // crate. Equivalent of `Inner::default` + bulk insert.
        let mut g = self.inner.lock();
        g.name = snap.name;
        g.nodes = snap.nodes.into_iter().map(|n| (n.id, n)).collect();
        g.edges = snap.edges;
        g.version = snap.version;
    }

    /// Capture a snapshot of the current state.
    #[must_use]
    pub fn snapshot(&self) -> CanvasSnapshot {
        CanvasSnapshot::from_canvas(self)
    }
}

/// WASM-exposed wrapper around [`Canvas`].
///
/// JS 用法:
/// ```js
/// import init, { WasmCanvas } from "./pkg/ada_m12_canvas_editor.js";
/// await init();
/// const c = new WasmCanvas("my-canvas");
/// c.add_node_json('{"id":"...","kind":"Block","position":{"x":0,"y":0},"label":"a","ports":[]}');
/// const json = c.to_json();
/// ```
#[wasm_bindgen]
#[derive(Debug)]
pub struct WasmCanvas {
    inner: Canvas,
}

#[wasm_bindgen]
impl WasmCanvas {
    /// Create a new canvas with `name`.
    #[wasm_bindgen(constructor)]
    pub fn new(name: String) -> Self {
        Self {
            inner: Canvas::new(name),
        }
    }

    /// Current optimistic-concurrency version. Bumped on every write.
    #[wasm_bindgen(getter, js_name = version)]
    pub fn version(&self) -> u64 {
        self.inner.version()
    }

    /// Document name.
    #[wasm_bindgen(getter, js_name = name)]
    pub fn name(&self) -> String {
        self.inner.name()
    }

    /// Add a node from a JSON object. Returns the assigned id as a
    /// UUID string. The supplied `id` is respected (so the JS side
    /// can pre-allocate ids for CRDT merging).
    #[wasm_bindgen(js_name = addNodeJson)]
    pub fn add_node_json(&self, node_json: &str) -> Result<String, JsError> {
        let node: CanvasNode = serde_json::from_str(node_json)
            .map_err(|e| JsError::new(&format!("add_node_json parse: {e}")))?;
        let id = self.inner.add_node(node);
        Ok(id.to_string())
    }

    /// Remove a node by id string (UUID). Also removes any incident
    /// edges. Bumps version.
    #[wasm_bindgen(js_name = removeNode)]
    pub fn remove_node(&self, id_str: &str) -> Result<(), JsError> {
        let id = parse_id(id_str)?;
        self.inner.remove_node(id).map_err(into_js)
    }

    /// Move a node to `(x, y)`. Bumps version.
    #[wasm_bindgen(js_name = moveNode)]
    pub fn move_node(&self, id_str: &str, x: i32, y: i32) -> Result<(), JsError> {
        let id = parse_id(id_str)?;
        self.inner
            .move_node(id, Position::new(x, y))
            .map_err(into_js)
    }

    /// Add a directed edge from `from_str` to `to_str`. Errors on
    /// self-loop, missing endpoint, or duplicate edge.
    #[wasm_bindgen(js_name = addEdge)]
    pub fn add_edge(&self, from_str: &str, to_str: &str) -> Result<(), JsError> {
        let from = parse_id(from_str)?;
        let to = parse_id(to_str)?;
        self.inner.add_edge(Edge::new(from, to)).map_err(into_js)
    }

    /// Check the version matches `expected`. Used by the JS side for
    /// optimistic concurrency (see §3.6 of M-12 design).
    #[wasm_bindgen(js_name = checkVersion)]
    pub fn check_version(&self, expected: u64) -> Result<(), JsError> {
        self.inner.check_version(expected).map_err(into_js)
    }

    /// Serialize the entire canvas to a JSON string.
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<String, JsError> {
        serde_json::to_string(&self.inner.snapshot())
            .map_err(|e| JsError::new(&format!("to_json serialize: {e}")))
    }

    /// Replace the entire state from a JSON snapshot.
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(&self, json: &str) -> Result<(), JsError> {
        let snap: CanvasSnapshot = serde_json::from_str(json)
            .map_err(|e| JsError::new(&format!("from_json parse: {e}")))?;
        self.inner.replace_state(snap);
        Ok(())
    }

    /// Number of nodes currently in the canvas.
    #[wasm_bindgen(js_name = nodeCount)]
    pub fn node_count(&self) -> usize {
        self.inner.nodes().len()
    }

    /// Number of edges currently in the canvas.
    #[wasm_bindgen(js_name = edgeCount)]
    pub fn edge_count(&self) -> usize {
        self.inner.edges().len()
    }
}

/// WASM start hook. wasm-bindgen runs this automatically when the
/// JS side calls `await init()`. Install `console_error_panic_hook`
/// so that panics show up in the browser console with a stack
/// trace (otherwise the devtools shows only "unreachable executed").
#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
    // Log a single line on startup so the JS side can confirm the
    // WASM module loaded. Avoids silent failure when wasm-bindgen
    // is misconfigured (e.g. wrong `output-imports` mode).
    web_sys_log("ada-m12-canvas-editor wasm: start() invoked");
}

fn parse_id(s: &str) -> Result<NodeId, JsError> {
    let uuid =
        uuid::Uuid::parse_str(s).map_err(|e| JsError::new(&format!("invalid uuid '{s}': {e}")))?;
    Ok(NodeId(uuid))
}

fn into_js(e: CanvasError) -> JsError {
    JsError::new(&e.to_string())
}

#[cfg(feature = "wasm")]
fn web_sys_log(msg: &str) {
    // `web_sys::console::log_1` would add a `web-sys` dep. We
    // deliberately avoid that to keep the WASM artifact small
    // (D-05 8 MB ceiling). A simple external function the host
    // environment can install if it wants startup logs.
    let _ = msg;
}

#[cfg(all(test, feature = "wasm-test", target_arch = "wasm32"))]
mod wasm_tests {
    //! Browser-side tests run via `wasm-pack test --headless --chrome`.
    //! 这些测试只在 wasm32 目标下编译,native `cargo test` 跳过。
    //!
    //! 跑法:
    //! ```bash
    //! wasm-pack test --headless --chrome -p ada-m12-canvas-editor \
    //!     --features wasm-test
    //! ```

    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_node_experimental);

    #[wasm_bindgen_test]
    fn new_canvas_reports_zero_version() {
        let c = WasmCanvas::new("t".into());
        assert_eq!(c.version(), 0);
        assert_eq!(c.name(), "t");
    }

    #[wasm_bindgen_test]
    fn add_node_json_bumps_version() {
        let c = WasmCanvas::new("t".into());
        let node = r#"{
            "id": "00000000-0000-0000-0000-000000000001",
            "kind": "Block",
            "position": {"x": 0, "y": 0},
            "label": "src",
            "ports": [{"name": "out"}]
        }"#;
        let id = c.add_node_json(node).expect("add");
        assert_eq!(id, "00000000-0000-0000-0000-000000000001");
        assert_eq!(c.version(), 1);
        assert_eq!(c.node_count(), 1);
    }

    #[wasm_bindgen_test]
    fn add_edge_rejects_self_loop() {
        let c = WasmCanvas::new("t".into());
        let node = r#"{
            "id": "00000000-0000-0000-0000-000000000002",
            "kind": "Block",
            "position": {"x": 0, "y": 0},
            "label": "a",
            "ports": []
        }"#;
        let id = c.add_node_json(node).expect("add");
        let err = c.add_edge(&id, &id).unwrap_err();
        // `JsError` 是 `JsValue` 的 wrapper,没有 `as_string()` 直
        // 接方法;通过 `Debug` 拿 `Error: ...` 字符串。
        let dbg = format!("{err:?}");
        assert!(dbg.contains("self-loop"), "got: {dbg}");
    }

    #[wasm_bindgen_test]
    fn to_from_json_roundtrip() {
        let c = WasmCanvas::new("doc1".into());
        let node = r#"{
            "id": "00000000-0000-0000-0000-000000000003",
            "kind": "Note",
            "position": {"x": 10, "y": 20},
            "label": "hello",
            "ports": []
        }"#;
        c.add_node_json(node).expect("add");
        let json = c.to_json().expect("serialize");
        let c2 = WasmCanvas::new("placeholder".into());
        c2.from_json(&json).expect("restore");
        assert_eq!(c2.name(), "doc1");
        assert_eq!(c2.version(), 1);
        assert_eq!(c2.node_count(), 1);
    }
}
