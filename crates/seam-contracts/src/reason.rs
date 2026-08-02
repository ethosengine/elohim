//! `ReasonLabel` — C8 (observability-per-decision) as a compile shape.
//!
//! Design: `genesis/docs/superpowers/plans/2026-08-02-seam-concern-contract-architecture-plan.md`
//! (canon row C8; Design surface 3).
//!
//! ## The guarantee
//!
//! > Every decision outcome increments a labeled counter through a typed reason
//! > (`label()` from an enum, never a raw string); failures are counted beside
//! > successes; no label is structurally constant; and every meter names its
//! > semantics.
//!
//! The trait supplies the two halves a raw string cannot: a **closed variant
//! set** (so a dashboard can enumerate what it might see, and a new outcome
//! cannot appear unannounced) and a **stable projection to a label** (so the
//! panel keyed on that string keeps working across refactors).
//!
//! ## What belongs here, and what does not
//!
//! A `ReasonLabel` implementor is a **fieldless discriminant** — the bounds
//! (`Copy + Eq + Debug`) enforce it in practice. A reason that wants to carry
//! context is not a label; it is a *capsule*, and its home is C14's
//! `ResidualWitness` (plan task P4.7). Mixing the two produces a metric with
//! unbounded cardinality, which is a different way of having no metric at all.
//!
//! Substrate-internal reason enums (`StaleReason`, `LocalResolve`, the contest
//! failure classes) stay crate-local and implement this trait. Only
//! discriminations a *client must render* graduate to schema-governed enums in
//! the SDK schema family — a Prometheus label vocabulary is not a
//! protocol-versioned artifact, and minting one as a DHT-validated enum would
//! move the DNA hash for vocabulary the DHT never validates.
//!
//! ## One standing warning from history
//!
//! Hash-typed fields inside conductor signals must stay typed (`HoloHashB64`).
//! A `String` mirror at the msgpack boundary makes the meter go dark while the
//! decision itself succeeds — the failure mode where observability is lost
//! without anything turning red. The class has already recurred once
//! (`project_conductor_signal_msgpack_decode_class`).

/// A closed, countable outcome vocabulary for one decision point.
///
/// **Concerns:** C8 (observability-per-decision).
///
/// **Forcing incident:** `486982bb8` (2026-07-25) — a leg that *"fails 100% and
/// says nothing,"* its only signal a dropped `debug!`; and `98a3c5f1e`
/// (2026-08-01) — a label hardcoded `0` "for symmetry," where structurally
/// constant is worse than absent. Both are impossible to write against a typed
/// vocabulary that must enumerate itself.
///
/// **Contract test:** [`check_reason_labels`] / [`assert_reason_labels_conformant`],
/// applied to each implementor by that implementor's own test module.
///
/// # Implementing
///
/// ```
/// use seam_contracts::ReasonLabel;
///
/// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// enum ContestFailure {
///     NoLocalChain,
///     NotRetrievable,
///     StampNotNewer,
/// }
///
/// impl ReasonLabel for ContestFailure {
///     const ALL: &'static [Self] = &[
///         ContestFailure::NoLocalChain,
///         ContestFailure::NotRetrievable,
///         ContestFailure::StampNotNewer,
///     ];
///
///     fn label(&self) -> &'static str {
///         match self {
///             ContestFailure::NoLocalChain => "no_local_chain",
///             ContestFailure::NotRetrievable => "not_retrievable",
///             ContestFailure::StampNotNewer => "stamp_not_newer",
///         }
///     }
/// }
///
/// seam_contracts::assert_reason_labels_conformant::<ContestFailure>();
/// ```
///
/// `ALL` is authored by hand rather than derived by a proc-macro: the crate is
/// a leaf and stays one (convention over macros, per Design surface 2). The
/// cost of forgetting a variant in `ALL` is caught by
/// [`assert_reason_labels_stable`] the moment the golden set is pinned.
pub trait ReasonLabel: Copy + Eq + core::fmt::Debug + 'static {
    /// Every variant, in a stable order.
    ///
    /// The order is the vocabulary's own order — it is what
    /// [`assert_reason_labels_stable`] pins, so a dashboard's legend ordering
    /// is a contract too, not an accident of the match arms.
    const ALL: &'static [Self];

    /// The metric-safe label for this outcome.
    ///
    /// Must be deterministic, non-empty, and drawn from `[a-z0-9_.-]` — the
    /// conservative intersection of what Prometheus, Loki, and a log-grep can
    /// all carry unquoted.
    fn label(&self) -> &'static str;

    /// Accessor form of [`ALL`](ReasonLabel::ALL), for call sites that would
    /// rather write `R::all()` than name the associated constant.
    fn all() -> &'static [Self] {
        Self::ALL
    }
}

