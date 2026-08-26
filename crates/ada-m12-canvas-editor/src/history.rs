//! Edit op enum and linear undo/redo stack.

use serde::{Deserialize, Serialize};

use crate::canvas::Edge;
use crate::error::{CanvasError, Result};
use crate::node::{NodeId, Position};

/// One edit operation. The v0.1.0 skeleton does not carry
/// reverse-applied state; undo simply re-issues the inverse op
/// against the [`crate::Canvas`] it was originally applied to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EditOp {
    /// Insert a new node.
    InsertNode {
        /// The new node's id.
        id: NodeId,
        /// Original kind.
        kind: crate::node::NodeKind,
        /// Original position.
        position: Position,
        /// Original label.
        label: String,
    },
    /// Remove a node.
    RemoveNode {
        /// The removed node's id.
        id: NodeId,
    },
    /// Move a node.
    MoveNode {
        /// The node's id.
        id: NodeId,
        /// The new position.
        new_position: Position,
    },
    /// Add a new edge.
    AddEdge(Edge),
}

/// Linear undo/redo history. Pushing a new op clears the redo
/// stack (the standard "branching" behaviour).
#[derive(Debug, Default)]
pub struct EditHistory {
    undo: Vec<EditOp>,
    redo: Vec<EditOp>,
}

impl EditHistory {
    /// Create an empty history.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a new op. Discards any pending redo ops.
    pub fn push(&mut self, op: EditOp) {
        self.undo.push(op);
        self.redo.clear();
    }

    /// Pop the most recent op for undoing. Returns the op; the
    /// caller is responsible for reversing it.
    pub fn undo(&mut self) -> Result<EditOp> {
        let op = self.undo.pop().ok_or(CanvasError::HistoryEmpty)?;
        self.redo.push(op.clone());
        Ok(op)
    }

    /// Re-apply the most recently undone op. Returns the op.
    pub fn redo(&mut self) -> Result<EditOp> {
        let op = self.redo.pop().ok_or(CanvasError::HistoryEmpty)?;
        self.undo.push(op.clone());
        Ok(op)
    }

    /// Number of pending undo ops.
    #[must_use]
    pub fn undo_len(&self) -> usize {
        self.undo.len()
    }

    /// Number of pending redo ops.
    #[must_use]
    pub fn redo_len(&self) -> usize {
        self.redo.len()
    }

    /// `true` if there are no pending undo or redo ops.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.undo.is_empty() && self.redo.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::NodeKind;
    use uuid::Uuid;

    #[test]
    fn empty_history_undo_errors() {
        let mut h = EditHistory::new();
        let err = h.undo().unwrap_err();
        assert!(matches!(err, CanvasError::HistoryEmpty));
    }

    #[test]
    fn empty_history_redo_errors() {
        let mut h = EditHistory::new();
        let err = h.redo().unwrap_err();
        assert!(matches!(err, CanvasError::HistoryEmpty));
    }

    #[test]
    fn push_then_undo_then_redo_round_trip() {
        let mut h = EditHistory::new();
        let op = EditOp::InsertNode {
            id: NodeId(Uuid::new_v4()),
            kind: NodeKind::Block,
            position: Position::new(0, 0),
            label: "x".into(),
        };
        h.push(op.clone());
        assert_eq!(h.undo_len(), 1);
        assert_eq!(h.redo_len(), 0);
        let undone = h.undo().expect("undo");
        assert_eq!(undone, op);
        assert_eq!(h.undo_len(), 0);
        assert_eq!(h.redo_len(), 1);
        let redone = h.redo().expect("redo");
        assert_eq!(redone, op);
        assert_eq!(h.undo_len(), 1);
        assert_eq!(h.redo_len(), 0);
    }

    #[test]
    fn push_after_undo_clears_redo() {
        let mut h = EditHistory::new();
        h.push(EditOp::RemoveNode {
            id: NodeId(Uuid::new_v4()),
        });
        h.undo().expect("undo");
        assert_eq!(h.redo_len(), 1);
        h.push(EditOp::RemoveNode {
            id: NodeId(Uuid::new_v4()),
        });
        assert!(h.redo.is_empty(), "expected redo to be cleared");
    }

    #[test]
    fn is_empty_reflects_state() {
        let mut h = EditHistory::new();
        assert!(h.is_empty());
        h.push(EditOp::RemoveNode {
            id: NodeId(Uuid::new_v4()),
        });
        assert!(!h.is_empty());
    }
}
