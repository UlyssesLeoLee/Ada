//! M-04: Pipeline orchestration. DAG-based dependency
//! resolution and execution control.
//!
//! ## v0.1.0 scope (B6)
//!
//! This crate is a **minimum skeleton** for the
//! cross-module orchestrator defined in
//! [`DOC-MOD-004`](../docs/modules/M-04-orchestration.md).
//! The v0.1.0 surface is the in-process [`Scheduler`]
//! contract that downstream crates (the API gateway, the
//! control-flow executor, the M-08 trigger manager) program
//! against. The default impl is [`InMemoryScheduler`], a
//! process-local `parking_lot::Mutex<Vec<Job>>` that tracks
//! state transitions but does **not** actually run the work
//! (the worker pool lands in B7+).
//!
//! - [`Job`] — `id`, `kind`, `payload`, `state`,
//!   `created_at_ms`, `started_at_ms`, `finished_at_ms`,
//!   `owner`
//! - [`JobState`] — six canonical states
//!   (`Pending / Queued / Running / Succeeded / Failed / Cancelled`)
//! - [`JobKind`] — five canonical kinds
//!   (`Acquisition / Normalization / FlowExecution / ControlFlow / Export`)
//! - [`Scheduler`] trait — `enqueue / poll / cancel / state_of`
//! - [`InMemoryScheduler`] — FIFO queue with state machine
//! - 5-variant [`OrchError`] (`JobNotFound`, `InvalidState`,
//!   `QueueFull`, `BackendError`, `Cancelled`)
//! - 12 unit tests + 4 integration tests
//!
//! ## What v0.1.0 explicitly does **not** do
//!
//! - Actually execute the work (state transitions only; the
//!   worker pool lands in B7+)
//! - Persist jobs to the `orchestration_jobs` table
//! - Distribute jobs across cluster nodes (M-16 territory)
//! - Honor cron / event-driven triggers (those live in
//!   `ada-m08-trigger`)
//! - Enforce RBAC on `enqueue` (M-11 wiring is B7+)
//!
//! See `docs/modules/M-04-orchestration.md` (DOC-MOD-004)
//! for the full design.
//!
//! 関連 IPA フェーズ: 22-52 (基本設計/詳細設計), 53-58 (実装), 59-95 (試験)
//! 設計書: docs/modules/M-04-orchestration.md (DOC-MOD-004)

#![allow(missing_docs)]
#![allow(rust_2018_idioms)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

mod error;
mod job;
mod scheduler;

pub use error::{OrchError, Result};
pub use job::{Job, JobId, JobKind, JobState};
pub use scheduler::{enqueue_job, InMemoryScheduler, Scheduler, DEFAULT_QUEUE_CAPACITY};

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
    fn default_queue_capacity_is_positive() {
        const { assert!(DEFAULT_QUEUE_CAPACITY > 0) };
    }
}
