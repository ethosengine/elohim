//! Shared reconcile-stream rails (spec §4.1, R-E): the gap state machine and
//! dispatch budget used by BOTH the replication stream (whole-inventory,
//! node-policy) and the acquisition stream (desired-set, user-declared).
//! ONE controller pattern governs all reconcile streams — a parallel bespoke
//! fetcher is a coherence violation.

use std::collections::{HashMap, HashSet};
use std::time::Duration;

/// Stable fingerprint of a set of pre-formatted entry lines: sort, then hash
/// with a `\n` delimiter after each entry. Entries containing `\n` use an
/// invalid-UTF-8 marker plus a big-endian byte length before their bytes; this
/// keeps the historical byte stream unchanged for ordinary entries while
/// preventing `["alpha", "beta"]` from colliding with `["alpha\nbeta"]`.
///
/// Extracted from `p2p::sync_round::corpus_digest` (T4, head-plane
/// trust-gradient program) so [`crate::db::content_diesel::head_corpus_digest`]
/// (a DIFFERENT corpus — the SQL content table, not the sync-round DocStore)
/// can reuse the exact same fold instead of restating it. Callers own their
/// own entry-line formatting (e.g. `"{doc_id}={sorted_heads.join(\",\")}"` or
/// `"{id}={dht_anchor_hash}"`) — this function only owns the sort+hash step,
/// which is where a byte-for-byte drift would silently disable an `InSync`/
/// `in_sync` shortcut.
///
/// Deliberately NOT in the `seam-contracts` crate: its no-heavy-deps boundary
/// test forbids a `sha2` dependency.
pub fn digest_of_entry_lines(mut entries: Vec<String>) -> String {
    use sha2::{Digest, Sha256};
    // Q11 atomic timing budget: the whole fold. Pure and expected
    // sub-millisecond — the histogram exists to PROVE that, not assume it.
    let digest_fold_started = std::time::Instant::now();
    entries.sort();
    let mut h = Sha256::new();
    for e in &entries {
        if e.contains('\n') {
            // 0xff cannot begin a valid UTF-8 String, so this framed form can
            // never alias the legacy unframed form used by ordinary entries.
            h.update([0xff]);
            h.update((e.len() as u64).to_be_bytes());
        }
        h.update(e.as_bytes());
        h.update(b"\n");
    }
    let digest = format!("sha256:{:x}", h.finalize());
    crate::metrics::observe_atom_duration(
        crate::metrics::ConvergenceAtom::DigestFold,
        digest_fold_started.elapsed(),
    );
    digest
}

/// Generic gap state machine: known-local / pending / completed / failed(retries).
/// Retry discipline is retry-on-NEXT-cycle (never immediate re-queue — the
/// freeze-at-partial battle-scar, see replication.rs mark_failed docs).
#[derive(Debug, Default)]
pub struct GapTracker {
    local_ids: HashSet<String>,
    pending: HashSet<String>,
    completed: HashSet<String>,
    failed: HashMap<String, u32>,
    caught_up: bool,
    max_retries: u32,
}

