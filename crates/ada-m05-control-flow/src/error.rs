//! Error surface for the control flow executor.
//!
//! [`ExecutorError`] is the single error type returned by
//! every public function in this crate. The v0.1.0 skeleton
//! keeps the enum at five variants covering the common
//! failure modes of an in-process step executor:
//!
//! | Variant                 | Trigger                                              |
//! |-------------------------|------------------------------------------------------|
//! | `StepNotFound`          | `next_step` references an id that does not exist.    |
//! | `ConditionError`        | A `Condition` could not be evaluated.                |
//! | `MaxRecursionExceeded`  | The execution depth exceeded the configured cap.     |
//! | `Timeout`               | The execution exceeded the configured time budget.   |
//! | `BackendError`          | Underlying store / driver / scheduler failed.        |
//!
//! See [`DOC-MOD-005`](../docs/modules/M-05-control-flow.md)
//! §3.4 for the full validation pipeline.

use thiserror::Error;

/// Failure modes surfaced by the control flow executor.
#[derive(Debug, Error)]
pub enum ExecutorError {
    /// The executor tried to follow a `next_step` pointer
    /// to an id that does not exist in the step table.
    #[error("step not found: {0}")]
    StepNotFound(String),

    /// A `Condition` could not be evaluated (e.g. a field
    /// path in the context was missing).
    #[error("condition error: {0}")]
    ConditionError(String),

    /// The execution depth exceeded the configured cap.
    /// The skeleton surfaces this as a distinct variant
    /// so callers can distinguish an infinite loop from
    /// a legitimate but deep run.
    #[error("max recursion exceeded: {0}")]
    MaxRecursionExceeded(String),

    /// The execution exceeded the configured time budget.
    /// The skeleton keeps the variant distinct from
    /// `MaxRecursionExceeded` so callers can pick the
    /// right backoff / restart policy.
    #[error("execution timed out after {0:?}")]
    Timeout(std::time::Duration),

    /// Underlying store / driver / scheduler failed.
    #[error("backend error: {0}")]
    BackendError(String),
}

/// `Result` alias for fallible executor operations.
pub type Result<T> = core::result::Result<T, ExecutorError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step_not_found_display() {
        let e = ExecutorError::StepNotFound("s-7".into());
        assert_eq!(e.to_string(), "step not found: s-7");
    }

    #[test]
    fn condition_error_display() {
        let e = ExecutorError::ConditionError("missing field".into());
        assert_eq!(e.to_string(), "condition error: missing field");
    }

    #[test]
    fn max_recursion_exceeded_display() {
        let e = ExecutorError::MaxRecursionExceeded("depth 100".into());
        assert_eq!(e.to_string(), "max recursion exceeded: depth 100");
    }

    #[test]
    fn timeout_display() {
        let e = ExecutorError::Timeout(std::time::Duration::from_millis(250));
        assert_eq!(e.to_string(), "execution timed out after 250ms");
    }

    #[test]
    fn backend_error_display() {
        let e = ExecutorError::BackendError("scheduler offline".into());
        assert_eq!(e.to_string(), "backend error: scheduler offline");
    }

    #[test]
    fn result_alias_carries_error() {
        let ok: Result<i32> = Ok(7);
        let err: Result<i32> = Err(ExecutorError::StepNotFound("x".into()));
        assert!(matches!(ok, Ok(7)));
        assert!(err.is_err());
    }

    #[test]
    fn error_is_send_sync_and_static() {
        fn assert_send_sync_static<E: std::error::Error + Send + Sync + 'static>(_: &E) {}
        let e = ExecutorError::BackendError("x".into());
        assert_send_sync_static(&e);
    }
}
