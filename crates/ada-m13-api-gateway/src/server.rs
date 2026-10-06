//! Gateway server bootstrap.
//!
//! Kept in the library rather than in the binary so it is reachable
//! from tests and so the binary stays a thin shell, matching
//! `gm-console`'s layout.
//!
//! ## Why this exists at all
//!
//! The crate was library-only until now. `deploy/k8s/gm-console.yaml`
//! points `GM_CONSOLE_UPSTREAM` at `http://ada-api-gateway:8080`, but
//! no Service by that name existed anywhere in this repository, so every
//! `/api/*` request through the only deployed service resolved no DNS
//! record and returned 502. `gm-console`'s own doc comment claims
//! "auth, rate limit and observability live in api-gateway" — this
//! module is the process that comment was describing.

use std::{future::IntoFuture, net::SocketAddr, sync::Arc, time::Duration};

use tokio::signal;
use tower_http::trace::TraceLayer;

use crate::{
    error::{ApiError, Result},
    health::MemoryHealthCheck,
    login::{CredentialDirectory, LoginService},
    router::build_router,
    state::AppState,
};

/// Time we let in-flight requests complete after a shutdown signal.
///
/// Must stay below the k8s `terminationGracePeriodSeconds` in
/// `deploy/k8s/ada-api-gateway.yaml`, or the kubelet SIGKILLs the
/// process before the drain can finish and the shutdown reason is lost.
pub const SHUTDOWN_GRACE: Duration = Duration::from_secs(25);

/// Bind the gateway and serve until SIGINT / SIGTERM.
///
/// The drain is bounded by [`SHUTDOWN_GRACE`]: `with_graceful_shutdown`
/// on its own waits for every in-flight request with no upper limit, so
/// one hung connection would hold the process until the kubelet killed
/// it. The signal is awaited exactly once here and handed to axum
/// through a oneshot, which is what lets the timeout start when the
/// signal arrives rather than when the server does.
pub async fn serve(bind: SocketAddr, name: &str) -> Result<()> {
    // Loaded here rather than inside `AppState::new` so that no
    // constructor reads the environment, and so a malformed credential
    // set refuses to start instead of serving 401 to every real user
    // with nothing in the log to explain it. The value is never logged:
    // it holds passwords.
    let directory = CredentialDirectory::from_env()?;
    if directory.is_empty() {
        tracing::warn!(
            "no login credentials configured; POST /api/v1/auth/login will refuse every request"
        );
    }
    let state = AppState::new(name, Arc::new(MemoryHealthCheck::new()))?
        .with_login(Arc::new(LoginService::new(Arc::new(directory))));
    let app = build_router(state).layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .map_err(|e| ApiError::Internal(format!("bind {bind}: {e}")))?;
    // Log only the bind address; never any env-derived secret.
    tracing::info!(%bind, "api-gateway listening");

    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    let server = axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            let _ = rx.await;
            tracing::info!("shutdown signal received, draining in-flight requests");
        })
        // `with_graceful_shutdown` returns a builder that only becomes
        // a future on `.await`; `select!` and `timeout` both need the
        // future itself.
        .into_future();
    tokio::pin!(server);

    tokio::select! {
        result = &mut server => {
            result.map_err(|e| ApiError::Internal(format!("serve: {e}")))
        }
        () = shutdown_signal() => {
            let _ = tx.send(());
            match tokio::time::timeout(SHUTDOWN_GRACE, &mut server).await {
                Ok(Ok(())) => Ok(()),
                Ok(Err(e)) => Err(ApiError::Internal(format!("serve during drain: {e}"))),
                Err(_) => {
                    tracing::warn!(
                        grace_secs = SHUTDOWN_GRACE.as_secs(),
                        "drain did not finish within the grace window; exiting with requests in flight"
                    );
                    Ok(())
                }
            }
        }
    }
}

/// Resolve once on SIGINT (Ctrl-C) or SIGTERM (k8s preStop).
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
        () = ctrl_c => tracing::info!("SIGINT received"),
        () = terminate => tracing::info!("SIGTERM received"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shutdown_grace_fits_inside_the_k8s_termination_window() {
        // deploy/k8s/ada-api-gateway.yaml sets
        // terminationGracePeriodSeconds: 30. The drain must finish
        // before the kubelet SIGKILLs the process, or the reason it
        // exited is never logged.
        assert!(SHUTDOWN_GRACE <= Duration::from_secs(29));
    }
}
