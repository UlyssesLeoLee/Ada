//! End-to-end integration tests for the M-12 canvas editor.

use ada_m12_canvas_editor::{Canvas, CanvasNode, EditHistory, EditOp, NodeKind, Position};

#[test]
fn node_lifecycle_with_history() {
    let canvas = Canvas::new("c1");
    let mut history = EditHistory::new();

    let n = CanvasNode::new(NodeKind::Block, Position::new(10, 20), "src").with_port("out");
    let id = canvas.add_node(n.clone());
    history.push(EditOp::InsertNode {
        id,
        kind: n.kind,
        position: n.position,
        label: n.label.clone(),
    });
    assert_eq!(canvas.version(), 1);
    assert!(canvas.get_node(id).is_some());

    // move
    canvas.move_node(id, Position::new(50, 60)).expect("move");
    history.push(EditOp::MoveNode {
        id,
        new_position: Position::new(50, 60),
    });
    let got = canvas.get_node(id).expect("node");
    assert_eq!(got.position, Position::new(50, 60));

    // undo the move
    let undone = history.undo().expect("undo");
    assert!(matches!(undone, EditOp::MoveNode { .. }));
    canvas.move_node(id, Position::new(10, 20)).expect("rewind");

    // redo
    let redone = history.redo().expect("redo");
    assert!(matches!(redone, EditOp::MoveNode { .. }));
    canvas
        .move_node(id, Position::new(50, 60))
        .expect("reapply");
}

#[test]
fn edge_creation_and_cascade_on_node_removal() {
    let canvas = Canvas::new("c1");
    let a = canvas.add_node(CanvasNode::new(NodeKind::Block, Position::new(0, 0), "a"));
    let b = canvas.add_node(CanvasNode::new(NodeKind::Block, Position::new(100, 0), "b"));
    let c = canvas.add_node(CanvasNode::new(NodeKind::Block, Position::new(200, 0), "c"));
    canvas
        .add_edge(ada_m12_canvas_editor::Edge::new(a, b))
        .expect("ab");
    canvas
        .add_edge(ada_m12_canvas_editor::Edge::new(b, c))
        .expect("bc");
    assert_eq!(canvas.edges().len(), 2);
    canvas.remove_node(b).expect("remove b");
    assert_eq!(canvas.edges().len(), 0);
    assert!(canvas.get_node(b).is_none());
}

#[test]
fn version_conflict_after_concurrent_writes() {
    let canvas = Canvas::new("c1");
    canvas.add_node(CanvasNode::new(NodeKind::Note, Position::new(0, 0), "x"));
    let v = canvas.version();
    canvas.add_node(CanvasNode::new(NodeKind::Note, Position::new(0, 0), "y"));
    let err = canvas.check_version(v).unwrap_err();
    let s = err.to_string();
    assert!(s.contains("version conflict"), "got: {s}");
}

#[test]
fn history_branching_resets_redo_stack() {
    let mut h = EditHistory::new();
    let id = ada_m12_canvas_editor::NodeId(uuid::Uuid::new_v4());
    h.push(EditOp::RemoveNode { id });
    h.undo().expect("undo");
    assert_eq!(h.redo_len(), 1);
    h.push(EditOp::RemoveNode { id });
    assert_eq!(h.redo_len(), 0);
}

// ===================================================================
// M-12 v0.5.0 server-side reconciliation integration tests.
//
// Gated by `--features server` so the default 5-gate CI build
// (cargo check / test / clippy / fmt) does not need the optional
// integration surface. The m13 smoke test (`tests/reconcile_smoke.rs`
// in `ada-m13-api-gateway`) exercises the cross-crate protocol;
// these tests focus on m12-internal reconcile semantics from an
// end-to-end (not just unit) perspective.
//
// 覆盖:
// - same version: 客户端 / 服务端各自加 node,merge 含两者
// - conflict LWW: 同 NodeId 不同 content,server wins
// - empty canvas: 双方都空,no panic
// - clock skew: client_version > server_version,deterministic
// ===================================================================

