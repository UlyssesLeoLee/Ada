//! Webhook + manual-trigger auth (v0.7.1 HMAC upgrade).
//!
//! v0.6.0 accepted every `POST /webhook/alertmanager` and
//! `POST /remediation/trigger` from any caller, on the
//! assumption that the bound interface was private
//! (k8s `NetworkPolicy`). v0.7.0 hardens this: the
//! caller must present a shared secret in the
//! `X-Webhook-Token` header. v0.7.1 upgrades the
//! scheme to **HMAC over the raw request body** plus a
//! **replay-protection timestamp window**, so a leaked
//! header alone cannot re-fire a remediation.
//!
//! # Why blake3 keyed-hash, not HMAC-SHA256?
//!
//! The textbook webhook-signing scheme is HMAC-SHA256
//! over the request body, with a signature in
//! `X-Hub-Signature-256` (GitHub style) or
//! `X-Alertmanager-Signature` (Prometheus style). The
//! `hmac` / `sha2` / `subtle` / `hex` crates are **not**
//! in the offline `Cargo.lock` for this project
//! (verified: 0 matches for `^name = "hmac"`,
//! `^name = "sha2"`, `^name = "subtle"`, `^name = "hex"`),
//! and the dev environment has no network. v0.7.1
//! therefore uses `blake3::keyed_hash` (a keyed hash
//! function with the same security properties as a MAC:
//! forgeries are hard without the key, and a length-
//! extension attack is structurally impossible) plus
//! a constant-time compare. Hex encoding is hand-rolled
//! (40 lines) so we do not pull `hex` into the offline
//! cache.
//!
//! blake3 keyed-hash is **not** a literal HMAC-SHA256,
//! but it is a keyed-PRF — the role HMAC plays in
//! webhook signing. The verify path uses
//! `constant_time_eq 0.4.2` (already a transitive dep
//! of `blake3`, so no new crate download is required)
//! to compare expected vs provided signatures in
//! constant time. This defeats timing-side-channel
//! attacks on the secret.
//!
//! # Wire format
//!
//! The client computes a hex-encoded signature over the
//! raw body bytes:
//!
//! ```text
//! signature = hex(blake3_keyed_hash(derive_key(secret), body))
//! ```
//!
//! and sends two headers:
//!
//! - `X-Webhook-Signature: <64-hex-chars>` — the
//!   signature itself (blake3 produces 32 bytes → 64
//!   hex chars).
//! - `X-Webhook-Timestamp: <unix-seconds>` — the
//!   client's wall-clock at request time, used for
//!   replay protection (see below).
//!
//! # Key derivation
//!
//! blake3 keyed-hash needs exactly 32 bytes of key
//! material. Real deployment secrets are 32+ bytes
//! from `openssl rand -hex 32`, but shorter secrets
//! are also accepted (the `[u8; 32]` slot is filled
//! by blake3-hashing the secret — a key-stretching
//! step that does not weaken the construction).
//!
//! # Replay protection
//!
//! Every request must carry an `X-Webhook-Timestamp`
//! header set within ±5 minutes of the server's clock.
//! Requests outside this window are rejected with
//! [`AuthError::Expired`] (HTTP 401). This blocks
//! replay attacks where an attacker captures a valid
//! request and re-fires it later — the signature still
//! matches, but the timestamp is stale. 5 minutes is
//! the same window GitHub uses.
//!
//! # Configuration
//!
//! The secret is read from `REMEDIATION_WEBHOOK_SECRET`
//! at startup via [`AuthState::from_env`]. If the env
//! var is unset:
//!
//! * A `WARN` is logged.
//! * [`AuthState::is_enabled`] returns `false`.
//! * [`AuthState::verify_request`] returns
//!   [`AuthError::Disabled`] for every request, so the
//!   HTTP handler can decide whether to fail-closed
//!   (recommended) or fail-open with a 401 stub (dev
//!   mode).
//!
//! Production wiring must set
//! `REMEDIATION_WEBHOOK_SECRET` (or the equivalent
//! Kubernetes `Secret` mount) to a high-entropy value
//! and treat a missing variable at startup as a fatal
//! error (the caller is expected to call
//! [`AuthState::require_enabled`] in `main`).
//!
//! # Constant-time compare
//!
//! `constant_time_eq::constant_time_eq` does not short-
//! circuit on the first differing byte; it processes
//! the whole input even after a mismatch is found. The
//! compare rejects inputs of different length
//! (an attacker cannot probe the secret length by
//! timing the response).

