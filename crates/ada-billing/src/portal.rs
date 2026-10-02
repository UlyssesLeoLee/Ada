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
    /// The single-use Stripe-hosted portal URL.
    pub url: String,
    /// The Stripe customer the session belongs to.
    pub customer_id: CustomerId,
    /// Unix epoch seconds after which the session is no longer served
    /// from cache (Stripe's own TTL is 1 h).
    pub expires_at_unix: i64,
}

/// In-process portal-session cache.
///
/// Sessions carry Stripe's own 1 h TTL, and the read path already treats a
/// lapsed entry as absent — it falls through to issuing a fresh one. So a
/// lapsed entry is not just stale, it is *unusable*, and keeping it serves
/// no purpose beyond consuming memory.
#[derive(Debug, Default)]
struct PortalCache {
    by_user: RwLock<std::collections::HashMap<String, PortalSession>>,
}

impl PortalCache {
    /// Store `session`, first dropping every entry already past its TTL.
    /// Returns the session so the caller can hand it straight back.
    ///
    /// Reclaiming here is safe in a way it would emphatically **not** be
    /// for [`crate::webhook::IdempotencyStore`], and the difference is
    /// worth stating because both are in-process tables that someone will
    /// eventually try to bound the same way:
    ///
    /// * A portal session past `expires_at_unix` is already dead. The
    ///   lookup at the top of `create_session` ignores it, so removing it
    ///   changes nothing a caller can observe. The worst outcome of being
    ///   wrong is one extra Stripe API call, which is exactly what that
    ///   customer would have caused anyway.
    /// * An idempotency key is *not* dead when its moment passes. It has
    ///   to outlive Stripe's retry window or a late retry gets processed a
    ///   second time. Bounding that table needs a retention decision, not
    ///   a sweep.
    ///
    /// Sweeping on the write path only is deliberate: writes are the only
    /// way this map grows, so sweeping anywhere else would cost a lock
    /// acquisition and reclaim nothing.
    fn store(&self, session: PortalSession, now: i64) -> PortalSession {
        let mut w = self.by_user.write();
        w.retain(|_, cached| cached.expires_at_unix > now);
        w.insert(session.customer_id.0.clone(), session.clone());
        session
    }
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
    /// Build a portal service with its own 10 s-timeout HTTP client
    /// and an empty session cache.
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
        let cid = customer
            .stripe_customer_id
            .clone()
            .ok_or(BillingError::MalformedEnvelope)?;
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
        // Storing also reclaims lapsed sessions. The entry this call is
        // about to write is the only reason the map grew, so this is the
        // one place a sweep can pay for itself.
        Ok(self.cache.store(session, now))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn portal_cfg() -> Arc<Config> {
        Arc::new(Config {
            stripe_secret_key: "sk_test_dummy".into(),
            stripe_webhook_secret: "whsec_dummy".into(),
            stripe_api_version: "2025-08-27.basil".into(),
            stripe_portal_return_url: Some("https://app.example.com/billing".into()),
            stripe_base_url: "https://api.stripe.com/v1".into(),
        })
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

    fn session(customer: &str, expires_at: i64) -> PortalSession {
        PortalSession {
            url: format!("https://billing.stripe.com/s/{customer}"),
            customer_id: CustomerId(customer.to_owned()),
            expires_at_unix: expires_at,
        }
    }

    /// Insert WITHOUT sweeping.
    ///
    /// This exists to build the state the cache used to accumulate, and it
    /// has to bypass `store` to do that: `store` reclaims on every write,
    /// so seeding a lapsed entry through it would sweep the previous one
    /// away and the fixture would never hold more than one entry. A test
    /// that says "four entries are held" cannot get there by calling the
    /// very method that makes fewer than four entries possible.
    fn seed_lapsed(cache: &PortalCache, customer: &str, expires_at: i64) {
        cache
            .by_user
            .write()
            .insert(customer.to_owned(), session(customer, expires_at));
    }

    /// The cache used to insert and never remove, so it grew to one entry
    /// per customer who had *ever* opened a portal and handed none of it
    /// back — including the lapsed ones the lookup had already stopped
    /// honouring.
    ///
    /// Asserting the count alone would be too weak: a sweep that dropped
    /// the *live* entry too would also leave two, so the identities are
    /// checked as well.
    #[test]
    fn storing_a_session_reclaims_the_lapsed_ones() {
        let cache = PortalCache::default();
        let now = 1_700_000_000i64;

        seed_lapsed(&cache, "cus_stale_0", now - 1);
        seed_lapsed(&cache, "cus_stale_1", now - 3_600);
        seed_lapsed(&cache, "cus_stale_2", now - 10);
        seed_lapsed(&cache, "cus_live", now + 3_600);
        assert_eq!(
            cache.by_user.read().len(),
            4,
            "precondition: four entries are held, three of them lapsed"
        );

        cache.store(session("cus_new", now + 3_600), now);

        assert_eq!(
            cache.by_user.read().len(),
            2,
            "the three lapsed sessions must be reclaimed, leaving the two \
             live ones"
        );
        let w = cache.by_user.read();
        assert!(
            w.contains_key("cus_live"),
            "a session inside its TTL must survive the sweep"
        );
        assert!(
            w.contains_key("cus_new"),
            "the session just written must be present"
        );
        for i in 0..3 {
            assert!(
                !w.contains_key(&format!("cus_stale_{i}")),
                "a lapsed session must not be retained"
            );
        }
    }

    /// A session expiring exactly at `now` is gone; one a second short of
    /// it is not. The comparison is `>`, and the boundary is the whole
    /// reason a sweep cannot be coarser than "strictly greater".
    #[test]
    fn the_sweep_boundary_is_strict() {
        let cache = PortalCache::default();
        let now = 1_700_000_000i64;

        seed_lapsed(&cache, "cus_expiring", now + 1);
        seed_lapsed(&cache, "cus_gone", now);
        seed_lapsed(&cache, "cus_fresh", now + 3_600);
        assert_eq!(
            cache.by_user.read().len(),
            3,
            "precondition: all three are held"
        );

        cache.store(session("cus_trigger", now + 3_600), now);
        let w = cache.by_user.read();
        assert!(
            w.contains_key("cus_expiring"),
            "expires_at_unix == now + 1 is still live"
        );
        assert!(
            !w.contains_key("cus_gone"),
            "expires_at_unix == now is not live; `>` is strict"
        );
    }
}
