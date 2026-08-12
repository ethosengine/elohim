//! Measure dynamics + confidence. Two orthogonal questions about a number:
//! WHAT KIND of quantity is it (level / rate / ratio), and HOW WELL is it known
//! (witnessed / estimated, with an interval and a basis).
//!
//! Both ride INSIDE the quantity's canonical bytes. A confidence that could be
//! detached could be narrowed after the fact, which is not an estimate.
//!
//! A third question arrives with the first one that has no answer: when an interval is
//! unknown, WHY. See [`UnknownReason`] — absence is not one thing, and a queue that ranks
//! "which edge, if measured better, most tightens this?" is unrankable until it can tell an
//! empty window from a zero-crossing denominator from a quantity nothing will ever tighten.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Period {
    Second,
    Minute,
    Hour,
    Day,
    Week,
    Month,
    Year,
}

/// A level is a stock; a rate is a flow and CANNOT forget its denominator.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum MeasureKind {
    Level,
    Rate { per: Period },
    Ratio,
}

impl MeasureKind {
    pub fn period(&self) -> Option<Period> {
        match self {
            MeasureKind::Rate { per } => Some(*per),
            _ => None,
        }
    }

    /// Dimensional algebra for division — the law that makes the shipped unit error
    /// unrepresentable rather than merely documented.
    ///
    /// The defect this exists to refuse is live in this tree: `spatial_capacity.rs` divides an
    /// ALL-TIME CUMULATIVE SUM (a level) by `max_sustainable_yield` (a rate, per its own name)
    /// and reads the quotient as a utilization ratio. There is no interpretation of that number.
    /// Under this algebra the same division is well-formed but returns a **Level denominated in
    /// the divisor's period** — a *time*, which is Meadows' coverage time and is emphatically
    /// not a utilization. The arithmetic never changes; what changes is that the result can no
    /// longer be mistaken for the thing it is not.
    ///
    /// `None` means REFUSED, and refusal is the honest default for every combination whose
    /// meaning we have not established. In particular a `Ratio` in either position is refused:
    /// a ratio of ratios has real uses (an index of indices) but no settled reading here, and
    /// inventing one inside a dimensional-safety helper is how a safety helper becomes the
    /// source of the next unit error.
    ///
    /// KNOWN GAP (spec Q15): `Level ÷ Rate` returns a bare `Level`, so the period it is
    /// denominated in survives only in the caller's `basis` string, not in the type. A turnover
    /// time of `3.0` is three *days* or three *years* depending on a divisor the result no
    /// longer carries — which is a weaker version of the very forgetting `Rate { per }` exists
    /// to prevent. It is recorded rather than fixed here because the fix is a new variant
    /// (`Duration { per }`) and minting protocol vocabulary is not an implementer's call.
    pub fn divide(self, by: MeasureKind) -> Option<MeasureKind> {
        match (self, by) {
            // Two flows of the same tempo: a dimensionless index. Meadows' harvest/regeneration
            // and emission/absorption, and the respite/response controllability ratio, are all
            // this one shape — which is why there is one function and not three.
            (MeasureKind::Rate { per: a }, MeasureKind::Rate { per: b }) if a == b => {
                Some(MeasureKind::Ratio)
            }
            // Different tempos do NOT silently convert. A per-day rate over a per-year rate is
            // 365x away from the number a caller expects, and picking a conversion here would
            // bury the assumption in a helper nobody reads.
            (MeasureKind::Rate { .. }, MeasureKind::Rate { .. }) => None,
            // Stock over drain: TIME, counted in the drain's period. Turnover and coverage time.
            (MeasureKind::Level, MeasureKind::Rate { .. }) => Some(MeasureKind::Level),
            // Stock against stock: a genuine dimensionless share (this corpus vs. its ceiling).
            (MeasureKind::Level, MeasureKind::Level) => Some(MeasureKind::Ratio),
            _ => None,
        }
    }

