//! Inventory broadcaster — snapshot timer + delta emitter + sequence allocator.
//!
//! Responsibilities:
//! - Periodically (per archetype-tunable cadence) compute the local set of
//!   hosted blob hashes and publish a `BlobInventorySnapshot` to
//!   `elohim/inventory/blob`.
//! - On local blob add/remove events, emit a `BlobInventoryDelta` (with a
//!   small batching window to coalesce bursts).
//! - Allocate per-this-peer monotonic sequence numbers for both message types.
//!
//! Source of truth for "what blobs do I host": the local blob store; the
//! enumeration is delegated to `LocalInventory` so it can be mocked in tests.

use crate::p2p::inventory_gossip::{BlobInventoryDelta, BlobInventorySnapshot};
use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Trait for enumerating the local blob inventory. Production: walks the
/// blob store. Tests: returns a fixed set.
pub trait LocalInventory: Send + Sync {
    fn current_hashes(&self) -> Vec<String>;
}

/// T22 review fix #1: wraps a pre-fetched list of hashes for `LocalInventory`.
///
/// Production callers fetch via `BlobStore::list_hashes()` first and pass the
/// result here so I/O failure can return early *before* ever entering
/// `build_snapshot`. Previously `BlobStoreInventory` swallowed the error and
/// returned an empty `Vec`, which caused `apply_snapshot` on every remote peer
/// to evict this peer's inventory entries during a transient I/O blip.
pub struct StaticInventory(Vec<String>);

impl StaticInventory {
    pub fn new(hashes: Vec<String>) -> Self {
        Self(hashes)
    }
}

impl LocalInventory for StaticInventory {
    fn current_hashes(&self) -> Vec<String> {
        self.0.clone()
    }
}

/// Per-this-peer monotonic sequence allocator.
#[derive(Debug, Clone, Default)]
pub struct SequenceAllocator {
    inner: Arc<AtomicU64>,
}

impl SequenceAllocator {
    pub fn new(initial: u64) -> Self {
        Self {
            inner: Arc::new(AtomicU64::new(initial)),
        }
    }

    /// Allocate the next sequence number.
    pub fn next(&self) -> u64 {
        self.inner.fetch_add(1, Ordering::SeqCst) + 1
    }

    pub fn current(&self) -> u64 {
        self.inner.load(Ordering::SeqCst)
    }
}

/// Build a snapshot for the given peer id with the given inventory.
pub fn build_snapshot<I: LocalInventory>(
    peer_id: &str,
    inventory: &I,
    seq: &SequenceAllocator,
    now_micros: i64,
) -> BlobInventorySnapshot {
    BlobInventorySnapshot {
        peer_id: peer_id.to_string(),
        hashes: inventory.current_hashes(),
        snapshot_at: now_micros,
        sequence: seq.next(),
        signature: vec![0x00], // Stage 1 structural non-empty
    }
}

/// Build a delta for the given add/remove batch.
pub fn build_delta(
    peer_id: &str,
    added: Vec<String>,
    removed: Vec<String>,
    seq: &SequenceAllocator,
    now_micros: i64,
) -> BlobInventoryDelta {
    BlobInventoryDelta {
        peer_id: peer_id.to_string(),
        added,
        removed,
        emitted_at: now_micros,
        sequence: seq.next(),
        signature: vec![0x00], // Stage 1 structural non-empty
    }
}

