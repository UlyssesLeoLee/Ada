//! JWKS endpoint. Production wiring reads the public key from the
//! configured `IDENTITY_JWT_PRIVATE_KEY` (PEM). For the v0.4.0
//! skeleton we publish the kid; the api-gateway's dev-mode
//! listener does not verify the signature.

use crate::error::Result;
use crate::mint::JwtAlgorithm;

#[derive(Debug, Clone, serde::Serialize)]
pub struct JwksKey {
    pub kty: &'static str,
    pub kid: String,
    pub alg: &'static str,
    pub use_: &'static str,
    pub n: String,
    pub e: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Jwks {
    pub keys: Vec<JwksKey>,
}

#[must_use]
pub fn build_jwks(kid: &str, modulus_b64: String, exponent_b64: String) -> Jwks {
    Jwks {
        keys: vec![JwksKey {
            kty: "RSA",
            kid: kid.into(),
            alg: match JwtAlgorithm::Rs256 {
                JwtAlgorithm::Rs256 => "RS256",
            },
            use_: "sig",
            n: modulus_b64,
            e: exponent_b64,
        }],
    }
}

#[must_use]
pub fn empty_jwks() -> Jwks {
    Jwks { keys: vec![] }
}

pub fn _ensure_compile() -> Result<()> {
    Ok(())
}