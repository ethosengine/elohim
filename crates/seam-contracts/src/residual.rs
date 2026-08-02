//! `ResidualWitness` — C14 (witnessed residual) as a capsule convention.
//!
//! Design: `genesis/docs/superpowers/plans/2026-08-02-seam-concern-contract-architecture-plan.md`
//! (canon row C14; Design surface 8 "The residual channel — metal to dashboard"; task P4.7).
//!
//! ## The guarantee
//!
//! > Every decision point's outcome set closes with a residual arm that is **witnessed, never
//! > dropped**: an exception that fits no class is captured with a context capsule sufficient for
//! > cross-node RCA (inputs digest, decision state, the clocks/lags/peer-states in play), carried
//! > as evidence, delivered to the developer-facing RCA report, and **dispositioned** — cure,
//! > declared variance, or a new concern class via the calibration ledger's miss-of-canon intake.
//!
//! ## The type specimen (why this organ exists at all)
//!
//! *Red Plenty*'s dirt-mover: a machine's brakes fail, the machine is destroyed, and the whole
//! planning system silently depended on that one machine. The plan had **no channel for the
//! unplanned event** — so reality did not re-enter the system as a *report*. It re-entered as
//! corrupted assumptions: every downstream projection kept computing, confidently, on a world
//! that no longer existed. The canon, the concern × seam matrix and the forecast govern the
//! *anticipated*. A distributed, diverse fleet also guarantees the **unanticipated** — lags
//! between EPR nodes, real-world interruptions, hydraulic failures upstream of every abstraction.
//! Those are **expected inputs, not anomalies**. This module is the channel they arrive through.
//!
//! ## Six stations, and where this type sits
//!
//! | Station | Mechanism | Where |
//! |---|---|---|
//! | **capture** | the capsule at the seam | [`ResidualWitness`] — *this module* |
//! | **carry** | the capsule travels as evidence, never authority | C5; see "Evidence, not authority" below |
//! | **collect** | the findings ledger | `.claude/data/runtime-findings.jsonl`, namespace `concern:c14-witnessed-residual` |
//! | **report** | developer-facing RCA surface, beside the forecast | `_lib/residual_channel.py::render_residual_panel` |
//! | **recover** | triage dispatch — **restored capability** | the `runtime-triage` agent |
//! | **learn** | disposition feeds the calibration ledger | `.claude/data/seam-calibration.jsonl` — miss-of-canon is the canon's only admission intake |
//!
//! The pipeline is **bound, not forked**: `runtime-findings.jsonl` + `runtime-triage` is the live
//! instance of exactly this shape for *known* exhaustion classes. A residual finding rides the
//! same ledger, the same fingerprint/reconcile core, and the same dispatch; only the `class`
//! namespace differs. See `.claude/scripts/_lib/residual_channel.py` for the intake half.
//!
//! ## Recovery is restored capability — Mishpat, never punishment
//!
//! The response to a deviation is to **restore the capability that failed**, not to penalize the
//! node that surfaced it. A fleet that treats residual reports as blame gets fewer reports, not
//! fewer residuals — and the reports it loses are exactly the novel ones, the only kind the canon
//! can grow from. Boundaries and negotiated consequences exist; punishment does not. This is the
//! same grace the protocol extends to people, applied to failure itself: peers witness and carry
//! each other's capsules so **no node RCAs its own crash alone**. The heal plane is already mutual
//! aid (fills, salvage, carried records, witness bootstrap); C5's verification is what makes that
//! good faith *safe to extend* — generosity without gullibility.
//!
//! ## Evidence, not authority (C5 applies to the capsule)
//!
//! A capsule is a **report**, never a command. A receiving node may act only on what it
//! re-derives for itself: the capsule's `inputs_digest` lets a receiver check it is reasoning
//! about the same inputs; the `context` entries are the *witness's* readings of clocks, lags and
//! peer states — a second node's readings will legitimately differ, and that difference is
//! usually the finding. A capsule confers no authority to move a head, evict a peer, or change a
//! declared state, and it never carries a decision another node must obey.
//!
//! ## Never a bare log line, never at a dropped level
//!
//! `486982bb8` (2026-07-25): a leg that *"fails 100% and says nothing"*, its only signal a
//! `debug!` the deployment dropped. The residual arm **existed** and was unwitnessed. A capsule
//! that is only logged is that defect with more fields — the contract is that it reaches the
//! ledger, and from the ledger the RCA surface. `.warn!`/`.error!` is the *floor* for the local
//! echo, not the delivery.
//!
//! ## Distributed causes arrive as stacks
//!
//! The 2026-07-12 five-defect convergence arc is why [`ContextKind`] names clocks, lags and peer
//! states specifically: *"when an outcome measure stays red through multiple correct fixes,
//! suspect a STACK, not a miss."* A capsule that carries only local state cannot resolve a
//! five-deep stack — the reader needs the *relative* picture (which clock, how far behind, what
//! the other peers were saying) at the instant the residual fired.

