//! Plan catalog: [`Plan`] enum + static Stripe Price ID map.
//!
//! The three plans and their Stripe Price placeholders come straight
//! from `docs/commercial/auth-billing-arch.md` §5. **They are
//! placeholders**: production wiring swaps them for the live Stripe
//! Price IDs by setting `STRIPE_PRICE_FREE`,
//! `STRIPE_PRICE_TEAM_MONTHLY` etc. in the operator's deployment
//! manifest. Until that lands the catalog below is the single
//! source of truth.
//!
//! ## Plan tier matrix
//!
//! | Plan         | Tenants | Pipelines     | Audit retention | SSO required |
//! |--------------|---------|---------------|-----------------|--------------|
//! | `Free`       | 1       | 5             | 14 days         | no           |
//! | `Team`       | 10      | unlimited     | 90 days         | no           |
//! | `Enterprise` | custom  | unlimited     | 365 days        | yes          |

use serde::{Deserialize, Serialize};

/// The three subscription tiers offered through Stripe.
///
/// The variants are ordered by privilege so that
/// [`Plan::at_least`] / [`Plan::cmp`] work the way other Ada
/// modules (e.g. `ada-m11-rbac-collab::Role`) expect: `Free <
/// Team < Enterprise`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Plan {
    /// Default tier for any new tenant; no Stripe Price backing
    /// (the customer is treated as never-subscribed).
    Free,
    /// The `Team` tier backed by Stripe Price `price_team_monthly`.
    Team,
    /// The `Enterprise` tier backed by Stripe Price
    /// `price_enterprise_monthly`; requires SSO.
    Enterprise,
}

impl Plan {
    /// Returns the canonical lowercase string tag (`"free"`,
    /// `"team"`, `"enterprise"`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Free => "free",
            Self::Team => "team",
            Self::Enterprise => "enterprise",
        }
    }

    /// Static map from plan → placeholder Stripe Price ID. The
    /// billing document (`auth-billing-arch.md` §5) names the same
    /// strings verbatim.
    #[must_use]
    pub const fn placeholder_price_id(self) -> Option<&'static str> {
        match self {
            // The Free plan has no Stripe Price — see the plan tier
            // matrix in the module docs.
            Self::Free => None,
            Self::Team => Some("price_team_monthly"),
            Self::Enterprise => Some("price_enterprise_monthly"),
        }
    }

    /// True iff the plan requires SSO. Only `Enterprise` does.
    #[must_use]
    pub const fn requires_sso(self) -> bool {
        matches!(self, Self::Enterprise)
    }

    /// Tier is at least as privileged as `other`.
    #[must_use]
    pub fn at_least(self, other: Self) -> bool {
        self >= other
    }
}

impl core::fmt::Display for Plan {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<Plan> for &'static str {
    fn from(p: Plan) -> Self {
        p.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_ordering_is_privilege_ascending() {
        assert!(Plan::Free < Plan::Team);
        assert!(Plan::Team < Plan::Enterprise);
    }

    #[test]
    fn at_least_is_inclusive() {
        assert!(Plan::Team.at_least(Plan::Team));
        assert!(Plan::Enterprise.at_least(Plan::Team));
        assert!(!Plan::Team.at_least(Plan::Enterprise));
        assert!(!Plan::Free.at_least(Plan::Team));
    }

    #[test]
    fn placeholder_price_ids_match_arch_doc() {
        assert_eq!(Plan::Free.placeholder_price_id(), None);
        assert_eq!(
            Plan::Team.placeholder_price_id(),
            Some("price_team_monthly")
        );
        assert_eq!(
            Plan::Enterprise.placeholder_price_id(),
            Some("price_enterprise_monthly")
        );
    }

    #[test]
    fn display_renders_lowercase() {
        assert_eq!(Plan::Free.to_string(), "free");
        assert_eq!(Plan::Team.to_string(), "team");
        assert_eq!(Plan::Enterprise.to_string(), "enterprise");
    }

    #[test]
    fn requires_sso_is_only_enterprise() {
        assert!(!Plan::Free.requires_sso());
        assert!(!Plan::Team.requires_sso());
        assert!(Plan::Enterprise.requires_sso());
    }

    #[test]
    fn plan_serde_roundtrip() {
        for plan in [Plan::Free, Plan::Team, Plan::Enterprise] {
            let json = serde_json::to_string(&plan).expect("serialize");
            let back: Plan = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(back, plan);
        }
    }
}