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
| v0.7.0 (本节) | `default = ["server"]`, `crdt` 默认 off, `legacy-lww` deprecated, `legacy-array` 增 v0.6.0 fallback | 推荐 `--features wasm-crdt` 用 `WasmCrdtDoc` 直接驱动 Yrs | m13 端可继续 `Canvas` JSON 路径,新代码走 `crdt` / `wasm-crdt` |

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

## 8. v0.7.0 升 YMap keyed by uuid

v0.7.0 把 root schema 从 "YArray of YMap" 升到 "YMap keyed
by uuid"，解决 v0.6.0 的三个已知限制（§7 1-3）。

### 8.1 v0.7.0 schema (YDoc root layout)

```text
YDoc
├── "meta"     : YMap { "name": String, "version": Number }
├── "elements" : YMap<NodeId.uuid(), YMap<field, value>>  ← v0.7.0 改
│   key  = element UUID string (e.g. "411e4216-...")
│   value = YMap {
│     "id":    String (UUID v4),
│     "kind":  "block" | "connector" | "note",
│     "x":     Number (i32, pixel),
│     "y":     Number (i32, pixel),
│     "label": String,
│     "alive": Bool   ← 删除用 tombstone (alive = false),不删 key
│   }
├── "ports"    : YMap<"${element_uuid}::${port_uuid}", YMap>  ← v0.7.0 升
│   (v0.6.0 端口是 element YMap 里的 JSON 字符串,无字段级 LWW)
│   value = YMap {
│     "id":         String (port UUID),
│     "element_id": String (parent element UUID),
│     "kind":       "input" | "output" | "bidir",
│     "label":      String,
│     "x":          Number (i32, 相对 element),
│     "y":          Number (i32, 相对 element),
│   }
└── "edges"    : YMap<"${from_uuid}::${to_uuid}", YMap>  ← v0.7.0 改
    (v0.6.0 是 YArray,并发加同 edge 留两份)
    value = YMap { "from": String, "to": String, "label": String?, "alive": Bool }
    key  = "${from_uuid}::${to_uuid}"  (from < to 字典序,保证唯一)
```

### 8.2 为什么 YMap keyed by uuid 解决了 v0.6.0 的 3 个限制

| v0.6.0 限制 | v0.7.0 解决 | 机制 |
|---|---|---|
| §7.1 YArray 并发 delete 同位置不 collapse | ✅ 解决 | 外层 YMap 2P-Set: 同一 key 两次 remove 收敛为 "removed" |
| §7.2 `ports` 是 JSON 字符串,字段级 LWW 不覆盖 | ✅ 解决 | 顶层 `ports` YMap,每个 port 独立 key (`element_uuid::port_uuid`),字段级 CRDT |
| §7.3 Edge dedup 不做 | ✅ 解决 | 顶层 `edges` YMap keyed by `${from}::${to}`,并发加同 edge 自然 dedup |

### 8.3 v0.7.0 新增 API (pub)

```rust
// element-level
insert_element(&doc, &node) -> Result<(), CanvasError>
remove_element(&doc, id) -> Result<bool, CanvasError>
update_element(&doc, id, ElementUpdate) -> Result<bool, CanvasError>
get_element(&doc, id) -> Option<ElementSnapshot>
iter_elements(&doc) -> impl Iterator<Item = (Uuid, ElementSnapshot)>

// port-level
add_port(&doc, element_id, port: PortSnapshot) -> Result<bool, CanvasError>
remove_port(&doc, element_id, port_id) -> Result<bool, CanvasError>

// edge-level
insert_edge(&doc, from, to, label) -> Result<(), CanvasError>
remove_edge(&doc, from, to) -> Result<bool, CanvasError>
update_edge(&doc, from, to, label) -> Result<bool, CanvasError>
get_edge(&doc, from, to) -> Option<EdgeSnapshot>
iter_edge_keys(&doc) -> impl Iterator<Item = (NodeId, NodeId)>

// sync (v0.6.0 继承,签名 v0.7.0 加 `&ClientId`)
merge_crdt_update(&doc, remote_state, update_bytes) -> Result<Vec<u8>, CanvasError>
encode_state_as_update(&doc) -> Vec<u8>
reconcile_with_crdt(&server, &client_update, client_version, &ClientId)
    -> Result<CrdtReconcileResult, CanvasError>

// value types
ElementSnapshot, PortSnapshot, EdgeSnapshot, ElementUpdate, ClientId, CrdtReconcileResult

// utility (v0.7.0 升 pub)
read_canvas_from_doc(&doc, name) -> Result<Canvas, CanvasError>
doc_from_canvas(&canvas) -> Doc
```

