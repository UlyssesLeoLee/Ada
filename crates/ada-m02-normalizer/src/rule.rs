//! [`NormalizationRule`] and the five [`RuleKind`] variants.
//!
//! A [`NormalizationRule`] is a single, declarative
//! transformation: "take the value at `field_path` and
//! apply `kind`". The pipeline ([`crate::pipeline`]) chains
//! an ordered `Vec<NormalizationRule>` and applies them
//! one after the other.
//!
//! The five [`RuleKind`]s cover the v0.1.0 scope:
//!
//! - [`RuleKind::Trim`] — strip leading / trailing whitespace
//!   from a string field.
//! - [`RuleKind::Lowercase`] — `str::to_lowercase` on a
//!   string field.
//! - [`RuleKind::Regex`] — compile a pattern once, then
//!   apply it as `re.replace_all(value, replacement)`.
//! - [`RuleKind::Date`] — parse the string field with an
//!   input format, then re-emit it with an output format.
//! - [`RuleKind::Coalesce`] — walk a list of candidate
//!   `field_path`s in order and pick the first non-null
//!   value (assigns it to the rule's primary `field_path`).
//!
//! See [`DOC-MOD-002`](../docs/modules/M-02-normalizer.md) §3.3
//! for the full rule schema.

use std::fmt;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::error::{NormalizerError, Result};

/// The five transformation kinds the v0.1.0 pipeline
/// understands. Adding a new kind requires extending this
/// enum **and** the [`apply`](NormalizationRule::apply)
/// method.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RuleKind {
    /// Strip leading and trailing whitespace from a string
    /// field. No configuration.
    Trim,
    /// `str::to_lowercase` on a string field. No
    /// configuration.
    Lowercase,
    /// `re.replace_all(value, replacement)` on a string
    /// field. The `pattern` and `replacement` strings are
    /// required at pipeline-build time; the [`Regex`] is
    /// compiled lazily on the first `apply` call.
    Regex {
        /// The pattern to match.
        pattern: String,
        /// The replacement template (`$0`, `$1`, ...).
        replacement: String,
    },
    /// Parse the field as `input_format` and re-emit it as
    /// `output_format` (both `chrono` format strings, e.g.
    /// `"%Y-%m-%d"`).
    Date {
        /// `chrono` format string for the input value.
        input_format: String,
        /// `chrono` format string for the output value.
        output_format: String,
    },
    /// Walk the candidate `field_path`s in order and pick
    /// the first one whose value is not `null`. The chosen
    /// value is assigned to the rule's primary `field_path`.
    Coalesce {
        /// Candidate field paths in priority order.
        candidates: Vec<String>,
    },
}

impl RuleKind {
    /// Canonical lowercase string tag, mirroring the
    /// `serde(rename_all = "snake_case")` representation.
    #[must_use]
    pub const fn tag(&self) -> &'static str {
        match self {
            Self::Trim => "trim",
            Self::Lowercase => "lowercase",
            Self::Regex { .. } => "regex",
            Self::Date { .. } => "date",
            Self::Coalesce { .. } => "coalesce",
        }
    }
}

impl fmt::Display for RuleKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.tag())
    }
}

/// A single declarative transformation: "apply `kind` to the
/// value at `field_path`". The pipeline applies rules in
/// declaration order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizationRule {
    /// Stable rule id (e.g. `"trim-email"`). Used in
    /// [`NormalizerError::RuleExecutionFailed`] for tracing.
    pub id: String,
    /// The field path this rule targets (dot-separated, e.g.
    /// `"user.email"`). For [`RuleKind::Coalesce`] this is
    /// the destination; the candidate fields live inside the
    /// enum.
    pub field_path: String,
    /// The transformation kind.
    pub kind: RuleKind,
}

impl NormalizationRule {
    /// Build a new rule.
    #[must_use]
    pub fn new(id: impl Into<String>, field_path: impl Into<String>, kind: RuleKind) -> Self {
        Self {
            id: id.into(),
            field_path: field_path.into(),
            kind,
        }
    }

