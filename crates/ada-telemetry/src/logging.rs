//! Logging pipeline.
//!
//! This module owns the `tracing_subscriber::fmt` layer — the
//! piece every binary needs to write structured logs to stdout.
//! It is feature-flag-agnostic: it works without the `otlp` or
//! `prometheus` features being enabled, so a `cargo test`
//! invocation in a downstream crate still gets a working
//! subscriber when it pulls in `ada-telemetry` for the macro
//! surface.
//!
//! Per [`DOC-OBS-004 §2.1`](../docs/observability/04-logging-design.md)
//! the production format is JSON Lines; the `ADA_LOG_FORMAT=pretty`
//! env var switches to the human-readable formatter for
//! development.
//!
//! # Examples
//!
//! ```no_run
//! use ada_telemetry::{LogFormat, TelemetryConfig, init};
//!
//! let cfg = TelemetryConfig::new("my-service")
//!     .with_log_format(LogFormat::Pretty);
//! let _guard = init(cfg).expect("telemetry init");
//! tracing::info!("structured event");
//! ```

use std::io::IsTerminal;

use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use tracing_subscriber::fmt::time::FormatTime;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

use crate::config::{LogFormat, TelemetryConfig};
use crate::error::{Result, TelemetryError};

/// RFC 3339 timestamp formatter used by the JSON layer.
#[derive(Debug)]
pub struct Rfc3339Timestamp;

impl FormatTime for Rfc3339Timestamp {
    fn format_time(
        &self,
        writer: &mut tracing_subscriber::fmt::format::Writer<'_>,
    ) -> std::fmt::Result {
        let now = OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .map_err(|_| std::fmt::Error)?;
        writer.write_str(&now)
    }
}

/// Resolve the `EnvFilter` directive for the logging layer.
///
/// Precedence:
/// 1. `cfg.env_filter` if set (user override).
/// 2. `RUST_LOG` env var if set.
/// 3. Default of `info,ada_telemetry=info`.
pub fn resolve_env_filter(cfg: &TelemetryConfig) -> Result<EnvFilter> {
    let directive = cfg
        .env_filter
        .clone()
        .or_else(|| std::env::var("RUST_LOG").ok())
        .unwrap_or_else(|| "info,ada_telemetry=info".to_string());
    EnvFilter::try_new(directive.clone())
        .map_err(|source| TelemetryError::InvalidEnvFilter { directive, source })
}

/// Install the global `tracing-subscriber` registry with the
/// JSON / pretty fmt layer and the env filter.
///
/// **This function is `pub(crate)` because the only legitimate
/// caller is [`crate::init`], which owns the
/// single-shot-installation contract.**
#[allow(dead_code)]
pub(crate) fn install_logging_layer(cfg: &TelemetryConfig) -> Result<()> {
    let env_filter = resolve_env_filter(cfg)?;

    let result = match cfg.log_format {
        LogFormat::Json => {
            // JSON output to stdout, with the canonical
            // `timestamp | level | fields` envelope.
            // `with_current_span` and `with_span_list` are
            // only available on the JSON layer impl, so
            // they must come after `.json()`.
            let json_layer = tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_thread_ids(false)
                .with_file(false)
                .with_line_number(false)
                .with_timer(Rfc3339Timestamp)
                .json()
                .with_current_span(true)
                .with_span_list(false)
                .with_writer(std::io::stdout);
            tracing_subscriber::registry()
                .with(env_filter)
                .with(json_layer)
                .try_init()
        }
        LogFormat::Pretty => {
            // Human-readable, with ANSI colours when stdout
            // is a TTY. Mirrors the legacy `env_logger` look
            // that most Ada developers are used to.
            let use_ansi = std::io::stdout().is_terminal();
            let pretty_layer = tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_thread_ids(false)
                .with_file(false)
                .with_line_number(false)
                .with_ansi(use_ansi)
                .with_timer(Rfc3339Timestamp)
                .with_writer(std::io::stdout);
            tracing_subscriber::registry()
                .with(env_filter)
                .with(pretty_layer)
                .try_init()
        }
    };

    result.map_err(|e| TelemetryError::SubscriberInit(Box::new(e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_filter_resolves_default() {
        let cfg = TelemetryConfig::default();
        let f = resolve_env_filter(&cfg).expect("default filter parses");
        // The default directive disables nothing.
        let _ = format!("{f:?}");
    }

    #[test]
    fn env_filter_uses_explicit_override() {
        let cfg = TelemetryConfig::default().with_env_filter("warn,foo=debug");
        let f = resolve_env_filter(&cfg).expect("override parses");
        let _ = format!("{f:?}");
    }

    #[test]
    fn env_filter_rejects_bogus_directive() {
        let cfg = TelemetryConfig::default().with_env_filter("not-a-real-directive=###");
        let err = resolve_env_filter(&cfg).unwrap_err();
        assert!(matches!(err, TelemetryError::InvalidEnvFilter { .. }));
    }

    #[test]
    fn rfc3339_timestamp_exists() {
        // `Rfc3339Timestamp` is a unit struct with no
        // observable state; the only thing we can do at
        // runtime is confirm that the type is constructible
        // and `Debug`-printable.
        let ts = Rfc3339Timestamp;
        let _ = format!("{ts:?}");
    }
}
