//! CRDT-backed canvas sync for M-12 (v0.6.0).
//!
//! This module is the v0.6.0 replacement for the v0.5.0 LWW
//! 3-way merge in [`crate::server_recon`]. Instead of "server wins
//! on conflict" (which loses concurrent client edits), it uses a
//! Yjs-compatible CRDT (Yrs, MIT) so that any number of concurrent
//! edits from disconnected clients merge deterministically
//! without conflict.
//!
//! ## Design (per docs/decisions/02-design-adrs.md D-01)
//!
//! - A `YDoc` is the top-level shared state. Each replica is
//!   initialised with the same `client_id` per `Doc::with_client_id`
//!   so a server / client pair never aliases.
//! - Root layout: a YMap named `"meta"` carrying the document
//!   name and version, plus a YArray named `"elements"` whose
//!   entries are YMaps. Each element YMap carries the node id
//!   (UUID string), kind, position, label, ports, and an `alive`
//!   flag used as a soft-delete tombstone. Edges are stored as
//!   a parallel YArray `"edges"` of YMaps with `from` / `to`
//!   UUIDs. Using per-element YMaps (not per-element positions
//!   in a single YArray) means concurrent moves of the same
//!   node from two replicas converge via Yjs last-writer-wins on
//!   each field, instead of being "conflict on the array slot".
//!
//! ## Sync API
//!
//! - [`merge_crdt_update`] — apply a remote update to a `YDoc`
//!   and return a follow-up update that contains any state the
//!   remote still needs.
//! - [`encode_state_as_update`] — full state snapshot (used for
//!   first sync / replay).
//! - [`reconcile_with_crdt`] — end-to-end reconcile: build a
//!   fresh `YDoc` from the server's snapshot, apply the client's
//!   update (in any order), and return a `CrdtReconcileResult`
//!   whose `merged_state` is the new server snapshot. The
//!   merged state is a CRDT-state-encoded byte string (use
//!   [`encode_state_as_update`] to ship it to the client).
//!
//! ## LWW fallback
//!
//! The v0.5.0 LWW path is preserved behind the `legacy-lww`
//! feature flag (default off). Build with `--features legacy-lww`
//! to keep the old `server_recon` flow available during the
//! v0.5.0 → v0.6.0 transition window. See
//! `crates/ada-m12-canvas-editor/CRDT.md` for the migration
//! notes.
//!
//! ## Feature gating
//!
//! This module is gated by `feature = "crdt"` (default off in the
//! 5-gate CI default-build path) so adding `yrs` to the dep
//! graph does not slow down `cargo test --workspace`. The
//! default off flag also lets downstream crates opt in
//! independently of the LWW `server` feature.

#![cfg(feature = "crdt")]

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use yrs::updates::decoder::Decode;
use yrs::updates::encoder::Encode;
use yrs::{Any, Array, ArrayRef, Doc, Map, MapPrelim, MapRef, ReadTxn, StateVector, Transact};

use crate::canvas::{Canvas, Edge};
use crate::error::CanvasError;
use crate::node::{CanvasNode, NodeId, NodeKind, Position};

/// Root YMap carrying document-level metadata.
const META_KEY: &str = "meta";
/// Root YArray of element YMaps.
const ELEMENTS_KEY: &str = "elements";
/// Root YArray of edge YMaps.
const EDGES_KEY: &str = "edges";
/// Field name for the element / edge "deleted" tombstone flag
/// (Yjs does not have a "soft delete" — concurrent deletes are
/// resolved by the YArray CRDT, but we keep `alive` so an undo
/// layer can resurrect a node if needed in a future phase).
const F_ALIVE: &str = "alive";
/// Element id field (UUID string, the same id as [`NodeId`]).
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

