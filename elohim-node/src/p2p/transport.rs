//! libp2p transport configuration
//!
//! Builds the ElohimSwarm with multi-transport support (QUIC + TCP/Noise/Yamux),
//! mDNS discovery, Kademlia DHT, and request-response for sync.

use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result};
use futures::StreamExt;
use libp2p::{
    identity, kad, mdns, noise, request_response, tcp, yamux,
    Multiaddr, PeerId, StreamProtocol, Swarm, SwarmBuilder,
};
use libp2p::swarm::NetworkBehaviour;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use crate::config::P2PConfig;
use crate::sync::protocol::SyncMessage;
use super::protocols::SyncCodec;

/// Combined libp2p behaviour for Elohim nodes.
#[derive(NetworkBehaviour)]
pub struct ElohimBehaviour {
    pub request_response: request_response::Behaviour<SyncCodec>,
    pub mdns: mdns::tokio::Behaviour,
    pub kademlia: kad::Behaviour<kad::store::MemoryStore>,
    pub identify: libp2p::identify::Behaviour,
}

/// Events emitted by the swarm for the coordinator to process.
#[derive(Debug)]
pub enum SwarmEvent {
    /// A new peer was discovered (mDNS or Kademlia).
    PeerDiscovered { peer_id: PeerId, addrs: Vec<Multiaddr> },
    /// A peer disconnected or expired.
    PeerExpired { peer_id: PeerId },
    /// Incoming sync request from a peer.
    IncomingRequest {
        peer_id: PeerId,
        request: SyncMessage,
        channel: request_response::ResponseChannel<SyncMessage>,
    },
    /// Response received from a peer for an outbound request.
    ResponseReceived {
        peer_id: PeerId,
        request_id: request_response::OutboundRequestId,
        response: SyncMessage,
    },
    /// An outbound request failed.
    OutboundFailure {
        peer_id: PeerId,
        request_id: request_response::OutboundRequestId,
        error: String,
    },
}

/// Wrapper around the libp2p Swarm with Elohim-specific helpers.
pub struct ElohimSwarm {
    swarm: Swarm<ElohimBehaviour>,
    local_peer_id: PeerId,
}

impl ElohimSwarm {
    /// Get our local peer ID.
    pub fn local_peer_id(&self) -> &PeerId {
        &self.local_peer_id
    }

    /// Add a known peer to the Kademlia routing table.
    pub fn add_peer(&mut self, peer_id: PeerId, addrs: Vec<Multiaddr>) {
        for addr in addrs {
            self.swarm
                .behaviour_mut()
                .kademlia
                .add_address(&peer_id, addr);
        }
    }

    /// Send a sync request to a peer. Returns the outbound request ID.
    pub fn send_sync_request(
        &mut self,
        peer: PeerId,
        request: SyncMessage,
    ) -> request_response::OutboundRequestId {
        self.swarm
            .behaviour_mut()
            .request_response
            .send_request(&peer, request)
    }

    /// Send a response back on a response channel.
    pub fn send_sync_response(
        &mut self,
        channel: request_response::ResponseChannel<SyncMessage>,
        response: SyncMessage,
    ) -> Result<(), SyncMessage> {
        self.swarm
            .behaviour_mut()
            .request_response
            .send_response(channel, response)
    }

    /// Run the swarm event loop, forwarding events to the coordinator.
    pub async fn run(mut self, event_tx: mpsc::Sender<SwarmEvent>) {
        use libp2p::swarm::SwarmEvent as LibSwarmEvent;

        loop {
            let Some(event) = self.swarm.next().await else {
                break;
            };

            match event {
                // mDNS discovery
                LibSwarmEvent::Behaviour(ElohimBehaviourEvent::Mdns(
                    mdns::Event::Discovered(peers),
                )) => {
                    for (peer_id, addr) in peers {
                        debug!(%peer_id, %addr, "mDNS: peer discovered");
                        self.swarm
                            .behaviour_mut()
                            .kademlia
                            .add_address(&peer_id, addr.clone());
                        let _ = event_tx
                            .send(SwarmEvent::PeerDiscovered {
                                peer_id,
                                addrs: vec![addr],
                            })
                            .await;
                    }
                }
                LibSwarmEvent::Behaviour(ElohimBehaviourEvent::Mdns(
                    mdns::Event::Expired(peers),
                )) => {
                    for (peer_id, _addr) in peers {
                        debug!(%peer_id, "mDNS: peer expired");
                        let _ = event_tx
                            .send(SwarmEvent::PeerExpired { peer_id })
                            .await;
                    }
                }

                // Request-response events
                LibSwarmEvent::Behaviour(ElohimBehaviourEvent::RequestResponse(
                    request_response::Event::Message { peer, message },
                )) => match message {
                    request_response::Message::Request {
                        request, channel, ..
                    } => {
                        debug!(%peer, "Incoming sync request");
                        let _ = event_tx
                            .send(SwarmEvent::IncomingRequest {
                                peer_id: peer,
                                request,
                                channel,
                            })
                            .await;
                    }
                    request_response::Message::Response {
                        request_id,
                        response,
                    } => {
                        debug!(%peer, "Sync response received");
                        let _ = event_tx
                            .send(SwarmEvent::ResponseReceived {
                                peer_id: peer,
                                request_id,
                                response,
                            })
                            .await;
                    }
                },
                LibSwarmEvent::Behaviour(ElohimBehaviourEvent::RequestResponse(
                    request_response::Event::OutboundFailure {
                        peer,
                        request_id,
                        error,
                    },
                )) => {
                    warn!(%peer, ?error, "Outbound sync request failed");
                    let _ = event_tx
                        .send(SwarmEvent::OutboundFailure {
                            peer_id: peer,
                            request_id,
                            error: format!("{:?}", error),
                        })
                        .await;
                }

                // Identify events (log only)
                LibSwarmEvent::Behaviour(ElohimBehaviourEvent::Identify(
                    libp2p::identify::Event::Received { peer_id, info },
                )) => {
                    debug!(
                        %peer_id,
                        agent = %info.agent_version,
                        "Identified peer"
                    );
                }

                // Kademlia events (log only for now)
                LibSwarmEvent::Behaviour(ElohimBehaviourEvent::Kademlia(event)) => {
                    debug!(?event, "Kademlia event");
                }

                // Connection events
                LibSwarmEvent::NewListenAddr { address, .. } => {
                    info!(%address, "Listening on");
                }
                LibSwarmEvent::ConnectionEstablished { peer_id, .. } => {
                    debug!(%peer_id, "Connection established");
                }
                LibSwarmEvent::ConnectionClosed { peer_id, .. } => {
                    debug!(%peer_id, "Connection closed");
                }

                _ => {}
            }
        }
    }
}