/// Why a [`ReasonLabel`] implementor failed conformance.
///
/// **Concerns:** C8 — this crate's own failure classes are reason-labeled, so
/// the contract surface obeys the contract it enforces.
///
/// **Contract test:** `violation_class_is_self_conformant`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReasonLabelViolationClass {
    /// `ALL` is empty — a decision point with no enumerable outcomes cannot be
    /// metered, and a dashboard has nothing to enumerate.
    EmptyVariantSet,
    /// Two variants project to the same label — the metric silently merges two
    /// distinct outcomes, which is how a correct refusal and a failure end up
    /// in the same bucket (`b19f12014`).
    DuplicateLabel,
    /// A label is the empty string.
    EmptyLabel,
    /// A label contains characters outside `[a-z0-9_.-]`.
    UnstableLabelCharacters,
    /// `label()` returned different values for the same variant across calls.
    NondeterministicLabel,
    /// The vocabulary has fewer than two variants, so the label is structurally
    /// constant — see [`assert_reason_labels_discriminating`].
    StructurallyConstant,
    /// The observed label set does not match the pinned golden set — see
    /// [`assert_reason_labels_stable`].
    UnstableVocabulary,
}

impl ReasonLabel for ReasonLabelViolationClass {
    const ALL: &'static [Self] = &[
        ReasonLabelViolationClass::EmptyVariantSet,
        ReasonLabelViolationClass::DuplicateLabel,
        ReasonLabelViolationClass::EmptyLabel,
        ReasonLabelViolationClass::UnstableLabelCharacters,
        ReasonLabelViolationClass::NondeterministicLabel,
        ReasonLabelViolationClass::StructurallyConstant,
        ReasonLabelViolationClass::UnstableVocabulary,
    ];

    fn label(&self) -> &'static str {
        match self {
            ReasonLabelViolationClass::EmptyVariantSet => "empty_variant_set",
            ReasonLabelViolationClass::DuplicateLabel => "duplicate_label",
            ReasonLabelViolationClass::EmptyLabel => "empty_label",
            ReasonLabelViolationClass::UnstableLabelCharacters => "unstable_label_characters",
            ReasonLabelViolationClass::NondeterministicLabel => "nondeterministic_label",
            ReasonLabelViolationClass::StructurallyConstant => "structurally_constant",
            ReasonLabelViolationClass::UnstableVocabulary => "unstable_vocabulary",
        }
    }
}

/// A conformance failure: the typed class plus the human-readable capsule.
///
/// **Concerns:** C8 (typed class), C14 (the `detail` capsule carries enough
/// context to act on without being a metric dimension).
///
/// **Contract test:** `duplicate_labels_are_rejected` and siblings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReasonLabelViolation {
    /// The typed failure class — countable, low cardinality.
    pub class: ReasonLabelViolationClass,
    /// Human-readable context. Never a metric label.
    pub detail: String,
}

impl core::fmt::Display for ReasonLabelViolation {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "[{}] {}", self.class.label(), self.detail)
    }
}

impl std::error::Error for ReasonLabelViolation {}

fn violation(class: ReasonLabelViolationClass, detail: impl Into<String>) -> ReasonLabelViolation {
    ReasonLabelViolation {
        class,
        detail: detail.into(),
    }
}

fn label_is_metric_safe(label: &str) -> bool {
    label
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-' || c == '.')
}

