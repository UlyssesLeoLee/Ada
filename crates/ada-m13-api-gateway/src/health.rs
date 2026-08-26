//! Health-check trait + a default in-memory implementation.
//!
//! The gateway exposes `GET /health/live` and `GET /health/ready` and
//! needs a pluggable strategy for "are my dependencies healthy?". The
//! [`HealthCheck`] trait is the seam; [`MemoryHealthCheck`] is the
//! zero-dependency default that always reports [`HealthStatus::Healthy`]
//! and stamps the current UTC time. Real adapters (DB pool, peer RPC,
//! ...) plug in by implementing the trait.

use std::time::SystemTime;

use ada_core::Result as AdaResult;
use async_trait::async_trait;
use serde::Serialize;

/// Health verdict reported by a [`HealthCheck`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    /// All dependencies reachable, ready to serve traffic.
    Healthy,
    /// Some non-critical dependency is degraded; traffic may be served
    /// with reduced confidence.
    Degraded,
    /// A critical dependency is down; readiness probes should fail.
    Unhealthy,
}

/// A pluggable health-check strategy.
///
/// Implementations report the *worst* verdict across the dependencies
/// they probe. Returning [`AdaError`] means the probe itself could not
/// produce a verdict (e.g. the health-check DB is unreachable).
#[async_trait]
pub trait HealthCheck: Send + Sync {
    /// Run the probe and report a [`HealthStatus`].
    async fn check(&self) -> AdaResult<HealthStatus>;
}

/// Always-healthy default [`HealthCheck`] used by `AppState::new`.
///
/// Stamps the current system time in the response payload so readiness
/// probes can confirm liveness (a stuck probe would surface as a stale
/// timestamp).
#[derive(Debug, Default, Clone, Copy)]
pub struct MemoryHealthCheck;

impl MemoryHealthCheck {
    /// Construct a fresh `MemoryHealthCheck` (no state).
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Current system time in milliseconds since the UNIX epoch.
    #[must_use]
    pub fn timestamp_millis() -> u128 {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_or(0, |d| d.as_millis())
    }
}

#[async_trait]
impl HealthCheck for MemoryHealthCheck {
    async fn check(&self) -> AdaResult<HealthStatus> {
        // Trivial impl: nothing to probe yet. The async signature
        // exists so production implementations can `await` real
        // probes without changing the trait.
        Ok(HealthStatus::Healthy)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn memory_health_check_is_healthy() {
        let hc = MemoryHealthCheck::new();
        let verdict = hc.check().await.expect("check should not fail");
        assert_eq!(verdict, HealthStatus::Healthy);
    }

    #[test]
    fn timestamp_millis_is_non_decreasing() {
        let a = MemoryHealthCheck::timestamp_millis();
        let b = MemoryHealthCheck::timestamp_millis();
        assert!(b >= a, "{b} should be >= {a}");
    }

    #[test]
    fn health_status_serde_is_lowercase() {
        let json = serde_json::to_string(&HealthStatus::Healthy).unwrap();
        assert_eq!(json, "\"healthy\"");
        let json = serde_json::to_string(&HealthStatus::Degraded).unwrap();
        assert_eq!(json, "\"degraded\"");
        let json = serde_json::to_string(&HealthStatus::Unhealthy).unwrap();
        assert_eq!(json, "\"unhealthy\"");
    }
}
