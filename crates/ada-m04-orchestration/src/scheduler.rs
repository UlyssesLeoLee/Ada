//! [`Scheduler`] trait + the in-process [`InMemoryScheduler`].
//!
//! The v0.1.0 surface is intentionally minimal:
//!
//! - [`Scheduler::enqueue`] appends a `Job` to the queue
//!   and returns its id.
//! - [`Scheduler::poll`] returns the `Pending` / `Queued`
//!   jobs the worker pool should look at on the next tick.
//! - [`Scheduler::cancel`] moves a `Pending` / `Queued` /
//!   `Running` job to `Cancelled` and returns the
//!   pre-cancel `Job` for inspection / audit.
//! - [`Scheduler::state_of`] returns the current
//!   [`JobState`] for a given id (used by the API gateway
//!   status endpoint).
//!
//! The v0.1.0 skeleton does **not** spawn workers; the
//! [`InMemoryScheduler`] just owns the state. The real
//! worker pool (a `tokio::task::spawn` fan-out against the
//! `Running` slot) lands in B7+ once the G4 (実装着手判定)
//! is approved.
//!
//! See [`DOC-MOD-004`](../docs/modules/M-04-orchestration.md)
//! §3.3 for the full lifecycle.

use std::collections::HashMap;

use async_trait::async_trait;
use parking_lot::Mutex;
use serde_json::Value;

use crate::error::{OrchError, Result};
use crate::job::{Job, JobId, JobState};

/// Default maximum queue depth used by
/// [`InMemoryScheduler::default`]. The value is intentionally
/// large enough for unit tests and small dev builds; the
/// production build will override it via
/// [`InMemoryScheduler::with_capacity`].
pub const DEFAULT_QUEUE_CAPACITY: usize = 4096;

/// The trait every scheduler implements. The skeleton
/// surfaces four operations; the production trait will add
/// `register_worker` and `lease_next` (a worker pulls one
/// job at a time and must acknowledge it).
#[async_trait]
pub trait Scheduler: Send + Sync {
    /// Append `job` to the queue. The job's `id` is
    /// preserved; pass an existing id only if you mean to
    /// re-enqueue (e.g. retry). Returns the id of the
    /// stored job.
    async fn enqueue(&self, job: Job) -> Result<JobId>;

    /// Drain every `Pending` / `Queued` job from the
    /// scheduler, returning them in FIFO order. The
    /// skeleton just snapshots the queue — it does **not**
    /// move them to `Running`. The production build will
    /// flip them to `Running` and hand the worker pool a
    /// `lease`.
    async fn poll(&self) -> Result<Vec<Job>>;

    /// Move the job with `id` to [`JobState::Cancelled`].
    /// Returns the pre-cancel `Job` for inspection. If the
    /// job is already terminal, the call is a no-op and the
    /// pre-cancel job is still returned.
    async fn cancel(&self, id: JobId) -> Result<Job>;

    /// Look up the current [`JobState`] for `id`. Returns
    /// [`OrchError::JobNotFound`] if the id is unknown.
    async fn state_of(&self, id: JobId) -> Result<JobState>;

    /// Current number of jobs in flight (Pending + Queued +
    /// Running). Useful for tests and operational metrics.
    async fn in_flight(&self) -> usize;
}

/// In-process scheduler, backed by a
/// `parking_lot::Mutex<HashMap<JobId, Job>>`. The `VecDeque`
/// inside the lock preserves FIFO ordering for `poll()`.
///
/// v0.1.0 capacity defaults to
/// [`DEFAULT_QUEUE_CAPACITY`]; override via
/// [`InMemoryScheduler::with_capacity`].
#[derive(Debug)]
pub struct InMemoryScheduler {
    capacity: usize,
    inner: Mutex<SchedulerState>,
}

#[derive(Debug)]
struct SchedulerState {
    order: std::collections::VecDeque<JobId>,
    jobs: HashMap<JobId, Job>,
}

impl InMemoryScheduler {
    /// Empty scheduler with the default queue capacity.
    #[must_use]
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_QUEUE_CAPACITY)
    }

    /// Empty scheduler with an explicit queue capacity. A
    /// capacity of `0` is rejected at the type level
    /// (callers should use `new`).
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        let cap = if capacity == 0 { 1 } else { capacity };
        Self {
            capacity: cap,
            inner: Mutex::new(SchedulerState {
                order: std::collections::VecDeque::new(),
                jobs: HashMap::new(),
            }),
        }
    }

    /// Configured queue capacity.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Snapshot every known job (FIFO). Mostly useful in
    /// tests; the production caller should use `state_of`
    /// per id.
    pub fn snapshot(&self) -> Vec<Job> {
        let guard = self.inner.lock();
        guard
            .order
            .iter()
            .filter_map(|id| guard.jobs.get(id).cloned())
            .collect()
    }

    /// Test / production helper: insert a pre-built `Job`
    /// directly (bypassing the queue-capacity check). The
    /// caller owns the id and is responsible for not
    /// double-enqueueing.
    pub fn insert(&self, job: Job) {
        let mut guard = self.inner.lock();
        if !guard.jobs.contains_key(&job.id) {
            guard.order.push_back(job.id);
        }
        guard.jobs.insert(job.id, job);
    }
}

