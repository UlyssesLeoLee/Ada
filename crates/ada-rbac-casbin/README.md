# ada-rbac-casbin

> RBAC + ABAC wrapper around `ada-m11-rbac-collab`. v0.5.0 ships a
> real `casbin` 2.x adapter with `notify`-based hot reload.

## v0.5.0 surface

| Module       | Purpose                                                                |
|--------------|------------------------------------------------------------------------|
| `enforcer`   | thin facade over the casbin implementation (was hand-rolled in v0.4.0) |
| `casbin_impl`| real `casbin::Enforcer` + `add_grouping_policy` for the role ladder    |
| `attrs`      | typed ABAC attribute bag (tenant, is_owner, now, ip)                   |
| `policy`     | `PolicySet` reader + `bundled()` loader                                |
| `admin`      | `AdminApi` with `persist` flag (default off) for v0.6.0 Postgres       |
| `hot_reload` | `HotReload` with `reload_now()` and real `notify`-backed `spawn_watcher` |

## v0.5.0 contract

- Internal evaluator is `casbin::Enforcer::new(model, FileAdapter)`.
- The role ladder (Owner > Admin > Editor > Executor > Viewer) is
  wired via `add_grouping_policy(role:owner, role:admin)` etc.
- Request tuples are `(sub, obj, act, tenant, is_owner_token)` per
  `policies/model.conf`.
- `HotReload::spawn_watcher` is real: a `notify::recommended_watcher`
  thread rebuilds the enforcer on `Create` / `Modify` / `Remove`
  events for the policy CSV and atomically swaps the handle.
- `AdminApi` keeps the in-memory override list and adds a
  `persist` flag (default `false`) so v0.6.0 can wire Postgres
  without an API change.

## Features

Evaluator selection is a two-way function of the feature, and does not
depend on the target:

| Target   | Features       | Evaluator   | `Enforcer::from_policy_set` |
|----------|----------------|-------------|------------------------------|
| any      | `default = []` | casbin 2.x  | builds the casbin enforcer  |
| any      | `hand-rolled`  | hand-rolled | builds the hand-rolled one  |

So **no flags are needed on any platform**, and `--features hand-rolled`
is the escape hatch for exercising the fallback evaluator.

A previous revision gated `casbin` on `cfg(target_os = "linux" | "macos")`
and documented the reason as casbin's transitive `openssl-sys`
dependency. That is not true of casbin 2.20 with
`default-features = false`: its only dependencies are async-trait,
fixedbitset, getrandom, hashlink, once_cell, parking_lot, petgraph,
regex, rhai, serde, serde_json, thiserror, tokio and wasm-bindgen-test,
all pure Rust. Building it on `x86_64-pc-windows-msvc` takes 4m20s and
needs no system libraries.

The gate was not free. It confined the production evaluator to one
platform, and since this repository's Linux CI had failed to start any
job in 24 consecutive runs, the casbin evaluator was not compiled
anywhere at all. Turning it on locally on Windows immediately surfaced
two live authorization defects that had been sitting in `casbin_impl.rs`
unnoticed: `enforce` hardcoded `ResourceType::Canvas` and ignored the
caller's object entirely, and the ownership gate that the hand-rolled
evaluator enforced was inert because every policy row sets `is_owner` to
`"*"`.

Ownership is now enforced in code, by
`ada_rbac_casbin::contract::requires_ownership`, which both evaluators
call before consulting the policy — so a runtime `AdminApi` policy row
cannot reopen the hole.

## Public API

```rust
use ada_rbac_casbin::{Enforcer, PolicySet, Attrs};
use ada_m11_rbac_collab::CollaborationMap;

let set = PolicySet::bundled();
let m11 = CollaborationMap::new();
let e = Enforcer::from_m11(&set, &m11)?;
let attrs = Attrs::new("tenant-abc");
let allowed = e.enforce_typed(
    "role:owner",
    ada_m11_rbac_collab::ResourceType::Canvas,
    "canvas-xyz",
    ada_m11_rbac_collab::Action::Write,
    &attrs,
    Some(&m11),
)?;
# Ok::<(), ada_rbac_casbin::RbacCasbinError>(())
```