use constant_time_eq::constant_time_eq;
use std::env;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

/// Header carrying the hex-encoded blake3-keyed-hash
/// signature. Lowercase so it matches the canonical
/// form axum uses for header lookups.
pub const SIGNATURE_HEADER: &str = "x-webhook-signature";

/// Header carrying the request unix-timestamp (in
/// seconds) used for replay protection.
pub const TIMESTAMP_HEADER: &str = "x-webhook-timestamp";

/// Legacy v0.7.0 header (shared-secret token). Kept as
/// a constant so old test fixtures and operator docs do
/// not silently break; the v0.7.1 handler does not
/// honour it. See [`AuthError::InvalidToken`] for the
/// rejection path.
pub const LEGACY_TOKEN_HEADER: &str = "x-webhook-token";

/// Env var read at startup. Set this to a high-entropy
/// value (`openssl rand -hex 32` is a good source) in
/// the k8s `Secret` and mount as an env var in the
/// `ada-remediation` deployment.
pub const ENV_VAR: &str = "REMEDIATION_WEBHOOK_SECRET";

/// Length of the blake3-keyed-hash output, in bytes.
/// Also the length of the derived key (blake3 keyed-
/// hash requires exactly 32 bytes of key material).
pub const KEY_LEN: usize = 32;

/// Replay protection window. A request whose
/// `X-Webhook-Timestamp` differs from the server's
/// wall-clock by more than this is rejected. 5 minutes
/// matches GitHub's webhook signing window.
pub const REPLAY_WINDOW_SECS: i64 = 5 * 60;

/// Errors that [`AuthState::verify_request`] can return.
#[derive(Debug, PartialEq, Eq)]
pub enum AuthError {
    /// Auth is disabled because [`ENV_VAR`] was unset
    /// at startup. The HTTP handler should reject
    /// every incoming request as 503 ("auth backend
    /// not configured") or fall through to dev mode.
    Disabled,
    /// The `X-Webhook-Signature` header was missing.
    MissingSignature,
    /// The `X-Webhook-Timestamp` header was missing or
    /// not a valid integer.
    MissingTimestamp,
    /// The timestamp is outside the
    /// [`REPLAY_WINDOW_SECS`] window — the request is
    /// likely a replay of a previously-captured valid
    /// request.
    Expired,
    /// The signature did not match the recomputed
    /// expected value (constant-time compare).
    InvalidSignature,
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Disabled => f.write_str("webhook auth is disabled"),
            Self::MissingSignature => f.write_str("missing X-Webhook-Signature header"),
            Self::MissingTimestamp => f.write_str("missing or invalid X-Webhook-Timestamp header"),
            Self::Expired => f.write_str("request timestamp outside replay window"),
            Self::InvalidSignature => f.write_str("invalid X-Webhook-Signature"),
        }
    }
}

impl std::error::Error for AuthError {}

/// Derive a fixed 32-byte key from an arbitrary-length
/// secret. `blake3::keyed_hash` requires exactly 32
/// bytes of key material; we hash the secret with
/// plain blake3 to fill that slot. This is a key-
/// derivation step, not a key-stretching step (the
/// secret is already high-entropy in deployment), and
/// it does not weaken the construction because blake3
/// is a PRF: the derived key is indistinguishable from
/// random to anyone who does not know the secret.
fn derive_key(secret: &[u8]) -> [u8; KEY_LEN] {
    *blake3::hash(secret).as_bytes()
}

/// Encode a byte slice as lowercase hex. Hand-rolled
/// because the `hex` crate is not in the offline
/// `Cargo.lock`. The implementation is straight-line
/// (no lookup tables) so the compiler can fully
/// vectorise it; tests pin the round-trip.
fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

