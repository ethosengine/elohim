//! HEAD-BATCH RESOLVER — the storage-side seam over the `content_store` batch
//! externs (`resolve_content_heads_local`, `resolve_canonical_elections`).
//!
//! Head-plane trust-gradient program, L1. The constraint of record
//! (`2026-07-20-adam-slow-link-write-guard-saturation.md`): the head sweep is a
//! READ path harmed by queueing behind kitsune2's write guard. Batching collapses
//! ROUND-TRIPS at flat read-permit cost. **The per-tick cap (200) and the
//! concurrency do NOT rise — only the yield per round-trip.** Raising caps is the
//! forbidden move, and nothing in this module raises one.
//!
//! ## What this seam owns
//!
//! Turning ONE coordinator batch answer into the vocabulary the reconcile arms
//! already speak:
//!
//! - per-item outcomes → [`seam_contracts::Answer`] (C4), with the coordinator's
//!   TYPED failure reason carried alongside rather than collapsed into text;
//! - `unattempted` → a list the caller must return to `pending`, NEVER a failure;
//! - `stop_reason` → surfaced, so "the conductor is backpressured"
//!   (`SharedFailure`) stays distinguishable from "the budget ran out on
//!   schedule" (`BudgetExhausted`/`IdCeiling`), which is not a failure at all;
//! - `queue_wait` = observed round-trip time MINUS the extern's own in-wasm
//!   `elapsed_ms` — a DIAGNOSTIC only (`elohim_head_batch_queue_wait_ms`). It
//!   was the AIMD controller's input until it was found to be non-monotonic in
//!   batch size: it subtracts exactly the interval a stalled extern spends
//!   waiting on a read permit, so a saturated conductor reads as headroom;
//! - `admission_wait` = how long the call queued at the LOCAL capacity gate
//!   ([`crate::conductor_admission`]) — the signal that now drives
//!   [`crate::p2p::reconcile_rails::AdaptiveBatchBudget`], because it rises with
//!   the offered load the controller actually governs;
//! - `schema_version` — read before anything else is believed.
//!
//! ## The fallback rule (BINDING — operator review 2026-08-08)
//!
//! Single-id fallback happens on EXACTLY TWO conditions, and both are "this
//! conductor cannot speak the batch contract at all":
//!
//! 1. an **unknown-function** error — the coordinator has not been hot-swapped
//!    onto this conductor yet (the DNA hash is blind to coordinator zomes, so
//!    this error is the ONLY way to learn it);
//! 2. an explicit **`schema_version` mismatch** or an **undecodable** output.
//!
//! It falls through ONCE, logs ONCE, and never retry-loops the batch: the
//! resolver LATCHES into single-id mode for the rest of the process, so a
//! pre-swap conductor is asked for the batch extern once and never again.
//!
//! Everything else is NOT a fallback trigger:
//!
//! - **per-item `Failed`** → [`Answer::Unreachable`] carrying the typed reason.
//!   Never a single-id retry: the coordinator already answered, with evidence.
//! - **call-level infrastructure failure** (transport, websocket timeout, DB
//!   read-permit saturation) → `Err`, which the caller turns into "ALL ids back
//!   to pending, with backoff". Never a fallback (falling back on a saturated
//!   conductor would multiply its load by the batch size — the exact failure this
//!   program exists to close) and never a failure-marking.
//!
//! ## Shape
//!
//! Trait + production impl + `Mock*` in ONE file, NOT behind `cfg(test)` — the
//! [`crate::services::commitment_fetcher`] template. The mock is reachable from
//! integration tests and from other modules' unit tests, which is the whole
//! reason it is not gated.

use async_trait::async_trait;
use seam_contracts::Answer;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, Once};
use std::time::{Duration, Instant};

use crate::error::StorageError;
use crate::services::conductor_writes::{
    self, BatchFailureReason, BatchOutcome, BatchStopReason, CanonicalElectionWire,
    ContentHeadWire, BATCH_RESOLVE_SCHEMA_VERSION,
};

/// Default in-wasm deadline handed to the batch externs.
///
/// Mirrors the coordinator's `BATCH_BUDGET_DEFAULT_MS`. It MUST stay strictly
/// below `HealPacing::attempt_timeout` (15s): the extern bounds itself from the
/// inside and returns partial results, whereas the caller-side timeout leaves
/// the conductor executing with nobody listening (`HcClient::call_zome` has no
/// cancellation). Pinned by
/// `projection_reconcile::tests::the_attempt_timeout_stays_above_the_extern_budget`.
pub const BATCH_EXTERN_BUDGET: Duration = Duration::from_secs(4);

/// One id's answer from a batch call.
///
/// **Concerns:** C4 — the three answer states are kept apart, and a per-item
/// `Failed` becomes [`Answer::Unreachable`] (nothing was established about the
/// id) rather than a borrowed `Absent`.
#[derive(Debug, Clone)]
pub struct BatchItem<T> {
    pub id: String,
    /// `Present` — a real answer. `Absent` — the SAME honest local absence the
    /// single-id twin reports (`Resolved(None)`). `Unreachable` — the
    /// coordinator recorded a typed failure for this id.
    pub answer: Answer<T>,
    /// `Some` exactly when `answer` is `Unreachable` from a per-item `Failed`.
    /// This is what lets an arm route a DETERMINISTIC failure into its ordinary
    /// failure accounting while a BACKPRESSURE failure goes back to pending.
    pub reason: Option<BatchFailureReason>,
}

impl<T> BatchItem<T> {
    /// Is this item's non-answer a shared/backpressure condition (so the id must
    /// return to `pending`) rather than a deterministic fact about the id?
    pub fn is_backpressure(&self) -> bool {
        self.reason.is_some_and(BatchFailureReason::is_backpressure)
    }
}

