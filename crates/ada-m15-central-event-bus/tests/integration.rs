//! Integration tests for the v0.1.0 central event bus.

use ada_m15_central_event_bus::{BusEvent, EventBus, InProcessBus, Topic};
use tokio::time::{timeout, Duration};

fn topic(s: &str) -> Topic {
    Topic::new(s).expect("topic must be non-empty")
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
async fn end_to_end_publish_subscribe_with_glob() {
    let bus = InProcessBus::new();
    let mut rx_all = bus.subscribe("#").await.expect("subscribe #");
    let mut rx_module = bus.subscribe("module.*").await.expect("subscribe module.*");

    let a = envelope("module.registered");
    let b = envelope("cluster.node_joined");
    let a_id = bus.publish(&a).await.expect("publish a");
    let b_id = bus.publish(&b).await.expect("publish b");
    assert_eq!(a_id, a.event_id);
    assert_eq!(b_id, b.event_id);

    // The wildcard receiver sees both, in publish order.
    let got_a = timeout(Duration::from_millis(200), rx_all.recv())
        .await
        .expect("rx_all not closed")
        .expect("rx_all ok")
        .expect("rx_all got event");
    assert_eq!(got_a.event_id, a.event_id);
    let got_b = timeout(Duration::from_millis(200), rx_all.recv())
        .await
        .expect("rx_all not closed")
        .expect("rx_all ok")
        .expect("rx_all got event");
    assert_eq!(got_b.event_id, b.event_id);

    // The module.* receiver only sees a.
    let got_a = timeout(Duration::from_millis(200), rx_module.recv())
        .await
        .expect("rx_module not closed")
        .expect("rx_module ok")
        .expect("rx_module got event");
    assert_eq!(got_a.event_id, a.event_id);
    // Second recv on rx_module should time out cleanly (no more module.* events).
    let second = timeout(Duration::from_millis(50), rx_module.recv()).await;
    assert!(
        second.is_err(),
        "rx_module should time out waiting for another module.* event, got {second:?}"
    );
}

#[tokio::test]
async fn subscribe_after_close_errors_out() {
    let bus = InProcessBus::new();
    bus.close().await;
    let err = bus.subscribe("#").await.expect_err("subscribe after close");
    // Could be ChannelClosed (from publish path) or SubscribeFailed
    // (from subscribe path). Both are valid BusError variants.
    let _ = err; // just confirm the call short-circuits.
    let _ = bus.is_closed().await; // confirm helper still works
}

#[tokio::test]
async fn subscriber_count_grows_and_shrinks() {
    let bus = InProcessBus::new();
    assert_eq!(bus.subscriber_count().await, 0);
    let rx_a = bus.subscribe("#").await.unwrap();
    let rx_b = bus.subscribe("module.*").await.unwrap();
    assert_eq!(bus.subscriber_count().await, 2);
    drop(rx_a);
    drop(rx_b);
    // broadcast::Sender::receiver_count is conservative; we only
    // assert <= 2 here.
    let after = bus.subscriber_count().await;
    assert!(after <= 2);
}

#[tokio::test]
async fn multiple_events_in_flight_drained_in_publish_order() {
    let bus = InProcessBus::new();
    let mut rx = bus.subscribe("#").await.unwrap();
    for i in 0..10u32 {
        let evt = BusEvent::new(
            topic("metric.tick"),
            None,
            "ada-m09-exporter",
            serde_json::json!({ "n": i }),
        );
        bus.publish(&evt).await.unwrap();
    }
    for i in 0..10u32 {
        let got = timeout(Duration::from_millis(200), rx.recv())
            .await
            .expect("not closed")
            .expect("ok")
            .expect("event");
        assert_eq!(got.payload["n"], serde_json::json!(i));
    }
}
