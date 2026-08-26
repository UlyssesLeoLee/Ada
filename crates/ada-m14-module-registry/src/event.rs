//! Registry events built on top of [`BusEvent`](ada_m15_central_event_bus::BusEvent).
//!
//! The registry emits three event kinds:
//!
//! - [`RegistryEventKind::Registered`]   — on a successful `register`
//! - [`RegistryEventKind::Deregistered`] — on a successful `deregister`
//! - [`RegistryEventKind::HealthChanged`] — on a `heartbeat` that
//!   actually changed the health state
//!
//! Every event is wrapped in a [`BusEvent`](ada_m15_central_event_bus::BusEvent)
//! with the canonical topic `module.<kind>` so subscribers can use
//! the M-15 glob filter. See
//! [`DOC-MOD-014`](../docs/modules/M-14-module-registry.md) §3.3
//! for the topic convention.

use std::collections::BTreeMap;

use ada_core::TenantId;
use ada_m15_central_event_bus::{BusEvent, EventId, Topic};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::registry::HealthState;

/// What the registry is telling the world.
///
/// The skeleton keeps the enum a small `Copy` type so it can be
/// stored on the `HealthTransition` log without an allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RegistryEventKind {
    /// A new module was registered.
    Registered,
    /// A module was deregistered.
    Deregistered,
    /// A module's health state transitioned from one value to
    /// another.
    HealthChanged,
}

impl RegistryEventKind {
    /// Canonical lowercase string tag, used in the M-15 topic
    /// (`module.<kind>`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Registered => "registered",
            Self::Deregistered => "deregistered",
            Self::HealthChanged => "health_changed",
        }
    }
}

/// The envelope the registry hands to the bus. `to_bus_event`
/// stamps the M-15 topic and a JSON payload, then returns the
/// concrete [`BusEvent`].
#[derive(Debug, Clone)]
pub struct RegistryEvent {
    /// What happened.
    pub kind: RegistryEventKind,
    /// Module this event is about.
    pub module_name: String,
    /// Tenant scope (`None` for system modules).
    pub tenant_id: Option<TenantId>,
    /// Old health (only for `HealthChanged`).
    pub old_health: Option<HealthState>,
    /// New health (only for `HealthChanged`).
    pub new_health: Option<HealthState>,
}

impl RegistryEvent {
    /// Convenience constructor for a `Registered` event.
    #[must_use]
    pub fn registered(module_name: impl Into<String>, tenant_id: Option<TenantId>) -> Self {
        Self {
            kind: RegistryEventKind::Registered,
            module_name: module_name.into(),
            tenant_id,
            old_health: None,
            new_health: None,
        }
    }

    /// Convenience constructor for a `Deregistered` event.
    #[must_use]
    pub fn deregistered(module_name: impl Into<String>, tenant_id: Option<TenantId>) -> Self {
        Self {
            kind: RegistryEventKind::Deregistered,
            module_name: module_name.into(),
            tenant_id,
            old_health: None,
            new_health: None,
        }
    }

    /// Convenience constructor for a `HealthChanged` event.
    #[must_use]
    pub fn health_changed(
        module_name: impl Into<String>,
        tenant_id: Option<TenantId>,
        old: HealthState,
        new: HealthState,
    ) -> Self {
        Self {
            kind: RegistryEventKind::HealthChanged,
            module_name: module_name.into(),
            tenant_id,
            old_health: Some(old),
            new_health: Some(new),
        }
    }

    /// Topic string the event will be published under. Always
    /// `module.<kind>`.
    #[must_use]
    pub fn topic_str(&self) -> String {
        format!("module.{}", self.kind.as_str())
    }

    /// Turn this envelope into a [`BusEvent`] ready to be handed
    /// to an M-15 `EventBus`.
    #[must_use]
    pub fn to_bus_event(&self) -> BusEvent {
        let topic =
            Topic::new(self.topic_str()).expect("registry event topic is a non-empty literal");
        let mut headers = BTreeMap::new();
        headers.insert("schema_version".to_string(), "1.0".to_string());
        headers.insert("event_kind".to_string(), self.kind.as_str().to_string());
        let payload = json!({
            "module_name": self.module_name,
            "old_health": self.old_health.map(HealthState::as_str),
            "new_health": self.new_health.map(HealthState::as_str),
        });
        let produced_at_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX));
        BusEvent {
            event_id: EventId::new(),
            topic,
            tenant_id: self.tenant_id,
            schema_version: "1.0".to_string(),
            producer: NAME.to_string(),
            trace_id: None,
            payload,
            headers,
            produced_at_ms,
        }
    }
}

// Local `NAME` reference so we don't have to thread a constant
// through the public surface; mirrors the M-15 producer pattern.
const NAME: &str = "ada-m14-module-registry";

#[cfg(test)]
mod tests {
    use super::*;
    use ada_core::TenantId;
    use uuid::Uuid;

    #[test]
    fn kind_as_str() {
        assert_eq!(RegistryEventKind::Registered.as_str(), "registered");
        assert_eq!(RegistryEventKind::Deregistered.as_str(), "deregistered");
        assert_eq!(RegistryEventKind::HealthChanged.as_str(), "health_changed");
    }

    #[test]
    fn registered_topic_and_payload() {
        let t = TenantId(Uuid::new_v4());
        let evt = RegistryEvent::registered("mod-a", Some(t));
        assert_eq!(evt.topic_str(), "module.registered");
        let be = evt.to_bus_event();
        assert_eq!(be.topic.as_str(), "module.registered");
        assert_eq!(be.tenant_id, Some(t));
        assert_eq!(be.producer, "ada-m14-module-registry");
        assert_eq!(
            be.headers.get("event_kind").map(String::as_str),
            Some("registered")
        );
    }

    #[test]
    fn deregistered_topic_and_payload() {
        let evt = RegistryEvent::deregistered("mod-a", None);
        assert_eq!(evt.topic_str(), "module.deregistered");
        let be = evt.to_bus_event();
        assert!(be.tenant_id.is_none());
        assert_eq!(be.payload["module_name"], "mod-a");
    }

    #[test]
    fn health_changed_carries_old_and_new() {
        let evt = RegistryEvent::health_changed(
            "mod-a",
            None,
            HealthState::Healthy,
            HealthState::Degraded,
        );
        let be = evt.to_bus_event();
        assert_eq!(be.topic.as_str(), "module.health_changed");
        assert_eq!(be.payload["old_health"], "healthy");
        assert_eq!(be.payload["new_health"], "degraded");
    }

    #[test]
    fn to_bus_event_assigns_fresh_event_id() {
        let a = RegistryEvent::registered("a", None).to_bus_event();
        let b = RegistryEvent::registered("a", None).to_bus_event();
        assert_ne!(a.event_id, b.event_id);
    }
}
