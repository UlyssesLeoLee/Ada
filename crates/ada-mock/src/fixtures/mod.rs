//! 黄金集 fixture 模块.
//!
//! 4 能力层之第 3 层: 静态 JSON/NDJSON 样本, 用于回归测试.
//!
//! ## 加载策略
//! - **不**用 `include_str!` 嵌入式常量 — 我们要"在测试中能看到
//!   文件路径"的语义, 便于调试.
//! - 走 `load_envelope(path)` 显式从 tests/fixtures/ 加载.
//! - 加载时强制 `GoldenEnvelope::validate()` 拦截旧 schema.

mod golden;
mod loader;

pub use golden::golden_event;
pub use loader::{load_envelope, load_ndjson, FixturePath};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn golden_event_factory_returns_deterministic_value() {
        let a = golden_event("topic.x", 1);
        let b = golden_event("topic.x", 1);
        assert_eq!(a, b);
    }

    #[test]
    fn load_ndjson_rejects_missing_file() {
        let p = FixturePath::absolute("does-not-exist.ndjson");
        assert!(load_ndjson(&p).is_err());
    }
}
