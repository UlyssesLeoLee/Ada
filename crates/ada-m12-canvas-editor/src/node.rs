//! Canvas node model.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Stable, opaque identifier for a canvas node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub Uuid);

impl NodeId {
    /// Create a fresh random id.
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

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The three canonical node kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeKind {
    /// A functional block (e.g. data source, transform, sink).
    Block,
    /// A connector that joins two blocks (an inline pipeline).
    Connector,
    /// A free-form annotation; not connected to anything.
    Note,
}

impl std::fmt::Display for NodeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Block => "block",
            Self::Connector => "connector",
            Self::Note => "note",
        };
        f.write_str(s)
    }
}

/// 2-D position. Coordinates are unit-less; the skeleton does
/// not enforce bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Position {
    /// X coordinate.
    pub x: i32,
    /// Y coordinate.
    pub y: i32,
}

impl Position {
    /// Create a new position.
    #[must_use]
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

/// A single port on a node. `name` is unique within a node
/// (the skeleton does not enforce uniqueness; callers should).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Port {
    /// Port name (e.g. "input", "output", "error").
    pub name: String,
}

impl Port {
    /// Create a new port.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

/// A canvas node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanvasNode {
    /// Stable id.
    pub id: NodeId,
    /// What kind.
    pub kind: NodeKind,
    /// Position on the canvas.
    pub position: Position,
    /// Human-readable label.
    pub label: String,
    /// Input / output ports.
    pub ports: Vec<Port>,
}

impl CanvasNode {
    /// Create a new node at `position`. The id is generated;
    /// `label` and `ports` are caller-supplied.
    #[must_use]
    pub fn new(kind: NodeKind, position: Position, label: impl Into<String>) -> Self {
        Self {
            id: NodeId::new(),
            kind,
            position,
            label: label.into(),
            ports: Vec::new(),
        }
    }

    /// Builder-style: add a port.
    #[must_use]
    pub fn with_port(mut self, name: impl Into<String>) -> Self {
        self.ports.push(Port::new(name));
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_id_unique() {
        assert_ne!(NodeId::new(), NodeId::new());
    }

    #[test]
    fn node_kind_display() {
        assert_eq!(NodeKind::Block.to_string(), "block");
        assert_eq!(NodeKind::Connector.to_string(), "connector");
        assert_eq!(NodeKind::Note.to_string(), "note");
    }

    #[test]
    fn position_construction() {
        let p = Position::new(10, 20);
        assert_eq!(p.x, 10);
        assert_eq!(p.y, 20);
    }

    #[test]
    fn canvas_node_builder() {
        let n = CanvasNode::new(NodeKind::Block, Position::new(0, 0), "src").with_port("out");
        assert_eq!(n.kind, NodeKind::Block);
        assert_eq!(n.label, "src");
        assert_eq!(n.ports.len(), 1);
        assert_eq!(n.ports[0].name, "out");
    }
}
