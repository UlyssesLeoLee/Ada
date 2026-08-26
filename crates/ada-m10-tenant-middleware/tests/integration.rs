//! Integration tests for the v0.1.0 tenant middleware.
//!
//! The v0.1.0 skeleton is in-process, so the "integration" tests
//! exercise the public surface the way a real HTTP handler would
//! use it: build a context, register it under a `RequestId`,
//! look it up from a "handler" task, and clear it at the end of
//! the request lifecycle.

use std::sync::Arc;

use ada_core::{TenantId, UserId};
use ada_m10_tenant_middleware::{
    InMemoryMiddleware, RequestId, TenantContext, TenantMiddleware, TenantResolver,
};
use uuid::Uuid;

fn tenant() -> TenantId {
    TenantId(Uuid::new_v4())
}

fn user() -> UserId {
    UserId(Uuid::new_v4())
}

#[tokio::test]
async fn end_to_end_request_lifecycle() {
    let mw = Arc::new(InMemoryMiddleware::new());

    // 1. Transport adapter parses headers and sets the context.
    let t = tenant();
    let u = user();
    let req = mw.set_context_for(t, Some(u));
    assert_eq!(mw.active_contexts().await, 1);

    // 2. Handler fetches the context and uses it for isolation.
    let mw_handler = Arc::clone(&mw);
    let req_handler = req;
    let ctx = mw_handler
        .get_tenant_context(req_handler)
        .await
        .expect("context present");
    assert_eq!(ctx.tenant_id, t);
    assert_eq!(ctx.user_id, Some(u));
    assert!(ctx.owns(t));
    assert!(!ctx.owns(tenant()));

    // 3. End of request: clear the entry.
    assert!(mw.clear_tenant_context(req).await);
    assert_eq!(mw.active_contexts().await, 0);
}

#[tokio::test]
async fn multiple_concurrent_requests_stay_isolated() {
    let mw = Arc::new(InMemoryMiddleware::new());

    // Spin up three requests for three different tenants. The
    // skeleton must keep their contexts disjoint.
    let t_a = tenant();
    let t_b = tenant();
    let t_c = tenant();
    let req_a = mw.set_context_for(t_a, Some(user()));
    let req_b = mw.set_context_for(t_b, Some(user()));
    let req_c = mw.set_context_for(t_c, Some(user()));
    assert_eq!(mw.active_contexts().await, 3);

    let ctx_a = mw.get_tenant_context(req_a).await.unwrap();
    let ctx_b = mw.get_tenant_context(req_b).await.unwrap();
    let ctx_c = mw.get_tenant_context(req_c).await.unwrap();
    assert_eq!(ctx_a.tenant_id, t_a);
    assert_eq!(ctx_b.tenant_id, t_b);
    assert_eq!(ctx_c.tenant_id, t_c);
    assert!(ctx_a.owns(t_a));
    assert!(!ctx_a.owns(t_b));
    assert!(!ctx_a.owns(t_c));
}

#[tokio::test]
async fn resolver_trait_returns_context_when_registered() {
    let mw = InMemoryMiddleware::new();
    let t = tenant();
    let u = user();
    let ctx = TenantContext::new(t, Some(u));
    let req = ctx.request_id;
    mw.set_tenant_context(req, ctx).await;
    // Use the resolver trait explicitly (not the concrete impl).
    let resolved: Option<TenantContext> = TenantResolver::resolve(&mw, req).await;
    let resolved = resolved.expect("some");
    assert_eq!(resolved.tenant_id, t);
    assert_eq!(resolved.user_id, Some(u));
}

#[tokio::test]
async fn clear_unknown_request_is_a_noop() {
    let mw = InMemoryMiddleware::new();
    assert!(!mw.clear_tenant_context(RequestId::new()).await);
    assert_eq!(mw.active_contexts().await, 0);
}
