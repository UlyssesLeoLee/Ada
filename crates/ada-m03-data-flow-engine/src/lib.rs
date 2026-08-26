//! M-03: Data flow engine. Execute canvas-defined nodes
//! sequentially/parallelly.
//!
//! ## v0.1.0 scope (B5 batch)
//!
//! This crate is the **minimum skeleton** for the
//! canvas-defined data flow engine. The v0.1.0 surface is:
//!
//! - [`DataFlow`] — `id`, `description`, `nodes`, `edges`
//! - [`FlowNode`] — `id`, `kind` (`Source / Transform /
//!   Sink`), `label`
//! - [`FlowEdge`] — `from -> to`
//! - [`NodeKind`] — the three canonical node kinds
//! - [`NJson`] — newtype around `serde_json::Value`
//!   (canonical NJSON data bus type, D-07)
//! - [`DataFlowEngine`] trait —
//!   `async fn execute(&DataFlow, &HashMap<..>, Value) -> Result<Value>`
//! - [`InMemoryEngine`] — topologically-sorted sequential
//!   executor with per-node `NodeBody` lookup
//! - [`FnNode`] — closure adapter for [`NodeBody`]
//! - 5-variant [`FlowError`] (CyclicGraph, UnknownNode,
//!   ExecutionFailed, TypeMismatch, BackendError)
//! - 10 unit tests + 4 integration tests
//!
//! ## What v0.1.0 explicitly does **not** do
//!
//! - Parallel execution of independent branches
//!   (sequential only; production lands in B5+)
//! - Compile the flow into a static plan (the
//!   `topo_sort` runs on every `execute` call)
//! - Persist execution traces
//! - Honor the `tracing` layer integration (the
//!   `ada-telemetry` crate wires that in B5+)
//!
//! See `docs/modules/M-03-data-flow-engine.md` (DOC-MOD-003)
//! for the full design.
//!
//! 関連 IPA フェーズ: 22-52 (基本設計/詳細設計), 53-58 (実装), 59-95 (試験)
//! 設計書: docs/modules/M-03-data-flow-engine.md (DOC-MOD-003)

#![allow(missing_docs)]
#![allow(rust_2018_idioms)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

mod engine;
mod error;
mod flow;
mod njson;

pub use engine::{DataFlowEngine, FnNode, InMemoryEngine, NodeBody};
pub use error::{FlowError, Result};
pub use flow::{DataFlow, FlowEdge, FlowNode, FlowNodeId, NodeKind};
pub use njson::NJson;

/// Crate version, taken from `CARGO_PKG_VERSION` (single workspace
/// version per D-09).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Crate name, taken from `CARGO_PKG_NAME`.
pub const NAME: &str = env!("CARGO_PKG_NAME");

/// `blood`-layer string tag (仿生モデル 4 層分類, see
/// [`DOC-ARCH-001`](https://example.invalid/docs/architecture/00-anatomy-model.md)).
pub const LAYER: &str = "blood";

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
