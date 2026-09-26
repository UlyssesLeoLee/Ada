#!/usr/bin/env python3
"""Ada ada-mock mock_switch reader tests (per ULYS-190 §4.4 stage7 reverse-decision).

Mirrors RGS `tests/test_rgs_mock_switch.py` (PR #51) 範式 + Star
`tests/test_mock_switch_plugins.py` (PR #151) 範式 + IM1.0
`tests/im_testkit_module_switch.rs` (PR #24) 範式 + CATs
`tests/cats_mock_module_switch.rs` (PR #18) 範式.

Assertions:
- 5 plugins total (mocks / server / fixtures / builders / tds) per §4.4 stage7
- 10 modules total (3+1+2+1+3) per lib.rs 4 capability layers + TDS docs
- per plugin × module count matches
- cluster enabled + mode assertions
- aci_compat_version consistency
- mock_switch_trace_format 真拼接
- module_count_total/enabled field verify
- run_helper 调 _lib_mock_switch_ada.py read-plugins (跨 Python invocations)
"""

import json
import os
import subprocess
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]  # tests/ → ada-mock/ → crates/ → <worktree_root>
ACI = ROOT / "crates" / "ada-mock" / ".aci.json"
CLUSTER = ROOT / "crates" / "ada-mock" / ".mock-cluster.json"
HELPER = ROOT / "crates" / "ada-mock" / "scripts" / "_lib_mock_switch_ada.py"


class TestAdaModuleSwitch(unittest.TestCase):
    def setUp(self):
        self.aci = json.loads(ACI.read_text(encoding="utf-8"))
        self.cluster = json.loads(CLUSTER.read_text(encoding="utf-8"))

    # ---- helper 基础 ----

    def test_aci_file_exists(self):
        self.assertTrue(ACI.exists(), f".aci.json missing at {ACI}")

    def test_cluster_file_exists(self):
        self.assertTrue(CLUSTER.exists(), f".mock-cluster.json missing at {CLUSTER}")

    def test_helper_exists(self):
        self.assertTrue(HELPER.exists(), f"helper script missing at {HELPER}")

    # ---- 总数 ----

    def test_plugin_count_is_5(self):
        plugins = self.aci["plugins"]
        self.assertEqual(
            len(plugins),
            5,
            f"expected 5 plugins (mocks/server/fixtures/builders/tds), got {len(plugins)}",
        )

    def test_module_switch_total_count_is_10(self):
        plugins = self.aci["plugins"]
        total = sum(len(p.get("modules", {})) for p in plugins.values())
        self.assertEqual(
            total,
            10,
            f"expected 10 modules across 5 plugins, got {total} (3+1+2+1+3)",
        )

    def test_module_switch_all_enabled_by_default(self):
        plugins = self.aci["plugins"]
        for pid, pconf in plugins.items():
            for mid, mconf in pconf.get("modules", {}).items():
                self.assertTrue(
                    mconf.get("enabled", False),
                    f"plugin={pid} module={mid} should be enabled",
                )

    def test_module_count_total_field_is_10(self):
        self.assertEqual(self.aci["module_count_total"], 10)

    def test_module_count_enabled_field_is_10(self):
        self.assertEqual(self.aci["module_count_enabled"], 10)

    # ---- per plugin × module count 验证 ----

    def test_mocks_plugin_has_3_modules(self):
        mods = self.aci["plugins"]["mocks"]["modules"]
        self.assertEqual(set(mods.keys()), {"event_bus", "scheduler", "connector"})

    def test_server_plugin_has_1_module(self):
        mods = self.aci["plugins"]["server"]["modules"]
        self.assertEqual(set(mods.keys()), {"fake_otlp_server"})

    def test_fixtures_plugin_has_2_modules(self):
        mods = self.aci["plugins"]["fixtures"]["modules"]
        self.assertEqual(set(mods.keys()), {"golden", "loader"})

    def test_builders_plugin_has_1_module(self):
        mods = self.aci["plugins"]["builders"]["modules"]
        self.assertEqual(set(mods.keys()), {"builders"})

    def test_tds_plugin_has_3_modules(self):
        mods = self.aci["plugins"]["tds"]["modules"]
        self.assertEqual(set(mods.keys()), {"tds_event_bus", "tds_scheduler", "tds_fake_otlp"})

    # ---- cluster assertions ----

    def test_cluster_enabled_false_and_mode_offline(self):
        # Ada 是 testkit scaffold, 預設 enabled=false (per G-MS-01 解決 設計)
        self.assertEqual(self.cluster["enabled"], False)
        self.assertEqual(self.cluster["mode"], "offline")

    def test_cluster_module_count_total_matches_aci(self):
        self.assertEqual(
            self.cluster.get("module_count_total"),
            self.aci["module_count_total"],
        )

    # ---- compat validation ----

    def test_aci_compat_version_consistent_across_cluster_and_aci(self):
        cluster_v = self.cluster.get("aci_compat_version")
        aci_v = self.aci.get("aci_compat_version")
        self.assertIsNotNone(cluster_v, "cluster.aci_compat_version missing")
        self.assertIsNotNone(aci_v, "aci.aci_compat_version missing")
        self.assertEqual(cluster_v, aci_v)

    # ---- trace format ----

    def test_trace_format_substitutes_placeholders(self):
        tmpl = self.cluster["mock_switch_trace_format"]
        self.assertIn("{cluster.enabled}", tmpl)
        self.assertIn("{cluster.mode}", tmpl)
        # 5 plugin x 10 module 字符串
        self.assertIn("mocks(3m)", tmpl)
        self.assertIn("server(1m)", tmpl)
        self.assertIn("fixtures(2m)", tmpl)
        self.assertIn("builders(1m)", tmpl)
        self.assertIn("tds(3m)", tmpl)
        self.assertIn("10/10 modules", tmpl)


