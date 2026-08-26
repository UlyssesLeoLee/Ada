# ada-m10-tenant-middleware

M-10: テナントミドルウェア (Tenant middleware).
11 tables, 6 PL/pgSQL procedures, RLS, multi-tenant isolation.
[NF-SEC] required.

## v0.1.0 scope (B4 batch)

This crate is a **minimum skeleton** for the multi-tenant
isolation layer. The v0.1.0 surface is the in-process
middleware contract that downstream crates (HTTP gateway,
gRPC handlers, queue consumers) program against.

The production deployment (PostgreSQL `tenant_context` table +
`app.current_tenant` session variable + RLS policies on the
11 tenant-scoped tables, see `DOC-MOD-010` §3.3 and the six
PL/pgSQL stored procedures in §3.5) is scheduled for B5+ once
G4 (実装着手判定) is approved.

### What v0.1.0 provides

- `TenantContext` — per-request `(tenant_id, user_id,
  request_id)` bundle
- `RequestId` — `Uuid`-backed per-request correlation key
- `TenantResolver` trait — transport-agnostic context lookup
- `TenantMiddleware` trait — `set` / `get` / `clear` /
  `active_contexts`
- `InMemoryMiddleware` — `parking_lot::RwLock<HashMap>`-backed
  in-process impl
- 5-variant `TenantError` (MissingContext, InvalidTenant,
  CrossTenantAccess, ContextNotInitialized, BackendError)
- 9 unit tests + 4 integration tests

### What v0.1.0 explicitly does **not** do

- Persist context into the `tenant_context` table
- Inject `app.current_tenant` into a real PostgreSQL session
- Enforce RLS on the 11 tenant-scoped tables
- Implement row-level audit logging (see `ada-m11-rbac-collab`
  for the audit-sink surface)
- Honor distributed-trace context propagation

## 関連 IPA フェーズ

22-52 (基本設計/詳細設計), 53-58 (実装), 59-95 (試験)

## 設計書

`docs/modules/M-10-tenant-middleware.md` (DOC-MOD-010)
