//! The first modelled stock — and the shipped defect it makes unrepresentable.
//!
//! Slice 1 gave the missing middle of Meadows' leverage ladder a vocabulary. This is the spine:
//! a level with what fills and drains it, the derived quantities that diagnose it, and the
//! refusals that make the dimensional errors we have actually shipped impossible to write.

use cid::Cid;
use elohim_epr::measure::*;
use elohim_epr::witness::{Magnitude, ReaVerb};
use elohim_epr_rea::model::{AgentRef, FlowEvent};
use elohim_epr_rea::stock::{respite_response, stock_over_window, Stock, StockError, Window};

fn cid_of(seed: &str) -> Cid {
    elohim_epr_rea::atom_cid(&serde_json::json!({ "seed": seed })).unwrap()
}

fn level(v: f64) -> Quantity {
    Quantity {
        value: v,
        kind: MeasureKind::Level,
        confidence: Confidence::witnessed(Interval::exact(v), "fixture level"),
    }
}

fn rate(v: f64, per: Period) -> Quantity {
    Quantity {
        value: v,
        kind: MeasureKind::Rate { per },
        confidence: Confidence::witnessed(Interval::exact(v), "fixture rate"),
    }
}

fn event(action: ReaVerb, resource: &Cid, at: &str, scope: &Cid) -> FlowEvent {
    FlowEvent {
        action,
        provider: AgentRef("repo:test".into()),
        receiver: AgentRef("repo:test".into()),
        resource: *resource,
        quantity: Magnitude::Count {
            value: 1.0,
            unit: "doc".into(),
        },
        process: None,
        in_scope_of: *scope,
        fulfills: vec![],
        satisfies: vec![],
        occurred_at: at.into(),
    }
}

// ------------------------------------------------------- construction refuses incoherence

#[test]
fn a_stock_refuses_a_rate_where_a_level_belongs() {
    let err = Stock::new(
        rate(5.0, Period::Day),
        rate(1.0, Period::Day),
        rate(1.0, Period::Day),
    )
    .unwrap_err();
    assert!(matches!(err, StockError::LevelIsNotALevel(_)));
}

#[test]
fn a_stock_refuses_a_level_where_a_flow_belongs() {
    // A "flow" that is really a cumulative total is precisely the shipped defect's operand:
    // spatial_capacity.rs sums every consume event for all time and treats the total as if it
    // were a yield. Here that operand cannot enter the model at all.
    let err = Stock::new(level(100.0), level(40.0), rate(1.0, Period::Day)).unwrap_err();
    assert!(matches!(
        err,
        StockError::FlowIsNotARate { side: "inflow", .. }
    ));
}

#[test]
fn flows_of_different_tempo_refuse_to_form_a_stock() {
    // 12 per year and 1 per day are not comparable without a conversion, and a conversion
    // chosen inside a constructor is an assumption nobody would ever read.
    let err = Stock::new(
        level(100.0),
        rate(12.0, Period::Year),
        rate(1.0, Period::Day),
    )
    .unwrap_err();
    assert!(matches!(err, StockError::MismatchedPeriods { .. }));
}

// ------------------------------------------------------- the derived quantities

#[test]
fn turnover_time_is_a_time_and_says_how_long_the_corpus_takes_to_replace_itself() {
    let s = Stock::new(
        level(300.0),
        rate(10.0, Period::Week),
        rate(4.0, Period::Week),
    )
    .unwrap();
    let t = s.turnover_time().unwrap();
    assert_eq!(t.value, 75.0, "300 docs draining at 4/week");
    assert_eq!(t.kind, MeasureKind::Level, "a duration, not a ratio");
    // Q11's cure at the binary arity: the derived basis names its inputs' grounding rather than
    // replacing it with a term count.
    assert!(t.confidence.basis.contains("fixture level"));
    assert!(t.confidence.basis.contains("fixture rate"));
}

