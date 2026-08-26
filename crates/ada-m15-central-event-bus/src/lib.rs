//! M-15: Central event bus. Pub/Sub. at-least-once + idempotent (D-07).
//!
//! ## v0.1.0 scope (B3)
//!
//! This crate is a **minimum skeleton** for the cross-module event
//! bus. It implements the trait surface and the in-process
//! `tokio::sync::broadcast`-backed adapter that downstream modules
//! (`ada-m13-api-gateway`, `ada-m14-module-registry`,
//! `ada-m16-cluster-coordinator`) will program against. The
//! production deployment (PostgreSQL `event_log` + NOTIFY/LISTEN +
//! Redis durable queue, see [`DOC-MOD-015`](../docs/modules/M-15-central-event-bus.md)
//! §3.4 and the `append_event()` PL/pgSQL procedure in §3.5) is
//! scheduled for B4+.
//!
//! ### What v0.1.0 provides
//!
//! - [`Event`] trait + [`BusEvent`] canonical envelope (event_id,
//!   topic, tenant_id, producer, trace_id, payload, headers,
//!   produced_at_ms) — see `DOC-MOD-015` §3.3
//! - [`Topic`] newtype with Kafka-style glob match
//!   (`*` one segment, `#` zero+) — see `DOC-MOD-015` §3.1
//! - [`EventBus`] trait with `publish` / `subscribe` /
//!   `subscriber_count` / `is_closed` / `close`
//! - [`InProcessBus`] — broadcast-channel-backed in-process impl
//! - [`TopicReceiver`] — pattern-filtered receiver; surfaces
//!   `tokio::sync::broadcast::RecvError::Lagged` as
//!   [`BusError::SerializationError`]
//! - [`BusError`] — five variants (PublishFailed, SubscribeFailed,
//!   ChannelClosed, NoSubscribers, SerializationError)
//! - 9 unit tests + 4 integration tests (`tests/integration.rs`)
//!
//! ### What v0.1.0 explicitly does **not** do
//!
//! - Persist events to the `event_log` table
//! - Honor durable `consumer_offset` / replay
//! - Distribute events across cluster nodes
//! - Honor at-least-once with explicit ACK semantics
//!   (the broadcast channel is best-effort within a single process)
//! - Support `serde_json::Value` payload conversion into typed NJSON
//!
//! These are scheduled for B4+ (after the G4 実装着手判定 milestone).
//!
//! 関連 IPA フェーズ: 22-52 (基本設計/詳細設計), 53-58 (実装), 59-95 (試験)
//! 設計書: docs/modules/M-15-central-event-bus.md (DOC-MOD-015)
//! ワークフロー: docs/architecture/08-workflow-overview.md

#![allow(missing_docs)]
#![allow(rust_2018_idioms)]
#![allow(clippy::doc_markdown)]

mod bus;
mod error;
mod event;

pub use bus::{EventBus, InProcessBus, TopicReceiver, DEFAULT_CAPACITY};
pub use error::{BusError, Result};
pub use event::{BusEvent, Event, EventId, Topic};

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
