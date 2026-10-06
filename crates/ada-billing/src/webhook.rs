//! Stripe webhook handler.
//!
//! Verifies `Stripe-Signature` (HMAC-SHA256 over `t.body`), enforces
//! a 5-minute timestamp tolerance, deduplicates via
//! `(event.id, tenant_id)` idempotency table, and emits a
//! `BillingEvent` into a tokio mpsc channel for downstream
//! processing. Audit log entries are emitted on every accepted event
//! via [`ada_m11_rbac_collab::record_audit_log`].
//!
//! ## Threat model (per `auth-billing-arch.md` §6)
//!
//! * Signature mismatch → `BillingError::InvalidSignature` (the raw
//!   header / body is never logged).
//! * Replay (idempotency hit) → silently dropped, audit entry
//!   recorded.
//! * Malformed envelope → `BillingError::MalformedEnvelope`.
//!
//! No env var values appear in any log line.
//!
//! ## Idempotency retention
//!
//! The dedup table is the one piece of state on this path where
//! "forget too early" and "never forget" are both money bugs, so its
//! policy is spelled out here rather than left to a reader of
//! [`IdempotencyStore`]:
//!
//! * **Durable, not process memory.** [`IdempotencyStore::new`] keeps
//!   keys in memory only and is for tests and local wiring;
//!   [`IdempotencyStore::open`] journals every key to disk and fsyncs
//!   it, so a restart cannot re-allow a charge that was already
//!   processed.
//! * **TTL 72 h** ([`IDEMPOTENCY_TTL_SECS`]), which covers Stripe's
//!   redelivery window ([`STRIPE_RETRY_WINDOW_SECS`]). Expiring a key
//!   earlier re-admits a retry that Stripe is still entitled to send.
//! * **A capacity ceiling** ([`DEFAULT_MAX_ENTRIES`]) that is an alarm
//!   rather than a wall, plus an **aging sweep** on the write path that
//!   reclaims every key past its TTL. Refusing a write because the
//!   table is full would be fail-closed on a route that receives every
//!   Stripe event, i.e. a permanent outage; evicting a key that is
//!   still inside the retry window would be fail-open onto a double
//!   charge. The store therefore reclaims what is expired, admits the
//!   rest, and counts the admissions so the ceiling is alertable.

use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use hmac::{Hmac, Mac};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use sha2::Sha256;

use ada_core::TenantId;

use crate::config::Config;
use crate::error::{BillingError, Result};

type HmacSha256 = Hmac<Sha256>;

/// Subscription state token Stripe uses in events. Newtype over
/// `&'static str` so the matcher exhausts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventKind {
    /// `customer.subscription.created`.
    CustomerSubscriptionCreated,
    /// `customer.subscription.updated`.
    CustomerSubscriptionUpdated,
    /// `customer.subscription.deleted`.
    CustomerSubscriptionDeleted,
    /// `invoice.paid`.
    InvoicePaid,
    /// `invoice.payment_failed`.
    InvoicePaymentFailed,
}

/// Parsing is a trait impl rather than an inherent `from_str` so the
/// type composes with `str::parse` and matches how
/// [`crate::subscription::SubscriptionStatus`] is parsed in this
/// crate. The error is [`BillingError::MalformedEnvelope`] because
/// the only caller reads the token out of a Stripe event envelope.
impl core::str::FromStr for EventKind {
    type Err = BillingError;

    fn from_str(s: &str) -> Result<Self> {
        Ok(match s {
            "customer.subscription.created" => Self::CustomerSubscriptionCreated,
            "customer.subscription.updated" => Self::CustomerSubscriptionUpdated,
            "customer.subscription.deleted" => Self::CustomerSubscriptionDeleted,
            "invoice.paid" => Self::InvoicePaid,
            "invoice.payment_failed" => Self::InvoicePaymentFailed,
            _ => return Err(BillingError::MalformedEnvelope),
        })
    }
}

/// One accepted event. Emitted into the channel for downstream
/// processing (DB updates, plan tier change notifications, …).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingEvent {
    /// Stripe's `event.id`; also the idempotency key.
    pub event_id: String,
    /// The event's `type`, parsed into a known token.
    pub kind: EventKind,
    /// The tenant resolved from the event's metadata.
    pub tenant_id: TenantId,
    /// Stripe-side identifier (`sub_…` / `cus_…`).
    pub target_id: String,
}

/// Stripe's automatic webhook redelivery window, in seconds.
///
/// Stripe retries an undelivered event for up to three days after the
/// first attempt. This is the number the retention TTL has to clear. A
/// key forgotten while this window is still open is not *stale*, it is
/// *missing*: the redelivery finds no key, is answered `Accepted`, and
/// the same subscription change is applied a second time.
///
/// It is also the line the store will not evict across — a key inside
/// this window is never dropped to make room, whatever the ceiling
/// says.
pub const STRIPE_RETRY_WINDOW_SECS: i64 = 3 * 24 * 60 * 60;

/// Retention TTL for one idempotency key: 72 h.
///
/// 72 h is the product decision, not a derived number. Note that it is
/// *equal* to [`STRIPE_RETRY_WINDOW_SECS`] rather than longer than it:
/// it covers the whole automatic retry window, and the last delivery of
/// that window lands on the expiry boundary, where the key is already
/// gone. `retention_ttl_covers_the_stripe_retry_window` and
/// `a_key_outlives_the_stripe_retry_window_and_then_expires` are the
/// two tests that pin that relationship; if Stripe's window is ever
/// measured as longer than three days, the TTL has to move with it.
pub const IDEMPOTENCY_TTL_SECS: i64 = 72 * 60 * 60;

/// Default capacity ceiling on held keys.
///
/// 250 k keys against a 72 h TTL is a sustained live-key rate of about
/// one event per 78 seconds before the ceiling is reached, so it is
/// sized to be a tripwire for a traffic spike or a mis-set TTL rather
/// than a number the steady state reaches. Crossing it is observable
/// through [`IdempotencyStore::is_over_capacity`] and
/// [`IdempotencyStore::overflow_admissions`]; it is not a condition
/// that rejects writes.
pub const DEFAULT_MAX_ENTRIES: usize = 250_000;

