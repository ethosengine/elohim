//! The `Quiescent` harness — C6b (idempotent effect) as a property.
//!
//! Design: `genesis/docs/superpowers/plans/2026-08-02-seam-concern-contract-architecture-plan.md`
//! (canon row C6b; Design surface 3).
//!
//! ## The guarantee
//!
//! > Replaying a decision against settled state mints nothing and double-applies
//! > nothing.
//!
//! A decision point that is re-entered — by a sweep, a retry, a restart, a
//! second gossip delivery of the same record — must reach a **fixed point**:
//! after the first application, every further round decides something whose
//! application mints zero effects and leaves the state byte-identical.
//!
//! ## The clause that keeps this from becoming laziness
//!
//! **Quiescence is about *effect*, not effort.** It never licenses lazy
//! acceptance. Principle P1 stands unchanged: elohim-storage is a reconciliation
//! controller and reconciles **eagerly** — it re-derives, re-compares, and
//! re-decides on every tick. What it must not do is *mint* on a tick where
//! nothing changed.
//!
//! The two failure modes are opposite and both real:
//!
//! - a controller that mints on every replay produces a mint-storm
//!   (`8844cd1d6`, 2026-07-11 — boot re-projection stamping the root-author
//!   fallback over the adopted canonical, 2,838 rows *per restart*);
//! - a controller that skips the *work* to avoid the mint stops converging, and
//!   the state it declines to examine is exactly the state that needed healing
//!   — which is a C3 (liveness) hole wearing a C6b costume.
//!
//! This harness therefore checks the *effect count* and the *state*, and says
//! nothing at all about how much work `decide` did. A caller that wants to prove
//! eagerness asserts on its own instrumentation; it must not do so by weakening
//! this property.
//!
//! ## Relationship to C1
//!
//! Replay-idempotence is also the mechanical half of anti-self-election
//! (C1): the contest-quiescence arm exists so a node that already holds the
//! canonical head does not re-contest it on every sweep, re-crowning what it
//! authored once per tick. A `(id, target)` idempotence ledger is the concrete
//! instance; this harness is its abstract property.
//!
//! **Forcing incident:** `8844cd1d6` (2026-07-11, boot re-projection),
//! `b91ee0f95` (2026-07-28, `upsert_with_anchor` self-declaring on every
//! own-conductor commit), and `2175f2b60` (2026-07-28, `heal_content` — a
//! *terminal* write invisible to both guarded sweeps). Three sites, one shape.

use crate::ReasonLabel;

/// Declared budget for [`check_quiescence_with`] — C6a applied to the
/// instrument.
///
/// **Concerns:** C6a (bounded work).
///
/// **Contract test:** `rounds_below_two_cannot_prove_settling`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuiescenceBudget {
    /// How many decide→apply rounds to run. Must be ≥ 2: round 1 may legally
    /// mint, and rounds 2.. are the ones that must not.
    pub rounds: usize,
}

impl Default for QuiescenceBudget {
    /// Four rounds — one that may mint, three that must not. Three replays
    /// rather than one because the mint-storm shapes in history were periodic
    /// (per-restart, per-sweep), and a two-round check cannot see a period-2
    /// oscillation.
    fn default() -> Self {
        Self { rounds: 4 }
    }
}

/// What the harness observed.
///
/// **Concerns:** C7 (advertise/serve symmetry — the report says what actually
/// ran), C8.
///
/// **Contract test:** `report_records_first_round_effects`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuiescenceReport {
    /// How many decide→apply rounds ran.
    pub rounds: usize,
    /// Effects minted by the FIRST round. May legally be non-zero — that is the
    /// work the decision exists to do.
    pub first_round_effects: usize,
}

/// Why a quiescence property failed.
///
/// **Concerns:** C6b, C8.
///
/// **Contract test:** `violation_class_is_conformant`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum QuiescenceViolationClass {
    /// A replay against settled state minted effects — the mint-storm shape.
    ReplayMintedEffects,
    /// A replay against settled state changed the state without minting — a
    /// silent drift that no effect counter would ever show.
    ReplayDriftedState,
    /// The harness was asked for fewer than two rounds, so it cannot
    /// distinguish "settled" from "never replayed."
    RoundsBelowTwo,
}

impl ReasonLabel for QuiescenceViolationClass {
    const ALL: &'static [Self] = &[
        QuiescenceViolationClass::ReplayMintedEffects,
        QuiescenceViolationClass::ReplayDriftedState,
        QuiescenceViolationClass::RoundsBelowTwo,
    ];

    fn label(&self) -> &'static str {
        match self {
            QuiescenceViolationClass::ReplayMintedEffects => "replay_minted_effects",
            QuiescenceViolationClass::ReplayDriftedState => "replay_drifted_state",
            QuiescenceViolationClass::RoundsBelowTwo => "rounds_below_two",
        }
    }
}

