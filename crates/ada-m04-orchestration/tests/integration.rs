//! Integration tests for the v0.1.0 orchestrator.
//!
//! The skeleton is in-process, so the "integration" tests
//! exercise the public trait the way a real worker pool or
//! API gateway would: enqueue a job, poll for it, transition
//! it to a terminal state, and verify the scheduler's
//! state-of/cancel surface behaves correctly.

use std::sync::Arc;

use ada_m04_orchestration::{
    enqueue_job, InMemoryScheduler, Job, JobKind, JobState, OrchError, Scheduler,
};
use serde_json::json;
use uuid::Uuid;

#[tokio::test]
async fn worker_pulls_enqueues_runs_succeeds() {
    let s = Arc::new(InMemoryScheduler::new());
    let id = enqueue_job(&*s, JobKind::FlowExecution, json!({"canvas": "c-1"}), None)
        .await
        .expect("enqueue");
    assert_eq!(s.state_of(id).await.unwrap(), JobState::Pending);

    // 1. Worker polls the queue.
    let mut ready = s.poll().await.expect("poll");
    assert_eq!(ready.len(), 1);
    let job = ready.pop().unwrap();
    assert_eq!(job.id, id);

    // 2. Worker drives the job through the state machine.
    let snap = s.snapshot();
    let mut live = snap.into_iter().find(|j| j.id == id).expect("present");
    assert!(live.transition_to(JobState::Queued));
    assert!(live.transition_to(JobState::Running));
    assert!(live.transition_to(JobState::Succeeded));
    s.insert(live);

    assert_eq!(s.state_of(id).await.unwrap(), JobState::Succeeded);
    assert_eq!(s.in_flight().await, 0);
    assert!(s.poll().await.unwrap().is_empty());
}

#[tokio::test]
async fn cancel_mid_run_marks_terminal_and_clears_in_flight() {
    let s = Arc::new(InMemoryScheduler::new());
    let id = enqueue_job(&*s, JobKind::Acquisition, json!({}), None)
        .await
        .expect("enqueue");
    // Pretend a worker started the job
    let snap = s.snapshot();
    let mut live = snap.into_iter().find(|j| j.id == id).expect("present");
    assert!(live.transition_to(JobState::Queued));
    assert!(live.transition_to(JobState::Running));
    s.insert(live);
    assert_eq!(s.in_flight().await, 1);

    // Cancel from a "control plane" task
    let returned = s.cancel(id).await.expect("cancel");
    assert_eq!(returned.state, JobState::Cancelled);
    assert_eq!(s.state_of(id).await.unwrap(), JobState::Cancelled);
    assert_eq!(s.in_flight().await, 0);
}

#[tokio::test]
async fn queue_full_rejects_extra_enqueue() {
    let s = InMemoryScheduler::with_capacity(1);
    s.enqueue(Job::new(JobKind::Export, json!({}), None))
        .await
        .expect("first");
    let err = s
        .enqueue(Job::new(JobKind::Export, json!({}), None))
        .await
        .expect_err("full");
    assert!(matches!(err, OrchError::QueueFull { capacity: 1 }));
}

#[tokio::test]
async fn unknown_id_for_state_of_and_cancel() {
    let s = InMemoryScheduler::new();
    let ghost = Uuid::new_v4().into();
    let a = s.state_of(ghost).await.expect_err("state_of");
    let b = s.cancel(ghost).await.expect_err("cancel");
    assert!(matches!(a, OrchError::JobNotFound(_)));
    assert!(matches!(b, OrchError::JobNotFound(_)));
}
