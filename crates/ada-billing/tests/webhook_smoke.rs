//! `webhook_smoke` — `Stripe-Signature` verification, idempotent replay
//! handling, and envelope validation.
//!
//! Drives `WebhookHandler` directly. The crate does not export an axum
//! `Router` for `POST /webhooks/stripe`, so the route-level round-trip
//! described in `auth-billing-arch.md` §7 cannot be assembled from the
//! public API; what is covered here is the handler contract underneath
//! it — the security-relevant part of that flow. Signatures are
//! produced in-test with the same HMAC-SHA256 construction the handler
//! verifies, so no network and no Stripe endpoint are involved.

use std::sync::{Arc, Mutex};

use ada_billing::webhook::EventKind;
use ada_billing::{
    BillingError, BillingEvent, Config, EventSink, IdempotencyStore, WebhookHandler, WebhookOutcome,
};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

const SECRET: &str = "whsec_smoke";
/// Fixed "now" so the 5-minute tolerance window is not time-dependent.
const NOW: i64 = 1_700_000_000;

/// Collects everything the handler dispatches.
#[derive(Default)]
struct CaptureSink(Mutex<Vec<BillingEvent>>);

impl CaptureSink {
    fn events(&self) -> Vec<BillingEvent> {
        self.0.lock().expect("sink lock").clone()
    }
}

impl EventSink for CaptureSink {
    fn handle(&self, ev: BillingEvent) {
        self.0.lock().expect("sink lock").push(ev);
    }
}

fn handler() -> WebhookHandler {
    handler_over(Arc::new(IdempotencyStore::new()))
}

/// Same handler, but over a store the test keeps a handle on, so the
/// registration side of the dedup contract is observable.
fn handler_over(idem: Arc<IdempotencyStore>) -> WebhookHandler {
    let cfg = Config {
        stripe_secret_key: "sk_test_smoke".into(),
        stripe_webhook_secret: SECRET.into(),
        stripe_api_version: "2025-08-27.basil".into(),
        stripe_portal_return_url: None,
        stripe_base_url: "https://api.stripe.com/v1".into(),
    };
    WebhookHandler::new(Arc::new(cfg), idem)
}

/// `t=<unix>,v1=<hex>` over `HMAC-SHA256(secret, "<t>.<body>")`.
fn sign(secret: &str, t: i64, body: &[u8]) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("hmac key");
    mac.update(format!("{t}.").as_bytes());
    mac.update(body);
    format!("t={t},v1={}", hex::encode(mac.finalize().into_bytes()))
}

/// A well-formed Stripe event envelope. `tenant` must be a bare UUID —
/// the handler parses `data.object.metadata.tenant_id` into a
/// `TenantId`, so a non-UUID value is a malformed envelope.
fn event_body(event_id: &str, kind: &str, tenant: &str, target: &str) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "id": event_id,
        "type": kind,
        "data": {
            "object": {
                "id": target,
                "metadata": { "tenant_id": tenant }
            }
        }
    }))
    .expect("serialize event")
}

#[test]
fn a_signature_from_the_configured_secret_is_accepted() {
    let h = handler();
    let body = b"{\"id\":\"evt_smoke_1\"}";

    assert!(h
        .verify_signature(&sign(SECRET, NOW, body), body, NOW)
        .is_ok());
    // Extra `v1=` entries are tolerated; the first one is checked.
    let multi = format!("{},v1=deadbeef", sign(SECRET, NOW, body));
    assert!(h.verify_signature(&multi, body, NOW).is_ok());
}

#[test]
fn a_wrong_secret_or_a_tampered_body_is_rejected() {
    let h = handler();
    let body = b"{\"id\":\"evt_smoke_1\"}";

    let err = h
        .verify_signature(&sign("whsec_attacker", NOW, body), body, NOW)
        .expect_err("a foreign secret must not verify");
    assert!(matches!(err, BillingError::InvalidSignature), "got {err}");

    // Signature covers the raw bytes, so any body edit invalidates it.
    let tampered = b"{\"id\":\"evt_smoke_2\"}";
    let err = h
        .verify_signature(&sign(SECRET, NOW, body), tampered, NOW)
        .expect_err("a tampered body must not verify");
    assert!(matches!(err, BillingError::InvalidSignature), "got {err}");

    // A garbage header cannot be parsed into `t` / `v1`.
    let err = h
        .verify_signature("not-a-signature", body, NOW)
        .expect_err("an unparsable header must not verify");
    assert!(matches!(err, BillingError::InvalidSignature), "got {err}");
}

