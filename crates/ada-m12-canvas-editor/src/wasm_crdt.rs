//! v0.7.0 WASM bindings for the M-12 CRDT (Yrs) sync path.
//!
//! This module is gated by `--features wasm-crdt` and
//! exposes a `WasmCrdtDoc` wrapper around `yrs::Doc` so
//! the v0.7.0 YMap-keyed-by-uuid CRDT schema can be driven
//! from a JS host (the v0.7.0 web frontend, m13
//! cross-crate consumers with a JS bridge, etc.).
//!
//! Distinct from the existing `wasm` feature, which
//! wraps the v0.5.0 `Canvas` surface in `src/wasm.rs`.
//! `wasm-crdt` is the v0.7.0 forward path and exports the
//! YMap-keyed-by-uuid schema. Both features are independent
//! and can be enabled together for clients that want both
//! surfaces.
//!
//! ## API surface
//!
//! - [`WasmCrdtDoc::new`] — fresh `YDoc` with a random
//!   yrs client id
//! - [`WasmCrdtDoc::new_with_client_id`] — fresh `YDoc`
//!   with an explicit yrs client id (u64 high bits; see
//!   `crdt::ClientId::uuid.as_u128() as u64`)
//! - [`WasmCrdtDoc::apply_update`] — apply a remote
//!   yrs-encoded update, return the diff
//! - [`WasmCrdtDoc::encode_state`] — full state snapshot
//! - [`WasmCrdtDoc::get_elements`] — list live elements
//!   as a `JsValue` (JS array of plain objects)
//! - [`WasmCrdtDoc::get_edges`] — list live edges as a
//!   `JsValue` (JS array of plain objects)
//! - [`WasmCrdtDoc::insert_element_json`] — insert an
//!   element from a `JsValue` (JSON)
//! - [`WasmCrdtDoc::update_element_position_json`] —
//!   update an element's position from a `JsValue`
//! - [`WasmCrdtDoc::remove_element`] — remove an element
//!   by uuid
//!
//! ## Snapshot roundtrip doctest
//!
//! ```
//! // The wasm-crdt feature requires WASM bindings; this
//! // doc-test is only meaningful under `wasm-pack test`,
//! // which is not part of the 5-gate CI default build.
//! // See `docs/decisions/02-design-adrs.md` D-02.
//! ```

#![cfg(feature = "wasm-crdt")]

use wasm_bindgen::prelude::*;
use yrs::updates::decoder::Decode;
use yrs::updates::encoder::Encode;
use yrs::{Doc, ReadTxn, Transact};

use crate::crdt;
use crate::node::CanvasNode;

/// WASM wrapper for a `yrs::Doc` carrying the v0.7.0
/// YMap-keyed-by-uuid CRDT schema. Exposed to JS via
/// `wasm-bindgen`. All `Vec<u8>` arguments are yrs
/// v1-encoded update bytes; see `yrs::Update::decode_v1` /
/// `yrs::TransactionMut::encode_state_as_update_v1`.
#[wasm_bindgen]
pub struct WasmCrdtDoc {
    // `pub(crate)` so the in-crate tests can sync via
    // `inner_state_vector` / `inner_apply_diff` without
    // widening the wasm-bindgen surface.
    pub(crate) doc: Doc,
}

impl WasmCrdtDoc {
    /// Test-only helper: encode the current state vector
    /// so tests can pass it as the `remote_state` arg to
    /// `apply_update`. Not exposed to JS.
    #[cfg(test)]
    pub(crate) fn inner_state_vector(&self) -> Vec<u8> {
        let txn = self.doc.transact();
        txn.state_vector().encode_v1()
    }

    /// Test-only helper: apply a yrs v1-encoded diff to
    /// the inner Doc (mirroring the
    /// `TransactionMut::apply_update` call the JS host
    /// would do after receiving the diff from
    /// `apply_update`). Not exposed to JS.
    #[cfg(test)]
    pub(crate) fn inner_apply_diff(&mut self, diff: &[u8]) {
        let mut txn = self.doc.transact_mut();
        let u = yrs::Update::decode_v1(diff).expect("dec diff");
        txn.apply_update(u);
    }
}

