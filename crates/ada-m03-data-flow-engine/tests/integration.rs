//! Integration tests for the v0.1.0 data flow engine.
//!
//! The v0.1.0 surface is in-process, so the "integration"
//! tests exercise the public surface the way a real
//! canvas-driven loop would: build a `DataFlow`, supply
//! per-node `NodeBody`s, call `execute` from a `tokio`
//! task, and assert the shape of the returned `Value`.

use std::collections::HashMap;

use ada_m03_data_flow_engine::{
    DataFlow, DataFlowEngine, FlowEdge, FlowError, FlowNode, FnNode, InMemoryEngine, NJson,
    NodeBody, NodeKind,
};
use serde_json::json;
use serde_json::Value;

fn linear_flow() -> DataFlow {
    DataFlow::new("f-linear", "two transform chain")
        .with_nodes(vec![
            FlowNode::new("src", NodeKind::Source).with_label("in"),
            FlowNode::new("upper", NodeKind::Transform).with_label("uppercase name"),
            FlowNode::new("lower", NodeKind::Transform).with_label("lowercase email"),
            FlowNode::new("sink", NodeKind::Sink).with_label("out"),
        ])
        .with_edges(vec![
            FlowEdge::new("src", "upper"),
            FlowEdge::new("upper", "lower"),
            FlowEdge::new("lower", "sink"),
        ])
}

#[tokio::test]
async fn execute_chain_in_declaration_order() {
    let flow = linear_flow();
    let mut mut_bodies: HashMap<String, Box<dyn NodeBody>> = HashMap::new();
    mut_bodies.insert(
        "upper".into(),
        Box::new(FnNode::new(|_id, v: Value| {
            let mut obj = v.as_object().cloned().unwrap_or_default();
            if let Some(n) = obj.get("name").and_then(|x| x.as_str()) {
                obj.insert("name".into(), json!(n.to_uppercase()));
            }
            Ok(serde_json::Value::Object(obj))
        })),
    );
    mut_bodies.insert(
        "lower".into(),
        Box::new(FnNode::new(|_id, v: Value| {
            let mut obj = v.as_object().cloned().unwrap_or_default();
            if let Some(e) = obj.get("email").and_then(|x| x.as_str()) {
                obj.insert("email".into(), json!(e.to_lowercase()));
            }
            Ok(serde_json::Value::Object(obj))
        })),
    );
    let engine = InMemoryEngine::new();
    let out = engine
        .execute(
            &flow,
            &mut_bodies,
            json!({"name": "alice", "email": "Foo@Example.COM"}),
        )
        .await
        .expect("ok");
    assert_eq!(out, json!({"name": "ALICE", "email": "foo@example.com"}));
}

#[tokio::test]
async fn execute_rejects_cyclic_graph() {
    let flow = DataFlow::new("f", "")
        .with_nodes(vec![
            FlowNode::new("a", NodeKind::Transform),
            FlowNode::new("b", NodeKind::Transform),
        ])
        .with_edges(vec![FlowEdge::new("a", "b"), FlowEdge::new("b", "a")]);
    let engine = InMemoryEngine::new();
    let bodies: HashMap<String, Box<dyn NodeBody>> = HashMap::new();
    let err = engine
        .execute(&flow, &bodies, json!({}))
        .await
        .expect_err("cycle");
    assert!(matches!(err, FlowError::CyclicGraph { .. }));
}

#[tokio::test]
async fn execute_rejects_unknown_node_edge() {
    let flow = DataFlow::new("f", "")
        .with_nodes(vec![FlowNode::new("a", NodeKind::Source)])
        .with_edges(vec![FlowEdge::new("a", "missing")]);
    let engine = InMemoryEngine::new();
    let bodies: HashMap<String, Box<dyn NodeBody>> = HashMap::new();
    let err = engine
        .execute(&flow, &bodies, json!({}))
        .await
        .expect_err("unknown");
    assert!(matches!(err, FlowError::UnknownNode(_)));
}

#[tokio::test]
async fn execute_rejects_transform_on_non_object_input() {
    let flow = DataFlow::new("f", "")
        .with_nodes(vec![
            FlowNode::new("src", NodeKind::Source),
            FlowNode::new("t", NodeKind::Transform),
            FlowNode::new("sink", NodeKind::Sink),
        ])
        .with_edges(vec![FlowEdge::new("src", "t"), FlowEdge::new("t", "sink")]);
    let engine = InMemoryEngine::new();
    let bodies: HashMap<String, Box<dyn NodeBody>> = HashMap::new();
    let err = engine
        .execute(&flow, &bodies, json!(7))
        .await
        .expect_err("type");
    assert!(matches!(err, FlowError::TypeMismatch { .. }));
}

#[tokio::test]
async fn njson_helpers_round_trip_through_engine() {
    // Build a flow that takes an NJson, marks it, and
    // returns it. The exercise proves NJson's helpers
    // are usable from the engine's `Value` API.
    let flow = DataFlow::new("f", "")
        .with_nodes(vec![
            FlowNode::new("src", NodeKind::Source),
            FlowNode::new("mark", NodeKind::Transform),
            FlowNode::new("sink", NodeKind::Sink),
        ])
        .with_edges(vec![
            FlowEdge::new("src", "mark"),
            FlowEdge::new("mark", "sink"),
        ]);
    let mut bodies: HashMap<String, Box<dyn NodeBody>> = HashMap::new();
    bodies.insert(
        "mark".into(),
        Box::new(FnNode::new(|_id, v: Value| {
            let mut obj = v.as_object().cloned().unwrap_or_default();
            obj.insert("marked".into(), NJson::bool(true).into_value());
            obj.insert("count".into(), NJson::uint(1).into_value());
            Ok(serde_json::Value::Object(obj))
        })),
    );
    let engine = InMemoryEngine::new();
    let out = engine
        .execute(
            &flow,
            &bodies,
            NJson::object().into_value(), // {}
        )
        .await
        .expect("ok");
    assert_eq!(out, json!({"marked": true, "count": 1}));
}
