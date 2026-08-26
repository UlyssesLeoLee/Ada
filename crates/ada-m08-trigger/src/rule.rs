//! Trigger rule model.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{Result, TriggerError};

/// Stable, opaque identifier for a trigger.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TriggerId(pub Uuid);

impl TriggerId {
    /// Create a fresh random id.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for TriggerId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TriggerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The four canonical trigger kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TriggerKind {
    /// Schedule-based (cron). `schedule` is the cron expression
    /// (5 fields: minute / hour / dom / month / dow).
    Cron,
    /// Inbound HTTP webhook. `schedule` is the URL path (e.g.
    /// `/hooks/flow/<id>`).
    Webhook,
    /// Event-driven (matches against the central event bus).
    /// `schedule` is the topic glob (e.g. `module.*.registered`).
    Event,
    /// Manually invoked via the API. `schedule` is unused.
    Manual,
}

impl std::fmt::Display for TriggerKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Cron => "cron",
            Self::Webhook => "webhook",
            Self::Event => "event",
            Self::Manual => "manual",
        };
        f.write_str(s)
    }
}

/// The action a trigger invokes. The skeleton treats the
/// payload as a JSON value; the kind is a free-form string.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Action {
    /// Logical kind (e.g. "run_flow", "export_metrics").
    pub kind: String,
    /// JSON payload.
    pub payload: serde_json::Value,
}

impl Action {
    /// Create a new action.
    #[must_use]
    pub fn new(kind: impl Into<String>, payload: serde_json::Value) -> Self {
        Self {
            kind: kind.into(),
            payload,
        }
    }
}

/// A trigger rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TriggerRule {
    /// Stable id.
    pub id: TriggerId,
    /// Human-readable name.
    pub name: String,
    /// Trigger kind.
    pub kind: TriggerKind,
    /// Schedule / topic glob / URL path. Required for
    /// `Cron`, `Webhook`, `Event`; ignored for `Manual`.
    pub schedule: String,
    /// Action to invoke.
    pub action: Action,
    /// Enabled flag. Disabled triggers do not fire.
    pub enabled: bool,
}

impl TriggerRule {
    /// Create a new enabled rule. Validates `schedule` for
    /// `Cron` (must be 5 whitespace-separated fields) and
    /// `Event` (must be non-empty).
    pub fn new(
        name: impl Into<String>,
        kind: TriggerKind,
        schedule: impl Into<String>,
        action: Action,
    ) -> Result<Self> {
        let schedule = schedule.into();
        match kind {
            TriggerKind::Cron => {
                let n = schedule.split_whitespace().count();
                if n != super::manager::DEFAULT_CRON_FIELDS {
                    return Err(TriggerError::InvalidCron(format!(
                        "expected {} fields, got {n}",
                        super::manager::DEFAULT_CRON_FIELDS
                    )));
                }
            }
            TriggerKind::Webhook | TriggerKind::Event => {
                if schedule.is_empty() {
                    return Err(TriggerError::InvalidCron(format!(
                        "{kind}: schedule is empty"
                    )));
                }
            }
            TriggerKind::Manual => {}
        }
        Ok(Self {
            id: TriggerId::new(),
            name: name.into(),
            kind,
            schedule,
            action,
            enabled: true,
        })
    }

    /// Enable the trigger.
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Disable the trigger.
    pub fn disable(&mut self) {
        self.enabled = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn action() -> Action {
        Action::new("run_flow", json!({"id": "f1"}))
    }

    #[test]
    fn trigger_id_unique() {
        assert_ne!(TriggerId::new(), TriggerId::new());
    }

    #[test]
    fn trigger_kind_display() {
        assert_eq!(TriggerKind::Cron.to_string(), "cron");
        assert_eq!(TriggerKind::Webhook.to_string(), "webhook");
        assert_eq!(TriggerKind::Event.to_string(), "event");
        assert_eq!(TriggerKind::Manual.to_string(), "manual");
    }

    #[test]
    fn cron_must_have_5_fields() {
        let err = TriggerRule::new("t", TriggerKind::Cron, "* * *", action()).unwrap_err();
        let s = err.to_string();
        assert!(s.contains("5 fields"), "got: {s}");
    }

    #[test]
    fn cron_with_5_fields_ok() {
        TriggerRule::new("t", TriggerKind::Cron, "*/5 * * * *", action()).expect("ok");
    }

    #[test]
    fn webhook_must_have_nonempty_schedule() {
        let err = TriggerRule::new("t", TriggerKind::Webhook, "", action()).unwrap_err();
        assert!(matches!(err, TriggerError::InvalidCron(_)));
    }

    #[test]
    fn event_must_have_nonempty_schedule() {
        let err = TriggerRule::new("t", TriggerKind::Event, "", action()).unwrap_err();
        assert!(matches!(err, TriggerError::InvalidCron(_)));
    }

    #[test]
    fn manual_skips_schedule_validation() {
        TriggerRule::new("t", TriggerKind::Manual, "", action()).expect("ok");
    }

    #[test]
    fn enable_disable_toggles() {
        let mut r = TriggerRule::new("t", TriggerKind::Manual, "", action()).expect("ok");
        assert!(r.enabled);
        r.disable();
        assert!(!r.enabled);
        r.enable();
        assert!(r.enabled);
    }
}
