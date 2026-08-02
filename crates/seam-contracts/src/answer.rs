//! `Answer<T>` — C4 (honest absence) as a type.
//!
//! Design: `genesis/docs/superpowers/plans/2026-08-02-seam-concern-contract-architecture-plan.md`
//! (canon row C4; Design surface 3).
//!
//! ## The guarantee
//!
//! > Absent, unreachable, refused, and unverifiable are distinct answers, and
//! > unreadable/unmeasured never reads as zero/complete.
//!
//! A boundary that returns `Option<T>` has one bit where the domain has at
//! least two provenances, and the collapse is silent: the caller cannot tell
//! *"I asked and the thing is not there"* from *"I never got an answer."* One
//! is a fact about the world; the other is a fact about the network. Treating
//! the second as the first is how a peer decides it is authoritative over an
//! absence it never observed.
//!
//! ## The full-arc law (the clause a first adoption most easily re-merges)
//!
//! On a **full-arc fleet** every zome `get` / `get_links` is local-only — the
//! conductor holds authority over the whole DHT space, so there is no network
//! fetch behind a miss. Therefore a local `get` miss means **gossip has not
//! delivered the record yet**, not that the record does not exist. It maps to
//! [`Answer::Unreachable`], never [`Answer::Absent`]. See
//! [`Answer::from_local_get`], which is the *only* honest constructor for that
//! call shape, and memory anchor `project_full_arc_authority_disables_network_get`.
//!
//! Plan canon, C4, verbatim: *"On a full-arc fleet a local `get` miss maps to
//! **Unreachable** (gossip failed), never Absent — the first adoption of
//! `Answer<T>` must not re-merge what wave 4 separated."*
//!
//! ## Prior art this generalizes (three independent re-derivations)
//!
//! - `elohim/elohim-storage/src/services/head_adoption.rs` — `LocalResolve`,
//!   whose `Known(None)` arm carries a 40-line `CONTRACT DEVIATION (2026-07-29)`
//!   comment enumerating *two* provenances (observed absence vs. resolve
//!   timeout). That comment is the type this module replaces prose with.
//! - `doorway/doorway-service/src/render/registry.rs` — `ReconcileDecision`,
//!   with an independently-derived `Unreachable(String)` arm.
//! - the render/fetch family (`FetchOutcome` / `RenderTerminal`), the freshness
//!   enum's `unverifiable`, and `epr-pull-status`'s documented "tri-state"
//!   nullable — the same distinction re-invented a third and fourth time.
//!
//! ## Scope: this is not the wire shape
//!
//! `Answer<T>` is Rust-generic. The wire is **monomorphic**: a per-view envelope
//! validated by `elohim/sdk/schemas/v1/objects/answer.schema.json`, authored
//! schema-first (plan task P1.4). The `serde` feature here derives a convenience
//! representation for internal and test use only — do not treat it as a
//! published contract, and do not add ts-rs generics (the repo has none, and a
//! flat 458-file generated directory is the wrong place for its first).

/// A boundary answer with the two absences kept apart — C4 as a type.
///
/// **Concerns:** C4 (honest absence). Composes with C8: [`Answer::state`]
/// yields an [`AnswerState`] that implements [`ReasonLabel`](crate::ReasonLabel),
/// so every boundary answer can increment a labeled counter without inventing a
/// string vocabulary at the call site.
///
/// **Forcing incident:** 2026-07-29, `LocalResolve::Known(None)` acquired a
/// second provenance (heal-loop resolve timeout) while its callers kept reading
/// it as observed absence — recorded in-place as a `CONTRACT DEVIATION` comment
/// because the type could not express it. Earlier and independently:
/// `dd1824e03` (2026-07-22) identity reconcile, *unreadable ≠ absent*, cured by
/// abstain-and-observe; `d6c88e385` (2026-07-23) shefa facing, *unmeasured ≠
/// zero*.
///
/// **Contract test:** `answer_tests` in this module; the label vocabulary is
/// proven stable by `answer_state_labels_are_stable`.
///
/// # Reading the variants
///
/// - [`Present`](Answer::Present) — the answer arrived and the thing exists.
/// - [`Absent`](Answer::Absent) — the answer arrived and the thing does not
///   exist. **This is a positive claim** and may only be constructed where
///   absence was genuinely *observed*.
/// - [`Unreachable`](Answer::Unreachable) — no answer arrived: timeout,
///   transport failure, undelivered gossip, an unresolvable validator. Nothing
///   is established in either direction.
///
/// Refusal and unverifiability are *reasons*, not states: a seam that must
/// distinguish "the peer declined" from "the peer timed out" pairs an `Answer`
/// with its own [`ReasonLabel`](crate::ReasonLabel) enum rather than growing a
/// fourth variant here. Keeping the state set at three is what lets every seam
/// share one vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Answer<T> {
    /// The answer arrived and carries a value.
    Present(T),
    /// The answer arrived and reports that the value does not exist —
    /// an *observed* absence, never an assumed one.
    Absent,
    /// No answer arrived. Absence is not established.
    Unreachable,
}

