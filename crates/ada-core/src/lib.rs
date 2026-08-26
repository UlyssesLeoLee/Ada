//! `ada-core` — shared types and error surface for the Ada workspace.
//!
//! This crate is the **shared layer** of the 仿生モデル (bionic model)
//! defined in [`DOC-ARCH-001`](https://example.invalid/docs/architecture/00-anatomy-model.md):
//! every other Ada crate is expected to depend on it for the
//! cross-cutting identifier types, the canonical error enum, and a
//! thin `telemetry!` macro that pins the `tracing` span name and the
//! `layer` field.
//!
//! ## What `ada-core` provides (v0.1.0)
//!
//! - [`AdaError`] + [`Result`] — the workspace-wide error enum
//!   (uses `thiserror` per [`DOC-ARCH-007 §8`](https://example.invalid/docs/architecture/06-rust-tech-selection.md))
//! - [`TenantId`], [`UserId`], [`CanvasId`], [`IdempotencyKey`] —
//!   `Uuid`-backed newtype identifiers
//! - [`AdaLayer`] — typed tag for the five layers
//!   (Skeleton / Blood / Nerve / Muscle / Shared)
//! - [`telemetry!`] — a one-line macro around `tracing::info_span!`
//!
//! ## What is **not** in `ada-core` yet
//!
//! - `NJson` and the NJSON data-bus types — owned by `ada-m03-data-flow-engine` (D-07)
//! - RBAC / permission types — owned by `ada-m11-rbac-collab`
//! - Concrete tracing subscriber / metrics exporter — owned by `ada-telemetry`
//!
//! 関連 IPA フェーズ: 22-52 (基本設計/詳細設計), 53-58 (実装), 59-95 (試験)
//! 設計書:
//! - [`DOC-ARCH-001`](../docs/architecture/00-anatomy-model.md) — 仿生モデル + shared layer
//! - [`DOC-ARCH-007`](../docs/architecture/06-rust-tech-selection.md) — Rust crate 選択
//! - [`DOC-DEC-002 D-09`](../docs/decisions/02-design-adrs.md) — single workspace version
//! - [`DOC-DEC-002 D-13`](../docs/decisions/02-design-adrs.md) — `ada-core` = MIT
//!
//! License: MIT (D-13)

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

mod error;
mod telemetry;
mod types;

pub use error::{AdaError, Result};
pub use types::{AdaLayer, CanvasId, IdempotencyKey, TenantId, UserId};

/// Crate version, taken from `CARGO_PKG_VERSION` (single workspace
/// version per D-09).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Crate name, taken from `CARGO_PKG_NAME`.
pub const NAME: &str = env!("CARGO_PKG_NAME");

/// `shared`-layer string tag. Kept as a `&str` for the `&str`-based
/// cfg checks that already exist in the workspace; the typed version
/// is [`AdaLayer::Shared`].
pub const LAYER: &str = "shared";

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
    fn layer_string_is_known() {
        assert!(
            ["skeleton", "blood", "nerve", "muscle", "shared"].contains(&LAYER),
            "Unknown layer: {LAYER}"
        );
    }

    #[test]
    fn ada_layer_str_round_trip() {
        for layer in [
            AdaLayer::Skeleton,
            AdaLayer::Blood,
            AdaLayer::Nerve,
            AdaLayer::Muscle,
            AdaLayer::Shared,
        ] {
            let s: &'static str = layer.into();
            assert!(
                ["skeleton", "blood", "nerve", "muscle", "shared"].contains(&s),
                "unexpected layer string: {s}"
            );
            assert_eq!(layer.to_string(), s);
        }
    }
}
