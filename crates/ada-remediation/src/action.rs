//! Remediation action model.
//!
//! One [`RemediationAction`] is the unit of evaluation: a
//! declarative definition of "when this alert fires, do these
//! steps, in this order, then sit in cooldown for N seconds
//! before I am willing to fire again".
//!
//! Steps are an *ordered* `Vec<ActionStep>`. Failure semantics:
//! - On the first failing step the engine short-circuits and
//!   records the action as `Failed` with the step index + error.
//! - The engine then re-attempts the full action up to
//!   `max_retries` (each attempt is logged separately in the
//!   history table).

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// One declarative remediation action, loaded from a runbook
/// JSON file (see `config/remediation/*.json`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemediationAction {
    /// Stable id (e.g. `disk-space-low`). This is the primary
    /// key for the persistent cooldown table and history table.
    pub id: String,

    /// Human-readable name shown in the dashboard ("Disk Space Low").
    #[serde(default)]
    pub name: String,

    /// Trigger pattern. Either an exact match against
    /// `AlertEvent::alert_name` (e.g. `ServiceDown`) or a
    /// shell-style glob (`SLIBurn*`, `DB*Pool*`). Engine uses
    /// `glob::Match::new` for matching (no extra crate — a
    /// minimal hand-rolled matcher is in [`crate::config`]).
    pub trigger: Trigger,

    /// Optional severity filter. If set, the action only
    /// matches alerts whose `severity` label is in this list.
    #[serde(default)]
    pub severities: Vec<String>,

    /// Ordered execution list.
    #[serde(default)]
    pub steps: Vec<ActionStep>,

    /// Cooldown window: after a successful execution, the
    /// engine refuses to re-fire the same action for this
    /// long, even if the alert keeps flapping.
    #[serde(with = "duration_secs", default = "default_cooldown")]
    pub cooldown: Duration,

    /// How many times the engine re-attempts a failing action
    /// before giving up and recording `Failed` permanently.
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
}

const fn default_cooldown() -> Duration {
    Duration::from_secs(300)
}

const fn default_max_retries() -> u32 {
    2
}

/// Trigger match mode.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum Trigger {
    /// Exact match against `alert_name`.
    Exact(String),
    /// Shell-style glob: `*` is any run, `?` is any char.
    /// No character classes (kept simple for offline parsing).
    Glob(String),
}

impl Trigger {
    #[must_use]
    pub fn matches(&self, alert_name: &str) -> bool {
        match self {
            Self::Exact(s) => s == alert_name,
            Self::Glob(g) => glob_match(g, alert_name),
        }
    }
}

/// One executable step inside a runbook. Six variants — chosen
/// to cover ~all of the runbook actions described in
/// `docs/observability/12-auto-remediation.md` §3.1.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ActionStep {
    /// Run a shell command. The process inherits nothing
    /// from the engine process by default (`std::process::Command`
    /// does not copy env unless explicitly asked).
    RunCommand {
        cmd: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default = "default_step_timeout_secs")]
        timeout_secs: u64,
    },
    /// Issue an HTTP call (e.g. to a Kubernetes API or an
    /// internal control plane). The engine does **not** include
    /// a real HTTP client in this crate (the offline build
    /// environment forbids it) — see [`crate::http`] for the
    /// server side. The executor is pluggable: the default
    /// `DryRunHttp` records the intent and returns success.
    HttpCall {
        url: String,
        method: HttpMethod,
        #[serde(default)]
        body: Option<String>,
        #[serde(default)]
        headers: Vec<(String, String)>,
    },
    /// Invoke a PL/pgSQL function. The function name is the
    /// bare identifier (e.g. `remediation_kill_idle`); the
    /// engine's persistent executor (separate binary) is
    /// responsible for resolving the connection pool and
    /// calling `SELECT fn_name(args...)`.
    PgFunction {
        name: String,
        #[serde(default)]
        args: Vec<String>,
    },
    /// Post to Slack. The `channel` is `#ada-incident` style
    /// (no URL is stored in runbook files; the channel name is
    /// mapped to a webhook URL at executor time so secrets
    /// stay in env vars per `docs/observability/09-security-design.md`).
    NotifySlack { channel: String, message: String },
    /// Page an operator. Severity is `high` (page on-call) or
    /// `low` (open a ticket). `runbook_url` is shown in the
    /// pager payload so the responder can read the procedure.
    PageOperator {
        severity: PageSeverity,
        runbook_url: String,
    },
    /// Composite of two or more steps that all need to succeed.
    /// The inner `Vec` runs sequentially; first failure aborts.
    Sequence { steps: Vec<ActionStep> },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