#[test]
fn a_stock_with_no_outflow_reports_unbounded_turnover_as_absence_not_infinity() {
    // THE DAY-ONE HAZARD. Our doc corpus produces exactly this shape — zero counted absorption
    // — and under the pre-slice-2 arithmetic it yielded [+inf, +inf], which `is_unknown()` then
    // reported as honest absence (Q12) while `value` read `inf` and compared `> 1.0 == true`
    // (Q13). Both legs are closed here, and both are asserted, because either one alone would
    // still let a consumer conclude "confirmed overshoot" from an absence.
    let s = Stock::new(
        level(300.0),
        rate(10.0, Period::Week),
        rate(0.0, Period::Week),
    )
    .unwrap();
    let t = s.turnover_time().unwrap();
    assert!(
        t.confidence.interval.is_unknown(),
        "unbounded, not infinite"
    );
    assert!(
        t.value.is_nan(),
        "a value the interval calls unknowable must not compare"
    );
    // Stated as incomparability rather than as a negated `>`, which is the sharper claim: an
    // unknowable value is not merely "not greater than 1.0", it has no ordering against 1.0 at
    // all. Every naive overshoot test (`value > 1.0`) therefore reads false instead of true.
    assert!(
        t.value.partial_cmp(&1.0).is_none(),
        "and must not silently satisfy an overshoot test"
    );

    let idx = s.emission_absorption().unwrap();
    assert!(idx.confidence.interval.is_unknown());
    assert!(idx.value.is_nan());
}

#[test]
fn the_sink_index_and_the_source_index_are_reciprocals_and_naming_them_apart_is_the_point() {
    // Above 1.0 is unsustainable in BOTH readings — which is exactly why the direction has to
    // be chosen deliberately. The same two rates say "this sink is filling" or "this source is
    // regrowing fine" depending only on which way up the caller's resource is.
    let s = Stock::new(
        level(300.0),
        rate(10.0, Period::Week),
        rate(4.0, Period::Week),
    )
    .unwrap();
    let sink = s.emission_absorption().unwrap();
    let source = s.harvest_regeneration().unwrap();
    assert_eq!(sink.value, 2.5, "emitting 2.5x faster than absorbing");
    assert_eq!(source.value, 0.4);
    assert_eq!(sink.kind, MeasureKind::Ratio);
    assert!((sink.value * source.value - 1.0).abs() < 1e-12);
}

#[test]
fn net_change_of_zero_is_dynamic_equilibrium_not_stasis() {
    // Meadows is emphatic that the sustainable state is not stillness. A corpus authoring 10
    // and retiring 10 a week is in equilibrium; one doing zero of both has a net change of zero
    // too, and is dead rather than healthy. The measure cannot tell them apart — the FLOWS can,
    // which is the argument for modelling flows at all instead of watching the level.
    let alive = Stock::new(
        level(300.0),
        rate(10.0, Period::Week),
        rate(10.0, Period::Week),
    )
    .unwrap();
    let dead = Stock::new(
        level(300.0),
        rate(0.0, Period::Week),
        rate(0.0, Period::Week),
    )
    .unwrap();
    assert_eq!(alive.net_change().unwrap().value, 0.0);
    assert_eq!(dead.net_change().unwrap().value, 0.0);
    assert_eq!(alive.turnover_time().unwrap().value, 30.0);
    assert!(
        dead.turnover_time().unwrap().value.is_nan(),
        "turnover is what distinguishes them, and for the dead corpus it is unknowable"
    );
}

#[test]
fn net_change_keeps_the_period_and_the_weakest_claim() {
    let s = Stock::new(
        level(300.0),
        Quantity {
            value: 10.0,
            kind: MeasureKind::Rate { per: Period::Week },
            confidence: Confidence::estimated(Interval::new(8.0, 12.0), "sampled"),
        },
        rate(4.0, Period::Week),
    )
    .unwrap();
    let net = s.net_change().unwrap();
    assert_eq!(net.kind, MeasureKind::Rate { per: Period::Week });
    assert_eq!(net.value, 6.0);
    assert_eq!(net.confidence.claim, ClaimKind::Estimated);
    // Correlation-naive subtraction over-widens; over-widening never manufactures precision.
    assert_eq!(net.confidence.interval, Interval::new(4.0, 8.0));
}

#[test]
fn respite_response_is_the_same_arithmetic_and_a_different_question() {
    // One shape, deliberately not duplicated: emission/absorption asks whether a stock is
    // bounded; respite/response asks whether "try harder" is on the menu. Above 1.0 it is not —
    // the only cures are raising the response rate structurally or lowering the growth rate.
    let growth = rate(20.0, Period::Week);
    let response = rate(5.0, Period::Week);
    let idx = respite_response(&growth, &response).unwrap();
    assert_eq!(idx.value, 4.0);
    assert_eq!(idx.kind, MeasureKind::Ratio);
    assert!(idx.confidence.basis.contains("respite/response"));
}

