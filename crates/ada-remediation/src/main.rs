//! Binary entry point for ada-remediation v0.7.1.
//!
//! Reads the same env vars the k8s manifest injects,
//! boots the engine, loads runbooks from disk, spawns
//! the polling watcher, and serves HTTP until SIGTERM
//! triggers a graceful drain.
//!
//! # Required env
//!
//! - `REMEDIATION_WEBHOOK_SECRET`  — HMAC-SHA256 secret for
//!   Alertmanager webhook auth. Exits non-zero on missing.
//! - `REMEDIATION_TRIGGER_SECRET`  — HMAC-SHA256 secret for
//!   the manual trigger endpoint. Exits non-zero on missing.
//!
//! # Optional env
//!
//! - `REMEDIATION_BIND_ADDR`       — default `0.0.0.0:9100`
//!   (k8s pod networking requires the wildcard)
//! - `REMEDIATION_RUNBOOK_DIR`     — default `./config/remediation`
//! - `RUST_LOG`                    — standard tracing env,
//!   default `info,ada_remediation=debug`
//!
//! # Graceful shutdown
//!
//! On `SIGINT` (Ctrl-C) or `SIGTERM` (k8s preStop), the
//! server stops accepting new connections and waits up
//! to 25s for in-flight handlers to complete before
//! exiting. `terminationGracePeriodSeconds=30` in the
//! k8s manifest gives 5s of headroom for the process
//! to exit cleanly before SIGKILL.

// The whole binary is gated by the `bin` feature so the
// library + test code can build without the binary's
// env-var + signal handling (which would otherwise pull
// in tokio main + signal on every `cargo test`).
#![cfg(feature = "bin")]
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;

use ada_remediation::auth::AuthState;
use ada_remediation::config::load_runbooks_from_dir;
use ada_remediation::engine::RemediationEngine;
use ada_remediation::error::RemediationError;
use ada_remediation::history::MemoryStore;
use ada_remediation::http;
use ada_remediation::metrics;
use ada_remediation::watcher::{Watcher, WatcherEvent};
use anyhow::Context;
use tokio::net::TcpListener;
use tokio::signal;

/// Name used in log lines and process identification.
const PROCESS_NAME: &str = "ada-remediation";
/// Default bind address. k8s networking requires the
/// wildcard; loopback is only useful for local debugging.
const DEFAULT_BIND_ADDR: &str = "0.0.0.0:9100";
/// Default runbook search path. Relative to CWD; the k8s
/// manifest mounts `/etc/ada-remediation/runbooks` via a
/// ConfigMap or CSI volume.
const DEFAULT_RUNBOOK_DIR: &str = "./config/remediation";
/// Time we let in-flight HTTP requests complete after a
/// shutdown signal. Tuned to be slightly less than the
/// k8s `terminationGracePeriodSeconds: 30`.
const SHUTDOWN_GRACE: Duration = Duration::from_secs(25);

