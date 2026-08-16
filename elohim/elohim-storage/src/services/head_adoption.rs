//! Adopt-before-author: consult the substrate for an existing canonical head
//! BEFORE minting a local root for a content id.
//!
//! ## The defect this closes
//!
//! Two peers can each hold a root for the same content id (`elohim-host-landing`
//! is the live case). Peer B's restart sweeps — the boot re-anchor pass
//! (`services::reanchor_backfill`) and the ghost-anchor witness
//! (`p2p::projection_reconcile::witness_ghost_anchors`) — selected their
//! candidates from LOCAL anchor state only, authored a fresh root through the
//! own conductor, and the projection immediately crowned that root as the
//! declared head. B therefore re-elected itself on every restart and could
//! never converge on A's declaration, no matter how many heal sweeps ran.
//!
//! The cure has two halves. The substrate half is
//! [`crate::db::content_diesel::HeadElection`]: authoring no longer implies
//! declaring. This module is the other half — a pre-flight that runs at the top
//! of each per-id loop body in both sweeps and asks, in order:
//!
//! 1. **LOCAL-DHT ARM.** Does my OWN conductor already resolve a CANONICAL head
//!    for this id? Then adopt it and do not author. This is the only arm that
//!    can act on notarized truth without any network round-trip.
//! 2. **PEER-HINT ARM.** Does a peer ADVERTISE a declaration (via the additive
//!    `ProjectionInventoryEntry::declared_head_action_hash`) while I hold none?
//!    Then fetch the head `Record` from that peer and declare it through MY
//!    conductor (declare-carries-Record). The advertised hash is a HINT that
//!    triggers a verified fetch — it is NEVER stamped into a row directly.
//! 3. **AUTHOR-THEN-ADOPT.** The zome refuses to declare a head for an id it has
//!    no local chain for (`declare_canonical_head: no content found for id`), so
//!    a genuinely fresh peer MUST author before it can adopt. Fix A makes that
//!    author non-declaring, so the sequence is: author the root, then
//!    immediately declare the PEER's head over it, in-process.
//!
//! ## What this module deliberately does NOT do
//!
//! **No cross-peer timestamp ordering.** `declared_head_at` is not globally
//! comparable — the zome substitutes the RECEIVING conductor's `sys_time` on the
//! carried-record branch, so "whose declaration is newer" cannot be decided from
//! two peers' projections. The Rust rule is PRESENCE-based
//! ([`decide_head_action`]); ordering is arbitrated in-zome by
//! `select_canonical_winner`. Inventing a newest-wins election here would
//! reintroduce the head-flapping this exists to stop. That remains true after
//! the CONTEST arm below: contesting supplies a CANDIDATE to the in-zome
//! arbiter, it never picks a winner.
//!
//! ## 4. CONTEST-THEN-OBEY (the two-way-declared class, 2026-08-02)
//!
//! The three arms above all assume at most one side holds a crown. The live
//! fleet's dominant class violates that: ~11.4k ids where THIS row declares X
//! and a peer advertises Y. `AdoptPeer` requires an undeclared local row, so it
//! never fired; the rule answered `Hold`; and because the ONLY automated minter
//! of canonical-head links is `AdoptPeer`, the DHT's `canonical_head` anchor
//! stayed EMPTY for the whole class. `select_canonical_winner` was never called
//! with a non-empty set — the arbiter existed but had nothing to arbitrate, so
//! every conductor answered with its own root-author fallback and every heal
//! honestly refused ("row already declared — heal left it to the canonical
//! channels", ~3.5k/hr on matthew alone). A permanent stalemate that no amount
//! of healing could break.
//!
//! [`HeadDecision::ContestPeer`] breaks it WITHOUT deciding anything locally: it
//! mints a canonical-head link naming the peer's head, which is one candidate in
//! the election. Every peer that does the same adds its own. The set is then
//! arbitrated on the DHT — tier, then notarized link clock, then link-hash
//! tiebreak — and the winner comes back through `resolve_content_head` as a
//! `canonical == true` answer carrying its election ordering, which
//! `content_diesel::canonical_move_verdict` obeys. Contest supplies; the DHT
//! elects; the projection obeys.
//!
//! Quiescence is structural, not timed: once a row carries an election
//! (`canonical_declared_at IS NOT NULL`) the rule returns to `Hold`, so a
//! converged corpus mints ZERO links per sweep. The transient window is at most
//! one sweep — the minting conductor's own next `resolve_content_head_local`
//! sees the link it just committed and stamps the election onto the row.
//!
//! ## Convergence arithmetic — what to watch on the meters (F-B, 2026-08-02)
//!
//! The pre-F-B estimate ("~11.4k ids, 200/tick, ~1-2h") was FALSIFIED live: the
//! first hour minted ~90 elections, not thousands. It was wrong in two ways, and
//! both are now fixed. Recorded here rather than deleted, because the corrected
//! arithmetic is only trustworthy if the way the first one lied is visible.
//!
//! **Why it was wrong.** (1) The 200/tick cap was never the binding constraint —
//! the 120s wall-clock budget was. At the observed ~0.4s per fast-failing
//! contest, 120s buys ~280 sequential attempt-slots, and a *productive* contest
//! (peer fetch + declare) costs seconds, so a handful of them consumed the whole
//! sweep. (2) The attempts were dominated by `no_local_chain` — a
//! target-independent refusal that CANNOT succeed on a later sweep either, so
//! the same predictable failures re-consumed the budget every 300s while
//! contestable ids sat behind them. Supply was ~200 attempts/sweep of which the
//! productive share was single digits: ~90/hour fleet-wide.
//!
//! **The corrected model.** Per pod, per sweep (300s cadence):
//!
//! ```text
//!   attempts/sweep  = min(200 cap, 120s ÷ (latency ÷ fanout))
//!   with fanout 8 and ~0.4s latency ⇒ 120 ÷ 0.05 = 2400 → capped at 200
//! ```
//!
//! So the fan-out moves the binding constraint OFF the wall clock and back ONTO
//! the 200/tick cap, which is where a cap belongs: the sweep now reliably spends
//! its full slice instead of timing out partway. The backoff then decides WHAT
//! that slice is spent on. With `p` = the share of candidates that are
//! predictable failures (live: `no_local_chain` was 887 of ~891 classified
//! attempts, so p ≈ 0.85-0.99 on the worst pods), the productive attempts per
//! sweep go from `200·(1-p)` — single digits at p = 0.97 — to ~200, because the
//! backoff moves the whole `p` share behind the rest.
//!
//! **Fleet drain, first order.** 7 pods × 200 productive attempts/sweep × 12
//! sweeps/hour = ~16.8k contest attempts/hour against ~11k contested ids. Even
//! discounting heavily for the productive-contest latency (seconds, not 0.4s —
//! a fetch plus a declare), for candidate overlap between pods, and for ids that
//! need two rounds (contest → election → obey), the whole contested corpus gets
//! its first real attempt within **hours, not days**. That is the F-B bar.
//!
//! **What to watch, in order of trustworthiness:**
//!
//! 1. `elohim_content_canonical_links_minted_total{source}` — the only series
//!    that proves supply. Must climb; `contest_peer_head` and
//!    `contest_self_head` should BOTH be non-zero once both sides participate.
//! 2. `elohim_content_contest_skipped_total{reason}` RISING while
//!    `elohim_content_contest_failed_total{class="no_local_chain"}` FALLS — that
//!    crossover IS the lever working (budget reclaimed, not merely re-spent).
//!    Skips rising while minting stays flat means the window is too long: cut
//!    `CONTEST_BACKOFF_SECONDS`, or set it to 0 to disable.
//! 3. `elohim_content_adopt_sweep_total{outcome}` — `budget_elapsed` should
//!    become RARE. If it stays dominant, the conductor is the constraint and
//!    raising `ADOPT_CONTEST_FANOUT` will not help.
//! 4. `elohim_content_election_obeyed_total` — the consumption half. Supply
//!    without obey means the elections are minting but not projecting.
//! 5. `elohim_projection_reconcile_divergent{stream="content"}` fleet-sum —
//!    the outcome, but the SLOWEST and noisiest signal (a rotating-page sample
//!    that oscillates ±5k by construction; a 30-60min decline proves nothing).
//!
//! The per-sweep `gaps` gauge remains untrustworthy as a trend. Judge F-B on
//! (1) and (2), not on gaps.
//!
//! ## Contest supply: what the first live window taught (2026-08-02)
//!
//! The first deploy of the contest arm ran attempts but minted ZERO, failing in
//! ~0.4s per attempt. Two hypotheses were raised and BOTH disconfirmed in code
//! before any fix was written — recorded here because each is the kind of theory
//! that looks obviously right and would have cost a deploy cycle:
//!
//! 1. **Reach starvation** (advertise-but-refuse). Disconfirmed: the inventory
//!    (`list_content_anchor_inventory`) and the head-record responder
//!    (`is_distribution_safe_reach`) read the SAME `DISTRIBUTION_SAFE_REACH`
//!    constant, and `community` is a member of it. Scoped rows are excluded from
//!    BOTH surfaces symmetrically, so no class is advertised yet refused. The
//!    observed `403 requiredReach:community` on `/db/content` is the *viewer
//!    standing* gate — a different plane — and actually confirms the contested
//!    rows are `community`, i.e. distribution-safe.
//! 2. **Fetcher never wired.** Disconfirmed: `run_heal` constructs
//!    `PeerHeadRecordFetcher` and passes `fetcher: Some(&…)` in the same
//!    `AdoptContext` the contest call receives.
//!
//! The mechanism is upstream of both. A two-way-declared row is DECLARED, so it
//! cannot enter `adopt_candidates` through the gapfill-refused route (that
//! predicate requires `local_declared.is_none()`). It enters only via
//! conductor-missing (`resolve_content_head_local` → `Ok(None)`) or timeout — and
//! for the conductor-missing majority the conductor has **no chain for the id**,
//! which `declare_canonical_head_inner` rejects at its FIRST, target-independent
//! gate. No candidate shape can pass that gate, and a carried record cannot help:
//! the gate runs before the carried path is consulted. The 0.4s fast-fail is the
//! tell — a `Network` `get_links` that truly left the box would block toward the
//! 60s conductor timeout, so it short-circuited at authority on genuine local
//! absence.
//!
//! Hence the split in `inc_contest_failed`: `no_local_chain` is NOT a storage
//! bug and self-candidacy cannot fix it. If it dominates the next window, the
//! remedy is upstream witnessing (or a deliberate coordinator-zome decision about
//! the no-chain gate), not another storage arm. `not_retrievable` / `fetch_none`
//! ARE storage-fixable, and self-candidacy is their answer.
//!
//! ## The second live window, and the composition that closes the loop
//!
//! Edge #1291 read: `no_local_chain` 887, `not_retrievable` 1, `declare_error` 3,
//! **`fetch_none` ZERO**, mints `adopt_peer=1` and no self-head. The zero is the
//! sharper signal: it means no CHAIN-HOLDING pod ever reached a fetch, i.e. the
//! holding side was not contesting at all.
//!
//! The reason is an admission gap, not a decision bug. A holding pod resolves its
//! declared rows fine, so they never take the conductor-missing or timeout
//! routes; and `gapfill_would_self_elect` requires `local_declared.is_none()`, so
//! a DECLARED row fails that guard too. Those rows landed in the general heal
//! arm, were honestly refused (`SkippedDeclared`), and were pushed to NO
//! candidate list — never reaching [`decide_head_action`]. So the election
//! received a candidate from neither side: the missing side *cannot* declare, and
//! the holding side was never *asked* to.
//! `declared_divergence_should_route_to_contest` admits exactly that class.
//!
//! **Expected composition once both sides participate:**
//!
//! 1. The HOLDING pod admits its declared+divergent rows and contests. Its fetch
//!    from the missing peer returns hash-only (`record: None`) — verified: a peer
//!    serves `head_action_hash` from its SQLite row while its conductor cannot
//!    produce the bytes, so the payload carries the hash and no record.
//! 2. Declaring the peer's head therefore fails `is not retrievable`, and
//!    SELF-CANDIDACY mints the holding pod's OWN head. That is the first real
//!    candidate the election has ever received. (`fetch_none` should now become
//!    non-zero where self-candidacy does not immediately succeed — its absence
//!    was the fingerprint of the admission gap.)
//! 3. An election now EXISTS for the id.
//! 4. The MISSING pod, which can see the election but not the content, obeys it
//!    via `try_obey_visible_election`: fetch the winner's bytes from the holding
//!    peer, have the zome prove them, stamp under the election's ordering.
//! 5. The HOLDING pod's own row converges through ordinary heal — its conductor
//!    now answers `canonical == true` with the election ordering, and the Fix-2
//!    guard authorizes the move.
//!
//! **Residual class:** ids where BOTH sides are conductor-missing. Neither can
//! declare (the no-chain gate is target-independent) and neither can obey (no
//! election can be minted).
//!
//! ## ADOPT-BEFORE-AUTHOR — the residual's exit, shipped DORMANT
//!
//! That residual's only automated exit is for a node WITHOUT a local chain to be
//! able to supply an election from a VALIDATED carried record. The coordinator
//! now permits exactly that (`content_store::classify_chain_gate`), and this
//! module drives it — behind `config::adopt_before_author_enabled()`, **default
//! OFF**. The capability is built, tested and wired; turning it on is an env-var
//! flip (`ELOHIM_ADOPT_BEFORE_AUTHOR=1`), not a build cycle.
//!
//! **Semantics.** The node ADOPTS a validated record's head as its declaration
//! instead of AUTHORING one from a chain it does not have. Nothing is trusted:
//! the zome re-derives the action hash, verifies the author's signature, checks
//! the entry↔action binding and enforces the target-id gate, so a transferred
//! claim confers only what the receiver re-derives (C5). The declaration is one
//! more candidate for `select_canonical_winner`, never a crowning (C1).
//!
//! **Where it fires.** Exactly the two sites that today refuse with
//! `no content found for id` — the CONTEST arm (the residual's real site) and
//! `declare_peer_head`. Both retry ONLY after the classic attempt has already
//! been refused for that reason, and only with bytes in hand, so the flag can
//! never change what the classic path does.
//!
//! **Convergence math.** The reachable set is the both-sides-missing pairs where
//! at least one side can FETCH the winner's bytes from any advertiser — i.e.
//! `contest_failed{no_local_chain}` candidates whose view-federation fetch
//! returns `Present` WITH bytes. Ids where no advertiser can serve the record
//! (`fetch_none`) stay out of reach and are unaffected: the arm requires
//! evidence, so it converts the byte-servable share of that class and nothing
//! else. `minted{source="adopt_before_author"}` over
//! `contest_failed{class="no_local_chain"}` reads that share directly, live.
//!
//! **Flag OFF is behaviour-identical.** Every new branch is inside
//! `adopt_before_author_param`, which is `false` unless the node flag is on AND
//! validated-shape bytes are present; with it false no extra conductor call, no
//! clone, and no metric increment happens, and the zome refuses identically on
//! `adopt_before_author: false`.
//!
//! **No admission or budget deny.** Every arm degrades to the author path and
//! retries on the next sweep. Pacing and yielding are the callers' existing
//! bounds; nothing here sheds work.
//!
//! ## In-process, never over HTTP
//!
//! The declare goes through `conductor_writes::call_declare_canonical_content_head`
//! directly. The HTTP declare route is auth-gated and admission-shed-eligible;
//! routing a heal through it would make convergence depend on the edge's load
//! shedding (the heal-exemption rule — see `p2p::sync_round`).

use std::collections::HashMap;
use std::sync::Arc;

use seam_contracts::{Answer, ReasonLabel};

use crate::conductor_admission::{is_admission_shed, AdmissionClass};
use crate::db::content_diesel::{self, StampMode, StampOutcome};
use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::hc_client::HcClient;
use crate::services::conductor_writes::{self, ContentHeadWire};
use crate::trust::pricer::PricedVerification;

/// Zome guard substring meaning "I cannot declare a head for this id because I
/// have no local content chain for it" (`content_store::declare_canonical_head_inner`).
///
/// This is the AUTHOR-THEN-ADOPT trigger, not a failure: a peer that has never
/// held the content genuinely has nothing to hang a declaration on yet.
const ERR_NO_LOCAL_CHAIN: &str = "no content found for id";

/// Zome guard substring meaning "I cannot retrieve the target action". On a
/// full-arc fleet this is TERMINAL, not slow (the `get` cascade short-circuits
/// at authority), so it means the carried record was absent or rejected. Degrade
/// to the author path; the next sweep may have a peer that can serve the record.
const ERR_NOT_RETRIEVABLE: &str = "is not retrievable";

/// A peer's ADVERTISED declaration for a content id, harvested from the
/// projection-inventory federation response.
///
/// Emphatically a hint, not a value: nothing in this module writes
/// `head_action_hash` into a local row. It selects WHICH peer to ask for a
/// verifiable `Record` and WHICH action to name in a conductor-side declare.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerHeadHint {
    /// The action the advertising peer says its canonical channel elected.
    pub head_action_hash: String,
    /// The ordering the peer carries. Recorded for logs only — see the
    /// module-level note on why this is never used to elect.
    pub declared_at: Option<i64>,
    /// Which peer advertised it (the one we ask for the `Record` FIRST).
    pub peer_id: String,
    /// OTHER peers that advertised the SAME `head_action_hash` this sweep — the
    /// courier plurality the evidence fetch may route to when
    /// [`Self::peer_id`] cannot serve the bytes.
    ///
    /// **Same head, deliberately.** An alternate here is a substitute COURIER
    /// for one declaration, never a second opinion about which head is
    /// canonical: nothing about the decision changes when we ask a different
    /// peer, only who hands us the bytes. A peer advertising a DIFFERENT head is
    /// not recorded, because adopting from it would silently change which action
    /// gets declared.
    ///
    /// Bounded at harvest time (`p2p::projection_reconcile`); empty when only
    /// one peer advertises the id, which is the pre-2026-08-03 shape exactly.
    pub alternates: Vec<String>,
}

/// How many ALTERNATE advertisers one hint retains.
///
/// Bounded, not unbounded: the fallback ladder in [`resolve_peer_evidence`]
/// walks up to [`crate::config::evidence_fallback_max_alternates`] of these,
/// healthiest first, per evidence resolution — this cap is what makes that
/// ladder a FIXED, pre-sized walk rather than a retry loop. Raised 3 → 6
/// (2026-08-09, B2b, drain lever 4): the household mesh the harvest observes
/// routinely advertises the same genesis-seeded row from more than 3 couriers,
/// so the old cap silently discarded plurality the fallback ladder could have
/// used. The memory cost is `ids × 6` short strings — still bounded by
/// construction, not by convention.
///
/// This cap and [`crate::config::evidence_fallback_max_alternates`] are BOTH
/// held under the SAME write-guard invariant as `adopt_contest_fanout` — see
/// `crate::config::assert_courier_ladder_budget`'s doc for the adam
/// 2026-07-20 melt this exists to prevent a repeat of.
pub const ALTERNATE_ADVERTISER_CAP: usize = 6;

impl PeerHeadHint {
    /// A hint with no alternates — the shape every construction site had before
    /// advertiser diversity existed.
    pub fn sole(head_action_hash: String, declared_at: Option<i64>, peer_id: String) -> Self {
        Self {
            head_action_hash,
            declared_at,
            peer_id,
            alternates: Vec::new(),
        }
    }
}

/// `content_id → ` the first peer-advertised declaration seen this sweep, plus
/// the other peers advertising that same declaration.
pub type PeerHeadHints = HashMap<String, PeerHeadHint>;

/// WHY a peer served a head hash-only — the responder's own account of its
/// absent `Record`, carried across the wire as a tolerant string
/// (`views::ContentHeadRecordPayload::record_absent_reason`) and typed here.
///
/// **Concerns:** C4 (honest absence — an absence with a stated provenance is a
/// different object from an unexplained one, and this type is what keeps them
/// from collapsing). C8 (the vocabulary is closed at the point of USE, while the
/// wire stays open so a future word cannot break a rolling deploy).
///
/// **Contract tests:** [`tests::a_wire_reason_round_trips_through_its_type`] and
/// [`tests::an_unrecognised_reason_is_unknown_not_a_decode_failure`].
///
/// ## Why this exists
///
/// Until 2026-08-03 every hash-only answer looked identical to the requester:
/// `record: None`, three causes, one bit. The live split behind that one bit was
/// ~61% STRUCTURAL (the advertiser's conductor cleanly holds no record — the
/// bytes are nowhere, and no capacity relief will make them appear) and ~39%
/// SATURATION (the bytes exist; the advertiser's conductor could not serve them
/// inside the responder budget). Those point at opposite remedies, and the arm
/// that consumes them was re-asking both every hour forever.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordAbsentReason {
    /// The advertiser's conductor answered cleanly and holds NO record for the
    /// head it advertises. Re-asking THIS peer cannot help; only new bytes
    /// entering the fleet can.
    NoRecord,
    /// The advertiser's conductor answered with an error.
    ConductorError,
    /// The advertiser's responder budget elapsed. The bytes plausibly exist and
    /// its conductor is saturated.
    BudgetElapsed,
    /// The responder named a reason this build does not know — a peer running a
    /// FUTURE version. Treated exactly like an unstated reason: no long backoff
    /// is taken on a word we cannot interpret.
    Unknown,
}

impl RecordAbsentReason {
    /// The wire token for this reason. Kept as the SAME strings the degrade
    /// counter has always used, so a log line and a metric label read alike.
    ///
    /// [`Self::Unknown`] is never emitted by a responder — it only ever arises
    /// from decoding — but it maps to `"unknown"` so a round-trip is total.
    pub fn wire_str(self) -> &'static str {
        match self {
            RecordAbsentReason::NoRecord => "no_record",
            RecordAbsentReason::ConductorError => "conductor_error",
            RecordAbsentReason::BudgetElapsed => "budget_elapsed",
            RecordAbsentReason::Unknown => "unknown",
        }
    }

    /// Decode the wire field.
    ///
    /// The two `None`-ish inputs stay DISTINCT from each other in the type even
    /// though both take the ordinary backoff downstream: `None` means the
    /// responder said nothing (a pre-2026-08-03 peer), `Some(Unknown)` means it
    /// said something this build cannot read. Neither may ever be widened into
    /// [`Self::NoRecord`] — the long evidence-absent backoff is licensed by
    /// stated evidence, never by its absence.
    pub fn from_wire(raw: Option<&str>) -> Option<Self> {
        let raw = raw?.trim();
        Some(match raw {
            "no_record" => RecordAbsentReason::NoRecord,
            "conductor_error" => RecordAbsentReason::ConductorError,
            "budget_elapsed" => RecordAbsentReason::BudgetElapsed,
            _ => RecordAbsentReason::Unknown,
        })
    }
}

/// A head `Record` fetched from a peer — the SOURCE half of
/// declare-carries-Record, obtained over the P2P view-federation plane.
#[derive(Debug, Clone)]
pub struct CarriedHeadRecord {
    /// The action the serving peer says the record is for. May differ from the
    /// hint if the peer's head moved between the inventory sweep and this fetch;
    /// the SERVED hash wins, because it is the one the bytes actually prove.
    pub head_action_hash: String,
    /// Serialized `Record` bytes, or `None` when the peer holds a head but its
    /// conductor cannot retrieve the record (honest absence — we then declare
    /// without a carried record, which is the classic behaviour).
    pub record: Option<Vec<u8>>,
    /// The responder's account of why [`Self::record`] is absent, when it gave
    /// one. Always `None` when bytes ARE carried; `None` also when the peer is
    /// too old to state a reason.
    pub record_absent_reason: Option<RecordAbsentReason>,
}

/// Fetches a peer's head `Record` over whichever transport the caller owns.
///
/// A trait rather than a `P2PHandle` parameter so this module — and the
/// `services` layer generally — stays transport-neutral: the libp2p and iroh
/// adapters both delegate to the same pre-flight.
///
/// This is the plan's first **external-facing conformance contract** (Design
/// surface 7): an implementor outside this repo receives the obligations below
/// as a type plus this doc, not as prose it must find and obey.
#[async_trait::async_trait]
pub trait HeadRecordFetcher: Send + Sync {
    /// Ask `peer_id` for its head `Record` for `content_id`.
    ///
    /// **Concerns:** C4 (honest absence), C6a (bounded work — the no-retry
    /// clause below is a budget, and a retry ladder against an uncancellable
    /// call IS a loop even with no `loop` token).
    ///
    /// **Contract test:** [`tests::answer_states_collapse_uniformly_at_every_fetch_site`];
    /// the transport-level mapping is asserted by
    /// `p2p::head_record_client::tests::fetch_failures_are_unreachable_not_absent`.
    ///
    /// # The contract (was prose above this signature until 2026-08-02)
    ///
    /// The previous signature returned `Option<CarriedHeadRecord>` and carried
    /// this obligation as a comment: *"`None` for EVERY failure mode, including
    /// 'this peer is too old to decode the request'. Implementations MUST log
    /// the degradation explicitly and MUST NOT retry-loop — the caller falls
    /// through to the author path, and the next sweep tries again."*
    ///
    /// That comment merged two genuinely different answers into one bit, so it
    /// is now a type. Implementations MUST map:
    ///
    /// - [`Answer::Present`] — the peer answered and served a head. `record` may
    ///   still be `None` inside it: a peer holding a head whose bytes it cannot
    ///   retrieve answers hash-only, and that IS an answer.
    /// - [`Answer::Absent`] — the peer answered and holds **no head** for this
    ///   id. An observed absence, and the only arm that may claim one.
    /// - [`Answer::Unreachable`] — no usable answer arrived: transport failure,
    ///   timeout, an unparseable peer id, an undecodable payload, or a peer too
    ///   old to decode the request. Nothing is established in either direction.
    ///
    /// Unchanged and still binding: implementations MUST log the degradation
    /// explicitly and **MUST NOT retry-loop**. The reconcile sweep is the retry,
    /// at its own cadence.
    ///
    /// # The evidence obligation (2026-08-03)
    ///
    /// A `Present` answer whose `record` is `None` SHOULD carry
    /// [`CarriedHeadRecord::record_absent_reason`] when the serving peer stated
    /// one, and MUST NOT invent one when it did not. Two rules make this safe to
    /// consume:
    ///
    /// - **`NoRecord` is a positive statement, never an inference.** It licenses
    ///   a 24h deferral of the id, so it may only be set when the responder
    ///   actually reported that its conductor holds no record. Silence, an
    ///   unrecognised token, a transport failure, and a locally-dropped
    ///   malformed record are ALL [`RecordAbsentReason::Unknown`]-or-`None`.
    /// - **The field is advisory.** An implementation that always answers `None`
    ///   here is conformant and behaves exactly as every implementation did
    ///   before this field existed — the consumer falls back to the ordinary
    ///   backoff window.
    ///
    /// # Behaviour-neutrality of the retrofit
    ///
    /// Every call site in this module currently treats `Absent` and
    /// `Unreachable` identically (both fall through to the record-less path),
    /// which is what made this retrofit admissible — see the plan's P2.3
    /// precondition. Each site names that collapse with
    /// [`Answer::into_option`], so a future arm that needs the distinction can
    /// find every place it is currently discarded with one grep.
    async fn fetch(&self, peer_id: &str, content_id: &str) -> Answer<CarriedHeadRecord>;
}

