//! P2P Network Module - rust-libp2p integration for shard transfer
//!
//! This module provides the P2P networking layer for elohim-storage,
//! enabling direct node-to-node shard transfer using rust-libp2p.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                     ElohimStorageBehaviour                       │
//! ├─────────────────────────────────────────────────────────────────┤
//! │  Kademlia        - DHT for content routing                      │
//! │  request_response - Shard transfer protocol                     │
//! │  mDNS            - Local network discovery                      │
//! │  relay           - NAT traversal                                │
//! │  dcutr           - Direct Connection Upgrade                    │
//! │  identify        - Protocol identification                      │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Usage
//!
//! ```ignore
//! use elohim_storage::p2p::{P2PNode, P2PConfig};
//!
//! let config = P2PConfig::default();
//! let node = P2PNode::new(identity, config).await?;
//! node.start().await?;
//! ```

pub mod behaviour;
pub mod kad_store;
pub mod shard_protocol;
pub mod sync_protocol;

use futures::StreamExt;
use libp2p::{
    autonat, dcutr, identify,
    kad,
    mdns, noise,
    multiaddr::Protocol,
    relay, request_response,
    swarm::{Swarm, SwarmEvent},
    tcp, yamux, Multiaddr, PeerId, SwarmBuilder,
};
use serde::Serialize;
use sha2::{Sha256, Digest};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, error, info, warn};

use crate::blob_store::BlobStore;
use crate::error::StorageError;
use crate::identity::NodeIdentity;
use crate::sync::{DocStore, DocStoreConfig, SyncManager, StreamTracker};

pub use behaviour::{ElohimStorageBehaviour, RelayMode};
pub use shard_protocol::{ShardCodec, ShardProtocol, ShardRequest, ShardResponse};
pub use sync_protocol::{SyncCodec, SyncProtocol, SyncRequest, SyncResponse, DocumentInfo};

/// Configuration for P2P node
#[derive(Debug, Clone)]
pub struct P2PConfig {
    /// Listen addresses (e.g., "/ip4/0.0.0.0/tcp/0")
    pub listen_addresses: Vec<String>,
    /// Bootstrap nodes for initial DHT population
    pub bootstrap_nodes: Vec<String>,
    /// Enable mDNS for local discovery
    pub enable_mdns: bool,
    /// Kademlia replication factor
    pub kad_replication: u8,
    /// Request timeout
    pub request_timeout: Duration,
    /// Storage directory for sync databases
    pub storage_dir: std::path::PathBuf,
    /// Relay mode: Client (desktop steward), Server (K8s pod), Both (doorway host)
    pub relay_mode: RelayMode,
    /// Addresses to announce to the network (e.g., public IP/DNS multiaddrs)
    pub announce_addresses: Vec<String>,
}

impl Default for P2PConfig {
    fn default() -> Self {
        Self {
            listen_addresses: vec!["/ip4/0.0.0.0/tcp/0".to_string()],
            bootstrap_nodes: Vec::new(),
            enable_mdns: true,
            kad_replication: 4,
            request_timeout: Duration::from_secs(30),
            storage_dir: dirs::data_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("elohim-storage"),
            relay_mode: RelayMode::default(),
            announce_addresses: Vec::new(),
        }
    }
}

/// P2P Node for elohim-storage
pub struct P2PNode {
    /// Node identity
    identity: NodeIdentity,
    /// Configuration
    config: P2PConfig,
    /// libp2p Swarm (wrapped for async access)
    swarm: Arc<RwLock<Swarm<ElohimStorageBehaviour>>>,
    /// Blob store for serving shard requests
    blob_store: Arc<BlobStore>,
    /// Sync manager for CRDT document exchange
    sync_manager: Arc<SyncManager>,
    /// Shutdown signal
    shutdown_tx: broadcast::Sender<()>,
    /// Status broadcast for HttpServer handle
    status_tx: tokio::sync::watch::Sender<P2PStatusInfo>,
    /// Current NAT status (updated by autonat events)
    nat_status: Arc<RwLock<String>>,
    /// Active relay reservation count
    relay_reservations: Arc<std::sync::atomic::AtomicUsize>,
}

