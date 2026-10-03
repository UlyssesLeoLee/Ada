//! Static policy loaded from `crates/ada-rbac-casbin/policies/`. The
//! v0.4.0 skeleton reads `base_policy.csv` (the m11 role × permission
//! matrix regenerated as Casbin-style rows). The on-disk layout is
//! the same as Casbin's so the v0.5.0 swap-in is mechanical.
//!
//! v0.5.0 anchors the bundled path to `CARGO_MANIFEST_DIR` so tests
//! (which run with cwd = `crates/ada-rbac-casbin/`) and the
//! api-gateway binary (cwd = workspace root) both find the policy
//! files without an env var.
//!
//! That reasoning is right for a developer machine and wrong for a
//! deployed one: `CARGO_MANIFEST_DIR` is baked in at compile time, so
//! the shipped binary hard-codes a path into the build tree. In a
//! container image that tree does not exist, and the gateway exits
//! fatally before serving anything:
//!
//! ```text
//! fatal: internal error: build rbac enforcer: policy reload failed:
//! policy file not found: /src/crates/ada-rbac-casbin/policies/model.conf
//! ```
//!
//! So the compiled-in path is the fallback, not the only source. A
//! container sets `ADA_RBAC_POLICY_DIR` and ships the two files.

use std::path::{Path, PathBuf};

use crate::error::{RbacCasbinError, Result};

/// Environment variable naming the directory holding `model.conf` and
/// `base_policy.csv`. Set this in any deployment where the source tree
/// is not present -- i.e. every container image.
pub const POLICY_DIR_ENV: &str = "ADA_RBAC_POLICY_DIR";

#[derive(Debug, Clone)]
pub struct PolicySet {
    pub model_path: PathBuf,
    pub policy_path: PathBuf,
    pub overlays: Vec<PathBuf>,
}

impl PolicySet {
    /// Bundled policy files.
    ///
    /// The paths are absolute so they resolve regardless of cwd.
    ///
    /// Resolution order is [`POLICY_DIR_ENV`] first, then the crate's
    /// manifest dir compiled into the binary. The env var wins because
    /// the compiled-in value cannot be right anywhere the source tree
    /// is not shipped, and a missing policy directory is a
    /// fail-closed crash rather than a degraded mode.
    #[must_use]
    pub fn bundled() -> Self {
        let base = std::env::var_os(POLICY_DIR_ENV).map_or_else(
            || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("policies"),
            |dir| {
                let dir = PathBuf::from(dir);
                tracing::debug!(
                    target: "ada_rbac_casbin::policy",
                    dir = %dir.display(),
                    "using policy directory from the environment"
                );
                dir
            },
        );
        Self {
            model_path: base.join("model.conf"),
            policy_path: base.join("base_policy.csv"),
            overlays: Vec::new(),
        }
    }

    pub fn validate(&self) -> Result<()> {
        check_readable(&self.model_path)?;
        check_readable(&self.policy_path)?;
        for o in &self.overlays {
            check_readable(o)?;
        }
        Ok(())
    }
}

fn check_readable(p: &Path) -> Result<()> {
    if !p.exists() {
        // Name the env var here, not just the path. The path alone is an
        // instruction the operator cannot follow: it points into whatever
        // tree the image happened to be built from.
        return Err(RbacCasbinError::ReloadFailed(format!(
            "policy file not found: {} (set {POLICY_DIR_ENV} to the \
             directory holding model.conf and base_policy.csv)",
            p.display()
        )));
    }
    let md = std::fs::metadata(p)
        .map_err(|e| RbacCasbinError::ReloadFailed(format!("metadata {}: {e}", p.display())))?;
    if md.permissions().readonly() && !p.is_file() {
        return Err(RbacCasbinError::ReloadFailed(format!(
            "not a regular file: {}",
            p.display()
        )));
    }
    Ok(())
}
