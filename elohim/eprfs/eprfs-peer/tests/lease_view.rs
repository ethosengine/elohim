//! The steward lease view: whether a claim on a steward-scoped resource is effective, pending,
//! unconfirmed or held by another device (spec §3.4; review focus items 1 and 2).
//!
//! Every case runs the view over berth records shaped as T1 folds them, own events bare and
//! another device's carrying `via:<did>`, against a head table and the roster's admission.

mod support;

use std::collections::BTreeMap;

use elohim_epr_rea::FlowEvent;
use eprfs_core::BlobCid;
use eprfs_peer::{
    steward_lease_view, Admission, DeviceHead, HeadTable, LeaseEvent, LeaseVerdict, OwnAnnounce,
    StewardScope, VIA_SLOT,
};
use support::{binding, contest, endpoint_row, event_with, genesis, household, Household, MEASURE};

const RESOURCE: &str = "dev-integration";
/// 2026-10-23T15:28:09Z, in unix seconds as berth stamps it.
const T0: f64 = 1_792_769_289.0;
const WAIT_S: u64 = 10;

fn ms(seconds: f64) -> u64 {
    (seconds * 1000.0).round() as u64
}

/// A berth record of `kind` on `resource` by `session`, written at `at` (unix seconds), with
/// extra `env:` fields.
fn record(kind: &str, session: &str, at: f64, extra: &[(&str, String)]) -> FlowEvent {
    let mut env = vec![
        format!("env:at={at:.3}"),
        format!("env:kind={kind}"),
        format!("env:resource={RESOURCE}"),
        format!("env:session={session}"),
    ];
    env.extend(extra.iter().map(|(k, v)| format!("env:{k}={v}")));
    env.sort();
    event_with(MEASURE, kind, env)
}

fn claim(session: &str, claimed_at: f64) -> FlowEvent {
    record(
        "claim",
        session,
        claimed_at,
        &[
            ("claimed_at", format!("{claimed_at:.3}")),
            ("class", "verify".to_string()),
        ],
    )
}

/// This device's own lease records, each at its position (1..) in its flow store.
fn own(events: &[FlowEvent]) -> Vec<LeaseEvent> {
    events
        .iter()
        .enumerate()
        .filter_map(|(i, e)| LeaseEvent::from_event(e, i as u64 + 1))
        .collect()
}

/// Another device's events as this device folded them: each carrying `via:<device>`.
fn folded(device: &str, events: &[FlowEvent]) -> Vec<LeaseEvent> {
    events
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let mut event = e.clone();
            event.classified_as.push(format!("{VIA_SLOT}{device}"));
            LeaseEvent::from_event(&event, 100 + i as u64).expect("a lease record")
        })
        .collect()
}

/// A head table on `me`'s side: its own announces (`(seq, events covered)`), and the heads it has
/// folded from others (`device → (acknowledged seq of mine, folded_at ms)`).
fn table(me: &str, own_log: &[(u64, u64)], heads: &[(&str, u64, u64)]) -> HeadTable {
    let mut table = HeadTable {
        own_log: own_log
            .iter()
            .map(|&(seq, events)| OwnAnnounce {
                seq,
                root: BlobCid::compute(format!("root-{seq}").as_bytes()),
                at: ms(T0) + seq,
                events,
            })
            .collect(),
        ..HeadTable::default()
    };
    for &(device, acked, folded_at) in heads {
        let mut folded = BTreeMap::new();
        folded.insert(me.to_string(), acked);
        table.devices.insert(
            device.to_string(),
            DeviceHead {
                seq: 9,
                root: BlobCid::compute(device.as_bytes()),
                at: folded_at,
                folded,
                folded_at,
                events_folded: 1,
            },
        );
    }
    table
}

fn verdict(
    me: &str,
    own: &[LeaseEvent],
    folded: &[LeaseEvent],
    heads: &HeadTable,
    admission: &Admission,
    now: u64,
) -> Option<LeaseVerdict> {
    steward_lease_view(me, own, folded, heads, admission, now, WAIT_S).remove(RESOURCE)
}

fn effective(session: &str) -> LeaseVerdict {
    LeaseVerdict::Effective {
        session: session.to_string(),
    }
}

