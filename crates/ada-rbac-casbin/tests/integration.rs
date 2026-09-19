//! integration.rs — v0.5.0 casbin 2.x contract tests.
//!
//! These exercise the public API end-to-end against the bundled
//! `PolicySet`:
//!
//! 1. `enforce("role:owner", "canvas:abc", Write, &attrs(false), None)` → `Ok(true)`
//! 2. `enforce("role:viewer", "canvas:abc", Delete, &attrs(no_owner), None)` → `Ok(false)`
//! 3. Role ladder: a request with `r.sub = "role:owner"` satisfies the
//!    `p, role:editor, canvas, write, *, *` policy line via the
//!    `add_grouping_policy` wiring.
//!
//! The tests use `PolicySet::bundled()` so they do not touch the
//! workspace layout.

use ada_m11_rbac_collab::{Action, CollaborationMap, ResourceType};
use ada_rbac_casbin::{Attrs, Enforcer, PolicySet};

fn build() -> Enforcer {
    Enforcer::from_m11(&PolicySet::bundled(), &CollaborationMap::new()).expect("enforcer")
}

#[test]
fn owner_can_write_canvas_without_owner_flag() {
    let e = build();
    let attrs = Attrs::new("tenant-a");
    let allowed = e
        .enforce(
            "role:owner",
            "canvas:abc",
            Action::Write,
            &attrs,
            None,
        )
        .expect("enforce ok");
    assert!(
        allowed,
        "role:owner must be allowed to write canvas (policy: p, role:owner, canvas, write, *, *)"
    );
}

#[test]
fn viewer_cannot_delete_canvas_without_owner_flag() {
    let e = build();
    let attrs = Attrs::new("tenant-a");
    let allowed = e
        .enforce(
            "role:viewer",
            "canvas:abc",
            Action::Delete,
            &attrs,
            None,
        )
        .expect("enforce ok");
    assert!(
        !allowed,
        "role:viewer must be denied Delete (no policy line for role:viewer delete)"
    );
}

#[test]
fn role_ladder_owner_satisfies_editor_policy_line() {
    // The matcher uses `g(r.sub, p.sub)`. We wire the ladder via
    // add_grouping_policy(role:owner, role:admin) / (role:admin, role:editor) /
    // (role:editor, role:executor) / (role:executor, role:viewer).
    // A request with `r.sub = "role:owner"` therefore satisfies the
    // `p, role:editor, canvas, write, *, *` line via the ladder.
    let e = build();
    let attrs = Attrs::new("tenant-a");
    let allowed = e
        .enforce(
            "role:owner",
            "canvas:abc",
            Action::Write,
            &attrs,
            None,
        )
        .expect("enforce ok");
    assert!(allowed, "role ladder: owner -> editor write line must match");

    // And vice-versa: a request with `r.sub = "role:editor"` also
    // satisfies that same policy line because it is its own sub.
    let allowed_editor = e
        .enforce(
            "role:editor",
            "canvas:abc",
            Action::Write,
            &attrs,
            None,
        )
        .expect("enforce ok");
    assert!(
        allowed_editor,
        "editor policy line is directly matched by editor request"
    );
}

#[test]
fn enforce_typed_matches_untyped_for_same_inputs() {
    let e = build();
    let attrs = Attrs::new("tenant-a");
    let untyped = e
        .enforce(
            "role:owner",
            "canvas:abc",
            Action::Write,
            &attrs,
            None,
        )
        .expect("enforce ok");
    let typed = e
        .enforce_typed(
            "role:owner",
            ResourceType::Canvas,
            "canvas-abc",
            Action::Write,
            &attrs,
            None,
        )
        .expect("enforce ok");
    assert_eq!(
        untyped, typed,
        "enforce and enforce_typed must agree on the same authorization tuple"
    );
}

#[test]
fn owner_flag_required_for_delete() {
    let e = build();
    let owner_attrs = Attrs::new("tenant-a").with_owner_flag(true);
    let no_owner_attrs = Attrs::new("tenant-a");
    let owner_allowed = e
        .enforce(
            "role:owner",
            "canvas:abc",
            Action::Delete,
            &owner_attrs,
            None,
        )
        .expect("ok");
    let no_owner_denied = e
        .enforce(
            "role:owner",
            "canvas:abc",
            Action::Delete,
            &no_owner_attrs,
            None,
        )
        .expect("ok");
    assert!(owner_allowed, "owner-flagged Delete must be allowed");
    assert!(!no_owner_denied, "non-owner Delete must be denied by is_owner gate");
}