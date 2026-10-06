//! Gateway router.
//!
//! ## Endpoints
//!
//! Unauthenticated, because a kubelet probe cannot carry a bearer token
//! and a probe that 401s takes the pod out of service:
//!
//! - `GET /health`       — JSON snapshot for human / dashboard
//!   consumption.
//! - `GET /health/live`  — Liveness probe; always 200 OK plain text.
//! - `GET /health/ready` — Readiness probe; 503 when the configured
//!   [`HealthCheck`] returns [`HealthStatus::Unhealthy`](crate::health::HealthStatus::Unhealthy)
//!   or an [`AdaError`], otherwise 200 with the verdict.
//!
//! Authenticated — `Authorization: Bearer <opaque session token>`:
//!
//! - `GET /api/v1/ping`  — Lightweight smoke endpoint (`pong: true`).
//! - `GET /api/v1/whoami` — the resolved principal, so the trust model
//!   is observable end-to-end rather than only asserted in tests.
//! - `GET /api/v1/canvases/:canvas_id` — a business read, gated on
//!   `Read` over `ResourceType::Canvas`.
//! - `POST /api/v1/canvases/:canvas_id/run` — a business action gated on
//!   `Execute` over the same resource. Kept separate from the read
//!   because the two are the smallest pair of actions the bundled
//!   policy actually distinguishes: `role:viewer` has a canvas `read`
//!   row and no `execute` row, so a viewer gets a 200 from one and a
//!   401 from the other. That is what makes these routes a test of the
//!   authorization layer rather than of the authentication layer.
//!
//! The whole `/api` subtree is wrapped in one layer that resolves the
//! principal, so a route added later cannot accidentally ship
//! unauthenticated: it is mounted inside the layer, not beside it.
//!
//! One `/api` route is deliberately *not* in that subtree:
//!
//! - `POST /api/v1/auth/login` — exchanges an email and a password for
//!   a session token. It is the only route reachable without a
//!   credential, because it is the one that issues them. See
//!   [`crate::login`] for its security posture.
//!
//! Note the two are not the same guarantee. The layer proves a
//! *credential* was presented; only the per-handler `require` call
//! proves the policy permitted the *action*. Authentication without
//! authorization is a 200 for every logged-in caller, which is the
//! failure mode a single subtree layer cannot prevent on its own.

