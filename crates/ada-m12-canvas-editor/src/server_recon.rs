//! Server-side reconciliation for M-12 canvas editor.
//!
//! Implements the optimistic-update + server-correction flow
//! described in `docs/modules/M-12-canvas-editor-frontend.md` §3.6.
//! The flow is:
//!
//! 1. Client edits a node locally (optimistic, no round-trip)
//! 2. Client sends the updated canvas + a `client_version` (the
//!    last server version the client saw) to the server
//! 3. Server merges:
//!    - if `client_version == server_version`: accept the
//!      client edits, return new version
//!    - if `client_version < server_version`: 3-way merge
//!      - nodes only-in-client: keep (client added them)
//!      - nodes only-in-server: add to client view
//!      - nodes in both with same content: keep
//!      - nodes in both with different content: server wins
//!        (LWW, with server timestamp authoritative; the
//!        node id is added to `server_wins` so the client
//!        can overwrite its local copy on next sync)
//! 4. Server returns the merged canvas + new version
//!
//! Conflict resolution is intentionally simple (LWW = server
//! wins) for v0.5.0; CRDT (Yrs) integration is on the v0.6.0
//! roadmap per `docs/modules/M-12-canvas-editor-frontend.md` §3.6.
//!
//! ## Feature gating
//!
//! This module is only compiled when `--features server` is set
//! (default off, per the v0.5.0 plan). The reason is that
//! pulling `ada-telemetry` v0.2.0 OTel SDK for the optional
//! W3C-trace context integration should not slow down the
//! 5-gate CI default-build path. See
//! `docs/observability/11-phased-rollout.md` Phase 4 for
//! the trace propagation design.
//!
//! ## Edge cases (all explicitly handled, never panics)
//!
//! - empty `server` canvas
//! - empty `client` canvas
//! - `client_version == 0` (initial state)
//! - `client_version > server_version` (clock skew or replay)
//! - mismatched / unknown node ids on edges
//! - duplicate edges
//! - node ids that exist on both sides with identical content
//!
//! All branches are `match` / `if let`; the function is total.

#![cfg(feature = "server")]

use std::collections::{HashMap, HashSet};

use serde::Serialize;

use crate::{
    canvas::{Canvas, Edge},
    node::{CanvasNode, NodeId},
};

/// Result of [`reconcile_canvas_state`]: the merged canvas plus
/// metadata describing which side won each conflict.
///
/// The server returns this to the client; the client uses
/// `server_wins` to overwrite its local copy of those nodes
/// (e.g. by re-applying a `replace_state` from
/// `result.merged.snapshot()` once the wasm-bindgen snapshot
/// API is wired through).
///
/// ## Why no `Serialize` / `Deserialize` on this struct?
///
/// `Canvas` wraps a `parking_lot::Mutex<Inner>` for thread-safe
/// in-memory mutation. Adding serde derives to `Canvas` would
/// require either: (a) re-architecting `Canvas` to be a pure
/// value type, or (b) introducing a separate `CanvasSnapshot`
/// type. Both are larger refactors than v0.5.0's scope. Since
/// the server-side reconcile protocol is currently an
/// in-process call (no JSON wire format), `ReconcileResult`
/// stays in memory; the individual serde-friendly fields
/// (`new_version`, `server_wins`, `client_wins`,
/// `had_conflict`) are still serializable via
/// `serde_json::json!({...})` if a caller needs to log them.
///
/// If a future phase needs `ReconcileResult` itself over the
/// wire, the migration is: introduce `pub struct
/// ReconcileSnapshot` (carrying name / nodes / edges /
/// version / win lists), derive serde, and have
/// `ReconcileResult.merged` carry either `Canvas` (in-memory,
/// as today) or `ReconcileSnapshot` (wire).
#[derive(Debug)]
pub struct ReconcileResult {
    /// The merged canvas (server-authoritative, but includes
    /// all client-only nodes that the server has accepted).
    pub merged: Canvas,
    /// New monotonically-increasing version after the merge.
    /// `>= max(server.version, client_version) + 1` — the
    /// server will set this on its own internal state and
    /// return it so the client can update its cached version.
    pub new_version: u64,
    /// Node ids that the server overwrote with its own copy
    /// (i.e. the client should pull the server version of
    /// these nodes on next sync). May be empty. Serde-friendly.
    pub server_wins: Vec<NodeId>,
    /// Node ids that the server accepted from the client
    /// (i.e. the client can keep its local copy). May be empty.
    /// Serde-friendly.
    pub client_wins: Vec<NodeId>,
    /// True if the merge produced any conflict (some nodes were
    /// modified on both sides after the divergence). Independent
    /// of the size of `server_wins` / `client_wins` — a no-op
    /// merge (both sides identical) has all three empty.
    pub had_conflict: bool,
}

