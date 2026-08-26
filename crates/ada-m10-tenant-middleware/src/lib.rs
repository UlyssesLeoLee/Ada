//! M-10: Tenant middleware. 11 tables, 6 PL/pgSQL procedures, RLS,
//! multi-tenant isolation. [NF-SEC] required.
//!
//! ## v0.1.0 scope (B4)
//!
//! This crate is a **minimum skeleton** for the multi-tenant
//! isolation layer defined in [`DOC-MOD-010`](../docs/modules/M-10-tenant-middleware.md).
//! The v0.1.0 surface is:
//!
//! - [`TenantContext`] — per-request `(tenant_id, user_id, request_id)`
//!   bundle that downstream handlers consult to enforce isolation.
//! - [`TenantResolver`] trait — abstracts *how* the request scope is
//!   recovered (HTTP header, JWT claim, gRPC metadata, ...).
//! - [`TenantMiddleware`] trait — the v0.1.0 surface that downstream
//!   crates program against. The default impl is
//!   [`InMemoryMiddleware`], a process-local `parking_lot::RwLock`
//!   keyed by `request_id`.
//! - [`InMemoryMiddleware`] — process-local middleware used in unit
//!   tests and single-process dev builds. v0.1.0 keeps it simple;
//!   production will swap to the SQLx/Postgres-backed
//!   `set_config('app.current_tenant', ...)` adapter once G4
//!   (実装着手判定) is approved.
//! - 5-variant [`TenantError`] (MissingContext, InvalidTenant,
//!   CrossTenantAccess, ContextNotInitialized, BackendError)
//! - 8 unit tests + 4 integration tests
//!
//! ## What v0.1.0 explicitly does **not** do
//!
//! - Persist context into the `tenant_context` table
//! - Inject `app.current_tenant` into a real PostgreSQL session
//! - Enforce RLS on the 11 tenant-scoped tables
//! - Implement row-level audit logging (see `ada-m11-rbac-collab`
//!   for the audit-sink surface)
//! - Honor distributed-trace context propagation (planned for B4+
//!   after the M-15 trait surface stabilises)
//!
//! See `docs/modules/M-10-tenant-middleware.md` (DOC-MOD-010) for
//! the full design.
//!
//! 関連 IPA フェーズ: 22-52 (基本設計/詳細設計), 53-58 (実装), 59-95 (試験)
//! 設計書: docs/modules/M-10-tenant-middleware.md (DOC-MOD-010)

#![allow(missing_docs)]
#![allow(rust_2018_idioms)]
#![allow(clippy::doc_markdown)]

mod context;
mod error;
mod middleware;

pub use context::{RequestId, TenantContext, TenantResolver};
pub use error::{Result, TenantError};
pub use middleware::{InMemoryMiddleware, TenantMiddleware};

/// Crate version, taken from `CARGO_PKG_VERSION` (single workspace
/// version per D-09).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Crate name, taken from `CARGO_PKG_NAME`.
pub const NAME: &str = env!("CARGO_PKG_NAME");

/// `skeleton`-layer string tag (仿生モデル 4 層分類, see
/// [`DOC-ARCH-001`](https://example.invalid/docs/architecture/00-anatomy-model.md)).
pub const LAYER: &str = "skeleton";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_not_empty() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn name_not_empty() {
        assert!(!NAME.is_empty());
    }

    #[test]
    fn layer_is_known() {
        assert!(
            ["skeleton", "blood", "nerve", "muscle", "shared"].contains(&LAYER),
            "Unknown layer: {LAYER}"
        );
    }
}
