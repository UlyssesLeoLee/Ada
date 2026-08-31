//! 三个 in-memory mock, 对应 4 能力层之第 1 层 (Mock 资源).
//!
//! | 类型                | 业务对应              | 适用模块                |
//! |---------------------|----------------------|-------------------------|
//! | `InMemoryEventBus`  | ada-m15 event-bus    | pub/sub + topic glob     |
//! | `InMemoryScheduler` | ada-m04 orchestration| 状态机 + capacity        |
//! | `StubConnector`     | ada-m01 acquisition  | stdin/file/http 抽象接口 |
//!
//! 这些 mock 是**纯本地**, 不引用任何业务 crate, 也不通过 trait
//! 与业务类型耦合 — 是"形状相似, 接口独立"的实现, 让 sample 测试
//! 能在 0 个上游依赖下完成验证.

mod connector;
mod event_bus;
mod scheduler;

pub use connector::{Record, StubConnector, StubKind};
pub use event_bus::{InMemoryEvent, InMemoryEventBus};
pub use scheduler::{InMemoryScheduler, JobState, ScheduledJob, SchedulerError};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn types_are_send_sync_static() {
        fn assert_bounds<T: Send + Sync + 'static>() {}
        assert_bounds::<InMemoryEventBus>();
        assert_bounds::<InMemoryScheduler>();
        assert_bounds::<StubConnector>();
    }
}
