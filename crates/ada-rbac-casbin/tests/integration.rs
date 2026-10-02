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

// Every test below needs a real evaluator. One is always compiled in
// now: casbin 2.x by default, the hand-rolled one under
// `--features hand-rolled`. There is no target-gated configuration
// left that could leave this build without one.
// for the guard on that path.
mod evaluator {
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
            .enforce("role:owner", "canvas:abc", Action::Write, &attrs, None)
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
            .enforce("role:viewer", "canvas:abc", Action::Delete, &attrs, None)
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
            .enforce("role:owner", "canvas:abc", Action::Write, &attrs, None)
            .expect("enforce ok");
        assert!(
            allowed,
            "role ladder: owner -> editor write line must match"
        );

        // And vice-versa: a request with `r.sub = "role:editor"` also
        // satisfies that same policy line because it is its own sub.
        let allowed_editor = e
            .enforce("role:editor", "canvas:abc", Action::Write, &attrs, None)
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
            .enforce("role:owner", "canvas:abc", Action::Write, &attrs, None)
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
        assert!(
            !no_owner_denied,
            "non-owner Delete must be denied by is_owner gate"
        );
    }

    // --- Negative coverage for the resource-type leak -------------
    //
    // These three assert that a grant on one resource type does not
    // carry over to another. They fail against any evaluator that
    // checks the action against the union of all resource types
    // instead of the specific `(role, resource_type)` pair: the m11
    // matrix gives Editor `Write` on Canvas only, so Editor must be
    // denied `Write` on Workspace and on Credential.
    //
    // They are evaluator-independent assertions about the policy in
    // `policies/base_policy.csv` and in m11's `role_permissions`.

    #[test]
    fn viewer_is_denied_write_on_canvas() {
        let e = build();
        let denied = e
            .enforce_typed(
                "role:viewer",
                ResourceType::Canvas,
                "canvas-abc",
                Action::Write,
                &Attrs::new("tenant-a"),
                None,
            )
            .expect("enforce ok");
        assert!(
            !denied,
            "role:viewer holds Read on Canvas only and must be denied Write"
        );
    }

    #[test]
    fn editor_is_denied_write_on_workspace() {
        let e = build();
        let denied = e
            .enforce_typed(
                "role:editor",
                ResourceType::Workspace,
                "workspace-abc",
                Action::Write,
                &Attrs::new("tenant-a"),
                None,
            )
            .expect("enforce ok");
        assert!(
            !denied,
            "role:editor holds Write on Canvas only; Write on Workspace must be denied \
             (there is no `p, role:editor, workspace, write` policy line)"
        );
    }

    #[test]
    fn editor_is_denied_write_on_credential() {
        let e = build();
        let denied = e
            .enforce_typed(
                "role:editor",
                ResourceType::Credential,
                "credential-abc",
                Action::Write,
                &Attrs::new("tenant-a"),
                None,
            )
            .expect("enforce ok");
        assert!(
            !denied,
            "role:editor holds Write on Canvas only; Write on Credential must be denied \
             (there is no `p, role:editor, credential, write` policy line)"
        );
    }

    // The positive counterpart, so the pair above cannot pass merely
    // because Editor was denied everything: Editor keeps the Canvas
    // grant the matrix actually gives it.
    #[test]
    fn editor_is_still_allowed_write_on_canvas() {
        let e = build();
        let allowed = e
            .enforce_typed(
                "role:editor",
                ResourceType::Canvas,
                "canvas-abc",
                Action::Write,
                &Attrs::new("tenant-a"),
                None,
            )
            .expect("enforce ok");
        assert!(allowed, "role:editor must keep its Write grant on Canvas");
    }

    // --- Fail-closed on an unresolvable subject --------------------
    //
    // The evaluators resolve the subject token to exactly one role.
    // A token that names no role must be denied outright rather than
    // falling back to a default role. This is the assertion that
    // `enforce_smoke.rs` used to encode backwards: it passed a bare
    // `"user-uuid-1"` and asserted `true`, which only held because
    // every subject used to resolve to `Role::Owner`.

    #[test]
    fn subject_that_names_no_role_is_denied() {
        let e = build();
        for subject in ["user-uuid-1", "", "role:not-a-role", "owner", "role:"] {
            let allowed = e
                .enforce(
                    subject,
                    "canvas:abc",
                    Action::Write,
                    &Attrs::new("tenant-a"),
                    None,
                )
                .expect("enforce ok");
            assert!(
                !allowed,
                "subject {subject:?} names no role and must be denied Write, not defaulted to one"
            );
        }
    }

    #[test]
    fn object_without_a_known_resource_type_is_denied() {
        let e = build();
        for object in ["abc", "notaresource:abc", "canvas", ""] {
            let allowed = e
                .enforce(
                    "role:owner",
                    object,
                    Action::Write,
                    &Attrs::new("tenant-a"),
                    None,
                )
                .expect("enforce ok");
            assert!(
                !allowed,
                "object {object:?} carries no known resource type and must be denied"
            );
        }
    }
}
