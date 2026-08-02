//! `select_arbitrated_winner` — C2 (monotonic authority) as a plain-data predicate.
//!
//! Design: `genesis/docs/superpowers/plans/2026-08-02-seam-concern-contract-architecture-plan.md`
//! (canon row C2; Design surface 3; task P5b — the P3.3 pull-forward).
//!
//! ## The guarantee
//!
//! > Selection is decided by declared authority tiers and, within a tier, a
//! > **named notarized clock** with a deterministic tiebreak — never a
//! > conductor-local clock, never arrival order.
//!
//! [`harness::arbitrated`](crate::harness::arbitrated) checks that a selector
//! *has* that property. This module is the selector itself: the ordering every
//! tiered election in the fleet should be built out of, so the property is
//! satisfied by construction rather than re-derived (and re-broken) at each new
//! seam.
//!
//! ## Why the ordering is a type, not a comparator
//!
//! [`ArbitrationKey`] has three fields and **all three are required**. That is
//! the whole design. The failure this module exists to prevent is not "someone
//! wrote a bad comparator" — it is "someone wrote a *reasonable* comparator that
//! stops one field short." A stable sort on `(tier, clock)` is correct on every
//! input where no two candidates share a clock, passes review, passes its unit
//! tests, and then resolves ties by whatever order the transport happened to
//! deliver in — permanently, and differently on each peer. A caller cannot
//! express that shape here: there is no key type without a `tiebreak`.
//!
//! ## Concerns
//!
//! - **C2** ([`PIN_C2_MONOTONIC_AUTHORITY_V2`](crate::canon::PIN_C2_MONOTONIC_AUTHORITY_V2))
//!   — answered structurally, and proven on this module's own predicate by
//!   `real_predicate_is_arbitration_conformant` (the [`Arbitrated`
//!   harness](crate::harness::arbitrated) run against its motivating instance).
//! - **C1** ([`PIN_C1_ANTI_SELF_ELECTION`](crate::canon::PIN_C1_ANTI_SELF_ELECTION))
//!   — *partial*. The predicate cannot consult who authored a candidate: its
//!   only inputs are the candidate set and a pure key function. But the key
//!   function is the caller's, and a caller could rank its own authorship
//!   highest. The structural guarantee here is "this predicate adds no
//!   self-election channel," not "self-election is impossible."
//!
//! ## Forcing incidents
//!
//! - `25921a07a` (2026-07-10) — *"tier-aware, partition-deterministic
//!   canonical-head selection."* The elohim DNA's `content_store` coordinator
//!   zome resolved the canonical head by `get_links` arrival order until the
//!   `(timestamp, create_link_hash)` tiebreak landed, and resolved
//!   **tier-blind**: a gossip-lagged peer that wrote a newer *staging* link
//!   displaced an *earned* head. Both halves are in [`ArbitrationKey`]'s field
//!   order.
//! - `a83b35079` (2026-07-29) — the epr-cli/REA sidecar took "latest" by append
//!   order rather than by `occurredAt`; the commit body reads *"the exact
//!   `8a1c531dc` bug, reintroduced via replay."* A different substrate family,
//!   the same uncanonized concern re-minting itself — which is the argument for
//!   the ordering living in one leaf crate instead of once per seam.
//! - Museum entry #12 — first-that-matches degenerating to
//!   index-0-always-wins.
//!
//! ## The live instance
//!
//! `content_store::select_canonical_winner` in the elohim DNA's `content_store`
//! **coordinator** zome is an HDK adapter over this predicate: it maps a
//! `Link`'s provenance tag to `tier`, its DHT link timestamp to `clock`, and its
//! `create_link_hash` to `tiebreak`. The adapter holds the HDK types; nothing
//! `hdi`/`hdk`-shaped crosses into this crate (see
//! `boundary_tests::no_heavy_deps_in_dep_tree`), which is what keeps the WASM
//! path open and lets the same ordering be linked from a service, a sidecar, or
//! an external peer runtime.

