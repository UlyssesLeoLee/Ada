//! [`Connector`] trait + the three concrete in-process impls.
//!
//! The v0.1.0 skeleton ships three connectors:
//!
//! - [`HttpConnector`] — an **in-process mock**. The production
//!   build (B5+) will swap it for a `reqwest`-backed
//!   implementation; until then the connector parses the
//!   endpoint as `<scheme>://<host>/<path>` and returns a
//!   fixed sample payload so the rest of the acquisition
//!   pipeline can be exercised end-to-end.
//! - [`FileConnector`] — reads a local file via
//!   [`std::fs::read_to_string`], one [`RawRecord`] per
//!   newline-delimited JSON line (NDJSON). The v0.1.0
//!   skeleton honours the most common ingestion format
//!   (JSONL); CSV is B5+.
//! - [`StdInConnector`] — reads a single NDJSON document from
//!   a supplied [`std::io::Read`] (typically [`std::io::stdin`],
//!   but tests pass a `&[u8]` slice).
//!
//! All three implement the same async [`Connector::fetch`]
//! signature:
//!
//! ```text
//! async fn fetch(&self) -> Result<Vec<RawRecord>, AcquisitionError>
//! ```
//!
//! The trait is `async_trait` (like the rest of the workspace)
//! so a future DB-backed connector can hold open a connection
//! without changing the call sites.
//!
//! See [`DOC-MOD-001`](../docs/modules/M-01-acquisition.md) §3.3
//! for the full connector contract.

use std::fmt;
use std::io::Read;
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::error::{AcquisitionError, Result};
use crate::source::SourceDescriptor;

/// A single raw record returned by a [`Connector::fetch`] call.
///
/// The v0.1.0 skeleton keeps the payload as a flat
/// `serde_json::Value` and tags it with the source `id` and a
/// per-record `seq` so downstream code can correlate the
/// payload back to the descriptor without re-parsing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RawRecord {
    /// Source `id` this record came from.
    pub source_id: String,
    /// Zero-based per-source sequence number (assigned by the
    /// connector at fetch time).
    pub seq: u64,
    /// Opaque payload. Production builds will tighten the
    /// type (e.g. `serde_json::Value` for JSON, `Vec<u8>` for
    /// binary).
    pub payload: serde_json::Value,
}

impl RawRecord {
    /// Build a new record. Mostly a convenience for tests and
    /// the in-process mock connectors.
    #[must_use]
    pub fn new(source_id: impl Into<String>, seq: u64, payload: serde_json::Value) -> Self {
        Self {
            source_id: source_id.into(),
            seq,
            payload,
        }
    }
}

/// The connector trait every acquisition adapter implements.
///
/// v0.1.0 keeps the surface intentionally small: a single
/// `fetch` call that returns a batch of [`RawRecord`]s. The
/// production build will grow the trait with `name()`,
/// `health_check()`, and `close()` methods, but the skeleton
/// shape is enough for the ingestion tests.
#[async_trait]
pub trait Connector: Send + Sync {
    /// Fetch a batch of records. The returned `Vec` may be
    /// empty (e.g. an empty file, no messages on the queue).
    async fn fetch(&self) -> Result<Vec<RawRecord>>;
}

// ---------------------------------------------------------------------------
// HttpConnector
// ---------------------------------------------------------------------------

/// In-process HTTP connector mock.
///
/// The v0.1.0 skeleton does **not** perform a real HTTP
/// request — production-grade HTTP fetch is B5+ work that
/// depends on `reqwest` and a connection pool. The mock
/// parses `endpoint` as `<scheme>://<host>/<path>` and
/// returns a fixed sample payload so callers can exercise
/// the rest of the ingestion pipeline today.
///
/// The connector holds an optional `status_override` that
/// tests use to simulate failure modes:
/// - `Some(401)` → [`AcquisitionError::AuthenticationFailed`]
/// - `Some(429)` → [`AcquisitionError::RateLimited`]
/// - `Some(500)` → [`AcquisitionError::SourceUnavailable`]
/// - `Some(other)` → [`AcquisitionError::SourceUnavailable`]
/// - `None` → success, returns one [`RawRecord`].
#[derive(Debug)]
pub struct HttpConnector {
    descriptor: SourceDescriptor,
    /// Per-call status override for tests. `None` = success.
    status_override: Mutex<Option<u16>>,
}

impl HttpConnector {
    /// Build a new in-process HTTP connector for `descriptor`.
    /// `descriptor.kind` must be [`crate::SourceKind::Http`].
    #[must_use]
    pub fn new(descriptor: SourceDescriptor) -> Self {
        Self {
            descriptor,
            status_override: Mutex::new(None),
        }
    }

