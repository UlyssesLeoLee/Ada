//! Distributed-tracing pipeline.
//!
//! When the `otlp` feature is enabled, this module builds an
//! [`opentelemetry_sdk::trace::SdkTracerProvider`] backed by an
//! [`opentelemetry_otlp::SpanExporter`] (gRPC) and wires it into
//! the global `tracing-subscriber` registry via
//! [`tracing_opentelemetry::layer().with_tracer()`].
//!
//! When the feature is **not** enabled, all public functions
//! here compile down to no-ops so the rest of the crate can
//! still build in feature-minimal configurations.
//!
//! Per [`DOC-OBS-005 §9`](../docs/observability/05-tracing-design.md)
//! the resource attributes every span carries are
//! `service.name`, `service.version`, and
//! `deployment.environment`.
//!
//! [`opentelemetry_sdk::trace::SdkTracerProvider`]: https://docs.rs/opentelemetry_sdk/0.32/opentelemetry_sdk/trace/struct.SdkTracerProvider.html
//! [`opentelemetry_otlp::SpanExporter`]: https://docs.rs/opentelemetry-otlp/0.32/opentelemetry_otlp/struct.SpanExporter.html
//! [`tracing_opentelemetry::layer().with_tracer()`]: https://docs.rs/tracing-opentelemetry/0.33/tracing_opentelemetry/struct.layer.html

use opentelemetry::KeyValue;
use opentelemetry_sdk::Resource;

use crate::config::TelemetryConfig;
use crate::error::Result;

/// The tracer name used for every `ada-telemetry` span.
pub const TRACER_NAME: &str = "ada-telemetry";

/// Build the OpenTelemetry resource (service.name, service.version,
/// deployment.environment) used to tag every span.
#[cfg(feature = "otlp")]
pub fn build_resource(cfg: &TelemetryConfig) -> Resource {
    Resource::builder()
        .with_attribute(KeyValue::new("service.name", cfg.service_name.clone()))
        .with_attribute(KeyValue::new(
            "service.version",
            cfg.service_version.clone(),
        ))
        .with_attribute(KeyValue::new(
            "deployment.environment",
            cfg.environment.clone(),
        ))
        .build()
}

#[cfg(not(feature = "otlp"))]
pub fn build_resource(cfg: &TelemetryConfig) -> otel_stub::StubResource {
    // Without the otlp feature we still want callers to be
    // able to "build" a resource so config wiring compiles.
    otel_stub::StubResource {
        service_name: cfg.service_name.clone(),
        service_version: cfg.service_version.clone(),
        environment: cfg.environment.clone(),
    }
}

/// Construct the OTLP trace layer that augments the global
/// `tracing-subscriber` registry.
///
/// Returns the layer (which can be `.with()`-ed onto a
/// `Registry`) and a [`SdkTracerProviderGuard`] that, when
/// dropped, flushes and shuts down the exporter.
///
/// The returned layer is typed against the concrete
/// `opentelemetry_sdk::trace::SdkTracerProvider`, so callers
/// outside the `ada-telemetry` crate can still feed it into
/// the `tracing-subscriber` registry.
#[cfg(feature = "otlp")]
#[allow(dead_code)]
pub fn build_otlp_layer<S>(
    cfg: &TelemetryConfig,
) -> Result<(
    tracing_opentelemetry::OpenTelemetryLayer<
        S,
        <opentelemetry_sdk::trace::SdkTracerProvider as opentelemetry::trace::TracerProvider>::Tracer,
    >,
    SdkTracerProviderGuard,
)>
where
    S: tracing_core::Subscriber + for<'span> tracing_subscriber::registry::LookupSpan<'span>,
{
    use opentelemetry_otlp::{Protocol, SpanExporter, WithExportConfig};
    use opentelemetry_sdk::trace::{Sampler as SdkSampler, SdkTracerProvider};

    let exporter = SpanExporter::builder()
        .with_tonic()
        .with_endpoint(cfg.otlp_endpoint.clone())
        .with_protocol(Protocol::Grpc)
        .build()
        .map_err(|e| crate::error::TelemetryError::OtlpExporter(Box::new(e)))?;

    let sampler = if cfg.sample_ratio.is_full() {
        SdkSampler::AlwaysOn
    } else if cfg.sample_ratio.get() <= 0.0 {
        SdkSampler::AlwaysOff
    } else {
        SdkSampler::TraceIdRatioBased(cfg.sample_ratio.get())
    };

    let provider = SdkTracerProvider::builder()
        .with_resource(build_resource(cfg))
        .with_sampler(sampler)
        .with_batch_exporter(exporter)
        .build();

    let tracer = opentelemetry::trace::TracerProvider::tracer(&provider, TRACER_NAME);
    let layer = tracing_opentelemetry::layer().with_tracer(tracer);
    Ok((layer, SdkTracerProviderGuard::new(provider)))
}