/// Compute the resolved cadence for this peer's archetype, honoring the
/// 4-layer override pattern (archetype default ← policy.toml ← env/CLI ←
/// admin trigger). Returns `None` to mean "broadcasting disabled."
pub fn resolved_cadence(archetype: Option<&str>, config_override: Option<u64>) -> Option<u64> {
    if let Some(seconds) = config_override {
        if seconds == 0 {
            return None; // explicit "0" means disabled
        }
        return Some(seconds);
    }
    crate::config::inventory_broadcast_seconds_default(archetype)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockInventory(Vec<String>);
    impl LocalInventory for MockInventory {
        fn current_hashes(&self) -> Vec<String> {
            self.0.clone()
        }
    }

    #[test]
    fn sequence_allocator_increments() {
        let alloc = SequenceAllocator::new(0);
        assert_eq!(alloc.next(), 1);
        assert_eq!(alloc.next(), 2);
        assert_eq!(alloc.next(), 3);
        assert_eq!(alloc.current(), 3);
    }

    #[test]
    fn snapshot_includes_inventory_and_advances_sequence() {
        let inv = MockInventory(vec!["a".repeat(64), "b".repeat(64)]);
        let alloc = SequenceAllocator::new(10);
        let snapshot = build_snapshot("12D3KooWtest", &inv, &alloc, 1_700_000_000_000_000);

        assert_eq!(snapshot.peer_id, "12D3KooWtest");
        assert_eq!(snapshot.hashes.len(), 2);
        assert_eq!(snapshot.sequence, 11);
        assert_eq!(snapshot.signature, vec![0x00]);
    }

    #[test]
    fn delta_carries_added_and_removed() {
        let alloc = SequenceAllocator::new(0);
        let delta = build_delta(
            "12D3KooWtest",
            vec!["a".repeat(64)],
            vec!["b".repeat(64)],
            &alloc,
            1_700_000_001_000_000,
        );

        assert_eq!(delta.added.len(), 1);
        assert_eq!(delta.removed.len(), 1);
        assert_eq!(delta.sequence, 1);
    }

    #[test]
    fn resolved_cadence_uses_override_when_present() {
        assert_eq!(resolved_cadence(Some("node"), Some(120)), Some(120));
    }

    #[test]
    fn resolved_cadence_falls_back_to_archetype_default() {
        assert_eq!(resolved_cadence(Some("node"), None), Some(60));
        assert_eq!(resolved_cadence(Some("desktop"), None), Some(300));
        assert_eq!(resolved_cadence(Some("mobile"), None), None);
        assert_eq!(resolved_cadence(Some("steward"), None), Some(60));
    }

    #[test]
    fn resolved_cadence_zero_override_disables() {
        assert_eq!(resolved_cadence(Some("node"), Some(0)), None);
    }

    /// T22 review fix #1: `StaticInventory` round-trips its provided hashes
    /// through `LocalInventory::current_hashes`. The production caller fetches
    /// hashes via `BlobStore::list_hashes()` *before* constructing this and
    /// returns early on I/O failure; the adapter itself is intentionally
    /// dumb so the failure path can never reach `build_snapshot`. BlobStore's
    /// own `list_hashes` tests cover the I/O behavior.
    #[test]
    fn static_inventory_returns_provided_hashes() {
        let inv = StaticInventory::new(vec!["aaa".to_string(), "bbb".to_string()]);
        let hashes = inv.current_hashes();
        assert_eq!(hashes, vec!["aaa".to_string(), "bbb".to_string()]);
    }

    /// T22 review fix #4: unknown archetype strings (typos, future archetypes
    /// not yet known to this build) fall back to the conservative `node`
    /// cadence and emit a `tracing::warn!` so the misconfiguration surfaces.
    /// We assert the return value here; the warn is exercised at runtime.
    #[test]
    fn unknown_archetype_logs_warn_and_defaults_to_node() {
        // Misspelled "nod" (missing 'e') and a plausible future archetype "tablet".
        assert_eq!(resolved_cadence(Some("tablet"), None), Some(60));
        assert_eq!(resolved_cadence(Some("nod"), None), Some(60));
    }
}

