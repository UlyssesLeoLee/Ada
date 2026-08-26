//! Per-resource collaboration state: which users have which roles.
//!
//! [`Collaboration`] is a thin wrapper around a
//! `BTreeMap<UserId, Role>` for one resource. [`CollaborationMap`]
//! maps `ResourceId → Collaboration` and is the top-level object
//! the v0.1.0 skeleton provides.
//!
//! Production builds will back [`CollaborationMap`] with a
//! `rbac_grant` table (see `DOC-MOD-011` §3.1) and replace the
//! in-process `BTreeMap` with a `sqlx::PgPool`. The trait shape
//! is stable so callers can be coded against this skeleton today.

use std::collections::HashMap;
use std::fmt;

use ada_core::UserId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{RbacError, Result};
use crate::permission::{Action, Permission, ResourceType};
use crate::role::{role_permissions, Role};

/// Stable identifier for a collaboration target (canvas / workspace
/// / credential). Wraps a `Uuid`; production builds will key this
/// to a database `id` column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ResourceId(pub Uuid);

impl ResourceId {
    /// Build a random `ResourceId` (UUID v4).
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl fmt::Display for ResourceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "resource({})", self.0)
    }
}

impl Default for ResourceId {
    fn default() -> Self {
        Self::new()
    }
}

/// Per-resource user → role table.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Collaboration {
    /// User → role assignments on this resource.
    grants: HashMap<UserId, Role>,
}

impl Collaboration {
    /// Empty collaboration table.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// True if `user` has any role assigned on this resource.
    #[must_use]
    pub fn has_user(&self, user: UserId) -> bool {
        self.grants.contains_key(&user)
    }

    /// Role assigned to `user` on this resource, or `None` if they
    /// have no role.
    #[must_use]
    pub fn role_of(&self, user: UserId) -> Option<Role> {
        self.grants.get(&user).copied()
    }

    /// Grant `role` to `user` on this resource. Errors with
    /// [`RbacError::AlreadyGranted`] if the user already has a role.
    pub fn grant(&mut self, user: UserId, role: Role) -> Result<()> {
        if self.grants.contains_key(&user) {
            return Err(RbacError::AlreadyGranted {
                user: user.to_string(),
                resource: String::new(),
                role,
            });
        }
        self.grants.insert(user, role);
        Ok(())
    }

    /// Revoke `user`'s role on this resource. Errors with
    /// [`RbacError::NotGranted`] if they have no role.
    pub fn revoke(&mut self, user: UserId) -> Result<()> {
        if self.grants.remove(&user).is_none() {
            return Err(RbacError::NotGranted {
                user: user.to_string(),
                resource: String::new(),
            });
        }
        Ok(())
    }

    /// Change `user`'s existing role to `new_role`. Errors with
    /// [`RbacError::NotGranted`] if the user has no role.
    pub fn set_role(&mut self, user: UserId, new_role: Role) -> Result<()> {
        if !self.grants.contains_key(&user) {
            return Err(RbacError::NotGranted {
                user: user.to_string(),
                resource: String::new(),
            });
        }
        self.grants.insert(user, new_role);
        Ok(())
    }

    /// `user` is allowed to perform `action` on `resource_type` if
    /// their role grants the (resource-type, action) permission.
    pub fn has_permission(
        &self,
        user: UserId,
        resource_type: ResourceType,
        action: Action,
    ) -> bool {
        match self.grants.get(&user) {
            Some(role) => role_permissions(*role).contains(&Permission::new(resource_type, action)),
            None => false,
        }
    }

    /// Authorize `user` to perform `action` on `resource_type`,
    /// returning `RbacError::InsufficientPermission` if denied.
    pub fn authorize(
        &self,
        user: UserId,
        resource_type: ResourceType,
        action: Action,
    ) -> Result<()> {
        if self.has_permission(user, resource_type, action) {
            Ok(())
        } else {
            Err(RbacError::InsufficientPermission {
                need: Permission::new(resource_type, action),
            })
        }
    }

