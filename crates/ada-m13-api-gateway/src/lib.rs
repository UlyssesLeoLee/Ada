//! M-13: API Gateway. axum + utoipa (D-11). REST + WebSocket. JWT auth (D-07).
//!
//! ## v0.1.0 scope (B2)
//!
//! This crate implements the **minimum skeleton** required for B2:
//!
//! - [`AppState`] — per-request state shared across handlers
//! - [`HealthCheck`] trait + [`MemoryHealthCheck`] default
//! - [`ApiError`] with `IntoResponse` mapping
//! - [`build_router`] with the endpoints below
//!
//! ## Authentication and authorization
//!
//! `/health`, `/health/live` and `/health/ready` are unauthenticated,
//! because a kubelet probe cannot carry a bearer token and a probe that
//! 401s takes the pod out of service.
//!
//! Everything under `/api` requires `Authorization: Bearer <token>`,
//! resolved by [`auth::Principal`] against
//! `ada_identity::session::SessionStore`. The tenant comes from the
//! server-side session and **never** from the client's `x-tenant-id`
//! header — `gm-console` forwards that header verbatim, and nothing
//! here reads it to make a decision. Authorization is answered by
//! `ada-rbac-casbin` via [`auth::AuthContext::authorize`].
//!
//! There is no JWT verification. `ada_identity::mint::verify_jwt_stub`
//! fails closed for every input because the crate has no asymmetric
//! crypto dependency, and a decode-only "verifier" would let a caller
//! mint their own `tenant` and `roles`. The stateless-JWT path is
//! future work; it must not be faked.
//!
//! A freshly started pod has an empty session store and no login flow,
//! so every `/api` request is a 401. That is the intended posture.
//!
//! ## Login
//!
//! [`login`] adds `POST /api/v1/auth/login`, the route that closes that
//! loop: it verifies an email + password and mints the same opaque
//! session token the rest of the crate already validates. It is mounted
//! outside the authentication layer, because it is the one route a
//! caller reaches without a credential.
//!
//! The token is **not** a JWT. `ada_identity::mint::mint_jwt` fails
//! closed for every input because no RS256 signer is wired in, and
//! `verify_jwt_stub` refuses every token for the same reason, so
//! issuing one here would mean shipping the unsigned-token forgery the
//! `ada-identity` tests exist to prevent. The client is unaffected: it
//! reads `token` off the response and sends it back as a bearer, and
//! both halves of that are unchanged.
//!
//! CORS / HSTS remain unwired. See
//! [`DOC-MOD-013`](../../../docs/modules/M-13-api-gateway.md) §3.1 for the
//! intended full chain and `../../../docs/api/error-codes.md` for the
//! canonical error-code mapping.

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

pub mod auth;
mod error;
mod health;
pub mod login;
mod router;
pub mod server;
mod state;

pub use error::{ApiError, Result};
pub use health::{HealthCheck, HealthStatus, MemoryHealthCheck};
pub use login::{CredentialDirectory, LoginService, StoredUser};
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
        assert_ne!(VERSION, "");
    }

    #[test]
    fn name_not_empty() {
        assert_ne!(NAME, "");
    }

    #[test]
    fn layer_is_known() {
        assert!(
            ["skeleton", "blood", "nerve", "muscle", "shared"].contains(&LAYER),
            "Unknown layer: {LAYER}"
        );
    }
}