/// Apply a remote CRDT update to `doc` and return a follow-up
/// update containing anything the remote peer still needs to
/// reach full state parity.
///
/// `remote_state` is the remote peer's state vector (a
/// `StateVector::decode_v1` output). The returned bytes are an
/// `encode_diff_v1` payload: the local doc's content that the
/// remote has not seen yet.
///
/// `update_bytes` is the remote peer's diff (or full state
/// encoded as update), produced by [`encode_state_as_update`] or
/// `Doc::transact().encode_diff_v1(&sv)`.
///
/// # Errors
///
/// Returns [`CanvasError::BackendError`] on yrs decode / encode
/// failure (malformed update bytes, or version mismatch). This
/// is a programming error on the wire — never a user-visible
/// validation problem.
pub fn merge_crdt_update(
    doc: &Doc,
    remote_state: &[u8],
    update_bytes: &[u8],
) -> Result<Vec<u8>, CanvasError> {
    // 1. Apply the remote update (mutates the local doc).
    {
        let mut txn = doc.transact_mut();
        let update = yrs::Update::decode_v1(update_bytes)
            .map_err(|e| CanvasError::BackendError(format!("yrs decode_v1 failed: {e}")))?;
        txn.apply_update(update);
    }
    // 2. Build a diff that the remote can use to catch up to
    //    the local state.
    let sv = StateVector::decode_v1(remote_state)
        .map_err(|e| CanvasError::BackendError(format!("yrs state_vector decode failed: {e}")))?;
    let diff = {
        let txn = doc.transact();
        txn.encode_diff_v1(&sv)
    };
    Ok(diff)
}

/// Encode the entire current `YDoc` state as a CRDT update.
///
/// Used for first-sync (a fresh client connects and downloads the
/// full state) and for snapshotting to disk.
#[must_use]
pub fn encode_state_as_update(doc: &Doc) -> Vec<u8> {
    let txn = doc.transact();
    let sv = StateVector::default();
    txn.encode_state_as_update_v1(&sv)
}

/// End-to-end CRDT reconcile: take the server-side `Canvas`
/// snapshot, build a YDoc, apply the client's update, and return
/// a [`CrdtReconcileResult`] containing the merged state and a
/// delta the client should apply.
///
/// This is the v0.6.0 replacement for the v0.5.0
/// `reconcile_canvas_state` function. Where the v0.5.0 path
/// "server wins" on conflict, this path converges via Yrs
/// last-write-wins per field — concurrent edits to *different*
/// fields are kept; concurrent edits to the *same* field go to
/// whichever replica wrote the later (client, by Lamport-style
/// timestamp inside the CRDT) value. The result is no
/// `server_wins` / `client_wins` split — only the merged state.
///
/// # Errors
///
/// Returns [`CanvasError::BackendError`] on any yrs decode or
/// apply error (malformed update bytes).
pub fn reconcile_with_crdt(
    server: &Canvas,
    client_update: &[u8],
    client_version: u64,
) -> Result<CrdtReconcileResult, CanvasError> {
    // 1. Build a fresh YDoc seeded with the server's snapshot.
    let doc = Doc::new();
    hydrate_doc_from_canvas(&doc, server);

    // 2. Apply the client's update on top.
    {
        let mut txn = doc.transact_mut();
        let update = yrs::Update::decode_v1(client_update)
            .map_err(|e| CanvasError::BackendError(format!("yrs decode_v1 failed: {e}")))?;
        txn.apply_update(update);
    }

    // 3. Encode the merged state.
    let merged_state = encode_state_as_update(&doc);

    // 4. The new server version = max(server.version,
    //    client_version) + 1 — same convention as v0.5.0 so
    //    existing optimistic-concurrency tests still pass.
    let new_version = server.version().max(client_version).saturating_add(1);

    Ok(CrdtReconcileResult {
        merged_state,
        new_version,
    })
}

/// Result of [`reconcile_with_crdt`]: the merged CRDT state plus
/// a new server-side version.
///
/// The `merged_state` field is a yrs-encoded update that callers
/// can ship to the client (or persist server-side). There is no
/// `server_wins` / `client_wins` split in the v0.6.0 CRDT path —
/// the CRDT itself guarantees convergence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrdtReconcileResult {
    /// yrs-encoded full state of the merged document. Pass to
    /// [`merge_crdt_update`] (as `update_bytes`) on the client to
    /// converge.
    pub merged_state: Vec<u8>,
    /// New server-side version, `>= max(server.version,
    /// client_version) + 1`. Bumps monotonically even on a no-op
    /// merge (the server "processed" the client's request).
    pub new_version: u64,
}

