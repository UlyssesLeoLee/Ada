//! End-to-end integration test for the auto-remediation engine.
//!
//! Wires the engine, the in-memory history store, and the
//! HTTP server together without a real Postgres instance. The
//! persistent half (PostgreSQL `remediation_history` +
//! `remediation_cooldowns`) is covered by
//! `db/tests/V003__phase8_remediation_test.sql`.
//!
//! The full E2E flow is:
//!   1. Load runbooks from `config/remediation/` (5 default
//!      files committed alongside this crate).
//!   2. Build an `AlertmanagerPayload` for `DiskSpaceFillingFast`.
//!   3. POST it to the in-process axum router.
//!   4. Assert the response shape, the cooldown state, the
//!      `/remediation/history` payload.
//!   5. Re-send the *same* alert to assert cooldown skip.
//!   6. Send a different severity to assert severity filter.
//!   7. `POST /remediation/trigger` for `force=true` to
//!      bypass cooldown.

use ada_remediation::http::{
    router, AlertmanagerAlert, AlertmanagerPayload, AppState, WebhookResponse,
};
use ada_remediation::MemoryStore;
use ada_remediation::{
    load_runbooks_from_dir, AlertEvent, ExecutorMode, RemediationAction, RemediationEngine,
};
use axum::body::Body;
use axum::http::Request;
use std::sync::Arc;
use tower::ServiceExt;

/// Locate the workspace's `config/remediation/` directory
/// regardless of where `cargo test` is invoked from.
fn runbooks_dir() -> std::path::PathBuf {
    // `cargo test` runs with CWD = the crate dir for
    // [[bin]]/integration tests, so the relative path
    // `../config/remediation` reaches the workspace root.
    let local = std::path::Path::new("../config/remediation");
    if local.exists() {
        return local.canonicalize().unwrap_or_else(|_| local.to_path_buf());
    }
    // Fallback: CARGO_MANIFEST_DIR is
    // `D:/Ada/.worktrees/ada-obs-autoremediation/crates/ada-remediation`,
    // so ../../../config/remediation reaches the workspace.
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .join("../../config/remediation")
        .canonicalize()
        .unwrap_or_else(|_| manifest_dir.join("../../config/remediation"))
}

#[test]
fn loads_all_five_default_runbooks() {
    let dir = runbooks_dir();
    assert!(dir.exists(), "runbooks dir missing: {}", dir.display());
    let actions = load_runbooks_from_dir(&dir).expect("load default runbooks");
    let ids: Vec<&str> = actions.iter().map(|a| a.id.as_str()).collect();
    for required in [
        "disk-space-low",
        "service-down-restart-and-page",
        "db-pool-exhausted-kill-idle",
        "slo-burn-fast-page",
        "slo-burn-slow-notify",
    ] {
        assert!(
            ids.contains(&required),
            "missing default action: {required} (have: {ids:?})"
        );
    }
    assert!(
        actions.len() >= 5,
        "expected >=5 default actions, got {}",
        actions.len()
    );
}

#[test]
fn evaluate_maps_each_default_to_at_least_one_alert() {
    let dir = runbooks_dir();
    let actions = load_runbooks_from_dir(&dir).unwrap();
    let engine = RemediationEngine::with_runbooks(actions);

    // Each default runbook declares a specific severity filter;
    // we use a matching severity so the test isolates trigger
    // matching from severity filtering (the latter is covered
    // by `severity_filter_rejects_mismatched_severity` below).
    let cases = [
        ("DiskSpaceFillingFast", "P2", "disk-space-low"),
        ("ServiceDown", "P1", "service-down-restart-and-page"),
        (
            "DBConnectionPoolExhausted",
            "P2",
            "db-pool-exhausted-kill-idle",
        ),
        ("SLIBurnRateFast", "P1", "slo-burn-fast-page"),
        ("SLIBurnRateSlow", "P2", "slo-burn-slow-notify"),
    ];
    for (alert_name, severity, expected_id) in cases {
        let alert = AlertEvent::builder(alert_name)
            .label("severity", severity)
            .build();
        let matched = engine.evaluate(&alert);
        let ids: Vec<&str> = matched.iter().map(|a| a.id.as_str()).collect();
        assert!(
            ids.contains(&expected_id),
            "alert {alert_name} sev={severity} should match {expected_id}, got {ids:?}"
        );
    }
}

#[test]
fn glob_trigger_matches_burn_rate_family() {
    let dir = runbooks_dir();
    let actions = load_runbooks_from_dir(&dir).unwrap();
    let engine = RemediationEngine::with_runbooks(actions);

    let alert = AlertEvent::builder("SLIBurnRateFast")
        .label("severity", "P1")
        .build();
    let matched = engine.evaluate(&alert);
    assert!(matched.iter().any(|a| a.id == "slo-burn-fast-page"));

    let alert = AlertEvent::builder("SLIBurnRateSlow")
        .label("severity", "P2")
        .build();
    let matched = engine.evaluate(&alert);
    assert!(matched.iter().any(|a| a.id == "slo-burn-slow-notify"));
}

#[test]
fn severity_filter_rejects_mismatched_severity() {
    let dir = runbooks_dir();
    let actions = load_runbooks_from_dir(&dir).unwrap();
    let engine = RemediationEngine::with_runbooks(actions);

    // `slo-burn-fast-page` declares severities=["P1"]; P3 must be filtered out.
    let p3 = AlertEvent::builder("SLIBurnRateFast")
        .label("severity", "P3")
        .build();
    let matched = engine.evaluate(&p3);
    assert!(matched.iter().all(|a| a.id != "slo-burn-fast-page"));

    // `disk-space-low` declares severities=["P2","P3"]; P1 must be filtered out.
    let p1 = AlertEvent::builder("DiskSpaceFillingFast")
        .label("severity", "P1")
        .build();
    let matched = engine.evaluate(&p1);
    assert!(matched.iter().all(|a| a.id != "disk-space-low"));
}

