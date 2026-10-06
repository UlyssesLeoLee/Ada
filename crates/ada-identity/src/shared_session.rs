//! Production session storage: a session table every replica can reach.
//!
//! `session::SessionStore` answers "is this token valid" correctly and
//! only inside the process that minted it. Two replicas behind a load
//! balancer mean a login that succeeds on pod A and 401s on pod B, and
//! a deploy that logs out every user. This module is the production
//! answer, and it is split in two on purpose:
//!
//! - [`SharedSessionBackend`] is the wire: five operations against shared storage. The
//!   production implementation is
//!   [`RedisSessionBackend`](crate::redis_session::RedisSessionBackend),
//!   built by the api-gateway from `ADA_SESSION_REDIS_URL`, which
//!   fails closed when unset. `Cargo.toml` records that decision and
//!   its reason. A Postgres `session` table was named here when this
//!   module shipped as a bare trait and was never built: no such
//!   table exists in `db/migrations`, and this crate has no Postgres
//!   client in its dependency tree. A second backend can still be
//!   added by implementing the trait; nothing here assumes one.
//! - [`SharedSessionStore`] is the session semantics: token minting,
//!   expiry, the ceiling, and the refusal to report a storage failure as
//!   a missing session. Those live here so both backends are tested
//!   against the same guarantees instead of each drifting.
//!
//! ## What sharing costs
//!
//! Two things this module gives up, stated rather than discovered:
//!
//! - **Atomicity of the ceiling.** The in-process store checks its
//!   count and inserts under one lock. Here the two are separate round
//!   trips, so N replicas racing past the ceiling can admit a few more
//!   sessions than [`DEFAULT_MAX_SESSIONS`](crate::session::DEFAULT_MAX_SESSIONS).
//!   The ceiling stays a fail-closed circuit breaker against a runaway
//!   mint loop, not a hard admission gate; the hard gate is a row cap
//!   or a partition limit in the backend.
//! - **Monotonic expiry.** A shared deadline is wall-clock, because
//!   `Instant` does not survive serialisation. See
//!   [`SessionRecord`](crate::session::SessionRecord).
//!
//! Neither is a reason to go back to per-process state: they bound a
//! misconfiguration, whereas per-process state breaks every multi-replica
//! deployment on the first request that crosses a pod boundary.

use async_trait::async_trait;

use crate::error::{IdentityError, Result};
use crate::session::{
    new_session_token, unix_millis_now, Session, SessionRecord, SessionStorage,
    DEFAULT_MAX_SESSIONS,
};

/// The shared storage a [`SharedSessionStore`] is built on.
///
/// Implemented by the deployment, not by this crate. Every method
/// returns [`IdentityError::SessionBackend`] on failure, and **must not
/// put a session token in that message** — a token in a log line is a
/// live credential.
#[async_trait]
pub trait SharedSessionBackend: Send + Sync {
    /// Read one session row, or `None` if the key is unknown.
    ///
    /// A key that is absent must be `Ok(None)`. `Err` is reserved for
    /// "the store could not answer".
    async fn get(&self, key: &str) -> Result<Option<SessionRecord>>;

    /// Write one session row, replacing any row at that key.
    async fn put(&self, key: &str, value: SessionRecord) -> Result<()>;

    /// Remove one row. Must succeed whether or not the key existed, so
    /// that `revoke` stays idempotent.
    async fn delete(&self, key: &str) -> Result<()>;

    /// Number of rows, expired ones included.
    ///
    /// The ceiling is checked against this, so a cheap `count` matters
    /// more than an exact one: a backend that cannot answer exactly
    /// should return a conservative over-estimate, which fails the mint
    /// closed rather than admitting one session too many.
    async fn count(&self) -> Result<usize>;

    /// Remove every row whose `expires_at_unix_ms` is at or before
    /// `now_unix_ms`, returning how many went.
    async fn delete_expired(&self, now_unix_ms: i64) -> Result<usize>;
}

