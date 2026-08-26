# ada-m15-central-event-bus

M-15: 中央イベントバス (Central event bus).
Pub/Sub. at-least-once + idempotent (D-07).

## v0.1.0 scope (B3 batch)

This crate is a **minimum skeleton** for the cross-module event
bus. It implements the trait surface and the in-process
`tokio::sync::broadcast`-backed adapter that downstream modules
(`ada-m13-api-gateway`, `ada-m14-module-registry`,
`ada-m16-cluster-coordinator`) will program against.

The production deployment (PostgreSQL `event_log` + NOTIFY/LISTEN
+ Redis durable queue, see `DOC-MOD-015` §3.4 and the
`append_event()` PL/pgSQL procedure in §3.5) is scheduled for
B4+.

### What v0.1.0 provides

- `Event` trait + `BusEvent` canonical envelope (event_id, topic,
  tenant_id, producer, trace_id, payload, headers, produced_at_ms)
- `Topic` newtype with Kafka-style glob match (`*` one segment,
  `#` zero+)
- `EventBus` trait with `publish` / `subscribe` /
  `subscriber_count` / `is_closed` / `close`
- `InProcessBus` — broadcast-channel-backed in-process impl
- `TopicReceiver` — pattern-filtered receiver; surfaces
  `tokio::sync::broadcast::RecvError::Lagged` as
  `BusError::SerializationError`
- `BusError` — five variants (PublishFailed, SubscribeFailed,
  ChannelClosed, NoSubscribers, SerializationError)
- 22 unit tests + 4 integration tests

### What v0.1.0 explicitly does **not** do

- Persist events to the `event_log` table
- Honor durable `consumer_offset` / replay
- Distribute events across cluster nodes
- Honor at-least-once with explicit ACK semantics (the broadcast
  channel is best-effort within a single process)

## 関連 IPA フェーズ

22-52 (基本設計/詳細設計), 53-58 (実装), 59-95 (試験)

## 設計書

`docs/modules/M-15-central-event-bus.md` (DOC-MOD-015)
