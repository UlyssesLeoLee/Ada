//! Identity configuration. All values via env. No env value printed
//! (per memory 2026-08-27 hard ban).

use crate::error::{IdentityError, Result};

#[derive(Debug, Clone)]
pub struct Config {
    pub identity_jwt_private_key: String,
    pub identity_jwt_kid: String,
    pub identity_base_url: String,
    pub identity_rp_origin: String,
    pub identity_trusted_proxies: Vec<String>,
    pub identity_rate_limit_per_min: u32,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let identity_jwt_private_key = std::env::var("IDENTITY_JWT_PRIVATE_KEY")
            .map_err(|_| IdentityError::Config("IDENTITY_JWT_PRIVATE_KEY".into()))?;
        if identity_jwt_private_key.is_empty() {
            return Err(IdentityError::Config("IDENTITY_JWT_PRIVATE_KEY".into()));
        }
        let identity_jwt_kid = std::env::var("IDENTITY_JWT_KID")
            .map_err(|_| IdentityError::Config("IDENTITY_JWT_KID".into()))?;
        let identity_base_url = std::env::var("IDENTITY_BASE_URL")
            .map_err(|_| IdentityError::Config("IDENTITY_BASE_URL".into()))?;
        let identity_rp_origin = std::env::var("IDENTITY_RP_ORIGIN")
            .unwrap_or_else(|_| "https://gm-console.kanvas.dev".into());
        let identity_trusted_proxies = std::env::var("IDENTITY_TRUSTED_PROXIES")
            .ok()
            .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_default();
        let identity_rate_limit_per_min = std::env::var("IDENTITY_RATE_LIMIT_PER_MIN")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(10);
        Ok(Self {
            identity_jwt_private_key,
            identity_jwt_kid,
            identity_base_url,
            identity_rp_origin,
            identity_trusted_proxies,
            identity_rate_limit_per_min,
        })
    }
}