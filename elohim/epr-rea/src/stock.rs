//! Stock and flow — the plant, not the regulator.
//!
//! We have built a great deal of controller (requisite variety, recursion, the algedonic
//! bypass) and never wrote down the physics of the thing being regulated. Meadows' vocabulary
//! for that is small and hard: a **stock** is a level, accumulated by past history; **flows**
//! are rates that fill and drain it; and the diagnostic quantities are ratios *of rates*, never
//! levels against ceilings.
//!
//! Slice 1 made those distinctions EXPRESSIBLE (`MeasureKind::{Level, Rate{per}, Ratio}`).
//! Nothing was modelled. This module models one, and the discipline is deliberate: one stock,
//! folded honestly, beats a stock-and-flow framework with nothing in it.
//!
//! # Everything here is DERIVED
//!
//! A `Stock` is never stored. It is a pure fold over [`FlowEvent`] history, exactly as
//! [`crate::fold::resource_state`] is — this crate's standing law is that resource *state* is
//! derived, so two peers folding the same events mint the same stock with no coordination and
//! no shared clock. A stored level would be a second home for a number that already has one,
//! which is the shape that lets `CarryingCapacity.current_utilization` drift against the events
//! that supposedly produce it.
//!
//! # Sources and sinks are different, and the direction of the index is not a detail
//!
//! A resource being *harvested* is unsustainable when `harvest / regeneration > 1.0`. A sink
//! being *filled* is unsustainable when `emission / absorption > 1.0`. They are reciprocals,
//! they are both computed from the same two rates, and picking the wrong one inverts the
//! finding — so both are named here rather than leaving a caller to remember which way up their
//! resource is. Our own doc corpus is a **sink**: documents are emitted, and compaction,
//! decompose, and moves to `held/` are the absorption.
//!
//! Either index is a **leading** signal. Meadows: *"deforestation is indicated not when the
//! forest is gone, but when the rate of harvest first exceeds the rate of regrowth."* A level
//! against a ceiling only fires once the damage is visible.

use cid::Cid;
use elohim_epr::measure::{ClaimKind, Confidence, Interval, MeasureKind, Period, Quantity};
use elohim_epr::witness::{Magnitude, ReaVerb};

use crate::model::FlowEvent;

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum StockError {
    #[error("a stock's level must be a Level, got {0:?}")]
    LevelIsNotALevel(MeasureKind),
    #[error("a stock's {side} must be a Rate with a declared period, got {kind:?}")]
    FlowIsNotARate {
        side: &'static str,
        kind: MeasureKind,
    },
    #[error("inflow is {inflow:?} but outflow is {outflow:?} — flows of different tempo cannot be compared without a conversion this crate refuses to guess")]
    MismatchedPeriods {
        inflow: MeasureKind,
        outflow: MeasureKind,
    },
    #[error("dimensionally refused: {0:?} / {1:?} has no defined meaning")]
    UndefinedDivision(MeasureKind, MeasureKind),
    #[error("a window must span a positive number of periods, got {0}")]
    EmptyWindow(f64),
}

/// The observation window a flow rate is measured over — and it is part of the measure's
/// IDENTITY, not an argument default.
///
/// This shape exists because of a specific finding: in the first live run of the doc-corpus
/// index, the 28-day window ALONE decided whether the headline read `unknown` or `3.2`. A
/// measure whose conclusion flips on an undeclared parameter is not yet honest — the window is
/// a claim about what counts as "now", and a claim belongs in the basis where a reader can
/// disagree with it.
///
/// `periods` is declared by the caller rather than computed from `start`/`end`. That is not
/// laziness about date arithmetic (though this crate deliberately carries no date dependency):
/// it forces the author to state the denominator their rate is actually per, instead of
/// inheriting whatever a subtraction happened to produce.
///
/// **Timestamp comparison is lexicographic**, which is correct for RFC3339 **in UTC with a
/// uniform format** (`2026-08-11T12:00:00Z`) and silently wrong for mixed offsets — an event
/// stamped `+02:00` sorts by its local wall clock, not its instant. Every timestamp on this
/// path comes from git via `epr flow project`, which is uniform; a producer that is not must
/// normalize before folding.
#[derive(Debug, Clone, PartialEq)]
pub struct Window {
    /// RFC3339, inclusive.
    pub start: String,
    /// RFC3339, exclusive.
    pub end: String,
    /// The denominator every rate folded over this window carries.
    pub per: Period,
    /// How many `per` units the window spans. Declared, never inferred.
    pub periods: f64,
}

