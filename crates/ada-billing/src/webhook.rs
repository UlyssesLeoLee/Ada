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
    /// `customer.subscription.created`.
    CustomerSubscriptionCreated,
    /// `customer.subscription.updated`.
    CustomerSubscriptionUpdated,
    /// `customer.subscription.deleted`.
    CustomerSubscriptionDeleted,
    /// `invoice.paid`.
    InvoicePaid,
    /// `invoice.payment_failed`.
    InvoicePaymentFailed,
}

/// Parsing is a trait impl rather than an inherent `from_str` so the
/// type composes with `str::parse` and matches how
/// [`crate::subscription::SubscriptionStatus`] is parsed in this
/// crate. The error is [`BillingError::MalformedEnvelope`] because
/// the only caller reads the token out of a Stripe event envelope.
impl core::str::FromStr for EventKind {
    type Err = BillingError;

    fn from_str(s: &str) -> Result<Self> {
        Ok(match s {
            "customer.subscription.created" => Self::CustomerSubscriptionCreated,
            "customer.subscription.updated" => Self::CustomerSubscriptionUpdated,
            "customer.subscription.deleted" => Self::CustomerSubscriptionDeleted,
            "invoice.paid" => Self::InvoicePaid,
            "invoice.payment_failed" => Self::InvoicePaymentFailed,
            _ => return Err(BillingError::MalformedEnvelope),
        })
    }
}

/// One accepted event. Emitted into the channel for downstream
/// processing (DB updates, plan tier change notifications, …).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingEvent {
    /// Stripe's `event.id`; also the idempotency key.
    pub event_id: String,
    /// The event's `type`, parsed into a known token.
    pub kind: EventKind,
    /// The tenant resolved from the event's metadata.
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
    /// Create an empty store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns `true` if this is the first time we've seen the key.
    ///
    /// This is the deduplication primitive, and it is deliberately the
    /// *only* way to ask "have I seen this?". The answer and the insert
    /// are decided under one write lock, so a caller that branches on the
    /// return value cannot lose the race. See [`IdempotencyStore::has_seen`]
    /// for why the read variant is not safe to pair with this.
    pub fn record(&self, event_id: &str, tenant_id: &str) -> bool {
        let key = (event_id.to_owned(), tenant_id.to_owned());
        let mut w = self.seen.write();
        w.insert(key)
    }

    /// Returns `true` if the `(event_id, tenant_id)` key was already
    /// recorded.
    ///
    /// **Not a safe way to deduplicate.** This takes the read lock and
    /// releases it before the caller does anything else, so
    ///
    /// ```text
    /// if !store.has_seen(id, tenant) && store.record(id, tenant) { ... }
    /// ```
    ///
    /// still lets two threads pass the check before either inserts.
    /// Branch on [`IdempotencyStore::record`]'s return value instead. This
    /// accessor exists for assertions and diagnostics, which is how the
    /// tests use it.
    #[must_use]
    pub fn has_seen(&self, event_id: &str, tenant_id: &str) -> bool {
        self.seen
            .read()
            .contains(&(event_id.to_owned(), tenant_id.to_owned()))
    }

    /// Number of keys currently held. Diagnostics only — see
    /// [`IdempotencyStore::has_seen`] for why this is not a dedup path.
    ///
    /// Worth watching: the store has no retention policy, so this only ever
    /// grows. It is the number to alert on before a long-running process
    /// starts losing its dedup history (and, if a bound is added, the number
    /// that bound applies to).
    #[must_use]
    pub fn len(&self) -> usize {
        self.seen.read().len()
    }

    /// `true` when no keys are held. Pairs with [`IdempotencyStore::len`].
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.seen.read().is_empty()
    }
}

/// Trait alias for "something that can receive a `BillingEvent`".
/// The api-gateway implements this; tests use a `mpsc::UnboundedSender`.
pub trait EventSink: Send + Sync + 'static {
    /// Receive one accepted [`BillingEvent`].
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
    /// Build a handler over the shared config + idempotency store.
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
            let (k, v) = part.split_once('=').ok_or(BillingError::InvalidSignature)?;
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
        if bool::from(subtle::ConstantTimeEq::ct_eq(
            got.as_bytes(),
            expected.as_bytes(),
        )) {
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
        let kind: EventKind = kind_str.parse()?;
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

        // Validate the tenant **before** the idempotency table is
        // touched. Registering the key first would make a non-UUID
        // `metadata.tenant_id` permanently swallow the event: the
        // first delivery errors out, and every Stripe retry then hits
        // `has_seen` and is answered `Duplicate` — a success Stripe
        // stops retrying, so the subscription change is lost with no
        // error surfaced. Validation must strictly precede dedup
        // registration.
        let tenant = TenantId(
            uuid::Uuid::parse_str(&tenant_id).map_err(|_| BillingError::MalformedEnvelope)?,
        );

        // Check-and-record in ONE step.
        //
        // This used to be:
        //
        //     if self.idem.has_seen(&event_id, &tenant_id) {
        //         return Ok(WebhookOutcome::Duplicate);
        //     }
        //     self.idem.record(&event_id, &tenant_id);   // return value discarded
        //
        // which takes the read lock, drops it, then takes the write lock.
        // Two deliveries of the same event that interleave between those
        // two acquisitions both observe "not seen", both insert, and both
        // dispatch — the subscription change is applied twice, from one
        // Stripe event, on the money path.
        //
        // `record` returns whether the key was new for exactly this
        // reason. Deciding the answer under the same write lock that
        // performs the insert is what makes the loser of the race receive
        // `Duplicate` instead of an event it has already processed.
        //
        // Note `has_seen` below is NOT the dedup path and must not be used
        // as one. It is a separate read lock, so pairing it with `record`
        // reintroduces the window this line removed.
        if !self.idem.record(&event_id, &tenant_id) {
            return Ok(WebhookOutcome::Duplicate);
        }
        // Audit emission goes through ada-m11-rbac-collab in the
        // api-gateway wiring; here we only emit the BillingEvent.
        sink.handle(BillingEvent {
            event_id,
            kind,
            tenant_id: tenant,
            target_id,
        });
        Ok(WebhookOutcome::Accepted)
    }
}

