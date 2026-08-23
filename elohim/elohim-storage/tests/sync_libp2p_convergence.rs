//! Two-node libp2p convergence proof for the Automerge content-sync plane.
//!
//! `tests/sync_integration.rs` proves the merge logic in isolation (no network).
//! This test closes the zero-integration-coverage gap on the swarm leg: a doc
//! authored on node A via the REAL producer (`sync::projector::project_content_doc`)
//! converges into node B's DocStore over a REAL libp2p swarm
//! (`/elohim/storage-sync/1.0.0`), exercising
//! `ListDocumentsSince → DocumentList → SyncChanges → Changes → apply_changes`
//! (the digest-mismatch fallback path — a fresh node's empty corpus digest never
//! matches, so it always falls through to enumeration, same as before).
//!
//! The harness is a slim copy of `tests/harness/mod.rs`'s SwarmBuilder pattern
//! with the behaviour swapped to the production `SyncCodec`. The request/response
//! handlers MIRROR the production `handle_sync_request`/`handle_sync_response`
//! (p2p/mod.rs) but call the same public `SyncManager` methods the production
//! handlers call — the CRDT/merge logic itself is reused, not reimplemented.

use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use elohim_storage::db::content_diesel::{self, CreateContentInput, MinTrust};
use elohim_storage::db::context::AppContext;
use elohim_storage::db::models::Content;
use elohim_storage::db::DbPool;
use elohim_storage::p2p::sync_round;
use elohim_storage::p2p::{DocumentInfo, SyncCodec, SyncProtocol, SyncRequest, SyncResponse};
use elohim_storage::sync::projector::{project_content_doc, reverse_project_content_doc};
use elohim_storage::sync::{DocStore, DocStoreConfig, StreamTracker, SyncManager};
use elohim_storage::test_util::test_pool;
use futures::StreamExt;
use libp2p::{
    identify, identity, noise,
    request_response::{self, ProtocolSupport},
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux, Multiaddr, PeerId, SwarmBuilder,
};
use tempfile::TempDir;
use tokio::sync::{mpsc, oneshot};

const SYNC_NS: &str = "elohim";
const DOC_ID: &str = "node:edit-prop-1";

// ---------------------------------------------------------------------------
// Slim sync behaviour — request-response over the production SyncCodec + identify.
// ---------------------------------------------------------------------------

#[derive(NetworkBehaviour)]
struct SyncBehaviour {
    sync_protocol: request_response::Behaviour<SyncCodec>,
    identify: identify::Behaviour,
}

enum Command {
    Dial(Multiaddr, oneshot::Sender<Result<(), String>>),
    HasConnection(PeerId, oneshot::Sender<bool>),
    /// Drive one sync round: open with `sync_round::round_opener` (the sole
    /// legal construction site) to every connected peer, mirroring
    /// `P2PNode::initiate_sync_round`, p2p/mod.rs:6982.
    InitiateSync,
    /// Ring the doorbell for a locally-authored change, mirroring the
    /// `P2PCommand::AnnounceLocalChange` arm in p2p/mod.rs: load the doc's change
    /// bytes, bound them with `sync_round::bounded_announce_payload`, and fan out
    /// through `sync_round::announcements_for_local_change_with_data` (the sole
    /// legal construction site for announce requests).
    AnnounceLocalChange(String),
}

struct TestSyncNode {
    peer_id: PeerId,
    addr: Multiaddr,
    sync: Arc<SyncManager>,
    /// Optional SQL content projection (the serving DB). Set via `with_pool` for
    /// the B3 serve-path heal proof (node B needs a real `content` table for the
    /// reverse projector to heal into); `None` for the DocStore-only B2 proof.
    pool: Option<DbPool>,
    cmd_tx: mpsc::Sender<Command>,
    /// How many `SyncChanges` requests this node has SERVED. The announce tests
    /// assert eager delivery, and "converged" alone cannot tell an applied push
    /// from a pull that quietly did the work — this counter can.
    sync_changes_served: Arc<AtomicUsize>,
    _temp: TempDir,
}

impl TestSyncNode {
    async fn spawn(name: &str) -> TestSyncNode {
        let temp_dir = TempDir::new().unwrap();
        let doc_store = Arc::new(
            DocStore::new(DocStoreConfig {
                db_path: temp_dir.path().join(format!("{name}.sled")),
                ..Default::default()
            })
            .await
            .unwrap(),
        );
        let stream_tracker = Arc::new(StreamTracker::new());
        let sync = Arc::new(SyncManager::new(doc_store, stream_tracker));

        let local_key = identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());

