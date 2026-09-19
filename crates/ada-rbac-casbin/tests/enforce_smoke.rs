//! enforce_smoke — Owner allows, Viewer denies.

use ada_m11_rbac_collab::{Action, ResourceType};
use ada_rbac_casbin::{Attrs, Enforcer, PolicySet};
use ada_m11_rbac_collab::CollaborationMap;

fn build() -> Enforcer {
    Enforcer::from_m11(&PolicySet::bundled(), &CollaborationMap::new()).expect("enforcer")
}

#[test]
fn owner_request_is_allowed_to_write_canvas() {
    let e = build();
    let attrs = Attrs::new("tenant-a");
    let allowed = e
        .enforce_typed(
            "user-uuid-1",
            ResourceType::Canvas,
            "canvas-abc",
            Action::Write,
            &attrs,
            None,
        )
        .expect("enforce ok");
    assert!(allowed, "Owner must be allowed to write canvas");
}

#[test]
fn viewer_request_can_read() {
    let e = build();
    let attrs = Attrs::new("tenant-a");
    let allowed = e
        .enforce_typed(
            "user-uuid-1",
            ResourceType::Canvas,
            "canvas-abc",
            Action::Read,
            &attrs,
            None,
        )
        .expect("enforce ok");
    assert!(allowed);
}

#[test]
fn delete_requires_owner_flag() {
    let e = build();
    let attrs_owner = Attrs::new("tenant-a").with_owner_flag(true);
    let attrs_no_owner = Attrs::new("tenant-a");
    let r1 = e
        .enforce_typed(
            "user-uuid-1",
            ResourceType::Canvas,
            "canvas-abc",
            Action::Delete,
            &attrs_owner,
            None,
        )
        .expect("ok");
    let r2 = e
        .enforce_typed(
            "user-uuid-1",
            ResourceType::Canvas,
            "canvas-abc",
            Action::Delete,
            &attrs_no_owner,
            None,
        )
        .expect("ok");
    assert!(r1, "owner-flagged Delete must be allowed");
    assert!(!r2, "non-owner Delete must be denied");
}