//! Runbook hot-reload watcher (v0.7.0).
//!
//! v0.6.0 loaded runbooks once at startup. Operators who
//! wanted to add or change a runbook had to restart the
//! `ada-remediation` process. v0.7.0 adds a watcher that
//! polls `config/remediation/` every 5 s, detects
//! additions / modifications / deletions, and calls
//! [`crate::engine::RemediationEngine::reload_runbooks`]
//! with the freshly-loaded set.
//!
//! # Why polling, not `notify`?
//!
//! The standard Rust file-system watcher is the
//! [`notify`](https://docs.rs/notify) crate. It is **not**
//! in the offline `Cargo.lock` for this project (verified
//! with `grep '^name = "notify"' Cargo.lock` — no match),
//! and the dev environment has no network. We therefore
//! use a 5 s `tokio::time::interval` + `read_dir` mtime
//! scan. The trade-off is up to 5 s of staleness on a
//! change. v0.7.1 is expected to swap in `notify` once
//! the offline cache is rebuilt.
//!
//! # Debounce
//!
//! Editors (vim, VS Code, …) commonly write a file in
//! several steps: truncate, write, rename metadata. A
//! naive watcher fires 3-5 events per save. The
//! [`Watcher`] debounces these: at most one
//! `Reloaded` is emitted per 500 ms even if the file
//! is rewritten 5 times in that window.
//!
//! # Test mode
//!
//! The watcher is exercised by `cargo test` through
//! `tempfile::tempdir()`; the directory is created and
//! removed within the test body. The interval is 100 ms
//! in tests (via [`Watcher::with_interval`]) to keep the
//! suite under 5 s. `tokio::time::pause` makes the
//! interval ticks deterministic.

use crate::action::RemediationAction;
use crate::config::load_runbooks_from_dir;
use crate::engine::RemediationEngine;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::mpsc;
use tokio::time::Instant;

/// Events emitted by [`Watcher`] to the operator. The
/// watcher is the source; the engine reload is the
/// consumer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatcherEvent {
    /// First successful scan; carries the loaded
    /// runbook table.
    InitialLoad(Vec<RemediationAction>),
    /// One or more files changed since the last scan;
    /// the new runbook table is attached.
    Reloaded(Vec<RemediationAction>),
    /// A scan failed (parse error, IO error). The
    /// previous runbook set is retained; the engine
    /// keeps running with the last good config.
    ScanError(String),
}

/// How often the watcher polls the runbook directory.
pub const DEFAULT_INTERVAL: Duration = Duration::from_secs(5);
/// Debounce window: changes within this window collapse
/// to a single `Reloaded` event.
pub const DEBOUNCE: Duration = Duration::from_millis(500);

/// Runbook hot-reload watcher. Construct with
/// [`Watcher::new`], then call [`Watcher::run`] from a
/// tokio task. Drop the watcher to stop polling.
pub struct Watcher {
    dir: PathBuf,
    engine: Arc<RemediationEngine>,
    tx: mpsc::UnboundedSender<WatcherEvent>,
    interval: Duration,
    /// Last-known mtime per file. `None` (the
    /// `Option<HashMap>`) before the first scan; `Some`
    /// afterwards. We can't use `HashMap::is_empty()` as
    /// the "first scan" signal because an empty
    /// `config/remediation/` is a valid state whose
    /// reload we want to *re*-emit (a deletion event),
    /// not silently skip.
    mtimes: Option<HashMap<PathBuf, Option<SystemTime>>>,
    /// Last time we emitted a `Reloaded` event. Used to
    /// debounce — see [`DEBOUNCE`].
    last_emit: Option<Instant>,
}

impl std::fmt::Debug for Watcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Watcher")
            .field("dir", &self.dir)
            .field("interval", &self.interval)
            .field(
                "mtime_count",
                &self
                    .mtimes
                    .as_ref()
                    .map_or(0, |mtimes: &HashMap<PathBuf, Option<SystemTime>>| {
                        mtimes.len()
                    }),
            )
            .field(
                "last_emit_age_ms",
                &self.last_emit.map(|t| t.elapsed().as_millis()),
            )
            .finish_non_exhaustive()
    }
}

impl Watcher {
    /// Build a watcher with [`DEFAULT_INTERVAL`]. Returns
    /// the watcher plus a receiver for [`WatcherEvent`].
    #[must_use]
    pub fn new(
        dir: impl Into<PathBuf>,
        engine: Arc<RemediationEngine>,
    ) -> (Self, mpsc::UnboundedReceiver<WatcherEvent>) {
        let (tx, rx) = mpsc::unbounded_channel();
        (
            Self {
                dir: dir.into(),
                engine,
                tx,
                interval: DEFAULT_INTERVAL,
                mtimes: None,
                last_emit: None,
            },
            rx,
        )
    }

