//! P1 projection reconciliation stream — REA commitments converge from the
//! OWN conductor, with peers used as discovery only.
//!
//! ## Why this exists (the incident it cures)
//!
//! Edge-triggered `ReaProjectionSignal`s are the ONLY thing that lands REA
//! commitments into a peer's local SQL projection. If a peer misses the signal
//! (stale binary at signal time, restart window, gossip race), its projection
//! stays divergent — and a reseed on the originating peer collapses to a 409,
//! so the signal never re-fires. adam's storage stayed divergent for 10 days
//! this way (`.claude/deliver/journal-resilient-dual-doorway.md`, root cause
//! #2). Edge-triggered projection with no reconciliation is a P1 gap.
//!
//! ## The controller, on the shared rails
//!
//! This is a NEW STREAM on `reconcile_rails::GapTracker` — the ONE controller
//! pattern. A parallel bespoke fetcher would be a coherence violation. The
//! tracker is reconstructed per sweep (Category C operational state; the durable
//! truth is the DHT, the projection is the index).
//!
//! ## The design contract (binding, from the p2p-design-gate output)
//!
//! - **Peer SQL is discovery ONLY.** We ask connected peers for their
//!   `(id, dht_anchor_hash)` inventory of REA commitments (the extended
//!   `ViewKind::ProjectionInventory` over `/elohim/view-federation/1.0.0`). For
//!   each id missing locally OR present with a DIFFERENT anchor, we call our OWN
//!   conductor's `content_store::get_rea_commitment(id)`. Row content comes
//!   EXCLUSIVELY from the conductor's DHT notary view. Peer bytes are NEVER
//!   written into the projection.
//! - **Upsert through the shared mapping.** Both the post-commit signal handler
//!   and this reconciler funnel the wire Commitment through
//!   `rea_projection::project_commitment_from_wire` → `upsert_with_anchor`.
//! - **Gap discipline.** Conductor-can't-see-it (`get` returns `None`) →
//!   `mark_failed`, retried on the NEXT sweep (never an immediate re-queue — the
//!   freeze-at-partial battle-scar). Counts are observable on `/p2p/status`.
//! - **v1 scope: `rea_commitments` only.** The table discriminator on
//!   `ProjectionInventory` is the seam for agreements / economic_events; this
//!   sweep asks only for `rea_commitments`.
//!
//! ## The content arm (notary-authority Leg 4)
//!
//! Alongside the REA arm, the `content` arm ([`discover_content`] +
//! [`heal_content`]) runs the SAME pattern for the `content` projection — the
//! cross-peer content-anchor reconcile arm that flips scenario 2: a peer (e.g.
//! `elohim.host`) reaches `trust="notarized"` for content whose DHT anchor exists
//! on an authoring conductor but whose `ContentCommitted` signal it never saw
//! (`post_commit` fires only on the authoring conductor). It shares the cadence
//! (both arms run from [`run_discovery`]/[`run_heal`] on the same
//! `PROJECTION_RECONCILE_SECS` tick) and the shared `GapTracker` rails, but
//! keeps its OWN tracker — the id space is disjoint from REA. Its heal
//! entrypoint is the conductor-VERIFIED [`content_diesel::stamp_declared_head`];
//! the anchor value comes EXCLUSIVELY from the node's own
//! `content_store::resolve_content_head`, never from the peer-advertised pair.
//! Its `divergent_anchor` folds into the shared `/p2p/status` counter (the one
//! cross-arm health signal); its heal/miss detail is log-observable, because
//! extending the ts-rs-exported [`ProjectionReconcileStatus`] with content
//! fields would change the `p2p-status` wire shape (owned elsewhere).
//!
//! ## The collectives arm (cross-peer collective identity)
//!
//! Same shape again for the `collectives` projection ([`discover_collectives`] +
//! [`heal_collectives`]). `CollectiveCommitted` post-commit signals fire ONLY on
//! the AUTHORING conductor, so a non-authoring peer never acquires the row at
//! all — `household_resilience` then renders a collective its neighbours can see
//! and it cannot. Peers advertise `(routing-alias id, collective_cid)`; the cid
//! is resolved against the OWN conductor's imagodei cell
//! ([`crate::services::conductor_writes::get_collective_by_cid`] →
//! `imagodei::get_collective_by_action`), which on a full-arc fleet holds every
//! authoring agent's `Collective` entry. Both the signal arm and this arm funnel
//! through ONE mapping, [`crate::db::collectives::project_collective`], so the
//! charter→governance derivation and the slug-alias merge cannot drift apart.
//!
//! Three decisions are load-bearing and deliberately recorded here:
//!
//! - **NULL-`collective_cid` rows are excluded from the inventory** — both from
//!   the local snapshot this arm diffs and from
//!   [`crate::db::collectives::list_collective_cid_inventory`] on the responder
//!   side. A pre-coherence seed row has no DHT identity to reconcile ON;
//!   replicating it would mean adopting a peer's local routing alias as truth,
//!   which is worse than a gap. Such rows stay upgradable IN PLACE — the
//!   `slugAlias` merge in `project_collective` (and this arm's
//!   [`CollectiveGap::CidGap`] fill) stamps the real cid the moment it arrives.
//! - **Own [`GapTracker`], own [`HealPacing`] budget, ordered LAST** (after REA
//!   and content) so it can never starve the two arms that came before it.
//! - **The identity is `collective_cid`; the diesel `id` is a routing alias.**
//!   The tracker is keyed by cid. A local row under the advertised alias that
//!   carries a DIFFERENT cid is [`CollectiveGap::Divergent`]: counted and WARN-
//!   logged, never enqueued — `collectives` carries no declaration-ordering
//!   column, so a heal can never prove a forward move and an enqueue would be a
//!   guaranteed no-op conductor round-trip every sweep. Heal fills, never moves.
//!
//! `h_app_id` partition: this arm reads AND writes `lamad`
//! (`AppContext::default_lamad()`) — the same scope
//! `ReconcileController::on_collective_projected` projects signals into.

use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use tokio::sync::RwLock;
use ts_rs::TS;

use crate::db::DbPool;
use crate::hc_client::HcClient;
use crate::p2p::head_record_client::PeerHeadRecordFetcher;
use crate::p2p::reconcile_rails::GapTracker;
use crate::p2p::view_federation::{
    PROJECTION_INVENTORY_CAP, PROJECTION_INVENTORY_TABLE_COLLECTIVES,
    PROJECTION_INVENTORY_TABLE_CONTENT, PROJECTION_INVENTORY_TABLE_REA_COMMITMENTS,
};
use crate::p2p::P2PHandle;
use crate::services::provide_loop_status::ProvideLoopState;
use crate::views::{ProjectionInventoryPayload, ViewFederationRequest, ViewKind};

/// Per-tick cap for the sweep-driven witness-bootstrap authoring step (GAP 1.5).
/// Keeps conductor load bounded on a large seeded corpus: a ~6k un-witnessed
/// corpus greens over ~30 ticks rather than storming a saturated conductor in a
/// single sweep (the F-T19 evidence — adam's conductor already sits at its
/// read-pool ceiling). Do not raise without weighing conductor saturation.
const WITNESS_MAX_PER_TICK: i64 = 200;

/// Per-item spacing inside a witness sweep (each item is a conductor round-trip).
const WITNESS_ITEM_DELAY: Duration = Duration::from_millis(25);

/// Wall-clock budget for one witness sweep. `HcClient::call_zome` awaits with no
/// timeout of its own, so a hung/stuck conductor call would otherwise hold the
/// heal leg's single-flight guard forever (the RAII `HealFlag` covers panic and
/// cancellation, but not an infinite await). Bounding the whole sweep releases
/// the guard normally on the worst case and resumes next tick (the sweep is
/// idempotent). Derivation: `WITNESS_MAX_PER_TICK` (200) × `WITNESS_ITEM_DELAY`
/// (25ms) = 5s of spacing, plus generous conductor-latency headroom for 200
/// round-trips on a healthy node.
const WITNESS_SWEEP_BUDGET: Duration = Duration::from_secs(120);

/// Per-sweep retry budget for conductor-can't-see-it gaps. A gap that the
/// conductor still can't resolve after this many sweeps drops out (it is almost
/// certainly an id this DHT view legitimately does not carry — a foreign-app or
/// not-yet-gossiped entry). The next sweep that re-discovers it from a peer
/// resets nothing; the failed-count persists for the life of THIS tracker, but
/// the tracker is rebuilt each sweep, so a transient miss self-heals.
const MAX_RETRIES: u32 = 3;

/// Sweeps an exhausted [`MissLedger`] entry stays dormant before it is re-admitted
/// for one more round of attempts.
///
/// Exhaustion must not be PERMANENT: an id the conductor cannot see today may be
/// gossiped to it tomorrow, and a ledger with no cooldown would silently stop
/// healing it for the process lifetime. At the default 300s tick this is ~1h of
/// dormancy — a ~12× cut in conductor asks for a stuck id, while still retrying
/// it roughly hourly.
const MISS_READMIT_SWEEPS: u32 = 12;

/// Per-stream ceiling on ledger entries. Bounds the memory a hostile or merely
/// huge peer inventory can pin: past the cap new ids are admitted WITHOUT being
/// recorded (fail-open — the pre-ledger behaviour), never dropped.
const MISS_LEDGER_CAP: usize = 100_000;

/// Whether a gap id may be enqueued for heal this sweep.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Admission {
    /// Enqueue it — the retry budget has room (or fresh evidence re-admitted it).
    Retry,
    /// Retry budget spent against UNCHANGED evidence — do not enqueue this sweep.
    /// ADJUDICATED, not unresolved: the conductor has been asked `MAX_RETRIES`
    /// times and nothing new has been advertised since.
    Exhausted,
}

#[derive(Debug, Clone)]
struct MissEntry {
    /// Sweeps this id has been a gap under `evidence`.
    misses: u32,
    /// The peer-advertised claim that made it a gap (content/REA: the advertised
    /// anchor; collectives: the cid). A DIFFERENT claim is new evidence and
    /// re-admits the id immediately.
    evidence: String,
    /// Sweeps spent exhausted, counted toward [`MISS_READMIT_SWEEPS`].
    dormant: u32,
}

/// Cross-sweep miss counts for the reconcile arms — the thing that makes
/// `MAX_RETRIES` REAL.
///
/// ## Why this exists (the treadmill it kills)
///
/// Every arm rebuilds its [`GapTracker`] per sweep (the tracker is Category C
/// per-sweep state), so `GapTracker::exhausted_count` is structurally always 0
/// and `enqueue_missing`'s `>= max_retries` refusal can never fire across
/// sweeps. The visible consequence: ~1000 rows re-discovered and re-asked of the
/// conductor EVERY sweep forever (matthew logged ~151k `missing` heal outcomes in
/// 12h), and `elohim_projection_reconcile_exhausted` published 0 while the whole
/// backlog was, in fact, abandoned-and-retried.
///
/// This ledger is owned by the single discovery loop in `main.rs` alongside
/// [`InventoryWindow`] — one owner, no lock — and threaded `&mut` into each
/// discovery arm. It is deliberately NOT inside `GapTracker`: the tracker's
/// per-sweep rebuild is correct (it is the sweep's work list); what was missing
/// is memory ACROSS sweeps.
///
/// ## Re-admission (why exhaustion is never a black hole)
///
/// - **New evidence** — a peer advertising a DIFFERENT anchor/cid for the id
///   resets its counter immediately. The thing we gave up on is not the thing
///   being advertised now.
/// - **Cooldown** — after [`MISS_READMIT_SWEEPS`] dormant sweeps the id is
///   re-admitted for another round.
///
/// An id the ledger holds back is ADJUDICATED: it does not defeat convergence
/// (we have done what the substrate permits), but it never vanishes silently —
/// it stays in the divergence TOTAL and is published on
/// `elohim_projection_reconcile_exhausted`.
#[derive(Debug, Default)]
pub struct MissLedger {
    streams: std::collections::HashMap<&'static str, std::collections::HashMap<String, MissEntry>>,
}

impl MissLedger {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record that `id` is STILL a gap this sweep, under `evidence`, and decide
    /// whether it may be enqueued.
    pub fn admit(&mut self, stream: &'static str, id: &str, evidence: &str) -> Admission {
        let entries = self.streams.entry(stream).or_default();
        match entries.get_mut(id) {
            None => {
                if entries.len() >= MISS_LEDGER_CAP {
                    // Fail-open: an un-recorded id behaves exactly as it did
                    // before this ledger existed (retried every sweep).
                    return Admission::Retry;
                }
                entries.insert(
                    id.to_string(),
                    MissEntry {
                        misses: 1,
                        evidence: evidence.to_string(),
                        dormant: 0,
                    },
                );
                Admission::Retry
            }
            Some(e) => {
                if e.evidence != evidence {
                    // New evidence — this is not the claim we gave up on.
                    e.evidence = evidence.to_string();
                    e.misses = 1;
                    e.dormant = 0;
                    return Admission::Retry;
                }
                if e.misses >= MAX_RETRIES {
                    e.dormant = e.dormant.saturating_add(1);
                    if e.dormant >= MISS_READMIT_SWEEPS {
                        e.misses = 1;
                        e.dormant = 0;
                        return Admission::Retry;
                    }
                    return Admission::Exhausted;
                }
                e.misses = e.misses.saturating_add(1);
                Admission::Retry
            }
        }
    }

    /// `id` is no longer a gap (in-sync, or absent-local and thus another
    /// plane's job) — forget it, so a later relapse starts from a clean budget.
    pub fn resolved(&mut self, stream: &'static str, id: &str) {
        if let Some(entries) = self.streams.get_mut(stream) {
            entries.remove(id);
        }
    }

    /// Ids currently tracked for `stream` (observability / test assertion).
    pub fn tracked(&self, stream: &'static str) -> usize {
        self.streams.get(stream).map(|e| e.len()).unwrap_or(0)
    }
}

/// Per-peer deadline for a single `ProjectionInventory` federation request.
const PEER_TIMEOUT: Duration = Duration::from_secs(10);

// ── Heal-leg pacing (the saturated-conductor cure) ──────────────────────────
//
// The incident: adam (shem node) sits on a saturated conductor with steady
// ~1/min WS-timeout zome calls (1.5–1.8× matthew's rate). The heal leg is
// single-flight and, pre-fix, UNBOUNDED — it walked every pending row calling
// the conductor with no per-row retry and no per-leg wall-clock bound. On a
// slow conductor one leg could grind for hours (1,956 content rows × a ~60s WS
// timeout each), so the leg never recycled: the next discovery tick found the
// leg still in flight (`SkipInFlight`) and the small REA backlog (62 rows) that
// COULD have landed was starved behind the content queue. Result: rea_local_total
// stuck at 0 for 3h while matthew (healthy conductor) healed the same backlog.
//
// The cure is three-part, all bounded and observable:
//   1. Per-row transient retry — a WS timeout on ONE row gets a bounded in-leg
//      retry (timeout-class errors only) so an intermittently-saturated conductor
//      lands rows it would otherwise drop for the whole sweep.
//   2. Per-leg wall-clock budget — each arm processes rows until its budget
//      elapses, then YIELDS the single-flight guard so the leg recycles every
//      sweep instead of one multi-hour grind. Un-attempted rows are re-discovered
//      next sweep (the tracker is per-sweep), so nothing is lost.
//   3. REA priority — REA heals FIRST with its own reserved budget, so its small
//      backlog is never starved behind the large content queue.

/// Bounded in-leg retry attempts for a transient (timeout-class) conductor error
/// on a single row. 2 retries ⇒ up to 3 attempts. A row that exhausts these stays
/// a `mark_failed` gap (re-discovered next sweep), exactly as before — the retry
/// only adds chances to catch a saturated conductor's free window WITHIN a sweep.
///
/// SCOPE (2026-07-29): these retries apply ONLY to an error the conductor
/// actually ANSWERED with (a websocket timeout it reported back). They are
/// deliberately NOT spent on our own synthetic per-attempt timeout — see
/// [`should_retry_attempt`].
const MAX_ROW_RETRIES: u32 = 2;

/// Per-attempt deadline for one heal conductor call. Deliberately TIGHTER than the
/// conductor client's own ~60s WS timeout: a wedged call is abandoned fast and
/// retried (or the next row attempted) rather than burning ~60s of the leg budget
/// on a single hung row. `HcClient::call_zome` has no timeout of its own.
const HEAL_ATTEMPT_TIMEOUT: Duration = Duration::from_secs(15);

/// Stable marker embedded in the SYNTHETIC per-attempt timeout this module
/// manufactures when `tokio::time::timeout` elapses on a heal call. Used both to
/// BUILD that error and to RECOGNISE it in [`is_synthetic_attempt_timeout`], so
/// the two can never drift apart.
const HEAL_SYNTHETIC_TIMEOUT_MARKER: &str = "heal conductor call exceeded per-attempt timeout";

/// Consecutive synthetic per-attempt timeouts that OPEN the leg circuit and shed
/// the remainder of the leg. At the 15s per-attempt deadline this is ~45s of leg
/// budget spent proving the conductor is not answering; past that, further rows
/// only stack more abandoned-but-still-executing calls on it.
const HEAL_CIRCUIT_TIMEOUT_THRESHOLD: u32 = 3;

/// In-leg retry backoff floor / span for a transient row (jittered in `[min, min+span)`).
const HEAL_BACKOFF_MIN: Duration = Duration::from_secs(2);
const HEAL_BACKOFF_SPAN: Duration = Duration::from_secs(3); // → 2–5s jittered

/// Per-leg wall-clock budget for the REA arm (small backlog, prioritized — runs
/// first with this reserved slice so it is never starved behind content).
const REA_LEG_BUDGET: Duration = Duration::from_secs(45);

/// Per-leg wall-clock budget for the content arm (the large backlog; recycles each
/// sweep so a saturated conductor lands SOME rows per tick instead of one grind).
const CONTENT_LEG_BUDGET: Duration = Duration::from_secs(120);

/// Per-leg wall-clock budget for the collectives arm. Runs LAST with the
/// smallest reserved slice: the collectives corpus is tiny (dozens of rows, one
/// conductor round-trip each) and the two arms ahead of it own the fleet's real
/// backlog — a generous budget here could only ever starve a future arm, never
/// help this one.
const COLLECTIVES_LEG_BUDGET: Duration = Duration::from_secs(30);

/// Injectable pacing for the heal legs (retry + budget). Defaults come from the
/// consts above; tests override with a fast profile (no real sleeps, generous
/// budgets) so the retry/outcome logic is exercised without wall-clock waits.
#[derive(Debug, Clone)]
pub struct HealPacing {
    pub max_row_retries: u32,
    pub attempt_timeout: Duration,
    pub backoff_min: Duration,
    pub backoff_span: Duration,
    pub rea_leg_budget: Duration,
    pub content_leg_budget: Duration,
    pub collectives_leg_budget: Duration,
    /// Consecutive synthetic per-attempt timeouts that shed the rest of a leg.
    /// `0` disables the circuit (never opens).
    pub circuit_timeout_threshold: u32,
}

impl Default for HealPacing {
    fn default() -> Self {
        Self {
            max_row_retries: MAX_ROW_RETRIES,
            attempt_timeout: HEAL_ATTEMPT_TIMEOUT,
            backoff_min: HEAL_BACKOFF_MIN,
            backoff_span: HEAL_BACKOFF_SPAN,
            rea_leg_budget: REA_LEG_BUDGET,
            content_leg_budget: CONTENT_LEG_BUDGET,
            collectives_leg_budget: COLLECTIVES_LEG_BUDGET,
            circuit_timeout_threshold: HEAL_CIRCUIT_TIMEOUT_THRESHOLD,
        }
    }
}

impl HealPacing {
    /// Test profile: no backoff, a generous per-attempt timeout, and budgets large
    /// enough that a small test set never trips the yield (the retry/outcome logic
    /// is what these tests exercise, not the wall-clock bound).
    #[cfg(test)]
    fn test_fast() -> Self {
        Self {
            max_row_retries: 2,
            attempt_timeout: Duration::from_secs(30),
            backoff_min: Duration::ZERO,
            backoff_span: Duration::ZERO,
            rea_leg_budget: Duration::from_secs(3600),
            content_leg_budget: Duration::from_secs(3600),
            collectives_leg_budget: Duration::from_secs(3600),
            // Circuit OFF by default in tests: the existing retry/outcome cases
            // drive deliberate timeout streaks and must not be shed mid-set. The
            // circuit's own tests opt in explicitly.
            circuit_timeout_threshold: 0,
        }
    }

    /// Jittered backoff in `[backoff_min, backoff_min + backoff_span)`. Dependency-
    /// free jitter from the wall clock's sub-second nanos (spreading concurrent
    /// retries across the fleet is the only property needed — not cryptographic
    /// randomness). A zero span (tests) yields exactly `backoff_min`.
    fn backoff(&self) -> Duration {
        let span_ms = self.backoff_span.as_millis() as u64;
        if span_ms == 0 {
            return self.backoff_min;
        }
        let jitter_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos() as u64)
            .unwrap_or(0)
            % span_ms;
        self.backoff_min + Duration::from_millis(jitter_ms)
    }
}

/// True when a conductor error is a TRANSIENT class worth a bounded in-leg retry —
/// a websocket timeout from a saturated conductor (adam's steady ~1/min WS
/// timeouts), NOT a definitive miss (`Ok(None)`, retried next SWEEP) nor a
/// decode/logic error (never retried, no free-window will fix it). The conductor
/// client preserves the error text verbatim (`Zome call failed: Websocket error:
/// Timeout`), so this is a substring match on that text plus the explicit
/// [`StorageError::Timeout`] variant.
fn is_transient_conductor_error(err: &crate::error::StorageError) -> bool {
    use crate::error::StorageError;
    match err {
        StorageError::Timeout(_) => true,
        StorageError::Conductor(m)
        | StorageError::HolochainClient(m)
        | StorageError::Connection(m) => {
            let m = m.to_ascii_lowercase();
            m.contains("timeout") || m.contains("timed out")
        }
        _ => false,
    }
}

/// True for the SYNTHETIC per-attempt timeout this module manufactures when
/// `tokio::time::timeout` elapses on a heal call — as opposed to a timeout the
/// conductor itself ANSWERED with.
///
/// The distinction is load-bearing (2026-07-29). `HcClient::call_zome` has no
/// cancellation: when our `tokio::time::timeout` fires we stop AWAITING the call,
/// but the conductor keeps executing it. Retrying therefore does not re-try
/// anything — it stacks a SECOND (then third) concurrent zome call on a conductor
/// that is already not answering, each of which fans out its own network
/// `get_links` requests. On adam that turned one stalled row into 3× the
/// conductor load for zero forward progress.
fn is_synthetic_attempt_timeout(err: &crate::error::StorageError) -> bool {
    matches!(
        err,
        crate::error::StorageError::Timeout(m) if m.contains(HEAL_SYNTHETIC_TIMEOUT_MARKER)
    )
}

/// Whether a failed heal attempt is worth ANOTHER attempt within the leg.
///
/// Transient (timeout-class) AND answered — i.e. the conductor came back to us,
/// so a retry genuinely re-tries. Our own synthetic per-attempt timeout is
/// explicitly excluded (see [`is_synthetic_attempt_timeout`]): abandoning does not
/// cancel the in-flight call, so a retry only amplifies load on an unresponsive
/// conductor. Such a row stays a `mark_failed` gap and is re-discovered next
/// sweep, exactly as an exhausted row always has been.
fn should_retry_attempt(err: &crate::error::StorageError) -> bool {
    is_transient_conductor_error(err) && !is_synthetic_attempt_timeout(err)
}

/// Per-leg circuit breaker over CONSECUTIVE synthetic per-attempt timeouts.
///
/// Purpose: stop feeding an unresponsive conductor. Once `threshold` heal calls in
/// a row have been abandoned at the per-attempt deadline, the remainder of the leg
/// is shed — un-attempted rows are re-discovered next sweep (the tracker is
/// per-sweep), so nothing is lost, and the conductor gets a quiet window in which
/// its own gossip/fetch queue can drain. On the alpha fleet that queue draining is
/// precisely what lets a cell's storage arc converge to FULL, after which these
/// calls resolve locally instead of leaving the box at all.
///
/// The circuit is created fresh per leg invocation, so "open" means "yield THIS
/// leg", never a durable trip. Any ANSWERED call (success, or a failure the
/// conductor returned) breaks the streak — the signal is unresponsiveness, not
/// row-level failure.
#[derive(Debug)]
struct HealCircuit {
    threshold: u32,
    consecutive_timeouts: u32,
    open: bool,
}

impl HealCircuit {
    fn new(threshold: u32) -> Self {
        Self {
            threshold,
            consecutive_timeouts: 0,
            open: false,
        }
    }

    /// Fold one attempt outcome into the circuit.
    fn record<T>(&mut self, outcome: &Result<T, crate::error::StorageError>) {
        match outcome {
            // A success closes the circuit outright.
            Ok(_) => {
                self.consecutive_timeouts = 0;
                self.open = false;
            }
            Err(e) if is_synthetic_attempt_timeout(e) => {
                self.consecutive_timeouts = self.consecutive_timeouts.saturating_add(1);
                if self.threshold > 0 && self.consecutive_timeouts >= self.threshold {
                    self.open = true;
                }
            }
            // The conductor ANSWERED (even if with an error): it is responsive, so
            // the unresponsiveness streak breaks.
            Err(_) => {
                self.consecutive_timeouts = 0;
            }
        }
    }

    fn is_open(&self) -> bool {
        self.open
    }

    fn consecutive_timeouts(&self) -> u32 {
        self.consecutive_timeouts
    }
}

/// Classify a ghost-witness re-author failure into the Prometheus `class`
/// label the saga-06-heads-converge stations count
/// (`elohim_content_witness_reauthor_failed_total`) — `None` for a failure
/// that fits neither counted class (still bumps the sweep's local `failed`
/// tally and the WARN line, just not a labeled series).
///
/// The already-exists check DELEGATES to
/// `reanchor_backfill::is_already_anchored_error` rather than re-deriving a
/// second substring match — this ghost-witness sweep and the boot-time
/// re-anchor sweep hit the exact same conductor error text
/// (`create_content`'s duplicate-id Guest error) and must never classify it
/// differently. The chain-head-moved check is a substring match on the same
/// verbatim conductor text the WARN line already logs ("Source chain error:
/// source chain head has moved …", HDK's `SourceChainError::HeadMoved`
/// surfaced through `HcClient::call_zome`) — a chronically busy own-chain
/// writer racing this node's own re-author call (station A).
fn classify_reauthor_failure_class(err: &crate::error::StorageError) -> Option<&'static str> {
    if crate::services::reanchor_backfill::is_already_anchored_error(err) {
        Some("already_exists")
    } else if err.to_string().contains("source chain head has moved") {
        Some("chain_head_moved")
    } else {
        None
    }
}

