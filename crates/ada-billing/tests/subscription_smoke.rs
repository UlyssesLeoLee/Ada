//! subscription_smoke — the seven-state machine and its transitions.
//!
//! This module has no HTTP surface: `SubscriptionService` validates
//! every transition locally *before* any Stripe REST call (see
//! `auth-billing-arch.md` §5) and keeps the row in an in-process
//! registry, so there is nothing to stub with `wiremock`. The tests
//! pin the documented state set, the legal/illegal transition matrix,
//! and the guarantee that a rejected transition leaves the stored row
//! untouched.

use std::sync::Arc;

use ada_billing::subscription::{validate_transition, SubscriptionRegistry};
use ada_billing::{
    BillingError, Plan, Subscription, SubscriptionService, SubscriptionStatus, TenantId, UserId,
};
use uuid::Uuid;

use SubscriptionStatus::{
    Active, Canceled, Incomplete, IncompleteExpired, PastDue, Trialing, Unpaid,
};

/// The seven Stripe states, verbatim (per §5).
const ALL: [SubscriptionStatus; 7] = [
    Active,
    PastDue,
    Canceled,
    Trialing,
    Incomplete,
    IncompleteExpired,
    Unpaid,
];

/// Every transition `validate_transition` accepts (self-loops excepted).
const LEGAL: [(SubscriptionStatus, SubscriptionStatus); 13] = [
    (Incomplete, Active),
    (Incomplete, IncompleteExpired),
    (Incomplete, Canceled),
    (Trialing, Active),
    (Trialing, Canceled),
    (Active, PastDue),
    (Active, Canceled),
    (Active, Unpaid),
    (PastDue, Active),
    (PastDue, Canceled),
    (PastDue, Unpaid),
    (Unpaid, Active),
    (Unpaid, Canceled),
];

fn service() -> (Arc<SubscriptionRegistry>, SubscriptionService) {
    let registry = Arc::new(SubscriptionRegistry::new());
    let svc = SubscriptionService::new(Arc::clone(&registry));
    (registry, svc)
}

#[test]
fn every_status_round_trips_through_its_wire_string() {
    for status in ALL {
        let text = status.as_str();
        let back: SubscriptionStatus = text.parse().expect("documented status must parse");
        assert_eq!(back, status);
        assert_eq!(status.to_string(), text);
    }

    let bad: Result<SubscriptionStatus, _> = "past_due_but_wrong".parse();
    assert!(
        matches!(bad, Err(BillingError::InvalidPayload)),
        "an unknown status string must be rejected"
    );
}

#[test]
fn documented_progressions_are_accepted() {
    for (from, to) in LEGAL {
        assert!(
            validate_transition(from, to).is_ok(),
            "{from} -> {to} must be a legal transition"
        );
    }
    // Self-loops are no-ops (webhook replays) and always legal.
    for status in ALL {
        assert!(validate_transition(status, status).is_ok());
    }
}

#[test]
fn terminal_states_reject_every_exit() {
    // `canceled` and `incomplete_expired` are absorbing: nothing but a
    // self-loop may follow.
    for from in [Canceled, IncompleteExpired] {
        for to in ALL {
            if to == from {
                continue;
            }
            let r = validate_transition(from, to);
            assert!(
                matches!(r, Err(BillingError::StripeApi(409))),
                "{from} -> {to} must be rejected, got {r:?}"
            );
        }
    }
}

#[test]
fn trialing_cannot_jump_straight_to_past_due() {
    // Pins current behaviour: only `active` / `past_due` / `unpaid`
    // move between themselves, so a trial that fails its first
    // invoice is not modelled as `trialing -> past_due`.
    let r = validate_transition(Trialing, PastDue);
    assert!(
        matches!(r, Err(BillingError::StripeApi(409))),
        "trialing -> past_due is not in the legal set, got {r:?}"
    );
    assert!(validate_transition(Trialing, Active).is_ok());
    assert!(validate_transition(Trialing, Canceled).is_ok());
}

#[test]
fn apply_transition_persists_the_row_for_a_new_tenant() {
    let (registry, svc) = service();
    let tenant = TenantId(Uuid::new_v4());

    // The first transition for a tenant has no predecessor to validate.
    svc.apply_transition(
        tenant,
        Active,
        Plan::Team,
        Some("sub_smoke_1".into()),
        Some(1_700_000_000),
    )
    .expect("seed subscription");

    let row = svc.current(tenant).expect("row is stored");
    assert_eq!(row.tenant_id, tenant);
    assert_eq!(row.status, Active);
    assert_eq!(row.plan, Plan::Team);
    assert_eq!(row.stripe_subscription_id.as_deref(), Some("sub_smoke_1"));
    assert_eq!(row.current_period_end_unix, Some(1_700_000_000));
    // The service and the registry are two views of the same state.
    assert_eq!(registry.get(tenant), Some(row));
    assert_eq!(
        svc.current_for_user(UserId(Uuid::new_v4()), tenant),
        svc.current(tenant)
    );
}

#[test]
fn rejected_transition_leaves_the_previous_row_untouched() {
    let (_registry, svc) = service();
    let tenant = TenantId(Uuid::new_v4());
    svc.apply_transition(
        tenant,
        Active,
        Plan::Team,
        Some("sub_smoke_2".into()),
        Some(1),
    )
    .expect("seed subscription");

    // active -> incomplete is not in the legal set.
    let err = svc
        .apply_transition(tenant, Incomplete, Plan::Free, None, None)
        .expect_err("active -> incomplete must be rejected");
    assert!(matches!(err, BillingError::StripeApi(409)), "got {err}");

    let row = svc.current(tenant).expect("row survives a rejected write");
    assert_eq!(row.status, Active, "a rejected transition must not persist");
    assert_eq!(row.plan, Plan::Team);
    assert_eq!(row.stripe_subscription_id.as_deref(), Some("sub_smoke_2"));
}

#[test]
fn trialing_can_be_promoted_to_active_then_flagged_past_due() {
    let (_registry, svc) = service();
    let tenant = TenantId(Uuid::new_v4());

    svc.apply_transition(
        tenant,
        Trialing,
        Plan::Team,
        Some("sub_smoke_3".into()),
        Some(10),
    )
    .expect("start trial");
    svc.apply_transition(
        tenant,
        Active,
        Plan::Team,
        Some("sub_smoke_3".into()),
        Some(20),
    )
    .expect("trial converts");
    svc.apply_transition(
        tenant,
        PastDue,
        Plan::Team,
        Some("sub_smoke_3".into()),
        Some(30),
    )
    .expect("invoice fails");

    let row = svc.current(tenant).expect("row");
    assert_eq!(row.status, PastDue);
    assert_eq!(row.current_period_end_unix, Some(30));
}

#[test]
fn only_active_trialing_and_past_due_are_entitled() {
    for status in [Active, Trialing, PastDue] {
        assert!(status.is_entitled(), "{status} must be entitled");
    }
    for status in [Canceled, Incomplete, IncompleteExpired, Unpaid] {
        assert!(!status.is_entitled(), "{status} must not be entitled");
    }
}

#[test]
fn the_free_row_defaults_to_active_without_a_stripe_id() {
    let tenant = TenantId(Uuid::new_v4());
    let row = Subscription::free(tenant);
    assert_eq!(row.tenant_id, tenant);
    assert_eq!(row.plan, Plan::Free);
    assert_eq!(row.status, Active);
    assert!(row.stripe_subscription_id.is_none());
    assert!(row.current_period_end_unix.is_none());
    assert!(row.status.is_entitled());
}
