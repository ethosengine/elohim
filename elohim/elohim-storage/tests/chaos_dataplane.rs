//! Chaos engineering — the byte plane under peer churn.
//!
//! D9 of the resilience dimensions proof suite
//! (spec: 2026-06-12-resilience-dimensions-proof-suite-design.md).
//!
//! These tests spawn REAL libp2p nodes (loopback TCP, the production
//! BlobProtocol/BlobCodec) and then kill them — proving the dataplane
//! properties every higher layer (race-fetch, heal-on-read, custody sweep)
//! quietly depends on:
//!
//!   1. a dead provider FAILS FAST — an in-flight or subsequent fetch
//!      resolves to an error within the protocol timeout, never a hang;
//!   2. the same content is immediately fetchable from a SURVIVING provider
//!      (the failover primitive under race_fetch);
//!   3. a provider that returns after death (new identity, same bytes —
//!      pod-restart shape) serves again after a fresh dial (churn recovery);
//!   4. a kill DURING transfer yields a bounded outcome (bytes or error,
//!      never a hang), and failover still completes the read.
//!
//! The companion live-cluster drills (kill an actual pod mid-suite) live in
//! genesis/a2o: federation/peer-loss-failover.feature + the
//! resilience/chaos-peer-churn.feature rows; CRDT-plane offline/merge churn
//! is covered by tests/sync_integration.rs::test_offline_merge.

use std::collections::HashMap;
use std::time::Duration;

use elohim_storage::p2p::{BlobCodec, BlobFetchRequest, BlobProtocol};
use futures::StreamExt;
use libp2p::{
    identity, noise,
    request_response::{self, ProtocolSupport},
    swarm::SwarmEvent,
    tcp, yamux, Multiaddr, PeerId, SwarmBuilder,
};
use sha2::{Digest, Sha256};
use tokio::sync::{mpsc, oneshot};

type FetchTx = oneshot::Sender<Result<Vec<u8>, String>>;

enum Cmd {
    Dial(Multiaddr, oneshot::Sender<Result<(), String>>),
    Fetch(PeerId, String, FetchTx),
    WaitConnected(PeerId, oneshot::Sender<()>),
    /// Chaos kill switch: drop the swarm (severing every connection and the
    /// listener) and exit the driver task. The pod-death analogue.
    Shutdown(oneshot::Sender<()>),
}

struct Node {
    peer_id: PeerId,
    addr: Multiaddr,
    cmd_tx: mpsc::Sender<Cmd>,
}

impl Node {
    async fn dial(&self, addr: Multiaddr) {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx.send(Cmd::Dial(addr, tx)).await.unwrap();
        rx.await.unwrap().expect("dial");
    }

    async fn wait_connected(&self, peer: PeerId) {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(Cmd::WaitConnected(peer, tx))
            .await
            .unwrap();
        rx.await.unwrap();
    }

    async fn fetch(&self, peer: PeerId, hash: &str) -> Result<Vec<u8>, String> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(Cmd::Fetch(peer, hash.to_string(), tx))
            .await
            .unwrap();
        rx.await.map_err(|_| "driver gone".to_string())?
    }

    /// Kill this node. Resolves once the swarm is dropped.
    async fn kill(&self) {
        let (tx, rx) = oneshot::channel();
        if self.cmd_tx.send(Cmd::Shutdown(tx)).await.is_ok() {
            let _ = rx.await;
        }
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    format!("sha256-{}", hex::encode(h.finalize()))
}

