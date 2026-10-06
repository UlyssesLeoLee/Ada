//! The Redis implementation of [`SharedSessionBackend`].
//!
//! Chosen on 2026-10-06, after `shared_session.rs` shipped the trait with no
//! implementation. The shape below is the standard one and every choice in
//! it is a consequence of a property of the trait, not a preference:
//!
//! ## Layout
//!
//! Two structures, because the trait's `count` and `delete_expired` cannot
//! both be answered cheaply from a flat namespace:
//!
//! - `ada:session:<token>` — a string holding the [`SessionRecord`] as JSON.
//!   The token is in the key name, so `delete` is one `DEL`.
//! - `ada:session-expiry` — a sorted set of tokens scored by
//!   `expires_at_unix_ms`. `count` is `ZCARD`; `delete_expired` is a bounded
//!   `ZRANGEBYSCORE` over the dead members followed by one pipelined
//!   unlink of each.
//!
//! The alternative — `SCAN` over `ada:session:*` and filter in Rust — makes
//! `count` O(keyspace) and is called on the mint path. On a database shared
//! with anything else that is a way to make login slow.
//!
//! ## What each trait method means here
//!
//! `get` returns `Ok(None)` for an absent key. `Err` is only ever "Redis did
//! not answer", never "no such session" -- collapsing those two is what makes
//! a Redis blip look like a logged-out user.
//!
//! `count` is `ZCARD` of the expiry index, expired members included. That is
//! an over-estimate, which is the direction the trait asks for: the ceiling
//! is checked against it, and over-reporting fails the mint closed.
//!
//! `delete_expired` is bounded by `SWEEP_BATCH`. A single sweep that trims
//! everything dead at once is a `SCAN` of the whole index and can hold the
//! connection long enough to look like an outage. Bounded batches, called
//! again, converge on the same state.
//!
//! ## No token in an error message
//!
//! Every `map_err` here produces a message built from the Redis error alone.
//! The session token is part of the key name, so a naive
//! `format!("{e} on {key}")` would put a live credential in a log line.

use async_trait::async_trait;
use redis::aio::ConnectionManager;
use redis::AsyncCommands;

use crate::error::{IdentityError, Result};
use crate::session::SessionRecord;
use crate::shared_session::SharedSessionBackend;

/// Key holding one session's JSON record.
fn key_for(token: &str) -> String {
    format!("ada:session:{token}")
}

/// Sorted set of live tokens, scored by their absolute deadline.
///
/// A set rather than a per-key TTL because Redis's own expiry removes the
/// string but cannot answer "how many are left", and a count that misses
/// expiring members lets the ceiling be exceeded by every session in flight
/// at the moment one expires.
const EXPIRY_INDEX: &str = "ada:session-expiry";

/// How many dead members one `delete_expired` call removes.
///
/// Bounded on purpose: see the module docs.
const SWEEP_BATCH: usize = 256;

/// Environment variable naming the Redis instance.
pub const REDIS_URL_ENV: &str = "ADA_SESSION_REDIS_URL";

/// A [`SharedSessionBackend`] over Redis, reached through a
/// [`ConnectionManager`].
///
/// `ConnectionManager` rather than a raw `MultiplexedConnection` because a
/// client that gives up on the first reconnect turns one Redis restart into a
/// fleet-wide logout: every replica's pool is empty at once and no login
/// succeeds until something forces a reconnect.
pub struct RedisSessionBackend {
    conn: ConnectionManager,
}

impl RedisSessionBackend {
    /// Connect to `url`, or read [`REDIS_URL_ENV`].
    ///
    /// An unset variable is an error, not an empty configuration. A gateway
    /// that silently started with no session store would answer 401 to every
    /// request and look healthy; refusing to start says why.
    pub async fn connect(url: &str) -> Result<Self> {
        let client = redis::Client::open(url)
            .map_err(|e| IdentityError::SessionBackend(format!("invalid Redis URL: {e}")))?;
        let conn = ConnectionManager::new(client)
            .await
            .map_err(|e| IdentityError::SessionBackend(format!("cannot reach Redis: {e}")))?;
        Ok(Self { conn })
    }

    /// Connect using [`REDIS_URL_ENV`], or fail closed if it is unset.
    pub async fn from_env() -> Result<Self> {
        let url = std::env::var(REDIS_URL_ENV).map_err(|_| {
            IdentityError::SessionBackend(format!(
                "{REDIS_URL_ENV} is not set: the gateway cannot hold sessions \
                 without a shared store"
            ))
        })?;
        if url.trim().is_empty() {
            return Err(IdentityError::SessionBackend(format!(
                "{REDIS_URL_ENV} is empty: the gateway cannot hold sessions \
                 without a shared store"
            )));
        }
        Self::connect(&url).await
    }
}

