//! [`NormalizationPipeline`]: an ordered `Vec<NormalizationRule>`
//! applied to a [`RawRecord`](crate::RawRecord).
//!
//! The v0.1.0 pipeline is intentionally trivial: it owns a
//! `Vec<NormalizationRule>` and applies them in order on
//! each [`apply`](NormalizationPipeline::apply) call. The
//! `validate` constructor eagerly compiles any `Regex`
//! rules so callers fail fast on a bad config rather than
//! at apply time.
//!
//! See [`DOC-MOD-002`](../docs/modules/M-02-normalizer.md) §3.5
//! for the full lifecycle.

use crate::error::{NormalizerError, Result};
use crate::rule::{NormalizationRule, RuleKind};

/// A normalized record returned by
/// [`NormalizationPipeline::apply`]. The skeleton keeps the
/// payload as `serde_json::Value` and tags it with the
/// source `id` + a per-record `seq` so downstream code can
/// correlate the output back to the input [`RawRecord`](crate::RawRecord).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NormalizedRecord {
    /// Source `id` this record came from.
    pub source_id: String,
    /// Per-source sequence number (copied from the input
    /// record).
    pub seq: u64,
    /// The normalized JSON payload.
    pub payload: serde_json::Value,
}

impl NormalizedRecord {
    /// Build a new record.
    #[must_use]
    pub fn new(source_id: impl Into<String>, seq: u64, payload: serde_json::Value) -> Self {
        Self {
            source_id: source_id.into(),
            seq,
            payload,
        }
    }
}

/// Ordered list of rules applied in declaration order. The
/// skeleton does **not** keep a compiled `Regex` cache: the
/// first call to a `Regex` rule compiles the pattern, and
/// subsequent calls re-compile (the `regex` crate's
/// `Regex::new` is cheap enough at v0.1.0 throughput).
#[derive(Debug, Clone, Default)]
pub struct NormalizationPipeline {
    rules: Vec<NormalizationRule>,
}

impl NormalizationPipeline {
    /// Build an empty pipeline.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Build a pipeline from a pre-validated rule list. The
    /// rules are **not** re-validated here; the
    /// [`builder`](Self::builder) constructor handles the
    /// eager regex / empty-id checks.
    #[must_use]
    pub fn from_rules(rules: Vec<NormalizationRule>) -> Self {
        Self { rules }
    }

    /// Builder-style constructor that validates every rule
    /// up front. Returns the first failure as
    /// [`NormalizerError`].
    pub fn builder(rules: Vec<NormalizationRule>) -> Result<Self> {
        for r in &rules {
            if r.id.trim().is_empty() {
                return Err(NormalizerError::BackendError("rule id is empty".into()));
            }
            if r.field_path.trim().is_empty() {
                return Err(NormalizerError::BackendError(
                    "rule field_path is empty".into(),
                ));
            }
            if let RuleKind::Regex { pattern, .. } = &r.kind {
                regex::Regex::new(pattern)
                    .map_err(|e| NormalizerError::InvalidRegex(e.to_string()))?;
            }
        }
        Ok(Self { rules })
    }

    /// Number of rules in the pipeline.
    #[must_use]
    pub fn len(&self) -> usize {
        self.rules.len()
    }

    /// True if the pipeline has no rules.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// Borrow the rule list (in declaration order).
    #[must_use]
    pub fn rules(&self) -> &[NormalizationRule] {
        &self.rules
    }

