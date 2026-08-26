//! [`DataFlow`], [`FlowNode`], [`FlowEdge`] — the static DAG
//! structure the engine consumes.
//!
//! The v0.1.0 surface is minimal:
//!
//! - [`DataFlow`] — `id`, `description`, `nodes`, `edges`
//! - [`FlowNode`] — `id`, `kind` ([`NodeKind`]), `label`
//! - [`FlowEdge`] — `from`, `to` (both [`FlowNodeId`])
//! - [`NodeKind`] — `Source / Transform / Sink`
//!
//! The skeleton keeps the node kind as a separate enum so a
//! future production build can add per-kind fields (e.g.
//! `Transform { function: ... }`) without churning the
//! [`FlowNode`] struct.
//!
//! See [`DOC-MOD-003`](../docs/modules/M-03-data-flow-engine.md)
//! §3.3 for the full schema.

use std::fmt;

use serde::{Deserialize, Serialize};

/// The kind of a single flow node. v0.1.0 supports three
/// kinds; production will add per-kind configuration in B5+.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    /// The entry point of the flow. A flow MUST have
    /// exactly one `Source`.
    Source,
    /// A node that maps its input to an output.
    Transform,
    /// The exit point of the flow. A flow MUST have
    /// exactly one `Sink`.
    Sink,
}

impl NodeKind {
    /// Canonical lowercase string tag.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::Transform => "transform",
            Self::Sink => "sink",
        }
    }
}

impl fmt::Display for NodeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Opaque, flow-local node id. The skeleton keeps it as a
/// `String` so JSON exports stay human-readable; production
/// may swap to a `NodeId(Uuid)` for cross-flow references.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FlowNodeId(pub String);

impl FlowNodeId {
    /// Build a new node id from any stringy value.
    #[must_use]
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

impl fmt::Display for FlowNodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for FlowNodeId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for FlowNodeId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// A single node in the flow DAG.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlowNode {
    /// Flow-local node id.
    pub id: FlowNodeId,
    /// What this node does.
    pub kind: NodeKind,
    /// Human-readable label (e.g. "ingest orders"). The
    /// skeleton does not require this to be unique.
    #[serde(default)]
    pub label: String,
}

impl FlowNode {
    /// Build a new node with an empty label.
    #[must_use]
    pub fn new(id: impl Into<FlowNodeId>, kind: NodeKind) -> Self {
        Self {
            id: id.into(),
            kind,
            label: String::new(),
        }
    }

    /// Builder-style setter for `label`.
    #[must_use]
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }
}

/// A directed edge in the flow DAG. `from -> to` means
/// "the output of `from` feeds into `to`".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlowEdge {
    /// Source node id.
    pub from: FlowNodeId,
    /// Target node id.
    pub to: FlowNodeId,
}

impl FlowEdge {
    /// Build a new edge.
    #[must_use]
    pub fn new(from: impl Into<FlowNodeId>, to: impl Into<FlowNodeId>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
        }
    }
}

/// The full data flow definition. The skeleton treats the
/// graph as immutable; mutation happens by building a new
/// `DataFlow`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataFlow {
    /// Stable, flow-level id.
    pub id: String,
    /// Human-readable description.
    #[serde(default)]
    pub description: String,
    /// The nodes (order is not significant; the engine
    /// topologically sorts).
    pub nodes: Vec<FlowNode>,
    /// The edges (order is not significant).
    pub edges: Vec<FlowEdge>,
}

impl DataFlow {
    /// Build a new flow with no nodes / edges.
    #[must_use]
    pub fn new(id: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            description: description.into(),
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }

    /// Builder-style setter for `nodes`.
    #[must_use]
    pub fn with_nodes(mut self, nodes: Vec<FlowNode>) -> Self {
        self.nodes = nodes;
        self
    }

    /// Builder-style setter for `edges`.
    #[must_use]
    pub fn with_edges(mut self, edges: Vec<FlowEdge>) -> Self {
        self.edges = edges;
        self
    }