/// Check a [`ReasonLabel`] vocabulary for uniqueness, metric-safety, and
/// determinism.
///
/// **Concerns:** C8.
///
/// **Forcing incident:** a stringly-typed `inc_contest_failed(&str)` where two
/// raw literals plus a variable class shared one counter — nothing prevented a
/// typo from minting a fourth, permanently-zero series, and nothing prevented
/// two call sites from colliding on one string.
///
/// **Contract test:** the `reason_tests` module below (each violation class has
/// a fixture that provokes exactly it).
///
/// Returns the first violation found; the classes are ordered so the most
/// structural failure is reported first. Callers who want a panic use
/// [`assert_reason_labels_conformant`].
pub fn check_reason_labels<R: ReasonLabel>() -> Result<(), ReasonLabelViolation> {
    let all = R::ALL;
    if all.is_empty() {
        return Err(violation(
            ReasonLabelViolationClass::EmptyVariantSet,
            format!(
                "{} declares no variants in ALL",
                core::any::type_name::<R>()
            ),
        ));
    }

    let mut seen: Vec<&'static str> = Vec::with_capacity(all.len());
    for variant in all {
        let label = variant.label();
        if label.is_empty() {
            return Err(violation(
                ReasonLabelViolationClass::EmptyLabel,
                format!("{variant:?} projects to the empty string"),
            ));
        }
        if !label_is_metric_safe(label) {
            return Err(violation(
                ReasonLabelViolationClass::UnstableLabelCharacters,
                format!(
                    "{variant:?} projects to `{label}`, which leaves [a-z0-9_.-] — \
                     a label that needs quoting is a label that will be re-quoted differently \
                     by the next exporter"
                ),
            ));
        }
        if variant.label() != label {
            return Err(violation(
                ReasonLabelViolationClass::NondeterministicLabel,
                format!("{variant:?} returned two different labels across calls"),
            ));
        }
        if seen.contains(&label) {
            return Err(violation(
                ReasonLabelViolationClass::DuplicateLabel,
                format!(
                    "`{label}` is projected by more than one variant of {} — \
                     two distinct outcomes would merge into one series",
                    core::any::type_name::<R>()
                ),
            ));
        }
        seen.push(label);
    }

    Ok(())
}

/// Panicking form of [`check_reason_labels`], for use inside `#[test]`.
///
/// **Concerns:** C8.
///
/// **Contract test:** `answer_state_is_reason_label_conformant` (in
/// [`crate::answer`]) is the in-crate application.
///
/// # Panics
///
/// If the vocabulary violates any conformance rule.
pub fn assert_reason_labels_conformant<R: ReasonLabel>() {
    if let Err(v) = check_reason_labels::<R>() {
        panic!(
            "ReasonLabel conformance failed for {}: {v}",
            core::any::type_name::<R>()
        );
    }
}

/// Assert the vocabulary actually discriminates — at least two variants.
///
/// **Concerns:** C8 (*"no label is structurally constant"*).
///
/// **Forcing incident:** `98a3c5f1e` (2026-08-01) — a label hardcoded for
/// symmetry. A one-variant vocabulary is the type-level version of the same
/// mistake: the dimension exists in the metric and carries no information.
///
/// **Contract test:** `single_variant_vocabulary_is_not_discriminating`.
///
/// Separate from [`assert_reason_labels_conformant`] on purpose: a
/// single-variant enum is *legal* (a decision point may genuinely have one
/// outcome today), so this is an opt-in assertion for the vocabularies that
/// claim to discriminate.
///
/// # Panics
///
/// If the vocabulary has fewer than two variants.
pub fn assert_reason_labels_discriminating<R: ReasonLabel>() {
    if R::ALL.len() < 2 {
        panic!(
            "ReasonLabel conformance failed for {}: {} — {} variant(s); a metric dimension that \
             can only take one value carries no information (C8)",
            core::any::type_name::<R>(),
            ReasonLabelViolationClass::StructurallyConstant.label(),
            R::ALL.len()
        );
    }
}

/// Pin a vocabulary's label set — the golden test that makes a rename loud.
///
/// **Concerns:** C8, C10 (contract-evolution honesty — a label vocabulary is a
/// contract with every dashboard that reads it; a change is rejected or
/// observed, never silent).
///
/// **Forcing incident:** the typed-`ContestFailure` migration (plan task P1.3)
/// is explicitly required to keep the label strings unchanged so
/// `elohim_content_contest_failed_total{class}` panels keep working. Without a
/// pinned golden set, "unchanged" is a claim in a commit message rather than a
/// test.
///
/// **Contract test:** `answer_state_labels_are_stable` (in [`crate::answer`]);
/// `stable_rejects_reordering` here.
///
/// Order matters: `expected` is compared positionally against
/// [`ReasonLabel::ALL`], so reordering the variants is itself reported. A
/// vocabulary whose order is genuinely irrelevant should still pin one, because
/// legend ordering is what an operator's eye learns.
///
/// # Panics
///
/// If the observed label set differs from `expected` in content or order, or if
/// the vocabulary is not otherwise conformant.
pub fn assert_reason_labels_stable<R: ReasonLabel>(expected: &[&str]) {
    assert_reason_labels_conformant::<R>();
    let observed: Vec<&'static str> = R::ALL.iter().map(ReasonLabel::label).collect();
    if observed != expected {
        panic!(
            "ReasonLabel conformance failed for {}: {} — observed {:?}, pinned {:?}. \
             Label strings are a dashboard contract: every panel, alert, and recording rule \
             keyed on the old string goes permanently zero, silently. If the change is \
             intended, migrate the dashboards in the same change and re-pin here.",
            core::any::type_name::<R>(),
            ReasonLabelViolationClass::UnstableVocabulary.label(),
            observed,
            expected
        );
    }
}

