//! Event envelope, [`Event`] trait, and [`Topic`] newtype.
//!
//! This is the data shape that flows through the central event bus.
//! See [`DOC-MOD-015`](../docs/modules/M-15-central-event-bus.md) §3.2
//! and §3.3 for the canonical schema (event_id, topic, tenant_id,
//! payload, headers, trace_id, produced_at).
//!
//! The v0.1.0 skeleton keeps the payload type as opaque `serde_json::Value`
//! so that the bus does not need to know the concrete NJSON / module
//! type that any given producer is sending. Real builds will likely
//! trade some of that flexibility for a typed payload.

use std::collections::BTreeMap;
use std::fmt;

use ada_core::TenantId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Event topic — a string-shaped identifier matching the
/// `<category>.<entity>.<action>` convention from
/// [`DOC-MOD-015`](../docs/modules/M-15-central-event-bus.md) §3.1
/// (e.g. `module.registered`, `cluster.node_joined`).
///
/// We keep the topic as a `String` newtype so we can:
/// - reject empty topics at the type boundary (`Topic::new`),
/// - attach helpers (`as_str`, `matches` for the `*`/`#` glob syntax)
///   in a single place, and
/// - log a stable `Display` form across the bus.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Topic(String);

impl Topic {
    /// Build a new topic. Returns `None` if `s` is empty or contains
    /// only ASCII whitespace, so that producers can't accidentally
    /// send to an unfilterable "blank" channel.
    #[must_use]
    pub fn new(s: impl Into<String>) -> Option<Self> {
        let s = s.into();
        if s.trim().is_empty() {
            None
        } else {
            Some(Self(s))
        }
    }

    /// Borrow the underlying topic string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Glob-style match against `pattern`. Supports two wildcards,
    /// matching the DOC-MOD-015 §3.1 Kafka-style convention:
    ///
    /// - `*`  matches exactly **one** dot-separated segment
    /// - `#`  matches **zero or more** dot-separated segments
    ///
    /// The pattern must be a plain topic name; it is **not** itself
    /// subject to the topic-validity check.
    ///
    /// # Examples
    ///
    /// ```
    /// use ada_m15_central_event_bus::Topic;
    ///
    /// let t = Topic::new("module.registered").unwrap();
    /// assert!(t.matches("module.*"));
    /// assert!(t.matches("module.#"));
    /// assert!(t.matches("#"));
    /// assert!(!t.matches("cluster.*"));
    /// ```
    #[must_use]
    pub fn matches(&self, pattern: &str) -> bool {
        glob_match(pattern, &self.0)
    }
}

impl fmt::Display for Topic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for Topic {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<Topic> for String {
    fn from(t: Topic) -> Self {
        t.0
    }
}

/// Kafka-style glob match: `*` = one segment, `#` = zero+ segments.
///
/// Both `pattern` and `topic` are dot-separated. Segments are
/// compared verbatim (no regex, no escapes). This is intentionally
/// tiny — the full Kafka grammar is out of scope for v0.1.0.
fn glob_match(pattern: &str, topic: &str) -> bool {
    let pat_segs: Vec<&str> = pattern.split('.').collect();
    let evt_segs: Vec<&str> = topic.split('.').collect();
    match_segments(&pat_segs, &evt_segs)
}

fn match_segments(pat: &[&str], evt: &[&str]) -> bool {
    // Walk both slices together. `#` consumes any number of
    // remaining event segments (including zero).
    let mut i = 0;
    let mut j = 0;
    while i < pat.len() {
        match pat[i] {
            "#" => {
                // `#` is always the last segment per the Kafka convention.
                // If there are more pattern segments after `#` it is
                // malformed; we treat it as "matches everything after this
                // point" so callers don't silently miss events.
                return true;
            }
            "*" => {
                if j >= evt.len() {
                    return false;
                }
                i += 1;
                j += 1;
            }
            literal => {
                if j >= evt.len() || evt[j] != literal {
                    return false;
                }
                i += 1;
                j += 1;
            }
        }
    }
    j == evt.len()
}

/// Identifier for a published event (UUID v4 in v0.1.0; production
/// upgrades to UUID v7 for time-ordered ids per DOC-MOD-015 §3.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EventId(pub Uuid);

impl EventId {
    /// Generate a fresh `EventId` (UUID v4).
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for EventId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for EventId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "event({})", self.0)
    }
}

/// The abstract event interface. Any type that can be turned into a
/// [`BusEvent`] and back implements this trait.
///
/// The default blanket impl turns any [`BusEvent`] into itself so
/// that `EventBus::publish` can be called with either a concrete
/// envelope or a producer-side wrapper type.
pub trait Event: Send + Sync + 'static {
    /// Stable, sortable event identifier.
    fn event_id(&self) -> EventId;
    /// Topic this event is published to.
    fn topic(&self) -> &Topic;
    /// Tenant scope; `None` means "system / un-tenanted".
    fn tenant_id(&self) -> Option<TenantId>;
    /// Producer-supplied wall-clock timestamp in milliseconds since
    /// the UNIX epoch.
    fn produced_at_ms(&self) -> u64;
    /// Downcast to a `&dyn Any` for down-stream consumers that need
    /// to recover the concrete type. Returns `None` for the default
    /// `BusEvent` blanket.
    fn as_any(&self) -> &dyn std::any::Any;
}

