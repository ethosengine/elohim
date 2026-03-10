//! Unified libp2p transport configuration
//!
//! Builds the ElohimSwarm with the unified behaviour combining:
//! - Node's doc-sync protocol (Automerge CRDT sync)
//! - Storage's shard protocol (blob transfer)
//! - Storage's storage-sync protocol (metadata CRDT sync)
//! - Shared infrastructure: Kademlia (sled-backed), mDNS, Relay, DCUtR, AutoNAT, Identify
//! - Optional: Bitswap block exchange

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use futures::StreamExt;
use libp2p::swarm::behaviour::toggle::Toggle;
use libp2p::swarm::NetworkBehaviour;
use libp2p::{
    autonat, dcutr, identity, kad, mdns, noise, relay, request_response, tcp, yamux, Multiaddr,
    PeerId, Swarm, SwarmBuilder,
};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use super::protocols::{DocSyncProtocol, SyncCodec};
use crate::config::{BitswapConfig as NodeBitswapConfig, P2PConfig};
use crate::sync::protocol::SyncMessage;

// Re-export storage protocol types for the event loop
pub use elohim_storage::p2p::kad_store::SledRecordStore;
pub use elohim_storage::p2p::shard_protocol::{
    ShardCodec, ShardProtocol, ShardRequest, ShardResponse,
};
pub use elohim_storage::p2p::sync_protocol::{
    SyncCodec as StorageSyncCodec, SyncProtocol as StorageSyncProtocol,
    SyncRequest as StorageSyncRequest, SyncResponse as StorageSyncResponse,
};

/// Combined libp2p behaviour for the unified Elohim node.
///
/// Merges node and storage protocols into a single swarm with one peer identity.
#[derive(NetworkBehaviour)]
pub struct ElohimBehaviour {
    // --- Node protocols ---
    /// Automerge document sync (node's own protocol: /elohim/doc-sync/1.0.0)
    pub doc_sync: request_response::Behaviour<SyncCodec>,

    // --- Storage protocols ---
    /// Blob shard transfer (/elohim/shard/1.0.0)
    pub shard_protocol: request_response::Behaviour<ShardCodec>,
    /// Storage CRDT sync (/elohim/storage-sync/1.0.0)
    pub storage_sync: request_response::Behaviour<StorageSyncCodec>,

    // --- Shared infrastructure ---
    /// Kademlia DHT with sled-backed persistence (from storage)
    pub kademlia: kad::Behaviour<SledRecordStore>,
    /// Local network discovery
    pub mdns: mdns::tokio::Behaviour,
    /// Relay client for NAT traversal
    pub relay_client: relay::client::Behaviour,
    /// Relay server (enabled for Server/Both modes)
    pub relay_server: Toggle<relay::Behaviour>,
    /// Direct Connection Upgrade through Relay (hole punching)
    pub dcutr: dcutr::Behaviour,
    /// Protocol identification
    pub identify: libp2p::identify::Behaviour,
    /// Automatic NAT detection
    pub autonat: autonat::Behaviour,

    // --- Bitswap (optional) ---
    /// Block exchange protocol (gated by --enable-bitswap)
    pub bitswap: Toggle<elohim_bitswap::Bitswap>,
}

/// Events emitted by the swarm for the coordinator to process.
#[derive(Debug)]
pub enum SwarmEvent {
    // --- Node events ---
    /// A new peer was discovered (mDNS or Kademlia).
    PeerDiscovered {
        peer_id: PeerId,
        #[allow(dead_code)]
        addrs: Vec<Multiaddr>,
    },
    /// A peer disconnected or expired.
    PeerExpired { peer_id: PeerId },
    /// Incoming doc-sync request from a peer.
    IncomingRequest {
        peer_id: PeerId,
        request: SyncMessage,
        channel: request_response::ResponseChannel<SyncMessage>,
    },
    /// Response received for an outbound doc-sync request.
    ResponseReceived {
        peer_id: PeerId,
        #[allow(dead_code)]
        request_id: request_response::OutboundRequestId,
        response: SyncMessage,
    },
    /// An outbound doc-sync request failed.
    OutboundFailure {
        peer_id: PeerId,
        #[allow(dead_code)]
        request_id: request_response::OutboundRequestId,
        error: String,
    },

