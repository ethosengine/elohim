//! Slice 2: the dimensional algebra and the interval arithmetic a stock model needs, plus the
//! two open questions whose closure the stock work depends on (spec Q10, Q12).
//!
//! Slice 1 made the honest states EXPRESSIBLE. These tests are about making the dishonest
//! *operations* unrepresentable: the shipped `spatial_capacity.rs` defect is not a bad number,
//! it is a division nobody could have written down as wrong, because nothing knew the operands'
//! kinds. Once `divide` exists, that division still computes — it just returns a TIME, and a
//! time cannot be read as a utilization ratio without someone noticing.

use elohim_epr::measure::*;

// ------------------------------------------------------------- dimensional algebra

#[test]
fn two_flows_of_the_same_tempo_divide_into_a_dimensionless_index() {
    // Meadows' harvest/regeneration, emission/absorption, and the respite/response
    // controllability index are ONE shape, which is why there is one operation and not three.
    let per_day = MeasureKind::Rate { per: Period::Day };
    assert_eq!(per_day.divide(per_day), Some(MeasureKind::Ratio));
}

#[test]
fn flows_of_different_tempo_refuse_to_divide() {
    // A per-day rate over a per-year rate is 365x from whatever the caller expected. Silently
    // converting here would bury that assumption in a helper nobody reads twice.
    let day = MeasureKind::Rate { per: Period::Day };
    let year = MeasureKind::Rate { per: Period::Year };
    assert_eq!(day.divide(year), None);
    assert_eq!(year.divide(day), None);
}

#[test]
fn a_stock_over_a_drain_is_a_time_not_a_utilization() {
    // THE SHIPPED DEFECT, made non-silent. spatial_capacity.rs divides a cumulative level by
    // max_sustainable_yield (a rate) and reads the result as utilization against 1.0. Under this
    // algebra the operation is legal and the RESULT IS A LEVEL DENOMINATED IN TIME — coverage
    // time, which is never a utilization. The arithmetic is unchanged; the misreading is what
    // becomes visible.
    let level = MeasureKind::Level;
    let yield_per_year = MeasureKind::Rate { per: Period::Year };
    assert_eq!(level.divide(yield_per_year), Some(MeasureKind::Level));
}

#[test]
fn a_ratio_in_either_position_is_refused() {
    // An index of indices has real uses and no settled reading here. Inventing one inside a
    // dimensional-SAFETY helper is how the safety helper becomes the next unit error's source.
    let ratio = MeasureKind::Ratio;
    let day = MeasureKind::Rate { per: Period::Day };
    assert_eq!(ratio.divide(day), None);
    assert_eq!(day.divide(ratio), None);
    assert_eq!(ratio.divide(ratio), None);
}

#[test]
fn only_like_combines_additively_and_the_period_is_part_of_likeness() {
    let day = MeasureKind::Rate { per: Period::Day };
    let week = MeasureKind::Rate { per: Period::Week };
    assert_eq!(day.combine_additive(day), Some(day));
    // Net change subtracts two flows; a period mismatch there produces a number that is per
    // neither, and nothing downstream would ever say so.
    assert_eq!(day.combine_additive(week), None);
    assert_eq!(MeasureKind::Level.combine_additive(day), None);
}

// ------------------------------------------------------------- Q12

#[test]
fn exact_infinity_no_longer_reports_as_honest_absence() {
    // Spec Q12. `is_unknown()` checked only `is_infinite()` on both bounds, so [+inf, +inf] —
    // an assertion that a quantity is EXACTLY infinite, the most confident claim expressible —
    // read as "unmeasured". The detector L3 depends on could not distinguish absence from
    // certainty at infinity, which is the one distinction it exists to make.
    let manufactured = Interval::new(f64::INFINITY, f64::INFINITY);
    assert!(
        !manufactured.is_unknown(),
        "[+inf,+inf] is a maximal claim, not an absence"
    );
    assert!(!Interval::new(f64::NEG_INFINITY, f64::NEG_INFINITY).is_unknown());
    assert!(Interval::unknown().is_unknown(), "(-inf,+inf) still holds");
}

