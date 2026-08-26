# ada-m12-canvas-editor

M-12: Canvas editor. 3 `NodeKind` (`Block / Connector / Note`),
`Canvas` document with optimistic-concurrency version, and a
linear `EditHistory` with undo/redo.

See `docs/modules/M-12-canvas-editor.md` (DOC-MOD-012) for the
full design.

## v0.1.0 status

Skeleton. Real CRDT collaboration (yrs/Yjs) lands in B7+;
optimistic-concurrency versioning is provided as the
single-writer fallback.

## v0.1.0 surface

- `NodeKind` — `Block | Connector | Note`
- `CanvasNode` — id, kind, position, label, ports
- `Position` — 2-D integer coordinates
- `Port` — name (input / output / ...)
- `Edge` — directed `from -> to`
- `Canvas` — `add_node / remove_node / move_node / add_edge / check_version / get_node`
- `EditOp` — `InsertNode | RemoveNode | MoveNode | AddEdge`
- `EditHistory` — linear undo/redo with branch reset
- 5-variant `CanvasError`
- ~30 unit tests + 4 integration tests
