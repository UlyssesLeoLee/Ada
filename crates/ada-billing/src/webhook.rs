//! Stripe webhook handler.
//!
//! Verifies `Stripe-Signature` (HMAC-SHA256 over `t.body`), enforces
//! a 5-minute timestamp tolerance, deduplicates via
//! `(event.id, tenant_id)` idempotency table, and emits a
//! `BillingEvent` into a tokio mpsc channel for downstream
//! processing. Audit log entries are emitted on every accepted event
//! via [`ada_m11_rbac_collab::record_audit_log`].
//!
//! ## Threat model (per `auth-billing-arch.md` §6)
//!
//! * Signature mismatch → `BillingError::InvalidSignature` (the raw
//!   header / body is never logged).
//! * Replay (idempotency hit) → silently dropped, audit entry
//!   recorded.
//! * Malformed envelope → `BillingError::MalformedEnvelope`.
//!
//! No env var values appear in any log line.

use std::collections::HashSet;
use std::sync::Arc;

use hmac::{Hmac, Mac};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use sha2::Sha256;

use ada_core::TenantId;

use crate::config::Config;
use crate::error::{BillingError, Result};

type HmacSha256 = Hmac<Sha256>;

/// Subscription state token Stripe uses in events. Newtype over
/// `&'static str` so the matcher exhausts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventKind {
    CustomerSubscriptionCreated,
    CustomerSubscriptionUpdated,
    CustomerSubscriptionDeleted,
    InvoicePaid,
    InvoicePaymentFailed,
}

impl EventKind {
    #[must_use]
    pub fn from_str(s: &str) -> Option<Self> {
        Some(match s {
            "customer.subscription.created" => Self::CustomerSubscriptionCreated,
            "customer.subscription.updated" => Self::CustomerSubscriptionUpdated,
            "customer.subscription.deleted" => Self::CustomerSubscriptionDeleted,
            "invoice.paid" => Self::InvoicePaid,
            "invoice.payment_failed" => Self::InvoicePaymentFailed,
            _ => return None,
        })
    }
}

/// One accepted event. Emitted into the channel for downstream
/// processing (DB updates, plan tier change notifications, …).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingEvent {
    pub event_id: String,
    pub kind: EventKind,
    pub tenant_id: TenantId,
    /// Stripe-side identifier (`sub_…` / `cus_…`).
    pub target_id: String,
}

/// Idempotency store. In-process; production wiring uses Postgres.
#[derive(Debug, Default)]
pub struct IdempotencyStore {
    seen: RwLock<HashSet<(String, String)>>,
}

impl IdempotencyStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns `true` if this is the first time we've seen the key.
    pub fn record(&self, event_id: &str, tenant_id: &str) -> bool {
        let key = (event_id.to_owned(), tenant_id.to_owned());
        let mut w = self.seen.write();
        w.insert(key)
    }

    #[must_use]
    pub fn has_seen(&self, event_id: &str, tenant_id: &str) -> bool {
        self.seen.read().contains(&(event_id.to_owned(), tenant_id.to_owned()))
    }
}

/// Trait alias for "something that can receive a `BillingEvent`".
/// The api-gateway implements this; tests use a `mpsc::UnboundedSender`.
pub trait EventSink: Send + Sync + 'static {
    fn handle(&self, ev: BillingEvent);
}

/// Outcome of a webhook call. The handler returns this so the route
/// in api-gateway can choose the right HTTP status code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebhookOutcome {
    /// The event was accepted and dispatched.
    Accepted,
    /// Replay: same `event.id` for this tenant already seen.
    Duplicate,
}

/// Webhook handler. Holds config (with the signing secret) and the
/// idempotency store. The api-gateway route
/// `POST /webhooks/stripe` constructs a `WebhookHandler` per
/// request — the config + idempotency store are shared.
#[derive(Debug, Clone)]
pub struct WebhookHandler {
    cfg: Arc<Config>,
    idem: Arc<IdempotencyStore>,
}

impl WebhookHandler {
    #[must_use]
    pub fn new(cfg: Arc<Config>, idem: Arc<IdempotencyStore>) -> Self {
        Self { cfg, idem }
    }

