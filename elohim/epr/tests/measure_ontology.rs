use elohim_epr::measure::*;

#[test]
fn a_rate_cannot_exist_without_a_period() {
    // MeasureKind::Rate carries its period in the type — there is no way to
    // construct a rate that forgot its denominator. This is the unit error
    // that shipped in spatial_capacity.rs made unrepresentable.
    let r = MeasureKind::Rate { per: Period::Year };
    assert_eq!(r.period(), Some(Period::Year));
    assert_eq!(MeasureKind::Level.period(), None);
}

#[test]
fn interval_unknown_is_the_degenerate_case_of_honest_absence() {
    // C4 honest absence is subsumed: "unmeasured" is the interval that
    // admits everything, NOT a separate nullable field.
    let u = Interval::unknown();
    assert!(u.is_unknown());
    assert!(!Interval::exact(3.0).is_unknown());
    assert!(u.contains(f64::MIN_POSITIVE) && u.contains(1e300));
}

#[test]
fn widening_is_free_and_narrowing_is_refused() {
    // The honesty asymmetry: an agent may always widen its own claim.
    // Narrowing is not a mutation — it requires a new observation.
    let c = Confidence::estimated(Interval::new(10.0, 20.0), "self-report");
    let wider = c
        .widen(Interval::new(5.0, 30.0))
        .expect("widening is always allowed");
    assert_eq!(wider.interval, Interval::new(5.0, 30.0));
    assert!(
        c.widen(Interval::new(12.0, 18.0)).is_err(),
        "narrowing must be refused"
    );
}

#[test]
fn witnessed_and_estimated_are_distinguishable_and_basis_is_required() {
    let w = Confidence::witnessed(Interval::exact(42.0), "wc -c on disk");
    assert_eq!(w.claim, ClaimKind::Witnessed);
    assert!(
        !w.basis.is_empty(),
        "a claim without a basis is uninterpretable"
    );
}

#[test]
fn quantity_serializes_with_confidence_inline_not_detachable() {
    // The interval must be INSIDE the canonical bytes. If Confidence were
    // serialized as a sibling document, it could be swapped post-hoc.
    let q = Quantity {
        value: 12.0,
        kind: MeasureKind::Rate { per: Period::Day },
        confidence: Confidence::estimated(Interval::new(8.0, 16.0), "3-week sample"),
    };
    let json = serde_json::to_string(&q).unwrap();
    assert!(
        json.contains("\"confidence\""),
        "confidence is inline in the quantity"
    );
    assert!(
        json.contains("\"per\":\"day\""),
        "the period survives the wire"
    );
}

#[test]
fn quantity_round_trips_through_json_despite_flatten_plus_internally_tagged_enum() {
    // Quantity flattens an internally-tagged enum (MeasureKind) into itself.
    // Flatten + internal tagging interact through serde's Content-buffering
    // path on the Deserialize side; verify the round trip actually holds
    // rather than assuming it from the serialize-only assertion above.
    let q = Quantity {
        value: 12.0,
        kind: MeasureKind::Rate { per: Period::Day },
        confidence: Confidence::estimated(Interval::new(8.0, 16.0), "3-week sample"),
    };
    let json = serde_json::to_string(&q).unwrap();
    let round_tripped: Quantity = serde_json::from_str(&json).unwrap();
    assert_eq!(round_tripped, q);
}
