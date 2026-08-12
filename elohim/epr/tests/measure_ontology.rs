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
fn an_unknown_reason_survives_the_wire_and_an_absent_one_leaves_no_trace() {
    // Q17's wire contract. The reason has to reach whoever reads the aggregate — a work-queue
    // that classifies uncertainty cannot classify what the serializer dropped — but it must be
    // INVISIBLE when absent, or every quantity ever encoded gains a field and moves its address.
    let plain = Confidence::estimated(Interval::new(8.0, 16.0), "3-week sample");
    let json = serde_json::to_string(&plain).unwrap();
    assert!(
        !json.contains("unknownReason"),
        "a confidence with no reason must serialize exactly as it always did: {json}"
    );

    let reasoned =
        Confidence::unknown_because(UnknownReason::Incommensurable, "weighted composite");
    let json = serde_json::to_string(&reasoned).unwrap();
    assert!(
        json.contains("\"unknownReason\":\"incommensurable\""),
        "{json}"
    );
    // The reason round-trips; the INTERVAL beside it does not, and that hole predates this
    // change — see `an_unknown_interval_is_not_encodable_on_either_wire_and_that_predates_q17`
    // in tests/canonical_bytes.rs. Asserted on the reason alone rather than on the whole
    // `Confidence`, so this test measures Q17's contract and not the codec's older gap.
    assert_eq!(
        serde_json::from_str::<UnknownReason>("\"incommensurable\"").unwrap(),
        UnknownReason::Incommensurable
    );

    // Old bytes still deserialize — the field defaults rather than erroring, so a stored
    // quantity written before Q17 is still readable.
    let legacy = r#"{"claim":"estimated","interval":{"lo":8.0,"hi":16.0},"basis":"3-week sample"}"#;
    let parsed: Confidence = serde_json::from_str(legacy).unwrap();
    assert_eq!(parsed, plain);
    assert_eq!(parsed.unknown_reason, None);
}

#[test]
fn unknown_reason_never_accompanies_a_bounded_interval() {
    // The invariant every constructor upholds: a reason explains an ABSENCE, so attaching one to
    // a bounded claim would be a category error — "here are the bounds, and here is why there
    // are none." Pinned on the constructors, which is where it can be enforced (the fields stay
    // public per Q8, so a struct literal can still write nonsense).
    assert_eq!(
        Confidence::witnessed(Interval::exact(42.0), "wc -c").unknown_reason,
        None
    );
    assert_eq!(
        Confidence::estimated(Interval::new(1.0, 2.0), "sample").unknown_reason,
        None
    );
    assert_eq!(Confidence::unknown("no idea").unknown_reason, None);

    let c = Confidence::unknown_because(UnknownReason::NoObservations, "empty window");
    assert!(c.interval.is_unknown());
    assert_eq!(c.unknown_reason, Some(UnknownReason::NoObservations));

    // from_reasoned is the constructor the arithmetic feeds, and it cannot break the invariant
    // either: the reason it copies came from a Reasoned, which only carries one when unknown.
    let known = Confidence::from_reasoned(
        ClaimKind::Estimated,
        Interval::exact(10.0).div_because(&Interval::exact(2.0)),
        "10/2",
    );
    assert_eq!(known.interval, Interval::exact(5.0));
    assert_eq!(known.unknown_reason, None);
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
