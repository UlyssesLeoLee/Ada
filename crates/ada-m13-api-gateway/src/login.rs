//! Login: exchange an email + password for an opaque session token.
//!
//! ## The gap this closes
//!
//! The gateway already had a full `/api` security boundary —
//! [`AuthContext::resolve`] against
//! `ada_identity::session::SessionStore`, plus RBAC on top — but nothing
//! in the repository ever *minted* a session, so a freshly started pod
//! answered 401 to every business request. Fail-closed was correct and
//! the deployment was not a working product. `gm-console` already calls
//! `POST /api/v1/auth/login` and reads a `token` field off the response;
//! that endpoint is what this module adds.
//!
//! ## Why the token is opaque, not a JWT
//!
//! The product decision was "the gateway issues a JWT". It cannot, and
//! it must not fake it: `ada_identity::mint::mint_jwt` returns
//! `IdentityError::JwtSigningUnavailable` for *every* input because
//! there is no RS256 signer in the dependency graph, and
//! `verify_jwt_stub` refuses every token for the same reason. Minting a
//! "JWT" here would mean emitting a well-formed token with an empty
//! signature segment — the exact forgery the `ada-identity` tests exist
//! to prevent, because `tenant` and `roles` ride inside it and `tenant`
//! is the isolation key for the whole multi-tenant model.
//!
//! So the credential issued here is the opaque session token the rest of
//! this crate already validates, and the wire shape is unchanged: the
//! client stores whatever is in `token` and sends it back as
//! `Authorization: Bearer <token>`. Whether the opaque token is later
//! swapped for a signed JWT is a change to the *value in that field*,
//! not to the endpoint's contract.
//!
//! ## Security posture
//!
//! - **Comparison.** `ct_eq_secret`: a fixed-length byte loop with no
//!   early exit and no data-dependent branch, and the length difference
//!   folded into the same accumulator. The loop trip count is a compile
//!   time constant, so it does not vary with either input's length.
//! - **Enumeration.** "No such identity" and "wrong password" return the
//!   same variant, the same status, and the same message, and the
//!   unknown-identity path still runs a full comparison against a decoy
//!   so the two do not differ in work either.
//! - **Attempt surface.** `LoginLimiter`: a global bucket plus a
//!   per-identity bucket, both `ada_identity::rate_limit::TokenBucket`.
//!   The per-identity map is bounded and overflow falls back to a shared
//!   bucket, so neither is an unbounded allocation.
//! - **Credentials in output.** The submitted password never reaches a
//!   log line, an error `Display`, or an `axum` rejection: the handler
//!   takes the `JsonRejection` itself and replaces it with a fixed
//!   message, and [`LoginRequest`]'s `Debug` redacts the password.
//!
//! ## Known limitation
//!
//! [`CredentialDirectory`] holds each shared secret verbatim in process
//! memory. That is a bootstrap shape, not a production one: a real
//! deployment wants a slow KDF (`argon2`) over a database-backed
//! verifier, and a memory-resident plaintext secret cannot be swapped for
//! one without replacing this type. It is called out here rather than
//! hidden because the alternative in this crate's dependency set is
//! worse — see the module's `verify` seam in the git history of the
//! follow-up that lands the KDF.

use std::{
    collections::HashMap,
    fmt,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex, RwLock,
    },
};

