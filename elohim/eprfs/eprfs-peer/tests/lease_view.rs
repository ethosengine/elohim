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
    other_devices, steward_lease_view, Admission, DeviceHead, HeadTable, LeaseStore, LeaseVerdict,
    NackLeaf, OwnAnnounce, PeerError, Settlement, StewardScope, VIA_SLOT,
};
use support::{binding, contest, endpoint_row, event_with, genesis, household, Household, MEASURE};

const RESOURCE: &str = "dev-integration";
/// 2026-10-23T15:28:09Z, in unix seconds as berth stamps it.
const T0: f64 = 1_792_769_289.0;
const WAIT_S: u64 = 10;
const FENCE_MARGIN_S: u64 = 30;
const MAX_SKEW_S: u64 = 30;

const SETTLEMENT: Settlement = Settlement {
    confirm_wait_s: WAIT_S,
    fence_margin_s: FENCE_MARGIN_S,
    max_skew_s: MAX_SKEW_S,
};

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

fn claim_ttl(session: &str, claimed_at: f64, ttl_s: u64) -> FlowEvent {
    record(
        "claim",
        session,
        claimed_at,
        &[
            ("claimed_at", format!("{claimed_at:.3}")),
            ("class", "verify".to_string()),
            ("ttl_s", ttl_s.to_string()),
        ],
    )
}

