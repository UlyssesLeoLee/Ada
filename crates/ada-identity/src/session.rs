//! Opaque session token + cookie management. Session IDs are
//! random 256-bit values; multi-replica storage is the api-gateway's job and is
//! Redis, not a database table: it builds `RedisSessionBackend`
//! from `ADA_SESSION_REDIS_URL` and refuses to start when that is
//! unset, so there is no in-process fallback to 401 a credential
//! that landed on another replica.
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
//!   map without limit. `with_max_sessions` bounds it and `mint` fails
//!   closed at the bound.
//!
//! The bound is per-process and it is not shared. Two replicas each get
//! their own store, which is the deeper limitation: a session minted on
//! one pod is unknown to the other, so a restart invalidates every
//! outstanding credential. That was acceptable *only* because nothing
//! minted yet. It is no longer acceptable, so this module is now a
//! trait plus two backends.
//!
//! ## Backends
//!
//! [`SessionStorage`] is the contract callers program against. It is
//! `async` for the same reason the workspace's other pluggable-backport
//! traits are (`EventBus` in `ada-m15-central-event-bus`): so a
//! networked production backend can `await` without changing the
//! signature again later.
//!
//! - [`SharedSessionStore`](crate::shared_session::SharedSessionStore) —
//!   the production path. State lives in a backend every replica can
//!   reach, so a session minted on one pod is a credential on all of
//!   them and survives a restart.
//! - `SessionStore` — the in-process store. **Test-only.** It is
//!   behind `cfg(any(test, feature = "inproc-sessions"))`, so naming it
//!   from production code is a compile error rather than a comment
//!   nobody reads. It is retained because it is a fast, deterministic
//!   double for testing a caller's own logic, and because the semantics
//!   it already had — sweep on mint, fail closed at the ceiling, reclaim
//!   on the read that observed the death — are the semantics the shared
//!   store has to reproduce.
//!
//! A test may enable the feature for its own build by depending on
//! `ada-identity` with `features = ["inproc-sessions"]` from
//! `[dev-dependencies]`, which applies to that crate's test targets and
//! not to its normal build.

use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

use crate::error::Result;
// Only the in-process store needs these, and a production build does
// not have it: an import used solely by a `cfg`-gated item is an
// unused import in every other build, which `-D warnings` rejects.
#[cfg(any(test, feature = "inproc-sessions"))]
use crate::error::IdentityError;
#[cfg(any(test, feature = "inproc-sessions"))]
use parking_lot::RwLock;
#[cfg(any(test, feature = "inproc-sessions"))]
use std::collections::HashMap;

/// Default ceiling on concurrently stored sessions.
///
/// Sized for a single process holding live sessions for a mid-size
/// deployment, not for a cache: at roughly 200 bytes per entry a
/// hundred thousand is tens of megabytes, so hitting it means
/// something is wrong upstream rather than that the limit is tight.
/// A process that reaches it should log and alarm, not grow.
///
/// Shared by both backends so that "full" means the same number
/// whichever one is configured.
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
    ///
    /// A process-local clock, which is exactly why a shared backend
    /// does not store this field: see [`SessionRecord`].
    pub expires_at: Instant,
}

/// Mint an opaque session token: URL-safe base64 of 32 random bytes
/// from the OS-seeded CSPRNG.
///
/// The single definition of the token format, so both backends mint
/// indistinguishable credentials and a token minted by one is
/// indistinguishable from one minted by the other.
#[must_use]
pub fn new_session_token() -> String {
    let mut buf = [0u8; 32];
    rand::Rng::fill(&mut rand::thread_rng(), &mut buf[..]);
    crate::base64util::b64url_encode(&buf)
}

/// Wall-clock milliseconds since the Unix epoch, the unit every
/// shared backend's expiry is expressed in.
#[must_use]
pub fn unix_millis_now() -> i64 {
    Utc::now().timestamp_millis()
}