impl<T> Answer<T> {
    /// Construct from an `Option` whose `None` was **observed** — the responder
    /// answered, and answered "no such thing."
    ///
    /// **Concerns:** C4.
    ///
    /// **Forcing incident:** the deliberate absence of a blanket
    /// `impl From<Option<T>>` (see the module note below) — a bare conversion
    /// makes the wrong mapping the ergonomic default.
    ///
    /// **Contract test:** `observed_absence_maps_none_to_absent`.
    ///
    /// Use this only where the `None` provably came back *from* a responder. If
    /// the `None` might mean "we never heard," use
    /// [`from_local_get`](Answer::from_local_get) or
    /// [`from_result`](Answer::from_result).
    pub fn observed_absence(value: Option<T>) -> Self {
        match value {
            Some(v) => Answer::Present(v),
            None => Answer::Absent,
        }
    }

    /// Construct from a **full-arc local `get`**: a miss is `Unreachable`.
    ///
    /// **Concerns:** C4, and C0 (plane location — this constructor names the
    /// truth plane it is reading; a projection-plane miss is a different
    /// question with a different honest answer).
    ///
    /// **Forcing incident:** full-arc authority disables every network `get`
    /// (`target_arc_factor` defaults to 1), so a conductor's local miss is a
    /// gossip-delivery fact, not an existence fact. Reading it as existence is
    /// how a peer declares authority over content it has simply not received.
    ///
    /// **Contract test:** `from_local_get_maps_miss_to_unreachable`.
    pub fn from_local_get(value: Option<T>) -> Self {
        match value {
            Some(v) => Answer::Present(v),
            None => Answer::Unreachable,
        }
    }

    /// Construct from the common `Result<Option<T>, E>` responder shape:
    /// `Ok(Some)` → `Present`, `Ok(None)` → `Absent`, `Err` → `Unreachable`.
    ///
    /// **Concerns:** C4.
    ///
    /// **Forcing incident:** call sites that flattened `.ok().flatten()` and
    /// thereby made every transport error indistinguishable from a clean "not
    /// found" — the exact collapse `ReconcileDecision::Unreachable` was added
    /// to undo in doorway's render registry.
    ///
    /// **Contract test:** `from_result_keeps_error_and_none_apart`.
    ///
    /// The error value is dropped on purpose: `Answer` is a *state*, not a
    /// diagnosis. Carry the diagnosis in a [`ReasonLabel`](crate::ReasonLabel)
    /// enum beside the answer, so it can be counted (C8) rather than formatted.
    pub fn from_result<E>(value: Result<Option<T>, E>) -> Self {
        match value {
            Ok(Some(v)) => Answer::Present(v),
            Ok(None) => Answer::Absent,
            Err(_) => Answer::Unreachable,
        }
    }

