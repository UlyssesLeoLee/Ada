//! Plugin host trait and the in-process [`InMemoryHost`] impl.

use async_trait::async_trait;
use parking_lot::Mutex;
use serde_json::Value;
use std::collections::HashMap;

use crate::error::{Result, SdkError};
use crate::manifest::{PluginId, PluginManifest};
use crate::sandbox::SandboxPolicy;

/// A registry that holds installed plugin manifests and an
/// active [`SandboxPolicy`]. Implementations are responsible
/// for actually loading and invoking the plugin artifacts; the
/// v0.1.0 skeleton only stores the manifest and runs a
/// capability check before recording a fake invoke result.
#[async_trait]
pub trait PluginHost: Send + Sync {
    /// Install (register) `manifest`. Returns the assigned
    /// `PluginId` (the manifest's own `id` is preserved on
    /// install).
    async fn install(&self, manifest: PluginManifest) -> Result<PluginId>;

    /// Remove a previously installed plugin.
    async fn uninstall(&self, id: PluginId) -> Result<()>;

    /// Invoke a previously installed plugin with `input`. The
    /// v0.1.0 skeleton returns `serde_json::Value::Null` and
    /// does **not** actually run the artifact — but the
    /// capability check is real.
    async fn invoke(&self, id: PluginId, input: Value) -> Result<Value>;

    /// Snapshot of all currently installed manifests.
    async fn list(&self) -> Result<Vec<PluginManifest>>;
}

/// In-process, `Mutex`-backed [`PluginHost`].
#[derive(Debug)]
pub struct InMemoryHost {
    plugins: Mutex<HashMap<PluginId, PluginManifest>>,
    policy: Mutex<SandboxPolicy>,
}

impl Default for InMemoryHost {
    fn default() -> Self {
        Self::new(SandboxPolicy::default())
    }
}

impl InMemoryHost {
    /// Create a new host with the given sandbox policy.
    #[must_use]
    pub fn new(policy: SandboxPolicy) -> Self {
        Self {
            plugins: Mutex::new(HashMap::new()),
            policy: Mutex::new(policy),
        }
    }

    /// Replace the active sandbox policy.
    pub fn set_policy(&self, policy: SandboxPolicy) {
        *self.policy.lock() = policy;
    }
}

#[async_trait]
impl PluginHost for InMemoryHost {
    async fn install(&self, manifest: PluginManifest) -> Result<PluginId> {
        manifest.validate()?;
        let id = manifest.id;
        self.plugins.lock().insert(id, manifest);
        Ok(id)
    }

    async fn uninstall(&self, id: PluginId) -> Result<()> {
        self.plugins
            .lock()
            .remove(&id)
            .map(|_| ())
            .ok_or(SdkError::PluginNotFound(id))
    }

    async fn invoke(&self, id: PluginId, _input: Value) -> Result<Value> {
        let plugins = self.plugins.lock();
        let manifest = plugins.get(&id).ok_or(SdkError::PluginNotFound(id))?;
        let policy = self.policy.lock();
        for cap in &manifest.capabilities {
            if !policy.allows(cap) {
                return Err(SdkError::CapabilityDenied {
                    capability: cap.clone(),
                });
            }
        }
        // Skeleton: real WASM/native/script execution lands in B7+.
        Ok(Value::Null)
    }

    async fn list(&self) -> Result<Vec<PluginManifest>> {
        Ok(self.plugins.lock().values().cloned().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::PluginKind;
    use crate::sandbox::ResourceLimits;

    fn manifest() -> PluginManifest {
        PluginManifest::new("a", "0.1.0", PluginKind::Wasm).with_entry_point("run")
    }

    #[tokio::test]
    async fn install_assigns_id() {
        let h = InMemoryHost::default();
        let m = manifest();
        let id = h.install(m).await.expect("install");
        let list = h.list().await.expect("list");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, id);
    }

    #[tokio::test]
    async fn install_rejects_invalid_manifest() {
        let h = InMemoryHost::default();
        let m = PluginManifest::new("a", "0.1.0", PluginKind::Wasm);
        let err = h.install(m).await.unwrap_err();
        assert!(matches!(err, SdkError::ManifestInvalid { .. }));
    }

    #[tokio::test]
    async fn uninstall_removes_plugin() {
        let h = InMemoryHost::default();
        let m = manifest();
        let id = h.install(m).await.expect("install");
        h.uninstall(id).await.expect("uninstall");
        assert!(h.list().await.expect("list").is_empty());
    }

    #[tokio::test]
    async fn uninstall_unknown_plugin_errors() {
        let h = InMemoryHost::default();
        let err = h.uninstall(PluginId::new()).await.unwrap_err();
        assert!(matches!(err, SdkError::PluginNotFound(_)));
    }

    #[tokio::test]
    async fn invoke_unknown_plugin_errors() {
        let h = InMemoryHost::default();
        let err = h.invoke(PluginId::new(), Value::Null).await.unwrap_err();
        assert!(matches!(err, SdkError::PluginNotFound(_)));
    }

    #[tokio::test]
    async fn invoke_with_no_capabilities_succeeds() {
        let h = InMemoryHost::default();
        let m = manifest();
        let id = h.install(m).await.expect("install");
        let out = h.invoke(id, Value::Null).await.expect("invoke");
        assert_eq!(out, Value::Null);
    }

    #[tokio::test]
    async fn invoke_denies_capability_not_in_policy() {
        let h = InMemoryHost::default();
        let m = manifest().with_capability("net:connect:api.example.com");
        let id = h.install(m).await.expect("install");
        let err = h.invoke(id, Value::Null).await.unwrap_err();
        match err {
            SdkError::CapabilityDenied { capability } => {
                assert_eq!(capability, "net:connect:api.example.com");
            }
            other => panic!("expected CapabilityDenied, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn invoke_allows_capability_in_policy() {
        let h = InMemoryHost::new(SandboxPolicy {
            allowed_capabilities: vec!["net:connect:api.example.com".into()],
            resource_limits: ResourceLimits::default(),
        });
        let m = manifest().with_capability("net:connect:api.example.com");
        let id = h.install(m).await.expect("install");
        h.invoke(id, Value::Null).await.expect("invoke");
    }
}
