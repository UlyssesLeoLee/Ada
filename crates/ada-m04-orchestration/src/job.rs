//! [`Job`] + [`JobId`] + [`JobState`] + [`JobKind`].
//!
//! The v0.1.0 surface is intentionally minimal: a `Job` is
//! the unit of work the orchestrator tracks, [`JobState`]
//! is its position in the canonical six-state machine, and
//! [`JobKind`] tells the worker pool *what kind* of work to
//! run when the production executor lands in B7+.
//!
//! State transitions allowed by the v0.1.0 skeleton
//! (validated by [`Job::transition_to`]):
//!
//! ```text
//!   Pending   -> Queued, Cancelled
//!   Queued    -> Running, Cancelled
//!   Running   -> Succeeded, Failed, Cancelled
//!   Succeeded -> (terminal)
//!   Failed    -> (terminal, except may be retried via a new Job)
//!   Cancelled -> (terminal)
//! ```
//!
//! See [`DOC-MOD-004`](../docs/modules/M-04-orchestration.md)
//! §3.2 for the full lifecycle.

use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use ada_core::UserId;

/// Stable, opaque job identifier. `Uuid`-backed so it can be
/// emitted in tracing spans and stored in the future
/// `orchestration_jobs` table without round-tripping through
/// a string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct JobId(pub Uuid);

impl JobId {
    /// Generate a fresh `JobId` (UUID v4).
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for JobId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for JobId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "job({})", self.0)
    }
}

impl From<Uuid> for JobId {
    fn from(u: Uuid) -> Self {
        Self(u)
    }
}

/// The position of a [`Job`] in the orchestrator's state
/// machine.
///
/// The six states are the canonical set agreed in
/// [`DOC-MOD-004`](../docs/modules/M-04-orchestration.md) §3.2.
/// `Pending` means "created but not yet visible to a worker";
/// `Queued` means "visible to a worker pool and waiting for a
/// slot"; `Running` means "a worker has claimed the job and is
/// executing it"; the remaining three are the terminal
/// outcomes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum JobState {
    /// Created, not yet visible to a worker pool.
    Pending,
    /// Visible to a worker pool and waiting for a slot.
    Queued,
    /// A worker has claimed the job and is executing it.
    Running,
    /// Terminal: the worker finished successfully.
    Succeeded,
    /// Terminal: the worker errored out.
    Failed,
    /// Terminal: cancelled by a worker or API caller.
    Cancelled,
}

impl JobState {
    /// Short, lowercase string tag (matches the variant name).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            JobState::Pending => "pending",
            JobState::Queued => "queued",
            JobState::Running => "running",
            JobState::Succeeded => "succeeded",
            JobState::Failed => "failed",
            JobState::Cancelled => "cancelled",
        }
    }

    /// `true` if no further state transitions are allowed.
    /// `Succeeded` / `Failed` / `Cancelled` are all terminal.
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            JobState::Succeeded | JobState::Failed | JobState::Cancelled
        )
    }

    /// Check whether a transition `self -> next` is allowed.
    /// See module docs for the full transition table.
    #[must_use]
    pub fn can_transition_to(self, next: JobState) -> bool {
        matches!(
            (self, next),
            (JobState::Pending, JobState::Queued | JobState::Cancelled)
                | (JobState::Queued, JobState::Running | JobState::Cancelled)
                | (
                    JobState::Running,
                    JobState::Succeeded | JobState::Failed | JobState::Cancelled
                )
        )
    }
}

impl fmt::Display for JobState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The kind of work a [`Job`] represents. The v0.1.0
/// orchestrator does not actually run the work (B7+); the
/// kind is just metadata that lets the worker pool pick the
/// right executor when the production build lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum JobKind {
    /// Run an M-01 data acquisition adapter.
    Acquisition,
    /// Run an M-02 normalizer transform.
    Normalization,
    /// Run an M-03 data flow.
    FlowExecution,
    /// Run an M-05 control-flow step.
    ControlFlow,
    /// Run an M-09 exporter.
    Export,
}

impl JobKind {
    /// Short, lowercase string tag.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            JobKind::Acquisition => "acquisition",
            JobKind::Normalization => "normalization",
            JobKind::FlowExecution => "flow-execution",
            JobKind::ControlFlow => "control-flow",
            JobKind::Export => "export",
        }
    }
}

