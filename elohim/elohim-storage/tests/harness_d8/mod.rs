//! D.8 in-process harness — Kademlia + Gossipsub + request-response.
//!
//! # Harness path rationale
//!
//! The existing `tests/harness/mod.rs` combines request-response (EPR atom +
//! identity handshake) + identify into a minimal `HarnessBehaviour`.  Adding
//! Kademlia and Gossipsub to that struct would require:
//!   - a new `NetworkBehaviour` derive with 5 sub-behaviours,
//!   - rewriting every `HarnessBehaviourEvent` match arm, and
//!   - wiring a `P2PCommand` channel that the existing harness API does not
//!     expose.
//!
//! That would be invasive and risk regressing the A.x / B.x integration tests
//! that rely on the existing harness.  D.8 therefore lives in its own
//! `harness_d8` module.  `tests/epr_atom_federation_d8.rs` uses only this
//! module; all other integration tests continue to use `tests/harness/mod.rs`.
//!
//! # What this harness provides
//!
//! Each `D8Node` owns:
//!   - a libp2p Swarm with Kademlia (MemoryStore), Gossipsub, identify, and
//!     request-response (EPR atom + identity handshake) — the same combination
//!     that production `ElohimStorageBehaviour` uses
//!   - an in-memory DB pool with migrations applied
//!   - a `P2PCommand` channel — `FederatedEprStore` can be wired to it so that
//!     `put()` triggers real Kad + gossip commands
//!   - a `gossip_rx` channel that the test thread reads to assert that a
//!     specific topic message arrived
//!   - a `direct_notify_rx` channel for IntegrityNotify arrivals
//!   - a Kademlia query responder via the `P2PCommand::KadGetProviders` path
//!
//! # HARNESS-LIMITATION comments
//!
//! See individual `// HARNESS-LIMITATION:` comments inline for gaps between
//! what this harness exercises and what full production wiring would provide.

#![allow(dead_code)]

use elohim_storage::db::DbPool;
use elohim_storage::p2p::identity_handshake::{IdentityHandshakeCodec, IdentityHandshakeProtocol};
use elohim_storage::p2p::P2PCommand;
use elohim_storage::p2p::{EprAtomCodec, EprAtomProtocol, EprAtomRequest, EprAtomResponse};
use elohim_storage::test_util::test_pool;
use futures::StreamExt;
use libp2p::{
    gossipsub::{self, IdentTopic},
    identify,
    identity::{self, Keypair},
    kad::{self, store::MemoryStore},
    noise,
    request_response::{self, ProtocolSupport},
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux, Multiaddr, PeerId, SwarmBuilder,
};
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};

// ---------------------------------------------------------------------------
// D8 behaviour — Kad + Gossipsub + request-response + identify
// ---------------------------------------------------------------------------

#[derive(NetworkBehaviour)]
pub struct D8Behaviour {
    pub kademlia: kad::Behaviour<MemoryStore>,
    pub gossipsub: gossipsub::Behaviour,
    pub epr_atom: request_response::Behaviour<EprAtomCodec>,
    pub identity_handshake: request_response::Behaviour<IdentityHandshakeCodec>,
    pub identify: identify::Behaviour,
}

// ---------------------------------------------------------------------------
// Commands from test thread → driver
// ---------------------------------------------------------------------------

#[allow(clippy::large_enum_variant)]
enum D8Command {
    Dial(Multiaddr, oneshot::Sender<Result<(), String>>),
    HasConnection(PeerId, oneshot::Sender<bool>),
    /// Subscribe to a gossipsub topic by name.
    Subscribe(String, oneshot::Sender<()>),
    /// Publish a message to a gossipsub topic.
    GossipPublish(String, Vec<u8>, oneshot::Sender<Result<(), String>>),
    /// Issue a KadStartProviding command.
    KadStartProviding(String, oneshot::Sender<()>),
    /// Issue a KadGetProviders query and return the first batch of providers.
    KadGetProviders(String, oneshot::Sender<Vec<PeerId>>),
    /// Add peer to Kad routing table.
    KadAddAddress(PeerId, Multiaddr, oneshot::Sender<()>),
    /// Send IntegrityNotify to a specific peer.
    DirectNotify(PeerId, String, Vec<u8>, oneshot::Sender<()>),
    /// Drain any pending P2PCommands from the FederatedEprStore channel.
    DrainP2PCommands(oneshot::Sender<Vec<P2PCommand>>),
}

