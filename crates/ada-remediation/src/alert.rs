//! Alert event model.
//!
//! Mirrors the Alertmanager v4 webhook payload shape, but stripped
//! down to the fields the engine actually consumes. Label values are
//! free-form strings so that any upstream (Alertmanager, Grafana
//! unified alerting, even a synthetic test) can feed the engine
//! without an adapter.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// One incoming alert (or one element of an Alertmanager
/// `alerts[].alerts` array). We accept both flat (single alert
/// wrapped in an envelope) and the canonical Alertmanager shape.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AlertEvent {
    /// Alertmanager `labels.alertname` — this is the trigger
    /// the engine matches against. Equality match is used by
    /// default; substring match is opt-in via
    /// [`RemediationAction::trigger`].
    pub alert_name: String,

    /// `firing` or `resolved`. The engine only acts on `firing`;
    /// resolved alerts are no-ops (and the dispatch is recorded
    /// for traceability but no runbook steps are executed).
    pub status: AlertStatus,

    /// Severity label (e.g. `P1`, `P2`, `P3`). Optional — many
    /// runbooks are severity-agnostic and just want the trigger.
    pub severity: Option<String>,

    /// All other labels (`service`, `cluster`, `instance`, ...).
    /// Stored as a `BTreeMap` so the JSON is deterministic and
    /// unit tests can assert exact key sets.
    #[serde(default)]
    pub labels: BTreeMap<String, String>,

    /// Annotation bag. Not used for matching but threaded through
    /// to the executor so runbook steps can interpolate
    /// `{{ $labels.service }}` etc. when templating messages.
    #[serde(default)]
    pub annotations: BTreeMap<String, String>,

    /// Optional externally supplied id (Alertmanager `fingerprint`).
    /// Used by the persistent history table as a correlation key.
    #[serde(default)]
    pub fingerprint: Option<String>,
}

impl AlertEvent {
    /// Convenience constructor for tests and call sites that only
    /// care about the alert name.
    #[must_use]
    pub fn new(alert_name: impl Into<String>) -> Self {
        Self {
            alert_name: alert_name.into(),
            status: AlertStatus::Firing,
            severity: None,
            labels: BTreeMap::new(),
            annotations: BTreeMap::new(),
            fingerprint: None,
        }
    }

    /// Builder-style entry point. Mirrors the test fixtures:
    /// `AlertEvent::builder("ServiceDown").label("severity", "P1")...`.
    #[must_use]
    pub fn builder(alert_name: impl Into<String>) -> Self {
        Self::new(alert_name)
    }

    /// Insert a label. Returns `self` for chaining.
    #[must_use]
    pub fn label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
        // Auto-populate `severity` from a label of the same name
        // so existing Alertmanager payloads work without an
        // explicit `severity` field on the wire.
        if self.severity.is_none() && self.labels.contains_key("severity") {
            self.severity = Some(self.labels["severity"].clone());
        }
        self
    }

    /// Insert an annotation. Returns `self` for chaining.
    #[must_use]
    pub fn annotation(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.annotations.insert(key.into(), value.into());
        self
    }

    /// Override status. Defaults to `Firing`.
    #[must_use]
    pub fn with_status(mut self, status: AlertStatus) -> Self {
        self.status = status;
        self
    }

    /// Finalise the builder.
    #[must_use]
    pub fn build(self) -> Self {
        self
    }

    /// Convenience: render `{{ $labels.X }}` style template
    /// references against the current label bag. Used by
    /// `ActionStep::HttpCall` and `ActionStep::NotifySlack`.
    ///
    /// Unknown `{{ $labels.X }}` placeholders are left in place
    /// rather than dropped — this makes it obvious in the
    /// destination message that a label was missing.
    #[must_use]
    pub fn render_template(&self, template: &str) -> String {
        let mut out = template.to_string();
        for (k, v) in &self.labels {
            let needle = format!("{{{{ $labels.{k} }}}}");
            out = out.replace(&needle, v);
        }
        out
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AlertStatus {
    Firing,
    Resolved,
    Suppressed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_populates_severity_from_label() {
        let alert = AlertEvent::builder("DiskSpaceFillingFast")
            .label("severity", "P2")
            .label("service", "m13-api-gateway")
            .build();
        assert_eq!(alert.alert_name, "DiskSpaceFillingFast");
        assert_eq!(alert.severity.as_deref(), Some("P2"));
        assert_eq!(
            alert.labels.get("service").map(String::as_str),
            Some("m13-api-gateway")
        );
    }

    #[test]
    fn resolved_alerts_are_first_class() {
        let alert = AlertEvent::builder("ServiceDown")
            .with_status(AlertStatus::Resolved)
            .build();
        assert_eq!(alert.status, AlertStatus::Resolved);
    }

    #[test]
    fn template_renderer_substitutes_known_labels() {
        let alert = AlertEvent::builder("ServiceDown")
            .label("service", "m03-data-flow-engine")
            .label("cluster", "prod-us-east-1")
            .build();
        let rendered = alert.render_template(
            "service={{ $labels.service }} cluster={{ $labels.cluster }} missing={{ $labels.absent }}",
        );
        assert_eq!(
            rendered,
            "service=m03-data-flow-engine cluster=prod-us-east-1 missing={{ $labels.absent }}"
        );
    }
}
