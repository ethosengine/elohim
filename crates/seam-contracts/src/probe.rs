//! `Reading<T>` — the probe shape a served promise is observed through.
//!
//! Design: `genesis/docs/superpowers/specs/2026-10-20-placement-contract-interfaces-design.md`
//! §2.I (findings I1, I2, I5, I7, I10, I12); spec §9 of
//! `genesis/docs/superpowers/specs/2026-10-20-what-every-app-inherits-placement-contract-design.md`
//! names the promises.
//!
//! ## The guarantee
//!
//! > A probe with nothing to read says so by returning `None` — rendered *unmeasured* — and never
//! > `Present(healthy)`. An `Unreachable` answer always carries a real transport reason. Every
//! > reading states its evidence stage (`fixture` or `real`) and who may read it.
//!
//! ## Why the answer and its reason are one value
//!
//! [`Answer<T>`](crate::Answer) and [`AnswerReason`] are separate types, so a reading that carried
//! both could say `Present` with a reason, `Unreachable` with none, or `Unreachable` with
//! `observed_absent` (a reason that belongs to `Absent`). [`ProbeAnswer`] puts them in one value:
//! `Present` has no reason field, `Absent` implies `observed_absent`, and `Unreachable` holds an
//! [`UnreachableReason`], which only [`UnreachableReason::new`] builds and which refuses
//! `observed_absent`. None of the three wrong shapes can be written.
//!
//! ## Unmeasured stays outside `Reading`
//!
//! "Nothing has been read yet" is not an `Unreachable` with an invented reason — it is
//! [`Probe::read`] returning `None`. That keeps [`AnswerReason`] (published, exhaustive) free of a
//! variant no transport produces, and keeps a dashboard from counting "not measured" as a failure.
//!
//! ## Scope: this is not the wire shape
//!
//! `Reading<T>` stays internal to Rust. The wire envelope is
//! `elohim/sdk/schemas/v1/objects/probe-reading.schema.json`. Per-promise wire structs (for example,
//! `MaterializationReadingView`) are planned for `crates/elohim-views`; they are not implemented by
//! this crate. The evidence vocabulary is `elohim/sdk/schemas/v1/enums/evidence-stage.schema.json`.
//! The `serde` derives here are the internal convenience form.

use crate::answer::{AnswerReason, AnswerState};
use crate::reason::ReasonLabel;

/// A transport reason that belongs to [`AnswerState::Unreachable`]: every [`AnswerReason`] except
/// `ObservedAbsent`.
///
/// **Concerns:** C4 (honest absence — an `Unreachable` reading cannot claim an observed absence),
/// C8 (the reason is a [`ReasonLabel`] and counts under its own label).
///
/// **Forcing incident:** interfaces spec finding I1 (2026-10-20): with the answer and its reason as
/// separate fields, a probe could write `Unreachable` paired with `observed_absent`, and the wire
/// envelope's three-branch partition would be lost in the Rust type that feeds it.
///
/// **Contract test:** `unreachable_reason_refuses_observed_absent`.
///
/// The field is private, so the only way in is [`UnreachableReason::new`] (or deserializing, which
/// goes through the same check).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(try_from = "AnswerReason", into = "AnswerReason")
)]
pub struct UnreachableReason(AnswerReason);

impl UnreachableReason {
    /// Accept a reason that belongs to `Unreachable`; give `ObservedAbsent` back as the error.
    ///
    /// **Concerns:** C4.
    ///
    /// **Contract test:** `unreachable_reason_refuses_observed_absent`.
    pub fn new(reason: AnswerReason) -> Result<Self, AnswerReason> {
        if reason.state() == AnswerState::Unreachable {
            Ok(Self(reason))
        } else {
            Err(reason)
        }
    }

    /// The reason itself.
    pub fn reason(&self) -> AnswerReason {
        self.0
    }
}

impl TryFrom<AnswerReason> for UnreachableReason {
    type Error = &'static str;

    fn try_from(reason: AnswerReason) -> Result<Self, Self::Error> {
        Self::new(reason)
            .map_err(|_| "observed_absent belongs to Absent, not Unreachable, in a probe answer")
    }
}

impl From<UnreachableReason> for AnswerReason {
    fn from(r: UnreachableReason) -> Self {
        r.0
    }
}

/// The transport reasons are a label vocabulary of their own: `AnswerReason`'s labels without
/// `observed_absent`, so `ProbeAnswer::Unreachable(r) => r.label()` counts under the same label as
/// the reason inside.
///
/// **Concerns:** C8.
///
/// **Contract test:** `unreachable_reason_labels_are_answer_reason_labels`.
impl ReasonLabel for UnreachableReason {
    const ALL: &'static [Self] = &[
        UnreachableReason(AnswerReason::Timeout),
        UnreachableReason(AnswerReason::TransportError),
        UnreachableReason(AnswerReason::Refused),
        UnreachableReason(AnswerReason::Unverifiable),
        UnreachableReason(AnswerReason::NotYetDelivered),
    ];

    fn label(&self) -> &'static str {
        self.0.label()
    }
}