/// One batch call's whole answer.
#[derive(Debug, Clone)]
pub struct BatchResolution<T> {
    /// The coordinator's declared schema version (already validated against
    /// [`BATCH_RESOLVE_SCHEMA_VERSION`] — a mismatch never reaches a caller,
    /// it triggers the fallback instead).
    pub schema_version: u16,
    /// Per-id answers, in the order the coordinator ATTEMPTED them. A caller's
    /// apply half stays the sequential body it always was.
    pub items: Vec<BatchItem<T>>,
    /// Ids NEVER STARTED (ceiling clamp, deadline, or behind a shared failure's
    /// stop point). **Not failures.** The caller returns them to `pending`.
    pub unattempted: Vec<String>,
    /// Why admission stopped short, or `None` if every id was attempted.
    pub stop_reason: Option<BatchStopReason>,
    /// In-wasm wall time the coordinator spent.
    pub elapsed_ms: u32,
    /// Observed round-trip MINUS [`Self::elapsed_ms`]. Saturating: a clock that
    /// makes this negative reports zero rather than wrapping.
    ///
    /// Kept as the DIAGNOSTIC it always was (`elohim_head_batch_queue_wait_ms`),
    /// but NO LONGER the AIMD controller's input — see [`Self::admission_wait`].
    pub queue_wait: Duration,
    /// How long this call queued at the LOCAL admission gate
    /// ([`crate::conductor_admission`]) before it was dispatched — the signal
    /// the AIMD controller folds.
    ///
    /// This replaced `queue_wait` as the control input because `queue_wait` is
    /// NOT monotonic in batch size: it subtracts the extern's own `elapsed_ms`,
    /// which is exactly where a saturated conductor's read-permit waiting is
    /// spent. A conductor stalling inside the extern therefore reported *zero*
    /// queue-wait and the controller read that as headroom and GREW the batch —
    /// a controller fed a non-monotonic error signal, not an underpowered one.
    /// Admission wait has the property the loop needs: bigger batches hold
    /// permits longer ⇒ occupancy rises ⇒ later calls queue longer.
    pub admission_wait: Duration,
    /// TRUE when this answer was assembled from single-id calls because the
    /// conductor cannot serve the batch externs.
    pub via_single_id_fallback: bool,
}

impl<T> BatchResolution<T> {
    /// The empty answer — used for an empty id list, so callers never special-case.
    pub fn empty() -> Self {
        Self {
            schema_version: BATCH_RESOLVE_SCHEMA_VERSION,
            items: Vec::new(),
            unattempted: Vec::new(),
            stop_reason: None,
            elapsed_ms: 0,
            queue_wait: Duration::ZERO,
            admission_wait: Duration::ZERO,
            via_single_id_fallback: false,
        }
    }

    /// Ids the coordinator answered `Failed` for with a BACKPRESSURE reason,
    /// plus every `unattempted` id — the full set that must go back to
    /// `pending` rather than be marked failed.
    ///
    /// **Concerns:** C3 (a deferral is never a terminality verdict).
    pub fn ids_to_return_to_pending(&self) -> Vec<String> {
        self.items
            .iter()
            .filter(|i| i.is_backpressure())
            .map(|i| i.id.clone())
            .chain(self.unattempted.iter().cloned())
            .collect()
    }
}

/// The seam. Both methods take the ids to ask about and the IN-WASM budget to
/// hand the coordinator; both return one [`BatchResolution`].
///
/// `Err` means a CALL-LEVEL infrastructure failure — the conductor never
/// delivered an answer of any kind. The contract on the caller is exact: return
/// EVERY id to `pending` with backoff, mark nothing failed, fall back to
/// nothing.
#[async_trait]
pub trait HeadBatchResolver: Send + Sync {
    /// Batched `content_store::resolve_content_heads_local`.
    async fn resolve_heads(
        &self,
        ids: &[String],
        budget: Duration,
    ) -> Result<BatchResolution<ContentHeadWire>, StorageError>;

    /// Batched `content_store::resolve_canonical_elections`.
    async fn resolve_elections(
        &self,
        ids: &[String],
        budget: Duration,
    ) -> Result<BatchResolution<CanonicalElectionWire>, StorageError>;
}

// ---------------------------------------------------------------------------
// Shared decoding of a coordinator batch output into the caller vocabulary
// ---------------------------------------------------------------------------

/// `queue_wait = observed RTT − in-wasm elapsed`, saturating at zero.
///
/// Pure so the arithmetic is testable without a conductor. Saturating rather
/// than signed: a monotonic-clock hiccup or a coordinator reporting more elapsed
/// time than the round-trip took must read as "no measurable queue wait", never
/// as a wrapped enormous one that would slam the AIMD controller to its floor.
///
/// **Contract test:** [`tests::queue_wait_is_rtt_minus_in_wasm_time`].
pub fn queue_wait_from(rtt: Duration, elapsed_ms: u32) -> Duration {
    rtt.saturating_sub(Duration::from_millis(u64::from(elapsed_ms)))
}

