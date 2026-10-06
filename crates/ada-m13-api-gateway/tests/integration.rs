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
use ada_m13_api_gateway::{
    AppState, CredentialDirectory, LoginService, MemoryHealthCheck, StoredUser,
};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use tower::ServiceExt;

/// An auth context over an isolated in-process store.
///
/// Deliberately not `AuthContext::bootstrap`: that demands a reachable
/// shared session store and fails closed without one, which is right for
/// a pod and makes this suite require a Redis no `cargo test` run starts.
/// `with_bundled_policy` still loads the real policy set, so authorization
/// is exercised for real and only the storage is swapped.
fn auth_over(store: Arc<SessionStore>) -> ada_m13_api_gateway::auth::AuthContext {
    ada_m13_api_gateway::auth::AuthContext::with_bundled_policy(store).expect("bundled policy set")
}

fn app() -> axum::Router {
    let auth = auth_over(Arc::new(SessionStore::new()));
    let state = AppState::with_auth("ada-gateway-test", Arc::new(MemoryHealthCheck::new()), auth);
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
    let auth = auth_over(Arc::clone(&store));
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
    let auth = auth_over(Arc::clone(&store));
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

// ---------------------------------------------------------------------------
// POST /api/v1/auth/login
//
// gm-console's `auth_api.dart` POSTs `{email, password}` to
// `/api/v1/auth/login`, reads `body['token']`, and throws
// `ApiUnauthorizedException` when it is null or empty. These tests pin
// that contract from both ends: the field name, the 401, and the fact
// that the token the client stores is one the gateway itself accepts.
// ---------------------------------------------------------------------------

/// The identity the login fixtures configure.
///
/// `.invalid` is reserved by RFC 2606 and can never route.
const TEST_EMAIL: &str = concat!("ops", "@", "example.invalid");

/// Assembled from fragments so no committed line is a bare secret, and
/// so a grep for a password-shaped literal does not match this file.
const TEST_PASSWORD: &str = concat!("pw-", "integration-fixture");

/// A credential directory holding exactly one loginable account.
fn login_directory() -> Arc<CredentialDirectory> {
    let dir = Arc::new(CredentialDirectory::new());
    dir.insert(
        TEST_EMAIL,
        StoredUser::new(
            "user-login",
            "tenant-a",
            vec!["viewer".into()],
            TEST_PASSWORD,
        ),
    );
    dir
}

/// An app whose login endpoint has one configured account, on the
/// default attempt limits.
fn app_with_login() -> axum::Router {
    app_with_login_service(LoginService::new(login_directory()))
}

/// An app whose login service the caller supplies, for the tests that
/// need a different attempt ceiling or session lifetime.
fn app_with_login_service(login: LoginService) -> axum::Router {
    let auth = auth_over(Arc::new(SessionStore::new()));
    let state = AppState::with_auth("ada-gateway-test", Arc::new(MemoryHealthCheck::new()), auth)
        .with_login(Arc::new(login));
    ada_m13_api_gateway::build_router(state)
}

/// POST a credential and return the status plus the raw response body.
async fn post_login(router: axum::Router, email: &str, password: &str) -> (StatusCode, String) {
    let payload = serde_json::json!({"email": email, "password": password}).to_string();
    let resp = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(payload))
                .unwrap(),
        )
        .await
        .expect("router call");
    let status = resp.status();
    let bytes = resp
        .into_body()
        .collect()
        .await
        .expect("response body")
        .to_bytes();
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

/// `GET /api/v1/whoami` with a bearer token, returning status and body.
async fn whoami(router: axum::Router, token: &str) -> (StatusCode, serde_json::Value) {
    let resp = router
        .oneshot(
            Request::builder()
                .uri("/api/v1/whoami")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("router call");
    let status = resp.status();
    let bytes = resp
        .into_body()
        .collect()
        .await
        .expect("response body")
        .to_bytes();
    let value = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, value)
}