/// A probe's answer and its reason in one value — I1.
///
/// **Concerns:** C4 (the three states stay three, and each carries exactly the reason it may), C8
/// ([`ProbeAnswer::reason`] gives the label to count).
///
/// **Forcing incident:** as [`UnreachableReason`] (interfaces spec, I1).
///
/// **Contract test:** `probe_answer_states_and_reasons_pair` and
/// `unreachable_constructor_refuses_observed_absent`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub enum ProbeAnswer<T> {
    /// Something was read and it carries a value. Never built when nothing was read.
    Present(T),
    /// The probe read and observed that the thing is not there (reason `observed_absent`).
    Absent,
    /// No answer arrived; the transport reason says why.
    Unreachable(UnreachableReason),
}

impl<T> ProbeAnswer<T> {
    /// Something was read.
    pub fn present(v: T) -> Self {
        ProbeAnswer::Present(v)
    }

    /// An observed absence.
    pub fn absent() -> Self {
        ProbeAnswer::Absent
    }

    /// No answer arrived, for `reason`. Refuses `ObservedAbsent`, giving it back as the error.
    ///
    /// **Contract test:** `unreachable_constructor_refuses_observed_absent`.
    pub fn unreachable(reason: AnswerReason) -> Result<Self, AnswerReason> {
        UnreachableReason::new(reason).map(ProbeAnswer::Unreachable)
    }

    /// The answer's state.
    ///
    /// **Contract test:** `probe_answer_states_and_reasons_pair`.
    pub fn state(&self) -> AnswerState {
        match self {
            ProbeAnswer::Present(_) => AnswerState::Present,
            ProbeAnswer::Absent => AnswerState::Absent,
            ProbeAnswer::Unreachable(_) => AnswerState::Unreachable,
        }
    }

    /// The reason that accompanies the state: `None` for `Present`, `ObservedAbsent` for `Absent`,
    /// and the transport reason for `Unreachable`. Always `reason.state() == self.state()`.
    ///
    /// **Contract test:** `probe_answer_states_and_reasons_pair`.
    pub fn reason(&self) -> Option<AnswerReason> {
        match self {
            ProbeAnswer::Present(_) => None,
            ProbeAnswer::Absent => Some(AnswerReason::ObservedAbsent),
            ProbeAnswer::Unreachable(r) => Some(r.reason()),
        }
    }
}

/// One observation of a served promise — the probe shape (spec §9).
///
/// **Concerns:** C4 (the answer is a [`ProbeAnswer`]; unmeasured is `None` outside it), C5
/// (evidence-not-authority — `evidence` is required, so a fixture observation is never read as a
/// real one), C2 (`observed_at` is supplied by the caller; this crate reads no clock).
///
/// **Forcing incident:** the habit `evidence-class-descends`
/// (`elohim/elohim-storage/.epr-meta/evidence-class-descends.habit.md`) declared the
/// `fixture | real` distinction with no field to carry it (interfaces spec, finding 16), and the
/// placement design needs one envelope for every promise it probes (finding I12 for the clock).
///
/// **Contract test:** `reading_carries_every_part` and `tests/probe.rs`.
///
/// A reading is built with [`Reading::new`], which takes the evidence stage and the disclosure as
/// required arguments (neither has a default):
///
/// ```
/// use seam_contracts::{Disclosure, EvidenceStage, ProbeAnswer, Reading};
/// let r: Reading<u8> = Reading::new(
///     "p@1".into(), "c".into(), "s".into(), ProbeAnswer::Absent,
///     EvidenceStage::Fixture, Disclosure::Holder, "2026-10-20T00:00:00Z".into(),
/// );
/// assert_eq!(r.disclosure(), &Disclosure::Holder);
/// ```
///
/// The disclosure is fixed when the reading is built. `disclosure` is a private field read through
/// [`Reading::disclosure`], so an existing reading cannot be relabelled, either by a struct update
/// from a clone:
///
/// ```compile_fail
/// use seam_contracts::{Disclosure, Reading};
/// fn widen(r: &Reading<u8>) -> Reading<u8> {
///     Reading { disclosure: Disclosure::AtReach { openness: 4 }, ..r.clone() }
/// }
/// ```
///
/// or in place:
///
/// ```compile_fail
/// use seam_contracts::{Disclosure, Reading};
/// fn widen(r: &mut Reading<u8>) { r.disclosure = Disclosure::AtReach { openness: 4 }; }
/// ```
///
/// What a type cannot stop: a producer that calls [`Reading::new`] with fields copied out of
/// another reading declares a new disclosure for a new reading. Routing follows the disclosure a
/// reading was built with; it does not check that disclosure against the promise's row in spec §9.
///
/// It serializes but does not deserialize, as [`Declassified`]: deserializing would be a second
/// constructor that skips [`Reading::new`], so a serialized holder reading could come back with
/// its disclosure rewritten. A received reading is read through the wire schema
/// (`objects/probe-reading.schema.json`), not rebuilt as this type:
///
/// ```compile_fail
/// use seam_contracts::Reading;
/// fn rebuild<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Reading<u8>, D::Error> {
///     <Reading<u8> as serde::Deserialize>::deserialize(d)
/// }
/// ```
///
/// The observation and its provenance are fixed at construction too. Consumers read them through
/// getters; they cannot promote fixture evidence or replace its answer in place:
///
/// ```compile_fail,E0616
/// use seam_contracts::{EvidenceStage, Reading};
/// fn promote(r: &mut Reading<u8>) { r.evidence = EvidenceStage::Real; }
/// ```
///
/// ```compile_fail,E0616
/// use seam_contracts::{ProbeAnswer, Reading};
/// fn replace(r: &mut Reading<u8>) { r.answer = ProbeAnswer::Present(1); }
/// ```
///
/// A clone cannot be relabelled with a struct update either:
///
/// ```compile_fail,E0451
/// use seam_contracts::{EvidenceStage, Reading};
/// fn promote(r: &Reading<u8>) -> Reading<u8> {
///     Reading { evidence: EvidenceStage::Real, ..r.clone() }
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize),
    serde(rename_all = "camelCase")
)]
pub struct Reading<T> {
    /// The promise kind (spec §9), e.g. `bytes-layer-reads-only-what-is-read@1`.
    promise: String,
    /// The `@concern:` tag joining the reading to its a2o scenario and habit.
    concern: String,
    /// What was observed: a root CID, a need key, an app id.
    subject: String,
    /// The answer. Never `Present(healthy)` when nothing was read.
    answer: ProbeAnswer<T>,
    /// Whether this came from a fixture or a real run.
    evidence: EvidenceStage,
    /// Who may read this reading. Private: fixed by [`Reading::new`], read by
    /// [`Reading::disclosure`].
    disclosure: Disclosure,
    /// When it was observed, caller-supplied RFC 3339.
    observed_at: String,
}

