//! ada-rbac-casbin — RBAC + ABAC wrapper on top of
//! ada-m11-rbac-collab.
//!
//! ## v0.4.0 scope
//!
//! This crate hand-rolls a small RBAC + ABAC evaluator. The m11
//! role × permission matrix is imported as the static policy base
//! (`policy::role_policy`), and the ABAC attribute bag
//! ([`attrs::Attrs`]) carries tenant scoping + ownership flag +
//! request IP for fine-grained enforcement.
//!
//! `casbin` 2.x is the canonical choice for v0.5.0; it is
//! explicitly **out of scope** for v0.4.0 because (a) its async +
//! rhai default-engine surface did not match the v0.4.0 skeleton
//! contract, and (b) wiring a real adapter requires Postgres which
//! is itself a v0.5.0 deliverable.
//!
//! See `docs/commercial/auth-billing-arch.md` §4 for the binding
//! contract this crate implements.

#![forbid(unsafe_code)]
#![warn(missing_debug_implementations)]

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