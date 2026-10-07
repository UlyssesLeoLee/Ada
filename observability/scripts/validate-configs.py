"""YAML/JSON syntax check for every Ada observability config.

Run from anywhere:

    python observability/scripts/validate-configs.py

Exits 0 when every discovered file parses, 1 otherwise, 2 when the
discovery itself is broken (see ANTI-VACUITY below). UTF-8 is forced
because the default platform encoding (CP936 / GBK on Chinese Windows)
rejects the multi-byte sequences the YAML files contain.

Why this walks the tree instead of carrying a list
---------------------------------------------------
It used to hold two hardcoded lists -- YAML_FILES and JSON_FILES -- and
that is the shape that rots silently:

* Five entries named ``prometheus/alerts/<name>.yml``. Those rules were
  moved to ``prometheus/alerts-disabled/`` (see the README there, which
  explains why each one is inert), and the list was never updated. The
  script printed five ``FAIL`` lines and exited 1. Nobody noticed,
  because **no CI workflow ran it** -- `observability/README.md` listed
  it as an available check and the check had been failing ever since.
* Eleven Grafana dashboards are tracked. The list had ten, so
  ``phase8-remediation-overview.json`` was never parsed by anything.

Both lists also happened to have the right *length* (23 YAML entries
for 23 YAML files), which is how a stale list passes a human skim: the
number matched while five of the paths did not.

A directory walk has no list to rot. Adding a config now validates it
without editing this file, and the count printed below is the count on
disk, so "23/23" cannot quietly become "28/33".

ANTI-VACUITY
------------
A walk that silently finds nothing exits 0 and looks exactly like a
clean run. So the script asserts that discovery found what this tree is
supposed to contain -- sentinels, plus a floor on each class -- and
exits **2** with a distinct message if not. Exit 2 is reserved for
"the checker is broken"; exit 1 means "a config is broken".
"""

from __future__ import annotations

import json
import pathlib
import sys

import yaml

REPO = pathlib.Path(__file__).resolve().parent.parent.parent
OBS = REPO / "observability"

# Paths that must appear in the discovered set. Each one is a file whose
# absence means the walk is pointed somewhere wrong, not that the
# repository lost a config -- that distinction is the whole point.
SENTINELS = (
    "observability/prometheus/prometheus.yml",
    "observability/prometheus/rules/slo_recording_rules.yml",
    "observability/alertmanager/alertmanager.yml",
    "observability/grafana/provisioning/datasources/datasources.yml",
    "observability/loki/loki-config.yaml",
    "observability/jaeger/otel-collector-config.yaml",
    "observability/tempo/tempo-config.yaml",
    "observability/docker-compose.yml",
)

# Directories that must each contribute at least one file.
SENTINEL_DIRS = (
    "observability/prometheus/alerts",
    "observability/prometheus/alerts-disabled",
    "observability/grafana/dashboards",
    "observability/slo",
)

# Floors, well below the real counts (23 YAML, 11 JSON) so an ordinary
# deletion is still reported as a deletion, not as a broken walk.
MIN_YAML = 15
MIN_JSON = 8


def _rel(path: pathlib.Path) -> str:
    return path.relative_to(REPO).as_posix()


def discover(root: pathlib.Path, suffixes: tuple[str, ...]) -> list[pathlib.Path]:
    """Every file under ``root`` whose name ends with one of ``suffixes``."""
    out: list[pathlib.Path] = []
    for path in root.rglob("*"):
        if not path.is_file():
            continue
        if path.suffix.lower() in suffixes:
            out.append(path)
    return sorted(out)


def check_anti_vacuity(yaml_files: list[pathlib.Path], json_files: list[pathlib.Path]) -> list[str]:
    """Return a list of discovery failures. Empty means discovery is sound."""
    problems: list[str] = []
    found = {_rel(p) for p in yaml_files} | {_rel(p) for p in json_files}

    for sentinel in SENTINELS:
        if sentinel not in found:
            problems.append(f"expected to find {sentinel}")

    for rel_dir in SENTINEL_DIRS:
        if not any(f.startswith(rel_dir + "/") for f in found):
            problems.append(f"expected at least one file under {rel_dir}")

    if len(yaml_files) < MIN_YAML:
        problems.append(f"found only {len(yaml_files)} YAML files, floor is {MIN_YAML}")
    if len(json_files) < MIN_JSON:
        problems.append(f"found only {len(json_files)} JSON files, floor is {MIN_JSON}")

    return problems


def main() -> int:
    if not OBS.is_dir():
        print(f"FAIL {OBS} does not exist -- pointed at the wrong tree")
        return 2

    yaml_files = discover(OBS, (".yml", ".yaml"))
    json_files = discover(OBS, (".json",))

    print(f"discovered {len(yaml_files)} YAML and {len(json_files)} JSON under observability/")

    vacuity = check_anti_vacuity(yaml_files, json_files)
    if vacuity:
        print("\nDISCOVERY BROKEN -- the walk did not see this repository's configs:")
        for p in vacuity:
            print(f"  {p}")
        print(
            "\nA run that validates nothing exits 0 and looks identical to a clean\n"
            "run. Fix the walk before trusting any result above."
        )
        return 2

    # Split the alerts the way prometheus.yml does: `rule_files` globs
    # alerts/*.yml and rules/*.yml, so anything under alerts-disabled/ is
    # tracked but never loaded. Printed because a rule that moved across
    # that line without anyone noticing is the failure this script exists
    # to make visible.
    live = [p for p in yaml_files if p.parent.name == "alerts"]
    disabled = [p for p in yaml_files if p.parent.name == "alerts-disabled"]
    print(f"alert rules: {len(live)} loaded from alerts/, {len(disabled)} inert in alerts-disabled/")

    ok = 0
    fail = 0

    for path in yaml_files:
        rel = _rel(path)
        try:
            with path.open(encoding="utf-8") as fp:
                yaml.safe_load(fp)
            print(f"  OK   {rel}")
            ok += 1
        except Exception as exc:  # noqa: BLE001 -- report, never crash the sweep
            print(f"  FAIL {rel}: {exc}")
            fail += 1

    for path in json_files:
        rel = _rel(path)
        try:
            with path.open(encoding="utf-8") as fp:
                json.load(fp)
            print(f"  OK   {rel}")
            ok += 1
        except Exception as exc:  # noqa: BLE001
            print(f"  FAIL {rel}: {exc}")
            fail += 1

    print(f"\n{ok}/{ok + fail} config files parse cleanly")
    return 0 if fail == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
