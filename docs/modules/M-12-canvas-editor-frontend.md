# M-12 前端画布编辑器（Canvas Editor Frontend）

> **ドキュメントID**：DOC-MOD-012
> **文書分類**：モジュール別設計書
> **バージョン**：v1.1.0
> **制定日**：2026-08-18
> **最終更新日**：2026-08-19
> **作成者**：Ada プロジェクトチーム
> **レビュー**：TBD
> **承認**：TBD
> **上位文書**：`docs/legacy/requirements.md`（DOC-REQ-001）、`docs/legacy/basic-design.md`（DOC-BSC-001）、`docs/legacy/detailed-design.md`（DOC-DTL-001）、`docs/architecture/01-tech-stack.md`（DOC-ARCH-002）
> **下位文書**：`docs/tests/UT-design.md`（DOC-TST-001 §12）
> **関連文書**：`docs/modules/M-11`（DOC-MOD-011）
> **適用 IPA 標準**：
> - IPA「共通フレーム2018」(SLCP-JCF2018) 第 6 章
> - IPA「非機能要求グレード2018」
> **機密区分**：社内
> **言語**：中文（简体）

---

## 改訂履歴

| バージョン | 日付 | 変更内容 | 起草 | レビュー | 承認 |
|---|---|---|---|---|---|
| v1.0.0 | 2026-08-18 | 初版制定 | Ada プロジェクトチーム | TBD | TBD |
| v1.1.0 | 2026-08-19 | IPA 準拠メタデータ追加、NF タグ付与 | Ada プロジェクトチーム | TBD | TBD |

---

## 目次

1. 需求来源（要件定義書）
2. 基本设计（基本設計書）
3. 詳細设计（詳細設計書）
4. 验收要点
5. 用語集
6. 参考文献

---

## 1. 需求来源（要件定義書）

### 1.1 涉及 F-IDs

- **F-01** 无限画布编辑器

### 1.2 关联用例

U-05 可视化调试与追溯（涉及画布交互）

### 1.3 非功能需求

- 7.2 性能：单画布 1,000 节点、5,000 条连线的流畅渲染与编辑（前端帧率不低于 30fps）
- 7.3 运用保守性：插件热更新、WASM 包体积控制

## 2. 基本设计（基本設計書）

### 2.1 混合渲染架构总览

前端采用 **Bevy（ECS 游戏引擎）+ bevy_egui（即时模式 GUI）渲染画布本体，HTML Overlay（原生 DOM）承载中文输入密集的表单**，两套渲染管线在同一浏览器页面内并存，通过共享的视口变换状态保持视觉对齐。

**选型理由**：

- 前后端统一 Rust 语言栈，`CanvasDefinition`/`NJson` 等核心数据结构可直接前后端共享，免去 TypeScript 类型重复定义与 codegen 维护成本
- Bevy 的 ECS 架构与 GPU 渲染管线天然契合"节点=实体、连线=关系"的画布模型，在 1000+ 节点场景下的渲染性能优于 DOM/SVG 方案
- bevy_egui 提供即时模式 GUI，适合工具栏、右键菜单、简单参数面板等轻量交互
- 仍以"浏览器打开即用"的网页形式交付，满足 F-09 免安装要求

**混合渲染的关键决策——中文输入法（IME）风险应对**：

| UI 区域 | 渲染技术 | 理由 |
|---|---|---|
| 画布本体（节点卡片、连线、缩放平移、框选） | Bevy + bevy_egui | 高性能渲染需求为主，文本展示为主，非密集编辑 |
| 工具栏、快捷操作菜单、简单数值/开关参数 | bevy_egui | 交互简单，无中文长文本输入场景 |
| 节点详细配置表单（含中文命名、映射规则、表达式编辑器、JSON 树查看） | **HTML Overlay**（原生 `<input>`/`<textarea>`，通过 `web-sys` 与 Bevy 画布坐标同步定位） | 依赖浏览器原生 IME 支持，保证中文输入体验；仅在打开配置面板时挂载，关闭时卸载，不常驻増加 DOM 开销 |
| 调试面板（执行日志、数据快照 JSON 展示） | HTML Overlay | 大段文本的选中/复制/搜索依赖浏览器原生能力更可靠 |

HTML Overlay 与 Bevy Canvas 的坐标同步机制：Overlay 面板锚定于触发它的节点在画布坐标系下的屏幕投影位置，画布平移/缩放时通过 Bevy 侧广播视口变换矩阵，Overlay 层（普通 DOM，`position: absolute`）据此同步偏移，两者视觉上保持贴合但渲染管线完全独立。

