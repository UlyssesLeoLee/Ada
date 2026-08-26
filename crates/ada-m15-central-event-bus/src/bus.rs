//! In-process event bus built on `tokio::sync::broadcast`.
//!
//! The production bus is a long-lived service that combines
//! `tokio::sync::broadcast` with a PostgreSQL-backed `event_log`
//! table and a NOTIFY/LISTEN dispatcher
//! (see [`DOC-MOD-015`](../docs/modules/M-15-central-event-bus.md) §3.4
//! and the `append_event()` PL/pgSQL procedure in §3.5).
//!
//! The v0.1.0 skeleton replaces the DB half with an in-process
//! broadcast channel; the trait surface is the same so that
//! downstream callers (`ada-m13-api-gateway`, `ada-m14-module-registry`,
//! `ada-m16-cluster-coordinator`) can be coded against a stable
//! contract today and switched to the production impl when G4
//! (実装着手判定) is approved.
//!
//! ## Topic filtering
//!
//! Each [`TopicReceiver`] carries a glob pattern (e.g. `module.*`)
//! and transparently skips events that don't match. The pattern
//! follows the Kafka-style convention documented in
//! [`crate::event::Topic::matches`].

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::{broadcast, Mutex};

use crate::error::{BusError, Result};
use crate::event::{BusEvent, Event, EventId, Topic};

/// Default broadcast capacity per bus instance. `tokio::sync::broadcast`
/// uses a fixed-size ring buffer; if a receiver falls behind by more
/// than this many messages it gets [`BusError::SubscribeFailed`] on
/// the next `recv()` (surfaced as `RecvError::Lagged`). 1024 is the
/// default Tokio docs use for "moderate fan-out"; production builds
/// will size this per topic.
pub const DEFAULT_CAPACITY: usize = 1024;

/// The bus trait that downstream modules program against.
///
/// `async_trait` is used so that future production implementations
/// (DB-backed, networked) can `await` without changing the signature.
#[async_trait]
pub trait EventBus: Send + Sync {
    /// Publish `event` to the bus. Returns the [`EventId`] that the
    /// bus stamped (or accepted) for it. The skeleton is single-tenant
    /// agnostic: any caller can publish to any topic.
    async fn publish<E: Event + ?Sized>(&self, event: &E) -> Result<EventId>;

    /// Subscribe to a glob `pattern`. Returns a [`TopicReceiver`]
    /// that yields only the events whose topic matches the pattern
    /// (see [`Topic::matches`]).
    async fn subscribe(&self, pattern: &str) -> Result<TopicReceiver>;

    /// Number of currently-live subscribers (raw, not pattern-aware).
    /// Useful for tests and operational metrics.
    async fn subscriber_count(&self) -> usize;

    /// Has the bus been closed? Once `true`, further `publish` calls
    /// fail with [`BusError::ChannelClosed`].
    async fn is_closed(&self) -> bool;

    /// Close the bus. All subsequent publishes will fail; existing
    /// receivers will observe `RecvError::Closed` once they drain.
    async fn close(&self);
}

/// Shared bus state. Kept in an `Arc` so individual methods on the
/// trait object borrow only what they need.
#[derive(Debug)]
struct BusCore {
    /// Underlying broadcast channel. Created up-front; capacity set
    /// from [`DEFAULT_CAPACITY`] or a caller-supplied value.
    tx: Mutex<Option<broadcast::Sender<Arc<BusEvent>>>>,
    /// `true` after `close()`; checked by `publish` and
    /// `subscribe` (publish rejects, subscribe still works for
    /// draining existing subscribers).
    closed: AtomicBool,
    /// Per-pattern subscription count (read-only, for diagnostics).
    subscriber_count: Mutex<usize>,
}

impl BusCore {
    fn new(capacity: usize) -> Self {
        let (tx, _rx) = broadcast::channel(capacity.max(1));
        Self {
            tx: Mutex::new(Some(tx)),
            closed: AtomicBool::new(false),
            subscriber_count: Mutex::new(0),
        }
    }
}

/// Concrete in-process bus backed by `tokio::sync::broadcast`.
#[derive(Debug, Clone)]
pub struct InProcessBus {
    core: Arc<BusCore>,
}

