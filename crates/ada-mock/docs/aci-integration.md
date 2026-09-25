# Ada × ACI emitter 集成设计

> **ULYS-191 §4.3.4** — Ada 接入 `aci-emitter v0.1.0` 集成设计.
> Stage 3.1 最小可交付 (本笔).

## §1 背景

Ada (`D:/Ada`) 是 23+ crate monorepo (per `Cargo.toml`).
`crates/ada-mock/` 是独立测试脚手架 crate (per `Cargo.toml` description "Independent test scaffolding crate... NOT wired into production crates — sample/standalone only").

ada-mock 4 层能力:

| 模块 | 文件 | 职责 |
|---|---|---|
| `src/mocks/` | connector/event_bus/scheduler.rs | in-memory mocks (无 tokio) |
| `src/fixtures/` | golden/loader.rs | 静态 JSON/NDJSON/CRDT 黄金集 |
| `src/builders.rs` | — | factory builders |
| `src/server/` | (需 `server` feature) | FakeOtlpServer (TcpListener 双端) |
| `tests/sample_mock_usage.rs` | — | 现有 sample 测试 |
| `docs/tds/` | TDS-MOCK-2026-001/002/003.md | TDS 模板 |

per ULYS-191 §4.3.4 brief v0.1:
- Stage 3.1 最小可交付 (本笔): 加 `.aci.json` + `aci-emitter` git dep + 1 helper 模块 + 3 IT + 1 smoke
- Stage 3.2+ 后续 brief: 与现有 4 层能力集成

## §2 集成路径

### 2.1 依赖声明 (2 处)

**`Cargo.toml` (workspace 根)** — 在 `[workspace.dependencies]` 段加:

```toml
aci-emitter = { git = "https://github.com/UlyssesLeoLee/aci-emitter", rev = "df28c56" }
```

**`crates/ada-mock/Cargo.toml`** — 在 `[dependencies]` 段加:

```toml
aci-emitter = { workspace = true }
```

锁 `rev = df28c56` (= ULYS-191.1 main HEAD), 防止上游 master 推进时漂移.

### 2.2 字段 1:1 对齐

Ada 的 `.aci.json` 1:1 拷贝自 Star [`tools/star-flash-mock/.aci.json`](https://github.com/UlyssesLeoLee/Star/blob/main/tools/star-flash-mock/.aci.json).
所有字段名 (10 必填 + 2 条件)、4 测试层、5 严重度、4 状态、17 expect_value_types、6 scope_dimensions 全部一致.

### 2.3 Ada 端 Helper

`crates/ada-mock/src/aci_emitter_helper.rs` 暴露 1 个公开函数:

```rust
use ada_mock::aci_emitter_helper::emit_sample_assertion;
let a = emit_sample_assertion();
assert_eq!(a.assertion_id, "ada-mock:sample:g-1");
```

### 2.4 集成 seam (Stage 3.2+ 后续 brief)

`emit_sample_assertion()` 与 ada-mock 现有 4 层能力集成:
- `mocks::{InMemoryEventBus, ...}` 配对: emit 消息 + 用 factory 验
- `fixtures::{golden, loader}` 配对: emit fixture 加载 + 验
- `builders::*` 配对: emit builder 创建 + 验
- `server::FakeOtlpServer` (需 `server` feature) 配对: emit OTLP payload + 验

## §3 CI 跨项目验证

### 3.1 当前 CI 范围 (Ada 仓)

Ada 仓 CI 配置不在本笔范围, 由后续 brief 处理.
Ada **无 branch protection** (per `gh api /branches/main/protection` 返回 404), PR merge 流程简化.

### 3.2 aci-emitter 上游变更同步

Ada CI **不主动** 监听 aci-emitter upstream 变更, 但:

- (a) `aci-emitter = { git = "...", rev = "df28c56" }` 锁 rev, 上游 master 推进不影响 Ada
- (b) 上游 release tag 时人工 bump `rev`, 走 PR 流程
- (c) §4.5 跨项目 CI 落地后, 改 crates.io publish 或 path 依赖

## §4 未来 (per Stage 3 报告)

| 阶段 | brief | 范围 |
|---|---|---|
| §4.3.1 | ULYS-191.3 (ULYS-226) | RGS 集成 ✅ SHIPPED |
| §4.3.2 | ULYS-191.4 (ULYS-233) | CATs 集成 ✅ SHIPPED |
| §4.3.3 | ULYS-191.5 (ULYS-227) | IM1.0 集成 ✅ SHIPPED |
| §4.3.4 | **本笔 (ULYS-191.6 / ULYS-236)** | Ada 集成 (接 ada-mock) |
| §4.3.5 | ULYS-191.7 | GitGit 集成 (TS emitter) |

## §5 风险

| # | 风险 | 缓解 |
|---|---|---|
| R-1 | **git dep aci-emitter 跨项目漂移** | 锁 rev + §4.5 跨项目 CI 监听 |
| R-2 | **Ada workspace 23+ crates 与 ada-mock 不共享 deps** | 本笔仅影响 ada-mock 子 crate |
| R-3 | **ada-mock "NOT wired into production crates"**: ada-mock 不被业务 crate 通过 dev-dependencies 引入, 本笔 aci-emitter 加到 ada-mock 仅影响 ada-mock 本身 | 本笔仅加到 ada-mock, 文档明确标注 |
| R-4 | **Ada 当前在 `dev` 分支**: 本笔 PR base 应是 `dev` | PR 创建时 base=`dev` |

## §6 参考

- [ULYS-191 §4.3.4 brief v0.1](./regression-report-2026-09-24.md)
- [aci-emitter v0.1.0 (ULYS-224)](https://github.com/UlyssesLeoLee/aci-emitter)
- [Star `.aci.json` schema v0.1](https://github.com/UlyssesLeoLee/Star/blob/main/tools/star-flash-mock/.aci.json)
- [IDE1.0 §4.2.2 集成设计](https://github.com/UlyssesLeoLee/IDE1.0) — 参考 pattern
- [RGS §4.3.1 集成设计](https://github.com/UlyssesLeoLee/RustGameServer) — 参考 pattern
- [IM1.0 §4.3.3 集成设计](https://github.com/UlyssesLeoLee/IM1.0) — 参考 pattern
- [CATs §4.3.2 集成设计](https://github.com/UlyssesLeoLee/CATs) — 参考 pattern
- ada-mock 设计书: `crates/ada-mock/docs/tds/00-README.md`