### 2.2 主要职责

- 无限画布编辑引擎（缩放、平移、节点拖拽、连线编辑，Bevy ECS 系统实现）
- 实时协作冲突解决（基于 `yrs`——Yjs 的 Rust 移植版 CRDT 库，前后端统一 Rust 实现）
- 节点配置面板与参数输入表单生成（HTML Overlay，见上表）
- WebSocket 长连接维护，实时推送节点执行状态
- 权限与多人编辑提示（谁在编辑哪个节点，通过 Bevy 中的协作者光标实体渲染）

### 2.3 特殊考虑

- ECS 层面的视口裁剪（Frustum Culling）实现大规模节点（1000+ 节点时）的渲染优化——仅渲染视口内实体，非可见节点组件休眠
- WASM 包体积控制：Bevy 默认功能集较全，需裁剪未使用的渲染特性（如 3D 相关模块）以缩短首次加载时间
- 本地 ECS 状态 vs 服务端权威状态同步的一致性策略（服务端为权威源，客户端乐观更新+服务端校正）

## 3. 详细设计（詳細設計書）

### 3.1 浏览器页面结构

```
浏览器页面
 ├─ <canvas id="bevy-canvas">                    // Bevy WASM 应用挂载点，占满视口
 │   （由 Bevy ECS 系统驱动，非 DOM 树，以下为 ECS 实体/系统结构，非 HTML 组件）
 │
 └─ <div id="html-overlay-root">                 // 与 bevy-canvas 同尺寸叠加的 DOM 层，pointer-events 默认穿透
     ├─ <NodeConfigForm />                        // 节点配置表单，仅当前选中节点打开配置时挂载
     ├─ <ExpressionEditor />                      // 表达式/JSON 映射规则编辑器（CodeMirror）
     ├─ <DebugPanel />                             // 执行日志时间轴、数据快照 JSON 树
     └─ <TenantWorkspaceSwitcher />                // 工作空间/租户切换器（顶部固定栏，非画布跟随）
```

### 3.2 Bevy ECS 画布数据模型

```rust
// ===== Component 定义 =====

#[derive(Component)]
struct CanvasNode {
    node_id: String,
    node_type: NodeType,
}

#[derive(Component)]
struct CanvasPosition(Vec2);          // 画布逻辑坐标系（非屏幕像素坐标）

#[derive(Component)]
struct NodeVisualState {
    status: NodeStatus,               // 驱动节点边框颜色（Pending灰/Running蓝/Success绿/Failed红）
    selected: bool,
}

#[derive(Component)]
struct CanvasEdge {
    edge_id: String,
    from_entity: Entity,
    to_entity: Entity,
    edge_kind: EdgeKind,              // DataFlow(血液，实线流光) | ControlFlow(肌肉，虚线箭头)
}

#[derive(Component)]
struct DataFlowAnimation {
    throughput: f32,                  // 来自 ChannelMetrics，驱动流光粒子速度
    queue_pressure: f32,              // 0.0~1.0，驱动连线颜色渐变（绿→红）
}

// ===== Resource（全局单例状态）=====

#[derive(Resource)]
struct ViewportTransform {
    pan: Vec2,
    zoom: f32,                        // 限制范围 0.1 ~ 10.0（对应需求 10%～1000%）
}

#[derive(Resource)]
struct SpatialIndex {
    // R-tree 空间索引，加速视锥裁剪查询与框选命中检测
    tree: rstar::RTree<IndexedNode>,
}

#[derive(Resource)]
struct SelectionState {
    selected_node_ids: HashSet<String>,
}

#[derive(Resource)]
struct OpenConfigPanel {
    // 非 None 时，通知 HTML Overlay 层挂载对应节点的配置表单
    node_id: Option<String>,
    anchor_screen_pos: Vec2,           // 用于 Overlay 定位
}
```

### 3.3 核心 ECS 系统（Systems）

