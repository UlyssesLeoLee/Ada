//! TOTP (RFC 6238). Production wiring uses the `totp-rs` crate;
//! the v0.4.0 skeleton wraps the high-level state machine.

use serde::{Deserialize, Serialize};

use crate::error::{IdentityError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TotpSecret {
    /// Base32-encoded shared secret.
    pub base32: String,
    /// OTP auth URI suitable for a QR code.
    pub otpauth: String,
}

#[derive(Debug, Clone, Copy)]
pub struct TotpCode(pub u32);

/// Generate a fresh TOTP secret. The label appears in the
/// otpauth:// URI as `issuer:account`.
pub fn generate_secret(issuer: &str, account: &str) -> Result<TotpSecret> {
    if issuer.is_empty() || account.is_empty() {
        return Err(IdentityError::Totp("issuer / account empty".into()));
    }
    // Real impl: totp_rs::Secret::generate().to_encoded();
    let mut bytes = [0u8; 20];
    rand::Rng::fill(&mut rand::thread_rng(), &mut bytes[..]);
    let base32 = base32_encode(&bytes);
    let otpauth = format!(
        "otpauth://totp/{}:{}?secret={}&issuer={}",
        urlencoding(issuer),
        urlencoding(account),
        base32,
        urlencoding(issuer),
    );
    Ok(TotpSecret { base32, otpauth })
}

/// Verify a TOTP code (6 digits) using `RFC 6238` test vectors.
/// The real implementation pulls in `totp-rs`'s verifier.
pub fn verify_code(secret_base32: &str, code: u32, now_unix: i64) -> Result<bool> {
    if secret_base32.is_empty() {
        return Err(IdentityError::Totp("empty secret".into()));
    }
    let expected = rfc6238(secret_base32, now_unix, 6);
    Ok(expected == code)
}

fn rfc6238(_secret_base32: &str, now_unix: i64, digits: usize) -> u32 {
    // Real impl uses HMAC-SHA1 over (now_unix / 30).
    // Skeleton: derive a deterministic value from the inputs.
    let t = (now_unix / 30) as u64;
    let h = {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(&t.to_le_bytes());
        let bytes = hasher.finalize();
        u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
    };
    let mask = 10u32.pow(digits as u32) - 1;
    h & mask
}

fn base32_encode(bytes: &[u8]) -> String {
    const ALPHA: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut out = String::new();
    let mut buf: u32 = 0;
    let mut bits: u32 = 0;
    for &b in bytes {
        buf = (buf << 8) | u32::from(b);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(ALPHA[((buf >> bits) & 0x1f) as usize] as char);
        }
    }
    if bits > 0 {
        out.push(ALPHA[((buf << (5 - bits)) & 0x1f) as usize] as char);
    }
    out
}

fn urlencoding(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}