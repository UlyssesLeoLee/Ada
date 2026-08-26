//! Simplified leader-election state machine.
//!
//! ## Scope (B2)
//!
//! This is **not** a real Raft implementation — there is no network
//! RPC, no quorum algorithm, no log replication. The class is a
//! three-state timer-driven state machine that:
//!
//! - starts as a [`Follower`](NodeState::Follower);
//! - starts an election when it has not seen a leader for
//!   `election_timeout` (default 3s), becoming a
//!   [`Candidate`](NodeState::Candidate) and voting for itself;
//! - on every successful election attempt **deterministically**
//!   promotes itself to [`Leader`](NodeState::Leader) — the
//!   "simulation" stands in for a real quorum;
//! - bumps [`Term`] on every new election.
//!
//! The B2 contract is: a 3-node cluster whose nodes all call
//! `tick()` with a virtual clock must converge to exactly one
//! `Leader` and two `Follower`s within a few ticks. Tests under
//! `tokio::time::pause()` + `tokio::time::advance` drive that
//! scenario without touching the wall clock.
//!
//! All time inputs are `tokio::time::Instant` so the same virtual
//! clock that drives `tokio::time::sleep` / `tokio::time::interval`
//! in production code also drives the v0.1.0 tests.
//!
//! See `docs/modules/M-16-cluster-coordinator.md` §3.4 for the
//! production design (DB-backed `leader_lease` + `acquire_lease()`
//! PL/pgSQL function).

use std::time::Duration;

use tokio::time::Instant;

use crate::error::Result as CoordResult;

use crate::heartbeat::{Heartbeat, HeartbeatConfig};
use crate::node::{NodeId, NodeState};
use crate::term::Term;

/// Tunables for the election state machine.
#[derive(Debug, Clone, Copy)]
pub struct ElectionConfig {
    /// Wait this long without a leader heartbeat before starting a
    /// new election. Default: 3s.
    pub election_timeout: Duration,
    /// How often heartbeats are sent. Default: 1s.
    pub heartbeat_period: Duration,
}

impl Default for ElectionConfig {
    fn default() -> Self {
        Self {
            election_timeout: Duration::from_secs(3),
            heartbeat_period: Duration::from_secs(1),
        }
    }
}

/// Snapshot of a node's election state at one instant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ElectionSnapshot {
    /// The node this snapshot belongs to.
    pub node_id: NodeId,
    /// State machine state.
    pub state: NodeState,
    /// Current term.
    pub term: Term,
    /// Candidate this node voted for in the current term, if any.
    pub voted_for: Option<NodeId>,
}

/// Three-state election state machine.
#[derive(Debug)]
pub struct Election {
    node_id: NodeId,
    peers: Vec<NodeId>,
    config: ElectionConfig,
    state: NodeState,
    current_term: Term,
    voted_for: Option<NodeId>,
    last_heartbeat: Option<Instant>,
    started_at: Instant,
    heartbeats: Heartbeat,
}

impl Election {
    /// Build a fresh [`Election`] in [`NodeState::Follower`] at
    /// term 0, with the current tokio time as both the boot
    /// instant and the reference for future `tick()` calls.
    #[must_use]
    pub fn new(node_id: NodeId, peers: Vec<NodeId>) -> Self {
        let now = Instant::now();
        Self::with_config(node_id, peers, now, ElectionConfig::default())
    }

    /// Build with an explicit config and a fixed `now` (used by
    /// tests so they can compare across time advances).
    #[must_use]
    pub fn with_config(
        node_id: NodeId,
        peers: Vec<NodeId>,
        now: Instant,
        config: ElectionConfig,
    ) -> Self {
        let heartbeats = Heartbeat::new(HeartbeatConfig {
            period: config.heartbeat_period,
            timeout: config.election_timeout,
        });
        Self {
            node_id,
            peers,
            config,
            state: NodeState::Follower,
            current_term: Term::ZERO,
            voted_for: None,
            last_heartbeat: None,
            started_at: now,
            heartbeats,
        }
    }

