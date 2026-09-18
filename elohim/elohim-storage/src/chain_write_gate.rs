//! CHAIN WRITE GATE — one source chain, one writer at a time, per cell.
//!
//! ## The failure this closes
//!
//! A Holochain cell has exactly ONE source chain, and a commit is an optimistic
//! compare-and-flush against its head. Any bundle that *began* on a head which
//! has since moved is refused whole:
//!
//! ```text
//! Source chain error: Attempted to commit a bundle to the source chain,
//! but the source chain head has moved since the bundle began
//! ```
//!
//! elohim-storage authors on its own cell from at least six independent tasks —
//! the reconcile sweep's adopt-before-author declare, the sync-triggered head
//! adoption worker, the re-anchor backfill, provide-reconcile's commons author,
//! the REA/ordered-signal path, and HTTP-driven author writes. None of them knew
//! about the others, so every one of them was racing the other five *inside a
//! single process*, and the loser surfaced the conductor's refusal verbatim to
//! its caller (measured 2026-09-17: `HTTP 502 {"error":"Conductor error: … head
//! has moved…"}` on a peer's own declare; `HTTP 503` on `seed-commitments`).
//!
//! Losing a race against *ourselves* is not a distributed-systems fact, it is a
//! missing mutual exclusion. Losing one against an EXTERNAL writer (a fixture,
//! the steward's app authoring on the same cell) IS a real race — but a
//! `HeadMoved` failure commits nothing, so the identical call can simply be
//! re-offered. Neither belongs in a caller's error.
//!
//! ## Shape
//!
//! [`dispatch`] is the single choke point. Every zome call
//! [`crate::hc_client::HcClient`] makes passes through it:
//!
//! * a READ ([`is_chain_write`] says no) goes straight through — no lock, no
//!   metric, byte-identical to the pre-gate path;
//! * a WRITE takes the per-chain async mutex, makes ONE conductor call while
//!   holding it, and releases.
//!
//! The lock is held across nothing but the conductor call itself — not across
//! admission acquisition (the permit is taken first, so capacity and chain
//! exclusivity nest in that order), not across a backoff sleep. A `HeadMoved`
//! retry therefore RE-QUEUES rather than camping on the lock.
//!
//! ## Fairness, and the bound on it
//!
//! `tokio::sync::Mutex` is FIFO-fair. An `Interactive` writer that arrives while
//! a `Background` batch is running therefore waits for AT MOST the one in-flight
//! conductor call plus whatever was already queued ahead of it — never for the
//! whole batch, because a batch holds the lock per CALL, not per batch. That is
//! the bound; there is no priority lane and this module deliberately does not
//! add one (a priority mutex would need its own starvation story, and the
//! measured hold is a single zome call). The admission gate
//! ([`crate::conductor_admission`]) keeps its own class priority and is
//! unchanged — it runs FIRST, so a shed still costs the conductor nothing and a
//! writer never holds chain exclusivity while queued for capacity.
//!
//! ## Why the classifier defaults to WRITE
//!
//! Guessing "read" for a function that commits loses the guarantee silently.
//! Guessing "write" for a function that only reads costs a little serialization
//! and nothing else. So [`is_chain_write`] is an allowlist of READ prefixes and
//! everything else is a write. The allowlist was read off the DNA, not assumed:
//! `verify_carried_election` creates a link and is deliberately NOT covered by a
//! `verify_` prefix, while `validate_carried_head_record`, `resolve_content_head`
//! and `resolve_canonical_election(s)` were each confirmed commit-free.
//!
//! ## What this is NOT
//!
//! Not a timeout, not a deadline extension, and not a cross-process lock. Two
//! storage processes on one cell (which the protocol does not do) would still
//! race, and that race would be absorbed by the bounded retry below exactly as
//! an external writer's is.

use std::collections::HashMap;
use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use holochain_client::CellId;
use tokio::time::Instant;

use crate::error::StorageError;

// =============================================================================
// Writer vocabulary
// =============================================================================

