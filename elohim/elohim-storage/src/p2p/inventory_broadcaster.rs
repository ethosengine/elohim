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
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Trait for enumerating the local blob inventory. Production: walks the
/// blob store. Tests: returns a fixed set.
pub trait LocalInventory: Send + Sync {
    fn current_hashes(&self) -> Vec<String>;
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
}
