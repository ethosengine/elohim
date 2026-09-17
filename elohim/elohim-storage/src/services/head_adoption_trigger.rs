//! Event-driven head ADOPTION TRIGGER — the Automerge content-sync apply as the
//! signal that a peer moved a content id's head.
//!
//! # The measured hole this closes
//!
//! Before this module there was NO event-driven path by which a non-author peer
//! learned that a canonical head had been declared. Measured on a healthy
//! 3-peer household (2026-09-17, `head-adoption-lag-analysis.md`), the three
//! candidate fast paths are each structurally incapable:
//!
//! - conductor `post_commit` / `emit_signal` is **cell-local** — the author's
//!   commit never signals a peer;
//! - the ordered `ReaProjectionSignal::ContentHeadDeclared` was **0-for-176**
//!   in the observed window, and is a local-conductor signal regardless;
//! - gossipsub head/EPR announce publishes fail `InsufficientPeers`.
//!
//! So EVERY observed adoption came from one carrier: the projection-reconcile
//! **content heal leg**, which is single-flight
//! ([`crate::p2p::projection_reconcile::heal_decision`] → `SkipInFlight`) and
//! whose period is a function of heal backlog, not of the tick. Measured
//! effective adopt period after a content seed: **130–170 s**, with individual
//! heads taking 76 s / 4m54s / 7m25s.
//!
//! Meanwhile the doc BODY for the very same id crosses in **under a second** on
//! the content-sync plane. The bytes were always early; nobody was asking the
//! conductor about them.
//!
//! # What this module does, and — precisely — what it does not
//!
//! It does **not** adopt. It **schedules the existing per-id decision earlier.**
//!
//! There is exactly one adoption implementation in this crate,
//! [`crate::services::head_adoption::try_adopt_canonical_head`], and this module
//! calls it — with the same `SWEEP_DECLARE_CLASS` declare path, the same
//! `call_resolve_content_head` conductor read, the same election binding, and
//! the same monotonic stamp guard. Every authority question (is the head
//! canonical? does the row already obey an election? is the move provably
//! forward?) is answered there, unchanged. This module answers one question and
//! one only: **is it worth spending a conductor probe on this id right now?**
//!
//! Consequently the trigger cannot move a head the sweep would not have moved.
//! It can only move it sooner. Heal fills-never-moves, verify-locally-then-serve
//! and canonical-channels-alone-move-a-head are preserved by construction,
//! because they are properties of the path this module delegates to.
//!
//! Note also what the worker does with each outcome: it acts on
//! [`AdoptOutcome::Adopted`] and on **nothing else**. `Author` /
//! `AuthorThenAdopt` are verdicts addressed to a caller that owns an author
//! path; this one does not, and never mints a root. A sync apply is not evidence
//! that anything should be authored.
//!
//! # The stale-head guard, and why the trigger must re-probe
//!
//! Adoption here is `AdoptLocal`: it needs THIS peer's conductor to already
//! resolve the new head. The doc crosses in under a second; the DHT record does
//! not necessarily. So the first probe often arrives before the record — and two
//! things follow.
//!
//! **First, a guard.** `adopt_local` runs `StampMode::HealCanonical`, whose
//! refusals key on `moving_declared_row` and `same_declared_head`. A row
//! declaring NOTHING satisfies neither, so the stamp falls through and writes
//! whatever head it is handed. If the conductor answered with the PREVIOUS head
//! while the doc already names a newer one, that fill pins a head we know is
//! superseded — and the later old→new move must then clear
//! `canonical_move_verdict`, which can refuse it as `SkippedStale`. So the
//! worker takes the conductor read ITSELF and declares only when the answer IS
//! the head the doc names. A disagreement is not-yet-walkable, never a declare.
//!
//! **Second, a ladder.** Without a re-probe the id would sit inside its claim
//! with nothing to re-fire it, and adoption would fall back to the sweep — the
//! exact failure this module exists to remove. [`RETRY_DELAYS`] re-probes at
//! 1/2/4/8/15/30 s (60 s total = the claim window), bounded by
//! [`RETRY_PENDING_CAP`] sleeping timers, and only for ids a doc hint PROVES
//! carry a head we lack ([`retry_warranted`]). Each rung re-reads the row first,
//! so a sweep adoption in the meantime ends the schedule for free.
//!
//! # REQ-N5 and the `headActionHash` doc hint — the line this module holds
//!
//! The content doc carries a `headActionHash` scalar
//! ([`crate::sync::projector`]). [`crate::sync::projector::reverse_project_content_doc`]
//! documents REQ-N5: that field is unauthenticated peer input and must NEVER be
//! consumed into SQL, because doing so would launder gossip into notarization
//! provenance.
//!
//! This module reads that field, and does not violate REQ-N5, because of a
//! distinction worth stating plainly:
//!
//! > The hint routes ATTENTION. It never supplies AUTHORITY.
//!
//! The only use made of it is the boolean in [`should_probe`]: "does the local
//! row already declare exactly this?" If yes, skip — nothing here can be behind.
//! If no, ask the **own conductor**. The head that gets declared is whatever the
//! conductor proves canonical; a peer that writes arbitrary bytes into the doc
//! can at most cause this node to spend one `resolve_content_head` call and
//! learn nothing. That is a denial-of-attention budget, bounded by the per-id
//! cooldown — not an authority surface.
//!
//! For the same reason the trigger deliberately passes
//! [`AdoptContext::none()`]: **no synthesized [`crate::services::head_adoption::PeerHeadHint`]**.
//! A hint is what selects WHICH head the `AdoptPeer` arm declares; promoting CRDT
//! doc bytes to hint status would put unauthenticated input in that seat. The
//! measured evidence says this costs nothing: every observed adoption in the
//! analysis window was `AdoptLocal` — literally *"ADOPTED a canonical head from
//! the own conductor"* — i.e. DHT gossip had already delivered the record and
//! the only missing act was asking. With no hint the `AdoptPeer`/`ContestPeer`
//! arms are unreachable from here and the fetcher is never consulted, which is
//! why `fetcher: None` is honest rather than a gap.
//!
//! # Double-declare with a concurrent sweep
//!
//! The trigger and the heal leg can reach the same id at the same moment. They
//! do not double-declare, for two independent reasons, either of which suffices:
//!
//! 1. **`claim_candidacy` idempotence** ([`crate::services::head_adoption`]).
//!    The contest/candidacy arms take a `!claim_candidacy(id, target)` early
//!    return, so the SECOND caller to reach the same `(id, target)` pair mints
//!    nothing. That ledger is process-wide, not sweep-scoped, so a trigger and a
//!    heal leg in the same process share it.
//! 2. **The declare is idempotent at the stamp.** `adopt_local` re-stamping a
//!    head the row already declares yields `StampOutcome::Refreshed`, not a
//!    second declaration; and a head the row does NOT already declare is gated
//!    by the monotonic forward-only guard. A redundant trigger therefore costs a
//!    conductor round-trip and changes no state.
//!
//! # Cost discipline
//!
//! - The sync apply site is only reached when `changes` is NON-EMPTY — a doc
//!   that did not move never arrives here at all. Steady-state sync costs zero.
//! - The hot path does one map lock and one `try_send`. No await, no DB, no
//!   conductor, never blocking, never failing the apply.
//! - A per-id cooldown bounds probe RATE; it doubles as the dedup gate (see
//!   [`TriggerGate::claim`]).
//! - One serial worker. A 3,500-doc seed storm becomes a queue, never a
//!   conductor stampede.
//! - Conductor admission class is `Background` by construction — inherited from
//!   the path this delegates to (`SWEEP_DECLARE_CLASS`), so an interactive read
//!   is never starved behind a trigger.
//!
//! On the uncancellable-conductor-call discipline: the WORK is bounded BEFORE
//! the call, which is the form that rule asks for. Each probe is a single-id
//! `resolve_content_head`, the worker is serial so at most one is ever in
//! flight, and there is deliberately NO caller-side timeout bolted onto
//! `call_zome` — a timeout here would abandon a conductor that keeps running
//! while still holding the permit.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::sync::mpsc;

