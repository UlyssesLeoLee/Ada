//! `aci_emitter_helper` — Ada ACI emitter helper (per ULYS-191 `§4.3.4` brief v0.1).
//!
//! 1 公开函数 `emit_sample_assertion()`, 与 `IDE1.0` + `RGS` + `IM1.0` + `CATs` pattern 1:1 对齐.
//!
//! 守门:
//! - `#7` `unsafe_code=forbid`: ada-mock/src/lib.rs L25 已设 `#![deny(unsafe_code)]`
//! - `#11` 缺标比错标: git dep 锁 rev=`df28c56`
//! - `#13` W/T/M: emit 公开函数 + 2 单测 + IT 配对
//! - `#24` vendor 中立: 仅 1 git dep aci-emitter (自家)

use aci_emitter::{AciEmitter, ExpectActual, ExpectValueType, Layer, Scope, Severity, Status};

/// Emit 一条 Ada placeholder sample assertion.
///
/// 默认值:
/// - `assertion_id`: `"ada-mock:sample:g-1"`
/// - scope: project=`ada-mock`, module=`sample`
/// - expect: `response_within_ms` = 50, "Ada sample mock should respond within 50ms"
/// - actual: `response_within_ms` = 5, "measured 5ms (placeholder)"
/// - status: PASS
/// - severity: info
///
/// # Returns
///
/// `Assertion` (per aci-emitter v0.1.0)
///
/// # Example
///
/// ```
/// use ada_mock::aci_emitter_helper::emit_sample_assertion;
/// let a = emit_sample_assertion();
/// assert_eq!(a.assertion_id, "ada-mock:sample:g-1");
/// ```
#[must_use]
pub fn emit_sample_assertion() -> aci_emitter::Assertion {
    let em = AciEmitter::new(Layer::It);
    em.build(
        "ada-mock:sample:g-1".to_string(),
        Scope::new(
            "ada-mock".to_string(),
            Some("sample".to_string()),
            None,
            None,
            None,
            None,
        ),
        ExpectActual::new(
            ExpectValueType::ResponseWithinMs,
            serde_json::json!(50),
            "Ada sample mock should respond within 50ms",
        ),
        ExpectActual::new(
            ExpectValueType::ResponseWithinMs,
            serde_json::json!(5),
            "measured 5ms (placeholder)",
        ),
        Status::Pass,
        Severity::Info,
        "actual << expect (10x margin) — placeholder per §4.3.4 brief v0.1",
    )
    .expect("sample assertion build must succeed")
}

/// 公开所有 10 必填字段名常量 (供 IT 与 cross-language parity 测试引用).
///
/// Per `.aci.json` `schema_required_fields`, alphabetic 排序.
pub const REQUIRED_FIELDS: [&str; 10] = [
    "aci_version",
    "actual",
    "assertion_id",
    "captured_at",
    "expect",
    "layer",
    "reasoning",
    "scope",
    "severity",
    "status",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sample_basic() {
        let a = emit_sample_assertion();
        assert_eq!(a.assertion_id, "ada-mock:sample:g-1");
        assert_eq!(a.aci_version, "0.1.0-draft");
        assert_eq!(a.layer, Layer::It);
        assert_eq!(a.status, Status::Pass);
        assert_eq!(a.severity, Severity::Info);
        assert_eq!(a.scope.project, "ada-mock");
        assert_eq!(a.scope.module.as_deref(), Some("sample"));
    }

    #[test]
    fn test_required_fields_sorted() {
        let mut sorted = REQUIRED_FIELDS;
        sorted.sort_unstable();
        for (a, b) in REQUIRED_FIELDS.iter().zip(sorted.iter()) {
            assert_eq!(a, b, "REQUIRED_FIELDS must be in alphabetic order");
        }
    }
}
