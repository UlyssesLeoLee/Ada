//! Metrics pipeline.
//!
//! When the `prometheus` feature is enabled, this module
//! installs a [`metrics_exporter_prometheus::PrometheusBuilder`]
//! that binds a TCP listener (Prometheus' pull model) and
//! hands out a [`PrometheusHandle`] for snapshotting the
//! current metrics on demand.
//!
//! When the feature is **not** enabled, the public surface
//! falls back to a no-op handle so the rest of the crate can
//! still compile. This is what enables `cargo test` runs in
//! downstream crates that don't actually need the
//! metrics endpoint.
//!
//! Per [`DOC-OBS-003 §2`](../../../docs/observability/03-metrics-design.md)
//! the canonical metric name format is
//! `ada.{layer}.{component}.{metric}_{unit}`. The helper
//! [`canonical_name`] enforces the `ada.` prefix.
//!
//! [`metrics_exporter_prometheus::PrometheusBuilder`]: https://docs.rs/metrics-exporter-prometheus/0.18/metrics_exporter_prometheus/struct.PrometheusBuilder.html
//! [`PrometheusHandle`]: https://docs.rs/metrics-exporter-prometheus/0.18/metrics_exporter_prometheus/struct.PrometheusHandle.html

use crate::config::TelemetryConfig;
// `TelemetryError` is referenced by the `prometheus` arm of
// `install_recorder` below. It used to be imported only under the
// non-prometheus cfg, which meant the entire `prometheus` feature had
// never compiled. No CI job enables it and `default = ["otlp"]`, so the
// breakage was invisible.
#[cfg(feature = "prometheus")]
use crate::error::{Result, TelemetryError};

/// Build the `ada.` prefixed canonical metric name from a
/// `(layer, component, metric)` tuple.
///
/// # Examples
///
/// ```rust
/// use ada_telemetry::canonical_name;
///
/// assert_eq!(
///     canonical_name("app", "api_gateway", "requests_total"),
///     "ada.app.api_gateway.requests_total"
/// );
/// ```
#[must_use]
pub fn canonical_name(layer: &str, component: &str, metric: &str) -> String {
    format!("ada.{layer}.{component}.{metric}")
}

/// Validate that a metric name conforms to the
/// `ada.{layer}.{component}.{metric}_{unit}` shape. Returns
/// `true` for any name that starts with `ada.` and contains
/// at least three dot-separated segments.
#[must_use]
pub fn is_canonical(name: &str) -> bool {
    let Some(rest) = name.strip_prefix("ada.") else {
        return false;
    };
    rest.split('.').count() >= 3 && !rest.is_empty()
}

/// Result of a `metrics::counter!` style call. The handle
/// returned by [`install_recorder`] is the source of truth
/// for snapshotting the registry.
#[derive(Debug, Clone)]
pub struct MetricsHandle {
    inner: HandleInner,
}

#[derive(Debug, Clone)]
enum HandleInner {
    /// Real Prometheus handle (feature = "prometheus").
    #[cfg(feature = "prometheus")]
    Prometheus(metrics_exporter_prometheus::PrometheusHandle),
    /// No-op handle used in feature-minimal builds.
    Noop,
}

impl MetricsHandle {
    /// Construct a no-op handle (no Prometheus exporter wired
    /// up). Returns the empty string from [`MetricsHandle::render`]
    /// and `false` from [`MetricsHandle::is_active`].
    #[must_use]
    pub const fn noop() -> Self {
        Self {
            inner: HandleInner::Noop,
        }
    }

    /// Render the current metrics registry as a Prometheus
    /// exposition payload. When the `prometheus` feature is
    /// off this returns the empty string.
    #[must_use]
    pub fn render(&self) -> String {
        match &self.inner {
            #[cfg(feature = "prometheus")]
            HandleInner::Prometheus(h) => h.render(),
            HandleInner::Noop => String::new(),
        }
    }

    /// `true` iff the handle is wired to a live exporter.
    #[must_use]
    pub const fn is_active(&self) -> bool {
        match &self.inner {
            #[cfg(feature = "prometheus")]
            HandleInner::Prometheus(_) => true,
            HandleInner::Noop => false,
        }
    }
}

