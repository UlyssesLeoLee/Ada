//! Integration tests for the gateway endpoints.
//!
//! These tests drive the router via `tower::ServiceExt::oneshot`, so
//! they exercise the full axum stack (routing, middleware, handler,
//! response shape) without binding a real TCP socket. See
//! [`DOC-MOD-013`](../docs/modules/M-13-api-gateway.md) §3 for the
//! endpoint contracts.
//!
//! The `/api/*` tests are the interesting ones: they are the first
//! coverage this crate has had for its own security boundary. Before
//! them, `/api/*` was reachable with no credential at all.

use std::sync::Arc;

use ada_identity::session::{Session, SessionStore};
use ada_m13_api_gateway::{AppState, MemoryHealthCheck};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use tower::ServiceExt;

fn app() -> axum::Router {
    let state = AppState::new("ada-gateway-test", Arc::new(MemoryHealthCheck::new()))
        .expect("bootstrap auth context");
    ada_m13_api_gateway::build_router(state)
}

/// An app whose session store already holds one token for
/// `viewer@tenant-a`. Used by the authenticated tests.
fn app_with_session() -> (axum::Router, String) {
    app_with_roles("tenant-a", vec!["viewer".into()])
}

/// An app with one minted session carrying `roles` in `tenant_id`.
///
/// Each call builds its own `SessionStore`, so no session is shared
/// between tests — otherwise a revocation in one test would leak into
/// another and the failures would depend on execution order.
fn app_with_roles(tenant_id: &str, roles: Vec<String>) -> (axum::Router, String) {
    let store = Arc::new(SessionStore::new());
    let token = store
        .mint(Session {
            user_id: "user-1".into(),
            tenant_id: tenant_id.into(),
            roles,
            expires_at: std::time::Instant::now() + std::time::Duration::from_secs(300),
        })
        .expect("mint");
    let auth = ada_m13_api_gateway::auth::AuthContext::bootstrap()
        .expect("bootstrap")
        .with_sessions(Arc::clone(&store));
    let state = AppState::with_auth("ada-gateway-test", Arc::new(MemoryHealthCheck::new()), auth);
    (ada_m13_api_gateway::build_router(state), token)
}

/// `GET /api/v1/canvases/:id` with a bearer token.
async fn get_canvas(router: axum::Router, token: &str, id: &str) -> StatusCode {
    router
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/canvases/{id}"))
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("router call")
        .status()
}

/// `POST /api/v1/canvases/:id/run` with a bearer token.
async fn run_canvas(router: axum::Router, token: &str, id: &str) -> StatusCode {
    router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/canvases/{id}/run"))
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("router call")
        .status()
}

/// A session the bundled policy does not recognise must be denied
/// every action.
///
/// This is the negative control for the whole authorization path. If
/// unknown roles were allowed, a row added to the session store by any
/// future login bug would become an all-access credential.
#[tokio::test]
async fn a_session_with_an_unknown_role_is_denied_every_action() {
    for role in ["superuser", "root", "admin,owner", "Role::Owner", ""] {
        let (router, token) = app_with_roles("tenant-a", vec![role.into()]);
        assert_eq!(
            get_canvas(router.clone(), &token, "c1").await,
            StatusCode::UNAUTHORIZED,
            "role {role:?} must not read a canvas"
        );
        assert_eq!(
            run_canvas(router, &token, "c1").await,
            StatusCode::UNAUTHORIZED,
            "role {role:?} must not run a canvas"
        );
    }
}

/// A session with no roles at all is denied, not defaulted.
///
/// The failure this guards against is "empty roles means unrestricted",
/// which is the shape a deserialization default takes when a field is
/// missing.
#[tokio::test]
async fn a_session_with_no_roles_is_denied() {
    let (router, token) = app_with_roles("tenant-a", vec![]);
    assert_eq!(
        get_canvas(router.clone(), &token, "c1").await,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        run_canvas(router, &token, "c1").await,
        StatusCode::UNAUTHORIZED
    );
}

/// `role:viewer` may read a canvas but not run one.
///
/// This is the test that makes the two business routes worth having.
/// Both requests carry a perfectly valid token from the same session, so
/// authentication is identical in both — only the policy differs. If it
/// ever returns the same status for both, the authorization call is not
/// being reached and the routes are authentication-only.
#[tokio::test]
async fn a_viewer_may_read_a_canvas_but_not_run_one() {
    let (router, token) = app_with_roles("tenant-a", vec!["viewer".into()]);
    assert_eq!(
        get_canvas(router.clone(), &token, "c1").await,
        StatusCode::OK
    );
    assert_eq!(
        run_canvas(router, &token, "c1").await,
        StatusCode::UNAUTHORIZED
    );
}