```rust
/// 每帧执行：将画布逻辑坐标转换为屏幕像素坐标，驱动 Bevy Transform
fn sync_node_screen_position(
    viewport: Res<ViewportTransform>,
    mut query: Query<(&CanvasPosition, &mut Transform), With<CanvasNode>>,
) {
    for (canvas_pos, mut transform) in query.iter_mut() {
        let screen_pos = (canvas_pos.0 - viewport.pan) * viewport.zoom;
        transform.translation = screen_pos.extend(0.0);
    }
}

/// 视锥裁剪：仅渲染视口 + buffer 区域内的节点，非可见节点组件标记为休眠
fn frustum_culling_system(
    viewport: Res<ViewportTransform>,
    spatial_index: Res<SpatialIndex>,
    mut query: Query<(&CanvasNode, &mut Visibility)>,
) {
    let buffer_margin = 200.0; // 画布单位，视口外缓冲区，避免快速平移时的闪烁
    let visible_bounds = expand_bounds(viewport.visible_bounds(), buffer_margin);
    let visible_ids: HashSet<String> = spatial_index.tree
        .locate_in_envelope(&visible_bounds)
        .map(|n| n.node_id.clone())
        .collect();

    for (node, mut visibility) in query.iter_mut() {
        *visibility = if visible_ids.contains(&node.node_id) {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

/// 双击节点时，向 OpenConfigPanel Resource 写入请求，由 JS 胶水代码桥接挂载 HTML Overlay 表单
fn handle_node_double_click(
    mut open_panel: ResMut<OpenConfigPanel>,
    mut click_events: EventReader<NodeDoubleClickEvent>,
    query: Query<(&CanvasNode, &GlobalTransform)>,
) {
    for ev in click_events.read() {
        if let Ok((node, transform)) = query.get(ev.entity) {
            open_panel.node_id = Some(node.node_id.clone());
            open_panel.anchor_screen_pos = transform.translation().truncate();
        }
    }
}

/// 数据流吞吐指标驱动连线流光动效速度与颜色（対応 [M-03 §3.4](../modules/M-03-data-flow-engine.md)）
fn update_dataflow_animation(
    metrics: Res<DataFlowMetricsCache>,   // WebSocket 推送更新
    mut query: Query<(&CanvasEdge, &mut DataFlowAnimation)>,
) {
    for (edge, mut anim) in query.iter_mut() {
        if let Some(m) = metrics.get(&edge.edge_id) {
            anim.throughput = m.throughput as f32;
            anim.queue_pressure = (m.current_queue_depth as f32 / m.capacity as f32).clamp(0.0, 1.0);
        }
    }
}
```

### 3.4 HTML Overlay 桥接机制（中文输入表单）

```rust
// Bevy 侧：通过 wasm-bindgen 暴露状态变化事件给 JS 胶水层
#[wasm_bindgen]
pub fn get_open_config_panel_state() -> JsValue {
    // 序列化 OpenConfigPanel Resource 当前状态，供 JS 端轮询或由 Bevy 主动 postMessage
}

#[wasm_bindgen]
pub fn submit_config_panel_form(node_id: String, config_json: String) {
    // HTML 表单提交后回调此函数，将配置写回 ECS World 中对应 NodeDefinition.config
    // 并触发画布重渲染 + 通过 WebSocket 同步至后端持久化
}
```

```typescript
// JS/TS 胶水层：监听 Bevy 侧的面板打开请求，动态挂载/卸载 React 组件到 Overlay 层
bevyModule.onConfigPanelOpen((state: OpenConfigPanelState) => {
  if (state.node_id) {
    mountConfigForm(state.node_id, state.anchor_screen_pos); // ReactDOM.createRoot 挂载
  } else {
    unmountConfigForm();  // 关闭面板时立即卸载，避免常驻 DOM 增加开销
  }
});

// 表单提交时调用回 Bevy WASM 导出函数，写回 ECS 状态
function onFormSubmit(nodeId: string, configJson: string) {
  bevyModule.submit_config_panel_form(nodeId, configJson);
}
```

**坐标同步细节**：Overlay 表单容器使用 `position: absolute`，其 `left/top` 由 `OpenConfigPanel.anchor_screen_pos` 决定；Bevy 侧 `ViewportTransform` 每帧变化时，通过同一份共享状态（`SharedArrayBuffer` 或高频 `postMessage`，视浏览器兼容性选择）同步给 JS 层，Overlay 位置随画布平移/缩放实时跟随，视觉上表现为"挂在节点旁边的浮动面板"。

**技术选型**：Overlay 表单内的富文本/表达式编辑场景使用 CodeMirror 6（原生支持 IME 组合输入事件 `compositionstart`/`compositionupdate`/`compositionend`），普通字段使用原生 `<input>`/`<textarea>`。

### 3.5 视口虚拟化渲染性能保证（对应 7.2 性能要件：1000 节点 30fps）