    /// The answer's state, as a countable label — the C8 bridge.
    ///
    /// **Concerns:** C8 (observability-per-decision), C4.
    ///
    /// **Forcing incident:** `98a3c5f1e` (2026-08-01) — a metric label
    /// hardcoded to `0` "for symmetry"; structurally constant is worse than
    /// absent. Deriving the label from the answer itself makes a constant label
    /// impossible to write by accident.
    ///
    /// **Contract test:** `state_projects_every_variant`.
    pub fn state(&self) -> AnswerState {
        match self {
            Answer::Present(_) => AnswerState::Present,
            Answer::Absent => AnswerState::Absent,
            Answer::Unreachable => AnswerState::Unreachable,
        }
    }

    /// Borrow the payload without consuming the answer.
    ///
    /// **Concerns:** C4 (the state is preserved across the borrow).
    ///
    /// **Contract test:** `as_ref_preserves_state`.
    pub fn as_ref(&self) -> Answer<&T> {
        match self {
            Answer::Present(v) => Answer::Present(v),
            Answer::Absent => Answer::Absent,
            Answer::Unreachable => Answer::Unreachable,
        }
    }

    /// Map the payload, preserving the state.
    ///
    /// **Concerns:** C4 — a mapping function can never turn `Unreachable` into
    /// `Absent`, because it is never called on those arms.
    ///
    /// **Contract test:** `map_preserves_state`.
    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> Answer<U> {
        match self {
            Answer::Present(v) => Answer::Present(f(v)),
            Answer::Absent => Answer::Absent,
            Answer::Unreachable => Answer::Unreachable,
        }
    }

    /// `true` iff the answer carries a value.
    ///
    /// **Concerns:** C4.
    ///
    /// **Contract test:** `predicates_partition_the_state_space`.
    pub fn is_present(&self) -> bool {
        matches!(self, Answer::Present(_))
    }

    /// `true` iff absence was **observed**.
    ///
    /// **Concerns:** C4. Note the asymmetry with `!is_present()`: an
    /// `Unreachable` answer is not absent, and a gauge that counts
    /// `!is_present()` as "missing" re-commits the collapse this type exists to
    /// prevent (`d6c88e385`, *unmeasured ≠ zero*).
    ///
    /// **Contract test:** `predicates_partition_the_state_space`.
    pub fn is_absent(&self) -> bool {
        matches!(self, Answer::Absent)
    }

    /// `true` iff no answer arrived.
    ///
    /// **Concerns:** C4, C3 (liveness — an `Unreachable` answer is the state a
    /// retry/heal transition should be enabled *from*; an `Absent` one is not).
    ///
    /// **Contract test:** `predicates_partition_the_state_space`.
    pub fn is_unreachable(&self) -> bool {
        matches!(self, Answer::Unreachable)
    }

    /// The **lossy** projection back to `Option`, for adapters at a boundary
    /// that has not yet adopted `Answer`.
    ///
    /// **Concerns:** C4 — this is the collapse, made explicit and greppable.
    ///
    /// **Forcing incident:** P1.2's behavior-neutral adoption order requires a
    /// bridge; without a named one, adopters write `matches!(a, Present(_))`
    /// inline and the collapse becomes invisible again.
    ///
    /// **Contract test:** `into_option_is_the_named_collapse`.
    ///
    /// Every call site is a place where "not there" and "never heard" merge.
    /// A search for `into_option()` is a search for the remaining C4 debt.
    pub fn into_option(self) -> Option<T> {
        match self {
            Answer::Present(v) => Some(v),
            Answer::Absent | Answer::Unreachable => None,
        }
    }

    /// The payload, or `default` for either non-present state.
    ///
    /// **Concerns:** C4 — as lossy as [`into_option`](Answer::into_option), and
    /// named so it can be audited.
    ///
    /// **Contract test:** `unwrap_or_returns_default_for_both_absences`.
    pub fn unwrap_or(self, default: T) -> T {
        self.into_option().unwrap_or(default)
    }
}

