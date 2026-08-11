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
