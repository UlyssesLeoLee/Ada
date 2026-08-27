//! Structured logging, distributed tracing, and metrics collection. OpenTelemetry-compatible.
//!
//! 関連 IPA フェーズ: 22-52 (基本設計/詳細設計), 53-58 (実装), 59-95 (試験)
//! 設計書:
//! - [`docs/observability/02-architecture.md`](../docs/observability/02-architecture.md) — 4-シグナル統合
//! - [`docs/observability/03-metrics-design.md`](../docs/observability/03-metrics-design.md) — RED/USE
//! - [`docs/observability/04-logging-design.md`](../docs/observability/04-logging-design.md) — JSON + 脱敏
//! - [`docs/observability/05-tracing-design.md`](../docs/observability/05-tracing-design.md) — W3C + OTLP
//!
//! # Quick start
//!
//! ```no_run
//! use ada_telemetry::{TelemetryConfig, init};
//!
//! fn main() {
//!     let cfg = TelemetryConfig::from_env("my-service");
//!     let guard = ada_telemetry::init(cfg).expect("telemetry init");
//!
//!     tracing::info!(event = "booted", "service started");
//!
//!     // ... do work ...
//!
//!     drop(guard); // flushes OTLP, stops Prometheus listener
//! }
//! ```
//!
//! # Feature flags
//!
//! | Feature      | Default | What it pulls in |
//! |--------------|:-------:|------------------|
//! | `otlp`       |   ✅    | OpenTelemetry SDK + OTLP gRPC exporter + `tracing-opentelemetry` bridge |
//! | `prometheus` |   ❌    | `metrics` facade + `metrics-exporter-prometheus` HTTP listener |
//! | `testing`    |   ❌    | Test-only utilities in [`testing`] |
//!
//! v0.2.0 keeps `prometheus` opt-in because the
//! `metrics-exporter-prometheus` crate opens a TCP listener
//! at process start, which isn't desirable in every binary
//! (CLI tools, ad-hoc workers, tests).
//!
//! # Architecture
//!
//! The crate is split into five internal modules, each
//! addressable in isolation for tests and downstream
//! composition:
//!
//! - [`config`] — the [`TelemetryConfig`] builder
//! - [`error`] — [`TelemetryError`] / [`error::Result`]
//! - [`logging`] — `tracing-subscriber` JSON / pretty layer
//! - [`tracing`] — OpenTelemetry SDK + OTLP exporter
//! - [`metrics`] — Prometheus pull endpoint + canonical name helper

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

mod config;
mod error;
mod logging;
mod metrics;
#[cfg(any(feature = "testing", test))]
pub mod testing;
mod tracing;

pub use config::{
    LogFormat, SampleRatio, TelemetryConfig, DEFAULT_ENVIRONMENT, DEFAULT_OTLP_ENDPOINT,
    DEFAULT_PROMETHEUS_ADDR,
};
pub use error::{Result, TelemetryError};
pub use metrics::{canonical_name, is_canonical, MetricsGuard, MetricsHandle};
pub use tracing::{build_resource, otel_stub, SdkTracerProviderGuard, StubGuard, TRACER_NAME};

use core::sync::atomic::{AtomicBool, Ordering};
use std::io::IsTerminal;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

/// Crate version, taken from `CARGO_PKG_VERSION` (single workspace
/// version per D-09).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Crate name, taken from `CARGO_PKG_NAME`.
pub const NAME: &str = env!("CARGO_PKG_NAME");

/// `shared`-layer string tag.
pub const LAYER: &str = "shared";