/// `role:editor` has a canvas `execute` row, so this is the other half
/// of the pair above: the 401 is a policy decision, not a blanket
/// refusal of the route.
#[tokio::test]
async fn an_editor_may_read_and_run_a_canvas() {
    let (router, token) = app_with_roles("tenant-a", vec!["editor".into()]);
    assert_eq!(
        get_canvas(router.clone(), &token, "c1").await,
        StatusCode::OK
    );
    assert_eq!(run_canvas(router, &token, "c1").await, StatusCode::OK);
}

/// A subject holds the union of its roles' grants.
///
/// `role:viewer` cannot execute and `role:viewer`-only would be denied;
/// adding `role:executor` grants execute. If authorization resolved only
/// the first role in the list, this returns 401.
#[tokio::test]
async fn an_allow_from_one_role_is_not_masked_by_another() {
    let (router, token) = app_with_roles("tenant-a", vec!["viewer".into(), "executor".into()]);
    assert_eq!(run_canvas(router, &token, "c1").await, StatusCode::OK);
}

/// The decision is per-object, not per-session.
///
/// Two different canvas ids in the same session must not share one
/// answer, so the handler has to pass the requested id into the
/// enforcer rather than a constant.
#[tokio::test]
async fn the_authorized_response_echoes_the_requested_object() {
    let (router, token) = app_with_roles("tenant-a", vec!["viewer".into()]);
    let resp = router
        .oneshot(
            Request::builder()
                .uri("/api/v1/canvases/canvas-42")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("router call");
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["object_id"], "canvas-42");
    assert_eq!(v["action"], "read");
    assert_eq!(
        v["tenant_id"], "tenant-a",
        "the reported tenant must be the session's, not a header's"
    );
}

/// The same client-supplied tenant header cannot unlock a business route
/// either — the tenant-trust test above only proves it on `/whoami`.
#[tokio::test]
async fn a_forged_tenant_header_does_not_reach_a_business_handler() {
    let (router, token) = app_with_roles("tenant-a", vec!["viewer".into()]);
    let resp = router
        .oneshot(
            Request::builder()
                .uri("/api/v1/canvases/c1")
                .header("authorization", format!("Bearer {token}"))
                .header("x-tenant-id", "tenant-evil")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("router call");
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["tenant_id"], "tenant-a");
}

#[tokio::test]
async fn get_health_returns_json_snapshot() {
    let resp = app()
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("router call");

    assert_eq!(resp.status(), StatusCode::OK);
    let ct = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(ct.starts_with("application/json"), "content-type was {ct}");

    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["status"], "healthy");
    assert_eq!(v["name"], "ada-gateway-test");
    assert!(v["version"].is_string());
    assert!(v["timestamp"].is_number());
}

