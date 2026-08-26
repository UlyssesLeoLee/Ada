//! M-08: Trigger manager. 4 `TriggerKind` (Cron / Webhook /
//! Event / Manual), `TriggerRule`, `TriggerManager`.
//!
//! ## v0.1.0 scope (B6)
//!
//! Minimum skeleton for the trigger / scheduling facilities
//! defined in [`DOC-MOD-008`](../docs/modules/M-08-trigger.md).
//! The v0.1.0 surface is:
//!
//! - [`TriggerKind`] — four kinds
//!   (`Cron / Webhook / Event / Manual`)
//! - [`TriggerRule`] — id, kind, schedule, action, enabled
//! - [`TriggerManager`] — `add / remove / list / enable /
//!   disable / match_event`
//! - 5-variant [`TriggerError`] (`TriggerNotFound`,
//!   `InvalidCron`, `ActionFailed`, `DuplicateId`,
//!   `BackendError`)
//!
//! The v0.1.0 skeleton uses a simple, dependency-free
//! 5-field cron parser (minute / hour / dom / month / dow).
//! B7+ will swap in the `cron` crate for the full spec.
//!
//! See `docs/modules/M-08-trigger.md` (DOC-MOD-008) for the
//! full design.
//!
//! 関連 IPA フェーズ: 22-52 (基本設計/詳細設計), 53-58 (実装), 59-95 (試験)
//! 設計書: docs/modules/M-08-trigger.md (DOC-MOD-008)

#![allow(missing_docs)]
#![allow(rust_2018_idioms)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

mod error;
mod manager;
mod rule;

pub use error::{Result, TriggerError};
pub use manager::{TriggerManager, DEFAULT_CRON_FIELDS};
pub use rule::{Action, TriggerId, TriggerKind, TriggerRule};

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
    fn default_cron_has_five_fields() {
        assert_eq!(DEFAULT_CRON_FIELDS, 5);
    }
}
