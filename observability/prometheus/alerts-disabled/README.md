# Disabled alert rules

These rules are **loaded by no one**. `prometheus.yml` globs
`alerts/*.yml` and `rules/*.yml`; this directory is `alerts-disabled/`,
which neither glob matches. They are kept, not deleted, so the
expressions can be reviewed and switched on once the telemetry they
depend on exists.

They were moved here rather than left in place because a rule that can
never fire is worse than an absent one: it loads cleanly, it appears in
Prometheus's rule list, and its absence of alerts reads as health.

## Why each file is inert

| File | Rules | Reason |
|---|---|---|
| `slo_burn_rate_fast.yml` | 3 | depends on `slo:sli_error:rate_5m` / `slo:sli_total:rate_1h`, which are recording rules derived from `ada_app_requests_total`. No Rust source emits that metric — zero occurrences in `crates/`. |
| `slo_burn_rate_slow.yml` | 6 | same chain as above. |
| `high_error_rate.yml` | 1 | queries `ada_app_requests_total` directly. |
| `high_latency.yml` | 1 | queries `ada_app_request_duration_seconds_bucket`. Zero occurrences in `crates/`. |
| `trace_high_error_rate.yml` | 1 | queries `traces_spanmetrics_latency_count`. The collector's traces pipeline is `[memory_limiter, resource/env, attributes/normalize, tail_sampling, batch]` — there is no `spanmetrics` processor, so the metric is never produced. |

A second, independent fault affects the burn-rate rules even if the
metric appeared: they select
`{service="m13-api-gateway"}`, `{service="m10-tenant-middleware"}` and
`{service="m03-data-flow-engine"}`. The only application scrape job is
`ada-app` in `prometheus.yml`, which stamps a single static
`service: ada-app` label across all 16 targets with no
`relabel_configs`. No series can ever carry those three values.

Those `m0x` names are module identifiers from before the services were
renamed; the deployed names are `ada-api-gateway` and `gm-console`.

## What has to exist before these can be enabled

1. **HTTP metrics in the services.** Only `ada-remediation` exposes a
   `/metrics` endpoint (`crates/ada-remediation/src/http.rs`). The
   gateway and the console emit none. Either add a metrics middleware
   emitting `ada_app_requests_total{status=...}` and
   `ada_app_request_duration_seconds_bucket`, or point these rules at
   whatever the services really expose.
2. **A per-target `service` label.** The scrape config must relabel
   each target with its own service name, or these selectors must be
   rewritten to match whatever label the scrape actually produces.
3. **A `spanmetrics` processor** in the OTel collector's traces
   pipeline, for `trace_high_error_rate.yml` only.
4. **Real runbook URLs.** Every rule in this directory — and every rule
   still in `alerts/` — carried `runbook: "https://wiki.example/..."`.
   `example.com` and its subdomains are reserved by RFC 2606 for
   examples; that host never resolves, so the one link an on-call
   engineer is given at 3am fails to load.

Note that `rules/slo_recording_rules.yml` was **left enabled**. Those
recording rules are correct and evaluate to empty; they start producing
series the moment `ada_app_requests_total` exists, and they generate no
alerts on their own.

## Still live in `alerts/`

`app_down.yml`, `low_disk.yml` and `scaling_alert.yml` — 3 rules, all on
`up` and `node_exporter_*` series that the shipped compose stack and
scrape config genuinely produce.
