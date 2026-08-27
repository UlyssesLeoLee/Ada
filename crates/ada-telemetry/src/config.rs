//! Configuration for the telemetry pipeline.
//!
//! [`TelemetryConfig`] is the single point of entry for
//! initialising the logging / tracing / metrics stack. Every
//! field has a sane default derived from environment variables
//! and workspace conventions, so the common "just give me JSON
//! logs to stdout" case is
//!
//! ```no_run
//! use ada_telemetry::{TelemetryConfig, init};
//!
//! let cfg = TelemetryConfig::from_env("ada-telemetry");
//! let _guard = init(cfg).expect("init");
//! ```
//!
//! Per [`DOC-OBS-002 §3`](../docs/observability/02-architecture.md)
//! the endpoint defaults to `http://localhost:4317` (the
//! otel-collector gRPC port). Override with the
//! `OTEL_EXPORTER_OTLP_ENDPOINT` env var or the
//! [`TelemetryConfig::with_endpoint`] builder method.

use std::env;

use serde::{Deserialize, Serialize};

use crate::error::{Result, TelemetryError};

/// Default OTLP gRPC endpoint (otel-collector default port).
pub const DEFAULT_OTLP_ENDPOINT: &str = "http://localhost:4317";

/// Default Prometheus bind address.
pub const DEFAULT_PROMETHEUS_ADDR: &str = "127.0.0.1:9090";

/// Default environment label when `ADA_ENV` is not set.
pub const DEFAULT_ENVIRONMENT: &str = "development";

/// Output format for log records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    /// JSON Lines, one event per line. Recommended for
    /// production (machine-readable, Loki-friendly).
    #[default]
    Json,
    /// Human-readable pretty output, for `cargo run` /
    /// `cargo test` development.
    Pretty,
}

impl LogFormat {
    /// Parse from the `ADA_LOG_FORMAT` env var, falling back
    /// to [`LogFormat::Json`].
    #[must_use]
    pub fn from_env() -> Self {
        match env::var("ADA_LOG_FORMAT")
            .unwrap_or_default()
            .to_lowercase()
            .as_str()
        {
            "pretty" | "human" | "text" => Self::Pretty,
            _ => Self::Json,
        }
    }
}

/// Sampling strategy for trace export.
///
/// v0.2.0 ships a single strategy: a constant ratio in
/// `[0.0, 1.0]`. More sophisticated strategies
/// (e.g. [`ParentBased`][pb]) will land in v0.3.0.
///
/// [pb]: https://docs.rs/opentelemetry/0.32/opentelemetry/trace/enum.SamplingDecision.html
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(into = "f64", from = "f64")]
pub struct SampleRatio(f64);

impl SampleRatio {
    /// Always sample (100%).
    pub const ALL: Self = Self(1.0);
    /// Never sample (0%).
    pub const NONE: Self = Self(0.0);

    /// Construct a ratio, clamping to `[0.0, 1.0]`.
    #[must_use]
    pub fn new(value: f64) -> Self {
        Self(value.clamp(0.0, 1.0))
    }

    /// Underlying ratio, in `[0.0, 1.0]`.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }

    /// `true` iff every trace should be sampled.
    #[must_use]
    pub const fn is_full(self) -> bool {
        self.0 >= 1.0
    }
}

impl Default for SampleRatio {
    fn default() -> Self {
        Self::ALL
    }
}

impl From<f64> for SampleRatio {
    fn from(value: f64) -> Self {
        Self::new(value)
    }
}

impl From<SampleRatio> for f64 {
    fn from(s: SampleRatio) -> Self {
        s.0
    }
}

/// Full configuration for [`crate::init`].
///
/// Construct via the typed builder methods
/// ([`TelemetryConfig::with_endpoint`],
/// [`TelemetryConfig::with_service_name`], …) or
/// from environment with [`TelemetryConfig::from_env`].
///
/// All fields are public for ergonomic `..Default::default()`
/// construction. The struct is `Clone` so configs can be
/// re-used across tests.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TelemetryConfig {
    /// `service.name` resource attribute (`OTel` semantic
    /// convention). Defaults to the value passed to
    /// [`TelemetryConfig::new`].
    pub service_name: String,

    /// `service.version` resource attribute. Defaults to the
    /// `CARGO_PKG_VERSION` of the calling crate at
    /// `TelemetryConfig::new` time.
    pub service_version: String,

    /// `deployment.environment` resource attribute.
    /// `production` / `staging` / `development`.
    pub environment: String,

    /// OTLP gRPC endpoint.
    pub otlp_endpoint: String,

    /// Prometheus bind address. Used only when the
    /// `prometheus` feature is enabled.
    pub prometheus_addr: String,

    /// Whether the OTLP trace pipeline is enabled.
    pub tracing_enabled: bool,

    /// Whether the Prometheus pull endpoint is enabled.
    pub metrics_enabled: bool,

    /// Trace sampling ratio.
    pub sample_ratio: SampleRatio,

    /// Log record format.
    pub log_format: LogFormat,

    /// Override for the `tracing_subscriber::EnvFilter`
    /// directive. `None` ⇒ use `RUST_LOG` env var with a
    /// sensible default of `info`.
    pub env_filter: Option<String>,
}