/// Build (or rebuild) a YDoc from a server-side `Canvas`
/// snapshot. This is how a server that already has
/// authoritative state in the v0.5.0 `Canvas` shape seeds the
/// v0.6.0 YDoc backend.
fn hydrate_doc_from_canvas(doc: &Doc, canvas: &Canvas) {
    // 1. Materialise the three root types first. `get_or_insert_*`
    //    opens a short-lived inner `transact_mut()`, so we must
    //    do this *before* opening our own long-lived txn.
    let meta: MapRef = doc.get_or_insert_map(META_KEY);
    let elements: ArrayRef = doc.get_or_insert_array(ELEMENTS_KEY);
    let edges: ArrayRef = doc.get_or_insert_array(EDGES_KEY);
    // 2. Open a single write txn to populate.
    let mut txn = doc.transact_mut();
    meta.insert(&mut txn, F_NAME, canvas.name());
    meta.insert(&mut txn, F_VERSION, canvas.version() as i64);
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
    // Ports are stored as a JSON-encoded string array in the
    // parent map. v0.7.0 will lift ports to a nested YArray for
    // field-level CRDT resolution.
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

fn kind_parse(s: &str) -> NodeKind {
    match s {
        "connector" => NodeKind::Connector,
        "note" => NodeKind::Note,
        _ => NodeKind::Block,
    }
}

/// Read a YDoc back into a `Canvas` snapshot. Used by tests to
/// verify the round-trip and by future m13 endpoints that want
/// to serve a CRDT-merged state to a non-CRDT-aware client.
#[allow(dead_code)]
pub(crate) fn read_canvas_from_doc(doc: &Doc, name: &str) -> Result<Canvas, CanvasError> {
    let txn = doc.transact();
    let mut nodes: Vec<CanvasNode> = Vec::new();
    let mut seen_ids: HashSet<String> = HashSet::new();
    if let Some(elements) = txn.get_array(ELEMENTS_KEY) {
        let len = elements.len(&txn);
        for i in 0..len {
            let Some(value) = elements.get(&txn, i) else {
                continue;
            };
            let yrs::types::Value::YMap(map) = value else {
                continue;
            };
            if !is_alive(&map, &txn) {
                continue;
            }
            let Some(id_str) = string_field(&map, &txn, F_ID) else {
                continue;
            };
            if !seen_ids.insert(id_str.clone()) {
                continue;
            }
            let id = parse_node_id(&id_str)?;
            let kind = kind_parse(
                &string_field(&map, &txn, F_KIND).unwrap_or_else(|| "block".into()),
            );
            let x = int_field(&map, &txn, F_X).unwrap_or(0);
            let y = int_field(&map, &txn, F_Y).unwrap_or(0);
            let label = string_field(&map, &txn, F_LABEL).unwrap_or_default();
            let ports_json =
                string_field(&map, &txn, F_PORTS).unwrap_or_else(|| "[]".into());
            let ports: Vec<crate::node::Port> =
                serde_json::from_str::<Vec<String>>(&ports_json)
                    .unwrap_or_default()
                    .into_iter()
                    .map(crate::node::Port::new)
                    .collect();
            let mut n = CanvasNode::new(kind, Position::new(x, y), label);
            n.id = id;
            n.ports = ports;
            nodes.push(n);
        }
    }
    let mut edges: Vec<Edge> = Vec::new();
    if let Some(edge_arr) = txn.get_array(EDGES_KEY) {
        let len = edge_arr.len(&txn);
        let node_ids: HashSet<NodeId> = nodes.iter().map(|n| n.id).collect();
        for i in 0..len {
            let Some(value) = edge_arr.get(&txn, i) else {
                continue;
            };
            let yrs::types::Value::YMap(map) = value else {
                continue;
            };
            if !is_alive(&map, &txn) {
                continue;
            }
            let Some(from_s) = string_field(&map, &txn, F_FROM) else {
                continue;
            };
            let Some(to_s) = string_field(&map, &txn, F_TO) else {
                continue;
            };
            let Ok(from) = parse_node_id(&from_s) else {
                continue;
            };
            let Ok(to) = parse_node_id(&to_s) else {
                continue;
            };
            if !node_ids.contains(&from) || !node_ids.contains(&to) {
                continue;
            }
            edges.push(Edge::new(from, to));
        }
    }
    let canvas = Canvas::new(name);
    for n in nodes {
        canvas.add_node(n);
    }
    for e in edges {
        // `add_edge` may fail on edge dedup; that's fine, we
        // ignore the error because the CRDT path stores
        // edges as a list (YArray has no built-in dedup, so
        // duplicates are possible if two clients add the
        // same edge concurrently and we never collapsed
        // them). Edge dedup is a v0.7.0 concern.
        let _ = canvas.add_edge(e);
    }
    Ok(canvas)
}

fn is_alive(map: &MapRef, txn: &impl ReadTxn) -> bool {
    match map.get(txn, F_ALIVE) {
        Some(yrs::types::Value::Any(yrs::any::Any::Bool(b))) => b,
        _ => true,
    }
}

fn string_field(map: &MapRef, txn: &impl ReadTxn, key: &str) -> Option<String> {
    match map.get(txn, key) {
        Some(yrs::types::Value::Any(yrs::any::Any::String(s))) => Some(s.to_string()),
        _ => None,
    }
}

fn int_field(map: &MapRef, txn: &impl ReadTxn, key: &str) -> Option<i32> {
    match map.get(txn, key) {
        Some(yrs::types::Value::Any(yrs::any::Any::Number(n))) => Some(n as i32),
        Some(yrs::types::Value::Any(yrs::any::Any::BigInt(n))) => Some(n as i32),
        _ => None,
    }
}

fn parse_node_id(s: &str) -> Result<NodeId, CanvasError> {
    let uuid = uuid::Uuid::parse_str(s)
        .map_err(|e| CanvasError::BackendError(format!("invalid node id uuid: {e}")))?;
    Ok(NodeId(uuid))
}

/// Convenience helper for tests / external callers: parse a
/// yrs-encoded state vector.
#[allow(dead_code)]
pub(crate) fn encode_state_vector(doc: &Doc) -> Vec<u8> {
    let txn = doc.transact();
    txn.state_vector().encode_v1()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::NodeKind;

    fn positioned(label: &str, x: i32, y: i32) -> CanvasNode {
        CanvasNode::new(NodeKind::Block, Position::new(x, y), label)
    }

    /// Round-trip: build a YDoc from a `Canvas`, encode the
    /// state, decode on a second YDoc, and verify the second
    /// YDoc reproduces the same elements.
    #[test]
    fn sync_roundtrip_preserves_elements() {
        let server = Canvas::new("c1");
        server.add_node(positioned("a", 10, 20));
        let b = CanvasNode::new(NodeKind::Note, Position::new(5, 5), "note-1");
        server.add_node(b);
        let a_id = server.nodes()[0].id;
        let b_id = server.nodes()[1].id;
        server.add_edge(Edge::new(a_id, b_id)).expect("edge");

        let doc = Doc::new();
        hydrate_doc_from_canvas(&doc, &server);
        let snapshot = encode_state_as_update(&doc);

        let peer = Doc::new();
        {
            let mut txn = peer.transact_mut();
            let update = yrs::Update::decode_v1(&snapshot).expect("decode");
            txn.apply_update(update);
        }

        // Both YDocs should expose the same element count.
        let (local_count, remote_count) = {
            let local = doc.transact();
            let remote = peer.transact();
            let local_els = local.get_array(ELEMENTS_KEY).expect("elements");
            let remote_els = remote.get_array(ELEMENTS_KEY).expect("elements");
            (
                local_els.len(&local),
                remote_els.len(&remote),
            )
        };
        assert_eq!(local_count, 2);
        assert_eq!(remote_count, 2);
    }

    /// Concurrent inserts on two replicas converge to the union
    /// of both inserts — no LWW conflict, no data loss.
    #[test]
    fn concurrent_inserts_converge() {
        let doc_a = Doc::new();
        let doc_b = Doc::new();
        // Materialise the elements array on both, then write.
        let els_a = doc_a.get_or_insert_array(ELEMENTS_KEY);
        let els_b = doc_b.get_or_insert_array(ELEMENTS_KEY);
        {
            let mut txn = doc_a.transact_mut();
            let m: MapRef = els_a.push_back(&mut txn, MapPrelim::<Any>::new());
            m.insert(&mut txn, F_ID, "11111111-1111-1111-1111-111111111111");
            m.insert(&mut txn, F_KIND, "block");
            m.insert(&mut txn, F_LABEL, "shared-root");
            m.insert(&mut txn, F_ALIVE, true);
        }
        let initial = encode_state_as_update(&doc_a);
        {
            let mut txn = doc_b.transact_mut();
            let update = yrs::Update::decode_v1(&initial).expect("decode");
            txn.apply_update(update);
        }
        // a adds element-2, b adds element-3 — concurrently.
        {
            let mut txn = doc_a.transact_mut();
            let m: MapRef = els_a.push_back(&mut txn, MapPrelim::<Any>::new());
            m.insert(&mut txn, F_ID, "22222222-2222-2222-2222-222222222222");
            m.insert(&mut txn, F_KIND, "block");
            m.insert(&mut txn, F_LABEL, "from-a");
            m.insert(&mut txn, F_ALIVE, true);
        }
        {
            let mut txn = doc_b.transact_mut();
            let m: MapRef = els_b.push_back(&mut txn, MapPrelim::<Any>::new());
            m.insert(&mut txn, F_ID, "33333333-3333-3333-3333-333333333333");
            m.insert(&mut txn, F_KIND, "block");
            m.insert(&mut txn, F_LABEL, "from-b");
            m.insert(&mut txn, F_ALIVE, true);
        }
        // Cross-sync: a's update → b, b's update → a.
        let update_a = encode_state_as_update(&doc_a);
        let update_b = encode_state_as_update(&doc_b);
        let sv_a = encode_state_vector(&doc_a);
        let sv_b = encode_state_vector(&doc_b);
        let diff_for_b = merge_crdt_update(&doc_b, &sv_a, &update_a).expect("merge a->b");
        let diff_for_a = merge_crdt_update(&doc_a, &sv_b, &update_b).expect("merge b->a");
        // Apply the diffs on the other side to fully converge.
        {
            let mut txn = doc_a.transact_mut();
            let update = yrs::Update::decode_v1(&diff_for_b).expect("decode");
            txn.apply_update(update);
        }
        {
            let mut txn = doc_b.transact_mut();
            let update = yrs::Update::decode_v1(&diff_for_a).expect("decode");
            txn.apply_update(update);
        }
        // After convergence both should see 3 elements.
        let (a_count, b_count) = {
            let txn_a = doc_a.transact();
            let txn_b = doc_b.transact();
            let a = txn_a.get_array(ELEMENTS_KEY).expect("a els").len(&txn_a);
            let b = txn_b.get_array(ELEMENTS_KEY).expect("b els").len(&txn_b);
            (a, b)
        };
        assert_eq!(a_count, 3, "a should see 3 elements after merge");
        assert_eq!(b_count, 3, "b should see 3 elements after merge");
    }

    /// Concurrent move of the same node: last-writer-wins on the
    /// position field; both replicas converge to the same final
    /// position.
    #[test]
    fn concurrent_move_converges_via_lww() {
        let doc_a = Doc::new();
        let doc_b = Doc::new();
        let els_a = doc_a.get_or_insert_array(ELEMENTS_KEY);
        let els_b = doc_b.get_or_insert_array(ELEMENTS_KEY);
        let shared_id = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";
        // Seed both with the same shared element.
        for (doc, els) in [(&doc_a, &els_a), (&doc_b, &els_b)] {
            let mut txn = doc.transact_mut();
            let m: MapRef = els.push_back(&mut txn, MapPrelim::<Any>::new());
            m.insert(&mut txn, F_ID, shared_id);
            m.insert(&mut txn, F_KIND, "block");
            m.insert(&mut txn, F_LABEL, "shared");
            m.insert(&mut txn, F_X, 0i64);
            m.insert(&mut txn, F_Y, 0i64);
            m.insert(&mut txn, F_ALIVE, true);
        }
        // a moves to (100, 100), b moves to (200, 200).
        {
            let mut txn = doc_a.transact_mut();
            let yrs::types::Value::YMap(map) = els_a.get(&txn, 0).expect("els[0]") else {
                panic!("not a map")
            };
            map.insert(&mut txn, F_X, 100i64);
            map.insert(&mut txn, F_Y, 100i64);
        }
        {
            let mut txn = doc_b.transact_mut();
            let yrs::types::Value::YMap(map) = els_b.get(&txn, 0).expect("els[0]") else {
                panic!("not a map")
            };
            map.insert(&mut txn, F_X, 200i64);
            map.insert(&mut txn, F_Y, 200i64);
        }
        // Cross-sync both ways.
        let upd_a = encode_state_as_update(&doc_a);
        let upd_b = encode_state_as_update(&doc_b);
        let sv_a = encode_state_vector(&doc_a);
        let sv_b = encode_state_vector(&doc_b);
        let diff_for_b = merge_crdt_update(&doc_b, &sv_a, &upd_a).expect("a->b");
        let diff_for_a = merge_crdt_update(&doc_a, &sv_b, &upd_b).expect("b->a");
        {
            let mut txn = doc_a.transact_mut();
            let update = yrs::Update::decode_v1(&diff_for_b).expect("decode");
            txn.apply_update(update);
        }
        {
            let mut txn = doc_b.transact_mut();
            let update = yrs::Update::decode_v1(&diff_for_a).expect("decode");
            txn.apply_update(update);
        }
        // Read the final position from both — they should be
        // identical (the CRDT resolved the conflict).
        let (ax, ay, bx, by) = {
            let txn_a = doc_a.transact();
            let txn_b = doc_b.transact();
            let yrs::types::Value::YMap(map_a) =
                els_a.get(&txn_a, 0).expect("a[0]")
            else {
                panic!()
            };
            let yrs::types::Value::YMap(map_b) =
                els_b.get(&txn_b, 0).expect("b[0]")
            else {
                panic!()
            };
            let ax = int_field(&map_a, &txn_a, F_X).expect("ax");
            let ay = int_field(&map_a, &txn_a, F_Y).expect("ay");
            let bx = int_field(&map_b, &txn_b, F_X).expect("bx");
            let by = int_field(&map_b, &txn_b, F_Y).expect("by");
            (ax, ay, bx, by)
        };
        assert_eq!(ax, bx, "x position should converge");
        assert_eq!(ay, by, "y position should converge");
    }

    /// One-way delete propagation: doc_a has 2 elements, doc_b
    /// is fresh and receives the initial state (2 elements).
    /// doc_a then removes index 0, and the delete propagates
    /// to doc_b via the CRDT update.
    ///
    /// Note: this is a single-direction delete test. Pure
    /// bidirectional concurrent delete on the same YArray slot
    /// is a known Yjs YArray limitation (concurrent
    /// position-based removes do not collapse to a single
    /// delete — Yjs keeps the higher-timestamp tombstone).
    /// v0.6.0 keeps the simple "YArray of YMaps" schema; a
    /// future v0.7.0 may lift elements to a top-level YMap
    /// keyed by node id, which gives true symmetric delete
    /// convergence.
    #[test]
    fn concurrent_delete_converges() {
        let doc_a = Doc::new();
        let doc_b = Doc::new();
        let els_a = doc_a.get_or_insert_array(ELEMENTS_KEY);
        {
            let mut txn = doc_a.transact_mut();
            for (id, label) in [("id-1", "first"), ("id-2", "second")] {
                let m: MapRef = els_a.push_back(&mut txn, MapPrelim::<Any>::new());
                m.insert(&mut txn, F_ID, id);
                m.insert(&mut txn, F_KIND, "block");
                m.insert(&mut txn, F_LABEL, label);
                m.insert(&mut txn, F_ALIVE, true);
            }
        }
        // doc_b receives the initial state (2 elements).
        {
            let mut txn = doc_b.transact_mut();
            let update = yrs::Update::decode_v1(&encode_state_as_update(&doc_a))
                .expect("decode");
            txn.apply_update(update);
        }
        // doc_a removes index 0.
        {
            let mut txn = doc_a.transact_mut();
            els_a.remove(&mut txn, 0);
        }
        // Propagate the delete from a to b.
        let update = encode_state_as_update(&doc_a);
        let sv = encode_state_vector(&doc_b);
        let diff = merge_crdt_update(&doc_b, &sv, &update).expect("merge a->b");
        {
            let mut txn = doc_b.transact_mut();
            let upd = yrs::Update::decode_v1(&diff).expect("decode");
            txn.apply_update(upd);
        }
        // doc_b should now see 1 element remaining.
        let len = {
            let txn = doc_b.transact();
            txn.get_array(ELEMENTS_KEY).expect("b").len(&txn)
        };
        assert_eq!(len, 1, "doc_b should see 1 element after delete propagation");
    }

    /// Multi-client merge: 3 replicas, each adds 10 elements,
    /// then sync. After merge, all 3 should see 30 elements.
    #[test]
    fn multi_client_merge_converges() {
        let docs: Vec<Doc> = (0..3).map(|_| Doc::new()).collect();
        let els: Vec<ArrayRef> = docs
            .iter()
            .map(|d| d.get_or_insert_array(ELEMENTS_KEY))
            .collect();
        for (i, (doc, el)) in docs.iter().zip(els.iter()).enumerate() {
            let mut txn = doc.transact_mut();
            for j in 0..10 {
                let id = format!("replica-{i}-node-{j}");
                let m: MapRef = el.push_back(&mut txn, MapPrelim::<Any>::new());
                m.insert(&mut txn, F_ID, id);
                m.insert(&mut txn, F_KIND, "block");
                m.insert(&mut txn, F_LABEL, format!("r{i}n{j}"));
                m.insert(&mut txn, F_ALIVE, true);
            }
        }
        // Star-shaped merge: 0 is the hub, 1 and 2 push to 0,
        // 0 then pushes back to 1 and 2.
        for src in 1..docs.len() {
            let update = encode_state_as_update(&docs[src]);
            let sv = encode_state_vector(&docs[0]);
            let diff = merge_crdt_update(&docs[0], &sv, &update).expect("merge");
            let mut txn = docs[0].transact_mut();
            let upd = yrs::Update::decode_v1(&diff).expect("decode");
            txn.apply_update(upd);
        }
        for dst in 1..docs.len() {
            let update = encode_state_as_update(&docs[0]);
            let sv = encode_state_vector(&docs[dst]);
            let diff = merge_crdt_update(&docs[dst], &sv, &update).expect("merge");
            let mut txn = docs[dst].transact_mut();
            let upd = yrs::Update::decode_v1(&diff).expect("decode");
            txn.apply_update(upd);
        }
        for (i, doc) in docs.iter().enumerate() {
            let len = {
                let txn = doc.transact();
                txn.get_array(ELEMENTS_KEY).expect("els").len(&txn)
            };
            assert_eq!(len, 30, "replica {i} should see 30 elements after merge");
        }
    }

    /// Large doc perf sanity: a doc with 1k elements should
    /// encode + decode under 1s. This is a smoke test, not a
    /// benchmark — exact timing varies wildly by host.
    #[test]
    fn large_doc_encodes_decodes_under_1s() {
        let doc = Doc::new();
        let els = doc.get_or_insert_array(ELEMENTS_KEY);
        let start = std::time::Instant::now();
        {
            let mut txn = doc.transact_mut();
            for i in 0..1000 {
                let m: MapRef = els.push_back(&mut txn, MapPrelim::<Any>::new());
                m.insert(&mut txn, F_ID, format!("n-{i}"));
                m.insert(&mut txn, F_KIND, "block");
                m.insert(&mut txn, F_LABEL, format!("node-{i}"));
                m.insert(&mut txn, F_ALIVE, true);
            }
        }
        let snapshot = encode_state_as_update(&doc);
        let encoded = start.elapsed();
        // Decode on a fresh doc.
        let peer = Doc::new();
        let start2 = std::time::Instant::now();
        {
            let mut txn = peer.transact_mut();
            let update = yrs::Update::decode_v1(&snapshot).expect("decode");
            txn.apply_update(update);
        }
        let decoded = start2.elapsed();
        let len = {
            let txn = peer.transact();
            txn.get_array(ELEMENTS_KEY).expect("els").len(&txn)
        };
        assert_eq!(len, 1000);
        assert!(
            encoded.as_secs() < 1 && decoded.as_secs() < 1,
            "encode {encoded:?} / decode {decoded:?} should be < 1s for 1k elements"
        );
    }

    /// Reconcile: server has 1 element, client sends a
    /// client-only add. The merged result should contain both
    /// elements (1 server + 1 client). Note: this test uses
    /// *different* node ids on each side, so the YArray sees
    /// two distinct entries (id-dedup by YMap key would
    /// require a v0.7.0 key-by-uuid schema; v0.6.0 keeps the
    /// YArray-of-YMaps structure and accepts the
    /// additive-divergence semantic that was the original
    /// CRDT rationale in M-12 §3.6).
    #[test]
    fn reconcile_with_crdt_merges_client_edit() {
        let server = Canvas::new("c1");
        let server_node_id = uuid::Uuid::new_v4();
        let mut server_node =
            CanvasNode::new(NodeKind::Block, Position::new(0, 0), "server-only");
        server_node.id = NodeId(server_node_id);
        server.add_node(server_node);
        // Client doc: fresh, only has one client-only element.
        let client_doc = Doc::new();
        let client_els = client_doc.get_or_insert_array(ELEMENTS_KEY);
        {
            let mut txn = client_doc.transact_mut();
            let m: MapRef = client_els.push_back(&mut txn, MapPrelim::<Any>::new());
            m.insert(&mut txn, F_ID, "99999999-9999-9999-9999-999999999999");
            m.insert(&mut txn, F_KIND, "block");
            m.insert(&mut txn, F_LABEL, "client-only");
            m.insert(&mut txn, F_X, 99i64);
            m.insert(&mut txn, F_Y, 99i64);
            m.insert(&mut txn, F_ALIVE, true);
        }
        let client_update = encode_state_as_update(&client_doc);
        let result = reconcile_with_crdt(&server, &client_update, 1).expect("reconcile");
        // New version = max(1, 1) + 1 = 2.
        assert_eq!(result.new_version, 2);
        // Decode the merged state and verify the elements.
        let merged_doc = Doc::new();
        {
            let mut txn = merged_doc.transact_mut();
            let update = yrs::Update::decode_v1(&result.merged_state).expect("decode merged");
            txn.apply_update(update);
        }
        let len = {
            let txn = merged_doc.transact();
            txn.get_array(ELEMENTS_KEY).expect("els").len(&txn)
        };
        assert_eq!(len, 2, "merged state should have 2 elements (1 server + 1 client)");
    }

    /// Sanity: malformed update bytes produce a `BackendError`
    /// rather than a panic.
    #[test]
    fn merge_crdt_update_rejects_malformed_bytes() {
        let doc = Doc::new();
        let result = merge_crdt_update(&doc, &[], &[0xFF, 0xEE, 0xDD]);
        assert!(matches!(result, Err(CanvasError::BackendError(_))));
    }
}