// ------------------------------------------------------------- Q10

#[test]
fn multiplier_widening_yields_absence_at_a_zero_base_rather_than_a_zero_width_band() {
    // Spec Q10, generalized off the one call site that hit it. `scalar × [1,3]` is the natural
    // way to say "we counted N, the truth is 1-3x that" — and at N == 0 both products are 0, so
    // the band collapses to zero width at exactly the moment the count is least trustworthy.
    let wide = Interval::multiplier_widen(4.0, 1.0, 3.0);
    assert_eq!(wide, Interval::new(4.0, 12.0));
    assert!(wide.width() > 0.0);

    let at_zero = Interval::multiplier_widen(0.0, 1.0, 3.0);
    assert!(
        at_zero.is_unknown(),
        "a multiplier cannot manufacture width from a zero base — say so"
    );
    // And an inverted multiplier pair is refused rather than silently reordered: the caller
    // meant something we cannot guess.
    assert!(Interval::multiplier_widen(4.0, 3.0, 1.0).is_unknown());
}

// ------------------------------------------------------------- interval arithmetic

#[test]
fn dividing_by_a_band_that_admits_zero_is_unknown_not_infinite() {
    // The structural cure for the bug slice 1 patched by hand at one call site. A stock whose
    // outflow band includes zero has genuinely unbounded turnover; "unknown" is true, and
    // "+infinity" is a claim.
    let level = Interval::exact(300.0);
    assert!(level.div(&Interval::new(0.0, 5.0)).is_unknown());
    assert!(level.div(&Interval::exact(0.0)).is_unknown());
    assert!(level.div(&Interval::new(-1.0, 1.0)).is_unknown());
    // A band strictly away from zero divides normally, widest-quotient-first.
    assert_eq!(
        level.div(&Interval::new(2.0, 3.0)),
        Interval::new(100.0, 150.0)
    );
}

#[test]
fn division_never_manufactures_exact_infinity_from_finite_inputs() {
    // The invariant that makes Q12's detector meaningful again, stated at the strength it
    // actually holds. The FIRST draft of this test asserted that no input produces [+inf,+inf]
    // and failed on `[+inf,+inf] / [2,2]` — correctly, because that quotient IS exactly
    // infinity. The input had already made the maximal claim; division propagated it faithfully.
    //
    // That distinction is the point rather than a caveat. `div` launders nothing: a caller who
    // asserts exact infinity keeps owning that assertion downstream, and Q12's detector will
    // now refuse to read it as absence wherever it surfaces. What `div` guarantees is narrower
    // and is the guarantee the stock model needs — it never CREATES that shape from operands
    // that did not already contain it.
    let cases = [
        (Interval::exact(1.0), Interval::exact(0.0)),
        (Interval::exact(1.0), Interval::new(0.0, 5.0)),
        (Interval::new(1e300, 1e300), Interval::new(1e-300, 1e-300)),
        (Interval::unknown(), Interval::exact(2.0)),
        (Interval::exact(1.0), Interval::unknown()),
        (Interval::unknown(), Interval::unknown()),
    ];
    for (n, d) in cases {
        let q = n.div(&d);
        assert!(
            q.is_unknown() || q.is_wellformed(),
            "division produced a malformed interval: {n:?} / {d:?} = {q:?}"
        );
        assert!(
            !(q.lo == f64::INFINITY && q.hi == f64::INFINITY),
            "division manufactured exact infinity from {n:?} / {d:?}"
        );
    }
    // Faithful propagation, asserted rather than merely tolerated.
    let maximal = Interval::new(f64::INFINITY, f64::INFINITY);
    assert_eq!(maximal.div(&Interval::exact(2.0)), maximal);
    assert!(!maximal.div(&Interval::exact(2.0)).is_unknown());
}