#[cfg(feature = "server")]
mod server_recon_integration {
    use ada_m12_canvas_editor::server_recon::reconcile_canvas_state;
    use ada_m12_canvas_editor::{Canvas, CanvasNode, NodeId, NodeKind, Position};

    fn positioned(label: &str, x: i32, y: i32) -> CanvasNode {
        CanvasNode::new(NodeKind::Block, Position::new(x, y), label)
    }

    fn node_with_id(id: NodeId, label: &str, x: i32, y: i32) -> CanvasNode {
        let mut n = CanvasNode::new(NodeKind::Block, Position::new(x, y), label);
        n.id = id;
        n
    }

    #[test]
    fn integration_same_version_merges_independent_nodes() {
        let server = Canvas::new("integration-doc");
        server.add_node(positioned("server-node", 10, 20));

        let client = Canvas::new("integration-doc");
        let cn = client.add_node(positioned("client-node", 30, 40));

        let r = reconcile_canvas_state(&server, &client, 0);

        assert_eq!(r.new_version, 2);
        assert!(!r.had_conflict);
        assert!(r.server_wins.is_empty());
        assert_eq!(r.client_wins, vec![cn]);
        assert_eq!(r.merged.nodes().len(), 2);
        assert_eq!(r.merged.name(), "integration-doc");
    }

    #[test]
    fn integration_conflict_last_write_wins_server() {
        let shared_id = NodeId::new();

        let server = Canvas::new("doc");
        let sn = server.add_node(node_with_id(shared_id, "shared", 0, 0));

        let client = Canvas::new("doc");
        client.add_node(node_with_id(shared_id, "shared", 99, 99));

        let r = reconcile_canvas_state(&server, &client, 0);

        assert_eq!(r.new_version, 2);
        assert!(r.had_conflict);
        assert_eq!(r.server_wins, vec![sn]);
        assert!(r.client_wins.is_empty());

        let merged_node = r.merged.get_node(sn).expect("node in merged");
        assert_eq!(merged_node.position, Position::new(0, 0));
    }

    #[test]
    fn integration_empty_canvas_no_panic() {
        let server = Canvas::new("empty-doc");
        let client = Canvas::new("empty-doc");

        let r = reconcile_canvas_state(&server, &client, 0);

        assert_eq!(r.new_version, 1);
        assert!(!r.had_conflict);
        assert!(r.server_wins.is_empty());
        assert!(r.client_wins.is_empty());
        assert!(r.merged.nodes().is_empty());
        assert!(r.merged.edges().is_empty());
    }

    #[test]
    fn integration_client_version_ahead_clock_skew() {
        let server = Canvas::new("doc");
        server.add_node(positioned("a", 0, 0));

        let client = Canvas::new("doc");
        client.add_node(positioned("b", 0, 0));

        let r = reconcile_canvas_state(&server, &client, 999);

        assert_eq!(r.new_version, 1000);
        assert_eq!(r.merged.nodes().len(), 2);
        assert!(!r.had_conflict);
    }

    #[test]
    fn integration_metadata_serializes_without_canvas_payload() {
        // Verify the manual `Serialize` impl on `ReconcileResult`
        // works: it carries the four scalar fields but omits
        // `merged: Canvas` (which can't be serde-derived because
        // of the Mutex<Inner> internals).
        let server = Canvas::new("doc");
        server.add_node(positioned("a", 0, 0));
        let client = Canvas::new("doc");

        let r = reconcile_canvas_state(&server, &client, 0);
        let json = serde_json::to_value(&r).expect("serialize");
        assert_eq!(json["new_version"], 2);
        assert_eq!(json["had_conflict"], false);
        assert!(json["server_wins"].is_array());
        assert!(json["client_wins"].is_array());
    }
}
