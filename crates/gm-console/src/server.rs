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

    let app = routes::router(state)
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new())
        .layer(CorsLayer::permissive()); // permissively tuned — tighten in worker-A.

    tracing::info!(%bind, upstream = %cfg.upstream_url, "gm-console starting");
    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .map_err(crate::Error::Io)?;
    axum::serve(listener, app)
        .await
        .map_err(crate::Error::Io)?;
    Ok(())
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