/// A session in the form a shared backend can persist.
///
/// The difference from [`Session`] is `expires_at` and nothing else.
/// [`Session::expires_at`] is an [`Instant`], which has no meaning
/// outside the process that created it — a serialised `Instant` is
/// garbage in a fresh pod — so a record crossing a network carries an
/// absolute deadline instead.
///
/// That trade is real and is the price of sharing: an absolute deadline
/// is wall-clock, so a session's remaining life now depends on the
/// clocks of the pods agreeing. The alternative was a session that
/// only exists on the pod that minted it, which is the defect this
/// module exists to remove. NTP-disciplined hosts and a TTL far shorter
/// than the worst plausible skew make the exposure smaller than the
/// outage it buys back.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionRecord {
    /// Stable subject id.
    pub user_id: String,
    /// Tenant the session was minted for.
    pub tenant_id: String,
    /// Role names, unprefixed.
    pub roles: Vec<String>,
    /// When this session stops being valid, in milliseconds since the
    /// Unix epoch. Absolute on purpose: a replica that restarts must
    /// compute the *same* remaining life, not a fresh one.
    pub expires_at_unix_ms: i64,
}

impl SessionRecord {
    /// Project a session into its persistable form.
    ///
    /// An already-expired session is recorded as expiring *now* rather
    /// than in the past: `Instant` differences can be negative, and an
    /// absolute deadline the backend would have to interpret. Zero
    /// remaining life is unambiguous to every reader.
    #[must_use]
    pub fn from_session(session: &Session) -> Self {
        let remaining = session
            .expires_at
            .saturating_duration_since(Instant::now())
            .as_millis();
        // `as_millis` is u128 and i64 is ~292 million years of
        // milliseconds. A TTL that long is a bug elsewhere, but it must
        // saturate rather than wrap into the past and expire the
        // session the moment it is stored.
        let remaining = i64::try_from(remaining).unwrap_or(i64::MAX);
        // Truncation note: `as_millis()` floors, so the stored deadline
        // can be up to 1 ms shorter than the true expiry. That is the
        // safe direction for a session -- never longer than intended --
        // and it is far below the clock skew this field already assumes
        // between replicas (see `SessionRecord`'s docs). The contract
        // test carries the matching 1 ms slack.
        Self {
            user_id: session.user_id.clone(),
            tenant_id: session.tenant_id.clone(),
            roles: session.roles.clone(),
            expires_at_unix_ms: unix_millis_now().saturating_add(remaining),
        }
    }

    /// Re-anchor a record onto this process's clock, or `None` if the
    /// deadline has passed.
    ///
    /// `now_unix_ms` is a parameter rather than a call to
    /// [`unix_millis_now`] so that expiry can be exercised at a chosen
    /// instant instead of by sleeping.
    #[must_use]
    pub fn to_session(&self, now_unix_ms: i64) -> Option<Session> {
        let remaining_ms = self.expires_at_unix_ms.checked_sub(now_unix_ms)?;
        if remaining_ms <= 0 {
            return None;
        }
        // Unreachable for a negative value, which the guard above
        // already excluded, but a corrupt backend row must not be able
        // to panic a request thread.
        let Ok(remaining_ms) = u64::try_from(remaining_ms) else {
            return None;
        };
        Some(Session {
            user_id: self.user_id.clone(),
            tenant_id: self.tenant_id.clone(),
            roles: self.roles.clone(),
            expires_at: Instant::now() + Duration::from_millis(remaining_ms),
        })
    }
}