/// Ask one peer for `id`'s head record and CLASSIFY what came back — the single
/// site in this module where an adopt-arm evidence resolution happens, and
/// therefore the single site where [`crate::metrics::inc_adopt_evidence`] fires.
///
/// **Concerns:** C4 (the named `into_option` collapse lives here now, once,
/// instead of at each fetch site). C8 (exactly one increment per resolution, so
/// the counter's sum is "how many times did this arm go looking for bytes" and
/// every state shares one denominator).
///
/// **Contract tests:** [`tests::evidence_classification_maps_every_answer_shape`]
/// and [`tests::an_absent_fetcher_resolves_no_evidence_at_all`].
///
/// Returns `(carried, evidence)`. `evidence` is `None` ONLY when no fetch
/// happened at all (the boot pass carries no fetcher) — an unasked question is
/// not an answer, and counting it would corrupt the denominator.
///
/// # ADVERTISER DIVERSITY (2026-08-03)
///
/// When the hinted advertiser answers without bytes and without a stated
/// structural absence, this asks the OTHER advertisers of the same declaration
/// (see [`fallback_warrants`] for the trigger and
/// [`crate::services::advertiser_health`] for the order). The hint is
/// first-advertiser-wins, and on the live fleet that peer was usually a
/// 100%-CFS-throttled conductor while idle peers holding the same genesis-seeded
/// rows were never asked once — which is why adopt-before-author had never
/// minted despite being wired correctly end to end. **The protocol adapts to its
/// weakest members rather than demanding they be strong.**
///
/// ## SOURCE WIDENING (drain lever 3, 2026-08-07)
///
/// The 2026-08-03 shape asked exactly ONE alternate and then gave up for the
/// sweep, while the harvest was already retaining up to
/// [`ALTERNATE_ADVERTISER_CAP`] (originally 3, widened to 6 2026-08-09 B2b)
/// couriers per id — so two thirds of the plurality the sweep had paid to
/// collect were never used, and an id whose second-best courier could serve the
/// bytes waited a full sweep (or a full backoff window) to find that out. The
/// probe now walks the ranked couriers, healthiest first, up to
/// [`crate::config::evidence_fallback_max_alternates`] (default 4, raised from
/// 2 the same day, held under `crate::config::assert_courier_ladder_budget`).
/// Fill bandwidth per gap id rises with the number of couriers that actually
/// exist, which is the point: an id converges the moment ANY courier serves it.
///
/// The bound is structural, not conventional: a FIXED, pre-sized ladder over a
/// list the harvest already capped — iterated once, never re-asking a peer, with
/// no recursion and no retry. And the accounting stays honest —
/// [`crate::metrics::inc_adopt_evidence`] fires EXACTLY ONCE per resolution, for
/// the FINAL state, so the evidence meter's denominator is unchanged by either
/// lever. The fallback's own decisions land on a separate
/// [`crate::metrics::CONTENT_ADOPT_EVIDENCE_FALLBACK`] series, where `attempted`
/// counts fallback FETCHES (so `attempted / carried` reads as a per-round-trip
/// hit rate) while `carried` / `degraded` / `no_alternative` stay one-per-
/// resolution.
async fn resolve_peer_evidence(
    fetcher: Option<&dyn HeadRecordFetcher>,
    hint: &PeerHeadHint,
    id: &str,
) -> (
    Option<CarriedHeadRecord>,
    Option<crate::metrics::AdoptEvidence>,
) {
    use crate::metrics::{AdoptEvidence, AdoptEvidenceFallback};

    let Some(f) = fetcher else {
        return (None, None);
    };
    // NAMED C4 COLLAPSE (`into_option`): `Absent` and `Unreachable` both mean
    // "no bytes and no stated cause" to every consumer below, and both classify
    // as `Unknown` — the pre-2026-08-03 behaviour exactly, now with the collapse
    // recorded on a meter instead of only in a log line.
    let carried = f.fetch(&hint.peer_id, id).await.into_option();
    let state = classify_evidence(carried.as_ref());
    crate::services::advertiser_health::record(&hint.peer_id, state == AdoptEvidence::Carried);

    if !fallback_warrants(state, crate::config::evidence_fallback_enabled()) {
        crate::metrics::inc_adopt_evidence(state);
        return (carried, Some(state));
    }

    // The primary could not serve, and the cause is one another peer might not
    // share. Ask the OTHER advertisers of the SAME declaration, healthiest
    // first, up to the configured breadth.
    let alternates = crate::services::advertiser_health::select_alternatives(
        &hint.peer_id,
        &hint.alternates,
        crate::config::evidence_fallback_max_alternates(),
    );
    if alternates.is_empty() {
        // No second courier exists for this id. Counted apart from a failed
        // fallback because the remedy differs: this says hint PLURALITY is the
        // wall, not peer health.
        crate::metrics::inc_adopt_evidence_fallback(AdoptEvidenceFallback::NoAlternative);
        crate::metrics::inc_adopt_evidence(state);
        return (carried, Some(state));
    }

    // bounded-work: at most `evidence_fallback_max_alternates()` fallback
    // fetches per evidence resolution — hard-clamped to
    // `ALTERNATE_ADVERTISER_CAP` (6), which is how many couriers the harvest
    // ever retains, so this is a FIXED, pre-sized ladder rather than a retry
    // loop: it iterates a bounded list once, never re-asks a peer, and cannot
    // recurse. Worst case is 1 + N round-trips per candidate per sweep; the
    // sweep's own 200/tick + 120s budget is unchanged and stays authoritative
    // over all of it.
    let mut last: Option<(&str, AdoptEvidence)> = None;
    for alternate in &alternates {
        crate::metrics::inc_adopt_evidence_fallback(AdoptEvidenceFallback::Attempted);
        let alt_carried = f.fetch(alternate, id).await.into_option();
        let alt_state = classify_evidence(alt_carried.as_ref());
        crate::services::advertiser_health::record(alternate, alt_state == AdoptEvidence::Carried);

        if alt_state == AdoptEvidence::Carried {
            crate::metrics::inc_adopt_evidence_fallback(AdoptEvidenceFallback::Carried);
            crate::metrics::inc_adopt_evidence(AdoptEvidence::Carried);
            tracing::info!(
                target: "elohim_storage::head_adoption",
                content_id = %id,
                primary_peer = %hint.peer_id,
                primary_evidence = state.label(),
                alternate_peer = %alternate,
                couriers_asked = alternates.len(),
                "adopt-before-author: routed around a starved advertiser — an alternative peer \
                 served the head Record the hinted one could not"
            );
            return (alt_carried, Some(AdoptEvidence::Carried));
        }
        last = Some((alternate.as_str(), alt_state));
    }

    // EVERY courier answered without bytes. The PRIMARY's evidence is still the
    // resolution: it is the peer whose declaration this row is reasoning about,
    // and the backoff class downstream must key on what IT said. (A conservative
    // choice on purpose — an alternate's `no_record` may never license the long
    // deferral when the primary said `budget_elapsed`, because the primary said
    // the bytes plausibly exist. Widening the probe does not widen that licence.)
    crate::metrics::inc_adopt_evidence_fallback(AdoptEvidenceFallback::Degraded);
    crate::metrics::inc_adopt_evidence(state);
    tracing::debug!(
        target: "elohim_storage::head_adoption",
        content_id = %id,
        primary_peer = %hint.peer_id,
        primary_evidence = state.label(),
        couriers_asked = alternates.len(),
        last_alternate_peer = last.map(|(p, _)| p).unwrap_or("none"),
        last_alternate_evidence = last.map(|(_, s)| s.label()).unwrap_or("none"),
        "adopt-before-author: no alternative advertiser could serve the Record either; \
         the primary's evidence stands"
    );
    (carried, Some(state))
}

/// Does this evidence state warrant asking a SECOND advertiser?
///
/// **Concerns:** C6a (this predicate IS the amplification gate — every `true`
/// here is one extra round-trip). C4 (the `no_record` arm is the honest-absence
/// rule: post-2026-08-10 a stated no-record is a fact about the ADVERTISER's
/// local store — the responder's get is LOCAL-strategy — not about the fleet;
/// it still warrants no same-sweep second ask, because the evidence-absent
/// backoff + the ghost-decay dwell own that population on the slower clock,
/// and a second ask would just sample another peer's cache).
///
/// **Contract tests:** [`tests::only_a_peer_shaped_failure_warrants_a_second_ask`]
/// and [`tests::the_disabled_lever_never_warrants_a_second_ask`].
///
/// Exhaustively matched rather than wildcarded: a future evidence state must
/// force an explicit decision here, because the default of "retry it" is the
/// expensive one.
///
/// `enabled` is a PARAMETER rather than a config read, for the reason
/// [`evidence_backoff_class`] documents: the OFF switch must be provable without
/// touching a process-wide `OnceLock` (the parallel-test flake this codebase has
/// already paid for once).
fn fallback_warrants(state: crate::metrics::AdoptEvidence, enabled: bool) -> bool {
    use crate::metrics::AdoptEvidence;
    if !enabled {
        return false;
    }
    match state {
        // Bytes in hand — nothing to route around.
        AdoptEvidence::Carried => false,
        // The advertiser's conductor cleanly holds no record. Post-2026-08-10
        // this is a PEER-LOCAL fact, not a fleet-wide one: the responder's
        // `get_record_for_action` answers from its LOCAL store only, so a
        // clean no-record can also mean not-yet-gossiped (full-arc law).
        // Still no fallback here: the evidence-absent backoff owns this
        // population on the slower clock, and the ghost-decay dwell is what
        // upgrades a persistent no-record into a standing verdict — a second
        // same-sweep ask would just sample another peer's cache.
        AdoptEvidence::NoRecord => false,
        // The three PEER-SHAPED failures — saturation, a conductor error, and an
        // answer that established nothing (including unreachable). All say
        // something about THIS peer's ability to serve right now, and another
        // peer plausibly does not share it.
        AdoptEvidence::BudgetElapsed | AdoptEvidence::ConductorError | AdoptEvidence::Unknown => {
            true
        }
    }
}

/// What one fetch established about the SUPPLY of head-record bytes.
///
/// Pure and total, split from [`resolve_peer_evidence`] for the same reason
/// `head_record_client::classify_payload` is split from its transport half: the
/// round-trip needs a swarm, the classification is the part that can be wrong.
///
/// The load-bearing rule is the LAST arm: an unstated or unrecognised reason is
/// `Unknown`, never `NoRecord`. `NoRecord` licenses a 24h deferral, and it must
/// be licensed by a peer's positive statement — never by silence.
fn classify_evidence(carried: Option<&CarriedHeadRecord>) -> crate::metrics::AdoptEvidence {
    use crate::metrics::AdoptEvidence;
    match carried {
        Some(c) if c.record.is_some() => AdoptEvidence::Carried,
        Some(c) => match c.record_absent_reason {
            Some(RecordAbsentReason::NoRecord) => AdoptEvidence::NoRecord,
            Some(RecordAbsentReason::ConductorError) => AdoptEvidence::ConductorError,
            Some(RecordAbsentReason::BudgetElapsed) => AdoptEvidence::BudgetElapsed,
            Some(RecordAbsentReason::Unknown) | None => AdoptEvidence::Unknown,
        },
        None => AdoptEvidence::Unknown,
    }
}

/// Which backoff class the no-chain refusal should record, given what the fetch
/// established about supply.
///
/// **Concerns:** C3 (both outcomes are DEFERRALS with automatic expiry — this
/// function chooses a duration, never an exclusion). C8 (the choice is a typed
/// class the skip counter reports, so "how much of the residual is
/// evidence-absent?" is a query, not an inference).
///
/// **Contract tests:** [`tests::only_a_stated_no_record_takes_the_long_backoff`]
/// and [`tests::a_zero_evidence_window_restores_the_ordinary_class`].
///
/// Pure and total, taking the window rather than reading config, so the
/// env-disabled behaviour is testable without touching a process-wide `OnceLock`
/// (the parallel-test flake this codebase has already paid for once).
///
/// `evidence_absent_window == 0` collapses to the ordinary class rather than to
/// "no backoff": disabling a lever must restore the behaviour before it existed,
/// never something worse than it.
fn evidence_backoff_class(
    evidence: Option<crate::metrics::AdoptEvidence>,
    evidence_absent_window: std::time::Duration,
) -> crate::metrics::ContestSkip {
    let stated_absent = evidence == Some(crate::metrics::AdoptEvidence::NoRecord);
    if stated_absent && !evidence_absent_window.is_zero() {
        crate::metrics::ContestSkip::EvidenceAbsentBackoff
    } else {
        crate::metrics::ContestSkip::NoLocalChainBackoff
    }
}

/// Record the generic Arm-1 declaration failure on the ordinary finite contest
/// clock while preserving its distinct C8 reason label.
fn note_declare_error_backoff(id: &str) {
    crate::services::contest_backoff::note(id, crate::metrics::ContestSkip::DeclareErrorBackoff);
}

/// The admission class EVERY sweep-side declare in this module travels under.
///
/// The contest/adopt arms are reconciler work: nobody is waiting on them, and
/// their deferral path is free (the next sweep, at its own cadence, under its
/// own slice cap). Standing in the INTERACTIVE lane for them buys nothing and
/// costs the one thing the gate exists to protect — the wait bound a person's
/// HTTP declare is holding. The declare ROUTES (`http.rs`) deliberately keep
/// [`conductor_writes::DECLARE_DEFAULT_CLASS`]: those are operator acts.
///
/// The trade this makes explicit: a background caller gives up sooner, so these
/// arms WILL see sheds under load. That is the intended outcome — see
/// [`declare_was_shed`] for why a shed must never be booked as a failure.
const SWEEP_DECLARE_CLASS: AdmissionClass = AdmissionClass::Background;

/// TRUE when a declare failed at the ADMISSION GATE rather than in the conductor.
///
/// A shed establishes NOTHING: the call never crossed the websocket, so the
/// candidate is exactly as eligible as it was a microsecond earlier and the
/// conductor spent nothing on it. Every sweep-side declare failure path in this
/// module books a *verdict* — a `contest_failed` count, a per-id backoff window,
/// or (in `declare_peer_head`) an AUTHOR fallback that mints a competing root —
/// and each of those is a durable consequence drawn from an answer that was
/// never given. Routing a shed like backpressure keeps local capacity pressure
/// from converting into fleet-visible state.
fn declare_was_shed(e: &StorageError) -> bool {
    is_admission_shed(e)
}

/// Book a sweep-side declare failure into the contest backoff ledger UNLESS it
/// was a gate shed. Returns TRUE when a backoff window was actually recorded.
///
/// This is the ONE place the shed/verdict split touches the ledger, so the
/// contract can be tested against the real ledger rather than a restatement of
/// the rule (see `a_shed_declare_earns_no_backoff_window`).
fn note_declare_backoff_unless_shed(
    id: &str,
    e: &StorageError,
    class: crate::metrics::ContestSkip,
) -> bool {
    if declare_was_shed(e) {
        return false;
    }
    crate::services::contest_backoff::note(id, class);
    true
}

/// What the pre-flight resolved from the OWN conductor.
///
/// **Concerns:** C4 (honest absence), C0 (plane location — the *subject* of this
/// question is THIS conductor's resolution, never DHT-wide existence; see the
/// scope note below).
///
/// **Contract test:** [`tests::local_resolve_keeps_observed_absence_apart_from_unresolved`]
/// and its siblings in this module.
///
/// The ghost sweep has ALREADY paid for this answer (`heal_content` collects its
/// candidates precisely from `resolve_content_head` returning `Ok(None)`), so it
/// passes [`LocalResolve::Resolved`] rather than burning a second round-trip on
/// an answer it holds.
///
/// # The CONTRACT DEVIATION this type replaced
///
/// Until 2026-08-02 the "already known" arm was `Known(Option<&ContentHeadWire>)`
/// and carried a 40-line `CONTRACT DEVIATION (2026-07-29)` comment: its `None`
/// had acquired TWO provenances — an OBSERVED absence (the ghost sweep saw
/// `Ok(None)`) and an UNKNOWN (the heal loop's resolve TIMED OUT, see
/// `projection_reconcile::timeout_should_route_to_adopt`) — while every caller
/// kept reading it as the first. The comment ended with an instruction to a
/// future editor: *split this variant rather than adding behaviour that silently
/// reads the timeout case as observed absence.*
///
/// That split happened (`Known(None)` vs `Unresolved`) and this is its second
/// half: the split is now expressed in the shared vocabulary
/// ([`seam_contracts::Answer`]) instead of in two bespoke variants, so the
/// distinction is a *type* every seam in the protocol reads the same way rather
/// than a comment each new reader must rediscover. Plan task P1.2; canon row C4.
///
/// # Scope — what `Absent` claims here, and what it must never claim
///
/// The full-arc law says a conductor's local `get` miss means gossip has not
/// delivered the record, NOT that the record does not exist — so
/// [`seam_contracts::Answer::from_local_get`] maps a miss to `Unreachable`. That
/// law is about **DHT-wide existence**. This type asks a narrower question:
/// *what did MY conductor resolve for this id?* A `resolve_content_head`
/// answering `Ok(None)` is an observed fact about this conductor, so it is
/// honestly [`Answer::Absent`] **at this plane** — which is exactly what the
/// wave-4 split established and what the first `Answer<T>` adoption is required
/// not to re-merge.
///
/// ASSUMPTION A FUTURE EDITOR MUST PRESERVE (carried forward verbatim in force):
/// `Absent` here must never become anything AUTHORITATIVE about the DHT — it
/// must not author, delete, tombstone, or otherwise treat the id as
/// proven-absent network-wide. Both non-present answers do exactly one thing
/// today: foreclose the `AdoptLocal` arm, leaving `AdoptPeer` / `ContestPeer` /
/// `Hold`. Neither asserts network-wide absence.
///
/// ONE REFINEMENT, not an exception ([`ghost_decay_authorizes_author`],
/// operator flag `ELOHIM_GHOST_DECLARATION_DECAY`, default off): the decay arm
/// may route `Hold`/`ContestPeer` to `Author`, but NEVER on `Absent` alone —
/// it additionally requires a LIVE advertising hint this sweep, the
/// advertising peer's OWN stated no-record verdict having STOOD the decay
/// dwell (evidence-absent backoff + [`crate::config::ghost_decay_min_dwell`]),
/// and the absence of a local canonical election. `Unreachable` never
/// qualifies. What it authors is a fresh provable root that a later real
/// canonical declaration outranks; it still asserts nothing about
/// network-wide absence.
#[derive(Debug, Clone, Copy)]
pub enum LocalResolve<'a> {
    /// Not yet asked — the pre-flight calls `resolve_content_head` itself.
    ///
    /// Deliberately NOT an [`Answer`] arm: "I have not asked" is an instruction
    /// to the pre-flight, not an answer about the world. Folding it into
    /// `Unreachable` would make an unasked question indistinguishable from an
    /// unanswered one — the same collapse one level up.
    Probe,
    /// The conductor was asked and the answer is in hand, with its two absences
    /// kept apart by the shared vocabulary:
    ///
    /// - [`Answer::Present`] — a head was resolved (canonical or fallback).
    /// - [`Answer::Absent`] — the conductor answered and holds no head for this
    ///   id (`Ok(None)`, the ghost class). Observed at THIS conductor; see the
    ///   scope note on the type.
    /// - [`Answer::Unreachable`] — the conductor did not answer (timeout class).
    ///   Absence was never observed, only unestablished.
    Resolved(Answer<&'a ContentHeadWire>),
}

impl<'a> LocalResolve<'a> {
    /// The ghost-sweep constructor: the conductor ANSWERED, with `Some(head)` or
    /// with a genuine `Ok(None)`.
    ///
    /// **Concerns:** C4 — this is [`Answer::observed_absence`], named at the
    /// call site so the provenance is visible in review.
    ///
    /// **Contract test:** [`tests::local_resolve_keeps_observed_absence_apart_from_unresolved`].
    pub fn observed(head: Option<&'a ContentHeadWire>) -> Self {
        LocalResolve::Resolved(Answer::observed_absence(head))
    }

    /// The timeout constructor: the conductor was asked and did NOT answer.
    ///
    /// **Concerns:** C4 — absence is not established in either direction.
    ///
    /// **Contract test:** [`tests::local_resolve_keeps_observed_absence_apart_from_unresolved`].
    pub fn unresolved() -> Self {
        LocalResolve::Resolved(Answer::Unreachable)
    }

    /// The head this resolve carries, if any — the single place both non-present
    /// answers collapse, so the collapse is greppable rather than inline.
    ///
    /// **Concerns:** C4 — every caller of this method is a place where "the
    /// conductor holds nothing" and "the conductor never answered" merge. Today
    /// that merge is correct at every call site (both only foreclose
    /// `AdoptLocal`); a future arm that needs the distinction matches on
    /// [`LocalResolve::Resolved`] instead.
    ///
    /// **Contract test:** [`tests::both_absences_foreclose_adopt_local_identically`].
    pub fn head(&self) -> Option<&'a ContentHeadWire> {
        match self {
            LocalResolve::Probe => None,
            LocalResolve::Resolved(answer) => (*answer).into_option(),
        }
    }
}

/// Has this id's DHT ELECTION already been read, or must the obey arm read it
/// itself?
///
/// The sibling of [`LocalResolve`], for the second conductor read
/// `try_adopt_canonical_head` makes. It exists for exactly one reason: the adopt
/// arm can now learn every candidate's election in 2–4 BATCHED round-trips
/// (`services::head_batch_resolver`) instead of one per candidate, and the
/// pre-flight must be able to consume that answer instead of re-buying it.
///
/// **Concerns:** C4 — the three answer states stay apart, and each routes the
/// way the single-id path already routed its equivalent:
///
/// - [`Answer::Present`] — an election is visible; obey it.
/// - [`Answer::Absent`] — the conductor answered and holds no election. EXACTLY
///   the old `Ok(None)` arm: not applicable, carry on with the normal decision.
/// - [`Answer::Unreachable`] — the conductor did not answer (backpressure, a
///   never-started batch id, or a coordinator that would not serve the extern).
///   EXACTLY the old `Err` arm: a conductor that will not answer is not evidence
///   of anything, so fall through to the normal decision rather than hold.
///
/// [`ElectionResolve::Probe`] is deliberately NOT an [`Answer`] arm, for the same
/// reason [`LocalResolve::Probe`] is not: "I have not asked" is an instruction,
/// not an answer about the world.
#[derive(Debug, Clone, Copy)]
pub enum ElectionResolve<'a> {
    /// Not yet asked — the obey arm calls `resolve_canonical_election` itself
    /// (the pre-batch behaviour, byte-for-byte).
    Probe,
    /// Already asked, in a batch the caller paid for.
    Resolved(Answer<&'a conductor_writes::CanonicalElectionWire>),
}

impl ElectionResolve<'_> {
    /// The batch-probe constructor: the coordinator ANSWERED, with an election
    /// or with a genuine "no election".
    pub fn observed(
        election: Option<&conductor_writes::CanonicalElectionWire>,
    ) -> ElectionResolve<'_> {
        ElectionResolve::Resolved(Answer::observed_absence(election))
    }

    /// The no-answer constructor: unattempted, backpressured, or unreachable.
    /// Absence was never observed.
    pub fn unresolved() -> ElectionResolve<'static> {
        ElectionResolve::Resolved(Answer::Unreachable)
    }
}

/// Everything the peer-hint arm needs. Both fields are absent at boot (the
/// one-shot re-anchor pass runs before P2P discovery), which correctly degrades
/// the pre-flight to the local-DHT arm alone.
pub struct AdoptContext<'a> {
    pub hints: &'a PeerHeadHints,
    pub fetcher: Option<&'a dyn HeadRecordFetcher>,
    /// `config.contest_two_way_declared` — the per-pod switch for the CONTEST
    /// arm. `false` restores the pre-2026-08-02 `Hold`.
    pub contest_enabled: bool,
    /// `content_id → advertising peer` for ids this sweep classified as
    /// ACTIONABLE anchor-divergence (a peer advertised a different non-empty
    /// anchor; the id passed the MissLedger and is not declared-partitioned).
    /// The SPIN-discharge evidence — sibling of [`Self::hints`], one authority
    /// plane down: an anchor advertisement, never a declaration.
    pub divergent_advertisers: &'a HashMap<String, String>,
    /// `config.contest_undeclared_divergence` — the per-pod switch for the
    /// SPIN-discharge arm. `false` restores the pre-2026-08-14 MissLedger spin.
    pub contest_divergent_enabled: bool,
}

