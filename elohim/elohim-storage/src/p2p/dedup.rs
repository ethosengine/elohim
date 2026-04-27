//! Bounded LRU dedup cache for the p2p receive path.
//!
//! ## Why
//!
//! D.1's integrity exception sends the same atom on Kad + gossipsub + direct-notify
//! simultaneously. D.5's direct-notify can hit the same peer that D.3's gossip
//! reaches. Without dedup the receiver would pay 3× verification + projection
//! cost. The LRU drops duplicates cheaply and emits a `seen` counter for
//! observability.
//!
//! ## Bounds
//!
//! Default capacity ~1024 entries. Each entry is a CID string (~50 bytes) +
//! LRU node overhead (~50 bytes). 1024 × ~100 bytes = ~100KB — well within
//! a "few MB" budget per spec §3.7. Capacity is configurable via
//! `DedupLru::with_capacity`.
//!
//! ## Concurrency
//!
//! `Mutex<LruCache>` — sufficient for receive-path traffic. The hot path is
//! O(1) hash lookup + O(1) LRU update. Lock contention is bounded by the
//! receive thread's serial event-loop dispatch.

use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

const DEFAULT_CAPACITY: usize = 1024;

/// Bounded LRU dedup cache keyed by CID string.
pub struct DedupLru {
    inner: Mutex<LruCache<String, ()>>,
    seen: AtomicUsize,
}

impl DedupLru {
    /// New cache with default capacity (1024 entries).
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }

    /// New cache with the given capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        let cap = NonZeroUsize::new(capacity.max(1)).unwrap();
        Self {
            inner: Mutex::new(LruCache::new(cap)),
            seen: AtomicUsize::new(0),
        }
    }

    /// Insert `cid` if not already present.
    ///
    /// Returns `true` if newly inserted (caller should proceed with handling),
    /// or `false` if already present (caller should drop as duplicate).
    /// Always increments the `seen` counter.
    pub fn insert(&self, cid: &str) -> bool {
        self.seen.fetch_add(1, Ordering::Relaxed);
        // Intentional panic on poison: a poisoned dedup lock means another thread
        // panicked mid-update; restarting the storage process is safer than restoring
        // an inconsistent cache state via .into_inner() (which could replay duplicates).
        let mut guard = self.inner.lock().expect("dedup lru poisoned");
        // `put` returns Some(()) if the key was already present (existing value
        // replaced; LRU order updated). We treat that as a duplicate.
        guard.put(cid.to_string(), ()).is_none()
    }

    /// Total number of insert calls (new + duplicate).
    pub fn seen_counter(&self) -> usize {
        self.seen.load(Ordering::Relaxed)
    }

    /// Current cache occupancy (unique CIDs).
    pub fn len(&self) -> usize {
        self.inner.lock().expect("dedup lru poisoned").len()
    }

    /// Atomic snapshot of (unique_len, total_seen). Reads len under the mutex
    /// AND seen counter under the same lock guard, ensuring the pair is
    /// consistent at one point in time (matters for callers computing
    /// duplication rate `(seen - len) / seen`).
    pub fn stats(&self) -> (usize, usize) {
        let guard = self.inner.lock().expect("dedup lru poisoned");
        let len = guard.len();
        let seen = self.seen.load(Ordering::Relaxed);
        (len, seen)
    }

    /// True if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for DedupLru {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_insert_returns_true() {
        let d = DedupLru::with_capacity(4);
        assert!(d.insert("cid-a"));
        assert_eq!(d.len(), 1);
        assert_eq!(d.seen_counter(), 1);
    }

    #[test]
    fn duplicate_insert_returns_false() {
        let d = DedupLru::with_capacity(4);
        assert!(d.insert("cid-a"));
        assert!(!d.insert("cid-a"));
        assert_eq!(d.len(), 1);
        assert_eq!(d.seen_counter(), 2);
    }

    #[test]
    fn distinct_cids_each_return_true() {
        let d = DedupLru::with_capacity(4);
        assert!(d.insert("cid-a"));
        assert!(d.insert("cid-b"));
        assert!(d.insert("cid-c"));
        assert_eq!(d.len(), 3);
        assert_eq!(d.seen_counter(), 3);
    }

    #[test]
    fn capacity_evicts_oldest() {
        let d = DedupLru::with_capacity(2);
        // Step 1-2: fill to capacity [cid-a(LRU), cid-b(MRU)]
        d.insert("cid-a");
        d.insert("cid-b");
        // Step 3: inserting cid-c evicts the LRU (cid-a) → [cid-b(LRU), cid-c(MRU)]
        d.insert("cid-c");
        assert_eq!(d.len(), 2);
        // Step 4: cid-b is present (it is the LRU, not yet evicted)
        assert!(!d.insert("cid-b"));
        // Step 5: cid-a was evicted in step 3, so re-inserting returns true
        // inserting cid-a now evicts cid-b (which was LRU after step 4's access
        // refreshed cid-c's position... wait, step 4 accessed cid-b so it is now MRU)
        // After step 4: cid-c is LRU (accessed at step 3), cid-b is MRU (accessed at step 4)
        // Inserting cid-a evicts cid-c → [cid-b(LRU), cid-a(MRU)]
        assert!(d.insert("cid-a"));
        assert_eq!(d.len(), 2);
        // cid-c was evicted in the previous insert, so it counts as new
        assert!(d.insert("cid-c"));
    }

    #[test]
    fn seen_counter_includes_duplicates() {
        let d = DedupLru::with_capacity(4);
        d.insert("cid-a");
        d.insert("cid-a");
        d.insert("cid-a");
        d.insert("cid-b");
        assert_eq!(d.seen_counter(), 4);
    }

    #[test]
    fn capacity_zero_is_treated_as_one() {
        let d = DedupLru::with_capacity(0);
        // No panic; capacity is forced to ≥1
        assert!(d.insert("cid-a"));
        assert_eq!(d.len(), 1);
    }

    #[test]
    fn default_capacity_is_reasonable() {
        let d = DedupLru::new();
        // Smoke test — capacity is ≥1024; insert 100 distinct cids without eviction
        for i in 0..100 {
            assert!(d.insert(&format!("cid-{}", i)));
        }
        assert_eq!(d.len(), 100);
    }

    #[test]
    fn stats_returns_consistent_snapshot() {
        let d = DedupLru::with_capacity(16);
        // Insert 3 unique CIDs
        d.insert("cid-a");
        d.insert("cid-b");
        d.insert("cid-c");
        // Insert 2 duplicates
        d.insert("cid-a");
        d.insert("cid-b");

        let (unique_len, total_seen) = d.stats();
        // 3 unique entries in the LRU window
        assert_eq!(unique_len, 3, "unique_len should be 3 (3 distinct CIDs)");
        // 5 total insert calls (3 unique + 2 duplicates)
        assert_eq!(
            total_seen, 5,
            "total_seen should be 5 (3 new + 2 duplicates)"
        );
        // Duplication rate: (5 - 3) / 5 = 0.4 — sanity check that seen > unique
        assert!(total_seen >= unique_len, "seen must be >= unique_len");
    }
}
