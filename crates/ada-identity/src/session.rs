//! Opaque session token + cookie management. Session IDs are
//! random 256-bit values; storage is left to the api-gateway
//! (Postgres `session` table per RFC 8693 §5.2).
//!
//! ## Retention
//!
//! This store is the credential authority for the api-gateway, so it
//! has to be bounded in two independent ways. Both were missing when the
//! gateway first wired it in:
//!
//! - **Expiry did not free anything.** `lookup` returned `None` for an
//!   expired token but left the entry in the map. Every session ever
//!   minted stayed resident for the life of the process, holding three
//!   `String`s and a `Vec<String>`. Since the module doc has always said
//!   a login flow would be the next thing built on top of this, the leak
//!   was one endpoint away from being a memory-exhaustion vector.
//!   Expired entries are now removed on the read that observes them,
//!   and on every mint.
//! - **There was no ceiling.** Unbounded minting meant a burst of
//!   logins — or anything that could reach a mint call — could grow the
//!   map without limit. [`SessionStore::with_max_sessions`] bounds it
//!   and [`SessionStore::mint`] fails closed at the bound.
//!
//! The bound is per-process and it is not shared. Two replicas each get
//! their own store, which is the deeper limitation: a session minted on
//! one pod is unknown to the other, so a restart invalidates every
//! outstanding credential. That is acceptable *only* because nothing
//! mints yet. The moment a login flow exists, this type has to be
//! backed by shared storage or a signed token, and this module is the
//! place that decision will be forced.

use parking_lot::RwLock;
use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::error::{IdentityError, Result};

/// Default ceiling on concurrently stored sessions.
///
/// Sized for a single process holding live sessions for a mid-size
/// deployment, not for a cache: at roughly 200 bytes per entry a
/// hundred thousand is tens of megabytes, so hitting it means
/// something is wrong upstream rather than that the limit is tight.
/// A process that reaches it should log and alarm, not grow.
pub const DEFAULT_MAX_SESSIONS: usize = 100_000;

/// An authenticated session as the server-side store holds it.
#[derive(Debug, Clone)]
pub struct Session {
    /// Stable subject id. This is what an audit record should name.
    pub user_id: String,
    /// Tenant the session was minted for. The isolation key for every
    /// authorization decision made with this session — which is why it
    /// lives here, on the server, and never in a client-supplied header.
    pub tenant_id: String,
    /// Role names, unprefixed (`"owner"`, not `"role:owner"`).
    pub roles: Vec<String>,
    /// When this session stops being valid. Compared against
    /// [`Instant`], so it is monotonic and immune to wall-clock jumps —
    /// a session cannot be prolonged by moving the host clock.
    pub expires_at: Instant,
}

/// In-memory session storage.
///
/// See the module docs for why it is bounded and why that is not yet
/// sufficient for more than one replica.
#[derive(Debug)]
pub struct SessionStore {
    by_token: RwLock<HashMap<String, Session>>,
    max_sessions: usize,
}

impl SessionStore {
    /// A store with the default ceiling.
    #[must_use]
    pub fn new() -> Self {
        Self::with_max_sessions(DEFAULT_MAX_SESSIONS)
    }

    /// A store holding at most `max_sessions` entries.
    ///
    /// A ceiling of 0 is not a way to disable sessions: every mint
    /// fails, which is a coherent (if useless) configuration and is
    /// what a test asserting the bound actually needs.
    #[must_use]
    pub fn with_max_sessions(max_sessions: usize) -> Self {
        Self {
            by_token: RwLock::new(HashMap::new()),
            max_sessions,
        }
    }

    /// The configured ceiling.
    #[must_use]
    pub fn max_sessions(&self) -> usize {
        self.max_sessions
    }

    /// Store `session` and return its opaque token (URL-safe base64 of
    /// 32 random bytes from the OS-seeded CSPRNG).
    ///
    /// Reclaims expired entries first. If the store is still at its
    /// ceiling afterwards this returns
    /// [`IdentityError::SessionStoreFull`] rather than evicting a live
    /// session — see that variant for why.
    pub fn mint(&self, session: Session) -> Result<String> {
        let mut buf = [0u8; 32];
        rand::Rng::fill(&mut rand::thread_rng(), &mut buf[..]);
        let token = crate::base64util::b64url_encode(&buf);

        let mut guard = self.by_token.write();
        Self::sweep_locked(&mut guard);
        if guard.len() >= self.max_sessions {
            return Err(IdentityError::SessionStoreFull(self.max_sessions));
        }
        // A 256-bit token colliding with an existing one is not a
        // concern worth handling: overwriting would be strictly
        // *safer* than inserting alongside anyway, which is what
        // `insert` already does.
        guard.insert(token.clone(), session);
        Ok(token)
    }