/// A failed quiescence property: typed class + capsule.
///
/// **Concerns:** C6b, C8, C14.
///
/// **Contract test:** `mint_on_every_round_is_caught`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuiescenceViolation {
    /// The typed failure class.
    pub class: QuiescenceViolationClass,
    /// Human-readable reproduction context — which round, what changed.
    pub detail: String,
}

impl core::fmt::Display for QuiescenceViolation {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "[{}] {}", self.class.label(), self.detail)
    }
}

impl std::error::Error for QuiescenceViolation {}

/// Check replay-idempotence with the default budget (4 rounds).
///
/// **Concerns:** C6b, C1.
///
/// **Contract test:** `settling_decision_passes`, `mint_on_every_round_is_caught`,
/// `silent_state_drift_is_caught`.
///
/// - `decide` is the pure predicate under test: state → decision.
/// - `apply` performs the decision against a mutable copy of the state and
///   returns **the number of effects minted** (rows written, links created,
///   messages sent). Returning `0` means "nothing was minted," which is exactly
///   what a replay must report.
///
/// Both closures must be deterministic; `apply` is the only one allowed to
/// mutate, and it mutates a local clone — the harness never touches real state.
pub fn check_quiescence<S, D, F, G>(
    initial: S,
    decide: F,
    apply: G,
) -> Result<QuiescenceReport, QuiescenceViolation>
where
    S: Clone + PartialEq + core::fmt::Debug,
    D: core::fmt::Debug,
    F: Fn(&S) -> D,
    G: Fn(&mut S, &D) -> usize,
{
    check_quiescence_with(initial, decide, apply, QuiescenceBudget::default())
}

/// Check replay-idempotence against an explicit budget.
///
/// **Concerns:** C6b, C6a.
///
/// **Contract test:** `rounds_below_two_cannot_prove_settling`,
/// `period_two_oscillation_is_caught`.
pub fn check_quiescence_with<S, D, F, G>(
    initial: S,
    decide: F,
    apply: G,
    budget: QuiescenceBudget,
) -> Result<QuiescenceReport, QuiescenceViolation>
where
    S: Clone + PartialEq + core::fmt::Debug,
    D: core::fmt::Debug,
    F: Fn(&S) -> D,
    G: Fn(&mut S, &D) -> usize,
{
    if budget.rounds < 2 {
        return Err(QuiescenceViolation {
            class: QuiescenceViolationClass::RoundsBelowTwo,
            detail: format!(
                "asked for {} round(s); quiescence is a statement about REPLAY, so it needs at \
                 least one round that may mint and one that must not",
                budget.rounds
            ),
        });
    }

    let mut state = initial;

    // Round 1 — may legally mint. This is the work.
    let decision = decide(&state);
    let first_round_effects = apply(&mut state, &decision);
    let settled = state.clone();

    // Rounds 2.. — eager re-decision (P1 stays eager), zero effect.
    for round in 2..=budget.rounds {
        let decision = decide(&state);
        let effects = apply(&mut state, &decision);

        if effects != 0 {
            return Err(QuiescenceViolation {
                class: QuiescenceViolationClass::ReplayMintedEffects,
                detail: format!(
                    "round {round} minted {effects} effect(s) against settled state {settled:?} \
                     (decision {decision:?}). Re-entry mints on every sweep/restart — the \
                     mint-storm shape. The cure is an idempotence key on the effect \
                     (e.g. an (id, target) ledger), NOT skipping the re-decision: \
                     reconciliation stays eager, only the mint is guarded."
                ),
            });
        }

        if state != settled {
            return Err(QuiescenceViolation {
                class: QuiescenceViolationClass::ReplayDriftedState,
                detail: format!(
                    "round {round} reported 0 effects but moved the state from {settled:?} to \
                     {state:?} (decision {decision:?}). A mutation no effect counter sees is \
                     invisible to every dashboard built on that counter (C8) — the state moved \
                     and nothing anywhere would say so."
                ),
            });
        }
    }

    Ok(QuiescenceReport {
        rounds: budget.rounds,
        first_round_effects,
    })
}

/// Panicking form of [`check_quiescence`], for use inside `#[test]`.
///
/// **Concerns:** C6b.
///
/// **Contract test:** `assert_form_panics_on_violation`.
///
/// # Panics
///
/// If a replay mints effects or drifts the state.
pub fn assert_quiescent<S, D, F, G>(initial: S, decide: F, apply: G)
where
    S: Clone + PartialEq + core::fmt::Debug,
    D: core::fmt::Debug,
    F: Fn(&S) -> D,
    G: Fn(&mut S, &D) -> usize,
{
    if let Err(v) = check_quiescence(initial, decide, apply) {
        panic!("quiescence contract (C6b) failed: {v}");
    }
}

#[cfg(test)]
mod quiescent_tests {
    use super::*;