#[cfg(test)]
mod reason_tests {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Empty {}

    impl ReasonLabel for Empty {
        const ALL: &'static [Self] = &[];
        fn label(&self) -> &'static str {
            match *self {}
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Duplicated {
        A,
        B,
    }

    impl ReasonLabel for Duplicated {
        const ALL: &'static [Self] = &[Duplicated::A, Duplicated::B];
        fn label(&self) -> &'static str {
            "same"
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum ShoutyLabel {
        Shouty,
    }

    impl ReasonLabel for ShoutyLabel {
        const ALL: &'static [Self] = &[ShoutyLabel::Shouty];
        fn label(&self) -> &'static str {
            "Not Safe"
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Blank {
        Nothing,
    }

    impl ReasonLabel for Blank {
        const ALL: &'static [Self] = &[Blank::Nothing];
        fn label(&self) -> &'static str {
            ""
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Solo {
        Only,
    }

    impl ReasonLabel for Solo {
        const ALL: &'static [Self] = &[Solo::Only];
        fn label(&self) -> &'static str {
            "only"
        }
    }

    #[test]
    fn empty_variant_set_is_rejected() {
        assert_eq!(
            check_reason_labels::<Empty>().unwrap_err().class,
            ReasonLabelViolationClass::EmptyVariantSet
        );
    }

    #[test]
    fn duplicate_labels_are_rejected() {
        assert_eq!(
            check_reason_labels::<Duplicated>().unwrap_err().class,
            ReasonLabelViolationClass::DuplicateLabel
        );
    }

    #[test]
    fn unsafe_label_characters_are_rejected() {
        assert_eq!(
            check_reason_labels::<ShoutyLabel>().unwrap_err().class,
            ReasonLabelViolationClass::UnstableLabelCharacters
        );
    }

    #[test]
    fn empty_label_is_rejected() {
        assert_eq!(
            check_reason_labels::<Blank>().unwrap_err().class,
            ReasonLabelViolationClass::EmptyLabel
        );
    }

    #[test]
    fn single_variant_vocabulary_is_conformant_but_not_discriminating() {
        // Legal: a decision point may genuinely have one outcome today.
        assert!(check_reason_labels::<Solo>().is_ok());
    }

    #[test]
    #[should_panic(expected = "structurally_constant")]
    fn single_variant_vocabulary_is_not_discriminating() {
        assert_reason_labels_discriminating::<Solo>();
    }

    #[test]
    #[should_panic(expected = "unstable_vocabulary")]
    fn stable_rejects_reordering() {
        assert_reason_labels_stable::<ReasonLabelViolationClass>(&[
            "duplicate_label",
            "empty_variant_set",
            "empty_label",
            "unstable_label_characters",
            "nondeterministic_label",
            "structurally_constant",
            "unstable_vocabulary",
        ]);
    }

    #[test]
    fn violation_class_is_self_conformant() {
        assert_reason_labels_conformant::<ReasonLabelViolationClass>();
        assert_reason_labels_discriminating::<ReasonLabelViolationClass>();
        assert_reason_labels_stable::<ReasonLabelViolationClass>(&[
            "empty_variant_set",
            "duplicate_label",
            "empty_label",
            "unstable_label_characters",
            "nondeterministic_label",
            "structurally_constant",
            "unstable_vocabulary",
        ]);
    }

    #[test]
    fn all_accessor_matches_the_constant() {
        assert_eq!(
            ReasonLabelViolationClass::all(),
            ReasonLabelViolationClass::ALL
        );
    }

    #[test]
    fn violation_display_carries_class_and_detail() {
        let v = violation(ReasonLabelViolationClass::EmptyLabel, "because");
        assert_eq!(v.to_string(), "[empty_label] because");
    }
}
