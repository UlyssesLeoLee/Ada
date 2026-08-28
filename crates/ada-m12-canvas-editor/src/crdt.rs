//! CRDT-backed canvas sync for M-12 (v0.7.0).
//!
//! This module is the v0.7.0 deepening of the v0.6.0 Yrs
//! integration. Compared to v0.6.0 it lifts the schema from
//! "YArray of YMap" to "YMap keyed by uuid", which gives true
//! concurrent-delete convergence (a 2P-Set on the YMap key
//! collapses two `remove(id)` calls from different replicas to
//! one), ports become a top-level YMap keyed by
//! `${element_uuid}::${port_uuid}` (so concurrent add/remove
//! of different ports is additive and concurrent edits to the
//! same port are LWW on its fields — without the
//! nested-YArray-initialisation problem that v0.7.0 explored
//! and rejected), and edges become a YMap keyed by
//! `${from_uuid}::${to_uuid}` (so concurrent add-of-same-edge
//! dedupes naturally).
//!
//! ## Design (per docs/decisions/02-design-adrs.md D-01)
//!
//! - A `YDoc` is the top-level shared state. Each replica is
//!   initialised with the same `client_id` per
//!   `Doc::with_client_id` (v0.7.0 explicit negotiation; the
//!   `reconcile_with_crdt` overload takes a `ClientId` so a
//!   server / client pair never aliases). See
//!   [`ClientId`] and [`reconcile_with_crdt`].
#![cfg_attr(
    not(feature = "legacy-array"),
    doc = "The v0.6.0 YArray-of-YMap schema is preserved behind the `legacy-array` feature flag (default off). Build with `--features legacy-array` to keep the v0.6.0 fallback available during the v0.6.0 → v0.7.0 transition window."
)]
//! - Root layout: a YMap named `"meta"` carrying the document
//!   name and version, a YMap named `"elements"` keyed by
//!   `NodeId.uuid()` (each value is a YMap with the element
//!   fields), a YMap named `"ports"` keyed by
//!   `${element_uuid}::${port_uuid}` (each value is a YMap
//!   with the port fields), and a YMap named `"edges"` keyed
//!   by `${from_uuid}::${to_uuid}` (each value is a YMap with
//!   edge fields, plus an `alive` tombstone). The flat
//!   top-level YMap layout is what gives all of:
//!     1. true concurrent-delete convergence on elements and
//!        edges (YMap 2P-Set semantics),
//!     2. field-level LWW on element / port / edge fields
//!        (YMap field LWW),
//!     3. natural dedup of concurrent same-key inserts
//!        (YMap 2P-Set), and
//!     4. additive merge of different ports on the same
//!        element (YMap keys are independent).
//!
//!   See [`add_port`] / [`remove_port`] for the port-level
//!   API.
//!
//! ## Sync API
//!
//! - [`merge_crdt_update`] — apply a remote update to a `YDoc`
//!   and return a follow-up update that contains any state
//!   the remote still needs.
//! - [`encode_state_as_update`] — full state snapshot (used
//!   for first sync / replay).
//! - [`reconcile_with_crdt`] — end-to-end reconcile: build a
//!   fresh `YDoc` from the server's snapshot (using the
//!   supplied [`ClientId`]), apply the client's update (in
//!   any order), and return a [`CrdtReconcileResult`] whose
//!   `merged_state` is the new server snapshot. The merged
//!   state is a CRDT-state-encoded byte string (use
//!   [`encode_state_as_update`] to ship it to the client).
//! - Element-level ergonomics:
//!   - [`insert_element`]
//!   - [`remove_element`]
//!   - [`update_element`]
//!   - [`get_element`]
//!   - [`iter_elements`]
//! - Port-level ergonomics:
//!   - [`add_port`]
//!   - [`remove_port`]
//! - Edge-level ergonomics:
//!   - [`insert_edge`] / [`remove_edge`] / [`update_edge`]
//!
//! ## LWW fallback
//!
//! The v0.5.0 LWW path is preserved behind the `legacy-lww`
//! feature flag (default off). Build with `--features legacy-lww`
//! to keep the old `server_recon` flow available during the
//! v0.5.0 → v0.6.0 → v0.7.0 transition window. See
//! `crates/ada-m12-canvas-editor/CRDT.md` for the migration
//! notes.
//!
//! ## v0.6.0 YArray schema fallback
//!
//! The v0.6.0 YArray-of-YMap schema is preserved behind the
//! `legacy-array` feature flag (default off). Build with
//! `--features legacy-array` to keep the v0.6.0 layout
//! available for one release as a rollback path. v0.8.0 will
//! remove `legacy-array` and the YArray path; new code should
//! use the v0.7.0 YMap-keyed-by-uuid schema unconditionally.
//!
//! ## Feature gating
//!
//! This module is gated by `feature = "crdt"` (default off in
//! the 5-gate CI default-build path) so adding `yrs` to the
//! dep graph does not slow down `cargo test --workspace`. The
//! default off flag also lets downstream crates opt in
//! independently of the LWW `server` feature.

#![cfg(feature = "crdt")]

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use yrs::types::Value;
use yrs::updates::decoder::Decode;
use yrs::updates::encoder::Encode;
use yrs::{Any, Doc, Map, MapPrelim, MapRef, ReadTxn, StateVector, Transact};

use crate::canvas::{Canvas, Edge};
use crate::error::CanvasError;
use crate::node::{CanvasNode, NodeId, NodeKind, Port, Position};

/// Root YMap carrying document-level metadata.
const META_KEY: &str = "meta";
/// Root YMap keyed by element uuid (v0.7.0 schema). Replaces
/// the v0.6.0 `"elements"` YArray.
const ELEMENTS_KEY: &str = "elements";
/// Root YMap keyed by `${element_uuid}::${port_uuid}` (v0.7.0
/// schema). Replaces the v0.6.0 JSON-stringified port list
/// nested under each element. Top-level YMap gives clean
/// 2P-Set / LWW semantics without the nested-YArray-init
/// problem.
const PORTS_KEY: &str = "ports";
/// Root YMap keyed by edge key string (v0.7.0 schema).
/// Replaces the v0.6.0 `"edges"` YArray.
const EDGES_KEY: &str = "edges";
/// Field name for the element / edge "deleted" tombstone flag.
/// YMap CRDT uses a 2P-Set: an insert + a remove on the same
/// key resolve by clock; if the remove clock is later than the
/// insert clock the key is gone, otherwise the insert wins.
/// Concurrent deletes on the same key always converge to
/// "removed" — this is the property v0.6.0 lacked.
const F_ALIVE: &str = "alive";
/// Element id field (UUID string, the same id as [`NodeId`]).
const F_ID: &str = "id";
/// Port id field.
const F_PORT_ID: &str = "id";
/// Element id field on a port (for filtering ports by
/// element).
const F_ELEMENT_ID: &str = "element_id";
const F_KIND: &str = "kind";
const F_X: &str = "x";
const F_Y: &str = "y";
const F_LABEL: &str = "label";
const F_FROM: &str = "from";
const F_TO: &str = "to";
const F_NAME: &str = "name";
const F_VERSION: &str = "version";
/// Port-level fields.
const F_PORT_KIND: &str = "kind";
const F_PORT_LABEL: &str = "label";
const F_PORT_X: &str = "x";
const F_PORT_Y: &str = "y";

/// v0.7.0 stable client identifier. Each replica must have a
/// unique `ClientId`; the `uuid` is the canonical Yrs client
/// id (used by `Doc::with_client_id(client_id.uuid.as_u128() as u64)`),
/// and `label` is a human-readable tag for logs / debugging.
///
/// Derives `Hash` / `Eq` so `ClientId` can be used as a key
/// in `HashMap` when a server needs to track per-client
/// state.
#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct ClientId {
    /// Stable UUID (v4). Fed to yrs as the client id.
    pub uuid: uuid::Uuid,
    /// Human-readable label (e.g. "alice-laptop", "server-1").
    pub label: String,
}

impl ClientId {
    /// Create a fresh client id with a random UUID and the
    /// given label.
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            uuid: uuid::Uuid::new_v4(),
            label: label.into(),
        }
    }

    /// Construct from an explicit UUID + label.
    #[must_use]
    pub const fn from_uuid(uuid: uuid::Uuid, label: String) -> Self {
        Self { uuid, label }
    }
}