/// Retention and capacity policy for an [`IdempotencyStore`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IdempotencyPolicy {
    /// How long a recorded key is retained, in seconds.
    pub ttl_secs: i64,
    /// The number of held keys above which the store reports itself
    /// over capacity. Expired keys are always reclaimed first, so this
    /// is reached only when every held key is still inside
    /// [`IDEMPOTENCY_TTL_SECS`].
    pub max_entries: usize,
}

impl Default for IdempotencyPolicy {
    fn default() -> Self {
        Self {
            ttl_secs: IDEMPOTENCY_TTL_SECS,
            max_entries: DEFAULT_MAX_ENTRIES,
        }
    }
}

/// The answer [`IdempotencyStore::record`] gives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordOutcome {
    /// The key was not on file. The caller must dispatch the event.
    Fresh,
    /// The key was on file and is still inside its TTL. The event is a
    /// replay and must be dropped.
    Duplicate,
}

/// One journal line: the key, and the Unix second it expires at.
#[derive(Debug, Deserialize)]
struct JournalRecord {
    e: String,
    t: String,
    x: i64,
}

/// Append-only on-disk journal of recorded keys.
///
/// One JSON object per line, fsynced on every append, so an append that
/// returned `Ok` is a key that survives a crash. A line that does not
/// parse is a torn tail from a crash *during* an append — that append
/// never returned `Ok`, so nobody was told the event was processed, and
/// skipping the line is the safe direction.
///
/// The file is a single-writer log: it is per-process/per-host. A
/// multi-instance deployment still wants this table in the shared
/// database; what this buys is that a restart on one host cannot
/// re-admit a charge.
#[derive(Debug)]
struct Journal {
    path: PathBuf,
    /// Records in the file, live or not. Compared against the live
    /// count to decide when the log has accumulated enough dead lines
    /// to be worth rewriting.
    lines: u64,
}

impl Journal {
    /// Append one key and fsync it. `Ok` means the record is durable.
    fn append(&mut self, key: &(String, String), expires_at_unix: i64) -> Result<()> {
        let mut line = serde_json::to_vec(&serde_json::json!({
            "e": key.0,
            "t": key.1,
            "x": expires_at_unix,
        }))
        .map_err(|e| persist_error("encode idempotency journal record", e))?;
        line.push(b'\n');

        // Opened per append rather than held open: the append is a
        // write plus an fsync either way, and a short-lived handle
        // means a rotation or an operator replacing the file cannot
        // leave this process writing into an unlinked inode.
        let mut f = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|e| persist_error("open idempotency journal", e))?;
        f.write_all(&line)
            .map_err(|e| persist_error("write idempotency journal", e))?;
        f.sync_data()
            .map_err(|e| persist_error("fsync idempotency journal", e))?;
        self.lines = self.lines.saturating_add(1);
        Ok(())
    }

    /// Path the compacted log is staged at before it replaces the
    /// journal.
    fn compaction_path(&self) -> PathBuf {
        self.path.with_extension("compacting")
    }
}

/// Live state behind the store's lock.
#[derive(Debug, Default)]
struct Inner {
    /// `(event.id, tenant_id)` -> the Unix second the key expires at.
    entries: HashMap<(String, String), i64>,
    /// `None` for an in-memory store.
    journal: Option<Journal>,
    policy: IdempotencyPolicy,
    /// How many keys were admitted while the store was over capacity.
    /// Never resets, so it is a counter and not a gauge.
    overflow_admissions: u64,
}

/// Idempotency store: `(event.id, tenant_id)` -> expiry, with a TTL, a
/// capacity ceiling, an aging sweep, and optional durability.
///
/// Built either in memory ([`IdempotencyStore::new`], for tests and
/// local wiring) or over an fsynced journal
/// ([`IdempotencyStore::open`], for anything that must survive a
/// restart).
#[derive(Debug, Default)]
pub struct IdempotencyStore {
    inner: RwLock<Inner>,
}

impl IdempotencyStore {
    /// Create an empty **in-memory** store with the default policy.
    ///
    /// This does not survive a restart: every key is lost when the
    /// process exits, so the first delivery of every event after a
    /// restart is answered `Accepted` and dispatched again. That is
    /// correct for a test and wrong for the money path — wire
    /// [`IdempotencyStore::open`] there.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create an empty **in-memory** store with an explicit policy.
    #[must_use]
    pub fn with_policy(policy: IdempotencyPolicy) -> Self {
        Self {
            inner: RwLock::new(Inner {
                policy,
                ..Inner::default()
            }),
        }
    }

    /// Open (or create) the journal at `path` and load the keys still
    /// inside their TTL. Keys already past their TTL are dropped and
    /// the file is compacted if it was mostly dead.
    ///
    /// The file is created if absent. An unreadable or unwritable path
    /// is an error rather than a silently empty store: a store that
    /// cannot be read would answer `Fresh` for keys that are on disk,
    /// which is the double-charge direction.
    pub fn open(path: impl AsRef<Path>, policy: IdempotencyPolicy) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let now = now_unix();

