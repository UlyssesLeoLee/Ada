//! Plugin manifest and the [`PluginKind`] enum.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Stable, opaque identifier for an installed plugin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PluginId(pub Uuid);

impl PluginId {
    /// Create a fresh random `PluginId`.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for PluginId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for PluginId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The three canonical plugin kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PluginKind {
    /// A WebAssembly module loaded via Wasmtime (B7+).
    Wasm,
    /// A dynamically-loaded native library (`.so` / `.dll`).
    Native,
    /// An embedded scripting language (Rhai, Lua, JS — language
    /// TBD in B7+).
    Script,
}

impl std::fmt::Display for PluginKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Wasm => "wasm",
            Self::Native => "native",
            Self::Script => "script",
        };
        f.write_str(s)
    }
}

/// Static description of a plugin: what it is, what it needs,
/// and how to invoke it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Stable id. Re-generated for each install.
    pub id: PluginId,
    /// Human-readable name (e.g. "ada-m14-module-registry").
    pub name: String,
    /// SemVer version string (e.g. "0.1.0").
    pub version: String,
    /// Kind of artifact.
    pub kind: PluginKind,
    /// Capability strings the plugin requires (e.g. "fs:read:/var/log").
    pub capabilities: Vec<String>,
    /// Path or function name to invoke. Interpretation depends
    /// on `kind` (e.g. exported WASM function, native symbol,
    /// script entry-point).
    pub entry_point: String,
    /// SHA-256 hex digest of the plugin artifact. Reserved;
    /// not enforced in v0.1.0.
    pub hash: String,
    /// Cryptographic signature of `hash`. Reserved; not
    /// enforced in v0.1.0.
    pub signature: String,
}

impl PluginManifest {
    /// Create a new manifest with a fresh `id`. The caller is
    /// responsible for setting `version`, `kind`, `capabilities`,
    /// `entry_point`, `hash`, `signature`.
    #[must_use]
    pub fn new(name: impl Into<String>, version: impl Into<String>, kind: PluginKind) -> Self {
        Self {
            id: PluginId::new(),
            name: name.into(),
            version: version.into(),
            kind,
            capabilities: Vec::new(),
            entry_point: String::new(),
            hash: String::new(),
            signature: String::new(),
        }
    }

    /// Builder-style: append a required capability.
    #[must_use]
    pub fn with_capability(mut self, cap: impl Into<String>) -> Self {
        self.capabilities.push(cap.into());
        self
    }

    /// Builder-style: set the entry point.
    #[must_use]
    pub fn with_entry_point(mut self, ep: impl Into<String>) -> Self {
        self.entry_point = ep.into();
        self
    }

    /// Builder-style: set the artifact hash.
    #[must_use]
    pub fn with_hash(mut self, hash: impl Into<String>) -> Self {
        self.hash = hash.into();
        self
    }

    /// Builder-style: set the signature.
    #[must_use]
    pub fn with_signature(mut self, sig: impl Into<String>) -> Self {
        self.signature = sig.into();
        self
    }

    /// Validate the manifest. Returns `Err(SdkError::ManifestInvalid)`
    /// if any required field is empty or `version` is not a
    /// 3-segment SemVer (e.g. `0.1.0`).
    pub fn validate(&self) -> crate::Result<()> {
        if self.name.is_empty() {
            return Err(crate::SdkError::ManifestInvalid {
                reason: "name is empty".into(),
            });
        }
        if self.version.is_empty() {
            return Err(crate::SdkError::ManifestInvalid {
                reason: "version is empty".into(),
            });
        }
        if self
            .version
            .split('.')
            .filter_map(|s| s.parse::<u32>().ok())
            .count()
            != 3
        {
            return Err(crate::SdkError::ManifestInvalid {
                reason: format!("version is not SemVer (X.Y.Z): {}", self.version),
            });
        }
        if self.entry_point.is_empty() {
            return Err(crate::SdkError::ManifestInvalid {
                reason: "entry_point is empty".into(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_id_is_unique() {
        let a = PluginId::new();
        let b = PluginId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn plugin_id_display() {
        let id = PluginId::new();
        let s = id.to_string();
        assert_eq!(s.len(), 36); // canonical UUID form
    }

    #[test]
    fn plugin_kind_display() {
        assert_eq!(PluginKind::Wasm.to_string(), "wasm");
        assert_eq!(PluginKind::Native.to_string(), "native");
        assert_eq!(PluginKind::Script.to_string(), "script");
    }

    #[test]
    fn manifest_new_has_empty_capabilities() {
        let m = PluginManifest::new("a", "0.1.0", PluginKind::Wasm);
        assert_eq!(m.name, "a");
        assert_eq!(m.version, "0.1.0");
        assert_eq!(m.kind, PluginKind::Wasm);
        assert!(m.capabilities.is_empty());
    }

    #[test]
    fn manifest_builder_appends_capability() {
        let m = PluginManifest::new("a", "0.1.0", PluginKind::Wasm)
            .with_capability("net:connect:api.example.com")
            .with_capability("fs:read:/var/log");
        assert_eq!(m.capabilities.len(), 2);
    }

    #[test]
    fn manifest_validate_ok() {
        let m =
            PluginManifest::new("a", "1.2.3", PluginKind::Native).with_entry_point("libfoo:run");
        m.validate().expect("valid");
    }

    #[test]
    fn manifest_validate_rejects_empty_name() {
        let m = PluginManifest::new("", "0.1.0", PluginKind::Wasm).with_entry_point("x");
        let err = m.validate().unwrap_err();
        assert!(matches!(err, crate::SdkError::ManifestInvalid { .. }));
    }

    #[test]
    fn manifest_validate_rejects_non_semver() {
        let m = PluginManifest::new("a", "1.2", PluginKind::Wasm).with_entry_point("x");
        let err = m.validate().unwrap_err();
        let s = err.to_string();
        assert!(s.contains("SemVer"), "got: {s}");
    }

    #[test]
    fn manifest_validate_rejects_empty_entry() {
        let m = PluginManifest::new("a", "0.1.0", PluginKind::Wasm);
        let err = m.validate().unwrap_err();
        let s = err.to_string();
        assert!(s.contains("entry_point"), "got: {s}");
    }
}