/// P2P node status for observability
#[derive(Debug, Clone, Serialize)]
pub struct P2PStatusInfo {
    pub peer_id: String,
    pub listen_addresses: Vec<String>,
    pub connected_peers: usize,
    pub bootstrap_nodes: Vec<String>,
    pub sync_documents: usize,
    /// NAT status detected by autonat: "Unknown", "Public", "Private"
    pub nat_status: String,
    /// Number of active relay reservations
    pub relay_reservations: usize,
    /// Addresses announced to the network
    pub announce_addresses: Vec<String>,
    /// Relay mode this node is running in
    pub relay_mode: String,
}

/// Send+Sync handle for querying P2P status from HttpServer.
/// Separated from P2PNode because libp2p Swarm types are not Send.
#[derive(Clone)]
pub struct P2PHandle {
    status_rx: tokio::sync::watch::Receiver<P2PStatusInfo>,
}

impl P2PHandle {
    /// Get the latest P2P status snapshot
    pub fn status(&self) -> P2PStatusInfo {
        self.status_rx.borrow().clone()
    }
}

impl P2PNode {
    /// Create a new P2P node
    pub async fn new(
        identity: NodeIdentity,
        config: P2PConfig,
        blob_store: Arc<BlobStore>,
    ) -> Result<Self, StorageError> {
        let keypair = identity.keypair().clone();
        let peer_id = *identity.peer_id();

        // Build swarm with relay client transport for NAT traversal
        let swarm = SwarmBuilder::with_existing_identity(keypair)
            .with_tokio()
            .with_tcp(
                tcp::Config::default(),
                noise::Config::new,
                yamux::Config::default,
            )
            .map_err(|e| StorageError::P2PNetwork(format!("Transport error: {}", e)))?
            .with_relay_client(noise::Config::new, yamux::Config::default)
            .map_err(|e| StorageError::P2PNetwork(format!("Relay client error: {}", e)))?
            .with_behaviour(|key, relay_client| {
                ElohimStorageBehaviour::new(key.clone(), config.clone(), relay_client)
            })
            .map_err(|e| StorageError::P2PNetwork(format!("Behaviour error: {}", e)))?
            .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
            .build();

        // Initialize sync infrastructure
        let doc_store_config = DocStoreConfig {
            db_path: config.storage_dir.join("sync.sled"),
            ..Default::default()
        };
        let doc_store = Arc::new(DocStore::new(doc_store_config).await?);
        let stream_tracker = Arc::new(StreamTracker::new());
        let sync_manager = Arc::new(SyncManager::new(doc_store, stream_tracker));

        let (shutdown_tx, _) = broadcast::channel(1);

        let initial_status = P2PStatusInfo {
            peer_id: peer_id.to_string(),
            listen_addresses: vec![],
            connected_peers: 0,
            bootstrap_nodes: config.bootstrap_nodes.clone(),
            sync_documents: 0,
            nat_status: "Unknown".to_string(),
            relay_reservations: 0,
            announce_addresses: config.announce_addresses.clone(),
            relay_mode: config.relay_mode.to_string(),
        };
        let (status_tx, _) = tokio::sync::watch::channel(initial_status);

        info!(peer_id = %peer_id, relay_mode = %config.relay_mode, "Created P2P node with NAT traversal");

        Ok(Self {
            identity,
            config,
            swarm: Arc::new(RwLock::new(swarm)),
            blob_store,
            sync_manager,
            shutdown_tx,
            status_tx,
            nat_status: Arc::new(RwLock::new("Unknown".to_string())),
            relay_reservations: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        })
    }

    /// Get the local PeerId
    pub fn peer_id(&self) -> &PeerId {
        self.identity.peer_id()
    }