impl Default for InMemoryScheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Scheduler for InMemoryScheduler {
    async fn enqueue(&self, job: Job) -> Result<JobId> {
        let id = job.id;
        {
            let mut guard = self.inner.lock();
            if guard.jobs.contains_key(&id) {
                return Err(OrchError::BackendError(format!("duplicate job id: {id}")));
            }
            if guard.order.len() >= self.capacity {
                return Err(OrchError::QueueFull {
                    capacity: self.capacity,
                });
            }
            guard.order.push_back(id);
            guard.jobs.insert(id, job);
        }
        Ok(id)
    }

    async fn poll(&self) -> Result<Vec<Job>> {
        let guard = self.inner.lock();
        let snapshot: Vec<Job> = guard
            .order
            .iter()
            .filter_map(|id| guard.jobs.get(id))
            .filter(|j| matches!(j.state, JobState::Pending | JobState::Queued))
            .cloned()
            .collect();
        Ok(snapshot)
    }

    async fn cancel(&self, id: JobId) -> Result<Job> {
        let mut guard = self.inner.lock();
        let job = guard.jobs.get_mut(&id).ok_or(OrchError::JobNotFound(id))?;
        if job.state.is_terminal() {
            return Ok(job.clone());
        }
        if !job.transition_to(JobState::Cancelled) {
            return Err(OrchError::InvalidState {
                from: job.state.to_string(),
                to: JobState::Cancelled.to_string(),
            });
        }
        Ok(job.clone())
    }

    async fn state_of(&self, id: JobId) -> Result<JobState> {
        let guard = self.inner.lock();
        guard
            .jobs
            .get(&id)
            .map(|j| j.state)
            .ok_or(OrchError::JobNotFound(id))
    }

    async fn in_flight(&self) -> usize {
        let guard = self.inner.lock();
        guard
            .jobs
            .values()
            .filter(|j| !j.state.is_terminal())
            .count()
    }
}