/// Turn one accepted [`conductor_writes::BatchCall`] into the caller vocabulary.
///
/// `rtt` comes from the call's own timing (measured FROM DISPATCH), not from a
/// wall clock started before the admission gate: including gate time in `rtt`
/// would inflate `queue_wait` with local queueing and quietly change what the
/// long-standing `elohim_head_batch_queue_wait_ms` series means.
fn resolution_from_call<T>(call: conductor_writes::BatchCall<T>) -> BatchResolution<T> {
    let conductor_writes::BatchCall { out, timing } = call;
    let queue_wait = queue_wait_from(timing.rtt, out.elapsed_ms);
    let items = out
        .attempted
        .into_iter()
        .map(|a| match a.outcome {
            // `Resolved(None)` is the C4-honest local absence — the coordinator
            // ANSWERED and holds nothing. Identical to the single-id twin.
            BatchOutcome::Resolved(v) => BatchItem {
                id: a.id,
                answer: Answer::observed_absence(v),
                reason: None,
            },
            // A per-item failure establishes NOTHING about the id, so it is
            // `Unreachable` — with the typed reason carried, never collapsed.
            BatchOutcome::Failed(f) => BatchItem {
                id: a.id,
                answer: Answer::Unreachable,
                reason: Some(f.reason),
            },
        })
        .collect();
    BatchResolution {
        schema_version: out.schema_version,
        items,
        unattempted: out.unattempted,
        stop_reason: out.stop_reason,
        elapsed_ms: out.elapsed_ms,
        queue_wait,
        admission_wait: timing.admission_wait,
        via_single_id_fallback: false,
    }
}

/// What to do with ONE raw batch-call result — the fallback rule as a PURE,
/// total function.
///
/// Pure on purpose: the fallback rule is BINDING and narrow (two triggers, no
/// more), and a rule enforced only inside an `async fn` that needs a live
/// conductor is a rule with no test. Every branch below is covered by
/// [`tests::the_fallback_rule_has_exactly_two_triggers`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatchCallDisposition {
    /// The coordinator answered in a shape this build understands.
    Accept,
    /// This conductor cannot speak the batch contract. Fall through to
    /// single-id ONCE and latch. The `&'static str` is the countable `why`.
    Fallback(&'static str),
    /// CALL-LEVEL infrastructure failure. No fallback, no failure-marking —
    /// every id goes back to pending with backoff.
    CallFailure,
}

/// Classify one raw batch-call result.
pub fn classify_batch_call<T>(
    result: &Result<conductor_writes::BatchCall<T>, StorageError>,
) -> BatchCallDisposition {
    match result {
        Ok(call) if call.out.schema_version != BATCH_RESOLVE_SCHEMA_VERSION => {
            BatchCallDisposition::Fallback("schema_version_mismatch")
        }
        Ok(_) => BatchCallDisposition::Accept,
        Err(e) if conductor_writes::is_unknown_function_error(e) => {
            BatchCallDisposition::Fallback("unknown_function")
        }
        Err(e) if is_undecodable_batch_output(e) => {
            BatchCallDisposition::Fallback("undecodable_output")
        }
        Err(_) => BatchCallDisposition::CallFailure,
    }
}

/// FALL THROUGH ONCE, LOG ONCE, NEVER RETRY-LOOP.
///
/// The latch half of the fallback rule, extracted so both halves — the trigger
/// (above) and the once-ness (here) — are provable without a conductor.
#[derive(Debug)]
pub struct FallbackLatch {
    latched: std::sync::atomic::AtomicBool,
    log_once: Once,
    logs_emitted: AtomicUsize,
}

impl Default for FallbackLatch {
    fn default() -> Self {
        Self {
            latched: std::sync::atomic::AtomicBool::new(false),
            log_once: Once::new(),
            logs_emitted: AtomicUsize::new(0),
        }
    }
}

impl FallbackLatch {
    /// TRUE once any trigger has fired. Every later call skips the batch extern
    /// entirely, which is what "never retry-loop the batch" means.
    pub fn is_latched(&self) -> bool {
        self.latched.load(Ordering::Relaxed)
    }

    /// How many times the one-time warning was actually emitted. Must never
    /// exceed 1, however many triggers fire.
    pub fn logs_emitted(&self) -> usize {
        self.logs_emitted.load(Ordering::Relaxed)
    }

    /// Latch, and log at most once across the process lifetime.
    pub fn latch(&self, extern_name: &str, why: &str) {
        self.latched.store(true, Ordering::Relaxed);
        self.log_once.call_once(|| {
            self.logs_emitted.fetch_add(1, Ordering::Relaxed);
            tracing::warn!(
                target: "elohim_storage::head_batch_resolver",
                extern_name,
                why,
                "head-batch: this conductor cannot serve the batch head-plane externs — \
                 falling through to the single-id path ONCE and LATCHING there for the rest \
                 of the process (never re-probed, never retry-looped). Expected before the \
                 coordinator hot-swap lands (the DNA hash is blind to coordinator zomes, so \
                 an unknown-function error is the only way to learn this)."
            );
        });
        crate::metrics::inc_head_batch_fallback(why);
    }
}

/// Is this the "the batch output could not be decoded at all" class?
///
/// Kept beside [`conductor_writes::is_unknown_function_error`] as the SECOND —
/// and last — fallback trigger. Deliberately narrow: it matches only the decode
/// failures THIS module's own wrappers produce, so an unrelated serialization
/// error elsewhere can never silently latch a node into single-id mode.
fn is_undecodable_batch_output(err: &StorageError) -> bool {
    matches!(err, StorageError::Serialization(msg) if msg.contains("decode BatchResolveOutput"))
}

// ---------------------------------------------------------------------------
// Production implementation
// ---------------------------------------------------------------------------

/// Production resolver — calls the coordinator's batch externs, with the
/// latched single-id fallback for a conductor that cannot serve them.
pub struct ConductorHeadBatchResolver {
    hc: Arc<crate::hc_client::HcClient>,
    /// Latched the first time this conductor proves it cannot speak the batch
    /// contract. Once set, every later call goes straight to single-id — the
    /// "never retry-loop the batch" half of the fallback rule.
    ///
    /// PROCESS-LIFETIME on purpose. A coordinator hot-swap that lands WITHOUT a
    /// storage restart leaves this node in single-id mode until it restarts;
    /// that is the conservative direction (single-id is exactly today's proven
    /// behaviour), and every rollout path that swaps coordinators also restarts
    /// the edge pods. The alternative — re-probing the batch extern every sweep
    /// — is the retry-loop the rule forbids.
    latch: FallbackLatch,
}