impl Window {
    pub fn contains(&self, occurred_at: &str) -> bool {
        occurred_at >= self.start.as_str() && occurred_at < self.end.as_str()
    }

    /// A short human label for a `basis` string — the reader's handle on which "now" this is.
    pub fn label(&self) -> String {
        format!(
            "{} .. {} ({} {:?}-periods)",
            self.start, self.end, self.periods, self.per
        )
    }
}

/// One modelled stock: a level, what fills it, what drains it.
///
/// Constructed only through [`Stock::new`], which refuses every dimensionally incoherent
/// combination. That refusal is the point — the defect this whole arc started from
/// (`spatial_capacity.rs` comparing an all-time cumulative sum to a per-year yield) was not a
/// wrong number, it was a comparison nobody could see was wrong because nothing knew the
/// operands' kinds.
#[derive(Debug, Clone, PartialEq)]
pub struct Stock {
    pub level: Quantity,
    pub inflow: Quantity,
    pub outflow: Quantity,
}

impl Stock {
    pub fn new(level: Quantity, inflow: Quantity, outflow: Quantity) -> Result<Self, StockError> {
        if level.kind != MeasureKind::Level {
            return Err(StockError::LevelIsNotALevel(level.kind));
        }
        if inflow.kind.period().is_none() {
            return Err(StockError::FlowIsNotARate {
                side: "inflow",
                kind: inflow.kind,
            });
        }
        if outflow.kind.period().is_none() {
            return Err(StockError::FlowIsNotARate {
                side: "outflow",
                kind: outflow.kind,
            });
        }
        if inflow.kind != outflow.kind {
            return Err(StockError::MismatchedPeriods {
                inflow: inflow.kind,
                outflow: outflow.kind,
            });
        }
        Ok(Stock {
            level,
            inflow,
            outflow,
        })
    }

    /// `inflow - outflow` — the rate the level is moving, in the flows' shared period.
    ///
    /// Zero net change is the DYNAMIC EQUILIBRIUM Meadows names as the actual target state, and
    /// it is emphatically not stillness: *"sustainability does not mean zero growth."* A corpus
    /// with high inflow and equally high outflow is healthy; one with zero of both is not
    /// stable, it is dead.
    pub fn net_change(&self) -> Result<Quantity, StockError> {
        let kind = self.inflow.kind.combine_additive(self.outflow.kind).ok_or(
            StockError::MismatchedPeriods {
                inflow: self.inflow.kind,
                outflow: self.outflow.kind,
            },
        )?;
        Ok(Quantity {
            value: self.inflow.value - self.outflow.value,
            kind,
            confidence: Confidence {
                claim: weaker(self.inflow.confidence.claim, self.outflow.confidence.claim),
                interval: self
                    .inflow
                    .confidence
                    .interval
                    .sub(&self.outflow.confidence.interval),
                basis: compose_basis("net change", &self.inflow, &self.outflow),
            },
        })
    }

    /// `level / outflow` — how long until the stock has fully turned over, counted in the
    /// outflow's periods.
    ///
    /// This is the measure that separates **dynamic equilibrium from silting**, and no level
    /// ever answers it. A corpus that never turns over is not stable; it is accumulating. When
    /// the outflow band admits zero, turnover is genuinely unbounded and the result is honest
    /// absence — `Interval::unknown()`, never `+∞`, because `+∞` is a claim and absence is not.
    ///
    /// Carries spec Q15: the returned `Level` is denominated in the outflow's period, and that
    /// period survives only in the `basis`. `3.0` here is three days or three years depending
    /// on a divisor the type no longer holds.
    pub fn turnover_time(&self) -> Result<Quantity, StockError> {
        divide(&self.level, &self.outflow, "turnover time")
    }

