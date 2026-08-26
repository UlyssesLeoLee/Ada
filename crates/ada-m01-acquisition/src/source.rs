//! [`SourceDescriptor`] and [`SourceKind`] for the acquisition
//! adapters.
//!
//! A [`SourceDescriptor`] is the immutable record the
//! acquisition layer hands to a [`Connector`](crate::Connector) to
//! tell it *what* to fetch. The v0.1.0 skeleton keeps the surface
//! minimal:
//!
//! - `id` — a stable, caller-chosen identifier (e.g.
//!   `"ingest-csv-orders"`).
//! - `kind` — [`SourceKind`] (Http / File / Stdin / Database /
//!   MessageQueue).
//! - `endpoint` — connection string (URL, file path, DSN,
//!   topic name, ...). The skeleton treats it as opaque; the
//!   concrete [`Connector`](crate::Connector) parses it.
//! - `credentials` — optional opaque blob. The skeleton does
//!   **not** log it.
//! - `poll_interval_ms` — how long the production poller sleeps
//!   between `fetch` calls. `0` means "fetch once, exit" (the
//!   skeleton honours this by returning a single batch).
//! - `batch_size` — upper bound on records per `fetch` call.
//!   Connectors may return fewer.
//!
//! See [`DOC-MOD-001`](../docs/modules/M-01-acquisition.md) §3.2
//! for the full schema.

use std::fmt;

use serde::{Deserialize, Serialize};

/// The five acquisition source kinds. Each variant maps to one
/// (or more) concrete [`Connector`](crate::Connector)
/// implementations. Adding a new kind requires extending this
/// enum **and** the connector module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SourceKind {
    /// HTTP / HTTPS endpoint (REST API, webhook pull, ...).
    /// Maps to [`HttpConnector`](crate::HttpConnector).
    Http,
    /// Local file (`std::fs::read`). Maps to
    /// [`FileConnector`](crate::FileConnector).
    File,
    /// Standard input. Maps to
    /// [`StdInConnector`](crate::StdInConnector).
    Stdin,
    /// Database (Postgres CDC, MySQL binlog, ...). No concrete
    /// connector in v0.1.0; the enum variant is reserved so the
    /// registry can describe database sources even before the
    /// production connector lands.
    Database,
    /// Message queue (Kafka, NATS, RabbitMQ, ...). No concrete
    /// connector in v0.1.0; same as `Database` — reserved for
    /// forward compatibility.
    MessageQueue,
}

impl SourceKind {
    /// Canonical lowercase string tag.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Http => "http",
            Self::File => "file",
            Self::Stdin => "stdin",
            Self::Database => "database",
            Self::MessageQueue => "message_queue",
        }
    }
}

impl fmt::Display for SourceKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Immutable description of a single acquisition source. The
/// production poller hands one of these to a
/// [`Connector`](crate::Connector) on every iteration.
///
/// The skeleton keeps `credentials` as `Option<String>` so the
/// caller can hold either an API token, a username/password pair
/// (JSON-encoded), or `None` for unauthenticated sources. The
/// connector is responsible for parsing it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceDescriptor {
    /// Stable, caller-chosen source id (e.g.
    /// `"ingest-csv-orders"`).
    pub id: String,
    /// What kind of source this is.
    pub kind: SourceKind,
    /// Connection string (URL, file path, DSN, topic name).
    /// The connector is responsible for parsing it.
    pub endpoint: String,
    /// Optional opaque credentials blob. The skeleton does
    /// **not** log this value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credentials: Option<String>,
    /// Milliseconds between `fetch` calls. `0` means "fetch
    /// once and exit" (the production poller will also honour
    /// this; the skeleton connectors always return one batch).
    #[serde(default)]
    pub poll_interval_ms: u64,
    /// Upper bound on records per `fetch` call. Connectors
    /// may return fewer.
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
}

fn default_batch_size() -> usize {
    64
}

