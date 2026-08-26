//! Shared newtype identifiers and the [`AdaLayer`] enum.
//!
//! Every identifier wraps a [`uuid::Uuid`] so that:
//! - serde round-trips keep the canonical hyphenated form
//! - the shared layer never has to invent ad-hoc string IDs
//! - downstream crates can `unwrap` or pattern-match without fear of
//!   mixing up two ID flavours
//!
//! See [`DOC-ARCH-007 §7.4`](https://example.invalid/docs/architecture/06-rust-tech-selection.md)
//! for the workspace-wide adoption of `Uuid` for primary keys and
//! [`DOC-ARCH-001 §5`](https://example.invalid/docs/architecture/00-anatomy-model.md)
//! for the multi-tenant rule that drives [`TenantId`].

use core::fmt;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Multi-tenant isolation key.
///
/// All data models carry a `tenant_id` field and every DB query / API
/// request injects a tenant filter (see
/// [`DOC-ARCH-001 §5`](https://example.invalid/docs/architecture/00-anatomy-model.md),
/// NF-SEC【必須】).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TenantId(pub Uuid);

impl fmt::Display for TenantId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "tenant({})", self.0)
    }
}

/// User (subject) identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(pub Uuid);

impl fmt::Display for UserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "user({})", self.0)
    }
}

/// Canvas (the unit of authoring / execution) identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CanvasId(pub Uuid);

impl fmt::Display for CanvasId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "canvas({})", self.0)
    }
}

/// Idempotency key for at-least-once + idempotent consumer delivery.
///
/// The central event bus (D-07) delivers events at least once; the
/// `idempotency_key` lets consumers drop duplicate processing
/// without coordinating with the producer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IdempotencyKey(pub Uuid);

impl fmt::Display for IdempotencyKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "idempotency({})", self.0)
    }
}

/// Tag identifying which of the five `ada-*` layers a value or a
/// tracing span belongs to.
///
/// Matches the `LAYER` string constants exposed elsewhere in the
/// workspace (`"skeleton" | "blood" | "nerve" | "muscle" | "shared"`,
/// see [`DOC-ARCH-001 §3`](https://example.invalid/docs/architecture/00-anatomy-model.md)).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AdaLayer {
    /// 骨格 (node runtime) — the structural cells.
    Skeleton,
    /// 血液 (data flow) — the standard NJSON data bus.
    Blood,
    /// 神経 (orchestration engine) — state machine & decisions.
    Nerve,
    /// 筋肉 (control flow) — actual node triggering & scheduling.
    Muscle,
    /// Cross-cutting (e.g. `ada-core`, `ada-telemetry`).
    Shared,
}

impl AdaLayer {
    /// Canonical lowercase string tag, matching the `LAYER` constants.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            AdaLayer::Skeleton => "skeleton",
            AdaLayer::Blood => "blood",
            AdaLayer::Nerve => "nerve",
            AdaLayer::Muscle => "muscle",
            AdaLayer::Shared => "shared",
        }
    }
}

impl From<AdaLayer> for &'static str {
    fn from(layer: AdaLayer) -> Self {
        layer.as_str()
    }
}

impl fmt::Display for AdaLayer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tenant_id_display() {
        let id = TenantId(Uuid::nil());
        assert_eq!(
            id.to_string(),
            "tenant(00000000-0000-0000-0000-000000000000)"
        );
    }

    #[test]
    fn user_id_display() {
        let id = UserId(Uuid::nil());
        assert_eq!(id.to_string(), "user(00000000-0000-0000-0000-000000000000)");
    }

    #[test]
    fn canvas_id_display() {
        let id = CanvasId(Uuid::nil());
        assert_eq!(
            id.to_string(),
            "canvas(00000000-0000-0000-0000-000000000000)"
        );
    }

    #[test]
    fn idempotency_key_display() {
        let id = IdempotencyKey(Uuid::nil());
        assert_eq!(
            id.to_string(),
            "idempotency(00000000-0000-0000-0000-000000000000)"
        );
    }

    #[test]
    fn tenant_id_serde_roundtrip() {
        let id = TenantId(Uuid::new_v4());
        let json = serde_json::to_string(&id).expect("serialize");
        let back: TenantId = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(id, back);
    }

    #[test]
    fn canvas_id_serde_roundtrip() {
        let id = CanvasId(Uuid::new_v4());
        let json = serde_json::to_string(&id).expect("serialize");
        let back: CanvasId = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(id, back);
    }

    #[test]
    fn ada_layer_as_str() {
        assert_eq!(AdaLayer::Skeleton.as_str(), "skeleton");
        assert_eq!(AdaLayer::Blood.as_str(), "blood");
        assert_eq!(AdaLayer::Nerve.as_str(), "nerve");
        assert_eq!(AdaLayer::Muscle.as_str(), "muscle");
        assert_eq!(AdaLayer::Shared.as_str(), "shared");
    }

    #[test]
    fn ada_layer_from_into_static_str() {
        let s: &'static str = AdaLayer::Nerve.into();
        assert_eq!(s, "nerve");
    }

    #[test]
    fn ada_layer_display() {
        assert_eq!(AdaLayer::Muscle.to_string(), "muscle");
        assert_eq!(AdaLayer::Shared.to_string(), "shared");
    }

    #[test]
    fn ada_layer_hash_eq() {
        let mut set = std::collections::HashSet::new();
        set.insert(AdaLayer::Nerve);
        set.insert(AdaLayer::Muscle);
        // Re-inserting the same value should not grow the set.
        set.insert(AdaLayer::Nerve);
        assert_eq!(set.len(), 2);
        assert!(set.contains(&AdaLayer::Nerve));
    }
}