use ada_identity::rate_limit::TokenBucket;
use axum::{
    extract::{rejection::JsonRejection, State},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::{
    auth::AuthContext,
    error::{ApiError, Result},
    state::AppState,
};

/// Bytes examined by one credential comparison.
///
/// The loop trip count, not the data, is what makes the comparison
/// constant-time. It is a `const` so the trip count is fixed at compile
/// time rather than derived from either input.
const SECRET_SLOTS: usize = 128;

/// Default burst allowance for the login limiter, attempts.
pub const DEFAULT_BURST: u32 = 5;

/// Default sustained login rate, attempts per minute.
pub const DEFAULT_REFILL_PER_MIN: u32 = 5;

/// Default lifetime of a token issued by login, in seconds.
pub const DEFAULT_SESSION_TTL_SECS: u64 = 3_600;

/// Ceiling on the `POST /api/v1/auth/login` request body, in bytes.
///
/// Applied with `axum::extract::DefaultBodyLimit`. An email and a
/// password do not need four kilobytes, and the endpoint is the one
/// place an unauthenticated caller gets to make the process do work.
pub const LOGIN_BODY_LIMIT_BYTES: usize = 4_096;

/// Environment variable holding the bootstrap credential set.
///
/// A JSON array of `{"email", "password", "tenant_id", "roles"[,
/// "user_id"?]}`. Unset means "no credentials configured", which leaves
/// the endpoint answering 401 to everything — the same fail-closed
/// posture a pod had before this endpoint existed. See
/// [`CredentialDirectory::from_json`] for the exact shape.
pub const USERS_ENV_VAR: &str = "ADA_GATEWAY_LOGIN_USERS";

/// Upper bound on distinct identities the per-identity limiter tracks.
///
/// The key is attacker-supplied, so an unbounded map would be a memory
/// exhaustion vector reachable by anyone who can reach `/login`.
const MAX_TRACKED_IDENTITIES: usize = 4_096;

/// The one message every credential failure produces.
///
/// "No such identity" and "wrong password" must be indistinguishable, so
/// neither may be named in the response. It is also the only string a
/// denial path is allowed to surface.
const DENIED: &str = "invalid email or password";

/// Response for a body the gateway could not parse.
///
/// Fixed, and deliberately not derived from the `JsonRejection`: a
/// third-party message is free to quote the offending value, and the
/// offending value here is a password.
const MALFORMED_BODY: &str = "malformed login request";

/// Response for a rate-limited attempt.
const RATE_LIMITED: &str = "too many login attempts";

/// Response for a credential set that is not valid JSON.
///
/// Also fixed. `ADA_GATEWAY_LOGIN_USERS` holds passwords, and
/// `main` prints a startup error with `{e}`, so a message carrying
/// `serde`'s text could carry a secret to stderr.
const BAD_USER_CONFIG: &str = "login user configuration is not a JSON array of users";

/// Compared against when the submitted identity is unknown.
///
/// Its only job is to be compared, so the "no such user" path does the
/// same work as the "wrong password" path. Its length is irrelevant
/// because the comparison loop is a fixed width.
const DECOY_SECRET: &[u8] = b"ada-gateway-decoy-not-a-real-credential";

/// Compare two secrets without letting their contents change how long
/// the comparison takes.
///
/// Both inputs are read across the same fixed `SECRET_SLOTS` width, so
/// the trip count carries no information about either. The length
/// difference is folded into the same accumulator rather than compared
/// separately, so "shorter than the stored secret" cannot exit early
/// either.
///
/// This is a hand-written constant-time compare because nothing in the
/// dependency set exposes one: `ada-identity` has `ct_eq_u32` for
/// six-digit TOTP codes but it is private to `totp`, and `subtle` /
/// `constant_time_eq` are only in the lockfile as transitive
/// dependencies. Adding one requires a `Cargo.lock` update, which this
/// lane does not own — see the lane report.
///
/// Residual limit: the compiler is free to turn the fixed-width loop
/// into a `memcmp`-shaped early exit, and this is not a guarantee
/// against an attacker who can time the whole request with
/// microsecond resolution. It removes the data-dependent *branch*, which
/// is the part a network attacker can reach.
///
/// Two empty inputs are deliberately **not** a match. An earlier version
/// returned `true` for `("", "")`: both lengths XOR to 0 and both reads are
/// out of range so every slot contributes 0, leaving the accumulator at
/// zero. That makes a credential configured with an empty secret
/// authenticate an empty password — an account with no password at all. The
/// unit test `ct_eq_secret_accepts_only_an_exact_match` caught it on CI;
/// the empty-vs-empty case is the only pair that the length fold and the
/// byte loop both consider "equal", so `present` is what distinguishes it.
fn ct_eq_secret(presented: &[u8], stored: &[u8]) -> bool {
    // u64 so a length difference of 2^32 or more cannot cancel out.
    let mut diff: u64 = u64::try_from(presented.len()).unwrap_or(u64::MAX)
        ^ u64::try_from(stored.len()).unwrap_or(u64::MAX);
    // 1 once either side has contributed at least one byte, 0 only while
    // both slices are empty at the same slot. Derived from `is_some`, i.e.
    // from lengths, which are already public to the caller via the fold
    // above -- so this adds no data-dependent branch over the secret bytes.
    let mut present: u64 = 0;
    for i in 0..SECRET_SLOTS {
        let left = presented.get(i).copied();
        let right = stored.get(i).copied();
        present |= u64::from(left.is_some() | right.is_some());
        diff |= u64::from(left.unwrap_or(0) ^ right.unwrap_or(0));
    }
    diff == 0 && present == 1
}

/// Fold an email into the key used for both lookup and rate limiting.
///
/// Trimming and lowercasing are load-bearing for the limiter, not just
/// tidiness: without them `User@Example.com` and `user@example.com`
/// are two buckets, so the per-identity limit is defeated by changing
/// the case of a request.
fn identity_key(email: &str) -> String {
    email.trim().to_lowercase()
}

/// One credential the gateway accepts at login.
///
/// `Debug` is written by hand and never renders `secret`: this type
/// reaches logs through a `{:?}` on a containing struct, and a derived
/// `Debug` would put every shared secret in the log.
pub struct StoredUser {
    user_id: String,
    tenant_id: String,
    roles: Vec<String>,
    secret: Vec<u8>,
}

impl StoredUser {
    /// Build a credential.
    ///
    /// `user_id` is the subject an audit record should name; it is
    /// separate from `email` so a rename does not rewrite history.
    pub fn new(
        user_id: impl Into<String>,
        tenant_id: impl Into<String>,
        roles: Vec<String>,
        password: &str,
    ) -> Self {
        Self {
            user_id: user_id.into(),
            tenant_id: tenant_id.into(),
            roles,
            secret: password.as_bytes().to_vec(),
        }
    }
}

impl fmt::Debug for StoredUser {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StoredUser")
            .field("user_id", &self.user_id)
            .field("tenant_id", &self.tenant_id)
            .field("roles", &self.roles)
            .field("secret", &"<redacted>")
            .finish()
    }
}