    /// Apply the pipeline to `record` and return the
    /// normalized record. The skeleton applies the rules
    /// **in place**; the returned record carries the same
    /// `source_id` + `seq` as the input.
    ///
    /// On the first rule failure, the loop bails and the
    /// error is returned. The record is left partially
    /// mutated (the skeleton does not snapshot; the
    /// production build will either use a copy-on-write
    /// arena or document the partial-state semantics).
    pub fn apply(
        &self,
        source_id: &str,
        seq: u64,
        payload: serde_json::Value,
    ) -> Result<NormalizedRecord> {
        let mut value = payload;
        for rule in &self.rules {
            rule.apply(&mut value)?;
        }
        Ok(NormalizedRecord::new(source_id, seq, value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rule::RuleKind;
    use serde_json::json;

    #[test]
    fn empty_pipeline_returns_input_unchanged() {
        let p = NormalizationPipeline::new();
        let out = p.apply("src", 0, json!({"k": "v"})).expect("ok");
        assert_eq!(out.source_id, "src");
        assert_eq!(out.seq, 0);
        assert_eq!(out.payload, json!({"k": "v"}));
    }

    #[test]
    fn from_rules_keeps_ordering() {
        let rules = vec![
            NormalizationRule::new("r1", "name", RuleKind::Trim),
            NormalizationRule::new("r2", "name", RuleKind::Lowercase),
        ];
        let p = NormalizationPipeline::from_rules(rules.clone());
        assert_eq!(p.rules(), rules.as_slice());
        assert_eq!(p.len(), 2);
        assert!(!p.is_empty());
    }

    #[test]
    fn builder_accepts_trim_lowercase_chain() {
        let rules = vec![
            NormalizationRule::new("r1", "name", RuleKind::Trim),
            NormalizationRule::new("r2", "name", RuleKind::Lowercase),
        ];
        let p = NormalizationPipeline::builder(rules).expect("ok");
        let out = p.apply("s", 1, json!({"name": "  ALICE  "})).expect("ok");
        assert_eq!(out.payload["name"], json!("alice"));
        assert_eq!(out.seq, 1);
    }

    #[test]
    fn builder_rejects_empty_rule_id() {
        let rules = vec![NormalizationRule::new("", "x", RuleKind::Trim)];
        let err = NormalizationPipeline::builder(rules).expect_err("empty id");
        assert!(matches!(err, NormalizerError::BackendError(_)));
    }

    #[test]
    fn builder_rejects_empty_field_path() {
        let rules = vec![NormalizationRule::new("r", "", RuleKind::Trim)];
        let err = NormalizationPipeline::builder(rules).expect_err("empty path");
        assert!(matches!(err, NormalizerError::BackendError(_)));
    }

    #[test]
    fn builder_rejects_invalid_regex() {
        let rules = vec![NormalizationRule::new(
            "r",
            "x",
            RuleKind::Regex {
                pattern: "[unclosed".into(),
                replacement: String::new(),
            },
        )];
        let err = NormalizationPipeline::builder(rules).expect_err("bad regex");
        assert!(matches!(err, NormalizerError::InvalidRegex(_)));
    }

    #[test]
    fn apply_short_circuits_on_first_error() {
        let p = NormalizationPipeline::from_rules(vec![
            NormalizationRule::new("r1", "missing", RuleKind::Trim),
            NormalizationRule::new("r2", "name", RuleKind::Lowercase),
        ]);
        let err = p.apply("s", 0, json!({"name": "X"})).expect_err("missing");
        assert!(matches!(err, NormalizerError::UnknownField(_)));
    }

    #[test]
    fn apply_full_chain_trim_lowercase_regex() {
        let p = NormalizationPipeline::from_rules(vec![
            NormalizationRule::new("r1", "email", RuleKind::Trim),
            NormalizationRule::new("r2", "email", RuleKind::Lowercase),
            NormalizationRule::new(
                "r3",
                "email",
                RuleKind::Regex {
                    pattern: r"@example\.com".into(),
                    replacement: "@example.org".into(),
                },
            ),
        ]);
        let out = p
            .apply("s", 0, json!({"email": "  Foo@Example.COM  "}))
            .expect("ok");
        assert_eq!(out.payload["email"], json!("foo@example.org"));
    }

    #[test]
    fn apply_with_date_and_coalesce() {
        let p = NormalizationPipeline::from_rules(vec![
            NormalizationRule::new(
                "r1",
                "ts",
                RuleKind::Date {
                    input_format: "%Y-%m-%d".into(),
                    output_format: "%Y/%m/%d".into(),
                },
            ),
            NormalizationRule::new(
                "r2",
                "display",
                RuleKind::Coalesce {
                    candidates: vec!["primary".into(), "secondary".into()],
                },
            ),
        ]);
        let out = p
            .apply(
                "s",
                0,
                json!({
                    "ts": "2026-01-15",
                    "primary": null,
                    "secondary": "fallback"
                }),
            )
            .expect("ok");
        assert_eq!(out.payload["ts"], json!("2026/01/15"));
        assert_eq!(out.payload["display"], json!("fallback"));
    }
}
