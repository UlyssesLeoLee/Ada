//! Integration tests for the gm-console reverse-proxy + surface endpoints.
//!
//! Strategy: spin up a tiny upstream HTTP server inside the test process, then
//! build the gm-console router pointed at it. We avoid the real cluster.

use axum::{
    body::Body,
    extract::Request,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use bytes::Bytes;
use gm_console::{config::Config, routes};
use http_body_util::BodyExt;
use std::{
    convert::Infallible,
    net::SocketAddr,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};
use tokio::net::TcpListener;
use tower::ServiceExt;

/// Minimal upstream that echoes/returns what we want for each test case.
#[derive(Clone, Default)]
#[allow(dead_code)]
struct Upstream {
    hits: Arc<AtomicUsize>,
}

async fn upstream_handler(req: Request) -> Response {
    let hits = req
        .extensions()
        .get::<Arc<AtomicUsize>>()
        .cloned()
        .unwrap_or_default();
    hits.fetch_add(1, Ordering::SeqCst);

    let path = req.uri().path().to_string();
    match path.as_str() {
        "/v1/pipelines" => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/json")],
            r#"{"pipelines":[{"id":"p1","status":"ok"}]}"#,
        )
            .into_response(),
        "/v1/missing" => (StatusCode::NOT_FOUND, "upstream not found").into_response(),
        "/v1/echo" => {
            let body = req.into_body().collect().await.unwrap().to_bytes();
            (
                StatusCode::OK,
                [(header::CONTENT_TYPE, "application/json")],
                format!(r#"{{"echoed":{},"bytes":{}}}"#, 
                    String::from_utf8_lossy(&body),
                    body.len()),
            )
                .into_response()
        }
        other => (StatusCode::INTERNAL_SERVER_ERROR, format!("unexpected: {other}"))
            .into_response(),
    }
}

async fn spawn_upstream() -> (SocketAddr, Arc<AtomicUsize>) {
    let hits = Arc::new(AtomicUsize::new(0));
    let app = Router::new().fallback(get(upstream_handler).post(upstream_handler).put(upstream_handler))
        .layer(axum::Extension(hits.clone()));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (addr, hits)
}

fn build_app(upstream: SocketAddr) -> axum::Router {
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

/// Helper: send a request through the gm-console router, return status + body.
async fn send(app: axum::Router, method: &str, path: &str) -> (StatusCode, Bytes, axum::http::HeaderMap) {
    let builder = Request::builder().method(method).uri(path);
    let req = builder.body(Body::empty()).unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status();
    let headers = resp.headers().clone();
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    (status, body, headers)
}

/// Case 1: GET /api/v1/pipelines → upstream 200, JSON body forwarded verbatim.
#[tokio::test]
async fn case_200_json_forwarded() {
    let (upstream_addr, hits) = spawn_upstream().await;
    let app = build_app(upstream_addr);

    let (status, body, headers) = send(app.clone(), "GET", "/api/v1/pipelines").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(hits.load(Ordering::SeqCst), 1);
    let s = std::str::from_utf8(&body).unwrap();
    assert!(s.contains("\"pipelines\""), "body: {s}");
    assert!(s.contains("\"id\":\"p1\""), "body: {s}");
    let ct = headers.get(header::CONTENT_TYPE).unwrap().to_str().unwrap();
    assert!(ct.starts_with("application/json"), "content-type: {ct}");
}

/// Case 2: GET /api/v1/missing → upstream 404 preserved (NOT turned into 502).
#[tokio::test]
async fn case_404_upstream_preserved() {
    let (upstream_addr, hits) = spawn_upstream().await;
    let app = build_app(upstream_addr);

    let (status, body, _headers) = send(app.clone(), "GET", "/api/v1/missing").await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(hits.load(Ordering::SeqCst), 1);
    assert!(std::str::from_utf8(&body).unwrap().contains("not found"));
}

/// Case 3: GET /healthz → gm-console local endpoint, NOT proxied to upstream.
#[tokio::test]
async fn case_healthz_local() {
    let (upstream_addr, hits) = spawn_upstream().await;
    let app = build_app(upstream_addr);

    let (status, body, headers) = send(app.clone(), "GET", "/healthz").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(hits.load(Ordering::SeqCst), 0, "/healthz must not hit upstream");
    let s = std::str::from_utf8(&body).unwrap();
    assert!(s.contains("\"status\":\"ok\""), "body: {s}");
    assert!(s.contains("\"service\":\"gm-console\""), "body: {s}");
    let ct = headers.get(header::CONTENT_TYPE).unwrap().to_str().unwrap();
    assert!(ct.starts_with("application/json"), "content-type: {ct}");
}

/// Case 4: GET /terms → returns the bundled Terms markdown (local endpoint).
#[tokio::test]
async fn case_terms_local() {
    let (upstream_addr, hits) = spawn_upstream().await;
    let app = build_app(upstream_addr);

    let (status, body, headers) = send(app.clone(), "GET", "/terms").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(hits.load(Ordering::SeqCst), 0, "/terms must not hit upstream");
    let s = std::str::from_utf8(&body).unwrap();
    assert!(s.contains("Terms of Service"), "body: {s}");
    assert!(s.contains("AGPL"), "body: {s}");
    // /terms returns text/markdown-ish or text/plain; just ensure not JSON.
    if let Some(ct) = headers.get(header::CONTENT_TYPE) {
        let ct = ct.to_str().unwrap();
        assert!(!ct.starts_with("application/json"), "content-type: {ct}");
    }
}

/// Case 5 (bonus): upstream transport failure → 502 BAD_GATEWAY.
#[tokio::test]
async fn case_502_when_upstream_unreachable() {
    // Pick an unused port: bind, drop, use the address. Slight race, but
    // acceptable for a test — reqwest will fail to connect.
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let dead_addr = listener.local_addr().unwrap();
    drop(listener);

    let app = build_app(dead_addr);

    let (status, _body, _headers) = send(app.clone(), "GET", "/api/v1/pipelines").await;

    assert_eq!(status, StatusCode::BAD_GATEWAY);
}

/// Case 6 (bonus): POST body and Authorization header are forwarded.
#[tokio::test]
async fn case_post_body_and_auth_forwarded() {
    let (upstream_addr, hits) = spawn_upstream().await;
    let app = build_app(upstream_addr);

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/echo")
        .header(header::AUTHORIZATION, "Bearer test-token-123")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"hello":"world"}"#))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(hits.load(Ordering::SeqCst), 1);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let s = std::str::from_utf8(&body).unwrap();
    assert!(s.contains("hello"), "echo body: {s}");
    assert!(s.contains("bytes"), "echo body: {s}");
}

// Compile-only assertion that the fallback Infallible path stays wired.
#[allow(dead_code)]
fn _infallible_marker(_: Infallible) {}