#[test]
fn a_ratio_cannot_be_divided_again_without_a_decision_nobody_has_made() {
    let a = respite_response(&rate(20.0, Period::Week), &rate(5.0, Period::Week)).unwrap();
    let b = respite_response(&rate(10.0, Period::Week), &rate(5.0, Period::Week)).unwrap();
    assert!(matches!(
        respite_response(&a, &b),
        Err(StockError::UndefinedDivision(..))
    ));
}

// ------------------------------------------------------- the fold over real events

#[test]
fn a_stock_is_folded_from_flow_events_and_the_level_is_not_windowed() {
    // The level is an accumulation of the WHOLE past; the flows are windowed. Windowing the
    // level too would report a recent delta as though it were the stock — the same level/rate
    // confusion, one layer up.
    let scope = cid_of("scope");
    let corpus = cid_of("doc-corpus");
    let events = vec![
        event(ReaVerb::Produce, &corpus, "2026-01-01T00:00:00Z", &scope),
        event(ReaVerb::Produce, &corpus, "2026-01-02T00:00:00Z", &scope),
        event(ReaVerb::Produce, &corpus, "2026-08-02T00:00:00Z", &scope),
        event(ReaVerb::Produce, &corpus, "2026-08-03T00:00:00Z", &scope),
        event(ReaVerb::Consume, &corpus, "2026-08-04T00:00:00Z", &scope),
        // Beyond the window's end: excluded from both the level and the flows.
        event(ReaVerb::Produce, &corpus, "2026-09-01T00:00:00Z", &scope),
        // A different resource entirely.
        event(
            ReaVerb::Produce,
            &cid_of("other"),
            "2026-08-02T00:00:00Z",
            &scope,
        ),
    ];
    let window = Window {
        start: "2026-08-01T00:00:00Z".into(),
        end: "2026-08-08T00:00:00Z".into(),
        per: Period::Week,
        periods: 1.0,
    };
    let s = stock_over_window(&corpus, &events, &window, "doc").unwrap();
    assert_eq!(
        s.level.value, 3.0,
        "4 produced - 1 consumed, through window end"
    );
    assert_eq!(
        s.inflow.value, 2.0,
        "2 produced inside the window, per week"
    );
    assert_eq!(s.outflow.value, 1.0);
    assert_eq!(s.emission_absorption().unwrap().value, 2.0);
    assert!(
        s.inflow.confidence.basis.contains("2026-08-01"),
        "the window is in the basis"
    );
}

#[test]
fn a_source_only_projection_folds_to_an_unknown_index_rather_than_a_confident_one() {
    // OUR ACTUAL SHAPE, and the finding that motivated slice 2's projection work: `epr flow
    // project` mints only ReaVerb::Produce, so the doc corpus has an inflow and no modelled
    // outflow at all. The instrument is not broken — the MODEL has no sink, and the honest
    // output of a model with no sink is "unknown", never "infinitely overshot".
    let scope = cid_of("scope");
    let corpus = cid_of("doc-corpus");
    let events: Vec<_> = ["2026-08-02T00:00:00Z", "2026-08-03T00:00:00Z"]
        .iter()
        .map(|at| event(ReaVerb::Produce, &corpus, at, &scope))
        .collect();
    let window = Window {
        start: "2026-08-01T00:00:00Z".into(),
        end: "2026-08-08T00:00:00Z".into(),
        per: Period::Week,
        periods: 1.0,
    };
    let s = stock_over_window(&corpus, &events, &window, "doc").unwrap();
    assert_eq!(s.outflow.value, 0.0);
    let idx = s.emission_absorption().unwrap();
    assert!(idx.confidence.interval.is_unknown());
    assert!(idx.value.is_nan());
}

#[test]
fn an_empty_window_is_refused_rather_than_dividing_by_zero() {
    let window = Window {
        start: "2026-08-01T00:00:00Z".into(),
        end: "2026-08-01T00:00:00Z".into(),
        per: Period::Week,
        periods: 0.0,
    };
    assert!(matches!(
        stock_over_window(&cid_of("x"), &[], &window, "doc"),
        Err(StockError::EmptyWindow(_))
    ));
}

