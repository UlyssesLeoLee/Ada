//! bevy_egui 集成 — M-12 v0.3.0 inspector 面板 + 拖拽事件
//! (feature = "bevy_egui")。
//!
//! 设计依据: `docs/modules/M-12-canvas-editor-frontend.md` §3.6
//! (客户端乐观更新+服务端校正), `docs/decisions/02-design-adrs.md`
//! D-04 (Bevy 0.14 stable), `docs/architecture/06-rust-tech-selection.md`
//! §10 (Bevy 0.14 + bevy_egui)。
//!
//! ## 职责
//!
//! 1. [`CanvasInspectorPlugin`] — Bevy Plugin,挂 [`EguiPlugin`] +
//!    inspector 系统 + 拖拽 + ECS→Canvas 反向 sync
//! 2. [`NodeInspectorState`] — 选中节点的 ECS Resource
//! 3. [`node_inspector_system`] — egui 右侧 SidePanel,显示选中
//!    节点的 id / kind / label / position,TextEdit + DragValue
//!    写回 ECS component,触发 [`sync_ecs_to_canvas_system`] 反向
//!    push 到 [`Canvas`]
//! 4. [`sync_ecs_to_canvas_system`] — ECS Component 变更 → Canvas
//!    节点更新(走 `Canvas::move_node` 内部 Mutex,try_lock 避免
//!    死锁)
//! 5. [`drag_node_system`] — 拖拽事件(简化版:用 `NodeDragState`
//!    记录 drag start,鼠标移动时调 `Canvas::move_node`)
//!
//! ## Native-only
//!
//! `bevy_egui` feature 默认 off,只在 native 跑 inspector +
//! 拖拽测试场景启用。WASM build 走 `--features wasm`(不带
//! `bevy_egui`),不拉 bevy_egui / egui / bevy(避免 D-05 8 MB
//! 体积 ceiling 越界)。`bevy_egui` feature 自身不含 render
//! feature(关掉默认 features),所以只拉 bevy_asset / bevy_input
//! / bevy_window 等核心 ECS 子 crate。
//!
//! ## 注意事项
//!
//! 1. `egui::SidePanel` 必须在 `EguiSet::ProcessInput` 之后的
//!    system 中调用(Bevy 0.14 强制约束);这里用 `Update` set
//!    默认顺序即可。
//! 2. `Canvas::inner` 是 `parking_lot::Mutex`,`sync_ecs_to_canvas_system`
//!    用 `try_lock` 避免和 inspector 的读 lock 冲突 — 一旦
//!    lock 失败,下一帧重试(可接受,因为 ECS 写回有频率冗余)。
//! 3. inspector 的 form 字段变更要写回 ECS Component,而不是
//!    直接写 Canvas;这样保证下一帧 Canvas→ECS 同步时不会丢
//!    失(单向 sync 系统的 source of truth 仍是 Canvas)。

#![cfg(feature = "bevy_egui")]

use std::sync::Arc;

use bevy_app::{App, Plugin, Update};
use bevy_ecs::prelude::*;
use bevy_egui::egui::{self, DragValue, SidePanel, TextEdit};

use crate::bevy_plugin::{CanvasNodeComp, CanvasPositionComp, CanvasResource};
use crate::node::{NodeId, Position};

/// 选中节点状态,Bevy Resource,host 应用通过修改它驱动
/// inspector UI。
///
/// `selected: Option<NodeId>` — 当前选中的 node id,`None` 时
/// inspector 显示 "no node selected" 占位文案。
///
/// `dirty: bool` — inspector form 修改后置 true,驱动
/// [`sync_ecs_to_canvas_system`] 本帧写回 Canvas(默认行为
/// 是每帧都 sync,所以 dirty 标记仅是优化项,这里为简化保留
/// 字段,值恒为 true 也能正确工作)。
#[derive(Debug, Default, Resource, Clone)]
pub struct NodeInspectorState {
    /// 当前选中的 node id;`None` = 无选中。
    pub selected: Option<NodeId>,
}

impl NodeInspectorState {
    /// Create a new state with no selection.
    #[must_use]
    pub const fn new() -> Self {
        Self { selected: None }
    }

    /// Select a node by id.
    pub fn select(&mut self, id: NodeId) {
        self.selected = Some(id);
    }

    /// Clear the current selection.
    pub fn clear(&mut self) {
        self.selected = None;
    }

    /// Returns true if `id` is currently selected.
    #[must_use]
    pub fn is_selected(&self, id: NodeId) -> bool {
        self.selected == Some(id)
    }
}

