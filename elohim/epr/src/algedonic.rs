//! Algedonic feedback signals — pain as a **viability** signal, orthogonal to judgment valence.
//!
//! See `genesis/docs/superpowers/specs/2026-08-10-algedonic-feedback-signal-design.md`
//! (§4b "Design canon" is the do/do-not list every construct here anchors).
//!
//! # The loop
//!
//! A **producer** (the `declarer` — a self-reporting node, storage validator, or the operator's
//! elohim-counsel) measures a **stock** (`evidence.stock`) against a **limit**
//! (`evidence.limit`, declared by the bounding commitment/manifest at `evidence.bound_ref`) and
//! emits one signal to the **consumer** — the `target`, the CID of the promise threatened. Two
//! kinds close the loop: [`AlgedonicKind::Approach`] fires at the band edge (`threshold_pct` of
//! the limit — the signal *before* the line) and [`AlgedonicKind::Breach`] fires once the bound
//! is crossed. A node in pain accuses no one; it reports its own sensed state against a limit it
//! declared, which is why `standing_impact` is fixed at [`STANDING_IMPACT`] (`advisory`) and why
//! no valence field exists anywhere in this module.
//!
//! # Invariants, and where each one lives
//!
//! - **No valence.** There is no agree/disagree/position/vote field — unrepresentable, not
//!   validated. Algedonic is a third feedback type, not an overload of judgment valence.
//! - **Evidence is mandatory.** `stock`, `limit` and `bound_ref` are non-optional fields of
//!   [`AlgedonicEvidence`]: a signal without them cannot be constructed, and refuses to
//!   deserialize.
//! - **`Approach` requires `threshold_pct`; `Breach` does not.** Carried by the evidence enum's
//!   own variants, so the two shapes cannot be confused — and the kind is *derived from* the
//!   evidence ([`AlgedonicEvidence::kind`]), so a signal whose `signal_kind` disagrees with its
//!   evidence is unrepresentable in [`AlgedonicSignal`] and refused at the wire boundary.
//! - **One open signal per `(declarer, target, kind)`.** [`open_signal_key`] is that key; pain is
//!   a held state, not a stream. [`should_emit`] is the **latch** that enforces it — and a latch
//!   is not hysteresis, which this line previously claimed it was. It has a SET condition
//!   (`crossed()`) and **no reset**: there is no `should_clear`, no second threshold, and no time
//!   dimension anywhere in this module. Whether pain ever ends is therefore a property of the
//!   caller, not of this type. Under a caller that closes an open signal on non-crossing, a stock
//!   oscillating at the band edge re-fires every cycle — precisely the failure hysteresis exists
//!   to prevent. Naming it correctly is load-bearing: Beer's Fourth Principle of Organization
//!   (cyclic maintenance "without hiatus or lags") is the one this module does not yet satisfy,
//!   and calling the latch hysteresis is what let that read as done.
//! - **Evidence is minted, never assembled.** The limit a signal reports is a bound on a
//!   *promise*, so evidence is derived by the REA fold that owns the bound
//!   (`elohim_epr_rea::fold::fulfillment`, whose `bound_ref` is the bounding commitment's CID) —
//!   the intended minting path. That is what keeps the extra-key edge theoretical today: the
//!   schemas permit unlisted evidence keys, so a stray `threshold_pct` on a breach payload would
//!   deserialize as `Approach` and be refused by the coherence check below; nothing on the minted
//!   path can build one, because the variant is chosen from the crossing.
//!
//! # Wire contract
//!
//! `elohim/sdk/schemas/v1/feedback-signals/algedonic-{approach,breach}.schema.json`. Those
//! schemas are the **contract only** — the source of truth is the DHT `FeedbackSignal` entry
//! (elohim zome, `content_store_integrity/src/feedback_signal.rs`), which is designed to be
//! extended by `signal_kind` vocabulary rather than by a new entry type. Field names here are
//! **snake_case**, matching those schemas exactly; that is deliberately unlike the crate's
//! camelCase ts-rs types, and is why nothing in this module derives `TS`: the schemas already
//! own the projection.
//!
//! **That extension is INTENDED, not shipped** (measured 2026-08-12). `SIGNAL_KINDS` is
//! `[squelch, correction, retraction, quarantine, vouch, forget-request]` — no algedonic kind is
//! in it, and no file under `elohim/holochain/` mentions algedonic at all. Read the paragraph
//! above as the destination; today an algedonic signal has no DHT home, which is the concrete
//! form of "the control plane emits nothing." Two things follow, and the second is a live hazard:
//!
//! 1. Adding the vocabulary is a one-line whitelist change plus its validation, NOT a migration
//!    of a shipped wire shape. Any ranking that prices it as the latter is inverted.
//! 2. `properties.evidence` sets **`additionalProperties: true`** in BOTH schemas (the `false`
//!    sits on the envelope). Unlisted evidence keys are accepted, so a payload carrying a
//!    not-yet-modelled discriminator — a floor `sense`, say — validates green today and is then
//!    read as a ceiling by every consumer calling [`AlgedonicEvidence::crossed`], which is
//!    unconditionally `stock >= band_edge`. Silent acceptance of a meaning nobody declared is the
//!    C4 shape this family exists to make unrepresentable. Until `Sense` lives in the type, the
//!    only thing preventing it is that `elohim_epr_rea::fold::level_evidence` withholds floor
//!    evidence at the mint.

use crate::error::{EprError, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// The fixed standing impact of every algedonic signal.
///
/// Pain never debits standing by itself — consequences flow only through the witnessed
/// governance path (spec §3). It is a **constant, not a field**: the wire schemas declare
/// `additionalProperties: false` and do not list `standing_impact`, so emitting it would fail
/// validation. Consumers read it from here.
pub const STANDING_IMPACT: &str = "advisory";

// ---------------------------------------------------------------------------
// Vocabulary
// ---------------------------------------------------------------------------

/// The algedonic signal kinds: the two pain kinds and the latch's `Recovered` reset.
///
/// Wire form is the full `signal_kind` const from the schemas (`algedonic-approach` /
/// `algedonic-breach`) — these are members of the DHT `FeedbackSignal` kind whitelist, not a
/// crate-local shorthand.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AlgedonicKind {
    /// Band edge crossed — the stock has passed `threshold_pct` of the limit but the bound
    /// still holds. The signal before the line.
    #[serde(rename = "algedonic-approach")]
    Approach,
    /// The bound has been crossed, or a self-heal mechanism is exhausted.
    #[serde(rename = "algedonic-breach")]
    Breach,
    /// The latch reset: the stock stayed on the safe side of the reset line for the dwell. A
    /// transition of the persisted latch, never an incident state; it carries
    /// [`RecoveryEvidence`] in a [`RecoveredSignal`], not an [`AlgedonicEvidence`].
    #[serde(rename = "algedonic-recovered")]
    Recovered,
}

impl AlgedonicKind {
    /// The exact `signal_kind` wire string.
    pub const fn as_str(self) -> &'static str {
        match self {
            AlgedonicKind::Approach => "algedonic-approach",
            AlgedonicKind::Breach => "algedonic-breach",
            AlgedonicKind::Recovered => "algedonic-recovered",
        }
    }
}

/// How loudly the producer is reporting. Advisory only — severity never touches standing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    /// The schema default.
    #[default]
    Warn,
    Critical,
}

// ---------------------------------------------------------------------------
// Evidence — the producer/stock/limit half of the loop
// ---------------------------------------------------------------------------

/// The measured evidence a signal carries. **Every field is non-optional**: evidence-free pain is
/// unrepresentable, not merely rejected.
///
/// The variants ARE the per-kind requirement: `threshold_pct` exists on `Approach` and nowhere
/// else, so "Breach must not require a band edge" needs no runtime rule. Serialization is
/// untagged — the wire evidence object carries no discriminator, because the kind is already at
/// the top level as `signal_kind`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AlgedonicEvidence {
    /// Band-edge evidence. Ordered first so an object carrying `threshold_pct` never falls
    /// through to `Breach`.
    Approach {
        /// The measured stock level at time of the signal.
        stock: f64,
        /// The bound being approached.
        limit: f64,
        /// CID of the bounding commitment/manifest that declared the limit.
        bound_ref: String,
        /// The band edge as a **percentage of the limit** (`85.0` means 85%), per the
        /// limit-governor stub's elevate step.
        threshold_pct: f64,
    },
    /// Bound-crossed evidence — no band edge, because the line itself is the edge.
    Breach {
        /// The measured stock level at time of breach.
        stock: f64,
        /// The bound that was breached.
        limit: f64,
        /// CID of the bounding commitment/manifest whose limit was breached.
        bound_ref: String,
    },
}

impl AlgedonicEvidence {
    /// The kind this evidence *is*. The signal's `signal_kind` is derived from here, never
    /// stored alongside it.
    pub const fn kind(&self) -> AlgedonicKind {
        match self {
            AlgedonicEvidence::Approach { .. } => AlgedonicKind::Approach,
            AlgedonicEvidence::Breach { .. } => AlgedonicKind::Breach,
        }
    }

    /// The measured quantity.
    pub const fn stock(&self) -> f64 {
        match self {
            AlgedonicEvidence::Approach { stock, .. } | AlgedonicEvidence::Breach { stock, .. } => {
                *stock
            }
        }
    }

    /// The declared bound.
    pub const fn limit(&self) -> f64 {
        match self {
            AlgedonicEvidence::Approach { limit, .. } | AlgedonicEvidence::Breach { limit, .. } => {
                *limit
            }
        }
    }

    /// CID of the commitment/manifest that declared the bound.
    pub fn bound_ref(&self) -> &str {
        match self {
            AlgedonicEvidence::Approach { bound_ref, .. }
            | AlgedonicEvidence::Breach { bound_ref, .. } => bound_ref,
        }
    }

    /// The stock level at which this kind's line is crossed: `limit × threshold_pct / 100` for an
    /// approach, the `limit` itself for a breach. Ceiling-bound semantics at v1 (a floor bound —
    /// pain for a stock falling *below* a line — is not modeled).
    pub fn band_edge(&self) -> f64 {
        match self {
            AlgedonicEvidence::Approach {
                limit,
                threshold_pct,
                ..
            } => limit * threshold_pct / 100.0,
            AlgedonicEvidence::Breach { limit, .. } => *limit,
        }
    }

    /// Whether the measured stock has actually reached this kind's line.
    pub fn crossed(&self) -> bool {
        self.stock() >= self.band_edge()
    }
}

// ---------------------------------------------------------------------------
// The signal
// ---------------------------------------------------------------------------

