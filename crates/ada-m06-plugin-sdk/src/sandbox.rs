//! Capability-based sandbox policy.
//!
//! The v0.1.0 skeleton keeps the policy declarative: a list of
//! `allowed_capabilities` and a [`ResourceLimits`] struct. Real
//! enforcement (syscall filters, cgroups, ...) lands in B7+.

use serde::{Deserialize, Serialize};

/// Default maximum memory in MiB for an in-process plugin.
pub const DEFAULT_MAX_MEMORY_MB: u32 = 128;

/// Hard limits for an in-process plugin. The skeleton does not
/// actually enforce them; the values are stored on
/// [`SandboxPolicy`] and returned to callers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum resident memory in MiB.
    pub max_memory_mb: u32,
    /// Maximum CPU time per call in milliseconds.
    pub max_cpu_ms_per_call: u32,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_mb: DEFAULT_MAX_MEMORY_MB,
            max_cpu_ms_per_call: 1_000,
        }
    }
}

/// Declarative sandbox policy attached to a [`PluginHost`].
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxPolicy {
    /// Capabilities the host is willing to grant. If empty, no
    /// capabilities are granted.
    pub allowed_capabilities: Vec<String>,
    /// Resource limits for any plugin invocation.
    pub resource_limits: ResourceLimits,
}

impl SandboxPolicy {
    /// `true` if `capability` is in the allow-list.
    #[must_use]
    pub fn allows(&self, capability: &str) -> bool {
        self.allowed_capabilities.iter().any(|c| c == capability)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_resource_limits_are_sane() {
        let l = ResourceLimits::default();
        assert!(l.max_memory_mb > 0);
        assert!(l.max_cpu_ms_per_call > 0);
    }

    #[test]
    fn default_policy_allows_nothing() {
        let p = SandboxPolicy::default();
        assert!(!p.allows("net:connect"));
    }

    #[test]
    fn policy_allows_listed_capability() {
        let p = SandboxPolicy {
            allowed_capabilities: vec!["net:connect:api.example.com".into()],
            resource_limits: ResourceLimits::default(),
        };
        assert!(p.allows("net:connect:api.example.com"));
        assert!(!p.allows("fs:write:/etc"));
    }

    #[test]
    fn policy_with_no_overrides_uses_defaults() {
        let p = SandboxPolicy::default();
        assert_eq!(p.resource_limits.max_memory_mb, DEFAULT_MAX_MEMORY_MB);
    }
}
