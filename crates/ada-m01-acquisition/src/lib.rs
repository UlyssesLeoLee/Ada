//! M-01: Data acquisition adapters. REST, DB CDC, gRPC,
//! WebSocket, File. Plugin-based.
//!
//! ## v0.1.0 scope (B5 batch)
//!
//! This crate is the **minimum skeleton** for the
//! acquisition layer. The v0.1.0 surface is the in-process
//! pull-from-source contract that the downstream
//! normalizer (`ada-m02-normalizer`) and the central event
//! bus (`ada-m15-central-event-bus`) plug into.
//!
//! - [`SourceDescriptor`] — id, kind, endpoint, optional
//!   credentials, poll interval, batch size
//! - [`SourceKind`] — `Http / File / Stdin / Database /
//!   MessageQueue` (the last two are reserved for B5+)
//! - [`Connector`] trait — `async fn fetch() -> Result<Vec<RawRecord>>`
//! - [`HttpConnector`] — in-process mock; real HTTP is B5+
//! - [`FileConnector`] — NDJSON-over-`std::fs` reader
//! - [`StdInConnector`] — NDJSON-over-`std::io::Read` reader
//!   (consumable from any source, not just stdin)
//! - [`RawRecord`] — `source_id + seq + payload`
//! - 5-variant [`AcquisitionError`] (SourceUnavailable,
//!   AuthenticationFailed, RateLimited, InvalidPayload,
//!   BackendError)
//! - 8 unit tests + 4 integration tests
//!
//! ## What v0.1.0 explicitly does **not** do
//!
//! - Persist records into a Kafka topic or queue
//! - Perform real HTTP requests (the [`HttpConnector`] is an
//!   in-process mock; production lands in B5+ with
//!   `reqwest`)
//! - Stream backpressure / flow control
//! - Schema validation against a registered schema
//! - Concurrent fetch from multiple sources in one process
//!   (the production poller will use `tokio::spawn` to
//!   fan-out)
//!
//! See `docs/modules/M-01-acquisition.md` (DOC-MOD-001) for
//! the full design.
//!
//! 関連 IPA フェーズ: 22-52 (基本設計/詳細設計), 53-58 (実装), 59-95 (試験)
//! 設計書: docs/modules/M-01-acquisition.md (DOC-MOD-001)

#![allow(missing_docs)]
#![allow(rust_2018_idioms)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

mod connector;
mod error;
mod source;

pub use connector::{Connector, FileConnector, HttpConnector, RawRecord, StdInConnector};
pub use error::{AcquisitionError, Result};
pub use source::{SourceDescriptor, SourceKind};

/// Crate version, taken from `CARGO_PKG_VERSION` (single workspace
/// version per D-09).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Crate name, taken from `CARGO_PKG_NAME`.
pub const NAME: &str = env!("CARGO_PKG_NAME");

/// `blood`-layer string tag (仿生モデル 4 層分類, see
/// [`DOC-ARCH-001`](https://example.invalid/docs/architecture/00-anatomy-model.md)).
pub const LAYER: &str = "blood";

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
