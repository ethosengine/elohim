//! The sibling fetch (artifact-store design §3.b): a device draws a derivation artifact one of its
//! person's other devices holds, manifest first and capped, every chunk verified before it is
//! stored, the manifest stored last, and the install read whole-blob verified; every other outcome
//! is a typed `FetchGap`.
//!
//! Holders here are either a real tier with an artifact mount (E3's `serve-blob`, which records
//! what it served) or a scripted tier that answers the same frames and can stall, trickle, corrupt
//! a chunk or hang up, and counts what it was asked.

mod support;

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use bytes::Bytes;
use elohim_epr::profile::ByteProfile;
use elohim_epr_rea::{atom_cid, AgentRef, FlowEvent, FlowStore, MemoryFlowStore, ReaVerb};
use eprfs_core::{
    BlobCid, BlobPresence, DerivationBasis, DerivationKey, DerivationRecord, EprfsStorage,
    KeyInput, MethodDescriptor, Witness,
};
use eprfs_peer::{
    bind, channel_tier_id, fetch_artifact, install_fetched, key_cid, open_payload_store,
    put_local_blob, put_method_record, resolve_holders, stream_local_blob, BlobMount, ChannelStore,
    DeviceScope, DrawOutcome, FetchBudget, FetchGap, FetchTransport, FetchedArtifact, Holder,
    LocalBlobContext, PeerKey, Rules, ServedDraw, SiblingRecord, TierAnswer, TierLimits,
    TierProtocol, TierRequest, TIER_ALPN,
};
use eprfs_storage::{put_tier_blob, MemoryStorage, TierBlobChunk, TierBlobManifest};
use iroh::endpoint::{Connection, RecvStream, SendStream, VarInt};
use iroh::protocol::{AcceptError, ProtocolHandler, Router};
use iroh::{Endpoint, NodeAddr};
use serde::Serialize;
use support::{binding, device, endpoint_row, genesis, loopback, peer_key, verifier, Device};
use tokio::sync::mpsc;

const WITHIN: Duration = Duration::from_secs(30);

// ---------------------------------------------------------------------------
// fixtures
// ---------------------------------------------------------------------------

/// The person's three devices: A and B hold, C fetches.
struct Three {
    a: Device,
    b: Device,
    c: Device,
    key_a: PeerKey,
    _key_b: PeerKey,
    key_c: PeerKey,
    rows: Vec<elohim_epr_rea::ParticipantRow>,
    /// The peer key directories of A, B and C, in that order.
    _dirs: Vec<tempfile::TempDir>,
}

fn three() -> Three {
    let (a, b, c) = (
        device("DeviceA", 1),
        device("DeviceB", 2),
        device("DeviceC", 3),
    );
    let (da, key_a) = peer_key();
    let (db, key_b) = peer_key();
    let (dc, key_c) = peer_key();
    let rows = vec![
        genesis(&a),
        binding(&a, &b),
        binding(&a, &c),
        endpoint_row(&a, &a, &key_a),
        endpoint_row(&a, &b, &key_b),
        endpoint_row(&a, &c, &key_c),
    ];
    Three {
        a,
        b,
        c,
        key_a,
        _key_b: key_b,
        key_c,
        rows,
        _dirs: vec![da, db, dc],
    }
}

fn descriptor() -> MethodDescriptor {
    MethodDescriptor {
        name: "mesh-build".into(),
        tools: BTreeMap::from([("toolchain".to_string(), "1.96.1".to_string())]),
        flags: vec!["--bin=elohim-storage".into(), "--profile=dev".into()],
        params: None,
        exec: None,
        step: Some("mesh-build:elohim-storage".into()),
        embedder: None,
        view: Vec::new(),
        non_path_inputs: BTreeMap::new(),
        deterministic: false,
    }
}

fn key_for(closure: &str) -> DerivationKey {
    DerivationKey {
        method: descriptor().cid().expect("method"),
        input: KeyInput::Raw(BlobCid::compute_bytes(closure.as_bytes())),
    }
}

fn record(key: &DerivationKey, bytes: &[u8]) -> DerivationRecord {
    DerivationRecord {
        key: key.clone(),
        output: BlobCid::compute_bytes(bytes),
        computed_by: "a".repeat(64),
        basis: DerivationBasis::Computed,
        deterministic: false,
        witness: Witness::Attestation {
            epr: "brit-build-ref:mesh-build:elohim-storage@0000".into(),
        },
    }
}

/// Bytes the content-defined chunker cuts into several chunks.
fn artifact_bytes(seed: u64, len: usize) -> Bytes {
    let mut state = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
    Bytes::from(
        (0..len)
            .map(|_| {
                state = state
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                (state >> 33) as u8
            })
            .collect::<Vec<u8>>(),
    )
}

fn context(custodian: &str) -> LocalBlobContext {
    LocalBlobContext {
        custodian: AgentRef(custodian.to_string()),
        in_scope_of: atom_cid(&"artifact-scope").expect("scope"),
        occurred_at: "2026-11-08T12:00:00Z".into(),
        crossing: None,
    }
}

fn budget() -> FetchBudget {
    FetchBudget {
        stall: Duration::from_secs(10),
        total: Duration::from_secs(60),
        ..FetchBudget::default()
    }
}

fn payload() -> (tempfile::TempDir, Arc<dyn EprfsStorage>) {
    let dir = tempfile::tempdir().expect("payload dir");
    let store = open_payload_store(&dir.path().join("payload")).expect("payload store");
    (dir, store)
}

fn distinct(manifest: &TierBlobManifest) -> Vec<TierBlobChunk> {
    let mut seen = std::collections::BTreeSet::new();
    manifest
        .chunks
        .iter()
        .filter(|chunk| seen.insert(chunk.cid.clone()))
        .cloned()
        .collect()
}

async fn is_held(store: &dyn EprfsStorage, cid: &BlobCid) -> bool {
    matches!(store.has_blob(cid).await, Ok(BlobPresence::Local))
}

fn slots(event: &FlowEvent) -> Vec<&str> {
    event.classified_as.iter().map(String::as_str).collect()
}