| 优化点 | 实现方式 |
|---|---|
| 视锥裁剪 | R-tree 空间索引（§3.3 `frustum_culling_system`），节点位置变更时仅更新局部索引节点，不整体重建 |
| 渲染批处理 | Bevy 的 Sprite Batching：同材质节点卡片合批绘制，减少 draw call |
| 连线渲染 | 使用 Bevy 的 `Gizmos` 或自定义 Mesh 而非逐帧生成新几何体，流光动效通过 Shader uniform 参数驱动（避免 CPU 端逐帧重算顶点） |
| ECS 查询优化 | 高频系统（如 `sync_node_screen_position`）使用 `Changed<T>` 过滤器，仅处理本帧发生变化的实体 |
| WASM 包体积 | 裁剪 Bevy 默认 feature（禁用 3D、音频、动画等未使用模块），启用 `wasm-opt -O3` |

### 3.6 状态管理架构

```rust
// Bevy Resource 承担传统前端框架中"全局 Store"的角色，按关注点分离为多个 Resource

#[derive(Resource, Default)]
struct CanvasState {
    canvas_id: Option<String>,
    version: u32,
}

#[derive(Resource, Default)]
struct ExecutionState {
    execution_id: Option<String>,
    node_statuses: HashMap<String, NodeStatus>,   // WebSocket 推送实时更新
}

#[derive(Resource, Default)]
struct TenantState {
    current_tenant_id: String,
    current_workspace_id: String,
    user_role: Role,
    quota: TenantQuotaView,
}

#[derive(Resource, Default)]
struct CollaborationState {
    // yrs（Yjs Rust 移植）文档句柄，协作者感知状态（Awareness）
    doc: Option<yrs::Doc>,
    remote_cursors: HashMap<String, RemoteCursor>,  // 驱动其他协作者光标实体的渲染
}
```

**本地状态与服务端权威状态的一致性策略**：客户端对节点位置拖拽等高频操作采用乐观更新（本地 ECS 立即响应），通过 WebSocket 异步同步至服务端；服务端为最终权威源，若同步失败或与其他协作者的 CRDT 合并结果冲突，客户端接收服务端校正后的状态并静默纠正本地渲染（不打断用户当前操作）。

#### 3.6.1 v0.6.0 CRDT 同步（Yrs）

> v0.6.0 升级: v0.5.0 LWW（last-write-wins, server-wins）→ Yjs-compatible
> CRDT（Yrs, MIT）。理由: 多人协作下 LWW 会静默丢并发客户端编辑,
> 与 m11 rbac-collab 多人同时编辑预期不符。

**CRDT 数据结构（per YDoc root layout）**:

- `meta` (YMap) → `{ name, version }`
- `elements` (YArray) → YMap 列表, 每个 YMap 含 `{ id (UUID), kind, x, y, label, ports, alive }`
- `edges` (YArray) → YMap 列表, 每个 YMap 含 `{ from (UUID), to (UUID), alive }`

每 element 是 YMap (不是 YArray 的一项) 是关键: 让 concurrent move
同一 node 走 Yjs YMap 字段级 LWW, 而不是 YArray position conflict。

**API surface** (`crates/ada-m12-canvas-editor/src/crdt.rs`,
gated by `--features crdt`):

| 函数 | 用途 |
|---|---|
| `merge_crdt_update(doc, remote_state, update_bytes) -> Vec<u8>` | apply remote update + return diff 让 remote 追平 |
| `encode_state_as_update(doc) -> Vec<u8>` | full state snapshot (首次同步 / 落盘) |
| `reconcile_with_crdt(server: &Canvas, client_update, client_version) -> CrdtReconcileResult` | end-to-end reconcile: server Canvas + client CRDT 互操作 |
| `CrdtReconcileResult { merged_state: Vec<u8>, new_version: u64 }` | serde-derived, wire-friendly |

**Fallback 开关** (Cargo features):

| Feature | 状态 | 路径 |
|---|---|---|
| `crdt` | v0.6.0 默认 off (可 opt-in) | Yrs sync (新) |
| `legacy-lww` | v0.6.0 默认 off | `server_recon.rs` 3-way LWW (v0.5.0) |
| `server` | v0.5.0 alias of `legacy-lww` | 兼容 m13 集成测试 |

5 门 CI 默认走 `default` (无 yrs 编译负担)。`--features crdt` 跑
CRDT 套件, `--features legacy-lww` 跑 v0.5.0 LWW 套件 (回归)。

**v0.6.0 同步协议 (3 客户端 star-shaped merge)**:

