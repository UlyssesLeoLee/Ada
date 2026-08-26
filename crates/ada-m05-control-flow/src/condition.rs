//! [`Condition`] — boolean expressions evaluated against a
//! shared `context: HashMap<String, Value>`.
//!
//! The v0.1.0 surface supports seven operators:
//!
//! - [`Condition::Eq`] / [`Condition::Ne`] — equality on
//!   JSON values
//! - [`Condition::Lt`] / [`Condition::Gt`] — numeric
//!   ordering (any other JSON type returns `false`)
//! - [`Condition::Contains`] — left contains right
//!   (string `contains` substring, or array `contains`
//!   element)
//! - [`Condition::And`] / [`Condition::Or`] /
//!   [`Condition::Not`] — short-circuit boolean logic
//!
//! `evaluate` returns [`ExecutorError::ConditionError`] when
//! a field path in the condition is not present in the
//! context. Numeric coercion is **not** performed; `"7"` is
//! a string, not the number 7.
//!
//! See [`DOC-MOD-005`](../docs/modules/M-05-control-flow.md)
//! §3.3 for the canonical condition grammar.

use std::cmp::Ordering;
use std::collections::HashMap;

use serde_json::Value;

use crate::error::{ExecutorError, Result};

/// A boolean expression evaluated against a context.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Condition {
    /// `field == literal` or `field == field`. Both sides
    /// are JSON values; the comparison is structural.
    Eq {
        /// Left-hand side field name.
        left: String,
        /// Right-hand side value (literal or field name).
        right: Value,
    },
    /// `field != literal`. Inverse of [`Condition::Eq`].
    Ne {
        /// Left-hand side field name.
        left: String,
        /// Right-hand side value.
        right: Value,
    },
    /// `field < literal`. Numeric ordering; non-numeric
    /// values yield `false`.
    Lt {
        /// Left-hand side field name.
        left: String,
        /// Right-hand side numeric value.
        right: f64,
    },
    /// `field > literal`. Numeric ordering; non-numeric
    /// values yield `false`.
    Gt {
        /// Left-hand side field name.
        left: String,
        /// Right-hand side numeric value.
        right: f64,
    },
    /// `field contains literal`. String substring or array
    /// element membership.
    Contains {
        /// Left-hand side field name.
        left: String,
        /// Right-hand side value (string substring or array
        /// element).
        right: Value,
    },
    /// Short-circuit logical AND.
    And {
        /// Left operand.
        left: Box<Condition>,
        /// Right operand.
        right: Box<Condition>,
    },
    /// Short-circuit logical OR.
    Or {
        /// Left operand.
        left: Box<Condition>,
        /// Right operand.
        right: Box<Condition>,
    },
    /// Logical NOT.
    Not {
        /// Inner operand.
        inner: Box<Condition>,
    },
}

impl Condition {
    /// Evaluate this condition against `context`. Returns
    /// `Err` only when a field path is missing; logical
    /// operations never error (they just yield `false`).
    pub fn evaluate(&self, context: &HashMap<String, Value>) -> Result<bool> {
        match self {
            Self::Eq { left, right } => {
                let lhs = lookup(context, left)?;
                Ok(values_eq(lhs, right))
            }
            Self::Ne { left, right } => {
                let lhs = lookup(context, left)?;
                Ok(!values_eq(lhs, right))
            }
            Self::Lt { left, right } => {
                let lhs = lookup(context, left)?;
                Ok(numeric_cmp(lhs, *right) == Some(Ordering::Less))
            }
            Self::Gt { left, right } => {
                let lhs = lookup(context, left)?;
                Ok(numeric_cmp(lhs, *right) == Some(Ordering::Greater))
            }
            Self::Contains { left, right } => {
                let lhs = lookup(context, left)?;
                Ok(contains(lhs, right))
            }
            Self::And { left, right } => {
                if !left.evaluate(context)? {
                    return Ok(false);
                }
                right.evaluate(context)
            }
            Self::Or { left, right } => {
                if left.evaluate(context)? {
                    return Ok(true);
                }
                right.evaluate(context)
            }
            Self::Not { inner } => Ok(!inner.evaluate(context)?),
        }
    }
}

fn lookup<'a>(context: &'a HashMap<String, Value>, field: &str) -> Result<&'a Value> {
    context
        .get(field)
        .ok_or_else(|| ExecutorError::ConditionError(format!("missing field: {field}")))
}

