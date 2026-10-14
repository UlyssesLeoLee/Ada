//! The console's proxy must reach the gateway at the paths the gateway
//! actually serves.
//!
//! ## The defect this gate exists for
//!
//! `crates/gm-console/src/routes.rs` rewrote `/api/<rest>` to
//! `<upstream>/<rest>` before forwarding, reading `/api` as a console-side
//! mount prefix to be removed. It is not one. The gateway mounts its API
//! *under* that prefix:
//!
//! ```text
//! crates/ada-m13-api-gateway/src/router.rs:236
//!     .route("/api/v1/ping", get(ping_handler))
//!     .route("/api/v1/whoami", get(whoami_handler))
//!     .route("/api/v1/canvases/:canvas_id", get(get_canvas))
//! ```
//!
//! and `GM_CONSOLE_UPSTREAM` is the gateway's root,
//! `http://ada-api-gateway:8080`. So the console sent `/api/v1/ping` to the
//! gateway as `/v1/ping`, which matches no route there, and the gateway's
//! fallback answered `404 {"error": ...}`. Every browser call to every
//! endpoint was a 404, and the console's own health check was on
//! `/healthz`, which the console answers locally — so the pod was healthy
//! and useless at the same time.
//!
//! ## Why the existing proxy tests could not see it
//!
//! `api_proxy.rs` runs a stub upstream whose routing table is keyed on the
//! *stripped* spelling:
//!
//! ```text
//! "/v1/pipelines" => ...,
//! "/v1/missing"   => (StatusCode::NOT_FOUND, "upstream not found").into_response(),
//! "/v1/echo"      => ...,
//! other           => (StatusCode::INTERNAL_SERVER_ERROR, format!("unexpected: {other}")),
//! ```
//!
//! The stub and the rewrite agreed with each other, and neither agreed
//! with the gateway. That is the general shape of this class of bug: a
//! double that is wrong in the same direction twice looks right. The stub
//! here is gateway-*shaped* instead — it 404s anything the gateway's router
//! does not serve — so a rewrite and a stub cannot be wrong together
//! again without the test going red.
//!
//! This is a separate file from `api_proxy.rs` rather than a section in it,
//! because the stub is deliberately different and mixing them would make it
//! easy to reintroduce a permissive catch-all.

use axum::{
    body::Body,
    extract::Request,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Router,
};
use bytes::Bytes;
use gm_console::{config::Config, routes};
use http_body_util::BodyExt;
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tokio::net::TcpListener;
use tower::ServiceExt;

/// The gateway's routing table, as `build_router` declares it, plus the
/// login route the api-gateway lane is adding
/// (`POST /api/v1/auth/login`). Anything not in this table is a 404, the
/// way the real gateway's `fallback(not_found)` behaves.
const GATEWAY_ROUTES: &[(&str, &str)] = &[
    ("GET", "/api/v1/ping"),
    ("GET", "/api/v1/whoami"),
    ("GET", "/api/v1/canvases/:canvas_id"),
    ("POST", "/api/v1/canvases/:canvas_id/run"),
    ("POST", "/api/v1/auth/login"),
];

/// Requests the stub saw, as `(method, path_and_query)`.
type Seen = Arc<Mutex<Vec<(String, String)>>>;