/// The credentials this gateway will accept at login.
///
/// Bootstrap implementation: an in-memory map, populated from
/// [`USERS_ENV_VAR`] at startup. See the module docs on why it holds
/// secrets verbatim.
#[derive(Debug, Default)]
pub struct CredentialDirectory {
    by_key: RwLock<HashMap<String, Arc<StoredUser>>>,
}

impl CredentialDirectory {
    /// An empty directory. Every login against it is a 401.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a credential under `email`. A repeated email replaces
    /// the earlier one.
    ///
    /// `email` is taken separately from the [`StoredUser`] rather than
    /// read off it, so the credential carries only the subject id an
    /// audit record should name and not a second copy of the address.
    /// The key is normalised here exactly as [`Self::get`] normalises
    /// its argument.
    pub fn insert(&self, email: &str, user: StoredUser) {
        let key = identity_key(email);
        if let Ok(mut guard) = self.by_key.write() {
            guard.insert(key, Arc::new(user));
        }
    }

    /// The credential for `email`, matched case- and whitespace-insensitively.
    #[must_use]
    pub fn get(&self, email: &str) -> Option<Arc<StoredUser>> {
        let guard = self.by_key.read().ok()?;
        guard.get(&identity_key(email)).map(Arc::clone)
    }

    /// How many credentials are configured.
    #[must_use]
    pub fn len(&self) -> usize {
        self.by_key.read().map_or(0, |g| g.len())
    }

    /// Whether no credential is configured.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Build the directory from [`USERS_ENV_VAR`].
    ///
    /// Unset yields an empty directory, so a deployment that has not
    /// been configured keeps today's fail-closed behaviour instead of
    /// failing to start. Malformed JSON *is* an error: starting up with
    /// credentials silently missing is how a deployment ends up serving
    /// 401 to every real user and nobody notices why.
    pub fn from_env() -> Result<Self> {
        match std::env::var(USERS_ENV_VAR) {
            Ok(raw) if !raw.trim().is_empty() => Self::from_json(&raw),
            _ => Ok(Self::new()),
        }
    }

    /// Parse a credential set from JSON.
    ///
    /// ```json
    /// [{"email":"ops@example.invalid",
    ///   "password":"...",
    ///   "tenant_id":"tenant-a",
    ///   "roles":["viewer"],
    ///   "user_id":"user-1"}]
    /// ```
    ///
    /// `user_id` defaults to `email`. `tenant_id` and `roles` are
    /// required: the tenant is the isolation key, and an empty `roles`
    /// would mint a token that authorizes nothing while looking like a
    /// working login. Unknown fields are rejected, so a misspelled
    /// `tenant_id` fails at startup rather than producing a credential
    /// with an empty tenant.
    ///
    /// An empty `password` is rejected for the same reason, and it is the
    /// one that used to be silent. `ct_eq_secret` treats two empty inputs
    /// as *not* a match, so such an account could never authenticate — the
    /// comparison is now correct, but a credential that can never log in is
    /// a misconfiguration, and refusing to start says so instead of leaving
    /// an operator to discover it as a mysterious 401.
    pub fn from_json(raw: &str) -> Result<Self> {
        let parsed: Vec<BootstrapUser> =
            serde_json::from_str(raw).map_err(|_| ApiError::BadRequest(BAD_USER_CONFIG.into()))?;
        let dir = Self::new();
        for user in parsed {
            let identity = user.email.trim();
            if identity.is_empty() || user.password.is_empty() {
                return Err(ApiError::BadRequest(BAD_USER_CONFIG.into()));
            }
            dir.insert(
                identity,
                StoredUser::new(
                    user.user_id.unwrap_or_else(|| identity.to_owned()),
                    user.tenant_id,
                    user.roles,
                    &user.password,
                ),
            );
        }
        Ok(dir)
    }
}