/// The total order a tiered election arbitrates on: **tier, then clock, then
/// tiebreak**, in that precedence — greater wins.
///
/// **Concerns:** C2 (the three components ARE the C2 guarantee, made
/// non-optional by the type); C1 (*partial* — carries no author/identity
/// component, but the caller builds the key).
///
/// **Forcing incident:** `25921a07a` (2026-07-10) — see the module doc. The
/// pre-cure selector had `clock` and nothing else; the tier-blindness and the
/// arrival-order tiebreak were one defect wearing two costumes.
///
/// **Contract test:** `tier_precedence_dominates_a_newer_clock`,
/// `equal_timestamps_break_deterministically_on_link_hash`.
///
/// # Field order is the precedence
///
/// The derived [`Ord`] is lexicographic in declaration order, so moving a field
/// changes the election. Keep them in this order:
///
/// 1. `tier` — declared authority. Any higher tier beats every lower one
///    **regardless of clock**. A two-tier election (the common case: earned vs
///    staging) maps naturally onto `bool`, where `false` is the lower tier;
///    a single-tier election passes `()`.
/// 2. `clock` — the **named notarized clock**: a value every peer reads
///    identically off the same evidence (a DHT link timestamp, an `occurredAt`
///    on a signed record). Never a conductor-local clock — three different
///    clocks sharing one struct field is how the live instance's `declared_at`
///    became unable to order two declarations.
/// 3. `tiebreak` — a deterministic discriminator for equal clocks, normally
///    content-addressed bytes (an action hash, a CID). Its *value* is
///    arbitrary; that every peer computes the *same* value is the point.
///
/// # Greater wins
///
/// Ordering is "bigger is better" throughout: the highest tier, then the newest
/// clock, then the greatest tiebreak. To invert a component (oldest-wins), wrap
/// it in [`core::cmp::Reverse`] — which keeps the inversion visible at the key
/// site instead of hidden in a comparator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ArbitrationKey<Tier, Clock, Tiebreak> {
    /// Declared authority tier. Dominates every other component: a higher tier
    /// wins regardless of clock or tiebreak.
    pub tier: Tier,
    /// The named notarized clock — newest wins **within** a tier.
    pub clock: Clock,
    /// Deterministic discriminator for equal `(tier, clock)`. Required by
    /// construction: without it, ties resolve by arrival order.
    pub tiebreak: Tiebreak,
}

/// Select the single winning candidate from a set, on the [`ArbitrationKey`]
/// total order.
///
/// **Concerns:** C2 (answered — the winner is a function of the candidate *set*,
/// never of the order it arrived in); C1 (*partial* — see the module doc).
///
/// **Forcing incident:** `25921a07a` (2026-07-10), `a83b35079` (2026-07-29),
/// museum entry #12 — module doc.
///
/// **Contract test:** `real_predicate_is_arbitration_conformant` (the
/// [`Arbitrated` harness](crate::harness::arbitrated) on this function), plus
/// the ported ordering suite in `election_tests`.
///
/// Returns `None` for an empty set — *"no admissible candidate,"* which a caller
/// reads as "no election stands here yet," never as an authoritative absence
/// (that distinction is C4's, and it belongs in the caller's [`Answer`]
/// mapping, not here).
///
/// # Determinism, exactly
///
/// The sort is stable and the maximum is taken from the end, so two candidates
/// whose keys compare **fully equal** resolve by input order. That residue is
/// deliberate and safe *iff* `tiebreak` is a genuine discriminator — two
/// distinct declarations cannot share a content-addressed hash. A `tiebreak`
/// that is constant (or that collides) reintroduces arrival-order dependence,
/// and the [`Arbitrated` harness](crate::harness::arbitrated) is what catches
/// that: run it over a fixture containing your worst-case tie.
///
/// # Example
///
/// ```
/// use seam_contracts::election::{select_arbitrated_winner, ArbitrationKey};
///
/// #[derive(Clone, Debug, PartialEq)]
/// struct Declaration {
///     earned: bool,
///     link_clock: i64,
///     link_hash: [u8; 4],
///     target: &'static str,
/// }
///
/// let winner = select_arbitrated_winner(
///     vec![
///         Declaration { earned: true,  link_clock: 100, link_hash: [1, 1, 1, 1], target: "earned-older" },
///         Declaration { earned: false, link_clock: 900, link_hash: [9, 9, 9, 9], target: "staging-newer" },
///     ],
///     |d| ArbitrationKey { tier: d.earned, clock: d.link_clock, tiebreak: d.link_hash },
/// )
/// .expect("a non-empty set always elects");
///
/// // Tier dominates recency: the newer STAGING declaration loses outright.
/// assert_eq!(winner.target, "earned-older");
/// ```
///
/// # Wiring it to the harness
///
/// [`check_arbitration`](crate::harness::arbitrated::check_arbitration) wants
/// `Fn(&[C]) -> Option<C>`; this predicate consumes its `Vec`. Bridge them with
/// a clone at the call site — the harness is a `#[cfg(test)]` instrument, so the
/// copy costs nothing in a runtime build:
///
/// ```rust,ignore
/// seam_contracts::harness::assert_arbitration(&candidates, |set: &[Declaration]| {
///     select_arbitrated_winner(set.to_vec(), key_of)
/// });
/// ```
///
/// [`Answer`]: crate::Answer
pub fn select_arbitrated_winner<C, Tier, Clock, Tiebreak, F>(
    candidates: Vec<C>,
    key: F,
) -> Option<C>
where
    F: Fn(&C) -> ArbitrationKey<Tier, Clock, Tiebreak>,
    Tier: Ord,
    Clock: Ord,
    Tiebreak: Ord,
{
    // Key ONCE per candidate, not once per comparison: `key` is caller-supplied
    // and may clone a hash out of the candidate, and an O(n log n) comparison
    // count would pay that clone a surprising number of times.
    let mut keyed: Vec<(ArbitrationKey<Tier, Clock, Tiebreak>, C)> =
        candidates.into_iter().map(|c| (key(&c), c)).collect();
    keyed.sort_by(|a, b| a.0.cmp(&b.0));
    keyed.pop().map(|(_, candidate)| candidate)
}