/// A stub that behaves like the gateway's router: 200 for a declared
/// route, 404 otherwise. It records what it was asked for, so a test can
/// assert the path the console *sent* and not only the status the console
/// *returned* — those are the two halves of a proxy, and asserting only
/// one of them is how the original defect survived.
async fn gateway_stub(req: Request) -> Response {
    let seen = req.extensions().get::<Seen>().cloned().unwrap_or_default();
    let method = req.method().to_string();
    let path_and_query = req
        .uri()
        .path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| req.uri().path().to_string());
    seen.lock()
        .expect("seen mutex")
        .push((method.clone(), path_and_query.clone()));

    // `:canvas_id` is a path parameter, so a declared route matches a
    // concrete path only by shape. The stub checks segment count and the
    // fixed segments, which is enough to be wrong in the same direction the
    // real gateway would be.
    let segments: Vec<&str> = req
        .uri()
        .path()
        .trim_start_matches('/')
        .split('/')
        .collect();
    let matched = GATEWAY_ROUTES.iter().any(|(m, pattern)| {
        if *m != method {
            return false;
        }
        let pat: Vec<&str> = pattern.trim_start_matches('/').split('/').collect();
        pat.len() == segments.len()
            && pat
                .iter()
                .zip(&segments)
                .all(|(p, s)| p.starts_with(':') || *p == *s)
    });

    if !matched {
        return (
            StatusCode::NOT_FOUND,
            [(header::CONTENT_TYPE, "application/json")],
            r#"{"error":{"code":"NOT_FOUND","message":"no such route"}}"#,
        )
            .into_response();
    }

    if path_and_query == "/api/v1/auth/login" {
        // The login response shape the Flutter client reads: `token` is
        // required, `expires_at` is optional.
        let body = req.into_body().collect().await.expect("body").to_bytes();
        let sent: serde_json::Value =
            serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null);
        // The stub only issues a token for credentials it recognises, so a
        // test that posts the wrong password gets a real 401 rather than a
        // 200 that hides a broken body.
        if sent.get("email").and_then(|v| v.as_str()) == Some("ops@example.test")
            && sent.get("password").and_then(|v| v.as_str()) == Some("correct-horse")
        {
            return (
                StatusCode::OK,
                [(header::CONTENT_TYPE, "application/json")],
                r#"{"token":"jwt-abc123","expires_at":"2026-01-01T00:00:00Z"}"#,
            )
                .into_response();
        }
        return (
            StatusCode::UNAUTHORIZED,
            [(header::CONTENT_TYPE, "application/json")],
            r#"{"error":{"code":"UNAUTHORIZED","message":"bad credentials"}}"#,
        )
            .into_response();
    }

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        r#"{"ok":true}"#,
    )
        .into_response()
}

async fn spawn_gateway() -> (SocketAddr, Seen) {
    let seen: Seen = Arc::new(Mutex::new(Vec::new()));
    let app = Router::new()
        .fallback(gateway_stub)
        .layer(axum::Extension(seen.clone()));
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve");
    });
    (addr, seen)
}

fn console(upstream: SocketAddr) -> axum::Router {
    let cfg = Config {
        bind_addr: "127.0.0.1:0".into(),
        upstream_url: format!("http://{upstream}"),
        static_dir: None,
        log_level: "warn".into(),
        enable_compression: false,
        enable_cors: true,
        allowed_origins: vec!["https://example.test".into()],
    };
    routes::router(Arc::new(cfg))
}

async fn send(app: axum::Router, method: &str, path: &str, body: &str) -> (StatusCode, Bytes) {
    let req = Request::builder()
        .method(method)
        .uri(path)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .expect("request");
    let resp = app.oneshot(req).await.expect("console call");
    let status = resp.status();
    let bytes = resp.into_body().collect().await.expect("body").to_bytes();
    (status, bytes)
}

fn paths_seen(seen: &Seen) -> Vec<String> {
    seen.lock()
        .expect("seen mutex")
        .iter()
        .map(|(_, p)| p.clone())
        .collect()
}

/// A browser call to a real gateway route arrives at the gateway's real
/// route, not at a 404 from its fallback.
#[tokio::test]
async fn a_browser_call_reaches_the_path_the_gateway_serves() {
    let (addr, seen) = spawn_gateway().await;
    let (status, body) = send(console(addr), "GET", "/api/v1/ping", "").await;

    assert_eq!(
        status,
        StatusCode::OK,
        "GET /api/v1/ping did not reach the gateway's route: {}",
        String::from_utf8_lossy(&body)
    );
    assert_eq!(
        paths_seen(&seen),
        vec!["/api/v1/ping".to_string()],
        "the console must forward the path unchanged. It used to strip `/api`, \
         so the gateway was asked for `/v1/ping` and answered 404 from its \
         fallback -- which the stub here would have reported, because it 404s \
         anything the real gateway does not serve."
    );
}

