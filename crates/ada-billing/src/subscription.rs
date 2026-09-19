//! Subscription state machine.
//!
//! Mirrors the seven Stripe states verbatim (`active`, `past_due`,
//! `canceled`, `trialing`, `incomplete`, `incomplete_expired`,
//! `unpaid`). Transitions are validated locally before any REST call
//! to Stripe — this gives the api-gateway a way to reject illegal
//! transitions without round-tripping.

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use ada_core::TenantId;
use ada_core::UserId;

use crate::error::{BillingError, Result};
use crate::plan::Plan;

/// The seven Stripe subscription states (verbatim).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SubscriptionStatus {
    Active,
    PastDue,
    Canceled,
    Trialing,
    Incomplete,
    IncompleteExpired,
    Unpaid,
}

impl SubscriptionStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::PastDue => "past_due",
            Self::Canceled => "canceled",
            Self::Trialing => "trialing",
            Self::Incomplete => "incomplete",
            Self::IncompleteExpired => "incomplete_expired",
            Self::Unpaid => "unpaid",
        }
    }

    /// True iff the subscription is currently usable (entitlement is
    /// honored). `PastDue` is still usable but typically triggers a
    /// grace-period banner.
    #[must_use]
    pub const fn is_entitled(self) -> bool {
        matches!(self, Self::Active | Self::Trialing | Self::PastDue)
    }
}

impl core::fmt::Display for SubscriptionStatus {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl core::str::FromStr for SubscriptionStatus {
    type Err = BillingError;
    fn from_str(s: &str) -> Result<Self> {
        Ok(match s {
            "active" => Self::Active,
            "past_due" => Self::PastDue,
            "canceled" => Self::Canceled,
            "trialing" => Self::Trialing,
            "incomplete" => Self::Incomplete,
            "incomplete_expired" => Self::IncompleteExpired,
            "unpaid" => Self::Unpaid,
            _ => return Err(BillingError::InvalidPayload),
        })
    }
}

/// One tenant's current subscription.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub tenant_id: TenantId,
    pub plan: Plan,
    pub status: SubscriptionStatus,
    /// Stripe subscription id (`sub_…`). `None` for the Free plan.
    pub stripe_subscription_id: Option<String>,
    /// Unix epoch seconds when the current period ends.
    pub current_period_end_unix: Option<i64>,
}

impl Subscription {
    #[must_use]
    pub fn free(tenant_id: TenantId) -> Self {
        Self {
            tenant_id,
            plan: Plan::Free,
            status: SubscriptionStatus::Active,
            stripe_subscription_id: None,
            current_period_end_unix: None,
        }
    }
}

/// Validate a state transition. Returns `Ok(())` if the transition
/// is legal; `Err(BillingError::StripeApi(...))` otherwise. We
/// reuse `StripeApi(409)` for illegal transitions to keep the
/// variant count narrow.
pub fn validate_transition(
    from: SubscriptionStatus,
    to: SubscriptionStatus,
) -> Result<()> {
    use SubscriptionStatus::*;
    let ok = matches!(
        (from, to),
        // The four "in-progress" states can advance into the
        // terminal/active states.
        (Incomplete, Active)
            | (Incomplete, IncompleteExpired)
            | (Incomplete, Canceled)
            | (Trialing, Active)
            | (Trialing, Canceled)
            | (Active, PastDue)
            | (Active, Canceled)
            | (Active, Unpaid)
            | (PastDue, Active)
            | (PastDue, Canceled)
            | (PastDue, Unpaid)
            | (Unpaid, Active)
            | (Unpaid, Canceled)
            // Self-loops are no-ops (e.g. webhook replays).
            | _ if from == to
    );
    if ok {
        Ok(())
    } else {
        Err(BillingError::StripeApi(409))
    }
}

/// In-process subscription registry, keyed by `TenantId`. Production
/// wiring backs this with Postgres.
#[derive(Debug, Default)]
pub struct SubscriptionRegistry {
    by_tenant: RwLock<HashMap<TenantId, Subscription>>,
}

impl SubscriptionRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn upsert(&self, sub: Subscription) {
        self.by_tenant.write().insert(sub.tenant_id, sub);
    }

    #[must_use]
    pub fn get(&self, tenant_id: TenantId) -> Option<Subscription> {
        self.by_tenant.read().get(&tenant_id).cloned()
    }
}