impl SourceDescriptor {
    /// Build a new descriptor with the default `batch_size`
    /// (`64`) and `poll_interval_ms = 0`.
    #[must_use]
    pub fn new(id: impl Into<String>, kind: SourceKind, endpoint: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            kind,
            endpoint: endpoint.into(),
            credentials: None,
            poll_interval_ms: 0,
            batch_size: default_batch_size(),
        }
    }

    /// Builder-style setter for `credentials`.
    #[must_use]
    pub fn with_credentials(mut self, credentials: impl Into<String>) -> Self {
        self.credentials = Some(credentials.into());
        self
    }

    /// Builder-style setter for `poll_interval_ms`.
    #[must_use]
    pub const fn with_poll_interval_ms(mut self, poll_interval_ms: u64) -> Self {
        self.poll_interval_ms = poll_interval_ms;
        self
    }

    /// Builder-style setter for `batch_size`.
    #[must_use]
    pub const fn with_batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size = batch_size;
        self
    }

    /// Cheap in-process validation: rejects empty `id` and
    /// empty `endpoint`. The connector is responsible for
    /// validating the endpoint's structure.
    pub fn validate(&self) -> std::result::Result<(), String> {
        if self.id.trim().is_empty() {
            return Err("id is empty".to_string());
        }
        if self.endpoint.trim().is_empty() {
            return Err("endpoint is empty".to_string());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_as_str() {
        assert_eq!(SourceKind::Http.as_str(), "http");
        assert_eq!(SourceKind::File.as_str(), "file");
        assert_eq!(SourceKind::Stdin.as_str(), "stdin");
        assert_eq!(SourceKind::Database.as_str(), "database");
        assert_eq!(SourceKind::MessageQueue.as_str(), "message_queue");
    }

    #[test]
    fn kind_display() {
        assert_eq!(SourceKind::Http.to_string(), "http");
        assert_eq!(SourceKind::MessageQueue.to_string(), "message_queue");
    }

    #[test]
    fn new_descriptor_uses_defaults() {
        let d = SourceDescriptor::new("ingest-1", SourceKind::Http, "https://api.example.com");
        assert_eq!(d.id, "ingest-1");
        assert_eq!(d.kind, SourceKind::Http);
        assert_eq!(d.endpoint, "https://api.example.com");
        assert!(d.credentials.is_none());
        assert_eq!(d.poll_interval_ms, 0);
        assert_eq!(d.batch_size, 64);
    }

    #[test]
    fn builder_setters_propagate() {
        let d = SourceDescriptor::new("f", SourceKind::File, "/tmp/x.csv")
            .with_credentials("token:abc")
            .with_poll_interval_ms(250)
            .with_batch_size(8);
        assert_eq!(d.credentials.as_deref(), Some("token:abc"));
        assert_eq!(d.poll_interval_ms, 250);
        assert_eq!(d.batch_size, 8);
    }

    #[test]
    fn validate_rejects_empty_id() {
        let d = SourceDescriptor::new("   ", SourceKind::Http, "https://x");
        assert!(d.validate().is_err());
    }

    #[test]
    fn validate_rejects_empty_endpoint() {
        let d = SourceDescriptor::new("a", SourceKind::Http, "");
        assert!(d.validate().is_err());
    }

    #[test]
    fn validate_accepts_well_formed() {
        let d = SourceDescriptor::new("a", SourceKind::Http, "https://x");
        assert!(d.validate().is_ok());
    }

    #[test]
    fn serde_roundtrip() {
        let d = SourceDescriptor::new("a", SourceKind::File, "/tmp/x")
            .with_credentials("tok")
            .with_poll_interval_ms(100)
            .with_batch_size(16);
        let json = serde_json::to_string(&d).expect("serialize");
        let back: SourceDescriptor = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, d);
    }

    #[test]
    fn serde_skips_none_credentials() {
        let d = SourceDescriptor::new("a", SourceKind::Http, "https://x");
        let json = serde_json::to_string(&d).expect("serialize");
        assert!(!json.contains("credentials"));
    }
}