impl<T> Reading<T> {
    /// A reading, with its observation, provenance and disclosure fixed for its lifetime.
    ///
    /// **Concerns:** C1 (who may read it is decided where the reading is made, not by whoever later
    /// holds it), C5 (`evidence` is a required argument).
    ///
    /// **Contract test:** `reading_carries_every_part`; the `compile_fail` doctests on [`Reading`].
    pub fn new(
        promise: String,
        concern: String,
        subject: String,
        answer: ProbeAnswer<T>,
        evidence: EvidenceStage,
        disclosure: Disclosure,
        observed_at: String,
    ) -> Self {
        Self {
            promise,
            concern,
            subject,
            answer,
            evidence,
            disclosure,
            observed_at,
        }
    }

    /// Who may read this reading, as it was built.
    ///
    /// **Contract test:** `reading_carries_every_part`.
    pub fn disclosure(&self) -> &Disclosure {
        &self.disclosure
    }

    /// The promise kind (spec §9).
    ///
    /// **Contract test:** `reading_carries_every_part`.
    pub fn promise(&self) -> &str {
        &self.promise
    }

    /// The `@concern:` tag.
    ///
    /// **Contract test:** `reading_carries_every_part`.
    pub fn concern(&self) -> &str {
        &self.concern
    }

    /// What was observed.
    ///
    /// **Contract test:** `reading_carries_every_part`.
    pub fn subject(&self) -> &str {
        &self.subject
    }

    /// The answer.
    ///
    /// **Contract test:** `reading_carries_every_part`.
    pub fn answer(&self) -> &ProbeAnswer<T> {
        &self.answer
    }

    /// Whether this came from a fixture or a real run.
    ///
    /// **Contract test:** `reading_carries_every_part`.
    pub fn evidence(&self) -> EvidenceStage {
        self.evidence
    }

    /// When it was observed, as the caller supplied it.
    ///
    /// **Contract test:** `reading_carries_every_part`.
    pub fn observed_at(&self) -> &str {
        &self.observed_at
    }

    /// `true` when only the holder may read it, so it may reach only a [`HolderSink`].
    ///
    /// **Contract test:** `reading_carries_every_part`.
    pub fn is_holder_only(&self) -> bool {
        self.disclosure == Disclosure::Holder
    }

    /// Sort the reading by who may read it. This is the only way to build the [`HolderReading`],
    /// [`PartyReading`] and [`SharedReading`] that the sinks accept, so a writer can be handed a
    /// reading only in the form its disclosure allows.
    ///
    /// **Concerns:** C1 (a holder reading reaches only a [`HolderSink`]; a party-scoped reading
    /// never reaches a [`SharedSink`]), C7.
    ///
    /// **Contract test:** `route_sorts_by_disclosure` and
    /// `tests/probe.rs::holder_readings_reach_only_a_holder_sink`.
    pub fn route(self) -> Routed<T> {
        match self.disclosure {
            Disclosure::Holder => Routed::Holder(HolderReading(self)),
            Disclosure::Parties { .. } => Routed::Parties(PartyReading(self)),
            Disclosure::AtReach { .. } => Routed::Shared(SharedReading(self)),
        }
    }
}

/// A [`Reading`] sorted by its [`Disclosure`] — what [`Reading::route`] returns.
///
/// **Concerns:** C1, C7 (what a reading says about who may see it decides which writer type can
/// take it).
///
/// **Forcing incident:** fix round 1 of C1 (2026-10-20): with the routing rule written by each host
/// as `if is_holder_only() { .. } else { public }`, a party-scoped reading fell into the public
/// branch, and the contract test that pinned the rule tested its own copy of it.
///
/// **Contract test:** `route_sorts_by_disclosure`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Routed<T> {
    /// Only the holder may read it; it may reach a [`HolderSink`].
    Holder(HolderReading<T>),
    /// Only the named parties may read it; it may reach neither a holder sink nor a shared one.
    Parties(PartyReading<T>),
    /// Anyone at its reach may read it; it may reach a [`SharedSink`].
    Shared(SharedReading<T>),
}