    /// Look up a node by id.
    #[must_use]
    pub fn node(&self, id: &str) -> Option<&FlowNode> {
        self.nodes.iter().find(|n| n.id.0 == id)
    }

    /// Build the adjacency list (`from -> [to]`).
    #[must_use]
    pub fn adjacency(&self) -> std::collections::HashMap<String, Vec<String>> {
        let mut out: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        for n in &self.nodes {
            out.entry(n.id.0.clone()).or_default();
        }
        for e in &self.edges {
            out.entry(e.from.0.clone())
                .or_default()
                .push(e.to.0.clone());
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_as_str() {
        assert_eq!(NodeKind::Source.as_str(), "source");
        assert_eq!(NodeKind::Transform.as_str(), "transform");
        assert_eq!(NodeKind::Sink.as_str(), "sink");
    }

    #[test]
    fn kind_display() {
        assert_eq!(NodeKind::Sink.to_string(), "sink");
    }

    #[test]
    fn flow_node_id_display() {
        let id = FlowNodeId::new("n-1");
        assert_eq!(id.to_string(), "n-1");
    }

    #[test]
    fn flow_node_id_from_str() {
        let a: FlowNodeId = "a".into();
        let b: FlowNodeId = String::from("b").into();
        assert_eq!(a, FlowNodeId("a".into()));
        assert_eq!(b, FlowNodeId("b".into()));
    }

    #[test]
    fn flow_node_builder() {
        let n = FlowNode::new("n-1", NodeKind::Transform).with_label("add one");
        assert_eq!(n.id, FlowNodeId("n-1".into()));
        assert_eq!(n.kind, NodeKind::Transform);
        assert_eq!(n.label, "add one");
    }

    #[test]
    fn flow_node_new_has_empty_label() {
        let n = FlowNode::new("n-1", NodeKind::Source);
        assert!(n.label.is_empty());
    }

    #[test]
    fn flow_edge_new() {
        let e = FlowEdge::new("a", "b");
        assert_eq!(e.from, FlowNodeId("a".into()));
        assert_eq!(e.to, FlowNodeId("b".into()));
    }

    #[test]
    fn data_flow_builder() {
        let flow = DataFlow::new("f-1", "demo")
            .with_nodes(vec![
                FlowNode::new("src", NodeKind::Source),
                FlowNode::new("sink", NodeKind::Sink),
            ])
            .with_edges(vec![FlowEdge::new("src", "sink")]);
        assert_eq!(flow.id, "f-1");
        assert_eq!(flow.description, "demo");
        assert_eq!(flow.nodes.len(), 2);
        assert_eq!(flow.edges.len(), 1);
        assert!(flow.node("src").is_some());
        assert!(flow.node("missing").is_none());
    }

    #[test]
    fn data_flow_adjacency_lists_successors() {
        let flow = DataFlow::new("f", "")
            .with_nodes(vec![
                FlowNode::new("a", NodeKind::Source),
                FlowNode::new("b", NodeKind::Transform),
                FlowNode::new("c", NodeKind::Sink),
            ])
            .with_edges(vec![FlowEdge::new("a", "b"), FlowEdge::new("b", "c")]);
        let adj = flow.adjacency();
        assert_eq!(
            adj.get("a").map(Vec::as_slice),
            Some(["b".to_string()].as_slice())
        );
        assert_eq!(
            adj.get("b").map(Vec::as_slice),
            Some(["c".to_string()].as_slice())
        );
        assert_eq!(adj.get("c").map(Vec::as_slice), Some([].as_slice()));
    }

    #[test]
    fn data_flow_serde_round_trip() {
        let flow = DataFlow::new("f", "demo")
            .with_nodes(vec![
                FlowNode::new("src", NodeKind::Source).with_label("in"),
                FlowNode::new("sink", NodeKind::Sink).with_label("out"),
            ])
            .with_edges(vec![FlowEdge::new("src", "sink")]);
        let json = serde_json::to_string(&flow).expect("serialize");
        let back: DataFlow = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, flow);
    }
}