#[cfg(test)]
mod election_tests {
    use super::*;

    /// A plain-data mirror of the live instance's `CanonicalCandidate` — the
    /// elohim DNA `content_store` coordinator zome's canonical-head declaration
    /// reduced to the fields that decide which one WINS.
    ///
    /// `link_hash` is `[u8; 36]` rather than a `u8` seed on purpose: the zome's
    /// own fixture builds `ActionHash::from_raw_36(vec![seed; 36])`, and a
    /// `HoloHash`'s `Ord` is lexicographic over its bytes — so with identical
    /// type prefixes the ordering reduces exactly to this array. The port
    /// mirrors the SHAPE the tiebreak compares, not a convenient scalar.
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct Candidate {
        is_earned: bool,
        timestamp: i64,
        link_hash: [u8; 36],
        target: [u8; 36],
    }

    /// Distinct, deterministically-ordered hashes from a single seed byte —
    /// the port of the zome fixture's `ah(seed)`.
    fn ah(seed: u8) -> [u8; 36] {
        [seed; 36]
    }

    fn cand(is_earned: bool, ts: i64, link_seed: u8, target_seed: u8) -> Candidate {
        Candidate {
            is_earned,
            timestamp: ts,
            link_hash: ah(link_seed),
            target: ah(target_seed),
        }
    }

    /// The key the live adapter builds. Tier is `bool`: `false` (staging) <
    /// `true` (earned).
    fn key_of(c: &Candidate) -> ArbitrationKey<bool, i64, [u8; 36]> {
        ArbitrationKey {
            tier: c.is_earned,
            clock: c.timestamp,
            tiebreak: c.link_hash,
        }
    }

    fn select(candidates: Vec<Candidate>) -> Option<Candidate> {
        select_arbitrated_winner(candidates, key_of)
    }

    // ───────────────────────────────────────────────────────────────────
    // PORTED ORDERING SUITE. Every test below is a field-for-field port of
    // `canonical_head_selector_tests` in
    // elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs — same name,
    // same fixture seeds, same assertions — so the two suites are readable as
    // one behavioural claim. The zome's copies stay green THROUGH the adapter;
    // these prove the ordering itself, with no conductor and no HDK.
    // ───────────────────────────────────────────────────────────────────

    /// Defect 2 (tier-blind resolution): ANY earned declaration beats ALL
    /// staging, even a staging link with a strictly NEWER timestamp — the
    /// partition-heal hazard, where a gossip-lagged peer wrote a newer staging
    /// link.
    #[test]
    fn earned_wins_over_newer_staging() {
        let earned_target = ah(10);
        let candidates = vec![
            cand(true, 100, 1, 10),  // earned, OLDER
            cand(false, 200, 2, 20), // staging, NEWER
            cand(false, 300, 3, 30), // staging, NEWEST
        ];
        let winner = select(candidates).expect("a winner exists");
        assert!(
            winner.is_earned,
            "earned tier must win regardless of staging recency"
        );
        assert_eq!(winner.target, earned_target);
    }

    /// Within the earned tier, newest timestamp wins; a newer STAGING link
    /// still loses to any earned link.
    #[test]
    fn newest_earned_wins_within_tier() {
        let newest_earned = ah(11);
        let candidates = vec![
            cand(true, 100, 1, 10),
            cand(true, 300, 3, 11), // newest earned
            cand(true, 200, 2, 12),
            cand(false, 999, 9, 90), // staging newer than all earned — still loses
        ];
        let winner = select(candidates).expect("winner");
        assert!(winner.is_earned);
        assert_eq!(winner.target, newest_earned);
    }