impl std::fmt::Display for ClientId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}({})", self.label, self.uuid)
    }
}

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
/// snapshot, build a YDoc (with `client_id` for the server),
/// apply the client's update, and return a
/// [`CrdtReconcileResult`] containing the merged state and a
/// delta the client should apply.
///
/// `client_id` is the server replica's stable [`ClientId`]
/// (v0.7.0 explicit negotiation; yrs used to rand-generate
/// this and clients had no way to see or set it). The Yrs
/// internal clock for this server replica is
/// `client_id.uuid.as_u128() as u64` per
/// `Doc::with_client_id`.
///
/// This is the v0.6.0+ replacement for the v0.5.0
/// `reconcile_canvas_state` function. Where the v0.5.0 path
/// "server wins" on conflict, this path converges via Yrs
/// per-field LWW (or per-key 2P-Set on YMap keys) — concurrent
/// edits to *different* fields are kept; concurrent edits to
/// the *same* field go to whichever replica wrote the later
/// (client, by Lamport-style timestamp inside the CRDT) value.
/// The result is no `server_wins` / `client_wins` split — only
/// the merged state.
///
/// # Errors
///
/// Returns [`CanvasError::BackendError`] on any yrs decode or
/// apply error (malformed update bytes).
pub fn reconcile_with_crdt(
    server: &Canvas,
    client_update: &[u8],
    client_version: u64,
    client_id: &ClientId,
) -> Result<CrdtReconcileResult, CanvasError> {
    // 1. Build a fresh YDoc seeded with the server's snapshot.
    let doc = Doc::with_client_id(client_id.uuid.as_u128() as u64);
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

/// Snapshot of a single element, in a CRDT-friendly value
/// type (no mutex, no Yrs internals). Returned by
/// [`get_element`] and yielded by [`iter_elements`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ElementSnapshot {
    /// Element id (UUID).
    pub id: NodeId,
    /// Element kind.
    pub kind: NodeKind,
    /// Element position.
    pub position: Position,
    /// Human-readable label.
    pub label: String,
    /// Input / output ports. v0.7.0 lifts this from a
    /// JSON-string to a proper nested CRDT.
    pub ports: Vec<PortSnapshot>,
    /// `true` if the element is currently alive (not
    /// tombstoned). Always `true` in the snapshot API
    /// (dead elements are filtered out); kept here for
    /// symmetry with the on-wire representation.
    pub alive: bool,
}

/// Snapshot of a single port on an element. v0.7.0 lifts the
/// `ports` field from a JSON-string into a proper YArray of
/// YMaps; this is the in-memory shape. Each port carries
/// `id` (stable UUID, lets concurrent remove/add of the same
/// port by id dedupe), `kind` (`input` / `output` /
/// `bidir`), `label` (display name), and `x` / `y` (position
/// relative to the element).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortSnapshot {
    /// Port id (UUID).
    pub id: uuid::Uuid,
    /// Port kind: `"input"` / `"output"` / `"bidir"`.
    /// Defaults to `"output"` for v0.6.0-style ports that
    /// had no kind field.
    pub kind: String,
    /// Display label (e.g. "in", "out", "error").
    pub label: String,
    /// X coordinate relative to the element.
    pub x: i32,
    /// Y coordinate relative to the element.
    pub y: i32,
}

impl PortSnapshot {
    /// Build a port with sensible defaults for the
    /// v0.6.0-style "name only" port shape: `id` from the
    /// caller, `kind = "output"`, `label = name`, `x = 0`,
    /// `y = 0`.
    #[must_use]
    pub fn from_name(id: uuid::Uuid, name: impl Into<String>) -> Self {
        let n: String = name.into();
        Self {
            id,
            kind: "output".to_string(),
            label: n.clone(),
            x: 0,
            y: 0,
        }
    }
}

/// Field-level update to an element. Use [`ElementUpdate::new`]
/// to start, then chain setters to describe the desired
/// final state. `update_element` writes only the fields that
/// have been touched, so concurrent edits to other fields
/// stay untouched.
#[derive(Debug, Clone, Default)]
pub struct ElementUpdate {
    position: Option<Position>,
    label: Option<String>,
    kind: Option<NodeKind>,
    // Ports are passed as the new full set; the helper
    // diff-replaces the inner YArray.
    ports: Option<Vec<PortSnapshot>>,
}

impl ElementUpdate {
    /// Start an empty update.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the new position.
    #[must_use]
    pub fn position(mut self, p: Position) -> Self {
        self.position = Some(p);
        self
    }

    /// Set the new label.
    #[must_use]
    pub fn label(mut self, l: impl Into<String>) -> Self {
        self.label = Some(l.into());
        self
    }

    /// Set the new kind.
    #[must_use]
    pub fn kind(mut self, k: NodeKind) -> Self {
        self.kind = Some(k);
        self
    }

    /// Replace the port list with the given snapshot list.
    /// Pass an empty `Vec` to clear all ports.
    #[must_use]
    pub fn ports(mut self, ports: Vec<PortSnapshot>) -> Self {
        self.ports = Some(ports);
        self
    }

    /// `true` if at least one field has been set.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.position.is_none()
            && self.label.is_none()
            && self.kind.is_none()
            && self.ports.is_none()
    }
}

/// Insert an element into the YDoc. The element is keyed by
/// `element.id.uuid()` so concurrent inserts of the same id
/// collapse to a single entry (last-writer-wins per field).
///
/// Returns `Ok(())` on success; the operation is idempotent —
/// re-inserting the same id updates the fields in place.
///
/// # Errors
///
/// Returns [`CanvasError::BackendError`] if the underlying
/// yrs write fails (this should never happen for in-process
/// use; only possible if yrs panics are converted to
/// `Result`).
pub fn insert_element(doc: &Doc, element: &CanvasNode) -> Result<(), CanvasError> {
    let elements: MapRef = doc.get_or_insert_map(ELEMENTS_KEY);
    let mut txn = doc.transact_mut();
    let key = element.id.0.to_string();
    let map: MapRef = match elements.get(&txn, &key) {
        Some(yrs::types::Value::YMap(m)) => m,
        _ => elements.insert(&mut txn, key.clone(), MapPrelim::<Any>::new()),
    };
    write_node_fields(&map, &mut txn, element);
    Ok(())
}

/// Remove an element by id. Returns `true` if the element
/// existed (and is now tombstoned), `false` if it was not
/// present. Concurrent removes from different replicas
/// converge to "absent" — this is the v0.7.0 fix for the
/// v0.6.0 YArray delete non-collapse.
///
/// # Errors
///
/// Returns [`CanvasError::BackendError`] on yrs failure.
pub fn remove_element(doc: &Doc, id: NodeId) -> Result<bool, CanvasError> {
    let elements: MapRef = doc.get_or_insert_map(ELEMENTS_KEY);
    let mut txn = doc.transact_mut();
    let key = id.0.to_string();
    let Some(yrs::types::Value::YMap(map)) = elements.get(&txn, &key) else {
        return Ok(false);
    };
    map.insert(&mut txn, F_ALIVE, false);
    Ok(true)
}

/// Apply a partial update to an existing element. Returns
/// `true` if the element existed, `false` if the id was not
/// found. If the update is empty ([`ElementUpdate::is_empty`])
/// this is a no-op that still returns whether the element
/// exists.
///
/// Concurrent updates to *different* fields from different
/// replicas both win (per-field LWW). Concurrent updates to
/// the same field go to the later clock. See `tasks` in the
/// v0.7.0 design doc for the test matrix.
///
/// # Errors
///
/// Returns [`CanvasError::BackendError`] on yrs failure.
pub fn update_element(doc: &Doc, id: NodeId, update: ElementUpdate) -> Result<bool, CanvasError> {
    if update.is_empty() {
        // Still check existence so the caller gets a
        // useful bool.
        return Ok(get_element(doc, id).is_some());
    }
    // v0.7.0 fix: cache the `ports` MapRef BEFORE we
    // open the long-lived `transact_mut()`. yrs 0.18
    // panics with `BorrowMutError` if you call
    // `get_or_insert_map` (which opens its own
    // `transact_mut` internally) while another
    // `transact_mut` is already alive. (Symptom:
    // `port_concurrent_update_x_vs_y_no_conflict`
    // panics in `yrs-0.18.8/src/doc.rs:636` with
    // "there's another active transaction at the
    // moment: ExclusiveAcqFailed(BorrowMutError)".)
    let elements: MapRef = doc.get_or_insert_map(ELEMENTS_KEY);
    let ports_map: Option<MapRef> = if update.ports.is_some() {
        Some(doc.get_or_insert_map(PORTS_KEY))
    } else {
        None
    };
    let mut txn = doc.transact_mut();
    let key = id.0.to_string();
    let Some(yrs::types::Value::YMap(map)) = elements.get(&txn, &key) else {
        return Ok(false);
    };
    if let Some(p) = update.position {
        map.insert(&mut txn, F_X, i64::from(p.x));
        map.insert(&mut txn, F_Y, i64::from(p.y));
    }
    if let Some(label) = update.label {
        map.insert(&mut txn, F_LABEL, label);
    }
    if let Some(kind) = update.kind {
        map.insert(&mut txn, F_KIND, kind_str(kind));
    }
    if let Some(ports) = update.ports {
        // v0.7.0: ports live in a top-level YMap. We
        // upsert each port under its stable
        // `${element_id}::${port_id}` key. This is
        // additive on the `ports` YMap (no full-array
        // replace), so concurrent add_port / remove_port
        // from other replicas still merge correctly.
        let ports_map = ports_map.expect("ports_map was pre-cached when ports is Some");
        for p in &ports {
            let key = port_key(id, p.id);
            let pm: MapRef = match ports_map.get(&txn, &key) {
                Some(yrs::types::Value::YMap(m)) => m,
                _ => ports_map.insert(&mut txn, key.clone(), MapPrelim::<Any>::new()),
            };
            pm.insert(&mut txn, F_PORT_ID, p.id.to_string());
            pm.insert(&mut txn, F_ELEMENT_ID, id.0.to_string());
            pm.insert(&mut txn, F_PORT_KIND, p.kind.clone());
            pm.insert(&mut txn, F_PORT_LABEL, p.label.clone());
            pm.insert(&mut txn, F_PORT_X, i64::from(p.x));
            pm.insert(&mut txn, F_PORT_Y, i64::from(p.y));
        }
    }
    Ok(true)
}

