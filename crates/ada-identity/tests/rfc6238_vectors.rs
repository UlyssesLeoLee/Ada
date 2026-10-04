//! RFC 6238 test vectors.
//!
//! ## Source of the expected values
//!
//! RFC 6238, "TOTP: Time-Based One-Time Password Algorithm", the
//! appendix titled "Test Vectors", Table 1. Fetched from
//! <https://www.rfc-editor.org/rfc/rfc6238.txt> on 2026-10-02.
//!
//! Note the appendix letter: the vector table is in RFC 6238's
//! "Test Vectors" appendix, not "Appendix D" — the widely copied
//! "Appendix D" label belongs to a different document. The contents
//! are the ones usually meant: the ASCII secret `"1234567890..."`,
//! 8-digit codes, and timestamps 59 … 20000000000.
//!
//! Every value below was additionally cross-checked by recomputing it
//! with an independent HMAC implementation (`CPython` `hmac` +
//! `hashlib`) before being written here, so the table is verified
//! rather than transcribed on faith. All 18 values matched.
//!
//! ## The secrets
//!
//! RFC 6238 Appendix A's `main()` declares the three seeds as hex
//! strings; Appendix B's Table 1 is the output for those same seeds.
//! They are 20, 32 and 64 ASCII bytes respectively:
//!
//! | Mode   | Seed |
//! |--------|------|
//! | SHA1   | `"12345678901234567890"` |
//! | SHA256 | `"12345678901234567890"` + `"123456789012"` |
//! | SHA512 | `"12345678901234567890"` x3 + `"1234"` |
//!
//! The verifier's public API takes base32, so the constants below are
//! the RFC 4648 base32 (padding-stripped) of those exact byte strings.

use ada_identity::totp::{self, TotpAlgorithm};

/// base32("12345678901234567890") — 20 bytes, HMAC-SHA1.
const SHA1_SECRET: &str = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";
/// base32 of the 32-byte `"1234567890..." x1 + "123456789012"` — HMAC-SHA256.
const SHA256_SECRET: &str = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQGEZA";
/// base32 of the 64-byte `"1234567890..." x3 + "1234"` — HMAC-SHA512.
const SHA512_SECRET: &str = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQGEZDGNA";

/// The six timestamps in RFC 6238 Table 1.
const TIMES: [i64; 6] = [
    59,
    1_111_111_109,
    1_111_111_111,
    1_234_567_890,
    2_000_000_000,
    20_000_000_000,
];

/// RFC 6238 Table 1, `SHA1` column. `07081804` is written as an
/// integer, so the leading zero the RFC prints is not represented.
const SHA1_CODES: [u32; 6] = [
    94_287_082, 7_081_804, 14_050_471, 89_005_924, 69_279_037, 65_353_130,
];

/// RFC 6238 Table 1, `SHA256` column.
const SHA256_CODES: [u32; 6] = [
    46_119_246, 68_084_774, 67_062_674, 91_819_424, 90_698_825, 77_737_706,
];

/// RFC 6238 Table 1, `SHA512` column.
const SHA512_CODES: [u32; 6] = [
    90_693_936, 25_091_201, 99_943_326, 93_441_116, 38_618_901, 47_863_826,
];

/// The same six instants, truncated to the 6 digits `verify_code`
/// compares. Not in the RFC — derived from the Table 1 secret and
/// timestamps with an independent HMAC implementation (`CPython`
/// `hmac`), so that the public API is checked against something
/// other than the code under test.
const SHA1_CODES_6_DIGIT: [u32; 6] = [287_082, 81_804, 50_471, 5_924, 279_037, 353_130];

fn assert_vectors(secret: &str, algorithm: TotpAlgorithm, expected: &[u32; 6], label: &str) {
    for (&now, &want) in TIMES.iter().zip(expected.iter()) {
        let got = totp::generate_code(secret, algorithm, 8, now)
            .unwrap_or_else(|e| panic!("{label} generate_code({now}) failed: {e}"));
        assert_eq!(got, want, "{label} code mismatch at t={now}");
    }
}

#[test]
fn appendix_sha1_vectors() {
    assert_vectors(SHA1_SECRET, TotpAlgorithm::Sha1, &SHA1_CODES, "sha1");
}

#[test]
fn appendix_sha256_vectors() {
    assert_vectors(
        SHA256_SECRET,
        TotpAlgorithm::Sha256,
        &SHA256_CODES,
        "sha256",
    );
}

