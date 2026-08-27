//! Bevy 0.14 plugin for the M-12 canvas editor (feature = "bevy").
//!
//! 暴露 [`CanvasPlugin`] 让 Bevy App 集成 [`Canvas`]:
//!
//! ```ignore
//! use ada_m12_canvas_editor::{bevy_integration::CanvasPlugin, Canvas};
//! use std::sync::Arc;
//!
//! let canvas = Canvas::new("my-canvas");
//! // ... populate canvas ...
//!
//! let mut app = bevy_app::App::new();
//! app.insert_resource(
//!     ada_m12_canvas_editor::bevy_integration::CanvasResource(Arc::new(canvas)),
//! );
//! app.add_plugins(CanvasPlugin);
//! app.run();
//! ```
//!
//! 设计依据: `docs/modules/M-12-canvas-editor-frontend.md` §3.2
//! (Bevy ECS 画布数据模型), `docs/decisions/02-design-adrs.md`
//! D-04 (Bevy 0.14 stable),
//! `docs/architecture/06-rust-tech-selection.md` §10 (Bevy 0.14 +
//! bevy_egui).
//!
//! 注意事项:
//!
//! 1. `bevy_ecs` / `bevy_app` 0.14 都用 `default-features = false`
//!    拉,避免把 bevy_render / bevy_audio 拖入(WASM 体积敏感,
//!    D-05 8 MB ceiling)。
//! 2. sync 系统在 `bevy_bridge.rs`,与本文件分离便于单测。
//! 3. 不在 `build()` 里默认 insert `CanvasResource`,因为
//!    Bevy App 与 Canvas 解耦,host 端决定何时插入。

#![cfg(feature = "bevy")]

use std::sync::Arc;

use bevy_app::{App, Plugin, Update};
use bevy_ecs::prelude::*;

use crate::node::{NodeId, NodeKind, Position};
use crate::Canvas;

/// Bevy Component: identity + display label of a canvas node, mirrors
/// [`crate::CanvasNode`] (sans the inner `ports` Vec, which is render
/// concern and out of scope for the bridge).
#[derive(Debug, Component, Clone)]
pub struct CanvasNodeComp {
    /// Stable id.
    pub id: NodeId,
    /// Kind.
    pub kind: NodeKind,
    /// Human-readable label.
    pub label: String,
}

/// Bevy Component: position of a canvas node in the canvas coordinate
/// system (not screen pixels). See `M-12-canvas-editor-frontend.md`
/// §3.2 `CanvasPosition(Vec2)` — we use the typed [`Position`] here
/// (i32 coordinates, the skeleton does not enforce bounds).
#[derive(Debug, Component, Clone, Copy)]
pub struct CanvasPositionComp(pub Position);

/// Bevy Resource: shared reference to the underlying [`Canvas`].
///
/// Wrap in `Arc<Canvas>` so the same canvas instance can be shared
/// between the Bevy ECS world and a non-Bevy owner (e.g. the JS bridge
/// holding its own `WasmCanvas`).
#[derive(Debug, Resource, Clone)]
pub struct CanvasResource(pub Arc<Canvas>);

/// Bevy Plugin: registers the canvas sync system.
///
/// The plugin does NOT own the canvas. The host application is
/// responsible for `app.insert_resource(CanvasResource(Arc::new(canvas)))`
/// before `app.update()` is called. This avoids hard-coding a
/// specific canvas name in the plugin.
#[derive(Debug, Default, Clone, Copy)]
pub struct CanvasPlugin;

impl Plugin for CanvasPlugin {
    fn build(&self, app: &mut App) {
        // sync_canvas_system is a no-op if CanvasResource is absent.
        // We do NOT panic on missing resource, to keep startup robust
        // when the host lazily inserts the resource mid-frame.
        app.add_systems(Update, crate::bevy_bridge::sync_canvas_system);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::CanvasNode;

    #[test]
    fn plugin_registers_and_runs_without_resource() {
        let mut app = App::new();
        app.add_plugins(CanvasPlugin);
        // No panic: sync_canvas_system is a no-op when CanvasResource
        // is missing.
        app.update();
    }

    #[test]
    fn plugin_syncs_single_node_into_ecs() {
        let canvas = Canvas::new("test");
        let id = canvas.add_node(CanvasNode::new(
            NodeKind::Block,
            Position::new(10, 20),
            "src",
        ));
        assert_eq!(canvas.nodes().len(), 1);

        let mut app = App::new();
        app.insert_resource(CanvasResource(Arc::new(canvas)));
        app.add_plugins(CanvasPlugin);
        app.update();

        let mut q = app
            .world_mut()
            .query::<(&CanvasNodeComp, &CanvasPositionComp)>();
        let items: Vec<_> = q.iter(app.world()).collect();
        assert_eq!(items.len(), 1, "expected one ECS entity per canvas node");
        assert_eq!(items[0].0.id, id);
        assert_eq!(items[0].0.label, "src");
        assert_eq!(items[0].1 .0, Position::new(10, 20));
    }
}