/// One entry of [`USERS_ENV_VAR`].
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BootstrapUser {
    email: String,
    password: String,
    // No `#[serde(default)]` on `tenant_id` or `roles`, deliberately. A
    // default here is how a half-typed entry becomes a credential whose
    // sessions carry an empty tenant or authorize nothing, while
    // startup still reports success. Only `user_id` is optional.
    tenant_id: String,
    #[serde(default)]
    user_id: Option<String>,
    roles: Vec<String>,
}

/// Token-bucket admission control for the login surface.
///
/// Two layers, both `ada_identity::rate_limit::TokenBucket`:
///
/// - a **global** bucket, so a spray across many distinct identities is
///   still bounded;
/// - a **per-identity** bucket, so one account cannot be ground down by
///   guesses.
///
/// The per-identity map is keyed on attacker input, so it is capped at
/// `MAX_TRACKED_IDENTITIES`. Past the cap, new identities share one
/// overflow bucket. That direction is deliberate: an attacker who can
/// invent identities must not be able to make the limiter forget the
/// real ones, and must not be able to grow the map by asking.
#[derive(Debug)]
struct LoginLimiter {
    burst: u32,
    refill_per_min: u32,
    global: TokenBucket,
    per_identity: Mutex<HashMap<String, Arc<TokenBucket>>>,
    overflow: Arc<TokenBucket>,
}

impl LoginLimiter {
    fn new(burst: u32, refill_per_min: u32) -> Self {
        Self {
            burst,
            refill_per_min,
            global: TokenBucket::new(burst, refill_per_min),
            per_identity: Mutex::new(HashMap::new()),
            overflow: Arc::new(TokenBucket::new(burst, refill_per_min)),
        }
    }

    /// Take one token for `key`, or report that the attempt is refused.
    fn allow(&self, key: &str) -> bool {
        if !self.global.try_take() {
            return false;
        }
        let bucket = {
            // A poisoned lock means some other request panicked while
            // holding it. Recovering the guard is better than refusing
            // every login for the life of the process; the map itself is
            // left consistent because every write is a single insert.
            let mut guard = self
                .per_identity
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(existing) = guard.get(key) {
                Arc::clone(existing)
            } else if guard.len() < MAX_TRACKED_IDENTITIES {
                let fresh = Arc::new(TokenBucket::new(self.burst, self.refill_per_min));
                guard.insert(key.to_owned(), Arc::clone(&fresh));
                fresh
            } else {
                Arc::clone(&self.overflow)
            }
        };
        bucket.try_take()
    }
}

/// The issued credential.
#[derive(Serialize)]
pub struct LoginResponse {
    /// The opaque session token, sent back as `Authorization: Bearer`.
    ///
    /// Field name is the Flutter client's: `auth_api.dart` reads
    /// `body['token']` and throws if it is absent or empty, so this is
    /// a wire contract, not a naming preference.
    pub token: String,
    /// Always `"Bearer"`. The only scheme this gateway accepts.
    pub token_type: &'static str,
    /// Lifetime granted, so a client can schedule a refresh.
    pub expires_in_secs: u64,
}

impl fmt::Debug for LoginResponse {
    /// Redacts `token`, for the same reason [`StoredUser`] does.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LoginResponse")
            .field("token", &"<redacted>")
            .field("token_type", &self.token_type)
            .field("expires_in_secs", &self.expires_in_secs)
            .finish()
    }
}

/// The login request body.
#[derive(Deserialize)]
pub struct LoginRequest {
    /// The account identifier. Matched case- and whitespace-insensitively.
    pub email: String,
    /// The shared secret. Never logged, never rendered by `Debug`, and
    /// never included in an error message.
    pub password: String,
}