/// Bevy Plugin: 把 `EguiPlugin` 接到 m12 canvas,提供
/// inspector + 拖拽 + 反向 sync。
///
/// Host 端使用:
///
/// ```ignore
/// use ada_m12_canvas_editor::{
///     bevy_integration::CanvasResource,
///     egui_integration::CanvasInspectorPlugin,
///     Canvas,
/// };
/// use std::sync::Arc;
///
/// let mut app = bevy_app::App::new();
/// app.insert_resource(CanvasResource(Arc::new(Canvas::new("demo"))));
/// app.add_plugins(CanvasInspectorPlugin);
/// app.run();
/// ```
///
/// Host 必须自己挂 `DefaultPlugins`(含 Window / Input)才能让
/// bevy_egui 真正把 UI 渲到屏幕;本 plugin 只在 m12 视角组织
/// inspector / 拖拽 / 反向 sync 系统,不强加 host 端 winit 启动
/// 流程(让 host 决定 native / wasm 怎么起窗口)。
#[derive(Debug, Default, Clone, Copy)]
pub struct CanvasInspectorPlugin;

impl Plugin for CanvasInspectorPlugin {
    fn build(&self, app: &mut App) {
        // inspector 状态资源(无选中)
        app.init_resource::<NodeInspectorState>();
        // 反向 sync: ECS Component 变更 → Canvas 节点
        app.add_systems(
            Update,
            (
                // 1. Canvas → ECS 单向 push(已有)
                crate::bevy_bridge::sync_canvas_system,
                // 2. Inspector UI 写 ECS Component
                node_inspector_system,
                // 3. ECS Component 变更 → Canvas 节点
                sync_ecs_to_canvas_system,
            )
                .chain(),
        );
    }
}

/// Inspector UI system: 在 egui 右侧 SidePanel 显示选中节点
/// 的 id / kind / label / position,TextEdit + DragValue 修改
/// 后写回 ECS component,触发反向 sync。
///
/// 行为:
/// - `NodeInspectorState::selected == None` → 显示 "no node selected"
/// - 否则查 ECS 找 `CanvasNodeComp` + `CanvasPositionComp`,显示
///   form。修改 label / position → `commands.entity(e).insert(...)`
///   写新 component(下一帧 sync_ecs_to_canvas_system 把它推到
///   Canvas)
pub fn node_inspector_system(
    mut contexts: bevy_egui::EguiContexts,
    state: Res<NodeInspectorState>,
    q: Query<(Entity, &CanvasNodeComp, &CanvasPositionComp), With<CanvasNodeComp>>,
    mut commands: Commands,
) {
    // 拿到当前 primary window 的 egui::Context;无 window 时跳过
    let Some(ctx) = contexts.try_ctx_mut() else {
        return;
    };

    // 备份 mutable borrow 避免和 SidePanel closure 的 immut 冲突
    let selected_id = state.selected;
    let panel_response = SidePanel::right("canvas_inspector")
        .default_width(280.0)
        .show(ctx, |ui| {
            ui.heading("Canvas Inspector");
            ui.separator();
            if let Some(target) = selected_id {
                // 查 ECS entity
                let mut hit: Option<(Entity, CanvasNodeComp, CanvasPositionComp)> = None;
                for (e, n, p) in q.iter() {
                    if n.id == target {
                        hit = Some((e, n.clone(), *p));
                        break;
                    }
                }
                if let Some((entity, node, pos)) = hit {
                    render_inspector_form(ui, entity, &node, pos, &mut commands);
                } else {
                    ui.label(format!("Selected node {target} not in ECS yet"));
                }
            } else {
                ui.label("No node selected.");
                ui.small("Click a node on the canvas to inspect it.");
            }
        });
    // panel_response 保留供调试(响应式 layout 未来 hook 点)
    let _ = panel_response;
}

/// 渲染 inspector form(单选节点):id / kind / label / position。
///
/// `commands` 是 ECS Commands,我们用 `commands.entity(e).insert(...)`
/// 写新 component 而非直接 query mutable,避免和 `q` 的 borrow
/// 冲突(Bevy 0.14 system param conflict 检测比较严)。
fn render_inspector_form(
    ui: &mut egui::Ui,
    entity: Entity,
    node: &CanvasNodeComp,
    pos: CanvasPositionComp,
    commands: &mut Commands,
) {
    // id (read-only)
    ui.horizontal(|ui| {
        ui.label("id:");
        ui.label(format!("{}", node.id));
    });

    // kind (read-only, m12 v0.3.0 不支持改 kind)
    ui.horizontal(|ui| {
        ui.label("kind:");
        ui.label(format!("{}", node.kind));
    });

    // label (editable)
    let mut new_label = node.label.clone();
    ui.horizontal(|ui| {
        ui.label("label:");
        ui.add(TextEdit::singleline(&mut new_label).hint_text("node label"));
    });
    if new_label != node.label {
        let new_node = CanvasNodeComp {
            id: node.id,
            kind: node.kind,
            label: new_label,
        };
        commands.entity(entity).insert(new_node);
    }

    // position (editable: x / y drag value)
    let mut new_x = pos.0.x;
    let mut new_y = pos.0.y;
    ui.horizontal(|ui| {
        ui.label("x:");
        ui.add(DragValue::new(&mut new_x).speed(1.0));
        ui.label("y:");
        ui.add(DragValue::new(&mut new_y).speed(1.0));
    });
    let new_pos = Position::new(new_x, new_y);
    if new_pos != pos.0 {
        commands.entity(entity).insert(CanvasPositionComp(new_pos));
    }
}