impl ConductorHeadBatchResolver {
    pub fn new(hc: Arc<crate::hc_client::HcClient>) -> Self {
        Self {
            hc,
            latch: FallbackLatch::default(),
        }
    }

    /// TRUE once this resolver has latched into single-id mode.
    /// Observability and test assertion only.
    pub fn in_single_id_mode(&self) -> bool {
        self.latch.is_latched()
    }

    async fn fallback_heads(&self, ids: &[String]) -> BatchResolution<ContentHeadWire> {
        let started = Instant::now();
        let mut items = Vec::with_capacity(ids.len());
        for id in ids {
            // SEQUENTIAL on purpose: the caller's fan-out already bounds how many
            // of these loops run at once, and a concurrent fallback would raise
            // the concurrency the write-guard constraint forbids raising.
            let answer = match conductor_writes::call_resolve_content_head_local(&self.hc, id).await
            {
                Ok(v) => BatchItem {
                    id: id.clone(),
                    answer: Answer::observed_absence(v),
                    reason: None,
                },
                Err(e) => BatchItem {
                    id: id.clone(),
                    answer: Answer::Unreachable,
                    reason: Some(single_id_error_reason(&e)),
                },
            };
            items.push(answer);
        }
        fallback_resolution(items, started)
    }

    async fn fallback_elections(&self, ids: &[String]) -> BatchResolution<CanonicalElectionWire> {
        let started = Instant::now();
        let mut items = Vec::with_capacity(ids.len());
        for id in ids {
            let answer = match conductor_writes::call_resolve_canonical_election(&self.hc, id).await
            {
                Ok(v) => BatchItem {
                    id: id.clone(),
                    answer: Answer::observed_absence(v),
                    reason: None,
                },
                Err(e) => BatchItem {
                    id: id.clone(),
                    answer: Answer::Unreachable,
                    reason: Some(single_id_error_reason(&e)),
                },
            };
            items.push(answer);
        }
        fallback_resolution(items, started)
    }
}

/// Classify a SINGLE-ID call's error into the same typed vocabulary the batch
/// extern uses, so the fallback path routes identically to the batch path.
///
/// Everything here is BACKPRESSURE-classed: a single-id call that errored told
/// us nothing about the id, exactly like a shared batch failure. Only the
/// coordinator can produce the deterministic reasons (`WrongContentId`,
/// `RecordShape`), because only it can see the record it refused.
fn single_id_error_reason(err: &StorageError) -> BatchFailureReason {
    let text = err.to_string().to_ascii_lowercase();
    if text.contains("deadline has elapsed") {
        BatchFailureReason::PermitTimeout
    } else if text.contains("timeout") || text.contains("websocket") || text.contains("transport") {
        BatchFailureReason::Transport
    } else {
        BatchFailureReason::Unknown
    }
}

fn fallback_resolution<T>(items: Vec<BatchItem<T>>, started: Instant) -> BatchResolution<T> {
    let rtt = started.elapsed();
    BatchResolution {
        schema_version: BATCH_RESOLVE_SCHEMA_VERSION,
        items,
        unattempted: Vec::new(),
        stop_reason: None,
        // The single-id path has no in-wasm clock, so the whole wall time reads
        // as queue wait. That is the HONEST reading for the AIMD controller: a
        // node in fallback mode is paying one round-trip per id and must not be
        // encouraged to ask for bigger batches.
        //
        // `admission_wait` (now the control input) carries the SAME whole wall
        // time for the same reason. Reporting the per-call gate waits — or worse,
        // zero — would read as headroom and grow the batch on the one path where
        // a bigger batch buys nothing at all.
        elapsed_ms: 0,
        queue_wait: rtt,
        admission_wait: rtt,
        via_single_id_fallback: true,
    }
}

#[async_trait]
impl HeadBatchResolver for ConductorHeadBatchResolver {
    async fn resolve_heads(
        &self,
        ids: &[String],
        budget: Duration,
    ) -> Result<BatchResolution<ContentHeadWire>, StorageError> {
        if ids.is_empty() {
            return Ok(BatchResolution::empty());
        }
        if self.in_single_id_mode() {
            return Ok(self.fallback_heads(ids).await);
        }
        let budget_ms = u32::try_from(budget.as_millis()).unwrap_or(u32::MAX);
        // No wall clock started here: the call carries its own timing, measured
        // from DISPATCH so the admission-gate wait never leaks into `rtt`.
        let raw =
            conductor_writes::call_resolve_content_heads_local(&self.hc, ids, Some(budget_ms))
                .await;
        match classify_batch_call(&raw) {
            BatchCallDisposition::Accept => {
                Ok(resolution_from_call(raw.expect("Accept implies Ok")))
            }
            BatchCallDisposition::Fallback(why) => {
                self.latch.latch("resolve_content_heads_local", why);
                Ok(self.fallback_heads(ids).await)
            }
            // CALL-LEVEL INFRASTRUCTURE FAILURE. No fallback, no failure-marking:
            // the caller returns every id to pending with backoff.
            BatchCallDisposition::CallFailure => Err(raw.expect_err("CallFailure implies Err")),
        }
    }

