//! The carry claim's contract, proven over hand-built fold records so no daemon or clock runs.
//!
//! Devices are named `did:key:a` (this device in most cases) and `did:key:b`/`did:key:c`. A record
//! a device wrote itself carries no `via:` slot; one folded from a sibling carries `via:<did>`.

use cid::Cid;
use elohim_epr_rea::{
    atom_cid, AgentRef, FlowEvent, FlowRecord, Intent, Magnitude, ReaVerb, ResourceSpec,
};

use super::{decide, live_carries, CarryDecision, CarryQuery, CarryRole, LanderBounds};
use crate::flow::intend::INTENT_TAG;

const A: &str = "did:key:a";
const B: &str = "did:key:b";
const C: &str = "did:key:c";
const PROJECT: &str = "eprfs";
const CLOSURE: &str = "abcdef01";
const CHANGE: &str = "I0123456789abcdef0123456789abcdef01234567";

fn cid(label: &str) -> Cid {
    atom_cid(&label).expect("a string always encodes")
}

fn intent() -> Cid {
    cid("intent-1")
}

/// Admits A and B; C is a device of no roster here.
fn admits(device: &str) -> bool {
    matches!(device, A | B)
}

fn event(resource: Cid, slots: Vec<String>) -> (Cid, FlowRecord) {
    let record = FlowRecord::Event(FlowEvent {
        action: ReaVerb::Cite,
        provider: AgentRef("human:test".to_string()),
        receiver: AgentRef("repo:test/landing".to_string()),
        resource,
        quantity: Magnitude::Count {
            value: 1.0,
            unit: "dev-landing".to_string(),
        },
        process: None,
        in_scope_of: cid("scope"),
        fulfills: Vec::new(),
        satisfies: Vec::new(),
        occurred_at: "2026-11-23T00:00:00Z".to_string(),
        classified_as: slots,
    });
    (record.cid().expect("an event always encodes"), record)
}

fn landing_slots(kind: &str, env: &[(&str, String)], via: Option<&str>) -> Vec<String> {
    let mut slots = vec![
        format!("dev-landing:{kind}"),
        CHANGE.to_string(),
        format!("reason:{kind}"),
        "measure:dev-landing@1".to_string(),
        "value:1".to_string(),
    ];
    for (key, value) in env {
        slots.push(format!("env:{key}={value}"));
    }
    if let Some(device) = via {
        slots.push(format!("via:{device}"));
    }
    slots
}

struct Carry<'a> {
    via: Option<&'a str>,
    intent: Cid,
    closure: &'a str,
    commitment: &'a str,
    at_s: u64,
    valid_until: u64,
    dseq: u64,
}

impl Carry<'_> {
    fn record(&self) -> (Cid, FlowRecord) {
        event(
            self.intent,
            landing_slots(
                "carry",
                &[
                    ("at", format!("{}.000", self.at_s)),
                    ("change", CHANGE.to_string()),
                    ("closure", self.closure.to_string()),
                    ("commitment", cid(self.commitment).to_string()),
                    ("dseq", self.dseq.to_string()),
                    ("miss", cid("misses-event").to_string()),
                    ("project", PROJECT.to_string()),
                    ("valid_until", self.valid_until.to_string()),
                ],
                self.via,
            ),
        )
    }
}

fn carry(via: Option<&str>, commitment: &str, at_s: u64, valid_until: u64) -> (Cid, FlowRecord) {
    Carry {
        via,
        intent: intent(),
        closure: CLOSURE,
        commitment,
        at_s,
        valid_until,
        dseq: 1,
    }
    .record()
}

fn verdict(via: Option<&str>, closure: &str, success: bool) -> (Cid, FlowRecord) {
    let mut slots = vec![
        "observation".to_string(),
        ".".to_string(),
        "reason:verdict".to_string(),
        "measure:gate-verdict@1".to_string(),
        format!("env:project={PROJECT}"),
        format!("env:closure={closure}"),
        format!("env:success={success}"),
    ];
    if let Some(device) = via {
        slots.push(format!("via:{device}"));
    }
    event(cid("subject"), slots)
}

