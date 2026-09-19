//! Customer mapping: `UserId ↔ Stripe customer.id`.
//!
//! In production the mapping is backed by Postgres; the v0.4.0
//! skeleton uses an in-process `parking_lot::RwLock<HashMap<…>>` so
//! the api-gateway's `BillingActor` calls can be wired without a DB.
//! See `auth-billing-arch.md` §5 for the production wiring.

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use ada_core::UserId;
use reqwest::Client;

use crate::config::Config;
use crate::error::{BillingError, Result};

/// Stripe customer identifier (`cus_…`). Newtype to prevent accidental
/// mixing with arbitrary strings.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CustomerId(pub String);

impl core::fmt::Display for CustomerId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A tenant's billing customer record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Customer {
    pub user_id: UserId,
    pub stripe_customer_id: Option<CustomerId>,
}

impl Customer {
    #[must_use]
    pub fn new(user_id: UserId) -> Self {
        Self {
            user_id,
            stripe_customer_id: None,
        }
    }
}

/// In-process customer registry. Thread-safe.
#[derive(Debug, Default)]
pub struct CustomerRegistry {
    by_user: RwLock<HashMap<UserId, Customer>>,
}

impl CustomerRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn upsert(&self, customer: Customer) {
        self.by_user.write().insert(customer.user_id, customer);
    }

    #[must_use]
    pub fn get(&self, user_id: UserId) -> Option<Customer> {
        self.by_user.read().get(&user_id).cloned()
    }

    pub fn ensure(&self, user_id: UserId) -> Customer {
        let mut w = self.by_user.write();
        w.entry(user_id).or_insert_with(|| Customer::new(user_id)).clone()
    }
}

/// Customer service: REST wrapper + registry. Holds the
/// `STRIPE_SECRET_KEY` (never logged) and the `reqwest::Client`.
#[derive(Debug, Clone)]
pub struct CustomerService {
    cfg: Arc<Config>,
    http: Client,
    registry: Arc<CustomerRegistry>,
}

impl CustomerService {
    #[must_use]
    pub fn new(cfg: Arc<Config>, registry: Arc<CustomerRegistry>) -> Self {
        let http = Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("reqwest client build");
        Self { cfg, http, registry }
    }

    /// Look up the local customer; lazily create + persist it on
    /// first call. The Stripe-side create is idempotent: we use the
    /// `idempotency-key` header keyed by the user's UUID so retries
    /// do not double-bill.
    pub async fn get_or_create(&self, user_id: UserId, email: &str) -> Result<Customer> {
        if let Some(existing) = self.registry.get(user_id) {
            if existing.stripe_customer_id.is_some() {
                return Ok(existing);
            }
        }
        // POST https://api.stripe.com/v1/customers with form-encoded
        // body. We don't carry the parsed response back — the test
        // suite uses wiremock to verify the request shape; in
        // production the api-gateway's BillingActor is the trusted
        // caller.
        let url = format!("{}/customers", self.cfg.stripe_base_url);
        let body = format!("email={email}");
        let idempotency = user_id.to_string();
        let resp = self
            .http
            .post(&url)
            .bearer_auth(&self.cfg.stripe_secret_key)
            .header("Stripe-Version", &self.cfg.stripe_api_version)
            .header("Idempotency-Key", idempotency)
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
        let id = body
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or(BillingError::MalformedEnvelope)?
            .to_owned();
        let mut customer = self.registry.ensure(user_id);
        customer.stripe_customer_id = Some(CustomerId(id));
        self.registry.upsert(customer.clone());
        Ok(customer)
    }

    /// Look up a known customer. Returns `None` if no Stripe mapping
    /// has been established yet.
    #[must_use]
    pub fn lookup(&self, user_id: UserId) -> Option<Customer> {
        self.registry.get(user_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_upsert_get_roundtrip() {
        let reg = CustomerRegistry::new();
        let uid = UserId(uuid::Uuid::new_v4());
        let c = reg.ensure(uid);
        assert_eq!(c.user_id, uid);
        assert!(c.stripe_customer_id.is_none());

        let mut c2 = c.clone();
        c2.stripe_customer_id = Some(CustomerId("cus_test_1".into()));
        reg.upsert(c2.clone());
        assert_eq!(reg.get(uid).and_then(|c| c.stripe_customer_id), c2.stripe_customer_id);
    }

    #[test]
    fn customer_id_display() {
        let id = CustomerId("cus_abc".into());
        assert_eq!(id.to_string(), "cus_abc");
    }
}