/// Session storage, as callers program against it.
///
/// Two implementations exist and they are not interchangeable:
/// [`SharedSessionStore`](crate::shared_session::SharedSessionStore),
/// whose state is reachable from every replica, and the test-only
/// `SessionStore`. Anything that must work in more than one process
/// has to take a `dyn SessionStorage` rather than naming a concrete
/// backend.
///
/// The guarantee both implementations owe a caller:
///
/// - a token is 256 bits of CSPRNG output and is the only handle to the
///   session;
/// - `lookup` returns `None` for an unknown **or** expired token, and
///   does not report a storage failure as either of those;
/// - an expired session stops occupying storage;
/// - hitting the ceiling refuses the new session rather than evicting a
///   live one.
///
/// # The in-process backend is not reachable from here
///
/// Naming `SessionStore` from a production build is a compile error: it
/// is `#[cfg(any(test, feature = "inproc-sessions"))]`, and nothing in a
/// production dependency graph enables that feature.
///
/// # Why there is no `compile_fail` doctest proving it
///
/// There was one, and it was **vacuous in the context it ran in** — CI
/// failed it with "Test compiled successfully, but it's marked
/// `compile_fail`". Its reasoning was that "a doc test builds this crate
/// as a normal dependency, not under `cfg(test)`". That part is true and
/// irrelevant. Under `cargo test --workspace`, Cargo unifies features
/// across every member, and `ada-m13-api-gateway` enables
/// `inproc-sessions` from its `[dev-dependencies]` so its own tests can
/// reach this double. The doctest was therefore compiled against a build
/// that *did* have the feature on, and could only ever pass or fail for
/// reasons unrelated to the property being claimed.
///
/// The property is real and is checked where it can actually be observed:
/// `no_production_dependency_enables_a_test_only_feature` in
/// `crates/ada-core/tests/ci_feature_coverage.rs` asserts that
/// `inproc-sessions` appears in the workspace only in
/// `[dev-dependencies]`, which is what makes a normal build of this crate
/// resolve without it. A feature flag cannot prove its own absence from
/// a build that has it switched on.
#[async_trait]
pub trait SessionStorage: Send + Sync {
    /// Store `session` and return its opaque token.
    ///
    /// Reclaims expired entries first. Fails closed with
    /// [`IdentityError::SessionStoreFull`] rather than evicting a live
    /// session if the store is still at its ceiling.
    async fn mint(&self, session: Session) -> Result<String>;

    /// Resolve a token to its [`Session`], or `None` if it is unknown
    /// or expired.
    ///
    /// `Err` is reserved for "the store could not answer", never for
    /// "no such session". Collapsing the two would let a backend outage
    /// look like a logged-out user.
    async fn lookup(&self, token: &str) -> Result<Option<Session>>;

    /// Drop `token` if present. Idempotent.
    async fn revoke(&self, token: &str) -> Result<()>;

    /// Remove every entry whose deadline has passed, returning how many
    /// went.
    ///
    /// Both backends also reclaim on `mint` and on the `lookup` that
    /// observes a death; this exists for the replica that stopped
    /// minting.
    async fn sweep_expired(&self) -> Result<usize>;
}

/// In-process session storage. **Test-only.**
///
/// # Not available in a production build
///
/// Gated on `cfg(any(test, feature = "inproc-sessions"))`. Naming this
/// type from a production build is a compile error, which is the point:
/// a per-process map cannot serve two replicas, so a build that quietly
/// accepted one would ship a login that works on one pod and 404s on
/// the next request that lands elsewhere. Enable the feature from a
/// *test* build to get the double back:
///
/// ```toml
/// [dev-dependencies]
/// ada-identity = { path = "../ada-identity", features = ["inproc-sessions"] }
/// ```
///
/// The production path is
/// [`SharedSessionStore`](crate::shared_session::SharedSessionStore).
/// Use [`SessionStorage`] when the choice should not be visible at the
/// call site.
///
/// The semantics below — sweep on mint, fail closed at the ceiling,
/// reclaim on the read that observed a death — are the contract
/// [`SessionStorage`] states, and the shared backend reproduces all
/// three. See the module docs for why they exist.
///
/// [`SessionStore::new`] and the other inherent methods are kept
/// synchronous, so a test that wants a plain map does not have to go
/// through a runtime to get one.
#[derive(Debug)]
#[cfg(any(test, feature = "inproc-sessions"))]
pub struct SessionStore {
    by_token: RwLock<HashMap<String, Session>>,
    max_sessions: usize,
}

#[cfg(any(test, feature = "inproc-sessions"))]
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
        let token = new_session_token();

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

