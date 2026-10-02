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

Evaluator selection is a three-way function of the feature and the
target:

| Target         | Features         | Evaluator      | `Enforcer::from_policy_set` |
|----------------|------------------|----------------|------------------------------|
| linux / macOS  | `default = []`   | casbin 2.x     | builds the casbin enforcer  |
| linux / macOS  | `hand-rolled`    | hand-rolled    | builds the hand-rolled one  |
| any other      | `default = []`   | none           | `Err(UnsupportedEvaluator)` |
| any other      | `hand-rolled`    | hand-rolled    | builds the hand-rolled one  |

So **Linux/macOS need no flags**, and **every other target (Windows)
must be built with `--features hand-rolled`** — `casbin` is only
declared as a dependency for Linux/macOS, so a Windows build with the
feature off has no evaluator and refuses to construct one rather than
falling back to something permissive.

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