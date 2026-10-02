//! Test-only utilities for `ada-telemetry` itself and for
//! downstream crates that need to assert on telemetry
//! behaviour without standing up a real otel-collector.
//!
//! All public items are gated behind the `testing` feature
//! (or `#[cfg(test)]` for the in-crate tests) so that
//! production binaries never link the test exporter.
//!
//! The headline helper is [`test_recorder`], which installs
//! a fresh `metrics::PrometheusHandle` bound to an
//! ephemeral TCP port. Tests can then:
//!
//! 1. emit a counter / histogram via `metrics::counter!` /
//!    `metrics::histogram!`,
//! 2. snapshot the registry with [`TestHandle::render`],
//! 3. assert on the resulting Prometheus exposition text.

#[cfg(feature = "prometheus")]
use crate::config::TelemetryConfig;
#[cfg(feature = "prometheus")]
use crate::error::Result;
#[cfg(feature = "prometheus")]
use crate::metrics::MetricsHandle;

/// Ephemeral metrics handle bundled with a guard for safe
/// teardown in tests.
#[cfg(feature = "prometheus")]
#[derive(Debug)]
pub struct TestHandle {
    /// Reports whether installation succeeded. It does **not** own the
    /// recorder: `PrometheusBuilder::install_recorder` installs the
    /// exporter into the `metrics` crate's process-global recorder slot
    /// and returns only a handle.
    pub guard: crate::metrics::MetricsGuard,
    /// Snapshot/render handle.
    pub handle: MetricsHandle,
}

#[cfg(feature = "prometheus")]
impl TestHandle {
    /// Render the current registry state.
    #[must_use]
    pub fn render(&self) -> String {
        self.handle.render()
    }

    /// `true` iff the recorder is wired up.
    #[must_use]
    pub const fn is_active(&self) -> bool {
        self.handle.is_active()
    }
}

/// Build a `TestHandle` with a Prometheus recorder bound to
/// `127.0.0.1:0` (the OS picks an ephemeral port).
///
/// Note the `metrics` crate allows a process-global recorder to be set
/// only once, so a second call to this function returns `Err`. A test
/// binary that needs several independent recorders cannot get them
/// through this helper; the crate currently has no caller, and adding
/// one is a real design decision rather than a mechanical change.
#[cfg(feature = "prometheus")]
pub fn test_recorder() -> Result<TestHandle> {
    let cfg = TelemetryConfig::new("ada-telemetry-test").with_prometheus_addr("127.0.0.1:0");
    let (guard, handle) = crate::metrics::install_recorder(&cfg)?;
    Ok(TestHandle { guard, handle })
}

/// Compile-time guard for the no-prometheus build: the test
/// helper is a no-op that returns an empty handle. Tests that
/// actually need to read the registry should gate themselves
/// behind `#[cfg(feature = "prometheus")]`.
#[cfg(not(feature = "prometheus"))]
pub fn test_recorder() -> NoopTestHandle {
    NoopTestHandle
}

/// Empty handle used when the `prometheus` feature is off.
#[cfg(not(feature = "prometheus"))]
#[derive(Debug)]
pub struct NoopTestHandle;

#[cfg(not(feature = "prometheus"))]
impl NoopTestHandle {
    /// Always returns the empty string.
    #[must_use]
    pub fn render(&self) -> String {
        String::new()
    }
    /// Always `false`.
    #[must_use]
    pub const fn is_active(&self) -> bool {
        false
    }
}

/// Parse a Prometheus exposition payload and return the names
/// of every metric series found. Useful for "did the counter
/// fire?" assertions without pulling in a full Prometheus
/// parser.
#[must_use]
pub fn metric_names(exposition: &str) -> Vec<&str> {
    exposition
        .lines()
        .filter_map(|line| {
            if line.starts_with('#') || line.is_empty() {
                return None;
            }
            line.split_whitespace().next()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metric_names_skips_comments_and_blanks() {
        let text = "\
# HELP ada_app_x_total Counter
# TYPE ada_app_x_total counter
ada_app_x_total{endpoint=\"/foo\"} 1
ada_app_y_total 2

";
        let names = metric_names(text);
        assert_eq!(
            names,
            vec!["ada_app_x_total{endpoint=\"/foo\"}", "ada_app_y_total"]
        );
    }

    #[test]
    fn metric_names_handles_empty() {
        assert_eq!(metric_names("").len(), 0);
        assert_eq!(metric_names("# only a comment\n\n").len(), 0);
    }
}