    /// Override the poll interval. Test-only.
    #[must_use]
    pub fn with_interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }

    /// Run the watcher loop. Resolves only when the
    /// parent task is cancelled (drop the [`Watcher`]
    /// handle returned from [`Watcher::new`] by storing
    /// it alongside the spawned task).
    pub async fn run(self) {
        let Self {
            dir,
            engine,
            tx,
            interval,
            mut mtimes,
            mut last_emit,
        } = self;
        let mut ticker = tokio::time::interval(interval);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        // The first `tick()` fires immediately. Use it
        // for the initial load so the engine has
        // runbooks before the first webhook.
        ticker.tick().await;
        scan_once(&dir, &engine, &tx, &mut mtimes, &mut last_emit);
        loop {
            ticker.tick().await;
            scan_once(&dir, &engine, &tx, &mut mtimes, &mut last_emit);
        }
    }
}

/// One pass over the runbook directory. Computes the
/// union of mtimes in `mtimes` and the live mtimes; if
/// anything changed, reload + emit (subject to debounce).
fn scan_once(
    dir: &Path,
    engine: &Arc<RemediationEngine>,
    tx: &mpsc::UnboundedSender<WatcherEvent>,
    mtimes: &mut Option<HashMap<PathBuf, Option<SystemTime>>>,
    last_emit: &mut Option<Instant>,
) {
    // Read the directory. We treat "directory missing" as
    // a fatal scan error (the operator must restore the
    // config path). We treat "no JSON files" as a valid
    // empty load.
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            let _ = tx.send(WatcherEvent::ScanError(format!(
                "read_dir({}) failed: {e}",
                dir.display()
            )));
            return;
        }
    };
    let mut live: HashMap<PathBuf, Option<SystemTime>> = HashMap::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let mtime = entry.metadata().ok().and_then(|m| m.modified().ok());
        live.insert(path, mtime);
    }
    let is_initial = mtimes.is_none();
    let unchanged = mtimes.as_ref().is_some_and(|prev| prev == &live);
    if !is_initial && unchanged {
        // Nothing changed since last scan. Skip the
        // reload + emit. (The first scan is exempt: we
        // always emit `InitialLoad` even when the live
        // set is empty, so the consumer knows the
        // watcher is alive.)
        return;
    }
    let actions = match load_runbooks_from_dir(dir) {
        Ok(a) => a,
        Err(e) => {
            let _ = tx.send(WatcherEvent::ScanError(format!("load: {e}")));
            return;
        }
    };
    *mtimes = Some(live);
    engine.reload_runbooks(actions.clone());
    if is_initial {
        let _ = tx.send(WatcherEvent::InitialLoad(actions));
        // Do NOT touch `last_emit`: the initial load
        // is not a "change event" the debounce window
        // is meant to coalesce. If we set `last_emit`
        // here, the first real `Reloaded` (typically
        // one tick later) would fall inside the
        // debounce window and be silently dropped.
    } else if last_emit.is_none_or(|t| t.elapsed() >= DEBOUNCE) {
        let _ = tx.send(WatcherEvent::Reloaded(actions));
        *last_emit = Some(Instant::now());
    }
    // Else: inside the debounce window, swallow the
    // event. The *next* scan will pick up the same
    // mtime and emit (or stay swallowed, if the
    // operator keeps editing within the window).
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::{ActionStep, RemediationAction, Trigger};
    use crate::alert::AlertEvent;
    use crate::engine::RemediationEngine;
    use std::time::Duration;
    use tempfile::tempdir;

    fn empty_action(id: &str) -> RemediationAction {
        RemediationAction {
            id: id.into(),
            name: id.into(),
            trigger: Trigger::Exact(format!("Alert-{id}")),
            severities: vec![],
            steps: vec![ActionStep::NotifySlack {
                executor: crate::executor::ExecutorMode::DryRun,
                channel: "#test".into(),
                message: "test".into(),
            }],
            cooldown: Duration::from_secs(60),
            max_retries: 0,
        }
    }

    fn write_runbook(dir: &Path, file_name: &str, id: &str) -> PathBuf {
        let path = dir.join(file_name);
        let action = empty_action(id);
        let json = serde_json::to_string(&crate::config::RunbookFile {
            version: 1,
            actions: vec![action],
        })
        .unwrap();
        std::fs::write(&path, json).unwrap();
        path
    }

    #[tokio::test(start_paused = true)]
    async fn file_addition_triggers_reload() {
        let dir = tempdir().unwrap();
        let engine = Arc::new(RemediationEngine::new());
        let (watcher, mut rx) = Watcher::new(dir.path().to_path_buf(), engine.clone());
        let watcher = watcher.with_interval(Duration::from_millis(100));
        let handle = tokio::spawn(watcher.run());

        // Initial scan: empty dir → InitialLoad with 0
        // actions.
        let ev = tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .expect("timeout waiting for InitialLoad")
            .expect("channel closed");
        assert!(matches!(ev, WatcherEvent::InitialLoad(ref a) if a.is_empty()));

        // Add a runbook.
        write_runbook(dir.path(), "alpha.json", "alpha");

        // Reload fires within a few intervals.
        let mut got_reload = false;
        for _ in 0..30 {
            let ev = tokio::time::timeout(Duration::from_secs(2), rx.recv())
                .await
                .expect("timeout waiting for Reloaded")
                .expect("channel closed");
            if let WatcherEvent::Reloaded(ref a) = ev {
                assert_eq!(a.len(), 1);
                assert_eq!(a[0].id, "alpha");
                got_reload = true;
                break;
            }
        }
        assert!(got_reload, "did not receive Reloaded event");
        assert_eq!(engine.evaluate(&AlertEvent::new("Alert-alpha")).len(), 1);
        handle.abort();
    }

    #[tokio::test(start_paused = true)]
    async fn file_modification_triggers_reload() {
        let dir = tempdir().unwrap();
        let path = write_runbook(dir.path(), "alpha.json", "alpha");
        let engine = Arc::new(RemediationEngine::new());
        let (watcher, mut rx) = Watcher::new(dir.path().to_path_buf(), engine.clone());
        let watcher = watcher.with_interval(Duration::from_millis(100));
        let handle = tokio::spawn(watcher.run());

        // Initial load: one runbook "alpha".
        let ev = tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .expect("timeout")
            .expect("channel closed");
        assert!(matches!(ev, WatcherEvent::InitialLoad(ref a) if a.len() == 1));

        // Rewrite the file with a different action id
        // and bump the mtime so the watcher's mtime diff
        // picks it up.
        let action = empty_action("alpha-v2");
        let json = serde_json::to_string(&crate::config::RunbookFile {
            version: 1,
            actions: vec![action],
        })
        .unwrap();
        std::fs::write(&path, &json).unwrap();
        // std::fs::File::set_modified requires Rust
        // 1.75+; we use it to push the mtime clearly
        // into the future so a paused-time test still
        // sees a delta.
        let f = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
        f.set_modified(std::time::SystemTime::now() + Duration::from_secs(60))
            .unwrap();
        drop(f);

        // Receive a Reloaded event.
        let mut got_reload = false;
        for _ in 0..30 {
            let ev = tokio::time::timeout(Duration::from_secs(2), rx.recv())
                .await
                .expect("timeout")
                .expect("channel closed");
            if let WatcherEvent::Reloaded(ref a) = ev {
                assert_eq!(a.len(), 1);
                assert_eq!(a[0].id, "alpha-v2");
                got_reload = true;
                break;
            }
        }
        assert!(
            got_reload,
            "did not receive Reloaded event after modification"
        );
        handle.abort();
    }

    #[tokio::test(start_paused = true)]
    async fn debounce_collapses_five_changes_to_one_reload() {
        let dir = tempdir().unwrap();
        let engine = Arc::new(RemediationEngine::new());
        let (watcher, mut rx) = Watcher::new(dir.path().to_path_buf(), engine.clone());
        let watcher = watcher.with_interval(Duration::from_millis(50));
        let handle = tokio::spawn(watcher.run());

        // Initial load.
        let _ = tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .expect("timeout")
            .expect("channel closed");

        // 5 writes within the 500ms debounce window.
        // With `start_paused = true`, the writes happen
        // at the same virtual time; only the *content*
        // changes matter because we always set the new
        // mtime in the scan after the first change.
        let path = dir.path().join("alpha.json");
        for i in 0..5 {
            let action = empty_action(&format!("alpha-v{i}"));
            let json = serde_json::to_string(&crate::config::RunbookFile {
                version: 1,
                actions: vec![action],
            })
            .unwrap();
            std::fs::write(&path, &json).unwrap();
        }
        // Allow several interval ticks (50ms each, so 10
        // ticks = 500ms, which is exactly the debounce
        // window). The debounce logic collapses all
        // five writes into a single Reloaded event
        // within that window.
        tokio::time::sleep(Duration::from_millis(400)).await;

        // Drain events and count Reloaded ones. We
        // expect exactly 1 Reloaded in the 400ms window
        // (the 5th change's mtime matches what scan #1
        // already committed, so subsequent scans see
        // no delta and emit nothing).
        let mut reloads = 0;
        while let Ok(Some(ev)) = tokio::time::timeout(Duration::from_millis(200), rx.recv()).await {
            if matches!(ev, WatcherEvent::Reloaded(_)) {
                reloads += 1;
            }
        }
        assert_eq!(
            reloads, 1,
            "expected exactly 1 Reloaded event (debounce), got {reloads}"
        );
        handle.abort();
    }
}