fn query<'a>(
    me: Option<&'a str>,
    now: u64,
    role: CarryRole<'a>,
    intent: &'a Cid,
) -> CarryQuery<'a> {
    CarryQuery {
        intent,
        project: PROJECT,
        closure: CLOSURE,
        admits: &admits,
        me,
        now_unix_s: now,
        role,
    }
}

fn carrier(records: &[(Cid, FlowRecord)], me: &str, now: u64) -> CarryDecision {
    let intent = intent();
    decide(records, &query(Some(me), now, CarryRole::Carrier, &intent))
}

/// A lander on device A that folded every sibling's head at `seen_s` and may wait `max_wait_s`.
fn lander(
    records: &[(Cid, FlowRecord)],
    now: u64,
    seen_s: Option<u64>,
    wait_until: Option<u64>,
    max_wait_s: u64,
) -> CarryDecision {
    let intent = intent();
    let last_seen = move |_: &str| seen_s.map(|s| s * 1000);
    decide(
        records,
        &query(
            Some(A),
            now,
            CarryRole::Lander(LanderBounds {
                freshness_s: Some(90),
                max_skew_s: Some(30),
                last_seen_ms: &last_seen,
                wait_until,
                max_wait_s,
            }),
            &intent,
        ),
    )
}

fn is_run_here(decision: &CarryDecision) -> bool {
    matches!(decision, CarryDecision::RunHere { .. })
}

#[test]
fn a_replay_written_while_the_claim_holds_never_extends_it() {
    let first = Carry {
        via: Some(B),
        intent: intent(),
        closure: CLOSURE,
        commitment: "c-first",
        at_s: 1_000,
        valid_until: 1_100,
        dseq: 1,
    };
    let replay = Carry {
        commitment: "c-replay",
        at_s: 1_050,
        valid_until: 5_000,
        dseq: 2,
        ..first
    };
    let records = vec![first.record(), replay.record()];
    let live = live_carries(&records, &admits, Some(A), 1_099);
    assert_eq!(live.len(), 1);
    assert_eq!(live[0].valid_until, 1_100);
    assert_eq!(live[0].commitment, cid("c-first").to_string());
    assert!(
        live_carries(&records, &admits, Some(A), 1_100).is_empty(),
        "the replay must not hold the miss past the first claim's own valid_until"
    );

    // Folding the same event twice is the same claim.
    let doubled = vec![first.record(), first.record()];
    assert_eq!(live_carries(&doubled, &admits, Some(A), 1_050).len(), 1);

    // A claim written after the first lapsed is a renewal: a new claim with its own bound.
    let renewal = Carry {
        commitment: "c-renewal",
        at_s: 1_100,
        valid_until: 1_300,
        dseq: 3,
        ..first
    };
    let records = vec![first.record(), replay.record(), renewal.record()];
    let live = live_carries(&records, &admits, Some(A), 1_200);
    assert_eq!(live.len(), 1);
    assert_eq!(live[0].valid_until, 1_300);
}

#[test]
fn a_carry_lapses_exactly_at_its_own_valid_until() {
    let records = vec![carry(Some(B), "c-b", 1_000, 1_100)];
    assert_eq!(live_carries(&records, &admits, Some(A), 1_099).len(), 1);
    assert!(live_carries(&records, &admits, Some(A), 1_100).is_empty());
    assert!(matches!(
        carrier(&records, A, 1_099),
        CarryDecision::AlreadyCarried { .. }
    ));
    assert_eq!(carrier(&records, A, 1_100), CarryDecision::Run);
    assert!(matches!(
        lander(&records, 1_099, Some(1_090), None, 600),
        CarryDecision::Wait {
            until: 1_100,
            wait_s: 1,
            ..
        }
    ));
    assert!(is_run_here(&lander(
        &records,
        1_100,
        Some(1_090),
        None,
        600
    )));
}