/// Install the metrics recorder. Returns a guard that, when
/// dropped, stops the HTTP listener; the handle is the
/// rendering endpoint.
#[cfg(feature = "prometheus")]
pub fn install_recorder(cfg: &TelemetryConfig) -> Result<(MetricsGuard, MetricsHandle)> {
    use std::net::SocketAddr;
    use std::str::FromStr;

    let addr =
        SocketAddr::from_str(&cfg.prometheus_addr).map_err(|e| TelemetryError::PrometheusBind {
            addr: cfg.prometheus_addr.clone(),
            source: std::io::Error::new(std::io::ErrorKind::InvalidInput, e),
        })?;

    let builder = metrics_exporter_prometheus::PrometheusBuilder::new().with_http_listener(addr);
    // `install_recorder` returns only the handle. It builds the recorder
    // internally and installs it into the `metrics` crate's global
    // recorder slot, where it lives for the rest of the process. This
    // code was written against an older release of the crate, which
    // returned a `(PrometheusRecorder, PrometheusHandle)` pair -- the
    // destructuring below no longer matched the real signature, so the
    // whole `prometheus` feature had never compiled.
    let handle = builder
        .install_recorder()
        .map_err(|e| TelemetryError::PrometheusBind {
            addr: cfg.prometheus_addr.clone(),
            source: std::io::Error::other(e.to_string()),
        })?;
    Ok((
        MetricsGuard { active: true },
        MetricsHandle {
            inner: HandleInner::Prometheus(handle),
        },
    ))
}

#[cfg(not(feature = "prometheus"))]
pub fn install_recorder(_cfg: &TelemetryConfig) -> (MetricsGuard, MetricsHandle) {
    (MetricsGuard::inactive(), MetricsHandle::noop())
}

/// Drop guard for the metrics exporter. When the
/// `prometheus` feature is off, the inner field is absent
/// and the type is a zero-sized marker.
pub struct MetricsGuard {
    #[cfg(feature = "prometheus")]
    active: bool,
}

impl MetricsGuard {
    /// Construct an "off" guard (no Prometheus listener).
    /// Used by [`crate::init`] when the user disabled the metrics
    /// pipeline.
    #[must_use]
    pub const fn inactive() -> Self {
        Self {
            #[cfg(feature = "prometheus")]
            active: false,
        }
    }

    /// `true` iff a Prometheus listener is actually running.
    #[must_use]
    pub const fn is_active(&self) -> bool {
        #[cfg(feature = "prometheus")]
        {
            self.active
        }
        #[cfg(not(feature = "prometheus"))]
        {
            false
        }
    }
}

impl Drop for MetricsGuard {
    fn drop(&mut self) {
        // The guard owns no recorder. `PrometheusBuilder::install_recorder`
        // installs the exporter into the `metrics` crate's *global*
        // recorder slot and returns only a handle, so the listener is
        // shut down by process teardown rather than by dropping this
        // guard. `is_active()` reports whether installation succeeded,
        // which is the only thing this type can truthfully answer.
        //
        // Note the `metrics` crate's global recorder is process-wide, so
        // a second `install_recorder` call fails rather than replacing it.
    }
}

impl core::fmt::Debug for MetricsGuard {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("MetricsGuard")
            .field("active", &self.is_active())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_name_format() {
        assert_eq!(
            canonical_name("app", "api_gateway", "requests_total"),
            "ada.app.api_gateway.requests_total"
        );
    }

    #[test]
    fn canonical_name_with_unit_suffix() {
        assert_eq!(
            canonical_name("infra", "node", "cpu_utilization_ratio"),
            "ada.infra.node.cpu_utilization_ratio"
        );
    }

    #[test]
    fn is_canonical_accepts_valid() {
        assert!(is_canonical("ada.app.api_gateway.requests_total"));
        assert!(is_canonical("ada.infra.node.cpu_utilization_ratio"));
        assert!(is_canonical("ada.k8s.pod.restart_total"));
    }

    #[test]
    fn is_canonical_rejects_wrong_prefix() {
        assert!(!is_canonical("foo.app.x.y"));
        assert!(!is_canonical("otel.x.y"));
        assert!(!is_canonical(""));
    }

    #[test]
    fn is_canonical_rejects_too_few_segments() {
        assert!(!is_canonical("ada.app"));
        assert!(!is_canonical("ada.app.x"));
        assert!(!is_canonical("ada."));
    }

    #[test]
    fn noop_handle_renders_empty() {
        let h = MetricsHandle {
            inner: HandleInner::Noop,
        };
        assert_eq!(h.render(), "");
        assert!(!h.is_active());
    }

    #[test]
    fn metrics_guard_debug_includes_active_flag() {
        let g = MetricsGuard {
            #[cfg(feature = "prometheus")]
            active: false,
        };
        let s = format!("{g:?}");
        assert!(s.contains("active"));
    }
}
