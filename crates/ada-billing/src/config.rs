//! Process configuration for `ada-billing`.
//!
//! All configuration is sourced from the process environment via
//! [`Config::from_env`]. **No environment value is ever logged** —
//! the env-safety guardrail from 2026-08-27 JST (`禁止把任何环境
//! 变量内容打印到对话/终端/log`) is hard-baked into this module:
//! we report presence / absence / shape, but never the value.
//!
//! ## Required
//!
//! - `STRIPE_SECRET_KEY` — Stripe secret API key (`sk_test_…` or
//!   `sk_live_…`).
//! - `STRIPE_WEBHOOK_SECRET` — endpoint signing secret
//!   (`whsec_…`); used to verify `Stripe-Signature` on inbound
//!   webhooks.
//! - `STRIPE_API_VERSION` — pinned Stripe API version (e.g.
//!   `2025-08-27.basil`); enforced by the api-gateway / Stripe
//!   SDK.
//!
//! ## Optional
//!
//! - `STRIPE_PORTAL_RETURN_URL` — URL the customer lands on after
//!   closing the Stripe-hosted Billing Portal session. If absent,
//!   we fall back to `/billing` and the operator is expected to
//!   wire that path on the SPA side.
//!
//! The base URL for the Stripe REST API is hard-coded to
//! `https://api.stripe.com/v1`. The constructor [`Config::with_base_url`]
//! is exposed for the test suite so it can point the SDK at a
//! `wiremock` server.

use crate::error::{BillingError, Result};

/// Base URL of the Stripe REST API. Used by the [`reqwest`] client
/// constructed in [`crate::customer::CustomerService`] and the
/// portal service.
pub const DEFAULT_STRIPE_BASE_URL: &str = "https://api.stripe.com/v1";

/// Stripe API version pinned for this build. The value comes from
/// `STRIPE_API_VERSION` and is included in the `Stripe-Version`
/// header on every outbound call (the Stripe SDK also sends its
/// own pinned version; both are accepted).
#[derive(Debug, Clone)]
pub struct Config {
    /// `STRIPE_SECRET_KEY`. Held by the service objects that need
    /// to make authenticated outbound calls; never logged.
    pub stripe_secret_key: String,
    /// `STRIPE_WEBHOOK_SECRET`. Held by
    /// [`crate::webhook::WebhookHandler`] only; never logged.
    pub stripe_webhook_secret: String,
    /// `STRIPE_API_VERSION`. Sent in the `Stripe-Version` header.
    pub stripe_api_version: String,
    /// `STRIPE_PORTAL_RETURN_URL` (optional).
    pub stripe_portal_return_url: Option<String>,
    /// Base URL of the Stripe REST API. Overridable for tests via
    /// [`Config::with_base_url`].
    pub stripe_base_url: String,
}

impl Config {
    /// Build a [`Config`] by reading the four documented env vars.
    ///
    /// # Errors
    ///
    /// Returns [`BillingError::Config`] if any of the three required
    /// env vars is missing or empty. The error message names the
    /// variable but never its value.
    pub fn from_env() -> Result<Self> {
        // We use `std::env::var` directly (not the `envy` crate) to
        // keep the dep tree small and to make the env-safety audit
        // easy: every `var()` call is on its own line and any value
        // that flows out of it is named in code.
        let stripe_secret_key = std::env::var("STRIPE_SECRET_KEY")
            .map_err(|_| BillingError::Config("STRIPE_SECRET_KEY".into()))?;
        if stripe_secret_key.is_empty() {
            return Err(BillingError::Config("STRIPE_SECRET_KEY".into()));
        }
        let stripe_webhook_secret = std::env::var("STRIPE_WEBHOOK_SECRET")
            .map_err(|_| BillingError::Config("STRIPE_WEBHOOK_SECRET".into()))?;
        if stripe_webhook_secret.is_empty() {
            return Err(BillingError::Config("STRIPE_WEBHOOK_SECRET".into()));
        }
        let stripe_api_version = std::env::var("STRIPE_API_VERSION")
            .map_err(|_| BillingError::Config("STRIPE_API_VERSION".into()))?;
        if stripe_api_version.is_empty() {
            return Err(BillingError::Config("STRIPE_API_VERSION".into()));
        }
        let stripe_portal_return_url = std::env::var("STRIPE_PORTAL_RETURN_URL")
            .ok()
            .filter(|s| !s.is_empty());
        Ok(Self {
            stripe_secret_key,
            stripe_webhook_secret,
            stripe_api_version,
            stripe_portal_return_url,
            stripe_base_url: DEFAULT_STRIPE_BASE_URL.to_owned(),
        })
    }