// ---------------------------------------------------------------------------
// D8Node handle
// ---------------------------------------------------------------------------

pub struct D8Node {
    pub peer_id: PeerId,
    pub addr: Multiaddr,
    pub keypair: Keypair,
    pub db_pool: DbPool,
    cmd_tx: mpsc::Sender<D8Command>,
    /// Gossip message receiver — each gossip message received by this node
    /// is forwarded here: (topic_str, data_bytes).
    pub gossip_rx: mpsc::Receiver<(String, Vec<u8>)>,
    /// IntegrityNotify message receiver — each IntegrityNotify request received
    /// by this node is forwarded here: (kind, payload_bytes).
    pub integrity_rx: mpsc::Receiver<(String, Vec<u8>)>,
    /// Channel through which `FederatedEprStore` can send P2PCommands to this
    /// node's swarm.  Clone and pass to `FederatedEprStore::with_swarm_tx`.
    p2p_cmd_tx: mpsc::Sender<P2PCommand>,
}

// ---------------------------------------------------------------------------
// spawn_d8_node
// ---------------------------------------------------------------------------

pub async fn spawn_d8_node(_name: &str, extra_topics: &[&str]) -> D8Node {
    let db_pool = test_pool();

    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());

    // Gossipsub — constraint: mesh_outbound_min <= mesh_n_low <= mesh_n <= mesh_n_high
    // Defaults are (2, 5, 6, 12). Reduce all to allow small (2–4 node) test meshes.
    let gossipsub_config = gossipsub::ConfigBuilder::default()
        .heartbeat_interval(Duration::from_millis(100))
        .validation_mode(gossipsub::ValidationMode::Permissive)
        .mesh_outbound_min(1)
        .mesh_n_low(2)
        .mesh_n(3)
        .mesh_n_high(6)
        .build()
        .expect("gossipsub config");
    let mut gossipsub_behaviour = gossipsub::Behaviour::new(
        gossipsub::MessageAuthenticity::Signed(local_key.clone()),
        gossipsub_config,
    )
    .expect("gossipsub behaviour");
    // Subscribe only to caller-specified topics — no default subscriptions.
    // Callers that want TOPIC_INTEGRITY_REVOCATION must include it in `extra_topics`.
    for t in extra_topics {
        gossipsub_behaviour
            .subscribe(&IdentTopic::new(*t))
            .expect("subscribe extra topic");
    }

    // Kademlia (in-memory store — no sled required in tests)
    let kad_store = MemoryStore::new(local_peer_id);
    let mut kad = kad::Behaviour::new(local_peer_id, kad_store);
    kad.set_mode(Some(kad::Mode::Server));

    let epr_atom = request_response::Behaviour::<EprAtomCodec>::new(
        [(EprAtomProtocol, ProtocolSupport::Full)],
        request_response::Config::default().with_request_timeout(Duration::from_secs(5)),
    );
    let identity_handshake = request_response::Behaviour::<IdentityHandshakeCodec>::new(
        [(IdentityHandshakeProtocol, ProtocolSupport::Full)],
        request_response::Config::default().with_request_timeout(Duration::from_secs(5)),
    );
    let identify_cfg = identify::Config::new("/elohim/d8-test/1.0.0".into(), local_key.public());

    let mut swarm = SwarmBuilder::with_existing_identity(local_key.clone())
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )
        .expect("tcp")
        .with_behaviour(|_key| D8Behaviour {
            kademlia: kad,
            gossipsub: gossipsub_behaviour,
            epr_atom,
            identity_handshake,
            identify: identify::Behaviour::new(identify_cfg),
        })
        .expect("behaviour")
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

    let (cmd_tx, cmd_rx) = mpsc::channel::<D8Command>(64);
    let (gossip_fwd_tx, gossip_fwd_rx) = mpsc::channel::<(String, Vec<u8>)>(64);
    let (integrity_fwd_tx, integrity_fwd_rx) = mpsc::channel::<(String, Vec<u8>)>(64);
    // P2PCommand channel: FederatedEprStore writes here; the driver reads and forwards to swarm.
    let (p2p_cmd_tx, p2p_cmd_rx) = mpsc::channel::<P2PCommand>(64);
    let driver_p2p_cmd_rx = p2p_cmd_rx;

    tokio::spawn(run_d8_driver(
        swarm,
        cmd_rx,
        driver_p2p_cmd_rx,
        gossip_fwd_tx,
        integrity_fwd_tx,
    ));

    D8Node {
        peer_id: local_peer_id,
        addr: listen_addr,
        keypair: local_key,
        db_pool,
        cmd_tx,
        gossip_rx: gossip_fwd_rx,
        integrity_rx: integrity_fwd_rx,
        p2p_cmd_tx,
    }
}

