//! InMemoryScheduler — 业务 ada-m04 orchestration 的"形状相似"克隆.
//!
//! 状态机: `Pending -> Queued -> Running -> (Succeeded | Failed | Cancelled)`.
//! 容量限制: 模拟业务版的 `Scheduler::with_capacity`.
//! **不**实现 worker poll — 这是一个同步/手动驱动的 mock, 便于测试断言.

use parking_lot::Mutex;
use std::collections::VecDeque;
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SchedulerError {
    #[error("queue full: capacity {0} reached")]
    QueueFull(usize),
    #[error("job not found: {0}")]
    JobNotFound(Uuid),
    #[error("illegal transition: {from} -> {to}")]
    IllegalTransition { from: String, to: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobState {
    Pending,
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

impl JobState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScheduledJob {
    pub id: Uuid,
    pub kind: String,
    pub state: JobState,
}

#[derive(Debug)]
struct Inner {
    capacity: usize,
    jobs: Vec<ScheduledJob>,
    /// 简化: 把"pending + queued"都视为 in_flight, 实际业务版区分两者.
    in_flight: usize,
    /// FIFO 取数顺序 — 业务版用 priority + age, 这里仅 age.
    insertion: VecDeque<Uuid>,
}

#[derive(Debug, Clone)]
pub struct InMemoryScheduler {
    inner: Arc<Mutex<Inner>>,
}

impl Default for InMemoryScheduler {
    fn default() -> Self {
        Self::with_capacity(64)
    }
}

impl InMemoryScheduler {
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                capacity: cap.max(1),
                jobs: Vec::new(),
                in_flight: 0,
                insertion: VecDeque::new(),
            })),
        }
    }

    /// 入队. `forced_id` = None 时分配新 UUID; `initial_state` 通常是
    /// `Pending` 或 `Queued` (Pending 表示尚未分配 in_flight 槽位).
    pub fn enqueue(
        &self,
        kind: impl Into<String>,
        forced_id: Option<Uuid>,
        initial_state: JobState,
    ) -> Result<ScheduledJob, SchedulerError> {
        let kind = kind.into();
        let id = forced_id.unwrap_or_else(Uuid::new_v4);

        let mut g = self.inner.lock();
        // 重复 ID 视为更新 (与业务版"duplicate id rejected"不同, 这里
        // 保持 mock 宽松, sample 测出来行为更可预测).
        if let Some(j) = g.jobs.iter_mut().find(|j| j.id == id) {
            j.kind = kind;
            j.state = initial_state;
            return Ok(j.clone());
        }

        if g.in_flight >= g.capacity
            && matches!(initial_state, JobState::Pending | JobState::Queued | JobState::Running)
        {
            return Err(SchedulerError::QueueFull(g.capacity));
        }

        let job = ScheduledJob {
            id,
            kind,
            state: initial_state,
        };
        if !matches!(initial_state, JobState::Succeeded | JobState::Failed | JobState::Cancelled)
        {
            g.in_flight += 1;
        }
        g.jobs.push(job.clone());
        g.insertion.push_back(id);
        Ok(job)
    }

    /// 显式状态转移 — 返回更新后的快照.
    pub fn transition(&self, id: Uuid, to: JobState) -> Result<ScheduledJob, SchedulerError> {
        let mut g = self.inner.lock();
        // 先取出 from 状态做合法性检查, 再决定 in_flight 是否减 1, 最后写回 state.
        let from = {
            let job = g
                .jobs
                .iter()
                .find(|j| j.id == id)
                .ok_or(SchedulerError::JobNotFound(id))?;
            job.state
        };
        assert_legal(from, to)?;
        let was_in_flight = !is_terminal(from);
        let will_be_in_flight = !is_terminal(to);
        if was_in_flight && !will_be_in_flight {
            g.in_flight = g.in_flight.saturating_sub(1);
        }
        let job = g.jobs.iter_mut().find(|j| j.id == id).expect("present");
        job.state = to;
        Ok(job.clone())
    }

    pub fn state_of(&self, id: Uuid) -> Result<JobState, SchedulerError> {
        let g = self.inner.lock();
        g.jobs
            .iter()
            .find(|j| j.id == id)
            .map(|j| j.state)
            .ok_or(SchedulerError::JobNotFound(id))
    }

    pub fn in_flight(&self) -> usize {
        self.inner.lock().in_flight
    }

    pub fn capacity(&self) -> usize {
        self.inner.lock().capacity
    }

    /// FIFO 顺序列出所有 job 快照 (便于断言顺序).
    pub fn snapshot(&self) -> Vec<ScheduledJob> {
        let g = self.inner.lock();
        g.insertion
            .iter()
            .filter_map(|id| g.jobs.iter().find(|j| j.id == *id).cloned())
            .collect()
    }
}

