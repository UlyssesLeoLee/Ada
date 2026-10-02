//! `enforce_smoke` — Owner allows, Viewer denies.

// Every test below needs a real evaluator. The evaluators live behind
// `hand-rolled`, or behind the casbin dependency that is only declared
// for Linux/macOS, so a build with neither (the "unsupported"
// configuration) has no evaluator and its constructors return a hard
// error by design — see `tests/unsupported_target.rs` for the guard
// on that path.
#[cfg(any(feature = "hand-rolled", target_os = "linux", target_os = "macos"))]
mod evaluator {
    use ada_m11_rbac_collab::CollaborationMap;
    use ada_m11_rbac_collab::{Action, ResourceType};
    use ada_rbac_casbin::{Attrs, Enforcer, PolicySet};

    fn build() -> Enforcer {
        Enforcer::from_m11(&PolicySet::bundled(), &CollaborationMap::new()).expect("enforcer")
    }

    // WHY THE SUBJECT IS A ROLE TOKEN
    //
    // These three tests used to pass the bare string `"user-uuid-1"` and
    // assert `allowed`. That was wrong, and it encoded the
    // authorization bypass rather than the intended policy:
    //
    // - `"user-uuid-1"` is not a role. It names no role, so under the
    //   documented contract (`user_id` is the caller's resolved role
    //   token, e.g. `"role:owner"` — see `Enforcer::enforce`) it must
    //   be DENIED.
    // - It asserted `allowed` only because the hand-rolled evaluator
    //   ignored its `user_id` argument entirely, returned all five
    //   roles with `Role::Owner` first, and `check()` short-circuited
    //   `Role::Owner` to `true`. Every subject was therefore allowed
    //   every action, and these tests passed for the wrong reason.
    // - The test names already said what was meant: the first is an
    //   *owner* request, the second a *viewer* request. They were
    //   passing a subject that was neither.
    //
    // Each test now passes the role its name describes and asserts the
    // outcome that name implies. The deny case for an unresolvable
    // subject is covered by
    // `subject_without_a_role_token_is_denied` below.

    #[test]
    fn owner_request_is_allowed_to_write_canvas() {
        let e = build();
        let attrs = Attrs::new("tenant-a");
        let allowed = e
            .enforce_typed(
                "role:owner",
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
                "role:viewer",
                ResourceType::Canvas,
                "canvas-abc",
                Action::Read,
                &attrs,
                None,
            )
            .expect("enforce ok");
        assert!(allowed, "Viewer must be allowed to read canvas");
    }

    #[test]
    fn delete_requires_owner_flag() {
        let e = build();
        let attrs_owner = Attrs::new("tenant-a").with_owner_flag(true);
        let attrs_no_owner = Attrs::new("tenant-a");
        let r1 = e
            .enforce_typed(
                "role:owner",
                ResourceType::Canvas,
                "canvas-abc",
                Action::Delete,
                &attrs_owner,
                None,
            )
            .expect("ok");
        let r2 = e
            .enforce_typed(
                "role:owner",
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

    // The corrected form of the old `"user-uuid-1"` + `assert!(allowed)`
    // pair: a subject that names no role is now denied instead of being
    // silently promoted to `Role::Owner`.
    #[test]
    fn subject_without_a_role_token_is_denied() {
        let e = build();
        let denied = e
            .enforce_typed(
                "user-uuid-1",
                ResourceType::Canvas,
                "canvas-abc",
                Action::Write,
                &Attrs::new("tenant-a"),
                None,
            )
            .expect("enforce ok");
        assert!(
            !denied,
            "\"user-uuid-1\" names no role, so it must be denied rather than \
             defaulting to Role::Owner"
        );
    }
}
