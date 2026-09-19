//! Stripe Billing Portal session URL generation.
//!
//! `POST /v1/billing_portal/sessions` with `customer` + `return_url`
//! returns a single-use session URL (1 h expiry enforced by Stripe).
//! The api-gateway exposes this as `GET /api/v1/billing/portal`.

use std::sync::Arc;
use std::time::Duration;

use parking_lot::RwLock;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::customer::{CustomerId, CustomerService};
use crate::error::{BillingError, Result};

/// A short-lived portal session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortalSession {
    pub url: String,
    pub customer_id: CustomerId,
    pub expires_at_unix: i64,
}

#[derive(Debug, Default)]
struct PortalCache {
    by_user: RwLock<std::collections::HashMap<String, PortalSession>>,
}

/// Portal service: REST wrapper + tiny in-process cache.
#[derive(Debug, Clone)]
pub struct PortalService {
    cfg: Arc<Config>,
    http: Client,
    customers: CustomerService,
    cache: Arc<PortalCache>,
}

impl PortalService {
    #[must_use]
    pub fn new(cfg: Arc<Config>, customers: CustomerService) -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("reqwest client build");
        Self {
            cfg,
            http,
            customers,
            cache: Arc::new(PortalCache::default()),
        }
    }

    /// Look up or create a portal session URL for `user`. The cache
    /// is keyed by `customer_id` and stores sessions for their full
    /// 1 h TTL; after expiry we issue a fresh one.
    pub async fn create_session(
        &self,
        user_id: ada_core::UserId,
        email: &str,
    ) -> Result<PortalSession> {
        let customer = self.customers.get_or_create(user_id, email).await?;
        let cid = customer.stripe_customer_id.clone().ok_or(BillingError::MalformedEnvelope)?;
        let now = chrono::Utc::now().timestamp();
        if let Some(cached) = self.cache.by_user.read().get(&cid.0).cloned() {
            if cached.expires_at_unix > now {
                return Ok(cached);
            }
        }
        let url = format!("{}/billing_portal/sessions", self.cfg.stripe_base_url);
        let body = format!(
            "customer={}&return_url={}",
            cid.0,
            self.cfg
                .stripe_portal_return_url
                .as_deref()
                .unwrap_or("/billing"),
        );
        let resp = self
            .http
            .post(&url)
            .bearer_auth(&self.cfg.stripe_secret_key)
            .header("Stripe-Version", &self.cfg.stripe_api_version)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .map_err(|e| BillingError::Transport(e.to_string()))?;
        let status = resp.status().as_u16();
        if !(200..300).contains(&status) {
            return Err(BillingError::StripeApi(status));
        }
        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|_| BillingError::InvalidPayload)?;
        let url = body
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or(BillingError::MalformedEnvelope)?
            .to_owned();
        let session = PortalSession {
            url,
            customer_id: cid,
            expires_at_unix: now + 3600,
        };
        self.cache.by_user.write().insert(session.customer_id.0.clone(), session.clone());
        Ok(session)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn portal_cfg() -> Arc<Config> {
        Arc::new(
            Config {
                stripe_secret_key: "sk_test_dummy".into(),
                stripe_webhook_secret: "whsec_dummy".into(),
                stripe_api_version: "2025-08-27.basil".into(),
                stripe_portal_return_url: Some("https://app.example.com/billing".into()),
                stripe_base_url: "https://api.stripe.com/v1".into(),
            },
        )
    }

    #[test]
    fn portal_session_display_fields_round_trip() {
        let s = PortalSession {
            url: "https://billing.stripe.com/s/test".into(),
            customer_id: CustomerId("cus_test".into()),
            expires_at_unix: 1_700_000_000,
        };
        let json = serde_json::to_string(&s).expect("serialize");
        let back: PortalSession = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.url, s.url);
        assert_eq!(back.customer_id, s.customer_id);
        assert_eq!(back.expires_at_unix, s.expires_at_unix);
    }

    #[test]
    fn portal_service_constructs() {
        let cfg = portal_cfg();
        let registry = Arc::new(crate::customer::CustomerRegistry::new());
        let customers = CustomerService::new(Arc::clone(&cfg), registry);
        let _svc = PortalService::new(cfg, customers);
    }
}