/// Single point of entry — install the full telemetry
/// pipeline (logging + tracing + metrics) and return a guard
/// that flushes everything on drop.
///
/// Subsequent calls in the same process return
/// [`TelemetryError::AlreadyInitialised`]; the global
/// `tracing-subscriber` registry is single-shot by design.
///
/// # Examples
///
/// ```no_run
/// use ada_telemetry::{TelemetryConfig, init};
///
/// let cfg = TelemetryConfig::new("my-service");
/// let _guard = init(cfg).expect("telemetry init");
/// tracing::info!("ready");
/// ```
#[allow(clippy::needless_pass_by_value)]
pub fn init(cfg: TelemetryConfig) -> Result<TelemetryGuard> {
    static INSTALLED: AtomicBool = AtomicBool::new(false);
    if INSTALLED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err(TelemetryError::AlreadyInitialised);
    }

    cfg.validate()?;

    let env_filter = logging::resolve_env_filter(&cfg)?;
    let use_ansi = std::io::stdout().is_terminal();
    let timer = logging::Rfc3339Timestamp;

    // Install the global subscriber. The OTLP layer is
    // built **inside** the install function so its `S`
    // type is the eventual `Layered<...>` stack rather
    // than a bare `Registry` — this is what the
    // `tracing_subscriber` trait machinery requires.
    #[cfg(feature = "otlp")]
    let otlp_guard = if cfg.tracing_enabled {
        install_with_otlp(&cfg, env_filter, timer, use_ansi)?
    } else {
        install_without_otlp(&cfg, env_filter, timer, use_ansi)?;
        SdkTracerProviderGuard::empty()
    };
    #[cfg(not(feature = "otlp"))]
    {
        install_without_otlp(&cfg, env_filter, timer, use_ansi)?;
    }
    #[cfg(not(feature = "otlp"))]
    let otlp_guard = StubGuard;

    // Build the metrics recorder (no-op if disabled).
    let metrics = if cfg.metrics_enabled {
        #[cfg(feature = "prometheus")]
        let (guard, handle) = metrics::install_recorder(&cfg)?;
        #[cfg(not(feature = "prometheus"))]
        let (guard, handle) = metrics::install_recorder(&cfg);
        TelemetryMetrics::Active { guard, handle }
    } else {
        TelemetryMetrics::Inactive {
            guard: MetricsGuard::inactive(),
            handle: MetricsHandle::noop(),
        }
    };

    // Emit the "telemetry_init" log record on the global
    // subscriber. The macro is at `::tracing::info!` once
    // the subscriber is installed.
    ::tracing::info!(
        event = "telemetry_init",
        service.name = %cfg.service_name,
        service.version = %cfg.service_version,
        deployment.environment = %cfg.environment,
        otlp = cfg.tracing_enabled,
        metrics = cfg.metrics_enabled,
        "telemetry pipeline initialised",
    );

    Ok(TelemetryGuard {
        otlp: otlp_guard,
        metrics,
    })
}

/// Install the registry without an OTLP layer.
fn install_without_otlp(
    cfg: &TelemetryConfig,
    env_filter: tracing_subscriber::EnvFilter,
    timer: logging::Rfc3339Timestamp,
    use_ansi: bool,
) -> Result<()> {
    match cfg.log_format {
        LogFormat::Json => {
            let json_layer = tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_timer(timer)
                .json()
                .with_current_span(true)
                .with_span_list(false)
                .with_writer(std::io::stdout);
            tracing_subscriber::registry()
                .with(env_filter)
                .with(json_layer)
                .try_init()
                .map_err(|e| TelemetryError::SubscriberInit(Box::new(e)))?;
        }
        LogFormat::Pretty => {
            let pretty_layer = tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_ansi(use_ansi)
                .with_timer(timer)
                .with_writer(std::io::stdout);
            tracing_subscriber::registry()
                .with(env_filter)
                .with(pretty_layer)
                .try_init()
                .map_err(|e| TelemetryError::SubscriberInit(Box::new(e)))?;
        }
    }
    Ok(())
}