    /// Apply this rule to `record`. The skeleton mutates a
    /// owned `serde_json::Value` (no path-walking library;
    /// v0.1.0 only supports top-level + single-segment nested
    /// fields, e.g. `"user"` or `"user.email"`).
    pub fn apply(&self, record: &mut serde_json::Value) -> Result<()> {
        match &self.kind {
            RuleKind::Trim => apply_trim(self, record),
            RuleKind::Lowercase => apply_lowercase(self, record),
            RuleKind::Regex { .. } => apply_regex(self, record),
            RuleKind::Date { .. } => apply_date(self, record),
            RuleKind::Coalesce { .. } => {
                apply_coalesce(self, record);
                Ok(())
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Path helpers
// ---------------------------------------------------------------------------

/// Read the value at `path` from `record`. Supports top-level
/// (`"email"`) and one-segment nested (`"user.email"`) paths.
fn read_path<'a>(path: &str, record: &'a serde_json::Value) -> Option<&'a serde_json::Value> {
    let mut parts = path.split('.');
    let head = parts.next()?;
    let mut cur = record.get(head)?;
    for seg in parts {
        cur = cur.get(seg)?;
    }
    Some(cur)
}

/// Mutably write `value` at `path` in `record`. Same path
/// shape as [`read_path`].
fn write_path(record: &mut serde_json::Value, path: &str, value: serde_json::Value) -> bool {
    let mut parts = path.split('.');
    let Some(head) = parts.next() else {
        return false;
    };
    if let Some(next) = parts.next() {
        let Some(obj) = record.get_mut(head) else {
            return false;
        };
        if let Some(child) = obj.get_mut(next) {
            *child = value;
            return true;
        }
        // Missing nested segment: create the intermediate
        // object so the write does not silently fail when the
        // parent is an object.
        if let serde_json::Value::Object(map) = obj {
            map.insert(next.to_string(), value);
            return true;
        }
        return false;
    }
    if let serde_json::Value::Object(map) = record {
        map.insert(head.to_string(), value);
        return true;
    }
    false
}

fn type_name(v: &serde_json::Value) -> &'static str {
    match v {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

fn expect_string<'a>(
    rule: &'a NormalizationRule,
    record: &'a serde_json::Value,
) -> std::result::Result<&'a str, NormalizerError> {
    let v = read_path(&rule.field_path, record)
        .ok_or_else(|| NormalizerError::UnknownField(rule.field_path.clone()))?;
    match v {
        serde_json::Value::String(s) => Ok(s.as_str()),
        other => Err(NormalizerError::TypeMismatch {
            field: rule.field_path.clone(),
            expected: "string",
            actual: type_name(other).to_string(),
        }),
    }
}

// ---------------------------------------------------------------------------
// Rule bodies
// ---------------------------------------------------------------------------

fn apply_trim(rule: &NormalizationRule, record: &mut serde_json::Value) -> Result<()> {
    let s = expect_string(rule, record)?;
    let trimmed = s.trim().to_string();
    write_path(record, &rule.field_path, serde_json::Value::String(trimmed));
    Ok(())
}

fn apply_lowercase(rule: &NormalizationRule, record: &mut serde_json::Value) -> Result<()> {
    let s = expect_string(rule, record)?;
    let lowered = s.to_lowercase();
    write_path(record, &rule.field_path, serde_json::Value::String(lowered));
    Ok(())
}

fn apply_regex(rule: &NormalizationRule, record: &mut serde_json::Value) -> Result<()> {
    let (pattern, replacement) = match &rule.kind {
        RuleKind::Regex {
            pattern,
            replacement,
        } => (pattern.as_str(), replacement.as_str()),
        _ => unreachable!("apply_regex called on non-regex rule"),
    };
    let re = Regex::new(pattern).map_err(|e| NormalizerError::InvalidRegex(e.to_string()))?;
    let s = expect_string(rule, record)?;
    let replaced = re.replace_all(s, replacement).into_owned();
    write_path(
        record,
        &rule.field_path,
        serde_json::Value::String(replaced),
    );
    Ok(())
}

fn apply_date(rule: &NormalizationRule, record: &mut serde_json::Value) -> Result<()> {
    let (input_format, output_format) = match &rule.kind {
        RuleKind::Date {
            input_format,
            output_format,
        } => (input_format.as_str(), output_format.as_str()),
        _ => unreachable!("apply_date called on non-date rule"),
    };
    let s = expect_string(rule, record)?;
    let parsed = chrono::NaiveDateTime::parse_from_str(s, input_format)
        .or_else(|_| {
            chrono::NaiveDate::parse_from_str(s, input_format)
                .map(|d| d.and_hms_opt(0, 0, 0).expect("midnight"))
        })
        .map_err(|e| NormalizerError::RuleExecutionFailed {
            rule: rule.id.clone(),
            message: format!("date parse: {e}"),
        })?;
    let out = parsed.format(output_format).to_string();
    write_path(record, &rule.field_path, serde_json::Value::String(out));
    Ok(())
}

fn apply_coalesce(rule: &NormalizationRule, record: &mut serde_json::Value) {
    let candidates = match &rule.kind {
        RuleKind::Coalesce { candidates } => candidates.clone(),
        _ => unreachable!("apply_coalesce called on non-coalesce rule"),
    };
    for cand in &candidates {
        if let Some(v) = read_path(cand, record) {
            if !v.is_null() {
                write_path(record, &rule.field_path, v.clone());
                return;
            }
        }
    }
    // No candidate had a non-null value. Leave the record
    // unchanged; the rule is a no-op in that case.
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn rec() -> serde_json::Value {
        json!({
            "email": "  Foo@Example.COM  ",
            "user": {
                "email": "  Bar@Example.COM  ",
                "name": "  Carol  "
            },
            "ts": "2026-08-26T10:00:00",
            "first": null,
            "second": "from-second"
        })
    }

    #[test]
    fn kind_tag_round_trip() {
        assert_eq!(RuleKind::Trim.tag(), "trim");
        assert_eq!(RuleKind::Lowercase.tag(), "lowercase");
        assert_eq!(
            RuleKind::Regex {
                pattern: "a".into(),
                replacement: "b".into()
            }
            .tag(),
            "regex"
        );
        assert_eq!(
            RuleKind::Date {
                input_format: "%Y-%m-%d".into(),
                output_format: "%Y/%m/%d".into()
            }
            .tag(),
            "date"
        );
        assert_eq!(
            RuleKind::Coalesce {
                candidates: vec!["a".into()]
            }
            .tag(),
            "coalesce"
        );
    }

    #[test]
    fn kind_display_uses_tag() {
        assert_eq!(RuleKind::Trim.to_string(), "trim");
    }

    #[test]
    fn trim_top_level() {
        let mut r = rec();
        let rule = NormalizationRule::new("r", "email", RuleKind::Trim);
        rule.apply(&mut r).unwrap();
        assert_eq!(r["email"], json!("Foo@Example.COM"));
    }

    #[test]
    fn trim_nested_field() {
        let mut r = rec();
        let rule = NormalizationRule::new("r", "user.name", RuleKind::Trim);
        rule.apply(&mut r).unwrap();
        assert_eq!(r["user"]["name"], json!("Carol"));
    }

    #[test]
    fn lowercase_top_level() {
        let mut r = rec();
        let rule = NormalizationRule::new("r", "email", RuleKind::Lowercase);
        rule.apply(&mut r).unwrap();
        assert_eq!(r["email"], json!("  foo@example.com  "));
    }

    #[test]
    fn lowercase_after_trim_pipeline_shape() {
        // Build a 2-rule sequence by hand and apply in
        // order. The skeleton keeps the chain inside the
        // pipeline module, but the rule interface is the
        // same.
        let mut r = json!({ "email": "  FOO@Example.com  " });
        NormalizationRule::new("r1", "email", RuleKind::Trim)
            .apply(&mut r)
            .unwrap();
        NormalizationRule::new("r2", "email", RuleKind::Lowercase)
            .apply(&mut r)
            .unwrap();
        assert_eq!(r["email"], json!("foo@example.com"));
    }

    #[test]
    fn regex_replaces_match() {
        let mut r = json!({ "phone": "555-1234" });
        let rule = NormalizationRule::new(
            "r",
            "phone",
            RuleKind::Regex {
                pattern: r"\d".into(),
                replacement: "X".into(),
            },
        );
        rule.apply(&mut r).unwrap();
        assert_eq!(r["phone"], json!("XXX-XXXX"));
    }

    #[test]
    fn regex_invalid_pattern_surfaces_invalid_regex() {
        let mut r = json!({ "x": "y" });
        let rule = NormalizationRule::new(
            "r",
            "x",
            RuleKind::Regex {
                pattern: "[unclosed".into(),
                replacement: String::new(),
            },
        );
        let err = rule.apply(&mut r).expect_err("bad");
        assert!(matches!(err, NormalizerError::InvalidRegex(_)));
    }

    #[test]
    fn date_reformats_value() {
        let mut r = json!({ "ts": "2026-08-26" });
        let rule = NormalizationRule::new(
            "r",
            "ts",
            RuleKind::Date {
                input_format: "%Y-%m-%d".into(),
                output_format: "%Y/%m/%d".into(),
            },
        );
        rule.apply(&mut r).unwrap();
        assert_eq!(r["ts"], json!("2026/08/26"));
    }

    #[test]
    fn date_unparseable_surfaces_rule_execution_failed() {
        let mut r = json!({ "ts": "not-a-date" });
        let rule = NormalizationRule::new(
            "r",
            "ts",
            RuleKind::Date {
                input_format: "%Y-%m-%d".into(),
                output_format: "%Y/%m/%d".into(),
            },
        );
        let err = rule.apply(&mut r).expect_err("bad");
        assert!(matches!(err, NormalizerError::RuleExecutionFailed { .. }));
    }

    #[test]
    fn coalesce_picks_first_non_null() {
        let mut r = rec();
        let rule = NormalizationRule::new(
            "r",
            "second",
            RuleKind::Coalesce {
                candidates: vec!["first".into(), "second".into()],
            },
        );
        rule.apply(&mut r).unwrap();
        assert_eq!(r["second"], json!("from-second"));
    }

    #[test]
    fn coalesce_into_missing_field_creates_it() {
        let mut r = json!({ "first": null, "second": "x" });
        let rule = NormalizationRule::new(
            "r",
            "result",
            RuleKind::Coalesce {
                candidates: vec!["first".into(), "second".into()],
            },
        );
        rule.apply(&mut r).unwrap();
        assert_eq!(r["result"], json!("x"));
    }

    #[test]
    fn unknown_field_surfaces_unknown_field_error() {
        let mut r = json!({});
        let rule = NormalizationRule::new("r", "missing", RuleKind::Trim);
        let err = rule.apply(&mut r).expect_err("missing");
        assert!(matches!(err, NormalizerError::UnknownField(_)));
    }

    #[test]
    fn type_mismatch_surfaces_type_mismatch_error() {
        let mut r = json!({ "n": 7 });
        let rule = NormalizationRule::new("r", "n", RuleKind::Trim);
        let err = rule.apply(&mut r).expect_err("type");
        assert!(matches!(err, NormalizerError::TypeMismatch { .. }));
    }
}