#[test]
fn a_claim_every_other_device_acknowledged_is_effective() {
    let h = household();
    let own = own(&[claim("claude-1", T0)]);
    // Head 3 was projected from both events of A's store: the claim is in it. B acknowledges 3.
    let heads = table(&h.a.did, &[(2, 0), (3, 1)], &[(&h.b.did, 3, ms(T0) + 1500)]);
    assert_eq!(
        verdict(&h.a.did, &own, &[], &heads, &h.admission(), ms(T0) + 2000),
        Some(effective("claude-1"))
    );
}

#[test]
fn an_unacknowledged_claim_is_pending_then_unconfirmed_naming_the_device() {
    let h = household();
    let own = own(&[claim("claude-1", T0)]);
    // B's last folded head is three minutes old and acknowledges only seq 2, from before the claim.
    let last_seen = ms(T0) - 180_000;
    let heads = table(&h.a.did, &[(2, 0), (3, 1)], &[(&h.b.did, 2, last_seen)]);
    let admission = h.admission();
    assert_eq!(
        verdict(&h.a.did, &own, &[], &heads, &admission, ms(T0) + 9_999),
        Some(LeaseVerdict::Pending {
            session: "claude-1".into(),
            waiting_on: vec![h.b.did.clone()],
        })
    );
    assert_eq!(
        verdict(&h.a.did, &own, &[], &heads, &admission, ms(T0) + 10_000),
        Some(LeaseVerdict::Unconfirmed {
            session: "claude-1".into(),
            device: h.b.did.clone(),
            last_seen: Some(last_seen),
        })
    );

    // A device never folded is unconfirmed with no last-seen time: never read as free.
    let never = table(&h.a.did, &[(3, 1)], &[]);
    assert_eq!(
        verdict(&h.a.did, &own, &[], &never, &admission, ms(T0) + 60_000),
        Some(LeaseVerdict::Unconfirmed {
            session: "claude-1".into(),
            device: h.b.did.clone(),
            last_seen: None,
        })
    );
}

#[test]
fn an_ack_of_a_head_projected_before_the_claim_reached_the_store_does_not_confirm_it() {
    let h = household();
    // Head 3 was announced after the claim's stamp, but projected from a store that held only one
    // event: the claim (the store's second event) is not in it. B acknowledging 3 proves nothing.
    let own = own(&[
        record("moor", "claude-1", T0 - 5.0, &[]),
        claim("claude-1", T0),
    ]);
    let heads = table(&h.a.did, &[(3, 1)], &[(&h.b.did, 3, ms(T0) + 900)]);
    assert!(matches!(
        verdict(&h.a.did, &own, &[], &heads, &h.admission(), ms(T0) + 1000),
        Some(LeaseVerdict::Pending { .. })
    ));
    // No own head has carried the claim yet: pending too, never effective.
    let none = table(&h.a.did, &[], &[(&h.b.did, 7, ms(T0) + 900)]);
    assert!(matches!(
        verdict(&h.a.did, &own, &[], &none, &h.admission(), ms(T0) + 1000),
        Some(LeaseVerdict::Pending { .. })
    ));
}

