//! In-process two-peer swarm harness for /elohim/epr-atom/1.0.0 and
//! /elohim/identity/handshake/1.0.0.
//!
//! Each `TestNode` owns:
//!   - a libp2p Swarm with a minimal NetworkBehaviour (request-response + identify
//!     + identity-handshake)
//!   - its own in-memory DbPool with migrations applied
//!   - a StubIdentityMap so tests can register peer→pubkey mappings
//!   - an AgentKeypair + signer_cid for authoring signed atoms
//!   - a spawned tokio task running the swarm event loop + command dispatcher
//!
//! Incoming requests are handled in-harness (the full P2PNode is NOT used)
//! but apply the real production `reach_gate_allows` + `epr_service` functions.
//! Handshake handling applies the real production `verify_handshake_request`
//! function and writes into the real `peer_identity_bindings` projection table.

#![allow(dead_code)]

use elohim_storage::db::models::NewPeerIdentityBindingRow;
use elohim_storage::db::DbPool;
use elohim_storage::p2p::identity_handshake::{
    synthesise_dht_anchor_hash, verify_handshake_request, HandshakeBindingPayload,
    IdentityHandshakeCodec, IdentityHandshakeProtocol, IdentityHandshakeRequest,
    IdentityHandshakeResponse, VerifyOutcome,
};
use elohim_storage::p2p::{
    reach_gate_allows, CallerIdentity, EprAtomCodec, EprAtomProtocol, EprAtomRequest,
    EprAtomResponse, HolochainBackedPeerIdentityMap, PeerIdentityMap, StubIdentityMap,
    MAX_BATCH_CIDS,
};
use elohim_storage::services::epr_service::{
    fetch_wire_bytes_by_cid, ingest, ingest_from_wire_bytes,
};
use elohim_storage::test_util::test_pool;
use futures::StreamExt;
use libp2p::{
    identify, identity, noise,
    request_response::{self, ProtocolSupport},
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux, Multiaddr, PeerId, SwarmBuilder,
};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};

// ---------------------------------------------------------------------------
// Harness behaviour — request-response (EPR atom + identity handshake) + identify.
// ---------------------------------------------------------------------------

#[derive(NetworkBehaviour)]
pub struct HarnessBehaviour {
    pub epr_atom: request_response::Behaviour<EprAtomCodec>,
    pub identity_handshake: request_response::Behaviour<IdentityHandshakeCodec>,
    pub identify: identify::Behaviour,
}

// ---------------------------------------------------------------------------
// Type aliases — reduces repetition and silences clippy::type_complexity.
// ---------------------------------------------------------------------------

type FetchTx = oneshot::Sender<Result<Option<Vec<u8>>, String>>;
type BatchTx = oneshot::Sender<Result<BatchOutcome, String>>;
type AnnounceTx = oneshot::Sender<Result<AnnouncedAck, String>>;

type FetchMap = HashMap<request_response::OutboundRequestId, FetchTx>;
type BatchMap = HashMap<request_response::OutboundRequestId, BatchTx>;
type AnnounceMap = HashMap<request_response::OutboundRequestId, AnnounceTx>;

// ---------------------------------------------------------------------------
// Command channel — test thread → driver task.
// ---------------------------------------------------------------------------

#[allow(clippy::large_enum_variant)]
enum Command {
    Dial(Multiaddr, oneshot::Sender<Result<(), String>>),
    FetchAtom(
        PeerId,
        String,
        oneshot::Sender<Result<Option<Vec<u8>>, String>>,
    ),
    FetchBatch(
        PeerId,
        Vec<String>,
        oneshot::Sender<Result<BatchOutcome, String>>,
    ),
    Announce(
        PeerId,
        Vec<u8>,
        oneshot::Sender<Result<AnnouncedAck, String>>,
    ),
    IngestLocal(elohim_epr::Epr, oneshot::Sender<()>),
    HasConnection(PeerId, oneshot::Sender<bool>),
}

// ---------------------------------------------------------------------------
// Public outcome types.
// ---------------------------------------------------------------------------

pub enum BatchOutcome {
    AtomBatch(Vec<Option<Vec<u8>>>),
    ProtocolError(String),
}

#[derive(Debug, Clone)]
pub struct AnnouncedAck {
    pub accepted: bool,
    pub reason: Option<String>,
}

// ---------------------------------------------------------------------------
// TestNode handle — given to test code.
// ---------------------------------------------------------------------------