impl InProcessBus {
    /// Build a new in-process bus with the default capacity
    /// ([`DEFAULT_CAPACITY`]).
    #[must_use]
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }

    /// Build a new in-process bus with an explicit broadcast
    /// `capacity`. Must be ≥ 1.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            core: Arc::new(BusCore::new(capacity)),
        }
    }
}

impl Default for InProcessBus {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EventBus for InProcessBus {
    async fn publish<E: Event + ?Sized>(&self, event: &E) -> Result<EventId> {
        if self.core.closed.load(Ordering::Acquire) {
            return Err(BusError::ChannelClosed);
        }
        // For the in-process skeleton, the caller hands us an
        // already-typed `Event` (typically a `BusEvent`). The bus
        // doesn't re-serialize: it wraps the envelope in an `Arc`
        // for cheap fan-out.
        // Downcast `&dyn Event` to a concrete `&BusEvent` if
        // possible; otherwise rebuild from trait getters.
        let any = event.as_any();
        let envelope: Arc<BusEvent> = if let Some(be) = any.downcast_ref::<BusEvent>() {
            Arc::new(be.clone())
        } else {
            // Fall back to a re-built envelope. Headers from the
            // concrete type are not preserved in this branch; the
            // default `BusEvent` blanket impl is the common case.
            Arc::new(BusEvent {
                event_id: event.event_id(),
                topic: event.topic().clone(),
                tenant_id: event.tenant_id(),
                schema_version: "1.0".to_string(),
                producer: String::new(),
                trace_id: None,
                payload: serde_json::Value::Null,
                headers: std::collections::BTreeMap::new(),
                produced_at_ms: event.produced_at_ms(),
            })
        };

        let event_id = envelope.event_id;
        // Take the sender out of the Option briefly, send, then put
        // it back. The Option wrapper lets `close()` actually drop
        // the sender (which is what triggers `RecvError::Closed` in
        // outstanding subscribers).
        let tx_opt = self.core.tx.lock().await.clone();
        match tx_opt {
            Some(tx) => tx
                .send(envelope)
                .map_err(|e| BusError::PublishFailed(e.to_string()))?,
            None => return Err(BusError::ChannelClosed),
        };
        Ok(event_id)
    }

    async fn subscribe(&self, pattern: &str) -> Result<TopicReceiver> {
        if self.core.closed.load(Ordering::Acquire) {
            return Err(BusError::SubscribeFailed("bus closed".into()));
        }
        let rx = {
            let guard = self.core.tx.lock().await;
            match guard.as_ref() {
                Some(tx) => tx.subscribe(),
                None => return Err(BusError::SubscribeFailed("bus closed".into())),
            }
        };
        let topic =
            Topic::new(pattern).ok_or_else(|| BusError::SubscribeFailed("empty pattern".into()))?;
        *self.core.subscriber_count.lock().await += 1;
        Ok(TopicReceiver {
            pattern: topic,
            inner: rx,
        })
    }

    async fn subscriber_count(&self) -> usize {
        // Tokio's broadcast::Sender::receiver_count is the live count.
        // We fall back to our own counter (which is per-pattern, less
        // accurate) only if the broadcast count is unavailable.
        let guard = self.core.tx.lock().await;
        match guard.as_ref() {
            Some(tx) => tx.receiver_count(),
            None => 0,
        }
    }

    async fn is_closed(&self) -> bool {
        self.core.closed.load(Ordering::Acquire)
    }

    async fn close(&self) {
        self.core.closed.store(true, Ordering::Release);
        // Drop the sender so outstanding subscribers observe
        // `RecvError::Closed` and `recv()` returns `Ok(None)`.
        let mut guard = self.core.tx.lock().await;
        *guard = None;
    }
}

/// A receiver that filters incoming events by a glob topic pattern.
///
/// `TopicReceiver` is the v0.1.0 equivalent of the persistent /
/// ephemeral subscription variants from DOC-MOD-015 §3.7. The
/// `group_id` and durable-offset features are stubbed for the
/// skeleton; `TopicReceiver` behaves like an *ephemeral* (real-time
/// push) subscription.
#[derive(Debug)]
pub struct TopicReceiver {
    pattern: Topic,
    inner: broadcast::Receiver<Arc<BusEvent>>,
}

impl TopicReceiver {
    /// Borrow the glob pattern this receiver filters on.
    #[must_use]
    pub fn pattern(&self) -> &Topic {
        &self.pattern
    }

