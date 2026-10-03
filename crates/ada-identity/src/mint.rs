//! JWT minting + verification (RS256). Kid-tagged for JWKS rollover.
//!
//! Both entry points currently fail closed with
//! [`IdentityError::JwtSigningUnavailable`]: no RS256 signer is wired in
//! at this layer, so this module neither mints nor accepts a token. See
//! the individual functions for the reasoning.

use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{IdentityError, Result};

#[derive(Debug, Clone, Copy)]
pub enum JwtAlgorithm {
    Rs256,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub iss: String,
    pub sub: String,
    pub aud: String,
    pub tenant: String,
    pub roles: Vec<String>,
    pub iat: i64,
    pub exp: i64,
}

#[derive(Debug, Clone)]
pub struct Jwt {
    pub token: String,
    pub kid: String,
    pub algorithm: JwtAlgorithm,
}

/// Mint an RS256 JWT.
///
/// **Fails closed.** RS256 signing is not implemented in this build
/// (no `ring` / `rsa` in `Cargo.lock`, deliberately — adding one is a
/// separate, reviewed change), so this returns
/// [`IdentityError::JwtSigningUnavailable`] for every input rather than
/// emitting a token.
///
/// This function previously built a header advertising `"alg": "RS256"`
/// and then set the signature segment to the empty string. That token
/// was well-formed and looked authentic, so it survived casual
/// inspection, but nothing ever signed it: an attacker could take any
/// token, edit `roles` / `tenant`, re-encode, and be accepted —
/// `tenant` being the isolation key for the whole multi-tenant model.
/// A caller that receives an `Err` cannot ship that bypass; a caller
/// that receives a token can. Unverifiable tokens must not leave this
/// function.
///
/// Parameter names are underscore-prefixed because the values are
/// deliberately not read while the signer is absent; the types are
/// unchanged so the call sites that already pass them still compile
/// and will keep compiling when a real signer lands.
// `Claims` carries PII (iss / sub / aud / tenant / roles). Taking it by
// value hands ownership to the signing path so the real v0.5.0 signer can
// drop or zeroize those strings instead of leaving a second live copy in
// the caller's frame — consistent with this crate's "never echo PII" rule
// in `error`. Borrowing would also be a breaking change to a public
// signature for no functional gain.
#[allow(clippy::needless_pass_by_value)]
pub fn mint_jwt(_claims: Claims, _kid: &str, _private_key: &str) -> Result<Jwt> {
    Err(IdentityError::JwtSigningUnavailable)
}

/// Helper for callers: build a default `Claims` with a 1-hour TTL.
pub fn build_default_claims(
    iss: &str,
    sub: &str,
    tenant: &str,
    roles: Vec<String>,
    aud: &str,
) -> Claims {
    let now = Utc::now();
    Claims {
        iss: iss.into(),
        sub: sub.into(),
        aud: aud.into(),
        tenant: tenant.into(),
        roles,
        iat: now.timestamp(),
        exp: (now + Duration::hours(1)).timestamp(),
    }
}

/// Verify a JWT.
///
/// **Fails closed.** Returns [`IdentityError::JwtSigningUnavailable`] for
/// every input, reusing the mint-side variant because the cause is the
/// same one: no RS256 signer is wired in, so no signature can be
/// produced *or* checked.
///
/// This function previously base64url-decoded the payload and checked
/// only `exp` — never a signature, and never `iss` / `aud`. A forged
/// token with an arbitrary `tenant` or an inflated `roles` list passed
/// that check, and callers were free to treat the result as an
/// authenticated principal. Decoding is not verifying: an unverifiable
/// token must not produce a `Claims` value.
pub fn verify_jwt_stub(_token: &str) -> Result<Claims> {
    Err(IdentityError::JwtSigningUnavailable)
}

#[cfg(test)]
mod tests {
    use super::{build_default_claims, mint_jwt, verify_jwt_stub, Claims};
    use crate::error::IdentityError;

    /// Placeholder only. `mint_jwt` never reads it, and nothing in this
    /// crate signs with it. Assembled from parts so no key-shaped literal
    /// is committed.
    const UNUSED_KEY_PLACEHOLDER: &str = concat!("not-a-", "real-key");

    /// `example.invalid` is reserved by RFC 2606 and can never route.
    const TEST_ACCOUNT: &str = concat!("ada-mint", "@", "example.invalid");

    fn claims() -> Claims {
        build_default_claims(
            "https://issuer.invalid",
            TEST_ACCOUNT,
            "tenant-a",
            vec!["reader".to_string()],
            "https://api.invalid",
        )
    }

    #[test]
    fn mint_jwt_fails_closed_instead_of_emitting_an_unsigned_token() {
        let err = mint_jwt(claims(), "kid-1", UNUSED_KEY_PLACEHOLDER)
            .expect_err("mint_jwt must not return a token while signing is unimplemented");
        assert!(
            matches!(err, IdentityError::JwtSigningUnavailable),
            "expected JwtSigningUnavailable, got {err:?}"
        );
    }

    #[test]
    fn mint_jwt_fails_closed_regardless_of_kid_or_key() {
        // The fail-closed path must not hinge on any input. In particular a
        // caller who supplies a real PEM must not be what "unlocks" token
        // issuance, because the signer does not exist to use it.
        for kid in ["", "kid-1", "kid-rotation-suffix"] {
            for key in ["", UNUSED_KEY_PLACEHOLDER] {
                assert!(
                    mint_jwt(claims(), kid, key).is_err(),
                    "mint must fail closed for kid {kid:?}"
                );
            }
        }
    }

    /// The exact attack the previous implementation invited: emit a
    /// well-formed token advertising RS256, rewrite the authorization
    /// claims, re-encode, present it. `tenant` is the isolation key for
    /// the whole multi-tenant model, so this must not verify.
    #[test]
    fn verify_rejects_a_forged_token_with_tampered_roles_and_tenant() {
        let header = serde_json::json!({
            "alg": "RS256",
            "typ": "JWT",
            "kid": "kid-1",
        });
        let forged_body = serde_json::json!({
            "iss": "https://attacker.invalid",
            "sub": TEST_ACCOUNT,
            "aud": "https://api.invalid",
            "tenant": "tenant-victim",
            "roles": ["platform-admin"],
            // Far-future expiry: the old verifier's only check was `exp`,
            // so this token passed it.
            "iat": 0,
            "exp": i64::MAX,
        });
        let token = format!(
            "{}.{}.",
            crate::base64util::b64url_encode(&serde_json::to_vec(&header).expect("header")),
            crate::base64util::b64url_encode(&serde_json::to_vec(&forged_body).expect("body")),
        );

        // Precondition: this really is the three-segment shape the old
        // `verify_jwt_stub` accepted, so the test is meaningful rather
        // than failing for an incidental reason.
        assert_eq!(
            token.split('.').count(),
            3,
            "fixture must keep the JWT three-segment shape"
        );
        assert!(
            token.ends_with('.'),
            "fixture must keep the empty signature segment"
        );

        let err = verify_jwt_stub(&token)
            .expect_err("a token with an empty signature segment must never verify");
        assert!(
            matches!(err, IdentityError::JwtSigningUnavailable),
            "expected JwtSigningUnavailable, got {err:?}"
        );
    }

    #[test]
    fn verify_fails_closed_for_malformed_input() {
        for token in [
            "",
            ".",
            "..",
            "a.b",
            "not-a-jwt",
            "eyJhbGciOiJSUzI1NiJ9.e30.",
        ] {
            assert!(
                verify_jwt_stub(token).is_err(),
                "verify must fail closed for {token:?}"
            );
        }
    }
}