/// Build the libp2p swarm from config.
///
/// Creates or loads an Ed25519 identity keypair, configures transports,
/// and constructs the composite behaviour.
pub fn build_swarm(config: &P2PConfig, data_dir: &Path) -> Result<ElohimSwarm> {
    // Load or generate identity keypair
    let keypair = load_or_generate_keypair(data_dir)?;
    let local_peer_id = PeerId::from(keypair.public());
    info!(%local_peer_id, "Node identity");

    // Build the swarm
    let swarm = SwarmBuilder::with_existing_identity(keypair)
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )
        .context("TCP transport")?
        .with_quic()
        .with_behaviour(|key| {
            // Request-response for sync protocol
            let sync_protocol = StreamProtocol::new(super::protocols::SYNC_PROTOCOL);
            let rr_config = request_response::Config::default()
                .with_request_timeout(Duration::from_secs(30));
            let request_response = request_response::Behaviour::with_codec(
                SyncCodec,
                [(sync_protocol, request_response::ProtocolSupport::Full)],
                rr_config,
            );

            // mDNS for local peer discovery
            let mdns = mdns::tokio::Behaviour::new(
                mdns::Config::default(),
                key.public().to_peer_id(),
            )
            .expect("mDNS behaviour");

            // Kademlia DHT
            let store = kad::store::MemoryStore::new(key.public().to_peer_id());
            let mut kademlia = kad::Behaviour::new(key.public().to_peer_id(), store);
            kademlia.set_mode(Some(kad::Mode::Server));

            // Identify protocol
            let identify = libp2p::identify::Behaviour::new(
                libp2p::identify::Config::new(
                    "/elohim/id/1.0.0".to_string(),
                    key.public(),
                )
                .with_agent_version(format!("elohim-node/{}", env!("CARGO_PKG_VERSION"))),
            );

            ElohimBehaviour {
                request_response,
                mdns,
                kademlia,
                identify,
            }
        })
        .context("swarm behaviour")?
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
        .build();

    // Start listening on configured addresses
    let mut elohim_swarm = ElohimSwarm {
        swarm,
        local_peer_id,
    };

    for addr_str in &config.listen_addrs {
        let addr: Multiaddr = addr_str
            .parse()
            .with_context(|| format!("invalid listen address: {}", addr_str))?;
        elohim_swarm
            .swarm
            .listen_on(addr)
            .with_context(|| format!("failed to listen on {}", addr_str))?;
    }

    // Add bootstrap nodes to Kademlia
    for node_str in &config.bootstrap_nodes {
        if let Some((peer_id, addr)) = parse_peer_addr(node_str) {
            elohim_swarm
                .swarm
                .behaviour_mut()
                .kademlia
                .add_address(&peer_id, addr);
            info!(%peer_id, "Added bootstrap node");
        } else {
            warn!(addr = %node_str, "Invalid bootstrap node address, skipping");
        }
    }

    Ok(elohim_swarm)
}

/// Load an Ed25519 keypair from disk, or generate and persist a new one.
///
/// The keypair is stored as protobuf-encoded bytes at `{data_dir}/node_key`.
fn load_or_generate_keypair(data_dir: &Path) -> Result<identity::Keypair> {
    let key_path = data_dir.join("node_key");

    if key_path.exists() {
        let bytes = std::fs::read(&key_path).context("reading node key")?;
        let keypair =
            identity::Keypair::from_protobuf_encoding(&bytes).context("decoding node key")?;
        info!("Loaded existing node identity");
        Ok(keypair)
    } else {
        let keypair = identity::Keypair::generate_ed25519();
        // Ensure data dir exists
        std::fs::create_dir_all(data_dir).context("creating data directory")?;
        let bytes = keypair
            .to_protobuf_encoding()
            .context("encoding node key")?;
        std::fs::write(&key_path, &bytes).context("writing node key")?;
        info!("Generated new node identity");
        Ok(keypair)
    }
}

/// Parse a multiaddr string like `/ip4/1.2.3.4/tcp/4001/p2p/12D3Koo...`
/// into a (PeerId, Multiaddr) pair.
fn parse_peer_addr(addr_str: &str) -> Option<(PeerId, Multiaddr)> {
    let addr: Multiaddr = addr_str.parse().ok()?;
    // Extract PeerId from the /p2p/ component
    let peer_id = addr.iter().find_map(|p| {
        if let libp2p::multiaddr::Protocol::P2p(peer_id) = p {
            Some(peer_id)
        } else {
            None
        }
    })?;
    // Return addr without the /p2p/ suffix for Kademlia
    let addr_without_p2p: Multiaddr = addr
        .iter()
        .filter(|p| !matches!(p, libp2p::multiaddr::Protocol::P2p(_)))
        .collect();
    Some((peer_id, addr_without_p2p))
}
