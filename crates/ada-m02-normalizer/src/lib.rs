//! M-02: Data normalization. Convert raw messages to
//! standard NJSON schema.
//!
//! ## v0.1.0 scope (B5 batch)
//!
//! This crate is the **minimum skeleton** for the
//! rule-driven normalization layer that sits between the
//! acquisition adapters (`ada-m01-acquisition`) and the
//! data-flow engine (`ada-m03-data-flow-engine`).
//!
//! - [`RuleKind`] — `Trim / Lowercase / Regex / Date /
//!   Coalesce`
//! - [`NormalizationRule`] — id, field_path, kind
//! - [`NormalizationPipeline`] — ordered `Vec<Rule>`, eager
//!   `Regex` validation at build time, fail-fast on the
//!   first rule error
//! - [`NormalizedRecord`] — `source_id + seq + payload`
//! - 5-variant [`NormalizerError`] (UnknownField,
//!   RuleExecutionFailed, TypeMismatch, InvalidRegex,
//!   BackendError)
//! - 9 unit tests + 4 integration tests
//!
//! ## What v0.1.0 explicitly does **not** do
//!
//! - Persist normalized records to a topic / table
//! - Support nested path wildcards (`user.*.email`); only
//!   top-level + one-segment nested fields are supported
//! - Snapshot / copy-on-write semantics; a failed apply
//!   leaves the record partially mutated (the production
//!   build will use an arena or document the partial-state
//!   contract)
//! - Type coercion (string → number, etc.)
//!
//! See `docs/modules/M-02-normalizer.md` (DOC-MOD-002) for
//! the full design.
//!
//! 関連 IPA フェーズ: 22-52 (基本設計/詳細設計), 53-58 (実装), 59-95 (試験)
//! 設計書: docs/modules/M-02-normalizer.md (DOC-MOD-002)

#![allow(missing_docs)]
#![allow(rust_2018_idioms)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

mod error;
mod pipeline;
mod rule;

pub use error::{NormalizerError, Result};
pub use pipeline::{NormalizationPipeline, NormalizedRecord};
pub use rule::{NormalizationRule, RuleKind};

/// Raw record type re-exported from the acquisition
/// adapters. The skeleton uses the same shape so the
/// pipeline can be called from either side without a
/// translation layer.
pub use ada_m01_acquisition::RawRecord;

/// Crate version, taken from `CARGO_PKG_VERSION` (single workspace
/// version per D-09).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Crate name, taken from `CARGO_PKG_NAME`.
pub const NAME: &str = env!("CARGO_PKG_NAME");

/// `blood`-layer string tag (仿生モデル 4 層分類, see
/// [`DOC-ARCH-001`](https://example.invalid/docs/architecture/00-anatomy-model.md)).
pub const LAYER: &str = "blood";

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