impl fmt::Display for JobKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The unit of work the orchestrator tracks.
///
/// `created_at_ms` is set on construction; `started_at_ms` and
/// `finished_at_ms` are updated by the scheduler as the job
/// progresses. The skeleton keeps the timestamps as
/// `Option<u64>` so a freshly-constructed `Job` does not have
/// to fabricate fake start/finish times.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Job {
    /// Stable, opaque job id (UUID v4).
    pub id: JobId,
    /// What kind of work this is.
    pub kind: JobKind,
    /// Free-form payload handed to the worker. The skeleton
    /// does not validate the shape — production will pin it
    /// per `JobKind`.
    pub payload: Value,
    /// Current state in the state machine.
    pub state: JobState,
    /// Wall-clock time the job was created, in milliseconds
    /// since the Unix epoch.
    pub created_at_ms: u64,
    /// Wall-clock time the worker started executing, in
    /// milliseconds since the Unix epoch. `None` until the
    /// job is `Running`.
    pub started_at_ms: Option<u64>,
    /// Wall-clock time the worker finished (success, failure,
    /// or cancel), in milliseconds since the Unix epoch.
    /// `None` while the job is in flight.
    pub finished_at_ms: Option<u64>,
    /// Owning user (or `None` for system-initiated jobs).
    pub owner: Option<UserId>,
}

impl Job {
    /// Build a new `Job` in [`JobState::Pending`] with the
    /// current wall-clock timestamp. `owner` is optional —
    /// pass `None` for system-initiated jobs.
    #[must_use]
    pub fn new(kind: JobKind, payload: Value, owner: Option<UserId>) -> Self {
        Self {
            id: JobId::new(),
            kind,
            payload,
            state: JobState::Pending,
            created_at_ms: now_ms(),
            started_at_ms: None,
            finished_at_ms: None,
            owner,
        }
    }

    /// Attempt to move the job into `next`. Returns
    /// `true` on success. The caller is expected to surface
    /// the rejection as an [`crate::OrchError::InvalidState`]
    /// when the operation comes from a public API.
    pub fn transition_to(&mut self, next: JobState) -> bool {
        if !self.state.can_transition_to(next) {
            return false;
        }
        let now = now_ms();
        if matches!(next, JobState::Running) && self.started_at_ms.is_none() {
            self.started_at_ms = Some(now);
        }
        if next.is_terminal() {
            self.finished_at_ms = Some(now);
        }
        self.state = next;
        true
    }
}

