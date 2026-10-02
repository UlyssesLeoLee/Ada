//! `entitlement_smoke` — plan-tier feature gating.
//!
//! Pure in-process surface: `Entitlement` is derived from a
//! `Subscription` row and exposes `can_use(Feature)`, so there is no
//! HTTP call to stub. The matrix asserted here is the one documented
//! in `auth-billing-arch.md` §5 and repeated in `plan.rs`:
//!
//! | Plan         | Tenants | Pipelines | Audit retention | SSO |
//! |--------------|---------|-----------|-----------------|-----|
//! | `Free`       | 1       | 5         | 14 days         | no  |
//! | `Team`       | 10      | unlimited | 90 days         | no  |
//! | `Enterprise` | custom  | unlimited | 365 days        | yes |

use std::sync::Arc;

use ada_billing::entitlement::entitlement_for;
use ada_billing::subscription::SubscriptionRegistry;
use ada_billing::{
    Entitlement, Feature, Plan, Subscription, SubscriptionService, SubscriptionStatus, TenantId,
    UserId,
};
use uuid::Uuid;

use SubscriptionStatus::{
    Active, Canceled, Incomplete, IncompleteExpired, PastDue, Trialing, Unpaid,
};

/// All five gates, so denials can be asserted exhaustively.
const ALL_FEATURES: [Feature; 5] = [
    Feature::MultiTenant,
    Feature::UnlimitedPipelines,
    Feature::ExtendedAuditRetention,
    Feature::SsoRequired,
    Feature::CustomSla,
];

/// Snapshot for an arbitrary plan/status pair.
fn entitlement(plan: Plan, status: SubscriptionStatus) -> Entitlement {
    Entitlement::from_subscription(&Subscription {
        tenant_id: TenantId(Uuid::new_v4()),
        plan,
        status,
        stripe_subscription_id: Some("sub_smoke".into()),
        current_period_end_unix: Some(1_700_000_000),
    })
}

fn seed(plan: Plan, status: SubscriptionStatus) -> (SubscriptionService, TenantId) {
    let registry = Arc::new(SubscriptionRegistry::new());
    let svc = SubscriptionService::new(registry);
    let tenant = TenantId(Uuid::new_v4());
    svc.apply_transition(
        tenant,
        status,
        plan,
        Some("sub_smoke".into()),
        Some(1_700_000_000),
    )
    .expect("seed subscription");
    (svc, tenant)
}

#[test]
fn the_free_tier_denies_every_paid_gate() {
    let e = entitlement(Plan::Free, Active);
    assert_eq!(e.plan(), Plan::Free);
    assert!(e.is_entitled());
    for f in ALL_FEATURES {
        assert!(!e.can_use(f), "free must not unlock {f:?}");
    }
}

#[test]
fn the_team_tier_unlocks_team_features_but_not_enterprise_ones() {
    let e = entitlement(Plan::Team, Active);
    assert!(e.can_use(Feature::MultiTenant));
    assert!(e.can_use(Feature::UnlimitedPipelines));
    assert!(e.can_use(Feature::ExtendedAuditRetention));
    assert!(
        !e.can_use(Feature::SsoRequired),
        "SSO is enterprise-only per the §5 plan table"
    );
    assert!(!e.can_use(Feature::CustomSla));
}

#[test]
fn the_enterprise_tier_unlocks_every_gate() {
    let e = entitlement(Plan::Enterprise, Active);
    assert_eq!(e.plan(), Plan::Enterprise);
    for f in ALL_FEATURES {
        assert!(e.can_use(f), "enterprise must unlock {f:?}");
    }
    // Cross-check against the plan catalogue's own SSO flag.
    assert!(e.plan().requires_sso());
}

#[test]
fn an_unentitled_status_denies_every_gate_even_on_enterprise() {
    for status in [Canceled, Incomplete, IncompleteExpired, Unpaid] {
        let e = entitlement(Plan::Enterprise, status);
        assert!(!e.is_entitled(), "{status} must not be entitled");
        for f in ALL_FEATURES {
            assert!(
                !e.can_use(f),
                "{status} must deny {f:?} regardless of plan tier"
            );
        }
    }
}

#[test]
fn a_trial_and_a_past_due_invoice_keep_the_entitlements() {
    // `past_due` stays usable during the grace period; only the
    // terminal/void states revoke access.
    for status in [Trialing, PastDue] {
        let e = entitlement(Plan::Team, status);
        assert!(e.is_entitled(), "{status} must stay entitled");
        assert!(e.can_use(Feature::MultiTenant), "{status} keeps Team gates");
        assert!(!e.can_use(Feature::SsoRequired));
    }
}

#[test]
fn an_unknown_tenant_falls_back_to_free_and_active() {
    let svc = SubscriptionService::new(Arc::new(SubscriptionRegistry::new()));
    let tenant = TenantId(Uuid::new_v4());

    let e = Entitlement::for_user(&svc, UserId(Uuid::new_v4()), tenant);
    assert_eq!(e.plan(), Plan::Free, "the safe default is the free tier");
    assert!(e.is_entitled());
    assert!(!e.can_use(Feature::MultiTenant));
    assert!(!e.can_use(Feature::UnlimitedPipelines));
}

#[test]
fn an_entitlement_resolves_through_the_subscription_service() {
    let (svc, tenant) = seed(Plan::Team, Active);

    let e = entitlement_for(&svc, tenant);
    assert_eq!(e.plan(), Plan::Team);
    assert!(e.is_entitled());
    assert!(e.can_use(Feature::MultiTenant));
    assert!(!e.can_use(Feature::SsoRequired));

    // `Entitlement::for_user` resolves the same snapshot.
    let by_user = Entitlement::for_user(&svc, UserId(Uuid::new_v4()), tenant);
    assert_eq!(by_user.plan(), e.plan());
    assert_eq!(
        by_user.can_use(Feature::CustomSla),
        e.can_use(Feature::CustomSla)
    );

    // A downgrade re-reads the same row and drops the gates again.
    svc.apply_transition(tenant, Canceled, Plan::Free, None, None)
        .expect("cancel");
    let after = entitlement_for(&svc, tenant);
    assert!(!after.is_entitled());
    assert_eq!(after.plan(), Plan::Free);
    assert!(!after.can_use(Feature::MultiTenant));
}

#[test]
fn the_plan_catalogue_matches_the_arch_doc_price_table() {
    // §5: `free` has no Stripe Price; the paid tiers carry placeholders.
    assert_eq!(Plan::Free.placeholder_price_id(), None);
    assert_eq!(
        Plan::Team.placeholder_price_id(),
        Some("price_team_monthly")
    );
    assert_eq!(
        Plan::Enterprise.placeholder_price_id(),
        Some("price_enterprise_monthly")
    );
    assert_eq!(Plan::Free.as_str(), "free");
    assert_eq!(Plan::Team.as_str(), "team");
    assert_eq!(Plan::Enterprise.as_str(), "enterprise");
    // Ordering is privilege-ascending, which `can_use` relies on.
    assert!(Plan::Free < Plan::Team);
    assert!(Plan::Team < Plan::Enterprise);
    assert!(Plan::Enterprise.at_least(Plan::Team));
    assert!(!Plan::Team.at_least(Plan::Enterprise));
}