impl AdoptContext<'_> {
    /// The boot-time shape: no peer inventory yet, no transport to fetch with.
    ///
    /// `contest_enabled: false` is not a policy choice — with no hints there is
    /// nothing to contest, and the boot pass runs before P2P discovery. The
    /// divergence map is empty for the same reason.
    pub fn none() -> AdoptContext<'static> {
        static EMPTY: std::sync::OnceLock<PeerHeadHints> = std::sync::OnceLock::new();
        static EMPTY_DIVERGENT: std::sync::OnceLock<HashMap<String, String>> =
            std::sync::OnceLock::new();
        AdoptContext {
            hints: EMPTY.get_or_init(PeerHeadHints::new),
            fetcher: None,
            contest_enabled: false,
            divergent_advertisers: EMPTY_DIVERGENT.get_or_init(HashMap::new),
            contest_divergent_enabled: false,
        }
    }
}

/// The decision rule, as a pure total function.
///
/// Pure on purpose: this is the part that got the live behaviour wrong, and it
/// is the part that must be readable in isolation and exhaustively tested
/// without a conductor, a pool, or a peer.
///
/// | own conductor canonical | peer declares | local declared | local election | ⇒ |
/// |---|---|---|---|---|
/// | yes | – | – | – | [`HeadDecision::AdoptLocal`] |
/// | no | yes | no | – | [`HeadDecision::AdoptPeer`] |
/// | no | yes | yes | no | [`HeadDecision::ContestPeer`] |
/// | no | yes | yes | yes | [`HeadDecision::Hold`] |
/// | no | no | yes | – | [`HeadDecision::Hold`] |
/// | no | no | no | – | [`HeadDecision::Author`] |
///
/// Rulings worth naming:
///
/// - **Peer-declares + local-declared + NO local election ⇒ ContestPeer.** Still
///   not an adopt: Rust does not order the two declarations, and nothing is
///   stamped. It mints the DHT evidence so the zome's arbiter has a set to elect
///   on. See [`HeadDecision::ContestPeer`].
/// - **Peer-declares + local-declared + local election ⇒ Hold.** An election has
///   already run and this row is obeying it. Contesting again would re-mint a
///   link every sweep forever — this is the QUIESCENCE gate, and it is why a
///   converged corpus mints exactly zero.
/// - **Nothing declared anywhere ⇒ Author.** Today's behaviour, unchanged, and
///   now safe: under [`content_diesel::HeadElection::PreserveExistingDeclaration`]
///   a self-authored root is NOT a declaration, so any later declaration — from
///   any peer, at any time — wins outright.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeadDecision {
    /// The own conductor holds a canonical head — adopt it, never author.
    AdoptLocal,
    /// A peer advertises a declaration and this row has none — verified-fetch
    /// then declare through the own conductor.
    AdoptPeer,
    /// BOTH sides declare, and this row's declaration has no canonical election
    /// behind it — mint a canonical-head link naming the PEER's head so the DHT
    /// has an election to run.
    ///
    /// ## Why this is not "racing to overwrite"
    ///
    /// The old rule held here, on the reasoning that two declarations cannot be
    /// ordered from Rust. That reasoning is still exactly right — and it is why
    /// this arm does NOT order them. It writes no row. It adds one candidate to
    /// the DHT link set and lets `content_store::select_canonical_winner` decide,
    /// which is deterministic (tier, then notarized link clock, then link-hash
    /// tiebreak) and identical on every peer.
    ///
    /// Two peers contesting each other therefore CONVERGE rather than flap: A
    /// mints a link naming B's head, B mints one naming A's, and both then
    /// resolve the same winner from the same set. The old `Hold` was safe but
    /// terminal — with no minter, the arbiter had an empty set and never ran, so
    /// the class could not converge by any path.
    ContestPeer,
    /// This row already carries a declaration that an election stands behind (or
    /// no peer is advertising anything) — neither adopt, contest, nor author.
    Hold,
    /// NOTHING is declared anywhere, but a peer advertised a DIVERGENT anchor
    /// for this id THIS sweep while the own conductor keeps answering this
    /// node's own root — the SPIN class (2026-08-14): two genuine roots, no
    /// declaration on either side, so the election has an empty candidate set
    /// and no other arm can ever supply it. `ContestPeer` requires a local
    /// declaration; ghost-decay requires an observed-absent conductor; and the
    /// head-record responder's hint-inflation guard deliberately refuses to
    /// serve an undeclared anchor as a head, so peer-head candidacy is
    /// structurally impossible for this class.
    ///
    /// The exit is SYMMETRIC SELF-CANDIDACY: nominate THIS node's own
    /// genuinely-held head as one canonical-election candidate. Same safety
    /// story as [`Self::ContestPeer`]'s self-head fallback — nothing is
    /// stamped, the peer's sweep mints its own competing candidate, and
    /// `select_canonical_winner` picks deterministically from the set; both
    /// projections then obey the winner through the ordinary canonical heal.
    ContestDivergent,
    /// Nothing is declared anywhere — mint a (non-declaring) root.
    Author,
}

/// DECLARE-STORM GATE, as a pure predicate: would declaring `incoming` actually
/// MOVE this row's declaration?
///
/// The pre-flight runs over every candidate of every sweep, and a zome declare
/// is a source-chain write plus a DHT link — orders of magnitude more expensive
/// than the SQL read that avoids it. Without this gate a CONVERGED corpus would
/// re-declare its already-correct head on every sweep forever: a cure that
/// installs a permanent write storm is not a cure. With it, a converged corpus
/// issues ZERO declares per sweep.
///
/// `None` (undeclared) always moves — that is the fill case the peer arm exists
/// for. Whitespace is trimmed on both sides so a stray-padding round-trip cannot
/// read as a difference and re-declare on every sweep.
pub fn declaration_would_move(current: Option<&str>, incoming: &str) -> bool {
    match current {
        None => true,
        Some(c) => c.trim() != incoming.trim(),
    }
}

/// ADOPT-BEFORE-AUTHOR, as a pure predicate: may this call site ask the
/// conductor to bypass its target-independent no-chain gate?
///
/// The mirror of `content_store::classify_chain_gate` on the storage side, and
/// the ONE place the two conditions are joined — so "the flag is on" can never
/// drift apart from "we actually hold evidence to send". A `true` from here
/// always travels WITH the bytes that justify it.
///
/// `node_flag_enabled` is `config::adopt_before_author_enabled()` (the OnceLock
/// mirror, default FALSE); `carried_bytes_present` means the view-federation
/// fetch answered `Present` **with** `record: Some(_)` — a hash-only answer is
/// not evidence and must not set the param.
///
/// Taking both as arguments rather than reading the config inside is what makes
/// this testable without process state, and is the same discipline as
/// [`decide_head_action`]'s `contest_enabled`.
pub fn adopt_before_author_param(node_flag_enabled: bool, carried_bytes_present: bool) -> bool {
    node_flag_enabled && carried_bytes_present
}

/// The retry payload for the adopt-before-author arms, cloned ONLY when the arm
/// could actually use it.
///
/// The dormant path must cost nothing — not a conductor call, not a metric, and
/// not a multi-KB `Vec<u8>` clone per contested id per sweep. Returning `None`
/// while the flag is off is what makes "behaviour-identical" also mean
/// "allocation-identical".
fn adopt_retry_bytes(carried_record: Option<&Vec<u8>>) -> Option<Vec<u8>> {
    let enabled = crate::config::adopt_before_author_enabled();
    if !adopt_before_author_param(enabled, carried_record.is_some()) {
        return None;
    }
    carried_record.cloned()
}

/// See [`HeadDecision`] for the table this implements.
///
/// `local_has_canonical_election` is `content.canonical_declared_at IS NOT NULL`
/// — "an election has already run and this row is obeying its outcome". It is
/// the QUIESCENCE input: it alone separates a contestable stalemate from a
/// settled one, and it is what makes a converged corpus mint zero links.
///
/// `contest_enabled` is the per-pod config switch. `false` collapses
/// `ContestPeer` back into `Hold`, i.e. exactly the pre-2026-08-02 behaviour.
///
/// `peer_divergent_anchor` (2026-08-14, SPIN discharge) is "a peer advertised a
/// DIFFERENT non-empty anchor for this id THIS sweep, and the per-pod
/// `contest_undeclared_divergence` switch is on" — both facts folded at the
/// call site, so `false` here is byte-for-byte the pre-flag table. It extends
/// the table with exactly one row and changes no existing one:
///
/// | own conductor canonical | peer declares | local declared | local election | peer divergent anchor | ⇒ |
/// |---|---|---|---|---|---|
/// | no | no | no | no | yes | [`HeadDecision::ContestDivergent`] |
pub fn decide_head_action(
    conductor_canonical: bool,
    peer_declared: bool,
    local_declared: bool,
    local_has_canonical_election: bool,
    contest_enabled: bool,
    peer_divergent_anchor: bool,
) -> HeadDecision {
    if conductor_canonical {
        HeadDecision::AdoptLocal
    } else if peer_declared && !local_declared {
        HeadDecision::AdoptPeer
    } else if local_declared {
        // Both sides declare. Contest ONLY when no election stands behind this
        // row — once one does, the row is obeying the DHT and re-minting would
        // be a permanent write storm against an already-settled question.
        if peer_declared && !local_has_canonical_election && contest_enabled {
            HeadDecision::ContestPeer
        } else {
            HeadDecision::Hold
        }
    } else if peer_divergent_anchor && !local_has_canonical_election && contest_enabled {
        // SPIN discharge: undeclared on both sides, two genuine roots, live
        // divergence evidence this sweep — supply the election by
        // self-candidacy. Gated on the election exactly like `ContestPeer`
        // (quiescence), and on the same family switch.
        HeadDecision::ContestDivergent
    } else {
        HeadDecision::Author
    }
}

/// GHOST-DECLARATION DECAY — may a `Hold`/`ContestPeer` decision be downgraded
/// to `Author` because the declaration evidence holding it is POSITIVELY dead?
///
/// The residual this exits (batch-3, diagnosed live 2026-08-10): SQL
/// projections whose declared heads outlived every conductor incarnation.
/// Nobody can back the declaration — contest cannot mint (`no_local_chain`),
/// adoption has no bytes to carry, obey sees no election — yet the
/// anti-self-election guard keeps blocking the author path on the strength of
/// that unbackable declaration, forever (the all-`held` ghost-witness sweeps).
///
/// Evidence discipline (C1/C4): every input is a POSITIVE observation, never a
/// missing answer —
/// - `local_head_observed_absent`: this conductor ANSWERED `resolve_content_
///   head` with none, this sweep ([`LocalResolve::Resolved`]+[`Answer::Absent`]
///   only; `Unreachable`/`Probe` never qualify).
/// - `peer_declared_now`: a peer is advertising a declared head for the id in
///   THIS sweep's hint window. Without a live advertiser the ledger evidence
///   below is stale hearsay (its peer may be offline or recovered), and there
///   is no declaration to falsify — decay never fires off the ledger alone.
/// - `evidence_absent_stood`: the advertising peer's responder STATED its own
///   conductor holds no record for the head it advertises, and that verdict
///   has STOOD un-refreshed for the decay dwell
///   ([`crate::services::contest_backoff::evidence_absent_stood`],
///   [`crate::config::ghost_decay_min_dwell`]). One sweep's answer is a fact
///   about one peer's local cache (the responder's get is LOCAL-strategy — a
///   clean no-record can also mean not-yet-gossiped, full-arc law); only its
///   persistence across the dwell upgrades it to a standing fleet verdict.
/// - `!local_has_canonical_election`: a row obeying a DHT election is settled
///   and is never decayed.
///
/// The flag keeps WHETHER a fleet replaces phantom declarations an operator
/// decision ([`crate::config::ghost_declaration_decay_enabled`]); dormant is
/// behaviour-identical. A later real canonical declaration (Declare mode)
/// outranks the authored root, which bounds the cost of a wrong decay to one
/// extra election candidate.
///
/// **Concerns:** C1 (authors only over a positively-falsified declaration,
/// flag-gated); C3 (this IS the liveness exit for the phantom-declaration
/// class); C4 (only observed absences count — an unreachable conductor, an
/// unanswered fetch, or a merely-fresh no-record answer never decays
/// anything); C6a (pure predicate, no loop).
///
/// **Contract tests:** [`tests::ghost_decay_is_dormant_by_default`],
/// [`tests::ghost_decay_requires_every_positive_observation`].
pub fn ghost_decay_authorizes_author(
    decay_enabled: bool,
    local_head_observed_absent: bool,
    local_has_canonical_election: bool,
    peer_declared_now: bool,
    evidence_absent_stood: bool,
) -> bool {
    decay_enabled
        && local_head_observed_absent
        && !local_has_canonical_election
        && peer_declared_now
        && evidence_absent_stood
}

/// WHICH leg of [`ghost_decay_authorizes_author`] blocked — `None` when the arm
/// authorizes. The observability twin of the predicate, over the same inputs.
///
/// A conjunction that refuses says nothing about WHY on its own, and the four
/// refusal causes call for four different operator actions: `disabled` means the
/// flag never reached this pod (redeploy), `local_election_stands` means the row
/// is settled (nothing to do), `no_live_advertiser` means no peer is advertising
/// the declaration this sweep (a SUPPLY question — the standing state of the
/// DHT-anchor-gap corpus), and `evidence_not_stood` means the dwell is still
/// running (wait). Without this the four are one silence, and a pod draining no
/// phantoms is indistinguishable from a pod that was never asked to.
///
/// Legs are reported in the predicate's own evaluation order, so the label names
/// the FIRST unmet condition rather than an arbitrary one of several; that order
/// is also triage order (a disabled pod's other legs are not worth reading).
///
/// **Concerns:** C8 (observability-per-decision — every refusal is counted under
/// a typed label); C6a (pure, allocation-free, no loop — safe to call per row on
/// a full-corpus sweep).
///
/// **Contract test:** [`tests::ghost_decay_blocked_leg_agrees_with_the_predicate`]
/// pins the two functions to the same verdict across the whole 2^5 input space,
/// so the meter can never disagree with the behaviour it explains.
pub fn ghost_decay_blocked_leg(
    decay_enabled: bool,
    local_head_observed_absent: bool,
    local_has_canonical_election: bool,
    peer_declared_now: bool,
    evidence_absent_stood: bool,
) -> Option<crate::metrics::GhostDecayBlockedLeg> {
    use crate::metrics::GhostDecayBlockedLeg as Leg;
    if !decay_enabled {
        return Some(Leg::Disabled);
    }
    if !local_head_observed_absent {
        return Some(Leg::LocalNotObservedAbsent);
    }
    if local_has_canonical_election {
        return Some(Leg::LocalElectionStands);
    }
    if !peer_declared_now {
        return Some(Leg::NoLiveAdvertiser);
    }
    if !evidence_absent_stood {
        return Some(Leg::EvidenceNotStood);
    }
    None
}

/// What the caller should do with this id after the pre-flight ran.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdoptOutcome {
    /// A canonical head is now stamped on the row. SKIP the author path.
    Adopted,
    /// The row already carries a declaration (or a declaration this pre-flight
    /// tried to move was refused as not-provably-forward). SKIP the author path
    /// and leave it to the canonical channels.
    Held,
    /// A canonical-head link was minted naming the PEER's head — one candidate
    /// added to the DHT election. The local row is deliberately UNCHANGED: the
    /// winner is decided on the DHT and arrives later through the ordinary
    /// canonical heal path. SKIP the author path.
    Contested,
    /// Nothing adoptable. Proceed with the caller's existing author path.
    Author,
    /// A peer's head is adoptable but this conductor has no local chain to hang
    /// the declaration on. Run the author path, then call
    /// [`finish_author_then_adopt`] with these values.
    AuthorThenAdopt {
        head_action_hash: String,
        carried_record: Option<Vec<u8>>,
        peer_id: String,
    },
}

/// Pre-flight for ONE content id. Never errors: every failure degrades to
/// [`AdoptOutcome::Author`] (or `Held`) and is retried on the next sweep.
///
/// `priced` is the trust gradient's verdict for THIS id
/// (`trust::adoption_pricing`). [`PricedVerification::inert`] — full chain — is
/// what every call site passed before Q7 and what the boot and ghost sweeps
/// still pass, so the parameter is a visible, greppable statement of which
/// paths are priced rather than an invisible default. The ONLY thing a
/// non-inert verdict may change is verification DEPTH; it never widens write
/// authority, and the declare→stamp path below is identical either way.
///
/// The `too_many_arguments` allow is a stated decision, not a suppression: the
/// three "what the caller already paid for" parameters (`local_resolve`,
/// `election_resolve`, `priced`) are cohesive and want a `PreflightEvidence<'a>`
/// bundle, which is the right follow-up — but bundling them rewrites this
/// crate's most-referenced signature and its prose, so it is a deliberate
/// standalone change rather than a rider on the pricing landing. The same
/// allow appears at ten comparable orchestration boundaries in this crate
/// (`reconcile::custody`, `services::replicates_dwelling_service`, ...).
#[allow(clippy::too_many_arguments)]
pub async fn try_adopt_canonical_head(
    hc: &Arc<HcClient>,
    pool: &DbPool,
    ctx: &AppContext,
    id: &str,
    local_resolve: LocalResolve<'_>,
    election_resolve: ElectionResolve<'_>,
    adopt: &AdoptContext<'_>,
    priced: PricedVerification,
) -> AdoptOutcome {
    // (0) What does this row already claim? A pool failure is not a licence to
    // author — treat an unreadable row as declared (Hold) so a transient DB
    // problem can never mint a competing root.
    // Read the declaration AND whether an election stands behind it in one
    // borrow. A row obeying an election is settled (Hold); a row declaring on
    // its own authority alone is contestable.
    let (local_declared, local_election) = match pool.get() {
        Ok(mut conn) => match content_diesel::declared_head_with_election(&mut conn, ctx, id) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(
                    content_id = %id, error = %e,
                    "adopt-before-author: could not read the local declaration — holding \
                     (a transient DB error must never authorize minting a competing root)"
                );
                return AdoptOutcome::Held;
            }
        },
        Err(e) => {
            tracing::warn!(
                content_id = %id, error = %e,
                "adopt-before-author: db conn unavailable — holding"
            );
            return AdoptOutcome::Held;
        }
    };

    // (1) LOCAL-DHT ARM. Reuse a resolve the caller already paid for.
    let probed: Option<ContentHeadWire> = match local_resolve {
        LocalResolve::Resolved(_) => None,
        LocalResolve::Probe => match conductor_writes::call_resolve_content_head(hc, id).await {
            Ok(head) => head,
            Err(e) => {
                // A conductor that will not answer cannot be shown to hold a
                // canonical head; fall through with "not canonical" rather than
                // aborting the whole sweep.
                tracing::debug!(
                    content_id = %id, error = %e,
                    "adopt-before-author: own-conductor resolve failed; continuing to the peer arm"
                );
                None
            }
        },
    };
    // The named C4 collapse: `Absent` (conductor answered, holds nothing) and
    // `Unreachable` (conductor never answered) both foreclose `AdoptLocal` and
    // nothing else — identical today, and `LocalResolve::head` is the ONE place
    // that identity is written down. A future arm that must tell them apart
    // matches on `LocalResolve::Resolved` rather than calling `head()`.
    let head: Option<&ContentHeadWire> = match local_resolve {
        LocalResolve::Resolved(_) => local_resolve.head(),
        LocalResolve::Probe => probed.as_ref(),
    };
    let canonical_head = head.filter(|h| h.canonical);
    let hint = adopt.hints.get(id);

    // (1b) ELECTION-OBEY ARM. Scoped to the CONDUCTOR-MISSING class — no head
    // answer of any kind. Those rows are the ones whose `Ok(None)` may be hiding
    // a visible election (see `try_obey_visible_election`); a row that DID get an
    // answer is already served by the arms below, so probing here would spend a
    // conductor call per row per sweep for nothing.
    //
    // Runs BEFORE the decision rule on purpose: obeying an election the DHT has
    // already settled is strictly better than contesting it again, and a row this
    // arm moves is left holding the elected head with its ordering recorded — so
    // the very next sweep sees `canonical_declared_at` set and quiesces.
    if should_probe_election(head.is_some()) {
        if let Some(outcome) =
            try_obey_visible_election(hc, pool, ctx, id, hint, adopt.fetcher, election_resolve)
                .await
        {
            return outcome;
        }
    }

    // SPIN-discharge evidence: a peer advertised a DIVERGENT non-empty anchor
    // for this id THIS sweep AND the per-pod switch is on. Folded into one bool
    // here so `decide_head_action` stays pure and OFF stays byte-identical.
    let divergent_peer = adopt
        .divergent_advertisers
        .get(id)
        .filter(|_| adopt.contest_divergent_enabled);
    let decision = decide_head_action(
        canonical_head.is_some(),
        hint.is_some(),
        local_declared.is_some(),
        local_election,
        adopt.contest_enabled,
        divergent_peer.is_some(),
    );

    // GHOST-DECLARATION DECAY (operator-reserved, default OFF). A Hold/Contest
    // produced by a declaration nobody can back is a permanent wedge, not a
    // safety property — see [`ghost_decay_authorizes_author`] for the evidence
    // discipline. Checked here, after the obey arm, so a visible election is
    // always obeyed in preference to authoring.
    if matches!(decision, HeadDecision::Hold | HeadDecision::ContestPeer) {
        let local_head_observed_absent =
            matches!(local_resolve, LocalResolve::Resolved(Answer::Absent));
        let evidence_absent_stood = crate::services::contest_backoff::evidence_absent_stood_for(
            id,
            crate::config::ghost_decay_min_dwell(),
            crate::services::contest_backoff::BackoffWindows::from_config(),
        );
        // The refusal path is metered too: a pod that decays nothing must say
        // which leg held it, or "no phantoms drained here" is indistinguishable
        // from "the arm was never live here" (the 2026-08-11 soak read, where
        // three pods sat at zero decay-authors over 17h with no way to tell
        // starvation from a missing flag).
        if let Some(leg) = ghost_decay_blocked_leg(
            crate::config::ghost_declaration_decay_enabled(),
            local_head_observed_absent,
            local_election,
            hint.is_some(),
            evidence_absent_stood,
        ) {
            crate::metrics::inc_ghost_decay_blocked(leg);
            tracing::debug!(
                target: "elohim_storage::head_adoption",
                content_id = %id,
                decision = ?decision,
                blocked_leg = ?leg,
                decay_enabled = crate::config::ghost_declaration_decay_enabled(),
                local_head_observed_absent,
                local_election,
                peer_declared_now = hint.is_some(),
                evidence_absent_stood,
                "ghost-declaration-decay: considered and refused — the row stays held \
                 (see blocked_leg for which evidence is missing)"
            );
        } else {
            crate::metrics::inc_ghost_declaration_decay_author();
            tracing::warn!(
                target: "elohim_storage::head_adoption",
                content_id = %id,
                declared = ?local_declared,
                decision = ?decision,
                "ghost-declaration-decay: the declaration holding this row is positively \
                 unbackable (own conductor observed empty this sweep; a live advertiser's \
                 stated no-record verdict has stood the decay dwell) — releasing the author \
                 path so the witness sweep can mint a provable root; a later real canonical \
                 declaration still wins outright"
            );
            return AdoptOutcome::Author;
        }
    }

    match decision {
        HeadDecision::AdoptLocal => {
            // `canonical_head.is_some()` is what produced this arm.
            let head = canonical_head.expect("AdoptLocal implies a canonical head");
            adopt_local(pool, ctx, id, head)
        }
        HeadDecision::AdoptPeer => {
            let hint = hint.expect("AdoptPeer implies a hint");
            adopt_peer(hc, pool, ctx, id, hint, adopt.fetcher, priced).await
        }
        HeadDecision::ContestPeer => {
            let hint = hint.expect("ContestPeer implies a hint");
            contest_peer(hc, ctx, id, hint, adopt.fetcher, local_declared.as_deref()).await
        }
        HeadDecision::ContestDivergent => {
            // Self-candidacy needs a head to nominate. The SPIN class carries
            // one by construction (the own conductor answered `Refreshed`
            // against it), but this pre-flight is also reached from the ghost
            // and boot sweeps where the local answer can be absent — degrade to
            // the caller's existing path there rather than minting blind.
            match head {
                Some(own) => {
                    contest_divergent(
                        hc,
                        id,
                        own,
                        divergent_peer.map(String::as_str).unwrap_or("unknown"),
                    )
                    .await
                }
                None => AdoptOutcome::Author,
            }
        }
        HeadDecision::Hold => {
            tracing::debug!(
                content_id = %id,
                declared = ?local_declared,
                "adopt-before-author: row already declared — neither adopting nor authoring"
            );
            AdoptOutcome::Held
        }
        HeadDecision::Author => AdoptOutcome::Author,
    }
}

/// Stamp a canonical head this node's OWN conductor resolved.
///
/// Stamp mode is [`StampMode::HealCanonical`], NOT `Declare`. This runs inside
/// the heal sweeps, and a conductor that has not yet integrated a newer
/// cross-root link answers with the OLD canonical record — canonical, yet stale.
/// An unconditional `Declare` here moved the head BACKWARDS in the 2026-07-12
/// regression (`elohim-host-landing` converged at edge #1187's seam-smoke and
/// healed back to the superseded head by #1188). `HealCanonical` FILLS an
/// undeclared row unconditionally — which is the adopt case this arm exists for
/// — and only MOVES a declared one with proof of forward ordering.
fn adopt_local(pool: &DbPool, ctx: &AppContext, id: &str, head: &ContentHeadWire) -> AdoptOutcome {
    let mut conn = match pool.get() {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(content_id = %id, error = %e, "adopt-before-author: db conn for local stamp");
            return AdoptOutcome::Held;
        }
    };
    match content_diesel::stamp_declared_head_mode(
        &mut conn,
        ctx,
        id,
        head.head_action_hash.as_str(),
        Some(head.declared_at),
        None,
        StampMode::HealCanonical,
        // The DHT election behind this answer — what the guard actually compares.
        head.canonical_ordering(),
    ) {
        Ok(StampOutcome::Stamped) => {
            crate::metrics::inc_content_head_adopted();
            tracing::warn!(
                target: "elohim_storage::head_adoption",
                content_id = %id,
                head = %head.head_action_hash,
                "adopt-before-author: ADOPTED a canonical head from the own conductor \
                 (no local root minted)"
            );
            AdoptOutcome::Adopted
        }
        // Already holding this head, or holding one this answer cannot be shown
        // to supersede: in every case the row HAS a canonical head and must not
        // be re-rooted.
        Ok(_) => AdoptOutcome::Held,
        Err(e) => {
            tracing::warn!(content_id = %id, error = %e, "adopt-before-author: local stamp failed");
            AdoptOutcome::Held
        }
    }
}

