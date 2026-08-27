# CRDT.md — M-12 v0.6.0 Yrs Integration

> **M-12 canvas editor**  v0.6.0 CRDT (Yrs) sync 集成说明。
> 与 `docs/modules/M-12-canvas-editor-frontend.md` §3.6 配套阅读。
> 设计依据: `docs/decisions/02-design-adrs.md` D-01 (CRDT/Yrs 决策)。

---

## 1. Why CRDT?

v0.5.0 用 3-way LWW merge (`crates/ada-m12-canvas-editor/src/server_recon.rs`),
server 在 conflict 时 wins, client 编辑被丢弃。这在 collaborative
editing 场景下导致 **concurrent client moves silently lost** —
用户体验差, 不符合 m11 rbac-collab 已经在做的多 actor 编辑预期。

v0.6.0 升级到 CRDT (Yjs-compatible) 解决:

- 任何数量的并发客户端在断开连接后 merge 时, **deterministic convergence**
- 字段级 LWW: 并发编辑同一节点的不同字段 → 全部保留
- 单一 replica 状态 = bytes; wire format 简单 (varint + lib0)

---

## 2. Library choice: Yrs 0.18

| 维度 | 选择 |
|---|---|
| 库 | [yrs 0.18](https://github.com/y-crdt/y-crdt/) (Yjs Rust port) |
| License | MIT |
| Wire format | 兼容 Yjs (`y-protocols/sync`), 已有 JS/TS/Python/Go 客户端 |
| WASM | yrs 0.18 不带 `wasm` feature; 浏览器内跑需 `yrs-wasm` binding (v0.7.0 评估) |
| 离线 cache | 0.18.8 已缓存到 `E:/DevCache/cargo/registry/cache/yrs-0.18.8.crate` (D:/Ada 无网) |

**为什么不是 Automerge / 自研**: 1 多年 Yjs 生态成熟, Yrs 与
Yjs binary 兼容, 现有 m11 前端用 `yjs` 包可与 m12 yrs 后端直接
对话; 自研成本 / 风险远高于依赖成熟库。

---

## 3. 数据结构 (YDoc root layout)

```text
YDoc
├── "meta"     : YMap { "name": String, "version": Number }
├── "elements" : YArray [
│   YMap {
│     "id":    String (UUID v4),
│     "kind":  "block" | "connector" | "note",
│     "x":     Number (i32, pixel),
│     "y":     Number (i32, pixel),
│     "label": String,
│     "ports": String (JSON array of port names; v0.7.0 升 YArray),
│     "alive": Bool
│   },
│   ...
│ ]
└── "edges"    : YArray [
    YMap { "from": String (UUID), "to": String (UUID), "alive": Bool },
    ...
]
```

**为什么 element 是 YMap (不是 YArray 的一项)**: 让 concurrent
move 同一 node 走 Yjs YMap 字段级 LWW, 而不是 YArray position
conflict (YArray position conflict 是"slot 谁的更新更晚",
会丢失数据)。

**为什么用 YArray of YMap 而不是顶层 YMap keyed by uuid**:
v0.6.0 选择前者因为 list 顺序是 canvas 渲染的隐含 z-order; v0.7.0
考虑加一个旁路 YMap keyed by uuid 加速 lookup, 不替换主 schema。

---

## 4. API surface (pub mod `crdt`, gated by `--features crdt`)

```rust
use ada_m12_canvas_editor::{
    encode_state_as_update, merge_crdt_update, reconcile_with_crdt, Canvas,
};

// 第一步同步 (full state)
let snapshot = encode_state_as_update(&server_doc);

// 客户端用 snapshot 初始化本地 YDoc (apply_update)
client_doc.transact_mut().apply_update(
    yrs::Update::decode_v1(&snapshot).unwrap()
);

// 后续增量 (state-vector diff dance)
let sv = client_doc.transact().state_vector().encode_v1();
let server_update = encode_state_as_update(&server_doc);
let diff_for_client = merge_crdt_update(&client_doc, &sv, &server_update)?;
// 客户端 apply diff_for_client 后 = 同步

// 端到端 reconcile (server Canvas + client update → merged state)
let result = reconcile_with_crdt(&server_canvas, &client_update_bytes, client_version)?;
// result.merged_state: Vec<u8> = 新的 server snapshot (apply 到 client)
// result.new_version: u64 = max(server.version, client_version) + 1
```

`merge_crdt_update` 错误路径返回 `CanvasError::BackendError`
(malformed update bytes / state vector); 这是 wire 协议问题,
不暴露给 UI 层。

---

## 5. v0.5.0 → v0.6.0 迁移路径

| 阶段 | server side | client side | 备注 |
|---|---|---|---|
| v0.5.0 | `Canvas` + `server_recon` (LWW), `--features server` | Canvas wasm snapshot | m13 endpoint 接受 `Canvas` JSON |
| v0.6.0 本段 | `Canvas` + `crdt` (Yrs), `--features crdt` | Yrs / Yjs client | m13 endpoint 透传 yrs update bytes |
| v0.6.x 过渡 | **双开**: `--features "crdt,legacy-lww"` 让两边都可用 | 双 client 路径 (推荐 Yrs 优先) | root 收尾时验证 m13 双向兼容 |
| v0.7.0 (计划) | `--features legacy-lww` 改默认 off; `--features crdt` 默认 on | 推荐 Yrs only | v0.7.0 移除 LWW 路径 |

`server` feature 在 v0.6.0 仍保留 (alias), 让 `m13/tests/reconcile_smoke.rs`
等 v0.5.0 集成测试不破坏。新代码应该用 `legacy-lww` + `crdt`。

---

## 6. Fallback 开关 (Cargo features)

```toml
[features]
default = []              # 5 门 CI 默认路径 (无 yrs 编译负担)
server   = []             # v0.5.0 兼容 alias → legacy-lww
legacy-lww = []           # v0.5.0 LWW path (server_recon.rs)
crdt     = []             # v0.6.0 Yrs path (crdt.rs)
wasm     = [...]          # v0.1.0 wasm-bindgen bindings (independent)
```

- `cargo test --workspace` 走 `default` (无 yrs 编译); 0 网络/编译时间开销
- `cargo test -p ada-m12-canvas-editor --features crdt` 跑 CRDT 套件 (本段)
- `cargo test -p ada-m12-canvas-editor --features legacy-lww` 跑 v0.5.0 LWW 套件 (回归)
- `cargo test -p ada-m12-canvas-editor --features "crdt,legacy-lww"` 同时跑两套 (收尾验证)

---

## 7. 已知限制 (v0.6.0)

> 缺标比错标安全 — 显式列出。详见 `docs/CHANGELOG.md` v2.9.0 段的 "已知缺口"。

1. **Yjs YArray 并发 delete 同位置不 collapse**: Yjs 已知行为,
   两 replica 同时 `remove(idx)` 后 merge 会留两个 tombstone。
   测试用单向传播覆盖; 真双向收敛留 v0.7.0 (改 YMap keyed by uuid)。
2. **`ports` 字段是 JSON 字符串**: 不是嵌套 YArray, 字段级
   CRDT 不覆盖 ports 子结构。v0.7.0 升 YArray。
3. **Edge dedup 不做**: YArray 没内置 dedup, 两 client 并发加
   同 edge 会留两个; v0.7.0 用 YMap keyed by `${from}->${to}`。
4. **`yrs` 0.18.8 offline cache**: 不能 `cargo update` 拉新版
   (D:/Ada 无网); 升级需手动 ship cache + `cargo update -p yrs`。
5. **WASM browser E2E 没测**: yrs 0.18 不带 `wasm` feature,
   浏览器内跑测留 v0.7.0 (`yrs-wasm` binding) 或 v0.6.x patch
   (若 community 推出 stable wasm feature)。
6. **`reconcile_with_crdt` 不知道 client 的 client_id**: v0.6.0
   假设 server / client 各自 Doc::new() 拿到不同 client_id
   (yrs 内部 rand); 显式 client_id 协商留 v0.7.0。
7. **`read_canvas_from_doc` 是 `pub(crate)`**: m13 cross-crate
   consumer 不能直接用, 留 m13 集成测试 v0.6.1 加 pub re-export。

---

## 8. 测试矩阵

| 测试 | 文件 | 类型 | 状态 |
|---|---|---|---|
| sync_roundtrip_preserves_elements | src/crdt.rs | lib unit | ✅ |
| concurrent_inserts_converge | src/crdt.rs | lib unit | ✅ |
| concurrent_move_converges_via_lww | src/crdt.rs | lib unit | ✅ |
| concurrent_delete_converges (单向) | src/crdt.rs | lib unit | ✅ |
| multi_client_merge_converges (3 客户端) | src/crdt.rs | lib unit | ✅ |
| large_doc_encodes_decodes_under_1s (1k) | src/crdt.rs | lib unit (perf 烟雾) | ✅ |
| reconcile_with_crdt_merges_client_edit | src/crdt.rs | lib unit | ✅ |
| merge_crdt_update_rejects_malformed_bytes | src/crdt.rs | lib unit (错误路径) | ✅ |
| three_clients_converge_to_same_state | tests/crdt_sync.rs | integration | ✅ |
| reconcile_with_server_canvas_preserves_client_additions | tests/crdt_sync.rs | integration | ✅ |

总计 10 tests (8 unit + 2 integration), 全 `--features crdt` 下 GREEN。

---

## 9. 参考

- 源码: `crates/ada-m12-canvas-editor/src/crdt.rs` (~830 lines)
- 集成测试: `crates/ada-m12-canvas-editor/tests/crdt_sync.rs`
- 设计依据: `docs/decisions/02-design-adrs.md` D-01
- 用户文档: `docs/modules/M-12-canvas-editor-frontend.md` §3.6
- 路线图: `docs/observability/11-phased-rollout.md` §10
- CHANGELOG: `docs/CHANGELOG.md` v2.9.0 段

---

*代签: 架构师（Mavis 接手 agent per DEC-008）*
*对应 commits: c6d19cf, 5587f7a, 92277db + docs commit*
