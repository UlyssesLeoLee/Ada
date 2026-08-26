//! [`DataFlowEngine`] trait + the in-process [`InMemoryEngine`]
//! implementation.
//!
//! The v0.1.0 surface is intentionally minimal:
//!
//! - [`DataFlowEngine::execute`] is `async`, takes a
//!   `&DataFlow` and a `serde_json::Value` input, and
//!   returns `Result<serde_json::Value, FlowError>`.
//! - The v0.1.0 engine is **sequential**: it topologically
//!   sorts the DAG, then runs each node in order. Production
//!   will parallelize independent branches in B5+.
//!
//! Node bodies are supplied as a `HashMap<String, Box<dyn
//! NodeBody>>` keyed by node id. The trait shape lets the
//! production build swap in a remote execution backend
//! (WebAssembly, gRPC) without changing the call site.
//!
//! See [`DOC-MOD-003`](../docs/modules/M-03-data-flow-engine.md)
//! §3.4 for the full execution pipeline.

use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;

use async_trait::async_trait;
use serde_json::Value;

use crate::error::{FlowError, Result};
use crate::flow::{DataFlow, FlowNode, NodeKind};

/// Per-node behaviour. The skeleton keeps the signature
/// minimal: take a `Value` in, return a `Value` out. The
/// engine wraps any error in
/// [`FlowError::ExecutionFailed`] so node authors do not
/// have to reach into the engine's error type.
#[async_trait]
pub trait NodeBody: Send + Sync {
    /// Apply this node to `input`.
    async fn run(&self, node_id: &str, input: Value) -> Result<Value>;
}

/// Boxed closure shape used by [`FnNode`]. Extracted as a
/// type alias so the `FnNode` field type stays readable and
/// clippy's `type_complexity` lint stays quiet.
pub type BoxedNodeFn = Box<dyn for<'a> Fn(&'a str, Value) -> Result<Value> + Send + Sync>;

/// Adapter that lets any `Fn(&str, Value) -> Result<Value>`
/// closure be used as a [`NodeBody`].
///
/// The skeleton stores the closure as a boxed trait object
/// (`Box<dyn Fn(...) + Send + Sync>`) so the `NodeBody`
/// trait itself is dyn-compatible and the lifetime
/// constraints on the closure do not interfere with
/// callers in async contexts.
pub struct FnNode {
    inner: BoxedNodeFn,
}

impl fmt::Debug for FnNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FnNode").finish_non_exhaustive()
    }
}

impl FnNode {
    /// Build a `FnNode` from any compatible closure.
    pub fn new<F>(f: F) -> Self
    where
        F: for<'a> Fn(&'a str, Value) -> Result<Value> + Send + Sync + 'static,
    {
        Self { inner: Box::new(f) }
    }
}

#[async_trait]
impl NodeBody for FnNode {
    async fn run(&self, node_id: &str, input: Value) -> Result<Value> {
        (self.inner)(node_id, input)
    }
}

/// The engine trait. The skeleton exposes a single
/// `execute` method; production will add `compile`, `cancel`,
/// etc.
#[async_trait]
pub trait DataFlowEngine: Send + Sync {
    /// Execute `flow` with `input` as the entry value. The
    /// `bodies` map supplies the per-node behaviour; nodes
    /// without a body behave as identity transforms (input
    /// passes through unchanged) — handy in tests.
    async fn execute(
        &self,
        flow: &DataFlow,
        bodies: &HashMap<String, Box<dyn NodeBody>>,
        input: Value,
    ) -> Result<Value>;
}

/// In-process sequential engine.
///
/// The v0.1.0 engine validates the flow (no unknown
/// nodes, no cycles, no duplicate ids), topologically
/// sorts, and then runs each node in order. The
/// accumulator `current` starts at `input`; the final
/// `current` is returned.
#[derive(Debug, Default, Clone, Copy)]
pub struct InMemoryEngine;

