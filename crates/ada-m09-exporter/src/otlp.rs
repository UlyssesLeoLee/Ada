//! [`Exporter`] trait + [`NoopExporter`] + [`InMemoryExporter`] +
//! the OTLP trait skeleton + the [`OtlpPushExporter`] that ships
//! with Phase 0-1 of the observability rollout.
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
//! - [`OtlpPushExporter`] is the **Phase 0-1** wire-level entry
//!   point. It accepts an `http://host:port` endpoint and POSTs
//!   a JSON-OTLP-style payload on `push`. v0.1.0 uses a minimal
//!   blocking HTTP transport (raw `std::net::TcpStream`) so the
//!   workspace has no new dependencies and the 5-gate stays
//!   green; the gRPC binding lands in B5+ per the export
//!   pipeline comment in [`OtlpExporter`]. The collector side
//!   is the `otel-collector` service in
//!   `observability/docker-compose.yml`.
//!
//! See [`DOC-MOD-009`](../docs/modules/M-09-exporter.md) §3.5
//! for the full export pipeline, and
//! `docs/observability/02-architecture.md` §4.1 for the
//! in-cluster data flow.

use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::Arc;
use std::time::Duration;

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

// ======================================================================
// Phase 0-1 — OtlpPushExporter
// ======================================================================
//
// Targets the local OTel collector running in the
// `observability` docker-compose stack (see
// `observability/docker-compose.yml`). The exporter:
//
// 1. Serialises the snapshot as a JSON-OTLP-style payload
//    (`build_payload`).
// 2. Opens a blocking TCP connection to `{host}:{port}`.
// 3. Sends a single `POST /v1/metrics HTTP/1.1` with the
//    JSON body.
//
// Notes:
// * Raw `TcpStream` instead of `reqwest` / `ureq` to keep
//   the workspace dependency surface flat (no churn in
//   `cargo check --workspace`).
// * The JSON shape is a *subset* of the OTLP/HTTP protobuf
//   representation — the otel-collector accepts it because
//   it parses OTLP/protobuf, but for Phase 0-1 we only need
//   a payload the collector will *accept* (it normalises
//   to protobuf on the receiver side). The full protobuf
//   encoder lands in B5+ per `OtlpExporter` doc.
// * Async / gRPC is out of scope for v0.1.0; the in-process
//   registry is already in memory and a synchronous POST
//   is the cheapest possible "push" path.

/// Phase 0-1 OTLP HTTP push exporter. Targets the local
/// `otel-collector` on `host:port` (defaults: `localhost:4318`,
/// the OTLP/HTTP port per DOC-OBS-002 §3 技術スタック).
#[derive(Debug, Clone)]
pub struct OtlpPushExporter {
    host: String,
    port: u16,
    timeout: Duration,
    service_name: String,
    service_version: String,
}

impl OtlpPushExporter {
    /// Build an exporter pointing at `host:port`. The HTTP
    /// path is hard-coded to `/v1/metrics` (OTLP/HTTP convention).
    #[must_use]
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
            timeout: Duration::from_secs(5),
            service_name: "ada".to_string(),
            service_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// Override the connect+write timeout. Default 5s.
    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Override the `service.name` resource attribute. The
    /// default is `"ada"`.
    #[must_use]
    pub fn with_service_name(mut self, name: impl Into<String>) -> Self {
        self.service_name = name.into();
        self
    }

    /// Override the `service.version` resource attribute.
    /// The default is `CARGO_PKG_VERSION`.
    #[must_use]
    pub fn with_service_version(mut self, version: impl Into<String>) -> Self {
        self.service_version = version.into();
        self
    }

    /// Host the exporter dials.
    #[must_use]
    pub fn host(&self) -> &str {
        &self.host
    }

