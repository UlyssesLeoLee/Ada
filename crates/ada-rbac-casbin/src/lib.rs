//! ada-rbac-casbin — RBAC + ABAC wrapper on top of
//! ada-m11-rbac-collab.
//!
//! ## v0.5.0 scope
//!
//! The v0.5.0 release swaps the internal evaluator to
//! `casbin::Enforcer` 2.x with `notify`-driven hot reload. The
//! public API (`Enforcer`, `Attrs`, `PolicySet`, `HotReload`,
//! `AdminApi`) is preserved unchanged from v0.4.0; only the internal
//! implementation moved.
//!
//! ## Evaluator selection
//!
//! Which evaluator is compiled is a two-way function of the
//! `hand-rolled` feature (full matrix in `Cargo.toml`):
//!
//! - `hand-rolled` on any target — the v0.4.0 evaluator.
//! - Feature off, any target — the real `casbin` 2.x adapter plus the
//!   `notify` watcher. This is the default on every platform.
//!
//! There is no "no evaluator compiled in" configuration. One used to
//! exist for targets that were neither Linux nor macOS, on the stated
//! grounds that casbin needs `openssl-sys`. That premise was false, and
//! the configuration's real effect was to make the production evaluator
//! unbuildable on Windows — where it was therefore never compiled, and
//! never tested, while the only CI leg that could have compiled it had
//! not started a single job in 24 consecutive runs.
//!
//! The public API (`Enforcer`, `Attrs`, `PolicySet`, `HotReload`,
//! `AdminApi`) is identical in all three configurations.
//!
//! See `docs/commercial/auth-billing-arch.md` §4 for the binding
//! contract this crate implements.

#![forbid(unsafe_code)]
#![warn(missing_debug_implementations)]

#[cfg(feature = "hand-rolled")]
pub mod hand_rolled;

#[cfg(not(feature = "hand-rolled"))]
pub mod casbin_impl;

pub mod admin;
pub mod attrs;
pub mod contract;
pub mod enforcer;
pub mod error;
pub mod hot_reload;
pub mod policy;

pub use admin::AdminApi;
pub use attrs::{Attrs, IpAddr};
pub use enforcer::Enforcer;
pub use error::{RbacCasbinError, Result};
pub use hot_reload::HotReload;
pub use policy::PolicySet;
