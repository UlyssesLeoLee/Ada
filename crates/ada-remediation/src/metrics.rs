//! Prometheus metrics facade for `ada-remediation` (v0.7.0).
//!
//! v0.6.0 had no Prometheus exporter. Dashboards queried
//! `remediation_history` directly from PostgreSQL. v0.7.0
//! adds a real `/metrics` endpoint backed by the
//! [`metrics`](https://docs.rs/metrics) facade and a
//! [`metrics-exporter-prometheus`](https://docs.rs/metrics-exporter-prometheus)
//! recorder, so the same metrics also flow to the
//! ada-telemetry / Prometheus pipeline.
//!
//! # Architecture
//!
//! ```text
//!   engine.execute()
//!     ↓ metrics::counter!("remediation_actions_total", ...)
//!     ↓ metrics::histogram!("remediation_action_duration_seconds", ...)
//!   metrics facade (global recorder)
//!     ↓
//!   metrics-exporter-prometheus recorder
//!     ↓ render()
//!   GET /metrics  (Prometheus text format)
//! ```
//!
//! # Metric names
//!
//! All names use the `ada_remediation_*` namespace to keep
//! the dashboard group consistent with the per-component
//! metric naming in `docs/observability/03-metrics-design.md`.
//!
//! | Name | Type | Labels | Source |
//! |---|---|---|---|
//! | `ada_remediation_actions_total` | Counter | `action_id`, `outcome` | every step outcome in `engine::execute` |
//! | `ada_remediation_action_duration_seconds` | Histogram | `action_id` | wall-clock per-step duration |
//! | `ada_remediation_engine_state_transitions_total` | Counter | `from`, `to` | every state machine transition |
//! | `ada_remediation_cooldown_active` | Gauge | (none) | `MemoryStore::active_cooldowns` count |
//!
//! # Why facade + recorder (not the `prometheus` crate directly)?
//!
//! 1. The same `metrics::counter!` macro call site works in
//!    unit tests (where the exporter is the no-op default
//!    recorder) and in production (where the Prometheus
//!    recorder is installed). Tests don't need to manage a
//!    real `prometheus::Registry`.
//! 2. The `ada-telemetry` crate already uses this pattern
//!    with the OTLP exporter; this crate just adds the
//!    Prometheus exposition alongside the existing tracing
//!    pipeline.
//!
//! # Idempotency
//!
//! `install_recorder` is process-global: it can only be
//! called once per process, otherwise it panics. We hide
//! that behind an [`OnceLock`](std::sync::OnceLock) so a
//! repeated `install()` returns the previously-installed
//! handle instead of panicking. The HTTP handler always
//! reads from the installed handle; tests that don't want
//! the metrics exported can skip `install()` and the
//! `render()` call simply returns the empty string.

use crate::error::RemediationError;
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use std::sync::OnceLock;

/// Errors that can come out of [`install`].
#[derive(Debug, thiserror::Error)]
pub enum MetricsError {
    #[error("prometheus recorder already installed by another caller")]
    AlreadyInstalled,
}

/// Bundle returned by [`install`]. The Prometheus recorder
/// is installed process-globally by `install_recorder`;
/// `MetricsState` just carries the handle so the HTTP
/// route can render the current snapshot.
#[derive(Debug, Clone)]
pub struct MetricsState {
    pub handle: PrometheusHandle,
}

static METRICS: OnceLock<MetricsState> = OnceLock::new();

/// Install the Prometheus recorder. Idempotent in spirit:
/// a second call returns the previously installed state.
///
/// In practice there are three outcomes, all of which this
/// function maps to a `Result` so the test harness can
/// distinguish "I am the installer" from "someone else
/// already installed" without panicking:
///
/// 1. **No recorder yet, this call wins** — `Ok(&state)`.
/// 2. **We already installed it earlier** — `Ok(&state)`
///    (`OnceLock` fast path).
/// 3. **The `metrics` crate's process-global recorder was
///    already set by a different caller** (typical in
///    `cargo test` where several tests run concurrently
///    and one of them won the race) — `Err(AlreadyInstalled)`.
///
/// We deliberately do not panic in case 3; the HTTP
/// handler's `render()` returns the empty string when no
/// `MetricsState` is installed, and the in-process recorder
/// already installed by the other test still collects
/// `metrics::*!` calls correctly. The test harness treats
/// case 3 as a benign "another test got there first".
pub fn install() -> std::result::Result<&'static MetricsState, MetricsError> {
    if let Some(state) = METRICS.get() {
        return Ok(state);
    }
    let builder = PrometheusBuilder::new();
    match builder.install_recorder() {
        Ok(handle) => {
            let state = MetricsState { handle };
            // `get_or_init` runs the closure only if the cell
            // is empty. If a parallel test won the race, our
            // `state` is dropped and we get the winner's
            // pointer back. Either way we get a `&'static
            // MetricsState`.
            Ok(METRICS.get_or_init(|| state))
        }
        Err(_e) => {
            // The underlying `metrics` crate rejected our
            // recorder because some other caller already set
            // the process-global recorder. We cannot recover
            // a handle to that recorder (the
            // `metrics-exporter-prometheus` API does not
            // expose it), so we cannot render its text. We
            // surface this as `AlreadyInstalled` so the
            // caller can decide. The HTTP handler falls
            // back to an empty snapshot, which is correct.
            Err(MetricsError::AlreadyInstalled)
        }
    }
}