impl HttpMethod {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Patch => "PATCH",
            Self::Delete => "DELETE",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PageSeverity {
    High,
    Low,
}

const fn default_step_timeout_secs() -> u64 {
    30
}

/// Result of executing a remediation action. Per-step results
/// are recorded so the dashboard can show "step 3/5 failed".
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActionOutcome {
    pub action_id: String,
    pub status: OutcomeStatus,
    pub step_results: Vec<StepResult>,
    pub total_duration_ms: u64,
}

impl ActionOutcome {
    #[must_use]
    pub fn new(action_id: impl Into<String>) -> Self {
        Self {
            action_id: action_id.into(),
            status: OutcomeStatus::Succeeded,
            step_results: Vec::new(),
            total_duration_ms: 0,
        }
    }

    pub fn push_step(&mut self, result: StepResult) {
        self.step_results.push(result);
    }

    pub fn fail(&mut self) {
        self.status = OutcomeStatus::Failed;
    }

    pub fn complete(&mut self, duration_ms: u64) {
        self.total_duration_ms = duration_ms;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OutcomeStatus {
    Succeeded,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StepResult {
    pub index: usize,
    pub kind: String,
    pub status: OutcomeStatus,
    pub message: String,
    pub duration_ms: u64,
}

impl StepResult {
    #[must_use]
    pub fn ok(
        index: usize,
        kind: impl Into<String>,
        message: impl Into<String>,
        duration_ms: u64,
    ) -> Self {
        Self {
            index,
            kind: kind.into(),
            status: OutcomeStatus::Succeeded,
            message: message.into(),
            duration_ms,
        }
    }

    #[must_use]
    pub fn fail(
        index: usize,
        kind: impl Into<String>,
        message: impl Into<String>,
        duration_ms: u64,
    ) -> Self {
        Self {
            index,
            kind: kind.into(),
            status: OutcomeStatus::Failed,
            message: message.into(),
            duration_ms,
        }
    }
}

/// `Duration` <-> integer-seconds serde adapter. We deliberately
/// use seconds, not millis, because runbook files are authored
/// by humans and `300` reads better than `300000`.
mod duration_secs {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S: Serializer>(d: &Duration, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_u64(d.as_secs())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<Duration, D::Error> {
        let secs = u64::deserialize(de)?;
        Ok(Duration::from_secs(secs))
    }
}

/// Minimal glob matcher that supports `*` (any run, including
/// empty) and `?` (any single char). All other characters are
/// literal. No character classes; we deliberately do not pull
/// in the `glob` crate (it isn't in the offline Cargo.lock).
fn glob_match(pattern: &str, text: &str) -> bool {
    let p = pattern.as_bytes();
    let t = text.as_bytes();
    glob_match_inner(p, t, 0, 0)
}

fn glob_match_inner(p: &[u8], t: &[u8], pi: usize, ti: usize) -> bool {
    let mut pi = pi;
    let mut ti = ti;
    while pi < p.len() {
        match p[pi] {
            b'*' => {
                // Try matching the rest of the pattern against every
                // suffix of `t` from `ti` onwards.
                pi += 1;
                while ti <= t.len() {
                    if glob_match_inner(p, t, pi, ti) {
                        return true;
                    }
                    ti += 1;
                }
                return false;
            }
            b'?' => {
                if ti >= t.len() {
                    return false;
                }
                pi += 1;
                ti += 1;
            }
            c => {
                if ti >= t.len() || t[ti] != c {
                    return false;
                }
                pi += 1;
                ti += 1;
            }
        }
    }
    ti == t.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glob_exact_match() {
        assert!(glob_match("ServiceDown", "ServiceDown"));
        assert!(!glob_match("ServiceDown", "ServiceDegraded"));
    }

    #[test]
    fn glob_star_matches_run() {
        assert!(glob_match("SLIBurn*", "SLIBurnRateFast"));
        assert!(glob_match("DB*Pool*", "DBConnectionPoolExhausted"));
        assert!(!glob_match("DB*Pool*", "DBConnectionTimeout"));
    }

    #[test]
    fn glob_question_matches_single_char() {
        assert!(glob_match("Pod?Down", "Pod1Down"));
        assert!(!glob_match("Pod?Down", "Pod12Down"));
    }

    #[test]
    fn trigger_glob_matches() {
        let t = Trigger::Glob("SLIBurn*".into());
        assert!(t.matches("SLIBurnRateFast"));
        assert!(t.matches("SLIBurnRateSlow"));
        assert!(!t.matches("SLOBreach"));
    }

    #[test]
    fn trigger_exact_matches() {
        let t = Trigger::Exact("ServiceDown".into());
        assert!(t.matches("ServiceDown"));
        assert!(!t.matches("ServiceDegraded"));
    }

    #[test]
    fn step_run_command_default_timeout() {
        let step: ActionStep = serde_json::from_str(
            r#"{ "kind": "run_command", "cmd": "du", "args": ["-sh", "/var/log"] }"#,
        )
        .unwrap();
        match step {
            ActionStep::RunCommand {
                cmd,
                args,
                timeout_secs,
            } => {
                assert_eq!(cmd, "du");
                assert_eq!(args, vec!["-sh", "/var/log"]);
                assert_eq!(timeout_secs, 30);
            }
            _ => panic!("wrong variant"),
        }
    }
}
