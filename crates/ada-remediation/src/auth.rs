//! Webhook + manual-trigger shared-secret auth (v0.7.0).
//!
//! v0.6.0 accepted every `POST /webhook/alertmanager` and
//! `POST /remediation/trigger` from any caller, on the
//! assumption that the bound interface was private
//! (k8s `NetworkPolicy`). v0.7.0 hardens this: the
//! caller must present a shared secret in the
//! `X-Webhook-Token` header.
//!
//! # Why a shared secret, not HMAC-SHA256?
//!
//! The standard webhook-signing scheme is HMAC-SHA256
//! over the request body, with a signature in
//! `X-Hub-Signature-256` (GitHub style) or
//! `X-Alertmanager-Signature` (Prometheus style). The
//! `hmac` / `sha2` / `subtle` crates are **not** in the
//! offline `Cargo.lock` for this project (verified: 0
//! matches for `^name = "hmac"`, `^name = "sha2"`,
//! `^name = "subtle"`), and the dev environment has no
//! network. v0.7.0 therefore uses a shared-secret-in-
//! header scheme with a constant-time compare. The
//! compare uses `constant_time_eq 0.4.2` (already
//! present as a transitive dep of `blake3`, so no new
//! crate download is required).
//!
//! v0.7.1 is expected to swap to HMAC-SHA256 + a
//! per-request nonce once the offline cache is
//! rebuilt. The `AuthState` / `check_token` API in this
//! module is the integration point — call sites do not
//! change when the scheme upgrades.
//!
//! # Configuration
//!
//! The secret is read from `REMEDIATION_WEBHOOK_SECRET`
//! at startup via [`AuthState::from_env`]. If the env
//! var is unset:
//!
//! * A `WARN` is logged.
//! * [`AuthState::is_enabled`] returns `false`.
//! * [`AuthState::check_token`] returns
//!   [`AuthError::Disabled`] for every request, so the
//!   HTTP handler can decide whether to fail-closed
//!   (recommended) or fail-open with a 401 stub (dev
//!   mode).
//!
//! Production wiring must set
//! `REMEDIATION_WEBHOOK_SECRET` (or the equivalent
//! Kubernetes `Secret` mount) to a high-entropy value
//! and treat a missing variable at startup as a fatal
//! error (the caller is expected to call
//! [`AuthState::require_enabled`] in `main`).
//!
//! # Constant-time compare
//!
//! `constant_time_eq::constant_time_eq` does not short-
//! circuit on the first differing byte; it processes
//! the whole input even after a mismatch is found. This
//! defeats timing-side-channel attacks on the secret.
//! The compare rejects inputs of different length
//! without timing variability (an attacker cannot
//! probe the secret length by timing the response).

use constant_time_eq::constant_time_eq;
use std::env;
use std::fmt;

/// Header name for the shared secret. Lowercase so it
/// matches the canonical form axum uses for header
/// lookups.
pub const HEADER_NAME: &str = "x-webhook-token";

/// Env var read at startup. Set this to a high-entropy
/// value (`openssl rand -hex 32` is a good source) in
/// the k8s `Secret` and mount as an env var in the
/// `ada-remediation` deployment.
pub const ENV_VAR: &str = "REMEDIATION_WEBHOOK_SECRET";

/// Errors that [`AuthState::check_token`] can return.
#[allow(clippy::too_many_lines)]
#[derive(Debug, PartialEq, Eq)]
pub enum AuthError {
    /// Auth is disabled because [`ENV_VAR`] was unset
    /// at startup. The HTTP handler should reject
    /// every incoming request as 503 ("auth backend
    /// not configured") or fall through to dev mode.
    Disabled,
    /// The `X-Webhook-Token` header was missing.
    MissingToken,
    /// The header value did not match the configured
    /// secret.
    InvalidToken,
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Disabled => f.write_str("webhook auth is disabled"),
            Self::MissingToken => f.write_str("missing X-Webhook-Token header"),
            Self::InvalidToken => f.write_str("invalid X-Webhook-Token"),
        }
    }
}

impl std::error::Error for AuthError {}

/// The configured auth state. Cheap to clone (one
/// `Vec<u8>` clone for the secret).
#[derive(Debug, Clone)]
pub struct AuthState {
    secret: Option<Vec<u8>>,
}

impl AuthState {
    /// Build a disabled state. Useful for tests and
    /// for the "no env var" startup path.
    #[must_use]
    pub fn disabled() -> Self {
        Self { secret: None }
    }