/// Snapshot counts for status surfaces.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GapCounts {
    pub pending: usize,
    pub completed: usize,
    pub failed: usize,
    /// This SWEEP is over: every discovered gap was healed OR exhausted its
    /// retry budget. Load-bearing for the existing `/health`, `/p2p/status`
    /// and a2o surfaces; deliberately NOT renamed. It does not mean the peer
    /// converged — see [`GapCounts::converged`].
    pub caught_up: bool,
    /// Gaps abandoned at `max_retries` — counted, never silently dropped.
    /// `enqueue_missing` refuses to re-queue these, so they are permanently
    /// absent from `pending`; that absence is why `pending.is_empty()`
    /// overstates convergence.
    ///
    /// ## NOT the same number as `elohim_projection_reconcile_exhausted`
    ///
    /// The metric and this field share a word and measure different things.
    /// Read this before concluding that "exhausted" is holding a gauge down:
    ///
    /// - **This field** counts abandonment inside ONE tracker's lifetime. It can
    ///   only fire on a tracker that OUTLIVES its cycles — `replication.rs`'s
    ///   `Arc<RwLock<GapTracker>>` and `acquisition.rs`'s per-pin trackers,
    ///   where a failure accumulates across cycles until it crosses
    ///   `max_retries`.
    /// - **The metric** publishes `projection_reconcile::MissLedger`'s
    ///   cross-sweep adjudication (`exhausted_persistent`), which is a wholly
    ///   separate structure and is already excluded from convergence BY
    ///   CONSTRUCTION: a `MissLedger`-exhausted id is never admitted to a
    ///   `GapTracker` at all, so it can appear in neither `pending` nor here.
    ///
    /// The projection-reconcile arms build a FRESH tracker per sweep and call
    /// `mark_failed` at most once per id per sweep (the in-leg `call_with_retry`
    /// never touches the tracker), so with `MAX_RETRIES = 3` this field is
    /// structurally 0 on those arms — verified live: alpha A published
    /// `exhausted: 0` beside `pending: 2894`. Pinned by
    /// [`tests::a_per_sweep_tracker_cannot_exhaust_under_a_multi_attempt_budget`].
    /// A reconcile arm that will not converge is held down by `pending` or by
    /// unadjudicated divergence, never by this term.
    pub exhausted: usize,
    /// This PEER holds what its peers advertised: nothing pending AND nothing
    /// abandoned. Strictly stronger than `caught_up`; this is the field an SLO
    /// may be offered over.
    ///
    /// The `exhausted` term is kept deliberately: on the long-lived trackers it
    /// is REAL undone work (attempted `max_retries` times this tracker's life
    /// and given up on), which is exactly what an SLO must not be blind to.
    /// Dropping it would collapse `converged` into `caught_up` and delete the
    /// distinction this pair exists to draw — see the `exhausted` field docs for
    /// why doing so would also be inert on the reconcile arms.
    pub converged: bool,
}

impl GapTracker {
    pub fn new(max_retries: u32) -> Self {
        Self {
            max_retries,
            ..Default::default()
        }
    }

    /// Raise (or lower) the retry budget on a LIVE tracker.
    ///
    /// The acquisition stream sizes its budget from the CONNECTED PEER COUNT —
    /// `MAX_RETRIES = 3` on a 6-peer fabric probes at most 3 of 6 possible
    /// providers before declaring an item unsatisfiable, so "exhausted" meant
    /// "half the fabric was never asked". Peer count changes between reconcile
    /// cycles and trackers outlive a cycle, so the budget must be adjustable in
    /// place. RAISING it re-admits ids whose failed-count is below the new bound
    /// on the next `reconcile_desired` — which is the point.
    pub fn set_max_retries(&mut self, max_retries: u32) {
        self.max_retries = max_retries;
    }

    /// Ids whose retry budget is spent. `enqueue_missing` refuses to re-queue
    /// these, so they leave `pending` permanently — the exact reason
    /// `pending.is_empty()` (i.e. `caught_up`) overstates convergence.
    pub fn exhausted_count(&self) -> usize {
        self.failed
            .values()
            .filter(|&&n| n >= self.max_retries)
            .count()
    }

    pub fn counts(&self) -> GapCounts {
        let exhausted = self.exhausted_count();
        GapCounts {
            pending: self.pending.len(),
            completed: self.completed.len(),
            failed: self.failed.len(),
            caught_up: self.caught_up,
            exhausted,
            converged: self.pending.is_empty() && exhausted == 0,
        }
    }

    /// Register IDs already present locally. `flip_caught_up_if_restored`
    /// preserves replication.rs's restored-pod semantics; the acquisition
    /// stream passes `false` (a pin is never caught-up before reconcile).
    /// Does NOT drain `pending` of ids now present in `local_ids`; callers
    /// pair this with `mark_completed` for cross-stream byte-arrivals.
    pub fn set_local_ids_with(&mut self, ids: HashSet<String>, flip_caught_up_if_restored: bool) {
        let had_content = !ids.is_empty();
        self.local_ids = ids;
        if flip_caught_up_if_restored && had_content && self.pending.is_empty() {
            self.caught_up = true;
        }
    }

    pub fn set_local_ids(&mut self, ids: HashSet<String>) {
        self.set_local_ids_with(ids, false);
    }

    /// Inventory-driven entry (replication): remote advertises, we diff.
    pub fn discover(&mut self, remote_ids: Vec<String>) -> Vec<String> {
        self.enqueue_missing(remote_ids)
    }

    /// Desired-set-driven entry (acquisition): WE declare, then diff.
    /// Identical machine, different direction of declaration (spec §4.2).
    pub fn reconcile_desired(&mut self, want_ids: Vec<String>) -> Vec<String> {
        self.enqueue_missing(want_ids)
    }

