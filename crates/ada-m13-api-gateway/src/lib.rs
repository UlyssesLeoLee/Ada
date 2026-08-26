//! M-13: API Gateway. axum + utoipa (D-11). REST + WebSocket. JWT auth (D-07).
//!
//! ## v0.1.0 scope (B2)
//!
//! This crate implements the **minimum skeleton** required for B2:
//!
//! - [`AppState`] — per-request state shared across handlers
//! - [`HealthCheck`] trait + [`MemoryHealthCheck`] default
//! - [`ApiError`] with `IntoResponse` mapping
//! - [`build_router`] with the four v0.1.0 endpoints:
//!   - `GET /health` — JSON snapshot for dashboards
//!   - `GET /health/live` — plain-text liveness probe
//!   - `GET /health/ready` — readiness probe backed by [`HealthCheck`]
//!   - `GET /api/v1/ping` — smoke endpoint (`pong: true`)
//!
//! Production middleware (CORS / HSTS / JWT / tenant / RBAC) and the
//! real authentication/authorization layer live in B3+. See
//! [`DOC-MOD-013`](../docs/modules/M-13-api-gateway.md) §3.1 for the
//! full middleware chain and [`api/error-codes.md`](../docs/api/error-codes.md)
//! for the canonical error-code mapping.
//!
//! 関連 IPA フェーズ: 22-52 (基本設計/詳細設計), 53-58 (実装), 59-95 (試験)
//! 設計書: docs/modules/M-13-api-gateway.md (DOC-MOD-013)
//! ワークフロー: docs/architecture/08-workflow-overview.md

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

mod error;
mod health;
mod router;
mod state;

pub use error::{ApiError, Result};
pub use health::{HealthCheck, HealthStatus, MemoryHealthCheck};
pub use router::{build_router, HealthSnapshot};
pub use state::AppState;

/// Crate version, taken from `CARGO_PKG_VERSION` (single workspace
/// version per D-09).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Crate name, taken from `CARGO_PKG_NAME`.
pub const NAME: &str = env!("CARGO_PKG_NAME");

/// `skeleton`-layer string tag (仿生モデル 4 層分類, see
/// [`DOC-ARCH-001`](https://example.invalid/docs/architecture/00-anatomy-model.md)).
pub const LAYER: &str = "skeleton";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_not_empty() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn name_not_empty() {
        assert!(!NAME.is_empty());
    }

    #[test]
    fn layer_is_known() {
        assert!(
            ["skeleton", "blood", "nerve", "muscle", "shared"].contains(&LAYER),
            "Unknown layer: {LAYER}"
        );
    }
}