#[test]
fn timestamps_outside_the_five_minute_window_are_rejected() {
    let h = handler();
    let body = b"{\"id\":\"evt_smoke_3\"}";

    // Boundary is inclusive at 300 s either way.
    assert!(h
        .verify_signature(&sign(SECRET, NOW - 300, body), body, NOW)
        .is_ok());
    assert!(h
        .verify_signature(&sign(SECRET, NOW + 300, body), body, NOW)
        .is_ok());

    for stale in [NOW - 301, NOW + 301] {
        let err = h
            .verify_signature(&sign(SECRET, stale, body), body, NOW)
            .expect_err("a stale signature must not verify");
        assert!(matches!(err, BillingError::InvalidSignature), "got {err}");
    }
}

#[test]
fn the_first_delivery_is_accepted_and_reaches_the_sink() {
    let h = handler();
    let sink = CaptureSink::default();
    let tenant = Uuid::new_v4();
    let body = event_body(
        "evt_smoke_4",
        "customer.subscription.updated",
        &tenant.to_string(),
        "sub_smoke_4",
    );

    let outcome = h.handle(&body, &sink).expect("handle");
    assert_eq!(outcome, WebhookOutcome::Accepted);

    let events = sink.events();
    assert_eq!(events.len(), 1);
    let ev = &events[0];
    assert_eq!(ev.event_id, "evt_smoke_4");
    assert_eq!(ev.kind, EventKind::CustomerSubscriptionUpdated);
    assert_eq!(ev.tenant_id, ada_billing::TenantId(tenant));
    assert_eq!(ev.target_id, "sub_smoke_4");
}

#[test]
fn a_replayed_event_is_dropped_as_a_duplicate() {
    let h = handler();
    let sink = CaptureSink::default();
    let body = event_body(
        "evt_smoke_5",
        "invoice.paid",
        &Uuid::new_v4().to_string(),
        "in_smoke_5",
    );

    assert_eq!(
        h.handle(&body, &sink).expect("first"),
        WebhookOutcome::Accepted
    );
    assert_eq!(
        h.handle(&body, &sink).expect("second"),
        WebhookOutcome::Duplicate
    );
    assert_eq!(
        sink.events().len(),
        1,
        "a replay must not be dispatched downstream twice"
    );
}

#[test]
fn the_idempotency_key_is_scoped_by_tenant() {
    // §6.5: the table is keyed by `event.id` + `tenant_id`, so the same
    // event id arriving for a different tenant is not a duplicate.
    let h = handler();
    let sink = CaptureSink::default();
    let tenant_a = Uuid::new_v4();
    let tenant_b = Uuid::new_v4();
    let body_a = event_body(
        "evt_smoke_6",
        "invoice.payment_failed",
        &tenant_a.to_string(),
        "in_6",
    );
    let body_b = event_body(
        "evt_smoke_6",
        "invoice.payment_failed",
        &tenant_b.to_string(),
        "in_6",
    );

    assert_eq!(
        h.handle(&body_a, &sink).expect("a"),
        WebhookOutcome::Accepted
    );
    assert_eq!(
        h.handle(&body_b, &sink).expect("b"),
        WebhookOutcome::Accepted
    );

    let events = sink.events();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].tenant_id, ada_billing::TenantId(tenant_a));
    assert_eq!(events[1].tenant_id, ada_billing::TenantId(tenant_b));
}

#[test]
fn every_documented_event_kind_is_recognised() {
    let h = handler();
    let expected = [
        (
            "customer.subscription.created",
            EventKind::CustomerSubscriptionCreated,
        ),
        (
            "customer.subscription.updated",
            EventKind::CustomerSubscriptionUpdated,
        ),
        (
            "customer.subscription.deleted",
            EventKind::CustomerSubscriptionDeleted,
        ),
        ("invoice.paid", EventKind::InvoicePaid),
        ("invoice.payment_failed", EventKind::InvoicePaymentFailed),
    ];

    for (idx, (type_str, kind)) in expected.iter().enumerate() {
        let body = event_body(
            &format!("evt_kind_{idx}"),
            type_str,
            &Uuid::new_v4().to_string(),
            "sub_kind",
        );
        h.handle(&body, &CaptureSink::default()).expect("handle");
        let parsed: EventKind = type_str.parse().expect("documented type must parse");
        assert_eq!(parsed, *kind);
    }
}

