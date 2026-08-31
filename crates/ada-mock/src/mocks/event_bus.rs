//! InMemoryEventBus — 业务 ada-m15 event-bus 的"形状相似"克隆.
//!
//! 接口形状 (publish / subscribe / recv) 与业务版本一致, 但**没有 glob
//! topic match 优化** — 这里只支持精确 topic. 业务版本做 `*`/`#` 通配,
//! sample 测试需要 glob 时, 应显式在 sample 内自己写 pattern loop,
//! 不要把这个 mock 撑大.

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::{MockError, Result};

/// 一条事件快照.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InMemoryEvent {
    pub id: Uuid,
    pub topic: String,
    pub payload: serde_json::Value,
    pub trace_id: Option<String>,
    /// 业务版本有"入队时间", 这里简化为单调自增 i64, 便于断言顺序.
    pub seq: i64,
}

/// 订阅者 ID (返回的句柄可调用 `unsubscribe`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SubscriberId(pub Uuid);

/// 默认实现是空 bus.
#[derive(Debug, Default, Clone)]
pub struct InMemoryEventBus {
    inner: Arc<Mutex<Inner>>,
}

#[derive(Debug)]
struct Inner {
    next_seq: i64,
    subscribers: Vec<Subscriber>,
    /// 每个订阅者一个 FIFO 队列, publish 时扇出.
    queues: Vec<VecDeque<InMemoryEvent>>,
}

impl Default for Inner {
    fn default() -> Self {
        Self {
            next_seq: 0,
            subscribers: Vec::new(),
            queues: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
struct Subscriber {
    id: SubscriberId,
    topic: String,
}

impl InMemoryEventBus {
    pub fn new() -> Self {
        Self::default()
    }

    /// 返回当前活跃订阅者数量.
    pub fn subscriber_count(&self) -> usize {
        self.inner.lock().subscribers.len()
    }

    /// 订阅精确 topic. 返回 `SubscriberId` 用于取消.
    pub fn subscribe(&self, topic: impl Into<String>) -> Result<SubscriberId> {
        let topic = topic.into();
        if topic.is_empty() {
            return Err(MockError::InvariantViolated(
                "topic must be non-empty".into(),
            ));
        }
        let mut g = self.inner.lock();
        let id = SubscriberId(Uuid::new_v4());
        g.subscribers.push(Subscriber { id, topic: topic.clone() });
        g.queues.push(VecDeque::new());
        Ok(id)
    }

    /// 取消订阅; 重复取消视为 no-op 并返回 false.
    pub fn unsubscribe(&self, id: SubscriberId) -> bool {
        let mut g = self.inner.lock();
        let Some(pos) = g.subscribers.iter().position(|s| s.id == id) else {
            return false;
        };
        g.subscribers.remove(pos);
        g.queues.remove(pos);
        true
    }

    /// 发布一条事件. 扇出到所有匹配精确 topic 的订阅者队列.
    pub fn publish(
        &self,
        topic: impl Into<String>,
        payload: serde_json::Value,
        trace_id: Option<String>,
    ) -> Result<InMemoryEvent> {
        let topic = topic.into();
        if topic.is_empty() {
            return Err(MockError::InvariantViolated(
                "publish topic must be non-empty".into(),
            ));
        }
        let mut g = self.inner.lock();
        g.next_seq += 1;
        let ev = InMemoryEvent {
            id: Uuid::new_v4(),
            topic: topic.clone(),
            payload,
            trace_id,
            seq: g.next_seq,
        };
        // 先收集匹配下标, 再分批 push_back — 避免在同一循环里同时借用 subscribers/queues.
        let targets: Vec<usize> = g
            .subscribers
            .iter()
            .enumerate()
            .filter_map(|(i, s)| if s.topic == topic { Some(i) } else { None })
            .collect();
        for i in targets {
            g.queues[i].push_back(ev.clone());
        }
        Ok(ev)
    }

    /// 拉取下一条事件, 队列空时返回 `None`.
    pub fn try_recv(&self, id: SubscriberId) -> Result<Option<InMemoryEvent>> {
        let mut g = self.inner.lock();
        let pos = g
            .subscribers
            .iter()
            .position(|s| s.id == id)
            .ok_or(MockError::CaptureClosed)?;
        Ok(g.queues[pos].pop_front())
    }

    /// 调试用: 复制当前每个订阅者队列长度.
    pub fn queue_depths(&self) -> Vec<usize> {
        self.inner.lock().queues.iter().map(|q| q.len()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subscribe_publish_recv_round_trip() {
        let bus = InMemoryEventBus::new();
        let sub = bus.subscribe("a.b").unwrap();
        let ev = bus.publish("a.b", serde_json::json!({"k": 1}), None).unwrap();
        let got = bus.try_recv(sub).unwrap().expect("one event");
        assert_eq!(got.id, ev.id);
        assert_eq!(got.seq, 1);
    }

    #[test]
    fn topic_mismatch_does_not_fan_out() {
        let bus = InMemoryEventBus::new();
        let sub_a = bus.subscribe("a").unwrap();
        let _sub_b = bus.subscribe("b").unwrap();
        bus.publish("a", serde_json::json!({}), None).unwrap();
        assert_eq!(bus.queue_depths(), vec![1, 0]);
        let got = bus.try_recv(sub_a).unwrap();
        assert!(got.is_some());
    }

    #[test]
    fn seq_is_monotonic_per_bus() {
        let bus = InMemoryEventBus::new();
        // 先订阅再发布 — mock 是 fan-out 而不是回放, 订阅前的事件不会投递.
        let sub = bus.subscribe("x").unwrap();
        let _ = bus.publish("x", serde_json::json!({}), None).unwrap();
        let _ = bus.publish("x", serde_json::json!({}), None).unwrap();
        let e1 = bus.try_recv(sub).unwrap().unwrap();
        let e2 = bus.try_recv(sub).unwrap().unwrap();
        assert!(e2.seq > e1.seq);
    }

    #[test]
    fn unsubscribe_drops_subscriber() {
        let bus = InMemoryEventBus::new();
        let sub = bus.subscribe("x").unwrap();
        assert_eq!(bus.subscriber_count(), 1);
        assert!(bus.unsubscribe(sub));
        assert_eq!(bus.subscriber_count(), 0);
        assert!(!bus.unsubscribe(sub));
    }

    #[test]
    fn empty_topic_rejected() {
        let bus = InMemoryEventBus::new();
        assert!(bus.subscribe("").is_err());
        assert!(bus.publish("", serde_json::json!({}), None).is_err());
    }
}