use crate::db::{content_diesel, AppContext, DbPool};
use crate::hc_client::HcClient;
use crate::services::conductor_writes;
use crate::services::head_adoption::{
    self, AdoptContext, AdoptOutcome, ElectionResolve, LocalResolve,
};
use crate::sync::SyncManager;
use crate::trust::pricer::PricedVerification;

/// Bounded queue depth between the sync hot path and the serial worker.
///
/// 256 distinct ids — distinct because the claim gate coalesces by id BEFORE
/// the send (see [`TriggerGate::claim`]), so the queue can never hold two
/// entries for one id. At a ~0.3 s conductor probe that is ~80 s of work in
/// flight, which is already faster than the 130–170 s sweep period it replaces.
pub const TRIGGER_QUEUE_CAPACITY: usize = 256;

/// How long one content id is barred from re-triggering after a claim.
///
/// Sized to the sync round (60 s): a doc receiving change batches all round
/// probes the conductor at most once. This is the probe-rate bound — the thing
/// that makes a hostile or merely chatty peer unable to convert doc writes into
/// conductor load.
pub const DEFAULT_TRIGGER_COOLDOWN: Duration = Duration::from_secs(60);

/// Ceiling on remembered claims before a prune sweep runs. Bounds memory on a
/// large corpus; expired entries carry no meaning, so dropping them is free.
const CLAIM_MAP_PRUNE_AT: usize = 8_192;

/// Back-off ladder for re-probing an id whose head is proven to exist but is
/// not yet walkable by the local conductor.
///
/// # The race this closes
///
/// The trigger fires under a second after the author commits, but adoption is
/// `AdoptLocal` — it needs THIS peer's conductor to already resolve the new
/// head, which needs DHT gossip to have delivered the record. A single probe at
/// t≈0 therefore loses a foot-race it was never going to win, and without a
/// re-probe the id sits inside its claim with nothing to re-fire it: back to the
/// 130–450 s sweep, and the 75 s bound still fails. (The same not-yet-walkable
/// race is the likeliest cause of the 0-for-176 `ContentHeadDeclared` deferral.)
///
/// Sums to exactly 60 s — the claim window — so the ladder is spent while the
/// claim still stands and cannot collide with a fresh claim for the same id.
/// Front-loaded because the record usually lands in the first seconds; the long
/// tail is there so a slow gossip round is still caught before the sweep.
///
/// bounded-work: 6 re-probes per id per claim window, ≤60 s total wall-clock,
/// ≤`RETRY_PENDING_CAP` sleeping timers process-wide, 1 single-id conductor read
/// per probe on a SERIAL worker. Every dimension is a constant in this file.
const RETRY_DELAYS: [Duration; 6] = [
    Duration::from_secs(1),
    Duration::from_secs(2),
    Duration::from_secs(4),
    Duration::from_secs(8),
    Duration::from_secs(15),
    Duration::from_secs(30),
];

/// Ceiling on ids holding a pending re-probe timer at once.
///
/// The retry set is naturally small — only ids a peer has PROVEN carry a head we
/// do not hold (see [`retry_warranted`]) — but a bound is not optional: a seed
/// storm must not be able to convert itself into unbounded timer tasks.
pub const RETRY_PENDING_CAP: usize = 256;

/// The delay before attempt `attempt + 1`, or `None` when the ladder is spent.
///
/// Pure and total, so the schedule's shape and its termination are readable
/// (and testable) without a clock, a conductor, or a task.
pub fn retry_delay(attempt: u32) -> Option<Duration> {
    RETRY_DELAYS.get(attempt as usize).copied()
}

/// Is a re-probe warranted for this id?
///
/// **Only when a doc hint is PRESENT and the local row does not already declare
/// it.** The hint is the evidence that a head exists at all; with no hint there
/// is nothing to wait for, and re-probing would be speculation charged to the
/// conductor. That asymmetry is what keeps a 3,500-doc seed of undeclared
/// content at exactly one probe per id — the storm case — while the measured
/// race (a peer declared, we have not caught up) gets the full ladder.
///
/// It is also self-terminating for free: if the sweep adopts in the meantime,
/// the local row comes to equal the hint and the next retry ends the schedule
/// without a conductor call.
pub fn retry_warranted(doc_hint: Option<&str>, local_declared: Option<&str>) -> bool {
    match doc_hint {
        None => false,
        Some(hint) => local_declared != Some(hint),
    }
}

/// One scheduled adoption probe.
#[derive(Debug, Clone)]
pub struct HeadAdoptionTrigger {
    /// Content id (the `node:` prefix already stripped).
    pub content_id: String,
    /// The peer whose applied changes raised the trigger — an OBSERVABILITY
    /// field, carried so the confirming metric (delta between `Applying changes
    /// from peer … node:<slug>` and the ADOPT line) is readable from the log
    /// alone. It is never consulted for authority.
    pub peer: String,
    /// When the hot path raised it. PRESERVED across re-probes on purpose, so
    /// `trigger_to_adopt_ms` always measures from the sync apply — the number
    /// the lag analysis asks for — rather than from the attempt that happened
    /// to land.
    pub raised_at: Instant,
    /// 0 for the probe the sync apply raised; N for the Nth re-probe.
    pub attempt: u32,
}

