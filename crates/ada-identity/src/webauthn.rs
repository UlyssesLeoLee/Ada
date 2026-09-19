//! WebAuthn RP (FIDO2). Registration + authentication ceremony.

use serde::{Deserialize, Serialize};

use crate::error::{IdentityError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebAuthnRpConfig {
    pub rp_id: String,
    pub rp_name: String,
    pub rp_origin: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrationChallenge {
    pub challenge_b64: String,
    pub user_id_b64: String,
    pub rp_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationChallenge {
    pub challenge_b64: String,
    pub rp_id: String,
    pub allow_credentials: Vec<String>,
}

pub fn begin_registration(cfg: &WebAuthnRpConfig, user_id: &[u8]) -> Result<RegistrationChallenge> {
    if cfg.rp_id.is_empty() || cfg.rp_origin.is_empty() {
        return Err(IdentityError::WebAuthn("rp_id / rp_origin empty".into()));
    }
    let mut challenge = [0u8; 32];
    rand::Rng::fill(&mut rand::thread_rng(), &mut challenge[..]);
    Ok(RegistrationChallenge {
        challenge_b64: crate::base64util::b64url_encode(&challenge),
        user_id_b64: crate::base64util::b64url_encode(user_id),
        rp_id: cfg.rp_id.clone(),
    })
}

pub fn begin_authentication(cfg: &WebAuthnRpConfig) -> Result<AuthenticationChallenge> {
    if cfg.rp_id.is_empty() {
        return Err(IdentityError::WebAuthn("rp_id empty".into()));
    }
    let mut challenge = [0u8; 32];
    rand::Rng::fill(&mut rand::thread_rng(), &mut challenge[..]);
    Ok(AuthenticationChallenge {
        challenge_b64: crate::base64util::b64url_encode(&challenge),
        rp_id: cfg.rp_id.clone(),
        allow_credentials: vec![],
    })
}