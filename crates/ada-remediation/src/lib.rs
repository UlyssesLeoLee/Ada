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
//! - [`docs/observability/14-auto-remediation.md`] — architecture, runbook
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
//! Cooldown is enforced in **one** layer, and it is not durable:
//!
//!  - **In-process** ([`MemoryStore`]) — a `HashMap` in this binary. It
//!    gates `evaluate()` from re-firing a recently executed action for as
//!    long as the pod lives.
//!
//! # There is no persistent cooldown layer
//!
//! An earlier version of this comment described a second layer:
//! PL/pgSQL `remediation_check_cooldown()` plus a `remediation_cooldowns`
//! table, as "durable source of truth across process restarts and
//! replicas ... replicas that boot mid-window must see the cooldown, not
//! silently re-fire".
//!
//! **That layer does not exist.** `remediation_check_cooldown` is
//! *defined* by `db/migrations/V003__phase8_remediation.sql` and is
//! called from nowhere in this workspace; the only store wired by
//! `main.rs` is `MemoryStore`.
//!
//! Three consequences an operator must plan for, given that
//! `deploy/k8s/ada-remediation.yaml` runs **two replicas**:
//!
//!  1. **Replicas do not share cooldowns.** An alert reaches one replica
//!     through the `ClusterIP` Service; it records the cooldown there. A
//!     second alert inside the same window can land on the other replica,
//!     which has no record and re-executes the remediation.
//!  2. **A restart forgets everything.** Cooldowns do not survive a pod
//!     restart, a rollout, or a reschedule.
//!  3. **The shipped runbooks are not all safe to run twice.**
//!     `config/remediation/disk-space-low.json` includes
//!     `find /var/log -type f -name '*.gz' -mtime +7 -delete`.
//!
//! Until the persistent store is wired, either run a single replica or
//! accept that the cooldown is advisory. Fixing the comment was not
//! optional; it was the only thing standing between an operator and a
//! false guarantee.
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
pub mod auth;
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
    DryRunExecutor, ExecutionContext, LoggingClient, NetworkClient, RealExecutor, RecordedRequest,
    StepExecutionResult, StepExecutor,
};
pub use history::{HistoryQuery, HistoryRecord, MemoryStore};
pub use state::EngineState;