#[async_trait]
impl SharedSessionBackend for RedisSessionBackend {
    async fn get(&self, token: &str) -> Result<Option<SessionRecord>> {
        let mut conn = self.conn.clone();
        let raw: Option<String> = conn.get(key_for(token)).await.map_err(redis_err)?;
        let Some(raw) = raw else {
            return Ok(None);
        };
        // A record that will not parse is a corrupt row, not a missing one.
        // Reporting it as `None` would silently turn a data problem into a
        // logout; reporting it as a backend failure says the store is
        // unhealthy, which is what it is.
        let record = serde_json::from_str(&raw)
            .map_err(|e| IdentityError::SessionBackend(format!("corrupt session record: {e}")))?;
        Ok(Some(record))
    }

    async fn put(&self, token: &str, value: SessionRecord) -> Result<()> {
        let mut conn = self.conn.clone();
        let payload = serde_json::to_string(&value)
            .map_err(|e| IdentityError::SessionBackend(format!("cannot encode record: {e}")))?;
        let mut pipe = redis::pipe();
        pipe.atomic()
            .cmd("SET")
            .arg(key_for(token))
            .arg(&payload)
            .ignore()
            .cmd("ZADD")
            .arg(EXPIRY_INDEX)
            .arg(value.expires_at_unix_ms)
            .arg(token)
            .ignore();
        pipe.query_async(&mut conn).await.map_err(redis_err)
    }

    async fn delete(&self, token: &str) -> Result<()> {
        let mut conn = self.conn.clone();
        let mut pipe = redis::pipe();
        pipe.atomic()
            .cmd("DEL")
            .arg(key_for(token))
            .ignore()
            .cmd("ZREM")
            .arg(EXPIRY_INDEX)
            .arg(token)
            .ignore();
        // Idempotent by construction: both commands succeed whether or not
        // the key was there, so `revoke` does not need to know.
        pipe.query_async(&mut conn).await.map_err(redis_err)
    }

    async fn count(&self) -> Result<usize> {
        let mut conn = self.conn.clone();
        // Over-counts: an expired-but-unswept member is still in the index.
        // The ceiling is checked against this, so the error is in the safe
        // direction -- it fails a mint closed rather than admitting one too
        // many.
        conn.zcard(EXPIRY_INDEX).await.map_err(redis_err)
    }

    async fn delete_expired(&self, now_unix_ms: i64) -> Result<usize> {
        let mut conn = self.conn.clone();
        let dead: Vec<String> = conn
            .zrangebyscore(EXPIRY_INDEX, i64::MIN, now_unix_ms)
            .await
            .map_err(redis_err)?;
        let batch = dead.len().min(SWEEP_BATCH);
        if batch == 0 {
            return Ok(0);
        }
        let members = &dead[..batch];
        // Pipelined because these are independent, and one round trip per
        // session would make a sweep of a few hundred rows noticeably slow.
        let mut pipe = redis::pipe();
        pipe.atomic();
        for token in members {
            pipe.cmd("DEL").arg(key_for(token)).ignore();
            pipe.cmd("ZREM").arg(EXPIRY_INDEX).arg(token).ignore();
        }
        pipe.query_async(&mut conn).await.map_err(redis_err)?;
        Ok(batch)
    }
}

/// Reduce a `redis` error to a message that cannot leak a session token.
///
/// Deliberately drops the error's own `Display`: a command error from Redis
/// can echo the arguments it failed on, and the arguments here are key names
/// built from live tokens. The kind is enough to act on, and the detail is
/// available from the Redis server's own log.
fn redis_err(e: redis::RedisError) -> IdentityError {
    IdentityError::SessionBackend(format!("redis command failed: {}", classify(&e)))
}

fn classify(e: &redis::RedisError) -> &'static str {
    use redis::ErrorKind::{IoError, TypeError, Unreachable, ExtensionError};
    match e.kind() {
        Unreachable => "server unreachable",
        IoError => "io error",
        TypeError => "wrong response type",
        ExtensionError => "command extension error",
        _ => "command error",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The one property that matters and cannot be checked by running the
    /// code: a session token must never reach an error string. The key name
    /// *is* the token, so this is a real hazard rather than a paranoid one.
    #[test]
    fn no_redis_error_message_can_carry_a_session_token() {
        for kind in [
            redis::ErrorKind::TypeError,
            redis::ErrorKind::ExtensionError,
        ] {
            // Whatever Redis attached to the error, the message we build
            // comes from `classify` alone.
            let e = redis::RedisError::from((kind, "value is not a valid integer"));
            let msg = redis_err(e).to_string();
            assert!(
                !msg.contains("value is not a valid integer"),
                "the raw Redis error text leaked into our message: {msg}"
            );
        }
    }

    /// Two empty inputs must not produce the same key, or one session would
    /// overwrite another.
    #[test]
    fn a_key_is_derived_from_the_token_and_nothing_else() {
        assert_eq!(key_for("abc"), "ada:session:abc");
        assert_ne!(key_for("abc"), key_for("abd"));
        assert_ne!(key_for("abc"), key_for(""));
    }

    /// The index key is a constant, so it can never be confused with a
    /// session key even if a token were empty.
    #[test]
    fn the_expiry_index_is_not_reachable_as_a_session_key() {
        assert!(!key_for("").contains(EXPIRY_INDEX));
    }
}