/// A reading only its holder may read. Built only by [`Reading::route`].
///
/// **Concerns:** C1.
///
/// **Contract test:** `route_sorts_by_disclosure`.
///
/// The field is private, so a holder reading cannot be wrapped as a shared one:
///
/// ```compile_fail
/// use seam_contracts::{HolderReading, Reading};
/// fn wrap<T>(r: Reading<T>) -> HolderReading<T> { HolderReading(r) }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HolderReading<T>(Reading<T>);

impl<T> HolderReading<T> {
    /// The reading, to encode for a holder writer.
    pub fn reading(&self) -> &Reading<T> {
        &self.0
    }

    /// The holder's choice to let named counts from this reading leave. This is the only way to
    /// build a [`Declassified`], so counts can leave only from a holder reading, and the promise
    /// they name is this reading's.
    ///
    /// **Concerns:** C1 (leaving is the holder's choice, named by `chosen_by`).
    ///
    /// **Contract test:** `declassified_comes_only_from_a_holder_reading`.
    pub fn declassify(&self, counts: Vec<(String, u64)>, chosen_by: String) -> Declassified {
        Declassified {
            promise: self.0.promise.clone(),
            counts,
            chosen_by,
        }
    }
}

/// A reading only the parties named by role may read. Built only by [`Reading::route`].
///
/// **Concerns:** C1 (a reading about a party is not broadcast past the parties).
///
/// **Contract test:** `route_sorts_by_disclosure`.
///
/// No sink trait takes it yet: delivery to the named parties belongs to the seam that carries for
/// them, and the type exists so that no [`SharedSink`] or [`HolderSink`] can be handed one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartyReading<T>(Reading<T>);

impl<T> PartyReading<T> {
    /// The reading.
    pub fn reading(&self) -> &Reading<T> {
        &self.0
    }

    /// The roles that may read it.
    pub fn roles(&self) -> &[PartyRole] {
        match &self.0.disclosure {
            Disclosure::Parties { roles } => roles,
            // Only `Reading::route` builds a PartyReading, and only from a Parties disclosure.
            _ => &[],
        }
    }
}

/// A reading anyone at its reach may read. Built only by [`Reading::route`].
///
/// **Concerns:** C1, C7.
///
/// **Contract test:** `route_sorts_by_disclosure`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedReading<T>(Reading<T>);

impl<T> SharedReading<T> {
    /// The reading.
    pub fn reading(&self) -> &Reading<T> {
        &self.0
    }

    /// The reach's openness ordinal.
    pub fn openness(&self) -> u8 {
        match self.0.disclosure {
            Disclosure::AtReach { openness } => openness,
            // Only `Reading::route` builds a SharedReading, and only from an AtReach disclosure.
            _ => 0,
        }
    }

    /// Hand the reading to `sink` if everyone who reads through the sink is inside the reading's
    /// reach. This is the only way to build the [`Admitted`] that [`SharedSink::write_shared`]
    /// takes, so the reach comparison is made here, once, and no adapter sink makes it or skips it.
    ///
    /// Refuses, writing nothing, when the reading's openness or the sink's audience is not a reach
    /// ordinal ([`OPENNESS_MIN`]..=[`OPENNESS_MAX`]), or when the sink's audience is more open than
    /// the reading's reach: a trusted-reach reading never reaches a public gauge.
    ///
    /// **Concerns:** C1 (a reading is not broadcast past its reach), C7 (what a sink declares about
    /// its readers decides what it may be handed), C8 (a refusal is a [`ShareRefusal`] label).
    ///
    /// **Contract test:** `deliver_compares_the_reach_with_the_audience` and
    /// `tests/probe.rs::a_shared_reading_reaches_only_a_sink_inside_its_reach`.
    pub fn deliver<S: SharedSink<T> + ?Sized>(&self, sink: &mut S) -> Result<(), ShareRefusal> {
        let reach = self.openness();
        let audience = sink.audience_openness();
        if !(OPENNESS_MIN..=OPENNESS_MAX).contains(&reach) {
            return Err(ShareRefusal::ReachOutOfRange);
        }
        if !(OPENNESS_MIN..=OPENNESS_MAX).contains(&audience) {
            return Err(ShareRefusal::AudienceOutOfRange);
        }
        if audience > reach {
            return Err(ShareRefusal::AudienceWiderThanReach);
        }
        sink.write_shared(&Admitted(self));
        Ok(())
    }
}

/// The least `Reach::openness()` ordinal (`private`).
///
/// **Contract test:** `tests/probe.rs::openness_ordinals_match_the_reach_schema`, which reads
/// `elohim/sdk/schemas/v1/enums/reach.schema.json`; elohim-epr's `openness_matches_generated_ordinal`
/// pins `Reach::openness()` to the same schema.
pub const OPENNESS_MIN: u8 = 1;
/// The greatest `Reach::openness()` ordinal (`commons`).
///
/// **Contract test:** as [`OPENNESS_MIN`].
pub const OPENNESS_MAX: u8 = 8;
/// `Reach::Public.openness()`: the audience a public gauge or exporter declares.
///
/// **Contract test:** as [`OPENNESS_MIN`].
pub const PUBLIC_OPENNESS: u8 = 7;

