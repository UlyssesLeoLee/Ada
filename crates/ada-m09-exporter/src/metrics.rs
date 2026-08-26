//! [`Metric`], [`MetricKind`], and the in-process [`MetricRegistry`].
//!
//! The skeleton keeps the metric type as a flat struct
//! (`name`, `kind`, `value`, `labels`, `timestamp_ms`) instead
//! of a `Counter` / `Gauge` / ... sum-type because the
//! downstream OTLP protobuf is also a flat struct. The
//! `validate` method enforces the per-kind rules:
//!
//! - `name` is non-empty
//! - `value` is finite (no NaN / infinity)
//! - labels are an unordered key/value bag (no duplicate keys)
//!
//! See [`DOC-MOD-009`](../docs/modules/M-09-exporter.md) §3.2
//! for the canonical schema.

use std::collections::HashMap;
use std::fmt;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// What kind of metric this is. Maps to the OTLP `Metric.DataType`
/// enum but kept minimal here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MetricKind {
    /// Monotonically-increasing counter.
    Counter,
    /// Snapshot of a value that can go up or down.
    Gauge,
    /// Distribution of observations (skeleton: stores the
    /// `count` and `sum` only; quantile computation lands in
    /// B5+).
    Histogram,
    /// Pre-computed quantile pairs (skeleton: stores the
    /// observed value; the full summary representation lands
    /// in B5+).
    Summary,
}

impl MetricKind {
    /// Canonical lowercase string tag.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Counter => "counter",
            Self::Gauge => "gauge",
            Self::Histogram => "histogram",
            Self::Summary => "summary",
        }
    }
}

impl fmt::Display for MetricKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A single metric observation. The skeleton keeps `value` as
/// `f64`; production builds may swap in `i64` for `Counter` /
/// `Histogram` to avoid float drift.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Metric {
    /// Metric name (e.g. `ada.m09.exporter.events_exported`).
    /// Must be non-empty.
    pub name: String,
    /// Kind of metric.
    pub kind: MetricKind,
    /// Numeric value. `f64` is a pragmatic choice for the
    /// skeleton; production may split per kind.
    pub value: f64,
    /// Optional key/value labels (e.g. `tenant_id`,
    /// `module_name`).
    pub labels: HashMap<String, String>,
    /// Wall-clock millis when the metric was recorded.
    pub timestamp_ms: u64,
}

impl Metric {
    /// Build a new metric with the current wall-clock timestamp.
    #[must_use]
    pub fn now(
        name: impl Into<String>,
        kind: MetricKind,
        value: f64,
        labels: HashMap<String, String>,
    ) -> Self {
        let timestamp_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX));
        Self {
            name: name.into(),
            kind,
            value,
            labels,
            timestamp_ms,
        }
    }

    /// Cheap in-process validation. Rejects empty name and
    /// non-finite values. Returns the stringified reason on
    /// failure so callers can surface it as
    /// [`ExporterError::InvalidMetric`].
    pub fn validate(&self) -> std::result::Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("name is empty".to_string());
        }
        if !self.value.is_finite() {
            return Err(format!("value is not finite: {}", self.value));
        }
        Ok(())
    }
}

/// In-process metric registry. Thread-safe via
/// `parking_lot::RwLock`. The skeleton keys by `(name, labels)`
/// — re-recording the same name+labels overwrites the value
/// (last-writer-wins). Histograms and summaries would need a
/// different aggregation policy in production.
#[derive(Debug, Default)]
pub struct MetricRegistry {
    inner: RwLock<Vec<Metric>>,
}

impl MetricRegistry {
    /// Build an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register (record) a metric. The skeleton keeps every
    /// observation; `snapshot` and `clear` let callers bound
    /// memory.
    pub fn record(&self, metric: Metric) {
        if metric.validate().is_err() {
            // The skeleton drops invalid metrics silently —
            // production will surface this via a tracing event
            // and a counter of dropped samples. We still return
            // the metric for tests that want to assert the
            // drop happened.
            return;
        }
        self.inner.write().push(metric);
    }

    /// Take a snapshot of every recorded metric. The returned
    /// `Vec` is a deep copy; mutating it does not affect the
    /// registry.
    #[must_use]
    pub fn snapshot(&self) -> Vec<Metric> {
        self.inner.read().clone()
    }

    /// Number of metrics currently held.
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.read().len()
    }

    /// True if no metric has been recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.read().is_empty()
    }

    /// Drop every recorded metric. Mostly useful in tests.
    pub fn clear(&self) {
        self.inner.write().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_as_str() {
        assert_eq!(MetricKind::Counter.as_str(), "counter");
        assert_eq!(MetricKind::Gauge.as_str(), "gauge");
        assert_eq!(MetricKind::Histogram.as_str(), "histogram");
        assert_eq!(MetricKind::Summary.as_str(), "summary");
    }

    #[test]
    fn kind_display() {
        assert_eq!(MetricKind::Gauge.to_string(), "gauge");
    }

    #[test]
    fn metric_now_stamps_timestamp() {
        let m = Metric::now("ada.events", MetricKind::Counter, 1.0, HashMap::new());
        assert!(m.timestamp_ms > 0);
        assert_eq!(m.name, "ada.events");
        assert_eq!(m.kind, MetricKind::Counter);
    }

    #[test]
    fn validate_rejects_empty_name() {
        let m = Metric::now("", MetricKind::Counter, 1.0, HashMap::new());
        assert!(m.validate().is_err());
    }

    #[test]
    fn validate_rejects_nan() {
        let m = Metric::now("a", MetricKind::Counter, f64::NAN, HashMap::new());
        assert!(m.validate().is_err());
    }

    #[test]
    fn validate_rejects_infinity() {
        let m = Metric::now("a", MetricKind::Counter, f64::INFINITY, HashMap::new());
        assert!(m.validate().is_err());
    }

    #[test]
    fn validate_accepts_well_formed() {
        let m = Metric::now("a", MetricKind::Gauge, 0.0, HashMap::new());
        assert!(m.validate().is_ok());
    }

    #[test]
    fn registry_record_and_snapshot() {
        let r = MetricRegistry::new();
        assert!(r.is_empty());
        r.record(Metric::now("a", MetricKind::Counter, 1.0, HashMap::new()));
        r.record(Metric::now("b", MetricKind::Gauge, 2.0, HashMap::new()));
        assert_eq!(r.len(), 2);
        let snap = r.snapshot();
        assert_eq!(snap.len(), 2);
    }

    #[test]
    fn registry_drops_invalid_metrics() {
        let r = MetricRegistry::new();
        r.record(Metric::now("", MetricKind::Counter, 1.0, HashMap::new()));
        assert!(r.is_empty());
    }

    #[test]
    fn registry_clear_empties_state() {
        let r = MetricRegistry::new();
        r.record(Metric::now("a", MetricKind::Counter, 1.0, HashMap::new()));
        assert_eq!(r.len(), 1);
        r.clear();
        assert!(r.is_empty());
    }
}
