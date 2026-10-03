//! gm-console server bootstrap.

use std::{net::SocketAddr, sync::Arc};
use axum::Router;
use tower_http::{
    compression::CompressionLayer,
    cors::{AllowOrigin, CorsLayer},
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::{config::Config, error::Result, routes};

/// Build the application that [`serve`] actually binds.
///
/// Exists so tests exercise the *same* layer stack production does. The
/// CORS layer used to be applied inline inside `serve`, and the
/// integration tests built a bare `routes::router(..)` instead — so no
/// test ever saw a CORS header, which is how the allow-list bug below
/// survived. A gate that builds its own app can pass while the shipped
/// one is broken; there is now only one builder.
pub fn app(state: Arc<Config>) -> Router {
    let cors = build_cors(&state.allowed_origins);
    routes::router(state)
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new())
        .layer(cors)
}

pub async fn serve(cfg: Config) -> Result<()> {
    init_tracing(&cfg.log_level);

    let state = Arc::new(cfg.clone());
    let bind: SocketAddr = cfg
        .bind_addr
        .parse()
        .map_err(|e| crate::Error::Config(format!("invalid bind_addr {}: {}", cfg.bind_addr, e)))?;

    let app = app(state);

    // We deliberately log only the bind address; never the upstream URL or
    // env-derived secrets.
    tracing::info!(%bind, "gm-console starting");
    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .map_err(crate::Error::Io)?;
    axum::serve(listener, app).await.map_err(crate::Error::Io)?;
    Ok(())
}

/// Build a tightened CORS layer from the configured allowed origins.
///
/// Default allow-list is `https://gm-console.kanvas.dev, https://localhost:3000`.
/// Methods/headers are minimal; credentials are NOT enabled (the SPA does not
/// rely on cookie auth — the api-gateway handles token auth).
fn build_cors(allowed_origins: &[String]) -> CorsLayer {
    let mut parsed: Vec<axum::http::HeaderValue> = Vec::new();
    for origin in allowed_origins {
        match origin.parse::<axum::http::HeaderValue>() {
            Ok(v) => parsed.push(v),
            Err(_) => {
                tracing::warn!(
                    target: "gm-console::cors",
                    len = origin.len(),
                    "ignoring unparseable origin (value redacted)"
                );
            }
        }
    }

    // `CorsLayer::allow_origin` OVERWRITES. tower-http 0.6 documents "multiple
    // calls to this method will override any previous calls", and the body is
    // a plain `self.allow_origin = origin.into()`. Calling it once per origin
    // therefore left only the LAST origin in effect: with the shipped default
    // list the console served `access-control-allow-origin:
    // https://localhost:3000` and never `https://gm-console.kanvas.dev`, so
    // the production console could not call its own API, and a request from
    // any other origin was handed that same fixed header instead of none.
    // The list has to be handed over in a single call.
    //
    // `AllowOrigin::list` panics on a `*` entry, so a configured wildcard is
    // routed to `AllowOrigin::any()` with a warning rather than panicking the
    // server at startup. An empty list stays empty, which is the fail-closed
    // default `CorsLayer` already had.
    let allow_origin = if parsed.iter().any(|o| o == "*") {
        tracing::warn!(
            target: "gm-console::cors",
            "wildcard origin configured: every site will be able to read API responses"
        );
        AllowOrigin::any()
    } else {
        AllowOrigin::list(parsed)
    };

    CorsLayer::new()
        .allow_origin(allow_origin)
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PUT,
            axum::http::Method::PATCH,
            axum::http::Method::DELETE,
            axum::http::Method::OPTIONS,
        ])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
            axum::http::header::ACCEPT,
            axum::http::header::HeaderName::from_static("x-request-id"),
            axum::http::header::HeaderName::from_static("x-tenant-id"),
            axum::http::header::HeaderName::from_static("x-correlation-id"),
            axum::http::header::HeaderName::from_static("x-trace-id"),
        ])
        .max_age(std::time::Duration::from_secs(600))
}

fn init_tracing(level: &str) {
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(level))
        .unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().compact())
        .init();
}
