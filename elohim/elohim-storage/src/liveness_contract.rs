//! C3 (liveness) contract for the head-election decision surface — the plan's
//! P2.2 regression demonstration.
//!
//! Design: `genesis/docs/superpowers/plans/2026-08-02-seam-concern-contract-architecture-plan.md`
//! (canon row C3; Design surface 3; task P2.2). Instrument:
//! [`seam_contracts::harness::liveness`] (task P2.1).
//!
//! ## What this module proves
//!
//! One state-space table, three transition sets over it:
//!
//! | Leg | Transition set | Expected |
//! |---|---|---|
//! | 1 | the **pre-wave-5** predicate set (deadlock cured 2026-08-02) | **FAILS** |
//! | 2 | the **2026-07-11 over-broad GapFill** guard set (froze convergence) | **FAILS** |
//! | 3 | the **live** predicates, called directly | **PASSES** |
//!
//! Two *independently dated* deadlocks, three weeks apart, at overlapping seams —
//! caught by one instrument that was written for neither. That is the anti-fitting
//! requirement: a harness demonstrated against a single incident is indistinguishable
//! from a harness fitted to it.
//!
//! ## The three disciplines this file is written under
//!
//! **1. Reconstructions never call live code.** [`pre_wave5_transitions`] and
//! [`july11_gapfill_transitions`] are plain-data restatements of what the historical
//! predicate sets *were*, transcribed from the commits cited on each function. They
//! do not call [`decide_head_action`] or any admission predicate. A reconstruction
//! that computed its expectation from today's code would be a mirror: it would track
//! every future edit and could never fail, which is precisely the "a passing contract
//! test can be measuring the mirror" trap the plan's verification-track note names.
//!
//! **2. Leg 3 calls the real functions.** No restatement, no adapter that re-derives
//! a verdict. The live leg's only adaptation is *shape*: the predicates take booleans
//! and `Option<&str>`, and [`RowState`] is exactly those booleans given names.
//!
//! **3. Transitions model DECISION-LOGIC effects, not environmental luck.** If "the
//! conductor might integrate the canonical link on some later sweep" counted as a
//! transition, every state would be trivially live and the table would be an
//! instrument that agrees with any bug. The single exception is the transient class
//! ([`OwnAnswer::Timeout`]), which is *defined* as recoverable — and it is modeled
//! identically in all three legs, so it can never be the thing that separates them.
//!
//! ## Scope of the table (stated so the silence is honest — C7)
//!
//! This table asks the **cross-node head-election** liveness question: does a row
//! that is undeclared, or that diverges from a peer, always have an automated path
//! to a settled declaration? It deliberately does **not** model:
//!
//! - **Forward-monotonic refresh of an already-agreed head.** That is
//!   `canonical_move_verdict`'s question (C2), registered separately in
//!   `seam-registry.yaml` with its own contract test. It is why `declared ∧ ¬diverging`
//!   is [`StateClass::Terminal`] here rather than a state with a refresh self-loop.
//! - **The both-sides-conductor-missing pair.** Neither pod can mint a candidate, so
//!   no storage arm moves it. That is a *documented open residual* of the 2026-08-02
//!   shift, not a cured wall, and it gets its own witness test —
//!   [`tests::both_sides_conductor_missing_is_a_known_open_residual`] — which asserts
//!   the harness NAMES it rather than hiding it inside a green main table. Since
//!   ADOPT-BEFORE-AUTHOR that witness runs TWO legs over the same state table,
//!   parameterised by the config bit exactly as [`CONTEST_ENABLED`] parameterises the
//!   contest arm: **flag OFF (the shipped default)** — still `manual_only_exit`, the
//!   residual is open and must keep saying so; **flag ON** — a genuine automated exit
//!   exists. A capability that ships dormant does not close a residual, and modeling
//!   both legs is what keeps that distinction legible instead of letting a built-but-off
//!   arm read as a cure.
//! - **Sweep SCHEDULING.** Whether a live row reaches its arm *this tick* — the
//!   per-tick slice cap, the item spacing, the wall-clock budget, the contest
//!   backoff's priority ordering — is a dimension this table does not carry. Every
//!   state here is asked "does an automated arm exist for it," never "does that arm
//!   get a turn." The distinction is load-bearing, and 2026-08-03 is why it is
//!   written down: an arm can be *modeled live and scheduled never*. On a
//!   high-volume pod the adopt sweep's `.take(200)` truncated all 134
//!   backoff-deferred candidates off the end of the slice, so the election-obey arm
//!   modeled below never ran for that whole class — a liveness-shaped outage this
//!   table is structurally blind to. The scheduling-layer guarantee lives beside the
//!   scheduler instead: [`crate::p2p::projection_reconcile::compose_adopt_slice`]
//!   reserves a tail of the cap for the deferred class, pinned by
//!   `a_binding_cap_still_reserves_a_tail_for_the_deferred_class`. The same
//!   reasoning is why the contest backoff is witnessed as its own two-state table
//!   ([`tests::the_contest_backoff_is_never_a_permanent_exclusion`]) rather than
//!   folded in here.
//! - **Anchor-plane divergence without declarations (the SPIN class).** The table's
//!   peer dimension models DECLARATION advertisements (`PeerHeadHints`); a peer
//!   holding a genuinely different ROOT while advertising no declaration is not a
//!   state it carries — which is exactly how that class stayed an invisible
//!   absorbing state until 2026-08-14 (`known_divergent{content}` pinned at
//!   13/13/13, zero `Healed`). Adding the dimension here would force both
//!   historical reconstructions to model evidence that postdates them (the same
//!   trade the courier note below declines), so the class gets its own witness
//!   instead: [`tests::the_spin_class_discharge_is_flag_shaped`], driven by the
//!   live `undeclared_divergence_should_route_to_contest` + `decide_head_action`
//!   predicates and parameterised by `config.contest_undeclared_divergence`.
//! - **Node-level configuration.** Whether this node has a head-record fetcher wired
//!   at all is a startup fact, not a row state; see the courier note on the
//!   election-obey arm in [`live_transitions_with`].

use seam_contracts::harness::liveness::{check_liveness, LivenessFailure, StateClass, Transition};

use crate::p2p::projection_reconcile::{
    conductor_missing_should_route_to_adopt, declared_divergence_should_route_to_contest,
    gapfill_would_self_elect, timeout_should_route_to_adopt,
};
use crate::services::head_adoption::{
    adopt_before_author_param, decide_head_action, should_probe_election, HeadDecision,
};

/// The `config.contest_two_way_declared` switch as shipped (`default_true()` in
/// `config.rs`). Leg 3 models the default pod, not a hand-tuned one.
const CONTEST_ENABLED: bool = true;

/// Stand-in for whatever action hash a declared row carries. Only its
/// presence/absence is load-bearing — `gapfill_would_self_elect` takes
/// `Option<&str>` and reads exactly that.
const LOCAL_HEAD: &str = "uhCkk-local-declared-head";

// ---------------------------------------------------------------------------
// The state space — ONE table, shared by all three legs.
// ---------------------------------------------------------------------------

/// What the own conductor answered for this id on this sweep
/// (`conductor_writes::call_resolve_content_head`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OwnAnswer {
    /// `Ok(Some(head))` with `canonical == true` — a real election result.
    Canonical,
    /// `Ok(Some(head))` with `canonical == false` — the root-author fallback a
    /// conductor gives while the canonical link has not gossiped in.
    Fallback,
    /// `Ok(None)` — this conductor holds no chain for an id the projection carries
    /// (the MISSING class, the largest live heal outcome).
    Missing,
    /// `Err(transient)` — the conductor did not answer. Absence is not established
    /// in either direction (C4).
    Timeout,
}

