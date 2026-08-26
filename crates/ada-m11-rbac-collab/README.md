# ada-m11-rbac-collab

M-11: 权限与协作 (RBAC & collaboration).
5 roles, permission matrix, in-process lock manager, audit_log
interface (D-07).

## v0.1.0 scope (B3 batch)

This crate is a **minimum skeleton** for the cross-cutting RBAC +
collaboration concerns. The v0.1.0 surface is:

- `Role` — five canonical roles (Owner / Admin / Editor /
  Executor / Viewer) with privilege-descending `Ord`
- `Permission` / `ResourceType` / `Action` — the
  (resource-type, action) pair
- `role_permissions(role)` — static role → permission matrix
  matching `DOC-MOD-011` §3.1
- `Collaboration` / `CollaborationMap` — per-resource user → role
  table, with `grant` / `revoke` / `set_role` / `authorize`
- `LockManager` — in-process per-resource read/write locks with
  `try_*` (non-blocking) and `*_lock` (await) variants
- `AuditSink` / `InMemoryAuditSink` / `record_audit_log` —
  pluggable audit logging interface
- 7-variant `RbacError` (UnknownUser, UnknownResource,
  AlreadyGranted, NotGranted, LockHeld, LockNotHeld,
  InsufficientPermission)
- 24 unit tests + 6 integration tests

## What v0.1.0 explicitly does **not** do

- Persist audit entries (no `audit_log` table yet)
- Distribute locks across cluster nodes
- Back the collaboration map with Postgres
- Real CRDT collaboration (only the in-process lock manager is
  provided; the yrs/Yjs integration lives in the M-12 frontend
  and the WebSocket relay in B4+)

## 関連 IPA フェーズ

22-52 (基本設計/詳細設計), 53-58 (実装), 59-95 (試験)

## 設計書

`docs/modules/M-11-rbac-collab.md` (DOC-MOD-011)