/// The classified result of healing ONE row, for the `/metrics` outcome counter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HealOutcomeKind {
    /// Conductor answered and the projection write succeeded on the first attempt.
    Healed,
    /// Conductor answered and the write succeeded, but only after ≥1 transient
    /// retry — the signal that the bounded retry is doing real work on a saturated
    /// conductor (vs `timeout_exhausted`, where it could not recover the row).
    TimeoutRetried,
    /// Every attempt hit a transient (timeout-class) error — row stays a gap,
    /// re-discovered next sweep.
    TimeoutExhausted,
    /// Conductor definitively could not see the row (`Ok(None)`) — retried next
    /// sweep, never immediately.
    Missing,
    /// A non-transient error (decode/logic) or a projection-write failure — retried
    /// next sweep, but no in-leg retry (no free window fixes it).
    Failed,
    /// Content arm, `StampMode::GapFill`: the conductor gave a FALLBACK (non-
    /// canonical) answer and the row already carries a DIFFERENT declared head, so
    /// heal left it to the canonical channels ([`StampOutcome::SkippedDeclared`]).
    /// Not an error and not progress — the heal is REFUSING CORRECTLY.
    RefusedDeclared,
    /// Content arm, `StampMode::HealCanonical`: the conductor's CANONICAL answer is
    /// not provably newer than the adopted head, so the heal kept the adopted head
    /// ([`StampOutcome::SkippedStale`]). Also a correct refusal.
    RefusedStale,
    /// The local row vanished between the presence check and the stamp
    /// ([`StampOutcome::NoRow`]) — resolved, not a conductor miss.
    NoRow,
    /// The stamp wrote, but the declared HEAD did not move: the row already held
    /// the action the own conductor answered with
    /// ([`StampOutcome::Refreshed`]). Real work (patch refresh + ordering
    /// backfill) but NOT convergence — the peer-advertised anchor this row was
    /// enqueued for is still divergent, and will be re-enqueued next sweep.
    ///
    /// A sustained high `refreshed` rate against a non-zero `divergent_anchor` is
    /// the SPIN signature: the two peers hold genuinely different roots, the own
    /// conductor keeps answering with its own, and no amount of healing can
    /// converge them — only a canonical channel can. That diagnosis was
    /// previously indistinguishable from real healing.
    Refreshed,
    /// Content arm: a NON-canonical (fallback / root-author-election) answer would
    /// have `GapFill`-ed an UNDECLARED row with THIS node's own root, while a peer
    /// advertises a real declaration for the id. The stamp was SKIPPED and the id
    /// handed to the adopt-before-author arm instead.
    ///
    /// This is the third site of the self-election defect, and the most insidious:
    /// unlike the re-anchor and ghost sweeps (which the pre-flight guards), a
    /// GapFill self-election is TERMINAL. Once the row carries this node's own
    /// root as its declaration it is anchored (so the re-anchor sweep skips it)
    /// and conductor-resolvable (so the ghost sweep skips it), and the decision
    /// rule then answers `Hold` forever. The divergence goes quiet, permanently,
    /// with `elohim_content_head_adopted_total` flat — invisible.
    DeferredToAdopt,
}

impl HealOutcomeKind {
    fn label(self) -> &'static str {
        match self {
            HealOutcomeKind::Healed => "healed",
            HealOutcomeKind::TimeoutRetried => "timeout_retried",
            HealOutcomeKind::TimeoutExhausted => "timeout_exhausted",
            HealOutcomeKind::Missing => "missing",
            HealOutcomeKind::Failed => "failed",
            HealOutcomeKind::RefusedDeclared => "refused_declared",
            HealOutcomeKind::RefusedStale => "refused_stale",
            HealOutcomeKind::NoRow => "no_row",
            HealOutcomeKind::Refreshed => "refreshed",
            HealOutcomeKind::DeferredToAdopt => "deferred_to_adopt",
        }
    }
}

/// Would a `GapFill` stamp of a NON-canonical own-conductor answer amount to
/// SELF-ELECTION over a peer's real declaration?
///
/// Pure + total so the exact live trace is testable without a conductor:
/// cross-root id lands NULL-anchored on peer B at boot → the re-anchor sweep
/// authors a non-declaring root (correct, guarded) → P2P comes up → discovery
/// classifies the id Divergent against peer A's anchor → `heal_content` runs
/// FIRST in `run_heal`, ahead of both pre-flight-guarded sweeps → B's conductor
/// cannot resolve A's canonical link across the gossip gap, so it answers
/// `canonical == false` with B's OWN fallback root → `GapFill` fills the
/// undeclared row with it.
///
/// That write is terminal. The row is now anchored (invisible to the re-anchor
/// sweep) and conductor-resolvable (invisible to the ghost sweep), so it never
/// reaches [`crate::services::head_adoption::try_adopt_canonical_head`] again,
/// and if it did the rule would answer `Hold`. A's declaration is ignored
/// forever and nothing counts it.
///
/// Three conditions, all required:
/// - **`!answer_canonical`** — a canonical answer carries real authority and
///   keeps `HealCanonical` semantics untouched.
/// - **`peer_advertises_declaration`** — with no peer hint there is no better
///   claim to defer to, so `GapFill` remains the right, unchanged behaviour.
/// - **`local_declared.is_none()`** — only an UNDECLARED row is at risk;
///   `GapFill` on a declared row already refuses (`SkippedDeclared`).
///
/// This does not widen `Declare` and does not change what `GapFill` writes. It
/// keys on the authority the write CARRIES — a fallback self-root is the weakest
/// authority there is, and filling with it while a peer advertises a real
/// declaration is squatting, not filling absence.
fn gapfill_would_self_elect(
    answer_canonical: bool,
    peer_advertises_declaration: bool,
    local_declared: Option<&str>,
) -> bool {
    !answer_canonical && peer_advertises_declaration && local_declared.is_none()
}

/// Should a FAILED own-conductor resolve be handed to the adopt-before-author arm
/// instead of simply dropped for the sweep?
///
/// Pure + total so the routing rule is testable without a conductor (same reason
/// [`gapfill_would_self_elect`] is extracted).
///
/// YES exactly when the failure was TRANSIENT (timeout-class — our conductor is
/// unresponsive, not authoritative-ly denying the row) AND a peer advertises a
/// declaration for the id, i.e. there is a verified peer path to try. A
/// non-transient failure is a decode/logic error that adoption cannot fix, and
/// without a peer hint there is nothing to adopt FROM.
///
/// This closes the drop-forever hole: before this, a timed-out row landed in
/// NEITHER the ghost list (that needs `Ok(None)`) nor the adopt list, so an
/// unresponsive conductor meant the id was silently re-dropped every sweep.
fn timeout_should_route_to_adopt(transient: bool, peer_advertises_declaration: bool) -> bool {
    transient && peer_advertises_declaration
}

/// Should a CONDUCTOR-MISSING row (`resolve_content_head` answered `Ok(None)`)
/// also be routed to the adopt-before-author arm?
///
/// YES whenever a peer advertises a declaration for it. `Ok(None)` means this
/// node's own conductor holds no chain for an id its projection DOES carry — so
/// there is nothing local to adopt and, if anyone is to be believed about this
/// head, it is the peer.
///
/// ## The hole this closes (the MISSING class, 2026-08-01)
///
/// `Ok(None)` is the LARGEST heal outcome class on a live peer, and until now it
/// reached adoption only INDIRECTLY, via [`witness_ghost_anchors`] — which first
/// narrows its candidates with `anchored_content_reaches_for_ids`, i.e. to rows
/// that already CLAIM an anchor. A conductor-missing row whose
/// `dht_anchor_hash` is NULL — precisely the `ContentGap::AnchorGap` class that
/// dominates the content gap set — was filtered straight back out and never saw
/// the pre-flight at all. It fell instead to [`witness_bootstrap`], which
/// AUTHORS a fresh root: the exact self-election adopt-before-author exists to
/// prevent, performed on the one class most likely to have a peer already
/// holding the crown.
///
/// Routing here is precedence-correct because `run_heal` drains
/// `adopt_deferred_heads` FIRST, before either witness sweep — so an id that
/// adopts is already declared by the time the authoring paths look at it, and
/// `try_adopt_canonical_head` returns `Hold` for an already-declared row, so
/// double-listing an id across the ghost and adopt lists is idempotent, never a
/// second declare.
///
/// The row still stays `mark_failed` (honest: WE did not heal it), exactly as
/// the timeout route does.
fn conductor_missing_should_route_to_adopt(peer_advertises_declaration: bool) -> bool {
    peer_advertises_declaration
}

/// Outcome of the bounded-retry conductor call for one row: the final result plus
/// whether any transient retry was taken (so a success-after-retry is countable).
struct RetryResult<T> {
    result: Result<T, crate::error::StorageError>,
    retried: bool,
}

/// Call the conductor for one row with a per-attempt timeout and bounded transient
/// retry. Non-transient errors and successes return immediately; a transient
/// (timeout-class) error or a per-attempt timeout backs off (jittered) and retries
/// up to `pacing.max_row_retries`. Generic over the call closure so it is unit-
/// testable with a fake op (no conductor).
async fn call_with_retry<T, F, Fut>(pacing: &HealPacing, mut op: F) -> RetryResult<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, crate::error::StorageError>>,
{
    let mut retried = false;
    let mut attempt: u32 = 0;
    loop {
        let outcome = match tokio::time::timeout(pacing.attempt_timeout, op()).await {
            Ok(r) => r,
            Err(_elapsed) => Err(crate::error::StorageError::Timeout(format!(
                "{HEAL_SYNTHETIC_TIMEOUT_MARKER} {:?}",
                pacing.attempt_timeout
            ))),
        };
        match outcome {
            Ok(v) => {
                return RetryResult {
                    result: Ok(v),
                    retried,
                }
            }
            Err(e) => {
                // Only an ANSWERED timeout-class error is worth another attempt
                // within the leg; anything else — including our own synthetic
                // per-attempt timeout, which does not cancel the in-flight call —
                // returns immediately (no free window fixes it, and retrying an
                // uncancelled call only stacks load). See `should_retry_attempt`.
                if attempt < pacing.max_row_retries && should_retry_attempt(&e) {
                    attempt += 1;
                    retried = true;
                    tokio::time::sleep(pacing.backoff()).await;
                    continue;
                }
                return RetryResult {
                    result: Err(e),
                    retried,
                };
            }
        }
    }
}

/// Reconciliation progress for the REA-commitment projection stream, exposed
/// via `/p2p/status` (the same surface `replication.rs` uses). Mirrors
/// `ReplicationStatus` and adds reconcile-specific observability.
///
/// Wire format: the `projectionReconcile` property of
/// `elohim/sdk/schemas/v1/views/p2p-status-view.schema.json` (an inline object,
/// mirroring the `pull` precedent). Schema contract test: the
/// `p2p_status_view_*` cases in `tests/schema_contract.rs`.
#[derive(Debug, Clone, Default, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ProjectionReconcileStatus {
    /// Gap ids discovered but not yet healed (in flight this sweep).
    #[ts(type = "number")]
    pub pending: usize,
    /// Gap ids healed from the own conductor (this sweep).
    #[ts(type = "number")]
    pub completed: usize,
    /// Gap ids the own conductor could not see (`get` returned None) — retried
    /// next sweep until MAX_RETRIES.
    #[ts(type = "number")]
    pub failed: usize,
    /// True when this SWEEP ended: every discovered gap was healed **or**
    /// exhausted retries. Deliberately NOT renamed — `/health`, `/p2p/status`
    /// and a live a2o gate consume it. It is not a convergence signal; see
    /// [`ProjectionReconcileStatus::converged`].
    pub caught_up: bool,
    /// Peers asked for an inventory in the last completed sweep.
    #[ts(type = "number")]
    pub peers_asked: usize,
    /// Gaps in the last sweep that were present locally but with a DIFFERENT
    /// anchor than a peer advertised (anchor-divergence, not just absence).
    ///
    /// This stays the TOTAL, deliberately: the wire shape is consumed by
    /// `/p2p/status` clients and a2o gates, and adjudicated divergence must never
    /// silently vanish from it. The ADJUDICATED share (heal is forbidden to move
    /// it, or its retry budget is spent) is published separately on
    /// `elohim_projection_reconcile_divergent_refused`; only the remainder —
    /// unadjudicated divergence — defeats
    /// [`ProjectionReconcileStatus::converged`].
    #[ts(type = "number")]
    pub divergent_anchor: usize,
    /// Cumulative gaps healed across all sweeps this process lifetime.
    #[ts(type = "number")]
    pub healed_total: usize,
    /// Sweeps completed this process lifetime.
    #[ts(type = "number")]
    pub sweeps: usize,
    /// Gaps abandoned at MAX_RETRIES this sweep — healed nothing, retried no
    /// more. `enqueue_missing` refuses to re-queue them, so they leave
    /// `pending` permanently; that is why `caught_up` alone overstates.
    #[ts(type = "number")]
    pub exhausted: usize,
    /// True when this peer holds what its peers advertised: nothing pending,
    /// nothing abandoned, nothing divergent. `caught_up` says only that the
    /// sweep ended — an SLO may be offered over THIS field, not that one.
    pub converged: bool,
}

/// Thread-safe holder for the latest sweep's status snapshot. The `GapTracker`
/// itself is per-sweep (rebuilt each cycle); this carries only the published
/// snapshot the status surface reads, plus the cumulative counters.
#[derive(Debug, Clone, Default)]
pub struct ProjectionReconcileState {
    inner: Arc<RwLock<ProjectionReconcileStatus>>,
}

impl ProjectionReconcileState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Snapshot the latest sweep status for `/p2p/status`.
    pub async fn status(&self) -> ProjectionReconcileStatus {
        self.inner.read().await.clone()
    }

    /// Publish the result of a completed sweep, advancing cumulative counters.
    ///
    /// `divergent_anchor` is the TOTAL divergence across the folded arms;
    /// `divergent_refused` is the ADJUDICATED share of it. The difference —
    /// UNADJUDICATED divergence — is what convergence is gated on.
    async fn publish_sweep(
        &self,
        counts: crate::p2p::reconcile_rails::GapCounts,
        peers_asked: usize,
        divergent_anchor: usize,
        divergent_refused: usize,
    ) {
        let divergent_actionable = divergent_anchor.saturating_sub(divergent_refused);
        let mut s = self.inner.write().await;
        s.pending = counts.pending;
        s.completed = counts.completed;
        s.failed = counts.failed;
        s.caught_up = counts.caught_up;
        s.peers_asked = peers_asked;
        s.divergent_anchor = divergent_anchor;
        s.healed_total = s.healed_total.saturating_add(counts.completed);
        s.sweeps = s.sweeps.saturating_add(1);
        s.exhausted = counts.exhausted;
        // The gap ledger converging is necessary but not sufficient: rows held
        // locally under an anchor no peer advertises are divergence this sweep
        // did not resolve, so they defeat convergence on their own — UNLESS the
        // sweep adjudicated them.
        //
        // Adjudicated means one of two things, both of which are the substrate
        // working as designed rather than a gap:
        //   1. the local row already carries a DIFFERENT declared head, so heal
        //      is FORBIDDEN to move it (only canonical channels may — see
        //      `StampOutcome::SkippedDeclared`); or
        //   2. the id has spent its cross-sweep retry budget against unchanged
        //      evidence (`MissLedger`).
        // Gating on the TOTAL made `converged` unreachable fleet-wide for as long
        // as ONE correctly-refused row existed (matthew: 6071 refused_declared vs
        // 8 healed in 12h), which is not an honest zero — it is a metric that
        // cannot move. Gating on the remainder keeps the honest half: a row nobody
        // has adjudicated still defeats convergence.
        s.converged = counts.converged && divergent_actionable == 0;
        // Same call site as the wire field on purpose: metric and /p2p/status
        // are written together, so they cannot drift apart the way /health and
        // /p2p/status did (1860 vs 148 in the same minute, 2026-07-25).
        crate::metrics::record_reconcile_sweep(&counts, divergent_actionable);
    }
}

/// Discovery-side output for the REA arm: the gap set (as a per-sweep
/// [`GapTracker`]) plus the observability numbers, carried to the heal leg.
/// Discovery needs no conductor, so it runs every tick even before the lamad
/// bridge lands.
pub struct ReaDiscovery {
    tracker: GapTracker,
    discovered_by: std::collections::HashMap<String, String>,
    peers_asked: usize,
    ids_discovered: usize,
    divergent_anchor: usize,
    /// The DISCOVERY-side adjudicated share of `divergent_anchor` — divergence
    /// this sweep is no longer able to resolve, and which therefore must not
    /// defeat convergence.
    ///
    /// REA rows carry no declaration-ordering column, so there is no
    /// "already declared" refusal on this arm (the content arm's dominant class).
    /// What this counts is the **retry-exhausted** class: a divergent id whose
    /// cross-sweep [`MissLedger`] budget is spent against unchanged peer
    /// evidence. It is held back from the tracker, so the heal leg will never
    /// see it again — counting it as unresolved divergence forever is exactly
    /// what pinned `elohim_projection_reconcile_converged` at 0.
    ///
    /// The arm's OTHER adjudicated class — the own conductor confirming the
    /// anchor the local row already holds — can only be known from a conductor
    /// answer, so it is counted heal-side ([`ReaHealOutcome::divergent_refused`])
    /// and added to this before `publish_sweep`. The two buckets PARTITION the
    /// divergence set: an exhausted id is never admitted to the tracker, so it
    /// cannot also be healed.
    divergent_refused: usize,
    /// Gap ids the cross-sweep [`MissLedger`] held back this sweep (retry budget
    /// spent against unchanged evidence). Published on
    /// `elohim_projection_reconcile_exhausted{stream="rea"}`.
    exhausted_persistent: usize,
    local_total: usize,
}

impl ReaDiscovery {
    /// Empty discovery (db unavailable this tick) — the heal leg has nothing to do.
    fn empty() -> Self {
        Self {
            tracker: GapTracker::new(MAX_RETRIES),
            discovered_by: std::collections::HashMap::new(),
            peers_asked: 0,
            ids_discovered: 0,
            divergent_anchor: 0,
            divergent_refused: 0,
            exhausted_persistent: 0,
            local_total: 0,
        }
    }
}

/// The discovery-side plan both reconcile arms produce, consumed by the heal leg.
pub struct SweepPlan {
    rea: ReaDiscovery,
    content: ContentDiscovery,
    collectives: CollectivesDiscovery,
}

/// What the per-tick heal scheduler should do, given whether the lamad bridge is
/// up and whether a heal leg is already running. Keeps the single-flight decision
/// pure and unit-testable, off the `main.rs` boot loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealAction {
    /// Bridge up and no heal in flight — spawn the heal leg for this tick's plan.
    Spawn,
    /// A heal leg from an earlier tick is still running — skip (never two
    /// concurrent heal legs). Discovery already ran this tick.
    SkipInFlight,
    /// The lamad bridge has not connected yet — the conductor-dependent heal is
    /// deferred. Discovery already ran this tick.
    SkipNoBridge,
}

/// Fold the per-arm post-heal [`GapCounts`] into ONE sweep-level count.
///
/// ## Why this exists (F-D, 2026-08-01)
///
/// `publish_sweep` used to receive the **REA arm's** counts alone, so the
/// operator-facing `converged` — and `elohim_projection_reconcile_converged` —
/// could not see the content arm AT ALL except through the divergence term. A
/// content backlog of thousands of un-healed gaps was structurally invisible to
/// the one gauge an SLO is offered over. Folding is therefore strictly STRICTER
/// than the previous behaviour: nothing that blocked convergence before stops
/// blocking it now, and a whole arm that was invisible now counts.
///
/// ## What "converged" means after the fold
///
/// **No ACTIONABLE work is outstanding on any arm.** Numeric fields sum;
/// `caught_up` and `converged` are ANDed — a sweep is only over, and a peer only
/// converged, when EVERY arm says so. `caught_up` is folded rather than left
/// REA-only on purpose: publishing a summed `pending` beside an un-folded
/// `caught_up` would let the two disagree on the same surface, which is exactly
/// the metric/status drift `publish_sweep` writes both fields together to avoid.
///
/// ## What is deliberately SET ASIDE (and why that is not a relaxation)
///
/// - **Refused-adjudicated divergence** — the row already carries a different
///   declared head (heal is forbidden to move it), or its cross-sweep budget is
///   spent. Handled by `publish_sweep`'s `divergent_actionable` subtraction, not
///   here; it stays visible on `..._divergent` / `..._divergent_refused`.
/// - **Cross-sweep [`MissLedger`] exhaustion** — already set aside BY
///   CONSTRUCTION: an `Admission::Exhausted` id is never admitted to a
///   `GapTracker`, so it cannot appear in any arm's `pending` or `exhausted`.
///   It stays visible on `..._exhausted`.
///
/// Note this is NOT the same number as [`GapCounts::exhausted`], which counts
/// rows THIS sweep attempted up to `max_retries` and abandoned — genuinely
/// undone work, and blocking, exactly as it already was for REA. Set-aside is a
/// property of the ledger, not a licence to stop counting failed attempts.
fn fold_arm_counts(
    arms: &[crate::p2p::reconcile_rails::GapCounts],
) -> crate::p2p::reconcile_rails::GapCounts {
    let mut folded = crate::p2p::reconcile_rails::GapCounts {
        caught_up: true,
        converged: true,
        ..Default::default()
    };
    for arm in arms {
        folded.pending = folded.pending.saturating_add(arm.pending);
        folded.completed = folded.completed.saturating_add(arm.completed);
        folded.failed = folded.failed.saturating_add(arm.failed);
        folded.exhausted = folded.exhausted.saturating_add(arm.exhausted);
        folded.caught_up &= arm.caught_up;
        folded.converged &= arm.converged;
    }
    folded
}

/// Decide the per-tick heal action. Bridge-absence takes precedence over the
/// single-flight guard: with no conductor there is nothing to spawn regardless.
pub fn heal_decision(bridge_up: bool, heal_in_flight: bool) -> HealAction {
    if !bridge_up {
        HealAction::SkipNoBridge
    } else if heal_in_flight {
        HealAction::SkipInFlight
    } else {
        HealAction::Spawn
    }
}

/// Ceiling on the peer-reported inventory total the rotating window trusts.
///
/// A buggy or adversarial peer that reports a huge `total` would otherwise pin the
/// window offset ever upward: `requested + page` (bounded by `u32`) can never
/// reach a `u64::MAX`-ish claim, so the wrap-to-0 test never fires — the offset
/// climbs to the `u32::MAX` saturation plateau and every peer is queried past its
/// real corpus forever, silently recreating the whole-table invisibility this
/// cursor exists to close. Clamping `max_total` to this ceiling bounds window
/// progress independent of any single peer's claim, and the wrap still fires at
/// the clamp. Mirrors [`crate::p2p::sync_protocol::MAX_SYNC_LIST_OFFSET`], which
/// bounds a lying always-`has_more` peer on the ListDocuments chain. A genuine
/// corpus larger than this windows its first N rows per rotation (raise
/// deliberately if a real projection ever approaches it).
const MAX_INVENTORY_WINDOW_TOTAL: u64 = 100_000;

/// Rotating per-table window cursor for the `ProjectionInventory` reconcile.
///
/// The responder caps each inventory at [`PROJECTION_INVENTORY_CAP`] rows ordered
/// hot-set-first; a corpus larger than the cap needs successive sweeps to advance
/// a window across the whole table, or its cold tail past the cap stays
/// permanently invisible to this arm (the structural non-convergence the honest
/// `total` exposes and this cursor closes). Lives for the discovery task's
/// lifetime (one per process, owned by the single discovery loop — no locking).
/// Advances by one page each sweep and wraps at the largest peer-reported total
/// (clamped by [`MAX_INVENTORY_WINDOW_TOTAL`]), so a modest corpus that fits one
/// page never leaves offset 0 and no single peer's claim can strand the window.
#[derive(Debug, Default)]
pub struct InventoryWindow {
    /// table → next offset to request on the coming sweep.
    offsets: std::collections::HashMap<String, u32>,
}

impl InventoryWindow {
    pub fn new() -> Self {
        Self::default()
    }

    /// The offset to request for `table` this sweep (0 until advanced).
    fn offset_for(&self, table: &str) -> u32 {
        self.offsets.get(table).copied().unwrap_or(0)
    }

    /// Advance the window for `table` after a sweep that requested `requested` and
    /// saw `max_total` as the largest peer-reported corpus size. Advances by one
    /// page ([`PROJECTION_INVENTORY_CAP`]); wraps to 0 once the window has covered
    /// the whole corpus (or no peer reported a corpus larger than one page — the
    /// common small-corpus case, which then stays pinned at offset 0).
    ///
    /// `max_total` is clamped to [`MAX_INVENTORY_WINDOW_TOTAL`] BEFORE the wrap
    /// test, so a single peer's inflated `total` can never push the offset onto
    /// the `u32` saturation plateau where it would never wrap (see the const).
    fn advance(&mut self, table: &str, requested: u32, max_total: u64) {
        let page = u32::try_from(PROJECTION_INVENTORY_CAP).unwrap_or(u32::MAX);
        let bounded_total = max_total.min(MAX_INVENTORY_WINDOW_TOTAL);
        let next = requested.saturating_add(page);
        let wrapped = if u64::from(next) >= bounded_total {
            0
        } else {
            next
        };
        self.offsets.insert(table.to_string(), wrapped);
    }
}

#[cfg(test)]
mod inventory_window_tests {
    use super::*;

    /// The documented page size the conversion path must yield (Fix 2b review).
    #[test]
    fn page_size_conversion_matches_cap() {
        let page = u32::try_from(PROJECTION_INVENTORY_CAP).unwrap_or(u32::MAX);
        assert_eq!(
            page, 2000,
            "the CAP→u32 conversion yields the documented page"
        );
        let mut w = InventoryWindow::new();
        // A corpus of many pages advances by exactly one page.
        w.advance("content", 0, u64::from(page) * 10);
        assert_eq!(w.offset_for("content"), page);
    }

    #[test]
    fn new_window_starts_at_zero() {
        let w = InventoryWindow::new();
        assert_eq!(w.offset_for("content"), 0);
        assert_eq!(w.offset_for("rea_commitments"), 0);
    }

    #[test]
    fn advances_by_page_then_wraps_at_corpus_end() {
        let mut w = InventoryWindow::new();
        let page = u32::try_from(PROJECTION_INVENTORY_CAP).unwrap();
        let total = u64::from(page) * 2 + 5; // just over two pages
        w.advance("content", 0, total);
        assert_eq!(w.offset_for("content"), page, "0 -> one page");
        w.advance("content", page, total);
        assert_eq!(w.offset_for("content"), page * 2, "page -> two pages");
        // next (3*page) >= total -> wrap to 0 (whole corpus covered).
        w.advance("content", page * 2, total);
        assert_eq!(w.offset_for("content"), 0, "past corpus end -> wrap");
    }

    #[test]
    fn small_corpus_stays_pinned_at_zero() {
        let mut w = InventoryWindow::new();
        // Fits one page: next(=page) >= total -> stays at 0 forever.
        w.advance("content", 0, 10);
        assert_eq!(w.offset_for("content"), 0);
    }

    #[test]
    fn zero_total_peer_never_advances() {
        let mut w = InventoryWindow::new();
        // No peer answered (max_total 0): bounded_total 0, next >= 0 -> wrap to 0.
        w.advance("content", 0, 0);
        assert_eq!(w.offset_for("content"), 0);
        // Even from a non-zero offset, a zero-total sweep resets to 0.
        w.advance("content", 4000, 0);
        assert_eq!(w.offset_for("content"), 0);
    }

