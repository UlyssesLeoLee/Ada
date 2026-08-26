//! Error surface for the M-04 orchestrator.
//!
//! [`OrchError`] is the single error type returned by every
//! public function in this crate. The v0.1.0 skeleton keeps
//! the enum at five variants covering the common failure modes
//! of an in-process job scheduler:
//!
//! | Variant         | Trigger                                              |
//! |-----------------|------------------------------------------------------|
//! | `JobNotFound`   | `state_of` / `cancel` for a job id that is unknown.   |
//! | `InvalidState`  | A state-machine transition that is not allowed       |
//! |                 | (e.g. `Succeeded -> Running`).                       |
//! | `QueueFull`     | `enqueue` rejected because the queue is at capacity. |
//! | `BackendError`  | The backing store (Postgres, Redis, ...) failed.     |
//! | `Cancelled`     | A worker / API caller explicitly cancelled the job.   |
//!
//! Production builds will map these to the canonical API
//! error codes defined in `docs/api/error-codes.md`; the
//! skeleton keeps the surface minimal. See
//! [`DOC-MOD-004`](../docs/modules/M-04-orchestration.md)
//! §3.4 for the full validation pipeline.

use thiserror::Error;

use crate::job::JobId;

/// Failure modes surfaced by the orchestrator.
#[derive(Debug, Error)]
pub enum OrchError {
    /// The job id was not found in the scheduler.
    #[error("job not found: {0}")]
    JobNotFound(JobId),

    /// A state-machine transition was attempted from a state
    /// that does not allow it (e.g. `Succeeded -> Running`,
    /// `Cancelled -> Queued`). Carries the offending `from`
    /// and `to` states for diagnostics.
    #[error("invalid state transition: from {from} to {to}")]
    InvalidState {
        /// The state the job was in.
        from: String,
        /// The state the caller tried to move the job to.
        to: String,
    },

    /// `enqueue` was rejected because the queue was at
    /// `capacity`. The v0.1.0 default is
    /// [`super::DEFAULT_QUEUE_CAPACITY`].
    #[error("queue full (capacity {capacity})")]
    QueueFull {
        /// Configured maximum queue depth.
        capacity: usize,
    },

    /// The backing store (Postgres, Redis, ...) failed.
    #[error("backend error: {0}")]
    BackendError(String),

    /// The job was explicitly cancelled by a worker or API
    /// caller. Surfaced to distinguish a normal cancel from a
    /// mid-flight `InvalidState` failure.
    #[error("job cancelled: {0}")]
    Cancelled(JobId),
}

/// `Result` alias for fallible orchestrator operations.
pub type Result<T> = core::result::Result<T, OrchError>;

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn job_id() -> JobId {
        JobId(Uuid::new_v4())
    }

    #[test]
    fn job_not_found_display() {
        let e = OrchError::JobNotFound(job_id());
        let s = e.to_string();
        assert!(s.starts_with("job not found: "), "got: {s}");
    }

    #[test]
    fn invalid_state_display() {
        let e = OrchError::InvalidState {
            from: "Succeeded".into(),
            to: "Running".into(),
        };
        assert_eq!(
            e.to_string(),
            "invalid state transition: from Succeeded to Running"
        );
    }

    #[test]
    fn queue_full_display() {
        let e = OrchError::QueueFull { capacity: 1024 };
        assert_eq!(e.to_string(), "queue full (capacity 1024)");
    }

    #[test]
    fn backend_error_display() {
        let e = OrchError::BackendError("pg: connection refused".into());
        assert_eq!(e.to_string(), "backend error: pg: connection refused");
    }

    #[test]
    fn cancelled_display() {
        let e = OrchError::Cancelled(job_id());
        let s = e.to_string();
        assert!(s.starts_with("job cancelled: "), "got: {s}");
    }

    #[test]
    fn result_alias_carries_error() {
        let ok: Result<i32> = Ok(7);
        let err: Result<i32> = Err(OrchError::BackendError("x".into()));
        assert!(matches!(ok, Ok(7)));
        assert!(err.is_err());
    }

    #[test]
    fn error_is_send_sync_and_static() {
        fn assert_send_sync_static<E: std::error::Error + Send + Sync + 'static>(_: &E) {}
        let e = OrchError::BackendError("x".into());
        assert_send_sync_static(&e);
    }
}