    // --- Storage events ---
    /// Incoming shard request from a peer.
    #[allow(dead_code)]
    ShardRequest {
        peer_id: PeerId,
        request: ShardRequest,
        channel: request_response::ResponseChannel<ShardResponse>,
    },
    /// Incoming storage-sync request from a peer.
    #[allow(dead_code)]
    StorageSyncRequest {
        peer_id: PeerId,
        request: StorageSyncRequest,
        channel: request_response::ResponseChannel<StorageSyncResponse>,
    },

    // --- Bitswap events ---
    /// Bitswap progress on a sync query.
    #[allow(dead_code)]
    BitswapProgress {
        query_id: elohim_bitswap::QueryId,
        missing: usize,
    },
    /// Bitswap query completed.
    #[allow(dead_code)]
    BitswapComplete {
        query_id: elohim_bitswap::QueryId,
        result: Result<(), String>,
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
    #[allow(dead_code)]
    pub fn add_peer(&mut self, peer_id: PeerId, addrs: Vec<Multiaddr>) {
        for addr in addrs {
            self.swarm
                .behaviour_mut()
                .kademlia
                .add_address(&peer_id, addr);
        }
    }

    /// Send a doc-sync request to a peer.
    #[allow(dead_code)]
    pub fn send_sync_request(
        &mut self,
        peer: PeerId,
        request: SyncMessage,
    ) -> request_response::OutboundRequestId {
        self.swarm
            .behaviour_mut()
            .doc_sync
            .send_request(&peer, request)
    }

    /// Send a doc-sync response.
    #[allow(dead_code)]
    pub fn send_sync_response(
        &mut self,
        channel: request_response::ResponseChannel<SyncMessage>,
        response: SyncMessage,
    ) -> Result<(), SyncMessage> {
        self.swarm
            .behaviour_mut()
            .doc_sync
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
                LibSwarmEvent::Behaviour(ElohimBehaviourEvent::Mdns(mdns::Event::Discovered(
                    peers,
                ))) => {
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
                LibSwarmEvent::Behaviour(ElohimBehaviourEvent::Mdns(mdns::Event::Expired(
                    peers,
                ))) => {
                    for (peer_id, _addr) in peers {
                        debug!(%peer_id, "mDNS: peer expired");
                        let _ = event_tx.send(SwarmEvent::PeerExpired { peer_id }).await;
                    }
                }

                // Doc-sync events (node protocol)
                LibSwarmEvent::Behaviour(ElohimBehaviourEvent::DocSync(
                    request_response::Event::Message { peer, message },
                )) => match message {
                    request_response::Message::Request {
                        request, channel, ..
                    } => {
                        debug!(%peer, "Incoming doc-sync request");
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
                        debug!(%peer, "Doc-sync response received");
                        let _ = event_tx
                            .send(SwarmEvent::ResponseReceived {
                                peer_id: peer,
                                request_id,
                                response,
                            })
                            .await;
                    }
                },
                LibSwarmEvent::Behaviour(ElohimBehaviourEvent::DocSync(
                    request_response::Event::OutboundFailure {
                        peer,
                        request_id,
                        error,
                    },
                )) => {
                    warn!(%peer, ?error, "Outbound doc-sync request failed");
                    let _ = event_tx
                        .send(SwarmEvent::OutboundFailure {
                            peer_id: peer,
                            request_id,
                            error: format!("{:?}", error),
                        })
                        .await;
                }

                // Shard protocol events (storage protocol)
                LibSwarmEvent::Behaviour(ElohimBehaviourEvent::ShardProtocol(
                    request_response::Event::Message {
                        peer,
                        message:
                            request_response::Message::Request {
                                request, channel, ..
                            },
                    },
                )) => {
                    debug!(%peer, "Incoming shard request");
                    let _ = event_tx
                        .send(SwarmEvent::ShardRequest {
                            peer_id: peer,
                            request,
                            channel,
                        })
                        .await;
                }

                // Storage-sync events (storage protocol)
                LibSwarmEvent::Behaviour(ElohimBehaviourEvent::StorageSync(
                    request_response::Event::Message {
                        peer,
                        message:
                            request_response::Message::Request {
                                request, channel, ..
                            },
                    },
                )) => {
                    debug!(%peer, "Incoming storage-sync request");
                    let _ = event_tx
                        .send(SwarmEvent::StorageSyncRequest {
                            peer_id: peer,
                            request,
                            channel,
                        })
                        .await;
                }

                // Bitswap events
                LibSwarmEvent::Behaviour(ElohimBehaviourEvent::Bitswap(
                    elohim_bitswap::BitswapEvent::Progress(query_id, missing),
                )) => {
                    debug!(%query_id, missing, "Bitswap progress");
                    let _ = event_tx
                        .send(SwarmEvent::BitswapProgress { query_id, missing })
                        .await;
                }
                LibSwarmEvent::Behaviour(ElohimBehaviourEvent::Bitswap(
                    elohim_bitswap::BitswapEvent::Complete(query_id, result),
                )) => {
                    debug!(%query_id, ok = result.is_ok(), "Bitswap complete");
                    let _ = event_tx
                        .send(SwarmEvent::BitswapComplete {
                            query_id,
                            result: result.map_err(|e| e.to_string()),
                        })
                        .await;
                }

                // Identify events
                LibSwarmEvent::Behaviour(ElohimBehaviourEvent::Identify(
                    libp2p::identify::Event::Received { peer_id, info, .. },
                )) => {
                    debug!(
                        %peer_id,
                        agent = %info.agent_version,
                        "Identified peer"
                    );
                }

                // Kademlia events
                LibSwarmEvent::Behaviour(ElohimBehaviourEvent::Kademlia(event)) => {
                    debug!(?event, "Kademlia event");
                }

                // Relay events
                LibSwarmEvent::Behaviour(ElohimBehaviourEvent::RelayClient(event)) => {
                    debug!(?event, "Relay client event");
                }
                LibSwarmEvent::Behaviour(ElohimBehaviourEvent::RelayServer(event)) => {
                    debug!(?event, "Relay server event");
                }

                // DCUtR events
                LibSwarmEvent::Behaviour(ElohimBehaviourEvent::Dcutr(event)) => {
                    debug!(?event, "DCUtR event");
                }

                // AutoNAT events
                LibSwarmEvent::Behaviour(ElohimBehaviourEvent::Autonat(event)) => {
                    debug!(?event, "AutoNAT event");
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

/// Build the unified libp2p swarm.
///
/// Creates or loads an Ed25519 identity keypair, configures transports,
/// and constructs the unified behaviour combining node + storage protocols.
pub fn build_swarm(
    config: &P2PConfig,
    data_dir: &Path,
    blob_store: Arc<elohim_storage::BlobStore>,
    bitswap_config: &NodeBitswapConfig,
) -> Result<ElohimSwarm> {
    // Load or generate identity keypair
    let keypair = load_or_generate_keypair(data_dir)?;
    let local_peer_id = PeerId::from(keypair.public());
    info!(%local_peer_id, "Node identity");

    // Open shared sled database for Kademlia + DocStore persistence
    let sled_path = data_dir.join("sync.sled");
    let sled_db = sled::open(&sled_path)
        .with_context(|| format!("opening sled database at {}", sled_path.display()))?;

    // Clone keypair for behaviour closure
    let _keypair_clone = keypair.clone();
    let sled_db_clone = sled_db;
    let bitswap_enabled = bitswap_config.enabled;
    let bitswap_timeout = Duration::from_secs(bitswap_config.request_timeout_secs);
    let blob_store_clone = blob_store;

    // Build swarm with relay client support for NAT traversal
    let swarm = SwarmBuilder::with_existing_identity(keypair)
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )
        .context("TCP transport")?
        .with_quic()
        .with_relay_client(noise::Config::new, yamux::Config::default)
        .context("relay client")?
        .with_behaviour(|key, relay_client| {
            // --- Node protocols ---
            let doc_sync_config =
                request_response::Config::default().with_request_timeout(Duration::from_secs(30));
            let doc_sync = request_response::Behaviour::new(
                [(DocSyncProtocol, request_response::ProtocolSupport::Full)],
                doc_sync_config,
            );

            // --- Storage protocols ---
            let shard_config =
                request_response::Config::default().with_request_timeout(Duration::from_secs(60));
            let shard_protocol = request_response::Behaviour::new(
                [(ShardProtocol, request_response::ProtocolSupport::Full)],
                shard_config,
            );

            let storage_sync_config =
                request_response::Config::default().with_request_timeout(Duration::from_secs(30));
            let storage_sync = request_response::Behaviour::new(
                [(StorageSyncProtocol, request_response::ProtocolSupport::Full)],
                storage_sync_config,
            );

            // --- Shared infrastructure ---
            let kad_store = SledRecordStore::from_db(sled_db_clone)
                .expect("Failed to open sled Kademlia store");
            let kademlia = kad::Behaviour::new(key.public().to_peer_id(), kad_store);

            let mdns =
                mdns::tokio::Behaviour::new(mdns::Config::default(), key.public().to_peer_id())
                    .expect("mDNS behaviour");

            // Relay server disabled by default (enable for doorway/edgenode deployments)
            let relay_server = Toggle::from(None);

            let dcutr = dcutr::Behaviour::new(key.public().to_peer_id());

            let identify = libp2p::identify::Behaviour::new(
                libp2p::identify::Config::new("/elohim/id/1.0.0".to_string(), key.public())
                    .with_agent_version(format!("elohim-node/{}", env!("CARGO_PKG_VERSION"))),
            );

            let autonat = autonat::Behaviour::new(
                key.public().to_peer_id(),
                autonat::Config {
                    boot_delay: Duration::from_secs(15),
                    ..Default::default()
                },
            );

            // --- Bitswap (optional) ---
            let bitswap = if bitswap_enabled {
                let store = crate::storage::blobs::BlobManager::new(blob_store_clone);
                let bs = elohim_bitswap::Bitswap::new(
                    elohim_bitswap::BitswapConfig {
                        request_timeout: bitswap_timeout,
                    },
                    store,
                    Box::new(|fut| {
                        tokio::spawn(fut);
                    }),
                );
                Toggle::from(Some(bs))
            } else {
                Toggle::from(None)
            };

            ElohimBehaviour {
                doc_sync,
                shard_protocol,
                storage_sync,
                kademlia,
                mdns,
                relay_client,
                relay_server,
                dcutr,
                identify,
                autonat,
                bitswap,
            }
        })
        .context("swarm behaviour")?
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
        .build();

    let mut elohim_swarm = ElohimSwarm {
        swarm,
        local_peer_id,
    };

    // Start listening on configured addresses
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
        std::fs::create_dir_all(data_dir).context("creating data directory")?;
        let bytes = keypair
            .to_protobuf_encoding()
            .context("encoding node key")?;
        std::fs::write(&key_path, &bytes).context("writing node key")?;
        info!("Generated new node identity");
        Ok(keypair)
    }
}

/// Parse a multiaddr like `/ip4/1.2.3.4/tcp/4001/p2p/12D3Koo...` into (PeerId, Multiaddr).
fn parse_peer_addr(addr_str: &str) -> Option<(PeerId, Multiaddr)> {
    let addr: Multiaddr = addr_str.parse().ok()?;
    let peer_id = addr.iter().find_map(|p| {
        if let libp2p::multiaddr::Protocol::P2p(peer_id) = p {
            Some(peer_id)
        } else {
            None
        }
    })?;
    let addr_without_p2p: Multiaddr = addr
        .iter()
        .filter(|p| !matches!(p, libp2p::multiaddr::Protocol::P2p(_)))
        .collect();
    Some((peer_id, addr_without_p2p))
}