/// A [`SharedReading`] that [`SharedReading::deliver`] has compared with the sink's audience. Built
/// only there, so a [`SharedSink`] can be handed a reading only after the comparison:
///
/// ```compile_fail
/// use seam_contracts::{Admitted, SharedReading};
/// fn skip<T>(r: &SharedReading<T>) -> Admitted<'_, T> { Admitted(r) }
/// ```
///
/// **Concerns:** C1, C7.
///
/// **Contract test:** `deliver_compares_the_reach_with_the_audience`.
#[derive(Debug)]
pub struct Admitted<'a, T>(&'a SharedReading<T>);

impl<T> Admitted<'_, T> {
    /// The admitted reading.
    pub fn reading(&self) -> &SharedReading<T> {
        self.0
    }
}

/// Why [`SharedReading::deliver`] refused to hand a reading to a sink.
///
/// **Concerns:** C1, C8 (each refusal counts under its own label).
///
/// **Forcing incident:** Codex review 1 of C1 (2026-10-20), finding 3: every `AtReach` reading,
/// whatever its openness, reached any `SharedSink`, so a trusted-reach reading reached a public
/// gauge and the reach comparison was left to each adapter.
///
/// **Contract test:** `share_refusal_labels_are_stable`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ShareRefusal {
    /// The reading's openness is not a reach ordinal.
    ReachOutOfRange,
    /// The sink's declared audience is not a reach ordinal.
    AudienceOutOfRange,
    /// The sink's readers are more open than the reading's reach.
    AudienceWiderThanReach,
}

impl ReasonLabel for ShareRefusal {
    const ALL: &'static [Self] = &[
        ShareRefusal::ReachOutOfRange,
        ShareRefusal::AudienceOutOfRange,
        ShareRefusal::AudienceWiderThanReach,
    ];

    fn label(&self) -> &'static str {
        match self {
            ShareRefusal::ReachOutOfRange => "reach-out-of-range",
            ShareRefusal::AudienceOutOfRange => "audience-out-of-range",
            ShareRefusal::AudienceWiderThanReach => "audience-wider-than-reach",
        }
    }
}

impl core::fmt::Display for ShareRefusal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.label())
    }
}

impl std::error::Error for ShareRefusal {}

/// Whether a reading came from a fixture or a real run — the habit `evidence-class-descends`'s two
/// values (I10).
///
/// **Concerns:** C5 (only `Real` evidence can settle; a fixture confers nothing outside the test it
/// ran in), C8 (a stable label vocabulary).
///
/// **Forcing incident:** as [`Reading`] (interfaces spec, finding 16).
///
/// **Contract test:** `evidence_stage_labels_are_stable`. The labels equal the values of
/// `enums/evidence-stage.schema.json`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub enum EvidenceStage {
    /// Observed on a fixture: a household, a test, a staged cast.
    Fixture,
    /// Observed on a real run.
    Real,
}

impl ReasonLabel for EvidenceStage {
    const ALL: &'static [Self] = &[EvidenceStage::Fixture, EvidenceStage::Real];

    fn label(&self) -> &'static str {
        match self {
            EvidenceStage::Fixture => "fixture",
            EvidenceStage::Real => "real",
        }
    }
}

/// Who may read a reading, in existing vocabulary (I5).
///
/// **Concerns:** C1 (a reading about a party is not broadcast past the parties), C7 (what a probe
/// advertises about who sees it is what its writers serve — see [`HolderSink`]).
///
/// **Forcing incident:** interfaces spec I5 and I7 (2026-10-20): spec §9's rows name who may see
/// each promise's readings (the holder alone, the parties by role, or anyone at a reach) and no
/// type carried it, so a holder-only count could reach a public gauge.
///
/// **Contract test:** `disclosure_distinguishes_its_forms`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(
        rename_all = "kebab-case",
        rename_all_fields = "camelCase",
        tag = "disclosure"
    )
)]
pub enum Disclosure {
    /// The holder alone.
    Holder,
    /// The parties named by role.
    Parties {
        /// The roles that may read it.
        roles: Vec<PartyRole>,
    },
    /// Anyone at a reach, carried as `Reach::openness()`'s ordinal.
    AtReach {
        /// The reach's openness ordinal.
        openness: u8,
    },
}

/// A party a reading may be disclosed to, by role (spec §9's rows).
///
/// **Concerns:** C1, C8 (a stable label vocabulary).
///
/// **Forcing incident:** as [`Disclosure`] (interfaces spec, I5).
///
/// **Contract test:** `party_role_labels_are_stable`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub enum PartyRole {
    /// The peer carrying something for another.
    Carrier,
    /// The peer something is carried for.
    CarriedFor,
    /// The one who granted a capability.
    Grantor,
    /// A custodian of a key or a record.
    Custodian,
    /// A member leaving.
    Leaver,
    /// A holder of the thing observed.
    Holder,
    /// The settlement record.
    Settlement,
}

impl ReasonLabel for PartyRole {
    const ALL: &'static [Self] = &[
        PartyRole::Carrier,
        PartyRole::CarriedFor,
        PartyRole::Grantor,
        PartyRole::Custodian,
        PartyRole::Leaver,
        PartyRole::Holder,
        PartyRole::Settlement,
    ];

    fn label(&self) -> &'static str {
        match self {
            PartyRole::Carrier => "carrier",
            PartyRole::CarriedFor => "carried-for",
            PartyRole::Grantor => "grantor",
            PartyRole::Custodian => "custodian",
            PartyRole::Leaver => "leaver",
            PartyRole::Holder => "holder",
            PartyRole::Settlement => "settlement",
        }
    }
}

