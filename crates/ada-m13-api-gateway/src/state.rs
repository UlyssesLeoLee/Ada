//! Application state shared by every gateway handler.
//!
//! [`AppState`] is cloned (cheaply, via `Arc`) into each axum request
//! and gives handlers access to the configured service name and the
//! [`HealthCheck`] strategy used by `/health/*` endpoints.

use std::sync::Arc;

use crate::auth::AuthContext;
use crate::health::HealthCheck;
use crate::login::{CredentialDirectory, LoginService};

/// State held by every gateway request handler.
#[derive(Clone)]
pub struct AppState {
    /// Human-readable service name reported by `/health`.
    pub name: String,
    /// Pluggable health-check strategy (used by `/health/ready`).
    pub db: Arc<dyn HealthCheck>,
    /// Session store and RBAC enforcer backing `/api/*`.
    ///
    /// Public so axum can extract it as request state. The session
    /// store is the only thing that turns a bearer token into a
    /// tenant, and the enforcer is the only thing that answers whether
    /// that tenant may act on an object. A business handler needs both,
    /// which is why this is state rather than a module-level global:
    /// tests must be able to build an isolated pair.
    pub auth: AuthContext,
    /// Backing `POST /api/v1/auth/login`.
    ///
    /// The only thing in the process that mints a session, so it is the
    /// one place the "empty store denies everything" invariant can be
    /// lifted. It defaults to an empty [`CredentialDirectory`], which
    /// keeps that invariant for every construction that does not
    /// configure credentials — see [`Self::with_login`].
    pub login: Arc<LoginService>,
}

impl core::fmt::Debug for AppState {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("AppState")
            .field("name", &self.name)
            .field("db", &"<dyn HealthCheck>")
            .field("auth", &self.auth)
            // `LoginService`'s own `Debug` is hand-written and redacts,
            // so this cannot surface a configured credential.
            .field("login", &self.login)
            .finish()
    }
}

impl AppState {
    /// Build an [`AppState`] with a name and a [`HealthCheck`].
    ///
    /// Most callers will pass [`MemoryHealthCheck`](crate::health::MemoryHealthCheck)
    /// for `db`; production builds will pass a wrapper that probes the
    /// real DB / peer pool.
    ///
    /// `auth` comes from [`AuthContext::bootstrap`], which needs the shared
    /// session store named by `ADA_SESSION_REDIS_URL` and therefore cannot
    /// be built offline. There is no way to reach `/api/*` without a session,
    /// which is deliberate.
    pub async fn new(name: impl Into<String>, db: Arc<dyn HealthCheck>) -> crate::Result<Self> {
        Ok(Self {
            name: name.into(),
            db,
            auth: AuthContext::bootstrap().await?,
            login: Arc::new(LoginService::new(Arc::new(CredentialDirectory::new()))),
        })
    }

    /// Build an [`AppState`] with an explicit [`AuthContext`].
    ///
    /// The seam tests use to supply their own session store, so they do
    /// not share principal state with each other.
    pub fn with_auth(name: impl Into<String>, db: Arc<dyn HealthCheck>, auth: AuthContext) -> Self {
        Self {
            name: name.into(),
            db,
            auth,
            login: Arc::new(LoginService::new(Arc::new(CredentialDirectory::new()))),
        }
    }

    /// Install the credential directory the login endpoint reads.
    ///
    /// Separate from [`Self::new`] rather than folded into it on
    /// purpose: `AppState::new` must not read the environment, or every
    /// test that builds state would inherit whatever credential set the
    /// developer's machine happens to export. Production wires this from
    /// [`CredentialDirectory::from_env`] in `server::serve`.
    #[must_use]
    pub fn with_login(mut self, login: Arc<LoginService>) -> Self {
        self.login = login;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::health::MemoryHealthCheck;

    /// An [`AppState`] built the way a test can build one.
    ///
    /// Not [`AppState::new`]: that one bootstraps from
    /// [`REDIS_URL_ENV`] and refuses to exist without a reachable shared
    /// store, which is the correct posture for a pod and makes every test
    /// below depend on infrastructure no `cargo test` run provides.
    /// [`Self::with_auth`] is the seam for exactly this, and it still
    /// loads the real bundled policy set — only the storage is swapped.
    fn state_with_isolated_sessions(name: &str) -> AppState {
        let auth =
            AuthContext::with_bundled_policy(Arc::new(ada_identity::session::SessionStore::new()))
                .expect("bundled policy set");
        AppState::with_auth(name, Arc::new(MemoryHealthCheck::new()), auth)
    }

    #[tokio::test]
    async fn new_takes_name_and_db() {
        let state = state_with_isolated_sessions("ada-gateway");
        assert_eq!(state.name, "ada-gateway");
    }

    #[tokio::test]
    async fn new_accepts_string_and_str() {
        let s = state_with_isolated_sessions("a");
        assert_eq!(s.name, "a");
        let s = state_with_isolated_sessions("b");
        assert_eq!(s.name, "b");
    }

    #[tokio::test]
    async fn a_bootstrapped_state_denies_every_token() {
        // The freshly-started shape. If this ever starts resolving a
        // token, something is minting sessions without a login flow.
        let state = state_with_isolated_sessions("ada-gateway");
        assert!(state
            .auth
            .resolve("any-token")
            .await
            .expect("lookup")
            .is_none());
    }

    /// The default construction must not become a back door. Before
    /// `/auth/login` existed, an empty session store was a complete
    /// answer; if a default-constructed `AppState` came with a
    /// credential directory, this crate would ship a way in that no
    /// test and no operator had asked for.
    #[tokio::test]
    async fn a_bootstrapped_state_has_no_configured_credentials() {
        let state = state_with_isolated_sessions("ada-gateway");
        assert!(!state.login.is_enabled());
    }

    /// `String` and `&str` are both accepted, because the two
    /// constructors below a test are the ones a caller actually has.
    #[tokio::test]
    async fn with_auth_accepts_both_string_and_str_names() {
        let auth =
            AuthContext::with_bundled_policy(Arc::new(ada_identity::session::SessionStore::new()))
                .expect("bundled policy set");
        let owned = AppState::with_auth(
            String::from("owned"),
            Arc::new(MemoryHealthCheck::new()),
            auth,
        );
        assert_eq!(owned.name, "owned");
    }
}