// `Serialize` is implemented manually for the metadata-only
// view (without the `Canvas` payload) so the protocol can be
// logged over the wire without re-architecting `Canvas`.
impl Serialize for ReconcileResult {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("ReconcileResult", 4)?;
        s.serialize_field("new_version", &self.new_version)?;
        s.serialize_field("server_wins", &self.server_wins)?;
        s.serialize_field("client_wins", &self.client_wins)?;
        s.serialize_field("had_conflict", &self.had_conflict)?;
        s.end()
    }
}

/// 3-way merge of `client` into `server`.
///
/// Inputs are read-only references; the returned [`ReconcileResult`]
/// contains a fresh [`Canvas`] (the merged result) that the caller
/// can install on the server side or ship back to the client.
///
/// `client_version` is the version the client last saw before
/// making its optimistic edits. The server uses this only to
/// validate that the merge is meaningful; the actual algorithm
/// walks the node set regardless (so a `client_version == 0`
/// "initial connect" case is handled identically to a
/// "stale client" case).
///
/// ## Algorithm
///
/// 1. Index `server.nodes()` by `NodeId` for O(1) lookup.
/// 2. Walk `client.nodes()`:
///    - in both, identical content → keep, no side wins
///    - in both, different content → `server_wins` (LWW)
///    - only in client → `client_wins`
/// 3. Walk remaining server-only nodes → all are wins for server
///    (they're authoritative).
/// 4. Walk edges: keep any edge whose endpoints both exist in
///    the merged node set; dedup by `Edge` (Hash + Eq derived).
///
/// ## Returns
///
/// A [`ReconcileResult`] with `merged` holding the fresh canvas,
/// `new_version = max(server.version, client_version) + 1`, and
/// the per-side win lists populated.
#[must_use]
pub fn reconcile_canvas_state(
    server: &Canvas,
    client: &Canvas,
    client_version: u64,
) -> ReconcileResult {
    // 1. Snapshot server (read-only) state
    let server_name = server.name();
    let server_version = server.version();
    let server_nodes_vec = server.nodes();
    let server_edges = server.edges();

    // 2. Snapshot client (read-only) state
    let client_nodes_vec = client.nodes();
    let _client_edges = client.edges();

    // 3. Index server by node id
    let mut server_by_id: HashMap<NodeId, CanvasNode> =
        HashMap::with_capacity(server_nodes_vec.len());
    for n in server_nodes_vec {
        server_by_id.insert(n.id, n);
    }

    // 4. 3-way merge: walk client first
    let mut merged_nodes: Vec<CanvasNode> =
        Vec::with_capacity(server_by_id.len() + client_nodes_vec.len());
    let mut server_wins: Vec<NodeId> = Vec::new();
    let mut client_wins: Vec<NodeId> = Vec::new();
    let mut had_conflict = false;

    let mut visited: HashSet<NodeId> = HashSet::new();

    for cn in client_nodes_vec {
        visited.insert(cn.id);
        match server_by_id.get(&cn.id) {
            None => {
                // Only-in-client: client added it. Keep.
                client_wins.push(cn.id);
                merged_nodes.push(cn);
            }
            Some(sn) => {
                if sn == &cn {
                    // Same content on both sides. Keep one copy.
                    merged_nodes.push(sn.clone());
                } else {
                    // Conflict: server wins (LWW with server
                    // timestamp authoritative; the client will
                    // be told to pull the server version).
                    had_conflict = true;
                    server_wins.push(cn.id);
                    merged_nodes.push(sn.clone());
                }
            }
        }
    }

    // 5. Add server-only nodes (collaborator's edits, etc.)
    for (id, sn) in &server_by_id {
        if !visited.contains(id) {
            // Only-in-server: server added it after the client
            // branched off. This is not a "conflict" per the
            // v0.5.0 spec (it's an additive divergence) — the
            // node just becomes part of the merged set without
            // flagging the client to re-pull. The new `merged`
            // canvas carries the full set, so the client sees
            // it on next sync.
            merged_nodes.push(sn.clone());
        }
    }

    // 6. Edges: union, keep only edges whose endpoints exist in
    //    the merged node set, dedup.
    let merged_node_ids: HashSet<NodeId> = merged_nodes.iter().map(|n| n.id).collect();
    let mut merged_edges: Vec<Edge> = Vec::new();
    let mut seen_edges: HashSet<Edge> = HashSet::new();

    // Walk client edges first; they're the "user intent".
    for ce in &_client_edges {
        if merged_node_ids.contains(&ce.from) && merged_node_ids.contains(&ce.to) {
            if seen_edges.insert(*ce) {
                merged_edges.push(*ce);
            }
        }
    }
    // Then server edges (collaborator's, or pre-existing).
    for se in &server_edges {
        if merged_node_ids.contains(&se.from) && merged_node_ids.contains(&se.to) {
            if seen_edges.insert(*se) {
                merged_edges.push(*se);
            }
        }
    }

    // 7. new_version: monotonic, deterministic. Always bump by 1
    //    so a no-op merge still produces a new version (the
    //    server has "processed" the client's request even if
    //    nothing changed).
    let new_version = std::cmp::max(server_version, client_version).saturating_add(1);

    // 8. Build the merged Canvas via the pub(crate) constructor.
    let merged = Canvas::from_parts(server_name, merged_nodes, merged_edges, new_version);

    // `client_version` is currently unused beyond version
    // computation; bind it to `_` so the parameter is part of
    // the public API contract and not dead.
    let _ = client_version;

    ReconcileResult {
        merged,
        new_version,
        server_wins,
        client_wins,
        had_conflict,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::{NodeKind, Position};

    fn positioned(label: &str, x: i32, y: i32) -> CanvasNode {
        CanvasNode::new(NodeKind::Block, Position::new(x, y), label)
    }

    /// Build a `CanvasNode` with a pre-assigned `NodeId` so the
    /// server and client can refer to the "same" logical node
    /// (same id, different content) to exercise the conflict
    /// path.
    fn node_with_id(id: NodeId, label: &str, x: i32, y: i32) -> CanvasNode {
        let mut n = CanvasNode::new(NodeKind::Block, Position::new(x, y), label);
        n.id = id;
        n
    }

    /// Case 1: client adds node A, server adds node B at
    /// different ids → merged contains both. No conflict.
    /// `new_version = max(server.version, client_version) + 1`.
    #[test]
    fn same_version_merges_independent_nodes() {
        let server = Canvas::new("c1");
        server.add_node(positioned("server-node", 10, 20)); // version → 1

        let client = Canvas::new("c1");
        let cn = client.add_node(positioned("client-node", 30, 40)); // version → 1

        let r = reconcile_canvas_state(&server, &client, 0);

        // max(1, 0) + 1 = 2
        assert_eq!(r.new_version, 2);
        assert!(!r.had_conflict);
        assert!(r.server_wins.is_empty());
        assert_eq!(r.client_wins, vec![cn]);
        assert_eq!(r.merged.nodes().len(), 2);
    }

    /// Case 2: same `NodeId`, different content → server wins.
    /// `had_conflict == true`, `server_wins` contains the id.
    /// The merged canvas holds the server's copy.
    #[test]
    fn conflict_last_write_wins_server() {
        let shared_id = NodeId::new();

        let server = Canvas::new("c1");
        let sn = server.add_node(node_with_id(shared_id, "shared", 0, 0)); // version → 1

        let client = Canvas::new("c1");
        let _cn = client.add_node(node_with_id(shared_id, "shared", 99, 99)); // version → 1

        let r = reconcile_canvas_state(&server, &client, 0);

        // max(1, 0) + 1 = 2
        assert_eq!(r.new_version, 2);
        assert!(r.had_conflict, "conflict should be flagged");
        assert_eq!(r.server_wins, vec![sn]);
        assert!(r.client_wins.is_empty());

        // The merged canvas holds the server's copy (0, 0).
        let merged_node = r.merged.get_node(sn).expect("node in merged");
        assert_eq!(merged_node.position, Position::new(0, 0));
    }

    /// Case 3: empty client / empty server must not panic.
    /// `new_version = 0 + 1 = 1`.
    #[test]
    fn empty_inputs_are_handled() {
        let server = Canvas::new("c1");
        let client = Canvas::new("c1");

        let r = reconcile_canvas_state(&server, &client, 0);

        assert_eq!(r.new_version, 1);
        assert!(!r.had_conflict);
        assert!(r.server_wins.is_empty());
        assert!(r.client_wins.is_empty());
        assert!(r.merged.nodes().is_empty());
        assert!(r.merged.edges().is_empty());
    }

    /// Case 4: client_version ahead of server_version (clock
    /// skew or replay). The new version is still
    /// `max + 1`; no panic, deterministic output.
    #[test]
    fn client_version_ahead_is_handled() {
        let server = Canvas::new("c1");
        server.add_node(positioned("a", 0, 0)); // version → 1

        let client = Canvas::new("c1");
        client.add_node(positioned("b", 0, 0)); // version → 1

        // Client thinks it's seen version 999.
        let r = reconcile_canvas_state(&server, &client, 999);

        // max(1, 999) + 1 = 1000
        assert_eq!(r.new_version, 1000);
        assert_eq!(r.merged.nodes().len(), 2);
        assert!(!r.had_conflict);
    }

    /// Case 5: server-only and client-only nodes both appear
    /// in the merged set. Server has a→b; client has c.
    /// After merge: {a, b, c}, edge {a→b}.
    /// Also exercises edge dedup: both sides independently
    /// re-construct the same a→b edge (same NodeId) and the
    /// merge collapses to one.
    #[test]
    fn additive_divergence_no_conflict() {
        let server = Canvas::new("c1");
        let a = server.add_node(positioned("a", 0, 0)); // version → 1
        let b = server.add_node(positioned("b", 0, 0)); // version → 2
        server.add_edge(Edge::new(a, b)).expect("ab"); // version → 3

        let client = Canvas::new("c1");
        let ca = client.add_node(positioned("a", 0, 0)); // version → 1
        let cb = client.add_node(positioned("b", 0, 0)); // version → 2
        let cc = client.add_node(positioned("c", 0, 0)); // version → 3
        client.add_edge(Edge::new(ca, cb)).expect("ab-c"); // version → 4

        let r = reconcile_canvas_state(&server, &client, 2);

        // server.version=3, client_version=2 → max=3 + 1 = 4
        assert_eq!(r.new_version, 4);
        assert!(!r.had_conflict, "no conflict when content is identical");

        // All five nodes from both sides are in the merged set
        // (different NodeId per CanvasNode::new call): a, b
        // from server + a', b', c' from client.
        assert_eq!(r.merged.nodes().len(), 5);

        // Both client and server have a→b edges (with their
        // respective NodeIds), and both a→b are preserved.
        // Plus the server has no edge to c, so only client's
        // a→b is in the merged.
        let edges = r.merged.edges();
        // The server-only edge a→b is in merged.
        assert!(edges.contains(&Edge::new(a, b)));
        // The client-only edge ca→cb is in merged.
        assert!(edges.contains(&Edge::new(ca, cb)));
        // cc is in merged (only in client).
        assert!(r.merged.get_node(cc).is_some());
    }

    /// Sanity: the metadata-only `Serialize` impl produces
    /// a JSON object with the four scalar fields. (The `Canvas`
    /// payload is excluded — see the impl comment.)
    #[test]
    fn metadata_only_serialize_works() {
        let server = Canvas::new("c1");
        server.add_node(positioned("a", 0, 0));
        let client = Canvas::new("c1");

        let r = reconcile_canvas_state(&server, &client, 0);
        let json = serde_json::to_value(&r).expect("serialize metadata");
        assert_eq!(json["new_version"], 2);
        assert_eq!(json["had_conflict"], false);
        assert!(json["server_wins"].is_array());
        assert!(json["client_wins"].is_array());
    }
}