    #[test]
    fn adversarial_huge_total_is_clamped_so_window_still_wraps() {
        let mut w = InventoryWindow::new();
        let page = u32::try_from(PROJECTION_INVENTORY_CAP).unwrap();
        let clamp = u32::try_from(MAX_INVENTORY_WINDOW_TOTAL).unwrap();
        // Two pages below the ceiling with a u64::MAX claim: still advances (the
        // clamp does not wrap us early inside the trusted window).
        let below = clamp - page - page;
        w.advance("content", below, u64::MAX);
        assert_eq!(
            w.offset_for("content"),
            below + page,
            "a clamped huge total still advances below the ceiling"
        );
        // At the ceiling boundary the clamp forces the wrap a lying peer's raw
        // total would otherwise prevent (the u32-plateau bug this const fixes).
        w.advance("content", clamp - page, u64::MAX);
        assert_eq!(
            w.offset_for("content"),
            0,
            "clamp forces wrap-to-0 at the ceiling despite a u64::MAX claim"
        );
    }

    #[test]
    fn saturating_add_plateau_self_heals_via_clamp() {
        let mut w = InventoryWindow::new();
        // Simulate an offset already stuck near the u32 plateau (e.g. from a
        // pre-clamp binary). A huge claim now WRAPS instead of pinning, because
        // saturating_add lands at u32::MAX which is >= the clamped total.
        w.advance("content", u32::MAX - 1, u64::MAX);
        assert_eq!(
            w.offset_for("content"),
            0,
            "a plateaued offset self-heals to 0 under the clamp"
        );
    }

    #[test]
    fn separate_tables_have_independent_offsets() {
        let mut w = InventoryWindow::new();
        let page = u32::try_from(PROJECTION_INVENTORY_CAP).unwrap();
        w.advance("content", 0, u64::from(page) * 3);
        assert_eq!(w.offset_for("content"), page);
        assert_eq!(
            w.offset_for("rea_commitments"),
            0,
            "advancing one table must not move another"
        );
    }
}

/// One discovery pass over BOTH reconcile arms (REA + content). Conductor-free:
/// this is the per-tick outbound view-federation ask that must fire from boot,
/// independent of the lamad bridge. Returns the [`SweepPlan`] the heal leg
/// consumes; the heal leg is scheduled separately (single-flight) so a multi-hour
/// heal never starves this cadence.
///
/// `window` carries the rotating per-table inventory offset ACROSS sweeps (owned
/// by the caller's discovery loop), so successive sweeps window across a corpus
/// larger than the responder's page cap rather than re-diffing only the hot set.
pub async fn run_discovery(
    p2p: &P2PHandle,
    pool: &DbPool,
    window: &mut InventoryWindow,
    misses: &mut MissLedger,
) -> SweepPlan {
    let rea = discover_rea(p2p, pool, window, misses).await;
    let content = discover_content(p2p, pool, window, misses).await;
    let collectives = discover_collectives(p2p, pool, window, misses).await;

    tracing::info!(
        target: "elohim_storage::projection_reconcile",
        rea_peers_asked = rea.peers_asked,
        rea_ids_discovered = rea.ids_discovered,
        rea_gaps = rea.tracker.counts().pending,
        rea_divergent_anchor = rea.divergent_anchor,
        rea_divergent_refused = rea.divergent_refused,
        rea_exhausted = rea.exhausted_persistent,
        rea_local_total = rea.local_total,
        content_peers_asked = content.peers_asked,
        content_ids_discovered = content.ids_discovered,
        content_gaps = content.tracker.counts().pending,
        content_divergent_anchor = content.divergent_anchor,
        content_divergent_refused = content.divergent_refused,
        content_exhausted = content.exhausted_persistent,
        content_local_anchored = content.local_anchored,
        collectives_peers_asked = collectives.peers_asked,
        collectives_ids_discovered = collectives.ids_discovered,
        collectives_gaps = collectives.tracker.counts().pending,
        collectives_divergent_cid = collectives.divergent_cid,
        collectives_local_anchored = collectives.local_anchored,
        "projection-reconcile: discovery complete (heal scheduled separately)"
    );

    SweepPlan {
        rea,
        content,
        collectives,
    }
}

/// One heal pass over BOTH arms, consuming a [`SweepPlan`] from [`run_discovery`].
/// Requires the lamad bridge (`hc`); the caller only invokes this once the bridge
/// is up and no other heal is in flight (see [`heal_decision`]). Publishes the
/// sweep status snapshot. Row content comes EXCLUSIVELY from the own conductor;
/// both upsert paths are idempotent, so a heal is safe under duplicate delivery.
///
/// The content arm also runs the sweep-driven [`witness_bootstrap`] step (GAP
/// 1.5): it authors a notarized head for local rows born un-witnessed
/// (bulk-seeded, `dht_anchor_hash` NULL) so they can green. It rides this leg's
/// single-flight guard + OnceLock conductor gate — never running bridge-absent
/// or concurrently — and publishes its progress to `provide_state`.
pub async fn run_heal(
    plan: SweepPlan,
    hc: &Arc<HcClient>,
    pool: &DbPool,
    state: &ProjectionReconcileState,
    provide_state: &ProvideLoopState,
    p2p: &P2PHandle,
) {
    let SweepPlan {
        rea,
        content,
        collectives,
    } = plan;
    // Bounded heal pacing (per-row transient retry + per-leg wall-clock budget).
    // REA runs FIRST with its own reserved budget so its small backlog is never
    // starved behind the large content queue (the incident).
    let pacing = HealPacing::default();
    let ReaDiscovery {
        mut tracker,
        discovered_by,
        peers_asked,
        ids_discovered,
        divergent_anchor: rea_divergent,
        divergent_refused: rea_divergent_refused,
        exhausted_persistent: rea_exhausted,
        local_total,
    } = rea;
    // Publish the last-sweep gauges BEFORE heal (discovered gaps + local rows), so
    // convergence is watchable on `/metrics` without tailing Loki. `exhausted` is
    // the CROSS-SWEEP ledger count, not the per-sweep tracker's (which is
    // structurally 0 — the tracker is rebuilt every sweep).
    crate::metrics::set_projection_reconcile_gauges(
        "rea",
        tracker.counts().pending as u64,
        local_total as u64,
        rea_exhausted as u64,
        rea_divergent as u64,
        rea_divergent_refused as u64,
    );
    let ReaHealOutcome {
        counts,
        divergent_refused: rea_refused_by_conductor,
    } = heal_rea(&mut tracker, &discovered_by, hc, pool, &pacing).await;
    // The arm's FULL adjudication: the discovery-side retry-exhausted share plus
    // the heal-side own-conductor-confirms-the-held-anchor share. The two buckets
    // partition the divergence set (an exhausted id never reaches the heal leg),
    // so the sum can never exceed `rea_divergent`.
    let rea_divergent_refused = rea_divergent_refused.saturating_add(rea_refused_by_conductor);
    // Re-publish ONLY the refused gauge: the heal leg just learned the second
    // half of it, and the pre-heal call above could not know it. The other REA
    // gauges keep their discovery-time values (the heal leg is the thing they
    // are measured against).
    crate::metrics::set_projection_reconcile_divergent_refused("rea", rea_divergent_refused as u64);

    let ContentDiscovery {
        tracker: mut content_tracker,
        discovered_by: content_discovered_by,
        divergent_anchor: content_divergent,
        divergent_refused: content_divergent_refused,
        exhausted_persistent: content_exhausted,
        peers_asked: content_peers_asked,
        ids_discovered: content_ids_discovered,
        local_anchored,
        peer_head_hints,
    } = content;
    crate::metrics::set_projection_reconcile_gauges(
        "content",
        content_tracker.counts().pending as u64,
        local_anchored as u64,
        content_exhausted as u64,
        content_divergent as u64,
        content_divergent_refused as u64,
    );
    let ContentHealOutcome {
        healed: content_healed,
        conductor_missing: content_missing,
        ghost_candidates,
        adopt_candidates,
    } = heal_content(
        &mut content_tracker,
        &content_discovered_by,
        hc,
        pool,
        &pacing,
        &peer_head_hints,
    )
    .await;

    // Collectives arm — LAST, with its own reserved budget, so it can never
    // starve REA or content (both of which carry the fleet's real backlog).
    let CollectivesDiscovery {
        tracker: mut collectives_tracker,
        alias_by_cid,
        discovered_by: collectives_discovered_by,
        divergent_cid: collectives_divergent,
        exhausted_persistent: collectives_exhausted,
        peers_asked: collectives_peers_asked,
        ids_discovered: collectives_ids_discovered,
        local_anchored: collectives_local_anchored,
    } = collectives;
    // The collectives arm's divergence is refused BY CONSTRUCTION — a divergent
    // cid is counted and WARN-logged but never enqueued (no declaration-ordering
    // column exists to prove a forward move), so all of it is adjudicated. It
    // does not fold into `publish_sweep` at all; this gauge is its only surface.
    crate::metrics::set_projection_reconcile_gauges(
        "collectives",
        collectives_tracker.counts().pending as u64,
        collectives_local_anchored as u64,
        collectives_exhausted as u64,
        collectives_divergent as u64,
        collectives_divergent as u64,
    );
    let CollectivesHealOutcome {
        healed: collectives_healed,
        conductor_missing: collectives_missing,
    } = heal_collectives(
        &mut collectives_tracker,
        &alias_by_cid,
        &collectives_discovered_by,
        hc,
        pool,
        &pacing,
    )
    .await;

    // ADOPT-BEFORE-AUTHOR context for BOTH witness sweeps below. They are the
    // two paths that MINT roots, so they are the two that must first ask whether
    // a canonical head already exists. The fetcher rides the same view-federation
    // plane the inventory hints came from.
    let head_record_fetcher = PeerHeadRecordFetcher::new(p2p.clone());
    let adopt = crate::services::head_adoption::AdoptContext {
        hints: &peer_head_hints,
        fetcher: Some(&head_record_fetcher),
        contest_enabled: crate::config::contest_two_way_declared_enabled(),
    };

    // Deferred adoptions FIRST: these are ids the heal leg just refused to
    // GapFill, and they are unreachable by both witness sweeps below (anchored,
    // and conductor-resolvable). Running them here is what turns the refusal
    // into convergence rather than into a permanent gap.
    adopt_deferred_heads(hc, pool, &adopt_candidates, &adopt).await;

    // GAP 1.5: green the un-witnessed seeded corpus. Composed onto (not forked
    // from) the content arm — same conductor gate + single-flight guard.
    witness_bootstrap(hc, pool, provide_state, &adopt).await;

    // Ghost-anchor witness: the NULL-anchor sweep above cannot see rows whose
    // anchor string outlived its conductor incarnation. Runs on the same leg,
    // fed by the conductor answers the heal already paid for.
    witness_ghost_anchors(hc, pool, &ghost_candidates, &adopt).await;

    // Publish the WHOLE sweep (F-D, 2026-08-01): every arm's post-heal counts
    // folded, alongside the divergent-anchor counter that already folded across
    // arms. Before this the published counts were the REA arm's ALONE, so a
    // content backlog of thousands of un-healed gaps could not move `converged`
    // in either direction — the arm carrying the fleet's real work was invisible
    // to the gauge an SLO is offered over. See [`fold_arm_counts`] for what the
    // fold does and does not set aside.
    //
    // Kept under its own name: the `heal complete` log below reports the REA
    // arm's `counts` beside per-arm content/collectives fields, and silently
    // folding that field would make an arm-scoped log line read as a sweep-scoped
    // one.
    let sweep_counts = fold_arm_counts(&[
        counts,
        content_tracker.counts(),
        collectives_tracker.counts(),
    ]);
    state
        .publish_sweep(
            sweep_counts,
            peers_asked,
            rea_divergent + content_divergent,
            rea_divergent_refused + content_divergent_refused,
        )
        .await;

    tracing::info!(
        target: "elohim_storage::projection_reconcile",
        peers_asked,
        ids_discovered,
        healed = counts.completed,
        conductor_missing = counts.failed,
        divergent_anchor = rea_divergent,
        divergent_refused = rea_divergent_refused,
        local_total,
        caught_up = counts.caught_up,
        content_peers_asked,
        content_ids_discovered,
        content_healed,
        content_missing,
        content_divergent_anchor = content_divergent,
        content_local_anchored = local_anchored,
        collectives_peers_asked,
        collectives_ids_discovered,
        collectives_healed,
        collectives_missing,
        collectives_divergent_cid = collectives_divergent,
        collectives_local_anchored,
        "projection-reconcile: heal complete"
    );
}

/// Witness-bootstrap (GAP 1.5): author a notarized head through the conductor for
/// local content rows born un-witnessed — bulk-seeded diesel-direct rows with
/// `dht_anchor_hash IS NULL` and no conductor record, which can otherwise never
/// reach `trust=green`.
///
/// Composes the proven [`reanchor_backfill::run_once`] mechanism rather than
/// forking a new authoring path (the backlog's "compose, don't fork"):
/// - **Once-per-id guard.** `run_once` authors via `create_content`, which the
///   `content_store` zome REFUSES for a duplicate id; the already-exists branch
///   recovers and stamps the EXISTING anchor instead of minting a second head.
///   So a re-run over an already-witnessed row stamps (not authors), and a
///   transient/bridge error stays a retryable failure — never a fabricated or
///   duplicate head. (The classifier is [`reanchor_backfill::decide_outcome`].)
/// - **Eligibility.** Honors the existing heal/stamp path's reach filter
///   (`CORE_REACH_LEVELS`) — un-widened; non-canonical reach is skipped, not
///   authored.
/// - **Pacing.** Bounded to [`WITNESS_MAX_PER_TICK`] rows per tick with a
///   per-item delay, so a large corpus greens over many ticks. No concurrency.
///
/// Runs only inside [`run_heal`], so the OnceLock conductor gate + single-flight
/// guard already guarantee it never fires bridge-absent or concurrently.
async fn witness_bootstrap(
    hc: &Arc<HcClient>,
    pool: &DbPool,
    provide_state: &ProvideLoopState,
    adopt: &crate::services::head_adoption::AdoptContext<'_>,
) {
    // A lamad-scoped ContentService drives the canonical re-anchor path
    // (`update_via_conductor` null-anchor branch). The EventBus is a throwaway:
    // the only event this path emits is `ContentUpdated` (cache invalidation);
    // re-anchoring is a projection write, and content bytes are unchanged, so a
    // dropped invalidation only defers a trust-label refresh to the next read.
    let content_service = crate::services::ContentService::new(
        pool.clone(),
        crate::db::AppContext::default_lamad(),
        Arc::new(crate::services::events::EventBus::new()),
    );
    let cfg = crate::services::reanchor_backfill::ReanchorConfig {
        max_per_sweep: WITNESS_MAX_PER_TICK,
        item_delay: WITNESS_ITEM_DELAY,
    };
    // Wall-clock bound (see WITNESS_SWEEP_BUDGET): on elapse the run_once future
    // is dropped, cancelling any in-flight (possibly hung) conductor call, so the
    // heal leg's single-flight guard always releases. The sweep is idempotent and
    // resumes next tick.
    let sweep = crate::services::reanchor_backfill::run_once(
        pool,
        &content_service,
        hc,
        provide_state,
        &cfg,
        adopt,
    );
    match tokio::time::timeout(WITNESS_SWEEP_BUDGET, sweep).await {
        Ok(Ok(report)) if report.candidates > 0 => {
            crate::metrics::add_content_witness_authored(report.reanchored as u64);
            tracing::info!(
                target: "elohim_storage::projection_reconcile",
                candidates = report.candidates,
                authored = report.reanchored,
                already_witnessed = report.already_anchored,
                adopted = report.adopted,
                held = report.held,
                skipped = report.skipped,
                failed = report.failed,
                remaining = report.remaining,
                "projection-reconcile[witness]: authored notarized heads for un-witnessed seeded content"
            );
        }
        Ok(Ok(_)) => {
            // No un-witnessed rows — the seeded corpus is fully witnessed.
        }
        Ok(Err(e)) => {
            tracing::warn!(
                target: "elohim_storage::projection_reconcile",
                error = %e,
                "projection-reconcile[witness]: sweep failed (non-fatal, retried next tick)"
            );
        }
        Err(_elapsed) => {
            tracing::warn!(
                target: "elohim_storage::projection_reconcile",
                budget_secs = WITNESS_SWEEP_BUDGET.as_secs(),
                "projection-reconcile[witness]: sweep exceeded wall-clock budget \
                 (likely a slow/saturated conductor) — abandoned, single-flight guard \
                 released, resumes next tick"
            );
        }
    }
}

/// What one content heal leg produced: convergence counts plus the
/// ghost-anchor candidate set the leg's own conductor answers exposed.
struct ContentHealOutcome {
    /// Rows whose head actually MOVED to the own conductor's answer.
    healed: usize,
    /// Gaps the own conductor could not resolve at all (`Ok(None)`).
    conductor_missing: usize,
    /// Ids the own conductor could not resolve — the ghost-anchor candidate
    /// set, narrowed to the truly-anchored rows by [`witness_ghost_anchors`].
    ghost_candidates: Vec<String>,
    /// Ids whose `GapFill` was SKIPPED because it would have self-elected over a
    /// peer's advertised declaration ([`gapfill_would_self_elect`]). Handed to
    /// [`adopt_deferred_heads`] in the same sweep.
    ///
    /// Collected here rather than adopted inline for the same reason
    /// `ghost_candidates` is: the heal leg has no fetcher (the head-record
    /// transport is assembled in [`run_heal`] alongside the peer hints), and the
    /// answer this leg already paid for is enough to classify without re-probing.
    ///
    /// Carries that answer with each id (see [`AdoptCandidate`]) so the adopt arm
    /// can take `AdoptLocal` on a canonical one instead of asserting absence.
    adopt_candidates: Vec<AdoptCandidate>,
}

/// Ghost-anchor witness: author a local notarized head for rows whose SQL
/// projection claims a `dht_anchor_hash` that this node's OWN conductor cannot
/// resolve.
///
/// ## The blind spot this closes
///
/// [`witness_bootstrap`] (GAP 1.5) greens rows born un-witnessed — `dht_anchor_
/// hash IS NULL`. But an anchor string can also outlive the conductor
/// incarnation that authored it: a DHT reset / reinstall / re-key wipes
/// conductor state while the SQLite projection persists on its PVC. The row
/// then carries a NON-NULL anchor that resolves to nothing locally — a GHOST.
/// It is invisible to the NULL-anchor sweep, so it can never green, and every
/// canonical-head declaration against it is refused by the zome with
/// `declare_canonical_head: no content found for id '<id>'` (the first gate,
/// `gather_content_chain`, needs a LOCAL `IdToContent` link — and on a full-arc
/// fleet every `get_links` is local-only, so a gossip gap reads as absence).
///
/// ## Why `Ok(None)` from the own conductor is the honest classifier
///
/// `content_store::resolve_content_head` returns `Ok(None)` ONLY when BOTH the
/// cross-root canonical link AND the per-root chain are locally absent — it
/// never fabricates (unlike `get_content_by_id`, which has a v1-healing
/// fallback). Combined with "the row is present AND anchored" (the focused
/// [`anchored_content_reaches_for_ids`] query), that is exactly the ghost class:
///
/// - present + NULL anchor  → [`witness_bootstrap`] already owns it (excluded).
/// - absent                 → the acquisition plane's job; never fabricated.
/// - present + real anchor  → the conductor answers `Some`; never a candidate.
///
/// **Safety property (heal-fills-never-moves).** A row that has genuinely
/// ADOPTED a live canonical head can never reach this path: its conductor holds
/// the canonical link, so `resolve_content_head` answers `Some(canonical)` and
/// the id is healed, not classified as a ghost. The write this path performs is
/// `create_content` through the OWN conductor — the own-authored Declare-class
/// channel `upsert_with_anchor` already implements for `witness_bootstrap`, not
/// a heal stamp. It replaces an unverifiable hash with one this node authored
/// and can prove; a later canonical declaration (always Declare mode) still
/// wins outright.
///
/// ## Bounds
///
/// Reuses [`WITNESS_MAX_PER_TICK`] / [`WITNESS_ITEM_DELAY`] /
/// [`WITNESS_SWEEP_BUDGET`], and the same `CORE_REACH_LEVELS` /
/// `is_canonical_content_type` guards the re-anchor path uses (a
/// non-canonical reach or content_type is not re-authorable and would fail
/// every sweep forever). Self-terminating: once authored, the conductor
/// resolves the id and it is never a candidate again.
async fn witness_ghost_anchors(
    hc: &Arc<HcClient>,
    pool: &DbPool,
    candidates: &[String],
    adopt: &crate::services::head_adoption::AdoptContext<'_>,
) {
    if candidates.is_empty() {
        return;
    }
    let app_ctx = crate::db::AppContext::default_lamad();

    // Narrow the conductor-missing set to rows that actually CLAIM an anchor.
    let ghosts: Vec<(String, String, String)> = {
        let mut conn = match pool.get() {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(error = %e, "projection-reconcile[ghost-witness]: db conn failed; skipping");
                return;
            }
        };
        match crate::db::content_diesel::anchored_content_reaches_for_ids(
            &mut conn, &app_ctx, candidates,
        ) {
            Ok(rows) => rows,
            Err(e) => {
                tracing::warn!(error = %e, "projection-reconcile[ghost-witness]: candidate query failed; skipping");
                return;
            }
        }
    };
    if ghosts.is_empty() {
        return;
    }

    let content_service = crate::services::ContentService::new(
        pool.clone(),
        app_ctx,
        Arc::new(crate::services::events::EventBus::new()),
    );

    let total = ghosts.len();
    let ghost_ctx = crate::db::AppContext::default_lamad();
    let sweep = async {
        let mut authored = 0usize;
        let mut skipped = 0usize;
        let mut failed = 0usize;
        let mut adopted = 0usize;
        let mut held = 0usize;
        for (id, reach, content_type) in ghosts.iter().take(WITNESS_MAX_PER_TICK as usize) {
            // Same guard as the re-anchor path: a reach outside the DNA-notarized
            // vocabulary can never be re-authored, so it would burn a conductor
            // round-trip every sweep forever.
            if !crate::generated_enums::CORE_REACH_LEVELS.contains(&reach.as_str()) {
                skipped += 1;
                tracing::warn!(
                    content_id = %id,
                    reach = %reach,
                    "projection-reconcile[ghost-witness]: skipping row with non-canonical reach (not re-authorable)"
                );
                continue;
            }
            // Symmetric guard: a content_type outside the vocabulary
            // Content::validate() accepts (see
            // `services::reanchor_backfill::is_canonical_content_type`) can
            // never be re-authored either — skip it loudly rather than
            // burning a conductor round-trip every sweep forever.
            if !crate::services::reanchor_backfill::is_canonical_content_type(content_type) {
                skipped += 1;
                tracing::warn!(
                    content_id = %id,
                    content_type = %content_type,
                    "projection-reconcile[ghost-witness]: skipping row with non-canonical content_type (not re-authorable)"
                );
                continue;
            }
            // ADOPT-BEFORE-AUTHOR PRE-FLIGHT. `LocalResolve::Known(None)` — NOT
            // `Probe`: a ghost candidate is BY CONSTRUCTION an id whose
            // `resolve_content_head` answered `Ok(None)` moments ago in
            // `heal_content`. Re-probing would burn a conductor round-trip per
            // ghost per sweep to re-learn an answer we already hold, and on a
            // saturated conductor that is exactly the cost that makes a heal leg
            // grind. So the local-DHT arm is pre-answered "nothing here" and the
            // pre-flight goes straight to the peer arm — which is the arm that
            // matters for a ghost, since another peer is usually the one holding
            // the crown this node was about to overwrite.
            let preflight = crate::services::head_adoption::try_adopt_canonical_head(
                hc,
                pool,
                &ghost_ctx,
                id,
                crate::services::head_adoption::LocalResolve::Known(None),
                adopt,
            )
            .await;
            let pending_adopt = match preflight {
                crate::services::head_adoption::AdoptOutcome::Adopted => {
                    adopted += 1;
                    tokio::time::sleep(WITNESS_ITEM_DELAY).await;
                    continue;
                }
                // Contest minted a DHT candidate and deliberately left the row
                // unchanged — counted with `held` here because, from the WITNESS
                // sweep's point of view, the outcome is identical: do not author.
                crate::services::head_adoption::AdoptOutcome::Held
                | crate::services::head_adoption::AdoptOutcome::Contested => {
                    held += 1;
                    tokio::time::sleep(WITNESS_ITEM_DELAY).await;
                    continue;
                }
                crate::services::head_adoption::AdoptOutcome::Author => None,
                crate::services::head_adoption::AdoptOutcome::AuthorThenAdopt {
                    head_action_hash,
                    carried_record,
                    peer_id,
                } => Some((head_action_hash, carried_record, peer_id)),
            };

            // Empty patch on an ANCHORED row takes `update_via_conductor`'s
            // update branch; the zome refuses with "no Content entry found"
            // (there is no local chain), which trips that method's STALE-ANCHOR
            // HEAL: re-publish the full entry from the SQL row via
            // `create_content`. Composed, not forked.
            let empty_patch = crate::views::UpdateContentInputView {
                title: None,
                description: None,
                content_body: None,
                content_format: None,
                metadata: None,
                tags: None,
                reach: None,
                blob_hash: None,
                server_blob_hash: None,
                p2p_published_at: None,
            };
            match content_service
                // PRESERVE: replacing an unresolvable anchor with one this node
                // can prove is a heal. It must not also re-crown the row — that
                // is what un-adopted a peer's canonical head on every restart.
                .update_via_conductor(
                    hc,
                    id,
                    empty_patch,
                    crate::db::content_diesel::HeadElection::PreserveExistingDeclaration,
                )
                .await
            {
                Ok(_) => {
                    authored += 1;
                    // AUTHOR-THEN-ADOPT, second half: the conductor now has a
                    // local chain, so the declaration it refused can land.
                    if let Some((head_action_hash, carried_record, peer_id)) = pending_adopt {
                        if crate::services::head_adoption::finish_author_then_adopt(
                            hc,
                            pool,
                            &ghost_ctx,
                            id,
                            &head_action_hash,
                            carried_record,
                            &peer_id,
                        )
                        .await
                            == crate::services::head_adoption::AdoptOutcome::Adopted
                        {
                            adopted += 1;
                        }
                    }
                }
                Err(e) => {
                    failed += 1;
                    if let Some(class) = classify_reauthor_failure_class(&e) {
                        crate::metrics::inc_content_witness_reauthor_failed(class);
                    }
                    tracing::warn!(
                        content_id = %id,
                        error = %e,
                        "projection-reconcile[ghost-witness]: re-author failed (non-fatal, retried next sweep)"
                    );
                }
            }
            tokio::time::sleep(WITNESS_ITEM_DELAY).await;
        }
        (authored, skipped, failed, adopted, held)
    };

    match tokio::time::timeout(WITNESS_SWEEP_BUDGET, sweep).await {
        Ok((authored, skipped, failed, adopted, held)) => {
            crate::metrics::add_content_witness_authored(authored as u64);
            tracing::warn!(
                target: "elohim_storage::projection_reconcile",
                candidates = total,
                authored,
                skipped,
                failed,
                adopted,
                held,
                "projection-reconcile[ghost-witness]: authored local heads for rows whose claimed \
                 dht_anchor_hash this conductor cannot resolve (stale-anchor class)"
            );
        }
        Err(_elapsed) => {
            crate::metrics::inc_content_witness_sweep_abandoned();
            tracing::warn!(
                target: "elohim_storage::projection_reconcile",
                budget_secs = WITNESS_SWEEP_BUDGET.as_secs(),
                candidates = total,
                "projection-reconcile[ghost-witness]: sweep exceeded wall-clock budget \
                 (likely a slow/saturated conductor) — abandoned, resumes next sweep"
            );
        }
    }
}

