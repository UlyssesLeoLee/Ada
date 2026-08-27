//! Runbook file loading.
//!
//! Each `config/remediation/*.json` file is a `RunbookFile` —
//! a top-level object that contains one or more actions.
//! Multiple actions per file is allowed so related runbooks
//! (e.g. `disk-space-low.yaml` and `disk-space-cleanup-verbose`)
//! can share a single source.

use crate::action::RemediationAction;
use crate::error::{RemediationError, Result};
use std::path::Path;

/// One runbook file. The top-level shape is:
///
/// ```json
/// {
///   "version": 1,
///   "actions": [ /* RemediationAction ... */ ]
/// }
/// ```
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct RunbookFile {
    /// Schema version. Currently `1`.
    pub version: u32,
    /// Actions declared in this file.
    pub actions: Vec<RemediationAction>,
}

impl RunbookFile {
    /// Read a runbook file from disk and parse it.
    pub fn from_path(path: &Path) -> Result<Self> {
        let bytes = std::fs::read(path)?;
        let file: Self = serde_json::from_slice(&bytes)?;
        if file.version != 1 {
            return Err(RemediationError::InvalidRunbook(format!(
                "unsupported runbook schema version {} (expected 1)",
                file.version
            )));
        }
        Ok(file)
    }
}

/// Load every `*.json` runbook in `dir` and concatenate the
/// actions into one flat `Vec`. Non-recursive by design — the
/// `config/remediation/` directory is intentionally flat.
pub fn load_runbooks_from_dir(dir: &Path) -> Result<Vec<RemediationAction>> {
    let mut actions = Vec::new();
    if !dir.exists() {
        return Ok(actions);
    }
    let entries = std::fs::read_dir(dir)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let file = RunbookFile::from_path(&path)?;
        actions.extend(file.actions);
    }
    Ok(actions)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_runbook() {
        let json = br##"{
            "version": 1,
            "actions": [
                {
                    "id": "test-action",
                    "name": "Test",
                    "trigger": "TestAlert",
                    "steps": [
                        { "kind": "notify_slack", "channel": "#ada-test", "message": "hi" }
                    ],
                    "cooldown": 60,
                    "max_retries": 0
                }
            ]
        }"##;
        let file: RunbookFile = serde_json::from_slice(json).unwrap();
        assert_eq!(file.version, 1);
        assert_eq!(file.actions.len(), 1);
        assert_eq!(file.actions[0].id, "test-action");
    }

    #[test]
    fn rejects_wrong_schema_version() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.json");
        std::fs::write(&path, br#"{ "version": 99, "actions": [] }"#).unwrap();
        let err = RunbookFile::from_path(&path).unwrap_err();
        assert!(matches!(err, RemediationError::InvalidRunbook(_)));
    }

    #[test]
    fn loads_all_json_in_dir() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("a.json"),
            br#"{ "version": 1, "actions": [{ "id": "a", "trigger": "A", "steps": [] }] }"#,
        )
        .unwrap();
        std::fs::write(
            dir.path().join("b.json"),
            br#"{ "version": 1, "actions": [{ "id": "b", "trigger": "B", "steps": [] }] }"#,
        )
        .unwrap();
        // Non-JSON file is skipped.
        std::fs::write(dir.path().join("README.md"), b"ignore me").unwrap();
        let actions = load_runbooks_from_dir(dir.path()).unwrap();
        assert_eq!(actions.len(), 2);
        let ids: Vec<_> = actions.iter().map(|a| a.id.clone()).collect();
        assert!(ids.contains(&"a".to_string()));
        assert!(ids.contains(&"b".to_string()));
    }
}