use ada_m11_rbac_collab::{Action, ResourceType};
use axum::{
    extract::{DefaultBodyLimit, Extension, FromRequestParts, Path, Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;

use crate::{
    auth::{self, AuthContext, Principal},
    error::ApiError,
    health::{HealthStatus, MemoryHealthCheck},
    login::{self, LOGIN_BODY_LIMIT_BYTES},
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

/// The resolved principal, echoed back.
///
/// Exists so the trust model can be observed: the `tenant` here comes
/// from the server-side session, never from the client's `x-tenant-id`
/// header, which this crate ignores.
#[derive(Debug, Serialize)]
struct Whoami {
    user_id: String,
    tenant_id: String,
    roles: Vec<String>,
}

async fn ping_handler() -> Json<Pong> {
    Json(Pong { pong: true })
}

async fn whoami_handler(Extension(principal): Extension<Principal>) -> Json<Whoami> {
    Json(Whoami {
        user_id: principal.user_id,
        tenant_id: principal.tenant_id,
        roles: principal.roles,
    })
}

/// What a permitted business operation reports back.
#[derive(Debug, Serialize)]
struct Authorized {
    /// Echoed so a caller can confirm which object the decision was
    /// about — two different ids in the same session must not share a
    /// decision.
    object_id: String,
    /// The action the policy actually permitted.
    action: &'static str,
    /// The tenant the decision was made in, which came from the
    /// server-side session rather than any client header.
    tenant_id: String,
}

/// A business read, gated on `Read` over the canvas resource.
async fn get_canvas(
    State(state): State<AppState>,
    Extension(principal): Extension<Principal>,
    Path(canvas_id): Path<String>,
) -> Result<Json<Authorized>, ApiError> {
    auth::require(
        &state.auth,
        &principal,
        ResourceType::Canvas,
        &canvas_id,
        Action::Read,
    )?;
    Ok(Json(Authorized {
        object_id: canvas_id,
        action: "read",
        tenant_id: principal.tenant_id,
    }))
}

/// A business action, gated on `Execute` over the canvas resource.
async fn run_canvas(
    State(state): State<AppState>,
    Extension(principal): Extension<Principal>,
    Path(canvas_id): Path<String>,
) -> Result<Json<Authorized>, ApiError> {
    auth::require(
        &state.auth,
        &principal,
        ResourceType::Canvas,
        &canvas_id,
        Action::Execute,
    )?;
    Ok(Json(Authorized {
        object_id: canvas_id,
        action: "execute",
        tenant_id: principal.tenant_id,
    }))
}

/// Resolve the bearer token for every route in this subtree and put the
/// [`Principal`] in the request extensions.
///
/// A `from_request_parts` rejection is already an [`ApiError::Unauthorized`],
/// so it maps straight to 401. Wrapping the whole `/api` subtree -- rather
/// than each handler -- is the point: there is no way to add a route under
/// `/api` and forget this.
async fn require_principal(
    Extension(auth): Extension<AuthContext>,
    mut request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let (mut parts, body) = request.into_parts();
    let principal = Principal::from_request_parts(&mut parts, &auth).await?;
    request = Request::from_parts(parts, body);
    request.extensions_mut().insert(principal);
    Ok(next.run(request).await)
}

/// 404 for a path no route matched.
///
/// Set **after** `merge(api)`, and deliberately not behind
/// [`require_principal`].
///
/// `Router::layer` wraps `catch_all_fallback` as well as the routes,
/// and `Fallback::merge` keeps the *incoming* router's fallback when
/// both sides are the default one. So the `/api` layer, applied before
/// the merge, was inherited as the whole router's fallback and every
/// unmatched path in the gateway answered 401. That is not a security
/// win — an unknown path returns no data either way — and it hides the
/// one signal you want when a route is mounted with a typo: the 404.
/// Assigning the fallback after the merge takes it back.
async fn not_found() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        ApiError::NotFound("no such route".into()),
    )
}

/// Build the gateway router.
///
/// `/health/*` is unauthenticated so kubelet probes work; everything
/// under `/api` sits behind [`require_principal`].
pub fn build_router(state: AppState) -> Router {
    // Cloned once and reused: the middleware needs its own owned copy
    // (`from_fn_with_state` takes `S` by value) and the extension layer
    // needs another, and `state` itself is consumed by `with_state` at
    // the end. Reading `state.auth` twice instead of moving it would
    // use a partially-moved value.
    let auth = state.auth.clone();
    let api = Router::new()
        .route("/api/v1/ping", get(ping_handler))
        .route("/api/v1/whoami", get(whoami_handler))
        .route("/api/v1/canvases/:canvas_id", get(get_canvas))
        .route("/api/v1/canvases/:canvas_id/run", post(run_canvas))
        .layer(axum::middleware::from_fn_with_state(
            auth.clone(),
            require_principal,
        ));

    // Mounted on the *outer* router, not inside `api`. It has to be:
    // this is the one route a caller reaches without a credential, and
    // putting it in the subtree that `require_principal` wraps would
    // make login require a session it is supposed to issue. The
    // `DefaultBodyLimit` is the one part of the production chain that
    // does apply here — see `LOGIN_BODY_LIMIT_BYTES`.
    let login = Router::new()
        .route("/api/v1/auth/login", post(login::login_handler))
        .layer(DefaultBodyLimit::max(LOGIN_BODY_LIMIT_BYTES));

    Router::new()
        .route("/health", get(health_handler))
        .route("/health/live", get(live_handler))
        .route("/health/ready", get(ready_handler))
        .merge(login)
        .merge(api)
        .fallback(not_found)
        .layer(Extension(auth))
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
