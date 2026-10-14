//! gm-console configuration. All values come from env vars or defaults — never from
//! disk-based secrets. NO environment value is ever logged (per memory 8/27 hard ban).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub bind_addr: String,
    pub upstream_url: String,
    /// Optional override for the static document root.
    ///
    /// # This directory is a PUBLIC document root
    ///
    /// When set, every readable file inside it — subdirectories
    /// included — is served by URL, minus dotfiles and dot-directories.
    /// Anything in here is public. Never point this at a directory that
    /// contains anything other than built assets: a project root, a
    /// directory holding `.env` variants, or a working copy.
    ///
    /// The served paths go through
    /// [`crate::routes::resolve_static_path`], which refuses any path
    /// segment beginning with `.` — that covers `.env`, `.git`, and
    /// `..` as a traversal, in one rule. Ordinary files in ordinary
    /// subdirectories are served, because that is what a static server
    /// does; the guard exists to keep secrets that are conventionally
    /// hidden out, not to second-guess what belongs in a build output
    /// directory.
    ///
    /// Set in `deploy/k8s/gm-console.yaml` to `/srv/static`, the document
    /// root baked into the image by
    /// `deploy/docker/gm-console.Dockerfile`. The deployed pod therefore
    /// serves the committed `apps/gm-console-web/dist/` and needs no
    /// volume for it, which is what lets that pod run
    /// `readOnlyRootFilesystem: true`. It was previously unset, so a
    /// deployed pod served the `index.html` compiled into the binary and
    /// had no directory to get wrong — which also meant no CSS, no login
    /// page, and nothing that looked like a console.
    ///
    /// Override it for local iteration, where a `dist/` is rebuilt
    /// constantly: point it at a checkout's `dist/`.
    pub static_dir: Option<String>,
    pub log_level: String,
    pub enable_compression: bool,
    pub enable_cors: bool,
    /// Comma-separated list of allowed CORS origins. Default: production kanvas + localhost dev.
    pub allowed_origins: Vec<String>,
}

impl Config {
    pub fn from_env() -> Self {
        let allowed_origins = std::env::var("GM_CONSOLE_ALLOWED_ORIGINS")
            .ok()
            .map(|raw| {
                raw.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
            })
            .filter(|v: &Vec<String>| !v.is_empty())
            .unwrap_or_else(|| {
                vec![
                    "https://gm-console.kanvas.dev".to_string(),
                    "https://localhost:3000".to_string(),
                ]
            });

        Self {
            bind_addr: std::env::var("GM_CONSOLE_BIND").unwrap_or_else(|_| "0.0.0.0:8080".into()),
            upstream_url: std::env::var("GM_CONSOLE_UPSTREAM")
                .unwrap_or_else(|_| "http://ada-api-gateway:8080".into()),
            static_dir: std::env::var("GM_CONSOLE_STATIC_DIR").ok(),
            log_level: std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
            enable_compression: true,
            enable_cors: true,
            allowed_origins,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::from_env()
    }
}
