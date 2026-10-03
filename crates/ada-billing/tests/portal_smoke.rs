//! `portal_smoke` — single-use Billing Portal session issuance.
//!
//! `PortalService` needs two upstream calls to satisfy one
//! `create_session`: the lazy `POST /v1/customers` (via
//! `CustomerService`) and then `POST /v1/billing_portal/sessions`.
//! Both are stubbed on a `wiremock` server so the whole flow runs
//! offline.

use std::sync::Arc;

use ada_billing::config::DEFAULT_STRIPE_BASE_URL;
use ada_billing::customer::CustomerRegistry;
use ada_billing::{BillingError, Config, CustomerId, CustomerService, PortalService, UserId};
use chrono::Utc;
use uuid::Uuid;
use wiremock::matchers::{body_string, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const SECRET_KEY: &str = "sk_test_smoke";

fn cfg_for(server: &MockServer, return_url: Option<&str>) -> Arc<Config> {
    let cfg = Config {
        stripe_secret_key: SECRET_KEY.to_owned(),
        stripe_webhook_secret: "whsec_smoke".into(),
        stripe_api_version: "2025-08-27.basil".into(),
        stripe_portal_return_url: return_url.map(str::to_owned),
        stripe_base_url: DEFAULT_STRIPE_BASE_URL.to_owned(),
    };
    Arc::new(cfg.with_base_url(format!("{}/v1", server.uri())))
}

fn service(cfg: Arc<Config>) -> PortalService {
    let customers = CustomerService::new(Arc::clone(&cfg), Arc::new(CustomerRegistry::new()));
    PortalService::new(cfg, customers)
}

/// `POST /v1/customers` stub for the lazy customer create.
async fn mount_customer_create(server: &MockServer, customer_id: &str) {
    Mock::given(method("POST"))
        .and(path("/v1/customers"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"id": customer_id, "object": "customer"})),
        )
        .mount(server)
        .await;
}

/// `POST /v1/billing_portal/sessions` stub for the session itself.
async fn mount_portal_session(server: &MockServer, portal_url: &str) {
    Mock::given(method("POST"))
        .and(path("/v1/billing_portal/sessions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"id": "bps_smoke_1", "url": portal_url})),
        )
        .mount(server)
        .await;
}

/// Both stubs the happy path needs.
async fn mount_happy_path(server: &MockServer, customer_id: &str, portal_url: &str) {
    mount_customer_create(server, customer_id).await;
    mount_portal_session(server, portal_url).await;
}

#[tokio::test]
async fn a_session_url_is_returned_with_a_one_hour_expiry() {
    let server = MockServer::start().await;
    mount_happy_path(
        &server,
        "cus_portal_1",
        "https://billing.stripe.com/s/smoke",
    )
    .await;
    let svc = service(cfg_for(&server, Some("https://app.example.com/billing")));

    let before = Utc::now().timestamp();
    let session = svc
        .create_session(UserId(Uuid::new_v4()), "[email protected]")
        .await
        .expect("create session");
    let after = Utc::now().timestamp();

    assert_eq!(session.url, "https://billing.stripe.com/s/smoke");
    assert_eq!(session.customer_id, CustomerId("cus_portal_1".into()));
    // Stripe enforces the 1 h TTL; the service mirrors it.
    assert!(
        session.expires_at_unix >= before + 3600 && session.expires_at_unix <= after + 3600,
        "expiry {} is not one hour after the call",
        session.expires_at_unix
    );
}

#[tokio::test]
async fn the_session_request_carries_the_customer_and_the_configured_return_url() {
    let server = MockServer::start().await;
    let return_url = "https://app.example.com/billing";
    mount_customer_create(&server, "cus_portal_2").await;

    // If the form body regresses, this stub stops matching and the
    // request 404s, failing the call below.
    Mock::given(method("POST"))
        .and(path("/v1/billing_portal/sessions"))
        .and(body_string(format!(
            "customer=cus_portal_2&return_url={return_url}"
        )))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"url": "https://billing.stripe.com/s/two"})),
        )
        .mount(&server)
        .await;

    let svc = service(cfg_for(&server, Some(return_url)));
    let session = svc
        .create_session(UserId(Uuid::new_v4()), "[email protected]")
        .await
        .expect("stub matched the request body");
    assert_eq!(session.url, "https://billing.stripe.com/s/two");

    // One customer create + one portal session, in that order.
    let requests = server.received_requests().await.expect("recorded requests");
    let created = requests
        .iter()
        .filter(|r| r.url.path() == "/v1/customers")
        .count();
    let sessions = requests
        .iter()
        .filter(|r| r.url.path() == "/v1/billing_portal/sessions")
        .count();
    assert_eq!(created, 1);
    assert_eq!(sessions, 1);
}

#[tokio::test]
async fn an_unset_return_url_falls_back_to_the_billing_path() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/customers"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "cus_portal_3"})),
        )
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/billing_portal/sessions"))
        .and(body_string("customer=cus_portal_3&return_url=/billing"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"url": "https://billing.stripe.com/s/three"})),
        )
        .mount(&server)
        .await;

    let svc = service(cfg_for(&server, None));
    let session = svc
        .create_session(UserId(Uuid::new_v4()), "[email protected]")
        .await
        .expect("default return_url is /billing");
    assert_eq!(session.url, "https://billing.stripe.com/s/three");
}

#[tokio::test]
async fn a_second_call_within_the_ttl_is_served_from_the_cache() {
    let server = MockServer::start().await;
    mount_happy_path(&server, "cus_portal_4", "https://billing.stripe.com/s/four").await;
    let svc = service(cfg_for(&server, Some("https://app.example.com/billing")));
    let uid = UserId(Uuid::new_v4());

    let first = svc
        .create_session(uid, "[email protected]")
        .await
        .expect("first session");
    let second = svc
        .create_session(uid, "[email protected]")
        .await
        .expect("cached session");

    assert_eq!(first.url, second.url);
    assert_eq!(first.expires_at_unix, second.expires_at_unix);
    assert_eq!(first.customer_id, second.customer_id);

    let requests = server.received_requests().await.expect("recorded requests");
    let sessions = requests
        .iter()
        .filter(|r| r.url.path() == "/v1/billing_portal/sessions")
        .count();
    assert_eq!(
        sessions, 1,
        "the cached session must not trigger a second Stripe call"
    );
    assert_eq!(requests.len(), 2, "1 customer create + 1 portal session");
}

#[tokio::test]
async fn a_stripe_error_status_is_propagated_as_a_billing_error() {
    let server = MockServer::start().await;
    // The customer create succeeds; the portal call fails.
    Mock::given(method("POST"))
        .and(path("/v1/customers"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "cus_portal_5"})),
        )
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/billing_portal/sessions"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;

    let svc = service(cfg_for(&server, Some("https://app.example.com/billing")));
    let err = svc
        .create_session(UserId(Uuid::new_v4()), "[email protected]")
        .await
        .expect_err("500 must not succeed");
    assert!(
        matches!(err, BillingError::StripeApi(500)),
        "unexpected error: {err}"
    );
}
