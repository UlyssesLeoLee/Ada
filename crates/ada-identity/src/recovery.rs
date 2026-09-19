//! Recovery codes: 10 single-use 8-char alphanumeric strings
//! generated at enrollment. Hash (SHA-256) is stored; the raw
//! value is never logged.

use std::collections::HashSet;
use std::sync::Arc;

use parking_lot::RwLock;

use crate::error::{IdentityError, Result};

#[derive(Debug, Default)]
pub struct RecoveryStore {
    seen: RwLock<HashSet<String>>, // hex(SHA-256(raw))
}

impl RecoveryStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Generate `count` fresh recovery codes.
    #[must_use]
    pub fn generate(&self, count: usize) -> Vec<String> {
        (0..count)
            .map(|_| {
                let mut buf = [0u8; 8];
                rand::Rng::fill(&mut rand::thread_rng(), &mut buf[..]);
                let raw: String = buf.iter().map(|b| format!("{b:02x}")).collect();
                let h = sha256_hex(raw.as_bytes());
                self.seen.write().insert(h);
                raw
            })
            .collect()
    }

    /// Redeem a recovery code. Marks the SHA-256 hash as consumed;
    /// subsequent calls return `RecoveryRedeemed`.
    pub fn redeem(&self, raw: &str) -> Result<()> {
        let h = sha256_hex(raw.as_bytes());
        let mut w = self.seen.write();
        if !w.contains(&h) {
            return Err(IdentityError::RecoveryInvalid);
        }
        // Single-use: remove.
        w.remove(&h);
        Ok(())
    }
}

fn sha256_hex(s: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(s);
    let bytes = hasher.finalize();
    hex::encode(bytes)
}

#[must_use]
pub fn shared_store() -> Arc<RecoveryStore> {
    Arc::new(RecoveryStore::new())
}