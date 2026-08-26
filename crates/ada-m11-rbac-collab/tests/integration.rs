//! Integration tests for the v0.1.0 RBAC + collaboration surface.
//!
//! These tests exercise the public API end-to-end (no internal
//! access) to lock in the contract that downstream modules
//! (`ada-m13-api-gateway`, `ada-m14-module-registry`) will program
//! against.

use ada_m11_rbac_collab::{
    record_audit_log, role_permissions, Action, CollaborationMap, InMemoryAuditSink, LockKind,
    LockManager, Permission, RbacError, ResourceId, ResourceType, Role, UserId,
};
use std::sync::Arc;

fn user(n: u8) -> UserId {
    UserId(uuid::Uuid::from_bytes([n; 16]))
}

fn resource() -> ResourceId {
    ResourceId::new()
}

#[test]
fn end_to_end_rbac_grant_revoke_authorize() {
    let mut map = CollaborationMap::new();
    let r = resource();
    map.ensure(r);
    map.grant(r, user(1), Role::Admin).unwrap();
    map.grant(r, user(2), Role::Viewer).unwrap();

    // Admin can read+write+share_manage but NOT delete credentials.
    map.authorize(r, user(1), ResourceType::Credential, Action::Read)
        .expect("admin read credential");
    map.authorize(r, user(1), ResourceType::Canvas, Action::Write)
        .expect("admin write canvas");
    map.authorize(r, user(1), ResourceType::Credential, Action::ShareManage)
        .expect("admin share-manage credential");
    let err = map
        .authorize(r, user(1), ResourceType::Credential, Action::Delete)
        .expect_err("admin cannot delete credential");
    assert!(matches!(err, RbacError::InsufficientPermission { .. }));

    // Viewer can read but not write.
    map.authorize(r, user(2), ResourceType::Canvas, Action::Read)
        .expect("viewer read canvas");
    let err = map
        .authorize(r, user(2), ResourceType::Canvas, Action::Write)
        .expect_err("viewer cannot write canvas");
    assert!(matches!(err, RbacError::InsufficientPermission { .. }));

    // Revoke and verify the user can no longer act.
    map.revoke(r, user(2)).unwrap();
    let err = map
        .authorize(r, user(2), ResourceType::Canvas, Action::Read)
        .expect_err("revoked user has no permission");
    // After revoke, the user has no role, so `has_permission` returns
    // false and `authorize` returns `InsufficientPermission`.
    assert!(matches!(err, RbacError::InsufficientPermission { .. }));
}

#[test]
fn owner_can_delete_credential_but_admin_cannot() {
    let mut map = CollaborationMap::new();
    let r = resource();
    map.ensure(r);
    map.grant(r, user(1), Role::Owner).unwrap();
    map.grant(r, user(2), Role::Admin).unwrap();
    map.authorize(r, user(1), ResourceType::Credential, Action::Delete)
        .expect("owner can delete credential");
    let err = map
        .authorize(r, user(2), ResourceType::Credential, Action::Delete)
        .expect_err("admin cannot delete credential");
    assert!(matches!(err, RbacError::InsufficientPermission { .. }));
}

#[test]
fn role_matrix_is_stable() {
    // Spot-check: owner has every (Canvas, action) permission.
    let owner = role_permissions(Role::Owner);
    for &action in &[
        Action::Read,
        Action::Write,
        Action::Execute,
        Action::Delete,
        Action::ShareManage,
    ] {
        assert!(owner.contains(&Permission::new(ResourceType::Canvas, action)));
    }
    // Editor does NOT have Delete on Canvas.
    let editor = role_permissions(Role::Editor);
    assert!(!editor.contains(&Permission::new(ResourceType::Canvas, Action::Delete)));
}

#[test]
fn lock_lifecycle_and_contention() {
    let mgr = LockManager::new();
    let r = resource();
    // Free resource: write lock succeeds.
    let lock = mgr.try_write_lock(r, "alice").expect("first write lock");
    assert_eq!(lock.kind, LockKind::Write);
    // Writer present: another try_write fails.
    let err = mgr.try_write_lock(r, "bob").expect_err("contention");
    assert!(matches!(err, RbacError::LockHeld { .. }));
    // Writer present: a read lock attempt fails too.
    let err = mgr
        .try_read_lock(r, "bob")
        .expect_err("writer blocks readers");
    assert!(matches!(err, RbacError::LockHeld { .. }));
    // Release the writer.
    mgr.unlock_write(r, "alice").unwrap();
    // Now two readers can coexist.
    mgr.try_read_lock(r, "alice").unwrap();
    mgr.try_read_lock(r, "bob").unwrap();
    assert_eq!(mgr.reader_count(r), 2);
    // But not a writer while readers hold.
    let err = mgr
        .try_write_lock(r, "charlie")
        .expect_err("readers block writer");
    assert!(matches!(err, RbacError::LockHeld { .. }));
    // Drop both readers.
    mgr.unlock_read(r, "alice").unwrap();
    mgr.unlock_read(r, "bob").unwrap();
    // Now a writer can take it.
    mgr.try_write_lock(r, "charlie")
        .expect("writer can take after readers drop");
}

#[test]
fn audit_log_records_canvas_write() {
    let sink = InMemoryAuditSink::new();
    let r = resource();
    let u = user(1);
    record_audit_log(
        &sink,
        None,
        u,
        "canvas.write",
        (ResourceType::Canvas, r),
        Some(serde_json::json!({"nodes": 3})),
        Some(serde_json::json!({"nodes": 4})),
    );
    assert_eq!(sink.len(), 1);
    let entries = sink.entries();
    assert_eq!(entries[0].action_type, "canvas.write");
    assert_eq!(entries[0].resource_id, r.0);
    assert_eq!(entries[0].user_id, u);
}

#[test]
fn audit_sink_clone_shares_storage_and_records_under_load() {
    let sink = Arc::new(InMemoryAuditSink::new());
    let mut handles = Vec::new();
    for i in 0..8u8 {
        let s = sink.clone();
        handles.push(std::thread::spawn(move || {
            let r = ResourceId::new();
            record_audit_log(
                s.as_ref(),
                None,
                user(i),
                "permission.grant",
                (ResourceType::Canvas, r),
                None,
                None,
            );
        }));
    }
    for h in handles {
        h.join().expect("thread");
    }
    assert_eq!(sink.len(), 8);
}