impl InMemoryEngine {
    /// Build a new in-process engine.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Topologically sort `flow`. Returns the node ids in
    /// execution order, or a [`FlowError::CyclicGraph`] if
    /// the graph has a cycle.
    pub fn topo_sort(flow: &DataFlow) -> Result<Vec<String>> {
        // First, validate that every edge references a
        // known node.
        let known: HashSet<&str> = flow.nodes.iter().map(|n| n.id.0.as_str()).collect();
        for e in &flow.edges {
            if !known.contains(e.from.0.as_str()) {
                return Err(FlowError::UnknownNode(e.from.0.clone()));
            }
            if !known.contains(e.to.0.as_str()) {
                return Err(FlowError::UnknownNode(e.to.0.clone()));
            }
        }
        // Kahn's algorithm: count in-degrees.
        let mut in_deg: HashMap<&str, usize> = known.iter().map(|id| (*id, 0)).collect();
        for e in &flow.edges {
            *in_deg.get_mut(e.to.0.as_str()).expect("to known") += 1;
        }
        let mut queue: VecDeque<&str> = in_deg
            .iter()
            .filter_map(|(id, deg)| if *deg == 0 { Some(*id) } else { None })
            .collect();
        let mut order = Vec::with_capacity(known.len());
        while let Some(id) = queue.pop_front() {
            order.push(id.to_string());
            for e in &flow.edges {
                if e.from.0 == id {
                    if let Some(deg) = in_deg.get_mut(e.to.0.as_str()) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push_back(e.to.0.as_str());
                        }
                    }
                }
            }
        }
        if order.len() != known.len() {
            return Err(FlowError::CyclicGraph {
                path: collect_cycle_hint(flow),
            });
        }
        Ok(order)
    }
}

#[async_trait]
impl DataFlowEngine for InMemoryEngine {
    async fn execute(
        &self,
        flow: &DataFlow,
        bodies: &HashMap<String, Box<dyn NodeBody>>,
        input: Value,
    ) -> Result<Value> {
        // Reject duplicate node ids up front.
        let mut seen: HashSet<&str> = HashSet::new();
        for n in &flow.nodes {
            if !seen.insert(n.id.0.as_str()) {
                return Err(FlowError::BackendError(format!(
                    "duplicate node id: {}",
                    n.id.0
                )));
            }
        }
        let order = Self::topo_sort(flow)?;
        let node_by_id: HashMap<&str, &FlowNode> =
            flow.nodes.iter().map(|n| (n.id.0.as_str(), n)).collect();

        // The skeleton treats the order as a strict chain:
        // each node's output feeds the next node. Real DAGs
        // with branches will land in B5+.
        let mut current = input;
        for id in &order {
            let node = node_by_id
                .get(id.as_str())
                .copied()
                .ok_or_else(|| FlowError::UnknownNode(id.clone()))?;
            // Type contract: Source accepts any input;
            // Transform expects an object; Sink expects any
            // input. The skeleton only enforces Source/Transform.
            match node.kind {
                NodeKind::Source | NodeKind::Sink => {}
                NodeKind::Transform => {
                    if !current.is_object() {
                        return Err(FlowError::TypeMismatch {
                            node: id.clone(),
                            expected: "object",
                            actual: json_type(&current).to_string(),
                        });
                    }
                }
            }
            if let Some(body) = bodies.get(id) {
                current = body.run(id, current).await?;
            }
            // No body: identity passthrough (handy for tests
            // that just want to validate the graph shape).
        }
        Ok(current)
    }
}

