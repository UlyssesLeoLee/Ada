//! M-07: Debug tools. Breakpoints, inspector, trace recorder.
//!
//! ## v0.1.0 scope (B6)
//!
//! Minimum skeleton for the cross-module debug facilities
//! defined in [`DOC-MOD-007`](../docs/modules/M-07-debug.md).
//! The v0.1.0 surface is:
//!
//! - [`Breakpoint`] — id, location, kind, state
//! - [`BreakpointKind`] — three canonical kinds
//!   (`Line / Conditional / Entry`)
//! - [`BreakpointState`] — three states (`Active / Disabled / Hit`)
//! - [`Inspector`] — walks a stack of [`InspectFrame`]
//! - [`TraceEvent`] / [`TraceRecorder`] — `Span / Log / Metric`
//!
//! ## What v0.1.0 explicitly does **not** do
//!
//! - Attach to a real process via `ptrace` / `gdb` (B7+)
//! - Source-map aware breakpoints
//! - Distributed trace export (OTLP / Jaeger)
//!
//! See `docs/modules/M-07-debug.md` (DOC-MOD-007) for the full
//! design.
//!
//! 関連 IPA フェーズ: 22-52 (基本設計/詳細設計), 53-58 (実装), 59-95 (試験)
//! 設計書: docs/modules/M-07-debug.md (DOC-MOD-007)

#![allow(missing_docs)]
#![allow(rust_2018_idioms)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

mod breakpoint;
mod error;
mod inspector;
mod trace;

pub use breakpoint::{Breakpoint, BreakpointId, BreakpointKind, BreakpointState, Location};
pub use error::{DebugError, Result};
pub use inspector::{InspectFrame, Inspector};
pub use trace::{TraceEvent, TraceKind, TraceRecorder};

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
}
