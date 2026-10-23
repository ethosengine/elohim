//! The tier ALPN: a device serves its eprfs store to the person's other admitted devices, named
//! by the node id iroh authenticated, and gives everyone else eprfs-serve's uniform refusal.

use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::{Arc, RwLock};
use std::time::Duration;

use bytes::Bytes;
use elohim_epr::proof::{self, AgentKeypair};
use elohim_epr_rea::{ActorRecord, ActorWitness, ParticipantRow};
use eprfs_core::{
    encode_tree, BlobCid, ItemAbsence, ReachReason, TierId, TreeEntry, TreeEntryKind, TreeLimits,
    TreeNode, TreePath, View,
};
use eprfs_peer::{
    bind, tier_call, tier_request, Admission, PeerKey, SharedAdmission, TierAnswer, TierProtocol,
    TierRequest, MAX_REQUEST_FRAME_BYTES, TIER_ALPN,
};
use eprfs_serve::testing::CountingTier;
use iroh::protocol::Router;
use iroh::{Endpoint, NodeAddr};

const HANDLE: &str = "matthew";
const WITHIN: Duration = Duration::from_secs(20);

struct Device {
    did: String,
    key: AgentKeypair,
}

fn device(name: &str, seed: u8) -> Device {
    Device {
        did: format!("did:key:z{name}"),
        key: AgentKeypair::from_secret(&[seed; 32]).expect("seed"),
    }
}

type Verifier = Box<dyn Fn(&str, &[u8], &[u8]) -> bool + Send + Sync>;