#[test]
fn malformed_envelopes_are_rejected_before_dispatch() {
    let h = handler();
    let sink = CaptureSink::default();
    let tenant = Uuid::new_v4();

    // Not JSON at all.
    let err = h.handle(b"not json", &sink).expect_err("non-JSON body");
    assert!(matches!(err, BillingError::InvalidPayload), "got {err}");

    // Missing `id`.
    let no_id = serde_json::to_vec(&serde_json::json!({
        "type": "invoice.paid",
        "data": { "object": { "id": "in_1", "metadata": { "tenant_id": tenant.to_string() } } }
    }))
    .expect("serialize");
    let err = h.handle(&no_id, &sink).expect_err("missing id");
    assert!(matches!(err, BillingError::MalformedEnvelope), "got {err}");

    // Unknown `type`.
    let unknown_kind = event_body(
        "evt_bad_kind",
        "charge.refunded",
        &tenant.to_string(),
        "ch_1",
    );
    let err = h.handle(&unknown_kind, &sink).expect_err("unknown type");
    assert!(matches!(err, BillingError::MalformedEnvelope), "got {err}");

    // Missing `data.object.metadata.tenant_id` (tenant comes from the
    // validated api-gateway context, never from an ad-hoc field).
    let no_tenant = serde_json::to_vec(&serde_json::json!({
        "id": "evt_no_tenant",
        "type": "invoice.paid",
        "data": { "object": { "id": "in_2" } }
    }))
    .expect("serialize");
    let err = h.handle(&no_tenant, &sink).expect_err("missing tenant_id");
    assert!(matches!(err, BillingError::MalformedEnvelope), "got {err}");

    // `tenant_id` present but not a UUID.
    let bad_tenant = event_body("evt_bad_tenant", "invoice.paid", "tenant-1", "in_3");
    let err = h.handle(&bad_tenant, &sink).expect_err("non-uuid tenant");
    assert!(matches!(err, BillingError::MalformedEnvelope), "got {err}");

    assert_eq!(
        sink.events().len(),
        0,
        "no malformed body may be dispatched"
    );
}

#[test]
fn a_non_uuid_tenant_id_does_not_register_the_idempotency_key() {
    // Regression: validation must precede dedup registration. If the
    // key were recorded before the tenant is parsed, the first
    // delivery would error out with the key already on file, and the
    // Stripe retry would be answered `Duplicate` — a success — so the
    // event would be lost silently and permanently.
    let idem = Arc::new(IdempotencyStore::new());
    let h = handler_over(Arc::clone(&idem));
    let sink = CaptureSink::default();
    let malformed = event_body(
        "evt_regression_1",
        "invoice.payment_failed",
        "tenant-1",
        "in_regression_1",
    );

    // (a) the malformed delivery is still rejected as a malformed
    // envelope, and nothing is dispatched.
    let err = h.handle(&malformed, &sink).expect_err("non-uuid tenant");
    assert!(matches!(err, BillingError::MalformedEnvelope), "got {err}");
    assert_eq!(sink.events().len(), 0, "nothing may be dispatched");

    // (b) the key was not recorded. The key is `(event.id, tenant_id)`
    // as a *string*, so the retry Stripe actually sends carries the
    // same non-UUID tenant: it must be re-validated and rejected
    // again, never answered `Duplicate`.
    assert!(
        !idem.has_seen("evt_regression_1", "tenant-1"),
        "a rejected envelope must not leave a key in the dedup table"
    );
    let retry = h
        .handle(&malformed, &sink)
        .expect_err("the retry must be re-validated, not deduped");
    assert!(
        matches!(retry, BillingError::MalformedEnvelope),
        "got {retry}"
    );

    // The event is not permanently lost: once Stripe sends the same
    // `event.id` with a well-formed tenant, it is processed rather
    // than silently dropped.
    let corrected = event_body(
        "evt_regression_1",
        "invoice.payment_failed",
        &Uuid::new_v4().to_string(),
        "in_regression_1",
    );
    assert_eq!(
        h.handle(&corrected, &sink).expect("corrected tenant"),
        WebhookOutcome::Accepted
    );
    assert_eq!(sink.events().len(), 1);
}
