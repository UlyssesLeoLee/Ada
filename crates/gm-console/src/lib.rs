//! gm-console — Commercial-grade Web dashboard and operations console for Ada platform.
//!
//! This crate provides:
//! - HTTP server (axum) hosting the Web frontend (static assets via rust-embed)
//! - Reverse-proxy routes forwarding into Ada core (api-gateway)
//! - Commercial-grade endpoints: health, metrics, license-info, terms acceptance
//!
//! Designed to be deployed standalone (port 8080) or behind envoy (D:/Ada user_policy 2026-09-01).
//!
//! Boundaries: gm-console is a UI/operations surface. It does NOT contain business logic.
//! All business operations MUST go through `ada-m13-api-gateway`. gm-console only:
//!   1. serves the static `dist/` Web bundle
//!   2. proxies /api/* to the upstream api-gateway svc
//!   3. exposes /healthz, /version, /license endpoints

#![forbid(unsafe_code)]
#![warn(missing_debug_implementations)]

pub mod config;
pub mod error;
pub mod routes;
pub mod server;

pub use config::Config;
pub use error::{Error, Result};
pub use server::serve;
