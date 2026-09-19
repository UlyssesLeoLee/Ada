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
//! ## Features
//!
//! - Default: real `casbin` 2.x adapter + `notify` watcher.
//! - `hand-rolled`: pin the v0.4.0 hand-rolled evaluator (no
//!   `casbin`/`notify` runtime cost, but no model.conf + CSV
//!   policy fidelity).
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