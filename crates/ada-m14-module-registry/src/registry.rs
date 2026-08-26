//! In-process [`ModuleRegistry`] backed by
//! `parking_lot::RwLock<HashMap<String, ModuleDescriptor>>`.
//!
//! The skeleton is intentionally simple: a single
//! `Arc<dyn EventBus>` is passed in (or `None` to skip
//! emission), the map is keyed by `ModuleDescriptor::name`,
//! and every state transition is published to the bus on a
//! best-effort basis (publish failures are surfaced as
//! [`RegistryError::BackendError`] but do **not** roll back the
//! state change — registry writes are the source of truth).
//!
//! ## Why `parking_lot::RwLock` and not `std::sync::RwLock`?
//!
//! Read traffic (heartbeats, `get`, `list`) dominates the
//! hot path; parking_lot's reader-biased lock is materially
//! better under read-heavy contention and is already used
//! elsewhere in the workspace (see `ada-m11-rbac-collab`).
//!
//! See [`DOC-MOD-014`](../docs/modules/M-14-module-registry.md)
//! §3.5 for the full lifecycle.

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use ada_m15_central_event_bus::{EventBus, InProcessBus};

use crate::error::{RegistryError, Result};
use crate::event::{RegistryEvent, RegistryEventKind};

/// What kind of work the module performs. Maps to the
/// `module_kind` enum in `docs/schemas/module-manifest.schema.json`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModuleKind {
    /// Pulls data from an external system (DB, S3, REST, ...).
    Ingest,
    /// Operates on data already in the pipeline.
    Transform,
    /// Pushes data out to an external system.
    Sink,
    /// Anything that does not fit the three categories above.
    Custom,
}

impl ModuleKind {
    /// Canonical lowercase string tag.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ingest => "ingest",
            Self::Transform => "transform",
            Self::Sink => "sink",
            Self::Custom => "custom",
        }
    }
}

impl fmt::Display for ModuleKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Health state of a registered module. `Unhealthy` causes the
/// skeleton to reject heartbeats (the canonical "fail closed"
/// policy; the registry stays the source of truth, the module
/// must `deregister` and re-`register` to recover).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HealthState {
    /// Module is up and responding within its SLO.
    Healthy,
    /// Module is up but degraded (e.g. some downstream is down).
    Degraded,
    /// Module is not responding; the skeleton rejects heartbeats
    /// in this state.
    Unhealthy,
    /// Initial state before the first heartbeat.
    Unknown,
}

impl HealthState {
    /// Canonical lowercase string tag.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Healthy => "healthy",
            Self::Degraded => "degraded",
            Self::Unhealthy => "unhealthy",
            Self::Unknown => "unknown",
        }
    }
}

impl fmt::Display for HealthState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A single capability tag. Kept as a `String` so producers can
/// use any taxonomy (e.g. `"sql"`, `"json"`, `"s3"`,
/// `"oauth2.read"`). The skeleton does not enforce a closed
/// set; that lives in the JSON Schema validator.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Capability(pub String);

impl Capability {
    /// Build a new capability tag.
    #[must_use]
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

impl fmt::Display for Capability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for Capability {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for Capability {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// A single health transition, kept for the audit log. The
/// skeleton does not persist the log; the integration tests
/// inspect it directly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthTransition {
    /// Previous health.
    pub from: HealthState,
    /// New health.
    pub to: HealthState,
    /// Wall-clock millis when the transition was recorded.
    pub at_ms: u64,
}

/// Immutable descriptor for a registered module. The registry
/// stores one of these per `name` and updates it on every
/// successful `register` / `heartbeat` / `deregister`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleDescriptor {
    /// Stable, globally-unique module name (e.g. `mod-ingest-csv`).
    pub name: String,
    /// Semver version string (e.g. `1.2.0`).
    pub version: String,
    /// What kind of work the module does.
    pub kind: ModuleKind,
    /// Capability tags (kept in declaration order, not sorted).
    pub capabilities: Vec<Capability>,
    /// Where the registry can reach the module (HTTP URL,
    /// gRPC socket, file path, ...). The skeleton does not
    /// dial the endpoint; the real build will.
    pub endpoint: String,
    /// Current health snapshot.
    pub health: HealthState,
    /// Wall-clock millis when the descriptor was first
    /// registered. Heartbeat updates do **not** touch this.
    pub registered_at_ms: u64,
    /// Wall-clock millis when the descriptor was last updated
    /// (registration, heartbeat, ...).
    pub updated_at_ms: u64,
}

impl ModuleDescriptor {
    /// Build a new descriptor with `health = Unknown` and the
    /// supplied `(registered_at_ms, updated_at_ms)` timestamps.
    /// Most callers should use [`Self::now`] instead.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        version: impl Into<String>,
        kind: ModuleKind,
        capabilities: Vec<Capability>,
        endpoint: impl Into<String>,
        registered_at_ms: u64,
        updated_at_ms: u64,
    ) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            kind,
            capabilities,
            endpoint: endpoint.into(),
            health: HealthState::Unknown,
            registered_at_ms,
            updated_at_ms,
        }
    }

    /// Convenience constructor that stamps both timestamps with
    /// the current wall-clock millis.
    #[must_use]
    pub fn now(
        name: impl Into<String>,
        version: impl Into<String>,
        kind: ModuleKind,
        capabilities: Vec<Capability>,
        endpoint: impl Into<String>,
    ) -> Self {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX));
        Self::new(name, version, kind, capabilities, endpoint, now_ms, now_ms)
    }

    /// Cheap in-process validation: rejects empty `name`,
    /// empty `version`, empty `endpoint`, or an endpoint that
    /// is just whitespace. The full JSON Schema validation is
    /// a follow-up; this method is what the registry calls
    /// before accepting a `register`.
    pub fn validate(&self) -> std::result::Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("name is empty".to_string());
        }
        if self.version.trim().is_empty() {
            return Err("version is empty".to_string());
        }
        if self.endpoint.trim().is_empty() {
            return Err("endpoint is empty".to_string());
        }
        Ok(())
    }
}