        let mut entries: HashMap<(String, String), i64> = HashMap::new();
        let mut lines = 0_u64;
        let text = match fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(e) => return Err(persist_error("read idempotency journal", e)),
        };
        for line in text.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let Ok(rec) = serde_json::from_str::<JournalRecord>(line) else {
                // Torn tail: the append that was in flight never
                // returned Ok, so no caller was told the event had been
                // processed. Skipping is fail-safe here.
                continue;
            };
            lines = lines.saturating_add(1);
            if rec.x > now {
                entries.insert((rec.e, rec.t), rec.x);
            }
        }

        let store = Self {
            inner: RwLock::new(Inner {
                entries,
                journal: Some(Journal { path, lines }),
                policy,
                overflow_admissions: 0,
            }),
        };
        {
            let mut w = store.inner.write();
            let live = w.entries.len() as u64;
            let mostly_dead = w
                .journal
                .as_ref()
                .is_some_and(|j| j.lines > live.saturating_add(8));
            if mostly_dead {
                compact_locked(&mut w, now)?;
            }
        }
        Ok(store)
    }

    /// Record `key` at the current time. `Ok(Fresh)` means the caller
    /// must dispatch; `Ok(Duplicate)` means it must not.
    ///
    /// Errors when the key could not be made durable. That is
    /// deliberately fail-closed: the key is not admitted in memory
    /// either, so nothing is dispatched and Stripe's retry is still
    /// processable. The reverse order — admit, then fail to persist —
    /// would hand out an `Accepted` for an event a restart would let
    /// through again.
    pub fn record(&self, event_id: &str, tenant_id: &str) -> Result<RecordOutcome> {
        self.record_at(event_id, tenant_id, now_unix())
    }

    /// [`IdempotencyStore::record`] with the caller's clock. The
    /// primitive the time-dependent tests drive.
    pub fn record_at(
        &self,
        event_id: &str,
        tenant_id: &str,
        now_unix: i64,
    ) -> Result<RecordOutcome> {
        let key = (event_id.to_owned(), tenant_id.to_owned());
        let mut w = self.inner.write();

        // Aging sweep, before anything else: expired keys are the only
        // ones that may be dropped without weakening dedup, so this is
        // the single place the store reclaims. Sweeping on the write
        // path is enough — writes are the only way the table grows.
        w.entries.retain(|_, expires_at| *expires_at > now_unix);

        // A key that survived the sweep is live, so answering Duplicate
        // here is safe. The check and the insert are both under this one
        // write lock, which is the property that stops two deliveries
        // of one event from both being told `Fresh`.
        if w.entries.contains_key(&key) {
            return Ok(RecordOutcome::Duplicate);
        }

        let expires_at = now_unix.saturating_add(w.policy.ttl_secs);
        if let Some(journal) = w.journal.as_mut() {
            journal.append(&key, expires_at)?;
        }
        w.entries.insert(key, expires_at);

        if w.entries.len() > w.policy.max_entries {
            // Past the ceiling with nothing expired to reclaim. The
            // alternatives are both worse: refusing is a permanent
            // outage on the one route that receives every event, and
            // evicting a live key re-admits a retry Stripe is still
            // entitled to send. So admit, and make it visible.
            w.overflow_admissions = w.overflow_admissions.saturating_add(1);
        }

        // Compaction is housekeeping, not part of admitting the key: the
        // append above is already durable, so a compaction that fails
        // must not turn a safe `Fresh` into an error. The log stays
        // correct (extra dead lines are skipped on the next load) and
        // `compact` reports the failure to whoever calls it.
        let live = w.entries.len() as u64;
        let mostly_dead = w
            .journal
            .as_ref()
            .is_some_and(|j| j.lines > live.saturating_mul(2).saturating_add(64));
        if mostly_dead {
            if let Err(e) = compact_locked(&mut w, now_unix) {
                tracing::warn!(error = %e, "idempotency journal compaction failed");
            }
        }
        Ok(RecordOutcome::Fresh)
    }

    /// Returns `true` if the `(event_id, tenant_id)` key was recorded
    /// and is still inside its TTL at the current time.
    ///
    /// **Not a safe way to deduplicate.** This takes the read lock and
    /// releases it before the caller does anything else, so
    ///
    /// ```text
    /// if !store.has_seen(id, tenant) && store.record(id, tenant) { ... }
    /// ```
    ///
    /// still lets two threads pass the check before either inserts.
    /// Branch on [`IdempotencyStore::record`]'s return value instead. This
    /// accessor exists for assertions and diagnostics, which is how the
    /// tests use it.
    #[must_use]
    pub fn has_seen(&self, event_id: &str, tenant_id: &str) -> bool {
        self.has_seen_at(event_id, tenant_id, now_unix())
    }

    /// [`IdempotencyStore::has_seen`] with the caller's clock. A key
    /// that is on file but past its expiry reads as unseen, which is the
    /// same answer [`IdempotencyStore::record_at`] would give.
    #[must_use]
    pub fn has_seen_at(&self, event_id: &str, tenant_id: &str, now_unix: i64) -> bool {
        // `RwLock<Inner>`, so the guard derefs to `Inner` and the map is a
        // *field* of that -- not the guard's own target. Both hops are
        // needed: `self.inner.read().get(..)` and `held.get(..)` are both
        // E0599, for different reasons.
        let held = self.inner.read();
        held.entries
            .get(&(event_id.to_owned(), tenant_id.to_owned()))
            .is_some_and(|expires_at| *expires_at > now_unix)
    }

    /// Number of keys currently held, expired-but-unswept ones
    /// included. Diagnostics only — see
    /// [`IdempotencyStore::has_seen`] for why this is not a dedup path.
    ///
    /// It is bounded by the arrival rate times the TTL, and the sweep
    /// keeps the dead ones from accumulating; the ceiling it is compared
    /// against is [`IdempotencyStore::capacity`].
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.read().entries.len()
    }

    /// `true` when no keys are held. Pairs with [`IdempotencyStore::len`].
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.read().entries.is_empty()
    }

    /// The configured ceiling. See [`IdempotencyPolicy::max_entries`].
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.inner.read().policy.max_entries
    }

    /// `true` when the live key count is above the ceiling. The
    /// condition to alert on: it means the arrival rate against the TTL
    /// is higher than the ceiling was sized for, not that writes are
    /// being refused (they are not).
    #[must_use]
    pub fn is_over_capacity(&self) -> bool {
        let w = self.inner.read();
        w.entries.len() > w.policy.max_entries
    }

    /// How many keys have been admitted while over capacity since the
    /// store was created. Monotonic, so a rate over an interval is the
    /// difference of two readings.
    #[must_use]
    pub fn overflow_admissions(&self) -> u64 {
        self.inner.read().overflow_admissions
    }

    /// Rewrite the journal with only the keys still inside their TTL,
    /// and drop the expired ones. The write path calls this on its own
    /// once the log is mostly dead lines; it is public so an operator
    /// can force it and see the failure.
    pub fn compact(&self) -> Result<()> {
        let now = now_unix();
        let mut w = self.inner.write();
        compact_locked(&mut w, now)
    }
}