```rust
use ada_m12_canvas_editor::{
    encode_state_as_update, merge_crdt_update, reconcile_with_crdt, Canvas,
};
use yrs::{Doc, Transact, WriteTxn};

// Client → Server 增量同步
let client_doc = yrs::Doc::new();
let update = encode_state_as_update(&client_doc);
let sv = server_doc.transact().state_vector().encode_v1();
let diff = merge_crdt_update(&server_doc, &sv, &update)?;
// client apply diff 后 = 同步

// End-to-end reconcile (server Canvas + client CRDT)
let result = reconcile_with_crdt(&server_canvas, &client_update_bytes, client_version)?;
// result.merged_state: 新的 server snapshot
// result.new_version: max(server.version, client_version) + 1
```

详细集成说明 / 迁移路径 / 已知限制见
`crates/ada-m12-canvas-editor/CRDT.md`。

## 4. 验收要点

1. **画布性能**：单画布 1,000 节点 5,000 条连线规模下，前端交互（拖拽/缩放/连线）保持 ≥ 30fps（[architecture/03-cross-cutting-risks.md §4.4](../architecture/03-cross-cutting-risks.md)）。
2. **画布基础能力**：F-01-01 缩放 10%~1000%、F-01-02 框选/多选、F-01-03 连线创建删除、F-01-04 数据流/控制流连线视觉区分、F-01-05 注释与分组框、F-01-06 Undo/Redo ≥ 50 步均正常工作。
3. **中文输入体验**：节点详细配置表单（HTML Overlay）下中文输入无候选词定位问题，与原生 `<input>` 体验一致。
4. **实时协作**：≥ 3 用户同时编辑，前端正确显示其他协作者光标与选中状态，CRDT 合并无状态错乱。
5. **免安装**：Runtime 在 Windows/macOS/Linux 三大桌面操作系统上免安装启动（单命令/单文件双击），满足 F-09。 [NF-MIG]【必須】

---

## 5. 用語集

| 用語 | 説明 | 出典 / 参照 |
|---|---|---|
| 无限画布 | 缩放/平移的二维操作界面 | §1、F-01 |
| Bevy | Rust 游戏引擎 | §2.1 |
| bevy_egui | Bevy 即时模式 GUI | §2.1 |
| HTML Overlay | DOM 叠加层，承载中文输入 | §2.1 [NF-OPS]【必須】 |
| ECS | Entity-Component-System | §3.2 [NF-PER]【必須】 |
| 视锥裁剪 | Frustum Culling | §3.3 [NF-PER]【必須】 |
| R-tree | 空间索引数据结构 | §3.3 |
| ViewportTransform | 视口平移/缩放 | §3.2 |
| wasm-bindgen | Rust ↔ JS 桥接 | §3.4 [NF-ENV]【必須】 |
| SharedArrayBuffer | 高性能数据共享 | §3.4 |
| CodeMirror 6 | 支持 IME 的富文本编辑器 | §3.4 |
| 协作者光标 | Awareness 渲染的远端光标 | §3.6 |
| 服务端权威 | 服务端为最终状态源 | §3.6 |
| CRDT | Conflict-free Replicated Data Type (Yjs-compatible) | §3.6.1 |
| Yrs | Yjs 的 Rust 移植（m12 v0.6.0 CRDT 后端, MIT） | §3.6.1 |
| LWW (legacy) | v0.5.0 3-way merge: server-wins on conflict | §3.6.1 |
| WASM 包体积 | 控制首屏加载时间 | §3.5 [NF-ENV]【必須】 |
| 帧率 (FPS) | 30fps 验收硬指标 | §3.5 [NF-PER]【必須】 |

## 6. 参考文献

1. IPA「共通フレーム2018 (SLCP-JCF2018)」、独立行政法人情報処理推進機構、2018年3月
2. IPA「非機能要求グレード2018」、独立行政法人情報処理推進機構、2018年4月
3. Bevy 公式ドキュメント「Bevy — A refreshingly simple data-driven game engine」
4. wasm-bindgen 公式ドキュメント「wasm-bindgen — Facilitating high-level interactions between Wasm modules and JavaScript」
5. CodeMirror 公式ドキュメント「CodeMirror 6 — A versatile text editor for the web」
6. Ada プロジェクトチーム「Ada 无限画布跨平台数据集成システム 詳細設計書 v1.3.0」、2026-08-18（[DOC-DTL-001](../legacy/detailed-design.md)）

---

*本書は IPA「共通フレーム2018」(SLCP-JCF2018) 及び IPA「非機能要求グレード」に準拠して作成された。*
*本書の無断転載・複製を禁ずる。© Ada プロジェクトチーム*