/// A probe of one served promise.
///
/// **Concerns:** C4 (`None` is unmeasured, never `Present(healthy)` and never an invented
/// `Unreachable`), C2 (the observation time is the caller's).
///
/// **Forcing incident:** interfaces spec I2 (2026-10-20): "not measured" had no home except a new
/// `AnswerReason` variant no transport produces; returning `None` keeps it outside the reading, as
/// `LocalResolve` keeps "not resolved" outside its answer.
///
/// **Contract test:** `tests/probe.rs::a_probe_with_nothing_to_read_returns_none`.
pub trait Probe {
    /// The reading's value.
    type Value;
    /// The promise kind this probe observes.
    fn promise(&self) -> &'static str;
    /// The current reading, or `None` when there is nothing to read yet (rendered unmeasured).
    fn read(&self, observed_at: &str) -> Option<Reading<Self::Value>>;
}

/// A writer a holder-only reading may reach (I7): the `.eprfs/status` file writer, loopback-only
/// admin routes, loopback-scraped gauges, and a worker-local store in TypeScript.
///
/// **Concerns:** C1 (a holder's own readings stay the holder's), C7.
///
/// **Forcing incident:** interfaces spec I7 (2026-10-20): without a named writer type, any metrics
/// exporter could take a holder-only reading.
///
/// **Contract test:** `tests/probe.rs::holder_readings_reach_only_a_holder_sink`.
///
/// It takes a [`HolderReading`], which only [`Reading::route`] builds, so the sink encodes the
/// reading itself and cannot be handed a shared or party-scoped one.
pub trait HolderSink<T> {
    /// Write the reading.
    fn write_holder(&mut self, reading: &HolderReading<T>);
}

/// A writer readers at some reach read through: a public gauge, a metrics exporter, a served status
/// route. The sink declares how open its readers are; [`SharedReading::deliver`] compares that with
/// the reading's reach and hands the reading over only if every reader is inside it.
///
/// **Concerns:** C1, C7 (only a reading disclosed at a reach at least as open as the sink's readers
/// can be handed to it).
///
/// **Forcing incident:** fix round 1 of C1 (2026-10-20), as [`Routed`]; Codex review 1, finding 3,
/// for the audience.
///
/// **Contract test:** `tests/probe.rs::holder_readings_reach_only_a_holder_sink` and
/// `tests/probe.rs::a_shared_reading_reaches_only_a_sink_inside_its_reach`. A holder reading cannot
/// be handed to it:
///
/// ```compile_fail
/// use seam_contracts::{HolderReading, SharedSink};
/// fn leak<T>(sink: &mut dyn SharedSink<T>, r: &HolderReading<T>) { sink.write_shared(r) }
/// ```
///
/// nor a shared one without the reach comparison:
///
/// ```compile_fail
/// use seam_contracts::{SharedReading, SharedSink};
/// fn skip<T>(sink: &mut dyn SharedSink<T>, r: &SharedReading<T>) { sink.write_shared(r) }
/// ```
///
/// while a shared one can be delivered:
///
/// ```
/// use seam_contracts::{ShareRefusal, SharedReading, SharedSink};
/// fn export<T>(sink: &mut dyn SharedSink<T>, r: &SharedReading<T>) -> Result<(), ShareRefusal> {
///     r.deliver(sink)
/// }
/// ```
pub trait SharedSink<T> {
    /// The openness ordinal (`Reach::openness()`) of the most open reader of this sink: a public
    /// gauge declares [`PUBLIC_OPENNESS`].
    fn audience_openness(&self) -> u8;
    /// Write a reading [`SharedReading::deliver`] has admitted to this sink.
    fn write_shared(&mut self, reading: &Admitted<'_, T>);
}

/// The holder's explicit choice to let a holder-only reading's aggregate numbers leave, e.g. into a
/// habit DELTA. It carries counts, never paths or keys.
///
/// **Concerns:** C1 (leaving is the holder's choice, named by `chosen_by`), C5 (counts are evidence
/// another party re-reads; they confer nothing).
///
/// **Forcing incident:** as [`HolderSink`] (interfaces spec I7): the slice-1 step (b) run has its
/// holder copy counts into the habit's DELTA by hand, and the copy needs a shape.
///
/// **Contract test:** `declassified_comes_only_from_a_holder_reading`.
///
/// The fields are private and the only constructor is [`HolderReading::declassify`], so a
/// `Declassified` exists only where a holder reading was in hand. A struct literal does not compile:
///
/// ```compile_fail
/// use seam_contracts::Declassified;
/// let d = Declassified { promise: String::new(), counts: vec![], chosen_by: String::new() };
/// ```
///
/// It serializes but does not deserialize: a received one is read through the wire schema, not
/// rebuilt as this type.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize),
    serde(rename_all = "camelCase")
)]
pub struct Declassified {
    promise: String,
    counts: Vec<(String, u64)>,
    chosen_by: String,
}

impl Declassified {
    /// The promise the counts come from.
    pub fn promise(&self) -> &str {
        &self.promise
    }

    /// The named counts.
    pub fn counts(&self) -> &[(String, u64)] {
        &self.counts
    }

    /// Who chose to let them leave.
    pub fn chosen_by(&self) -> &str {
        &self.chosen_by
    }
}

