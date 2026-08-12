use elohim_epr::measure::*;
use elohim_epr_rea::fold;

fn est(v: f64, lo: f64, hi: f64) -> Quantity {
    Quantity {
        value: v,
        kind: MeasureKind::Level,
        confidence: Confidence::estimated(Interval::new(lo, hi), "fixture"),
    }
}
fn wit(v: f64) -> Quantity {
    Quantity {
        value: v,
        kind: MeasureKind::Level,
        confidence: Confidence::witnessed(Interval::exact(v), "fixture"),
    }
}

#[test]
fn a_fold_over_intervals_returns_an_interval() {
    let out = fold::with_uncertainty(&[est(10.0, 8.0, 12.0), est(20.0, 18.0, 22.0)]).unwrap();
    assert_eq!(out.value, 30.0);
    assert_eq!(out.confidence.interval, Interval::new(26.0, 34.0));
}

#[test]
fn the_aggregate_takes_the_weakest_claim_of_its_inputs() {
    // One estimate makes the whole sum an estimate. False precision is the
    // mechanism behind "lies, damn lies, and statistics".
    let out = fold::with_uncertainty(&[wit(10.0), est(20.0, 18.0, 22.0)]).unwrap();
    assert_eq!(out.confidence.claim, ClaimKind::Estimated);
}

#[test]
fn one_unknown_term_makes_the_aggregate_unknown_not_wrong() {
    let unknown = Quantity {
        value: 0.0,
        kind: MeasureKind::Level,
        confidence: Confidence::unknown("never measured"),
    };
    let out = fold::with_uncertainty(&[wit(10.0), unknown]).unwrap();
    assert!(
        out.confidence.interval.is_unknown(),
        "an unmeasured term must not silently contribute zero"
    );
}

#[test]
fn folding_mixed_kinds_is_refused() {
    let level = wit(10.0);
    let rate = Quantity {
        value: 5.0,
        kind: MeasureKind::Rate { per: Period::Day },
        confidence: Confidence::witnessed(Interval::exact(5.0), "fixture"),
    };
    // This is the spatial_capacity.rs defect as a compile-adjacent guard:
    // a cumulative level and a per-day rate have no honest sum.
    assert!(fold::with_uncertainty(&[level, rate]).is_err());
}

#[test]
fn folding_is_deterministic_under_input_reordering() {
    // Sibling of the bound_stock determinism decision: the closure law is
    // worthless if two peers fold to different intervals.
    let a = [
        est(10.0, 8.0, 12.0),
        est(20.0, 18.0, 22.0),
        est(30.0, 29.0, 31.0),
    ];
    let mut b = a.clone();
    b.reverse();
    assert_eq!(
        fold::with_uncertainty(&a).unwrap(),
        fold::with_uncertainty(&b).unwrap()
    );
}

// ------------------------------------------- Q17: the reason survives the rollup

fn unknown_because(reason: UnknownReason) -> Quantity {
    Quantity {
        value: 0.0,
        kind: MeasureKind::Level,
        confidence: Confidence::unknown_because(reason, "fixture"),
    }
}

#[test]
fn a_fold_inherits_the_least_tightenable_reason_among_its_unknown_terms() {
    // An aggregate is no more improvable than its worst term. If one contributor is
    // incommensurable by construction, no measurement anywhere in the fold narrows the total —
    // so the total must say `Incommensurable` and not the friendlier `NoObservations` that
    // happened to be listed first. Getting this backwards is exactly how an uncertainty
    // work-queue ends up recommending "go measure the composite harder."
    let out = fold::with_uncertainty(&[
        wit(10.0),
        unknown_because(UnknownReason::NoObservations),
        unknown_because(UnknownReason::Incommensurable),
    ])
    .unwrap();
    assert!(out.confidence.interval.is_unknown());
    assert_eq!(
        out.confidence.unknown_reason,
        Some(UnknownReason::Incommensurable)
    );
    assert_eq!(
        out.confidence.unknown_reason.unwrap().tightenable(),
        Tightenable::Never,
        "and therefore the aggregate must never enter the measurement queue"
    );

    // Order-independent, same as the weakest-claim reduction beside it.
    let reversed = fold::with_uncertainty(&[
        unknown_because(UnknownReason::Incommensurable),
        unknown_because(UnknownReason::NoObservations),
        wit(10.0),
    ])
    .unwrap();
    assert_eq!(
        reversed.confidence.unknown_reason,
        out.confidence.unknown_reason
    );
}