/// Drop every expired key and rewrite the journal without them. A
/// no-op for an in-memory store.
fn compact_locked(inner: &mut Inner, now_unix: i64) -> Result<()> {
    let Some(journal) = inner.journal.as_ref() else {
        return Ok(());
    };
    inner.entries.retain(|_, expires_at| *expires_at > now_unix);

    let mut body: Vec<u8> = Vec::new();
    for ((event_id, tenant_id), expires_at) in &inner.entries {
        serde_json::to_writer(
            &mut body,
            &serde_json::json!({ "e": event_id, "t": tenant_id, "x": expires_at }),
        )
        .map_err(|e| persist_error("encode idempotency journal record", e))?;
        body.push(b'\n');
    }

    let tmp = journal.compaction_path();
    let mut staged = fs::File::create(&tmp)
        .map_err(|e| persist_error("stage compacted idempotency journal", e))?;
    staged
        .write_all(&body)
        .map_err(|e| persist_error("write compacted idempotency journal", e))?;
    // fsync the staged copy before the rename, so the rename cannot
    // publish a journal whose contents are still only in the page cache.
    staged
        .sync_all()
        .map_err(|e| persist_error("fsync compacted idempotency journal", e))?;
    drop(staged);
    fs::rename(&tmp, &journal.path)
        .map_err(|e| persist_error("publish compacted idempotency journal", e))?;

    if let Some(journal) = inner.journal.as_mut() {
        journal.lines = inner.entries.len() as u64;
    }
    Ok(())
}

/// Current wall clock in Unix seconds.
fn now_unix() -> i64 {
    chrono::Utc::now().timestamp()
}

/// Map a persistence failure onto the crate error type.
///
/// The context says which step failed; the cause is the OS or serde
/// message. Event ids and file paths are deliberately not echoed — this
/// crate does not put request input into errors, and the caller can
/// find both from the request it already holds.
fn persist_error<E: std::fmt::Display>(context: &str, cause: E) -> BillingError {
    BillingError::IdempotencyPersist(format!("{context}: {cause}"))
}

/// Trait alias for "something that can receive a `BillingEvent`".
/// The api-gateway implements this; tests use a `mpsc::UnboundedSender`.
pub trait EventSink: Send + Sync + 'static {
    /// Receive one accepted [`BillingEvent`].
    fn handle(&self, ev: BillingEvent);
}

/// Outcome of a webhook call. The handler returns this so the route
/// in api-gateway can choose the right HTTP status code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebhookOutcome {
    /// The event was accepted and dispatched.
    Accepted,
    /// Replay: same `event.id` for this tenant already seen.
    Duplicate,
}

/// Webhook handler. Holds config (with the signing secret) and the
/// idempotency store. The api-gateway route
/// `POST /webhooks/stripe` constructs a `WebhookHandler` per
/// request — the config + idempotency store are shared.
#[derive(Debug, Clone)]
pub struct WebhookHandler {
    cfg: Arc<Config>,
    idem: Arc<IdempotencyStore>,
}

impl WebhookHandler {
    /// Build a handler over the shared config + idempotency store.
    #[must_use]
    pub fn new(cfg: Arc<Config>, idem: Arc<IdempotencyStore>) -> Self {
        Self { cfg, idem }
    }

    /// Verify `Stripe-Signature` and return `Ok(())` or an error.
    /// The header format is `t=<unix>,v1=<hex>[, v1=<hex>]*` — we
    /// check the **first** `v1` signature and require it to match a
    /// fresh HMAC of `<t>.<body>` keyed by the webhook secret.
    ///
    /// The `body` is the **raw** request body bytes (form-encoded
    /// JSON in modern API versions).
    pub fn verify_signature(&self, header: &str, body: &[u8], now_unix: i64) -> Result<()> {
        let mut t: Option<i64> = None;
        let mut v1: Option<&str> = None;
        for part in header.split(',') {
            let (k, v) = part.split_once('=').ok_or(BillingError::InvalidSignature)?;
            match k.trim() {
                "t" => t = v.trim().parse().ok(),
                "v1" if v1.is_none() => v1 = Some(v.trim()),
                _ => {}
            }
        }
        let t = t.ok_or(BillingError::InvalidSignature)?;
        if (now_unix - t).abs() > 300 {
            // 5-minute tolerance window.
            return Err(BillingError::InvalidSignature);
        }
        let expected = v1.ok_or(BillingError::InvalidSignature)?;
        let mut mac = HmacSha256::new_from_slice(self.cfg.stripe_webhook_secret.as_bytes())
            .map_err(|_| BillingError::InvalidSignature)?;
        mac.update(format!("{t}.").as_bytes());
        mac.update(body);
        let got = hex::encode(mac.finalize().into_bytes());
        if bool::from(subtle::ConstantTimeEq::ct_eq(
            got.as_bytes(),
            expected.as_bytes(),
        )) {
            Ok(())
        } else {
            Err(BillingError::InvalidSignature)
        }
    }

    /// Process a verified event. Returns the outcome so the
    /// api-gateway route can pick the right HTTP status.
    pub fn handle(&self, body: &[u8], sink: &dyn EventSink) -> Result<WebhookOutcome> {
        let env: serde_json::Value =
            serde_json::from_slice(body).map_err(|_| BillingError::InvalidPayload)?;
        let event_id = env
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or(BillingError::MalformedEnvelope)?
            .to_owned();
        let kind_str = env
            .get("type")
            .and_then(|v| v.as_str())
            .ok_or(BillingError::MalformedEnvelope)?;
        let kind: EventKind = kind_str.parse()?;
        // Tenant resolution: in v0.4.0 we tag every event with the
        // tenant that owns the customer. The full mapping comes from
        // the api-gateway's tenant context; for the v0.4.0 skeleton
        // we read it from `data.object.metadata.tenant_id`.
        let tenant_id = env
            .pointer("/data/object/metadata/tenant_id")
            .and_then(|v| v.as_str())
            .ok_or(BillingError::MalformedEnvelope)?
            .to_owned();
        let target_id = env
            .pointer("/data/object/id")
            .and_then(|v| v.as_str())
            .ok_or(BillingError::MalformedEnvelope)?
            .to_owned();

        // Validate the tenant **before** the idempotency table is
        // touched. Registering the key first would make a non-UUID
        // `metadata.tenant_id` permanently swallow the event: the
        // first delivery errors out, and every Stripe retry then hits
        // `has_seen` and is answered `Duplicate` — a success Stripe
        // stops retrying, so the subscription change is lost with no
        // error surfaced. Validation must strictly precede dedup
        // registration.
        let tenant = TenantId(
            uuid::Uuid::parse_str(&tenant_id).map_err(|_| BillingError::MalformedEnvelope)?,
        );

        // Check-and-record in ONE step.
        //
        // This used to be:
        //
        //     if self.idem.has_seen(&event_id, &tenant_id) {
        //         return Ok(WebhookOutcome::Duplicate);
        //     }
        //     self.idem.record(&event_id, &tenant_id);   // return value discarded
        //
        // which takes the read lock, drops it, then takes the write lock.
        // Two deliveries of the same event that interleave between those
        // two acquisitions both observe "not seen", both insert, and both
        // dispatch — the subscription change is applied twice, from one
        // Stripe event, on the money path.
        //
        // `record` returns whether the key was new for exactly this
        // reason. Deciding the answer under the same write lock that
        // performs the insert is what makes the loser of the race receive
        // `Duplicate` instead of an event it has already processed.
        //
        // Note `has_seen` below is NOT the dedup path and must not be used
        // as one. It is a separate read lock, so pairing it with `record`
        // reintroduces the window this line removed.
        //
        // A persistence failure propagates as an error, and the event is
        // NOT dispatched. The caller turns that into a 5xx, Stripe keeps
        // retrying, and the key is still absent, so the retry is
        // processed. Dispatching an event we could not record would be
        // the worse of the two: nothing would stop the same event being
        // processed again after a restart.
        match self.idem.record(&event_id, &tenant_id) {
            Ok(RecordOutcome::Duplicate) => return Ok(WebhookOutcome::Duplicate),
            Ok(RecordOutcome::Fresh) => {}
            Err(e) => return Err(e),
        }
        // Audit emission goes through ada-m11-rbac-collab in the
        // api-gateway wiring; here we only emit the BillingEvent.
        sink.handle(BillingEvent {
            event_id,
            kind,
            tenant_id: tenant,
            target_id,
        });
        Ok(WebhookOutcome::Accepted)
    }
}

