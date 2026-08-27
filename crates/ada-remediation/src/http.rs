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
use crate::engine::RemediationEngine;
use crate::error::RemediationError;
use crate::history::{HistoryQuery, MemoryStore};
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
}

/// Build the axum `Router`. The caller is responsible for
/// binding the listener (`axum::serve(...)`).
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
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
    Json(payload): Json<AlertmanagerPayload>,
) -> Result<Json<WebhookResponse>, HttpError> {
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
    Json(req): Json<ManualTriggerRequest>,
) -> Result<Json<ManualTriggerResponse>, HttpError> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::{ActionStep, RemediationAction, Trigger};
    use axum::body::Body;
    use axum::http::{Request, StatusCode as AxStatus};
    use std::time::Duration;
    use tower::ServiceExt;

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

    fn app() -> Router {
        let state = AppState {
            engine: Arc::new(RemediationEngine::with_runbooks(vec![sample_action()])),
            store: MemoryStore::new(),
        };
        router(state)
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
        let r = app()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/webhook/alertmanager")
                    .header("content-type", "application/json")
                    .body(Body::from(
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
                                fingerprint: Some("abc".into()),
                            }],
                        })
                        .unwrap(),
                    ))
                    .unwrap(),
            )
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
        let r = app()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/webhook/alertmanager")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&AlertmanagerPayload {
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
                        .unwrap(),
                    ))
                    .unwrap(),
            )
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
        };
        let app = router(state);
        let r = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/webhook/alertmanager")
                    .header("content-type", "application/json")
                    .body(Body::from(
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
                                fingerprint: Some("h1".into()),
                            }],
                        })
                        .unwrap(),
                    ))
                    .unwrap(),
            )
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
}