#[test]
fn float_overflow_is_reported_as_a_bound_we_have_not_as_infinity_we_claim() {
    // `1e300 / 1e-300` is ~1e600: unrepresentable in f64, so the hardware answers +inf. Under
    // L3 that reads as an assertion of EXACT infinity manufactured entirely by the type's range
    // — a claim nobody made, from two perfectly finite operands. Found by the test above.
    //
    // Both easy answers are wrong. [+inf,+inf] asserts exactness we do not have;
    // Interval::unknown() asserts ignorance we do not have either, since we know the quotient
    // exceeds f64::MAX. The honest shape is the one that says both things at once.
    let q = Interval::new(1e300, 1e300).div(&Interval::new(1e-300, 1e-300));
    assert_eq!(
        q.lo,
        f64::MAX,
        "we know the quotient is at least this large"
    );
    assert_eq!(q.hi, f64::INFINITY, "and we cannot bound it above");
    assert!(!q.is_unknown(), "this is a real bound, not an absence");
    assert!(q.is_wellformed());
}

#[test]
fn subtraction_over_widens_rather_than_narrowing() {
    // Correlation-naive (spec Q1) and deliberately so: over-widening is the safe direction
    // because it can never manufacture precision. Net change is the one subtraction a stock
    // model performs, and it is where a too-tight band would read as false confidence.
    let inflow = Interval::new(10.0, 12.0);
    let outflow = Interval::new(3.0, 5.0);
    let net = inflow.sub(&outflow);
    assert_eq!(net, Interval::new(5.0, 9.0));
    assert!(net.width() >= inflow.width());
    // Infinity at a bound cancels to NaN under naive arithmetic; that must surface as absence.
    assert!(Interval::unknown().sub(&Interval::unknown()).is_unknown());
}

#[test]
fn malformed_intervals_propagate_as_absence_not_as_nan() {
    // Q9 stays open — `new` still accepts a malformed pair. What is closed here is the SILENT
    // path: NaN satisfies no comparison, so a NaN bound answers `false` to every containment
    // check and looks like a well-behaved empty interval to every downstream reader.
    let malformed = Interval::new(5.0, 1.0);
    assert!(!malformed.is_wellformed());
    assert!(malformed.div(&Interval::exact(2.0)).is_unknown());
    assert!(malformed.sub(&Interval::exact(2.0)).is_unknown());
    assert!(Interval::exact(2.0).div(&malformed).is_unknown());
}

// ------------------------------------------- Q17: an unknown that says WHY it is unknown