    /// All-staging set: newest staging wins (scaffold-over-scaffold semantics).
    #[test]
    fn newest_staging_wins_when_no_earned() {
        let newest = ah(30);
        let candidates = vec![
            cand(false, 100, 1, 10),
            cand(false, 300, 3, 30),
            cand(false, 200, 2, 20),
        ];
        let winner = select(candidates).expect("winner");
        assert!(!winner.is_earned);
        assert_eq!(winner.target, newest);
    }

    /// Defect 4 (timestamp-tie nondeterminism): equal timestamps break on the
    /// tiebreak hash, so an identical set — in ANY input order — yields the
    /// SAME winner on every peer. Asserts order-INDEPENDENCE (the real
    /// invariant), not a hard-coded direction.
    #[test]
    fn equal_timestamps_break_deterministically_on_link_hash() {
        let mk = || {
            vec![
                cand(true, 500, 5, 50),
                cand(true, 500, 8, 80), // same timestamp, distinct link hash
            ]
        };
        let w1 = select(mk()).expect("winner");
        let mut reversed = mk();
        reversed.reverse();
        let w2 = select(reversed).expect("winner");
        assert_eq!(
            w1.target, w2.target,
            "tie winner must not depend on arrival order"
        );
        assert_eq!(w1.link_hash, w2.link_hash);
    }

    /// Defect 3 (no fallback to superseded heads): the selector returns exactly
    /// ONE winner — the newest in the winning tier — so the resolver has no
    /// older/lower-tier candidate to fall back to when that winner's target is
    /// unretrievable.
    #[test]
    fn superseding_earned_is_the_sole_winner() {
        let superseded = ah(10);
        let current = ah(11);
        let candidates = vec![
            cand(true, 100, 1, 10),  // OLDER earned — the "retracted" head
            cand(true, 300, 3, 11),  // NEWER earned — the steward's correction
            cand(false, 400, 4, 40), // even-newer staging — still loses to earned
        ];
        let winner = select(candidates).expect("winner");
        assert_eq!(
            winner.target, current,
            "the newer earned declaration supersedes the older"
        );
        assert_ne!(
            winner.target, superseded,
            "the superseded (older earned) head is never selected — no fallback path"
        );
    }

    /// Empty set → no winner.
    #[test]
    fn empty_set_has_no_winner() {
        assert!(select(vec![]).is_none());
    }

    /// The winner's OWN ordering is what travels: the live consumer propagates
    /// `clock` and `tier` onto the projection and MOVES an already-declared row
    /// on them, so reporting any other candidate's clock would let a peer
    /// "advance" onto a losing declaration.
    #[test]
    fn winner_reports_its_own_clock_and_tier_not_a_losers() {
        let candidates = vec![
            cand(false, 900, 9, 90), // staging, NEWEST clock — must not be reported
            cand(true, 100, 1, 10),  // earned, older
            cand(true, 300, 3, 11),  // earned, newest-within-tier ⇒ the winner
        ];
        let winner = select(candidates).expect("winner");
        assert_eq!(winner.target, ah(11), "newest earned is the winner");
        assert_eq!(
            winner.timestamp, 300,
            "the propagated clock is the WINNER's link timestamp, never the newest link's — \
             a staging link at 900 must not become the ordering a peer advances on"
        );
        assert!(
            winner.is_earned,
            "the propagated tier is the winning tier; the consumer replays \
             earned-beats-staging on it"
        );
    }

    /// The tier bit must be reported honestly for an all-staging set too — a
    /// staging winner that reported the earned tier would gain protection
    /// downstream that it never won.
    #[test]
    fn staging_winner_reports_staging_tier() {
        let winner =
            select(vec![cand(false, 100, 1, 10), cand(false, 300, 3, 30)]).expect("winner");
        assert_eq!(winner.target, ah(30));
        assert_eq!(winner.timestamp, 300);
        assert!(
            !winner.is_earned,
            "an unmarked/staging declaration must never report the earned tier"
        );
    }

    /// The tiebreak decides the propagated CLOCK as well as the target: two
    /// peers reading the same tied set must advance on the same ordering, or
    /// a monotonic guard downstream would let them leapfrog each other forever.
    #[test]
    fn tied_winners_propagate_an_identical_clock_in_any_input_order() {
        let mk = || vec![cand(true, 500, 5, 50), cand(true, 500, 8, 80)];
        let w1 = select(mk()).expect("winner");
        let mut reversed = mk();
        reversed.reverse();
        let w2 = select(reversed).expect("winner");
        assert_eq!(w1.timestamp, w2.timestamp);
        assert_eq!(w1.is_earned, w2.is_earned);
        assert_eq!(w1.target, w2.target);
    }

