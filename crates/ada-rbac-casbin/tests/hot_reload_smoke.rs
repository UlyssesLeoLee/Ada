//! hot_reload_smoke — reload_now swaps the enforcer in place.
//!
//! v0.5.0 makes `spawn_watcher` real (notify-backed); the watcher is
//! short-lived so we exercise the construction path only.

use ada_rbac_casbin::{HotReload, PolicySet};

#[test]
fn reload_now_constructs_a_hot_reload() {
    let set = PolicySet::bundled();
    let hr = HotReload::new(&set).expect("hot reload");
    let _current = hr.current();
    hr.reload_now().expect("reload_now");
}

#[test]
fn policy_path_exposed() {
    let set = PolicySet::bundled();
    let hr = HotReload::new(&set).expect("hot reload");
    assert!(hr.policy_path().ends_with("base_policy.csv"));
}

#[test]
fn watcher_succeeds_in_v5() {
    // v0.5.0: spawn_watcher returns an `ActiveWatcher` (notify-backed)
    // that lives until dropped. The v0.4.0 deferred behaviour is
    // removed; we only assert the watcher constructs cleanly here —
    // the real watcher semantics are covered by `tests/hot_reload_real.rs`.
    let set = PolicySet::bundled();
    let hr = HotReload::new(&set).expect("hot reload");
    let watcher = hr.spawn_watcher().expect("v0.5.0 spawn_watcher succeeds");
    drop(watcher);
}