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

    #[error("jwt verification failed")]
    JwtVerification,

    #[error("rate limited")]
    RateLimited,

    #[error("invalid recovery code")]
    RecoveryInvalid,

    #[error("recovery code already used")]
    RecoveryRedeemed,
}