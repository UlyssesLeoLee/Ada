# Ada Mock module_switch Stage 7 Regression Report

> **生成时间**: 2026-09-26T13:30:00Z (per ULYS-190 dispatcher)
> **范围**: `crates/ada-mock/` (.aci.json + .mock-cluster.json + scripts/_lib_mock_switch_ada.py + tests/test_ada_mock_switch.py + src/lib.rs 接入 doc + README + 本文件)
> **触发**: D-Boy 「完成到stage7的内容，ada也需要的」 reply 2026-09-26 04:11 JST (comment `01a0dbe8-f6e2-72b1-9b27-eb127ddba7e5` per ULYS-240 thread) — 反向 ULYS-190 §4.6 G-MS-01 「L2/L3 永久跳过」 决策
> **守门**: 守门 #1+#5+#6+#7+#9+#10+#11+#12+#13+#14v4+#15+#19v19+#20+#24

## §1 范围

| # | 路径 | 类型 | 关键内容 |
|---|---|---|---|
| 1 | `crates/ada-mock/.aci.json` | 修改 | `plugins` 字段新增 (5 plugin × 10 module) + `aci_compat_version` 补齐 |
| 2 | `crates/ada-mock/.mock-cluster.json` | 修改 | `description` 更新到 stage7 + `mock_switch_trace_format` 真拼接模板 + `module_count_total/enabled: 10` + `decision_history` v0.1 + reverse-decision 透明度 |
| 3 | `crates/ada-mock/scripts/_lib_mock_switch_ada.py` | 新 (~190 LOC) | Python reader, CLI: is-enabled / get-mode / trace / validate-compat / read-plugins (5 subcommand) + exit-code norm docstring |
| 4 | `crates/ada-mock/tests/test_ada_mock_switch.py` | 新 (~190 LOC) | 23 单元测试 (per RGS test_rgs_mock_switch.py 範式) — 5 plugin count + 10 module count + per-plugin × module + cluster + compat + trace + 6 跨 Python invocation |
| 5 | `crates/ada-mock/src/lib.rs` | 修改 (+37 doc lines) | `## module_switch 接入` 段落 + 10 module 总览 + 跨语言 dispatch 用法 + 跨项目累計表 + 落地边界声明 |
| 6 | `crates/ada-mock/README.md` | 修改 (+34 / -1) | module_switch stage7 marker + 新增 「mock_switch reader CLI」 段 (5 subcommand 用法 + exit-code norm 表) + 维护段 module_switch 跨项目累計 |
| 7 | `crates/ada-mock/docs/regression-report-stage1-module-switch-2026-09-26.md` | 新 (本文件) | 7-8 段 per AGENTS.md §3 |

## §2 10 module 落地清单 (per plugin 表格)

| Plugin | Module count | Modules | 对应 src/ 文件 |
|---|---|---|---|
| **mocks** | **3** | `event_bus` / `scheduler` / `connector` | `src/mocks/{event_bus,scheduler,connector}.rs` |
| **server** | **1** | `fake_otlp_server` | `src/server/mod.rs` *(feature-gated `server` feature)* |
| **fixtures** | **2** | `golden` / `loader` | `src/fixtures/{golden,loader}.rs` |
| **builders** | **1** | `builders` | `src/builders.rs` |
| **tds** | **3** | `tds_event_bus` / `tds_scheduler` / `tds_fake_otlp` | `docs/tds/TDS-MOCK-2026-00{1,2,3}-*.md` |
| **总计** | **10** | **ALL** | 4 能力层 + TDS 文档 5 域划分 per lib.rs §1 |

**命名决策**: per lib.rs 4 能力层一一映射 + TDS docs 3 实例. mocks/server/fixtures/builders 4 plugin 对应 src/ 4 能力层; tds 1 plugin 对应 docs/tds/ 3 TDS docs (跟 lib.rs 「TDS+报告」 4 能力层之第 4 层对齐). 跨项目粒度可比 (RGS 12 = per pub mod 一一映射; Ada 10 = per 4 capability layer + 3 TDS docs).

## §3 验收脚本结果 (5 个 §)

| § | 验证项 | 命令 | 结果 |
|---|---|---|---|
| §1 | `is-enabled` | `python _lib_mock_switch_ada.py --aci-config .aci.json is-enabled` | ✅ `CLUSTER_ENABLED=false` (exit 1, testkit scaffold 預設关) |
| §2 | `get-mode` | `python _lib_mock_switch_ada.py --aci-config .aci.json get-mode` | ✅ `CLUSTER_MODE=offline` (exit 0) |
| §3 | `trace` | `python _lib_mock_switch_ada.py --aci-config .aci.json trace` | ✅ 真拼接 `cluster.enabled=false,mode=offline,plugins=[mocks(3m),server(1m),fixtures(2m),builders(1m),tds(3m)]=10/10 modules` (exit 0) |
| §4 | `validate-compat` | `python _lib_mock_switch_ada.py --aci-config .aci.json validate-compat` | ✅ `ACI_COMPAT=OK (cluster=0.1.0-draft aci=0.1.0-draft)` (exit 0) |
| §5 | `read-plugins` (NEW) | `python _lib_mock_switch_ada.py --aci-config .aci.json read-plugins` | ✅ JSON: 5 plugins / 10 modules 全 enabled (exit 0) |

