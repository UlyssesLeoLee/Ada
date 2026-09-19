//! SAML 2.0 SP. AuthnRequest generation + assertion parsing.
//!
//! Real wire-format signing / verification goes through the
//! `samael` crate (XML + xmlsec). The v0.4.0 skeleton wraps the
//! high-level state machine without binding it to a specific IdP.

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

/// Begin a SAML 2.0 AuthnRequest. Returns the encoded
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

/// Parse a SAML 2.0 response (SAMLResponse=b64xml). Validates the
/// issuer + audience + signature in production; the v0.4.0
/// skeleton returns a parsed-OK stub.
pub async fn parse_response(saml_response_b64: &str) -> Result<SamlAssertion> {
    if saml_response_b64.is_empty() {
        return Err(IdentityError::Saml("empty response".into()));
    }
    Ok(SamlAssertion {
        subject: String::new(),
        audience: String::new(),
        issuer: String::new(),
        attributes: vec![],
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamlAssertion {
    pub subject: String,
    pub audience: String,
    pub issuer: String,
    pub attributes: Vec<(String, String)>,
}