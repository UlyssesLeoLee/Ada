# `ada-mock` — 独立测试脚手架 crate

> **目的**: 给 Ada 项目提供"抄模板、抄 fixture、抄 mock"的统一脚手架
> **不接入**: 不通过 `dev-dependencies` 被业务 crate 引入 (per `docs/tds/00-README.md` DR-002)
> **四能力层**: Mock 资源 / HTTP 拦截 / 黄金集 / TDS+报告
> **module_switch stage7 (per ULYS-190 §4.4)**: L1 cluster + L2 plugin + L3 module 全部落地, 5p × 10m

## 5 分钟入门

```bash
# 跑全部测试 (含 server feature)
cargo test -p ada-mock --all-features

# 跑覆盖率 (需要 nightly + cargo-llvm-cov)
pwsh scripts/coverage_report.ps1 -Threshold 80

# 列 TDS 状态
python scripts/list_tds.py

# mock_switch reader CLI (stage7, per ULYS-190 §4.4)
python crates/ada-mock/scripts/_lib_mock_switch_ada.py \
    --aci-config crates/ada-mock/.aci.json read-plugins
```

## 模块地图

| 路径 | 说明 |
|---|---|
| `src/mocks/event_bus.rs` | `InMemoryEventBus` (精确-topic 扇出) |
| `src/mocks/scheduler.rs` | `InMemoryScheduler` (6 状态状态机 + capacity) |
| `src/mocks/connector.rs` | `StubConnector` (stdin/file/http 三合一) |
| `src/server/mod.rs` *(feature)* | `FakeOtlpServer` (本地 TcpListener, 录 raw+body) |
| `src/fixtures/` | `load_envelope` / `load_ndjson` + 黄金事件工厂 |
| `src/builders.rs` | `EventBuilder` / `JobBuilder` / `fixed_now` / `fresh_id` |
| `tests/fixtures/` | 真实黄金数据 (NDJSON + envelope JSON) |
| `tests/sample_mock_usage.rs` | 4 能力层一站式 smoke |
| `tests/test_ada_mock_switch.py` | stage7 mock_switch reader 测试 (23 unittest) |
| `docs/tds/` | 测试设计书: 00-README + TEMPLATE + 3 实例 |
| `docs/regression-report-stage1-module-switch-2026-09-26.md` | stage7 regression report |
| `scripts/` | coverage_report.ps1 / run_tests.ps1 / list_tds.py / _lib_mock_switch_ada.py |
| `.aci.json` | ACI 契约 v0.1 + plugin × module 10 module 声明 |
| `.mock-cluster.json` | cluster_switch v0.1 (enabled=false 預設因 testkit) |

## mock_switch reader CLI (stage7)

```bash
python crates/ada-mock/scripts/_lib_mock_switch_ada.py \
    --aci-config crates/ada-mock/.aci.json is-enabled        # CLUSTER_ENABLED=false exit 1
python crates/ada-mock/scripts/_lib_mock_switch_ada.py \
    --aci-config crates/ada-mock/.aci.json get-mode          # CLUSTER_MODE=offline exit 0
python crates/ada-mock/scripts/_lib_mock_switch_ada.py \
    --aci-config crates/ada-mock/.aci.json trace             # 真拼接 plugins=[mocks(3m),server(1m),fixtures(2m),builders(1m),tds(3m)]=10/10 modules exit 0
python crates/ada-mock/scripts/_lib_mock_switch_ada.py \
    --aci-config crates/ada-mock/.aci.json validate-compat  # ACI_COMPAT=OK exit 0
python crates/ada-mock/scripts/_lib_mock_switch_ada.py \
    --aci-config crates/ada-mock/.aci.json read-plugins      # 5 plugins / 10 modules JSON exit 0
```

| Subcommand | Exit 0 | Exit 1 | 跨项目範式 |
|---|---|---|---|
| `is-enabled` | `cluster.enabled=true` | `cluster.enabled=false` | Star/IM1.0/CATs/RGS/IDE1.0/GitGit 一致 |
| `get-mode` | 成功 | (无 FAIL 路径) | 一致 |
| `trace` | 成功 | (无 FAIL 路径) | 一致 |
| `validate-compat` | `compat OK` | `compat FAIL` | 一致 |
| `read-plugins` | 成功 JSON 输出 | (无 FAIL 路径, 仅 Python error) | 一致 (永远 exit 0 on success) |

## 与业务 crate 的边界

- ✅ 可以: 改 mock 内部实现、改 fixture 路径、改 TDS 模板
- ❌ 不可以: 在 `src/mocks/*` 里 `use ada_m09_exporter::*` 之类
- ❌ 不可以: 把 mock 通过 `path = "../ada-mock"` 加进业务 crate 的 dev-dep
- ❌ 不可以: 引入 tokio / axum / hyper / reqwest / tonic

## 维护

- 模块 Owner: Mavis (per DEC-008, 自审 + DDD Review 阶段补签字)
- 复审: 季度 / 接口变更即触发
- TDS 索引: 跑 `python scripts/list_tds.py` 看当前状态
- module_switch 跨项目累計 (per §4.4): 7/7 项目落地 (IM1.0/CATs/Star/RGS/IDE1.0/GitGit/Ada)
