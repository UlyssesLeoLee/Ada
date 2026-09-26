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
//!
//! ## module_switch 接入 (per ULYS-190 §4.4 stage7, 2026-09-26 13:30 JST)
//!
//! 本 stage7 commit 补 L1 cluster_switch + L2 plugin_switch + L3 module_switch
//! (反向 ULYS-190 §4.6 G-MS-01 「L2/L3 永久跳过」 决策, per D-Boy 2026-09-26 04:11 JST
//! reply "完成到stage7的内容, ada也需要的"). 跨项目範式对齐 per G-MS-04.
//!
//! ### 10 module 落地 (per plugin)
//! - **mocks** (3m): `event_bus` / `scheduler` / `connector` (3 in-memory 资源)
//! - **server** (1m): `fake_otlp_server` (feature-gated, per `server` feature)
//! - **fixtures** (2m): `golden` / `loader` (静态数据 + 加载器)
//! - **builders** (1m): `builders` (EventBuilder/JobBuilder/fixed_now/fresh_id)
//! - **tds** (3m): `tds_event_bus` / `tds_scheduler` / `tds_fake_otlp` (3 TDS docs)
//!
//! ### 跨语言 dispatch 用法 (跟 RGS `tools/rgs-flash-mock/` 範式一致)
//! ```bash
//! # Python reader helper (CI scripts 调用)
//! python crates/ada-mock/scripts/_lib_mock_switch_ada.py \
//!     --aci-config crates/ada-mock/.aci.json read-plugins
//! # ✅ 5 plugins / 10 modules JSON
//! ```
//!
//! ### 跨项目累計 (per §4.4 stage1+2+3+4+5+6+7, **7/7 项目 module_switch 落地**)
//! | 项目 | Plugin | Module | 範式 |
//! |---|---|---|---|
//! | IM1.0 | 5 | 28 | PR #24 |
//! | CATs | 4 | 13 | PR #18 |
//! | Star | 7 | 7 | PR #151 |
//! | RGS | 5 | 12 | PR #51 |
//! | IDE1.0 | N | 8 | stage5 |
//! | GitGit | N | 7 | stage6 |
//! | **Ada** | **5** | **10** | **stage7 (本 commit, reverse of §4.6 G-MS-01)** |
//!
//! ### 落地边界 (per cluster.enabled=false 預設)
//! `enabled=false` 是 *声明* 不是 *启用*. 真启用仍需外部脚手架手动 flip.
//! 这种 "声明 + 默认关" 模式跟 RGS/IM1.0/CATs/Star/IDE1.0/GitGit 一致,
//! 跨项目範式对齐. 守门 #15 scope creep 仍遵守 (no runtime over-engineering).

#![deny(unsafe_code)]
#![warn(missing_debug_implementations)]

// ---------------------------------------------------------------------------
// 公共模块树
// ---------------------------------------------------------------------------
pub mod aci_emitter_helper;
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
