//! M-14: Module registry. Atomic swap (D-02 WASM). Module
//! manifest validation (D-04 JSON Schema).
//!
//! ## v0.1.0 scope (B4)
//!
//! This crate is a **minimum skeleton** for the cross-module
//! registry. The v0.1.0 surface is:
//!
//! - [`ModuleDescriptor`] — `name`, `version`, `kind`,
//!   `capabilities`, `endpoint`, `health` snapshot
//! - [`ModuleKind`] — `Ingest / Transform / Sink / Custom`
//! - [`HealthState`] — `Healthy / Degraded / Unhealthy / Unknown`
//! - [`ModuleRegistry`] — in-process store with
//!   `register / deregister / get / list / heartbeat` and
//!   pluggable event emission via `Arc<dyn EventBus>`
//! - [`RegistryEvent`] — `Registered / Deregistered / HealthChanged`
//!   envelope built on top of [`BusEvent`](ada_m15_central_event_bus::BusEvent)
//! - 5-variant [`RegistryError`] (`AlreadyRegistered`,
//!   `NotFound`, `InvalidDescriptor`, `HealthCheckFailed`,
//!   `BackendError`)
//! - 11 unit tests + 4 integration tests
//!
//! ## What v0.1.0 explicitly does **not** do
//!
//! - Persist descriptors to the `module_registry` table
//! - Hot-swap WASM modules atomically (planned for B5+ once
//!   the `ada-m06-plugin-sdk` runtime is wired in)
//! - Validate the descriptor against the JSON Schema
//!   `schemas/module-manifest.schema.json` (the validation hook
//!   is a single `validate()` method on `ModuleDescriptor`; the
//!   schema itself lives in `docs/schemas/`)
//! - Distribute registrations across cluster nodes
//!   (cross-node replication is M-16 territory)
//!
//! See `docs/modules/M-14-module-registry.md` (DOC-MOD-014) for
//! the full design.
//!
//! 関連 IPA フェーズ: 22-52 (基本設計/詳細設計), 53-58 (実装), 59-95 (試験)
//! 設計書: docs/modules/M-14-module-registry.md (DOC-MOD-014)

#![allow(missing_docs)]
#![allow(rust_2018_idioms)]
#![allow(clippy::doc_markdown)]

mod error;
mod event;
mod registry;

pub use error::{RegistryError, Result};
pub use event::{RegistryEvent, RegistryEventKind};
pub use registry::{
    Capability, HealthState, HealthTransition, ModuleDescriptor, ModuleKind, ModuleRegistry,
};

/// Crate version, taken from `CARGO_PKG_VERSION` (single workspace
/// version per D-09).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Crate name, taken from `CARGO_PKG_NAME`.
pub const NAME: &str = env!("CARGO_PKG_NAME");

/// `skeleton`-layer string tag (仿生モデル 4 層分類, see
/// [`DOC-ARCH-001`](https://example.invalid/docs/architecture/00-anatomy-model.md)).
pub const LAYER: &str = "skeleton";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_not_empty() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn name_not_empty() {
        assert!(!NAME.is_empty());
    }

    #[test]
    fn layer_is_known() {
        assert!(
            ["skeleton", "blood", "nerve", "muscle", "shared"].contains(&LAYER),
            "Unknown layer: {LAYER}"
        );
    }
}
