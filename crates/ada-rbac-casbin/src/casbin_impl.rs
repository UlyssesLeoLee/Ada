//! v0.5.0 real casbin 2.x adapter.
//!
//! The production implementation behind the [`crate::Enforcer`]
//! facade. The synchronous public API bridges into casbin's async
//! `Enforcer::new` / `add_grouping_policy` via a dedicated tokio
//! runtime handle — see [`run_casbin_blocking`].
//!
//! The casbin evaluator is built around `policies/model.conf` (RBAC
//! matcher) and `policies/base_policy.csv` (rules). The role ladder
//! `Owner > Admin > Editor > Executor > Viewer` is wired via
//! `add_grouping_policy` in [`build_enforcer`].

use std::path::Path;
use std::sync::Arc;

use casbin::{CoreApi, DefaultModel, Enforcer as CasbinEnforcer, FileAdapter, MgmtApi};
use tokio::sync::Mutex;

use ada_m11_rbac_collab::{Action, CollaborationMap, ResourceType, Role};

use crate::attrs::Attrs;
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
/// hot reloads can swap it atomically.
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
    /// accepted the composite `object_id` ("canvas:<uuid>"); v0.5.0
    /// matches on the resource type only because the
    /// `policies/base_policy.csv` rows use resource-type tokens. The
    /// per-instance identifier (`<uuid>`) is forwarded by the
    /// api-gateway for tenant scoping but is not part of the
    /// authorization tuple.
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
        _object_id: &str,
        action: Action,
        attrs: &Attrs,
        m11: Option<&CollaborationMap>,
    ) -> Result<bool> {
        let _ = m11;
        self.enforce_internal(user_id, ResourceType::Canvas.as_str(), action, attrs)
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
        let guard = run_casbin_blocking(self.inner.lock());
        let is_owner_token = if attrs.is_owner { "true" } else { "false" };
        let tuple = (user_id, obj, action.as_str(), attrs.tenant_id.as_str(), is_owner_token);
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
fn build_enforcer(model_path: &Path, policy_path: &Path) -> Result<CasbinEnforcer> {
    let policy_str = policy_path
        .to_str()
        .ok_or_else(|| RbacCasbinError::ReloadFailed("policy path not utf-8".into()))?
        .to_owned();

    // Read the model file synchronously (file is small) before the
    // async bridge.
    let body = std::fs::read_to_string(model_path).map_err(|e| {
        RbacCasbinError::ReloadFailed(format!("model read {}: {e}", model_path.display()))
    })?;

    let mut enforcer: CasbinEnforcer = run_casbin_blocking(async move {
        let model = DefaultModel::from_str(&body).await.map_err(|e| {
            RbacCasbinError::ReloadFailed(format!("model parse: {e}"))
        })?;
        let adapter = FileAdapter::new(policy_str);
        CasbinEnforcer::new(model, adapter)
            .await
            .map_err(|e| RbacCasbinError::ReloadFailed(format!("casbin Enforcer::new: {e}")))
    })?;

    // `add_grouping_policy` takes `&mut self`. casbin's `Enforcer::new`
    // already returns an Enforcer with `auto_build_role_links` enabled
    // for the model.g lines that are loaded from the adapter; the role
    // ladder rows we add here are NOT in the CSV, so we must add them
    // explicitly.
    for (high, low) in ROLE_LADDER {
        let row = vec![
            format!("role:{}", high.as_str()),
            format!("role:{}", low.as_str()),
        ];
        run_casbin_blocking(async { enforcer.add_grouping_policy(row).await }).map_err(|e| {
            RbacCasbinError::ReloadFailed(format!(
                "add_grouping_policy(role:{}, role:{}): {e}",
                high.as_str(),
                low.as_str()
            ))
        })?;
    }
    Ok(enforcer)
}

/// Bridge async -> sync for the casbin handle.
///
/// We try to reuse a current tokio runtime handle when present (so
/// the api-gateway, which is async, can still call the sync
/// constructors without "runtime within runtime" panics). When
/// called from a synchronous context (tests, the admin CLI) we
/// build a one-shot runtime.
pub(crate) fn run_casbin_blocking<F, T>(future: F) -> T
where
    F: std::future::Future<Output = T>,
{
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => handle.block_on(future),
        Err(_) => tokio::runtime::Runtime::new()
            .expect("tokio runtime")
            .block_on(future),
    }
}