#[test]
fn a_second_idle_device_observes_the_incumbent_and_declines() {
    let records = vec![carry(Some(B), "c-b", 1_000, 1_600)];
    assert_eq!(
        carrier(&records, A, 1_010),
        CarryDecision::AlreadyCarried {
            by: B.to_string(),
            until: 1_600,
            commitment: cid("c-b").to_string(),
        }
    );
}

#[test]
fn simultaneous_claims_may_both_run_then_converge_on_the_lowest_commitment() {
    // Each device claims before the exchange carries the other's claim.
    let own_a = carry(None, "c-a", 1_000, 1_600);
    let own_b = carry(None, "c-b", 1_000, 1_600);
    let store_a = vec![own_a.clone()];
    let store_b = vec![own_b.clone()];
    // Before convergence both run: a duplicate execution is possible, and harmless below.
    assert_eq!(carrier(&store_a, A, 1_001), CarryDecision::Run);
    assert_eq!(carrier(&store_b, B, 1_001), CarryDecision::Run);

    // After the exchange each store holds its own claim and the sibling's.
    let store_a = vec![own_a, carry(Some(B), "c-b", 1_000, 1_600)];
    let store_b = vec![own_b, carry(Some(A), "c-a", 1_000, 1_600)];
    let a = carrier(&store_a, A, 1_002);
    let b = carrier(&store_b, B, 1_002);
    let a_keeps = cid("c-a").to_bytes() < cid("c-b").to_bytes();
    let (keeper, yielder) = if a_keeps { (&a, &b) } else { (&b, &a) };
    assert_eq!(*keeper, CarryDecision::Run);
    let lower = if a_keeps { cid("c-a") } else { cid("c-b") };
    assert!(
        matches!(yielder, CarryDecision::AlreadyCarried { commitment, .. } if *commitment == lower.to_string()),
        "the other carrier yields to the lowest commitment CID: {yielder:?}"
    );

    // Harmless: whichever run finishes first settles the miss on both devices.
    let mut settled_a = store_a.clone();
    settled_a.push(verdict(Some(B), CLOSURE, true));
    let mut settled_b = store_b.clone();
    settled_b.push(verdict(None, CLOSURE, true));
    assert!(live_carries(&settled_a, &admits, Some(A), 1_003).is_empty());
    assert!(live_carries(&settled_b, &admits, Some(B), 1_003).is_empty());
}

#[test]
fn a_silent_or_unreachable_carrier_means_run_here_and_a_fresh_one_a_wait_that_ends() {
    let records = vec![carry(Some(B), "c-b", 1_000, 1_600)];
    // No head ever folded from the carrier: it is unreachable from here.
    assert!(is_run_here(&lander(&records, 1_010, None, None, 600)));
    // Its last head is older than freshness_s: silent.
    assert!(is_run_here(&lander(
        &records,
        1_200,
        Some(1_100),
        None,
        600
    )));
    // Fresh: a bounded wait, the earliest of the carry's bound, the lander's own and the deadline.
    assert_eq!(
        lander(&records, 1_010, Some(1_000), None, 60),
        CarryDecision::Wait {
            by: B.to_string(),
            until: 1_070,
            wait_s: 60,
            commitment: cid("c-b").to_string(),
        }
    );
    assert!(matches!(
        lander(&records, 1_010, Some(1_000), Some(1_040), 600),
        CarryDecision::Wait {
            until: 1_040,
            wait_s: 30,
            ..
        }
    ));
    assert!(matches!(
        lander(&records, 1_010, Some(1_000), None, 6_000),
        CarryDecision::Wait {
            until: 1_600,
            wait_s: 590,
            ..
        }
    ));
    // The wait ends: past the lander's own time, or with no deadline left, it runs here.
    assert!(is_run_here(&lander(
        &records,
        1_040,
        Some(1_000),
        Some(1_040),
        600
    )));
    assert!(is_run_here(&lander(&records, 1_010, Some(1_000), None, 0)));
    // A carrier that never writes a verdict cannot hold the lander past its bound.
    assert!(is_run_here(&lander(
        &records,
        1_600,
        Some(1_590),
        None,
        600
    )));
    // Unknown budgets are doubt.
    let intent = intent();
    let seen = |_: &str| Some(1_000_000);
    let unknown = decide(
        &records,
        &query(
            Some(A),
            1_010,
            CarryRole::Lander(LanderBounds {
                freshness_s: None,
                max_skew_s: Some(30),
                last_seen_ms: &seen,
                wait_until: None,
                max_wait_s: 600,
            }),
            &intent,
        ),
    );
    assert!(is_run_here(&unknown));
}