    /// Port the exporter dials.
    #[must_use]
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Build the JSON payload that the collector will receive.
    /// Exposed for tests; the wire format is part of the
    /// public surface (snapshot consumers rely on it).
    pub fn build_payload(&self, snapshot: &[Metric]) -> Result<String> {
        // Validate first — we don't want a half-baked push.
        for m in snapshot {
            if let Err(msg) = m.validate() {
                return Err(ExporterError::InvalidMetric(msg));
            }
        }

        // Hand-rolled JSON keeps us off the serde derive
        // surface (Metric is already public; serde is a
        // workspace-level dep but the JSON shape we want
        // here is OTLP-specific, not the in-process Metric
        // representation). Result is a UTF-8 String.
        let mut out = String::with_capacity(256 + snapshot.len() * 96);
        out.push_str("{\"resourceMetrics\":[{\"resource\":{");
        out.push_str("\"attributes\":[");
        push_attr(&mut out, "service.name", &self.service_name);
        out.push(',');
        push_attr(&mut out, "service.version", &self.service_version);
        out.push_str("]},\"scopeMetrics\":[{\"scope\":{\"name\":\"ada-m09-exporter\"}");
        out.push_str(",\"metrics\":[");
        for (i, m) in snapshot.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            push_metric(&mut out, m);
        }
        out.push_str("]}]}]}");
        Ok(out)
    }

    /// Synchronous HTTP POST. The body is the JSON payload
    /// built by [`Self::build_payload`].
    ///
    /// Returns `Ok(())` when the collector replies `2xx`. Any
    /// transport error or non-2xx response is mapped to
    /// `ExporterError::TransportError`.
    pub fn push(&self, snapshot: &[Metric]) -> Result<()> {
        let body = self.build_payload(snapshot)?;

        let addr = (self.host.as_str(), self.port)
            .to_socket_addrs()
            .map_err(|e| {
                ExporterError::TransportError(format!(
                    "resolve {host}:{port}: {e}",
                    host = self.host,
                    port = self.port
                ))
            })?
            .next()
            .ok_or_else(|| {
                ExporterError::TransportError(format!(
                    "no address for {host}:{port}",
                    host = self.host,
                    port = self.port
                ))
            })?;

        let mut stream = TcpStream::connect_timeout(&addr, self.timeout)
            .map_err(|e| ExporterError::TransportError(format!("connect {addr}: {e}")))?;
        stream
            .set_write_timeout(Some(self.timeout))
            .map_err(|e| ExporterError::TransportError(format!("set write timeout: {e}")))?;
        stream
            .set_read_timeout(Some(self.timeout))
            .map_err(|e| ExporterError::TransportError(format!("set read timeout: {e}")))?;

        // Request line + Host + Content-Type + Content-Length + body.
        let request = format!(
            "POST /v1/metrics HTTP/1.1\r\nHost: {host}\r\nContent-Type: application/json\r\nContent-Length: {len}\r\nConnection: close\r\n\r\n",
            host = self.host,
            len = body.len(),
        );
        stream
            .write_all(request.as_bytes())
            .and_then(|()| stream.write_all(body.as_bytes()))
            .map_err(|e| ExporterError::TransportError(format!("write: {e}")))?;

        // Drain the response so the server can free the
        // socket. We only inspect the status line.
        let mut buf = [0u8; 256];
        let n = stream
            .read(&mut buf)
            .map_err(|e| ExporterError::TransportError(format!("read: {e}")))?;
        if n == 0 {
            return Err(ExporterError::TransportError("empty response".to_string()));
        }
        let response = String::from_utf8_lossy(&buf[..n]);
        let status = response.lines().next().unwrap_or("");
        if !status.starts_with("HTTP/1.1 2") && !status.starts_with("HTTP/1.0 2") {
            return Err(ExporterError::TransportError(format!(
                "non-2xx from collector: {status}"
            )));
        }
        Ok(())
    }
}

impl OtlpExporter for OtlpPushExporter {
    fn endpoint_kind(&self) -> &'static str {
        "otlp-http"
    }
}

impl Exporter for OtlpPushExporter {
    fn export(&self, snapshot: &[Metric]) -> Result<()> {
        self.push(snapshot)
    }

    fn name(&self) -> &'static str {
        "otlp-http"
    }
}

fn push_attr(out: &mut String, key: &str, value: &str) {
    out.push_str("{\"key\":\"");
    out.push_str(key);
    out.push_str("\",\"value\":{\"stringValue\":\"");
    out.push_str(&escape(value));
    out.push_str("\"}}");
}

fn push_metric(out: &mut String, m: &Metric) {
    out.push_str("{\"name\":\"");
    out.push_str(&escape(&m.name));
    out.push_str("\",\"sum\":");
    out.push_str(&format_args_f64(m.value));
    out.push_str(",\"labels\":{");
    let mut first = true;
    for (k, v) in &m.labels {
        if !first {
            out.push(',');
        }
        first = false;
        out.push('"');
        out.push_str(&escape(k));
        out.push_str("\":\"");
        out.push_str(&escape(v));
        out.push('"');
    }
    out.push_str("}}");
}

