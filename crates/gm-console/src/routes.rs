//! gm-console HTTP routes.
//!
//! Three route groups:
//!   1. `/api/*` — reverse-proxy to upstream Ada api-gateway
//!   2. `/healthz`, `/version`, `/license`, `/terms`, `/privacy` — commercial surface
//!   3. `/*` — static fallback (SPA index.html)
//!
//! Reverse-proxy contract:
//!   - Forwards method, query string, and body verbatim.
//!   - Forwards `Content-*` and `Authorization` headers; strips hop-by-hop + restricted.
//!   - Request timeout 10s; connect timeout 2s.
//!   - Preserves upstream status codes; on transport failure returns 502 BAD_GATEWAY.
//!
//! NO secrets (env values, upstream URLs) are written to logs. The proxy logs only the path.

use axum::{
    body::{to_bytes, Body},
    extract::{Request, State},
    http::{header, HeaderMap, HeaderName, HeaderValue, Method, StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use bytes::Bytes;
use reqwest::header::{HeaderMap as ReqHeaderMap, HeaderName as ReqHeaderName};
use serde_json::json;
use std::{sync::Arc, time::Duration};
use tokio::sync::OnceCell;

use crate::{config::Config, error::Result};

pub type SharedState = Arc<Config>;

/// Lazily-initialized shared reqwest client. Building a client per request is
/// expensive (TLS handshake / pool), so we share one.
static HTTP_CLIENT: OnceCell<reqwest::Client> = OnceCell::const_new();

async fn http_client() -> &'static reqwest::Client {
    HTTP_CLIENT
        .get_or_init(|| async {
            reqwest::Client::builder()
                .connect_timeout(Duration::from_secs(2))
                .timeout(Duration::from_secs(10))
                .pool_idle_timeout(Duration::from_secs(60))
                .build()
                .expect("reqwest client builder must succeed with defaults")
        })
        .await
}

pub fn router(state: SharedState) -> Router {
    Router::new()
        // commercial + ops surface
        .route("/healthz", get(healthz))
        .route("/version", get(version))
        .route("/license", get(license))
        .route("/terms", get(terms))
        .route("/privacy", get(privacy))
        // seo surface (served as bundled files)
        .route("/robots.txt", get(robots_txt))
        .route("/sitemap.xml", get(sitemap_xml))
        // api reverse proxy — wildcard catches any HTTP method
        .route("/api/*path", get(proxy).post(proxy).put(proxy).delete(proxy).patch(proxy))
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

/// SEO helpers: robots.txt + sitemap.xml.
async fn robots_txt() -> Response {
    text_response(
        include_str!("../../../apps/gm-console-web/dist/robots.txt"),
        "text/plain; charset=utf-8",
    )
}

async fn sitemap_xml() -> Response {
    xml_response(include_str!(
        "../../../apps/gm-console-web/dist/sitemap.xml"
    ))
}

fn text_response(body: &'static str, content_type: &'static str) -> Response {
    Response::builder()
        .header(header::CONTENT_TYPE, content_type)
        .body(Body::from(body.to_string()))
        .unwrap_or_else(|_| {
            (StatusCode::INTERNAL_SERVER_ERROR, "static build failed").into_response()
        })
}

fn xml_response(body: &'static str) -> Response {
    Response::builder()
        .header(header::CONTENT_TYPE, "application/xml; charset=utf-8")
        .body(Body::from(body.to_string()))
        .unwrap_or_else(|_| {
            (StatusCode::INTERNAL_SERVER_ERROR, "static build failed").into_response()
        })
}

/// Maximum request body we forward to upstream. 16 MiB — anything bigger is a
/// streaming/large-file concern that should not hit the proxy.
const MAX_PROXY_BODY: usize = 16 * 1024 * 1024;

/// Reverse-proxy: rewrites `/api/<rest>` → `<upstream>/<rest>` and forwards the request body.
///
/// Pure forward implementation — auth, rate limit and observability live in api-gateway.
async fn proxy(State(cfg): State<SharedState>, uri: Uri, req: Request) -> Result<Response> {
    // Strip the `/api` prefix; preserve everything else (path + query).
    let path_and_query = uri
        .path_and_query()
        .map(|pq| pq.as_str().strip_prefix("/api").unwrap_or(pq.as_str()).to_string())
        .unwrap_or_else(|| uri.path().to_string());

    let upstream = format!(
        "{}/{}",
        cfg.upstream_url.trim_end_matches('/'),
        path_and_query.trim_start_matches('/')
    );

    // NEVER log the upstream URL (could carry secrets in query) or env values.
    tracing::debug!(target: "gm-console::proxy", method = %req.method(), "proxy forward");

    let method = reqwest::Method::from_bytes(req.method().as_str().as_bytes())
        .map_err(|e| crate::Error::Internal(format!("invalid method: {e}")))?;
    let headers = forward_headers(req.headers());

    // Buffer the body to a Bytes — streaming forwarding is possible but increases
    // complexity without a measured benefit for our internal cluster traffic.
    let body = to_bytes(req.into_body(), MAX_PROXY_BODY)
        .await
        .map_err(|e| crate::Error::Internal(format!("body read failed: {e}")))?;

    let client = http_client().await;
    let upstream_resp = client
        .request(method, &upstream)
        .headers(headers)
        .body(body)
        .send()
        .await
        .map_err(|e| {
            tracing::warn!(target: "gm-console::proxy", error = %e, "upstream transport error");
            crate::Error::Upstream(e.to_string())
        })?;

    let status = StatusCode::from_u16(upstream_resp.status().as_u16())
        .unwrap_or(StatusCode::BAD_GATEWAY);
    let resp_headers = response_headers(upstream_resp.headers());
    let body_bytes = upstream_resp
        .bytes()
        .await
        .map_err(|e| crate::Error::Upstream(format!("upstream body read failed: {e}")))?;

    let mut response = Response::new(Body::from(body_bytes));
    *response.status_mut() = status;
    *response.headers_mut() = resp_headers;
    Ok(response)
}

/// Forward client → upstream headers. We keep `Content-*` and `Authorization` verbatim;
/// strip hop-by-hop + Host + reqwest-restricted.
fn forward_headers(incoming: &HeaderMap) -> ReqHeaderMap {
    let mut out = ReqHeaderMap::with_capacity(incoming.len());
    let allowed = ["authorization", "content-type", "content-length", "content-encoding",
                   "accept", "accept-encoding", "accept-language", "user-agent",
                   "x-request-id", "x-forwarded-for", "x-forwarded-proto", "x-tenant-id",
                   "x-correlation-id", "x-trace-id"];
    let denied = ["host", "connection", "transfer-encoding", "upgrade", "cookie",
                  "keep-alive", "proxy-authenticate", "proxy-authorization", "te",
                  "trailers", "expect"];
    for (name, value) in incoming.iter() {
        let lname = name.as_str().to_ascii_lowercase();
        if denied.iter().any(|d| *d == lname.as_str()) {
            continue;
        }
        if !allowed.iter().any(|a| *a == lname.as_str()) {
            continue;
        }
        if let (Ok(n), Ok(v)) = (ReqHeaderName::from_bytes(name.as_str().as_bytes()),
                                 HeaderValue::from_bytes(value.as_bytes())) {
            out.append(n, v);
        }
    }
    out
}

/// Translate upstream response headers back into an axum HeaderMap, again
/// stripping hop-by-hop / forbidden response headers.
fn response_headers(upstream: &reqwest::header::HeaderMap) -> HeaderMap {
    let mut out = HeaderMap::with_capacity(upstream.len());
    let denied = ["connection", "transfer-encoding", "upgrade", "keep-alive",
                  "proxy-authenticate", "proxy-authorization", "te", "trailers",
                  "server", "set-cookie"];
    for (name, value) in upstream.iter() {
        let lname = name.as_str().to_ascii_lowercase();
        if denied.iter().any(|d| *d == lname.as_str()) {
            continue;
        }
        if let (Ok(n), Ok(v)) = (
            HeaderName::from_bytes(name.as_str().as_bytes()),
            HeaderValue::from_bytes(value.as_bytes()),
        ) {
            out.append(n, v);
        }
    }
    out
}

/// Static fallback: serves index.html for SPA routes.
///
/// If `GM_CONSOLE_STATIC_DIR` is set and the requested path resolves to a file
/// under that directory, serve it (with mime sniffing). Otherwise serve the
/// bundled `apps/gm-console-web/dist/index.html`.
async fn static_fallback(req: Request) -> Response {
    let path = req.uri().path().to_string();
    // Try disk fallback first (dev / hot-iteration).
    if let Ok(dir) = std::env::var("GM_CONSOLE_STATIC_DIR") {
        if let Some(resp) = try_disk(&dir, &path).await {
            return resp;
        }
    }
    serve_index()
}

/// Serve a file from disk under `dir`. Returns `None` when the path is unsafe
/// or the file does not exist; caller falls back to bundled index.html.
async fn try_disk(dir: &str, path: &str) -> Option<Response> {
    // Reject any path-traversal attempts.
    if path.contains("..") {
        return None;
    }
    let stripped = path.trim_start_matches('/');
    // SPA: a directory request becomes /<dir>/index.html
    let candidate = if stripped.is_empty() {
        format!("{dir}/index.html")
    } else {
        format!("{dir}/{stripped}")
    };
    let bytes: Bytes = match tokio::fs::read(&candidate).await {
        Ok(b) => Bytes::from(b),
        Err(_) => return None,
    };
    let mime = mime_guess::from_path(&candidate)
        .first_or_octet_stream()
        .to_string();
    let mut resp = Response::new(Body::from(bytes));
    *resp.status_mut() = StatusCode::OK;
    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(&mime).unwrap_or(HeaderValue::from_static("application/octet-stream")),
    );
    resp.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=300"),
    );
    Some(resp)
}

/// Serve the bundled SPA index.html (rust-embed via include_str!).
fn serve_index() -> Response {
    let body = include_str!("../../../apps/gm-console-web/dist/index.html");
    Response::builder()
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .header(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"))
        .body(Body::from(body.to_string()))
        .unwrap_or_else(|_| {
            (StatusCode::INTERNAL_SERVER_ERROR, "static fallback build failed").into_response()
        })
}

// Re-export unused symbols to silence dead-code warnings when
// the proxy module is feature-gated in future.
#[allow(dead_code)]
fn _unused_method_silencer(_: Method) {
    let _ = (StatusCode::OK, header::ACCEPT);
}