#[test]
fn a_caller_that_knows_its_projection_is_incomplete_widens_on_the_way_out() {
    // The separation of concerns the fold depends on. `stock_over_window` claims Witnessed
    // about its ARITHMETIC — those events happened. Whether the projection SEES every real
    // absorption is knowledge the folding layer does not have and the calling instrument does.
    // Widening is free by law (L4); narrowing needs a new observation.
    let scope = cid_of("scope");
    let corpus = cid_of("doc-corpus");
    let events = vec![
        event(ReaVerb::Produce, &corpus, "2026-08-02T00:00:00Z", &scope),
        event(ReaVerb::Consume, &corpus, "2026-08-03T00:00:00Z", &scope),
    ];
    let window = Window {
        start: "2026-08-01T00:00:00Z".into(),
        end: "2026-08-08T00:00:00Z".into(),
        per: Period::Week,
        periods: 1.0,
    };
    let s = stock_over_window(&corpus, &events, &window, "doc").unwrap();
    assert_eq!(s.outflow.confidence.claim, ClaimKind::Witnessed);

    let widened = s
        .outflow
        .confidence
        .widen(Interval::multiplier_widen(s.outflow.value, 1.0, 3.0))
        .expect("widening is always free");
    assert_eq!(widened.interval, Interval::new(1.0, 3.0));
    // ...and the asymmetry holds on the way back. Once the instrument has admitted it cannot
    // see every absorption path, it cannot un-admit that without a new observation.
    assert!(widened
        .widen(Interval::new(1.5, 2.0))
        .is_err_and(|e| e == ConfidenceError::NarrowingRefused));
}

#[test]
fn an_undrained_stock_reports_a_structural_reason_not_a_bare_unknown() {
    // Q17 at the one call site that most needs it. A stock whose outflow band admits zero has
    // genuinely unbounded turnover — slice 2 already returned honest absence there rather than
    // `+inf`. What it could not say is WHY, and the why is the actionable half: this is a
    // property of the DENOMINATOR's structure (nothing drains), so measuring the level more
    // carefully will never narrow it. An uncertainty work-queue that saw only `(-inf, +inf)`
    // would rank it beside "nobody has instrumented this yet," which is the one class more
    // measurement genuinely fixes.
    let stock = Stock::new(
        Quantity {
            value: 300.0,
            kind: MeasureKind::Level,
            confidence: Confidence::witnessed(Interval::exact(300.0), "counted"),
        },
        Quantity {
            value: 10.0,
            kind: MeasureKind::Rate { per: Period::Day },
            confidence: Confidence::witnessed(Interval::exact(10.0), "counted"),
        },
        Quantity {
            value: 0.0,
            kind: MeasureKind::Rate { per: Period::Day },
            confidence: Confidence::estimated(Interval::new(0.0, 2.0), "outflow band spans zero"),
        },
    );

    let turnover = stock
        .expect("stock is well-formed")
        .turnover_time()
        .expect("level / rate is well-formed");
    assert!(turnover.confidence.interval.is_unknown());
    assert_eq!(
        turnover.confidence.unknown_reason,
        Some(UnknownReason::UndefinedDivision)
    );
    assert_eq!(
        turnover.confidence.unknown_reason.unwrap().tightenable(),
        Tightenable::ByStructure,
        "queue it as structural work, never as 'sample the level harder'"
    );

    // A drained stock divides normally and carries no reason at all.
    let drained = Stock::new(
        Quantity {
            value: 300.0,
            kind: MeasureKind::Level,
            confidence: Confidence::witnessed(Interval::exact(300.0), "counted"),
        },
        Quantity {
            value: 10.0,
            kind: MeasureKind::Rate { per: Period::Day },
            confidence: Confidence::witnessed(Interval::exact(10.0), "counted"),
        },
        Quantity {
            value: 3.0,
            kind: MeasureKind::Rate { per: Period::Day },
            confidence: Confidence::witnessed(Interval::exact(3.0), "counted"),
        },
    );
    let ok = drained
        .expect("stock is well-formed")
        .turnover_time()
        .unwrap();
    assert!(!ok.confidence.interval.is_unknown());
    assert_eq!(ok.confidence.unknown_reason, None);
}

// ------------------------------------ Q17: an OPERAND's reason survives the arithmetic

/// A rate whose confidence declares an unknown interval AND why.
fn unknown_rate(reason: UnknownReason, per: Period) -> Quantity {
    Quantity {
        value: f64::NAN,
        kind: MeasureKind::Rate { per },
        confidence: Confidence::unknown_because(reason, "fixture"),
    }
}