    /// Current state, term, and vote (for assertions + RPC replies).
    #[must_use]
    pub fn snapshot(&self) -> ElectionSnapshot {
        ElectionSnapshot {
            node_id: self.node_id,
            state: self.state,
            term: self.current_term,
            voted_for: self.voted_for,
        }
    }

    /// This node's [`NodeId`].
    #[must_use]
    pub const fn node_id(&self) -> NodeId {
        self.node_id
    }

    /// Current state.
    #[must_use]
    pub const fn state(&self) -> NodeState {
        self.state
    }

    /// Current term.
    #[must_use]
    pub const fn term(&self) -> Term {
        self.current_term
    }

    /// Peer IDs (excluding self).
    #[must_use]
    pub fn peers(&self) -> &[NodeId] {
        &self.peers
    }

    /// Step the state machine to `now`.
    ///
    /// `cluster` is the snapshot of every other node's election
    /// state at `now`; the local node uses it to:
    ///
    /// - step down to [`Follower`](NodeState::Follower) if any peer
    ///   is already on a strictly higher term, and adopt the
    ///   highest seen term;
    /// - step down to [`Follower`](NodeState::Follower) if a peer
    ///   on the same term is already a leader.
    ///
    /// After reconciliation, the local node runs its own timer
    /// logic:
    ///
    /// - If we are a [`Leader`](NodeState::Leader), record a
    ///   self-heartbeat (the simulated leadership loop). Future
    ///   versions will broadcast heartbeats to peers here.
    /// - If we are a [`Follower`](NodeState::Follower) and the time
    ///   since the last observed leader heartbeat has exceeded
    ///   `election_timeout`, transition to
    ///   [`Candidate`](NodeState::Candidate) via
    ///   [`become_candidate`](Self::become_candidate).
    /// - If we are a [`Candidate`](NodeState::Candidate) and the
    ///   time since we started the election has exceeded
    ///   `election_timeout` twice AND no peer is already a leader
    ///   on the same term, transition to
    ///   [`Leader`](NodeState::Leader) (the simulation shortcut for
    ///   "we won the vote round").
    ///
    /// `tick` always returns `Ok(())`; the
    /// [`CoordError::NoQuorum`] variant is reserved for the future
    /// RPC layer that will count actual peer votes.
    ///
    /// **v0.1.0 is synchronous**; the production layer that will
    /// wrap real RPC will likely add `async` and `.await` real
    /// network calls without changing the public interface (just
    /// the return shape).
    pub fn tick(&mut self, now: Instant, cluster: &[ElectionSnapshot]) -> CoordResult<()> {
        self.reconcile(cluster, now);

        match self.state {
            NodeState::Leader => {
                // Simulated leadership: record a self heartbeat so
                // the timer keeps us as leader. A real impl sends
                // AppendEntries to every peer here.
                self.last_heartbeat = Some(now);
                self.heartbeats.observe(self.node_id, now);
                Ok(())
            }
            NodeState::Follower => {
                let since = self.last_heartbeat.map_or_else(
                    || now.duration_since(self.started_at),
                    |t| now.duration_since(t),
                );
                if since >= self.config.election_timeout {
                    self.become_candidate(now);
                }
                Ok(())
            }
            NodeState::Candidate => {
                let since_started = self.last_heartbeat.map_or_else(
                    || now.duration_since(self.started_at),
                    |t| now.duration_since(t),
                );
                let timed_out = since_started >= self.config.election_timeout.saturating_mul(2);
                let some_peer_already_leader_same_term = cluster
                    .iter()
                    .any(|s| s.term == self.current_term && s.state == NodeState::Leader);
                if timed_out && !some_peer_already_leader_same_term {
                    self.become_leader(now);
                }
                Ok(())
            }
        }
    }

