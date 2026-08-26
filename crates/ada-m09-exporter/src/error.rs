//! Error surface for the metrics exporter.
//!
//! [`ExporterError`] is the single error type returned by every
//! public function in this crate. The v0.1.0 skeleton keeps the
//! enum at five variants covering the common failure modes of an
//! in-process exporter pipeline:
//!
//! | Variant              | Trigger                                                 |
//! |----------------------|---------------------------------------------------------|
//! | `SerializationError` | Metric / snapshot (de)serialization failed.             |
//! | `TransportError`     | The remote endpoint rejected the request.               |
//! | `InvalidMetric`      | The metric failed the in-process `validate` check.     |
//! | `BackendError`       | The underlying store (file, DB, TSDB) failed.           |
//! | `ShuttingDown`       | `export` was called on an exporter that was closed.     |
//!
//! See [`DOC-MOD-009`](../docs/modules/M-09-exporter.md) §3.4
//! for the full validation pipeline.

use thiserror::Error;

/// Failure modes surfaced by the metrics exporter.
#[derive(Debug, Error)]
pub enum ExporterError {
    /// Metric or snapshot (de)serialization failed (e.g. via
    /// `serde_json` or the OTLP protobuf).
    #[error("serialization error: {0}")]
    SerializationError(String),

    /// The remote endpoint rejected the request (HTTP 5xx,
    /// gRPC UNAVAILABLE, file write refused, ...).
    #[error("transport error: {0}")]
    TransportError(String),

    /// The metric failed the in-process `validate` check
    /// (empty name, NaN value, ...).
    #[error("invalid metric: {0}")]
    InvalidMetric(String),

    /// The underlying store failed.
    #[error("backend error: {0}")]
    BackendError(String),

    /// `export` was called on an exporter that was closed.
    #[error("exporter is shutting down")]
    ShuttingDown,
}

/// `Result` alias for fallible exporter operations.
pub type Result<T> = core::result::Result<T, ExporterError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialization_error_display() {
        let e = ExporterError::SerializationError("bad json".into());
        assert_eq!(e.to_string(), "serialization error: bad json");
    }

    #[test]
    fn transport_error_display() {
        let e = ExporterError::TransportError("HTTP 503".into());
        assert_eq!(e.to_string(), "transport error: HTTP 503");
    }

    #[test]
    fn invalid_metric_display() {
        let e = ExporterError::InvalidMetric("empty name".into());
        assert_eq!(e.to_string(), "invalid metric: empty name");
    }

    #[test]
    fn backend_error_display() {
        let e = ExporterError::BackendError("disk full".into());
        assert_eq!(e.to_string(), "backend error: disk full");
    }

    #[test]
    fn shutting_down_display() {
        let e = ExporterError::ShuttingDown;
        assert_eq!(e.to_string(), "exporter is shutting down");
    }

    #[test]
    fn result_alias_carries_error() {
        let ok: Result<i32> = Ok(7);
        let err: Result<i32> = Err(ExporterError::ShuttingDown);
        assert!(matches!(ok, Ok(7)));
        assert!(err.is_err());
    }

    #[test]
    fn error_is_send_sync_and_static() {
        fn assert_send_sync_static<E: std::error::Error + Send + Sync + 'static>(_: &E) {}
        let e = ExporterError::BackendError("x".into());
        assert_send_sync_static(&e);
    }
}