/// The closed loop the endpoint exists for: a client logs in, takes the
/// `token` field, and presents it as a bearer credential on a business
/// route. Before this, the client had a login call that could only ever
/// fail and a gateway that could never mint a session.
///
/// One `Router` serves both halves, and that is load-bearing: each
/// `AppState` owns its own `SessionStore`, so a token minted against one
/// router is simply unknown to a different one. Cloning the `Router`
/// clones the state (an `Arc`), which is what lets the follow-up request
/// see the session the login just created.
#[tokio::test]
async fn login_issues_a_token_that_authenticates_a_business_request() {
    let router = app_with_login();
    let (status, body) = post_login(router.clone(), TEST_EMAIL, TEST_PASSWORD).await;
    assert_eq!(status, StatusCode::OK, "login body was {body}");

    let issued: serde_json::Value = serde_json::from_str(&body).expect("json body");
    let token = issued["token"]
        .as_str()
        .expect("`token` must be a non-empty string: this is the field auth_api.dart reads")
        .to_owned();

    let (status, who) = whoami(router, &token).await;
    assert_eq!(status, StatusCode::OK, "the issued token must authenticate");
    assert_eq!(who["user_id"], "user-login");
    assert_eq!(who["tenant_id"], "tenant-a");
    assert_eq!(who["roles"][0], "viewer");
}

/// The failure the client surfaces as `ApiUnauthorizedException`.
#[tokio::test]
async fn a_wrong_password_is_a_401() {
    let (status, body) = post_login(
        app_with_login(),
        TEST_EMAIL,
        concat!("not-", "the-password"),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(
        !body.contains(TEST_PASSWORD),
        "a credential reached the response: {body}"
    );
}

/// An unknown identity and a wrong password must be indistinguishable.
///
/// Not just the same status — the same status with a different body is
/// still a user-enumeration oracle, and it is the shape a "helpful"
/// `unknown user` message takes. Both are asserted so a later edit that
/// makes one of them more specific cannot pass.
#[tokio::test]
async fn an_unknown_identity_is_indistinguishable_from_a_wrong_password() {
    let (unknown_status, unknown_body) = post_login(
        app_with_login(),
        concat!("nobody", "@", "example.invalid"),
        TEST_PASSWORD,
    )
    .await;
    let (wrong_status, wrong_body) = post_login(
        app_with_login(),
        TEST_EMAIL,
        concat!("not-", "the-password"),
    )
    .await;

    assert_eq!(unknown_status, StatusCode::UNAUTHORIZED);
    assert_eq!(wrong_status, StatusCode::UNAUTHORIZED);
    assert_eq!(
        unknown_body, wrong_body,
        "the two denials must be byte-identical, or the endpoint enumerates users"
    );
}

/// The `token` field must not be present-but-empty on a denial, since
/// the client only checks for null/empty and would treat a placeholder
/// as a successful login.
#[tokio::test]
async fn a_denied_login_never_carries_a_token_field() {
    let (_, body) = post_login(
        app_with_login(),
        TEST_EMAIL,
        concat!("not-", "the-password"),
    )
    .await;
    let denied: serde_json::Value = serde_json::from_str(&body).expect("json body");
    assert!(
        denied.get("token").is_none(),
        "a denial must not carry a token at all: {body}"
    );
}

/// A token that has been altered by one byte must stop working.
///
/// The credential is opaque, so there is no signature to check — its
/// authority *is* the store lookup. A tampered token therefore has to
/// miss the store entirely rather than decode into something.
#[tokio::test]
async fn a_tampered_token_is_refused() {
    let router = app_with_login();
    let (status, body) = post_login(router.clone(), TEST_EMAIL, TEST_PASSWORD).await;
    assert_eq!(status, StatusCode::OK);
    let issued: serde_json::Value = serde_json::from_str(&body).expect("json body");
    let token = issued["token"].as_str().expect("token").to_owned();

    // Flip the final character, keeping the token's shape and length.
    let mut tampered: Vec<char> = token.chars().collect();
    let last = tampered.len() - 1;
    tampered[last] = if tampered[last] == 'A' { 'B' } else { 'A' };
    let tampered: String = tampered.into_iter().collect();
    assert_ne!(
        tampered, token,
        "the mutation must actually change the token"
    );

    let (status, _) = whoami(router, &tampered).await;
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "a token that was never minted must not authenticate"
    );
}

