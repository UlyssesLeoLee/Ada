# ada-remediation

Auto-remediation runbook engine for **observability Phase 8**
(`docs/observability/11-phased-rollout.md` §10).

## What it does

1. Loads runbooks from JSON files under `config/remediation/`.
2. Receives Alertmanager v4 webhook payloads at
   `POST /webhook/alertmanager`.
3. Matches every alert against the runbook table by `alertname`.
4. Executes matching actions in order, with step-level
   short-circuit + retry semantics.
5. Records every execution in an in-memory store + the
   `remediation_history` PostgreSQL table (durable copy written
   by the production wiring).
6. Exposes introspection endpoints for the Grafana dashboard.

## Runbook file format

Each `config/remediation/*.json` file is a `RunbookFile`:

```json
{
  "version": 1,
  "actions": [
    {
      "id": "disk-space-low",
      "name": "Disk space low",
      "trigger": "DiskSpaceFillingFast",
      "steps": [
        { "kind": "run_command", "cmd": "du", "args": ["-sh", "/var/log"], "timeout_secs": 30 },
        { "kind": "notify_slack", "channel": "#ada-ops", "message": "disk low on {{ $labels.instance }}" }
      ],
      "cooldown": 300,
      "max_retries": 2
    }
  ]
}
```

Triggers can be either an exact alert name (`"ServiceDown"`) or
a shell-style glob (`"SLIBurn*"`, `"DB*Pool*"`).

## HTTP endpoints

| Method | Path | Purpose |
|---|---|---|
| `GET` | `/health` | Liveness probe |
| `POST` | `/webhook/alertmanager` | Receive Alertmanager v4 payload |
| `GET` | `/remediation/history` | Query execution history |
| `GET` | `/remediation/cooldowns` | List active cooldowns |
| `POST` | `/remediation/trigger` | Operator-trigger / dashboard "run now" |

## State machine

```
Idle → Evaluating → Executing → Cooldown → Idle
                  ↘ Idle    (no match)
                  ↘ Failed  (max_retries exhausted)
                  ↘ Retrying → Executing (next attempt)
```

## Five gate baseline

```
cargo check --workspace --all-targets
cargo test  --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt   --all -- --check
cargo clippy --workspace
```

See `docs/observability/12-auto-remediation.md` for the
architecture, runbook authoring guide, and cooldown policy.
