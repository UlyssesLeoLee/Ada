//! SAML 2.0 SP. `AuthnRequest` generation + Response parsing.
//!
//! v0.4.0 skeleton: state-machine + RFC shapes only. Real wire-format
//! signing / verification via `samael = "0.0.22"` is deferred to
//! v0.5.0 — see `v0.5.0-roadmap.md` §3. Samael pulls xmlsec (C lib)
//! on Windows; CI's Linux runner is the right environment for the
//! real swap-in.

use serde::{Deserialize, Serialize};

use crate::error::{IdentityError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamlSpConfig {
    pub entity_id: String,
    pub acs_url: String,
    pub slo_url: Option<String>,
    pub idp_metadata_url: String,
    pub name_id_format: String,
}

/// Begin a SAML 2.0 `AuthnRequest`. Returns the encoded
/// `SAMLRequest` query string the SP redirects to.
pub fn begin_authn(cfg: &SamlSpConfig, acs_index: u32) -> Result<String> {
    if cfg.entity_id.is_empty() || cfg.acs_url.is_empty() {
        return Err(IdentityError::Saml("entity_id / acs_url empty".into()));
    }
    let relay_state = uuid::Uuid::new_v4().to_string();
    let _ = acs_index;
    Ok(format!(
        "SAMLRequest=authn_request&RelayState={relay_state}&acs={}",
        urlencoding(&cfg.acs_url)
    ))
}

fn urlencoding(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

/// Parse a SAML 2.0 response (SAMLResponse=b64xml).
///
/// **Fails closed.** Issuer, audience and signature are not validated
/// here, so this returns `Err` for every input rather than an
/// `Ok(SamlAssertion)`. Validation needs the xmlsec-backed `samael`
/// integration, deferred to v0.5.0 (see the module docs).
///
/// This function previously rejected only the empty string and otherwise
/// returned a successfully-parsed assertion with an empty `subject`,
/// `audience` and `issuer`. Any non-empty input therefore "authenticated"
/// — and since `subject` came back empty, downstream code had nothing to
/// bind a session to, so a caller wiring this up was likely to substitute
/// its own untrusted field. An unvalidated assertion must not be
/// constructible.
///
/// The parameter is underscore-prefixed because it is deliberately not
/// read; the type is unchanged.
// The `async` signature is deliberate forward-compatibility, not a
// mistake: v0.5.0 swaps this body for the real xmlsec-backed assertion
// validation, which is I/O bound. Callers are written against the `.await`
// today; dropping `async` would break that public API shape and force a
// second breaking change when the real implementation lands. The
// `#[allow]` below therefore has to stay until that swap lands.
#[allow(clippy::unused_async)]
pub async fn parse_response(_saml_response_b64: &str) -> Result<SamlAssertion> {
    Err(IdentityError::Saml(
        "assertion validation not implemented".into(),
    ))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamlAssertion {
    pub subject: String,
    pub audience: String,
    pub issuer: String,
    pub attributes: Vec<(String, String)>,
}

#[cfg(test)]
mod tests {
    use super::parse_response;
    use crate::error::IdentityError;

    /// The old implementation returned `Ok` with empty `subject` /
    /// `audience` / `issuer` for exactly this shape, so the fixture is the
    /// regression, not an incidental input.
    const BASE64_LOOKING_BLOB: &str = "PHNhbWxwOlJlc3BvbnNlPjwvc2FtbHA6UmVzcG9uc2U+";

    #[tokio::test]
    async fn parse_response_rejects_a_well_formed_looking_assertion() {
        let err = parse_response(BASE64_LOOKING_BLOB)
            .await
            .expect_err("an unvalidated assertion must not parse");
        assert!(
            matches!(err, IdentityError::Saml(_)),
            "expected IdentityError::Saml, got {err:?}"
        );
        assert_eq!(
            err.to_string(),
            "saml: assertion validation not implemented",
            "the error must name the real cause, not a generic parse failure"
        );
    }

    #[tokio::test]
    async fn parse_response_still_rejects_empty_input() {
        // The empty case used to have its own message. It now falls through
        // to the same fail-closed error, which is correct: an empty
        // response is not "more validated" than a non-empty one.
        let err = parse_response("")
            .await
            .expect_err("empty input must error");
        assert!(
            matches!(err, IdentityError::Saml(_)),
            "expected IdentityError::Saml, got {err:?}"
        );
    }

    #[tokio::test]
    async fn parse_response_fails_closed_for_every_input() {
        // Nothing gets through, including inputs that are obviously not
        // XML and inputs that name a trusted issuer.
        for input in [
            "",
            " ",
            "not-base64-at-all!!",
            "<samlp:Response/>",
            BASE64_LOOKING_BLOB,
        ] {
            assert!(
                parse_response(input).await.is_err(),
                "parse must fail closed for {input:?}"
            );
        }
    }
}