/// High-level subscription service. Holds the registry; webhook
/// handlers call `apply_transition` to update state.
#[derive(Debug, Clone)]
pub struct SubscriptionService {
    registry: Arc<SubscriptionRegistry>,
}

impl SubscriptionService {
    #[must_use]
    pub fn new(registry: Arc<SubscriptionRegistry>) -> Self {
        Self { registry }
    }

    pub fn apply_transition(
        &self,
        tenant_id: TenantId,
        to: SubscriptionStatus,
        plan: Plan,
        stripe_id: Option<String>,
        period_end_unix: Option<i64>,
    ) -> Result<()> {
        let prev = self.registry.get(tenant_id);
        if let Some(prev) = prev {
            validate_transition(prev.status, to)?;
        }
        let next = Subscription {
            tenant_id,
            plan,
            status: to,
            stripe_subscription_id: stripe_id,
            current_period_end_unix: period_end_unix,
        };
        self.registry.upsert(next);
        Ok(())
    }

    #[must_use]
    pub fn current(&self, tenant_id: TenantId) -> Option<Subscription> {
        self.registry.get(tenant_id)
    }

    /// Lookup a subscription by the user that owns the tenant. For
    /// v0.4.0 the UserId and TenantId share the same id space at
    /// creation time (the registry's per-user customer record is
    /// the source of truth); the SubscriptionService only needs
    /// TenantId at the API surface.
    #[must_use]
    pub fn current_for_user(&self, _user_id: UserId, tenant_id: TenantId) -> Option<Subscription> {
        self.registry.get(tenant_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_from_str_roundtrip() {
        for s in [
            SubscriptionStatus::Active,
            SubscriptionStatus::PastDue,
            SubscriptionStatus::Canceled,
            SubscriptionStatus::Trialing,
            SubscriptionStatus::Incomplete,
            SubscriptionStatus::IncompleteExpired,
            SubscriptionStatus::Unpaid,
        ] {
            let txt = s.as_str();
            let back: SubscriptionStatus = txt.parse().expect("parse");
            assert_eq!(back, s);
        }
    }

    #[test]
    fn status_from_str_unknown_errors() {
        let r: Result<SubscriptionStatus> = "no-such-status".parse();
        assert!(matches!(r, Err(BillingError::InvalidPayload)));
    }

    #[test]
    fn entitled_statuses() {
        assert!(SubscriptionStatus::Active.is_entitled());
        assert!(SubscriptionStatus::Trialing.is_entitled());
        assert!(SubscriptionStatus::PastDue.is_entitled());
        assert!(!SubscriptionStatus::Canceled.is_entitled());
        assert!(!SubscriptionStatus::Incomplete.is_entitled());
    }

    #[test]
    fn legal_transitions_are_allowed() {
        // Active <-> PastDue <-> Unpaid are all legal.
        assert!(validate_transition(SubscriptionStatus::Active, SubscriptionStatus::PastDue).is_ok());
        assert!(validate_transition(SubscriptionStatus::PastDue, SubscriptionStatus::Active).is_ok());
        assert!(validate_transition(SubscriptionStatus::Active, SubscriptionStatus::Canceled).is_ok());
        // Self-loop is always legal (replays).
        assert!(validate_transition(SubscriptionStatus::Active, SubscriptionStatus::Active).is_ok());
    }

    #[test]
    fn illegal_transitions_rejected() {
        // Canceled is terminal.
        let r = validate_transition(SubscriptionStatus::Canceled, SubscriptionStatus::Active);
        assert!(r.is_err());
    }

    #[test]
    fn apply_transition_roundtrip() {
        let reg = Arc::new(SubscriptionRegistry::new());
        let svc = SubscriptionService::new(Arc::clone(&reg));
        let tenant = TenantId(uuid::Uuid::new_v4());
        svc.apply_transition(
            tenant,
            SubscriptionStatus::Active,
            Plan::Team,
            Some("sub_test".into()),
            Some(1_700_000_000),
        )
        .expect("transition");
        let got = svc.current(tenant).expect("current");
        assert_eq!(got.status, SubscriptionStatus::Active);
        assert_eq!(got.plan, Plan::Team);
    }
}