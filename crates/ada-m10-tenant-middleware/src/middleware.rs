//! In-process [`TenantMiddleware`] implementation.
//!
//! The v0.1.0 skeleton stores the active [`TenantContext`] in a
//! `parking_lot::RwLock<HashMap<RequestId, TenantContext>>` keyed
//! by `RequestId`. We use `parking_lot` rather than
//! `std::sync::RwLock` because the read path is hot and parking_lot
//! has measurably better contention behaviour (the lock guard
//! is *not* held across `.await`).
//!
//! ## Why not `tokio::task_local!`?
//!
//! `task_local` would give us implicit propagation through `.await`
//! boundaries, but it is restricted to a single task tree. The
//! production middleware will be invoked from many concurrent
//! tasks (HTTP worker, queue consumer, gRPC handler) and needs an
//! explicit, indexed store. The v0.1.0 skeleton picks the
//! explicit-map approach so the trait surface is the same shape
//! we will ship to production.
//!
//! See [`DOC-MOD-010`](../docs/modules/M-10-tenant-middleware.md)
//! §3.3 for the full lifecycle.

use std::collections::HashMap;

use async_trait::async_trait;
use parking_lot::RwLock;

use ada_core::{TenantId, UserId};

use crate::context::{RequestId, TenantContext, TenantResolver};
use crate::error::{Result, TenantError};

/// Trait implemented by every tenant middleware. The skeleton
/// surfaces three operations:
///
/// - [`set_tenant_context`](TenantMiddleware::set_tenant_context):
///   register a context for a given `RequestId` (called by the
///   transport adapter once the tenant claim is parsed).
/// - [`get_tenant_context`](TenantMiddleware::get_tenant_context):
///   fetch the context for a `RequestId` (called by downstream
///   handlers to enforce isolation).
/// - [`clear_tenant_context`](TenantMiddleware::clear_tenant_context):
///   drop the entry (called at the end of the request lifecycle
///   to bound the map size).
///
/// `TenantResolver` is implemented automatically by anything that
/// also implements [`TenantMiddleware`], so downstream code can
/// inject either an `Arc<dyn TenantMiddleware>` or an
/// `Arc<dyn TenantResolver>` depending on how much surface it
/// needs.
#[async_trait]
pub trait TenantMiddleware: Send + Sync {
    /// Register `context` for `request_id`. Overwrites any prior
    /// entry (this matters for retries that reuse the same
    /// `RequestId`; the latest claim wins).
    async fn set_tenant_context(&self, request_id: RequestId, context: TenantContext);

    /// Look up the context for `request_id`. Returns
    /// [`TenantError::ContextNotInitialized`] if no entry exists.
    async fn get_tenant_context(&self, request_id: RequestId) -> Result<TenantContext>;

    /// Drop the entry for `request_id`. Returns `true` if an
    /// entry was actually removed.
    async fn clear_tenant_context(&self, request_id: RequestId) -> bool;

    /// Number of live entries. Useful for tests and operational
    /// metrics.
    async fn active_contexts(&self) -> usize;
}

/// Default in-process middleware, backed by a
/// `parking_lot::RwLock<HashMap<RequestId, TenantContext>>`.
#[derive(Debug, Default)]
pub struct InMemoryMiddleware {
    inner: RwLock<HashMap<RequestId, TenantContext>>,
}

impl InMemoryMiddleware {
    /// Build an empty in-memory middleware.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Convenience helper: build a context for `(tenant, user)`
    /// and register it under a fresh `RequestId`. Returns the
    /// new `RequestId` so the caller can echo it back in
    /// response headers or traces.
    pub fn set_context_for(&self, tenant_id: TenantId, user_id: Option<UserId>) -> RequestId {
        let ctx = TenantContext::new(tenant_id, user_id);
        let req = ctx.request_id;
        self.inner.write().insert(req, ctx);
        req
    }
}