    /// Dimensional algebra for addition and subtraction: only like adds to like.
    ///
    /// Net change (`inflow - outflow`) is the one place a stock model subtracts, and it is
    /// exactly where a period mismatch would be invisible — two flows both "per" something,
    /// differenced into a number that is per neither.
    pub fn combine_additive(self, other: MeasureKind) -> Option<MeasureKind> {
        if self == other {
            Some(self)
        } else {
            None
        }
    }
}

/// WHY an interval lost its bounds. Absence is not one thing, and treating it as one is how a
/// work-queue over uncertainty ranks the unfixable first.
///
/// `Interval::unknown()` is honest about *that* it knows nothing and silent about *why*, and the
/// arithmetic below discards a reason it structurally holds at seven distinct mint sites. That
/// silence is affordable right up until something tries to act on the uncertainty. The thing that
/// will (measure-family-borrows-backlog row 18) decomposes an aggregate's uncertainty by
/// contribution and emits a prioritized queue answering *"which edge, if measured better, most
/// tightens this?"* — and three unknowns that are indistinguishable today must not be:
///
/// - **no observations in the window** — the instrument exists (or could); more measurement
///   genuinely narrows this. It BELONGS in the queue, near the top.
/// - **a denominator that crossed zero, or an indeterminate form at a bound** — a structure
///   problem. Sampling the numerator harder changes nothing; the model or an upstream operand
///   has to change first.
/// - **incommensurable by construction** — see `_lib/drift_score.py::measure`'s own confession
///   (spec Q16): a weighted sum of normalized time and log-counts is a preference ordering
///   wearing a quantity's clothes. NOTHING will ever tighten it. It must NEVER appear in the
///   queue, because "go measure the incommensurable composite harder" is precisely the
///   false-precision reflex this ontology exists to refuse.
///
/// Without the type, row 18's ranking is computed over a set whose members it cannot tell apart,
/// so its top recommendation is unfalsifiable. That is the whole argument for a seven-variant
/// enum instead of a comment.
///
/// **On the variant set.** `NoObservations` / `UndefinedDivision` / `IndeterminateForm` /
/// `ZeroBase` / `Incommensurable` / `NotYetInstrumented` are the six the design brief named.
/// `MalformedInput` is added because it is the seventh reason the arithmetic *already knows* and
/// was already discarding: `sub` and `div` both open with an `is_wellformed` guard, and
/// `multiplier_widen` refuses an inverted multiplier pair. A `lo > hi` operand is an upstream
/// defect, not an absence of observation, and folding it into `NoObservations` would put a bug
/// report into a measurement queue. `NotYetInstrumented` is kept distinct from `NoObservations`
/// on the same principle in the other direction: an empty window is an observation *result*, no
/// instrument at all is an observation *capability* — and row 18's stated purpose ("the system's
/// own ignorance tells it where to add senses") is served only by the second being nameable.
///
/// Derives match [`ClaimKind`] exactly — the nearest sibling and the other claim-quality
/// vocabulary — so this round-trips identically through serde/dag-cbor. `kebab-case` likewise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UnknownReason {
    /// The observation window contained no observations. The instrument exists; nothing arrived.
    NoObservations,
    /// No instrument exists for this quantity yet. Distinct from `NoObservations`: that one has
    /// a sense and an empty result, this one has no sense.
    NotYetInstrumented,
    /// The denominator interval admits zero, so the quotient is genuinely unbounded
    /// (`Interval::div`). More measurement of the numerator cannot help.
    UndefinedDivision,
    /// `∞/∞`, `∞−∞`, or `0·∞` at a bound — an operand already asserted infinity, and the
    /// arithmetic has no finite answer to give.
    IndeterminateForm,
    /// Multiplicative widening was asked to manufacture width from a zero base
    /// (`Interval::multiplier_widen`). A zero count IS an observation; what is missing is an
    /// additive band, i.e. a different instrument, not more of this one.
    ZeroBase,
    /// An operand was malformed (`lo > hi`, a NaN bound, an inverted multiplier pair). An
    /// upstream defect wearing an absence's clothes.
    MalformedInput,
    /// The quantity is a composite of terms that share no scale (spec Q16). No observation of
    /// any input narrows it, because the number is not an estimate OF anything observable.
    Incommensurable,
}