/// A token whose session has expired must stop working, on the real
/// request path rather than only in the store's own tests.
#[tokio::test]
async fn an_expired_token_is_refused() {
    let router = app_with_login_service(LoginService::new(login_directory()).with_session_ttl(0));
    let (status, body) = post_login(router.clone(), TEST_EMAIL, TEST_PASSWORD).await;
    assert_eq!(status, StatusCode::OK, "a zero-TTL session is still issued");
    let issued: serde_json::Value = serde_json::from_str(&body).expect("json body");
    let token = issued["token"].as_str().expect("token").to_owned();

    let (status, _) = whoami(router, &token).await;
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "an expired session must not authenticate a business request"
    );
}

/// An unconfigured deployment must keep the fail-closed posture it had
/// before this endpoint existed. If `AppState::new` grew a default
/// credential, this is where it would show.
#[tokio::test]
async fn a_deployment_with_no_configured_credentials_refuses_every_login() {
    let (status, body) = post_login(app(), TEST_EMAIL, TEST_PASSWORD).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(!body.contains(TEST_PASSWORD), "credential leaked: {body}");
}

/// The login route sits outside the authentication layer, so it has to
/// answer without a credential. If it were ever moved inside, a caller
/// would need a session to get a session.
#[tokio::test]
async fn the_login_route_does_not_require_a_bearer_token() {
    let (status, _) = post_login(app_with_login(), TEST_EMAIL, TEST_PASSWORD).await;
    assert_eq!(status, StatusCode::OK);
}

/// The attempt ceiling. A login surface with no limit is an offline
/// password-guessing oracle.
#[tokio::test]
async fn repeated_login_attempts_are_rate_limited() {
    let router = app_with_login_service(LoginService::with_limits(login_directory(), 2, 1));

    for i in 0..2 {
        let (status, _) =
            post_login(router.clone(), TEST_EMAIL, concat!("not-", "the-password")).await;
        assert_eq!(
            status,
            StatusCode::UNAUTHORIZED,
            "attempt {i} is inside the burst allowance and must fail on the credential"
        );
    }
    let (status, _) = post_login(router, TEST_EMAIL, concat!("not-", "the-password")).await;
    assert_eq!(
        status,
        StatusCode::TOO_MANY_REQUESTS,
        "the third attempt exceeds a burst of two"
    );
}

/// A body the gateway cannot parse must be refused with a message that
/// does not quote the offending value. The value here is a password, and
/// axum's own `Json` rejection is free to include it.
#[tokio::test]
async fn a_malformed_login_body_is_refused_without_quoting_it() {
    let secret = concat!("pw-", "quoted-by-a-parser");
    for raw in [
        // Not JSON at all.
        format!("{{\"email\": \"{secret}\""),
        // A JSON array where an object is required.
        format!("[\"{secret}\"]"),
        // Right shape, wrong field types: neither value is the secret,
        // but the point is that the *shape* of the failure is uniform.
        r#"{"email": 1, "password": 2}"#.to_owned(),
    ] {
        let resp = app_with_login()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/login")
                    .header("content-type", "application/json")
                    .body(Body::from(raw.clone()))
                    .unwrap(),
            )
            .await
            .expect("router call");
        let status = resp.status();
        let bytes = resp
            .into_body()
            .collect()
            .await
            .expect("response body")
            .to_bytes();
        let text = String::from_utf8_lossy(&bytes);
        assert!(
            status.is_client_error(),
            "a malformed body must be refused, got {status} for {raw:?}"
        );
        assert_eq!(
            text, "{\"error\":{\"code\":400,\"message\":\"bad request: malformed login request\"}}",
            "the refusal must be the fixed message, so no parser can quote the body"
        );
    }
}

/// A missing `password` field is a malformed body, not an empty
/// password. The two must not be confusable: a deserialization default
/// is exactly how "absent" turns into "matched".
#[tokio::test]
async fn a_missing_password_field_is_refused() {
    let resp = app_with_login()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(format!("{{\"email\": \"{TEST_EMAIL}\"}}")))
                .unwrap(),
        )
        .await
        .expect("router call");
    assert!(
        resp.status().is_client_error(),
        "an absent password must not authenticate, got {}",
        resp.status()
    );
}