#[cfg(any(test, feature = "inproc-sessions"))]
impl Default for SessionStore {
    fn default() -> Self {
        Self::new()
    }
}

/// The in-process store as a [`SessionStorage`].
///
/// Each method forwards to the synchronous inherent method of the same
/// name — written out in full rather than as `self.mint(..)`, which
/// would resolve to the inherent method and make the forwarding look
/// recursive.
#[cfg(any(test, feature = "inproc-sessions"))]
#[async_trait]
impl SessionStorage for SessionStore {
    async fn mint(&self, session: Session) -> Result<String> {
        SessionStore::mint(self, session)
    }

    async fn lookup(&self, token: &str) -> Result<Option<Session>> {
        Ok(SessionStore::lookup(self, token))
    }

    async fn revoke(&self, token: &str) -> Result<()> {
        SessionStore::revoke(self, token);
        Ok(())
    }

    async fn sweep_expired(&self) -> Result<usize> {
        Ok(SessionStore::sweep_expired(self))
    }
}

/// Build a session expiring `ttl` from now. Test convenience so a test
/// cannot accidentally express its intent in the wrong unit.
#[must_use]
pub fn session_ttl(ttl: Duration) -> Instant {
    Instant::now() + ttl
}

#[cfg(any(test, feature = "inproc-sessions"))]
#[cfg(test)]
mod inproc_tests {
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

/// Tests for the parts of this module that are not the in-process
/// store: the token format, the persistable record, and the trait both
/// backends are held to. Deliberately not gated on
/// `inproc-sessions` — these describe the production contract, so they
/// must run in a build where the in-process store does not exist.
#[cfg(test)]
mod contract_tests {
    use super::*;

    fn session(ttl: Duration) -> Session {
        Session {
            user_id: "u1".into(),
            tenant_id: "t1".into(),
            roles: vec!["owner".into(), "viewer".into()],
            expires_at: session_ttl(ttl),
        }
    }

    /// The token is the only handle to a session, so its length is a
    /// security property and not a formatting detail.
    #[test]
    fn a_session_token_is_256_bits_of_randomness() {
        let decoded = crate::base64util::b64url_decode(&new_session_token()).expect("base64url");
        assert_eq!(decoded.len(), 32, "32 bytes = 256 bits of entropy");
    }

    /// Base64 of CSPRNG output is not a counter, but a generator that
    /// repeated or shortened would hand out duplicate credentials, and
    /// a duplicate token silently resolves to the wrong session.
    #[test]
    fn session_tokens_are_distinct() {
        let first = new_session_token();
        let second = new_session_token();
        assert_ne!(first, second, "two mints must not collide");
    }

    /// The record is what crosses the network, so it has to survive a
    /// serialisation round trip with the authorization-relevant fields
    /// intact. Losing `tenant_id` here would be a cross-tenant leak;
    /// losing `roles` would be a silent privilege change.
    #[test]
    fn a_record_survives_a_json_round_trip_intact() {
        let record = SessionRecord::from_session(&session(Duration::from_secs(60)));
        let wire = serde_json::to_string(&record).expect("serialise");
        let back: SessionRecord = serde_json::from_str(&wire).expect("deserialise");

        assert_eq!(back.user_id, "u1");
        assert_eq!(back.tenant_id, "t1");
        assert_eq!(back.roles, vec!["owner".to_string(), "viewer".to_string()]);
        assert_eq!(
            back.expires_at_unix_ms, record.expires_at_unix_ms,
            "the deadline must not drift through the wire"
        );
    }

