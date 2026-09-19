//! `ada-identity` — SAML 2.0 / OIDC RP + TOTP / WebAuthn / Passkey
//! authentication crate for Ada v0.4.0.
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
//! - [`oidc`] — OpenID Connect Authorization Code + PKCE RP.
//! - [`saml`] — SAML 2.0 SP: AuthnRequest generation + assertion
//!   parsing.
//! - [`webauthn`] — WebAuthn RP: registration + authentication.
//! - [`totp`] — RFC 6238 TOTP.
//! - [`passkey`] — WebAuthn resident-key flow (re-export of webauthn).
//! - [`recovery`] — single-use recovery code generation + redemption.
//! - [`session`] — opaque session token + cookie management.
//! - [`rate_limit`] — token-bucket rate limiter for `/login`.
//!
//! ## Out of scope (v0.5.0+)
//!
//! - Real IdP integrations tested against a live SAML/OIDC IdP
//!   (the v0.4.0 tests use mocked endpoints).
//! - SAML IdP-initiated flow.
//! - SCIM provisioning.

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
pub use totp::{TotpCode, TotpSecret};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = env!("CARGO_PKG_NAME");
pub const LAYER: &str = "skeleton";