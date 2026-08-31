//! 内存内"金标数据"工厂. 每次相同输入返回相同输出, 便于断言.

use serde_json::{json, Value};

/// 构造一条标准金标事件, 形状:
/// ```json
/// { "id": "<topic>:<seq>", "topic": "...", "payload": { ... }, "trace_id": null }
/// ```
pub fn golden_event(topic: &str, seq: u64) -> Value {
    json!({
        "id": format!("{}:{}", topic, seq),
        "topic": topic,
        "payload": { "seq": seq, "kind": "golden" },
        "trace_id": serde_json::Value::Null,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shape_has_required_fields() {
        let v = golden_event("t", 7);
        assert!(v.get("id").is_some());
        assert!(v.get("topic").is_some());
        assert!(v.get("payload").is_some());
        assert!(v.get("trace_id").is_some());
    }

    #[test]
    fn seq_propagates_to_payload() {
        let v = golden_event("t", 42);
        assert_eq!(v["payload"]["seq"], 42);
    }
}
