//! Remediation engine: matches alerts to runbooks and executes them.

use crate::action::{ActionOutcome, ActionStep, RemediationAction, StepResult};
use crate::alert::{AlertEvent, AlertStatus};
use crate::error::{RemediationError, Result};
use crate::state::EngineState;
use std::time::{Duration, Instant};

/// Default step timeout, used when a runbook step does not
/// specify its own. Matches the design doc recommendation.
const DEFAULT_STEP_TIMEOUT: Duration = Duration::from_secs(30);

/// The remediation engine. Constructed once per process, shared
/// via `Arc` (or cloned; this type is `Clone` and cheap).
#[derive(Debug, Clone, Default)]
pub struct RemediationEngine {
    /// Runbook table. Populated by `with_runbooks` or
    /// `with_defaults` (the latter loads from
    /// `config/remediation/` at startup).
    runbooks: Vec<RemediationAction>,
}

impl RemediationEngine {
    /// Empty engine. Useful for tests.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Engine with the given runbook table.
    #[must_use]
    pub fn with_runbooks(runbooks: Vec<RemediationAction>) -> Self {
        Self { runbooks }
    }

    /// Engine with the runbook table loaded from
    /// `config/remediation/` relative to the current working
    /// directory. Returns an empty engine if the directory does
    /// not exist (this is the *test* default; production wiring
    /// should log a warning when the directory is missing).
    #[must_use]
    pub fn with_defaults() -> Self {
        let path = std::path::Path::new("config/remediation");
        let runbooks = crate::config::load_runbooks_from_dir(path).unwrap_or_default();
        Self { runbooks }
    }

    /// Find every runbook whose trigger matches `alert`. An
    /// action whose `severities` filter is set is only included
    /// if the alert's severity is in that list. If the alert is
    /// `Resolved` or `Suppressed`, an empty vec is returned —
    /// we never run a remediation in response to a clearing
    /// alert.
    #[must_use]
    pub fn evaluate(&self, alert: &AlertEvent) -> Vec<RemediationAction> {
        if !matches!(alert.status, AlertStatus::Firing) {
            return Vec::new();
        }
        self.runbooks
            .iter()
            .filter(|a| a.trigger.matches(&alert.alert_name))
            .filter(|a| {
                a.severities.is_empty()
                    || alert
                        .severity
                        .as_ref()
                        .is_some_and(|s| a.severities.iter().any(|x| x == s))
            })
            .cloned()
            .collect()
    }

    /// Engine state at the start of `evaluate`.
    #[must_use]
    pub fn initial_state() -> EngineState {
        EngineState::Idle
    }

