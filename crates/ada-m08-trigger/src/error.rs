//! Error surface for the M-08 trigger crate.

use thiserror::Error;

use crate::rule::TriggerId;

/// Failure modes surfaced by the trigger manager.
#[derive(Debug, Error)]
pub enum TriggerError {
    /// The trigger id is not registered with the manager.
    #[error("trigger not found: {0}")]
    TriggerNotFound(TriggerId),

    /// The cron expression is malformed. The skeleton accepts
    /// exactly 5 whitespace-separated fields; any other shape
    /// is rejected.
    #[error("invalid cron: {0}")]
    InvalidCron(String),

    /// The action attached to a trigger returned an error
    /// (e.g. the downstream API was unreachable).
    #[error("action failed: {0}")]
    ActionFailed(String),

    /// The caller tried to add a trigger with an id that
    /// already exists in the manager.
    #[error("duplicate trigger id: {0}")]
    DuplicateId(TriggerId),

    /// The backing store (DB, scheduler, ...) failed.
    #[error("backend error: {0}")]
    BackendError(String),
}

/// `Result` alias for fallible trigger operations.
pub type Result<T> = core::result::Result<T, TriggerError>;

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn trigger_not_found_display() {
        let e = TriggerError::TriggerNotFound(TriggerId(Uuid::new_v4()));
        let s = e.to_string();
        assert!(s.starts_with("trigger not found: "), "got: {s}");
    }

    #[test]
    fn invalid_cron_display() {
        let e = TriggerError::InvalidCron("only 3 fields".into());
        assert_eq!(e.to_string(), "invalid cron: only 3 fields");
    }

    #[test]
    fn action_failed_display() {
        let e = TriggerError::ActionFailed("http: 502".into());
        assert_eq!(e.to_string(), "action failed: http: 502");
    }

    #[test]
    fn duplicate_id_display() {
        let e = TriggerError::DuplicateId(TriggerId(Uuid::new_v4()));
        let s = e.to_string();
        assert!(s.starts_with("duplicate trigger id: "), "got: {s}");
    }

    #[test]
    fn backend_error_display() {
        let e = TriggerError::BackendError("pg: down".into());
        assert_eq!(e.to_string(), "backend error: pg: down");
    }
}
