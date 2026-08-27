# Ada SLO / SLI Framework (Phase 7)

Source of truth: [`docs/observability/08-slo-design.md`](../../docs/observability/08-slo-design.md)
and [`docs/observability/11-phased-rollout.md` §9 Phase 7](../../docs/observability/11-phased-rollout.md).

## Files in this directory

| File | Purpose |
|---|---|
| `availability.yml`   | Availability SLI formulas + SLO targets (3 services) |
| `latency.yml`        | Latency SLI (p95 / p99) + SLO targets (3 services) |
| `error_rate.yml`     | Error Rate SLI (5xx ratio) + SLO targets |
| `throughput.yml`     | Throughput SLI (RPS) — capacity-planning metric, no SLO target |
| `slo_recording_rules.yml` | Prometheus recording rules pre-computing the SLI ratios |
| `slo_burn_rate_fast.yml`  | Fast-burn (1h / 6h) page alerts |
| `slo_burn_rate_slow.yml`  | Slow-burn (24h / 72h) ticket alerts |

The recording rules and alerts are wired into the
`observability/prometheus/prometheus.yml` rule_files glob
(`rules/*.yml`); this directory adds the SLO-specific
files but does not change the loader pattern.

## SLO matrix (Phase 7 default)

| Service                  | Availability | p99 Latency | Error Rate |
|--------------------------|--------------|-------------|------------|
| `m13-api-gateway`        | 99.9%        | < 200ms     | < 0.5%     |
| `m03-data-flow-engine`   | 99.5%        | < 500ms     | < 1.0%     |
| `m10-tenant-middleware`  | 99.95%       | < 50ms      | < 0.1%     |

## Error Budget (30-day window, per Google SRE Workbook)

| SLO      | Budget / 30d | Per hour  | Page trigger (Fast Burn 1h × 14.4) |
|----------|--------------|-----------|------------------------------------|
| 99.9%    | 43m 49s      | 1m 27s    | 2% in 1h = 21m of error |
| 99.5%    | 3h 39m 22s   | 7m 18s    | 7.2% in 1h = 1h 44m of error |
| 99.95%   | 4m 22s       | 8.4s      | 0.72% in 1h = 2m 11s of error |

## Multi-Window / Multi-Burn-Rate (MWMB) alerts

The four standard alerts per
[`08-slo-design.md` §3.4](../../docs/observability/08-slo-design.md):

1. **Fast Burn 1h** (page) — `slo:sli_error:rate_1h / slo:sli_total:rate_1h > 14.4 * (1 - SLO_target)`, severity P1
2. **Fast Burn 6h** (page) — same ratio over 6h window × 14.4, severity P2
3. **Slow Burn 24h** (ticket) — same ratio × 6 over 24h, severity P3
4. **Slow Burn 72h** (ticket) — same ratio × 6 over 72h, severity P3 (digest)

The 14.4x / 6x / 3x / 1x multipliers are the standard
Google SRE multi-window multipliers. They keep the alert
sensitive to short-burn rate spikes (a 2%-of-budget-in-1h
event is a P1) while not paging on noise from long-tail
sustained degradation (a 5%-of-budget-over-72h event is
still actionable but at P3).

## Dashboards

Provisioned via `observability/grafana/provisioning/dashboards/`:

- `slo-overview.json` — Error Budget 90-day dashboard (one row per SLO with budget remaining gauge + 28d availability trend)
- `slo-burn-rate.json` — MWMB burn rate dashboard (fast / slow burn over multi-window)
- `slo-availability.json` — Availability-only focused dashboard (used by the on-call when investigating a P1 availability alert)

The dashboards read from the Prometheus datasource via
the recording rules in `slo_recording_rules.yml`.
