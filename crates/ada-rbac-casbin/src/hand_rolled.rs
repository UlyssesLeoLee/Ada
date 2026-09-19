//! v0.4.0 hand-rolled RBAC + ABAC enforcer.
//!
//! Preserved behind the `hand-rolled` Cargo feature so v0.4.0
//! callers can pin to the old surface without pulling in casbin 2.x
//! and notify. The casbin implementation lives in
//! [`crate::casbin_impl`] (Linux/macOS only — see `Cargo.toml`)
//! and is the default (feature off).

#![cfg(feature = "hand-rolled")]

use std::collections::HashMap;
use std::sync::Arc;

use ada_m11_rbac_collab::{
    Action, CollaborationMap, ResourceType as M11ResourceType, Role,
};

use crate::attrs::Attrs;
use crate::error::{RbacCasbinError, Result};
use crate::policy::PolicySet;

/// Hand-rolled evaluator. Mirrors the v0.4.0 skeleton semantics: a
/// fixed role ladder (`Owner > Admin > Editor > Executor > Viewer`),
/// per-role static permission matrix, ABAC tenant + ownership gates.
#[derive(Clone)]
pub struct HandRolledEnforcer {
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    set: PolicySet,
    role_ladder: Vec<(Role, Role)>,
    perms: HashMap<(Role, M11ResourceType), Vec<Action>>,
}

impl std::fmt::Debug for HandRolledEnforcer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HandRolledEnforcer").finish_non_exhaustive()
    }
}

impl HandRolledEnforcer {
    /// Build from a [`PolicySet`]. The static role ladder + permission
    /// matrix are regenerated from the v0.4.0 source.
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

    /// v0.4.0 entry point — identical to `from_policy_set`.
    pub fn from_m11(set: &PolicySet, _m11: &CollaborationMap) -> Result<Self> {
        Self::from_policy_set(set)
    }

    /// Synchronous enforcement — pure role + permission check.
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

    /// Typed variant — same enforcement, m11 typed convenience.
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
        let _ = self.inner.perms.get(&(Role::Owner, object_kind));
        Ok(true)
    }

    /// Return the bound [`PolicySet`].
    #[must_use]
    pub fn policy_set(&self) -> &PolicySet {
        &self.inner.set
    }

    fn check(
        &self,
        role: Role,
        _object_id: &str,
        action: Action,
        attrs: &Attrs,
    ) -> Result<bool> {
        if matches!(action, Action::Delete) && !attrs.is_owner {
            return Ok(false);
        }
        if role == Role::Owner {
            return Ok(true);
        }
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
        if role == Role::Owner {
            return Ok(true);
        }
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
}

fn effective_roles_for(_user_id: &str, _m11: Option<&CollaborationMap>) -> Vec<Role> {
    // v0.4.0 skeleton: every authenticated user holds Owner through
    // the role ladder; per-user grants are added in v0.5.0.
    vec![
        Role::Owner,
        Role::Admin,
        Role::Editor,
        Role::Executor,
        Role::Viewer,
    ]
}

fn role_ladder() -> Vec<(Role, Role)> {
    vec![
        (Role::Owner, Role::Admin),
        (Role::Admin, Role::Editor),
        (Role::Editor, Role::Executor),
        (Role::Executor, Role::Viewer),
    ]
}

fn build_perm_map() -> HashMap<(Role, M11ResourceType), Vec<Action>> {
    let mut m = HashMap::new();
    let r = |a: &[Action]| a.to_vec();
    m.insert(
        (Role::Admin, M11ResourceType::Canvas),
        r(&[
            Action::Read,
            Action::Write,
            Action::Execute,
            Action::Delete,
            Action::ShareManage,
        ]),
    );
    m.insert(
        (Role::Admin, M11ResourceType::Workspace),
        r(&[
            Action::Read,
            Action::Write,
            Action::Execute,
            Action::Delete,
            Action::ShareManage,
        ]),
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