//! End-to-end integration tests for the M-08 trigger crate.

use ada_m08_trigger::{Action, TriggerKind, TriggerManager, TriggerRule};
use serde_json::json;

#[test]
fn cron_lifecycle_and_match() {
    let m = TriggerManager::new();
    let r = TriggerRule::new(
        "every-5-min",
        TriggerKind::Cron,
        "*/5 * * * *",
        Action::new("run_export", json!({"id": "exp-1"})),
    )
    .expect("ok");
    let id = m.add(r).expect("add");
    assert!(m.get(id).is_some());
    m.set_enabled(id, false).expect("disable");
    assert!(!m.get(id).expect("rule").enabled);
    m.remove(id).expect("remove");
    assert!(m.get(id).is_none());
}

#[test]
fn event_trigger_matches_glob_topics() {
    let m = TriggerManager::new();
    m.add(
        TriggerRule::new(
            "module-registered",
            TriggerKind::Event,
            "module.*",
            Action::new("on_register", json!({})),
        )
        .expect("ok"),
    )
    .expect("add");
    let hits = m.match_event("module.registered");
    assert_eq!(hits.len(), 1);
    let hits = m.match_event("module.removed");
    assert_eq!(hits.len(), 1);
    let hits = m.match_event("module.ada-m14.registered");
    assert!(hits.is_empty());
}

#[test]
fn webhook_trigger_stores_path() {
    let m = TriggerManager::new();
    let r = TriggerRule::new(
        "github-push",
        TriggerKind::Webhook,
        "/hooks/github/push",
        Action::new("rebuild", json!({"repo": "ada"})),
    )
    .expect("ok");
    let id = m.add(r).expect("add");
    let got = m.get(id).expect("rule");
    assert_eq!(got.schedule, "/hooks/github/push");
    assert_eq!(got.kind, TriggerKind::Webhook);
}

#[test]
fn invalid_cron_is_rejected() {
    let err =
        TriggerRule::new("bad", TriggerKind::Cron, "* *", Action::new("x", json!({}))).unwrap_err();
    let s = err.to_string();
    assert!(s.contains("5 fields"), "got: {s}");
}
