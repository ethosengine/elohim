//! Measure dynamics + confidence. Two orthogonal questions about a number:
//! WHAT KIND of quantity is it (level / rate / ratio), and HOW WELL is it known
//! (witnessed / estimated, with an interval and a basis).
//!
//! Both ride INSIDE the quantity's canonical bytes. A confidence that could be
//! detached could be narrowed after the fact, which is not an estimate.

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
    pub fn unknown() -> Self {
        Interval {
            lo: f64::NEG_INFINITY,
            hi: f64::INFINITY,
        }
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
        if !self.is_wellformed() || !other.is_wellformed() {
            return Interval::unknown();
        }
        let (lo, hi) = (self.lo - other.hi, self.hi - other.lo);
        if lo.is_nan() || hi.is_nan() {
            return Interval::unknown(); // ∞ - ∞ at a bound
        }
        Interval::new(lo, hi)
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
        if !self.is_wellformed() || !other.is_wellformed() {
            return Interval::unknown();
        }
        if other.lo <= 0.0 && other.hi >= 0.0 {
            return Interval::unknown();
        }
        let candidates = [
            self.lo / other.lo,
            self.lo / other.hi,
            self.hi / other.lo,
            self.hi / other.hi,
        ];
        if candidates.iter().any(|v| v.is_nan()) {
            return Interval::unknown(); // ∞/∞ at a bound
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
        Interval::new(lo, hi)
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
        if scalar.is_nan() || scalar == 0.0 || lo_mult > hi_mult {
            return Interval::unknown();
        }
        let (a, b) = (scalar * lo_mult, scalar * hi_mult);
        if a.is_nan() || b.is_nan() {
            return Interval::unknown();
        }
        Interval::new(a.min(b), a.max(b))
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
        }
    }
    pub fn estimated(interval: Interval, basis: impl Into<String>) -> Self {
        Confidence {
            claim: ClaimKind::Estimated,
            interval,
            basis: basis.into(),
        }
    }
    pub fn unknown(basis: impl Into<String>) -> Self {
        Confidence {
            claim: ClaimKind::Estimated,
            interval: Interval::unknown(),
            basis: basis.into(),
        }
    }
    /// Widening is always free. Narrowing is refused — it needs a new observation.
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