use crate::canon::{PolicyPin, PIN_C14_WITNESSED_RESIDUAL};
use crate::reason::ReasonLabel;

/// The class of context reading a capsule carries.
///
/// **Concerns:** C8 (observability-per-decision — a fieldless discriminant with a stable label,
/// so a residual's *shape* is countable even before anyone reads its text); C14.
///
/// **Forcing incident:** the 2026-07-12 five-defect convergence arc — five stacked concern classes
/// behind one red outcome measure. The classes that resolved it were all *relative*: which clock
/// was being compared, how far behind a projector was, what each peer's declared state said.
///
/// **Contract test:** `context_kind_is_conformant`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum ContextKind {
    /// A named clock and its reading. **Which** clock is the whole question (C2): a DHT link
    /// timestamp, an `occurredAt`, and a conductor-local `now` are three different facts that a
    /// single "timestamp" field has repeatedly merged.
    Clock,
    /// A lag or backlog measurement — projector lag, queue depth, gossip age. The distance
    /// between two clocks, named as such.
    Lag,
    /// Another peer's state as this witness observed it. A *reading*, not a truth: the receiver
    /// re-derives (C5).
    PeerState,
    /// A decision input, redacted or digested. The capsule carries the *shape* of what was
    /// decided on, never a payload it has no standing to republish.
    Input,
    /// Deployment/runtime facts the residual is only legible against — image digest, DNA hash,
    /// transport backend, arc factor.
    Env,
}

impl ReasonLabel for ContextKind {
    const ALL: &'static [Self] = &[
        ContextKind::Clock,
        ContextKind::Lag,
        ContextKind::PeerState,
        ContextKind::Input,
        ContextKind::Env,
    ];

    fn label(&self) -> &'static str {
        match self {
            ContextKind::Clock => "clock",
            ContextKind::Lag => "lag",
            ContextKind::PeerState => "peer_state",
            ContextKind::Input => "input",
            ContextKind::Env => "env",
        }
    }
}

/// One context reading: a kind, the thing it is about, and what was read.
///
/// **Concerns:** C14, C5 (a reading, carried as evidence).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct ContextEntry {
    /// What class of reading this is.
    pub kind: ContextKind,
    /// What it is about — the clock's name, the peer's id, the input's field.
    pub subject: String,
    /// The reading itself, rendered. Kept as text on purpose: a capsule crossing nodes must not
    /// require the receiver to share the witness's numeric types.
    pub value: String,
}

impl ContextEntry {
    /// Build one reading.
    pub fn new(kind: ContextKind, subject: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            kind,
            subject: subject.into(),
            value: value.into(),
        }
    }
}

/// What the decision point would have said, had any class fit.
///
/// **Concerns:** C14, C8, C4 (the two arms are two *provenances* of "unclassified", kept apart:
/// a label existed but no arm consumed it, versus no label existed at all).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "kind", content = "value"))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum DecisionState {
    /// A typed [`ReasonLabel`] existed and this is the label the point *would* have emitted —
    /// the residual is that no arm handled it, not that the outcome was nameless. Prefer this
    /// arm: it keeps the residual joinable to the point's existing counter vocabulary.
    WouldHaveEmitted(String),
    /// No label fits. This is the residual proper — the arm C14 exists for. The text is a short,
    /// human-authored classification attempt, not a stack trace and not a payload.
    Raw(String),
}

