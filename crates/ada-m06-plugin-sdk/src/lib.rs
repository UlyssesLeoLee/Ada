//! M-06: Plugin SDK. 3 PluginKind, manifest + host + sandbox
//! (capability-based, declarative).
//!
//! ## v0.1.0 scope (B6)
//!
//! This crate is the **minimum skeleton** for the cross-module
//! plugin system defined in [`DOC-MOD-006`](../docs/modules/M-06-plugin-sdk.md).
//! The v0.1.0 surface is:
//!
//! - [`PluginKind`] — three canonical kinds
//!   (`Wasm / Native / Script`)
//! - [`PluginManifest`] — id, version, kind, capabilities,
//!   entry_point, hash, signature
//! - [`PluginHost`] trait — `install / uninstall / invoke / list`
//! - [`InMemoryHost`] — process-local plugin registry
//! - [`SandboxPolicy`] — declarative capability list and
//!   resource limits (max_memory_mb, max_cpu_ms_per_call)
//! - 5-variant [`SdkError`] (`PluginNotFound`, `ManifestInvalid`,
//!   `CapabilityDenied`, `HashMismatch`, `BackendError`)
//! - 10 unit tests + 4 integration tests
//!
//! ## What v0.1.0 explicitly does **not** do
//!
//! - Real WASM execution (Wasmtime integration is B7+)
//! - Dynamic library loading for `Native` plugins
//! - Scripted plugin runtime (Rhai / Lua / JS)
//! - Persistent plugin storage (the `plugin_manifest` table is
//!   B7+ work)
//! - Signature verification (the field is reserved; the v0.1.0
//!   stores it but does not enforce it)
//!
//! See `docs/modules/M-06-plugin-sdk.md` (DOC-MOD-006) for the
//! full design.
//!
//! 関連 IPA フェーズ: 22-52 (基本設計/詳細設計), 53-58 (実装), 59-95 (試験)
//! 設計書: docs/modules/M-06-plugin-sdk.md (DOC-MOD-006)

#![allow(missing_docs)]
#![allow(rust_2018_idioms)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

mod error;
mod host;
mod manifest;
mod sandbox;

pub use error::{Result, SdkError};
pub use host::{InMemoryHost, PluginHost};
pub use manifest::{PluginId, PluginKind, PluginManifest};
pub use sandbox::{ResourceLimits, SandboxPolicy, DEFAULT_MAX_MEMORY_MB};

/// Crate version, taken from `CARGO_PKG_VERSION` (single workspace
/// version per D-09).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Crate name, taken from `CARGO_PKG_NAME`.
pub const NAME: &str = env!("CARGO_PKG_NAME");

/// `nerve`-layer string tag (仿生モデル 4 層分類, see
/// [`DOC-ARCH-001`](https://example.invalid/docs/architecture/00-anatomy-model.md)).
pub const LAYER: &str = "nerve";

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

    #[test]
    fn default_max_memory_is_positive() {
        const { assert!(DEFAULT_MAX_MEMORY_MB > 0) };
    }
}