    async fn resolve_elections(
        &self,
        ids: &[String],
        budget: Duration,
    ) -> Result<BatchResolution<CanonicalElectionWire>, StorageError> {
        if ids.is_empty() {
            return Ok(BatchResolution::empty());
        }
        if self.in_single_id_mode() {
            return Ok(self.fallback_elections(ids).await);
        }
        let budget_ms = u32::try_from(budget.as_millis()).unwrap_or(u32::MAX);
        // See `resolve_heads` — the call carries its own dispatch-relative timing.
        let raw =
            conductor_writes::call_resolve_canonical_elections(&self.hc, ids, Some(budget_ms))
                .await;
        match classify_batch_call(&raw) {
            BatchCallDisposition::Accept => {
                Ok(resolution_from_call(raw.expect("Accept implies Ok")))
            }
            BatchCallDisposition::Fallback(why) => {
                self.latch.latch("resolve_canonical_elections", why);
                Ok(self.fallback_elections(ids).await)
            }
            BatchCallDisposition::CallFailure => Err(raw.expect_err("CallFailure implies Err")),
        }
    }
}

// ---------------------------------------------------------------------------
// Mock (NOT behind cfg(test) — the commitment_fetcher template)
// ---------------------------------------------------------------------------

/// What a [`MockHeadBatchResolver`] should do with one id.
#[derive(Debug, Clone)]
pub enum MockAnswer<T> {
    /// The coordinator answered with a head/election.
    Present(T),
    /// The coordinator answered "nothing here" (`Resolved(None)`).
    Absent,
    /// The coordinator recorded a typed per-item failure.
    Failed(BatchFailureReason),
    /// The coordinator never started this id — it lands in `unattempted`.
    Unattempted,
}

/// Scripted in-memory [`HeadBatchResolver`] for arm tests.
///
/// Two independent scripting layers, both optional:
///
/// - **per-id answers** (`seed_head` / `seed_election`) — the default for any id
///   not named is [`MockAnswer::Absent`], which is the live corpus's dominant
///   outcome;
/// - **call-level failures** (`fail_next_call`) — a queue of infrastructure
///   errors, consumed one per call, for the "all ids back to pending" trap.
///
/// It also records peak concurrency so a fan-out ceiling can be PROVEN rather
/// than asserted (the `resolve_pipeline` contract-test pattern).
pub struct MockHeadBatchResolver {
    heads: Mutex<HashMap<String, MockAnswer<ContentHeadWire>>>,
    elections: Mutex<HashMap<String, MockAnswer<CanonicalElectionWire>>>,
    call_failures: Mutex<Vec<StorageError>>,
    calls: AtomicUsize,
    ids_asked: AtomicUsize,
    in_flight: AtomicUsize,
    peak_in_flight: AtomicUsize,
    /// Per-call synthetic latency, so a concurrency probe has a window to
    /// observe overlap in.
    latency: Mutex<Duration>,
    /// Reported by every scripted resolution.
    queue_wait: Mutex<Duration>,
}

impl Default for MockHeadBatchResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl MockHeadBatchResolver {
    pub fn new() -> Self {
        Self {
            heads: Mutex::new(HashMap::new()),
            elections: Mutex::new(HashMap::new()),
            call_failures: Mutex::new(Vec::new()),
            calls: AtomicUsize::new(0),
            ids_asked: AtomicUsize::new(0),
            in_flight: AtomicUsize::new(0),
            peak_in_flight: AtomicUsize::new(0),
            latency: Mutex::new(Duration::from_millis(5)),
            queue_wait: Mutex::new(Duration::ZERO),
        }
    }

    pub fn seed_head(&self, id: &str, answer: MockAnswer<ContentHeadWire>) {
        self.heads.lock().unwrap().insert(id.to_string(), answer);
    }

    pub fn seed_election(&self, id: &str, answer: MockAnswer<CanonicalElectionWire>) {
        self.elections
            .lock()
            .unwrap()
            .insert(id.to_string(), answer);
    }

    /// Queue a CALL-LEVEL infrastructure failure (consumed FIFO, one per call).
    pub fn fail_next_call(&self, err: StorageError) {
        self.call_failures.lock().unwrap().push(err);
    }

    pub fn set_latency(&self, latency: Duration) {
        *self.latency.lock().unwrap() = latency;
    }

    pub fn set_queue_wait(&self, queue_wait: Duration) {
        *self.queue_wait.lock().unwrap() = queue_wait;
    }

    pub fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }

    pub fn ids_asked(&self) -> usize {
        self.ids_asked.load(Ordering::SeqCst)
    }

    pub fn peak_in_flight(&self) -> usize {
        self.peak_in_flight.load(Ordering::SeqCst)
    }

    fn enter(&self) -> Option<StorageError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let n = self.in_flight.fetch_add(1, Ordering::SeqCst) + 1;
        self.peak_in_flight.fetch_max(n, Ordering::SeqCst);
        let mut failures = self.call_failures.lock().unwrap();
        if failures.is_empty() {
            None
        } else {
            Some(failures.remove(0))
        }
    }

    fn leave(&self) {
        self.in_flight.fetch_sub(1, Ordering::SeqCst);
    }

    fn build<T: Clone>(
        &self,
        ids: &[String],
        table: &HashMap<String, MockAnswer<T>>,
    ) -> BatchResolution<T> {
        let mut items = Vec::new();
        let mut unattempted = Vec::new();
        for id in ids {
            match table.get(id).cloned().unwrap_or(MockAnswer::Absent) {
                MockAnswer::Present(v) => items.push(BatchItem {
                    id: id.clone(),
                    answer: Answer::Present(v),
                    reason: None,
                }),
                MockAnswer::Absent => items.push(BatchItem {
                    id: id.clone(),
                    answer: Answer::Absent,
                    reason: None,
                }),
                MockAnswer::Failed(reason) => items.push(BatchItem {
                    id: id.clone(),
                    answer: Answer::Unreachable,
                    reason: Some(reason),
                }),
                MockAnswer::Unattempted => unattempted.push(id.clone()),
            }
        }
        let stop_reason = (!unattempted.is_empty()).then_some(BatchStopReason::BudgetExhausted);
        // Locked ONCE. `std::sync::Mutex` is not reentrant, and both temporaries
        // in a struct literal live to the end of the statement — so taking this
        // lock twice inside the initializer below self-deadlocks.
        let scripted_pressure = *self.queue_wait.lock().unwrap();
        BatchResolution {
            schema_version: BATCH_RESOLVE_SCHEMA_VERSION,
            items,
            unattempted,
            stop_reason,
            elapsed_ms: 1,
            // ONE scripted pressure knob feeds BOTH fields. The mock's
            // `queue_wait` setter predates the admission gate and is what arm
            // tests use to script "this conductor is backpressured"; mirroring it
            // into `admission_wait` keeps that intent driving the AIMD controller
            // now that the controller reads the admission signal instead.
            queue_wait: scripted_pressure,
            admission_wait: scripted_pressure,
            via_single_id_fallback: false,
        }
    }
}