/// Filesystem-vs-gossip parity report.
///
/// Populated by `compute_parity` and served at
/// `GET /api/v1/diagnostics/inventory-parity`.  Defends against the failure
/// mode where gossip runs cleanly but bytes never replicate — inventory lists
/// diverge before blob mobility does.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct ParityReport {
    /// Hashes that were included in the last gossiped snapshot but are absent
    /// from the local filesystem.  Non-empty means gossip over-reports custody.
    pub gossiped_but_missing: Vec<String>,
    /// Hashes present on the local filesystem that were not in the last gossiped
    /// snapshot.  Non-empty means gossip under-reports custody.
    pub local_but_not_gossiped: Vec<String>,
    /// Number of blobs actually on the local filesystem at time of check.
    pub filesystem_count: usize,
    /// Number of hashes in the last gossiped snapshot (0 when no snapshot has
    /// been published yet — Stage 1 baseline).
    pub gossiped_count: usize,
    /// ISO-8601 timestamp of when this report was generated.
    pub checked_at: String,
}

/// Compute a `ParityReport` by diffing `local_store.current_hashes()` against
/// `last_gossiped`.
///
/// Uses `HashSet::difference` for O(n) comparison; the returned vecs are
/// sorted for deterministic output.
pub fn compute_parity<I: LocalInventory>(
    local_store: &I,
    last_gossiped: &[String],
    now_iso: &str,
) -> ParityReport {
    let local: HashSet<String> = local_store.current_hashes().into_iter().collect();
    let gossiped: HashSet<String> = last_gossiped.iter().cloned().collect();

    let mut gossiped_but_missing: Vec<String> = gossiped.difference(&local).cloned().collect();
    gossiped_but_missing.sort();

    let mut local_but_not_gossiped: Vec<String> = local.difference(&gossiped).cloned().collect();
    local_but_not_gossiped.sort();

    ParityReport {
        filesystem_count: local.len(),
        gossiped_count: gossiped.len(),
        gossiped_but_missing,
        local_but_not_gossiped,
        checked_at: now_iso.to_string(),
    }
}

#[cfg(test)]
mod parity_tests {
    use super::*;

    struct MockInventory(Vec<String>);
    impl LocalInventory for MockInventory {
        fn current_hashes(&self) -> Vec<String> {
            self.0.clone()
        }
    }

    #[test]
    fn parity_clean_when_sets_match() {
        let hashes = vec!["aaa".to_string(), "bbb".to_string()];
        let inv = MockInventory(hashes.clone());
        let report = compute_parity(&inv, &hashes, "2026-05-02T00:00:00Z");

        assert!(report.gossiped_but_missing.is_empty());
        assert!(report.local_but_not_gossiped.is_empty());
        assert_eq!(report.filesystem_count, 2);
        assert_eq!(report.gossiped_count, 2);
        assert_eq!(report.checked_at, "2026-05-02T00:00:00Z");
    }

    #[test]
    fn parity_detects_gossiped_but_missing() {
        // Gossip claims "ccc" is hosted but filesystem only has "aaa" and "bbb"
        let local = MockInventory(vec!["aaa".to_string(), "bbb".to_string()]);
        let gossiped = vec!["aaa".to_string(), "bbb".to_string(), "ccc".to_string()];
        let report = compute_parity(&local, &gossiped, "2026-05-02T00:00:00Z");

        assert_eq!(report.gossiped_but_missing, vec!["ccc".to_string()]);
        assert!(report.local_but_not_gossiped.is_empty());
        assert_eq!(report.filesystem_count, 2);
        assert_eq!(report.gossiped_count, 3);
    }

    #[test]
    fn parity_detects_local_but_not_gossiped() {
        // Filesystem has "ddd" that was never included in the gossip snapshot
        let local = MockInventory(vec!["aaa".to_string(), "ddd".to_string()]);
        let gossiped = vec!["aaa".to_string()];
        let report = compute_parity(&local, &gossiped, "2026-05-02T00:00:00Z");

        assert!(report.gossiped_but_missing.is_empty());
        assert_eq!(report.local_but_not_gossiped, vec!["ddd".to_string()]);
        assert_eq!(report.filesystem_count, 2);
        assert_eq!(report.gossiped_count, 1);
    }
}