    /// Resolve a token to its [`Session`], or `None` if it is unknown
    /// or expired.
    ///
    /// A live session is read under the read lock, so the authorization
    /// path — every request the gateway serves — never contends for a
    /// write. Only the expired branch takes the write lock, and only to
    /// reclaim the entry it just found dead.
    #[must_use]
    pub fn lookup(&self, token: &str) -> Option<Session> {
        let now = Instant::now();
        {
            let guard = self.by_token.read();
            let found = guard.get(token)?;
            if found.expires_at >= now {
                return Some(found.clone());
            }
        }
        // Expired. Fall out of the read guard before taking the write
        // one — parking_lot is not reentrant and would deadlock.
        self.by_token.write().remove(token);
        None
    }

    /// Drop `token` if present. Idempotent.
    pub fn revoke(&self, token: &str) {
        self.by_token.write().remove(token);
    }

    /// Remove every entry whose `expires_at` has passed, returning how
    /// many went.
    ///
    /// Exposed because a process that stops minting — an idle replica,
    /// or one whose traffic has fallen to nothing — would otherwise
    /// hold its last batch of dead sessions until the next login.
    pub fn sweep_expired(&self) -> usize {
        Self::sweep_locked(&mut self.by_token.write())
    }

    /// Total entries held, expired ones included.
    ///
    /// After a `sweep_expired` this equals [`Self::live_len`]. The two
    /// are separate because the interesting number operationally is the
    /// raw footprint, and a number that silently excludes the dead
    /// entries would under-report exactly the leak it exists to catch.
    #[must_use]
    pub fn len(&self) -> usize {
        self.by_token.read().len()
    }

    /// Entries that have not yet expired.
    #[must_use]
    pub fn live_len(&self) -> usize {
        let now = Instant::now();
        self.by_token
            .read()
            .values()
            .filter(|s| s.expires_at >= now)
            .count()
    }

    /// Whether the store holds nothing at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_token.read().is_empty()
    }

    /// Caller holds the write guard.
    fn sweep_locked(guard: &mut HashMap<String, Session>) -> usize {
        let now = Instant::now();
        let before = guard.len();
        guard.retain(|_, s| s.expires_at >= now);
        before - guard.len()
    }
}