/// Convenience wrapper used by the api-gateway route. Holds the
/// handler + an event sink + the mpsc sender used by tests.
pub struct WebhookService {
    /// The signature-verifying / deduplicating handler.
    pub handler: WebhookHandler,
    /// The downstream receiver for accepted events.
    pub sink: Arc<dyn EventSink>,
}

// Manual rather than derived: `Arc<dyn EventSink>` is not `Debug`, and
// the sink is an opaque trait object whose internals are not ours to
// print. Eliding it keeps the impl total without leaking event data.
impl core::fmt::Debug for WebhookService {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("WebhookService")
            .field("handler", &self.handler)
            .field("sink", &"<dyn EventSink>")
            .finish()
    }
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
            // Must be a parseable UUID: `handle` converts the metadata
            // tenant into a `TenantId` via `Uuid::parse_str` and returns
            // `MalformedEnvelope` otherwise, which would fail the
            // `.expect("first")` below for the wrong reason.
            "018f0000-0000-4000-8000-000000000001",
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
        assert!(!s.is_empty());
        assert_eq!(s.len(), 1);
    }

    /// The dedup decision and the dispatch have to be serialized with each
    /// other, not merely with other calls.
    ///
    /// `handle` used to read `has_seen` and *then* call `record`. Those are
    /// two separate lock acquisitions, so concurrent deliveries of one
    /// Stripe event could all pass the check before any of them inserted —
    /// and every one of them would go on to dispatch, applying the same
    /// subscription change several times from a single event. Stripe does
    /// deliver duplicates in practice (that is the whole reason this table
    /// exists), and nothing about the duplicate has to be simultaneous for
    /// the bug to be a bug: it only needs two deliveries to overlap.
    ///
    /// The barrier is what makes this a real test rather than a hopeful
    /// one. Without it the threads rarely collide and the old code would
    /// pass most of the time, which is the usual way a race "tests green"
    /// for months.
    #[test]
    fn concurrent_deliveries_of_one_event_dispatch_exactly_once() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Barrier;

        const THREADS: usize = 16;
        const ROUNDS: usize = 16;

        struct CountingSink(AtomicUsize);
        impl EventSink for CountingSink {
            fn handle(&self, _ev: BillingEvent) {
                self.0.fetch_add(1, Ordering::SeqCst);
            }
        }

        let mut accepted = 0usize;
        let mut duplicates = 0usize;
        let mut dispatched = 0usize;

        for round in 0..ROUNDS {
            let cfg = Arc::new(Config {
                stripe_secret_key: "sk_test_dummy".into(),
                stripe_webhook_secret: "whsec".into(),
                stripe_api_version: "2025-08-27.basil".into(),
                stripe_portal_return_url: None,
                stripe_base_url: "https://api.stripe.com/v1".into(),
            });
            let h = Arc::new(WebhookHandler::new(cfg, Arc::new(IdempotencyStore::new())));
            let sink = Arc::new(CountingSink(AtomicUsize::new(0)));
            let barrier = Arc::new(Barrier::new(THREADS));
            let body = serde_json::to_vec(&evt(
                &format!("evt_race_{round}"),
                "customer.subscription.updated",
                "018f0000-0000-4000-8000-000000000001",
                "sub_1",
            ))
            .unwrap();

            let handles: Vec<_> = (0..THREADS)
                .map(|_| {
                    let h = Arc::clone(&h);
                    let sink = Arc::clone(&sink);
                    let barrier = Arc::clone(&barrier);
                    let body = body.clone();
                    std::thread::spawn(move || {
                        barrier.wait();
                        h.handle(&body, &*sink)
                    })
                })
                .collect();

            for t in handles {
                match t.join().expect("worker thread") {
                    Ok(WebhookOutcome::Accepted) => accepted += 1,
                    Ok(WebhookOutcome::Duplicate) => duplicates += 1,
                    Err(e) => panic!("handle must not fail for a well-formed event: {e}"),
                }
            }
            dispatched += sink.0.load(Ordering::SeqCst);
        }

        assert_eq!(
            accepted, ROUNDS,
            "exactly one delivery per event may be Accepted"
        );
        assert_eq!(
            duplicates,
            THREADS * ROUNDS - ROUNDS,
            "every other delivery must be told Duplicate"
        );
        assert_eq!(
            dispatched, ROUNDS,
            "the sink must see each event exactly once — more than one \
             dispatch of a single Stripe event is a double charge"
        );
    }
}
