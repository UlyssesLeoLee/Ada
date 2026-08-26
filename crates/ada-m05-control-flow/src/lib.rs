//! M-05: Control flow executor. Conditional branches,
//! loops, parallel branches.
//!
//! ## v0.1.0 scope (B5 batch)
//!
//! This crate is the **minimum skeleton** for the
//! canvas-defined control flow executor. The v0.1.0
//! surface is:
//!
//! - [`ControlStep`] — `id`, `kind` (`Action / Branch /
//!   Loop / Switch / Parallel`), optional `condition`,
//!   `next_step`
//! - [`StepKind`] — the five canonical step kinds
//! - [`BranchStep`] / [`LoopStep`] / [`SwitchStep`] /
//!   [`ParallelStep`] — per-kind payloads
//! - [`Condition`] — boolean expression over
//!   `HashMap<String, Value>` (Eq / Ne / Lt / Gt /
//!   Contains / And / Or / Not)
//! - [`ControlFlowExecutor`] — walks the step table, with
//!   `max_iterations` + `timeout` caps
//! - [`ExecutionResult`] — `final_step`, `context`, `trace`
//! - 5-variant [`ExecutorError`] (StepNotFound,
//!   ConditionError, MaxRecursionExceeded, Timeout,
//!   BackendError)
//! - 10 unit tests + 4 integration tests
//!
//! ## What v0.1.0 explicitly does **not** do
//!
//! - Parallel branch execution (the [`StepKind::Parallel`]
//!   step walks branches sequentially; production will
//!   run them in B5+)
//! - Action side-effects (the [`StepKind::Action`] step is
//!   a no-op; production will let actions mutate the
//!   context in B5+)
//! - Persistence of the trace (the
//!   [`ExecutionResult::trace`] is returned but not
//!   written to a store)
//! - Honor distributed-trace context propagation
//!
//! See `docs/modules/M-05-control-flow.md` (DOC-MOD-005)
//! for the full design.
//!
//! 関連 IPA フェーズ: 22-52 (基本設計/詳細設計), 53-58 (実装), 59-95 (試験)
//! 設計書: docs/modules/M-05-control-flow.md (DOC-MOD-005)

#![allow(missing_docs)]
#![allow(rust_2018_idioms)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

mod condition;
mod error;
mod executor;
mod step;

pub use condition::Condition;
pub use error::{ExecutorError, Result};
pub use executor::{ControlFlowExecutor, ExecutionResult, DEFAULT_MAX_ITERATIONS, DEFAULT_TIMEOUT};
pub use step::{BranchStep, ControlStep, LoopStep, ParallelStep, StepKind, SwitchCase, SwitchStep};

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
