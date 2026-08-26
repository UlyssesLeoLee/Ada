//! In-process per-resource read/write lock manager.
//!
//! The skeleton implements a simple `Read` / `Write` lock manager
//! that fits the v0.1.0 collaboration use case (a single canvas is
//! edited by a small group; cross-cluster coordination is deferred
//! to M-16). Production builds will move to a distributed lock
//! service (Redis Redlock or the `rbac_grant` table with row-level
//! locks; see `DOC-MOD-011` §3.4).
//!
//! Concurrency model:
//! - Multiple readers can hold the lock simultaneously.
//! - A writer holds the lock alone (no readers, no other writers).
//! - `try_*` methods are non-blocking; `*_blocking` methods await
//!   until the lock can be acquired.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use parking_lot::Mutex;
use tokio::sync::Notify;

use crate::collaboration::ResourceId;
use crate::error::{RbacError, Result};

/// What kind of lock a holder wants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockKind {
    /// Shared / read lock.
    Read,
    /// Exclusive / write lock.
    Write,
}

impl LockKind {
    /// Short, lowercase string tag.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
        }
    }
}

/// A granted lock — `kind` plus the `holder` identifier (any
/// caller-chosen string, typically `UserId` or
/// `(tenant_id, user_id)`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lock {
    /// Resource the lock is on.
    pub resource: ResourceId,
    /// Kind of lock.
    pub kind: LockKind,
    /// Who holds the lock.
    pub holder: String,
}

#[derive(Debug)]
struct ResourceLockState {
    readers: HashSet<String>,
    writer: Option<String>,
    /// One Notify per waiter; we use a single shared Notify (Arc'd
    /// so we can clone it out from under the mutex when the wait
    /// condition fails) for all waiters on a resource, which is
    /// sufficient for the skeleton's bounded concurrency.
    notify: Arc<Notify>,
}

impl Default for ResourceLockState {
    fn default() -> Self {
        Self {
            readers: HashSet::new(),
            writer: None,
            notify: Arc::new(Notify::new()),
        }
    }
}

/// In-process per-resource lock manager.
#[derive(Debug, Default)]
pub struct LockManager {
    state: Mutex<HashMap<ResourceId, ResourceLockState>>,
}

