//! `VerificationMemo` — a record of a verification that already happened, so
//! a later request for the same (subject, evaluator, epoch, lens) can skip
//! re-verifying at full cost.
//!
//! Design: `genesis/docs/superpowers/plans/2026-08-08-head-plane-trust-gradient-program-plan.md`
//! §3 L5. This module ships the `VerificationMemo` type (plus T9's `outcome`
//! field — see below), the `VerificationMemoStore` trait, and
//! [`InMemoryVerificationMemoStore`] (T7's process-lifetime plumbing).
//! `epr_service::authorize_reach_for_human`'s `familiar`-tier arm is T9's
//! wiring: a HIT (fresh, per `config::trust_memo_ttl_seconds`) short-circuits
//! the stewards-times-collectives fan-out and returns the memoized outcome; a
//! MISS runs the fan-out and records the result. Consultation is gated by
//! BOTH a wired store (`TrustGradient::memo` is `Some`) AND
//! `config::trust_memo_reads_enabled()` (default OFF) — `TrustGradient::inert()`
//! still hard-codes `memo: None`, so every OTHER call site (and this one, with
//! the flag off) stays byte-identical to pre-T9 behavior.
//!
//! `invalidate_for_subject` still has no caller — that is T19's FeedbackSignal
//! hook (`EprService::invalidate_verification_memo_for_subject` is the seam
//! it will call). Until T19 lands, TTL expiry and LRU eviction are the only
//! invalidation, which is exactly why reads default off: a memo can outlive a
//! standing change by up to the TTL window with no way to hear about it early.
//!
//! **NO NUMERIC FIELD, EVER** — §4.2's derived-not-stored discipline applies
//! to this memo specifically (the `.epr-meta` rail T12 adds exists to catch
//! `score:`-shaped drift here). `epoch` is a [`crate::trust::snapshot::TrustEpoch`]
//! token (a `String` wrapper), never a bare integer; `depth` is the
//! [`crate::trust::pricer::VerificationDepth`] enum, never a numeric score.

use std::num::NonZeroUsize;
use std::sync::Mutex;

use chrono::{DateTime, Utc};
use lru::LruCache;

use crate::trust::pricer::VerificationDepth;
use crate::trust::snapshot::TrustEpoch;

/// A record of one verification: who did it, when, through which
/// constitutional lens, at what depth, against which epoch and edge set.
///
/// No numeric field appears here by design — see the module doc.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationMemo {
    /// Who verified — the evaluating agent's cid.
    pub verifier_agent_cid: String,
    /// When the verification ran.
    pub verified_at: DateTime<Utc>,
    /// Which lens — the constitutional-manifest CID the verification ran
    /// through (an agent participating in multiple collectives has multiple
    /// standing views, one per lens — brainstorm §4.2).
    pub lens_manifest_cid: String,
    /// The depth actually performed (from `pricer.rs`) — NOT a numeric
    /// score.
    pub depth: VerificationDepth,
    /// The epoch this memo is valid through.
    pub epoch: TrustEpoch,
    /// The edge-set digest this memo attests against.
    pub edge_set_digest: String,
    /// T9: the field T6/T7 shipped this type without — a memo that records
    /// only "a verification happened" cannot let a consumer skip
    /// recomputation, because the whole point of skipping is to avoid
    /// re-deriving the ALLOW/DENY answer. `VerificationOutcome` is a
    /// closed, non-numeric vocabulary (a discriminant plus a `String`
    /// reason on the deny arm — neither is a score), so it doesn't reopen
    /// the "no numeric field, ever" door.
    pub outcome: VerificationOutcome,
}

/// The closed outcome vocabulary a [`VerificationMemo`] carries. Deliberately
/// NOT a `bool`: `Denied` carries the human-readable reason the original
/// verification produced, so a memo hit can return the SAME error message a
/// miss would have, not a generic "cached deny".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerificationOutcome {
    /// The verification allowed the request.
    Authorized,
    /// The verification denied the request, for `reason`.
    Denied { reason: String },
}