/// Install the registry with the OTLP layer on top. The
/// OTLP layer is built **inside** this function so the
/// `S` type of `OpenTelemetryLayer<S, T>` is inferred to
/// be the actual `Layered<...>` stack we are about to
/// install. Returns the [`SdkTracerProviderGuard`] so the
/// caller can flush / shut the SDK down on drop.
#[cfg(feature = "otlp")]
fn install_with_otlp(
    cfg: &TelemetryConfig,
    env_filter: tracing_subscriber::EnvFilter,
    timer: logging::Rfc3339Timestamp,
    use_ansi: bool,
) -> Result<SdkTracerProviderGuard> {
    use opentelemetry_otlp::{Protocol, SpanExporter, WithExportConfig};
    use opentelemetry_sdk::trace::{Sampler as SdkSampler, SdkTracerProvider};
    use tracing_subscriber::layer::SubscriberExt as _;

    // Build the OTLP exporter + provider.
    let exporter = SpanExporter::builder()
        .with_tonic()
        .with_endpoint(cfg.otlp_endpoint.clone())
        .with_protocol(Protocol::Grpc)
        .build()
        .map_err(|e| TelemetryError::OtlpExporter(Box::new(e)))?;
    let sampler = if cfg.sample_ratio.is_full() {
        SdkSampler::AlwaysOn
    } else if cfg.sample_ratio.get() <= 0.0 {
        SdkSampler::AlwaysOff
    } else {
        SdkSampler::TraceIdRatioBased(cfg.sample_ratio.get())
    };
    let provider = SdkTracerProvider::builder()
        .with_resource(tracing::build_resource(cfg))
        .with_sampler(sampler)
        .with_batch_exporter(exporter)
        .build();
    let tracer = opentelemetry::trace::TracerProvider::tracer(&provider, tracing::TRACER_NAME);
    let otlp_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    // Per tracing-subscriber semantics, the OTLP layer must be
    // applied to a Subscriber that already implements
    // `Subscriber + LookupSpan`. The chain
    //     `registry().with(env_filter).with(otlp_layer)`
    // pins `otlp_layer`'s `S` to `Layered<EnvFilter, Registry>`,
    // which the compiler can verify as a Subscriber+LookupSpan.
    // We then attach the fmt layer on top so it sees the OTLP
    // spans. Layer order is: registry ← env_filter ← otlp_layer
    // ← fmt_layer, which gives the desired fan-out (every
    // event hits both the fmt writer and the OTLP exporter).
    match cfg.log_format {
        LogFormat::Json => {
            let json_layer = tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_timer(timer)
                .json()
                .with_current_span(true)
                .with_span_list(false)
                .with_writer(std::io::stdout);
            tracing_subscriber::registry()
                .with(env_filter)
                .with(otlp_layer)
                .with(json_layer)
                .try_init()
                .map_err(|e| TelemetryError::SubscriberInit(Box::new(e)))?;
        }
        LogFormat::Pretty => {
            let pretty_layer = tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_ansi(use_ansi)
                .with_timer(timer)
                .with_writer(std::io::stdout);
            tracing_subscriber::registry()
                .with(env_filter)
                .with(otlp_layer)
                .with(pretty_layer)
                .try_init()
                .map_err(|e| TelemetryError::SubscriberInit(Box::new(e)))?;
        }
    }
    Ok(SdkTracerProviderGuard::new(provider))
}

/// Small `Either`-style helper used by [`init`] to thread
/// the two log-format branches through one code path.
#[allow(dead_code)]
enum EitherLayer<L, R> {
    /// JSON path.
    Left(L),
    /// Pretty path.
    Right(R),
}

/// Drop guard returned by [`init`]. When dropped, the OTLP
/// exporter is flushed and the Prometheus listener is stopped.
///
/// Construct only via [`init`].
#[must_use = "dropping the guard flushes the OTLP exporter and stops the Prometheus listener immediately"]
pub struct TelemetryGuard {
    otlp: OtlpGuardInner,
    metrics: TelemetryMetrics,
}

#[cfg(feature = "otlp")]
type OtlpGuardInner = SdkTracerProviderGuard;
#[cfg(not(feature = "otlp"))]
type OtlpGuardInner = StubGuard;