impl DecisionState {
    /// The label a residual is metered under. `raw` for the nameless arm; otherwise the label the
    /// point would have emitted, so a residual and its ordinary outcomes share one vocabulary.
    pub fn label(&self) -> &str {
        match self {
            DecisionState::WouldHaveEmitted(l) => l,
            DecisionState::Raw(_) => "raw",
        }
    }
}

/// A witnessed residual — the capsule C14 contracts for.
///
/// **Concerns:** C14 (witnessed residual — this *is* its contract); C5 (carried as evidence, never
/// authority); C8 (the capsule is metered by [`ResidualWitness::finding_provenance`], a
/// low-cardinality classifier, while the unbounded detail stays out of the label); C4 (an
/// unclassified outcome is a distinct answer, not an absence).
///
/// **Forcing incident:** `486982bb8` (2026-07-25) — a leg that fails 100% and says nothing, its
/// only signal a dropped `debug!`; and the 2026-07-12 five-defect arc, where five stacked classes
/// hid behind one red measure because no capsule carried the relative picture.
///
/// **Contract test:** `residual_tests` in this module; the ledger-intake half is proven by
/// `.claude/scripts/_lib/__tests__/residual_channel_test.py` (fixture capsule -> ledger entry ->
/// runtime-triage dispatch), which is deliberately in the *other* language so neither side can be
/// the mirror.
///
/// # The crate has no clock and no hasher
///
/// Both are deliberate — this is a side-effect-free plain-data crate (D1 IoC seam). The caller
/// supplies `observed_at` (RFC 3339) and `capsule_id`. For a **content-derived** capsule id — so
/// two peers witnessing the same residual mint the same id — hash
/// [`ResidualWitness::capsule_id_material`] with the substrate's canonical hasher and use the
/// entry-hash discipline: identity is the *content* address, never a provenance/anchor hash.
///
/// # Example
///
/// ```
/// use seam_contracts::residual::{ContextEntry, ContextKind, DecisionState, ResidualWitness};
///
/// let capsule = ResidualWitness::new(
///     "elohim-storage::head_adoption::decide_head_action",
///     "declared-divergence-no-arm",
///     "blake3:9f2c…",
///     DecisionState::Raw("both sides declared, neither carries a candidate".into()),
/// )
/// .with_context(ContextEntry::new(ContextKind::Clock, "dht_link_ts", "1785…"))
/// .with_context(ContextEntry::new(ContextKind::Lag, "projector", "42s"))
/// .with_context(ContextEntry::new(ContextKind::PeerState, "adam", "declared=uhC0k…"));
///
/// assert_eq!(capsule.finding_class(), "concern:c14-witnessed-residual");
/// assert!(capsule.finding_provenance().starts_with("residual:"));
/// assert!(capsule.finding_line().len() <= 300);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct ResidualWitness {
    /// The registered decision point that produced this residual — the same `id` its crate's
    /// `seam-registry.yaml` registers, so a residual joins straight to the concern × seam matrix
    /// cell it landed in.
    pub seam: String,
    /// A short, **low-cardinality, stable** classifier for this residual's shape, authored at the
    /// seam (`"declared-divergence-no-arm"`, `"codec-unknown-variant"`). It is the fingerprint
    /// axis: two occurrences of the same shape must share it, and it must not embed ids, counts,
    /// or timestamps. Cardinality discipline is the whole reason it is separate from
    /// [`Self::decision_state`].
    pub residual_kind: String,
    /// A digest of the decision's inputs — enough for a receiver to check it is reasoning about
    /// the same inputs, never the inputs themselves. Opaque to this crate.
    pub inputs_digest: String,
    /// What the point would have said, or that nothing fit.
    pub decision_state: DecisionState,
    /// The relative picture: clocks, lags, peer states, inputs, env. Distributed causes arrive as
    /// stacks; this is what lets a reader unstack them.
    pub context: Vec<ContextEntry>,
    /// Caller-supplied capsule identity. Empty means "not yet minted" — see
    /// [`Self::capsule_id_material`].
    pub capsule_id: String,
    /// Caller-supplied RFC 3339 instant. Empty means the witness declined to claim one; a capsule
    /// with no time is still a capsule, and inventing one here would be a fourth clock.
    pub observed_at: String,
}

