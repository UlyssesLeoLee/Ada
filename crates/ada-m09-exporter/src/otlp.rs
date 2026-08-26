//! [`Exporter`] trait + [`NoopExporter`] + [`InMemoryExporter`]
//! + the OTLP trait skeleton.
//!
//! The trait surface is intentionally minimal:
//!
//! - [`Exporter::export`] is synchronous, takes a `&[Metric]`
//!   slice, and returns `Result<(), ExporterError>`. v0.1.0
//!   keeps the call sync because the in-process registry
//!   snapshot is already in memory; production OTLP gRPC will
//!   add an async variant in B5+.
//! - [`OtlpExporter`] is a separate trait so the gRPC binding
//!   has a place to grow without churning the generic
//!   `Exporter` interface.
//!
//! See [`DOC-MOD-009`](../docs/modules/M-09-exporter.md) §3.5
//! for the full export pipeline.

use std::sync::Arc;

use parking_lot::Mutex;

use crate::error::{ExporterError, Result};
use crate::metrics::Metric;

/// The trait every exporter implements. Synchronous: pass the
/// snapshot in, get the result back. v0.1.0 keeps the call
/// sync because the in-process snapshot is already in memory
/// and async would not buy us anything yet.
pub trait Exporter: Send + Sync {
    /// Export `snapshot`. Returns `Ok(())` on success,
    /// `Err(ExporterError)` on failure.
    fn export(&self, snapshot: &[Metric]) -> Result<()>;

    /// Human-readable exporter name, used in tracing spans and
    /// test assertions.
    fn name(&self) -> &'static str;
}

/// The OTLP trait is a separate surface so the gRPC binding
/// (B5+) can grow without churning the generic `Exporter`
/// contract. The v0.1.0 skeleton has no impl; the type is
/// here so the trait composition is in place.
pub trait OtlpExporter: Send + Sync {
    /// OTLP service name (e.g. `otlp-grpc`, `otlp-http`).
    fn endpoint_kind(&self) -> &'static str;
}

/// Exporter that discards every metric. Useful in tests that
/// only care about the *fact* that a metric was emitted, not
/// the value.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopExporter;

impl Exporter for NoopExporter {
    fn export(&self, _snapshot: &[Metric]) -> Result<()> {
        Ok(())
    }

    fn name(&self) -> &'static str {
        "noop"
    }
}

impl OtlpExporter for NoopExporter {
    fn endpoint_kind(&self) -> &'static str {
        "noop"
    }
}

/// Thread-safe in-memory accumulator. Every `export` call
/// appends the snapshot to an internal `Vec<Metric>`. Tests
/// use it to assert that "exactly these metrics were emitted"
/// or "the exporter was called N times".
#[derive(Debug, Default, Clone)]
pub struct InMemoryExporter {
    inner: Arc<Mutex<Vec<Metric>>>,
}

impl InMemoryExporter {
    /// Build an empty in-memory exporter.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Borrow the accumulated metrics (in export-call order).
    /// Returns a deep copy.
    #[must_use]
    pub fn accumulated(&self) -> Vec<Metric> {
        self.inner.lock().clone()
    }

    /// Number of metrics currently held.
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.lock().len()
    }

    /// True if no metric has been exported.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.lock().is_empty()
    }

    /// Drop every accumulated metric. Mostly useful in tests.
    pub fn clear(&self) {
        self.inner.lock().clear();
    }
}

impl Exporter for InMemoryExporter {
    fn export(&self, snapshot: &[Metric]) -> Result<()> {
        let mut guard = self.inner.lock();
        for m in snapshot {
            if let Err(msg) = m.validate() {
                return Err(ExporterError::InvalidMetric(msg));
            }
            guard.push(m.clone());
        }
        Ok(())
    }

    fn name(&self) -> &'static str {
        "in-memory"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::MetricKind;
    use std::collections::HashMap;

    fn metric(name: &str, value: f64) -> Metric {
        Metric::now(name, MetricKind::Counter, value, HashMap::new())
    }

    #[test]
    fn noop_export_succeeds_with_any_snapshot() {
        let e = NoopExporter;
        let snap = vec![metric("a", 1.0), metric("b", 2.0)];
        assert!(e.export(&snap).is_ok());
        assert_eq!(e.name(), "noop");
    }

    #[test]
    fn noop_export_succeeds_with_empty_snapshot() {
        let e = NoopExporter;
        assert!(e.export(&[]).is_ok());
    }

    #[test]
    fn in_memory_export_appends() {
        let e = InMemoryExporter::new();
        assert!(e.is_empty());
        e.export(&[metric("a", 1.0)]).unwrap();
        e.export(&[metric("b", 2.0), metric("c", 3.0)]).unwrap();
        assert_eq!(e.len(), 3);
        let acc = e.accumulated();
        assert_eq!(acc[0].name, "a");
        assert_eq!(acc[1].name, "b");
        assert_eq!(acc[2].name, "c");
    }

    #[test]
    fn in_memory_export_rejects_invalid_metric() {
        let e = InMemoryExporter::new();
        let bad = Metric::now("", MetricKind::Counter, 1.0, HashMap::new());
        let err = e.export(&[bad]).expect_err("invalid");
        assert!(matches!(err, ExporterError::InvalidMetric(_)));
        assert!(e.is_empty(), "failed export must not write");
    }

    #[test]
    fn in_memory_clear_empties_state() {
        let e = InMemoryExporter::new();
        e.export(&[metric("a", 1.0)]).unwrap();
        e.clear();
        assert!(e.is_empty());
    }

    #[test]
    fn in_memory_default_is_empty() {
        let e = InMemoryExporter::default();
        assert!(e.is_empty());
        assert_eq!(e.name(), "in-memory");
    }

    #[test]
    fn noop_endpoint_kind() {
        let e = NoopExporter;
        assert_eq!(OtlpExporter::endpoint_kind(&e), "noop");
    }
}