    /// Set the next `fetch` call's status code. The override
    /// is consumed on the next call and reset to `None`.
    /// Tests use this to exercise the failure paths without
    /// spinning up a real HTTP server.
    pub fn set_status_override(&self, status: Option<u16>) {
        *self.status_override.lock() = status;
    }

    /// Build a sample success payload. The skeleton hard-codes
    /// a small JSON object so callers can assert the
    /// "ingest end-to-end" path without a real server.
    #[must_use]
    pub fn sample_payload(endpoint: &str) -> serde_json::Value {
        serde_json::json!({
            "endpoint": endpoint,
            "sampled_at_ms": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX)),
            "items": [
                {"id": 1, "name": "alpha"},
                {"id": 2, "name": "beta"},
            ],
        })
    }
}

#[async_trait]
impl Connector for HttpConnector {
    async fn fetch(&self) -> Result<Vec<RawRecord>> {
        // Take (and clear) the override.
        let status = self.status_override.lock().take();
        if let Some(code) = status {
            return Err(match code {
                401 | 403 => AcquisitionError::AuthenticationFailed(format!("HTTP {code}")),
                429 => AcquisitionError::RateLimited("HTTP 429".to_string()),
                _ => AcquisitionError::SourceUnavailable(format!("HTTP {code}")),
            });
        }
        let record = RawRecord::new(
            self.descriptor.id.clone(),
            0,
            Self::sample_payload(&self.descriptor.endpoint),
        );
        Ok(vec![record])
    }
}

// ---------------------------------------------------------------------------
// FileConnector
// ---------------------------------------------------------------------------

/// File-backed connector. Reads the file as UTF-8 and parses
/// each non-empty line as a JSON value. Empty / whitespace-only
/// lines are skipped. A parse failure surfaces as
/// [`AcquisitionError::InvalidPayload`].
#[derive(Debug)]
pub struct FileConnector {
    descriptor: SourceDescriptor,
    path: PathBuf,
    /// Monotonic per-connector counter so two `fetch` calls
    /// on the same connector return distinct `seq` values.
    next_seq: Mutex<u64>,
}

impl FileConnector {
    /// Build a new file connector. `descriptor.kind` should
    /// be [`crate::SourceKind::File`]; the connector uses
    /// `descriptor.endpoint` as the file path.
    #[must_use]
    pub fn new(descriptor: SourceDescriptor) -> Self {
        let path = PathBuf::from(&descriptor.endpoint);
        Self {
            descriptor,
            path,
            next_seq: Mutex::new(0),
        }
    }

    /// Borrow the file path the connector is bound to.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[async_trait]
impl Connector for FileConnector {
    async fn fetch(&self) -> Result<Vec<RawRecord>> {
        let contents = std::fs::read_to_string(&self.path).map_err(|e| {
            AcquisitionError::BackendError(format!("read {}: {e}", self.path.display()))
        })?;
        let mut out = Vec::new();
        let mut seq = self.next_seq.lock();
        for (lineno, raw) in contents.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() {
                continue;
            }
            let value: serde_json::Value = serde_json::from_str(line).map_err(|e| {
                AcquisitionError::InvalidPayload(format!("line {}: {e}", lineno + 1))
            })?;
            out.push(RawRecord::new(self.descriptor.id.clone(), *seq, value));
            *seq = seq.saturating_add(1);
        }
        Ok(out)
    }
}

// ---------------------------------------------------------------------------
// StdInConnector
// ---------------------------------------------------------------------------

/// Stdin-style connector that reads NDJSON from any
/// [`std::io::Read`].
///
/// The v0.1.0 skeleton does **not** call
/// [`std::io::stdin`] directly; instead, the caller supplies
/// a boxed reader (which is trivially `std::io::stdin()` in
/// production). The reader is consumed on the first
/// `fetch` call so the connector is one-shot.
pub struct StdInConnector {
    descriptor: SourceDescriptor,
    reader: Mutex<Option<Box<dyn Read + Send>>>,
    /// Monotonic per-connector counter.
    next_seq: Mutex<u64>,
}

impl fmt::Debug for StdInConnector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StdInConnector")
            .field("descriptor", &self.descriptor)
            .field("has_reader", &self.reader.lock().is_some())
            .field("next_seq", &self.next_seq)
            .finish()
    }
}

impl StdInConnector {
    /// Build a new stdin connector that reads from `reader`
    /// on the next `fetch` call.
    pub fn new(descriptor: SourceDescriptor, reader: Box<dyn Read + Send>) -> Self {
        Self {
            descriptor,
            reader: Mutex::new(Some(reader)),
            next_seq: Mutex::new(0),
        }
    }

    /// Convenience constructor that reads from a static byte
    /// slice. Tests use this; production code passes
    /// `Box::new(std::io::stdin())`.
    pub fn from_slice(descriptor: SourceDescriptor, bytes: &'static [u8]) -> Self {
        Self::new(descriptor, Box::new(bytes))
    }
}

