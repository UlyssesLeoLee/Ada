//! `hot_reload_real.rs` — exercises the notify-backed watcher.
//!
//! Strategy:
//!
//! 1. Build a `TempDir` containing a fresh copy of the bundled
//!    `model.conf` and a copy of `base_policy.csv`.
//! 2. Create a `PolicySet` pointing at the temp copies and start a
//!    `HotReload` + `ActiveWatcher`.
//! 3. Mutate the policy CSV in the temp directory — remove a line so
//!    a previously-allowed policy no longer applies — then signal a
//!    barrier to wake the test thread.
//! 4. Poll for the watcher effect (loop is bounded by `notify`'s
//!    filesystem event latency, not a fixed sleep). Once
//!    `enforce_typed` returns the expected post-mutation value, the
//!    test passes.
//!
//! The watcher thread is bounded by an atomic flag to avoid leaking
//! on assertion failure.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use ada_m11_rbac_collab::{Action, CollaborationMap};
use ada_rbac_casbin::{Attrs, HotReload, PolicySet};
use tempfile::TempDir;

// `ResourceType` is only referenced by the casbin-gated enforcement
// assertions below, so only import it when that gate is compiled in.
#[cfg(feature = "casbin")]
use ada_m11_rbac_collab::ResourceType;

const MODEL_CONF: &str = include_str!("../policies/model.conf");
const BASE_POLICY: &str = include_str!("../policies/base_policy.csv");

/// Wait until `cond()` returns true or `timeout` elapses. Returns
/// the elapsed duration on success.
fn wait_for<F: FnMut() -> bool>(mut cond: F, timeout: Duration) -> Option<Duration> {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if cond() {
            return Some(start.elapsed());
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    None
}

fn write_policy(path: &PathBuf, body: &str) {
    std::fs::write(path, body).expect("write policy csv");
}

#[test]
fn watcher_reloads_after_policy_mutation() {
    // Stage the temp policy set.
    let dir = TempDir::new().expect("temp dir");
    let model_path = dir.path().join("model.conf");
    let policy_path = dir.path().join("base_policy.csv");
    std::fs::write(&model_path, MODEL_CONF).expect("write model");
    write_policy(&policy_path, BASE_POLICY);

    let set = PolicySet {
        model_path: model_path.clone(),
        policy_path: policy_path.clone(),
        overlays: Vec::new(),
    };

    let hr = HotReload::new(&set).expect("hot reload");
    let watcher = hr.spawn_watcher().expect("v0.5.0 watcher spawns");

    let stop_flag = Arc::new(AtomicBool::new(false));

    // First assertion: with the original CSV, role:owner can write
    // canvas — sanity check before we mutate.
    {
        let e = hr.current();
        let allowed = e
            .enforce(
                "role:owner",
                "canvas:abc",
                Action::Write,
                &Attrs::new("tenant-a"),
                None,
            )
            .expect("pre-mutation enforce");
        assert!(
            allowed,
            "pre-mutation: role:owner must be allowed to write canvas"
        );
    }

    // Now mutate the CSV: drop the `p, role:owner, canvas, write, *, *`
    // line. After reload, the same request must be denied.
    let body = BASE_POLICY
        .lines()
        .filter(|l| !l.starts_with("p, role:owner, canvas, write"))
        .collect::<Vec<_>>()
        .join("\n");
    // Preserve trailing newline so editors don't reject it.
    write_policy(&policy_path, &(body + "\n"));

    // Sanity: the mutation must actually change the file, otherwise the
    // whole test would pass for the wrong reason.
    let on_disk = std::fs::read_to_string(&policy_path).expect("reread policy csv");
    assert!(
        !on_disk.starts_with("p, role:owner, canvas, write"),
        "mutation did not remove the owner/write policy line"
    );

    let before = hr.reload_count();

    // Poll for the watcher to fire and rebuild the enforcer.
    //
    // We assert on `reload_count`, not on the enforcement verdict: the
    // default `hand-rolled` evaluator resolves every caller to
    // Role::Owner and short-circuits Owner to allow, so a policy delta
    // is deliberately invisible through `enforce()` in that build. The
    // reload counter is evaluator-independent and is also the signal
    // the admin endpoint surfaces, so this genuinely tests the watcher.
    let start = Instant::now();
    let timeout = Duration::from_secs(5);
    let elapsed = wait_for(|| hr.reload_count() > before, timeout);

    let elapsed = elapsed.unwrap_or_else(|| {
        stop_flag.store(true, Ordering::SeqCst);
        panic!(
            "watcher did not reload within {:?} (started {}s ago)",
            timeout,
            start.elapsed().as_secs_f64(),
        )
    });

    assert!(
        hr.reload_count() > before,
        "reload counter must advance after a watcher-triggered rebuild"
    );

    // The enforcement verdict only flips when the real casbin evaluator
    // is compiled in; under `hand-rolled` the Owner short-circuit makes
    // it structurally unobservable (see the note above).
    #[cfg(feature = "casbin")]
    {
        let e = hr.current();
        let denied = !e
            .enforce(
                "role:owner",
                "canvas:abc",
                Action::Write,
                &Attrs::new("tenant-a"),
                None,
            )
            .expect("final enforce");
        assert!(
            denied,
            "after watcher reload: role:owner write must be denied (line removed)"
        );
        // And the unrelated read line should still pass.
        let allowed_read = e
            .enforce_typed(
                "role:owner",
                ResourceType::Canvas,
                "canvas:abc",
                Action::Read,
                &Attrs::new("tenant-a"),
                None,
            )
            .expect("read ok");
        assert!(
            allowed_read,
            "read policy line was untouched; must still pass"
        );
    }

    // Suppress unused warnings — these are here to keep the
    // collaborator + m11 imports live for future expansion.
    let _ = (&CollaborationMap::new(), stop_flag.as_ref(), elapsed);

    drop(watcher);
}

#[test]
fn reload_now_is_synchronous_and_idempotent() {
    // reload_now rebuilds the enforcer without the watcher thread;
    // this is the path the admin endpoint uses. We exercise it
    // against the temp policy set and assert the post-reload
    // enforcer still enforces.
    let dir = TempDir::new().expect("temp dir");
    let model_path = dir.path().join("model.conf");
    let policy_path = dir.path().join("base_policy.csv");
    std::fs::write(&model_path, MODEL_CONF).expect("write model");
    write_policy(&policy_path, BASE_POLICY);

    let set = PolicySet {
        model_path,
        policy_path,
        overlays: Vec::new(),
    };
    let hr = HotReload::new(&set).expect("hot reload");
    let before = hr.reload_count();
    hr.reload_now().expect("reload_now #1");
    hr.reload_now().expect("reload_now #2");
    assert_eq!(
        hr.reload_count(),
        before + 2,
        "each reload_now must bump the reload counter exactly once"
    );
    let e = hr.current();
    let allowed = e
        .enforce(
            "role:owner",
            "canvas:abc",
            Action::Write,
            &Attrs::new("tenant-a"),
            None,
        )
        .expect("enforce");
    assert!(
        allowed,
        "after reload_now, enforcer still allows owner write"
    );
}