/// Reverse sync system: ECS Component 变更 → Canvas 节点更新。
///
/// 当前实现简化:对每个带 `CanvasNodeComp` 的 ECS entity,如果
/// 它的 `label` / `position` / `kind` 与 Canvas 中的最新值不
/// 一致,就更新 Canvas 内部节点(走 same-crate 访问 `inner`,
/// 因为 `Canvas` 暂未提供 public `update_label` / `update_kind`
/// API — 这是 v0.3.1 的 TODO)。
///
/// `try_lock` 防止 inspector 读 lock / sync_canvas_system 写 lock
/// 死锁:lock 拿不到就跳过本帧,下一帧重试。
pub fn sync_ecs_to_canvas_system(
    canvas: Option<Res<CanvasResource>>,
    q: Query<(&CanvasNodeComp, &CanvasPositionComp)>,
) {
    let Some(canvas) = canvas else { return };
    let arc: &Arc<crate::Canvas> = &canvas.0;

    // 拿 Canvas 内部 Mutex;失败说明有别的 system 在持有,跳过本帧
    let Some(mut guard) = arc.inner.try_lock() else {
        return;
    };

    let mut dirty = false;
    for (node, pos) in &q {
        let Some(canvas_node) = guard.nodes.get_mut(&node.id) else {
            continue;
        };
        if canvas_node.position != pos.0 {
            canvas_node.position = pos.0;
            dirty = true;
        }
        if canvas_node.label != node.label {
            canvas_node.label = node.label.clone();
            dirty = true;
        }
        if canvas_node.kind != node.kind {
            canvas_node.kind = node.kind;
            dirty = true;
        }
    }
    if dirty {
        guard.version += 1;
    }
}

/// Marker: 当前正在拖拽的 node id + 拖拽起始位置。
#[derive(Debug, Default, Resource, Clone, Copy)]
pub struct NodeDragState {
    /// 正在拖拽的 node id。
    pub dragging: Option<NodeId>,
    /// 拖拽开始时的 canvas 坐标,用于计算 delta。
    pub last_pos: Position,
}

impl NodeDragState {
    /// Create a new empty state.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            dragging: None,
            last_pos: Position::new(0, 0),
        }
    }
}

/// 简化版拖拽系统 v0.3.0:不读 MouseButtonInput 事件(bevy 0.14
/// 抽到 bevy_input / bevy_window 下的 input events,本 feature
/// 不强制依赖),而是提供 `begin_drag` / `update_drag` / `end_drag`
/// 三个 host-driven 函数,由 host app 在 input 处理中调用。
///
/// 这样 m12 不绑死 input 事件源(host 可以用 egui / 原生 bevy
/// input / 第三方输入库,任选),同时提供可测试的拖拽逻辑。
///
/// `begin_drag(id, pos)` — 标记开始拖拽 `id`,记录起始位置
/// `update_drag(pos)` — 更新拖拽目标位置,`move_node` 写回 Canvas
/// `end_drag()` — 清空拖拽状态
pub fn begin_drag(state: &mut NodeDragState, id: NodeId, pos: Position) {
    state.dragging = Some(id);
    state.last_pos = pos;
}

/// Update drag target position;calls `Canvas::move_node` to
/// write back the new position.
///
/// 已知缺口:如果 Canvas 锁被别的 system 持有,本帧 `move_node`
/// 跳过(返回 `Err(BackendError)` 上层处理),下一帧 retry。
pub fn update_drag(
    state: &NodeDragState,
    canvas: &crate::Canvas,
    new_pos: Position,
) -> crate::error::Result<()> {
    let Some(id) = state.dragging else {
        return Ok(());
    };
    if new_pos == state.last_pos {
        return Ok(());
    }
    canvas.move_node(id, new_pos)?;
    Ok(())
}