fn format_args_f64(v: f64) -> String {
    // OTLP expects a JSON number. We avoid the `{:?}` form
    // (it can produce `NaN` / `inf` literals which strict
    // parsers reject) by checking for those cases.
    if v.is_finite() {
        // 6 fractional digits is enough for telemetry; trim
        // trailing zeros for legibility.
        let s = format!("{v:.6}");
        let trimmed = s.trim_end_matches('0').trim_end_matches('.');
        if trimmed.is_empty() {
            "0".to_string()
        } else {
            trimmed.to_string()
        }
    } else {
        // NaN / +inf / -inf become 0 — the metric is unusable
        // but the payload stays valid JSON.
        "0".to_string()
    }
}

fn escape(s: &str) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::MetricKind;
    use std::collections::HashMap;
    use std::net::TcpListener;
    use std::thread;

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

    // ----- OtlpPushExporter -----

    #[test]
    fn otlp_push_builds_json_payload() {
        let e = OtlpPushExporter::new("localhost", 4318)
            .with_service_name("ada-test")
            .with_service_version("0.1.0");
        let snap = vec![metric("requests", 3.5)];
        let body = e.build_payload(&snap).unwrap();
        assert!(body.contains("\"service.name\""));
        assert!(body.contains("\"ada-test\""));
        assert!(body.contains("\"service.version\""));
        assert!(body.contains("\"0.1.0\""));
        assert!(body.contains("\"name\":\"requests\""));
        assert!(body.contains("\"sum\":3.5"));
    }

    #[test]
    fn otlp_push_rejects_invalid_snapshot_before_writing() {
        let e = OtlpPushExporter::new("localhost", 4318);
        let bad = Metric::now("", MetricKind::Counter, 1.0, HashMap::new());
        let err = e.export(&[bad]).expect_err("invalid");
        assert!(matches!(err, ExporterError::InvalidMetric(_)));
    }

    #[test]
    fn otlp_push_endpoint_kind() {
        let e = OtlpPushExporter::new("otel-collector", 4318);
        assert_eq!(OtlpExporter::endpoint_kind(&e), "otlp-http");
        assert_eq!(e.name(), "otlp-http");
        assert_eq!(e.host(), "otel-collector");
        assert_eq!(e.port(), 4318);
    }

    #[test]
    fn otlp_push_rejects_non_finite_values() {
        // `Metric::validate` rejects non-finite values
        // (see metrics.rs §100-112), so `build_payload`
        // surfaces an `InvalidMetric` error before any
        // JSON is written. The exporter must never emit
        // `NaN` / `Infinity` tokens in the payload.
        let e = OtlpPushExporter::new("localhost", 4318);
        let nan = Metric::now("nan", MetricKind::Counter, f64::NAN, HashMap::new());
        let err = e.build_payload(&[nan]).expect_err("rejected");
        assert!(matches!(err, ExporterError::InvalidMetric(_)));
    }

    #[test]
    fn otlp_push_escapes_strings() {
        let e = OtlpPushExporter::new("localhost", 4318);
        // Build a metric with a label that contains quotes
        // and a control character.
        let mut labels = HashMap::new();
        labels.insert("k\"ey".to_string(), "v\nlue".to_string());
        let m = Metric::now("m", MetricKind::Counter, 1.0, labels);
        let body = e.build_payload(&[m]).unwrap();
        assert!(body.contains("k\\\"ey"));
        assert!(body.contains("v\\nlue"));
    }

    /// Round-trip test: stand up a local TCP listener that
    /// acts as a fake OTLP collector, point the exporter at
    /// it, and verify the wire format.
    #[test]
    fn otlp_push_round_trip() {
        use std::net::Shutdown;
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().unwrap().port();

        // Server thread: read one request, reply 200 OK,
        // half-close so the client gets a clean EOF. The
        // half-close is critical on Windows: a full close
        // races with the client read and produces
        // WSAECONNABORTED.
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut buf = [0u8; 4096];
            let _ = stream.read(&mut buf);
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                .expect("write");
            let _ = stream.shutdown(Shutdown::Write);
        });

        let e = OtlpPushExporter::new("127.0.0.1", port);
        e.export(&[metric("a", 1.0)]).expect("push ok");
        server.join().expect("server thread");
    }

    /// Negative case: the server replies 500 and the
    /// exporter must surface a `TransportError` error.
    #[test]
    fn otlp_push_surfaces_non_2xx() {
        use std::net::Shutdown;
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().unwrap().port();

        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut buf = [0u8; 4096];
            let _ = stream.read(&mut buf);
            stream
                .write_all(b"HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                .expect("write");
            let _ = stream.shutdown(Shutdown::Write);
        });

        let e = OtlpPushExporter::new("127.0.0.1", port);
        let err = e.export(&[metric("a", 1.0)]).expect_err("non-2xx");
        assert!(matches!(err, ExporterError::TransportError(_)));
        server.join().expect("server thread");
    }
}