fn uses(flow: &MemoryFlowStore) -> Vec<FlowEvent> {
    flow.events()
        .expect("events")
        .into_iter()
        .map(|(_, event)| event)
        .filter(|event| event.action == ReaVerb::Use)
        .collect()
}

// ---------------------------------------------------------------------------
// a real holder: a tier with an artifact mount (E3)
// ---------------------------------------------------------------------------

struct RealHolder {
    store: Arc<dyn EprfsStorage>,
    flow: MemoryFlowStore,
    mount: Arc<BlobMount>,
    served: mpsc::Receiver<ServedDraw>,
    addr: NodeAddr,
    _router: Router,
    _dir: tempfile::TempDir,
}

async fn real_holder(t: &Three, me: &Device, key: &PeerKey) -> RealHolder {
    let (dir, store) = payload();
    let (mount, served) = BlobMount::new(store.clone(), me.did.clone());
    let mount = Arc::new(mount);
    let admission = Arc::new(RwLock::new(elohim_epr_rea_admission(t)));
    let channel = Arc::new(ChannelStore::default());
    let mut policy = support::policy();
    policy["berth"]["artifacts"] = serde_json::json!({ "serves": { "mesh-build": ["steward"] } });
    let serves = Rules::from_policy(&policy).expect("rules").serves().clone();
    let scope = DeviceScope::new(
        channel.clone(),
        Some(mount.clone()),
        Arc::new(RwLock::new(serves)),
        admission.clone(),
    );
    let protocol = TierProtocol::with_scope(
        channel_tier_id(),
        channel,
        admission,
        TierLimits::default(),
        Arc::new(scope),
        Some(mount.clone()),
    )
    .expect("protocol");
    let endpoint = bind(key, None, vec![TIER_ALPN.to_vec()])
        .await
        .expect("bind holder");
    let addr = loopback(&endpoint);
    let router = Router::builder(endpoint)
        .accept(TIER_ALPN, protocol)
        .spawn();
    RealHolder {
        store,
        flow: MemoryFlowStore::new(),
        mount,
        served,
        addr,
        _router: router,
        _dir: dir,
    }
}

fn elohim_epr_rea_admission(t: &Three) -> eprfs_peer::Admission {
    eprfs_peer::Admission::from_roster(&t.rows, &t.a.did, &*verifier(&[&t.a, &t.b, &t.c]))
}

impl RealHolder {
    /// Put `bytes` in this holder's custody under `key`, with the method record, and index it.
    async fn put(&mut self, custodian: &str, key: &DerivationKey, bytes: Bytes) -> BlobCid {
        put_method_record(self.store.as_ref(), &descriptor())
            .await
            .expect("method record");
        let put = put_local_blob(
            self.store.as_ref(),
            &mut self.flow,
            record(key, &bytes),
            bytes,
            &context(custodian),
        )
        .await
        .expect("put");
        let events = self.flow.events().expect("events");
        let unindexed = self.mount.rebuild_from_flow(&events).await;
        assert!(unindexed.is_empty(), "{unindexed:?}");
        put.manifest_cid
    }