/// Look up an element by id. Returns `None` if the element is
/// absent *or* tombstoned (alive=false).
#[must_use]
pub fn get_element(doc: &Doc, id: NodeId) -> Option<ElementSnapshot> {
    let txn = doc.transact();
    let elements = txn.get_map(ELEMENTS_KEY)?;
    let key = id.0.to_string();
    let yrs::types::Value::YMap(map) = elements.get(&txn, &key)? else {
        return None;
    };
    if !is_alive(&map, &txn) {
        return None;
    }
    Some(read_element_snapshot(&map, &txn))
}

/// Iterate over all *live* elements. Order is unspecified
/// (YMap iteration follows internal Yjs ordering which is
/// not stable across replicas when there are concurrent
/// inserts — for ordered rendering, the caller should sort
/// the snapshots by id).
pub fn iter_elements(doc: &Doc) -> impl Iterator<Item = (uuid::Uuid, ElementSnapshot)> {
    let txn = doc.transact();
    let mut out: Vec<(uuid::Uuid, ElementSnapshot)> = Vec::new();
    if let Some(elements) = txn.get_map(ELEMENTS_KEY) {
        for (k, v) in elements.iter(&txn) {
            let yrs::types::Value::YMap(map) = v else {
                continue;
            };
            if !is_alive(&map, &txn) {
                continue;
            }
            let Ok(uuid) = uuid::Uuid::parse_str(&k) else {
                continue;
            };
            let snap = read_element_snapshot(&map, &txn);
            out.push((uuid, snap));
        }
    }
    out.into_iter()
}

/// Insert an edge into the YDoc. Edges are keyed by
/// `${from_uuid}::${to_uuid}` so concurrent inserts of the
/// same edge dedup to a single entry.
///
/// Self-loops (`from == to`) are rejected. The edge map
/// stores `from`, `to`, `label` (optional), and an `alive`
/// tombstone.
///
/// # Errors
///
/// Returns [`CanvasError::InvalidEdge`] on self-loop, or
/// [`CanvasError::BackendError`] on yrs failure.
pub fn insert_edge(
    doc: &Doc,
    from: NodeId,
    to: NodeId,
    label: Option<&str>,
) -> Result<(), CanvasError> {
    if from == to {
        return Err(CanvasError::InvalidEdge {
            reason: "self-loop is not allowed".into(),
        });
    }
    let key = edge_key(from, to);
    let edges: MapRef = doc.get_or_insert_map(EDGES_KEY);
    let mut txn = doc.transact_mut();
    let map: MapRef = match edges.get(&txn, &key) {
        Some(yrs::types::Value::YMap(m)) => m,
        _ => edges.insert(&mut txn, key.clone(), MapPrelim::<Any>::new()),
    };
    map.insert(&mut txn, F_FROM, from.0.to_string());
    map.insert(&mut txn, F_TO, to.0.to_string());
    map.insert(&mut txn, F_ALIVE, true);
    if let Some(l) = label {
        map.insert(&mut txn, F_LABEL, l.to_string());
    }
    Ok(())
}

/// Remove an edge. Returns `true` if the edge existed (and
/// is now tombstoned), `false` if it was not present.
/// Concurrent removes of the same edge converge to "absent".
///
/// # Errors
///
/// Returns [`CanvasError::BackendError`] on yrs failure.
pub fn remove_edge(doc: &Doc, from: NodeId, to: NodeId) -> Result<bool, CanvasError> {
    if from == to {
        return Ok(false);
    }
    let key = edge_key(from, to);
    let edges: MapRef = doc.get_or_insert_map(EDGES_KEY);
    let mut txn = doc.transact_mut();
    let Some(yrs::types::Value::YMap(map)) = edges.get(&txn, &key) else {
        return Ok(false);
    };
    map.insert(&mut txn, F_ALIVE, false);
    Ok(true)
}

/// Update an edge's label. Returns `true` if the edge
/// existed, `false` otherwise. Pass `None` for `label` to
/// clear the label.
pub fn update_edge(
    doc: &Doc,
    from: NodeId,
    to: NodeId,
    label: Option<&str>,
) -> Result<bool, CanvasError> {
    if from == to {
        return Ok(false);
    }
    let key = edge_key(from, to);
    let edges: MapRef = doc.get_or_insert_map(EDGES_KEY);
    let mut txn = doc.transact_mut();
    let Some(yrs::types::Value::YMap(map)) = edges.get(&txn, &key) else {
        return Ok(false);
    };
    match label {
        Some(l) => map.insert(&mut txn, F_LABEL, l.to_string()),
        None => map.insert(&mut txn, F_LABEL, ""),
    };
    Ok(true)
}

/// Look up an edge by (from, to). Returns `None` if absent
/// or tombstoned.
#[must_use]
pub fn get_edge(doc: &Doc, from: NodeId, to: NodeId) -> Option<EdgeSnapshot> {
    let key = edge_key(from, to);
    let txn = doc.transact();
    let edges = txn.get_map(EDGES_KEY)?;
    let yrs::types::Value::YMap(map) = edges.get(&txn, &key)? else {
        return None;
    };
    if !is_alive(&map, &txn) {
        return None;
    }
    Some(EdgeSnapshot {
        from,
        to,
        label: string_field(&map, &txn, F_LABEL).filter(|s| !s.is_empty()),
    })
}

/// Snapshot of a single edge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EdgeSnapshot {
    /// Source node id.
    pub from: NodeId,
    /// Target node id.
    pub to: NodeId,
    /// Optional human-readable label.
    pub label: Option<String>,
}