impl ResidualWitness {
    /// Capture a residual at a seam.
    pub fn new(
        seam: impl Into<String>,
        residual_kind: impl Into<String>,
        inputs_digest: impl Into<String>,
        decision_state: DecisionState,
    ) -> Self {
        Self {
            seam: seam.into(),
            residual_kind: residual_kind.into(),
            inputs_digest: inputs_digest.into(),
            decision_state,
            context: Vec::new(),
            capsule_id: String::new(),
            observed_at: String::new(),
        }
    }

    /// Add one context reading.
    #[must_use]
    pub fn with_context(mut self, entry: ContextEntry) -> Self {
        self.context.push(entry);
        self
    }

    /// Set the caller-minted capsule id (see [`Self::capsule_id_material`]).
    #[must_use]
    pub fn with_capsule_id(mut self, id: impl Into<String>) -> Self {
        self.capsule_id = id.into();
        self
    }

    /// Set the caller-supplied RFC 3339 observation instant.
    #[must_use]
    pub fn with_observed_at(mut self, ts: impl Into<String>) -> Self {
        self.observed_at = ts.into();
        self
    }

    /// The canon pin this capsule answers to — C14, live version.
    pub const fn concern_pin(&self) -> PolicyPin {
        PIN_C14_WITNESSED_RESIDUAL
    }

    /// The findings-ledger **namespace** for residuals: `concern:c14-witnessed-residual`.
    ///
    /// Derived from the C14 pin's `id` rather than written out, so the namespace and the canon row
    /// cannot drift apart — the canon row's own `findings_namespace` field states the same string,
    /// and that is the one this composes from.
    ///
    /// **Concerns:** C7 (advertise/serve symmetry — the ledger class a runtime emits is the class
    /// the canon declares).
    pub fn finding_class(&self) -> String {
        let mut s = String::from("concern:");
        s.push_str(PIN_C14_WITNESSED_RESIDUAL.id);
        s
    }

    /// The ledger **provenance** — the fingerprint axis, and the whole cardinality budget.
    ///
    /// `residual:<seam>:<residual_kind>`. Deliberately excludes ids, counts, timestamps and the
    /// decision text: two occurrences of one residual shape must fingerprint identically so the
    /// ledger's reconcile bumps `seen` instead of minting a second finding. (The intake also
    /// normalizes digits out, but a provenance that *needs* normalization is a provenance chosen
    /// badly.)
    ///
    /// **Concerns:** C8 (no unbounded-cardinality label); C6b (idempotent effect — replaying the
    /// same residual against a ledger that already holds it mints nothing).
    pub fn finding_provenance(&self) -> String {
        let mut s = String::from("residual:");
        s.push_str(&self.seam);
        s.push(':');
        s.push_str(&self.residual_kind);
        s
    }