    /// Start listening and event loop
    pub async fn start(&self) -> Result<(), StorageError> {
        let mut swarm = self.swarm.write().await;

        // Listen on configured addresses
        for addr in &self.config.listen_addresses {
            let multiaddr: Multiaddr = addr
                .parse()
                .map_err(|e| StorageError::P2PNetwork(format!("Invalid address: {}", e)))?;

            swarm
                .listen_on(multiaddr)
                .map_err(|e| StorageError::P2PNetwork(format!("Listen error: {}", e)))?;
        }

        // Add external addresses for announcement (e.g., public IP, DNS names)
        for addr_str in &self.config.announce_addresses {
            match addr_str.parse::<Multiaddr>() {
                Ok(addr) => {
                    swarm.add_external_address(addr.clone());
                    info!(address = %addr, "Added external announce address");
                }
                Err(e) => {
                    warn!("Invalid announce address '{}': {}", addr_str, e);
                }
            }
        }

        // Dial bootstrap nodes
        for addr_str in &self.config.bootstrap_nodes {
            let addr: Multiaddr = match addr_str.parse() {
                Ok(a) => a,
                Err(e) => {
                    warn!("Invalid bootstrap multiaddr '{}': {}", addr_str, e);
                    continue;
                }
            };

            // Extract PeerId from multiaddr if present
            let peer_id = addr.iter().find_map(|p| {
                if let Protocol::P2p(peer_id) = p {
                    Some(peer_id)
                } else {
                    None
                }
            });

            match swarm.dial(addr.clone()) {
                Ok(_) => {
                    info!("Dialing bootstrap node: {}", addr);
                    if let Some(peer_id) = peer_id {
                        swarm.behaviour_mut().kademlia.add_address(&peer_id, addr);
                    }
                }
                Err(e) => {
                    warn!("Failed to dial bootstrap node {}: {}", addr, e);
                }
            }
        }

        info!("P2P node started");
        Ok(())
    }

    /// Run the event loop (call in background task)
    pub async fn run(&self, mut shutdown: broadcast::Receiver<()>) {
        // Initial status snapshot after start
        self.refresh_status().await;

        let mut status_interval = tokio::time::interval(Duration::from_secs(30));
        let mut sync_interval = tokio::time::interval(Duration::from_secs(60));

        loop {
            let mut swarm = self.swarm.write().await;

            tokio::select! {
                event = swarm.select_next_some() => {
                    drop(swarm); // Release write lock before handling
                    self.handle_event(event).await;
                }
                _ = status_interval.tick() => {
                    drop(swarm);
                    self.refresh_status().await;
                }
                _ = sync_interval.tick() => {
                    drop(swarm);
                    self.initiate_sync_round().await;
                }
                _ = shutdown.recv() => {
                    info!("P2P node shutting down");
                    break;
                }
            }
        }
    }