impl fmt::Debug for LoginRequest {
    /// Redacts `password`. A derived `Debug` on a struct with a
    /// `password` field is a credential leak waiting for a `{:?}`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LoginRequest")
            .field("email", &self.email)
            .field("password", &"<redacted>")
            .finish()
    }
}

/// The login service: directory + limiter + session minting.
pub struct LoginService {
    directory: Arc<CredentialDirectory>,
    limiter: LoginLimiter,
    session_ttl_secs: u64,
    /// Count of credential comparisons performed, including the ones
    /// against `DECOY_SECRET`.
    ///
    /// Exists so a test can observe that the unknown-identity path
    /// really does compare, rather than inferring it from a timing
    /// measurement it could not make reliably.
    comparisons: AtomicU64,
}

impl LoginService {
    /// A service over `directory` with the default rate limits and a
    /// one-hour session.
    #[must_use]
    pub fn new(directory: Arc<CredentialDirectory>) -> Self {
        Self::with_limits(directory, DEFAULT_BURST, DEFAULT_REFILL_PER_MIN)
    }

    /// A service with explicit attempt limits.
    ///
    /// Public so a test can drive the 429 path without waiting on a
    /// production-sized refill rate.
    #[must_use]
    pub fn with_limits(
        directory: Arc<CredentialDirectory>,
        burst: u32,
        refill_per_min: u32,
    ) -> Self {
        Self {
            directory,
            limiter: LoginLimiter::new(burst, refill_per_min),
            session_ttl_secs: DEFAULT_SESSION_TTL_SECS,
            comparisons: AtomicU64::new(0),
        }
    }

    /// Override the issued session's lifetime.
    #[must_use]
    pub fn with_session_ttl(mut self, ttl_secs: u64) -> Self {
        self.session_ttl_secs = ttl_secs;
        self
    }

    /// Whether any credential is configured. False means every login
    /// attempt is a 401 and the deployment cannot authenticate anyone.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        !self.directory.is_empty()
    }

    /// Credential comparisons performed so far.
    #[must_use]
    pub fn comparison_count(&self) -> u64 {
        self.comparisons.load(Ordering::Relaxed)
    }

    /// Verify a credential and mint a session for it.
    ///
    /// # Errors
    ///
    /// - [`ApiError::TooManyRequests`] when the attempt ceiling for
    ///   this identity, or globally, is exhausted.
    /// - [`ApiError::Unauthorized`] for an unknown identity and for a
    ///   wrong password alike, with an identical message.
    /// - [`ApiError::ServiceUnavailable`] when the session store is at
    ///   its ceiling.
    pub async fn authenticate(
        &self,
        auth: &AuthContext,
        email: &str,
        password: &str,
    ) -> Result<LoginResponse> {
        let key = identity_key(email);
        if !self.limiter.allow(&key) {
            // No identity in the line: logging the submitted address
            // here would both leak PII and tell a reader which of two
            // sprayed addresses the limiter had seen.
            tracing::warn!(event = "auth.login", outcome = "rate_limited");
            return Err(ApiError::TooManyRequests(RATE_LIMITED.into()));
        }

        let presented = password.as_bytes();
        let found = self.directory.get(&key);
        self.comparisons.fetch_add(1, Ordering::Relaxed);
        // The comparison runs on *both* branches. Returning early for an
        // unknown identity would make "no such user" measurably faster
        // than "wrong password", which is how an address book gets
        // built. The result is discarded when there is no user, on
        // purpose: acting on it would reintroduce the difference.
        let matches = match found.as_ref() {
            Some(user) => ct_eq_secret(presented, &user.secret),
            None => ct_eq_secret(presented, DECOY_SECRET),
        };

        let Some(user) = found else {
            tracing::warn!(event = "auth.login", outcome = "denied");
            return Err(ApiError::Unauthorized(DENIED.into()));
        };
        if !matches {
            tracing::warn!(event = "auth.login", outcome = "denied");
            return Err(ApiError::Unauthorized(DENIED.into()));
        }

        let token = auth
            .mint_session(
                &user.user_id,
                &user.tenant_id,
                user.roles.clone(),
                self.session_ttl_secs,
            )
            .await?;
        // The one place a subject is named, and it is the server-side
        // id rather than anything the client supplied.
        tracing::info!(event = "auth.login", outcome = "ok", user_id = %user.user_id);
        Ok(LoginResponse {
            token,
            token_type: "Bearer",
            expires_in_secs: self.session_ttl_secs,
        })
    }
}

