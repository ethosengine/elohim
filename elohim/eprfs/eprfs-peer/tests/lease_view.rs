//! The steward lease view: whether a claim on a steward-scoped resource is effective, pending,
//! unconfirmed or held by another device (spec §3.4; review focus items 1 and 2).
//!
//! Every case runs the view over a device's flow store as an ordered list of berth records shaped
//! as T1 folds them — its own bare, another device's carrying `via:<did>` — against a head table
//! and the roster's admission.

mod support;

use std::collections::BTreeMap;

use elohim_epr_rea::{atom_cid, FlowEvent};
use eprfs_core::BlobCid;
use eprfs_peer::{
    steward_lease_view, Admission, DeviceHead, HeadTable, LeaseStore, LeaseVerdict, OwnAnnounce,
    PeerError, StewardScope, VIA_SLOT,
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
    claim_seen(session, claimed_at, &[])
}

/// A claim carrying the claimant's acknowledgement vector when it wrote: for each other device,
/// the highest of its heads the claimant had folded (berth writes it as compact JSON).
fn claim_seen(session: &str, claimed_at: f64, seen: &[(&str, u64)]) -> FlowEvent {
    let mut extra = vec![
        ("claimed_at", format!("{claimed_at:.3}")),
        ("class", "verify".to_string()),
    ];
    if !seen.is_empty() {
        let seen: BTreeMap<&str, u64> = seen.iter().copied().collect();
        extra.push(("seen", serde_json::to_string(&seen).expect("json")));
    }
    record("claim", session, claimed_at, &extra)
}

fn override_of(session: &str, claimed_at: f64, at: f64, silent: &str) -> FlowEvent {
    record(
        "override",
        session,
        at,
        &[
            ("claimed_at", format!("{claimed_at:.3}")),
            ("holder", session.to_string()),
            ("overrode", silent.to_string()),
        ],
    )
}

/// One record of a device's flow store: its own, or folded from another device.
enum At {
    Own(FlowEvent),
    From(String, FlowEvent),
}

fn own(event: FlowEvent) -> At {
    At::Own(event)
}

fn from(device: &str, event: FlowEvent) -> At {
    At::From(device.to_string(), event)
}

/// A device's flow store, in append order.
fn store(entries: Vec<At>) -> LeaseStore {
    LeaseStore::from_events(entries.into_iter().map(|entry| {
        let event = match entry {
            At::Own(event) => event,
            At::From(device, mut event) => {
                event.classified_as.push(format!("{VIA_SLOT}{device}"));
                event
            }
        };
        (atom_cid(&event).expect("cid"), event)
    }))
}