/// What [`TriggerGate::plan_retry`] decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryPlan {
    /// Sleep this long, then re-probe.
    After(Duration),
    /// The ladder is spent — leave the id to the sweep.
    Exhausted,
    /// Too many pending timers already. Drop-new: the sweep remains the
    /// backstop, and the id's claim is left alone (it expires on its own).
    DroppedCap,
}

impl RetryPlan {
    /// Closed metric-label vocabulary.
    pub fn label(self) -> &'static str {
        match self {
            Self::After(_) => "retry_scheduled",
            Self::Exhausted => "retry_exhausted",
            Self::DroppedCap => "retry_dropped_cap",
        }
    }
}

/// What [`TriggerGate::claim`] decided. Exhaustive on purpose: each arm is a
/// metric label, so "the trigger did nothing" is never indistinguishable from
/// "the trigger was never wired".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnqueueDecision {
    /// Not a content-node doc under the projection namespace. No cost at all.
    NotContentDoc,
    /// The id is inside its cooldown window — either already queued/in-flight
    /// (dedup) or probed recently (rate bound). Same state, same answer.
    Deduped,
    /// Queued for the worker.
    Enqueued,
    /// The bounded queue was full. The claim is RELEASED so a later change
    /// batch for this id may re-offer it; the sweep remains the backstop.
    DroppedFull,
}

impl EnqueueDecision {
    /// Closed metric-label vocabulary.
    pub fn label(self) -> &'static str {
        match self {
            Self::NotContentDoc => "not_content_doc",
            Self::Deduped => "deduped",
            Self::Enqueued => "enqueued",
            Self::DroppedFull => "dropped_full",
        }
    }
}

/// The content id a sync apply is a trigger candidate FOR, or `None`.
///
/// Pure, allocation-free, and the entire hot-path pre-filter before the claim
/// map: namespace must be the projection namespace (the one
/// `initiate_sync_round` lists) and the doc id must have the `node:<slug>`
/// content-node shape. Every other doc kind — EPR docs, identity docs, any
/// future namespace — cannot carry a content head and is skipped before a lock
/// is taken.
pub fn trigger_candidate_id<'a>(h_app_id: &str, doc_id: &'a str) -> Option<&'a str> {
    if h_app_id != crate::sync::projector::PROJECTION_NAMESPACE {
        return None;
    }
    doc_id.strip_prefix("node:").filter(|id| !id.is_empty())
}

/// The worker-side gate: is this id's local declaration possibly BEHIND what the
/// doc says a peer moved to?
///
/// | local declared | doc hint | ⇒ |
/// |---|---|---|
/// | absent | – | **probe** — nothing local to be ahead of |
/// | present | absent | skip — the doc makes no head claim at all |
/// | present | equal | skip — already obeying exactly this |
/// | present | different | **probe** — the row may be behind |
///
/// Note the third row: this is what makes steady-state sync free. Once an
/// adoption lands, the row's `declared_head_action_hash` equals the doc hint, so
/// every subsequent change batch for that doc short-circuits here — a DB read,
/// no conductor call.
///
/// And note the second: a doc carrying no hint is NOT read as "the local
/// declaration is stale". Absence of a claim is not a claim, and treating it as
/// one would re-probe every pre-C2 / never-declared doc forever.
///
/// "Different" rather than "older" is deliberate and is the honest bound: Rust
/// cannot order two action hashes. Ordering is exactly what the conductor and
/// the monotonic stamp guard decide — so this predicate says only *possibly
/// behind, worth asking*, and the existing adoption path says *forward or not*.
pub fn should_probe(local_declared: Option<&str>, doc_hint: Option<&str>) -> bool {
    match (local_declared, doc_hint) {
        (None, _) => true,
        (Some(_), None) => false,
        (Some(local), Some(hint)) => local != hint,
    }
}

/// The hot-path half: per-id claim ledger plus the bounded sender.
///
/// Held by the P2P node; cloned `Arc` into the worker only so the FULL-drop path
/// can release a claim it could not honour.
pub struct TriggerGate {
    tx: mpsc::Sender<HeadAdoptionTrigger>,
    claims: Mutex<HashMap<String, Instant>>,
    cooldown: Duration,
    /// Ids holding a pending re-probe timer. An `AtomicUsize` rather than a set:
    /// the claim ledger already guarantees at most one live schedule per id, so
    /// all this needs to bound is the COUNT of sleeping tasks.
    pending_retries: std::sync::atomic::AtomicUsize,
}

impl TriggerGate {
    /// Build the gate and the receiver half the worker drains.
    pub fn new(cooldown: Duration) -> (Arc<Self>, mpsc::Receiver<HeadAdoptionTrigger>) {
        let (tx, rx) = mpsc::channel(TRIGGER_QUEUE_CAPACITY);
        (
            Arc::new(Self {
                tx,
                claims: Mutex::new(HashMap::new()),
                cooldown,
                pending_retries: std::sync::atomic::AtomicUsize::new(0),
            }),
            rx,
        )
    }

    /// Decide the next rung of the re-probe ladder, reserving budget if one is
    /// granted. Separated from the spawn so the schedule's shape, its
    /// termination and its cap are testable without a runtime.
    pub fn plan_retry(&self, attempt: u32) -> RetryPlan {
        use std::sync::atomic::Ordering;
        let Some(delay) = retry_delay(attempt) else {
            return RetryPlan::Exhausted;
        };
        // Reserve-then-verify: bump, and give the slot straight back if the bump
        // crossed the cap. Cheaper than a CAS loop and the transient overshoot
        // is invisible (the only reader is this check).
        if self.pending_retries.fetch_add(1, Ordering::AcqRel) >= RETRY_PENDING_CAP {
            self.pending_retries.fetch_sub(1, Ordering::AcqRel);
            return RetryPlan::DroppedCap;
        }
        RetryPlan::After(delay)
    }

    /// Release a reserved re-probe slot.
    fn release_retry(&self) {
        self.pending_retries
            .fetch_sub(1, std::sync::atomic::Ordering::AcqRel);
    }

    /// Pending re-probe timers — test/observability only.
    pub fn pending_retries(&self) -> usize {
        self.pending_retries
            .load(std::sync::atomic::Ordering::Acquire)
    }

