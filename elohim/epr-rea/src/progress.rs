//! Progress toward a promise's declared parts — how far each part has moved toward its target,
//! and how the parts compose into the whole.
//!
//! A [`crate::model::Commitment`] says WHAT is promised; its fulfillment events say when it was
//! discharged. Between the two there was no reading at all: a promise three-quarters of the way
//! there and one nobody has touched both read "open". This module is that reading, and it is
//! deliberately domain-neutral — a habit's checks, a pool charter's goals, a learning path's
//! mastery parts and a device's arrival conditions are all "declared parts, some measured, each
//! some distance from its target".
//!
//! **Admission** (requisite-variety guidestar §3a — a second independent framework must need the
//! same missing distinction). Three do. Meadows' balancing loop acts on the DISCREPANCY between a
//! stock and its goal, which is this module's gauge; Beer's System 3 reads achievement as
//! actuality over capability, which is this module's ratio; and VF/REA already reads partial
//! fulfillment, but only for a promise with ONE expected quantity
//! ([`crate::fold::FulfillmentStatus::ratio`]). What none of them had here was a promise of
//! several parts measured in different shapes. Not a second measure ontology (§6.3): a `Part` is a
//! gap-to-goal reading; the measure that produced it stays a middot record, the composition rule
//! is [`Composition`], the claim is [`ClaimKind`], the rate is a [`Quantity`].
//!
//! Three laws carry over from the rest of the crate:
//!
//! - **Honest absence.** An unmeasured part is `None`, never `0.0` or `1.0`. [`compose`] reports
//!   both readings a reader needs — the FLOOR, which counts unmeasured parts as not yet moved and
//!   so can only rise with evidence, and the value over measured parts with its coverage — so
//!   measuring one easy part of eleven cannot inflate the whole.
//! - **Composition is declared, never defaulted silently.** The parts compose by
//!   [`Composition`], the same rule bounds roll up by: `sum` (equal shares of the whole), `min`
//!   (Liebig — the whole is as far along as its least-advanced part) or `harmonic`
//!   (worst-off-sensitive — one stalled part cannot be averaged away).
//! - **Deterioration is not hidden by a clamp.** A gauge that moved past its baseline in the
//!   wrong direction reads progress `0.0`, AND reports how far past it went.

use elohim_epr::measure::{ClaimKind, Confidence, Interval, MeasureKind, Period, Quantity};
use serde::{Deserialize, Serialize};

use crate::model::Composition;

/// One part's reading, in the shape its measurement takes.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Part {
    /// `passed` of `declared` — scenarios, checks, steps. Whatever did not run counts in
    /// `declared` and not in `passed`: a skipped scenario is not a green one.
    Ratio { passed: f64, declared: f64 },
    /// A number moving from `baseline` toward `target`. The direction is the sign of
    /// `target - baseline`, so lower-is-better and higher-is-better need no extra field.
    Gauge {
        baseline: f64,
        target: f64,
        reading: f64,
    },
    /// A fixed assertion that holds or does not.
    Check { holds: bool },
}

#[derive(Debug, Clone, thiserror::Error, PartialEq)]
pub enum ProgressError {
    #[error("a ratio needs 0 <= passed <= declared and declared > 0, got {passed}/{declared}")]
    Ratio { passed: f64, declared: f64 },
    #[error(
        "a gauge needs a finite reading and finite baseline != target, got baseline {baseline}, \
         target {target}, reading {reading}"
    )]
    Gauge {
        baseline: f64,
        target: f64,
        reading: f64,
    },
    #[error("a composed reading must be a progress fraction in [0, 1], got {0}")]
    NotAFraction(f64),
    #[error("a recurrence must be a finite non-negative count and a scale finite and positive")]
    Recurrence,
}

impl Part {
    /// Check the reading's own rules. Never panics.
    pub fn validate(&self) -> Result<(), ProgressError> {
        match *self {
            Part::Ratio { passed, declared } => {
                let ok = passed.is_finite()
                    && declared.is_finite()
                    && declared > 0.0
                    && (0.0..=declared).contains(&passed);
                ok.then_some(())
                    .ok_or(ProgressError::Ratio { passed, declared })
            }
            Part::Gauge {
                baseline,
                target,
                reading,
            } => {
                let ok = baseline.is_finite()
                    && target.is_finite()
                    && reading.is_finite()
                    && baseline != target;
                ok.then_some(()).ok_or(ProgressError::Gauge {
                    baseline,
                    target,
                    reading,
                })
            }
            Part::Check { .. } => Ok(()),
        }
    }