/// One algedonic signal: a producer reporting a stock against a limit, to the promise it threatens.
///
/// There is **no `signal_kind` field** — the kind is derived from [`Self::evidence`], so a signal
/// that claims `algedonic-approach` while carrying breach evidence cannot exist. The wire form
/// carries `signal_kind` (the schemas require it); the coherence check runs once, at the
/// deserialization boundary, where untrusted bytes arrive.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(into = "AlgedonicWire", try_from = "AlgedonicWire")]
pub struct AlgedonicSignal {
    /// Consumer — CID of the promise (Commitment; `cid = entry_hash`) threatened.
    pub target: String,
    /// Producer — CID or agent identifier of the node that declared the pain.
    pub declarer: String,
    /// Stock, limit and bound — mandatory by construction.
    pub evidence: AlgedonicEvidence,
    /// Advisory loudness; never a standing consequence.
    pub severity: Severity,
    /// When the producer signed the report.
    pub signed_at: DateTime<Utc>,
}

impl AlgedonicSignal {
    /// The kind, derived from the evidence.
    pub const fn kind(&self) -> AlgedonicKind {
        self.evidence.kind()
    }

    /// Always [`STANDING_IMPACT`] — pain informs, it does not withdraw standing.
    pub const fn standing_impact(&self) -> &'static str {
        STANDING_IMPACT
    }
}

/// The wire projection: `signal_kind` restored to the top level for the schemas' benefit.
///
/// Private on purpose — it is the only shape in which kind and evidence can disagree, and it
/// exists for exactly as long as one `TryFrom` takes to reject that.
#[derive(Serialize, Deserialize)]
struct AlgedonicWire {
    signal_kind: AlgedonicKind,
    target: String,
    declarer: String,
    evidence: AlgedonicEvidence,
    /// Optional on the wire — the schemas omit it from `required` and declare
    /// `"default": "warn"`. Without `default` here, serde would reject a
    /// contract-valid document that simply didn't say how loud the pain is.
    #[serde(default)]
    severity: Severity,
    signed_at: DateTime<Utc>,
}

impl From<AlgedonicSignal> for AlgedonicWire {
    fn from(s: AlgedonicSignal) -> Self {
        AlgedonicWire {
            signal_kind: s.evidence.kind(),
            target: s.target,
            declarer: s.declarer,
            evidence: s.evidence,
            severity: s.severity,
            signed_at: s.signed_at,
        }
    }
}

impl TryFrom<AlgedonicWire> for AlgedonicSignal {
    type Error = EprError;

    fn try_from(w: AlgedonicWire) -> Result<Self> {
        if w.signal_kind != w.evidence.kind() {
            return Err(EprError::Algedonic(format!(
                "signal_kind {} disagrees with its evidence ({})",
                w.signal_kind.as_str(),
                w.evidence.kind().as_str()
            )));
        }
        Ok(AlgedonicSignal {
            target: w.target,
            declarer: w.declarer,
            evidence: w.evidence,
            severity: w.severity,
            signed_at: w.signed_at,
        })
    }
}

// ---------------------------------------------------------------------------
// Dedupe key + emission bounds
// ---------------------------------------------------------------------------

/// The dedupe key: at most ONE open signal exists per `(declarer, target, kind)`.
///
/// An internal index key, never an address — see the spec's head-plane honesty rule. Because
/// `kind` participates, an open `Approach` never suppresses the `Breach` that escalates it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OpenSignalKey {
    pub declarer: String,
    pub target: String,
    pub kind: AlgedonicKind,
}

/// The dedupe key of a signal. Two signals differing only in their evidence collide here —
/// that is the point: re-measuring the same pain is not new pain.
pub fn open_signal_key(signal: &AlgedonicSignal) -> OpenSignalKey {
    OpenSignalKey {
        declarer: signal.declarer.clone(),
        target: signal.target.clone(),
        kind: signal.kind(),
    }
}

/// Structural validation — the refusals that the type system cannot make unrepresentable.
///
/// Mirrors [`crate::validation::validate_coupling`]: a positive `Ok(())` or a refusal naming the
/// requirement. Everything checked here is a `minLength`/finiteness obligation of the wire
/// schemas; the *shape* obligations (evidence present, `threshold_pct` on approaches only, no
/// valence) are already unrepresentable and are deliberately absent from this function.
pub fn validate(signal: &AlgedonicSignal) -> Result<()> {
    let refuse = |what: &str| -> Result<()> { Err(EprError::Algedonic(what.to_string())) };

    if signal.target.is_empty() {
        return refuse("target (the threatened promise) must be non-empty");
    }
    if signal.declarer.is_empty() {
        return refuse("declarer (the producer) must be non-empty");
    }
    if signal.evidence.bound_ref().is_empty() {
        return refuse("evidence.bound_ref (the bound's declaring CID) must be non-empty");
    }
    if !signal.evidence.stock().is_finite() {
        return refuse("evidence.stock must be a finite measurement");
    }
    if !signal.evidence.limit().is_finite() {
        return refuse("evidence.limit must be a finite bound");
    }
    if let AlgedonicEvidence::Approach { threshold_pct, .. } = signal.evidence {
        if !threshold_pct.is_finite() || threshold_pct <= 0.0 || threshold_pct > 100.0 {
            return refuse("evidence.threshold_pct must be a percentage in (0, 100]");
        }
    }
    Ok(())
}

/// **Ceiling only.** A Floor consumer calls [`should_emit_sensed`]; this function would read
/// Floor evidence on the wrong side of the limit.
///
/// The latch predicate — the answer to "may this signal be emitted now?". (A latch, not
/// hysteresis: see the module header — there is a SET condition and no reset.)
///
/// Two rules, both from the spec's emission-bounds clause:
///
/// 1. **Inside the band, silence.** A signal whose stock has not reached its own line
///    ([`AlgedonicEvidence::crossed`]) is not pain yet. Nothing to report.
/// 2. **One open signal per key.** While a signal with the same [`OpenSignalKey`] is open, no
///    re-fire — pain is a held state, not a stream, and re-tolling it is the anti-pattern this
///    predicate exists to prevent. An open `Approach` does NOT suppress a `Breach` (different
///    kind ⇒ different key ⇒ the escalation gets through).
///
/// `prev` is a currently-open signal, if the caller holds one — it need not share `new`'s
/// [`OpenSignalKey`]; rule 2 above is exactly the comparison that decides whether it does.
pub fn should_emit(prev: Option<&AlgedonicSignal>, new: &AlgedonicSignal) -> bool {
    if !new.evidence.crossed() {
        return false;
    }
    match prev {
        Some(open) => open_signal_key(open) != open_signal_key(new),
        None => true,
    }
}

// ---------------------------------------------------------------------------
// Recovery — the latch's reset half (dialogue-stasis slice 4)
// ---------------------------------------------------------------------------
//
// Spec: `genesis/docs/superpowers/specs/2026-10-30-dialogue-stasis-loop-design.md`
// ("AlgedonicSignal and its latch"). Everything below is ADDITIVE: [`should_emit`] and the
// caller-held memory model above are untouched. `Recovered` is a transition of the latch, never
// an incident state; incident closure has no dwell and lives with the consumer.

/// Which side of the limit is the safe side.
///
/// A local mirror of `elohim_epr_rea::Sense`: `elohim-epr-rea` depends on this crate, so this
/// crate cannot name that type without a dependency cycle. The wire words are identical
/// (`ceiling` / `floor`) and the consumer converts at the call site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LatchSense {
    /// Safe below the limit; pain as the stock rises.
    Ceiling,
    /// Safe above the limit; pain as the stock falls.
    Floor,
}

/// Entry, reset and dwell for a persisted latch.
///
/// `threshold_pct` is in the same unit the wire schemas and `elohim_epr_rea::Bound` use, so a
/// Floor signal stays schema-valid (`threshold_pct` in `(0, 100]`) with the existing
/// approach/breach schemas unchanged:
///
/// - Ceiling: Approach at stock `>= limit × threshold_pct / 100`; Breach at stock `>= limit`.
/// - Floor: `threshold_pct` is the **margin**, mirrored around the limit as `Bound` does:
///   Approach at stock `<= limit × (200 − threshold_pct) / 100`; Breach at stock `<= limit`.
///
/// The reset line is `limit × reset_pct / 100` for both senses and the safe comparison is
/// inclusive (Ceiling `stock <=` line, Floor `stock >=` line), so `reset_pct = 0` is expressible
/// for a stock that is exactly zero when healthy (a dialogue stall's duration).
/// Ceiling needs `0 <= reset_pct < threshold_pct <= 100`; Floor needs `0 < threshold_pct <= 100`
/// and `reset_pct` above both `threshold_pct` and the Approach edge's percentage, so the safe
/// side is strictly beyond the line that raised the Approach.
///
/// Floor Approach/Breach signals use the wire's ceiling-shaped evidence; [`should_emit`] and
/// [`AlgedonicEvidence::crossed`] judge it as a ceiling, so a Floor consumer takes its
/// transitions from [`AlgedonicLatch`], or calls [`should_emit_sensed`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Hysteresis {
    /// Entry margin, percent of the limit, in `(0, 100]`.
    pub threshold_pct: f64,
    /// Reset line, percent of the limit.
    pub reset_pct: f64,
    /// Seconds the stock must stay on the safe side before the latch resets.
    pub dwell_s: u64,
}

impl Hysteresis {
    /// The Approach edge as a percentage of the limit, for the given sense.
    pub fn edge_pct(&self, sense: LatchSense) -> f64 {
        match sense {
            LatchSense::Ceiling => self.threshold_pct,
            LatchSense::Floor => 200.0 - self.threshold_pct,
        }
    }

    /// Refuse a declaration whose reset line is on the wrong side of its entry line.
    pub fn validate(&self, sense: LatchSense) -> Result<()> {
        let bad = |what: &str| Err(EprError::Algedonic(what.to_string()));
        if !self.threshold_pct.is_finite() || !self.reset_pct.is_finite() {
            return bad("hysteresis percentages must be finite");
        }
        if self.threshold_pct <= 0.0 || self.threshold_pct > 100.0 {
            return bad("threshold_pct must be in (0, 100]");
        }
        match sense {
            LatchSense::Ceiling => {
                if self.reset_pct < 0.0 || self.reset_pct >= self.threshold_pct {
                    return bad("ceiling reset_pct must be in [0, threshold_pct)");
                }
            }
            LatchSense::Floor => {
                if self.reset_pct <= self.edge_pct(sense) {
                    return bad("floor reset_pct must be above the approach edge");
                }
            }
        }
        Ok(())
    }
}

