//! v0.5.0 real casbin 2.x adapter.
//!
//! The production implementation behind the [`crate::Enforcer`]
//! facade.
//!
//! ## Why there is still a runtime bridge at all
//!
//! `casbin::Enforcer::enforce` is **synchronous** — it is only
//! `Enforcer::new` (which reads the model and the policy file through
//! the tokio-flavoured `FileAdapter`) and `add_grouping_policy` that are
//! `async`. So the hot authorization path needs no runtime whatsoever;
//! the bridge in [`run_casbin_blocking`] is used exactly twice per
//! enforcer, at construction.
//!
//! ## The defect this shape fixes
//!
//! `inner` used to be a `tokio::sync::Mutex`, so `enforce` had to
//! `block_on` the lock acquisition. `tokio::runtime::Handle::block_on`
//! **panics** when called from a thread that is already driving a
//! runtime — "Cannot start a runtime from within a runtime" — so every
//! authorization made from an async handler (which is all of them, in
//! the api-gateway) panicked, as did every construction from
//! `#[tokio::main]`. Nothing caught it: the unit and integration tests
//! for this crate all call the synchronous API from a synchronous test,
//! where `Handle::try_current()` fails and the old fallback built its
//! own one-shot runtime, so the panic branch was never executed. It
//! surfaced only when the gateway actually called in from `async fn`.
//!
//! `inner` is a `parking_lot::Mutex` now, so enforcement locks
//! synchronously and needs no runtime, and [`run_casbin_blocking`]
//! runs construction on a thread that owns a runtime of its own —
//! which works from a synchronous test, a `current_thread` runtime and
//! a multi-thread runtime alike.

use std::path::Path;
use std::sync::Arc;

use casbin::{CoreApi, DefaultModel, Enforcer as CasbinEnforcer, FileAdapter, MgmtApi};
use parking_lot::Mutex;

use ada_m11_rbac_collab::{Action, CollaborationMap, ResourceType, Role};

use crate::attrs::Attrs;
use crate::contract::{requires_ownership, resource_type_of};
use crate::error::{RbacCasbinError, Result};
use crate::policy::PolicySet;

/// Privilege-descending ladder mirroring `ada-m11-rbac-collab`'s role
/// graph. Higher rows inherit all permissions of lower rows.
const ROLE_LADDER: &[(Role, Role)] = &[
    (Role::Owner, Role::Admin),
    (Role::Admin, Role::Editor),
    (Role::Editor, Role::Executor),
    (Role::Executor, Role::Viewer),
];

/// Synchronous wrapper around [`CasbinEnforcer`].
///
/// `casbin::Enforcer::enforce` is synchronous; only construction is
/// async. We keep a single casbin handle behind `Arc<Mutex<...>>` so
/// hot reloads can swap it atomically. The mutex is `parking_lot`'s, not
/// `tokio::sync`'s: enforcement happens inside `async fn` request
/// handlers, and awaiting a tokio lock from a synchronous public API
/// would require `block_on`, which panics on a runtime thread.
#[derive(Clone)]
pub struct RealEnforcer {
    inner: Arc<Mutex<CasbinEnforcer>>,
    set: PolicySet,
}

impl std::fmt::Debug for RealEnforcer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RealEnforcer")
            .field("model_path", &self.set.model_path)
            .field("policy_path", &self.set.policy_path)
            .finish_non_exhaustive()
    }
}

impl RealEnforcer {
    /// Build an enforcer from a [`PolicySet`], applying the role
    /// ladder via `add_grouping_policy` so Owner inherits Viewer etc.
    pub fn from_policy_set(set: &PolicySet) -> Result<Self> {
        set.validate()?;
        let enforcer = build_enforcer(&set.model_path, &set.policy_path)?;
        Ok(Self {
            inner: Arc::new(Mutex::new(enforcer)),
            set: set.clone(),
        })
    }

