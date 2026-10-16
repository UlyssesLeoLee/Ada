#!/usr/bin/env python3
"""aggregate_results.py - read each layer's -latest.log and emit summary.json + summary.md.

Inputs (relative to workspace root):
    test-results/ut/ut-latest.log
    test-results/it/it-latest.log
    test-results/st/st-latest.log

Outputs (to $REGRESSION_OUT_DIR or --out-dir, default test-results/regression-latest/):
    summary.json
    summary.md

Usage:
    python scripts/aggregate_results.py
    python scripts/aggregate_results.py --out-dir <path>
"""
from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import re
import sys
from pathlib import Path

RESULT_RE = re.compile(
    r"test result:\s*(?P<status>ok|FAILED).*?(?P<passed>\d+)\s+passed;\s*(?P<failed>\d+)\s+failed",
    re.IGNORECASE,
)
FAILED_NAMES_RE = re.compile(r"^test\s+(?P<name>\S+)\s+\.\.\.\s+FAILED\s*$", re.MULTILINE)

LAYER_LOGS = [
    ("UT", "test-results/ut/ut-latest.log"),
    ("IT", "test-results/it/it-latest.log"),
    ("ST", "test-results/st/st-latest.log"),
]


def parse_layer(layer: str, log_path: Path) -> dict:
    if not log_path.exists():
        return {"layer": layer, "status": "missing", "log": str(log_path),
                "passed": 0, "failed": 0, "failures": []}
    text = log_path.read_text(encoding="utf-8", errors="replace")
    m = RESULT_RE.search(text)
    if not m:
        return {"layer": layer, "status": "unknown", "log": str(log_path),
                "passed": 0, "failed": 0, "failures": []}
    passed = int(m.group("passed"))
    failed = int(m.group("failed"))
    status = "pass" if (failed == 0 and m.group("status").lower() == "ok") else "fail"
    failures = []
    if failed > 0:
        for fm in FAILED_NAMES_RE.finditer(text):
            failures.append(fm.group("name"))
    return {
        "layer": layer,
        "status": status,
        "log": str(log_path),
        "passed": passed,
        "failed": failed,
        "failures": failures,
    }


def render_summary(layers):
    total_passed = sum(l["passed"] for l in layers)
    total_failed = sum(l["failed"] for l in layers)
    total = total_passed + total_failed
    overall = "pass" if (total > 0 and all(l["status"] == "pass" for l in layers)) else "fail"
    return {
        "generated_at": dt.datetime.now().isoformat(timespec="seconds"),
        "overall": overall,
        "total_tests": total,
        "total_passed": total_passed,
        "total_failed": total_failed,
        "layers": layers,
    }


def render_markdown(summary):
    lines = []
    lines.append("# ada-mock Regression Summary")
    lines.append("")
    lines.append(f"- **Generated**: {summary['generated_at']}")
    lines.append(f"- **Overall**: **{summary['overall'].upper()}**")
    lines.append(f"- **Totals**: {summary['total_tests']} "
                 f"(passed={summary['total_passed']}, failed={summary['total_failed']})")
    lines.append("")
    lines.append("| Layer | Status | passed | failed | log |")
    lines.append("|---|---|---|---|---|")
    for l in summary["layers"]:
        log_disp = Path(l["log"]).name if l["log"] else "-"
        lines.append(f"| {l['layer']} | {l['status'].upper()} | "
                     f"{l['passed']} | {l['failed']} | `{log_disp}` |")
    lines.append("")
    fails = [l for l in summary["layers"] if l["status"] != "pass" or l["failed"] > 0]
    if fails:
        lines.append("## Failure details")
        lines.append("")
        for l in fails:
            if l["failures"]:
                for n in l["failures"]:
                    lines.append(f"- **{l['layer']}** :: `{n}`")
            elif l["status"] == "missing":
                lines.append(f"- **{l['layer']}** :: log missing ({l['log']})")
            else:
                lines.append(f"- **{l['layer']}** :: status={l['status']} (see log)")
    return "\n".join(lines) + "\n"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--root", default=".", help="workspace root (default: cwd)")
    ap.add_argument("--out-dir", default=None,
                    help="output dir (default: env REGRESSION_OUT_DIR or test-results/regression-latest)")
    args = ap.parse_args()

    root = Path(args.root).resolve()
    out_dir_env = os.environ.get("REGRESSION_OUT_DIR")
    out_dir = Path(args.out_dir or out_dir_env or (root / "test-results" / "regression-latest")).resolve()
    out_dir.mkdir(parents=True, exist_ok=True)

    layers = [parse_layer(layer, root / rel) for layer, rel in LAYER_LOGS]
    summary = render_summary(layers)

    (out_dir / "summary.json").write_text(
        json.dumps(summary, ensure_ascii=False, indent=2), encoding="utf-8")
    (out_dir / "summary.md").write_text(render_markdown(summary), encoding="utf-8")

    print(f"[OK] {out_dir / 'summary.json'}")
    print(f"[OK] {out_dir / 'summary.md'}")
    print(f"overall={summary['overall']} "
          f"passed={summary['total_passed']} failed={summary['total_failed']}")
    return 0 if summary["overall"] == "pass" else 1


if __name__ == "__main__":
    sys.exit(main())