    /// The deadline is absolute, not a duration. If it were relative,
    /// every replica would re-anchor it to its own start-up and a
    /// session would live forever, which is the same class of bug as
    /// never expiring at all.
    #[test]
    fn a_record_carries_an_absolute_deadline_not_a_duration() {
        // One millisecond of slack on the lower bound, because that is the
        // precision `from_session` actually has: it converts the remaining
        // lifetime with `as_millis()`, which truncates, so the stored
        // deadline can land up to 1 ms *short* of the true expiry.
        //
        // The bound as it stood -- `deadline >= before + 60_000` -- asserted
        // a precision the field does not carry, and failed on CI by exactly
        // one millisecond (deadline 1791282363194 against now 1791282303195,
        // i.e. 59 999 ms out rather than 60 000). A session expiring a
        // millisecond early is not a defect worth changing behaviour over:
        // the deadline is already wall-clock and therefore already assumes
        // the pods' clocks agree, which `SessionRecord`'s own docs state.
        // Claiming sub-millisecond exactness here only tested the rounding
        // mode of a conversion.
        //
        // Declared before the bindings because a `const` after a statement
        // is `clippy::items_after_statements` -- items exist from the start
        // of the scope, so putting it here reads the way the code means it.
        const TRUNCATION_SLACK_MS: i64 = 1;

        let before = unix_millis_now();
        let record = SessionRecord::from_session(&session(Duration::from_secs(60)));
        let after = unix_millis_now();
        let deadline = record.expires_at_unix_ms;

        assert!(
            deadline + TRUNCATION_SLACK_MS >= before + 60_000 && deadline <= after + 60_000,
            "deadline must sit ~60s past now (+/-{TRUNCATION_SLACK_MS}ms \
             for the truncation in as_millis), got {deadline} against \
             now {after}"
        );
    }

    /// A restart is the case a per-process store cannot survive. The
    /// record still says when the session dies, so a fresh process
    /// reading it must agree with the one that wrote it.
    #[test]
    fn a_live_record_read_by_a_fresh_process_keeps_its_deadline() {
        let record = SessionRecord::from_session(&session(Duration::from_secs(60)));
        let now = unix_millis_now();

        let recovered = record.to_session(now).expect("still live");
        let remaining = recovered
            .expires_at
            .saturating_duration_since(Instant::now());
        assert!(
            remaining <= Duration::from_secs(60),
            "a restarted process must not extend the session, got {remaining:?}"
        );
        assert!(
            remaining > Duration::from_secs(55),
            "…nor lose the remaining life, got {remaining:?}"
        );
    }

    /// The mirror of the above, and the one that matters: a deadline
    /// that has passed is refused by whoever reads it, including a
    /// process that never saw the mint.
    #[test]
    fn a_dead_record_is_refused_by_any_process() {
        let record = SessionRecord::from_session(&session(Duration::from_secs(60)));
        assert!(
            record.to_session(record.expires_at_unix_ms).is_none(),
            "expiry is inclusive: at the deadline the session is already over"
        );
        assert!(record.to_session(record.expires_at_unix_ms + 1).is_none());
    }

    /// An already-expired session converts without panicking and lands
    /// as "expired", not as a session in the past. `Instant::duration_since`
    /// panics on a negative gap; the record path has to be the one that
    /// cannot.
    #[test]
    fn an_already_expired_session_records_as_expired_rather_than_panicking() {
        let record = SessionRecord::from_session(&session(Duration::from_secs(0)));
        assert!(
            record.expires_at_unix_ms <= unix_millis_now(),
            "zero remaining life must record a deadline that has already passed"
        );
    }

    /// The trait has to be usable as a trait object, through an
    /// `async` dispatch: that is how a caller holds "whatever backend
    /// this deployment configured" without naming either one. `Arc<dyn
    /// SessionStorage>` is the shape the api-gateway needs.
    #[tokio::test]
    async fn the_in_process_store_is_usable_as_a_dyn_session_storage() {
        let store = SessionStore::new();
        let as_dyn: Box<dyn SessionStorage> = Box::new(store);

        let token = as_dyn
            .mint(session(Duration::from_secs(60)))
            .await
            .expect("mint through the trait object");
        let found = as_dyn
            .lookup(&token)
            .await
            .expect("lookup through the trait object")
            .expect("the session is live");
        assert_eq!(found.tenant_id, "t1");
        as_dyn.revoke(&token).await.expect("revoke");
        assert!(as_dyn.lookup(&token).await.expect("lookup").is_none());
    }
}
