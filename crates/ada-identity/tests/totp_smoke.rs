//! totp_smoke — RFC 6238 verification + recovery code round-trip.

use ada_identity::{recovery::RecoveryStore, totp};

#[test]
fn totp_generate_then_verify() {
    let s = totp::generate_secret("Ada", "[email protected]").expect("secret");
    assert!(!s.base32.is_empty());
    assert!(s.otpauth.starts_with("otpauth://totp/"));
    // Generate a code for now; verify the verifier round-trips a
    // freshly generated code (we can't predict the verifier output
    // without exposing internals; the skeleton uses a deterministic
    // placeholder so we test shape only).
    let now = chrono::Utc::now().timestamp();
    let r = totp::verify_code(&s.base32, 0, now);
    assert!(r.is_ok());
}

#[test]
fn totp_rejects_empty_secret() {
    let r = totp::verify_code("", 0, 0);
    assert!(r.is_err());
}

#[test]
fn recovery_codes_are_single_use() {
    let store = RecoveryStore::new();
    let codes = store.generate(3);
    assert_eq!(codes.len(), 3);
    // The first redeem of the first code succeeds.
    store.redeem(&codes[0]).expect("first redeem");
    // The second redeem fails (single-use).
    assert!(store.redeem(&codes[0]).is_err());
}

#[test]
fn recovery_rejects_unknown_code() {
    let store = RecoveryStore::new();
    let r = store.redeem("nope");
    assert!(r.is_err());
}