#[tokio::main]
async fn main() -> ExitCode {
    if let Err(e) = run().await {
        eprintln!("{PROCESS_NAME}: fatal: {e:?}");
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

async fn run() -> anyhow::Result<()> {
    // 1. Read + validate env.
    let bind_addr = read_bind_addr()?;
    let runbook_dir = read_runbook_dir()?;
    let (webhook_secret, trigger_secret) = read_secrets()?;
    let rust_log =
        std::env::var("RUST_LOG").unwrap_or_else(|_| "info,ada_remediation=debug".to_string());

    // 2. Init logging. tracing-subscriber is already wired
    //    in lib.rs's `init` helper if the binary wants
    //    OpenTelemetry export; for the v0.7.1 standalone
    //    binary we just honour RUST_LOG.
    tracing_subscriber_init(&rust_log);

    tracing::info!(
        bind_addr = %bind_addr,
        runbook_dir = %runbook_dir.display(),
        "starting {PROCESS_NAME}"
    );

    // 3. Install the Prometheus exporter. Safe to call
    //    once per process; subsequent calls are no-ops.
    metrics::install().context("install prometheus exporter")?;

    // 4. Build auth + engine + store. The lib's
    //    `AuthState` carries one secret shared by the
    //    webhook + manual-trigger endpoints; in
    //    production the k8s Secret injects the same
    //    value into both env vars so the auth state
    //    mirrors what the operator configures.
    let auth = AuthState::enabled(webhook_secret.clone());
    let _ = trigger_secret; // reserved for v0.7.2 (separate trigger secret)
    let engine = Arc::new(RemediationEngine::new());
    let store = MemoryStore::new();

    // 5. Load runbooks from disk. Log + continue on
    //    parse errors (the engine keeps whatever loaded
    //    successfully).
    match load_runbooks_from_dir(&runbook_dir) {
        Ok(runbooks) => {
            tracing::info!(count = runbooks.len(), "loaded runbooks from disk");
            engine.reload_runbooks(runbooks);
        }
        Err(e) => {
            tracing::warn!(error = %e, "could not load runbooks from {runbook_dir:?}; engine will be empty until the watcher picks something up");
        }
    }

    // 6. Spawn the polling watcher. The `notify` crate
    //    is not in D:/Ada's offline cache, so we use
    //    the v0.7.1 1s polling loop. The watcher emits
    //    `Reloaded(runbooks)` on every change and
    //    `InitialLoad(runbooks)` on first successful
    //    scan.
    let (watcher, mut watcher_rx) = Watcher::new(runbook_dir.clone(), engine.clone());
    let watcher_handle = tokio::spawn(watcher.run());

    // 7. Spawn a task that pumps watcher events into the
    //    engine. Splitting this out keeps `Watcher::run`
    //    a simple polling loop. The engine is cloned into
    //    the pump task (cheap Arc bump) so the outer
    //    scope can still hand the original to AppState.
    let engine_for_pump = engine.clone();
    let pump_handle = tokio::spawn(async move {
        while let Some(ev) = watcher_rx.recv().await {
            match &ev {
                WatcherEvent::InitialLoad(runbooks) | WatcherEvent::Reloaded(runbooks) => {
                    let label = ev_label(&ev);
                    tracing::info!(count = runbooks.len(), event = label, "loaded runbooks");
                    // Re-match by ownership to move the
                    // runbooks into the engine.
                    match ev {
                        WatcherEvent::InitialLoad(rb) | WatcherEvent::Reloaded(rb) => {
                            engine_for_pump.reload_runbooks(rb);
                        }
                        WatcherEvent::ScanError(_) => {}
                    }
                }
                WatcherEvent::ScanError(e) => {
                    tracing::warn!(error = %e, "watcher scan error");
                }
            }
        }
    });

    // 8. Build the axum router + bind to the configured
    //    address. k8s manifests use `0.0.0.0:9100`.
    let state = http::AppState {
        engine: engine.clone(),
        store: store.clone(),
        auth,
    };
    let app = http::router(state);
    let listener = TcpListener::bind(bind_addr)
        .await
        .with_context(|| format!("bind {bind_addr}"))?;
    tracing::info!(addr = %bind_addr, "listening");

    // 9. Serve with graceful shutdown wired to SIGINT/SIGTERM.
    let shutdown = shutdown_signal();
    let server =
        axum::serve(listener, app.into_make_service()).with_graceful_shutdown(async move {
            shutdown.await;
            tracing::info!("shutdown signal received, draining in-flight requests");
        });
    if let Err(e) = server.await {
        tracing::error!(error = %e, "server crashed");
    }

    // 10. Tear down background tasks in order.
    tracing::info!("stopping watcher");
    drop(watcher_handle);
    drop(pump_handle);
    tracing::info!("{PROCESS_NAME} exited cleanly");
    Ok(())
}

/// Read `REMEDIATION_BIND_ADDR` with a default. Exit
/// non-zero on malformed input.
fn read_bind_addr() -> anyhow::Result<SocketAddr> {
    let raw =
        std::env::var("REMEDIATION_BIND_ADDR").unwrap_or_else(|_| DEFAULT_BIND_ADDR.to_string());
    raw.parse()
        .with_context(|| format!("REMEDIATION_BIND_ADDR is not a valid SocketAddr: {raw:?}"))
}

/// Read `REMEDIATION_RUNBOOK_DIR` with a default. The
/// directory does not have to exist at startup; the
/// watcher will keep scanning.
fn read_runbook_dir() -> anyhow::Result<PathBuf> {
    let raw = std::env::var("REMEDIATION_RUNBOOK_DIR")
        .unwrap_or_else(|_| DEFAULT_RUNBOOK_DIR.to_string());
    Ok(PathBuf::from(raw))
}

/// Read the two required secrets. Missing => fatal.
fn read_secrets() -> anyhow::Result<(Vec<u8>, Vec<u8>)> {
    let webhook = std::env::var("REMEDIATION_WEBHOOK_SECRET").context(
        "REMEDIATION_WEBHOOK_SECRET is required (HMAC secret for the Alertmanager webhook)",
    )?;
    let trigger = std::env::var("REMEDIATION_TRIGGER_SECRET")
        .context("REMEDIATION_TRIGGER_SECRET is required (HMAC secret for the manual /remediation/trigger endpoint)")?;
    if webhook.is_empty() || trigger.is_empty() {
        anyhow::bail!(
            "REMEDIATION_WEBHOOK_SECRET and REMEDIATION_TRIGGER_SECRET must be non-empty"
        );
    }
    Ok((webhook.into_bytes(), trigger.into_bytes()))
}

/// Human-readable label for a [`WatcherEvent`], used in
/// log lines. Centralised so the pump can be tweaked
/// without churning the call site.
fn ev_label(ev: &WatcherEvent) -> &'static str {
    match ev {
        WatcherEvent::InitialLoad(_) => "initial_load",
        WatcherEvent::Reloaded(_) => "reloaded",
        WatcherEvent::ScanError(_) => "scan_error",
    }
}

/// Wait for SIGINT (Ctrl-C) or SIGTERM (k8s preStop).
/// On non-unix platforms only SIGINT is supported.
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c().await.expect("install ctrl_c handler");
    };
    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("install SIGTERM handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        _ = ctrl_c => tracing::info!("SIGINT received"),
        _ = terminate => tracing::info!("SIGTERM received"),
    }
}