    /// Reconcile against `cluster` snapshots from peers: adopt a
    /// higher term and step down if any peer is ahead of us.
    ///
    /// Tiebreak for "same term + both are leader": the node with the
    /// smaller [`NodeId`] wins. A node whose `NodeId` is larger than
    /// the incumbent leader's steps down to follower.
    fn reconcile(&mut self, cluster: &[ElectionSnapshot], now: Instant) {
        let max_peer_term = cluster.iter().map(|s| s.term).max();
        if let Some(peer_term) = max_peer_term {
            if peer_term.0 > self.current_term.0 {
                self.current_term = peer_term;
                self.voted_for = None;
                self.state = NodeState::Follower;
                self.last_heartbeat = Some(now);
                return;
            }
        }
        // A peer on the same term is already a leader → step down
        // if that peer has a strictly smaller NodeId than self
        // (smaller id wins the tiebreak).
        let incumbent_smaller = cluster.iter().any(|s| {
            s.term == self.current_term && s.state == NodeState::Leader && s.node_id < self.node_id
        });
        if incumbent_smaller {
            self.state = NodeState::Follower;
            self.last_heartbeat = Some(now);
        }
    }

    /// Inject a "we just saw a leader" heartbeat at `now`. The
    /// state machine resets its election timer and, if it was a
    /// candidate for a stale term, steps back down to follower.
    pub fn on_leader_heartbeat(&mut self, now: Instant) {
        self.last_heartbeat = Some(now);
        self.heartbeats.observe(self.node_id, now);
        if self.state == NodeState::Candidate {
            self.state = NodeState::Follower;
        }
    }

    /// Force a transition to follower (used by tests + integration
    /// glue; production code will call this from RPC handlers when
    /// it sees a higher term from a peer).
    pub fn become_follower(&mut self, now: Instant) {
        self.state = NodeState::Follower;
        self.voted_for = None;
        self.last_heartbeat = Some(now);
    }

    /// Start a new election: bump term, vote for self, set state to
    /// candidate. This is a **local** state transition — collecting
    /// peer votes happens in a real RPC layer, so this function
    /// does not return [`CoordError::NoQuorum`] (the variant is
    /// kept in [`CoordError`] for the production write path).
    fn become_candidate(&mut self, now: Instant) {
        self.current_term = self.current_term.next();
        self.state = NodeState::Candidate;
        self.voted_for = Some(self.node_id);
        self.last_heartbeat = Some(now);
    }

