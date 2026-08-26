# ada-m05-control-flow

M-05: 制御フローエグゼキュータ (Control flow executor).
Conditional branches, loops, parallel branches.

## v0.1.0 scope (B5 batch)

This crate is the **minimum skeleton** for the
canvas-defined control flow executor. The v0.1.0 surface
is the in-process step walker that the B5+ parallel
executor and the action-side-effect implementation will
replace.

The production deployment (parallel branch execution,
action side-effects, persisted trace, distributed-trace
propagation, see `DOC-MOD-005` §3.5-§3.7) is scheduled
for B5+ once G4 (実装着手判定) is approved.

### What v0.1.0 provides

- `ControlStep` — `id`, `kind` (`Action / Branch / Loop /
  Switch / Parallel`), optional `condition`, `next_step`
- `StepKind` — the five canonical step kinds
- `BranchStep` / `LoopStep` / `SwitchStep` /
  `ParallelStep` — per-kind payloads
- `Condition` — boolean expression over
  `HashMap<String, Value>` (Eq / Ne / Lt / Gt / Contains /
  And / Or / Not)
- `ControlFlowExecutor` — walks the step table, with
  `max_iterations` + `timeout` caps
- `ExecutionResult` — `final_step`, `context`, `trace`
- 5-variant `ExecutorError` (StepNotFound, ConditionError,
  MaxRecursionExceeded, Timeout, BackendError)
- 10 unit tests + 4 integration tests

### What v0.1.0 explicitly does **not** do

- Parallel branch execution (the `StepKind::Parallel`
  step walks branches sequentially; production will run
  them in B5+)
- Action side-effects (the `StepKind::Action` step is a
  no-op; production will let actions mutate the context
  in B5+)
- Persistence of the trace (the
  `ExecutionResult::trace` is returned but not written to
  a store)
- Honor distributed-trace context propagation

## 関連 IPA フェーズ

22-52 (基本設計/詳細設計), 53-58 (実装), 59-95 (試験)

## 設計書

`docs/modules/M-05-control-flow.md` (DOC-MOD-005)
