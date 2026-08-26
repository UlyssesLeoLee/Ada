//! Error surface for the central event bus.
//!
//! [`BusError`] is the single error type returned by every public
//! function in this crate. The v0.1.0 skeleton keeps the enum at
//! five variants covering the common failure modes of an in-process
//! Pub/Sub built on `tokio::sync::broadcast`:
//!
//! | Variant            | Trigger                                                 |
//! |--------------------|---------------------------------------------------------|
//! | `PublishFailed`    | The internal `broadcast::Sender` rejected the send.     |
//! | `SubscribeFailed`  | `subscribe` could not allocate a new receiver.          |
//! | `ChannelClosed`    | The bus has been closed (no more sends possible).       |
//! | `NoSubscribers`    | `publish` was called with zero live receivers.          |
//! | `SerializationError` | `payload` could not be (de)serialized via `serde_json`. |
//!
//! Production builds will map these to richer diagnostics
//! (correlation ids, retry advisories); the skeleton keeps the
//! surface minimal. See
//! [`DOC-MOD-015`](../docs/modules/M-15-central-event-bus.md) §3.4
//! for the full publish pipeline.

use thiserror::Error;

/// Failure modes surfaced by the central event bus.
#[derive(Debug, Error)]
pub enum BusError {
    /// The internal broadcast channel rejected a publish (channel
    /// closed, capacity zero, or a rare internal failure).
    #[error("publish failed: {0}")]
    PublishFailed(String),

    /// `subscribe` could not allocate a new receiver. Typically only
    /// triggered if the underlying broadcast channel is exhausted.
    #[error("subscribe failed: {0}")]
    SubscribeFailed(String),

    /// The bus has been closed; no further publishes are accepted.
    #[error("bus channel closed")]
    ChannelClosed,

    /// A publish was attempted but no live subscribers exist for
    /// the given topic. Configurable per `InProcessBus`; the
    /// skeleton defaults to *allowing* such publishes.
    #[error("no subscribers for topic: {0}")]
    NoSubscribers(String),

    /// Payload (de)serialization failed.
    #[error("serialization error: {0}")]
    SerializationError(String),
}

/// `Result` alias for fallible bus operations.
pub type Result<T> = core::result::Result<T, BusError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publish_failed_display() {
        let e = BusError::PublishFailed("inner channel dead".into());
        assert_eq!(e.to_string(), "publish failed: inner channel dead");
    }

    #[test]
    fn subscribe_failed_display() {
        let e = BusError::SubscribeFailed("queue full".into());
        assert_eq!(e.to_string(), "subscribe failed: queue full");
    }

    #[test]
    fn channel_closed_display() {
        let e = BusError::ChannelClosed;
        assert_eq!(e.to_string(), "bus channel closed");
    }

    #[test]
    fn no_subscribers_display() {
        let e = BusError::NoSubscribers("module.registered".into());
        assert_eq!(e.to_string(), "no subscribers for topic: module.registered");
    }

    #[test]
    fn serialization_error_display() {
        let e = BusError::SerializationError("unexpected EOF".into());
        assert_eq!(e.to_string(), "serialization error: unexpected EOF");
    }

    #[test]
    fn result_alias_carries_error() {
        let ok: Result<i32> = Ok(7);
        let err: Result<i32> = Err(BusError::ChannelClosed);
        assert!(matches!(ok, Ok(7)));
        assert!(matches!(err, Err(BusError::ChannelClosed)));
    }

    #[test]
    fn error_is_send_sync_and_static() {
        fn assert_send_sync_static<E: std::error::Error + Send + Sync + 'static>(_: &E) {}
        let e = BusError::PublishFailed("x".into());
        assert_send_sync_static(&e);
    }
}