#[test]
fn a_folded_overlapping_lease_is_held_by_its_holder() {
    let h = household();
    let theirs = folded(&h.a.did, &[claim("claude-1", T0)]);
    let heads = table(&h.b.did, &[], &[(&h.a.did, 0, ms(T0) + 500)]);
    let admission = h.admission();
    let held = Some(LeaseVerdict::HeldBy {
        device: h.a.did.clone(),
        session: "claude-1".into(),
        since: ms(T0),
        overriding: false,
    });
    // Before B claims (an owner check), and after: B's later claim is refused at once, no wait.
    assert_eq!(
        verdict(&h.b.did, &[], &theirs, &heads, &admission, ms(T0) + 1000),
        held
    );
    let mine = own(&[claim("codex-1", T0 + 30.0)]);
    assert_eq!(
        verdict(
            &h.b.did,
            &mine,
            &theirs,
            &heads,
            &admission,
            ms(T0) + 30_001
        ),
        held
    );

    // Released on A and folded: B's claim no longer meets it.
    let released = folded(
        &h.a.did,
        &[
            claim("claude-1", T0),
            record("release", "claude-1", T0 + 20.0, &[]),
        ],
    );
    let acked = table(&h.b.did, &[(5, 1)], &[(&h.a.did, 5, ms(T0) + 31_000)]);
    assert_eq!(
        verdict(
            &h.b.did,
            &mine,
            &released,
            &acked,
            &admission,
            ms(T0) + 31_500
        ),
        Some(effective("codex-1"))
    );

    // Past its ttl a folded lease no longer binds.
    let lapsed = folded(
        &h.a.did,
        &[record(
            "claim",
            "claude-1",
            T0,
            &[("claimed_at", format!("{T0:.3}")), ("ttl_s", "20".into())],
        )],
    );
    assert_eq!(
        verdict(
            &h.b.did,
            &mine,
            &lapsed,
            &acked,
            &admission,
            ms(T0) + 31_500
        ),
        Some(effective("codex-1"))
    );
}

/// Both devices claim with the link down, then each folds the other's claim. The view, run from
/// each side over the same records, names the same winner.
fn concurrent(h: &Household, a_at: f64, b_at: f64) -> (Option<LeaseVerdict>, Option<LeaseVerdict>) {
    let a_claim = [claim("claude-1", a_at)];
    let b_claim = [claim("codex-1", b_at)];
    let admission = h.admission();
    let now = ms(a_at.max(b_at)) + 2_000;
    // Each has announced the claim (seq 4, covering it) and the other has acknowledged it.
    let on_a = verdict(
        &h.a.did,
        &own(&a_claim),
        &folded(&h.b.did, &b_claim),
        &table(&h.a.did, &[(4, 1)], &[(&h.b.did, 4, now - 500)]),
        &admission,
        now,
    );
    let on_b = verdict(
        &h.b.did,
        &own(&b_claim),
        &folded(&h.a.did, &a_claim),
        &table(&h.b.did, &[(4, 1)], &[(&h.a.did, 4, now - 500)]),
        &admission,
        now,
    );
    (on_a, on_b)
}

#[test]
fn a_concurrent_pair_is_settled_the_same_way_on_both_devices() {
    let h = household();
    // The fixture's device A has the lexically lower did:key, as the story's examples assume.
    assert!(h.a.did < h.b.did);
    let a_wins = |at: f64| LeaseVerdict::HeldBy {
        device: h.a.did.clone(),
        session: "claude-1".into(),
        since: ms(at),
        overriding: false,
    };
    let b_wins = |at: f64| LeaseVerdict::HeldBy {
        device: h.b.did.clone(),
        session: "codex-1".into(),
        since: ms(at),
        overriding: false,
    };

    // The earlier claim wins.
    let (on_a, on_b) = concurrent(&h, T0 + 0.2, T0 + 0.7);
    assert_eq!(on_a, Some(effective("claude-1")));
    assert_eq!(on_b, Some(a_wins(T0 + 0.2)));

    let (on_a, on_b) = concurrent(&h, T0 + 0.7, T0 + 0.2);
    assert_eq!(on_a, Some(b_wins(T0 + 0.2)));
    assert_eq!(on_b, Some(effective("codex-1")));

    // On equal claim times, the lower device did:key wins.
    let (on_a, on_b) = concurrent(&h, T0 + 0.2, T0 + 0.2);
    assert_eq!(on_a, Some(effective("claude-1")));
    assert_eq!(on_b, Some(a_wins(T0 + 0.2)));
}

