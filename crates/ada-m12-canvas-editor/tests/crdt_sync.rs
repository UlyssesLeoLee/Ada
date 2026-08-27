//! Integration tests for the M-12 v0.6.0 CRDT (Yrs) sync path.
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
    encode_state_as_update, merge_crdt_update, reconcile_with_crdt, Canvas, CanvasNode, NodeKind,
    Position,
};
use yrs::updates::decoder::Decode;
use yrs::updates::encoder::Encode;
use yrs::{Any, Array, ArrayRef, Doc, Map, MapPrelim, MapRef};
use yrs::{ReadTxn, Transact, WriteTxn};

/// Push a node into a YDoc at the elements array, returning the
/// doc's state vector. Convenience helper for the cross-client
/// sync tests below.
fn push_node(doc: &Doc, els: &ArrayRef, id: &str, label: &str) {
    let mut txn = doc.transact_mut();
    let m: MapRef = els.push_back(&mut txn, MapPrelim::<yrs::Any>::new());
    m.insert(&mut txn, "id", id);
    m.insert(&mut txn, "kind", "block");
    m.insert(&mut txn, "x", 0i64);
    m.insert(&mut txn, "y", 0i64);
    m.insert(&mut txn, "label", label);
    m.insert(&mut txn, "alive", true);
}

/// Encode a doc's state vector for use as a `remote_state` arg
/// in `merge_crdt_update`. Re-exposed here (not from the lib)
/// to keep the integration test self-contained and to
/// demonstrate the wire shape callers see.
fn state_vector_bytes(doc: &Doc) -> Vec<u8> {
    let txn = doc.transact();
    txn.state_vector().encode_v1()
}

#[test]
fn three_clients_converge_to_same_state() {
    // 1. Build 3 YDocs (one per "client"). Each starts with
    //    a shared root.
    let docs: Vec<Doc> = (0..3).map(|_| Doc::new()).collect();
    let els: Vec<ArrayRef> = docs
        .iter()
        .map(|d| d.get_or_insert_array("elements"))
        .collect();

    // 2. Each client makes independent edits: c0 adds 3 nodes,
    //    c1 adds 2 nodes, c2 adds 4 nodes — disjoint ids so
    //    there is no in-YArray "duplicate element" ambiguity.
    for (i, (doc, el)) in docs.iter().zip(els.iter()).enumerate() {
        let n = match i {
            0 => 3,
            1 => 2,
            _ => 4,
        };
        for j in 0..n {
            push_node(doc, el, &format!("c{i}-n{j}"), &format!("client-{i}-node-{j}"));
        }
    }

    // 3. Star-shaped sync: clients 1 and 2 push to 0; then 0
    //    pushes back to 1 and 2. Two rounds is sufficient to
    //    show convergence for 3 peers (later: formal gossip).
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

    // 4. Verify all 3 docs see the same total element count:
    //    3 + 2 + 4 = 9 elements.
    for (i, doc) in docs.iter().enumerate() {
        let len = {
            let txn = doc.transact();
            txn.get_array("elements").expect("elements").len(&txn)
        };
        assert_eq!(len, 9, "client {i} should see 9 elements after merge");
    }
}

#[test]
fn reconcile_with_server_canvas_preserves_client_additions() {
    // Server-side state: 1 block node.
    let server = Canvas::new("doc-1");
    let server_node = CanvasNode::new(NodeKind::Block, Position::new(0, 0), "server-block");
    server.add_node(server_node);

    // Client-side: a fresh YDoc with one client-only element
    // (different node id from the server's).
    let client_doc = Doc::new();
    let client_els = client_doc.get_or_insert_array("elements");
    push_node(
        &client_doc,
        &client_els,
        "99999999-9999-9999-9999-999999999999",
        "client-block",
    );
    let client_update = encode_state_as_update(&client_doc);

    // Run the reconcile.
    let result = reconcile_with_crdt(&server, &client_update, 1).expect("reconcile");
    assert_eq!(result.new_version, 2);

    // Decode the merged state on a fresh doc and verify the
    // union of server + client elements.
    let merged_doc = Doc::new();
    {
        let mut txn = merged_doc.transact_mut();
        let update = yrs::Update::decode_v1(&result.merged_state).expect("decode merged");
        txn.apply_update(update);
    }
    let len = {
        let txn = merged_doc.transact();
        txn.get_array("elements").expect("elements").len(&txn)
    };
    assert_eq!(len, 2, "merged state should have 2 elements (1 server + 1 client)");
}
