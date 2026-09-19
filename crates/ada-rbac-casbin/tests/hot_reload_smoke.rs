//! hot_reload_smoke — reload_now swaps the enforcer in place.

use ada_m11_rbac_collab::CollaborationMap;
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
fn watcher_is_v5_deferred() {
    let set = PolicySet::bundled();
    let hr = HotReload::new(&set).expect("hot reload");
    let r = hr.spawn_watcher();
    assert!(r.is_err(), "spawn_watcher is deferred to v0.5.0; must return Err");
}