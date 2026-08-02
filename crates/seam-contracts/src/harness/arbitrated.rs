//! The `Arbitrated` harness — C2 (monotonic authority) as a property.
//!
//! Design: `genesis/docs/superpowers/plans/2026-08-02-seam-concern-contract-architecture-plan.md`
//! (canon row C2; Design surface 3).
//!
//! ## The guarantee
//!
//! > Selection is decided by declared authority tiers and, within a tier, a
//! > **named notarized clock** with a deterministic tiebreak — never a
//! > conductor-local clock, never arrival order.
//!
//! The testable residue of that guarantee is: **the winner is a function of the
//! candidate *set*, not of the order the candidates arrived in.** Two peers
//! holding the same candidates must elect the same winner, and neither peer
//! controls the order its transport delivered them in.
//!
//! ## What this harness checks
//!
//! 1. **Permutation-invariance** — the selector returns the same winner for
//!    every ordering of the same candidate set. A selector that sorts on a
//!    non-total key (timestamp with no tiebreak) passes on most orderings and
//!    fails on the ones where a tie exists, which is precisely the shape that
//!    survives review and then diverges in production.
//! 2. **Call determinism** — repeated calls on the *identical* input return the
//!    identical winner. Catches a selector that reads a clock, a `HashMap`
//!    iteration order, or any ambient state.
//!
//! ## The abstract form of a live instance
//!
//! `select_canonical_winner` in the elohim DNA's `content_store` coordinator
//! zome is the concrete instance: tier precedence first (any *earned*
//! declaration beats every *staging* one), then newest-within-tier on the DHT
//! link timestamp — *"the one clock that can order two declarations"* — then a
//! deterministic tiebreak on the create-link action hash.
//!
//! **Forcing incident:** `25921a07a` (2026-07-10) — the zome selector ordered by
//! `get_links` arrival until the `(timestamp, create_link_hash)` tiebreak
//! landed; a stable timestamp-only sort resolves ties by per-peer arrival order
//! and therefore diverges permanently. Independently, `a83b35079` (2026-07-29)
//! in the epr-cli/REA sidecar: "latest" by append order rather than by
//! `occurredAt` — the commit body reads *"the exact `8a1c531dc` bug,
//! reintroduced via replay."* An uncanonized concern re-minting itself in a
//! different substrate family is the whole argument for this harness existing.
//! And museum entry #12: first-that-matches degenerating to
//! index-0-always-wins.
//!
//! ## What this harness does NOT check
//!
//! It does not check that the selector picked the *right* winner — that is the
//! predicate's own unit tests. It checks that whatever it picks, it picks
//! consistently. A selector that always returns the first candidate is
//! permutation-*dependent* and fails; a selector that always returns `None`
//! passes, which is why a harness result is never a substitute for a behavioural
//! test.
//!
//! It also does not check monotonicity across *time* (the "never backwards"
//! half of C2) — that is a stamp-guard property (`provably_newer`) over a
//! sequence of states, not over a candidate set.

use crate::ReasonLabel;

/// Declared budget for [`check_arbitration_with`] — C6a applied to the
/// instrument.
///
/// **Concerns:** C6a (bounded work), C7 (the report says which regime ran).
///
/// **Forcing incident:** `6369721fd` (2026-07-04) — a view-federation responder
/// rejected its own oversized reply; unbounded work inside a checker is the same
/// failure with a friendlier face. `n!` grows past any wall-clock budget at
/// n = 11, so an unbounded permutation walk is a hang, not a test.
///
/// **Contract test:** `budget_caps_permutation_walk`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArbitrationBudget {
    /// Maximum number of orderings to try. When `n!` exceeds this, the harness
    /// switches from exhaustive enumeration to a deterministic pseudo-random
    /// sample and says so in the [`ArbitrationReport`].
    pub max_permutations: usize,
    /// How many times to re-invoke the selector on the identical input to catch
    /// ambient-state dependence.
    pub repeat_calls: usize,
    /// Seed for the sampling PRNG. Fixed by default: a harness that samples
    /// differently on every run turns a real defect into a flake and trains
    /// people to re-run rather than to look.
    pub sample_seed: u64,
}