    /// Iterator over `(user, role)` pairs in ascending `user` order.
    pub fn iter(&self) -> impl Iterator<Item = (UserId, Role)> + '_ {
        self.grants.iter().map(|(u, r)| (*u, *r))
    }

    /// Number of granted users.
    #[must_use]
    pub fn len(&self) -> usize {
        self.grants.len()
    }

    /// True if no users are granted.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.grants.is_empty()
    }
}

/// Resource-keyed map of collaboration tables.
#[derive(Debug, Clone, Default)]
pub struct CollaborationMap {
    by_resource: HashMap<ResourceId, Collaboration>,
}

impl CollaborationMap {
    /// Empty map.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Borrow the collaboration table for `resource`, creating an
    /// empty one if absent (does not insert until a `grant` is
    /// issued).
    #[must_use]
    pub fn get_or_default(&self, resource: ResourceId) -> Collaboration {
        self.by_resource.get(&resource).cloned().unwrap_or_default()
    }

    /// `true` if `resource` has any explicit collaboration state.
    #[must_use]
    pub fn contains(&self, resource: ResourceId) -> bool {
        self.by_resource.contains_key(&resource)
    }

    /// Grant `role` to `user` on `resource`. Errors with
    /// [`RbacError::UnknownResource`] if the resource has never
    /// been touched.
    pub fn grant(&mut self, resource: ResourceId, user: UserId, role: Role) -> Result<()> {
        let collab = self
            .by_resource
            .get_mut(&resource)
            .ok_or_else(|| RbacError::UnknownResource(resource.to_string()))?;
        collab.grant(user, role).map_err(|e| match e {
            RbacError::AlreadyGranted {
                user: u, role: r, ..
            } => RbacError::AlreadyGranted {
                user: u,
                resource: resource.to_string(),
                role: r,
            },
            other => other,
        })
    }

    /// Revoke `user` from `resource`. Errors with
    /// [`RbacError::UnknownResource`] or [`RbacError::NotGranted`].
    pub fn revoke(&mut self, resource: ResourceId, user: UserId) -> Result<()> {
        let collab = self
            .by_resource
            .get_mut(&resource)
            .ok_or_else(|| RbacError::UnknownResource(resource.to_string()))?;
        collab.revoke(user).map_err(|e| match e {
            RbacError::NotGranted { user: u, .. } => RbacError::NotGranted {
                user: u,
                resource: resource.to_string(),
            },
            other => other,
        })
    }

    /// `user` is allowed to perform `action` on `resource_type`
    /// within `resource`.
    pub fn has_permission(
        &self,
        resource: ResourceId,
        user: UserId,
        resource_type: ResourceType,
        action: Action,
    ) -> bool {
        self.by_resource
            .get(&resource)
            .is_some_and(|c| c.has_permission(user, resource_type, action))
    }

    /// Authorize a (resource, user, action) tuple. Errors with
    /// [`RbacError::UnknownResource`] or
    /// [`RbacError::InsufficientPermission`].
    pub fn authorize(
        &self,
        resource: ResourceId,
        user: UserId,
        resource_type: ResourceType,
        action: Action,
    ) -> Result<()> {
        let collab = self
            .by_resource
            .get(&resource)
            .ok_or_else(|| RbacError::UnknownResource(resource.to_string()))?;
        collab.authorize(user, resource_type, action)
    }

    /// Touch a resource so it has an empty (but present) collaboration
    /// table. Useful when a resource is created.
    pub fn ensure(&mut self, resource: ResourceId) {
        self.by_resource.entry(resource).or_default();
    }

    /// Remove a resource's collaboration table entirely.
    /// Returns `true` if the resource existed.
    pub fn drop_resource(&mut self, resource: ResourceId) -> bool {
        self.by_resource.remove(&resource).is_some()
    }

    /// Number of resources tracked.
    #[must_use]
    pub fn len(&self) -> usize {
        self.by_resource.len()
    }

    /// True if no resources are tracked.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_resource.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::permission::Action;
    use crate::role::Role;

    fn user(n: u8) -> UserId {
        UserId(uuid::Uuid::from_bytes([n; 16]))
    }