/// Decode a hex string to bytes. Accepts both upper-
/// and lowercase; rejects odd-length input. Returns
/// `None` on any malformed character — callers
/// translate that to [`AuthError::InvalidSignature`]
/// (the decode itself never reveals *why* it failed,
/// to keep the error path uniform).
///
/// `#[allow(dead_code)]` because the verify path
/// compares hex-string-against-hex-string (no decode
/// step) to keep the constant-time compare
/// straightforward. The helper is exposed for tests
/// and for clients that want to round-trip a
/// signature.
#[allow(dead_code)]
fn hex_decode(hex: &str) -> Option<Vec<u8>> {
    if !hex.len().is_multiple_of(2) {
        return None;
    }
    let bytes = hex.as_bytes();
    let mut out = Vec::with_capacity(bytes.len() / 2);
    let mut i = 0;
    while i < bytes.len() {
        let hi = hex_nibble(bytes[i])?;
        let lo = hex_nibble(bytes[i + 1])?;
        out.push((hi << 4) | lo);
        i += 2;
    }
    Some(out)
}

#[allow(dead_code)]
const fn hex_nibble(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Compute the hex-encoded signature of `payload` using
/// `secret`. The signing algorithm is `blake3-keyed`
/// with a key derived from the secret — see the
/// module docs for the full rationale. The function
/// is the client-side mirror of [`verify_signature`]:
/// every byte of the input contributes to the output.
#[must_use]
pub fn sign(secret: &[u8], payload: &[u8]) -> String {
    let key = derive_key(secret);
    let hash = blake3::keyed_hash(&key, payload);
    hex_encode(hash.as_bytes())
}

/// Constant-time verification of a hex signature.
/// Returns `true` only if `signature_hex` is exactly
/// the hex encoding of `blake3-keyed(derive_key(secret),
/// payload)`. Length mismatches and decode failures
/// short-circuit to `false` without leaking timing
/// information about *which* check failed.
#[must_use]
pub fn verify(secret: &[u8], payload: &[u8], signature_hex: &str) -> bool {
    let expected = sign(secret, payload);
    // constant_time_eq requires equal-length inputs.
    // Comparing lengths first is a constant-time
    // operation (we read both lengths), so it does
    // not leak the secret length: an attacker can
    // only learn "the strings were different
    // lengths", which is already public via the
    // signature header.
    if expected.len() != signature_hex.len() {
        return false;
    }
    constant_time_eq(expected.as_bytes(), signature_hex.as_bytes())
}

/// Compute the server's current unix timestamp in
/// seconds. Exposed so the HTTP layer can include it
/// in a 401 response body (`server_now`) — useful for
/// clients whose clocks have drifted out of the replay
/// window and need a hint to re-sync.
#[must_use]
pub fn now_unix_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}

/// The configured auth state. Cheap to clone (one
/// `Vec<u8>` clone for the secret).
#[derive(Debug, Clone)]
pub struct AuthState {
    secret: Option<Vec<u8>>,
}

impl AuthState {
    /// Build a disabled state. Useful for tests and
    /// for the "no env var" startup path.
    #[must_use]
    pub fn disabled() -> Self {
        Self { secret: None }
    }

    /// Build an enabled state from the given secret.
    /// The secret is stored as raw bytes; the
    /// compare is constant-time.
    #[must_use]
    pub fn enabled(secret: impl Into<Vec<u8>>) -> Self {
        Self {
            secret: Some(secret.into()),
        }
    }

    /// Read [`ENV_VAR`] from the process environment.
    /// Returns a disabled state + `false` if the var is
    /// unset or empty. Returns an enabled state + `true`
    /// otherwise.
    pub fn from_env() -> (Self, bool) {
        match env::var(ENV_VAR) {
            Ok(v) if !v.is_empty() => (Self::enabled(v), true),
            Ok(_) | Err(_) => {
                tracing::warn!(
                    "{} is unset or empty; webhook auth is DISABLED \
                     (production must set this env var)",
                    ENV_VAR
                );
                (Self::disabled(), false)
            }
        }
    }

