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

pub use canvas::{Canvas, Edge};
pub use error::{CanvasError, Result};
pub use history::{EditHistory, EditOp};
pub use node::{CanvasNode, NodeId, NodeKind, Port, Position};

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