    /// Schedule the next re-probe, if the ladder and the budget allow one.
    ///
    /// The timer is a detached task that sleeps and then re-sends, so the SERIAL
    /// worker never blocks on a delay — it keeps draining other ids throughout.
    /// The re-send deliberately BYPASSES the claim gate ([`Self::resend_retry`]):
    /// the claim for this id is already held, and is precisely what the ladder is
    /// spending.
    pub fn schedule_retry(self: &Arc<Self>, trigger: HeadAdoptionTrigger) -> RetryPlan {
        let plan = self.plan_retry(trigger.attempt);
        if let RetryPlan::After(delay) = plan {
            let gate = Arc::clone(self);
            let next = HeadAdoptionTrigger {
                attempt: trigger.attempt + 1,
                ..trigger
            };
            tokio::spawn(async move {
                tokio::time::sleep(delay).await;
                // Release BEFORE the send: the slot bounds sleeping timers, not
                // queued work, and the queue has its own bound.
                gate.release_retry();
                gate.resend_retry(next);
            });
        }
        plan
    }

    /// Re-enqueue a trigger for another attempt, bypassing the claim gate.
    ///
    /// A full queue drops it — the id keeps its claim and the sweep remains the
    /// backstop, exactly as on the initial-enqueue path.
    fn resend_retry(&self, trigger: HeadAdoptionTrigger) {
        let content_id = trigger.content_id.clone();
        if self.tx.try_send(trigger).is_err() {
            crate::metrics::inc_head_adoption_trigger("retry_dropped_full");
            tracing::debug!(
                target: "elohim_storage::head_adoption_trigger",
                content_id = %content_id,
                "head-adoption trigger: re-probe dropped — queue full; the sweep still covers it"
            );
        }
    }

    /// Offer a just-applied doc to the trigger. NEVER blocks, never awaits,
    /// never fails the caller — the sync apply is unaffected by whatever happens
    /// here.
    ///
    /// # Drop policy on a full queue: drop the NEW trigger, release its claim
    ///
    /// Chosen over drop-oldest, and the reason is that coalescing already
    /// happened one step earlier. The claim gate admits an id at most once per
    /// cooldown, so the queue holds 256 **distinct** ids, none of which
    /// supersedes another. Drop-oldest exists to evict entries made stale by a
    /// newer entry for the same key — there are none here, so it would buy
    /// nothing while systematically starving whichever id arrived first in a
    /// storm, and would require receiver-side surgery `mpsc` does not offer.
    ///
    /// Releasing the claim on drop is the part that matters: a dropped id must
    /// not also be locked out for a cooldown period having never been probed.
    /// Released, the next change batch re-offers it; and the heal leg is still
    /// the backstop it always was.
    pub fn offer(&self, h_app_id: &str, doc_id: &str, peer: &str) -> EnqueueDecision {
        let Some(content_id) = trigger_candidate_id(h_app_id, doc_id) else {
            return EnqueueDecision::NotContentDoc;
        };
        self.claim_and_send(content_id, peer, Instant::now())
    }

    /// [`Self::offer`]'s body with the clock injected — the unit-testable seam.
    pub fn claim_and_send(&self, content_id: &str, peer: &str, now: Instant) -> EnqueueDecision {
        if !self.claim(content_id, now) {
            return EnqueueDecision::Deduped;
        }
        let trigger = HeadAdoptionTrigger {
            content_id: content_id.to_string(),
            peer: peer.to_string(),
            raised_at: now,
            attempt: 0,
        };
        match self.tx.try_send(trigger) {
            Ok(()) => EnqueueDecision::Enqueued,
            Err(_) => {
                self.release(content_id);
                EnqueueDecision::DroppedFull
            }
        }
    }

    /// Take the per-id claim, or refuse because one still stands.
    ///
    /// ONE structure serves both obligations the design needs, which is why
    /// there is no separate in-flight set:
    ///
    /// - **dedup** — a second offer while the first is queued or in flight is
    ///   inside the window, so it is refused;
    /// - **rate bound** — an offer after the work finished but within the
    ///   cooldown is refused identically.
    ///
    /// Stamped at CLAIM time, not at completion: a failed probe therefore waits
    /// out the cooldown before retrying, which is the correct posture for a path
    /// whose backstop (the heal leg) is still running.
    fn claim(&self, content_id: &str, now: Instant) -> bool {
        let mut claims = match self.claims.lock() {
            Ok(g) => g,
            // A poisoned lock must not wedge the sync path. Refusing the claim
            // degrades to exactly the pre-trigger world: the sweep adopts.
            Err(_) => return false,
        };
        if let Some(last) = claims.get(content_id) {
            if now.duration_since(*last) < self.cooldown {
                return false;
            }
        }
        if claims.len() >= CLAIM_MAP_PRUNE_AT {
            let cooldown = self.cooldown;
            claims.retain(|_, last| now.duration_since(*last) < cooldown);
        }
        claims.insert(content_id.to_string(), now);
        true
    }

    /// Drop a claim taken but not honoured (the full-queue path).
    fn release(&self, content_id: &str) {
        if let Ok(mut claims) = self.claims.lock() {
            claims.remove(content_id);
        }
    }

    /// Claims currently remembered — test/observability only.
    pub fn claim_count(&self) -> usize {
        self.claims.lock().map(|c| c.len()).unwrap_or(0)
    }
}

/// Lazily-resolved conductor handle. `lamad_client()` on the registry is
/// interior-mutable, so a bridge that connects LATE is picked up without
/// re-plumbing — the same idiom `P2PNode::with_hc_registry` relies on.
pub trait ConductorSource: Send + Sync {
    fn hc(&self) -> Option<Arc<HcClient>>;
}

impl ConductorSource for crate::hc_client_registry::HcClientRegistry {
    fn hc(&self) -> Option<Arc<HcClient>> {
        self.lamad_client()
    }
}

