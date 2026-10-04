//! The CORS allow-list must actually be a list.
//!
//! ## The bug this pins
//!
//! `build_cors` applied each configured origin with its own
//! `CorsLayer::allow_origin(..)` call. tower-http 0.6 documents, in the
//! method's own rustdoc, that "multiple calls to this method will override
//! any previous calls", and the body is a plain
//! `self.allow_origin = origin.into()`. So the loop left only the **last**
//! origin in effect.
//!
//! Observed against the real binary, with the shipped default list
//! `["https://gm-console.kanvas.dev", "https://localhost:3000"]`:
//!
//! ```text
//! OPTIONS /api/v1/pipelines  Origin: https://gm-console.kanvas.dev
//!   -> access-control-allow-origin: https://localhost:3000
//! OPTIONS /api/v1/pipelines  Origin: https://evil.example
//!   -> access-control-allow-origin: https://localhost:3000
//! ```
//!
//! Two separate failures in one. The production origin was never allowed,
//! so the deployed console could not call its own API. And a request from
//! an origin that was configured nowhere was handed the same fixed header
//! rather than none, so the policy was "whoever happens to be last in the
//! list" — for the shipped default, the localhost dev origin.
//!
//! ## Why these tests exist at all
//!
//! The CORS layer used to be applied inline inside `serve()`, while the
//! existing integration tests built a bare `routes::router(..)`. No test
//! ever saw a CORS header, which is how a one-line layer-stack mistake
//! survived a test suite that otherwise exercises the proxy thoroughly.
//! `server::app` now exists so tests and `serve` build the *same* stack —
//! a gate that assembled its own router would pass while the shipped one
//! stayed broken.
//!
//! Requests are preflights (`OPTIONS` + `Access-Control-Request-Method`),
//! which `CorsLayer` answers without ever reaching the proxy handler. That
//! makes these tests hermetic: no upstream has to be listening.
//!
//! Each property below was proven to fail when the property is broken.

use axum::{
    body::Body,
    extract::Request,
    http::{header, HeaderMap, Method, StatusCode},
};
use gm_console::{config::Config, server};
use std::sync::Arc;
use tower::ServiceExt;

const ALLOWED_A: &str = "https://gm-console.kanvas.dev";
const ALLOWED_B: &str = "https://localhost:3000";
const HOSTILE: &str = "https://evil.example";

fn app_with(origins: Vec<String>) -> axum::Router {
    let cfg = Config {
        bind_addr: "127.0.0.1:0".into(),
        // Never dialled: a preflight is answered by the CORS layer before
        // the proxy handler runs.
        upstream_url: "http://127.0.0.1:1".into(),
        static_dir: None,
        log_level: "warn".into(),
        enable_compression: false,
        enable_cors: true,
        allowed_origins: origins,
    };
    server::app(Arc::new(cfg))
}

/// Send a CORS preflight and return the response headers.
async fn preflight(origins: Vec<String>, origin: &str) -> (StatusCode, HeaderMap) {
    let request = Request::builder()
        .method(Method::OPTIONS)
        .uri("/api/v1/pipelines")
        .header(header::ORIGIN, origin)
        .header(header::ACCESS_CONTROL_REQUEST_METHOD, "GET")
        .body(Body::empty())
        .expect("valid preflight request");
    let response = app_with(origins)
        .oneshot(request)
        .await
        .expect("app responds");
    (response.status(), response.headers().clone())
}

fn allow_origin(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
        .and_then(|v| v.to_str().ok())
}

#[tokio::test]
async fn every_configured_origin_is_allowed() {
    // The property the bug violated most directly. A single call to
    // allow_origin with the whole list makes this pass; the old per-origin
    // loop answered the last origin for every request, so ALLOWED_A came
    // back as the localhost origin and failed.
    for origin in [ALLOWED_A, ALLOWED_B] {
        let (status, headers) = preflight(vec![ALLOWED_A.into(), ALLOWED_B.into()], origin).await;
        assert!(
            status.is_success(),
            "preflight for {origin} should succeed, got {status}"
        );
        assert_eq!(
            allow_origin(&headers),
            Some(origin),
            "configured origin {origin} was not echoed back; the allow-list \
             is behaving as a single origin, not a list"
        );
    }
}

#[tokio::test]
async fn an_unconfigured_origin_is_not_allowed() {
    // The security half. A preflight from a site nobody configured must
    // come back WITHOUT access-control-allow-origin: a browser blocks the
    // real request without that header, which is the entire point of the
    // allow-list. The old code answered this with the fixed last origin, so
    // asserting only "no error" would have passed against the bug.
    let (status, headers) = preflight(vec![ALLOWED_A.into(), ALLOWED_B.into()], HOSTILE).await;
    assert!(
        status.is_success(),
        "tower-http answers preflights for unknown origins with 200; \
         the browser, not the server, is what blocks. got {status}"
    );
    assert_eq!(
        allow_origin(&headers),
        None,
        "origin {HOSTILE} is not in the allow-list but was granted \
         access-control-allow-origin"
    );
}

#[tokio::test]
async fn an_unparseable_origin_is_dropped_without_losing_the_others() {
    // The warn-and-skip path. A newline is rejected by HeaderValue, so it
    // stands in for any operator-supplied garbage.
    //
    // The bad entry is tried in three positions, which is what makes this
    // test discriminating. With the bad origin first, a handler that
    // `break`s or that clears the list on a parse error is invisible: the
    // two good origins are parsed afterwards and refill it. A typo in a
    // real `GM_CONSOLE_ALLOWED_ORIGINS` can land anywhere in the string,
    // so "first" alone is the one ordering that proves nothing.
    //
    // This is also the case a naive "fix" breaks: skipping the bad entry
    // and then collapsing to a single `allow_origin` call would allow only
    // the last good origin.
    let bad = "https://bad.example\ninjected".to_string();
    let orderings: [Vec<String>; 3] = [
        vec![bad.clone(), ALLOWED_A.into(), ALLOWED_B.into()],
        vec![ALLOWED_A.into(), bad.clone(), ALLOWED_B.into()],
        vec![ALLOWED_A.into(), ALLOWED_B.into(), bad.clone()],
    ];

    for origins in &orderings {
        let (_, headers) = preflight(origins.clone(), ALLOWED_A).await;
        assert_eq!(
            allow_origin(&headers),
            Some(ALLOWED_A),
            "ALLOWED_A was lost when an unparseable origin was present: {origins:?}"
        );

        let (_, headers) = preflight(origins.clone(), ALLOWED_B).await;
        assert_eq!(
            allow_origin(&headers),
            Some(ALLOWED_B),
            "ALLOWED_B was lost when an unparseable origin was present: {origins:?}"
        );

        let (_, headers) = preflight(origins.clone(), HOSTILE).await;
        assert_eq!(
            allow_origin(&headers),
            None,
            "an unconfigured origin was granted access when the allow-list \
             also held an unparseable entry: {origins:?}"
        );
    }
}

#[tokio::test]
async fn an_empty_allow_list_allows_nothing() {
    // Fail-closed. `AllowOrigin::list(vec![])` matches the `CorsLayer`
    // default (`OriginInner::List(vec![])`), so this pins that a
    // misconfiguration that yields no usable origin denies everything
    // rather than falling open.
    let (status, headers) = preflight(Vec::new(), ALLOWED_A).await;
    assert!(status.is_success());
    assert_eq!(allow_origin(&headers), None);
}
