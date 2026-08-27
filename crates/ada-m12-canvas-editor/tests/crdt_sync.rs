//! Integration tests for the M-12 v0.7.0 CRDT (Yrs) sync path.
//!
//! Cross-crate smoke test: 3 concurrent clients edit a shared
//! canvas independently, then sync via `merge_crdt_update` /
//! `encode_state_as_update` and verify all replicas converge to
//! the same final state.
//!
//! This file is `[[test]]`-registered in `Cargo.toml` per the
//! B2 lesson (Cargo 1.85+ silently drops integration tests
//! discovered only via `tests/*.rs` when `[lib]` has an
//! explicit `path = ...`).
//!
//! Gated by `--features crdt` because the `crdt` module is
//! feature-gated in the main library.

#![cfg(feature = "crdt")]

use ada_m12_canvas_editor::{
    encode_state_as_update, insert_element, iter_elements, merge_crdt_update, reconcile_with_crdt,
    Canvas, CanvasNode, ClientId, NodeKind, Position,
};
use yrs::types::Value;
use yrs::updates::decoder::Decode;
use yrs::updates::encoder::Encode;
use yrs::Map;
use yrs::{Doc, ReadTxn, Transact};

/// Encode a doc's state vector for use as a `remote_state` arg
/// in `merge_crdt_update`. Re-exposed here (not from the lib)
/// to keep the integration test self-contained and to
/// demonstrate the wire shape callers see.
fn state_vector_bytes(doc: &Doc) -> Vec<u8> {
    let txn = doc.transact();
    txn.state_vector().encode_v1()
}

/// v0.7.0: 3 concurrent clients insert elements (different
/// uuids per client) and then sync. After sync, all 3 docs
/// should expose the same 9 elements. Uses the v0.7.0
/// YMap-keyed-by-uuid schema (`insert_element` /
/// `iter_elements`).
#[test]
fn three_clients_converge_to_same_state() {
    // 1. Build 3 YDocs (one per "client"). Each starts with
    //    the v0.7.0 root layout (elements YMap, etc.).
    let docs: Vec<Doc> = (0..3).map(|_| Doc::new()).collect();

    // 2. Each client makes independent inserts: c0 adds 3,
    //    c1 adds 2, c2 adds 4 — disjoint uuids so there is
    //    no in-YMap "same-key conflict" (each insert
    //    creates a unique key).
    for (i, doc) in docs.iter().enumerate() {
        let n = match i {
            0 => 3,
            1 => 2,
            _ => 4,
        };
        for j in 0..n {
            let mut node = CanvasNode::new(
                NodeKind::Block,
                Position::new(0, 0),
                &format!("client-{i}-node-{j}"),
            );
            node.id = ada_m12_canvas_editor::NodeId(uuid::Uuid::new_v4());
            insert_element(doc, &node).expect("insert");
        }
    }

    // 3. Star-shaped sync: clients 1 and 2 push to 0; then
    //    0 pushes back to 1 and 2. Two rounds is sufficient
    //    to show convergence for 3 peers (later: formal
    //    gossip).
    for src in 1..docs.len() {
        let update = encode_state_as_update(&docs[src]);
        let sv = state_vector_bytes(&docs[0]);
        let diff = merge_crdt_update(&docs[0], &sv, &update).expect("merge src -> 0");
        {
            let mut txn = docs[0].transact_mut();
            let upd = yrs::Update::decode_v1(&diff).expect("decode");
            txn.apply_update(upd);
        }
    }
    for dst in 1..docs.len() {
        let update = encode_state_as_update(&docs[0]);
        let sv = state_vector_bytes(&docs[dst]);
        let diff = merge_crdt_update(&docs[dst], &sv, &update).expect("merge 0 -> dst");
        {
            let mut txn = docs[dst].transact_mut();
            let upd = yrs::Update::decode_v1(&diff).expect("decode");
            txn.apply_update(upd);
        }
    }

    // 4. Verify all 3 docs see the same total element
    //    count: 3 + 2 + 4 = 9 elements.
    for (i, doc) in docs.iter().enumerate() {
        let count = iter_elements(doc).count();
        assert_eq!(count, 9, "client {i} should see 9 elements after merge");
    }
}

/// v0.7.0: server has 1 element (in v0.5.0 `Canvas` shape),
/// client has 1 element (in v0.7.0 YDoc shape). After
/// `reconcile_with_crdt` with a fresh `ClientId`, the
/// merged state encodes both elements under the v0.7.0
/// YMap-keyed-by-uuid schema.
#[test]
fn reconcile_with_server_canvas_preserves_client_additions() {
    // Server-side state: 1 block node.
    let server = Canvas::new("doc-1");
    let mut server_node = CanvasNode::new(NodeKind::Block, Position::new(0, 0), "server-block");
    server_node.id = ada_m12_canvas_editor::NodeId(uuid::Uuid::new_v4());
    server.add_node(server_node);

    // Client-side: a fresh YDoc with one client-only element
    // (different node id from the server's).
    let client_doc = Doc::new();
    let mut client_node = CanvasNode::new(NodeKind::Block, Position::new(0, 0), "client-block");
    client_node.id = ada_m12_canvas_editor::NodeId(uuid::Uuid::new_v4());
    insert_element(&client_doc, &client_node).expect("insert client");
    let client_update = encode_state_as_update(&client_doc);

    // Run the reconcile with a fresh server-side ClientId.
    let server_client_id = ClientId::new("server-1");
    let result =
        reconcile_with_crdt(&server, &client_update, 1, &server_client_id).expect("reconcile");
    assert_eq!(result.new_version, 2);

    // Decode the merged state on a fresh doc and verify
    // the union of server + client elements.
    let merged_doc = Doc::new();
    {
        let mut txn = merged_doc.transact_mut();
        let update = yrs::Update::decode_v1(&result.merged_state).expect("decode merged");
        txn.apply_update(update);
    }
    let count = {
        let txn = merged_doc.transact();
        let elements = txn.get_map("elements").expect("elements");
        let mut live = 0usize;
        for (_k, v) in elements.iter(&txn) {
            if let Value::YMap(m) = v {
                let alive = m
                    .get(&txn, "alive")
                    .map(|x| matches!(x, Value::Any(yrs::any::Any::Bool(true))))
                    .unwrap_or(true);
                if alive {
                    live += 1;
                }
            }
        }
        live
    };
    assert_eq!(
        count, 2,
        "merged state should have 2 live elements (1 server + 1 client)"
    );
}
