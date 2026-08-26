//! Permission model for the RBAC module.
//!
//! A [`Permission`] is a (resource-type, action) pair per
//! `DOC-MOD-011` §3.1. The role → permission matrix is a static
//! mapping in [`role::role_permissions`] so that the per-check
//! cost is O(1) hash lookup.
//!
//! ## Resource types
//!
//! The v0.1.0 skeleton covers the three resource types named in
//! the design doc: Canvas, Workspace, Credential. New resource
//! types should be added by extending the [`ResourceType`] enum
//! and the `role_permissions()` matrix together.

use std::fmt;

use serde::{Deserialize, Serialize};

/// The kind of resource a permission talks about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ResourceType {
    /// A canvas (top-level editing surface).
    Canvas,
    /// A workspace (folder / project container).
    Workspace,
    /// A credential (API key, OAuth token, ...).
    Credential,
}

impl ResourceType {
    /// Short, lowercase string tag for logs and JSON.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Canvas => "canvas",
            Self::Workspace => "workspace",
            Self::Credential => "credential",
        }
    }
}

/// An action a caller wants to perform on a resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Action {
    /// Read-only access (GET).
    Read,
    /// Mutate the resource (POST / PATCH).
    Write,
    /// Trigger the resource's execution (canvas run).
    Execute,
    /// Delete the resource.
    Delete,
    /// Manage sharing / role grants for the resource.
    ShareManage,
}

impl Action {
    /// Short, lowercase string tag for logs and JSON.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::Execute => "execute",
            Self::Delete => "delete",
            Self::ShareManage => "share_manage",
        }
    }
}

/// A permission is a (resource-type, action) pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Permission {
    /// Which resource type.
    pub resource_type: ResourceType,
    /// Which action.
    pub action: Action,
}

impl Permission {
    /// Build a new permission.
    #[must_use]
    pub const fn new(resource_type: ResourceType, action: Action) -> Self {
        Self {
            resource_type,
            action,
        }
    }
}

impl fmt::Display for Permission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}.{}",
            self.resource_type.as_str(),
            self.action.as_str()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_type_as_str() {
        assert_eq!(ResourceType::Canvas.as_str(), "canvas");
        assert_eq!(ResourceType::Workspace.as_str(), "workspace");
        assert_eq!(ResourceType::Credential.as_str(), "credential");
    }

    #[test]
    fn action_as_str() {
        assert_eq!(Action::Read.as_str(), "read");
        assert_eq!(Action::Write.as_str(), "write");
        assert_eq!(Action::Execute.as_str(), "execute");
        assert_eq!(Action::Delete.as_str(), "delete");
        assert_eq!(Action::ShareManage.as_str(), "share_manage");
    }

    #[test]
    fn permission_construct_and_eq() {
        let p1 = Permission::new(ResourceType::Canvas, Action::Read);
        let p2 = Permission::new(ResourceType::Canvas, Action::Read);
        let p3 = Permission::new(ResourceType::Canvas, Action::Write);
        assert_eq!(p1, p2);
        assert_ne!(p1, p3);
    }

    #[test]
    fn permission_hash_eq_works_in_set() {
        use std::collections::HashSet;
        let mut s: HashSet<Permission> = HashSet::new();
        s.insert(Permission::new(ResourceType::Canvas, Action::Read));
        s.insert(Permission::new(ResourceType::Canvas, Action::Read)); // dup
        s.insert(Permission::new(ResourceType::Credential, Action::Delete));
        assert_eq!(s.len(), 2);
    }
}