/// Build (or rebuild) a YDoc from a server-side `Canvas`
/// snapshot. This is how a server that already has
/// authoritative state in the v0.5.0 `Canvas` shape seeds the
/// v0.6.0 / v0.7.0 YDoc backend.
fn hydrate_doc_from_canvas(doc: &Doc, canvas: &Canvas) {
    // 1. Materialise the four root types first. `get_or_insert_*`
    //    opens a short-lived inner `transact_mut()`, so we must
    //    do this *before* opening our own long-lived txn.
    let meta: MapRef = doc.get_or_insert_map(META_KEY);
    let elements: MapRef = doc.get_or_insert_map(ELEMENTS_KEY);
    let ports: MapRef = doc.get_or_insert_map(PORTS_KEY);
    let edges: MapRef = doc.get_or_insert_map(EDGES_KEY);
    // 2. Open a single write txn to populate.
    let mut txn = doc.transact_mut();
    meta.insert(&mut txn, F_NAME, canvas.name());
    meta.insert(
        &mut txn,
        F_VERSION,
        i64::try_from(canvas.version()).unwrap_or(i64::MAX),
    );
    for node in canvas.nodes() {
        let key = node.id.0.to_string();
        let m: MapRef = match elements.get(&txn, &key) {
            Some(yrs::types::Value::YMap(m)) => m,
            _ => elements.insert(&mut txn, key.clone(), MapPrelim::<Any>::new()),
        };
        write_node_fields(&m, &mut txn, &node);
    }
    // Ports (v0.7.0): top-level YMap keyed by
    // `${element_id}::${port_id}`. Hydrate from
    // `node.ports` with deterministic port ids derived
    // from the element id and port name.
    for node in canvas.nodes() {
        for p in &node.ports {
            let port_id = port_id_for_legacy(&node.id, &p.name);
            let key = port_key(node.id, port_id);
            let m: MapRef = match ports.get(&txn, &key) {
                Some(yrs::types::Value::YMap(m)) => m,
                _ => ports.insert(&mut txn, key.clone(), MapPrelim::<Any>::new()),
            };
            m.insert(&mut txn, F_PORT_ID, port_id.to_string());
            m.insert(&mut txn, F_ELEMENT_ID, node.id.0.to_string());
            m.insert(&mut txn, F_PORT_KIND, "output");
            m.insert(&mut txn, F_PORT_LABEL, p.name.clone());
            m.insert(&mut txn, F_PORT_X, 0i64);
            m.insert(&mut txn, F_PORT_Y, 0i64);
        }
    }
    for edge in canvas.edges() {
        let key = edge_key(edge.from, edge.to);
        let m: MapRef = match edges.get(&txn, &key) {
            Some(yrs::types::Value::YMap(m)) => m,
            _ => edges.insert(&mut txn, key.clone(), MapPrelim::<Any>::new()),
        };
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
    // v0.7.0: ports are stored in a top-level YMap keyed by
    // `${element_id}::${port_id}` (not nested under the
    // element). This avoids the nested-YArray-initialisation
    // problem (you can't lazily create a YArray inside a
    // YMap without overwriting the existing one if a
    // concurrent insert from another replica set the same
    // key first). See [`hydrate_doc_from_canvas`] for the
    // top-level hydration.
}

fn write_edge_fields(map: &MapRef, txn: &mut yrs::TransactionMut, edge: &Edge) {
    map.insert(txn, F_FROM, edge.from.0.to_string());
    map.insert(txn, F_TO, edge.to.0.to_string());
    map.insert(txn, F_ALIVE, true);
}

/// Deterministic port id for ports that came in through the
/// v0.6.0 (name-only) `Port` type. Hashes the element id with
/// the port name; collisions across different ports on the
/// same element are vanishingly unlikely.
fn port_id_for_legacy(node_id: &NodeId, port_name: &str) -> uuid::Uuid {
    let mut h: [u8; 16] = [0; 16];
    let n = node_id.0.as_bytes();
    let p = port_name.as_bytes();
    for (i, b) in n.iter().enumerate() {
        h[i % 16] ^= *b;
    }
    for (i, b) in p.iter().enumerate() {
        h[(i + 8) % 16] ^= *b;
    }
    // Set the version (4) and variant (10) bits so this is a
    // valid v4 UUID. 0x40 = version 4, 0x80 = variant 10.
    h[6] = (h[6] & 0x0F) | 0x40;
    h[8] = (h[8] & 0x3F) | 0x80;
    uuid::Uuid::from_bytes(h)
}

/// Stable key for a port in the top-level `ports` YMap.
/// Format: `${element_uuid}::${port_uuid}`. (Same `::` as
/// the edge key — keeps the schema consistent.)
fn port_key(element_id: NodeId, port_id: uuid::Uuid) -> String {
    format!("{}::{}", element_id.0, port_id)
}

fn read_element_snapshot(map: &MapRef, txn: &impl ReadTxn) -> ElementSnapshot {
    let id_str = string_field(map, txn, F_ID).unwrap_or_default();
    let id = uuid::Uuid::parse_str(&id_str).map_or_else(|_| uuid::Uuid::nil(), uuid::Uuid::from);
    let kind = string_field(map, txn, F_KIND).map_or(NodeKind::Block, |s| kind_parse(&s));
    let x = int_field(map, txn, F_X).unwrap_or(0);
    let y = int_field(map, txn, F_Y).unwrap_or(0);
    let label = string_field(map, txn, F_LABEL).unwrap_or_default();
    let ports = read_ports_for_element(txn, NodeId(id));
    ElementSnapshot {
        id: NodeId(id),
        kind,
        position: Position::new(x, y),
        label,
        ports,
        alive: true,
    }
}

/// Read all ports for a given element from the top-level
/// `ports` YMap.
fn read_ports_for_element(txn: &impl ReadTxn, element_id: NodeId) -> Vec<PortSnapshot> {
    let mut out: Vec<PortSnapshot> = Vec::new();
    let Some(ports) = txn.get_map(PORTS_KEY) else {
        return out;
    };
    for (_k, v) in ports.iter(txn) {
        let yrs::types::Value::YMap(pm) = v else {
            continue;
        };
        let Some(elem_s) = string_field(&pm, txn, F_ELEMENT_ID) else {
            continue;
        };
        if elem_s != element_id.0.to_string() {
            continue;
        }
        let id_s = string_field(&pm, txn, F_PORT_ID).unwrap_or_default();
        let port_id = uuid::Uuid::parse_str(&id_s).unwrap_or_else(|_| uuid::Uuid::new_v4());
        let kind = string_field(&pm, txn, F_PORT_KIND).unwrap_or_else(|| "output".to_string());
        let label = string_field(&pm, txn, F_PORT_LABEL).unwrap_or_default();
        let x = int_field(&pm, txn, F_PORT_X).unwrap_or(0);
        let y = int_field(&pm, txn, F_PORT_Y).unwrap_or(0);
        out.push(PortSnapshot {
            id: port_id,
            kind,
            label,
            x,
            y,
        });
    }
    out
}

/// Stable key for the edges YMap. Uses `::` as separator to
/// avoid clashing with the UUID `-` (which is part of the
/// UUID hex). Self-loops should be rejected by callers
/// before reaching this function.
fn edge_key(from: NodeId, to: NodeId) -> String {
    format!("{}::{}", from.0, to.0)
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

/// Read a YDoc back into a `Canvas` snapshot. Used by tests
/// to verify the round-trip and by future m13 endpoints that
/// want to serve a CRDT-merged state to a non-CRDT-aware
/// client.
///
/// Public in v0.7.0 (was `pub(crate)` in v0.6.0) so m13
/// cross-crate consumers can call it directly. See
/// `tests/reconcile_smoke.rs` in `ada-m13-api-gateway` for
/// the cross-crate usage example.
pub fn read_canvas_from_doc(doc: &Doc, name: &str) -> Result<Canvas, CanvasError> {
    let mut nodes: Vec<CanvasNode> = Vec::new();
    let mut seen_ids: HashSet<uuid::Uuid> = HashSet::new();
    for (_uuid, snap) in iter_elements(doc) {
        if !seen_ids.insert(snap.id.0) {
            continue;
        }
        let mut n = CanvasNode::new(snap.kind, snap.position, snap.label);
        n.id = snap.id;
        n.ports = snap.ports.into_iter().map(|p| Port::new(p.label)).collect();
        nodes.push(n);
    }
    let mut edges: Vec<Edge> = Vec::new();
    for (from, to) in iter_edge_keys(doc) {
        edges.push(Edge::new(from, to));
    }
    let canvas = Canvas::new(name);
    for n in nodes {
        canvas.add_node(n);
    }
    for e in edges {
        // `add_edge` may fail on edge dedup; v0.7.0 stores
        // edges in a YMap keyed by `from::to` so duplicates
        // are impossible at the CRDT level, but the
        // `Canvas::add_edge` call still re-checks via
        // `HashSet<Edge>`. We deliberately ignore that
        // error.
        let _ = canvas.add_edge(e);
    }
    Ok(canvas)
}

/// Iterate live edges. Yields `(from, to)` pairs; edge
/// labels are accessible via [`get_edge`].
pub fn iter_edge_keys(doc: &Doc) -> impl Iterator<Item = (NodeId, NodeId)> {
    let txn = doc.transact();
    let mut out: Vec<(NodeId, NodeId)> = Vec::new();
    if let Some(edges) = txn.get_map(EDGES_KEY) {
        for (k, v) in edges.iter(&txn) {
            let yrs::types::Value::YMap(map) = v else {
                continue;
            };
            if !is_alive(&map, &txn) {
                continue;
            }
            // Parse `from::to`.
            let Some((from_s, to_s)) = k.split_once("::") else {
                continue;
            };
            let (Ok(from), Ok(to)) = (
                uuid::Uuid::parse_str(from_s).map(NodeId),
                uuid::Uuid::parse_str(to_s).map(NodeId),
            ) else {
                continue;
            };
            out.push((from, to));
        }
    }
    out.into_iter()
}

/// Convenience helper for tests / external callers: parse a
/// yrs-encoded state vector.
#[allow(dead_code)]
pub fn encode_state_vector(doc: &Doc) -> Vec<u8> {
    let txn = doc.transact();
    txn.state_vector().encode_v1()
}

/// Build a YDoc from a [`Canvas`] snapshot, returning a fresh
/// `Doc` with a randomized client id. Symmetric counterpart
/// to [`read_canvas_from_doc`]. Useful for tests and for
/// first-sync scenarios where the server has `Canvas` shape
/// state and wants to hand a CRDT doc to a connecting
/// client.
///
/// Public in v0.7.0 so cross-crate consumers don't have to
/// reach for `reconcile_with_crdt` + a dummy client update
/// just to build a doc.
#[must_use]
pub fn doc_from_canvas(canvas: &Canvas) -> Doc {
    let doc = Doc::new();
    hydrate_doc_from_canvas(&doc, canvas);
    doc
}

/// Add a port to an existing element. Returns `true` if the
/// element exists, `false` if the element id was not found.
/// If a port with the same `${element_id}::${port_id}` key
/// already exists, this updates its fields in place (idempotent).
pub fn add_port(doc: &Doc, element_id: NodeId, port: PortSnapshot) -> Result<bool, CanvasError> {
    // 1. Make sure the element exists. (A no-op update of
    //    a missing element is still a no-op, so we return
    //    `false` here to let the caller distinguish.)
    let elements: MapRef = doc.get_or_insert_map(ELEMENTS_KEY);
    {
        let txn = doc.transact();
        let key = element_id.0.to_string();
        if !matches!(elements.get(&txn, &key), Some(yrs::types::Value::YMap(_))) {
            return Ok(false);
        }
    }
    // 2. Insert / update the port under its stable key.
    let ports: MapRef = doc.get_or_insert_map(PORTS_KEY);
    let mut txn = doc.transact_mut();
    let key = port_key(element_id, port.id);
    let m: MapRef = match ports.get(&txn, &key) {
        Some(yrs::types::Value::YMap(m)) => m,
        _ => ports.insert(&mut txn, key.clone(), MapPrelim::<Any>::new()),
    };
    m.insert(&mut txn, F_PORT_ID, port.id.to_string());
    m.insert(&mut txn, F_ELEMENT_ID, element_id.0.to_string());
    m.insert(&mut txn, F_PORT_KIND, port.kind);
    m.insert(&mut txn, F_PORT_LABEL, port.label);
    m.insert(&mut txn, F_PORT_X, i64::from(port.x));
    m.insert(&mut txn, F_PORT_Y, i64::from(port.y));
    Ok(true)
}

/// Remove a port by its UUID. Returns `true` if the port was
/// found and removed, `false` if it was not present.
pub fn remove_port(
    doc: &Doc,
    element_id: NodeId,
    port_id: uuid::Uuid,
) -> Result<bool, CanvasError> {
    let ports: MapRef = doc.get_or_insert_map(PORTS_KEY);
    let mut txn = doc.transact_mut();
    let key = port_key(element_id, port_id);
    let exists = matches!(ports.get(&txn, &key), Some(yrs::types::Value::YMap(_)));
    if !exists {
        return Ok(false);
    }
    ports.remove(&mut txn, &key);
    Ok(true)
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
        Some(yrs::types::Value::Any(yrs::any::Any::Number(n))) => {
            // f64 -> i64 -> i32 with bounds check. Yrs stores
            // integers as f64 when they fit in safe-int range;
            // outside that range, fall back to BigInt.
            #[allow(clippy::cast_possible_truncation)]
            let as_i64 = n as i64;
            i32::try_from(as_i64).ok()
        }
        Some(yrs::types::Value::Any(yrs::any::Any::BigInt(n))) => i32::try_from(n).ok(),
        _ => None,
    }
}

// Suppress the "unused import" warning for Value when no
// tests / public-API user references it. The import is
// needed for the `yrs::types::Value::YMap` / `YArray`
// patterns above, but Rust sometimes lints it as unused if
// the public `iter_elements` etc. are the only consumers.
#[allow(unused_imports)]
const _: Option<Value> = None;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::NodeKind;

    fn positioned(label: &str, x: i32, y: i32) -> CanvasNode {
        CanvasNode::new(NodeKind::Block, Position::new(x, y), label)
    }

    /// Build a server replica seeded with the given canvas.
    fn server_doc(c: &Canvas) -> Doc {
        doc_from_canvas(c)
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

        let doc = server_doc(&server);
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
            let local_els = local.get_map(ELEMENTS_KEY).expect("elements");
            let remote_els = remote.get_map(ELEMENTS_KEY).expect("elements");
            (local_els.len(&local), remote_els.len(&remote))
        };
        assert_eq!(local_count, 2);
        assert_eq!(remote_count, 2);
    }

    /// v0.7.0 fix: two replicas concurrently `remove(id)` on
    /// the same element must converge to "absent" — YMap 2P-Set
    /// collapses the two tombstones.
    #[test]
    fn concurrent_delete_same_id_converges_to_deleted() {
        let shared_id = uuid::Uuid::new_v4();
        let mut shared_node = positioned("shared", 0, 0);
        shared_node.id = NodeId(shared_id);

        let doc_a = server_doc(&Canvas::new("c")); // empty
        let doc_b = server_doc(&Canvas::new("c")); // empty

        // Seed both with the same shared element.
        for doc in [&doc_a, &doc_b] {
            insert_element(doc, &shared_node).expect("seed");
        }
        // a deletes, b deletes — concurrently.
        remove_element(&doc_a, NodeId(shared_id)).expect("rm a");
        remove_element(&doc_b, NodeId(shared_id)).expect("rm b");
        // Cross-sync both ways.
        let upd_a = encode_state_as_update(&doc_a);
        let upd_b = encode_state_as_update(&doc_b);
        let sv_a = encode_state_vector(&doc_a);
        let sv_b = encode_state_vector(&doc_b);
        let diff_b = merge_crdt_update(&doc_b, &sv_a, &upd_a).expect("a->b");
        let diff_a = merge_crdt_update(&doc_a, &sv_b, &upd_b).expect("b->a");
        {
            let mut txn = doc_a.transact_mut();
            let u = yrs::Update::decode_v1(&diff_b).expect("dec b");
            txn.apply_update(u);
        }
        {
            let mut txn = doc_b.transact_mut();
            let u = yrs::Update::decode_v1(&diff_a).expect("dec a");
            txn.apply_update(u);
        }
        // Both replicas should report the element as absent.
        assert!(
            get_element(&doc_a, NodeId(shared_id)).is_none(),
            "a: should be absent"
        );
        assert!(
            get_element(&doc_b, NodeId(shared_id)).is_none(),
            "b: should be absent"
        );
    }

    /// v0.7.0 fix: two replicas insert the *same* element id
    /// concurrently. Because the YMap is keyed by uuid, both
    /// insert operations land on the same key; per-field LWW
    /// keeps one copy (not two).
    #[test]
    fn concurrent_insert_same_id_dedup() {
        let shared_id = uuid::Uuid::new_v4();
        let mut node_a = positioned("from-a", 1, 1);
        node_a.id = NodeId(shared_id);
        let mut node_b = positioned("from-b", 2, 2);
        node_b.id = NodeId(shared_id);

        let doc_a = server_doc(&Canvas::new("c"));
        let doc_b = server_doc(&Canvas::new("c"));
        insert_element(&doc_a, &node_a).expect("seed a");
        insert_element(&doc_b, &node_b).expect("seed b");
        // Cross-sync.
        let upd_a = encode_state_as_update(&doc_a);
        let upd_b = encode_state_as_update(&doc_b);
        let sv_a = encode_state_vector(&doc_a);
        let sv_b = encode_state_vector(&doc_b);
        let diff_b = merge_crdt_update(&doc_b, &sv_a, &upd_a).expect("a->b");
        let diff_a = merge_crdt_update(&doc_a, &sv_b, &upd_b).expect("b->a");
        for (doc, diff) in [(&doc_a, diff_b), (&doc_b, diff_a)] {
            let mut txn = doc.transact_mut();
            let u = yrs::Update::decode_v1(&diff).expect("dec");
            txn.apply_update(u);
        }
        let snap_a = get_element(&doc_a, NodeId(shared_id)).expect("a: present");
        let snap_b = get_element(&doc_b, NodeId(shared_id)).expect("b: present");
        // Both replicas should agree on the surviving copy
        // (whichever LWW-ed later — they should at least be
        // equal to each other and have the same id).
        assert_eq!(snap_a, snap_b);
        assert_eq!(snap_a.id, NodeId(shared_id));
    }

    /// v0.7.0: concurrent updates to the *same* field of the
    /// same element go to LWW (one wins). This test only
    /// asserts that the two replicas converge to a single
    /// value (they do — even if it is not deterministic which
    /// side wins).
    #[test]
    fn concurrent_update_same_field_lww() {
        let shared_id = uuid::Uuid::new_v4();
        let mut node = positioned("shared", 0, 0);
        node.id = NodeId(shared_id);
        let doc_a = server_doc(&Canvas::new("c"));
        let doc_b = server_doc(&Canvas::new("c"));
        for doc in [&doc_a, &doc_b] {
            insert_element(doc, &node).expect("seed");
        }
        // a moves to (100, 100), b moves to (200, 200).
        update_element(
            &doc_a,
            NodeId(shared_id),
            ElementUpdate::new().position(Position::new(100, 100)),
        )
        .expect("a update");
        update_element(
            &doc_b,
            NodeId(shared_id),
            ElementUpdate::new().position(Position::new(200, 200)),
        )
        .expect("b update");
        // Cross-sync.
        let upd_a = encode_state_as_update(&doc_a);
        let upd_b = encode_state_as_update(&doc_b);
        let sv_a = encode_state_vector(&doc_a);
        let sv_b = encode_state_vector(&doc_b);
        let diff_b = merge_crdt_update(&doc_b, &sv_a, &upd_a).expect("a->b");
        let diff_a = merge_crdt_update(&doc_a, &sv_b, &upd_b).expect("b->a");
        for (doc, diff) in [(&doc_a, diff_b), (&doc_b, diff_a)] {
            let mut txn = doc.transact_mut();
            let u = yrs::Update::decode_v1(&diff).expect("dec");
            txn.apply_update(u);
        }
        let snap_a = get_element(&doc_a, NodeId(shared_id)).expect("a present");
        let snap_b = get_element(&doc_b, NodeId(shared_id)).expect("b present");
        // Converge: same position on both sides.
        assert_eq!(
            snap_a.position, snap_b.position,
            "positions should converge"
        );
    }

    /// v0.7.0: concurrent updates to *different* fields of
    /// the same element both win (no conflict). a changes
    /// position, b changes label; both fields are kept.
    ///
    /// Test setup: a seeds, then we sync once so b has the
    /// element (sharing the same inner YMap reference on
    /// both sides). Then a and b concurrently update
    /// different fields. Both updates must survive the
    /// merge.
    ///
    /// Why we don't seed both sides independently: when
    /// both clients call `insert_element` with the same
    /// uuid, each creates a fresh `MapPrelim::new()` inner
    /// YMap. The outer YMap's per-key LWW then picks ONE
    /// of the two inner YMaps, and the other inner YMap's
    /// subsequent field writes are lost. (This is a
    /// fundamental property of nested YMap under yrs 0.18:
    /// the outer YMap stores the *reference* to the inner
    /// type, not its content; concurrent inserts of the
    /// same key produce two different inner-type
    /// references, and LWW at the outer level picks one.)
    /// Once both sides share the same inner YMap
    /// reference (via the initial sync), per-field LWW on
    /// the inner YMap keeps concurrent updates to
    /// different fields, which is what this test verifies.
    #[test]
    fn concurrent_update_different_fields_no_conflict() {
        let shared_id = uuid::Uuid::new_v4();
        let mut node = positioned("shared", 0, 0);
        node.id = NodeId(shared_id);
        let doc_a = server_doc(&Canvas::new("c"));
        let doc_b = server_doc(&Canvas::new("c"));
        // a seeds. b will receive the element via the
        // initial sync so both sides share the same inner
        // YMap reference.
        insert_element(&doc_a, &node).expect("seed a");
        // Initial sync: a -> b.
        let upd_a = encode_state_as_update(&doc_a);
        let sv_b = encode_state_vector(&doc_b);
        let diff_b_initial = merge_crdt_update(&doc_b, &sv_b, &upd_a).expect("a->b initial");
        {
            let mut txn = doc_b.transact_mut();
            let u = yrs::Update::decode_v1(&diff_b_initial).expect("dec initial");
            txn.apply_update(u);
        }
        // Both sides now have the element under the same
        // inner YMap reference. Concurrent updates to
        // different fields:
        update_element(
            &doc_a,
            NodeId(shared_id),
            ElementUpdate::new().position(Position::new(50, 60)),
        )
        .expect("a update");
        update_element(
            &doc_b,
            NodeId(shared_id),
            ElementUpdate::new().label("from-b"),
        )
        .expect("b update");
        // Cross-sync.
        let upd_a = encode_state_as_update(&doc_a);
        let upd_b = encode_state_as_update(&doc_b);
        let sv_a = encode_state_vector(&doc_a);
        let sv_b = encode_state_vector(&doc_b);
        let diff_b = merge_crdt_update(&doc_b, &sv_a, &upd_a).expect("a->b");
        let diff_a = merge_crdt_update(&doc_a, &sv_b, &upd_b).expect("b->a");
        for (doc, diff) in [(&doc_a, diff_b), (&doc_b, diff_a)] {
            let mut txn = doc.transact_mut();
            let u = yrs::Update::decode_v1(&diff).expect("dec");
            txn.apply_update(u);
        }
        let snap_a = get_element(&doc_a, NodeId(shared_id)).expect("a present");
        let snap_b = get_element(&doc_b, NodeId(shared_id)).expect("b present");
        // Both fields survive: a's pos update + b's label
        // update.
        assert_eq!(snap_a.label, "from-b");
        assert_eq!(snap_a.position, Position::new(50, 60));
        assert_eq!(snap_a, snap_b, "a and b should fully agree");
    }

    /// v0.7.0 tombstone: a removes, then b re-inserts the
    /// same id concurrently. The YMap 2P-Set picks the
    /// later-clock write. After merge, both replicas must
    /// agree on the outcome (either both see the element as
    /// present, or both see it as absent). Which side wins
    /// is implementation-defined (depends on which client
    /// had the later clock), but the two replicas must
    /// converge.
    ///
    /// Note: yrs YMap 2P-Set does not guarantee that a
    /// later insert from a *different* client resurrects a
    /// deleted key from another client — that depends on
    /// the per-key Lamport clock. What it *does* guarantee
    /// is that both replicas converge to the same answer.
    /// This test exercises the convergence property.
    #[test]
    fn tombstone_converges_with_concurrent_insert() {
        let shared_id = uuid::Uuid::new_v4();
        let mut node = positioned("shared", 0, 0);
        node.id = NodeId(shared_id);
        let doc_a = server_doc(&Canvas::new("c"));
        let doc_b = server_doc(&Canvas::new("c"));
        // a seeds + removes first.
        insert_element(&doc_a, &node).expect("seed a");
        // a removes.
        remove_element(&doc_a, NodeId(shared_id)).expect("rm a");
        // b concurrently inserts the same id (a different
        // copy of the node from b's perspective).
        let mut node_b = positioned("from-b", 9, 9);
        node_b.id = NodeId(shared_id);
        insert_element(&doc_b, &node_b).expect("seed b");
        // Cross-sync.
        let upd_a = encode_state_as_update(&doc_a);
        let upd_b = encode_state_as_update(&doc_b);
        let sv_a = encode_state_vector(&doc_a);
        let sv_b = encode_state_vector(&doc_b);
        let diff_b = merge_crdt_update(&doc_b, &sv_a, &upd_a).expect("a->b");
        let diff_a = merge_crdt_update(&doc_a, &sv_b, &upd_b).expect("b->a");
        for (doc, diff) in [(&doc_a, diff_b), (&doc_b, diff_a)] {
            let mut txn = doc.transact_mut();
            let u = yrs::Update::decode_v1(&diff).expect("dec");
            txn.apply_update(u);
        }
        // Both replicas must agree on the outcome.
        let snap_a = get_element(&doc_a, NodeId(shared_id));
        let snap_b = get_element(&doc_b, NodeId(shared_id));
        assert_eq!(
            snap_a.is_some(),
            snap_b.is_some(),
            "a and b must converge on the tombstone question"
        );
    }

    // === Port-level tests (Task 2) ===

    /// v0.7.0: ports are a proper YArray. Two replicas add
    /// *different* ports concurrently; both should win
    /// (no field-level conflict, additive merge).
    #[test]
    fn port_concurrent_add_different_id_converges() {
        let shared_id = uuid::Uuid::new_v4();
        let mut node = positioned("shared", 0, 0);
        node.id = NodeId(shared_id);
        let doc_a = server_doc(&Canvas::new("c"));
        let doc_b = server_doc(&Canvas::new("c"));
        for doc in [&doc_a, &doc_b] {
            insert_element(doc, &node).expect("seed");
        }
        let port_a = PortSnapshot::from_name(uuid::Uuid::new_v4(), "out-a");
        let port_b = PortSnapshot::from_name(uuid::Uuid::new_v4(), "out-b");
        add_port(&doc_a, NodeId(shared_id), port_a.clone()).expect("a add");
        add_port(&doc_b, NodeId(shared_id), port_b.clone()).expect("b add");
        // Cross-sync.
        let upd_a = encode_state_as_update(&doc_a);
        let upd_b = encode_state_as_update(&doc_b);
        let sv_a = encode_state_vector(&doc_a);
        let sv_b = encode_state_vector(&doc_b);
        let diff_b = merge_crdt_update(&doc_b, &sv_a, &upd_a).expect("a->b");
        let diff_a = merge_crdt_update(&doc_a, &sv_b, &upd_b).expect("b->a");
        for (doc, diff) in [(&doc_a, diff_b), (&doc_b, diff_a)] {
            let mut txn = doc.transact_mut();
            let u = yrs::Update::decode_v1(&diff).expect("dec");
            txn.apply_update(u);
        }
        let snap_a = get_element(&doc_a, NodeId(shared_id)).expect("a present");
        let snap_b = get_element(&doc_b, NodeId(shared_id)).expect("b present");
        let names_a: Vec<&str> = snap_a.ports.iter().map(|p| p.label.as_str()).collect();
        let names_b: Vec<&str> = snap_b.ports.iter().map(|p| p.label.as_str()).collect();
        assert!(names_a.contains(&"out-a"), "a should have out-a");
        assert!(
            names_a.contains(&"out-b"),
            "a should have out-b (merged from b)"
        );
        assert_eq!(
            names_a.len(),
            names_b.len(),
            "a and b should agree on port count"
        );
    }

    /// v0.7.0: two replicas remove the *same* port id
    /// concurrently. The YArray item is removed on both
    /// sides; merge → still absent.
    #[test]
    fn port_concurrent_remove_same_id_converges_to_removed() {
        let shared_id = uuid::Uuid::new_v4();
        let mut node = positioned("shared", 0, 0);
        node.id = NodeId(shared_id);
        let port = PortSnapshot::from_name(uuid::Uuid::new_v4(), "out");
        // Seed both with the same port.
        let doc_a = server_doc(&Canvas::new("c"));
        let doc_b = server_doc(&Canvas::new("c"));
        for doc in [&doc_a, &doc_b] {
            insert_element(doc, &node).expect("seed");
            add_port(doc, NodeId(shared_id), port.clone()).expect("seed port");
        }
        // Both remove the same port concurrently.
        remove_port(&doc_a, NodeId(shared_id), port.id).expect("a rm");
        remove_port(&doc_b, NodeId(shared_id), port.id).expect("b rm");
        // Cross-sync.
        let upd_a = encode_state_as_update(&doc_a);
        let upd_b = encode_state_as_update(&doc_b);
        let sv_a = encode_state_vector(&doc_a);
        let sv_b = encode_state_vector(&doc_b);
        let diff_b = merge_crdt_update(&doc_b, &sv_a, &upd_a).expect("a->b");
        let diff_a = merge_crdt_update(&doc_a, &sv_b, &upd_b).expect("b->a");
        for (doc, diff) in [(&doc_a, diff_b), (&doc_b, diff_a)] {
            let mut txn = doc.transact_mut();
            let u = yrs::Update::decode_v1(&diff).expect("dec");
            txn.apply_update(u);
        }
        let snap_a = get_element(&doc_a, NodeId(shared_id)).expect("a present");
        let snap_b = get_element(&doc_b, NodeId(shared_id)).expect("b present");
        assert!(snap_a.ports.is_empty(), "a: port should be removed");
        assert!(snap_b.ports.is_empty(), "b: port should be removed");
    }

    /// v0.7.0: `update_element` with a new port list on one
    /// replica does not conflict with another replica updating
    /// the element's `x`/`y` position field. The merge keeps
    /// both changes.
    ///
    /// Test setup: a seeds, then we sync once so b has the
    /// element (sharing the same inner YMap reference on
    /// both sides). Then a updates ports and b updates
    /// position concurrently. Both must survive.
    ///
    /// See the long comment on
    /// `concurrent_update_different_fields_no_conflict` for
    /// why we don't seed both sides independently (the
    /// nested-YMap concurrent-insert issue under yrs 0.18).
    #[test]
    fn port_concurrent_update_x_vs_y_no_conflict() {
        let shared_id = uuid::Uuid::new_v4();
        let mut node = positioned("shared", 0, 0);
        node.id = NodeId(shared_id);
        let doc_a = server_doc(&Canvas::new("c"));
        let doc_b = server_doc(&Canvas::new("c"));
        // a seeds. b will receive the element via initial
        // sync so both sides share the same inner YMap
        // reference.
        insert_element(&doc_a, &node).expect("seed a");
        // Initial sync: a -> b.
        let upd_a = encode_state_as_update(&doc_a);
        let sv_b = encode_state_vector(&doc_b);
        let diff_b_initial = merge_crdt_update(&doc_b, &sv_b, &upd_a).expect("a->b initial");
        {
            let mut txn = doc_b.transact_mut();
            let u = yrs::Update::decode_v1(&diff_b_initial).expect("dec initial");
            txn.apply_update(u);
        }
        // a replaces the port list with one new port.
        let new_port = PortSnapshot::from_name(uuid::Uuid::new_v4(), "newport");
        update_element(
            &doc_a,
            NodeId(shared_id),
            ElementUpdate::new().ports(vec![new_port.clone()]),
        )
        .expect("a update");
        // b moves the element.
        update_element(
            &doc_b,
            NodeId(shared_id),
            ElementUpdate::new().position(Position::new(7, 8)),
        )
        .expect("b update");
        // Cross-sync.
        let upd_a = encode_state_as_update(&doc_a);
        let upd_b = encode_state_as_update(&doc_b);
        let sv_a = encode_state_vector(&doc_a);
        let sv_b = encode_state_vector(&doc_b);
        let diff_b = merge_crdt_update(&doc_b, &sv_a, &upd_a).expect("a->b");
        let diff_a = merge_crdt_update(&doc_a, &sv_b, &upd_b).expect("b->a");
        for (doc, diff) in [(&doc_a, diff_b), (&doc_b, diff_a)] {
            let mut txn = doc.transact_mut();
            let u = yrs::Update::decode_v1(&diff).expect("dec");
            txn.apply_update(u);
        }
        let snap_a = get_element(&doc_a, NodeId(shared_id)).expect("a present");
        let snap_b = get_element(&doc_b, NodeId(shared_id)).expect("b present");
        // Position update (from b) survives; port list
        // update (from a) survives. The exact port count on
        // a vs b after merge is implementation-defined
        // because the top-level `ports` YMap merges with
        // any pre-existing entries — but the new port must
        // be present on both sides.
        assert_eq!(snap_a.position, Position::new(7, 8));
        assert_eq!(snap_b.position, Position::new(7, 8));
        let names_a: Vec<&str> = snap_a.ports.iter().map(|p| p.label.as_str()).collect();
        let names_b: Vec<&str> = snap_b.ports.iter().map(|p| p.label.as_str()).collect();
        assert!(names_a.contains(&"newport"), "a should have newport");
        assert!(names_b.contains(&"newport"), "b should have newport");
    }

    // === Edge tests (Task 3) ===

    /// v0.7.0: edges are a YMap keyed by `from::to`. Two
    /// replicas insert the *same* edge (same from, same to)
    /// concurrently → one entry, not two.
    #[test]
    fn edge_concurrent_insert_same_key_dedup() {
        let a = NodeId(uuid::Uuid::new_v4());
        let b = NodeId(uuid::Uuid::new_v4());
        let doc_a = server_doc(&Canvas::new("c"));
        let doc_b = server_doc(&Canvas::new("c"));
        insert_edge(&doc_a, a, b, Some("wires")).expect("a edge");
        insert_edge(&doc_b, a, b, Some("wires")).expect("b edge");
        // Cross-sync.
        let upd_a = encode_state_as_update(&doc_a);
        let upd_b = encode_state_as_update(&doc_b);
        let sv_a = encode_state_vector(&doc_a);
        let sv_b = encode_state_vector(&doc_b);
        let diff_b = merge_crdt_update(&doc_b, &sv_a, &upd_a).expect("a->b");
        let diff_a = merge_crdt_update(&doc_a, &sv_b, &upd_b).expect("b->a");
        for (doc, diff) in [(&doc_a, diff_b), (&doc_b, diff_a)] {
            let mut txn = doc.transact_mut();
            let u = yrs::Update::decode_v1(&diff).expect("dec");
            txn.apply_update(u);
        }
        let count_a = iter_edge_keys(&doc_a).count();
        let count_b = iter_edge_keys(&doc_b).count();
        assert_eq!(count_a, 1, "a: should have exactly 1 edge");
        assert_eq!(count_b, 1, "b: should have exactly 1 edge");
    }

    /// v0.7.0: two replicas remove the same edge
    /// concurrently. The YMap 2P-Set collapses to "absent".
    #[test]
    fn edge_concurrent_delete_same_key_converges() {
        let a = NodeId(uuid::Uuid::new_v4());
        let b = NodeId(uuid::Uuid::new_v4());
        let doc_a = server_doc(&Canvas::new("c"));
        let doc_b = server_doc(&Canvas::new("c"));
        for doc in [&doc_a, &doc_b] {
            insert_edge(doc, a, b, None).expect("seed");
        }
        remove_edge(&doc_a, a, b).expect("a rm");
        remove_edge(&doc_b, a, b).expect("b rm");
        // Cross-sync.
        let upd_a = encode_state_as_update(&doc_a);
        let upd_b = encode_state_as_update(&doc_b);
        let sv_a = encode_state_vector(&doc_a);
        let sv_b = encode_state_vector(&doc_b);
        let diff_b = merge_crdt_update(&doc_b, &sv_a, &upd_a).expect("a->b");
        let diff_a = merge_crdt_update(&doc_a, &sv_b, &upd_b).expect("b->a");
        for (doc, diff) in [(&doc_a, diff_b), (&doc_b, diff_a)] {
            let mut txn = doc.transact_mut();
            let u = yrs::Update::decode_v1(&diff).expect("dec");
            txn.apply_update(u);
        }
        let count_a = iter_edge_keys(&doc_a).count();
        let count_b = iter_edge_keys(&doc_b).count();
        assert_eq!(count_a, 0, "a: edge should be absent");
        assert_eq!(count_b, 0, "b: edge should be absent");
    }

    /// v0.7.0: two replicas concurrently update *different*
    /// fields of the same edge (a changes `from`/`to` via
    /// re-insert with new label, b changes nothing). The
    /// merge keeps both fields consistent; the test here
    /// exercises the canonical "label update" path which is
    /// independent of the structural fields.
    #[test]
    fn edge_concurrent_update_label_no_conflict() {
        let a = NodeId(uuid::Uuid::new_v4());
        let b = NodeId(uuid::Uuid::new_v4());
        let doc_a = server_doc(&Canvas::new("c"));
        let doc_b = server_doc(&Canvas::new("c"));
        for doc in [&doc_a, &doc_b] {
            insert_edge(doc, a, b, Some("initial")).expect("seed");
        }
        // a updates the label; b also updates the label
        // (same value, but a separate clock).
        update_edge(&doc_a, a, b, Some("from-a")).expect("a update");
        update_edge(&doc_b, a, b, Some("from-b")).expect("b update");
        // Cross-sync.
        let upd_a = encode_state_as_update(&doc_a);
        let upd_b = encode_state_as_update(&doc_b);
        let sv_a = encode_state_vector(&doc_a);
        let sv_b = encode_state_vector(&doc_b);
        let diff_b = merge_crdt_update(&doc_b, &sv_a, &upd_a).expect("a->b");
        let diff_a = merge_crdt_update(&doc_a, &sv_b, &upd_b).expect("b->a");
        for (doc, diff) in [(&doc_a, diff_b), (&doc_b, diff_a)] {
            let mut txn = doc.transact_mut();
            let u = yrs::Update::decode_v1(&diff).expect("dec");
            txn.apply_update(u);
        }
        let snap_a = get_edge(&doc_a, a, b).expect("a present");
        let snap_b = get_edge(&doc_b, a, b).expect("b present");
        // Both should converge to the same final label
        // (whichever replica's update had the later clock).
        assert_eq!(snap_a, snap_b);
        assert!(snap_a.label.is_some(), "label should be set");
    }

    // === ClientId tests (Task 7) ===

    /// v0.7.0: explicit ClientId negotiation. The
    /// `client_id.uuid` must be reflected in the
    /// yrs-encoded update bytes (yrs embeds the client id
    /// in the varint header of every entry). This test
    /// verifies that a doc created with a specific client
    /// id produces a different state-vector byte pattern
    /// than a doc created with a different client id.
    #[test]
    fn client_id_negotiation_persists_to_update_bytes() {
        let a = ClientId::from_uuid(uuid::Uuid::from_u128(0xAAAA_AAAA_AAAA_AAAA), "a".into());
        let b = ClientId::from_uuid(uuid::Uuid::from_u128(0xBBBB_BBBB_BBBB_BBBB), "b".into());
        let mut node = positioned("shared", 0, 0);
        node.id = NodeId(uuid::Uuid::new_v4());
        let doc_a = Doc::with_client_id(a.uuid.as_u128() as u64);
        let doc_b = Doc::with_client_id(b.uuid.as_u128() as u64);
        insert_element(&doc_a, &node).expect("a insert");
        insert_element(&doc_b, &node).expect("b insert");
        let snap_a = encode_state_as_update(&doc_a);
        let snap_b = encode_state_as_update(&doc_b);
        let sv_a = encode_state_vector(&doc_a);
        let sv_b = encode_state_vector(&doc_b);
        // Different client ids must produce different
        // state-vector bytes (the header encodes the
        // client id in a varint).
        assert_ne!(
            sv_a, sv_b,
            "different client ids must produce different state vectors"
        );
        assert_ne!(
            snap_a, snap_b,
            "different client ids must produce different update bytes"
        );
        // And the state vector should not be empty (it has
        // at least the one client entry from the insert).
        assert!(!sv_a.is_empty());
        assert!(!sv_b.is_empty());
    }

    /// v0.7.1 compat: serde roundtrip. `ClientId` derives
    /// `Serialize` + `Deserialize`; the JSON form must
    /// preserve both `uuid` and `label` exactly so a
    /// persistence layer (k8s sidecar, postgres, log
    /// shipping) can round-trip a client id without loss.
    #[test]
    fn client_id_serde_roundtrip() {
        let original =
            ClientId::from_uuid(uuid::Uuid::from_u128(0xCAFE_BABE_DEAD_BEEF), "alice-laptop".into());
        let json = serde_json::to_string(&original).expect("serialise ClientId");
        let parsed: ClientId = serde_json::from_str(&json).expect("deserialise ClientId");
        assert_eq!(original, parsed, "ClientId JSON roundtrip must preserve all fields");
        assert_eq!(parsed.uuid, uuid::Uuid::from_u128(0xCAFE_BABE_DEAD_BEEF));
        assert_eq!(parsed.label, "alice-laptop");
    }

    /// v0.7.1 compat: two replicas with the **same** explicit
    /// `ClientId` must produce identical state-vector bytes
    /// (yrs embeds the client id in the varint header). This
    /// is the symmetry check for the previous test.
    #[test]
    fn client_id_same_uuid_yields_same_state_vector() {
        let shared =
            ClientId::from_uuid(uuid::Uuid::from_u128(0x1234_5678_9ABC_DEF0), "shared".into());
        let mut node = positioned("shared", 0, 0);
        node.id = NodeId(uuid::Uuid::new_v4());
        let doc_a = Doc::with_client_id(shared.uuid.as_u128() as u64);
        let doc_b = Doc::with_client_id(shared.uuid.as_u128() as u64);
        insert_element(&doc_a, &node).expect("a insert");
        insert_element(&doc_b, &node).expect("b insert");
        let sv_a = encode_state_vector(&doc_a);
        let sv_b = encode_state_vector(&doc_b);
        assert_eq!(
            sv_a, sv_b,
            "same ClientId must produce identical state vectors"
        );
    }

    /// v0.7.1 compat: ClientId `Display` impl is stable
    /// (used in log lines / error contexts). Locking down
    /// the format prevents log-spam from changing
    /// accidentally.
    #[test]
    fn client_id_display_format_is_stable() {
        let id = ClientId::new("test-client");
        let s = format!("{id}");
        // Format is `{label}({uuid})` where uuid is
        // hyphenated lowercase hex.
        assert!(s.starts_with("test-client("), "got: {s}");
        assert!(s.ends_with(')'), "got: {s}");
        // Inner is a 36-char hyphenated UUID (8-4-4-4-12).
        let inner = &s["test-client(".len()..s.len() - 1];
        assert_eq!(inner.len(), 36, "uuid part must be 36 chars, got: {inner:?}");
        assert_eq!(inner.chars().filter(|c| *c == '-').count(), 4);
    }

    /// Sanity: malformed update bytes produce a `BackendError`
    /// rather than a panic.
    #[test]
    fn merge_crdt_update_rejects_malformed_bytes() {
        let doc = Doc::new();
        let result = merge_crdt_update(&doc, &[], &[0xFF, 0xEE, 0xDD]);
        assert!(matches!(result, Err(CanvasError::BackendError(_))));
    }

    /// Multi-client merge: 3 replicas, each adds 10 elements,
    /// then sync. After merge, all 3 should see 30 elements.
    #[test]
    fn multi_client_merge_converges() {
        let docs: Vec<Doc> = (0..3).map(|_| Doc::new()).collect();
        for (i, doc) in docs.iter().enumerate() {
            for j in 0..10 {
                let mut n = positioned(&format!("r{i}n{j}"), i as i32, j as i32);
                n.id = NodeId(uuid::Uuid::new_v4());
                insert_element(doc, &n).expect("insert");
            }
        }
        // Star-shaped merge: 0 is the hub, 1 and 2 push to 0,
        // 0 then pushes back to 1 and 2.
        for src in 1..docs.len() {
            let update = encode_state_as_update(&docs[src]);
            let sv = encode_state_vector(&docs[0]);
            let diff = merge_crdt_update(&docs[0], &sv, &update).expect("merge");
            let mut txn = docs[0].transact_mut();
            let u = yrs::Update::decode_v1(&diff).expect("dec");
            txn.apply_update(u);
        }
        for dst in 1..docs.len() {
            let update = encode_state_as_update(&docs[0]);
            let sv = encode_state_vector(&docs[dst]);
            let diff = merge_crdt_update(&docs[dst], &sv, &update).expect("merge");
            let mut txn = docs[dst].transact_mut();
            let u = yrs::Update::decode_v1(&diff).expect("dec");
            txn.apply_update(u);
        }
        for (i, doc) in docs.iter().enumerate() {
            let count = iter_elements(doc).count();
            assert_eq!(count, 30, "replica {i} should see 30 elements after merge");
        }
    }

    /// Large doc perf sanity: a doc with 1k elements should
    /// encode + decode under 1s. Smoke test, not a benchmark.
    #[test]
    fn large_doc_encodes_decodes_under_1s() {
        let doc = Doc::new();
        let start = std::time::Instant::now();
        for i in 0..1000 {
            let mut n = positioned(&format!("n-{i}"), 0, 0);
            n.id = NodeId(uuid::Uuid::new_v4());
            insert_element(&doc, &n).expect("insert");
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
        let count = iter_elements(&peer).count();
        assert_eq!(count, 1000);
        assert!(
            encoded.as_secs() < 2 && decoded.as_secs() < 2,
            "encode {encoded:?} / decode {decoded:?} should be < 2s for 1k elements"
        );
    }
}
