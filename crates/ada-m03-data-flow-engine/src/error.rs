//! Error surface for the data flow engine.
//!
//! [`FlowError`] is the single error type returned by every
//! public function in this crate. The v0.1.0 skeleton keeps
//! the enum at five variants covering the common failure
//! modes of an in-process DAG executor:
//!
//! | Variant           | Trigger                                              |
//! |-------------------|------------------------------------------------------|
//! | `CyclicGraph`     | The DAG has a cycle (topological sort failed).       |
//! | `UnknownNode`     | An edge references a node id that does not exist.    |
//! | `ExecutionFailed` | A node body returned an error during `execute`.      |
//! | `TypeMismatch`    | A node expected a JSON type the input did not have.  |
//! | `BackendError`    | Underlying store / driver / scheduler failed.        |
//!
//! See [`DOC-MOD-003`](../docs/modules/M-03-data-flow-engine.md)
//! §3.4 for the full validation pipeline.

use thiserror::Error;

/// Failure modes surfaced by the data flow engine.
#[derive(Debug, Error)]
pub enum FlowError {
    /// The DAG has a cycle (topological sort failed).
    /// `path` is the offending back-edge, when known.
    #[error("cyclic graph: {path}")]
    CyclicGraph {
        /// Stringified cycle path (e.g. `"a -> b -> a"`).
        path: String,
    },

    /// An edge references a node id that does not exist in
    /// the flow's `nodes` map.
    #[error("unknown node: {0}")]
    UnknownNode(String),

    /// A node body returned an error during `execute`.
    #[error("execution failed at node {node}: {message}")]
    ExecutionFailed {
        /// Node id that failed.
        node: String,
        /// Human-readable error message.
        message: String,
    },

    /// A node expected a JSON type the input did not have
    /// (e.g. a numeric node received an object).
    #[error("type mismatch at node {node}: expected {expected}, got {actual}")]
    TypeMismatch {
        /// Node id where the mismatch happened.
        node: String,
        /// Expected JSON type (e.g. `"number"`).
        expected: &'static str,
        /// Actual JSON type.
        actual: String,
    },

    /// Underlying store / driver / scheduler failed.
    #[error("backend error: {0}")]
    BackendError(String),
}

/// `Result` alias for fallible data flow operations.
pub type Result<T> = core::result::Result<T, FlowError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cyclic_graph_display() {
        let e = FlowError::CyclicGraph {
            path: "a -> b -> a".into(),
        };
        assert_eq!(e.to_string(), "cyclic graph: a -> b -> a");
    }

    #[test]
    fn unknown_node_display() {
        let e = FlowError::UnknownNode("n-7".into());
        assert_eq!(e.to_string(), "unknown node: n-7");
    }

    #[test]
    fn execution_failed_display() {
        let e = FlowError::ExecutionFailed {
            node: "n-1".into(),
            message: "boom".into(),
        };
        assert_eq!(e.to_string(), "execution failed at node n-1: boom");
    }

    #[test]
    fn type_mismatch_display() {
        let e = FlowError::TypeMismatch {
            node: "n-1".into(),
            expected: "number",
            actual: "string".into(),
        };
        assert_eq!(
            e.to_string(),
            "type mismatch at node n-1: expected number, got string"
        );
    }

    #[test]
    fn backend_error_display() {
        let e = FlowError::BackendError("scheduler full".into());
        assert_eq!(e.to_string(), "backend error: scheduler full");
    }

    #[test]
    fn result_alias_carries_error() {
        let ok: Result<i32> = Ok(7);
        let err: Result<i32> = Err(FlowError::UnknownNode("x".into()));
        assert!(matches!(ok, Ok(7)));
        assert!(err.is_err());
    }

    #[test]
    fn error_is_send_sync_and_static() {
        fn assert_send_sync_static<E: std::error::Error + Send + Sync + 'static>(_: &E) {}
        let e = FlowError::BackendError("x".into());
        assert_send_sync_static(&e);
    }
}
