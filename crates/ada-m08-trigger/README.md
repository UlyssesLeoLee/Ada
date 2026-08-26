# ada-m08-trigger

M-08: Trigger manager. 4 `TriggerKind` (`Cron / Webhook /
Event / Manual`), `TriggerRule`, `TriggerManager` with
in-process storage and event-topic glob matching.

See `docs/modules/M-08-trigger.md` (DOC-MOD-008) for the full
design.

## v0.1.0 status

Skeleton. The cron parser is a 5-field whitespace split
(minute / hour / dom / month / dow). B7+ will swap in the
`cron` crate for the full spec.

## v0.1.0 surface

- `TriggerKind` — `Cron | Webhook | Event | Manual`
- `Action` — kind + JSON payload
- `TriggerRule` — id, name, kind, schedule, action, enabled
- `TriggerManager` — `add / remove / list / get / set_enabled / match_event`
- Event-topic matching: literal, `prefix.*` (one segment), `prefix.#` (zero+)
- 5-variant `TriggerError`
- ~20 unit tests + 4 integration tests
