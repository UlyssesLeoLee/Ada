//! Admin API surface.
//!
//! The api-gateway exposes `POST /admin/policies` which calls
//! [`AdminApi::add_policy`] / [`remove_policy`]. The api-gateway
//! is responsible for enforcing that the caller is `Role::Owner`
//! AND has cleared step-up MFA *before* dispatching here.
//!
//! v0.5.0: per-tenant overrides are still held in memory but the
//! `persist` flag (default `false`) lets the api-gateway opt into
//! forwarding each mutation to a future Postgres sink without an
//! API change. v0.6.0 wires the Postgres adapter; the flag is the
//! seam. The mutable `bool` return shape is preserved from v0.4.0.

use std::sync::Arc;

use crate::enforcer::Enforcer;
use crate::error::Result;

pub struct AdminApi {
    /// The enforcer this admin API is bound to. Held by value so the
    /// api-gateway keeps the same handle the watcher swaps behind;
    /// v0.5.0 keeps the field for source compatibility with the
    /// v0.4.0 surface even though the override list is the only
    /// mutable state today.
    #[allow(dead_code)]
    enforcer: Enforcer,
    /// Per-tenant overrides applied in addition to the static
    /// base policy. v0.6.0 will back this with Postgres via the
    /// `persist` flag.
    overrides: Arc<parking_lot::RwLock<Vec<PolicyOverride>>>,
    /// When `true`, every `add_policy` / `remove_policy` call would
    /// forward the mutation to the durable sink. v0.5.0 keeps this
    /// `false` (in-memory only) so the api-gateway can ship today.
    persist: bool,
}

#[derive(Debug, Clone)]
pub struct PolicyOverride {
    pub sub: String,
    pub obj: String,
    pub act: String,
    pub tenant: String,
    pub is_owner: String,
}

impl std::fmt::Debug for AdminApi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdminApi")
            .field("persist", &self.persist)
            .finish_non_exhaustive()
    }
}

impl AdminApi {
    /// Default constructor — `persist = false`, in-memory overrides
    /// only. Behaviour is identical to v0.4.0.
    #[must_use]
    pub fn new(enforcer: Enforcer) -> Self {
        Self {
            enforcer,
            overrides: Arc::new(parking_lot::RwLock::new(Vec::new())),
            persist: false,
        }
    }

    /// Constructor with the persist flag explicit. v0.6.0 will set
    /// this to `true` once the Postgres adapter lands.
    #[must_use]
    pub fn with_persist(enforcer: Enforcer, persist: bool) -> Self {
        Self {
            enforcer,
            overrides: Arc::new(parking_lot::RwLock::new(Vec::new())),
            persist,
        }
    }

    /// Current value of the persist flag. v0.5.0 callers can read
    /// this to decide whether to forward the mutation elsewhere.
    #[must_use]
    pub fn persist(&self) -> bool {
        self.persist
    }

    pub fn add_policy(
        &self,
        sub: &str,
        obj: &str,
        act: &str,
        tenant: &str,
        is_owner: &str,
    ) -> Result<bool> {
        let o = PolicyOverride {
            sub: sub.into(),
            obj: obj.into(),
            act: act.into(),
            tenant: tenant.into(),
            is_owner: is_owner.into(),
        };
        let mut w = self.overrides.write();
        if w.iter().any(|p| p.sub == o.sub && p.obj == o.obj && p.act == o.act && p.tenant == o.tenant) {
            return Ok(false);
        }
        w.push(o);
        Ok(true)
    }

    pub fn remove_policy(
        &self,
        sub: &str,
        obj: &str,
        act: &str,
        tenant: &str,
        is_owner: &str,
    ) -> Result<bool> {
        let mut w = self.overrides.write();
        let before = w.len();
        w.retain(|p| !(p.sub == sub && p.obj == obj && p.act == act && p.tenant == tenant && p.is_owner == is_owner));
        Ok(w.len() < before)
    }

    pub fn add_grouping(&self, _user: &str, _role: &str) -> Result<bool> {
        // v0.4.0: per-user role grants go through m11.
        Ok(true)
    }

    pub fn remove_grouping(&self, _user: &str, _role: &str) -> Result<bool> {
        Ok(true)
    }

    pub fn overrides_count(&self) -> usize {
        self.overrides.read().len()
    }
}