#[test]
fn appendix_sha512_vectors() {
    assert_vectors(
        SHA512_SECRET,
        TotpAlgorithm::Sha512,
        &SHA512_CODES,
        "sha512",
    );
}

#[test]
fn verify_code_accepts_sha1_codes_at_each_vector_timestamp() {
    for (&now, &want) in TIMES.iter().zip(SHA1_CODES_6_DIGIT.iter()) {
        assert!(
            totp::verify_code(SHA1_SECRET, want, now).expect("verdict"),
            "verify_code must accept {want} at t={now}"
        );
    }
}

#[test]
fn verify_code_rejects_a_wrong_code() {
    let now = 1_234_567_890;
    let right = SHA1_CODES_6_DIGIT[3];
    let wrong = (right + 1) % 1_000_000;
    assert!(!totp::verify_code(SHA1_SECRET, wrong, now).expect("verdict"));
}

#[test]
fn verify_code_rejects_code_from_another_secret() {
    // Regression guard for the bug these vectors were deferred past:
    // the code used to be derived from the clock alone, so every
    // secret produced the same value and this assertion passed.
    let now = 1_234_567_890;
    let mine = SHA1_CODES_6_DIGIT[3];
    assert!(totp::verify_code(SHA1_SECRET, mine, now).expect("verdict"));
    assert!(!totp::verify_code(SHA256_SECRET, mine, now).expect("verdict"));
}

#[test]
fn different_secrets_produce_different_codes_at_the_same_instant() {
    // The direct regression test: with the secret ignored, both arms
    // returned an identical value and this failed.
    for &now in &TIMES {
        let a = totp::generate_code(SHA1_SECRET, TotpAlgorithm::Sha1, 8, now).expect("a");
        let b = totp::generate_code(SHA256_SECRET, TotpAlgorithm::Sha1, 8, now).expect("b");
        assert_ne!(a, b, "two secrets collided at t={now}");
    }
}

#[test]
fn same_secret_produces_different_codes_in_different_steps() {
    let base = 1_234_567_890;
    let a = totp::generate_code(SHA1_SECRET, TotpAlgorithm::Sha1, 8, base).expect("base");
    let b = totp::generate_code(SHA1_SECRET, TotpAlgorithm::Sha1, 8, base + 30).expect("next");
    assert_ne!(a, b);
}

#[test]
fn verify_code_allows_one_step_of_drift_either_way() {
    // RFC 6238 §5.2: at most one time step of network delay.
    let now = 1_234_567_890;
    for offset in [-30i64, 0, 30] {
        let code =
            totp::generate_code(SHA1_SECRET, TotpAlgorithm::Sha1, 6, now + offset).expect("code");
        assert!(
            totp::verify_code(SHA1_SECRET, code, now).expect("verdict"),
            "code from offset {offset}s must verify"
        );
    }
}

#[test]
fn verify_code_rejects_two_steps_of_drift() {
    let now = 1_234_567_890;
    for offset in [-60i64, -90, 60, 90] {
        let code =
            totp::generate_code(SHA1_SECRET, TotpAlgorithm::Sha1, 6, now + offset).expect("code");
        assert!(
            !totp::verify_code(SHA1_SECRET, code, now).expect("verdict"),
            "code from offset {offset}s must not verify"
        );
    }
}

#[test]
fn verify_code_rejects_malformed_secret() {
    let now = 1_234_567_890;
    for bad in [
        "not base32!",
        "MZXW6YTBO0",
        "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOQ0",
    ] {
        assert!(
            totp::verify_code(bad, 287_082, now).is_err(),
            "malformed secret {bad:?} must be an error, not a silent Ok(false)"
        );
    }
}

#[test]
fn verify_code_rejects_empty_and_pre_epoch() {
    assert!(totp::verify_code("", 287_082, 1_234_567_890).is_err());
    assert!(totp::verify_code(SHA1_SECRET, 287_082, -1).is_err());
}

#[test]
fn generate_code_rejects_out_of_range_digit_counts() {
    let now = 1_234_567_890;
    assert!(totp::generate_code(SHA1_SECRET, TotpAlgorithm::Sha1, 0, now).is_err());
    assert!(totp::generate_code(SHA1_SECRET, TotpAlgorithm::Sha1, 10, now).is_err());
    assert!(totp::generate_code(SHA1_SECRET, TotpAlgorithm::Sha1, 9, now).is_ok());
}