// ---------------------------------------------------------------------------
// Driver task
// ---------------------------------------------------------------------------

async fn run_d8_driver(
    mut swarm: libp2p::Swarm<D8Behaviour>,
    mut cmd_rx: mpsc::Receiver<D8Command>,
    mut p2p_cmd_rx: mpsc::Receiver<P2PCommand>,
    gossip_fwd: mpsc::Sender<(String, Vec<u8>)>,
    integrity_fwd: mpsc::Sender<(String, Vec<u8>)>,
) {
    let mut connected: std::collections::HashSet<PeerId> = Default::default();
    // KadGetProviders: query_id → (accumulated_providers, reply_oneshot)
    let mut pending_kad_providers: HashMap<
        kad::QueryId,
        (Vec<PeerId>, oneshot::Sender<Vec<PeerId>>),
    > = HashMap::new();
    // Deferred DrainP2PCommands replies — accumulate commands until requested
    let mut buffered_p2p_cmds: Vec<P2PCommand> = Vec::new();
    let mut drain_reply: Option<oneshot::Sender<Vec<P2PCommand>>> = None;

    loop {
        tokio::select! {
            biased;

            // Test-thread commands
            Some(cmd) = cmd_rx.recv() => {
                match cmd {
                    D8Command::Dial(addr, reply) => {
                        let res = swarm.dial(addr).map_err(|e| e.to_string());
                        let _ = reply.send(res);
                    }
                    D8Command::HasConnection(peer, reply) => {
                        let _ = reply.send(connected.contains(&peer));
                    }
                    D8Command::Subscribe(topic, reply) => {
                        let t = IdentTopic::new(&topic);
                        let _ = swarm.behaviour_mut().gossipsub.subscribe(&t);
                        let _ = reply.send(());
                    }
                    D8Command::GossipPublish(topic, data, reply) => {
                        let t = IdentTopic::new(&topic);
                        let result = swarm
                            .behaviour_mut()
                            .gossipsub
                            .publish(t, data)
                            .map(|_| ())
                            .map_err(|e| format!("{e:?}"));
                        let _ = reply.send(result);
                    }
                    D8Command::KadStartProviding(cid, reply) => {
                        use libp2p::kad::RecordKey;
                        let key_str = format!("epr-atom:{cid}");
                        let key = RecordKey::new(&key_str);
                        let _ = swarm.behaviour_mut().kademlia.start_providing(key);
                        let _ = reply.send(());
                    }
                    D8Command::KadGetProviders(cid, reply) => {
                        use libp2p::kad::RecordKey;
                        let key_str = format!("epr-atom:{cid}");
                        let key = RecordKey::new(&key_str);
                        let query_id = swarm.behaviour_mut().kademlia.get_providers(key);
                        pending_kad_providers.insert(query_id, (Vec::new(), reply));
                    }
                    D8Command::KadAddAddress(peer, addr, reply) => {
                        swarm.behaviour_mut().kademlia.add_address(&peer, addr);
                        let _ = reply.send(());
                    }
                    D8Command::DirectNotify(peer, kind, payload, reply) => {
                        let req = EprAtomRequest::IntegrityNotify {
                            kind,
                            payload_bytes: payload,
                        };
                        swarm.behaviour_mut().epr_atom.send_request(&peer, req);
                        let _ = reply.send(());
                    }
                    D8Command::DrainP2PCommands(reply) => {
                        if buffered_p2p_cmds.is_empty() {
                            // Store the reply to fulfil when the next command arrives
                            drain_reply = Some(reply);
                        } else {
                            let cmds = std::mem::take(&mut buffered_p2p_cmds);
                            let _ = reply.send(cmds);
                        }
                    }
                }
            }

            // P2PCommands from FederatedEprStore
            Some(p2p_cmd) = p2p_cmd_rx.recv() => {
                handle_p2p_command(
                    &mut swarm,
                    p2p_cmd,
                    &mut pending_kad_providers,
                    &mut buffered_p2p_cmds,
                    &mut drain_reply,
                );
            }

            // Swarm events
            event = swarm.next() => {
                let Some(event) = event else { break; };
                handle_d8_event(
                    event,
                    &mut swarm,
                    &mut connected,
                    &mut pending_kad_providers,
                    &gossip_fwd,
                    &integrity_fwd,
                );
            }
        }
    }
}

