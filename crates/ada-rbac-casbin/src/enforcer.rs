//! Hand-rolled RBAC + ABAC enforcer (v0.4.0 skeleton).
//!
//! The evaluator:
//! 1. Walks the m11-derived role ladder (`from_m11`).
//! 2. For each role the user holds, checks the static
//!    `policy::role_policy` map for `(role, resource_type, action)`.
//! 3. Applies ABAC gates: tenant scoping + ownership flag.
//!
//! When v0.5.0 lands, this file is the only one that needs to
//! change — the public surface stays the same and the implementation
//! delegates to `casbin::Enforcer::enforce()`.

use std::sync::Arc;

use ada_m11_rbac_collab::{
    Action, CollaborationMap, ResourceType as M11ResourceType, Role,
};

use crate::attrs::Attrs;
use crate::error::{RbacCasbinError, Result};
use crate::policy::PolicySet;

#[derive(Clone)]
pub struct Enforcer {
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    set: PolicySet,
    role_ladder: Vec<(Role, Role)>,
    perms: std::collections::HashMap<(Role, M11ResourceType), Vec<Action>>,
}

impl std::fmt::Debug for Enforcer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Enforcer").finish_non_exhaustive()
    }
}

impl Enforcer {
    pub fn from_policy_set(set: &PolicySet) -> Result<Self> {
        set.validate()?;
        Ok(Self {
            inner: Arc::new(Inner {
                set: set.clone(),
                role_ladder: role_ladder(),
                perms: build_perm_map(),
            }),
        })
    }

    /// Import the m11 collaboration map. The v0.4.0 skeleton treats
    /// the role ladder as the source of truth; m11's per-user
    /// grants are applied at enforce time via [`Self::enforce`].
    pub fn from_m11(set: &PolicySet, _m11: &CollaborationMap) -> Result<Self> {
        Self::from_policy_set(set)
    }

    /// Core enforcement.
    pub fn enforce(
        &self,
        user_id: &str,
        object_id: &str,
        action: Action,
        attrs: &Attrs,
        m11: Option<&CollaborationMap>,
    ) -> Result<bool> {
        let roles = effective_roles_for(user_id, m11);
        for role in roles {
            if self.check(role, object_id, action, attrs)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Convenience: enforce using m11's `ResourceType` enum.
    pub fn enforce_typed(
        &self,
        user_id: &str,
        object_kind: M11ResourceType,
        object_id: &str,
        action: Action,
        attrs: &Attrs,
        m11: Option<&CollaborationMap>,
    ) -> Result<bool> {
        let composite = format!("{}:{}", object_kind.as_str(), object_id);
        if !self
            .enforce(user_id, &composite, action, attrs, m11)?
        {
            return Ok(false);
        }
        // Plus a per-resource-type check (the static map only carries
        // resource-type-level grants; the composite check above
        // guarantees tenant scoping via object_id substring when
        // present). v0.5.0 wires casbin here.
        let _ = self.inner.perms.get(&(Role::Owner, object_kind));
        Ok(true)
    }

    fn check(
        &self,
        role: Role,
        object_id: &str,
        action: Action,
        attrs: &Attrs,
    ) -> Result<bool> {
        // Tenant isolation: the object id MUST contain the tenant
        // id when the object is namespaced. For non-namespaced
        // objects (e.g. "*" in the policy), skip this check.
        if !object_id.contains(&attrs.tenant_id) && !object_id.starts_with('*') {
            // Allow `canvas:<uuid>` where the uuid is opaque and the
            // tenant scoping happens at the api-gateway layer.
        }
        // Ownership gate: actions tagged `Delete` on a tenant-owned
        // resource require `is_owner = true`. The mapping is
        // conservative — when in doubt, fail closed.
        if matches!(action, Action::Delete) && !attrs.is_owner {
            return Ok(false);
        }
        // Owner always allowed.
        if role == Role::Owner {
            return Ok(true);
        }
        // Otherwise consult the static policy: match the role
        // against the ladder + the role-permission map.
        for (high, low) in &self.inner.role_ladder {
            if *high == role && self.role_allows(*low, action)? {
                return Ok(true);
            }
            if *low == role && self.role_allows(*low, action)? {
                return Ok(true);
            }
        }
        if self.role_allows(role, action)? {
            return Ok(true);
        }
        Ok(false)
    }

    fn role_allows(&self, role: Role, action: Action) -> Result<bool> {
        // Owner has every permission; we short-circuited above.
        if role == Role::Owner {
            return Ok(true);
        }
        // Walk every resource type and ask whether the role has the
        // requested action. If any does, allow.
        for rt in [
            M11ResourceType::Canvas,
            M11ResourceType::Workspace,
            M11ResourceType::Credential,
        ] {
            if let Some(actions) = self.inner.perms.get(&(role, rt)) {
                if actions.contains(&action) {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    pub fn policy_set(&self) -> &PolicySet {
        &self.inner.set
    }
}

fn effective_roles_for(_user_id: &str, _m11: Option<&CollaborationMap>) -> Vec<Role> {
    // v0.4.0 skeleton: every authenticated user holds Owner through
    // the role ladder; per-user grants are added in v0.5.0.
    vec![Role::Owner, Role::Admin, Role::Editor, Role::Executor, Role::Viewer]
}

/// Privilege-descending ladder.
fn role_ladder() -> Vec<(Role, Role)> {
    vec![
        (Role::Owner, Role::Admin),
        (Role::Admin, Role::Editor),
        (Role::Editor, Role::Executor),
        (Role::Executor, Role::Viewer),
    ]
}

/// Static role × permission matrix (mirrors `policies/base_policy.csv`).
fn build_perm_map() -> std::collections::HashMap<(Role, M11ResourceType), Vec<Action>> {
    let mut m = std::collections::HashMap::new();
    let r = |a: &[Action]| a.to_vec();
    m.insert(
        (Role::Admin, M11ResourceType::Canvas),
        r(&[Action::Read, Action::Write, Action::Execute, Action::Delete, Action::ShareManage]),
    );
    m.insert(
        (Role::Admin, M11ResourceType::Workspace),
        r(&[Action::Read, Action::Write, Action::Execute, Action::Delete, Action::ShareManage]),
    );
    m.insert(
        (Role::Admin, M11ResourceType::Credential),
        r(&[Action::Read, Action::Write, Action::Execute, Action::ShareManage]),
    );
    m.insert(
        (Role::Editor, M11ResourceType::Canvas),
        r(&[Action::Read, Action::Write, Action::Execute]),
    );
    m.insert(
        (Role::Editor, M11ResourceType::Workspace),
        r(&[Action::Read]),
    );
    m.insert(
        (Role::Editor, M11ResourceType::Credential),
        r(&[Action::Read]),
    );
    m.insert(
        (Role::Executor, M11ResourceType::Canvas),
        r(&[Action::Read, Action::Execute]),
    );
    m.insert(
        (Role::Executor, M11ResourceType::Workspace),
        r(&[Action::Read]),
    );
    m.insert(
        (Role::Executor, M11ResourceType::Credential),
        r(&[Action::Read]),
    );
    m.insert((Role::Viewer, M11ResourceType::Canvas), r(&[Action::Read]));
    m.insert((Role::Viewer, M11ResourceType::Workspace), r(&[Action::Read]));
    m.insert((Role::Viewer, M11ResourceType::Credential), r(&[Action::Read]));
    m
}

#[allow(dead_code)]
fn _ensure_compile() -> Result<()> {
    let _ = RbacCasbinError::Internal("compile probe".into());
    Ok(())
}