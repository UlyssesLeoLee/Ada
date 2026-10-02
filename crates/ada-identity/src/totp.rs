//! TOTP (RFC 6238): time-based one-time passwords, derived as an HMAC
//! over the *shared secret* and the 30-second time step.
//!
//! This is a real RFC 6238 implementation, not a skeleton. It
//! reproduces the published test vectors — see
//! `tests/rfc6238_vectors.rs`, which asserts the full table from RFC
//! 6238 §Appendix B for all three algorithms the RFC defines.
//!
//! The default for [`verify_code`] is HMAC-SHA1. That is not a legacy
//! choice: RFC 4226 §1.2 builds HOTP on HMAC-SHA1, RFC 6238 inherits
//! it, and mainstream authenticator apps provision with SHA-1 unless
//! told otherwise. SHA-256 and SHA-512 are available for apps that
//! negotiate them through the `otpauth://` URI.

use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha1::Sha1;
use sha2::{Sha256, Sha512};

use crate::error::{IdentityError, Result};

/// RFC 6238 §4.1: default time step `X` in seconds.
const TIME_STEP_SECS: i64 = 30;

/// RFC 6238 §4.1: default time origin `T0` is the Unix epoch.
const TIME_ORIGIN_UNIX: i64 = 0;

/// Digit count [`verify_code`] compares against.
const VERIFY_DIGITS: u32 = 6;

/// The algorithm [`verify_code`] derives codes with.
///
/// Bound to a single const so that the `algorithm` parameter
/// [`generate_secret`] writes into the `otpauth://` URI cannot drift
/// away from what the verifier actually checks. A provisioning URI
/// that advertised a different algorithm than the one verified would
/// produce codes no server accepts, or worse, be silently ignored.
const VERIFY_ALGORITHM: TotpAlgorithm = TotpAlgorithm::Sha1;

/// RFC 6238 §5.2 recommends tolerating at most one time step of
/// network delay, so [`verify_code`] accepts the previous, current and
/// next step.
const WINDOW_STEPS: u64 = 1;

/// Largest supported code length. `10^digits` must fit in the `u32`
/// that carries the truncated HMAC.
const MAX_DIGITS: u32 = 9;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TotpSecret {
    /// Base32-encoded shared secret.
    pub base32: String,
    /// OTP auth URI suitable for a QR code.
    pub otpauth: String,
}

#[derive(Debug, Clone, Copy)]
pub struct TotpCode(pub u32);

/// HMAC hash function backing a TOTP code.
///
/// RFC 6238 §1.2 permits any of these in place of HMAC-SHA1; the
/// prover and verifier must agree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TotpAlgorithm {
    /// RFC 4226 / RFC 6238 default. What authenticator apps default to.
    Sha1,
    Sha256,
    Sha512,
}

impl TotpAlgorithm {
    /// The name the `otpauth://` key URI format uses for this
    /// algorithm (Google Authenticator's "Key Uri Format").
    fn otpauth_name(self) -> &'static str {
        match self {
            TotpAlgorithm::Sha1 => "SHA1",
            TotpAlgorithm::Sha256 => "SHA256",
            TotpAlgorithm::Sha512 => "SHA512",
        }
    }
}

/// Generate a fresh TOTP secret. The label appears in the
/// otpauth:// URI as `issuer:account`.
pub fn generate_secret(issuer: &str, account: &str) -> Result<TotpSecret> {
    if issuer.is_empty() || account.is_empty() {
        return Err(IdentityError::Totp("issuer / account empty".into()));
    }
    let mut bytes = [0u8; 20];
    rand::Rng::fill(&mut rand::thread_rng(), &mut bytes[..]);
    let base32 = base32_encode(&bytes);
    // `algorithm` and `digits` are spelled out so a provisioning
    // client never has to guess which variant this server verifies.
    // Both are bound to the same consts the verifier uses.
    let otpauth = format!(
        "otpauth://totp/{}:{}?secret={}&issuer={}&algorithm={}&digits={}",
        urlencoding(issuer),
        urlencoding(account),
        base32,
        urlencoding(issuer),
        VERIFY_ALGORITHM.otpauth_name(),
        VERIFY_DIGITS,
    );
    Ok(TotpSecret { base32, otpauth })
}