#[test]
fn every_arithmetic_path_that_mints_an_unknown_now_says_which_one_it_minted() {
    // One assertion per mint site in `measure.rs`. Before Q17 all seven returned the identical
    // `(-inf,+inf)`, so a reader downstream — including the uncertainty work-queue that is the
    // whole point of decomposing an aggregate — could not tell a bug from a structural
    // unboundedness from an empty window. The reason was known AT EACH of these lines and thrown
    // away one statement later.
    let malformed = Interval::new(5.0, 1.0);
    let ok = Interval::exact(2.0);

    // div: three genuinely different absences, previously one.
    assert_eq!(
        malformed.div_because(&ok).reason,
        Some(UnknownReason::MalformedInput),
        "lo > hi is an upstream defect, not an absence of observation"
    );
    assert_eq!(
        Interval::exact(300.0)
            .div_because(&Interval::new(0.0, 5.0))
            .reason,
        Some(UnknownReason::UndefinedDivision),
        "a denominator band admitting zero is a structure finding about the DENOMINATOR"
    );
    assert_eq!(
        Interval::new(1.0, f64::INFINITY)
            .div_because(&Interval::new(2.0, f64::INFINITY))
            .reason,
        Some(UnknownReason::IndeterminateForm),
        "inf/inf at a bound, with a denominator that stays clear of zero"
    );
    // Written first as `unknown / unknown` and CORRECTED by this assertion: an unknown
    // denominator admits zero, so the honest reason is the zero-crossing one, which is checked
    // (and must stay checked) before the indeterminate-form probe. The ordering of the guards is
    // itself a claim about which explanation is the more specific, and this pins it.
    assert_eq!(
        Interval::unknown().div_because(&Interval::unknown()).reason,
        Some(UnknownReason::UndefinedDivision)
    );

    // sub: malformed vs. the indeterminate form, which are not the same problem.
    assert_eq!(
        malformed.sub_because(&ok).reason,
        Some(UnknownReason::MalformedInput)
    );
    assert_eq!(
        Interval::new(f64::INFINITY, f64::INFINITY)
            .sub_because(&Interval::new(3.0, f64::INFINITY))
            .reason,
        Some(UnknownReason::IndeterminateForm),
        "inf - inf at a bound"
    );
    // Also corrected by running it: `unknown - unknown` does NOT reach that guard. Its bounds
    // are (-inf) - (+inf) and (+inf) - (-inf), which are -inf and +inf — no NaN anywhere, so the
    // result is unknown by honest arithmetic rather than by refusal, and `None` is the truthful
    // reason. The refusal path needs an operand that already asserted a one-sided infinity.
    let both_unknown = Interval::unknown().sub_because(&Interval::unknown());
    assert!(both_unknown.is_unknown());
    assert_eq!(both_unknown.reason, None);

    // multiplier_widen: the zero base is its own reason and must not read as "no observations."
    // A zero count IS a count; sampling harder returns zero again.
    assert_eq!(
        Interval::multiplier_widen_because(0.0, 1.0, 3.0).reason,
        Some(UnknownReason::ZeroBase)
    );
    assert_eq!(
        Interval::multiplier_widen_because(4.0, 3.0, 1.0).reason,
        Some(UnknownReason::MalformedInput),
        "an inverted multiplier pair is a caller defect, not an absence"
    );
    assert_eq!(
        Interval::multiplier_widen_because(f64::INFINITY, 0.0, 1.0).reason,
        Some(UnknownReason::IndeterminateForm),
        "0 x inf is the one indeterminate product the zero-scalar guard does not already catch"
    );

    // And the success path carries no reason at all. Note this is the invariant's ONE direction
    // (a reason implies an unknown interval) holding at a bounded result — NOT its converse,
    // which `both_unknown` above already refuted.
    let fine = Interval::multiplier_widen_because(4.0, 1.0, 3.0);
    assert_eq!(fine.interval, Interval::new(4.0, 12.0));
    assert_eq!(fine.reason, None);
    assert_eq!(Interval::exact(300.0).div_because(&ok).reason, None);
}

#[test]
fn the_declared_reasons_no_arithmetic_can_mint_are_still_expressible() {
    // NoObservations / NotYetInstrumented / Incommensurable are DECLARED by whoever knows the
    // observation situation — arithmetic cannot derive them, and inventing an arithmetic path
    // that guessed at them would be exactly the false precision this ontology refuses.
    for reason in [
        UnknownReason::NoObservations,
        UnknownReason::NotYetInstrumented,
        UnknownReason::Incommensurable,
    ] {
        let r = Interval::unknown_because(reason);
        assert!(r.is_unknown());
        assert_eq!(r.interval, Interval::unknown(), "same bounds, more context");
        assert_eq!(r.reason, Some(reason));
    }
}