fn is_terminal(s: JobState) -> bool {
    matches!(s, JobState::Succeeded | JobState::Failed | JobState::Cancelled)
}

fn assert_legal(from: JobState, to: JobState) -> Result<(), SchedulerError> {
    use JobState::*;
    let ok = matches!(
        (from, to),
        (Pending, Queued)
            | (Pending, Cancelled)
            | (Queued, Running)
            | (Queued, Cancelled)
            | (Running, Succeeded)
            | (Running, Failed)
            | (Running, Cancelled)
    );
    if ok {
        Ok(())
    } else {
        Err(SchedulerError::IllegalTransition {
            from: from.as_str().to_string(),
            to: to.as_str().to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn happy_path_pending_to_succeeded() {
        let s = InMemoryScheduler::with_capacity(4);
        let j = s.enqueue("k", None, JobState::Pending).unwrap();
        s.transition(j.id, JobState::Queued).unwrap();
        s.transition(j.id, JobState::Running).unwrap();
        let final_ = s.transition(j.id, JobState::Succeeded).unwrap();
        assert_eq!(final_.state, JobState::Succeeded);
        assert_eq!(s.in_flight(), 0);
    }

    #[test]
    fn illegal_transition_rejected() {
        let s = InMemoryScheduler::with_capacity(4);
        let j = s.enqueue("k", None, JobState::Pending).unwrap();
        // Pending -> Running 非法
        let err = s.transition(j.id, JobState::Running).unwrap_err();
        assert!(matches!(err, SchedulerError::IllegalTransition { .. }));
    }

    #[test]
    fn capacity_enforced() {
        let s = InMemoryScheduler::with_capacity(2);
        let _a = s.enqueue("a", None, JobState::Pending).unwrap();
        let _b = s.enqueue("b", None, JobState::Pending).unwrap();
        let err = s.enqueue("c", None, JobState::Pending).unwrap_err();
        assert!(matches!(err, SchedulerError::QueueFull(2)));
    }

    #[test]
    fn terminal_release_slot() {
        let s = InMemoryScheduler::with_capacity(1);
        let j = s.enqueue("k", None, JobState::Pending).unwrap();
        s.transition(j.id, JobState::Queued).unwrap();
        s.transition(j.id, JobState::Running).unwrap();
        s.transition(j.id, JobState::Succeeded).unwrap();
        assert_eq!(s.in_flight(), 0);
        // 槽位释放, 可再入队
        let _b = s.enqueue("k2", None, JobState::Pending).unwrap();
        assert_eq!(s.in_flight(), 1);
    }

    #[test]
    fn snapshot_is_fifo() {
        let s = InMemoryScheduler::with_capacity(8);
        let a = s.enqueue("a", None, JobState::Pending).unwrap();
        let b = s.enqueue("b", None, JobState::Pending).unwrap();
        let snap = s.snapshot();
        assert_eq!(snap[0].id, a.id);
        assert_eq!(snap[1].id, b.id);
    }
}
