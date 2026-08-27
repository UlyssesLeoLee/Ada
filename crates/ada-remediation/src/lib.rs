//! `ada-remediation` — observability Phase 8 Auto-remediation runbook engine.
//!
//! The crate turns declarative *runbooks* into executable
//! *remediation actions*. Runbooks are loaded from
//! JSON config files under `config/remediation/`, mapped 1:1
//! to Alertmanager alert names, and executed by an in-process
//! state machine. Cooldown and retry policy live on the
//! [`RemediationAction`] struct itself; persistent history
//! is written to PostgreSQL by the `remediation_record_execution()`
//! function in `db/migrations/V003__phase8_remediation.sql`.
//!
//! # Design
//!
//! Sources of truth (these are the only documents the engine
//! is constrained to follow):
//!
//! - [`docs/observability/11-phased-rollout.md` §10] — phase 8 scope
//! - [`docs/observability/12-auto-remediation.md`] — architecture, runbook
//!   authoring guide, cooldown policy (introduced by v0.6.0)
//! - [`db/migrations/V003__phase8_remediation.sql`] — durable history
//!
//! # State machine
//!
//! ```text
//!             evaluate()                     all steps OK
//!   Idle ────────────────▶ Evaluating ────────────────────▶ Cooldown
//!                              │
//!                              │ step fails
//!                              ▼
//!                          Executing
//!                              │
//!                  ┌───────────┼───────────┐
//!                  ▼           ▼           ▼
//!              Failed     Retrying     Cooldown
//!           (max_retries  (backoff)    (window elapses
//!            exhausted)                 → back to Idle)
//! ```
//!
//! Cooldown is enforced in two layers:
//!
//!  1. **In-process** (this crate, [`MemoryStore`]) — fast path that gates
//!     `evaluate()` from re-firing a recently executed action while the
//!     persistent row is being written.
//!  2. **Persistent** (PL/pgSQL `remediation_check_cooldown()` plus
//!     `remediation_cooldowns` table) — durable source of truth across
//!     process restarts and replicas. Replicas that boot mid-window must
//!     see the cooldown, not silently re-fire.
//!
//! # Quick start
//!
//! ```no_run
//! use ada_remediation::{RemediationEngine, MemoryStore, AlertEvent};
//! use std::time::Duration;
//!
//! # async fn run() -> anyhow::Result<()> {
//! let engine = RemediationEngine::with_defaults();
//! let store  = MemoryStore::new();
//!
//! let alert = AlertEvent::builder("DiskSpaceFillingFast")
//!     .label("severity", "P2")
//!     .label("service", "m13-api-gateway")
//!     .build();
//!
//! let actions = engine.evaluate(&alert);
//! for action in &actions {
//!     if store.is_in_cooldown(&action.id) {
//!         continue;
//!     }
//!     let outcome = engine.execute(action).await?;
//!     store.record_success(&action.id, action.cooldown, &alert.alert_name);
//! }
//! # Ok(()) }
//! ```

#![deny(unsafe_code)]
#![warn(missing_debug_implementations)]

pub mod action;
pub mod alert;
pub mod config;
pub mod engine;
pub mod error;
pub mod executor;
pub mod history;
pub mod http;
pub mod metrics;
pub mod state;
pub mod watcher;

pub use action::{ActionOutcome, ActionStep, ExecutorMode, RemediationAction};
pub use alert::AlertEvent;
pub use config::{load_runbooks_from_dir, RunbookFile};
pub use engine::RemediationEngine;
pub use error::{RemediationError, Result};
pub use executor::{
    DryRunExecutor, ExecutionContext, LoggingClient, NetworkClient, RealExecutor,
    RecordedRequest, StepExecutionResult, StepExecutor,
};
pub use history::{HistoryQuery, HistoryRecord, MemoryStore};
pub use state::EngineState;