#[tokio::test]
async fn get_health_live_is_plain_ok() {
    let resp = app()
        .oneshot(
            Request::builder()
                .uri("/health/live")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("router call");

    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(body.as_ref(), b"OK");
}

#[tokio::test]
async fn get_health_ready_is_200_when_healthy() {
    let resp = app()
        .oneshot(
            Request::builder()
                .uri("/health/ready")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("router call");

    assert_eq!(resp.status(), StatusCode::OK);
}

/// `/api/*` without a credential is a 401.
///
/// This is the assertion that matters most in this file. Before the
/// auth layer existed, this exact request returned 200 with a body, and
/// nothing in the repository said so.
///
/// Every `/api` route is listed, not just `ping`: a new business route
/// added to the subtree but forgotten in the layer would otherwise be
/// reachable, and a spot check on one route cannot see that.
#[tokio::test]
async fn api_requires_a_bearer_token() {
    for (method, path) in [
        ("GET", "/api/v1/ping"),
        ("GET", "/api/v1/whoami"),
        ("GET", "/api/v1/canvases/c1"),
        ("POST", "/api/v1/canvases/c1/run"),
    ] {
        let resp = app()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(path)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("router call");
        assert_eq!(
            resp.status(),
            StatusCode::UNAUTHORIZED,
            "{method} {path} must require a credential"
        );
    }
}

#[tokio::test]
async fn a_malformed_authorization_header_is_a_401() {
    // Each of these must be rejected without the handler running. A
    // lenient scheme parse is how "Bearer" and "bearer" and
    // "Bearer  x" end up handled by different code in different
    // places.
    for header in [
        "",
        "token abc",
        "Bearer",
        "Bearer ",
        "bearer abc",
        "Basic abc",
    ] {
        let resp = app()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/ping")
                    .header("authorization", header)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("router call");
        assert_eq!(
            resp.status(),
            StatusCode::UNAUTHORIZED,
            "header {header:?} must be rejected"
        );
    }
}

#[tokio::test]
async fn an_unknown_bearer_token_is_a_401() {
    let resp = app()
        .oneshot(
            Request::builder()
                .uri("/api/v1/ping")
                .header("authorization", "Bearer not-a-real-session")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("router call");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

/// A client-supplied `x-tenant-id` cannot change the resolved tenant.
///
/// `gm-console` forwards this header verbatim from the browser, so
/// whatever the client asserts, the tenant that reaches a handler comes
/// from the server-side session.
#[tokio::test]
async fn the_client_cannot_choose_the_tenant() {
    let (router, token) = app_with_session();
    let resp = router
        .oneshot(
            Request::builder()
                .uri("/api/v1/whoami")
                .header("authorization", format!("Bearer {token}"))
                .header("x-tenant-id", "tenant-evil")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("router call");

    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        v["tenant_id"], "tenant-a",
        "tenant must come from the session, not the header"
    );
    assert_eq!(v["user_id"], "user-1");
    assert_eq!(v["roles"][0], "viewer");
}

#[tokio::test]
async fn a_valid_bearer_token_reaches_the_handler() {
    let (router, token) = app_with_session();
    let resp = router
        .oneshot(
            Request::builder()
                .uri("/api/v1/ping")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("router call");

    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["pong"], true);
}

/// Revoking a session takes effect immediately.
#[tokio::test]
async fn a_revoked_session_stops_working() {
    let store = Arc::new(SessionStore::new());
    let token = store
        .mint(Session {
            user_id: "user-1".into(),
            tenant_id: "tenant-a".into(),
            roles: vec!["viewer".into()],
            expires_at: std::time::Instant::now() + std::time::Duration::from_secs(300),
        })
        .expect("mint");
    let auth = ada_m13_api_gateway::auth::AuthContext::bootstrap()
        .expect("bootstrap")
        .with_sessions(Arc::clone(&store));
    let router = ada_m13_api_gateway::build_router(AppState::with_auth(
        "t",
        Arc::new(MemoryHealthCheck::new()),
        auth,
    ));

    let before = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/ping")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("router call");
    assert_eq!(before.status(), StatusCode::OK);

    store.revoke(&token);

    let after = router
        .oneshot(
            Request::builder()
                .uri("/api/v1/ping")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("router call");
    assert_eq!(after.status(), StatusCode::UNAUTHORIZED);
}

/// Health routes stay reachable without a credential.
///
/// If this ever starts returning 401 the kubelet probes fail and the
/// pod is pulled from service while the API is perfectly healthy.
#[tokio::test]
async fn health_routes_stay_unauthenticated() {
    for path in ["/health", "/health/live", "/health/ready"] {
        let resp = app()
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .expect("router call");
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "{path} must stay open for probes"
        );
    }
}

/// An unmatched path is a 404, both outside and inside `/api`.
///
/// This one is not cosmetic. `Router::layer` wraps the router's
/// catch-all fallback as well as its routes, and `Fallback::merge`
/// keeps the incoming router's fallback when both sides are the
/// default — so the `/api` auth layer was being inherited as the whole
/// gateway's fallback and *every* unmatched path answered 401. A route
/// mounted with a typo would have been indistinguishable from a
/// protected one, and the only way to notice was to know the path was
/// wrong already.
///
/// Both cases are asserted because they are separate code paths: a
/// future refactor could restore the outer fallback and leave `/api`
/// behind the layer.
#[tokio::test]
async fn unknown_route_returns_404() {
    for path in ["/no/such/path", "/api/v1/no-such-route", "/api/v1/canvases"] {
        let resp = app()
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .expect("router call");
        assert_eq!(
            resp.status(),
            StatusCode::NOT_FOUND,
            "{path} matched no route, so it must be 404 and not a rejection from the auth layer"
        );
    }
}

/// A wrong method on a real path is a 405, not a 404 and not a 401.
///
/// Included because it is the other way a route mount goes wrong
/// silently, and because `api_requires_a_bearer_token` deliberately
/// probes each path with a specific method — if a method were wrong,
/// that test would be asserting a 404 rather than a 401 and would
/// still be green.
#[tokio::test]
async fn a_wrong_method_on_a_real_path_is_405() {
    let (router, token) = app_with_session();
    let resp = router
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/v1/ping")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("router call");
    assert_eq!(resp.status(), StatusCode::METHOD_NOT_ALLOWED);
}
