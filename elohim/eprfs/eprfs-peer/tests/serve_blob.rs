//! The `serve-blob` mount (artifact-store design §1): a device serves the derivation artifacts it
//! holds custody of to the person's other admitted devices, per method name, on the tier ALPN;
//! an admitted device asking outside its grant gets a typed refusal, and everyone else the tier's
//! uniform refusal, whatever the artifact is. Every served session (and every refusal of an
//! admitted device) is recorded as one `serve-blob` Use event that never crosses.

mod support;

use std::path::Path;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use bytes::Bytes;
use elohim_epr_rea::{
    atom_cid, AgentRef, FlowEvent, FlowStore, Magnitude, MemoryFlowStore, ParticipantRow, ReaVerb,
    SidecarFlowStore, SidecarRoster,
};
use eprfs_core::{
    BlobCid, DerivationBasis, DerivationKey, DerivationRecord, EprfsStorage, ItemAbsence, KeyInput,
    MethodDescriptor, ReachReason, ReachRefusal, TreePath, Witness,
};
use eprfs_peer::{
    bind, channel_tier_id, open_payload_store, put_local_blob, tier_call, tier_request,
    ArtifactServeList, BlobMount, ChannelStore, CrossingAllowList, DeviceScope, DrawOutcome,
    LocalBlobContext, PeerConfig, PeerDaemon, PeerError, PeerKey, Rules, ServedDraw,
    SharedAdmission, TierAnswer, TierLimits, TierProtocol, TierRequest, Unindexed,
    REFUSALS_PER_WINDOW, TIER_ALPN,
};
use eprfs_storage::{TierBlobManifest, TierBlobPut};
use iroh::endpoint::Connection;
use iroh::protocol::{AcceptError, ProtocolHandler, Router};
use iroh::{Endpoint, NodeAddr};
use support::{
    binding, contest, device, endpoint_row, genesis, household, loopback, peer_key, verifier,
    Checkout, Device, Household, HANDLE,
};
use tokio::sync::mpsc;

const WITHIN: Duration = Duration::from_secs(20);

/// The method the policy grants to the steward's devices.
const GRANTED: &str = "mesh-build";
/// A method no policy grants.
const NOT_SHARED: &str = "not-shared";

/// The policy every holder here enforces: the berth crossings, and `mesh-build` served to the
/// steward's devices.
fn serve_policy() -> serde_json::Value {
    let mut policy = support::policy();
    policy["berth"]["artifacts"] = serde_json::json!({
        "serves": { GRANTED: ["steward"] }
    });
    policy
}

fn descriptor(name: &str) -> MethodDescriptor {
    MethodDescriptor {
        name: name.to_string(),
        tools: Default::default(),
        flags: Vec::new(),
        params: None,
        exec: None,
        step: None,
        embedder: None,
        view: Vec::new(),
        non_path_inputs: Default::default(),
        deterministic: true,
    }
}

/// Bytes the content-defined chunker cuts into more than one chunk: a deterministic stream.
fn artifact_bytes(seed: u64, len: usize) -> Bytes {
    let mut state = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
    let bytes = (0..len)
        .map(|_| {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (state >> 33) as u8
        })
        .collect::<Vec<u8>>();
    Bytes::from(bytes)
}

fn context() -> LocalBlobContext {
    LocalBlobContext {
        custodian: AgentRef("did:key:zDeviceA".into()),
        in_scope_of: atom_cid(&"artifact-scope").expect("scope"),
        occurred_at: "2026-11-07T12:00:00Z".into(),
        crossing: None,
    }
}

/// Put `bytes` in custody as the output of the method named `method`, recording its Produce
/// event in `flow`. The method's descriptor record is put in the store too when `with_method`.
async fn put_artifact<F: FlowStore + ?Sized>(
    store: &dyn EprfsStorage,
    flow: &mut F,
    method: &str,
    bytes: Bytes,
    with_method: bool,
) -> TierBlobPut {
    let descriptor = descriptor(method);
    let method_cid = descriptor.cid().expect("method cid");
    if with_method {
        let encoded = serde_ipld_dagcbor::to_vec(&descriptor).expect("descriptor");
        assert_eq!(BlobCid::compute(&encoded), method_cid);
        store
            .put_blob_verified(&method_cid, Bytes::from(encoded))
            .await
            .expect("method record");
    }
    let record = DerivationRecord {
        key: DerivationKey {
            method: method_cid,
            input: KeyInput::Raw(BlobCid::compute_raw(method.as_bytes())),
        },
        output: BlobCid::compute_bytes(&bytes),
        computed_by: "did:key:zDeviceA".into(),
        basis: DerivationBasis::Computed,
        deterministic: true,
        witness: Witness::Signature {
            sig: "fixture-signature".into(),
        },
    };
    put_local_blob(store, flow, record, bytes, &context())
        .await
        .expect("put")
}

fn output(put: &TierBlobPut) -> BlobCid {
    put.manifest.record.output.clone()
}

fn chunks(put: &TierBlobPut) -> Vec<BlobCid> {
    put.manifest.chunks.iter().map(|c| c.cid.clone()).collect()
}

fn typed(artifact: &BlobCid, reason: ReachReason, standing: Option<&str>) -> TierAnswer {
    TierAnswer::Unanswered {
        cid: artifact.clone(),
        path: TreePath::root(),
        why: ItemAbsence::refused(
            &channel_tier_id(),
            ReachRefusal::new(reason, standing.map(str::to_string)),
        ),
    }
}

fn uniform(artifact: &BlobCid) -> TierAnswer {
    typed(artifact, ReachReason::OutsideStanding, None)
}

/// A device's tier with an artifact mount, served in-process (no daemon): its payload store, the
/// flow store its custody events go to, and what its mount records.
struct Holder {
    payload: Arc<dyn EprfsStorage>,
    flow: MemoryFlowStore,
    mount: Arc<BlobMount>,
    served: mpsc::Receiver<ServedDraw>,
    admission: SharedAdmission,
    protocol: TierProtocol,
    router: Router,
    addr: NodeAddr,
    dir: tempfile::TempDir,
}