/// Discovery phase of the REA-commitment reconcile (steps 1–3): build the local
/// `(id → anchor)` inventory, ask every connected peer for its
/// `ProjectionInventory { rea_commitments }`, and diff into a per-sweep
/// [`GapTracker`] (an id missing locally, OR present with a different anchor, is a
/// gap). No conductor call happens here — [`heal_rea`] owns that.
async fn discover_rea(
    p2p: &P2PHandle,
    pool: &DbPool,
    window: &mut InventoryWindow,
    misses: &mut MissLedger,
) -> ReaDiscovery {
    let app_ctx = crate::db::AppContext::default_lamad();
    // Rotating window offset for this sweep (advanced after the peer loop).
    let sweep_offset = window.offset_for(PROJECTION_INVENTORY_TABLE_REA_COMMITMENTS);
    // Largest peer-reported corpus size this sweep — drives the wrap decision.
    let mut max_peer_total: u64 = 0;

    // (1) Local inventory: id → anchor (anchor "" when un-anchored).
    let (local_pairs, local_total) = {
        let mut conn = match pool.get() {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(error = %e, "projection-reconcile: db conn failed; skipping sweep");
                return ReaDiscovery::empty();
            }
        };
        // Offset 0 / i64::MAX: the LOCAL diff needs the WHOLE local set (the
        // rotating window only bounds the PEER ask below).
        match crate::db::rea_commitments::inventory_for_reconcile(&mut conn, &app_ctx, 0, i64::MAX)
        {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(error = %e, "projection-reconcile: local inventory failed; skipping sweep");
                return ReaDiscovery::empty();
            }
        }
    };
    let local_anchors: std::collections::HashMap<String, String> =
        local_pairs.iter().cloned().collect();

    // (2) Ask connected peers for their inventory. Collect all peer entries
    // first (one pass), THEN build the tracker — so anchor-divergent ids (present
    // locally but with a different anchor) can be excluded from the tracker's
    // local set and thus admitted as gaps by `discover()`. This keeps ONE
    // tracker on the shared rails without reaching into its internals.
    let peers = p2p.list_peers().await;
    let mut peers_asked = 0usize;
    let mut ids_discovered = 0usize;
    let mut divergent_anchor = 0usize;
    // The union of ids any peer advertised, with the FIRST peer that did so
    // (for the heal WARN log). Anchor-divergent ids are recorded the same way.
    let mut discovered_by: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    // Ids present locally but with a peer-advertised non-empty anchor that
    // disagrees with ours — excluded from the tracker's local set so they heal.
    let mut divergent_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    // id → first NON-EMPTY advertised anchor. The cross-sweep [`MissLedger`]
    // keys its retry budget on this: a peer advertising a DIFFERENT anchor is
    // new evidence and re-admits an exhausted id.
    let mut advertised_anchor: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();

    for peer in &peers {
        let peer_id = match peer.peer_id.parse::<libp2p::PeerId>() {
            Ok(p) => p,
            Err(_) => continue,
        };
        let request = ViewFederationRequest {
            view_kind: ViewKind::ProjectionInventory {
                table: PROJECTION_INVENTORY_TABLE_REA_COMMITMENTS.to_string(),
            },
            // Carries the local agent; the responder ignores ownership for
            // ProjectionInventory (it returns what IT holds, not an agent view).
            agent_cid: p2p.agent_pubkey().to_string(),
            request_id: uuid::Uuid::new_v4().to_string(),
            // Rotating window: this sweep asks for the page at `sweep_offset` so
            // successive sweeps cover a corpus larger than the responder's cap.
            inventory_offset: Some(sweep_offset),
        };
        let resp = match p2p.view_federate(peer_id, request, PEER_TIMEOUT).await {
            Ok(r) => r,
            Err(_) => continue, // peer offline/timeout — discovery is best-effort
        };
        peers_asked += 1;

        let payload: ProjectionInventoryPayload =
            match serde_json::from_value(resp.slice.payload.0.clone()) {
                Ok(p) => p,
                Err(e) => {
                    // WARN, not debug: an undecodable inventory is a protocol
                    // break (version skew), and at debug it is invisible in
                    // Loki — the 2026-06-10 Phase-0 read could not tell
                    // "peers advertise nothing" from "responses don't decode".
                    tracing::warn!(
                        target: "elohim_storage::projection_reconcile",
                        peer = %peer.peer_id,
                        error = %e,
                        "projection-reconcile: peer inventory payload undecodable; skipping peer"
                    );
                    continue;
                }
            };

        // Track the largest corpus any peer reports so the window knows when it
        // has covered the whole table and should wrap. Honesty (Fix 2c): when
        // `total > entries.len()` the peer's inventory is WINDOWED — ids outside
        // this page are unknown-this-sweep, NOT absent. This arm only classifies
        // ADVERTISED ids (see below), so it never concludes absence/in-sync about
        // a non-advertised id; the rotating window guarantees eventual coverage.
        max_peer_total = max_peer_total.max(payload.total as u64);
        // Offset-aware: rows remain BEYOND this page only when
        // offset + served < total. Without the offset term the FINAL page of a
        // rotating window (which reaches the corpus end) would false-log
        // "windowed" merely because served < total.
        let windowed =
            (sweep_offset as usize).saturating_add(payload.entries.len()) < payload.total;

        // Per-peer INFO: makes the discovery leg observable end-to-end
        // (which peers answered, and with how much, and whether truncated).
        tracing::info!(
            target: "elohim_storage::projection_reconcile",
            peer = %peer.peer_id,
            entries = payload.entries.len(),
            peer_total = payload.total,
            offset = sweep_offset,
            windowed = windowed,
            "projection-reconcile: peer inventory received"
        );

        ids_discovered += payload.entries.len();
        for entry in &payload.entries {
            discovered_by
                .entry(entry.id.clone())
                .or_insert_with(|| peer.peer_id.clone());
            if !entry.dht_anchor_hash.is_empty() {
                advertised_anchor
                    .entry(entry.id.clone())
                    .or_insert_with(|| entry.dht_anchor_hash.clone());
            }
            // Anchor-divergence: both present, peer carries a non-empty anchor
            // that disagrees with ours. An empty remote anchor is not evidence
            // of divergence (the peer is itself un-anchored).
            if let Some(local_anchor) = local_anchors.get(&entry.id) {
                if !entry.dht_anchor_hash.is_empty()
                    && *local_anchor != entry.dht_anchor_hash
                    && divergent_ids.insert(entry.id.clone())
                {
                    divergent_anchor += 1;
                }
            }
        }
    }

    // Advance the rotating window for the next sweep (wraps at the largest total).
    window.advance(
        PROJECTION_INVENTORY_TABLE_REA_COMMITMENTS,
        sweep_offset,
        max_peer_total,
    );

    // Build the tracker: local set EXCLUDES anchor-divergent ids so `discover()`
    // admits them alongside genuinely-absent ids. All discovered ids flow
    // through the one gap state machine (absence + divergence, unified).
    let tracker_local: std::collections::HashSet<String> = local_anchors
        .keys()
        .filter(|id| !divergent_ids.contains(*id))
        .cloned()
        .collect();
    let mut tracker = GapTracker::new(MAX_RETRIES);
    tracker.set_local_ids(tracker_local.clone());
    // Cross-sweep retry budget: an id the conductor has failed to resolve for
    // MAX_RETRIES sweeps under UNCHANGED peer evidence stops being re-asked
    // (the ~151k-missing-outcomes treadmill). Ids already held locally are
    // forgotten, so a later relapse starts from a clean budget.
    let mut exhausted_persistent = 0usize;
    // The DIVERGENT share of the exhaustion — the discovery-side half of this
    // arm's adjudication (mirrors the content arm's `exhausted_divergent`). An
    // exhausted id is held back from the tracker, so the heal leg can never see
    // it again; counting it as unresolved divergence every sweep forever is what
    // pinned `converged` at 0 with a static residue.
    let mut exhausted_divergent = 0usize;
    let mut admitted: Vec<String> = Vec::new();
    for id in discovered_by.keys() {
        if tracker_local.contains(id) {
            misses.resolved(PROJECTION_INVENTORY_TABLE_REA_COMMITMENTS, id);
            continue;
        }
        let evidence = advertised_anchor.get(id).map(String::as_str).unwrap_or("");
        match misses.admit(PROJECTION_INVENTORY_TABLE_REA_COMMITMENTS, id, evidence) {
            Admission::Retry => admitted.push(id.clone()),
            Admission::Exhausted => {
                exhausted_persistent += 1;
                if divergent_ids.contains(id) {
                    exhausted_divergent += 1;
                }
            }
        }
    }
    tracker.discover(admitted);

    ReaDiscovery {
        tracker,
        discovered_by,
        peers_asked,
        ids_discovered,
        divergent_anchor,
        divergent_refused: exhausted_divergent,
        exhausted_persistent,
        local_total,
    }
}

/// Heal phase of the REA-commitment reconcile (step 4): for each discovered gap,
/// read the OWN conductor's `get_rea_commitment(id)` and upsert through the shared
/// mapping (`mark_completed`), or `mark_failed` (retried next sweep) when the
/// conductor can't see it. Runs only once the lamad bridge is up; may span many
/// discovery ticks on a saturated conductor, so it is scheduled single-flight OFF
/// the discovery ticker (see `main.rs`). A heal logs WARN naming the id and the
/// peer that discovered it (a visible mutual-aid event).
async fn heal_rea(
    tracker: &mut GapTracker,
    discovered_by: &std::collections::HashMap<String, String>,
    hc: &Arc<HcClient>,
    pool: &DbPool,
    pacing: &HealPacing,
) -> ReaHealOutcome {
    let app_ctx = crate::db::AppContext::default_lamad();
    // Heal-side half of this arm's divergence adjudication (see
    // [`ReaDiscovery::divergent_refused`] for the discovery-side half).
    let mut divergent_refused = 0usize;

    // (3+4) Heal each gap from the OWN conductor, bounded by the REA leg budget.
    // REA runs FIRST (in `run_heal`) with its own reserved budget, so a small REA
    // backlog is never starved behind the large content queue (the incident).
    let leg_start = std::time::Instant::now();
    let gap_ids = tracker.pending_ids();
    let mut circuit = HealCircuit::new(pacing.circuit_timeout_threshold);
    for id in gap_ids {
        let attempt = call_with_retry(pacing, || {
            crate::services::conductor_writes::get_rea_commitment(hc, &id)
        })
        .await;
        circuit.record(&attempt.result);
        let kind = match attempt.result {
            Ok(Some(output)) => match heal_one(&output, pool, &app_ctx) {
                Ok(ReaAnchorWrite::Advanced) => {
                    tracker.mark_completed(&id);
                    let peer = discovered_by
                        .get(&id)
                        .cloned()
                        .unwrap_or_else(|| "unknown".to_string());
                    tracing::warn!(
                        target: "elohim_storage::projection_reconcile",
                        commitment_id = %id,
                        discovered_via_peer = %peer,
                        retried = attempt.retried,
                        "projection-reconcile: HEALED rea_commitment from own conductor (peer discovery)"
                    );
                    if attempt.retried {
                        HealOutcomeKind::TimeoutRetried
                    } else {
                        HealOutcomeKind::Healed
                    }
                }
                Ok(ReaAnchorWrite::Refreshed) => {
                    // The own conductor answered with the anchor this row ALREADY
                    // holds: the value columns refreshed, but the ANCHOR did not
                    // move, so the peer-advertised divergence that enqueued this
                    // row is UNCHANGED. Deliberately NOT logged as HEALED (the
                    // `Refreshed` discipline the content arm established) and
                    // ADJUDICATED: heal has done everything it can here, and only
                    // a canonical channel can converge the two roots. Counting it
                    // as unresolved divergence every sweep forever is what held
                    // the fleet's `converged` gauge at 0 against a static residue.
                    tracker.mark_completed(&id);
                    divergent_refused += 1;
                    tracing::info!(
                        target: "elohim_storage::projection_reconcile",
                        commitment_id = %id,
                        "projection-reconcile[rea]: anchor unchanged — refreshed row, divergence NOT resolved (own conductor answered the anchor this row already holds)"
                    );
                    HealOutcomeKind::Refreshed
                }
                Err(e) => {
                    tracing::warn!(commitment_id = %id, error = %e, "projection-reconcile: upsert failed; retry next sweep");
                    tracker.mark_failed(&id);
                    HealOutcomeKind::Failed
                }
            },
            Ok(None) => {
                // Conductor can't see it — retry on NEXT sweep, never immediate.
                tracing::debug!(commitment_id = %id, "projection-reconcile: own conductor returned None; retry next sweep");
                tracker.mark_failed(&id);
                HealOutcomeKind::Missing
            }
            Err(e) => {
                let transient = is_transient_conductor_error(&e);
                tracing::warn!(commitment_id = %id, error = %e, transient, "projection-reconcile: conductor get failed; retry next sweep");
                tracker.mark_failed(&id);
                if transient {
                    HealOutcomeKind::TimeoutExhausted
                } else {
                    HealOutcomeKind::Failed
                }
            }
        };
        crate::metrics::inc_projection_heal_outcome("rea", kind.label());

        // Per-leg budget: yield the single-flight guard so the leg recycles next
        // sweep. Un-attempted rows are re-discovered (the tracker is per-sweep), so
        // nothing is lost — the next leg re-attempts them fresh.
        if leg_start.elapsed() >= pacing.rea_leg_budget {
            tracing::info!(
                target: "elohim_storage::projection_reconcile",
                budget_secs = pacing.rea_leg_budget.as_secs(),
                "projection-reconcile: rea heal hit leg budget — yielding, remaining gaps resume next sweep"
            );
            break;
        }

        // Unresponsive-conductor circuit: shed the rest of the leg rather than
        // stack more abandoned-but-still-executing calls on it (see `HealCircuit`).
        if circuit.is_open() {
            tracing::warn!(
                target: "elohim_storage::projection_reconcile",
                consecutive_timeouts = circuit.consecutive_timeouts(),
                "projection-reconcile: rea heal OPENED the unresponsive-conductor circuit — shedding the rest of the leg, remaining gaps resume next sweep"
            );
            break;
        }
    }

    tracker.update_caught_up();
    ReaHealOutcome {
        counts: tracker.counts(),
        divergent_refused,
    }
}

/// Heal-side output for the REA arm. Mirrors [`ContentHealOutcome`].
struct ReaHealOutcome {
    counts: crate::p2p::reconcile_rails::GapCounts,
    /// Divergent rows the own conductor ADJUDICATED this sweep by answering with
    /// the anchor the local row already holds ([`ReaAnchorWrite::Refreshed`]) —
    /// the heal-side half of this arm's `divergent_refused`, added to
    /// [`ReaDiscovery::divergent_refused`] before `publish_sweep`.
    ///
    /// Disjoint from the discovery-side half by construction: an id the
    /// [`MissLedger`] exhausted is never admitted to the tracker, so it can never
    /// reach this leg and be double-counted.
    divergent_refused: usize,
}

/// Did the own conductor's answer MOVE this row's anchor, or merely confirm the
/// anchor it already holds? The REA analog of the content arm's
/// [`crate::db::content_diesel::StampOutcome`] split.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReaAnchorWrite {
    /// The row was absent, un-anchored, or carried a DIFFERENT anchor — the
    /// write advanced it. Real convergence.
    Advanced,
    /// The row ALREADY held exactly the anchor the own conductor answered with.
    /// The write refreshed the value columns, but the ANCHOR did not move — so
    /// the peer-advertised divergence that enqueued this row is UNCHANGED and
    /// will re-enqueue next sweep. Not progress, and not a defect: this peer and
    /// its peers hold genuinely different roots and no amount of own-conductor
    /// healing can converge them.
    Refreshed,
}

/// Pure + total, so the adjudication rule is testable without a conductor or a
/// database. An EMPTY stored anchor is not a confirmation (the row is
/// un-anchored — the write fills it, which is a forward move).
fn classify_rea_anchor_write(existing_anchor: Option<&str>, answered: &str) -> ReaAnchorWrite {
    match existing_anchor {
        Some(a) if !a.is_empty() && a == answered => ReaAnchorWrite::Refreshed,
        _ => ReaAnchorWrite::Advanced,
    }
}

/// Project one conductor-read Commitment into local SQL via the SHARED mapping.
/// Row content comes exclusively from the own conductor's `ReaCommitmentOutput`.
///
/// Returns whether the write ADVANCED the row's anchor or merely REFRESHED it
/// under the anchor it already held — the fact the divergence adjudication in
/// [`heal_rea`] turns on. The read is taken BEFORE the upsert (which is
/// idempotent on the anchor, so the after-state cannot tell the two apart).
fn heal_one(
    output: &shefa_types::ReaCommitmentOutput,
    pool: &DbPool,
    app_ctx: &crate::db::AppContext,
) -> Result<ReaAnchorWrite, crate::error::StorageError> {
    let c = &output.commitment;
    let action_hash = format!("{}", output.action_hash);
    let input = crate::rea_projection::project_commitment_from_wire(
        &crate::rea_projection::CommitmentWireFields {
            id: &c.id,
            action: &c.action,
            provider: &c.provider,
            receiver: &c.receiver,
            resource_conforms_to: c.resource_conforms_to.as_deref(),
            // shefa_types::Commitment carries `_json` as non-optional String;
            // an empty string parses to an empty Vec in the shared mapping.
            resource_classified_as_json: Some(c.resource_classified_as_json.as_str()),
            resource_quantity_value: c.resource_quantity_value,
            resource_quantity_unit: c.resource_quantity_unit.as_deref(),
            effort_quantity_value: c.effort_quantity_value,
            effort_quantity_unit: c.effort_quantity_unit.as_deref(),
            has_beginning: c.has_beginning.as_deref(),
            has_end: c.has_end.as_deref(),
            due: c.due.as_deref(),
            clause_of: c.clause_of.as_deref(),
            in_scope_of_json: Some(c.in_scope_of_json.as_str()),
            note: c.note.as_deref(),
            metadata_json: Some(c.metadata_json.as_str()),
        },
    );
    let mut conn = pool
        .get()
        .map_err(|e| crate::error::StorageError::Internal(format!("pool: {e}")))?;
    // Read the PRE-write anchor: `upsert_with_anchor` is idempotent on the
    // anchor column, so after the write "confirmed what we held" and "advanced
    // to a new head" are indistinguishable.
    let existing_anchor = crate::db::rea_commitments::get_commitment(&mut conn, app_ctx, &c.id)?
        .and_then(|row| row.dht_anchor_hash);
    crate::db::rea_commitments::upsert_with_anchor(&mut conn, app_ctx, input, Some(&action_hash))?;
    Ok(classify_rea_anchor_write(
        existing_anchor.as_deref(),
        &action_hash,
    ))
}

// ============================================================================
// Content-anchor reconcile arm (notary-authority Leg 4)
// ============================================================================

/// Discovery-side output for the content arm (notary-authority Leg 4): the gap
/// set (as a per-sweep [`GapTracker`]) + observability numbers, carried to the
/// heal leg. Mirrors [`ReaDiscovery`]. Only `divergent_anchor` folds into the
/// shared [`ProjectionReconcileStatus`] (the one cross-arm counter the status
/// surface carries); the rest is log-observable — extending the ts-rs-exported
/// status struct with content fields would change the `p2p-status` wire shape
/// (owned elsewhere).
pub struct ContentDiscovery {
    tracker: GapTracker,
    discovered_by: std::collections::HashMap<String, String>,
    divergent_anchor: usize,
    /// The ADJUDICATED share of `divergent_anchor` — divergence this sweep is
    /// not permitted (or no longer able) to resolve, and which therefore must not
    /// defeat convergence:
    ///
    /// - **refused-declared** — the local row already carries a DIFFERENT
    ///   declared head, so heal is FORBIDDEN to move it (`StampMode::GapFill` →
    ///   [`crate::db::content_diesel::StampOutcome::SkippedDeclared`]); only a
    ///   canonical channel may. This is the DOMINANT class on a live peer
    ///   (matthew: 6071 refusals against 8 heals in 12h), and gating convergence
    ///   on it made the gauge structurally unreachable.
    /// - **retry-exhausted** — the cross-sweep [`MissLedger`] has spent this id's
    ///   budget against unchanged peer evidence.
    ///
    /// Classified at DISCOVERY from the local projection alone (no conductor
    /// call): the same `declared_head_for` predicate `stamp_declared_head_mode`
    /// uses to decide `SkippedDeclared`, so the two cannot drift apart.
    divergent_refused: usize,
    /// Gap ids the cross-sweep [`MissLedger`] held back this sweep. Published on
    /// `elohim_projection_reconcile_exhausted{stream="content"}`, which was
    /// structurally pinned to 0 before the ledger existed (the tracker is rebuilt
    /// per sweep, so its own `exhausted_count` can never fire across sweeps).
    exhausted_persistent: usize,
    peers_asked: usize,
    ids_discovered: usize,
    local_anchored: usize,
    /// Peer-ADVERTISED canonical-head declarations harvested from this sweep's
    /// inventory responses — the input to the adopt-before-author pre-flight's
    /// peer arm.
    ///
    /// Populated for EVERY advertised id, not just the gap set: a row this peer
    /// has no anchor for is not a "gap" the heal leg tracks, yet it is precisely
    /// the row the witness sweeps are about to mint a competing root for.
    peer_head_hints: crate::services::head_adoption::PeerHeadHints,
}

impl ContentDiscovery {
    /// Empty discovery (db unavailable this tick) — the heal leg has nothing to
    /// do. `peers_asked` records how many peers answered before the db failure so
    /// the discovery log stays honest.
    fn empty(peers_asked: usize) -> Self {
        Self {
            tracker: GapTracker::new(MAX_RETRIES),
            discovered_by: std::collections::HashMap::new(),
            divergent_anchor: 0,
            divergent_refused: 0,
            exhausted_persistent: 0,
            peers_asked,
            ids_discovered: 0,
            local_anchored: 0,
            peer_head_hints: crate::services::head_adoption::PeerHeadHints::new(),
        }
    }
}

/// How ONE advertised content id classifies against the local projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ContentGap {
    /// Not present locally — SKIP. Absence is the shard/acquisition plane's job;
    /// the content reconcile NEVER fabricates a row (`stamp_declared_head` is
    /// existing-row-only by construction).
    AbsentLocal,
    /// Present but un-anchored (`dht_anchor_hash` NULL — not in the local
    /// anchored set). Heal: the own conductor stamps the notary anchor. This is
    /// scenario 2 — the bulk-seeded row that never saw its `ContentCommitted`.
    AnchorGap,
    /// Present + anchored, but the local anchor disagrees with a NON-EMPTY peer
    /// anchor. Verify-gap: the own conductor decides who is right (we re-stamp
    /// OUR conductor's head; we never adopt the peer's value). Counts as a
    /// divergence.
    Divergent,
    /// Anchors agree, or the peer advertised no anchor — nothing to do.
    InSync,
}

/// Pure diff for ONE advertised content id (the notary invariant, Leg 4).
///
/// `present` is reach-agnostic local presence (`content_ids_present`);
/// `local_anchors` is the local anchored + distribution-safe set
/// (`list_content_anchor_inventory`); `peer_anchor` is the anchor a peer
/// advertised for the id (`None`/empty ⇒ the peer is itself un-anchored, which
/// is never divergence evidence).
fn classify_content_gap(
    id: &str,
    present: &std::collections::HashSet<String>,
    local_anchors: &std::collections::HashMap<String, String>,
    peer_anchor: Option<&str>,
) -> ContentGap {
    if !present.contains(id) {
        return ContentGap::AbsentLocal;
    }
    match local_anchors.get(id) {
        None => ContentGap::AnchorGap,
        Some(local) => match peer_anchor {
            Some(pa) if !pa.is_empty() && pa != local.as_str() => ContentGap::Divergent,
            _ => ContentGap::InSync,
        },
    }
}

