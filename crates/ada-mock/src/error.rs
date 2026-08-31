//! 错误类型 — 整个 mock crate 共用一个 `MockError`.
//!
//! 设计原则:
//! - **Send + Sync + 'static** — 在测试 thread::spawn / rayon 并行下都可用.
//! - **不依赖 workspace 业务错误** — `ada-m09-exporter::ExporterError` 之类的我们不
//!   重新导出, 让 sample mock 保持"纯本地"特征.
//! - **`thiserror` 派生** — 与 ada-core / ada-telemetry 风格一致.

use thiserror::Error;

pub type Result<T> = std::result::Result<T, MockError>;

#[derive(Debug, Error)]
pub enum MockError {
    #[error("fixture file not found: {0}")]
    FixtureNotFound(String),

    #[error("fixture parse error: {0}")]
    FixtureParse(String),

    #[error("in-memory store invariant violated: {0}")]
    InvariantViolated(String),

    #[error("capture buffer closed")]
    CaptureClosed,

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_is_send_sync_and_static() {
        fn assert_bounds<T: Send + Sync + 'static>() {}
        assert_bounds::<MockError>();
    }

    #[test]
    fn display_includes_variant_context() {
        let e = MockError::InvariantViolated("queue full".into());
        assert!(e.to_string().contains("queue full"));
    }
}
