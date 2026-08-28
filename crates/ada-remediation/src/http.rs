//! HTTP server: Alertmanager webhook + introspection endpoints.
//!
//! `POST /webhook/alertmanager` accepts a canonical
//! Alertmanager v4 payload (single alert or batch), matches
//! every alert against the runbook table, and dispatches
//! matching actions.
//!
//! The other three endpoints (`/remediation/history`,
//! `/remediation/cooldowns`, `/remediation/trigger`) are
//! inspection / operator-trigger surfaces used by the Grafana
//! dashboard and ad-hoc operator debugging.
//!
//! This module is HTTP-server only — the actual engine
//! execution is delegated to [`crate::engine`].

use crate::action::ActionOutcome;
use crate::alert::{AlertEvent, AlertStatus};
use crate::auth::{now_unix_secs, AuthError, AuthState, SIGNATURE_HEADER, TIMESTAMP_HEADER};
use crate::engine::RemediationEngine;
use crate::error::RemediationError;
use crate::history::{HistoryQuery, MemoryStore};
use axum::body::Bytes;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json};
use axum::routing::{get, post};
use axum::Router;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Shared state passed to every handler.
#[derive(Debug, Clone)]
pub struct AppState {
    pub engine: Arc<RemediationEngine>,
    pub store: MemoryStore,
    /// Webhook + manual-trigger auth. The v0.6.0
    /// handlers accepted every request; v0.7.0
    /// required a shared-secret token; **v0.7.1**
    /// upgrades to **HMAC over the raw body**
    /// (`X-Webhook-Signature`) + replay protection
    /// (`X-Webhook-Timestamp`). See
    /// [`crate::auth`] for the full scheme and the
    /// rationale for the blake3-keyed choice (the
    /// `hmac` / `sha2` crates are not in the offline
    /// `Cargo.lock`).
    pub auth: AuthState,
}

/// Build the axum `Router`. The caller is responsible for
/// binding the listener (`axum::serve(...)`).
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/metrics", get(handle_metrics))
        .route("/webhook/alertmanager", post(handle_alertmanager_webhook))
        .route("/remediation/history", get(handle_history))
        .route("/remediation/cooldowns", get(handle_cooldowns))
        .route("/remediation/trigger", post(handle_manual_trigger))
        .with_state(state)
}

async fn health() -> &'static str {
    "ok"
}

// ----------------------------------------------------------------------
// Prometheus exposition
// ----------------------------------------------------------------------