#[async_trait]
impl Connector for StdInConnector {
    async fn fetch(&self) -> Result<Vec<RawRecord>> {
        let mut reader = self
            .reader
            .lock()
            .take()
            .ok_or_else(|| AcquisitionError::BackendError("reader already consumed".into()))?;
        let mut buf = String::new();
        reader
            .read_to_string(&mut buf)
            .map_err(|e| AcquisitionError::BackendError(format!("read stdin: {e}")))?;
        let mut out = Vec::new();
        let mut seq = self.next_seq.lock();
        for (lineno, raw) in buf.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() {
                continue;
            }
            let value: serde_json::Value = serde_json::from_str(line).map_err(|e| {
                AcquisitionError::InvalidPayload(format!("line {}: {e}", lineno + 1))
            })?;
            out.push(RawRecord::new(self.descriptor.id.clone(), *seq, value));
            *seq = seq.saturating_add(1);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::SourceKind;

    fn desc(kind: SourceKind, endpoint: &str) -> SourceDescriptor {
        SourceDescriptor::new("src-1", kind, endpoint)
    }

    #[test]
    fn raw_record_builder_round_trip() {
        let r = RawRecord::new("a", 7, serde_json::json!({"k": "v"}));
        assert_eq!(r.source_id, "a");
        assert_eq!(r.seq, 7);
        assert_eq!(r.payload, serde_json::json!({"k": "v"}));
    }

    #[tokio::test]
    async fn http_connector_success_returns_one_record() {
        let c = HttpConnector::new(desc(SourceKind::Http, "https://api.example.com/items"));
        let records = c.fetch().await.expect("ok");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].source_id, "src-1");
        assert_eq!(records[0].seq, 0);
    }

    #[tokio::test]
    async fn http_connector_status_override_auth_failed() {
        let c = HttpConnector::new(desc(SourceKind::Http, "https://x"));
        c.set_status_override(Some(401));
        let err = c.fetch().await.expect_err("401");
        assert!(matches!(err, AcquisitionError::AuthenticationFailed(_)));
    }

    #[tokio::test]
    async fn http_connector_status_override_rate_limited() {
        let c = HttpConnector::new(desc(SourceKind::Http, "https://x"));
        c.set_status_override(Some(429));
        let err = c.fetch().await.expect_err("429");
        assert!(matches!(err, AcquisitionError::RateLimited(_)));
    }

    #[tokio::test]
    async fn http_connector_status_override_unavailable() {
        let c = HttpConnector::new(desc(SourceKind::Http, "https://x"));
        c.set_status_override(Some(500));
        let err = c.fetch().await.expect_err("500");
        assert!(matches!(err, AcquisitionError::SourceUnavailable(_)));
    }

    #[tokio::test]
    async fn http_connector_status_override_is_consumed() {
        let c = HttpConnector::new(desc(SourceKind::Http, "https://x"));
        c.set_status_override(Some(500));
        let _ = c.fetch().await;
        // Second call must NOT see the override.
        let records = c.fetch().await.expect("ok");
        assert_eq!(records.len(), 1);
    }

    #[test]
    fn http_connector_sample_payload_includes_endpoint() {
        let v = HttpConnector::sample_payload("https://x");
        assert_eq!(v["endpoint"], serde_json::json!("https://x"));
        assert!(v["items"].is_array());
    }

    #[test]
    fn file_connector_path_accessor() {
        let c = FileConnector::new(desc(SourceKind::File, "/tmp/x.ndjson"));
        assert_eq!(c.path(), Path::new("/tmp/x.ndjson"));
    }

    #[tokio::test]
    async fn file_connector_missing_file_is_backend_error() {
        let c = FileConnector::new(desc(
            SourceKind::File,
            "Z:/definitely-missing-zzz-12345.ndjson",
        ));
        let err = c.fetch().await.expect_err("missing");
        assert!(matches!(err, AcquisitionError::BackendError(_)));
    }

    #[tokio::test]
    async fn stdin_connector_reader_is_consumed() {
        let c = StdInConnector::from_slice(desc(SourceKind::Stdin, "-"), b"");
        let _ = c.fetch().await.expect("first");
        let err = c.fetch().await.expect_err("gone");
        assert!(matches!(err, AcquisitionError::BackendError(_)));
    }

    #[tokio::test]
    async fn stdin_connector_invalid_json_is_invalid_payload() {
        let bytes: &'static [u8] = b"not-json\n";
        let c = StdInConnector::from_slice(desc(SourceKind::Stdin, "-"), bytes);
        let err = c.fetch().await.expect_err("bad");
        assert!(matches!(err, AcquisitionError::InvalidPayload(_)));
    }
}