#[test]
fn a_turnover_time_over_an_incommensurable_level_reports_incommensurable_not_a_bare_unknown() {
    // The half the guards cannot see. `div_because` receives two bare INTERVALS, so its only
    // vocabulary is what its own guards discover — and here no guard fires: the denominator is a
    // clean 3/day, and the quotient loses its bounds purely because the NUMERATOR arrived
    // unbounded. Before the operands were threaded through, this returned `unknown` with
    // `reason: None` — an unclassified absence, free to be ranked by row 18's queue as "go
    // measure the level harder" against a level that is a preference ordering wearing a
    // quantity's clothes. That is the exact failure the enum was minted to prevent, reproduced
    // one layer above the enum.
    let stock = Stock::new(
        Quantity {
            value: f64::NAN,
            kind: MeasureKind::Level,
            confidence: Confidence::unknown_because(
                UnknownReason::Incommensurable,
                "weighted composite of re-tunable terms",
            ),
        },
        rate(10.0, Period::Day),
        rate(3.0, Period::Day),
    )
    .expect("stock is well-formed");

    let turnover = stock.turnover_time().expect("level / rate is well-formed");
    assert!(turnover.confidence.interval.is_unknown());
    assert_eq!(
        turnover.confidence.unknown_reason,
        Some(UnknownReason::Incommensurable),
        "the level's reason is the derived quantity's reason; no guard fired to supply one"
    );
    assert_eq!(
        turnover.confidence.unknown_reason.unwrap().tightenable(),
        Tightenable::Never,
        "and so this must never enter a measurement queue at all"
    );
}

#[test]
fn a_net_change_over_an_uninstrumented_inflow_says_so() {
    // Same defect on the subtraction path. `unknown() - exact()` produces no NaN, so
    // `sub_because`'s indeterminate-form guard never fires and the result was a reasonless
    // unknown — indistinguishable from a net change whose window happened to be empty, when in
    // fact nobody has built the instrument yet. `NotYetInstrumented` is the one class that says
    // "add a sense", which is precisely what row 18 says the system's own ignorance is for.
    let stock = Stock::new(
        level(300.0),
        unknown_rate(UnknownReason::NotYetInstrumented, Period::Day),
        rate(3.0, Period::Day),
    )
    .expect("stock is well-formed");

    let net = stock.net_change().expect("rate - rate is well-formed");
    assert!(net.confidence.interval.is_unknown());
    assert_eq!(
        net.confidence.unknown_reason,
        Some(UnknownReason::NotYetInstrumented)
    );
    assert_eq!(
        net.confidence.unknown_reason.unwrap().tightenable(),
        Tightenable::ByMeasurement,
        "this one genuinely belongs in the queue — build the sense"
    );

    // Outflow side, symmetrically: the operand that is unknown is not privileged by position.
    let other = Stock::new(
        level(300.0),
        rate(3.0, Period::Day),
        unknown_rate(UnknownReason::NoObservations, Period::Day),
    )
    .expect("stock is well-formed")
    .net_change()
    .unwrap();
    assert_eq!(
        other.confidence.unknown_reason,
        Some(UnknownReason::NoObservations)
    );
}

#[test]
fn when_a_guard_and_an_operand_both_speak_the_least_tightenable_one_wins() {
    // The documented precedence rule, exercised in both directions so it cannot be mistaken for
    // "the guard always wins" or "the operand always wins". `UnknownReason::reduce` ranks the
    // guard's finding as just another contributor: a derived quantity is no more improvable than
    // the worst thing that fed it.

    // (a) Zero-crossing denominator (UndefinedDivision, ByStructure) + incommensurable numerator
    //     (Never). Fixing the model would not help — the numerator is not an estimate of
    //     anything — so the LEVEL's reason must survive the division's own finding.
    let both = Stock::new(
        Quantity {
            value: f64::NAN,
            kind: MeasureKind::Level,
            confidence: Confidence::unknown_because(UnknownReason::Incommensurable, "composite"),
        },
        rate(10.0, Period::Day),
        Quantity {
            value: 0.0,
            kind: MeasureKind::Rate { per: Period::Day },
            confidence: Confidence::estimated(Interval::new(0.0, 2.0), "band spans zero"),
        },
    )
    .expect("stock is well-formed")
    .turnover_time()
    .unwrap();
    assert_eq!(
        both.confidence.unknown_reason,
        Some(UnknownReason::Incommensurable)
    );

    // (b) Same denominator, but a numerator that merely lacks observations (ByMeasurement). Now
    //     the guard's structural finding is the actionable one — sampling the level is exactly
    //     the useless work — so it wins.
    let structural = Stock::new(
        Quantity {
            value: f64::NAN,
            kind: MeasureKind::Level,
            confidence: Confidence::unknown_because(UnknownReason::NoObservations, "empty window"),
        },
        rate(10.0, Period::Day),
        Quantity {
            value: 0.0,
            kind: MeasureKind::Rate { per: Period::Day },
            confidence: Confidence::estimated(Interval::new(0.0, 2.0), "band spans zero"),
        },
    )
    .expect("stock is well-formed")
    .turnover_time()
    .unwrap();
    assert_eq!(
        structural.confidence.unknown_reason,
        Some(UnknownReason::UndefinedDivision)
    );
}