/// Spawn a node. `holdings` maps content hashes to payload bytes the driver
/// serves; pass an empty map for a pure fetcher. `request_timeout` keeps
/// dead-peer failures FAST — chaos tests assert bounded outcomes, so the
/// timeout is part of the contract under test.
async fn spawn_node(holdings: HashMap<String, Vec<u8>>, request_timeout: Duration) -> Node {
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
        .with_behaviour(|_| {
            request_response::Behaviour::<BlobCodec>::with_codec(
                BlobCodec::with_max_response_size(64 * 1024 * 1024),
                [(BlobProtocol, ProtocolSupport::Full)],
                request_response::Config::default().with_request_timeout(request_timeout),
            )
        })
        .expect("behaviour build")
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(120)))
        .build();

    swarm
        .listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap())
        .expect("listen");

    let listen_addr = loop {
        match swarm.next().await.expect("first event") {
            SwarmEvent::NewListenAddr { address, .. } => break address,
            _ => continue,
        }
    };

    let (cmd_tx, mut cmd_rx) = mpsc::channel::<Cmd>(64);

    tokio::spawn(async move {
        let mut pending_fetches: HashMap<request_response::OutboundRequestId, FetchTx> =
            HashMap::new();
        let mut connected: HashMap<PeerId, ()> = HashMap::new();
        let mut wait_connected: Vec<(PeerId, oneshot::Sender<()>)> = Vec::new();

        loop {
            tokio::select! {
                biased;

                Some(cmd) = cmd_rx.recv() => {
                    match cmd {
                        Cmd::Dial(addr, reply) => {
                            let _ = reply.send(swarm.dial(addr).map_err(|e| e.to_string()));
                        }
                        Cmd::Fetch(peer, hash, reply) => {
                            let req_id = swarm
                                .behaviour_mut()
                                .send_request(&peer, BlobFetchRequest { hash });
                            pending_fetches.insert(req_id, reply);
                        }
                        Cmd::WaitConnected(peer, reply) => {
                            if connected.contains_key(&peer) {
                                let _ = reply.send(());
                            } else {
                                wait_connected.push((peer, reply));
                            }
                        }
                        Cmd::Shutdown(reply) => {
                            // Dropping the swarm severs all connections and
                            // the listener — peer death, as the network sees it.
                            drop(swarm);
                            let _ = reply.send(());
                            return;
                        }
                    }
                }

                event = swarm.select_next_some() => {
                    match event {
                        SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                            connected.insert(peer_id, ());
                            // Waiters fire in the drain below.
                        }
                        SwarmEvent::Behaviour(request_response::Event::Message {
                            message,
                            ..
                        }) => match message {
                            request_response::Message::Request {
                                request, channel, ..
                            } => {
                                let resp = match holdings.get(&request.hash) {
                                    Some(bytes) => {
                                        elohim_storage::p2p::BlobFetchResponse::Found(
                                            bytes.clone(),
                                        )
                                    }
                                    None => elohim_storage::p2p::BlobFetchResponse::NotFound,
                                };
                                let _ = swarm
                                    .behaviour_mut()
                                    .send_response(channel, resp);
                            }
                            request_response::Message::Response {
                                request_id,
                                response,
                            } => {
                                if let Some(tx) = pending_fetches.remove(&request_id) {
                                    match response {
                                        elohim_storage::p2p::BlobFetchResponse::Found(
                                            bytes,
                                        ) => {
                                            let _ = tx.send(Ok(bytes));
                                        }
                                        _ => {
                                            let _ = tx.send(Err("not found".into()));
                                        }
                                    }
                                }
                            }
                        },
                        SwarmEvent::Behaviour(request_response::Event::OutboundFailure {
                            request_id,
                            error,
                            ..
                        }) => {
                            if let Some(tx) = pending_fetches.remove(&request_id) {
                                let _ = tx.send(Err(format!("outbound failure: {error}")));
                            }
                        }
                        _ => {}
                    }
                    // Fire any waiters whose peer is now connected.
                    let mut still_waiting = Vec::new();
                    for (p, tx) in wait_connected.drain(..) {
                        if connected.contains_key(&p) {
                            let _ = tx.send(());
                        } else {
                            still_waiting.push((p, tx));
                        }
                    }
                    wait_connected = still_waiting;
                }
            }
        }
    });

    Node {
        peer_id: local_peer_id,
        addr: listen_addr,
        cmd_tx,
    }
}

const FAST_TIMEOUT: Duration = Duration::from_secs(5);
/// Outer guard: every chaos outcome must resolve well inside this — a hang
/// is itself the failure mode under test.
const BOUNDED: Duration = Duration::from_secs(20);

fn payload(tag: &str, len: usize) -> Vec<u8> {
    tag.bytes().cycle().take(len).collect()
}

#[tokio::test]
async fn chaos_dead_provider_fails_fast_never_hangs() {
    let bytes = payload("chaos-a", 256 * 1024);
    let hash = sha256_hex(&bytes);
    let provider = spawn_node(HashMap::from([(hash.clone(), bytes)]), FAST_TIMEOUT).await;
    let fetcher = spawn_node(HashMap::new(), FAST_TIMEOUT).await;

    fetcher.dial(provider.addr.clone()).await;
    fetcher.wait_connected(provider.peer_id).await;

    // Sanity: alive fetch works.
    let ok = tokio::time::timeout(BOUNDED, fetcher.fetch(provider.peer_id, &hash))
        .await
        .expect("alive fetch bounded")
        .expect("alive fetch succeeds");
    assert_eq!(sha256_hex(&ok), hash);

    // Kill the provider, then fetch again: must resolve to an ERROR within
    // the bound — never hang, never bytes.
    provider.kill().await;
    let dead = tokio::time::timeout(BOUNDED, fetcher.fetch(provider.peer_id, &hash))
        .await
        .expect("dead-provider fetch must resolve within the bound (no hang)");
    assert!(dead.is_err(), "fetch from a dead provider must error");
}