/// Convenience wrapper used by the api-gateway route. Holds the
/// handler + an event sink + the mpsc sender used by tests.
pub struct WebhookService {
    /// The signature-verifying / deduplicating handler.
    pub handler: WebhookHandler,
    /// The downstream receiver for accepted events.
    pub sink: Arc<dyn EventSink>,
}

// Manual rather than derived: `Arc<dyn EventSink>` is not `Debug`, and
// the sink is an opaque trait object whose internals are not ours to
// print. Eliding it keeps the impl total without leaking event data.
impl core::fmt::Debug for WebhookService {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("WebhookService")
            .field("handler", &self.handler)
            .field("sink", &"<dyn EventSink>")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evt(event_id: &str, kind: &str, tenant: &str, target: &str) -> serde_json::Value {
        serde_json::json!({
            "id": event_id,
            "type": kind,
            "data": {
                "object": {
                    "id": target,
                    "metadata": { "tenant_id": tenant }
                }
            }
        })
    }

    struct CaptureSink(std::sync::Mutex<Vec<BillingEvent>>);
    impl EventSink for CaptureSink {
        fn handle(&self, ev: BillingEvent) {
            self.0.lock().unwrap().push(ev);
        }
    }

    fn sign(secret: &str, t: i64, body: &[u8]) -> String {
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(format!("{t}.").as_bytes());
        mac.update(body);
        format!("t={t},v1={}", hex::encode(mac.finalize().into_bytes()))
    }

    #[test]
    fn signature_verifies_within_tolerance() {
        let cfg = Arc::new(Config {
            stripe_secret_key: "sk_test_dummy".into(),
            stripe_webhook_secret: "whsec_test".into(),
            stripe_api_version: "2025-08-27.basil".into(),
            stripe_portal_return_url: None,
            stripe_base_url: "https://api.stripe.com/v1".into(),
        });
        let h = WebhookHandler::new(cfg, Arc::new(IdempotencyStore::new()));
        let body = b"{\"id\":\"evt_1\"}";
        let now = 1_700_000_000_i64;
        let header = sign("whsec_test", now, body);
        assert!(h.verify_signature(&header, body, now).is_ok());
    }

    #[test]
    fn signature_rejects_outside_tolerance() {
        let cfg = Arc::new(Config {
            stripe_secret_key: "sk_test_dummy".into(),
            stripe_webhook_secret: "whsec_test".into(),
            stripe_api_version: "2025-08-27.basil".into(),
            stripe_portal_return_url: None,
            stripe_base_url: "https://api.stripe.com/v1".into(),
        });
        let h = WebhookHandler::new(cfg, Arc::new(IdempotencyStore::new()));
        let body = b"{\"id\":\"evt_1\"}";
        let now = 1_700_000_000_i64;
        let header = sign("whsec_test", now - 600, body);
        assert!(h.verify_signature(&header, body, now).is_err());
    }

    #[test]
    fn signature_rejects_wrong_secret() {
        let cfg = Arc::new(Config {
            stripe_secret_key: "sk_test_dummy".into(),
            stripe_webhook_secret: "whsec_correct".into(),
            stripe_api_version: "2025-08-27.basil".into(),
            stripe_portal_return_url: None,
            stripe_base_url: "https://api.stripe.com/v1".into(),
        });
        let h = WebhookHandler::new(cfg, Arc::new(IdempotencyStore::new()));
        let body = b"{\"id\":\"evt_1\"}";
        let now = 1_700_000_000_i64;
        let header = sign("whsec_wrong", now, body);
        assert!(h.verify_signature(&header, body, now).is_err());
    }