impl LockManager {
    /// Empty lock manager.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn bucket(
        state: &mut HashMap<ResourceId, ResourceLockState>,
        resource: ResourceId,
    ) -> &mut ResourceLockState {
        state.entry(resource).or_default()
    }

    /// Acquire a write lock, waiting until the resource is free of
    /// readers and writers. Returns the granted [`Lock`] on success.
    pub async fn write_lock(
        &self,
        resource: ResourceId,
        holder: impl Into<String>,
    ) -> Result<Lock> {
        let holder = holder.into();
        loop {
            let notify = {
                let mut state = self.state.lock();
                let bucket = Self::bucket(&mut state, resource);
                if bucket.readers.is_empty() && bucket.writer.is_none() {
                    bucket.writer = Some(holder.clone());
                    return Ok(Lock {
                        resource,
                        kind: LockKind::Write,
                        holder,
                    });
                }
                bucket.notify.clone()
            };
            notify.notified().await;
        }
    }

    /// Acquire a read lock, waiting while a writer is present. Multiple
    /// readers on the same resource do not block each other.
    pub async fn read_lock(&self, resource: ResourceId, holder: impl Into<String>) -> Result<Lock> {
        let holder = holder.into();
        loop {
            let notify = {
                let mut state = self.state.lock();
                let bucket = Self::bucket(&mut state, resource);
                match &bucket.writer {
                    Some(w) if w != &holder => bucket.notify.clone(),
                    _ => {
                        bucket.readers.insert(holder.clone());
                        return Ok(Lock {
                            resource,
                            kind: LockKind::Read,
                            holder,
                        });
                    }
                }
            };
            notify.notified().await;
        }
    }

    /// Non-blocking write attempt. Returns `RbacError::LockHeld` if
    /// the resource is not exclusively free.
    pub fn try_write_lock(&self, resource: ResourceId, holder: impl Into<String>) -> Result<Lock> {
        let holder = holder.into();
        let mut state = self.state.lock();
        let bucket = Self::bucket(&mut state, resource);
        if bucket.readers.is_empty() && bucket.writer.is_none() {
            bucket.writer = Some(holder.clone());
            Ok(Lock {
                resource,
                kind: LockKind::Write,
                holder,
            })
        } else {
            let by = bucket
                .writer
                .clone()
                .or_else(|| bucket.readers.iter().next().cloned())
                .unwrap_or_else(|| "<unknown>".into());
            Err(RbacError::LockHeld {
                resource: resource.to_string(),
                holder: by,
            })
        }
    }

    /// Non-blocking read attempt.
    pub fn try_read_lock(&self, resource: ResourceId, holder: impl Into<String>) -> Result<Lock> {
        let holder = holder.into();
        let mut state = self.state.lock();
        let bucket = Self::bucket(&mut state, resource);
        if let Some(w) = &bucket.writer {
            if w != &holder {
                return Err(RbacError::LockHeld {
                    resource: resource.to_string(),
                    holder: w.clone(),
                });
            }
        }
        bucket.readers.insert(holder.clone());
        Ok(Lock {
            resource,
            kind: LockKind::Read,
            holder,
        })
    }

    /// Release a read lock. Returns `RbacError::LockNotHeld` if the
    /// holder doesn't actually hold a read lock on the resource.
    pub fn unlock_read(&self, resource: ResourceId, holder: &str) -> Result<()> {
        let mut state = self.state.lock();
        let bucket = Self::bucket(&mut state, resource);
        if !bucket.readers.remove(holder) {
            return Err(RbacError::LockNotHeld {
                resource: resource.to_string(),
                holder: holder.to_string(),
            });
        }
        bucket.notify.notify_waiters();
        Ok(())
    }

    /// Release a write lock.
    pub fn unlock_write(&self, resource: ResourceId, holder: &str) -> Result<()> {
        let mut state = self.state.lock();
        let bucket = Self::bucket(&mut state, resource);
        match &bucket.writer {
            Some(w) if w == holder => {
                bucket.writer = None;
                bucket.notify.notify_waiters();
                Ok(())
            }
            Some(_) | None => Err(RbacError::LockNotHeld {
                resource: resource.to_string(),
                holder: holder.to_string(),
            }),
        }
    }

    /// `true` if `resource` is currently held by someone (any
    /// writer or ≥1 reader).
    #[must_use]
    pub fn is_locked(&self, resource: ResourceId) -> bool {
        let mut state = self.state.lock();
        let bucket = Self::bucket(&mut state, resource);
        bucket.writer.is_some() || !bucket.readers.is_empty()
    }

    /// Number of readers currently holding a read lock on `resource`.
    #[must_use]
    pub fn reader_count(&self, resource: ResourceId) -> usize {
        let mut state = self.state.lock();
        let bucket = Self::bucket(&mut state, resource);
        bucket.readers.len()
    }

    /// Current writer of `resource`, if any.
    #[must_use]
    pub fn current_writer(&self, resource: ResourceId) -> Option<String> {
        let mut state = self.state.lock();
        let bucket = Self::bucket(&mut state, resource);
        bucket.writer.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resource() -> ResourceId {
        ResourceId::new()
    }

    #[test]
    fn try_write_succeeds_on_free_resource() {
        let mgr = LockManager::new();
        let lock = mgr
            .try_write_lock(resource(), "alice")
            .expect("free resource");
        assert_eq!(lock.holder, "alice");
        assert_eq!(lock.kind, LockKind::Write);
    }

    #[test]
    fn try_write_fails_when_writer_present() {
        let mgr = LockManager::new();
        let r = resource();
        mgr.try_write_lock(r, "alice").unwrap();
        let err = mgr.try_write_lock(r, "bob").expect_err("contention");
        assert!(matches!(err, RbacError::LockHeld { .. }));
    }

    #[test]
    fn try_read_fails_when_writer_present() {
        let mgr = LockManager::new();
        let r = resource();
        mgr.try_write_lock(r, "alice").unwrap();
        let err = mgr.try_read_lock(r, "bob").expect_err("writer present");
        assert!(matches!(err, RbacError::LockHeld { .. }));
    }

    #[test]
    fn multiple_readers_coexist() {
        let mgr = LockManager::new();
        let r = resource();
        mgr.try_read_lock(r, "alice").unwrap();
        mgr.try_read_lock(r, "bob").unwrap();
        assert_eq!(mgr.reader_count(r), 2);
        assert!(!mgr.is_locked(r) || mgr.reader_count(r) == 2);
    }

    #[test]
    fn write_lock_blocks_until_released() {
        // Use a multi-threaded runtime so we can release the lock
        // from a blocking task while the awaiter is parked.
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("build runtime");
        rt.block_on(async {
            let mgr = std::sync::Arc::new(LockManager::new());
            let r = resource();
            mgr.try_write_lock(r, "alice").unwrap();
            let mgr2 = mgr.clone();
            let task = tokio::spawn(async move { mgr2.write_lock(r, "bob").await });
            // Give the task a moment to actually block on the notify.
            tokio::time::sleep(std::time::Duration::from_millis(80)).await;
            assert!(!task.is_finished(), "bob should still be waiting");
            mgr.unlock_write(r, "alice").unwrap();
            let lock_result = tokio::time::timeout(std::time::Duration::from_secs(2), task)
                .await
                .expect("bob task should complete in time")
                .expect("bob task should not panic");
            let lock = lock_result.expect("write_lock should succeed after alice releases");
            assert_eq!(lock.holder, "bob");
        });
    }

    #[test]
    fn unlock_read_errors_for_missing_holder() {
        let mgr = LockManager::new();
        let r = resource();
        let err = mgr.unlock_read(r, "alice").expect_err("no lock");
        assert!(matches!(err, RbacError::LockNotHeld { .. }));
    }

    #[test]
    fn unlock_write_errors_for_wrong_holder() {
        let mgr = LockManager::new();
        let r = resource();
        mgr.try_write_lock(r, "alice").unwrap();
        let err = mgr.unlock_write(r, "bob").expect_err("wrong holder");
        assert!(matches!(err, RbacError::LockNotHeld { .. }));
    }

    #[test]
    fn current_writer_reports_holder() {
        let mgr = LockManager::new();
        let r = resource();
        assert_eq!(mgr.current_writer(r), None);
        mgr.try_write_lock(r, "alice").unwrap();
        assert_eq!(mgr.current_writer(r), Some("alice".into()));
    }
}