/// What a peer advertises for this id in `PeerHeadHints`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PeerHint {
    /// No peer advertises a declaration.
    Absent,
    /// A peer advertises the head this row already declares.
    Agreeing,
    /// A peer advertises a head this row does not hold.
    Diverging,
}

impl PeerHint {
    /// `peer_head_hints.contains_key(&id)` — the `peer_advertises_declaration`
    /// input shared by four of the five live predicates.
    const fn advertises(self) -> bool {
        !matches!(self, PeerHint::Absent)
    }
}

/// One content row at one node, as the reconcile pipeline sees it.
///
/// The four fields are exactly the inputs the registered decision points consume,
/// given names: `conductor` → `answer_canonical` / `transient` / `head_answer_present`,
/// `peer` → `peer_advertises_declaration` / `peer_advertises_different_head`,
/// `declared` → `local_declared`, `election` → `local_has_canonical_election`
/// (`content.canonical_declared_at IS NOT NULL`).
#[derive(Clone, Copy, PartialEq, Eq)]
struct RowState {
    conductor: OwnAnswer,
    declared: bool,
    election: bool,
    peer: PeerHint,
}

impl RowState {
    const fn new(conductor: OwnAnswer, declared: bool, election: bool, peer: PeerHint) -> Self {
        Self {
            conductor,
            declared,
            election,
            peer,
        }
    }

    /// `peer_advertises_different_head`, computed the way the live call site
    /// computes it (`projection_reconcile.rs`, the declared-divergence admission):
    /// `declared.as_deref().is_none_or(|d| d.trim() != hint.head_action_hash.trim())`
    /// — so a hint on an UNDECLARED row differs by construction.
    const fn peer_differs(self) -> bool {
        self.peer.advertises() && (!self.declared || matches!(self.peer, PeerHint::Diverging))
    }

    /// The `local_declared: Option<&str>` argument `gapfill_would_self_elect` takes.
    const fn declared_head(self) -> Option<&'static str> {
        if self.declared {
            Some(LOCAL_HEAD)
        } else {
            None
        }
    }
}

/// Compact, greppable rendering — this is what a counterexample prints, so it is
/// written to be read in a panic message rather than to look like a struct dump.
impl core::fmt::Debug for RowState {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{:?}/{}/{}/peer={:?}",
            self.conductor,
            if self.declared {
                "declared"
            } else {
                "undeclared"
            },
            if self.election {
                "election"
            } else {
                "no-election"
            },
            self.peer
        )
    }
}

/// The named corner the 2026-08-02 shift deadlocked on: both sides declare, the own
/// conductor can only offer its fallback root, and no election stands behind the row.
const TWO_WAY_DECLARED: RowState =
    RowState::new(OwnAnswer::Fallback, true, false, PeerHint::Diverging);

/// A dead-end shared by BOTH historical sets: the own conductor answers with a
/// genuinely CANONICAL head and the row still cannot move (a manual-only exit in
/// the 2026-07-11 and the 2026-08-02 predicate sets alike). The corner that
/// actually discriminates the two is `UNDECLARED_WITH_A_DIVERGING_PEER` below.
const CANONICAL_ANSWER_ON_A_DECLARED_ROW: RowState =
    RowState::new(OwnAnswer::Canonical, true, false, PeerHint::Diverging);

/// The corner that separates the two historical sets in the other direction: an
/// UNDECLARED row with a diverging peer. 2026-08-02 could adopt the peer's
/// declaration (terminal); 2026-07-11 had no adopt arm at all and filled with its
/// own root instead, trapping the row.
const UNDECLARED_WITH_A_DIVERGING_PEER: RowState =
    RowState::new(OwnAnswer::Fallback, false, false, PeerHint::Diverging);

/// The full 4 × 2 × 2 × 3 = 48-state product.
///
/// Exhaustive by construction. The harness checks exactly what is enumerated, so a
/// product is used rather than a hand-picked list: hand-picking is how the one dead
/// corner goes unlisted, and 48 is far under [`LivenessBudget`]'s 4096 ceiling.
///
/// [`LivenessBudget`]: seam_contracts::harness::liveness::LivenessBudget
fn state_space() -> Vec<RowState> {
    let mut states = Vec::with_capacity(48);
    for conductor in [
        OwnAnswer::Canonical,
        OwnAnswer::Fallback,
        OwnAnswer::Missing,
        OwnAnswer::Timeout,
    ] {
        for declared in [false, true] {
            for election in [false, true] {
                for peer in [PeerHint::Absent, PeerHint::Agreeing, PeerHint::Diverging] {
                    states.push(RowState::new(conductor, declared, election, peer));
                }
            }
        }
    }
    states
}

/// The classifier — **shared by all three legs, deliberately**.
///
/// Only the transitions differ between legs, because transitions are what a
/// predicate set determines. If each leg brought its own classifier, a historical
/// leg could be made to fail (or pass) by reclassifying rather than by its rules,
/// and the demonstration would prove nothing. Both non-`Live` arms carry the written
/// justification the harness requires: the classifier is the one place a caller can
/// defeat this instrument.
fn classify(s: &RowState) -> StateClass {
    if !s.declared && s.election {
        return StateClass::Unreachable(
            "an election cannot stand behind a declaration that does not exist: \
             `canonical_declared_at` is written only alongside `declared_head_action_hash`, \
             and `declared_head_with_election` reads the pair from one row",
        );
    }
    if !s.declared && matches!(s.peer, PeerHint::Agreeing) {
        return StateClass::Unreachable(
            "`peer_advertises_different_head` is computed as \
             `declared.is_none_or(|d| d != hint)`, so a peer hint on an UNDECLARED row is \
             Diverging by construction — 'agreeing' has no referent until the row declares",
        );
    }
    if s.declared && !matches!(s.peer, PeerHint::Diverging) {
        return StateClass::Terminal(
            "the row carries a declaration and no peer advertises a different head — the \
             head-election pipeline owes nothing further for this id. Forward-monotonic \
             refresh of an already-agreed head is `canonical_move_verdict`'s question (C2), \
             registered separately, and is out of this table's scope",
        );
    }
    if !s.declared && matches!(s.peer, PeerHint::Absent) && s.conductor != OwnAnswer::Canonical {
        return StateClass::Terminal(
            "no head exists anywhere for this id — this is `decide_head_action`'s Author arm, \
             and it is an end rather than a stall: under \
             `HeadElection::PreserveExistingDeclaration` a self-authored root is NOT a \
             declaration, so a later declaration from any peer at any time wins outright",
        );
    }
    StateClass::Live
}

// ---------------------------------------------------------------------------
// LEG 3 — the LIVE predicate set, called directly.
// ---------------------------------------------------------------------------

/// Drive the live decision surface over the table, at the shipped configuration.
fn live_transitions(s: &RowState) -> Vec<Transition<RowState>> {
    live_transitions_with(s, CONTEST_ENABLED)
}