    /// A projection row keyed by id, with a declared head. The decision is
    /// "adopt the incoming head if it is newer"; the effect is the write.
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct Projection {
        head: u64,
        writes: u64,
    }

    #[derive(Debug)]
    enum Decision {
        Adopt(u64),
        Hold,
    }

    const INCOMING: u64 = 7;

    fn decide_adopt(state: &Projection) -> Decision {
        if INCOMING > state.head {
            Decision::Adopt(INCOMING)
        } else {
            Decision::Hold
        }
    }

    fn apply_guarded(state: &mut Projection, decision: &Decision) -> usize {
        match decision {
            Decision::Adopt(h) => {
                state.head = *h;
                state.writes += 1;
                1
            }
            Decision::Hold => 0,
        }
    }

    #[test]
    fn settling_decision_passes() {
        let report = check_quiescence(
            Projection { head: 3, writes: 0 },
            decide_adopt,
            apply_guarded,
        )
        .expect("adopt-if-newer settles after one round");
        assert_eq!(report.first_round_effects, 1);
        assert_eq!(report.rounds, 4);
    }

    #[test]
    fn already_settled_state_mints_nothing_at_all() {
        let report = check_quiescence(
            Projection { head: 9, writes: 0 },
            decide_adopt,
            apply_guarded,
        )
        .expect("a head newer than the incoming one holds");
        assert_eq!(
            report.first_round_effects, 0,
            "eager re-decision, zero mint — the P1 shape"
        );
    }

    #[test]
    fn mint_on_every_round_is_caught() {
        // The `8844cd1d6` shape: re-projection stamps unconditionally.
        let err = check_quiescence(
            Projection { head: 3, writes: 0 },
            |_state: &Projection| Decision::Adopt(INCOMING),
            apply_guarded,
        )
        .expect_err("an unconditional stamp mints on every replay");
        assert_eq!(err.class, QuiescenceViolationClass::ReplayMintedEffects);
        assert!(err.to_string().contains("replay_minted_effects"));
        assert!(
            err.to_string().contains("reconciliation stays eager"),
            "the violation must carry the anti-laziness clause, or the obvious 'fix' is to \
             stop reconciling"
        );
    }

    #[test]
    fn silent_state_drift_is_caught() {
        // Reports 0 effects but mutates anyway — invisible to every counter.
        let err = check_quiescence(
            Projection { head: 3, writes: 0 },
            decide_adopt,
            |state: &mut Projection, decision: &Decision| match decision {
                Decision::Adopt(h) => {
                    state.head = *h;
                    state.writes += 1;
                    1
                }
                Decision::Hold => {
                    state.writes += 1; // drift with no reported effect
                    0
                }
            },
        )
        .expect_err("a mutation with no reported effect is a silent drift");
        assert_eq!(err.class, QuiescenceViolationClass::ReplayDriftedState);
    }

    #[test]
    fn period_two_oscillation_is_caught() {
        // Flips between two heads; a 2-round check would call this settled.
        let err = check_quiescence_with(
            Projection { head: 0, writes: 0 },
            |state: &Projection| {
                if state.head == 0 {
                    Decision::Adopt(1)
                } else {
                    Decision::Adopt(0)
                }
            },
            apply_guarded,
            QuiescenceBudget { rounds: 4 },
        )
        .expect_err("an oscillation never settles");
        assert_eq!(err.class, QuiescenceViolationClass::ReplayMintedEffects);
    }

    #[test]
    fn rounds_below_two_cannot_prove_settling() {
        let err = check_quiescence_with(
            Projection { head: 3, writes: 0 },
            decide_adopt,
            apply_guarded,
            QuiescenceBudget { rounds: 1 },
        )
        .expect_err("one round proves nothing about replay");
        assert_eq!(err.class, QuiescenceViolationClass::RoundsBelowTwo);
    }

    #[test]
    fn report_records_first_round_effects() {
        let report = check_quiescence_with(
            Projection { head: 0, writes: 0 },
            decide_adopt,
            apply_guarded,
            QuiescenceBudget { rounds: 9 },
        )
        .expect("settles");
        assert_eq!(report.rounds, 9);
        assert_eq!(report.first_round_effects, 1);
    }

    #[test]
    #[should_panic(expected = "quiescence contract (C6b) failed")]
    fn assert_form_panics_on_violation() {
        assert_quiescent(
            Projection { head: 3, writes: 0 },
            |_state: &Projection| Decision::Adopt(INCOMING),
            apply_guarded,
        );
    }

    #[test]
    fn violation_class_is_conformant() {
        crate::assert_reason_labels_conformant::<QuiescenceViolationClass>();
        crate::assert_reason_labels_discriminating::<QuiescenceViolationClass>();
        crate::assert_reason_labels_stable::<QuiescenceViolationClass>(&[
            "replay_minted_effects",
            "replay_drifted_state",
            "rounds_below_two",
        ]);
    }
}
