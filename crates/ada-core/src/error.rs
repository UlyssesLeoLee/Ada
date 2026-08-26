//! Top-level [`AdaError`] type used across the Ada workspace.
//!
//! This crate-level error is the canonical surface that other Ada crates
//! either re-export or wrap with their own narrower error enums
//! (see [`DOC-ARCH-007 §8`](https://example.invalid/docs/architecture/06-rust-tech-selection.md)
//! for the `thiserror` + `anyhow` split: `thiserror` for library
//! error enums, `anyhow` for application / `main` / scripts).
//!
//! The five variants cover the common failure modes a shared crate
//! must surface to upstream callers without leaking implementation
//! details of any specific module.

use thiserror::Error;

/// Top-level error type for `ada-core` and (by convention) other
/// crates that choose to re-export it.
///
/// All variants are `Send + Sync + 'static` thanks to the underlying
/// payload types (`String`, `&'static str`), which makes `AdaError`
/// safe to return from `async` tasks and to share across threads.
#[derive(Debug, Error)]
pub enum AdaError {
    /// Configuration error: missing or invalid setting, parse failure,
    /// or a value that violates an invariant of the shared layer.
    #[error("config error: {0}")]
    Config(String),

    /// Entity not found by id.
    ///
    /// `entity` is a short, statically-known name of the resource type
    /// (e.g. `"tenant"`, `"canvas"`); `id` is its stringified identifier.
    #[error("{entity} not found: {id}")]
    NotFound {
        /// Static resource type label.
        entity: &'static str,
        /// Stringified identifier that was looked up.
        id: String,
    },

    /// Caller supplied an invalid input or the system is in an
    /// invalid state to fulfil the request.
    #[error("invalid: {0}")]
    Invalid(String),

    /// Authentication or authorization failure (e.g. missing /
    /// expired token, insufficient permission).
    #[error("unauthorized: {0}")]
    Unauthorized(String),

    /// Internal / unexpected error that does not fit the categories
    /// above (e.g. invariant violation, post-condition failure).
    #[error("internal error: {0}")]
    Internal(String),
}

/// Result alias for `ada-core` operations.
pub type Result<T> = core::result::Result<T, AdaError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_variant_display() {
        let err = AdaError::Config("missing database.url".to_string());
        assert_eq!(err.to_string(), "config error: missing database.url");
    }

    #[test]
    fn not_found_variant_display() {
        let err = AdaError::NotFound {
            entity: "tenant",
            id: "t-42".to_string(),
        };
        assert_eq!(err.to_string(), "tenant not found: t-42");
    }

    #[test]
    fn invalid_variant_display() {
        let err = AdaError::Invalid("empty payload".to_string());
        assert_eq!(err.to_string(), "invalid: empty payload");
    }

    #[test]
    fn unauthorized_variant_display() {
        let err = AdaError::Unauthorized("token expired".to_string());
        assert_eq!(err.to_string(), "unauthorized: token expired");
    }

    #[test]
    fn internal_variant_display() {
        let err = AdaError::Internal("invariant broken".to_string());
        assert_eq!(err.to_string(), "internal error: invariant broken");
    }

    #[test]
    fn error_is_send_sync_and_static() {
        fn assert_send_sync_static<E: std::error::Error + Send + Sync + 'static>(_: &E) {}
        let err = AdaError::Internal("x".to_string());
        assert_send_sync_static(&err);
    }

    #[test]
    fn result_alias_works() {
        let ok: Result<i32> = Ok(7);
        let err: Result<i32> = Err(AdaError::Invalid("nope".to_string()));
        assert!(matches!(ok, Ok(7)));
        assert!(err.is_err());
    }
}