    fn holder(&self, device: &str, manifest: &BlobCid) -> Holder {
        Holder {
            device: device.to_string(),
            addr: self.addr.clone(),
            manifest: manifest.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// a scripted holder: the tier's frames, with faults
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
struct Faults {
    /// Every chunk answer waits this long first.
    delay_chunks: Option<Duration>,
    /// This chunk is answered with other bytes under its CID.
    corrupt: Option<BlobCid>,
    /// After this many chunk answers, the holder hangs up on the next chunk asked.
    hang_up_after: Option<usize>,
}

#[derive(Clone)]
struct Scripted {
    store: Arc<MemoryStorage>,
    manifest: (BlobCid, Bytes),
    faults: Faults,
    asked: Arc<Mutex<Vec<TierRequest>>>,
    connections: Arc<AtomicUsize>,
    answered: Arc<AtomicUsize>,
}

impl std::fmt::Debug for Scripted {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Scripted").finish_non_exhaustive()
    }
}

async fn read_request(recv: &mut RecvStream) -> Option<TierRequest> {
    let mut header = [0u8; 4];
    recv.read_exact(&mut header).await.ok()?;
    let len = u32::from_be_bytes(header) as usize;
    let mut body = vec![0u8; len];
    recv.read_exact(&mut body).await.ok()?;
    serde_ipld_dagcbor::from_slice(&body).ok()
}

async fn write_answer(send: &mut SendStream, answer: &TierAnswer) {
    let body = serde_ipld_dagcbor::to_vec(answer).expect("encode");
    let mut frame = u32::try_from(body.len())
        .expect("small")
        .to_be_bytes()
        .to_vec();
    frame.extend_from_slice(&body);
    let _ = send.write_all(&frame).await;
    let _ = send.finish();
}

impl Scripted {
    async fn answer(&self, connection: Connection, mut send: SendStream, mut recv: RecvStream) {
        let Some(request) = read_request(&mut recv).await else {
            return;
        };
        self.asked.lock().expect("asked").push(request.clone());
        let answer = match &request {
            TierRequest::Manifest { .. } => TierAnswer::Blob {
                cid: self.manifest.0.clone(),
                bytes: self.manifest.1.clone(),
            },
            TierRequest::Blob { cid, .. } => {
                let answered = self.answered.fetch_add(1, Ordering::SeqCst) + 1;
                if self
                    .faults
                    .hang_up_after
                    .is_some_and(|limit| answered > limit)
                {
                    connection.close(VarInt::from_u32(9), b"hung up");
                    return;
                }
                if let Some(delay) = self.faults.delay_chunks {
                    tokio::time::sleep(delay).await;
                }
                let bytes = self
                    .store
                    .fetch_blob(cid, eprfs_core::FetchPolicy::LocalOnly)
                    .await
                    .expect("scripted holder holds every chunk")
                    .bytes;
                let bytes = if self.faults.corrupt.as_ref() == Some(cid) {
                    Bytes::from(vec![0u8; bytes.len()])
                } else {
                    bytes
                };
                TierAnswer::Blob {
                    cid: cid.clone(),
                    bytes,
                }
            }
            _ => TierAnswer::Failed {
                reason: "scripted".into(),
            },
        };
        write_answer(&mut send, &answer).await;
    }

    fn chunk_asks(&self) -> Vec<BlobCid> {
        self.asked
            .lock()
            .expect("asked")
            .iter()
            .filter_map(|request| match request {
                TierRequest::Blob { cid, .. } => Some(cid.clone()),
                _ => None,
            })
            .collect()
    }

    fn manifest_asks(&self) -> usize {
        self.asked
            .lock()
            .expect("asked")
            .iter()
            .filter(|request| matches!(request, TierRequest::Manifest { .. }))
            .count()
    }
}

impl ProtocolHandler for Scripted {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        self.connections.fetch_add(1, Ordering::SeqCst);
        let mut streams = tokio::task::JoinSet::new();
        while let Ok((send, recv)) = connection.accept_bi().await {
            let this = self.clone();
            let connection = connection.clone();
            streams.spawn(async move { this.answer(connection, send, recv).await });
        }
        while streams.join_next().await.is_some() {}
        Ok(())
    }
}

/// An artifact held in memory: its chunks, manifest bytes and CID.
struct Held {
    store: Arc<MemoryStorage>,
    manifest_cid: BlobCid,
    manifest_bytes: Bytes,
    manifest: TierBlobManifest,
    bytes: Bytes,
}

async fn held_artifact(key: &DerivationKey, bytes: Bytes) -> Held {
    let store = Arc::new(MemoryStorage::default());
    let put = put_tier_blob(store.as_ref(), record(key, &bytes), bytes.clone())
        .await
        .expect("put");
    let manifest_bytes = store
        .fetch_blob(&put.manifest_cid, eprfs_core::FetchPolicy::LocalOnly)
        .await
        .expect("manifest")
        .bytes;
    Held {
        store,
        manifest_cid: put.manifest_cid,
        manifest_bytes,
        manifest: put.manifest,
        bytes,
    }
}

struct Running {
    tier: Scripted,
    addr: NodeAddr,
    _router: Router,
    _key: tempfile::TempDir,
}

async fn scripted(held: &Held, faults: Faults) -> Running {
    let (dir, key) = peer_key();
    let tier = Scripted {
        store: held.store.clone(),
        manifest: (held.manifest_cid.clone(), held.manifest_bytes.clone()),
        faults,
        asked: Arc::default(),
        connections: Arc::default(),
        answered: Arc::default(),
    };
    let endpoint = bind(&key, None, vec![TIER_ALPN.to_vec()])
        .await
        .expect("bind scripted");
    let addr = loopback(&endpoint);
    let router = Router::builder(endpoint)
        .accept(TIER_ALPN, tier.clone())
        .spawn();
    Running {
        tier,
        addr,
        _router: router,
        _key: dir,
    }
}

impl Running {
    fn holder(&self, device: &str, manifest: &BlobCid) -> Holder {
        Holder {
            device: device.to_string(),
            addr: self.addr.clone(),
            manifest: manifest.clone(),
        }
    }
}

async fn fetcher(key: &PeerKey) -> Endpoint {
    bind(key, None, vec![]).await.expect("bind fetcher")
}

async fn within<T>(what: &str, future: impl std::future::Future<Output = T>) -> T {
    tokio::time::timeout(WITHIN, future)
        .await
        .unwrap_or_else(|_| panic!("{what} within {WITHIN:?}"))
}

// ---------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn a_sibling_fetches_the_closure_from_one_holder_and_records_use_against_the_holders_produce()
{
    let t = three();
    let mut a = real_holder(&t, &t.a, &t.key_a).await;
    let key = key_for("closure-at-head");
    let bytes = artifact_bytes(11, 3 * 1024 * 1024);
    let manifest = a.put(&t.a.did, &key, bytes.clone()).await;
    let produce = a
        .flow
        .events()
        .expect("events")
        .into_iter()
        .map(|(_, event)| event)
        .find(|event| event.action == ReaVerb::Produce)
        .expect("the holder's Produce");
    let output = BlobCid::compute_bytes(&bytes);

    let endpoint = fetcher(&t.key_c).await;
    let (_cdir, store) = payload();
    let fetched = within(
        "the fetch",
        fetch_artifact(
            &endpoint,
            store.as_ref(),
            &key,
            &output,
            &[a.holder(&t.a.did, &manifest)],
            &budget(),
        ),
    )
    .await
    .expect("fetched");
    assert_eq!(fetched.manifest_cid, manifest);
    assert_eq!(fetched.chunks_resumed, 0);
    assert_eq!(fetched.chunks_fetched, distinct(&fetched.manifest).len());
    assert_eq!(fetched.wasted, 0);
    assert!(
        is_held(store.as_ref(), &manifest).await,
        "the manifest is stored last, and stored"
    );

    let dest_dir = tempfile::tempdir().expect("dest");
    let dest = dest_dir.path().join("mesh-bin/elohim-storage");
    let mut flow = MemoryFlowStore::new();
    install_fetched(
        store.as_ref(),
        &mut flow,
        &fetched,
        &context(&t.c.did),
        &dest,
    )
    .await
    .expect("installed");
    assert_eq!(
        std::fs::read(&dest).expect("installed bytes"),
        bytes.to_vec()
    );

    // The fetcher's one Use draws the very resource the holder's Produce produced.
    let used = uses(&flow);
    assert_eq!(used.len(), 1, "{used:?}");
    let draw = &used[0];
    assert_eq!(draw.resource, produce.resource);
    assert_eq!(draw.receiver, AgentRef(t.c.did.clone()));
    let draw_slots = slots(draw);
    for slot in [
        "draw:hit",
        "copy:self",
        "basis:sibling",
        &format!("fetched-from:{}", t.a.did),
        "wasted:0",
    ] {
        assert!(draw_slots.contains(&slot), "{slot} in {draw_slots:?}");
    }
    assert!(
        !draw_slots.iter().any(|slot| slot.starts_with("measure:")),
        "the install Use never crosses"
    );

    // And the holder recorded serving it to the fetcher.
    let served = within("the holder's serve record", a.served.recv())
        .await
        .expect("a draw");
    assert_eq!(served.requester, t.c.did);
    assert_eq!(served.artifact, output);
    assert_eq!(served.outcome, DrawOutcome::Completed);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_fetch_draws_one_artifact_over_one_connection() {
    let t = three();
    let key = key_for("closure-at-head");
    // A real holder: one connection is one serve session, recorded once as completed.
    let mut a = real_holder(&t, &t.a, &t.key_a).await;
    let bytes = artifact_bytes(12, 5 * 1024 * 1024);
    let manifest = a.put(&t.a.did, &key, bytes.clone()).await;
    let output = BlobCid::compute_bytes(&bytes);
    let endpoint = fetcher(&t.key_c).await;
    let (_cdir, store) = payload();
    let fetched = within(
        "the fetch",
        fetch_artifact(
            &endpoint,
            store.as_ref(),
            &key,
            &output,
            &[a.holder(&t.a.did, &manifest)],
            &budget(),
        ),
    )
    .await
    .expect("fetched");
    assert!(fetched.chunks_fetched > 1, "a multi-chunk artifact");
    let served = within("the holder's serve record", a.served.recv())
        .await
        .expect("a draw");
    assert_eq!(served.outcome, DrawOutcome::Completed);
    assert_eq!(served.chunks, served.of);
    // Nothing further: no partial session per chunk.
    tokio::time::sleep(Duration::from_millis(500)).await;
    a.mount
        .sweep_idle(Instant::now() + Duration::from_secs(3600));
    assert!(
        a.served.try_recv().is_err(),
        "exactly one serve-blob session"
    );

    // A scripted holder counts the connections themselves.
    let held = held_artifact(&key, artifact_bytes(13, 5 * 1024 * 1024)).await;
    let b = scripted(&held, Faults::default()).await;
    let (_ddir, store) = payload();
    let output = held.manifest.record.output.clone();
    within(
        "the fetch",
        fetch_artifact(
            &endpoint,
            store.as_ref(),
            &key,
            &output,
            &[b.holder(&t.b.did, &held.manifest_cid)],
            &budget(),
        ),
    )
    .await
    .expect("fetched");
    assert_eq!(b.tier.connections.load(Ordering::SeqCst), 1);
    assert_eq!(b.tier.manifest_asks(), 1);
    assert_eq!(b.tier.chunk_asks().len(), distinct(&held.manifest).len());
}

#[tokio::test(flavor = "multi_thread")]
async fn an_interrupted_fetch_resumes_only_missing_chunks_from_the_remaining_holder() {
    let t = three();
    let key = key_for("closure-at-head");
    let held = held_artifact(&key, artifact_bytes(21, 8 * 1024 * 1024)).await;
    let output = held.manifest.record.output.clone();
    let chunks = distinct(&held.manifest);
    assert!(
        chunks.len() >= 5,
        "enough chunks to interrupt: {}",
        chunks.len()
    );
    let endpoint = fetcher(&t.key_c).await;
    let (_cdir, store) = payload();

    // A hangs up after three chunks.
    let a = scripted(
        &held,
        Faults {
            hang_up_after: Some(3),
            ..Faults::default()
        },
    )
    .await;
    let first = within(
        "the interrupted fetch",
        fetch_artifact(
            &endpoint,
            store.as_ref(),
            &key,
            &output,
            &[a.holder(&t.a.did, &held.manifest_cid)],
            &FetchBudget {
                in_flight_per_holder: 1,
                ..budget()
            },
        ),
    )
    .await;
    assert!(
        matches!(first, Err(FetchGap::NoRoute { .. })),
        "a hung-up holder is a gap: {first:?}"
    );
    assert!(
        !is_held(store.as_ref(), &held.manifest_cid).await,
        "no manifest before every chunk"
    );
    let mut already = Vec::new();
    for chunk in &chunks {
        if is_held(store.as_ref(), &chunk.cid).await {
            already.push(chunk.cid.clone());
        }
    }
    assert!(
        !already.is_empty(),
        "the interrupted fetch kept what verified"
    );
    assert!(already.len() < chunks.len());

    // B is asked for exactly the chunks still missing.
    let b = scripted(&held, Faults::default()).await;
    let fetched = within(
        "the resumed fetch",
        fetch_artifact(
            &endpoint,
            store.as_ref(),
            &key,
            &output,
            &[b.holder(&t.b.did, &held.manifest_cid)],
            &budget(),
        ),
    )
    .await
    .expect("resumed");
    assert_eq!(fetched.chunks_resumed, already.len());
    assert_eq!(fetched.chunks_fetched, chunks.len() - already.len());
    let asked = b.tier.chunk_asks();
    assert_eq!(asked.len(), chunks.len() - already.len());
    assert!(
        asked.iter().all(|cid| !already.contains(cid)),
        "nothing held is asked again"
    );
    assert!(is_held(store.as_ref(), &held.manifest_cid).await);
    let dest = tempfile::tempdir().expect("dest");
    let mut flow = MemoryFlowStore::new();
    install_fetched(
        store.as_ref(),
        &mut flow,
        &fetched,
        &context(&t.c.did),
        &dest.path().join("bin"),
    )
    .await
    .expect("installed");
    assert_eq!(
        std::fs::read(dest.path().join("bin")).unwrap(),
        held.bytes.to_vec()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_trickling_holder_is_abandoned_on_the_total_deadline_and_the_chunk_re_raced() {
    let t = three();
    let key = key_for("closure-at-head");
    let held = held_artifact(&key, artifact_bytes(31, 6 * 1024 * 1024)).await;
    let output = held.manifest.record.output.clone();
    let endpoint = fetcher(&t.key_c).await;

    // A trickles past the stall bound; its chunks are re-raced to B, which finishes.
    let a = scripted(
        &held,
        Faults {
            delay_chunks: Some(Duration::from_secs(3)),
            ..Faults::default()
        },
    )
    .await;
    let b = scripted(&held, Faults::default()).await;
    let (_cdir, store) = payload();
    let started = Instant::now();
    let fetched = within(
        "the re-raced fetch",
        fetch_artifact(
            &endpoint,
            store.as_ref(),
            &key,
            &output,
            &[
                a.holder(&t.a.did, &held.manifest_cid),
                b.holder(&t.b.did, &held.manifest_cid),
            ],
            &FetchBudget {
                stall: Duration::from_millis(500),
                total: Duration::from_secs(20),
                ..FetchBudget::default()
            },
        ),
    )
    .await
    .expect("re-raced");
    assert!(
        started.elapsed() < Duration::from_secs(3),
        "not held up by the trickler"
    );
    let a_asked = a.tier.chunk_asks();
    assert!(!a_asked.is_empty(), "A was asked first");
    let b_asked = b.tier.chunk_asks();
    assert!(
        a_asked.iter().all(|cid| b_asked.contains(cid)),
        "every chunk A trickled was re-raced to B"
    );
    assert!(fetched.fetched_from.contains(&t.b.did));
    assert_eq!(fetched.chunks_fetched, distinct(&held.manifest).len());

    // Alone, a trickler that answers within the stall bound runs out the total deadline: a typed
    // gap naming what is held, never a partial install.
    let slow = scripted(
        &held,
        Faults {
            delay_chunks: Some(Duration::from_millis(700)),
            ..Faults::default()
        },
    )
    .await;
    let (_ddir, store) = payload();
    let gap = within(
        "the deadline",
        fetch_artifact(
            &endpoint,
            store.as_ref(),
            &key,
            &output,
            &[slow.holder(&t.a.did, &held.manifest_cid)],
            &FetchBudget {
                stall: Duration::from_secs(5),
                total: Duration::from_millis(1800),
                in_flight_per_holder: 1,
                ..FetchBudget::default()
            },
        ),
    )
    .await
    .expect_err("the deadline passes");
    match gap {
        FetchGap::DeadlineExceeded { fetched, total } => {
            assert_eq!(total, held.manifest.byte_len);
            assert!(fetched > 0 && fetched < total, "{fetched} of {total}");
        }
        other => panic!("expected DeadlineExceeded, got {other:?}"),
    }
    assert!(!is_held(store.as_ref(), &held.manifest_cid).await);
}

#[tokio::test(flavor = "multi_thread")]
async fn an_oversized_manifest_is_refused_before_any_chunk_moves() {
    let t = three();
    let key = key_for("closure-at-head");
    let held = held_artifact(&key, artifact_bytes(41, 3 * 1024 * 1024)).await;
    let a = scripted(&held, Faults::default()).await;
    let endpoint = fetcher(&t.key_c).await;
    let (_cdir, store) = payload();
    let gap = within(
        "the refusal",
        fetch_artifact(
            &endpoint,
            store.as_ref(),
            &key,
            &held.manifest.record.output,
            &[a.holder(&t.a.did, &held.manifest_cid)],
            &FetchBudget {
                manifest_cap: 64,
                ..budget()
            },
        ),
    )
    .await
    .expect_err("refused");
    match &gap {
        FetchGap::ManifestTooLarge { len, cap } => {
            assert_eq!(*cap, 64);
            assert!(*len > 64);
        }
        other => panic!("expected ManifestTooLarge, got {other:?}"),
    }
    assert!(gap.to_string().contains("no chunk moved"), "{gap}");
    assert!(a.tier.chunk_asks().is_empty(), "no chunk was asked");
    for chunk in &held.manifest.chunks {
        assert!(!is_held(store.as_ref(), &chunk.cid).await);
    }
    assert!(!is_held(store.as_ref(), &held.manifest_cid).await);
}

#[tokio::test(flavor = "multi_thread")]
async fn two_holders_with_different_outputs_for_one_key_is_a_mismatch_not_a_fetch() {
    let key = key_for("closure-at-head");
    let x = held_artifact(&key, artifact_bytes(51, 300 * 1024)).await;
    let y = held_artifact(&key, artifact_bytes(52, 300 * 1024)).await;
    let records = vec![
        SiblingRecord {
            device: "did:key:zDeviceA".into(),
            output: x.manifest.record.output.clone(),
            manifest: x.manifest_cid.clone(),
        },
        SiblingRecord {
            device: "did:key:zDeviceB".into(),
            output: y.manifest.record.output.clone(),
            manifest: y.manifest_cid.clone(),
        },
    ];
    let gap = resolve_holders(&records).expect_err("a mismatch");
    let mut expected = vec![
        x.manifest.record.output.clone(),
        y.manifest.record.output.clone(),
    ];
    expected.sort();
    assert_eq!(gap, FetchGap::Mismatch { outputs: expected });
    assert!(gap.to_string().contains("build here"), "{gap}");

    // A device's newer record supersedes its own older one; agreement is then one fetch.
    let agreeing = vec![
        records[0].clone(),
        records[1].clone(),
        SiblingRecord {
            device: "did:key:zDeviceB".into(),
            output: x.manifest.record.output.clone(),
            manifest: x.manifest_cid.clone(),
        },
    ];
    let (output, holders) = resolve_holders(&agreeing).expect("one output");
    assert_eq!(output, x.manifest.record.output);
    assert_eq!(holders.len(), 2);
    assert_eq!(resolve_holders(&[]), Err(FetchGap::NoRecord));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_manifest_keyed_for_another_closure_is_rejected() {
    let t = three();
    let key = key_for("closure-at-head");
    let other = key_for("another-closure");
    // The holder's manifest names the same output, under another key.
    let held = held_artifact(&other, artifact_bytes(61, 2 * 1024 * 1024)).await;
    let a = scripted(&held, Faults::default()).await;
    let endpoint = fetcher(&t.key_c).await;
    let (_cdir, store) = payload();
    let gap = within(
        "the rejection",
        fetch_artifact(
            &endpoint,
            store.as_ref(),
            &key,
            &held.manifest.record.output,
            &[a.holder(&t.a.did, &held.manifest_cid)],
            &budget(),
        ),
    )
    .await
    .expect_err("rejected");
    match &gap {
        FetchGap::ManifestRejected { reason } => {
            assert!(reason.contains("another closure"), "{reason}");
        }
        other => panic!("expected ManifestRejected, got {other:?}"),
    }
    assert!(
        a.tier.chunk_asks().is_empty(),
        "nothing moved for another closure"
    );
    assert!(!is_held(store.as_ref(), &held.manifest_cid).await);

    // A manifest that is not the one the crossed record named is rejected too.
    let right = held_artifact(&key, artifact_bytes(62, 2 * 1024 * 1024)).await;
    let b = scripted(&right, Faults::default()).await;
    let gap = within(
        "the rejection",
        fetch_artifact(
            &endpoint,
            store.as_ref(),
            &key,
            &right.manifest.record.output,
            &[b.holder(&t.b.did, &held.manifest_cid)],
            &budget(),
        ),
    )
    .await
    .expect_err("rejected");
    assert!(matches!(gap, FetchGap::ManifestRejected { .. }), "{gap:?}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_chunk_that_fails_its_cid_is_metered_as_wasted_and_re_fetched() {
    let t = three();
    let key = key_for("closure-at-head");
    let held = held_artifact(&key, artifact_bytes(71, 4 * 1024 * 1024)).await;
    let chunks = distinct(&held.manifest);
    let bad = chunks[0].clone();
    let a = scripted(
        &held,
        Faults {
            corrupt: Some(bad.cid.clone()),
            ..Faults::default()
        },
    )
    .await;
    let b = scripted(&held, Faults::default()).await;
    let endpoint = fetcher(&t.key_c).await;
    let (_cdir, store) = payload();
    let fetched = within(
        "the fetch",
        fetch_artifact(
            &endpoint,
            store.as_ref(),
            &key,
            &held.manifest.record.output,
            &[
                a.holder(&t.a.did, &held.manifest_cid),
                b.holder(&t.b.did, &held.manifest_cid),
            ],
            &budget(),
        ),
    )
    .await
    .expect("fetched");
    assert!(
        a.tier.chunk_asks().contains(&bad.cid),
        "A was asked the chunk it corrupts"
    );
    assert!(
        b.tier.chunk_asks().contains(&bad.cid),
        "the chunk was re-fetched from B"
    );
    assert_eq!(fetched.wasted, u64::from(bad.byte_len));
    let dest = tempfile::tempdir().expect("dest");
    let mut flow = MemoryFlowStore::new();
    install_fetched(
        store.as_ref(),
        &mut flow,
        &fetched,
        &context(&t.c.did),
        &dest.path().join("bin"),
    )
    .await
    .expect("installed");
    assert_eq!(
        std::fs::read(dest.path().join("bin")).unwrap(),
        held.bytes.to_vec()
    );
    let used = uses(&flow);
    assert!(
        slots(&used[0]).contains(&format!("wasted:{}", bad.byte_len).as_str()),
        "{:?}",
        used[0].classified_as
    );

    // With no other holder to ask, the corrupt chunk is a typed integrity gap.
    let (_ddir, store) = payload();
    let gap = within(
        "the gap",
        fetch_artifact(
            &endpoint,
            store.as_ref(),
            &key,
            &held.manifest.record.output,
            &[a.holder(&t.a.did, &held.manifest_cid)],
            &FetchBudget {
                in_flight_per_holder: 1,
                ..budget()
            },
        ),
    )
    .await
    .expect_err("no holder left");
    assert_eq!(
        gap,
        FetchGap::IntegrityFailed {
            cid: bad.cid.clone()
        }
    );
    assert!(
        !is_held(store.as_ref(), &bad.cid).await,
        "corrupt bytes are never stored"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn pure_iroh_mode_reports_a_typed_transport_gap() {
    let t = three();
    let key = key_for("closure-at-head");
    let held = held_artifact(&key, artifact_bytes(81, 600 * 1024)).await;
    let a = scripted(&held, Faults::default()).await;
    let endpoint = fetcher(&t.key_c).await;
    let (_cdir, store) = payload();
    let gap = fetch_artifact(
        &endpoint,
        store.as_ref(),
        &key,
        &held.manifest.record.output,
        &[a.holder(&t.a.did, &held.manifest_cid)],
        &FetchBudget {
            transport: FetchTransport::IrohBlobs,
            ..budget()
        },
    )
    .await
    .expect_err("no byte path");
    assert_eq!(
        gap,
        FetchGap::TransportCannotDeliver {
            mode: "iroh-blobs".into()
        }
    );
    assert_eq!(gap.kind(), "transport-cannot-deliver");
    assert_eq!(
        a.tier.connections.load(Ordering::SeqCst),
        0,
        "nothing was dialled"
    );
}

/// A manifest whose derivation record carries a D7 `ext`: the shape a newer writer may send.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RecordWithExt<'a> {
    key: &'a DerivationKey,
    output: &'a BlobCid,
    computed_by: &'a str,
    basis: &'a DerivationBasis,
    deterministic: bool,
    witness: &'a Witness,
    ext: BTreeMap<String, String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ManifestWithExt<'a> {
    epoch: u32,
    profile: &'a ByteProfile,
    record: RecordWithExt<'a>,
    byte_len: u64,
    chunks: &'a [TierBlobChunk],
}

#[tokio::test(flavor = "multi_thread")]
async fn a_manifest_whose_record_carries_ext_keeps_its_cid_through_the_fetch() {
    let t = three();
    let key = key_for("closure-at-head");
    let mut held = held_artifact(&key, artifact_bytes(91, 2 * 1024 * 1024)).await;
    let record = &held.manifest.record;
    let with_ext = ManifestWithExt {
        epoch: held.manifest.epoch,
        profile: &held.manifest.profile,
        record: RecordWithExt {
            key: &record.key,
            output: &record.output,
            computed_by: &record.computed_by,
            basis: &record.basis,
            deterministic: record.deterministic,
            witness: &record.witness,
            ext: BTreeMap::from([("signedBy".to_string(), "a-newer-writer".to_string())]),
        },
        byte_len: held.manifest.byte_len,
        chunks: &held.manifest.chunks,
    };
    let bytes = Bytes::from(serde_ipld_dagcbor::to_vec(&with_ext).expect("encode"));
    let cid = BlobCid::compute(&bytes);
    assert_ne!(
        cid, held.manifest_cid,
        "the extension is part of the manifest's bytes"
    );
    held.store.insert_blob(cid.clone(), bytes.clone()).await;
    held.manifest_cid = cid.clone();
    held.manifest_bytes = bytes.clone();
    let a = scripted(&held, Faults::default()).await;
    let endpoint = fetcher(&t.key_c).await;
    let (_cdir, store) = payload();
    let fetched = within(
        "the fetch",
        fetch_artifact(
            &endpoint,
            store.as_ref(),
            &key,
            &held.manifest.record.output,
            &[a.holder(&t.a.did, &cid)],
            &budget(),
        ),
    )
    .await
    .expect("fetched");
    assert_eq!(fetched.manifest_cid, cid);
    // Stored exactly as it verified: never re-encoded, so the extension and the CID survive.
    let stored = store
        .fetch_blob(&cid, eprfs_core::FetchPolicy::LocalOnly)
        .await
        .expect("stored manifest");
    assert_eq!(stored.bytes, bytes);
    let dest = tempfile::tempdir().expect("dest");
    let mut flow = MemoryFlowStore::new();
    install_fetched(
        store.as_ref(),
        &mut flow,
        &fetched,
        &context(&t.c.did),
        &dest.path().join("bin"),
    )
    .await
    .expect("installed");
    assert_eq!(
        std::fs::read(dest.path().join("bin")).unwrap(),
        held.bytes.to_vec()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_held_manifest_with_a_missing_chunk_records_an_unavailable_use() {
    let key = key_for("closure-at-head");
    let held = held_artifact(&key, artifact_bytes(101, 3 * 1024 * 1024)).await;
    let chunks = distinct(&held.manifest);
    let missing = chunks[1].clone();
    // A store holding the manifest and every chunk but one.
    let store = MemoryStorage::default();
    store
        .insert_blob(held.manifest_cid.clone(), held.manifest_bytes.clone())
        .await;
    for chunk in &chunks {
        if chunk.cid != missing.cid {
            let bytes = held
                .store
                .fetch_blob(&chunk.cid, eprfs_core::FetchPolicy::LocalOnly)
                .await
                .expect("chunk")
                .bytes;
            store.insert_blob(chunk.cid.clone(), bytes).await;
        }
    }
    let reason = format!("reason:chunk is missing: {}", missing.cid);

    // The local draw records one Use: unavailable, with its reason.
    let mut flow = MemoryFlowStore::new();
    let mut sink = |_: &TierBlobChunk, _: Bytes| Ok(());
    stream_local_blob(
        &store,
        &mut flow,
        &held.manifest_cid,
        &context("did:key:zDeviceC"),
        &mut sink,
    )
    .await
    .expect("recorded");
    let used = uses(&flow);
    assert_eq!(used.len(), 1);
    let used_slots = slots(&used[0]);
    assert!(used_slots.contains(&"draw:unavailable"), "{used_slots:?}");
    assert!(used_slots.contains(&reason.as_str()), "{used_slots:?}");

    // The install read of a fetch over it is the same unavailable Use, and installs nothing.
    let fetched = FetchedArtifact {
        manifest_cid: held.manifest_cid.clone(),
        manifest: held.manifest.clone(),
        manifest_from: "did:key:zDeviceA".into(),
        fetched_from: ["did:key:zDeviceA".to_string()].into_iter().collect(),
        chunks_fetched: chunks.len() - 1,
        chunks_resumed: 0,
        bytes_fetched: 0,
        wasted: 0,
        elapsed: Duration::ZERO,
    };
    let dest = tempfile::tempdir().expect("dest");
    let target = dest.path().join("bin");
    let mut flow = MemoryFlowStore::new();
    let gap = install_fetched(
        &store,
        &mut flow,
        &fetched,
        &context("did:key:zDeviceC"),
        &target,
    )
    .await
    .expect_err("not installed");
    assert_eq!(
        gap,
        FetchGap::IntegrityFailed {
            cid: held.manifest_cid.clone()
        }
    );
    assert!(
        !target.exists(),
        "nothing reaches the destination unverified"
    );
    assert_eq!(
        std::fs::read_dir(dest.path()).unwrap().count(),
        0,
        "no staging file is left"
    );
    let used = uses(&flow);
    assert_eq!(used.len(), 1);
    let used_slots = slots(&used[0]);
    for slot in [
        "draw:unavailable",
        "copy:self",
        "basis:sibling",
        reason.as_str(),
    ] {
        assert!(used_slots.contains(&slot), "{slot} in {used_slots:?}");
    }
}

// ---------------------------------------------------------------------------
// the daemon's queue (design §3.b.2)
// ---------------------------------------------------------------------------

/// One device's checkout (flow store and roster), peer state and payload store, for a daemon.
struct Side {
    checkout: support::Checkout,
    state: tempfile::TempDir,
    payload: tempfile::TempDir,
}

impl Side {
    fn new(rows: &[elohim_epr_rea::ParticipantRow]) -> Side {
        let checkout = support::Checkout::new();
        let roster = elohim_epr_rea::SidecarRoster::open(checkout.dir.path(), support::HANDLE)
            .expect("roster");
        for row in rows {
            roster.append(row.clone()).expect("row");
        }
        Side {
            checkout,
            state: tempfile::tempdir().expect("state"),
            payload: tempfile::tempdir().expect("payload"),
        }
    }

    fn store(&self) -> Arc<dyn EprfsStorage> {
        open_payload_store(&self.payload.path().join("payload")).expect("payload store")
    }

    async fn start(
        &self,
        t: &Three,
        me: &Device,
        key: PeerKey,
        port: u16,
        peers: Vec<NodeAddr>,
    ) -> eprfs_peer::PeerDaemon {
        let mut policy = support::policy();
        policy["berth"]["artifacts"] =
            serde_json::json!({ "serves": { "mesh-build": ["steward"] } });
        let roster = elohim_epr_rea::SidecarRoster::open(self.checkout.dir.path(), support::HANDLE)
            .expect("roster")
            .log_path()
            .to_path_buf();
        eprfs_peer::PeerDaemon::start(eprfs_peer::PeerConfig {
            key,
            device: me.did.clone(),
            chain_root: t.a.did.clone(),
            flow_root: self.checkout.dir.path().to_path_buf(),
            roster,
            state_dir: self.state.path().to_path_buf(),
            rules: Rules::from_policy(&policy).expect("rules"),
            policy: None,
            relay: None,
            listen: Some(std::net::SocketAddrV4::new(
                std::net::Ipv4Addr::LOCALHOST,
                port,
            )),
            peers,
            signer: Arc::new(me.signer()),
            verifier: verifier(&[&t.a, &t.b, &t.c]),
            log: Arc::new(|_line: &str| {}),
            payload_store: Some(self.store()),
        })
        .await
        .expect("daemon starts")
    }
}

fn free_port() -> u16 {
    std::net::UdpSocket::bind("127.0.0.1:0")
        .and_then(|s| s.local_addr())
        .map(|a| a.port())
        .expect("a free port")
}

#[tokio::test(flavor = "multi_thread")]
async fn a_queued_fetch_is_run_by_the_devices_daemon_and_answered() {
    let t = three();
    let key = key_for("closure-at-head");
    let bytes = artifact_bytes(111, 3 * 1024 * 1024);
    let output = BlobCid::compute_bytes(&bytes);

    // A holds the artifact in custody before its daemon starts (a daemon-down put).
    let a_side = Side::new(&t.rows);
    let a_store = a_side.store();
    put_method_record(a_store.as_ref(), &descriptor())
        .await
        .expect("method");
    let put = put_local_blob(
        a_store.as_ref(),
        &mut a_side.checkout.store(),
        record(&key, &bytes),
        bytes.clone(),
        &context(&t.a.did),
    )
    .await
    .expect("put");
    let a_port = free_port();
    let a_key = PeerKey::load_or_generate(t._dirs[0].path()).expect("key a");
    let a_daemon = a_side.start(&t, &t.a, a_key, a_port, Vec::new()).await;
    let a_addr =
        NodeAddr::new(a_daemon.node_id()).with_direct_addresses([std::net::SocketAddr::from((
            std::net::Ipv4Addr::LOCALHOST,
            a_port,
        ))]);

    let c_side = Side::new(&t.rows);
    let c_key = PeerKey::load_or_generate(t._dirs[2].path()).expect("key c");
    let c_daemon = c_side
        .start(&t, &t.c, c_key, free_port(), vec![a_addr])
        .await;

    let queue = eprfs_peer::fetch_queue_dir(c_side.state.path());
    std::fs::create_dir_all(&queue).expect("queue");
    let dest = c_side.payload.path().join("mesh-bin/elohim-storage");
    let request = eprfs_peer::FetchRequest {
        key: key.clone(),
        key_cid: key_cid(&key).expect("key cid"),
        output: output.clone(),
        holders: vec![
            eprfs_peer::RequestedHolder {
                device: t.a.did.clone(),
                manifest: put.manifest_cid.clone(),
            },
            // A device the roster does not admit is dropped by the daemon, never dialled.
            eprfs_peer::RequestedHolder {
                device: "did:key:zStranger".into(),
                manifest: put.manifest_cid.clone(),
            },
        ],
        dest: dest.clone(),
        scope: atom_cid(&"artifact-scope").expect("scope").to_string(),
        transport: FetchTransport::Tier,
        stall_s: 10,
        total_s: 30,
    };
    let done_path = eprfs_peer::fetch_done_path(&queue, &request.key_cid);
    // A's mount indexes on its own tick; until then A refuses, which is a gap, so ask again.
    let done = within("an installed answer", async {
        loop {
            eprfs_peer::write_queue_file(
                &eprfs_peer::fetch_request_path(&queue, &request.key_cid),
                &request,
            )
            .expect("queued");
            let answer = loop {
                if let Ok(bytes) = std::fs::read(&done_path) {
                    if let Ok(done) = serde_json::from_slice::<eprfs_peer::FetchDone>(&bytes) {
                        let _ = std::fs::remove_file(&done_path);
                        break done;
                    }
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            };
            match answer {
                eprfs_peer::FetchDone::Installed { .. } => break answer,
                eprfs_peer::FetchDone::Gap {
                    gap: FetchGap::Refused { .. },
                    ..
                } => {
                    tokio::time::sleep(Duration::from_millis(300)).await;
                }
                other => panic!("expected an installed answer, got {other:?}"),
            }
        }
    })
    .await;
    match done {
        eprfs_peer::FetchDone::Installed {
            manifest,
            fetched_from,
            ..
        } => {
            assert_eq!(manifest, put.manifest_cid);
            assert_eq!(fetched_from, vec![t.a.did.clone()]);
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(std::fs::read(&dest).expect("installed"), bytes.to_vec());
    assert!(
        !eprfs_peer::fetch_request_path(&queue, &key_cid(&key).unwrap()).exists(),
        "the request was claimed"
    );
    let installed: Vec<FlowEvent> = c_side
        .checkout
        .events()
        .into_iter()
        .map(|(_, event)| event)
        .filter(|event| {
            event.action == ReaVerb::Use && event.classified_as.iter().any(|s| s == "draw:hit")
        })
        .collect();
    assert_eq!(installed.len(), 1, "one install Use on the fetching device");
    assert_eq!(installed[0].resource, *output.as_cid());
    c_daemon.shutdown().await;
    a_daemon.shutdown().await;
}
