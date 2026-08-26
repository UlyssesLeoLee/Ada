//! Shard assignment — `NodeId` → `Vec<ShardId>` mapping.
//!
//! Real production builds will use a `tenant_id`-hashed consistent
//! ring (see `docs/modules/M-16-cluster-coordinator.md` §3.5 —
//! "状态分片按 `tenant_id` hash 均匀分布"). The v0.1.0 skeleton uses
//! a much simpler deterministic policy: each node owns a contiguous
//! `shards / N` slice of `[0, shards)`, in node-id order, with any
//! remainder going to the first nodes.
//!

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::node::NodeId;

/// Opaque shard identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ShardId(pub u16);

/// Result of a shard assignment: node → owned shards.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShardAssignment {
    /// Node id (sorted by value) → owned shards (sorted ascending).
    assignments: BTreeMap<NodeId, Vec<ShardId>>,
    total_shards: u16,
}

impl ShardAssignment {
    /// Distribute `shards` total shards across `nodes` in node-id
    /// order. Returns `None` if `nodes` is empty or `shards` is 0.
    ///
    /// The first `shards % N` nodes get one extra shard each
    /// (e.g. 7 shards across 3 nodes → 3 / 2 / 2).
    #[must_use]
    pub fn assign(nodes: &[NodeId], shards: u16) -> Option<Self> {
        if nodes.is_empty() || shards == 0 {
            return None;
        }
        let n = u16::try_from(nodes.len()).expect("node count fits in u16");
        let base = shards / n;
        let extra = shards % n;
        let mut sorted: Vec<NodeId> = nodes.to_vec();
        sorted.sort();

        let mut assignments: BTreeMap<NodeId, Vec<ShardId>> = BTreeMap::new();
        let mut cursor: u16 = 0;
        for (i, node) in sorted.iter().enumerate() {
            let i_u16 = u16::try_from(i).expect("index fits in u16");
            let count = base + u16::from(i_u16 < extra);
            let owned: Vec<ShardId> = (cursor..cursor + count).map(ShardId).collect();
            cursor += count;
            assignments.insert(*node, owned);
        }
        Some(Self {
            assignments,
            total_shards: shards,
        })
    }

    /// Total shards covered by this assignment.
    #[must_use]
    pub const fn total_shards(&self) -> u16 {
        self.total_shards
    }

    /// Number of nodes that own at least one shard.
    #[must_use]
    pub fn node_count(&self) -> usize {
        self.assignments.len()
    }

    /// Shards owned by `node` (empty if `node` is not in this
    /// assignment).
    #[must_use]
    pub fn shards_of(&self, node: NodeId) -> &[ShardId] {
        self.assignments.get(&node).map_or(&[], Vec::as_slice)
    }

    /// Lookup the node that owns `shard`. Linear scan — fine for
    /// the small assignments the v0.1.0 skeleton produces.
    #[must_use]
    pub fn owner_of(&self, shard: ShardId) -> Option<NodeId> {
        self.assignments
            .iter()
            .find(|(_, v)| v.contains(&shard))
            .map(|(k, _)| *k)
    }

    /// All shard→node pairs in ascending shard order.
    #[must_use]
    pub fn pairs(&self) -> Vec<(ShardId, NodeId)> {
        let mut out = Vec::with_capacity(self.total_shards as usize);
        for (node, shards) in &self.assignments {
            for s in shards {
                out.push((*s, *node));
            }
        }
        out.sort_by_key(|(s, _)| *s);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nodes<const N: usize>() -> Vec<NodeId> {
        // Deterministic NodeIds so test output is stable.
        (0..N)
            .map(|i| {
                let byte = u8::try_from(i).expect("test index fits in u8");
                NodeId(uuid::Uuid::from_bytes([byte; 16]))
            })
            .collect()
    }

    #[test]
    fn assign_rejects_empty_inputs() {
        assert!(ShardAssignment::assign(&[], 7).is_none());
        let ns = nodes::<2>();
        assert!(ShardAssignment::assign(&ns, 0).is_none());
    }

    #[test]
    fn assign_uniform_distribution() {
        let ns = nodes::<3>();
        let a = ShardAssignment::assign(&ns, 6).expect("assign");
        for n in &ns {
            assert_eq!(a.shards_of(*n).len(), 2);
        }
        assert_eq!(a.total_shards(), 6);
    }

    #[test]
    fn assign_uneven_with_remainder() {
        let ns = nodes::<3>();
        let a = ShardAssignment::assign(&ns, 7).expect("assign");
        let sizes: Vec<usize> = ns.iter().map(|n| a.shards_of(*n).len()).collect();
        // Remainder 1 goes to the first node in sorted order.
        let mut sorted_sizes = sizes.clone();
        sorted_sizes.sort_by(|a, b| b.cmp(a));
        assert_eq!(sorted_sizes, vec![3, 2, 2]);
        // Total covered shards == 7.
        let total: usize = sizes.iter().sum();
        assert_eq!(total, 7);
    }

    #[test]
    fn owner_of_round_trip() {
        let ns = nodes::<4>();
        let a = ShardAssignment::assign(&ns, 12).expect("assign");
        for s in 0..12u16 {
            let shard = ShardId(s);
            let owner = a.owner_of(shard).expect("owner");
            assert!(a.shards_of(owner).contains(&shard));
        }
    }

    #[test]
    fn unknown_node_owns_nothing() {
        let ns = nodes::<2>();
        let a = ShardAssignment::assign(&ns, 4).expect("assign");
        let stranger = NodeId::new();
        assert!(a.shards_of(stranger).is_empty());
    }
}