/// No-op stub used when the `otlp` feature is disabled.
#[cfg(not(feature = "otlp"))]
#[allow(dead_code)]
pub fn build_otlp_layer<S>(_cfg: &TelemetryConfig) -> Result<(NoopLayer<S>, StubGuard)> {
    Ok((NoopLayer(std::marker::PhantomData), StubGuard))
}

/// Flush + shutdown guard for the OpenTelemetry SDK tracer
/// provider. The SDK's gRPC exporter needs an explicit
/// `shutdown()` call to drain its batch; dropping the provider
/// alone leaves pending spans in memory.
pub struct SdkTracerProviderGuard(
    #[cfg(feature = "otlp")] Option<opentelemetry_sdk::trace::SdkTracerProvider>,
);

impl SdkTracerProviderGuard {
    /// Wrap an active SDK tracer provider in the guard.
    /// Internal — the only legitimate caller is the
    /// `init` function in `lib.rs`.
    #[cfg(feature = "otlp")]
    #[must_use]
    pub(crate) fn new(provider: opentelemetry_sdk::trace::SdkTracerProvider) -> Self {
        Self(Some(provider))
    }

    /// Construct an empty guard (no SDK installed). Useful in
    /// test paths that want a uniform `Option<Guard>` shape.
    #[must_use]
    pub const fn empty() -> Self {
        Self(
            #[cfg(feature = "otlp")]
            None,
        )
    }

    /// `true` iff an SDK is actually installed.
    #[must_use]
    pub const fn is_active(&self) -> bool {
        #[cfg(feature = "otlp")]
        {
            self.0.is_some()
        }
        #[cfg(not(feature = "otlp"))]
        {
            false
        }
    }

    /// Manually flush + shutdown. Idempotent.
    pub fn shutdown(&mut self) {
        #[cfg(feature = "otlp")]
        {
            if let Some(provider) = self.0.take() {
                let _ = provider.shutdown();
            }
        }
    }
}

impl Drop for SdkTracerProviderGuard {
    fn drop(&mut self) {
        self.shutdown();
    }
}

impl core::fmt::Debug for SdkTracerProviderGuard {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SdkTracerProviderGuard")
            .field("active", &self.is_active())
            .finish()
    }
}

// --- stubs for the no-otlp build --------------------------------------

/// Placeholder for the OTLP layer when the feature is off.
#[allow(dead_code)]
pub struct NoopLayer<S>(std::marker::PhantomData<S>);

/// Placeholder guard for the no-otlp build.
#[derive(Debug, Default, Clone, Copy)]
pub struct StubGuard;

/// Placeholder for [`opentelemetry_sdk::Resource`] when the
/// `otlp` feature is off. Holds the same attribute values so
/// callers can `Display` / log them without dragging the SDK
/// into the build.
pub mod otel_stub {
    /// Resource attribute bag used in the no-otlp build.
    #[derive(Debug, Clone)]
    pub struct StubResource {
        /// `service.name` attribute.
        pub service_name: String,
        /// `service.version` attribute.
        pub service_version: String,
        /// `deployment.environment` attribute.
        pub environment: String,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_guard_is_inactive() {
        let g = SdkTracerProviderGuard::empty();
        assert!(!g.is_active());
    }

    #[test]
    fn empty_guard_shutdown_is_idempotent() {
        let mut g = SdkTracerProviderGuard::empty();
        g.shutdown();
        g.shutdown();
        // No panic, no double-shutdown.
    }

    #[test]
    fn debug_does_not_leak_sdk_internals() {
        let g = SdkTracerProviderGuard::empty();
        let s = format!("{g:?}");
        assert!(s.contains("active"));
    }

    #[test]
    fn build_resource_works_without_otlp() {
        // Even without the otlp feature, the resource helper
        // should be callable.
        let cfg = TelemetryConfig::new("svc");
        let r = build_resource(&cfg);
        // The exact type of `r` depends on the feature, so
        // we exercise the no-otlp path with an assertion and
        // accept either an SDK Resource or the stub on the
        // otlp path. `let _ = r;` silences the unused warning
        // in the feature-on build.
        let _ = r;
        #[cfg(not(feature = "otlp"))]
        assert_eq!(r.service_name, "svc");
    }
}
