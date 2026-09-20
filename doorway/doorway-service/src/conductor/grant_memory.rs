//! What the chaperone has already granted — so it grants once per device, not
//! once per page load.
//!
//! # Why this exists
//!
//! `POST /hc/connect` calls `grant_zome_call_capability` once per role cell.
//! A grant is a chain write that nothing revokes, and the conductor re-reads
//! **every** grant of the matching access class on **every** zome call
//! (`DhtStoreRead::valid_cap_grants` — load-all-then-filter). At the
//! 11 619 / 15 768 / 18 516 grants measured on matthew and adam on 2026-09-19
//! that cost ~47 000 SQL queries and 4–9 s per call, which is what held those
//! peers' admission permits for their full 60 s timeout.
//!
//! The browser used to mint a FRESH keypair and cap secret on every
//! `connectViaChaperone`, so every page load of every hosted human authored
//! three more permanent grants on the conductors the two public doorways front.
//! That is the shape that fits the measurement, and it is unbounded.
//!
//! # The decision this encodes
//!
//! **A browser/device is a relationship: one witnessed trust act, reused.**
//! The client now persists its signing keypair and cap secret per (doorway
//! origin, agent) and presents the SAME ones on every later connect; this
//! memory lets the doorway recognise them and skip the grant.
//!
//! Revoke-on-session-end was considered and rejected: a deleted grant still
//! pays its per-row queries until the fork's lookup is indexed (deletion only
//! removes the step-3 entry fetch), and grant + delete per session *doubles*
//! the writes on every hosted human's chain.
//!
//! # What is remembered, and what is not
//!
//! Only a SHA-256 fingerprint of
//! `(conductor, cell dna, cell agent, signing public key, cap secret)`.
//! The cap secret is a shared secret — it is hashed, never held, and never
//! logged. The signing key is public by construction. The fingerprint is
//! one-way, so this memory leaks nothing if it is dumped.
//!
//! # Why in-process, and what a restart costs
//!
//! In-process is the honest floor: a doorway restart then costs **at most one
//! grant per device per role cell**, not one per page load. That is a bounded,
//! rare cost against the unbounded per-session one being removed. Promoting
//! this to the doorway's MongoDB-backed records is a later, strictly-additive
//! step — it would make a restart free, and nothing in this module's API
//! assumes memory residence.
//!
//! # Concurrency: the grant is serialized PER DEVICE
//!
//! `decide` and `record` straddle a conductor round trip, and the browser now
//! presents the SAME key material on every connect — so two tabs opening at
//! once carry the SAME fingerprint into that window. Without serialization both
//! would observe `Grant` and both would author a chain write, which is the
//! defect this memory exists to remove.
//!
//! [`GrantMemory::lease`] takes a per-fingerprint async lock
//! ([`crate::keyed_lock`]) that the caller holds across `decide` → grant →
//! `record`. Distinct devices never contend, so a wedged conductor for one
//! human cannot stall another's connect. The wait is bounded by the crate's
//! per-conductor-call deadline: on expiry the caller proceeds UNSERIALIZED,
//! which is exactly the pre-lock behaviour — a possible duplicate grant, never
//! a failed connect and never a skipped grant that did not land.
//!
//! # The self-correcting edge
//!
//! If the doorway remembers a grant the CONDUCTOR has lost (reinstall, or a
//! database restored from an older snapshot), the client's zome calls come back
//! unauthorized. The client discards its stored credential and reconnects ONCE
//! with a fresh keypair; that keypair is unknown here, so it is granted, and the
//! human is healed in one round trip. No loop on either side.

use sha2::{Digest, Sha256};
use std::collections::{HashSet, VecDeque};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use crate::keyed_lock::{KeyedGuard, KeyedLock};

/// How many distinct (device, cell) grants are remembered.
///
/// A fingerprint is 64 hex bytes; 100 000 of them is ~13 MB with the eviction
/// queue — cheap next to the cost of a needless grant, which is permanent and
/// taxes every later zome call on that chain. Overflow evicts the OLDEST
/// fingerprint, so the worst case is one extra grant for a device that has not
/// connected in a very long time.
pub const DEFAULT_CAPACITY: usize = 100_000;

/// What the chaperone will do about one cell's capability grant.
///
/// The decision is separated from its enactment so it can be asserted without
/// a conductor — the same split `elohim-storage`'s closed-chain fence makes,
/// and for the same reason: the irreversible half is a chain write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrantDecision {
    /// This exact (conductor, cell, signing key, cap secret) was already
    /// granted. **Nothing is authored on the conductor.**
    Skip,
    /// Never seen. Grant once, then record.
    Grant,
}

