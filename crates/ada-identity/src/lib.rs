//! `ada-identity` — SAML 2.0 / OIDC RP + TOTP / WebAuthn / Passkey
//! authentication crate for Ada v0.5.0.
//!
//! Implements the binding contract in
//! `docs/commercial/auth-billing-arch.md` §1-3.
//!
//! ## Module surface
//!
//! - [`config`] — env-driven configuration. NO env var values
//!   printed (per memory 2026-08-27).
//! - [`jwks`] — JWKS endpoint exposing the RS256 public key.
//! - [`mint`] — JWT minting (RS256, kid-tagged).
//! - [`oidc`] — OpenID Connect Authorization Code + PKCE RP
//!   (`openidconnect = "3"`).
//! - [`saml`] — SAML 2.0 SP: AuthnRequest generation + assertion
//!   parsing (`samael = "0.0"`).
//! - [`webauthn`] — WebAuthn RP: registration + authentication
//!   (`webauthn-rs = "0.6"`).
//! - [`totp`] — RFC 6238 TOTP (`totp-rs = "5"`).
//! - [`passkey`] — WebAuthn resident-key flow (re-export of webauthn).
//! - [`recovery`] — single-use recovery code generation + redemption.
//! - [`session`] — opaque session token + cookie management.
//! - [`rate_limit`] — token-bucket rate limiter for `/login`.
//!
//! ## Feature flags
//!
//! - default (no feature): real wire-format deps active.
//! - `stub`: restore the v0.4.0 in-house primitive implementations
//!   for downgrade safety. Build without `openidconnect` /
//!   `samael` / `webauthn-rs` / `totp-rs`.

#![forbid(unsafe_code)]
#![warn(missing_debug_implementations)]

pub mod base64util;
pub mod config;
pub mod error;
pub mod jwks;
pub mod mint;
pub mod oidc;
pub mod passkey;
pub mod rate_limit;
pub mod recovery;
pub mod saml;
pub mod session;
pub mod totp;
pub mod webauthn;

pub use config::Config;
pub use error::{IdentityError, Result};
pub use mint::{Claims, Jwt, JwtAlgorithm};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = env!("CARGO_PKG_NAME");
pub const LAYER: &str = "wire";