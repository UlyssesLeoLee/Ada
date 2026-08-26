# ada-m01-acquisition

M-01: データ取得アダプタ (Data acquisition adapters).
REST, DB CDC, gRPC, WebSocket, File. Plugin-based.

## v0.1.0 scope (B5 batch)

This crate is the **minimum skeleton** for the data
acquisition layer. The v0.1.0 surface is the in-process
pull-from-source contract that the downstream
`ada-m02-normalizer` and `ada-m15-central-event-bus`
crate plug into.

The production deployment (real `reqwest`-backed HTTP
poller, SQLx CDC connector, Kafka consumer, see
`DOC-MOD-001` §3.3) is scheduled for B5+ once G4
(実装着手判定) is approved.

### What v0.1.0 provides

- `SourceDescriptor` — id, kind, endpoint, optional
  credentials, `poll_interval_ms`, `batch_size`
- `SourceKind` — `Http / File / Stdin / Database /
  MessageQueue` (the last two are reserved for B5+)
- `Connector` trait —
  `async fn fetch() -> Result<Vec<RawRecord>>`
- `HttpConnector` — in-process mock that returns a fixed
  sample payload; real HTTP is B5+
- `FileConnector` — NDJSON-over-`std::fs` reader
- `StdInConnector` — NDJSON-over-`std::io::Read` reader
  (consumable from any source, not just stdin)
- `RawRecord` — `source_id + seq + payload`
- 5-variant `AcquisitionError` (SourceUnavailable,
  AuthenticationFailed, RateLimited, InvalidPayload,
  BackendError)
- 8 unit tests + 4 integration tests

### What v0.1.0 explicitly does **not** do

- Persist records into a Kafka topic or queue
- Perform real HTTP requests (`HttpConnector` is an
  in-process mock; production lands in B5+ with `reqwest`)
- Stream backpressure / flow control
- Schema validation against a registered schema
- Concurrent fetch from multiple sources in one process

## 関連 IPA フェーズ

22-52 (基本設計/詳細設計), 53-58 (実装), 59-95 (試験)

## 設計書

`docs/modules/M-01-acquisition.md` (DOC-MOD-001)
