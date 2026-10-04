//! Public `Enforcer` facade.
//!
//! The public API is preserved unchanged from v0.4.0; only the
//! internal evaluator moved. Which evaluator is compiled is a
//! two-way function of the `hand-rolled` feature — see the
//! `[features]` matrix in `Cargo.toml`:
//!
//! - feature on: the v0.4.0 hand-rolled evaluator behind
//!   [`crate::hand_rolled::HandRolledEnforcer`].
//! - feature off (the default): the real `casbin` 2.x adapter behind
//!   [`crate::casbin_impl::RealEnforcer`].
//!
//! There is no third configuration. A previous revision gated casbin
//! on `cfg(target_os = "linux" | "macos")` on the stated grounds that
//! it transitively needed `openssl-sys`, which is false; a target with
//! neither evaluator produced an `Enforcer` whose every constructor
//! returned a hard error. That left the production evaluator compilable
//! on one platform only, and in practice on none, because this
//! repository's Linux CI had not successfully started a job in 24
//! consecutive runs.

use std::sync::Arc;

use ada_m11_rbac_collab::{Action, CollaborationMap, ResourceType as M11ResourceType};

use crate::attrs::Attrs;
use crate::error::Result;
use crate::policy::PolicySet;

/// Public enforcer handle. Internally `Arc<...>` so it can be cheaply
/// cloned and shared between the api-gateway, the admin CLI, and the
/// watcher thread.
#[derive(Clone)]
pub struct Enforcer {
    inner: Arc<Inner>,
}

/// Implementation seam. One variant per compiled-in evaluator.
#[derive(Clone)]
enum Inner {
    #[cfg(not(feature = "hand-rolled"))]
    Casbin(crate::casbin_impl::RealEnforcer),
    #[cfg(feature = "hand-rolled")]
    HandRolled(crate::hand_rolled::HandRolledEnforcer),
}

impl std::fmt::Debug for Enforcer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Enforcer").finish_non_exhaustive()
    }
}

impl Enforcer {
    /// Build from a [`PolicySet`] (model + CSV).
    ///
    /// With the `hand-rolled` feature off this delegates to
    /// [`crate::casbin_impl::RealEnforcer::from_policy_set`]; with it
    /// on, to
    /// [`crate::hand_rolled::HandRolledEnforcer::from_policy_set`].
    pub fn from_policy_set(set: &PolicySet) -> Result<Self> {
        Ok(Self {
            inner: Arc::new(Inner::from_set(set)?),
        })
    }

    /// Import the m11 collaboration map. The m11 map is consulted for
    /// per-resource role lookups at enforce time.
    pub fn from_m11(set: &PolicySet, m11: &CollaborationMap) -> Result<Self> {
        Ok(Self {
            inner: Arc::new(Inner::from_m11(set, m11)?),
        })
    }

    /// Synchronous enforcement. The casbin implementation builds the
    /// `(sub, obj, act, tenant, is_owner_token)` tuple from the
    /// arguments and delegates to `casbin::Enforcer::enforce`.
    ///
    /// `object_id` is a `"<kind>:<id>"` composite; the resource type
    /// comes from that prefix. An object with no known prefix is
    /// denied — see [`crate::contract::resource_type_of`].
    ///
    /// The `user_id` is the caller's resolved role token (e.g.
    /// `"role:owner"`); a token that names no role is denied.
    pub fn enforce(
        &self,
        user_id: &str,
        object_id: &str,
        action: Action,
        attrs: &Attrs,
        m11: Option<&CollaborationMap>,
    ) -> Result<bool> {
        match &*self.inner {
            #[cfg(not(feature = "hand-rolled"))]
            Inner::Casbin(e) => e.enforce(user_id, object_id, action, attrs, m11),
            #[cfg(feature = "hand-rolled")]
            Inner::HandRolled(e) => e.enforce(user_id, object_id, action, attrs, m11),
        }
    }

    /// Typed variant — accepts m11's [`ResourceType`] enum directly
    /// so the caller does not have to format the object string.
    pub fn enforce_typed(
        &self,
        user_id: &str,
        object_kind: M11ResourceType,
        object_id: &str,
        action: Action,
        attrs: &Attrs,
        m11: Option<&CollaborationMap>,
    ) -> Result<bool> {
        match &*self.inner {
            #[cfg(not(feature = "hand-rolled"))]
            Inner::Casbin(e) => {
                e.enforce_typed(user_id, object_kind, object_id, action, attrs, m11)
            }
            #[cfg(feature = "hand-rolled")]
            Inner::HandRolled(e) => {
                e.enforce_typed(user_id, object_kind, object_id, action, attrs, m11)
            }
        }
    }

    /// The [`PolicySet`] this enforcer is bound to.
    #[must_use]
    pub fn policy_set(&self) -> &PolicySet {
        match &*self.inner {
            #[cfg(not(feature = "hand-rolled"))]
            Inner::Casbin(e) => e.policy_set(),
            #[cfg(feature = "hand-rolled")]
            Inner::HandRolled(e) => e.policy_set(),
        }
    }
}

impl Inner {
    fn from_set(set: &PolicySet) -> Result<Self> {
        #[cfg(not(feature = "hand-rolled"))]
        {
            Ok(Self::Casbin(
                crate::casbin_impl::RealEnforcer::from_policy_set(set)?,
            ))
        }
        #[cfg(feature = "hand-rolled")]
        {
            Ok(Self::HandRolled(
                crate::hand_rolled::HandRolledEnforcer::from_policy_set(set)?,
            ))
        }
    }

    fn from_m11(set: &PolicySet, m11: &CollaborationMap) -> Result<Self> {
        #[cfg(not(feature = "hand-rolled"))]
        {
            Ok(Self::Casbin(crate::casbin_impl::RealEnforcer::from_m11(
                set, m11,
            )?))
        }
        #[cfg(feature = "hand-rolled")]
        {
            Ok(Self::HandRolled(
                crate::hand_rolled::HandRolledEnforcer::from_m11(set, m11)?,
            ))
        }
    }
}
