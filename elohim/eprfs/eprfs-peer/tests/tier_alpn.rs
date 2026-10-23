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
    encode_tree, BlobCid, DirListing, ItemAbsence, LeafKind, ReachReason, ReadLeaf, TierId,
    TreeEntry, TreeEntryKind, TreeLimits, TreeNode, TreePath, View,
};
use eprfs_peer::{
    bind, tier_call, tier_request, Admission, PeerError, PeerKey, SharedAdmission, TierAnswer,
    TierLimits, TierProtocol, TierRequest, MAX_ANSWER_FRAME_BYTES, MAX_REQUEST_FRAME_BYTES,
    TIER_ALPN,
};
use eprfs_serve::testing::CountingTier;
use iroh::endpoint::{Connection, RecvStream, SendStream};
use iroh::protocol::{AcceptError, ProtocolHandler, Router};
use iroh::{Endpoint, NodeAddr};
use tokio::sync::Semaphore;
use tokio::time::Instant;

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
    serve_with(key, store, admission, id, TierLimits::default()).await
}

async fn serve_with(
    key: &PeerKey,
    store: &Store,
    admission: SharedAdmission,
    id: TierId,
    limits: TierLimits,
) -> Serving {
    let protocol = TierProtocol::new(id, store.tier.clone(), admission, limits).expect("protocol");
    mount(key, protocol.clone()).await.with(protocol)
}

/// A router on `key` answering `TIER_ALPN` with `handler`, and the loopback address to dial.
async fn mount<P: ProtocolHandler>(key: &PeerKey, handler: P) -> Mounted {
    let endpoint = bind(key, None, vec![TIER_ALPN.to_vec()])
        .await
        .expect("bind server");
    let port = endpoint
        .bound_sockets()
        .into_iter()
        .find(SocketAddr::is_ipv4)
        .expect("an ipv4 socket")
        .port();
    let addr = NodeAddr::new(endpoint.node_id())
        .with_direct_addresses([SocketAddr::from((Ipv4Addr::LOCALHOST, port))]);
    let router = Router::builder(endpoint).accept(TIER_ALPN, handler).spawn();
    Mounted { router, addr }
}

struct Mounted {
    router: Router,
    addr: NodeAddr,
}