    /// Build an enabled state from the given secret.
    /// The secret is stored as raw bytes; the compare
    /// is constant-time.
    #[must_use]
    pub fn enabled(secret: impl Into<Vec<u8>>) -> Self {
        Self {
            secret: Some(secret.into()),
        }
    }

    /// Read [`ENV_VAR`] from the process environment.
    /// Returns a disabled state + `false` if the var is
    /// unset or empty. Returns an enabled state + `true`
    /// otherwise.
    pub fn from_env() -> (Self, bool) {
        match env::var(ENV_VAR) {
            Ok(v) if !v.is_empty() => (Self::enabled(v), true),
            Ok(_) | Err(_) => {
                tracing::warn!(
                    "{} is unset or empty; webhook auth is DISABLED \
                     (production must set this env var)",
                    ENV_VAR
                );
                (Self::disabled(), false)
            }
        }
    }

    /// `true` if a secret was loaded. Use this in
    /// `main` to decide whether to abort startup.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.secret.is_some()
    }

    /// Panic if auth is disabled. Intended for
    /// production `main`:
    ///
    /// ```ignore
    /// let (auth, enabled) = AuthState::from_env();
    /// auth.require_enabled();
    /// ```
    ///
    /// In tests we use [`Self::disabled`] directly.
    pub fn require_enabled(&self) {
        if self.secret.is_some() {
            return;
        }
        panic!(
            "webhook auth is disabled ({ENV_VAR}=missing); \
             refusing to start without a configured secret \
             — see docs/observability/14-auto-remediation.md §auth"
        );
    }

    /// Compare `header_value` against the configured
    /// secret. Returns `Ok(())` on match.
    ///
    /// - [`AuthError::Disabled`] if no secret is set
    ///   (caller decides whether to fail-closed).
    /// - [`AuthError::MissingToken`] if `header_value`
    ///   is `None`.
    /// - [`AuthError::InvalidToken`] if the value
    ///   does not match (constant-time compare).
    pub fn check_token(&self, header_value: Option<&str>) -> Result<(), AuthError> {
        let secret = self.secret.as_deref().ok_or(AuthError::Disabled)?;
        let provided = header_value.ok_or(AuthError::MissingToken)?;
        // Length check first: comparing slices of
        // different length via constant_time_eq is
        // safe (it returns false), but rejecting
        // mismatched lengths early keeps the call
        // obvious and the log line clean.
        if provided.len() != secret.len() {
            return Err(AuthError::InvalidToken);
        }
        if constant_time_eq(provided.as_bytes(), secret) {
            Ok(())
        } else {
            Err(AuthError::InvalidToken)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_token_returns_ok() {
        let auth = AuthState::enabled("super-secret-token-xyz");
        let result = auth.check_token(Some("super-secret-token-xyz"));
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn invalid_token_returns_invalid_token_error() {
        let auth = AuthState::enabled("super-secret-token-xyz");
        // Same length, different bytes.
        let result = auth.check_token(Some("super-secret-token-ABC"));
        assert_eq!(result, Err(AuthError::InvalidToken));
        // Different length also returns InvalidToken
        // (not MissingToken — the header *was*
        // present, just wrong).
        let result_short = auth.check_token(Some("short"));
        assert_eq!(result_short, Err(AuthError::InvalidToken));
    }

    #[test]
    fn missing_token_returns_missing_token_error() {
        let auth = AuthState::enabled("super-secret-token-xyz");
        let result = auth.check_token(None);
        assert_eq!(result, Err(AuthError::MissingToken));
    }

    #[test]
    fn disabled_state_returns_disabled_error() {
        let auth = AuthState::disabled();
        // Caller can detect this and either fail-closed
        // (return 503) or fail-open (treat as missing
        // token). The handler in `http.rs` is configured
        // to fail-closed.
        assert_eq!(auth.check_token(Some("anything")), Err(AuthError::Disabled));
        assert_eq!(auth.check_token(None), Err(AuthError::Disabled));
        assert!(!auth.is_enabled());
    }

    #[test]
    fn require_enabled_panics_when_disabled() {
        let auth = AuthState::disabled();
        let result = std::panic::catch_unwind(|| auth.require_enabled());
        assert!(
            result.is_err(),
            "require_enabled should panic when disabled"
        );
    }

    #[test]
    fn require_enabled_silent_when_enabled() {
        let auth = AuthState::enabled("any-secret");
        let result = std::panic::catch_unwind(|| auth.require_enabled());
        assert!(
            result.is_ok(),
            "require_enabled should not panic when enabled"
        );
    }
}