/// Why a latch reset: the stock, its bound, the reset line crossed, the dwell served, and the
/// event that moved the stock back.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveryEvidence {
    /// The stock when recovery was confirmed.
    pub stock: f64,
    /// The declared bound.
    pub limit: f64,
    /// CID of the bounding commitment/manifest that declared the limit.
    pub bound_ref: String,
    /// The reset line, percent of the limit.
    pub reset_pct: f64,
    /// When the stock last crossed back to the safe side (start of the served dwell).
    pub crossed_back_at: DateTime<Utc>,
    /// The dwell that was served, in seconds.
    pub dwell_s: u64,
    /// The event that moved the stock back (a dialogue stall's closing event).
    pub cause_ref: String,
}

/// A recovery report: the latch's `Recovered` transition on the wire.
///
/// Separate from [`AlgedonicSignal`] on purpose — that type's evidence is the Approach/Breach
/// pair, and a recovery is not a third pain shape. Same envelope, same snake_case wire,
/// `algedonic-recovered.schema.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(into = "RecoveredWire", try_from = "RecoveredWire")]
pub struct RecoveredSignal {
    /// CID of the promise whose pain cleared.
    pub target: String,
    /// The producer that declared the pain and its recovery.
    pub declarer: String,
    /// What recovered, and why.
    pub evidence: RecoveryEvidence,
    /// Advisory loudness.
    pub severity: Severity,
    /// When the producer signed the report.
    pub signed_at: DateTime<Utc>,
}

impl RecoveredSignal {
    /// Always [`AlgedonicKind::Recovered`].
    pub const fn kind(&self) -> AlgedonicKind {
        AlgedonicKind::Recovered
    }

    /// Always [`STANDING_IMPACT`].
    pub const fn standing_impact(&self) -> &'static str {
        STANDING_IMPACT
    }

    /// The wire schema's `minLength`/finiteness obligations; also run on every decode.
    pub fn validate(&self) -> Result<()> {
        let refuse = |what: &str| Err(EprError::Algedonic(what.to_string()));
        let e = &self.evidence;
        if self.target.is_empty() || self.declarer.is_empty() {
            return refuse("recovered target and declarer must be non-empty");
        }
        if e.bound_ref.is_empty() || e.cause_ref.is_empty() {
            return refuse("recovery evidence bound_ref and cause_ref must be non-empty");
        }
        if !e.stock.is_finite() || !e.limit.is_finite() {
            return refuse("recovery evidence stock and limit must be finite");
        }
        if !e.reset_pct.is_finite() || e.reset_pct < 0.0 {
            return refuse("recovery evidence reset_pct must be a finite, non-negative percentage");
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RecoveredWire {
    signal_kind: AlgedonicKind,
    target: String,
    declarer: String,
    evidence: RecoveryEvidence,
    #[serde(default)]
    severity: Severity,
    signed_at: DateTime<Utc>,
}

impl From<RecoveredSignal> for RecoveredWire {
    fn from(s: RecoveredSignal) -> Self {
        RecoveredWire {
            signal_kind: AlgedonicKind::Recovered,
            target: s.target,
            declarer: s.declarer,
            evidence: s.evidence,
            severity: s.severity,
            signed_at: s.signed_at,
        }
    }
}

impl TryFrom<RecoveredWire> for RecoveredSignal {
    type Error = EprError;

    fn try_from(w: RecoveredWire) -> Result<Self> {
        if w.signal_kind != AlgedonicKind::Recovered {
            return Err(EprError::Algedonic(format!(
                "signal_kind {} is not a recovery",
                w.signal_kind.as_str()
            )));
        }
        let signal = RecoveredSignal {
            target: w.target,
            declarer: w.declarer,
            evidence: w.evidence,
            severity: w.severity,
            signed_at: w.signed_at,
        };
        signal.validate()?;
        Ok(signal)
    }
}

/// What a latch declares about the pain it holds.
#[derive(Debug, Clone, PartialEq)]
pub struct LatchConfig {
    /// Producer identity.
    pub declarer: String,
    /// CID of the promise threatened.
    pub target: String,
    /// CID of the bound's declaring commitment/manifest.
    pub bound_ref: String,
    /// The declared limit, in the stock's unit.
    pub limit: f64,
    /// Which side of the limit is safe.
    pub sense: LatchSense,
    /// Entry, reset and dwell.
    pub hysteresis: Hysteresis,
}

/// The single transition one [`AlgedonicLatch::observe`] call can produce.
#[derive(Debug, Clone, PartialEq)]
pub enum LatchTransition {
    /// Band edge crossed; Approach key opened.
    Approach(AlgedonicSignal),
    /// Limit crossed; Breach key opened (and Approach, implied by it, if not already open).
    Breach(AlgedonicSignal),
    /// Dwell served on the safe side; every open key cleared.
    Recovered(RecoveredSignal),
}

/// What the pain source reports alongside the stock on one observation.
///
/// The stock alone cannot say "a stall is open at duration 0" apart from "the stall closed", so
/// the source says which. Only a `Closed` reading can count toward a dwell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reading<'a> {
    /// The source is still active, whatever the stock: never safe, so it cancels any dwell.
    Active,
    /// The source closed; `cause_ref` names the closing event. When the stock is also on the
    /// safe side, a dwell starts and this ref is captured as its cause.
    Closed {
        /// The event that moved the stock back.
        cause_ref: &'a str,
    },
}

/// One open key's lifecycle record, persisted verbatim.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct OpenState {
    kind: AlgedonicKind,
    since: DateTime<Utc>,
    #[serde(default)]
    safe_since: Option<DateTime<Utc>>,
    /// The closing event captured when the running dwell began.
    #[serde(default)]
    safe_cause: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct LatchState {
    declarer: String,
    target: String,
    bound_ref: String,
    open: Vec<OpenState>,
}

/// A persisted latch with hysteresis: the alternative to caller-held [`should_emit`] memory.
///
/// ```text
/// Clear --Approach or Breach--> Signal open
/// Signal open --Closed reading, stock safe--> Dwelling   (safe_since + cause captured)
/// Dwelling --Active reading or unsafe stock--> Signal open (resume; nothing re-emitted)
/// Dwelling --dwell_s elapsed--> Clear                     (Recovered emitted)
/// ```
///
/// State is written to the caller-supplied path on every change (unique temp file, fsync, rename,
/// directory fsync) and read back by [`Self::open`], so a restart neither re-emits an open
/// signal nor forgets a dwell. Each transition is persisted before it is committed in memory: a
/// failed write returns an error and leaves the latch unchanged, so a retry re-derives the same
/// transition. The latch holds an exclusive lock on `<path>.lock` for its lifetime; a second
/// handle on the same path is refused. Time is a parameter everywhere; the latch never reads a
/// clock.
///
/// **Downtime is not credited.** The latch cannot see stalls that happened while it was down, so
/// the first observation after [`Self::open`] restarts any running dwell from that observation
/// and cannot itself complete a recovery.
#[derive(Debug)]
pub struct AlgedonicLatch {
    config: LatchConfig,
    path: PathBuf,
    open: Vec<OpenState>,
    /// True until the first observation after load.
    fresh: bool,
    /// Directory fsync, injectable so tests can fail it.
    dir_sync: fn(&std::path::Path) -> std::io::Result<()>,
    /// Set when a state write was renamed into place but its directory fsync failed.
    durability_warning: Option<String>,
    /// Held for the latch's lifetime; releasing is dropping.
    _lock: std::fs::File,
}

impl AlgedonicLatch {
    /// Load the latch persisted at `path` (or start Clear if none), after validating `config`.
    /// A state file written for a different declarer/target/bound is refused, not adopted.
    pub fn open(config: LatchConfig, path: impl Into<PathBuf>) -> Result<Self> {
        config.hysteresis.validate(config.sense)?;
        if !config.limit.is_finite() || config.limit <= 0.0 {
            return Err(EprError::Algedonic(
                "latch limit must be finite and positive".into(),
            ));
        }
        if config.declarer.is_empty() || config.target.is_empty() || config.bound_ref.is_empty() {
            return Err(EprError::Algedonic(
                "latch declarer, target and bound_ref must be non-empty".into(),
            ));
        }
        for pct in [
            config.hysteresis.edge_pct(config.sense),
            config.hysteresis.reset_pct,
        ] {
            if !(config.limit * (pct / 100.0)).is_finite() {
                return Err(EprError::Algedonic(
                    "latch limit overflows its threshold or reset line".into(),
                ));
            }
        }
        let path = path.into();
        let mut lock_name = path.clone().into_os_string();
        lock_name.push(".lock");
        let lock = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&lock_name)
            .map_err(|e| EprError::Algedonic(format!("latch lock not opened: {e}")))?;
        lock.try_lock().map_err(|_| {
            EprError::Algedonic(format!(
                "latch state {} is held by another handle",
                path.display()
            ))
        })?;
        let open = match std::fs::read(&path) {
            Ok(bytes) => {
                let state: LatchState = serde_json::from_slice(&bytes).map_err(|e| {
                    EprError::Algedonic(format!("latch state {} unreadable: {e}", path.display()))
                })?;
                if state.declarer != config.declarer
                    || state.target != config.target
                    || state.bound_ref != config.bound_ref
                {
                    return Err(EprError::Algedonic(format!(
                        "latch state {} belongs to another (declarer, target, bound)",
                        path.display()
                    )));
                }
                state.open
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(e) => {
                return Err(EprError::Algedonic(format!(
                    "latch state {} unreadable: {e}",
                    path.display()
                )))
            }
        };
        Ok(Self {
            config,
            path,
            open,
            fresh: true,
            dir_sync: |d| std::fs::File::open(d).and_then(|f| f.sync_all()),
            durability_warning: None,
            _lock: lock,
        })
    }

    /// The kinds currently held open (Approach and/or Breach).
    pub fn open_kinds(&self) -> Vec<AlgedonicKind> {
        self.open.iter().map(|o| o.kind).collect()
    }

    /// Whether the latch is Clear.
    pub fn is_clear(&self) -> bool {
        self.open.is_empty()
    }

    /// Whether a dwell is currently running.
    pub fn is_dwelling(&self) -> bool {
        self.open.iter().any(|o| o.safe_since.is_some())
    }

    fn line(&self, pct: f64) -> f64 {
        self.config.limit * (pct / 100.0)
    }

    fn safe(&self, stock: f64) -> bool {
        let reset = self.line(self.config.hysteresis.reset_pct);
        match self.config.sense {
            LatchSense::Ceiling => stock <= reset,
            LatchSense::Floor => stock >= reset,
        }
    }

    fn approach_crossed(&self, stock: f64) -> bool {
        let edge = self.line(self.config.hysteresis.edge_pct(self.config.sense));
        match self.config.sense {
            LatchSense::Ceiling => stock >= edge,
            LatchSense::Floor => stock <= edge,
        }
    }

    fn breach_crossed(&self, stock: f64) -> bool {
        match self.config.sense {
            LatchSense::Ceiling => stock >= self.config.limit,
            LatchSense::Floor => stock <= self.config.limit,
        }
    }

    /// Take the pending durability warning, if any: a state write landed (rename succeeded) but
    /// the directory fsync failed, so the new state may not survive a power loss. The transition
    /// was still committed and returned; this is a warning, not a rollback.
    pub fn take_durability_warning(&mut self) -> Option<String> {
        self.durability_warning.take()
    }

    /// Durably write `open`: unique temp file, fsync, rename, then directory fsync. Once the
    /// rename succeeds the write has happened: a directory-sync failure is returned as
    /// `Ok(Some(warning))`, never as an error.
    fn persist(&self, open: &[OpenState]) -> Result<Option<String>> {
        use std::io::Write;
        use std::sync::atomic::{AtomicU64, Ordering};
        static SEQ: AtomicU64 = AtomicU64::new(0);

        let state = LatchState {
            declarer: self.config.declarer.clone(),
            target: self.config.target.clone(),
            bound_ref: self.config.bound_ref.clone(),
            open: open.to_vec(),
        };
        let bytes = serde_json::to_vec_pretty(&state)
            .map_err(|e| EprError::Algedonic(format!("latch state not serializable: {e}")))?;
        let io = |e: std::io::Error| {
            EprError::Algedonic(format!(
                "latch state {} not written: {e}",
                self.path.display()
            ))
        };
        let mut tmp = self.path.clone().into_os_string();
        tmp.push(format!(
            ".{}.{}.tmp",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::SeqCst)
        ));
        let tmp = PathBuf::from(tmp);
        let written = (|| {
            let mut f = std::fs::File::create(&tmp)?;
            f.write_all(&bytes)?;
            f.sync_all()?;
            std::fs::rename(&tmp, &self.path)
        })();
        if let Err(e) = written {
            let _ = std::fs::remove_file(&tmp);
            return Err(io(e));
        }
        let dir = match self.path.parent() {
            Some(d) if !d.as_os_str().is_empty() => d,
            _ => std::path::Path::new("."),
        };
        Ok((self.dir_sync)(dir).err().map(|e| {
            format!(
                "latch state {} renamed but directory fsync failed: {e}",
                self.path.display()
            )
        }))
    }

    /// Persist `next`, then swap it in. A failed write leaves the latch unchanged.
    fn commit(&mut self, next: Vec<OpenState>) -> Result<()> {
        if next != self.open {
            let warning = self.persist(&next)?;
            self.open = next;
            if warning.is_some() {
                self.durability_warning = warning;
            }
        }
        self.fresh = false;
        Ok(())
    }

    fn pain(
        &self,
        evidence: AlgedonicEvidence,
        severity: Severity,
        now: DateTime<Utc>,
    ) -> AlgedonicSignal {
        AlgedonicSignal {
            target: self.config.target.clone(),
            declarer: self.config.declarer.clone(),
            evidence,
            severity,
            signed_at: now,
        }
    }

    /// Observe the stock at `now`; returns at most one transition.
    ///
    /// - Clear, edge crossed: `Approach`. Limit crossed: `Breach` (which also opens the Approach
    ///   key it implies, so falling back between the lines never re-announces an Approach).
    /// - Open, [`Reading::Closed`] and the stock on the safe side of the reset line: a dwell
    ///   starts, capturing that `cause_ref`; once `dwell_s` has elapsed every key clears and
    ///   `Recovered` is returned carrying the captured cause.
    /// - Open and [`Reading::Active`], or the stock not safe: any running dwell is cancelled and
    ///   the same signal resumes. Nothing is re-emitted: the keys never closed.
    /// - The first observation after [`Self::open`] never completes a recovery (downtime is
    ///   not credited).
    ///
    /// Errors, including a failed state write, leave the latch exactly as it was.
    pub fn observe(
        &mut self,
        stock: f64,
        now: DateTime<Utc>,
        reading: Reading<'_>,
    ) -> Result<Option<LatchTransition>> {
        if !stock.is_finite() {
            return Err(EprError::Algedonic("observed stock must be finite".into()));
        }
        let h = self.config.hysteresis;
        let mut next = self.open.clone();

        if !next.is_empty() {
            let closing = match reading {
                Reading::Closed { cause_ref } if self.safe(stock) => Some(cause_ref),
                _ => None,
            };
            if let Some(cause_ref) = closing {
                let dwelling = next[0].safe_since.is_some();
                let restarted = dwelling && self.fresh;
                if !dwelling && cause_ref.is_empty() {
                    return Err(EprError::Algedonic(
                        "a recovery dwell needs a non-empty cause_ref".into(),
                    ));
                }
                let started = if dwelling && !restarted {
                    next[0].safe_since.unwrap_or(now)
                } else {
                    now
                };
                for o in &mut next {
                    o.safe_since = Some(started);
                    if !dwelling {
                        o.safe_cause = Some(cause_ref.to_string());
                    }
                }
                let served = (now - started).num_seconds();
                if !restarted && served >= 0 && served as u64 >= h.dwell_s {
                    let cause = next[0].safe_cause.clone().unwrap_or_default();
                    next.clear();
                    self.commit(next)?;
                    return Ok(Some(LatchTransition::Recovered(RecoveredSignal {
                        target: self.config.target.clone(),
                        declarer: self.config.declarer.clone(),
                        evidence: RecoveryEvidence {
                            stock,
                            limit: self.config.limit,
                            bound_ref: self.config.bound_ref.clone(),
                            reset_pct: h.reset_pct,
                            crossed_back_at: started,
                            dwell_s: h.dwell_s,
                            cause_ref: cause,
                        },
                        severity: Severity::Info,
                        signed_at: now,
                    })));
                }
                self.commit(next)?;
                return Ok(None);
            }
            for o in &mut next {
                o.safe_since = None;
                o.safe_cause = None;
            }
        }

        let holds = |next: &[OpenState], k: AlgedonicKind| next.iter().any(|o| o.kind == k);
        let opened = |kind| OpenState {
            kind,
            since: now,
            safe_since: None,
            safe_cause: None,
        };
        let transition = if self.breach_crossed(stock) && !holds(&next, AlgedonicKind::Breach) {
            if !holds(&next, AlgedonicKind::Approach) {
                next.push(opened(AlgedonicKind::Approach));
            }
            next.push(opened(AlgedonicKind::Breach));
            let ev = AlgedonicEvidence::Breach {
                stock,
                limit: self.config.limit,
                bound_ref: self.config.bound_ref.clone(),
            };
            Some(LatchTransition::Breach(self.pain(
                ev,
                Severity::Critical,
                now,
            )))
        } else if self.approach_crossed(stock) && !holds(&next, AlgedonicKind::Approach) {
            next.push(opened(AlgedonicKind::Approach));
            let ev = AlgedonicEvidence::Approach {
                stock,
                limit: self.config.limit,
                bound_ref: self.config.bound_ref.clone(),
                threshold_pct: h.threshold_pct,
            };
            Some(LatchTransition::Approach(self.pain(
                ev,
                Severity::Warn,
                now,
            )))
        } else {
            None
        };
        self.commit(next)?;
        Ok(transition)
    }
}

