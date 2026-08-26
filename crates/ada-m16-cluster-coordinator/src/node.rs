//! Node identity and state-machine states for the cluster coordinator.
//!
//! A [`NodeId`] is a `Uuid`-backed newtype (per DOC-ARCH-007 §7.4). The
//! state-machine is a simplified three-state Raft-style model
//! ([`NodeState`]) used by [`crate::election::Election`] to drive
//! `tick()` transitions. No real RPC / quorum algorithm is implemented
//! in v0.1.0 — see B2 brief and `docs/modules/M-16-cluster-coordinator.md`.

use core::fmt;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Stable identifier for a cluster node.
///
/// In production this is derived from hostname + boot time + nonce
/// (see `docs/modules/M-16-cluster-coordinator.md` §3.1); the
/// skeleton accepts a fresh `Uuid::new_v4()` from the caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct NodeId(pub Uuid);

impl NodeId {
    /// Build a random `NodeId`.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "node({})", self.0)
    }
}

/// Simplified Raft-style node state.
///
/// Transitions are driven by [`crate::election::Election::tick`]:
///
/// ```text
///        become_follower
///  Candidate  ────────────►  Follower
///      │                          ▲
///      │ become_candidate         │
///      ▼                          │ become_leader
///   Leader ───────────────────────┘
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeState {
    /// Passive follower; receives heartbeats from a leader.
    Follower,
    /// Candidate that incremented its term and is collecting votes.
    Candidate,
    /// Cluster leader; sends heartbeats and accepts writes.
    Leader,
}

impl NodeState {
    /// Short, lowercase string tag, used in logs and snapshots.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Follower => "follower",
            Self::Candidate => "candidate",
            Self::Leader => "leader",
        }
    }
}

impl fmt::Display for NodeState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_id_new_is_unique() {
        let a = NodeId::new();
        let b = NodeId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn node_id_display() {
        let id = NodeId(Uuid::nil());
        assert_eq!(id.to_string(), "node(00000000-0000-0000-0000-000000000000)");
    }

    #[test]
    fn node_state_as_str() {
        assert_eq!(NodeState::Follower.as_str(), "follower");
        assert_eq!(NodeState::Candidate.as_str(), "candidate");
        assert_eq!(NodeState::Leader.as_str(), "leader");
    }

    #[test]
    fn node_id_serde_roundtrip() {
        let id = NodeId::new();
        let json = serde_json::to_string(&id).unwrap();
        let back: NodeId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, back);
    }
}