fn values_eq(a: &Value, b: &Value) -> bool {
    a == b
}

fn numeric_cmp(a: &Value, b: f64) -> Option<std::cmp::Ordering> {
    let af = a.as_f64()?;
    af.partial_cmp(&b)
}

fn contains(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::String(s), Value::String(t)) => s.contains(t.as_str()),
        (Value::Array(arr), needle) => arr.iter().any(|v| v == needle),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ctx(pairs: &[(&str, Value)]) -> HashMap<String, Value> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), v.clone()))
            .collect()
    }

    #[test]
    fn eq_matches_same_value() {
        let c = Condition::Eq {
            left: "k".into(),
            right: json!("v"),
        };
        assert!(c.evaluate(&ctx(&[("k", json!("v"))])).unwrap());
    }

    #[test]
    fn eq_rejects_different_value() {
        let c = Condition::Eq {
            left: "k".into(),
            right: json!("x"),
        };
        assert!(!c.evaluate(&ctx(&[("k", json!("v"))])).unwrap());
    }

    #[test]
    fn eq_missing_field_errors() {
        let c = Condition::Eq {
            left: "missing".into(),
            right: json!("v"),
        };
        let err = c.evaluate(&ctx(&[])).unwrap_err();
        assert!(matches!(err, ExecutorError::ConditionError(_)));
    }

    #[test]
    fn ne_inverts_eq() {
        let c = Condition::Ne {
            left: "k".into(),
            right: json!("x"),
        };
        assert!(c.evaluate(&ctx(&[("k", json!("v"))])).unwrap());
        let c = Condition::Ne {
            left: "k".into(),
            right: json!("v"),
        };
        assert!(!c.evaluate(&ctx(&[("k", json!("v"))])).unwrap());
    }

    #[test]
    fn lt_gt_numeric_ordering() {
        let lt = Condition::Lt {
            left: "n".into(),
            right: 10.0,
        };
        let gt = Condition::Gt {
            left: "n".into(),
            right: 10.0,
        };
        assert!(lt.evaluate(&ctx(&[("n", json!(5))])).unwrap());
        assert!(!lt.evaluate(&ctx(&[("n", json!(15))])).unwrap());
        assert!(gt.evaluate(&ctx(&[("n", json!(15))])).unwrap());
        assert!(!gt.evaluate(&ctx(&[("n", json!(5))])).unwrap());
    }

    #[test]
    fn lt_on_non_numeric_yields_false() {
        let c = Condition::Lt {
            left: "n".into(),
            right: 10.0,
        };
        assert!(!c.evaluate(&ctx(&[("n", json!("x"))])).unwrap());
    }

    #[test]
    fn contains_string_substring() {
        let c = Condition::Contains {
            left: "s".into(),
            right: json!("ell"),
        };
        assert!(c.evaluate(&ctx(&[("s", json!("hello"))])).unwrap());
    }

    #[test]
    fn contains_array_membership() {
        let c = Condition::Contains {
            left: "arr".into(),
            right: json!(2),
        };
        assert!(c.evaluate(&ctx(&[("arr", json!([1, 2, 3]))])).unwrap());
        let c = Condition::Contains {
            left: "arr".into(),
            right: json!(7),
        };
        assert!(!c.evaluate(&ctx(&[("arr", json!([1, 2, 3]))])).unwrap());
    }

    #[test]
    fn and_or_not_compose() {
        let c = Condition::And {
            left: Box::new(Condition::Eq {
                left: "a".into(),
                right: json!(1),
            }),
            right: Box::new(Condition::Eq {
                left: "b".into(),
                right: json!(2),
            }),
        };
        assert!(c
            .evaluate(&ctx(&[("a", json!(1)), ("b", json!(2))]))
            .unwrap());

        let c = Condition::Or {
            left: Box::new(Condition::Eq {
                left: "a".into(),
                right: json!(0),
            }),
            right: Box::new(Condition::Eq {
                left: "b".into(),
                right: json!(2),
            }),
        };
        assert!(c
            .evaluate(&ctx(&[("a", json!(1)), ("b", json!(2))]))
            .unwrap());

        let c = Condition::Not {
            inner: Box::new(Condition::Eq {
                left: "a".into(),
                right: json!(1),
            }),
        };
        assert!(!c.evaluate(&ctx(&[("a", json!(1))])).unwrap());
        assert!(c.evaluate(&ctx(&[("a", json!(2))])).unwrap());
    }
}