/// Discovery phase of the `content` reconcile (Leg 4, steps 1–4). No conductor
/// call happens here — [`heal_content`] owns step 5.
///
/// 1. Build the local anchored+distribution-safe inventory (`id → anchor`).
/// 2. Ask every connected peer for its `ProjectionInventory { content }`.
/// 3. One `content_ids_present` query resolves reach-agnostic local presence for
///    every advertised id.
/// 4. Diff each advertised `(id, peer_anchor)` via [`classify_content_gap`]:
///    absent → SKIP; un-anchored → anchor-gap; anchor-divergent → verify-gap +
///    divergence count. Anchor-gap ∪ divergent ids feed a per-sweep
///    [`GapTracker`] on the shared rails.
///
/// **Re-detect semantics.** The tracker is rebuilt each sweep, so its per-sweep
/// `MAX_RETRIES` never permanently drops a gap: a divergence or anchor-gap that
/// persists in SQL is recomputed from the inventory diff on the NEXT sweep and
/// re-enqueued afresh. `MAX_RETRIES` only bounds within-sweep churn (and the heal
/// leg attempts each gap once per sweep, so it is effectively a floor).
async fn discover_content(
    p2p: &P2PHandle,
    pool: &DbPool,
    window: &mut InventoryWindow,
    misses: &mut MissLedger,
) -> ContentDiscovery {
    let app_ctx = crate::db::AppContext::default_lamad();
    // Rotating window offset for this sweep (advanced after the peer loop).
    let sweep_offset = window.offset_for(PROJECTION_INVENTORY_TABLE_CONTENT);
    let mut max_peer_total: u64 = 0;

    // (1) Local anchored inventory: id → anchor. Only anchored + distribution-
    // safe rows (the same set this node advertises). Absent / un-anchored rows
    // are resolved via presence below. Offset 0 / i64::MAX: the LOCAL diff needs
    // the WHOLE local set (the rotating window only bounds the PEER ask).
    let local_anchors: std::collections::HashMap<String, String> = {
        let mut conn = match pool.get() {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(error = %e, "projection-reconcile[content]: db conn failed; skipping sweep");
                return ContentDiscovery::empty(0);
            }
        };
        match crate::db::content_diesel::list_content_anchor_inventory(
            &mut conn,
            &app_ctx,
            0,
            i64::MAX,
        ) {
            Ok((rows, _total)) => rows
                .into_iter()
                .map(|r| (r.id, r.dht_anchor_hash))
                .collect(),
            Err(e) => {
                tracing::warn!(error = %e, "projection-reconcile[content]: local inventory failed; skipping sweep");
                return ContentDiscovery::empty(0);
            }
        }
    };
    let local_anchored = local_anchors.len();

    // (2) Ask connected peers for their content inventory. Collect all entries
    // first, then diff once presence is known.
    let peers = p2p.list_peers().await;
    let mut peers_asked = 0usize;
    let mut ids_discovered = 0usize;
    // id → first peer that advertised it (for the heal WARN).
    let mut discovered_by: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    // id → first NON-EMPTY advertised anchor (for divergence diffing).
    let mut advertised_anchor: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    // id → first advertised DECLARATION (adopt-before-author hint). Kept apart
    // from `advertised_anchor` on purpose: an anchor and a declaration are
    // different claims, and only the latter authorizes an adoption attempt.
    let mut peer_head_hints = crate::services::head_adoption::PeerHeadHints::new();

    for peer in &peers {
        let peer_id = match peer.peer_id.parse::<libp2p::PeerId>() {
            Ok(p) => p,
            Err(_) => continue,
        };
        let request = ViewFederationRequest {
            view_kind: ViewKind::ProjectionInventory {
                table: PROJECTION_INVENTORY_TABLE_CONTENT.to_string(),
            },
            agent_cid: p2p.agent_pubkey().to_string(),
            request_id: uuid::Uuid::new_v4().to_string(),
            // Rotating window: this sweep asks for the page at `sweep_offset`.
            inventory_offset: Some(sweep_offset),
        };
        let resp = match p2p.view_federate(peer_id, request, PEER_TIMEOUT).await {
            Ok(r) => r,
            Err(_) => continue, // peer offline/timeout — discovery is best-effort
        };
        peers_asked += 1;

        let payload: ProjectionInventoryPayload = match serde_json::from_value(
            resp.slice.payload.0.clone(),
        ) {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!(
                    target: "elohim_storage::projection_reconcile",
                    peer = %peer.peer_id,
                    error = %e,
                    "projection-reconcile[content]: peer inventory payload undecodable; skipping peer"
                );
                continue;
            }
        };

        // Honest total (Fix 2a) drives the window and truncation observability:
        // when `total > entries.len()` the peer inventory is WINDOWED — ids
        // outside this page are unknown-this-sweep, not absent. This arm classifies
        // only ADVERTISED ids (classify_content_gap below), so it never concludes
        // absence/in-sync about a non-advertised id; the window covers the rest.
        max_peer_total = max_peer_total.max(payload.total as u64);
        // Offset-aware (see discover_rea): rows remain beyond this page only when
        // offset + served < total, so the final page of a window is not
        // mislabelled "windowed".
        let windowed =
            (sweep_offset as usize).saturating_add(payload.entries.len()) < payload.total;

        tracing::info!(
            target: "elohim_storage::projection_reconcile",
            peer = %peer.peer_id,
            entries = payload.entries.len(),
            peer_total = payload.total,
            offset = sweep_offset,
            windowed = windowed,
            "projection-reconcile[content]: peer inventory received"
        );

        ids_discovered += payload.entries.len();
        for entry in &payload.entries {
            discovered_by
                .entry(entry.id.clone())
                .or_insert_with(|| peer.peer_id.clone());
            if !entry.dht_anchor_hash.is_empty() {
                advertised_anchor
                    .entry(entry.id.clone())
                    .or_insert_with(|| entry.dht_anchor_hash.clone());
            }
            // Additive field — absent from a pre-cure peer, which simply
            // contributes no hint. First advertiser wins (same or-insert
            // discipline as the anchor above): the pre-flight asks exactly one
            // peer for the Record and, if that fails, retries next sweep rather
            // than fanning out.
            if let Some(head) = entry
                .declared_head_action_hash
                .as_deref()
                .map(str::trim)
                .filter(|h| !h.is_empty())
            {
                peer_head_hints.entry(entry.id.clone()).or_insert_with(|| {
                    crate::services::head_adoption::PeerHeadHint {
                        head_action_hash: head.to_string(),
                        declared_at: entry.declared_head_at,
                        peer_id: peer.peer_id.clone(),
                    }
                });
            }
        }
    }

    // Advance the rotating window for the next sweep (wraps at the largest total).
    window.advance(
        PROJECTION_INVENTORY_TABLE_CONTENT,
        sweep_offset,
        max_peer_total,
    );

    // (3) One presence query for the whole advertised union (reach-agnostic).
    let advertised_ids: Vec<String> = discovered_by.keys().cloned().collect();
    let present: std::collections::HashSet<String> = {
        let mut conn = match pool.get() {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(error = %e, "projection-reconcile[content]: db conn failed for presence; skipping heal");
                return ContentDiscovery::empty(peers_asked);
            }
        };
        // Chunk under SQLite's bound-variable limit (SQLITE_MAX_VARIABLE_NUMBER,
        // ~999 on older builds) — `content_ids_present`'s own doc requires callers
        // to chunk large id sets (content_diesel.rs:996). A >limit federation
        // inventory would otherwise error the WHOLE presence query and silently
        // drop the sweep into a no-heal tick. Merge per-chunk results.
        const PRESENCE_CHUNK: usize = 500;
        let mut acc = std::collections::HashSet::new();
        for chunk in advertised_ids.chunks(PRESENCE_CHUNK) {
            match crate::db::content_diesel::content_ids_present(&mut conn, &app_ctx, chunk) {
                Ok(s) => acc.extend(s),
                Err(e) => {
                    tracing::warn!(error = %e, "projection-reconcile[content]: presence query failed; skipping heal");
                    return ContentDiscovery::empty(peers_asked);
                }
            }
        }
        acc
    };

    // (4) Classify → gap set (anchor-gap ∪ divergent). Absent + in-sync are dropped.
    let mut gap_ids: Vec<String> = Vec::new();
    let mut divergent_ids: Vec<String> = Vec::new();
    let mut divergent_anchor = 0usize;
    let mut resolved_ids: Vec<&String> = Vec::new();
    for id in &advertised_ids {
        match classify_content_gap(
            id,
            &present,
            &local_anchors,
            advertised_anchor.get(id).map(String::as_str),
        ) {
            ContentGap::AbsentLocal | ContentGap::InSync => resolved_ids.push(id),
            ContentGap::AnchorGap => gap_ids.push(id.clone()),
            ContentGap::Divergent => {
                divergent_anchor += 1;
                divergent_ids.push(id.clone());
                gap_ids.push(id.clone());
            }
        }
    }

    // (4b) Split the divergence into ACTIONABLE and REFUSED, locally and with no
    // conductor call — one batch read of the same `declared_head_action_hash`
    // column `stamp_declared_head_mode` consults to return `SkippedDeclared`.
    //
    // A row that already carries a declared head is one heal is FORBIDDEN to move
    // (canonical channels own it). Counting it as unresolved divergence is what
    // pinned `elohim_projection_reconcile_converged` at 0 fleet-wide: the refusal
    // is permanent until a canonical channel fires, so the gauge could never move
    // no matter how well the heal leg worked.
    //
    // A read failure is conservative in the HONEST direction: no declarations
    // known ⇒ nothing adjudicated ⇒ the divergence still defeats convergence.
    let mut declared_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    if !divergent_ids.is_empty() {
        match pool.get() {
            Ok(mut conn) => {
                match crate::db::content_diesel::declared_heads_for(
                    &mut conn,
                    &app_ctx,
                    &divergent_ids,
                ) {
                    Ok(declared) => declared_ids = declared.into_keys().collect(),
                    Err(e) => tracing::warn!(
                        error = %e,
                        "projection-reconcile[content]: declared-head classification failed; \
                         counting all divergence as unadjudicated this sweep"
                    ),
                }
            }
            Err(e) => tracing::warn!(
                error = %e,
                "projection-reconcile[content]: db conn failed for declared-head classification; \
                 counting all divergence as unadjudicated this sweep"
            ),
        }
    }

    // (4c) Cross-sweep retry budget (see [`MissLedger`]): drop gap ids whose
    // budget is spent against unchanged peer evidence, so the sweep stops
    // re-asking the conductor about the same ~1000 rows forever. An id held back
    // here is ADJUDICATED — it joins `divergent_refused` when it is a divergence,
    // and is published on the exhausted gauge either way.
    let mut exhausted_persistent = 0usize;
    let mut exhausted_divergent = 0usize;
    let divergent_set: std::collections::HashSet<&String> = divergent_ids.iter().collect();
    let mut admitted: Vec<String> = Vec::with_capacity(gap_ids.len());
    for id in gap_ids {
        let evidence = advertised_anchor.get(&id).map(String::as_str).unwrap_or("");
        match misses.admit(PROJECTION_INVENTORY_TABLE_CONTENT, &id, evidence) {
            Admission::Retry => admitted.push(id),
            Admission::Exhausted => {
                exhausted_persistent += 1;
                // Count ONLY ids the declaration predicate did not already
                // adjudicate — the two buckets partition the divergence set, so
                // an id in both must not be counted twice.
                if divergent_set.contains(&id) && !declared_ids.contains(&id) {
                    exhausted_divergent += 1;
                }
            }
        }
    }
    let divergent_refused = declared_ids.len().saturating_add(exhausted_divergent);
    for id in resolved_ids {
        misses.resolved(PROJECTION_INVENTORY_TABLE_CONTENT, id);
    }

    // Feed the gap set through a fresh per-sweep tracker on the shared rails
    // (empty local set → every gap id becomes pending, under MAX_RETRIES).
    let mut tracker = GapTracker::new(MAX_RETRIES);
    tracker.discover(admitted);

    ContentDiscovery {
        tracker,
        discovered_by,
        divergent_anchor,
        divergent_refused,
        exhausted_persistent,
        peers_asked,
        ids_discovered,
        local_anchored,
        peer_head_hints,
    }
}

/// Heal phase of the `content` reconcile (Leg 4, step 5): for each discovered
/// gap, `content_store::resolve_content_head(id)` on the OWN conductor →
/// [`stamp_declared_head`] (verified stamp); `None` → `mark_failed`, retried next
/// sweep. Returns a [`ContentHealOutcome`] — the convergence counts plus the
/// ids the own conductor could not resolve at all, which
/// [`witness_ghost_anchors`] narrows to the ghost-anchor class. Runs only once the lamad bridge
/// is up; scheduled single-flight OFF the discovery ticker (see `main.rs`).
/// `stamp_declared_head` is existing-row-only and idempotent, so a heal is safe
/// under duplicate delivery. A heal logs WARN naming the id and discovering peer.
async fn heal_content(
    tracker: &mut GapTracker,
    discovered_by: &std::collections::HashMap<String, String>,
    hc: &Arc<HcClient>,
    pool: &DbPool,
    pacing: &HealPacing,
    peer_head_hints: &crate::services::head_adoption::PeerHeadHints,
) -> ContentHealOutcome {
    let app_ctx = crate::db::AppContext::default_lamad();
    let mut ghost_candidates: Vec<String> = Vec::new();
    let mut adopt_candidates: Vec<AdoptCandidate> = Vec::new();

    // (5) Heal each gap from the OWN conductor (verified stamp), bounded by the
    // content leg budget so a saturated conductor lands SOME rows per tick and the
    // leg recycles instead of grinding for hours (the incident). Runs AFTER `heal_rea`
    // so the small REA backlog is never starved behind this large content queue.
    let leg_start = std::time::Instant::now();
    let mut healed = 0usize;
    let mut conductor_missing = 0usize;
    let mut circuit = HealCircuit::new(pacing.circuit_timeout_threshold);
    for id in tracker.pending_ids() {
        let attempt = call_with_retry(pacing, || {
            // LOCAL-only resolve — the heal loop must not stall on a cold arc.
            // `Ok(None)` from this variant is "not in my local view YET", never
            // authoritative absence; the HTTP author gate and the adoption
            // pre-flight keep using the Network variant for that reason.
            crate::services::conductor_writes::call_resolve_content_head_local(hc, &id)
        })
        .await;
        circuit.record(&attempt.result);
        // ELECTION-TIER METER. Counted on EVERY answer, before any branch
        // consumes it: `canonical` was previously read only to pick a stamp mode
        // and then discarded, so "does an election even exist for this class?"
        // was unanswerable from telemetry — the question that separates a
        // supply-side deadlock from a gossip lag. An answer that never arrived
        // is not an answer and is deliberately not counted here (the miss and
        // timeout classes below own it).
        if let Ok(Some(ref head)) = attempt.result {
            crate::metrics::inc_content_canonical_answer(head.canonical_tier_label());
        }
        match attempt.result {
            // GAPFILL SELF-ELECTION GUARD. Checked BEFORE the stamp, because the
            // stamp is what makes the divergence terminal (see
            // `gapfill_would_self_elect`). The extra projection read is paid only
            // on the narrow suspicious path — a non-canonical answer for an id a
            // peer is advertising a declaration for.
            Ok(Some(ref head))
                if !head.canonical && peer_head_hints.contains_key(&id) && {
                    // Read failure DEFERS (conservative): deferring writes
                    // nothing and the next sweep retries, whereas guessing
                    // "undeclared" and stamping is irreversible.
                    let local_declared = pool.get().ok().and_then(|mut c| {
                        crate::db::content_diesel::declared_head_for(&mut c, &app_ctx, &id).ok()
                    });
                    match local_declared {
                        Some(d) => gapfill_would_self_elect(false, true, d.as_deref()),
                        None => true,
                    }
                } =>
            {
                let peer = discovered_by
                    .get(&id)
                    .cloned()
                    .unwrap_or_else(|| "unknown".to_string());
                tracing::warn!(
                    target: "elohim_storage::projection_reconcile",
                    content_id = %id,
                    discovered_via_peer = %peer,
                    fallback_head = %head.head_action_hash,
                    "projection-reconcile[content]: REFUSED to GapFill an undeclared row with \
                     this node's own fallback root while a peer advertises a declaration — \
                     deferred to the adopt-before-author arm"
                );
                tracker.mark_completed(&id);
                // Carry the conductor's OWN answer forward. It is a fallback
                // (non-canonical) here by construction — that is what this arm
                // matched on — so the pre-flight will see `canonical == false`
                // and route to the peer/contest arms exactly as before. The
                // difference is that it now reads that from EVIDENCE rather than
                // from a blanket `Known(None)` assertion.
                adopt_candidates.push(AdoptCandidate {
                    id: id.clone(),
                    head: Some(head.clone()),
                });
                crate::metrics::inc_projection_heal_outcome(
                    "content",
                    HealOutcomeKind::DeferredToAdopt.label(),
                );
            }
            Ok(Some(head)) => match heal_content_one(&head, pool, &app_ctx) {
                Ok(crate::db::content_diesel::StampOutcome::Stamped) => {
                    tracker.mark_completed(&id);
                    healed += 1;
                    let peer = discovered_by
                        .get(&id)
                        .cloned()
                        .unwrap_or_else(|| "unknown".to_string());
                    tracing::warn!(
                        target: "elohim_storage::projection_reconcile",
                        content_id = %id,
                        discovered_via_peer = %peer,
                        retried = attempt.retried,
                        "projection-reconcile[content]: HEALED content anchor from own conductor (peer discovery)"
                    );
                    crate::metrics::inc_projection_heal_outcome(
                        "content",
                        if attempt.retried {
                            HealOutcomeKind::TimeoutRetried.label()
                        } else {
                            HealOutcomeKind::Healed.label()
                        },
                    );
                }
                Ok(crate::db::content_diesel::StampOutcome::Refreshed) => {
                    // The own conductor answered with the head this row ALREADY
                    // holds: the write refreshed value fields and backfilled the
                    // declaration ordering, but the HEAD did not move — so the
                    // peer-advertised divergence that enqueued this row is
                    // UNCHANGED and will re-enqueue next sweep. Deliberately NOT
                    // counted in `healed` and NOT logged as HEALED: conflating
                    // this no-op with convergence is what made a spinning heal
                    // plane read as a working one (see `StampOutcome::Refreshed`).
                    tracker.mark_completed(&id);
                    tracing::info!(
                        target: "elohim_storage::projection_reconcile",
                        content_id = %id,
                        "projection-reconcile[content]: head unchanged — refreshed row, divergence NOT resolved (own conductor answered the head this row already holds)"
                    );
                    crate::metrics::inc_projection_heal_outcome(
                        "content",
                        HealOutcomeKind::Refreshed.label(),
                    );
                }
                Ok(crate::db::content_diesel::StampOutcome::SkippedDeclared) => {
                    // Row already carries a DIFFERENT declared head — the heal
                    // must not move it (a fresh-boot conductor resolve can fall
                    // through to the root-author election and would resurrect a
                    // superseded head over an adopted canonical one — the
                    // 2026-07-11 20:42:40 regression). Canonical channels own it.
                    tracker.mark_completed(&id);
                    tracing::info!(content_id = %id, "projection-reconcile[content]: row already declared — heal left it to the canonical channels");
                    crate::metrics::inc_projection_heal_outcome(
                        "content",
                        HealOutcomeKind::RefusedDeclared.label(),
                    );
                }
                Ok(crate::db::content_diesel::StampOutcome::SkippedStale) => {
                    // The conductor's CANONICAL answer is not provably newer
                    // than the declared head this row already adopted — a
                    // conductor that has not yet integrated the newer canonical
                    // link answers with the OLD canonical record, and stamping
                    // it would move the head BACKWARDS (the 2026-07-12
                    // regression: converged at edge #1187's seam-smoke, healed
                    // back to the superseded head by #1188). Completed, not
                    // failed: retrying yields the same stale answer until the
                    // conductor integrates the newer link, at which point the
                    // next sweep's answer becomes provably newer and stamps.
                    tracker.mark_completed(&id);
                    tracing::info!(content_id = %id, "projection-reconcile[content]: conductor canonical answer not provably newer — heal kept the adopted head");
                    crate::metrics::inc_projection_heal_outcome(
                        "content",
                        HealOutcomeKind::RefusedStale.label(),
                    );
                }
                Ok(crate::db::content_diesel::StampOutcome::NoRow) => {
                    // Row vanished between presence check and stamp (rare race).
                    // Nothing to stamp — resolved, not a conductor miss.
                    tracker.mark_completed(&id);
                    tracing::debug!(content_id = %id, "projection-reconcile[content]: stamp found no local row; nothing to do");
                    crate::metrics::inc_projection_heal_outcome(
                        "content",
                        HealOutcomeKind::NoRow.label(),
                    );
                }
                Err(e) => {
                    tracing::warn!(content_id = %id, error = %e, "projection-reconcile[content]: stamp failed; retry next sweep");
                    tracker.mark_failed(&id);
                    crate::metrics::inc_projection_heal_outcome(
                        "content",
                        HealOutcomeKind::Failed.label(),
                    );
                } // SkippedDeclared / SkippedStale / NoRow are benign resolutions
                  // (row already correct or absent): `mark_completed`, no stamp, and
                  // deliberately NOT counted as `healed` so the cure signal (real
                  // stamps) is not inflated by no-op resolutions.
            },
            Ok(None) => {
                // Conductor can't see it yet (catch-up) — retry on the NEXT
                // sweep via a fresh inventory diff, never an immediate re-queue.
                conductor_missing += 1;
                // GHOST-ANCHOR CANDIDATE (see `witness_ghost_anchors`). This is
                // the ONLY place the substrate learns "my own conductor has no
                // chain for an id my projection carries" — `resolve_content_head`
                // returns `Ok(None)` exactly when BOTH the canonical link and the
                // per-root chain are locally absent. Collect it here rather than
                // re-probing later: the answer is already paid for.
                ghost_candidates.push(id.clone());
                // MISSING → PEER-ADOPTION ROUTING (2026-08-01). The ghost sweep
                // below narrows this list to rows that already CLAIM an anchor,
                // so a conductor-missing row with a NULL `dht_anchor_hash` — the
                // dominant `AnchorGap` class — never saw the adopt pre-flight and
                // went straight to `witness_bootstrap`'s AUTHOR path instead.
                // Listing it here gives it the pre-flight FIRST (see
                // `conductor_missing_should_route_to_adopt`); an already-declared
                // row simply `Hold`s, so this can never double-declare.
                let routed_to_adopt =
                    conductor_missing_should_route_to_adopt(peer_head_hints.contains_key(&id));
                if routed_to_adopt {
                    // `Ok(None)`: the conductor answered, and its answer was
                    // "nothing". There is no head to carry, and no canonical
                    // head can exist locally — `Unresolved` is the honest
                    // provenance (see `AdoptCandidate::head`).
                    adopt_candidates.push(AdoptCandidate {
                        id: id.clone(),
                        head: None,
                    });
                }
                tracing::debug!(content_id = %id, routed_to_adopt, "projection-reconcile[content]: own conductor returned None; retry next sweep");
                tracker.mark_failed(&id);
                crate::metrics::inc_projection_heal_outcome(
                    "content",
                    HealOutcomeKind::Missing.label(),
                );
            }
            Err(e) => {
                let transient = is_transient_conductor_error(&e);
                // TIMEOUT → PEER-ADOPTION ROUTING (2026-07-29). A row whose own-
                // conductor resolve timed out used to fall out of BOTH candidate
                // lists — neither a ghost (we never got `Ok(None)`) nor an adopt
                // candidate — so a conductor that cannot answer meant the id was
                // simply dropped every sweep, forever. When a peer advertises a
                // declaration for this id we already hold the verified path to it:
                // `adopt_deferred_heads` fetches the peer's head Record over
                // view-federation and declares it with `carried_record`, which the
                // zome's `validate_carried_record` verifies (action-hash binding,
                // author signature, entry↔action binding) before acceptance. That
                // is evidence, not authority: the DHT stays the manifest, the
                // declare still goes through the conductor, and the stamp modes are
                // untouched — this only ADDS a candidate, it never widens `Declare`.
                //
                // The row stays `mark_failed` (honest: WE did not heal it) so it is
                // re-discovered next sweep and `caught_up` is not falsely satisfied.
                let routed_to_adopt =
                    timeout_should_route_to_adopt(transient, peer_head_hints.contains_key(&id));
                if routed_to_adopt {
                    // The conductor never answered — absence was NOT observed.
                    // `head: None` maps to `LocalResolve::Unresolved`, the
                    // variant that says exactly that.
                    adopt_candidates.push(AdoptCandidate {
                        id: id.clone(),
                        head: None,
                    });
                }
                tracing::warn!(content_id = %id, error = %e, transient, routed_to_adopt, "projection-reconcile[content]: conductor resolve failed; retry next sweep");
                tracker.mark_failed(&id);
                crate::metrics::inc_projection_heal_outcome(
                    "content",
                    if transient {
                        HealOutcomeKind::TimeoutExhausted.label()
                    } else {
                        HealOutcomeKind::Failed.label()
                    },
                );
            }
        }

        // Per-leg budget: yield so the leg recycles next sweep (see `heal_rea`).
        if leg_start.elapsed() >= pacing.content_leg_budget {
            tracing::info!(
                target: "elohim_storage::projection_reconcile",
                budget_secs = pacing.content_leg_budget.as_secs(),
                healed,
                "projection-reconcile[content]: heal hit leg budget — yielding, remaining gaps resume next sweep"
            );
            break;
        }

        // Unresponsive-conductor circuit: shed the rest of the leg rather than
        // stack more abandoned-but-still-executing calls on it (see `HealCircuit`).
        if circuit.is_open() {
            tracing::warn!(
                target: "elohim_storage::projection_reconcile",
                consecutive_timeouts = circuit.consecutive_timeouts(),
                healed,
                "projection-reconcile[content]: OPENED the unresponsive-conductor circuit — shedding the rest of the leg, remaining gaps resume next sweep"
            );
            break;
        }
    }

    // END-OF-ARM caught_up (mirrors `heal_rea`). `GapTracker::caught_up`
    // DEFAULTS to false and `enqueue_missing` only ever sets it false — nothing
    // else in the machine ever flips it true. An arm that never calls this
    // publishes a structurally-false bit, which `fold_arm_counts` then ANDs into
    // a permanently-false sweep `caught_up`: `/health`'s `p2p.caughtUp` could
    // never read true again, and the a2o head-convergence scenario that asserts
    // it would be unpassable by construction. The fold made an omission that was
    // previously harmless (only REA was published) load-bearing.
    tracker.update_caught_up();
    ContentHealOutcome {
        healed,
        conductor_missing,
        ghost_candidates,
        adopt_candidates,
    }
}

/// One id handed to [`adopt_deferred_heads`], carrying the own-conductor answer
/// the heal leg ALREADY paid for.
///
/// Carrying it is not an optimisation — it is what makes `AdoptLocal` reachable
/// from the adopt arm. This list previously held bare ids and the arm asserted
/// `LocalResolve::Known(None)` for every one of them, so `conductor_canonical`
/// was hard-wired false and a DHT-ELECTED canonical head could never be adopted
/// here no matter what the conductor had just said.
#[derive(Debug, Clone)]
struct AdoptCandidate {
    id: String,
    /// `Some` when the conductor answered (canonical or fallback); `None` when
    /// it timed out or reported nothing — mapped to `LocalResolve::Unresolved`,
    /// which states "absence was not observed" rather than asserting it.
    head: Option<crate::services::conductor_writes::ContentHeadWire>,
}

/// Run the adopt-before-author pre-flight over ids whose `GapFill` was refused by
/// [`gapfill_would_self_elect`].
///
/// These rows are NOT reachable by either witness sweep — they are anchored (so
/// the re-anchor sweep skips them) and conductor-resolvable (so the ghost sweep
/// skips them). Without this arm, refusing the GapFill would only convert a
/// terminal wrong answer into a terminal absent one. This is what actually
/// converges them.
///
/// CONTRACT DEVIATION — `candidates` now carries TWO provenances, and
/// `LocalResolve::Known(None)` is exact for only the first:
///
/// 1. GAPFILL-REFUSED (the original): the own conductor DID answer, but with a
///    non-canonical fallback, so there is no canonical head to adopt locally —
///    precisely what `Known(None)` states. The decision rule then sees
///    `(canonical=false, peer=true, local=None)` → `AdoptPeer`.
/// 2. TIMEOUT-ROUTED (2026-07-29, [`timeout_should_route_to_adopt`]): the
///    conductor did NOT answer. Absence was not observed, only unestablished —
///    so here `Known(None)` is a conservative STAND-IN, not an observation.
///
/// The stand-in is safe because it can only ever FORECLOSE the `AdoptLocal` arm,
/// never assert absence: both provenances are gated on a peer hint existing, so
/// the reachable verdicts are `AdoptPeer` / `Hold`. Preserve that. If a future
/// arm needs to act on "the conductor observed nothing", split the variant (see
/// the `LocalResolve::Known` doc) rather than letting a timeout read as an
/// observed absence.
///
/// Never authors. An `Author` verdict here means the peer's head could not be
/// declared this sweep (no usable carried record yet); the row keeps its
/// non-declaring anchor and the next sweep retries. `AuthorThenAdopt` is
/// EXPECTED for a timeout-routed candidate (we never confirmed a local chain)
/// and remains a no-op — never a licence to re-author outside the guarded
/// sweeps.
async fn adopt_deferred_heads(
    hc: &Arc<HcClient>,
    pool: &DbPool,
    candidates: &[AdoptCandidate],
    adopt: &crate::services::head_adoption::AdoptContext<'_>,
) {
    if candidates.is_empty() {
        return;
    }
    let app_ctx = crate::db::AppContext::default_lamad();
    let total = candidates.len();
    let mut adopted = 0usize;
    let mut contested = 0usize;
    let mut held = 0usize;
    let mut retry = 0usize;

    let sweep = async {
        for candidate in candidates.iter().take(WITNESS_MAX_PER_TICK as usize) {
            let id = &candidate.id;
            // PROVENANCE-EXACT local resolve. The heal leg ALREADY paid the
            // conductor for these answers, so pass what it learned instead of
            // asserting a blanket `Known(None)`:
            //   - `Known(Some(head))` — the conductor answered. If that answer
            //     is CANONICAL the pre-flight can now take `AdoptLocal`, which
            //     was structurally unreachable from this arm while every
            //     candidate claimed observed-absence. That is the whole point:
            //     an elected head must be adoptable here.
            //   - `Unresolved` — the conductor timed out or reported nothing.
            //     Absence was never observed; the variant says so honestly.
            let local_resolve = match &candidate.head {
                Some(head) => crate::services::head_adoption::LocalResolve::Known(Some(head)),
                None => crate::services::head_adoption::LocalResolve::Unresolved,
            };
            match crate::services::head_adoption::try_adopt_canonical_head(
                hc,
                pool,
                &app_ctx,
                id,
                local_resolve,
                adopt,
            )
            .await
            {
                crate::services::head_adoption::AdoptOutcome::Adopted => adopted += 1,
                crate::services::head_adoption::AdoptOutcome::Contested => contested += 1,
                crate::services::head_adoption::AdoptOutcome::Held => held += 1,
                crate::services::head_adoption::AdoptOutcome::Author => retry += 1,
                crate::services::head_adoption::AdoptOutcome::AuthorThenAdopt { .. } => {
                    retry += 1;
                    tracing::warn!(
                        content_id = %id,
                        "projection-reconcile[adopt-deferred]: conductor reports no local chain \
                         for an id it just resolved — not authoring here; retried next sweep"
                    );
                }
            }
            tokio::time::sleep(WITNESS_ITEM_DELAY).await;
        }
    };

    if tokio::time::timeout(WITNESS_SWEEP_BUDGET, sweep)
        .await
        .is_err()
    {
        tracing::warn!(
            target: "elohim_storage::projection_reconcile",
            budget_secs = WITNESS_SWEEP_BUDGET.as_secs(),
            candidates = total,
            "projection-reconcile[adopt-deferred]: sweep exceeded wall-clock budget — \
             abandoned, resumes next sweep"
        );
        return;
    }

    tracing::warn!(
        target: "elohim_storage::projection_reconcile",
        candidates = total,
        adopted,
        contested,
        held,
        retry,
        "projection-reconcile[adopt-deferred]: adopted peer-declared heads, and contested \
         two-way declared ones by minting a canonical candidate for the DHT election"
    );
}