    #[test]
    fn handle_emits_event_and_dedupes_replay() {
        let cfg = Arc::new(Config {
            stripe_secret_key: "sk_test_dummy".into(),
            stripe_webhook_secret: "whsec".into(),
            stripe_api_version: "2025-08-27.basil".into(),
            stripe_portal_return_url: None,
            stripe_base_url: "https://api.stripe.com/v1".into(),
        });
        let h = WebhookHandler::new(cfg, Arc::new(IdempotencyStore::new()));
        let sink = std::sync::Arc::new(CaptureSink(std::sync::Mutex::new(Vec::new())));
        let body = serde_json::to_vec(&evt(
            "evt_1",
            "customer.subscription.updated",
            // Must be a parseable UUID: `handle` converts the metadata
            // tenant into a `TenantId` via `Uuid::parse_str` and returns
            // `MalformedEnvelope` otherwise, which would fail the
            // `.expect("first")` below for the wrong reason.
            "018f0000-0000-4000-8000-000000000001",
            "sub_1",
        ))
        .unwrap();

        let first = h.handle(&body, &*sink).expect("first");
        assert_eq!(first, WebhookOutcome::Accepted);
        let second = h.handle(&body, &*sink).expect("second");
        assert_eq!(second, WebhookOutcome::Duplicate);

        let captured = sink.0.lock().unwrap();
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0].event_id, "evt_1");
        assert_eq!(captured[0].kind, EventKind::CustomerSubscriptionUpdated);
    }

    #[test]
    fn handle_rejects_malformed_envelope() {
        let cfg = Arc::new(Config {
            stripe_secret_key: "sk_test_dummy".into(),
            stripe_webhook_secret: "whsec".into(),
            stripe_api_version: "2025-08-27.basil".into(),
            stripe_portal_return_url: None,
            stripe_base_url: "https://api.stripe.com/v1".into(),
        });
        let h = WebhookHandler::new(cfg, Arc::new(IdempotencyStore::new()));
        let sink = std::sync::Arc::new(CaptureSink(std::sync::Mutex::new(Vec::new())));
        let body = b"not json";
        let r = h.handle(body, &*sink);
        assert!(matches!(r, Err(BillingError::InvalidPayload)));
    }

    #[test]
    fn idempotency_store_basics() {
        let s = IdempotencyStore::new();
        assert_eq!(s.record("evt_a", "tenant_a").unwrap(), RecordOutcome::Fresh);
        assert_eq!(
            s.record("evt_a", "tenant_a").unwrap(),
            RecordOutcome::Duplicate
        );
        assert!(s.has_seen("evt_a", "tenant_a"));
        assert!(!s.has_seen("evt_b", "tenant_a"));
        assert!(!s.is_empty());
        assert_eq!(s.len(), 1);
    }

    // -----------------------------------------------------------------
    // Retention
    // -----------------------------------------------------------------

    /// A scratch directory for the journal tests, removed on drop.
    ///
    /// Built by hand because `tempfile` is not a dependency of this
    /// crate; `ada-m01-acquisition/tests/integration.rs` takes the same
    /// trade-off for the same reason.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(tag: &str) -> Self {
            static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let mut dir = std::env::temp_dir();
            dir.push(format!("ada-billing-idem-{tag}-{}-{n}", std::process::id()));
            fs::create_dir_all(&dir).expect("scratch dir");
            Self(dir)
        }

        fn journal(&self) -> PathBuf {
            self.0.join("idempotency.jsonl")
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    const T0: i64 = 1_700_000_000;

    /// The TTL and the retry window are related by a number, and the
    /// number is the whole safety argument: a key that expires while
    /// Stripe is still entitled to redeliver the event is a
    /// double charge. Pin both values, and pin that the TTL is exactly
    /// the window.
    ///
    /// The TTL is 72 h and Stripe's automatic redelivery window is
    /// three days, so the TTL is *equal* to the window, not longer. It
    /// covers the window, and the last delivery inside the window lands
    /// on the expiry boundary — which is the residual risk this test
    /// documents rather than hides.
    ///
    /// Equality rather than "at least" is deliberate: a TTL longer than
    /// the window would hold every key past the last moment Stripe can
    /// redeliver within, which costs memory and buys nothing.
    #[test]
    fn retention_ttl_equals_the_stripe_retry_window() {
        assert_eq!(
            STRIPE_RETRY_WINDOW_SECS,
            3 * 24 * 60 * 60,
            "the retry window this code accounts for is Stripe's three days \
             of automatic redelivery"
        );
        assert_eq!(
            IDEMPOTENCY_TTL_SECS,
            72 * 60 * 60,
            "the TTL is the 72 h product decision; changing it is a product \
             change, not a refactor"
        );
        // The relationship, asserted directly.
        //
        // This started life as `assert!(IDEMPOTENCY_TTL_SECS >=
        // STRIPE_RETRY_WINDOW_SECS)`, which compares two `const`s, so the
        // whole condition is a compile-time constant and clippy's
        // `assertions_on_constants` rejects it. The two `assert_eq!`
        // above already pin each constant's value on its own, so this
        // third line exists purely as a tripwire: it fails if someone
        // moves one constant without moving the other and breaks the
        // safety argument between them.
        //
        // `assert_eq!` states the same relationship without asking clippy
        // to evaluate a constant boolean, and it is the *stronger*
        // claim -- equality, not merely ">=". That is what the module
        // documents: the TTL is exactly Stripe's window, not longer than
        // it. A TTL above the window would mean a key outliving the
        // period Stripe can redeliver within, which is memory held for
        // nothing and a discrepancy an operator would have to notice by
        // reading two constants side by side.
        assert_eq!(
            IDEMPOTENCY_TTL_SECS, STRIPE_RETRY_WINDOW_SECS,
            "a TTL that differs from the retry window re-admits or outlives \
             Stripe retries: TTL {IDEMPOTENCY_TTL_SECS}s vs \
             window {STRIPE_RETRY_WINDOW_SECS}s"
        );
    }

    /// The behavioural half of the same claim: a key recorded at the
    /// first attempt is still held at the last instant of Stripe's
    /// retry window, and is gone once the TTL closes. The constants
    /// test alone would still pass if the sweep were wrong.
    #[test]
    fn a_key_outlives_the_stripe_retry_window_and_then_expires() {
        let s = IdempotencyStore::new();
        assert_eq!(
            s.record_at("evt_1", "t1", T0).unwrap(),
            RecordOutcome::Fresh
        );

        // The final delivery Stripe is entitled to make.
        assert_eq!(
            s.record_at("evt_1", "t1", T0 + STRIPE_RETRY_WINDOW_SECS)
                .unwrap(),
            RecordOutcome::Duplicate,
            "a key must not be forgotten while Stripe may still redeliver it"
        );

        // The TTL boundary is inclusive of the key: `expires_at` is
        // swept at exactly the expiry second, not after it.
        assert_eq!(
            s.record_at("evt_1", "t1", T0 + IDEMPOTENCY_TTL_SECS)
                .unwrap(),
            RecordOutcome::Fresh,
            "the key must be reclaimable once its TTL has closed"
        );
        assert_eq!(s.len(), 1, "the sweep must not leave the lapsed key behind");
    }

    /// Aging cleanup: keys past their TTL are reclaimed on the write
    /// path, so a store that sees a steady stream of new events holds
    /// one TTL's worth of keys and not all of them.
    #[test]
    fn the_sweep_reclaims_expired_keys() {
        let s = IdempotencyStore::new();
        for i in 0..3 {
            assert_eq!(
                s.record_at(&format!("evt_{i}"), "t", T0).unwrap(),
                RecordOutcome::Fresh
            );
        }
        assert_eq!(s.len(), 3, "precondition: three keys held");

        // One new event, arriving after the three lapsed.
        assert_eq!(
            s.record_at("evt_new", "t", T0 + IDEMPOTENCY_TTL_SECS)
                .unwrap(),
            RecordOutcome::Fresh
        );

        assert_eq!(
            s.len(),
            1,
            "the three lapsed keys must be reclaimed, leaving only the new one"
        );
        for i in 0..3 {
            assert!(
                !s.has_seen_at(&format!("evt_{i}"), "t", T0 + IDEMPOTENCY_TTL_SECS),
                "a lapsed key must not be retained"
            );
        }
    }

    /// Capacity ceiling, first half: reaching it reclaims expired keys
    /// rather than refusing. A store that answered `Err` here would
    /// fail closed on the route that receives every Stripe event —
    /// a permanent outage, not a degraded mode.
    #[test]
    fn at_capacity_the_store_reclaims_rather_than_refusing() {
        let s = IdempotencyStore::with_policy(IdempotencyPolicy {
            ttl_secs: IDEMPOTENCY_TTL_SECS,
            max_entries: 2,
        });
        assert_eq!(s.record_at("evt_a", "t", T0).unwrap(), RecordOutcome::Fresh);
        assert_eq!(s.record_at("evt_b", "t", T0).unwrap(), RecordOutcome::Fresh);
        assert_eq!(s.len(), 2, "precondition: the ceiling is reached");
        assert!(!s.is_over_capacity(), "at the ceiling is not over it");

        // Both of the held keys have lapsed by now, so there is room
        // again without dropping anything that is still in use.
        for id in ["evt_c", "evt_d"] {
            assert_eq!(
                s.record_at(id, "t", T0 + IDEMPOTENCY_TTL_SECS).unwrap(),
                RecordOutcome::Fresh,
                "a write at the ceiling must succeed once there is \
                 something expired to reclaim"
            );
        }
        assert_eq!(s.len(), 2, "the ceiling must be held by reclaiming");
        assert!(!s.is_over_capacity());
        assert_eq!(
            s.overflow_admissions(),
            0,
            "nothing had to be admitted over it"
        );
    }

    /// Capacity ceiling, second half: when every held key is still
    /// inside the retry window there is nothing safe to reclaim, and
    /// the store must still accept the event. Refusing would be a
    /// permanent outage; evicting a live key would hand the same event
    /// to the downstream sink twice. It admits, and counts.
    #[test]
    fn a_full_store_of_live_keys_admits_and_counts_rather_than_refusing() {
        let s = IdempotencyStore::with_policy(IdempotencyPolicy {
            ttl_secs: IDEMPOTENCY_TTL_SECS,
            max_entries: 2,
        });
        assert_eq!(s.record_at("evt_a", "t", T0).unwrap(), RecordOutcome::Fresh);
        assert_eq!(s.record_at("evt_b", "t", T0).unwrap(), RecordOutcome::Fresh);

        assert_eq!(
            s.record_at("evt_c", "t", T0).unwrap(),
            RecordOutcome::Fresh,
            "a full store must not fail closed on the webhook path"
        );
        assert!(s.is_over_capacity(), "the ceiling must be reportable");
        assert_eq!(s.overflow_admissions(), 1);
        assert_eq!(s.len(), 3, "the admission must actually be held");

        // And the keys it kept are still doing their job.
        for id in ["evt_a", "evt_b", "evt_c"] {
            assert_eq!(
                s.record_at(id, "t", T0 + STRIPE_RETRY_WINDOW_SECS - 1)
                    .unwrap(),
                RecordOutcome::Duplicate,
                "{id} is still inside the retry window and must dedupe"
            );
        }
    }

    /// Durability: the reason the journal exists. A key written by one
    /// store instance is read back by the next one over the same path,
    /// so a restart cannot re-allow a charge.
    #[test]
    fn a_recorded_key_survives_a_restart() {
        let scratch = Scratch::new("restart");
        let journal = scratch.journal();
        // Real time: `open` drops keys that are already past their TTL
        // relative to the wall clock, so a synthetic T0 would be
        // reclaimed on load and the test would pass for the wrong reason.
        let t0 = now_unix();

        {
            let first = IdempotencyStore::open(&journal, IdempotencyPolicy::default()).unwrap();
            assert_eq!(
                first.record_at("evt_1", "t1", t0).unwrap(),
                RecordOutcome::Fresh
            );
            assert!(journal.exists(), "the append must have created the file");
        }

        let reopened = IdempotencyStore::open(&journal, IdempotencyPolicy::default()).unwrap();
        assert_eq!(reopened.len(), 1, "the key must be read back off disk");
        assert_eq!(
            reopened.record_at("evt_1", "t1", t0).unwrap(),
            RecordOutcome::Duplicate,
            "a restart must not re-allow a duplicate charge"
        );
    }

    /// The journal does not grow without bound: compaction drops the
    /// dead lines, so the file on disk tracks the live set.
    #[test]
    fn compaction_keeps_the_journal_to_the_live_set() {
        let scratch = Scratch::new("compaction");
        let journal = scratch.journal();
        let s = IdempotencyStore::open(&journal, IdempotencyPolicy::default()).unwrap();
        let t0 = now_unix();

        for i in 0..10 {
            s.record_at(&format!("evt_{i}"), "t", t0).unwrap();
        }
        // A later wave, long enough after the first that the write-path
        // sweep has something to reclaim.
        for i in 10..20 {
            s.record_at(&format!("evt_{i}"), "t", t0 + IDEMPOTENCY_TTL_SECS)
                .unwrap();
        }

        assert_eq!(s.len(), 10, "only the second wave is live");
        s.compact().unwrap();
        let lines = fs::read_to_string(&journal)
            .unwrap()
            .lines()
            .filter(|l| !l.trim().is_empty())
            .count();
        assert_eq!(
            lines, 10,
            "after compaction the journal must hold only the live keys"
        );
    }

    /// Fail-closed on the write path. A journal that cannot be written
    /// must produce an error and must not admit the key in memory
    /// either — a key in memory that is not on disk is a key a restart
    /// forgets, which is the double-charge direction.
    #[test]
    fn a_journal_that_cannot_be_written_fails_closed() {
        let scratch = Scratch::new("unwritable");
        let journal = scratch.journal();
        let s = IdempotencyStore::open(&journal, IdempotencyPolicy::default()).unwrap();
        let t0 = now_unix();
        assert_eq!(
            s.record_at("evt_ok", "t", t0).unwrap(),
            RecordOutcome::Fresh
        );

        // Put a directory where the journal is. Opening it for append
        // then fails, on every platform this runs on.
        fs::remove_file(&journal).unwrap();
        fs::create_dir(&journal).unwrap();

        let err = s
            .record_at("evt_2", "t", t0)
            .expect_err("an unpersistable key must not be admitted");
        assert!(
            matches!(err, BillingError::IdempotencyPersist(_)),
            "got {err}"
        );
        assert!(
            !s.has_seen_at("evt_2", "t", t0),
            "a key that failed to persist must not be held in memory"
        );
    }

    /// The same fail-closed rule at the level that matters: a delivery
    /// the store cannot record is not dispatched, and the retry is
    /// still processable.
    #[test]
    fn a_delivery_that_cannot_be_recorded_is_not_dispatched() {
        let scratch = Scratch::new("handle-fail");
        let journal = scratch.journal();
        let idem =
            Arc::new(IdempotencyStore::open(&journal, IdempotencyPolicy::default()).unwrap());
        let cfg = Arc::new(Config {
            stripe_secret_key: "sk_test_dummy".into(),
            stripe_webhook_secret: "whsec".into(),
            stripe_api_version: "2025-08-27.basil".into(),
            stripe_portal_return_url: None,
            stripe_base_url: "https://api.stripe.com/v1".into(),
        });
        let h = WebhookHandler::new(cfg, Arc::clone(&idem));
        let sink = Arc::new(CaptureSink(std::sync::Mutex::new(Vec::new())));
        let body = serde_json::to_vec(&evt(
            "evt_1",
            "invoice.paid",
            "018f0000-0000-4000-8000-000000000001",
            "in_1",
        ))
        .unwrap();

        assert_eq!(h.handle(&body, &*sink).unwrap(), WebhookOutcome::Accepted);
        assert_eq!(sink.0.lock().unwrap().len(), 1);

        fs::remove_file(&journal).unwrap();
        fs::create_dir(&journal).unwrap();

        let err = h
            .handle(&body, &*sink)
            .expect_err("a delivery that cannot be recorded must not be accepted");
        assert!(
            matches!(err, BillingError::IdempotencyPersist(_)),
            "got {err}"
        );
        assert_eq!(
            sink.0.lock().unwrap().len(),
            1,
            "the retry must not be dispatched twice"
        );
    }

    /// The dedup decision and the dispatch have to be serialized with each
    /// other, not merely with other calls.
    ///
    /// `handle` used to read `has_seen` and *then* call `record`. Those are
    /// two separate lock acquisitions, so concurrent deliveries of one
    /// Stripe event could all pass the check before any of them inserted —
    /// and every one of them would go on to dispatch, applying the same
    /// subscription change several times from a single event. Stripe does
    /// deliver duplicates in practice (that is the whole reason this table
    /// exists), and nothing about the duplicate has to be simultaneous for
    /// the bug to be a bug: it only needs two deliveries to overlap.
    ///
    /// The barrier is what makes this a real test rather than a hopeful
    /// one. Without it the threads rarely collide and the old code would
    /// pass most of the time, which is the usual way a race "tests green"
    /// for months.
    #[test]
    fn concurrent_deliveries_of_one_event_dispatch_exactly_once() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Barrier;

        const THREADS: usize = 16;
        const ROUNDS: usize = 16;

        struct CountingSink(AtomicUsize);
        impl EventSink for CountingSink {
            fn handle(&self, _ev: BillingEvent) {
                self.0.fetch_add(1, Ordering::SeqCst);
            }
        }

        let mut accepted = 0usize;
        let mut duplicates = 0usize;
        let mut dispatched = 0usize;

        for round in 0..ROUNDS {
            let cfg = Arc::new(Config {
                stripe_secret_key: "sk_test_dummy".into(),
                stripe_webhook_secret: "whsec".into(),
                stripe_api_version: "2025-08-27.basil".into(),
                stripe_portal_return_url: None,
                stripe_base_url: "https://api.stripe.com/v1".into(),
            });
            let h = Arc::new(WebhookHandler::new(cfg, Arc::new(IdempotencyStore::new())));
            let sink = Arc::new(CountingSink(AtomicUsize::new(0)));
            let barrier = Arc::new(Barrier::new(THREADS));
            let body = serde_json::to_vec(&evt(
                &format!("evt_race_{round}"),
                "customer.subscription.updated",
                "018f0000-0000-4000-8000-000000000001",
                "sub_1",
            ))
            .unwrap();

            let handles: Vec<_> = (0..THREADS)
                .map(|_| {
                    let h = Arc::clone(&h);
                    let sink = Arc::clone(&sink);
                    let barrier = Arc::clone(&barrier);
                    let body = body.clone();
                    std::thread::spawn(move || {
                        barrier.wait();
                        h.handle(&body, &*sink)
                    })
                })
                .collect();

            for t in handles {
                match t.join().expect("worker thread") {
                    Ok(WebhookOutcome::Accepted) => accepted += 1,
                    Ok(WebhookOutcome::Duplicate) => duplicates += 1,
                    Err(e) => panic!("handle must not fail for a well-formed event: {e}"),
                }
            }
            dispatched += sink.0.load(Ordering::SeqCst);
        }

        assert_eq!(
            accepted, ROUNDS,
            "exactly one delivery per event may be Accepted"
        );
        assert_eq!(
            duplicates,
            THREADS * ROUNDS - ROUNDS,
            "every other delivery must be told Duplicate"
        );
        assert_eq!(
            dispatched, ROUNDS,
            "the sink must see each event exactly once — more than one \
             dispatch of a single Stripe event is a double charge"
        );
    }
}
