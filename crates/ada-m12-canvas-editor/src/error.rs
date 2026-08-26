//! Error surface for the M-12 canvas editor.

use thiserror::Error;

use crate::node::NodeId;

/// Failure modes surfaced by the canvas editor.
#[derive(Debug, Error)]
pub enum CanvasError {
    /// The node id is not in the canvas.
    #[error("node not found: {0}")]
    NodeNotFound(NodeId),

    /// The canvas was modified by another writer; the
    /// `expected_version` did not match.
    #[error("version conflict: expected {expected}, current {current}")]
    VersionConflict {
        /// What the caller expected.
        expected: u64,
        /// What the canvas actually has.
        current: u64,
    },

    /// The edge's endpoints are invalid (self-loop, missing
    /// endpoint, duplicate edge).
    #[error("invalid edge: {reason}")]
    InvalidEdge {
        /// Human-readable reason.
        reason: String,
    },

    /// An undo/redo was attempted on an empty stack.
    #[error("history empty")]
    HistoryEmpty,

    /// The backing store (file, DB, ...) failed.
    #[error("backend error: {0}")]
    BackendError(String),
}

/// `Result` alias for fallible canvas operations.
pub type Result<T> = core::result::Result<T, CanvasError>;

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn node_not_found_display() {
        let e = CanvasError::NodeNotFound(NodeId(Uuid::new_v4()));
        let s = e.to_string();
        assert!(s.starts_with("node not found: "), "got: {s}");
    }

    #[test]
    fn version_conflict_display() {
        let e = CanvasError::VersionConflict {
            expected: 3,
            current: 5,
        };
        assert_eq!(e.to_string(), "version conflict: expected 3, current 5");
    }

    #[test]
    fn invalid_edge_display() {
        let e = CanvasError::InvalidEdge {
            reason: "self-loop".into(),
        };
        assert_eq!(e.to_string(), "invalid edge: self-loop");
    }

    #[test]
    fn history_empty_display() {
        assert_eq!(CanvasError::HistoryEmpty.to_string(), "history empty");
    }

    #[test]
    fn backend_error_display() {
        let e = CanvasError::BackendError("file: not found".into());
        assert_eq!(e.to_string(), "backend error: file: not found");
    }
}
