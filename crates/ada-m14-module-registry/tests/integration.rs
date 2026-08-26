//! Integration tests for the v0.1.0 module registry.
//!
//! The v0.1.0 skeleton is in-process but exercises the public
//! surface that downstream services will see:
//!
//! - register / deregister / heartbeat lifecycle
//! - pluggable `EventBus` for state-change notifications
//! - `Unknown` health -> heartbeat -> `Healthy` transition
//!   emits a `module.health_changed` event on the bus

use std::sync::Arc;
use std::time::Duration;

use ada_m14_module_registry::{
    Capability, HealthState, ModuleDescriptor, ModuleKind, ModuleRegistry, RegistryError,
};
use ada_m15_central_event_bus::{EventBus, InProcessBus};

fn desc(name: &str) -> ModuleDescriptor {
    ModuleDescriptor::now(
        name,
        "1.0.0",
        ModuleKind::Ingest,
        vec![Capability::new("sql"), Capability::new("json")],
        "http://localhost:8080",
    )
}

#[tokio::test]
async fn end_to_end_register_heartbeat_deregister() {
    let bus = Arc::new(InProcessBus::new());
    let mut rx = bus.subscribe("module.#").await.expect("subscribe");
    let registry = ModuleRegistry::with_bus(bus.clone());

    // 1. Register
    let stored = registry.register(desc("mod-ingest-csv")).await.unwrap();
    assert_eq!(stored.name, "mod-ingest-csv");
    assert_eq!(stored.health, HealthState::Unknown);
    let first = tokio::time::timeout(Duration::from_millis(200), rx.recv())
        .await
        .expect("not closed")
        .expect("ok")
        .expect("event");
    assert_eq!(first.topic.as_str(), "module.registered");

    // 2. Heartbeat Healthy
    let (updated, t) = registry
        .heartbeat("mod-ingest-csv", HealthState::Healthy)
        .await
        .unwrap();
    assert_eq!(updated.health, HealthState::Healthy);
    let t = t.expect("transition");
    assert_eq!(t.from, HealthState::Unknown);
    let second = tokio::time::timeout(Duration::from_millis(200), rx.recv())
        .await
        .expect("not closed")
        .expect("ok")
        .expect("event");
    assert_eq!(second.topic.as_str(), "module.health_changed");

    // 3. Deregister
    let removed = registry.deregister("mod-ingest-csv").await.unwrap();
    assert_eq!(removed.name, "mod-ingest-csv");
    let third = tokio::time::timeout(Duration::from_millis(200), rx.recv())
        .await
        .expect("not closed")
        .expect("ok")
        .expect("event");
    assert_eq!(third.topic.as_str(), "module.deregistered");
    assert!(registry.get("mod-ingest-csv").is_none());
}

#[tokio::test]
async fn duplicate_register_is_idempotent_failure() {
    let registry = ModuleRegistry::new();
    registry.register(desc("a")).await.unwrap();
    let err = registry.register(desc("a")).await.expect_err("dup");
    assert!(matches!(err, RegistryError::AlreadyRegistered(_)));
}

#[tokio::test]
async fn invalid_descriptor_is_rejected_before_storage() {
    let registry = ModuleRegistry::new();
    let bad = ModuleDescriptor::now("", "1.0.0", ModuleKind::Ingest, vec![], "http://x");
    let err = registry.register(bad).await.expect_err("invalid");
    assert!(matches!(err, RegistryError::InvalidDescriptor(_)));
    assert_eq!(registry.len(), 0);
}