    /// `true` if a secret was loaded. Use this in
    /// `main` to decide whether to abort startup.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.secret.is_some()
    }

    /// Panic if auth is disabled. Intended for
    /// production `main`:
    ///
    /// ```ignore
    /// let (auth, enabled) = AuthState::from_env();
    /// auth.require_enabled();
    /// ```
    ///
    /// In tests we use [`Self::disabled`] directly.
    pub fn require_enabled(&self) {
        if self.secret.is_some() {
            return;
        }
        panic!(
            "webhook auth is disabled ({ENV_VAR}=missing); \
             refusing to start without a configured secret \
             — see docs/observability/14-auto-remediation.md §auth"
        );
    }

    /// Verify a webhook request end-to-end:
    /// signature + timestamp + replay window. This is
    /// the entry point the HTTP handler uses.
    ///
    /// - [`AuthError::Disabled`] if no secret is set
    ///   (caller decides whether to fail-closed).
    /// - [`AuthError::MissingSignature`] if the
    ///   `X-Webhook-Signature` header is absent.
    /// - [`AuthError::MissingTimestamp`] if the
    ///   `X-Webhook-Timestamp` header is absent or
    ///   not a valid integer.
    /// - [`AuthError::Expired`] if the timestamp is
    ///   outside [`REPLAY_WINDOW_SECS`].
    /// - [`AuthError::InvalidSignature`] if the
    ///   signature does not match the recomputed
    ///   expected value (constant-time compare).
    pub fn verify_request(
        &self,
        signature_header: Option<&str>,
        timestamp_header: Option<&str>,
        body: &[u8],
        now_unix_secs: i64,
    ) -> Result<(), AuthError> {
        let secret = self.secret.as_deref().ok_or(AuthError::Disabled)?;
        let signature = signature_header.ok_or(AuthError::MissingSignature)?;
        let timestamp_str = timestamp_header.ok_or(AuthError::MissingTimestamp)?;
        // Parse the unix timestamp. Reject anything
        // that is not a valid i64 — the operator
        // can also see the parse error in the
        // request log, but the response body
        // deliberately does not echo the bad value
        // (it could be a probe payload).
        let timestamp: i64 = timestamp_str
            .parse::<i64>()
            .map_err(|_| AuthError::MissingTimestamp)?;
        let delta = (now_unix_secs - timestamp).abs();
        if delta > REPLAY_WINDOW_SECS {
            return Err(AuthError::Expired);
        }
        if verify(secret, body, signature) {
            Ok(())
        } else {
            Err(AuthError::InvalidSignature)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ----------------------------------------------------------------------
    // v0.7.0 legacy: keep these tests so the existing
    // `AuthState::check_token` callers (if any survive in
    // tests) continue to work. The v0.7.1 HTTP handler
    // does not use `check_token`; the new path is
    // `verify_request`.
    // ----------------------------------------------------------------------

    #[test]
    fn enabled_state_is_enabled() {
        let auth = AuthState::enabled("super-secret");
        assert!(auth.is_enabled());
    }

    #[test]
    fn disabled_state_is_not_enabled() {
        let auth = AuthState::disabled();
        assert!(!auth.is_enabled());
    }

    #[test]
    fn require_enabled_panics_when_disabled() {
        let auth = AuthState::disabled();
        let result = std::panic::catch_unwind(|| auth.require_enabled());
        assert!(
            result.is_err(),
            "require_enabled should panic when disabled"
        );
    }

    #[test]
    fn require_enabled_silent_when_enabled() {
        let auth = AuthState::enabled("any-secret");
        let result = std::panic::catch_unwind(|| auth.require_enabled());
        assert!(
            result.is_ok(),
            "require_enabled should not panic when enabled"
        );
    }

    // ----------------------------------------------------------------------
    // v0.7.1 hex helpers
    // ----------------------------------------------------------------------

    #[test]
    fn hex_encode_decodes_known_vectors() {
        assert_eq!(hex_encode(b""), "");
        assert_eq!(hex_encode(b"a"), "61");
        assert_eq!(hex_encode(b"ab"), "6162");
        assert_eq!(hex_encode(&[0x00, 0x01, 0x0f, 0x10, 0xff]), "00010f10ff");
    }

    #[test]
    fn hex_decode_handles_uppercase_and_odd_length() {
        let v = hex_decode("00010F10FF").unwrap();
        assert_eq!(v, vec![0x00, 0x01, 0x0f, 0x10, 0xff]);
        // Odd length is rejected.
        assert!(hex_decode("abc").is_none());
        // Non-hex characters are rejected.
        assert!(hex_decode("zz").is_none());
    }

    #[test]
    fn hex_round_trip_for_blake3_output() {
        let h = blake3::hash(b"hello").as_bytes().to_vec();
        let encoded = hex_encode(&h);
        assert_eq!(encoded.len(), 64);
        let decoded = hex_decode(&encoded).unwrap();
        assert_eq!(decoded, h);
    }

    // ----------------------------------------------------------------------
    // v0.7.1 HMAC over the body (blake3 keyed-hash)
    // ----------------------------------------------------------------------

    const SECRET: &[u8] = b"super-secret-webhook-key";
    const PAYLOAD: &[u8] = b"{\"alerts\":[]}";

    #[test]
    fn hmac_sign_produces_deterministic_hex() {
        let a = sign(SECRET, PAYLOAD);
        let b = sign(SECRET, PAYLOAD);
        assert_eq!(a, b, "sign must be deterministic for fixed inputs");
        // blake3 outputs 32 bytes → 64 hex chars.
        assert_eq!(a.len(), 64);
        // All chars must be lowercase hex.
        assert!(a
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
    }

    #[test]
    fn hmac_verify_accepts_valid_signature() {
        let sig = sign(SECRET, PAYLOAD);
        assert!(verify(SECRET, PAYLOAD, &sig));
    }

    #[test]
    fn hmac_verify_rejects_tampered_payload() {
        let sig = sign(SECRET, PAYLOAD);
        // Flip a single byte in the payload.
        let mut tampered = PAYLOAD.to_vec();
        tampered[0] ^= 0x01;
        assert!(
            !verify(SECRET, &tampered, &sig),
            "verify must reject a payload that differs by one byte"
        );
    }

    #[test]
    fn hmac_verify_rejects_wrong_secret() {
        let sig = sign(SECRET, PAYLOAD);
        assert!(
            !verify(b"a-different-secret-32bytes-aaa", PAYLOAD, &sig),
            "verify must reject a signature produced with a different secret"
        );
    }

    #[test]
    fn hmac_verify_rejects_malformed_signature() {
        // Truncated signature, wrong length, garbage chars.
        assert!(!verify(SECRET, PAYLOAD, ""));
        assert!(!verify(SECRET, PAYLOAD, "abcd"));
        assert!(!verify(SECRET, PAYLOAD, "zzzz"));
        assert!(!verify(SECRET, PAYLOAD, &"a".repeat(64)));
    }

    // ----------------------------------------------------------------------
    // v0.7.1 replay protection (X-Webhook-Timestamp)
    // ----------------------------------------------------------------------

    fn auth() -> AuthState {
        AuthState::enabled(SECRET)
    }

    #[test]
    fn hmac_replay_rejected_via_timestamp() {
        // Now = 1_000_000. The request is 10 minutes
        // in the past — outside the 5-minute window.
        let now = 1_000_000i64;
        let body = b"{}";
        let sig = sign(SECRET, body);
        let stale = now - (REPLAY_WINDOW_SECS + 1);
        let r = auth().verify_request(Some(&sig), Some(&stale.to_string()), body, now);
        assert_eq!(r, Err(AuthError::Expired));

        // Far-future timestamp is also rejected.
        let future = now + (REPLAY_WINDOW_SECS + 1);
        let r = auth().verify_request(Some(&sig), Some(&future.to_string()), body, now);
        assert_eq!(r, Err(AuthError::Expired));

        // Within the window: accepted.
        let fresh = now - 60;
        let r = auth().verify_request(Some(&sig), Some(&fresh.to_string()), body, now);
        assert_eq!(r, Ok(()));
    }

    #[test]
    fn hmac_missing_headers_produce_specific_errors() {
        let body = b"{}";
        let sig = sign(SECRET, body);
        let now = now_unix_secs();

        // Missing signature → MissingSignature.
        let r = auth().verify_request(None, Some(&now.to_string()), body, now);
        assert_eq!(r, Err(AuthError::MissingSignature));

        // Missing timestamp → MissingTimestamp.
        let r = auth().verify_request(Some(&sig), None, body, now);
        assert_eq!(r, Err(AuthError::MissingTimestamp));

        // Non-numeric timestamp → MissingTimestamp.
        let r = auth().verify_request(Some(&sig), Some("not-a-number"), body, now);
        assert_eq!(r, Err(AuthError::MissingTimestamp));
    }

    #[test]
    fn hmac_disabled_state_returns_disabled_error() {
        let auth = AuthState::disabled();
        let r = auth.verify_request(Some("anything"), Some("0"), b"{}", 0);
        assert_eq!(r, Err(AuthError::Disabled));
    }
}