    /// v0.4.0 entry point — identical behaviour to
    /// [`from_policy_set`](Self::from_policy_set) in v0.5.0. The
    /// `CollaborationMap` is consulted only when present in the
    /// request side via [`enforce_typed`](Self::enforce_typed); we
    /// keep the parameter for source compatibility.
    pub fn from_m11(set: &PolicySet, _m11: &CollaborationMap) -> Result<Self> {
        Self::from_policy_set(set)
    }

    /// Synchronous enforce entry point. The v0.4.0 hand-rolled surface
    /// accepted the composite `object_id` (`"canvas:<uuid>"`); the
    /// resource type is parsed from that prefix and matched against the
    /// `policies/base_policy.csv` rows, which are keyed by
    /// resource-type token. The per-instance identifier (`<uuid>`) is
    /// forwarded by the api-gateway for tenant scoping but is not part
    /// of the authorization tuple.
    ///
    /// This previously hardcoded `ResourceType::Canvas` and ignored
    /// `object_id` entirely, so a request for `"workspace:<uuid>"`
    /// was evaluated against the canvas policy rows. That let a caller
    /// reach canvas permissions through a workspace (or credential)
    /// object, and it also meant an object with no recognised prefix
    /// was authorized rather than denied — the exact behaviour
    /// `tests/integration.rs::object_without_a_known_resource_type_is_denied`
    /// forbids. Fail closed instead.
    ///
    /// The caller passes the resolved role token as `user_id`
    /// (e.g. `"role:owner"`); in production the api-gateway maps the
    /// authenticated user -> role token before dispatching here. The
    /// `add_grouping_policy` ladder wired in [`build_enforcer`]
    /// ensures inheritance works: a request with `r.sub = "role:owner"`
    /// satisfies any policy line whose `p.sub` is in its chain
    /// (`role:admin` / `role:editor` / etc.).
    pub fn enforce(
        &self,
        user_id: &str,
        object_id: &str,
        action: Action,
        attrs: &Attrs,
        m11: Option<&CollaborationMap>,
    ) -> Result<bool> {
        let _ = m11;
        // Fail closed: an object with no known resource type is a deny,
        // never a guess at which policy rows to check.
        let Some(resource_type) = resource_type_of(object_id) else {
            return Ok(false);
        };
        self.enforce_internal(user_id, resource_type.as_str(), action, attrs)
    }

    /// Typed variant — accepts m11's [`ResourceType`] directly so
    /// callers don't have to format the object string themselves.
    pub fn enforce_typed(
        &self,
        user_id: &str,
        object_kind: ResourceType,
        _object_id: &str,
        action: Action,
        attrs: &Attrs,
        m11: Option<&CollaborationMap>,
    ) -> Result<bool> {
        let _ = m11;
        self.enforce_internal(user_id, object_kind.as_str(), action, attrs)
    }

    fn enforce_internal(
        &self,
        user_id: &str,
        obj: &str,
        action: Action,
        attrs: &Attrs,
    ) -> Result<bool> {
        // ABAC gate: destructive actions require the ownership flag,
        // independent of what the role ladder grants. This is enforced
        // here in Rust rather than left to the matcher because
        // `policies/base_policy.csv` sets `is_owner = "*"` on every
        // row, which makes the matcher's
        // `(p.is_owner == r.is_owner || p.is_owner == "*")` clause true
        // unconditionally — so `role:owner` could delete an object it
        // did not own. The same gate is applied by the hand-rolled
        // evaluator; both call [`crate::contract::requires_ownership`]
        // so the two cannot drift apart again.
        if requires_ownership(action) && !attrs.is_owner {
            return Ok(false);
        }
        let guard = self.inner.lock();
        let is_owner_token = if attrs.is_owner { "true" } else { "false" };
        let tuple = (
            user_id,
            obj,
            action.as_str(),
            attrs.tenant_id.as_str(),
            is_owner_token,
        );
        guard
            .enforce(tuple)
            .map_err(|e| RbacCasbinError::Internal(format!("casbin enforce: {e}")))
    }

    /// Return the [`PolicySet`] backing this enforcer.
    #[must_use]
    pub fn policy_set(&self) -> &PolicySet {
        &self.set
    }
}

