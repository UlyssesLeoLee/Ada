//! Static policy loaded from `crates/ada-rbac-casbin/policies/`. The
//! v0.4.0 skeleton reads `base_policy.csv` (the m11 role × permission
//! matrix regenerated as Casbin-style rows). The on-disk layout is
//! the same as Casbin's so the v0.5.0 swap-in is mechanical.
//!
//! v0.5.0 anchors the bundled path to `CARGO_MANIFEST_DIR` so tests
//! (which run with cwd = `crates/ada-rbac-casbin/`) and the
//! api-gateway binary (cwd = workspace root) both find the policy
//! files without an env var.

use std::path::{Path, PathBuf};

use crate::error::{RbacCasbinError, Result};

#[derive(Debug, Clone)]
pub struct PolicySet {
    pub model_path: PathBuf,
    pub policy_path: PathBuf,
    pub overlays: Vec<PathBuf>,
}

impl PolicySet {
    /// Bundled policy files relative to this crate's manifest dir.
    /// The paths are absolute so they resolve regardless of cwd.
    #[must_use]
    pub fn bundled() -> Self {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("policies");
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
        return Err(RbacCasbinError::ReloadFailed(format!(
            "policy file not found: {}",
            p.display()
        )));
    }
    let md = std::fs::metadata(p).map_err(|e| {
        RbacCasbinError::ReloadFailed(format!("metadata {}: {e}", p.display()))
    })?;
    if md.permissions().readonly() && !p.is_file() {
        return Err(RbacCasbinError::ReloadFailed(format!(
            "not a regular file: {}",
            p.display()
        )));
    }
    Ok(())
}