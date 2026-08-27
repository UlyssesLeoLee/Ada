//! Error types for the `ada-telemetry` crate.
//!
//! All public APIs that can fail return [`Result<T, TelemetryError>`].
//! The variants are deliberately coarse-grained: this crate is
//! observability plumbing, not business logic, and a one-line
//! `match` on the variant is enough for any caller.

use thiserror::Error;

/// Result alias for `ada-telemetry` operations.
pub type Result<T> = core::result::Result<T, TelemetryError>;

/// Errors that can occur while initialising, running, or
/// shutting down the telemetry pipeline.
///
/// The enum is `#[non_exhaustive]` so we can add new failure
/// modes (e.g. a `PrometheusPortInUse` variant) without breaking
/// downstream `match` blocks that don't have a wildcard arm.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum TelemetryError {
    /// [`crate::init`] was called more than once in the same
    /// process. The global tracing subscriber can only be
    /// installed once; subsequent calls are programmer errors.
    #[error("telemetry already initialised (telemetry::init is single-shot per process)")]
    AlreadyInitialised,

    /// `tracing-subscriber::EnvFilter` could not parse the
    /// `RUST_LOG` / override directive.
    #[error("invalid env-filter directive `{directive}`: {source}")]
    InvalidEnvFilter {
        /// The directive that failed to parse.
        directive: String,
        /// The underlying parse error.
        #[source]
        source: tracing_subscriber::filter::ParseError,
    },

    /// `tracing_subscriber::fmt::SubscriberBuilder` failed
    /// to install itself in the global registry.
    #[error("failed to install tracing subscriber: {0}")]
    SubscriberInit(#[source] Box<dyn core::error::Error + Send + Sync>),

    /// The OpenTelemetry OTLP exporter builder returned an
    /// error (e.g. invalid endpoint URL, DNS failure).
    #[error("OTLP exporter initialisation failed: {0}")]
    OtlpExporter(#[source] Box<dyn core::error::Error + Send + Sync>),

    /// The Prometheus HTTP listener could not bind to the
    /// configured port (e.g. address already in use).
    #[error("Prometheus listener bind failed on `{addr}`: {source}")]
    PrometheusBind {
        /// The address we tried to bind to.
        addr: String,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// The OTLP endpoint string was malformed.
    #[error("invalid OTLP endpoint URL `{url}`: {reason}")]
    InvalidEndpoint {
        /// The URL the caller passed in.
        url: String,
        /// Why we rejected it.
        reason: &'static str,
    },

    /// A guard was dropped in a context that expected the
    /// pipeline to be live. This is a programmer error.
    #[error("telemetry guard dropped while still in use")]
    GuardPoisoned,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn already_initialised_displays() {
        let e = TelemetryError::AlreadyInitialised;
        assert!(e.to_string().contains("already initialised"));
    }

    #[test]
    fn invalid_endpoint_displays_url() {
        let e = TelemetryError::InvalidEndpoint {
            url: "not-a-url".to_string(),
            reason: "missing scheme",
        };
        let s = e.to_string();
        assert!(s.contains("not-a-url"));
        assert!(s.contains("missing scheme"));
    }

    #[test]
    fn guard_poisoned_displays() {
        let e = TelemetryError::GuardPoisoned;
        assert!(e.to_string().contains("guard dropped"));
    }

    #[test]
    fn result_alias_works() {
        let ok: Result<u32> = Ok(7);
        let err: Result<u32> = Err(TelemetryError::GuardPoisoned);
        assert!(matches!(ok, Ok(7)));
        assert!(err.is_err());
    }

    #[test]
    fn error_implements_debug_and_error() {
        // Compile-time check that the auto-derive for `Error`
        // produces a usable trait object.
        let e: Box<dyn std::error::Error + Send + Sync> = TelemetryError::InvalidEndpoint {
            url: "x".to_string(),
            reason: "test",
        }
        .into();
        assert!(!e.to_string().is_empty());
    }
}