/// Compute the TOTP code for `now_unix`.
///
/// This is the provisioning/self-service counterpart to
/// [`verify_code`]: an authenticator app derives the same value from
/// the same secret, so a mismatch means the two sides disagree about
/// the secret, the time step, or the algorithm.
///
/// `digits` must be 1..=9.
pub fn generate_code(
    secret_base32: &str,
    algorithm: TotpAlgorithm,
    digits: u32,
    now_unix: i64,
) -> Result<u32> {
    if !(1..=MAX_DIGITS).contains(&digits) {
        return Err(IdentityError::Totp(format!(
            "unsupported digit count {digits}"
        )));
    }
    let secret = decode_secret(secret_base32)?;
    let step = time_step(now_unix)?;
    otp_for(&secret, algorithm, step, digits)
}

/// Verify a 6-digit TOTP code against `secret_base32` using
/// HMAC-SHA1, the RFC 6238 default.
///
/// Accepts the current time step plus one step either side, per RFC
/// 6238 §5.2 ("at most one time step is allowed as the network
/// delay"). `Ok(false)` means "well-formed request, wrong code";
/// `Err` means the request could not be evaluated at all (empty or
/// malformed secret, pre-epoch timestamp).
///
/// The secret is part of the HMAC key, so two accounts sharing a
/// timestamp never share a code.
pub fn verify_code(secret_base32: &str, code: u32, now_unix: i64) -> Result<bool> {
    if secret_base32.is_empty() {
        return Err(IdentityError::Totp("empty secret".into()));
    }
    let secret = decode_secret(secret_base32)?;
    let step = time_step(now_unix)?;
    // Every candidate step is evaluated. Bailing out on the first
    // match would make the work done depend on which step matched,
    // which is exactly the signal a timing attacker looks for.
    let mut matched = false;
    for candidate in [
        Some(step),
        step.checked_sub(WINDOW_STEPS),
        step.checked_add(WINDOW_STEPS),
    ] {
        let Some(s) = candidate else { continue };
        let expected = otp_for(&secret, VERIFY_ALGORITHM, s, VERIFY_DIGITS)?;
        matched |= ct_eq_u32(expected, code);
    }
    Ok(matched)
}

/// RFC 6238 §4.2: `T = (now - T0) / X`, floored.
fn time_step(now_unix: i64) -> Result<u64> {
    // A pre-epoch timestamp has no valid RFC 6238 time step. Casting
    // with `as` would silently wrap the negative counter into a huge
    // `u64` and derive a code from a step that cannot exist, so
    // reject it outright instead of comparing user input against a
    // garbage-derived value. `div_euclid` is the floor the RFC asks
    // for; the conversion then rejects anything negative.
    u64::try_from((now_unix - TIME_ORIGIN_UNIX).div_euclid(TIME_STEP_SECS))
        .map_err(|_| IdentityError::Totp("pre-epoch timestamp".into()))
}

/// RFC 4226 §5.3 dynamic truncation, shared by all three algorithms.
///
/// The counter is the 8-byte big-endian time step; the message is the
/// only 8 bytes the HMAC sees, which is why the secret has to be the
/// *key*.
macro_rules! hotp {
    ($digest:ty, $secret:expr, $step:expr, $digits:expr) => {{
        let mut mac = <Hmac<$digest>>::new_from_slice($secret)
            .map_err(|_| IdentityError::Totp("invalid HMAC key".into()))?;
        mac.update(&$step.to_be_bytes());
        let out = mac.finalize().into_bytes();
        // Low-order 4 bits of the last byte give the offset; read four
        // big-endian bytes from there, clear the sign bit, then take
        // the code modulo 10^digits.
        let offset = usize::from(out[out.len() - 1] & 0x0f);
        let binary = u32::from_be_bytes([
            out[offset],
            out[offset + 1],
            out[offset + 2],
            out[offset + 3],
        ]) & 0x7fff_ffff;
        Ok(binary % 10u32.pow($digits))
    }};
}