    /// `inflow / outflow` — the **sink** index. Above 1.0, the sink is filling faster than
    /// anything drains it.
    ///
    /// This is the leg our resource ontology has never had. We model what is drawn — bytes
    /// held, compute delegated — and nothing about what is deposited or whether anything
    /// absorbs it. Generated context is an emission; compaction is the absorption process.
    pub fn emission_absorption(&self) -> Result<Quantity, StockError> {
        divide(&self.inflow, &self.outflow, "emission/absorption")
    }

    /// `outflow / inflow` — the **source** index, Meadows' harvest/regeneration. Above 1.0, the
    /// resource is being taken faster than it comes back.
    ///
    /// The exact reciprocal of [`Stock::emission_absorption`], and named separately because the
    /// direction is where the reasoning error lives: a caller who reaches for "the Meadows
    /// index" without asking whether their resource is a source or a sink has a 50% chance of
    /// reporting sustainability as overshoot.
    pub fn harvest_regeneration(&self) -> Result<Quantity, StockError> {
        divide(&self.outflow, &self.inflow, "harvest/regeneration")
    }
}

/// The controllability index (Meadows/Biesiot, *Indicators* pp.31–32): how fast a problem grows
/// against how fast the system responds.
///
/// > *"Any system in which the rate of growth of a problem is significantly faster than the
/// > rate of response is, quite simply, out of control. There are only two ways to bring it
/// > back into the realm of manageability: either quicken the response rate (if possible) or
/// > slow the growth rate of the problem (or both)."*
///
/// The arithmetic is [`Stock::emission_absorption`]'s — one shape, and there is no honest way
/// to make it two. What differs is which rates you feed it and therefore what the number means:
/// emission/absorption asks whether a stock is bounded; respite/response asks whether *trying
/// harder* is even on the menu. It is not — effort is an attempt to raise the response rate by
/// will, which is the *confuse effort with result* trap Meadows names separately.
///
/// This is the missing denominator of the algedonic layer. A signal that says "this is bad" is
/// weaker than one that says "this is getting worse faster than we can respond," because only
/// the second names the two available cures and rules out the third thing people always try.
pub fn respite_response(
    problem_growth: &Quantity,
    response: &Quantity,
) -> Result<Quantity, StockError> {
    divide(problem_growth, response, "respite/response")
}

fn weaker(a: ClaimKind, b: ClaimKind) -> ClaimKind {
    fn rank(c: ClaimKind) -> u8 {
        match c {
            ClaimKind::Witnessed => 0,
            ClaimKind::InstrumentMeasured => 1,
            ClaimKind::Estimated => 2,
            ClaimKind::Modelled => 3,
            ClaimKind::Imputed => 4,
        }
    }
    if rank(a) >= rank(b) {
        a
    } else {
        b
    }
}

/// Compose a derived quantity's basis from its inputs' own grounding.
///
/// Spec Q11 says `fold::with_uncertainty` DESTROYS basis, replacing every input's grounding
/// with `"fold of N terms"` — a term count, which is exactly the bare ± that `Confidence`'s own
/// doc comment calls uninterpretable. Q11 stays open for the general n-ary fold (whose answer
/// has to bound composition somehow), but a BINARY derivation has no such problem: two bases
/// fit in one string, and there is no reason for a stock's turnover time to name no observation.
fn compose_basis(op: &str, a: &Quantity, b: &Quantity) -> String {
    format!(
        "{op} of [{}] and [{}]",
        a.confidence.basis, b.confidence.basis
    )
}

