//! Error surface for the tenant middleware.
//!
//! [`TenantError`] is the single error type returned by every
//! public function in this crate. The v0.1.0 skeleton keeps the
//! enum at five variants covering the common failure modes of an
//! in-process tenant-context middleware:
//!
//! | Variant               | Trigger                                                  |
//! |-----------------------|----------------------------------------------------------|
//! | `MissingContext`      | The resolver returned no context for the given request. |
//! | `InvalidTenant`       | The tenant id was nil / malformed / unknown.             |
//! | `CrossTenantAccess`   | The active context's tenant_id differs from the target.  |
//! | `ContextNotInitialized` | A read attempted before `set_tenant_context`.          |
//! | `BackendError`        | The backing store (Postgres, Redis, ...) failed.        |
//!
//! Production builds will map these to the canonical API error
//! codes defined in `docs/api/error-codes.md`; the skeleton keeps
//! the surface minimal. See
//! [`DOC-MOD-010`](../docs/modules/M-10-tenant-middleware.md) §3.4
//! for the full validation pipeline.

use thiserror::Error;

/// Failure modes surfaced by the tenant middleware.
#[derive(Debug, Error)]
pub enum TenantError {
    /// The resolver could not find a context for the given request
    /// (e.g. missing `X-Tenant-Id` header on an HTTP request).
    #[error("missing tenant context: {0}")]
    MissingContext(String),

    /// The supplied tenant id is not a valid / known tenant
    /// (e.g. nil UUID or absent from the `tenants` table).
    #[error("invalid tenant: {0}")]
    InvalidTenant(String),

    /// The active context's `tenant_id` does not match the tenant
    /// id of the resource being accessed. This is the canonical
    /// multi-tenant isolation violation and MUST be audited
    /// (see `ada-m11-rbac-collab::AuditSink`).
    #[error("cross-tenant access denied: active={active}, target={target}")]
    CrossTenantAccess {
        /// Tenant id on the active context.
        active: String,
        /// Tenant id on the target resource.
        target: String,
    },

    /// A `get_tenant_context` call was made without a preceding
    /// `set_tenant_context` for the same scope. The skeleton uses
    /// this in single-thread / unit-test scenarios where the
    /// context map is empty.
    #[error("tenant context not initialized")]
    ContextNotInitialized,

    /// The backing store failed (Postgres unreachable, RLS policy
    /// denied the row, etc.).
    #[error("backend error: {0}")]
    BackendError(String),
}

/// `Result` alias for fallible tenant-middleware operations.
pub type Result<T> = core::result::Result<T, TenantError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_context_display() {
        let e = TenantError::MissingContext("no X-Tenant-Id".into());
        assert_eq!(e.to_string(), "missing tenant context: no X-Tenant-Id");
    }

    #[test]
    fn invalid_tenant_display() {
        let e = TenantError::InvalidTenant("00000000-0000-0000-0000-000000000000".into());
        assert_eq!(
            e.to_string(),
            "invalid tenant: 00000000-0000-0000-0000-000000000000"
        );
    }

    #[test]
    fn cross_tenant_access_display() {
        let e = TenantError::CrossTenantAccess {
            active: "tenant(a)".into(),
            target: "tenant(b)".into(),
        };
        assert_eq!(
            e.to_string(),
            "cross-tenant access denied: active=tenant(a), target=tenant(b)"
        );
    }

    #[test]
    fn context_not_initialized_display() {
        let e = TenantError::ContextNotInitialized;
        assert_eq!(e.to_string(), "tenant context not initialized");
    }

    #[test]
    fn backend_error_display() {
        let e = TenantError::BackendError("pg: connection refused".into());
        assert_eq!(e.to_string(), "backend error: pg: connection refused");
    }

    #[test]
    fn result_alias_carries_error() {
        let ok: Result<i32> = Ok(7);
        let err: Result<i32> = Err(TenantError::ContextNotInitialized);
        assert!(matches!(ok, Ok(7)));
        assert!(err.is_err());
    }

    #[test]
    fn error_is_send_sync_and_static() {
        fn assert_send_sync_static<E: std::error::Error + Send + Sync + 'static>(_: &E) {}
        let e = TenantError::BackendError("x".into());
        assert_send_sync_static(&e);
    }
}
