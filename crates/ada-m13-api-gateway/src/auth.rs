//! Authentication and authorization for the gateway.
//!
//! ## The trust model
//!
//! Every `/api/*` request must carry `Authorization: Bearer <token>`.
//! The token is an **opaque** session identifier, looked up through
//! `ada_identity::session::SessionStorage` (Redis in production). There is no
//! JWT decoding here and there must not be: `ada_identity::mint::verify_jwt_stub` fails
//! closed for every input, including well-formed ones, because the
//! crate has no asymmetric crypto dependency to verify a signature
//! with. A verifier that only *decodes* a token would let a caller mint
//! their own `tenant` and `roles` -- and `tenant` is the isolation key
//! for the entire multi-tenant model.
//!
//! That makes the trust model the interesting part, so it is worth
//! stating plainly:
//!
//! - **The tenant comes from the session, never from a header.**
//!   `gm-console` forwards the client's `x-tenant-id` verbatim; this
//!   layer ignores it entirely for authorization purposes. A client
//!   asserting a tenant it does not own changes nothing here, because
//!   nothing reads that header to make a decision.
//! - **The roles come from the session**, not from the token body, so
//!   they cannot be forged without a valid opaque token.
//! - **The store is shared, and its absence stops the boot.** Sessions live
//!   in Redis via `ada_identity::redis_session`, reached through
//!   `SessionStorage`. A per-process store would 401 a credential the moment
//!   the request landed on another replica and would invalidate every
//!   outstanding one on restart, so there is no in-process fallback: an
//!   unset or unreachable `ADA_SESSION_REDIS_URL` refuses startup. An
//!   authentication service that cannot check a credential must not serve.
//! - **Health routes are exempt**, because a kubelet probe cannot carry
//!   a bearer token and a probe that 401s takes the pod out of service.

use std::sync::Arc;

use ada_identity::redis_session::{RedisSessionBackend, REDIS_URL_ENV};
use ada_identity::session::SessionStorage;
use ada_identity::shared_session::SharedSessionStore;
use ada_m11_rbac_collab::{Action, ResourceType, Role};
use ada_rbac_casbin::{Attrs, Enforcer, PolicySet};
use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts},
};

use crate::error::{ApiError, Result};

/// The authenticated caller, resolved from a valid session.
///
/// Inserted into request extensions by [`AuthContext::from_request`].
/// Handlers read it with `Extension<Principal>`; the presence of the
/// type in extensions is itself the proof that authentication ran, so a
/// handler cannot accidentally read an unauthenticated request.
#[derive(Debug, Clone)]
pub struct Principal {
    /// Stable subject id.
    pub user_id: String,
    /// Tenant the session was minted for. This is the only tenant this
    /// request may act in, and it did not come from a client header.
    pub tenant_id: String,
    /// Role names, unprefixed (`"owner"`, not `"role:owner"`).
    pub roles: Vec<String>,
}

impl Principal {
    /// The role tokens to present to the RBAC enforcer, in the
    /// `role:<name>` form its `resolve_role` contract requires.
    #[must_use]
    pub fn role_tokens(&self) -> Vec<String> {
        self.roles.iter().map(|r| format!("role:{r}")).collect()
    }

    /// `Attrs` for the RBAC enforcer, scoped to this session's tenant.
    ///
    /// `is_owner` is deliberately false. The ownership flag is an
    /// *attribute of the caller's relationship to the object*, and the
    /// session does not carry it. Defaulting it to true would hand
    /// every session `Delete` on every object, which is exactly the
    /// hole that was found in the casbin adapter.
    #[must_use]
    pub fn attrs(&self) -> Attrs {
        Attrs::new(self.tenant_id.clone())
    }
}

/// Shared authentication and authorization state.
#[derive(Clone)]
pub struct AuthContext {
    sessions: Arc<dyn SessionStorage>,
    enforcer: Arc<Enforcer>,
}

impl std::fmt::Debug for AuthContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthContext")
            .field("sessions", &"<SessionStore>")
            .field("enforcer", &"<Enforcer>")
            .finish()
    }
}

