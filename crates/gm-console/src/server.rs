//! gm-console server bootstrap.

use std::{net::SocketAddr, sync::Arc};
use tower_http::{compression::CompressionLayer, cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::{config::Config, error::Result, routes};

pub async fn serve(cfg: Config) -> Result<()> {
    init_tracing(&cfg.log_level);

    let state = Arc::new(cfg.clone());
    let bind: SocketAddr = cfg
        .bind_addr
        .parse()
        .map_err(|e| crate::Error::Config(format!("invalid bind_addr {}: {}", cfg.bind_addr, e)))?;

    let cors = build_cors(&cfg.allowed_origins);

    let app = routes::router(state)
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new())
        .layer(cors);

    // We deliberately log only the bind address; never the upstream URL or
    // env-derived secrets.
    tracing::info!(%bind, "gm-console starting");
    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .map_err(crate::Error::Io)?;
    axum::serve(listener, app)
        .await
        .map_err(crate::Error::Io)?;
    Ok(())
}

/// Build a tightened CORS layer from the configured allowed origins.
///
/// Default allow-list is `https://gm-console.kanvas.dev, https://localhost:3000`.
/// Methods/headers are minimal; credentials are NOT enabled (the SPA does not
/// rely on cookie auth — the api-gateway handles token auth).
fn build_cors(allowed_origins: &[String]) -> CorsLayer {
    let mut layer = CorsLayer::new()
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
        .max_age(std::time::Duration::from_secs(600));

    for origin in allowed_origins {
        match origin.parse::<axum::http::HeaderValue>() {
            Ok(v) => layer = layer.allow_origin(v),
            Err(_) => {
                tracing::warn!(
                    target: "gm-console::cors",
                    len = origin.len(),
                    "ignoring unparseable origin (value redacted)"
                );
            }
        }
    }
    layer
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