#[cfg(test)]
mod probe_tests {
    use super::*;
    use crate::reason::{assert_reason_labels_conformant, assert_reason_labels_stable};

    #[test]
    fn unreachable_reason_refuses_observed_absent() {
        for reason in AnswerReason::ALL {
            let made = UnreachableReason::new(*reason);
            if *reason == AnswerReason::ObservedAbsent {
                assert_eq!(made, Err(AnswerReason::ObservedAbsent));
            } else {
                assert_eq!(made.map(|r| r.reason()), Ok(*reason));
            }
        }
        assert!(UnreachableReason::try_from(AnswerReason::ObservedAbsent).is_err());
        assert_eq!(
            AnswerReason::from(UnreachableReason::new(AnswerReason::Timeout).unwrap()),
            AnswerReason::Timeout
        );
    }

    #[test]
    fn unreachable_constructor_refuses_observed_absent() {
        assert_eq!(
            ProbeAnswer::<u8>::unreachable(AnswerReason::ObservedAbsent),
            Err(AnswerReason::ObservedAbsent)
        );
        assert!(ProbeAnswer::<u8>::unreachable(AnswerReason::Refused).is_ok());
    }

    #[test]
    fn probe_answer_states_and_reasons_pair() {
        let present = ProbeAnswer::present(3u8);
        assert_eq!(present.state(), AnswerState::Present);
        assert_eq!(present.reason(), None);

        let absent = ProbeAnswer::<u8>::absent();
        assert_eq!(absent.state(), AnswerState::Absent);
        assert_eq!(absent.reason(), Some(AnswerReason::ObservedAbsent));

        for reason in AnswerReason::ALL {
            if let Ok(a) = ProbeAnswer::<u8>::unreachable(*reason) {
                assert_eq!(a.state(), AnswerState::Unreachable);
                let r = a.reason().expect("unreachable always carries a reason");
                assert_eq!(r, *reason);
                assert_eq!(r.state(), a.state(), "the reason belongs to the state");
            }
        }
    }

    #[test]
    fn reading_carries_every_part() {
        let r = Reading::new(
            "bytes-layer-reads-only-what-is-read@1".into(),
            "materialize-what-is-read".into(),
            "bafy-root".into(),
            ProbeAnswer::present(12u32),
            EvidenceStage::Fixture,
            Disclosure::Holder,
            "2026-10-20T00:00:00Z".into(),
        );
        assert_eq!(r.promise(), "bytes-layer-reads-only-what-is-read@1");
        assert_eq!(r.concern(), "materialize-what-is-read");
        assert_eq!(r.subject(), "bafy-root");
        assert_eq!(r.answer(), &ProbeAnswer::Present(12));
        assert_eq!(r.evidence().label(), "fixture");
        assert_eq!(r.disclosure(), &Disclosure::Holder);
        assert_eq!(r.observed_at(), "2026-10-20T00:00:00Z");
        assert!(r.is_holder_only());
        // A clone keeps the disclosure it was built with.
        assert_eq!(r.clone().disclosure(), &Disclosure::Holder);
    }

    #[test]
    fn unreachable_reason_labels_are_answer_reason_labels() {
        assert_reason_labels_conformant::<UnreachableReason>();
        let expected: Vec<AnswerReason> = AnswerReason::ALL
            .iter()
            .copied()
            .filter(|r| *r != AnswerReason::ObservedAbsent)
            .collect();
        let got: Vec<AnswerReason> = UnreachableReason::ALL.iter().map(|r| r.reason()).collect();
        assert_eq!(
            got, expected,
            "every transport reason, in AnswerReason's order"
        );
        for r in UnreachableReason::ALL {
            assert_eq!(r.label(), r.reason().label());
        }
        let ProbeAnswer::<u8>::Unreachable(r) =
            ProbeAnswer::unreachable(AnswerReason::Timeout).expect("timeout is a transport reason")
        else {
            panic!("unreachable");
        };
        assert_eq!(r.label(), "timeout");
    }

    #[test]
    fn evidence_stage_labels_are_stable() {
        assert_reason_labels_conformant::<EvidenceStage>();
        assert_reason_labels_stable::<EvidenceStage>(&["fixture", "real"]);
    }

    #[test]
    fn party_role_labels_are_stable() {
        assert_reason_labels_conformant::<PartyRole>();
        assert_reason_labels_stable::<PartyRole>(&[
            "carrier",
            "carried-for",
            "grantor",
            "custodian",
            "leaver",
            "holder",
            "settlement",
        ]);
    }

    #[test]
    fn disclosure_distinguishes_its_forms() {
        assert_ne!(
            Disclosure::Holder,
            Disclosure::Parties {
                roles: vec![PartyRole::Holder]
            }
        );
        assert_ne!(
            Disclosure::AtReach { openness: 1 },
            Disclosure::AtReach { openness: 2 }
        );
        assert_ne!(
            Disclosure::Parties {
                roles: vec![PartyRole::Carrier]
            },
            Disclosure::Parties {
                roles: vec![PartyRole::CarriedFor]
            }
        );
    }

    fn reading(disclosure: Disclosure) -> Reading<u32> {
        Reading::new(
            "bytes-layer-reads-only-what-is-read@1".into(),
            "materialize-what-is-read".into(),
            "bafy-root".into(),
            ProbeAnswer::present(12),
            EvidenceStage::Real,
            disclosure,
            "2026-10-20T00:00:00Z".into(),
        )
    }