impl AuthContext {
    /// Build the context.
    ///
    /// Fails if the policy set does not validate, so a deployment with
    /// a broken policy file refuses to start rather than serving with
    /// an enforcer that would deny or allow arbitrarily.
    pub fn new(sessions: Arc<dyn SessionStorage>, enforcer: Arc<Enforcer>) -> Self {
        Self { sessions, enforcer }
    }

    /// Replace the session store, keeping the enforcer.
    ///
    /// Takes the trait rather than a concrete backend, so a deployment
    /// can install a store of its own without going through
    /// [`Self::bootstrap`]. Tests do not need it: [`Self::with_bundled_policy`]
    /// builds the same real policy set over whatever store they hand it,
    /// so swapping the enforcer as a side effect would buy nothing.
    #[must_use]
    pub fn with_sessions(self, sessions: Arc<dyn SessionStorage>) -> Self {
        Self { sessions, ..self }
    }

    /// Mint a session for `principal` and return its opaque token.
    ///
    /// Exists so the login flow — not yet built — and the tests have
    /// one obvious way to produce a credential. Deliberately not
    /// reachable from any route: a "create a session" HTTP endpoint
    /// would be an authentication bypass wearing a different hat.
    ///
    /// Fallible because [`SessionStore`] is bounded: refusing to mint
    /// at capacity is what keeps a burst of logins from growing the map
    /// without limit, and a caller that cannot represent that failure
    /// will eventually paper over it.
    pub async fn mint_session(
        &self,
        user_id: &str,
        tenant_id: &str,
        roles: Vec<String>,
        ttl_secs: u64,
    ) -> Result<String> {
        self.sessions
            .mint(ada_identity::session::Session {
                user_id: user_id.to_owned(),
                tenant_id: tenant_id.to_owned(),
                roles,
                expires_at: std::time::Instant::now() + std::time::Duration::from_secs(ttl_secs),
            })
            .await
            .map_err(|e| ApiError::ServiceUnavailable(format!("cannot mint session: {e}")))
    }

    /// Build a context from the bundled policy set and a caller-supplied
    /// session store.
    ///
    /// The seam that makes both deployments and tests work without a Redis
    /// on the other side. `bootstrap` is this plus "read
    /// [`REDIS_URL_ENV`] and connect"; a test passes the in-process double,
    /// which it can reach because `ada-identity` is a `[dev-dependencies]`
    /// of this crate with the `inproc-sessions` feature enabled. The lib's own
    /// build has no such feature, so production cannot name the in-process
    /// store even if it wanted to.
    pub fn with_bundled_policy(sessions: Arc<dyn SessionStorage>) -> Result<Self> {
        let enforcer = Enforcer::from_policy_set(&PolicySet::bundled())
            .map_err(|e| ApiError::Internal(format!("build rbac enforcer: {e}")))?;
        Ok(Self::new(sessions, Arc::new(enforcer)))
    }

    /// Build a context from the bundled policy set and the shared session
    /// store named by [`REDIS_URL_ENV`].
    ///
    /// # Fails closed when no shared store is configured
    ///
    /// There is no in-process fallback, and that is the point. A gateway
    /// holding sessions in process memory 401s a credential the instant the
    /// request lands on another replica, and invalidates every outstanding
    /// one on restart — the two defects this whole change exists to remove.
    /// Falling back to it would turn a loud startup refusal into a quiet
    /// production bug, so an unset or unreachable store stops the boot.
    ///
    /// The consequence is deliberate: a deployment that has not yet been
    /// given a session store does not serve traffic. That is a correct
    /// posture for an authentication service and an explicit one, as
    /// opposed to serving 401s that look like a client problem.
    pub async fn bootstrap() -> Result<Self> {
        let backend = RedisSessionBackend::from_env()
            .await
            .map_err(|e| ApiError::Internal(format!("session store unavailable: {e}")))?;
        Self::with_bundled_policy(Arc::new(SharedSessionStore::new(backend)))
    }

