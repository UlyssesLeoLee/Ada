//! Error surface for `ada-billing`.
//!
//! Every public fallible function in this crate returns a
//! [`Result<T>`] whose error variant is [`BillingError`]. The
//! variants are kept narrow: the crate is a thin wrapper over the
//! Stripe REST API, and the only failure modes we surface to
//! upstream callers are (a) the things Stripe itself returns, (b)
//! the security-relevant events (signature, idempotency), and (c)
//! the misconfigurations we can detect at startup.
//!
//! See `auth-billing-arch.md` §5 and §6 for the threat model that
//! drives the signature / idempotency / config variants.

use thiserror::Error;

/// Single error type for all `ada-billing` fallible operations.
#[derive(Debug, Error)]
pub enum BillingError {
    /// A required environment variable was missing or malformed.
    ///
    /// Wrapped value is the variable name; the actual env value is
    /// **never** included (see §"Hard requirements" in the crate
    /// brief + the env-safety guardrail from 2026-08-27).
    #[error("missing or invalid config: {0}")]
    Config(String),

    /// The `Stripe-Signature` header failed HMAC-SHA256 verification
    /// or the `t=` timestamp was outside the 5-minute tolerance
    /// window. The raw header value is **not** logged.
    #[error("invalid stripe signature")]
    InvalidSignature,

    /// The webhook body could not be deserialized as JSON. The raw
    /// body is **not** logged.
    #[error("invalid webhook payload")]
    InvalidPayload,

    /// The webhook body's envelope is missing required fields
    /// (`id`, `type`, `data.object`).
    #[error("malformed webhook envelope")]
    MalformedEnvelope,

    /// An idempotency table lookup hit a key that was already
    /// processed. The handler treats this as a replay (the brief
    /// says "silently dropped"), so this variant is internal-only:
    /// it is used by [`crate::webhook::WebhookService::handle`] to
    /// communicate the duplicate back to the handler test but never
    /// surfaced to upstream.
    #[error("event already processed: event.id + tenant_id")]
    DuplicateEvent,

    /// Stripe returned a non-success HTTP status. The status code is
    /// preserved; the body is **not** echoed back to the caller (it
    /// may contain PII / Stripe-internal diagnostics).
    #[error("stripe API error: status={0}")]
    StripeApi(u16),

    /// A network or transport-level failure talking to Stripe.
    #[error("stripe transport error: {0}")]
    Transport(String),

    /// The [`crate::plan::Plan`] → price-id mapping did not find a
    /// configured Stripe Price ID for the requested plan + cadence.
    /// Surfaced by [`crate::customer::CustomerService`] and
    /// [`crate::portal::PortalService`] before they call Stripe.
    #[error("no stripe price id configured for {plan}/{cadence}")]
    PriceNotConfigured {
        /// The plan that was requested.
        plan: String,
        /// The cadence that was requested (`monthly`, `yearly`).
        cadence: String,
    },
}

/// `Result` alias for `ada-billing` fallible operations.
pub type Result<T> = core::result::Result<T, BillingError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_strings_are_descriptive_and_do_not_leak_secrets() {
        let e = BillingError::Config("STRIPE_SECRET_KEY".into());
        assert_eq!(
            e.to_string(),
            "missing or invalid config: STRIPE_SECRET_KEY"
        );
        // Invalid signature / payload / envelope must NOT include any
        // body / header content.
        assert!(BillingError::InvalidSignature.to_string().contains("invalid stripe signature"));
        assert!(BillingError::InvalidPayload.to_string().contains("invalid webhook payload"));
        assert!(BillingError::MalformedEnvelope.to_string().contains("malformed webhook envelope"));
    }

    #[test]
    fn stripe_api_error_carries_status_only() {
        let e = BillingError::StripeApi(402);
        assert_eq!(e.to_string(), "stripe API error: status=402");
    }

    #[test]
    fn price_not_configured_carries_plan_and_cadence() {
        let e = BillingError::PriceNotConfigured {
            plan: "team".into(),
            cadence: "yearly".into(),
        };
        assert!(e.to_string().contains("team"));
        assert!(e.to_string().contains("yearly"));
    }

    #[test]
    fn result_alias_carries_error() {
        let ok: Result<i32> = Ok(7);
        let err: Result<i32> = Err(BillingError::DuplicateEvent);
        assert!(matches!(ok, Ok(7)));
        assert!(matches!(err, Err(BillingError::DuplicateEvent)));
    }

    #[test]
    fn error_is_send_sync_and_static() {
        fn assert_send_sync_static<E: std::error::Error + Send + Sync + 'static>(_: &E) {}
        let e = BillingError::InvalidSignature;
        assert_send_sync_static(&e);
    }
}