fn handle_p2p_command(
    swarm: &mut libp2p::Swarm<D8Behaviour>,
    cmd: P2PCommand,
    pending_kad_providers: &mut HashMap<kad::QueryId, (Vec<PeerId>, oneshot::Sender<Vec<PeerId>>)>,
    buffered_p2p_cmds: &mut Vec<P2PCommand>,
    drain_reply: &mut Option<oneshot::Sender<Vec<P2PCommand>>>,
) {
    use libp2p::kad::RecordKey;

    match cmd {
        P2PCommand::KadStartProviding { cid } => {
            let key = RecordKey::new(&format!("epr-atom:{cid}"));
            let _ = swarm.behaviour_mut().kademlia.start_providing(key);
            // KadStartProviding is fire-and-forget; no buffering needed.
            return;
        }
        P2PCommand::KadGetProviders { cid, reply } => {
            let key = RecordKey::new(&format!("epr-atom:{cid}"));
            let query_id = swarm.behaviour_mut().kademlia.get_providers(key);
            pending_kad_providers.insert(query_id, (Vec::new(), reply));
            // KadGetProviders is handled inline; no buffering needed.
            return;
        }
        P2PCommand::PublishEprAnnounce {
            ref topic,
            ref payload,
        } => {
            let t = IdentTopic::new(topic);
            let _ = swarm.behaviour_mut().gossipsub.publish(t, payload.clone());
        }
        P2PCommand::DirectNotifyIntegrity {
            ref peer_ids,
            ref kind,
            ref payload_bytes,
        } => {
            for peer_id in peer_ids {
                let req = EprAtomRequest::IntegrityNotify {
                    kind: kind.clone(),
                    payload_bytes: payload_bytes.clone(),
                };
                swarm.behaviour_mut().epr_atom.send_request(peer_id, req);
            }
        }
        _ => {
            // Other P2PCommands (PushShard, ResolveEpr, etc.) not relevant to D.8.
        }
    }

    // Buffer command for DrainP2PCommands
    if let Some(reply) = drain_reply.take() {
        buffered_p2p_cmds.push(cmd);
        let cmds = std::mem::take(buffered_p2p_cmds);
        let _ = reply.send(cmds);
    } else {
        buffered_p2p_cmds.push(cmd);
    }
}