/// The CLOSED vocabulary of in-process chain writers, used as the single metric
/// label on this module's series.
///
/// Closed on purpose: a metric label that can take a content id (or any
/// unbounded value) is a cardinality bomb in a Prometheus scrape. A writer with
/// no entry here reports as [`WriterKind::Other`] — which is a legible answer,
/// not a gap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriterKind {
    /// The reconcile sweep's adopt-before-author declare leg.
    SweepAdopt,
    /// The sync-triggered head-adoption worker
    /// ([`crate::services::head_adoption_trigger`]).
    TriggerAdopt,
    /// [`crate::services::reanchor_backfill`] re-authoring a dead/NULL anchor.
    Reanchor,
    /// Provide-reconcile's commons author.
    Provide,
    /// The REA ledger / ordered-signal author path.
    ReaSignal,
    /// An operator-driven HTTP write (PATCH /db/content → declare, and friends).
    HttpAuthor,
    /// Anything that did not name itself.
    Other,
}

impl WriterKind {
    /// Every variant, for metric pre-touch.
    pub const ALL: [WriterKind; 7] = [
        WriterKind::SweepAdopt,
        WriterKind::TriggerAdopt,
        WriterKind::Reanchor,
        WriterKind::Provide,
        WriterKind::ReaSignal,
        WriterKind::HttpAuthor,
        WriterKind::Other,
    ];

    /// The metric label. Stable wire vocabulary — do not rename without moving
    /// the dashboards that read it.
    pub fn label(self) -> &'static str {
        match self {
            WriterKind::SweepAdopt => "sweep_adopt",
            WriterKind::TriggerAdopt => "trigger_adopt",
            WriterKind::Reanchor => "reanchor",
            WriterKind::Provide => "provide",
            WriterKind::ReaSignal => "rea_signal",
            WriterKind::HttpAuthor => "http_author",
            WriterKind::Other => "other",
        }
    }
}

tokio::task_local! {
    /// Which writer the CURRENT task is acting as.
    ///
    /// A task-local rather than a parameter because the alternative is threading
    /// a label through a dozen `conductor_writes` helpers and their callers — a
    /// crate-wide signature change for an observability field. It does NOT cross
    /// `tokio::spawn`, so a writer that spawns must set it inside the spawned
    /// future; the default is [`WriterKind::Other`], so forgetting mislabels but
    /// never breaks.
    static WRITER_KIND: WriterKind;

    /// A deadline already owned by the caller, which this module may only ever
    /// SHORTEN its retry budget against — never extend.
    static WRITE_DEADLINE: Instant;
}

/// Run `fut` labelled as `kind`.
pub async fn as_writer<F: Future>(kind: WriterKind, fut: F) -> F::Output {
    WRITER_KIND.scope(kind, fut).await
}

/// Run `fut` under a caller-owned deadline. The gate never retries past it.
pub async fn with_write_deadline<F: Future>(deadline: Instant, fut: F) -> F::Output {
    WRITE_DEADLINE.scope(deadline, fut).await
}

/// The current task's writer kind, or [`WriterKind::Other`].
pub fn current_writer_kind() -> WriterKind {
    WRITER_KIND.try_with(|k| *k).unwrap_or(WriterKind::Other)
}

/// The current task's caller-owned deadline, if one is in scope.
pub fn current_write_deadline() -> Option<Instant> {
    WRITE_DEADLINE.try_with(|d| *d).ok()
}

// =============================================================================
// Write / read classification
// =============================================================================

/// Coordinator-function name prefixes that only ever READ.
///
/// Read off the live DNA (`content_store`, `imagodei`, `mishpat` coordinators),
/// not inferred from English. Anything not matching is treated as a write — see
/// the module doc for why that asymmetry is the safe one.
const READ_FN_PREFIXES: [&str; 7] = [
    "get_",
    "list_",
    "query_",
    "resolve_",
    "find_",
    "validate_",
    "count_",
];

/// Does this coordinator call commit to the source chain?
///
/// `zome` is accepted (and currently unused) so a future per-zome exception can
/// land without touching every call site.
pub fn is_chain_write(_zome: &str, fn_name: &str) -> bool {
    !READ_FN_PREFIXES
        .iter()
        .any(|prefix| fn_name.starts_with(prefix))
}

/// Exactly the source-chain `HeadMoved` class, and nothing else.
///
/// Both spellings are real: the conductor surfaces `SourceChainError::HeadMoved`
/// from some paths and the prose "source chain head has moved …" from others.
/// The [`StorageError::Conductor`] discriminant is part of the match on purpose
/// — the same words arriving as an `InvalidInput` are a caller's payload, not a
/// race, and replaying them would be a bug.
pub fn is_source_chain_head_moved(error: &StorageError) -> bool {
    let StorageError::Conductor(message) = error else {
        return false;
    };
    message.contains("HeadMoved") || message.contains("source chain head has moved")
}