#[async_trait]
impl TenantMiddleware for InMemoryMiddleware {
    async fn set_tenant_context(&self, request_id: RequestId, context: TenantContext) {
        self.inner.write().insert(request_id, context);
    }

    async fn get_tenant_context(&self, request_id: RequestId) -> Result<TenantContext> {
        self.inner
            .read()
            .get(&request_id)
            .copied()
            .ok_or(TenantError::ContextNotInitialized)
    }

    async fn clear_tenant_context(&self, request_id: RequestId) -> bool {
        self.inner.write().remove(&request_id).is_some()
    }

    async fn active_contexts(&self) -> usize {
        self.inner.read().len()
    }
}

#[async_trait]
impl TenantResolver for InMemoryMiddleware {
    async fn resolve(&self, request_id: RequestId) -> Option<TenantContext> {
        self.inner.read().get(&request_id).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn tenant() -> TenantId {
        TenantId(Uuid::new_v4())
    }

    fn user() -> UserId {
        UserId(Uuid::new_v4())
    }

    #[tokio::test]
    async fn set_then_get_round_trip() {
        let mw = InMemoryMiddleware::new();
        let t = tenant();
        let req = mw.set_context_for(t, Some(user()));
        let got = mw.get_tenant_context(req).await.expect("context");
        assert_eq!(got.tenant_id, t);
        assert!(got.user_id.is_some());
    }

    #[tokio::test]
    async fn get_missing_context_returns_context_not_initialized() {
        let mw = InMemoryMiddleware::new();
        let err = mw
            .get_tenant_context(RequestId::new())
            .await
            .expect_err("missing");
        assert!(matches!(err, TenantError::ContextNotInitialized));
    }

    #[tokio::test]
    async fn clear_returns_true_then_false() {
        let mw = InMemoryMiddleware::new();
        let req = mw.set_context_for(tenant(), None);
        assert!(mw.clear_tenant_context(req).await);
        assert!(!mw.clear_tenant_context(req).await);
        assert!(matches!(
            mw.get_tenant_context(req).await.expect_err("gone"),
            TenantError::ContextNotInitialized
        ));
    }

    #[tokio::test]
    async fn active_contexts_counts_live_entries() {
        let mw = InMemoryMiddleware::new();
        assert_eq!(mw.active_contexts().await, 0);
        let a = mw.set_context_for(tenant(), None);
        let b = mw.set_context_for(tenant(), None);
        assert_eq!(mw.active_contexts().await, 2);
        mw.clear_tenant_context(a).await;
        assert_eq!(mw.active_contexts().await, 1);
        mw.clear_tenant_context(b).await;
        assert_eq!(mw.active_contexts().await, 0);
    }

    #[tokio::test]
    async fn set_overwrites_prior_context() {
        let mw = InMemoryMiddleware::new();
        let t1 = tenant();
        let t2 = tenant();
        let req = mw.set_context_for(t1, None);
        let ctx2 = TenantContext::new(t2, Some(user()));
        mw.set_tenant_context(req, ctx2).await;
        let got = mw.get_tenant_context(req).await.expect("ctx2");
        assert_eq!(got.tenant_id, t2);
    }

    #[tokio::test]
    async fn resolver_returns_some_for_known_request() {
        let mw = InMemoryMiddleware::new();
        let t = tenant();
        let req = mw.set_context_for(t, Some(user()));
        let resolved = TenantResolver::resolve(&mw, req).await;
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap().tenant_id, t);
    }

    #[tokio::test]
    async fn resolver_returns_none_for_unknown_request() {
        let mw = InMemoryMiddleware::new();
        let resolved = TenantResolver::resolve(&mw, RequestId::new()).await;
        assert!(resolved.is_none());
    }

    #[tokio::test]
    async fn set_context_for_assigns_distinct_request_ids() {
        let mw = InMemoryMiddleware::new();
        let a = mw.set_context_for(tenant(), None);
        let b = mw.set_context_for(tenant(), None);
        assert_ne!(a, b);
        assert_eq!(mw.active_contexts().await, 2);
    }
}