pub struct TestNode {
    peer_id: PeerId,
    addr: Multiaddr,
    agent_pubkey: String,
    /// Stable agent CID string used in identity bindings (derived from agent_pubkey).
    agent_cid: String,
    keypair: elohim_epr::proof::AgentKeypair,
    signer_cid: cid::Cid,
    db_pool: DbPool,
    identity_map: Arc<StubIdentityMap>,
    cmd_tx: mpsc::Sender<Command>,
}

// ---------------------------------------------------------------------------
// spawn_test_node
// ---------------------------------------------------------------------------

pub async fn spawn_test_node(name: &str) -> TestNode {
    let db_pool = test_pool();
    let identity_map = Arc::new(StubIdentityMap::new());

    // libp2p identity — fresh keypair per node.
    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());

    // Build swarm with minimal behaviour (tcp + yamux + noise + request-response + identify).
    let mut swarm = SwarmBuilder::with_existing_identity(local_key.clone())
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )
        .expect("tcp transport")
        .with_behaviour(|key| {
            let epr_atom = request_response::Behaviour::<EprAtomCodec>::new(
                [(EprAtomProtocol, ProtocolSupport::Full)],
                request_response::Config::default().with_request_timeout(Duration::from_secs(5)),
            );
            let identity_handshake = request_response::Behaviour::<IdentityHandshakeCodec>::new(
                [(IdentityHandshakeProtocol, ProtocolSupport::Full)],
                request_response::Config::default().with_request_timeout(Duration::from_secs(5)),
            );
            let identify_cfg = identify::Config::new("/elohim/test/1.0.0".into(), key.public());
            HarnessBehaviour {
                epr_atom,
                identity_handshake,
                identify: identify::Behaviour::new(identify_cfg),
            }
        })
        .expect("behaviour build")
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(30)))
        .build();

    // Listen on a random TCP port.
    swarm
        .listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap())
        .expect("listen");

    // Pump events until we learn the actual listen address.
    let listen_addr = loop {
        match swarm.next().await.expect("swarm event") {
            SwarmEvent::NewListenAddr { address, .. } => break address,
            _ => continue,
        }
    };

    // Deterministic keypair from the node name so atoms authored by "a" always
    // have the same signer_cid regardless of run.
    let seed = {
        let mut s = [0u8; 32];
        let bytes = name.as_bytes();
        let len = bytes.len().min(32);
        s[..len].copy_from_slice(&bytes[..len]);
        s
    };
    let keypair =
        elohim_epr::proof::AgentKeypair::from_secret(&seed).expect("agent keypair from seed");
    let agent_pubkey = format!("agent-{name}");
    let signer_cid = elohim_epr::cid::compute_cid(agent_pubkey.as_bytes());
    // Stable agent CID string for use in handshake bindings.
    let agent_cid = signer_cid.to_string();

    let (cmd_tx, cmd_rx) = mpsc::channel::<Command>(32);

    // Spawn the async driver task.
    // The driver holds the local peer_id and agent_cid so it can send handshake requests.
    let driver_pool = db_pool.clone();
    let driver_identity = Arc::clone(&identity_map);
    let driver_peer_id = local_peer_id;
    let driver_agent_cid = agent_cid.clone();
    tokio::spawn(run_driver(
        swarm,
        cmd_rx,
        driver_pool,
        driver_identity,
        driver_peer_id,
        driver_agent_cid,
    ));

    TestNode {
        peer_id: local_peer_id,
        addr: listen_addr,
        agent_pubkey,
        agent_cid,
        keypair,
        signer_cid,
        db_pool,
        identity_map,
        cmd_tx,
    }
}

// ---------------------------------------------------------------------------
// Driver task — owns the swarm and routes commands ↔ events.
// ---------------------------------------------------------------------------

