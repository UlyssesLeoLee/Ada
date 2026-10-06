# Auto-remediation runbook configs

This directory contains the declarative runbook files loaded
by `crates/ada-remediation` at startup
(`RemediationEngine::with_defaults()` walks
`config/remediation/*.json`).

## File format

Each file is a `RunbookFile` (see
`crates/ada-remediation/src/config.rs`):

```json
{
  "version": 1,
  "actions": [
    {
      "id": "disk-space-low",
      "name": "Disk space low on {{ $labels.instance }}",
      "trigger": "DiskSpaceFillingFast",
      "severities": ["P2", "P3"],
      "steps": [
        { "kind": "run_command", "cmd": "du", "args": ["-sh", "/var/log"], "timeout_secs": 30 }
      ],
      "cooldown": 1800,
      "max_retries": 1
    }
  ]
}
```

## JSON vs. YAML

The Phase 8 design spec refers to "YAML" runbook files. The
implementation accepts **JSON** (`.json`) because the offline
build environment forbids pulling `serde_yaml` (it is not in
the existing `Cargo.lock`). JSON is a strict subset of YAML,
so the same shape parses under any YAML reader downstream.

If/when `serde_yaml` is added to the workspace, the loader
can be swapped with one line; the rest of the engine does
not need to change.

## Only one of these five runbooks can fire today

The `trigger` field is matched against the `alertname` of a Prometheus rule.
Cross-referencing every runbook against the rules that `prometheus.yml`
actually loads (`alerts/*.yml` and `rules/*.yml`) gives:

| Runbook | `trigger` | Alert that exists? |
|---|---|---|
| `service-down.json` | `ServiceDown` | **yes** — `alerts/app_down.yml` |
| `disk-space-low.json` | `DiskSpaceFillingFast` | **no** — the disk alert is named `LowDiskSpace` |
| `db-connection-pool-exhausted.json` | `DBConnectionPoolExhausted` | **no** — declared by no rule, in or out of quarantine |
| `slo-budget-burn-rate-fast.json` | `SLIBurnRateFast` | **no** — the rules declare `SLOBurnRateFast1h…` (SLO, and with a window suffix) |
| `slo-budget-burn-rate-slow.json` | `SLIBurnRateSlow` | **no** — the rules declare `SLOBurnRateSlow24h…`/`SLOBurnRateSlow72h…` |

Only three alerts can fire at all right now: `ServiceDown`, `LowDiskSpace`
and `CPUHigh`. So four of the five runbooks sit dormant, and the only live one
runs a `page_operator` step — no command executes.

**Do not "fix" the trigger names without reading this first.**
`disk-space-low.json` is the trap. Its second step is

```
find /var/log -type f -name '*.gz' -mtime +7 -delete
```

Pointing its trigger at `LowDiskSpace` would arm real log deletion on a real
alert, while the cooldown is still per-process: `deploy/k8s/ada-remediation.yaml`
runs two replicas and `main.rs` wires only `MemoryStore`, so two alerts in one
window can delete twice through different pods, and a restart forgets every
cooldown. That gap is written up under Known gaps in `deploy/k8s/README.md`.
Quarantining these runbooks and documenting them is the deliberate choice; the
alternative — arming them — is not.

Separately, both `pg_function` steps name functions that do not exist:
`remediation_restart_service` (service-down) and `remediation_kill_idle`
(db-connection-pool-exhausted). `db/migrations` declares eight functions and
neither is among them. With the shipped `LoggingClient` the call is recorded
and reported as success, so a run would claim it restarted a service or killed
sessions when it did neither. The `_comment` in each file already says
"v0.6.x future"; that is honest, and the runbook table below was not.

## Files

| File | Trigger | Severity | Steps | Cooldown | Fires today |
|---|---|---|---|---|---|
| `disk-space-low.json` | `DiskSpaceFillingFast` | P2/P3 | du → find -delete → notify | 30m | no — no such alert |
| `service-down.json` | `ServiceDown` | P1 | pg restart → page high | 10m | yes |
| `db-connection-pool-exhausted.json` | `DBConnectionPoolExhausted` | P2/P3 | pg kill_idle → notify | 15m | no — no such alert |
| `slo-budget-burn-rate-fast.json` | `SLIBurnRateFast` | P1 | page high | 1h | no — name mismatch |
| `slo-budget-burn-rate-slow.json` | `SLIBurnRateSlow` | P2/P3 | notify slack | 2h | no — name mismatch |

## Adding a new runbook

1. Create `config/remediation/<trigger>.json`.
2. Reuse the same `id` if you are editing; create a new
   `id` for a new action.
3. Pick a conservative `cooldown` (>= 5 min) — cooldowns
   are the principal defense against retry storms.
4. Pick `max_retries` = 0 for *page-and-forget* actions, 1
   for *try once then page*, 2+ for *really persistent* ones.
5. Reload the engine (no daemon / file watcher in v0.6.0;
   restart the binary).

## Template variables

`{{ $labels.X }}` placeholders in `message` and `name` are
substituted against the Alertmanager labels at execution
time. Unknown placeholders are left intact so the destination
message makes the missing label obvious.
