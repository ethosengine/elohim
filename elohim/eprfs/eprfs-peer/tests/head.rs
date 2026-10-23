//! Signed channel heads and how a device judges another device's head: the signature against the
//! device's did:key, admission, and per-device `seq` order (spec §3.2–§3.3; review focus item 4).

mod support;

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use eprfs_core::BlobCid;
use eprfs_peer::{
    topic_for, Channel, ChannelHead, HeadTable, HeadVerdict, PeerError, ANNOUNCE_DEBOUNCE,
    REANNOUNCE_EVERY,
};
use support::{device, household, Household};

fn root(label: &str) -> BlobCid {
    BlobCid::compute(label.as_bytes())
}

fn head_of(h: &Household, from_b: bool, seq: u64, root_label: &str) -> ChannelHead {
    let dev = if from_b { &h.b } else { &h.a };
    ChannelHead::sign(
        dev.did.clone(),
        root(root_label),
        seq,
        1_000 + seq,
        BTreeMap::new(),
        &dev.signer(),
    )
    .expect("signs")
}

#[test]
fn head_signature_verifies_against_device_did() {
    let h = household();
    let mut folded = BTreeMap::new();
    folded.insert(h.b.did.clone(), 4);
    let head = ChannelHead::sign(
        h.a.did.clone(),
        root("r1"),
        7,
        1_792_769_518_961,
        folded,
        &h.a.signer(),
    )
    .expect("signs");
    head.verify(&h.a.did, &*h.verify).expect("verifies as A");
    assert!(matches!(
        head.verify(&h.b.did, &*h.verify),
        Err(PeerError::HeadSignature { .. })
    ));

    // The wire form round-trips and still verifies.
    let wire = head.to_wire().expect("encodes");
    let back = ChannelHead::from_wire(&wire).expect("decodes");
    assert_eq!(back, head);
    back.verify(&h.a.did, &*h.verify)
        .expect("verifies after the wire");

    // Every signed field is covered, the acknowledgement vector included.
    let mut tampered = head.clone();
    tampered.folded.insert(h.b.did.clone(), 5);
    assert!(tampered.verify(&h.a.did, &*h.verify).is_err(), "folded");
    let mut tampered = head.clone();
    tampered.seq = 8;
    assert!(tampered.verify(&h.a.did, &*h.verify).is_err(), "seq");
    let mut tampered = head.clone();
    tampered.root = root("r2");
    assert!(tampered.verify(&h.a.did, &*h.verify).is_err(), "root");
    let mut tampered = head;
    tampered.at += 1;
    assert!(tampered.verify(&h.a.did, &*h.verify).is_err(), "at");

    // Junk on the wire is refused, never half-read.
    assert!(ChannelHead::from_wire(b"not a head").is_err());
}

#[test]
fn forged_head_dropped_and_counted() {
    let h = household();
    let admission = h.admission();
    let mut channel = Channel::new(h.a.did.clone(), HeadTable::default());

    // A head that names B but was signed by a key that is not B's.
    let stranger = device("Stranger", 9);
    let forged = ChannelHead::sign(
        h.b.did.clone(),
        root("r1"),
        3,
        1,
        BTreeMap::new(),
        &stranger.signer(),
    )
    .expect("signs");
    assert_eq!(
        channel.judge(&forged, &admission, &*h.verify),
        HeadVerdict::Forged
    );
    // A genuine head whose seq was raised in transit.
    let mut raised = head_of(&h, true, 3, "r1");
    raised.seq = 99;
    assert_eq!(
        channel.judge(&raised, &admission, &*h.verify),
        HeadVerdict::Forged
    );
    // A validly signed head from a device the roster does not admit.
    let unadmitted = ChannelHead::sign(
        stranger.did.clone(),
        root("r1"),
        1,
        1,
        BTreeMap::new(),
        &stranger.signer(),
    )
    .expect("signs");
    let verify_all = support::verifier(&[&h.a, &h.b, &stranger]);
    assert_eq!(
        channel.judge(&unadmitted, &admission, &*verify_all),
        HeadVerdict::Unadmitted
    );
    // This device's own head, heard back over gossip, is neither folded nor counted.
    assert_eq!(
        channel.judge(&head_of(&h, false, 1, "r1"), &admission, &*h.verify),
        HeadVerdict::Own
    );

    let table = channel.table();
    assert_eq!(table.dropped.forged, 2);
    assert_eq!(table.dropped.unadmitted, 1);
    assert!(table.devices.is_empty(), "nothing was folded");
    assert_eq!(channel.last_folded(&h.b.did), None);

    // A genuine head is still folded after the forgeries.
    let genuine = head_of(&h, true, 3, "r1");
    assert_eq!(
        channel.judge(&genuine, &admission, &*h.verify),
        HeadVerdict::Fold
    );
}

#[test]
fn lower_seq_head_refused_as_rollback() {
    let h = household();
    let admission = h.admission();
    let mut channel = Channel::new(h.a.did.clone(), HeadTable::default());
    let now = Instant::now();

    let five = head_of(&h, true, 5, "r5");
    assert_eq!(
        channel.judge(&five, &admission, &*h.verify),
        HeadVerdict::Fold
    );
    channel.folded(&five, 2, 10_000, now);
    assert_eq!(channel.last_folded(&h.b.did), Some((5, &root("r5"))));

    let three = head_of(&h, true, 3, "r3");
    assert_eq!(
        channel.judge(&three, &admission, &*h.verify),
        HeadVerdict::Rollback
    );
    assert_eq!(
        channel.judge(&five, &admission, &*h.verify),
        HeadVerdict::Replayed
    );
    let table = channel.table();
    assert_eq!((table.dropped.rollback, table.dropped.replayed), (1, 1));
    let b = &table.devices[&h.b.did];
    assert_eq!((b.seq, &b.root, b.events_folded), (5, &root("r5"), 2));
    assert_eq!(b.folded_at, 10_000);

    // The table persists with its counters and reloads to the same state.
    let dir = tempfile::tempdir().expect("tempdir");
    table.save(dir.path()).expect("saves");
    assert_eq!(&HeadTable::load(dir.path()).expect("loads"), table);
    // An absent table is an empty one.
    let empty = tempfile::tempdir().expect("tempdir");
    assert_eq!(
        HeadTable::load(empty.path()).expect("loads"),
        HeadTable::default()
    );
}