fn divide(num: &Quantity, den: &Quantity, op: &str) -> Result<Quantity, StockError> {
    let kind = num
        .kind
        .divide(den.kind)
        .ok_or(StockError::UndefinedDivision(num.kind, den.kind))?;
    let interval = num.confidence.interval.div(&den.confidence.interval);
    // Q13, structurally: the VALUE gets the same treatment as the interval. Slice 1 fixed only
    // the interval at a zero denominator and left `value: inf` behind, so a second consumer
    // reading the documented return shape saw `value > 1.0 -> True` and concluded "confirmed
    // overshoot" from an absence. A value that the interval declares unknowable is NaN, which
    // no comparison silently passes.
    let value = if interval.is_unknown() || den.value == 0.0 {
        f64::NAN
    } else {
        num.value / den.value
    };
    Ok(Quantity {
        value,
        kind,
        confidence: Confidence {
            claim: weaker(num.confidence.claim, den.confidence.claim),
            interval,
            basis: compose_basis(op, num, den),
        },
    })
}

/// Fold a resource's [`FlowEvent`] history into a stock over `window`.
///
/// - **level** = every `Produce` minus every `Consume` up to `window.end`, in `unit`. A level
///   is an accumulation of the whole past, so it is NOT windowed — windowing it would report a
///   recent delta as if it were the stock, which is the level/rate confusion one layer up.
/// - **inflow / outflow** = events inside the window, divided by `window.periods`.
///
/// Claims are `Witnessed`: these events are the observation. That is a statement about the
/// ARITHMETIC, not about the projection's completeness — if a caller knows its projection
/// misses absorption paths (ours does), the honest move is [`Confidence::widen`] on the way
/// out, which is free by law. Widening at the edge that knows beats guessing at the edge that
/// counts.
pub fn stock_over_window(
    resource: &Cid,
    events: &[FlowEvent],
    window: &Window,
    unit: &str,
) -> Result<Stock, StockError> {
    // NaN spelled out rather than caught by a negated comparison: a NaN window length would
    // otherwise slip past `<= 0.0` and make every rate NaN with no error anywhere.
    if window.periods.is_nan() || window.periods <= 0.0 {
        return Err(StockError::EmptyWindow(window.periods));
    }
    let (mut produced_all, mut consumed_all) = (0.0f64, 0.0f64);
    let (mut produced_win, mut consumed_win) = (0.0f64, 0.0f64);
    for e in events.iter().filter(|e| &e.resource == resource) {
        let Some(v) = count_in(&e.quantity, unit) else {
            continue;
        };
        if e.occurred_at.as_str() >= window.end.as_str() {
            continue; // the future relative to this window is not part of its level
        }
        let inside = window.contains(&e.occurred_at);
        match e.action {
            ReaVerb::Produce => {
                produced_all += v;
                if inside {
                    produced_win += v;
                }
            }
            ReaVerb::Consume => {
                consumed_all += v;
                if inside {
                    consumed_win += v;
                }
            }
            _ => {}
        }
    }

    let rate = |count: f64, what: &str| Quantity {
        value: count / window.periods,
        kind: MeasureKind::Rate { per: window.per },
        confidence: Confidence {
            claim: ClaimKind::Witnessed,
            interval: Interval::exact(count / window.periods),
            basis: format!(
                "{count} {what} events on {unit} witnessed in the flow log over {}",
                window.label()
            ),
        },
    };

    Stock::new(
        Quantity {
            value: produced_all - consumed_all,
            kind: MeasureKind::Level,
            confidence: Confidence {
                claim: ClaimKind::Witnessed,
                interval: Interval::exact(produced_all - consumed_all),
                basis: format!(
                    "{produced_all} produced - {consumed_all} consumed ({unit}), all history \
                     through {}",
                    window.end
                ),
            },
        },
        rate(produced_win, "produce"),
        rate(consumed_win, "consume"),
    )
}

fn count_in(magnitude: &Magnitude, unit: &str) -> Option<f64> {
    match magnitude {
        Magnitude::Count { value, unit: u } if u == unit => Some(*value),
        _ => None,
    }
}
