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

    /// The session store is at its configured ceiling and no expired
    /// entry could be reclaimed to make room.
    ///
    /// Reported instead of evicting a live session on purpose. Silently
    /// logging a signed-in user out to protect the process is the worse
    /// of the two failures: it is invisible, it is not attributable,
    /// and it produces a support ticket that reads "I got logged out"
    /// with no way to correlate it to load. Refusing the new session
    /// fails loudly, at login, and the store drains on its own as
    /// sessions reach their expiry.
    ///
    /// The number is a *ceiling*, not a target — see
    /// [`crate::session::DEFAULT_MAX_SESSIONS`].
    #[error("session store is full ({0} sessions)")]
    SessionStoreFull(usize),

    /// A session storage operation could not be carried out at all: the
    /// shared backend was unreachable, timed out, or returned something
    /// this crate could not store.
    ///
    /// Deliberately distinct from a `None` lookup. A shared backend that
    /// is down must **not** be reportable as "no such session": the two
    /// are different facts and a caller that cannot tell them apart
    /// either logs users out on a network blip or, worse, treats a
    /// store failure as an authorization answer. `lookup` therefore
    /// propagates this variant rather than collapsing it into
    /// `Ok(None)`.
    ///
    /// Carries the backend's own message, so a
    /// [`SharedSessionBackend`](crate::shared_session::SharedSessionBackend)
    /// implementation must not format a session token into it — a token
    /// in a log line is a live credential.
    #[error("session backend unavailable: {0}")]
    SessionBackend(String),
}
