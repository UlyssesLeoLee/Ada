//! Error surface for the RBAC + collaboration module.
//!
//! [`RbacError`] is the single error type returned by every public
//! function in this crate. The v0.1.0 skeleton keeps the enum at
//! five variants covering the common failure modes of an in-process
//! RBAC + lock manager:
//!
//! | Variant            | Trigger                                                |
//! |--------------------|--------------------------------------------------------|
//! | `UnknownUser`      | The user has no role assigned on the target resource.    |
//! | `UnknownResource`  | The resource has no collaboration state.                |
//! | `AlreadyGranted`   | `grant` was called for an existing role assignment.     |
//! | `NotGranted`       | `revoke` was called for a missing role assignment.      |
//! | `LockHeld`         | A `read_lock` / `write_lock` attempt found a contention.  |
//! | `LockNotHeld`      | An `unlock` attempt found no matching lock.              |
//! | `InsufficientPermission` | The caller is missing a required permission.       |
//!
//! Production builds will map these to richer diagnostics and to
//! the `audit_log` table described in `DOC-MOD-011` §3.3.

use thiserror::Error;

/// Failure modes surfaced by the RBAC + collaboration module.
#[derive(Debug, Error)]
pub enum RbacError {
    /// The user has no role on the resource.
    #[error("unknown user: {0}")]
    UnknownUser(String),

    /// The resource has no collaboration state.
    #[error("unknown resource: {0}")]
    UnknownResource(String),

    /// `grant` was called for an existing role assignment.
    #[error("role already granted: user={user} resource={resource} role={role:?}")]
    AlreadyGranted {
        /// The user who already has the role.
        user: String,
        /// The resource the role was attached to.
        resource: String,
        /// The conflicting role.
        role: super::role::Role,
    },

    /// `revoke` was called for a missing role assignment.
    #[error("role not granted: user={user} resource={resource}")]
    NotGranted {
        /// The user that had no role on the resource.
        user: String,
        /// The resource the user was queried for.
        resource: String,
    },

    /// A `read_lock` / `write_lock` attempt found a contention.
    #[error("lock held: resource={resource} by={holder}")]
    LockHeld {
        /// Resource the lock attempt was on.
        resource: String,
        /// The current holder of the lock.
        holder: String,
    },

    /// An `unlock` attempt found no matching lock.
    #[error("lock not held: resource={resource} by={holder}")]
    LockNotHeld {
        /// Resource the lock attempt was on.
        resource: String,
        /// The alleged holder that did not actually hold the lock.
        holder: String,
    },

    /// The caller is missing a required permission.
    #[error("insufficient permission: need {need}")]
    InsufficientPermission {
        /// The permission that was required.
        need: super::permission::Permission,
    },
}

/// `Result` alias for fallible RBAC + collaboration operations.
pub type Result<T> = core::result::Result<T, RbacError>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::permission::{Action, Permission, ResourceType};

    #[test]
    fn display_strings_are_descriptive() {
        let e = RbacError::UnknownUser("alice".into());
        assert_eq!(e.to_string(), "unknown user: alice");
        let e = RbacError::UnknownResource("canvas-1".into());
        assert_eq!(e.to_string(), "unknown resource: canvas-1");
        let p = Permission::new(ResourceType::Canvas, Action::Write);
        let e = RbacError::InsufficientPermission { need: p };
        assert!(e.to_string().contains("insufficient permission"));
        assert!(e.to_string().contains("canvas"));
        assert!(e.to_string().contains("write"));
    }

    #[test]
    fn result_alias_carries_error() {
        let ok: Result<i32> = Ok(7);
        let err: Result<i32> = Err(RbacError::LockNotHeld {
            resource: "r".into(),
            holder: "u".into(),
        });
        assert!(matches!(ok, Ok(7)));
        assert!(matches!(err, Err(RbacError::LockNotHeld { .. })));
    }

    #[test]
    fn error_is_send_sync_and_static() {
        fn assert_send_sync_static<E: std::error::Error + Send + Sync + 'static>(_: &E) {}
        let e = RbacError::UnknownUser("x".into());
        assert_send_sync_static(&e);
    }
}