// =============================================================================
// Retry bounds
// =============================================================================

/// Total conductor attempts for one write, including the first.
const MAX_ATTEMPTS: u32 = 4;

/// Backoff before attempts 2, 3 and 4. Small, because the loser of a local race
/// is already serialized behind the winner by the time it retries — this delay
/// only exists to let an EXTERNAL writer's commit land.
const BACKOFFS: [Duration; 3] = [
    Duration::from_millis(50),
    Duration::from_millis(120),
    Duration::from_millis(280),
];

/// Ceiling on the whole retry sequence when the caller owns no deadline.
/// `min(caller deadline, start + this)` is the budget; the gate never extends.
const DEFAULT_BUDGET: Duration = Duration::from_millis(2_000);

/// Deterministic ±20% jitter, so two writers that lose the same race do not
/// re-offer in lockstep forever.
///
/// Process-local counter rather than `rand`: `rand` is behind the `p2p-iroh`
/// feature in this crate, and a jitter source that vanishes in a default build
/// is worse than one that is merely predictable. Nothing security-bearing reads
/// this.
fn jittered(base: Duration) -> Duration {
    static TICK: AtomicU64 = AtomicU64::new(0x9E3779B97F4A7C15);
    let mut x = TICK.fetch_add(0x9E3779B97F4A7C15, Ordering::Relaxed);
    x ^= x >> 33;
    x = x.wrapping_mul(0xFF51AFD7ED558CCD);
    x ^= x >> 33;
    // 0..=40 percent, centred by subtracting 20.
    let pct = (x % 41) as i64 - 20;
    let millis = base.as_millis() as i64;
    let adjusted = millis + (millis * pct / 100);
    Duration::from_millis(adjusted.max(1) as u64)
}

// =============================================================================
// The per-chain lock registry
// =============================================================================

/// The process-wide registry of per-chain write locks.
///
/// Keyed by a chain key (`<dna>:<agent>`) rather than by `CellId` so the same
/// gate is shared by every [`crate::hc_client::HcClient`] in the per-role
/// registry that happens to target the same cell — two clients, one chain, one
/// lock. The outer `std::sync::Mutex` guards only the map lookup and is never
/// held across an await.
#[derive(Default)]
struct ChainWriteGate {
    locks: Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>,
}

impl ChainWriteGate {
    fn lock_for(&self, chain_key: &str) -> Arc<tokio::sync::Mutex<()>> {
        let mut locks = match self.locks.lock() {
            Ok(guard) => guard,
            // A poisoned registry means a previous holder panicked WHILE holding
            // the map lock — which this code never does across an await. Recover
            // rather than propagate: refusing every write because a map insert
            // once panicked would be a worse outage than the one it reports.
            Err(poisoned) => poisoned.into_inner(),
        };
        locks
            .entry(chain_key.to_string())
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone()
    }
}

fn gate() -> &'static ChainWriteGate {
    static GATE: OnceLock<ChainWriteGate> = OnceLock::new();
    GATE.get_or_init(ChainWriteGate::default)
}

/// The chain key for a cell — one source chain per `(dna, agent)` pair.
pub fn chain_key_of(cell_id: &CellId) -> String {
    format!("{}:{}", cell_id.dna_hash(), cell_id.agent_pubkey())
}

// =============================================================================
// The choke point
// =============================================================================

/// THE choke point. Route one conductor call: reads straight through, writes
/// through the per-chain lock with a bounded `HeadMoved` retry.
///
/// Returns the answer and the duration of the ATTEMPT that produced it — not
/// the serialized wall-clock — so a caller's RTT signal keeps meaning "what the
/// conductor took", with queueing reported separately as
/// `elohim_chain_write_serialized_wait_ms`.
///
/// `call` is re-invoked per attempt and must therefore be replay-safe. That is
/// sound for every write in this crate: a `HeadMoved` refusal commits NOTHING
/// (the conductor rejects the whole bundle before flush), and no in-crate write
/// performs a non-idempotent side effect between the gate and the websocket —
/// the payload is encoded by the caller before it arrives here, and the only
/// per-attempt work is signing plus the call itself.
pub async fn dispatch<F, Fut, T>(
    chain_key: &str,
    zome: &str,
    fn_name: &str,
    call: F,
) -> Result<(T, Duration), StorageError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, StorageError>>,
{
    if !is_chain_write(zome, fn_name) {
        let mut call = call;
        let started = Instant::now();
        let value = call().await?;
        return Ok((value, started.elapsed()));
    }
    write_serialized(chain_key, fn_name, call).await
}

