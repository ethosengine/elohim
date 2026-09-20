//! A per-identity async lock: serialize check-then-act, never the whole world.
//!
//! # The bug class this exists for
//!
//! Two of this crate's caches decide "already done?" and then, only after a
//! CONDUCTOR ROUND TRIP, record the result:
//!
//! * [`crate::conductor::grant_memory`] — `decide` … `grant_zome_call_capability` … `record`
//! * [`crate::services::signing_credentials`] — `decide` … `authorize_signing_credentials` … `add_credentials`
//!
//! Both of those middle steps are `.await`s of many milliseconds, and both end
//! in a Holochain CHAIN WRITE that nothing revokes. A check-then-act pair with
//! an await in the middle and nothing spanning it is a race by construction:
//! two callers with the SAME identity both observe "not done", both complete
//! the round trip, and both author a grant.
//!
//! This is the common case, not an edge. The browser now persists its signing
//! keypair and cap secret, so two tabs opening at once present byte-identical
//! key material — the same fingerprint, entering the window together. On the
//! doorway's own side, a primary connect and a half-open re-probe can run at
//! once for one endpoint.
//!
//! # What this primitive promises
//!
//! One `tokio::sync::Mutex` PER KEY, acquired before the check and held across
//! the round trip, so the second caller waits and then re-decides — seeing
//! `Skip`/`Reuse` if the first succeeded, and proceeding to act if the first
//! failed (nothing was recorded for a failure).
//!
//! Five properties, each load-bearing:
//!
//! 1. **No global serialization.** Distinct keys never contend: a wedged
//!    conductor for one human cannot stall another human's connect. Only the
//!    tiny map lookup is shared, and it is held for nanoseconds.
//! 2. **No `std::sync::Mutex` across an await.** The map is a `std` mutex
//!    locked only SYNCHRONOUSLY, long enough to clone or insert an `Arc`; the
//!    thing awaited is the per-key `tokio::sync::Mutex`.
//! 3. **The map does not grow without bound.** When a guard drops it releases
//!    the tokio mutex first, then — under the map lock — removes the entry if
//!    its `Arc::strong_count` is 1 (the map's own reference). A waiter always
//!    holds its own clone, taken under the same map lock, so "count is 1" and
//!    "somebody is waiting" cannot both be true.
//! 4. **Bounded waiting.** [`KeyedLock::acquire_within`] returns `None` rather
//!    than waiting forever behind a wedged holder. `None` means "unserialized"
//!    — the caller falls through to exactly its pre-lock behaviour, which is a
//!    possible duplicate grant, never a failed connect and never a skipped
//!    grant that did not land. Degrading to today is the correct expiry.
//! 5. **Cancellation-safe.** The guard's `Drop` does the release, so a caller
//!    that is dropped mid-round-trip frees the key immediately. Nothing is
//!    recorded for it, because recording is the caller's explicit step after a
//!    success.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

type KeyMutex = Arc<tokio::sync::Mutex<()>>;

/// A map of per-key async mutexes that cleans up after itself.
#[derive(Debug, Default)]
pub struct KeyedLock {
    keys: Mutex<HashMap<String, KeyMutex>>,
}

impl KeyedLock {
    pub fn new() -> Self {
        Self::default()
    }