#[test]
fn clock_skew_or_a_clock_jump_never_extends_the_wait() {
    // A claim far into the future: only the lander's monotonic deadline bounds the wait.
    let records = vec![carry(Some(B), "c-b", 1_000, 1_000_000)];
    assert!(matches!(
        lander(&records, 1_010, Some(1_005), None, 30),
        CarryDecision::Wait { wait_s: 30, .. }
    ));
    // The wall clock jumps back 500 s: the head folded at 1_005 now reads as the future, which is
    // doubt, so the lander runs here rather than wait longer.
    assert!(is_run_here(&lander(&records, 510, Some(1_005), None, 30)));
    // Even where the head and the claim look sane after the jump, so that the claim's own bound
    // reads ~1e6 s away, the remaining monotonic deadline still caps the wait.
    let jumped = vec![carry(Some(B), "c-b", 500, 1_000_000)];
    assert!(matches!(
        lander(&jumped, 510, Some(505), None, 20),
        CarryDecision::Wait { wait_s: 20, .. }
    ));
    // A carry written further ahead of this clock than max_skew_s is doubt.
    let ahead = vec![carry(Some(B), "c-b", 2_000, 3_000)];
    assert!(is_run_here(&lander(&ahead, 1_010, Some(1_005), None, 600)));
    // Within the skew budget it still counts.
    let near = vec![carry(Some(B), "c-b", 1_030, 3_000)];
    assert!(matches!(
        lander(&near, 1_010, Some(1_005), None, 600),
        CarryDecision::Wait { .. }
    ));
}

#[test]
fn a_green_or_red_verdict_closes_a_carry() {
    for success in [true, false] {
        for via in [None, Some(B)] {
            let records = vec![
                carry(Some(B), "c-b", 1_000, 1_600),
                verdict(via, CLOSURE, success),
            ];
            assert!(
                live_carries(&records, &admits, Some(A), 1_010).is_empty(),
                "a verdict success={success} via={via:?} settles the carry"
            );
            assert_eq!(carrier(&records, A, 1_010), CarryDecision::Run);
        }
    }
    // A verdict for another closure, or from an unadmitted device, does not.
    let records = vec![
        carry(Some(B), "c-b", 1_000, 1_600),
        verdict(Some(B), "ffff", true),
        verdict(Some(C), CLOSURE, true),
    ];
    assert_eq!(live_carries(&records, &admits, Some(A), 1_010).len(), 1);
}

