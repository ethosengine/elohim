//! Contract tests for the probe shape (interfaces spec §2.I).
//!
//! A probe with nothing to read returns `None` (rendered unmeasured), never `Present(healthy)`; an
//! `Unreachable` answer always carries a real transport reason; a holder-only reading reaches only a
//! `HolderSink` and a party-scoped one reaches no public writer, through the crate's
//! `Reading::route` and the sinks' parameter types; a holder's numbers leave only as a
//! `Declassified` built from the holder reading; every label a probe counts is a conformant
//! `ReasonLabel`. A shared reading reaches a sink only through `SharedReading::deliver`, which
//! refuses a sink whose readers are more open than the reading's reach. That an existing reading
//! cannot be relabelled with a wider disclosure, in place, by a struct update or by deserializing,
//! is `compile_fail` doctests on `Reading`; `evidence` and `disclosure` are required arguments of
//! `Reading::new`.

use seam_contracts::{
    check_reason_labels, Admitted, AnswerReason, AnswerState, Disclosure, EvidenceStage,
    HolderReading, HolderSink, PartyRole, Probe, ProbeAnswer, Reading, Routed, ShareRefusal,
    SharedSink, UnreachableReason, OPENNESS_MAX, OPENNESS_MIN, PUBLIC_OPENNESS,
};

const PROMISE: &str = "bytes-layer-reads-only-what-is-read@1";

/// A probe over a walk counter that may not have run yet, or whose tier may not have answered.
struct WalkProbe {
    walked: Option<Result<u32, AnswerReason>>,
}

impl Probe for WalkProbe {
    type Value = u32;
    fn promise(&self) -> &'static str {
        PROMISE
    }
    fn read(&self, observed_at: &str) -> Option<Reading<u32>> {
        let answer = match self.walked? {
            Ok(n) => ProbeAnswer::present(n),
            Err(reason) => ProbeAnswer::unreachable(reason).ok()?,
        };
        Some(Reading::new(
            self.promise().into(),
            "materialize-what-is-read".into(),
            "bafy-root".into(),
            answer,
            EvidenceStage::Fixture,
            Disclosure::Holder,
            observed_at.into(),
        ))
    }
}

#[test]
fn a_probe_with_nothing_to_read_returns_none() {
    let p = WalkProbe { walked: None };
    assert!(
        p.read("2026-10-20T00:00:00Z").is_none(),
        "unmeasured is None, not a reading"
    );
}

#[test]
fn an_unreachable_answer_always_carries_a_transport_reason() {
    for reason in [
        AnswerReason::Timeout,
        AnswerReason::TransportError,
        AnswerReason::Refused,
        AnswerReason::Unverifiable,
        AnswerReason::NotYetDelivered,
    ] {
        let r = WalkProbe {
            walked: Some(Err(reason)),
        }
        .read("t")
        .expect("read");
        assert_eq!(r.answer().state(), AnswerState::Unreachable);
        assert_eq!(r.answer().reason(), Some(reason));
    }
    // ObservedAbsent cannot ride an Unreachable answer: the probe has no way to build one.
    assert_eq!(
        ProbeAnswer::<u32>::unreachable(AnswerReason::ObservedAbsent),
        Err(AnswerReason::ObservedAbsent)
    );
}

#[derive(Default)]
struct StatusFile {
    written: Vec<(String, String)>,
}

impl HolderSink<u32> for StatusFile {
    fn write_holder(&mut self, reading: &HolderReading<u32>) {
        let r = reading.reading();
        self.written
            .push((r.promise().to_string(), format!("{:?}", r.answer())));
    }
}

#[derive(Default)]
struct PublicGauge {
    exported: Vec<(String, u8)>,
}

/// A public gauge: anyone may read it, so it declares public's ordinal and never compares a reach
/// itself.
impl SharedSink<u32> for PublicGauge {
    fn audience_openness(&self) -> u8 {
        PUBLIC_OPENNESS
    }
    fn write_shared(&mut self, reading: &Admitted<'_, u32>) {
        let shared = reading.reading();
        self.exported
            .push((shared.reading().promise().to_string(), shared.openness()));
    }
}

/// What a host does with a routed reading. The routing itself is the crate's `Reading::route`; the
/// sinks' parameter types are what keep a holder or party-scoped reading out of the gauge, so this
/// match has no arm that could send one there.
fn deliver(
    reading: Reading<u32>,
    file: &mut StatusFile,
    gauge: &mut PublicGauge,
    parties: &mut Vec<Vec<PartyRole>>,
) -> Result<(), ShareRefusal> {
    match reading.route() {
        Routed::Holder(h) => file.write_holder(&h),
        Routed::Parties(p) => parties.push(p.roles().to_vec()),
        Routed::Shared(s) => return s.deliver(gauge),
    }
    Ok(())
}

/// A new reading of the same observation, declared with `disclosure` by the producer that builds it.
fn rebuilt(r: &Reading<u32>, disclosure: Disclosure) -> Reading<u32> {
    Reading::new(
        r.promise().into(),
        r.concern().into(),
        r.subject().into(),
        r.answer().clone(),
        r.evidence(),
        disclosure,
        r.observed_at().into(),
    )
}

