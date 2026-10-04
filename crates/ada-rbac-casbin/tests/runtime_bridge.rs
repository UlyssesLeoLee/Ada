//! The enforcer must be usable from inside a tokio runtime.
//!
//! ## The defect this file exists for
//!
//! `casbin::Enforcer::enforce` is synchronous; only construction is
//! `async`. The adapter bridged the two with
//!
//! ```text
//! match Handle::try_current() {
//!     Ok(h)  => h.block_on(future),
//!     Err(_) => Runtime::new().block_on(future),
//! }
//! ```
//!
//! `Handle::block_on` panics when the calling thread is already driving
//! a runtime, so every call made from an `async fn` — which is every
//! authorization in the api-gateway, and `AppState::new` under
//! `#[tokio::main]` — died with "Cannot start a runtime from within a
//! runtime".
//!
//! ## Why the existing tests did not catch it
//!
//! Every other test in this crate calls the synchronous API from a
//! plain `#[test]`, where `Handle::try_current()` returns `Err` and the
//! fallback arm runs. The panicking arm — `Ok(handle)` — had no
//! coverage at all. `#[tokio::test]` is the only way to reach it, so
//! these tests are `async` and that is the entire point of them.
//!
//! `#[tokio::test]` uses a **current-thread** runtime, which is the
//! harsher of the two cases: `tokio::task::block_in_place`, the usual
//! answer to "sync work inside async", panics there too. A fix that
//! only works on a multi-thread runtime would still fail every test
//! below.

use ada_m11_rbac_collab::{Action, ResourceType};
use ada_rbac_casbin::{Attrs, Enforcer, PolicySet};

fn enforcer() -> Enforcer {
    Enforcer::from_policy_set(&PolicySet::bundled()).expect("bundled policy set must validate")
}

fn viewer_attrs() -> Attrs {
    Attrs::new("tenant-a".to_string())
}

/// Construction from inside a runtime.
///
/// This is the path `AppState::new` takes, and therefore the path a
/// gateway pod takes on startup. Under the old bridge it panicked
/// before the listener was ever bound, which presents as a crash-looping
/// pod rather than as an auth bug.
#[tokio::test]
async fn construction_from_inside_a_runtime_does_not_panic() {
    let e = enforcer();
    assert!(e
        .enforce_typed(
            "role:viewer",
            ResourceType::Canvas,
            "c1",
            Action::Read,
            &viewer_attrs(),
            None
        )
        .expect("enforce"));
}

/// Enforcement from inside a runtime, on the multi-thread flavour of
/// tokio — the configuration `#[tokio::main]` uses by default.
///
/// Same call as the previous test, different runtime. Both had to fail
/// under the old bridge; only the current-thread one is covered by the
/// `#[tokio::test]` default, and a fix that only handled one runtime
/// flavour would be invisible in half the deployments.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn enforcement_from_inside_a_multi_thread_runtime_does_not_panic() {
    let e = enforcer();
    assert!(e
        .enforce_typed(
            "role:viewer",
            ResourceType::Canvas,
            "c1",
            Action::Read,
            &viewer_attrs(),
            None
        )
        .expect("enforce"));
}

/// Repeated enforcement from async context, which is what a served
/// request stream actually does.
///
/// A lock that is only safe to take once is not a usable lock; this is
/// what catches an implementation that "works" by, say, taking the
/// handle out of the runtime once and never putting it back.
#[tokio::test]
async fn repeated_enforcement_from_async_context_is_stable() {
    let e = enforcer();
    for _ in 0..64 {
        let allowed = e
            .enforce_typed(
                "role:viewer",
                ResourceType::Canvas,
                "c1",
                Action::Read,
                &viewer_attrs(),
                None,
            )
            .expect("enforce");
        assert!(allowed, "viewer must retain canvas read across calls");
    }
}

/// The async path must reach the *same verdict* as the sync path, not
/// merely avoid panicking.
///
/// A fix that returned a constant, swallowed an error, or quietly
/// degraded to "deny everything" would satisfy the panic tests above.
/// This one pins the answer.
#[tokio::test]
async fn async_and_sync_contexts_agree() {
    let e = enforcer();

    // Built and used entirely inside the runtime.
    let in_runtime = e
        .enforce_typed(
            "role:viewer",
            ResourceType::Canvas,
            "c1",
            Action::Execute,
            &viewer_attrs(),
            None,
        )
        .expect("enforce");

    // A plain synchronous call on a separate thread, outside any
    // runtime — the shape every other test in this crate uses.
    let out_of_runtime = std::thread::spawn({
        let policy = PolicySet::bundled();
        move || {
            let e = Enforcer::from_policy_set(&policy).expect("build");
            e.enforce_typed(
                "role:viewer",
                ResourceType::Canvas,
                "c1",
                Action::Execute,
                &viewer_attrs(),
                None,
            )
            .expect("enforce")
        }
    })
    .join()
    .expect("sync thread");

    assert_eq!(
        in_runtime, out_of_runtime,
        "the runtime context must not change the verdict"
    );
    assert!(
        !in_runtime,
        "role:viewer has no canvas execute row, so this must be denied"
    );
}