#[async_trait]
impl HeadBatchResolver for MockHeadBatchResolver {
    async fn resolve_heads(
        &self,
        ids: &[String],
        _budget: Duration,
    ) -> Result<BatchResolution<ContentHeadWire>, StorageError> {
        // Mirror the production short-circuit: an empty batch costs no
        // round-trip, so a mock that counted one would let an arm regress into
        // asking the conductor about nothing.
        if ids.is_empty() {
            return Ok(BatchResolution::empty());
        }
        self.ids_asked.fetch_add(ids.len(), Ordering::SeqCst);
        let failure = self.enter();
        let latency = *self.latency.lock().unwrap();
        tokio::time::sleep(latency).await;
        let out = match failure {
            Some(e) => Err(e),
            None => Ok(self.build(ids, &self.heads.lock().unwrap())),
        };
        self.leave();
        out
    }

    async fn resolve_elections(
        &self,
        ids: &[String],
        _budget: Duration,
    ) -> Result<BatchResolution<CanonicalElectionWire>, StorageError> {
        // Mirror the production short-circuit: an empty batch costs no
        // round-trip, so a mock that counted one would let an arm regress into
        // asking the conductor about nothing.
        if ids.is_empty() {
            return Ok(BatchResolution::empty());
        }
        self.ids_asked.fetch_add(ids.len(), Ordering::SeqCst);
        let failure = self.enter();
        let latency = *self.latency.lock().unwrap();
        tokio::time::sleep(latency).await;
        let out = match failure {
            Some(e) => Err(e),
            None => Ok(self.build(ids, &self.elections.lock().unwrap())),
        };
        self.leave();
        out
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::conductor_writes::{BatchAttempt, BatchResolveFailure, BatchResolvePhase};

    fn ids(n: usize) -> Vec<String> {
        (0..n).map(|i| format!("id-{i}")).collect()
    }

    #[test]
    fn queue_wait_is_rtt_minus_in_wasm_time() {
        // The whole point of the signal: a 6s round-trip on a 500ms extern is
        // 5.5s of QUEUE WAIT, not 6s of work.
        assert_eq!(
            queue_wait_from(Duration::from_millis(6_000), 500),
            Duration::from_millis(5_500)
        );
        // A coordinator claiming MORE elapsed time than the round-trip took
        // (clock skew) must read as zero, never wrap.
        assert_eq!(
            queue_wait_from(Duration::from_millis(100), 900),
            Duration::ZERO
        );
    }

    /// Wrap a raw wire output with the admission timing a real call would carry.
    /// `rtt` is dispatch-relative, exactly as [`crate::hc_client::ZomeCallTiming`]
    /// reports it.
    fn call_of<T>(
        out: conductor_writes::BatchResolveOutput<T>,
        rtt: Duration,
        admission_wait: Duration,
    ) -> conductor_writes::BatchCall<T> {
        conductor_writes::BatchCall {
            out,
            timing: crate::hc_client::ZomeCallTiming {
                admission_wait,
                rtt,
            },
        }
    }

    /// WHY `queue_wait` is not the controller's input, stated as arithmetic.
    ///
    /// The saturation case is a conductor that spends its whole round-trip
    /// blocked on a DB read permit INSIDE the extern. That waiting is counted in
    /// the extern's own `elapsed_ms`, so subtracting it reports near-zero
    /// queue-wait — a signal that says "plenty of headroom" at the exact moment
    /// there is none, and drives the AIMD controller to grow the batch. This is
    /// the sign inversion the admission signal replaced; the metric stays as a
    /// diagnostic, but nothing closes a loop over it.
    #[test]
    fn queue_wait_reads_as_headroom_when_the_conductor_stalls_in_the_extern() {
        // 9s round-trip, of which the extern reports 8.9s in-wasm — nearly all of
        // it spent waiting on a read permit it could not get.
        let signal = queue_wait_from(Duration::from_millis(9_000), 8_900);
        assert_eq!(signal, Duration::from_millis(100));
        assert!(
            signal < crate::p2p::reconcile_rails::BATCH_QUEUE_WAIT_THRESHOLD,
            "a badly saturated conductor reads as UNDER the backpressure threshold"
        );
        // Fed to the control law, that reads as room to grow.
        assert_eq!(
            crate::p2p::reconcile_rails::AdaptiveBatchBudget::next_size(
                32,
                signal,
                crate::p2p::reconcile_rails::BATCH_QUEUE_WAIT_THRESHOLD,
                8,
                128,
                16,
            ),
            48,
            "the old signal grows the batch against a stalled conductor"
        );
    }

    /// The contract-revision mapping, in one test: `Resolved(Some)` → `Present`,
    /// `Resolved(None)` → `Absent` (the honest local absence), `Failed` →
    /// `Unreachable` WITH the typed reason. A `Failed` mapped to `Absent` would
    /// let a poisoned link be read as proof the id does not exist.
    #[test]
    fn per_item_outcomes_map_onto_the_c4_vocabulary() {
        #[derive(Debug, Clone, PartialEq, Eq)]
        struct P(u8);
        let wire = conductor_writes::BatchResolveOutput {
            schema_version: BATCH_RESOLVE_SCHEMA_VERSION,
            attempted: vec![
                BatchAttempt {
                    id: "a".into(),
                    outcome: BatchOutcome::Resolved(Some(P(1))),
                },
                BatchAttempt {
                    id: "b".into(),
                    outcome: BatchOutcome::Resolved(None),
                },
                BatchAttempt {
                    id: "c".into(),
                    outcome: BatchOutcome::Failed(BatchResolveFailure {
                        reason: BatchFailureReason::WrongContentId,
                        phase: BatchResolvePhase::HeadResolve,
                    }),
                },
                BatchAttempt {
                    id: "d".into(),
                    outcome: BatchOutcome::Failed(BatchResolveFailure {
                        reason: BatchFailureReason::PermitTimeout,
                        phase: BatchResolvePhase::HeadResolve,
                    }),
                },
            ],
            unattempted: vec!["e".into()],
            stop_reason: Some(BatchStopReason::SharedFailure(
                BatchFailureReason::PermitTimeout,
            )),
            elapsed_ms: 250,
        };

        let r = resolution_from_call(call_of(
            wire,
            Duration::from_millis(1_000),
            Duration::from_millis(40),
        ));
        // Both signals ride along, and they are NOT the same number: 1000ms RTT
        // minus 250ms in-wasm is the diagnostic; 40ms at the gate is the control
        // input.
        assert_eq!(r.queue_wait, Duration::from_millis(750));
        assert_eq!(r.admission_wait, Duration::from_millis(40));

        assert_eq!(r.items.len(), 4);
        assert_eq!(r.items[0].answer, Answer::Present(P(1)));
        assert!(r.items[0].reason.is_none());
        assert_eq!(r.items[1].answer, Answer::Absent);
        assert_eq!(r.items[2].answer, Answer::Unreachable);
        assert_eq!(r.items[2].reason, Some(BatchFailureReason::WrongContentId));
        assert!(
            !r.items[2].is_backpressure(),
            "WrongContentId is DETERMINISTIC — it is a real fact about the id"
        );
        assert!(
            r.items[3].is_backpressure(),
            "PermitTimeout is a SATURATED READ POOL — it says nothing about the id"
        );
        assert_eq!(r.unattempted, vec!["e".to_string()]);
        assert_eq!(r.queue_wait, Duration::from_millis(750));
        assert!(!r.via_single_id_fallback);

        // The set the caller must return to pending: the backpressure item plus
        // every unattempted id — and NOT the deterministic failure.
        let pending = r.ids_to_return_to_pending();
        assert_eq!(pending, vec!["d".to_string(), "e".to_string()]);
    }

    /// TRAP: `unattempted` ids are NEVER FAILURES. They must appear in the
    /// return-to-pending set and never anywhere else. Marking them failed burns
    /// `MAX_RETRIES` and poisons the `MissLedger` for ids the conductor never
    /// even looked at.
    #[tokio::test]
    async fn unattempted_ids_are_returned_to_pending_never_failed() {
        let mock = MockHeadBatchResolver::new();
        mock.seed_head("id-2", MockAnswer::Unattempted);
        mock.seed_head("id-3", MockAnswer::Unattempted);
        let all = ids(4);

        let r = mock
            .resolve_heads(&all, BATCH_EXTERN_BUDGET)
            .await
            .expect("mock must answer");

        assert_eq!(r.items.len(), 2, "only the started ids get an item");
        assert_eq!(
            r.unattempted,
            vec!["id-2".to_string(), "id-3".to_string()],
            "never-started ids stay in `unattempted`"
        );
        assert!(r.stop_reason.is_some());
        assert!(r
            .ids_to_return_to_pending()
            .iter()
            .all(|id| id == "id-2" || id == "id-3"));
    }

    /// TRAP: a CALL-LEVEL failure returns `Err`. It never falls back, and the
    /// caller's contract is "all ids back to pending".
    #[tokio::test]
    async fn a_call_level_failure_is_an_error_not_a_fallback() {
        let mock = MockHeadBatchResolver::new();
        mock.fail_next_call(StorageError::Timeout("Websocket error: Timeout".into()));
        let err = mock
            .resolve_heads(&ids(8), BATCH_EXTERN_BUDGET)
            .await
            .expect_err("a call-level failure must surface as Err");
        assert!(err.to_string().contains("Timeout"));
        assert_eq!(mock.calls(), 1, "and must NOT be retried inside the seam");
    }

    /// An empty id list is a no-op that costs no round-trip — the caller never
    /// has to special-case it.
    #[tokio::test]
    async fn an_empty_batch_costs_no_round_trip() {
        let mock = MockHeadBatchResolver::new();
        let r = mock
            .resolve_heads(&[], BATCH_EXTERN_BUDGET)
            .await
            .expect("empty is fine");
        assert!(r.items.is_empty() && r.unattempted.is_empty());
        assert_eq!(mock.calls(), 0);
    }

    /// A single-id fallback classifies its own errors into the SAME typed
    /// vocabulary, all of it backpressure-classed: a single-id call that errored
    /// told us nothing about the id.
    #[test]
    fn single_id_errors_classify_as_backpressure() {
        assert_eq!(
            single_id_error_reason(&StorageError::Internal(
                "WasmError Host(\"deadline has elapsed\")".into()
            )),
            BatchFailureReason::PermitTimeout
        );
        assert_eq!(
            single_id_error_reason(&StorageError::Timeout("Websocket error: Timeout".into())),
            BatchFailureReason::Transport
        );
        assert_eq!(
            single_id_error_reason(&StorageError::Internal("something else".into())),
            BatchFailureReason::Unknown
        );
        for e in [
            StorageError::Internal("WasmError Host(\"deadline has elapsed\")".into()),
            StorageError::Timeout("Websocket error: Timeout".into()),
            StorageError::Internal("something else".into()),
        ] {
            assert!(
                single_id_error_reason(&e).is_backpressure(),
                "every single-id call error must route back to pending, never blame the id"
            );
        }
    }

    /// Only the two licensed triggers are undecodable-classed; an unrelated
    /// serialization error must NOT latch a node into single-id mode.
    #[test]
    fn only_our_own_batch_decode_failure_counts_as_undecodable() {
        assert!(is_undecodable_batch_output(&StorageError::Serialization(
            "conductor_writes: decode BatchResolveOutput<ContentHeadWire>: bad".into()
        )));
        assert!(!is_undecodable_batch_output(&StorageError::Serialization(
            "conductor_writes: decode ContentHeadWire (resolve local): bad".into()
        )));
        assert!(!is_undecodable_batch_output(&StorageError::Internal(
            "decode BatchResolveOutput".into()
        )));
    }

    /// THE FALLBACK RULE, exhaustively: exactly TWO trigger classes, and
    /// everything else is a call-level failure that must NOT fall back.
    ///
    /// The negative half is the load-bearing one. Falling back on a saturated
    /// conductor turns ONE refused batch into `batch_size` single-id calls
    /// against a pool that is already refusing work — the amplification this
    /// whole program exists to remove.
    #[test]
    fn the_fallback_rule_has_exactly_two_triggers() {
        type R = Result<conductor_writes::BatchCall<u8>, StorageError>;
        let empty = |schema_version| conductor_writes::BatchResolveOutput::<u8> {
            schema_version,
            attempted: Vec::new(),
            unattempted: Vec::new(),
            stop_reason: None,
            elapsed_ms: 0,
        };

        let good: R = Ok(call_of(
            empty(BATCH_RESOLVE_SCHEMA_VERSION),
            Duration::ZERO,
            Duration::ZERO,
        ));
        assert_eq!(classify_batch_call(&good), BatchCallDisposition::Accept);

        // TRIGGER 1 — schema_version mismatch.
        let skewed: R = Ok(call_of(
            empty(BATCH_RESOLVE_SCHEMA_VERSION + 1),
            Duration::ZERO,
            Duration::ZERO,
        ));
        assert_eq!(
            classify_batch_call(&skewed),
            BatchCallDisposition::Fallback("schema_version_mismatch")
        );

        // TRIGGER 2a — unknown function (the pre-hot-swap conductor).
        let unknown: R = Err(StorageError::Internal(
            "Zome call failed: Zome function resolve_content_heads_local does not exist".into(),
        ));
        assert_eq!(
            classify_batch_call(&unknown),
            BatchCallDisposition::Fallback("unknown_function")
        );

        // TRIGGER 2b — undecodable output.
        let undecodable: R = Err(StorageError::Serialization(
            "conductor_writes: decode BatchResolveOutput<ContentHeadWire>: bad".into(),
        ));
        assert_eq!(
            classify_batch_call(&undecodable),
            BatchCallDisposition::Fallback("undecodable_output")
        );

        // NOT triggers — every one of these is a call-level failure.
        for e in [
            StorageError::Timeout("Websocket error: Timeout".into()),
            StorageError::Internal("WasmError Host(\"deadline has elapsed\")".into()),
            StorageError::Connection("conductor unreachable".into()),
            StorageError::Serialization("conductor_writes: decode ContentHeadWire: bad".into()),
        ] {
            let r: R = Err(e);
            assert_eq!(
                classify_batch_call(&r),
                BatchCallDisposition::CallFailure,
                "backpressure/transport must NEVER license the fallback"
            );
        }
    }

    /// ONCE, and only once: however many triggers fire, the warning is emitted
    /// exactly one time and the resolver stays latched — it never re-probes the
    /// batch extern, which is the "never retry-loop" half of the rule.
    #[test]
    fn the_fallback_latches_once_and_logs_once() {
        let latch = FallbackLatch::default();
        assert!(!latch.is_latched());
        assert_eq!(latch.logs_emitted(), 0);

        latch.latch("resolve_content_heads_local", "unknown_function");
        assert!(latch.is_latched());
        assert_eq!(latch.logs_emitted(), 1);

        // Two more triggers (a different extern, a different why) must not
        // produce a second log line, and must not un-latch.
        latch.latch("resolve_canonical_elections", "schema_version_mismatch");
        latch.latch("resolve_content_heads_local", "undecodable_output");
        assert!(latch.is_latched());
        assert_eq!(
            latch.logs_emitted(),
            1,
            "a latched fallback must log exactly once for the life of the process"
        );
    }

    /// The mock's fan-out probe works, so the arm tests that use it are
    /// measuring something real.
    #[tokio::test]
    async fn the_mock_records_peak_concurrency() {
        use futures::stream::StreamExt as _;
        let mock = Arc::new(MockHeadBatchResolver::new());
        mock.set_latency(Duration::from_millis(20));
        let chunks: Vec<Vec<String>> = (0..8).map(|i| vec![format!("id-{i}")]).collect();
        futures::stream::iter(chunks.iter())
            .map(|c| {
                let mock = mock.clone();
                async move {
                    let _ = mock.resolve_heads(c, BATCH_EXTERN_BUDGET).await;
                }
            })
            .buffered(2)
            .collect::<Vec<_>>()
            .await;
        assert_eq!(mock.calls(), 8);
        assert!(mock.peak_in_flight() <= 2, "peak {}", mock.peak_in_flight());
        assert!(mock.peak_in_flight() > 1, "the probe never overlapped");
    }
}