    fn enqueue_missing(&mut self, ids: Vec<String>) -> Vec<String> {
        let mut new_gaps = Vec::new();
        for id in ids {
            if self.local_ids.contains(&id)
                || self.completed.contains(&id)
                || self.pending.contains(&id)
            {
                continue;
            }
            if self.failed.get(&id).copied().unwrap_or(0) >= self.max_retries {
                continue;
            }
            self.pending.insert(id.clone());
            new_gaps.push(id);
        }
        if !new_gaps.is_empty() {
            self.caught_up = false;
        }
        new_gaps
    }

    pub fn mark_completed(&mut self, id: &str) {
        self.pending.remove(id);
        self.failed.remove(id);
        self.completed.insert(id.to_string());
        self.local_ids.insert(id.to_string());
    }

    /// Acquisition-only: count wanted ids that are ALREADY present locally as
    /// completed, so the acquisition rollup's `total == fetched` holds for
    /// content the node already holds. `enqueue_missing` skips already-local
    /// ids (neither pending nor completed), so without this they inflate
    /// `total` but never `fetched` → the pull stream reports `caughtUp=false`
    /// forever on a content-holder node (the live matthew case).
    ///
    /// This is NOT a false-complete: R-A's "never claim done before bytes
    /// ARRIVE" is satisfied — the bytes ARE present locally (that is what
    /// `local_ids` means). Distinct from `mark_completed` (the cross-stream
    /// byte-ARRIVAL signal). Idempotent; leaves `pending`/`failed` untouched.
    pub fn mark_local_wants_satisfied(&mut self, want_ids: &[String]) {
        for id in want_ids {
            if self.local_ids.contains(id) {
                self.completed.insert(id.clone());
            }
        }
    }

    /// Removed from pending, NOT re-queued — the next reconcile/discover
    /// cycle re-includes it while fail_count < max_retries (R-E).
    pub fn mark_failed(&mut self, id: &str) {
        self.pending.remove(id);
        *self.failed.entry(id.to_string()).or_insert(0) += 1;
    }

    pub fn update_caught_up(&mut self) {
        self.caught_up = self.pending.is_empty();
    }

    /// True if this tracker currently has `id` in flight (in `pending`).
    /// Does NOT return true for retryable-next-cycle items — a failed-but-
    /// retryable id was removed from `pending` and re-enters only on the next
    /// `reconcile_desired`/`discover` call. Used as a dispatch-time filter.
    pub fn wants(&self, id: &str) -> bool {
        self.pending.contains(id)
    }

    /// Distinct ids currently in flight (in `pending`). Used by the per-EPR
    /// rollup to union across pins of the same head_ref (shared ids dedupe).
    pub fn pending_ids(&self) -> Vec<String> {
        self.pending.iter().cloned().collect()
    }

    /// Distinct ids byte-arrival complete (in `completed`).
    pub fn completed_ids(&self) -> Vec<String> {
        self.completed.iter().cloned().collect()
    }

    /// Distinct ids that have failed at least once (keys of `failed`).
    pub fn failed_ids(&self) -> Vec<String> {
        self.failed.keys().cloned().collect()
    }
}

/// Slot-backpressure helper (R-E): dispatch rate becomes a natural function
/// of peer response speed — mirrors drain_gap_queue's MAX_REPLICATION_INFLIGHT.
#[derive(Debug, Clone, Copy)]
pub struct DispatchBudget {
    max_inflight: usize,
}

impl DispatchBudget {
    pub fn new(max_inflight: usize) -> Self {
        Self { max_inflight }
    }
    pub fn available(&self, in_flight: usize) -> usize {
        self.max_inflight.saturating_sub(in_flight)
    }
}