/// Drive the live decision surface over the table.
///
/// Every branch below is decided by a call into production code — no verdict is
/// restated here. The dispatch ORDER mirrors `try_adopt_canonical_head` and
/// `heal_content`: transient class, then the election-obey arm, then heal-leg
/// admission, then the decision rule, then the peer's own automated moves.
///
/// `contest_enabled` is the real `config.contest_two_way_declared` switch, threaded
/// rather than hardcoded so a test can flip the shipped lever and watch the *same
/// production code* answer differently — see
/// [`tests::disabling_the_contest_arm_returns_the_live_code_to_the_two_way_declared_deadlock`].
/// Both the local decision and the peer's read it, because both pods run this rule.
fn live_transitions_with(s: &RowState, contest_enabled: bool) -> Vec<Transition<RowState>> {
    if !classify(s).is_live() {
        return Vec::new();
    }

    let answer_canonical = s.conductor == OwnAnswer::Canonical;
    let head_answer_present = matches!(s.conductor, OwnAnswer::Canonical | OwnAnswer::Fallback);
    let peer_declares = s.peer.advertises();

    // (0) TRANSIENT CLASS. `StorageError::Timeout` is the recoverable class by
    // construction; the bounded retry ladder plus next-sweep re-enqueue keep the
    // row in the pipeline. `timeout_should_route_to_adopt` decides WHICH list it
    // lands on, which is what the label records — and the pre-cure alternative
    // (the drop-forever hole) is exactly what that predicate closed.
    if s.conductor == OwnAnswer::Timeout {
        let routed = timeout_should_route_to_adopt(true, peer_declares);
        return vec![Transition::automated(if routed {
            "bounded retry ladder; timeout_should_route_to_adopt keeps the row on the adopt list"
        } else {
            "bounded retry ladder; the gap sweep re-discovers the row on the next tick"
        })
        .to(RowState {
            conductor: OwnAnswer::Missing,
            ..*s
        })];
    }

    let mut out = Vec::new();

    // (1b) ELECTION-OBEY ARM. Scoped to the conductor-missing class, and it runs
    // BEFORE the decision rule: obeying an election the DHT has already settled
    // beats contesting it again. This is wave 5's cure — `resolve_canonical_election`
    // reads the election WITHOUT target retrieval, so a pod with no local chain can
    // still obey.
    //
    // COURIER GATE (2026-08-03 honesty fix). `try_obey_visible_election` needs
    // BOTH halves: a visible election AND a byte courier. With `(hint, fetcher)`
    // not `(Some, Some)` it returns `None` having moved nothing — the
    // `no_courier` exit on `elohim_content_election_obey_probe_total`. Modeling
    // the arm on election-visibility alone claimed a move the live code does not
    // make.
    //
    // This adds no DIMENSION to the table, deliberately. The courier's per-row
    // half IS `peer_head_hints.get(id)` — the same map `peer_advertises_declaration`
    // reads — so it is already carried here as `PeerHint::advertises()`. A new
    // `RowState` bit would double the 48-state product, force leg 2 to model a
    // mechanism that did not exist in 2026-07-11 (`PeerHeadHints` postdates it
    // entirely), and change the counts the three legs are compared on; that is
    // the trade the F-B backoff witness declined for the same reason. The
    // courier's OTHER half (whether this node has a `HeadRecordFetcher` wired at
    // all) is a startup fact, not a row state, and stays outside the space — see
    // the module doc's configuration note.
    let courier_present = s.peer.advertises();
    if should_probe_election(head_answer_present) && s.election && courier_present {
        out.push(
            Transition::automated(
                "try_obey_visible_election: read the visible election without target retrieval, \
                 fetch the elected head's bytes from the advertising peer, and stamp them under \
                 the election's own ordering",
            )
            // A row that OBEYED lands settled: `stamp_declared_head_mode` writes
            // `declared_head_action_hash` AND, under `HealCanonical` with the
            // election's ordering, `canonical_declared_at` — and the head it now
            // declares is the elected one, which is what the peer advertises.
            // Leaving `declared`/`election` untouched described a row that had
            // agreed with the fleet while still carrying no declaration, which is
            // not a state this arm can produce. It is a no-op for the corner the
            // arm reaches today (already declared, already elected), and the
            // point is that it stays honest if that corner ever widens.
            .to(RowState {
                declared: true,
                election: true,
                peer: PeerHint::Agreeing,
                ..*s
            }),
        );
    }

    // (2) HEAL-LEG ADMISSION. Which candidate list does this row reach? The wall
    // that made the 2026-08-02 deadlock invisible lived here: a declared+divergent
    // row was refused (`SkippedDeclared`), marked completed, and pushed to NO list.
    let admitted = match s.conductor {
        // A canonical answer is the canonical channel's own business
        // (`StampMode::HealCanonical` → `canonical_move_verdict`); the pre-flight
        // sees it and answers AdoptLocal.
        OwnAnswer::Canonical => true,
        OwnAnswer::Fallback => {
            gapfill_would_self_elect(answer_canonical, peer_declares, s.declared_head())
                || declared_divergence_should_route_to_contest(
                    answer_canonical,
                    s.declared,
                    s.peer_differs(),
                    s.election,
                )
        }
        OwnAnswer::Missing => conductor_missing_should_route_to_adopt(peer_declares),
        OwnAnswer::Timeout => unreachable!("handled by the transient arm above"),
    };

    // (3) THE DECISION RULE. `peer_divergent_anchor` is `false` here because
    // anchor-plane divergence is not a dimension of this table (see the module
    // doc's scope note) — the SPIN class is witnessed separately by
    // [`tests::the_spin_class_discharge_is_flag_shaped`].
    if admitted {
        match decide_head_action(
            answer_canonical,
            peer_declares,
            s.declared,
            s.election,
            contest_enabled,
            false,
        ) {
            HeadDecision::AdoptLocal => out.push(
                Transition::automated(
                    "adopt_local: HealCanonical stamp — the answer carries \
                     canonical_declared_at/canonical_earned, so the stamp records the election \
                     ordering (wave 2's cure; before it the guard compared a NULLed wall clock)",
                )
                .to(RowState {
                    declared: true,
                    election: true,
                    ..*s
                }),
            ),
            HeadDecision::AdoptPeer => out.push(
                Transition::automated(if s.conductor == OwnAnswer::Missing {
                    "AuthorThenAdopt: no local chain to hang a declaration on — author a root, \
                     then declare the peer's verified head"
                } else {
                    "adopt_peer: verified fetch of the peer's head record, then declare it \
                     through the own conductor"
                })
                .to(RowState {
                    declared: true,
                    peer: PeerHint::Agreeing,
                    ..*s
                }),
            ),
            HeadDecision::ContestPeer => {
                // A contest is a source-chain write. A conductor-missing pod has no
                // chain, so its contest fails `contest_failed{no_local_chain}` (887
                // in the first live window) — that class is served by the obey arm
                // above once the chain-holding side supplies the election.
                if s.conductor != OwnAnswer::Missing {
                    out.push(
                        Transition::automated(
                            "contest_peer: mint a canonical-head candidate naming the peer's \
                             head — select_canonical_winner finally has a set to elect on",
                        )
                        .to(RowState {
                            election: true,
                            ..*s
                        }),
                    );
                }
            }
            // Hold and Author move nothing on this side by design. Whether that is
            // a correct refusal or a deadlock is the question this whole table asks,
            // and it is answered below rather than asserted here. ContestDivergent
            // is unreachable with `peer_divergent_anchor = false` (pinned by
            // `contest_divergent_is_reachable_only_from_the_undeclared_divergent_corner`
            // in `head_adoption`); its liveness claim lives in the SPIN witness.
            HeadDecision::Hold | HeadDecision::Author | HeadDecision::ContestDivergent => {}
        }
    }

    // (4) THE PEER'S OWN AUTOMATED MOVES. C3 asks whether THE FLEET has a move, not
    // whether this node does — and the peer runs these same predicates. Scope: the
    // main table assumes the peer holds a chain; both-sides-conductor-missing is the
    // documented open residual witnessed separately.
    if s.declared && matches!(s.peer, PeerHint::Diverging) {
        if s.election {
            out.push(
                Transition::automated(
                    "the peer resolves the SAME DHT election — select_canonical_winner is \
                     deterministic over one shared anchor — and its own obey arm takes it",
                )
                .to(RowState {
                    peer: PeerHint::Agreeing,
                    ..*s
                }),
            );
        } else if declared_divergence_should_route_to_contest(false, true, true, false)
            && decide_head_action(false, true, true, false, contest_enabled, false)
                == HeadDecision::ContestPeer
        {
            // The peer's own state, run through the peer's own two gates: it is
            // admitted at the SkippedDeclared refusal site, and its decision rule
            // answers ContestPeer. Both are the LIVE predicates — a peer runs the
            // same binary with the same switch.
            out.push(
                Transition::automated(
                    "the chain-holding peer admits its own declared-divergence row at the \
                     SkippedDeclared refusal site and contests, minting the election",
                )
                .to(RowState {
                    election: true,
                    ..*s
                }),
            );
        }
    }

    // The one exit that still exists when every automated arm has refused: R1's
    // deploy declare-cycle. Appending it only ever turns a `dead_end` finding into a
    // `manual_only_exit` one — both fail — so it can never launder a hole into a
    // pass, and it keeps the live leg's report describing the same world the two
    // historical legs are measured in.
    if out.is_empty() {
        out.push(deploy_declare_cycle(s));
    }

    out
}