    /// Handle a swarm event
    async fn handle_event(&self, event: SwarmEvent<behaviour::ElohimStorageBehaviourEvent>) {
        match event {
            SwarmEvent::NewListenAddr { address, .. } => {
                info!(address = %address, "Listening on");
            }
            SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } => {
                debug!(peer = %peer_id, "Connected to peer");
                // In K8s (mDNS disabled), add connected peers to Kademlia for DHT routing.
                // With mDNS enabled, discovery handles this (line 362-368). Without mDNS,
                // peers dialed via DNS bootstrap connect but aren't added to Kademlia
                // because DNS multiaddrs lack PeerIDs at deploy time.
                if !self.config.enable_mdns {
                    let addr = endpoint.get_remote_address().clone();
                    let mut swarm = self.swarm.write().await;
                    swarm.behaviour_mut().kademlia.add_address(&peer_id, addr.clone());
                    info!(peer = %peer_id, addr = %addr, "Added peer to Kademlia (bootstrap connection)");
                }
                self.refresh_status().await;
            }
            SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
                debug!(peer = %peer_id, cause = ?cause, "Disconnected from peer");
                self.refresh_status().await;
            }
            SwarmEvent::Behaviour(event) => {
                self.handle_behaviour_event(event).await;
            }
            _ => {}
        }
    }

    /// Handle behaviour-specific events
    async fn handle_behaviour_event(&self, event: behaviour::ElohimStorageBehaviourEvent) {
        match event {
            behaviour::ElohimStorageBehaviourEvent::ShardProtocol(
                request_response::Event::Message { peer, message },
            ) => {
                match message {
                    request_response::Message::Request {
                        request,
                        channel,
                        ..
                    } => {
                        debug!(peer = %peer, request = ?request, "Received shard request");
                        let response = self.handle_shard_request(request).await;

                        // Send response
                        let mut swarm = self.swarm.write().await;
                        if let Err(e) = swarm.behaviour_mut().shard_protocol.send_response(channel, response) {
                            warn!(peer = %peer, error = ?e, "Failed to send shard response");
                        }
                    }
                    request_response::Message::Response {
                        request_id,
                        response,
                    } => {
                        debug!(request_id = ?request_id, response = ?response, "Received shard response");
                        // Response handling would go here for outbound requests
                    }
                }
            }
            behaviour::ElohimStorageBehaviourEvent::ShardProtocol(
                request_response::Event::OutboundFailure {
                    peer,
                    request_id,
                    error,
                },
            ) => {
                warn!(peer = %peer, request_id = ?request_id, error = ?error, "Outbound shard request failed");
            }
            behaviour::ElohimStorageBehaviourEvent::ShardProtocol(
                request_response::Event::InboundFailure {
                    peer,
                    request_id,
                    error,
                },
            ) => {
                warn!(peer = %peer, request_id = ?request_id, error = ?error, "Inbound shard request failed");
            }
            behaviour::ElohimStorageBehaviourEvent::ShardProtocol(
                request_response::Event::ResponseSent { peer, request_id },
            ) => {
                debug!(peer = %peer, request_id = ?request_id, "Shard response sent");
            }
            behaviour::ElohimStorageBehaviourEvent::Kademlia(kad::Event::RoutingUpdated {
                peer,
                is_new_peer,
                ..
            }) => {
                if is_new_peer {
                    debug!(peer = %peer, "New peer added to Kademlia routing table");
                }
            }
            behaviour::ElohimStorageBehaviourEvent::Kademlia(event) => {
                debug!(event = ?event, "Kademlia event");
            }
            behaviour::ElohimStorageBehaviourEvent::Mdns(mdns::Event::Discovered(peers)) => {
                let mut swarm = self.swarm.write().await;
                for (peer_id, addr) in peers {
                    info!(peer = %peer_id, addr = %addr, "mDNS: discovered peer");
                    // Add peer to Kademlia routing table
                    swarm.behaviour_mut().kademlia.add_address(&peer_id, addr);
                }
            }
            behaviour::ElohimStorageBehaviourEvent::Mdns(mdns::Event::Expired(peers)) => {
                for (peer_id, _addr) in peers {
                    debug!(peer = %peer_id, "mDNS: peer expired");
                }
            }
            // Sync protocol events
            behaviour::ElohimStorageBehaviourEvent::SyncProtocol(
                request_response::Event::Message { peer, message },
            ) => {
                match message {
                    request_response::Message::Request {
                        request,
                        channel,
                        ..
                    } => {
                        debug!(peer = %peer, request = ?request, "Received sync request");
                        let response = self.handle_sync_request(request).await;
                        let mut swarm = self.swarm.write().await;
                        if let Err(e) = swarm.behaviour_mut().sync_protocol.send_response(channel, response) {
                            warn!(peer = %peer, error = ?e, "Failed to send sync response");
                        }
                    }
                    request_response::Message::Response {
                        request_id,
                        response,
                    } => {
                        self.handle_sync_response(peer, request_id, response).await;
                    }
                }
            }
            behaviour::ElohimStorageBehaviourEvent::SyncProtocol(
                request_response::Event::OutboundFailure {
                    peer,
                    request_id,
                    error,
                },
            ) => {
                warn!(peer = %peer, request_id = ?request_id, error = ?error, "Outbound sync request failed");
            }
            behaviour::ElohimStorageBehaviourEvent::SyncProtocol(
                request_response::Event::InboundFailure {
                    peer,
                    request_id,
                    error,
                },
            ) => {
                warn!(peer = %peer, request_id = ?request_id, error = ?error, "Inbound sync request failed");
            }
            behaviour::ElohimStorageBehaviourEvent::SyncProtocol(
                request_response::Event::ResponseSent { peer, request_id },
            ) => {
                debug!(peer = %peer, request_id = ?request_id, "Sync response sent");
            }

            // === NAT traversal events ===

            behaviour::ElohimStorageBehaviourEvent::Identify(identify::Event::Received {
                peer_id,
                info,
                ..
            }) => {
                info!(
                    peer = %peer_id,
                    agent = %info.agent_version,
                    protocols = ?info.protocols.len(),
                    "Identify: received peer info"
                );
                // Add observed addresses to Kademlia for better routing
                let mut swarm = self.swarm.write().await;
                for addr in info.listen_addrs {
                    swarm.behaviour_mut().kademlia.add_address(&peer_id, addr);
                }
            }
            behaviour::ElohimStorageBehaviourEvent::Identify(identify::Event::Sent { peer_id, .. }) => {
                debug!(peer = %peer_id, "Identify: sent our info to peer");
            }
            behaviour::ElohimStorageBehaviourEvent::Identify(event) => {
                debug!(event = ?event, "Identify event");
            }

            behaviour::ElohimStorageBehaviourEvent::AutoNat(autonat::Event::StatusChanged {
                old,
                new,
            }) => {
                let status_str = match &new {
                    autonat::NatStatus::Public(_addr) => "Public",
                    autonat::NatStatus::Private => "Private",
                    autonat::NatStatus::Unknown => "Unknown",
                };
                info!(
                    old = ?old,
                    new = ?new,
                    "AutoNAT: NAT status changed to {}",
                    status_str
                );
                *self.nat_status.write().await = status_str.to_string();
            }
            behaviour::ElohimStorageBehaviourEvent::AutoNat(event) => {
                debug!(event = ?event, "AutoNAT event");
            }

            behaviour::ElohimStorageBehaviourEvent::RelayClient(
                relay::client::Event::ReservationReqAccepted { relay_peer_id, renewal, .. },
            ) => {
                if renewal {
                    debug!(relay = %relay_peer_id, "Relay: reservation renewed");
                } else {
                    info!(relay = %relay_peer_id, "Relay: reservation accepted");
                    self.relay_reservations.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
            }
            behaviour::ElohimStorageBehaviourEvent::RelayClient(event) => {
                debug!(event = ?event, "Relay client event");
            }

            behaviour::ElohimStorageBehaviourEvent::RelayServer(event) => {
                debug!(event = ?event, "Relay server event");
            }

            behaviour::ElohimStorageBehaviourEvent::Dcutr(dcutr::Event {
                remote_peer_id,
                result,
            }) => {
                match result {
                    Ok(_) => info!(peer = %remote_peer_id, "DCUtR: direct connection upgraded"),
                    Err(ref e) => debug!(peer = %remote_peer_id, error = %e, "DCUtR: upgrade failed"),
                }
            }
        }
    }

    /// Handle an incoming shard request
    async fn handle_shard_request(&self, request: ShardRequest) -> ShardResponse {
        match request {
            ShardRequest::Get { hash } => {
                debug!(hash = %hash, "Handling shard Get request");
                match self.blob_store.get(&hash).await {
                    Ok(data) => {
                        info!(hash = %hash, size = data.len(), "Serving shard");
                        ShardResponse::Data(data)
                    }
                    Err(_) => {
                        debug!(hash = %hash, "Shard not found");
                        ShardResponse::NotFound
                    }
                }
            }
            ShardRequest::Have { hash } => {
                debug!(hash = %hash, "Handling shard Have request");
                let exists = self.blob_store.exists(&hash).await;
                ShardResponse::Have(exists)
            }
            ShardRequest::Push { hash, data } => {
                debug!(hash = %hash, size = data.len(), "Handling shard Push request");
                match self.blob_store.store(&data).await {
                    Ok(result) => {
                        if result.hash == hash {
                            info!(hash = %hash, "Shard stored via P2P push");
                            ShardResponse::PushAck
                        } else {
                            warn!(expected = %hash, actual = %result.hash, "Shard hash mismatch");
                            ShardResponse::Error("Hash mismatch".to_string())
                        }
                    }
                    Err(e) => {
                        error!(hash = %hash, error = %e, "Failed to store shard");
                        ShardResponse::Error(format!("Storage error: {}", e))
                    }
                }
            }
        }
    }

    /// Handle an incoming sync request
    async fn handle_sync_request(&self, request: SyncRequest) -> SyncResponse {
        match request {
            SyncRequest::GetHeads { app_id, doc_id } => {
                debug!(app_id = %app_id, doc_id = %doc_id, "Handling GetHeads request");
                match self.sync_manager.get_heads(&app_id, &doc_id).await {
                    Ok(heads) => {
                        // Get change count from doc store
                        let change_count = match self.sync_manager.list_documents(&app_id, Some(&doc_id), 0, 1).await {
                            Ok((docs, _)) => docs.first().map(|d| d.change_count).unwrap_or(0),
                            Err(_) => 0,
                        };
                        SyncResponse::Heads {
                            app_id,
                            doc_id,
                            heads,
                            change_count,
                        }
                    }
                    Err(e) => {
                        warn!(app_id = %app_id, doc_id = %doc_id, error = %e, "Failed to get heads");
                        SyncResponse::Error {
                            message: format!("Failed to get heads: {}", e),
                        }
                    }
                }
            }
            SyncRequest::SyncChanges {
                app_id,
                doc_id,
                have_heads,
                bloom_filter: _, // TODO: Use bloom filter for optimization
            } => {
                debug!(app_id = %app_id, doc_id = %doc_id, have_heads = ?have_heads, "Handling SyncChanges request");
                match self.sync_manager.get_changes_since(&app_id, &doc_id, &have_heads).await {
                    Ok((changes, new_heads)) => {
                        info!(app_id = %app_id, doc_id = %doc_id, changes_count = changes.len(), "Sending changes");
                        SyncResponse::Changes {
                            app_id,
                            doc_id,
                            changes,
                            has_more: false, // TODO: Implement pagination
                            new_heads,
                        }
                    }
                    Err(e) => {
                        warn!(app_id = %app_id, doc_id = %doc_id, error = %e, "Failed to get changes");
                        SyncResponse::Error {
                            message: format!("Failed to get changes: {}", e),
                        }
                    }
                }
            }
            SyncRequest::GetChanges {
                app_id,
                doc_id,
                change_hashes,
            } => {
                debug!(app_id = %app_id, doc_id = %doc_id, change_hashes = ?change_hashes, "Handling GetChanges request");
                // For now, return all changes since empty heads (full sync)
                // TODO: Implement selective change fetching by hash
                match self.sync_manager.get_changes_since(&app_id, &doc_id, &[]).await {
                    Ok((changes, _)) => {
                        let changes_with_hashes: Vec<(String, Vec<u8>)> = changes
                            .into_iter()
                            .map(|c| {
                                let mut hasher = Sha256::new();
                                hasher.update(&c);
                                let result = hasher.finalize();
                                let hash = hex::encode(&result[..8]);
                                (hash, c)
                            })
                            .collect();
                        SyncResponse::RequestedChanges {
                            app_id,
                            doc_id,
                            changes: changes_with_hashes,
                            not_found: vec![],
                        }
                    }
                    Err(e) => {
                        warn!(app_id = %app_id, doc_id = %doc_id, error = %e, "Failed to get requested changes");
                        SyncResponse::Error {
                            message: format!("Failed to get changes: {}", e),
                        }
                    }
                }
            }
            SyncRequest::AnnounceChange {
                app_id,
                doc_id,
                change_hash: _,
                change_data,
            } => {
                debug!(app_id = %app_id, doc_id = %doc_id, "Handling AnnounceChange request");
                if let Some(data) = change_data {
                    match self.sync_manager.apply_changes(&app_id, &doc_id, vec![data]).await {
                        Ok(_) => {
                            info!(app_id = %app_id, doc_id = %doc_id, "Applied announced change");
                            SyncResponse::ChangeAck {
                                app_id,
                                doc_id,
                                was_new: true,
                            }
                        }
                        Err(e) => {
                            warn!(app_id = %app_id, doc_id = %doc_id, error = %e, "Failed to apply change");
                            SyncResponse::Error {
                                message: format!("Failed to apply change: {}", e),
                            }
                        }
                    }
                } else {
                    // Just an announcement, we'd need to request the change
                    SyncResponse::ChangeAck {
                        app_id,
                        doc_id,
                        was_new: false,
                    }
                }
            }
            SyncRequest::ListDocuments {
                app_id,
                prefix,
                offset,
                limit,
            } => {
                debug!(app_id = %app_id, prefix = ?prefix, offset = offset, limit = limit, "Handling ListDocuments request");
                match self.sync_manager.list_documents(&app_id, prefix.as_deref(), offset, limit).await {
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
                            app_id,
                            documents,
                            total,
                            has_more,
                        }
                    }
                    Err(e) => {
                        warn!(app_id = %app_id, error = %e, "Failed to list documents");
                        SyncResponse::Error {
                            message: format!("Failed to list documents: {}", e),
                        }
                    }
                }
            }
        }
    }

    /// Handle an outbound sync response from a peer.
    /// Called when we receive responses to our sync requests (e.g., from initiate_sync_round).
    async fn handle_sync_response(
        &self,
        peer: PeerId,
        request_id: request_response::OutboundRequestId,
        response: SyncResponse,
    ) {
        match response {
            SyncResponse::DocumentList { app_id, documents, total, .. } => {
                debug!(
                    peer = %peer, app_id = %app_id, doc_count = documents.len(),
                    total = total, "Received document list from peer"
                );

                // Compare with local documents and request changes for diverged ones
                for remote_doc in &documents {
                    match self.sync_manager.get_heads(&app_id, &remote_doc.doc_id).await {
                        Ok(local_heads) => {
                            if local_heads != remote_doc.heads {
                                // Heads differ — request changes from this peer
                                let sync_request = SyncRequest::SyncChanges {
                                    app_id: app_id.clone(),
                                    doc_id: remote_doc.doc_id.clone(),
                                    have_heads: local_heads,
                                    bloom_filter: None,
                                };
                                let mut swarm = self.swarm.write().await;
                                let req_id = swarm
                                    .behaviour_mut()
                                    .sync_protocol
                                    .send_request(&peer, sync_request);
                                debug!(
                                    peer = %peer, doc_id = %remote_doc.doc_id,
                                    request_id = ?req_id, "Requested changes for diverged document"
                                );
                            }
                        }
                        Err(_) => {
                            // Document doesn't exist locally — request full sync
                            let sync_request = SyncRequest::SyncChanges {
                                app_id: app_id.clone(),
                                doc_id: remote_doc.doc_id.clone(),
                                have_heads: vec![],
                                bloom_filter: None,
                            };
                            let mut swarm = self.swarm.write().await;
                            let req_id = swarm
                                .behaviour_mut()
                                .sync_protocol
                                .send_request(&peer, sync_request);
                            debug!(
                                peer = %peer, doc_id = %remote_doc.doc_id,
                                request_id = ?req_id, "Requested full sync for new document"
                            );
                        }
                    }
                }
            }
            SyncResponse::Changes { app_id, doc_id, changes, new_heads, .. } => {
                if changes.is_empty() {
                    debug!(peer = %peer, app_id = %app_id, doc_id = %doc_id, "No new changes from peer");
                    return;
                }
                info!(
                    peer = %peer, app_id = %app_id, doc_id = %doc_id,
                    change_count = changes.len(), "Applying changes from peer"
                );
                if let Err(e) = self.sync_manager.apply_changes(&app_id, &doc_id, changes).await {
                    warn!(
                        peer = %peer, app_id = %app_id, doc_id = %doc_id,
                        error = %e, "Failed to apply sync changes"
                    );
                } else {
                    debug!(
                        peer = %peer, doc_id = %doc_id,
                        new_heads = ?new_heads, "Changes applied successfully"
                    );
                }
            }
            SyncResponse::Heads { app_id, doc_id, heads, .. } => {
                // Compare with local heads and request changes if different
                match self.sync_manager.get_heads(&app_id, &doc_id).await {
                    Ok(local_heads) if local_heads != heads => {
                        let sync_request = SyncRequest::SyncChanges {
                            app_id: app_id.clone(),
                            doc_id: doc_id.clone(),
                            have_heads: local_heads,
                            bloom_filter: None,
                        };
                        let mut swarm = self.swarm.write().await;
                        swarm.behaviour_mut().sync_protocol.send_request(&peer, sync_request);
                        debug!(peer = %peer, doc_id = %doc_id, "Heads differ, requesting changes");
                    }
                    _ => {
                        debug!(peer = %peer, doc_id = %doc_id, "Heads match, in sync");
                    }
                }
            }
            SyncResponse::Error { message } => {
                warn!(peer = %peer, request_id = ?request_id, error = %message, "Sync error from peer");
            }
            _ => {
                debug!(peer = %peer, request_id = ?request_id, "Unhandled sync response type");
            }
        }
    }

    /// Get shutdown sender for graceful shutdown
    pub fn shutdown_sender(&self) -> broadcast::Sender<()> {
        self.shutdown_tx.clone()
    }

    /// Get reference to sync manager for external use
    pub fn sync_manager(&self) -> &Arc<SyncManager> {
        &self.sync_manager
    }

    /// Create a Send+Sync handle for HttpServer to query P2P status
    pub fn handle(&self) -> P2PHandle {
        P2PHandle {
            status_rx: self.status_tx.subscribe(),
        }
    }

    /// Initiate a sync round with all connected peers.
    /// Sends ListDocuments requests; responses arrive as SyncProtocol events
    /// and are handled in `handle_behaviour_event`.
    async fn initiate_sync_round(&self) {
        let swarm = self.swarm.read().await;
        let peers: Vec<PeerId> = swarm.connected_peers().cloned().collect();
        drop(swarm);

        if peers.is_empty() {
            debug!("Sync round: no connected peers, skipping");
            return;
        }

        info!(peer_count = peers.len(), "Initiating sync round");

        for peer_id in peers {
            let request = SyncRequest::ListDocuments {
                app_id: "elohim".to_string(),
                prefix: None,
                offset: 0,
                limit: 1000,
            };

            let mut swarm = self.swarm.write().await;
            let request_id = swarm
                .behaviour_mut()
                .sync_protocol
                .send_request(&peer_id, request);
            debug!(peer = %peer_id, request_id = ?request_id, "Sent ListDocuments sync request");
        }
    }

    /// Refresh the status snapshot (called from event loop)
    async fn refresh_status(&self) {
        let swarm = self.swarm.read().await;
        let connected_peers = swarm.connected_peers().count();
        let listen_addresses: Vec<String> = swarm.listeners()
            .map(|a| a.to_string())
            .collect();
        let bootstrap_nodes: Vec<String> = self.config.bootstrap_nodes.clone();
        let sync_documents = self.sync_manager.count_documents("_all").await.unwrap_or(0) as usize;
        let nat_status = self.nat_status.read().await.clone();
        let relay_reservations = self.relay_reservations.load(std::sync::atomic::Ordering::Relaxed);

        let status = P2PStatusInfo {
            peer_id: self.peer_id().to_string(),
            listen_addresses,
            connected_peers,
            bootstrap_nodes,
            sync_documents,
            nat_status,
            relay_reservations,
            announce_addresses: self.config.announce_addresses.clone(),
            relay_mode: self.config.relay_mode.to_string(),
        };
        let _ = self.status_tx.send(status);
    }
}
