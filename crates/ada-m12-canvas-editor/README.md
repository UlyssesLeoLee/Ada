# ada-m12-canvas-editor

M-12: Canvas editor. 3 `NodeKind` (`Block / Connector / Note`),
`Canvas` document with optimistic-concurrency version, and a
linear `EditHistory` with undo/redo.

WASM 编译目标: `wasm32-unknown-unknown`,供 Bevy 0.14 + HTML Overlay
前端通过 JS 调用(per `docs/modules/M-12-canvas-editor-frontend.md`
§3.4, `docs/decisions/02-design-adrs.md` D-02 / D-04 / D-05)。

## v0.1.0 status

Skeleton. Real CRDT collaboration (yrs/Yjs) lands in B7+;
optimistic-concurrency versioning is provided as the
single-writer fallback.

## v0.1.0 surface (default features)

- `NodeKind` — `Block | Connector | Note`
- `CanvasNode` — id, kind, position, label, ports
- `Position` — 2-D integer coordinates
- `Port` — name (input / output / ...)
- `Edge` — directed `from -> to`
- `Canvas` — `add_node / remove_node / move_node / add_edge / check_version / get_node`
- `EditOp` — `InsertNode | RemoveNode | MoveNode | AddEdge`
- `EditHistory` — linear undo/redo with branch reset
- 5-variant `CanvasError`
- ~30 unit tests + 4 integration tests

## Feature flags

| Feature | 引入的 deps | 暴露的 surface |
|---|---|---|
| `default` (空) | 仅 core deps | `Canvas` / `EditHistory` / `EditOp` / `CanvasNode` / `Edge` / ... |
| `wasm` | `wasm-bindgen` 0.2 + `js-sys` 0.3 + `serde-wasm-bindgen` 0.6 + `console_error_panic_hook` 0.1 | `wasm_bindings::WasmCanvas` + `wasm_bindings::CanvasSnapshot` |
| `bevy` | `bevy_ecs` 0.14 + `bevy_app` 0.14 (`default-features = false`) | `bevy_integration::CanvasPlugin` + `CanvasResource` + `CanvasNodeComp` + `CanvasPositionComp` + `sync_canvas_system` |
| `full` | `wasm` + `bevy` 全部 | 上述全部 |
| `wasm-test` | `wasm` + `wasm-bindgen-test` 0.3 | `wasm_bindings::WasmCanvas` 4 个浏览器内单元测试 |

设计依据:
- D-02 sandbox WASM
- D-04 Bevy 0.14 stable
- D-05 WASM 8 MB / gzip 3 MB
- `docs/modules/M-12-canvas-editor-frontend.md` §3.4 (WASM ↔ JS 桥接契约)

## 构建 (native)

```bash
# 5 门 CI 必跑
cargo check --workspace --all-targets
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

## 构建 (WASM)

```bash
# 1) 安装工具链 (host 首次)
rustup target add wasm32-unknown-unknown
cargo install wasm-pack      # 或用 rustwasm 官方脚本

# 2) 一键构建 + 尺寸校验 (per D-05)
./wasm/build.sh              # 默认: target=web, release
./wasm/build.sh --features full

# 3) 浏览器内单元测试 (可选)
./wasm/test.sh               # 默认: headless chrome
./wasm/test.sh --node        # 改用 node
```

详细说明见 `wasm/README.md`。