// DELIBERATELY ABSENT: `impl<T> From<Option<T>> for Answer<T>`.
//
// **Concerns:** C4.
//
// A blanket conversion has to pick one mapping for `None`, and whichever it
// picks becomes the ergonomic default at every call site — including the call
// sites where it is wrong. The full-arc law makes `None → Absent` wrong for
// every conductor-local read, which is the majority of reads in the substrate.
// Naming the provenance at the call site (`observed_absence` / `from_local_get`
// / `from_result`) costs one word and makes the wrong choice visible in review.
//
// The same reasoning forbids `impl Default for Answer<T>`: there is no honest
// default answer.

/// The three answer states as a countable, stable label vocabulary — the C8
/// half of C4.
///
/// **Concerns:** C8 (observability-per-decision), C4.
///
/// **Forcing incident:** `b19f12014` (2026-07-31) — a gauge blind to the
/// difference between a correct refusal and a failure: *"not a conservative
/// gauge; an unreadable one."* A metric that cannot distinguish `absent` from
/// `unreachable` cannot tell an operator whether to heal or to accept.
///
/// **Contract test:** `answer_state_labels_are_stable` (golden label set) and
/// `answer_state_is_reason_label_conformant`.
///
/// The label strings are a **dashboard contract**: renaming one silently breaks
/// every panel and alert keyed on it. `answer_state_labels_are_stable` pins
/// them, which is the whole point of a golden test on a vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
// No `#[serde(rename_all = ...)]` and no `#[ts(export)]` on purpose: the wire
// vocabulary for this enum is `answer-state` in the SDK schema family, authored
// schema-first by plan task P1.4 (which will bind it to the `label()` strings
// below). Shaping it here would pre-empt the schema and create a second source
// of truth — the exact drift `schema_contract.rs` exists to catch.
pub enum AnswerState {
    /// The answer arrived with a value.
    Present,
    /// Absence was observed.
    Absent,
    /// No answer arrived.
    Unreachable,
}

impl crate::ReasonLabel for AnswerState {
    const ALL: &'static [Self] = &[
        AnswerState::Present,
        AnswerState::Absent,
        AnswerState::Unreachable,
    ];

    fn label(&self) -> &'static str {
        match self {
            AnswerState::Present => "present",
            AnswerState::Absent => "absent",
            AnswerState::Unreachable => "unreachable",
        }
    }
}

/// WHY an answer is not `Present` — the wire-facing reason vocabulary, and the
/// only reason vocabulary in this crate that a *client* is expected to render.
///
/// **Concerns:** C4 (honest absence — the reason is what makes a non-present
/// answer actionable), C8 (a typed, countable label), C5 (the
/// [`Unverifiable`](AnswerReason::Unverifiable) arm is the may-believe clause:
/// evidence that cannot be evaluated confers nothing, in either direction).
///
/// **Forcing incident:** the same distinction re-derived at four seams with four
/// private vocabularies — `LocalResolve`'s observed-vs-timed-out split,
/// `ReconcileDecision::Unreachable(String)`'s free-text error, the render
/// family's `Stalled`/`Errored`, and the freshness enum's `unverifiable`. Four
/// spellings of one idea is what makes a cross-seam dashboard impossible.
///
/// **Contract test:** `answer_reason_labels_are_stable` in this module; the
/// schema half is asserted by
/// `elohim/elohim-storage/tests/schema_contract.rs::answer_state_and_reason_schemas_match_the_rust_vocabulary`,
/// which reads `elohim/sdk/schemas/v1/enums/answer-reason.schema.json` from disk
/// — two genuinely independent sources, so the test cannot be measuring its own
/// mirror.
///
/// # Wire status
///
/// Governed by `elohim/sdk/schemas/v1/enums/answer-reason.schema.json`,
/// deliberately **without `_dna` metadata**: this vocabulary is never validated
/// by the DHT, and minting it as a DNA-validated enum would move the DNA hash
/// for something no integrity zome reads.
///
/// # Which state each reason belongs to
///
/// Exactly one reason ([`ObservedAbsent`](AnswerReason::ObservedAbsent)) belongs
/// to [`AnswerState::Absent`]; every other reason belongs to
/// [`AnswerState::Unreachable`]. [`AnswerState::Present`] takes no reason —
/// "why is it there" is not a question a boundary answers. See
/// [`AnswerReason::state`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
// Same rationale as `AnswerState`: no `rename_all` / `#[ts(export)]` here, so
// `answer-reason.schema.json` stays the single wire source of truth.
pub enum AnswerReason {
    /// The responder answered and reports the thing does not exist. The ONE
    /// reason that pairs with [`AnswerState::Absent`].
    ObservedAbsent,
    /// No answer arrived within the declared budget.
    Timeout,
    /// The request failed at the transport (connection, codec, an unparseable
    /// address, a peer too old to decode the request).
    TransportError,
    /// The responder was reached and declined — authorization, reach, or policy.
    /// Distinct from absence: something IS there, and we were not shown it.
    Refused,
    /// An answer arrived but could not be verified, so it establishes nothing.
    /// C5: unevaluatable evidence confers nothing, in either direction.
    Unverifiable,
    /// A local read missed on a full-arc holder: the record has not been
    /// delivered yet, which is a fact about gossip, never about existence.
    NotYetDelivered,
}