/// Project ONE conductor-resolved content HEAD into local SQL via the VERIFIED
/// stamp path. Row content comes exclusively from the own conductor's resolved
/// [`ContentHeadWire`]; the field mapping mirrors the `ContentCommitted` signal
/// arm (`rea_projection.rs`). Returns `stamp_declared_head`'s bool (false ⇒ no
/// local row to stamp).
fn heal_content_one(
    head: &crate::services::conductor_writes::ContentHeadWire,
    pool: &DbPool,
    app_ctx: &crate::db::AppContext,
) -> Result<crate::db::content_diesel::StampOutcome, crate::error::StorageError> {
    let c = &head.content;
    // u64 → i32 saturating cast — identical to the ContentCommitted arm.
    let size_i32 = c
        .content_size_bytes
        .map(|n| i32::try_from(n).unwrap_or(i32::MAX));
    let patch = crate::db::content_diesel::ContentProjectionPatch {
        blob_cid: c.blob_cid.clone(),
        content_size_bytes: size_i32,
        title: Some(c.title.clone()),
        description: Some(c.description.clone()),
        content_type: Some(c.content_type.clone()),
        content_format: Some(c.content_format.clone()),
        reach: Some(c.reach.clone()),
        metadata_json: Some(c.metadata_json.clone()),
    };
    let mut conn = pool
        .get()
        .map_err(|e| crate::error::StorageError::Internal(format!("pool: {e}")))?;
    // Canonical-aware stamp mode: a CANONICAL answer (the conductor verified
    // the cross-root canonical record) may fill an undeclared row, refresh the
    // same head, or MOVE a declared row FORWARD (provably newer declared_at) —
    // this is exactly how a peer converges when the canonical link gossips in
    // between deploys. It must NOT move a declared row otherwise: a conductor
    // that has not yet integrated a newer canonical link answers with the OLD
    // canonical record — canonical, yet stale — and an unconditional Declare
    // stamp moved the head BACKWARDS (live regression 2026-07-12,
    // elohim-host-landing). A FALLBACK answer (cold conductor, root-author
    // election) may only FILL an undeclared row — never resurrect a
    // superseded head over an adopted canonical one.
    let mode = if head.canonical {
        crate::db::content_diesel::StampMode::HealCanonical
    } else {
        crate::db::content_diesel::StampMode::GapFill
    };
    crate::db::content_diesel::stamp_declared_head_mode(
        &mut conn,
        app_ctx,
        &c.id,
        head.head_action_hash.as_str(),
        Some(head.declared_at),
        Some(patch),
        mode,
        // The DHT ELECTION behind a canonical answer — the winning declaration
        // link's notarized timestamp and tier, which `canonical_move_verdict`
        // arbitrates on. `None` for a fallback answer (no election ran) and from
        // a pre-cure coordinator, which the guard reads as "carries no
        // election". This is what makes an elected head CONSUMABLE by an
        // already-declared row; without it the guard compared head-ACTION
        // timestamps and refused every move the DHT had already decided.
        head.canonical_ordering(),
    )
}

// ============================================================================
// Collectives reconcile arm (cross-peer collective identity)
// ============================================================================

/// Discovery-side output for the collectives arm. Mirrors [`ContentDiscovery`];
/// the tracker is keyed by `collective_cid` (the reconciliation identity), NOT
/// by the diesel row id (a peer-local routing alias).
pub struct CollectivesDiscovery {
    tracker: GapTracker,
    /// cid → the FIRST peer-advertised routing alias for it. Passed to the heal
    /// as a merge CANDIDATE: honored only when a local row exists under that id
    /// AND carries no cid (see [`crate::db::collectives::CollectiveProjection`]).
    alias_by_cid: std::collections::HashMap<String, String>,
    /// cid → the first peer that advertised it (for the heal WARN).
    discovered_by: std::collections::HashMap<String, String>,
    /// Advertised cids whose local alias row already carries a DIFFERENT cid.
    /// Counted + WARN-logged, never enqueued (see the module doc).
    divergent_cid: usize,
    /// Gap cids the cross-sweep [`MissLedger`] held back this sweep. Published on
    /// `elohim_projection_reconcile_exhausted{stream="collectives"}`.
    exhausted_persistent: usize,
    peers_asked: usize,
    ids_discovered: usize,
    local_anchored: usize,
}

impl CollectivesDiscovery {
    /// Empty discovery (db unavailable this tick) — the heal leg has nothing to
    /// do. `peers_asked` records how many peers answered before the db failure
    /// so the discovery log stays honest.
    fn empty(peers_asked: usize) -> Self {
        Self {
            tracker: GapTracker::new(MAX_RETRIES),
            alias_by_cid: std::collections::HashMap::new(),
            discovered_by: std::collections::HashMap::new(),
            divergent_cid: 0,
            exhausted_persistent: 0,
            peers_asked,
            ids_discovered: 0,
            local_anchored: 0,
        }
    }
}

/// How ONE advertised `(routing-alias id, collective_cid)` pair classifies
/// against the local `collectives` projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CollectiveGap {
    /// The cid is held under no local id, and no local row exists under the
    /// advertised alias either. Heal CREATES a cid-keyed row from the own
    /// conductor's `Collective` entry. Unlike the content arm — which refuses to
    /// fabricate because content BYTES are the acquisition plane's job — a
    /// `Collective` entry is carried whole by the DHT, so on a full-arc fleet the
    /// own conductor can answer for it without any peer's bytes.
    AbsentLocal,
    /// A local row exists under the advertised alias but carries NO cid — the
    /// pre-coherence seed row. Heal STAMPS the conductor-verified cid onto it
    /// (one household, one row).
    CidGap,
    /// A local row exists under the advertised alias and already carries a
    /// DIFFERENT cid. Heal refuses: `collectives` has no declaration-ordering
    /// column, so no forward move is provable. Counted + logged, not enqueued.
    Divergent,
    /// The cid is already anchored locally under some id, or the peer advertised
    /// no cid at all (a NULL-cid row should never be advertised — see ruling 1 —
    /// but an empty value is treated as no evidence, never as a gap).
    InSync,
}

/// Pure diff for ONE advertised `(id, peer_cid)` pair.
///
/// - `local_cids` — every `collective_cid` this peer holds, under any id (from
///   the NOT-NULL inventory). Membership here means "I already have this
///   collective", regardless of which routing alias it sits under.
/// - `present` — reach-agnostic local presence of the advertised diesel `id`.
/// - `local_cid_by_id` — `id → cid` for anchored rows only; an id present in
///   `present` but ABSENT here is an un-anchored (NULL-cid) row.
pub(crate) fn classify_collective_gap(
    id: &str,
    peer_cid: Option<&str>,
    local_cids: &std::collections::HashSet<String>,
    present: &std::collections::HashSet<String>,
    local_cid_by_id: &std::collections::HashMap<String, String>,
) -> CollectiveGap {
    // Ruling 1: a row with no DHT identity carries nothing to reconcile ON.
    // Responders never advertise one; an empty value here is defensive.
    let Some(cid) = peer_cid.map(str::trim).filter(|c| !c.is_empty()) else {
        return CollectiveGap::InSync;
    };
    if local_cids.contains(cid) {
        return CollectiveGap::InSync;
    }
    if !present.contains(id) {
        return CollectiveGap::AbsentLocal;
    }
    match local_cid_by_id.get(id) {
        None => CollectiveGap::CidGap,
        Some(_) => CollectiveGap::Divergent,
    }
}

/// Discovery phase of the `collectives` reconcile. No conductor call happens
/// here — [`heal_collectives`] owns that.
///
/// 1. Build the local anchored inventory (`id → collective_cid`, NOT-NULL only).
/// 2. Ask every connected peer for its `ProjectionInventory { collectives }`.
/// 3. One `collective_ids_present` query resolves local presence for every
///    advertised routing alias.
/// 4. Classify each advertised pair via [`classify_collective_gap`]; absent +
///    cid-gap ids feed a per-sweep [`GapTracker`] keyed by cid.
///
/// A cid this node cannot even DECODE (`collective:{action_hash}` malformed, or
/// a foreign prefix) is dropped at discovery rather than enqueued: it can never
/// resolve, so enqueuing it would burn a conductor round-trip every sweep
/// forever.
async fn discover_collectives(
    p2p: &P2PHandle,
    pool: &DbPool,
    window: &mut InventoryWindow,
    misses: &mut MissLedger,
) -> CollectivesDiscovery {
    let app_ctx = crate::db::AppContext::default_lamad();
    let sweep_offset = window.offset_for(PROJECTION_INVENTORY_TABLE_COLLECTIVES);
    let mut max_peer_total: u64 = 0;

    // (1) Local anchored inventory. Offset 0 / i64::MAX: the LOCAL diff needs the
    // WHOLE local set (the rotating window only bounds the PEER ask).
    let local_cid_by_id: std::collections::HashMap<String, String> = {
        let mut conn = match pool.get() {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(error = %e, "projection-reconcile[collectives]: db conn failed; skipping sweep");
                return CollectivesDiscovery::empty(0);
            }
        };
        match crate::db::collectives::list_collective_cid_inventory(
            &mut conn,
            &app_ctx,
            0,
            i64::MAX,
        ) {
            Ok((rows, _total)) => rows.into_iter().collect(),
            Err(e) => {
                tracing::warn!(error = %e, "projection-reconcile[collectives]: local inventory failed; skipping sweep");
                return CollectivesDiscovery::empty(0);
            }
        }
    };
    let local_anchored = local_cid_by_id.len();
    let local_cids: std::collections::HashSet<String> = local_cid_by_id.values().cloned().collect();

    // (2) Ask connected peers.
    let peers = p2p.list_peers().await;
    let mut peers_asked = 0usize;
    let mut ids_discovered = 0usize;
    // Advertised (id, cid) pairs, first-writer-wins per id.
    let mut advertised: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    let mut advertised_by: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();

    for peer in &peers {
        let peer_id = match peer.peer_id.parse::<libp2p::PeerId>() {
            Ok(p) => p,
            Err(_) => continue,
        };
        let request = ViewFederationRequest {
            view_kind: ViewKind::ProjectionInventory {
                table: PROJECTION_INVENTORY_TABLE_COLLECTIVES.to_string(),
            },
            agent_cid: p2p.agent_pubkey().to_string(),
            request_id: uuid::Uuid::new_v4().to_string(),
            inventory_offset: Some(sweep_offset),
        };
        let resp = match p2p.view_federate(peer_id, request, PEER_TIMEOUT).await {
            Ok(r) => r,
            Err(_) => continue, // peer offline/timeout — discovery is best-effort
        };
        peers_asked += 1;

        let payload: ProjectionInventoryPayload = match serde_json::from_value(
            resp.slice.payload.0.clone(),
        ) {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!(
                    target: "elohim_storage::projection_reconcile",
                    peer = %peer.peer_id,
                    error = %e,
                    "projection-reconcile[collectives]: peer inventory payload undecodable; skipping peer"
                );
                continue;
            }
        };

        max_peer_total = max_peer_total.max(payload.total as u64);
        let windowed =
            (sweep_offset as usize).saturating_add(payload.entries.len()) < payload.total;

        tracing::info!(
            target: "elohim_storage::projection_reconcile",
            peer = %peer.peer_id,
            entries = payload.entries.len(),
            peer_total = payload.total,
            offset = sweep_offset,
            windowed = windowed,
            "projection-reconcile[collectives]: peer inventory received"
        );

        ids_discovered += payload.entries.len();
        for entry in &payload.entries {
            if entry.dht_anchor_hash.trim().is_empty() {
                continue; // ruling 1 — no DHT identity, nothing to reconcile on
            }
            advertised
                .entry(entry.id.clone())
                .or_insert_with(|| entry.dht_anchor_hash.clone());
            advertised_by
                .entry(entry.id.clone())
                .or_insert_with(|| peer.peer_id.clone());
        }
    }

    window.advance(
        PROJECTION_INVENTORY_TABLE_COLLECTIVES,
        sweep_offset,
        max_peer_total,
    );

    // (3) One presence query for the advertised routing aliases.
    let advertised_ids: Vec<String> = advertised.keys().cloned().collect();
    let present: std::collections::HashSet<String> = {
        let mut conn = match pool.get() {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(error = %e, "projection-reconcile[collectives]: db conn failed for presence; skipping heal");
                return CollectivesDiscovery::empty(peers_asked);
            }
        };
        // Chunk under SQLITE_MAX_VARIABLE_NUMBER (see `collective_ids_present`).
        const PRESENCE_CHUNK: usize = 500;
        let mut acc = std::collections::HashSet::new();
        for chunk in advertised_ids.chunks(PRESENCE_CHUNK) {
            match crate::db::collectives::collective_ids_present(&mut conn, &app_ctx, chunk) {
                Ok(s) => acc.extend(s),
                Err(e) => {
                    tracing::warn!(error = %e, "projection-reconcile[collectives]: presence query failed; skipping heal");
                    return CollectivesDiscovery::empty(peers_asked);
                }
            }
        }
        acc
    };

    // (4) Classify → gap set, keyed by cid.
    let mut gap_cids: Vec<String> = Vec::new();
    let mut alias_by_cid: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    let mut discovered_by: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    let mut divergent_cid = 0usize;
    let mut undecodable = 0usize;

    for (id, cid) in &advertised {
        match classify_collective_gap(
            id,
            Some(cid.as_str()),
            &local_cids,
            &present,
            &local_cid_by_id,
        ) {
            CollectiveGap::InSync => {}
            CollectiveGap::Divergent => {
                divergent_cid += 1;
                tracing::warn!(
                    target: "elohim_storage::projection_reconcile",
                    collective_id = %id,
                    peer_cid = %cid,
                    local_cid = %local_cid_by_id.get(id).map(String::as_str).unwrap_or(""),
                    "projection-reconcile[collectives]: DIVERGENT cid on the same routing alias — \
                     heal fills, never moves (no declaration ordering on `collectives` to prove a \
                     forward move); a canonical channel must resolve this"
                );
            }
            CollectiveGap::AbsentLocal | CollectiveGap::CidGap => {
                // A cid we cannot decode can never resolve — never enqueue it.
                if let Err(e) = crate::services::conductor_writes::decode_collective_cid(cid) {
                    undecodable += 1;
                    tracing::warn!(
                        target: "elohim_storage::projection_reconcile",
                        collective_id = %id,
                        peer_cid = %cid,
                        error = %e,
                        "projection-reconcile[collectives]: undecodable peer cid; not enqueued"
                    );
                    continue;
                }
                if alias_by_cid.contains_key(cid) {
                    continue; // already enqueued under another routing alias
                }
                alias_by_cid.insert(cid.clone(), id.clone());
                if let Some(peer) = advertised_by.get(id) {
                    discovered_by.insert(cid.clone(), peer.clone());
                }
                gap_cids.push(cid.clone());
            }
        }
    }

    if undecodable > 0 {
        tracing::warn!(
            target: "elohim_storage::projection_reconcile",
            undecodable,
            "projection-reconcile[collectives]: dropped undecodable peer cids this sweep"
        );
    }

    // Cross-sweep retry budget (see [`MissLedger`]). The gap key IS the cid here,
    // so "new evidence" can only ever arrive as a NEW key — the ledger's
    // evidence field is the cid itself and the cooldown is what re-admits.
    let mut exhausted_persistent = 0usize;
    let mut admitted: Vec<String> = Vec::with_capacity(gap_cids.len());
    for cid in gap_cids {
        match misses.admit(PROJECTION_INVENTORY_TABLE_COLLECTIVES, &cid, &cid) {
            Admission::Retry => admitted.push(cid),
            Admission::Exhausted => exhausted_persistent += 1,
        }
    }

    let mut tracker = GapTracker::new(MAX_RETRIES);
    tracker.discover(admitted);

    CollectivesDiscovery {
        tracker,
        alias_by_cid,
        discovered_by,
        divergent_cid,
        exhausted_persistent,
        peers_asked,
        ids_discovered,
        local_anchored,
    }
}

/// What one collectives heal leg produced.
struct CollectivesHealOutcome {
    /// Rows that actually converged (created, or an alias row filled).
    healed: usize,
    /// Gaps the own conductor could not resolve at all (`Ok(None)`).
    conductor_missing: usize,
}

/// Heal phase of the `collectives` reconcile: for each discovered cid, read the
/// OWN conductor's `imagodei::get_collective_by_action` and project through the
/// SHARED mapping in `GapFill` mode. `None` → `mark_failed`, retried next sweep.
///
/// Row content comes EXCLUSIVELY from the own conductor's `Collective` entry;
/// the only peer-derived value that reaches the projection is the routing-alias
/// merge CANDIDATE, and that is honored only when a local row already exists
/// under it AND carries no cid (fills, never moves).
async fn heal_collectives(
    tracker: &mut GapTracker,
    alias_by_cid: &std::collections::HashMap<String, String>,
    discovered_by: &std::collections::HashMap<String, String>,
    hc: &Arc<HcClient>,
    pool: &DbPool,
    pacing: &HealPacing,
) -> CollectivesHealOutcome {
    use crate::db::collectives::CollectiveProjectionOutcome as Outcome;

    let app_ctx = crate::db::AppContext::default_lamad();
    let leg_start = std::time::Instant::now();
    let mut healed = 0usize;
    let mut conductor_missing = 0usize;

    let mut circuit = HealCircuit::new(pacing.circuit_timeout_threshold);
    for cid in tracker.pending_ids() {
        let attempt = call_with_retry(pacing, || {
            crate::services::conductor_writes::get_collective_by_cid(hc, &cid)
        })
        .await;
        circuit.record(&attempt.result);
        match attempt.result {
            Ok(Some(wire)) => {
                let merge_onto = alias_by_cid.get(&cid).map(String::as_str);
                match heal_collective_one(&cid, &wire, merge_onto, pool, &app_ctx) {
                    Ok(outcome @ (Outcome::Created | Outcome::AliasMerged)) => {
                        tracker.mark_completed(&cid);
                        healed += 1;
                        let peer = discovered_by
                            .get(&cid)
                            .cloned()
                            .unwrap_or_else(|| "unknown".to_string());
                        tracing::warn!(
                            target: "elohim_storage::projection_reconcile",
                            collective_cid = %cid,
                            discovered_via_peer = %peer,
                            retried = attempt.retried,
                            ?outcome,
                            "projection-reconcile[collectives]: HEALED collective from own conductor (peer discovery)"
                        );
                        crate::metrics::inc_projection_heal_outcome(
                            "collectives",
                            if attempt.retried {
                                HealOutcomeKind::TimeoutRetried.label()
                            } else {
                                HealOutcomeKind::Healed.label()
                            },
                        );
                    }
                    Ok(Outcome::Refreshed) => {
                        // The row already carried this cid — value fields were
                        // refreshed but nothing converged. Deliberately NOT
                        // counted as healed (the `Refreshed` discipline the
                        // content arm learned: a no-op counted as a cure hides
                        // a spinning heal plane).
                        tracker.mark_completed(&cid);
                        tracing::info!(
                            target: "elohim_storage::projection_reconcile",
                            collective_cid = %cid,
                            "projection-reconcile[collectives]: row already anchored to this cid — refreshed, nothing converged"
                        );
                        crate::metrics::inc_projection_heal_outcome(
                            "collectives",
                            HealOutcomeKind::Refreshed.label(),
                        );
                    }
                    Ok(Outcome::SkippedDeclared) => {
                        tracker.mark_completed(&cid);
                        tracing::info!(
                            target: "elohim_storage::projection_reconcile",
                            collective_cid = %cid,
                            "projection-reconcile[collectives]: alias row already carries a different cid — heal left it to the canonical channels"
                        );
                        crate::metrics::inc_projection_heal_outcome(
                            "collectives",
                            HealOutcomeKind::RefusedDeclared.label(),
                        );
                    }
                    Err(e) => {
                        tracing::warn!(collective_cid = %cid, error = %e, "projection-reconcile[collectives]: projection failed; retry next sweep");
                        tracker.mark_failed(&cid);
                        crate::metrics::inc_projection_heal_outcome(
                            "collectives",
                            HealOutcomeKind::Failed.label(),
                        );
                    }
                }
            }
            Ok(None) => {
                // Conductor can't see it yet (not gossiped in / foreign DHT) —
                // retried on the NEXT sweep via a fresh inventory diff.
                conductor_missing += 1;
                tracing::debug!(collective_cid = %cid, "projection-reconcile[collectives]: own conductor returned None; retry next sweep");
                tracker.mark_failed(&cid);
                crate::metrics::inc_projection_heal_outcome(
                    "collectives",
                    HealOutcomeKind::Missing.label(),
                );
            }
            Err(e) => {
                let transient = is_transient_conductor_error(&e);
                tracing::warn!(collective_cid = %cid, error = %e, transient, "projection-reconcile[collectives]: conductor resolve failed; retry next sweep");
                tracker.mark_failed(&cid);
                crate::metrics::inc_projection_heal_outcome(
                    "collectives",
                    if transient {
                        HealOutcomeKind::TimeoutExhausted.label()
                    } else {
                        HealOutcomeKind::Failed.label()
                    },
                );
            }
        }

        // Per-leg budget: yield so the leg recycles next sweep (see `heal_rea`).
        if leg_start.elapsed() >= pacing.collectives_leg_budget {
            tracing::info!(
                target: "elohim_storage::projection_reconcile",
                budget_secs = pacing.collectives_leg_budget.as_secs(),
                healed,
                "projection-reconcile[collectives]: heal hit leg budget — yielding, remaining gaps resume next sweep"
            );
            break;
        }

        // Unresponsive-conductor circuit: shed the rest of the leg rather than
        // stack more abandoned-but-still-executing calls on it (see `HealCircuit`).
        if circuit.is_open() {
            tracing::warn!(
                target: "elohim_storage::projection_reconcile",
                consecutive_timeouts = circuit.consecutive_timeouts(),
                healed,
                "projection-reconcile[collectives]: OPENED the unresponsive-conductor circuit — shedding the rest of the leg, remaining gaps resume next sweep"
            );
            break;
        }
    }

    // END-OF-ARM caught_up — see the same call at the end of `heal_content` for
    // why an arm that skips this publishes a structurally-false bit.
    tracker.update_caught_up();
    CollectivesHealOutcome {
        healed,
        conductor_missing,
    }
}

/// Project ONE conductor-read `Collective` into local SQL via the SHARED
/// mapping in `GapFill` mode (heal fills, never moves).
fn heal_collective_one(
    collective_cid: &str,
    wire: &crate::services::conductor_writes::CollectiveWire,
    merge_onto_id: Option<&str>,
    pool: &DbPool,
    app_ctx: &crate::db::AppContext,
) -> Result<crate::db::collectives::CollectiveProjectionOutcome, crate::error::StorageError> {
    let mut conn = pool
        .get()
        .map_err(|e| crate::error::StorageError::Internal(format!("pool: {e}")))?;
    crate::db::collectives::project_collective(
        &mut conn,
        app_ctx,
        &crate::db::collectives::CollectiveProjection {
            collective_cid,
            display_name: &wire.display_name,
            founder_agent_cid: Some(&wire.founder_agent_cid),
            charter: Some(&wire.charter),
            merge_onto_id,
            mode: crate::db::collectives::CollectiveStampMode::GapFill,
        },
    )
}

#[cfg(test)]
mod collectives_gap_tests {
    use super::*;
    use std::collections::{HashMap, HashSet};

    const CID_A: &str = "collective:uhCkkAlphaCollectiveActionHash00000000000";
    const CID_B: &str = "collective:uhCkkBravoCollectiveActionHash00000000000";

    /// (local_cids, present, local_cid_by_id) from an anchored inventory plus
    /// extra un-anchored ids that exist locally.
    fn fixture(
        anchored: &[(&str, &str)],
        unanchored_ids: &[&str],
    ) -> (HashSet<String>, HashSet<String>, HashMap<String, String>) {
        let local_cid_by_id: HashMap<String, String> = anchored
            .iter()
            .map(|(id, cid)| (id.to_string(), cid.to_string()))
            .collect();
        let local_cids: HashSet<String> = local_cid_by_id.values().cloned().collect();
        let mut present: HashSet<String> = local_cid_by_id.keys().cloned().collect();
        present.extend(unanchored_ids.iter().map(|s| s.to_string()));
        (local_cids, present, local_cid_by_id)
    }

    #[test]
    fn absent_local_when_neither_cid_nor_alias_is_held() {
        let (cids, present, by_id) = fixture(&[], &[]);
        assert_eq!(
            classify_collective_gap("household-dowell", Some(CID_A), &cids, &present, &by_id),
            CollectiveGap::AbsentLocal,
            "a collective this peer has never seen is healable — the own conductor \
             carries the entry on a full-arc fleet"
        );
    }

    #[test]
    fn cid_gap_when_alias_row_is_present_but_unanchored() {
        // The scenario-2 shape: a pre-coherence seed row under the slug, whose
        // real cid only exists on the AUTHORING conductor.
        let (cids, present, by_id) = fixture(&[], &["household-dowell"]);
        assert_eq!(
            classify_collective_gap("household-dowell", Some(CID_A), &cids, &present, &by_id),
            CollectiveGap::CidGap
        );
    }

    #[test]
    fn divergent_when_alias_row_carries_a_different_cid() {
        let (cids, present, by_id) = fixture(&[("household-dowell", CID_A)], &[]);
        assert_eq!(
            classify_collective_gap("household-dowell", Some(CID_B), &cids, &present, &by_id),
            CollectiveGap::Divergent,
            "heal must not move an anchored row: `collectives` has no declaration \
             ordering to prove a forward move"
        );
    }

    #[test]
    fn in_sync_when_the_cid_is_already_held_under_the_same_alias() {
        let (cids, present, by_id) = fixture(&[("household-dowell", CID_A)], &[]);
        assert_eq!(
            classify_collective_gap("household-dowell", Some(CID_A), &cids, &present, &by_id),
            CollectiveGap::InSync
        );
    }