/// Sense-aware sibling of [`should_emit`]: the same two rules (silence inside the band, one open
/// signal per key), with the crossing judged on the correct side of the limit.
///
/// [`should_emit`] is ceiling-only by contract and stays so. A Floor consumer must call this:
/// Floor evidence travels in the wire's ceiling-shaped payload (margin-form `threshold_pct`),
/// which [`AlgedonicEvidence::crossed`] would misread. Floor Approach edge is
/// `limit × (200 − threshold_pct) / 100` (stock at or below it), Floor Breach is stock at or below
/// the limit.
pub fn should_emit_sensed(
    sense: LatchSense,
    prev: Option<&AlgedonicSignal>,
    new: &AlgedonicSignal,
) -> bool {
    let crossed = match sense {
        LatchSense::Ceiling => new.evidence.crossed(),
        LatchSense::Floor => {
            let limit = new.evidence.limit();
            let edge = match &new.evidence {
                AlgedonicEvidence::Approach { threshold_pct, .. } => {
                    limit * ((200.0 - threshold_pct) / 100.0)
                }
                AlgedonicEvidence::Breach { .. } => limit,
            };
            new.evidence.stock() <= edge
        }
    };
    if !crossed {
        return false;
    }
    match prev {
        Some(open) => open_signal_key(open) != open_signal_key(new),
        None => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use serde_json::Value;
    use std::fs;
    use std::path::PathBuf;

    // ── Harness ──────────────────────────────────────────────────────

    /// The wire contract lives in the repo, not in a constant here — this test reads the same
    /// bytes CI and the seeder read.
    fn load_schema(file: &str) -> Value {
        // CARGO_MANIFEST_DIR = elohim/epr/ ; parent = elohim/
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("parent of epr/ must be elohim/")
            .join("sdk/schemas/v1/feedback-signals")
            .join(file);
        let content = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read schema {}: {}", path.display(), e));
        serde_json::from_str(&content)
            .unwrap_or_else(|e| panic!("failed to parse schema {}: {}", path.display(), e))
    }

    fn assert_conforms(schema_file: &str, instance: &Value) {
        let schema = load_schema(schema_file);
        let validator = jsonschema::validator_for(&schema)
            .unwrap_or_else(|e| panic!("failed to compile schema {}: {}", schema_file, e));
        let errors: Vec<String> = validator
            .iter_errors(instance)
            .map(|e| format!("  - {} (at {})", e, e.instance_path))
            .collect();
        assert!(
            errors.is_empty(),
            "schema validation failed for {}:\n{}\n\ninstance:\n{}",
            schema_file,
            errors.join("\n"),
            serde_json::to_string_pretty(instance).unwrap()
        );
    }

    fn signed_at() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 8, 10, 12, 0, 0).unwrap()
    }

    fn approach() -> AlgedonicSignal {
        AlgedonicSignal {
            target: "bafy-promise".into(),
            declarer: "bafy-matthew".into(),
            evidence: AlgedonicEvidence::Approach {
                stock: 880.0,
                limit: 1000.0,
                bound_ref: "bafy-bound".into(),
                threshold_pct: 85.0,
            },
            severity: Severity::Warn,
            signed_at: signed_at(),
        }
    }

    fn breach() -> AlgedonicSignal {
        AlgedonicSignal {
            target: "bafy-promise".into(),
            declarer: "bafy-matthew".into(),
            evidence: AlgedonicEvidence::Breach {
                stock: 1200.0,
                limit: 1000.0,
                bound_ref: "bafy-bound".into(),
            },
            severity: Severity::Critical,
            signed_at: signed_at(),
        }
    }

    // ── (a) Evidence is mandatory ─────────────────────────────────────

    /// Evidence-free pain does not deserialize. `stock`/`limit`/`bound_ref` are non-`Option`, so
    /// there is no in-Rust way to omit them at all; the wire is the only place absence can be
    /// attempted, and it is refused there.
    #[test]
    fn evidence_fields_are_mandatory_on_the_wire() {
        for missing in ["stock", "limit", "bound_ref"] {
            let mut wire = serde_json::to_value(breach()).unwrap();
            wire["evidence"]
                .as_object_mut()
                .unwrap()
                .remove(missing)
                .expect("field must have been present to remove");
            let parsed: std::result::Result<AlgedonicSignal, _> = serde_json::from_value(wire);
            assert!(
                parsed.is_err(),
                "a signal missing evidence.{missing} must be refused"
            );
        }
    }

    /// An empty `bound_ref` is present-but-useless evidence: the bound cannot be resolved, so the
    /// pain has no address. `validate` refuses it with the crate's error type.
    #[test]
    fn validate_refuses_evidence_that_names_no_bound() {
        let mut s = breach();
        s.evidence = AlgedonicEvidence::Breach {
            stock: 1200.0,
            limit: 1000.0,
            bound_ref: String::new(),
        };
        let err = validate(&s).expect_err("empty bound_ref must refuse");
        assert!(err.to_string().contains("bound_ref"), "got: {err}");
        assert!(
            matches!(err, EprError::Algedonic(_)),
            "pain has its own refusal class, not the envelope's"
        );

        // …and the valid samples pass, so the refusal is not a blanket one.
        validate(&approach()).expect("valid approach");
        validate(&breach()).expect("valid breach");
    }

    #[test]
    fn validate_refuses_a_signal_with_no_producer_or_consumer() {
        let mut s = approach();
        s.target = String::new();
        assert!(validate(&s).is_err(), "empty target must refuse");

        let mut s = approach();
        s.declarer = String::new();
        assert!(validate(&s).is_err(), "empty declarer must refuse");
    }

    // ── (b) Approach requires threshold_pct; Breach does not ──────────

    /// The per-kind requirement is carried by the evidence variants, so it holds in both
    /// directions: an approach without a band edge is refused, a breach without one is fine.
    #[test]
    fn approach_requires_threshold_pct_and_breach_does_not() {
        let mut wire = serde_json::to_value(approach()).unwrap();
        wire["evidence"]
            .as_object_mut()
            .unwrap()
            .remove("threshold_pct")
            .expect("approach evidence must carry threshold_pct");
        let parsed: std::result::Result<AlgedonicSignal, _> = serde_json::from_value(wire);
        assert!(
            parsed.is_err(),
            "an approach whose evidence lost its band edge must be refused"
        );

        // A breach never carried one, and round-trips clean.
        let wire = serde_json::to_value(breach()).unwrap();
        assert!(
            wire["evidence"].get("threshold_pct").is_none(),
            "breach evidence must not emit a band edge"
        );
        let back: AlgedonicSignal = serde_json::from_value(wire).expect("breach round-trips");
        assert_eq!(back, breach());
    }

    /// The kind is derived from the evidence, so the two cannot drift apart in Rust; the only
    /// place they can disagree is untrusted wire input, and that is refused.
    #[test]
    fn kind_and_evidence_cannot_disagree() {
        assert_eq!(approach().kind(), AlgedonicKind::Approach);
        assert_eq!(breach().kind(), AlgedonicKind::Breach);

        let mut wire = serde_json::to_value(breach()).unwrap();
        wire["signal_kind"] = Value::String("algedonic-approach".into());
        let err = serde_json::from_value::<AlgedonicSignal>(wire)
            .expect_err("mismatched signal_kind must be refused");
        assert!(err.to_string().contains("disagrees"), "got: {err}");
    }

    // ── (c) No valence, ever ──────────────────────────────────────────

    /// Algedonic is a viability signal, not a judgment. The struct simply has no valence field —
    /// this test pins the consequence: nothing valence-shaped reaches the wire, and the emitted
    /// key-set stays inside what the schema declares (`additionalProperties: false`).
    #[test]
    fn no_valence_field_reaches_the_wire() {
        const VALENCE_SHAPED: [&str; 7] = [
            "position", "valence", "vote", "sign", "agree", "disagree", "polarity",
        ];

        for (signal, schema_file) in [
            (approach(), "algedonic-approach.schema.json"),
            (breach(), "algedonic-breach.schema.json"),
        ] {
            let wire = serde_json::to_value(&signal).unwrap();
            let declared = load_schema(schema_file);
            let properties = declared["properties"].as_object().unwrap();

            for key in wire.as_object().unwrap().keys() {
                assert!(
                    !VALENCE_SHAPED.contains(&key.as_str()),
                    "{schema_file}: valence-shaped key `{key}` must not exist"
                );
                assert!(
                    properties.contains_key(key),
                    "{schema_file}: emitted key `{key}` is not declared by the schema"
                );
            }

            // standing_impact is a constant, not a field — emitting it would breach
            // additionalProperties: false.
            assert!(
                wire.get("standing_impact").is_none(),
                "standing_impact must not serialize"
            );
            assert_eq!(signal.standing_impact(), "advisory");
        }
    }

    // ── (d) Wire parity with the schemas on disk ──────────────────────

    /// Serialize a valid sample of each kind and validate it against the schema file itself —
    /// no copied constant, no restated field list.
    #[test]
    fn samples_conform_to_the_wire_schemas() {
        for (signal, schema_file) in [
            (approach(), "algedonic-approach.schema.json"),
            (breach(), "algedonic-breach.schema.json"),
        ] {
            let wire = serde_json::to_value(&signal).unwrap();
            assert_conforms(schema_file, &wire);

            // Every schema-`required` field is actually present (the validator would catch this,
            // but naming it here is what makes a future schema addition fail loudly).
            let declared = load_schema(schema_file);
            for required in declared["required"].as_array().unwrap() {
                let field = required.as_str().unwrap();
                assert!(
                    wire.get(field).is_some(),
                    "{schema_file}: required field `{field}` missing from the emitted signal"
                );
            }
            for required in declared["properties"]["evidence"]["required"]
                .as_array()
                .unwrap()
            {
                let field = required.as_str().unwrap();
                assert!(
                    wire["evidence"].get(field).is_some(),
                    "{schema_file}: required evidence field `{field}` missing"
                );
            }
        }
    }

    #[test]
    fn signal_kind_matches_the_schema_const() {
        for (kind, schema_file) in [
            (AlgedonicKind::Approach, "algedonic-approach.schema.json"),
            (AlgedonicKind::Breach, "algedonic-breach.schema.json"),
        ] {
            let declared = load_schema(schema_file);
            assert_eq!(
                declared["properties"]["signal_kind"]["const"]
                    .as_str()
                    .unwrap(),
                kind.as_str()
            );
            assert_eq!(
                serde_json::to_value(kind).unwrap().as_str().unwrap(),
                kind.as_str()
            );
        }
    }

    #[test]
    fn severity_vocabulary_matches_the_schema() {
        let declared = load_schema("algedonic-approach.schema.json");
        let allowed: Vec<&str> = declared["properties"]["severity"]["enum"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        for s in [Severity::Info, Severity::Warn, Severity::Critical] {
            let wire = serde_json::to_value(s).unwrap();
            assert!(
                allowed.contains(&wire.as_str().unwrap()),
                "severity {wire} not in schema enum {allowed:?}"
            );
        }
        // The Rust default agrees with the schema's declared default. This asserts the two
        // *declarations* line up — it says nothing about the ingest path, which is
        // `severity_defaults_to_warn_when_the_wire_omits_it`'s job.
        assert_eq!(
            declared["properties"]["severity"]["default"]
                .as_str()
                .expect("the schema must declare a severity default"),
            serde_json::to_value(Severity::default())
                .unwrap()
                .as_str()
                .unwrap()
        );
    }

    /// `severity` is optional on the wire: the schemas leave it out of `required` and declare
    /// `"default": "warn"`. A producer that omits it is emitting a **contract-valid** document —
    /// proved here by running it through the schema validator — so ingest must accept it and
    /// supply the default, not refuse with `missing field 'severity'`.
    #[test]
    fn severity_defaults_to_warn_when_the_wire_omits_it() {
        for (signal, schema_file) in [
            (approach(), "algedonic-approach.schema.json"),
            (breach(), "algedonic-breach.schema.json"),
        ] {
            let mut wire = serde_json::to_value(&signal).unwrap();
            wire.as_object_mut()
                .unwrap()
                .remove("severity")
                .expect("the fixture must have carried a severity to remove");

            // It is still a valid document by the contract on disk…
            assert_conforms(schema_file, &wire);

            // …so it must deserialize, landing on the schema's default rather than refusing.
            let parsed: AlgedonicSignal = serde_json::from_value(wire).unwrap_or_else(|e| {
                panic!("{schema_file}: a schema-valid signal without severity must parse: {e}")
            });
            assert_eq!(
                parsed.severity,
                Severity::Warn,
                "{schema_file}: an omitted severity must default to warn"
            );

            // Everything else survived the round-trip through the default.
            assert_eq!(parsed.evidence, signal.evidence);
            assert_eq!(parsed.kind(), signal.kind());
        }
    }

    // ── (e) One open signal per (declarer, target, kind) ──────────────

    /// Re-measuring the same pain is not new pain: only `(declarer, target, kind)` keys it.
    #[test]
    fn dedupe_key_ignores_evidence() {
        let a = approach();
        let mut b = approach();
        b.evidence = AlgedonicEvidence::Approach {
            stock: 999.0, // a worse measurement…
            limit: 1000.0,
            bound_ref: "bafy-other-bound".into(),
            threshold_pct: 90.0,
        };
        b.severity = Severity::Critical;
        b.signed_at = Utc.with_ymd_and_hms(2026, 8, 10, 18, 0, 0).unwrap();

        assert_ne!(a, b, "the signals themselves differ");
        assert_eq!(
            open_signal_key(&a),
            open_signal_key(&b),
            "…but they are the same open pain"
        );
    }

    #[test]
    fn dedupe_key_separates_producer_consumer_and_kind() {
        let base = approach();

        let mut other_declarer = approach();
        other_declarer.declarer = "bafy-john".into();
        assert_ne!(open_signal_key(&base), open_signal_key(&other_declarer));

        let mut other_target = approach();
        other_target.target = "bafy-other-promise".into();
        assert_ne!(open_signal_key(&base), open_signal_key(&other_target));

        // Same declarer + target, escalated kind — a distinct key by design.
        assert_ne!(open_signal_key(&base), open_signal_key(&breach()));
    }

    // ── (f) Hysteresis ────────────────────────────────────────────────

    /// Inside the band there is nothing to report; crossing it emits once and then holds.
    #[test]
    fn should_emit_is_false_inside_the_band_and_true_crossing_it() {
        // 84% of the limit — the 85% band edge has not been reached.
        let mut inside = approach();
        inside.evidence = AlgedonicEvidence::Approach {
            stock: 840.0,
            limit: 1000.0,
            bound_ref: "bafy-bound".into(),
            threshold_pct: 85.0,
        };
        assert!(
            !should_emit(None, &inside),
            "a stock inside the band is not pain yet"
        );

        // 88% — crossed.
        assert!(
            should_emit(None, &approach()),
            "crossing the band edge with nothing open must emit"
        );

        // Breach's band edge is the limit itself.
        let mut under = breach();
        under.evidence = AlgedonicEvidence::Breach {
            stock: 999.0,
            limit: 1000.0,
            bound_ref: "bafy-bound".into(),
        };
        assert!(!should_emit(None, &under), "the bound still holds");
        assert!(should_emit(None, &breach()), "the bound is crossed");
    }

    /// Pain is a held state, not a stream.
    #[test]
    fn should_emit_suppresses_re_fire_while_a_signal_is_open() {
        let open = approach();

        let mut worse = approach();
        worse.evidence = AlgedonicEvidence::Approach {
            stock: 950.0,
            limit: 1000.0,
            bound_ref: "bafy-bound".into(),
            threshold_pct: 85.0,
        };
        assert!(
            !should_emit(Some(&open), &worse),
            "an open signal on the same key must not re-fire"
        );
    }

    /// …but an open approach must never swallow the breach that escalates it, and pain about a
    /// different promise is never suppressed by pain about this one.
    #[test]
    fn should_emit_lets_escalation_and_unrelated_pain_through() {
        let open_approach = approach();
        assert!(
            should_emit(Some(&open_approach), &breach()),
            "an open approach must not suppress its breach"
        );

        let mut other_promise = approach();
        other_promise.target = "bafy-other-promise".into();
        assert!(
            should_emit(Some(&open_approach), &other_promise),
            "an open signal must not suppress a different promise's pain"
        );
    }

    // ── Round-trip ────────────────────────────────────────────────────

    #[test]
    fn signals_round_trip() {
        for s in [approach(), breach()] {
            let j = serde_json::to_string(&s).unwrap();
            let back: AlgedonicSignal = serde_json::from_str(&j).unwrap();
            assert_eq!(s, back);
        }
    }

    // ── Recovery latch (slice 4) ──────────────────────────────────────

    use std::sync::atomic::{AtomicU32, Ordering};

    fn state_path() -> PathBuf {
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "algedonic-latch-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir_all(&dir).unwrap();
        dir.join("latch.json")
    }

    fn t(secs: i64) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 30, 12, 0, 0).unwrap() + chrono::Duration::seconds(secs)
    }

    /// Dialogue shape: 300 s Breach limit, Approach at 60%, stock zero when healthy.
    fn dialogue(path: &PathBuf) -> AlgedonicLatch {
        AlgedonicLatch::open(
            LatchConfig {
                declarer: "bafy-sense".into(),
                target: "bafy-promise".into(),
                bound_ref: "bafy-bound".into(),
                limit: 300.0,
                sense: LatchSense::Ceiling,
                hysteresis: Hysteresis {
                    threshold_pct: 60.0,
                    reset_pct: 0.0,
                    dwell_s: 120,
                },
            },
            path,
        )
        .unwrap()
    }

    /// Ceiling shaped like `approach()`/`breach()`: limit 1000, edge 85%, reset 70%, dwell 60 s.
    fn ceiling(path: &PathBuf) -> AlgedonicLatch {
        AlgedonicLatch::open(
            LatchConfig {
                declarer: "bafy-matthew".into(),
                target: "bafy-promise".into(),
                bound_ref: "bafy-bound".into(),
                limit: 1000.0,
                sense: LatchSense::Ceiling,
                hysteresis: Hysteresis {
                    threshold_pct: 85.0,
                    reset_pct: 70.0,
                    dwell_s: 60,
                },
            },
            path,
        )
        .unwrap()
    }

    /// Floor: limit 100, margin 80 => edge 120% (stock <= 120), reset 150% (safe at stock >= 150), dwell 60 s.
    fn floor(path: &PathBuf) -> AlgedonicLatch {
        AlgedonicLatch::open(
            LatchConfig {
                declarer: "bafy-matthew".into(),
                target: "bafy-promise".into(),
                bound_ref: "bafy-bound".into(),
                limit: 100.0,
                sense: LatchSense::Floor,
                hysteresis: Hysteresis {
                    threshold_pct: 80.0,
                    reset_pct: 150.0,
                    dwell_s: 60,
                },
            },
            path,
        )
        .unwrap()
    }

    fn cl(cause_ref: &str) -> Reading<'_> {
        Reading::Closed { cause_ref }
    }

    fn kind_of(tr: &Option<LatchTransition>) -> Option<AlgedonicKind> {
        match tr {
            Some(LatchTransition::Approach(_)) => Some(AlgedonicKind::Approach),
            Some(LatchTransition::Breach(_)) => Some(AlgedonicKind::Breach),
            Some(LatchTransition::Recovered(_)) => Some(AlgedonicKind::Recovered),
            None => None,
        }
    }

    #[test]
    fn recovered_kind_has_its_wire_string_and_schema_const() {
        assert_eq!(AlgedonicKind::Recovered.as_str(), "algedonic-recovered");
        assert_eq!(
            serde_json::to_value(AlgedonicKind::Recovered).unwrap(),
            Value::String("algedonic-recovered".into())
        );
        let schema = load_schema("algedonic-recovered.schema.json");
        assert_eq!(
            schema["properties"]["signal_kind"]["const"],
            "algedonic-recovered"
        );
    }

    #[test]
    fn hysteresis_refuses_a_reset_line_on_the_wrong_side() {
        let h = |t, r| Hysteresis {
            threshold_pct: t,
            reset_pct: r,
            dwell_s: 1,
        };
        assert!(h(85.0, 70.0).validate(LatchSense::Ceiling).is_ok());
        assert!(h(60.0, 0.0).validate(LatchSense::Ceiling).is_ok());
        assert!(h(85.0, 85.0).validate(LatchSense::Ceiling).is_err());
        assert!(h(85.0, 90.0).validate(LatchSense::Ceiling).is_err());
        assert!(h(80.0, 150.0).validate(LatchSense::Floor).is_ok());
        assert!(h(80.0, 120.0).validate(LatchSense::Floor).is_err());
        assert!(h(80.0, 110.0).validate(LatchSense::Floor).is_err());
        assert!(
            h(120.0, 250.0).validate(LatchSense::Floor).is_err(),
            "margin form is <= 100"
        );
    }

    #[test]
    fn ceiling_recovers_after_dwell_with_evidence() {
        let path = state_path();
        let mut l = ceiling(&path);
        assert_eq!(kind_of(&l.observe(500.0, t(0), cl("c")).unwrap()), None);
        assert_eq!(
            kind_of(&l.observe(900.0, t(10), cl("c")).unwrap()),
            Some(AlgedonicKind::Approach)
        );
        assert_eq!(
            kind_of(&l.observe(950.0, t(20), cl("c")).unwrap()),
            None,
            "open key suppresses"
        );
        assert_eq!(
            kind_of(&l.observe(1100.0, t(30), cl("c")).unwrap()),
            Some(AlgedonicKind::Breach)
        );
        // Between reset (700) and edge (850): neither safe nor crossing.
        assert_eq!(kind_of(&l.observe(800.0, t(40), cl("c")).unwrap()), None);
        assert!(!l.is_dwelling());
        // Safe: dwell starts at t=100 and is not served until t=160.
        assert_eq!(
            kind_of(&l.observe(600.0, t(100), cl("close-1")).unwrap()),
            None
        );
        assert!(l.is_dwelling());
        assert_eq!(
            kind_of(&l.observe(600.0, t(159), cl("close-1")).unwrap()),
            None
        );
        let Some(LatchTransition::Recovered(r)) = l.observe(650.0, t(160), cl("close-1")).unwrap()
        else {
            panic!("dwell served: expected Recovered");
        };
        assert!(l.is_clear());
        assert_eq!(r.kind(), AlgedonicKind::Recovered);
        assert_eq!(
            r.evidence,
            RecoveryEvidence {
                stock: 650.0,
                limit: 1000.0,
                bound_ref: "bafy-bound".into(),
                reset_pct: 70.0,
                crossed_back_at: t(100),
                dwell_s: 60,
                cause_ref: "close-1".into(),
            }
        );
        assert_eq!(r.signed_at, t(160));
        // Clear again: pain can be announced anew.
        assert_eq!(
            kind_of(&l.observe(900.0, t(200), cl("c")).unwrap()),
            Some(AlgedonicKind::Approach)
        );
    }

    #[test]
    fn floor_bound_signals_and_recovers_on_the_low_side() {
        let path = state_path();
        let mut l = floor(&path);
        assert_eq!(kind_of(&l.observe(200.0, t(0), cl("c")).unwrap()), None);
        assert_eq!(
            kind_of(&l.observe(115.0, t(10), cl("c")).unwrap()),
            Some(AlgedonicKind::Approach)
        );
        assert_eq!(
            kind_of(&l.observe(90.0, t(20), cl("c")).unwrap()),
            Some(AlgedonicKind::Breach)
        );
        // 140 is above the limit but below the reset line (150): not safe.
        assert_eq!(kind_of(&l.observe(140.0, t(30), cl("c")).unwrap()), None);
        assert!(!l.is_dwelling());
        assert_eq!(kind_of(&l.observe(160.0, t(40), cl("heal")).unwrap()), None);
        assert!(l.is_dwelling());
        let Some(LatchTransition::Recovered(r)) = l.observe(170.0, t(100), cl("heal")).unwrap()
        else {
            panic!("floor dwell served: expected Recovered");
        };
        assert_eq!(r.evidence.crossed_back_at, t(40));
        assert_eq!(r.evidence.reset_pct, 150.0);
        assert!(l.is_clear());
    }

    #[test]
    fn dwell_outcome_one_a_new_stall_before_approach_resumes_the_same_signal() {
        let path = state_path();
        let mut l = dialogue(&path);
        assert_eq!(
            kind_of(&l.observe(200.0, t(0), cl("c")).unwrap()),
            Some(AlgedonicKind::Approach)
        );
        // Stall closes: stock 0 starts the dwell.
        assert_eq!(kind_of(&l.observe(0.0, t(10), cl("close")).unwrap()), None);
        assert!(l.is_dwelling());
        // A new stall on the same key, 5 s long: far below Approach (180 s). Cancels dwell,
        // emits nothing, and the Approach key is still the open one.
        assert_eq!(
            kind_of(&l.observe(5.0, t(60), Reading::Active).unwrap()),
            None
        );
        assert!(!l.is_dwelling());
        assert_eq!(l.open_kinds(), vec![AlgedonicKind::Approach]);
        // It closes; a NEW full dwell is required: 120 s from t=70, not from t=10.
        assert_eq!(
            kind_of(&l.observe(0.0, t(70), cl("close-2")).unwrap()),
            None
        );
        assert_eq!(
            kind_of(&l.observe(0.0, t(130), cl("close-2")).unwrap()),
            None,
            "old dwell must not count"
        );
        let rec = l.observe(0.0, t(190), cl("close-2")).unwrap();
        assert_eq!(kind_of(&rec), Some(AlgedonicKind::Recovered));
        let Some(LatchTransition::Recovered(r)) = rec else {
            unreachable!()
        };
        assert_eq!(r.evidence.crossed_back_at, t(70));
        assert_eq!(r.evidence.cause_ref, "close-2");
    }

    #[test]
    fn dwell_outcome_two_no_new_stall_emits_recovered_once() {
        let path = state_path();
        let mut l = dialogue(&path);
        assert_eq!(
            kind_of(&l.observe(310.0, t(0), cl("c")).unwrap()),
            Some(AlgedonicKind::Breach)
        );
        assert_eq!(kind_of(&l.observe(0.0, t(20), cl("close")).unwrap()), None);
        assert_eq!(kind_of(&l.observe(0.0, t(139), cl("close")).unwrap()), None);
        assert_eq!(
            kind_of(&l.observe(0.0, t(140), cl("close")).unwrap()),
            Some(AlgedonicKind::Recovered)
        );
        assert_eq!(
            kind_of(&l.observe(0.0, t(300), cl("close")).unwrap()),
            None,
            "recovery is not repeated"
        );
        assert!(l.is_clear());
    }

    #[test]
    fn recrossing_the_reset_line_cancels_and_restarts_dwell() {
        let path = state_path();
        let mut l = ceiling(&path);
        l.observe(900.0, t(0), cl("c")).unwrap();
        l.observe(600.0, t(10), cl("c")).unwrap();
        assert!(l.is_dwelling());
        // Back over the reset line (700), still under the edge: no signal, dwell cancelled.
        assert_eq!(kind_of(&l.observe(750.0, t(40), cl("c")).unwrap()), None);
        assert!(!l.is_dwelling());
        // Safe again at t=50: dwell restarts, so t=70 (60 s after the FIRST safe) is not enough.
        l.observe(600.0, t(50), cl("c")).unwrap();
        assert_eq!(kind_of(&l.observe(600.0, t(70), cl("c")).unwrap()), None);
        assert_eq!(
            kind_of(&l.observe(600.0, t(110), cl("c")).unwrap()),
            Some(AlgedonicKind::Recovered)
        );
    }

    #[test]
    fn exactly_on_the_reset_line_is_safe_and_on_the_edge_is_crossed() {
        let path = state_path();
        let mut l = ceiling(&path);
        assert_eq!(
            kind_of(&l.observe(850.0, t(0), cl("c")).unwrap()),
            Some(AlgedonicKind::Approach)
        );
        l.observe(700.0, t(1), cl("c")).unwrap();
        assert!(l.is_dwelling());
    }

    #[test]
    fn open_signal_survives_restart_without_re_emitting() {
        let path = state_path();
        {
            let mut l = ceiling(&path);
            assert_eq!(
                kind_of(&l.observe(1100.0, t(0), cl("c")).unwrap()),
                Some(AlgedonicKind::Breach)
            );
        }
        let mut l = ceiling(&path);
        assert_eq!(
            l.open_kinds(),
            vec![AlgedonicKind::Approach, AlgedonicKind::Breach]
        );
        assert_eq!(
            kind_of(&l.observe(1200.0, t(5), cl("c")).unwrap()),
            None,
            "no re-emit after restart"
        );
        assert_eq!(
            kind_of(&l.observe(900.0, t(6), cl("c")).unwrap()),
            None,
            "approach key already open"
        );
    }

    #[test]
    fn dwell_survives_restart_but_downtime_is_not_credited_and_cause_is_kept() {
        let path = state_path();
        {
            let mut l = ceiling(&path);
            l.observe(900.0, t(0), cl("c")).unwrap();
            l.observe(600.0, t(10), cl("the-closing-event")).unwrap();
            assert!(l.is_dwelling());
            // A later observation carries a different ref; it must not replace the captured one.
            l.observe(600.0, t(30), cl("later-noise")).unwrap();
        }
        let mut l = ceiling(&path);
        assert!(l.is_dwelling());
        // 3600 s of downtime. The first observation cannot complete the dwell, even though
        // t=3610 is far past dwell_s; it restarts the dwell from itself.
        assert_eq!(
            kind_of(&l.observe(600.0, t(3610), cl("after-restart")).unwrap()),
            None
        );
        assert_eq!(
            kind_of(&l.observe(600.0, t(3640), cl("after-restart")).unwrap()),
            None
        );
        let Some(LatchTransition::Recovered(r)) =
            l.observe(600.0, t(3670), cl("after-restart")).unwrap()
        else {
            panic!("a full dwell after the restart observation: expected Recovered");
        };
        assert_eq!(r.evidence.crossed_back_at, t(3610));
        assert_eq!(r.evidence.cause_ref, "the-closing-event");
        drop(l);
        assert!(ceiling(&path).is_clear());
    }

    #[test]
    fn captured_cause_is_not_the_last_observations_ref() {
        let path = state_path();
        let mut l = ceiling(&path);
        l.observe(900.0, t(0), cl("c")).unwrap();
        l.observe(600.0, t(10), cl("close-A")).unwrap();
        let Some(LatchTransition::Recovered(r)) = l.observe(600.0, t(70), cl("close-B")).unwrap()
        else {
            panic!("expected Recovered");
        };
        assert_eq!(r.evidence.cause_ref, "close-A");
    }

    #[test]
    fn a_new_stall_at_duration_zero_cancels_dwell_and_cannot_recover() {
        let path = state_path();
        let mut l = dialogue(&path);
        l.observe(200.0, t(0), cl("c")).unwrap();
        l.observe(0.0, t(10), cl("close")).unwrap();
        assert!(l.is_dwelling());
        // Stall reopens: duration exactly 0, indistinguishable by stock from closure.
        assert_eq!(
            kind_of(&l.observe(0.0, t(100), Reading::Active).unwrap()),
            None
        );
        assert!(!l.is_dwelling());
        // Past the original dwell deadline (t=130) and still active: no Recovered.
        assert_eq!(
            kind_of(&l.observe(0.0, t(200), Reading::Active).unwrap()),
            None
        );
        assert!(!l.is_clear());
    }

    #[test]
    fn a_failed_write_leaves_the_latch_unchanged_and_a_retry_still_emits() {
        let path = state_path();
        let dir = path.parent().unwrap().to_path_buf();
        let mut l = ceiling(&path);
        fs::remove_dir_all(&dir).unwrap();
        assert!(l.observe(900.0, t(0), cl("c")).is_err());
        assert!(l.is_clear(), "memory must not run ahead of the disk");
        fs::create_dir_all(&dir).unwrap();
        assert_eq!(
            kind_of(&l.observe(900.0, t(1), cl("c")).unwrap()),
            Some(AlgedonicKind::Approach)
        );

        // Recovery: a failed write must not clear the keys.
        l.observe(600.0, t(10), cl("close")).unwrap();
        fs::remove_dir_all(&dir).unwrap();
        assert!(l.observe(600.0, t(100), cl("close")).is_err());
        assert_eq!(l.open_kinds(), vec![AlgedonicKind::Approach]);
        fs::create_dir_all(&dir).unwrap();
        assert_eq!(
            kind_of(&l.observe(600.0, t(101), cl("close")).unwrap()),
            Some(AlgedonicKind::Recovered)
        );
    }

    #[test]
    fn a_second_handle_on_the_same_state_is_refused() {
        let path = state_path();
        let _first = ceiling(&path);
        let err = AlgedonicLatch::open(
            LatchConfig {
                declarer: "bafy-matthew".into(),
                target: "bafy-promise".into(),
                bound_ref: "bafy-bound".into(),
                limit: 1000.0,
                sense: LatchSense::Ceiling,
                hysteresis: Hysteresis {
                    threshold_pct: 85.0,
                    reset_pct: 70.0,
                    dwell_s: 60,
                },
            },
            &path,
        )
        .expect_err("concurrent handle must be refused");
        assert!(err.to_string().contains("another handle"), "got: {err}");
    }

    #[test]
    fn floor_pain_signals_are_schema_valid_with_the_existing_schemas() {
        let path = state_path();
        let mut l = floor(&path);
        let Some(LatchTransition::Approach(a)) = l.observe(115.0, t(0), cl("c")).unwrap() else {
            panic!("expected Approach");
        };
        let Some(LatchTransition::Breach(b)) = l.observe(90.0, t(1), cl("c")).unwrap() else {
            panic!("expected Breach");
        };
        for (sig, file) in [
            (&a, "algedonic-approach.schema.json"),
            (&b, "algedonic-breach.schema.json"),
        ] {
            validate(sig).unwrap();
            assert_conforms(file, &serde_json::to_value(sig).unwrap());
        }
        let AlgedonicEvidence::Approach { threshold_pct, .. } = a.evidence else {
            panic!("approach evidence");
        };
        assert_eq!(
            threshold_pct, 80.0,
            "margin form, within the schema's (0, 100]"
        );
    }

    #[test]
    fn recovered_decode_validates_and_denies_unknown_fields() {
        let path = state_path();
        let mut l = ceiling(&path);
        l.observe(900.0, t(0), cl("c")).unwrap();
        l.observe(600.0, t(10), cl("x")).unwrap();
        let Some(LatchTransition::Recovered(r)) = l.observe(600.0, t(80), cl("x")).unwrap() else {
            panic!("expected Recovered");
        };
        let good = serde_json::to_value(&r).unwrap();
        let mutate = |f: &dyn Fn(&mut Value)| {
            let mut v = good.clone();
            f(&mut v);
            serde_json::from_value::<RecoveredSignal>(v)
        };
        assert!(mutate(&|_| {}).is_ok());
        assert!(mutate(&|v| v["target"] = Value::String(String::new())).is_err());
        assert!(mutate(&|v| v["evidence"]["cause_ref"] = Value::String(String::new())).is_err());
        assert!(mutate(&|v| v["evidence"]["bound_ref"] = Value::String(String::new())).is_err());
        assert!(mutate(&|v| v["evidence"]["reset_pct"] = serde_json::json!(-1.0)).is_err());
        assert!(mutate(&|v| v["mystery"] = Value::Bool(true)).is_err());
        assert!(mutate(&|v| v["evidence"]["mystery"] = Value::Bool(true)).is_err());
    }

    #[test]
    fn recovered_schema_rejects_malformed_timestamps() {
        let path = state_path();
        let mut l = ceiling(&path);
        l.observe(900.0, t(0), cl("c")).unwrap();
        l.observe(600.0, t(10), cl("x")).unwrap();
        let Some(LatchTransition::Recovered(r)) = l.observe(600.0, t(80), cl("x")).unwrap() else {
            panic!("expected Recovered");
        };
        let validator =
            jsonschema::validator_for(&load_schema("algedonic-recovered.schema.json")).unwrap();
        let good = serde_json::to_value(&r).unwrap();
        assert!(validator.is_valid(&good));
        for bad in [
            "yesterday",
            "2026-10-30",
            "2026-10-30 12:00:00",
            "2026-99-99T99:99:99Z",
            "2026-10-30T24:00:00Z",
        ] {
            let mut v = good.clone();
            v["signed_at"] = Value::String(bad.into());
            assert!(!validator.is_valid(&v), "signed_at {bad:?} must be refused");
            let mut v = good.clone();
            v["evidence"]["crossed_back_at"] = Value::String(bad.into());
            assert!(
                !validator.is_valid(&v),
                "crossed_back_at {bad:?} must be refused"
            );
        }
    }

    #[test]
    fn state_for_another_target_is_refused() {
        let path = state_path();
        ceiling(&path).observe(900.0, t(0), cl("c")).unwrap();
        let err = AlgedonicLatch::open(
            LatchConfig {
                declarer: "bafy-matthew".into(),
                target: "bafy-someone-else".into(),
                bound_ref: "bafy-bound".into(),
                limit: 1000.0,
                sense: LatchSense::Ceiling,
                hysteresis: Hysteresis {
                    threshold_pct: 85.0,
                    reset_pct: 70.0,
                    dwell_s: 60,
                },
            },
            &path,
        )
        .expect_err("foreign state must not be adopted");
        assert!(err.to_string().contains("another"), "got: {err}");
    }

    #[test]
    fn recovered_signal_validates_against_its_schema_beside_approach_and_breach() {
        let path = state_path();
        let mut l = ceiling(&path);
        l.observe(900.0, t(0), cl("c")).unwrap();
        l.observe(600.0, t(10), cl("c")).unwrap();
        let Some(LatchTransition::Recovered(r)) = l.observe(600.0, t(80), cl("close-9")).unwrap()
        else {
            panic!("expected Recovered");
        };
        let wire = serde_json::to_value(&r).unwrap();
        assert_eq!(wire["signal_kind"], "algedonic-recovered");
        assert!(wire.get("standing_impact").is_none());
        assert_conforms("algedonic-recovered.schema.json", &wire);
        // The pain fixtures still conform to their own schemas, and not to this one.
        assert_conforms(
            "algedonic-approach.schema.json",
            &serde_json::to_value(approach()).unwrap(),
        );
        assert_conforms(
            "algedonic-breach.schema.json",
            &serde_json::to_value(breach()).unwrap(),
        );
        let validator =
            jsonschema::validator_for(&load_schema("algedonic-recovered.schema.json")).unwrap();
        assert!(!validator.is_valid(&serde_json::to_value(breach()).unwrap()));
        // Required evidence fields all present; round-trips; wrong kind refused.
        for f in load_schema("algedonic-recovered.schema.json")["properties"]["evidence"]
            ["required"]
            .as_array()
            .unwrap()
        {
            assert!(
                wire["evidence"].get(f.as_str().unwrap()).is_some(),
                "missing {f}"
            );
        }
        let back: RecoveredSignal = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(back, r);
        let mut wrong = wire;
        wrong["signal_kind"] = Value::String("algedonic-breach".into());
        assert!(serde_json::from_value::<RecoveredSignal>(wrong).is_err());
    }

    #[test]
    fn pain_signals_refuse_a_recovered_kind_and_should_emit_is_unchanged() {
        let mut wire = serde_json::to_value(breach()).unwrap();
        wire["signal_kind"] = Value::String("algedonic-recovered".into());
        assert!(serde_json::from_value::<AlgedonicSignal>(wire).is_err());

        // The pre-slice-4 behaviours, case by case.
        assert!(should_emit(None, &breach()));
        assert!(should_emit(None, &approach()));
        let mut inside = approach();
        inside.evidence = AlgedonicEvidence::Approach {
            stock: 100.0,
            limit: 1000.0,
            bound_ref: "bafy-bound".into(),
            threshold_pct: 85.0,
        };
        assert!(!should_emit(None, &inside), "inside the band: silence");
        assert!(!should_emit(Some(&breach()), &breach()));
        assert!(!should_emit(Some(&approach()), &approach()));
        assert!(
            should_emit(Some(&approach()), &breach()),
            "escalation passes"
        );
        let mut other = breach();
        other.target = "bafy-other".into();
        assert!(should_emit(Some(&breach()), &other), "other promise passes");
        let mut other_declarer = breach();
        other_declarer.declarer = "bafy-other".into();
        assert!(should_emit(Some(&breach()), &other_declarer));
        assert_eq!(AlgedonicKind::Approach.as_str(), "algedonic-approach");
        assert_eq!(AlgedonicKind::Breach.as_str(), "algedonic-breach");
    }

    #[test]
    fn sensed_emit_accepts_a_floor_breach_that_legacy_should_emit_rejects() {
        let floor_breach = AlgedonicSignal {
            target: "bafy-promise".into(),
            declarer: "bafy-matthew".into(),
            evidence: AlgedonicEvidence::Breach {
                stock: 90.0,
                limit: 100.0,
                bound_ref: "bafy-bound".into(),
            },
            severity: Severity::Critical,
            signed_at: signed_at(),
        };
        assert!(!should_emit(None, &floor_breach), "legacy is ceiling-only");
        assert!(should_emit_sensed(LatchSense::Floor, None, &floor_breach));
        assert!(!should_emit_sensed(
            LatchSense::Floor,
            Some(&floor_breach),
            &floor_breach
        ));
        let mut healthy = floor_breach.clone();
        healthy.evidence = AlgedonicEvidence::Breach {
            stock: 130.0,
            limit: 100.0,
            bound_ref: "bafy-bound".into(),
        };
        assert!(!should_emit_sensed(LatchSense::Floor, None, &healthy));
        assert!(
            should_emit(None, &healthy),
            "legacy misreads it, which is why the sibling exists"
        );
        let floor_approach = AlgedonicSignal {
            evidence: AlgedonicEvidence::Approach {
                stock: 115.0,
                limit: 100.0,
                bound_ref: "bafy-bound".into(),
                threshold_pct: 80.0,
            },
            ..floor_breach.clone()
        };
        assert!(should_emit_sensed(LatchSense::Floor, None, &floor_approach));
        assert!(should_emit_sensed(
            LatchSense::Floor,
            Some(&floor_approach),
            &floor_breach
        ));
        // Ceiling through the sibling is exactly the legacy answer.
        for (prev, new) in [
            (None, breach()),
            (None, approach()),
            (Some(approach()), breach()),
        ] {
            assert_eq!(
                should_emit_sensed(LatchSense::Ceiling, prev.as_ref(), &new),
                should_emit(prev.as_ref(), &new)
            );
        }
    }

    #[test]
    fn directory_sync_failure_is_a_warning_not_a_rollback() {
        let path = state_path();
        let mut l = ceiling(&path);
        l.dir_sync = |_| Err(std::io::Error::other("injected"));
        let tr = l.observe(900.0, t(0), cl("c")).unwrap();
        assert_eq!(kind_of(&tr), Some(AlgedonicKind::Approach));
        assert_eq!(
            l.open_kinds(),
            vec![AlgedonicKind::Approach],
            "memory follows disk"
        );
        assert!(l.take_durability_warning().unwrap().contains("injected"));
        assert!(l.take_durability_warning().is_none());
        // Disk really advanced: a restart sees the open key.
        drop(l);
        assert_eq!(ceiling(&path).open_kinds(), vec![AlgedonicKind::Approach]);
    }

    #[test]
    fn thresholds_that_overflow_are_refused_and_large_finite_limits_work() {
        let cfg = |limit| LatchConfig {
            declarer: "bafy-matthew".into(),
            target: "bafy-promise".into(),
            bound_ref: "bafy-bound".into(),
            limit,
            sense: LatchSense::Floor,
            hysteresis: Hysteresis {
                threshold_pct: 80.0,
                reset_pct: 150.0,
                dwell_s: 1,
            },
        };
        let mut ok = AlgedonicLatch::open(cfg(1e307), state_path()).expect("1e307 x 1.5 is finite");
        assert_eq!(
            kind_of(&ok.observe(1.1e307, t(0), cl("c")).unwrap()),
            Some(AlgedonicKind::Approach)
        );
        let err =
            AlgedonicLatch::open(cfg(1.7e308), state_path()).expect_err("1.5 x 1.7e308 overflows");
        assert!(err.to_string().contains("overflows"), "got: {err}");
    }
}
