//! Bridge system: Rust canvas ↔ Bevy ECS 桥接 (feature = "bevy").
//!
//! 见 `src/bevy_plugin.rs` 顶部注释了解整体集成示例。
//!
//! 设计依据: `docs/modules/M-12-canvas-editor-frontend.md` §3.2
//! (Bevy ECS 画布数据模型), §3.3 (核心 ECS 系统), §3.6 (本地
//! ECS 状态 vs 服务端权威状态一致策略)。
//!
//! 当前实现只做"Canvas → ECS"单向 push(每帧同步):
//!
//! 1. Canvas 增删节点 → ECS entity 增删
//! 2. 节点位置 / label / kind 变更 → ECS component 字段更新
//! 3. Canvas 引用计数为零时 ECS entity 仍保留(等下一帧 stale
//!    回收),防止误删
//!
//! "ECS → Canvas"反向 push 留给上层应用(通常经由
//! [`bevy_egui`](https://crates.io/crates/bevy_egui) 拖拽事件)。

#![cfg(feature = "bevy")]

use std::collections::{HashMap, HashSet};

use bevy_ecs::prelude::*;

use crate::bevy_plugin::{CanvasNodeComp, CanvasPositionComp, CanvasResource};

/// Sync system: keep ECS entities in lockstep with the canvas's node
/// set.
///
/// Behaviour per frame:
///
/// 1. For every node id in the canvas, ensure exactly one ECS entity
///    exists with [`CanvasNodeComp`] + [`CanvasPositionComp`] carrying
///    the latest snapshot.
/// 2. Despawn ECS entities whose node id is no longer in the canvas.
///
/// Performance: O(N) in the number of nodes, using a `Local<HashMap>`
/// cache to remember the entity → id mapping across frames. This is
/// the bridge hot path; in production a diff algorithm would be
/// preferred (see `M-12 §3.5` ECS 查询优化). The skeleton keeps the
/// O(N) version for clarity.
///
/// Robustness: if [`CanvasResource`] is missing, the system is a
/// no-op. This keeps startup ordering flexible (host may insert the
/// resource mid-frame).
pub fn sync_canvas_system(
    canvas: Option<Res<CanvasResource>>,
    mut commands: Commands,
    mut existing: Local<HashMap<crate::node::NodeId, Entity>>,
) {
    let Some(canvas) = canvas else { return };

    let current = canvas.0.nodes();
    let current_ids: HashSet<crate::node::NodeId> = current.iter().map(|n| n.id).collect();

    // 1. Remove stale entities (node was deleted from the canvas).
    let stale: Vec<crate::node::NodeId> = existing
        .keys()
        .copied()
        .filter(|id| !current_ids.contains(id))
        .collect();
    for id in stale {
        if let Some(e) = existing.remove(&id) {
            commands.entity(e).despawn();
        }
    }

    // 2. Add new entities / update changed ones.
    for node in &current {
        if let Some(&entity) = existing.get(&node.id) {
            // Update the existing entity's components. We cannot
            // mutate the position in-place through `&mut World` from
            // a system param without `Query`, so we use
            // `commands.entity(e).insert(...)` which is buffered
            // and applied at the end of the stage. Bevy 0.14
            // de-duplicates identical inserts.
            commands.entity(entity).insert((
                CanvasNodeComp {
                    id: node.id,
                    kind: node.kind,
                    label: node.label.clone(),
                },
                CanvasPositionComp(node.position),
            ));
        } else {
            let e = commands
                .spawn((
                    CanvasNodeComp {
                        id: node.id,
                        kind: node.kind,
                        label: node.label.clone(),
                    },
                    CanvasPositionComp(node.position),
                ))
                .id();
            existing.insert(node.id, e);
        }
    }
}

#[cfg(test)]
mod tests {
    //! 单元测试不依赖 bevy_app::App 的完整 Stage 调度,直接
    //! 走 `World::run_system` 跑一次 sync system。`Local<HashMap>`
    //! 跨帧持久化通过 `World::local` 隐式维护 — 单帧 `run_system`
    //! 也能复用,因为 Local state 绑定到 system id。

    use super::*;
    use crate::node::{CanvasNode, NodeKind, Position};
    use crate::Canvas;
    use std::sync::Arc;

    fn world_with_canvas(c: Canvas) -> World {
        let mut world = World::new();
        world.insert_resource(CanvasResource(Arc::new(c)));
        world
    }

    #[test]
    fn sync_is_noop_without_resource() {
        let mut world = World::new();
        let mut schedule = Schedule::default();
        schedule.add_systems(sync_canvas_system);
        // No panic.
        schedule.run(&mut world);
    }

    #[test]
    fn sync_creates_entity_for_each_node() {
        let canvas = Canvas::new("t");
        canvas.add_node(CanvasNode::new(NodeKind::Block, Position::new(1, 2), "a"));
        canvas.add_node(CanvasNode::new(NodeKind::Note, Position::new(3, 4), "b"));

        let mut world = world_with_canvas(canvas);
        let mut schedule = Schedule::default();
        schedule.add_systems(sync_canvas_system);
        schedule.run(&mut world);

        let mut q = world.query::<(&CanvasNodeComp, &CanvasPositionComp)>();
        let items: Vec<_> = q.iter(&world).collect();
        assert_eq!(items.len(), 2, "expected 2 ECS entities");
        let labels: Vec<&str> = items.iter().map(|(n, _)| n.label.as_str()).collect();
        assert!(labels.contains(&"a"));
        assert!(labels.contains(&"b"));
    }

    #[test]
    fn sync_despawns_stale_entities_after_node_removal() {
        let canvas = Canvas::new("t");
        let a = canvas.add_node(CanvasNode::new(NodeKind::Block, Position::new(0, 0), "a"));
        canvas.add_node(CanvasNode::new(NodeKind::Block, Position::new(0, 0), "b"));

        let arc = Arc::new(canvas);
        let mut world = World::new();
        world.insert_resource(CanvasResource(Arc::clone(&arc)));

        let mut schedule = Schedule::default();
        schedule.add_systems(sync_canvas_system);
        schedule.run(&mut world);
        assert_eq!(world.query::<&CanvasNodeComp>().iter(&world).count(), 2);

        // Remove a node, then run the schedule again.
        arc.remove_node(a).expect("remove a");
        schedule.run(&mut world);

        let mut q = world.query::<&CanvasNodeComp>();
        let labels: Vec<String> = q.iter(&world).map(|n| n.label.clone()).collect();
        assert_eq!(
            labels,
            vec!["b".to_string()],
            "stale entity should be despawned"
        );
    }
}
