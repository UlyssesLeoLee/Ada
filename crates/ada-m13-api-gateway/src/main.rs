//! `ada-api-gateway` process entry point.
//!
//! A thin shell: read configuration from the environment, initialise
//! tracing, hand off to [`ada_m13_api_gateway::server::serve`].
//!
//! ## Why this binary has to exist
//!
//! `gm-console` is the only service in `deploy/k8s/` that had a
//! manifest, and it proxies `/api/*` to
//! `http://ada-api-gateway:8080` -- a name that, until this binary and
//! `deploy/k8s/ada-api-gateway.yaml` were added, resolved to no Service
//! in the repository at all. Every API call through the deployed product
//! returned 502. `gm-console`'s doc comment asserts that "auth, rate
//! limit and observability live in api-gateway"; this is that gateway.

use std::{net::SocketAddr, process::ExitCode};

/// Service name reported by `GET /health`.
const SERVICE_NAME: &str = "ada-api-gateway";

/// Default bind. Wildcard, because k8s routing reaches the pod IP.
const DEFAULT_BIND: &str = "0.0.0.0:8080";

#[tokio::main]
async fn main() -> ExitCode {
    // Must be initialised before `serve` so its first log lines are not
    // dropped, and must not fail hard: a missing subscriber only costs
    // observability, not correctness.
    init_tracing();

    let raw_bind = std::env::var("ADA_GATEWAY_BIND").unwrap_or_else(|_| DEFAULT_BIND.to_owned());
    let bind: SocketAddr = match raw_bind.parse() {
        Ok(addr) => addr,
        Err(e) => {
            eprintln!("{SERVICE_NAME}: invalid ADA_GATEWAY_BIND: {e}");
            return ExitCode::from(2);
        }
    };

    match ada_m13_api_gateway::server::serve(bind, SERVICE_NAME).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{SERVICE_NAME}: fatal: {e}");
            ExitCode::from(1)
        }
    }
}

/// Initialise `RUST_LOG`-draced output.
///
/// `tracing_subscriber` is a dev-time convenience here rather than a
/// hard dependency: if it is unavailable the gateway still serves, it
/// just does not log. That trade is deliberate — a logger must not be
/// the reason the API is down.
fn init_tracing() {
    let filter =
        std::env::var("RUST_LOG").unwrap_or_else(|_| "info,ada_m13_api_gateway=debug".to_owned());
    let Ok(filter) = filter.parse::<tracing_subscriber::EnvFilter>() else {
        return;
    };
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .try_init();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_bind_matches_what_deploy_publishes() {
        // gm-console.yaml points at http://ada-api-gateway:8080 and
        // ada-api-gateway.yaml publishes containerPort 8080. If this
        // drifts, the proxy gets a connection refused rather than a
        // 502, which is a much harder failure to diagnose from the
        // outside.
        assert_eq!(DEFAULT_BIND, "0.0.0.0:8080");
    }

    #[test]
    fn the_default_bind_parses() {
        assert!(DEFAULT_BIND.parse::<SocketAddr>().is_ok());
    }

    #[test]
    fn the_service_name_is_stable() {
        // Reported by GET /health and used as the k8s Service name.
        assert_eq!(SERVICE_NAME, "ada-api-gateway");
    }
}