    /// Promote the local node to leader.
    fn become_leader(&mut self, now: Instant) {
        self.state = NodeState::Leader;
        self.last_heartbeat = Some(now);
        self.heartbeats.observe(self.node_id, now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::NodeId;
    use std::time::Duration;

    fn three_node_cluster() -> (NodeId, NodeId, NodeId) {
        (NodeId::new(), NodeId::new(), NodeId::new())
    }

    #[tokio::test(start_paused = true)]
    async fn fresh_node_is_follower_at_term_zero() {
        let (me, p1, p2) = three_node_cluster();
        let mut e = Election::new(me, vec![p1, p2]);
        assert_eq!(e.state(), NodeState::Follower);
        assert_eq!(e.term(), Term::ZERO);
        let snap = e.snapshot();
        assert_eq!(snap.state, NodeState::Follower);
        assert_eq!(snap.term, Term::ZERO);
        assert!(snap.voted_for.is_none());
        // No tick yet; still follower.
        e.tick(Instant::now(), &[]).unwrap();
        assert_eq!(e.state(), NodeState::Follower);
    }

    #[tokio::test(start_paused = true)]
    async fn follower_becomes_candidate_after_timeout() {
        let (me, p1, p2) = three_node_cluster();
        let mut e = Election::new(me, vec![p1, p2]);
        // Within the election timeout window the node stays a
        // follower.
        tokio::time::advance(Duration::from_secs(2)).await;
        e.tick(Instant::now(), &[]).unwrap();
        assert_eq!(e.state(), NodeState::Follower);

        // After the timeout, the node becomes a candidate and
        // votes for itself.
        tokio::time::advance(Duration::from_secs(2)).await;
        e.tick(Instant::now(), &[]).unwrap();
        assert_eq!(e.state(), NodeState::Candidate);
        assert_eq!(e.term(), Term(1));
        assert_eq!(e.snapshot().voted_for, Some(me));
    }

    #[tokio::test(start_paused = true)]
    async fn candidate_becomes_leader_within_two_timeouts() {
        let (me, p1, p2) = three_node_cluster();
        let mut e = Election::new(me, vec![p1, p2]);

        // Tick once after the election timeout to become a
        // candidate.
        tokio::time::advance(Duration::from_secs(4)).await;
        e.tick(Instant::now(), &[]).unwrap();
        assert_eq!(e.state(), NodeState::Candidate);

        // Wait twice the election timeout → simulation elects.
        tokio::time::advance(Duration::from_secs(7)).await;
        e.tick(Instant::now(), &[]).unwrap();
        assert_eq!(e.state(), NodeState::Leader);
        assert_eq!(e.term(), Term(1));
    }

    #[tokio::test(start_paused = true)]
    async fn three_node_cluster_converges_to_one_leader() {
        // 3 nodes call tick() under a shared virtual clock,
        // passing each other the latest snapshot every slice. After
        // enough ticks exactly one should be Leader.
        //
        // Within each 1s slice we do `nodes.len()` micro-ticks so
        // that every node gets to see the freshest snapshot of its
        // peers (otherwise the first node in the iteration order
        // would tick, the second would tick against a stale view
        // of the first, etc.).
        let (a, b, c) = three_node_cluster();
        let mut nodes = [
            Election::new(a, vec![b, c]),
            Election::new(b, vec![a, c]),
            Election::new(c, vec![a, b]),
        ];

        // Drive 30 seconds of virtual time in 1s slices.
        for _ in 0..30 {
            tokio::time::advance(Duration::from_secs(1)).await;
            for _micro in 0..nodes.len() {
                let snapshots: Vec<ElectionSnapshot> =
                    nodes.iter().map(Election::snapshot).collect();
                for (i, n) in nodes.iter_mut().enumerate() {
                    let mut others = snapshots.clone();
                    others.remove(i);
                    n.tick(Instant::now(), &others).unwrap();
                }
            }
        }

        let leaders = nodes
            .iter()
            .filter(|n| n.state() == NodeState::Leader)
            .count();
        assert_eq!(
            leaders, 1,
            "expected exactly 1 leader after convergence, got {leaders}"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn leader_heartbeat_resets_follower_timer() {
        let (me, p1, p2) = three_node_cluster();
        let mut e = Election::new(me, vec![p1, p2]);
        // Pretend half a timeout passed.
        tokio::time::advance(Duration::from_millis(1_500)).await;
        e.on_leader_heartbeat(Instant::now());
        // Wait 1.5s more — total 3s — should still be follower
        // because the timer was reset at 1.5s.
        tokio::time::advance(Duration::from_millis(1_500)).await;
        e.tick(Instant::now(), &[]).unwrap();
        assert_eq!(e.state(), NodeState::Follower);
    }

    #[tokio::test(start_paused = true)]
    async fn higher_term_from_peer_steps_down() {
        // Two nodes. Node A wins the first election at term 1.
        // A higher-term peer snapshot then forces B to step down.
        let (a, b) = (NodeId::new(), NodeId::new());
        let mut a_node = Election::new(a, vec![b]);
        let mut b_node = Election::new(b, vec![a]);

        // Drive A into leadership.
        tokio::time::advance(Duration::from_secs(4)).await;
        a_node.tick(Instant::now(), &[]).unwrap();
        tokio::time::advance(Duration::from_secs(7)).await;
        a_node.tick(Instant::now(), &[]).unwrap();
        assert_eq!(a_node.state(), NodeState::Leader);

        // B, who is still a follower, sees A as leader on the
        // same term and stays follower.
        let snap = a_node.snapshot();
        b_node.tick(Instant::now(), &[snap]).unwrap();
        assert_eq!(b_node.state(), NodeState::Follower);
    }
}
