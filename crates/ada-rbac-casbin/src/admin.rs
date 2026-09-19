//! Admin API surface.
//!
//! The api-gateway exposes `POST /admin/policies` which calls
//! [`AdminApi::add_policy`] / [`remove_policy`]. The api-gateway
//! is responsible for enforcing that the caller is `Role::Owner`
//! AND has cleared step-up MFA *before* dispatching here.

use std::sync::Arc;

use crate::enforcer::Enforcer;
use crate::error::Result;

pub struct AdminApi {
    enforcer: Enforcer,
    /// Per-tenant overrides applied in addition to the static
    /// base policy. v0.5.0 will back this with Postgres.
    overrides: Arc<parking_lot::RwLock<Vec<PolicyOverride>>>,
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
        f.debug_struct("AdminApi").finish_non_exhaustive()
    }
}

impl AdminApi {
    #[must_use]
    pub fn new(enforcer: Enforcer) -> Self {
        Self {
            enforcer,
            overrides: Arc::new(parking_lot::RwLock::new(Vec::new())),
        }
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