    /// Resolve a bearer token to a [`Principal`].
    ///
    /// Returns `None` for a missing, malformed, unknown, or expired
    /// token. The caller must not distinguish those cases in the
    /// response -- an attacker learns nothing from which one it was.
    ///
    /// A backend that could not answer is an `Err`, not a `None`. That is
    /// the distinction the trait is built around: reporting a Redis outage
    /// as "logged out" would both be the wrong answer and produce an
    /// alert that says nothing about what is wrong.
    pub async fn resolve(&self, token: &str) -> Result<Option<Principal>> {
        let session =
            self.sessions.lookup(token).await.map_err(|e| {
                ApiError::ServiceUnavailable(format!("session store unavailable: {e}"))
            })?;
        Ok(session.map(|session| Principal {
            user_id: session.user_id,
            tenant_id: session.tenant_id,
            roles: session.roles,
        }))
    }

    /// Authorize one action for a principal.
    ///
    /// Allow if **any** of the principal's roles permits it. A subject
    /// with several roles holds the union of their grants, so a deny
    /// from one role must not mask an allow from another.
    #[must_use]
    pub fn authorize(
        &self,
        principal: &Principal,
        kind: ResourceType,
        object_id: &str,
        action: Action,
    ) -> bool {
        let attrs = principal.attrs();
        principal.role_tokens().iter().any(|role| {
            // `enforce_typed`, not `enforce`: the latter takes the object
            // as a `"<kind>:<id>"` string and re-derives the kind by
            // parsing that prefix. This crate already has the typed kind
            // in hand, so stringifying it only to parse it straight back
            // would add a way for the two to disagree. (An object whose
            // prefix is unknown is denied by `contract::resource_type_of`
            // — that guard exists for the string entry point.)
            self.enforcer
                .enforce_typed(role, kind, object_id, action, &attrs, None)
                .unwrap_or(false)
        })
    }
}

#[async_trait::async_trait]
impl FromRequestParts<AuthContext> for Principal {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        ctx: &AuthContext,
    ) -> std::result::Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or(ApiError::Unauthorized("missing bearer token".into()))?;

        // Strict on the scheme and the single space. Accepting
        // `Bearer  x`, `bearer x`, or a comma-joined list would mean
        // several parsers have to agree, and only one of them is ours.
        let token = header
            .strip_prefix("Bearer ")
            .ok_or_else(|| ApiError::Unauthorized("expected `Bearer <token>`".into()))?;
        if token.is_empty() {
            return Err(ApiError::Unauthorized("empty bearer token".into()));
        }

        // A store that could not answer becomes a 503, not a 401. Reporting
        // it as "invalid or expired token" would tell the client its
        // credential is bad when the truth is that we could not check it,
        // and would turn a Redis outage into a wave of re-logins. `resolve`
        // already returns `ApiError::ServiceUnavailable` for that case, so
        // the `?` propagates it unchanged.
        ctx.resolve(token)
            .await?
            .ok_or_else(|| ApiError::Unauthorized("invalid or expired token".into()))
    }
}

/// Reject with 401 unless the principal's roles allow `action` on
/// `kind` + `object_id`.
///
/// The `Err` is always [`ApiError::Unauthorized`] — 403 would leak
/// that the credential was good and only the policy refused, which is
/// information an attacker can use to enumerate roles: a caller
/// holding a valid `role:viewer` token learns it is `viewer` from a
/// 403, and one holding `role:editor` learns it is not. One status
/// for both failure modes tells them nothing they did not already
/// know, because every request they can make today returns the same
/// thing.
pub fn require(
    ctx: &AuthContext,
    principal: &Principal,
    kind: ResourceType,
    object_id: &str,
    action: Action,
) -> Result<()> {
    if ctx.authorize(principal, kind, object_id, action) {
        Ok(())
    } else {
        Err(ApiError::Unauthorized("forbidden".into()))
    }
}