/// Stores and retrieves [`VerificationMemo`]s, keyed by (subject, evaluator).
///
/// `TrustGradient::inert()` always passes `memo: None`, so callers that stay
/// on the inert landing shape never consult a store. `authorize_reach_for_human`
/// (T9) is the first live consumer, behind `config::trust_memo_reads_enabled()`.
/// `Send + Sync` because the store is `Arc<dyn VerificationMemoStore>` shared
/// process-lifetime (T7's prerequisite fix to the per-request `EprService`
/// construction).
pub trait VerificationMemoStore: Send + Sync {
    /// Look up a still-valid memo for (subject, evaluator). `None` means "no
    /// memo" — the caller applies its own staleness check (T9's consumer
    /// checks TTL against `verified_at`) and re-verifies at full cost on a
    /// miss or a stale hit, exactly today's (pre-T9, and still flag-off)
    /// behavior.
    fn get(&self, subject_cid: &str, evaluator_agent_cid: &str) -> Option<VerificationMemo>;

    /// Record a fresh verification result.
    fn put(&self, subject_cid: &str, evaluator_agent_cid: &str, memo: VerificationMemo);

    /// Drop every memo naming `subject_cid`. The primary invalidation path
    /// is the FeedbackSignal hook (T19: correct-but-dormant until then) —
    /// TTL eviction, if a store implements one, is a backstop only, never
    /// primary.
    fn invalidate_for_subject(&self, subject_cid: &str);
}

/// Default number of subject/evaluator pairs retained process-wide. At roughly
/// a few hundred bytes per memo, 4,096 entries keeps the cache in the low-MiB
/// range; eviction is fail-safe because a miss causes full re-verification.
pub const DEFAULT_VERIFICATION_MEMO_CAPACITY: usize = 4_096;

/// The minimal in-memory [`VerificationMemoStore`] — a bounded
/// `Mutex<LruCache<...>>`
/// keyed by `(subject_cid, evaluator_agent_cid)`, following the crate's
/// established hot-path cache shape (`p2p::dedup::DedupLru`).
///
/// **T7 plumbing, T9 consumption.** This exists so a store can be `Arc`'d
/// once in `main.rs` and given process lifetime, threaded to every
/// `EprService` construction site (HTTP per-request, libp2p per-request
/// snapshot, iroh process-lifetime instance). [`Self::get`] / [`Self::put`]
/// are called from `epr_service::authorize_reach_for_human`'s `familiar`-tier
/// arm — but only when `config::trust_memo_reads_enabled()` is true (default
/// false) AND the caller's `TrustGradient` carries this store; every other
/// path, and every path while the flag is off, still sees `TrustGradient::
/// inert()`'s `memo: None`. T19's `standing_projector` is what eventually
/// makes a cache hit here track a REAL standing change (via
/// `invalidate_for_subject`) rather than only TTL/LRU expiry.
///
/// No persistence and no TTL eviction (memo.rs module doc: TTL is a backstop
/// only). Capacity eviction is active before T9 starts writing on the hot path:
/// losing the least-recently-used memo only restores full verification cost,
/// while allowing unbounded growth would turn evaluator/subject diversity into
/// process-memory pressure.
#[derive(Debug)]
pub struct InMemoryVerificationMemoStore {
    memos: Mutex<LruCache<(String, String), VerificationMemo>>,
}