/// Verified-fetch a peer's head record, then declare it through the OWN
/// conductor. Returns [`AdoptOutcome::AuthorThenAdopt`] when the conductor
/// refuses for lack of a local chain.
///
/// # SKIP-POINT 2 (Q7): the per-id `ContentHeadRecord` verify RPC
///
/// When `priced` is `AcceptWithProvenance` the evidence fetch below is SKIPPED
/// entirely — one view-federation round-trip per candidate, plus up to
/// `config::evidence_fallback_max_alternates` more on the courier ladder. The
/// declare then runs record-less, on the peer's ADVERTISED head hash.
///
/// **What that does and does not give up.** It gives up the peer serving the
/// bytes that PROVE the advertised action, i.e. this node's own re-derivation
/// of the courier's claim. It does NOT give up verification: the record-less
/// declare succeeds only if this conductor can retrieve the target itself, and
/// the declaration is still minted by `declare_canonical_head` and arbitrated
/// by `select_canonical_winner` on the DHT. The write path is byte-identical —
/// `StampMode::Declare` over the CONDUCTOR's answer, never over the hint. That
/// is exactly what "the declared grant lets us accept the peer's provenance
/// instead of re-deriving it" means, and it is why the skip is licensed by a
/// declared stage rather than by a config flag.
///
/// **Bounded by the path it replaces.** If the conductor cannot retrieve the
/// target (`ERR_NOT_RETRIEVABLE`) or has no local chain (`ERR_NO_LOCAL_CHAIN`),
/// the skip would have turned a convergence into a root-mint — strictly worse
/// than full chain. So it falls back ONCE: buy the evidence that was skipped
/// and re-offer the declare. Worst case is therefore the full-chain cost, never
/// more, and the common case (a fixture corpus the conductor already holds)
/// pays nothing.
async fn adopt_peer(
    hc: &Arc<HcClient>,
    pool: &DbPool,
    ctx: &AppContext,
    id: &str,
    hint: &PeerHeadHint,
    fetcher: Option<&dyn HeadRecordFetcher>,
    priced: PricedVerification,
) -> AdoptOutcome {
    if priced.accepts_with_provenance() {
        return adopt_peer_with_provenance(hc, pool, ctx, id, hint, fetcher).await;
    }
    // Verified fetch: ask the ADVERTISING peer for the serialized Record behind
    // its head. Absent fetcher (boot pass) or an old peer that cannot serve one
    // both degrade to a record-less declare, which succeeds iff this conductor
    // can already retrieve the target itself.
    //
    // The evidence STATE is discarded here on purpose: this arm's row already
    // carries a chain (that is what routed it here rather than to the contest
    // arm), so a missing-bytes cause changes nothing it would do. It is still
    // COUNTED inside `resolve_peer_evidence` — the meter's denominator is every
    // ask this node makes, not only the asks one arm acts on.
    //
    // ADVERTISER DIVERSITY applies here too, and matters MORE than the state
    // does: this arm can only declare a head it has bytes for, so a starved
    // hinted advertiser is exactly the wall a second courier removes.
    let (carried, _evidence) = resolve_peer_evidence(fetcher, hint, id).await;
    // The SERVED hash wins over the advertised one: it is the action the bytes
    // actually prove. Falling back to the hint keeps the record-less path working.
    let (head_action_hash, carried_record) = match carried {
        Some(c) => (c.head_action_hash, c.record),
        None => (hint.head_action_hash.clone(), None),
    };

    declare_peer_head(hc, pool, ctx, id, &head_action_hash, carried_record, hint).await
}

/// The `AcceptWithProvenance` half of [`adopt_peer`] — declare the peer's
/// ADVERTISED head through the own conductor with NO evidence round-trip, and
/// fall back to the full-chain path exactly once if the conductor cannot serve
/// the declaration itself.
///
/// Kept as its own function rather than as branches inside `adopt_peer` so the
/// compressed path is readable end-to-end and so the fallback cannot be
/// mistaken for a retry ladder: it is one re-offer of the SAME declare with the
/// evidence the fast path chose not to buy.
async fn adopt_peer_with_provenance(
    hc: &Arc<HcClient>,
    pool: &DbPool,
    ctx: &AppContext,
    id: &str,
    hint: &PeerHeadHint,
    fetcher: Option<&dyn HeadRecordFetcher>,
) -> AdoptOutcome {
    let outcome = declare_peer_head(
        hc,
        pool,
        ctx,
        id,
        &hint.head_action_hash,
        // NO carried record — the skip. The conductor still verifies what it
        // declares; nothing is stamped from the advertised pair.
        None,
        hint,
    )
    .await;

    // `Author` / `AuthorThenAdopt` are the two outcomes that mean "this
    // conductor could not stand the declaration up on its own" — precisely the
    // cases the skipped bytes exist to serve. Everything else (Adopted, Held,
    // Contested) is a settled answer the evidence could not improve.
    if !matches!(
        outcome,
        AdoptOutcome::Author | AdoptOutcome::AuthorThenAdopt { .. }
    ) {
        return outcome;
    }

    let (carried, _evidence) = resolve_peer_evidence(fetcher, hint, id).await;
    let Some(carried) = carried else {
        // No courier could serve the bytes either — the record-less answer
        // stands, exactly as the full-chain path's own no-bytes case would.
        tracing::debug!(
            target: "elohim_storage::head_adoption",
            content_id = %id,
            from_peer = %hint.peer_id,
            "accept-with-provenance: the record-less declare did not stand and no courier \
             served the Record either — same outcome the full-chain path would have reached"
        );
        return outcome;
    };
    tracing::info!(
        target: "elohim_storage::head_adoption",
        content_id = %id,
        from_peer = %hint.peer_id,
        "accept-with-provenance: the record-less declare did not stand (this conductor could \
         not retrieve the head itself) — buying the evidence the fast path skipped and \
         re-offering the declare ONCE; total cost is bounded by the full-chain path"
    );
    declare_peer_head(
        hc,
        pool,
        ctx,
        id,
        &carried.head_action_hash,
        carried.record,
        hint,
    )
    .await
}

/// ELECTION-OBEY: move a row to the head the DHT elected, when this conductor can
/// SEE the election but cannot resolve the winner's bytes itself.
///
/// Returns `None` to mean "not applicable — carry on with the normal decision",
/// and `Some(outcome)` when this arm took responsibility for the id.
///
/// ## The class this closes
///
/// `resolve_content_head_local` answers `Ok(None)` for two unrelated reasons, and
/// the caller cannot tell them apart: no election exists, OR an election exists
/// whose target has not gossiped in. A pod in the second state is holding the
/// answer and unable to act on it — the ~24.9k/2h conductor-missing class, which
/// no amount of contesting can fix because contest SUPPLIES elections and this
/// class needs to OBEY one.
///
/// Link ops and entry ops travel independently: canonical-head links gossip on
/// the `canonical_head` StringAnchor authority, while the target Content is a
/// separate entry the conductor may simply not hold. `resolve_canonical_election`
/// reports the election as soon as the LINKS land; the winner's bytes then come
/// from a peer.
///
/// ## Why this is not "trusting a peer"
///
/// Nothing is stamped on a peer's say-so. Two independent gates must BOTH pass:
///
/// 1. **The election is read from THIS node's own conductor.** The peer does not
///    get to say which head is canonical — the DHT already decided, and we read
///    that decision locally.
/// 2. **The bytes are proven in wasm.** `validate_carried_head_record` re-derives
///    the action hash, verifies the author's signature, checks the entry↔action
///    binding, and enforces the target-id gate. A peer that serves anything other
///    than the elected action fails.
///
/// The peer is a byte courier, never an authority. And the stamp is
/// `HealCanonical` carrying the ELECTION's ordering, so
/// [`content_diesel::canonical_move_verdict`] still arbitrates — this arm cannot
/// move a row that already obeys an equal-or-newer election, and cannot move one
/// backwards. It never authors, never self-elects, and never widens `Declare`.
/// Should the election-obey arm be probed for this row?
///
/// Pure + total, matching the sibling routing predicates
/// (`gapfill_would_self_elect`, `conductor_missing_should_route_to_adopt`) — the
/// scope rule is the part worth testing without a conductor.
///
/// YES exactly when the own conductor gave NO head answer at all. That `Ok(None)`
/// is the state that may be hiding a visible election (see
/// [`try_obey_visible_election`]). A row that DID get an answer — canonical or
/// fallback — is already served by the existing arms, so probing it would spend a
/// conductor round-trip per row per sweep for nothing.
pub(crate) fn should_probe_election(head_answer_present: bool) -> bool {
    !head_answer_present
}

async fn try_obey_visible_election(
    hc: &Arc<HcClient>,
    pool: &DbPool,
    ctx: &AppContext,
    id: &str,
    hint: Option<&PeerHeadHint>,
    fetcher: Option<&dyn HeadRecordFetcher>,
    election_resolve: ElectionResolve<'_>,
) -> Option<AdoptOutcome> {
    // (0) The DENOMINATOR. Counted at entry, before any gate, because the
    // question this arm went two shifts unable to answer was "how often does a
    // probe even reach the fetch?" — and a success series alone cannot say. See
    // [`crate::metrics::CONTENT_ELECTION_OBEY_PROBE`] for how the four labels
    // read as walls.
    crate::metrics::inc_election_obey_probe(crate::metrics::ElectionObeyProbe::Attempted);

    // (1) What did the DHT elect? Read from our OWN conductor — either right
    // here (`Probe`, the single-id path) or from the BATCH answer the caller
    // already paid for (`Resolved`, the arm-4 two-phase path). Same conductor,
    // same question, same three routings; only the round-trip count differs.
    let election = match election_resolve {
        ElectionResolve::Resolved(Answer::Present(e)) => e.clone(),
        ElectionResolve::Resolved(Answer::Absent) => {
            // Identical to the `Ok(None)` arm below: no election visible,
            // nothing to obey, behaviour exactly unchanged for this id.
            crate::metrics::inc_election_obey_probe(crate::metrics::ElectionObeyProbe::NoElection);
            return None;
        }
        ElectionResolve::Resolved(Answer::Unreachable) => {
            // Identical to the `Err` arm below: the batch never answered for
            // this id (unattempted, backpressured, or extern-absent). A
            // conductor that will not answer is not evidence of anything —
            // fall through to the normal decision rather than holding the id.
            crate::metrics::inc_election_obey_probe(
                crate::metrics::ElectionObeyProbe::ResolveError,
            );
            tracing::warn!(
                target: "elohim_storage::head_adoption",
                content_id = %id,
                "election-obey: the BATCH election probe returned no answer for this id \
                 (unattempted, or a shared conductor-backpressure stop) — no election could \
                 be obeyed for this row; continuing with the normal decision, retried next sweep"
            );
            return None;
        }
        ElectionResolve::Probe => {
            match conductor_writes::call_resolve_canonical_election(hc, id).await {
                Ok(Some(e)) => e,
                // No election visible — nothing to obey. Behaviour is exactly unchanged
                // for every id in this state, which is the pre-wave-4 world. Counted,
                // though: a `no_election`-dominated series names an ELECTION-VISIBILITY
                // wall (the canonical-head links have not gossiped in), which is a
                // gossip/link-layer finding, not an obey-arm one.
                Ok(None) => {
                    crate::metrics::inc_election_obey_probe(
                        crate::metrics::ElectionObeyProbe::NoElection,
                    );
                    return None;
                }
                Err(e) => {
                    // A conductor that will not answer is not evidence of anything.
                    // Fall through to the normal decision rather than holding the id.
                    //
                    // WARN, not debug: this deployment drops `debug!` before Loki, so
                    // the level IS the observability. Two known producers of this arm,
                    // distinguishable only by the error text carried below: (a) the
                    // conductor's DB read-pool is saturated ("deadline has elapsed" =
                    // the 10s acquire_semaphore_permit timeout — C11 backpressure, seen
                    // live 2026-08-03; retryable, not a defect in the read itself); or
                    // (b) the shipped coordinator zome is not on the running conductor
                    // (the DNA hash is blind to coordinator zomes — an unknown-function
                    // error here is the didn't-land signal). Do NOT assume (b) from
                    // rate alone: the 2026-08-03 misdiagnosis assumed it while the
                    // hot-swap was proven applied 7/7 — read the error text.
                    crate::metrics::inc_election_obey_probe(
                        crate::metrics::ElectionObeyProbe::ResolveError,
                    );
                    tracing::warn!(
                        target: "elohim_storage::head_adoption",
                        content_id = %id, error = %e,
                        "election-obey: ELECTION READ FAILED — this conductor would not answer \
                         resolve_canonical_election, so no election could be obeyed for this row; \
                         continuing with the normal decision (read the error text: 'deadline has \
                         elapsed' = conductor DB-pool saturation/backpressure; unknown-function = \
                         coordinator zome not on the running conductor)"
                    );
                    return None;
                }
            }
        }
    };

    // (2) Someone has to hand us the bytes. With no peer or no transport this is
    // not a failure — there is simply nothing to try yet. It IS counted: an
    // election we can see but never have a courier for is a hint-supply wall,
    // and it is indistinguishable from "the arm is idle" without this label.
    let (hint, fetcher) = match (hint, fetcher) {
        (Some(h), Some(f)) => (h, f),
        _ => {
            crate::metrics::inc_election_obey_probe(crate::metrics::ElectionObeyProbe::NoCourier);
            return None;
        }
    };

    let winner = election.winner_target.to_string();

    // (3) Fetch. The peer answers with ITS head for this id; that is only useful
    // if it happens to BE the elected action. A mismatch is not a lie — the peer
    // may simply hold a different head — but it is not the bytes we need.
    // NAMED C4 COLLAPSE (`into_option`): "the peer holds no head" and "we never
    // reached the peer" both land on the `None` arm below, which holds and
    // retries next sweep — identical to the pre-P2.3 behaviour. The answer state
    // is logged so the two are still readable in a trace even though they route
    // the same way.
    let answer = fetcher.fetch(&hint.peer_id, id).await;
    let answer_state = answer.state().label();
    let carried = answer.into_option();
    let bytes = match &carried {
        Some(c) if c.head_action_hash.trim() == winner.trim() => c.record.clone(),
        Some(c) => {
            crate::metrics::inc_election_obey_failed("fetch");
            tracing::debug!(
                content_id = %id,
                from_peer = %hint.peer_id,
                elected = %winner,
                peer_served = %c.head_action_hash,
                "election-obey: this peer holds a different head than the DHT elected — \
                 no bytes to prove the winner with; holding"
            );
            return Some(AdoptOutcome::Held);
        }
        None => {
            crate::metrics::inc_election_obey_failed("fetch");
            tracing::debug!(
                content_id = %id,
                from_peer = %hint.peer_id,
                elected = %winner,
                answer_state,
                "election-obey: no record served for the elected head; holding, retried next sweep"
            );
            return Some(AdoptOutcome::Held);
        }
    };
    if bytes.is_none() {
        crate::metrics::inc_election_obey_failed("fetch");
        tracing::debug!(
            content_id = %id,
            from_peer = %hint.peer_id,
            elected = %winner,
            "election-obey: peer served the elected hash but carried no record bytes; holding"
        );
        return Some(AdoptOutcome::Held);
    }

    // (4) PROVE the bytes in wasm before they can touch a row.
    let proven =
        match conductor_writes::call_validate_carried_head_record(hc, id, &winner, bytes).await {
            Ok(Some(head)) => head,
            Ok(None) => {
                crate::metrics::inc_election_obey_failed("validate");
                return Some(AdoptOutcome::Held);
            }
            Err(e) => {
                crate::metrics::inc_election_obey_failed("validate");
                tracing::warn!(
                    target: "elohim_storage::head_adoption",
                    content_id = %id,
                    from_peer = %hint.peer_id,
                    elected = %winner,
                    error = %e,
                    "election-obey: a peer served bytes that do NOT prove the elected head — \
                     refusing them and holding (this is a peer serving something it cannot back)"
                );
                return Some(AdoptOutcome::Held);
            }
        };

    // (5) Stamp under the ELECTION's ordering. `HealCanonical` + the election
    // clock means `canonical_move_verdict` still decides: fill, forward-move, or
    // refuse. Proof of the bytes never becomes a licence to move backwards.
    let mut conn = match pool.get() {
        Ok(c) => c,
        Err(e) => {
            crate::metrics::inc_election_obey_failed("stamp_refused");
            tracing::warn!(content_id = %id, error = %e, "election-obey: db conn for stamp");
            return Some(AdoptOutcome::Held);
        }
    };
    let c = &proven.content;
    let patch = content_diesel::ContentProjectionPatch {
        blob_cid: c.blob_cid.clone(),
        content_size_bytes: c
            .content_size_bytes
            .map(|n| i32::try_from(n).unwrap_or(i32::MAX)),
        title: Some(c.title.clone()),
        description: Some(c.description.clone()),
        content_type: Some(c.content_type.clone()),
        content_format: Some(c.content_format.clone()),
        reach: Some(c.reach.clone()),
        metadata_json: Some(c.metadata_json.clone()),
    };
    match content_diesel::stamp_declared_head_mode(
        &mut conn,
        ctx,
        id,
        proven.head_action_hash.to_string().as_str(),
        Some(proven.declared_at),
        Some(patch),
        StampMode::HealCanonical,
        Some(election.ordering()),
    ) {
        Ok(StampOutcome::Stamped) => {
            crate::metrics::inc_election_obeyed("carried");
            crate::metrics::inc_content_head_adopted();
            tracing::warn!(
                target: "elohim_storage::head_adoption",
                content_id = %id,
                head = %winner,
                from_peer = %hint.peer_id,
                earned = election.canonical_earned,
                "election-obey: OBEYED the DHT-elected canonical head — this conductor could \
                 see the election but not the content, so a peer supplied the bytes and the \
                 zome proved them; the row now agrees with the fleet"
            );
            Some(AdoptOutcome::Adopted)
        }
        Ok(other) => {
            // Already obeying this or a newer/earned election — a correct refusal.
            crate::metrics::inc_election_obey_failed("stamp_refused");
            tracing::debug!(
                content_id = %id, outcome = ?other,
                "election-obey: the row already obeys an equal-or-newer election; no move"
            );
            Some(AdoptOutcome::Held)
        }
        Err(e) => {
            crate::metrics::inc_election_obey_failed("stamp_refused");
            tracing::warn!(content_id = %id, error = %e, "election-obey: stamp failed");
            Some(AdoptOutcome::Held)
        }
    }
}

/// Process-local record of `(id, target)` pairs this node has already
/// CANDIDATED — a candidate it minted, or tried to.
///
/// Two shapes share this ledger, and the key is what lets them:
///
/// - **SELF-candidacy** names THIS row's own declared head. The target is stable
///   across sweeps, so a re-mint would be byte-identical.
/// - **ADOPT-BEFORE-AUTHOR** (operator-reserved) names the PEER's head, minted
///   by a node with no local chain from a validated carried record. Its target
///   moves as the advertising peer's head moves — which is exactly why the key
///   is `(id, target)` and not `id`.
///
/// Without a ledger a node would re-mint the identical link every sweep during
/// the window between minting and the election projecting onto the row. Keying
/// on `(id, target)` keeps a genuinely NEW candidate mintable: if the nominated
/// head moves, that is a new candidate and must be minted. The two shapes cannot
/// collide — they nominate different targets, so they occupy different keys.
///
/// Process-local on purpose: it is a de-duplication cache, not a truth store.
/// The DURABLE quiescence gate is `canonical_declared_at` on the row (see
/// [`decide_head_action`]) — once the election projects, contest is never
/// reached at all. This only bounds the transient window, so losing it on
/// restart costs at most one redundant mint per id.
static SELF_CANDIDATE_MINTS: std::sync::OnceLock<
    std::sync::Mutex<std::collections::HashSet<String>>,
> = std::sync::OnceLock::new();

/// Hard cap on the de-dup ledger. The contested set is ~11.4k ids fleet-wide;
/// this leaves generous headroom while making unbounded growth impossible on a
/// pathological corpus. On overflow the ledger CLEARS rather than evicting
/// selectively — the cost of a clear is at most one redundant mint per id, and a
/// simple bound is worth more here than a perfect one.
const SELF_CANDIDATE_LEDGER_CAP: usize = 50_000;

/// Ledger key. Separate fn so the `(id, target)` shape is asserted in one place.
fn self_candidate_key(id: &str, target: &str) -> String {
    format!("{id}\u{1}{target}")
}

/// Claim the right to candidate `(id, target)`. Returns `true` on the FIRST
/// call for a pair and `false` afterwards — so a caller mints at most once per
/// (id, nominated head) per process, whichever shape nominated it.
///
/// The claim is taken BEFORE the declare, not after it succeeds, and that is
/// deliberate: under the fan-out sweep several tasks can reach this arm for the
/// same id concurrently, and claim-on-success would let all of them mint. The
/// cost of claiming early is that a FAILED attempt must hand the claim back —
/// see [`release_candidacy`].
fn claim_candidacy(id: &str, target: &str) -> bool {
    let ledger = SELF_CANDIDATE_MINTS.get_or_init(|| std::sync::Mutex::new(Default::default()));
    let mut guard = match ledger.lock() {
        Ok(g) => g,
        // A poisoned lock must not silently license a re-mint storm; refuse.
        Err(_) => return false,
    };
    if guard.len() >= SELF_CANDIDATE_LEDGER_CAP {
        guard.clear();
    }
    guard.insert(self_candidate_key(id, target))
}

/// Hand back a claim whose declare FAILED — the C3 half of
/// [`claim_candidacy`].
///
/// ## The permanent exclusion this closes (2026-08-02, F-B)
///
/// The claim was taken before the attempt and never released, so ONE failed
/// self-candidacy retired `(id, own_head)` for the life of the process: the next
/// sweep took the `!claim_candidacy` early-return and never called the
/// conductor again. The live fingerprint was `contest_self_head = 4` against
/// `contest_failed{fetch_none} = 603` — 603 ids that had each burned their only
/// attempt and could never nominate again without a pod restart.
///
/// That is a *permanent* exclusion with no automated exit, which C3 forbids. A
/// released claim plus a [`contest_backoff`](crate::services::contest_backoff)
/// entry converts it into a BOUNDED one: retried roughly hourly instead of
/// either never (before) or every sweep (unbounded).
fn release_candidacy(id: &str, target: &str) {
    if let Some(ledger) = SELF_CANDIDATE_MINTS.get() {
        if let Ok(mut guard) = ledger.lock() {
            guard.remove(&self_candidate_key(id, target));
        }
    }
}

/// Which candidate a contest attempt named — the label for
/// `elohim_content_canonical_links_minted_total{source}`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ContestShape {
    /// Named the PEER's head, carried over view-federation.
    PeerHead,
    /// Named THIS row's own declared head, because the peer's could not be
    /// resolved by our conductor.
    SelfHead,
}

impl ContestShape {
    fn source_label(self) -> &'static str {
        match self {
            ContestShape::PeerHead => "contest_peer_head",
            ContestShape::SelfHead => "contest_self_head",
        }
    }
}