/// The canonical event envelope carried over the bus.
///
/// See [`DOC-MOD-015`](../docs/modules/M-15-central-event-bus.md) §3.3
/// for the JSON shape and §3.5 for the PL/pgSQL `append_event`
/// stored procedure that the production build persists this as.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusEvent {
    /// Stable event identifier.
    pub event_id: EventId,
    /// Topic the event was published to.
    pub topic: Topic,
    /// Tenant scope (`None` for system events).
    pub tenant_id: Option<TenantId>,
    /// Schema version in `headers["schema_version"]` is exposed here
    /// for convenience.
    pub schema_version: String,
    /// Producer service / module name.
    pub producer: String,
    /// Optional distributed-trace correlation id.
    pub trace_id: Option<String>,
    /// Opaque, type-erased JSON payload.
    pub payload: serde_json::Value,
    /// Arbitrary, alphabetically-ordered header bag.
    pub headers: BTreeMap<String, String>,
    /// Wall-clock time the event was produced, in milliseconds since
    /// the UNIX epoch.
    pub produced_at_ms: u64,
}

impl BusEvent {
    /// Convenience constructor: assigns a fresh [`EventId`], stamps
    /// `produced_at_ms` from the system clock, and sets
    /// `schema_version = "1.0"`.
    ///
    /// # Examples
    ///
    /// ```
    /// use ada_m15_central_event_bus::{BusEvent, Topic};
    ///
    /// let evt = BusEvent::new(
    ///     Topic::new("module.registered").unwrap(),
    ///     None,
    ///     "ada-m14-module-registry",
    ///     serde_json::json!({ "module_id": "mod-1" }),
    /// );
    /// assert_eq!(evt.schema_version, "1.0");
    /// assert_eq!(evt.producer, "ada-m14-module-registry");
    /// ```
    #[must_use]
    pub fn new(
        topic: Topic,
        tenant_id: Option<TenantId>,
        producer: impl Into<String>,
        payload: serde_json::Value,
    ) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let produced_at_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX));
        Self {
            event_id: EventId::new(),
            topic,
            tenant_id,
            schema_version: "1.0".to_string(),
            producer: producer.into(),
            trace_id: None,
            payload,
            headers: BTreeMap::new(),
            produced_at_ms,
        }
    }

    /// Builder-style: stamp a `trace_id` (returns `self` by value).
    #[must_use]
    pub fn with_trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = Some(trace_id.into());
        self
    }

    /// Builder-style: set a header (returns `self` by value).
    #[must_use]
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }
}

impl Event for BusEvent {
    fn event_id(&self) -> EventId {
        self.event_id
    }
    fn topic(&self) -> &Topic {
        &self.topic
    }
    fn tenant_id(&self) -> Option<TenantId> {
        self.tenant_id
    }
    fn produced_at_ms(&self) -> u64 {
        self.produced_at_ms
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn topic(s: &str) -> Topic {
        Topic::new(s).expect("topic must be non-empty")
    }

    #[test]
    fn topic_new_rejects_blank() {
        assert!(Topic::new("").is_none());
        assert!(Topic::new("   ").is_none());
        assert!(Topic::new("a").is_some());
    }

    #[test]
    fn topic_display_and_as_ref() {
        let t = topic("module.registered");
        assert_eq!(t.to_string(), "module.registered");
        assert_eq!(t.as_str(), "module.registered");
        assert_eq!(AsRef::<str>::as_ref(&t), "module.registered");
        let s: String = t.clone().into();
        assert_eq!(s, "module.registered");
    }

    #[test]
    fn topic_matches_star_matches_one_segment() {
        let t = topic("module.registered");
        assert!(t.matches("module.*"));
        assert!(t.matches("*.registered"));
        assert!(!t.matches("module.*.foo"));
        assert!(!t.matches("cluster.*"));
    }

    #[test]
    fn topic_matches_hash_matches_zero_or_more_segments() {
        let t = topic("module.registered");
        assert!(t.matches("#"));
        assert!(t.matches("module.#"));
        assert!(t.matches("module.registered.#"));
        let t2 = topic("a");
        assert!(t2.matches("#"));
    }

    #[test]
    fn topic_matches_no_wildcard_is_equality() {
        let t = topic("cluster.node_joined");
        assert!(t.matches("cluster.node_joined"));
        assert!(!t.matches("cluster.node_left"));
    }

    #[test]
    fn event_id_is_unique_per_new() {
        let a = EventId::new();
        let b = EventId::new();
        assert_ne!(a, b);
        assert_eq!(EventId::default().0.get_version_num(), 4);
    }

    #[test]
    fn bus_event_new_stamps_metadata() {
        let evt = BusEvent::new(
            topic("module.registered"),
            None,
            "ada-m14-module-registry",
            serde_json::json!({ "module_id": "mod-1" }),
        );
        assert_eq!(evt.schema_version, "1.0");
        assert_eq!(evt.producer, "ada-m14-module-registry");
        assert_eq!(evt.topic.as_str(), "module.registered");
        assert!(evt.produced_at_ms > 0);
        // Blanket impl
        assert_eq!(Event::event_id(&evt), evt.event_id);
    }

    #[test]
    fn bus_event_builder_headers_and_trace() {
        let evt = BusEvent::new(topic("a.b.c"), None, "p", serde_json::json!({}))
            .with_trace_id("trace-42")
            .with_header("schema_version", "1.0")
            .with_header("correlation_id", "c-1");
        assert_eq!(evt.trace_id.as_deref(), Some("trace-42"));
        assert_eq!(
            evt.headers.get("schema_version").map(String::as_str),
            Some("1.0")
        );
        assert_eq!(
            evt.headers.get("correlation_id").map(String::as_str),
            Some("c-1")
        );
    }

    #[test]
    fn bus_event_serde_roundtrip() {
        let evt = BusEvent::new(
            topic("a.b"),
            None,
            "producer-x",
            serde_json::json!({ "k": 1 }),
        );
        let json = serde_json::to_string(&evt).expect("serialize");
        let back: BusEvent = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.event_id, evt.event_id);
        assert_eq!(back.topic, evt.topic);
        assert_eq!(back.payload, evt.payload);
    }
}
