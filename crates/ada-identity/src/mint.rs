//! JWT minting + verification (RS256). Kid-tagged for JWKS rollover.

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

/// Mint an RS256 JWT. Production wiring uses `ring` / `rsa` to
/// sign; for the v0.4.0 skeleton we emit the JSON claims + a
/// placeholder signature (clearly marked). The api-gateway's JWT
/// validator is configured to accept unsigned tokens at the
/// dev-only listener; the production listener requires real RS256
/// signatures from the JWKS endpoint.
pub fn mint_jwt(
    claims: Claims,
    kid: &str,
    _private_key: &str,
) -> Result<Jwt> {
    let header = serde_json::json!({
        "alg": "RS256",
        "typ": "JWT",
        "kid": kid,
    });
    let body = serde_json::to_string(&claims).map_err(|_| IdentityError::JwtSigning)?;
    let header_b64 = crate::base64util::b64url_encode(
        &serde_json::to_vec(&header).map_err(|_| IdentityError::JwtSigning)?,
    );
    let body_b64 = crate::base64util::b64url_encode(body.as_bytes());
    // Real signing happens in production via the `ring` /
    // `rsa` integration. The skeleton returns a token whose
    // signature segment is empty so the api-gateway's dev-mode
    // listener can parse it without doing crypto.
    let signature_b64 = "";
    Ok(Jwt {
        token: format!("{header_b64}.{body_b64}.{signature_b64}"),
        kid: kid.to_string(),
        algorithm: JwtAlgorithm::Rs256,
    })
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

/// Stub JWT verifier: decodes the body segment and checks the
/// expiry. Production wiring verifies the RS256 signature against
/// the configured JWKS.
pub fn verify_jwt_stub(token: &str) -> Result<Claims> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err(IdentityError::JwtVerification);
    }
    let body_bytes = crate::base64util::b64url_decode(parts[1])
        .map_err(|_| IdentityError::JwtVerification)?;
    let claims: Claims = serde_json::from_slice(&body_bytes)
        .map_err(|_| IdentityError::JwtVerification)?;
    if claims.exp <= Utc::now().timestamp() {
        return Err(IdentityError::JwtVerification);
    }
    Ok(claims)
}