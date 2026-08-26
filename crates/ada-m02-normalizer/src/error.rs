//! Error surface for the data normalizer.
//!
//! [`NormalizerError`] is the single error type returned by
//! every public function in this crate. The v0.1.0 skeleton
//! keeps the enum at five variants covering the common
//! failure modes of a rule-driven normalization pipeline:
//!
//! | Variant                | Trigger                                                |
//! |------------------------|--------------------------------------------------------|
//! | `UnknownField`         | `field_path` references a field that does not exist.   |
//! | `RuleExecutionFailed`  | A rule's body threw / returned an error.               |
//! | `TypeMismatch`         | The value at `field_path` has the wrong JSON type.     |
//! | `InvalidRegex`         | A `Regex` rule's pattern failed to compile.            |
//! | `BackendError`         | Underlying store / driver / parser failed.             |
//!
//! See [`DOC-MOD-002`](../docs/modules/M-02-normalizer.md) §3.4
//! for the full validation pipeline.

use thiserror::Error;

/// Failure modes surfaced by the normalizer.
#[derive(Debug, Error)]
pub enum NormalizerError {
    /// The rule's `field_path` does not exist on the record.
    /// The pipeline keeps going for the other rules but
    /// surfaces the missing field so callers can log it.
    #[error("unknown field: {0}")]
    UnknownField(String),

    /// A rule's body returned an error (e.g. the `Date` rule
    /// could not parse the input string).
    #[error("rule execution failed (rule={rule}): {message}")]
    RuleExecutionFailed {
        /// Rule id that failed.
        rule: String,
        /// Human-readable error message from the rule body.
        message: String,
    },

    /// The value at `field_path` has the wrong JSON type
    /// (e.g. `Trim` on a number).
    #[error("type mismatch at {field}: expected {expected}, got {actual}")]
    TypeMismatch {
        /// Field path where the mismatch happened.
        field: String,
        /// Expected JSON type (e.g. `"string"`).
        expected: &'static str,
        /// Actual JSON type (e.g. `"number"`).
        actual: String,
    },

    /// A `Regex` rule's pattern failed to compile. Surfaced
    /// at pipeline-build time so callers fail fast on a bad
    /// config instead of at apply time.
    #[error("invalid regex: {0}")]
    InvalidRegex(String),

    /// Underlying store / driver / parser failed.
    #[error("backend error: {0}")]
    BackendError(String),
}

/// `Result` alias for fallible normalizer operations.
pub type Result<T> = core::result::Result<T, NormalizerError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_field_display() {
        let e = NormalizerError::UnknownField("user.email".into());
        assert_eq!(e.to_string(), "unknown field: user.email");
    }

    #[test]
    fn rule_execution_failed_display() {
        let e = NormalizerError::RuleExecutionFailed {
            rule: "r-1".into(),
            message: "parse error".into(),
        };
        assert_eq!(
            e.to_string(),
            "rule execution failed (rule=r-1): parse error"
        );
    }

    #[test]
    fn type_mismatch_display() {
        let e = NormalizerError::TypeMismatch {
            field: "user.email".into(),
            expected: "string",
            actual: "number".into(),
        };
        assert_eq!(
            e.to_string(),
            "type mismatch at user.email: expected string, got number"
        );
    }

    #[test]
    fn invalid_regex_display() {
        let e = NormalizerError::InvalidRegex("[unclosed".into());
        assert_eq!(e.to_string(), "invalid regex: [unclosed");
    }

    #[test]
    fn backend_error_display() {
        let e = NormalizerError::BackendError("serde_json: unexpected EOF".into());
        assert_eq!(e.to_string(), "backend error: serde_json: unexpected EOF");
    }

    #[test]
    fn result_alias_carries_error() {
        let ok: Result<i32> = Ok(7);
        let err: Result<i32> = Err(NormalizerError::UnknownField("x".into()));
        assert!(matches!(ok, Ok(7)));
        assert!(err.is_err());
    }

    #[test]
    fn error_is_send_sync_and_static() {
        fn assert_send_sync_static<E: std::error::Error + Send + Sync + 'static>(_: &E) {}
        let e = NormalizerError::BackendError("x".into());
        assert_send_sync_static(&e);
    }
}