/// What, if anything, would narrow an unknown — the discriminator row 18's queue is built on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Tightenable {
    /// More or better observation of the same quantity narrows this. Queue it as measurement work.
    ByMeasurement,
    /// No amount of the same measurement helps — the model, the instrument, or an upstream
    /// operand must change first. Queue it, but as STRUCTURAL work, never as "sample harder."
    ByStructure,
    /// Nothing will ever tighten this. Must never enter the queue at all.
    Never,
}

impl UnknownReason {
    /// The three-way split row 18 cannot rank without. Deliberately not a boolean: collapsing
    /// `ByStructure` into "actionable" would send a sampling task to a zero-crossing denominator,
    /// and collapsing it into "not actionable" would hide a real, fixable defect.
    pub fn tightenable(self) -> Tightenable {
        match self {
            UnknownReason::NoObservations | UnknownReason::NotYetInstrumented => {
                Tightenable::ByMeasurement
            }
            UnknownReason::UndefinedDivision
            | UnknownReason::IndeterminateForm
            | UnknownReason::ZeroBase
            | UnknownReason::MalformedInput => Tightenable::ByStructure,
            UnknownReason::Incommensurable => Tightenable::Never,
        }
    }

    /// A **total** order over the vocabulary, used only as the tie-break inside [`reduce`].
    ///
    /// [`UnknownReason::tightenable`] is deliberately non-injective — seven reasons onto three
    /// responses — which makes it useless on its own as a reduction key. `max_by_key` keeps the
    /// LAST maximum, so a reduction keyed on `tightenable()` alone resolves an equal-rank pair by
    /// INPUT ORDER: `[ZeroBase, UndefinedDivision]` and `[UndefinedDivision, ZeroBase]` are the
    /// same multiset and would fold to different reasons, hence to `Quantity` values that are not
    /// `PartialEq`. Two peers folding the same terms in different orders would mint different
    /// values — exactly what `epr-rea`'s
    /// `uncertainty_closure.rs::folding_is_deterministic_under_input_reordering` forbids. The
    /// adjacent `claim_rank` has no such problem only because it is injective over five kinds.
    ///
    /// The index is the declaration order and carries **no semantics** — do not read
    /// `a.stable_index() < b.stable_index()` as "less stuck." It exists so that a reduction over a
    /// multiset has exactly one answer. It is written out rather than derived so that the numbers
    /// are visible at the one place a change to them would silently change fold outputs (and so
    /// that `_lib/signal_measure.py::UNKNOWN_REASONS`, whose list position is the same tie-break
    /// on the Python side, has something order-sensitive to be tested against).
    pub fn stable_index(self) -> u8 {
        match self {
            UnknownReason::NoObservations => 0,
            UnknownReason::NotYetInstrumented => 1,
            UnknownReason::UndefinedDivision => 2,
            UnknownReason::IndeterminateForm => 3,
            UnknownReason::ZeroBase => 4,
            UnknownReason::MalformedInput => 5,
            UnknownReason::Incommensurable => 6,
        }
    }