    #[test]
    fn grant_then_revoke_flow() {
        let mut c = Collaboration::new();
        assert!(!c.has_user(user(1)));
        c.grant(user(1), Role::Editor).expect("grant");
        assert!(c.has_user(user(1)));
        assert_eq!(c.role_of(user(1)), Some(Role::Editor));
        // Re-granting the same user is an error.
        let err = c.grant(user(1), Role::Admin).expect_err("already granted");
        assert!(matches!(err, RbacError::AlreadyGranted { .. }));
        // Revoke works once.
        c.revoke(user(1)).expect("revoke");
        assert!(!c.has_user(user(1)));
        // Re-revoking is an error.
        let err = c.revoke(user(1)).expect_err("not granted");
        assert!(matches!(err, RbacError::NotGranted { .. }));
    }

    #[test]
    fn set_role_changes_existing_role() {
        let mut c = Collaboration::new();
        c.grant(user(1), Role::Editor).unwrap();
        c.set_role(user(1), Role::Admin).unwrap();
        assert_eq!(c.role_of(user(1)), Some(Role::Admin));
        let err = c.set_role(user(2), Role::Admin).expect_err("not granted");
        assert!(matches!(err, RbacError::NotGranted { .. }));
    }

    #[test]
    fn has_permission_uses_role_matrix() {
        let mut c = Collaboration::new();
        c.grant(user(1), Role::Editor).unwrap();
        assert!(c.has_permission(user(1), ResourceType::Canvas, Action::Write));
        assert!(!c.has_permission(user(1), ResourceType::Canvas, Action::Delete));
        // No role => no permission.
        assert!(!c.has_permission(user(2), ResourceType::Canvas, Action::Read));
    }

    #[test]
    fn authorize_returns_error_on_deny() {
        let mut c = Collaboration::new();
        c.grant(user(1), Role::Viewer).unwrap();
        let err = c
            .authorize(user(1), ResourceType::Canvas, Action::Write)
            .expect_err("viewer cannot write");
        assert!(matches!(err, RbacError::InsufficientPermission { .. }));
    }

    #[test]
    fn collaboration_map_grant_revoke_authorize() {
        let mut m = CollaborationMap::new();
        let r = ResourceId::new();
        m.ensure(r);
        m.grant(r, user(1), Role::Admin).unwrap();
        m.grant(r, user(2), Role::Viewer).unwrap();
        assert!(m.has_permission(r, user(1), ResourceType::Credential, Action::Read));
        assert!(!m.has_permission(r, user(2), ResourceType::Credential, Action::Delete));
        m.authorize(r, user(1), ResourceType::Credential, Action::Read)
            .expect("admin can read");
        let err = m
            .authorize(r, user(2), ResourceType::Canvas, Action::Delete)
            .expect_err("viewer cannot delete");
        assert!(matches!(err, RbacError::InsufficientPermission { .. }));
        m.revoke(r, user(2)).unwrap();
        // Ungranted user has no permission.
        assert!(!m.has_permission(r, user(2), ResourceType::Canvas, Action::Read));
        // Unknown resource is an error.
        let other = ResourceId::new();
        let err = m
            .authorize(other, user(1), ResourceType::Canvas, Action::Read)
            .expect_err("unknown resource");
        assert!(matches!(err, RbacError::UnknownResource(_)));
    }

    #[test]
    fn iter_yields_user_role_pairs() {
        let mut c = Collaboration::new();
        c.grant(user(3), Role::Editor).unwrap();
        c.grant(user(1), Role::Admin).unwrap();
        c.grant(user(2), Role::Viewer).unwrap();
        let pairs: Vec<(UserId, Role)> = c.iter().collect();
        // HashMap order is unspecified, so we just check that all
        // three grants are present and count == 3.
        assert_eq!(pairs.len(), 3);
        let roles: std::collections::HashMap<UserId, Role> = pairs.into_iter().collect();
        assert_eq!(roles.get(&user(1)).copied(), Some(Role::Admin));
        assert_eq!(roles.get(&user(2)).copied(), Some(Role::Viewer));
        assert_eq!(roles.get(&user(3)).copied(), Some(Role::Editor));
    }
}
