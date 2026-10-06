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
        // the login page, as a route rather than only as a file, so `/login`
        // is a real URL. It resolves through the same disk-then-bundled order
        // as the shell: /srv/static/login.html when a document root is
        // configured, the include_str! copy otherwise.
        .route("/login", get(login_page))
        // api reverse proxy — wildcard catches any HTTP method
        .route(
            "/api/*path",
            get(proxy).post(proxy).put(proxy).delete(proxy).patch(proxy),
        )
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
    // `CARGO_PKG_LICENSE` is only the SPDX id, and no SPDX id can express
    // the AGPL §13-style additional term in the repository LICENSE: any
    // commercial exploitation requires prior written consent. Reporting
    // the bare id here understated the terms, so the full LICENSE is served
    // alongside it and the restriction is stated explicitly rather than
    // left for the reader to discover in a file link.
    Json(json!({
        "license": env!("CARGO_PKG_LICENSE"),
        "spdx_note": "SPDX identifies the base license only; the binding terms are in `license_text`.",
        "commercial_use_requires_written_consent": true,
        "permission_contact": "lidian727@gmail.com",
        "source_url": "https://github.com/UlyssesLeoLee/ada/blob/main/LICENSE",
        "license_text": include_str!("../../../LICENSE"),
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

/// Reverse-proxy: forwards `/api/<rest>` to `<upstream>/api/<rest>` along
/// with the request body.
///
/// ## The path is forwarded verbatim, and that is load-bearing
///
/// `GM_CONSOLE_UPSTREAM` names the gateway's *root*
/// (`http://ada-api-gateway:8080`), and the gateway mounts its API under
/// the `/api` prefix — `/api/v1/ping`, `/api/v1/whoami`,
/// `/api/v1/canvases/:id` (crates/ada-m13-api-gateway/src/router.rs:236).
/// So the console's `/api/*` namespace and the gateway's are the same
/// namespace, and the console is a pass-through in front of it.
///
/// This used to strip `/api` before forwarding, on the reading that
/// `/api/<rest>` was a console-side mount prefix to be removed. That
/// reading is wrong, and the result was that no browser call ever reached
/// a real endpoint: `/api/v1/ping` was sent to the gateway as `/v1/ping`,
/// which matches no route there, and the gateway's fallback answered
/// `404 {"error":...}`. Nothing caught it because the proxy tests use a
/// stub upstream keyed on the *stripped* paths, so the stub and the
/// rewrite agreed with each other and neither agreed with the gateway.
/// `tests/proxy_upstream_paths.rs` now pins the gateway's real spelling.
///
/// ## Pure forward, and it stays that way
///
/// Auth, authorization and observability live in the gateway, which
/// mounts its whole `/api` subtree behind a bearer-token layer and takes
/// its tenant from the server-side session. Nothing here may start
/// trusting `x-tenant-id` (or any other client header) for a decision: it
/// is forwarded verbatim so the gateway can log what the client claimed,
/// and it is not a credential. Rate limiting is still not implemented on
/// either side.
async fn proxy(State(cfg): State<SharedState>, uri: Uri, req: Request) -> Result<Response> {
    // Path and query exactly as received. `path_and_query` is origin-form
    // and always carries its leading `/`, so it is appended, not joined.
    let path_and_query = uri
        .path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string());

    let upstream = format!(
        "{}{}",
        cfg.upstream_url.trim_end_matches('/'),
        path_and_query
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

    let status =
        StatusCode::from_u16(upstream_resp.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
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
    let allowed = [
        "authorization",
        "content-type",
        "content-length",
        "content-encoding",
        "accept",
        "accept-encoding",
        "accept-language",
        "user-agent",
        "x-request-id",
        "x-forwarded-for",
        "x-forwarded-proto",
        "x-tenant-id",
        "x-correlation-id",
        "x-trace-id",
    ];
    let denied = [
        "host",
        "connection",
        "transfer-encoding",
        "upgrade",
        "cookie",
        "keep-alive",
        "proxy-authenticate",
        "proxy-authorization",
        "te",
        "trailers",
        "expect",
    ];
    for (name, value) in incoming.iter() {
        let lname = name.as_str().to_ascii_lowercase();
        if denied.contains(&lname.as_str()) {
            continue;
        }
        if !allowed.contains(&lname.as_str()) {
            continue;
        }
        if let (Ok(n), Ok(v)) = (
            ReqHeaderName::from_bytes(name.as_str().as_bytes()),
            HeaderValue::from_bytes(value.as_bytes()),
        ) {
            out.append(n, v);
        }
    }
    out
}

/// Translate upstream response headers back into an axum HeaderMap, again
/// stripping hop-by-hop / forbidden response headers.
fn response_headers(upstream: &reqwest::header::HeaderMap) -> HeaderMap {
    let mut out = HeaderMap::with_capacity(upstream.len());
    let denied = [
        "connection",
        "transfer-encoding",
        "upgrade",
        "keep-alive",
        "proxy-authenticate",
        "proxy-authorization",
        "te",
        "trailers",
        "server",
        "set-cookie",
    ];
    for (name, value) in upstream.iter() {
        let lname = name.as_str().to_ascii_lowercase();
        if denied.contains(&lname.as_str()) {
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
///
/// A path under `/api` is never answered from here. The gateway owns that
/// namespace and its statuses carry meaning (401, 403, 404, 502); handing a
/// bare `/api` back as `200 text/html` tells an API client it called a working
/// endpoint. Verified: `/api` and `/api/` both used to return the SPA shell,
/// because `/api/*path` needs a segment to match.
async fn static_fallback(req: Request) -> Response {
    let path = req.uri().path().to_string();
    if path == "/api" || path.starts_with("/api/") {
        return ApiNotFound.into_response();
    }
    // Try disk fallback first (dev / hot-iteration).
    if let Ok(dir) = std::env::var("GM_CONSOLE_STATIC_DIR") {
        if let Some(resp) = try_disk(&dir, &path).await {
            return resp;
        }
    }
    serve_index()
}

/// 404 for an `/api` path that matched no route, so the gateway's
/// namespace never answers with the SPA shell.
#[derive(Debug)]
struct ApiNotFound;

impl IntoResponse for ApiNotFound {
    fn into_response(self) -> Response {
        (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": { "code": "NOT_FOUND", "message": "no such api route" }
            })),
        )
            .into_response()
    }
}

/// Map a request path to a readable file under `dir`, or `None` if it
/// must not be served.
///
/// ## The rule
///
/// Every path segment is rejected if it starts with `.`. One predicate,
/// two properties:
///
/// - **Containment.** `..` is the only thing in a path that can move you
///   out of a directory, and it always does so as a whole segment. A
///   substring test for `".."` was both too broad (`foo..bar` is a
///   perfectly good filename) and too narrow — it missed the Windows
///   spelling, where `/assets/x\..\..\..\secret` is a single `/`-segment
///   containing backslashes, so the substring was never checked against
///   the segments the filesystem actually resolves.
/// - **Dotfile refusal.** `.env`, `.npmrc` and `.git` are how secrets
///   actually leak out of a static directory. This was not in the old
///   check at all: with `GM_CONSOLE_STATIC_DIR` pointed at a build output
///   directory, `GET /.env` returned its contents. Verified against a
///   running server, not inferred.
///
/// The candidate is then built by pushing validated segments onto the
/// root, rather than by string concatenation, so there is no
/// unvalidated text left for the filesystem to reinterpret.
///
/// ## Do not add percent-decoding here
///
/// The path is used exactly as it arrives, undecoded, and
/// `tokio::fs::read` does not decode either — that is why `%2e%2e` is
/// inert and cannot become `..`. Decoding the path here would look like a
/// correctness improvement and would silently reopen the traversal. If
/// decoding is ever genuinely needed, it has to happen *before* this
/// function, on the segment list.
///
/// Symlinks under `dir` are not resolved or refused. That is an operator
/// decision about what they put in the directory, not something a
/// request can control.
fn resolve_static_path(dir: &str, request_path: &str) -> Option<std::path::PathBuf> {
    if dir.is_empty() {
        return None;
    }
    // Both separators, so a Windows-style traversal cannot hide inside a
    // single `/`-delimited segment.
    let segments: Vec<&str> = request_path
        .trim_start_matches('/')
        .split(['/', '\\'])
        .filter(|s| !s.is_empty())
        .collect();

    // A directory request becomes the index document.
    let segments: Vec<&str> = if segments.is_empty() {
        vec!["index.html"]
    } else {
        segments
    };

    let mut out = std::path::PathBuf::from(dir);
    for seg in segments {
        if seg.starts_with('.') {
            return None;
        }
        out.push(seg);
    }
    Some(out)
}

/// Serve a file from disk under `dir`. Returns `None` when the path is
/// unsafe or the file does not exist; caller falls back to the bundled
/// index.html.
async fn try_disk(dir: &str, path: &str) -> Option<Response> {
    let candidate = resolve_static_path(dir, path)?;
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
        HeaderValue::from_str(&mime)
            .unwrap_or(HeaderValue::from_static("application/octet-stream")),
    );
    resp.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=300"),
    );
    Some(resp)
}

/// Serve the bundled SPA index.html (include_str!).
fn serve_index() -> Response {
    Response::builder()
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .header(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"))
        .body(Body::from(
            include_str!("../../../apps/gm-console-web/dist/index.html").to_string(),
        ))
        .unwrap_or_else(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "static fallback build failed",
            )
                .into_response()
        })
}

/// Serve the login page: `login.html` from the configured document root,
/// and the copy compiled into the binary when there is no document root or
/// no file behind it.
///
/// The route exists so `/login` is the URL rather than `/login.html`.
/// `dist/screenshots/INDEX.md` already documents `/login` as the route for
/// the login screenshot, and an extensionless `login` file would be served
/// as `application/octet-stream` by `mime_guess` -- a download, not a page.
async fn login_page() -> Response {
    if let Ok(dir) = std::env::var("GM_CONSOLE_STATIC_DIR") {
        if let Some(resp) = try_disk(&dir, "/login.html").await {
            return resp;
        }
    }
    let mut resp = text_response(
        include_str!("../../../apps/gm-console-web/dist/login.html"),
        "text/html; charset=utf-8",
    );
    // `text_response` is the shared helper and does not set caching; this
    // page must not sit in a shared cache for the `max-age=300` that
    // `try_disk` puts on a static asset.
    resp.headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    resp
}

// Re-export unused symbols to silence dead-code warnings when
// the proxy module is feature-gated in future.
#[allow(dead_code)]
fn _unused_method_silencer(_: Method) {
    let _ = (StatusCode::OK, header::ACCEPT);
}

#[cfg(test)]
mod tests {
    use super::resolve_static_path;
    use std::path::PathBuf;

    /// The predicate is private, so its edge cases live here rather than
    /// in `tests/static_files.rs`, which tests the *served* behaviour
    /// instead. Widening the public API to reach one function would be
    /// the wrong trade for a security check.
    #[test]
    fn ordinary_files_resolve_under_the_root() {
        let p = |s: &str| resolve_static_path("/srv/static", s);
        assert_eq!(p("/"), Some(PathBuf::from("/srv/static/index.html")));
        assert_eq!(p(""), Some(PathBuf::from("/srv/static/index.html")));
        assert_eq!(p("/a/b/c.js"), Some(PathBuf::from("/srv/static/a/b/c.js")));
        // Repeated and trailing separators are cosmetic.
        assert_eq!(p("//a//b.js"), Some(PathBuf::from("/srv/static/a/b.js")));
    }

    /// A dot *inside* a name is not a dot segment. The old
    /// `path.contains("..")` test rejected these real filenames.
    #[test]
    fn dots_inside_a_name_are_not_a_traversal() {
        assert_eq!(
            resolve_static_path("/srv/static", "/v1..2.js"),
            Some(PathBuf::from("/srv/static/v1..2.js"))
        );
        assert_eq!(
            resolve_static_path("/srv/static", "/a..b/c.js"),
            Some(PathBuf::from("/srv/static/a..b/c.js"))
        );
    }

    #[test]
    fn dot_segments_are_refused() {
        let p = |s: &str| resolve_static_path("/srv/static", s);
        assert_eq!(p("/.."), None);
        assert_eq!(p("/a/../b"), None);
        assert_eq!(p("/a/b/.."), None);
        assert_eq!(p("/./a"), None);
    }

    /// A Windows-style traversal is a single `/`-segment containing
    /// backslashes, so splitting on `/` alone would miss it. This is the
    /// case the old substring check could not see, because it never
    /// compared against the segments the filesystem actually resolves.
    #[test]
    fn backslash_separated_traversal_is_refused() {
        let p = |s: &str| resolve_static_path("/srv/static", s);
        assert_eq!(p(r"/a\b\..\..\c"), None);
        assert_eq!(p(r"..\..\secret"), None);
        assert_eq!(p(r"/a\..\b"), None);
    }

    /// The other half of the predicate: dotfiles are how secrets leave a
    /// static directory. Verified against a running server before the fix
    /// — `GET /.env` returned its contents.
    #[test]
    fn dotfiles_and_dot_directories_are_refused() {
        let p = |s: &str| resolve_static_path("/srv/static", s);
        assert_eq!(p("/.env"), None);
        assert_eq!(p("/.env.local"), None);
        assert_eq!(p("/.git/config"), None);
        assert_eq!(p("/secrets/.aws/credentials"), None);
    }

    /// An empty root is not a usable static directory, and must not
    /// degrade into "serve from the process working directory".
    #[test]
    fn an_empty_root_is_refused() {
        assert_eq!(resolve_static_path("", "/a.js"), None);
    }
}
