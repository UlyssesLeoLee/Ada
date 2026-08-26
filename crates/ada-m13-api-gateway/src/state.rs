//! Application state shared by every gateway handler.
//!
//! [`AppState`] is cloned (cheaply, via `Arc`) into each axum request
//! and gives handlers access to the configured service name and the
//! [`HealthCheck`] strategy used by `/health/*` endpoints.

use std::sync::Arc;

use crate::health::HealthCheck;

/// State held by every gateway request handler.
#[derive(Clone)]
pub struct AppState {
    /// Human-readable service name reported by `/health`.
    pub name: String,
    /// Pluggable health-check strategy (used by `/health/ready`).
    pub db: Arc<dyn HealthCheck>,
}

impl core::fmt::Debug for AppState {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("AppState")
            .field("name", &self.name)
            .field("db", &"<dyn HealthCheck>")
            .finish()
    }
}

impl AppState {
    /// Build an [`AppState`] with a name and a [`HealthCheck`].
    ///
    /// Most callers will pass [`MemoryHealthCheck`](crate::health::MemoryHealthCheck)
    /// for `db`; production builds will pass a wrapper that probes the
    /// real DB / peer pool.
    pub fn new(name: impl Into<String>, db: Arc<dyn HealthCheck>) -> Self {
        Self {
            name: name.into(),
            db,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::health::MemoryHealthCheck;

    #[test]
    fn new_takes_name_and_db() {
        let state = AppState::new("ada-gateway", Arc::new(MemoryHealthCheck::new()));
        assert_eq!(state.name, "ada-gateway");
    }

    #[test]
    fn new_accepts_string_and_str() {
        let s = AppState::new(String::from("a"), Arc::new(MemoryHealthCheck::new()));
        assert_eq!(s.name, "a");
        let s = AppState::new("b", Arc::new(MemoryHealthCheck::new()));
        assert_eq!(s.name, "b");
    }
}
