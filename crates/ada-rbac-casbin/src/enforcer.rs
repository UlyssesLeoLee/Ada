//! Public `Enforcer` facade.
//!
//! The public API is preserved unchanged from v0.4.0; only the
//! internal evaluator moved. Which evaluator is compiled is a
//! three-way function of the `hand-rolled` feature and the build
//! target — see the `[features]` matrix in `Cargo.toml`:
//!
//! - `hand-rolled` on any target: the v0.4.0 hand-rolled evaluator
//!   behind [`crate::hand_rolled::HandRolledEnforcer`]. This is the
//!   only way to get an evaluator on Windows.
//! - Feature off, Linux/macOS: the real `casbin` 2.x adapter behind
//!   [`crate::casbin_impl::RealEnforcer`].
//! - Feature off, any other target: no evaluator is compiled.
//!   [`Enforcer::from_policy_set`] and [`Enforcer::from_m11`] return
//!   [`RbacCasbinError::UnsupportedEvaluator`] — a hard error, never a
//!   permissive fallback.

use std::sync::Arc;

use ada_m11_rbac_collab::{Action, CollaborationMap, ResourceType as M11ResourceType};

use crate::attrs::Attrs;
use crate::error::Result;
// Only the unsupported configuration names the error type; on every
// other configuration this import would be unused.
#[cfg(all(
    not(feature = "hand-rolled"),
    not(any(target_os = "linux", target_os = "macos"))
))]
use crate::error::RbacCasbinError;
use crate::policy::PolicySet;

/// Public enforcer handle. Internally `Arc<...>` so it can be cheaply
/// cloned and shared between the api-gateway, the admin CLI, and the
/// watcher thread.
#[derive(Clone)]
pub struct Enforcer {
    inner: Arc<Inner>,
}

/// Implementation seam. One variant per compiled-in evaluator.
///
/// The unsupported configuration deliberately has no variant: an
/// `Enforcer` cannot be built at all there, so there is nothing for
/// the accessors below to dispatch to. Their matches are empty on that
/// configuration, which Rust accepts as exhaustive.
#[derive(Clone)]
enum Inner {
    #[cfg(all(
        not(feature = "hand-rolled"),
        any(target_os = "linux", target_os = "macos")
    ))]
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
    /// On Linux/macOS with the `hand-rolled` feature off this
    /// delegates to [`crate::casbin_impl::RealEnforcer::from_policy_set`].
    /// With `hand-rolled` on, to
    /// [`crate::hand_rolled::HandRolledEnforcer::from_policy_set`].
    /// With the feature off on any other target it returns
    /// [`RbacCasbinError::UnsupportedEvaluator`].
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
            #[cfg(all(
                not(feature = "hand-rolled"),
                any(target_os = "linux", target_os = "macos")
            ))]
            Inner::Casbin(e) => e.enforce(user_id, object_id, action, attrs, m11),
            #[cfg(feature = "hand-rolled")]
            Inner::HandRolled(e) => e.enforce(user_id, object_id, action, attrs, m11),
            // Unreachable: the constructors refuse on this
            // configuration, so no `Enforcer` value can exist. Returned
            // as the same hard error anyway, so that even a
            // hand-constructed handle yields an error instead of a
            // verdict.
            #[cfg(all(
                not(feature = "hand-rolled"),
                not(any(target_os = "linux", target_os = "macos"))
            ))]
            _ => {
                let _ = (user_id, object_id, action, attrs, m11);
                Err(unsupported())
            }
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
            #[cfg(all(
                not(feature = "hand-rolled"),
                any(target_os = "linux", target_os = "macos")
            ))]
            Inner::Casbin(e) => {
                e.enforce_typed(user_id, object_kind, object_id, action, attrs, m11)
            }
            #[cfg(feature = "hand-rolled")]
            Inner::HandRolled(e) => {
                e.enforce_typed(user_id, object_kind, object_id, action, attrs, m11)
            }
            // Unreachable — see the note on `Self::enforce`.
            #[cfg(all(
                not(feature = "hand-rolled"),
                not(any(target_os = "linux", target_os = "macos"))
            ))]
            _ => {
                let _ = (user_id, object_kind, object_id, action, attrs, m11);
                Err(unsupported())
            }
        }
    }

    /// The [`PolicySet`] this enforcer is bound to.
    #[must_use]
    pub fn policy_set(&self) -> &PolicySet {
        match &*self.inner {
            #[cfg(all(
                not(feature = "hand-rolled"),
                any(target_os = "linux", target_os = "macos")
            ))]
            Inner::Casbin(e) => e.policy_set(),
            #[cfg(feature = "hand-rolled")]
            Inner::HandRolled(e) => e.policy_set(),
            // Unreachable — see the note on `Self::enforce`. Unlike the
            // two enforce methods this one cannot report an error, so
            // it documents the unreachability instead of inventing a
            // `PolicySet` to hand back.
            #[cfg(all(
                not(feature = "hand-rolled"),
                not(any(target_os = "linux", target_os = "macos"))
            ))]
            _ => unreachable!("no Enforcer can be constructed on an unsupported build"),
        }
    }
}

impl Inner {
    fn from_set(set: &PolicySet) -> Result<Self> {
        #[cfg(all(
            not(feature = "hand-rolled"),
            any(target_os = "linux", target_os = "macos")
        ))]
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
        #[cfg(all(
            not(feature = "hand-rolled"),
            not(any(target_os = "linux", target_os = "macos"))
        ))]
        {
            let _ = set;
            Err(unsupported())
        }
    }

    fn from_m11(set: &PolicySet, m11: &CollaborationMap) -> Result<Self> {
        #[cfg(all(
            not(feature = "hand-rolled"),
            any(target_os = "linux", target_os = "macos")
        ))]
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
        #[cfg(all(
            not(feature = "hand-rolled"),
            not(any(target_os = "linux", target_os = "macos"))
        ))]
        {
            let _ = (set, m11);
            Err(unsupported())
        }
    }
}

/// The hard error returned by both constructors on a build that has
/// no evaluator compiled in.
///
/// It names the fix rather than degrading to a permissive evaluator:
/// a build that cannot authorize correctly must not authorize at all.
#[cfg(all(
    not(feature = "hand-rolled"),
    not(any(target_os = "linux", target_os = "macos"))
))]
fn unsupported() -> RbacCasbinError {
    RbacCasbinError::UnsupportedEvaluator {
        target: std::env::consts::OS.to_owned(),
    }
}