    /// Clone (or create) the mutex for `key`. Synchronous and brief — the
    /// `std` map lock is never held across an await.
    fn handle(&self, key: &str) -> KeyMutex {
        let mut keys = self.keys.lock().unwrap_or_else(|e| e.into_inner());
        Arc::clone(
            keys.entry(key.to_string())
                .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(()))),
        )
    }

    /// Drop the entry for `key` if nobody holds or wants it.
    ///
    /// The caller MUST have released its own `Arc` first. A count of exactly 1
    /// means the map itself is the only remaining reference.
    fn release(&self, key: &str) {
        let mut keys = self.keys.lock().unwrap_or_else(|e| e.into_inner());
        let idle = keys.get(key).is_some_and(|m| Arc::strong_count(m) == 1);
        if idle {
            keys.remove(key);
        }
    }

    /// Take the lock for `key`, waiting at most `deadline`.
    ///
    /// `Some(guard)` — serialized; hold it across the whole check-act-record
    /// sequence. `None` — the holder outlived the deadline, so the caller
    /// proceeds UNSERIALIZED, which is precisely the behaviour that existed
    /// before this primitive. See property 4 above.
    pub async fn acquire_within(&self, key: &str, deadline: Duration) -> Option<KeyedGuard<'_>> {
        let lock = self.handle(key);
        match tokio::time::timeout(deadline, Arc::clone(&lock).lock_owned()).await {
            Ok(guard) => Some(KeyedGuard {
                owner: self,
                key: key.to_string(),
                guard: Some(guard),
            }),
            Err(_elapsed) => {
                // Release OUR reference before asking whether the entry is idle,
                // or we would always see ourselves and never clean up.
                drop(lock);
                self.release(key);
                None
            }
        }
    }

    /// How many keys are currently held or contended. Diagnostics and tests —
    /// this is the number that must return to zero.
    pub fn live_keys(&self) -> usize {
        self.keys.lock().unwrap_or_else(|e| e.into_inner()).len()
    }
}

/// Holds one key's mutex. Releasing is `Drop`, so cancellation frees the key.
#[derive(Debug)]
pub struct KeyedGuard<'a> {
    owner: &'a KeyedLock,
    key: String,
    /// `Option` only so `Drop` can release the tokio mutex BEFORE the map is
    /// asked whether the entry is idle.
    guard: Option<tokio::sync::OwnedMutexGuard<()>>,
}