#[test]
fn the_announcers_settlement_of_the_intent_closes_a_carry() {
    let carried = carry(Some(A), "c-a", 1_000, 1_600);
    let announce = event(intent(), landing_slots("misses", &[], Some(B)));
    for kind in ["landed", "withdraw", "dismiss"] {
        let settle = event(intent(), landing_slots(kind, &[], Some(B)));
        let records = vec![announce.clone(), carried.clone(), settle];
        assert!(
            live_carries(&records, &admits, Some(C), 1_010).is_empty(),
            "the announcer's {kind} settles the carry"
        );
    }
    // Superseded: the announcer's crossing intent event names this one as its predecessor.
    let successor = event(
        cid("intent-2"),
        landing_slots("intent", &[("supersedes", intent().to_string())], Some(B)),
    );
    let records = vec![announce.clone(), carried.clone(), successor];
    assert!(live_carries(&records, &admits, Some(C), 1_010).is_empty());
    // Settlement by a device that did not announce the intent does not count.
    let stranger = event(intent(), landing_slots("withdraw", &[], Some(A)));
    let records = vec![announce, carried.clone(), stranger];
    assert_eq!(live_carries(&records, &admits, Some(C), 1_010).len(), 1);

    // An intent this device raised settles by its own fold.
    let atom = FlowRecord::Intent(Intent {
        action: ReaVerb::Produce,
        resource_spec: ResourceSpec {
            quantity: None,
            classified_as: vec![INTENT_TAG.to_string(), CHANGE.to_string()],
        },
        in_scope_of: cid("scope"),
        raised_by: AgentRef("human:test".to_string()),
    });
    let own_intent = atom.cid().expect("an intent always encodes");
    let own_carry = Carry {
        via: Some(B),
        intent: own_intent,
        closure: CLOSURE,
        commitment: "c-b",
        at_s: 1_000,
        valid_until: 1_600,
        dseq: 1,
    }
    .record();
    let withdraw = event(own_intent, landing_slots("withdraw", &[], None));
    let live_before = vec![(own_intent, atom.clone()), own_carry.clone()];
    assert_eq!(live_carries(&live_before, &admits, Some(A), 1_010).len(), 1);
    let records = vec![(own_intent, atom), own_carry, withdraw];
    assert!(live_carries(&records, &admits, Some(A), 1_010).is_empty());
}

#[test]
fn a_carry_for_another_closure_intent_or_from_an_unadmitted_device_is_ignored() {
    let other_closure = Carry {
        via: Some(B),
        intent: intent(),
        closure: "ffff",
        commitment: "c-b",
        at_s: 1_000,
        valid_until: 1_600,
        dseq: 1,
    };
    let other_intent = Carry {
        intent: cid("intent-other"),
        closure: CLOSURE,
        ..other_closure
    };
    let records = vec![
        other_closure.record(),
        other_intent.record(),
        carry(Some(C), "c-c", 1_000, 1_600),
    ];
    assert_eq!(carrier(&records, A, 1_010), CarryDecision::Run);
    assert!(is_run_here(&lander(
        &records,
        1_010,
        Some(1_005),
        None,
        600
    )));
    // The unadmitted device's carry is not live at all; the other two are live for their own miss.
    let live = live_carries(&records, &admits, Some(A), 1_010);
    assert_eq!(live.len(), 2);
    assert!(live.iter().all(|carry| carry.carrier.as_deref() == Some(B)));

    // A record that names two writers names none, and this device's own record folded back to it
    // is not a second carrier.
    let (_, FlowRecord::Event(mut doubled)) = carry(Some(B), "c-b", 1_000, 1_600) else {
        unreachable!("carry() builds an event")
    };
    doubled.classified_as.push(format!("via:{A}"));
    let doubled = FlowRecord::Event(doubled);
    let records = vec![
        (doubled.cid().expect("encodes"), doubled),
        carry(Some(A), "c-a", 1_000, 1_600),
    ];
    assert!(live_carries(&records, &admits, Some(A), 1_010).is_empty());
}

#[test]
fn a_lapsed_carry_cannot_suppress_execution() {
    let records = vec![carry(Some(B), "c-b", 1_000, 1_100)];
    assert_eq!(carrier(&records, A, 1_500), CarryDecision::Run);
    assert!(is_run_here(&lander(
        &records,
        1_500,
        Some(1_495),
        None,
        600
    )));
}

#[test]
fn a_lander_never_waits_on_its_own_devices_carry() {
    let records = vec![carry(None, "c-a", 1_000, 1_600)];
    assert!(is_run_here(&lander(
        &records,
        1_010,
        Some(1_005),
        None,
        600
    )));
    assert_eq!(carrier(&records, A, 1_010), CarryDecision::Run);
}