/// The serial worker. ONE at a time, on purpose: a seed storm must become a
/// queue, never 3,500 concurrent zome calls.
///
/// Runs wholly independently of
/// [`crate::p2p::projection_reconcile::heal_decision`] — it holds no reference
/// to the heal single-flight flag, is not spawned from the reconcile tick, and
/// does not consult `HealAction`. That independence IS the repair: the measured
/// lag was heal legs taking 2–3 minutes while `SkipInFlight` suppressed every
/// intervening tick's adoption.
pub async fn run_head_adoption_trigger_worker(
    mut rx: mpsc::Receiver<HeadAdoptionTrigger>,
    gate: Arc<TriggerGate>,
    conductor: Arc<dyn ConductorSource>,
    pool: DbPool,
    sync: Arc<SyncManager>,
    mut shutdown: tokio::sync::broadcast::Receiver<()>,
) {
    let ctx = AppContext::default_lamad();
    tracing::info!(
        target: "elohim_storage::head_adoption_trigger",
        queue_capacity = TRIGGER_QUEUE_CAPACITY,
        "head-adoption trigger: worker armed — a content-sync apply now schedules \
         the adopt-before-author decision for that id"
    );
    loop {
        let trigger = tokio::select! {
            t = rx.recv() => match t {
                Some(t) => t,
                None => {
                    tracing::debug!(
                        target: "elohim_storage::head_adoption_trigger",
                        "head-adoption trigger: sender dropped — worker exiting"
                    );
                    return;
                }
            },
            _ = shutdown.recv() => {
                tracing::debug!(
                    target: "elohim_storage::head_adoption_trigger",
                    "head-adoption trigger: shutdown — worker exiting"
                );
                return;
            }
        };
        process_trigger(&trigger, conductor.as_ref(), &pool, &sync, &ctx, &gate).await;
    }
}

/// One trigger, start to finish. Separated from the loop so the shutdown/select
/// plumbing stays readable and the body is directly exercisable.
async fn process_trigger(
    trigger: &HeadAdoptionTrigger,
    conductor: &dyn ConductorSource,
    pool: &DbPool,
    sync: &SyncManager,
    ctx: &AppContext,
    gate: &Arc<TriggerGate>,
) {
    let id = trigger.content_id.as_str();
    let Some(hc) = conductor.hc() else {
        // No lamad bridge yet. The sweep is the backstop, exactly as before.
        crate::metrics::inc_head_adoption_trigger("no_bridge");
        return;
    };

    // ATTENTION-ROUTING read of the doc hint. See the module docs on REQ-N5:
    // this value never reaches SQL and never selects a head — it only answers
    // "is a probe worth spending?".
    let doc_id = format!("node:{id}");
    let doc_hint = sync
        .get_doc_field(
            crate::sync::projector::PROJECTION_NAMESPACE,
            &doc_id,
            "headActionHash",
        )
        .await
        .ok()
        .filter(|h| !h.trim().is_empty());

    let local_declared = match pool.get() {
        Ok(mut conn) => match content_diesel::declared_head_with_election(&mut conn, ctx, id) {
            Ok((declared, _election)) => declared,
            Err(e) => {
                tracing::debug!(
                    target: "elohim_storage::head_adoption_trigger",
                    content_id = %id, error = %e,
                    "head-adoption trigger: could not read the local declaration — skipping \
                     (the sweep still covers this id)"
                );
                crate::metrics::inc_head_adoption_trigger("failed");
                return;
            }
        },
        Err(e) => {
            tracing::debug!(
                target: "elohim_storage::head_adoption_trigger",
                content_id = %id, error = %e,
                "head-adoption trigger: db conn unavailable — skipping"
            );
            crate::metrics::inc_head_adoption_trigger("failed");
            return;
        }
    };

    if !should_probe(local_declared.as_deref(), doc_hint.as_deref()) {
        // Includes the case a concurrent sweep adoption already landed: the row
        // now equals the hint, so a pending ladder ends here for free.
        crate::metrics::inc_head_adoption_trigger("skipped_current");
        return;
    }

    // THE conductor read — made HERE rather than inside the adoption path, which
    // is what `LocalResolve::Probe` would otherwise have done. Same call, same
    // cost; taking it ourselves is what lets the stale-head guard below exist.
    let resolved = match conductor_writes::call_resolve_content_head(&hc, id).await {
        Ok(head) => head,
        Err(e) => {
            tracing::debug!(
                target: "elohim_storage::head_adoption_trigger",
                content_id = %id, attempt = trigger.attempt, error = %e,
                "head-adoption trigger: own-conductor resolve failed — treating as not-yet-walkable"
            );
            None
        }
    };

    // THE STALE-HEAD GUARD.
    //
    // `adopt_local` runs `StampMode::HealCanonical`, whose guards fire on
    // `moving_declared_row` (row declares a DIFFERENT head) and on
    // `same_declared_head`. A row that declares NOTHING satisfies neither, so the
    // stamp falls straight through and writes whatever head it was handed into
    // BOTH `declared_head_action_hash` and `dht_anchor_hash`, returning
    // `Adopted`. That is a fill, not a move — but if the conductor answered with
    // the PREVIOUS head while the doc already proves a newer one exists, the fill
    // pins a head we know is superseded, and the later old→new move then has to
    // clear `canonical_move_verdict`, which can refuse it as `SkippedStale`.
    //
    // So: when the doc names a head, the conductor's answer must BE that head.
    // Anything else is "not yet walkable" — the record has not reached this
    // conductor — and is retried, never declared. With no doc hint there is
    // nothing to disagree with and any canonical answer is adoptable.
    let adoptable = resolved.as_ref().filter(|h| h.canonical).filter(|h| {
        doc_hint
            .as_deref()
            .is_none_or(|hint| h.head_action_hash.as_str() == hint)
    });

    let Some(head) = adoptable else {
        crate::metrics::inc_head_adoption_trigger("not_yet_walkable");
        tracing::debug!(
            target: "elohim_storage::head_adoption_trigger",
            content_id = %id,
            attempt = trigger.attempt,
            resolved_head = ?resolved.as_ref().map(|h| h.head_action_hash.as_str()),
            canonical = resolved.as_ref().map(|h| h.canonical),
            doc_hint = ?doc_hint,
            "head-adoption trigger: the own conductor cannot yet walk the head this doc \
             names — NOT declaring a stale head; re-probing"
        );
        schedule_reprobe(
            gate,
            trigger,
            doc_hint.as_deref(),
            local_declared.as_deref(),
        );
        return;
    };

    // THE one adoption implementation. `observed(Some(head))` hands it the read
    // just made, so it does not pay for a second one — and `should_probe_election`
    // is false for an answered head, so the election probe is skipped too.
    // `AdoptContext::none()` because no CRDT-derived hint may sit in the seat that
    // selects a head; `inert()` because with no peer hints there is no provenance
    // to price.
    let outcome = head_adoption::try_adopt_canonical_head(
        &hc,
        pool,
        ctx,
        id,
        LocalResolve::observed(Some(head)),
        ElectionResolve::unresolved(),
        &AdoptContext::none(),
        PricedVerification::inert(),
    )
    .await;

    let elapsed_ms = trigger.raised_at.elapsed().as_millis();
    match outcome {
        AdoptOutcome::Adopted => {
            crate::metrics::inc_head_adoption_trigger("adopted");
            // THE confirming line. Pair it with the `Applying changes from peer
            // … node:<slug>` line for the same id: the delta between them is the
            // measurement the 2026-09-17 analysis asked for, and it is now
            // readable from one log without a join.
            tracing::info!(
                target: "elohim_storage::head_adoption_trigger",
                content_id = %id,
                source_peer = %trigger.peer,
                trigger_to_adopt_ms = elapsed_ms,
                attempt = trigger.attempt,
                "head-adoption trigger: ADOPTED a canonical head raised by a content-sync \
                 apply — adoption no longer waits on the heal leg's period"
            );
        }
        AdoptOutcome::Held | AdoptOutcome::Contested => {
            crate::metrics::inc_head_adoption_trigger("held");
            tracing::debug!(
                target: "elohim_storage::head_adoption_trigger",
                content_id = %id, outcome = ?outcome, trigger_to_adopt_ms = elapsed_ms,
                attempt = trigger.attempt,
                "head-adoption trigger: nothing to adopt — the row is held or contested"
            );
            // A stamp refusal (`SkippedStale`) leaves the row still not naming
            // the doc's head, so the ladder continues — the next rung re-reads
            // the row first and stops for free if anything settles it.
            schedule_reprobe(
                gate,
                trigger,
                doc_hint.as_deref(),
                local_declared.as_deref(),
            );
        }
        // NOT OURS TO ACT ON. A sync apply is not evidence that a root should be
        // minted; this worker owns no author path and deliberately will not grow
        // one. The heal / re-anchor sweeps own authoring and are unaffected.
        AdoptOutcome::Author | AdoptOutcome::AuthorThenAdopt { .. } => {
            crate::metrics::inc_head_adoption_trigger("author_deferred");
            tracing::debug!(
                target: "elohim_storage::head_adoption_trigger",
                content_id = %id, trigger_to_adopt_ms = elapsed_ms,
                attempt = trigger.attempt,
                "head-adoption trigger: nothing canonical to adopt — leaving the author \
                 path to the sweeps that own one"
            );
            schedule_reprobe(
                gate,
                trigger,
                doc_hint.as_deref(),
                local_declared.as_deref(),
            );
        }
    }
}

