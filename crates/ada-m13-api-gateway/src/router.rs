//! Gateway router + the four canonical endpoints.
//!
//! ## Endpoints
//!
//! - `GET /health`       — JSON snapshot for human / dashboard
//!   consumption.
//! - `GET /health/live`  — Liveness probe; always 200 OK plain text.
//! - `GET /health/ready` — Readiness probe; 503 when the configured
//!   [`HealthCheck`] returns [`HealthStatus::Unhealthy`](crate::health::HealthStatus::Unhealthy)
//!   or an [`AdaError`], otherwise 200 with the verdict.
//! - `GET /api/v1/ping`  — Lightweight smoke endpoint used by the
//!   deployment pipeline (`pong: true`).
//!
//! See [`DOC-MOD-013`](../docs/modules/M-13-api-gateway.md) §3.1 for
//! the full middleware chain (CORS / HSTS / JWT / tenant / RBAC) that
//! production builds wrap around [`build_router`]. The v0.1.0 skeleton
//! mounts only the four endpoints above; the middleware chain is added
//! in B3+.

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::Serialize;

use crate::{
    error::ApiError,
    health::{HealthStatus, MemoryHealthCheck},
    state::AppState,
};

/// JSON payload returned by `GET /health`.
#[derive(Debug, Serialize)]
pub struct HealthSnapshot {
    /// Verdict, one of `"healthy" | "degraded" | "unhealthy"`.
    pub status: &'static str,
    /// Service name from [`AppState`].
    pub name: String,
    /// Crate version reported back to the caller.
    pub version: &'static str,
    /// `ms since UNIX epoch` produced by the health check.
    pub timestamp: u128,
}

async fn health_handler(State(state): State<AppState>) -> Json<HealthSnapshot> {
    let verdict = state.db.check().await.unwrap_or(HealthStatus::Unhealthy);
    let status = match verdict {
        HealthStatus::Healthy => "healthy",
        HealthStatus::Degraded => "degraded",
        HealthStatus::Unhealthy => "unhealthy",
    };
    Json(HealthSnapshot {
        status,
        name: state.name,
        version: env!("CARGO_PKG_VERSION"),
        timestamp: MemoryHealthCheck::timestamp_millis(),
    })
}

async fn live_handler() -> Response {
    (StatusCode::OK, "OK").into_response()
}

async fn ready_handler(State(state): State<AppState>) -> Response {
    match state.db.check().await {
        Ok(HealthStatus::Healthy | HealthStatus::Degraded) => {
            (StatusCode::OK, "ready").into_response()
        }
        Ok(HealthStatus::Unhealthy) => {
            ApiError::ServiceUnavailable("not ready".into()).into_response()
        }
        Err(e) => ApiError::ServiceUnavailable(format!("health probe failed: {e}")).into_response(),
    }
}

#[derive(Debug, Serialize)]
struct Pong {
    pong: bool,
}

async fn ping_handler() -> Json<Pong> {
    Json(Pong { pong: true })
}

/// Build the gateway router with the four v0.1.0 endpoints mounted.
///
/// CORS / tracing layers are added in a future release; for now the
/// function returns the bare router so that the integration tests
/// can drive it with `tower::ServiceExt::oneshot`.
pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .route("/health/live", get(live_handler))
        .route("/health/ready", get(ready_handler))
        .route("/api/v1/ping", get(ping_handler))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_snapshot_serializes() {
        let s = HealthSnapshot {
            status: "healthy",
            name: "ada-gateway".to_string(),
            version: "0.1.0",
            timestamp: 1_700_000_000_000,
        };
        let json = serde_json::to_value(&s).unwrap();
        assert_eq!(json["status"], "healthy");
        assert_eq!(json["name"], "ada-gateway");
        assert_eq!(json["version"], "0.1.0");
        assert_eq!(json["timestamp"], 1_700_000_000_000_u64);
    }
}
