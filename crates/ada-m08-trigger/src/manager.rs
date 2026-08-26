//! Trigger manager: in-process storage + event-topic matching.

use parking_lot::Mutex;
use std::collections::HashMap;

use crate::error::{Result, TriggerError};
use crate::rule::{TriggerId, TriggerRule};

/// Number of whitespace-separated fields expected in a cron
/// expression. The v0.1.0 skeleton uses 5 (minute / hour / dom
/// / month / dow). B7+ may extend to 6 (with seconds) when the
/// `cron` crate is added.
pub const DEFAULT_CRON_FIELDS: usize = 5;

/// In-process trigger registry.
#[derive(Debug, Default)]
pub struct TriggerManager {
    rules: Mutex<HashMap<TriggerId, TriggerRule>>,
}

impl TriggerManager {
    /// Create an empty manager.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a new rule. Returns the rule's `id`. Errors with
    /// `DuplicateId` if a rule with the same id is already
    /// present.
    pub fn add(&self, rule: TriggerRule) -> Result<TriggerId> {
        let mut rules = self.rules.lock();
        if rules.contains_key(&rule.id) {
            return Err(TriggerError::DuplicateId(rule.id));
        }
        let id = rule.id;
        rules.insert(id, rule);
        Ok(id)
    }

    /// Remove a rule by id.
    pub fn remove(&self, id: TriggerId) -> Result<()> {
        self.rules
            .lock()
            .remove(&id)
            .map(|_| ())
            .ok_or(TriggerError::TriggerNotFound(id))
    }

    /// Snapshot of all current rules.
    #[must_use]
    pub fn list(&self) -> Vec<TriggerRule> {
        self.rules.lock().values().cloned().collect()
    }

    /// Enable or disable a rule.
    pub fn set_enabled(&self, id: TriggerId, enabled: bool) -> Result<()> {
        let mut rules = self.rules.lock();
        let rule = rules
            .get_mut(&id)
            .ok_or(TriggerError::TriggerNotFound(id))?;
        rule.enabled = enabled;
        Ok(())
    }

    /// Look up a rule by id (clone).
    #[must_use]
    pub fn get(&self, id: TriggerId) -> Option<TriggerRule> {
        self.rules.lock().get(&id).cloned()
    }

    /// Match `topic` against every enabled `Event` trigger. The
    /// match is a literal prefix comparison plus optional
    /// `*` (one segment) and `#` (zero or more segments) globs
    /// (same semantics as M-15 Kafka-style topic match). Returns
    /// the ids of matched triggers.
    pub fn match_event(&self, topic: &str) -> Vec<TriggerId> {
        self.rules
            .lock()
            .values()
            .filter(|r| {
                r.enabled
                    && matches!(r.kind, crate::rule::TriggerKind::Event)
                    && topic_matches(&r.schedule, topic)
            })
            .map(|r| r.id)
            .collect()
    }
}

/// Lightweight topic match: literal equality, or `prefix.*`
/// (one segment wildcard), or `prefix.#` (zero or more
/// segments). Segments are dot-separated.
fn topic_matches(pattern: &str, topic: &str) -> bool {
    if pattern == topic {
        return true;
    }
    if let Some(prefix) = pattern.strip_suffix(".*") {
        if let Some(rest) = topic.strip_prefix(prefix) {
            return rest.starts_with('.') && !rest[1..].contains('.');
        }
        return false;
    }
    if let Some(prefix) = pattern.strip_suffix(".#") {
        return topic == prefix || topic.starts_with(&format!("{prefix}."));
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rule::{Action, TriggerKind, TriggerRule};
    use serde_json::json;

    fn ev(name: &str, topic: &str) -> TriggerRule {
        TriggerRule::new(name, TriggerKind::Event, topic, Action::new("x", json!({}))).expect("ok")
    }

    #[test]
    fn add_remove_round_trip() {
        let m = TriggerManager::new();
        let r = ev("e1", "module.*");
        let id = m.add(r).expect("add");
        assert_eq!(m.list().len(), 1);
        m.remove(id).expect("remove");
        assert!(m.list().is_empty());
    }

    #[test]
    fn remove_unknown_errors() {
        let m = TriggerManager::new();
        let err = m.remove(TriggerId::new()).unwrap_err();
        assert!(matches!(err, TriggerError::TriggerNotFound(_)));
    }

    #[test]
    fn duplicate_add_errors() {
        let m = TriggerManager::new();
        let r = ev("e1", "x");
        let id = r.id;
        m.add(r).expect("first add");
        let r2 = ev("e2", "y");
        let mut r2 = r2;
        r2.id = id;
        let err = m.add(r2).unwrap_err();
        assert!(matches!(err, TriggerError::DuplicateId(_)));
    }

    #[test]
    fn set_enabled_unknown_errors() {
        let m = TriggerManager::new();
        let err = m.set_enabled(TriggerId::new(), true).unwrap_err();
        assert!(matches!(err, TriggerError::TriggerNotFound(_)));
    }

    #[test]
    fn set_enabled_toggles() {
        let m = TriggerManager::new();
        let r = ev("e1", "x");
        let id = m.add(r).expect("add");
        m.set_enabled(id, false).expect("disable");
        assert!(!m.get(id).expect("rule").enabled);
        m.set_enabled(id, true).expect("enable");
        assert!(m.get(id).expect("rule").enabled);
    }

    #[test]
    fn match_event_literal() {
        let m = TriggerManager::new();
        let r = ev("e1", "module.registered");
        m.add(r).expect("add");
        let matched = m.match_event("module.registered");
        assert_eq!(matched.len(), 1);
    }

    #[test]
    fn match_event_star_glob() {
        let m = TriggerManager::new();
        let r = ev("e1", "module.*");
        m.add(r).expect("add");
        assert_eq!(m.match_event("module.registered").len(), 1);
        assert_eq!(m.match_event("module.removed").len(), 1);
        assert!(m.match_event("module.x.y").is_empty());
        assert!(m.match_event("other.registered").is_empty());
    }

    #[test]
    fn match_event_hash_glob() {
        let m = TriggerManager::new();
        let r = ev("e1", "module.#");
        m.add(r).expect("add");
        assert_eq!(m.match_event("module.registered").len(), 1);
        assert_eq!(m.match_event("module.x.y").len(), 1);
        assert!(m.match_event("other").is_empty());
    }

    #[test]
    fn match_event_skips_disabled() {
        let m = TriggerManager::new();
        let r = ev("e1", "module.*");
        let id = m.add(r).expect("add");
        m.set_enabled(id, false).expect("disable");
        assert!(m.match_event("module.registered").is_empty());
    }

    #[test]
    fn match_event_skips_non_event_kinds() {
        let m = TriggerManager::new();
        let r = TriggerRule::new("c1", TriggerKind::Manual, "", Action::new("x", json!({})))
            .expect("ok");
        m.add(r).expect("add");
        assert!(m.match_event("anything").is_empty());
    }
}