    /// How far this part has moved toward its target, in `[0, 1]`.
    pub fn progress(&self) -> Result<f64, ProgressError> {
        self.validate()?;
        Ok(match *self {
            Part::Ratio { passed, declared } => passed / declared,
            Part::Gauge { .. } => self.signed_progress().clamp(0.0, 1.0),
            Part::Check { holds } => f64::from(u8::from(holds)),
        })
    }

    /// How far a gauge has moved PAST its baseline in the wrong direction, as a fraction of the
    /// baseline→target span. `None` for every reading that has not deteriorated, and for every
    /// part that is not a gauge (a ratio or a check cannot fall below its own zero).
    pub fn deterioration(&self) -> Option<f64> {
        match self {
            Part::Gauge { .. } if self.validate().is_ok() => {
                let signed = self.signed_progress();
                (signed < 0.0).then_some(-signed)
            }
            _ => None,
        }
    }

    fn signed_progress(&self) -> f64 {
        match *self {
            Part::Gauge {
                baseline,
                target,
                reading,
            } => (reading - baseline) / (target - baseline),
            _ => 0.0,
        }
    }
}

/// How a declared whole stands, given readings for some of its parts.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Composed {
    pub rule: Composition,
    /// Parts the whole declares.
    pub declared: usize,
    /// Parts that carry a reading.
    pub measured: usize,
    /// Unmeasured parts read as not yet moved. The headline number: it rises only with evidence.
    pub floor: f64,
    /// The rule applied to measured parts only. `None` when nothing was measured — no parts is
    /// no answer, not a whole at zero.
    pub over_measured: Option<f64>,
}

/// Compose part readings (`None` = unmeasured) into the whole's progress by `rule`.
///
/// `sum` reads each part as an equal share of the whole, so its result is the mean; `min` and
/// `harmonic` are [`Composition::fold`] unchanged — including its law that a zero part zeroes a
/// harmonic whole, which is exactly why an unmeasured part (read as `0.0` for the floor) pulls a
/// `min` or `harmonic` floor to zero.
///
/// A whole that declares no parts composes to a floor of `0.0` and no measured value: a promise
/// with nothing to measure has not moved, and saying so is the reading.
pub fn compose(rule: Composition, readings: &[Option<f64>]) -> Result<Composed, ProgressError> {
    if let Some(bad) = readings
        .iter()
        .flatten()
        .find(|p| !p.is_finite() || !(0.0..=1.0).contains(*p))
    {
        return Err(ProgressError::NotAFraction(*bad));
    }
    let declared = readings.len();
    let measured: Vec<f64> = readings.iter().flatten().copied().collect();
    let floor_parts: Vec<f64> = readings.iter().map(|p| p.unwrap_or(0.0)).collect();
    Ok(Composed {
        rule,
        declared,
        measured: measured.len(),
        floor: whole(rule, &floor_parts).unwrap_or(0.0),
        over_measured: whole(rule, &measured),
    })
}

/// One rule over non-empty fractions; `None` when there are no parts.
fn whole(rule: Composition, parts: &[f64]) -> Option<f64> {
    // `fold` only refuses empty or negative/non-finite input, which `compose` has excluded.
    let folded = rule.fold(parts).ok()?;
    Some(match rule {
        Composition::Sum => folded / parts.len() as f64,
        Composition::Min | Composition::Harmonic => folded,
    })
}