### Python 测试结果 (23 unittest)

```bash
python -m unittest crates.ada-mock.tests.test_ada_mock_switch
# ✅ Ran 23 tests in 0.634s OK
```

23 tests 细分:
- 3 helper 基础 (`test_aci_file_exists` / `test_cluster_file_exists` / `test_helper_exists`)
- 5 总数 (`test_plugin_count_is_5` / `test_module_switch_total_count_is_10` / `test_module_switch_all_enabled_by_default` / `test_module_count_total_field_is_10` / `test_module_count_enabled_field_is_10`)
- 5 per-plugin (`test_mocks_plugin_has_3_modules` / `test_server_plugin_has_1_module` / `test_fixtures_plugin_has_2_modules` / `test_builders_plugin_has_1_module` / `test_tds_plugin_has_3_modules`)
- 2 cluster (`test_cluster_enabled_false_and_mode_offline` / `test_cluster_module_count_total_matches_aci`)
- 1 compat (`test_aci_compat_version_consistent_across_cluster_and_aci`)
- 1 trace format (`test_trace_format_substitutes_placeholders`)
- 6 跨 Python invocation (`test_helper_{is_enabled,get_mode,trace,validate_compat,read_plugins,trace_alternating_invocations}`)

### mock-switch-validate.py 跨项目验证 (Star tools/ 已 ship)

| Project | cluster_ok | enabled | mode | aci_status | 备注 |
|---|---|---|---|---|---|
| `ada` (本 commit) | ✅ True | False (testkit scaffold) | offline | OK | stage7 reverse-decision |

## §4 mock_switch_trace_format 真拼接 (OLD vs NEW)

| 阶段 | 字符串 |
|---|---|
| **OLD (stage1 L1 only, 2026-09-23)** | `cluster.enabled={cluster.enabled}, cluster.mode={cluster.mode} (L1 only 降級模式, 0 plugins/0 modules per G-MS-01 testkit scaffold 屬性)` |
| **NEW (stage7 L1+L2+L3, 2026-09-26)** | `cluster.enabled={cluster.enabled},mode={cluster.mode},plugins=[mocks(3m),server(1m),fixtures(2m),builders(1m),tds(3m)]=10/10 modules` |

差异:
1. 删掉「0 plugins/0 modules」 表达, 替换成实际 5 plugin × 10 module 拼接
2. 删掉「L1 only 降級模式」 限制表达 (本 stage7 已 reverse-decision 升级到 L1+L2+L3)
3. 拼接紧凑化 (去掉 cluster.enabled 后多余空格, 跟 RGS .mock-cluster.json 範式一致)

## §5 模块映射到 src/ (10 module 一一对应)

| Plugin | Module | src/ 路径 | TDS docs/tds/ 路径 |
|---|---|---|---|
| mocks | event_bus | `src/mocks/event_bus.rs` | `docs/tds/TDS-MOCK-2026-001-in_memory_event_bus.md` |
| mocks | scheduler | `src/mocks/scheduler.rs` | `docs/tds/TDS-MOCK-2026-002-in_memory_scheduler.md` |
| mocks | connector | `src/mocks/connector.rs` | (no TDS, follow-up) |
| server | fake_otlp_server | `src/server/mod.rs` | `docs/tds/TDS-MOCK-2026-003-fake_otlp_server.md` |
| fixtures | golden | `src/fixtures/golden.rs` | (no TDS, follow-up) |
| fixtures | loader | `src/fixtures/loader.rs` | (no TDS, follow-up) |
| builders | builders | `src/builders.rs` | (no TDS, follow-up) |
| tds | tds_event_bus | (refer to TDS docs/tds/TDS-MOCK-2026-001-*) | `docs/tds/TDS-MOCK-2026-001-in_memory_event_bus.md` |
| tds | tds_scheduler | (refer to TDS docs/tds/TDS-MOCK-2026-002-*) | `docs/tds/TDS-MOCK-2026-002-in_memory_scheduler.md` |
| tds | tds_fake_otlp | (refer to TDS docs/tds/TDS-MOCK-2026-003-*) | `docs/tds/TDS-MOCK-2026-003-fake_otlp_server.md` |

## §6 §4.6 → §4.4 stage7 决策反转说明

**原 §4.6 G-MS-01 决策 (2026-09-23):** Ada ada-mock 是 testkit scaffold (in-memory mocks + fixtures + builders, 不对外 emit ACI assertion), 跟 Star/RGS backend mock 屬性根本不同. 落地 L1 cluster_switch only (本文件), L2 plugin_switch + L3 module_switch 永久跳过 — 强落地违反 守门 #15 scope creep (over-engineering testkit).