async fn holder(h: &Household) -> Holder {
    let dir = tempfile::tempdir().expect("payload dir");
    let payload = open_payload_store(&dir.path().join("payload")).expect("payload store");
    let (mount, served) = BlobMount::new(payload.clone(), h.a.did.clone());
    let mount = Arc::new(mount);
    let admission = h.shared_admission();
    let channel = Arc::new(ChannelStore::default());
    let serves = Rules::from_policy(&serve_policy())
        .expect("rules")
        .serves()
        .clone();
    let scope = DeviceScope::new(
        channel.clone(),
        Some(mount.clone()),
        Arc::new(RwLock::new(serves)),
        admission.clone(),
    );
    let protocol = TierProtocol::with_scope(
        channel_tier_id(),
        channel,
        admission.clone(),
        TierLimits::default(),
        Arc::new(scope),
        Some(mount.clone()),
    )
    .expect("protocol");
    let endpoint = bind(&h.key_a, None, vec![TIER_ALPN.to_vec()])
        .await
        .expect("bind holder");
    let addr = loopback(&endpoint);
    let router = Router::builder(endpoint)
        .accept(TIER_ALPN, protocol.clone())
        .spawn();
    Holder {
        payload,
        flow: MemoryFlowStore::new(),
        mount,
        served,
        admission,
        protocol,
        router,
        addr,
        dir,
    }
}

impl Holder {
    async fn put(&mut self, method: &str, bytes: Bytes, with_method: bool) -> TierBlobPut {
        put_artifact(
            self.payload.as_ref(),
            &mut self.flow,
            method,
            bytes,
            with_method,
        )
        .await
    }

    async fn reindex(&self) -> Vec<PeerError> {
        let events = self.flow.events().expect("events");
        self.mount.rebuild_from_flow(&events).await
    }
}

async fn client(key: &PeerKey) -> Endpoint {
    bind(key, None, vec![]).await.expect("bind client")
}

async fn call(client: &Endpoint, addr: &NodeAddr, req: TierRequest) -> TierAnswer {
    tokio::time::timeout(
        WITHIN,
        tier_call(
            client,
            addr.clone(),
            req,
            &eprfs_core::TreeLimits::default(),
        ),
    )
    .await
    .expect("answers in time")
    .expect("answers")
}

async fn ask(connection: &Connection, req: TierRequest) -> TierAnswer {
    tokio::time::timeout(
        WITHIN,
        tier_request(connection, &req, &eprfs_core::TreeLimits::default()),
    )
    .await
    .expect("answers in time")
    .expect("answers")
}

fn served_bytes(answer: TierAnswer, asked: &BlobCid) -> Bytes {
    match answer {
        TierAnswer::Blob { cid, bytes } if cid == *asked => bytes,
        other => panic!("expected the object {asked}, got {other:?}"),
    }
}

/// One device's checkout (flow store and roster) and its peer state, for a daemon.
struct Side {
    checkout: Checkout,
    state: tempfile::TempDir,
    payload_dir: tempfile::TempDir,
}

impl Side {
    fn new(rows: &[ParticipantRow]) -> Side {
        let checkout = Checkout::new();
        let roster = SidecarRoster::open(checkout.dir.path(), HANDLE).expect("roster");
        for row in rows {
            roster.append(row.clone()).expect("row");
        }
        Side {
            checkout,
            state: tempfile::tempdir().expect("state"),
            payload_dir: tempfile::tempdir().expect("payload"),
        }
    }

    fn payload(&self) -> Arc<dyn EprfsStorage> {
        open_payload_store(&self.payload_dir.path().join("payload")).expect("payload store")
    }

    fn flow(&self) -> SidecarFlowStore {
        self.checkout.store()
    }

    fn serve_events(&self) -> Vec<FlowEvent> {
        self.checkout
            .events()
            .into_iter()
            .map(|(_, event)| event)
            .filter(|event| event.classified_as.iter().any(|slot| slot == "serve-blob"))
            .collect()
    }
}

/// Device A (the holder) and device B (its sibling), with node keys loadable twice.
struct Pair {
    a: Device,
    b: Device,
    rows: Vec<ParticipantRow>,
    key_dir_a: tempfile::TempDir,
    key_dir_b: tempfile::TempDir,
}

fn load_key(dir: &Path) -> PeerKey {
    PeerKey::load_or_generate(dir).expect("key")
}

fn pair() -> Pair {
    let a = device("DeviceA", 1);
    let b = device("DeviceB", 2);
    let (key_dir_a, key_dir_b) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let (key_a, key_b) = (load_key(key_dir_a.path()), load_key(key_dir_b.path()));
    let rows = vec![
        genesis(&a),
        binding(&a, &b),
        endpoint_row(&a, &a, &key_a),
        endpoint_row(&a, &b, &key_b),
    ];
    Pair {
        a,
        b,
        rows,
        key_dir_a,
        key_dir_b,
    }
}

fn free_port() -> u16 {
    std::net::UdpSocket::bind("127.0.0.1:0")
        .and_then(|s| s.local_addr())
        .map(|a| a.port())
        .expect("a free port")
}

fn local(port: u16) -> std::net::SocketAddrV4 {
    std::net::SocketAddrV4::new(std::net::Ipv4Addr::LOCALHOST, port)
}

/// Start device A's daemon over `side`, serving its payload store under [`serve_policy`].
async fn start_holder(p: &Pair, side: &Side, port: u16) -> PeerDaemon {
    start_holder_with(p, side, port, None).await
}

/// [`start_holder`], reloading its rules from `policy` when given.
async fn start_holder_with(
    p: &Pair,
    side: &Side,
    port: u16,
    policy: Option<std::path::PathBuf>,
) -> PeerDaemon {
    let rules = match &policy {
        Some(path) => Rules::from_policy_file(path).expect("policy file"),
        None => Rules::from_policy(&serve_policy()).expect("policy"),
    };
    let signer = p.a.signer();
    let roster = SidecarRoster::open(side.checkout.dir.path(), HANDLE)
        .expect("roster")
        .log_path()
        .to_path_buf();
    PeerDaemon::start(PeerConfig {
        key: load_key(p.key_dir_a.path()),
        device: p.a.did.clone(),
        chain_root: p.a.did.clone(),
        flow_root: side.checkout.dir.path().to_path_buf(),
        roster,
        state_dir: side.state.path().to_path_buf(),
        rules,
        policy,
        relay: None,
        listen: Some(local(port)),
        peers: Vec::new(),
        signer: Arc::new(signer),
        verifier: verifier(&[&p.a, &p.b]),
        log: Arc::new(|_line: &str| {}),
        payload_store: Some(side.payload()),
    })
    .await
    .expect("A starts")
}

