//! `audit_log` interface (skeleton).
//!
//! `DOC-MOD-011` §3.3 says every write that passes through M-11
//! must call [`record_audit_log`] before commit, persisting a
//! `before` / `after` JSON-Patch (RFC 6902) pair to the `audit_log`
//! table. The v0.1.0 skeleton does **not** persist; it just builds
//! the [`AuditLogEntry`] and hands it to a pluggable sink so that
//! the production layer can attach a `sqlx::PgPool`-backed sink
//! without changing call sites.

use std::sync::{Arc, Mutex};

use ada_core::{TenantId, UserId};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::collaboration::ResourceId;
use crate::permission::ResourceType;

/// A single audit log entry. The production build persists this
/// shape to the `audit_log` table (DDL in `DOC-MOD-010` §4.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditLogEntry {
    /// Tenant scope; `None` for system-level events.
    pub tenant_id: Option<TenantId>,
    /// Acting user.
    pub user_id: UserId,
    /// Free-form action type, e.g. `canvas.write`,
    /// `permission.grant`, `credential.access`.
    pub action_type: String,
    /// Resource type.
    pub resource_type: ResourceType,
    /// Resource id (the `audit_log.resource_id` column).
    pub resource_id: Uuid,
    /// Snapshot of the resource before the action, if known.
    pub before: Option<serde_json::Value>,
    /// Snapshot of the resource after the action, if known.
    pub after: Option<serde_json::Value>,
    /// Wall-clock timestamp in milliseconds since the UNIX epoch.
    pub timestamp_ms: u64,
}

/// A pluggable sink for audit log entries. The default in-memory
/// sink (used by the v0.1.0 skeleton) is [`InMemoryAuditSink`].
pub trait AuditSink: Send + Sync {
    /// Consume an entry. Errors are non-fatal — they should be
    /// logged but not propagated to the caller.
    fn record(&self, entry: AuditLogEntry);
}

/// Default [`AuditSink`] used by the v0.1.0 skeleton. Stores
/// entries in a `Vec` behind a `Mutex`. Useful for tests and
/// short-lived production tools (CLI / one-shot migrator).
#[derive(Debug, Default, Clone)]
pub struct InMemoryAuditSink {
    inner: Arc<Mutex<Vec<AuditLogEntry>>>,
}

impl InMemoryAuditSink {
    /// Empty sink.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Borrow all entries recorded so far.
    #[must_use]
    pub fn entries(&self) -> Vec<AuditLogEntry> {
        self.inner.lock().expect("poisoned").clone()
    }

    /// Number of recorded entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.lock().expect("poisoned").len()
    }

    /// True if no entries have been recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl AuditSink for InMemoryAuditSink {
    fn record(&self, entry: AuditLogEntry) {
        self.inner.lock().expect("poisoned").push(entry);
    }
}

/// Build a `before / after` audit log entry for a write action.
///
/// Real call sites will replace the timestamp / sink with whatever
/// the production stack expects; the v0.1.0 skeleton uses
/// `SystemTime::now().duration_since(UNIX_EPOCH).as_millis()` and
/// pushes to an [`InMemoryAuditSink`].
pub fn record_audit_log(
    sink: &dyn AuditSink,
    tenant_id: Option<TenantId>,
    user_id: UserId,
    action_type: impl Into<String>,
    resource: (ResourceType, ResourceId),
    before: Option<serde_json::Value>,
    after: Option<serde_json::Value>,
) {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX));
    sink.record(AuditLogEntry {
        tenant_id,
        user_id,
        action_type: action_type.into(),
        resource_type: resource.0,
        resource_id: resource.1 .0,
        before,
        after,
        timestamp_ms,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user_id() -> UserId {
        UserId(uuid::Uuid::new_v4())
    }

    #[test]
    fn in_memory_sink_records_entries() {
        let sink = InMemoryAuditSink::new();
        let r = ResourceId::new();
        record_audit_log(
            &sink,
            None,
            user_id(),
            "canvas.write",
            (ResourceType::Canvas, r),
            None,
            None,
        );
        assert_eq!(sink.len(), 1);
        let entries = sink.entries();
        assert_eq!(entries[0].action_type, "canvas.write");
        assert_eq!(entries[0].resource_id, r.0);
    }

    #[test]
    fn audit_log_entry_serde_roundtrip() {
        let r = ResourceId::new();
        let entry = AuditLogEntry {
            tenant_id: None,
            user_id: user_id(),
            action_type: "permission.grant".into(),
            resource_type: ResourceType::Canvas,
            resource_id: r.0,
            before: Some(serde_json::json!({"role": "viewer"})),
            after: Some(serde_json::json!({"role": "editor"})),
            timestamp_ms: 1_700_000_000_000,
        };
        let json = serde_json::to_string(&entry).expect("serialize");
        let back: AuditLogEntry = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, entry);
    }

    #[test]
    fn in_memory_sink_clone_shares_storage() {
        let s1 = InMemoryAuditSink::new();
        let s2 = s1.clone();
        let r = ResourceId::new();
        record_audit_log(
            &s1,
            None,
            user_id(),
            "x",
            (ResourceType::Canvas, r),
            None,
            None,
        );
        assert_eq!(s2.len(), 1);
    }
}