/// CONTEST: add ONE candidate to the DHT election for a two-way declared id.
/// Never stamps the local row.
///
/// This is the supply half of contest-then-obey (see the module note). The
/// declare goes through the OWN conductor and the DHT still decides the winner —
/// evidence, not authority.
///
/// ## Two candidate shapes, and why the second exists
///
/// **PEER-HEAD (preferred).** Fetch the peer's `Record` over view-federation and
/// declare it with `carried_record`, which the zome's `validate_carried_record`
/// proves in wasm (action-hash binding, author signature, entry↔action binding)
/// before writing the link.
///
/// **SELF-HEAD (fallback).** When the conductor answers `is not retrievable` for
/// the peer's head, declare THIS row's own declared head instead. This is
/// symmetric candidacy, NOT self-election: nothing is stamped, the peer is hitting
/// the same wall from its side and will mint ITS head as the competing candidate,
/// and `select_canonical_winner` then picks deterministically from the resulting
/// set. Both projections obey the winner through the ordinary canonical heal path.
///
/// The fallback is gated on the id being LOCALLY PRESENT, and the gate is free:
/// `is not retrievable` is returned only AFTER `declare_canonical_head_inner`'s
/// first gate (`gather_content_chain(id)`) has already passed, so that error is
/// itself proof that a local chain exists. `no local chain` gets NO fallback —
/// that first gate is target-INDEPENDENT, so naming a different target would fail
/// identically. Attempting it would burn a conductor round-trip per id per sweep
/// to reproduce the same refusal.
///
/// ## Reach is not the constraint here (2026-08-02 disconfirmation)
///
/// A reach-starvation hypothesis was investigated and DISCONFIRMED in code: the
/// inventory (`list_content_anchor_inventory`) and the head-record responder
/// (`is_distribution_safe_reach`) read the SAME `DISTRIBUTION_SAFE_REACH`
/// constant, and `community` is in it. There is no advertise-but-refuse class.
/// Scoped rows are excluded symmetrically from BOTH surfaces, so they never
/// become contest candidates at all.
///
/// Self-candidacy is nonetheless the reach-SAFE shape by construction: it moves
/// no records between peers. It writes a DHT link and calls this node's own
/// conductor about this node's own head — so even if a scoped row ever reached
/// this path, no bytes would cross a peer boundary.
///
/// Every failure degrades to [`AdoptOutcome::Held`] — the pre-cure behaviour —
/// and rides the next sweep. Deliberately no retry loop: the contest set is
/// ~11.4k ids and a retry ladder inside a 200/tick budget would starve the tail.
async fn contest_peer(
    hc: &Arc<HcClient>,
    ctx: &AppContext,
    id: &str,
    hint: &PeerHeadHint,
    fetcher: Option<&dyn HeadRecordFetcher>,
    local_declared: Option<&str>,
) -> AdoptOutcome {
    let _ = ctx;
    // BACKOFF GATE, ahead of every round-trip. A previous contest for this id
    // failed PREDICTABLY (the zome's target-independent no-chain gate, or a
    // refused self-head), and nothing about re-asking can change that until the
    // conductor's holdings change. Skipping here saves BOTH the view-federation
    // fetch and the declare — the two costs that were burning the sweep budget.
    //
    // Scoped to CONTEST only, on purpose: `try_obey_visible_election` runs
    // upstream of this function and is never gated, so the arm that actually
    // converges the conductor-missing class keeps running for a backed-off id.
    //
    // `Held` is the same outcome the skipped attempt would have produced (every
    // contest failure path below returns `Held`), so every caller — the adopt
    // sweep, the ghost witness, the boot re-anchor — sees exactly what it saw
    // before, minus the wasted round-trips.
    if let Some(reason) = crate::services::contest_backoff::skip_class(
        id,
        crate::services::contest_backoff::BackoffWindows::from_config(),
    ) {
        crate::metrics::inc_contest_skipped(reason);
        tracing::debug!(
            content_id = %id,
            reason = ?reason,
            "adopt-before-author: contest skipped — this id is serving a backoff for a \
             predictable repeat failure; it becomes eligible again on window expiry or as \
             soon as an author path lands a local chain"
        );
        return AdoptOutcome::Held;
    }

    // IDEMPOTENCE, first gate: if the peer is advertising the head this row
    // already declares, there is no contest — the two sides agree and the
    // divergence is elsewhere (an anchor, not a head). Minting here would add a
    // candidate identical to our own position, every sweep, forever.
    if !declaration_would_move(local_declared, &hint.head_action_hash) {
        tracing::debug!(
            content_id = %id,
            "adopt-before-author: contest gate — the peer advertises the head this row already \
             declares, nothing to contest"
        );
        return AdoptOutcome::Held;
    }

    let fetcher_present = fetcher.is_some();
    // EVIDENCE RESOLUTION. `carried_present` below still asks "did we get
    // BYTES?" exactly as before; `evidence` additionally carries WHY not, which
    // is what the no-chain tail uses to pick a backoff window. Both non-present
    // answer states classify as `Unknown`, so the pre-P2.3 collapse is
    // preserved — the `fetch_none` / `not_retrievable` split is downstream of
    // the bytes, not of the answer state.
    let (carried, evidence) = resolve_peer_evidence(fetcher, hint, id).await;
    // "Did we get BYTES?" — not "did the peer answer?". A peer holding a row it
    // cannot resolve answers hash-only (`record: None`), which is the DOMINANT
    // shape when the advertising peer is itself conductor-missing. Counting that
    // as `carried` would label it `not_retrievable` when it is precisely
    // `fetch_none`, and those two point at different remedies.
    let carried_present = carried.as_ref().is_some_and(|c| c.record.is_some());
    // The SERVED hash wins over the advertised one (same rule as `adopt_peer`):
    // it is the action the bytes actually prove.
    let (peer_head, carried_record) = match carried {
        Some(c) => (c.head_action_hash, c.record),
        None => (hint.head_action_hash.clone(), None),
    };
    // When the declare fails for want of a resolvable target, the actionable
    // distinction is whether we HAD a record to carry. Split here so the two
    // read as different problems on the dashboard.
    let unresolvable_class = if carried_present {
        crate::metrics::ContestFailure::NotRetrievable
    } else {
        crate::metrics::ContestFailure::FetchNone
    };
    // ADOPT-BEFORE-AUTHOR retry budget, reserved BEFORE the classic declare
    // consumes the bytes. `None` whenever the flag is off or no bytes were
    // served, so the dormant path clones nothing.
    let adopt_retry = adopt_retry_bytes(carried_record.as_ref());

    // ARM 1 — PEER-HEAD CANDIDACY. `adopt_before_author: false` — this is the
    // classic attempt, and it must stay the classic attempt: the bypass may only
    // ever run AFTER the no-chain gate has actually refused, never instead of it.
    match conductor_writes::call_declare_canonical_content_head_classed(
        hc,
        id,
        peer_head.clone(),
        carried_record,
        false,
        SWEEP_DECLARE_CLASS,
    )
    .await
    {
        Ok(declared) => {
            return contested(
                ContestShape::PeerHead,
                id,
                &declared.head_action_hash,
                local_declared,
                hint,
                carried_present,
                fetcher_present,
            );
        }
        Err(e) => {
            // ADMISSION SHED — before any classification, because a shed carries
            // no coordinator verdict to classify. The gate refused LOCAL
            // capacity; the zome never ran, so neither the no-chain gate nor the
            // retrievability gate answered. Nothing is counted, nothing is
            // backed off, and the id rides the very next sweep unchanged.
            if declare_was_shed(&e) {
                tracing::debug!(
                    target: "elohim_storage::head_adoption",
                    content_id = %id,
                    from_peer = %hint.peer_id,
                    "contest declare shed at the admission gate — backpressure, not a verdict; \
                     the conductor never saw it, so the candidate stays eligible next sweep"
                );
                return AdoptOutcome::Held;
            }
            let msg = e.to_string();

            // NO LOCAL CHAIN — terminal for this sweep. The first gate in
            // `declare_canonical_head_inner` is target-independent, so no
            // fallback target can pass it. This arm must NOT author: the row
            // already carries a declaration, and authoring here is exactly the
            // self-election the module exists to stop.
            if msg.contains(ERR_NO_LOCAL_CHAIN) {
                // Counted FIRST and unconditionally, even when the bypass below
                // then succeeds. This series is the DENOMINATOR of the
                // convergence math: `minted{source="adopt_before_author"}` over
                // `contest_failed{class="no_local_chain"}` is the share of the
                // residual the operator flip actually converts. Skipping the
                // count on success would make the flag look free and the class
                // look smaller than it is.
                crate::metrics::inc_contest_failed(crate::metrics::ContestFailure::NoLocalChain);

                // ADOPT-BEFORE-AUTHOR (operator-reserved, default OFF). The ONE
                // arm that can move a both-sides-missing id. Returns `None` when
                // dormant, when no bytes were served, when this node already
                // nominated this target, or when the zome refused the evidence —
                // and every one of those falls through to the unchanged tail
                // below, so the dormant path is the pre-flag path exactly.
                if let Some(outcome) = try_adopt_before_author(
                    hc,
                    id,
                    &peer_head,
                    adopt_retry,
                    hint,
                    carried_present,
                    fetcher_present,
                )
                .await
                {
                    return outcome;
                }

                // BACKOFF, recorded at the one site that can prove the class:
                // this gate is target-independent (`gather_content_chain(id)`),
                // so the verdict is a property of the ID, not of this peer or
                // this target — which is why the ledger keys on id alone.
                //
                // EVIDENCE-SUPPLY SPLIT (2026-08-03). Two ids that both land
                // here are in genuinely different states, and re-asking them at
                // the same rate wastes the sweep on the hopeless half: one has
                // bytes on a saturated peer (they arrive when the conductor
                // catches up — an hour is a fair bet), the other has bytes
                // nowhere at all (nothing changes until content enters the
                // fleet). `evidence_backoff_class` picks the window; BOTH are
                // deferrals that expire on their own, so the second class
                // self-heals the moment its bytes appear.
                let skip_class = evidence_backoff_class(
                    evidence,
                    crate::config::evidence_absent_backoff_window(),
                );
                crate::services::contest_backoff::note(id, skip_class);
                tracing::info!(
                    target: "elohim_storage::head_adoption",
                    content_id = %id,
                    from_peer = %hint.peer_id,
                    carried = carried_present,
                    fetcher = fetcher_present,
                    evidence = evidence.map(|e| e.label()).unwrap_or("not_asked"),
                    backoff = skip_class.label(),
                    "adopt-before-author: contest could not be minted — this conductor holds no \
                     chain for the id at all, so no candidate of any shape can be declared; \
                     holding, retried after the backoff named above"
                );
                return AdoptOutcome::Held;
            }

            // NOT RETRIEVABLE — the chain EXISTS (that gate passed) but the
            // peer's head action cannot be resolved here. This is exactly the
            // subclass self-candidacy answers.
            if !msg.contains(ERR_NOT_RETRIEVABLE) {
                crate::metrics::inc_contest_failed(crate::metrics::ContestFailure::DeclareError);
                note_declare_error_backoff(id);
                tracing::warn!(
                    target: "elohim_storage::head_adoption",
                    content_id = %id,
                    from_peer = %hint.peer_id,
                    carried = carried_present,
                    fetcher = fetcher_present,
                    error = %msg,
                    "adopt-before-author: contest declare failed — holding, retried after the \
                     contest backoff"
                );
                return AdoptOutcome::Held;
            }
        }
    }

    // ARM 2 — SELF-HEAD CANDIDACY (the not-retrievable subclass).
    let Some(own_head) = local_declared.filter(|h| !h.trim().is_empty()) else {
        // Nothing of our own to nominate. (Unreachable via `decide_head_action`,
        // which only routes here when the row IS declared — kept explicit rather
        // than unwrapped so a future caller cannot turn it into a panic.)
        crate::metrics::inc_contest_failed(unresolvable_class);
        tracing::info!(
            target: "elohim_storage::head_adoption",
            content_id = %id,
            from_peer = %hint.peer_id,
            carried = carried_present,
            fetcher = fetcher_present,
            "adopt-before-author: contest could not be minted — the peer's head is not \
             retrievable here and this row names no head of its own to nominate; holding"
        );
        return AdoptOutcome::Held;
    };

    // QUIESCENCE, second shape: we already nominated this exact head. Re-minting
    // would add an identical candidate every sweep until the election projects.
    if !claim_candidacy(id, own_head) {
        tracing::debug!(
            content_id = %id,
            "adopt-before-author: contest gate — this node already nominated its own head for \
             this id; awaiting the election"
        );
        return AdoptOutcome::Held;
    }

    // `adopt_before_author: false`, and structurally so: this arm is reached only
    // AFTER the chain gate has already PASSED (the zome answers `is not
    // retrievable` only downstream of it), so there is no no-chain gate here to
    // bypass — and it carries no record to license one with either.
    match conductor_writes::call_declare_canonical_content_head_classed(
        hc,
        id,
        own_head.to_string(),
        None,
        false,
        SWEEP_DECLARE_CLASS,
    )
    .await
    {
        Ok(declared) => contested(
            ContestShape::SelfHead,
            id,
            &declared.head_action_hash,
            local_declared,
            hint,
            carried_present,
            fetcher_present,
        ),
        Err(e) => {
            // A SHED is not a refusal. The claim is still handed back (the
            // candidacy ledger must not retire a pair the conductor never
            // judged), but nothing is counted and no window opens — the arm is
            // simply re-attempted next sweep. Ordered claim-release-first so
            // both paths release exactly once; `inc_contest_failed` and
            // `release_candidacy` touch unrelated state, so the swap is
            // behaviour-preserving on the non-shed path.
            release_candidacy(id, own_head);
            if declare_was_shed(&e) {
                tracing::debug!(
                    target: "elohim_storage::head_adoption",
                    content_id = %id,
                    from_peer = %hint.peer_id,
                    "self-candidacy declare shed at the admission gate — backpressure, not a \
                     verdict; no backoff recorded, retried on the next sweep"
                );
                return AdoptOutcome::Held;
            }
            crate::metrics::inc_contest_failed(unresolvable_class);
            // C3 REPAIR (2026-08-02, F-B). Hand the claim back and record a
            // BOUNDED backoff instead. Before this pair of lines a failed
            // self-candidacy retired `(id, own_head)` for the life of the
            // process — a permanent exclusion with no automated exit, and the
            // mechanism behind the live `contest_self_head=4` vs
            // `fetch_none=603` split. Releasing without the backoff would swing
            // to the other extreme (a predictable failure retried every sweep),
            // so the two must land together. (The release itself moved a few
            // lines up so the shed path shares it.)
            note_declare_backoff_unless_shed(
                id,
                &e,
                crate::metrics::ContestSkip::SelfCandidacyBackoff,
            );
            tracing::warn!(
                target: "elohim_storage::head_adoption",
                content_id = %id,
                from_peer = %hint.peer_id,
                carried = carried_present,
                fetcher = fetcher_present,
                error = %e,
                "adopt-before-author: self-candidacy declare failed — this node could not even \
                 nominate its own declared head; holding, retried after the contest backoff"
            );
            AdoptOutcome::Held
        }
    }
}

/// SPIN DISCHARGE (2026-08-14): supply the canonical election for an UNDECLARED
/// two-way root divergence by nominating THIS node's own head.
///
/// ## The class and why no other arm reaches it
///
/// Both peers hold a genuine root for the id; neither ever declared. The own
/// conductor answers this node's own (non-canonical) head every sweep —
/// `StampOutcome::Refreshed`, "real work, zero convergence" — and the id cycles
/// through the [`MissLedger`](crate::p2p::projection_reconcile) forever
/// (live: `known_divergent{stream="content"}` pinned at 13/13/13 across the
/// household mesh, zero `Healed` outcomes, saga ch06 blocked). Every existing
/// lever is structurally out of reach:
///
/// - [`HeadDecision::ContestPeer`] requires `local_declared`;
/// - ghost-decay requires an observed-ABSENT conductor answer;
/// - PEER-HEAD candidacy is impossible BY DESIGN: the head-record responder's
///   hint-inflation guard answers `Absent` for anchored-but-undeclared rows,
///   precisely so a requester can never turn a peer's anchor into a
///   declaration. This arm honors that boundary by nominating only what this
///   node itself holds — which is also why there is NO evidence fetch here at
///   all: the only answer a fetch could return for this class is the absence
///   we already know.
///
/// ## Why self-candidacy is supply, not self-election (C1)
///
/// Nothing is stamped. The mint adds ONE candidate to the DHT election on
/// positive same-sweep divergence evidence (a live peer advertised a different
/// non-empty anchor); the peer's own sweep mints ITS root as the competing
/// candidate (both pods default-ON, symmetric); `select_canonical_winner`
/// arbitrates deterministically — tier, then notarized link clock, then
/// link-hash tiebreak — and BOTH projections obey the winner through the
/// ordinary canonical heal (`HealCanonical` fills undeclared rows
/// unconditionally, moves declared ones only with forward ordering). A later
/// earned-tier declaration outranks a staging-tier self-candidate outright, so
/// the cost of a "wrong" nomination is one extra election candidate.
///
/// bounded-work (C6a): the `(id, own_head)` candidacy ledger caps mints at one
/// per process; the contest backoff bounds re-attempts after a failure; the
/// caller's slice cap and sweep budget bound the population. Zome gates pass by
/// construction — the chain exists (the conductor just answered this node's own
/// head) and the target is this node's own action, locally retrievable.
async fn contest_divergent(
    hc: &Arc<HcClient>,
    id: &str,
    own_head: &ContentHeadWire,
    divergent_peer: &str,
) -> AdoptOutcome {
    // BACKOFF GATE — same ledger as `contest_peer`, same reason: a predictable
    // repeat failure must not re-consume the sweep budget every 300s.
    if let Some(reason) = crate::services::contest_backoff::skip_class(
        id,
        crate::services::contest_backoff::BackoffWindows::from_config(),
    ) {
        crate::metrics::inc_contest_skipped(reason);
        tracing::debug!(
            content_id = %id,
            reason = ?reason,
            "spin-discharge: contest skipped — this id is serving a backoff for a predictable \
             repeat failure"
        );
        return AdoptOutcome::Held;
    }

    let target = own_head.head_action_hash.as_str();

    // IDEMPOTENCE — the row is NOT stamped by a contest, so without the claim
    // this arm would re-mint the identical link every sweep for the whole
    // window between minting and the election projecting back.
    if !claim_candidacy(id, target) {
        tracing::debug!(
            content_id = %id,
            "spin-discharge: this node already nominated its own head for this id; \
             awaiting the election"
        );
        return AdoptOutcome::Held;
    }

    match conductor_writes::call_declare_canonical_content_head_classed(
        hc,
        id,
        target.to_string(),
        None,
        false,
        SWEEP_DECLARE_CLASS,
    )
    .await
    {
        Ok(declared) => {
            crate::metrics::inc_content_canonical_link_minted(
                crate::metrics::MINTED_SOURCE_CONTEST_DIVERGENT_SELF,
            );
            tracing::warn!(
                target: "elohim_storage::head_adoption",
                content_id = %id,
                contested_head = %declared.head_action_hash,
                divergent_peer = %divergent_peer,
                "spin-discharge: CONTESTED an undeclared two-way root divergence by nominating \
                 THIS node's own head — the peer advertises a different root and holds no \
                 declaration either, so the election had no candidate from any side; both \
                 sides nominate their own and the DHT election picks; the row is unchanged \
                 until it does"
            );
            AdoptOutcome::Contested
        }
        Err(e) => {
            // C3: hand the claim back and take a BOUNDED backoff — one refusal
            // must not retire this (id, target) for the life of the process,
            // and a predictable failure must not be retried every sweep.
            release_candidacy(id, target);
            // ...unless the gate SHED it, in which case there was no refusal to
            // learn from: the conductor never saw the declare, so the arm is
            // re-attempted next sweep with no window and no failure count.
            if declare_was_shed(&e) {
                tracing::debug!(
                    target: "elohim_storage::head_adoption",
                    content_id = %id,
                    divergent_peer = %divergent_peer,
                    "spin-discharge declare shed at the admission gate — backpressure, not a \
                     verdict; no backoff recorded, retried on the next sweep"
                );
                return AdoptOutcome::Held;
            }
            crate::metrics::inc_contest_failed(crate::metrics::ContestFailure::DeclareError);
            note_declare_backoff_unless_shed(
                id,
                &e,
                crate::metrics::ContestSkip::SelfCandidacyBackoff,
            );
            tracing::warn!(
                target: "elohim_storage::head_adoption",
                content_id = %id,
                divergent_peer = %divergent_peer,
                error = %e,
                "spin-discharge: self-candidacy declare failed — this node could not nominate \
                 its own head; holding, retried after the contest backoff"
            );
            AdoptOutcome::Held
        }
    }
}

/// ADOPT-BEFORE-AUTHOR: mint a canonical-head candidate for an id this conductor
/// holds **no chain for**, from a carried record the zome proves in wasm.
///
/// Operator-reserved and DORMANT by default — see the module note. This is the
/// only arm that can move a both-sides-missing pair, because it is the only one
/// that does not require the declaring node to already know the id.
///
/// ## What licenses it
///
/// Evidence, and only evidence. `adopt_before_author: true` travels with the
/// bytes ([`adopt_before_author_param`] joins the two conditions in one place),
/// and the coordinator then runs `validate_carried_record` UNCONDITIONALLY on
/// this branch — action-hash binding, author signature, entry↔action binding —
/// plus the target-id gate. Nothing is taken on the peer's word: the peer is a
/// byte courier, exactly as in `try_obey_visible_election`.
///
/// Nothing is stamped here either. Like every contest shape this adds ONE
/// candidate to the DHT election and lets `select_canonical_winner` decide; the
/// row converges later through the ordinary canonical heal path.
///
/// ## Returns
///
/// `Some` **only** on a successful mint. Dormant, no bytes, already-nominated,
/// and refused-by-the-zome all return `None` so the caller's unchanged
/// no-chain tail (backoff + `Held`) runs — which is what keeps the flag-off
/// path byte-identical to the pre-flag one.
///
/// bounded-work: AT MOST ONE conductor round-trip per (id, target) per process,
/// and zero when the flag is off. There is deliberately NO retry ladder here —
/// the reconcile sweep IS the retry, at its own cadence, under its own slice cap
/// (`compose_adopt_slice`). The `(id, target)` candidacy ledger bounds the mint
/// count and the contest backoff bounds the re-attempt rate, so this arm adds no
/// new pacing scheme to a seam that already has two.
async fn try_adopt_before_author(
    hc: &Arc<HcClient>,
    id: &str,
    peer_head: &str,
    bytes: Option<Vec<u8>>,
    hint: &PeerHeadHint,
    carried_present: bool,
    fetcher_present: bool,
) -> Option<AdoptOutcome> {
    // DORMANT GATE. `adopt_retry_bytes` already applied
    // `adopt_before_author_param`, so a `Some` here means BOTH the node flag is
    // on AND validated-shape bytes are in hand.
    let bytes = bytes?;

    // IDEMPOTENCE. Same `(id, target)` ledger the self-candidacy shape uses: the
    // row is NOT stamped by a contest, so without this the arm would re-mint the
    // identical link every sweep for the whole window between minting and the
    // election projecting. Claimed BEFORE the attempt (the fan-out sweep can
    // reach this arm concurrently for one id) and handed back on failure, which
    // is the C3 repair that keeps a failed attempt from retiring the pair for
    // the life of the process.
    if !claim_candidacy(id, peer_head) {
        tracing::debug!(
            content_id = %id,
            "adopt-before-author: bypass gate — this node already nominated this target for \
             this id; awaiting the election"
        );
        return None;
    }

    match conductor_writes::call_declare_canonical_content_head_classed(
        hc,
        id,
        peer_head.to_string(),
        Some(bytes),
        true,
        SWEEP_DECLARE_CLASS,
    )
    .await
    {
        Ok(declared) => {
            crate::metrics::inc_content_canonical_link_minted(
                crate::metrics::MINTED_SOURCE_ADOPT_BEFORE_AUTHOR,
            );
            tracing::warn!(
                target: "elohim_storage::head_adoption",
                content_id = %id,
                contested_head = %declared.head_action_hash,
                from_peer = %hint.peer_id,
                carried = carried_present,
                fetcher = fetcher_present,
                "adopt-before-author: ADOPTED a peer's head as this node's canonical \
                 declaration WITHOUT a local chain — the zome re-derived the record's action \
                 hash, author signature, entry↔action binding and content id before writing \
                 the link, so this is evidence the DHT arbiter can elect on, not a claim. The \
                 both-sides-missing residual now has a candidate; the row is unchanged until \
                 the election decides"
            );
            Some(AdoptOutcome::Contested)
        }
        Err(e) => {
            // C3: hand the claim back, so one refusal cannot retire this
            // (id, target) for the life of the process. The caller's no-chain
            // tail then records the BOUNDED backoff — the same window, cleared
            // the moment an author path lands a chain.
            release_candidacy(id, peer_head);
            // ...EXCEPT on an admission shed, which is the one failure the
            // caller's tail must not see. `None` there would hand control back
            // to the no-chain tail, and that tail's whole job is to record a
            // backoff window — an evidence-class verdict drawn from a declare
            // the conductor never received. `Some(Held)` is the same outcome
            // the tail would have returned, minus the bookkeeping.
            if declare_was_shed(&e) {
                tracing::debug!(
                    target: "elohim_storage::head_adoption",
                    content_id = %id,
                    from_peer = %hint.peer_id,
                    "adopt-before-author declare shed at the admission gate — backpressure, not \
                     a refusal; no backoff recorded, retried on the next sweep"
                );
                return Some(AdoptOutcome::Held);
            }
            crate::metrics::inc_contest_failed(crate::metrics::ContestFailure::AdoptRefused);
            tracing::warn!(
                target: "elohim_storage::head_adoption",
                content_id = %id,
                from_peer = %hint.peer_id,
                error = %e,
                "adopt-before-author: the conductor REFUSED a chainless declaration despite \
                 carried evidence — the record did not prove itself (hash, signature, \
                 entry↔action binding, or content id), or the coordinator predates the flag; \
                 holding, retried after the contest backoff"
            );
            None
        }
    }
}

/// Shared success path for both contest shapes: count the mint and say, in one
/// human sentence, which candidate was nominated and why.
#[allow(clippy::too_many_arguments)]
fn contested(
    shape: ContestShape,
    id: &str,
    minted_head: &crate::signals::HoloHashB64,
    local_declared: Option<&str>,
    hint: &PeerHeadHint,
    carried: bool,
    fetcher: bool,
) -> AdoptOutcome {
    crate::metrics::inc_content_canonical_link_minted(shape.source_label());
    match shape {
        ContestShape::PeerHead => tracing::warn!(
            target: "elohim_storage::head_adoption",
            content_id = %id,
            contested_head = %minted_head,
            held_head = %local_declared.unwrap_or(""),
            from_peer = %hint.peer_id,
            carried,
            fetcher,
            "adopt-before-author: CONTESTED a two-way declared head — minted a canonical \
             declaration naming the peer's head; the DHT election decides, the row is \
             unchanged until it does"
        ),
        ContestShape::SelfHead => tracing::warn!(
            target: "elohim_storage::head_adoption",
            content_id = %id,
            contested_head = %minted_head,
            held_head = %local_declared.unwrap_or(""),
            from_peer = %hint.peer_id,
            carried,
            fetcher,
            "adopt-before-author: CONTESTED a two-way declared head by nominating THIS node's \
             own head — the peer's head is not retrievable here, so both sides nominate their \
             own and the DHT election picks between them; the row is unchanged until it does"
        ),
    }
    AdoptOutcome::Contested
}

/// Second half of AUTHOR-THEN-ADOPT: call this after the caller's author path
/// has given this conductor a local chain for `id`.
///
/// A transient failure here is NOT fatal — the row is left "anchored,
/// undeclared", which is a legal, self-healing state: the next sweep finds it
/// undeclared and re-runs the pre-flight.
pub async fn finish_author_then_adopt(
    hc: &Arc<HcClient>,
    pool: &DbPool,
    ctx: &AppContext,
    id: &str,
    head_action_hash: &str,
    carried_record: Option<Vec<u8>>,
    peer_id: &str,
) -> AdoptOutcome {
    // CHAIN ARRIVAL. The caller's author path just gave this conductor a chain
    // for `id`, so any `no_local_chain` backoff standing against it is stale by
    // construction. Clearing here (rather than waiting out the window) matters
    // inside a single tick: `adopt_deferred_heads` runs BEFORE both witness
    // sweeps, so an id can fail the no-chain gate and be authored seconds later.
    crate::services::contest_backoff::note_local_chain_arrived(id);
    // Same fact, the heal leg's ledger: a chain landing falsifies any cached
    // conductor-missing verdict for this id.
    crate::services::heal_backoff::note_resolved(id);
    // No alternates: this call already HAS the bytes in hand (the caller carried
    // them through the author path), so there is no evidence fetch to route.
    let hint = PeerHeadHint::sole(head_action_hash.to_string(), None, peer_id.to_string());
    match declare_peer_head(hc, pool, ctx, id, head_action_hash, carried_record, &hint).await {
        // A second "no local chain" refusal means the author did not actually
        // land; do not recurse — the next sweep retries from the top.
        AdoptOutcome::AuthorThenAdopt { .. } => {
            tracing::warn!(
                content_id = %id,
                "adopt-before-author: conductor still reports no local chain after authoring — \
                 left anchored-and-undeclared, retried next sweep"
            );
            AdoptOutcome::Author
        }
        other => other,
    }
}