impl AnswerReason {
    /// The [`AnswerState`] this reason can accompany.
    ///
    /// **Concerns:** C4 — the pairing is total and fixed, so a caller cannot
    /// attach `timeout` to an `absent` answer and quietly assert an absence it
    /// never observed.
    ///
    /// **Contract test:** `only_observed_absent_pairs_with_absent`.
    pub fn state(&self) -> AnswerState {
        match self {
            AnswerReason::ObservedAbsent => AnswerState::Absent,
            AnswerReason::Timeout
            | AnswerReason::TransportError
            | AnswerReason::Refused
            | AnswerReason::Unverifiable
            | AnswerReason::NotYetDelivered => AnswerState::Unreachable,
        }
    }
}

impl crate::ReasonLabel for AnswerReason {
    const ALL: &'static [Self] = &[
        AnswerReason::ObservedAbsent,
        AnswerReason::Timeout,
        AnswerReason::TransportError,
        AnswerReason::Refused,
        AnswerReason::Unverifiable,
        AnswerReason::NotYetDelivered,
    ];

    fn label(&self) -> &'static str {
        match self {
            AnswerReason::ObservedAbsent => "observed_absent",
            AnswerReason::Timeout => "timeout",
            AnswerReason::TransportError => "transport_error",
            AnswerReason::Refused => "refused",
            AnswerReason::Unverifiable => "unverifiable",
            AnswerReason::NotYetDelivered => "not_yet_delivered",
        }
    }
}

#[cfg(test)]
mod answer_tests {
    use super::*;
    use crate::ReasonLabel;

    #[test]
    fn observed_absence_maps_none_to_absent() {
        assert_eq!(Answer::observed_absence(Some(7)), Answer::Present(7));
        assert_eq!(Answer::<u8>::observed_absence(None), Answer::Absent);
    }

    #[test]
    fn from_local_get_maps_miss_to_unreachable() {
        // The full-arc law. If this test ever "fails" because someone made a
        // local miss mean Absent, the fleet has re-acquired the wave-4 defect.
        assert_eq!(Answer::from_local_get(Some(7)), Answer::Present(7));
        assert_eq!(Answer::<u8>::from_local_get(None), Answer::Unreachable);
        assert_ne!(Answer::<u8>::from_local_get(None), Answer::Absent);
    }

    #[test]
    fn from_result_keeps_error_and_none_apart() {
        let ok_some: Result<Option<u8>, &str> = Ok(Some(1));
        let ok_none: Result<Option<u8>, &str> = Ok(None);
        let err: Result<Option<u8>, &str> = Err("timeout");
        assert_eq!(Answer::from_result(ok_some), Answer::Present(1));
        assert_eq!(Answer::from_result(ok_none), Answer::Absent);
        assert_eq!(Answer::from_result(err), Answer::Unreachable);
    }

    #[test]
    fn state_projects_every_variant() {
        assert_eq!(Answer::Present(1).state(), AnswerState::Present);
        assert_eq!(Answer::<u8>::Absent.state(), AnswerState::Absent);
        assert_eq!(Answer::<u8>::Unreachable.state(), AnswerState::Unreachable);
    }