/// AIMD batch-size controller for the head-plane batch externs — the SECOND,
/// orthogonal pacing axis beside [`DispatchBudget`].
///
/// The two are deliberately both kept and never merged: `DispatchBudget` bounds
/// **concurrency** (how many conductor round-trips may be in flight at once),
/// this bounds **batch size** (how many ids ride ONE round-trip). The write-guard
/// constraint (`2026-07-20-adam-slow-link-write-guard-saturation.md`) forbids
/// raising the per-tick cap OR the concurrency; only the per-round-trip YIELD may
/// rise, which is exactly and only what this controller moves.
///
/// ## Why AIMD rather than a token bucket
///
/// A token bucket needs a refill RATE, and there is no honest number to put
/// there: adam (WAN, saturated conductor) and matthew (LAN) differ by an order of
/// magnitude and the difference is a real fixture property, not noise to be
/// laundered into one constant. AIMD needs no rate — it needs only a signal that
/// says "you asked for too much", which the batch extern hands back for free as
/// `queue_wait` (observed round-trip time MINUS the extern's own in-wasm
/// `elapsed_ms` — i.e. the time the call spent queued for a conductor DB read
/// permit rather than doing work). Under the same constants adam converges small
/// and matthew large, and the heterogeneity survives in the
/// `elohim_head_batch_size` gauge instead of being averaged away.
///
/// ## The control law
///
/// [`AdaptiveBatchBudget::next_size`] is a PURE total function — no clock, no
/// state, no I/O — so the convergence properties are testable without a
/// conductor:
///
/// - `queue_wait <= threshold` ⇒ additive increase (`current + increment`)
/// - `queue_wait  > threshold` ⇒ multiplicative decrease (`current / 2`)
/// - always clamped into `[floor, ceiling]`
///
/// Defaults: floor 8, ceiling 128, threshold 2s, +16 / ×0.5.
#[derive(Debug, Clone, Copy)]
pub struct AdaptiveBatchBudget {
    current: usize,
    floor: usize,
    ceiling: usize,
    threshold: Duration,
    increment: usize,
}

/// Smallest batch the controller will ever ask for. Below this the round-trip
/// overhead dominates and batching stops paying for itself.
pub const BATCH_SIZE_FLOOR: usize = 8;
/// Largest batch the controller will ever ask for. Kept well under the zome's
/// `BATCH_ID_CEILING` (256) so a ceiling-clamp `unattempted` tail is a
/// coordinator-side ceiling event, never this controller's doing.
pub const BATCH_SIZE_CEILING: usize = 128;
/// Queue-wait above which the conductor is telling us it is backpressured.
pub const BATCH_QUEUE_WAIT_THRESHOLD: Duration = Duration::from_secs(2);
/// Additive-increase step.
pub const BATCH_SIZE_INCREMENT: usize = 16;

impl Default for AdaptiveBatchBudget {
    fn default() -> Self {
        Self::new(
            BATCH_SIZE_FLOOR,
            BATCH_SIZE_CEILING,
            BATCH_QUEUE_WAIT_THRESHOLD,
            BATCH_SIZE_INCREMENT,
        )
    }
}

impl AdaptiveBatchBudget {
    /// Start at the FLOOR, never the ceiling: an unknown conductor is assumed
    /// backpressured until it proves otherwise (fail-closed toward the
    /// write-guard constraint, the same direction `NetworkStage` fails).
    pub fn new(floor: usize, ceiling: usize, threshold: Duration, increment: usize) -> Self {
        let floor = floor.max(1);
        let ceiling = ceiling.max(floor);
        Self {
            current: floor,
            floor,
            ceiling,
            threshold,
            increment,
        }
    }

    /// The batch size to ask for on the NEXT call.
    pub fn current(&self) -> usize {
        self.current
    }

    /// Fold one COMPLETED batch call into the controller: the partial-Ok arm.
    ///
    /// A call can be admitted instantly (empty local gate) and still attempt
    /// almost nothing — the coordinator's in-wasm deadline refuses the tail
    /// (`budget_exhausted`), or its id ceiling does. Feeding only
    /// `admission_wait` read exactly that call as headroom → additive increase:
    /// the controller GREW on the strongest evidence it should shrink. This is
    /// the same defect class as the once-silent `Err` arm
    /// ([`Self::observe_call_failure`]) — there the controller heard nothing;
    /// here it heard the wrong half of the story.
    ///
    /// A refused tail (`unattempted > 0`) takes the decrease path regardless of
    /// admission wait; a fully-attempted call folds `admission_wait` as before.
    /// One control law either way ([`Self::next_size`]).
    ///
    /// **Contract test:** `a_budget_starved_batch_shrinks_even_with_instant_admission`.
    pub fn observe_batch_outcome(&mut self, admission_wait: Duration, unattempted: usize) {
        if unattempted > 0 {
            self.observe_call_failure();
        } else {
            self.observe(admission_wait);
        }
    }

