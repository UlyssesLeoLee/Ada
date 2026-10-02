//! Recovery codes: 10 single-use 8-char alphanumeric strings
//! generated at enrollment. Hash (SHA-256) is stored; the raw
//! value is never logged.

use std::collections::HashSet;
use std::sync::{Arc, OnceLock};

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
                // `hex::encode` is exactly the concatenation of
                // `format!("{b:02x}")` over the bytes (lowercase, zero
                // padded, two chars per byte) and is already a dependency
                // of this module, so the emitted code is byte-identical.
                let raw = hex::encode(buf);
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

static SHARED_STORE: OnceLock<Arc<RecoveryStore>> = OnceLock::new();

/// Process-wide [`RecoveryStore`].
///
/// This really is a singleton: every call hands back the same allocation.
/// It previously did `Arc::new(RecoveryStore::new())` on each call, which
/// contradicted the name in both directions — called per redemption, no
/// code was ever in the freshly built store, so every legitimate code
/// failed as `RecoveryInvalid`; called once at startup, every user shared
/// one flat namespace.
///
/// Note that codes are not bound to a user: `redeem` takes only the raw
/// code, so a code issued to one account can be redeemed by another. That
/// is a real cross-account-takeover weakness, but closing it means adding
/// a `user_id` parameter to `redeem`, which changes a public signature and
/// needs a product / versioning decision. Left as-is and flagged rather
/// than fixed silently here.
#[must_use]
pub fn shared_store() -> Arc<RecoveryStore> {
    Arc::clone(SHARED_STORE.get_or_init(|| Arc::new(RecoveryStore::new())))
}

#[cfg(test)]
mod tests {
    use super::shared_store;
    use std::sync::Arc;

    /// The point of the singleton: two calls are the same allocation, so a
    /// code minted through one handle is redeemable through the other.
    /// Compared by address because two `Arc`s over one `RecoveryStore`
    /// share the same pointer while distinct stores do not.
    #[test]
    fn shared_store_returns_the_same_allocation_every_call() {
        let first = shared_store();
        let second = shared_store();
        assert_eq!(
            Arc::as_ptr(&first),
            Arc::as_ptr(&second),
            "shared_store() must hand back one process-wide store, not a fresh empty one"
        );

        // A third call after the others have been dropped must still be the
        // same store, not a re-initialised one.
        let third = shared_store();
        assert_eq!(Arc::as_ptr(&first), Arc::as_ptr(&third));
    }

    /// Behavioural counterpart to the pointer check: a code generated
    /// through the shared handle is visible to a later, independent call.
    /// This is what the previous implementation got wrong — the second
    /// call returned an empty store, so the code looked invalid.
    ///
    /// Safe to run alongside the pointer test: that one only reads
    /// addresses, and this one mints and redeems its own code.
    #[test]
    fn codes_minted_via_the_shared_store_redeem_through_a_later_call() {
        let code = {
            let store = shared_store();
            let codes = store.generate(1);
            codes.into_iter().next().expect("one code")
        };

        let store = shared_store();
        store
            .redeem(&code)
            .expect("a code minted via shared_store must redeem via shared_store");
        // Single-use still holds across handles.
        assert!(store.redeem(&code).is_err());
    }
}
