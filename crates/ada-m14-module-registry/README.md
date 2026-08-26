# ada-m14-module-registry

M-14: モジュールレジストリ (Module registry).
Atomic swap (D-02 WASM). Module manifest validation (D-04 JSON Schema).

## v0.1.0 scope (B4 batch)

This crate is a **minimum skeleton** for the cross-module
registry. The v0.1.0 surface is the in-process
`parking_lot::RwLock<HashMap<String, ModuleDescriptor>>` store
plus the trait surface that downstream services program against.

The production deployment (PostgreSQL `module_registry` table +
JSON-Schema validation hook + WASM atomic-swap via the
`ada-m06-plugin-sdk` runtime, see `DOC-MOD-014` §3.5) is
scheduled for B5+ once G4 (実装着手判定) is approved.

### What v0.1.0 provides

- `ModuleDescriptor` — name, version, kind, capabilities,
  endpoint, health snapshot, timestamps
- `ModuleKind` — `Ingest / Transform / Sink / Custom`
- `HealthState` — `Healthy / Degraded / Unhealthy / Unknown`
- `Capability` newtype (string-tag)
- `HealthTransition` audit-log entry
- `ModuleRegistry` — `register / deregister / get / list /
  heartbeat / transitions` with pluggable
  `Arc<dyn EventBus>` for state-change events
- `RegistryEvent` — `Registered / Deregistered / HealthChanged`
  envelopes built on top of `ada-m15-central-event-bus::BusEvent`
- 5-variant `RegistryError` (AlreadyRegistered, NotFound,
  InvalidDescriptor, HealthCheckFailed, BackendError)
- 16 unit tests + 4 integration tests

### What v0.1.0 explicitly does **not** do

- Persist descriptors to the `module_registry` table
- Hot-swap WASM modules atomically
- Validate the descriptor against
  `schemas/module-manifest.schema.json`
- Distribute registrations across cluster nodes (M-16 territory)
- Persist the in-process `transitions` log

## 関連 IPA フェーズ

22-52 (基本設計/詳細設計), 53-58 (実装), 59-95 (試験)

## 設計書

`docs/modules/M-14-module-registry.md` (DOC-MOD-014)