impl Default for ArbitrationBudget {
    /// 720 orderings (exhaustive through n = 6), 8 repeat calls, fixed seed.
    fn default() -> Self {
        Self {
            max_permutations: 720,
            repeat_calls: 8,
            sample_seed: 0x5EA1_C047_1AC7,
        }
    }
}

/// What the harness actually did — the honest report (C7).
///
/// **Concerns:** C7 (advertise/serve symmetry), C8 (`exhaustive` is the label
/// that keeps a sampled run from being read as a proof).
///
/// **Contract test:** `report_marks_sampled_runs_as_not_exhaustive`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArbitrationReport {
    /// How many orderings were tried.
    pub permutations_checked: usize,
    /// `true` iff *every* ordering of the candidate set was tried. When
    /// `false`, a pass is evidence, not proof.
    pub exhaustive: bool,
    /// How many identical-input calls were compared.
    pub repeat_calls: usize,
}

/// Why an arbitration property failed.
///
/// **Concerns:** C2, C8 (the harness's own outcomes are reason-labeled).
///
/// **Contract test:** `violation_class_is_conformant`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ArbitrationViolationClass {
    /// The winner changed when the candidates were reordered — two peers with
    /// the same set will elect different winners and diverge permanently.
    PermutationDependent,
    /// The selector returned different winners for byte-identical input — it is
    /// reading something that is not in its arguments (a clock, a hash-map
    /// order, a global).
    Nondeterministic,
}

impl ReasonLabel for ArbitrationViolationClass {
    const ALL: &'static [Self] = &[
        ArbitrationViolationClass::PermutationDependent,
        ArbitrationViolationClass::Nondeterministic,
    ];

    fn label(&self) -> &'static str {
        match self {
            ArbitrationViolationClass::PermutationDependent => "permutation_dependent",
            ArbitrationViolationClass::Nondeterministic => "nondeterministic",
        }
    }
}

/// A failed arbitration property: typed class + capsule.
///
/// **Concerns:** C2, C8, C14 (the `detail` capsule carries the reproducing
/// ordering — enough context to act on, without becoming a metric dimension).
///
/// **Contract test:** `permutation_dependent_selector_is_caught`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArbitrationViolation {
    /// The typed failure class.
    pub class: ArbitrationViolationClass,
    /// Human-readable reproduction context.
    pub detail: String,
}

impl core::fmt::Display for ArbitrationViolation {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "[{}] {}", self.class.label(), self.detail)
    }
}

impl std::error::Error for ArbitrationViolation {}

/// Check permutation-invariance and call determinism with the default budget.
///
/// **Concerns:** C2.
///
/// **Contract test:** `deterministic_selector_passes`,
/// `permutation_dependent_selector_is_caught`,
/// `nondeterministic_selector_is_caught`.
///
/// `select` receives the candidates in some order and returns the winner (or
/// `None` for "no admissible candidate"). It must be pure.
pub fn check_arbitration<C, S>(
    candidates: &[C],
    select: S,
) -> Result<ArbitrationReport, ArbitrationViolation>
where
    C: Clone + PartialEq + core::fmt::Debug,
    S: Fn(&[C]) -> Option<C>,
{
    check_arbitration_with(candidates, select, ArbitrationBudget::default())
}