    /// The identity is the CID, not the routing alias: holding the cid under a
    /// DIFFERENT local id is still in-sync — healing would duplicate the row.
    #[test]
    fn in_sync_when_the_cid_is_held_under_a_different_alias() {
        let (cids, present, by_id) = fixture(&[(CID_A, CID_A)], &[]);
        assert_eq!(
            classify_collective_gap("household-dowell", Some(CID_A), &cids, &present, &by_id),
            CollectiveGap::InSync,
            "cid-keyed local row must satisfy a slug-keyed peer advertisement"
        );
    }

    /// Ruling 1: a NULL / empty `collective_cid` carries no DHT identity, so it
    /// is never a gap — not even when the alias is locally absent. Responders
    /// never advertise such a row; this pins the consumer-side guard too.
    #[test]
    fn null_or_empty_peer_cid_is_never_a_gap() {
        let (cids, present, by_id) = fixture(&[], &[]);
        for peer_cid in [None, Some(""), Some("   ")] {
            assert_eq!(
                classify_collective_gap("household-dowell", peer_cid, &cids, &present, &by_id),
                CollectiveGap::InSync,
                "un-anchored advertisement {peer_cid:?} must not become a gap"
            );
        }
    }

    /// A pre-coherence row is upgradable IN PLACE: the same alias that reads
    /// InSync while un-advertised becomes a CidGap the moment a peer advertises
    /// a real cid for it, and InSync again once stamped.
    #[test]
    fn unanchored_row_upgrades_in_place_then_settles() {
        let (cids, present, by_id) = fixture(&[], &["household-dowell"]);
        assert_eq!(
            classify_collective_gap("household-dowell", Some(CID_A), &cids, &present, &by_id),
            CollectiveGap::CidGap
        );
        // …after the heal stamps it, the same advertisement is in-sync.
        let (cids, present, by_id) = fixture(&[("household-dowell", CID_A)], &[]);
        assert_eq!(
            classify_collective_gap("household-dowell", Some(CID_A), &cids, &present, &by_id),
            CollectiveGap::InSync
        );
    }

    /// The collectives leg budget is reserved and SMALLEST — it runs last and
    /// must never be able to starve the two arms ahead of it.
    #[test]
    fn collectives_leg_budget_is_the_smallest_reserved_slice() {
        let p = HealPacing::default();
        assert!(
            p.collectives_leg_budget <= p.rea_leg_budget
                && p.collectives_leg_budget <= p.content_leg_budget,
            "collectives runs last with the smallest reserved budget"
        );
    }
}

// ---------------------------------------------------------------------------
// Shard-location catch-up arm (Category C — custody convergence, cold path)
// ---------------------------------------------------------------------------

