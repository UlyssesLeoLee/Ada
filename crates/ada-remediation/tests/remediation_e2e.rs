//! End-to-end integration test for the auto-remediation engine.
//!
//! This test wires the engine, the in-memory history store,
//! and the HTTP server together without a real Postgres
//! instance. The persistent half (PostgreSQL
//! `remediation_history` + `remediation_cooldowns`) is covered
//! by `db/tests/V003*` (out of scope for the Rust crate).
//!
//! The full E2E flow is:
//!   1. Load runbooks from a temp dir.
//!   2. Build an `AlertmanagerPayload` for `DiskSpaceFillingFast`.
//!   3. POST it to the in-process axum router.
//!   4. Assert the response shape, the cooldown state, and the
//!      `/remediation/history` payload.
//!
//! Stubs to be expanded in commit 8 of the Phase 8 rollout.

use ada_remediation::{load_runbooks_from_dir, RemediationEngine};

#[test]
fn stub_loads_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    let actions = load_runbooks_from_dir(dir.path()).unwrap();
    assert!(actions.is_empty());
    let engine = RemediationEngine::with_runbooks(actions);
    let alert = ada_remediation::AlertEvent::new("anything");
    assert!(engine.evaluate(&alert).is_empty());
}
