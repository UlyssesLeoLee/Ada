//! M-11: RBAC + collaboration. 5 roles, permission matrix, in-process
//! lock manager, audit_log interface (D-07).
//!
//! ## v0.1.0 scope (B3)
//!
//! This crate is a **minimum skeleton** for the cross-cutting
//! RBAC + collaboration concerns. The v0.1.0 surface is:
//!
//! - [`Role`] — five canonical roles (Owner / Admin / Editor /
//!   Executor / Viewer) with privilege-descending `Ord`
//! - [`Permission`] / [`ResourceType`] / [`Action`] — the
//!   (resource-type, action) pair
//! - [`role_permissions`] — static role → permission matrix
//! - [`Collaboration`] / [`CollaborationMap`] — per-resource user
//!   → role table, with `grant` / `revoke` / `set_role` /
//!   `authorize`
//! - [`LockManager`] — in-process per-resource read/write locks
//!   with `try_*` (non-blocking) and `*_lock` (await) variants
//! - [`AuditSink`] / [`InMemoryAuditSink`] /
//!   [`record_audit_log`] — pluggable audit logging interface
//! - 7-variant [`RbacError`] (UnknownUser, UnknownResource,
//!   AlreadyGranted, NotGranted, LockHeld, LockNotHeld,
//!   InsufficientPermission)
//! - 9 unit tests + 4 integration tests (`tests/integration.rs`)
//!
//! ## What v0.1.0 explicitly does **not** do
//!
//! - Persist audit entries (no `audit_log` table yet)
//! - Distribute locks across cluster nodes
//! - Back the collaboration map with Postgres
//! - Real CRDT collaboration (only the in-process lock manager
//!   is provided; the yrs/Yjs integration lives in the M-12
//!   frontend and the WebSocket relay in B4+)
//!
//! See `docs/modules/M-11-rbac-collab.md` (DOC-MOD-011) for the
//! full design.
//!
//! 関連 IPA フェーズ: 22-52 (基本設計/詳細設計), 53-58 (実装), 59-95 (試験)
//! 設計書: docs/modules/M-11-rbac-collab.md (DOC-MOD-011)

#![allow(missing_docs)]
#![allow(rust_2018_idioms)]
#![allow(clippy::doc_markdown)]

mod audit;
mod collaboration;
mod error;
mod lock;
mod permission;
mod role;

pub use ada_core::UserId;
pub use audit::{record_audit_log, AuditLogEntry, AuditSink, InMemoryAuditSink};
pub use collaboration::{Collaboration, CollaborationMap, ResourceId};
pub use error::{RbacError, Result};
pub use lock::{Lock, LockKind, LockManager};
pub use permission::{Action, Permission, ResourceType};
pub use role::{role_permissions, Role};

/// Crate version, taken from `CARGO_PKG_VERSION` (single workspace
/// version per D-09).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Crate name, taken from `CARGO_PKG_NAME`.
pub const NAME: &str = env!("CARGO_PKG_NAME");

/// `skeleton`-layer string tag (仿生モデル 4 層分類).
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