    /// Fold one observed queue-wait into the controller.
    ///
    /// The caller supplies the ADMISSION wait
    /// ([`crate::services::head_batch_resolver::BatchResolution::admission_wait`]),
    /// not `RTT − in-wasm elapsed_ms`. The latter is not monotonic in batch size
    /// — it subtracts exactly the interval a stalled extern spends waiting on a
    /// conductor read permit — so feeding it here made the loop grow the batch
    /// precisely when the conductor was struggling.
    pub fn observe(&mut self, queue_wait: Duration) {
        self.current = Self::next_size(
            self.current,
            queue_wait,
            self.threshold,
            self.floor,
            self.ceiling,
            self.increment,
        );
    }

    /// Fold a CALL-LEVEL FAILURE into the controller: multiplicative decrease.
    ///
    /// A call that never returned an answer is the strongest backpressure
    /// evidence available — stronger than any queue-wait, because the conductor
    /// could not complete the round-trip at all. Before this existed the
    /// controller only ever heard from the `Ok` arm, so on a conductor failing
    /// every call it kept its batch size forever and additively grew it back the
    /// moment one call succeeded: the loop was structurally unable to shrink on
    /// the evidence that mattered most.
    ///
    /// Implemented as [`Self::next_size`] over a queue-wait past the threshold,
    /// so failure and severe backpressure take the SAME decrease path and there
    /// is only one control law to reason about.
    ///
    /// **Contract test:** `aimd_shrinks_on_a_call_level_failure`.
    pub fn observe_call_failure(&mut self) {
        self.current = Self::next_size(
            self.current,
            self.threshold.saturating_add(Duration::from_millis(1)),
            self.threshold,
            self.floor,
            self.ceiling,
            self.increment,
        );
    }

