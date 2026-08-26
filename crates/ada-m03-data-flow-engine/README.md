# ada-m03-data-flow-engine

M-03: データフローエンジン (Data flow engine).
Execute canvas-defined nodes sequentially/parallelly.

## v0.1.0 scope (B5 batch)

This crate is the **minimum skeleton** for the
canvas-defined data flow engine. The v0.1.0 surface is the
in-process sequential executor that the B5+ parallel
executor will replace.

The production deployment (parallel branch execution,
compiled execution plans, tracing integration, see
`DOC-MOD-003` §3.4-§3.6) is scheduled for B5+ once G4
(実装着手判定) is approved.

### What v0.1.0 provides

- `DataFlow` — `id`, `description`, `nodes`, `edges`
- `FlowNode` — `id`, `kind` (`Source / Transform / Sink`),
  `label`
- `FlowEdge` — `from -> to`
- `NodeKind` — the three canonical node kinds
- `NJson` — newtype around `serde_json::Value` (canonical
  NJSON data bus type, D-07)
- `DataFlowEngine` trait — `async fn execute(&DataFlow,
  &HashMap<..>, Value) -> Result<Value>`
- `InMemoryEngine` — topologically-sorted sequential
  executor with per-node `NodeBody` lookup
- `FnNode` — closure adapter for `NodeBody`
- 5-variant `FlowError` (CyclicGraph, UnknownNode,
  ExecutionFailed, TypeMismatch, BackendError)
- 10 unit tests + 4 integration tests

### What v0.1.0 explicitly does **not** do

- Parallel execution of independent branches
  (sequential only; production lands in B5+)
- Compile the flow into a static plan (the `topo_sort`
  runs on every `execute` call)
- Persist execution traces
- Honor the `tracing` layer integration (the
  `ada-telemetry` crate wires that in B5+)

## 関連 IPA フェーズ

22-52 (基本設計/詳細設計), 53-58 (実装), 59-95 (試験)

## 設計書

`docs/modules/M-03-data-flow-engine.md` (DOC-MOD-003)