fn verifier(devices: &[&Device]) -> Verifier {
    let keys: HashMap<String, [u8; 32]> = devices
        .iter()
        .map(|d| (d.did.clone(), d.key.public_key_bytes()))
        .collect();
    Box::new(move |did, message, signature| {
        keys.get(did)
            .is_some_and(|pk| proof::verify(pk, message, signature))
    })
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn sign(device: &Device, message: &[u8]) -> String {
    hex(&proof::sign(&device.key, message))
}

fn genesis(root: &Device) -> ParticipantRow {
    let founding = ActorRecord::Witness(
        ActorWitness::new(
            "human:matthew",
            "agent:orchestrator@claude-opus-5-5",
            "s1",
            "operator of this stewarded device; present in this session",
            "2026-10-23T00:00:00Z",
        )
        .expect("witness"),
    );
    let mut row = ParticipantRow::Genesis {
        handle: HANDLE.into(),
        chain_root: root.did.clone(),
        record_cid: founding.cid().expect("cid").to_string(),
        signer: root.did.clone(),
        signature: String::new(),
    };
    let message = row.signing_message().expect("message");
    if let ParticipantRow::Genesis { signature, .. } = &mut row {
        *signature = sign(root, &message);
    }
    row
}

fn binding(root: &Device, joiner: &Device) -> ParticipantRow {
    let mut row = ParticipantRow::Binding {
        handle: HANDLE.into(),
        chain_root: root.did.clone(),
        controller: joiner.did.clone(),
        authorized_by: root.did.clone(),
        sig_a: String::new(),
        sig_b: String::new(),
        nonce: "nonce-bind".into(),
    };
    let message = row.signing_message().expect("message");
    if let ParticipantRow::Binding { sig_a, sig_b, .. } = &mut row {
        *sig_a = sign(root, &message);
        *sig_b = sign(joiner, &message);
    }
    row
}

fn endpoint_row(root: &Device, dev: &Device, key: &PeerKey) -> ParticipantRow {
    let mut row = ParticipantRow::PeerEndpoint {
        handle: HANDLE.into(),
        chain_root: root.did.clone(),
        device: dev.did.clone(),
        node_id: key.node_id().to_string(),
        seq: 1,
        nonce: "nonce-1".into(),
        signature: String::new(),
    };
    let message = row.signing_message().expect("message");
    if let ParticipantRow::PeerEndpoint { signature, .. } = &mut row {
        *signature = sign(dev, &message);
    }
    row
}

fn contest(root: &Device, target: &ParticipantRow) -> ParticipantRow {
    let mut row = ParticipantRow::Contest {
        handle: HANDLE.into(),
        target_cid: target.cid().expect("cid").to_string(),
        by: root.did.clone(),
        basis: "that device is lost".into(),
        signature: String::new(),
    };
    let message = row.signing_message().expect("message");
    if let ParticipantRow::Contest { signature, .. } = &mut row {
        *signature = sign(root, &message);
    }
    row
}

fn peer_key() -> (tempfile::TempDir, PeerKey) {
    let dir = tempfile::tempdir().expect("tempdir");
    let key = PeerKey::load_or_generate(dir.path()).expect("key");
    (dir, key)
}

fn tp(path: &str) -> TreePath {
    TreePath::from_slash_bytes(path.as_bytes()).expect("path")
}

fn put_tree(tier: &CountingTier, entries: Vec<TreeEntry>) -> BlobCid {
    let (cid, bytes) = encode_tree(
        &TreeNode::new(entries).expect("node"),
        &TreeLimits::default(),
    )
    .expect("enc");
    tier.insert_at(cid.clone(), Bytes::from(bytes));
    cid
}

fn file(name: &str, cid: &BlobCid) -> TreeEntry {
    TreeEntry {
        name: name.as_bytes().to_vec(),
        kind: TreeEntryKind::File {
            blob: cid.clone().into(),
            executable: false,
        },
    }
}

fn dir(name: &str, cid: &BlobCid) -> TreeEntry {
    TreeEntry {
        name: name.as_bytes().to_vec(),
        kind: TreeEntryKind::Directory {
            tree: cid.clone().into(),
        },
    }
}

/// `channel/events/<leaf>` for each leaf, as a root.
fn channel_root(tier: &CountingTier, leaves: &[(&str, &BlobCid)]) -> BlobCid {
    let events = put_tree(
        tier,
        leaves.iter().map(|(name, cid)| file(name, cid)).collect(),
    );
    let channel = put_tree(tier, vec![dir("events", &events)]);
    put_tree(tier, vec![dir("channel", &channel)])
}

/// Device A (the chain root) serves; device B is bound and has an endpoint row.
struct Household {
    a: Device,
    b: Device,
    rows: Vec<ParticipantRow>,
    verify: Verifier,
    _dirs: Vec<tempfile::TempDir>,
    key_a: PeerKey,
    key_b: PeerKey,
}

fn household() -> Household {
    let a = device("DeviceA", 1);
    let b = device("DeviceB", 2);
    let (da, key_a) = peer_key();
    let (db, key_b) = peer_key();
    let rows = vec![
        genesis(&a),
        binding(&a, &b),
        endpoint_row(&a, &a, &key_a),
        endpoint_row(&a, &b, &key_b),
    ];
    let verify = verifier(&[&a, &b]);
    Household {
        a,
        b,
        rows,
        verify,
        _dirs: vec![da, db],
        key_a,
        key_b,
    }
}

/// The store device A serves: an old root with one event and a new root with two.
struct Store {
    tier: Arc<CountingTier>,
    old: BlobCid,
    new: BlobCid,
    first: BlobCid,
    second: BlobCid,
}

fn store(serving: &TierId) -> Store {
    let tier = Arc::new(CountingTier::new(serving.clone()));
    let first = tier.insert(b"event one");
    let second = tier.insert(b"event two");
    let old = channel_root(&tier, &[("e1", &first)]);
    let new = channel_root(&tier, &[("e1", &first), ("e2", &second)]);
    Store {
        tier,
        old,
        new,
        first,
        second,
    }
}

struct Serving {
    router: Router,
    addr: NodeAddr,
    protocol: TierProtocol,
}

async fn serve(key: &PeerKey, store: &Store, admission: SharedAdmission, id: TierId) -> Serving {
    let endpoint = bind(key, None, vec![TIER_ALPN.to_vec()])
        .await
        .expect("bind server");
    let protocol = TierProtocol::new(id, store.tier.clone(), admission, TreeLimits::default())
        .expect("protocol");
    let port = endpoint
        .bound_sockets()
        .into_iter()
        .find(SocketAddr::is_ipv4)
        .expect("an ipv4 socket")
        .port();
    let addr = NodeAddr::new(endpoint.node_id())
        .with_direct_addresses([SocketAddr::from((Ipv4Addr::LOCALHOST, port))]);
    let router = Router::builder(endpoint)
        .accept(TIER_ALPN, protocol.clone())
        .spawn();
    Serving {
        router,
        addr,
        protocol,
    }
}

async fn client(key: &PeerKey) -> Endpoint {
    bind(key, None, vec![]).await.expect("bind client")
}

async fn call(client: &Endpoint, addr: &NodeAddr, req: TierRequest) -> TierAnswer {
    tokio::time::timeout(WITHIN, tier_call(client, addr.clone(), req))
        .await
        .expect("answers in time")
        .expect("answers")
}

fn tier_id(device: &Device) -> TierId {
    TierId::agent(device.did.clone())
}

#[tokio::test(flavor = "multi_thread")]
async fn admitted_device_reads_and_diffs_a_root() {
    let h = household();
    let served = tier_id(&h.a);
    let s = store(&served);
    let admission = Arc::new(RwLock::new(Admission::from_roster(
        &h.rows, &h.a.did, &h.verify,
    )));
    let server = serve(&h.key_a, &s, admission, served.clone()).await;
    let b = client(&h.key_b).await;

    let read = call(
        &b,
        &server.addr,
        TierRequest::Read {
            root: s.new.clone(),
            path: tp("channel/events/e2"),
        },
    )
    .await;
    let TierAnswer::Read(leaf) = read else {
        panic!("an admitted device reads the leaf, got {read:?}");
    };
    assert_eq!(leaf.cid, s.second);
    assert_eq!(leaf.bytes.as_ref(), b"event two");
    assert_eq!(leaf.path, tp("channel/events/e2"));

    let list = call(
        &b,
        &server.addr,
        TierRequest::List {
            root: s.new.clone(),
            path: tp("channel/events"),
        },
    )
    .await;
    let TierAnswer::List(listing) = list else {
        panic!("an admitted device lists the directory, got {list:?}");
    };
    let names: Vec<&[u8]> = listing.entries.iter().map(|e| e.name.as_slice()).collect();
    assert_eq!(names, vec![b"e1".as_slice(), b"e2".as_slice()]);

    let diff = call(
        &b,
        &server.addr,
        TierRequest::Diff {
            old: s.old.clone(),
            new: s.new.clone(),
            view: View::whole(),
        },
    )
    .await;
    let TierAnswer::Diff(changes) = diff else {
        panic!("an admitted device diffs two roots, got {diff:?}");
    };
    assert_eq!(changes.len(), 1, "exactly the added event: {changes:?}");
    assert_eq!(changes[0].path, tp("channel/events/e2"));
    assert_eq!(changes[0].old, None);
    assert!(matches!(
        &changes[0].new,
        Some(TreeEntryKind::File { blob, .. }) if BlobCid::from(blob.clone()) == s.second
    ));
    assert_ne!(s.first, s.second);

    b.close().await;
    server.router.shutdown().await.expect("shutdown");
}

#[tokio::test(flavor = "multi_thread")]
async fn stranger_gets_uniform_refusal_for_held_and_unknown_roots() {
    let h = household();
    let served = tier_id(&h.a);
    let s = store(&served);
    let admission = Arc::new(RwLock::new(Admission::from_roster(
        &h.rows, &h.a.did, &h.verify,
    )));
    let server = serve(&h.key_a, &s, admission, served.clone()).await;
    let (_ds, stranger_key) = peer_key();
    let stranger = client(&stranger_key).await;
    let unknown = BlobCid::compute(b"unknown root");
    let path = tp("channel/events/e1");

    let mut expected = None;
    for root in [&s.new, &unknown] {
        let requests = [
            TierRequest::Read {
                root: root.clone(),
                path: path.clone(),
            },
            TierRequest::List {
                root: root.clone(),
                path: path.clone(),
            },
            TierRequest::Diff {
                old: root.clone(),
                new: s.new.clone(),
                view: View::whole(),
            },
        ];
        for request in requests {
            let answer = call(&stranger, &server.addr, request).await;
            let TierAnswer::Unanswered { cid, why, .. } = answer else {
                panic!("a stranger must receive the uniform refusal, got {answer:?}");
            };
            assert_eq!(&cid, root, "the refusal names only what was asked");
            assert!(matches!(
                &why.why,
                ItemAbsence::Refused { by, refusal }
                    if by == &served
                        && refusal.reason == ReachReason::OutsideStanding
                        && refusal.needs_standing.is_none()
            ));
            if let Some(previous) = &expected {
                assert_eq!(previous, &why, "held and unknown roots refuse alike");
            }
            expected = Some(why);
        }
    }
    assert_eq!(
        server.protocol.counters().unadmitted,
        6,
        "every refused request is counted where the stranger cannot see it"
    );
    let calls = s.tier.calls();
    assert!(
        calls.batches.is_empty() && calls.fetch_blob == 0,
        "a stranger's request never reaches the store: {calls:?}"
    );

    stranger.close().await;
    server.router.shutdown().await.expect("shutdown");
}

#[tokio::test(flavor = "multi_thread")]
async fn contested_device_refused_after_admission_reload() {
    let h = household();
    let served = tier_id(&h.a);
    let s = store(&served);
    let admission = Arc::new(RwLock::new(Admission::from_roster(
        &h.rows, &h.a.did, &h.verify,
    )));
    let server = serve(&h.key_a, &s, admission.clone(), served.clone()).await;
    let b = client(&h.key_b).await;
    let connection = tokio::time::timeout(WITHIN, b.connect(server.addr.clone(), TIER_ALPN))
        .await
        .expect("connects in time")
        .expect("connects");
    let read = TierRequest::Read {
        root: s.new.clone(),
        path: tp("channel/events/e1"),
    };

    let before = tier_request(&connection, &read).await.expect("answers");
    assert!(
        matches!(&before, TierAnswer::Read(leaf) if leaf.cid == s.first),
        "admitted before the contest, got {before:?}"
    );

    let mut rows = h.rows.clone();
    rows.push(contest(&h.a, &binding(&h.a, &h.b)));
    admission
        .write()
        .expect("admission lock")
        .reload(&rows, &h.verify);

    let after = tier_request(&connection, &read).await.expect("answers");
    assert!(
        matches!(
            &after,
            TierAnswer::Unanswered { why, .. }
                if matches!(&why.why, ItemAbsence::Refused { refusal, .. }
                    if refusal.reason == ReachReason::OutsideStanding)
        ),
        "the next request on the same open connection is refused, got {after:?}"
    );

    drop(connection);
    b.close().await;
    server.router.shutdown().await.expect("shutdown");
}

#[tokio::test(flavor = "multi_thread")]
async fn an_oversize_request_frame_is_refused_before_it_is_read() {
    let h = household();
    let served = tier_id(&h.a);
    let s = store(&served);
    let admission = Arc::new(RwLock::new(Admission::from_roster(
        &h.rows, &h.a.did, &h.verify,
    )));
    let server = serve(&h.key_a, &s, admission, served).await;
    let b = client(&h.key_b).await;
    let connection = tokio::time::timeout(WITHIN, b.connect(server.addr.clone(), TIER_ALPN))
        .await
        .expect("connects in time")
        .expect("connects");

    // Only a length header past the cap: no body follows, and none is waited for.
    let (mut send, mut recv) = connection.open_bi().await.expect("stream");
    send.write_all(&(MAX_REQUEST_FRAME_BYTES + 1).to_be_bytes())
        .await
        .expect("header");
    let read = tokio::time::timeout(WITHIN, recv.read_to_end(1024))
        .await
        .expect("the server ends the stream in time");
    assert!(read.is_err(), "the stream is reset, not answered: {read:?}");
    let counters = server.protocol.counters();
    assert_eq!(
        (counters.refused_frames, counters.requests),
        (1, 0),
        "{counters:?}"
    );

    let after = tier_request(
        &connection,
        &TierRequest::Read {
            root: s.new.clone(),
            path: tp("channel/events/e1"),
        },
    )
    .await
    .expect("the connection still answers");
    assert!(matches!(after, TierAnswer::Read(_)), "{after:?}");

    drop(connection);
    b.close().await;
    server.router.shutdown().await.expect("shutdown");
}
