//! `unsupported_target` — the "no evaluator compiled in" guard.
//!
//! A build that falls into the unsupported configuration (the
//! `hand-rolled` feature is OFF and the target is neither Linux nor
//! macOS — i.e. Windows) has no evaluator compiled in: `casbin` is only
//! declared as a dependency for Linux/macOS, and the hand-rolled
//! evaluator is opt-in. Rather than fall back to something permissive,
//! both constructors must return a hard error.
//!
//! This is the regression guard for the whole point of the change: a
//! misconfigured build has to fail loudly at construction time instead
//! of quietly authorizing every request.
//!
//! The tests are gated to that configuration so they neither run nor
//! break the Linux/macOS build.

#![cfg(all(
    not(feature = "hand-rolled"),
    not(any(target_os = "linux", target_os = "macos"))
))]

use ada_m11_rbac_collab::CollaborationMap;
use ada_rbac_casbin::{Attrs, Enforcer, HotReload, PolicySet, RbacCasbinError};

#[test]
fn from_policy_set_is_a_hard_error_not_a_permissive_enforcer() {
    let err = Enforcer::from_policy_set(&PolicySet::bundled())
        .expect_err("unsupported build must refuse to build an enforcer");
    assert!(
        matches!(err, RbacCasbinError::UnsupportedEvaluator { .. }),
        "expected UnsupportedEvaluator, got {err:?}"
    );
}

#[test]
fn from_m11_is_a_hard_error_not_a_permissive_enforcer() {
    let err = Enforcer::from_m11(&PolicySet::bundled(), &CollaborationMap::new())
        .expect_err("unsupported build must refuse to build an enforcer");
    assert!(
        matches!(err, RbacCasbinError::UnsupportedEvaluator { .. }),
        "expected UnsupportedEvaluator, got {err:?}"
    );
}

#[test]
fn hot_reload_propagates_the_same_hard_error() {
    // `HotReload::new` builds an `Enforcer`, so it must refuse too
    // rather than handing back a handle that cannot authorize.
    let err = HotReload::new(&PolicySet::bundled())
        .expect_err("unsupported build must refuse to build a HotReload");
    assert!(
        matches!(err, RbacCasbinError::UnsupportedEvaluator { .. }),
        "expected UnsupportedEvaluator, got {err:?}"
    );
}

#[test]
fn the_error_names_the_target_and_the_fix() {
    let err = Enforcer::from_policy_set(&PolicySet::bundled())
        .expect_err("unsupported build must refuse to build an enforcer");
    let rendered = err.to_string();
    assert!(
        rendered.contains(std::env::consts::OS),
        "error should name the target it was built for, got: {rendered}"
    );
    assert!(
        rendered.contains("hand-rolled"),
        "error should tell the operator which feature turns an evaluator on, got: {rendered}"
    );
}

// The `Attrs` import is used only to document that no verdict of any
// kind is reachable here; keeping a reference to it makes the intent
// explicit without needing an `Enforcer` to call `enforce` on.
#[test]
fn no_verdict_is_reachable_on_this_build() {
    // There is no `Enforcer` value to call, so this test only asserts
    // the premise: the constructor refuses, therefore no allow/deny
    // verdict can be produced from this build.
    assert!(
        Enforcer::from_policy_set(&PolicySet::bundled()).is_err(),
        "if this ever starts succeeding, an evaluator leaked into the \
         unsupported build and the fail-closed guarantee is gone"
    );
    let _ = Attrs::new("tenant-a");
}