async fn run_driver(
    mut swarm: libp2p::Swarm<HarnessBehaviour>,
    mut cmd_rx: mpsc::Receiver<Command>,
    pool: DbPool,
    identity: Arc<StubIdentityMap>,
    local_peer_id: PeerId,
    local_agent_cid: String,
) {
    let mut pending_fetches: FetchMap = HashMap::new();
    let mut pending_batches: BatchMap = HashMap::new();
    let mut pending_announces: AnnounceMap = HashMap::new();
    let mut connected: HashSet<PeerId> = HashSet::new();

    loop {
        tokio::select! {
            biased;

            // --- incoming commands from test thread ---
            Some(cmd) = cmd_rx.recv() => {
                handle_command(
                    cmd,
                    &mut swarm,
                    &pool,
                    &mut pending_fetches,
                    &mut pending_batches,
                    &mut pending_announces,
                    &mut connected,
                );
            }

            // --- swarm events ---
            event = swarm.next() => {
                let Some(event) = event else { break; };
                handle_event(
                    event,
                    &mut swarm,
                    &pool,
                    &identity,
                    &local_peer_id,
                    &local_agent_cid,
                    &mut pending_fetches,
                    &mut pending_batches,
                    &mut pending_announces,
                    &mut connected,
                );
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_command(
    cmd: Command,
    swarm: &mut libp2p::Swarm<HarnessBehaviour>,
    pool: &DbPool,
    pending_fetches: &mut FetchMap,
    pending_batches: &mut BatchMap,
    pending_announces: &mut AnnounceMap,
    connected: &mut HashSet<PeerId>,
) {
    match cmd {
        Command::Dial(addr, reply) => {
            let res = swarm.dial(addr).map_err(|e| e.to_string());
            let _ = reply.send(res);
        }
        Command::FetchAtom(peer, cid, reply) => {
            let req_id = swarm
                .behaviour_mut()
                .epr_atom
                .send_request(&peer, EprAtomRequest::Fetch { cid });
            pending_fetches.insert(req_id, reply);
        }
        Command::FetchBatch(peer, cids, reply) => {
            let req_id = swarm
                .behaviour_mut()
                .epr_atom
                .send_request(&peer, EprAtomRequest::FetchBatch { cids });
            pending_batches.insert(req_id, reply);
        }
        Command::Announce(peer, bytes, reply) => {
            let req_id = swarm.behaviour_mut().epr_atom.send_request(
                &peer,
                EprAtomRequest::Announce {
                    envelope_bytes: bytes,
                },
            );
            pending_announces.insert(req_id, reply);
        }
        Command::IngestLocal(epr, reply) => {
            if let Ok(mut conn) = pool.get() {
                let _ = ingest(&mut conn, epr);
            }
            let _ = reply.send(());
        }
        Command::HasConnection(peer, reply) => {
            let _ = reply.send(connected.contains(&peer));
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_event(
    event: SwarmEvent<HarnessBehaviourEvent>,
    swarm: &mut libp2p::Swarm<HarnessBehaviour>,
    pool: &DbPool,
    identity: &Arc<StubIdentityMap>,
    local_peer_id: &PeerId,
    local_agent_cid: &str,
    pending_fetches: &mut FetchMap,
    pending_batches: &mut BatchMap,
    pending_announces: &mut AnnounceMap,
    connected: &mut HashSet<PeerId>,
) {
    match event {
        SwarmEvent::ConnectionEstablished { peer_id, .. } => {
            connected.insert(peer_id);
            // Send the identity handshake request to the newly connected peer.
            // The binding uses this node's own peer_id (base58) and agent_cid.
            let now_iso = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
            // Use uuid v4 bytes as a nonce (uuid::Uuid::new_v4 is available as a dependency).
            let nonce = {
                let bytes = *uuid::Uuid::new_v4().as_bytes();
                base64::Engine::encode(&base64::engine::general_purpose::STANDARD, bytes)
            };
            let request = IdentityHandshakeRequest {
                binding: HandshakeBindingPayload {
                    peer_id: local_peer_id.to_base58(),
                    agent_cid: local_agent_cid.to_string(),
                    // TODO(A.11): replace with AgentPeerBinding.valid_from from DHT signal
                    // stream when real binding source is wired.
                    valid_from: now_iso.clone(),
                    valid_until: None,
                    device_archetype: "node".to_string(),
                    // Use a non-empty sentinel signature for Stage 1 structural check.
                    signature: "aGFybmVzcy10ZXN0LXNpZ25hdHVyZQ==".to_string(),
                    dht_anchor_hash: None,
                },
                timestamp: now_iso,
                nonce,
            };
            swarm
                .behaviour_mut()
                .identity_handshake
                .send_request(&peer_id, request);
        }
        SwarmEvent::ConnectionClosed { peer_id, .. } => {
            connected.remove(&peer_id);
        }
        // Incoming EPR atom request — handle with production reach gate + epr_service.
        SwarmEvent::Behaviour(HarnessBehaviourEvent::EprAtom(
            request_response::Event::Message {
                peer,
                message:
                    request_response::Message::Request {
                        request, channel, ..
                    },
            },
        )) => {
            let response = handle_incoming_epr_request(pool, identity, peer, request);
            let _ = swarm
                .behaviour_mut()
                .epr_atom
                .send_response(channel, response);
        }
        // Outbound EPR atom response — resolve pending oneshot.
        SwarmEvent::Behaviour(HarnessBehaviourEvent::EprAtom(
            request_response::Event::Message {
                message:
                    request_response::Message::Response {
                        request_id,
                        response,
                    },
                ..
            },
        )) => {
            resolve_outbound_response(
                request_id,
                response,
                pending_fetches,
                pending_batches,
                pending_announces,
            );
        }
        // Outbound EPR atom failure — fail pending oneshot.
        SwarmEvent::Behaviour(HarnessBehaviourEvent::EprAtom(
            request_response::Event::OutboundFailure {
                request_id, error, ..
            },
        )) => {
            fail_outbound(
                request_id,
                error,
                pending_fetches,
                pending_batches,
                pending_announces,
            );
        }
        // Incoming identity handshake request — verify and insert into DB.
        SwarmEvent::Behaviour(HarnessBehaviourEvent::IdentityHandshake(
            request_response::Event::Message {
                peer,
                message:
                    request_response::Message::Request {
                        request, channel, ..
                    },
            },
        )) => {
            let response = handle_incoming_handshake(pool, peer, request);
            let _ = swarm
                .behaviour_mut()
                .identity_handshake
                .send_response(channel, response);
        }
        // Outbound handshake response — no pending oneshot needed (fire-and-forget).
        SwarmEvent::Behaviour(HarnessBehaviourEvent::IdentityHandshake(
            request_response::Event::Message {
                message: request_response::Message::Response { .. },
                ..
            },
        )) => {
            // Handshake response acknowledged; no action needed in the harness.
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Identity handshake handler — mirrors production logic from mod.rs.
// ---------------------------------------------------------------------------

fn handle_incoming_handshake(
    pool: &DbPool,
    peer: PeerId,
    request: IdentityHandshakeRequest,
) -> IdentityHandshakeResponse {
    let now_iso = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let peer_id_str = peer.to_base58();

    // Verify structural integrity + validity window.
    match verify_handshake_request(&request, &peer_id_str, &now_iso) {
        VerifyOutcome::Invalid(reason) => {
            tracing::warn!(
                peer = %peer_id_str,
                reason = %reason,
                "identity handshake rejected"
            );
            return IdentityHandshakeResponse::Rejected { reason };
        }
        VerifyOutcome::Valid => {}
    }

    // Insert into peer_identity_bindings.
    let b = &request.binding;
    let dht_anchor_hash = b
        .dht_anchor_hash
        .clone()
        .unwrap_or_else(|| synthesise_dht_anchor_hash(&b.peer_id, &b.agent_cid));

    let row = NewPeerIdentityBindingRow {
        peer_id: peer_id_str.clone(),
        agent_cid: b.agent_cid.clone(),
        dht_anchor_hash,
        valid_from: b.valid_from.clone(),
        valid_until: b.valid_until.clone(),
        observed_at: now_iso,
        source: "handshake".to_string(),
    };

    match pool.get() {
        Ok(mut conn) => {
            if let Err(e) = elohim_storage::db::peer_identity_bindings::upsert(&mut conn, &row) {
                tracing::warn!(
                    peer = %peer_id_str,
                    error = %e,
                    "identity handshake: db upsert failed"
                );
                return IdentityHandshakeResponse::Error(format!("db error: {e}"));
            }
        }
        Err(e) => {
            tracing::warn!(
                peer = %peer_id_str,
                error = %e,
                "identity handshake: pool exhausted"
            );
            return IdentityHandshakeResponse::Error("pool exhausted".to_string());
        }
    }

    IdentityHandshakeResponse::Accepted
}

// ---------------------------------------------------------------------------
// Incoming EPR atom request handler — mirrors production P2PNode logic.
// ---------------------------------------------------------------------------

fn handle_incoming_epr_request(
    pool: &DbPool,
    identity: &Arc<StubIdentityMap>,
    peer: PeerId,
    request: EprAtomRequest,
) -> EprAtomResponse {
    let caller: CallerIdentity = identity.lookup(&peer);
    let Ok(mut conn) = pool.get() else {
        return EprAtomResponse::Error {
            message: "pool exhausted".into(),
        };
    };
    match request {
        EprAtomRequest::Fetch { cid } => match fetch_wire_bytes_by_cid(&mut conn, &cid) {
            Ok(Some(f)) if reach_gate_allows(&f.reach, &caller, Some(&f.signer_cid)) => {
                EprAtomResponse::Atom {
                    envelope_bytes: f.wire_bytes,
                }
            }
            Ok(_) => EprAtomResponse::NotFound,
            Err(_) => EprAtomResponse::Error {
                message: "db error".into(),
            },
        },
        EprAtomRequest::Announce { envelope_bytes } => {
            match ingest_from_wire_bytes(&mut conn, &envelope_bytes) {
                Ok(_) => EprAtomResponse::Announced {
                    accepted: true,
                    reason: None,
                },
                Err(e) => EprAtomResponse::Announced {
                    accepted: false,
                    reason: Some(format!("{e:?}")),
                },
            }
        }
        EprAtomRequest::FetchBatch { cids } => {
            if cids.len() > MAX_BATCH_CIDS {
                return EprAtomResponse::Error {
                    message: format!("batch too large: {} (max {})", cids.len(), MAX_BATCH_CIDS),
                };
            }
            let mut atoms = Vec::with_capacity(cids.len());
            for cid in &cids {
                atoms.push(match fetch_wire_bytes_by_cid(&mut conn, cid) {
                    Ok(Some(f)) if reach_gate_allows(&f.reach, &caller, Some(&f.signer_cid)) => {
                        Some(f.wire_bytes)
                    }
                    _ => None,
                });
            }
            EprAtomResponse::AtomBatch { atoms }
        }
        EprAtomRequest::IntegrityNotify { .. } => EprAtomResponse::IntegrityAck {
            received: false,
            reason: Some("harness: not handled".to_string()),
        },
    }
}

// ---------------------------------------------------------------------------
// Outbound response resolution helpers.
// ---------------------------------------------------------------------------

fn resolve_outbound_response(
    request_id: request_response::OutboundRequestId,
    response: EprAtomResponse,
    fetches: &mut FetchMap,
    batches: &mut BatchMap,
    announces: &mut AnnounceMap,
) {
    if let Some(tx) = fetches.remove(&request_id) {
        let result = match response {
            EprAtomResponse::Atom { envelope_bytes } => Ok(Some(envelope_bytes)),
            EprAtomResponse::NotFound => Ok(None),
            EprAtomResponse::Error { message } => Err(message),
            other => Err(format!("unexpected fetch response: {other:?}")),
        };
        let _ = tx.send(result);
        return;
    }
    if let Some(tx) = batches.remove(&request_id) {
        let result = match response {
            EprAtomResponse::AtomBatch { atoms } => Ok(BatchOutcome::AtomBatch(atoms)),
            EprAtomResponse::Error { message } => Ok(BatchOutcome::ProtocolError(message)),
            other => Err(format!("unexpected batch response: {other:?}")),
        };
        let _ = tx.send(result);
        return;
    }
    if let Some(tx) = announces.remove(&request_id) {
        let result = match response {
            EprAtomResponse::Announced { accepted, reason } => {
                Ok(AnnouncedAck { accepted, reason })
            }
            other => Err(format!("unexpected announce response: {other:?}")),
        };
        let _ = tx.send(result);
    }
}

fn fail_outbound(
    request_id: request_response::OutboundRequestId,
    error: request_response::OutboundFailure,
    fetches: &mut FetchMap,
    batches: &mut BatchMap,
    announces: &mut AnnounceMap,
) {
    let msg = format!("outbound failure: {error:?}");
    if let Some(tx) = fetches.remove(&request_id) {
        let _ = tx.send(Err(msg));
        return;
    }
    if let Some(tx) = batches.remove(&request_id) {
        let _ = tx.send(Err(msg));
        return;
    }
    if let Some(tx) = announces.remove(&request_id) {
        let _ = tx.send(Err(msg));
    }
}

// ---------------------------------------------------------------------------
// TestNode impl
// ---------------------------------------------------------------------------

impl TestNode {
    pub fn peer_id(&self) -> PeerId {
        self.peer_id
    }

    pub fn addr(&self) -> Multiaddr {
        self.addr.clone()
    }

    pub fn agent_pubkey(&self) -> String {
        self.agent_pubkey.clone()
    }

    /// Stable agent CID string used in identity handshake bindings.
    pub fn agent_cid(&self) -> String {
        self.agent_cid.clone()
    }

    pub async fn dial(&self, addr: Multiaddr) -> Result<(), String> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(Command::Dial(addr, tx))
            .await
            .map_err(|e| e.to_string())?;
        rx.await.map_err(|_| "dial channel dropped".to_string())?
    }

    pub async fn wait_for_connection(&self, peer: &PeerId, timeout: Duration) {
        let start = tokio::time::Instant::now();
        while start.elapsed() < timeout {
            let (tx, rx) = oneshot::channel();
            if self
                .cmd_tx
                .send(Command::HasConnection(*peer, tx))
                .await
                .is_err()
            {
                return;
            }
            if let Ok(true) = rx.await {
                return;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    /// Look up a peer's identity in this node's `peer_identity_bindings` table.
    ///
    /// Uses `HolochainBackedPeerIdentityMap` backed by this node's real DB pool.
    /// Returns `CallerIdentity::Agent(cid)` if the handshake inserted a row, or
    /// `CallerIdentity::Anonymous` if no active binding exists.
    pub async fn lookup_peer_identity(&self, peer: &PeerId) -> CallerIdentity {
        let map = HolochainBackedPeerIdentityMap::new(self.db_pool.clone());
        map.lookup(peer)
    }

    /// Author a freshly signed atom owned by this node.
    ///
    /// Uses `EprKind::Manifest` with a fixed dummy governance coupling so the
    /// ingest validator is satisfied. Reach is parsed from the string form.
    pub async fn author_test_atom(&self, reach: &str, payload: &[u8]) -> elohim_epr::Epr {
        use elohim_epr::{Coupling, EprKind, Reach};

        let reach_parsed = match reach {
            "private" => Reach::Private,
            "self" => Reach::SelfScope,
            "intimate" => Reach::Intimate,
            "trusted" => Reach::Trusted,
            "familiar" => Reach::Familiar,
            "community" => Reach::Community,
            "public" => Reach::Public,
            "commons" => Reach::Commons,
            other => panic!("unknown reach: {other}"),
        };

        // Deterministic dummy coupling targets so tests are reproducible.
        let gov = elohim_epr::cid::compute_cid(b"harness-governance-target");
        let schema_ref = elohim_epr::cid::compute_cid(b"harness-schema-ref");
        let coupling = Coupling {
            knowledge: None,
            value: None,
            governance: Some(gov),
        };

        elohim_epr::Epr::builder()
            .kind(EprKind::Manifest)
            .schema_ref(schema_ref)
            .schema_key("harness/test")
            .reach(reach_parsed)
            .coupling(coupling)
            .issued_at(chrono::Utc::now())
            .payload(payload.to_vec())
            .sign(&self.keypair, self.signer_cid)
            .expect("sign test atom")
    }

    pub async fn ingest_local(&self, epr: elohim_epr::Epr) {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(Command::IngestLocal(epr, tx))
            .await
            .expect("send ingest command");
        rx.await.expect("ingest response");
    }

    pub async fn encode_envelope(&self, epr: &elohim_epr::Epr) -> Vec<u8> {
        let mut buf = Vec::new();
        ciborium::ser::into_writer(epr, &mut buf).expect("cbor encode");
        buf
    }

    pub async fn fetch_atom_from(
        &self,
        peer: &PeerId,
        cid: &str,
    ) -> Result<Option<Vec<u8>>, String> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(Command::FetchAtom(*peer, cid.to_string(), tx))
            .await
            .map_err(|e| e.to_string())?;
        rx.await.map_err(|_| "fetch channel dropped".to_string())?
    }

    pub async fn fetch_batch_from(
        &self,
        peer: &PeerId,
        cids: Vec<String>,
    ) -> Result<BatchOutcome, String> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(Command::FetchBatch(*peer, cids, tx))
            .await
            .map_err(|e| e.to_string())?;
        rx.await.map_err(|_| "batch channel dropped".to_string())?
    }

    pub async fn announce_to(&self, peer: &PeerId, bytes: Vec<u8>) -> Result<AnnouncedAck, String> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(Command::Announce(*peer, bytes, tx))
            .await
            .map_err(|e| e.to_string())?;
        rx.await
            .map_err(|_| "announce channel dropped".to_string())?
    }

    pub async fn register_peer_identity(&self, peer: &PeerId, agent_pubkey: &str) {
        self.identity_map.register(*peer, agent_pubkey);
    }
}