    #[test]
    fn as_ref_preserves_state() {
        let present = Answer::Present(String::from("x"));
        assert_eq!(present.as_ref().state(), AnswerState::Present);
        assert_eq!(Answer::<u8>::Absent.as_ref().state(), AnswerState::Absent);
        assert_eq!(
            Answer::<u8>::Unreachable.as_ref().state(),
            AnswerState::Unreachable
        );
        // still owned
        assert_eq!(present, Answer::Present(String::from("x")));
    }

    #[test]
    fn map_preserves_state() {
        assert_eq!(Answer::Present(2).map(|v| v * 3), Answer::Present(6));
        assert_eq!(Answer::<u8>::Absent.map(|v| v * 3), Answer::Absent);
        assert_eq!(
            Answer::<u8>::Unreachable.map(|v| v * 3),
            Answer::Unreachable
        );
    }

    #[test]
    fn predicates_partition_the_state_space() {
        let cases: [Answer<u8>; 3] = [Answer::Present(1), Answer::Absent, Answer::Unreachable];
        for case in cases {
            let flags = [case.is_present(), case.is_absent(), case.is_unreachable()];
            assert_eq!(
                flags.iter().filter(|f| **f).count(),
                1,
                "exactly one predicate must hold for {case:?}"
            );
        }
        // The asymmetry the doc warns about, asserted:
        assert!(!Answer::<u8>::Unreachable.is_absent());
    }

    #[test]
    fn into_option_is_the_named_collapse() {
        assert_eq!(Answer::Present(5).into_option(), Some(5));
        assert_eq!(Answer::<u8>::Absent.into_option(), None);
        assert_eq!(Answer::<u8>::Unreachable.into_option(), None);
    }

    #[test]
    fn unwrap_or_returns_default_for_both_absences() {
        assert_eq!(Answer::Present(5).unwrap_or(9), 5);
        assert_eq!(Answer::<u8>::Absent.unwrap_or(9), 9);
        assert_eq!(Answer::<u8>::Unreachable.unwrap_or(9), 9);
    }

    /// Golden label set. These strings are a dashboard contract; changing one
    /// is a breaking change to every panel keyed on it.
    #[test]
    fn answer_state_labels_are_stable() {
        crate::assert_reason_labels_stable::<AnswerState>(&["present", "absent", "unreachable"]);
    }

    #[test]
    fn answer_state_is_reason_label_conformant() {
        crate::assert_reason_labels_conformant::<AnswerState>();
        crate::assert_reason_labels_discriminating::<AnswerState>();
        assert_eq!(AnswerState::ALL.len(), 3);
    }

    /// Golden label set for the wire reason vocabulary. Mirrored — deliberately,
    /// independently — by `answer-reason.schema.json`, which
    /// `elohim-storage/tests/schema_contract.rs` reads from disk and compares
    /// against these very labels.
    #[test]
    fn answer_reason_labels_are_stable() {
        crate::assert_reason_labels_stable::<AnswerReason>(&[
            "observed_absent",
            "timeout",
            "transport_error",
            "refused",
            "unverifiable",
            "not_yet_delivered",
        ]);
        crate::assert_reason_labels_discriminating::<AnswerReason>();
    }

    /// The pairing that keeps a reason from smuggling in an absence: exactly one
    /// reason belongs to `Absent`; every other reason belongs to `Unreachable`;
    /// no reason belongs to `Present`.
    #[test]
    fn only_observed_absent_pairs_with_absent() {
        let absent_reasons: Vec<_> = AnswerReason::ALL
            .iter()
            .filter(|r| r.state() == AnswerState::Absent)
            .collect();
        assert_eq!(absent_reasons, vec![&AnswerReason::ObservedAbsent]);

        for reason in AnswerReason::ALL {
            assert_ne!(
                reason.state(),
                AnswerState::Present,
                "{reason:?} must not claim a present answer — 'why is it there' is \
                 not a question a boundary answers"
            );
        }
    }
}