impl Drop for KeyedGuard<'_> {
    fn drop(&mut self) {
        // Order matters: dropping the owned guard releases both the mutex and
        // the `Arc` it carries, so the count seen by `release` is honest.
        drop(self.guard.take());
        self.owner.release(&self.key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::sync::Notify;

    const LONG: Duration = Duration::from_secs(30);

    /// THE INVARIANT this primitive exists for: two callers on the SAME key
    /// cannot be inside the window at once. The second observes the first's
    /// effect, not a stale "not done".
    #[tokio::test]
    async fn the_same_key_serializes_check_then_act() {
        let lock = Arc::new(KeyedLock::new());
        let done = Arc::new(AtomicUsize::new(0));
        let inside = Arc::new(AtomicUsize::new(0));
        let hold = Arc::new(Notify::new());

        let a = {
            let (lock, done, inside, hold) = (
                Arc::clone(&lock),
                Arc::clone(&done),
                Arc::clone(&inside),
                Arc::clone(&hold),
            );
            tokio::spawn(async move {
                let _g = lock.acquire_within("same", LONG).await.expect("acquired");
                // Provably inside the window until released.
                assert_eq!(inside.fetch_add(1, Ordering::SeqCst), 0);
                hold.notified().await;
                inside.fetch_sub(1, Ordering::SeqCst);
                done.fetch_add(1, Ordering::SeqCst);
            })
        };

        // Let A get in first.
        tokio::task::yield_now().await;
        assert_eq!(inside.load(Ordering::SeqCst), 1);

        let b = {
            let (lock, done, inside) = (Arc::clone(&lock), Arc::clone(&done), Arc::clone(&inside));
            tokio::spawn(async move {
                let _g = lock.acquire_within("same", LONG).await.expect("acquired");
                // If the lock did not serialize, A would still be counted here.
                assert_eq!(inside.load(Ordering::SeqCst), 0, "B entered while A held");
                done.fetch_add(1, Ordering::SeqCst);
            })
        };

        // B must NOT have finished while A holds.
        tokio::task::yield_now().await;
        assert_eq!(done.load(Ordering::SeqCst), 0);

        hold.notify_one();
        a.await.unwrap();
        b.await.unwrap();
        assert_eq!(done.load(Ordering::SeqCst), 2);
    }

    /// NO GLOBAL SERIALIZATION. A wedged holder on one key must not stall a
    /// different key — a slow conductor for one human cannot block another's
    /// connect.
    #[tokio::test]
    async fn different_keys_never_contend() {
        let lock = Arc::new(KeyedLock::new());
        let wedged = Arc::new(Notify::new());

        let stuck = {
            let (lock, wedged) = (Arc::clone(&lock), Arc::clone(&wedged));
            tokio::spawn(async move {
                let _g = lock
                    .acquire_within("human-a", LONG)
                    .await
                    .expect("acquired");
                wedged.notified().await;
            })
        };
        tokio::task::yield_now().await;

        // The other identity completes while the first is still blocked.
        let other = tokio::time::timeout(Duration::from_secs(1), async {
            let _g = lock
                .acquire_within("human-b", LONG)
                .await
                .expect("acquired");
        })
        .await;
        assert!(other.is_ok(), "a wedged key must not stall a different key");

        wedged.notify_one();
        stuck.await.unwrap();
    }

    /// BOUNDED WAITING. A waiter behind a wedged holder is released by the
    /// deadline with `None` — "proceed unserialized" — never an error and never
    /// an indefinite park.
    #[tokio::test]
    async fn a_waiter_is_released_by_the_deadline() {
        let lock = Arc::new(KeyedLock::new());
        let wedged = Arc::new(Notify::new());

        let stuck = {
            let (lock, wedged) = (Arc::clone(&lock), Arc::clone(&wedged));
            tokio::spawn(async move {
                let _g = lock.acquire_within("wedged", LONG).await.expect("acquired");
                wedged.notified().await;
            })
        };
        tokio::task::yield_now().await;

        let waited = lock
            .acquire_within("wedged", Duration::from_millis(50))
            .await;
        assert!(
            waited.is_none(),
            "the waiter must fall through, not wait on a wedged holder forever"
        );

        wedged.notify_one();
        stuck.await.unwrap();
    }

    /// CANCELLATION. A caller dropped mid-window releases the key immediately,
    /// so the next caller is not blocked by a task that no longer exists.
    #[tokio::test]
    async fn an_aborted_holder_releases_the_key() {
        let lock = Arc::new(KeyedLock::new());
        let entered = Arc::new(Notify::new());

        let doomed = {
            let (lock, entered) = (Arc::clone(&lock), Arc::clone(&entered));
            tokio::spawn(async move {
                let _g = lock.acquire_within("k", LONG).await.expect("acquired");
                entered.notify_one();
                std::future::pending::<()>().await;
            })
        };
        entered.notified().await;
        doomed.abort();
        let _ = doomed.await;

        let after = tokio::time::timeout(Duration::from_secs(1), lock.acquire_within("k", LONG))
            .await
            .expect("an aborted holder must not hold the key");
        assert!(after.is_some());
        drop(after);
        assert_eq!(lock.live_keys(), 0);
    }

    /// THE MAP DOES NOT GROW. Every key is removed once its last holder and
    /// waiter are gone — including after a contended run.
    #[tokio::test]
    async fn the_key_map_returns_to_empty() {
        let lock = Arc::new(KeyedLock::new());

        for key in ["a", "b", "c"] {
            let g = lock.acquire_within(key, LONG).await.expect("acquired");
            assert_eq!(lock.live_keys(), 1, "one key live at a time");
            drop(g);
            assert_eq!(lock.live_keys(), 0);
        }

        // Contended: the entry must survive while a waiter wants it, and vanish
        // once both are done.
        let hold = Arc::new(Notify::new());
        let a = {
            let (lock, hold) = (Arc::clone(&lock), Arc::clone(&hold));
            tokio::spawn(async move {
                let _g = lock.acquire_within("shared", LONG).await.unwrap();
                hold.notified().await;
            })
        };
        tokio::task::yield_now().await;
        let b = {
            let lock = Arc::clone(&lock);
            tokio::spawn(async move {
                let _g = lock.acquire_within("shared", LONG).await.unwrap();
            })
        };
        tokio::task::yield_now().await;
        assert_eq!(lock.live_keys(), 1, "a contended key stays live");

        hold.notify_one();
        a.await.unwrap();
        b.await.unwrap();
        assert_eq!(
            lock.live_keys(),
            0,
            "the entry must be reclaimed once nobody holds or wants it"
        );
    }
}