/// Convenience: build a `Job` and enqueue it in one call.
pub async fn enqueue_job<S: Scheduler + ?Sized>(
    scheduler: &S,
    kind: crate::job::JobKind,
    payload: Value,
    owner: Option<ada_core::UserId>,
) -> Result<JobId> {
    let job = Job::new(kind, payload, owner);
    scheduler.enqueue(job).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::job::JobKind;
    use serde_json::json;
    use uuid::Uuid;

    fn new_scheduler() -> InMemoryScheduler {
        InMemoryScheduler::new()
    }

    fn small_scheduler(cap: usize) -> InMemoryScheduler {
        InMemoryScheduler::with_capacity(cap)
    }

    fn job(kind: JobKind) -> Job {
        Job::new(kind, json!({}), None)
    }

    #[tokio::test]
    async fn enqueue_assigns_id_and_in_flight() {
        let s = new_scheduler();
        let j = job(JobKind::Acquisition);
        let id = s.enqueue(j).await.expect("enqueue");
        assert_eq!(s.in_flight().await, 1);
        assert_eq!(s.state_of(id).await.unwrap(), JobState::Pending);
    }

    #[tokio::test]
    async fn enqueue_duplicate_id_rejected() {
        let s = new_scheduler();
        let j = job(JobKind::Acquisition);
        let id = j.id;
        s.enqueue(j).await.expect("first");
        // Build a second job with the same id by re-using
        // the field — we use insert() because enqueue()
        // generates a fresh id. The duplicate path is
        // exercised via a custom Job.
        let mut j2 = job(JobKind::Normalization);
        j2.id = id;
        let err = s.enqueue(j2).await.expect_err("dup");
        assert!(matches!(err, OrchError::BackendError(_)));
    }

    #[tokio::test]
    async fn enqueue_respects_capacity() {
        let s = small_scheduler(2);
        s.enqueue(job(JobKind::Acquisition)).await.unwrap();
        s.enqueue(job(JobKind::Normalization)).await.unwrap();
        let err = s.enqueue(job(JobKind::Export)).await.expect_err("full");
        assert!(matches!(err, OrchError::QueueFull { capacity: 2 }));
    }

    #[tokio::test]
    async fn poll_returns_pending_and_queued() {
        let s = new_scheduler();
        let j1 = job(JobKind::Acquisition);
        let j2 = job(JobKind::Normalization);
        let j3 = job(JobKind::FlowExecution);
        s.enqueue(j1.clone()).await.unwrap();
        s.enqueue(j2.clone()).await.unwrap();
        s.enqueue(j3.clone()).await.unwrap();
        let polled = s.poll().await.unwrap();
        // FIFO order preserved
        assert_eq!(polled.len(), 3);
        assert_eq!(polled[0].id, j1.id);
        assert_eq!(polled[1].id, j2.id);
        assert_eq!(polled[2].id, j3.id);
    }

    #[tokio::test]
    async fn poll_omits_terminal_jobs() {
        let s = new_scheduler();
        let j1 = job(JobKind::Acquisition);
        let id1 = j1.id;
        let j2 = job(JobKind::Export);
        s.enqueue(j1).await.unwrap();
        s.enqueue(j2).await.unwrap();
        // Drive j1 to Succeeded
        let snap = s.snapshot();
        let mut drive = snap.into_iter().find(|j| j.id == id1).unwrap();
        drive.transition_to(JobState::Queued);
        drive.transition_to(JobState::Running);
        drive.transition_to(JobState::Succeeded);
        s.insert(drive);
        let polled = s.poll().await.unwrap();
        assert_eq!(polled.len(), 1);
        assert_ne!(polled[0].id, id1);
    }

    #[tokio::test]
    async fn state_of_unknown_returns_job_not_found() {
        let s = new_scheduler();
        let err = s
            .state_of(JobId(Uuid::new_v4()))
            .await
            .expect_err("unknown");
        assert!(matches!(err, OrchError::JobNotFound(_)));
    }

    #[tokio::test]
    async fn cancel_moves_to_cancelled() {
        let s = new_scheduler();
        let j = job(JobKind::Acquisition);
        let id = j.id;
        s.enqueue(j).await.unwrap();
        let returned = s.cancel(id).await.expect("cancel");
        assert_eq!(returned.state, JobState::Cancelled);
        assert_eq!(s.state_of(id).await.unwrap(), JobState::Cancelled);
        assert_eq!(s.in_flight().await, 0);
    }

    #[tokio::test]
    async fn cancel_unknown_returns_job_not_found() {
        let s = new_scheduler();
        let err = s.cancel(JobId(Uuid::new_v4())).await.expect_err("missing");
        assert!(matches!(err, OrchError::JobNotFound(_)));
    }

    #[tokio::test]
    async fn cancel_already_terminal_is_a_noop() {
        let s = new_scheduler();
        let j = job(JobKind::Acquisition);
        let id = j.id;
        s.enqueue(j).await.unwrap();
        // Drive to Succeeded
        let snap = s.snapshot();
        let mut drive = snap.into_iter().next().unwrap();
        drive.transition_to(JobState::Queued);
        drive.transition_to(JobState::Running);
        drive.transition_to(JobState::Succeeded);
        s.insert(drive);
        let returned = s.cancel(id).await.expect("noop");
        assert_eq!(returned.state, JobState::Succeeded);
    }

    #[tokio::test]
    async fn in_flight_counts_correctly() {
        let s = new_scheduler();
        assert_eq!(s.in_flight().await, 0);
        s.enqueue(job(JobKind::Acquisition)).await.unwrap();
        s.enqueue(job(JobKind::Normalization)).await.unwrap();
        s.enqueue(job(JobKind::Export)).await.unwrap();
        assert_eq!(s.in_flight().await, 3);
        // Cancel one
        let snap = s.snapshot();
        let first = snap[0].id;
        s.cancel(first).await.unwrap();
        assert_eq!(s.in_flight().await, 2);
    }

    #[tokio::test]
    async fn snapshot_returns_fifo() {
        let s = new_scheduler();
        let a = job(JobKind::Acquisition);
        let b = job(JobKind::Normalization);
        let aid = a.id;
        let bid = b.id;
        s.enqueue(a).await.unwrap();
        s.enqueue(b).await.unwrap();
        let snap = s.snapshot();
        assert_eq!(snap.len(), 2);
        assert_eq!(snap[0].id, aid);
        assert_eq!(snap[1].id, bid);
    }

    #[tokio::test]
    async fn enqueue_helper_works() {
        let s = new_scheduler();
        let id = enqueue_job(&s, JobKind::Acquisition, json!({"a": 1}), None)
            .await
            .expect("enqueue");
        assert_eq!(s.state_of(id).await.unwrap(), JobState::Pending);
    }

    #[tokio::test]
    async fn with_capacity_zero_clamps_to_one() {
        // A capacity of 0 is meaningless; the constructor
        // bumps it to 1 so the scheduler remains usable.
        let s = InMemoryScheduler::with_capacity(0);
        assert_eq!(s.capacity(), 1);
        s.enqueue(job(JobKind::Acquisition)).await.unwrap();
        let err = s.enqueue(job(JobKind::Export)).await.expect_err("full");
        assert!(matches!(err, OrchError::QueueFull { capacity: 1 }));
    }

    #[tokio::test]
    async fn dyn_dispatch_through_trait_object() {
        let s: std::sync::Arc<dyn Scheduler> = std::sync::Arc::new(new_scheduler());
        let id = s.enqueue(job(JobKind::FlowExecution)).await.unwrap();
        assert_eq!(s.state_of(id).await.unwrap(), JobState::Pending);
    }
}
