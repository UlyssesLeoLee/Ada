//! OpenID Connect RP. Authorization Code + PKCE flow.
//!
//! v0.4.0 skeleton: state-machine + RFC shapes only. Real wire-format
//! signing / verification via `openidconnect = "3"` is deferred to
//! v0.5.0 — see `v0.5.0-roadmap.md` §3.

use serde::{Deserialize, Serialize};

use crate::error::{IdentityError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcProviderConfig {
    pub issuer: String,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
}

/// Build a PKCE `code_verifier` (43-128 chars). Caller must base64url
/// encode the SHA256 of this value into `code_challenge`.
#[must_use]
pub fn generate_code_verifier() -> String {
    use rand::RngCore;
    let mut buf = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut buf);
    crate::base64util::b64url_encode(&buf)
}

/// Begin the OIDC Authorization Code + PKCE flow. Returns the
/// `state`, `nonce`, and the `code_verifier` the caller must
/// persist into a server-side session bound to the browser cookie.
pub fn begin_flow(
    cfg: &OidcProviderConfig,
) -> Result<(String /* state */, String /* nonce */, String /* code_verifier */)> {
    if cfg.scopes.is_empty() {
        return Err(IdentityError::Oidc("scopes empty".into()));
    }
    let state = generate_code_verifier();
    let nonce = generate_code_verifier();
    let code_verifier = generate_code_verifier();
    let _ = nonce;
    Ok((state, nonce, code_verifier))
}

/// Finish the OIDC flow by exchanging `code` for tokens. Real
/// implementation delegates to `openidconnect::CoreClient`.
pub async fn complete_flow(
    cfg: &OidcProviderConfig,
    code: &str,
    code_verifier: &str,
) -> Result<OidcTokenSet> {
    if code.is_empty() || code_verifier.is_empty() {
        return Err(IdentityError::Oidc("missing code or verifier".into()));
    }
    let _ = cfg;
    Ok(OidcTokenSet {
        access_token: String::new(),
        id_token: String::new(),
        refresh_token: None,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcTokenSet {
    pub access_token: String,
    pub id_token: String,
    pub refresh_token: Option<String>,
}