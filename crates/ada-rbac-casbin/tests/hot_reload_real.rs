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

// `HotReload::new` builds an `Enforcer`, so every test below needs a real
// evaluator. One is always compiled in now: casbin 2.x by default, the
// hand-rolled one under `--features hand-rolled`. There is no target-gated
// configuration left that could leave this build without one.
mod evaluator {
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use ada_m11_rbac_collab::{Action, CollaborationMap};
    use ada_rbac_casbin::{Attrs, HotReload, PolicySet};
    use tempfile::TempDir;

    // The real casbin evaluator — as opposed to the hand-rolled one —
    // is compiled whenever `hand-rolled` is off, on every platform. This
    // replaced a `#[cfg(feature = "casbin")]` gate that never matched
    // reality: nothing in `src/` read that feature, so the casbin-only
    // assertions below were dead code. They have been unreachable on
    // Windows ever since, because casbin itself was declared only for
    // Linux/macOS.
    const CASBIN_EVALUATOR: bool = cfg!(not(feature = "hand-rolled"));

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

        // Now mutate the CSV: drop every row granting `write` on `canvas`.
        //
        // Dropping only the `role:owner` row is NOT enough, and the
        // previous version of this test did exactly that, which is why it
        // only ever passed on the hand-rolled evaluator. The casbin
        // matcher resolves `g(r.sub, p.sub)`, and `build_enforcer` wires
        // the ladder `role:owner -> role:admin -> role:editor -> ...`, so
        // `role:owner` still satisfies `p, role:admin, canvas, write` and
        // `p, role:editor, canvas, write` after the owner row is gone. The
        // verdict correctly stayed `true`; the test's model of the policy
        // was what was wrong.
        let body = BASE_POLICY
            .lines()
            .filter(|l| {
                !l.starts_with("p, role:owner, canvas, write")
                    && !l.starts_with("p, role:admin, canvas, write")
                    && !l.starts_with("p, role:editor, canvas, write")
            })
            .collect::<Vec<_>>()
            .join("\n");
        // Sample the counter BEFORE touching the file. The watcher's notify
        // event can land within microseconds of the write, so a sample taken
        // afterwards can already include the very reload we are waiting for
        // and the poll would then never observe a delta. Sampling first
        // makes the causal window exactly "the write we are about to make".
        let before = hr.reload_count();
        // Preserve trailing newline so editors don't reject it.
        write_policy(&policy_path, &(body + "\n"));

        // Sanity: the mutation must actually change the file, otherwise the
        // whole test would pass for the wrong reason.
        let on_disk = std::fs::read_to_string(&policy_path).expect("reread policy csv");
        for role in ["role:owner", "role:admin", "role:editor"] {
            assert!(
                !on_disk.contains(&format!("p, {role}, canvas, write")),
                "mutation did not remove the {role}/canvas/write policy line"
            );
        }

        // Poll for the watcher to fire and rebuild the enforcer.
        //
        // We assert on `reload_count`, not on the enforcement verdict: the
        // hand-rolled evaluator derives grants from the m11 role matrix
        // and does not read the CSV at all, so a policy delta is
        // deliberately invisible through `enforce()` in that build. The
        // reload counter is evaluator-independent and is also the signal
        // the admin endpoint surfaces, so this genuinely tests the watcher.
        // The casbin-only verdict assertions follow below.
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
        // is compiled in; the hand-rolled evaluator ignores the policy CSV
        // by design, so there the delta is structurally unobservable (see
        // the note above).
        if CASBIN_EVALUATOR {
            use ada_m11_rbac_collab::ResourceType;

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
                "after watcher reload: no role in the ladder grants canvas/write, \
                 so role:owner must be denied"
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
}