impl Mounted {
    fn with(self, protocol: TierProtocol) -> Serving {
        Serving {
            router: self.router,
            addr: self.addr,
            protocol,
        }
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

fn admission(h: &Household) -> SharedAdmission {
    Arc::new(RwLock::new(Admission::from_roster(
        &h.rows, &h.a.did, &h.verify,
    )))
}

async fn connect(client: &Endpoint, addr: &NodeAddr) -> Connection {
    tokio::time::timeout(WITHIN, client.connect(addr.clone(), TIER_ALPN))
        .await
        .expect("connects in time")
        .expect("connects")
}

/// Open a stream and send only a request header claiming `len` body bytes, never the body. The
/// caller keeps the send half open, so the server is left waiting for a body that never comes.
async fn header_only(connection: &Connection, len: u32) -> (SendStream, RecvStream) {
    let (mut send, recv) = connection.open_bi().await.expect("stream");
    send.write_all(&len.to_be_bytes()).await.expect("header");
    (send, recv)
}

/// Wait until `permits` of the in-flight bound are free.
async fn permits_free(bound: &Semaphore, permits: usize) {
    tokio::time::timeout(WITHIN, async {
        while bound.available_permits() != permits {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("the in-flight permits settle in time");
}

fn read_e1(s: &Store) -> TierRequest {
    TierRequest::Read {
        root: s.new.clone(),
        path: tp("channel/events/e1"),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_header_only_stream_is_dropped_at_the_deadline_and_its_permit_returned() {
    let h = household();
    let served = tier_id(&h.a);
    let s = store(&served);
    let bound = Arc::new(Semaphore::new(1));
    let limits = TierLimits {
        frame_deadline: Duration::from_millis(300),
        admitted_in_flight: Arc::clone(&bound),
        ..TierLimits::default()
    };
    let server = serve_with(&h.key_a, &s, admission(&h), served, limits).await;
    let b = client(&h.key_b).await;
    let connection = connect(&b, &server.addr).await;

    let started = Instant::now();
    let (_send, mut stalled) = header_only(&connection, 100).await;
    permits_free(&bound, 0).await;
    let shed = tier_request(&connection, &read_e1(&s)).await;
    assert!(
        matches!(shed, Err(PeerError::TierStream(_))),
        "with the one permit held, a further stream is shed, not queued: {shed:?}"
    );

    let ended = tokio::time::timeout(WITHIN, stalled.read_to_end(1024))
        .await
        .expect("the stalled stream ends in time");
    assert!(ended.is_err(), "the stalled stream is reset: {ended:?}");
    assert!(
        started.elapsed() >= Duration::from_millis(250),
        "not before the deadline: {:?}",
        started.elapsed()
    );
    permits_free(&bound, 1).await;

    let after = tier_request(&connection, &read_e1(&s))
        .await
        .expect("answers");
    assert!(matches!(after, TierAnswer::Read(_)), "{after:?}");
    let counters = server.protocol.counters();
    assert_eq!(
        (
            counters.timed_out_frames,
            counters.shed_streams,
            counters.requests
        ),
        (1, 1, 1),
        "{counters:?}"
    );

    drop(connection);
    b.close().await;
    server.router.shutdown().await.expect("shutdown");
}

#[tokio::test(flavor = "multi_thread")]
async fn strangers_share_their_own_bound_and_never_shed_an_admitted_device() {
    let h = household();
    let served = tier_id(&h.a);
    let s = store(&served);
    let strangers = Arc::new(Semaphore::new(1));
    let limits = TierLimits {
        frame_deadline: Duration::from_secs(60),
        unadmitted_in_flight: Arc::clone(&strangers),
        ..TierLimits::default()
    };
    let server = serve_with(&h.key_a, &s, admission(&h), served, limits).await;

    // A stranger stalls one stream and holds the whole unadmitted bound.
    let (_d1, key_1) = peer_key();
    let stranger_1 = client(&key_1).await;
    let conn_1 = connect(&stranger_1, &server.addr).await;
    let _stalled = header_only(&conn_1, 100).await;
    permits_free(&strangers, 0).await;

    let b = client(&h.key_b).await;
    let conn_b = connect(&b, &server.addr).await;
    let admitted = tier_request(&conn_b, &read_e1(&s)).await.expect("answers");
    assert!(
        matches!(admitted, TierAnswer::Read(_)),
        "the person's own device is answered while strangers hold their bound: {admitted:?}"
    );

    // A second stranger, on a connection of its own, is shed.
    let (_d2, key_2) = peer_key();
    let stranger_2 = client(&key_2).await;
    let conn_2 = connect(&stranger_2, &server.addr).await;
    let shed = tier_request(&conn_2, &read_e1(&s)).await;
    assert!(
        matches!(shed, Err(PeerError::TierStream(_))),
        "the unadmitted bound holds across connections: {shed:?}"
    );
    assert_eq!(server.protocol.counters().shed_streams, 1);

    // A stranger that goes away returns its permit; the next stranger gets the uniform refusal.
    conn_1.close(0u32.into(), b"");
    permits_free(&strangers, 1).await;
    let refused = tier_request(&conn_2, &read_e1(&s)).await.expect("answers");
    assert!(
        matches!(&refused, TierAnswer::Unanswered { why, .. }
            if matches!(&why.why, ItemAbsence::Refused { refusal, .. }
                if refusal.reason == ReachReason::OutsideStanding
                    && refusal.needs_standing.is_none())),
        "{refused:?}"
    );

    drop((conn_2, conn_b));
    for endpoint in [stranger_1, stranger_2, b] {
        endpoint.close().await;
    }
    server.router.shutdown().await.expect("shutdown");
}

#[tokio::test(flavor = "multi_thread")]
async fn an_answer_the_peer_never_reads_is_reset_at_the_deadline() {
    let h = household();
    let served = tier_id(&h.a);
    let s = store(&served);
    let big = s.tier.insert(&vec![7u8; 12 * 1024 * 1024]);
    let root = channel_root(&s.tier, &[("big", &big)]);
    let bound = Arc::new(Semaphore::new(1));
    let limits = TierLimits {
        frame_deadline: Duration::from_millis(300),
        admitted_in_flight: Arc::clone(&bound),
        ..TierLimits::default()
    };
    let server = serve_with(&h.key_a, &s, admission(&h), served, limits).await;
    let b = client(&h.key_b).await;
    let connection = connect(&b, &server.addr).await;

    // A whole request, then never a byte read of the 12 MiB answer.
    let (mut send, _unread) = connection.open_bi().await.expect("stream");
    let request = TierRequest::Read {
        root,
        path: tp("channel/events/big"),
    };
    send.write_all(&frame_of(&request)).await.expect("request");
    send.finish().expect("finish");

    tokio::time::timeout(WITHIN, async {
        while server.protocol.counters().timed_out_frames == 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("the stalled answer is reset in time");
    permits_free(&bound, 1).await;

    drop(connection);
    b.close().await;
    server.router.shutdown().await.expect("shutdown");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_connection_answers_at_most_its_stream_bound_at_once() {
    let h = household();
    let served = tier_id(&h.a);
    let s = store(&served);
    let limits = TierLimits {
        frame_deadline: Duration::from_millis(500),
        max_streams_per_connection: 1,
        ..TierLimits::default()
    };
    let server = serve_with(&h.key_a, &s, admission(&h), served, limits).await;
    let b = client(&h.key_b).await;
    let connection = connect(&b, &server.addr).await;

    let started = Instant::now();
    let _stalled = header_only(&connection, 100).await;
    let queued = tokio::time::timeout(WITHIN, tier_request(&connection, &read_e1(&s)))
        .await
        .expect("answers in time")
        .expect("answers");
    assert!(matches!(queued, TierAnswer::Read(_)), "{queued:?}");
    assert!(
        started.elapsed() >= Duration::from_millis(400),
        "the second stream waits for the first to end: {:?}",
        started.elapsed()
    );
    let counters = server.protocol.counters();
    assert_eq!(
        (counters.timed_out_frames, counters.shed_streams),
        (1, 0),
        "queued per connection, not shed: {counters:?}"
    );

    drop(connection);
    b.close().await;
    server.router.shutdown().await.expect("shutdown");
}

#[tokio::test(flavor = "multi_thread")]
async fn an_answer_past_the_frame_cap_is_sent_as_failed() {
    let h = household();
    let served = tier_id(&h.a);
    let s = store(&served);
    let limits = TierLimits {
        max_answer_frame_bytes: 16,
        ..TierLimits::default()
    };
    let server = serve_with(&h.key_a, &s, admission(&h), served, limits).await;
    let b = client(&h.key_b).await;

    let answer = call(&b, &server.addr, read_e1(&s)).await;
    assert!(
        matches!(&answer, TierAnswer::Failed { reason } if reason.contains("cap")),
        "never truncated: {answer:?}"
    );
    assert_eq!(server.protocol.counters().oversized_answers, 1);

    b.close().await;
    server.router.shutdown().await.expect("shutdown");
}

/// A tier that answers every stream with the same raw bytes, whatever was asked.
#[derive(Debug, Clone)]
struct Rogue {
    frame: Vec<u8>,
}

impl ProtocolHandler for Rogue {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        while let Ok((mut send, mut recv)) = connection.accept_bi().await {
            let _ = recv.read_to_end(1024 * 1024).await;
            let _ = send.write_all(&self.frame).await;
            let _ = send.finish();
        }
        Ok(())
    }
}

fn rogue_frame(answer: &TierAnswer) -> Vec<u8> {
    frame_of(answer)
}

/// A length-prefixed DAG-CBOR frame, as the tier writes one.
fn frame_of<T: serde::Serialize>(value: &T) -> Vec<u8> {
    let body = serde_ipld_dagcbor::to_vec(value).expect("encode");
    let mut frame = u32::try_from(body.len())
        .expect("small")
        .to_be_bytes()
        .to_vec();
    frame.extend_from_slice(&body);
    frame
}

/// Ask a rogue tier answering with `frame` one question.
async fn ask_rogue(frame: Vec<u8>, req: TierRequest) -> Result<TierAnswer, PeerError> {
    let (_ds, server_key) = peer_key();
    let (_dc, client_key) = peer_key();
    let rogue = mount(&server_key, Rogue { frame }).await;
    let client = client(&client_key).await;
    let answer = tokio::time::timeout(WITHIN, tier_call(&client, rogue.addr.clone(), req))
        .await
        .expect("answers in time");
    client.close().await;
    rogue.router.shutdown().await.expect("shutdown");
    answer
}

fn read(path: &str) -> TierRequest {
    TierRequest::Read {
        root: BlobCid::compute(b"some root"),
        path: tp(path),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn the_client_rejects_a_leaf_whose_bytes_fail_its_address() {
    let leaf = ReadLeaf::new(
        tp("channel/events/e1"),
        BlobCid::compute_raw(b"event one"),
        LeafKind::File { executable: false },
        Bytes::from_static(b"not event one"),
    );
    let answer = ask_rogue(
        rogue_frame(&TierAnswer::Read(leaf)),
        read("channel/events/e1"),
    )
    .await;
    assert!(
        matches!(answer, Err(PeerError::TierLeafIntegrity { .. })),
        "{answer:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn the_client_rejects_a_leaf_for_another_path() {
    let leaf = ReadLeaf::new(
        tp("channel/events/e2"),
        BlobCid::compute_raw(b"event two"),
        LeafKind::File { executable: false },
        Bytes::from_static(b"event two"),
    );
    let answer = ask_rogue(
        rogue_frame(&TierAnswer::Read(leaf)),
        read("channel/events/e1"),
    )
    .await;
    assert!(
        matches!(answer, Err(PeerError::TierLeafIntegrity { .. })),
        "a self-consistent leaf at the wrong path is refused: {answer:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn the_client_rejects_a_listing_that_is_not_its_cid() {
    let listing = DirListing::new(
        tp("channel/events"),
        BlobCid::compute(b"another directory"),
        vec![file("e1", &BlobCid::compute_raw(b"event one"))],
    );
    let req = TierRequest::List {
        root: BlobCid::compute(b"some root"),
        path: tp("channel/events"),
    };
    let answer = ask_rogue(rogue_frame(&TierAnswer::List(listing)), req).await;
    assert!(
        matches!(answer, Err(PeerError::TierLeafIntegrity { .. })),
        "{answer:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn the_client_refuses_an_answer_frame_past_its_cap() {
    let header_only = (MAX_ANSWER_FRAME_BYTES + 1).to_be_bytes().to_vec();
    let answer = ask_rogue(header_only, read("channel/events/e1")).await;
    assert!(
        matches!(answer, Err(PeerError::TierFrame { .. })),
        "{answer:?}"
    );
}