// ---------------------------------------------------------------------------
// LEG 1 — the pre-wave-5 predicate set, reconstructed as plain data.
// ---------------------------------------------------------------------------

/// `decide_head_action` **as it stood at `496a4aba8^`** — three inputs, no
/// `ContestPeer` arm, blind to whether an election stands behind the row.
///
/// Transcribed verbatim from the historical source; it calls nothing. The blindness
/// is the point: the cure keys on a dimension the old rule had no input for.
///
/// ```text
/// if conductor_canonical      { AdoptLocal }
/// else if peer_declared && !local_declared { AdoptPeer }
/// else if local_declared      { Hold }
/// else                        { Author }
/// ```
fn pre_wave5_decide(
    conductor_canonical: bool,
    peer_declared: bool,
    local_declared: bool,
) -> HeadDecision {
    if conductor_canonical {
        HeadDecision::AdoptLocal
    } else if peer_declared && !local_declared {
        HeadDecision::AdoptPeer
    } else if local_declared {
        HeadDecision::Hold
    } else {
        HeadDecision::Author
    }
}

/// The four stacked walls of the 2026-08-02 deadlock, restated as plain data.
///
/// Sources (read, not called): `496a4aba8^` for `head_adoption.rs`,
/// `projection_reconcile.rs`, and `content_diesel.rs`; the wall numbering and the
/// live evidence are `genesis/data/timeline/backlog/content-gap-limit-cycle-blocks-convergence.md`
/// §"RCA v3". Cured by `496a4aba8`, `134331c83`, `da8975176`.
///
/// 1. **Supply deadlock** — [`pre_wave5_decide`] answers `Hold` for every declared
///    row, so `AdoptPeer` (the only automated canonical-link minter) is unreachable
///    once the local row declares. The `canonical_head` anchor stayed EMPTY for the
///    whole class and `select_canonical_winner` never ran.
/// 2. **Consumption wall** — `StampMode::HealCanonical` moved a declared row only
///    with `provably_newer = matches!((declared_at, stored_declared_at), (Some(i),
///    Some(s)) if i > s)`, and the deploy PATCH path had NULLed the stored side.
///    `AdoptLocal` was therefore structurally unreachable on any declared row.
/// 3. **Declare wall** — a contest/declare on a conductor-missing id failed the
///    target-independent no-chain gate.
/// 4. **Admission wall** — `declared_divergence_should_route_to_contest` did not
///    exist. A declared+divergent row got `SkippedDeclared`, was marked completed,
///    and reached no candidate list at all.
fn pre_wave5_transitions(s: &RowState) -> Vec<Transition<RowState>> {
    if !classify(s).is_live() {
        return Vec::new();
    }

    // The transient class behaves the same in every era; modeling it identically
    // across the legs keeps it from being the thing that separates them.
    if s.conductor == OwnAnswer::Timeout {
        return vec![Transition::automated(
            "bounded retry ladder; the row is re-asked on the next sweep",
        )
        .to(RowState {
            conductor: OwnAnswer::Missing,
            ..*s
        })];
    }

    let answer_canonical = s.conductor == OwnAnswer::Canonical;
    let peer_declares = s.peer.advertises();
    let mut out = Vec::new();

    // WALL 4 — ADMISSION. The gapfill route required an UNDECLARED row, and nothing
    // else admitted a declared one.
    let admitted = match s.conductor {
        OwnAnswer::Canonical => true,
        OwnAnswer::Fallback => !answer_canonical && peer_declares && !s.declared,
        OwnAnswer::Missing => peer_declares,
        OwnAnswer::Timeout => unreachable!("handled above"),
    };

    if admitted {
        // WALL 1 — SUPPLY.
        match pre_wave5_decide(answer_canonical, peer_declares, s.declared) {
            HeadDecision::AdoptLocal => {
                // WALL 2 — CONSUMPTION. Filling an undeclared row was fine; MOVING a
                // declared one needed an ordering proof the deploy path had erased.
                if !s.declared {
                    out.push(
                        Transition::automated(
                            "heal_content_one fills the undeclared row with the conductor's \
                             canonical answer",
                        )
                        .to(RowState {
                            declared: true,
                            ..*s
                        }),
                    );
                }
            }
            HeadDecision::AdoptPeer => {
                // WALL 3 — DECLARE.
                if s.conductor != OwnAnswer::Missing {
                    out.push(
                        Transition::automated(
                            "adopt_peer: fetch and declare the peer's head through the own \
                             conductor",
                        )
                        .to(RowState {
                            declared: true,
                            peer: PeerHint::Agreeing,
                            ..*s
                        }),
                    );
                }
            }
            // No ContestPeer arm existed — and no ContestDivergent either (the
            // SPIN discharge postdates this reconstruction by twelve days).
            HeadDecision::ContestPeer
            | HeadDecision::ContestDivergent
            | HeadDecision::Hold
            | HeadDecision::Author => {}
        }
    }

    if out.is_empty() {
        out.push(deploy_declare_cycle(s));
    }
    out
}

// ---------------------------------------------------------------------------
// LEG 2 — the 2026-07-11 over-broad GapFill guard, reconstructed as plain data.
// ---------------------------------------------------------------------------

/// The over-broad GapFill guard **as it stood at `8844cd1d6`** (2026-07-11), one
/// month and one seam-cluster away from leg 1.
///
/// Sources (read, not called): `8844cd1d6` — `StampMode` had exactly two variants
/// (`Declare`, `GapFill`), `StampOutcome` exactly three, and the heal call site in
/// `projection_reconcile.rs` passed `GapFill` **unconditionally**. There was no
/// canonical/fallback distinction in the answer (`ContentHeadOutput.canonical`
/// arrived with `0644132fb` later the same day), no `PeerHeadHints`, and no
/// adopt-before-author pre-flight at all (`head_adoption.rs` was created
/// 2026-07-28 by `b91ee0f95`).
///
/// The guard itself was correct and necessary: a freshly-restarted conductor falls
/// through to the root-author election while the canonical link is not yet
/// retrievable, and stamping that RESURRECTS a superseded head (the live 2026-07-11
/// 20:42:40 regression). What made it a liveness hole was its breadth — it could not
/// tell "resurrect a superseded head" from "adopt the newer canonical head", so it
/// refused both. `0644132fb`'s own commit body names the interaction: *"closes the
/// interaction where the boot-resurrection guard also blocked forward adoption."*
///
/// A different bug from leg 1, three weeks earlier, with a different signature — and
/// the same instrument catches it.
fn july11_gapfill_transitions(s: &RowState) -> Vec<Transition<RowState>> {
    if !classify(s).is_live() {
        return Vec::new();
    }

    if s.conductor == OwnAnswer::Timeout {
        return vec![Transition::automated(
            "bounded retry ladder; the row is re-asked on the next sweep",
        )
        .to(RowState {
            conductor: OwnAnswer::Missing,
            ..*s
        })];
    }

    let mut out = Vec::new();
    match s.conductor {
        // ONE stamp mode for every heal answer. The peer dimension is invisible to
        // this predicate set: `PeerHeadHints` did not exist yet.
        OwnAnswer::Canonical | OwnAnswer::Fallback if !s.declared => out.push(
            Transition::automated("heal GapFill fills the undeclared row").to(RowState {
                declared: true,
                ..*s
            }),
        ),
        // THE OVER-BROAD GUARD. A declared row is refused even when the answer is
        // the genuinely canonical one. Forward adoption is frozen.
        OwnAnswer::Canonical | OwnAnswer::Fallback => {}
        // `Ok(None)`: nothing to stamp; re-asked next sweep, and no other arm exists.
        OwnAnswer::Missing => {}
        OwnAnswer::Timeout => unreachable!("handled above"),
    }

    if out.is_empty() {
        out.push(deploy_declare_cycle(s));
    }
    out
}