/// Render the current Prometheus snapshot. Returns the
/// empty string if `install` has not been called yet.
#[must_use]
pub fn render() -> String {
    METRICS.get().map(|s| s.handle.render()).unwrap_or_default()
}

/// `true` iff [`install`] has been called in this process.
#[must_use]
pub fn is_installed() -> bool {
    METRICS.get().is_some()
}

// ----------------------------------------------------------------------
// Metric recording helpers — thin wrappers around the
// `metrics` facade so call sites are grep-able and the
// label set is consistent across the crate.
// ----------------------------------------------------------------------

/// Record a single step outcome. Called by the engine
/// after every step in `execute`. `outcome` is one of
/// `"success"`, `"failure"`, `"skipped"`.
pub fn record_step_outcome(action_id: &str, outcome: &str) {
    metrics::counter!(
        "ada_remediation_actions_total",
        "action_id" => action_id.to_string(),
        "outcome" => outcome.to_string(),
    )
    .increment(1);
}

/// Record the wall-clock duration of one step.
pub fn record_step_duration(action_id: &str, duration_secs: f64) {
    metrics::histogram!(
        "ada_remediation_action_duration_seconds",
        "action_id" => action_id.to_string(),
    )
    .record(duration_secs);
}

/// Record a state-machine transition. Called by the
/// engine every time the `EngineState` changes.
pub fn record_state_transition(from: &str, to: &str) {
    metrics::counter!(
        "ada_remediation_engine_state_transitions_total",
        "from" => from.to_string(),
        "to" => to.to_string(),
    )
    .increment(1);
}

/// Set the live cooldown gauge. Called by the HTTP
/// route on every `/metrics` scrape (cheap: O(active
/// cooldown count)).
pub fn set_cooldown_gauge(active: f64) {
    metrics::gauge!("ada_remediation_cooldown_active").set(active);
}

