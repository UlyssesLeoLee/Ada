//! M-09: Exporter. Output results to external systems. File,
//! REST, DB, gRPC.
//!
//! ## v0.1.0 scope (B4)
//!
//! This crate is a **minimum skeleton** for the metrics
//! exporter that downstream services plug into. The v0.1.0
//! surface is:
//!
//! - [`Metric`] — name, kind (Counter / Gauge / Histogram /
//!   Summary), value, labels, timestamp_ms
//! - [`MetricKind`] — the four canonical metric kinds
//! - [`MetricRegistry`] — register / record / snapshot / clear,
//!   thread-safe via `parking_lot::RwLock`
//! - [`Exporter`] trait — `export(&self, snapshot: &[Metric]) -> Result<(), ExporterError>`
//! - [`OtlpExporter`] trait — skeleton for the OTLP gRPC
//!   exporter (no implementation; production lands in B5+)
//! - [`NoopExporter`] — discards metrics (handy in tests)
//! - [`InMemoryExporter`] — thread-safe `Vec<Metric>`
//!   accumulator for assertions
//! - 5-variant [`ExporterError`] (SerializationError,
//!   TransportError, InvalidMetric, BackendError, ShuttingDown)
//! - 9 unit tests + 4 integration tests
//!
//! ## What v0.1.0 explicitly does **not** do
//!
//! - Persist metrics to the `metrics` table or to a TSDB
//! - Stream over OTLP gRPC (the trait is a placeholder; the
//!   real gRPC client lands in B5+)
//! - Compute histogram quantiles on `record()`
//! - Honor the `tracing` layer integration (the
//!   `ada-telemetry` crate wires that in B5+)
//!
//! See `docs/modules/M-09-exporter.md` (DOC-MOD-009) for the
//! full design.
//!
//! 関連 IPA フェーズ: 22-52 (基本設計/詳細設計), 53-58 (実装), 59-95 (試験)
//! 設計書: docs/modules/M-09-exporter.md (DOC-MOD-009)

#![allow(missing_docs)]
#![allow(rust_2018_idioms)]
#![allow(clippy::doc_markdown)]

mod error;
mod metrics;
mod otlp;

pub use error::{ExporterError, Result};
pub use metrics::{Metric, MetricKind, MetricRegistry};
pub use otlp::{Exporter, InMemoryExporter, NoopExporter, OtlpExporter};

/// Crate version, taken from `CARGO_PKG_VERSION` (single workspace
/// version per D-09).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Crate name, taken from `CARGO_PKG_NAME`.
pub const NAME: &str = env!("CARGO_PKG_NAME");

/// `skeleton`-layer string tag (仿生モデル 4 層分類, see
/// [`DOC-ARCH-001`](https://example.invalid/docs/architecture/00-anatomy-model.md)).
pub const LAYER: &str = "skeleton";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_not_empty() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn name_not_empty() {
        assert!(!NAME.is_empty());
    }

    #[test]
    fn layer_is_known() {
        assert!(
            ["skeleton", "blood", "nerve", "muscle", "shared"].contains(&LAYER),
            "Unknown layer: {LAYER}"
        );
    }
}