/// Check permutation-invariance and call determinism against an explicit
/// budget.
///
/// **Concerns:** C2, C6a.
///
/// **Contract test:** `budget_caps_permutation_walk`,
/// `report_marks_sampled_runs_as_not_exhaustive`.
///
/// The reference ordering is the caller's own; every other ordering is compared
/// against it, so the reported violation names an ordering you can paste back
/// into a unit test.
pub fn check_arbitration_with<C, S>(
    candidates: &[C],
    select: S,
    budget: ArbitrationBudget,
) -> Result<ArbitrationReport, ArbitrationViolation>
where
    C: Clone + PartialEq + core::fmt::Debug,
    S: Fn(&[C]) -> Option<C>,
{
    let reference = select(candidates);

    // 1. Call determinism on the identical input.
    for call in 1..budget.repeat_calls {
        let again = select(candidates);
        if again != reference {
            return Err(ArbitrationViolation {
                class: ArbitrationViolationClass::Nondeterministic,
                detail: format!(
                    "call #{call} on byte-identical input returned {again:?}, first call returned \
                     {reference:?} — the selector reads state that is not in its arguments \
                     (a clock, a HashMap iteration order, a global). Two peers cannot converge \
                     on a winner that depends on ambient state."
                ),
            });
        }
    }

    // 2. Permutation-invariance.
    let n = candidates.len();
    let exhaustive = factorial_fits(n, budget.max_permutations);
    let mut checked = 0usize;
    let mut violation = None;

    if exhaustive {
        for_each_permutation(candidates, budget.max_permutations, |ordering| {
            checked += 1;
            let winner = select(ordering);
            if winner == reference {
                true
            } else {
                violation = Some(permutation_violation(
                    &reference, &winner, ordering, candidates,
                ));
                false
            }
        });
    } else {
        let mut state = budget.sample_seed | 1;
        let mut scratch = candidates.to_vec();
        // The caller's own ordering counts as one checked ordering.
        checked = 1;
        while checked < budget.max_permutations {
            shuffle(&mut scratch, &mut state);
            checked += 1;
            let winner = select(&scratch);
            if winner != reference {
                violation = Some(permutation_violation(
                    &reference, &winner, &scratch, candidates,
                ));
                break;
            }
        }
    }

    match violation {
        Some(v) => Err(v),
        None => Ok(ArbitrationReport {
            permutations_checked: checked,
            exhaustive,
            repeat_calls: budget.repeat_calls.max(1),
        }),
    }
}

/// Panicking form of [`check_arbitration`], for use inside `#[test]`.
///
/// **Concerns:** C2.
///
/// **Contract test:** `assert_form_panics_on_violation`.
///
/// # Panics
///
/// If the selector is permutation-dependent or nondeterministic.
pub fn assert_arbitration<C, S>(candidates: &[C], select: S)
where
    C: Clone + PartialEq + core::fmt::Debug,
    S: Fn(&[C]) -> Option<C>,
{
    if let Err(v) = check_arbitration(candidates, select) {
        panic!("arbitration contract (C2) failed: {v}");
    }
}

fn permutation_violation<C: core::fmt::Debug>(
    reference: &Option<C>,
    winner: &Option<C>,
    ordering: &[C],
    original: &[C],
) -> ArbitrationViolation {
    ArbitrationViolation {
        class: ArbitrationViolationClass::PermutationDependent,
        detail: format!(
            "ordering {ordering:?} elected {winner:?}, but ordering {original:?} elected \
             {reference:?}. The winner is a function of arrival order, not of the candidate set — \
             two peers holding the same candidates will diverge permanently. The cure is a TOTAL \
             order: the tier, then the named notarized clock, then a deterministic tiebreak on a \
             hash (never a stable sort on the clock alone)."
        ),
    }
}

/// `true` iff `n!` is computable and ≤ `limit`.
fn factorial_fits(n: usize, limit: usize) -> bool {
    let mut acc: u128 = 1;
    for k in 2..=(n as u128) {
        acc = acc.saturating_mul(k);
        if acc > limit as u128 {
            return false;
        }
    }
    acc <= limit as u128
}

