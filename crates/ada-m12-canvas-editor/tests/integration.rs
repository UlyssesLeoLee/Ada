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
