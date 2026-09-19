//! Public `Enforcer` facade.
//!
//! v0.5.0 swaps the internal evaluator to `casbin::Enforcer` 2.x; the
//! public API is preserved unchanged from v0.4.0. The
//! implementation is selected by the Cargo feature flag:
//!
//! - Default (feature off): real `casbin` 2.x adapter behind
//!   [`crate::casbin_impl::RealEnforcer`].
//! - Feature `hand-rolled`: the v0.4.0 hand-rolled evaluator behind
//!   [`crate::hand_rolled::HandRolledEnforcer`] (preserved for
//!   v0.4.0 callers that want to skip the `casbin` / `notify`
//!   runtime).

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

/// Implementation seam. The default branch wires the real casbin
/// adapter; the `hand-rolled` feature pins to the v0.4.0 evaluator.
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
    /// Build from a [`PolicySet`] (model + CSV). In v0.5.0 this
    /// delegates to [`crate::casbin_impl::RealEnforcer::from_policy_set`].
    pub fn from_policy_set(set: &PolicySet) -> Result<Self> {
        Ok(Self {
            inner: Arc::new(Inner::from_set(set)?),
        })
    }

    /// Import the m11 collaboration map. In v0.5.0 the m11 map is
    /// consulted only for per-resource role lookups at enforce time.
    pub fn from_m11(set: &PolicySet, m11: &CollaborationMap) -> Result<Self> {
        Ok(Self {
            inner: Arc::new(Inner::from_m11(set, m11)?),
        })
    }

    /// Synchronous enforcement. The casbin implementation builds the
    /// `(sub, obj, act, tenant, is_owner_token)` tuple from the
    /// arguments and delegates to `casbin::Enforcer::enforce`. The
    /// `user_id` is treated as the resolved role token (e.g.
    /// `"role:owner"`).
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
            Ok(Self::Casbin(crate::casbin_impl::RealEnforcer::from_policy_set(
                set,
            )?))
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