/// Heap's algorithm, iterative, budget-capped. `visit` returns `false` to stop.
/// Returns the number of orderings visited.
fn for_each_permutation<T: Clone>(
    items: &[T],
    limit: usize,
    mut visit: impl FnMut(&[T]) -> bool,
) -> usize {
    if limit == 0 {
        return 0;
    }
    let mut a = items.to_vec();
    let n = a.len();
    if !visit(&a) {
        return 1;
    }
    let mut visited = 1usize;
    let mut c = vec![0usize; n];
    let mut i = 1usize;
    while i < n && visited < limit {
        if c[i] < i {
            if i % 2 == 0 {
                a.swap(0, i);
            } else {
                a.swap(c[i], i);
            }
            visited += 1;
            if !visit(&a) {
                return visited;
            }
            c[i] += 1;
            i = 1;
        } else {
            c[i] = 0;
            i += 1;
        }
    }
    visited
}

/// xorshift64 — a fixed, dependency-free PRNG. Deterministic by design: see
/// [`ArbitrationBudget::sample_seed`].
fn next_u64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

fn shuffle<T>(items: &mut [T], state: &mut u64) {
    if items.len() < 2 {
        return;
    }
    for i in (1..items.len()).rev() {
        let j = (next_u64(state) % (i as u64 + 1)) as usize;
        items.swap(i, j);
    }
}

#[cfg(test)]
mod arbitrated_tests {
    use super::*;

    /// The shape of a canonical-head candidate: (tier, clock, tiebreak-hash).
    /// `tier` 1 beats `tier` 0 outright (earned over staging); within a tier the
    /// newest clock wins; equal clocks break on the hash.
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct Candidate {
        tier: u8,
        clock: u64,
        hash: u8,
    }

    fn c(tier: u8, clock: u64, hash: u8) -> Candidate {
        Candidate { tier, clock, hash }
    }

    /// Total order — the cured `select_canonical_winner` shape.
    fn select_total(candidates: &[Candidate]) -> Option<Candidate> {
        candidates
            .iter()
            .max_by(|a, b| {
                a.tier
                    .cmp(&b.tier)
                    .then(a.clock.cmp(&b.clock))
                    .then(a.hash.cmp(&b.hash))
            })
            .cloned()
    }

    /// The regression fixture: newest-within-tier with NO tiebreak. A stable
    /// `max_by` on a partial key returns whichever tied element arrived last —
    /// i.e. arrival order decides. This is the pre-`25921a07a` shape.
    fn select_no_tiebreak(candidates: &[Candidate]) -> Option<Candidate> {
        candidates
            .iter()
            .max_by(|a, b| a.tier.cmp(&b.tier).then(a.clock.cmp(&b.clock)))
            .cloned()
    }

    /// Museum #12: first-that-matches degenerating to index-0-always-wins.
    fn select_first(candidates: &[Candidate]) -> Option<Candidate> {
        candidates.first().cloned()
    }

    fn tied_set() -> Vec<Candidate> {
        vec![c(0, 10, 1), c(1, 5, 2), c(1, 5, 3), c(0, 99, 4)]
    }

    #[test]
    fn deterministic_selector_passes() {
        let report =
            check_arbitration(&tied_set(), select_total).expect("total order is invariant");
        assert!(
            report.exhaustive,
            "4! = 24 ≤ 720, so the walk is exhaustive"
        );
        assert_eq!(report.permutations_checked, 24);
    }

    #[test]
    fn permutation_dependent_selector_is_caught() {
        // Ties on (tier=1, clock=5) — hashes 2 and 3 — so arrival order decides.
        let err = check_arbitration(&tied_set(), select_no_tiebreak)
            .expect_err("a clock-only order over a tied set is arrival-order dependent");
        assert_eq!(err.class, ArbitrationViolationClass::PermutationDependent);
        assert!(err.to_string().contains("permutation_dependent"));
    }

    #[test]
    fn index_zero_always_wins_is_caught() {
        let err = check_arbitration(&tied_set(), select_first)
            .expect_err("first-that-matches is arrival-order dependent");
        assert_eq!(err.class, ArbitrationViolationClass::PermutationDependent);
    }