fn otp_for(secret: &[u8], algorithm: TotpAlgorithm, step: u64, digits: u32) -> Result<u32> {
    match algorithm {
        TotpAlgorithm::Sha1 => hotp!(Sha1, secret, step, digits),
        TotpAlgorithm::Sha256 => hotp!(Sha256, secret, step, digits),
        TotpAlgorithm::Sha512 => hotp!(Sha512, secret, step, digits),
    }
}

/// Branch-free equality for the code comparison.
///
/// This only removes the data-dependent branch from a comparison
/// between an attacker-supplied code and a locally derived one. It is
/// not a defence against an attacker who can time the whole call.
fn ct_eq_u32(a: u32, b: u32) -> bool {
    let mut x = a ^ b;
    x |= x >> 1;
    x |= x >> 2;
    x |= x >> 4;
    x |= x >> 8;
    x |= x >> 16;
    x == 0
}

fn decode_secret(secret_base32: &str) -> Result<Vec<u8>> {
    let secret = base32_decode(secret_base32)?;
    if secret.is_empty() {
        return Err(IdentityError::Totp("empty secret".into()));
    }
    Ok(secret)
}

const BASE32_ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

fn base32_encode(bytes: &[u8]) -> String {
    let mut out = String::new();
    let mut buf: u32 = 0;
    let mut bits: u32 = 0;
    for &b in bytes {
        buf = (buf << 8) | u32::from(b);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(BASE32_ALPHABET[((buf >> bits) & 0x1f) as usize] as char);
        }
    }
    if bits > 0 {
        out.push(BASE32_ALPHABET[((buf << (5 - bits)) & 0x1f) as usize] as char);
    }
    out
}

/// Decode an RFC 4648 base32 string.
///
/// Padding is optional: [`base32_encode`] emits padding-free secrets
/// (which is what `otpauth://` URIs carry), while hand-entered
/// secrets from other tools are usually padded. Both decode the same,
/// and lower case is accepted.
fn base32_decode(s: &str) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(s.len() * 5 / 8);
    let mut buf: u32 = 0;
    let mut bits: u32 = 0;
    let mut padded = false;
    for b in s.bytes() {
        if b == b'=' {
            padded = true;
            continue;
        }
        if padded {
            return Err(IdentityError::Totp("base32: data after padding".into()));
        }
        let value = base32_value(b)
            .ok_or_else(|| IdentityError::Totp(format!("base32: invalid byte {b:#04x}")))?;
        buf = (buf << 5) | value;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            // `& 0xff` keeps the shift result inside a `u8`, so the
            // conversion cannot fail.
            out.push(u8::try_from((buf >> bits) & 0xff).expect("masked to 8 bits"));
        }
    }
    // A well-formed encoding leaves fewer than 5 bits over. More than
    // that means a symbol was dropped, and decoding it anyway would
    // silently hand the caller a shorter key than the string claims.
    if bits >= 5 {
        return Err(IdentityError::Totp("base32: truncated input".into()));
    }
    Ok(out)
}

/// Base32 symbol value, or `None` for a byte outside the RFC 4648
/// alphabet. `0`, `1`, `8` and `9` are rejected: they are the classic
/// O vs 0 / I vs 1 transcription errors and base32 has no room for
/// them.
fn base32_value(b: u8) -> Option<u32> {
    match b {
        b'A'..=b'Z' => Some(u32::from(b - b'A')),
        b'a'..=b'z' => Some(u32::from(b - b'a')),
        b'2'..=b'7' => Some(u32::from(b - b'2') + 26),
        _ => None,
    }
}

