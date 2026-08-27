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

## Files

| File | Trigger | Severity | Steps | Cooldown |
|---|---|---|---|---|
| `disk-space-low.json` | `DiskSpaceFillingFast` | P2/P3 | du → find -delete → notify | 30m |
| `service-down.json` | `ServiceDown` | P1 | pg restart → page high | 10m |
| `db-connection-pool-exhausted.json` | `DBConnectionPoolExhausted` | P2/P3 | pg kill_idle → notify | 15m |
| `slo-budget-burn-rate-fast.json` | `SLIBurnRateFast` | P1 | page high | 1h |
| `slo-budget-burn-rate-slow.json` | `SLIBurnRateSlow` | P2/P3 | notify slack | 2h |

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