    /// THE control law, pure and total.
    ///
    /// **Concerns:** C6a (bounded work — the result is always inside
    /// `[floor, ceiling]`, for every input including a `current` already outside
    /// it).
    ///
    /// **Contract tests:** `aimd_increases_additively_under_the_threshold`,
    /// `aimd_halves_over_the_threshold`, `aimd_clamps_into_the_band`,
    /// `aimd_converges_down_then_back_up`.
    pub fn next_size(
        current: usize,
        queue_wait: Duration,
        threshold: Duration,
        floor: usize,
        ceiling: usize,
        increment: usize,
    ) -> usize {
        let floor = floor.max(1);
        let ceiling = ceiling.max(floor);
        let next = if queue_wait > threshold {
            // MULTIPLICATIVE DECREASE — back off hard and immediately. A
            // saturated conductor is the one resource the whole pacing layer
            // exists to protect.
            current / 2
        } else {
            // ADDITIVE INCREASE — probe upward slowly.
            current.saturating_add(increment)
        };
        next.clamp(floor, ceiling)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aimd_increases_additively_under_the_threshold() {
        let t = BATCH_QUEUE_WAIT_THRESHOLD;
        assert_eq!(
            AdaptiveBatchBudget::next_size(8, Duration::from_millis(10), t, 8, 128, 16),
            24
        );
        // Exactly AT the threshold is not over it — the comparison is strict, so
        // a conductor sitting on the line is not punished for it.
        assert_eq!(AdaptiveBatchBudget::next_size(8, t, t, 8, 128, 16), 24);
    }

    #[test]
    fn aimd_halves_over_the_threshold() {
        let t = BATCH_QUEUE_WAIT_THRESHOLD;
        assert_eq!(
            AdaptiveBatchBudget::next_size(128, Duration::from_secs(5), t, 8, 128, 16),
            64
        );
        assert_eq!(
            AdaptiveBatchBudget::next_size(64, Duration::from_secs(5), t, 8, 128, 16),
            32
        );
    }

    #[test]
    fn aimd_shrinks_on_a_call_level_failure() {
        // The arm that used to be silent. A conductor that cannot complete a
        // round-trip is the strongest backpressure signal there is.
        let mut b = AdaptiveBatchBudget::new(8, 128, BATCH_QUEUE_WAIT_THRESHOLD, 16);
        b.observe(Duration::ZERO); // 8 → 24
        b.observe(Duration::ZERO); // 24 → 40
        assert_eq!(b.current(), 40);

        b.observe_call_failure();
        assert_eq!(b.current(), 20, "a call-level failure must halve the batch");

        // Repeated failures walk it to the floor and stop there — never below.
        for _ in 0..10 {
            b.observe_call_failure();
        }
        assert_eq!(b.current(), BATCH_SIZE_FLOOR);
    }

    #[test]
    fn a_budget_starved_batch_shrinks_even_with_instant_admission() {
        // The partial-Ok blind spot: a call can be ADMITTED instantly (local
        // gate empty) and still attempt almost nothing because the coordinator's
        // in-wasm deadline refused the tail (`budget_exhausted`). Feeding only
        // `admission_wait` reads that call as headroom → additive increase —
        // growth on the exact evidence that should shrink it.
        let mut b = AdaptiveBatchBudget::new(8, 128, BATCH_QUEUE_WAIT_THRESHOLD, 16);
        b.observe(Duration::ZERO); // 8 → 24
        b.observe(Duration::ZERO); // 24 → 40
        assert_eq!(b.current(), 40);

        // Instant admission + a refused tail must take the DECREASE path.
        b.observe_batch_outcome(Duration::ZERO, 32);
        assert_eq!(
            b.current(),
            20,
            "a batch with a refused (unattempted) tail must halve even when \
             admission was instant — the coordinator declining ids IS the \
             backpressure signal"
        );

        // With no refused tail the same call folds admission_wait as before.
        b.observe_batch_outcome(Duration::ZERO, 0);
        assert_eq!(b.current(), 36, "a fully-attempted batch keeps the AI path");

        // Repeated starvation walks to the floor and never below.
        for _ in 0..10 {
            b.observe_batch_outcome(Duration::ZERO, 8);
        }
        assert_eq!(b.current(), BATCH_SIZE_FLOOR);
    }

    #[test]
    fn aimd_clamps_into_the_band() {
        let t = BATCH_QUEUE_WAIT_THRESHOLD;
        // Ceiling clamp on increase.
        assert_eq!(
            AdaptiveBatchBudget::next_size(120, Duration::ZERO, t, 8, 128, 16),
            128
        );
        // Floor clamp on decrease — a backpressured conductor must never drive
        // the batch to zero, which would silently stop the leg.
        assert_eq!(
            AdaptiveBatchBudget::next_size(8, Duration::from_secs(60), t, 8, 128, 16),
            8
        );
        assert_eq!(
            AdaptiveBatchBudget::next_size(1, Duration::from_secs(60), t, 8, 128, 16),
            8
        );
        // A `current` that starts OUTSIDE the band is pulled back in, in both
        // directions — the clamp is on the RESULT, not on the input.
        assert_eq!(
            AdaptiveBatchBudget::next_size(4096, Duration::ZERO, t, 8, 128, 16),
            128
        );
        assert_eq!(
            AdaptiveBatchBudget::next_size(0, Duration::ZERO, t, 8, 128, 16),
            16
        );
    }

    /// The property that matters operationally: a backpressure burst drives the
    /// size DOWN monotonically to the floor, and a quiet conductor drives it back
    /// UP monotonically to the ceiling. Neither direction oscillates or stalls.
    #[test]
    fn aimd_converges_down_then_back_up() {
        let mut b = AdaptiveBatchBudget::default();
        // Climb to the ceiling on a quiet conductor.
        let mut last = b.current();
        for _ in 0..32 {
            b.observe(Duration::from_millis(1));
            assert!(b.current() >= last, "additive increase must be monotone");
            last = b.current();
        }
        assert_eq!(b.current(), BATCH_SIZE_CEILING);

        // Now saturate it: halving must reach the floor and STOP there.
        let mut last = b.current();
        for _ in 0..32 {
            b.observe(Duration::from_secs(30));
            assert!(
                b.current() <= last,
                "multiplicative decrease must be monotone"
            );
            last = b.current();
        }
        assert_eq!(b.current(), BATCH_SIZE_FLOOR);
    }

    /// The heterogeneity claim from the plan, as a test: the SAME constants must
    /// separate a slow (WAN) conductor from a fast (LAN) one rather than
    /// averaging them into one number.
    #[test]
    fn the_same_constants_separate_a_slow_conductor_from_a_fast_one() {
        let mut adam = AdaptiveBatchBudget::default();
        let mut matthew = AdaptiveBatchBudget::default();
        for _ in 0..16 {
            adam.observe(Duration::from_secs(6)); // saturated WAN conductor
            matthew.observe(Duration::from_millis(40)); // quiet LAN conductor
        }
        assert_eq!(adam.current(), BATCH_SIZE_FLOOR);
        assert_eq!(matthew.current(), BATCH_SIZE_CEILING);
        assert!(adam.current() < matthew.current());
    }

    #[test]
    fn digest_entry_line_edge_cases_are_byte_pinned() {
        let cases = [
            (
                "empty set",
                Vec::<String>::new(),
                "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            ),
            (
                "empty entry",
                vec![String::new()],
                "sha256:01ba4719c80b6fe911b091a7c05124b64eeece964e09c058ef8f9805daca546b",
            ),
            (
                "unicode entry",
                vec!["café=α,β".to_string()],
                "sha256:970a36aa23ba1629e6d87fac7229402cf5821a784d234e3c4c38c5be323bcba3",
            ),
            (
                "embedded newline",
                vec!["alpha\nbeta".to_string()],
                "sha256:981be2eac0bd6bf3cf016a96d4e0252fa456f574e5e8c2ccb0dc2d6073f133e9",
            ),
        ];

        for (case, entries, expected) in cases {
            assert_eq!(digest_of_entry_lines(entries), expected, "{case}");
        }
    }

    #[test]
    fn embedded_newline_cannot_impersonate_two_entries() {
        assert_ne!(
            digest_of_entry_lines(vec!["alpha".into(), "beta".into()]),
            digest_of_entry_lines(vec!["alpha\nbeta".into()]),
            "an embedded newline must not create a false InSync result"
        );
    }

    #[test]
    fn desired_set_reconcile_diffs_wants_against_local() {
        let mut t = GapTracker::new(3);
        t.set_local_ids(["a".into()].into_iter().collect());
        let gaps = t.reconcile_desired(vec!["a".into(), "b".into(), "c".into()]);
        assert_eq!(gaps.len(), 2);
        assert!(gaps.contains(&"b".to_string()) && gaps.contains(&"c".to_string()));
        let s = t.counts();
        assert_eq!((s.pending, s.completed, s.failed), (2, 0, 0));
        assert!(!s.caught_up);
    }

    #[test]
    fn exhausted_retries_drop_out_of_reconcile() {
        let mut t = GapTracker::new(1);
        t.reconcile_desired(vec!["x".into()]);
        t.mark_failed("x");
        // next cycle: fail_count=1 >= max_retries=1 → not re-queued
        let gaps = t.reconcile_desired(vec!["x".into()]);
        assert!(gaps.is_empty());
        assert_eq!(t.counts().failed, 1);
    }

    #[test]
    fn retry_exhausted_gaps_are_caught_up_but_not_converged() {
        // The live lie: a tracker whose gaps all failed past max_retries drains
        // `pending` (mark_failed removes without re-queue) and reports
        // caught_up=true while nothing was healed. That is "this cycle is
        // over", not "this peer holds what its peers hold" — the shape behind
        // caughtUp:true over 1860 divergent anchors.
        //
        // SCOPE: this drives the CROSS-CYCLE tracker shape (one tracker, three
        // cycles of re-discover-then-fail) — i.e. `replication.rs` and
        // `acquisition.rs`, which hold their trackers across cycles. It is NOT
        // the projection-reconcile shape: those arms rebuild the tracker every
        // sweep and answer each id once, so they can never reach this state (see
        // `a_per_sweep_tracker_cannot_exhaust_under_a_multi_attempt_budget`).
        // Reading this test as if it covered the reconcile arms is what sent a
        // convergence diagnosis after the wrong term. The reconcile arms' real
        // shape is guarded in `projection_reconcile.rs` by
        // `a_sweep_whose_every_gap_answered_none_does_not_claim_convergence`.
        let mut t = GapTracker::new(2);
        t.discover(vec!["a".into(), "b".into(), "c".into()]);
        for _ in 0..2 {
            for id in ["a", "b", "c"] {
                t.mark_failed(id);
            }
            // Retry-on-NEXT-cycle: re-discovery is what re-enqueues (R-E).
            t.discover(vec!["a".into(), "b".into(), "c".into()]);
        }
        t.update_caught_up();

        let c = t.counts();
        assert_eq!(c.completed, 0, "nothing was healed");
        assert!(
            c.caught_up,
            "existing semantics preserved: the sweep is over"
        );
        assert_eq!(c.exhausted, 3, "all three exhausted their retry budget");
        assert!(
            !c.converged,
            "converged must be FALSE when gaps were abandoned, not healed"
        );
    }

    #[test]
    fn a_fully_healed_sweep_is_both_caught_up_and_converged() {
        let mut t = GapTracker::new(2);
        t.discover(vec!["a".into(), "b".into()]);
        t.mark_completed("a");
        t.mark_completed("b");
        t.update_caught_up();

        let c = t.counts();
        assert!(c.caught_up);
        assert!(c.converged, "healed gaps converge");
        assert_eq!(c.exhausted, 0);
    }

    #[test]
    fn a_gap_still_retrying_is_neither_caught_up_nor_converged() {
        // Guards the other direction: a failure that has budget left is still
        // in flight, so `exhausted` must not count it.
        let mut t = GapTracker::new(3);
        t.discover(vec!["a".into()]);
        t.mark_failed("a");
        t.discover(vec!["a".into()]); // re-queued: 1 < 3
        t.update_caught_up();

        let c = t.counts();
        assert!(!c.caught_up);
        assert!(!c.converged);
        assert_eq!(c.exhausted, 0, "retryable != exhausted");
    }

    #[test]
    fn raising_the_budget_re_admits_an_exhausted_id() {
        // Peer-breadth: an item asked of 3 peers on a 6-peer fabric has not been
        // proven unsatisfiable — five of six providers may never have been
        // probed. Raising the budget to the peer count must let it back in.
        let mut t = GapTracker::new(3);
        t.reconcile_desired(vec!["x".into()]);
        for _ in 0..3 {
            t.mark_failed("x");
            t.reconcile_desired(vec!["x".into()]);
        }
        assert_eq!(t.counts().exhausted, 1, "spent at the 3-retry budget");

        t.set_max_retries(6);
        let gaps = t.reconcile_desired(vec!["x".into()]);
        assert_eq!(
            gaps,
            vec!["x".to_string()],
            "re-admitted under the new bound"
        );
        assert_eq!(
            t.counts().exhausted,
            0,
            "not exhausted while unprobed providers remain"
        );
    }

    #[test]
    fn a_per_sweep_tracker_cannot_exhaust_under_a_multi_attempt_budget() {
        // The shape EVERY projection-reconcile arm runs (`discover_*` builds a
        // fresh tracker, `heal_*` marks each id completed-or-failed exactly
        // once, then `update_caught_up`). With a budget above 1, one failure per
        // id per sweep can never reach `max_retries`, so `GapCounts::exhausted`
        // is structurally 0 there and `converged` reduces to `pending.is_empty()`
        // on its own.
        //
        // This is the regression guard for a misdiagnosis, not a feature: the
        // `elohim_projection_reconcile_exhausted{stream=…}` metric (the
        // cross-sweep `MissLedger`, hundreds of ids live) was read as if it were
        // THIS field and blamed for pinning
        // `elohim_projection_reconcile_converged` at 0. It cannot be — alpha A
        // published `exhausted: 0` beside `pending: 2894` while the gauge read
        // 0. Relaxing the `exhausted` term would have changed nothing here and
        // weakened the long-lived trackers below.
        let mut t = GapTracker::new(3);
        t.discover(vec!["a".into(), "b".into(), "c".into()]);
        for id in ["a", "b", "c"] {
            t.mark_failed(id); // one conductor answer per id per sweep
        }
        t.update_caught_up();

        let c = t.counts();
        assert_eq!(c.failed, 3, "all three failed this sweep");
        assert_eq!(
            c.exhausted, 0,
            "one attempt per sweep can never spend a 3-retry budget within ONE tracker"
        );
        assert!(c.caught_up, "the sweep ended: mark_failed drained pending");
        assert!(
            c.converged,
            "with exhausted structurally 0, this arm's convergence is gated by pending alone \
             (and, above the rails, by unadjudicated divergence)"
        );
    }

    #[test]
    fn a_long_lived_tracker_still_exhausts_and_still_blocks() {
        // The other half of the pair: replication/acquisition hold ONE tracker
        // across cycles, so failures accumulate and `exhausted` becomes real
        // undone work. It must keep defeating `converged` — that is the term an
        // SLO would otherwise be blind to.
        let mut t = GapTracker::new(3);
        for _ in 0..3 {
            t.discover(vec!["a".into()]); // re-discovered each cycle
            t.mark_failed("a");
        }
        t.update_caught_up();

        let c = t.counts();
        assert_eq!(
            c.exhausted, 1,
            "three cycles spent the budget on this tracker"
        );
        assert!(c.caught_up, "the cycle ended");
        assert!(
            !c.converged,
            "abandoned-after-max_retries is undone work, not adjudication"
        );
    }

    #[test]
    fn dispatch_budget_caps_inflight() {
        assert_eq!(DispatchBudget::new(50).available(47), 3);
        assert_eq!(DispatchBudget::new(50).available(50), 0);
        assert_eq!(DispatchBudget::new(50).available(60), 0); // saturating
    }
}