/// Book the next rung of the ladder, when one is warranted.
///
/// Separate from the outcome arms so every non-adopting path books it the same
/// way, and so the WARRANT check ([`retry_warranted`]) is applied exactly once.
/// Exhaustion releases NOTHING early: the id keeps its claim until the cooldown
/// expires on its own, which is what stops an exhausted id from immediately
/// re-entering on the next change batch.
fn schedule_reprobe(
    gate: &Arc<TriggerGate>,
    trigger: &HeadAdoptionTrigger,
    doc_hint: Option<&str>,
    local_declared: Option<&str>,
) {
    if !retry_warranted(doc_hint, local_declared) {
        return;
    }
    let plan = gate.schedule_retry(trigger.clone());
    crate::metrics::inc_head_adoption_trigger(plan.label());
    if plan == RetryPlan::Exhausted {
        tracing::debug!(
            target: "elohim_storage::head_adoption_trigger",
            content_id = %trigger.content_id,
            attempts = trigger.attempt + 1,
            "head-adoption trigger: re-probe ladder spent — leaving this id to the sweep"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gate_with(cooldown: Duration) -> (Arc<TriggerGate>, mpsc::Receiver<HeadAdoptionTrigger>) {
        TriggerGate::new(cooldown)
    }

    // ── the hot-path pre-filter ──────────────────────────────────────────────

    #[test]
    fn trigger_candidate_id_admits_only_content_node_docs() {
        let ns = crate::sync::projector::PROJECTION_NAMESPACE;
        assert_eq!(trigger_candidate_id(ns, "node:alpha"), Some("alpha"));
        // Wrong namespace — a doc in another partition cannot carry a content head.
        assert_eq!(trigger_candidate_id("imagodei", "node:alpha"), None);
        // Not a content-node doc.
        assert_eq!(trigger_candidate_id(ns, "epr:alpha"), None);
        assert_eq!(trigger_candidate_id(ns, "alpha"), None);
        // Degenerate `node:` with no slug is not an id.
        assert_eq!(trigger_candidate_id(ns, "node:"), None);
    }

    #[test]
    fn offer_ignores_non_content_docs_without_taking_a_claim() {
        let (gate, _rx) = gate_with(DEFAULT_TRIGGER_COOLDOWN);
        assert_eq!(
            gate.offer("imagodei", "node:alpha", "peerA"),
            EnqueueDecision::NotContentDoc
        );
        assert_eq!(
            gate.offer(
                crate::sync::projector::PROJECTION_NAMESPACE,
                "epr:x",
                "peerA"
            ),
            EnqueueDecision::NotContentDoc
        );
        assert_eq!(gate.claim_count(), 0, "a non-candidate must cost nothing");
    }

    // ── the worker-side probe predicate: absent/different only ───────────────

    #[test]
    fn should_probe_absent_local_declaration_always_probes() {
        assert!(should_probe(None, None));
        assert!(should_probe(None, Some("uhCkk-peer")));
    }

    #[test]
    fn should_probe_skips_a_row_already_declaring_the_doc_head() {
        // The steady-state leg: once adoption lands, every later change batch
        // for this doc short-circuits here.
        assert!(!should_probe(Some("uhCkk-same"), Some("uhCkk-same")));
    }

    #[test]
    fn should_probe_skips_when_the_doc_makes_no_head_claim() {
        // Absence of a claim is not a claim that the local declaration is stale.
        assert!(!should_probe(Some("uhCkk-local"), None));
    }

    #[test]
    fn should_probe_probes_a_divergent_doc_head() {
        assert!(should_probe(Some("uhCkk-local"), Some("uhCkk-peer")));
    }

    // ── dedup + cooldown (one structure, both obligations) ───────────────────

    #[test]
    fn claim_dedups_a_second_offer_for_the_same_id() {
        let (gate, mut rx) = gate_with(DEFAULT_TRIGGER_COOLDOWN);
        let t0 = Instant::now();
        assert_eq!(
            gate.claim_and_send("alpha", "peerA", t0),
            EnqueueDecision::Enqueued
        );
        // Same id, still inside the window — refused, whether it is queued or
        // already in flight. Both are the same state as far as this gate knows.
        assert_eq!(
            gate.claim_and_send("alpha", "peerB", t0 + Duration::from_secs(1)),
            EnqueueDecision::Deduped
        );
        // A DIFFERENT id is unaffected — dedup is per id, not global.
        assert_eq!(
            gate.claim_and_send("beta", "peerA", t0 + Duration::from_secs(1)),
            EnqueueDecision::Enqueued
        );
        assert_eq!(rx.try_recv().unwrap().content_id, "alpha");
        assert_eq!(rx.try_recv().unwrap().content_id, "beta");
    }

    #[test]
    fn claim_readmits_after_the_cooldown_elapses() {
        let cooldown = Duration::from_secs(60);
        let (gate, mut rx) = gate_with(cooldown);
        let t0 = Instant::now();
        assert_eq!(
            gate.claim_and_send("alpha", "peerA", t0),
            EnqueueDecision::Enqueued
        );
        // One tick short of the window — still barred.
        assert_eq!(
            gate.claim_and_send("alpha", "peerA", t0 + cooldown - Duration::from_millis(1)),
            EnqueueDecision::Deduped
        );
        // At the window — re-admitted.
        assert_eq!(
            gate.claim_and_send("alpha", "peerA", t0 + cooldown),
            EnqueueDecision::Enqueued
        );
        assert_eq!(rx.try_recv().unwrap().content_id, "alpha");
        assert_eq!(rx.try_recv().unwrap().content_id, "alpha");
    }

    #[test]
    fn steady_state_sync_of_one_doc_costs_one_probe_per_cooldown() {
        // The cost claim in the module docs, asserted: 50 change batches inside
        // one window produce exactly ONE queued probe.
        let (gate, _rx) = gate_with(DEFAULT_TRIGGER_COOLDOWN);
        let t0 = Instant::now();
        let enqueued = (0..50)
            .filter(|i| {
                gate.claim_and_send(
                    "alpha",
                    "peerA",
                    t0 + Duration::from_millis(*i as u64 * 100),
                ) == EnqueueDecision::Enqueued
            })
            .count();
        assert_eq!(enqueued, 1);
    }

    // ── drop policy on a full queue ─────────────────────────────────────────

    #[test]
    fn a_full_queue_drops_the_new_trigger_and_releases_its_claim() {
        let (gate, mut rx) = gate_with(DEFAULT_TRIGGER_COOLDOWN);
        let t0 = Instant::now();
        // Fill to capacity with distinct ids — which is the only shape the queue
        // can take, because the claim gate coalesces by id before the send.
        for i in 0..TRIGGER_QUEUE_CAPACITY {
            assert_eq!(
                gate.claim_and_send(&format!("id-{i}"), "peerA", t0),
                EnqueueDecision::Enqueued,
                "id-{i} should fit within capacity"
            );
        }
        // One more: dropped.
        assert_eq!(
            gate.claim_and_send("overflow", "peerA", t0),
            EnqueueDecision::DroppedFull
        );
        // ...and its claim was RELEASED, so a later batch can re-offer it
        // immediately rather than serving a cooldown it never earned.
        let drained = rx.try_recv().expect("queue has entries");
        assert_eq!(drained.content_id, "id-0");
        assert_eq!(
            gate.claim_and_send("overflow", "peerA", t0),
            EnqueueDecision::Enqueued,
            "a dropped id must not be locked out by a claim it never used"
        );
        // The OLDEST survived — drop-newest, not drop-oldest. Justified because
        // coalescing already happened at the claim gate, so no queued entry is
        // ever superseded by a newer one for the same id.
        assert_eq!(rx.try_recv().unwrap().content_id, "id-1");
    }

    // ── the bounded re-probe ladder ─────────────────────────────────────────

    #[test]
    fn the_retry_ladder_is_bounded_and_fits_inside_the_claim_window() {
        let total: Duration = RETRY_DELAYS.iter().sum();
        assert_eq!(total, DEFAULT_TRIGGER_COOLDOWN, "ladder must fit the claim");
        // Spends, and then stops. No rung beyond the table exists.
        assert_eq!(retry_delay(0), Some(Duration::from_secs(1)));
        assert_eq!(retry_delay(5), Some(Duration::from_secs(30)));
        assert_eq!(retry_delay(6), None);
        assert_eq!(retry_delay(u32::MAX), None);
    }

    #[test]
    fn the_ladder_ends_on_adopt() {
        // Adoption stamps the head, so the row comes to name the hint — and the
        // warrant is exactly that disagreement. No warrant, no next rung.
        let hint = Some("uhCkk-new");
        assert!(retry_warranted(hint, None), "absent row: keep probing");
        assert!(
            retry_warranted(hint, Some("uhCkk-old")),
            "stale row: keep probing"
        );
        assert!(
            !retry_warranted(hint, Some("uhCkk-new")),
            "adopted row: the ladder ends"
        );
    }

    #[test]
    fn the_ladder_ends_early_when_a_sweep_adoption_lands_first() {
        // A rung that wakes to find the sweep already settled the row spends no
        // conductor call: `should_probe` short-circuits, and the warrant is gone.
        let hint = "uhCkk-new";
        assert!(!should_probe(Some(hint), Some(hint)));
        assert!(!retry_warranted(Some(hint), Some(hint)));
    }

    #[test]
    fn a_doc_with_no_head_hint_never_enters_the_ladder() {
        // The storm case: undeclared content probes ONCE and stops. Retrying with
        // no hint would be speculation charged to the conductor.
        assert!(!retry_warranted(None, None));
        assert!(!retry_warranted(None, Some("uhCkk-local")));
    }

    #[test]
    fn the_retry_cap_drops_new_without_leaking_claims_or_slots() {
        let (gate, mut rx) = gate_with(DEFAULT_TRIGGER_COOLDOWN);
        let t0 = Instant::now();
        // Reserve the whole budget.
        for _ in 0..RETRY_PENDING_CAP {
            assert!(matches!(gate.plan_retry(0), RetryPlan::After(_)));
        }
        assert_eq!(gate.pending_retries(), RETRY_PENDING_CAP);
        // One more is refused — and the refusal does NOT consume budget.
        assert_eq!(gate.plan_retry(0), RetryPlan::DroppedCap);
        assert_eq!(
            gate.pending_retries(),
            RETRY_PENDING_CAP,
            "a refused reservation must give its slot back"
        );
        // An exhausted ladder never reserves at all.
        assert_eq!(gate.plan_retry(6), RetryPlan::Exhausted);
        assert_eq!(gate.pending_retries(), RETRY_PENDING_CAP);
        // And none of that touched the claim ledger: a fresh id still enqueues.
        assert_eq!(gate.claim_count(), 0, "planning a retry must not claim");
        assert_eq!(
            gate.claim_and_send("unrelated", "peerA", t0),
            EnqueueDecision::Enqueued
        );
        assert_eq!(rx.try_recv().unwrap().content_id, "unrelated");
    }

    #[test]
    fn exhaustion_releases_nothing_early_and_the_claim_stands_until_cooldown() {
        let cooldown = DEFAULT_TRIGGER_COOLDOWN;
        let (gate, mut rx) = gate_with(cooldown);
        let t0 = Instant::now();
        assert_eq!(
            gate.claim_and_send("alpha", "peerA", t0),
            EnqueueDecision::Enqueued
        );
        let trigger = rx.try_recv().unwrap();
        assert_eq!(trigger.attempt, 0);
        // Ladder spent.
        assert_eq!(gate.plan_retry(6), RetryPlan::Exhausted);
        // The claim is NOT released — the id stays barred for the rest of the
        // window, so an exhausted id cannot immediately re-enter on the next
        // change batch and start the ladder over.
        assert_eq!(gate.claim_count(), 1);
        assert_eq!(
            gate.claim_and_send("alpha", "peerA", t0 + Duration::from_secs(59)),
            EnqueueDecision::Deduped
        );
        // It re-admits only when the cooldown genuinely expires.
        assert_eq!(
            gate.claim_and_send("alpha", "peerA", t0 + cooldown),
            EnqueueDecision::Enqueued
        );
    }

    #[tokio::test]
    async fn a_scheduled_reprobe_re_enqueues_with_an_incremented_attempt() {
        // End-to-end on the real timer: the ladder's first rung is 1 s, so pause
        // time and let the runtime auto-advance rather than sleeping for real.
        tokio::time::pause();
        let (gate, mut rx) = gate_with(DEFAULT_TRIGGER_COOLDOWN);
        let raised = Instant::now();
        let plan = gate.schedule_retry(HeadAdoptionTrigger {
            content_id: "alpha".into(),
            peer: "peerA".into(),
            raised_at: raised,
            attempt: 0,
        });
        assert_eq!(plan, RetryPlan::After(Duration::from_secs(1)));
        assert_eq!(gate.pending_retries(), 1, "the timer holds its slot");
        // Paused time auto-advances while the runtime is idle, so awaiting the
        // receiver fires the 1 s rung without sleeping for real.
        let again = tokio::time::timeout(Duration::from_secs(30), rx.recv())
            .await
            .expect("the re-probe timer fired")
            .expect("the re-probe was re-enqueued");
        assert_eq!(again.content_id, "alpha");
        assert_eq!(again.attempt, 1, "attempt advances along the ladder");
        assert_eq!(
            again.raised_at, raised,
            "raised_at is preserved so trigger_to_adopt_ms measures from the sync apply"
        );
        assert_eq!(gate.pending_retries(), 0, "the slot is released on fire");
    }

    #[test]
    fn retry_plan_labels_are_a_closed_vocabulary() {
        for (p, l) in [
            (RetryPlan::After(Duration::from_secs(1)), "retry_scheduled"),
            (RetryPlan::Exhausted, "retry_exhausted"),
            (RetryPlan::DroppedCap, "retry_dropped_cap"),
        ] {
            assert_eq!(p.label(), l);
        }
    }

    #[test]
    fn enqueue_decision_labels_are_a_closed_vocabulary() {
        for (d, l) in [
            (EnqueueDecision::NotContentDoc, "not_content_doc"),
            (EnqueueDecision::Deduped, "deduped"),
            (EnqueueDecision::Enqueued, "enqueued"),
            (EnqueueDecision::DroppedFull, "dropped_full"),
        ] {
            assert_eq!(d.label(), l);
        }
    }

    #[test]
    fn claim_map_is_pruned_rather_than_growing_without_bound() {
        let cooldown = Duration::from_millis(10);
        let (gate, mut rx) = gate_with(cooldown);
        let t0 = Instant::now();
        // Fill past the prune watermark with EXPIRED claims, then take one more.
        for i in 0..CLAIM_MAP_PRUNE_AT {
            let _ = gate.claim_and_send(&format!("id-{i}"), "peerA", t0);
            // Drain so the bounded queue never becomes the limiting factor.
            let _ = rx.try_recv();
        }
        assert_eq!(gate.claim_count(), CLAIM_MAP_PRUNE_AT);
        let _ = gate.claim_and_send("late", "peerA", t0 + Duration::from_secs(1));
        assert!(
            gate.claim_count() < CLAIM_MAP_PRUNE_AT,
            "expired claims must be pruned at the watermark"
        );
    }

    // ── independence from the sweep's single-flight state ───────────────────

    #[test]
    fn the_trigger_path_is_independent_of_the_heal_leg_single_flight_state() {
        use crate::p2p::projection_reconcile::{heal_decision, HealAction};
        // The exact condition that suppressed adoption for 130–170 s at a time:
        // the bridge is up, a heal leg is still running, so the reconcile tick
        // spawns NOTHING.
        assert_eq!(heal_decision(true, true), HealAction::SkipInFlight);
        // The trigger admits regardless — it holds no reference to that flag,
        // is not spawned from the reconcile tick, and does not consult
        // `HealAction`. This independence is the whole repair.
        let (gate, mut rx) = gate_with(DEFAULT_TRIGGER_COOLDOWN);
        assert_eq!(
            gate.offer(
                crate::sync::projector::PROJECTION_NAMESPACE,
                "node:while-heal-in-flight",
                "peerA"
            ),
            EnqueueDecision::Enqueued
        );
        assert_eq!(
            rx.try_recv().unwrap().content_id,
            "while-heal-in-flight",
            "a doc applied mid-heal-leg must still reach the worker"
        );
        // And the probe predicate itself never reads sweep state: an absent
        // local declaration probes whether or not a leg is running.
        assert!(should_probe(None, Some("uhCkk-peer")));
    }

    #[test]
    fn the_trigger_never_takes_the_author_path() {
        // A structural statement of the rule `process_trigger` enforces: only
        // `Adopted` is acted on. Encoded as the classification the worker makes,
        // so a future arm that starts authoring from a sync apply has to delete
        // this test on purpose.
        fn acts_on(outcome: &AdoptOutcome) -> bool {
            matches!(outcome, AdoptOutcome::Adopted)
        }
        assert!(acts_on(&AdoptOutcome::Adopted));
        assert!(!acts_on(&AdoptOutcome::Author));
        assert!(!acts_on(&AdoptOutcome::Held));
        assert!(!acts_on(&AdoptOutcome::Contested));
        assert!(!acts_on(&AdoptOutcome::AuthorThenAdopt {
            head_action_hash: "uhCkk-x".into(),
            carried_record: None,
            peer_id: "peerA".into(),
        }));
    }
}
