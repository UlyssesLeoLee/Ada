//! Entitlement surface: read subscription + plan tier, expose
//! `can_use(Feature)` for the api-gateway middleware.
//!
//! Plan tier matrix (per `auth-billing-arch.md` §5):
//!
//! | Plan         | Tenants | Pipelines     | Audit retention | SSO required |
//! |--------------|---------|---------------|-----------------|--------------|
//! | `Free`       | 1       | 5             | 14 days         | no           |
//! | `Team`       | 10      | unlimited     | 90 days         | no           |
//! | `Enterprise` | custom  | unlimited     | 365 days        | yes          |

use std::sync::Arc;

use ada_core::{TenantId, UserId};

use crate::error::Result;
use crate::plan::Plan;
use crate::subscription::{Subscription, SubscriptionService, SubscriptionStatus};

/// The product-level feature gates the api-gateway consults. Adding
/// a new gate means adding a variant here + the plan matrix in
/// [`Entitlement::can_use`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Feature {
    /// Multi-tenant org membership (`false` on Free, `true` on Team+).
    MultiTenant,
    /// Pipeline count above the Free cap of 5.
    UnlimitedPipelines,
    /// Audit log retention beyond the Free cap of 14 days.
    ExtendedAuditRetention,
    /// SSO required (Enterprise-only).
    SsoRequired,
    /// Custom SLAs (Enterprise only).
    CustomSla,
}

/// Per-tenant entitlement snapshot. Cheap to clone.
#[derive(Debug, Clone)]
pub struct Entitlement {
    plan: Plan,
    status: SubscriptionStatus,
}

impl Entitlement {
    /// Resolve the entitlement for `(user, tenant)`. Returns
    /// `Free + Active` if no subscription is found — the safe default.
    #[must_use]
    pub fn for_user(subs: &SubscriptionService, _user: UserId, tenant: TenantId) -> Self {
        match subs.current_for_user(UserId(uuid::Uuid::nil()), tenant) {
            Some(s) => Self {
                plan: s.plan,
                status: s.status,
            },
            None => Self {
                plan: Plan::Free,
                status: SubscriptionStatus::Active,
            },
        }
    }

    /// Build an entitlement directly from a [`Subscription`].
    #[must_use]
    pub fn from_subscription(sub: &Subscription) -> Self {
        Self {
            plan: sub.plan,
            status: sub.status,
        }
    }

    /// True iff the subscription is in a state that honors
    /// entitlements (`active`, `trialing`, `past_due`).
    #[must_use]
    pub fn is_entitled(&self) -> bool {
        self.status.is_entitled()
    }

    #[must_use]
    pub fn plan(&self) -> Plan {
        self.plan
    }

    /// Check whether this entitlement grants a particular feature.
    #[must_use]
    pub fn can_use(&self, f: Feature) -> bool {
        // Canceled/incomplete/unpaid → deny everything.
        if !self.is_entitled() {
            return false;
        }
        match f {
            Feature::MultiTenant => self.plan >= Plan::Team,
            Feature::UnlimitedPipelines => self.plan >= Plan::Team,
            Feature::ExtendedAuditRetention => self.plan >= Plan::Team,
            Feature::SsoRequired => self.plan == Plan::Enterprise,
            Feature::CustomSla => self.plan == Plan::Enterprise,
        }
    }
}