impl Default for SessionStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Build a session expiring `ttl` from now. Test convenience so a test
/// cannot accidentally express its intent in the wrong unit.
#[must_use]
pub fn session_ttl(ttl: Duration) -> Instant {
    Instant::now() + ttl
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(ttl: Duration) -> Session {
        Session {
            user_id: "u1".into(),
            tenant_id: "t1".into(),
            roles: vec!["viewer".into()],
            expires_at: session_ttl(ttl),
        }
    }

    #[test]
    fn a_minted_token_resolves_back_to_its_session() {
        let store = SessionStore::new();
        let token = store.mint(session(Duration::from_secs(60))).expect("mint");
        let got = store.lookup(&token).expect("lookup");
        assert_eq!(got.user_id, "u1");
        assert_eq!(got.tenant_id, "t1");
        assert_eq!(got.roles, vec!["viewer".to_string()]);
    }

    #[test]
    fn an_unknown_token_resolves_to_nothing() {
        let store = SessionStore::new();
        assert!(store.lookup("never-minted").is_none());
        assert!(store.lookup("").is_none());
    }

    #[test]
    fn an_expired_token_is_refused() {
        let store = SessionStore::new();
        let token = store.mint(session(Duration::from_secs(0))).expect("mint");
        assert!(store.lookup(&token).is_none());
    }

    /// The regression this module exists for: expiry used to be a read
    /// filter and nothing else, so the entry stayed resident forever.
    #[test]
    fn an_expired_session_is_removed_when_it_is_looked_up() {
        let store = SessionStore::new();
        let token = store.mint(session(Duration::from_secs(0))).expect("mint");
        assert_eq!(store.len(), 1, "precondition: the entry exists");

        assert!(store.lookup(&token).is_none());
        assert_eq!(
            store.len(),
            0,
            "an expired session must not stay resident after the read that observed it"
        );
    }

    /// The property that matters, and the reason `sweep_expired` is
    /// hard to test the obvious way.
    ///
    /// It is tempting to write "mint 5 expired, mint 3 live, assert
    /// `sweep_expired()` reclaims 5". That test cannot be written,
    /// because the store never reaches the state it describes: `mint`
    /// sweeps first, so each expired session is reclaimed by the *next*
    /// mint rather than sitting there for a manual sweep. Asserting
    /// `len() == 8` after those eight mints fails, and it should — that
    /// failure is the guarantee working, not a bug in it.
    ///
    /// So the invariant is stated directly instead: expired sessions do
    /// not accumulate, no matter how many pass through.
    #[test]
    fn expired_sessions_never_accumulate_however_many_are_minted() {
        let store = SessionStore::with_max_sessions(3);
        for i in 0..50 {
            store
                .mint(session(Duration::from_secs(0)))
                .unwrap_or_else(|e| panic!("mint {i} must fit: {e}"));
        }
        assert_eq!(
            store.len(),
            1,
            "each mint reclaims the previous dead entry, so the store never grows past one"
        );
        assert_eq!(store.live_len(), 0);
    }

    /// A burst of logins followed by an equal burst of expiries must not
    /// permanently wedge the store at its ceiling.
    #[test]
    fn the_ceiling_is_never_reached_by_accumulated_garbage() {
        let store = SessionStore::with_max_sessions(3);
        for _ in 0..20 {
            store.mint(session(Duration::from_secs(0))).expect("mint");
        }
        store
            .mint(session(Duration::from_secs(600)))
            .expect("a live session must fit: the ceiling counts live sessions");
        assert_eq!(store.len(), 1);
        assert_eq!(store.live_len(), 1);
    }

    /// The manual sweep is for the process that stopped minting. With
    /// only live sessions it is a no-op, and it must be safe to call
    /// speculatively.
    #[test]
    fn sweeping_an_all_live_store_reclaims_nothing() {
        let store = SessionStore::new();
        for _ in 0..4 {
            store.mint(session(Duration::from_secs(600))).expect("mint");
        }
        assert_eq!(store.sweep_expired(), 0);
        assert_eq!(store.len(), 4);
        assert_eq!(store.live_len(), 4);
    }

    /// `len` and `live_len` are separate on purpose: a number that
    /// silently excluded dead entries would under-report the exact leak
    /// it exists to catch.
    ///
    /// The order matters and is easy to get wrong. Minting the dead
    /// session *first* and the live one second leaves `len() == 1`,
    /// because the second mint sweeps the first. A dead entry only
    /// lingers when it arrives after the live ones it is hiding
    /// behind.
    #[test]
    fn len_counts_dead_entries_that_live_len_excludes() {
        let store = SessionStore::with_max_sessions(10);
        store.mint(session(Duration::from_secs(600))).expect("mint");
        store.mint(session(Duration::from_secs(0))).expect("mint");
        assert_eq!(store.len(), 2, "one dead entry is still resident");
        assert_eq!(store.live_len(), 1, "and only one is usable");
        assert_eq!(store.sweep_expired(), 1);
        assert_eq!(store.len(), 1);
        assert_eq!(store.live_len(), 1, "the live session survived the sweep");
    }

    /// The ceiling must be unreachable by expiry pressure. A burst of
    /// logins whose sessions all lapse must not leave the store
    /// permanently refusing new sessions.
    #[test]
    fn minting_fails_closed_only_when_live_sessions_fill_the_store() {
        let store = SessionStore::with_max_sessions(3);
        for i in 0..3 {
            store
                .mint(session(Duration::from_secs(600)))
                .unwrap_or_else(|e| panic!("mint {i} should fit under the ceiling: {e}"));
        }
        let err = store
            .mint(session(Duration::from_secs(600)))
            .expect_err("the fourth mint exceeds a ceiling of three");
        assert!(
            matches!(err, IdentityError::SessionStoreFull(3)),
            "expected SessionStoreFull(3), got {err:?}"
        );
        assert_eq!(store.len(), 3, "a refused mint must not have grown the map");
    }

    /// Refusing a mint is only acceptable if the sessions already in
    /// the store keep working. A cap that logs everyone out under load
    /// would be a worse failure than the one it prevents.
    #[test]
    fn hitting_the_ceiling_does_not_disturb_existing_sessions() {
        let store = SessionStore::with_max_sessions(2);
        let first = store.mint(session(Duration::from_secs(600))).expect("mint");
        let second = store.mint(session(Duration::from_secs(600))).expect("mint");
        let _ = store.mint(session(Duration::from_secs(600)));

        assert!(
            store.lookup(&first).is_some(),
            "the first session must survive"
        );
        assert!(
            store.lookup(&second).is_some(),
            "the second session must survive"
        );
    }

    #[test]
    fn revoke_removes_the_entry_not_just_the_answer() {
        let store = SessionStore::new();
        let token = store.mint(session(Duration::from_secs(600))).expect("mint");
        store.revoke(&token);
        assert!(store.lookup(&token).is_none());
        assert_eq!(store.len(), 0, "revoke must free the entry, not mask it");
        // Idempotent.
        store.revoke(&token);
    }

    #[test]
    fn a_fresh_store_is_empty() {
        let store = SessionStore::new();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
        assert_eq!(store.live_len(), 0);
        assert_eq!(store.max_sessions(), DEFAULT_MAX_SESSIONS);
    }
}