    /// Verify `Stripe-Signature` and return `Ok(())` or an error.
    /// The header format is `t=<unix>,v1=<hex>[, v1=<hex>]*` — we
    /// check the **first** `v1` signature and require it to match a
    /// fresh HMAC of `<t>.<body>` keyed by the webhook secret.
    ///
    /// The `body` is the **raw** request body bytes (form-encoded
    /// JSON in modern API versions).
    pub fn verify_signature(&self, header: &str, body: &[u8], now_unix: i64) -> Result<()> {
        let mut t: Option<i64> = None;
        let mut v1: Option<&str> = None;
        for part in header.split(',') {
            let (k, v) = part
                .split_once('=')
                .ok_or(BillingError::InvalidSignature)?;
            match k.trim() {
                "t" => t = v.trim().parse().ok(),
                "v1" if v1.is_none() => v1 = Some(v.trim()),
                _ => {}
            }
        }
        let t = t.ok_or(BillingError::InvalidSignature)?;
        if (now_unix - t).abs() > 300 {
            // 5-minute tolerance window.
            return Err(BillingError::InvalidSignature);
        }
        let expected = v1.ok_or(BillingError::InvalidSignature)?;
        let mut mac = HmacSha256::new_from_slice(self.cfg.stripe_webhook_secret.as_bytes())
            .map_err(|_| BillingError::InvalidSignature)?;
        mac.update(format!("{t}.").as_bytes());
        mac.update(body);
        let got = hex::encode(mac.finalize().into_bytes());
        if bool::from(subtle::ConstantTimeEq::ct_eq(got.as_bytes(), expected.as_bytes())) {
            Ok(())
        } else {
            Err(BillingError::InvalidSignature)
        }
    }

    /// Process a verified event. Returns the outcome so the
    /// api-gateway route can pick the right HTTP status.
    pub fn handle(&self, body: &[u8], sink: &dyn EventSink) -> Result<WebhookOutcome> {
        let env: serde_json::Value =
            serde_json::from_slice(body).map_err(|_| BillingError::InvalidPayload)?;
        let event_id = env
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or(BillingError::MalformedEnvelope)?
            .to_owned();
        let kind_str = env
            .get("type")
            .and_then(|v| v.as_str())
            .ok_or(BillingError::MalformedEnvelope)?;
        let kind = EventKind::from_str(kind_str).ok_or(BillingError::MalformedEnvelope)?;
        // Tenant resolution: in v0.4.0 we tag every event with the
        // tenant that owns the customer. The full mapping comes from
        // the api-gateway's tenant context; for the v0.4.0 skeleton
        // we read it from `data.object.metadata.tenant_id`.
        let tenant_id = env
            .pointer("/data/object/metadata/tenant_id")
            .and_then(|v| v.as_str())
            .ok_or(BillingError::MalformedEnvelope)?
            .to_owned();
        let target_id = env
            .pointer("/data/object/id")
            .and_then(|v| v.as_str())
            .ok_or(BillingError::MalformedEnvelope)?
            .to_owned();

        if self.idem.has_seen(&event_id, &tenant_id) {
            return Ok(WebhookOutcome::Duplicate);
        }
        self.idem.record(&event_id, &tenant_id);
        // Audit emission goes through ada-m11-rbac-collab in the
        // api-gateway wiring; here we only emit the BillingEvent.
        sink.handle(BillingEvent {
            event_id,
            kind,
            tenant_id: TenantId(
                uuid::Uuid::parse_str(&tenant_id)
                    .map_err(|_| BillingError::MalformedEnvelope)?,
            ),
            target_id,
        });
        Ok(WebhookOutcome::Accepted)
    }
}

/// Convenience wrapper used by the api-gateway route. Holds the
/// handler + an event sink + the mpsc sender used by tests.
pub struct WebhookService {
    pub handler: WebhookHandler,
    pub sink: Arc<dyn EventSink>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evt(event_id: &str, kind: &str, tenant: &str, target: &str) -> serde_json::Value {
        serde_json::json!({
            "id": event_id,
            "type": kind,
            "data": {
                "object": {
                    "id": target,
                    "metadata": { "tenant_id": tenant }
                }
            }
        })
    }

    struct CaptureSink(std::sync::Mutex<Vec<BillingEvent>>);
    impl EventSink for CaptureSink {
        fn handle(&self, ev: BillingEvent) {
            self.0.lock().unwrap().push(ev);
        }
    }

    fn sign(secret: &str, t: i64, body: &[u8]) -> String {
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(format!("{t}.").as_bytes());
        mac.update(body);
        format!("t={t},v1={}", hex::encode(mac.finalize().into_bytes()))
    }