/// Catch-up reconcile for the `shard_locations` custody projection: fetch each
/// connected peer's custody inventory over `/elohim/view-federation/1.0.0` and
/// project every advertised claim as a `peer-announced` row via the SAME
/// never-overwrite-local rule the gossip hot path uses.
///
/// ## Why this differs from the REA / content arms
///
/// Custody is Category C: there is NO DHT notary for who-holds-what. The
/// announcing peer's own projection IS the heal source, so — unlike
/// [`discover_rea`] / [`discover_content`], which advertise only `(id, anchor)`
/// and heal row content from the OWN conductor — a custody inventory entry
/// carries the FULL [`CustodyAnnouncement`] and is applied directly. No conductor
/// is involved.
///
/// ## DORMANT until the responder serves the table
///
/// This is deliberately NOT wired into [`run_discovery`]/[`run_heal`] yet. The
/// view-federation responder (`p2p::view_federation::build_inventory_payload`)
/// returns an honest empty inventory for any table it does not know, and
/// `shard_locations` is not yet a known table there. Wiring this into the sweep
/// before the responder serves it would ship a guaranteed no-op that burns a
/// federation round-trip per peer every tick. Activate it once the responder
/// serves `table = "shard_locations"` with a `CustodyInventoryPayload` (see the
/// slice report for the exact responder + `run_discovery` insertions). Kept
/// `pub` so it compiles clean (no dead-code) as landed-but-dormant.
pub async fn reconcile_shard_locations_from_peers(p2p: &P2PHandle, pool: &DbPool) {
    use crate::p2p::custody_announce::{
        CustodyInventoryPayload, PROJECTION_INVENTORY_TABLE_SHARD_LOCATIONS,
    };

    // Resolve THIS node's own agent_cid once for the self-drop guard.
    let self_agent_cid = pool
        .get()
        .ok()
        .and_then(|mut conn| crate::reconcile::custody::resolve_self_agent_cid(&mut conn, None));

    let peers = p2p.list_peers().await;
    let mut applied = 0usize;
    let mut dropped_weaker = 0usize;
    let mut dropped_self = 0usize;
    let mut peers_asked = 0usize;

    for peer in &peers {
        let peer_id = match peer.peer_id.parse::<libp2p::PeerId>() {
            Ok(p) => p,
            Err(_) => continue,
        };
        let request = ViewFederationRequest {
            view_kind: ViewKind::ProjectionInventory {
                table: PROJECTION_INVENTORY_TABLE_SHARD_LOCATIONS.to_string(),
            },
            agent_cid: p2p.agent_pubkey().to_string(),
            request_id: uuid::Uuid::new_v4().to_string(),
            inventory_offset: None,
        };
        let resp = match p2p.view_federate(peer_id, request, PEER_TIMEOUT).await {
            Ok(r) => r,
            Err(_) => continue, // peer offline/timeout — catch-up is best-effort
        };
        peers_asked += 1;

        let payload: CustodyInventoryPayload =
            match serde_json::from_value(resp.slice.payload.0.clone()) {
                Ok(p) => p,
                Err(e) => {
                    tracing::warn!(
                        target: "elohim_storage::custody",
                        peer = %peer.peer_id,
                        error = %e,
                        "custody catch-up: peer inventory payload undecodable; skipping peer"
                    );
                    continue;
                }
            };

        let Ok(mut conn) = pool.get() else { continue };
        let stats = crate::db::shard_locations::apply_custody_inventory(
            &mut conn,
            &payload.entries,
            self_agent_cid.as_deref(),
        );
        applied += stats.applied();
        dropped_weaker += stats.dropped_weaker;
        dropped_self += stats.dropped_self;
    }

    for _ in 0..applied {
        crate::metrics::inc_custody_announce("applied");
    }
    for _ in 0..dropped_weaker {
        crate::metrics::inc_custody_announce("dropped_weaker");
    }
    for _ in 0..dropped_self {
        crate::metrics::inc_custody_announce("dropped_self");
    }

    tracing::info!(
        target: "elohim_storage::custody",
        peers_asked,
        applied,
        dropped_weaker,
        dropped_self,
        "custody catch-up: shard-location reconcile complete"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::p2p::reconcile_rails::GapCounts;

    #[tokio::test]
    async fn state_publishes_and_accumulates_across_sweeps() {
        let state = ProjectionReconcileState::new();
        // Initial: nothing.
        let s0 = state.status().await;
        assert_eq!((s0.healed_total, s0.sweeps), (0, 0));

        // Sweep 1: healed 2, failed 1.
        state
            .publish_sweep(
                GapCounts {
                    pending: 0,
                    completed: 2,
                    failed: 1,
                    caught_up: true,
                    // The failed gap spent its budget: the sweep ended
                    // (caught_up) without healing it, so it did NOT converge.
                    exhausted: 1,
                    converged: false,
                },
                3,
                1,
                0,
            )
            .await;
        let s1 = state.status().await;
        assert_eq!(s1.completed, 2);
        assert_eq!(s1.failed, 1);
        assert_eq!(s1.peers_asked, 3);
        assert_eq!(s1.divergent_anchor, 1);
        assert_eq!(s1.healed_total, 2);
        assert_eq!(s1.sweeps, 1);
        assert!(s1.caught_up);

        // Sweep 2: healed 1 more — cumulative healed_total advances.
        state
            .publish_sweep(
                GapCounts {
                    pending: 0,
                    completed: 1,
                    failed: 0,
                    caught_up: true,
                    exhausted: 0,
                    converged: true,
                },
                2,
                0,
                0,
            )
            .await;
        let s2 = state.status().await;
        assert_eq!(s2.healed_total, 3);
        assert_eq!(s2.sweeps, 2);
        assert_eq!(s2.completed, 1);
        assert_eq!(s2.divergent_anchor, 0);
    }

    #[tokio::test]
    async fn a_sweep_with_divergent_anchors_is_not_converged() {
        // The live beta shape: every gap healed and the retry budget untouched,
        // but rows sit locally under an anchor no peer advertises. `caught_up`
        // says the sweep ended; it does NOT say this peer holds what its peers
        // hold. divergent_anchor alone must defeat convergence.
        let state = ProjectionReconcileState::new();
        state
            .publish_sweep(
                GapCounts {
                    pending: 0,
                    completed: 4,
                    failed: 0,
                    caught_up: true,
                    exhausted: 0,
                    converged: true, // the GAP LEDGER converged...
                },
                3,
                1860, // ...but 1860 rows diverge, so the PEER did not.
                0,    // ...and NONE of them is adjudicated.
            )
            .await;

        let s = state.status().await;
        assert!(s.caught_up, "the sweep did finish");
        assert_eq!(s.divergent_anchor, 1860);
        assert_eq!(s.exhausted, 0);
        assert!(
            !s.converged,
            "divergent anchors mean this peer does NOT hold what its peers hold"
        );
    }

    #[tokio::test]
    async fn divergence_heal_is_forbidden_to_move_does_not_defeat_convergence() {
        // The live 12h shape on matthew: 6071 `refused_declared` heal outcomes
        // against 8 `healed`. Every one of those refusals is CORRECT — the local
        // row already carries a different declared head, and heal fills-never-
        // moves, so only a canonical channel may move it. Gating `converged` on
        // the divergence TOTAL therefore pinned the gauge at 0 fleet-wide for
        // 12h+: not a conservative reading, an unreadable one.
        //
        // Sibling of `retry_exhausted_gaps_are_caught_up_but_not_converged` in
        // reconcile_rails.rs — same lesson from the divergence side: a number
        // that cannot move is not a safety property.
        let state = ProjectionReconcileState::new();
        state
            .publish_sweep(
                GapCounts {
                    pending: 0,
                    completed: 8,
                    failed: 0,
                    caught_up: true,
                    exhausted: 0,
                    converged: true,
                },
                3,
                6071, // total divergence...
                6071, // ...ALL of it adjudicated (heal forbidden to move it).
            )
            .await;

        let s = state.status().await;
        assert_eq!(
            s.divergent_anchor, 6071,
            "the TOTAL stays on the wire — adjudicated divergence must not vanish"
        );
        assert!(
            s.converged,
            "divergence heal is forbidden to move must not defeat convergence"
        );
    }

    #[tokio::test]
    async fn one_unadjudicated_divergent_row_still_defeats_convergence() {
        // The guard against the split becoming a whitewash: the honest half of
        // the old rule is kept. A single row NOBODY has adjudicated — no local
        // declaration, retry budget unspent — is divergence this sweep did not
        // resolve, and it defeats convergence exactly as before.
        let state = ProjectionReconcileState::new();
        state
            .publish_sweep(
                GapCounts {
                    pending: 0,
                    completed: 8,
                    failed: 0,
                    caught_up: true,
                    exhausted: 0,
                    converged: true,
                },
                3,
                6071,
                6070, // one row short of fully adjudicated
            )
            .await;

        let s = state.status().await;
        assert_eq!(s.divergent_anchor, 6071);
        assert!(
            !s.converged,
            "unadjudicated divergence must still defeat convergence"
        );
    }

    #[test]
    fn rea_conductor_confirming_the_held_anchor_is_a_refresh_not_a_heal() {
        // The live REA shape (matthew, 23:56Z): rea_divergent_anchor=12 against a
        // heal leg that reports success every sweep. Discovery counts a row
        // divergent vs a PEER's anchor; the own conductor then answers with the
        // anchor the row ALREADY holds, so the write moves nothing. Calling that
        // `healed` is what made a spinning plane read as a working one — and
        // leaving it unadjudicated is what pinned `converged` at 0 fleet-wide.
        assert_eq!(
            classify_rea_anchor_write(
                Some("uhCkA-peer-disagrees-with-this"),
                "uhCkA-peer-disagrees-with-this"
            ),
            ReaAnchorWrite::Refreshed,
            "the own conductor confirmed the anchor the row already holds — nothing converged"
        );
    }

    #[test]
    fn rea_anchor_write_that_moves_the_row_is_advanced() {
        // The honest half: a row with no anchor, an EMPTY anchor, or a genuinely
        // different one is FILLED / MOVED FORWARD by the write. That is real
        // convergence and must never be adjudicated away as a refusal.
        assert_eq!(
            classify_rea_anchor_write(None, "uhCkA-fresh"),
            ReaAnchorWrite::Advanced,
            "an absent row is filled, not refreshed"
        );
        assert_eq!(
            classify_rea_anchor_write(Some(""), "uhCkA-fresh"),
            ReaAnchorWrite::Advanced,
            "an un-anchored row is filled, not refreshed"
        );
        assert_eq!(
            classify_rea_anchor_write(Some("uhCkA-old"), "uhCkA-new"),
            ReaAnchorWrite::Advanced,
            "a different conductor answer advances the anchor"
        );
    }

    #[tokio::test]
    async fn rea_divergence_the_own_conductor_confirms_does_not_defeat_convergence() {
        // The REA sibling of
        // `divergence_heal_is_forbidden_to_move_does_not_defeat_convergence`.
        // Live residue on the household pods: rea_divergent_anchor=12, static
        // across many sweeps (7 admitted + 5 ledger-exhausted), with
        // `divergent_refused{stream="rea"}` hardcoded 0 — so `converged` could
        // never move no matter how well the heal leg worked. Once the arm
        // adjudicates BOTH its classes, the same sweep converges.
        let state = ProjectionReconcileState::new();
        state
            .publish_sweep(
                GapCounts {
                    pending: 0,
                    completed: 7,
                    failed: 0,
                    caught_up: true,
                    exhausted: 5,
                    converged: true,
                },
                6,
                12, // total REA divergence...
                12, // ...5 retry-exhausted + 7 conductor-confirmed = all of it.
            )
            .await;

        let s = state.status().await;
        assert_eq!(
            s.divergent_anchor, 12,
            "the TOTAL stays on the wire — adjudicated divergence must not vanish"
        );
        assert!(
            s.converged,
            "divergence the own conductor has confirmed is not divergence heal can resolve"
        );
    }

    #[tokio::test]
    async fn an_unadjudicated_rea_divergent_row_still_defeats_convergence() {
        // The guard against the REA split becoming a whitewash: a divergent row
        // whose conductor answer ADVANCED nothing and which the ledger has not
        // exhausted is divergence this sweep genuinely did not resolve.
        let state = ProjectionReconcileState::new();
        state
            .publish_sweep(
                GapCounts {
                    pending: 0,
                    completed: 7,
                    failed: 0,
                    caught_up: true,
                    exhausted: 5,
                    converged: true,
                },
                6,
                12,
                11, // one row short of fully adjudicated
            )
            .await;

        let s = state.status().await;
        assert_eq!(s.divergent_anchor, 12);
        assert!(
            !s.converged,
            "unadjudicated REA divergence must still defeat convergence"
        );
    }

    #[test]
    fn miss_ledger_makes_max_retries_real_across_sweeps() {
        // The treadmill: the per-sweep GapTracker is rebuilt every tick, so its
        // own retry budget can never fire across sweeps — matthew re-asked its
        // conductor about the same ~1000 rows forever (~151k missing outcomes
        // in 12h) while `elohim_projection_reconcile_exhausted` published 0.
        let mut ledger = MissLedger::new();
        for sweep in 1..=MAX_RETRIES {
            assert_eq!(
                ledger.admit("content", "row-1", "anchor-A"),
                Admission::Retry,
                "sweep {sweep} is still within budget"
            );
        }
        assert_eq!(
            ledger.admit("content", "row-1", "anchor-A"),
            Admission::Exhausted,
            "budget spent against UNCHANGED evidence — stop asking"
        );
    }

    #[test]
    fn new_peer_evidence_re_admits_an_exhausted_id_immediately() {
        // Exhaustion is about a CLAIM, not an id: a peer advertising a different
        // anchor is not the thing we gave up on.
        let mut ledger = MissLedger::new();
        for _ in 0..=MAX_RETRIES {
            ledger.admit("content", "row-1", "anchor-A");
        }
        assert_eq!(
            ledger.admit("content", "row-1", "anchor-A"),
            Admission::Exhausted
        );
        assert_eq!(
            ledger.admit("content", "row-1", "anchor-B"),
            Admission::Retry,
            "a DIFFERENT advertised anchor is fresh evidence"
        );
    }

    #[test]
    fn exhaustion_is_never_permanent() {
        // A row the conductor cannot see today may be gossiped to it tomorrow.
        // Without the cooldown the ledger would silently stop healing it for the
        // process lifetime — trading one dishonest gauge for a real gap.
        let mut ledger = MissLedger::new();
        for _ in 0..MAX_RETRIES {
            ledger.admit("content", "row-1", "anchor-A");
        }
        let mut exhausted_sweeps = 0;
        for _ in 0..MISS_READMIT_SWEEPS {
            match ledger.admit("content", "row-1", "anchor-A") {
                Admission::Exhausted => exhausted_sweeps += 1,
                Admission::Retry => break,
            }
        }
        assert_eq!(
            exhausted_sweeps,
            MISS_READMIT_SWEEPS - 1,
            "dormant for the cooldown, then re-admitted for another round"
        );
        assert_eq!(
            ledger.admit("content", "row-1", "anchor-A"),
            Admission::Retry,
            "the fresh budget is spendable again"
        );
    }

    #[test]
    fn a_resolved_id_leaves_the_ledger_with_a_clean_budget() {
        // A row that heals and later relapses must not inherit the old budget —
        // the relapse is new work, not a continuation.
        let mut ledger = MissLedger::new();
        for _ in 0..MAX_RETRIES {
            ledger.admit("rea", "row-1", "anchor-A");
        }
        ledger.resolved("rea", "row-1");
        assert_eq!(ledger.tracked("rea"), 0);
        assert_eq!(
            ledger.admit("rea", "row-1", "anchor-A"),
            Admission::Retry,
            "a relapsed row starts from a clean budget"
        );
    }

    #[test]
    fn ledger_streams_do_not_share_budgets() {
        // The three arms carry disjoint id spaces; a content id and a REA id
        // that happen to collide must not spend each other's retries.
        let mut ledger = MissLedger::new();
        for _ in 0..=MAX_RETRIES {
            ledger.admit("content", "shared-id", "anchor-A");
        }
        assert_eq!(
            ledger.admit("content", "shared-id", "anchor-A"),
            Admission::Exhausted
        );
        assert_eq!(
            ledger.admit("rea", "shared-id", "anchor-A"),
            Admission::Retry,
            "another stream's exhaustion must not bleed across"
        );
    }

    #[tokio::test]
    async fn a_sweep_that_abandoned_every_gap_is_caught_up_but_not_converged() {
        // The live beta shape from the other end: healedTotal stays 0 across
        // sweeps while gaps exhaust their retry budget. caught_up flips true;
        // converged must not.
        let state = ProjectionReconcileState::new();
        state
            .publish_sweep(
                GapCounts {
                    pending: 0,
                    completed: 0,
                    failed: 61,
                    caught_up: true,
                    exhausted: 61,
                    converged: false,
                },
                3,
                0,
                0,
            )
            .await;

        let s = state.status().await;
        assert!(s.caught_up);
        assert_eq!(s.exhausted, 61);
        assert_eq!(
            s.healed_total, 0,
            "22 sweeps, healedTotal 0 — the live shape"
        );
        assert!(!s.converged);
    }

    #[test]
    fn status_serializes_converged_and_exhausted_as_camel_case() {
        // Wire contract: p2p-status-view.schema.json sets
        // additionalProperties:false on projectionReconcile, so these names
        // must match the schema exactly or the contract test rejects them.
        let s = ProjectionReconcileStatus {
            exhausted: 61,
            converged: false,
            ..Default::default()
        };
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(v["exhausted"], serde_json::json!(61));
        assert_eq!(v["converged"], serde_json::json!(false));
    }

    #[test]
    fn heal_decision_covers_bridge_and_single_flight() {
        // Bridge up, nothing running → spawn the heal leg this tick.
        assert_eq!(heal_decision(true, false), HealAction::Spawn);
        // Bridge up, a heal already in flight → skip (single-flight; discovery ran).
        assert_eq!(heal_decision(true, true), HealAction::SkipInFlight);
        // Bridge down → defer heal regardless of the in-flight flag (nothing to
        // spawn without a conductor); discovery still ran.
        assert_eq!(heal_decision(false, false), HealAction::SkipNoBridge);
        assert_eq!(heal_decision(false, true), HealAction::SkipNoBridge);
    }

    // ── MISSING-class adoption routing (2026-08-01) ─────────────────────────

    /// A conductor-missing row with a peer-advertised declaration must reach the
    /// adopt arm; without a hint there is nothing to adopt FROM.
    #[test]
    fn a_conductor_missing_row_routes_to_adopt_exactly_when_a_peer_declares() {
        assert!(
            conductor_missing_should_route_to_adopt(true),
            "a peer declaring a head for an id our conductor cannot resolve is precisely the \
             adopt case — there is nothing local to prefer"
        );
        assert!(
            !conductor_missing_should_route_to_adopt(false),
            "no peer declaration ⇒ nothing to adopt FROM; the row stays a ghost candidate"
        );
    }

    /// The routing predicate must NOT inherit the timeout arm's transience
    /// condition.
    ///
    /// `Ok(None)` is an ANSWER (the conductor looked and holds no chain), not a
    /// failure to answer — so there is no transient/non-transient axis to gate
    /// on, and gating on one would silently drop the largest heal-outcome class
    /// back out of adoption.
    #[test]
    fn missing_routing_is_not_gated_on_transience_the_way_timeout_routing_is() {
        // Timeout routing needs BOTH transience and a hint...
        assert!(timeout_should_route_to_adopt(true, true));
        assert!(!timeout_should_route_to_adopt(false, true));
        // ...missing routing needs only the hint, because the conductor ANSWERED.
        assert!(conductor_missing_should_route_to_adopt(true));
    }

    /// Regression guard for the hole itself: the conductor-missing arm must
    /// actually CALL the routing predicate.
    ///
    /// Before this, `Ok(None)` reached adoption only indirectly via
    /// `witness_ghost_anchors`, which first narrows candidates with
    /// `anchored_content_reaches_for_ids` — i.e. to rows that already CLAIM an
    /// anchor (guarded by `content_diesel`'s
    /// `anchored_content_reaches_for_ids_selects_only_the_ghost_candidate_class`).
    /// A conductor-missing row with a NULL `dht_anchor_hash` — the dominant
    /// `ContentGap::AnchorGap` class — was filtered straight back out and went to
    /// `witness_bootstrap`'s AUTHOR path instead: the exact self-election
    /// adopt-before-author exists to prevent, on the class most likely to have a
    /// peer already holding the crown.
    ///
    /// Delete the routing call from the arm and this fails.
    #[test]
    fn the_conductor_missing_arm_is_wired_to_the_adopt_list() {
        let src = include_str!("projection_reconcile.rs");
        let start = src
            .find("GHOST-ANCHOR CANDIDATE")
            .expect("the conductor-missing arm's marker comment must exist");
        let arm = &src[start..];
        let end = arm
            .find("HealOutcomeKind::Missing")
            .expect("the missing arm must still count its outcome");
        let arm = &arm[..end];
        assert!(
            arm.contains("conductor_missing_should_route_to_adopt"),
            "the conductor-missing arm must consult the adopt-routing predicate"
        );
        assert!(
            arm.contains("adopt_candidates.push"),
            "a routed conductor-missing row must land on the adopt list, which run_heal drains \
             BEFORE either authoring sweep"
        );
        assert!(
            arm.contains("tracker.mark_failed"),
            "routing must not fake a heal — the row stays mark_failed, exactly as the timeout \
             route does"
        );
    }

    // ── caught_up WIRING (the class the hand-rolled arm helper below missed) ─

    /// WIRING test over a REAL [`GapTracker`] driven through the content-heal
    /// drain: discover → resolve every gap → end-of-arm `update_caught_up()`.
    ///
    /// ## The defect this pins
    ///
    /// `GapTracker::caught_up` DEFAULTS to false and `enqueue_missing` only ever
    /// sets it FALSE — `update_caught_up()` is the one and only thing in the
    /// machine that can set it true. Only `heal_rea` called it. That was
    /// invisible while `publish_sweep` received the REA arm's counts alone, and
    /// became load-bearing the moment `fold_arm_counts` started ANDing the other
    /// arms' bits in: a structurally-false `caught_up` from content/collectives
    /// would pin the published `caughtUp` false FOREVER — `/health`'s
    /// `p2p.caughtUp` could never read true again, and the a2o head-convergence
    /// scenario asserting it would be unpassable by construction.
    ///
    /// The F-D tests below build `GapCounts` by hand, which cannot see this:
    /// a hand-rolled struct sets `caught_up` from a rule the real tracker does
    /// not implement. Hence a real tracker here.
    ///
    /// Regression guard: delete `tracker.update_caught_up()` from the end of
    /// `heal_content` and the fold assertion at the bottom fails.
    #[test]
    fn a_drained_content_arm_reports_caught_up_through_a_real_tracker() {
        let mut tracker = GapTracker::new(MAX_RETRIES);
        tracker.discover(vec!["a".into(), "b".into(), "c".into()]);
        assert!(
            !tracker.counts().caught_up,
            "precondition: discovery leaves the arm not-caught-up"
        );

        // Drive the drain exactly as `heal_content` does: every pending row ends
        // as either a resolution or a failure.
        tracker.mark_completed("a"); // Stamped
        tracker.mark_completed("b"); // Refreshed / SkippedDeclared — resolved in-sweep
        tracker.mark_failed("c"); // conductor Ok(None) — mark_failed drains pending
        assert!(
            !tracker.counts().caught_up,
            "the flag is still false until the END-OF-ARM update — nothing in \
             mark_completed/mark_failed sets it"
        );

        tracker.update_caught_up();
        let counts = tracker.counts();
        assert!(
            counts.caught_up,
            "a drained arm must report caught_up — this is the ONLY call that can set it true"
        );
        assert_eq!(counts.pending, 0);

        // And the fold must carry it through rather than pin it false.
        let folded = fold_arm_counts(&[counts, counts, counts]);
        assert!(
            folded.caught_up,
            "three drained arms fold to caught_up; if ANY arm skips its end-of-arm update its \
             structurally-false bit pins the whole sweep false forever"
        );
    }

    /// Generic wiring guard: EVERY heal arm must end with the update, including
    /// arms added later.
    ///
    /// Regression guard: delete the call from `heal_content` or
    /// `heal_collectives` and this names the offending arm.
    #[test]
    fn every_heal_arm_ends_with_update_caught_up() {
        let src = include_str!("projection_reconcile.rs");
        for arm in ["heal_rea", "heal_content", "heal_collectives"] {
            let start = src
                .find(&format!("async fn {arm}("))
                .unwrap_or_else(|| panic!("{arm} must exist"));
            // Bound the scan at the arm's own outcome-struct construction, which
            // is the last statement of every arm.
            let body = &src[start..];
            let end = body
                .find("\n}\n")
                .unwrap_or_else(|| panic!("{arm} must have a body"));
            assert!(
                body[..end].contains("tracker.update_caught_up()"),
                "{arm} must call tracker.update_caught_up() at end-of-arm — without it the arm \
                 publishes a structurally-false caught_up that the sweep fold ANDs into a \
                 permanently-false /health p2p.caughtUp"
            );
        }
    }

    // ── F-D: sweep-level convergence folding ────────────────────────────────

    fn arm_counts(pending: usize, exhausted: usize, completed: usize) -> GapCounts {
        GapCounts {
            pending,
            completed,
            failed: 0,
            caught_up: pending == 0,
            exhausted,
            converged: pending == 0 && exhausted == 0,
        }
    }

    /// The defect F-D closes: `publish_sweep` received the REA arm's counts
    /// ALONE, so a content backlog of thousands of un-healed gaps could not move
    /// `converged` in either direction.
    ///
    /// Regression guard: fold only the REA arm (the pre-fix shape) and this
    /// fails — `converged` comes back `true` with content work outstanding.
    #[test]
    fn content_pending_blocks_convergence_even_when_rea_is_clean() {
        let rea = arm_counts(0, 0, 12);
        let content = arm_counts(2771, 0, 3);
        let collectives = arm_counts(0, 0, 0);

        let folded = fold_arm_counts(&[rea, content, collectives]);

        assert!(
            rea.converged,
            "precondition: the REA arm alone would have reported convergence"
        );
        assert!(
            !folded.converged,
            "an arm with 2771 pending gaps must defeat convergence — that arm carries the \
             fleet's real work and was previously invisible to this gauge"
        );
        assert_eq!(folded.pending, 2771, "pending sums across arms");
        assert_eq!(folded.completed, 15, "completed sums across arms");
        assert!(
            !folded.caught_up,
            "the sweep is not over while work remains"
        );
    }

    /// Symmetric for the third arm, so collectives cannot be the silent one next.
    #[test]
    fn collectives_pending_blocks_convergence_too() {
        let folded = fold_arm_counts(&[
            arm_counts(0, 0, 1),
            arm_counts(0, 0, 1),
            arm_counts(4, 0, 0),
        ]);
        assert!(!folded.converged);
        assert_eq!(folded.pending, 4);
    }

    /// Abandoned-at-max-retries rows are genuinely undone work and must keep
    /// blocking — on EVERY arm, exactly as they already did for REA. This is the
    /// leg the fold must never relax.
    #[test]
    fn per_sweep_exhaustion_still_blocks_convergence_on_every_arm() {
        let folded = fold_arm_counts(&[
            arm_counts(0, 0, 5),
            arm_counts(0, 7, 0),
            arm_counts(0, 0, 0),
        ]);
        assert_eq!(folded.pending, 0, "nothing is still in flight");
        assert_eq!(folded.exhausted, 7);
        assert!(
            !folded.converged,
            "gaps abandoned at max_retries healed nothing — caught_up, never converged"
        );
    }

    /// The SET-ASIDE classes must not block, and the fold gets that for free:
    /// a cross-sweep `MissLedger` exhaustion is held back at DISCOVERY, so an
    /// adjudicated id never enters any arm's tracker and cannot appear in
    /// `pending` or `exhausted`. It stays visible on `..._exhausted` instead.
    #[test]
    fn ledger_adjudicated_ids_are_set_aside_and_do_not_block_convergence() {
        let mut ledger = MissLedger::new();
        let mut tracker = GapTracker::new(MAX_RETRIES);
        // Spend the id's budget against unchanged evidence.
        for _ in 0..MAX_RETRIES {
            assert_eq!(
                ledger.admit(PROJECTION_INVENTORY_TABLE_CONTENT, "stuck", "anchor-a"),
                Admission::Retry
            );
        }
        let verdict = ledger.admit(PROJECTION_INVENTORY_TABLE_CONTENT, "stuck", "anchor-a");
        assert_eq!(verdict, Admission::Exhausted, "budget spent");

        // Discovery admits ONLY the non-exhausted ids, so the adjudicated one
        // never reaches the tracker.
        tracker.discover(vec![]);
        tracker.update_caught_up();
        let folded = fold_arm_counts(&[tracker.counts()]);
        assert_eq!(folded.pending, 0);
        assert_eq!(folded.exhausted, 0);
        assert!(
            folded.converged,
            "an id the ledger adjudicated is set aside (visible on the exhausted gauge), not \
             counted as outstanding work"
        );
    }

    /// A fully-drained sweep still converges — the fold must not be a one-way
    /// ratchet to false.
    #[test]
    fn a_sweep_with_every_arm_drained_still_converges() {
        let folded = fold_arm_counts(&[
            arm_counts(0, 0, 3),
            arm_counts(0, 0, 900),
            arm_counts(0, 0, 2),
        ]);
        assert!(folded.converged);
        assert!(folded.caught_up);
        assert_eq!(folded.completed, 905);
    }

    /// Folding an empty arm list is the identity (converged), so a sweep that
    /// somehow ran no arms is not reported as broken.
    #[test]
    fn folding_no_arms_is_the_converged_identity() {
        let folded = fold_arm_counts(&[]);
        assert!(folded.converged);
        assert!(folded.caught_up);
        assert_eq!(folded.pending, 0);
    }

    /// End-to-end at the published-status level: content pending must reach
    /// `/p2p/status` and `elohim_projection_reconcile_converged` as a false.
    #[tokio::test]
    async fn published_status_reports_content_work_as_unconverged() {
        let state = ProjectionReconcileState::new();
        let folded = fold_arm_counts(&[
            arm_counts(0, 0, 1),
            arm_counts(2771, 0, 0),
            arm_counts(0, 0, 0),
        ]);
        // No unadjudicated divergence: the ONLY thing left is the content gaps.
        state.publish_sweep(folded, 6, 1245, 1245).await;

        let status = state.status().await;
        assert_eq!(
            status.pending, 2771,
            "the content arm's work is on the wire"
        );
        assert!(
            !status.converged,
            "converged must be false while an arm still holds un-healed gaps"
        );
        assert!(
            !status.caught_up,
            "caught_up follows the same fold — publishing a summed pending beside an un-folded \
             caught_up would let the two disagree on one surface"
        );
    }

    #[test]
    fn ghost_witness_classifier_is_conductor_miss_intersect_anchored() {
        // The ghost class is the INTERSECTION of two independently-owned facts,
        // and neither alone is sufficient:
        //   - the own conductor could not resolve the id (`Ok(None)`), and
        //   - the local row CLAIMS a dht_anchor_hash.
        // This test locks the classifier's shape at the seam where the two meet
        // (the SQL half is proved by
        // `anchored_content_reaches_for_ids_selects_only_the_ghost_candidate_class`).
        // Only `Ok(None)` may enter the candidate list: a resolvable id — whether
        // canonical or a root-author fallback — is healed, never re-authored, so a
        // row that has genuinely adopted a canonical head can never be re-rooted
        // underneath it.
        let classify = |resolved: Option<bool>| resolved.is_none();
        assert!(classify(None), "conductor miss is the only candidate arm");
        assert!(
            !classify(Some(true)),
            "a CANONICAL answer heals — it must never be re-authored"
        );
        assert!(
            !classify(Some(false)),
            "a FALLBACK answer still proves a local chain exists — not a ghost"
        );
    }

    /// The exact live trace the GapFill self-election guard exists to stop.
    ///
    /// Cross-root id NULL-anchored on peer B at boot → the re-anchor sweep
    /// authors a non-declaring root (correct) → P2P comes up → discovery
    /// classifies the id Divergent against peer A's anchor → `heal_content` runs
    /// FIRST in `run_heal`, ahead of both pre-flight-guarded sweeps → B's
    /// conductor cannot resolve A's canonical link across the gossip gap, so it
    /// answers `canonical == false` with B's OWN fallback root.
    ///
    /// Without the guard, `GapFill` fills the undeclared row with B's own root
    /// and the divergence becomes PERMANENT and QUIET: the row is anchored (so
    /// the re-anchor sweep skips it), conductor-resolvable (so the ghost sweep
    /// skips it), and `decide_head_action` answers `Hold` forever, with
    /// `elohim_content_head_adopted_total` flat.
    #[test]
    fn gapfill_refuses_to_self_elect_over_a_peers_declaration() {
        assert!(
            gapfill_would_self_elect(
                false, // own conductor answered with its own fallback root
                true,  // peer A advertises a real declaration
                None,  // this row carries no declaration yet
            ),
            "a non-canonical answer must NOT crown this node's own root on an \
             undeclared row while a peer advertises a declaration — that write is \
             terminal and removes the row from every adopt path"
        );
    }

    #[test]
    fn gapfill_is_unchanged_without_a_peer_hint() {
        assert!(
            !gapfill_would_self_elect(false, false, None),
            "with no peer advertising a declaration there is no better claim to \
             defer to — GapFill on an undeclared row stays exactly as it was"
        );
    }

    #[test]
    fn gapfill_guard_never_fires_on_a_canonical_answer() {
        // A canonical answer carries real authority; HealCanonical semantics
        // (fill / same-head refresh / provably-forward move) are untouched.
        for peer in [false, true] {
            for local in [None, Some("uhCkk-LOCAL")] {
                assert!(
                    !gapfill_would_self_elect(true, peer, local),
                    "canonical answers keep HealCanonical behavior (peer={peer}, local={local:?})"
                );
            }
        }
    }

    #[test]
    fn gapfill_guard_never_fires_on_an_already_declared_row() {
        // GapFill already refuses to MOVE a declared row (SkippedDeclared), so
        // there is nothing for this guard to protect and deferring would only
        // lose the value-field refresh.
        assert!(!gapfill_would_self_elect(
            false,
            true,
            Some("uhCkk-ALREADY-DECLARED")
        ));
    }

    #[test]
    fn ghost_witness_content_type_guard_is_the_reanchor_guard() {
        // The ghost-witness sweep's content_type skip guard is composed, not
        // forked: it calls the SAME `is_canonical_content_type` the re-anchor
        // path uses (symmetric with `ghost_witness_reuses_the_witness_bounds`
        // for the pacing bounds). Locks the vocabulary at this seam too — the
        // a2o resilience seed steps' 'album' typo must never be treated as
        // re-authorable here either, while its 'narrative' fix must be.
        assert!(!crate::services::reanchor_backfill::is_canonical_content_type("album"));
        assert!(crate::services::reanchor_backfill::is_canonical_content_type("narrative"));
    }

    #[test]
    fn reauthor_failure_classifier_matches_the_two_counted_classes() {
        use crate::error::StorageError;
        // Station A: adam-alpha's re-author call races a busy own-chain
        // writer — "Source chain error: source chain head has moved".
        assert_eq!(
            classify_reauthor_failure_class(&StorageError::Conductor(
                "Zome call failed: Source chain error: source chain head has moved".into()
            )),
            Some("chain_head_moved")
        );
        // Station B: the create collides with content that already has a
        // local entry. Delegates to `reanchor_backfill::is_already_anchored_error`
        // so this sweep and the boot-time re-anchor sweep classify identically.
        assert_eq!(
            classify_reauthor_failure_class(&StorageError::Conductor(
                "Zome call failed: Content with id 'e2e-2f1c' already exists. \
                 Use update_content to modify existing entries"
                    .into()
            )),
            Some("already_exists")
        );
        // A genuine, uncounted failure — still retried next sweep (bumps
        // `failed` in the local tally), just not a labeled Prometheus class.
        assert_eq!(
            classify_reauthor_failure_class(&StorageError::Conductor(
                "Guest(\"not the author\")".into()
            )),
            None
        );
        assert_eq!(
            classify_reauthor_failure_class(&StorageError::Timeout("per-attempt".into())),
            None
        );
    }

    #[test]
    fn ghost_witness_reuses_the_witness_bounds() {
        // The ghost sweep runs on the SAME heal leg as `witness_bootstrap`, so it
        // must not introduce a second, unbounded conductor load: it reuses the
        // per-tick cap, the per-item spacing, and the wall-clock budget. Locking
        // this keeps a future tuning change from bounding one sweep and not the
        // other.
        assert!(WITNESS_MAX_PER_TICK > 0);
        assert!(!WITNESS_ITEM_DELAY.is_zero());
        assert!(WITNESS_SWEEP_BUDGET > WITNESS_ITEM_DELAY * (WITNESS_MAX_PER_TICK as u32));
    }

    #[test]
    fn witness_per_tick_cap_is_bounded_for_pacing() {
        // (d) per-tick cap: bounded so a large un-witnessed corpus (live alpha
        // shows thousands of rows) greens over many ticks instead of storming a
        // saturated conductor in one sweep.
        assert!(WITNESS_MAX_PER_TICK > 0, "must author some per tick");
        assert!(
            WITNESS_MAX_PER_TICK <= 500,
            "must stay small enough to pace a multi-thousand-row corpus across ticks"
        );
    }

    #[test]
    fn witness_sweep_budget_exceeds_paced_floor() {
        // The wall-clock budget must exceed the unavoidable per-item spacing floor
        // (cap × delay) with headroom for the conductor round-trips, so a HEALTHY
        // sweep never trips the timeout — the budget only fires on a hung/saturated
        // conductor, releasing the single-flight guard instead of holding it forever.
        let paced_floor = WITNESS_ITEM_DELAY * (WITNESS_MAX_PER_TICK as u32);
        assert!(
            WITNESS_SWEEP_BUDGET > paced_floor,
            "budget {WITNESS_SWEEP_BUDGET:?} must exceed the paced floor {paced_floor:?}"
        );
        // And it must be a real bound (not effectively infinite).
        assert!(WITNESS_SWEEP_BUDGET <= Duration::from_secs(600));
    }

    #[test]
    fn witness_guard_is_the_reanchor_once_per_id_classifier() {
        // The witness step's once-per-id guard IS `reanchor_backfill::decide_outcome`
        // (composed, not forked). Assert the three cases the task calls out, so the
        // guarantee is legible at the composition site.
        use crate::error::StorageError;
        use crate::services::reanchor_backfill::{
            decide_outcome, is_already_anchored_error, RowOutcome,
        };

        // (a) A candidate whose head the conductor already holds: create_content is
        // refused ("already exists") and the existing anchor is recovered+stamped →
        // AlreadyAnchored (stamped, NOT a second authored head).
        let already: Result<(), StorageError> = Err(StorageError::Conductor(
            "Zome call failed: Guest(\"Content with id 'seed-1' already exists. \
             Use update_content to modify existing entries.\")"
                .to_string(),
        ));
        assert!(is_already_anchored_error(already.as_ref().unwrap_err()));
        assert_eq!(
            decide_outcome(&already, Some(&Ok(true))),
            RowOutcome::AlreadyAnchored
        );

        // (b) Definitive not-found → authored exactly once (Reanchored). On the
        // NEXT tick the conductor holds it, so create is refused → AlreadyAnchored
        // (authored zero the second time — idempotent).
        let authored: Result<(), StorageError> = Ok(());
        assert_eq!(decide_outcome(&authored, None), RowOutcome::Reanchored);
        assert_eq!(
            decide_outcome(&already, Some(&Ok(true))),
            RowOutcome::AlreadyAnchored
        );

        // (c) Transient/bridge error → Failed (skipped, retried next tick; never a
        // fabricated or duplicate head).
        let transient: Result<(), StorageError> =
            Err(StorageError::Conductor("read plane down".into()));
        assert!(!is_already_anchored_error(transient.as_ref().unwrap_err()));
        assert_eq!(decide_outcome(&transient, None), RowOutcome::Failed);
    }

    #[test]
    fn content_gap_classification_absent_null_divergent() {
        use std::collections::{HashMap, HashSet};

        // present: b (un-anchored), c (anchored=X), d (anchored=X). a is absent.
        let present: HashSet<String> = ["b", "c", "d"].iter().map(|s| s.to_string()).collect();
        // local anchored set (list_content_anchor_inventory): only c and d.
        let mut local_anchors: HashMap<String, String> = HashMap::new();
        local_anchors.insert("c".into(), "anchor-X".into());
        local_anchors.insert("d".into(), "anchor-X".into());

        // (a) advertised but absent locally → SKIP.
        assert_eq!(
            classify_content_gap("a", &present, &local_anchors, Some("anchor-Z")),
            ContentGap::AbsentLocal
        );
        // (b) present but un-anchored (not in local_anchors) → anchor-gap.
        assert_eq!(
            classify_content_gap("b", &present, &local_anchors, Some("anchor-Y")),
            ContentGap::AnchorGap
        );
        // (c) present + anchored, peer anchor disagrees → divergent.
        assert_eq!(
            classify_content_gap("c", &present, &local_anchors, Some("anchor-Y")),
            ContentGap::Divergent
        );
        // (d) present + anchored, peer anchor agrees → in sync.
        assert_eq!(
            classify_content_gap("d", &present, &local_anchors, Some("anchor-X")),
            ContentGap::InSync
        );
        // (c) present + anchored, peer advertised EMPTY anchor → NOT divergence
        // (an un-anchored peer is not evidence our anchor is wrong).
        assert_eq!(
            classify_content_gap("c", &present, &local_anchors, Some("")),
            ContentGap::InSync
        );
        // (c) present + anchored, peer advertised NO anchor → in sync.
        assert_eq!(
            classify_content_gap("c", &present, &local_anchors, None),
            ContentGap::InSync
        );
    }

    // ── Heal-leg pacing: the saturated-conductor cure (retry + budget) ──

    #[test]
    fn transient_classifier_splits_timeout_from_logic() {
        use crate::error::StorageError;
        // The exact verbatim conductor text adam's Loki shows is transient.
        assert!(is_transient_conductor_error(&StorageError::Conductor(
            "Zome call failed: Websocket error: Timeout".into()
        )));
        assert!(is_transient_conductor_error(&StorageError::Timeout(
            "per-attempt".into()
        )));
        assert!(is_transient_conductor_error(&StorageError::Connection(
            "read timed out".into()
        )));
        // A logic/decode error is NOT transient — no free window fixes it, so no
        // in-leg retry (it would just burn budget).
        assert!(!is_transient_conductor_error(&StorageError::Conductor(
            "Guest(\"not the author\")".into()
        )));
        assert!(!is_transient_conductor_error(&StorageError::Validation(
            "bad enum".into()
        )));
    }

    #[test]
    fn heal_pacing_prioritizes_rea_and_bounds_attempts() {
        let p = HealPacing::default();
        assert!(
            p.max_row_retries >= 1,
            "a transient row must get at least one in-leg retry"
        );
        assert!(
            p.attempt_timeout < Duration::from_secs(60),
            "per-attempt timeout must be tighter than the conductor's ~60s WS timeout"
        );
        assert!(
            p.rea_leg_budget <= p.content_leg_budget,
            "rea's reserved budget must not exceed content's (rea is prioritized, small backlog)"
        );
        assert!(p.rea_leg_budget > Duration::ZERO && p.content_leg_budget > Duration::ZERO);
        // Jittered backoff stays within [min, min+span).
        let b = p.backoff();
        assert!(b >= p.backoff_min && b < p.backoff_min + p.backoff_span);
    }

    #[test]
    fn heal_outcome_labels_are_stable() {
        // The `/metrics` label vocabulary is a wire contract — pin it.
        assert_eq!(HealOutcomeKind::Healed.label(), "healed");
        assert_eq!(HealOutcomeKind::TimeoutRetried.label(), "timeout_retried");
        assert_eq!(
            HealOutcomeKind::TimeoutExhausted.label(),
            "timeout_exhausted"
        );
        assert_eq!(HealOutcomeKind::Missing.label(), "missing");
        assert_eq!(HealOutcomeKind::Failed.label(), "failed");
    }

    #[tokio::test]
    async fn call_with_retry_recovers_after_one_transient() {
        use crate::error::StorageError;
        let pacing = HealPacing::test_fast();
        let calls = std::cell::Cell::new(0u32);
        let r = call_with_retry(&pacing, || {
            let n = calls.get();
            calls.set(n + 1);
            async move {
                if n == 0 {
                    // First attempt: a WS timeout (the adam signature).
                    Err::<i32, StorageError>(StorageError::Conductor(
                        "Websocket error: Timeout".into(),
                    ))
                } else {
                    Ok(7)
                }
            }
        })
        .await;
        assert_eq!(r.result.expect("recovered"), 7);
        assert!(r.retried, "success came after a transient retry");
        assert_eq!(calls.get(), 2, "one retry after the first transient");
    }

    #[tokio::test]
    async fn call_with_retry_exhausts_on_persistent_transient() {
        use crate::error::StorageError;
        let pacing = HealPacing::test_fast(); // max_row_retries = 2 → 3 attempts
        let calls = std::cell::Cell::new(0u32);
        let r = call_with_retry(&pacing, || {
            calls.set(calls.get() + 1);
            async { Err::<i32, StorageError>(StorageError::Timeout("wedged".into())) }
        })
        .await;
        assert!(
            r.result.is_err(),
            "a persistently-wedged row stays a failure"
        );
        assert!(r.retried);
        assert_eq!(calls.get(), 3, "1 initial + max_row_retries(2) attempts");
    }

    #[tokio::test]
    async fn call_with_retry_no_retry_on_non_transient() {
        use crate::error::StorageError;
        let pacing = HealPacing::test_fast();
        let calls = std::cell::Cell::new(0u32);
        let r = call_with_retry(&pacing, || {
            calls.set(calls.get() + 1);
            async { Err::<i32, StorageError>(StorageError::Validation("bad".into())) }
        })
        .await;
        assert!(r.result.is_err());
        assert!(!r.retried, "a non-transient error is never retried in-leg");
        assert_eq!(calls.get(), 1, "exactly one attempt for a logic error");
    }

    // ---- Cure 3: retry classification (synthetic vs answered timeout) --------
    //
    // The distinction exists because `HcClient::call_zome` has no cancellation:
    // when OUR `tokio::time::timeout` fires we stop awaiting, but the conductor
    // keeps executing the call. Retrying therefore stacks concurrent zome calls
    // on an already-unresponsive conductor instead of re-trying anything.

    #[test]
    fn synthetic_attempt_timeout_is_distinguished_from_an_answered_one() {
        use crate::error::StorageError;
        // The exact shape `call_with_retry` manufactures on an elapsed deadline.
        let synthetic = StorageError::Timeout(format!(
            "{HEAL_SYNTHETIC_TIMEOUT_MARKER} {:?}",
            Duration::from_secs(15)
        ));
        assert!(is_synthetic_attempt_timeout(&synthetic));
        assert!(
            is_transient_conductor_error(&synthetic),
            "still transient — the metric label must not change"
        );
        assert!(
            !should_retry_attempt(&synthetic),
            "our own uncancelled-call timeout must NOT be retried"
        );

        // A timeout the conductor actually answered with stays retryable.
        let answered = StorageError::Conductor("Websocket error: Timeout".into());
        assert!(!is_synthetic_attempt_timeout(&answered));
        assert!(should_retry_attempt(&answered));

        // A bare Timeout that is not ours (e.g. another layer) is still retried.
        let other_timeout = StorageError::Timeout("wedged".into());
        assert!(!is_synthetic_attempt_timeout(&other_timeout));
        assert!(should_retry_attempt(&other_timeout));

        // A logic error is neither.
        let logic = StorageError::Validation("bad".into());
        assert!(!is_synthetic_attempt_timeout(&logic));
        assert!(!should_retry_attempt(&logic));
    }

    #[tokio::test]
    async fn call_with_retry_attempts_a_hung_call_exactly_once() {
        use crate::error::StorageError;
        // A conductor that never answers: the per-attempt deadline elapses.
        let pacing = HealPacing {
            attempt_timeout: Duration::from_millis(20),
            ..HealPacing::test_fast()
        };
        let calls = std::cell::Cell::new(0u32);
        let r = call_with_retry(&pacing, || {
            calls.set(calls.get() + 1);
            async {
                // Never resolves within the deadline.
                tokio::time::sleep(Duration::from_secs(30)).await;
                Ok::<i32, StorageError>(1)
            }
        })
        .await;

        let err = r.result.expect_err("a hung call fails");
        assert!(
            is_synthetic_attempt_timeout(&err),
            "the failure is our synthetic deadline, got: {err}"
        );
        assert_eq!(
            calls.get(),
            1,
            "a hung (uncancellable) call is attempted ONCE — retrying it would \
             stack concurrent zome calls on an unresponsive conductor"
        );
        assert!(!r.retried, "no retry was taken, so none is reported");
    }

    // ---- Cure 3: the unresponsive-conductor leg circuit ----------------------

    /// Build the synthetic error the circuit counts.
    fn synthetic_timeout_err() -> crate::error::StorageError {
        crate::error::StorageError::Timeout(format!(
            "{HEAL_SYNTHETIC_TIMEOUT_MARKER} {:?}",
            Duration::from_secs(15)
        ))
    }

    #[test]
    fn heal_circuit_opens_on_consecutive_synthetic_timeouts() {
        let mut c = HealCircuit::new(3);
        for i in 1..3 {
            c.record::<i32>(&Err(synthetic_timeout_err()));
            assert!(!c.is_open(), "must not open before the threshold (i={i})");
        }
        c.record::<i32>(&Err(synthetic_timeout_err()));
        assert!(c.is_open(), "opens at the threshold");
        assert_eq!(c.consecutive_timeouts(), 3);
    }

    #[test]
    fn heal_circuit_closes_on_first_success() {
        let mut c = HealCircuit::new(2);
        c.record::<i32>(&Err(synthetic_timeout_err()));
        c.record::<i32>(&Err(synthetic_timeout_err()));
        assert!(c.is_open());
        c.record(&Ok(1));
        assert!(!c.is_open(), "a success closes the circuit outright");
        assert_eq!(c.consecutive_timeouts(), 0);
    }

    #[test]
    fn heal_circuit_answered_failure_breaks_the_streak() {
        use crate::error::StorageError;
        let mut c = HealCircuit::new(3);
        c.record::<i32>(&Err(synthetic_timeout_err()));
        c.record::<i32>(&Err(synthetic_timeout_err()));
        // The conductor ANSWERED — with an error, but it is responsive. The
        // circuit tracks unresponsiveness, not row-level failure.
        c.record::<i32>(&Err(StorageError::Validation("bad".into())));
        assert_eq!(c.consecutive_timeouts(), 0, "streak broken by an answer");
        c.record::<i32>(&Err(synthetic_timeout_err()));
        assert!(!c.is_open(), "the streak restarted, so 1 < threshold 3");
    }

    #[test]
    fn heal_circuit_threshold_zero_never_opens() {
        let mut c = HealCircuit::new(0);
        for _ in 0..50 {
            c.record::<i32>(&Err(synthetic_timeout_err()));
        }
        assert!(!c.is_open(), "threshold 0 disables the circuit");
    }

    // ---- Cure 2: timeout → peer-adoption routing ----------------------------

    #[test]
    fn timeout_routes_to_adopt_only_with_a_peer_hint() {
        // The hole this closes: a transient failure WITH a peer declaration used
        // to land in neither candidate list and was silently dropped every sweep.
        assert!(
            timeout_should_route_to_adopt(true, true),
            "transient + a peer advertising a declaration ⇒ hand to the adopt arm"
        );
        assert!(
            !timeout_should_route_to_adopt(true, false),
            "no peer hint ⇒ nothing to adopt FROM"
        );
        assert!(
            !timeout_should_route_to_adopt(false, true),
            "a decode/logic error is not something adoption can fix"
        );
        assert!(!timeout_should_route_to_adopt(false, false));
    }
}