/// The path parameter route, which the old rewrite also broke: it 404'd on
/// the prefix before ever reaching the parameter.
#[tokio::test]
async fn a_path_parameter_route_is_reached() {
    let (addr, seen) = spawn_gateway().await;
    let (status, _) = send(console(addr), "GET", "/api/v1/canvases/42", "").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(paths_seen(&seen), vec!["/api/v1/canvases/42".to_string()]);
}

/// A query string is part of the target and must survive. A rewrite that
/// rebuilds the URL from the path alone loses it, and the gateway then runs
/// a different query than the client asked for — usually silently.
#[tokio::test]
async fn the_query_string_is_forwarded() {
    let (addr, seen) = spawn_gateway().await;
    let (status, _) = send(console(addr), "GET", "/api/v1/ping?full=1&trace=abc", "").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        paths_seen(&seen),
        vec!["/api/v1/ping?full=1&trace=abc".to_string()],
        "the query string is dropped or reordered in transit"
    );
}

/// The login POST, end to end through the console: the body the Flutter
/// client sends, the gateway's route spelling, and the `token` field the
/// client reads.
///
/// This is the case the login page in `apps/gm-console-web/dist/login.html`
/// depends on. If the path is rewritten or the body is not forwarded, the
/// page's `token` is never issued and the sign-in silently fails.
#[tokio::test]
async fn the_login_post_reaches_the_gateway_and_returns_a_token() {
    let (addr, seen) = spawn_gateway().await;
    let (status, body) = send(
        console(addr),
        "POST",
        "/api/v1/auth/login",
        r#"{"email":"ops@example.test","password":"correct-horse"}"#,
    )
    .await;

    assert_eq!(
        status,
        StatusCode::OK,
        "the login POST did not succeed through the console: {}",
        String::from_utf8_lossy(&body)
    );
    let json: serde_json::Value =
        serde_json::from_slice(&body).expect("the gateway's login body is JSON");
    assert_eq!(
        json.get("token").and_then(|v| v.as_str()),
        Some("jwt-abc123"),
        "the `token` field the Flutter client reads (auth_api.dart:32) must \
         survive the proxy: {body:?}"
    );
    assert_eq!(
        paths_seen(&seen),
        vec!["/api/v1/auth/login".to_string()],
        "the login request arrived at the wrong path"
    );
}

/// Rejected credentials stay a 401 and are not turned into a 502, so the
/// page can tell "wrong password" from "console is down". The Flutter
/// client raises `ApiUnauthorizedException` on 401 and
/// `ApiNetworkException` on a transport failure; conflating them sends the
/// user down a re-auth flow for a broken network.
#[tokio::test]
async fn rejected_credentials_stay_a_401() {
    let (addr, _) = spawn_gateway().await;
    let (status, body) = send(
        console(addr),
        "POST",
        "/api/v1/auth/login",
        r#"{"email":"ops@example.test","password":"wrong"}"#,
    )
    .await;

    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "a rejected password must stay 401, got {}: {}",
        status,
        String::from_utf8_lossy(&body)
    );
}

/// A path the gateway does not serve stays a 404, and reaches the gateway
/// to be told so. The console must not invent a status of its own here.
#[tokio::test]
async fn an_unknown_api_route_is_answered_by_the_gateway() {
    let (addr, seen) = spawn_gateway().await;
    let (status, _) = send(console(addr), "GET", "/api/v1/nope", "").await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(
        paths_seen(&seen),
        vec!["/api/v1/nope".to_string()],
        "the request must actually reach the gateway for it to be the \
         gateway's 404 rather than the console's"
    );
}

/// The console's own routes are not proxied. Without this, "forward the
/// path verbatim" could be read as "forward everything", and the local
/// commercial surface would be shadowed by the gateway.
#[tokio::test]
async fn local_routes_are_never_proxied() {
    let (addr, seen) = spawn_gateway().await;
    let app = console(addr);

    let (status, _) = send(app.clone(), "GET", "/healthz", "").await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = send(app.clone(), "GET", "/robots.txt", "").await;
    assert_eq!(status, StatusCode::OK);

    assert!(
        paths_seen(&seen).is_empty(),
        "a local route was forwarded upstream: {:?}",
        paths_seen(&seen)
    );
}
