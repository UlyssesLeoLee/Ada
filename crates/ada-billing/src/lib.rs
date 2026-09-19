//! `ada-billing` — Stripe Billing integration for Ada v0.4.0.
//!
//! This crate implements the binding contract in
//! `docs/commercial/auth-billing-arch.md` §5 ("Billing (Stripe)").
//! It is a **skeleton**-layer module: the trait surfaces and HTTP
//! routing are stable, but the persistent state (Stripe customer ID
//! mapping, subscription row, idempotency table) is held in
//! in-process maps so that the test suite and the api-gateway's
//! `BillingActor` calls can be wired without a database.
//!
//! ## Module surface
//!
//! - [`customer`] — `UserId → Stripe customer.id` mapping; create +
//!   fetch round-trip via the Stripe REST API.
//! - [`subscription`] — subscription state machine over the seven
//!   Stripe states (`active`, `past_due`, `canceled`, `trialing`,
//!   `incomplete`, `incomplete_expired`, `unpaid`) with explicit
//!   transition validation.
//! - [`webhook`] — `POST /webhooks/stripe` handler: signature
//!   verification, idempotency table keyed by `event.id + tenant_id`,
//!   audit-log emission, downstream `BillingEvent` channel.
//! - [`portal`] — `GET /api/v1/billing/portal` returns a single-use
//!   Stripe Billing Portal session URL (1 h expiry).
//! - [`entitlement`] — `Entitlement::for(user, tenant) -> Entitlement`
//!   reads subscription + plan tier, exposes `can_use(Feature)`.
//! - [`plan`] — `Plan` enum (Free / Team / Enterprise) with the
//!   static Stripe Price ID map.
//!
//! ## What this crate does **not** do
//!
//! - Persist audit entries (the sink is `InMemoryAuditSink`; the
//!   production wiring goes through the same `record_audit_log` fn
//!   that `ada-m11-rbac-collab` exports).
//! - Verify the OIDC JWT (that is the api-gateway's job; we receive
//!   a `BillingActor`).
//! - Back the customer / subscription map with Postgres.
//!
//! ## Stripe SDK version
//!
//! Per the brief we pin `stripe-rust` (the canonical crate name) at
//! `0.12`, with the `webhook-events` feature for
//! `Webhook::construct_event`. The maintained successor is published
//! as `async-stripe`; a future v0.5.x may swap the dep when
//! edition2024 lands in `rust-version`.
//!
//! License: MIT (matches the workspace D-13).

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::module_name_repetitions)]

pub mod config;
pub mod customer;
pub mod entitlement;
pub mod error;
pub mod plan;
pub mod portal;
pub mod subscription;
pub mod webhook;

pub use ada_core::{TenantId, UserId};
pub use config::Config;
pub use customer::{Customer, CustomerId, CustomerService};
pub use entitlement::{Entitlement, Feature};
pub use error::{BillingError, Result};
pub use plan::Plan;
pub use portal::{PortalSession, PortalService};
pub use subscription::{Subscription, SubscriptionService, SubscriptionStatus};
pub use webhook::{
    BillingEvent, EventSink, IdempotencyStore, WebhookHandler, WebhookOutcome, WebhookService,
};

/// Crate version, taken from `CARGO_PKG_VERSION` (single workspace
/// version per D-09).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Crate name, taken from `CARGO_PKG_NAME`.
pub const NAME: &str = env!("CARGO_PKG_NAME");

/// `skeleton`-layer string tag (仿生モデル 4 層分類).
pub const LAYER: &str = "skeleton";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_not_empty() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn name_not_empty() {
        assert!(!NAME.is_empty());
    }

    #[test]
    fn layer_is_known() {
        assert!(
            ["skeleton", "blood", "nerve", "muscle", "shared"].contains(&LAYER),
            "Unknown layer: {LAYER}"
        );
    }
}