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
    pub fn is_unknown(&self) -> bool {
        self.lo.is_infinite() && self.hi.is_infinite()
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