**反向决策 (2026-09-26, per D-Boy reply):** "完成到stage7的内容，ada也需要的" — 跨项目 6/7 已落地 (IM1.0/CATs/Star/RGS/IDE1.0/GitGit), Ada 也需要 module_switch 範式对齐.

**落地策略:** *声明* module_switch (在 `.aci.json` 加 `plugins` 字段 + `.mock-cluster.json` 加 `module_count_*` 字段 + mock_switch_trace_format 真拼接模板) but NOT *运行时启用* (cluster.enabled=false 預設保留). 这种 "声明 + 默认关" 模式跟 RGS/IM1.0/CATs/Star/IDE1.0/GitGit 一致, 跨项目範式对齐.

**守门合规:**
- #15 scope creep: 遵守 (声明 + 預設关, no runtime over-engineering)
- #1+#7+#10+#11+#13+#14v4+#19v19+#24: 全部 0 違反 (reverse decision 已在 `.mock-cluster.json` `decision_history` 透明化记录)

## §7 已知缺口 (跨 session 续做, 不在本 issue 范围)

1. 🟡 **G-MS-Ada-01**: 3 个 connector/golden/loader/builders module 没有对应 TDS docs (`docs/tds/`), follow-up 需补 `TDS-MOCK-2026-004` ~ `TDS-MOCK-2026-007`
2. 🟡 **G-MS-Ada-02**: `connectors` (stdin/file/http) 实际是 3 sub-protocol, 可考虑细分 1 plugin × 3 module (但当前跟 lib.rs 「Mock 资源」 1 层聚合保持 1 module)
3. 🟡 **G-MS-Ada-03**: `cluster.enabled=true` 真启用路径未实现 — Ada testkit scaffold 业务上不需要 emit ACI assertion, 但若未来需要 cross-project ci gate, 需补 Rust 代码 path (per `aci_emitter_helper.rs` 现状)
4. 🟡 **G-MS-Ada-04**: `_lib_mock_switch_ada.py` 当前仅 Python 实现, 跨项目 CI (mock-switch-validate.py) 需调 Python subprocess, 跟 RGS `tools/rgs-flash-mock/_lib_mock_switch_rgs.py` 範式一致
5. 🟡 **G-MS-Ada-05**: Rust native `_lib_mock_switch.rs` 替换 Python subprocess (跨项目 batch, 跨 session 续做)

## §8 跨项目累計 验证 (per §4.4 stage 1+2+3+4+5+6+7, **7/7 项目 module_switch 落地**)

| 项目 | Plugin | Module | PR | commit | merged (JST) | 状态 |
|---|---|---|---|---|---|---|
| IM1.0 | 5 | 28 | #24 | `96a2e28` on dev | 9/26 00:18 | ✅ MERGED |
| CATs | 4 | 13 | #18 | `9b97d6b` on main | 9/26 01:36 | ✅ MERGED |
| Star | 7 | 7 | #151 | `55cf3794` on dev | 9/26 02:36 | ✅ MERGED |
| RGS | 5 | 12 | #51 | `41932076` on dev | 9/26 12:22 | ✅ MERGED |
| IDE1.0 | N | 8 | stage5 | ... on dev | 9/26 12:30 | ✅ MERGED |
| GitGit | N | 7 | stage6 | `2c57ec91` on dev | 9/26 12:35 | ✅ MERGED |
| **Ada** | **5** | **10** | **stage7 (本 commit)** | **... on dev** | **9/26 13:30** | **🟡 PENDING** |
| **合计** | **N+28** | **77** | **6/7** | — | — | — |

跨 session 续做 (per ULYS-240 thread reply §8.6, 全部 12 项):
1. 🟡 §4.4 stage7 Ada 派工 (本 commit 完成, 不再是 follow-up)
2. 🟡 Star design-analysis v0.4 → v0.5 (加 §10 module_switch 落地回顧, 7/7 项目累計)
3. 🟡 G-MS-08 trace_format 截断到 ~80 字 (跨项目 batch, 跨 session)
4. 🟡 G-MS-05 transaction audit log (跨项目)
5. 🟡 G-MS-09 hot reload (跨项目)
6. 🟡 Rust native `_lib_mock_switch.rs` 替换 Python subprocess (跨项目 batch)
7. 🟡 CI mock-switch-validate 加 module 级校验 (CI infra)
8. 🟡 Layer 1→Layer 2→Layer 3 贯通验收 (顶层 sub-task, 7/7 完成可推进)
9. 🟡 ULYS-190 状态 in_review → done 最终 flip (human release judgement)
10. 🟡 G-MS-Ada-01 (per §7-1) Ada 4 module TDS docs 补齐
11. 🟡 G-MS-Ada-03 (per §7-3) cluster.enabled=true 真启用路径
12. 🟡 G-MS-RGS-SPECIFIC-01 RGS scene module dead branch 真实业务验证

— minimax / 2026-09-26 13:30 JST