enum TelemetryMetrics {
    /// Real Prometheus recorder wired up.
    #[allow(dead_code)]
    Active {
        /// Drop guard for the Prometheus builder.
        guard: MetricsGuard,
        /// Render handle.
        handle: MetricsHandle,
    },
    /// Disabled by config.
    #[allow(dead_code)]
    Inactive {
        /// Inactive guard.
        guard: MetricsGuard,
        /// Noop handle.
        handle: MetricsHandle,
    },
}

impl TelemetryGuard {
    /// Access the metrics handle for ad-hoc snapshotting.
    /// Useful in integration tests that want to assert the
    /// metrics registry state without rendering the full
    /// Prometheus payload.
    #[must_use]
    pub const fn metrics_handle(&self) -> &MetricsHandle {
        match &self.metrics {
            TelemetryMetrics::Active { handle, .. } | TelemetryMetrics::Inactive { handle, .. } => {
                handle
            }
        }
    }

    /// `true` iff the OTLP trace pipeline is live.
    #[must_use]
    pub fn is_tracing_active(&self) -> bool {
        #[cfg(feature = "otlp")]
        {
            // `OtlpGuardInner` is a type alias for
            // `SdkTracerProviderGuard` when the `otlp`
            // feature is on, so `is_active` is the
            // inherent method on the underlying struct.
            self.otlp.is_active()
        }
        #[cfg(not(feature = "otlp"))]
        {
            let _ = &self.otlp;
            false
        }
    }

    /// `true` iff the Prometheus listener is live.
    #[must_use]
    pub const fn is_metrics_active(&self) -> bool {
        match &self.metrics {
            TelemetryMetrics::Active { .. } => true,
            TelemetryMetrics::Inactive { .. } => false,
        }
    }
}

impl core::fmt::Debug for TelemetryGuard {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TelemetryGuard")
            .field("tracing_active", &self.is_tracing_active())
            .field("metrics_active", &self.is_metrics_active())
            .finish()
    }
}

/// Build the [`EnvFilter`] the subscriber uses to decide
/// which records make it to the fmt / OTLP layers.
///
/// Re-exported so downstream binaries can construct the
/// same filter without going through [`TelemetryConfig`].
pub fn env_filter_from(directive: &str) -> Result<EnvFilter> {
    EnvFilter::try_new(directive).map_err(|source| TelemetryError::InvalidEnvFilter {
        directive: directive.to_string(),
        source,
    })
}

/// Re-exports of the most-used `tracing` macros so callers
/// don't have to add a second `use` line.
pub mod prelude {
    pub use tracing::{debug, error, info, instrument, span, trace, warn, Span};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crate_version_is_set() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn crate_name_is_set() {
        assert!(!NAME.is_empty());
    }

    #[test]
    fn layer_string_is_known() {
        assert!(
            ["skeleton", "blood", "nerve", "muscle", "shared"].contains(&LAYER),
            "Unknown layer: {LAYER}",
        );
    }

    #[test]
    fn metrics_handle_noop_renders_empty() {
        let h = MetricsHandle::noop();
        assert_eq!(h.render(), "");
        assert!(!h.is_active());
    }

    #[test]
    fn metrics_guard_inactive_is_not_active() {
        let g = MetricsGuard::inactive();
        assert!(!g.is_active());
    }

    #[test]
    fn env_filter_from_accepts_default_directive() {
        let _f = env_filter_from("info,ada_telemetry=info").expect("parses");
    }

    #[test]
    fn env_filter_from_rejects_bogus_directive() {
        let err = env_filter_from("not=###").unwrap_err();
        assert!(matches!(err, TelemetryError::InvalidEnvFilter { .. }));
    }

    #[test]
    fn prelude_reexports_macros() {
        // Symbol reachability check; no global subscriber
        // means we can't actually emit events.
        let _ = ::tracing::Level::INFO;
    }
}