/// Shared declare + stamp for both peer-adopt entrypoints.
///
/// DECLARE-STORM GATE: the declare is skipped outright when it would not MOVE
/// anything (this row already declares exactly this head). Combined with the
/// once-per-id-per-sweep loop structure, a converged corpus issues ZERO zome
/// declares per sweep — the difference between a cure and a new write storm.
async fn declare_peer_head(
    hc: &Arc<HcClient>,
    pool: &DbPool,
    ctx: &AppContext,
    id: &str,
    head_action_hash: &str,
    carried_record: Option<Vec<u8>>,
    hint: &PeerHeadHint,
) -> AdoptOutcome {
    if let Ok(mut conn) = pool.get() {
        let current = match content_diesel::declared_head_for(&mut conn, ctx, id) {
            Ok(v) => v,
            Err(e) => {
                // A DB error here silently opened the declare-storm gate (the
                // old `.unwrap_or(None)` read it identically to "undeclared").
                // Behavior is unchanged — this arm still falls through as
                // `None` — but the miss is now visible.
                tracing::warn!(
                    content_id = %id,
                    error = %e,
                    "adopt-before-author: declared_head_for failed — declare-storm gate cannot \
                     see the current declaration; proceeding as undeclared"
                );
                None
            }
        };
        if !declaration_would_move(current.as_deref(), head_action_hash) {
            tracing::debug!(
                content_id = %id,
                "adopt-before-author: declare-storm gate — already declaring this head, skipping"
            );
            return AdoptOutcome::Held;
        }
    } else {
        // Same silent-open: no connection means the declare-storm gate is
        // skipped entirely (falls straight through to the declare below).
        // Behavior is unchanged — only the visibility of the skip is new.
        tracing::warn!(
            content_id = %id,
            "adopt-before-author: could not get a DB connection — declare-storm gate skipped, \
             may re-declare an already-declared head"
        );
    }

    let carried_bytes = carried_record.as_ref().map(|b| b.len()).unwrap_or(0);
    // ADOPT-BEFORE-AUTHOR retry budget: `None` unless the node flag is on AND
    // bytes are in hand, so the dormant path clones nothing and the ladder below
    // collapses to the single classic call it has always been.
    let adopt_retry = adopt_retry_bytes(carried_record.as_ref());
    // Which channel actually minted, for `links_minted{source}`. Set to the
    // adopt label ONLY on the branch that provably used the bypass — a label
    // that could also mean "the classic path worked" would be unreadable as the
    // flag's live probe.
    let mut minted_source = "adopt_peer";

    // bounded-work: at most TWO conductor round-trips (the classic declare, then
    // the evidence retry), and exactly one while the flag is off. Not a retry
    // ladder — the second call is a DIFFERENT request (evidence-backed) made
    // once, only after the first was refused for the one reason it can answer.
    let attempted = conductor_writes::call_declare_canonical_content_head_classed(
        hc,
        id,
        head_action_hash.to_string(),
        carried_record.clone(),
        false,
        SWEEP_DECLARE_CLASS,
    )
    .await;
    let attempted = match attempted {
        Ok(declared) => Ok(declared),
        Err(first) => {
            let refused_for_no_chain = first.to_string().contains(ERR_NO_LOCAL_CHAIN);
            match adopt_retry.filter(|_| refused_for_no_chain) {
                None => Err(first),
                Some(bytes) => {
                    match conductor_writes::call_declare_canonical_content_head_classed(
                        hc,
                        id,
                        head_action_hash.to_string(),
                        Some(bytes),
                        true,
                        SWEEP_DECLARE_CLASS,
                    )
                    .await
                    {
                        Ok(declared) => {
                            minted_source = crate::metrics::MINTED_SOURCE_ADOPT_BEFORE_AUTHOR;
                            tracing::warn!(
                                target: "elohim_storage::head_adoption",
                                content_id = %id,
                                head = %head_action_hash,
                                from_peer = %hint.peer_id,
                                "adopt-before-author: declared a peer's head WITHOUT a local \
                                 chain, on a record the zome proved in wasm — no competing \
                                 local root was authored at all (the author-then-adopt \
                                 round-trip is skipped entirely)"
                            );
                            Ok(declared)
                        }
                        Err(second) if declare_was_shed(&second) => {
                            // The evidence retry was SHED, not refused. Counting
                            // `AdoptRefused` here would blame the record for a
                            // gate decision. The original no-chain error still
                            // propagates, so the tail below behaves exactly as
                            // it does when the flag is off — and the retry is
                            // simply re-offered on the next sweep.
                            tracing::debug!(
                                target: "elohim_storage::head_adoption",
                                content_id = %id,
                                from_peer = %hint.peer_id,
                                "adopt-before-author: the evidence retry was shed at the \
                                 admission gate — backpressure, not a refusal; falling back to \
                                 the unchanged author-then-adopt path"
                            );
                            Err(first)
                        }
                        Err(second) => {
                            crate::metrics::inc_contest_failed(
                                crate::metrics::ContestFailure::AdoptRefused,
                            );
                            tracing::warn!(
                                target: "elohim_storage::head_adoption",
                                content_id = %id,
                                from_peer = %hint.peer_id,
                                error = %second,
                                "adopt-before-author: the conductor refused a chainless \
                                 declaration despite carried evidence; falling back to the \
                                 unchanged author-then-adopt path"
                            );
                            // The ORIGINAL error propagates, so the tail below —
                            // including the `AuthorThenAdopt` classification —
                            // behaves exactly as it did before the flag existed.
                            Err(first)
                        }
                    }
                }
            }
        }
    };

    match attempted {
        Ok(declared) => {
            crate::metrics::inc_content_canonical_link_minted(minted_source);
            // This declaration is a DELIBERATE own-conductor canonical act that
            // this process just caused — the same class as the declare route's
            // eager stamp — so `Declare` is the correct mode here (unlike the
            // local-resolve arm above, which is a heal-class read).
            let stamped = pool.get().map_err(|e| e.to_string()).and_then(|mut conn| {
                content_diesel::stamp_declared_head_mode(
                    &mut conn,
                    ctx,
                    id,
                    declared.head_action_hash.as_str(),
                    Some(declared.declared_at),
                    None,
                    StampMode::Declare,
                    // The declare has written a link but not read back its
                    // notarized timestamp, so it carries no election to record.
                    // The row's ordering backfills on the next canonical heal
                    // resolve — which answers with the election just created.
                    None,
                )
                .map_err(|e| e.to_string())
            });
            match stamped {
                Ok(StampOutcome::Stamped) | Ok(StampOutcome::Refreshed) => {
                    crate::metrics::inc_content_head_adopted();
                    tracing::warn!(
                        target: "elohim_storage::head_adoption",
                        content_id = %id,
                        head = %declared.head_action_hash,
                        from_peer = %hint.peer_id,
                        carried_bytes,
                        "adopt-before-author: ADOPTED a peer-advertised canonical head via \
                         conductor declare (no competing root minted)"
                    );
                    AdoptOutcome::Adopted
                }
                Ok(other) => {
                    tracing::info!(
                        content_id = %id, outcome = ?other,
                        "adopt-before-author: conductor declared but the local stamp was a no-op"
                    );
                    AdoptOutcome::Held
                }
                Err(e) => {
                    tracing::warn!(
                        content_id = %id, error = %e,
                        "adopt-before-author: conductor declared but the local stamp failed \
                         (the DHT declaration stands; the projection heals next sweep)"
                    );
                    AdoptOutcome::Held
                }
            }
        }
        Err(e) => {
            // ADMISSION SHED — the ONE failure here that must not reach the
            // tail. Every other branch below ends in `AdoptOutcome::Author` (or
            // `AuthorThenAdopt`), i.e. MINTING A LOCAL ROOT because the peer's
            // head could not be declared. A shed says nothing about the peer's
            // head — the conductor never saw the declare — so authoring on it
            // would manufacture exactly the competing root this module exists to
            // stop, out of nothing but local capacity pressure. Hold instead;
            // the next sweep re-offers the same declare.
            if declare_was_shed(&e) {
                tracing::debug!(
                    target: "elohim_storage::head_adoption",
                    content_id = %id,
                    from_peer = %hint.peer_id,
                    "adopt-before-author: declare shed at the admission gate — backpressure, \
                     not a verdict; HOLDING rather than authoring a competing root, retried on \
                     the next sweep"
                );
                return AdoptOutcome::Held;
            }
            let msg = e.to_string();
            if msg.contains(ERR_NO_LOCAL_CHAIN) {
                // Expected on a genuinely fresh peer — not an error. Author
                // first (non-declaring), then come back.
                tracing::info!(
                    content_id = %id,
                    from_peer = %hint.peer_id,
                    "adopt-before-author: conductor has no local chain for this id — \
                     authoring a non-declaring root first, then adopting the peer's head"
                );
                return AdoptOutcome::AuthorThenAdopt {
                    head_action_hash: head_action_hash.to_string(),
                    carried_record,
                    peer_id: hint.peer_id.clone(),
                };
            }
            if msg.contains(ERR_NOT_RETRIEVABLE) {
                tracing::info!(
                    content_id = %id,
                    from_peer = %hint.peer_id,
                    carried_bytes,
                    "adopt-before-author: peer head not retrievable and no usable carried record \
                     — falling back to the author path, retried next sweep"
                );
            } else {
                tracing::warn!(
                    content_id = %id,
                    from_peer = %hint.peer_id,
                    error = %msg,
                    "adopt-before-author: conductor declare failed — falling back to the \
                     author path, retried next sweep"
                );
            }
            AdoptOutcome::Author
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Contest ON — the rollout default. Keeps the older tests reading as
    /// situations rather than argument lists. `peer_divergent_anchor` OFF: the
    /// legacy table, which the SPIN arm must never change.
    fn decide(c: bool, p: bool, l: bool, election: bool) -> HeadDecision {
        decide_head_action(c, p, l, election, true, false)
    }

    // ── Ghost-declaration decay (C1/C3/C4 contract) ─────────────────────────

    /// Dormant is behaviour-identical: with the flag off, NO combination of
    /// evidence — including the full positively-falsified state — authorizes
    /// the author path. This is the regression pin for "off = the pre-flag
    /// decision table byte-for-byte".
    #[test]
    fn ghost_decay_is_dormant_by_default() {
        for absent in [false, true] {
            for election in [false, true] {
                for hint in [false, true] {
                    for evidence in [false, true] {
                        assert!(
                            !ghost_decay_authorizes_author(false, absent, election, hint, evidence),
                            "dormant flag must never authorize (absent={absent}, \
                             election={election}, hint={hint}, evidence={evidence})"
                        );
                    }
                }
            }
        }
    }

    /// Enabled, the decay still requires EVERY positive observation: the own
    /// conductor's observed absence, a LIVE advertising hint this sweep, AND
    /// the advertiser's stated no-record verdict having STOOD the dwell — and
    /// never over a row obeying a local election. Exactly one input short →
    /// no author (C4: a missing answer is not evidence, and stale ledger
    /// hearsay without a live advertiser is not a declaration to falsify).
    #[test]
    fn ghost_decay_requires_every_positive_observation() {
        // The one authorized state.
        assert!(ghost_decay_authorizes_author(true, true, false, true, true));
        // Each single defection kills it.
        assert!(
            !ghost_decay_authorizes_author(true, false, false, true, true),
            "no observed local absence (Probe/Unreachable) must not decay"
        );
        assert!(
            !ghost_decay_authorizes_author(true, true, true, true, true),
            "a row obeying a local canonical election is settled, never decayed"
        );
        assert!(
            !ghost_decay_authorizes_author(true, true, false, false, true),
            "no live advertising hint this sweep — stale ledger evidence alone must not decay"
        );
        assert!(
            !ghost_decay_authorizes_author(true, true, false, true, false),
            "no stood stated-absent verdict (fresh or absent) must not decay"
        );
    }

    /// The meter may never disagree with the behaviour it explains: over the
    /// FULL 2^5 input space, `ghost_decay_blocked_leg` reports `None` exactly
    /// when `ghost_decay_authorizes_author` authorizes, and names a leg whose
    /// input is genuinely unmet otherwise. A drifted classifier would be worse
    /// than no meter — it would send an operator to redeploy a pod whose flag
    /// was fine.
    #[test]
    fn ghost_decay_blocked_leg_agrees_with_the_predicate() {
        use crate::metrics::GhostDecayBlockedLeg as Leg;
        for bits in 0u8..32 {
            let enabled = bits & 1 != 0;
            let absent = bits & 2 != 0;
            let election = bits & 4 != 0;
            let hint = bits & 8 != 0;
            let stood = bits & 16 != 0;

            let authorized = ghost_decay_authorizes_author(enabled, absent, election, hint, stood);
            let leg = ghost_decay_blocked_leg(enabled, absent, election, hint, stood);

            assert_eq!(
                authorized,
                leg.is_none(),
                "predicate/leg disagreement at (enabled={enabled}, absent={absent}, \
                 election={election}, hint={hint}, stood={stood})"
            );

            // The named leg must be a genuinely unmet input, not merely the
            // first one the match happened to reach.
            match leg {
                None => {}
                Some(Leg::Disabled) => assert!(!enabled),
                Some(Leg::LocalNotObservedAbsent) => assert!(!absent),
                Some(Leg::LocalElectionStands) => assert!(election),
                Some(Leg::NoLiveAdvertiser) => assert!(!hint),
                Some(Leg::EvidenceNotStood) => assert!(!stood),
            }
        }
    }

    // ── C4 contract: LocalResolve on seam_contracts::Answer (plan P1.2) ──
    //
    // The registry recorded this exact gap: "no unit test asserts on
    // LocalResolve::Known vs LocalResolve::Unresolved directly — decide_head_action's
    // tests exercise the downstream bool this degrades to, not the enum itself."
    // These close it.

    /// A minimal `ContentHeadWire` through its real `Deserialize` impl — the
    /// same path the conductor's answer takes, so the fixture cannot drift from
    /// the wire shape by construction.
    fn wire(canonical: bool) -> ContentHeadWire {
        serde_json::from_value(serde_json::json!({
            "content_id": "c1",
            "head_action_hash": "uhCkkContractTestHead0000000000000000000",
            "declared_at": 1_700_000_000_000_000i64,
            "canonical": canonical,
            "content": {
                "id": "c1",
                "content_type": "concept",
                "title": "t",
                "description": "d",
                "content_format": "markdown",
                "reach": "commons",
            },
        }))
        .expect("ContentHeadWire fixture must deserialize")
    }

    /// The wave-4 split, asserted at the type level: an OBSERVED absence and an
    /// unanswered resolve are different `Answer` states. If a future edit
    /// re-merges them, this fails.
    #[test]
    fn local_resolve_keeps_observed_absence_apart_from_unresolved() {
        let observed = LocalResolve::observed(None);
        let unresolved = LocalResolve::unresolved();

        let observed_state = match observed {
            LocalResolve::Resolved(a) => a.state(),
            LocalResolve::Probe => panic!("observed() must not construct Probe"),
        };
        let unresolved_state = match unresolved {
            LocalResolve::Resolved(a) => a.state(),
            LocalResolve::Probe => panic!("unresolved() must not construct Probe"),
        };

        assert_eq!(observed_state, seam_contracts::AnswerState::Absent);
        assert_eq!(unresolved_state, seam_contracts::AnswerState::Unreachable);
        assert_ne!(
            observed_state, unresolved_state,
            "the CONTRACT DEVIATION this type replaced was exactly these two \
             collapsing into one answer"
        );
    }

    /// A conductor that answered WITH a head is `Present` and carries it.
    #[test]
    fn an_answered_head_is_present_and_carried() {
        let head = wire(true);
        let resolved = LocalResolve::observed(Some(&head));
        match resolved {
            LocalResolve::Resolved(a) => {
                assert_eq!(a.state(), seam_contracts::AnswerState::Present)
            }
            LocalResolve::Probe => panic!("observed(Some) must not construct Probe"),
        }
        assert!(resolved.head().is_some());
    }

    /// BEHAVIOUR-NEUTRALITY of the P1.2 adoption: both non-present answers
    /// degrade to the same `Option` at the one named collapse point, which is
    /// what makes `AdoptLocal` unreachable from either — and nothing else.
    #[test]
    fn both_absences_foreclose_adopt_local_identically() {
        assert!(LocalResolve::observed(None).head().is_none());
        assert!(LocalResolve::unresolved().head().is_none());
        // ...and `Probe` carries nothing either: it is an instruction, not an
        // answer, so it must never masquerade as one.
        assert!(LocalResolve::Probe.head().is_none());

        // The downstream input to the decision rule is `canonical_head.is_some()`,
        // and it is false for every non-present answer — the identity the P1.2
        // retrofit had to preserve.
        for resolve in [LocalResolve::observed(None), LocalResolve::unresolved()] {
            let canonical = resolve.head().filter(|h| h.canonical);
            assert!(canonical.is_none());
        }
    }

    /// `Probe` deliberately sits OUTSIDE the `Answer` triad: "I have not asked"
    /// is not an answer about the world, and folding it into `Unreachable` would
    /// make an unasked question indistinguishable from an unanswered one.
    #[test]
    fn probe_is_not_an_answer() {
        assert!(matches!(LocalResolve::Probe, LocalResolve::Probe));
        assert!(!matches!(LocalResolve::Probe, LocalResolve::Resolved(_)));
    }

    /// P2.3 precondition, asserted rather than asserted-about: every `Answer`
    /// state that is not `Present` collapses to the same `None` at a fetch site,
    /// which is why the `HeadRecordFetcher` retrofit is behaviour-neutral.
    #[test]
    fn answer_states_collapse_uniformly_at_every_fetch_site() {
        let absent: Answer<CarriedHeadRecord> = Answer::Absent;
        let unreachable: Answer<CarriedHeadRecord> = Answer::Unreachable;
        assert!(absent.into_option().is_none());
        assert!(unreachable.into_option().is_none());

        let present = Answer::Present(CarriedHeadRecord {
            head_action_hash: "uhCkk-head".into(),
            record: None,
            record_absent_reason: None,
        });
        // A hash-only answer is still PRESENT — the peer answered. Reading it as
        // an absence is the `carried_present` mislabel (`da8975176`).
        assert!(present.is_present());
        let carried = present.into_option().expect("present carries a value");
        assert!(carried.record.is_none());
    }

    /// Build a carried answer with a stated (or unstated) absence reason.
    fn hash_only(reason: Option<RecordAbsentReason>) -> CarriedHeadRecord {
        CarriedHeadRecord {
            head_action_hash: "uhCkk-head".into(),
            record: None,
            record_absent_reason: reason,
        }
    }

    /// Every shape a fetch can come back in maps to exactly one evidence state,
    /// and the mapping is the input to a 24×-longer deferral — so it is worth
    /// asserting exhaustively rather than by example.
    #[test]
    fn evidence_classification_maps_every_answer_shape() {
        use crate::metrics::AdoptEvidence;

        let carried = CarriedHeadRecord {
            head_action_hash: "uhCkk-head".into(),
            record: Some(vec![1, 2, 3]),
            record_absent_reason: None,
        };
        assert_eq!(
            classify_evidence(Some(&carried)),
            AdoptEvidence::Carried,
            "bytes in hand is the only state the adopt exit can act on"
        );

        assert_eq!(
            classify_evidence(Some(&hash_only(Some(RecordAbsentReason::NoRecord)))),
            AdoptEvidence::NoRecord
        );
        assert_eq!(
            classify_evidence(Some(&hash_only(Some(RecordAbsentReason::BudgetElapsed)))),
            AdoptEvidence::BudgetElapsed
        );
        assert_eq!(
            classify_evidence(Some(&hash_only(Some(RecordAbsentReason::ConductorError)))),
            AdoptEvidence::ConductorError
        );

        // The three no-stated-cause shapes: an old peer, a future peer, and a
        // non-Present answer already collapsed to `None`. All three are
        // `Unknown`, and none of them is `NoRecord`.
        for unknown in [
            classify_evidence(Some(&hash_only(None))),
            classify_evidence(Some(&hash_only(Some(RecordAbsentReason::Unknown)))),
            classify_evidence(None),
        ] {
            assert_eq!(unknown, AdoptEvidence::Unknown);
            assert_ne!(
                unknown,
                AdoptEvidence::NoRecord,
                "silence must never be read as a structural absence"
            );
        }
    }

    /// A carried answer whose bytes ARE present outranks any reason the
    /// responder might contradict itself with. Bytes in hand is bytes in hand.
    #[test]
    fn carried_bytes_outrank_a_contradictory_reason() {
        let contradictory = CarriedHeadRecord {
            head_action_hash: "uhCkk-head".into(),
            record: Some(vec![7]),
            record_absent_reason: Some(RecordAbsentReason::NoRecord),
        };
        assert_eq!(
            classify_evidence(Some(&contradictory)),
            crate::metrics::AdoptEvidence::Carried
        );
    }

    /// ONLY a stated `no_record` earns the long deferral. Every other state —
    /// including both mixed-version shapes — keeps the ordinary window, which is
    /// byte-for-byte the behaviour before this lever existed.
    #[test]
    fn only_a_stated_no_record_takes_the_long_backoff() {
        use crate::metrics::{AdoptEvidence, ContestSkip};
        let day = std::time::Duration::from_secs(86_400);

        assert_eq!(
            evidence_backoff_class(Some(AdoptEvidence::NoRecord), day),
            ContestSkip::EvidenceAbsentBackoff
        );
        for ordinary in [
            Some(AdoptEvidence::Carried),
            Some(AdoptEvidence::BudgetElapsed),
            Some(AdoptEvidence::ConductorError),
            Some(AdoptEvidence::Unknown),
            // No fetch happened at all (the boot pass carries no fetcher).
            None,
        ] {
            assert_eq!(
                evidence_backoff_class(ordinary, day),
                ContestSkip::NoLocalChainBackoff,
                "{ordinary:?} must keep the ordinary window — the long one is for stated \
                 structural absence only"
            );
        }
    }

    /// The OFF switch is a real off switch, and it fails in the SAFE direction:
    /// a zero evidence window records the ORDINARY backoff, not "no backoff".
    /// Disabling a throughput lever must restore the prior behaviour, never
    /// something worse than it (which "no backoff" would be — every phantom id
    /// contested every sweep again).
    #[test]
    fn a_zero_evidence_window_restores_the_ordinary_class() {
        use crate::metrics::{AdoptEvidence, ContestSkip};
        assert_eq!(
            evidence_backoff_class(Some(AdoptEvidence::NoRecord), std::time::Duration::ZERO),
            ContestSkip::NoLocalChainBackoff,
            "with the lever disabled a stated absence must still be backed off, just on the \
             ordinary window"
        );
    }

    /// No fetcher means no fetch, and an unasked question must not be counted as
    /// an answer — a `Some(Unknown)` here would inflate the meter's denominator
    /// with every boot-pass candidate and make the evidence split unreadable.
    #[tokio::test]
    async fn an_absent_fetcher_resolves_no_evidence_at_all() {
        let hint = PeerHeadHint::sole("uhCkk-head".into(), None, "peer".into());
        let (carried, evidence) = resolve_peer_evidence(None, &hint, "id").await;
        assert!(carried.is_none());
        assert_eq!(
            evidence, None,
            "an unasked question is not an evidence state"
        );
    }

    // =========================================================================
    // ADVERTISER DIVERSITY (2026-08-03)
    // =========================================================================

    /// A fetcher scripted per peer, recording who was asked and in what order —
    /// the ONLY way to assert the amplification bound, which is a statement about
    /// round-trips rather than about return values.
    struct ScriptedFetcher {
        answers: std::collections::HashMap<String, Answer<CarriedHeadRecord>>,
        asked: std::sync::Mutex<Vec<String>>,
    }

    impl ScriptedFetcher {
        fn new(script: Vec<(&str, Answer<CarriedHeadRecord>)>) -> Self {
            Self {
                answers: script
                    .into_iter()
                    .map(|(p, a)| (p.to_string(), a))
                    .collect(),
                asked: std::sync::Mutex::new(Vec::new()),
            }
        }

        fn asked(&self) -> Vec<String> {
            self.asked.lock().expect("asked lock").clone()
        }
    }

    #[async_trait::async_trait]
    impl HeadRecordFetcher for ScriptedFetcher {
        async fn fetch(&self, peer_id: &str, _content_id: &str) -> Answer<CarriedHeadRecord> {
            self.asked
                .lock()
                .expect("asked lock")
                .push(peer_id.to_string());
            match self.answers.get(peer_id) {
                Some(Answer::Present(c)) => Answer::Present(c.clone()),
                Some(Answer::Absent) => Answer::Absent,
                _ => Answer::Unreachable,
            }
        }
    }

    fn carrying(bytes: &[u8]) -> Answer<CarriedHeadRecord> {
        Answer::Present(CarriedHeadRecord {
            head_action_hash: "uhCkk-head".into(),
            record: Some(bytes.to_vec()),
            record_absent_reason: None,
        })
    }

    fn hash_only_answer(reason: RecordAbsentReason) -> Answer<CarriedHeadRecord> {
        Answer::Present(hash_only(Some(reason)))
    }

    fn hint_with(primary: &str, alternates: &[&str]) -> PeerHeadHint {
        PeerHeadHint {
            head_action_hash: "uhCkk-head".into(),
            declared_at: None,
            peer_id: primary.to_string(),
            alternates: alternates.iter().map(|s| (*s).to_string()).collect(),
        }
    }

    /// Read one `adopt_evidence{state}` series. Deltas, never absolutes: the
    /// registry is process-wide.
    fn evidence_count(state: crate::metrics::AdoptEvidence) -> u64 {
        crate::metrics::CONTENT_ADOPT_EVIDENCE
            .with_label_values(&[state.label()])
            .get()
    }

    fn fallback_count(outcome: crate::metrics::AdoptEvidenceFallback) -> u64 {
        crate::metrics::CONTENT_ADOPT_EVIDENCE_FALLBACK
            .with_label_values(&[outcome.label()])
            .get()
    }

    /// THE AMPLIFICATION GATE, exhaustively. Every `true` here is an extra
    /// conductor round-trip on a fleet whose conductors are the scarce resource,
    /// and the `no_record` arm is the one that must never flip: a stated
    /// no-record is peer-local evidence (local-strategy responder get) whose
    /// population the evidence-absent backoff + ghost-decay dwell already own —
    /// a same-sweep second ask is amplification, not new information.
    #[test]
    fn only_a_peer_shaped_failure_warrants_a_second_ask() {
        use crate::metrics::AdoptEvidence;
        for peer_shaped in [
            AdoptEvidence::BudgetElapsed,
            AdoptEvidence::ConductorError,
            AdoptEvidence::Unknown,
        ] {
            assert!(
                fallback_warrants(peer_shaped, true),
                "{peer_shaped:?} says something about THIS peer — another peer may not share it"
            );
        }
        assert!(
            !fallback_warrants(AdoptEvidence::NoRecord, true),
            "a stated structural absence must never be re-asked elsewhere"
        );
        assert!(
            !fallback_warrants(AdoptEvidence::Carried, true),
            "bytes in hand leave nothing to route around"
        );
    }

    /// The OFF switch is a real off switch: with the lever disabled NO state
    /// warrants a second ask, which is the single-ask path byte for byte.
    #[test]
    fn the_disabled_lever_never_warrants_a_second_ask() {
        use crate::metrics::AdoptEvidence;
        use seam_contracts::ReasonLabel as _;
        for state in AdoptEvidence::ALL {
            assert!(
                !fallback_warrants(*state, false),
                "{state:?} must not trigger a fallback when the lever is off"
            );
        }
    }

    /// THE CURE, end to end: the hinted advertiser is starved, an idle peer
    /// holds the same rows, and the bytes arrive. This is the live shape —
    /// `adopt_evidence{budget_elapsed}` dominating while matthew/adam idled — and
    /// the resolution must report `carried`, because the arm downstream acts on
    /// bytes, not on who supplied them.
    #[tokio::test]
    #[allow(
        clippy::await_holding_lock,
        reason = "the process-global advertiser-health table must stay exclusive across each async resolution"
    )]
    async fn a_degraded_primary_routes_to_a_healthy_alternate() {
        let _exclusive = crate::services::advertiser_health::test_exclusive();
        crate::services::advertiser_health::reset();
        use crate::metrics::{AdoptEvidence, AdoptEvidenceFallback};

        let before_carried = evidence_count(AdoptEvidence::Carried);
        let before_budget = evidence_count(AdoptEvidence::BudgetElapsed);
        let before_attempted = fallback_count(AdoptEvidenceFallback::Attempted);
        let before_fb_carried = fallback_count(AdoptEvidenceFallback::Carried);

        let fetcher = ScriptedFetcher::new(vec![
            (
                "throttled",
                hash_only_answer(RecordAbsentReason::BudgetElapsed),
            ),
            ("idle", carrying(b"record-bytes")),
        ]);
        let hint = hint_with("throttled", &["idle"]);
        let (carried, evidence) = resolve_peer_evidence(Some(&fetcher), &hint, "id").await;

        assert_eq!(
            carried.and_then(|c| c.record),
            Some(b"record-bytes".to_vec()),
            "the fallback's BYTES are what the caller gets"
        );
        assert_eq!(evidence, Some(AdoptEvidence::Carried));
        assert_eq!(fetcher.asked(), vec!["throttled", "idle"]);

        // ACCOUNTING: exactly ONE evidence increment, for the FINAL state. A
        // per-attempt count would inflate the denominator and make the evidence
        // split — the whole reason that meter exists — unreadable.
        assert_eq!(evidence_count(AdoptEvidence::Carried), before_carried + 1);
        assert_eq!(
            evidence_count(AdoptEvidence::BudgetElapsed),
            before_budget,
            "the primary's degraded state must NOT also be counted"
        );
        assert_eq!(
            fallback_count(AdoptEvidenceFallback::Attempted),
            before_attempted + 1
        );
        assert_eq!(
            fallback_count(AdoptEvidenceFallback::Carried),
            before_fb_carried + 1
        );
        crate::services::advertiser_health::reset();
    }

    /// A stated no-record is never re-asked same-sweep (peer-local evidence;
    /// the backoff + dwell own it). This is the amplification bound at its
    /// most load-bearing: ~61% of the live refusal population is phantom ids
    /// whose bytes exist NOWHERE, and fanning those out would double the cost
    /// of the population with the least to gain.
    #[tokio::test]
    #[allow(
        clippy::await_holding_lock,
        reason = "the process-global advertiser-health table must stay exclusive across each async resolution"
    )]
    async fn a_structural_absence_is_never_routed_around() {
        let _exclusive = crate::services::advertiser_health::test_exclusive();
        crate::services::advertiser_health::reset();
        use crate::metrics::{AdoptEvidence, AdoptEvidenceFallback};

        let before_attempted = fallback_count(AdoptEvidenceFallback::Attempted);
        let before_no_record = evidence_count(AdoptEvidence::NoRecord);

        let fetcher = ScriptedFetcher::new(vec![
            ("primary", hash_only_answer(RecordAbsentReason::NoRecord)),
            ("idle", carrying(b"never-asked")),
        ]);
        let hint = hint_with("primary", &["idle"]);
        let (carried, evidence) = resolve_peer_evidence(Some(&fetcher), &hint, "id").await;

        assert!(carried.is_some_and(|c| c.record.is_none()));
        assert_eq!(evidence, Some(AdoptEvidence::NoRecord));
        assert_eq!(
            fetcher.asked(),
            vec!["primary"],
            "a structural absence must cost exactly one round-trip"
        );
        assert_eq!(
            fallback_count(AdoptEvidenceFallback::Attempted),
            before_attempted
        );
        assert_eq!(
            evidence_count(AdoptEvidence::NoRecord),
            before_no_record + 1
        );
        crate::services::advertiser_health::reset();
    }

    /// A carried primary is the common converged case, and it must stay a single
    /// round-trip — otherwise the lever taxes the ids that are already working.
    #[tokio::test]
    #[allow(
        clippy::await_holding_lock,
        reason = "the process-global advertiser-health table must stay exclusive across each async resolution"
    )]
    async fn a_carried_primary_is_never_routed_around() {
        let _exclusive = crate::services::advertiser_health::test_exclusive();
        crate::services::advertiser_health::reset();
        let fetcher = ScriptedFetcher::new(vec![
            ("primary", carrying(b"bytes")),
            ("idle", carrying(b"also-bytes")),
        ]);
        let hint = hint_with("primary", &["idle"]);
        let (_carried, evidence) = resolve_peer_evidence(Some(&fetcher), &hint, "id").await;
        assert_eq!(evidence, Some(crate::metrics::AdoptEvidence::Carried));
        assert_eq!(fetcher.asked(), vec!["primary"]);
        crate::services::advertiser_health::reset();
    }

    /// A BOUNDED LADDER, never an unbounded one (drain lever 3, 2026-08-07;
    /// widened again 2026-08-09, B2b). Five alternates are offered, the
    /// configured breadth is 4, and exactly four are asked — the fifth is
    /// never reached however promising it looks. That bound is what keeps
    /// source-widening from becoming a fan-out against a fleet whose
    /// conductors are already the scarce resource. The invariant this replaces
    /// asserted a ladder of ONE; the ladder is now `min(alternates,
    /// evidence_fallback_max_alternates())`, and it must still be a fixed,
    /// pre-sized walk with no retry and no recursion.
    #[tokio::test]
    #[allow(
        clippy::await_holding_lock,
        reason = "the process-global advertiser-health table must stay exclusive across each async resolution"
    )]
    async fn the_fallback_ladder_is_bounded_by_its_budget() {
        let _exclusive = crate::services::advertiser_health::test_exclusive();
        crate::services::advertiser_health::reset();
        use crate::metrics::{AdoptEvidence, AdoptEvidenceFallback};

        let budget = crate::config::evidence_fallback_max_alternates();
        let before_budget_state = evidence_count(AdoptEvidence::BudgetElapsed);
        let before_degraded = fallback_count(AdoptEvidenceFallback::Degraded);
        let before_attempted = fallback_count(AdoptEvidenceFallback::Attempted);

        // Every offered alternate is starved EXCEPT the last, which the bound
        // must keep out of reach — so a widened ladder cannot be mistaken for an
        // exhaustive one.
        let fetcher = ScriptedFetcher::new(vec![
            (
                "primary",
                hash_only_answer(RecordAbsentReason::BudgetElapsed),
            ),
            ("alt-a", hash_only_answer(RecordAbsentReason::BudgetElapsed)),
            ("alt-b", hash_only_answer(RecordAbsentReason::BudgetElapsed)),
            ("alt-c", hash_only_answer(RecordAbsentReason::BudgetElapsed)),
            ("alt-d", hash_only_answer(RecordAbsentReason::BudgetElapsed)),
            ("alt-e", carrying(b"beyond-the-bound")),
        ]);
        let hint = hint_with("primary", &["alt-a", "alt-b", "alt-c", "alt-d", "alt-e"]);
        let (carried, evidence) = resolve_peer_evidence(Some(&fetcher), &hint, "id").await;

        assert_eq!(
            fetcher.asked().len(),
            1 + budget,
            "the primary plus exactly `evidence_fallback_max_alternates()` couriers — no more, \
             whatever they answer"
        );
        assert!(
            !fetcher.asked().contains(&"alt-e".to_string()),
            "the courier past the bound must never be asked, even though it holds the bytes"
        );
        assert!(carried.is_some_and(|c| c.record.is_none()));
        assert_eq!(
            evidence,
            Some(AdoptEvidence::BudgetElapsed),
            "a fully-failed ladder leaves the PRIMARY's evidence as the resolution — the \
             backoff class downstream keys on what the advertising peer said, and widening \
             the probe must not widen that licence"
        );
        assert_eq!(
            evidence_count(AdoptEvidence::BudgetElapsed),
            before_budget_state + 1,
            "still exactly ONE evidence increment across the whole ladder"
        );
        assert_eq!(
            fallback_count(AdoptEvidenceFallback::Degraded),
            before_degraded + 1,
            "`degraded` stays one per RESOLUTION"
        );
        assert_eq!(
            fallback_count(AdoptEvidenceFallback::Attempted),
            before_attempted + budget as u64,
            "`attempted` counts FETCHES, so it is the round-trip denominator"
        );
        crate::services::advertiser_health::reset();
    }

    /// THE FILL-BANDWIDTH CLAIM, which is the whole reason lever 3 exists: an id
    /// whose SECOND-best courier holds the bytes now converges in this sweep.
    /// Under the pre-widening single-ask rule this exact shape resolved
    /// `budget_elapsed`, took a backoff, and waited — with the bytes sitting on a
    /// peer the sweep had already harvested and simply never asked.
    #[tokio::test]
    #[allow(
        clippy::await_holding_lock,
        reason = "the process-global advertiser-health table must stay exclusive across each async resolution"
    )]
    async fn a_second_alternate_can_still_supply_the_bytes() {
        let _exclusive = crate::services::advertiser_health::test_exclusive();
        crate::services::advertiser_health::reset();
        use crate::metrics::{AdoptEvidence, AdoptEvidenceFallback};

        assert!(
            crate::config::evidence_fallback_max_alternates() >= 2,
            "this test asserts the SECOND courier is reached; it is meaningless at breadth < 2"
        );
        let before_carried = evidence_count(AdoptEvidence::Carried);
        let before_fb_carried = fallback_count(AdoptEvidenceFallback::Carried);

        let fetcher = ScriptedFetcher::new(vec![
            (
                "primary",
                hash_only_answer(RecordAbsentReason::BudgetElapsed),
            ),
            (
                "alt-a",
                hash_only_answer(RecordAbsentReason::ConductorError),
            ),
            ("alt-b", carrying(b"record-bytes")),
        ]);
        let hint = hint_with("primary", &["alt-a", "alt-b"]);
        let (carried, evidence) = resolve_peer_evidence(Some(&fetcher), &hint, "id").await;

        assert_eq!(
            carried.and_then(|c| c.record),
            Some(b"record-bytes".to_vec()),
            "the ladder must keep walking past a second starved courier to the one that carries"
        );
        assert_eq!(evidence, Some(AdoptEvidence::Carried));
        assert_eq!(fetcher.asked(), vec!["primary", "alt-a", "alt-b"]);
        assert_eq!(
            evidence_count(AdoptEvidence::Carried),
            before_carried + 1,
            "one resolution, one evidence increment — the ladder's depth never inflates it"
        );
        assert_eq!(
            fallback_count(AdoptEvidenceFallback::Carried),
            before_fb_carried + 1,
            "`carried` is one per RESOLUTION — the ladder stops at the first courier that serves"
        );
        crate::services::advertiser_health::reset();
    }

    /// No second courier ⇒ the primary's outcome, unchanged, and a counted
    /// `no_alternative`. That series is the one that says HINT PLURALITY is the
    /// wall rather than peer health — a different remedy entirely.
    #[tokio::test]
    #[allow(
        clippy::await_holding_lock,
        reason = "the process-global advertiser-health table must stay exclusive across each async resolution"
    )]
    async fn no_alternative_returns_the_primary_outcome_and_says_so() {
        let _exclusive = crate::services::advertiser_health::test_exclusive();
        crate::services::advertiser_health::reset();
        use crate::metrics::{AdoptEvidence, AdoptEvidenceFallback};

        let before_none = fallback_count(AdoptEvidenceFallback::NoAlternative);
        let before_attempted = fallback_count(AdoptEvidenceFallback::Attempted);
        let before_unknown = evidence_count(AdoptEvidence::Unknown);

        // Unreachable primary, and the only "alternate" is the primary itself.
        let fetcher = ScriptedFetcher::new(vec![]);
        let hint = hint_with("lonely", &["lonely", "  "]);
        let (carried, evidence) = resolve_peer_evidence(Some(&fetcher), &hint, "id").await;

        assert!(carried.is_none(), "an unreachable peer carries nothing");
        assert_eq!(evidence, Some(AdoptEvidence::Unknown));
        assert_eq!(fetcher.asked(), vec!["lonely"]);
        assert_eq!(
            fallback_count(AdoptEvidenceFallback::NoAlternative),
            before_none + 1
        );
        assert_eq!(
            fallback_count(AdoptEvidenceFallback::Attempted),
            before_attempted,
            "no candidate means no fetch was attempted"
        );
        assert_eq!(evidence_count(AdoptEvidence::Unknown), before_unknown + 1);
        crate::services::advertiser_health::reset();
    }

    /// The health tracker is fed by the SAME site that counts evidence, for both
    /// the primary and the fallback — otherwise the second ask would be a blind
    /// pick forever.
    #[tokio::test]
    #[allow(
        clippy::await_holding_lock,
        reason = "the process-global advertiser-health table must stay exclusive across each async resolution"
    )]
    async fn both_asks_feed_the_health_tracker() {
        let _exclusive = crate::services::advertiser_health::test_exclusive();
        crate::services::advertiser_health::reset();

        let fetcher = ScriptedFetcher::new(vec![
            (
                "health:throttled",
                hash_only_answer(RecordAbsentReason::BudgetElapsed),
            ),
            ("health:idle", carrying(b"bytes")),
        ]);
        let hint = hint_with("health:throttled", &["health:idle"]);
        let _ = resolve_peer_evidence(Some(&fetcher), &hint, "id").await;

        let throttled =
            crate::services::advertiser_health::health("health:throttled").expect("tracked");
        let idle = crate::services::advertiser_health::health("health:idle").expect("tracked");
        assert_eq!((throttled.carried, throttled.degraded), (0, 1));
        assert_eq!((idle.carried, idle.degraded), (1, 0));
        assert!(
            idle.carried_share() > throttled.carried_share(),
            "the peer that served bytes must now be the preferred courier"
        );
        crate::services::advertiser_health::reset();
    }

    /// The `elohim_content_adopt_evidence_fallback_total{outcome}` vocabulary is
    /// a dashboard contract from its first deploy, same as the evidence states.
    #[test]
    fn fallback_labels_are_stable() {
        seam_contracts::assert_reason_labels_stable::<crate::metrics::AdoptEvidenceFallback>(&[
            "attempted",
            "carried",
            "degraded",
            "no_alternative",
        ]);
        seam_contracts::assert_reason_labels_discriminating::<crate::metrics::AdoptEvidenceFallback>(
        );
    }

    /// The snapshot vocabulary likewise.
    #[test]
    fn backoff_snapshot_labels_are_stable() {
        seam_contracts::assert_reason_labels_stable::<crate::metrics::BackoffSnapshot>(&[
            "saved",
            "save_failed",
            "loaded",
            "load_failed",
        ]);
        seam_contracts::assert_reason_labels_discriminating::<crate::metrics::BackoffSnapshot>();
    }

    /// The `elohim_content_adopt_evidence_total{state}` vocabulary is a
    /// dashboard contract from its FIRST deploy, for the same reason the
    /// obey-probe's is: the whole point of the meter is that an operator reads
    /// WHICH supply wall dominates, and a renamed label silently zeroes the
    /// panel that says so.
    ///
    /// `carried` is pinned alongside the four absence states deliberately — it
    /// is the success member, and dropping it leaves the failures with nothing
    /// to divide by.
    #[test]
    fn adopt_evidence_labels_are_stable() {
        seam_contracts::assert_reason_labels_stable::<crate::metrics::AdoptEvidence>(&[
            "carried",
            "no_record",
            "budget_elapsed",
            "conductor_error",
            "unknown",
        ]);
        seam_contracts::assert_reason_labels_discriminating::<crate::metrics::AdoptEvidence>();
    }

    /// `ContestSkip`'s labels are the same kind of contract. `evidence_absent_backoff`
    /// is APPENDED — no existing string moves, so every panel keyed on the two
    /// originals keeps its exact series.
    #[test]
    fn contest_skip_labels_are_stable() {
        seam_contracts::assert_reason_labels_stable::<crate::metrics::ContestSkip>(&[
            "no_local_chain_backoff",
            "self_candidacy_backoff",
            "declare_error_backoff",
            "evidence_absent_backoff",
        ]);
        seam_contracts::assert_reason_labels_discriminating::<crate::metrics::ContestSkip>();
    }

    /// A SHED IS NOT A VERDICT. The gate refused local capacity; the conductor
    /// never saw the declare, so nothing was learned about this id and nothing
    /// may be booked against it. Exercised through the real recording helper and
    /// the real ledger — the same shape as the sibling test below — so it cannot
    /// pass from a restatement of the rule.
    ///
    /// The negative half is the load-bearing one: an ordinary conductor error
    /// must STILL open its window, or this guard would have silently disabled
    /// the backoff the sweep budget depends on.
    #[test]
    fn a_shed_declare_earns_no_backoff_window() {
        use crate::conductor_admission::ADMISSION_SHED_MARKER;
        use crate::metrics::ContestSkip;
        use crate::services::contest_backoff::{self, BackoffWindows};

        const WINDOWS: BackoffWindows = BackoffWindows {
            contest: std::time::Duration::from_secs(3600),
            evidence_absent: std::time::Duration::from_secs(86_400),
        };

        let _exclusive = contest_backoff::test_exclusive();
        contest_backoff::reset();

        // The shed the gate actually mints (`AdmissionGate::shed_error`) — same
        // variant, same marker const, so a change to either fails here.
        let shed = StorageError::Timeout(format!(
            "{ADMISSION_SHED_MARKER}: no conductor permit for content_store within 1000ms \
             (class=background, capacity=5, in_flight=5) — nothing was dispatched"
        ));
        assert!(
            declare_was_shed(&shed),
            "the gate's own shed error must be recognised as backpressure"
        );
        assert!(
            !note_declare_backoff_unless_shed("shed:id", &shed, ContestSkip::SelfCandidacyBackoff),
            "a shed must not be recorded as a contest verdict"
        );
        assert_eq!(
            contest_backoff::skip_class("shed:id", WINDOWS),
            None,
            "a shed declare must leave the candidate eligible for the very next sweep"
        );

        // A real conductor answer — dispatched, executed, refused — still earns
        // its window. Same helper, opposite verdict.
        let refused = StorageError::Conductor(
            "Zome call failed: declare_canonical_head: earned head is protected".to_string(),
        );
        assert!(
            !declare_was_shed(&refused),
            "a conductor refusal is an answer, not a shed"
        );
        assert!(
            note_declare_backoff_unless_shed(
                "refused:id",
                &refused,
                ContestSkip::SelfCandidacyBackoff
            ),
            "a genuine declare failure must still be booked"
        );
        assert_eq!(
            contest_backoff::skip_class("refused:id", WINDOWS),
            Some(ContestSkip::SelfCandidacyBackoff),
            "a predictable repeat failure must still be deferred under its own class"
        );

        contest_backoff::reset();
    }

    /// The sweep declares must not stand in the lane a person is waiting in.
    /// Pinned as an inequality against the class the HTTP declare routes keep,
    /// so flipping either constant to match the other fails here rather than
    /// silently re-merging the two lanes.
    #[test]
    fn sweep_declares_do_not_share_the_interactive_lane() {
        assert_eq!(SWEEP_DECLARE_CLASS, AdmissionClass::Background);
        assert_ne!(
            SWEEP_DECLARE_CLASS,
            conductor_writes::DECLARE_DEFAULT_CLASS,
            "the reconciler's declares and an operator's declare route must not draw on the \
             same wait bound"
        );
    }

    /// The generic Arm-1 error used to spend fanout on the same id every sweep.
    /// Exercise the branch's real recording helper and the real ledger read so
    /// this contract cannot pass from a duplicated classification formula.
    #[test]
    fn a_generic_declare_error_enters_the_ordinary_per_id_backoff() {
        use crate::metrics::ContestSkip;
        use crate::services::contest_backoff::{self, BackoffWindows};

        let _exclusive = contest_backoff::test_exclusive();
        contest_backoff::reset();
        let id = "declare-error:backoff";
        note_declare_error_backoff(id);

        assert_eq!(
            contest_backoff::skip_class(
                id,
                BackoffWindows {
                    contest: std::time::Duration::from_secs(3600),
                    evidence_absent: std::time::Duration::from_secs(86_400),
                },
            ),
            Some(ContestSkip::DeclareErrorBackoff),
            "the id must be deferred on the next sweep under its distinct typed reason"
        );
        contest_backoff::reset();
    }

    /// The wire vocabulary round-trips through its type. `wire_str` feeds the
    /// responder's payload and `from_wire` reads it back, so a mismatch between
    /// them is a silent cross-fleet mislabel that no single-sided test catches.
    #[test]
    fn a_wire_reason_round_trips_through_its_type() {
        for reason in [
            RecordAbsentReason::NoRecord,
            RecordAbsentReason::ConductorError,
            RecordAbsentReason::BudgetElapsed,
        ] {
            assert_eq!(
                RecordAbsentReason::from_wire(Some(reason.wire_str())),
                Some(reason),
                "{reason:?} must survive its own wire token"
            );
        }
        assert_eq!(RecordAbsentReason::from_wire(None), None);
    }

    /// An unrecognised token is `Unknown`, never a decode failure and never a
    /// verdict. This is what lets a FUTURE peer add a fourth reason without
    /// stopping adoption against this build.
    #[test]
    fn an_unrecognised_reason_is_unknown_not_a_decode_failure() {
        assert_eq!(
            RecordAbsentReason::from_wire(Some("some_future_word")),
            Some(RecordAbsentReason::Unknown)
        );
        assert_eq!(
            RecordAbsentReason::from_wire(Some("  budget_elapsed  ")),
            Some(RecordAbsentReason::BudgetElapsed),
            "whitespace around a known token must not demote it to Unknown"
        );
    }

    /// P1.3: the `elohim_content_contest_failed_total{class}` label vocabulary is
    /// a dashboard contract. The first four strings are BYTE-IDENTICAL to the raw
    /// literals they replaced; changing one silently zeroes every panel keyed on
    /// it.
    ///
    /// RE-PINNED 2026-08-03 (adopt-before-author). `adopt_refused` is APPENDED,
    /// and appending is the only mutation this pin should ever accept without a
    /// migration: no existing label string moved, so every panel, alert and
    /// recording rule keyed on the four originals keeps its exact series and
    /// cannot go silently zero. The new class is genuinely new — "the evidence
    /// branch ran and the zome rejected the record" fits none of the four — and
    /// it is structurally zero while the flag is dormant, so a dashboard that
    /// never adds it simply shows what it always showed.
    ///
    /// A future edit that RENAMES or REORDERS any of the first four is the case
    /// this pin exists to stop; re-pinning it away is not a fix.
    #[test]
    fn contest_failure_labels_are_stable() {
        seam_contracts::assert_reason_labels_stable::<crate::metrics::ContestFailure>(&[
            "no_local_chain",
            "not_retrievable",
            "fetch_none",
            "declare_error",
            "adopt_refused",
        ]);
        seam_contracts::assert_reason_labels_discriminating::<crate::metrics::ContestFailure>();
    }

    /// The `elohim_content_election_obey_probe_total{outcome}` vocabulary is a
    /// dashboard contract from its FIRST deploy, not after it earns one: the
    /// whole point of the meter is that an operator reads which of the three
    /// walls dominates, and a renamed label silently zeroes the panel that says
    /// so.
    ///
    /// `attempted` is pinned alongside the exits deliberately. It is the
    /// denominator — drop it and the exits become counts with nothing to divide
    /// by, which is the state that made a 100%-failing arm look idle.
    #[test]
    fn election_obey_probe_labels_are_stable() {
        seam_contracts::assert_reason_labels_stable::<crate::metrics::ElectionObeyProbe>(&[
            "attempted",
            "no_election",
            "resolve_error",
            "no_courier",
        ]);
        seam_contracts::assert_reason_labels_discriminating::<crate::metrics::ElectionObeyProbe>();
    }

    /// The obey-probe vocabulary must stay DISJOINT from the obey-FAILURE
    /// vocabulary it sits beside.
    ///
    /// `CONTENT_ELECTION_OBEY_FAILED{class}` is scoped by its own doc to
    /// had-an-election-didn't-move (`fetch` | `validate` | `stamp_refused`). If a
    /// probe outcome ever collided with one of those strings, a reader
    /// correlating the two meters would double-count one population and the
    /// scoping sentence in both docs would quietly become false — the same
    /// two-vocabularies-one-string hazard `ContestSkip` is kept separate from
    /// `ContestFailure` to avoid.
    #[test]
    fn obey_probe_outcomes_never_collide_with_obey_failure_classes() {
        use seam_contracts::ReasonLabel as _;
        const OBEY_FAILURE_CLASSES: [&str; 3] = ["fetch", "validate", "stamp_refused"];
        for outcome in crate::metrics::ElectionObeyProbe::ALL {
            assert!(
                !OBEY_FAILURE_CLASSES.contains(&outcome.label()),
                "probe outcome {:?} collides with an obey-failure class — the two meters answer \
                 different questions and must stay readable apart",
                outcome.label()
            );
        }
    }

    /// All branches of the decision rule, named by the situation each one
    /// describes rather than by its inputs.
    #[test]
    fn own_conductor_canonical_always_adopts_never_authors() {
        // Regardless of what peers say, what this row already claims, or whether
        // an election stands behind it.
        for peer_declared in [false, true] {
            for local_declared in [false, true] {
                for election in [false, true] {
                    assert_eq!(
                        decide(true, peer_declared, local_declared, election),
                        HeadDecision::AdoptLocal,
                        "canonical own-conductor answer wins over every other input \
                         (peer={peer_declared}, local={local_declared}, election={election})"
                    );
                }
            }
        }
    }

    #[test]
    fn peer_declaration_on_an_undeclared_row_adopts() {
        assert_eq!(
            decide(false, true, false, false),
            HeadDecision::AdoptPeer,
            "this is the live cure: peer B holds no crown, alpha-A advertises one"
        );
    }

    #[test]
    fn an_existing_local_declaration_holds() {
        assert_eq!(
            decide(false, false, true, false),
            HeadDecision::Hold,
            "nobody is advertising anything better; keep what we have"
        );
        assert_eq!(
            decide(false, false, true, true),
            HeadDecision::Hold,
            "no peer hint means nothing to contest, election or not"
        );
    }

    /// THE LIVE CLASS (2026-08-02): both sides declare, no election has run.
    /// This used to `Hold` forever — and because `AdoptPeer` is the only
    /// automated minter of canonical links, holding meant the DHT arbiter never
    /// got a set to elect on. ~11.4k ids fleet-wide sat here.
    #[test]
    fn two_way_declared_without_an_election_contests() {
        assert_eq!(
            decide(false, true, true, false),
            HeadDecision::ContestPeer,
            "supply the election a candidate; do NOT decide the winner locally"
        );
    }

    /// QUIESCENCE — the property that makes this a cure and not a write storm.
    /// Once an election stands behind the row, a peer advertising the LOSING
    /// head must not provoke another mint: the question is settled on the DHT
    /// and this row is already obeying it.
    #[test]
    fn a_row_obeying_an_election_holds_against_a_peer_advertising_the_loser() {
        assert_eq!(
            decide(false, true, true, true),
            HeadDecision::Hold,
            "a converged corpus mints ZERO links per sweep — re-contesting a \
             settled election is exactly the permanent write storm this gate exists to stop"
        );
    }

    /// The config switch is a true rollback: OFF restores the pre-cure rule.
    #[test]
    fn contest_disabled_restores_the_old_hold() {
        assert_eq!(
            decide_head_action(false, true, true, false, false, false),
            HeadDecision::Hold,
            "per-pod OFF switch must yield exactly the prior behaviour"
        );
    }

    // ── SPIN discharge (undeclared two-way root divergence, 2026-08-14) ──────

    /// The new arm fires from exactly ONE corner: nothing declared anywhere,
    /// no election, live divergence evidence, family switch on.
    #[test]
    fn contest_divergent_is_reachable_only_from_the_undeclared_divergent_corner() {
        for c in [false, true] {
            for p in [false, true] {
                for l in [false, true] {
                    for e in [false, true] {
                        for enabled in [false, true] {
                            for d in [false, true] {
                                if decide_head_action(c, p, l, e, enabled, d)
                                    == HeadDecision::ContestDivergent
                                {
                                    assert!(
                                        !c && !p && !l && !e && enabled && d,
                                        "ContestDivergent escaped its corner \
                                         (canonical={c}, peer={p}, local={l}, election={e}, \
                                          enabled={enabled}, divergent={d})"
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// With the divergence input OFF, the whole 5-input table is byte-for-byte
    /// the pre-SPIN rule — the regression pin for "off = prior behaviour".
    #[test]
    fn spin_arm_off_leaves_every_existing_row_unchanged() {
        for c in [false, true] {
            for p in [false, true] {
                for l in [false, true] {
                    for e in [false, true] {
                        for enabled in [false, true] {
                            assert_ne!(
                                decide_head_action(c, p, l, e, enabled, false),
                                HeadDecision::ContestDivergent,
                                "divergence=false must never produce the new arm \
                                 (canonical={c}, peer={p}, local={l}, election={e}, \
                                  enabled={enabled})"
                            );
                        }
                    }
                }
            }
        }
    }

    /// QUIESCENCE: a row already obeying an election is settled — divergence
    /// evidence must not re-open it (the same gate that keeps a converged
    /// corpus minting zero links).
    #[test]
    fn spin_arm_defers_to_a_standing_election() {
        assert_eq!(
            decide_head_action(false, false, false, true, true, true),
            HeadDecision::Author,
            "an election-settled row must not be re-contested by anchor evidence"
        );
    }

    /// PRECEDENCE: a peer DECLARATION is stronger evidence than a divergent
    /// anchor — the adopt arm keeps the id even when both are present.
    #[test]
    fn a_peer_declaration_outranks_divergence_evidence() {
        assert_eq!(
            decide_head_action(false, true, false, false, true, true),
            HeadDecision::AdoptPeer,
            "declaration evidence must win over anchor evidence"
        );
    }

    #[test]
    fn nothing_declared_anywhere_authors() {
        assert_eq!(
            decide(false, false, false, false),
            HeadDecision::Author,
            "today's behaviour, now safe: the authored root is not a declaration"
        );
    }

    /// The rule is total: every input maps to exactly one decision, and
    /// authoring happens ONLY when no head exists anywhere.
    #[test]
    fn author_is_reachable_only_when_no_head_exists_anywhere() {
        let mut authored = 0;
        for c in [false, true] {
            for p in [false, true] {
                for l in [false, true] {
                    for e in [false, true] {
                        for enabled in [false, true] {
                            if decide_head_action(c, p, l, e, enabled, false)
                                == HeadDecision::Author
                            {
                                authored += 1;
                                assert!(
                                    !c && !p && !l,
                                    "Author must be unreachable when any head exists \
                                     (canonical={c}, peer={p}, local={l}, election={e})"
                                );
                            }
                        }
                    }
                }
            }
        }
        assert_eq!(
            authored, 4,
            "exactly the four no-head-anywhere corners author (election and the \
             config switch are irrelevant when nothing is declared)"
        );
    }

    /// CONTEST never authors and never adopts — it is reachable only from the
    /// two-way-declared corner, and only with the switch on.
    #[test]
    fn contest_is_reachable_only_from_the_two_way_declared_corner() {
        for c in [false, true] {
            for p in [false, true] {
                for l in [false, true] {
                    for e in [false, true] {
                        for enabled in [false, true] {
                            for d in [false, true] {
                                if decide_head_action(c, p, l, e, enabled, d)
                                    == HeadDecision::ContestPeer
                                {
                                    assert!(
                                        !c && p && l && !e && enabled,
                                        "ContestPeer escaped its corner \
                                         (canonical={c}, peer={p}, local={l}, election={e}, \
                                          enabled={enabled}, divergent={d})"
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// DECLARE-STORM GATE. The cure must not install a write storm: a converged
    /// corpus has to issue ZERO zome declares per sweep.
    #[test]
    fn declare_storm_gate_refuses_a_declare_that_moves_nothing() {
        assert!(
            !declaration_would_move(Some("uhCkk-ALPHA-A"), "uhCkk-ALPHA-A"),
            "already declaring this exact head — re-declaring is a source-chain \
             write plus a DHT link for no change, every sweep, forever"
        );
        assert!(
            !declaration_would_move(Some("  uhCkk-ALPHA-A "), "uhCkk-ALPHA-A"),
            "stray padding must not read as a difference and re-declare each sweep"
        );
    }

    #[test]
    fn declare_storm_gate_allows_a_fill_and_a_real_move() {
        assert!(
            declaration_would_move(None, "uhCkk-ALPHA-A"),
            "an UNDECLARED row is the fill case the peer arm exists for"
        );
        assert!(
            declaration_would_move(Some("uhCkk-LOCAL-ROOT"), "uhCkk-ALPHA-A"),
            "a genuinely different head is a real move and must be allowed"
        );
    }

    /// QUIESCENCE, shape 2 (self-candidacy). The durable gate is
    /// `canonical_declared_at` on the row, but that only closes once the election
    /// PROJECTS. In the window between minting and projecting, the ledger is what
    /// stops a node re-nominating the identical head every sweep.
    #[test]
    fn self_candidacy_is_claimed_once_per_id_and_head() {
        let id = "quiescence:self-candidacy";
        let head = "uhCkk-own-head";

        assert!(
            claim_candidacy(id, head),
            "the first nomination must be allowed — this is the candidate that \
             gives the DHT election something to pick"
        );
        for sweep in 0..5 {
            assert!(
                !claim_candidacy(id, head),
                "sweep {sweep}: re-nominating the SAME head adds an identical candidate \
                 for no gain — a converged corpus must mint zero"
            );
        }
    }

    /// A genuinely NEW local head is a NEW candidate and must be nominatable —
    /// keying the ledger on the id alone would freeze a node out of the election
    /// forever the moment its own head legitimately moved.
    #[test]
    fn a_moved_local_head_may_be_nominated_again() {
        let id = "quiescence:moved-head";
        assert!(claim_candidacy(id, "uhCkk-head-A"));
        assert!(!claim_candidacy(id, "uhCkk-head-A"));
        assert!(
            claim_candidacy(id, "uhCkk-head-B"),
            "our head moved; that is a different candidate and the election must see it"
        );
    }

    /// The ledger keys two different ids independently — a shared key would let
    /// one id's nomination silently suppress another's.
    #[test]
    fn the_self_candidacy_ledger_does_not_collide_across_ids() {
        assert!(claim_candidacy("quiescence:id-one", "uhCkk-shared"));
        assert!(
            claim_candidacy("quiescence:id-two", "uhCkk-shared"),
            "same head hash under a different id is a distinct candidacy"
        );
    }

    /// **F-B, C3 repair.** A FAILED self-candidacy hands its claim back, so the
    /// id can nominate again — bounded by the contest backoff rather than
    /// retired for the life of the process.
    ///
    /// This is the assertion that closes the live `contest_self_head = 4` vs
    /// `contest_failed{fetch_none} = 603` split: 603 ids had each burned their
    /// one attempt and could never nominate again without a pod restart.
    /// Claim-on-attempt is still correct (the fan-out sweep can reach this arm
    /// for one id concurrently, and claim-on-success would let every task mint);
    /// what was missing was the release.
    #[test]
    fn a_failed_self_candidacy_releases_its_claim_rather_than_retiring_the_id() {
        let id = "quiescence:failed-then-retried";
        let head = "uhCkk-head-that-failed";

        assert!(claim_candidacy(id, head), "first attempt claims the pair");
        assert!(
            !claim_candidacy(id, head),
            "a second attempt while the first is in flight must NOT double-mint — this is \
             why the claim is taken before the declare, not after it succeeds"
        );

        // The declare failed. Hand the claim back.
        release_candidacy(id, head);

        assert!(
            claim_candidacy(id, head),
            "after a released claim the id must be nominatable again — before this repair it \
             was retired for the life of the process, a permanent exclusion with no \
             automated exit"
        );
    }

    /// The release is SCOPED: handing back one pair must not release another
    /// id's claim, or one failure would license a re-mint storm elsewhere.
    #[test]
    fn releasing_one_claim_leaves_every_other_claim_standing() {
        assert!(claim_candidacy("release:kept", "uhCkk-kept"));
        assert!(claim_candidacy("release:dropped", "uhCkk-dropped"));

        release_candidacy("release:dropped", "uhCkk-dropped");

        assert!(
            !claim_candidacy("release:kept", "uhCkk-kept"),
            "an unrelated id's claim must survive another id's release"
        );
        assert!(
            claim_candidacy("release:dropped", "uhCkk-dropped"),
            "the released pair is claimable again"
        );
    }

    /// Releasing a pair that was never claimed is a no-op, not a panic — the
    /// error path calls this unconditionally.
    #[test]
    fn releasing_an_unclaimed_pair_is_harmless() {
        release_candidacy("release:never-claimed", "uhCkk-nothing");
        assert!(
            claim_candidacy("release:never-claimed", "uhCkk-nothing"),
            "the pair is still freshly claimable"
        );
    }

    /// The key is unambiguous: no (id, target) pair can be confused with another
    /// by concatenation. Uses a separator that cannot occur in either component.
    #[test]
    fn self_candidate_keys_are_unambiguous() {
        assert_ne!(
            self_candidate_key("a:b", "c"),
            self_candidate_key("a", "b:c"),
            "a naive join would collide these two distinct candidacies"
        );
    }

    /// QUIESCENCE, shape 1 (the durable gate) — restated against the shapes so
    /// the two are visibly complementary: the ledger bounds the transient window,
    /// `canonical_declared_at` closes it permanently.
    #[test]
    fn the_durable_gate_supersedes_the_ledger_once_the_election_projects() {
        assert_eq!(
            decide(false, true, true, true),
            HeadDecision::Hold,
            "once an election stands behind the row, contest is never reached at \
             all — the ledger is only needed before that lands"
        );
        assert_eq!(
            decide(false, true, true, false),
            HeadDecision::ContestPeer,
            "and before it lands, contest IS reached — which is exactly why the \
             ledger has to hold the line in the meantime"
        );
    }

    /// ELECTION-OBEY scope: the arm is probed ONLY for the conductor-missing
    /// class (no head answer at all). A row that got an answer — canonical or
    /// fallback — is already served by the existing arms, and probing it would
    /// spend a conductor round-trip per row per sweep for nothing.
    ///
    /// This mirrors the gate in `try_adopt_canonical_head`.
    #[test]
    fn election_obey_is_scoped_to_the_conductor_missing_class() {
        assert!(
            should_probe_election(false),
            "no head answer at all is exactly the class that may be hiding a \
             visible election — probe it"
        );
        assert!(
            !should_probe_election(true),
            "a row the conductor DID answer for is already served by the adopt/contest \
             arms; probing it spends a conductor round-trip per row per sweep for nothing"
        );
    }

    /// The obey path must stamp under the ELECTION's ordering in `HealCanonical`
    /// mode — never `Declare`. Proof of the bytes authorizes believing WHAT the
    /// head is; it never authorizes moving a row backwards. The tempting shortcut
    /// (bytes are proven, so just `Declare` them) is exactly the 2026-07-12
    /// backwards-heal regression.
    ///
    /// Asserted on the VERDICT function rather than by scanning source text: the
    /// property that matters is that an obey-shaped stamp is still arbitrated,
    /// and a source scan for `StampMode::` is both brittle and — as this test
    /// originally proved — capable of passing by accident when its slice runs
    /// past the end of the function it meant to check.
    #[test]
    fn election_obey_ordering_is_still_arbitrated_not_forced() {
        use crate::db::content_diesel::{canonical_move_verdict, StaleReason};

        // Forward election ⇒ the obey stamp moves.
        assert!(canonical_move_verdict(Some((9_000, false)), Some((1_000, false))).is_ok());
        // Older election ⇒ refused, even though the bytes were cryptographically
        // proven. Proof of WHAT is not permission to go BACKWARDS.
        assert_eq!(
            canonical_move_verdict(Some((1_000, false)), Some((9_000, false))).unwrap_err(),
            StaleReason::NotNewer,
        );
        // Staging election cannot displace an earned one, proven bytes or not.
        assert_eq!(
            canonical_move_verdict(Some((9_000, false)), Some((1_000, true))).unwrap_err(),
            StaleReason::Tier,
        );
    }

    /// QUIESCENCE, shape 3 (obeyed rows). Once the obey path stamps, the row
    /// carries `canonical_declared_at`, so the decision rule Holds — no re-obey
    /// attempt and, critically, no CONTEST of a question the DHT already settled.
    #[test]
    fn an_obeyed_row_neither_re_obeys_nor_contests() {
        assert_eq!(
            decide(false, true, true, true),
            HeadDecision::Hold,
            "a row that obeyed an election must not then contest it — that would \
             re-open a settled question every sweep"
        );
        // And the obey arm itself refuses a second move: `canonical_move_verdict`
        // declines an equal election (NotNewer), which the arm counts as
        // `stamp_refused` rather than treating as progress.
        assert_eq!(
            crate::db::content_diesel::canonical_move_verdict(
                Some((7_000, false)),
                Some((7_000, false))
            )
            .unwrap_err(),
            crate::db::content_diesel::StaleReason::NotNewer,
            "re-obeying the SAME election must not read as a move"
        );
    }

    /// A no-election answer must leave behaviour EXACTLY as it was — the arm
    /// returns `None` and the normal decision rule runs. This is what keeps
    /// wave 4 safe to land before the wave-3 window confirms anything.
    ///
    /// Expressed as the decision rule the fall-through lands on: with no election
    /// visible the obey arm returns `None`, so a conductor-missing row with a peer
    /// hint reaches exactly the verdict it reached before wave 4.
    #[test]
    fn no_visible_election_falls_through_to_the_unchanged_decision() {
        assert_eq!(
            decide(false, true, false, false),
            HeadDecision::AdoptPeer,
            "undeclared + peer declares ⇒ AdoptPeer, exactly as before wave 4"
        );
        assert_eq!(
            decide(false, true, true, false),
            HeadDecision::ContestPeer,
            "two-way declared ⇒ ContestPeer, exactly as before wave 4 — the obey arm \
             must not silently swallow ids it could not act on"
        );
        assert_eq!(
            decide(false, false, false, false),
            HeadDecision::Author,
            "nothing anywhere ⇒ Author, unchanged"
        );
    }

    #[test]
    fn contest_shape_source_labels_are_distinct_and_stable() {
        assert_eq!(ContestShape::PeerHead.source_label(), "contest_peer_head");
        assert_eq!(ContestShape::SelfHead.source_label(), "contest_self_head");
        assert_ne!(
            ContestShape::PeerHead.source_label(),
            ContestShape::SelfHead.source_label(),
            "the two shapes must be separable on the dashboard — which one \
             converges the fleet is the open question this metric answers"
        );
    }

    #[test]
    fn boot_context_has_no_hints_and_no_fetcher() {
        let ctx = AdoptContext::none();
        assert!(ctx.hints.is_empty());
        assert!(
            ctx.fetcher.is_none(),
            "the one-shot boot pass runs before P2P discovery — the peer arm must \
             degrade to the local-DHT arm alone, not error"
        );
    }

    // -----------------------------------------------------------------------
    // ADOPT-BEFORE-AUTHOR (operator-reserved, DEFAULT OFF)
    // -----------------------------------------------------------------------

    /// The predicate is the AND of both conditions over its whole input space —
    /// exhaustive, because the one thing that must never happen is the flag
    /// drifting apart from the evidence it is supposed to travel with.
    #[test]
    fn adopt_before_author_requires_both_the_flag_and_the_evidence() {
        assert!(
            adopt_before_author_param(true, true),
            "flag on AND bytes in hand is the only licensed shape"
        );
        assert!(
            !adopt_before_author_param(true, false),
            "the flag alone must never set the param — an unevidenced chainless \
             declaration is exactly the self-crowning this seam refuses"
        );
        assert!(
            !adopt_before_author_param(false, true),
            "evidence alone must never set it either; that is what makes the ship dormant"
        );
        assert!(!adopt_before_author_param(false, false));
    }

    /// THE DEFAULT-OFF PROOF, read from the shipped config rather than restated:
    /// a process that never published the switch reports it disabled, so every
    /// call site computes `false` and sends the pre-flag conductor payload.
    ///
    /// Deliberately asserts the CONFIG accessor, not a literal — a future edit
    /// that flipped `config::adopt_before_author_enabled`'s fallback to `true`
    /// would turn a dormant capability live on every pod in the fleet, and this
    /// is the test that would catch it.
    #[test]
    fn the_node_flag_defaults_to_off_so_the_capability_ships_dormant() {
        assert!(
            !crate::config::adopt_before_author_enabled(),
            "adopt-before-author must be OFF unless an operator sets \
             ELOHIM_ADOPT_BEFORE_AUTHOR — the whole point of the dormant ship is \
             that turning it on is a deliberate act"
        );
        assert!(
            !adopt_before_author_param(crate::config::adopt_before_author_enabled(), true),
            "with the shipped default even a validated carried record must not set the param"
        );
    }

    /// FLAG OFF ⇒ NO RETRY BUDGET. The dormant path does not merely decline to
    /// use the bytes — it never clones them. This is what makes "behaviour
    /// identical" also mean "allocation identical" on the hottest sweep in the
    /// crate (one contested id per candidate per sweep, records are multi-KB).
    #[test]
    fn a_dormant_node_reserves_no_adopt_retry_budget() {
        let record = vec![0xABu8; 64];
        assert!(
            adopt_retry_bytes(Some(&record)).is_none(),
            "with the flag off there is no retry payload, so no second conductor \
             call and no multi-KB clone can occur"
        );
        assert!(
            adopt_retry_bytes(None).is_none(),
            "and with no bytes served there is nothing to reserve either way"
        );
    }

    /// IDEMPOTENCE for the NEW shape, on the SAME `(id, target)` ledger the
    /// self-candidacy shape uses. A contest never stamps the row, so without this
    /// the arm would re-mint an identical link every sweep for the whole window
    /// between minting and the election projecting.
    #[test]
    fn an_adopted_candidacy_mints_at_most_once_per_id_and_target() {
        let id = "adopt-ledger:once";
        let peer_head = "uhCkk-peer-head";
        assert!(
            claim_candidacy(id, peer_head),
            "the first adopt-before-author nomination of this target is minted"
        );
        assert!(
            !claim_candidacy(id, peer_head),
            "a replay of the SAME nomination must be refused — this is the write-storm gate"
        );
    }

    /// The peer's head MOVING is a genuinely new candidate and must be
    /// nominatable. This is why the ledger keys on `(id, target)` and not on
    /// `id`: the adopt shape's target tracks the advertising peer, unlike the
    /// self-candidacy shape's, which is stable.
    #[test]
    fn a_moved_peer_head_is_a_new_adopted_candidacy() {
        let id = "adopt-ledger:moved";
        assert!(claim_candidacy(id, "uhCkk-peer-head-A"));
        assert!(
            claim_candidacy(id, "uhCkk-peer-head-B"),
            "when the advertised head moves the node must be able to nominate the new one"
        );
    }

    /// C3: a REFUSED adopt attempt hands its claim back, so one failure cannot
    /// retire the pair for the life of the process. Mirrors the self-candidacy
    /// repair — the same permanent-exclusion shape, at a new call site.
    #[test]
    fn a_refused_adopt_attempt_returns_its_claim() {
        let id = "adopt-ledger:refused";
        let peer_head = "uhCkk-refused-head";
        assert!(claim_candidacy(id, peer_head));
        release_candidacy(id, peer_head);
        assert!(
            claim_candidacy(id, peer_head),
            "a refused evidence branch must leave the pair re-nominatable — a \
             permanent exclusion with no automated exit is a C3 violation"
        );
    }

    /// The new failure label is additive and distinguishable: `adopt_refused`
    /// isolates "the bypass ran and the zome rejected the evidence" from every
    /// pre-existing class, and — critically — the four pinned labels are
    /// unchanged, because dashboards key on them byte-for-byte.
    #[test]
    fn the_adopt_refusal_label_is_additive_and_distinct() {
        use crate::metrics::ContestFailure;
        let labels: Vec<&str> = ContestFailure::ALL.iter().map(|c| c.label()).collect();
        assert_eq!(
            labels,
            vec![
                "no_local_chain",
                "not_retrievable",
                "fetch_none",
                "declare_error",
                "adopt_refused",
            ],
            "the four original labels must stay byte-identical and in place; the \
             new one is APPENDED, never inserted"
        );
        assert_eq!(
            labels.len(),
            labels
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len(),
            "every failure class must be separable on the dashboard"
        );
    }

    /// The mint label is a named const rather than a literal precisely because it
    /// IS the flag's live probe: a typo would mint a permanently-zero series and
    /// read as "the operator flip did nothing".
    #[test]
    fn the_adopt_mint_label_is_separable_from_every_other_source() {
        let adopt = crate::metrics::MINTED_SOURCE_ADOPT_BEFORE_AUTHOR;
        assert_eq!(adopt, "adopt_before_author");
        assert_ne!(adopt, ContestShape::PeerHead.source_label());
        assert_ne!(adopt, ContestShape::SelfHead.source_label());
        assert_ne!(
            adopt, "adopt_peer",
            "the bypass must not share a series with the classic peer-adopt arm — \
             the whole convergence reading depends on telling them apart"
        );
    }
}
