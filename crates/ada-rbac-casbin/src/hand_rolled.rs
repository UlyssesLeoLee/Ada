//! Hand-rolled RBAC + ABAC enforcer.
//!
//! Compiled only when the `hand-rolled` Cargo feature is on (see the
//! `[features]` matrix in `Cargo.toml` for the other two
//! configurations). It exists so the crate is usable on targets that
//! cannot link `casbin` 2.x — notably Windows, where casbin's
//! transitive `openssl-sys` needs system OpenSSL headers.
//!
//! ## Authorization model
//!
//! Both decisions this evaluator makes are delegated to
//! `ada-m11-rbac-collab`, which is the single source of truth for the
//! role model. Nothing about the matrix is duplicated here:
//!
//! - **Who** the subject is: [`resolve_role`] maps the caller's
//!   subject token to exactly one [`Role`].
//! - **What** they may do: [`HandRolledEnforcer::check`] asks
//!   `role_permissions(role)` whether that role holds the specific
//!   `(resource_type, action)` [`Permission`]. The resource type is
//!   an explicit input, so a grant on one resource type never leaks
//!   onto another.
//!
//! Anything that cannot be resolved to a role, or whose object string
//! does not carry a known resource-type prefix, is denied. The
//! evaluator never falls back to a default role.

#![cfg(feature = "hand-rolled")]

use std::sync::Arc;

use ada_m11_rbac_collab::{
    role_permissions, Action, CollaborationMap, Permission, ResourceType, Role,
};

use crate::attrs::Attrs;
use crate::error::Result;
use crate::policy::PolicySet;

/// Hand-rolled evaluator. See the module docs for the model.
#[derive(Clone)]
pub struct HandRolledEnforcer {
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    set: PolicySet,
}

impl std::fmt::Debug for HandRolledEnforcer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HandRolledEnforcer").finish_non_exhaustive()
    }
}

impl HandRolledEnforcer {
    /// Build from a [`PolicySet`].
    pub fn from_policy_set(set: &PolicySet) -> Result<Self> {
        set.validate()?;
        Ok(Self {
            inner: Arc::new(Inner { set: set.clone() }),
        })
    }

    /// v0.4.0 entry point — identical to `from_policy_set`.
    ///
    /// The `CollaborationMap` is not consulted here: m11 keys its
    /// role grants by [`ada_m11_rbac_collab::ResourceId`] (a UUID),
    /// while this API receives opaque object strings such as
    /// `"canvas:abc"`, so there is no key to look the subject up by.
    /// Resolving one would mean inventing a mapping from object string
    /// to UUID, which this crate deliberately does not do.
    pub fn from_m11(set: &PolicySet, _m11: &CollaborationMap) -> Result<Self> {
        Self::from_policy_set(set)
    }

    /// Synchronous enforcement — role + `(resource type, action)`
    /// permission lookup, gated by the ABAC ownership check.
    ///
    /// `user_id` is the caller's resolved role token, e.g.
    /// `"role:owner"`. `object_id` carries the resource type as a
    /// `"<kind>:<id>"` prefix, e.g. `"canvas:abc"` — the composite
    /// form the v0.4.0 surface already used and the one
    /// [`Self::enforce_typed`] builds.
    pub fn enforce(
        &self,
        user_id: &str,
        object_id: &str,
        action: Action,
        attrs: &Attrs,
        m11: Option<&CollaborationMap>,
    ) -> Result<bool> {
        // Fail closed: an unresolvable subject or an object with no
        // known resource type is a deny, not a default role.
        let Some(role) = resolve_role(user_id, m11) else {
            return Ok(false);
        };
        let Some(resource_type) = resource_type_of(object_id) else {
            return Ok(false);
        };
        Ok(Self::check(role, resource_type, action, attrs))
    }

    /// Typed variant — same enforcement, m11 typed convenience.
    pub fn enforce_typed(
        &self,
        user_id: &str,
        object_kind: ResourceType,
        object_id: &str,
        action: Action,
        attrs: &Attrs,
        m11: Option<&CollaborationMap>,
    ) -> Result<bool> {
        let composite = format!("{}:{}", object_kind.as_str(), object_id);
        self.enforce(user_id, &composite, action, attrs, m11)
    }

    /// Return the bound [`PolicySet`].
    #[must_use]
    pub fn policy_set(&self) -> &PolicySet {
        &self.inner.set
    }

    /// Does `role` hold `action` on `resource_type`?
    ///
    /// The lookup is keyed on the exact `(role, resource_type)` pair
    /// via m11's `role_permissions`, so a role that holds `Write` on
    /// Canvas is not granted `Write` on Workspace or Credential.
    ///
    /// An associated function rather than a method: the answer depends
    /// only on the m11 matrix and the ABAC attributes, not on any state
    /// this evaluator holds.
    fn check(role: Role, resource_type: ResourceType, action: Action, attrs: &Attrs) -> bool {
        // ABAC gate: `Delete` always requires the ownership flag,
        // independent of what the role ladder grants.
        if matches!(action, Action::Delete) && !attrs.is_owner {
            return false;
        }
        role_permissions(role).contains(&Permission::new(resource_type, action))
    }
}

/// Resolve a subject token to the single role it names.
///
/// The contract is the role token `"role:<name>"` documented on
/// [`crate::enforcer::Enforcer::enforce`]; `Role::as_str` supplies the
/// canonical names. Returns `None` for anything else, which makes
/// [`HandRolledEnforcer::enforce`] deny. No default or fallback role
/// is applied — a subject that does not name a role has no grants.
fn resolve_role(subject: &str, _m11: Option<&CollaborationMap>) -> Option<Role> {
    let name = subject.strip_prefix("role:")?;
    [
        Role::Owner,
        Role::Admin,
        Role::Editor,
        Role::Executor,
        Role::Viewer,
    ]
    .into_iter()
    .find(|role| role.as_str() == name)
}

/// Extract the resource type from a `"<kind>:<id>"` object string.
///
/// `enforce_typed` builds this composite itself; the untyped
/// `enforce` receives it from the caller. Returns `None` when the
/// prefix is absent or names no known resource type, so an
/// unrecognised object is denied rather than checked against every
/// resource type in turn.
fn resource_type_of(object_id: &str) -> Option<ResourceType> {
    let (kind, _id) = object_id.split_once(':')?;
    [
        ResourceType::Canvas,
        ResourceType::Workspace,
        ResourceType::Credential,
    ]
    .into_iter()
    .find(|rt| rt.as_str() == kind)
}