impl InMemoryVerificationMemoStore {
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_VERIFICATION_MEMO_CAPACITY)
    }

    /// Construct a bounded store. Zero is clamped to one, matching the crate's
    /// other LRU caches; a disabled cache is represented by `memo: None` on
    /// `TrustGradient`, not by a surprising always-miss store.
    pub fn with_capacity(capacity: usize) -> Self {
        let capacity =
            NonZeroUsize::new(capacity.max(1)).expect("capacity.max(1) is always non-zero");
        Self {
            memos: Mutex::new(LruCache::new(capacity)),
        }
    }

    /// Current occupancy, primarily for operational and bound-verification
    /// tests. It can never exceed the configured capacity.
    pub fn len(&self) -> usize {
        self.memos
            .lock()
            .expect("InMemoryVerificationMemoStore mutex poisoned")
            .len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for InMemoryVerificationMemoStore {
    fn default() -> Self {
        Self::new()
    }
}

impl VerificationMemoStore for InMemoryVerificationMemoStore {
    fn get(&self, subject_cid: &str, evaluator_agent_cid: &str) -> Option<VerificationMemo> {
        let key = (subject_cid.to_string(), evaluator_agent_cid.to_string());
        self.memos
            .lock()
            .expect("InMemoryVerificationMemoStore mutex poisoned")
            .get(&key)
            .cloned()
    }

    fn put(&self, subject_cid: &str, evaluator_agent_cid: &str, memo: VerificationMemo) {
        let key = (subject_cid.to_string(), evaluator_agent_cid.to_string());
        self.memos
            .lock()
            .expect("InMemoryVerificationMemoStore mutex poisoned")
            .put(key, memo);
    }

    fn invalidate_for_subject(&self, subject_cid: &str) {
        let mut memos = self
            .memos
            .lock()
            .expect("InMemoryVerificationMemoStore mutex poisoned");
        let keys: Vec<_> = memos
            .iter()
            .filter(|((subject, _), _)| subject == subject_cid)
            .map(|(key, _)| key.clone())
            .collect();
        for key in keys {
            memos.pop(&key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trust::pricer::VerificationDepth;

    fn sample_memo() -> VerificationMemo {
        VerificationMemo {
            verifier_agent_cid: "uhCAk-verifier".to_string(),
            verified_at: DateTime::<Utc>::UNIX_EPOCH,
            lens_manifest_cid: "bafyreilens".to_string(),
            depth: VerificationDepth::FullChain,
            epoch: TrustEpoch("edge-set-token".to_string()),
            edge_set_digest: "sha256-deadbeef".to_string(),
            outcome: VerificationOutcome::Authorized,
        }
    }

    fn sample_denied_memo() -> VerificationMemo {
        VerificationMemo {
            outcome: VerificationOutcome::Denied {
                reason: "No shared collective with content steward".to_string(),
            },
            ..sample_memo()
        }
    }

    #[test]
    fn memo_carries_no_numeric_field_by_construction() {
        // Field-by-field type check: every field on VerificationMemo is
        // String, DateTime, or a fieldless-discriminant enum — none is a
        // bare integer/float. This test exists so that ADDING a numeric
        // field later requires touching (and re-reading) this assertion.
        let memo = sample_memo();
        let _: &String = &memo.verifier_agent_cid;
        let _: &DateTime<Utc> = &memo.verified_at;
        let _: &String = &memo.lens_manifest_cid;
        let _: &VerificationDepth = &memo.depth;
        let _: &TrustEpoch = &memo.epoch;
        let _: &String = &memo.edge_set_digest;
        let _: &VerificationOutcome = &memo.outcome;
        // The Denied arm's payload is a reason STRING, never a score either.
        if let VerificationOutcome::Denied { reason } = &sample_denied_memo().outcome {
            let _: &String = reason;
        } else {
            panic!("sample_denied_memo() must construct the Denied arm");
        }
    }

    #[test]
    fn memo_equality_is_structural() {
        assert_eq!(sample_memo(), sample_memo());
    }

    /// A trivial in-memory store double, proving the trait is object-safe
    /// (`Option<&dyn VerificationMemoStore>` is exactly the shape
    /// `TrustGradient` needs) without shipping a real implementation.
    struct NullStore;

    impl VerificationMemoStore for NullStore {
        fn get(&self, _subject_cid: &str, _evaluator_agent_cid: &str) -> Option<VerificationMemo> {
            None
        }
        fn put(&self, _subject_cid: &str, _evaluator_agent_cid: &str, _memo: VerificationMemo) {}
        fn invalidate_for_subject(&self, _subject_cid: &str) {}
    }

    #[test]
    fn store_trait_is_object_safe() {
        let store: &dyn VerificationMemoStore = &NullStore;
        assert!(store.get("subject", "evaluator").is_none());
    }

    #[test]
    fn in_memory_store_starts_empty() {
        let store = InMemoryVerificationMemoStore::new();
        assert!(store.is_empty());
        assert!(store.get("subject", "evaluator").is_none());
    }

    #[test]
    fn in_memory_store_put_then_get_round_trips() {
        let store = InMemoryVerificationMemoStore::new();
        store.put("subject-a", "evaluator-a", sample_memo());
        assert_eq!(store.get("subject-a", "evaluator-a"), Some(sample_memo()));
    }

    #[test]
    fn in_memory_store_keys_by_subject_and_evaluator_pair() {
        let store = InMemoryVerificationMemoStore::new();
        store.put("subject-a", "evaluator-a", sample_memo());
        // Same subject, different evaluator: no memo.
        assert!(store.get("subject-a", "evaluator-b").is_none());
        // Different subject, same evaluator: no memo.
        assert!(store.get("subject-b", "evaluator-a").is_none());
    }

    #[test]
    fn in_memory_store_invalidate_for_subject_drops_only_that_subject() {
        let store = InMemoryVerificationMemoStore::new();
        store.put("subject-a", "evaluator-a", sample_memo());
        store.put("subject-b", "evaluator-a", sample_memo());
        store.invalidate_for_subject("subject-a");
        assert!(store.get("subject-a", "evaluator-a").is_none());
        assert!(store.get("subject-b", "evaluator-a").is_some());
    }

    #[test]
    fn in_memory_store_never_grows_past_capacity() {
        let store = InMemoryVerificationMemoStore::with_capacity(2);
        store.put("subject-a", "evaluator", sample_memo());
        store.put("subject-b", "evaluator", sample_memo());
        store.put("subject-c", "evaluator", sample_memo());

        assert_eq!(store.len(), 2);
        assert!(store.get("subject-a", "evaluator").is_none());
        assert!(store.get("subject-b", "evaluator").is_some());
        assert!(store.get("subject-c", "evaluator").is_some());
    }

    #[test]
    fn in_memory_store_eviction_respects_recent_reads() {
        let store = InMemoryVerificationMemoStore::with_capacity(2);
        store.put("subject-a", "evaluator", sample_memo());
        store.put("subject-b", "evaluator", sample_memo());
        assert!(store.get("subject-a", "evaluator").is_some());

        store.put("subject-c", "evaluator", sample_memo());

        assert!(store.get("subject-a", "evaluator").is_some());
        assert!(store.get("subject-b", "evaluator").is_none());
        assert!(store.get("subject-c", "evaluator").is_some());
    }

    /// The T7 plumbing invariant: two handles sharing the SAME `Arc` see
    /// each other's writes — proving the store is fit to be process-lifetime
    /// (constructed once in `main.rs`, `Arc`'d, cloned into every
    /// `EprService` construction site) rather than accidentally
    /// per-request-fresh.
    #[test]
    fn shared_arc_handles_see_each_others_writes() {
        use std::sync::Arc;

        let store: Arc<dyn VerificationMemoStore> = Arc::new(InMemoryVerificationMemoStore::new());
        let handle_a = store.clone();
        let handle_b = store.clone();

        handle_a.put("subject-a", "evaluator-a", sample_memo());

        // A second Arc handle to the SAME underlying store — as if it came
        // from a second per-request EprService construction — sees the
        // write the first handle made.
        assert_eq!(
            handle_b.get("subject-a", "evaluator-a"),
            Some(sample_memo())
        );
    }
}