/// Helper to attach an entitlement snapshot service to a
/// `SubscriptionService`. This is the boundary the api-gateway
/// uses: pass a `&SubscriptionService`, get back an `Entitlement`.
#[must_use]
pub fn entitlement_for(
    subs: &SubscriptionService,
    tenant: TenantId,
) -> Entitlement {
    Entitlement::for_user(subs, UserId(uuid::Uuid::nil()), tenant)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::subscription::SubscriptionRegistry;

    fn svc_with_plan(plan: Plan, status: SubscriptionStatus) -> SubscriptionService {
        let reg = Arc::new(SubscriptionRegistry::new());
        let svc = SubscriptionService::new(Arc::clone(&reg));
        let tenant = TenantId(uuid::Uuid::new_v4());
        svc.apply_transition(
            tenant,
            status,
            plan,
            Some("sub_test".into()),
            Some(1_700_000_000),
        )
        .expect("apply");
        // `apply_transition` only registers the last tenant; we just
        // need an Entitlement snapshot — the SubscriptionService is
        // not queried for tests, we use from_subscription directly.
        svc
    }

    #[test]
    fn free_denies_team_features() {
        let sub = Subscription {
            tenant_id: TenantId(uuid::Uuid::new_v4()),
            plan: Plan::Free,
            status: SubscriptionStatus::Active,
            stripe_subscription_id: None,
            current_period_end_unix: None,
        };
        let e = Entitlement::from_subscription(&sub);
        assert!(!e.can_use(Feature::MultiTenant));
        assert!(!e.can_use(Feature::UnlimitedPipelines));
        assert!(!e.can_use(Feature::SsoRequired));
    }

    #[test]
    fn team_allows_team_features_but_not_enterprise() {
        let sub = Subscription {
            tenant_id: TenantId(uuid::Uuid::new_v4()),
            plan: Plan::Team,
            status: SubscriptionStatus::Active,
            stripe_subscription_id: Some("sub_team".into()),
            current_period_end_unix: Some(1_700_000_000),
        };
        let e = Entitlement::from_subscription(&sub);
        assert!(e.can_use(Feature::MultiTenant));
        assert!(e.can_use(Feature::UnlimitedPipelines));
        assert!(!e.can_use(Feature::SsoRequired));
        assert!(!e.can_use(Feature::CustomSla));
    }

    #[test]
    fn enterprise_unlocks_all() {
        let sub = Subscription {
            tenant_id: TenantId(uuid::Uuid::new_v4()),
            plan: Plan::Enterprise,
            status: SubscriptionStatus::Active,
            stripe_subscription_id: Some("sub_ent".into()),
            current_period_end_unix: Some(1_700_000_000),
        };
        let e = Entitlement::from_subscription(&sub);
        assert!(e.can_use(Feature::MultiTenant));
        assert!(e.can_use(Feature::UnlimitedPipelines));
        assert!(e.can_use(Feature::SsoRequired));
        assert!(e.can_use(Feature::CustomSla));
    }

    #[test]
    fn canceled_subscription_denies_everything() {
        let sub = Subscription {
            tenant_id: TenantId(uuid::Uuid::new_v4()),
            plan: Plan::Enterprise,
            status: SubscriptionStatus::Canceled,
            stripe_subscription_id: None,
            current_period_end_unix: None,
        };
        let e = Entitlement::from_subscription(&sub);
        assert!(!e.is_entitled());
        for f in [
            Feature::MultiTenant,
            Feature::UnlimitedPipelines,
            Feature::ExtendedAuditRetention,
            Feature::SsoRequired,
            Feature::CustomSla,
        ] {
            assert!(!e.can_use(f), "feature {:?} must be denied", f);
        }
    }

    #[test]
    fn past_due_still_entitled() {
        let sub = Subscription {
            tenant_id: TenantId(uuid::Uuid::new_v4()),
            plan: Plan::Team,
            status: SubscriptionStatus::PastDue,
            stripe_subscription_id: Some("sub_team".into()),
            current_period_end_unix: Some(1_700_000_000),
        };
        let e = Entitlement::from_subscription(&sub);
        assert!(e.is_entitled());
        assert!(e.can_use(Feature::MultiTenant));
    }

    #[test]
    fn svc_with_plan_builds_subscription_without_panicking() {
        // Just verifies the helper. Real assertions live above.
        let _ = svc_with_plan(Plan::Team, SubscriptionStatus::Active);
    }
}