    /// The ONE reduction over reasons, shared by every site that combines them.
    ///
    /// **Rule: the least tightenable wins; ties break on [`stable_index`](Self::stable_index).**
    /// A derived quantity is no more improvable than the worst thing that fed it, so if any
    /// contributor is `Incommensurable`, the result is — and a work-queue that inherited the
    /// friendlier reason would rank "measure this harder" against a number no measurement can
    /// touch. The tie-break is arbitrary by design (see `stable_index`) and load-bearing anyway:
    /// without it the answer depends on argument order.
    ///
    /// Returns `None` for an empty input, which is the honest answer and not a default: nothing
    /// contributed a reason, so nothing is claimed. Callers must not invent one.
    ///
    /// This is the same reduction `fold::with_uncertainty` applies across a sum's terms and
    /// [`Reasoned::attributed_to`] applies across an operation's guards and operands. One rule,
    /// three call sites — because two rules would eventually disagree about the same multiset.
    pub fn reduce<I: IntoIterator<Item = UnknownReason>>(reasons: I) -> Option<UnknownReason> {
        reasons
            .into_iter()
            .max_by_key(|r| (r.tightenable_rank(), r.stable_index()))
    }

    /// How stuck, ascending. Private: the public discriminator is [`tightenable`](Self::tightenable),
    /// which is a named three-way answer rather than a magic number.
    fn tightenable_rank(self) -> u8 {
        match self.tightenable() {
            Tightenable::ByMeasurement => 0,
            Tightenable::ByStructure => 1,
            Tightenable::Never => 2,
        }
    }
}

/// An interval operation's outcome, carrying WHY when the bounds are lost.
///
/// **Why this is a pair and not a third field on `Interval`.** `Interval` is hash-bearing:
/// `Quantity` (which embeds it through `Confidence`) is serialized with `serde_ipld_dagcbor` and
/// fed to `cid::compute_cid` — that is L2, and `tests/canonical_bytes.rs` asserts it as the
/// property the ontology is named for. A third field on `Interval` would appear in the canonical
/// dag-cbor map of *every* quantity ever encoded and move every one of those addresses. So the
/// reason rides alongside the bounds here, and reaches the wire only on `Confidence`, where it is
/// an `Option` that is skipped when absent — byte-identical for every value that does not carry
/// one. See `tests/canonical_bytes.rs::adding_an_unknown_reason_moves_no_existing_address`.
///
/// **Invariant, one direction only: `reason.is_some()` implies `interval.is_unknown()`.** Both
/// constructors uphold that, and so does [`Reasoned::attributed_to`].
///
/// The converse is FALSE and a reader must not assume it. `Reasoned::known` accepts any interval
/// — including an unknown one — because an operation can lose its bounds without any guard
/// firing: `unknown().sub_because(&unknown())` computes `(-∞) − (+∞) = -∞` and `(+∞) − (-∞) = +∞`,
/// produces no `NaN`, and so returns `known((-∞,+∞))` with no reason (pinned in
/// `tests/measure_dimensional.rs`). `reason.is_none()` therefore means *nothing named a reason*,
/// NOT *the bounds are finite*. Test `interval.is_unknown()` before touching `lo`/`hi`; reading
/// the absence of a reason as the presence of bounds dereferences ±∞.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Reasoned {
    pub interval: Interval,
    #[serde(
        rename = "unknownReason",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub reason: Option<UnknownReason>,
}

impl Reasoned {
    /// Bounds survived; there is nothing to explain.
    pub fn known(interval: Interval) -> Self {
        Reasoned {
            interval,
            reason: None,
        }
    }
    /// Bounds were lost, and this is why.
    pub fn unknown(reason: UnknownReason) -> Self {
        Reasoned {
            interval: Interval::unknown(),
            reason: Some(reason),
        }
    }
    pub fn is_unknown(&self) -> bool {
        self.interval.is_unknown()
    }

