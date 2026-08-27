//! M-12: Canvas editor. 3 `NodeKind` (Block / Connector / Note),
//! `Canvas` document, edit history with undo/redo.
//!
//! ## v0.1.0 scope (B6)
//!
//! Minimum skeleton for the canvas editor defined in
//! [`DOC-MOD-012`](../docs/modules/M-12-canvas-editor.md). The
//! v0.1.0 surface is:
//!
//! - [`NodeKind`] — three kinds (`Block / Connector / Note`)
//! - [`CanvasNode`] — id, kind, position, label, ports
//! - [`Edge`] — from / to node ids
//! - [`Canvas`] — in-memory document with
//!   `add_node / remove_node / move_node / add_edge`
//! - [`EditOp`] / [`EditHistory`] — undo/redo stack
//! - 5-variant [`CanvasError`] (`NodeNotFound`,
//!   `VersionConflict`, `InvalidEdge`, `HistoryEmpty`,
//!   `BackendError`)
//!
//! See `docs/modules/M-12-canvas-editor.md` (DOC-MOD-012) for
//! the full design.
//!
//! 関連 IPA フェーズ: 22-52 (基本設計/詳細設計), 53-58 (実装), 59-95 (試験)
//! 設計書: docs/modules/M-12-canvas-editor.md (DOC-MOD-012)
//!
//! ## Feature flags
//!
//! - `default = []` — 纯 Rust,无 WASM / Bevy 编译负担。`cargo
//!   test --workspace` 与 CI 5 门(检查/测试/clippy/fmt/wasm-pack)
//!   走 default 路径。
//! - `wasm` — 拉 `wasm-bindgen` / `js-sys` /
//!   `serde-wasm-bindgen` / `console_error_panic_hook`,启用
//!   [`wasm::WasmCanvas`] JS 绑定(见 `src/wasm.rs`)。
//! - `bevy` — 拉 `bevy_ecs` 0.14 + `bevy_app` 0.14,启用
//!   [`bevy_plugin::CanvasPlugin`] Bevy 插件(见
//!   `src/bevy_plugin.rs` + `src/bevy_bridge.rs`)。
//! - `full` — 同时启用 wasm + bevy,产出最大体积的 WASM。
//! - `wasm-test` — 在 wasm feature 之上加 `wasm-bindgen-test`,
//!   供 `wasm-pack test --headless --chrome` 跑浏览器内测试。
//! - `server` (M-12 v0.5.0) — 启用 `pub mod server_recon` 提供
//!   3-way merge (LWW, server-authoritative) 协议实现。`default
//!   off`,只为 m13 ↔ m12 集成测试 + 远端 reconcile 场景使用。
//!
//! 设计依据: `docs/decisions/02-design-adrs.md` D-02 (sandbox
//! WASM), D-04 (Bevy 0.14 stable), D-05 (WASM 8 MB / gzip 3 MB),
//! `docs/modules/M-12-canvas-editor-frontend.md` §3.4 (WASM ↔
//! JS 桥接契约), `docs/architecture/06-rust-tech-selection.md`
//! §10 (Bevy 0.14 + bevy_egui) + §20 (WASM size 风险对策).

#![allow(missing_docs)]
#![allow(rust_2018_idioms)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

mod canvas;
mod error;
mod history;
mod node;

#[cfg(feature = "bevy")]
mod bevy_bridge;
#[cfg(feature = "bevy")]
mod bevy_plugin;
#[cfg(feature = "bevy_egui")]
mod egui_integration;
/// M-12 v0.5.0 server-side reconciliation. Only compiled with
/// `--features server` so the default 5-gate CI path doesn't
/// pull the optional integration surface. See `src/server_recon.rs`
/// for the algorithm. Made `pub mod` (not just `pub use`) so
/// cross-crate integration tests (e.g. `m13/tests/reconcile_smoke.rs`)
/// can address the types as
/// `ada_m12_canvas_editor::server_recon::reconcile_canvas_state`.
#[cfg(feature = "server")]
pub mod server_recon;
#[cfg(feature = "wasm")]
mod wasm;

pub use canvas::{Canvas, Edge};
pub use error::{CanvasError, Result};
pub use history::{EditHistory, EditOp};
pub use node::{CanvasNode, NodeId, NodeKind, Port, Position};

/// WASM 绑定模块。仅在 `--features wasm` 时存在。
#[cfg(feature = "wasm")]
pub mod wasm_bindings {
    pub use crate::wasm::{CanvasSnapshot, WasmCanvas};
}

/// Bevy 插件模块。仅在 `--features bevy` 时存在。
#[cfg(feature = "bevy")]
pub mod bevy_integration {
    pub use crate::bevy_bridge::sync_canvas_system;
    pub use crate::bevy_plugin::{
        CanvasNodeComp, CanvasPlugin, CanvasPositionComp, CanvasResource,
    };
}

/// bevy_egui 集成模块 — M-12 v0.3.0 新增。仅在
/// `--features bevy_egui` 时存在(native-only,不会拖入 WASM
/// 体积)。提供 inspector 面板 + 拖拽事件 + ECS↔Canvas
/// 双向 sync。
///
/// 包含:
/// - [`CanvasInspectorPlugin`] — Bevy Plugin
/// - [`NodeInspectorState`] — 选中节点 Resource
/// - [`NodeDragState`] — 拖拽状态 Resource
/// - [`begin_drag`]/[`update_drag`]/[`end_drag`] — host-driven
///   拖拽 API
/// - [`node_inspector_system`] / [`sync_ecs_to_canvas_system`] —
///   注册到 Plugin 的 ECS 系统
#[cfg(feature = "bevy_egui")]
pub use egui_integration::{
    begin_drag, end_drag, node_inspector_system, sync_ecs_to_canvas_system, update_drag,
    CanvasInspectorPlugin, NodeDragState, NodeInspectorState,
};

/// M-12 v0.5.0 server-side reconciliation public surface.
/// Only compiled with `--features server`.
///
/// 包含:
/// - [`reconcile_canvas_state`] — 3-way merge (LWW, server wins)
/// - [`ReconcileResult`] — 合并结果 (merged canvas + win lists)
#[cfg(feature = "server")]
pub use server_recon::{reconcile_canvas_state, ReconcileResult};

/// Crate version, taken from `CARGO_PKG_VERSION` (single workspace
/// version per D-09).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Crate name, taken from `CARGO_PKG_NAME`.
pub const NAME: &str = env!("CARGO_PKG_NAME");

/// `nerve`-layer string tag (仿生モデル 4 層分類, see
/// [`DOC-ARCH-001`](https://example.invalid/docs/architecture/00-anatomy-model.md)).
pub const LAYER: &str = "nerve";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_not_empty() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn name_not_empty() {
        assert!(!NAME.is_empty());
    }

    #[test]
    fn layer_is_known() {
        assert!(
            ["skeleton", "blood", "nerve", "muscle", "shared"].contains(&LAYER),
            "Unknown layer: {LAYER}"
        );
    }
}