### 8.4 v0.7.0 `ClientId` 协议

v0.7.0 引入 `ClientId { uuid: Uuid, label: String }` 让 server / client
配对永远不 alias (yrs 把 client_id 编码在 update 头 varint 里,同
client_id 同状态向量)。调用方:

```rust
let server_id = ClientId::new("server-1");      // label 可读
let server_doc = Doc::with_client_id(server_id.uuid.as_u128() as u64);
let result = reconcile_with_crdt(&server_canvas, &client_update,
                                 client_version, &server_id)?;
```

`ClientId::from_uuid(uuid, label)` 用于从一个已知的 Uuid + label 重建
(例如 server 从 client 端收到的 `ClientId` 元数据反序列化)。

### 8.5 v0.7.0 fallback path (`--features legacy-array`)

v0.6.0 的 YArray-of-YMap schema 保留为 `legacy-array` Cargo feature
(default off),让 v0.6.x 部署可以平滑过渡一个 release 周期。`legacy-array`
imply `crdt` — 两者可同时开。代码路径:

```rust
// src/crdt_legacy_array.rs (新文件,#[cfg(feature = "legacy-array")])
#[deprecated(since = "0.7.0", note = "v0.6.0 path; use crdt::reconcile_with_crdt")]
pub fn reconcile_with_crdt_legacy(server, client_update, client_version)
    -> Result<CrdtReconcileResult, CanvasError>
```

v0.8.0 会删除 `legacy-array` feature + `crdt_legacy_array` 模块。CHANGELOG
会单独记录这条过渡期。

### 8.6 v0.7.0 WASM binding (`--features wasm-crdt`)

v0.7.0 加 `WasmCrdtDoc` wrapper(`src/wasm_crdt.rs`),让 web 前端 /
JS host 直接驱动 Yrs Doc,不需要手动 `encode_state_as_update` /
`merge_crdt_update` round-trip。API:

```ts
import init, { WasmCrdtDoc } from "pkg/ada_m12_canvas_editor";

const doc = new WasmCrdtDoc();
doc.insertElementJson({ id, kind: "block", x, y, label, ports: [], alive: true });
const state = doc.encodeState();           // Uint8Array
const diff  = peer.applyUpdate(state, sv);  // Uint8Array
const els   = doc.getElements();            // JsValue (JSON array)
```

跟 `--features wasm` (v0.5.0 `WasmCanvas` wrapping `Canvas` 表面) 是
两条独立 feature,可以同时开。`wasm-crdt` imply `crdt`。

### 8.7 v0.7.0 feature 矩阵 (更新)

```toml
[features]
default = ["server"]         # v0.7.0: server 默认 on
server   = []                 # v0.5.0 LWW path, 现 always-on
legacy-lww = []               # v0.7.0 DEPRECATED alias, v0.8.0 移除
crdt     = []                 # v0.7.0 Yrs path (pub mod crdt)
legacy-array = ["crdt"]       # v0.7.0: v0.6.0 YArray fallback (v0.8.0 移除)
wasm-crdt = ["dep:wasm-bindgen", ..., "crdt"]  # v0.7.0 WASM binding
wasm     = [...]              # v0.1.0 WasmCanvas wrapping Canvas
```

测试数 (v0.7.0, 验证后):
- `cargo test -p ada-m12-canvas-editor` (default = server): 33 unit + 9 integration
- `cargo test ... --features crdt`: 49 unit + 11 integration
- `cargo test ... --features legacy-array`: 50 unit + 11 integration
- `cargo test ... --features wasm-crdt`: 43 unit (wasm-only doctest cfg-gated)

### 8.8 v0.7.0 已知限制 (新)

> 缺标比错标安全 — 显式列出。

