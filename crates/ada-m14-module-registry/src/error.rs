//! Error surface for the module registry.
//!
//! [`RegistryError`] is the single error type returned by every
//! public function in this crate. The v0.1.0 skeleton keeps the
//! enum at five variants covering the common failure modes of an
//! in-process module registry:
//!
//! | Variant              | Trigger                                                 |
//! |----------------------|---------------------------------------------------------|
//! | `AlreadyRegistered`  | `register` called for a name that is already present.   |
//! | `NotFound`           | `get` / `deregister` / `heartbeat` for unknown name.    |
//! | `InvalidDescriptor`  | The descriptor failed `ModuleDescriptor::validate`.     |
//! | `HealthCheckFailed`  | A `heartbeat` carried a non-`Healthy` state and the     |
//! |                      | registry policy chose to reject the update.             |
//! | `BackendError`       | The optional event-bus publish failed.                  |
//!
//! See [`DOC-MOD-014`](../docs/modules/M-14-module-registry.md)
//! §3.4 for the full validation pipeline.

use thiserror::Error;

/// Failure modes surfaced by the module registry.
#[derive(Debug, Error)]
pub enum RegistryError {
    /// `register` was called for a name that is already present.
    #[error("module already registered: {0}")]
    AlreadyRegistered(String),

    /// `get` / `deregister` / `heartbeat` for an unknown name.
    #[error("module not found: {0}")]
    NotFound(String),

    /// The descriptor failed the in-process `validate` check
    /// (empty name, empty endpoint, malformed version, ...).
    #[error("invalid descriptor: {0}")]
    InvalidDescriptor(String),

    /// A `heartbeat` carried a non-`Healthy` state and the
    /// registry policy chose to reject the update. The skeleton
    /// only rejects when the new state is `Unhealthy` (the
    /// most disruptive); other states are accepted.
    #[error("health check failed: {0}")]
    HealthCheckFailed(String),

    /// The optional event-bus publish failed.
    #[error("backend error: {0}")]
    BackendError(String),
}

/// `Result` alias for fallible registry operations.
pub type Result<T> = core::result::Result<T, RegistryError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn already_registered_display() {
        let e = RegistryError::AlreadyRegistered("mod-ingest-csv".into());
        assert_eq!(e.to_string(), "module already registered: mod-ingest-csv");
    }

    #[test]
    fn not_found_display() {
        let e = RegistryError::NotFound("mod-sink-s3".into());
        assert_eq!(e.to_string(), "module not found: mod-sink-s3");
    }

    #[test]
    fn invalid_descriptor_display() {
        let e = RegistryError::InvalidDescriptor("empty name".into());
        assert_eq!(e.to_string(), "invalid descriptor: empty name");
    }

    #[test]
    fn health_check_failed_display() {
        let e = RegistryError::HealthCheckFailed("upstream down".into());
        assert_eq!(e.to_string(), "health check failed: upstream down");
    }

    #[test]
    fn backend_error_display() {
        let e = RegistryError::BackendError("bus closed".into());
        assert_eq!(e.to_string(), "backend error: bus closed");
    }

    #[test]
    fn result_alias_carries_error() {
        let ok: Result<i32> = Ok(7);
        let err: Result<i32> = Err(RegistryError::NotFound("x".into()));
        assert!(matches!(ok, Ok(7)));
        assert!(err.is_err());
    }

    #[test]
    fn error_is_send_sync_and_static() {
        fn assert_send_sync_static<E: std::error::Error + Send + Sync + 'static>(_: &E) {}
        let e = RegistryError::BackendError("x".into());
        assert_send_sync_static(&e);
    }
}