    /// A single ≤300-char ledger line: the shape, the state, and the *census* of context by kind.
    ///
    /// The context census (`clock=2 lag=1 peer_state=3`) is here on purpose — C8's "every meter
    /// names its semantics". A reader can tell at the ledger, without opening the capsule, whether
    /// this residual was witnessed with enough relative picture to unstack a distributed cause, or
    /// whether it arrived blind.
    pub fn finding_line(&self) -> String {
        let mut census = String::new();
        for kind in ContextKind::ALL {
            let n = self.context.iter().filter(|c| c.kind == *kind).count();
            if n > 0 {
                if !census.is_empty() {
                    census.push(' ');
                }
                census.push_str(kind.label());
                census.push('=');
                census.push_str(&n.to_string());
            }
        }
        if census.is_empty() {
            census.push_str("none");
        }
        let detail = match &self.decision_state {
            DecisionState::WouldHaveEmitted(l) => {
                let mut s = String::from("would_have_emitted=");
                s.push_str(l);
                s
            }
            DecisionState::Raw(t) => {
                let mut s = String::from("raw=");
                s.push_str(t);
                s
            }
        };
        let mut line = String::new();
        line.push_str(&self.seam);
        line.push_str(" residual [");
        line.push_str(&self.residual_kind);
        line.push_str("] ");
        line.push_str(&detail);
        line.push_str(" | inputs=");
        line.push_str(&self.inputs_digest);
        line.push_str(" | ctx: ");
        line.push_str(&census);
        truncate_chars(line, 300)
    }

    /// The exact bytes to hash for a **content-derived** capsule id.
    ///
    /// Two peers witnessing the same residual must mint the same id, so the material is
    /// canonical: field order fixed here, `observed_at` and `capsule_id` deliberately EXCLUDED
    /// (they are per-witness facts — including them would make every witness's capsule a
    /// different object and destroy the join), context entries in the order they were witnessed.
    ///
    /// The crate does not hash: it has no hasher and may not acquire one (leaf boundary). Feed
    /// this to the substrate's canonical hasher and apply the entry-hash discipline — identity is
    /// the *content* address, never a provenance/anchor hash.
    ///
    /// **Concerns:** C5 (a receiver re-derives the id from the same material and so can tell a
    /// re-witness from a new event); C9 (identity is content-derived, stable across whichever peer
    /// carries it).
    pub fn capsule_id_material(&self) -> String {
        let mut s = String::new();
        s.push_str("seam=");
        s.push_str(&self.seam);
        s.push_str("\nkind=");
        s.push_str(&self.residual_kind);
        s.push_str("\ninputs=");
        s.push_str(&self.inputs_digest);
        s.push_str("\nstate=");
        s.push_str(self.decision_state.label());
        if let DecisionState::Raw(t) = &self.decision_state {
            s.push('/');
            s.push_str(t);
        }
        for c in &self.context {
            s.push_str("\nctx=");
            s.push_str(c.kind.label());
            s.push('/');
            s.push_str(&c.subject);
            s.push('/');
            s.push_str(&c.value);
        }
        s
    }

    /// Is this capsule carrying enough relative picture for cross-node RCA?
    ///
    /// The C14 statement names *clocks, lags and peer-states* because a capsule of purely local
    /// state cannot resolve a stacked distributed cause. This returns `false` when the capsule
    /// carries none of those three kinds — a **warning about the witness, not about the fleet**,
    /// and never a reason to drop the capsule: a thin residual is still infinitely better than a
    /// dropped one, and the intake files it either way.
    ///
    /// **Concerns:** C14, C4 (a thin capsule is a *reported* residual with weak evidence, not an
    /// absent one).
    pub fn carries_relative_context(&self) -> bool {
        self.context.iter().any(|c| {
            matches!(
                c.kind,
                ContextKind::Clock | ContextKind::Lag | ContextKind::PeerState
            )
        })
    }
}

