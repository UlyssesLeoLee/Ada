//! Opaque session token + cookie management. Session IDs are
//! random 256-bit values; storage is left to the api-gateway
//! (Postgres `session` table per RFC 8693 §5.2).

use parking_lot::RwLock;
use std::collections::HashMap;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct Session {
    pub user_id: String,
    pub tenant_id: String,
    pub roles: Vec<String>,
    pub expires_at: Instant,
}

#[derive(Debug, Default)]
pub struct SessionStore {
    by_token: RwLock<HashMap<String, Session>>,
}

impl SessionStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Mint a fresh session token (URL-safe base64 of 32 random bytes).
    pub fn mint(&self, session: Session) -> String {
        let mut buf = [0u8; 32];
        rand::Rng::fill(&mut rand::thread_rng(), &mut buf[..]);
        let token = crate::base64util::b64url_encode(&buf);
        self.by_token.write().insert(token.clone(), session);
        token
    }

    #[must_use]
    pub fn lookup(&self, token: &str) -> Option<Session> {
        let s = self.by_token.read().get(token).cloned()?;
        if s.expires_at < Instant::now() {
            return None;
        }
        Some(s)
    }

    pub fn revoke(&self, token: &str) {
        self.by_token.write().remove(token);
    }
}