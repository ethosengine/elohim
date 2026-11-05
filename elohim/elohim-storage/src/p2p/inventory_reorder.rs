//! Receive-side reorder window for paged inventory deltas.
//!
//! A large announcement is N contiguous signed delta pages
//! (`inventory_broadcaster::build_delta_pages`), carried on BOTH planes in dual
//! mode. `apply_delta` is strictly in-order: a page arriving ahead of its
//! predecessor reads as a gap, and dropping it dropped every later page with it —
//! measured 2026-08-29 (household mesh): a neighbour's inventory sat at 184 of 2046
//! (~9 %). See backlog `inventory-refresh-pages-dropped-as-gaps`. A gap the window
//! cannot close is healed by a recon session with the holder.
//!
//! This buffer holds an early, already ADMITTED (signature-verified) page until the
//! cursor reaches it, then hands it back to be applied in order. It is keyed by publisher peer id, bounded per
//! peer (the lowest sequences are kept — they apply first — and the highest
//! is evicted on overflow), and process-global because the cursor it serves
//! (`peer_inventory_cursor`) is one row per peer shared by both planes.

use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;

use crate::p2p::inventory_gossip::InventoryEnvelope;

/// Pages held per publisher. Two full refreshes on the measured corpus.
pub const MAX_PENDING_PER_PEER: usize = 160;

#[derive(Debug, Default)]
pub struct InventoryReorder {
    pending: HashMap<String, BTreeMap<i64, InventoryEnvelope>>,
}

/// What `stash` did with an early page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StashOutcome {
    Buffered,
    /// Already held at that sequence (the other plane's copy).
    Duplicate,
    /// Buffer full and this page was the highest sequence — dropped.
    Overflow,
}

impl InventoryReorder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Hold an out-of-order page. bounded-work: at most
    /// [`MAX_PENDING_PER_PEER`] pages per publisher; on overflow the HIGHEST
    /// sequence is evicted (or the new page refused if it is the highest).
    pub fn stash(&mut self, delta: InventoryEnvelope) -> StashOutcome {
        let seq = delta.body.sequence as i64;
        let pages = self.pending.entry(delta.author_peer.clone()).or_default();
        if pages.contains_key(&seq) {
            return StashOutcome::Duplicate;
        }
        if pages.len() >= MAX_PENDING_PER_PEER {
            let highest = *pages.keys().next_back().expect("non-empty");
            if seq >= highest {
                return StashOutcome::Overflow;
            }
            pages.remove(&highest);
        }
        pages.insert(seq, delta);
        StashOutcome::Buffered
    }

    /// The page at exactly `next_seq` for `peer`, if held. Pages below
    /// `next_seq` are stale (already applied via the other plane) and are
    /// discarded as they are passed.
    pub fn take_next(&mut self, peer: &str, next_seq: i64) -> Option<InventoryEnvelope> {
        let pages = self.pending.get_mut(peer)?;
        while let Some((&lowest, _)) = pages.iter().next() {
            if lowest < next_seq {
                pages.remove(&lowest);
            } else {
                break;
            }
        }
        let page = pages.remove(&next_seq);
        if pages.is_empty() {
            self.pending.remove(peer);
        }
        page
    }

    pub fn pending_for(&self, peer: &str) -> usize {
        self.pending.get(peer).map_or(0, |p| p.len())
    }
}

/// The process-wide buffer (one node per process; keyed by publisher).
pub static INVENTORY_REORDER: std::sync::LazyLock<Mutex<InventoryReorder>> =
    std::sync::LazyLock::new(|| Mutex::new(InventoryReorder::new()));

/// Lock helper that survives a poisoned mutex (a panic elsewhere must not
/// turn every later page into a gap).
pub fn with_reorder<R>(f: impl FnOnce(&mut InventoryReorder) -> R) -> R {
    let mut guard = INVENTORY_REORDER
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    f(&mut guard)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(peer: &str, seq: u64) -> InventoryEnvelope {
        use crate::p2p::inventory_gossip::{BlobAddress, InventoryDelta, InventoryLevel};
        let keypair = elohim_epr::proof::AgentKeypair::from_secret(&[5u8; 32]).unwrap();
        InventoryEnvelope::sign(
            &keypair,
            peer,
            InventoryLevel::Full,
            InventoryDelta {
                added: vec![BlobAddress::from(elohim_epr::address::compute_bytes_cid(
                    &seq.to_le_bytes(),
                ))],
                hints: vec![],
                removed: vec![],
                sequence: seq,
                emitted_at: 0,
            },
        )
    }

    #[test]
    fn early_pages_come_back_in_sequence_order() {
        let mut r = InventoryReorder::new();
        assert_eq!(r.stash(page("p", 12)), StashOutcome::Buffered);
        assert_eq!(r.stash(page("p", 11)), StashOutcome::Buffered);
        assert_eq!(r.stash(page("p", 11)), StashOutcome::Duplicate);
        assert!(r.take_next("p", 10).is_none(), "10 was never held");
        assert_eq!(r.take_next("p", 11).unwrap().body.sequence, 11);
        assert_eq!(r.take_next("p", 12).unwrap().body.sequence, 12);
        assert!(r.take_next("p", 13).is_none());
        assert_eq!(r.pending_for("p"), 0);
    }

    #[test]
    fn stale_pages_below_the_cursor_are_discarded_on_the_way() {
        let mut r = InventoryReorder::new();
        for s in [3, 4, 7] {
            r.stash(page("p", s));
        }
        // Cursor already at 6 (the other plane applied 3..6): 3 and 4 are stale.
        assert!(r.take_next("p", 6).is_none());
        assert_eq!(r.pending_for("p"), 1);
        assert_eq!(r.take_next("p", 7).unwrap().body.sequence, 7);
    }

    #[test]
    fn the_buffer_is_bounded_per_peer_and_keeps_the_lowest_sequences() {
        let mut r = InventoryReorder::new();
        for s in 1..=MAX_PENDING_PER_PEER as u64 {
            assert_eq!(r.stash(page("p", s)), StashOutcome::Buffered);
        }
        // A higher page than everything held is refused.
        assert_eq!(
            r.stash(page("p", MAX_PENDING_PER_PEER as u64 + 5)),
            StashOutcome::Overflow
        );
        // A lower page evicts the current highest.
        r.take_next("p", 1);
        assert_eq!(r.pending_for("p"), MAX_PENDING_PER_PEER - 1);
        assert_eq!(r.stash(page("p", 0)), StashOutcome::Buffered);
        assert_eq!(r.pending_for("p"), MAX_PENDING_PER_PEER);
        // Other peers are independent.
        assert_eq!(r.stash(page("q", 1)), StashOutcome::Buffered);
    }
}
