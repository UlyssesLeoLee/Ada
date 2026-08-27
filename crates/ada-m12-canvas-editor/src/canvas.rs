//! In-memory canvas document with optimistic-concurrency version.

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::{CanvasError, Result};
use crate::node::{CanvasNode, NodeId, Position};

/// Directed edge between two nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Edge {
    /// Source node id.
    pub from: NodeId,
    /// Target node id.
    pub to: NodeId,
}

impl Edge {
    /// Create a new edge.
    #[must_use]
    pub const fn new(from: NodeId, to: NodeId) -> Self {
        Self { from, to }
    }
}

/// In-memory canvas document.
#[derive(Debug, Default)]
pub struct Canvas {
    /// Mutex-guarded state. `pub(crate)` so the feature-gated WASM
    /// bindings (`src/wasm.rs`) can do bulk snapshot / restore via
    /// `replace_state` without going through the public mutation API
    /// one node at a time.
    pub(crate) inner: Mutex<Inner>,
}

#[derive(Debug, Default)]
pub(crate) struct Inner {
    /// Document name (e.g. "ada-flow-1").
    pub(crate) name: String,
    /// All nodes, keyed by id.
    pub(crate) nodes: HashMap<NodeId, CanvasNode>,
    /// All edges. Stored as a list (not a set) so the order is
    /// deterministic and the same edge can be inspected for
    /// ordering.
    pub(crate) edges: Vec<Edge>,
    /// Optimistic-concurrency version, bumped on every write.
    pub(crate) version: u64,
}

