# ada-rbac-casbin

> RBAC + ABAC wrapper around `ada-m11-rbac-collab`. Per-tier v0.4.0
> auth/billing scaffold.

## v0.4.0 surface

| Module       | Purpose                                                                |
|--------------|------------------------------------------------------------------------|
| `enforcer`   | hand-rolled RBAC + ABAC evaluator (m11 role × permission matrix)       |
| `attrs`      | typed ABAC attribute bag (tenant, is_owner, now, ip)                  |
| `policy`     | `PolicySet` reader + `bundled()` loader                                |
| `admin`      | `AdminApi` for `POST /admin/policies` (Owner-gated by the api-gateway) |
| `hot_reload` | `HotReload` wrapper with `reload_now()`; `spawn_watcher` deferred      |

## v0.5.0 deferred

- `casbin` 2.x adapter (the API surface for the async + rhai
  default engine did not match the v0.4.0 skeleton contract).
- Real `notify`-based file watcher for `spawn_watcher`.
- Postgres-backed per-tenant policy overlays (the v0.4.0
  `AdminApi` holds overrides in memory).

## Public API

```rust
use ada_rbac_casbin::{Enforcer, PolicySet, Attrs};
use ada_m11_rbac_collab::CollaborationMap;

let set = PolicySet::bundled();
let m11 = CollaborationMap::new();
let e = Enforcer::from_m11(&set, &m11)?;
let attrs = Attrs::new("tenant-abc");
let allowed = e.enforce_typed(
    "user-uuid",
    ada_m11_rbac_collab::ResourceType::Canvas,
    "canvas-xyz",
    ada_m11_rbac_collab::Action::Write,
    &attrs,
    Some(&m11),
)?;
# Ok::<(), ada_rbac_casbin::RbacCasbinError>(())
```