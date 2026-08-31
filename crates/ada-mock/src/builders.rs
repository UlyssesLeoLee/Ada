//! 测试对象构造器 (Builder).
//!
//! 风格统一: 链式 `::new().with_xxx().build()`, 默认值偏真实业务, 便于 `unwrap_or_default()`.
//!
//! ## 不引入业务类型的硬规则
//! Builder 只产出**自有类型** (见 `mocks::*` / `fixtures::*`),
//! 不允许构造 `ada_m01_acquisition::Source` 之类的业务类型 —
//! 那意味着两个 crate 之间出现隐式契约, 违反"独立项目"定位.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::mocks::{InMemoryEvent, InMemoryEventBus, InMemoryScheduler, JobState, ScheduledJob, SchedulerError};
use crate::Result;

// ---------------------------------------------------------------------------
// 通用工具
// ---------------------------------------------------------------------------

/// 固定时钟 (UTC) — 全部 fixture 用同一时间锚, 避免时间漂移.
pub fn fixed_now() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-08-31T00:00:00Z")
        .expect("static rfc3339")
        .with_timezone(&Utc)
}

/// 随机但稳定的 UUID (v4) — 不带时钟序, 仅用于区分对象身份.
pub fn fresh_id() -> Uuid {
    Uuid::new_v4()
}

// ---------------------------------------------------------------------------
// EventBuilder
// ---------------------------------------------------------------------------

/// 构造发布到 [`InMemoryEventBus`] 的事件.
#[derive(Debug, Clone)]
pub struct EventBuilder {
    topic: String,
    payload: serde_json::Value,
    trace_id: Option<String>,
}

impl EventBuilder {
    pub fn new(topic: impl Into<String>) -> Self {
        Self {
            topic: topic.into(),
            payload: serde_json::json!({}),
            trace_id: None,
        }
    }

    pub fn with_payload(mut self, payload: serde_json::Value) -> Self {
        self.payload = payload;
        self
    }

    pub fn with_trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = Some(trace_id.into());
        self
    }

    /// 发布到 bus, 同步返回生成的 `InMemoryEvent` (含分配的事件 ID).
    pub fn publish(self, bus: &InMemoryEventBus) -> Result<InMemoryEvent> {
        bus.publish(self.topic, self.payload, self.trace_id)
    }
}

// ---------------------------------------------------------------------------
// JobBuilder
// ---------------------------------------------------------------------------

/// 构造调度到 [`InMemoryScheduler`] 的 job.
#[derive(Debug, Clone)]
pub struct JobBuilder {
    id: Option<Uuid>,
    kind: String,
    initial_state: JobState,
}

impl JobBuilder {
    pub fn new(kind: impl Into<String>) -> Self {
        Self {
            id: None,
            kind: kind.into(),
            initial_state: JobState::Pending,
        }
    }

    pub fn with_id(mut self, id: Uuid) -> Self {
        self.id = Some(id);
        self
    }

    pub fn starting_in(mut self, state: JobState) -> Self {
        self.initial_state = state;
        self
    }

    /// 入队 — 调度器分配 ID, 返回 `ScheduledJob` 句柄.
    /// 错误用 `SchedulerError` 表示, 通过 `From` 桥接到 crate 顶级 `Result`.
    pub fn enqueue(self, sched: &mut InMemoryScheduler) -> std::result::Result<ScheduledJob, SchedulerError> {
        sched.enqueue(self.kind, self.id, self.initial_state)
    }
}

// ---------------------------------------------------------------------------
// JsonFixtureBuilder — 通用 JSON 形状 (供黄金集加载使用)
// ---------------------------------------------------------------------------

/// 黄金集文件标准 schema:
/// ```json
/// { "schema_version": 1, "name": "...", "events": [ ... ] }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GoldenEnvelope {
    pub schema_version: u32,
    pub name: String,
    pub events: Vec<serde_json::Value>,
}

impl GoldenEnvelope {
    /// 校验 schema_version 落在受支持范围 (避免读到旧文件沉默通过).
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != 1 {
            return Err(crate::MockError::FixtureParse(format!(
                "unsupported schema_version {} (expected 1)",
                self.schema_version
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_now_is_stable() {
        assert_eq!(fixed_now(), fixed_now());
    }

    #[test]
    fn event_builder_publishes_with_default_payload() {
        let bus = InMemoryEventBus::default();
        let ev = EventBuilder::new("test.topic")
            .publish(&bus)
            .expect("publish");
        assert_eq!(ev.topic, "test.topic");
    }

    #[test]
    fn job_builder_uses_generated_id_when_omitted() {
        let mut sched = InMemoryScheduler::default();
        let a = JobBuilder::new("k").enqueue(&mut sched).expect("enq a");
        let b = JobBuilder::new("k").enqueue(&mut sched).expect("enq b");
        assert_ne!(a.id, b.id);
    }

    #[test]
    fn golden_envelope_rejects_wrong_schema_version() {
        let env = GoldenEnvelope {
            schema_version: 99,
            name: "x".into(),
            events: vec![],
        };
        assert!(env.validate().is_err());
    }
}