#[test]
fn the_work_queue_can_tell_measurable_ignorance_from_the_kind_nothing_fixes() {
    // The classification measure-family-borrows-backlog row 18 cannot be built without. Three
    // unknowns, three different correct responses, and the middle one is the trap: a
    // zero-crossing denominator LOOKS like missing data and is not.
    assert_eq!(
        UnknownReason::NoObservations.tightenable(),
        Tightenable::ByMeasurement,
        "the instrument exists and the window was empty — go measure; queue it"
    );
    assert_eq!(
        UnknownReason::NotYetInstrumented.tightenable(),
        Tightenable::ByMeasurement,
        "row 18's stated purpose: the system's own ignorance names where to add senses"
    );
    assert_eq!(
        UnknownReason::UndefinedDivision.tightenable(),
        Tightenable::ByStructure,
        "sampling the numerator forever leaves a zero-crossing denominator unbounded"
    );
    assert_eq!(
        UnknownReason::IndeterminateForm.tightenable(),
        Tightenable::ByStructure
    );
    assert_eq!(
        UnknownReason::ZeroBase.tightenable(),
        Tightenable::ByStructure
    );
    assert_eq!(
        UnknownReason::MalformedInput.tightenable(),
        Tightenable::ByStructure,
        "a bug belongs in a defect queue, never in a measurement queue"
    );
    assert_eq!(
        UnknownReason::Incommensurable.tightenable(),
        Tightenable::Never,
        "drift_score's own confession (spec Q16): no observation narrows a preference ordering"
    );

    // The distinction stated as the queue would consume it, because the failure this prevents is
    // specific: ranking "go measure the incommensurable composite harder" at the top.
    let starved = Interval::unknown_because(UnknownReason::NoObservations);
    let hopeless = Interval::unknown_because(UnknownReason::Incommensurable);
    assert_eq!(
        starved.interval, hopeless.interval,
        "identical bounds — which is precisely why the bounds cannot be the discriminator"
    );
    assert_ne!(starved.reason, hopeless.reason);
    assert_ne!(
        starved.reason.unwrap().tightenable(),
        hopeless.reason.unwrap().tightenable()
    );
}

#[test]
fn unknown_still_means_exactly_what_it_meant_before_reasons_existed() {
    // The non-breaking guarantee, asserted rather than asserted-about. `unknown()` is unchanged
    // in bounds, in `is_unknown()`, in containment, and in what the reason-free arithmetic
    // returns — `unknown_because` is beside it, never in front of it.
    let u = Interval::unknown();
    assert_eq!(u.lo, f64::NEG_INFINITY);
    assert_eq!(u.hi, f64::INFINITY);
    assert!(u.is_unknown());
    assert!(u.contains(0.0) && u.contains(1e300) && u.contains(-1e300));

    // Every `-> Interval` signature still returns an `Interval` and still returns the same one.
    assert!(Interval::unknown().sub(&Interval::unknown()).is_unknown());
    assert!(Interval::exact(300.0)
        .div(&Interval::new(0.0, 5.0))
        .is_unknown());
    assert!(Interval::multiplier_widen(0.0, 1.0, 3.0).is_unknown());
    assert_eq!(
        Interval::multiplier_widen(4.0, 1.0, 3.0),
        Interval::new(4.0, 12.0)
    );
    assert_eq!(
        Interval::exact(300.0).div(&Interval::new(2.0, 3.0)),
        Interval::new(100.0, 150.0)
    );
    assert_eq!(
        Interval::new(10.0, 12.0).sub(&Interval::new(3.0, 5.0)),
        Interval::new(5.0, 9.0)
    );

    // And each of those is the `.interval` of the reasoned form — one implementation, two views,
    // so the pair can never drift from the plain call the way two copies of the logic would.
    assert_eq!(
        Interval::exact(300.0).div(&Interval::new(2.0, 3.0)),
        Interval::exact(300.0)
            .div_because(&Interval::new(2.0, 3.0))
            .interval
    );
    assert_eq!(
        Interval::unknown_because(UnknownReason::NoObservations).interval,
        Interval::unknown()
    );
}

