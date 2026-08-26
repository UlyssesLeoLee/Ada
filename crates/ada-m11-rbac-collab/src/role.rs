//! Five-role RBAC model.
//!
//! Roles are ordered by privilege: `Owner > Admin > Editor >
//! Executor > Viewer`. The order is significant because the
//! `Collaboration::is_privileged_at_least` helper uses it for
//! escalation checks.
//!
//! The role → permission matrix is computed in [`role_permissions`]
//! from a static table, matching the design in `DOC-MOD-011` §3.1.
//! Real implementations will likely ship this as a build-time
//! constant; the v0.1.0 skeleton uses a `match` to keep the matrix
//! readable in a single file.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::permission::{Action, Permission, ResourceType};

/// The five canonical roles defined in `DOC-MOD-011` §3.1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Role {
    /// Tenant owner. Can manage billing and delete the tenant.
    Owner = 0,
    /// Can manage members, permissions, and integration settings.
    Admin = 1,
    /// Can edit a canvas.
    Editor = 2,
    /// Can trigger a canvas run, but cannot edit it.
    Executor = 3,
    /// Read-only access.
    Viewer = 4,
}

impl Role {
    /// Short, lowercase string tag for logs and JSON.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Admin => "admin",
            Self::Editor => "editor",
            Self::Executor => "executor",
            Self::Viewer => "viewer",
        }
    }
}

/// Compute the set of permissions granted to `role` across all
/// resource types. Matches the table in `DOC-MOD-011` §3.1.
///
/// Owner   — all (read / write / execute / delete / share-manage
///           on every resource type).
/// Admin   — all except `Delete` on Tenant (Owner-only).
/// Editor  — Read / Write / Execute on Canvas; Read on the rest.
/// Executor — Read / Execute on Canvas; Read on the rest.
/// Viewer  — Read on every resource type.
#[must_use]
pub fn role_permissions(role: Role) -> BTreeSet<Permission> {
    let mut out = BTreeSet::new();
    let mut add = |rt, action| {
        out.insert(Permission::new(rt, action));
    };
    match role {
        Role::Owner => {
            for &rt in &[
                ResourceType::Canvas,
                ResourceType::Workspace,
                ResourceType::Credential,
            ] {
                add(rt, Action::Read);
                add(rt, Action::Write);
                add(rt, Action::Execute);
                add(rt, Action::Delete);
                add(rt, Action::ShareManage);
            }
        }
        Role::Admin => {
            for &rt in &[
                ResourceType::Canvas,
                ResourceType::Workspace,
                ResourceType::Credential,
            ] {
                add(rt, Action::Read);
                add(rt, Action::Write);
                add(rt, Action::Execute);
                add(rt, Action::ShareManage);
            }
            // Admin can delete Canvas / Workspace but NOT Credential.
            add(ResourceType::Canvas, Action::Delete);
            add(ResourceType::Workspace, Action::Delete);
        }
        Role::Editor => {
            add(ResourceType::Canvas, Action::Read);
            add(ResourceType::Canvas, Action::Write);
            add(ResourceType::Canvas, Action::Execute);
            add(ResourceType::Workspace, Action::Read);
            add(ResourceType::Credential, Action::Read);
        }
        Role::Executor => {
            add(ResourceType::Canvas, Action::Read);
            add(ResourceType::Canvas, Action::Execute);
            add(ResourceType::Workspace, Action::Read);
            add(ResourceType::Credential, Action::Read);
        }
        Role::Viewer => {
            add(ResourceType::Canvas, Action::Read);
            add(ResourceType::Workspace, Action::Read);
            add(ResourceType::Credential, Action::Read);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owner_has_every_permission() {
        let perms = role_permissions(Role::Owner);
        for &rt in &[
            ResourceType::Canvas,
            ResourceType::Workspace,
            ResourceType::Credential,
        ] {
            for &action in &[
                Action::Read,
                Action::Write,
                Action::Execute,
                Action::Delete,
                Action::ShareManage,
            ] {
                assert!(
                    perms.contains(&Permission::new(rt, action)),
                    "owner missing {action:?} on {rt:?}"
                );
            }
        }
    }

    #[test]
    fn admin_cannot_delete_credential() {
        let perms = role_permissions(Role::Admin);
        assert!(perms.contains(&Permission::new(ResourceType::Canvas, Action::Delete)));
        assert!(perms.contains(&Permission::new(ResourceType::Workspace, Action::Delete)));
        assert!(!perms.contains(&Permission::new(ResourceType::Credential, Action::Delete)));
    }

    #[test]
    fn editor_can_write_canvas_but_not_delete() {
        let perms = role_permissions(Role::Editor);
        assert!(perms.contains(&Permission::new(ResourceType::Canvas, Action::Write)));
        assert!(!perms.contains(&Permission::new(ResourceType::Canvas, Action::Delete)));
        assert!(!perms.contains(&Permission::new(ResourceType::Canvas, Action::ShareManage)));
    }

    #[test]
    fn executor_can_run_but_not_write() {
        let perms = role_permissions(Role::Executor);
        assert!(perms.contains(&Permission::new(ResourceType::Canvas, Action::Execute)));
        assert!(!perms.contains(&Permission::new(ResourceType::Canvas, Action::Write)));
    }

    #[test]
    fn viewer_is_read_only() {
        let perms = role_permissions(Role::Viewer);
        for &action in &[
            Action::Write,
            Action::Execute,
            Action::Delete,
            Action::ShareManage,
        ] {
            assert!(
                !perms.contains(&Permission::new(ResourceType::Canvas, action)),
                "viewer should not have {action:?}"
            );
        }
        assert!(perms.contains(&Permission::new(ResourceType::Canvas, Action::Read)));
    }

    #[test]
    fn role_ordering_is_privilege_descending() {
        assert!(Role::Owner < Role::Admin);
        assert!(Role::Admin < Role::Editor);
        assert!(Role::Editor < Role::Executor);
        assert!(Role::Executor < Role::Viewer);
    }

    #[test]
    fn role_as_str() {
        assert_eq!(Role::Owner.as_str(), "owner");
        assert_eq!(Role::Admin.as_str(), "admin");
        assert_eq!(Role::Editor.as_str(), "editor");
        assert_eq!(Role::Executor.as_str(), "executor");
        assert_eq!(Role::Viewer.as_str(), "viewer");
    }

    #[test]
    fn role_permissions_is_deterministic() {
        // Calling role_permissions twice yields equal sets.
        assert_eq!(
            role_permissions(Role::Editor),
            role_permissions(Role::Editor)
        );
    }
}
