//! `customer_smoke` — `UserId ↔ Stripe customer.id` round-trip.
//!
//! Drives `CustomerService` against a `wiremock` Stripe endpoint: the
//! form-encoded `POST /v1/customers` request shape, the lazy-create +
//! registry-cache behaviour, and the error mapping for non-2xx and
//! malformed responses. No network access — every HTTP call goes to the
//! mock server.

use std::sync::Arc;

use ada_billing::config::DEFAULT_STRIPE_BASE_URL;
use ada_billing::customer::CustomerRegistry;
use ada_billing::{BillingError, Config, CustomerId, CustomerService, UserId};
use uuid::Uuid;
use wiremock::matchers::{body_string, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const SECRET_KEY: &str = "sk_test_smoke";
const API_VERSION: &str = "2025-08-27.basil";

/// Config pointed at the mock server. `with_base_url` is the
/// documented test-only override; the `/v1` suffix keeps the request
/// paths identical to the real Stripe REST API.
fn cfg_for(server: &MockServer) -> Arc<Config> {
    let cfg = Config {
        stripe_secret_key: SECRET_KEY.to_owned(),
        stripe_webhook_secret: "whsec_smoke".into(),
        stripe_api_version: API_VERSION.to_owned(),
        stripe_portal_return_url: None,
        stripe_base_url: DEFAULT_STRIPE_BASE_URL.to_owned(),
    };
    Arc::new(cfg.with_base_url(format!("{}/v1", server.uri())))
}

fn service(cfg: Arc<Config>) -> CustomerService {
    CustomerService::new(cfg, Arc::new(CustomerRegistry::new()))
}

fn header_of(req: &wiremock::Request, name: &str) -> String {
    req.headers
        .get(name)
        .unwrap_or_else(|| panic!("missing `{name}` header"))
        .to_str()
        .expect("header is utf8")
        .to_owned()
}

/// `POST /v1/customers` stub answering with a Stripe-shaped customer.
async fn mount_customer_create(server: &MockServer, id: &str) {
    Mock::given(method("POST"))
        .and(path("/v1/customers"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": id,
            "object": "customer",
        })))
        .mount(server)
        .await;
}

#[tokio::test]
async fn get_or_create_stores_the_stripe_id_and_lookup_reads_it_back() {
    let server = MockServer::start().await;
    mount_customer_create(&server, "cus_smoke_1").await;
    let svc = service(cfg_for(&server));
    let uid = UserId(Uuid::new_v4());

    // No mapping exists before the first call.
    assert!(
        svc.lookup(uid).is_none(),
        "lookup must miss before creation"
    );

    let customer = svc
        .get_or_create(uid, "[email protected]")
        .await
        .expect("create customer");

    assert_eq!(customer.user_id, uid);
    assert_eq!(
        customer.stripe_customer_id,
        Some(CustomerId("cus_smoke_1".into()))
    );
    // The mapping is readable through `lookup` without another call.
    assert_eq!(svc.lookup(uid), Some(customer));
}

#[tokio::test]
async fn request_shape_is_form_encoded_with_bearer_auth_and_idempotency_key() {
    let email = "[email protected]";
    let server = MockServer::start().await;
    let uid = UserId(Uuid::new_v4());

    // Matching on the body and headers means a regression in the
    // outbound request shape leaves the stub unmatched (404) and the
    // call fails, rather than silently passing.
    Mock::given(method("POST"))
        .and(path("/v1/customers"))
        .and(body_string(format!("email={email}")))
        .and(header("authorization", format!("Bearer {SECRET_KEY}")))
        .and(header("stripe-version", API_VERSION))
        .and(header(
            "idempotency-key",
            // `UserId`'s Display already renders `user(<uuid>)` — that
            // Display output *is* the header value, so do not wrap it
            // in a second `user(...)`.
            uid.to_string(),
        ))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "cus_smoke_2"})),
        )
        .mount(&server)
        .await;

    let svc = service(cfg_for(&server));
    let customer = svc
        .get_or_create(uid, email)
        .await
        .expect("stub matched the request shape");

    assert_eq!(
        customer.stripe_customer_id,
        Some(CustomerId("cus_smoke_2".into()))
    );

    let requests = server.received_requests().await.expect("recorded requests");
    assert_eq!(requests.len(), 1);
    let req = &requests[0];
    assert_eq!(req.method.as_str(), "POST");
    assert_eq!(req.url.path(), "/v1/customers");
    assert_eq!(String::from_utf8_lossy(&req.body), format!("email={email}"));
    assert_eq!(
        header_of(req, "content-type"),
        "application/x-www-form-urlencoded"
    );
}

#[tokio::test]
async fn repeated_get_or_create_is_served_from_the_registry() {
    let server = MockServer::start().await;
    mount_customer_create(&server, "cus_smoke_3").await;
    let svc = service(cfg_for(&server));
    let uid = UserId(Uuid::new_v4());

    let first = svc
        .get_or_create(uid, "[email protected]")
        .await
        .expect("first create");
    let second = svc
        .get_or_create(uid, "[email protected]")
        .await
        .expect("second create");

    assert_eq!(first.stripe_customer_id, second.stripe_customer_id);
    let requests = server.received_requests().await.expect("recorded requests");
    assert_eq!(
        requests.len(),
        1,
        "the second call must be served from the registry, not re-POSTed"
    );
}

#[tokio::test]
async fn non_2xx_status_maps_to_stripe_api_error_and_persists_nothing() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/customers"))
        .respond_with(ResponseTemplate::new(402))
        .mount(&server)
        .await;
    let svc = service(cfg_for(&server));
    let uid = UserId(Uuid::new_v4());

    let err = svc
        .get_or_create(uid, "[email protected]")
        .await
        .expect_err("402 must not succeed");
    assert!(
        matches!(err, BillingError::StripeApi(402)),
        "unexpected error: {err}"
    );
    assert!(
        svc.lookup(uid).is_none(),
        "a failed create must not persist a customer mapping"
    );
}

#[tokio::test]
async fn response_without_an_id_is_rejected_as_a_malformed_envelope() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/customers"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"object": "customer"})),
        )
        .mount(&server)
        .await;
    let svc = service(cfg_for(&server));

    let err = svc
        .get_or_create(UserId(Uuid::new_v4()), "[email protected]")
        .await
        .expect_err("a response without `id` must not succeed");
    assert!(
        matches!(err, BillingError::MalformedEnvelope),
        "unexpected error: {err}"
    );
}