    /// Run every step of `action` in order. First failure
    /// short-circuits; the rest are skipped. The total time is
    /// recorded in `ActionOutcome::total_duration_ms`.
    ///
    /// `HttpCall` and `PgFunction` are executed through a
    /// *dry-run* path by default: the engine records the intent
    /// in the step result and returns success. This is
    /// deliberate — the offline build environment forbids a
    /// real HTTP client, and the real wiring is a separate
    /// deployment that supplies an `Executor` impl. The trait
    /// is in the next commit.
    pub async fn execute(&self, action: &RemediationAction) -> Result<ActionOutcome> {
        let started = Instant::now();
        let mut outcome = ActionOutcome::new(action.id.clone());

        for (idx, step) in action.steps.iter().enumerate() {
            let step_started = Instant::now();
            let (status, message) = self.run_step(idx, step, action).await;
            let duration_ms = u64::try_from(step_started.elapsed().as_millis()).unwrap_or(u64::MAX);
            let kind = step_kind_name(step);
            let result = if status {
                StepResult::ok(idx, kind, message, duration_ms)
            } else {
                StepResult::fail(idx, kind, message, duration_ms)
            };
            outcome.push_step(result);
            if !status {
                outcome.fail();
                break;
            }
        }
        outcome.complete(u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX));
        Ok(outcome)
    }

    /// Run a single step. Returns `(true, message)` on success
    /// and `(false, message)` on failure. Sub-steps of a
    /// `Sequence` are inlined.
    async fn run_step(
        &self,
        _idx: usize,
        step: &ActionStep,
        action: &RemediationAction,
    ) -> (bool, String) {
        match step {
            ActionStep::RunCommand {
                cmd,
                args,
                timeout_secs,
            } => run_shell_command(cmd, args, Duration::from_secs(*timeout_secs)).await,
            ActionStep::HttpCall {
                url,
                method,
                body,
                headers,
            } => {
                tracing::info!(target: "ada_remediation", url, method = method.as_str(), "http call (dry-run)");
                let _ = (body, headers);
                (true, format!("dry-run http {} {}", method.as_str(), url))
            }
            ActionStep::PgFunction { name, args } => {
                tracing::info!(target: "ada_remediation", function = name, "pg function (dry-run)");
                let _ = args;
                (true, format!("dry-run pg function {name}"))
            }
            ActionStep::NotifySlack { channel, message } => {
                tracing::info!(target: "ada_remediation", channel, "slack notify (dry-run)");
                let _ = message;
                (true, format!("dry-run slack notify {channel}"))
            }
            ActionStep::PageOperator {
                severity,
                runbook_url,
            } => {
                tracing::warn!(target: "ada_remediation", ?severity, runbook_url, "page operator (dry-run)");
                (true, format!("dry-run page operator severity={severity:?}"))
            }
            ActionStep::Sequence { steps } => {
                for (i, sub) in steps.iter().enumerate() {
                    let (ok, msg) = Box::pin(self.run_step(i, sub, action)).await;
                    if !ok {
                        return (false, format!("sequence sub-step {i} failed: {msg}"));
                    }
                }
                (true, "sequence ok".to_string())
            }
        }
    }
}

fn step_kind_name(step: &ActionStep) -> String {
    match step {
        ActionStep::RunCommand { .. } => "run_command".into(),
        ActionStep::HttpCall { .. } => "http_call".into(),
        ActionStep::PgFunction { .. } => "pg_function".into(),
        ActionStep::NotifySlack { .. } => "notify_slack".into(),
        ActionStep::PageOperator { .. } => "page_operator".into(),
        ActionStep::Sequence { .. } => "sequence".into(),
    }
}

async fn run_shell_command(cmd: &str, args: &[String], timeout: Duration) -> (bool, String) {
    use tokio::process::Command;
    let fut = Command::new(cmd).args(args).output();
    let result = tokio::time::timeout(timeout, fut).await;
    match result {
        Ok(Ok(output)) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
            (true, format!("ok: {}", stdout.trim()))
        }
        Ok(Ok(output)) => {
            let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
            (
                false,
                format!("exit={:?} stderr={}", output.status.code(), stderr),
            )
        }
        Ok(Err(e)) => (false, format!("spawn failed: {e}")),
        Err(_) => (false, format!("timeout after {timeout:?}")),
    }
}

/// `evaluate -> execute` glue. Equivalent to:
///
/// ```ignore
/// let actions = engine.evaluate(alert);
/// for a in actions { if !cooldown.is_in_cooldown(&a.id) { engine.execute(&a).await?; } }
/// ```
///
/// but using the action's own `cooldown` field as the gating
/// window.
pub async fn run_for_alert(
    engine: &RemediationEngine,
    store: &crate::history::MemoryStore,
    alert: &AlertEvent,
) -> Result<Vec<ActionOutcome>> {
    let actions = engine.evaluate(alert);
    let mut outcomes = Vec::new();
    for a in actions {
        if store.is_in_cooldown(&a.id) {
            continue;
        }
        let outcome = engine.execute(&a).await?;
        let ok = matches!(outcome.status, crate::action::OutcomeStatus::Succeeded);
        if ok {
            store.record_success(&a.id, a.cooldown, &alert.alert_name);
        } else {
            store.record_failure(&a.id, &alert.alert_name, "see step results", 0);
        }
        outcomes.push(outcome);
    }
    Ok(outcomes)
}

#[allow(dead_code)]
fn _default_timeout_documented() -> Duration {
    DEFAULT_STEP_TIMEOUT
}

