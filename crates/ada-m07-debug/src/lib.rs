//! M-07: Debug service. Breakpoints, step execution, variable watch, trace logging.
//!
//! 関連 IPA フェーズ: 22-52 (基本設計/詳細設計), 53-58 (実装), 59-95 (試験)
//! 設計書: docs/modules/M-m07-*.md
//! ワークフロー: docs/architecture/08-workflow-overview.md
//!
//! この crate は v0.1.0 scaffold 段階です。実際の実装は G4（実装着手判定）通過後に開始。

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

/// プレースホルダ: クレートバージョン
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// プレースホルダ: クレート名
pub const NAME: &str = env!("CARGO_PKG_NAME");

/// プレースホルダ: レイヤー (仿生モデル 4 層: skeleton/blood/nerve/muscle)
pub const LAYER: &str = "muscle";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_not_empty() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn test_name_not_empty() {
        assert!(!NAME.is_empty());
    }

    #[test]
    fn test_layer_is_known() {
        assert!(
            ["skeleton", "blood", "nerve", "muscle", "shared"].contains(&LAYER),
            "Unknown layer: {}",
            LAYER
        );
    }
}