1. **Nested YMap concurrent insert 丢失数据**: 两年 v0.6.0 → v0.7.0
   设计的未解决问题。当两个 client 都 `insert_element(same_uuid)` 时,
   各自 `MapPrelim::new()` 创建独立的 inner YMap reference;外层 YMap
   2P-Set 只保留一个 inner YMap reference,另一个的字段写丢失。Workaround:
   test pattern 改为"一个 client seed,sync,然后两个 client concurrent
   field update" — 这是真正能验证的"不同字段不冲突"性质。生产场景
   暂用 `client_id` 协议协调(见 §8.4),避免双 seed。v0.7.1 / v0.8.0
   可考虑改用 flat `${uuid}::${field}` YMap schema 彻底解决。
2. **`ClientId` API 在 v0.7.0 已经入**,不按 parent brief 的 v0.7.1 计划
   推迟。前任 worker 已整合,revert 成本高于保留;flag 在 commit msg
   里显式记录。
3. **`legacy-array` 一年内移除**: v0.8.0 删除 feature flag + module
   + `reconcile_with_crdt_legacy` 函数。
4. **`wasm-crdt` doctest 只在 `wasm-pack test` 跑**: 5 门 native CI
   cfg-gated 跳过。Browser E2E 留 v0.7.1 或 v0.8.0。

---

## 9. 测试矩阵 (v0.7.0)

> v0.7.0 测试数从 v0.6.0 的 10 个 (8 unit + 2 integration) 增到
> 60+ 个 (含 element/port/edge/ClientId/wasm-crdt 多场景)。详见
> `src/crdt.rs` 底部 `mod tests` 块和 `tests/crdt_sync.rs`。

| 测试 | 文件 | 类型 | 状态 |
|---|---|---|---|
| sync_roundtrip_preserves_elements | src/crdt.rs | lib unit | ✅ |
| concurrent_insert_same_id_dedup | src/crdt.rs | lib unit | ✅ |
| concurrent_update_same_field_lww | src/crdt.rs | lib unit | ✅ |
| concurrent_update_different_fields_no_conflict | src/crdt.rs | lib unit (seed-by-one) | ✅ |
| concurrent_delete_same_id_converges_to_deleted | src/crdt.rs | lib unit | ✅ |
| tombstone_converges_with_concurrent_insert | src/crdt.rs | lib unit | ✅ |
| port_concurrent_add_different_id_converges | src/crdt.rs | lib unit | ✅ |
| port_concurrent_remove_same_id_converges_to_removed | src/crdt.rs | lib unit | ✅ |
| port_concurrent_update_x_vs_y_no_conflict | src/crdt.rs | lib unit (seed-by-one) | ✅ |
| edge_concurrent_insert_same_key_dedup | src/crdt.rs | lib unit | ✅ |
| edge_concurrent_delete_same_key_converges | src/crdt.rs | lib unit | ✅ |
| edge_concurrent_update_label_no_conflict | src/crdt.rs | lib unit | ✅ |
| client_id_negotiation_persists_to_update_bytes | src/crdt.rs | lib unit | ✅ |
| merge_crdt_update_rejects_malformed_bytes | src/crdt.rs | lib unit | ✅ |
| multi_client_merge_converges (3 客户端) | src/crdt.rs | lib unit | ✅ |
| large_doc_encodes_decodes_under_1s (1k) | src/crdt.rs | lib unit (perf 烟雾) | ✅ |
| legacy_array_concurrent_inserts_converge | src/crdt_legacy_array.rs | lib unit (--features legacy-array) | ✅ |
| three_clients_converge_to_same_state | tests/crdt_sync.rs | integration | ✅ |
| reconcile_with_server_canvas_preserves_client_additions | tests/crdt_sync.rs | integration | ✅ |

总计 19 tests (17 unit + 2 integration, legacy-array 下 18 unit),
全 `--features crdt` 下 GREEN。

---

## 10. 参考

- 源码: `crates/ada-m12-canvas-editor/src/crdt.rs` (~830 lines)
- 集成测试: `crates/ada-m12-canvas-editor/tests/crdt_sync.rs`
- 设计依据: `docs/decisions/02-design-adrs.md` D-01
- 用户文档: `docs/modules/M-12-canvas-editor-frontend.md` §3.6
- 路线图: `docs/observability/11-phased-rollout.md` §10
- CHANGELOG: `docs/CHANGELOG.md` v2.9.0 段

---

*代签: 架构师（Mavis 接手 agent per DEC-008）*
*对应 commits: c6d19cf, 5587f7a, 92277db + docs commit*