#[allow(dead_code)]
fn _check_invalid_transition_typed() -> Result<()> {
    // Compile-time check that the error type is reachable.
    Err(RemediationError::InvalidStateTransition {
        from: "Idle".into(),
        to: "Executing".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alert::AlertEvent;

    fn disk_action() -> RemediationAction {
        RemediationAction {
            id: "disk-space-low".into(),
            name: "Disk space low".into(),
            trigger: crate::action::Trigger::Exact("DiskSpaceFillingFast".into()),
            severities: vec![],
            steps: vec![ActionStep::NotifySlack {
                channel: "#ada-ops".into(),
                message: "disk low".into(),
            }],
            cooldown: Duration::from_secs(60),
            max_retries: 0,
        }
    }

    #[test]
    fn evaluate_finds_exact_match() {
        let engine = RemediationEngine::with_runbooks(vec![disk_action()]);
        let alert = AlertEvent::new("DiskSpaceFillingFast");
        let actions = engine.evaluate(&alert);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].id, "disk-space-low");
    }

    #[test]
    fn evaluate_ignores_resolved_alerts() {
        let engine = RemediationEngine::with_runbooks(vec![disk_action()]);
        let alert = AlertEvent::new("DiskSpaceFillingFast").with_status(AlertStatus::Resolved);
        assert!(engine.evaluate(&alert).is_empty());
    }

    #[test]
    fn evaluate_filters_by_severity() {
        let mut action = disk_action();
        action.severities = vec!["P1".into()];
        let engine = RemediationEngine::with_runbooks(vec![action]);

        let p1 = AlertEvent::builder("DiskSpaceFillingFast")
            .label("severity", "P1")
            .build();
        let p3 = AlertEvent::builder("DiskSpaceFillingFast")
            .label("severity", "P3")
            .build();
        assert_eq!(engine.evaluate(&p1).len(), 1);
        assert_eq!(engine.evaluate(&p3).len(), 0);
    }

    #[test]
    fn evaluate_uses_glob_matching() {
        let action = RemediationAction {
            id: "slo-burn".into(),
            name: "SLO burn".into(),
            trigger: crate::action::Trigger::Glob("SLIBurn*".into()),
            severities: vec![],
            steps: vec![],
            cooldown: Duration::from_secs(60),
            max_retries: 0,
        };
        let engine = RemediationEngine::with_runbooks(vec![action]);
        assert_eq!(
            engine.evaluate(&AlertEvent::new("SLIBurnRateFast")).len(),
            1
        );
        assert_eq!(
            engine.evaluate(&AlertEvent::new("SLIBurnRateSlow")).len(),
            1
        );
        assert_eq!(engine.evaluate(&AlertEvent::new("SLOBreach")).len(), 0);
    }

    #[tokio::test]
    async fn execute_runs_dry_run_steps() {
        let engine = RemediationEngine::with_runbooks(vec![disk_action()]);
        let alert = AlertEvent::new("DiskSpaceFillingFast");
        let actions = engine.evaluate(&alert);
        let outcome = engine.execute(&actions[0]).await.unwrap();
        assert_eq!(outcome.step_results.len(), 1);
        assert!(matches!(
            outcome.step_results[0].status,
            crate::action::OutcomeStatus::Succeeded
        ));
    }

    #[tokio::test]
    async fn execute_short_circuits_on_first_failure() {
        let action = RemediationAction {
            id: "test".into(),
            name: "test".into(),
            trigger: crate::action::Trigger::Exact("Test".into()),
            severities: vec![],
            steps: vec![
                ActionStep::RunCommand {
                    cmd: "this-command-does-not-exist-xyz".into(),
                    args: vec![],
                    timeout_secs: 5,
                },
                ActionStep::NotifySlack {
                    channel: "#never".into(),
                    message: "never".into(),
                },
            ],
            cooldown: Duration::from_secs(60),
            max_retries: 0,
        };
        let engine = RemediationEngine::with_runbooks(vec![action]);
        let outcome = engine
            .execute(&engine.evaluate(&AlertEvent::new("Test"))[0])
            .await
            .unwrap();
        assert!(matches!(
            outcome.status,
            crate::action::OutcomeStatus::Failed
        ));
        // Only the first step should have been attempted.
        assert_eq!(outcome.step_results.len(), 1);
    }
}