fn handle_d8_event(
    event: SwarmEvent<D8BehaviourEvent>,
    swarm: &mut libp2p::Swarm<D8Behaviour>,
    connected: &mut std::collections::HashSet<PeerId>,
    pending_kad_providers: &mut HashMap<kad::QueryId, (Vec<PeerId>, oneshot::Sender<Vec<PeerId>>)>,
    gossip_fwd: &mpsc::Sender<(String, Vec<u8>)>,
    integrity_fwd: &mpsc::Sender<(String, Vec<u8>)>,
) {
    match event {
        SwarmEvent::ConnectionEstablished { peer_id, .. } => {
            connected.insert(peer_id);
        }
        SwarmEvent::ConnectionClosed { peer_id, .. } => {
            connected.remove(&peer_id);
        }
        // Kad: provider query progress
        SwarmEvent::Behaviour(D8BehaviourEvent::Kademlia(
            kad::Event::OutboundQueryProgressed {
                id,
                result:
                    kad::QueryResult::GetProviders(Ok(kad::GetProvidersOk::FoundProviders {
                        providers,
                        ..
                    })),
                ..
            },
        )) => {
            if let Some(entry) = pending_kad_providers.get_mut(&id) {
                entry.0.extend(providers);
            }
        }
        SwarmEvent::Behaviour(D8BehaviourEvent::Kademlia(
            kad::Event::OutboundQueryProgressed {
                id,
                result:
                    kad::QueryResult::GetProviders(Ok(
                        kad::GetProvidersOk::FinishedWithNoAdditionalRecord { .. },
                    )),
                ..
            },
        )) => {
            if let Some((providers, reply)) = pending_kad_providers.remove(&id) {
                let _ = reply.send(providers);
            }
        }
        SwarmEvent::Behaviour(D8BehaviourEvent::Kademlia(
            kad::Event::OutboundQueryProgressed {
                id,
                result: kad::QueryResult::GetProviders(Err(_)),
                ..
            },
        )) => {
            if let Some((providers, reply)) = pending_kad_providers.remove(&id) {
                let _ = reply.send(providers);
            }
        }
        // Gossipsub: forward received messages to test thread
        SwarmEvent::Behaviour(D8BehaviourEvent::Gossipsub(gossipsub::Event::Message {
            message,
            ..
        })) => {
            let topic = message.topic.as_str().to_string();
            let _ = gossip_fwd.try_send((topic, message.data));
        }
        // EprAtom request-response: handle IntegrityNotify
        SwarmEvent::Behaviour(D8BehaviourEvent::EprAtom(request_response::Event::Message {
            peer: _,
            message:
                request_response::Message::Request {
                    request: EprAtomRequest::IntegrityNotify { kind, payload_bytes },
                    channel,
                    ..
                },
        })) => {
            let _ = integrity_fwd.try_send((kind.clone(), payload_bytes));
            let _ = swarm.behaviour_mut().epr_atom.send_response(
                channel,
                EprAtomResponse::IntegrityAck {
                    received: true,
                    reason: None,
                },
            );
        }
        // Identify — needed for Gossipsub mesh formation
        SwarmEvent::Behaviour(D8BehaviourEvent::Identify(identify::Event::Received {
            peer_id,
            info,
            ..
        })) => {
            // Add observed addresses to Kademlia so the DHT can route queries
            for addr in info.listen_addrs {
                swarm.behaviour_mut().kademlia.add_address(&peer_id, addr);
            }
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// D8Node impl
// ---------------------------------------------------------------------------

impl D8Node {
    pub fn peer_id(&self) -> PeerId {
        self.peer_id
    }

    pub fn addr(&self) -> Multiaddr {
        self.addr.clone()
    }

    /// The P2PCommand sender to wire into `FederatedEprStore::with_swarm_tx`.
    pub fn p2p_cmd_tx(&self) -> mpsc::Sender<P2PCommand> {
        self.p2p_cmd_tx.clone()
    }

    pub async fn dial(&self, addr: Multiaddr) -> Result<(), String> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx.send(D8Command::Dial(addr, tx)).await.unwrap();
        rx.await.unwrap()
    }

    pub async fn wait_for_connection(&self, peer: PeerId, timeout: Duration) {
        let start = tokio::time::Instant::now();
        while start.elapsed() < timeout {
            let (tx, rx) = oneshot::channel();
            let _ = self.cmd_tx.send(D8Command::HasConnection(peer, tx)).await;
            if let Ok(true) = rx.await {
                return;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    pub async fn subscribe(&self, topic: &str) {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(D8Command::Subscribe(topic.to_string(), tx))
            .await
            .unwrap();
        rx.await.unwrap();
    }

    pub async fn gossip_publish(&self, topic: &str, data: Vec<u8>) -> Result<(), String> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(D8Command::GossipPublish(topic.to_string(), data, tx))
            .await
            .unwrap();
        rx.await.unwrap()
    }

    pub async fn kad_start_providing(&self, cid: &str) {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(D8Command::KadStartProviding(cid.to_string(), tx))
            .await
            .unwrap();
        rx.await.unwrap();
    }

    pub async fn kad_get_providers(&self, cid: &str) -> Vec<PeerId> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(D8Command::KadGetProviders(cid.to_string(), tx))
            .await
            .unwrap();
        // Wait up to 5 s for Kad to reply
        tokio::time::timeout(Duration::from_secs(5), rx)
            .await
            .expect("kad_get_providers timed out")
            .expect("channel closed")
    }

    pub async fn kad_add_address(&self, peer: PeerId, addr: Multiaddr) {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(D8Command::KadAddAddress(peer, addr, tx))
            .await
            .unwrap();
        rx.await.unwrap();
    }

    /// Send a direct IntegrityNotify to a peer.
    pub async fn direct_notify(&self, peer: PeerId, kind: &str, payload: Vec<u8>) {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(D8Command::DirectNotify(peer, kind.to_string(), payload, tx))
            .await
            .unwrap();
        rx.await.unwrap();
    }

    /// Wait up to `timeout` for a gossip message on any topic.
    /// Returns `Some((topic, data))` if received within timeout, else `None`.
    pub async fn wait_gossip(&mut self, timeout: Duration) -> Option<(String, Vec<u8>)> {
        tokio::time::timeout(timeout, self.gossip_rx.recv())
            .await
            .ok()
            .flatten()
    }

    /// Wait up to `timeout` for a gossip message on the specified topic.
    pub async fn wait_gossip_on_topic(
        &mut self,
        topic: &str,
        timeout: Duration,
    ) -> Option<Vec<u8>> {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return None;
            }
            match tokio::time::timeout(remaining, self.gossip_rx.recv()).await {
                Ok(Some((t, data))) if t == topic => return Some(data),
                Ok(Some(_)) => continue, // wrong topic — keep waiting
                Ok(None) | Err(_) => return None,
            }
        }
    }

    /// Wait up to `timeout` for an IntegrityNotify of the given `kind`.
    pub async fn wait_integrity_notify(
        &mut self,
        kind: &str,
        timeout: Duration,
    ) -> Option<Vec<u8>> {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return None;
            }
            match tokio::time::timeout(remaining, self.integrity_rx.recv()).await {
                Ok(Some((k, data))) if k == kind => return Some(data),
                Ok(Some(_)) => continue,
                Ok(None) | Err(_) => return None,
            }
        }
    }

    /// Assert no gossip message arrives within `timeout`.
    pub async fn assert_no_gossip(&mut self, timeout: Duration) {
        match tokio::time::timeout(timeout, self.gossip_rx.recv()).await {
            Ok(Some((topic, _))) => {
                panic!("unexpected gossip message on topic: {topic}");
            }
            Ok(None) => {}
            Err(_) => {} // timeout — expected
        }
    }

    /// Assert no IntegrityNotify arrives within `timeout`.
    pub async fn assert_no_integrity_notify(&mut self, timeout: Duration) {
        match tokio::time::timeout(timeout, self.integrity_rx.recv()).await {
            Ok(Some((kind, _))) => {
                panic!("unexpected IntegrityNotify of kind: {kind}");
            }
            Ok(None) => {}
            Err(_) => {}
        }
    }
}

/// Wait until a connection between `from` and `to` is established, or panic.
pub async fn wait_connected(from: &D8Node, to: &D8Node) {
    from.wait_for_connection(to.peer_id, Duration::from_secs(5))
        .await;
}

/// Dial `b` from `a` and wait for both sides to report the connection.
pub async fn connect(a: &D8Node, b: &D8Node) {
    a.dial(b.addr()).await.expect("dial must succeed");
    a.wait_for_connection(b.peer_id, Duration::from_secs(5))
        .await;
    b.wait_for_connection(a.peer_id, Duration::from_secs(5))
        .await;
    // Add each peer to the other's Kad routing table.
    a.kad_add_address(b.peer_id, b.addr()).await;
    b.kad_add_address(a.peer_id, a.addr()).await;
    // Give gossipsub mesh time to form.
    tokio::time::sleep(Duration::from_millis(200)).await;
}
