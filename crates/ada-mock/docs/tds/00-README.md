# ada-mock — 测试脚手架 README

> 状态: **v0.1.0 草案** (2026-08-31 立)
> 决策人: Ulysses (per 8/27 21:59 JST 第三次代签授权, Mavis 接手)
> 责任: Mavis (架构代理, per DEC-008)

## 0. 项目定位

### 0.1 一句话
**专门用于测试的脚手架 crate** — 包含 in-memory mock、黄金 fixture、HTTP/OTLP 拦截、测试设计书模板与覆盖率报告脚本。**不接入任何业务 crate 的 dev-dep**, 仅作独立示例供后续写测试的人抄模板。

### 0.2 为什么独立
Ada 现有 19 个业务 crate 已经有 700+ 个测试, 但缺:
- 跨模块**共用**的 mock 工厂 (现在每个 crate 各自造轮子)
- 标准**黄金集**格式 (NDJSON / envelope schema_version 都没有规范)
- **HTTP/OTLP** 拦截的统一方案 (ada-m09 exporter 自己造了 5xx 监听)
- 测试**设计书**模板 (TDS) 和**报告**脚本

如果把这些"测试基础设施"塞进某个业务 crate, 会:
- 污染生产编译产物
- 隐式建立 crate 间契约, 让"重构接口"变难
- 跨多 crate 复用时需要重复 `dev-dependencies` 配置

### 0.3 决策记录 (DR)
| 编号 | 决策 | 替代方案 | 选择理由 |
|---|---|---|---|
| DR-001 | 独立 crate (同 workspace) | (a) 散布到各 crate 的 tests/common (b) 外部 git 子模块 | 同 workspace 享受 `cargo test` 一把跑, 但不污染生产依赖 |
| DR-002 | 不接入业务 crate dev-dep | 通过 `path = "../ada-mock"` 引入 | 避免隐式契约; 框架可独立演化 |
| DR-003 | 纯同步 + 无 tokio | 引入 async runtime | 强制 mock 保持可预测; sample 测试能在 0 个 async 上下文下写 |
| DR-004 | `server` feature 隔离 TcpListener | 默认依赖 | 默认 build 快; 不需要 server 的人不付 TcpListener 编译成本 |
| DR-005 | 4 能力层一次性出 | 仅 mock 或 仅 fixture | 用户要求"都要"; 一次性铺平, 后续逐层扩展 |
| DR-006 | TDS 用 Markdown + 模板字段 | 接入专门的 TDS 工具 (TestRail / qTest) | 离线 + git 友好; 后续可导入 |

## 1. 四能力层

| 层 | 模块 | 入口 | 对应业务能力 |
|---|---|---|---|
| 1. Mock 资源 | `src/mocks/` | `InMemoryEventBus` / `InMemoryScheduler` / `StubConnector` | m15 event-bus / m04 orchestration / m01 acquisition |
| 2. HTTP/Tracing 拦截 | `src/server/` (feature `server`) | `FakeOtlpServer` | m09 exporter push 路径 |
| 3. 黄金集 fixture | `src/fixtures/` + `tests/fixtures/` | `load_envelope` / `load_ndjson` / `golden_event()` | 跨模块回归 |
| 4. TDS + 报告 | `docs/tds/` + `scripts/` | 模板 + PowerShell/Python 脚本 | 评审 + CI |

## 2. 快速开始

```bash
# 跑 mock crate 自己的所有测试 (含 server feature)
cargo test -p ada-mock --all-features

# 单独跑 sample 集成测试
cargo test -p ada-mock --test sample_mock_usage --all-features

# 在 IDE 调试某个 mock
cargo test -p ada-mock --lib mocks::scheduler::tests::capacity_enforced
```

## 3. 不要做的事

- ❌ 不要 `pub use ada_m09_exporter::*` (引入业务类型, 违反"独立"原则)
- ❌ 不要在 mock crate 里写 `tokio::test` (破坏同步约定)
- ❌ 不要把黄金集 fixture 写成 Rust const (失去 git diff 友好性)
- ❌ 不要修改 `tests/fixtures/*.envelope.json` 的 `schema_version` 而不写迁移脚本
- ❌ 不要在业务 crate 的 `Cargo.toml` 里加 `ada-mock = { path = ... }` (per DR-002)

## 4. 版本历史

| 版本 | 日期 | 变更 |
|---|---|---|
| 0.1.0 | 2026-08-31 | 初版: 4 能力层全部铺平, 28 unit + 2 integration 测试通过 |