/// A shared handle to a backend passes straight through.
///
/// A pooled client is a `Arc` (or the pool's own handle type) that every
/// request path shares, and requiring a wrapper struct to satisfy a
/// trait it already satisfies behind a pointer would push that wrapper
/// into every call site.
#[async_trait]
impl<T> SharedSessionBackend for std::sync::Arc<T>
where
    T: SharedSessionBackend + ?Sized,
{
    async fn get(&self, key: &str) -> Result<Option<SessionRecord>> {
        (**self).get(key).await
    }

    async fn put(&self, key: &str, value: SessionRecord) -> Result<()> {
        (**self).put(key, value).await
    }

    async fn delete(&self, key: &str) -> Result<()> {
        (**self).delete(key).await
    }

    async fn count(&self) -> Result<usize> {
        (**self).count().await
    }

    async fn delete_expired(&self, now_unix_ms: i64) -> Result<usize> {
        (**self).delete_expired(now_unix_ms).await
    }
}

/// Session storage shared across replicas.
///
/// Holds the session semantics and delegates the wire. Two instances
/// over one backend are two replicas: a session minted through either
/// resolves through the other, and it survives the process that minted
/// it.
#[derive(Debug)]
pub struct SharedSessionStore<B> {
    backend: B,
    max_sessions: usize,
}

impl<B> SharedSessionStore<B> {
    /// A store with the default ceiling.
    #[must_use]
    pub fn new(backend: B) -> Self {
        Self::with_max_sessions(backend, DEFAULT_MAX_SESSIONS)
    }

    /// A store admitting at most `max_sessions` rows.
    ///
    /// A ceiling of 0 refuses every mint, which is a coherent
    /// configuration and is what a test asserting the bound needs.
    #[must_use]
    pub fn with_max_sessions(backend: B, max_sessions: usize) -> Self {
        Self {
            backend,
            max_sessions,
        }
    }

    /// The configured ceiling.
    #[must_use]
    pub fn max_sessions(&self) -> usize {
        self.max_sessions
    }

    /// The underlying backend, for a health probe or a raw read.
    #[must_use]
    pub fn backend(&self) -> &B {
        &self.backend
    }
}