/// In-process module registry.
///
/// The registry can be built with or without an
/// `Arc<InProcessBus>`. When the bus is `None`, state changes
/// are still applied but no event is published.
///
/// ## Note on the bus type
///
/// We hold the concrete `Arc<InProcessBus>` rather than
/// `Arc<dyn EventBus>` because the v0.1.0 `EventBus` trait is
/// not dyn-compatible (`publish` carries a generic parameter).
/// Production builds will add a non-generic
/// `publish_event(&self, &BusEvent)` helper on the bus trait
/// so this can become `Arc<dyn EventBus>`. The wiring here
/// is a single `bus.publish(&envelope)` call, so the upgrade
/// is mechanical.
pub struct ModuleRegistry {
    inner: RwLock<HashMap<String, ModuleDescriptor>>,
    bus: Option<Arc<InProcessBus>>,
    transitions: RwLock<Vec<(String, HealthTransition)>>,
}

impl std::fmt::Debug for ModuleRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let map = self.inner.read();
        let transitions = self.transitions.read();
        f.debug_struct("ModuleRegistry")
            .field("module_count", &map.len())
            .field("has_bus", &self.bus.is_some())
            .field("transitions", &transitions.len())
            .finish_non_exhaustive()
    }
}

impl ModuleRegistry {
    /// Build a registry with no event bus (state changes still
    /// apply, no event is published).
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
            bus: None,
            transitions: RwLock::new(Vec::new()),
        }
    }

    /// Build a registry that publishes state changes to `bus`.
    /// The bus is held as `Arc<InProcessBus>` (see the type-level
    /// doc on `ModuleRegistry` for why this is not
    /// `Arc<dyn EventBus>`).
    #[must_use]
    pub fn with_bus(bus: Arc<InProcessBus>) -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
            bus: Some(bus),
            transitions: RwLock::new(Vec::new()),
        }
    }

    /// Register a new module. Returns the stored descriptor
    /// (which may differ from the input if the registry stamps
    /// extra fields).
    pub async fn register(&self, mut descriptor: ModuleDescriptor) -> Result<ModuleDescriptor> {
        if let Err(msg) = descriptor.validate() {
            return Err(RegistryError::InvalidDescriptor(msg));
        }
        let name = descriptor.name.clone();
        {
            let mut map = self.inner.write();
            if map.contains_key(&name) {
                return Err(RegistryError::AlreadyRegistered(name));
            }
            // First registration: stamp `Unknown` health so the
            // initial `module.registered` event reflects "we've
            // heard of you but haven't seen a heartbeat yet".
            descriptor.health = HealthState::Unknown;
            map.insert(name.clone(), descriptor.clone());
        }
        // Best-effort event publish. Bus failures are surfaced
        // to the caller but the descriptor is still stored.
        self.try_publish(RegistryEvent {
            kind: RegistryEventKind::Registered,
            module_name: name,
            tenant_id: None,
            old_health: None,
            new_health: None,
        })
        .await?;
        Ok(descriptor)
    }

    /// Deregister a module. Returns the removed descriptor.
    pub async fn deregister(&self, name: &str) -> Result<ModuleDescriptor> {
        let removed = {
            let mut map = self.inner.write();
            map.remove(name)
                .ok_or_else(|| RegistryError::NotFound(name.to_string()))?
        };
        self.try_publish(RegistryEvent {
            kind: RegistryEventKind::Deregistered,
            module_name: removed.name.clone(),
            tenant_id: None,
            old_health: Some(removed.health),
            new_health: None,
        })
        .await?;
        Ok(removed)
    }

    /// Look up a module by `name`.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<ModuleDescriptor> {
        self.inner.read().get(name).cloned()
    }

    /// List every registered module, sorted by `name`.
    #[must_use]
    pub fn list(&self) -> Vec<ModuleDescriptor> {
        let mut v: Vec<ModuleDescriptor> = self.inner.read().values().cloned().collect();
        v.sort_by(|a, b| a.name.cmp(&b.name));
        v
    }

    /// Number of registered modules.
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.read().len()
    }

    /// True if no module is registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.read().is_empty()
    }

    /// Update the health state of an existing module. Returns
    /// the new descriptor and the recorded transition (if any).
    ///
    /// Policy:
    /// - `Unhealthy` updates are rejected with
    ///   [`RegistryError::HealthCheckFailed`].
    /// - Heartbeats that do not change the state are accepted
    ///   but **no** `HealthChanged` event is emitted.
    pub async fn heartbeat(
        &self,
        name: &str,
        new_health: HealthState,
    ) -> Result<(ModuleDescriptor, Option<HealthTransition>)> {
        if matches!(new_health, HealthState::Unhealthy) {
            return Err(RegistryError::HealthCheckFailed(format!(
                "{name} reported Unhealthy; call deregister + register to recover"
            )));
        }
        let (updated, transition) = {
            let mut map = self.inner.write();
            let entry = map
                .get_mut(name)
                .ok_or_else(|| RegistryError::NotFound(name.to_string()))?;
            let old = entry.health;
            entry.updated_at_ms = now_ms();
            entry.health = new_health;
            let transition = if old == new_health {
                None
            } else {
                let t = HealthTransition {
                    from: old,
                    to: new_health,
                    at_ms: entry.updated_at_ms,
                };
                self.transitions
                    .write()
                    .push((entry.name.clone(), t.clone()));
                Some(t)
            };
            (entry.clone(), transition)
        };
        if let Some(ref t) = transition {
            self.try_publish(RegistryEvent {
                kind: RegistryEventKind::HealthChanged,
                module_name: updated.name.clone(),
                tenant_id: None,
                old_health: Some(t.from),
                new_health: Some(t.to),
            })
            .await?;
        }
        Ok((updated, transition))
    }

    /// Read-only access to the recorded health transitions.
    #[must_use]
    pub fn transitions(&self) -> Vec<(String, HealthTransition)> {
        self.transitions.read().clone()
    }

    /// Publish `event` to the configured bus (if any). A missing
    /// bus is a no-op; a present-but-closed bus surfaces
    /// [`RegistryError::BackendError`].
    async fn try_publish(&self, event: RegistryEvent) -> Result<()> {
        let Some(bus) = self.bus.as_ref() else {
            return Ok(());
        };
        let envelope = event.to_bus_event();
        bus.publish(&envelope)
            .await
            .map_err(|e| RegistryError::BackendError(e.to_string()))?;
        Ok(())
    }
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn desc(name: &str) -> ModuleDescriptor {
        ModuleDescriptor::now(
            name,
            "1.0.0",
            ModuleKind::Ingest,
            vec![Capability::new("sql")],
            "http://localhost:8080",
        )
    }

    #[test]
    fn module_kind_as_str() {
        assert_eq!(ModuleKind::Ingest.as_str(), "ingest");
        assert_eq!(ModuleKind::Transform.as_str(), "transform");
        assert_eq!(ModuleKind::Sink.as_str(), "sink");
        assert_eq!(ModuleKind::Custom.as_str(), "custom");
    }

    #[test]
    fn health_state_as_str() {
        assert_eq!(HealthState::Healthy.as_str(), "healthy");
        assert_eq!(HealthState::Degraded.as_str(), "degraded");
        assert_eq!(HealthState::Unhealthy.as_str(), "unhealthy");
        assert_eq!(HealthState::Unknown.as_str(), "unknown");
    }

    #[test]
    fn descriptor_validate_rejects_empty_name() {
        let d = ModuleDescriptor::now("", "1.0.0", ModuleKind::Ingest, vec![], "http://x");
        assert!(d.validate().is_err());
    }

    #[test]
    fn descriptor_validate_rejects_empty_version() {
        let d = ModuleDescriptor::now("a", "  ", ModuleKind::Ingest, vec![], "http://x");
        assert!(d.validate().is_err());
    }

    #[test]
    fn descriptor_validate_rejects_empty_endpoint() {
        let d = ModuleDescriptor::now("a", "1.0.0", ModuleKind::Ingest, vec![], "");
        assert!(d.validate().is_err());
    }

    #[test]
    fn descriptor_validate_accepts_well_formed() {
        let d = desc("a");
        assert!(d.validate().is_ok());
    }

    #[tokio::test]
    async fn register_then_get() {
        let r = ModuleRegistry::new();
        let d = desc("a");
        let stored = r.register(d.clone()).await.expect("register");
        assert_eq!(stored.name, "a");
        assert_eq!(stored.health, HealthState::Unknown);
        let got = r.get("a").expect("present");
        assert_eq!(got.name, "a");
    }

    #[tokio::test]
    async fn register_rejects_duplicate() {
        let r = ModuleRegistry::new();
        r.register(desc("a")).await.expect("first");
        let err = r.register(desc("a")).await.expect_err("dup");
        assert!(matches!(err, RegistryError::AlreadyRegistered(_)));
    }

    #[tokio::test]
    async fn register_rejects_invalid_descriptor() {
        let r = ModuleRegistry::new();
        let d = ModuleDescriptor::now("", "1.0.0", ModuleKind::Ingest, vec![], "http://x");
        let err = r.register(d).await.expect_err("invalid");
        assert!(matches!(err, RegistryError::InvalidDescriptor(_)));
    }

    #[tokio::test]
    async fn deregister_then_not_found() {
        let r = ModuleRegistry::new();
        r.register(desc("a")).await.expect("register");
        let removed = r.deregister("a").await.expect("deregister");
        assert_eq!(removed.name, "a");
        assert!(r.get("a").is_none());
        let err = r.deregister("a").await.expect_err("not found");
        assert!(matches!(err, RegistryError::NotFound(_)));
    }

    #[tokio::test]
    async fn list_returns_sorted_modules() {
        let r = ModuleRegistry::new();
        r.register(desc("c")).await.unwrap();
        r.register(desc("a")).await.unwrap();
        r.register(desc("b")).await.unwrap();
        let listed = r.list();
        let names: Vec<&str> = listed.iter().map(|d| d.name.as_str()).collect();
        assert_eq!(names, vec!["a", "b", "c"]);
        assert_eq!(r.len(), 3);
        assert!(!r.is_empty());
    }

    #[tokio::test]
    async fn heartbeat_records_transition_and_updates_state() {
        let r = ModuleRegistry::new();
        r.register(desc("a")).await.unwrap();
        let (updated, t) = r.heartbeat("a", HealthState::Healthy).await.unwrap();
        assert_eq!(updated.health, HealthState::Healthy);
        let t = t.expect("transition");
        assert_eq!(t.from, HealthState::Unknown);
        assert_eq!(t.to, HealthState::Healthy);
        assert_eq!(r.transitions().len(), 1);
    }

    #[tokio::test]
    async fn heartbeat_with_no_state_change_records_no_transition() {
        let r = ModuleRegistry::new();
        r.register(desc("a")).await.unwrap();
        r.heartbeat("a", HealthState::Healthy).await.unwrap();
        // Second heartbeat with the same state should not record
        // a transition.
        let (_, t) = r.heartbeat("a", HealthState::Healthy).await.unwrap();
        assert!(t.is_none());
        assert_eq!(r.transitions().len(), 1);
    }

    #[tokio::test]
    async fn heartbeat_rejects_unhealthy() {
        let r = ModuleRegistry::new();
        r.register(desc("a")).await.unwrap();
        let err = r
            .heartbeat("a", HealthState::Unhealthy)
            .await
            .expect_err("rejected");
        assert!(matches!(err, RegistryError::HealthCheckFailed(_)));
    }

    #[tokio::test]
    async fn heartbeat_for_unknown_module_errors() {
        let r = ModuleRegistry::new();
        let err = r
            .heartbeat("missing", HealthState::Healthy)
            .await
            .expect_err("not found");
        assert!(matches!(err, RegistryError::NotFound(_)));
    }

    #[tokio::test]
    async fn empty_registry_is_empty() {
        let r = ModuleRegistry::new();
        assert!(r.is_empty());
        assert_eq!(r.len(), 0);
        assert!(r.list().is_empty());
    }
}