/// Monotonic-ish wall-clock millis. Uses
/// [`std::time::SystemTime`]; the v0.1.0 skeleton does not
/// inject a clock for testability (the worker pool lands in
/// B7+ and will own that abstraction).
fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let d = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    u64::try_from(d.as_millis()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn job_id_is_unique() {
        let a = JobId::new();
        let b = JobId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn job_id_display() {
        let r = JobId(Uuid::nil());
        assert_eq!(r.to_string(), "job(00000000-0000-0000-0000-000000000000)");
    }

    #[test]
    fn job_id_from_uuid() {
        let u = Uuid::new_v4();
        let r = JobId::from(u);
        assert_eq!(r.0, u);
    }

    #[test]
    fn state_is_terminal() {
        assert!(!JobState::Pending.is_terminal());
        assert!(!JobState::Queued.is_terminal());
        assert!(!JobState::Running.is_terminal());
        assert!(JobState::Succeeded.is_terminal());
        assert!(JobState::Failed.is_terminal());
        assert!(JobState::Cancelled.is_terminal());
    }

    #[test]
    fn state_as_str_matches_variant() {
        assert_eq!(JobState::Pending.as_str(), "pending");
        assert_eq!(JobState::Queued.as_str(), "queued");
        assert_eq!(JobState::Running.as_str(), "running");
        assert_eq!(JobState::Succeeded.as_str(), "succeeded");
        assert_eq!(JobState::Failed.as_str(), "failed");
        assert_eq!(JobState::Cancelled.as_str(), "cancelled");
    }

    #[test]
    fn state_display_round_trip() {
        for s in [
            JobState::Pending,
            JobState::Queued,
            JobState::Running,
            JobState::Succeeded,
            JobState::Failed,
            JobState::Cancelled,
        ] {
            assert_eq!(s.to_string(), s.as_str());
        }
    }

    #[test]
    fn kind_as_str_matches_variant() {
        assert_eq!(JobKind::Acquisition.as_str(), "acquisition");
        assert_eq!(JobKind::Normalization.as_str(), "normalization");
        assert_eq!(JobKind::FlowExecution.as_str(), "flow-execution");
        assert_eq!(JobKind::ControlFlow.as_str(), "control-flow");
        assert_eq!(JobKind::Export.as_str(), "export");
    }

    #[test]
    fn kind_display_round_trip() {
        for k in [
            JobKind::Acquisition,
            JobKind::Normalization,
            JobKind::FlowExecution,
            JobKind::ControlFlow,
            JobKind::Export,
        ] {
            assert_eq!(k.to_string(), k.as_str());
        }
    }

    #[test]
    fn job_new_starts_in_pending() {
        let job = Job::new(JobKind::Acquisition, json!({"x": 1}), None);
        assert_eq!(job.state, JobState::Pending);
        assert!(job.started_at_ms.is_none());
        assert!(job.finished_at_ms.is_none());
        assert!(job.created_at_ms > 0);
    }

    #[test]
    fn transition_pending_to_queued_succeeds() {
        let mut job = Job::new(JobKind::FlowExecution, json!({}), None);
        assert!(job.transition_to(JobState::Queued));
        assert_eq!(job.state, JobState::Queued);
    }

    #[test]
    fn transition_running_records_started_at() {
        let mut job = Job::new(JobKind::Export, json!({}), None);
        job.transition_to(JobState::Queued);
        assert!(job.transition_to(JobState::Running));
        assert!(job.started_at_ms.is_some());
    }

    #[test]
    fn transition_to_succeeded_records_finished_at() {
        let mut job = Job::new(JobKind::Acquisition, json!({}), None);
        job.transition_to(JobState::Queued);
        job.transition_to(JobState::Running);
        assert!(job.transition_to(JobState::Succeeded));
        assert_eq!(job.state, JobState::Succeeded);
        assert!(job.finished_at_ms.is_some());
    }

    #[test]
    fn terminal_state_rejects_further_transitions() {
        let mut job = Job::new(JobKind::Acquisition, json!({}), None);
        job.transition_to(JobState::Queued);
        job.transition_to(JobState::Running);
        job.transition_to(JobState::Succeeded);
        assert!(!job.transition_to(JobState::Running));
        assert!(!job.transition_to(JobState::Cancelled));
        assert_eq!(job.state, JobState::Succeeded);
    }

    #[test]
    fn invalid_transition_pending_to_running_is_rejected() {
        let mut job = Job::new(JobKind::Acquisition, json!({}), None);
        assert!(!job.transition_to(JobState::Running));
        assert_eq!(job.state, JobState::Pending);
    }

    #[test]
    fn invalid_transition_succeeded_to_queued_is_rejected() {
        let mut job = Job::new(JobKind::Acquisition, json!({}), None);
        job.transition_to(JobState::Queued);
        job.transition_to(JobState::Running);
        job.transition_to(JobState::Succeeded);
        assert!(!job.transition_to(JobState::Queued));
    }

    #[test]
    fn cancel_is_legal_from_running() {
        let mut job = Job::new(JobKind::FlowExecution, json!({}), None);
        job.transition_to(JobState::Queued);
        job.transition_to(JobState::Running);
        assert!(job.transition_to(JobState::Cancelled));
        assert_eq!(job.state, JobState::Cancelled);
        assert!(job.finished_at_ms.is_some());
    }

    #[test]
    fn serde_roundtrip() {
        let job = Job::new(JobKind::Normalization, json!({"a": 1}), None);
        let s = serde_json::to_string(&job).expect("serialize");
        let back: Job = serde_json::from_str(&s).expect("deserialize");
        assert_eq!(back.id, job.id);
        assert_eq!(back.kind, job.kind);
        assert_eq!(back.state, job.state);
        assert_eq!(back.payload, job.payload);
    }
}
