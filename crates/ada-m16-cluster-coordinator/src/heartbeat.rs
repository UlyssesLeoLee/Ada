//! Per-peer heartbeat bookkeeping.
//!
//! Real production builds send heartbeats over the network and
//! persist the `last_seen` map to the `cluster_node` table (see
//! `docs/modules/M-16-cluster-coordinator.md` §3.2). The v0.1.0
//! skeleton keeps [`Heartbeat`] in-process and lets the election
//! state machine query it for peer liveness; tests advance virtual
//! time via `tokio::time::pause` + `tokio::time::advance`.
//!
//! All times are `tokio::time::Instant` so that the same virtual
//! clock that drives `tokio::time::sleep` / `tokio::time::interval`
//! in production code also drives the v0.1.0 tests.

use std::collections::HashMap;
use std::time::Duration;

use tokio::time::Instant;

use crate::node::NodeId;

/// Tunables for the heartbeat subsystem.
#[derive(Debug, Clone, Copy)]
pub struct HeartbeatConfig {
    /// How often heartbeats are sent (or simulated as sent). Default: 1s.
    pub period: Duration,
    /// After this much wall-time without a heartbeat, a peer is
    /// considered suspect. Default: 3s.
    pub timeout: Duration,
}

impl Default for HeartbeatConfig {
    fn default() -> Self {
        Self {
            period: Duration::from_secs(1),
            timeout: Duration::from_secs(3),
        }
    }
}

/// In-process per-peer heartbeat tracker.
#[derive(Debug)]
pub struct Heartbeat {
    config: HeartbeatConfig,
    last_seen: HashMap<NodeId, Instant>,
}

impl Heartbeat {
    /// Build a new [`Heartbeat`] with the given config and an empty
    /// peer table.
    #[must_use]
    pub fn new(config: HeartbeatConfig) -> Self {
        Self {
            config,
            last_seen: HashMap::new(),
        }
    }

    /// Config used to build this tracker.
    #[must_use]
    pub const fn config(&self) -> &HeartbeatConfig {
        &self.config
    }

    /// Record that we just saw `peer` at `now`. Idempotent.
    pub fn observe(&mut self, peer: NodeId, now: Instant) {
        self.last_seen.insert(peer, now);
    }

    /// True if `peer`'s most recent heartbeat is older than
    /// `config.timeout` at `now`, or if we have never seen `peer`.
    #[must_use]
    pub fn is_suspect(&self, peer: NodeId, now: Instant) -> bool {
        match self.last_seen.get(&peer) {
            Some(last) => now.duration_since(*last) >= self.config.timeout,
            None => true,
        }
    }

    /// All peers that are suspect at `now`.
    #[must_use]
    pub fn suspect_peers(&self, now: Instant) -> Vec<NodeId> {
        self.last_seen
            .iter()
            .filter_map(|(peer, last)| {
                if now.duration_since(*last) >= self.config.timeout {
                    Some(*peer)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Number of peers we have ever observed.
    #[must_use]
    pub fn observed_count(&self) -> usize {
        self.last_seen.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(n: u8) -> NodeId {
        NodeId(uuid::Uuid::from_bytes([n; 16]))
    }

    #[test]
    fn default_config_is_one_and_three() {
        let c = HeartbeatConfig::default();
        assert_eq!(c.period, Duration::from_secs(1));
        assert_eq!(c.timeout, Duration::from_secs(3));
    }

    #[tokio::test(start_paused = true)]
    async fn never_seen_is_suspect() {
        let hb = Heartbeat::new(HeartbeatConfig::default());
        let now = Instant::now();
        assert!(hb.is_suspect(node(1), now));
        assert_eq!(hb.observed_count(), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn freshly_observed_is_not_suspect() {
        let mut hb = Heartbeat::new(HeartbeatConfig::default());
        let now = Instant::now();
        hb.observe(node(1), now);
        assert!(!hb.is_suspect(node(1), now));
    }

    #[tokio::test(start_paused = true)]
    async fn suspect_after_timeout() {
        let cfg = HeartbeatConfig {
            period: Duration::from_millis(10),
            timeout: Duration::from_millis(50),
        };
        let mut hb = Heartbeat::new(cfg);
        let now = Instant::now();
        hb.observe(node(1), now);
        // Advance virtual time 100 ms past the observation.
        tokio::time::advance(Duration::from_millis(100)).await;
        let later = Instant::now();
        assert!(hb.is_suspect(node(1), later));
        let suspects = hb.suspect_peers(later);
        assert_eq!(suspects.len(), 1);
        assert_eq!(suspects[0], node(1));
    }
}