impl Canvas {
    /// Create a new empty canvas with `name`.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            inner: Mutex::new(Inner {
                name: name.into(),
                ..Inner::default()
            }),
        }
    }

    /// Build a canvas from explicit parts. `pub(crate)` so the
    /// feature-gated `server_recon` module can construct a merged
    /// canvas without going through the version-bumping public
    /// mutators one node at a time.
    ///
    /// Caller is responsible for supplying a `version` that is
    /// consistent with the merge result (typically
    /// `max(server.version, client.version) + 1`). Node ids in
    /// `nodes` should be unique; duplicates are silently
    /// overwritten (last write wins at the `HashMap` level —
    /// callers that need strict dedup must pre-filter).
    ///
    /// Gated by `feature = "server"` to avoid dead-code warnings
    /// in the default 5-gate CI build (the only caller,
    /// `server_recon`, is itself feature-gated).
    #[cfg(feature = "server")]
    pub(crate) fn from_parts(
        name: String,
        nodes: Vec<CanvasNode>,
        edges: Vec<Edge>,
        version: u64,
    ) -> Self {
        let nodes_map = nodes.into_iter().map(|n| (n.id, n)).collect();
        Self {
            inner: Mutex::new(Inner {
                name,
                nodes: nodes_map,
                edges,
                version,
            }),
        }
    }

    /// Current version.
    #[must_use]
    pub fn version(&self) -> u64 {
        self.inner.lock().version
    }

    /// Document name.
    #[must_use]
    pub fn name(&self) -> String {
        self.inner.lock().name.clone()
    }

    /// Snapshot of all nodes (in unspecified order).
    #[must_use]
    pub fn nodes(&self) -> Vec<CanvasNode> {
        self.inner.lock().nodes.values().cloned().collect()
    }

    /// Snapshot of all edges.
    #[must_use]
    pub fn edges(&self) -> Vec<Edge> {
        self.inner.lock().edges.clone()
    }

    /// Look up a node by id.
    #[must_use]
    pub fn get_node(&self, id: NodeId) -> Option<CanvasNode> {
        self.inner.lock().nodes.get(&id).cloned()
    }

    /// Add a node. Returns the assigned id. Bumps version.
    pub fn add_node(&self, node: CanvasNode) -> NodeId {
        let id = node.id;
        let mut g = self.inner.lock();
        g.nodes.insert(id, node);
        g.version += 1;
        id
    }

    /// Remove a node. Also removes any edges incident to it.
    /// Bumps version.
    pub fn remove_node(&self, id: NodeId) -> Result<()> {
        let mut g = self.inner.lock();
        if g.nodes.remove(&id).is_none() {
            return Err(CanvasError::NodeNotFound(id));
        }
        g.edges.retain(|e| e.from != id && e.to != id);
        g.version += 1;
        Ok(())
    }

    /// Move a node to a new position. Bumps version.
    pub fn move_node(&self, id: NodeId, new_pos: Position) -> Result<()> {
        let mut g = self.inner.lock();
        let node = g.nodes.get_mut(&id).ok_or(CanvasError::NodeNotFound(id))?;
        node.position = new_pos;
        g.version += 1;
        Ok(())
    }

    /// Add an edge. Errors on self-loop, missing endpoint, or
    /// duplicate edge. Bumps version.
    pub fn add_edge(&self, edge: Edge) -> Result<()> {
        if edge.from == edge.to {
            return Err(CanvasError::InvalidEdge {
                reason: "self-loop is not allowed".into(),
            });
        }
        let mut g = self.inner.lock();
        if !g.nodes.contains_key(&edge.from) {
            return Err(CanvasError::InvalidEdge {
                reason: format!("source node not found: {}", edge.from),
            });
        }
        if !g.nodes.contains_key(&edge.to) {
            return Err(CanvasError::InvalidEdge {
                reason: format!("target node not found: {}", edge.to),
            });
        }
        if g.edges.contains(&edge) {
            return Err(CanvasError::InvalidEdge {
                reason: "edge already exists".into(),
            });
        }
        g.edges.push(edge);
        g.version += 1;
        Ok(())
    }

    /// Check the version matches `expected`; if not, return
    /// `VersionConflict`.
    pub fn check_version(&self, expected: u64) -> Result<()> {
        let g = self.inner.lock();
        if g.version != expected {
            return Err(CanvasError::VersionConflict {
                expected,
                current: g.version,
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::NodeKind;

    fn node(label: &str) -> CanvasNode {
        CanvasNode::new(NodeKind::Block, Position::new(0, 0), label)
    }

    #[test]
    fn new_canvas_has_version_zero() {
        let c = Canvas::new("test");
        assert_eq!(c.version(), 0);
        assert_eq!(c.name(), "test");
    }

    #[test]
    fn add_node_bumps_version() {
        let c = Canvas::new("t");
        let id = c.add_node(node("a"));
        assert_eq!(c.version(), 1);
        assert!(c.get_node(id).is_some());
    }

    #[test]
    fn remove_unknown_node_errors() {
        let c = Canvas::new("t");
        let err = c.remove_node(NodeId::new()).unwrap_err();
        assert!(matches!(err, CanvasError::NodeNotFound(_)));
    }

    #[test]
    fn remove_node_cascades_to_edges() {
        let c = Canvas::new("t");
        let a = c.add_node(node("a"));
        let b = c.add_node(node("b"));
        c.add_edge(Edge::new(a, b)).expect("edge");
        assert_eq!(c.edges().len(), 1);
        c.remove_node(a).expect("remove");
        assert!(c.edges().is_empty());
    }

    #[test]
    fn move_node_updates_position() {
        let c = Canvas::new("t");
        let id = c.add_node(node("a"));
        c.move_node(id, Position::new(50, 60)).expect("move");
        let got = c.get_node(id).expect("node");
        assert_eq!(got.position, Position::new(50, 60));
    }

    #[test]
    fn add_edge_rejects_self_loop() {
        let c = Canvas::new("t");
        let a = c.add_node(node("a"));
        let err = c.add_edge(Edge::new(a, a)).unwrap_err();
        let s = err.to_string();
        assert!(s.contains("self-loop"), "got: {s}");
    }

    #[test]
    fn add_edge_rejects_missing_endpoint() {
        let c = Canvas::new("t");
        let a = c.add_node(node("a"));
        let ghost = NodeId::new();
        let err = c.add_edge(Edge::new(a, ghost)).unwrap_err();
        let s = err.to_string();
        assert!(s.contains("target"), "got: {s}");
    }

    #[test]
    fn add_edge_rejects_duplicate() {
        let c = Canvas::new("t");
        let a = c.add_node(node("a"));
        let b = c.add_node(node("b"));
        c.add_edge(Edge::new(a, b)).expect("first");
        let err = c.add_edge(Edge::new(a, b)).unwrap_err();
        let s = err.to_string();
        assert!(s.contains("already exists"), "got: {s}");
    }

    #[test]
    fn version_check_passes_on_match() {
        let c = Canvas::new("t");
        c.add_node(node("a"));
        c.check_version(1).expect("match");
    }

    #[test]
    fn version_check_fails_on_mismatch() {
        let c = Canvas::new("t");
        c.add_node(node("a"));
        c.add_node(node("b"));
        let err = c.check_version(1).unwrap_err();
        match err {
            CanvasError::VersionConflict { expected, current } => {
                assert_eq!(expected, 1);
                assert_eq!(current, 2);
            }
            other => panic!("expected VersionConflict, got {other:?}"),
        }
    }
}