#[async_trait]
impl<B> SessionStorage for SharedSessionStore<B>
where
    B: SharedSessionBackend,
{
    /// Sweep, check the ceiling, then write — the same order the
    /// in-process store uses, and the reason a burst of expiring
    /// sessions cannot leave the store permanently refusing new ones.
    async fn mint(&self, session: Session) -> Result<String> {
        let token = new_session_token();
        let record = SessionRecord::from_session(&session);

        self.sweep_expired().await?;
        if self.backend.count().await? >= self.max_sessions {
            return Err(IdentityError::SessionStoreFull(self.max_sessions));
        }
        self.backend.put(&token, record).await?;
        Ok(token)
    }

    /// An expired row is deleted on the read that observed it, exactly
    /// as the in-process store does. Without that, a session nobody
    /// looks up again — the common case, a user who closed the tab —
    /// occupies a row until something happens to sweep.
    async fn lookup(&self, token: &str) -> Result<Option<Session>> {
        let Some(record) = self.backend.get(token).await? else {
            return Ok(None);
        };
        if let Some(session) = record.to_session(unix_millis_now()) {
            return Ok(Some(session));
        }
        self.backend.delete(token).await?;
        Ok(None)
    }

    async fn revoke(&self, token: &str) -> Result<()> {
        self.backend.delete(token).await
    }

    async fn sweep_expired(&self) -> Result<usize> {
        self.backend.delete_expired(unix_millis_now()).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base64util::b64url_decode;
    // Only built under `cfg(test)` or the `inproc-sessions` feature, so
    // this import is only legal in a test build — which is the point.
    use crate::session::SessionStore;
    use parking_lot::Mutex;
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    /// Stands in for the Redis/Postgres client a deployment supplies.
    ///
    /// Serialises through `serde_json` rather than holding
    /// [`SessionRecord`] values, because that is what a real client
    /// does: if a field is not actually serialisable, this double must
    /// not paper over it. One `FakeSharedBackend` is one namespace, and
    /// two handles onto it are two replicas of the same deployment.
    #[derive(Debug, Default)]
    struct FakeSharedBackend {
        rows: Mutex<HashMap<String, String>>,
        /// When set, every operation fails. Models an outage.
        down: Mutex<bool>,
    }

    impl FakeSharedBackend {
        fn rows(&self) -> usize {
            self.rows.lock().len()
        }

        fn go_down(&self) {
            *self.down.lock() = true;
        }

        fn check(&self) -> Result<()> {
            if *self.down.lock() {
                return Err(IdentityError::SessionBackend("connection refused".into()));
            }
            Ok(())
        }
    }

    #[async_trait]
    impl SharedSessionBackend for FakeSharedBackend {
        async fn get(&self, key: &str) -> Result<Option<SessionRecord>> {
            self.check()?;
            let rows = self.rows.lock();
            rows.get(key)
                .map(|raw| {
                    serde_json::from_str(raw).map_err(|e| {
                        IdentityError::SessionBackend(format!("decode session row: {e}"))
                    })
                })
                .transpose()
        }

        async fn put(&self, key: &str, value: SessionRecord) -> Result<()> {
            self.check()?;
            let raw = serde_json::to_string(&value)
                .map_err(|e| IdentityError::SessionBackend(format!("encode session row: {e}")))?;
            self.rows.lock().insert(key.to_owned(), raw);
            Ok(())
        }

        async fn delete(&self, key: &str) -> Result<()> {
            self.check()?;
            self.rows.lock().remove(key);
            Ok(())
        }

        async fn count(&self) -> Result<usize> {
            self.check()?;
            Ok(self.rows.lock().len())
        }

        async fn delete_expired(&self, now_unix_ms: i64) -> Result<usize> {
            self.check()?;
            let mut rows = self.rows.lock();
            let before = rows.len();
            // A row that will not decode is kept, not swept: the
            // fault is in how it was written, and deleting a record
            // this code cannot read is how a session table loses data.
            rows.retain(|_, raw| {
                serde_json::from_str::<SessionRecord>(raw)
                    .map_or(true, |r| r.expires_at_unix_ms > now_unix_ms)
            });
            Ok(before - rows.len())
        }
    }

    fn session(ttl: Duration) -> Session {
        Session {
            user_id: "u1".into(),
            tenant_id: "t1".into(),
            roles: vec!["owner".into()],
            expires_at: Instant::now() + ttl,
        }
    }

    /// One deployment, two replicas.
    fn two_replicas(
        max_sessions: usize,
    ) -> (
        SharedSessionStore<Arc<FakeSharedBackend>>,
        SharedSessionStore<Arc<FakeSharedBackend>>,
        Arc<FakeSharedBackend>,
    ) {
        let backend = Arc::new(FakeSharedBackend::default());
        (
            SharedSessionStore::with_max_sessions(Arc::clone(&backend), max_sessions),
            SharedSessionStore::with_max_sessions(Arc::clone(&backend), max_sessions),
            backend,
        )
    }

    /// **The defect.** A session minted on one replica has to be a
    /// credential on the next one. The in-process store cannot do this
    /// and never could; this is the assertion that says the production
    /// backend is not that store.
    #[tokio::test]
    async fn a_session_minted_on_one_replica_is_valid_on_the_next() {
        let (pod_a, pod_b, _backend) = two_replicas(10);

        let token = pod_a
            .mint(session(Duration::from_secs(60)))
            .await
            .expect("mint on pod A");

        let found = pod_b
            .lookup(&token)
            .await
            .expect("lookup on pod B")
            .expect("pod B must resolve a session pod A minted");
        assert_eq!(found.user_id, "u1");
        assert_eq!(found.tenant_id, "t1");
        assert_eq!(found.roles, vec!["owner".to_string()]);
    }

    /// The other half of the defect: a restart is indistinguishable
    /// from a fresh pod, so it must not invalidate anything. The old
    /// store's `HashMap` is exactly what made this impossible.
    #[tokio::test]
    async fn a_restart_does_not_invalidate_an_outstanding_credential() {
        let backend = Arc::new(FakeSharedBackend::default());

        let token = SharedSessionStore::new(Arc::clone(&backend))
            .mint(session(Duration::from_secs(600)))
            .await
            .expect("mint before the restart");
        // The first store is gone: a restart is a new process, not a
        // cleared map. Nothing here holds it any more.
        drop(backend.clone());

        let after_restart = SharedSessionStore::new(Arc::clone(&backend));
        let found = after_restart
            .lookup(&token)
            .await
            .expect("lookup after the restart")
            .expect("the credential outlives the process that minted it");
        assert_eq!(found.user_id, "u1");
    }

    /// The two backends must not be interchangeable, and this is the
    /// assertion that proves it rather than assuming it: the same
    /// lifecycle, run against each, has to diverge at exactly the
    /// sharing step. A test that passed identically against both would
    /// prove nothing about either.
    #[tokio::test]
    async fn the_two_backends_differ_exactly_where_the_defect_was() {
        let (pod_a, pod_b, _shared) = two_replicas(10);
        let here = SessionStore::new();
        let over_there = SessionStore::new();

        let shared_token = pod_a
            .mint(session(Duration::from_secs(60)))
            .await
            .expect("mint on the shared backend");
        let local_token = SessionStore::mint(&here, session(Duration::from_secs(60)))
            .expect("mint on the in-process store");

        assert!(
            pod_b.lookup(&shared_token).await.expect("lookup").is_some(),
            "the shared backend crosses a replica boundary"
        );
        assert!(
            SessionStore::lookup(&over_there, &local_token).is_none(),
            "the in-process store does not, which is why it is test-only"
        );
    }

    /// A row whose deadline has passed must be reclaimed by the read
    /// that observed it, or sessions nobody looks up again pile up
    /// until the next sweep.
    #[tokio::test]
    async fn an_expired_session_is_deleted_from_the_backend_when_looked_up() {
        let (store, _other, backend) = two_replicas(10);
        let token = store
            .mint(session(Duration::from_secs(0)))
            .await
            .expect("mint");
        assert_eq!(backend.rows(), 1, "precondition: the row exists");

        assert!(store.lookup(&token).await.expect("lookup").is_none());
        assert_eq!(
            backend.rows(),
            0,
            "an expired session must not stay resident after the read that observed it"
        );
    }

    /// Same guarantee, on a session that is dead when it is written —
    /// which is how a test asserts expiry without sleeping, and how a
    /// mint against an already-stale clock behaves.
    #[tokio::test]
    async fn a_session_that_expires_immediately_never_becomes_a_live_row() {
        let (store, _other, backend) = two_replicas(10);
        let token = store
            .mint(session(Duration::from_secs(0)))
            .await
            .expect("mint");

        assert!(store.lookup(&token).await.expect("lookup").is_none());
        assert_eq!(backend.rows(), 0);
    }

    /// The ceiling is inherited from the store this replaces, so it has
    /// to behave the same way: fail closed, and never as a reason to
    /// evict somebody who is already signed in.
    #[tokio::test]
    async fn the_ceiling_refuses_the_new_session_without_disturbing_the_live_ones() {
        let (pod_a, pod_b, _backend) = two_replicas(2);
        let first = pod_a
            .mint(session(Duration::from_secs(600)))
            .await
            .expect("mint 1");
        let second = pod_a
            .mint(session(Duration::from_secs(600)))
            .await
            .expect("mint 2");

        let err = pod_b
            .mint(session(Duration::from_secs(600)))
            .await
            .expect_err("a third session exceeds a ceiling of two");
        assert!(
            matches!(err, IdentityError::SessionStoreFull(2)),
            "expected SessionStoreFull(2), got {err:?}"
        );

        assert!(
            pod_b.lookup(&first).await.expect("lookup").is_some(),
            "a refused mint must not log out a live session"
        );
        assert!(pod_b.lookup(&second).await.expect("lookup").is_some());
    }

    /// Minting sweeps first, so expiries drain the ceiling rather than
    /// wedging it. Without the sweep, a deployment that had a busy
    /// minute would refuse logins forever after.
    #[tokio::test]
    async fn accumulated_expiries_never_wedge_the_ceiling() {
        let (store, _other, _backend) = two_replicas(3);
        for i in 0..20 {
            store
                .mint(session(Duration::from_secs(0)))
                .await
                .unwrap_or_else(|e| panic!("mint {i} must fit: {e}"));
        }
        store
            .mint(session(Duration::from_secs(600)))
            .await
            .expect("a live session must fit after the dead ones drained");
    }

    /// `revoke` is the logout path, so it has to reach the shared row
    /// — a revoke that only forgot the local copy of the answer would
    /// leave the credential live on every other replica.
    #[tokio::test]
    async fn revoke_removes_the_shared_row_and_is_idempotent() {
        let (pod_a, pod_b, backend) = two_replicas(10);
        let token = pod_a
            .mint(session(Duration::from_secs(600)))
            .await
            .expect("mint");

        pod_b.revoke(&token).await.expect("revoke");
        assert_eq!(backend.rows(), 0, "revoke must free the row");
        assert!(pod_a.lookup(&token).await.expect("lookup").is_none());
        pod_b.revoke(&token).await.expect("revoking twice is fine");
    }

    /// The failure mode this design is most careful about. A backend
    /// outage must not be reportable as "no such session": the caller
    /// would log every user out on a network blip, and a caller that
    /// treated the error as an authorization answer would fail open.
    #[tokio::test]
    async fn a_backend_outage_is_an_error_not_a_missing_session() {
        let (store, _other, backend) = two_replicas(10);
        let token = store
            .mint(session(Duration::from_secs(600)))
            .await
            .expect("mint");
        backend.go_down();

        let err = store
            .lookup(&token)
            .await
            .expect_err("an outage must not look like a logged-out user");
        assert!(
            matches!(err, IdentityError::SessionBackend(_)),
            "expected SessionBackend, got {err:?}"
        );
        assert!(
            !err.to_string().contains(&token),
            "a live credential must not reach an error message: {err}"
        );

        store
            .mint(session(Duration::from_secs(60)))
            .await
            .expect_err("a mint against a down backend must fail");
        store.revoke(&token).await.expect_err("revoke too");
        store.sweep_expired().await.expect_err("sweep too");
    }

    /// An unknown token is not an error. Only a token that is absent
    /// from a *reachable* store is `None`.
    #[tokio::test]
    async fn an_unknown_token_resolves_to_nothing_without_erroring() {
        let (store, _other, _backend) = two_replicas(10);
        assert!(store
            .lookup("never-minted")
            .await
            .expect("lookup")
            .is_none());
        assert!(store.lookup("").await.expect("lookup").is_none());
    }

    /// The sweep exists for the replica that stopped minting. It has to
    /// count what it removed, and leave live rows alone.
    #[tokio::test]
    async fn a_sweep_reclaims_the_dead_and_keeps_the_live() {
        let (store, _other, backend) = two_replicas(10);
        store
            .mint(session(Duration::from_secs(600)))
            .await
            .expect("mint live");
        for _ in 0..3 {
            store
                .mint(session(Duration::from_secs(0)))
                .await
                .expect("mint dead");
        }
        // A dead row written after the live ones is not collected by
        // the sweep the next mint performed, which is what leaves
        // something for the manual sweep to do.
        //
        // Captured *before* the sweep. It used to be read inline in the
        // assertion below, where it measures the post-sweep state: by
        // then `rows()` is already 1, so `rows() - 1` is 0 and the
        // assertion demanded that a sweep which correctly reclaimed three
        // rows had reclaimed none. It failed on CI with `left: 1,
        // right: 0` -- the count and the arithmetic, not the sweep.
        let resident_before = backend.rows();
        assert!(resident_before > 1, "precondition: dead rows are resident");

        let reclaimed = store.sweep_expired().await.expect("sweep");
        assert_eq!(reclaimed, resident_before - 1, "only the dead go");
        assert_eq!(backend.rows(), 1);
        assert_eq!(
            store.sweep_expired().await.expect("sweep again"),
            0,
            "nothing is left to reclaim"
        );
    }

    /// The token is minted here, so a caller cannot end up with a
    /// shared store that issues a different credential shape than the
    /// store it replaced.
    #[tokio::test]
    async fn the_shared_store_mints_256_bit_tokens() {
        let (store, _other, _backend) = two_replicas(10);
        let token = store
            .mint(session(Duration::from_secs(60)))
            .await
            .expect("mint");
        assert_eq!(b64url_decode(&token).expect("base64url").len(), 32);
    }

    /// The ceiling the shared store advertises has to be the one it
    /// enforces, and it has to default to the same number the
    /// in-process store used, so "full" means one thing per deployment.
    #[test]
    fn the_shared_store_uses_the_documented_default_ceiling() {
        let backend = FakeSharedBackend::default();
        assert_eq!(
            SharedSessionStore::new(backend).max_sessions(),
            DEFAULT_MAX_SESSIONS
        );
    }
}
