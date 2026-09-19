//! abac_smoke — tenant isolation + attrs builder behaviour.

use ada_m11_rbac_collab::{Action, ResourceType};
use ada_m11_rbac_collab::CollaborationMap;
use ada_rbac_casbin::{Attrs, Enforcer, PolicySet};

fn build() -> Enforcer {
    Enforcer::from_m11(&PolicySet::bundled(), &CollaborationMap::new()).expect("enforcer")
}

#[test]
fn tenant_id_propagates_into_attrs() {
    let a = Attrs::new("tenant-a");
    let b = a.clone().with_owner_flag(true);
    assert_eq!(a.tenant_id, "tenant-a");
    assert!(!a.is_owner);
    assert!(b.is_owner);
}

#[test]
fn abac_owner_request_is_allowed() {
    let e = build();
    let attrs = Attrs::new("tenant-a");
    let r = e
        .enforce_typed(
            "user-uuid-1",
            ResourceType::Canvas,
            "canvas-abc",
            Action::Write,
            &attrs,
            None,
        )
        .expect("ok");
    assert!(r, "owner-flagged request is allowed");
}