#[tokio::test]
async fn state_changes_publish_to_a_pluggable_bus() {
    // Two registries, one bus, prove the bus parameter is what
    // carries the events. We register on `r1`, observe on the
    // shared bus, then deregister and observe again.
    let bus = Arc::new(InProcessBus::new());
    let mut rx = bus.subscribe("module.#").await.expect("subscribe");
    let r1 = ModuleRegistry::with_bus(bus.clone());
    let r2 = ModuleRegistry::with_bus(bus.clone());
    r1.register(desc("a")).await.unwrap();
    r2.register(desc("b")).await.unwrap();
    let first = tokio::time::timeout(Duration::from_millis(200), rx.recv())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let second = tokio::time::timeout(Duration::from_millis(200), rx.recv())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let mut names: Vec<String> = vec![
        first.payload["module_name"].as_str().unwrap().to_string(),
        second.payload["module_name"].as_str().unwrap().to_string(),
    ];
    names.sort();
    assert_eq!(names, vec!["a".to_string(), "b".to_string()]);

    // Filtering works the same way: subscribe to "module.deregistered"
    // and confirm the deregister event is delivered, while
    // "module.registered" does NOT match the deregister event.
    let mut rx_dereg = bus.subscribe("module.deregistered").await.unwrap();
    r1.deregister("a").await.unwrap();
    let evt = tokio::time::timeout(Duration::from_millis(200), rx_dereg.recv())
        .await
        .expect("deregistered event expected")
        .expect("ok")
        .expect("event");
    assert_eq!(evt.topic.as_str(), "module.deregistered");
    assert_eq!(evt.payload["module_name"].as_str(), Some("a"));
    // No further events on this filter.
    let none = tokio::time::timeout(Duration::from_millis(50), rx_dereg.recv()).await;
    assert!(
        none.is_err(),
        "no further deregistered events expected, got {none:?}"
    );
}

#[tokio::test]
async fn heartbeat_with_no_state_change_does_not_emit_event() {
    let bus = Arc::new(InProcessBus::new());
    let mut rx = bus.subscribe("module.#").await.expect("subscribe");
    let registry = ModuleRegistry::with_bus(bus);
    registry.register(desc("a")).await.unwrap();
    // Drain the registration event.
    let _ = tokio::time::timeout(Duration::from_millis(200), rx.recv())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    // First heartbeat: Unknown -> Healthy IS a change, so it
    // emits and we drain it.
    registry.heartbeat("a", HealthState::Healthy).await.unwrap();
    let evt = tokio::time::timeout(Duration::from_millis(200), rx.recv())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(evt.topic.as_str(), "module.health_changed");
    // Second heartbeat with the same state: NO event.
    registry.heartbeat("a", HealthState::Healthy).await.unwrap();
    let none = tokio::time::timeout(Duration::from_millis(50), rx.recv()).await;
    assert!(
        none.is_err(),
        "no-op heartbeat should not emit, got {none:?}"
    );
}

#[tokio::test]
async fn wildcard_topic_receives_all_three_kinds() {
    let bus = Arc::new(InProcessBus::new());
    let mut rx = bus.subscribe("#").await.expect("subscribe");
    let registry = ModuleRegistry::with_bus(bus);
    registry.register(desc("a")).await.unwrap();
    registry.heartbeat("a", HealthState::Healthy).await.unwrap();
    registry.deregister("a").await.unwrap();
    let mut topics: Vec<String> = Vec::with_capacity(3);
    for _ in 0..3 {
        let evt = tokio::time::timeout(Duration::from_millis(200), rx.recv())
            .await
            .expect("not closed")
            .expect("ok")
            .expect("event");
        topics.push(evt.topic.as_str().to_string());
    }
    assert!(topics.contains(&"module.registered".to_string()));
    assert!(topics.contains(&"module.health_changed".to_string()));
    assert!(topics.contains(&"module.deregistered".to_string()));
}

#[tokio::test]
async fn topic_constant_is_module_prefix() {
    // Defensive: any drift in the topic naming breaks the M-15
    // filters. Pin it here.
    let bus = Arc::new(InProcessBus::new());
    let mut rx = bus.subscribe("module.*").await.expect("subscribe");
    let registry = ModuleRegistry::with_bus(bus);
    registry.register(desc("a")).await.unwrap();
    let evt = tokio::time::timeout(Duration::from_millis(200), rx.recv())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(evt.topic.as_str().starts_with("module."));
}
