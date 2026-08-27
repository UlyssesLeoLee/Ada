"""Quick YAML/JSON syntax check for the Ada observability
configs. Run from the repo root:

    python observability/scripts/validate-configs.py

Exits 0 when every file parses, 1 otherwise. UTF-8 is forced
because the default platform encoding (CP936 / GBK on
Chinese Windows) rejects the multi-byte sequences the YAML
files contain.
"""
import json
import sys
import pathlib

import yaml

REPO = pathlib.Path(__file__).resolve().parent.parent.parent

YAML_FILES = [
    "observability/prometheus/prometheus.yml",
    "observability/prometheus/alerts/app_down.yml",
    "observability/prometheus/alerts/high_error_rate.yml",
    "observability/prometheus/alerts/high_latency.yml",
    "observability/prometheus/alerts/low_disk.yml",
    "observability/prometheus/alerts/scaling_alert.yml",
    "observability/prometheus/alerts/trace_high_error_rate.yml",
    "observability/prometheus/alerts/slo_burn_rate_fast.yml",
    "observability/prometheus/alerts/slo_burn_rate_slow.yml",
    "observability/prometheus/rules/slo_recording_rules.yml",
    "observability/slo/availability.yml",
    "observability/slo/latency.yml",
    "observability/slo/error_rate.yml",
    "observability/slo/throughput.yml",
    "observability/alertmanager/alertmanager.yml",
    "observability/loki/loki-config.yaml",
    "observability/loki/promtail-config.yaml",
    "observability/grafana/provisioning/datasources/datasources.yml",
    "observability/grafana/provisioning/dashboards/dashboards.yml",
    "observability/jaeger/jaeger-config.yaml",
    "observability/jaeger/otel-collector-config.yaml",
    "observability/tempo/tempo-config.yaml",
    "observability/docker-compose.yml",
]

JSON_FILES = [
    "observability/grafana/dashboards/app-overview.json",
    "observability/grafana/dashboards/rust-runtime.json",
    "observability/grafana/dashboards/db-overview.json",
    "observability/grafana/dashboards/trace-overview.json",
    "observability/grafana/dashboards/slo-overview.json",
    "observability/grafana/dashboards/slo-burn-rate.json",
    "observability/grafana/dashboards/slo-availability.json",
]


def main() -> int:
    ok = 0
    fail = 0
    for rel in YAML_FILES:
        path = REPO / rel
        try:
            with path.open(encoding="utf-8") as fp:
                yaml.safe_load(fp)
            print(f"  OK   {rel}")
            ok += 1
        except Exception as exc:
            print(f"  FAIL {rel}: {exc}")
            fail += 1
    for rel in JSON_FILES:
        path = REPO / rel
        try:
            with path.open(encoding="utf-8") as fp:
                json.load(fp)
            print(f"  OK   {rel}")
            ok += 1
        except Exception as exc:
            print(f"  FAIL {rel}: {exc}")
            fail += 1
    print(f"\n{ok}/{ok + fail} config files parse cleanly")
    return 0 if fail == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