/// Render the current Prometheus snapshot. Mirrors the
/// `/metrics` endpoint convention from `ada-telemetry`:
/// plain text, no auth (relies on k8s `NetworkPolicy` to
/// keep the path private to the cluster's Prometheus
/// scraper).
async fn handle_metrics(
    State(state): State<AppState>,
) -> ([(axum::http::HeaderName, &'static str); 1], String) {
    // Update the cooldown gauge from the live in-memory
    // store before rendering. Cheap: O(active count).
    crate::metrics::set_cooldown_gauge(f64::from(
        u32::try_from(state.store.active_cooldowns().len()).unwrap_or(u32::MAX),
    ));
    let body = crate::metrics::render();
    (
        [(
            axum::http::HeaderName::from_static("content-type"),
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        body,
    )
}

// ----------------------------------------------------------------------
// Alertmanager webhook
// ----------------------------------------------------------------------

/// Alertmanager v4 webhook payload. The full schema is much
/// larger, but we only need `alerts[].labels.alertname` and
/// `alerts[].labels.severity` plus `status` (firing/resolved).
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct AlertmanagerPayload {
    pub version: Option<String>,
    pub status: Option<String>,
    #[serde(default)]
    pub alerts: Vec<AlertmanagerAlert>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct AlertmanagerAlert {
    pub status: String,
    #[serde(default)]
    pub labels: serde_json::Map<String, serde_json::Value>,
    #[serde(default)]
    pub annotations: serde_json::Map<String, serde_json::Value>,
    #[serde(default)]
    pub fingerprint: Option<String>,
}

impl AlertmanagerAlert {
    fn into_event(self) -> AlertEvent {
        let alert_name = self
            .labels
            .get("alertname")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let severity = self
            .labels
            .get("severity")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let status = match self.status.as_str() {
            "resolved" => AlertStatus::Resolved,
            "suppressed" => AlertStatus::Suppressed,
            _ => AlertStatus::Firing,
        };
        let mut labels = std::collections::BTreeMap::new();
        for (k, v) in self.labels {
            if let Some(s) = v.as_str() {
                labels.insert(k, s.to_string());
            } else {
                labels.insert(k, v.to_string());
            }
        }
        let mut annotations = std::collections::BTreeMap::new();
        for (k, v) in self.annotations {
            if let Some(s) = v.as_str() {
                annotations.insert(k, s.to_string());
            } else {
                annotations.insert(k, v.to_string());
            }
        }
        AlertEvent {
            alert_name,
            status,
            severity,
            labels,
            annotations,
            fingerprint: self.fingerprint,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WebhookResponse {
    pub received: usize,
    pub matched: usize,
    pub executed: usize,
    pub skipped: usize,
    pub outcomes: Vec<ActionOutcome>,
}

async fn handle_alertmanager_webhook(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    body: Bytes,
) -> Result<Json<WebhookResponse>, HttpError> {
    // v0.7.1 webhook auth: HMAC over the raw body +
    // replay-protection timestamp. The body must be
    // read as raw bytes (NOT deserialised first)
    // because the signature is computed over the
    // exact bytes the client sent — any whitespace
    // or key reordering by `serde_json` would
    // invalidate the signature.
    //
    // `verify_request` returns
    //   Ok(())                          - signature matched, timestamp fresh
    //   Err(AuthError::Disabled)        - no env var, fail-closed 503
    //   Err(AuthError::MissingSignature)  - 401
    //   Err(AuthError::MissingTimestamp)  - 401
    //   Err(AuthError::Expired)         - 401
    //   Err(AuthError::InvalidSignature)  - 403
    let signature_header = headers.get(SIGNATURE_HEADER).and_then(|v| v.to_str().ok());
    let timestamp_header = headers.get(TIMESTAMP_HEADER).and_then(|v| v.to_str().ok());
    if let Err(e) =
        state
            .auth
            .verify_request(signature_header, timestamp_header, &body, now_unix_secs())
    {
        return Err(map_auth_error(&e));
    }
    // Body verified. Now deserialise. We do not
    // stream the body into the engine: the
    // Alertmanager payload is small (< 64 KB even
    // for batch alerts) and we need the full
    // `Vec<AlertmanagerAlert>` in memory to walk it.
    let payload: AlertmanagerPayload = serde_json::from_slice(&body).map_err(|e| {
        HttpError(
            StatusCode::BAD_REQUEST,
            format!("alertmanager payload parse error: {e}"),
        )
    })?;
    let received = payload.alerts.len();
    let mut outcomes = Vec::new();
    let mut matched = 0;
    let mut executed = 0;
    let mut skipped = 0;
    for a in payload.alerts {
        let event = a.into_event();
        if event.alert_name.is_empty() {
            skipped += 1;
            continue;
        }
        let actions = state.engine.evaluate(&event);
        if actions.is_empty() {
            skipped += 1;
            continue;
        }
        matched += actions.len();
        for action in actions {
            if state.store.is_in_cooldown(&action.id) {
                skipped += 1;
                continue;
            }
            let outcome = state
                .engine
                .execute(&action)
                .await
                .map_err(HttpError::from)?;
            let ok = matches!(outcome.status, crate::action::OutcomeStatus::Succeeded);
            if ok {
                state
                    .store
                    .record_success(&action.id, action.cooldown, &event.alert_name);
            } else {
                state
                    .store
                    .record_failure(&action.id, &event.alert_name, "see step_results", 0);
            }
            executed += 1;
            outcomes.push(outcome);
        }
    }
    Ok(Json(WebhookResponse {
        received,
        matched,
        executed,
        skipped,
        outcomes,
    }))
}

// ----------------------------------------------------------------------
// History / cooldowns
// ----------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct HistoryQueryParams {
    pub action_id: Option<String>,
    pub since: Option<chrono::DateTime<chrono::Utc>>,
    pub limit: Option<usize>,
}

async fn handle_history(
    State(state): State<AppState>,
    Query(params): Query<HistoryQueryParams>,
) -> Json<serde_json::Value> {
    let q = HistoryQuery {
        action_id: params.action_id,
        since: params.since,
        limit: params.limit,
    };
    let rows = state.store.query_history(&q);
    Json(serde_json::json!({ "history": rows }))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CooldownsResponse {
    pub cooldowns: Vec<CooldownEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CooldownEntry {
    pub action_id: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

async fn handle_cooldowns(State(state): State<AppState>) -> Json<CooldownsResponse> {
    let cooldowns = state
        .store
        .active_cooldowns()
        .into_iter()
        .map(|(action_id, expires_at)| CooldownEntry {
            action_id,
            expires_at,
        })
        .collect();
    Json(CooldownsResponse { cooldowns })
}

// ----------------------------------------------------------------------
// Manual trigger (operator / dashboard "run now" button)
// ----------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ManualTriggerRequest {
    pub alert_name: String,
    #[serde(default)]
    pub labels: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    pub severity: Option<String>,
    #[serde(default)]
    pub force: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManualTriggerResponse {
    pub matched: Vec<String>,
    pub executed: usize,
    pub outcomes: Vec<ActionOutcome>,
}

async fn handle_manual_trigger(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    body: Bytes,
) -> Result<Json<ManualTriggerResponse>, HttpError> {
    // v0.7.1 manual-trigger auth: same HMAC +
    // timestamp scheme as the Alertmanager webhook.
    // The trigger endpoint can run runbooks with
    // `force=true` to bypass cooldowns, so it is
    // explicitly gated even though the Alertmanager
    // webhook is the primary attack surface. See
    // [`crate::auth`] for the credential model and
    // [`map_auth_error`] for the HTTP status
    // mapping.
    let signature_header = headers.get(SIGNATURE_HEADER).and_then(|v| v.to_str().ok());
    let timestamp_header = headers.get(TIMESTAMP_HEADER).and_then(|v| v.to_str().ok());
    if let Err(e) =
        state
            .auth
            .verify_request(signature_header, timestamp_header, &body, now_unix_secs())
    {
        return Err(map_auth_error(&e));
    }
    let req: ManualTriggerRequest = serde_json::from_slice(&body).map_err(|e| {
        HttpError(
            StatusCode::BAD_REQUEST,
            format!("manual trigger payload parse error: {e}"),
        )
    })?;
    let mut event = AlertEvent::builder(req.alert_name.clone())
        .with_status(AlertStatus::Firing)
        .build();
    if let Some(sev) = req.severity {
        event = event.label("severity", sev);
    }
    for (k, v) in req.labels {
        event = event.label(k, v);
    }
    let actions = state.engine.evaluate(&event);
    let mut outcomes = Vec::new();
    let mut executed = 0;
    for a in actions.clone() {
        if !req.force && state.store.is_in_cooldown(&a.id) {
            continue;
        }
        let outcome = state.engine.execute(&a).await.map_err(HttpError::from)?;
        let ok = matches!(outcome.status, crate::action::OutcomeStatus::Succeeded);
        if ok {
            state
                .store
                .record_success(&a.id, a.cooldown, &event.alert_name);
        } else {
            state
                .store
                .record_failure(&a.id, &event.alert_name, "see step_results", 0);
        }
        executed += 1;
        outcomes.push(outcome);
    }
    Ok(Json(ManualTriggerResponse {
        matched: actions.into_iter().map(|a| a.id).collect(),
        executed,
        outcomes,
    }))
}

// ----------------------------------------------------------------------
// Error mapping
// ----------------------------------------------------------------------

#[derive(Debug)]
pub struct HttpError(pub StatusCode, pub String);

impl From<RemediationError> for HttpError {
    fn from(e: RemediationError) -> Self {
        let status = match &e {
            RemediationError::ActionNotFound(_) => StatusCode::NOT_FOUND,
            RemediationError::InvalidRunbook(_) => StatusCode::BAD_REQUEST,
            RemediationError::StepFailed { .. } => StatusCode::UNPROCESSABLE_ENTITY,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        Self(status, e.to_string())
    }
}

impl IntoResponse for HttpError {
    fn into_response(self) -> axum::response::Response {
        (self.0, self.1).into_response()
    }
}

/// Map an [`AuthError`] to the appropriate HTTP status
/// for the response body. Production wiring is
/// fail-closed: a missing `REMEDIATION_WEBHOOK_SECRET`
/// at startup yields 503 for every webhook request
/// until the operator sets the var and restarts. This
/// is the safe default — better to refuse traffic
/// than to silently accept it.
fn map_auth_error(e: &AuthError) -> HttpError {
    match e {
        AuthError::Disabled => HttpError(
            StatusCode::SERVICE_UNAVAILABLE,
            "webhook auth is disabled (REMEDIATION_WEBHOOK_SECRET unset)".into(),
        ),
        AuthError::MissingSignature => HttpError(
            StatusCode::UNAUTHORIZED,
            "missing X-Webhook-Signature header".into(),
        ),
        AuthError::MissingTimestamp => HttpError(
            StatusCode::UNAUTHORIZED,
            "missing or invalid X-Webhook-Timestamp header".into(),
        ),
        AuthError::Expired => HttpError(
            StatusCode::UNAUTHORIZED,
            "request timestamp outside replay window (5 minutes)".into(),
        ),
        AuthError::InvalidSignature => {
            HttpError(StatusCode::FORBIDDEN, "invalid X-Webhook-Signature".into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::{ActionStep, RemediationAction, Trigger};
    use crate::auth::sign;
    use axum::body::Body;
    use axum::http::{Request, StatusCode as AxStatus};
    use std::collections::BTreeMap;
    use std::time::Duration;
    use tower::ServiceExt;

    const TEST_SECRET: &[u8] = b"TEST_SECRET";

    fn sample_action() -> RemediationAction {
        RemediationAction {
            id: "disk-space-low".into(),
            name: "Disk space low".into(),
            trigger: Trigger::Exact("DiskSpaceFillingFast".into()),
            severities: vec![],
            steps: vec![ActionStep::NotifySlack {
                executor: crate::executor::ExecutorMode::DryRun,
                channel: "#ada-ops".into(),
                message: "disk low".into(),
            }],
            cooldown: Duration::from_secs(60),
            max_retries: 0,
        }
    }

    /// Default app for tests that do not exercise
    /// auth. Auth is disabled, which means the
    /// webhook / trigger handlers return 503. Tests
    /// that need auth wire up [`authed_app`].
    fn app() -> Router {
        let state = AppState {
            engine: Arc::new(RemediationEngine::with_runbooks(vec![sample_action()])),
            store: MemoryStore::new(),
            auth: crate::auth::AuthState::disabled(),
        };
        router(state)
    }

    /// App with webhook auth enabled. The secret is
    /// `TEST_SECRET`. Tests that exercise the
    /// webhook / trigger paths sign the body with
    /// this secret and send `X-Webhook-Signature` +
    /// `X-Webhook-Timestamp`. See [`signed_request`]
    /// for the wire-format helper.
    fn authed_app() -> Router {
        let state = AppState {
            engine: Arc::new(RemediationEngine::with_runbooks(vec![sample_action()])),
            store: MemoryStore::new(),
            auth: crate::auth::AuthState::enabled(TEST_SECRET),
        };
        router(state)
    }

    /// Build a `Request` for the webhook / trigger
    /// paths with a valid HMAC signature over
    /// `body`. Tests that need a *bad* signature
    /// (rejection path) call `Request::builder()`
    /// themselves and bypass this helper.
    fn signed_request(method: &str, uri: &str, body: Vec<u8>) -> Request<Body> {
        let sig = sign(TEST_SECRET, &body);
        let ts = crate::auth::now_unix_secs().to_string();
        Request::builder()
            .method(method)
            .uri(uri)
            .header("content-type", "application/json")
            .header("x-webhook-signature", sig)
            .header("x-webhook-timestamp", ts)
            .body(Body::from(body))
            .unwrap()
    }

    #[tokio::test]
    async fn health_returns_ok() {
        let r = app()
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(r.status(), AxStatus::OK);
    }

    #[tokio::test]
    async fn webhook_dispatches_matching_action() {
        let body = serde_json::to_vec(&AlertmanagerPayload {
            version: Some("4".into()),
            status: Some("firing".into()),
            alerts: vec![AlertmanagerAlert {
                status: "firing".into(),
                labels: serde_json::Map::from_iter([(
                    "alertname".into(),
                    serde_json::Value::String("DiskSpaceFillingFast".into()),
                )]),
                annotations: serde_json::Map::new(),
                fingerprint: Some("abc".into()),
            }],
        })
        .unwrap();
        let r = authed_app()
            .oneshot(signed_request("POST", "/webhook/alertmanager", body))
            .await
            .unwrap();
        assert_eq!(r.status(), AxStatus::OK);
        let body_bytes = axum::body::to_bytes(r.into_body(), 65536).await.unwrap();
        let resp: WebhookResponse = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(resp.received, 1);
        assert_eq!(resp.executed, 1);
        assert_eq!(resp.outcomes.len(), 1);
    }

    #[tokio::test]
    async fn webhook_with_unknown_alert_is_noop() {
        let body = serde_json::to_vec(&AlertmanagerPayload {
            version: Some("4".into()),
            status: Some("firing".into()),
            alerts: vec![AlertmanagerAlert {
                status: "firing".into(),
                labels: serde_json::Map::from_iter([(
                    "alertname".into(),
                    serde_json::Value::String("SomeOtherAlert".into()),
                )]),
                annotations: serde_json::Map::new(),
                fingerprint: None,
            }],
        })
        .unwrap();
        let r = authed_app()
            .oneshot(signed_request("POST", "/webhook/alertmanager", body))
            .await
            .unwrap();
        let body_bytes = axum::body::to_bytes(r.into_body(), 65536).await.unwrap();
        let resp: WebhookResponse = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(resp.executed, 0);
        assert_eq!(resp.skipped, 1);
    }

    #[tokio::test]
    async fn history_endpoint_returns_rows() {
        // Share one AppState between the two requests so the
        // webhook execution is visible to the history query.
        let state = AppState {
            engine: Arc::new(RemediationEngine::with_runbooks(vec![sample_action()])),
            store: MemoryStore::new(),
            auth: crate::auth::AuthState::enabled(TEST_SECRET),
        };
        let app = router(state);
        let body = serde_json::to_vec(&AlertmanagerPayload {
            version: Some("4".into()),
            status: Some("firing".into()),
            alerts: vec![AlertmanagerAlert {
                status: "firing".into(),
                labels: serde_json::Map::from_iter([(
                    "alertname".into(),
                    serde_json::Value::String("DiskSpaceFillingFast".into()),
                )]),
                annotations: serde_json::Map::new(),
                fingerprint: Some("h1".into()),
            }],
        })
        .unwrap();
        let r = app
            .clone()
            .oneshot(signed_request("POST", "/webhook/alertmanager", body))
            .await
            .unwrap();
        assert_eq!(r.status(), AxStatus::OK);
        let r2 = app
            .oneshot(
                Request::builder()
                    .uri("/remediation/history")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body_bytes = axum::body::to_bytes(r2.into_body(), 65536).await.unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        let arr = v["history"].as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["action_id"], "disk-space-low");
    }

    #[tokio::test]
    async fn metrics_endpoint_serves_prometheus_text() {
        // Install the recorder so `/metrics` returns real
        // Prometheus text instead of the empty string.
        let _ = crate::metrics::install();
        let state = AppState {
            engine: Arc::new(RemediationEngine::with_runbooks(vec![sample_action()])),
            store: MemoryStore::new(),
            auth: crate::auth::AuthState::disabled(),
        };
        let r = router(state)
            .oneshot(
                Request::builder()
                    .uri("/metrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(r.status(), AxStatus::OK);
        let body_bytes = axum::body::to_bytes(r.into_body(), 65536).await.unwrap();
        let body = std::str::from_utf8(&body_bytes).expect("prometheus text is utf-8");
        if !body.is_empty() {
            for line in body.lines() {
                if line.starts_with('#') || line.trim().is_empty() {
                    continue;
                }
                assert!(
                    line.split_whitespace().count() >= 2,
                    "malformed prometheus line: {line:?}"
                );
            }
        }
    }

    // ----------------------------------------------------------------------
    // Webhook HMAC auth (v0.7.1 hardening)
    // ----------------------------------------------------------------------

    /// The unit-level "valid signature" path is covered
    /// by [`crate::auth::tests::hmac_verify_accepts_valid_signature`].
    /// The end-to-end HTTP test below exercises the
    /// header -> handler -> engine path with a real
    /// axum `oneshot`.
    #[tokio::test]
    async fn webhook_accepts_valid_signature() {
        let body = serde_json::to_vec(&AlertmanagerPayload {
            version: Some("4".into()),
            status: Some("firing".into()),
            alerts: vec![],
        })
        .unwrap();
        let r = authed_app()
            .oneshot(signed_request("POST", "/webhook/alertmanager", body))
            .await
            .unwrap();
        assert_eq!(r.status(), AxStatus::OK);
    }

    #[tokio::test]
    async fn webhook_rejects_missing_signature() {
        // Auth is enabled but the request omits both
        // the signature and the timestamp header.
        // Expect 401 Unauthorized (MissingSignature
        // wins because we check it before the
        // timestamp).
        let body = serde_json::to_vec(&AlertmanagerPayload {
            version: Some("4".into()),
            status: Some("firing".into()),
            alerts: vec![],
        })
        .unwrap();
        let r = authed_app()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/webhook/alertmanager")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(r.status(), AxStatus::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn webhook_rejects_missing_timestamp() {
        // Auth is enabled, signature is present, but
        // the timestamp header is absent. Expect 401.
        let body = serde_json::to_vec(&AlertmanagerPayload {
            version: Some("4".into()),
            status: Some("firing".into()),
            alerts: vec![],
        })
        .unwrap();
        let sig = sign(TEST_SECRET, &body);
        let r = authed_app()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/webhook/alertmanager")
                    .header("content-type", "application/json")
                    .header("x-webhook-signature", sig)
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(r.status(), AxStatus::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn webhook_rejects_expired_timestamp() {
        // Timestamp 10 minutes in the past. Even with
        // a valid signature, the replay window
        // rejects it.
        let body = serde_json::to_vec(&AlertmanagerPayload {
            version: Some("4".into()),
            status: Some("firing".into()),
            alerts: vec![],
        })
        .unwrap();
        let sig = sign(TEST_SECRET, &body);
        let stale_ts = (crate::auth::now_unix_secs() - 600).to_string();
        let r = authed_app()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/webhook/alertmanager")
                    .header("content-type", "application/json")
                    .header("x-webhook-signature", sig)
                    .header("x-webhook-timestamp", stale_ts)
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(r.status(), AxStatus::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn webhook_rejects_tampered_signature() {
        // Body is one alert; we sign a *different*
        // body. The signature will not match — the
        // constant-time compare returns false.
        let body = serde_json::to_vec(&AlertmanagerPayload {
            version: Some("4".into()),
            status: Some("firing".into()),
            alerts: vec![],
        })
        .unwrap();
        let different_body = b"{\"alerts\":[]}".to_vec();
        let sig = sign(TEST_SECRET, &different_body);
        let r = authed_app()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/webhook/alertmanager")
                    .header("content-type", "application/json")
                    .header("x-webhook-signature", sig)
                    .header(
                        "x-webhook-timestamp",
                        crate::auth::now_unix_secs().to_string(),
                    )
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(r.status(), AxStatus::FORBIDDEN);
    }

    // ----------------------------------------------------------------------
    // Manual-trigger auth (v0.7.1 hardening)
    // ----------------------------------------------------------------------

    #[tokio::test]
    async fn manual_trigger_requires_signature() {
        // Auth is enabled but the request omits the
        // signature header. Expect 401 Unauthorized.
        let body = serde_json::to_vec(&ManualTriggerRequest {
            alert_name: "DiskSpaceFillingFast".into(),
            labels: BTreeMap::new(),
            severity: Some("P2".into()),
            force: false,
        })
        .unwrap();
        let r = authed_app()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/remediation/trigger")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(r.status(), AxStatus::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn manual_trigger_accepts_valid_signature() {
        // Auth is enabled and the signed body is
        // accepted. Expect 200 OK.
        let body = serde_json::to_vec(&ManualTriggerRequest {
            alert_name: "DiskSpaceFillingFast".into(),
            labels: BTreeMap::new(),
            severity: Some("P2".into()),
            force: true,
        })
        .unwrap();
        let r = authed_app()
            .oneshot(signed_request("POST", "/remediation/trigger", body))
            .await
            .unwrap();
        assert_eq!(r.status(), AxStatus::OK);
        let body_bytes = axum::body::to_bytes(r.into_body(), 65536).await.unwrap();
        let resp: ManualTriggerResponse = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(resp.matched, vec!["disk-space-low".to_string()]);
        assert_eq!(resp.executed, 1);
    }
}
