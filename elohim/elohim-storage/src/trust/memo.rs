//! `VerificationMemo` — a record of a verification that already happened, so
//! a later request for the same (subject, evaluator, epoch, lens) can skip
//! re-verifying at full cost.
//!
//! Design: `genesis/docs/superpowers/plans/2026-08-08-head-plane-trust-gradient-program-plan.md`
//! §3 L5. This module ships the `VerificationMemo` type, the
//! `VerificationMemoStore` trait, and [`InMemoryVerificationMemoStore`] (T7's
//! process-lifetime plumbing) — but no WIRING into `authorize_reach_for_human`
//! (T9's job) and no `invalidate_for_subject` caller (T19's FeedbackSignal
//! hook). `TrustGradient::inert()` hard-codes `memo: None` at every call site,
//! so nothing in this module is consulted for a decision yet.
//!
//! **NO NUMERIC FIELD, EVER** — §4.2's derived-not-stored discipline applies
//! to this memo specifically (the `.epr-meta` rail T12 adds exists to catch
//! `score:`-shaped drift here). `epoch` is a [`crate::trust::snapshot::TrustEpoch`]
//! token (a `String` wrapper), never a bare integer; `depth` is the
//! [`crate::trust::pricer::VerificationDepth`] enum, never a numeric score.

use std::collections::HashMap;
use std::sync::Mutex;

use chrono::{DateTime, Utc};

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
}

/// Stores and retrieves [`VerificationMemo`]s, keyed by (subject, evaluator).
///
/// No implementation ships with this task — `TrustGradient::inert()` always
/// passes `memo: None`, so no caller consults a store yet. `Send + Sync`
/// because the eventual store is `Arc<dyn VerificationMemoStore>` shared
/// process-lifetime (T7's prerequisite fix to the per-request `EprService`
/// construction).
pub trait VerificationMemoStore: Send + Sync {
    /// Look up a still-valid memo for (subject, evaluator). `None` means "no
    /// memo, or it was invalidated" — the caller re-verifies at full cost,
    /// exactly today's (INERT) behavior.
    fn get(&self, subject_cid: &str, evaluator_agent_cid: &str) -> Option<VerificationMemo>;

    /// Record a fresh verification result.
    fn put(&self, subject_cid: &str, evaluator_agent_cid: &str, memo: VerificationMemo);

    /// Drop every memo naming `subject_cid`. The primary invalidation path
    /// is the FeedbackSignal hook (T19: correct-but-dormant until then) —
    /// TTL eviction, if a store implements one, is a backstop only, never
    /// primary.
    fn invalidate_for_subject(&self, subject_cid: &str);
}

/// The minimal in-memory [`VerificationMemoStore`] — a `Mutex<HashMap<...>>`
/// keyed by `(subject_cid, evaluator_agent_cid)`, following the crate's
/// established small-store shape (`services::commitment_fetcher::MockCommitmentFetcher`).
///
/// **T7 plumbing only.** This exists so a store can be `Arc`'d once in
/// `main.rs` and given process lifetime, threaded to every `EprService`
/// construction site (HTTP per-request, libp2p per-request snapshot, iroh
/// process-lifetime instance). Nothing calls [`Self::get`] or [`Self::put`]
/// yet — `TrustGradient::inert()` still passes `memo: None` everywhere, so
/// no caller of `authorize_reach_for_human` consults this store. T9 wires
/// consumption; T19's `standing_projector` is what eventually makes a cache
/// hit here mean something beyond "we asked before."
///
/// No persistence, no TTL eviction (memo.rs module doc: TTL is a backstop
/// ONLY, and only once something writes memos — moot until T9).
#[derive(Debug, Default)]
pub struct InMemoryVerificationMemoStore {
    memos: Mutex<HashMap<(String, String), VerificationMemo>>,
}

impl InMemoryVerificationMemoStore {
    pub fn new() -> Self {
        Self::default()
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
            .insert(key, memo);
    }

    fn invalidate_for_subject(&self, subject_cid: &str) {
        self.memos
            .lock()
            .expect("InMemoryVerificationMemoStore mutex poisoned")
            .retain(|(subject, _), _| subject != subject_cid);
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