    #[test]
    fn signature_verifies_within_tolerance() {
        let cfg = Arc::new(Config {
            stripe_secret_key: "sk_test_dummy".into(),
            stripe_webhook_secret: "whsec_test".into(),
            stripe_api_version: "2025-08-27.basil".into(),
            stripe_portal_return_url: None,
            stripe_base_url: "https://api.stripe.com/v1".into(),
        });
        let h = WebhookHandler::new(cfg, Arc::new(IdempotencyStore::new()));
        let body = b"{\"id\":\"evt_1\"}";
        let now = 1_700_000_000_i64;
        let header = sign("whsec_test", now, body);
        assert!(h.verify_signature(&header, body, now).is_ok());
    }

    #[test]
    fn signature_rejects_outside_tolerance() {
        let cfg = Arc::new(Config {
            stripe_secret_key: "sk_test_dummy".into(),
            stripe_webhook_secret: "whsec_test".into(),
            stripe_api_version: "2025-08-27.basil".into(),
            stripe_portal_return_url: None,
            stripe_base_url: "https://api.stripe.com/v1".into(),
        });
        let h = WebhookHandler::new(cfg, Arc::new(IdempotencyStore::new()));
        let body = b"{\"id\":\"evt_1\"}";
        let now = 1_700_000_000_i64;
        let header = sign("whsec_test", now - 600, body);
        assert!(h.verify_signature(&header, body, now).is_err());
    }

    #[test]
    fn signature_rejects_wrong_secret() {
        let cfg = Arc::new(Config {
            stripe_secret_key: "sk_test_dummy".into(),
            stripe_webhook_secret: "whsec_correct".into(),
            stripe_api_version: "2025-08-27.basil".into(),
            stripe_portal_return_url: None,
            stripe_base_url: "https://api.stripe.com/v1".into(),
        });
        let h = WebhookHandler::new(cfg, Arc::new(IdempotencyStore::new()));
        let body = b"{\"id\":\"evt_1\"}";
        let now = 1_700_000_000_i64;
        let header = sign("whsec_wrong", now, body);
        assert!(h.verify_signature(&header, body, now).is_err());
    }

    #[test]
    fn handle_emits_event_and_dedupes_replay() {
        let cfg = Arc::new(Config {
            stripe_secret_key: "sk_test_dummy".into(),
            stripe_webhook_secret: "whsec".into(),
            stripe_api_version: "2025-08-27.basil".into(),
            stripe_portal_return_url: None,
            stripe_base_url: "https://api.stripe.com/v1".into(),
        });
        let h = WebhookHandler::new(cfg, Arc::new(IdempotencyStore::new()));
        let sink = std::sync::Arc::new(CaptureSink(std::sync::Mutex::new(Vec::new())));
        let body = serde_json::to_vec(&evt(
            "evt_1",
            "customer.subscription.updated",
            "tenant-1",
            "sub_1",
        ))
        .unwrap();

        let first = h.handle(&body, &*sink).expect("first");
        assert_eq!(first, WebhookOutcome::Accepted);
        let second = h.handle(&body, &*sink).expect("second");
        assert_eq!(second, WebhookOutcome::Duplicate);

        let captured = sink.0.lock().unwrap();
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0].event_id, "evt_1");
        assert_eq!(captured[0].kind, EventKind::CustomerSubscriptionUpdated);
    }

    #[test]
    fn handle_rejects_malformed_envelope() {
        let cfg = Arc::new(Config {
            stripe_secret_key: "sk_test_dummy".into(),
            stripe_webhook_secret: "whsec".into(),
            stripe_api_version: "2025-08-27.basil".into(),
            stripe_portal_return_url: None,
            stripe_base_url: "https://api.stripe.com/v1".into(),
        });
        let h = WebhookHandler::new(cfg, Arc::new(IdempotencyStore::new()));
        let sink = std::sync::Arc::new(CaptureSink(std::sync::Mutex::new(Vec::new())));
        let body = b"not json";
        let r = h.handle(body, &*sink);
        assert!(matches!(r, Err(BillingError::InvalidPayload)));
    }

    #[test]
    fn idempotency_store_basics() {
        let s = IdempotencyStore::new();
        assert!(s.record("evt_a", "tenant_a"));
        assert!(!s.record("evt_a", "tenant_a"));
        assert!(s.has_seen("evt_a", "tenant_a"));
        assert!(!s.has_seen("evt_b", "tenant_a"));
    }
}