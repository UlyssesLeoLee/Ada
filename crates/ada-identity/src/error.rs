//! Error types for ada-identity. Never echoes env values, secrets, or
//! PII to logs.

use thiserror::Error;

pub type Result<T> = std::result::Result<T, IdentityError>;

#[derive(Debug, Error)]
pub enum IdentityError {
    #[error("missing or invalid config: {0}")]
    Config(String),

    #[error("oidc: {0}")]
    Oidc(String),

    #[error("saml: {0}")]
    Saml(String),

    #[error("webauthn: {0}")]
    WebAuthn(String),

    #[error("totp: {0}")]
    Totp(String),

    #[error("jwt signing failed")]
    JwtSigning,

    /// RS256 signing is not implemented in this build, so `mint_jwt` and
    /// `verify_jwt_stub` both fail closed with this variant instead of
    /// producing or accepting a token whose signature segment is empty.
    ///
    /// This is deliberately distinct from [`IdentityError::JwtSigning`]
    /// (a signer that ran and failed) and from
    /// [`IdentityError::JwtVerification`] (a signature that was checked
    /// and did not match). A caller triaging this variant needs to know
    /// that the fix is "wire up a real signer", not "retry" and not
    /// "check the JWKS".
    #[error("jwt signing not implemented")]
    JwtSigningUnavailable,

    #[error("jwt verification failed")]
    JwtVerification,

    #[error("rate limited")]
    RateLimited,

    #[error("invalid recovery code")]
    RecoveryInvalid,

    #[error("recovery code already used")]
    RecoveryRedeemed,
}