class TestAdaModuleSwitchHelperInvocation(unittest.TestCase):
    """跨 Python invocation 测试 — 调 _lib_mock_switch_ada.py 实测."""

    def _run_helper(self, *args):
        cmd = [sys.executable, str(HELPER), "--aci-config", str(ACI), *args]
        return subprocess.run(cmd, capture_output=True, text=True, timeout=10)

    def test_helper_is_enabled(self):
        result = self._run_helper("is-enabled")
        # cluster.enabled=false → exit 1
        self.assertEqual(result.returncode, 1)
        self.assertIn("CLUSTER_ENABLED=false", result.stdout)

    def test_helper_get_mode(self):
        result = self._run_helper("get-mode")
        self.assertEqual(result.returncode, 0)
        self.assertIn("CLUSTER_MODE=offline", result.stdout)

    def test_helper_trace(self):
        result = self._run_helper("trace")
        self.assertEqual(result.returncode, 0)
        # 真拼接 — cluster.enabled + mode + plugins 列表
        self.assertIn("cluster.enabled=false", result.stdout)
        self.assertIn("mode=offline", result.stdout)
        self.assertIn("plugins=[mocks(3m),server(1m),fixtures(2m),builders(1m),tds(3m)]=10/10 modules", result.stdout)

    def test_helper_validate_compat(self):
        result = self._run_helper("validate-compat")
        self.assertEqual(result.returncode, 0)
        self.assertIn("ACI_COMPAT=OK", result.stdout)
        self.assertIn("0.1.0-draft", result.stdout)

    def test_helper_read_plugins(self):
        result = self._run_helper("read-plugins")
        # read-plugins 永远 exit 0 on success (跨项目範式)
        self.assertEqual(result.returncode, 0)
        data = json.loads(result.stdout)
        self.assertEqual(data["plugins_total"], 5)
        self.assertEqual(data["plugins_enabled"], 5)
        self.assertEqual(data["modules_total"], 10)
        self.assertEqual(data["modules_enabled"], 10)
        for pid in ("mocks", "server", "fixtures", "builders", "tds"):
            self.assertIn(pid, data["plugins"])

    def test_helper_trace_alternating_invocations(self):
        """跨 invocation 反复 trace, 确认输出稳定 (no state leak)."""
        outputs = []
        for _ in range(3):
            r = self._run_helper("trace")
            self.assertEqual(r.returncode, 0)
            outputs.append(r.stdout)
        self.assertEqual(outputs[0], outputs[1])
        self.assertEqual(outputs[1], outputs[2])


if __name__ == "__main__":
    unittest.main()