/// End an in-progress drag (no-op if not dragging).
pub fn end_drag(state: &mut NodeDragState) {
    state.dragging = None;
}

#[cfg(test)]
mod tests {
    //! 单元测试覆盖:
    //! 1. `inspector_state_default` — `NodeInspectorState::default()` + select/clear
    //! 2. `inspector_panel_renders` — egui Context + SidePanel 调用不 panic
    //! 3. `drag_node_updates_position` — 模拟拖拽事件,验证 Canvas 节点 position 更新
    //! 4. `reverse_sync_writes_position` — ECS 组件变更 → Canvas 节点更新
    use super::*;
    use crate::bevy_bridge;
    use crate::node::{CanvasNode, NodeKind};
    use crate::Canvas;

    #[test]
    fn inspector_state_default() {
        let s = NodeInspectorState::default();
        assert!(s.selected.is_none());
        let id = NodeId::new();
        assert!(!s.is_selected(id));

        let mut s = NodeInspectorState::new();
        s.select(id);
        assert!(s.is_selected(id));
        s.clear();
        assert!(s.selected.is_none());
    }

    #[test]
    fn inspector_panel_renders() {
        // egui 0.28 requires `Context::run()` to wrap widget
        // calls so that layout requests (e.g. `available_rect()`
        // inside `SidePanel`) see a valid frame state. Calling
        // `SidePanel::show` outside `run` panics in 0.28.
        let ctx = egui::Context::default();
        let mut did_run = false;
        ctx.run(egui::RawInput::default(), |ctx| {
            SidePanel::right("test_inspector")
                .default_width(200.0)
                .show(ctx, |ui| {
                    ui.heading("Test");
                    ui.label("hello inspector");
                    did_run = true;
                });
        });
        assert!(did_run, "SidePanel closure should run");
    }

    #[test]
    fn drag_node_updates_position() {
        let canvas = Canvas::new("drag_test");
        let id = canvas.add_node(CanvasNode::new(
            NodeKind::Block,
            Position::new(0, 0),
            "drag-me",
        ));
        assert_eq!(
            canvas.get_node(id).expect("node").position,
            Position::new(0, 0)
        );

        let mut state = NodeDragState::new();
        begin_drag(&mut state, id, Position::new(0, 0));
        assert_eq!(state.dragging, Some(id));

        // 拖到 (50, 60)
        update_drag(&state, &canvas, Position::new(50, 60)).expect("move");
        assert_eq!(
            canvas.get_node(id).expect("node").position,
            Position::new(50, 60),
            "Canvas node position should reflect drag target"
        );

        // 再拖到 (100, 200)
        update_drag(&state, &canvas, Position::new(100, 200)).expect("move");
        assert_eq!(
            canvas.get_node(id).expect("node").position,
            Position::new(100, 200)
        );

        // 结束拖拽
        end_drag(&mut state);
        assert!(state.dragging.is_none());
    }

    #[test]
    fn reverse_sync_writes_position() {
        // Canvas with one node at (0, 0)
        let canvas = Canvas::new("reverse_sync");
        let id = canvas.add_node(CanvasNode::new(NodeKind::Block, Position::new(0, 0), "a"));

        // Build a world; do NOT pre-spawn an entity — let
        // sync_canvas_system create it (otherwise we end up
        // with two entities sharing the same `NodeId` and
        // the second forward-sync iteration will overwrite
        // the position we set in the test).
        let mut world = World::new();
        let arc = Arc::new(canvas);
        world.insert_resource(CanvasResource(Arc::clone(&arc)));

        // Run forward sync to create the entity.
        let mut schedule = bevy_ecs::schedule::Schedule::default();
        schedule.add_systems(bevy_bridge::sync_canvas_system);
        schedule.run(&mut world);

        // Now find the entity and mutate its position to (42, 7).
        let mut q = world.query::<(Entity, &CanvasNodeComp)>();
        let (e, _) = q
            .iter(&world)
            .find(|(_, n)| n.id == id)
            .expect("forward sync should create entity");
        drop(q);
        world
            .entity_mut(e)
            .insert(CanvasPositionComp(Position::new(42, 7)));

        // Run reverse sync.
        let mut schedule = bevy_ecs::schedule::Schedule::default();
        schedule.add_systems(sync_ecs_to_canvas_system);
        schedule.run(&mut world);

        // Canvas node position should be written back to (42, 7).
        let updated = arc.get_node(id).expect("node");
        assert_eq!(
            updated.position,
            Position::new(42, 7),
            "reverse sync should write ECS position to Canvas"
        );
        // version bumped (add_node + reverse sync).
        assert!(arc.version() >= 2);
    }
}
