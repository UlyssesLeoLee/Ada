//! gm-console configuration. All values come from env vars or defaults — never from
//! disk-based secrets. NO environment value is ever logged (per memory 8/27 hard ban).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub bind_addr: String,
    pub upstream_url: String,
    pub static_dir: Option<String>,
    pub log_level: String,
    pub enable_compression: bool,
    pub enable_cors: bool,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            bind_addr: std::env::var("GM_CONSOLE_BIND").unwrap_or_else(|_| "0.0.0.0:8080".into()),
            upstream_url: std::env::var("GM_CONSOLE_UPSTREAM")
                .unwrap_or_else(|_| "http://ada-api-gateway:8080".into()),
            static_dir: std::env::var("GM_CONSOLE_STATIC_DIR").ok(),
            log_level: std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
            enable_compression: true,
            enable_cors: true,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::from_env()
    }
}
