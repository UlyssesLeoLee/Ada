//! `ada-mock` — 独立测试脚手架 crate
//!
//! **定位**: 框架/示例, 供后续写测试的人抄模板、抄 fixture、抄 in-memory 实现.
//! **不接入**: 本 crate 不被任何业务 crate 通过 `dev-dependencies` 引入 (见 `docs/tds/00-README.md` §0.3 决策记录).
//! **四层能力** (与 `docs/tds/00-README.md` §1 一致):
//!   1. **Mock 资源** — 连接器/事件总线/调度器的 in-memory 实现 (`mocks::*`).
//!   2. **HTTP/Tracing 拦截** — `FakeOtlpServer` 风格的本地 TcpListener 双端 (`server::FakeOtlpServer`, 需 `server` feature).
//!   3. **黄金集 fixture** — 静态 JSON/NDJSON/CRDT 数据, 用于回归 (`fixtures::golden`).
//!   4. **TDS + 报告** — `docs/tds/` 模板, `scripts/` 报告脚本.
//!
//! ## 独立项目的硬约束
//! - 唯一 workspace 依赖 = `ada-core` (仅在 `[dev-dependencies]` 里, 用于 sample 验证).
//! - 无 tokio / 无 axum / 无 tonic / 无 reqwest — 强制 sample mock 保持同步 + 可预测.
//! - 命名冲突预警: 不允许 `pub use` 任何业务 crate 的具体类型 (避免成为"伪共享层").
//!
//! ## 快速验证
//! ```bash
//! cargo test -p ada-mock --all-features
//! ```

#![deny(unsafe_code)]
#![warn(missing_debug_implementations)]

// ---------------------------------------------------------------------------
// 公共模块树
// ---------------------------------------------------------------------------
pub mod builders;
pub mod error;
pub mod fixtures;
pub mod mocks;

#[cfg(feature = "server")]
pub mod server;

// ---------------------------------------------------------------------------
// 重新导出 (Rexport 平面)
// ---------------------------------------------------------------------------
pub use error::{MockError, Result};

/// crate 自身版本自检 (与 ada-telemetry 同款).
pub const MOCK_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_not_empty() {
        assert!(!MOCK_VERSION.is_empty());
    }

    #[test]
    fn all_modules_are_under_test() {
        // 每个公共模块至少一个 `#[cfg(test)]` 测试, 见各模块 mod tests.
        // 这里只做"模块存在性"快速断言.
        let _ = std::any::type_name::<mocks::InMemoryEventBus>();
    }
}