fn urlencoding(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

#[cfg(test)]
mod tests {
    use super::{base32_decode, base32_encode};

    /// RFC 4648 §10 base32 test vector: "foobar" <-> "MZXW6YTBOI======".
    const FOOBAR_B32: &str = "MZXW6YTBOI";
    const FOOBAR: &[u8] = b"foobar";

    #[test]
    fn base32_encode_matches_rfc4648() {
        assert_eq!(base32_encode(FOOBAR), FOOBAR_B32);
    }

    /// [`base32_encode`] with RFC 4648 `=` padding, which
    /// [`base32_encode`] itself never emits. Secrets arrive from both
    /// sides — this crate generates padding-free, other tools hand out
    /// padded — so the decoder has to survive both.
    fn base32_encode_padded(bytes: &[u8]) -> String {
        let mut s = base32_encode(bytes);
        while !s.len().is_multiple_of(8) {
            s.push('=');
        }
        s
    }

    /// Deterministic, non-uniform sample bytes for a round trip.
    ///
    /// The arithmetic is deliberately `wrapping`: an earlier revision
    /// built these with `u8::try_from(i * 37)`, which overflows once
    /// `i` reaches 8, so the test panicked on its own fixture before
    /// reaching any assertion.
    fn sample_bytes(len: usize) -> Vec<u8> {
        (0..len)
            .map(|i| {
                u8::try_from(i)
                    .expect("len < 256")
                    .wrapping_mul(37)
                    .wrapping_add(11)
            })
            .collect()
    }

    #[test]
    fn base32_round_trips_every_length_1_to_40() {
        // Lengths 5, 10, ... land on a whole number of base32 blocks and
        // need no padding; every other length does. The sweep covers both
        // shapes, plus lengths past the 20-byte secret and past the
        // 32/64-byte RFC 6238 seeds.
        for len in 1..=40usize {
            let bytes = sample_bytes(len);
            let encoded = base32_encode(&bytes);
            assert!(!encoded.contains('='), "encoder must not pad (len {len})");
            let decoded = base32_decode(&encoded).expect("decodes");
            assert_eq!(decoded, bytes, "round trip failed at len {len}");
        }
    }

    #[test]
    fn base32_round_trips_the_padded_form_too() {
        // The encoding a hand-pasted secret from another tool looks like.
        // 6 bytes forces `MZXW6YTBOI======`; 20 bytes (what
        // `generate_secret` mints) forces no padding at all, so the two
        // ends of the range are both covered.
        for len in 1..=40usize {
            let bytes = sample_bytes(len);
            let padded = base32_encode_padded(&bytes);
            assert_eq!(
                padded.len() % 8,
                0,
                "padded form must be a whole block (len {len})"
            );
            let decoded = base32_decode(&padded).expect("decodes padded");
            assert_eq!(decoded, bytes, "padded round trip failed at len {len}");
        }
    }

    #[test]
    fn base32_decode_accepts_padding() {
        assert_eq!(
            base32_decode(&format!("{FOOBAR_B32}======")).expect("padded"),
            FOOBAR
        );
        assert_eq!(base32_decode(FOOBAR_B32).expect("unpadded"), FOOBAR);
    }

    #[test]
    fn base32_decode_accepts_lowercase() {
        assert_eq!(
            base32_decode(&FOOBAR_B32.to_lowercase()).expect("lower"),
            FOOBAR
        );
    }

    #[test]
    fn base32_decode_rejects_invalid_characters() {
        for bad in [
            "MZXW6YTBO0",
            "MZXW6YTBO1",
            "MZXW6YTBO8",
            "MZXW6YTBO9",
            "MZXW6YTBO!",
            "MZ XW6",
        ] {
            assert!(base32_decode(bad).is_err(), "must reject {bad:?}");
        }
    }

    #[test]
    fn base32_decode_rejects_non_ascii() {
        // A multi-byte UTF-8 char is not in the alphabet; it must not
        // be silently skipped.
        assert!(base32_decode("MZXW6YTBO\u{3042}").is_err());
    }

    #[test]
    fn base32_decode_rejects_data_after_padding() {
        assert!(base32_decode("MZXW6==YTO").is_err());
    }

    #[test]
    fn base32_decode_rejects_dropped_symbol() {
        // One symbol short: 45 bits is 5 whole bytes plus a dangling
        // 5-bit group, so the key would silently shrink.
        assert!(base32_decode("MZXW6YTBO").is_err());
    }

    #[test]
    fn base32_decode_empty_is_empty() {
        assert!(base32_decode("").expect("decodes").is_empty());
    }
}