/// Build the casbin enforcer and wire in the role ladder via
/// `add_grouping_policy`. The ladder rows are
/// `g(role:owner, role:admin)`, `g(role:admin, role:editor)` etc.,
/// mirroring the m11 privilege-descending graph so a request with
/// `r.sub = "role:owner"` satisfies any policy line `p.sub = "*"` or
/// any role in its inheritance chain.
///
/// The whole construction is one future rather than one future per
/// `add_grouping_policy` call, so it crosses the runtime boundary once
/// instead of `1 + ROLE_LADDER.len()` times.
fn build_enforcer(model_path: &Path, policy_path: &Path) -> Result<CasbinEnforcer> {
    let policy_str = policy_path
        .to_str()
        .ok_or_else(|| RbacCasbinError::ReloadFailed("policy path not utf-8".into()))?
        .to_owned();

    // Read the model file synchronously (file is small) before handing
    // the work to the runtime-owning thread.
    let body = std::fs::read_to_string(model_path).map_err(|e| {
        RbacCasbinError::ReloadFailed(format!("model read {}: {e}", model_path.display()))
    })?;

    run_casbin_blocking(|| async move {
        let model = DefaultModel::from_str(&body)
            .await
            .map_err(|e| RbacCasbinError::ReloadFailed(format!("model parse: {e}")))?;
        let adapter = FileAdapter::new(policy_str);
        let mut enforcer = CasbinEnforcer::new(model, adapter)
            .await
            .map_err(|e| RbacCasbinError::ReloadFailed(format!("casbin Enforcer::new: {e}")))?;

        // `add_grouping_policy` takes `&mut self`. casbin's
        // `Enforcer::new` already returns an Enforcer with
        // `auto_build_role_links` enabled for the model.g lines loaded
        // from the adapter; the role ladder rows we add here are NOT in
        // the CSV, so we must add them explicitly.
        for (high, low) in ROLE_LADDER {
            let row = vec![
                format!("role:{}", high.as_str()),
                format!("role:{}", low.as_str()),
            ];
            enforcer.add_grouping_policy(row).await.map_err(|e| {
                RbacCasbinError::ReloadFailed(format!(
                    "add_grouping_policy(role:{}, role:{}): {e}",
                    high.as_str(),
                    low.as_str()
                ))
            })?;
        }
        Ok(enforcer)
    })
}

/// Run an async casbin operation to completion from a synchronous caller.
///
/// Takes a *factory* rather than a future so the future is constructed
/// on the thread that owns the runtime, not on the caller's.
///
/// ## Why a thread and not `Handle::block_on`
///
/// The previous implementation did this:
///
/// ```text
/// match Handle::try_current() {
///     Ok(h)  => h.block_on(future),          // panics on a runtime thread
///     Err(_) => Runtime::new().block_on(future),   // only correct outside a runtime
/// }
/// ```
///
/// Both arms are wrong for a caller that is already inside a runtime,
/// which is the api-gateway and nothing else: the first panics outright
/// ("Cannot start a runtime from within a runtime") and the second is
/// unreachable, because `try_current()` succeeds there. Spawning a
/// thread with a runtime of its own sidesteps the question — the
/// runtime is on a thread that was never a runtime worker.
///
/// `current_thread` is deliberate. The work is model parsing, a policy
/// file read and five in-memory grouping-policy inserts; there is
/// nothing here worth a work-stealing pool, and a current-thread
/// runtime cannot be re-entered from inside itself.
pub(crate) fn run_casbin_blocking<F, Fut, T>(make: F) -> T
where
    // `F` crosses the thread boundary; `Fut` does not, because it is
    // constructed on the far side by calling `make()` there. That is the
    // reason the parameter is a factory and not the future itself.
    F: FnOnce() -> Fut + Send,
    Fut: std::future::Future<Output = T>,
    T: Send,
{
    std::thread::scope(|scope| {
        scope
            .spawn(|| {
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("casbin worker runtime")
                    .block_on(make())
            })
            .join()
            .expect("casbin worker thread panicked")
    })
}