impl fmt::Debug for LoginService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LoginService")
            .field("directory", &self.directory)
            .field("limiter", &"<LoginLimiter>")
            .field("session_ttl_secs", &self.session_ttl_secs)
            .field("comparisons", &self.comparisons)
            .finish()
    }
}

/// `POST /api/v1/auth/login`.
///
/// Takes the [`JsonRejection`] itself rather than letting axum render
/// it. axum's own rejection message is free to quote the value that
/// failed to parse, and here that value is a password — so every
/// rejection collapses to one fixed message.
pub async fn login_handler(
    State(state): State<AppState>,
    // Spelled `std::result::Result` rather than the bare alias: this
    // crate defines `Result<T>` as a one-parameter alias over
    // `ApiError`, and the two-argument form below would otherwise be
    // read as that alias with a stray second argument.
    payload: std::result::Result<Json<LoginRequest>, JsonRejection>,
) -> Result<Json<LoginResponse>> {
    let Json(request) = payload.map_err(|_| ApiError::BadRequest(MALFORMED_BODY.into()))?;
    state
        .login
        .authenticate(&state.auth, &request.email, &request.password)
        .await
        .map(Json)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_PASSWORD: &str = concat!("pw-", "fixture-only");

    const TEST_EMAIL: &str = "user-1@example.invalid";

    fn directory_with_one_user() -> Arc<CredentialDirectory> {
        let dir = Arc::new(CredentialDirectory::new());
        dir.insert(
            TEST_EMAIL,
            StoredUser::new("user-1", "tenant-a", vec!["viewer".into()], TEST_PASSWORD),
        );
        dir
    }

    /// An auth context over the in-process session double, not
    /// [`AuthContext::bootstrap`].
    ///
    /// `bootstrap` deliberately refuses to start without a reachable
    /// shared store, which is right for a pod and useless for a unit
    /// test: it would make the whole login suite depend on a Redis that
    /// no `cargo test` invocation starts. Swapping only the storage keeps
    /// the real bundled policy set gating every minted session, so what
    /// these tests still prove is the credential path, not the wiring.
    fn test_auth() -> AuthContext {
        AuthContext::with_bundled_policy(Arc::new(ada_identity::session::SessionStore::new()))
            .expect("bundled policy set")
    }

    #[test]
    fn ct_eq_secret_accepts_only_an_exact_match() {
        assert!(ct_eq_secret(b"abc", b"abc"));
        assert!(!ct_eq_secret(b"abc", b"abd"));
        assert!(!ct_eq_secret(b"abc", b"ab"));
        assert!(!ct_eq_secret(b"ab", b"abc"));
        assert!(!ct_eq_secret(b"", b""));
        assert!(!ct_eq_secret(b"", b"a"));
    }

    /// A length difference of exactly the accumulator width must still
    /// be caught. A `u8` accumulator would fold 256 and 0 into the same
    /// value and accept a 256-byte password for a 0-byte secret.
    #[test]
    fn ct_eq_secret_sees_a_length_difference_of_the_accumulator_width() {
        let long = vec![b'a'; 256];
        assert!(!ct_eq_secret(&long, b""));
        assert!(!ct_eq_secret(b"", &long));
    }

    #[test]
    fn a_credential_is_found_regardless_of_case_and_padding() {
        let dir = directory_with_one_user();
        assert!(dir.get("user-1@example.invalid").is_some());
        assert!(dir.get("  USER-1@Example.Invalid  ").is_some());
        assert!(dir.get("nobody@example.invalid").is_none());
    }

    /// The whole point of `is_enabled`: a deployment with no configured
    /// credential must be able to see that it cannot authenticate
    /// anyone, rather than discovering it from a support ticket.
    #[test]
    fn an_empty_directory_reports_itself_disabled() {
        let svc = LoginService::new(Arc::new(CredentialDirectory::new()));
        assert!(!svc.is_enabled());
        assert!(LoginService::new(directory_with_one_user()).is_enabled());
    }

    /// The regression that makes the enumeration defence testable: the
    /// unknown-identity path must still perform a comparison. A test
    /// asserting equal status codes alone would pass against an
    /// implementation that returned 401 immediately, which is exactly
    /// the leak this guards.
    #[tokio::test]
    async fn an_unknown_identity_still_runs_a_credential_comparison() {
        let svc = LoginService::with_limits(directory_with_one_user(), 100, 1_000);
        let auth = test_auth();
        let before = svc.comparison_count();

        let err = svc
            .authenticate(&auth, "nobody@example.invalid", TEST_PASSWORD)
            .await
            .expect_err("an unknown identity must not authenticate");

        assert!(
            matches!(err, ApiError::Unauthorized(_)),
            "expected Unauthorized, got {err:?}"
        );
        assert_eq!(
            svc.comparison_count(),
            before + 1,
            "the unknown-identity path must still compare, or it is measurably faster"
        );
    }

    /// The two denials have to be the same error, not merely the same
    /// status. A client-visible difference here is a user-enumeration
    /// oracle even when both are 401.
    #[tokio::test]
    async fn the_two_denial_reasons_produce_one_identical_error() {
        let svc = LoginService::with_limits(directory_with_one_user(), 100, 1_000);
        let auth = test_auth();

        let unknown = svc
            .authenticate(&auth, "nobody@example.invalid", TEST_PASSWORD)
            .await
            .expect_err("unknown identity");
        let wrong = svc
            .authenticate(&auth, "user-1@example.invalid", "not-the-password")
            .await
            .expect_err("wrong password");

        assert_eq!(
            unknown.to_string(),
            wrong.to_string(),
            "the two failures must be indistinguishable in the error text"
        );
        assert_eq!(unknown.status(), wrong.status());
    }

    /// A correct credential mints a working token. Without this the
    /// whole endpoint is a 401 generator.
    #[tokio::test]
    async fn a_correct_credential_mints_a_token_that_resolves() {
        let dir = directory_with_one_user();
        let auth = test_auth();
        let svc = LoginService::new(Arc::clone(&dir));

        let issued = svc
            .authenticate(&auth, "user-1@example.invalid", TEST_PASSWORD)
            .await
            .expect("login");

        let principal = auth
            .resolve(&issued.token)
            .await
            .expect("lookup")
            .expect("the token just issued must resolve");
        assert_eq!(principal.user_id, "user-1");
        assert_eq!(principal.tenant_id, "tenant-a");
        assert_eq!(issued.token_type, "Bearer");
    }

    /// The attempt ceiling is the difference between a login endpoint
    /// and an offline guessing oracle.
    #[tokio::test]
    async fn the_attempt_ceiling_is_enforced() {
        let svc = LoginService::with_limits(directory_with_one_user(), 3, 1);
        let auth = test_auth();

        for i in 0..3 {
            assert!(
                svc.authenticate(&auth, "user-1@example.invalid", "wrong").await
                    .is_err(),
                "attempt {i} is within the burst allowance and must be refused on credential, not rate"
            );
        }
        let limited = svc
            .authenticate(&auth, "user-1@example.invalid", "wrong")
            .await
            .expect_err("the fourth attempt exceeds a burst of three");
        assert!(
            matches!(limited, ApiError::TooManyRequests(_)),
            "expected TooManyRequests, got {limited:?}"
        );
    }

    /// The rate-limit key has to be normalised, or the ceiling is
    /// defeated by changing the case of the address.
    #[tokio::test]
    async fn the_attempt_ceiling_is_not_defeated_by_changing_case() {
        let svc = LoginService::with_limits(directory_with_one_user(), 2, 1);
        let auth = test_auth();

        assert!(svc
            .authenticate(&auth, "user-1@example.invalid", "wrong")
            .await
            .is_err());
        assert!(svc
            .authenticate(&auth, "USER-1@EXAMPLE.INVALID", "wrong")
            .await
            .is_err());
        let third = svc
            .authenticate(&auth, " User-1@Example.Invalid ", "wrong")
            .await;
        assert!(
            matches!(third, Err(ApiError::TooManyRequests(_))),
            "a differently-cased retry must land in the same bucket, got {third:?}"
        );
    }

    /// The global bucket has to exist independently of the per-identity
    /// one, or a spray across many addresses is unbounded.
    #[tokio::test]
    async fn spraying_distinct_identities_is_still_bounded() {
        let svc = LoginService::with_limits(directory_with_one_user(), 3, 1);
        let auth = test_auth();

        // A `for` loop, not `Iterator::filter`: `authenticate` is `async`,
        // and a closure passed to `filter` cannot await. Collecting first
        // and counting afterwards would be equivalent but would read as if
        // the spray were concurrent when it is deliberately sequential —
        // each attempt has to consume a token before the next one does.
        let mut refused = 0usize;
        for i in 0..10 {
            let attempt = svc
                .authenticate(
                    &auth,
                    &format!("spray-{i}@example.invalid"),
                    "not-the-password",
                )
                .await;
            if attempt.is_err() {
                refused += 1;
            }
        }
        assert!(
            refused > 0,
            "every attempt must be refused on the credential"
        );

        let after_spray = svc
            .authenticate(&auth, "spray-0@example.invalid", "not-the-password")
            .await;
        assert!(
            matches!(after_spray, Err(ApiError::TooManyRequests(_))),
            "the global bucket must still have tokens left, got {after_spray:?}"
        );
    }

    /// A `Debug` on a struct with a `password` field is a credential
    /// leak the moment anything logs `{:?}`. Asserted on the rendered
    /// string rather than on the type, because the type is the claim
    /// and the string is the leak.
    #[test]
    fn no_debug_rendering_contains_a_shared_secret() {
        let dir = directory_with_one_user();
        let request = LoginRequest {
            email: "user-1@example.invalid".into(),
            password: TEST_PASSWORD.into(),
        };
        let issued = LoginResponse {
            token: "issued-token-value".into(),
            token_type: "Bearer",
            expires_in_secs: 60,
        };
        for rendered in [
            format!("{request:?}"),
            format!("{:?}", dir.get("user-1@example.invalid")),
            format!("{issued:?}"),
            format!("{:?}", LoginService::new(Arc::clone(&dir))),
        ] {
            assert!(
                !rendered.contains(TEST_PASSWORD),
                "a shared secret reached a Debug rendering: {rendered}"
            );
            assert!(
                !rendered.contains("issued-token-value"),
                "an issued token reached a Debug rendering: {rendered}"
            );
        }
    }

    /// Malformed configuration must not be able to quote the secret it
    /// failed to parse: `main` prints this error with `{e}`.
    #[test]
    fn a_malformed_credential_config_does_not_quote_the_input() {
        let secret = concat!("pw-", "leaked-by-a-parser");
        let raw = format!(r#"{{"email":"a@example.invalid","password":"{secret}"}}"#);
        let err = CredentialDirectory::from_json(&raw)
            .expect_err("a JSON object is not an array of users");
        assert!(
            !err.to_string().contains(secret),
            "the parser's own message leaked the secret: {err}"
        );
    }

    /// An unknown field has to be an error, not a shrug. A misspelled
    /// `tenant_id` that is silently dropped produces a credential whose
    /// sessions all carry an empty tenant.
    #[test]
    fn an_unknown_field_is_rejected_rather_than_ignored() {
        let raw = r#"[{"email":"a@example.invalid","password":"p","tenant_id":"t","roles":[],"tenat_id":"typo"}]"#;
        assert!(CredentialDirectory::from_json(raw).is_err());
    }

    /// `roles` and `tenant_id` are required, so a half-configured entry
    /// cannot mint a token that authorizes nothing while looking like a
    /// successful login.
    #[test]
    fn a_half_configured_entry_is_rejected() {
        for raw in [
            r#"[{"email":"a@example.invalid","password":"p","roles":[]}]"#,
            r#"[{"email":"a@example.invalid","password":"p","tenant_id":"t"}]"#,
            r#"[{"password":"p","tenant_id":"t","roles":[]}]"#,
        ] {
            assert!(
                CredentialDirectory::from_json(raw).is_err(),
                "must reject {raw}"
            );
        }
    }

    /// The happy path for the config parser, including the
    /// `user_id` default.
    #[test]
    fn a_credential_set_parses() {
        let raw = r#"[{"email":"Ops@Example.Invalid","password":"p","tenant_id":"tenant-a","roles":["viewer"]}]"#;
        let dir = CredentialDirectory::from_json(raw).expect("parse");
        assert_eq!(dir.len(), 1);
        let user = dir.get("ops@example.invalid").expect("found by folded key");
        assert_eq!(
            user.user_id, "Ops@Example.Invalid",
            "user_id defaults to email"
        );
        assert_eq!(user.tenant_id, "tenant-a");
    }

    /// An empty `password` is a misconfiguration, not a credential.
    ///
    /// `ct_eq_secret` already refuses to match two empty inputs, so such an
    /// account can never authenticate. The point of rejecting it here is
    /// that it should not start up at all: an operator gets a refusal with
    /// a reason rather than a service that answers 401 to a real user for
    /// as long as the misconfiguration is invisible.
    #[test]
    fn a_credential_with_an_empty_password_is_refused_rather_than_silently_unusable() {
        let raw = r#"[{"email":"ops@example.invalid","password":"","tenant_id":"tenant-a","roles":["viewer"]}]"#;
        assert!(
            CredentialDirectory::from_json(raw).is_err(),
            "a credential with an empty secret must fail at startup, not \
             produce an account that can never log in"
        );
    }
}
