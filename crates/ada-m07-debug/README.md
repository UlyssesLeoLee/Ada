# ada-m07-debug

M-07: Debug tools. Breakpoints (3 kinds × 3 states), stack
inspector, and a bounded in-process trace recorder.

See `docs/modules/M-07-debug.md` (DOC-MOD-007) for the full
design.

## v0.1.0 status

Skeleton. Real debugger / `ptrace` integration lands in B7+.

## v0.1.0 surface

- `Breakpoint` — id, location, kind, state
- `BreakpointKind` — `Line | Conditional | Entry`
- `BreakpointState` — `Active | Disabled | Hit`
- `Location` — `Line { file, line } | Function(String)`
- `InspectFrame` / `Inspector` — call-stack snapshot
- `TraceEvent` / `TraceRecorder` — bounded buffer, overflow flag
- 5-variant `DebugError`
- ~30 unit tests + 4 integration tests
