//! `abac_smoke` — tenant isolation + attrs builder behaviour.

use ada_rbac_casbin::Attrs;

#[test]
fn tenant_id_propagates_into_attrs() {
    let a = Attrs::new("tenant-a");
    let b = a.clone().with_owner_flag(true);
    assert_eq!(a.tenant_id, "tenant-a");
    assert!(!a.is_owner);
    assert!(b.is_owner);
}

// Needs a real evaluator; see the note in `tests/enforce_smoke.rs` for
// why the bare `"user-uuid-1"` subject this used to pass (asserting
// `true`) encoded the authorization bypass rather than the policy.
#[test]
fn abac_owner_request_is_allowed() {
    use ada_m11_rbac_collab::CollaborationMap;
    use ada_m11_rbac_collab::{Action, ResourceType};
    use ada_rbac_casbin::{Enforcer, PolicySet};

    let e = Enforcer::from_m11(&PolicySet::bundled(), &CollaborationMap::new()).expect("enforcer");
    let attrs = Attrs::new("tenant-a");
    let r = e
        .enforce_typed(
            "role:owner",
            ResourceType::Canvas,
            "canvas-abc",
            Action::Write,
            &attrs,
            None,
        )
        .expect("ok");
    assert!(r, "owner-role request is allowed");
}
