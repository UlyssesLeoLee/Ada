//! M-13 ↔ M-12 server-reconciliation smoke test.
//!
//! Verifies the cross-crate protocol:
//!
//! - the m12 `reconcile_canvas_state` function returns a result
//!   the m13 handler can ship to the client;
//! - the result carries the four scalar fields (`new_version` /
//!   `server_wins` / `client_wins` / `had_conflict`) plus the in-memory
//!   merged `Canvas`;
//! - applying the merged canvas's nodes/edges to a fresh
//!   downstream `Canvas` reproduces the expected end state.
//!
//! Per `docs/observability/11-phased-rollout.md` §6 Phase 4 +
//! `docs/observability/05-tracing-design.md` §3.4, the W3C
//! `traceparent` header is propagated by `tower-http`'s
//! `TraceLayer` (already wired in m13's `Cargo.toml`); we do
//! not need to test `OTel` SDK internals here — that's the
//! `ada-telemetry` v0.2.0 test surface's job.
//!
//! The m12 dependency is declared in `[dev-dependencies]` with
//! the `server` feature, so it only pulls the
//! `server_recon` module when this test is built (not for
//! the regular `cargo build -p ada-m13-api-gateway` path).

use std::sync::Arc;

use ada_m12_canvas_editor::server_recon::reconcile_canvas_state;
use ada_m12_canvas_editor::{Canvas, CanvasNode, NodeId, NodeKind, Position};
use ada_m13_api_gateway::{AppState, MemoryHealthCheck};

fn positioned(label: &str, x: i32, y: i32) -> CanvasNode {
    CanvasNode::new(NodeKind::Block, Position::new(x, y), label)
}

fn node_with_id(id: NodeId, label: &str, x: i32, y: i32) -> CanvasNode {
    let mut n = CanvasNode::new(NodeKind::Block, Position::new(x, y), label);
    n.id = id;
    n
}

/// Smoke: building the `AppState` and router (the m13 "container")
/// does not require m12's `server_recon` types — the m12 module
/// is just a leaf dependency here. We still verify it imports
/// cleanly to catch a feature-gating regression.
#[test]
fn appstate_builds_without_reconcile_payload() {
    let state = AppState::new("ada-gateway-recon", Arc::new(MemoryHealthCheck::new()));
    let _router = ada_m13_api_gateway::build_router(state);
}

/// Smoke: m12's reconcile can be invoked from m13's test
/// surface. Simulates a client that has made one optimistic edit
/// (a node) while the server has independently received a
/// different node. The merge should contain both, no conflict.
#[test]
fn reconcile_endpoint_accepts_client_version() {
    let server = Canvas::new("recon-doc");
    let sn = server.add_node(positioned("server-side", 0, 0)); // version → 1

    let client = Canvas::new("recon-doc");
    let cn = client.add_node(positioned("client-side", 100, 100)); // version → 1

    let r = reconcile_canvas_state(&server, &client, 0);

    // No conflict: both nodes are new (different NodeIds).
    assert!(!r.had_conflict);
    // new_version = max(1, 0) + 1 = 2
    assert_eq!(r.new_version, 2);
    // Server's node was already in server, not a "client win".
    assert!(r.server_wins.is_empty());
    // Client's node was a "client win".
    assert_eq!(r.client_wins, vec![cn]);
    // Merged canvas has both nodes.
    assert_eq!(r.merged.nodes().len(), 2);
    assert!(r.merged.get_node(sn).is_some());
    assert!(r.merged.get_node(cn).is_some());
}

/// Smoke: when client and server both edit the SAME node
/// (same `NodeId`, different content), the server wins and
/// `server_wins` carries the id. The merged canvas holds the
/// server's copy.
#[test]
fn reconcile_endpoint_conflict_marks_server_wins() {
    let shared = NodeId::new();

    let server = Canvas::new("recon-doc");
    let sn = server.add_node(node_with_id(shared, "shared", 10, 10));

    let client = Canvas::new("recon-doc");
    client.add_node(node_with_id(shared, "shared", 99, 99));

    let r = reconcile_canvas_state(&server, &client, 0);

    assert!(r.had_conflict);
    assert_eq!(r.server_wins, vec![sn]);
    assert!(r.client_wins.is_empty());

    // Server's version of the node is in the merged canvas.
    let merged_node = r.merged.get_node(sn).expect("node in merged");
    assert_eq!(merged_node.position, Position::new(10, 10));
}

/// Smoke: the metadata-only JSON view of `ReconcileResult`
/// (the manual `Serialize` impl that excludes the in-memory
/// `Canvas` payload) contains exactly the four scalar fields
/// the m13 handler would log or return in a response header.
#[test]
fn reconcile_metadata_serializes_for_logging() {
    let server = Canvas::new("recon-doc");
    server.add_node(positioned("a", 0, 0));
    let client = Canvas::new("recon-doc");

    let r = reconcile_canvas_state(&server, &client, 0);
    let json = serde_json::to_value(&r).expect("serialize metadata");

    assert!(json.get("new_version").is_some());
    assert!(json.get("server_wins").is_some());
    assert!(json.get("client_wins").is_some());
    assert!(json.get("had_conflict").is_some());
    // The `Canvas` payload is intentionally excluded from
    // this view (see the comment in `server_recon.rs`).
    assert!(json.get("merged").is_none());
}

/// Smoke: applying the merged canvas's nodes/edges into a
/// fresh downstream `Canvas` (e.g. one that the m13 handler
/// will hand to a `replace_state` call) reproduces the merged
/// state. This exercises the m12 public surface
/// (`Canvas::new` + `add_node` + `add_edge`) that m13 uses.
#[test]
fn reconcile_merged_state_can_be_replayed_into_fresh_canvas() {
    let server = Canvas::new("recon-doc");
    server.add_node(positioned("a", 0, 0));

    let client = Canvas::new("recon-doc");
    client.add_node(positioned("b", 0, 0));

    let r = reconcile_canvas_state(&server, &client, 0);

    // Build a fresh downstream Canvas from the merge result.
    let downstream = Canvas::new(r.merged.name());
    for n in r.merged.nodes() {
        downstream.add_node(n);
    }
    for e in r.merged.edges() {
        downstream.add_edge(e).expect("replay edge");
    }

    assert_eq!(downstream.name(), "recon-doc");
    assert_eq!(downstream.nodes().len(), 2);
    assert_eq!(downstream.version(), r.merged.nodes().len() as u64);
}
