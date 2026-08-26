//! Error surface for the M-06 plugin SDK.

use thiserror::Error;

use crate::manifest::PluginId;

/// Failure modes surfaced by the plugin host.
#[derive(Debug, Error)]
pub enum SdkError {
    /// The plugin id was not registered with the host.
    #[error("plugin not found: {0}")]
    PluginNotFound(PluginId),

    /// The manifest failed validation (missing field, bad
    /// version, malformed entry_point, ...).
    #[error("invalid manifest: {reason}")]
    ManifestInvalid {
        /// Human-readable reason.
        reason: String,
    },

    /// The plugin tried to use a capability that the active
    /// `SandboxPolicy` does not allow.
    #[error("capability denied: {capability}")]
    CapabilityDenied {
        /// The capability that was requested.
        capability: String,
    },

    /// The plugin's `hash` field did not match the on-disk
    /// bytes (in production, the on-disk artifact is fetched
    /// and SHA-256'd). The v0.1.0 skeleton stores the field
    /// but does not enforce it.
    #[error("plugin hash mismatch: expected {expected}, got {actual}")]
    HashMismatch {
        /// The hash recorded in the manifest.
        expected: String,
        /// The hash computed from the artifact.
        actual: String,
    },

    /// The backing store (Postgres, object storage, ...) failed.
    #[error("backend error: {0}")]
    BackendError(String),
}

/// `Result` alias for fallible plugin-host operations.
pub type Result<T> = core::result::Result<T, SdkError>;

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn plugin_id() -> PluginId {
        PluginId(Uuid::new_v4())
    }

    #[test]
    fn plugin_not_found_display() {
        let e = SdkError::PluginNotFound(plugin_id());
        let s = e.to_string();
        assert!(s.starts_with("plugin not found: "), "got: {s}");
    }

    #[test]
    fn manifest_invalid_display() {
        let e = SdkError::ManifestInvalid {
            reason: "missing capabilities".into(),
        };
        assert_eq!(e.to_string(), "invalid manifest: missing capabilities");
    }

    #[test]
    fn capability_denied_display() {
        let e = SdkError::CapabilityDenied {
            capability: "fs:write:/etc".into(),
        };
        assert_eq!(e.to_string(), "capability denied: fs:write:/etc");
    }

    #[test]
    fn hash_mismatch_display() {
        let e = SdkError::HashMismatch {
            expected: "abc".into(),
            actual: "def".into(),
        };
        assert_eq!(e.to_string(), "plugin hash mismatch: expected abc, got def");
    }

    #[test]
    fn backend_error_display() {
        let e = SdkError::BackendError("s3: timeout".into());
        assert_eq!(e.to_string(), "backend error: s3: timeout");
    }

    #[test]
    fn result_alias_carries_error() {
        let ok: Result<u32> = Ok(1);
        let err: Result<u32> = Err(SdkError::BackendError("x".into()));
        assert!(matches!(ok, Ok(1)));
        assert!(err.is_err());
    }
}