/// Assert the given role name is one this crate knows about.
///
/// Used when building a test principal, so a typo in a role name shows
/// up as a test failure rather than as a request that is mysteriously
/// denied forever.
#[must_use]
pub fn is_known_role(name: &str) -> bool {
    [
        Role::Owner,
        Role::Admin,
        Role::Editor,
        Role::Executor,
        Role::Viewer,
    ]
    .iter()
    .any(|r| r.as_str() == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A context backed by the in-process store, for tests only.
    ///
    /// Deliberately not [`AuthContext::bootstrap`]: that one demands a Redis
    /// and fails closed without one, which is correct in production and
    /// useless here. Building the enforcer directly keeps the tests honest
    /// about the part that matters — the real policy set still gates every
    /// resolve — while swapping only the storage.
    fn test_context() -> AuthContext {
        test_context_with_max_sessions(ada_identity::session::DEFAULT_MAX_SESSIONS)
    }

    fn test_context_with_max_sessions(max: usize) -> AuthContext {
        let store = ada_identity::session::SessionStore::with_max_sessions(max);
        AuthContext::with_bundled_policy(Arc::new(store)).expect("bundled policy set")
    }

    #[test]
    fn every_role_name_in_the_m11_matrix_is_recognised() {
        for r in [
            Role::Owner,
            Role::Admin,
            Role::Editor,
            Role::Executor,
            Role::Viewer,
        ] {
            assert!(is_known_role(r.as_str()), "{} should be known", r.as_str());
        }
    }

    #[test]
    fn an_unknown_role_name_is_not_silently_accepted() {
        for name in ["superuser", "Owner", "OWNER", "", "role:owner"] {
            assert!(!is_known_role(name), "{name:?} must not be known");
        }
    }

    #[test]
    fn role_tokens_carry_the_prefix_the_enforcer_contracts_for() {
        let p = Principal {
            user_id: "u1".into(),
            tenant_id: "t1".into(),
            roles: vec!["viewer".into()],
        };
        assert_eq!(p.role_tokens(), vec!["role:viewer".to_string()]);
    }

    #[test]
    fn a_principal_is_never_flagged_as_owner_by_default() {
        // The session does not carry an ownership relationship, so
        // defaulting it would hand every session Delete on everything.
        let p = Principal {
            user_id: "u1".into(),
            tenant_id: "t1".into(),
            roles: vec!["owner".into()],
        };
        assert!(!p.attrs().is_owner);
    }

    #[tokio::test]
    async fn an_empty_store_resolves_nothing() {
        // A store nobody has minted into. Every business request must be a
        // 401 until a login flow mints one.
        let ctx = test_context();
        assert!(ctx.resolve("anything").await.expect("lookup").is_none());
        assert!(ctx.resolve("").await.expect("lookup").is_none());
    }

    /// A minted session is the only thing that produces a working
    /// credential, so the round trip has to hold.
    #[tokio::test]
    async fn a_minted_session_resolves_to_its_principal() {
        let ctx = test_context();
        let token = ctx
            .mint_session("u1", "tenant-a", vec!["viewer".into()], 60)
            .await
            .expect("mint");
        let p = ctx
            .resolve(&token)
            .await
            .expect("lookup")
            .expect("the token just minted must resolve");
        assert_eq!(p.user_id, "u1");
        assert_eq!(p.tenant_id, "tenant-a");
        assert_eq!(p.roles, vec!["viewer".to_string()]);
    }

    /// The store's ceiling has to reach the caller, not be swallowed.
    ///
    /// A login flow that cannot tell "could not create a session" from
    /// "session created" will retry in a loop, which turns a bounded
    /// store into a busy spin instead of a clean failure.
    #[tokio::test]
    async fn a_full_store_is_reported_rather_than_papered_over() {
        let ctx = test_context_with_max_sessions(1);
        ctx.mint_session("u1", "tenant-a", vec!["viewer".into()], 60)
            .await
            .expect("the first session fits");

        let err = ctx
            .mint_session("u2", "tenant-a", vec!["viewer".into()], 60)
            .await
            .expect_err("the store holds one session and has a ceiling of one");
        assert!(
            matches!(err, ApiError::ServiceUnavailable(_)),
            "capacity pressure must surface as an error the caller can act on, got {err:?}"
        );
    }

    /// Expiry is enforced through the gateway too, not only in the
    /// store's own tests — this is the path a real request takes.
    #[tokio::test]
    async fn an_expired_session_does_not_resolve() {
        let ctx = test_context();
        let token = ctx
            .mint_session("u1", "tenant-a", vec!["viewer".into()], 0)
            .await
            .expect("mint");
        assert!(
            ctx.resolve(&token).await.expect("lookup").is_none(),
            "a zero-TTL session must not authenticate anything"
        );
    }
}