/// What a lower-is-better gauge RETURNS each period: the distance it moved below its baseline,
/// times how often the measured thing recurs, as a [`MeasureKind::Rate`].
///
/// `scale` converts the gauge's unit into the unit the rate is denominated in (`1.0 / 3600.0`
/// turns a gauge in seconds into hours); the unit itself lives in `basis`, because a
/// [`Quantity`] carries its kind and its confidence but not its unit.
///
/// `None` when nothing is returned: a part that is not a gauge, a higher-is-better gauge (a
/// throughput rising is not time given back), or a reading that has not moved below its
/// baseline. A deteriorated reading returns nothing rather than a negative saving — the cost of
/// deterioration is [`Part::deterioration`]'s to report, not a debit hidden in this rate.
///
/// The claim is the caller's, and should be the weaker of the reading's and the recurrence's:
/// a measured reading times an estimated frequency is an estimate.
pub fn recurring_return(
    part: &Part,
    scale: f64,
    occurrences: f64,
    per: Period,
    claim: ClaimKind,
    basis: impl Into<String>,
) -> Result<Option<Quantity>, ProgressError> {
    if !scale.is_finite() || scale <= 0.0 || !occurrences.is_finite() || occurrences < 0.0 {
        return Err(ProgressError::Recurrence);
    }
    let Part::Gauge {
        baseline,
        target,
        reading,
    } = *part
    else {
        return Ok(None);
    };
    part.validate()?;
    if target >= baseline || reading >= baseline {
        return Ok(None);
    }
    let value = (baseline - reading) * scale * occurrences;
    let confidence = match claim {
        ClaimKind::Witnessed => Confidence::witnessed(Interval::exact(value), basis),
        _ => Confidence {
            claim,
            ..Confidence::estimated(Interval::exact(value), basis)
        },
    };
    Ok(Some(Quantity {
        value,
        kind: MeasureKind::Rate { per },
        confidence,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stock::coverage_time;

    fn ratio(passed: f64, declared: f64) -> Part {
        Part::Ratio { passed, declared }
    }

    #[test]
    fn a_ratio_counts_unrun_parts_as_not_passed() {
        assert_eq!(ratio(3.0, 4.0).progress(), Ok(0.75));
        assert!(ratio(5.0, 4.0).progress().is_err());
        assert!(ratio(0.0, 0.0).progress().is_err());
    }

    #[test]
    fn a_gauge_reads_direction_from_its_sign_and_reports_deterioration_past_the_clamp() {
        // Lower is better: 43 s toward 12 s.
        let improved = Part::Gauge {
            baseline: 43.0,
            target: 12.0,
            reading: 10.0,
        };
        assert_eq!(improved.progress(), Ok(1.0));
        assert_eq!(improved.deterioration(), None);
        let half = Part::Gauge {
            baseline: 40.0,
            target: 20.0,
            reading: 30.0,
        };
        assert_eq!(half.progress(), Ok(0.5));
        // Higher is better: 2 toward 10, read at 6.
        let rising = Part::Gauge {
            baseline: 2.0,
            target: 10.0,
            reading: 6.0,
        };
        assert_eq!(rising.progress(), Ok(0.5));
        // Worse than where it started: progress clamps to zero, the deterioration is named.
        let worse = Part::Gauge {
            baseline: 40.0,
            target: 20.0,
            reading: 50.0,
        };
        assert_eq!(worse.progress(), Ok(0.0));
        assert_eq!(worse.deterioration(), Some(0.5));
        let flat = Part::Gauge {
            baseline: 5.0,
            target: 5.0,
            reading: 5.0,
        };
        assert!(flat.progress().is_err());
    }

    #[test]
    fn a_check_is_all_or_nothing() {
        assert_eq!(Part::Check { holds: true }.progress(), Ok(1.0));
        assert_eq!(Part::Check { holds: false }.progress(), Ok(0.0));
    }

    #[test]
    fn the_floor_counts_unmeasured_parts_and_the_measured_value_carries_its_coverage() {
        // One easy part measured of four: the floor stays honest, the measured value says why.
        let readings = [Some(1.0), None, None, None];
        let sum = compose(Composition::Sum, &readings).unwrap();
        assert_eq!(sum.floor, 0.25);
        assert_eq!(sum.over_measured, Some(1.0));
        assert_eq!((sum.measured, sum.declared), (1, 4));
    }

    #[test]
    fn min_and_harmonic_floors_fall_to_zero_on_any_unmeasured_part() {
        let readings = [Some(0.5), Some(1.0), None];
        assert_eq!(compose(Composition::Min, &readings).unwrap().floor, 0.0);
        assert_eq!(
            compose(Composition::Harmonic, &readings).unwrap().floor,
            0.0
        );
        let measured = [Some(0.5), Some(1.0)];
        assert_eq!(compose(Composition::Min, &measured).unwrap().floor, 0.5);
        let harmonic = compose(Composition::Harmonic, &measured).unwrap().floor;
        assert!((harmonic - 2.0 / 3.0).abs() < 1e-12, "{harmonic}");
    }

    #[test]
    fn no_parts_is_a_floor_of_zero_and_no_measured_answer() {
        let none = compose(Composition::Sum, &[]).unwrap();
        assert_eq!(
            (none.floor, none.over_measured, none.declared),
            (0.0, None, 0)
        );
        let unmeasured = compose(Composition::Sum, &[None, None]).unwrap();
        assert_eq!((unmeasured.floor, unmeasured.over_measured), (0.0, None));
    }

    #[test]
    fn compose_refuses_a_reading_that_is_not_a_fraction() {
        assert_eq!(
            compose(Composition::Sum, &[Some(1.5)]),
            Err(ProgressError::NotAFraction(1.5))
        );
        assert!(compose(Composition::Sum, &[Some(f64::NAN)]).is_err());
    }

    #[test]
    fn a_shortened_wait_returns_time_and_pays_back_what_it_cost() {
        // 43 s -> 10 s, forty times a week, in hours.
        let gauge = Part::Gauge {
            baseline: 43.0,
            target: 12.0,
            reading: 10.0,
        };
        let returned = recurring_return(
            &gauge,
            1.0 / 3600.0,
            40.0,
            Period::Week,
            ClaimKind::Estimated,
            "hours/week returned",
        )
        .unwrap()
        .expect("a lower-is-better gauge below its baseline returns time");
        assert!((returned.value - 33.0 * 40.0 / 3600.0).abs() < 1e-12);
        assert_eq!(returned.kind, MeasureKind::Rate { per: Period::Week });
        assert_eq!(returned.confidence.claim, ClaimKind::Estimated);

        // Eleven hours spent; payback is a time in the rate's period, carrying the weaker claim.
        let spent = Quantity {
            value: 11.0,
            kind: MeasureKind::Level,
            confidence: Confidence::witnessed(Interval::exact(11.0), "hours spent"),
        };
        let payback = coverage_time(&spent, &returned).unwrap();
        assert!((payback.value - 30.0).abs() < 1e-9, "{}", payback.value);
        assert_eq!(payback.kind, MeasureKind::Level);
        assert_eq!(payback.confidence.claim, ClaimKind::Estimated);
    }

    #[test]
    fn nothing_returned_is_none_not_a_negative_saving() {
        let rising = Part::Gauge {
            baseline: 2.0,
            target: 10.0,
            reading: 6.0,
        };
        let worse = Part::Gauge {
            baseline: 40.0,
            target: 20.0,
            reading: 50.0,
        };
        for part in [rising, worse, Part::Check { holds: true }] {
            let got = recurring_return(&part, 1.0, 5.0, Period::Week, ClaimKind::Estimated, "x");
            assert_eq!(got, Ok(None), "{part:?}");
        }
        let gauge = Part::Gauge {
            baseline: 4.0,
            target: 1.0,
            reading: 2.0,
        };
        assert!(
            recurring_return(&gauge, 0.0, 5.0, Period::Week, ClaimKind::Estimated, "x").is_err()
        );
    }

    #[test]
    fn payback_over_a_rate_that_admits_zero_is_unknown_not_infinite() {
        let spent = Quantity {
            value: 3.0,
            kind: MeasureKind::Level,
            confidence: Confidence::witnessed(Interval::exact(3.0), "spent"),
        };
        let nothing = Quantity {
            value: 0.0,
            kind: MeasureKind::Rate { per: Period::Week },
            confidence: Confidence::witnessed(Interval::exact(0.0), "returned"),
        };
        let payback = coverage_time(&spent, &nothing).unwrap();
        assert!(payback.value.is_nan());
        assert!(
            coverage_time(&nothing, &spent).is_err(),
            "a rate over a level is not a time"
        );
    }
}