fn sample_action() -> RemediationAction {
    use ada_remediation::action::{ActionStep, Trigger};
    use std::time::Duration;
    RemediationAction {
        id: "disk-space-low".into(),
        name: "Disk space low".into(),
        trigger: Trigger::Exact("DiskSpaceFillingFast".into()),
        severities: vec![],
        steps: vec![ActionStep::NotifySlack {
            executor: ExecutorMode::DryRun,
            channel: "#ada-ops".into(),
            message: "disk low".into(),
        }],
        cooldown: Duration::from_secs(60),
        max_retries: 0,
    }
}

/// Shared secret used by every E2E webhook call.
const E2E_WEBHOOK_SECRET: &str = "E2E_SECRET";

fn app_with_state() -> (axum::Router, Arc<RemediationEngine>, MemoryStore) {
    let engine = Arc::new(RemediationEngine::with_runbooks(vec![sample_action()]));
    let store = MemoryStore::new();
    let state = AppState {
        engine: engine.clone(),
        store: store.clone(),
        // E2E tests use the same shared-secret
        // scheme as production. Each test sends
        // `x-webhook-token: E2E_SECRET` to authenticate.
        auth: ada_remediation::auth::AuthState::enabled(E2E_WEBHOOK_SECRET),
    };
    (router(state), engine, store)
}

fn disk_alert_webhook_body() -> Vec<u8> {
    serde_json::to_vec(&AlertmanagerPayload {
        version: Some("4".into()),
        status: Some("firing".into()),
        alerts: vec![AlertmanagerAlert {
            status: "firing".into(),
            labels: serde_json::Map::from_iter([(
                "alertname".into(),
                serde_json::Value::String("DiskSpaceFillingFast".into()),
            )]),
            annotations: serde_json::Map::new(),
            fingerprint: Some("e2e-fp-1".into()),
        }],
    })
    .unwrap()
}

#[tokio::test]
async fn webhook_executes_and_records_history() {
    let (app, _engine, store) = app_with_state();
    let r = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/webhook/alertmanager")
                .header("content-type", "application/json")
                .header("x-webhook-token", E2E_WEBHOOK_SECRET)
                .body(Body::from(disk_alert_webhook_body()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let body_bytes = axum::body::to_bytes(r.into_body(), 65536).await.unwrap();
    let resp: WebhookResponse = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(resp.received, 1);
    assert_eq!(resp.executed, 1);
    assert_eq!(resp.outcomes.len(), 1);
    assert_eq!(store.history_len(), 1);
    assert!(store.is_in_cooldown("disk-space-low"));
}

#[tokio::test]
async fn second_webhook_is_skipped_by_cooldown() {
    let (app, _engine, store) = app_with_state();
    // First delivery: executes.
    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/webhook/alertmanager")
                .header("content-type", "application/json")
                .header("x-webhook-token", E2E_WEBHOOK_SECRET)
                .body(Body::from(disk_alert_webhook_body()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(store.history_len(), 1);

    // Second delivery (same fingerprint payload, same alert_name):
    // engine hits cooldown; `executed=0`, `skipped=1`.
    let r = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/webhook/alertmanager")
                .header("content-type", "application/json")
                .header("x-webhook-token", E2E_WEBHOOK_SECRET)
                .body(Body::from(disk_alert_webhook_body()))
                .unwrap(),
        )
        .await
        .unwrap();
    let body_bytes = axum::body::to_bytes(r.into_body(), 65536).await.unwrap();
    let resp: WebhookResponse = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(resp.executed, 0, "second call should be in cooldown");
    assert!(
        resp.skipped >= 1,
        "skipped count should include cooldown skip"
    );
    // History is unchanged.
    assert_eq!(store.history_len(), 1);
}

#[tokio::test]
async fn manual_trigger_with_force_bypasses_cooldown() {
    let (app, _engine, store) = app_with_state();
    // Warm the cooldown.
    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/webhook/alertmanager")
                .header("content-type", "application/json")
                .header("x-webhook-token", E2E_WEBHOOK_SECRET)
                .body(Body::from(disk_alert_webhook_body()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(store.history_len(), 1);

    // Manual trigger with force=true bypasses cooldown.
    let r = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/remediation/trigger")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&serde_json::json!({
                        "alert_name": "DiskSpaceFillingFast",
                        "labels": { "instance": "host-42" },
                        "severity": "P2",
                        "force": true
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let body_bytes = axum::body::to_bytes(r.into_body(), 65536).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(v["executed"], 1);
    assert_eq!(v["matched"][0], "disk-space-low");
    assert_eq!(store.history_len(), 2);
}

#[tokio::test]
async fn cooldowns_endpoint_reflects_live_state() {
    let (app, _engine, store) = app_with_state();
    // Warm the cooldown.
    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/webhook/alertmanager")
                .header("content-type", "application/json")
                .header("x-webhook-token", E2E_WEBHOOK_SECRET)
                .body(Body::from(disk_alert_webhook_body()))
                .unwrap(),
        )
        .await
        .unwrap();

    let r = app
        .oneshot(
            Request::builder()
                .uri("/remediation/cooldowns")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body_bytes = axum::body::to_bytes(r.into_body(), 65536).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let arr = v["cooldowns"].as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["action_id"], "disk-space-low");
    // The store's own view should agree.
    let live = store.active_cooldowns();
    assert_eq!(live.len(), 1);
    assert_eq!(live[0].0, "disk-space-low");
}