#[wasm_bindgen]
impl WasmCrdtDoc {
    /// Create a fresh `WasmCrdtDoc` with a random yrs
    /// client id. For explicit client_id negotiation (the
    /// v0.7.0 recommended path), use
    /// [`WasmCrdtDoc::new_with_client_id`].
    #[wasm_bindgen(constructor)]
    pub fn new() -> WasmCrdtDoc {
        WasmCrdtDoc { doc: Doc::new() }
    }

    /// Create a fresh `WasmCrdtDoc` with an explicit yrs
    /// client id (the low 64 bits of the v0.7.0
    /// `crdt::ClientId::uuid.as_u128()`). Use this when
    /// the JS host wants to negotiate a stable client id
    /// with the server (yrs embeds the client id in the
    /// varint header of every update entry, so two
    /// replicas with the same client id produce the same
    /// state vectors; replicas with different client ids
    /// never alias).
    #[wasm_bindgen(js_name = newWithClientId)]
    pub fn new_with_client_id(client_id: u64) -> WasmCrdtDoc {
        WasmCrdtDoc {
            doc: Doc::with_client_id(client_id),
        }
    }

    /// Apply a remote yrs-encoded update to the doc and
    /// return a follow-up diff (yrs v1-encoded bytes) that
    /// the remote can apply to converge. Mirrors
    /// `crdt::merge_crdt_update` semantics exactly.
    ///
    /// `update_bytes` — the remote's full state or diff
    /// (produced by `encode_state` on the other replica).
    ///
    /// `remote_state` — the remote's state vector (yrs v1-
    /// encoded), so the returned diff is exactly what the
    /// remote is missing.
    #[wasm_bindgen(js_name = applyUpdate)]
    pub fn apply_update(
        &mut self,
        update_bytes: &[u8],
        remote_state: &[u8],
    ) -> Result<Vec<u8>, JsError> {
        crdt::merge_crdt_update(&self.doc, remote_state, update_bytes)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Encode the full doc state as a yrs v1 update. Use
    /// for first-sync (a fresh client connects and
    /// downloads the full state) and for snapshotting to
    /// disk. Mirrors `crdt::encode_state_as_update`.
    #[wasm_bindgen(js_name = encodeState)]
    pub fn encode_state(&self) -> Vec<u8> {
        crdt::encode_state_as_update(&self.doc)
    }

    /// List all live elements as a `JsValue` (JS array of
    /// plain objects with `id`, `kind`, `x`, `y`, `label`,
    /// `ports`, `alive`). Mirrors `crdt::iter_elements` /
    /// `crdt::ElementSnapshot` (serialised via
    /// `serde_json`). The JS host can `JSON.parse` or
    /// pass the array to other wasm-bindgen consumers.
    #[wasm_bindgen(js_name = getElements)]
    pub fn get_elements(&self) -> Result<JsValue, JsError> {
        let arr: Vec<serde_json::Value> = crdt::iter_elements(&self.doc)
            .map(|(_uuid, snap)| serde_json::to_value(&snap).unwrap_or(serde_json::Value::Null))
            .collect();
        serde_wasm_bindgen::to_value(&arr).map_err(|e| JsError::new(&e.to_string()))
    }

    /// List all live edges as a `JsValue` (JS array of
    /// plain objects with `from`, `to`, `label`). Mirrors
    /// `crdt::iter_edge_keys` + `crdt::get_edge`. Edge
    /// `from` / `to` are uuid strings; `label` is the
    /// optional human-readable label (or `null` if unset).
    #[wasm_bindgen(js_name = getEdges)]
    pub fn get_edges(&self) -> Result<JsValue, JsError> {
        let mut edges: Vec<serde_json::Value> = Vec::new();
        for (from, to) in crdt::iter_edge_keys(&self.doc) {
            if let Some(snap) = crdt::get_edge(&self.doc, from, to) {
                let v = serde_json::to_value(&snap).unwrap_or(serde_json::Value::Null);
                edges.push(v);
            }
        }
        serde_wasm_bindgen::to_value(&edges).map_err(|e| JsError::new(&e.to_string()))
    }

    /// Insert an element from a JSON object. The JSON
    /// shape matches `crdt::ElementSnapshot` (i.e. the
    /// output of `get_elements`). Returns `true` on
    /// success. The element id must be present and
    /// unique; mirrors `crdt::insert_element`.
    #[wasm_bindgen(js_name = insertElementJson)]
    pub fn insert_element_json(&mut self, json: &JsValue) -> Result<bool, JsError> {
        let snap: crdt::ElementSnapshot = serde_wasm_bindgen::from_value(json.clone())
            .map_err(|e| JsError::new(&e.to_string()))?;
        let mut n = CanvasNode::new(snap.kind, snap.position, &snap.label);
        n.id = snap.id;
        crdt::insert_element(&self.doc, &n).map_err(|e| JsError::new(&e.to_string()))?;
        Ok(true)
    }

    /// Update an element's position from a JSON object
    /// `{"id": <uuid>, "x": <i32>, "y": <i32>}`. Returns
    /// `true` if the element existed.
    #[wasm_bindgen(js_name = updateElementPositionJson)]
    pub fn update_element_position_json(&mut self, json: &JsValue) -> Result<bool, JsError> {
        #[derive(serde::Deserialize)]
        struct Pos {
            id: uuid::Uuid,
            x: i32,
            y: i32,
        }
        let p: Pos = serde_wasm_bindgen::from_value(json.clone())
            .map_err(|e| JsError::new(&e.to_string()))?;
        let upd = crdt::ElementUpdate::new().position(crate::node::Position::new(p.x, p.y));
        crdt::update_element(&self.doc, crate::node::NodeId(p.id), upd)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Remove an element by uuid. Returns `true` if the
    /// element existed. Mirrors `crdt::remove_element`.
    #[wasm_bindgen(js_name = removeElement)]
    pub fn remove_element(&mut self, id: &str) -> Result<bool, JsError> {
        let uuid =
            uuid::Uuid::parse_str(id).map_err(|e| JsError::new(&format!("invalid uuid: {e}")))?;
        crdt::remove_element(&self.doc, crate::node::NodeId(uuid))
            .map_err(|e| JsError::new(&e.to_string()))
    }
}

impl Default for WasmCrdtDoc {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen(start)]
pub fn _start() {
    // Forward panics to `console.error` so the JS host
    // gets a clear stack trace instead of a hard
    // `RuntimeError: unreachable executed`.
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

#[cfg(all(test, target_arch = "wasm32"))]
mod tests {
    //! WASM-target-only smoke test. The `serde_wasm_bindgen`
    //! / `JsValue` round-trip is only meaningful under
    //! `wasm-pack test --headless --chrome`; on native
    //! targets those types are stubs and panic. The test
    //! verifies a basic snapshot round-trip:
    //!   1. a creates a doc, inserts an element via
    //!      `insert_element_json`
    //!   2. a encodes its state + state vector
    //!   3. b applies a's update and the returned diff
    //!   4. b reads back the element list via
    //!      `get_elements` and asserts the round-trip
    //!      preserved the label

    use super::*;

    #[test]
    fn snapshot_roundtrip_smoke() {
        let mut a = WasmCrdtDoc::new();
        let mut b = WasmCrdtDoc::new();

        // a inserts an element via JSON.
        let json = serde_json::json!({
            "id": "11111111-1111-1111-1111-111111111111",
            "kind": "block",
            "position": { "x": 10, "y": 20 },
            "label": "hello",
            "ports": [],
            "alive": true,
        });
        a.insert_element_json(&serde_wasm_bindgen::to_value(&json).unwrap())
            .expect("insert");

        // a → b sync (full-state + diff, mirroring the
        // 2-step merge pattern used by the native
        // `merge_crdt_update` integration test).
        let upd_a = a.encode_state();
        let sv_a = a.inner_state_vector();
        let diff_b = b.apply_update(&upd_a, &sv_a).expect("a->b");
        b.inner_apply_diff(&diff_b);

        // b should now see the same element.
        let elements = b.get_elements().expect("get");
        let arr: Vec<serde_json::Value> = serde_wasm_bindgen::from_value(elements).unwrap();
        assert_eq!(arr.len(), 1, "b should see 1 element after sync");
        assert_eq!(arr[0]["label"], "hello");
    }
}
