//! Remediation engine: matches alerts to runbooks and executes them.

use crate::action::{ActionOutcome, ActionStep, RemediationAction, StepResult};
use crate::alert::{AlertEvent, AlertStatus};
use crate::error::{RemediationError, Result};
use crate::executor::{DryRunExecutor, ExecutionContext, StepExecutor};
use crate::state::EngineState;
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Default step timeout, used when a runbook step does not
/// specify its own. Matches the design doc recommendation.
const DEFAULT_STEP_TIMEOUT: Duration = Duration::from_secs(30);

/// The remediation engine. Constructed once per process, shared
/// via `Arc` (or cloned; this type is `Clone` and cheap).
pub struct RemediationEngine {
    /// Runbook table. Populated by `with_runbooks` or
    /// `with_defaults` (the latter loads from
    /// `config/remediation/` at startup). Wrapped in an
    /// `RwLock` so the v0.7.0
    /// [`crate::watcher::Watcher`] can swap it via
    /// [`RemediationEngine::reload_runbooks`] without
    /// forcing every read path to take a `Mutex` on
    /// `&mut self`.
    runbooks: Arc<RwLock<Vec<RemediationAction>>>,
    /// Per-step executor. v0.7.0 defaults to `DryRunExecutor`
    /// so the v0.6.0 behaviour is preserved; v0.7.0 callers
    /// that want network side effects can swap in a
    /// `RealExecutor` via [`RemediationEngine::with_executor`].
    executor: Arc<dyn StepExecutor>,
}

impl std::fmt::Debug for RemediationEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RemediationEngine")
            .field("runbook_count", &self.runbooks.read().len())
            .field("executor", &"Arc<dyn StepExecutor>")
            .finish()
    }
}

impl Clone for RemediationEngine {
    fn clone(&self) -> Self {
        Self {
            runbooks: Arc::clone(&self.runbooks),
            executor: Arc::clone(&self.executor),
        }
    }
}

impl Default for RemediationEngine {
    fn default() -> Self {
        Self {
            runbooks: Arc::new(RwLock::new(Vec::new())),
            executor: Arc::new(DryRunExecutor),
        }
    }
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
        Self {
            runbooks: Arc::new(RwLock::new(runbooks)),
            executor: Arc::new(DryRunExecutor),
        }
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
        Self {
            runbooks: Arc::new(RwLock::new(runbooks)),
            executor: Arc::new(DryRunExecutor),
        }
    }

    /// Replace the runbook table. Called by
    /// [`crate::watcher::Watcher`] when it detects a
    /// change in the runbook directory. Subsequent calls
    /// to [`Self::evaluate`] and [`Self::execute`] see
    /// the new table on the next read.
    ///
    /// The current implementation is infallible (a
    /// `Vec` swap). The method is named with a fallible
    /// return type to leave room for v0.7.1 validation
    /// hooks (e.g. cross-runbook id uniqueness) without
    /// breaking the call site.
    pub fn reload_runbooks(&self, new: Vec<RemediationAction>) {
        let mut guard = self.runbooks.write();
        *guard = new;
    }

    /// Snapshot of the current runbook table. Cheap
    /// (`Vec::clone` of small structs). Used by tests
    /// and the watcher's "fired" event.
    #[must_use]
    pub fn runbooks_snapshot(&self) -> Vec<RemediationAction> {
        self.runbooks.read().clone()
    }

    /// Builder-style: swap the per-step executor. Used by
    /// production wiring to inject a `RealExecutor`.
    ///
    /// ```ignore
    /// let engine = RemediationEngine::with_defaults()
    ///     .with_executor(Arc::new(RealExecutor::with_logging_client()));
    /// ```
    #[must_use]
    pub fn with_executor(mut self, executor: Arc<dyn StepExecutor>) -> Self {
        self.executor = executor;
        self
    }

    /// Read-only access to the inner executor. Tests use
    /// this to assert what was dispatched.
    #[must_use]
    pub fn executor(&self) -> &Arc<dyn StepExecutor> {
        &self.executor
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
        let guard = self.runbooks.read();
        guard
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
    /// `HttpCall` / `PgFunction` / `NotifySlack` /
    /// `PageOperator` are dispatched through the configured
    /// [`StepExecutor`]. The default is a [`DryRunExecutor`]
    /// that records intent and returns success; production
    /// wiring swaps in a [`RealExecutor`].
    ///
    /// Every step outcome is recorded to the [`metrics`]
    /// facade (`ada_remediation_actions_total`,
    /// `ada_remediation_action_duration_seconds`). State
    /// transitions are recorded in `record_state_transition`.
    /// Install the recorder with
    /// [`crate::metrics::install`] before scraping.
    pub async fn execute(&self, action: &RemediationAction) -> Result<ActionOutcome> {
        let started = Instant::now();
        let mut outcome = ActionOutcome::new(action.id.clone());
        let ctx = ExecutionContext::new(action.id.clone(), String::new());

        crate::metrics::record_state_transition("Idle", "Evaluating");
        crate::metrics::record_state_transition("Evaluating", "Executing");

        for (idx, step) in action.steps.iter().enumerate() {
            let (status, message, duration_ms) = self.run_step(idx, step, &ctx).await;
            let kind = step_kind_name(step);
            let outcome_label = if status { "success" } else { "failure" };
            crate::metrics::record_step_outcome(&action.id, outcome_label);
            crate::metrics::record_step_duration(
                &action.id,
                f64::from(u32::try_from(duration_ms).unwrap_or(u32::MAX)) / 1000.0,
            );
            let result = if status {
                StepResult::ok(idx, kind, message, duration_ms)
            } else {
                StepResult::fail(idx, kind, message, duration_ms)
            };
            outcome.push_step(result);
            if !status {
                outcome.fail();
                crate::metrics::record_state_transition("Executing", "Failed");
                break;
            }
        }
        outcome.complete(u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX));
        let final_state = if matches!(outcome.status, crate::action::OutcomeStatus::Succeeded) {
            "Cooldown"
        } else {
            "Failed"
        };
        crate::metrics::record_state_transition("Executing", final_state);
        Ok(outcome)
    }

    /// Run a single step. Returns
    /// `(success, message, duration_ms)`. Sub-steps of a
    /// `Sequence` are inlined; if any sub-step fails the
    /// whole `Sequence` fails with that message.
    async fn run_step(
        &self,
        _idx: usize,
        step: &ActionStep,
        ctx: &ExecutionContext,
    ) -> (bool, String, u64) {
        match step {
            ActionStep::RunCommand {
                cmd,
                args,
                timeout_secs,
            } => {
                let (ok, msg) =
                    run_shell_command(cmd, args, Duration::from_secs(*timeout_secs)).await;
                (ok, msg, 0)
            }
            ActionStep::Sequence { steps } => {
                for (i, sub) in steps.iter().enumerate() {
                    let (ok, sub_msg, _dur) =
                        Box::pin(self.run_step(i, sub, ctx)).await;
                    if !ok {
                        return (
                            false,
                            format!("sequence sub-step {i} failed: {sub_msg}"),
                            0,
                        );
                    }
                }
                (true, "sequence ok".to_string(), 0)
            }
            other => match self.executor.execute(other, ctx).await {
                Ok(r) => {
                    let dur_ms = u64::try_from(r.duration.as_millis()).unwrap_or(u64::MAX);
                    (true, r.message, dur_ms)
                }
                Err(e) => (false, e.to_string(), 0),
            },
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
                executor: crate::executor::ExecutorMode::DryRun,
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
                    executor: crate::executor::ExecutorMode::DryRun,
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