/// The write arm of [`dispatch`], exposed for callers that have already
/// classified (and for tests).
pub async fn write_serialized<F, Fut, T>(
    chain_key: &str,
    fn_name: &str,
    mut call: F,
) -> Result<(T, Duration), StorageError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, StorageError>>,
{
    let writer = current_writer_kind();
    let lock = gate().lock_for(chain_key);

    let started = Instant::now();
    let budget_end = match current_write_deadline() {
        // Shorten to the caller's deadline; NEVER extend past it.
        Some(deadline) => deadline.min(started + DEFAULT_BUDGET),
        None => started + DEFAULT_BUDGET,
    };

    let mut queued_total = Duration::ZERO;
    let mut last_error: Option<StorageError> = None;

    for attempt in 1..=MAX_ATTEMPTS {
        let queue_started = Instant::now();
        let guard = lock.lock().await;
        let waited = queue_started.elapsed();
        queued_total += waited;

        let dispatched_at = Instant::now();
        let outcome = call().await;
        let rtt = dispatched_at.elapsed();
        // The lock models exclusive access to the chain HEAD, which the
        // conductor has finished contending for the instant the call returns.
        drop(guard);

        match outcome {
            Ok(value) => {
                crate::metrics::observe_chain_write_serialized(writer.label(), queued_total);
                return Ok((value, rtt));
            }
            Err(error) if is_source_chain_head_moved(&error) => {
                let backoff = BACKOFFS
                    .get(attempt as usize - 1)
                    .copied()
                    .map(jittered)
                    .unwrap_or(Duration::ZERO);
                let out_of_attempts = attempt == MAX_ATTEMPTS;
                // A budget that cannot fit the backoff PLUS another attempt of
                // roughly the size we just measured is a budget that has ended.
                let out_of_budget = Instant::now() + backoff + rtt >= budget_end;
                last_error = Some(error);
                if out_of_attempts || out_of_budget {
                    crate::metrics::observe_chain_write_serialized(writer.label(), queued_total);
                    crate::metrics::inc_head_moved_exhausted(writer.label());
                    tracing::info!(
                        writer = writer.label(),
                        fn_name = %fn_name,
                        attempts = attempt,
                        reason = if out_of_attempts { "attempts" } else { "budget" },
                        "chain write exhausted its source-chain head-moved retries"
                    );
                    break;
                }
                crate::metrics::inc_head_moved_retried(writer.label());
                tokio::time::sleep(backoff).await;
            }
            // Every other failure is a verdict, not a race. Return it untouched.
            Err(error) => {
                crate::metrics::observe_chain_write_serialized(writer.label(), queued_total);
                return Err(error);
            }
        }
    }

    Err(last_error.unwrap_or_else(|| {
        StorageError::Internal("chain write gate exhausted with no recorded error".into())
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

    fn head_moved() -> StorageError {
        StorageError::Conductor("Zome call failed: SourceChainError::HeadMoved".into())
    }

    fn head_moved_prose() -> StorageError {
        StorageError::Conductor(
            "Zome call failed: Source chain error: Attempted to commit a bundle to the source \
             chain, but the source chain head has moved since the bundle began"
                .into(),
        )
    }

    /// A conductor stand-in that refuses with the EXACT production error the
    /// moment two calls are inside it at once — so "serialized" is proved by the
    /// absence of that refusal, not by a timing assertion.
    #[derive(Default)]
    struct OverlapDetectingChain {
        in_flight: AtomicUsize,
        max_in_flight: AtomicUsize,
        calls: AtomicUsize,
    }

    impl OverlapDetectingChain {
        async fn call(&self, work: Duration) -> Result<Vec<u8>, StorageError> {
            self.calls.fetch_add(1, AtomicOrdering::SeqCst);
            let now = self.in_flight.fetch_add(1, AtomicOrdering::SeqCst) + 1;
            self.max_in_flight.fetch_max(now, AtomicOrdering::SeqCst);
            tokio::time::sleep(work).await;
            self.in_flight.fetch_sub(1, AtomicOrdering::SeqCst);
            if now > 1 {
                return Err(head_moved_prose());
            }
            Ok(b"committed".to_vec())
        }
    }

    #[test]
    fn classifier_reads_are_reads_and_everything_else_is_a_write() {
        for read in [
            "get_content_by_id",
            "list_memberships_for_collective_cid",
            "query_my_source_chain",
            "resolve_content_head",
            "resolve_canonical_election",
            "find_publishers",
            "validate_carried_head_record",
        ] {
            assert!(!is_chain_write("content_store", read), "{read}");
        }
        for write in [
            "create_content",
            "declare_content_head",
            "declare_canonical_content_head",
            "update_rea_commitment_state",
            "create_rea_economic_event",
            "attest_collab_agreement",
            "issue_attestation",
            "record_peer_status",
            "withdraw_membership_clean",
            "queue_import",
            "process_import_chunk",
            // Confirmed against the DNA: this one creates a link.
            "verify_carried_election",
            // Unknown names default to write, which is the safe direction.
            "some_future_extern",
        ] {
            assert!(is_chain_write("content_store", write), "{write}");
        }
    }

    #[test]
    fn head_moved_matcher_is_exact() {
        assert!(is_source_chain_head_moved(&head_moved()));
        assert!(is_source_chain_head_moved(&head_moved_prose()));
        assert!(!is_source_chain_head_moved(&StorageError::Conductor(
            "Zome call failed: HTTP 503 Service Unavailable".into()
        )));
        // Same words, wrong discriminant — a caller's payload, not a race.
        assert!(!is_source_chain_head_moved(&StorageError::InvalidInput(
            "HeadMoved is not an input".into()
        )));
    }

    #[tokio::test(start_paused = true)]
    async fn concurrent_writers_on_one_cell_never_overlap_and_all_succeed() {
        let chain = Arc::new(OverlapDetectingChain::default());
        let key = "chain:serialize-me";

        let mut handles = Vec::new();
        for _ in 0..8 {
            let chain = chain.clone();
            handles.push(tokio::spawn(async move {
                write_serialized(key, "create_content", || {
                    let chain = chain.clone();
                    async move { chain.call(Duration::from_millis(10)).await }
                })
                .await
            }));
        }

        for handle in handles {
            let (bytes, _rtt) = handle.await.unwrap().expect("every writer commits");
            assert_eq!(bytes, b"committed");
        }
        assert_eq!(
            chain.max_in_flight.load(AtomicOrdering::SeqCst),
            1,
            "two writers were inside the conductor call at once"
        );
        assert_eq!(
            chain.calls.load(AtomicOrdering::SeqCst),
            8,
            "serialization should cost no extra attempts"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn reads_are_not_serialized() {
        let chain = Arc::new(OverlapDetectingChain::default());
        let key = "chain:reads";

        let mut handles = Vec::new();
        for _ in 0..4 {
            let chain = chain.clone();
            handles.push(tokio::spawn(async move {
                dispatch(key, "content_store", "get_content_by_id", || {
                    let chain = chain.clone();
                    // A read that overlaps returns the stand-in's refusal; we
                    // assert on the OVERLAP COUNTER, not on the result.
                    async move {
                        let _ = chain.call(Duration::from_millis(10)).await;
                        Ok::<_, StorageError>(())
                    }
                })
                .await
            }));
        }
        for handle in handles {
            handle.await.unwrap().unwrap();
        }
        assert!(
            chain.max_in_flight.load(AtomicOrdering::SeqCst) > 1,
            "reads must stay concurrent — the gate is for writes only"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn an_external_writers_head_move_is_retried_and_succeeds() {
        let calls = AtomicUsize::new(0);
        let (bytes, _rtt) = write_serialized("chain:external", "declare_content_head", || {
            let attempt = calls.fetch_add(1, AtomicOrdering::SeqCst);
            async move {
                if attempt == 0 {
                    Err(head_moved_prose())
                } else {
                    Ok(b"declared".to_vec())
                }
            }
        })
        .await
        .expect("the replay commits");

        assert_eq!(bytes, b"declared");
        assert_eq!(calls.load(AtomicOrdering::SeqCst), 2);
    }

    #[tokio::test(start_paused = true)]
    async fn a_non_head_moved_error_is_not_retried() {
        let calls = AtomicUsize::new(0);
        let error = write_serialized("chain:verdict", "create_content", || {
            calls.fetch_add(1, AtomicOrdering::SeqCst);
            async {
                Err::<(), _>(StorageError::Conductor(
                    "Zome call failed: HTTP 503 Service Unavailable".into(),
                ))
            }
        })
        .await
        .unwrap_err();

        assert_eq!(calls.load(AtomicOrdering::SeqCst), 1);
        assert!(error.to_string().contains("503 Service Unavailable"));
    }

    #[tokio::test(start_paused = true)]
    async fn exhaustion_returns_the_original_error_within_the_attempt_bound() {
        let calls = AtomicUsize::new(0);
        let error = write_serialized("chain:exhaust", "declare_content_head", || {
            calls.fetch_add(1, AtomicOrdering::SeqCst);
            async { Err::<(), _>(head_moved()) }
        })
        .await
        .unwrap_err();

        assert_eq!(calls.load(AtomicOrdering::SeqCst), MAX_ATTEMPTS as usize);
        assert!(is_source_chain_head_moved(&error), "{error}");
    }

    #[tokio::test(start_paused = true)]
    async fn a_caller_deadline_shortens_the_retry_budget_and_is_never_extended() {
        let calls = AtomicUsize::new(0);
        let started = Instant::now();
        // 60ms is shorter than the first backoff plus another attempt, so the
        // gate must give up after the FIRST call rather than spend the caller's
        // remaining time.
        let deadline = started + Duration::from_millis(60);
        let error = with_write_deadline(deadline, async {
            write_serialized("chain:deadline", "declare_content_head", || {
                calls.fetch_add(1, AtomicOrdering::SeqCst);
                async {
                    tokio::time::sleep(Duration::from_millis(40)).await;
                    Err::<(), _>(head_moved())
                }
            })
            .await
        })
        .await
        .unwrap_err();

        assert_eq!(calls.load(AtomicOrdering::SeqCst), 1);
        assert!(is_source_chain_head_moved(&error));
        assert!(
            Instant::now() <= deadline,
            "the gate spent past the caller's deadline"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn an_interactive_writer_is_not_starved_by_a_background_batch() {
        let key = "chain:fairness";
        let batch_calls = 20;
        let per_call = Duration::from_millis(10);

        let batch = tokio::spawn(async move {
            as_writer(WriterKind::SweepAdopt, async move {
                for _ in 0..batch_calls {
                    let _ = write_serialized(key, "declare_content_head", || async {
                        tokio::time::sleep(per_call).await;
                        Ok::<_, StorageError>(())
                    })
                    .await;
                }
            })
            .await;
        });

        // Let the batch get a few calls in, then arrive as the person waiting.
        tokio::time::sleep(Duration::from_millis(25)).await;
        let arrived = Instant::now();
        as_writer(WriterKind::HttpAuthor, async {
            write_serialized(key, "update_content", || async {
                Ok::<_, StorageError>(())
            })
            .await
            .unwrap();
        })
        .await;
        let waited = arrived.elapsed();

        // FIFO fairness: at most the ONE in-flight call, because the batch holds
        // the lock per call, not per batch. Generous ceiling so the assertion is
        // about the SHAPE (constant, not proportional to the batch) — the whole
        // batch would be ~200ms.
        assert!(
            waited < per_call * 4,
            "interactive waited {waited:?} — that is batch-shaped, not call-shaped"
        );
        batch.await.unwrap();
    }

    #[tokio::test(start_paused = true)]
    async fn writer_kind_defaults_to_other_and_scopes_cleanly() {
        assert_eq!(current_writer_kind(), WriterKind::Other);
        as_writer(WriterKind::Reanchor, async {
            assert_eq!(current_writer_kind(), WriterKind::Reanchor);
        })
        .await;
        assert_eq!(current_writer_kind(), WriterKind::Other);
    }

    #[test]
    fn jitter_stays_within_twenty_percent_and_never_reaches_zero() {
        for _ in 0..200 {
            let j = jittered(Duration::from_millis(100));
            assert!(j >= Duration::from_millis(80), "{j:?}");
            assert!(j <= Duration::from_millis(120), "{j:?}");
        }
        assert!(jittered(Duration::from_millis(1)) >= Duration::from_millis(1));
    }
}
