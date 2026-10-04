//! `import_smoke` — verify the enforcer builds from the bundled policy set.

use ada_rbac_casbin::PolicySet;

// Needs a real evaluator; see the note in `tests/enforce_smoke.rs` for
// the `user_id` role-token contract and why the unsupported build has
// no evaluator at all.
#[test]
fn from_m11_returns_a_working_enforcer() {
    use ada_m11_rbac_collab::CollaborationMap;
    use ada_rbac_casbin::Enforcer;

    let m11 = CollaborationMap::new();
    let set = PolicySet::bundled();
    set.validate().expect("policy set valid");
    let _ = Enforcer::from_m11(&set, &m11).expect("enforcer built");
}

#[test]
fn bundled_policy_set_files_exist() {
    let set = PolicySet::bundled();
    assert!(
        set.model_path.exists(),
        "model.conf not found at {}",
        set.model_path.display()
    );
    assert!(
        set.policy_path.exists(),
        "base_policy.csv not found at {}",
        set.policy_path.display()
    );
}