    /// Attribute an unknown outcome to its OPERANDS as well as to this operation's own guards.
    ///
    /// `sub_because` / `div_because` / `multiplier_widen_because` see bare `Interval`s, so they
    /// can only ever mint the reasons their own guards discover. That is half the story: an
    /// operand may arrive already carrying one, and if it does, dropping it reproduces the exact
    /// failure this vocabulary was minted to prevent — `turnover_time` over an `Incommensurable`
    /// level returned an unclassified unknown, free to be ranked as "go measure it harder"
    /// against something no measurement can ever tighten.
    ///
    /// **Precedence: none. There is a reduction instead** — [`UnknownReason::reduce`], the same
    /// least-tightenable-wins rule `fold::with_uncertainty` applies across a sum's terms, with the
    /// guard's own reason entering as just another contributor. A guard does not outrank an
    /// operand and an operand does not outrank a guard; the *least improvable* of them wins,
    /// because that is what the derived quantity actually is. Worked both ways: a zero-crossing
    /// denominator (`UndefinedDivision`, structural) over an `Incommensurable` numerator yields
    /// `Incommensurable` — fixing the model would not help, the numerator is not an estimate of
    /// anything. The same denominator over a `NoObservations` numerator yields
    /// `UndefinedDivision` — sampling the numerator is precisely the useless work, and the
    /// structural finding is the one to queue.
    ///
    /// A bounded outcome is returned untouched: operand reasons explain an absence, and there is
    /// no absence here. That keeps the type's one-directional invariant intact.
    pub fn attributed_to<I>(self, operands: I) -> Self
    where
        I: IntoIterator<Item = Option<UnknownReason>>,
    {
        if !self.interval.is_unknown() {
            return self;
        }
        let reason = UnknownReason::reduce(
            self.reason
                .into_iter()
                .chain(operands.into_iter().flatten()),
        );
        Reasoned {
            interval: self.interval,
            reason,
        }
    }
}

