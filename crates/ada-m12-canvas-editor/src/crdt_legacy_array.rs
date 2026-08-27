//! v0.6.0 CRDT legacy-array schema (YArray of YMap).
//!
//! This module preserves the v0.6.0 YArray-of-YMap schema
//! behind the `legacy-array` Cargo feature (default off) so
//! callers can opt into the v0.6.0 wire format during the
//! v0.6.0 → v0.7.0 transition window. v0.8.0 will remove
//! this module.
//!
//! The v0.7.0 default in [`crate::crdt`] is YMap-keyed-by-uuid,
//! which gives:
//! - true concurrent-delete convergence (2P-Set on YMap keys)
//! - natural dedup on concurrent insert of same id
//! - ports as a proper nested YArray (field-level CRDT
//!   coverage for ports)
//! - edge dedup via YMap keyed by `from::to`
//!
//! The v0.6.0 YArray-of-YMap schema has known limitations
//! (see `CRDT.md` §7) that v0.7.0 fixes.
//!
//! ## Feature gating
//!
//! This module is only compiled with `--features legacy-array`.
//! It is the exact v0.6.0 implementation, lifted into its own
//! file so the v0.7.0 schema in [`crate::crdt`] can evolve
//! independently. Build with `--features legacy-array` to
//! keep v0.6.0 behavior available.

#![cfg(feature = "legacy-array")]

use yrs::types::Value;
use yrs::updates::decoder::Decode;
use yrs::updates::encoder::Encode;
use yrs::{Any, Array, ArrayRef, Doc, Map, MapPrelim, MapRef, ReadTxn, StateVector, Transact};

use crate::canvas::{Canvas, Edge};
use crate::error::CanvasError;
use crate::node::{CanvasNode, NodeKind, Position};

const META_KEY: &str = "meta";
const ELEMENTS_KEY: &str = "elements";
const EDGES_KEY: &str = "edges";
const F_ALIVE: &str = "alive";
const F_ID: &str = "id";
const F_KIND: &str = "kind";
const F_X: &str = "x";
const F_Y: &str = "y";
const F_LABEL: &str = "label";
const F_PORTS: &str = "ports";
const F_FROM: &str = "from";
const F_TO: &str = "to";
const F_NAME: &str = "name";
const F_VERSION: &str = "version";

/// v0.6.0 reconcile (YArray-of-YMap schema). See
/// [`crate::crdt`] for the v0.7.0 default. This function
/// preserves the v0.6.0 algorithm and wire format verbatim.
#[deprecated(
    since = "0.7.0",
    note = "v0.6.0 YArray-of-YMap schema; use `crdt::reconcile_with_crdt` (v0.7.0 YMap-keyed-by-uuid) instead. Will be removed in v0.8.0."
)]
pub fn reconcile_with_crdt_legacy(
    server: &Canvas,
    client_update: &[u8],
    client_version: u64,
) -> Result<crate::crdt::CrdtReconcileResult, CanvasError> {
    let doc = Doc::new();
    hydrate_doc_from_canvas(&doc, server);
    {
        let mut txn = doc.transact_mut();
        let update = yrs::Update::decode_v1(client_update)
            .map_err(|e| CanvasError::BackendError(format!("yrs decode_v1 failed: {e}")))?;
        txn.apply_update(update);
    }
    let merged_state = crate::crdt::encode_state_as_update(&doc);
    let new_version = server.version().max(client_version).saturating_add(1);
    Ok(crate::crdt::CrdtReconcileResult {
        merged_state,
        new_version,
    })
}

fn hydrate_doc_from_canvas(doc: &Doc, canvas: &Canvas) {
    let meta: MapRef = doc.get_or_insert_map(META_KEY);
    let elements: ArrayRef = doc.get_or_insert_array(ELEMENTS_KEY);
    let edges: ArrayRef = doc.get_or_insert_array(EDGES_KEY);
    let mut txn = doc.transact_mut();
    meta.insert(&mut txn, F_NAME, canvas.name());
    meta.insert(
        &mut txn,
        F_VERSION,
        i64::try_from(canvas.version()).unwrap_or(i64::MAX),
    );
    for node in canvas.nodes() {
        let m: MapRef = elements.push_back(&mut txn, MapPrelim::<Any>::new());
        write_node_fields(&m, &mut txn, &node);
    }
    for edge in canvas.edges() {
        let m: MapRef = edges.push_back(&mut txn, MapPrelim::<Any>::new());
        write_edge_fields(&m, &mut txn, &edge);
    }
}

fn write_node_fields(map: &MapRef, txn: &mut yrs::TransactionMut, node: &CanvasNode) {
    map.insert(txn, F_ID, node.id.0.to_string());
    map.insert(txn, F_KIND, kind_str(node.kind));
    map.insert(txn, F_X, i64::from(node.position.x));
    map.insert(txn, F_Y, i64::from(node.position.y));
    map.insert(txn, F_LABEL, node.label.clone());
    map.insert(txn, F_ALIVE, true);
    let ports_json = serde_json::to_string(
        &node
            .ports
            .iter()
            .map(|p| p.name.as_str())
            .collect::<Vec<_>>(),
    )
    .unwrap_or_else(|_| "[]".to_string());
    map.insert(txn, F_PORTS, ports_json);
}

fn write_edge_fields(map: &MapRef, txn: &mut yrs::TransactionMut, edge: &Edge) {
    map.insert(txn, F_FROM, edge.from.0.to_string());
    map.insert(txn, F_TO, edge.to.0.to_string());
    map.insert(txn, F_ALIVE, true);
}

fn kind_str(kind: NodeKind) -> &'static str {
    match kind {
        NodeKind::Block => "block",
        NodeKind::Connector => "connector",
        NodeKind::Note => "note",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crdt::{encode_state_as_update, merge_crdt_update};

    /// v0.6.0 smoke: legacy YArray path still works under
    /// the legacy-array feature gate.
    #[test]
    #[allow(deprecated)]
    fn legacy_array_concurrent_inserts_converge() {
        let server = Canvas::new("c1");
        server.add_node(CanvasNode::new(NodeKind::Block, Position::new(0, 0), "a"));

        let client_doc = Doc::new();
        let els = client_doc.get_or_insert_array(ELEMENTS_KEY);
        {
            let mut txn = client_doc.transact_mut();
            let m: MapRef = els.push_back(&mut txn, MapPrelim::<Any>::new());
            m.insert(&mut txn, F_ID, "11111111-1111-1111-1111-111111111111");
            m.insert(&mut txn, F_KIND, "block");
            m.insert(&mut txn, F_LABEL, "from-client");
            m.insert(&mut txn, F_ALIVE, true);
        }
        let update = encode_state_as_update(&client_doc);
        let result = reconcile_with_crdt_legacy(&server, &update, 1).expect("reconcile");
        assert_eq!(result.new_version, 2);

        let peer = Doc::new();
        {
            let mut txn = peer.transact_mut();
            let u = yrs::Update::decode_v1(&result.merged_state).expect("dec");
            txn.apply_update(u);
        }
        let len = {
            let txn = peer.transact();
            let els = txn.get_array(ELEMENTS_KEY).expect("els");
            els.len(&txn)
        };
        assert_eq!(len, 2);

        // Touch `merge_crdt_update` to satisfy the unused
        // import lint when running with `cargo test
        // --features legacy-array`.
        let _ = merge_crdt_update(&peer, &[], &[]);
    }
}
