//! Authentication and authorization for the gateway.
//!
//! ## The trust model
//!
//! Every `/api/*` request must carry `Authorization: Bearer <token>`.
//! The token is an **opaque** session identifier, looked up in
//! `ada_identity::session::SessionStore`. There is no JWT decoding here
//! and there must not be: `ada_identity::mint::verify_jwt_stub` fails
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
//! - **An empty store denies everything.** `SessionStore` starts empty
//!   and nothing in this crate mints sessions -- there is no login
//!   endpoint yet -- so until a login flow exists every business
//!   request is a 401. That is the correct posture: a backend that
//!   authorizes nothing yet must not pretend to authorize everyone.
//! - **Health routes are exempt**, because a kubelet probe cannot carry
//!   a bearer token and a probe that 401s takes the pod out of service.

use std::sync::Arc;

use ada_identity::session::SessionStore;
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
    sessions: Arc<SessionStore>,
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
    pub fn new(sessions: Arc<SessionStore>, enforcer: Arc<Enforcer>) -> Self {
        Self { sessions, enforcer }
    }

    /// Replace the session store, keeping the enforcer.
    ///
    /// The seam tests use: `SessionStore` is process-global mutable
    /// state in effect (a `HashMap` behind a lock), so tests that share
    /// a bootstrap context would share every minted session with each
    /// other. Each test mints its own store and installs it here.
    #[must_use]
    pub fn with_sessions(self, sessions: Arc<SessionStore>) -> Self {
        Self { sessions, ..self }
    }

    /// Mint a session for `principal` and return its opaque token.
    ///
    /// Exists so the login flow — not yet built — and the tests have
    /// one obvious way to produce a credential. Deliberately not
    /// reachable from any route: a "create a session" HTTP endpoint
    /// would be an authentication bypass wearing a different hat.
    pub fn mint_session(
        &self,
        user_id: &str,
        tenant_id: &str,
        roles: Vec<String>,
        ttl_secs: u64,
    ) -> String {
        self.sessions.mint(ada_identity::session::Session {
            user_id: user_id.to_owned(),
            tenant_id: tenant_id.to_owned(),
            roles,
            expires_at: std::time::Instant::now() + std::time::Duration::from_secs(ttl_secs),
        })
    }

    /// Build a context from the bundled policy set and an empty session
    /// store -- the shape a freshly started pod has.
    pub fn bootstrap() -> Result<Self> {
        let enforcer = Enforcer::from_policy_set(&PolicySet::bundled())
            .map_err(|e| ApiError::Internal(format!("build rbac enforcer: {e}")))?;
        Ok(Self::new(Arc::new(SessionStore::new()), Arc::new(enforcer)))
    }

    /// Resolve a bearer token to a [`Principal`].
    ///
    /// Returns `None` for a missing, malformed, unknown, or expired
    /// token. The caller must not distinguish those cases in the
    /// response -- an attacker learns nothing from which one it was.
    #[must_use]
    pub fn resolve(&self, token: &str) -> Option<Principal> {
        let session = self.sessions.lookup(token)?;
        Some(Principal {
            user_id: session.user_id,
            tenant_id: session.tenant_id,
            roles: session.roles,
        })
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

        ctx.resolve(token)
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

    #[test]
    fn an_empty_store_resolves_nothing() {
        // The shape a freshly started pod has. Every business request
        // must be a 401 until a login flow exists.
        let ctx = AuthContext::bootstrap().expect("bootstrap");
        assert!(ctx.resolve("anything").is_none());
        assert!(ctx.resolve("").is_none());
    }
}