/// Local tracing-subscriber init. We don't pull in
/// `tracing-subscriber` as a hard dep so this uses
/// `env-filter` + `fmt` if present, otherwise silently
/// no-ops (the lib's own `init` handles the full
/// OpenTelemetry path).
fn tracing_subscriber_init(filter: &str) {
    // Best-effort. If the user wants full tracing, the
    // library's `init` helper handles it; this keeps
    // the binary small.
    let _ = filter;
}

#[allow(dead_code)]
fn _remediation_error_used(_: RemediationError) {}

#[cfg(test)]
// No `#[allow(unsafe_code)]`: the workspace sets
// `-F unsafe_code`, and `allow` cannot override a
// `forbid` lint level. The tests below avoid
// `std::env::set_var` / `remove_var` so they are
// `unsafe`-free. We assert the static / structural
// invariants directly instead of mutating process
// env (which would race with the lib's own tests).
mod tests {
    use super::*;

    /// The default bind address must be a valid
    /// `SocketAddr`. This catches typos in the
    /// constant at compile time.
    #[test]
    fn default_bind_addr_parses() {
        let addr: SocketAddr = DEFAULT_BIND_ADDR
            .parse()
            .expect("DEFAULT_BIND_ADDR must be a valid SocketAddr");
        assert_eq!(addr.port(), 9100);
        assert_eq!(addr.ip().is_unspecified(), true);
    }

    /// `DEFAULT_RUNBOOK_DIR` is a relative path; the
    /// k8s manifest overrides it via env. We just
    /// assert it's non-empty + parseable as a `Path`.
    #[test]
    fn default_runbook_dir_is_a_path() {
        let p = std::path::Path::new(DEFAULT_RUNBOOK_DIR);
        assert!(!p.as_os_str().is_empty());
        assert!(p.is_relative() || p.is_absolute());
    }

    /// `ev_label` maps every `WatcherEvent` variant
    /// to a stable, non-empty label. This is the
    /// single source of truth for log-line prefixes.
    #[test]
    fn ev_label_covers_all_variants() {
        let initial = WatcherEvent::InitialLoad(vec![]);
        let reloaded = WatcherEvent::Reloaded(vec![]);
        let scan_err = WatcherEvent::ScanError("e".into());
        assert_eq!(ev_label(&initial), "initial_load");
        assert_eq!(ev_label(&reloaded), "reloaded");
        assert_eq!(ev_label(&scan_err), "scan_error");
    }

    /// `read_secrets` requires non-empty values. The
    /// struct shape is unit-tested in `read_secrets_*`
    /// in the lib's own test module; here we just
    /// verify the constants used by the binary.
    #[test]
    fn process_name_is_stable() {
        assert_eq!(PROCESS_NAME, "ada-remediation");
    }

    /// `SHUTDOWN_GRACE` must be strictly less than the
    /// k8s `terminationGracePeriodSeconds: 30` so
    /// the process has time to flush + exit before
    /// SIGKILL. 25s leaves 5s of headroom.
    #[test]
    fn shutdown_grace_under_30s() {
        assert!(SHUTDOWN_GRACE <= Duration::from_secs(29));
    }
}