/// The one exit both historical sets left standing: the deploy's `DECLARE_ONLY`
/// propagation, wired 2026-07-11 (`11ea79cb2`) and made load-bearing as R1's labeled
/// *interim* authority channel.
///
/// It is [`Agency::Deploy`], which the C3 law refuses to count: a channel that runs
/// when CI runs is exactly as unavailable to a fleet at 03:00 as a human hand. That
/// refusal is the entire finding — both historical sets had a *complete* state
/// machine, and a transition-counting check would have passed both.
///
/// [`Agency::Deploy`]: seam_contracts::harness::liveness::Agency::Deploy
fn deploy_declare_cycle(s: &RowState) -> Transition<RowState> {
    Transition::deploy(
        "deploy declare-cycle (DECLARE_ONLY propagation) — R1's labeled interim authority channel",
    )
    .to(RowState {
        declared: true,
        election: true,
        peer: PeerHint::Agreeing,
        ..*s
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every state a failure names, as rendered.
    fn violating_states(failure: &LivenessFailure) -> Vec<String> {
        failure.violations.iter().map(|v| v.state.clone()).collect()
    }

    fn names(failure: &LivenessFailure, state: RowState) -> bool {
        violating_states(failure).contains(&format!("{state:?}"))
    }

    /// The counterexample a historical leg produces, dumped for a reader who asks
    /// (`cargo test --lib liveness_contract -- --nocapture`). Captured by default, so
    /// a green run stays quiet — but the demonstration's whole value is *what the
    /// harness says*, and a report nobody can read is the C8 failure this canon
    /// names. Deliberately not an assertion: the text is prose and will drift.
    fn dump(leg: &str, failure: &LivenessFailure) {
        eprintln!("\n=== {leg} — what the C3 harness reports ===\n{failure}\n");
    }

    // -----------------------------------------------------------------------
    // The table itself.
    // -----------------------------------------------------------------------

    /// The three legs must be comparable, which means the space and the classifier
    /// are shared and fixed. If a future edit changes these counts, the regression
    /// demonstration below is measuring a different world and says so here first.
    #[test]
    fn the_shared_state_space_has_the_shape_all_three_legs_are_measured_against() {
        let states = state_space();
        assert_eq!(
            states.len(),
            48,
            "4 conductor x 2 declared x 2 election x 3 peer"
        );

        let live = states.iter().filter(|s| classify(s).is_live()).count();
        let terminal = states.iter().filter(|s| classify(s).is_terminal()).count();
        let unreachable = states
            .iter()
            .filter(|s| classify(s).is_unreachable())
            .count();

        assert_eq!(live, 13, "reachable states with work still owed");
        assert_eq!(
            terminal, 19,
            "16 settled-declared + 3 nothing-declared-anywhere"
        );
        assert_eq!(
            unreachable, 16,
            "12 election-without-declaration + 4 agreeing-while-undeclared"
        );
        assert_eq!(live + terminal + unreachable, states.len());

        // The corners the demonstration turns on must actually be in the table and
        // must actually be live — a corner classified away is a corner not checked.
        for corner in [
            TWO_WAY_DECLARED,
            CANONICAL_ANSWER_ON_A_DECLARED_ROW,
            UNDECLARED_WITH_A_DIVERGING_PEER,
        ] {
            assert!(
                states.contains(&corner),
                "{corner:?} missing from the table"
            );
            assert!(classify(&corner).is_live(), "{corner:?} is not Live");
        }
    }

    // -----------------------------------------------------------------------
    // LEG 1 — FAILS against the pre-wave-5 predicate set.
    // -----------------------------------------------------------------------

    #[test]
    fn leg1_the_pre_wave5_predicate_set_fails_the_liveness_table() {
        let states = state_space();
        let failure = check_liveness(&states, classify, pre_wave5_transitions)
            .expect_err("the four-wall deadlock must not read as live");
        dump(
            "LEG 1 — pre-wave-5 (2026-08-02 four-wall deadlock)",
            &failure,
        );

        // The law's exact shape: a COMPLETE state machine whose only exit needs a
        // hand. Not a dead end — that is what made it survive five serial waves.
        assert!(
            failure.has_class(
                seam_contracts::harness::liveness::LivenessViolationClass::ManualOnlyExit
            ),
            "expected manual_only_exit, got:\n{failure}"
        );

        // The counterexample must NAME the corner the fleet actually deadlocked on.
        assert!(
            names(&failure, TWO_WAY_DECLARED),
            "the two-way-declared corner {TWO_WAY_DECLARED:?} is unnamed:\n{failure}"
        );

        // …and it must name the whole composition at once, not the first wall. The
        // deadlock cost five serial production waves precisely because each wall was
        // only visible after the previous was cured.
        assert!(
            failure.total_violations >= 7,
            "expected the whole stuck class, got {}:\n{failure}",
            failure.total_violations
        );

        // The instrument's own honesty: the composition check RAN (it did not
        // silently skip and read as a pass).
        assert!(
            failure.report.progress_checked,
            "composition check skipped:\n{failure}"
        );
    }

    // -----------------------------------------------------------------------
    // LEG 2 — FAILS against the 2026-07-11 over-broad GapFill guard.
    // -----------------------------------------------------------------------

    #[test]
    fn leg2_the_july11_over_broad_gapfill_guard_fails_the_liveness_table() {
        let states = state_space();
        let failure = check_liveness(&states, classify, july11_gapfill_transitions)
            .expect_err("a guard that refuses forward adoption must not read as live");
        dump("LEG 2 — 2026-07-11 over-broad GapFill guard", &failure);

        assert!(
            failure.has_class(
                seam_contracts::harness::liveness::LivenessViolationClass::ManualOnlyExit
            ),
            "expected manual_only_exit, got:\n{failure}"
        );

        // The signature of THIS bug: the own conductor answers with a genuinely
        // CANONICAL head and the declared row still cannot move.
        assert!(
            names(&failure, CANONICAL_ANSWER_ON_A_DECLARED_ROW),
            "the frozen-forward-adoption corner {CANONICAL_ANSWER_ON_A_DECLARED_ROW:?} is \
             unnamed:\n{failure}"
        );

        assert!(
            failure.report.progress_checked,
            "composition check skipped:\n{failure}"
        );
    }

    /// The anti-fitting evidence, stated as a claim rather than a hope: the two
    /// historical sets fail the SAME table with DIFFERENT signatures.
    ///
    /// The discriminating corner is an undeclared row with a diverging peer. The
    /// 2026-08-02 set could adopt the peer's declaration outright and reach a settled
    /// state; the 2026-07-11 set had no adopt arm at all, filled the row with its own
    /// root instead, and trapped it. A harness fitted to one deadlock could not
    /// produce two different verdicts here.
    #[test]
    fn the_two_historical_sets_fail_the_same_table_with_different_signatures() {
        let states = state_space();
        let leg1 = check_liveness(&states, classify, pre_wave5_transitions).unwrap_err();
        let leg2 = check_liveness(&states, classify, july11_gapfill_transitions).unwrap_err();

        assert!(
            !names(&leg1, UNDECLARED_WITH_A_DIVERGING_PEER),
            "2026-08-02's set could adopt a peer declaration onto an undeclared row:\n{leg1}"
        );
        assert!(
            names(&leg2, UNDECLARED_WITH_A_DIVERGING_PEER),
            "2026-07-11's set had no adopt arm — the row should be trapped:\n{leg2}"
        );

        let mut a = violating_states(&leg1);
        let mut b = violating_states(&leg2);
        a.sort();
        b.sort();
        assert_ne!(
            a, b,
            "two different deadlocks must not produce one identical report"
        );
    }

    // -----------------------------------------------------------------------
    // LEG 3 — PASSES against the live predicate set.
    // -----------------------------------------------------------------------

    #[test]
    fn leg3_the_live_predicate_set_passes_the_liveness_table() {
        let states = state_space();
        let report =
            check_liveness(&states, classify, live_transitions).unwrap_or_else(|failure| {
                // Not a massage point. If this ever fires, the counterexample below is a
                // candidate REAL liveness hole in shipped code and belongs in triage, not
                // in a widened table.
                panic!(
                    "LIVE liveness contract (C3) failed — treat as a production finding, \
                 not a test to relax:\n{failure}"
                )
            });

        // A pass is only worth what the instrument measured, so assert the
        // measurement too (C7 applied to our own green).
        assert_eq!(report.states_checked, 48);
        assert_eq!(report.live_states, 13);
        assert!(
            report.progress_checked,
            "per-state legality without the composition check is exactly what four \
             locally-correct guards each passed: {report}"
        );
        assert!(report.transitions_enumerated >= 13, "{report}");
    }

    /// The election-obey arm must be modeled the way it actually behaves: it
    /// needs a byte COURIER as well as a visible election, and a row that obeys
    /// lands SETTLED.
    ///
    /// Both halves are honesty fixes from the 2026-08-03 trace, and both are
    /// no-ops for the single corner the arm reaches in today's table
    /// (`Missing/declared/election/peer=Diverging`, where a courier is present by
    /// construction and the row is already declared+elected). That is exactly why
    /// they need a test: a claim that happens to be true only because of where
    /// the classifier draws its Live/Terminal line will silently become false
    /// when that line moves. This asserts the MODEL's own shape rather than a
    /// verdict, so it fails on the edit that would otherwise re-introduce the
    /// overclaim.
    #[test]
    fn the_election_obey_transition_is_courier_gated_and_lands_settled() {
        let mut fired = 0usize;
        for state in state_space() {
            for transition in live_transitions(&state) {
                if !transition.label.starts_with("try_obey_visible_election") {
                    continue;
                }
                fired += 1;

                assert!(
                    state.peer.advertises(),
                    "{state:?}: the obey arm is modeled as moving this row, but no peer \
                     advertises a head — `try_obey_visible_election` returns None with no \
                     courier (the no_courier probe exit)"
                );

                let landed = transition.to.expect("the obey arm models its target");
                assert!(
                    landed.declared && landed.election,
                    "{state:?} -> {landed:?}: a row that obeyed carries the elected head AND \
                     the election ordering that moved it"
                );
                assert!(
                    matches!(landed.peer, PeerHint::Agreeing),
                    "{state:?} -> {landed:?}: the row now holds the head the peer advertises"
                );
                assert!(
                    classify(&landed).is_terminal(),
                    "{state:?} -> {landed:?}: obeying settles the row; if this lands Live the \
                     arm is modeled as making progress it does not make"
                );
            }
        }
        assert!(
            fired >= 1,
            "the obey arm fired nowhere in the table — wave 5's cure would be unmodeled, and \
             leg 3's green would no longer say anything about it"
        );
    }

    /// The cure is load-bearing, not incidental: flip the one shipped switch that
    /// restores the pre-2026-08-02 behaviour and the same live code deadlocks again.
    ///
    /// This is the strongest available statement that leg 3's green comes from the
    /// contest arm rather than from a generous table — and it is produced by the
    /// LIVE `decide_head_action`, not by a reconstruction.
    #[test]
    fn disabling_the_contest_arm_returns_the_live_code_to_the_two_way_declared_deadlock() {
        let states = state_space();
        // The SAME production code, with the shipped switch flipped — no
        // reconstruction, no filtering of the result. `contest_two_way_declared =
        // false` collapses ContestPeer back into Hold on both pods, which is exactly
        // the pre-2026-08-02 behaviour the config comment claims it restores.
        let failure = check_liveness(&states, classify, |s| live_transitions_with(s, false))
            .expect_err(
                "with the contest arm off, the two-way-declared class has no automated exit",
            );

        assert!(
            names(&failure, TWO_WAY_DECLARED),
            "expected the two-way-declared corner back:\n{failure}"
        );
    }

    // -----------------------------------------------------------------------
    // F-B: the contest backoff, witnessed under the same instrument.
    // -----------------------------------------------------------------------

    /// **A backed-off id is DELAYED, never excluded** — the C3 obligation the
    /// F-B throughput lever takes on.
    ///
    /// Witnessed separately rather than folded into the main table on purpose.
    /// Backoff is a *scheduling* dimension, not a head-election one: adding it
    /// to [`RowState`] would double the 48-state product, force both historical
    /// legs to model a mechanism that did not exist in 2026-07-11 or
    /// 2026-08-02, and change the counts the three legs are compared on. The
    /// main table stays exactly as it was; this witnesses the new state space
    /// beside it — the same shape as
    /// [`both_sides_conductor_missing_is_a_known_open_residual`].
    ///
    /// Both legs below are driven by LIVE code: leg A calls
    /// [`crate::services::contest_backoff::backoff_is_active`] and the real
    /// ledger; leg B is a plain-data reconstruction of the pre-F-B
    /// self-candidacy ledger (claim-on-attempt, never released) and calls
    /// nothing — the same reconstruction discipline as legs 1 and 2.
    #[test]
    fn the_contest_backoff_is_never_a_permanent_exclusion() {
        use crate::metrics::ContestSkip;
        use crate::services::contest_backoff;
        use std::time::Duration;

        #[derive(Debug, PartialEq, Eq, Clone, Copy)]
        enum Backoff {
            /// A contest for this id failed predictably; the id is serving its
            /// window and the sweep de-prioritises it.
            Serving,
            /// Eligible to contest again.
            Contestable,
        }

        const WINDOW: Duration = Duration::from_secs(3600);

        fn classify(s: &Backoff) -> StateClass {
            match s {
                Backoff::Contestable => StateClass::Terminal(
                    "the id is contestable — the scheduling question this table asks is \
                     answered, and whether the contest then SUCCEEDS is the main table's \
                     question, not this one",
                ),
                Backoff::Serving => StateClass::Live,
            }
        }

        // EXIT 1 — time expiry, from the live predicate, asked at exactly the
        // window: if it still held here the backoff would have no finite horizon.
        let expiry_releases = !contest_backoff::backoff_is_active(WINDOW, WINDOW);

        // EXIT 2 — local chain arrival, exercised against the REAL ledger. This
        // is the exit that matters inside ONE tick: the adopt sweep runs before
        // both witness sweeps, so an id can be backed off and authored seconds
        // apart.
        //
        // Exercised ONCE, here, rather than inside the transition closure: the
        // closure is called per state visit, and taking a mutex inside a
        // repeatedly-invoked callback is a deadlock waiting for a harness that
        // ever nests. Serialised against the contest_backoff module's own ledger
        // tests, which `cargo test` runs in parallel threads of ONE process
        // (this raced their `reset()` before it was fixed).
        let arrival_releases = {
            let _exclusive = contest_backoff::test_exclusive();
            contest_backoff::note("c3:witness", ContestSkip::NoLocalChainBackoff);
            let held_before = contest_backoff::skip_class(
                "c3:witness",
                contest_backoff::BackoffWindows::uniform(WINDOW),
            )
            .is_some();
            contest_backoff::note_local_chain_arrived("c3:witness");
            let released = contest_backoff::skip_class(
                "c3:witness",
                contest_backoff::BackoffWindows::uniform(WINDOW),
            )
            .is_none();
            held_before && released
        };

        // ── LEG A: the live backoff. Both exits driven by production code. ──
        let report = check_liveness(
            &[Backoff::Serving, Backoff::Contestable],
            classify,
            |s| match s {
                Backoff::Contestable => vec![],
                Backoff::Serving => {
                    let mut out = Vec::new();
                    if expiry_releases {
                        out.push(
                            Transition::automated(
                                "window expiry: backoff_is_active(elapsed >= window) is false, so \
                                 the sweep re-admits the id at full priority",
                            )
                            .to(Backoff::Contestable),
                        );
                    }
                    if arrival_releases {
                        out.push(
                            Transition::automated(
                                "note_local_chain_arrived: an author path landed a chain, so the \
                                 no-chain verdict is stale and the entry is cleared at once",
                            )
                            .to(Backoff::Contestable),
                        );
                    }
                    out
                }
            },
        )
        .unwrap_or_else(|failure| {
            // Not a massage point — same law as leg 3. A backoff with no
            // automated exit is a production finding.
            panic!(
                "the contest backoff has no automated exit — that is a C3 violation in \
                 shipped code, not a test to relax:\n{failure}"
            )
        });
        assert!(
            report.transitions_enumerated >= 2,
            "both exits (expiry AND chain arrival) must be present, not just one: {report}"
        );
        assert!(report.progress_checked, "{report}");

        // ── LEG B: the pre-F-B self-candidacy ledger, reconstructed. ──
        //
        // Transcribed from `head_adoption.rs` at `da8975176`: the claim was
        // taken BEFORE the declare and never handed back, so a single failed
        // self-candidacy retired `(id, own_head)` for the life of the process.
        // The live fingerprint was `contest_self_head = 4` against
        // `contest_failed{fetch_none} = 603`. It calls nothing — a
        // reconstruction that computed its expectation from today's code would
        // track every future edit and could never fail.
        let failure = check_liveness(&[Backoff::Serving, Backoff::Contestable], classify, |s| {
            match s {
                Backoff::Contestable => vec![],
                // No expiry, no release, no clear: the only way back was a pod
                // restart, which is exactly the agency C3 refuses to count.
                Backoff::Serving => vec![Transition::deploy(
                    "process restart drops the in-memory ledger — an operator/deploy action, \
                     never an automated one",
                )
                .to(Backoff::Contestable)],
            }
        })
        .expect_err(
            "claim-on-attempt with no release is a PERMANENT exclusion — if this now passes, \
             the reconstruction has drifted into calling live code",
        );
        assert!(
            failure.has_class(
                seam_contracts::harness::liveness::LivenessViolationClass::ManualOnlyExit
            ),
            "expected manual_only_exit for the never-released claim:\n{failure}"
        );
        dump(
            "F-B negative control — pre-release self-candidacy ledger",
            &failure,
        );
    }

    // -----------------------------------------------------------------------
    // The documented open residual — witnessed, not hidden.
    // -----------------------------------------------------------------------

    /// **Both-sides-conductor-missing is a KNOWN OPEN residual, and this test asserts
    /// the harness NAMES it.** It is not a regression and not a new finding.
    ///
    /// RCA v3 (`genesis/data/timeline/backlog/content-gap-limit-cycle-blocks-convergence.md`)
    /// records it verbatim: *"Both-sides-missing pairs — unreachable by any storage
    /// arm"*, with the operator-facing follow-up being a coordinator-zome decision
    /// (declare with a validated carried record when no local chain exists).
    ///
    /// The main table above scopes it out — it assumes the peer holds a chain — and
    /// that scoping is only honest if the excluded corner is stated somewhere the
    /// instrument can see it. Here it is: the exit that exists today is the deploy
    /// declare-cycle, which C3 refuses to count, so the harness reports
    /// `manual_only_exit`. When the follow-up lands, this test flips to a pass and
    /// the corner folds into the main table.
    #[test]
    fn both_sides_conductor_missing_is_a_known_open_residual() {
        #[derive(Debug, PartialEq, Eq, Clone, Copy)]
        enum BothMissing {
            /// Two pods, both declared, both diverging, NEITHER holding a source
            /// chain for the id — so neither can mint a canonical-head candidate.
            NeitherPodHoldsAChain,
            /// One elected head, obeyed on both sides.
            Converged,
        }

        /// The residual's state table, parameterised by the ONE config bit that
        /// changes it — modeled exactly the way [`CONTEST_ENABLED`] models
        /// `config.contest_two_way_declared`, and driven by the same live
        /// predicate the production arms consult
        /// ([`crate::services::head_adoption::adopt_before_author_param`]).
        ///
        /// `carried_bytes_in_hand` is held TRUE here on purpose: the reachable
        /// share of the residual is exactly the pairs where at least one side can
        /// fetch the winner's bytes from an advertiser. A pair with no servable
        /// record is a different (still-open) state that no flag can move, and
        /// the arm honestly returns `None` for it — see the convergence-math note
        /// in `services::head_adoption`'s module doc.
        fn residual_transitions(
            adopt_before_author: bool,
        ) -> impl Fn(&BothMissing) -> Vec<Transition<BothMissing>> {
            move |s: &BothMissing| match s {
                BothMissing::Converged => vec![],
                BothMissing::NeitherPodHoldsAChain => {
                    // The live predicate, not a re-statement of it: if the two
                    // conditions ever stop being ANDed, this table changes shape.
                    let bypass_licensed = adopt_before_author_param(adopt_before_author, true);
                    if bypass_licensed {
                        vec![Transition::automated(
                            "adopt-before-author: the pod fetches the advertiser's head Record \
                             over view-federation and declares with adopt_before_author=true; \
                             the zome re-derives the action hash, author signature, \
                             entry↔action binding and content id, then writes the canonical \
                             link — the arbiter finally has a candidate and both pods obey the \
                             election through the ordinary heal path",
                        )
                        .to(BothMissing::Converged)]
                    } else {
                        // conductor_missing_should_route_to_adopt admits the row, and
                        // decide_head_action answers ContestPeer — but the declare fails
                        // the no-chain gate on BOTH sides, so nothing is minted and there
                        // is no election for the obey arm to read.
                        vec![Transition::deploy(
                            "deploy declare-cycle, or the operator flip of \
                             ELOHIM_ADOPT_BEFORE_AUTHOR (the capability ships dormant, so \
                             turning it on is an operator act — which C3 does not count as \
                             an automated exit)",
                        )
                        .to(BothMissing::Converged)]
                    }
                }
            }
        }

        fn classify(s: &BothMissing) -> StateClass {
            match s {
                BothMissing::Converged => {
                    StateClass::Terminal("both pods obey one elected head; nothing further is owed")
                }
                BothMissing::NeitherPodHoldsAChain => StateClass::Live,
            }
        }

        const STATES: &[BothMissing] =
            &[BothMissing::NeitherPodHoldsAChain, BothMissing::Converged];

        // ── LEG A — FLAG OFF (the shipped default). The residual is OPEN: the
        // only exit is an operator/deploy act, which C3 refuses to count. This
        // is the same verdict this witness has always reported, and it must stay
        // reported while the capability ships dormant — a dormant cure is not a
        // cure, and a green table here would claim otherwise.
        let failure = check_liveness(STATES, classify, residual_transitions(false)).expect_err(
            "with ELOHIM_ADOPT_BEFORE_AUTHOR unset the both-sides-missing pair still has no \
             automated exit — if this now passes, the default changed and the honesty matrix \
             is stale",
        );
        assert!(
            failure.has_class(
                seam_contracts::harness::liveness::LivenessViolationClass::ManualOnlyExit
            ),
            "{failure}"
        );

        // ── LEG B — FLAG ON. The residual is CLOSED: a genuine automated exit
        // exists, driven by the live `adopt_before_author_param` predicate. This
        // is the half that proves the capability is real rather than declared —
        // if the arm is ever removed or its predicate stops ANDing the flag with
        // the evidence, this leg goes red.
        let report = check_liveness(STATES, classify, residual_transitions(true)).unwrap_or_else(
            |failure| {
                panic!(
                    "adopt-before-author is supposed to be the residual's automated exit; the \
                     harness cannot find one:\n{failure}"
                )
            },
        );
        assert!(
            report.progress_checked,
            "the flag-on leg must actually check progress, not merely enumerate: {report}"
        );
        assert!(
            report.transitions_enumerated >= 1,
            "the flag-on leg must enumerate the adopt-before-author transition: {report}"
        );
    }

    /// **The SPIN class has a discharge, and the discharge is flag-shaped.**
    ///
    /// The class (canonicalized 2026-08-14,
    /// `genesis/data/timeline/backlog/spin-divergent-undeclared-rows-block-a-convergence.md`):
    /// two peers each hold a genuine root for an id, NEITHER ever declared, and
    /// each conductor answers its own root every sweep — `StampOutcome::Refreshed`
    /// forever, `known_divergent{stream="content"}` pinned at 13/13/13 across the
    /// household mesh with zero `Healed` outcomes. Every pre-existing lever was
    /// structurally out of reach (ContestPeer needs `local_declared`; ghost-decay
    /// needs `Answer::Absent`; the responder's hint-inflation guard forecloses
    /// peer-head candidacy), so the DHT election received a candidate from
    /// NEITHER side: an absorbing state the main table above is scope-blind to
    /// (see the module doc's scope note).
    ///
    /// The discharge is symmetric SELF-CANDIDACY (`contest_divergent`), gated by
    /// `config.contest_undeclared_divergence` — default ON, unlike
    /// adopt-before-author, because a node with a chain nominating its own
    /// genuinely-held root is a safe supply act. So the two legs here carry the
    /// INVERSE burden of the residual witness above: leg A proves the
    /// kill-switch is a true rollback (OFF returns the class to its pre-cure
    /// manual-only spin), leg B proves the shipped default actually discharges.
    #[test]
    fn the_spin_class_discharge_is_flag_shaped() {
        #[derive(Debug, PartialEq, Eq, Clone, Copy)]
        enum Spin {
            /// Undeclared on both sides, anchors genuinely divergent, no election
            /// — the class that spun through the MissLedger forever.
            UndeclaredTwoWayRootDivergence,
            /// A canonical-head candidate exists on the DHT. From here the MAIN
            /// table owns the row: the minting side's next resolve answers
            /// canonical (AdoptLocal/HealCanonical fills it), the peer's obey
            /// arm reads the same election, and the settled-declared rows are
            /// proven Terminal above.
            ElectionSupplied,
        }

        /// Driven by the LIVE predicates, with the config bit folded exactly
        /// where the production call sites fold it: the `Refreshed`-site
        /// admission checks `contest_undeclared_divergence_enabled()` before
        /// consulting the predicate, and the pre-flight folds the same bit into
        /// `decide_head_action`'s `peer_divergent_anchor` input.
        fn spin_transitions(flag_on: bool) -> impl Fn(&Spin) -> Vec<Transition<Spin>> {
            use crate::p2p::projection_reconcile::undeclared_divergence_should_route_to_contest;
            move |s: &Spin| match s {
                Spin::ElectionSupplied => vec![],
                Spin::UndeclaredTwoWayRootDivergence => {
                    // The live admission: non-canonical answer, undeclared row,
                    // actionable divergence evidence this sweep, no election.
                    let admitted = flag_on
                        && undeclared_divergence_should_route_to_contest(false, false, true, false);
                    // The live decision, at the shipped contest configuration.
                    let decision =
                        decide_head_action(false, false, false, false, CONTEST_ENABLED, admitted);
                    if admitted && decision == HeadDecision::ContestDivergent {
                        vec![Transition::automated(
                            "contest_divergent: nominate THIS node's own genuinely-held head as \
                             a canonical-election candidate (chain exists — the conductor just \
                             answered it; target locally retrievable — it is our own action); \
                             the peer's symmetric sweep nominates its root, \
                             select_canonical_winner arbitrates, and both rows converge \
                             through the ordinary canonical heal",
                        )
                        .to(Spin::ElectionSupplied)]
                    } else {
                        vec![Transition::deploy(
                            "operator flip of CONTEST_UNDECLARED_DIVERGENCE back on, or a \
                             manual canonical stamp — with the kill-switch thrown the class \
                             returns to its pre-2026-08-14 MissLedger spin \
                             (Refreshed → dormant → re-admitted, forever), which C3 refuses \
                             to count as an exit",
                        )
                        .to(Spin::ElectionSupplied)]
                    }
                }
            }
        }

        fn classify(s: &Spin) -> StateClass {
            match s {
                Spin::ElectionSupplied => StateClass::Terminal(
                    "a candidate exists; the settled-row machinery proven in the main table \
                     owns the row from here",
                ),
                Spin::UndeclaredTwoWayRootDivergence => StateClass::Live,
            }
        }

        const STATES: &[Spin] = &[Spin::UndeclaredTwoWayRootDivergence, Spin::ElectionSupplied];

        // ── LEG A — KILL-SWITCH THROWN. The class must honestly return to its
        // absorbing pre-cure shape: if this leg ever passes, the flag stopped
        // gating the arm and the rollback story is a lie.
        let failure = check_liveness(STATES, classify, spin_transitions(false)).expect_err(
            "with CONTEST_UNDECLARED_DIVERGENCE off the SPIN class must have no automated \
             exit — if this passes, the kill-switch no longer gates the discharge",
        );
        assert!(
            failure.has_class(
                seam_contracts::harness::liveness::LivenessViolationClass::ManualOnlyExit
            ),
            "{failure}"
        );

        // ── LEG B — THE SHIPPED DEFAULT (flag ON). The discharge is real: the
        // live admission predicate + the live decision rule produce the
        // ContestDivergent transition, and the harness finds the exit.
        let report =
            check_liveness(STATES, classify, spin_transitions(true)).unwrap_or_else(|failure| {
                panic!(
                    "contest_divergent is supposed to be the SPIN class's automated exit; the \
                     harness cannot find one:\n{failure}"
                )
            });
        assert!(
            report.progress_checked,
            "the flag-on leg must actually check progress, not merely enumerate: {report}"
        );
        assert!(
            report.transitions_enumerated >= 1,
            "the flag-on leg must enumerate the contest_divergent transition: {report}"
        );
    }
}