    #[test]
    fn nondeterministic_selector_is_caught() {
        use std::cell::Cell;
        let calls = Cell::new(0u32);
        let err = check_arbitration(&tied_set(), |cands: &[Candidate]| {
            calls.set(calls.get() + 1);
            // Ambient state decides — the shape of a selector reading a clock.
            if calls.get() % 2 == 0 {
                cands.first().cloned()
            } else {
                cands.last().cloned()
            }
        })
        .expect_err("a selector reading a counter is not a function of its arguments");
        assert_eq!(err.class, ArbitrationViolationClass::Nondeterministic);
    }

    #[test]
    fn empty_and_singleton_sets_are_trivially_invariant() {
        let empty: Vec<Candidate> = vec![];
        let report = check_arbitration(&empty, select_total).expect("empty set");
        assert!(report.exhaustive);
        let one = vec![c(0, 1, 1)];
        let report = check_arbitration(&one, select_total).expect("singleton");
        assert!(report.exhaustive);
    }

    #[test]
    fn budget_caps_permutation_walk() {
        let budget = ArbitrationBudget {
            max_permutations: 10,
            ..ArbitrationBudget::default()
        };
        let report = check_arbitration_with(&tied_set(), select_total, budget)
            .expect("total order is invariant");
        assert!(
            !report.exhaustive,
            "4! = 24 > 10, so the harness must report a SAMPLED run"
        );
        assert_eq!(report.permutations_checked, 10);
    }

    #[test]
    fn report_marks_sampled_runs_as_not_exhaustive() {
        // 8 candidates → 40320 orderings, far past the default 720 budget.
        let many: Vec<Candidate> = (0..8u8).map(|i| c(0, u64::from(i), i)).collect();
        let report = check_arbitration(&many, select_total).expect("total order is invariant");
        assert!(!report.exhaustive);
        assert_eq!(report.permutations_checked, 720);
    }

    #[test]
    fn sampling_is_reproducible() {
        // 8 candidates (8! = 40320 ⇒ sampled) with a genuine tie at the top:
        // two tier-1 candidates share clock 9, so only the hash tiebreak
        // separates them.
        let mut many: Vec<Candidate> = (0..6u8).map(|i| c(0, u64::from(i), i)).collect();
        many.push(c(1, 9, 21));
        many.push(c(1, 9, 22));

        let a = check_arbitration(&many, select_no_tiebreak);
        let b = check_arbitration(&many, select_no_tiebreak);
        assert!(
            a.is_err(),
            "the sampled walk must still find the tie-driven divergence"
        );
        assert_eq!(
            a, b,
            "a fixed seed must make the sampled walk reproducible — a harness that samples \
             differently every run turns a defect into a flake, and trains people to re-run \
             instead of to look"
        );
    }

    #[test]
    #[should_panic(expected = "arbitration contract (C2) failed")]
    fn assert_form_panics_on_violation() {
        assert_arbitration(&tied_set(), select_first);
    }

    #[test]
    fn violation_class_is_conformant() {
        crate::assert_reason_labels_conformant::<ArbitrationViolationClass>();
        crate::assert_reason_labels_discriminating::<ArbitrationViolationClass>();
        crate::assert_reason_labels_stable::<ArbitrationViolationClass>(&[
            "permutation_dependent",
            "nondeterministic",
        ]);
    }

    #[test]
    fn factorial_fits_saturates_instead_of_overflowing() {
        assert!(factorial_fits(0, 1));
        assert!(factorial_fits(6, 720));
        assert!(!factorial_fits(7, 720));
        assert!(!factorial_fits(50, usize::MAX));
    }

    #[test]
    fn permutation_walk_is_exhaustive_and_unique() {
        let items = [1u8, 2, 3];
        let mut seen: Vec<Vec<u8>> = Vec::new();
        let visited = for_each_permutation(&items, 100, |ordering| {
            seen.push(ordering.to_vec());
            true
        });
        assert_eq!(visited, 6);
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(
            seen.len(),
            6,
            "Heap's algorithm must not repeat an ordering"
        );
    }
}