#[tokio::test]
async fn chaos_failover_to_surviving_provider() {
    let bytes = payload("chaos-b", 512 * 1024);
    let hash = sha256_hex(&bytes);
    // Two independent providers hold the same content-addressed bytes.
    let provider_a = spawn_node(HashMap::from([(hash.clone(), bytes.clone())]), FAST_TIMEOUT).await;
    let provider_b = spawn_node(HashMap::from([(hash.clone(), bytes)]), FAST_TIMEOUT).await;
    let fetcher = spawn_node(HashMap::new(), FAST_TIMEOUT).await;

    fetcher.dial(provider_a.addr.clone()).await;
    fetcher.wait_connected(provider_a.peer_id).await;
    fetcher.dial(provider_b.addr.clone()).await;
    fetcher.wait_connected(provider_b.peer_id).await;

    // A dies. The read completes from B — the failover primitive that
    // race_fetch/heal-on-read are built on.
    provider_a.kill().await;
    let _ = tokio::time::timeout(BOUNDED, fetcher.fetch(provider_a.peer_id, &hash))
        .await
        .expect("dead-A fetch bounded");
    let from_b = tokio::time::timeout(BOUNDED, fetcher.fetch(provider_b.peer_id, &hash))
        .await
        .expect("survivor fetch bounded")
        .expect("survivor serves the bytes");
    assert_eq!(
        sha256_hex(&from_b),
        hash,
        "content-addressing must verify on the failover path"
    );
}

#[tokio::test]
async fn chaos_returning_provider_serves_again() {
    let bytes = payload("chaos-c", 128 * 1024);
    let hash = sha256_hex(&bytes);
    let provider = spawn_node(HashMap::from([(hash.clone(), bytes.clone())]), FAST_TIMEOUT).await;
    let fetcher = spawn_node(HashMap::new(), FAST_TIMEOUT).await;

    fetcher.dial(provider.addr.clone()).await;
    fetcher.wait_connected(provider.peer_id).await;
    provider.kill().await;

    // The pod-restart shape: the peer returns with a fresh identity and
    // listen addr but the same content-addressed holdings. A fresh dial +
    // fetch must serve — churn is an inconvenience, not a loss.
    let reborn = spawn_node(HashMap::from([(hash.clone(), bytes)]), FAST_TIMEOUT).await;
    fetcher.dial(reborn.addr.clone()).await;
    fetcher.wait_connected(reborn.peer_id).await;
    let again = tokio::time::timeout(BOUNDED, fetcher.fetch(reborn.peer_id, &hash))
        .await
        .expect("post-churn fetch bounded")
        .expect("returning provider serves again");
    assert_eq!(sha256_hex(&again), hash);
}

#[tokio::test]
async fn chaos_kill_during_transfer_is_bounded_and_failover_completes() {
    // Large enough that the kill lands mid-conversation on loopback.
    let bytes = payload("chaos-d", 8 * 1024 * 1024);
    let hash = sha256_hex(&bytes);
    let provider_a = spawn_node(HashMap::from([(hash.clone(), bytes.clone())]), FAST_TIMEOUT).await;
    let provider_b = spawn_node(HashMap::from([(hash.clone(), bytes)]), FAST_TIMEOUT).await;
    let fetcher = spawn_node(HashMap::new(), FAST_TIMEOUT).await;

    fetcher.dial(provider_a.addr.clone()).await;
    fetcher.wait_connected(provider_a.peer_id).await;
    fetcher.dial(provider_b.addr.clone()).await;
    fetcher.wait_connected(provider_b.peer_id).await;

    // Race the kill against the in-flight transfer.
    let inflight = fetcher.fetch(provider_a.peer_id, &hash);
    let kill = provider_a.kill();
    let (inflight_result, _) = tokio::join!(
        async { tokio::time::timeout(BOUNDED, inflight).await },
        kill
    );
    let inflight_result = inflight_result
        .expect("mid-transfer kill must yield a bounded outcome (bytes or error), never a hang");
    if let Ok(data) = &inflight_result {
        // The transfer beat the kill — fine, but the bytes must verify.
        assert_eq!(sha256_hex(data), hash);
    }

    // Either way the read completes from the survivor.
    let from_b = tokio::time::timeout(BOUNDED, fetcher.fetch(provider_b.peer_id, &hash))
        .await
        .expect("survivor fetch bounded")
        .expect("survivor serves after mid-transfer chaos");
    assert_eq!(sha256_hex(&from_b), hash);
}