impl TelemetryConfig {
    /// Construct a new config with the given service name.
    /// All other fields get the documented defaults.
    #[must_use]
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
            service_version: env!("CARGO_PKG_VERSION").to_string(),
            environment: env::var("ADA_ENV").unwrap_or_else(|_| DEFAULT_ENVIRONMENT.to_string()),
            otlp_endpoint: env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
                .unwrap_or_else(|_| DEFAULT_OTLP_ENDPOINT.to_string()),
            prometheus_addr: env::var("ADA_PROMETHEUS_ADDR")
                .unwrap_or_else(|_| DEFAULT_PROMETHEUS_ADDR.to_string()),
            tracing_enabled: true,
            metrics_enabled: cfg!(feature = "prometheus"),
            sample_ratio: SampleRatio::ALL,
            log_format: LogFormat::from_env(),
            env_filter: None,
        }
    }

    /// Read configuration entirely from environment variables
    /// (with the same defaults as [`TelemetryConfig::new`]).
    /// Equivalent to `TelemetryConfig::new(service_name)`
    /// followed by `apply_env_overrides` — the two-step
    /// pattern is preserved so the builder is still
    /// programmable from Rust code.
    #[must_use]
    pub fn from_env(service_name: impl Into<String>) -> Self {
        let mut cfg = Self::new(service_name);
        cfg.apply_env_overrides();
        cfg
    }

    /// Re-read all `OTEL_*` / `ADA_*` env vars and overwrite
    /// the corresponding fields. Existing values are kept
    /// where no env var is set.
    pub fn apply_env_overrides(&mut self) {
        if let Ok(v) = env::var("OTEL_EXPORTER_OTLP_ENDPOINT") {
            self.otlp_endpoint = v;
        }
        if let Ok(v) = env::var("ADA_PROMETHEUS_ADDR") {
            self.prometheus_addr = v;
        }
        if let Ok(v) = env::var("ADA_ENV") {
            self.environment = v;
        }
        if let Ok(v) = env::var("ADA_LOG_FORMAT") {
            self.log_format = match v.to_lowercase().as_str() {
                "pretty" | "human" | "text" => LogFormat::Pretty,
                _ => LogFormat::Json,
            };
        }
        if let Ok(v) = env::var("RUST_LOG") {
            self.env_filter = Some(v);
        }
    }

    /// Override the OTLP endpoint.
    #[must_use]
    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.otlp_endpoint = endpoint.into();
        self
    }

    /// Override the service name (replacing the one passed to
    /// `new` / `from_env`).
    #[must_use]
    pub fn with_service_name(mut self, name: impl Into<String>) -> Self {
        self.service_name = name.into();
        self
    }

    /// Override the service version.
    #[must_use]
    pub fn with_service_version(mut self, version: impl Into<String>) -> Self {
        self.service_version = version.into();
        self
    }

    /// Override the environment label.
    #[must_use]
    pub fn with_environment(mut self, env: impl Into<String>) -> Self {
        self.environment = env.into();
        self
    }

    /// Override the Prometheus bind address.
    #[must_use]
    pub fn with_prometheus_addr(mut self, addr: impl Into<String>) -> Self {
        self.prometheus_addr = addr.into();
        self
    }

    /// Toggle the OTLP trace pipeline on/off.
    #[must_use]
    pub const fn with_tracing_enabled(mut self, enabled: bool) -> Self {
        self.tracing_enabled = enabled;
        self
    }

    /// Toggle the Prometheus pipeline on/off.
    #[must_use]
    pub const fn with_metrics_enabled(mut self, enabled: bool) -> Self {
        self.metrics_enabled = enabled;
        self
    }

    /// Override the trace sample ratio.
    #[must_use]
    pub const fn with_sample_ratio(mut self, ratio: SampleRatio) -> Self {
        self.sample_ratio = ratio;
        self
    }

    /// Override the log format.
    #[must_use]
    pub const fn with_log_format(mut self, format: LogFormat) -> Self {
        self.log_format = format;
        self
    }

    /// Override the `EnvFilter` directive.
    #[must_use]
    pub fn with_env_filter(mut self, directive: impl Into<String>) -> Self {
        self.env_filter = Some(directive.into());
        self
    }

    /// Validate that the OTLP endpoint is a syntactically
    /// reasonable URL. Returns [`TelemetryError::InvalidEndpoint`]
    /// when the URL is missing a scheme, an empty host, or
    /// contains a NUL byte.
    ///
    /// A successful return does **not** mean the endpoint is
    /// reachable — DNS / TCP failures happen later inside the
    /// OTLP exporter.
    pub fn validate(&self) -> Result<()> {
        let url = &self.otlp_endpoint;
        if url.contains('\0') {
            return Err(TelemetryError::InvalidEndpoint {
                url: url.clone(),
                reason: "contains NUL byte",
            });
        }
        // Cheap structural check before the SDK does its own
        // parsing: must contain `://`, the scheme must be ASCII
        // letters, and the part after `://` must be non-empty.
        let Some((scheme, rest)) = url.split_once("://") else {
            return Err(TelemetryError::InvalidEndpoint {
                url: url.clone(),
                reason: "missing `://`",
            });
        };
        if scheme.is_empty()
            || !scheme
                .chars()
                .all(|c| c.is_ascii_alphabetic() || c == '+' || c == '-' || c == '.')
        {
            return Err(TelemetryError::InvalidEndpoint {
                url: url.clone(),
                reason: "scheme must be RFC 3986 alpha / `+` / `-` / `.`",
            });
        }
        let host = rest
            .split(['/', '?', '#'])
            .next()
            .unwrap_or("")
            .trim_end_matches(':');
        if host.is_empty() {
            return Err(TelemetryError::InvalidEndpoint {
                url: url.clone(),
                reason: "host portion is empty",
            });
        }
        Ok(())
    }
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self::new("ada-telemetry")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::float_cmp)] // exact bounds checks; clamp contract is bit-exact
    fn sample_ratio_clamps() {
        assert_eq!(SampleRatio::new(2.0).get(), 1.0);
        assert_eq!(SampleRatio::new(-0.5).get(), 0.0);
        assert_eq!(SampleRatio::new(0.42).get(), 0.42);
    }

    #[test]
    fn sample_ratio_constants() {
        assert!(SampleRatio::ALL.is_full());
        assert!(!SampleRatio::NONE.is_full());
    }

    #[test]
    fn sample_ratio_serde_round_trip() {
        let r = SampleRatio::new(0.25);
        let json = serde_json::to_string(&r).expect("serialize");
        let back: SampleRatio = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(r, back);
    }

    #[test]
    fn log_format_default_is_json() {
        assert_eq!(LogFormat::default(), LogFormat::Json);
    }

    #[test]
    fn log_format_serde_round_trip() {
        for fmt in [LogFormat::Json, LogFormat::Pretty] {
            let json = serde_json::to_string(&fmt).expect("serialize");
            let back: LogFormat = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(fmt, back);
        }
    }

    #[test]
    fn config_validate_accepts_default_endpoint() {
        let cfg = TelemetryConfig::default();
        cfg.validate().expect("default endpoint should validate");
    }

    #[test]
    fn config_validate_rejects_missing_scheme() {
        let cfg = TelemetryConfig::default().with_endpoint("localhost:4317");
        let err = cfg.validate().unwrap_err();
        assert!(matches!(
            err,
            TelemetryError::InvalidEndpoint {
                reason: "missing `://`",
                ..
            }
        ));
    }

    #[test]
    fn config_validate_rejects_empty_host() {
        let cfg = TelemetryConfig::default().with_endpoint("http:///path");
        let err = cfg.validate().unwrap_err();
        assert!(matches!(
            err,
            TelemetryError::InvalidEndpoint {
                reason: "host portion is empty",
                ..
            }
        ));
    }

    #[test]
    fn config_validate_rejects_nul_byte() {
        let cfg = TelemetryConfig::default().with_endpoint("http://local\0host");
        let err = cfg.validate().unwrap_err();
        assert!(matches!(
            err,
            TelemetryError::InvalidEndpoint {
                reason: "contains NUL byte",
                ..
            }
        ));
    }

    #[test]
    fn config_builder_overrides_take_precedence() {
        let cfg = TelemetryConfig::new("svc")
            .with_endpoint("http://otel:4317")
            .with_environment("staging")
            .with_sample_ratio(SampleRatio::new(0.1))
            .with_log_format(LogFormat::Pretty);
        assert_eq!(cfg.otlp_endpoint, "http://otel:4317");
        assert_eq!(cfg.environment, "staging");
        // Avoid `clippy::float_cmp` by comparing with an
        // epsilon — the configured sample ratio is read
        // straight back through the same `SampleRatio::new`.
        assert!((cfg.sample_ratio.get() - 0.1).abs() < f64::EPSILON);
        assert_eq!(cfg.log_format, LogFormat::Pretty);
    }

    #[test]
    fn config_clone_is_independent() {
        let cfg = TelemetryConfig::new("svc");
        let mut cfg2 = cfg.clone();
        cfg2.service_name = "other".to_string();
        assert_eq!(cfg.service_name, "svc");
        assert_eq!(cfg2.service_name, "other");
    }
}
