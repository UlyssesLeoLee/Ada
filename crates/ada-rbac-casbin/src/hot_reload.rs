//! Hot reload: holds an `Arc<RwLock<Enforcer>>` and exposes
//! `reload_now()` for the admin endpoint. v0.5.0 adds a real
//! `notify`-based file watcher via [`HotReload::spawn_watcher`] —
//! on `Modify` / `Create` / `Remove` events for the policy CSV the
//! watcher rebuilds the enforcer and atomically swaps the handle.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use notify::{event::ModifyKind, Event, EventKind, RecursiveMode, Watcher};
use parking_lot::RwLock;

use crate::enforcer::Enforcer;
use crate::error::{RbacCasbinError, Result};
use crate::policy::PolicySet;

pub struct HotReload {
    cell: Arc<RwLock<Enforcer>>,
    policy_path: PathBuf,
}

impl std::fmt::Debug for HotReload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HotReload")
            .field("policy_path", &self.policy_path)
            .finish_non_exhaustive()
    }
}

impl HotReload {
    /// Initialise with the bundled policy set. The model path is
    /// `crates/ada-rbac-casbin/policies/model.conf` relative to the
    /// workspace root.
    pub fn new(set: &PolicySet) -> Result<Self> {
        let enforcer = Enforcer::from_policy_set(set)?;
        Ok(Self {
            cell: Arc::new(RwLock::new(enforcer)),
            policy_path: set.policy_path.clone(),
        })
    }

    /// Snapshot of the current enforcer.
    #[must_use]
    pub fn current(&self) -> Enforcer {
        self.cell.read().clone()
    }

    /// Synchronous reload — rebuilds the enforcer from disk and swaps
    /// it into the cell.
    pub fn reload_now(&self) -> Result<()> {
        let set = PolicySet {
            model_path: self.model_path(),
            policy_path: self.policy_path.clone(),
            overlays: Vec::new(),
        };
        let next = Enforcer::from_policy_set(&set)?;
        let mut w = self.cell.write();
        *w = next;
        Ok(())
    }

    /// The model.conf path this watcher observes. Derived from
    /// `policy_path.parent()` so both files stay co-located; if
    /// the policy file has no parent we anchor to the crate manifest.
    fn model_path(&self) -> PathBuf {
        self.policy_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("policies"))
            .join("model.conf")
    }

    /// Spawn a `notify` watcher thread that triggers `reload_now()`
    /// on policy file mutations. Returns the active watcher (kept
    /// alive by the caller — when dropped the watcher stops).
    ///
    /// The watcher uses `notify::recommended_watcher` (cross-platform
    /// default backend) and filters to events touching
    /// `policy_path()`; on `Create`, `Modify(_)`, or `Remove` we call
    /// `reload_now()` so subsequent [`Self::current`] calls see the
    /// refreshed enforcer. The thread is daemon, so dropping the
    /// watcher cleanly stops it.
    pub fn spawn_watcher(&self) -> Result<ActiveWatcher> {
        let policy_path = self.policy_path.clone();
        let policy_for_filter = policy_path.clone();
        let policy_for_reload = self.policy_path.clone();
        let model_path = self.model_path();
        let cell = Arc::clone(&self.cell);

        let (tx, rx) = std::sync::mpsc::channel::<notify::Result<Event>>();
        let mut watcher = notify::recommended_watcher(move |res| {
            let _ = tx.send(res);
        })
        .map_err(|e| RbacCasbinError::ReloadFailed(format!("recommended_watcher: {e}")))?;

        // Watch the directory containing the policy CSV. Watching the
        // file directly can fail on editors that swap the inode
        // (atomic save); watching the parent covers both rename and
        // modify-in-place patterns.
        let watch_root = policy_path
            .parent()
            .ok_or_else(|| RbacCasbinError::ReloadFailed("policy_path has no parent".into()))?
            .to_path_buf();
        watcher
            .watch(&watch_root, RecursiveMode::NonRecursive)
            .map_err(|e| RbacCasbinError::ReloadFailed(format!("watch {watch_root:?}: {e}")))?;

        let handle = thread::Builder::new()
            .name("ada-rbac-casbin::hot_reload".into())
            .spawn(move || {
                while let Ok(res) = rx.recv() {
                    eprintln!("[hot_reload] raw event: {res:?}");
                    match res {
                        Ok(ev) if is_policy_event(&ev, &policy_for_filter) => {
                            eprintln!("[hot_reload] policy event matched; reloading");
                            // Debounce by sleeping briefly; editors that
                            // emit multiple events per save will
                            // collapse into a single reload.
                            thread::sleep(Duration::from_millis(50));
                            let set = PolicySet {
                                model_path: model_path.clone(),
                                policy_path: policy_for_reload.clone(),
                                overlays: Vec::new(),
                            };
                            match Enforcer::from_policy_set(&set) {
                                Ok(next) => {
                                    eprintln!("[hot_reload] reload_now ok");
                                    let mut w = cell.write();
                                    *w = next;
                                }
                                Err(e) => {
                                    eprintln!("[hot_reload] reload_now failed: {e}");
                                }
                            }
                        }
                        Ok(ev) => {
                            eprintln!(
                                "[hot_reload] ignored: {:?} paths={:?}",
                                ev.kind, ev.paths
                            );
                        }
                        Err(e) => {
                            eprintln!("[hot_reload] stream error: {e}");
                        }
                    }
                }
            })
            .map_err(|e| RbacCasbinError::ReloadFailed(format!("spawn watcher thread: {e}")))?;

        Ok(ActiveWatcher {
            _watcher: Box::new(watcher),
            _thread: Some(handle),
        })
    }

    /// The policy CSV path this watcher observes.
    #[must_use]
    pub fn policy_path(&self) -> &Path {
        &self.policy_path
    }
}

/// Decide whether a notify event should trigger a reload. We match on
/// `Create(_) | Modify(_) | Remove(_)` against the policy file path.
/// Path comparison uses canonicalization so Windows' `\\?\` UNC
/// prefix doesn't trip equality (and so on macOS symlinks resolve).
fn is_policy_event(ev: &Event, policy: &Path) -> bool {
    let kind = match ev.kind {
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => ev.kind,
        _ => return false,
    };
    let _ = kind;
    let _ = ModifyKind::Any;
    let policy_canon = policy.canonicalize().ok();
    ev.paths.iter().any(|p| {
        if p == policy {
            return true;
        }
        if let Some(ref target) = policy_canon {
            if let Ok(p_canon) = p.canonicalize() {
                if p_canon == *target {
                    return true;
                }
            }
        }
        if let Some(name) = p.file_name() {
            if let Some(target_name) = policy.file_name() {
                if name == target_name && p.parent() == policy.parent() {
                    return true;
                }
            }
        }
        false
    })
}

/// Owns the active `notify::Watcher` and its dispatch thread. When
/// this guard is dropped the watcher stops and the thread exits
/// (the channel closes when the watcher drops the sender).
pub struct ActiveWatcher {
    _watcher: Box<dyn Watcher + Send>,
    _thread: Option<thread::JoinHandle<()>>,
}

impl std::fmt::Debug for ActiveWatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ActiveWatcher").finish_non_exhaustive()
    }
}

impl Drop for ActiveWatcher {
    fn drop(&mut self) {
        // Dropping `_watcher` closes the channel sender; the
        // dispatch thread observes `rx.recv()` returning Err and
        // exits. Joining the thread would block shutdown; we let it
        // detach naturally.
        self._thread.take();
    }
}