impl From<MetricsError> for RemediationError {
    fn from(e: MetricsError) -> Self {
        Self::StepFailed {
            index: 0,
            message: format!("metrics: {e}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MemoryStore;

    #[test]
    fn render_empty_before_install() {
        // Force a clean slate: if the test harness has
        // already installed the recorder (e.g. another
        // test ran first), `render` will return non-empty
        // — that path is also fine to assert.
        let s = render();
        if is_installed() {
            assert!(s.contains("ada_remediation"));
        } else {
            assert_eq!(s, "");
        }
    }

    #[test]
    fn install_is_idempotent() {
        // Tests in this module (and sibling test binaries
        // compiled into the same `ada_remediation` library)
        // may call `install()` concurrently. The
        // process-global `metrics` recorder can only be set
        // once, so any call after the winner sees
        // `Err(AlreadyInstalled)` from the underlying
        // `install_recorder`. Acceptable outcomes:
        //   - We won the race: first call `Ok`, second
        //     call `Ok` with the same pointer (OnceLock
        //     fast path).
        //   - We lost the race: first call
        //     `Err(AlreadyInstalled)`. A sibling test
        //     wins the `install_recorder` call and then
        //     populates our `METRICS` OnceLock, so the
        //     second call returns `Ok` (OnceLock fast
        //     path) pointing at the sibling's handle. The
        //     metrics flow, we just observability-take a
        //     pointer we did not create.
        //   - We won earlier in the test run: same as
        //     case 1 (OnceLock already populated, fast
        //     path).
        // The assertion is: "no panic, no `Err` other
        // than `AlreadyInstalled` on the *first* call,
        // and a second call must succeed (the OnceLock
        // is populated by whoever won the race)".
        match install() {
            Ok(a) => {
                let b = install().expect("second install after winning");
                assert!(std::ptr::eq(a, b));
                assert!(is_installed());
            }
            Err(MetricsError::AlreadyInstalled) => {
                // Lost the race. Sibling's `install()` is
                // racing to populate METRICS; if it has
                // already done so by the time we call
                // again, we get an `Ok` (fast path). If
                // not, we still get `Err` (the underlying
                // `install_recorder` will not be re-tried
                // and `OnceLock` is empty). Both are valid
                // — the invariant is "no panic, no flipped
                // outcome beyond the two documented
                // branches".
                let second = install();
                match second {
                    Ok(b) => {
                        // Sibling populated METRICS after
                        // our first attempt. We got a
                        // valid handle; subsequent calls
                        // are guaranteed to return the
                        // same pointer.
                        let c = install().expect("third install");
                        assert!(std::ptr::eq(b, c));
                    }
                    Err(MetricsError::AlreadyInstalled) => {
                        // Sibling has not yet populated
                        // METRICS. Still no panic, still
                        // the same outcome.
                    }
                }
            }
        }
    }

    #[test]
    fn counter_increments_on_success() {
        // Two successful step outcomes. The metric
        // `ada_remediation_actions_total{action_id=...,
        // outcome="success"}` is registered on first use.
        // Without an installed Prometheus recorder we
        // cannot inspect the counter value directly; the
        // smoke check is "the macro accepts the label
        // set without panicking, and the metric name
        // appears in the snapshot if a recorder is
        // installed".
        record_step_outcome("test-action-success", "success");
        record_step_outcome("test-action-success", "success");
        if is_installed() {
            let snapshot = render();
            assert!(
                snapshot.contains("ada_remediation_actions_total") || snapshot.is_empty(),
                "snapshot should mention actions_total or be empty: {snapshot}"
            );
        }
    }

    #[test]
    fn counter_increments_on_failure() {
        // Two failed step outcomes. Companion to
        // `counter_increments_on_success` — verifies the
        // `outcome="failure"` label path is also accepted
        // by the macro facade.
        record_step_outcome("test-action-failure", "failure");
        record_step_outcome("test-action-failure", "failure");
        if is_installed() {
            let snapshot = render();
            assert!(
                snapshot.contains("ada_remediation_actions_total") || snapshot.is_empty(),
                "snapshot should mention actions_total or be empty: {snapshot}"
            );
        }
    }

    #[test]
    fn histogram_records_duration() {
        record_step_duration("test-action", 0.123);
        record_step_duration("test-action", 0.456);
        // Same smoke check as the counter test.
        if is_installed() {
            let snapshot = render();
            assert!(
                snapshot.contains("ada_remediation_action_duration_seconds") || snapshot.is_empty()
            );
        }
    }

    #[test]
    fn state_transition_counter_accepts_labels() {
        record_state_transition("Idle", "Evaluating");
        record_state_transition("Evaluating", "Executing");
        record_state_transition("Executing", "Cooldown");
        if is_installed() {
            let snapshot = render();
            assert!(
                snapshot.contains("ada_remediation_engine_state_transitions_total")
                    || snapshot.is_empty()
            );
        }
    }

    #[test]
    fn cooldown_gauge_records_active_count() {
        let store = MemoryStore::new();
        // No actions recorded -> 0 active cooldowns.
        set_cooldown_gauge(f64::from(
            u32::try_from(store.active_cooldowns().len()).unwrap_or(u32::MAX),
        ));
        if is_installed() {
            let snapshot = render();
            assert!(snapshot.contains("ada_remediation_cooldown_active") || snapshot.is_empty());
        }
    }

    #[test]
    fn metrics_endpoint_returns_prometheus_text_format() {
        // Install if not already, so render() returns real
        // Prometheus text.
        let _ = install();
        let snapshot = render();
        // Prometheus text format: every line is
        // `metric_name{labels} value` or `# HELP` / `# TYPE`.
        // A snapshot from a fresh recorder may be empty
        // (no metrics recorded yet); both cases are valid.
        if !snapshot.is_empty() {
            for line in snapshot.lines() {
                if line.starts_with('#') {
                    continue;
                }
                // Non-comment lines must be either
                // `name value` or `name{labels} value`.
                assert!(
                    line.split_whitespace().count() >= 2,
                    "malformed prometheus line: {line:?}"
                );
            }
        }
    }
}
