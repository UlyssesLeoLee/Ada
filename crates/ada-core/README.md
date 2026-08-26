# `ada-core`

> Shared types, error surface, and the `telemetry!` macro for the
> Ada workspace — the **shared layer** of the 仿生モデル (bionic
> model) defined in `docs/architecture/00-anatomy-model.md`.

- **Version**: workspace `0.1.0` (D-09: single workspace version)
- **License**: MIT (D-13)
- **Layer**: `shared`
- **MSRV**: 1.74 (workspace-fixed)

---

## What v0.1.0 provides

| Item | Type | Notes |
|---|---|---|
| `AdaError` | `enum` | workspace-wide error type, `thiserror` (D-ADR-D-05) |
| `Result<T>` | type alias | `Result<T, AdaError>`, `#[must_use]` via `Result` |
| `TenantId` | newtype (`pub Uuid`) | multi-tenant isolation key |
| `UserId` | newtype (`pub Uuid`) | subject identifier |
| `CanvasId` | newtype (`pub Uuid`) | canvas (authoring / execution unit) identifier |
| `IdempotencyKey` | newtype (`pub Uuid`) | at-least-once + idempotent consumer (D-07) |
| `AdaLayer` | `enum` | `Skeleton` / `Blood` / `Nerve` / `Muscle` / `Shared` |
| `telemetry!` | `macro_rules!` | thin wrapper around `tracing::info_span!` |
| `VERSION` / `NAME` / `LAYER` | `&str` consts | unchanged from the v0.1.0 scaffold |

All newtypes derive `Debug` + `Clone` + `Copy` + `PartialEq` + `Eq`
+ `Hash` + `Serialize` + `Deserialize`, and implement `Display` as
`tenant(<uuid>)`, `user(<uuid>)`, `canvas(<uuid>)`,
`idempotency(<uuid>)`.

`AdaError` is `Send + Sync + 'static` (its payload types are too) and
implements `std::error::Error` via the `thiserror::Error` derive.

## What is **not** in `ada-core` yet (out of scope for B1)

- `NJson` and the NJSON data-bus types — owned by `ada-m03-data-flow-engine`
- RBAC / permission types — owned by `ada-m11-rbac-collab`
- Concrete tracing subscriber / metrics exporter — owned by `ada-telemetry`
- `Permission` / `Resource` / `AuditEvent` etc. — slated for later
  milestones; the B1 scope is the 9 items above and we deliberately
  do not exceed it (see `docs/architecture/06-rust-tech-selection.md`
  §18 for which types live in which crate).

## How to use it

```rust
use ada_core::{
    AdaError, Result, AdaLayer, TenantId, CanvasId, IdempotencyKey,
    telemetry,
};
use uuid::Uuid;

// Build IDs
let tenant = TenantId(Uuid::new_v4());
let canvas = CanvasId(Uuid::new_v4());
let idem   = IdempotencyKey(Uuid::new_v4());

// Layer-tagged tracing span (expands to a tracing::info_span!)
let _span = telemetry!(layer: AdaLayer::Nerve, "canvas executed", canvas_id = %canvas);

// Error handling
fn load_tenant(_id: TenantId) -> Result<()> {
    Err(AdaError::NotFound { entity: "tenant", id: "t-1".to_string() })
}
```

## Acceptance gates (B1)

| Gate | Status |
|---|---|
| `cargo check -p ada-core` | green |
| `cargo test -p ada-core` | green — **23** unit tests |
| `cargo check --workspace` | green — no other crate broken |
| `cargo clippy -p ada-core --all-targets -- -D warnings` | green — 0 warnings (pedantic) |
| `cargo fmt --all -- --check` | green — 0 diff |

## Layout

```
crates/ada-core/
├── Cargo.toml      # crate-local deps (uuid, serde, thiserror, tracing)
│                   # + serde_json dev-dep for round-trip tests
├── README.md       # this file
└── src/
    ├── lib.rs      # mod decls, re-exports, VERSION/NAME/LAYER consts
    ├── error.rs    # AdaError + Result
    ├── types.rs    # 4 newtypes + AdaLayer
    └── telemetry.rs # telemetry! macro
```

## References

- `docs/architecture/00-anatomy-model.md` — shared-layer scope
- `docs/architecture/06-rust-tech-selection.md` — crate / `thiserror` /
  `tracing` / `uuid` rationale
- `docs/decisions/02-design-adrs.md`:
  - **D-09** single workspace version
  - **D-13** `ada-core` = MIT
  - **D-07** at-least-once + idempotent consumer (`IdempotencyKey`)
