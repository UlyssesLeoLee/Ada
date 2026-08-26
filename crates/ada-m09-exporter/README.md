# ada-m09-exporter

M-09: エクスポータ (Exporter). Output results to external systems.
File, REST, DB, gRPC.

## v0.1.0 scope (B4 batch)

This crate is a **minimum skeleton** for the metrics exporter.
The v0.1.0 surface is the in-process registry + the
`Exporter` / `OtlpExporter` trait pair that downstream
telemetry pipelines plug into.

The production deployment (OTLP gRPC push, file-rotation
exporter, DB-backed metrics sink, see `DOC-MOD-009` §3.5) is
scheduled for B5+ once G4 (実装着手判定) is approved.

### What v0.1.0 provides

- `Metric` — name, kind, value, labels, timestamp_ms
- `MetricKind` — `Counter / Gauge / Histogram / Summary`
- `MetricRegistry` — `record / snapshot / clear / len /
  is_empty`, thread-safe via `parking_lot::RwLock`
- `Exporter` trait — `export(&self, &[Metric]) -> Result<()>`,
  `name() -> &'static str`
- `OtlpExporter` trait — `endpoint_kind() -> &'static str`
  (skeleton for the gRPC binding)
- `NoopExporter` — discards everything (handy in tests)
- `InMemoryExporter` — thread-safe `Vec<Metric>` accumulator
- 5-variant `ExporterError` (SerializationError, TransportError,
  InvalidMetric, BackendError, ShuttingDown)
- 17 unit tests + 6 integration tests

### What v0.1.0 explicitly does **not** do

- Persist metrics to the `metrics` table or to a TSDB
- Stream over OTLP gRPC (trait is a placeholder; real client
  lands in B5+)
- Compute histogram quantiles on `record()`
- Honor the `tracing` layer integration (the `ada-telemetry`
  crate wires that in B5+)

## 関連 IPA フェーズ

22-52 (基本設計/詳細設計), 53-58 (実装), 59-95 (試験)

## 設計書

`docs/modules/M-09-exporter.md` (DOC-MOD-009)