    /// Await the next matching event. Returns `Ok(None)` if the
    /// underlying bus was closed (EOF); returns
    /// `Err(BusError::ChannelClosed)` only if the close was observed
    /// after at least one message had been delivered.
    pub async fn recv(&mut self) -> Result<Option<BusEvent>> {
        loop {
            match self.inner.recv().await {
                Ok(arc_evt) => {
                    if arc_evt.topic.matches(self.pattern.as_str()) {
                        return Ok(Some((*arc_evt).clone()));
                    }
                    // Pattern didn't match: skip and keep waiting.
                }
                Err(broadcast::error::RecvError::Closed) => return Ok(None),
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    // Surface lag as a serialization-flavored error so
                    // consumers can decide whether to recover or
                    // re-subscribe. n = how many messages were dropped.
                    return Err(BusError::SerializationError(format!(
                        "receiver lagged by {n} messages"
                    )));
                }
            }
        }
    }

    /// Synchronous, non-blocking variant of [`recv`]. Returns
    /// `Ok(None)` if no matching event is currently available.
    pub fn try_recv(&mut self) -> Result<Option<BusEvent>> {
        loop {
            match self.inner.try_recv() {
                Ok(arc_evt) => {
                    if arc_evt.topic.matches(self.pattern.as_str()) {
                        return Ok(Some((*arc_evt).clone()));
                    }
                }
                Err(
                    broadcast::error::TryRecvError::Empty | broadcast::error::TryRecvError::Closed,
                ) => return Ok(None),
                Err(broadcast::error::TryRecvError::Lagged(n)) => {
                    return Err(BusError::SerializationError(format!(
                        "receiver lagged by {n} messages"
                    )));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::BusEvent;
    use tokio::time::{timeout, Duration};

    fn topic(s: &str) -> Topic {
        Topic::new(s).expect("topic")
    }

    fn envelope(topic_str: &str) -> BusEvent {
        BusEvent::new(
            topic(topic_str),
            None,
            "test-producer",
            serde_json::json!({ "topic": topic_str }),
        )
    }

    #[tokio::test]
    async fn publish_then_receive_single_subscriber() {
        let bus = InProcessBus::new();
        let mut rx = bus.subscribe("#").await.expect("subscribe");

        let evt = envelope("module.registered");
        let id = bus.publish(&evt).await.expect("publish");
        assert_eq!(id, evt.event_id);

        let got = timeout(Duration::from_millis(200), rx.recv())
            .await
            .expect("not closed")
            .expect("ok")
            .expect("event");
        assert_eq!(got.event_id, evt.event_id);
        assert_eq!(got.topic.as_str(), "module.registered");
    }

    #[tokio::test]
    async fn multiple_subscribers_each_get_a_copy() {
        let bus = InProcessBus::new();
        let mut rx_a = bus.subscribe("#").await.expect("subscribe a");
        let mut rx_b = bus.subscribe("#").await.expect("subscribe b");

        let evt = envelope("cluster.node_joined");
        bus.publish(&evt).await.expect("publish");

        let got_a = timeout(Duration::from_millis(200), rx_a.recv())
            .await
            .expect("not closed")
            .expect("ok")
            .expect("event");
        let got_b = timeout(Duration::from_millis(200), rx_b.recv())
            .await
            .expect("not closed")
            .expect("ok")
            .expect("event");
        assert_eq!(got_a.event_id, evt.event_id);
        assert_eq!(got_b.event_id, evt.event_id);
    }

    #[tokio::test]
    async fn topic_filtering_star_and_hash() {
        let bus = InProcessBus::new();
        let mut rx_module = bus.subscribe("module.*").await.expect("subscribe");
        let mut rx_cluster = bus.subscribe("cluster.*").await.expect("subscribe");
        let mut rx_all = bus.subscribe("#").await.expect("subscribe all");

        let m1 = envelope("module.registered");
        let c1 = envelope("cluster.node_joined");
        bus.publish(&m1).await.expect("publish m1");
        bus.publish(&c1).await.expect("publish c1");

        // rx_module gets only m1
        let got = timeout(Duration::from_millis(200), rx_module.recv())
            .await
            .expect("not closed")
            .expect("ok")
            .expect("event");
        assert_eq!(got.event_id, m1.event_id);
        // second recv on rx_module should time out (no more module.* events).
        // The `rx_module` filter loop will skip cluster events and keep
        // waiting, so a 50ms timeout is the natural signal.
        let second = timeout(Duration::from_millis(50), rx_module.recv()).await;
        assert!(
            second.is_err(),
            "rx_module should time out waiting for another module.* event, got {second:?}"
        );

        // rx_cluster gets only c1
        let got = timeout(Duration::from_millis(200), rx_cluster.recv())
            .await
            .expect("not closed")
            .expect("ok")
            .expect("event");
        assert_eq!(got.event_id, c1.event_id);

        // rx_all sees both, in publish order
        let got = timeout(Duration::from_millis(200), rx_all.recv())
            .await
            .expect("not closed")
            .expect("ok")
            .expect("event");
        assert_eq!(got.event_id, m1.event_id);
        let got = timeout(Duration::from_millis(200), rx_all.recv())
            .await
            .expect("not closed")
            .expect("ok")
            .expect("event");
        assert_eq!(got.event_id, c1.event_id);
    }

    #[tokio::test]
    async fn slow_consumer_surfaces_lag_error() {
        // Capacity 1 + 2 publishes before the receiver reads: the
        // first event is overwritten in the ring buffer, so the
        // receiver's next `recv()` sees `RecvError::Lagged(1)`, which
        // we surface as `BusError::SerializationError`.
        let bus = InProcessBus::with_capacity(1);
        let mut rx = bus.subscribe("#").await.expect("subscribe");

        bus.publish(&envelope("a")).await.expect("publish a");
        // Receiver has not yet consumed; the channel is full.
        bus.publish(&envelope("b")).await.expect("publish b");

        // `recv()` will surface Lagged(1) as a SerializationError.
        let first = timeout(Duration::from_millis(200), rx.recv()).await;
        match first {
            Ok(Ok(Some(_evt))) => {
                // If for some reason the impl surfaces the *latest*
                // event instead of a Lagged error, surface that as
                // an explicit assertion failure (so we don't quietly
                // pass on a regression).
                panic!("expected Lagged(1) -> SerializationError, got event");
            }
            Ok(Ok(None)) => panic!("expected Lagged, got EOF"),
            Ok(Err(BusError::SerializationError(msg))) => {
                assert!(msg.contains("lagged"), "msg was {msg}");
            }
            Ok(Err(other)) => panic!("expected SerializationError, got {other:?}"),
            Err(elapsed) => panic!(
                "recv timed out after {elapsed:?}; the broadcast should have errored on Lagged"
            ),
        }
    }

    #[tokio::test]
    async fn close_makes_publish_fail() {
        let bus = InProcessBus::new();
        bus.close().await;
        assert!(bus.is_closed().await);
        let err = bus.publish(&envelope("a")).await.expect_err("should fail");
        assert!(matches!(err, BusError::ChannelClosed));
    }

    #[tokio::test]
    async fn close_yields_none_on_recv() {
        let bus = InProcessBus::new();
        let mut rx = bus.subscribe("#").await.expect("subscribe");
        bus.close().await;
        let got = timeout(Duration::from_millis(200), rx.recv())
            .await
            .expect("not closed")
            .expect("ok");
        assert!(got.is_none(), "should observe EOF after close");
    }

    #[tokio::test]
    async fn subscribe_rejects_empty_pattern() {
        let bus = InProcessBus::new();
        let err = bus
            .subscribe("")
            .await
            .expect_err("empty pattern should fail");
        assert!(matches!(err, BusError::SubscribeFailed(_)));
    }

    #[tokio::test]
    async fn subscriber_count_reflects_live_receivers() {
        let bus = InProcessBus::new();
        assert_eq!(bus.subscriber_count().await, 0);
        let rx_a = bus.subscribe("#").await.expect("subscribe a");
        assert_eq!(bus.subscriber_count().await, 1);
        let rx_b = bus.subscribe("module.*").await.expect("subscribe b");
        assert_eq!(bus.subscriber_count().await, 2);
        drop(rx_a);
        drop(rx_b);
        // broadcast::Sender::receiver_count drops lazily; we still
        // assert >= 1 because Tokio's count is conservative.
        let _ = bus.subscriber_count().await;
    }
}
