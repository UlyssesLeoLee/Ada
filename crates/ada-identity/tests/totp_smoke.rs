//! totp_smoke — RFC 6238 verification + recovery code round-trip.
//!
//! Updated for v0.5.0: `verify_code` now takes a string code
//! (matching `totp-rs` `TwoFactorAuth::verify` semantics).

use ada_identity::{recovery::RecoveryStore, totp};

/// Account address used by the TOTP fixtures.
///
/// Assembled from parts on purpose: a literal email address in source is
/// liable to be rewritten to a redaction placeholder by commit/CI tooling,
/// which silently corrupts the fixture. `example.invalid` is reserved by
/// RFC 2606 and can never route anywhere.
const TEST_ACCOUNT: &str = concat!("ada-smoke", "@", "example.invalid");

#[test]
fn totp_generate_then_verify() {
    let s = totp::generate_secret("Ada", TEST_ACCOUNT).expect("secret");
    assert!(!s.base32.is_empty());
    assert!(s.otpauth.starts_with("otpauth://totp/"));
    let now = chrono::Utc::now().timestamp();
    let bytes = totp_rs::Secret::Encoded(s.base32.clone())
        .to_bytes()
        .unwrap();
    let expected = totp_rs::TOTP::new(
        totp_rs::Algorithm::SHA1,
        6,
        1,
        30,
        bytes,
        Some("Ada".into()),
        TEST_ACCOUNT.into(),
    )
    .unwrap()
    .generate_current()
    .unwrap();
    totp::verify_code(&s.base32, &expected, now).expect("verify current");
}

#[test]
fn totp_rejects_empty_secret() {
    let r = totp::verify_code("", "000000", 0);
    assert!(r.is_err());
}

#[test]
fn recovery_codes_are_single_use() {
    let store = RecoveryStore::new();
    let codes = store.generate(3);
    assert_eq!(codes.len(), 3);
    store.redeem(&codes[0]).expect("first redeem");
    assert!(store.redeem(&codes[0]).is_err());
}

#[test]
fn recovery_rejects_unknown_code() {
    let store = RecoveryStore::new();
    let r = store.redeem("nope");
    assert!(r.is_err());
}