#[test]
fn a_bounded_result_never_inherits_an_operand_reason() {
    // The invariant's guard at the arithmetic boundary. An operand can carry a reason only while
    // its own interval is unknown, and an unknown operand cannot produce a bounded quotient — but
    // the threading must not be able to attach one anyway. Bounded in, bounded out, no reason.
    let ok = Stock::new(
        level(300.0),
        rate(10.0, Period::Day),
        rate(3.0, Period::Day),
    )
    .expect("stock is well-formed");
    let turnover = ok.turnover_time().unwrap();
    assert!(!turnover.confidence.interval.is_unknown());
    assert_eq!(turnover.confidence.unknown_reason, None);
    let net = ok.net_change().unwrap();
    assert!(!net.confidence.interval.is_unknown());
    assert_eq!(net.confidence.unknown_reason, None);
}

#[test]
fn respite_responses_own_guard_outranks_a_merely_unmeasured_operand() {
    // NAMED FOR WHAT IT PINS. An earlier revision called this "threads its operands' reasons
    // too" and asserted `UndefinedDivision` — which `div_because`'s own guard mints unaided, so
    // deleting `attributed_to` from `stock::divide` left it green. It pins the LOSING half of
    // the precedence rule, which is worth having; the winning half is the test below.
    //
    // The Meadows controllability index is `divide` under another name, and it is the function
    // `_lib/signal_measure.py::ratio_of_rates` mirrors — so this is the Rust side of the
    // mint-site parity the Python test pins. An unmeasured response rate is a `ByMeasurement`
    // finding ("quicken the response rate" is only on the menu once you can see it), not the
    // reasonless absence it used to be.
    let out = respite_response(
        &rate(4.0, Period::Week),
        &unknown_rate(UnknownReason::NoObservations, Period::Week),
    )
    .expect("rate / rate is well-formed");
    assert!(out.confidence.interval.is_unknown());
    assert!(out.value.is_nan(), "Q13: an unknowable value is NaN");
    assert_eq!(
        out.confidence.unknown_reason,
        // The denominator's unknown band admits zero, so `div_because` mints UndefinedDivision;
        // reduced against the operand's ByMeasurement reason, the structural finding wins.
        Some(UnknownReason::UndefinedDivision)
    );
}

#[test]
fn an_incommensurable_operand_outranks_the_divisions_own_guard() {
    // The half that actually pins `attributed_to`. Delete the `.attributed_to(...)` call from
    // `stock::divide` and this test goes red, because `Incommensurable` can ONLY have arrived
    // from the operand — no guard in `div_because` mints it.
    //
    // Why this pair and not another: `Incommensurable` is `Tightenable::Never`, so
    // `UnknownReason::reduce` returns it against ANY reason the guard might mint. That makes
    // the assertion independent of which branch of `div_because` fires, so it pins the
    // precedence rule rather than one arithmetic path.
    //
    // This is the case the whole enum exists for. A controllability index whose numerator is a
    // `drift_score`-style incommensurable composite must not surface as a plain unknown: row 18's
    // work-queue would rank "measure this harder" against a number no instrument can ever tighten.
    let out = respite_response(
        &Quantity {
            value: f64::NAN,
            kind: MeasureKind::Rate { per: Period::Week },
            confidence: Confidence::unknown_because(
                UnknownReason::Incommensurable,
                "weighted composite of re-tunable terms — no observation it is an estimate OF",
            ),
        },
        &rate(2.0, Period::Week),
    )
    .expect("rate / rate is well-formed");
    assert!(out.confidence.interval.is_unknown());
    assert_eq!(
        out.confidence.unknown_reason,
        Some(UnknownReason::Incommensurable),
        "an operand no measurement can tighten must not be laundered into a tightenable finding"
    );
}
