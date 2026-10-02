//! totp_smoke — TOTP secret generation, code verification and recovery
//! code round-trip.
//!
//! ## Why this file does not use `totp-rs`
//!
//! An earlier revision of this test computed the expected code with the
//! `totp-rs` crate and passed it to `verify_code` as a `&str`. Neither
//! matched the crate as it actually exists: `totp-rs` is not a dependency
//! of `ada-identity` (the manifest defers the real wire-format crypto
//! deps to v0.5.0), and `verify_code` takes a `u32` code, not a string.
//! The file therefore did not compile, and nothing caught it because CI
//! had never been able to run a build.
//!
//! Until `totp-rs` lands in v0.5.0 the independent-reference half of a
//! round-trip cannot be written: the crate's own RFC 6238 step is a
//! private, time-only derivation, so there is no public way to compute an
//! expected code from a secret. These tests cover the contract the public
//! API does expose. The RFC 6238 Appendix D reference vectors stay
//! deferred in `rfc6238_vectors.rs` until that dependency exists.

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
    assert!(s.otpauth.contains("issuer=Ada"), "otpauth: {}", s.otpauth);
    assert!(
        s.otpauth.contains(&format!("secret={}", s.base32)),
        "otpauth must carry the shared secret: {}",
        s.otpauth
    );

    // A freshly generated secret has no valid current code, so the
    // verifier must answer Ok(false) rather than erroring. On this API a
    // wrong code and a rejected request are distinct states.
    let now = chrono::Utc::now().timestamp();
    let r = totp::verify_code(&s.base32, 0, now).expect("verify returns a verdict");
    assert!(!r, "code 0 must not verify");
}

#[test]
fn totp_generate_secret_rejects_empty_inputs() {
    assert!(totp::generate_secret("", TEST_ACCOUNT).is_err());
    assert!(totp::generate_secret("Ada", "").is_err());
    assert!(totp::generate_secret("", "").is_err());
}

#[test]
fn totp_secrets_are_unpredictable() {
    let a = totp::generate_secret("Ada", TEST_ACCOUNT).expect("first");
    let b = totp::generate_secret("Ada", TEST_ACCOUNT).expect("second");
    assert_ne!(a.base32, b.base32, "secrets must not repeat");
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
    store.redeem(&codes[0]).expect("first redeem");
    assert!(store.redeem(&codes[0]).is_err());
}

#[test]
fn recovery_rejects_unknown_code() {
    let store = RecoveryStore::new();
    let r = store.redeem("nope");
    assert!(r.is_err());
}