/// A head table on `me`'s side: its own announces (`(seq, events covered)`, each naming the store
/// event it counted up to), and the heads it has folded from others (`device → (acknowledged seq
/// of mine, folded_at ms)`).
fn table(
    me: &str,
    own_log: &[(u64, u64)],
    store: &LeaseStore,
    heads: &[(&str, u64, u64)],
) -> HeadTable {
    let mut table = HeadTable {
        own_log: own_log
            .iter()
            .map(|&(seq, events)| OwnAnnounce {
                seq,
                root: BlobCid::compute(format!("root-{seq}").as_bytes()),
                at: ms(T0) + seq,
                events,
                last: events
                    .checked_sub(1)
                    .and_then(|i| store.events.get(i as usize))
                    .map(ToString::to_string),
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
    store: &LeaseStore,
    heads: &HeadTable,
    admission: &Admission,
    now: u64,
) -> Option<LeaseVerdict> {
    steward_lease_view(me, store, heads, admission, now, WAIT_S)
        .expect("this device is admitted")
        .remove(RESOURCE)
}

fn effective(session: &str) -> LeaseVerdict {
    LeaseVerdict::Effective {
        session: session.to_string(),
    }
}

fn held(device: &str, session: &str, since: f64, overriding: bool) -> LeaseVerdict {
    LeaseVerdict::HeldBy {
        device: device.to_string(),
        session: session.to_string(),
        since: ms(since),
        overriding,
    }
}

#[test]
fn a_claim_every_other_device_acknowledged_is_effective() {
    let h = household();
    let s = store(vec![own(claim("claude-1", T0))]);
    // Head 3 was projected from the store's one event: the claim is in it. B acknowledges 3.
    let heads = table(
        &h.a.did,
        &[(2, 0), (3, 1)],
        &s,
        &[(&h.b.did, 3, ms(T0) + 1500)],
    );
    assert_eq!(
        verdict(&h.a.did, &s, &heads, &h.admission(), ms(T0) + 2000),
        Some(effective("claude-1"))
    );
}

#[test]
fn an_unacknowledged_claim_is_pending_then_unconfirmed_naming_the_device() {
    let h = household();
    let s = store(vec![own(claim("claude-1", T0))]);
    // B's last folded head is three minutes old and acknowledges only seq 2, from before the claim.
    let last_seen = ms(T0) - 180_000;
    let heads = table(&h.a.did, &[(2, 0), (3, 1)], &s, &[(&h.b.did, 2, last_seen)]);
    let admission = h.admission();
    assert_eq!(
        verdict(&h.a.did, &s, &heads, &admission, ms(T0) + 9_999),
        Some(LeaseVerdict::Pending {
            session: "claude-1".into(),
            waiting_on: vec![h.b.did.clone()],
        })
    );
    assert_eq!(
        verdict(&h.a.did, &s, &heads, &admission, ms(T0) + 10_000),
        Some(LeaseVerdict::Unconfirmed {
            session: "claude-1".into(),
            device: h.b.did.clone(),
            last_seen: Some(last_seen),
        })
    );

    // A device never folded is unconfirmed with no last-seen time: never read as free.
    let never = table(&h.a.did, &[(3, 1)], &s, &[]);
    assert_eq!(
        verdict(&h.a.did, &s, &never, &admission, ms(T0) + 60_000),
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
    let s = store(vec![
        own(record("moor", "claude-1", T0 - 5.0, &[])),
        own(claim("claude-1", T0)),
    ]);
    let heads = table(&h.a.did, &[(3, 1)], &s, &[(&h.b.did, 3, ms(T0) + 900)]);
    assert!(matches!(
        verdict(&h.a.did, &s, &heads, &h.admission(), ms(T0) + 1000),
        Some(LeaseVerdict::Pending { .. })
    ));
    // No own head has carried the claim yet: pending too, never effective.
    let none = table(&h.a.did, &[], &s, &[(&h.b.did, 7, ms(T0) + 900)]);
    assert!(matches!(
        verdict(&h.a.did, &s, &none, &h.admission(), ms(T0) + 1000),
        Some(LeaseVerdict::Pending { .. })
    ));
}

#[test]
fn an_own_log_that_counted_another_store_never_confirms_a_claim() {
    let h = household();
    let s = store(vec![own(claim("claude-1", T0))]);
    let admission = h.admission();
    // Head 3 counted one event, but the event it names is not this store's first: the store at
    // this path was reset or replaced since. B acknowledged 3, and still the claim is pending.
    let mut heads = table(&h.a.did, &[(3, 1)], &s, &[(&h.b.did, 3, ms(T0) + 900)]);
    heads.own_log[0].last = Some(BlobCid::compute(b"another store's event").to_string());
    assert!(matches!(
        verdict(&h.a.did, &s, &heads, &admission, ms(T0) + 1000),
        Some(LeaseVerdict::Pending { .. })
    ));
    // Nor does one that counted more events than this store holds.
    let mut heads = table(&h.a.did, &[(3, 1)], &s, &[(&h.b.did, 3, ms(T0) + 900)]);
    heads.own_log[0].events = 5;
    assert!(matches!(
        verdict(&h.a.did, &s, &heads, &admission, ms(T0) + 1000),
        Some(LeaseVerdict::Pending { .. })
    ));
}

#[test]
fn a_folded_overlapping_lease_is_held_by_its_holder() {
    let h = household();
    let admission = h.admission();
    let a_claim = || from(&h.a.did, claim("claude-1", T0));
    let held_by_a = Some(held(&h.a.did, "claude-1", T0, false));
    // Before B claims (an owner check): held.
    let s = store(vec![a_claim()]);
    let heads = table(&h.b.did, &[], &s, &[(&h.a.did, 0, ms(T0) + 500)]);
    assert_eq!(
        verdict(&h.b.did, &s, &heads, &admission, ms(T0) + 1000),
        held_by_a
    );
    // A claim B writes after folding A's: held at once, no wait.
    let s = store(vec![a_claim(), own(claim("codex-1", T0 + 30.0))]);
    let heads = table(&h.b.did, &[(5, 2)], &s, &[(&h.a.did, 5, ms(T0) + 30_500)]);
    assert_eq!(
        verdict(&h.b.did, &s, &heads, &admission, ms(T0) + 30_001),
        held_by_a
    );

    // Released on A and folded: B's claim no longer meets it.
    let s = store(vec![
        a_claim(),
        from(&h.a.did, record("release", "claude-1", T0 + 20.0, &[])),
        own(claim("codex-1", T0 + 30.0)),
    ]);
    let heads = table(&h.b.did, &[(5, 3)], &s, &[(&h.a.did, 5, ms(T0) + 31_000)]);
    assert_eq!(
        verdict(&h.b.did, &s, &heads, &admission, ms(T0) + 31_500),
        Some(effective("codex-1"))
    );

    // Past its ttl a folded lease no longer binds.
    let s = store(vec![
        from(
            &h.a.did,
            record(
                "claim",
                "claude-1",
                T0,
                &[("claimed_at", format!("{T0:.3}")), ("ttl_s", "20".into())],
            ),
        ),
        own(claim("codex-1", T0 + 30.0)),
    ]);
    let heads = table(&h.b.did, &[(5, 2)], &s, &[(&h.a.did, 5, ms(T0) + 31_000)]);
    assert_eq!(
        verdict(&h.b.did, &s, &heads, &admission, ms(T0) + 31_500),
        Some(effective("codex-1"))
    );
}

#[test]
fn another_devices_records_are_replayed_in_its_order_not_the_order_they_were_folded() {
    let h = household();
    // A fold appends a head's events in its channel tree's order, which is by CID: A's release can
    // land here before the claim it closes. Replayed in A's own order (its record times), the lease
    // is closed.
    let s = store(vec![
        from(&h.a.did, record("release", "claude-1", T0 + 5.0, &[])),
        from(&h.a.did, claim("claude-1", T0)),
    ]);
    let heads = table(&h.b.did, &[], &s, &[(&h.a.did, 3, ms(T0) + 6_000)]);
    assert_eq!(
        verdict(&h.b.did, &s, &heads, &h.admission(), ms(T0) + 7_000),
        None
    );
}

/// Both devices claim with the link down, so each wrote its claim before folding the other's;
/// then each folds the other's. The view, run from each side over the same records, names the
/// same winner.
fn concurrent(
    h: &Household,
    a: Vec<FlowEvent>,
    b: Vec<FlowEvent>,
    now: u64,
) -> [Option<LeaseVerdict>; 2] {
    let admission = h.admission();
    let side = |me: &str, mine: &[FlowEvent], other: &str, theirs: &[FlowEvent]| {
        let s = store(
            mine.iter()
                .cloned()
                .map(own)
                .chain(theirs.iter().cloned().map(|e| from(other, e)))
                .collect(),
        );
        // Each has announced its own records and the other has acknowledged them.
        let covered = mine.len() as u64;
        let heads = table(me, &[(4, covered)], &s, &[(other, 4, now - 500)]);
        verdict(me, &s, &heads, &admission, now)
    };
    [
        side(&h.a.did, &a, &h.b.did, &b),
        side(&h.b.did, &b, &h.a.did, &a),
    ]
}

#[test]
fn a_concurrent_pair_is_settled_the_same_way_on_both_devices() {
    let h = household();
    // The fixture's device A has the lexically lower did:key, as the story's examples assume.
    assert!(h.a.did < h.b.did);
    let pair = |a_at: f64, b_at: f64| {
        concurrent(
            &h,
            vec![claim("claude-1", a_at)],
            vec![claim("codex-1", b_at)],
            ms(a_at.max(b_at)) + 2_000,
        )
    };
    // The earlier claim wins.
    assert_eq!(
        pair(T0 + 0.2, T0 + 0.7),
        [
            Some(effective("claude-1")),
            Some(held(&h.a.did, "claude-1", T0 + 0.2, false))
        ]
    );
    assert_eq!(
        pair(T0 + 0.7, T0 + 0.2),
        [
            Some(held(&h.b.did, "codex-1", T0 + 0.2, false)),
            Some(effective("codex-1"))
        ]
    );
    // On equal claim times, the lower device did:key wins.
    assert_eq!(
        pair(T0 + 0.2, T0 + 0.2),
        [
            Some(effective("claude-1")),
            Some(held(&h.a.did, "claude-1", T0 + 0.2, false))
        ]
    );
}

#[test]
fn two_overrides_taken_while_the_link_was_down_are_settled_by_the_tie_break() {
    let h = household();
    let a = vec![
        claim("claude-1", T0 + 0.2),
        override_of("claude-1", T0 + 0.2, T0 + 10.2, &h.b.did),
    ];
    let b = vec![
        claim("codex-1", T0 + 0.7),
        override_of("codex-1", T0 + 0.7, T0 + 10.7, &h.a.did),
    ];
    // Exactly one holds once both have folded the other's override: the earlier claim.
    assert_eq!(
        concurrent(&h, a, b, ms(T0 + 30.0)),
        [
            Some(effective("claude-1")),
            Some(held(&h.a.did, "claude-1", T0 + 0.2, true))
        ]
    );
}

#[test]
fn a_lease_the_claimant_had_already_seen_is_not_overturned_by_its_earlier_clock() {
    let h = household();
    let admission = h.admission();
    // A's claim, carried by A's head 4, is effective: B folded and acknowledged it.
    let a_claim = claim("claude-1", T0 + 10.0);
    // B's clock runs 20 s behind. Having folded A's head 4, B claims anyway (it wrote before its
    // own view showed A's lease), stamping an earlier claimed_at and carrying seen = {A: 4}.
    let b_claim = claim_seen("codex-1", T0 - 10.0, &[(&h.a.did, 4)]);
    let now = ms(T0 + 12.0);

    // On A: B's claim saw A's, so A's lease stays effective whatever the clocks say.
    let on_a = store(vec![own(a_claim.clone()), from(&h.b.did, b_claim.clone())]);
    let heads_a = table(&h.a.did, &[(4, 1)], &on_a, &[(&h.b.did, 4, now - 500)]);
    assert_eq!(
        verdict(&h.a.did, &on_a, &heads_a, &admission, now),
        Some(effective("claude-1"))
    );
    // On B: A's claim was in B's store before B wrote its own: held by A.
    let on_b = store(vec![from(&h.a.did, a_claim.clone()), own(b_claim)]);
    let heads_b = table(&h.b.did, &[(6, 2)], &on_b, &[(&h.a.did, 6, now - 500)]);
    assert_eq!(
        verdict(&h.b.did, &on_b, &heads_b, &admission, now),
        Some(held(&h.a.did, "claude-1", T0 + 10.0, false))
    );

    // Had B's seen been stale (its last fold recorded before A's head 4), A cannot tell B saw it:
    // both sides yield rather than both hold, and B's release then hands it back to A.
    let stale = claim_seen("codex-1", T0 - 10.0, &[(&h.a.did, 3)]);
    let on_a = store(vec![own(a_claim.clone()), from(&h.b.did, stale.clone())]);
    let on_b = store(vec![from(&h.a.did, a_claim), own(stale)]);
    let a = verdict(
        &h.a.did,
        &on_a,
        &table(&h.a.did, &[(4, 1)], &on_a, &[(&h.b.did, 6, now - 500)]),
        &admission,
        now,
    );
    let b = verdict(
        &h.b.did,
        &on_b,
        &table(&h.b.did, &[(6, 2)], &on_b, &[(&h.a.did, 6, now - 500)]),
        &admission,
        now,
    );
    assert!(matches!(a, Some(LeaseVerdict::HeldBy { .. })), "{a:?}");
    assert!(matches!(b, Some(LeaseVerdict::HeldBy { .. })), "{b:?}");
}

#[test]
fn a_contested_devices_leases_are_ignored_and_it_is_not_waited_for() {
    let h = household();
    // A contests B's binding: B is no longer admitted on A.
    let mut rows = h.rows.clone();
    rows.push(contest(&h.a, &binding(&h.a, &h.b)));
    let admission = Admission::from_roster(&rows, &h.a.did, &*h.verify);
    assert!(!admission.admits_device(&h.b.did));

    let theirs = || from(&h.b.did, claim("codex-1", T0));
    // B's lease no longer pins the resource: there is nothing to report before A claims.
    let s = store(vec![theirs()]);
    let heads = table(&h.a.did, &[], &s, &[(&h.b.did, 0, ms(T0) - 180_000)]);
    assert_eq!(
        verdict(&h.a.did, &s, &heads, &admission, ms(T0) + 60_000),
        None
    );
    // A's claim is effective at once: B is neither a holder nor a device to wait for.
    let s = store(vec![theirs(), own(claim("claude-1", T0 + 60.0))]);
    assert_eq!(
        verdict(&h.a.did, &s, &heads, &admission, ms(T0) + 60_001),
        Some(effective("claude-1"))
    );
    // And on B, which A's roster contests, there is no view at all: B is not admitted.
    let on_b = store(vec![own(claim("codex-1", T0))]);
    assert!(matches!(
        steward_lease_view(
            &h.b.did,
            &on_b,
            &HeadTable::default(),
            &admission,
            ms(T0),
            WAIT_S
        ),
        Err(PeerError::NotAdmitted { .. })
    ));
}

#[test]
fn with_no_other_admitted_device_a_claim_is_effective_at_once() {
    let h = household();
    let rows = vec![genesis(&h.a), endpoint_row(&h.a, &h.a, &h.key_a)];
    let admission = Admission::from_roster(&rows, &h.a.did, &*h.verify);
    let s = store(vec![own(claim("claude-1", T0))]);
    // No head was ever announced or folded: nothing to wait for.
    assert_eq!(
        verdict(&h.a.did, &s, &HeadTable::default(), &admission, ms(T0)),
        Some(effective("claude-1"))
    );
}

#[test]
fn an_offline_ok_override_is_effective_for_its_holder_and_binds_the_other_device() {
    let h = household();
    let events = [
        claim("codex-1", T0),
        override_of("codex-1", T0, T0 + 10.0, &h.a.did),
    ];
    // On B, the silent device A never acknowledged: the override makes it effective anyway.
    let on_b = store(events.iter().cloned().map(own).collect());
    let heads = table(
        &h.b.did,
        &[(4, 2)],
        &on_b,
        &[(&h.a.did, 1, ms(T0) - 180_000)],
    );
    assert_eq!(
        verdict(&h.b.did, &on_b, &heads, &h.admission(), ms(T0) + 10_500),
        Some(effective("codex-1"))
    );
    // On A, once folded, it is a lease held by B, marked as an override.
    let on_a = store(
        events
            .iter()
            .cloned()
            .map(|e| from(&h.b.did, e))
            .chain([own(claim("claude-1", T0 + 200.0))])
            .collect(),
    );
    let heads = table(&h.a.did, &[], &on_a, &[(&h.b.did, 0, ms(T0) + 200_000)]);
    assert_eq!(
        verdict(&h.a.did, &on_a, &heads, &h.admission(), ms(T0) + 200_001),
        Some(held(&h.b.did, "codex-1", T0, true))
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