async fn eventually(what: &str, mut check: impl FnMut() -> bool) {
    tokio::time::timeout(WITHIN, async {
        while !check() {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("{what} within {WITHIN:?}"));
}

/// Ask on `connection` until the holder answers with an object (its mount indexes on its tick).
async fn until_served_on(connection: &Connection, req: TierRequest) -> TierAnswer {
    tokio::time::timeout(WITHIN, async {
        loop {
            let answer = ask(connection, req.clone()).await;
            if matches!(answer, TierAnswer::Blob { .. }) {
                return answer;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("served within the deadline")
}

/// Ask until the holder answers with an object (its mount indexes on its tick).
async fn until_served(client: &Endpoint, addr: &NodeAddr, req: TierRequest) -> TierAnswer {
    tokio::time::timeout(WITHIN, async {
        loop {
            let answer = call(client, addr, req.clone()).await;
            if matches!(answer, TierAnswer::Blob { .. }) {
                return answer;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("served within the deadline")
}

#[tokio::test(flavor = "multi_thread")]
async fn an_admitted_sibling_draws_a_granted_artifact_and_the_holder_records_one_serve_blob_use() {
    let p = pair();
    let side = Side::new(&p.rows);
    let bytes = artifact_bytes(7, 3 * 1024 * 1024);
    let put = put_artifact(
        side.payload().as_ref(),
        &mut side.flow(),
        GRANTED,
        bytes.clone(),
        true,
    )
    .await;
    assert!(put.manifest.chunks.len() >= 2, "a multi-chunk artifact");
    let port = free_port();
    let daemon = start_holder(&p, &side, port).await;
    let to_a = NodeAddr::new(daemon.node_id())
        .with_direct_addresses([std::net::SocketAddr::V4(local(port))]);
    let b = client(&load_key(p.key_dir_b.path())).await;
    let artifact = output(&put);
    // One session is one requester drawing one artifact over one connection, as a fetcher does.
    let connection = b.connect(to_a.clone(), TIER_ALPN).await.expect("connect");

    let manifest = served_bytes(
        until_served_on(
            &connection,
            TierRequest::Manifest {
                artifact: artifact.clone(),
            },
        )
        .await,
        &put.manifest_cid,
    );
    let manifest: TierBlobManifest = serde_ipld_dagcbor::from_slice(&manifest).expect("manifest");
    assert_eq!(manifest.record.output, artifact, "the fetcher's own check");
    let mut drawn = Vec::new();
    for chunk in &manifest.chunks {
        let answer = ask(
            &connection,
            TierRequest::Blob {
                artifact: artifact.clone(),
                cid: chunk.cid.clone(),
            },
        )
        .await;
        drawn.extend_from_slice(&served_bytes(answer, &chunk.cid));
    }
    assert_eq!(Bytes::from(drawn), bytes);

    // Polls that arrived before the mount indexed the artifact were refused and recorded as
    // such; the draw itself is one session.
    let draws = || {
        side.serve_events()
            .into_iter()
            .filter(|e| !e.classified_as.iter().any(|s| s == "draw:refused"))
            .collect::<Vec<_>>()
    };
    eventually("A records the serve", || !draws().is_empty()).await;
    // A later tick records nothing more for one completed session.
    tokio::time::sleep(Duration::from_millis(800)).await;
    let events = draws();
    assert_eq!(events.len(), 1, "{events:?}");
    let event = &events[0];
    assert_eq!(event.action, ReaVerb::Use);
    assert_eq!(event.provider, AgentRef(p.a.did.clone()));
    assert_eq!(event.receiver, AgentRef(p.b.did.clone()));
    assert_eq!(event.resource, *artifact.as_cid());
    assert_eq!(
        event.quantity,
        Magnitude::Count {
            value: bytes.len() as f64,
            unit: "bytes".into()
        }
    );
    assert_eq!(
        event.classified_as,
        vec![
            "custody-blob".to_string(),
            artifact.to_string(),
            "serve-blob".to_string(),
            format!("copy:{}", p.b.did),
            format!("manifest:{}", put.manifest_cid),
            "draw:completed".to_string(),
        ]
    );
    assert!(
        !event
            .classified_as
            .iter()
            .any(|s| s.starts_with("measure:")),
        "no measure slot"
    );
    let crossing = CrossingAllowList::from_policy(&serve_policy()).expect("crossing");
    assert!(!crossing.admits(event), "a serve-blob Use never crosses");
    // The connection closing after the last chunk adds nothing: the session already closed.
    connection.close(iroh::endpoint::VarInt::from_u32(0), b"");
    tokio::time::sleep(Duration::from_millis(800)).await;
    assert_eq!(draws().len(), 1);

    b.close().await;
    daemon.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn an_admitted_sibling_asking_outside_its_grant_is_refused_with_a_typed_reason() {
    let p = pair();
    let side = Side::new(&p.rows);
    let payload = side.payload();
    let granted = put_artifact(
        payload.as_ref(),
        &mut side.flow(),
        GRANTED,
        artifact_bytes(1, 600 * 1024),
        true,
    )
    .await;
    let other = put_artifact(
        payload.as_ref(),
        &mut side.flow(),
        GRANTED,
        artifact_bytes(2, 600 * 1024),
        true,
    )
    .await;
    let not_shared = put_artifact(
        payload.as_ref(),
        &mut side.flow(),
        NOT_SHARED,
        artifact_bytes(3, 600 * 1024),
        true,
    )
    .await;
    let port = free_port();
    let daemon = start_holder(&p, &side, port).await;
    let to_a = NodeAddr::new(daemon.node_id())
        .with_direct_addresses([std::net::SocketAddr::V4(local(port))]);
    let b = client(&load_key(p.key_dir_b.path())).await;
    // Indexed and served: the grant is live.
    until_served(
        &b,
        &to_a,
        TierRequest::Manifest {
            artifact: output(&granted),
        },
    )
    .await;

    // A method outside the grant: refused naming the standing the device lacks, once the mount
    // has indexed it (until then it is unknown here, and refused without a label).
    let labelled = typed(
        &output(&not_shared),
        ReachReason::OutsideStanding,
        Some("serve-blob:not-shared"),
    );
    tokio::time::timeout(WITHIN, async {
        loop {
            let refused = call(
                &b,
                &to_a,
                TierRequest::Manifest {
                    artifact: output(&not_shared),
                },
            )
            .await;
            if refused == labelled {
                break;
            }
            assert_eq!(
                refused,
                typed(&output(&not_shared), ReachReason::OutsideStanding, None)
            );
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("indexed and refused by name");
    let refused_chunk = call(
        &b,
        &to_a,
        TierRequest::Blob {
            artifact: output(&not_shared),
            cid: chunks(&not_shared)[0].clone(),
        },
    )
    .await;
    assert_eq!(
        refused_chunk,
        typed(
            &output(&not_shared),
            ReachReason::OutsideStanding,
            Some("serve-blob:not-shared")
        )
    );
    // A granted chunk asked under another granted artifact: not under that root.
    let foreign = chunks(&other)[0].clone();
    assert!(!chunks(&granted).contains(&foreign));
    let not_under = call(
        &b,
        &to_a,
        TierRequest::Blob {
            artifact: output(&granted),
            cid: foreign,
        },
    )
    .await;
    assert_eq!(
        not_under,
        typed(&output(&granted), ReachReason::NotUnderRoot, None)
    );

    eventually("A records both refusals", || {
        side.serve_events()
            .iter()
            .filter(|e| e.classified_as.iter().any(|s| s == "draw:refused"))
            .count()
            >= 2
    })
    .await;
    let refusals: Vec<FlowEvent> = side
        .serve_events()
        .into_iter()
        .filter(|e| e.classified_as.iter().any(|s| s == "draw:refused"))
        .collect();
    let reasons: Vec<&str> = refusals
        .iter()
        .flat_map(|e| e.classified_as.iter())
        .filter_map(|s| s.strip_prefix("reason:"))
        .collect();
    assert!(reasons.contains(&"serve-blob:not-shared"), "{reasons:?}");
    assert!(reasons.contains(&"not-under-root"), "{reasons:?}");
    for refusal in &refusals {
        assert_eq!(refusal.receiver, AgentRef(p.b.did.clone()));
        assert_eq!(
            refusal.quantity,
            Magnitude::Count {
                value: 0.0,
                unit: "bytes".into()
            }
        );
        assert!(!refusal
            .classified_as
            .iter()
            .any(|s| s.starts_with("measure:")));
    }
    // The same refusal asked again within the session window is recorded once.
    assert_eq!(
        refusals
            .iter()
            .filter(|e| e.resource == *output(&not_shared).as_cid()
                && e.classified_as
                    .iter()
                    .any(|s| s == "reason:serve-blob:not-shared"))
            .count(),
        1,
        "{refusals:?}"
    );

    b.close().await;
    daemon.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_stranger_gets_the_uniform_refusal_for_granted_ungranted_and_unknown_artifacts() {
    let h = household();
    let mut holder = holder(&h).await;
    let granted = holder
        .put(GRANTED, artifact_bytes(11, 400 * 1024), true)
        .await;
    let ungranted = holder
        .put(NOT_SHARED, artifact_bytes(12, 400 * 1024), true)
        .await;
    assert!(holder.reindex().await.is_empty());
    let unknown = BlobCid::compute_bytes(b"an artifact this device never held");
    let (_dir, stranger_key) = peer_key();
    let stranger = client(&stranger_key).await;

    let mut frames = Vec::new();
    for (artifact, chunk) in [
        (output(&granted), chunks(&granted)[0].clone()),
        (output(&ungranted), chunks(&ungranted)[0].clone()),
        (unknown.clone(), unknown.clone()),
    ] {
        let manifest = call(
            &stranger,
            &holder.addr,
            TierRequest::Manifest {
                artifact: artifact.clone(),
            },
        )
        .await;
        let blob = call(
            &stranger,
            &holder.addr,
            TierRequest::Blob {
                artifact: artifact.clone(),
                cid: chunk,
            },
        )
        .await;
        // The tree endpoint's own refusal of that root, to a stranger.
        let tree = call(
            &stranger,
            &holder.addr,
            TierRequest::List {
                root: artifact.clone(),
                path: TreePath::root(),
            },
        )
        .await;
        assert_eq!(manifest, uniform(&artifact));
        let encoded = |answer: &TierAnswer| serde_ipld_dagcbor::to_vec(answer).expect("encode");
        assert_eq!(
            encoded(&manifest),
            encoded(&tree),
            "byte-identical to the tree"
        );
        assert_eq!(encoded(&blob), encoded(&tree), "byte-identical to the tree");
        // With the echoed CID set aside, the three frames are one frame.
        let TierAnswer::Unanswered { path, why, .. } = manifest else {
            panic!("a refusal");
        };
        frames.push(encoded(&TierAnswer::Unanswered {
            cid: BlobCid::compute(b"echo"),
            path,
            why,
        }));
    }
    assert_eq!(frames[0], frames[1]);
    assert_eq!(frames[1], frames[2]);

    // Strangers are counted, never recorded.
    assert_eq!(holder.protocol.counters().refused_scope, 6);
    assert_eq!(holder.protocol.counters().served_chunks, 0);
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(holder.served.try_recv().is_err(), "nothing recorded");

    stranger.close().await;
    holder.router.shutdown().await.expect("shutdown");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_device_dropped_by_a_reload_is_refused_on_its_next_chunk_on_an_open_connection() {
    let h = household();
    let mut holder = holder(&h).await;
    let put = holder
        .put(GRANTED, artifact_bytes(21, 3 * 1024 * 1024), true)
        .await;
    assert!(holder.reindex().await.is_empty());
    let chunks = chunks(&put);
    assert!(chunks.len() >= 2, "a multi-chunk artifact");
    let artifact = output(&put);
    let b = client(&h.key_b).await;
    let connection = b
        .connect(holder.addr.clone(), TIER_ALPN)
        .await
        .expect("connect");

    served_bytes(
        ask(
            &connection,
            TierRequest::Manifest {
                artifact: artifact.clone(),
            },
        )
        .await,
        &put.manifest_cid,
    );
    served_bytes(
        ask(
            &connection,
            TierRequest::Blob {
                artifact: artifact.clone(),
                cid: chunks[0].clone(),
            },
        )
        .await,
        &chunks[0],
    );

    // A Contest of B's endpoint takes effect at the next request, on this same connection.
    let b_row = endpoint_row(&h.a, &h.b, &h.key_b);
    let mut rows = h.rows.clone();
    rows.push(contest(&h.a, &b_row));
    holder
        .admission
        .write()
        .expect("admission")
        .reload(&rows, &*h.verify);
    let next = ask(
        &connection,
        TierRequest::Blob {
            artifact: artifact.clone(),
            cid: chunks[1].clone(),
        },
    )
    .await;
    assert_eq!(next, uniform(&artifact), "B is a stranger now");

    // Its session closes with the connection, as partial: one chunk of all of them.
    connection.close(iroh::endpoint::VarInt::from_u32(0), b"");
    let draw = tokio::time::timeout(WITHIN, holder.served.recv())
        .await
        .expect("recorded in time")
        .expect("a draw");
    assert_eq!(draw.requester, h.b.did);
    assert_eq!(draw.artifact, artifact);
    assert_eq!(draw.outcome, DrawOutcome::Partial);
    assert_eq!(draw.chunks, 1);
    assert_eq!(
        draw.of,
        chunks
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
    );
    assert_eq!(holder.protocol.counters().served_chunks, 1);

    b.close().await;
    holder.router.shutdown().await.expect("shutdown");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_daemon_down_put_is_indexed_and_served_after_start() {
    let p = pair();
    let side = Side::new(&p.rows);
    // Put while no daemon runs: direct library I/O and one Produce in the flow store.
    let bytes = artifact_bytes(31, 700 * 1024);
    let put = put_artifact(
        side.payload().as_ref(),
        &mut side.flow(),
        GRANTED,
        bytes.clone(),
        true,
    )
    .await;
    let port = free_port();
    let daemon = start_holder(&p, &side, port).await;
    let to_a = NodeAddr::new(daemon.node_id())
        .with_direct_addresses([std::net::SocketAddr::V4(local(port))]);
    let b = client(&load_key(p.key_dir_b.path())).await;

    let manifest = until_served(
        &b,
        &to_a,
        TierRequest::Manifest {
            artifact: output(&put),
        },
    )
    .await;
    served_bytes(manifest, &put.manifest_cid);
    let mut drawn = Vec::new();
    for chunk in chunks(&put) {
        let answer = call(
            &b,
            &to_a,
            TierRequest::Blob {
                artifact: output(&put),
                cid: chunk.clone(),
            },
        )
        .await;
        drawn.extend_from_slice(&served_bytes(answer, &chunk));
    }
    assert_eq!(Bytes::from(drawn), bytes);

    b.close().await;
    daemon.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_manifest_without_its_method_record_is_unindexed_and_never_served() {
    let h = household();
    let mut holder = holder(&h).await;
    let put = holder
        .put(GRANTED, artifact_bytes(41, 400 * 1024), false)
        .await;
    let unindexed = holder.reindex().await;
    assert!(
        matches!(
            unindexed.as_slice(),
            [PeerError::ArtifactUnindexed { manifest, reason: Unindexed::MethodUnknown }]
                if **manifest == put.manifest_cid
        ),
        "{unindexed:?}"
    );
    assert_eq!(
        holder.mount.unindexed(&put.manifest_cid),
        Some(Unindexed::MethodUnknown)
    );
    assert!(holder.mount.held(&output(&put)).is_none());

    let b = client(&h.key_b).await;
    let artifact = output(&put);
    // Even under a granted method name, an artifact whose method cannot be read is unknown.
    for req in [
        TierRequest::Manifest {
            artifact: artifact.clone(),
        },
        TierRequest::Blob {
            artifact: artifact.clone(),
            cid: chunks(&put)[0].clone(),
        },
    ] {
        let answer = call(&b, &holder.addr, req).await;
        assert_eq!(answer, typed(&artifact, ReachReason::OutsideStanding, None));
    }
    assert_eq!(holder.protocol.counters().served_manifests, 0);

    // The record arriving later makes it servable at the next re-index, not before.
    let descriptor = descriptor(GRANTED);
    let encoded = serde_ipld_dagcbor::to_vec(&descriptor).expect("descriptor");
    holder
        .payload
        .put_blob_verified(&descriptor.cid().expect("cid"), Bytes::from(encoded))
        .await
        .expect("method record");
    assert!(holder.reindex().await.is_empty());
    let answer = call(&b, &holder.addr, TierRequest::Manifest { artifact }).await;
    served_bytes(answer, &put.manifest_cid);

    b.close().await;
    holder.router.shutdown().await.expect("shutdown");
}

/// A tier that answers every request with the same frame, whatever was asked.
#[derive(Debug, Clone)]
struct Liar {
    frame: Vec<u8>,
}

impl ProtocolHandler for Liar {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        while let Ok((mut send, mut recv)) = connection.accept_bi().await {
            let _ = recv.read_to_end(1024 * 1024).await;
            let _ = send.write_all(&self.frame).await;
            let _ = send.finish();
        }
        Ok(())
    }
}

fn frame_of(answer: &TierAnswer) -> Vec<u8> {
    let body = serde_ipld_dagcbor::to_vec(answer).expect("encode");
    let mut frame = u32::try_from(body.len())
        .expect("small")
        .to_be_bytes()
        .to_vec();
    frame.extend_from_slice(&body);
    frame
}

async fn ask_liar(answer: &TierAnswer, req: TierRequest) -> Result<TierAnswer, PeerError> {
    let (_ds, server_key) = peer_key();
    let (_dc, client_key) = peer_key();
    let endpoint = bind(&server_key, None, vec![TIER_ALPN.to_vec()])
        .await
        .expect("bind liar");
    let addr = loopback(&endpoint);
    let router = Router::builder(endpoint)
        .accept(
            TIER_ALPN,
            Liar {
                frame: frame_of(answer),
            },
        )
        .spawn();
    let client = client(&client_key).await;
    let answer = tokio::time::timeout(
        WITHIN,
        tier_call(&client, addr, req, &eprfs_core::TreeLimits::default()),
    )
    .await
    .expect("answers in time");
    client.close().await;
    router.shutdown().await.expect("shutdown");
    answer
}

#[tokio::test(flavor = "multi_thread")]
async fn a_swapped_manifest_cid_for_output_cid_is_refused_both_ways() {
    let h = household();
    let mut holder = holder(&h).await;
    let put = holder
        .put(GRANTED, artifact_bytes(51, 3 * 1024 * 1024), true)
        .await;
    assert!(holder.reindex().await.is_empty());
    let b = client(&h.key_b).await;
    let artifact = output(&put);
    let manifest = put.manifest_cid.clone();
    let chunk = chunks(&put)[0].clone();

    // The holder: a manifest CID in the artifact slot names no artifact here.
    for req in [
        TierRequest::Manifest {
            artifact: manifest.clone(),
        },
        TierRequest::Blob {
            artifact: manifest.clone(),
            cid: chunk.clone(),
        },
        TierRequest::Blob {
            artifact: manifest.clone(),
            cid: manifest.clone(),
        },
    ] {
        let answer = call(&b, &holder.addr, req).await;
        assert_eq!(answer, typed(&manifest, ReachReason::OutsideStanding, None));
    }
    // The output CID asked as the artifact still serves (the swap is the only refusal).
    let served = call(
        &b,
        &holder.addr,
        TierRequest::Manifest {
            artifact: artifact.clone(),
        },
    )
    .await;
    let manifest_bytes = served_bytes(served, &manifest);
    let chunk_bytes = served_bytes(
        call(
            &b,
            &holder.addr,
            TierRequest::Blob {
                artifact: artifact.clone(),
                cid: chunk.clone(),
            },
        )
        .await,
        &chunk,
    );

    // The fetcher: an object served under another CID than the one asked is refused, and so
    // are bytes that do not hash to the CID they are served under.
    let swapped = TierAnswer::Blob {
        cid: manifest.clone(),
        bytes: manifest_bytes,
    };
    let asked_chunk = TierRequest::Blob {
        artifact: artifact.clone(),
        cid: chunk.clone(),
    };
    assert!(matches!(
        ask_liar(&swapped, asked_chunk).await,
        Err(PeerError::TierLeafIntegrity { cid }) if cid == manifest
    ));
    let unverified = TierAnswer::Blob {
        cid: manifest.clone(),
        bytes: chunk_bytes,
    };
    assert!(matches!(
        ask_liar(&unverified, TierRequest::Manifest { artifact }).await,
        Err(PeerError::TierLeafIntegrity { cid }) if cid == manifest
    ));

    b.close().await;
    holder.router.shutdown().await.expect("shutdown");
}

#[test]
fn the_serve_grant_parses_its_vocabulary_and_refuses_anything_else() {
    let h = household();
    let admission = h.admission();
    let rules = Rules::from_policy(&serve_policy()).expect("rules");
    assert!(rules.serves().grants(&h.b.did, GRANTED, &admission));
    assert!(!rules.serves().grants(&h.b.did, NOT_SHARED, &admission));
    assert!(!rules
        .serves()
        .grants("did:key:zNotAdmitted", GRANTED, &admission));

    let carrier = serde_json::json!({"berth": {"artifacts": {"serves": {
        GRANTED: ["carrier:did:key:zCarrier"]
    }}}});
    let reserved = ArtifactServeList::from_policy(&carrier).expect("reserved audience parses");
    assert!(!reserved.grants("did:key:zCarrier", GRANTED, &admission));
    assert!(!reserved.grants(&h.b.did, GRANTED, &admission));

    for bad in [
        serde_json::json!({"berth": {"artifacts": {"serves": {GRANTED: ["everyone"]}}}}),
        serde_json::json!({"berth": {"artifacts": {"serves": {GRANTED: "steward"}}}}),
        serde_json::json!({"berth": {"artifacts": {"serves": ["steward"]}}}),
    ] {
        assert!(ArtifactServeList::from_policy(&bad).is_err(), "{bad}");
    }
    assert!(ArtifactServeList::from_policy(&support::policy())
        .expect("absent")
        .is_empty());
}

#[test]
fn a_payload_store_is_created_with_private_parents() {
    let dir = tempfile::tempdir().expect("dir");
    let path = dir.path().join("data/epr/payload");
    open_payload_store(&path).expect("created");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for created in [
            dir.path().join("data"),
            dir.path().join("data/epr"),
            path.clone(),
        ] {
            let mode = std::fs::metadata(&created)
                .expect("meta")
                .permissions()
                .mode();
            assert_eq!(mode & 0o077, 0, "{} is private", created.display());
        }
    }
    // Opening it again opens the same store.
    open_payload_store(&path).expect("reopened");
}

impl Holder {
    /// Where the payload store keeps `cid`'s bytes (one file per object).
    fn object_path(&self, cid: &BlobCid) -> std::path::PathBuf {
        self.dir.path().join("payload").join(cid.to_string())
    }

    /// Drain what the mount recorded so far.
    fn drain(&mut self) -> Vec<ServedDraw> {
        let mut draws = Vec::new();
        while let Ok(draw) = self.served.try_recv() {
            draws.push(draw);
        }
        draws
    }
}

/// Write an object file as the store does: readable by this account only.
fn write_object(path: &Path, bytes: &[u8]) {
    use std::io::Write as _;
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options
        .open(path)
        .and_then(|mut file| file.write_all(bytes))
        .expect("write object");
}

/// The Produce event `put` wrote, with `edit` applied, appended to `holder`'s flow store.
fn append_edited(holder: &mut Holder, from: &MemoryFlowStore, edit: impl FnOnce(&mut FlowEvent)) {
    let (_, mut event) = from
        .events()
        .expect("events")
        .into_iter()
        .find(|(_, event)| event.action == ReaVerb::Produce)
        .expect("a custody Produce");
    edit(&mut event);
    holder
        .flow
        .append(elohim_epr_rea::FlowRecord::Event(event))
        .expect("append");
}

#[tokio::test(flavor = "multi_thread")]
async fn replacing_a_derivation_drops_the_old_manifest_from_membership() {
    let h = household();
    let mut holder = holder(&h).await;
    let bytes = artifact_bytes(61, 700 * 1024);
    // One output, two derivations: first under an ungranted method, then under a granted one.
    let old = holder.put(NOT_SHARED, bytes.clone(), true).await;
    let new = holder.put(GRANTED, bytes, true).await;
    assert_eq!(output(&old), output(&new));
    assert_ne!(old.manifest_cid, new.manifest_cid);
    assert!(holder.reindex().await.is_empty());
    let artifact = output(&new);
    assert_eq!(
        holder.mount.held(&artifact).map(|held| held.manifest),
        Some(new.manifest_cid.clone())
    );
    let b = client(&h.key_b).await;
    let connection = b
        .connect(holder.addr.clone(), TIER_ALPN)
        .await
        .expect("connect");

    // The superseded, ungranted manifest is no object of the granted entry.
    for _ in 0..2 {
        let stale = ask(
            &connection,
            TierRequest::Blob {
                artifact: artifact.clone(),
                cid: old.manifest_cid.clone(),
            },
        )
        .await;
        assert_eq!(
            stale,
            typed(&artifact, ReachReason::NotUnderRoot, None),
            "never served under the new method"
        );
        // A rebuild never brings the superseded derivation back.
        assert!(holder.reindex().await.is_empty());
    }
    served_bytes(
        ask(
            &connection,
            TierRequest::Manifest {
                artifact: artifact.clone(),
            },
        )
        .await,
        &new.manifest_cid,
    );
    for chunk in chunks(&new) {
        served_bytes(
            ask(
                &connection,
                TierRequest::Blob {
                    artifact: artifact.clone(),
                    cid: chunk.clone(),
                },
            )
            .await,
            &chunk,
        );
    }
    b.close().await;
    holder.router.shutdown().await.expect("shutdown");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_folded_sibling_produce_never_indexes_a_manifest() {
    let h = household();
    let mut holder = holder(&h).await;
    // The manifest, its chunks and its method record are all held here, but the only Produce
    // naming them came from B: folded `via` B, as fold.rs appends it, and crossing as F2's
    // artifact Produce will.
    let mut sibling_flow = MemoryFlowStore::new();
    let put = put_artifact(
        holder.payload.as_ref(),
        &mut sibling_flow,
        GRANTED,
        artifact_bytes(71, 400 * 1024),
        true,
    )
    .await;
    let b_did = h.b.did.clone();
    append_edited(&mut holder, &sibling_flow, |event| {
        event
            .classified_as
            .push("measure:artifact-produce@1".into());
        event.classified_as.push("reason:produce".into());
        event.classified_as.push(format!("via:{b_did}"));
    });
    assert!(holder.reindex().await.is_empty(), "not even tried");
    assert!(holder.mount.held(&output(&put)).is_none());
    assert!(holder.mount.unindexed(&put.manifest_cid).is_none());

    let b = client(&h.key_b).await;
    let answer = call(
        &b,
        &holder.addr,
        TierRequest::Manifest {
            artifact: output(&put),
        },
    )
    .await;
    assert_eq!(
        answer,
        typed(&output(&put), ReachReason::OutsideStanding, None)
    );
    b.close().await;
    holder.router.shutdown().await.expect("shutdown");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_produce_naming_another_provider_is_not_local_custody() {
    let h = household();
    let mut holder = holder(&h).await;
    // Custody in a Produce is its receiver (`LocalBlobContext::custodian`); its provider is the
    // computing agent. A Produce putting the artifact in B's custody is B's, not this device's.
    let mut elsewhere = MemoryFlowStore::new();
    let theirs = put_artifact(
        holder.payload.as_ref(),
        &mut elsewhere,
        GRANTED,
        artifact_bytes(81, 400 * 1024),
        true,
    )
    .await;
    let b_did = h.b.did.clone();
    append_edited(&mut holder, &elsewhere, |event| {
        event.receiver = AgentRef(b_did.clone());
        event.provider = AgentRef(b_did);
    });
    // Computed by another agent, received into this device's custody: this device's own.
    let mut computed_elsewhere = MemoryFlowStore::new();
    let ours = put_artifact(
        holder.payload.as_ref(),
        &mut computed_elsewhere,
        GRANTED,
        artifact_bytes(82, 400 * 1024),
        true,
    )
    .await;
    append_edited(&mut holder, &computed_elsewhere, |event| {
        event.provider = AgentRef("did:key:zSomeBuilder".into());
    });
    assert!(holder.reindex().await.is_empty());
    assert!(holder.mount.held(&output(&theirs)).is_none(), "B's custody");
    assert!(
        holder.mount.held(&output(&ours)).is_some(),
        "this device's custody"
    );

    let b = client(&h.key_b).await;
    let refused = call(
        &b,
        &holder.addr,
        TierRequest::Manifest {
            artifact: output(&theirs),
        },
    )
    .await;
    assert_eq!(
        refused,
        typed(&output(&theirs), ReachReason::OutsideStanding, None)
    );
    let served = call(
        &b,
        &holder.addr,
        TierRequest::Manifest {
            artifact: output(&ours),
        },
    )
    .await;
    served_bytes(served, &ours.manifest_cid);
    b.close().await;
    holder.router.shutdown().await.expect("shutdown");
}

#[tokio::test(flavor = "multi_thread")]
async fn refusal_events_per_requester_are_bounded() {
    let h = household();
    let mut holder = holder(&h).await;
    let b = client(&h.key_b).await;
    let connection = b
        .connect(holder.addr.clone(), TIER_ALPN)
        .await
        .expect("connect");
    let asked = 3 * REFUSALS_PER_WINDOW;
    for n in 0..asked {
        let unknown = BlobCid::compute_bytes(format!("never held {n}").as_bytes());
        let answer = ask(
            &connection,
            TierRequest::Manifest {
                artifact: unknown.clone(),
            },
        )
        .await;
        // Every refusal is still answered, typed.
        assert_eq!(answer, typed(&unknown, ReachReason::OutsideStanding, None));
    }
    // The same refusal asked again is neither recorded nor counted twice.
    let again = BlobCid::compute_bytes(b"never held 0");
    ask(&connection, TierRequest::Manifest { artifact: again }).await;

    let draws = holder.drain();
    assert_eq!(
        draws.len(),
        REFUSALS_PER_WINDOW,
        "recorded up to the window's bound"
    );
    assert!(draws
        .iter()
        .all(|draw| matches!(draw.outcome, DrawOutcome::Refused { .. })));
    assert_eq!(
        holder.mount.suppressed_refusals(),
        (asked - REFUSALS_PER_WINDOW) as u64,
        "the rest only counted"
    );
    assert_eq!(holder.mount.dropped_draws(), 0);
    assert_eq!(holder.protocol.counters().refused_scope, asked as u64 + 1);
    b.close().await;
    holder.router.shutdown().await.expect("shutdown");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_malformed_policy_reload_revokes_serving() {
    let p = pair();
    let side = Side::new(&p.rows);
    let put = put_artifact(
        side.payload().as_ref(),
        &mut side.flow(),
        GRANTED,
        artifact_bytes(91, 400 * 1024),
        true,
    )
    .await;
    let policy_dir = tempfile::tempdir().expect("policy dir");
    let policy = policy_dir.path().join("pool-policy.json");
    let valid = serde_json::to_vec_pretty(&serve_policy()).expect("policy");
    std::fs::write(&policy, &valid).expect("write policy");
    let port = free_port();
    let daemon = start_holder_with(&p, &side, port, Some(policy.clone())).await;
    let to_a = NodeAddr::new(daemon.node_id())
        .with_direct_addresses([std::net::SocketAddr::V4(local(port))]);
    let b = client(&load_key(p.key_dir_b.path())).await;
    let manifest = TierRequest::Manifest {
        artifact: output(&put),
    };
    served_bytes(
        until_served(&b, &to_a, manifest.clone()).await,
        &put.manifest_cid,
    );
    let revoked = typed(
        &output(&put),
        ReachReason::OutsideStanding,
        Some("serve-blob:mesh-build"),
    );

    let mut unknown_audience = serve_policy();
    unknown_audience["berth"]["artifacts"]["serves"][GRANTED] = serde_json::json!(["everyone"]);
    for bad in [
        b"{ not a policy".to_vec(),
        serde_json::to_vec(&unknown_audience).expect("policy"),
    ] {
        // Through the real reload path: the file the daemon watches changes.
        std::fs::write(&policy, &bad).expect("write bad policy");
        tokio::time::timeout(WITHIN, async {
            while call(&b, &to_a, manifest.clone()).await != revoked {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        .expect("serving revoked by a policy that does not read");
        // A valid policy restores the grant.
        std::fs::write(&policy, &valid).expect("restore policy");
        served_bytes(
            until_served(&b, &to_a, manifest.clone()).await,
            &put.manifest_cid,
        );
    }

    b.close().await;
    daemon.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn an_output_its_chunks_do_not_reconstruct_here_is_not_served() {
    let h = household();
    let mut holder = holder(&h).await;
    let b = client(&h.key_b).await;

    // A chunk missing here: deferred, not served, tried again at the next rebuild.
    let deferred = holder
        .put(GRANTED, artifact_bytes(101, 3 * 1024 * 1024), true)
        .await;
    let last = chunks(&deferred).last().cloned().expect("a chunk");
    let kept = std::fs::read(holder.object_path(&last)).expect("chunk file");
    std::fs::remove_file(holder.object_path(&last)).expect("remove chunk");
    let unindexed = holder.reindex().await;
    assert!(
        matches!(
            holder.mount.unindexed(&deferred.manifest_cid),
            Some(Unindexed::OutputUnverified { .. })
        ),
        "{unindexed:?}"
    );
    assert_eq!(
        call(
            &b,
            &holder.addr,
            TierRequest::Manifest {
                artifact: output(&deferred),
            }
        )
        .await,
        typed(&output(&deferred), ReachReason::OutsideStanding, None)
    );
    write_object(&holder.object_path(&last), &kept);
    let retried = holder.reindex().await;
    assert!(
        retried.is_empty(),
        "verified once the chunk is held: {retried:?}"
    );
    assert!(holder.mount.held(&output(&deferred)).is_some());

    // A chunk whose bytes do not hash to it: invalid, never served.
    let corrupt = holder
        .put(GRANTED, artifact_bytes(102, 400 * 1024), true)
        .await;
    let chunk = chunks(&corrupt)[0].clone();
    // Private, as the store writes objects, so only the hash is wrong.
    std::fs::remove_file(holder.object_path(&chunk)).expect("remove chunk");
    write_object(&holder.object_path(&chunk), b"not the chunk");
    holder.reindex().await;
    assert!(holder.mount.held(&output(&corrupt)).is_none());
    assert!(matches!(
        holder.mount.unindexed(&corrupt.manifest_cid),
        Some(Unindexed::ManifestInvalid { .. })
    ));

    // An accepted entry whose manifest no longer reads is dropped at the next rebuild.
    std::fs::remove_file(holder.object_path(&deferred.manifest_cid)).expect("remove manifest");
    let dropped = holder.reindex().await;
    assert!(
        dropped.iter().any(|error| matches!(
            error,
            PeerError::ArtifactUnindexed { manifest, .. } if **manifest == deferred.manifest_cid
        )),
        "{dropped:?}"
    );
    assert!(holder.mount.held(&output(&deferred)).is_none());
    assert_eq!(
        call(
            &b,
            &holder.addr,
            TierRequest::Blob {
                artifact: output(&deferred),
                cid: chunks(&deferred)[0].clone(),
            }
        )
        .await,
        typed(&output(&deferred), ReachReason::OutsideStanding, None)
    );
    b.close().await;
    holder.router.shutdown().await.expect("shutdown");
}
