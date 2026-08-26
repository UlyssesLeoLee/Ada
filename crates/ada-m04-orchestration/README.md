# ada-m04-orchestration

M-04: パイプラインオーケストレーション (Pipeline orchestration).
DAG-based dependency resolution and execution control.

## v0.1.0 scope (B6 batch)

This crate is a **minimum skeleton** for the cross-module
orchestrator that the B5+ M-05 control-flow executor and the
M-13 API gateway program against. The v0.1.0 surface is the
in-process [`Scheduler`] contract that downstream crates can
mock, plus a working [`InMemoryScheduler`] for unit tests and
single-process dev builds.

### What v0.1.0 provides

- [`Job`] — `id`, `kind`, `payload`, `state`, `created_at_ms`,
  `started_at_ms`, `finished_at_ms`, `owner`
- [`JobState`] — six canonical states
  (`Pending / Queued / Running / Succeeded / Failed / Cancelled`)
- [`JobKind`] — five canonical kinds
  (`Acquisition / Normalization / FlowExecution / ControlFlow / Export`)
- [`Scheduler`] trait —
  `enqueue / poll / cancel / state_of`
- [`InMemoryScheduler`] — `parking_lot::Mutex<Vec<Job>>`-backed
  FIFO queue with a state machine that never actually
  *executes* the work (a caller pulls a job, marks it
  `Running`, then `Succeeded`/`Failed`/`Cancelled`).
- 5-variant [`OrchError`] (`JobNotFound`, `InvalidState`,
  `QueueFull`, `BackendError`, `Cancelled`)
- 12 unit tests + 4 integration tests

### What v0.1.0 explicitly does **not** do

- Actually execute the work. The skeleton tracks state
  transitions only; the real worker pool (or kubernetes job
  runner) lands in B7+ once G4 (実装着手判定) is approved.
- Persist jobs to the `orchestration_jobs` table
- Distribute jobs across cluster nodes (M-16 territory)
- Honor cron / event-driven triggers (those live in
  `ada-m08-trigger`)
- Honor RBAC checks on `enqueue` (the M-11 permission check
  will be wired in B7+)

## 関連 IPA フェーズ

22-52 (基本設計/詳細設計), 53-58 (実装), 59-95 (試験)

## 設計書

`docs/modules/M-04-orchestration.md` (DOC-MOD-004)