    // ───────────────────────────── key-shape tests ─────────────────────────

    /// Field ORDER is the precedence — stated as its own assertion so a
    /// reordering of [`ArbitrationKey`]'s fields fails here with the reason,
    /// not three tests away with a puzzle.
    #[test]
    fn tier_precedence_dominates_a_newer_clock() {
        let earned_old = ArbitrationKey {
            tier: true,
            clock: 1i64,
            tiebreak: 0u8,
        };
        let staging_new = ArbitrationKey {
            tier: false,
            clock: i64::MAX,
            tiebreak: u8::MAX,
        };
        assert!(
            earned_old > staging_new,
            "tier is the FIRST field: a higher tier outranks every clock"
        );
    }

    /// A single-tier election passes `()` — the unit tier is a legal, zero-cost
    /// way to say "no tiers here," and it must not change the ordering.
    #[test]
    fn unit_tier_degrades_to_clock_then_tiebreak() {
        let items = vec![(5i64, 1u8), (5i64, 9u8), (4i64, 99u8)];
        let winner = select_arbitrated_winner(items, |&(clock, tiebreak)| ArbitrationKey {
            tier: (),
            clock,
            tiebreak,
        })
        .expect("winner");
        assert_eq!(winner, (5, 9), "newest clock, then greatest tiebreak");
    }

    // ─────────────────────── the harness on its own instance ───────────────

    /// **The Arbitrated harness run against its motivating instance.**
    ///
    /// `harness::arbitrated`'s module doc names `select_canonical_winner` as
    /// "the concrete instance" of the C2 property. Until this extraction the
    /// harness could not actually be pointed at it — the predicate lived
    /// behind HDK types in a WASM zome. It does now.
    ///
    /// The fixture carries the worst case on purpose: a genuine `(tier, clock)`
    /// tie that ONLY the tiebreak separates. Against the pre-cure shape (sort
    /// on the clock alone) the harness reports `PermutationDependent` — see
    /// `arbitrated_tests::permutation_dependent_selector_is_caught`, which runs
    /// exactly that regression fixture.
    #[cfg(feature = "harness")]
    #[test]
    fn real_predicate_is_arbitration_conformant() {
        use crate::harness::arbitrated::check_arbitration;

        let candidates = vec![
            cand(false, 10, 1, 10),  // staging
            cand(true, 500, 5, 50),  // earned ┐ tied on (tier, clock):
            cand(true, 500, 8, 80),  // earned ┘ only the tiebreak separates them
            cand(false, 999, 9, 90), // staging, newest clock of all
        ];

        let report = check_arbitration(&candidates, |set: &[Candidate]| {
            select_arbitrated_winner(set.to_vec(), key_of)
        })
        .expect("the canonical-head ordering is permutation-invariant and call-deterministic");

        assert!(
            report.exhaustive,
            "4! = 24 ≤ the default 720 budget, so EVERY ordering was tried — this run is a \
             proof over the fixture, not a sample"
        );
        assert_eq!(report.permutations_checked, 24);
    }

    /// The same property over a set with no ties at all — the ordinary case,
    /// checked so a regression that only breaks the tie path is distinguishable
    /// from one that breaks the ordering outright.
    #[cfg(feature = "harness")]
    #[test]
    fn untied_set_is_arbitration_conformant() {
        use crate::harness::arbitrated::assert_arbitration;

        let candidates = vec![
            cand(true, 100, 1, 10),
            cand(true, 300, 3, 11),
            cand(false, 999, 9, 90),
        ];
        assert_arbitration(&candidates, |set: &[Candidate]| {
            select_arbitrated_winner(set.to_vec(), key_of)
        });
    }

    /// The empty and singleton sets are trivially invariant — worth pinning
    /// because `None` is a legal winner and a harness pass on `None` is not
    /// evidence of anything else.
    #[cfg(feature = "harness")]
    #[test]
    fn empty_and_singleton_sets_are_arbitration_conformant() {
        use crate::harness::arbitrated::assert_arbitration;

        let none: Vec<Candidate> = vec![];
        assert_arbitration(&none, |set: &[Candidate]| {
            select_arbitrated_winner(set.to_vec(), key_of)
        });
        let one = vec![cand(true, 1, 1, 1)];
        assert_arbitration(&one, |set: &[Candidate]| {
            select_arbitrated_winner(set.to_vec(), key_of)
        });
    }
}
