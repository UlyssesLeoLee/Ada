//! Error surface for the data-acquisition adapters.
//!
//! [`AcquisitionError`] is the single error type returned by every
//! public function in this crate. The v0.1.0 skeleton keeps the
//! enum at five variants covering the common failure modes of an
//! in-process acquisition pipeline:
//!
//! | Variant               | Trigger                                                |
//! |-----------------------|--------------------------------------------------------|
//! | `SourceUnavailable`   | The remote source is unreachable (DNS, TCP, HTTP 5xx). |
//! | `AuthenticationFailed`| Credentials were rejected (HTTP 401/403, MQTT auth).  |
//! | `RateLimited`         | The source returned HTTP 429 / quota exceeded.        |
//! | `InvalidPayload`      | The payload could not be decoded into `RawRecord`s.    |
//! | `BackendError`        | Underlying store / driver / file system failed.        |
//!
//! See [`DOC-MOD-001`](../docs/modules/M-01-acquisition.md) §3.4
//! for the full validation pipeline.

use thiserror::Error;

/// Failure modes surfaced by the acquisition adapters.
#[derive(Debug, Error)]
pub enum AcquisitionError {
    /// The remote source is unreachable (DNS, TCP refused,
    /// HTTP 5xx, broker offline, ...).
    #[error("source unavailable: {0}")]
    SourceUnavailable(String),

    /// Credentials were rejected (HTTP 401 / 403, MQTT
    /// authentication failed, S3 access denied, ...).
    #[error("authentication failed: {0}")]
    AuthenticationFailed(String),

    /// The source returned HTTP 429 or reported a quota /
    /// rate-limit error. The skeleton treats this as a
    /// distinct variant so the caller can apply a backoff
    /// policy without parsing the message string.
    #[error("rate limited: {0}")]
    RateLimited(String),

    /// The payload could not be decoded into `RawRecord`s
    /// (malformed JSON, unexpected schema, ...).
    #[error("invalid payload: {0}")]
    InvalidPayload(String),

    /// The underlying store / driver / file system failed.
    /// Used for `std::io` errors and any DB-driver-level
    /// failure the skeleton does not classify more specifically.
    #[error("backend error: {0}")]
    BackendError(String),
}

/// `Result` alias for fallible acquisition operations.
pub type Result<T> = core::result::Result<T, AcquisitionError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_unavailable_display() {
        let e = AcquisitionError::SourceUnavailable("connection refused".into());
        assert_eq!(e.to_string(), "source unavailable: connection refused");
    }

    #[test]
    fn authentication_failed_display() {
        let e = AcquisitionError::AuthenticationFailed("HTTP 401".into());
        assert_eq!(e.to_string(), "authentication failed: HTTP 401");
    }

    #[test]
    fn rate_limited_display() {
        let e = AcquisitionError::RateLimited("HTTP 429".into());
        assert_eq!(e.to_string(), "rate limited: HTTP 429");
    }

    #[test]
    fn invalid_payload_display() {
        let e = AcquisitionError::InvalidPayload("malformed JSON".into());
        assert_eq!(e.to_string(), "invalid payload: malformed JSON");
    }

    #[test]
    fn backend_error_display() {
        let e = AcquisitionError::BackendError("disk full".into());
        assert_eq!(e.to_string(), "backend error: disk full");
    }

    #[test]
    fn result_alias_carries_error() {
        let ok: Result<i32> = Ok(7);
        let err: Result<i32> = Err(AcquisitionError::SourceUnavailable("x".into()));
        assert!(matches!(ok, Ok(7)));
        assert!(err.is_err());
    }

    #[test]
    fn error_is_send_sync_and_static() {
        fn assert_send_sync_static<E: std::error::Error + Send + Sync + 'static>(_: &E) {}
        let e = AcquisitionError::BackendError("x".into());
        assert_send_sync_static(&e);
    }
}