#[derive(Debug, Default)]
struct Inner {
    seen: HashSet<String>,
    /// Insertion order, for bounded eviction.
    order: VecDeque<String>,
}

/// The chaperone's bounded memory of grants already made.
///
/// Constructed with nothing (or a capacity), so every test drives a real one
/// rather than a mock.
#[derive(Debug)]
pub struct GrantMemory {
    inner: Mutex<Inner>,
    capacity: usize,
    /// One async lock per fingerprint, so a check-then-act pair on ONE device
    /// cannot interleave with itself. See the module header.
    leases: KeyedLock,
}

impl Default for GrantMemory {
    fn default() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }
}

impl GrantMemory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: Mutex::new(Inner::default()),
            capacity: capacity.max(1),
            leases: KeyedLock::new(),
        }
    }

    /// The one-way identity of a single grant.
    ///
    /// Every component is load-bearing:
    /// * `conductor_id` — a grant lives on one conductor's chain;
    /// * `dna_hash` + `cell_agent` — the cell it was granted on;
    /// * `signing_key` — the `Assigned` grant's only assignee;
    /// * `cap_secret` — the `Assigned` grant's secret. A client that reuses its
    ///   keypair but rolls its cap secret does NOT match the old grant, so the
    ///   secret must be part of the identity or we would skip a grant the
    ///   conductor will then refuse.
    ///
    /// Fields are length-prefixed so no concatenation of two of them can
    /// collide with a different split.
    pub fn fingerprint(
        conductor_id: &str,
        dna_hash: &[u8],
        cell_agent: &[u8],
        signing_key: &[u8],
        cap_secret: &[u8],
    ) -> String {
        let mut hasher = Sha256::new();
        for part in [
            conductor_id.as_bytes(),
            dna_hash,
            cell_agent,
            signing_key,
            cap_secret,
        ] {
            hasher.update((part.len() as u64).to_be_bytes());
            hasher.update(part);
        }
        hex::encode(hasher.finalize())
    }

    /// Take the per-device lease that makes `decide` → grant → `record` atomic
    /// for THIS fingerprint. Hold the returned guard across all three.
    ///
    /// `None` means the previous holder outlived `deadline`; the caller then
    /// proceeds unserialized — the pre-lock behaviour — rather than failing the
    /// connect or skipping a grant that may never have landed. Distinct
    /// fingerprints never wait on each other.
    pub async fn lease(&self, fingerprint: &str, deadline: Duration) -> Option<KeyedGuard<'_>> {
        self.leases.acquire_within(fingerprint, deadline).await
    }

    /// How many device leases are live. Diagnostics and tests — must return to
    /// zero once every in-flight connect finishes.
    pub fn live_leases(&self) -> usize {
        self.leases.live_keys()
    }

    /// Grant, or skip? Pure read — [`Self::record`] is what remembers.
    ///
    /// Only meaningful under the matching [`Self::lease`]: without it, a
    /// concurrent caller with the same fingerprint can read the same `Grant`.
    ///
    /// Read and record are deliberately separate: a grant that FAILS (a missing
    /// or disabled cell) must not be remembered, or the next connect would skip
    /// a grant that was never made.
    pub fn decide(&self, fingerprint: &str) -> GrantDecision {
        let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if inner.seen.contains(fingerprint) {
            GrantDecision::Skip
        } else {
            GrantDecision::Grant
        }
    }

    /// Remember a grant that actually landed on the chain.
    ///
    /// Idempotent: recording the same fingerprint twice is one entry, which is
    /// what makes the client's 502/503 retry loop mint at most one grant.
    pub fn record(&self, fingerprint: String) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if !inner.seen.insert(fingerprint.clone()) {
            return;
        }
        inner.order.push_back(fingerprint);
        while inner.order.len() > self.capacity {
            if let Some(evicted) = inner.order.pop_front() {
                inner.seen.remove(&evicted);
            }
        }
    }

    /// How many grants are currently remembered (diagnostics and tests).
    pub fn len(&self) -> usize {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .seen
            .len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// The process-wide memory. Skipping is only skipping if every request shares one.
pub fn grant_memory() -> &'static GrantMemory {
    static MEMORY: OnceLock<GrantMemory> = OnceLock::new();
    MEMORY.get_or_init(GrantMemory::new)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DNA: &[u8] = b"dna-hash-bytes";
    const AGENT: &[u8] = b"cell-agent-bytes";
    const KEY: &[u8] = b"browser-signing-pubkey";
    const SECRET: &[u8] = b"browser-cap-secret";

    fn fp() -> String {
        GrantMemory::fingerprint("conductor-0", DNA, AGENT, KEY, SECRET)
    }

    /// GRANT WHEN UNKNOWN. A device the doorway has never seen must be granted
    /// — that is the one witnessed trust act.
    #[test]
    fn an_unknown_device_is_granted() {
        let memory = GrantMemory::new();
        assert_eq!(memory.decide(&fp()), GrantDecision::Grant);
    }

    /// SKIP WHEN KNOWN. THE INVARIANT. The second page load of the same browser
    /// presents the same persisted keypair and cap secret, and must author
    /// nothing — this is the 15 000 rows.
    #[test]
    fn a_returning_device_is_skipped() {
        let memory = GrantMemory::new();
        memory.record(fp());
        assert_eq!(memory.decide(&fp()), GrantDecision::Skip);
    }

    /// IDEMPOTENT UNDER RETRY. The browser retries `/hc/connect` up to three
    /// times on 502/503 with the SAME key material. Those retries must not
    /// multiply grants.
    #[test]
    fn the_client_retry_loop_grants_at_most_once() {
        let memory = GrantMemory::new();

        let mut grants = 0;
        for _attempt in 0..4 {
            if memory.decide(&fp()) == GrantDecision::Grant {
                grants += 1;
                memory.record(fp());
            }
        }
        assert_eq!(grants, 1, "three retries must not author three grants");
        assert_eq!(memory.len(), 1);
    }

    /// A FAILED grant must NOT be recorded, or the retry that follows would
    /// skip a grant that never landed and the human's calls would 401 forever.
    #[test]
    fn a_grant_that_did_not_land_is_not_remembered() {
        let memory = GrantMemory::new();
        // decide() is a pure read; nothing is remembered until record().
        assert_eq!(memory.decide(&fp()), GrantDecision::Grant);
        assert!(memory.is_empty());
        assert_eq!(
            memory.decide(&fp()),
            GrantDecision::Grant,
            "a failed grant must be retried, not skipped"
        );
    }

    /// Every component of the identity must separate grants. Change any one and
    /// the old grant does not apply, so skipping would hand the human a
    /// capability the conductor refuses.
    #[test]
    fn each_component_of_the_identity_separates_grants() {
        let memory = GrantMemory::new();
        memory.record(fp());

        for (what, other) in [
            (
                "a different conductor",
                GrantMemory::fingerprint("conductor-1", DNA, AGENT, KEY, SECRET),
            ),
            (
                "a different cell dna",
                GrantMemory::fingerprint("conductor-0", b"other-dna", AGENT, KEY, SECRET),
            ),
            (
                "a different cell agent",
                GrantMemory::fingerprint("conductor-0", DNA, b"other-agent", KEY, SECRET),
            ),
            (
                "a different signing key (a NEW device, or the client's heal)",
                GrantMemory::fingerprint("conductor-0", DNA, AGENT, b"other-key", SECRET),
            ),
            (
                "a rolled cap secret — the Assigned grant is keyed on it",
                GrantMemory::fingerprint("conductor-0", DNA, AGENT, KEY, b"other-secret"),
            ),
        ] {
            assert_eq!(
                memory.decide(&other),
                GrantDecision::Grant,
                "{what} must be granted, not skipped"
            );
        }
    }

    /// Length-prefixing: no re-split of the same bytes can collide.
    #[test]
    fn field_boundaries_cannot_be_shifted() {
        assert_ne!(
            GrantMemory::fingerprint("c", b"ab", b"c", KEY, SECRET),
            GrantMemory::fingerprint("c", b"a", b"bc", KEY, SECRET),
        );
    }

    /// The fingerprint is one-way and carries no secret in the clear.
    #[test]
    fn the_fingerprint_reveals_no_secret() {
        let printed = fp();
        assert_eq!(printed.len(), 64, "sha256 hex");
        assert!(printed.chars().all(|c| c.is_ascii_hexdigit()));
        assert!(!printed.contains("cap-secret"));
        assert!(!printed.contains(&hex::encode(SECRET)));
    }

    /// Bounded: a long-running doorway cannot grow this without limit. Eviction
    /// is oldest-first, so the cost of overflow is at most one extra grant for a
    /// device that has been away the longest.
    #[test]
    fn memory_is_bounded_and_evicts_the_oldest() {
        let memory = GrantMemory::with_capacity(3);
        for i in 0..5u8 {
            memory.record(GrantMemory::fingerprint("c", &[i], AGENT, KEY, SECRET));
        }
        assert_eq!(memory.len(), 3);
        assert_eq!(
            memory.decide(&GrantMemory::fingerprint("c", &[0], AGENT, KEY, SECRET)),
            GrantDecision::Grant,
            "the oldest entry is evicted first"
        );
        assert_eq!(
            memory.decide(&GrantMemory::fingerprint("c", &[4], AGENT, KEY, SECRET)),
            GrantDecision::Skip,
            "the newest entry is retained"
        );
    }
}