#[test]
fn holder_readings_reach_only_a_holder_sink() {
    let mut file = StatusFile::default();
    let mut gauge = PublicGauge::default();
    let mut parties = Vec::new();
    let held = WalkProbe {
        walked: Some(Ok(12)),
    }
    .read("t")
    .expect("read");

    deliver(held.clone(), &mut file, &mut gauge, &mut parties).expect("holder");
    assert_eq!(
        file.written,
        vec![(PROMISE.to_string(), "Present(12)".to_string())]
    );
    assert!(
        gauge.exported.is_empty(),
        "no holder reading reached a public writer"
    );
    assert!(parties.is_empty());

    // A party-scoped reading reaches neither the holder's file nor the public gauge. It is a new
    // reading, built with its own disclosure: an existing reading cannot be relabelled (the
    // compile_fail doctests on `Reading`).
    let party_scoped = rebuilt(
        &held,
        Disclosure::Parties {
            roles: vec![PartyRole::Carrier, PartyRole::CarriedFor],
        },
    );
    deliver(party_scoped, &mut file, &mut gauge, &mut parties).expect("parties");
    assert_eq!(file.written.len(), 1);
    assert!(
        gauge.exported.is_empty(),
        "no party-scoped reading reached a public writer"
    );
    assert_eq!(
        parties,
        vec![vec![PartyRole::Carrier, PartyRole::CarriedFor]]
    );

    // Only a reading disclosed at a reach at least as open as the gauge's readers reaches it.
    let at_public = rebuilt(
        &held,
        Disclosure::AtReach {
            openness: PUBLIC_OPENNESS,
        },
    );
    deliver(at_public, &mut file, &mut gauge, &mut parties).expect("public");
    assert_eq!(gauge.exported, vec![(PROMISE.to_string(), PUBLIC_OPENNESS)]);
    assert_eq!(file.written.len(), 1);

    // The holder's numbers leave only through a Declassified built from the holder reading.
    let Routed::Holder(h) = held.route() else {
        panic!("a holder reading routes to Holder");
    };
    let d = h.declassify(vec![("nodesFetched".into(), 12)], "human:matthew".into());
    assert_eq!(d.promise(), PROMISE);
    assert_eq!(d.counts(), &[("nodesFetched".to_string(), 12)]);
}

#[test]
fn a_shared_reading_reaches_only_a_sink_inside_its_reach() {
    let mut file = StatusFile::default();
    let mut gauge = PublicGauge::default();
    let mut parties = Vec::new();
    let held = WalkProbe {
        walked: Some(Ok(12)),
    }
    .read("t")
    .expect("read");

    // A trusted-reach reading (Reach::Trusted.openness() == 4) is refused by the public gauge,
    // which compares nothing itself.
    let trusted = rebuilt(&held, Disclosure::AtReach { openness: 4 });
    assert_eq!(
        deliver(trusted, &mut file, &mut gauge, &mut parties),
        Err(ShareRefusal::AudienceWiderThanReach)
    );
    // An openness that is no reach at all is refused too.
    for openness in [0, OPENNESS_MAX + 1] {
        let bogus = rebuilt(&held, Disclosure::AtReach { openness });
        assert_eq!(
            deliver(bogus, &mut file, &mut gauge, &mut parties),
            Err(ShareRefusal::ReachOutOfRange)
        );
    }
    assert!(
        gauge.exported.is_empty(),
        "nothing below public was exported"
    );

    // A commons-reach reading reaches it.
    let commons = rebuilt(
        &held,
        Disclosure::AtReach {
            openness: OPENNESS_MAX,
        },
    );
    deliver(commons, &mut file, &mut gauge, &mut parties).expect("commons");
    assert_eq!(gauge.exported, vec![(PROMISE.to_string(), OPENNESS_MAX)]);
    assert!(file.written.is_empty() && parties.is_empty());
}

/// The ordinals this crate compares are `Reach::openness()`'s: index + 1 in the reach schema's
/// ordered `enum`. elohim-epr's `openness_matches_generated_ordinal` pins `Reach::openness()` to
/// the same schema, so the two crates agree without this leaf depending on elohim-epr.
#[test]
fn openness_ordinals_match_the_reach_schema() {
    let schema = include_str!("../../../elohim/sdk/schemas/v1/enums/reach.schema.json");
    let line = schema
        .lines()
        .find(|l| l.trim_start().starts_with("\"enum\""))
        .expect("reach.schema.json has an enum line");
    let values: Vec<&str> = line
        .split('"')
        .skip(3)
        .step_by(2)
        .filter(|v| !v.trim().is_empty())
        .collect();
    assert_eq!(
        values,
        [
            "private",
            "self",
            "intimate",
            "trusted",
            "familiar",
            "community",
            "public",
            "commons"
        ]
    );
    assert_eq!(OPENNESS_MIN, 1, "private is first");
    assert_eq!(usize::from(OPENNESS_MAX), values.len(), "commons is last");
    let public = values.iter().position(|v| *v == "public").expect("public");
    assert_eq!(usize::from(PUBLIC_OPENNESS), public + 1);
}

#[test]
fn every_label_a_probe_counts_is_conformant() {
    check_reason_labels::<AnswerReason>().expect("AnswerReason");
    check_reason_labels::<EvidenceStage>().expect("EvidenceStage");
    check_reason_labels::<PartyRole>().expect("PartyRole");
    check_reason_labels::<UnreachableReason>().expect("UnreachableReason");
    check_reason_labels::<ShareRefusal>().expect("ShareRefusal");
}