#[test]
fn a_contested_devices_leases_are_ignored_and_it_is_not_waited_for() {
    let h = household();
    // A contests B's binding: B is no longer admitted on A.
    let mut rows = h.rows.clone();
    rows.push(contest(&h.a, &binding(&h.a, &h.b)));
    let admission = Admission::from_roster(&rows, &h.a.did, &*h.verify);
    assert!(!admission.admits_device(&h.b.did));

    let theirs = folded(
        &h.b.did,
        &[record(
            "claim",
            "codex-1",
            T0,
            &[("claimed_at", format!("{T0:.3}"))],
        )],
    );
    let heads = table(&h.a.did, &[], &[(&h.b.did, 0, ms(T0) - 180_000)]);
    // B's lease no longer pins the resource: there is nothing to report before A claims.
    assert_eq!(
        verdict(&h.a.did, &[], &theirs, &heads, &admission, ms(T0) + 60_000),
        None
    );
    // A's claim is effective at once: B is neither a holder nor a device to wait for.
    let mine = own(&[claim("claude-1", T0 + 60.0)]);
    assert_eq!(
        verdict(
            &h.a.did,
            &mine,
            &theirs,
            &heads,
            &admission,
            ms(T0) + 60_001
        ),
        Some(effective("claude-1"))
    );
}

#[test]
fn with_no_other_admitted_device_a_claim_is_effective_at_once() {
    let h = household();
    let rows = vec![genesis(&h.a), endpoint_row(&h.a, &h.a, &h.key_a)];
    let admission = Admission::from_roster(&rows, &h.a.did, &*h.verify);
    let mine = own(&[claim("claude-1", T0)]);
    // No head was ever announced or folded: nothing to wait for.
    assert_eq!(
        verdict(
            &h.a.did,
            &mine,
            &[],
            &HeadTable::default(),
            &admission,
            ms(T0)
        ),
        Some(effective("claude-1"))
    );
}

#[test]
fn an_offline_ok_override_is_effective_for_its_holder_and_binds_the_other_device() {
    let h = household();
    let claimed = format!("{T0:.3}");
    let events = [
        claim("codex-1", T0),
        record(
            "override",
            "codex-1",
            T0 + 10.0,
            &[
                ("claimed_at", claimed),
                ("holder", "codex-1".into()),
                ("overrode", h.a.did.clone()),
            ],
        ),
    ];
    // On B, the silent device A never acknowledged: the override makes it effective anyway.
    let heads = table(&h.b.did, &[(4, 2)], &[(&h.a.did, 1, ms(T0) - 180_000)]);
    assert_eq!(
        verdict(
            &h.b.did,
            &own(&events),
            &[],
            &heads,
            &h.admission(),
            ms(T0) + 10_500
        ),
        Some(effective("codex-1"))
    );
    // On A, once folded, it is a lease held by B, marked as an override.
    let heads = table(&h.a.did, &[], &[(&h.b.did, 0, ms(T0) + 200_000)]);
    let mine = own(&[claim("claude-1", T0 + 200.0)]);
    assert_eq!(
        verdict(
            &h.a.did,
            &mine,
            &folded(&h.b.did, &events),
            &heads,
            &h.admission(),
            ms(T0) + 200_001
        ),
        Some(LeaseVerdict::HeldBy {
            device: h.b.did.clone(),
            session: "codex-1".into(),
            since: ms(T0),
            overriding: true,
        })
    );
}

#[test]
fn the_steward_scope_reads_names_and_lane_prefix_from_the_policy() {
    let policy = serde_json::json!({
        "berth": {
            "resources": {
                "mesh": 1,
                "dev-integration": {"capacity": 1, "scope": "steward"},
                "alpha-fleet": {"capacity": 1, "scope": "steward"},
                "cargo": {"capacity": 2, "scope": "container"},
                "lane:": {"capacity": 1, "scope": "steward"}
            },
            "channel": {"confirm_wait_s": 7, "freshness_s": 90}
        }
    });
    let scope = StewardScope::from_policy(&policy).expect("reads");
    assert!(scope.covers("dev-integration"));
    assert!(scope.covers("alpha-fleet"));
    assert!(scope.covers("lane:t6-lease-view"));
    assert!(!scope.covers("lane:"));
    assert!(!scope.covers("mesh"));
    assert!(!scope.covers("cargo"));
    assert!(!scope.covers("jenkins-dispatch"));
    assert_eq!(scope.confirm_wait_s(), 7);
    // A policy that declares none: nothing is steward-scoped, and the wait is the spec's 10 s.
    let none = StewardScope::from_policy(&serde_json::json!({})).expect("reads");
    assert!(!none.covers("dev-integration"));
    assert_eq!(none.confirm_wait_s(), 10);
}
