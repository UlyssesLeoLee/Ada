//! gm-console HTTP routes.
//!
//! Three route groups:
//!   1. `/api/*` — reverse-proxy to upstream Ada api-gateway
//!   2. `/healthz`, `/version`, `/license`, `/terms`, `/privacy` — commercial surface
//!   3. `/*` — static fallback (SPA index.html)

use axum::{
    body::Body,
    extract::{Request, State},
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde_json::json;
use std::sync::Arc;

use crate::{config::Config, error::Result};

pub type SharedState = Arc<Config>;

pub fn router(state: SharedState) -> Router {
    Router::new()
        // commercial + ops surface
        .route("/healthz", get(healthz))
        .route("/version", get(version))
        .route("/license", get(license))
        .route("/terms", get(terms))
        .route("/privacy", get(privacy))
        // api reverse proxy
        .route("/api/*path", get(proxy).post(proxy).put(proxy).delete(proxy).patch(proxy))
        // metadata
        .with_state(state)
        .fallback(static_fallback)
}

async fn healthz() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok", "service": "gm-console" }))
}

async fn version() -> Json<serde_json::Value> {
    Json(json!({
        "service": "gm-console",
        "version": env!("CARGO_PKG_VERSION"),
        "ada_platform": env!("CARGO_PKG_VERSION"),
    }))
}

async fn license() -> Json<serde_json::Value> {
    Json(json!({
        "license": env!("CARGO_PKG_LICENSE"),
        "source_url": "https://github.com/UlyssesLeoLee/ada/blob/main/LICENSE",
    }))
}

async fn terms() -> &'static str {
    include_str!("../../../docs/commercial/TERMS.md")
}

async fn privacy() -> &'static str {
    include_str!("../../../docs/commercial/PRIVACY.md")
}

/// Reverse-proxy: rewrites `/api/<rest>` → `<upstream>/<rest>` and forwards the request body.
///
/// Pure forward implementation — auth, rate limit and observability live in api-gateway.
async fn proxy(State(cfg): State<SharedState>, uri: Uri, req: Request) -> Result<Response> {
    let path = uri.path().strip_prefix("/api").unwrap_or(uri.path());
    let upstream = format!("{}{}", cfg.upstream_url.trim_end_matches('/'), path);
    if let Some(q) = uri.query() {
        let _ = format!("{upstream}?{q}");
    }

    tracing::debug!(target: "gm-console::proxy", "proxy {upstream}");

    // Production this should use `reqwest` streaming. Scaffold returns 501 until
    // worker-A wires it up with the real upstream contract.
    let _ = req;
    Ok((StatusCode::NOT_IMPLEMENTED, "proxy not yet wired (worker-A scaffolding)").into_response())
}

/// Static fallback: serves index.html for SPA routes.
async fn static_fallback() -> Response {
    let body = include_str!("../../../apps/gm-console-web/dist/index.html");
    Response::builder()
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .body(Body::from(body.to_string()))
        .unwrap_or_else(|_| (StatusCode::INTERNAL_SERVER_ERROR, "static fallback build failed").into_response())
}