#[test]
fn a_missing_reason_does_not_license_reading_bounds() {
    // The `Reasoned` invariant holds in ONE direction only, and a reader who assumes the other
    // dereferences infinity. `reason.is_some()` does imply `interval.is_unknown()` — every
    // constructor upholds that. The converse is false: `Reasoned::known` is public and accepts
    // any interval, and the arithmetic routes a genuinely unbounded result through it whenever no
    // guard fires (`unknown - unknown` computes cleanly to (-inf, +inf) with nothing to explain).
    //
    // So `reason.is_none()` means "nothing named a reason", NOT "the bounds are finite". This
    // test exists because the doc on `Reasoned` claimed an `iff` — under which a consumer could
    // legitimately treat a reasonless outcome as bounded and hand `lo`/`hi` to arithmetic that
    // has no answer for +-inf.
    let reasonless_unknown = Interval::unknown().sub_because(&Interval::unknown());
    assert_eq!(reasonless_unknown.reason, None);
    assert!(
        reasonless_unknown.is_unknown(),
        "no reason, and yet no bounds — the iff would have to be false here"
    );
    assert!(!reasonless_unknown.interval.lo.is_finite());
    assert!(!reasonless_unknown.interval.hi.is_finite());

    // Reasoned::known is the public door that makes this reachable, not an internal accident.
    let hand_built = Reasoned::known(Interval::unknown());
    assert_eq!(hand_built.reason, None);
    assert!(hand_built.is_unknown());

    // The direction that DOES hold, over every variant: a reason never accompanies bounds.
    for reason in [
        UnknownReason::NoObservations,
        UnknownReason::NotYetInstrumented,
        UnknownReason::UndefinedDivision,
        UnknownReason::IndeterminateForm,
        UnknownReason::ZeroBase,
        UnknownReason::MalformedInput,
        UnknownReason::Incommensurable,
    ] {
        let r = Interval::unknown_because(reason);
        assert!(r.is_unknown(), "{reason:?} must ride an unknown interval");
    }
}

#[test]
fn the_reason_reduction_is_a_total_order_so_a_multiset_has_one_answer() {
    // `UnknownReason::reduce` is the single rule every combining site uses (the fold across a
    // sum's terms, `Reasoned::attributed_to` across an operation's operands). It has to be
    // order-independent or two peers reducing the same multiset mint different values, and
    // `tightenable()` alone cannot deliver that: seven reasons onto three responses is
    // non-injective, so distinct reasons tie and `max_by_key` resolves ties by position.
    let all = [
        UnknownReason::NoObservations,
        UnknownReason::NotYetInstrumented,
        UnknownReason::UndefinedDivision,
        UnknownReason::IndeterminateForm,
        UnknownReason::ZeroBase,
        UnknownReason::MalformedInput,
        UnknownReason::Incommensurable,
    ];

    // stable_index is injective over the whole vocabulary — that is the property the tie-break
    // rests on, and a future variant added with a duplicated index would break it silently.
    let mut seen: Vec<u8> = all.iter().map(|r| r.stable_index()).collect();
    seen.sort_unstable();
    seen.dedup();
    assert_eq!(seen.len(), all.len(), "stable_index must be injective");

    // Every unordered pair reduces the same way from either side.
    for (i, a) in all.iter().enumerate() {
        for b in &all[i..] {
            assert_eq!(
                UnknownReason::reduce([*a, *b]),
                UnknownReason::reduce([*b, *a]),
                "{a:?} vs {b:?} resolved by argument order"
            );
        }
    }

    // Least tightenable wins across ranks, and an empty input claims nothing.
    assert_eq!(
        UnknownReason::reduce([
            UnknownReason::NoObservations,
            UnknownReason::Incommensurable
        ]),
        Some(UnknownReason::Incommensurable)
    );
    assert_eq!(
        UnknownReason::reduce([UnknownReason::NoObservations, UnknownReason::ZeroBase]),
        Some(UnknownReason::ZeroBase)
    );
    assert_eq!(UnknownReason::reduce([]), None, "no contributor, no claim");

    // And `attributed_to` refuses to attach one to a bounded outcome.
    let bounded = Interval::exact(10.0)
        .div_because(&Interval::exact(2.0))
        .attributed_to([Some(UnknownReason::Incommensurable), None]);
    assert_eq!(bounded.interval, Interval::exact(5.0));
    assert_eq!(bounded.reason, None);
}