    #[test]
    fn route_sorts_by_disclosure() {
        match reading(Disclosure::Holder).route() {
            Routed::Holder(h) => assert_eq!(h.reading().disclosure(), &Disclosure::Holder),
            other => panic!("a holder reading routed as {other:?}"),
        }
        let roles = vec![PartyRole::Carrier, PartyRole::CarriedFor];
        match reading(Disclosure::Parties {
            roles: roles.clone(),
        })
        .route()
        {
            Routed::Parties(p) => assert_eq!(p.roles(), roles.as_slice()),
            other => panic!("a party-scoped reading routed as {other:?}"),
        }
        // An empty role list is still party-scoped: it names nobody, so nobody else may read it.
        assert!(matches!(
            reading(Disclosure::Parties { roles: vec![] }).route(),
            Routed::Parties(_)
        ));
        match reading(Disclosure::AtReach { openness: 4 }).route() {
            Routed::Shared(s) => assert_eq!(s.openness(), 4),
            other => panic!("a reading at a reach routed as {other:?}"),
        }
    }

    /// A sink that records what it was handed, declaring `audience`.
    struct Gauge {
        audience: u8,
        written: Vec<u8>,
    }

    impl SharedSink<u32> for Gauge {
        fn audience_openness(&self) -> u8 {
            self.audience
        }
        fn write_shared(&mut self, reading: &Admitted<'_, u32>) {
            self.written.push(reading.reading().openness());
        }
    }

    fn shared(openness: u8) -> SharedReading<u32> {
        match reading(Disclosure::AtReach { openness }).route() {
            Routed::Shared(s) => s,
            other => panic!("an AtReach reading routed as {other:?}"),
        }
    }

    #[test]
    fn deliver_compares_the_reach_with_the_audience() {
        let mut public = Gauge {
            audience: PUBLIC_OPENNESS,
            written: vec![],
        };
        // Below public: refused, nothing written.
        for openness in OPENNESS_MIN..PUBLIC_OPENNESS {
            assert_eq!(
                shared(openness).deliver(&mut public),
                Err(ShareRefusal::AudienceWiderThanReach),
                "openness {openness} reached a public sink"
            );
        }
        assert!(public.written.is_empty());
        // At public and above: delivered.
        assert_eq!(shared(PUBLIC_OPENNESS).deliver(&mut public), Ok(()));
        assert_eq!(shared(OPENNESS_MAX).deliver(&mut public), Ok(()));
        assert_eq!(public.written, vec![PUBLIC_OPENNESS, OPENNESS_MAX]);

        // Out-of-range readings are refused whatever the audience.
        let mut widest = Gauge {
            audience: OPENNESS_MIN,
            written: vec![],
        };
        for openness in [0, OPENNESS_MAX + 1, u8::MAX] {
            assert_eq!(
                shared(openness).deliver(&mut widest),
                Err(ShareRefusal::ReachOutOfRange)
            );
        }
        assert!(widest.written.is_empty());

        // A sink whose audience is not a reach ordinal is handed nothing.
        for audience in [0, OPENNESS_MAX + 1] {
            let mut broken = Gauge {
                audience,
                written: vec![],
            };
            assert_eq!(
                shared(OPENNESS_MAX).deliver(&mut broken),
                Err(ShareRefusal::AudienceOutOfRange)
            );
            assert!(broken.written.is_empty());
        }

        // A sink read only inside a narrow reach takes a reading at that reach or wider.
        let mut trusted = Gauge {
            audience: 4,
            written: vec![],
        };
        assert_eq!(
            shared(3).deliver(&mut trusted),
            Err(ShareRefusal::AudienceWiderThanReach)
        );
        assert_eq!(shared(4).deliver(&mut trusted), Ok(()));
        assert_eq!(shared(6).deliver(&mut trusted), Ok(()));
        assert_eq!(trusted.written, vec![4, 6]);
    }

    #[test]
    fn share_refusal_labels_are_stable() {
        assert_reason_labels_conformant::<ShareRefusal>();
        assert_reason_labels_stable::<ShareRefusal>(&[
            "reach-out-of-range",
            "audience-out-of-range",
            "audience-wider-than-reach",
        ]);
    }

    #[test]
    fn declassified_comes_only_from_a_holder_reading() {
        let Routed::Holder(held) = reading(Disclosure::Holder).route() else {
            panic!("holder reading");
        };
        let d = held.declassify(vec![("nodesFetched".into(), 7)], "human:matthew".into());
        assert_eq!(d.promise(), "bytes-layer-reads-only-what-is-read@1");
        assert_eq!(d.counts(), &[("nodesFetched".to_string(), 7)]);
        assert_eq!(d.chosen_by(), "human:matthew");
    }

    #[cfg(feature = "serde")]
    #[test]
    fn wire_convenience_derives_exist() {
        fn both<T: serde::Serialize + for<'de> serde::Deserialize<'de>>() {}
        both::<ProbeAnswer<u32>>();
        both::<UnreachableReason>();
        both::<EvidenceStage>();
        both::<Disclosure>();
        both::<PartyRole>();
        fn ser<T: serde::Serialize>() {}
        // Serialize-only: deserializing either would be a constructor that skips the one the
        // type's invariant rests on (`Reading::new`, `HolderReading::declassify`).
        ser::<Reading<u32>>();
        ser::<Declassified>();
    }
}