/// Truncate on a char boundary, marking that truncation happened.
fn truncate_chars(s: String, max: usize) -> String {
    if s.chars().count() <= max {
        return s;
    }
    let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

#[cfg(test)]
mod residual_tests {
    use super::*;
    use crate::reason::assert_reason_labels_conformant;

    fn capsule() -> ResidualWitness {
        ResidualWitness::new(
            "elohim-storage::head_adoption::decide_head_action",
            "declared-divergence-no-arm",
            "blake3:9f2c",
            DecisionState::Raw("both sides declared, neither carries a candidate".into()),
        )
        .with_context(ContextEntry::new(ContextKind::Clock, "dht_link_ts", "1785"))
        .with_context(ContextEntry::new(ContextKind::Lag, "projector", "42s"))
        .with_context(ContextEntry::new(
            ContextKind::PeerState,
            "adam",
            "declared=uhC0k",
        ))
    }

    #[test]
    fn context_kind_is_conformant() {
        assert_reason_labels_conformant::<ContextKind>();
    }

    #[test]
    fn finding_class_is_the_canon_namespace() {
        assert_eq!(capsule().finding_class(), "concern:c14-witnessed-residual");
        assert_eq!(capsule().concern_pin().concern, "C14");
    }

    #[test]
    fn provenance_is_the_fingerprint_axis_and_carries_no_churn() {
        let a = capsule();
        let mut b = capsule();
        b.observed_at = "2026-08-02T12:00:00Z".into();
        b.capsule_id = "abc".into();
        b.inputs_digest = "blake3:different".into();
        assert_eq!(
            a.finding_provenance(),
            b.finding_provenance(),
            "provenance must be invariant to per-occurrence detail, or the ledger mints a new \
             finding for every recurrence instead of bumping `seen`"
        );
        assert_eq!(
            a.finding_provenance(),
            "residual:elohim-storage::head_adoption::decide_head_action:declared-divergence-no-arm"
        );
        let other = ResidualWitness::new(
            "same-seam",
            "other-shape",
            "d",
            DecisionState::Raw("x".into()),
        );
        let same_seam =
            ResidualWitness::new("same-seam", "shape", "d", DecisionState::Raw("x".into()));
        assert_ne!(other.finding_provenance(), same_seam.finding_provenance());
    }

    #[test]
    fn finding_line_carries_the_context_census_and_stays_bounded() {
        let line = capsule().finding_line();
        assert!(line.contains("clock=1"), "{line}");
        assert!(line.contains("lag=1"), "{line}");
        assert!(line.contains("peer_state=1"), "{line}");
        assert!(line.contains("raw=both sides declared"), "{line}");
        assert!(line.chars().count() <= 300);

        let fat = ResidualWitness::new(
            "seam",
            "kind",
            "digest",
            DecisionState::Raw("x".repeat(1000)),
        );
        let fat_line = fat.finding_line();
        assert_eq!(fat_line.chars().count(), 300);
        assert!(fat_line.ends_with('…'));
    }

    #[test]
    fn a_capsule_with_no_context_says_so_rather_than_omitting_the_field() {
        let bare = ResidualWitness::new("seam", "kind", "d", DecisionState::Raw("x".into()));
        assert!(bare.finding_line().contains("ctx: none"));
        assert!(!bare.carries_relative_context());
        assert!(capsule().carries_relative_context());
        // Env/Input alone is local state: legible, but not enough to unstack a distributed cause.
        let local_only = ResidualWitness::new("seam", "kind", "d", DecisionState::Raw("x".into()))
            .with_context(ContextEntry::new(ContextKind::Env, "dna", "uhC0k"));
        assert!(!local_only.carries_relative_context());
    }

    #[test]
    fn capsule_id_material_is_witness_invariant() {
        let a = capsule()
            .with_observed_at("2026-08-02T12:00:00Z")
            .with_capsule_id("witness-a");
        let b = capsule()
            .with_observed_at("2026-08-02T12:00:09Z")
            .with_capsule_id("witness-b");
        assert_eq!(
            a.capsule_id_material(),
            b.capsule_id_material(),
            "two peers witnessing ONE residual must derive ONE content id — otherwise the fleet \
             cannot tell a carried re-witness from a second event, and 'no node RCAs its own \
             crash alone' becomes 'every node files its own finding'"
        );
        let different = capsule().with_context(ContextEntry::new(ContextKind::Env, "dna", "x"));
        assert_ne!(a.capsule_id_material(), different.capsule_id_material());
    }

    #[test]
    fn decision_state_keeps_its_two_provenances_apart() {
        let labeled = DecisionState::WouldHaveEmitted("stamp_not_newer".into());
        assert_eq!(labeled.label(), "stamp_not_newer");
        assert_eq!(DecisionState::Raw("anything".into()).label(), "raw");
        assert_ne!(labeled, DecisionState::Raw("stamp_not_newer".into()));
    }
}
