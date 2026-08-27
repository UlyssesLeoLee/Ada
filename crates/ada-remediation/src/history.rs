//! In-process history + cooldown store.
//!
//! The persistent source of truth for history is the
//! `remediation_history` and `remediation_cooldowns` tables in
//! PostgreSQL (see `db/migrations/V003__phase8_remediation.sql`).
//! This `MemoryStore` is the *fast path* the executor consults
//! before the engine asks the DB. It is the only state the
//! unit tests in this crate can reason about without standing
//! up a real Postgres instance.
//!
//! The split is intentional: production code uses both layers
//! (memory then DB) and the integration test in
//! `tests/remediation_e2e.rs` exercises the memory layer only.

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

/// Filter applied to [`MemoryStore::query_history`]. Mirrors the
/// query parameters of the `GET /remediation/history` HTTP
/// endpoint, kept here so the binary and the test share a type.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct HistoryQuery {
    /// Restrict to this action id (exact match).
    pub action_id: Option<String>,
    /// Only include executions whose `executed_at >= since`.
    pub since: Option<DateTime<Utc>>,
    /// Maximum number of rows returned. `None` = unlimited.
    pub limit: Option<usize>,
}

/// One row of history. Mirrors the schema of
/// `remediation_history` in `V003__phase8_remediation.sql`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct HistoryRecord {
    pub id: u64,
    pub action_id: String,
    pub alert_name: String,
    pub executed_at: DateTime<Utc>,
    pub outcome: String,
    pub retry_count: u32,
    pub error_msg: Option<String>,
}

/// Thread-safe in-memory history + cooldown store.
///
/// `cooldown_until` is keyed by action id. `record_success` /
/// `record_failure` also push a row into the history vec. The
/// history vec is unbounded; a future task can swap it for a
/// ring buffer or a Postgres-backed writer.
#[derive(Debug, Default, Clone)]
pub struct MemoryStore {
    inner: Arc<RwLock<MemoryStoreInner>>,
}

#[derive(Debug, Default)]
struct MemoryStoreInner {
    cooldowns: HashMap<String, DateTime<Utc>>,
    history: Vec<HistoryRecord>,
    next_id: u64,
}

impl MemoryStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns `true` if `action_id` is currently in cooldown.
    /// The current wall-clock time is used; expired cooldowns
    /// are not eagerly evicted.
    #[must_use]
    pub fn is_in_cooldown(&self, action_id: &str) -> bool {
        let guard = self.inner.read();
        guard
            .cooldowns
            .get(action_id)
            .is_some_and(|until| *until > Utc::now())
    }

    /// Mark `action_id` as successfully executed, with a
    /// cooldown window of `cooldown` starting now.
    pub fn record_success(&self, action_id: &str, cooldown: Duration, alert_name: &str) {
        let mut guard = self.inner.write();
        let now = Utc::now();
        let until = now + chrono::Duration::from_std(cooldown).unwrap_or_default();
        guard.cooldowns.insert(action_id.to_string(), until);
        guard.next_id += 1;
        let id = guard.next_id;
        guard.history.push(HistoryRecord {
            id,
            action_id: action_id.to_string(),
            alert_name: alert_name.to_string(),
            executed_at: now,
            outcome: "succeeded".to_string(),
            retry_count: 0,
            error_msg: None,
        });
    }

    /// Mark `action_id` as failed, with a shorter (10s) cooldown
    /// so a flapping alert can be retried shortly without
    /// being permanently silenced. A failed execution does
    /// *not* sit in the full cooldown window — operators want
    /// the action to be eligible to retry quickly.
    pub fn record_failure(&self, action_id: &str, alert_name: &str, err: &str, retries: u32) {
        let mut guard = self.inner.write();
        let now = Utc::now();
        let until = now + chrono::Duration::seconds(10);
        guard.cooldowns.insert(action_id.to_string(), until);
        guard.next_id += 1;
        let id = guard.next_id;
        guard.history.push(HistoryRecord {
            id,
            action_id: action_id.to_string(),
            alert_name: alert_name.to_string(),
            executed_at: now,
            outcome: "failed".to_string(),
            retry_count: retries,
            error_msg: Some(err.to_string()),
        });
    }

    /// Query the history. Returns rows in **reverse chronological
    /// order** (newest first) so the dashboard's default
    /// `limit=50` returns the most recent executions.
    #[must_use]
    pub fn query_history(&self, q: &HistoryQuery) -> Vec<HistoryRecord> {
        let guard = self.inner.read();
        let mut rows: Vec<HistoryRecord> = guard
            .history
            .iter()
            .filter(|r| q.action_id.as_ref().is_none_or(|id| id == &r.action_id))
            .filter(|r| q.since.is_none_or(|s| r.executed_at >= s))
            .cloned()
            .collect();
        rows.sort_by_key(|r| std::cmp::Reverse(r.executed_at));
        if let Some(lim) = q.limit {
            rows.truncate(lim);
        }
        rows
    }

    /// Return every active cooldown as `(action_id, expires_at)`.
    /// Expired cooldowns are filtered out so the dashboard
    /// shows only the live set.
    #[must_use]
    pub fn active_cooldowns(&self) -> Vec<(String, DateTime<Utc>)> {
        let now = Utc::now();
        let guard = self.inner.read();
        let mut out: Vec<_> = guard
            .cooldowns
            .iter()
            .filter(|(_, until)| **until > now)
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }

    /// Number of records currently in the history vec. Used by
    /// the integration test as a basic invariant after
    /// executing a runbook.
    #[must_use]
    pub fn history_len(&self) -> usize {
        self.inner.read().history.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn success_sets_cooldown() {
        let store = MemoryStore::new();
        assert!(!store.is_in_cooldown("a"));
        store.record_success("a", Duration::from_secs(60), "DiskSpaceLow");
        assert!(store.is_in_cooldown("a"));
    }

    #[test]
    fn cooldown_expires() {
        let store = MemoryStore::new();
        store.record_success("a", Duration::from_millis(50), "DiskSpaceLow");
        assert!(store.is_in_cooldown("a"));
        thread::sleep(Duration::from_millis(80));
        assert!(!store.is_in_cooldown("a"));
    }

    #[test]
    fn failure_uses_short_cooldown() {
        let store = MemoryStore::new();
        store.record_failure("a", "DiskSpaceLow", "boom", 0);
        assert!(store.is_in_cooldown("a"));
        thread::sleep(Duration::from_millis(20));
        // 10s cooldown still active
        assert!(store.is_in_cooldown("a"));
    }

    #[test]
    fn history_query_newest_first() {
        let store = MemoryStore::new();
        store.record_success("a", Duration::from_secs(60), "Alert1");
        thread::sleep(Duration::from_millis(5));
        store.record_success("b", Duration::from_secs(60), "Alert2");
        let q = HistoryQuery::default();
        let rows = store.query_history(&q);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].action_id, "b");
        assert_eq!(rows[1].action_id, "a");
    }

    #[test]
    fn history_query_filter_by_action() {
        let store = MemoryStore::new();
        store.record_success("a", Duration::from_secs(60), "Alert1");
        store.record_success("b", Duration::from_secs(60), "Alert2");
        let q = HistoryQuery {
            action_id: Some("a".into()),
            ..Default::default()
        };
        let rows = store.query_history(&q);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].action_id, "a");
    }
}