#[test]
fn folding_a_newer_head_reannounces_with_updated_ack() {
    let h = household();
    let mut channel = Channel::new(h.a.did.clone(), HeadTable::default());
    let signer = h.a.signer();
    let t0 = Instant::now();

    // Nothing to announce until this device has a channel root.
    assert!(!channel.due(t0));
    assert!(channel
        .announce(t0, 1, &signer)
        .expect("announce")
        .is_none());

    channel.changed(root("a1"), t0);
    assert!(!channel.due(t0), "a change waits out the debounce");
    let t1 = t0 + ANNOUNCE_DEBOUNCE;
    assert!(channel.due(t1));
    let first = channel
        .announce(t1, 2, &signer)
        .expect("announce")
        .expect("a head");
    assert_eq!((first.seq, &first.root), (1, &root("a1")));
    assert!(first.folded.is_empty());
    assert!(!channel.due(t1), "announced");

    // B's head with seq s carries new events: folding it schedules a re-announce.
    let s = 7;
    let from_b = head_of(&h, true, s, "b1");
    let t2 = t1 + Duration::from_secs(3);
    channel.folded(&from_b, 1, 3, t2);
    assert!(!channel.due(t2));
    let t3 = t2 + ANNOUNCE_DEBOUNCE;
    assert!(
        channel.due(t3),
        "the acknowledgement goes out within the debounce"
    );
    let ack = channel
        .announce(t3, 4, &signer)
        .expect("announce")
        .expect("a head");
    assert_eq!(ack.seq, 2, "seq strictly increases");
    assert_eq!(ack.folded.get(&h.b.did), Some(&s));
    ack.verify(&h.a.did, &*h.verify).expect("the ack is signed");

    // A newer head with the same root (B acknowledging us) is recorded, but acknowledging an
    // acknowledgement would ping-pong forever: no re-announce is scheduled for it.
    let echo = head_of(&h, true, s + 1, "b1");
    channel.folded(&echo, 0, 5, t3);
    assert!(!channel.due(t3 + ANNOUNCE_DEBOUNCE));
    // The periodic re-announce still carries it.
    let t4 = t3 + REANNOUNCE_EVERY;
    assert!(channel.due(t4));
    let periodic = channel
        .announce(t4, 6, &signer)
        .expect("announce")
        .expect("a head");
    assert_eq!(periodic.seq, 3);
    assert_eq!(periodic.folded.get(&h.b.did), Some(&(s + 1)));

    // A neighbour coming up hears the current head within the debounce.
    channel.neighbor_up(t4);
    assert!(channel.due(t4 + ANNOUNCE_DEBOUNCE));

    // The own head persists, so a restarted daemon keeps seq strictly increasing.
    let restarted = Channel::new(h.a.did.clone(), channel.table().clone());
    assert_eq!(restarted.table().own.as_ref().map(|o| o.seq), Some(3));
}

#[test]
fn the_topic_is_blake3_of_the_named_chain_root() {
    let chain_root = "did:key:zDeviceA";
    let expected = blake3::hash(format!("elohim/steward-channel/1/{chain_root}").as_bytes());
    assert_eq!(topic_for(chain_root).as_bytes(), expected.as_bytes());
    assert_ne!(topic_for(chain_root), topic_for("did:key:zOther"));
}

#[test]
fn a_wiped_peer_dir_recovers_its_seq_from_the_others_acks() {
    let h = household();
    let admission = h.admission();
    let signer = h.a.signer();
    // A's head table is gone; B's last head acknowledges A up to seq 40.
    let mut channel = Channel::new(h.a.did.clone(), HeadTable::default());
    let mut acked = BTreeMap::new();
    acked.insert(h.a.did.clone(), 40);
    let from_b =
        ChannelHead::sign(h.b.did.clone(), root("b1"), 9, 1, acked, &h.b.signer()).expect("signs");
    assert_eq!(
        channel.judge(&from_b, &admission, &*h.verify),
        HeadVerdict::Fold
    );
    let t0 = Instant::now();
    channel.changed(root("a1"), t0);
    let head = channel
        .announce(t0 + ANNOUNCE_DEBOUNCE, 2, &signer)
        .expect("announce")
        .expect("a head");
    assert_eq!(
        head.seq, 41,
        "the next head is above every seq the others folded"
    );

    // A forged head claiming a high acknowledgement moves nothing.
    let mut forged_ack = BTreeMap::new();
    forged_ack.insert(h.a.did.clone(), 1_000);
    let mut forged = ChannelHead::sign(
        h.b.did.clone(),
        root("b1"),
        10,
        1,
        BTreeMap::new(),
        &h.b.signer(),
    )
    .expect("signs");
    forged.folded = forged_ack;
    assert_eq!(
        channel.judge(&forged, &admission, &*h.verify),
        HeadVerdict::Forged
    );
    let next = channel
        .announce(t0 + REANNOUNCE_EVERY * 2, 3, &signer)
        .expect("announce")
        .expect("a head");
    assert_eq!(next.seq, 42);
}
