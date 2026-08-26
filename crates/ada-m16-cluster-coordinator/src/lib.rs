//! M-16: Cluster coordinator. Leader election, shard assignment, heartbeat.
//!
//! ## v0.1.0 scope (B2)
//!
//! This crate is a **minimum skeleton** for the cluster coordinator:
//!
//! - [`NodeId`] / [`NodeState`] — node identity + state-machine states
//! - [`Term`] — monotonic election term with saturating `next()`
//! - [`Heartbeat`] — in-process per-peer liveness tracker
//! - [`Election`] — three-state timer-driven state machine
//!   (`Follower` / `Candidate` / `Leader`) that **simulates** a
//!   leader election under `tokio::time::pause` + virtual clock
//!   advance. No RPC, no real quorum algorithm.
//! - [`ShardAssignment`] — deterministic `NodeId → Vec<ShardId>`
//!   distribution (real builds will use a tenant-id-hashed ring).
//! - [`CoordError`] — single error enum (5 variants).
//!
//! See [`DOC-MOD-016`](../docs/modules/M-16-cluster-coordinator.md)
//! for the full design (DB-backed `leader_lease` + PL/pgSQL
//! `acquire_lease()`).
//!
//! 関連 IPA フェーズ: 22-52 (基本設計/詳細設計), 53-58 (実装), 59-95 (試験)
//! 設計書: docs/modules/M-16-cluster-coordinator.md (DOC-MOD-016)
//! ワークフロー: docs/architecture/08-workflow-overview.md

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

mod election;
mod error;
mod heartbeat;
mod node;
mod shard;
mod term;

pub use election::{Election, ElectionConfig, ElectionSnapshot};
pub use error::{CoordError, Result};
pub use heartbeat::{Heartbeat, HeartbeatConfig};
pub use node::{NodeId, NodeState};
pub use shard::{ShardAssignment, ShardId};
pub use term::Term;

/// Crate version, taken from `CARGO_PKG_VERSION` (single workspace
/// version per D-09).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Crate name, taken from `CARGO_PKG_NAME`.
pub const NAME: &str = env!("CARGO_PKG_NAME");

/// `skeleton`-layer string tag (仿生モデル 4 層分類, see
/// [`DOC-ARCH-001`](https://example.invalid/docs/architecture/00-anatomy-model.md)).
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