impl From<Reasoned> for Interval {
    fn from(r: Reasoned) -> Interval {
        r.interval
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Interval {
    pub lo: f64,
    pub hi: f64,
}

impl Interval {
    pub fn new(lo: f64, hi: f64) -> Self {
        Interval { lo, hi }
    }
    pub fn exact(v: f64) -> Self {
        Interval { lo: v, hi: v }
    }
    /// Honest absence: the interval that admits everything.
    ///
    /// Unchanged, deliberately. Every existing caller keeps the exact bounds and the exact
    /// canonical bytes it had; [`Interval::unknown_because`] is strictly additive beside it.
    pub fn unknown() -> Self {
        Interval {
            lo: f64::NEG_INFINITY,
            hi: f64::INFINITY,
        }
    }
    /// Honest absence that also says WHY — the same bounds as [`Interval::unknown`], plus the
    /// reason, carried in the pair rather than in the hash-bearing struct (see [`Reasoned`]).
    ///
    /// `Interval::unknown_because(r).interval == Interval::unknown()` holds by construction, so
    /// this is additive in the strongest sense available: it cannot change what any existing
    /// reader sees, only give a new reader something more to read.
    pub fn unknown_because(reason: UnknownReason) -> Reasoned {
        Reasoned::unknown(reason)
    }
    /// Honest absence is `(-∞, +∞)` SPECIFICALLY, and the sign check is load-bearing.
    ///
    /// Spec Q12: this previously read `lo.is_infinite() && hi.is_infinite()`, which reports
    /// `[+∞, +∞]` — an assertion that a quantity is *exactly* infinite, the most confident
    /// claim expressible and the precise shape L3 exists to forbid — as honest absence. The
    /// detector that L3 depends on could not tell absence from certainty at infinity, so the
    /// one guard against manufactured precision was blind to the manufactured value it was
    /// there to catch. Reachable, not theoretical: a stock with zero outflow produces `[+∞, +∞]`
    /// by construction under any multiply-through widening scheme (Q10).
    pub fn is_unknown(&self) -> bool {
        self.lo == f64::NEG_INFINITY && self.hi == f64::INFINITY
    }
    /// `lo <= hi`, both non-NaN. Q9 asks whether `new` should REJECT a malformed pair; this
    /// deliberately does not answer that — it only lets arithmetic that would propagate a
    /// malformed input as `NaN` (which satisfies no comparison and so fails silently) refuse
    /// instead. A predicate is not a constructor decision.
    pub fn is_wellformed(&self) -> bool {
        !self.lo.is_nan() && !self.hi.is_nan() && self.lo <= self.hi
    }
    pub fn contains(&self, v: f64) -> bool {
        v >= self.lo && v <= self.hi
    }
    pub fn width(&self) -> f64 {
        self.hi - self.lo
    }
    /// True iff `other` is at least as wide as self on both sides.
    pub fn is_widened_by(&self, other: &Interval) -> bool {
        other.lo <= self.lo && other.hi >= self.hi
    }

    /// `self - other`, correlation-naive (spec Q1): `[a,b] - [c,d] = [a-d, b-c]`.
    ///
    /// Assumes perfect anti-correlation and therefore OVER-widens for independent terms, which
    /// is the safe direction — over-widening never manufactures precision. Malformed or unknown
    /// inputs propagate as unknown rather than as `NaN`.
    pub fn sub(&self, other: &Interval) -> Interval {
        self.sub_because(other).interval
    }

    /// [`Interval::sub`], keeping the reason it already knew.
    ///
    /// The two absences this operation can produce are not the same thing and never were:
    /// a malformed operand is an upstream defect, and `∞ − ∞` at a bound is an indeterminate
    /// form created by an operand that already asserted infinity. `sub` computed both and told
    /// its caller neither.
    pub fn sub_because(&self, other: &Interval) -> Reasoned {
        if !self.is_wellformed() || !other.is_wellformed() {
            return Interval::unknown_because(UnknownReason::MalformedInput);
        }
        let (lo, hi) = (self.lo - other.hi, self.hi - other.lo);
        if lo.is_nan() || hi.is_nan() {
            // ∞ - ∞ at a bound
            return Interval::unknown_because(UnknownReason::IndeterminateForm);
        }
        Reasoned::known(Interval::new(lo, hi))
    }

    /// `self / other` — and the divide-by-a-band-containing-zero case is the whole point.
    ///
    /// When the denominator interval admits zero, the quotient is genuinely unbounded, and the
    /// ONLY honest answer is `Interval::unknown()`. This is the structural cure for the bug
    /// slice 1 hit by hand and patched at one call site (spec Q10/Q13): a zero denominator used
    /// to collapse both bounds onto `+∞`, asserting exact infinity at the exact moment the data
    /// was most absent. Routing every division through here means no future index has to
    /// rediscover that — a stock with no outflow now reports unbounded turnover as *unknown*,
    /// which is true, instead of as *infinite*, which is a claim.
    pub fn div(&self, other: &Interval) -> Interval {
        self.div_because(other).interval
    }

    /// [`Interval::div`], keeping the reason it already knew.
    ///
    /// Three absences leave this function and they rank differently in row 18's queue:
    /// `MalformedInput` is a bug, `UndefinedDivision` is a zero-crossing denominator (measure the
    /// numerator forever and it stays unbounded — the *denominator's* structure is the finding),
    /// and `IndeterminateForm` is `∞/∞` at a bound. Returning a bare `(-∞,+∞)` for all three
    /// made them one thing to every reader downstream.
    pub fn div_because(&self, other: &Interval) -> Reasoned {
        if !self.is_wellformed() || !other.is_wellformed() {
            return Interval::unknown_because(UnknownReason::MalformedInput);
        }
        if other.lo <= 0.0 && other.hi >= 0.0 {
            return Interval::unknown_because(UnknownReason::UndefinedDivision);
        }
        let candidates = [
            self.lo / other.lo,
            self.lo / other.hi,
            self.hi / other.lo,
            self.hi / other.hi,
        ];
        if candidates.iter().any(|v| v.is_nan()) {
            // ∞/∞ at a bound
            return Interval::unknown_because(UnknownReason::IndeterminateForm);
        }
        let mut lo = candidates.iter().copied().fold(f64::INFINITY, f64::min);
        let mut hi = candidates.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        // OVERFLOW IS NOT A CLAIM. `1e300 / 1e-300` is two finite operands whose true quotient
        // (~1e600) is unrepresentable in f64, and the hardware answers `+∞` — which under L3
        // would read as an assertion that the ratio is EXACTLY infinite, manufactured purely by
        // the float type's range. Found by this module's own test, which is the argument for
        // having written it.
        //
        // Neither available shortcut is honest: `[+∞, +∞]` claims exact infinity (false), and
        // `Interval::unknown()` claims we know nothing (also false — we know the quotient
        // exceeds `f64::MAX`). So say exactly that: bounded below by the largest representable
        // value, unbounded above. The guard is conditioned on the OPERANDS being finite, so a
        // caller who genuinely asserted infinity still has that assertion propagated verbatim
        // rather than laundered into a range they did not claim.
        let operands_finite = [self.lo, self.hi, other.lo, other.hi]
            .iter()
            .all(|v| v.is_finite());
        if operands_finite {
            if lo == f64::INFINITY {
                lo = f64::MAX;
            }
            if hi == f64::NEG_INFINITY {
                hi = f64::MIN;
            }
        }
        Reasoned::known(Interval::new(lo, hi))
    }

    /// Widen a point reading into a band by a multiplier pair — the GENERAL mechanism for
    /// spec Q10, so no future index re-derives its zero case.
    ///
    /// `scalar × [lo_mult, hi_mult]` is the natural way to say "this count is what we could
    /// see; the truth is somewhere between 1x and 3x it." It has one unsound point, and it is
    /// the point that matters most: at `scalar == 0` both products are `0`, so the band
    /// collapses to zero width exactly when the underlying count is least trustworthy. That is
    /// the inverse of what a widening scheme is for. Any scalar that is zero — or that spans
    /// zero in sign, where the multiplier would flip the bounds' order — yields honest absence.
    pub fn multiplier_widen(scalar: f64, lo_mult: f64, hi_mult: f64) -> Interval {
        Interval::multiplier_widen_because(scalar, lo_mult, hi_mult).interval
    }

    /// [`Interval::multiplier_widen`], keeping the reason it already knew.
    ///
    /// The zero-base case is the one this distinction most repays. A zero count is an
    /// *observation*, not a missing one — sampling harder returns zero again. What is missing is
    /// an additive band (an absolute floor on what we could have failed to see), which is a
    /// different instrument. `ZeroBase → ByStructure` says exactly that; folding it into
    /// `NoObservations` would have queued "go count more" against a number that is already
    /// correctly counted.
    ///
    /// Argument order matters here and is checked in this order deliberately: a NaN scalar or an
    /// inverted/NaN multiplier pair is `MalformedInput` (an upstream defect), `scalar == 0.0` is
    /// `ZeroBase`, and only a `0 · ∞` product survives to `IndeterminateForm`.
    pub fn multiplier_widen_because(scalar: f64, lo_mult: f64, hi_mult: f64) -> Reasoned {
        if scalar.is_nan() || lo_mult.is_nan() || hi_mult.is_nan() || lo_mult > hi_mult {
            return Interval::unknown_because(UnknownReason::MalformedInput);
        }
        if scalar == 0.0 {
            return Interval::unknown_because(UnknownReason::ZeroBase);
        }
        let (a, b) = (scalar * lo_mult, scalar * hi_mult);
        if a.is_nan() || b.is_nan() {
            // 0 · ∞ — the only remaining way to get here, the scalar's zero case being gone.
            return Interval::unknown_because(UnknownReason::IndeterminateForm);
        }
        Reasoned::known(Interval::new(a.min(b), a.max(b)))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ClaimKind {
    Witnessed,
    InstrumentMeasured,
    Estimated,
    Modelled,
    Imputed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Confidence {
    pub claim: ClaimKind,
    pub interval: Interval,
    /// What grounds this claim. A bare ± is uninterpretable.
    pub basis: String,
    /// WHY the interval is unknown, when it is. `None` on every claim whose interval carries
    /// bounds, and on every unknown minted before this field existed.
    ///
    /// This is the field's WIRE home, and the placement is forced. The reason has to survive to
    /// wherever the aggregate is read (measure-family-borrows-backlog row 18 decomposes an
    /// aggregate's uncertainty and cannot classify what it cannot see), so it cannot live only
    /// in a transient [`Reasoned`] — but it also cannot live on [`Interval`], which is inside
    /// the canonical bytes of every quantity ever encoded (L2). `Option` + `skip_serializing_if`
    /// resolves both: a `Confidence` without a reason encodes to the byte-identical dag-cbor it
    /// always did, so no existing address moves, while one *with* a reason legitimately mints a
    /// different address — it carries strictly more of a claim, which is what L2 says an address
    /// is for.
    ///
    /// Invariant, upheld by every constructor here: `Some` only when `interval.is_unknown()`.
    /// The field is public (matching its siblings and Q8's standing caveat), so a struct-literal
    /// caller can still violate it; `unknown_reason_never_accompanies_a_bounded_interval` in
    /// `tests/measure_ontology.rs` pins the constructors, not the type.
    #[serde(
        rename = "unknownReason",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub unknown_reason: Option<UnknownReason>,
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ConfidenceError {
    #[error("narrowing an interval requires a new observation, not a mutation")]
    NarrowingRefused,
}

impl Confidence {
    pub fn witnessed(interval: Interval, basis: impl Into<String>) -> Self {
        Confidence {
            claim: ClaimKind::Witnessed,
            interval,
            basis: basis.into(),
            unknown_reason: None,
        }
    }
    pub fn estimated(interval: Interval, basis: impl Into<String>) -> Self {
        Confidence {
            claim: ClaimKind::Estimated,
            interval,
            basis: basis.into(),
            unknown_reason: None,
        }
    }
    /// Unchanged in meaning and in bytes: an unknown that does not say why. Kept because a
    /// caller who genuinely cannot name the reason should not be forced to invent one — the
    /// enum exists to record structurally-known reasons, not to extract guesses.
    pub fn unknown(basis: impl Into<String>) -> Self {
        Confidence {
            claim: ClaimKind::Estimated,
            interval: Interval::unknown(),
            basis: basis.into(),
            unknown_reason: None,
        }
    }
    /// [`Confidence::unknown`] that also says why.
    pub fn unknown_because(reason: UnknownReason, basis: impl Into<String>) -> Self {
        Confidence {
            claim: ClaimKind::Estimated,
            interval: Interval::unknown(),
            basis: basis.into(),
            unknown_reason: Some(reason),
        }
    }
    /// Build a confidence directly from an arithmetic outcome, so the reason `sub_because` /
    /// `div_because` / `multiplier_widen_because` computed reaches the wire instead of being
    /// dropped one line after it was known. This is the constructor a derived quantity should
    /// use; the alternative is a struct literal that silently forgets the field.
    pub fn from_reasoned(claim: ClaimKind, outcome: Reasoned, basis: impl Into<String>) -> Self {
        Confidence {
            claim,
            interval: outcome.interval,
            basis: basis.into(),
            unknown_reason: outcome.reason,
        }
    }
    /// Widening is always free. Narrowing is refused — it needs a new observation.
    ///
    /// `unknown_reason` rides along unchanged. Only `(-∞,+∞)` widens an already-unknown
    /// interval, so a carried reason still describes the interval it is attached to; and a
    /// bounded claim widened all the way to unknown keeps `None`, because the widener named no
    /// reason and this constructor will not invent one.
    pub fn widen(&self, to: Interval) -> Result<Confidence, ConfidenceError> {
        if self.interval.is_widened_by(&to) {
            Ok(Confidence {
                interval: to,
                ..self.clone()
            })
        } else {
            Err(ConfidenceError::NarrowingRefused)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Quantity {
    pub value: f64,
    #[serde(flatten)]
    pub kind: MeasureKind,
    pub confidence: Confidence,
}