    /// Override the base URL. **Test-only**: the production binary
    /// never calls this. The brief explicitly prohibits using real
    /// Stripe endpoints in the test suite.
    #[must_use]
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.stripe_base_url = base_url.into();
        self
    }

    /// True iff the secret looks like a test-mode key. Stripe test
    /// keys start with `sk_test_`; live keys start with `sk_live_`.
    /// Used by the test suite to assert that no live keys leak into
    /// a CI run.
    #[must_use]
    pub fn is_test_mode(&self) -> bool {
        self.stripe_secret_key.starts_with("sk_test_")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: build a `Config` with all four required env vars set,
    /// then clear them so the next test starts from a clean slate.
    fn set_env_all() {
        std::env::set_var("STRIPE_SECRET_KEY", "sk_test_dummy");
        std::env::set_var("STRIPE_WEBHOOK_SECRET", "whsec_dummy");
        std::env::set_var("STRIPE_API_VERSION", "2025-08-27.basil");
        std::env::set_var("STRIPE_PORTAL_RETURN_URL", "https://app.example.com/billing");
    }

    fn clear_env_all() {
        std::env::remove_var("STRIPE_SECRET_KEY");
        std::env::remove_var("STRIPE_WEBHOOK_SECRET");
        std::env::remove_var("STRIPE_API_VERSION");
        std::env::remove_var("STRIPE_PORTAL_RETURN_URL");
    }

    #[test]
    fn from_env_happy_path() {
        set_env_all();
        let cfg = Config::from_env().expect("from_env");
        clear_env_all();

        assert!(cfg.is_test_mode());
        assert_eq!(cfg.stripe_api_version, "2025-08-27.basil");
        assert_eq!(
            cfg.stripe_portal_return_url.as_deref(),
            Some("https://app.example.com/billing")
        );
        assert_eq!(cfg.stripe_base_url, DEFAULT_STRIPE_BASE_URL);
    }

    #[test]
    fn from_env_portal_return_url_optional() {
        std::env::set_var("STRIPE_SECRET_KEY", "sk_test_dummy");
        std::env::set_var("STRIPE_WEBHOOK_SECRET", "whsec_dummy");
        std::env::set_var("STRIPE_API_VERSION", "2025-08-27.basil");
        std::env::remove_var("STRIPE_PORTAL_RETURN_URL");
        let cfg = Config::from_env().expect("from_env");
        assert!(cfg.stripe_portal_return_url.is_none());
        // clean up
        clear_env_all();
    }

    #[test]
    fn from_env_empty_value_is_rejected() {
        std::env::set_var("STRIPE_SECRET_KEY", "");
        std::env::set_var("STRIPE_WEBHOOK_SECRET", "whsec_dummy");
        std::env::set_var("STRIPE_API_VERSION", "2025-08-27.basil");
        let err = Config::from_env().expect_err("empty secret rejected");
        clear_env_all();
        assert!(matches!(err, BillingError::Config(name) if name == "STRIPE_SECRET_KEY"));
    }

    #[test]
    fn from_env_missing_secret_reports_variable_name_only() {
        // Only set the webhook secret; the secret key is missing.
        std::env::remove_var("STRIPE_SECRET_KEY");
        std::env::set_var("STRIPE_WEBHOOK_SECRET", "whsec_dummy");
        std::env::set_var("STRIPE_API_VERSION", "2025-08-27.basil");
        let err = Config::from_env().expect_err("missing secret rejected");
        clear_env_all();
        // Critical: the error message must name the variable but
        // never echo any value.
        let s = err.to_string();
        assert!(s.contains("STRIPE_SECRET_KEY"));
        assert!(!s.contains("whsec_dummy"));
    }

    #[test]
    fn with_base_url_overrides_stripe_base_url() {
        set_env_all();
        let cfg = Config::from_env()
            .expect("from_env")
            .with_base_url("http://localhost:9999/v1");
        clear_env_all();
        assert_eq!(cfg.stripe_base_url, "http://localhost:9999/v1");
    }

    #[test]
    fn is_test_mode_distinguishes_live_key() {
        std::env::set_var("STRIPE_SECRET_KEY", "sk_live_dummy");
        std::env::set_var("STRIPE_WEBHOOK_SECRET", "whsec_dummy");
        std::env::set_var("STRIPE_API_VERSION", "2025-08-27.basil");
        let cfg = Config::from_env().expect("from_env");
        clear_env_all();
        assert!(!cfg.is_test_mode());
    }
}