fn json_type(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn dfs_cycle(
    node: &str,
    adj: &HashMap<String, Vec<String>>,
    visited: &mut HashSet<String>,
    stack: &mut Vec<String>,
) -> Option<Vec<String>> {
    if !visited.insert(node.to_string()) {
        // Already visited; not on the current stack,
        // so this is not a back-edge.
        return None;
    }
    stack.push(node.to_string());
    if let Some(succs) = adj.get(node) {
        for s in succs {
            if stack.iter().any(|n| n == s) {
                // Back-edge: return the cycle path.
                let start = stack.iter().position(|n| n == s).expect("position");
                let mut path = stack[start..].to_vec();
                path.push(s.clone());
                return Some(path);
            }
            if let Some(cycle) = dfs_cycle(s, adj, visited, stack) {
                return Some(cycle);
            }
        }
    }
    stack.pop();
    None
}

fn collect_cycle_hint(flow: &DataFlow) -> String {
    // Heuristic: find a back-edge by walking from each
    // node and reporting the first cycle. The skeleton
    // does a depth-first search and returns the first
    // back-edge it finds.
    let adj = flow.adjacency();
    let mut visited: HashSet<String> = HashSet::new();
    let mut stack: Vec<String> = Vec::new();
    for n in &flow.nodes {
        if !visited.contains(&n.id.0) {
            if let Some(path) = dfs_cycle(&n.id.0, &adj, &mut visited, &mut stack) {
                return path.join(" -> ");
            }
        }
    }
    "<unknown>".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flow::FlowEdge;
    use serde_json::json;

    fn linear_flow() -> DataFlow {
        DataFlow::new("f", "linear")
            .with_nodes(vec![
                FlowNode::new("src", NodeKind::Source),
                FlowNode::new("t1", NodeKind::Transform),
                FlowNode::new("sink", NodeKind::Sink),
            ])
            .with_edges(vec![
                FlowEdge::new("src", "t1"),
                FlowEdge::new("t1", "sink"),
            ])
    }

    #[test]
    fn topo_sort_linear_order() {
        let flow = linear_flow();
        let order = InMemoryEngine::topo_sort(&flow).expect("ok");
        assert_eq!(order, vec!["src", "t1", "sink"]);
    }

    #[test]
    fn topo_sort_detects_cycle() {
        let flow = DataFlow::new("f", "")
            .with_nodes(vec![
                FlowNode::new("a", NodeKind::Transform),
                FlowNode::new("b", NodeKind::Transform),
            ])
            .with_edges(vec![FlowEdge::new("a", "b"), FlowEdge::new("b", "a")]);
        let err = InMemoryEngine::topo_sort(&flow).expect_err("cycle");
        assert!(matches!(err, FlowError::CyclicGraph { .. }));
    }

    #[test]
    fn topo_sort_detects_unknown_node() {
        let flow = DataFlow::new("f", "")
            .with_nodes(vec![FlowNode::new("a", NodeKind::Source)])
            .with_edges(vec![FlowEdge::new("a", "missing")]);
        let err = InMemoryEngine::topo_sort(&flow).expect_err("unknown");
        assert!(matches!(err, FlowError::UnknownNode(_)));
    }

    #[tokio::test]
    async fn execute_runs_chain_in_order() {
        let flow = linear_flow();
        let mut bodies: HashMap<String, Box<dyn NodeBody>> = HashMap::new();
        bodies.insert(
            "t1".into(),
            Box::new(FnNode::new(|_id, v: Value| {
                let mut obj = v.as_object().cloned().unwrap_or_default();
                obj.insert("visited_t1".into(), json!(true));
                Ok(Value::Object(obj))
            })),
        );
        let engine = InMemoryEngine::new();
        let out = engine
            .execute(&flow, &bodies, json!({"hello": "world"}))
            .await
            .expect("ok");
        assert_eq!(out, json!({"hello": "world", "visited_t1": true}));
    }

    #[tokio::test]
    async fn execute_rejects_transform_on_non_object() {
        let flow = linear_flow();
        let bodies: HashMap<String, Box<dyn NodeBody>> = HashMap::new();
        let engine = InMemoryEngine::new();
        let err = engine
            .execute(&flow, &bodies, json!(7))
            .await
            .expect_err("type");
        assert!(matches!(err, FlowError::TypeMismatch { .. }));
    }

    #[tokio::test]
    async fn execute_passes_through_when_no_bodies() {
        let flow = linear_flow();
        let bodies: HashMap<String, Box<dyn NodeBody>> = HashMap::new();
        let engine = InMemoryEngine::new();
        let out = engine
            .execute(&flow, &bodies, json!({"k": 1}))
            .await
            .expect("ok");
        assert_eq!(out, json!({"k": 1}));
    }
}
