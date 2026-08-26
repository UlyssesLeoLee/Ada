//! Per-request [`TenantContext`] and the [`TenantResolver`] trait.
//!
//! The v0.1.0 skeleton keeps the context type minimal: a triple of
//! `(tenant_id, user_id, request_id)`. Real builds will extend it
//! with `trace_id`, `span_id`, and the role bundle from
//! `ada-m11-rbac-collab` (see [`DOC-MOD-010`](../docs/modules/M-10-tenant-middleware.md)
//! §3.2 for the full schema).
//!
//! ## Why a trait and not a concrete type?
//!
//! A request arrives over a transport (HTTP, gRPC, internal queue).
//! Each transport has its own way of carrying the tenant claim:
//!
//! - HTTP: `X-Tenant-Id` header (or a JWT claim)
//! - gRPC: `tenant-id` metadata
//! - Queue: an envelope field
//!
//! The [`TenantResolver`] trait lets downstream code stay
//! transport-agnostic. Concrete resolvers (HTTP header parser,
//! JWT claim extractor, gRPC metadata reader) are wired in by
//! the transport adapters in B4+ and live outside the v0.1.0
//! skeleton.

use std::fmt;

use ada_core::{TenantId, UserId};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Per-request identifier. Distinct from any `tracing` span id;
/// used as the key into the in-memory middleware map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RequestId(pub Uuid);

impl RequestId {
    /// Generate a fresh `RequestId` (UUID v4).
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for RequestId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for RequestId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "request({})", self.0)
    }
}

impl From<Uuid> for RequestId {
    fn from(u: Uuid) -> Self {
        Self(u)
    }
}

/// The per-request tenant context. The `tenant_id` is the
/// canonical isolation key (NF-SEC【必須】) and MUST be present
/// in every authenticated request.
///
/// The skeleton keeps the type `Clone + PartialEq + Eq + Hash` so
/// the middleware can index a `HashMap<RequestId, TenantContext>`
/// without holding locks across awaits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TenantContext {
    /// Tenant scope.
    pub tenant_id: TenantId,
    /// Authenticated subject (or `None` for system jobs).
    pub user_id: Option<UserId>,
    /// Per-request correlation id.
    pub request_id: RequestId,
}

impl TenantContext {
    /// Build a new context with a freshly generated `RequestId`.
    #[must_use]
    pub fn new(tenant_id: TenantId, user_id: Option<UserId>) -> Self {
        Self {
            tenant_id,
            user_id,
            request_id: RequestId::new(),
        }
    }

    /// Build a new context with an explicit `RequestId`. Useful in
    /// tests where a deterministic id is convenient.
    #[must_use]
    pub fn with_request_id(
        tenant_id: TenantId,
        user_id: Option<UserId>,
        request_id: RequestId,
    ) -> Self {
        Self {
            tenant_id,
            user_id,
            request_id,
        }
    }

    /// True if `target` tenant matches this context's tenant. The
    /// middleware uses this as the cheap first-line isolation
    /// check; downstream stores add the RLS filter on top.
    #[must_use]
    pub fn owns(&self, target: TenantId) -> bool {
        self.tenant_id == target
    }
}

impl fmt::Display for TenantContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.user_id {
            Some(u) => write!(
                f,
                "TenantContext(tenant={}, user={}, req={})",
                self.tenant_id, u, self.request_id
            ),
            None => write!(
                f,
                "TenantContext(tenant={}, user=system, req={})",
                self.tenant_id, self.request_id
            ),
        }
    }
}

/// Transport-agnostic resolver. The middleware asks a resolver
/// for a [`TenantContext`] given a request id; the resolver
/// returns the context or a [`MissingContext`](crate::TenantError::MissingContext) error.
///
/// The v0.1.0 skeleton does **not** ship a concrete impl; that
/// is wired in by the HTTP / gRPC / queue adapters in B4+.
#[async_trait::async_trait]
pub trait TenantResolver: Send + Sync {
    /// Resolve the tenant context for `request_id`. Returns
    /// `None` if no context is registered for the request.
    async fn resolve(&self, request_id: RequestId) -> Option<TenantContext>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_id_is_unique() {
        let a = RequestId::new();
        let b = RequestId::new();
        assert_ne!(a, b);
        assert_eq!(RequestId::default().0.get_version_num(), 4);
    }

    #[test]
    fn request_id_display() {
        let r = RequestId(Uuid::nil());
        assert_eq!(
            r.to_string(),
            "request(00000000-0000-0000-0000-000000000000)"
        );
    }

    #[test]
    fn request_id_from_uuid() {
        let u = Uuid::new_v4();
        let r = RequestId::from(u);
        assert_eq!(r.0, u);
    }

    #[test]
    fn context_new_assigns_fresh_request_id() {
        let t = TenantId(Uuid::new_v4());
        let u = UserId(Uuid::new_v4());
        let ctx = TenantContext::new(t, Some(u));
        assert_eq!(ctx.tenant_id, t);
        assert_eq!(ctx.user_id, Some(u));
    }

    #[test]
    fn context_with_request_id_keeps_value() {
        let t = TenantId(Uuid::new_v4());
        let r = RequestId(Uuid::nil());
        let ctx = TenantContext::with_request_id(t, None, r);
        assert_eq!(ctx.request_id, r);
        assert!(ctx.user_id.is_none());
    }

    #[test]
    fn owns_returns_true_for_same_tenant() {
        let t = TenantId(Uuid::new_v4());
        let ctx = TenantContext::new(t, None);
        assert!(ctx.owns(t));
    }

    #[test]
    fn owns_returns_false_for_different_tenant() {
        let ctx = TenantContext::new(TenantId(Uuid::new_v4()), None);
        assert!(!ctx.owns(TenantId(Uuid::new_v4())));
    }

    #[test]
    fn display_includes_user_when_present() {
        let t = TenantId(Uuid::nil());
        let u = UserId(Uuid::nil());
        let ctx = TenantContext::new(t, Some(u));
        let s = ctx.to_string();
        assert!(s.contains("tenant(00000000-0000-0000-0000-000000000000)"));
        assert!(s.contains("user(00000000-0000-0000-0000-000000000000)"));
    }

    #[test]
    fn display_labels_system_when_user_absent() {
        let t = TenantId(Uuid::nil());
        let ctx = TenantContext::new(t, None);
        let s = ctx.to_string();
        assert!(s.contains("user=system"), "got: {s}");
    }

    #[test]
    fn serde_roundtrip() {
        let t = TenantId(Uuid::new_v4());
        let u = UserId(Uuid::new_v4());
        let ctx = TenantContext::new(t, Some(u));
        let json = serde_json::to_string(&ctx).expect("serialize");
        let back: TenantContext = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, ctx);
    }
}
