//! Hot-reload: in v0.4.0 the `HotReload` wrapper holds an
//! `Arc<RwLock<Enforcer>>` and exposes `reload_now()` for the
//! admin endpoint. `spawn_watcher` is a stub that returns
//! `Err(ReloadFailed)` — the real `notify` watcher lands in
//! v0.5.0 alongside the casbin adapter.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use parking_lot::RwLock;

use crate::enforcer::Enforcer;
use crate::error::{RbacCasbinError, Result};
use crate::policy::PolicySet;

pub struct HotReload {
    cell: Arc<RwLock<Enforcer>>,
    policy_path: PathBuf,
}

impl std::fmt::Debug for HotReload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HotReload")
            .field("policy_path", &self.policy_path)
            .finish_non_exhaustive()
    }
}

impl HotReload {
    pub fn new(set: &PolicySet) -> Result<Self> {
        let enforcer = Enforcer::from_policy_set(set)?;
        Ok(Self {
            cell: Arc::new(RwLock::new(enforcer)),
            policy_path: set.policy_path.clone(),
        })
    }

    #[must_use]
    pub fn current(&self) -> Enforcer {
        self.cell.read().clone()
    }

    pub fn reload_now(&self) -> Result<()> {
        let set = PolicySet {
            model_path: PathBuf::from("crates/ada-rbac-casbin/policies/model.conf"),
            policy_path: self.policy_path.clone(),
            overlays: Vec::new(),
        };
        let next = Enforcer::from_policy_set(&set)?;
        let mut w = self.cell.write();
        *w = next;
        Ok(())
    }

    /// Stub. The real `notify` watcher lands in v0.5.0 when the
    /// casbin adapter lands.
    pub fn spawn_watcher(&self) -> Result<()> {
        Err(RbacCasbinError::ReloadFailed(
            "watcher deferred to v0.5.0 (casbin adapter not yet wired)".into(),
        ))
    }

    #[must_use]
    pub fn policy_path(&self) -> &Path {
        &self.policy_path
    }
}