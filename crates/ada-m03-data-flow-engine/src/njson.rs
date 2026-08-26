//! [`NJson`] — a thin newtype around `serde_json::Value` that
//! gives the data-flow engine a stable, named entry point for
//! "the standard NJSON value" (D-07).
//!
//! v0.1.0 is intentionally trivial: [`NJson`] is
//! `#[serde(transparent)]` over `serde_json::Value`, so the
//! JSON representation is identical and `From`/`Into` to
//! [`serde_json::Value`] is a no-op. The wrapper exists so:
//!
//! - downstream code can name the type and have a single
//!   place to attach helpers (`null()`, `string()`, ...),
//!   and
//! - a future production build can swap in a stricter
//!   representation without churning call sites that pass
//!   the value through the engine.
//!
//! See [`DOC-MOD-003`](../docs/modules/M-03-data-flow-engine.md)
//! §3.2 for the canonical NJSON schema.

use serde::{Deserialize, Serialize};

/// Newtype around `serde_json::Value` representing a single
/// node in the NJSON data bus.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NJson(pub serde_json::Value);

impl NJson {
    /// Build a `null` `NJson`.
    #[must_use]
    pub fn null() -> Self {
        Self(serde_json::Value::Null)
    }

    /// Build a string `NJson`.
    #[must_use]
    pub fn string(s: impl Into<String>) -> Self {
        Self(serde_json::Value::String(s.into()))
    }

    /// Build a number `NJson` from a `u64`.
    #[must_use]
    pub fn uint(n: u64) -> Self {
        Self(serde_json::Value::Number(serde_json::Number::from(n)))
    }

    /// Build a number `NJson` from an `i64`.
    #[must_use]
    pub fn int(n: i64) -> Self {
        Self(serde_json::Value::Number(serde_json::Number::from(n)))
    }

    /// Build a number `NJson` from an `f64`. `NaN` and
    /// infinity collapse to `null` (JSON does not support
    /// either).
    #[must_use]
    pub fn float(n: f64) -> Self {
        serde_json::Number::from_f64(n)
            .map_or_else(Self::null, |num| Self(serde_json::Value::Number(num)))
    }

    /// Build a boolean `NJson`.
    #[must_use]
    pub const fn bool(b: bool) -> Self {
        Self(serde_json::Value::Bool(b))
    }

    /// Build an empty array `NJson`.
    #[must_use]
    pub fn array() -> Self {
        Self(serde_json::Value::Array(Vec::new()))
    }

    /// Build an empty object `NJson`.
    #[must_use]
    pub fn object() -> Self {
        Self(serde_json::Value::Object(serde_json::Map::new()))
    }

    /// Consume `self` and return the inner
    /// `serde_json::Value`.
    #[must_use]
    pub fn into_value(self) -> serde_json::Value {
        self.0
    }

    /// Borrow the inner `serde_json::Value`.
    #[must_use]
    pub const fn as_value(&self) -> &serde_json::Value {
        &self.0
    }

    /// True if the inner value is `null`.
    #[must_use]
    pub const fn is_null(&self) -> bool {
        matches!(self.0, serde_json::Value::Null)
    }

    /// Canonical lowercase JSON type tag (matches the
    /// `type_name` helper used by the other crates).
    #[must_use]
    pub const fn type_tag(&self) -> &'static str {
        match &self.0 {
            serde_json::Value::Null => "null",
            serde_json::Value::Bool(_) => "boolean",
            serde_json::Value::Number(_) => "number",
            serde_json::Value::String(_) => "string",
            serde_json::Value::Array(_) => "array",
            serde_json::Value::Object(_) => "object",
        }
    }
}

impl Default for NJson {
    fn default() -> Self {
        Self::null()
    }
}

impl From<serde_json::Value> for NJson {
    fn from(v: serde_json::Value) -> Self {
        Self(v)
    }
}

impl From<NJson> for serde_json::Value {
    fn from(n: NJson) -> Self {
        n.0
    }
}

impl From<&str> for NJson {
    fn from(s: &str) -> Self {
        Self::string(s)
    }
}

impl From<String> for NJson {
    fn from(s: String) -> Self {
        Self::string(s)
    }
}

impl From<i64> for NJson {
    fn from(n: i64) -> Self {
        Self::int(n)
    }
}

impl From<u64> for NJson {
    fn from(n: u64) -> Self {
        Self::uint(n)
    }
}

impl From<bool> for NJson {
    fn from(b: bool) -> Self {
        Self::bool(b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_constructor() {
        assert!(NJson::null().is_null());
        assert_eq!(NJson::null().type_tag(), "null");
    }

    #[test]
    fn string_constructor() {
        let n = NJson::string("hello");
        assert_eq!(n.as_value(), &serde_json::json!("hello"));
        assert_eq!(n.type_tag(), "string");
    }

    #[test]
    fn uint_int_float_bool_constructors() {
        assert_eq!(NJson::uint(7).as_value(), &serde_json::json!(7));
        assert_eq!(NJson::int(-3).as_value(), &serde_json::json!(-3));
        assert_eq!(NJson::float(1.5).as_value(), &serde_json::json!(1.5));
        assert_eq!(NJson::bool(true).as_value(), &serde_json::json!(true));
    }

    #[test]
    fn float_nan_collapses_to_null() {
        let n = NJson::float(f64::NAN);
        assert!(n.is_null());
    }

    #[test]
    fn array_and_object_constructors() {
        let a = NJson::array();
        let o = NJson::object();
        assert_eq!(a.type_tag(), "array");
        assert_eq!(o.type_tag(), "object");
    }

    #[test]
    fn into_value_unwraps() {
        let n = NJson::string("x");
        let v: serde_json::Value = n.into_value();
        assert_eq!(v, serde_json::json!("x"));
    }

    #[test]
    fn as_value_borrows() {
        let n = NJson::int(42);
        let v: &serde_json::Value = n.as_value();
        assert_eq!(*v, serde_json::json!(42));
    }

    #[test]
    fn from_conversions() {
        let n: NJson = "hello".into();
        assert_eq!(n.as_value(), &serde_json::json!("hello"));
        let n: NJson = String::from("world").into();
        assert_eq!(n.as_value(), &serde_json::json!("world"));
        let n: NJson = 7i64.into();
        assert_eq!(n.as_value(), &serde_json::json!(7));
        let n: NJson = 8u64.into();
        assert_eq!(n.as_value(), &serde_json::json!(8));
        let n: NJson = true.into();
        assert_eq!(n.as_value(), &serde_json::json!(true));
    }

    #[test]
    fn from_serde_json_value_round_trip() {
        let v = serde_json::json!({"a": 1});
        let n: NJson = v.clone().into();
        let back: serde_json::Value = n.into();
        assert_eq!(back, v);
    }

    #[test]
    fn default_is_null() {
        let n: NJson = NJson::default();
        assert!(n.is_null());
    }

    #[test]
    fn serde_transparent_round_trip() {
        let n = NJson::string("hello");
        let json = serde_json::to_string(&n).expect("serialize");
        // Transparent: the value is serialized as the
        // string, not as `{"NJson":"hello"}`.
        assert_eq!(json, "\"hello\"");
        let back: NJson = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, n);
    }
}