fn renew(session: &str, claimed_at: f64, at: f64, ttl_s: u64) -> FlowEvent {
    record(
        "renew",
        session,
        at,
        &[
            ("claimed_at", format!("{claimed_at:.3}")),
            ("class", "verify".to_string()),
            ("ttl_s", ttl_s.to_string()),
        ],
    )
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
                nack: Default::default(),
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
    steward_lease_view(me, store, heads, admission, now, SETTLEMENT)
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
fn fenced_holder_self_demotes_when_a_peer_goes_silent() {
    let h = household();
    let s = store(vec![own(claim_ttl("claude-1", T0, 120))]);
    let admission = h.admission();
    let stale = table(&h.a.did, &[(3, 1)], &s, &[(&h.b.did, 3, ms(T0))]);
    assert!(matches!(
        verdict(&h.a.did, &s, &stale, &admission, ms(T0) + 90_001),
        Some(LeaseVerdict::Unconfirmed {
            ref device,
            fenced: true,
            ..
        }) if device == &h.b.did
    ));

    // A peer heard recently that acknowledged a renew inside the window keeps the holder.
    let renewed = store(vec![
        own(claim_ttl("claude-1", T0, 120)),
        own(renew("claude-1", T0, T0 + 40.0, 120)),
    ]);
    let fresh = table(
        &h.a.did,
        &[(3, 1), (4, 2)],
        &renewed,
        &[(&h.b.did, 4, ms(T0) + 90_001)],
    );
    assert_eq!(
        verdict(&h.a.did, &renewed, &fresh, &admission, ms(T0) + 90_001),
        Some(effective("claude-1"))
    );
}

#[test]
fn a_renew_in_flight_rides_on_the_last_acknowledged_one() {
    // The holder re-reads its lease right after writing a renew, before any head carrying it was
    // acknowledged: the claim B did acknowledge still binds B, so the holder stays effective.
    let h = household();
    let s = store(vec![
        own(claim_ttl("claude-1", T0, 120)),
        own(renew("claude-1", T0, T0 + 60.0, 120)),
    ]);
    let admission = h.admission();
    let claim_only = |now| table(&h.a.did, &[(3, 1), (4, 2)], &s, &[(&h.b.did, 3, now)]);
    assert_eq!(
        verdict(
            &h.a.did,
            &s,
            &claim_only(ms(T0) + 60_500),
            &admission,
            ms(T0) + 61_000
        ),
        Some(effective("claude-1"))
    );

    // Once the acknowledged claim is older than ttl - fence_margin (120 - 30 s), B may soon read
    // the lease as lapsed: the holder demotes itself although B is still heard.
    assert!(matches!(
        verdict(
            &h.a.did,
            &s,
            &claim_only(ms(T0) + 90_001),
            &admission,
            ms(T0) + 90_001
        ),
        Some(LeaseVerdict::Unconfirmed { fenced: true, .. })
    ));

    let renew_acked = table(
        &h.a.did,
        &[(3, 1), (4, 2)],
        &s,
        &[(&h.b.did, 4, ms(T0) + 90_001)],
    );
    assert_eq!(
        verdict(&h.a.did, &s, &renew_acked, &admission, ms(T0) + 90_001),
        Some(effective("claude-1"))
    );
}

#[test]
fn a_nacked_claim_is_refused_and_an_overflowed_nack_is_unconfirmed() {
    let h = household();
    let s = store(vec![own(claim("claude-1", T0))]);
    let claim_cid = s.events[0].to_string();
    let mut heads = table(&h.a.did, &[(3, 1)], &s, &[(&h.b.did, 3, ms(T0) + 500)]);
    heads.devices.get_mut(&h.b.did).expect("B head").nack = NackLeaf {
        refused: vec![],
        quarantined: vec![claim_cid],
        refused_total: 1,
    };
    assert_eq!(
        verdict(&h.a.did, &s, &heads, &h.admission(), ms(T0) + 1000),
        Some(LeaseVerdict::RefusedBy {
            session: "claude-1".into(),
            device: h.b.did.clone(),
        })
    );

    heads.devices.get_mut(&h.b.did).expect("B head").nack = NackLeaf {
        refused: vec![],
        quarantined: vec![],
        refused_total: 65,
    };
    assert!(matches!(
        verdict(
            &h.a.did,
            &s,
            &heads,
            &h.admission(),
            ms(T0) + 10_000
        ),
        Some(LeaseVerdict::Unconfirmed { ref device, .. }) if device == &h.b.did
    ));
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
            fenced: false,
            override_at: None,
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
            fenced: false,
            override_at: None,
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

/// A record as berth stamps it with its device's record counter, `dseq`.
fn counted(event: FlowEvent, dseq: u64) -> FlowEvent {
    let mut event = event;
    event.classified_as.push(format!("env:dseq={dseq}"));
    event
}

#[test]
fn another_devices_records_are_replayed_in_its_order_not_the_order_they_were_folded() {
    let h = household();
    let admission = h.admission();
    // A fold appends a head's events in its channel tree's order, which is by CID: A's release can
    // land here before the claim it closes. Replayed in A's own order, the lease is closed.
    let s = store(vec![
        from(&h.a.did, record("release", "claude-1", T0 + 5.0, &[])),
        from(&h.a.did, claim("claude-1", T0)),
    ]);
    let heads = table(&h.b.did, &[], &s, &[(&h.a.did, 3, ms(T0) + 6_000)]);
    assert_eq!(
        verdict(&h.b.did, &s, &heads, &admission, ms(T0) + 7_000),
        None
    );

    // A's order is its record counter, not its clock: A releases, its clock steps back 50 s, and
    // A re-claims. By record time the re-claim would sort before the release and the resource
    // would read free here while A holds it; by counter it is held.
    let s = store(vec![
        from(
            &h.a.did,
            counted(record("release", "claude-1", T0 + 100.0, &[]), 2),
        ),
        from(&h.a.did, counted(claim("claude-1", T0 + 50.0), 3)),
        from(&h.a.did, counted(claim("claude-1", T0), 1)),
    ]);
    let heads = table(&h.b.did, &[], &s, &[(&h.a.did, 3, ms(T0) + 101_000)]);
    assert_eq!(
        verdict(&h.b.did, &s, &heads, &admission, ms(T0) + 102_000),
        Some(held(&h.a.did, "claude-1", T0 + 50.0, false))
    );

    // Records folded before berth counted them sort by record time, ahead of every counted one.
    let s = store(vec![
        from(
            &h.a.did,
            counted(record("release", "claude-1", T0 - 100.0, &[]), 1),
        ),
        from(&h.a.did, claim("claude-1", T0)),
    ]);
    let heads = table(&h.b.did, &[], &s, &[(&h.a.did, 3, ms(T0) + 1_000)]);
    assert_eq!(
        verdict(&h.b.did, &s, &heads, &admission, ms(T0) + 2_000),
        None
    );
}

#[test]
fn this_devices_own_records_are_replayed_in_its_order_not_the_order_they_reached_its_store() {
    let h = household();
    let admission = h.admission();
    // A push is interrupted while its claim is being taken: the hook's release (counter 2) reaches
    // the store before the claim it closes (counter 1), because the two berth processes flushed
    // their folds in the other order. Replayed by counter, the lease is closed, here as on B.
    let s = store(vec![
        own(counted(record("release", "claude-1", T0 + 1.0, &[]), 2)),
        own(counted(claim("claude-1", T0), 1)),
    ]);
    let heads = table(&h.a.did, &[(3, 2)], &s, &[(&h.b.did, 3, ms(T0) + 2_000)]);
    assert_eq!(
        verdict(&h.a.did, &s, &heads, &admission, ms(T0) + 3_000),
        None,
        "no ghost lease"
    );
    // The session's next claim is not refused by that ghost. It is carried by the first head
    // projected from its own store position (4), not by its place in the counter order.
    let s = store(vec![
        own(counted(record("release", "claude-1", T0 + 1.0, &[]), 2)),
        own(counted(claim("claude-1", T0), 1)),
        own(counted(record("note", "claude-1", T0 + 2.0, &[]), 3)),
        own(counted(claim("codex-2", T0 + 5.0), 4)),
    ]);
    let heads = table(
        &h.a.did,
        &[(3, 3), (4, 4)],
        &s,
        &[(&h.b.did, 3, ms(T0) + 6_000)],
    );
    assert!(matches!(
        verdict(&h.a.did, &s, &heads, &admission, ms(T0) + 6_000),
        Some(LeaseVerdict::Pending { ref session, .. }) if session == "codex-2"
    ));
    let heads = table(
        &h.a.did,
        &[(3, 3), (4, 4)],
        &s,
        &[(&h.b.did, 4, ms(T0) + 6_000)],
    );
    assert_eq!(
        verdict(&h.a.did, &s, &heads, &admission, ms(T0) + 6_000),
        Some(effective("codex-2"))
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
    concurrent_with_last_seen(h, a, b, now, now - 500)
}

fn concurrent_with_last_seen(
    h: &Household,
    a: Vec<FlowEvent>,
    b: Vec<FlowEvent>,
    now: u64,
    last_seen: u64,
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
        let heads = table(me, &[(4, covered)], &s, &[(other, 4, last_seen)]);
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
        claim_ttl("claude-1", T0 + 0.2, 60),
        override_of("claude-1", T0 + 0.2, T0 + 10.2, &h.b.did),
        renew("claude-1", T0 + 0.2, T0 + 250.0, 60),
    ];
    let b = vec![
        claim_ttl("codex-1", T0 + 0.7, 60),
        override_of("codex-1", T0 + 0.7, T0 + 10.7, &h.a.did),
        renew("codex-1", T0 + 0.7, T0 + 250.0, 60),
    ];
    // Exactly one holds once both have folded the other's override: the earlier claim.
    assert_eq!(
        concurrent_with_last_seen(&h, a, b, ms(T0 + 300.0), ms(T0)),
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
            SETTLEMENT
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
fn override_waits_ttl_plus_skew_plus_margin() {
    let h = household();
    let events = [
        claim_ttl("codex-1", T0, 60),
        override_of("codex-1", T0, T0 + 10.0, &h.a.did),
        renew("codex-1", T0, T0 + 20.0, 60),
        renew("codex-1", T0, T0 + 40.0, 60),
        renew("codex-1", T0, T0 + 60.0, 60),
        renew("codex-1", T0, T0 + 80.0, 60),
        renew("codex-1", T0, T0 + 100.0, 60),
        renew("codex-1", T0, T0 + 120.0, 60),
    ];
    let on_b = store(events.iter().cloned().map(own).collect());
    let last_seen = ms(T0) + 5_000;
    let heads = table(
        &h.b.did,
        &[(4, 2), (10, 8)],
        &on_b,
        &[(&h.a.did, 1, last_seen)],
    );
    assert_eq!(
        verdict(&h.b.did, &on_b, &heads, &h.admission(), last_seen + 119_999),
        Some(LeaseVerdict::Unconfirmed {
            session: "codex-1".into(),
            device: h.a.did.clone(),
            last_seen: Some(last_seen),
            fenced: false,
            override_at: Some(last_seen + 120_000),
        })
    );
    assert_eq!(
        verdict(&h.b.did, &on_b, &heads, &h.admission(), last_seen + 120_000),
        Some(effective("codex-1"))
    );
}

#[test]
fn a_no_ttl_override_never_becomes_effective_by_override_but_binds_the_other_device() {
    let h = household();
    let events = [
        claim("codex-1", T0),
        override_of("codex-1", T0, T0 + 10.0, &h.a.did),
    ];
    let on_b = store(events.iter().cloned().map(own).collect());
    let heads = table(
        &h.b.did,
        &[(4, 2)],
        &on_b,
        &[(&h.a.did, 1, ms(T0) - 180_000)],
    );
    assert!(matches!(
        verdict(&h.b.did, &on_b, &heads, &h.admission(), ms(T0) + 86_400_000),
        Some(LeaseVerdict::Unconfirmed {
            override_at: None,
            fenced: false,
            ..
        })
    ));

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
fn partition_longer_than_ttl_never_has_two_effective_holders() {
    const TTL_S: u64 = 90;
    let h = household();
    let admission = h.admission();
    let mut a_events = vec![claim_ttl("claude-1", T0, TTL_S)];
    let a_claim = a_events[0].clone();
    let mut b_events = Vec::new();
    let mut b_claimed_at = None;
    let mut b_claimed_elapsed = None;
    let mut b_override_written = false;

    for elapsed_s in 0..=TTL_S * 3 {
        let real_now = ms(T0) + elapsed_s * 1000;
        let b_now = real_now + MAX_SKEW_S * 1000;
        if elapsed_s > 0 && elapsed_s % (TTL_S / 3) == 0 {
            a_events.push(renew("claude-1", T0, T0 + elapsed_s as f64, TTL_S));
        }

        let a_store = store(a_events.iter().cloned().map(own).collect());
        let a_log: Vec<(u64, u64)> = (1..=a_events.len() as u64)
            .map(|events| (events, events))
            .collect();
        let a_heads = table(&h.a.did, &a_log, &a_store, &[(&h.b.did, 1, ms(T0))]);
        let a_effective = matches!(
            verdict(&h.a.did, &a_store, &a_heads, &admission, real_now),
            Some(LeaseVerdict::Effective { .. })
        );

        if b_claimed_at.is_none() {
            let before = store(vec![from(&h.a.did, a_claim.clone())]);
            let heads = table(
                &h.b.did,
                &[],
                &before,
                &[(&h.a.did, 0, ms(T0) + MAX_SKEW_S * 1000)],
            );
            if !matches!(
                verdict(&h.b.did, &before, &heads, &admission, b_now),
                Some(LeaseVerdict::HeldBy { .. })
            ) {
                let claimed_at = T0 + (elapsed_s + MAX_SKEW_S) as f64;
                b_events.push(claim_ttl("codex-1", claimed_at, TTL_S));
                b_claimed_at = Some(claimed_at);
                b_claimed_elapsed = Some(elapsed_s);
            }
        } else if let (Some(claimed_at), Some(claimed_elapsed)) = (b_claimed_at, b_claimed_elapsed)
        {
            if !b_override_written && elapsed_s >= claimed_elapsed + WAIT_S {
                b_events.push(override_of(
                    "codex-1",
                    claimed_at,
                    T0 + (elapsed_s + MAX_SKEW_S) as f64,
                    &h.a.did,
                ));
                b_override_written = true;
            }
            if elapsed_s > claimed_elapsed && (elapsed_s - claimed_elapsed) % (TTL_S / 3) == 0 {
                b_events.push(renew(
                    "codex-1",
                    claimed_at,
                    T0 + (elapsed_s + MAX_SKEW_S) as f64,
                    TTL_S,
                ));
            }
        }

        let mut on_b = vec![from(&h.a.did, a_claim.clone())];
        on_b.extend(b_events.iter().cloned().map(own));
        let b_store = store(on_b);
        let b_log: Vec<(u64, u64)> = (2..=b_events.len() as u64 + 1)
            .map(|events| (events, events))
            .collect();
        let b_heads = table(
            &h.b.did,
            &b_log,
            &b_store,
            &[(&h.a.did, 0, ms(T0) + MAX_SKEW_S * 1000)],
        );
        let b_effective = matches!(
            verdict(&h.b.did, &b_store, &b_heads, &admission, b_now),
            Some(LeaseVerdict::Effective { .. })
        );
        assert!(
            !(a_effective && b_effective),
            "both effective {elapsed_s}s into the partition"
        );
    }
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
            "channel": {
                "confirm_wait_s": 7,
                "freshness_s": 90,
                "fence_margin_s": 17,
                "max_skew_s": 23
            }
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
    assert_eq!(
        scope.settlement(),
        Settlement {
            confirm_wait_s: 7,
            fence_margin_s: 17,
            max_skew_s: 23,
        }
    );
    // A policy that declares none: nothing is steward-scoped, and the wait is the spec's 10 s.
    let none = StewardScope::from_policy(&serde_json::json!({})).expect("reads");
    assert!(!none.covers("dev-integration"));
    assert_eq!(none.confirm_wait_s(), 10);
    assert_eq!(none.settlement(), SETTLEMENT);
}

/// A berth mooring record (`moor` or `unmoor`) by `session`, written at `at` as the device's
/// `dseq`th record. A mooring names no resource.
fn mooring(kind: &str, session: &str, at: f64, dseq: u64) -> FlowEvent {
    let mut env = vec![
        format!("env:at={at:.3}"),
        format!("env:dseq={dseq}"),
        format!("env:kind={kind}"),
        format!("env:session={session}"),
    ];
    env.sort();
    event_with(MEASURE, kind, env)
}

#[test]
fn other_devices_moorings_are_replayed_in_each_devices_order_from_the_fold() {
    let h = household();
    let counted = |event: FlowEvent, dseq: u64| {
        let mut event = event;
        event.classified_as.push(format!("env:dseq={dseq}"));
        event
    };
    // B moored codex-1 and codex-2, claimed, then unmoored codex-2; A folded B's head in its
    // tree's order, the unmoor first. A's own mooring is A's, not another device's.
    let s = store(vec![
        own(mooring("moor", "claude-1", T0 - 5.0, 1)),
        from(&h.b.did, mooring("unmoor", "codex-2", T0 + 3.0, 4)),
        from(&h.b.did, counted(claim("codex-1", T0 + 2.0), 3)),
        from(&h.b.did, mooring("moor", "codex-2", T0 + 1.0, 2)),
        from(&h.b.did, mooring("moor", "codex-1", T0, 1)),
        // codex-1 moored again later: its latest moor is what a status reads its age from.
        from(&h.b.did, mooring("moor", "codex-1", T0 + 5.0, 5)),
    ]);
    let heads = table(&h.a.did, &[], &s, &[(&h.b.did, 0, ms(T0) + 4_000)]);
    let others = other_devices(&h.a.did, &s, &heads, &h.admission()).expect("A is admitted");
    let b = others.get(&h.b.did).expect("B is listed under its own did");
    assert_eq!(b.moorings.keys().collect::<Vec<_>>(), vec!["codex-1"]);
    assert_eq!(b.moorings["codex-1"], ms(T0 + 5.0));
    assert_eq!((b.seq, b.last_seen), (Some(9), Some(ms(T0) + 4_000)));
    assert_eq!(others.len(), 1, "this device is not among the others");

    // A device admitted but never folded is listed, silent: no head, no moorings.
    let never = other_devices(
        &h.a.did,
        &store(vec![]),
        &HeadTable::default(),
        &h.admission(),
    )
    .expect("A is admitted");
    assert_eq!(never[&h.b.did].last_seen, None);
    assert!(never[&h.b.did].moorings.is_empty());

    // A contested device is not one of A's devices any more: it is not listed at all.
    let mut rows = h.rows.clone();
    rows.push(contest(&h.a, &binding(&h.a, &h.b)));
    let admission = Admission::from_roster(&rows, &h.a.did, &*h.verify);
    assert!(other_devices(&h.a.did, &s, &heads, &admission)
        .expect("A is admitted")
        .is_empty());
}