#[test]
fn the_reason_reduction_is_order_independent_at_equal_tightenability() {
    // The case the test above CANNOT catch, and the one that actually broke.
    //
    // `folding_is_deterministic_under_input_reordering` and the reversal check above both vary
    // across DIFFERENT tightenable ranks, where the reduction has a strict maximum and any
    // tie-rule agrees. `tightenable()` is non-injective — seven reasons, three responses — so two
    // distinct reasons at the SAME rank tie, and `max_by_key` keeps the last maximum: keyed on
    // rank alone, `[ZeroBase, UndefinedDivision]` folded to `UndefinedDivision` and the reversed
    // multiset folded to `ZeroBase`. Both are `ByStructure`, so no ranking consumer would notice
    // — but the EMITTED FIELD differs, `Quantity` derives `PartialEq`, and two peers folding the
    // same terms in different orders therefore mint non-equal values. That is the determinism
    // property this file exists to hold, broken by an unranked tie rather than by float order.
    let forward = fold::with_uncertainty(&[
        unknown_because(UnknownReason::ZeroBase),
        unknown_because(UnknownReason::UndefinedDivision),
    ])
    .unwrap();
    let reverse = fold::with_uncertainty(&[
        unknown_because(UnknownReason::UndefinedDivision),
        unknown_because(UnknownReason::ZeroBase),
    ])
    .unwrap();
    assert_eq!(
        forward.confidence.unknown_reason, reverse.confidence.unknown_reason,
        "equal-rank reasons must resolve on a total order, not on input position"
    );
    assert_eq!(
        forward, reverse,
        "and the whole Quantity must be equal — this is what a peer compares"
    );

    // Every equal-rank pair in the `ByStructure` class, both directions. A tie-break that only
    // happened to work for one pair is not a total order.
    let structural = [
        UnknownReason::UndefinedDivision,
        UnknownReason::IndeterminateForm,
        UnknownReason::ZeroBase,
        UnknownReason::MalformedInput,
    ];
    for (i, a) in structural.iter().enumerate() {
        for b in &structural[i + 1..] {
            let ab = fold::with_uncertainty(&[unknown_because(*a), unknown_because(*b)]).unwrap();
            let ba = fold::with_uncertainty(&[unknown_because(*b), unknown_because(*a)]).unwrap();
            assert_eq!(ab, ba, "{a:?} vs {b:?} resolved by input order");
        }
    }
    // And the other equal-rank class.
    let m1 = unknown_because(UnknownReason::NoObservations);
    let m2 = unknown_because(UnknownReason::NotYetInstrumented);
    assert_eq!(
        fold::with_uncertainty(&[m1.clone(), m2.clone()]).unwrap(),
        fold::with_uncertainty(&[m2, m1]).unwrap()
    );
}

#[test]
fn a_repeated_reason_folds_to_itself_and_a_reordered_multiset_folds_identically() {
    // The reduction over a multiset with duplicates, which is the shape a real rollup has.
    let terms = [
        unknown_because(UnknownReason::NoObservations),
        unknown_because(UnknownReason::MalformedInput),
        unknown_because(UnknownReason::NoObservations),
        unknown_because(UnknownReason::ZeroBase),
    ];
    let mut shuffled = terms.clone();
    shuffled.reverse();
    let a = fold::with_uncertainty(&terms).unwrap();
    let b = fold::with_uncertainty(&shuffled).unwrap();
    assert_eq!(a.confidence.unknown_reason, b.confidence.unknown_reason);
    assert_eq!(
        a.confidence.unknown_reason.unwrap().tightenable(),
        Tightenable::ByStructure,
        "a structural term outranks the measurable ones regardless of how many there are"
    );
}

#[test]
fn a_fold_that_stays_bounded_carries_no_reason_and_an_unreasoned_unknown_stays_unreasoned() {
    // The invariant in both directions at the fold boundary.
    let bounded = fold::with_uncertainty(&[est(10.0, 8.0, 12.0), est(20.0, 18.0, 22.0)]).unwrap();
    assert!(!bounded.confidence.interval.is_unknown());
    assert_eq!(bounded.confidence.unknown_reason, None);

    // Pre-Q17 shape: an unknown term with no reason. The fold does not invent one.
    let legacy = Quantity {
        value: 0.0,
        kind: MeasureKind::Level,
        confidence: Confidence::unknown("never measured"),
    };
    let out = fold::with_uncertainty(&[wit(10.0), legacy]).unwrap();
    assert!(out.confidence.interval.is_unknown());
    assert_eq!(out.confidence.unknown_reason, None);
}
