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
//! Which evaluator is compiled is a three-way function of the
//! `hand-rolled` feature and the build target (full matrix in
//! `Cargo.toml`):
//!
//! - `hand-rolled` on any target — the v0.4.0 evaluator. The only
//!   way to get an evaluator on Windows.
//! - Feature off, Linux/macOS — the real `casbin` 2.x adapter plus
//!   the `notify` watcher.
//! - Feature off, any other target — no evaluator. `Enforcer` and
//!   `HotReload` constructors return
//!   [`RbacCasbinError::UnsupportedEvaluator`]; there is no permissive
//!   fallback.
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

#[cfg(all(
    not(feature = "hand-rolled"),
    any(target_os = "linux", target_os = "macos")
))]
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