        let mut swarm = SwarmBuilder::with_existing_identity(local_key)
            .with_tokio()
            .with_tcp(
                tcp::Config::default(),
                noise::Config::new,
                yamux::Config::default,
            )
            .expect("tcp transport")
            .with_behaviour(|key| {
                let sync_protocol = request_response::Behaviour::<SyncCodec>::new(
                    [(SyncProtocol, ProtocolSupport::Full)],
                    request_response::Config::default()
                        .with_request_timeout(Duration::from_secs(10)),
                );
                let identify_cfg =
                    identify::Config::new("/elohim/test-sync/1.0.0".into(), key.public());
                SyncBehaviour {
                    sync_protocol,
                    identify: identify::Behaviour::new(identify_cfg),
                }
            })
            .expect("behaviour build")
            .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(30)))
            .build();

        swarm
            .listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap())
            .expect("listen");

        let listen_addr = loop {
            match swarm.next().await.expect("swarm event") {
                SwarmEvent::NewListenAddr { address, .. } => break address,
                _ => continue,
            }
        };

        let (cmd_tx, cmd_rx) = mpsc::channel::<Command>(32);
        let sync_changes_served = Arc::new(AtomicUsize::new(0));
        tokio::spawn(run_driver(
            swarm,
            cmd_rx,
            sync.clone(),
            sync_changes_served.clone(),
        ));

        TestSyncNode {
            peer_id: local_peer_id,
            addr: listen_addr,
            sync,
            pool: None,
            cmd_tx,
            sync_changes_served,
            _temp: temp_dir,
        }
    }

    /// Attach an SQL content projection (the serving DB) to this node. The B3
    /// serve-path heal proof needs node B to carry a real `content` table so the
    /// reverse projector has somewhere to heal the converged `blob_hash` into.
    fn with_pool(mut self, pool: DbPool) -> Self {
        self.pool = Some(pool);
        self
    }

    fn peer_id(&self) -> PeerId {
        self.peer_id
    }

    fn listen_addr(&self) -> Multiaddr {
        self.addr.clone()
    }

    async fn dial(&self, addr: Multiaddr) -> Result<(), String> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(Command::Dial(addr, tx))
            .await
            .map_err(|e| e.to_string())?;
        rx.await.map_err(|_| "dial channel dropped".to_string())?
    }

    async fn wait_for_connection(&self, peer: &PeerId, timeout: Duration) -> bool {
        let start = tokio::time::Instant::now();
        while start.elapsed() < timeout {
            let (tx, rx) = oneshot::channel();
            if self
                .cmd_tx
                .send(Command::HasConnection(*peer, tx))
                .await
                .is_err()
            {
                return false;
            }
            if let Ok(true) = rx.await {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        false
    }

    async fn initiate_sync(&self) {
        let _ = self.cmd_tx.send(Command::InitiateSync).await;
    }

    /// Announce a locally-authored change to every connected peer. Deliberately
    /// NOT paired with `initiate_sync`: the announce tests assert that the
    /// doorbell alone propagates, with the 60s round never driven.
    async fn announce(&self, doc_id: &str) {
        let _ = self
            .cmd_tx
            .send(Command::AnnounceLocalChange(doc_id.to_string()))
            .await;
    }

    /// Poll this node's DocStore until `doc_id` has non-empty heads, or the
    /// timeout expires. Returns how long convergence took.
    async fn wait_for_doc(&self, doc_id: &str, timeout: Duration) -> Option<Duration> {
        let start = tokio::time::Instant::now();
        while start.elapsed() < timeout {
            if self
                .sync
                .get_heads(SYNC_NS, doc_id)
                .await
                .map(|h| !h.is_empty())
                .unwrap_or(false)
            {
                return Some(start.elapsed());
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        None
    }
}

// ---------------------------------------------------------------------------
// Driver task — owns the swarm; routes commands ↔ events.
// ---------------------------------------------------------------------------

async fn run_driver(
    mut swarm: libp2p::Swarm<SyncBehaviour>,
    mut cmd_rx: mpsc::Receiver<Command>,
    sync: Arc<SyncManager>,
    sync_changes_served: Arc<AtomicUsize>,
) {
    let mut connected: HashSet<PeerId> = HashSet::new();

    loop {
        tokio::select! {
            biased;

            Some(cmd) = cmd_rx.recv() => match cmd {
                Command::Dial(addr, reply) => {
                    let _ = reply.send(swarm.dial(addr).map_err(|e| e.to_string()));
                }
                Command::HasConnection(peer, reply) => {
                    let _ = reply.send(connected.contains(&peer));
                }
                Command::InitiateSync => {
                    let peers: Vec<PeerId> = swarm.connected_peers().cloned().collect();
                    if peers.is_empty() {
                        continue;
                    }
                    // Call the SAME construction site production uses
                    // (p2p/sync_round.rs is the only legal opener constructor) so
                    // this harness exercises the wire, not a hand-rolled mirror.
                    let local = match sync
                        .list_documents(SYNC_NS, None, 0, sync_round::SYNC_LIST_PAGE_LIMIT)
                        .await
                    {
                        Ok((docs, _total)) => sync_round::LocalCorpusState {
                            docs: docs
                                .into_iter()
                                .map(|d| sync_round::DocHead {
                                    doc_id: d.doc_id,
                                    heads: d.heads,
                                })
                                .collect(),
                        },
                        Err(_) => sync_round::LocalCorpusState::default(),
                    };
                    let req = sync_round::round_opener(SYNC_NS, &local);
                    for peer in peers {
                        swarm
                            .behaviour_mut()
                            .sync_protocol
                            .send_request(&peer, req.clone());
                    }
                }
                Command::AnnounceLocalChange(doc_id) => {
                    let peers: Vec<PeerId> = swarm.connected_peers().cloned().collect();
                    if peers.is_empty() {
                        continue;
                    }
                    // Mirrors the production command arm: the projector reports
                    // the new head as the change hash, and the payload is THAT
                    // ONE change (bounded), never the doc's whole history.
                    let change_hash = sync
                        .get_heads(SYNC_NS, &doc_id)
                        .await
                        .ok()
                        .and_then(|h| h.first().cloned())
                        .unwrap_or_default();
                    let change_data =
                        match sync.get_change_by_hash(SYNC_NS, &doc_id, &change_hash).await {
                            Ok(Some(bytes)) => sync_round::bounded_announce_payload(vec![bytes]),
                            _ => None,
                        };
                    let announcements = sync_round::announcements_for_local_change_with_data(
                        SYNC_NS,
                        &doc_id,
                        &change_hash,
                        &peers,
                        change_data,
                    );
                    for (peer, req) in announcements {
                        swarm.behaviour_mut().sync_protocol.send_request(&peer, req);
                    }
                }
            },

            event = swarm.next() => {
                let Some(event) = event else { break; };
                match event {
                    SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                        connected.insert(peer_id);
                    }
                    SwarmEvent::ConnectionClosed { peer_id, .. } => {
                        connected.remove(&peer_id);
                    }
                    SwarmEvent::Behaviour(SyncBehaviourEvent::SyncProtocol(rr)) => {
                        handle_sync_rr(&mut swarm, &sync, &sync_changes_served, rr).await;
                    }
                    _ => {}
                }
            }
        }
    }
}

async fn handle_sync_rr(
    swarm: &mut libp2p::Swarm<SyncBehaviour>,
    sync: &SyncManager,
    sync_changes_served: &AtomicUsize,
    event: request_response::Event<SyncRequest, SyncResponse>,
) {
    if let request_response::Event::Message { peer, message } = event {
        match message {
            request_response::Message::Request {
                request, channel, ..
            } => {
                let response =
                    handle_request(swarm, sync, sync_changes_served, peer, request).await;
                let _ = swarm
                    .behaviour_mut()
                    .sync_protocol
                    .send_response(channel, response);
            }
            request_response::Message::Response { response, .. } => {
                handle_response(swarm, sync, peer, response).await;
            }
        }
    }
}

/// Mirrors `P2PNode::handle_sync_request` (p2p/mod.rs:6303) for the two variants
/// the round uses; reuses the public `SyncManager` methods the production handler
/// calls.
async fn handle_request(
    swarm: &mut libp2p::Swarm<SyncBehaviour>,
    sync: &SyncManager,
    sync_changes_served: &AtomicUsize,
    peer: PeerId,
    request: SyncRequest,
) -> SyncResponse {
    match request {
        // Mirrors P2PNode's AnnounceChange arm: bytes within the bound are
        // applied through the SAME `apply_changes` a pulled change takes; an
        // oversized or absent payload opens a `SyncChanges` pull back at the
        // announcer (`P2PNode::pull_announced_doc`).
        SyncRequest::AnnounceChange {
            h_app_id,
            doc_id,
            change_hash,
            change_data,
        } => {
            let data = change_data.filter(|d| d.len() <= sync_round::MAX_ANNOUNCE_PAYLOAD_BYTES);
            match data {
                Some(data) => match sync.apply_changes(&h_app_id, &doc_id, vec![data]).await {
                    // Landing check, mirroring production: a change whose deps we
                    // lack is QUEUED, not applied, and `apply_changes` still
                    // returns Ok — so ask whether the doc actually holds it.
                    Ok(_) => {
                        let landed = sync
                            .get_change_by_hash(&h_app_id, &doc_id, &change_hash)
                            .await
                            .unwrap_or(None)
                            .is_some();
                        if landed {
                            SyncResponse::ChangeAck {
                                h_app_id,
                                doc_id,
                                was_new: true,
                            }
                        } else {
                            let have_heads =
                                sync.get_heads(&h_app_id, &doc_id).await.unwrap_or_default();
                            let req = SyncRequest::SyncChanges {
                                h_app_id: h_app_id.clone(),
                                doc_id: doc_id.clone(),
                                have_heads,
                                bloom_filter: None,
                            };
                            swarm.behaviour_mut().sync_protocol.send_request(&peer, req);
                            SyncResponse::ChangeAck {
                                h_app_id,
                                doc_id,
                                was_new: false,
                            }
                        }
                    }
                    Err(e) => SyncResponse::Error {
                        message: format!("apply_changes: {e}"),
                    },
                },
                None => {
                    let have_heads = sync.get_heads(&h_app_id, &doc_id).await.unwrap_or_default();
                    let req = SyncRequest::SyncChanges {
                        h_app_id: h_app_id.clone(),
                        doc_id: doc_id.clone(),
                        have_heads,
                        bloom_filter: None,
                    };
                    swarm.behaviour_mut().sync_protocol.send_request(&peer, req);
                    SyncResponse::ChangeAck {
                        h_app_id,
                        doc_id,
                        was_new: false,
                    }
                }
            }
        }
        SyncRequest::ListDocuments {
            h_app_id,
            prefix,
            offset,
            limit,
        } => match sync
            .list_documents(&h_app_id, prefix.as_deref(), offset, limit)
            .await
        {
            Ok((docs, total)) => {
                let documents: Vec<DocumentInfo> = docs
                    .into_iter()
                    .map(|d| DocumentInfo {
                        doc_id: d.doc_id,
                        doc_type: d.doc_type,
                        change_count: d.change_count,
                        last_modified: d.last_modified,
                        heads: d.heads,
                    })
                    .collect();
                let has_more = (offset as u64 + documents.len() as u64) < total;
                SyncResponse::DocumentList {
                    h_app_id,
                    documents,
                    total,
                    has_more,
                }
            }
            Err(e) => SyncResponse::Error {
                message: format!("list_documents: {e}"),
            },
        },
        SyncRequest::SyncChanges {
            h_app_id,
            doc_id,
            have_heads,
            ..
        } => {
            sync_changes_served.fetch_add(1, Ordering::SeqCst);
            let served = sync
                .get_changes_since(&h_app_id, &doc_id, &have_heads)
                .await;
            match served {
                Ok((changes, new_heads)) => SyncResponse::Changes {
                    h_app_id,
                    doc_id,
                    changes,
                    has_more: false,
                    new_heads,
                },
                Err(e) => SyncResponse::Error {
                    message: format!("get_changes_since: {e}"),
                },
            }
        }
        // Mirrors P2PNode's ListDocumentsSince arm (p2p/mod.rs:6656): equal
        // digests answer InSync and enumerate nothing; divergent digests fall
        // through to exactly the ListDocuments behaviour above.
        SyncRequest::ListDocumentsSince {
            h_app_id,
            prefix,
            corpus_digest,
            limit,
        } => match sync
            .list_documents(
                &h_app_id,
                prefix.as_deref(),
                0,
                sync_round::SYNC_LIST_PAGE_LIMIT,
            )
            .await
        {
            Ok((docs, total)) => {
                let local = sync_round::LocalCorpusState {
                    docs: docs
                        .iter()
                        .map(|d| sync_round::DocHead {
                            doc_id: d.doc_id.clone(),
                            heads: d.heads.clone(),
                        })
                        .collect(),
                };
                let ours = sync_round::corpus_digest(&local);
                if ours == corpus_digest {
                    SyncResponse::InSync {
                        h_app_id,
                        corpus_digest: ours,
                    }
                } else {
                    let documents: Vec<DocumentInfo> = docs
                        .into_iter()
                        .take(limit as usize)
                        .map(|d| DocumentInfo {
                            doc_id: d.doc_id,
                            doc_type: d.doc_type,
                            change_count: d.change_count,
                            last_modified: d.last_modified,
                            heads: d.heads,
                        })
                        .collect();
                    let has_more = (documents.len() as u64) < total;
                    SyncResponse::DocumentList {
                        h_app_id,
                        documents,
                        total,
                        has_more,
                    }
                }
            }
            Err(e) => SyncResponse::Error {
                message: format!("list_documents (digest compare): {e}"),
            },
        },
        _ => SyncResponse::Error {
            message: "unhandled sync request in test harness".to_string(),
        },
    }
}

/// Mirrors `P2PNode::handle_sync_response` (p2p/mod.rs:6483) for DocumentList +
/// Changes; reuses the public `SyncManager` methods.
async fn handle_response(
    swarm: &mut libp2p::Swarm<SyncBehaviour>,
    sync: &SyncManager,
    peer: PeerId,
    response: SyncResponse,
) {
    match response {
        SyncResponse::DocumentList {
            h_app_id,
            documents,
            ..
        } => {
            for remote in &documents {
                let local_heads = sync
                    .get_heads(&h_app_id, &remote.doc_id)
                    .await
                    .unwrap_or_default();
                if local_heads != remote.heads {
                    let req = SyncRequest::SyncChanges {
                        h_app_id: h_app_id.clone(),
                        doc_id: remote.doc_id.clone(),
                        have_heads: local_heads,
                        bloom_filter: None,
                    };
                    swarm.behaviour_mut().sync_protocol.send_request(&peer, req);
                }
            }
        }
        SyncResponse::Changes {
            h_app_id,
            doc_id,
            changes,
            ..
        } => {
            if changes.is_empty() {
                return;
            }
            let _ = sync.apply_changes(&h_app_id, &doc_id, changes).await;
        }
        // Digest-equal steady state: nothing enumerated, nothing to apply.
        SyncResponse::InSync { .. } => {}
        _ => {}
    }
}

fn sample_content(id: &str, title: &str) -> Content {
    Content {
        id: id.to_string(),
        h_app_id: "lamad".to_string(),
        title: title.to_string(),
        description: None,
        content_type: "concept".to_string(),
        content_format: "markdown".to_string(),
        blob_hash: None,
        blob_cid: None,
        content_size_bytes: None,
        metadata_json: Some("{}".to_string()),
        // Broadcast-tier: the sync plane's fail-closed reach gate excludes
        // non-broadcast (and unknown-vocabulary) reach values by design.
        reach: "commons".to_string(),
        validation_status: "valid".to_string(),
        created_by: None,
        created_at: "2026-06-27T00:00:00Z".to_string(),
        updated_at: "2026-06-27T00:00:00Z".to_string(),
        content_body: Some("hello".to_string()),
        dht_anchor_hash: None,
        p2p_published_at: None,
        server_blob_hash: None,
        crdt_converged_at: None,
        declared_head_action_hash: None,
        declared_head_at: None,
        canonical_declared_at: None,
        canonical_earned: None,
        dht_anchor_state: None,
        dht_anchor_checked_at: None,
    }
}

#[tokio::test]
async fn doc_authored_on_a_converges_to_b() {
    let node_a = TestSyncNode::spawn("a").await;
    let node_b = TestSyncNode::spawn("b").await;

    node_b.dial(node_a.listen_addr()).await.expect("B dials A");
    assert!(
        node_b
            .wait_for_connection(&node_a.peer_id(), Duration::from_secs(10))
            .await,
        "node B never connected to node A"
    );

    // Author on A via the REAL producer path (exercises G1 end-to-end).
    let content = sample_content("edit-prop-1", "Edited v2");
    project_content_doc(&node_a.sync, &content)
        .await
        .expect("project content doc on A");

    // Precondition: A has the doc, B does not yet.
    assert!(
        !node_a
            .sync
            .get_heads(SYNC_NS, DOC_ID)
            .await
            .unwrap()
            .is_empty(),
        "node A must hold the authored doc"
    );
    assert!(
        node_b
            .sync
            .get_heads(SYNC_NS, DOC_ID)
            .await
            .unwrap()
            .is_empty(),
        "node B must NOT hold the doc before sync"
    );

    // Drive the round directly (don't wait on a 60s timer): B initiates
    // ListDocuments rounds until it converges, polling its DocStore.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    let mut converged = false;
    while tokio::time::Instant::now() < deadline {
        node_b.initiate_sync().await;
        tokio::time::sleep(Duration::from_millis(250)).await;
        if node_b
            .sync
            .get_heads(SYNC_NS, DOC_ID)
            .await
            .map(|h| !h.is_empty())
            .unwrap_or(false)
        {
            converged = true;
            break;
        }
    }
    assert!(converged, "node B did not converge {DOC_ID} within 30s");

    // The converged value must match what A authored.
    let title = node_b
        .sync
        .get_doc_field(SYNC_NS, DOC_ID, "title")
        .await
        .unwrap();
    assert_eq!(title, "Edited v2", "converged title must match A");

    // And the heads must match (full CRDT convergence, not partial).
    let heads_a = node_a.sync.get_heads(SYNC_NS, DOC_ID).await.unwrap();
    let heads_b = node_b.sync.get_heads(SYNC_NS, DOC_ID).await.unwrap();
    assert_eq!(heads_a, heads_b, "heads must converge between A and B");
}

/// Dial B→A, wait for the connection, then drive `ListDocuments` rounds from B
/// until `doc_id` converges (its heads become non-empty) or 30s elapses. Uses the
/// harness's existing poll-until-converged pattern — no arbitrary sleeps. Because
/// `project_content_doc` writes all fields in ONE Automerge change, non-empty
/// heads on B mean the whole doc (every field) landed — never a partial field set.
async fn connect_and_converge(node_a: &TestSyncNode, node_b: &TestSyncNode, doc_id: &str) {
    node_b.dial(node_a.listen_addr()).await.expect("B dials A");
    assert!(
        node_b
            .wait_for_connection(&node_a.peer_id(), Duration::from_secs(10))
            .await,
        "node B never connected to node A"
    );

    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    let mut converged = false;
    while tokio::time::Instant::now() < deadline {
        node_b.initiate_sync().await;
        tokio::time::sleep(Duration::from_millis(250)).await;
        if node_b
            .sync
            .get_heads(SYNC_NS, doc_id)
            .await
            .map(|h| !h.is_empty())
            .unwrap_or(false)
        {
            converged = true;
            break;
        }
    }
    assert!(converged, "node B did not converge {doc_id} within 30s");
}

/// B2 — the DocStore convergence carries the REAL `blobHash`, zero notary.
///
/// A authors a content doc via the real producer (`project_content_doc`) carrying
/// a non-empty `blob_hash`; it converges to B over a real libp2p swarm; B's
/// DocStore serves back the exact hash. RED if convergence drops or empties the
/// serving-critical `blobHash` field. No conductor anywhere in the harness.
#[tokio::test]
async fn converges_real_blobhash_zero_notary() {
    const REAL_HASH: &str =
        "sha256-1111111111111111111111111111111111111111111111111111111111111111";

    let node_a = TestSyncNode::spawn("a").await;
    let node_b = TestSyncNode::spawn("b").await;

    // Author on A a doc carrying a REAL, non-empty blob_hash.
    let mut content = sample_content("edit-prop-1", "landing");
    content.blob_hash = Some(REAL_HASH.to_string());
    project_content_doc(&node_a.sync, &content)
        .await
        .expect("project real-blob doc on A");

    // Precondition: B holds nothing yet.
    assert!(
        node_b
            .sync
            .get_heads(SYNC_NS, DOC_ID)
            .await
            .unwrap()
            .is_empty(),
        "node B must NOT hold the doc before sync"
    );

    connect_and_converge(&node_a, &node_b, DOC_ID).await;

    // The serving-critical blobHash converged intact (not dropped/emptied).
    let converged = node_b
        .sync
        .get_doc_field(SYNC_NS, DOC_ID, "blobHash")
        .await
        .unwrap();
    assert_eq!(
        converged, REAL_HASH,
        "converged blobHash must equal A's real hash"
    );

    // Full CRDT convergence, not partial.
    let heads_a = node_a.sync.get_heads(SYNC_NS, DOC_ID).await.unwrap();
    let heads_b = node_b.sync.get_heads(SYNC_NS, DOC_ID).await.unwrap();
    assert_eq!(heads_a, heads_b, "heads must converge between A and B");
}

/// B3 — the serve-path heal proven end-to-end over real libp2p, zero notary.
///
/// The elohim.host 404 as a red→green: node B's SQL projection carries a content
/// row with `blob_hash = NULL` (the 404 case — SERVING reads SQL, not the
/// DocStore). A authors the real-blob doc; it converges to B; the reverse
/// projector heals B's SQL row at the amber tier so `get_content(Amber)` returns
/// the converged hash — with `dht_anchor_hash` still NULL (no notary). RED without
/// the reverse projector. This is the whole thesis proven with zero conductor.
#[tokio::test]
async fn converges_and_serves_zero_notary() {
    const REAL_HASH: &str =
        "sha256-2222222222222222222222222222222222222222222222222222222222222222";

    let node_a = TestSyncNode::spawn("a").await;
    let node_b = TestSyncNode::spawn("b").await.with_pool(test_pool());
    let ctx = AppContext::default_lamad();

    // Seed node B's SQL projection with the elohim.host case: a content row whose
    // blob_hash is NULL (so /blob serving 404s until a peer heals it), never
    // notarized (dht_anchor_hash NULL).
    {
        let pool = node_b.pool.as_ref().expect("node B carries a pool");
        let mut conn = pool.get().unwrap();
        content_diesel::create_content(
            &mut conn,
            &ctx,
            CreateContentInput {
                id: "edit-prop-1".to_string(),
                title: "landing".to_string(),
                description: None,
                content_type: "concept".to_string(),
                content_format: "markdown".to_string(),
                blob_hash: None, // the 404 case
                blob_cid: None,
                content_size_bytes: None,
                metadata_json: None,
                // Broadcast-tier: the sync plane's fail-closed reach gate excludes
                // non-broadcast (and unknown-vocabulary) reach values by design.
                reach: "commons".to_string(),
                created_by: None,
                tags: vec![],
                content_body: Some("b".to_string()),
                dht_anchor_hash: None, // never notarized
            },
        )
        .unwrap();
    }

    // Author the real-blob doc on A and converge to B over real libp2p.
    let mut content = sample_content("edit-prop-1", "landing");
    content.blob_hash = Some(REAL_HASH.to_string());
    project_content_doc(&node_a.sync, &content)
        .await
        .expect("project real-blob doc on A");

    connect_and_converge(&node_a, &node_b, DOC_ID).await;

    // Serve-path heal: reverse-project B's converged DocStore into its SQL row.
    let pool = node_b.pool.as_ref().unwrap();
    let healed = reverse_project_content_doc(&node_b.sync, pool, DOC_ID)
        .await
        .expect("reverse-project must not error");
    assert!(
        healed,
        "a real converged blob_hash into a NULL local row must heal"
    );

    // The serving read (amber tier) now returns the converged hash — the 404 fixed
    // with ZERO notary (dht_anchor_hash still NULL).
    let mut conn = pool.get().unwrap();
    let row = content_diesel::get_content(&mut conn, &ctx, "edit-prop-1", MinTrust::Amber)
        .unwrap()
        .expect("row served at the amber tier");
    assert_eq!(
        row.blob_hash.as_deref(),
        Some(REAL_HASH),
        "served blob_hash must be the converged hash"
    );
    assert!(
        row.crdt_converged_at.is_some(),
        "healed row must be amber-stamped"
    );
    assert!(
        row.dht_anchor_hash.is_none(),
        "heal must never notarize (zero notary)"
    );
}

/// Dial B→A and wait for the connection, WITHOUT driving a sync round. The
/// announce tests must prove the doorbell propagates on its own — if the round
/// runs, convergence proves nothing about the push path.
async fn connect_only(node_a: &TestSyncNode, node_b: &TestSyncNode) {
    node_b.dial(node_a.listen_addr()).await.expect("B dials A");
    assert!(
        node_b
            .wait_for_connection(&node_a.peer_id(), Duration::from_secs(10))
            .await,
        "node B never connected to node A"
    );
    // BOTH directions: the announce fans out to A's `connected_peers()`, and A
    // registers the inbound connection slightly after B registers the outbound
    // one. Announcing on B's view alone races that gap — a single un-retried
    // doorbell fired into an empty peer set is simply lost (bounded and lossy by
    // design), and the test would fail for a reason that is not the cure.
    assert!(
        node_a
            .wait_for_connection(&node_b.peer_id(), Duration::from_secs(10))
            .await,
        "node A never registered the inbound connection from B"
    );
}

/// The doorbell PROPAGATES at first contact — a locally-authored change reaches
/// a peer that holds no history for the doc, on the announce alone, with no sync
/// round anywhere in the test.
///
/// RED before the cure: `announcements_for_local_change` sent `change_data: None`
/// and the receiver answered a bare `ChangeAck`, so nothing moved until the next
/// 60s round (measured cross-peer arrival on the live 3-peer mesh: ~24s). Node B
/// here never calls `initiate_sync`, so a pass means the push path carried it.
/// A doc's FIRST change has no dependencies, so B applies it holding no history
/// at all; `announce_of_a_change_whose_deps_the_peer_lacks_falls_back_to_a_pull`
/// covers the case where it cannot.
#[tokio::test]
async fn announce_carries_the_change_to_a_connected_peer() {
    let node_a = TestSyncNode::spawn("a").await;
    let node_b = TestSyncNode::spawn("b").await;

    connect_only(&node_a, &node_b).await;

    let content = sample_content("edit-prop-1", "Announced v1");
    project_content_doc(&node_a.sync, &content)
        .await
        .expect("project content doc on A");
    assert!(
        node_b
            .sync
            .get_heads(SYNC_NS, DOC_ID)
            .await
            .unwrap()
            .is_empty(),
        "node B must NOT hold the doc before the announce"
    );

    // The payload must actually ride the wire — a doorbell here would make the
    // test pass only via the receiver's pull, which a later test covers.
    let head = node_a.sync.get_heads(SYNC_NS, DOC_ID).await.unwrap()[0].clone();
    let change = node_a
        .sync
        .get_change_by_hash(SYNC_NS, DOC_ID, &head)
        .await
        .unwrap()
        .expect("A must hold the change it is announcing");
    assert!(
        sync_round::bounded_announce_payload(vec![change]).is_some(),
        "a projected content doc's change must fit the announce bound"
    );

    node_a.announce(DOC_ID).await;

    let elapsed = node_b
        .wait_for_doc(DOC_ID, Duration::from_secs(10))
        .await
        .expect("node B did not converge on the announce alone within 10s — the doorbell is inert");
    assert!(
        elapsed < Duration::from_secs(10),
        "announce propagation must be prompt, took {elapsed:?}"
    );
    // PURE PUSH, even at first contact: this doc has exactly one change, and a
    // doc's first change has no dependencies, so B can apply it holding no
    // history at all. Nothing was pulled.
    assert_eq!(
        node_a.sync_changes_served.load(Ordering::SeqCst),
        0,
        "the pushed bytes must have converged B on their own, with no pull"
    );

    let title = node_b
        .sync
        .get_doc_field(SYNC_NS, DOC_ID, "title")
        .await
        .unwrap();
    assert_eq!(title, "Announced v1", "the pushed value must be A's");
    let heads_a = node_a.sync.get_heads(SYNC_NS, DOC_ID).await.unwrap();
    let heads_b = node_b.sync.get_heads(SYNC_NS, DOC_ID).await.unwrap();
    assert_eq!(heads_a, heads_b, "heads must converge between A and B");
}

/// An OVERSIZED change still propagates eagerly — as a doorbell the receiver
/// answers with a pull, not as a fan-out of `bytes x peers`.
///
/// The bound exists so one large local edit cannot amplify across the mesh; the
/// pull is what keeps propagation from silently regressing to the 60s round for
/// exactly the changes that matter most. Node B never calls `initiate_sync`.
#[tokio::test]
async fn an_oversized_announce_degrades_to_a_pull_and_still_converges() {
    const BIG_DOC_ID: &str = "node:big-doc-1";

    let node_a = TestSyncNode::spawn("a").await;
    let node_b = TestSyncNode::spawn("b").await;

    connect_only(&node_a, &node_b).await;

    let mut content = sample_content("big-doc-1", "Oversized");
    content.content_body = Some("x".repeat(sync_round::MAX_ANNOUNCE_PAYLOAD_BYTES + 4096));
    project_content_doc(&node_a.sync, &content)
        .await
        .expect("project oversized doc on A");

    // Precondition: this change genuinely exceeds the bound, so the announce
    // MUST be metadata-only and the receiver's pull is the only eager path.
    let (changes, _) = node_a
        .sync
        .get_changes_since(SYNC_NS, BIG_DOC_ID, &[])
        .await
        .unwrap();
    assert!(
        sync_round::bounded_announce_payload(changes).is_none(),
        "the fixture must exceed MAX_ANNOUNCE_PAYLOAD_BYTES for this test to mean anything"
    );

    node_a.announce(BIG_DOC_ID).await;

    node_b
        .wait_for_doc(BIG_DOC_ID, Duration::from_secs(15))
        .await
        .expect("node B did not converge — a metadata-only announce must trigger a pull");

    let title = node_b
        .sync
        .get_doc_field(SYNC_NS, BIG_DOC_ID, "title")
        .await
        .unwrap();
    assert_eq!(title, "Oversized");
    let heads_a = node_a.sync.get_heads(SYNC_NS, BIG_DOC_ID).await.unwrap();
    let heads_b = node_b.sync.get_heads(SYNC_NS, BIG_DOC_ID).await.unwrap();
    assert_eq!(heads_a, heads_b, "heads must converge between A and B");
}

/// The steady state the bound is FOR: a mature doc with real history announces
/// only the NEW change, and a peer that already holds the history applies it
/// eagerly — no pull, no round.
///
/// RED before this cure (payload was `get_changes_since(.., &[])` = every change
/// since genesis): the payload grows with the doc, so a doc with history
/// overflows `MAX_ANNOUNCE_PAYLOAD_BYTES` on every local edit and every announce
/// silently degrades to pull-on-announce (+1 RTT, plus a full-doc serialization
/// burned on the sender to discover it). This test pins both halves — the
/// payload is a small delta, and no `SyncChanges` is served.
#[tokio::test]
async fn announce_of_a_mature_doc_lands_eagerly_without_a_pull() {
    let node_a = TestSyncNode::spawn("a").await;
    let node_b = TestSyncNode::spawn("b").await;

    connect_only(&node_a, &node_b).await;

    // Give the doc real history: several distinct authored versions, each a
    // separate Automerge change with a sizeable body.
    let filler = "y".repeat(8 * 1024);
    for v in 1..=6 {
        let mut content = sample_content("edit-prop-1", &format!("Version {v}"));
        content.content_body = Some(format!("{filler}{v}"));
        project_content_doc(&node_a.sync, &content)
            .await
            .expect("project version on A");
    }

    // Bring B up to that history the ordinary way (one round), then take the
    // pull counter to zero: everything after this point must be push-only.
    connect_and_converge(&node_a, &node_b, DOC_ID).await;
    node_a.sync_changes_served.store(0, Ordering::SeqCst);

    // One more locally-authored change on the now-mature doc.
    let mut content = sample_content("edit-prop-1", "Version 7");
    content.content_body = Some(format!("{filler}7"));
    project_content_doc(&node_a.sync, &content)
        .await
        .expect("project version 7 on A");

    // The announced payload is ONE change, not the doc. Pin the ratio, not just
    // the bound: a full-history payload would be several times larger AND (with
    // this fixture) over the bound entirely.
    let head = node_a.sync.get_heads(SYNC_NS, DOC_ID).await.unwrap()[0].clone();
    let one_change = node_a
        .sync
        .get_change_by_hash(SYNC_NS, DOC_ID, &head)
        .await
        .unwrap()
        .expect("A holds the change it is announcing");
    let (whole_doc, _) = node_a
        .sync
        .get_changes_since(SYNC_NS, DOC_ID, &[])
        .await
        .unwrap();
    let whole_doc_len = whole_doc.iter().map(|c| c.len()).sum::<usize>();
    assert!(
        one_change.len() < whole_doc_len / 2,
        "the announced payload must be a delta ({} bytes), not the doc ({whole_doc_len} bytes)",
        one_change.len()
    );
    assert!(
        sync_round::bounded_announce_payload(vec![one_change]).is_some(),
        "a single change on a mature doc must still fit the announce bound"
    );

    node_a.announce(DOC_ID).await;

    // B converges on the pushed bytes alone.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    loop {
        if node_b
            .sync
            .get_doc_field(SYNC_NS, DOC_ID, "title")
            .await
            .map(|t| t == "Version 7")
            .unwrap_or(false)
        {
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "node B never received Version 7 from the announce"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    assert_eq!(
        node_a.sync_changes_served.load(Ordering::SeqCst),
        0,
        "the eager push must have landed on its own — a pull here means the payload \
         was not the announced change"
    );
    let heads_a = node_a.sync.get_heads(SYNC_NS, DOC_ID).await.unwrap();
    let heads_b = node_b.sync.get_heads(SYNC_NS, DOC_ID).await.unwrap();
    assert_eq!(heads_a, heads_b, "heads must converge between A and B");
}

/// The landing check earns its keep: an announced change whose DEPENDENCIES the
/// receiver lacks is queued by Automerge, not applied — `apply_changes` still
/// returns `Ok` and the doc is untouched. Treating that as converged is silent
/// data loss; the receiver must notice and pull.
///
/// A authors six versions while B holds nothing, so the announced (sixth) change
/// depends on five changes B has never seen. B never calls `initiate_sync`.
#[tokio::test]
async fn announce_of_a_change_whose_deps_the_peer_lacks_falls_back_to_a_pull() {
    let node_a = TestSyncNode::spawn("a").await;
    let node_b = TestSyncNode::spawn("b").await;

    connect_only(&node_a, &node_b).await;

    let mut early_head = String::new();
    for v in 1..=6 {
        let content = sample_content("edit-prop-1", &format!("Version {v}"));
        project_content_doc(&node_a.sync, &content)
            .await
            .expect("project version on A");
        if v == 1 {
            early_head = node_a.sync.get_heads(SYNC_NS, DOC_ID).await.unwrap()[0].clone();
        }
    }

    assert!(
        node_b
            .sync
            .get_heads(SYNC_NS, DOC_ID)
            .await
            .unwrap()
            .is_empty(),
        "node B must hold no history for this doc"
    );

    node_a.announce(DOC_ID).await;

    node_b
        .wait_for_doc(DOC_ID, Duration::from_secs(15))
        .await
        .expect("node B never converged — a queued (unapplied) change was mistaken for a landing");

    assert!(
        node_a.sync_changes_served.load(Ordering::SeqCst) >= 1,
        "B could not have applied a change whose deps it lacks — it must have pulled"
    );
    // The pull carried the DEPENDENCIES, not just the head: B holds the doc's
    // first change too. A head without its history is the shape that would let a
    // corpus digest match while the doc is quietly incomplete.
    assert!(
        node_b
            .sync
            .get_change_by_hash(SYNC_NS, DOC_ID, &early_head)
            .await
            .unwrap()
            .is_some(),
        "B must hold the doc's earliest change, not just the announced head"
    );
    let title = node_b
        .sync
        .get_doc_field(SYNC_NS, DOC_ID, "title")
        .await
        .unwrap();
    assert_eq!(title, "Version 6");
    let heads_a = node_a.sync.get_heads(SYNC_NS, DOC_ID).await.unwrap();
    let heads_b = node_b.sync.get_heads(SYNC_NS, DOC_ID).await.unwrap();
    assert_eq!(heads_a, heads_b, "heads must converge between A and B");
}
