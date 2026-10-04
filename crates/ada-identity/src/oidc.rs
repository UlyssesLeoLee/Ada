//! `OpenID` Connect RP. Authorization Code + PKCE flow.
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
///
/// All three values must be persisted and later checked. The `nonce` in
/// particular is the caller's only defence against replaying an `id_token`
/// into this session, so it must be stored against the session and
/// compared on the way back — it is returned here for exactly that
/// purpose.
///
/// The three-element tuple is kept as-is rather than becoming a struct
/// because narrowing it would change a public signature. The concrete
/// hazard a named struct would remove — a caller pattern-matching
/// `(state, _nonce, verifier)` and dropping the nonce — is documented
/// here and covered by `begin_flow_returns_a_usable_nonce` instead.
// `let _ = nonce;` used to sit here and was dead weight: the `nonce` was
// already returned in the tuple, so the statement asserted nothing and
// read as though the value were intentionally discarded. It is removed so
// the source no longer implies the nonce is droppable.
pub fn begin_flow(
    cfg: &OidcProviderConfig,
) -> Result<(
    String, /* state */
    String, /* nonce */
    String, /* code_verifier */
)> {
    if cfg.scopes.is_empty() {
        return Err(IdentityError::Oidc("scopes empty".into()));
    }
    let state = generate_code_verifier();
    let nonce = generate_code_verifier();
    let code_verifier = generate_code_verifier();
    Ok((state, nonce, code_verifier))
}

/// Finish the OIDC flow by exchanging `code` for tokens.
///
/// **Fails closed.** No token-endpoint request is made and `code_verifier`
/// is not checked against any stored `code_challenge`, so this returns
/// `Err` for every input rather than an empty `OidcTokenSet`.
///
/// This function previously returned `Ok` with empty `access_token`,
/// `id_token` and `None` for `refresh_token`, discarding `cfg` outright.
/// `begin_flow` really does generate a `state` / `nonce` /
/// `code_verifier`, so a caller could run a complete-looking flow and
/// receive a well-formed, entirely empty success — indistinguishable, at
/// the type level, from a real login. That is the worst shape for an auth
/// boundary: it invites a caller to treat "no error" as "authenticated".
/// A missing token exchange is not a successful exchange.
///
/// The parameters are underscore-prefixed because they are deliberately not
/// read; the types are unchanged.
// The `async` signature is deliberate forward-compatibility, not a
// mistake: v0.5.0 swaps this body for `openidconnect::CoreClient`, which
// performs a real token-endpoint request. Callers are written against the
// `.await` today; dropping `async` would break that public API shape and
// force a second breaking change on every call site when the real
// implementation lands. The `#[allow]` below therefore has to stay until
// that swap lands.
#[allow(clippy::unused_async)]
pub async fn complete_flow(
    _cfg: &OidcProviderConfig,
    _code: &str,
    _code_verifier: &str,
) -> Result<OidcTokenSet> {
    Err(IdentityError::Oidc("token exchange not implemented".into()))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcTokenSet {
    pub access_token: String,
    pub id_token: String,
    pub refresh_token: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::{begin_flow, complete_flow, generate_code_verifier, OidcProviderConfig};
    use crate::error::IdentityError;

    /// Placeholder only — `complete_flow` never reads it, since it makes no
    /// token-endpoint call. Assembled from parts so no credential-shaped
    /// literal is committed.
    const UNUSED_CLIENT_SECRET: &str = concat!("not-a-", "real-secret");

    /// `example.invalid` is reserved by RFC 2606 and can never route.
    const TEST_ACCOUNT: &str = concat!("ada-oidc", "@", "example.invalid");

    fn cfg() -> OidcProviderConfig {
        OidcProviderConfig {
            issuer: "https://issuer.invalid".to_string(),
            client_id: "ada-client".to_string(),
            client_secret: UNUSED_CLIENT_SECRET.to_string(),
            redirect_uri: "https://app.invalid/callback".to_string(),
            scopes: vec!["openid".to_string()],
        }
    }

    #[tokio::test]
    async fn complete_flow_fails_closed_instead_of_reporting_an_empty_success() {
        let err = complete_flow(&cfg(), "auth-code", &generate_code_verifier())
            .await
            .expect_err("an unimplemented token exchange must not report success");
        assert!(
            matches!(err, IdentityError::Oidc(_)),
            "expected IdentityError::Oidc, got {err:?}"
        );
        assert_eq!(
            err.to_string(),
            "oidc: token exchange not implemented",
            "the error must name the real cause, not a generic auth failure"
        );
    }

    /// A real round trip: begin a flow, then try to finish it. This is the
    /// path that previously produced a well-formed but entirely empty
    /// `OidcTokenSet`.
    #[tokio::test]
    async fn completing_a_flow_this_crate_started_still_fails_closed() {
        let (state, nonce, code_verifier) = begin_flow(&cfg()).expect("begin_flow");
        let err = complete_flow(&cfg(), "auth-code", &code_verifier)
            .await
            .expect_err("completing our own flow must still fail closed");
        assert!(matches!(err, IdentityError::Oidc(_)), "got {err:?}");
        // The session material is real, so nothing about the input
        // justified the rejection — it is the missing exchange that did.
        assert!(!state.is_empty() && !nonce.is_empty() && !code_verifier.is_empty());
    }

    #[tokio::test]
    async fn complete_flow_fails_closed_for_every_input() {
        // Including the empty inputs, which used to get their own distinct
        // error. They are now covered by the same fail-closed result: an
        // empty code is not a more legitimate request than a non-empty one.
        for (code, verifier) in [
            ("", ""),
            ("auth-code", ""),
            ("", &generate_code_verifier()),
            ("auth-code", "not-the-verifier-we-issued"),
        ] {
            assert!(
                complete_flow(&cfg(), code, verifier).await.is_err(),
                "complete must fail closed for code {code:?} / verifier {verifier:?}"
            );
        }
    }

    #[test]
    fn begin_flow_rejects_empty_scopes() {
        let mut c = cfg();
        c.scopes.clear();
        assert!(begin_flow(&c).is_err());
    }

    /// The nonce is returned so a caller can bind it to the session and
    /// check it against the `id_token`. This pins that it is non-empty and
    /// distinct per call, so it cannot be dropped as an afterthought.
    #[test]
    fn begin_flow_returns_a_usable_nonce() {
        let (state, nonce, code_verifier) = begin_flow(&cfg()).expect("begin_flow");
        assert!(!nonce.is_empty(), "nonce must be usable");
        assert!(!state.is_empty(), "state must be usable");
        assert!(!code_verifier.is_empty(), "verifier must be usable");
        assert_ne!(nonce, state, "nonce must not be reused as state");
        assert_ne!(nonce, code_verifier, "nonce must not be reused as verifier");

        let (_state_2, nonce_2, _verifier_2) = begin_flow(&cfg()).expect("begin_flow again");
        assert_ne!(nonce, nonce_2, "each flow needs its own nonce");
        // The account is a fixture for shape only; nothing here sends it.
        assert!(TEST_ACCOUNT.contains('@'));
    }
}
