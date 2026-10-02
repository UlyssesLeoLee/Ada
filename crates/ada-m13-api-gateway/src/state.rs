//! Application state shared by every gateway handler.
//!
//! [`AppState`] is cloned (cheaply, via `Arc`) into each axum request
//! and gives handlers access to the configured service name and the
//! [`HealthCheck`] strategy used by `/health/*` endpoints.

use std::sync::Arc;

use crate::auth::AuthContext;
use crate::health::HealthCheck;

/// State held by every gateway request handler.
#[derive(Clone)]
pub struct AppState {
    /// Human-readable service name reported by `/health`.
    pub name: String,
    /// Pluggable health-check strategy (used by `/health/ready`).
    pub db: Arc<dyn HealthCheck>,
    /// Session store and RBAC enforcer backing `/api/*`.
    ///
    /// Public so axum can extract it as request state. The session
    /// store is the only thing that turns a bearer token into a
    /// tenant, and the enforcer is the only thing that answers whether
    /// that tenant may act on an object. A business handler needs both,
    /// which is why this is state rather than a module-level global:
    /// tests must be able to build an isolated pair.
    pub auth: AuthContext,
}

impl core::fmt::Debug for AppState {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("AppState")
            .field("name", &self.name)
            .field("db", &"<dyn HealthCheck>")
            .field("auth", &self.auth)
            .finish()
    }
}

impl AppState {
    /// Build an [`AppState`] with a name and a [`HealthCheck`].
    ///
    /// Most callers will pass [`MemoryHealthCheck`](crate::health::MemoryHealthCheck)
    /// for `db`; production builds will pass a wrapper that probes the
    /// real DB / peer pool.
    ///
    /// `auth` comes from [`AuthContext::bootstrap`], which is the
    /// freshly-started shape: an empty session store and an enforcer
    /// built from the bundled policy. There is no way to reach
    /// `/api/*` until a session is minted, which is deliberate.
    pub fn new(name: impl Into<String>, db: Arc<dyn HealthCheck>) -> crate::Result<Self> {
        Ok(Self {
            name: name.into(),
            db,
            auth: AuthContext::bootstrap()?,
        })
    }

    /// Build an [`AppState`] with an explicit [`AuthContext`].
    ///
    /// The seam tests use to supply their own session store, so they do
    /// not share principal state with each other.
    pub fn with_auth(name: impl Into<String>, db: Arc<dyn HealthCheck>, auth: AuthContext) -> Self {
        Self {
            name: name.into(),
            db,
            auth,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::health::MemoryHealthCheck;

    #[test]
    fn new_takes_name_and_db() {
        let state = AppState::new("ada-gateway", Arc::new(MemoryHealthCheck::new()))
            .expect("bootstrap auth context");
        assert_eq!(state.name, "ada-gateway");
    }

    #[test]
    fn new_accepts_string_and_str() {
        let s = AppState::new(String::from("a"), Arc::new(MemoryHealthCheck::new())).unwrap();
        assert_eq!(s.name, "a");
        let s = AppState::new("b", Arc::new(MemoryHealthCheck::new())).unwrap();
        assert_eq!(s.name, "b");
    }

    #[test]
    fn a_bootstrapped_state_denies_every_token() {
        // The freshly-started shape. If this ever starts resolving a
        // token, something is minting sessions without a login flow.
        let state = AppState::new("ada-gateway", Arc::new(MemoryHealthCheck::new())).unwrap();
        assert!(state.auth.resolve("any-token").is_none());
    }
}
