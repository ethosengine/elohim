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
pub mod epr_protocol;
pub mod kad_store;
pub mod replication;
pub mod shard_protocol;
pub mod sync_protocol;
pub mod trust_cache;
pub mod trust_protocol;

use futures::StreamExt;
use libp2p::kad::{store::RecordStore, Record, RecordKey};
use libp2p::{
    autonat, dcutr, identify, kad, mdns,
    multiaddr::Protocol,
    noise, relay, request_response,
    swarm::{Swarm, SwarmEvent},
    tcp, yamux, Multiaddr, PeerId, SwarmBuilder,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, oneshot, RwLock};
use tracing::{debug, error, info, warn};

use crate::db::DbPool;

/// Map of pending EPR resolve requests: request ID → (requested content ID, reply sender)
type PendingEprMap = Arc<
    tokio::sync::Mutex<
        std::collections::HashMap<
            request_response::OutboundRequestId,
            (String, oneshot::Sender<Option<Vec<u8>>>),
        >,
    >,
>;

/// Map of pending shard fetch requests: outbound request ID → reply sender
type PendingShardMap = Arc<
    tokio::sync::Mutex<
        std::collections::HashMap<
            request_response::OutboundRequestId,
            oneshot::Sender<Option<Vec<u8>>>,
        >,
    >,
>;

/// Map of pending shard push requests: outbound request ID → reply sender
type PendingShardPushMap = Arc<
    tokio::sync::Mutex<
        std::collections::HashMap<
            request_response::OutboundRequestId,
            oneshot::Sender<Result<(), String>>,
        >,
    >,
>;

/// Map of pending shard verification requests: outbound request ID → (shard_hash, peer_id)
type PendingVerificationMap = Arc<
    tokio::sync::Mutex<
        std::collections::HashMap<request_response::OutboundRequestId, (String, String)>,
    >,
>;

use dashmap::DashMap;

use crate::blob_store::BlobStore;
use crate::error::StorageError;
use crate::identity::NodeIdentity;

/// A peer discovered on the network with its delivery capabilities.
/// Populated from mDNS discovery + identify protocol info.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeliveryPeer {
    /// libp2p PeerId (base58)
    pub peer_id: String,
    /// Known multiaddrs for this peer
    pub multiaddrs: Vec<String>,
    /// Network proximity: "lan" (mDNS) or "wan"
    pub network: String,
    /// Capability strings from CapacityAnnouncement (e.g., "serves_extracted", "warm:sha256-abc")
    pub capabilities: Vec<String>,
    /// When this peer was last seen (unix ms)
    pub last_seen: u64,
    /// HTTP port for direct file serving (default 8090)
    pub http_port: u16,
}
use crate::sync::{DocStore, StreamTracker, SyncManager};
use elohim_cache_core::extraction::ExtractionCache;

pub use behaviour::{ElohimStorageBehaviour, RelayMode};
pub use epr_protocol::{EprCodec, EprProtocol, EprRequest, EprResponse};
pub use shard_protocol::{ShardCodec, ShardProtocol, ShardRequest, ShardResponse};
pub use sync_protocol::{DocumentInfo, SyncCodec, SyncProtocol, SyncRequest, SyncResponse};

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
    /// Command channel receiver (consumed by run())
    command_rx: Arc<tokio::sync::Mutex<mpsc::Receiver<P2PCommand>>>,
    /// Command channel sender (cloned into P2PHandle)
    command_tx: mpsc::Sender<P2PCommand>,
    /// Database pool for EPR Head construction from content records
    db_pool: Option<DbPool>,
    /// Policy enforcement for content filtering on P2P path
    policy_enforcement: Option<Arc<crate::db::policy_cache::PolicyEnforcement>>,
    /// Per-connection trust context cache for ambient authorization
    peer_trust_cache: trust_cache::PeerTrustCache,
    /// Pending EPR resolve requests awaiting responses from peers
    pending_epr_resolves: PendingEprMap,
    /// Pending shard fetch requests awaiting responses from peers
    pending_shard_fetches: PendingShardMap,
    /// Pending shard push requests awaiting acknowledgment
    pending_shard_pushes: PendingShardPushMap,
    /// Pending shard verification (Have) requests
    pending_verifications: PendingVerificationMap,
    /// Whether startup EPR Head publication has run
    initial_publish_done: Arc<std::sync::atomic::AtomicBool>,
    /// Extraction cache for delivery capability advertisement
    extraction_cache: Option<Arc<ExtractionCache>>,
    /// Discovered peers with delivery capabilities (populated from mDNS + identify)
    delivery_peers: Arc<DashMap<String, DeliveryPeer>>,
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
    /// Replication progress for identity-driven content sync
    pub replication: replication::ReplicationStatus,
}

/// Commands sent from HTTP handlers to the P2P event loop.
pub enum P2PCommand {
    /// Publish an EPR Head to Kademlia DHT
    PublishEprHead { id: String, head_bytes: Vec<u8> },
    /// Resolve an EPR Head via Kademlia DHT lookup
    ResolveEpr {
        id: String,
        agent_pubkey: String,
        reply: oneshot::Sender<Option<Vec<u8>>>,
    },
    /// Fetch content bytes via shard protocol from a connected peer
    FetchShard {
        hash: String,
        reply: oneshot::Sender<Option<Vec<u8>>>,
    },
    /// Push a shard to a peer for replication
    PushShard {
        peer_id: PeerId,
        hash: String,
        data: Vec<u8>,
        reply: oneshot::Sender<Result<(), String>>,
    },
}

/// Send+Sync handle for querying P2P status and sending commands from HttpServer.
/// Separated from P2PNode because libp2p Swarm types are not Send.
#[derive(Clone)]
pub struct P2PHandle {
    status_rx: tokio::sync::watch::Receiver<P2PStatusInfo>,
    command_tx: mpsc::Sender<P2PCommand>,
    agent_pubkey: String,
    /// Shared ref to delivery peer registry (populated by P2P event loop)
    delivery_peers: Arc<DashMap<String, DeliveryPeer>>,
}

impl P2PHandle {
    /// Get the latest P2P status snapshot
    pub fn status(&self) -> P2PStatusInfo {
        self.status_rx.borrow().clone()
    }

    /// Get all known delivery peers with their capabilities.
    /// Used by the /api/v1/peers/delivery HTTP endpoint.
    pub fn delivery_peers(&self) -> Vec<DeliveryPeer> {
        self.delivery_peers
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Publish an EPR Head to the DHT. Fire-and-forget.
    pub async fn publish_epr_head(&self, id: String, head_bytes: Vec<u8>) {
        if let Err(e) = self
            .command_tx
            .send(P2PCommand::PublishEprHead { id, head_bytes })
            .await
        {
            warn!(error = %e, "Failed to send PublishEprHead command to P2P loop");
        }
    }

    /// Resolve an EPR Head from the DHT. Returns None on timeout or not found.
    pub async fn resolve_epr(&self, id: &str) -> Option<Vec<u8>> {
        let (reply_tx, reply_rx) = oneshot::channel();
        if self
            .command_tx
            .send(P2PCommand::ResolveEpr {
                id: id.to_string(),
                agent_pubkey: self.agent_pubkey.clone(),
                reply: reply_tx,
            })
            .await
            .is_err()
        {
            return None;
        }
        match tokio::time::timeout(Duration::from_secs(5), reply_rx).await {
            Ok(Ok(result)) => result,
            _ => None,
        }
    }

    /// Fetch content bytes via shard protocol. Returns None on timeout or not found.
    pub async fn fetch_shard(&self, hash: &str) -> Option<Vec<u8>> {
        let (reply_tx, reply_rx) = oneshot::channel();
        if self
            .command_tx
            .send(P2PCommand::FetchShard {
                hash: hash.to_string(),
                reply: reply_tx,
            })
            .await
            .is_err()
        {
            return None;
        }
        match tokio::time::timeout(Duration::from_secs(5), reply_rx).await {
            Ok(Ok(result)) => result,
            _ => None,
        }
    }

    /// Push a shard to a specific peer for replication.
    /// Returns Ok(()) on acknowledgment, Err on timeout/failure.
    pub async fn push_shard(&self, peer_id: &str, hash: &str, data: Vec<u8>) -> Result<(), String> {
        let peer_id: PeerId = peer_id
            .parse()
            .map_err(|e| format!("Invalid peer ID: {e}"))?;
        let (tx, rx) = oneshot::channel();
        self.command_tx
            .send(P2PCommand::PushShard {
                peer_id,
                hash: hash.to_string(),
                data,
                reply: tx,
            })
            .await
            .map_err(|_| "P2P command channel closed".to_string())?;

        match tokio::time::timeout(std::time::Duration::from_secs(30), rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err("Push response channel dropped".to_string()),
            Err(_) => Err("Push timed out after 30s".to_string()),
        }
    }

    /// Distribute all shards of a blob to delivery peers.
    /// Returns the number of shards successfully distributed.
    pub async fn distribute_shards(
        &self,
        content_id: &str,
        blob_data: &[u8],
        pool: &crate::db::DbPool,
        h_app_id: &str,
    ) -> Result<usize, String> {
        let encoder = crate::sharding::ShardEncoder::new(crate::sharding::ShardConfig::default());
        let manifest = encoder.create_manifest(blob_data, "application/octet-stream", "commons");
        let shards = encoder.create_shards(blob_data, &manifest.encoding);

        let peers = self.delivery_peers();
        if peers.is_empty() {
            tracing::info!(content_id, "No delivery peers for shard distribution");
            return Ok(0);
        }

        let mut distributed = 0usize;

        for (i, shard_data) in shards.iter().enumerate() {
            let hash = &manifest.shard_hashes[i];
            let peer = &peers[i % peers.len()];

            match self
                .push_shard(&peer.peer_id, hash, shard_data.clone())
                .await
            {
                Ok(()) => {
                    tracing::info!(content_id, shard_index = i, peer = %peer.peer_id, "Shard distributed");
                    if let Ok(mut conn) = pool.get() {
                        let location = crate::db::models::NewShardLocation {
                            shard_hash: hash,
                            peer_id: &peer.peer_id,
                            h_app_id,
                            status: "announced",
                        };
                        let _ = crate::db::shard_locations::upsert_location(&mut conn, &location);
                    }
                    distributed += 1;
                }
                Err(e) => {
                    tracing::warn!(content_id, shard_index = i, peer = %peer.peer_id, error = %e, "Shard push failed");
                }
            }
        }

        Ok(distributed)
    }

    /// Full P2P content resolution: EPR Head -> shard fetch -> (EprHead, content_bytes).
    ///
    /// Resolves the EPR Head for metadata, extracts blob_cid, then fetches the
    /// actual content bytes via shard protocol. Returns None if either step fails.
    ///
    /// The resolution logic is decoupled from peer selection — today it uses the
    /// first connected peer; future versions can rank by latency or load-balance.
    pub async fn resolve_and_fetch(
        &self,
        id: &str,
    ) -> Option<(crate::epr_codec::EprHead, Vec<u8>)> {
        // Step 1: Resolve EPR Head
        let head_bytes = self.resolve_epr(id).await?;
        let head: crate::epr_codec::EprHead = rmp_serde::from_slice(&head_bytes).ok()?;

        // Step 2: Fetch content bytes via shard protocol using blob_cid
        if head.content.is_empty() {
            debug!(id = %id, "EPR Head has no blob_cid, skipping shard fetch");
            return None;
        }
        let content_bytes = self.fetch_shard(&head.content).await?;

        // Step 3: Verify content integrity (accept sha256-hex or CID formats)
        use sha2::{Digest, Sha256};
        let computed_hash = format!("sha256-{}", hex::encode(Sha256::digest(&content_bytes)));
        if computed_hash != head.content && !head.content.starts_with("bafkrei") {
            warn!(
                id = %id,
                expected = %head.content,
                actual = %computed_hash,
                "Content integrity check failed"
            );
            return None;
        }

        Some((head, content_bytes))
    }
}

/// Map reach level string to numeric index for comparison.
/// Used by the cache fast-path to compare ambient ceiling against requested reach.
/// Extract an HTTP port hint from a multiaddr string.
/// mDNS peers expose their libp2p port; HTTP is conventionally at 8090.
fn extract_http_port(_addr: &str) -> u16 {
    // Default HTTP port for elohim-storage — peers don't advertise
    // HTTP port in multiaddr (that's for libp2p). The convention is 8090.
    8090
}

fn reach_level_index(reach: &str) -> u8 {
    match reach {
        "commons" | "public" => 0,
        "community" => 1,
        "familiar" => 2,
        "trusted" => 3,
        "intimate" => 4,
        "self" | "private" => 5,
        _ => 0,
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

        // Open sled database ONCE — shared between Kademlia store and DocStore.
        // sled uses flock() which prevents multiple opens of the same path.
        let sled_path = config.storage_dir.join("sync.sled");
        if let Some(parent) = sled_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let sled_db = sled::Config::new()
            .path(&sled_path)
            .cache_capacity(64 * 1024 * 1024)
            .mode(sled::Mode::HighThroughput)
            .open()
            .map_err(|e| StorageError::Database(format!("Failed to open sync.sled: {}", e)))?;

        // Clone handle for the behaviour closure (sled::Db is Arc-based, clone is cheap)
        let kad_db = sled_db.clone();

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
                ElohimStorageBehaviour::new(key.clone(), config.clone(), relay_client, kad_db)
            })
            .map_err(|e| StorageError::P2PNetwork(format!("Behaviour error: {}", e)))?
            .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
            .build();

        // Initialize sync infrastructure (reuses same sled::Db handle)
        let doc_store = Arc::new(
            DocStore::from_db(sled_db)
                .map_err(|e| StorageError::Database(format!("DocStore init failed: {}", e)))?,
        );
        let stream_tracker = Arc::new(StreamTracker::new());
        let sync_manager = Arc::new(SyncManager::new(doc_store, stream_tracker));

        let (shutdown_tx, _) = broadcast::channel(1);
        let (command_tx, command_rx) = mpsc::channel(64);

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
            replication: replication::ReplicationStatus::default(),
        };
        let (status_tx, _) = tokio::sync::watch::channel(initial_status);

        info!(peer_id = %peer_id, relay_mode = %config.relay_mode, "Created P2P node with NAT traversal");

        Ok(Self {
            identity,
            config,
            #[allow(clippy::arc_with_non_send_sync)]
            swarm: Arc::new(RwLock::new(swarm)),
            blob_store,
            sync_manager,
            shutdown_tx,
            status_tx,
            nat_status: Arc::new(RwLock::new("Unknown".to_string())),
            relay_reservations: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            command_rx: Arc::new(tokio::sync::Mutex::new(command_rx)),
            command_tx,
            db_pool: None,
            policy_enforcement: None,
            peer_trust_cache: trust_cache::PeerTrustCache::new(),
            pending_epr_resolves: Arc::new(tokio::sync::Mutex::new(
                std::collections::HashMap::new(),
            )),
            pending_shard_fetches: Arc::new(tokio::sync::Mutex::new(
                std::collections::HashMap::new(),
            )),
            pending_shard_pushes: Arc::new(tokio::sync::Mutex::new(
                std::collections::HashMap::new(),
            )),
            pending_verifications: Arc::new(tokio::sync::Mutex::new(
                std::collections::HashMap::new(),
            )),
            initial_publish_done: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            extraction_cache: None,
            delivery_peers: Arc::new(DashMap::new()),
        })
    }

    /// Set the database pool for EPR Head construction
    pub fn with_db_pool(mut self, pool: DbPool) -> Self {
        self.db_pool = Some(pool);
        self
    }

    /// Set policy enforcement for content filtering on P2P path
    pub fn with_policy_enforcement(
        mut self,
        enforcement: Arc<crate::db::policy_cache::PolicyEnforcement>,
    ) -> Self {
        self.policy_enforcement = Some(enforcement);
        self
    }

    /// Set the extraction cache for delivery capability queries.
    /// Called after cache initialization (which may happen after P2P node start).
    pub fn set_extraction_cache(&mut self, cache: Arc<ExtractionCache>) {
        self.extraction_cache = Some(cache);
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
        let mut verify_interval = tokio::time::interval(Duration::from_secs(300));
        verify_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut command_rx = self.command_rx.lock().await;

        loop {
            let mut swarm = self.swarm.write().await;

            tokio::select! {
                event = swarm.select_next_some() => {
                    drop(swarm); // Release write lock before handling
                    self.handle_event(event).await;
                }
                Some(cmd) = command_rx.recv() => {
                    self.handle_command(&mut swarm, cmd).await;
                    drop(swarm);
                }
                _ = status_interval.tick() => {
                    drop(swarm);
                    self.refresh_status().await;
                    // One-time startup EPR Head publication
                    if !self.initial_publish_done.load(std::sync::atomic::Ordering::Relaxed) {
                        self.initial_publish_done.store(true, std::sync::atomic::Ordering::Relaxed);
                        self.publish_all_epr_heads().await;
                    }
                }
                _ = sync_interval.tick() => {
                    drop(swarm);
                    self.initiate_sync_round().await;
                }
                _ = verify_interval.tick() => {
                    self.verify_shard_locations(&mut swarm).await;
                    drop(swarm);
                }
                _ = shutdown.recv() => {
                    info!("P2P node shutting down");
                    break;
                }
            }
        }
    }

    /// Handle a command from P2PHandle (HTTP handlers)
    async fn handle_command(&self, swarm: &mut Swarm<ElohimStorageBehaviour>, cmd: P2PCommand) {
        match cmd {
            P2PCommand::PublishEprHead { id, head_bytes } => {
                let key = RecordKey::new(&format!("epr:{}", id));
                let record = Record {
                    key,
                    value: head_bytes,
                    publisher: Some(*self.identity.peer_id()),
                    expires: None,
                };
                match swarm
                    .behaviour_mut()
                    .kademlia
                    .put_record(record, libp2p::kad::Quorum::One)
                {
                    Ok(_) => {
                        info!(id = %id, "Published EPR Head to Kademlia");
                    }
                    Err(e) => {
                        warn!(id = %id, error = ?e, "Failed to publish EPR Head to Kademlia");
                    }
                }
            }
            P2PCommand::ResolveEpr {
                id,
                agent_pubkey,
                reply,
            } => {
                let key = RecordKey::new(&format!("epr:{}", id));
                // First check local Kademlia store
                let local_result = swarm
                    .behaviour_mut()
                    .kademlia
                    .store_mut()
                    .get(&key)
                    .map(|cow| cow.value.clone());

                if let Some(data) = local_result {
                    debug!(id = %id, "EPR Head resolved from local Kademlia store");
                    let _ = reply.send(Some(data));
                } else {
                    // Not in local store — send resolve request to the first connected
                    // peer and register the reply sender so handle_epr_response can
                    // deliver the result back to the HTTP handler.
                    let peers: Vec<PeerId> = swarm.connected_peers().cloned().collect();
                    if let Some(peer_id) = peers.first() {
                        let req_id = swarm.behaviour_mut().epr_protocol.send_request(
                            peer_id,
                            EprRequest::Resolve {
                                id: id.clone(),
                                agent_pubkey: Some(agent_pubkey.clone()),
                            },
                        );
                        debug!(peer = %peer_id, id = %id, request_id = ?req_id, "Sent EPR Resolve request to peer");
                        self.pending_epr_resolves
                            .lock()
                            .await
                            .insert(req_id, (id, reply));
                    } else {
                        debug!(id = %id, "No connected peers for EPR resolve");
                        let _ = reply.send(None);
                    }
                }
            }
            P2PCommand::FetchShard { hash, reply } => {
                let peers: Vec<PeerId> = swarm.connected_peers().cloned().collect();
                if let Some(peer_id) = peers.first() {
                    let req_id = swarm
                        .behaviour_mut()
                        .shard_protocol
                        .send_request(peer_id, ShardRequest::Get { hash: hash.clone() });
                    debug!(peer = %peer_id, hash = %hash, request_id = ?req_id, "Sent shard fetch request to peer");
                    self.pending_shard_fetches
                        .lock()
                        .await
                        .insert(req_id, reply);
                } else {
                    debug!(hash = %hash, "No connected peers for shard fetch");
                    let _ = reply.send(None);
                }
            }
            P2PCommand::PushShard {
                peer_id,
                hash,
                data,
                reply,
            } => {
                let request = ShardRequest::Push {
                    hash: hash.clone(),
                    data,
                };
                let request_id = swarm
                    .behaviour_mut()
                    .shard_protocol
                    .send_request(&peer_id, request);
                debug!(peer = %peer_id, hash = %hash, request_id = ?request_id, "Sent shard push request to peer");
                self.pending_shard_pushes
                    .lock()
                    .await
                    .insert(request_id, reply);
            }
        }
    }

    /// Publish EPR Heads for all existing content to Kademlia DHT.
    /// Runs once on startup with adaptive rate limiting.
    async fn publish_all_epr_heads(&self) {
        let pool = match self.db_pool.as_ref() {
            Some(p) => p,
            None => {
                info!("Skipping startup EPR publish — no DB pool");
                return;
            }
        };
        let mut conn = match pool.get() {
            Ok(c) => c,
            Err(e) => {
                warn!(error = %e, "Skipping startup EPR publish — DB connection failed");
                return;
            }
        };

        let app_ctx = crate::db::AppContext::default_lamad();
        let query = crate::db::content_diesel::ContentQuery {
            limit: 10000,
            ..Default::default()
        };
        let content_items =
            match crate::db::content_diesel::list_content(&mut conn, &app_ctx, &query) {
                Ok(items) => items,
                Err(e) => {
                    warn!(error = %e, "Skipping startup EPR publish — content query failed");
                    return;
                }
            };
        drop(conn); // Release connection before async work

        let total = content_items.len();
        if total == 0 {
            info!("No content to publish EPR Heads for");
            return;
        }

        info!(
            total = total,
            "Starting EPR Head publication for existing content"
        );

        let mut published = 0u64;
        let mut failed = 0u64;
        let mut batch_delay = Duration::from_millis(1);

        for item in &content_items {
            if let Some(head_bytes) = self.resolve_epr_head_locally(&item.content.id) {
                let key = RecordKey::new(&format!("epr:{}", item.content.id));
                let record = Record {
                    key,
                    value: head_bytes,
                    publisher: Some(*self.identity.peer_id()),
                    expires: None,
                };
                let mut swarm = self.swarm.write().await;
                match swarm
                    .behaviour_mut()
                    .kademlia
                    .put_record(record, libp2p::kad::Quorum::One)
                {
                    Ok(_) => {
                        published += 1;
                        // Adaptive: success reduces delay (floor 1ms)
                        batch_delay =
                            Duration::from_millis((batch_delay.as_millis() as u64 / 2).max(1));
                    }
                    Err(e) => {
                        failed += 1;
                        debug!(id = %item.content.id, error = ?e, "Failed to publish EPR Head");
                        // Adaptive: failure increases delay (cap 500ms)
                        batch_delay =
                            Duration::from_millis((batch_delay.as_millis() as u64 * 2).min(500));
                    }
                }
                drop(swarm);
            }

            // Adaptive pacing
            if batch_delay.as_millis() > 1 {
                tokio::time::sleep(batch_delay).await;
            } else {
                tokio::task::yield_now().await;
            }
        }

        info!(
            published = published,
            failed = failed,
            total = total,
            "Startup EPR Head publication complete"
        );
    }

    /// Handle a swarm event
    async fn handle_event(&self, event: SwarmEvent<behaviour::ElohimStorageBehaviourEvent>) {
        match event {
            SwarmEvent::NewListenAddr { address, .. } => {
                info!(address = %address, "Listening on");
            }
            SwarmEvent::ConnectionEstablished {
                peer_id, endpoint, ..
            } => {
                debug!(peer = %peer_id, "Connected to peer");
                {
                    let mut swarm = self.swarm.write().await;
                    // In K8s (mDNS disabled), add connected peers to Kademlia for DHT routing.
                    // With mDNS enabled, discovery handles this. Without mDNS,
                    // peers dialed via DNS bootstrap connect but aren't added to Kademlia
                    // because DNS multiaddrs lack PeerIDs at deploy time.
                    if !self.config.enable_mdns {
                        let addr = endpoint.get_remote_address().clone();
                        swarm
                            .behaviour_mut()
                            .kademlia
                            .add_address(&peer_id, addr.clone());
                        info!(peer = %peer_id, addr = %addr, "Added peer to Kademlia (bootstrap connection)");
                    }
                    // Trigger trust handshake with new peer
                    debug!(peer = %peer_id, "Sending trust handshake");
                    let handshake = trust_protocol::TrustHandshake {
                        agent_pubkey: self.identity.peer_id().to_string(),
                        membership_cids: vec![],
                        relationship_cids: vec![],
                        attestation_cids: vec![],
                        stewardship_cids: vec![],
                    };
                    swarm
                        .behaviour_mut()
                        .trust_protocol
                        .send_request(&peer_id, handshake);
                }
                self.refresh_status().await;
            }
            SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
                debug!(peer = %peer_id, cause = ?cause, "Disconnected from peer");
                self.peer_trust_cache.remove(&peer_id).await;
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
                        request, channel, ..
                    } => {
                        debug!(peer = %peer, request = ?request, "Received shard request");
                        let response = self.handle_shard_request(request).await;

                        // Send response
                        let mut swarm = self.swarm.write().await;
                        if let Err(e) = swarm
                            .behaviour_mut()
                            .shard_protocol
                            .send_response(channel, response)
                        {
                            warn!(peer = %peer, error = ?e, "Failed to send shard response");
                        }
                    }
                    request_response::Message::Response {
                        request_id,
                        response,
                    } => {
                        // Check pending fetch requests
                        let pending_fetch_tx =
                            self.pending_shard_fetches.lock().await.remove(&request_id);
                        if let Some(tx) = pending_fetch_tx {
                            match response {
                                ShardResponse::Data(data) => {
                                    debug!(request_id = ?request_id, size = data.len(), "Shard fetch completed");
                                    let _ = tx.send(Some(data));
                                }
                                _ => {
                                    debug!(request_id = ?request_id, response = ?response, "Shard fetch returned non-data");
                                    let _ = tx.send(None);
                                }
                            }
                        }
                        // Check pending push requests
                        else if let Some(tx) =
                            self.pending_shard_pushes.lock().await.remove(&request_id)
                        {
                            match response {
                                ShardResponse::PushAck => {
                                    debug!(request_id = ?request_id, "Shard push acknowledged");
                                    let _ = tx.send(Ok(()));
                                }
                                ShardResponse::Error(e) => {
                                    debug!(request_id = ?request_id, error = %e, "Shard push rejected");
                                    let _ = tx.send(Err(e));
                                }
                                _ => {
                                    let _ = tx.send(Err("Unexpected response to push".to_string()));
                                }
                            }
                        }
                        // Check pending verification requests
                        else if let Some((shard_hash, peer_id_str)) =
                            self.pending_verifications.lock().await.remove(&request_id)
                        {
                            if let Some(ref pool) = self.db_pool {
                                if let Ok(mut conn) = pool.get() {
                                    match &response {
                                        ShardResponse::Have(true) => {
                                            let _ = crate::db::shard_locations::update_verified(
                                                &mut conn,
                                                &shard_hash,
                                                &peer_id_str,
                                            );
                                        }
                                        ShardResponse::Have(false) | ShardResponse::NotFound => {
                                            let _ = crate::db::shard_locations::mark_lost(
                                                &mut conn,
                                                &shard_hash,
                                                &peer_id_str,
                                            );
                                            info!(shard = %shard_hash, peer = %peer_id_str, "Shard lost — peer reports not having it");
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        } else {
                            debug!(request_id = ?request_id, response = ?response, "Received shard response");
                        }
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
                // Clean up any pending shard fetch so the caller gets None instead of hanging
                if let Some(tx) = self.pending_shard_fetches.lock().await.remove(&request_id) {
                    let _ = tx.send(None);
                }
                // Clean up any pending shard push so the caller gets an error instead of hanging
                if let Some(tx) = self.pending_shard_pushes.lock().await.remove(&request_id) {
                    let _ = tx.send(Err(format!("Outbound failure: {error:?}")));
                }
                // Mark lost for any pending verification request that failed
                if let Some((shard_hash, peer_id_str)) =
                    self.pending_verifications.lock().await.remove(&request_id)
                {
                    if let Some(ref pool) = self.db_pool {
                        if let Ok(mut conn) = pool.get() {
                            let _ = crate::db::shard_locations::mark_lost(
                                &mut conn,
                                &shard_hash,
                                &peer_id_str,
                            );
                        }
                    }
                }
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
                for (peer_id, addr) in &peers {
                    info!(peer = %peer_id, addr = %addr, "mDNS: discovered peer");
                    // Add peer to Kademlia routing table
                    swarm
                        .behaviour_mut()
                        .kademlia
                        .add_address(peer_id, addr.clone());
                }
                drop(swarm);

                // Register as LAN delivery peers
                let now_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64;
                for (peer_id, addr) in peers {
                    let key = peer_id.to_string();
                    let addr_str = addr.to_string();

                    // Extract IP-based HTTP port hint (default 8090)
                    let http_port = extract_http_port(&addr_str);

                    self.delivery_peers
                        .entry(key.clone())
                        .and_modify(|p| {
                            if !p.multiaddrs.contains(&addr_str) {
                                p.multiaddrs.push(addr_str.clone());
                            }
                            p.last_seen = now_ms;
                            p.network = "lan".to_string();
                        })
                        .or_insert_with(|| DeliveryPeer {
                            peer_id: key,
                            multiaddrs: vec![addr_str],
                            network: "lan".to_string(),
                            capabilities: vec!["serves_compressed".to_string()],
                            last_seen: now_ms,
                            http_port,
                        });
                }
            }
            behaviour::ElohimStorageBehaviourEvent::Mdns(mdns::Event::Expired(peers)) => {
                for (peer_id, _addr) in &peers {
                    debug!(peer = %peer_id, "mDNS: peer expired");
                    self.delivery_peers.remove(&peer_id.to_string());
                }
            }
            // Sync protocol events
            behaviour::ElohimStorageBehaviourEvent::SyncProtocol(
                request_response::Event::Message { peer, message },
            ) => match message {
                request_response::Message::Request {
                    request, channel, ..
                } => {
                    debug!(peer = %peer, request = ?request, "Received sync request");
                    let response = self.handle_sync_request(request).await;
                    let mut swarm = self.swarm.write().await;
                    if let Err(e) = swarm
                        .behaviour_mut()
                        .sync_protocol
                        .send_response(channel, response)
                    {
                        warn!(peer = %peer, error = ?e, "Failed to send sync response");
                    }
                }
                request_response::Message::Response {
                    request_id,
                    response,
                } => {
                    self.handle_sync_response(peer, request_id, response).await;
                }
            },
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

            // === EPR protocol events ===
            behaviour::ElohimStorageBehaviourEvent::EprProtocol(
                request_response::Event::Message { peer, message },
            ) => match message {
                request_response::Message::Request {
                    request, channel, ..
                } => {
                    debug!(peer = %peer, request = ?request, "Received EPR request");
                    let response = self.handle_epr_request(request).await;
                    let mut swarm = self.swarm.write().await;
                    if let Err(e) = swarm
                        .behaviour_mut()
                        .epr_protocol
                        .send_response(channel, response)
                    {
                        warn!(peer = %peer, error = ?e, "Failed to send EPR response");
                    }
                }
                request_response::Message::Response {
                    request_id,
                    response,
                } => {
                    self.handle_epr_response(peer, request_id, response).await;
                }
            },
            behaviour::ElohimStorageBehaviourEvent::EprProtocol(
                request_response::Event::OutboundFailure {
                    peer,
                    request_id,
                    error,
                },
            ) => {
                warn!(peer = %peer, request_id = ?request_id, error = ?error, "Outbound EPR request failed");
            }
            behaviour::ElohimStorageBehaviourEvent::EprProtocol(
                request_response::Event::InboundFailure {
                    peer,
                    request_id,
                    error,
                },
            ) => {
                warn!(peer = %peer, request_id = ?request_id, error = ?error, "Inbound EPR request failed");
            }
            behaviour::ElohimStorageBehaviourEvent::EprProtocol(
                request_response::Event::ResponseSent { peer, request_id },
            ) => {
                debug!(peer = %peer, request_id = ?request_id, "EPR response sent");
            }

            // === Trust protocol events ===
            behaviour::ElohimStorageBehaviourEvent::TrustProtocol(
                request_response::Event::Message { peer, message },
            ) => match message {
                request_response::Message::Request {
                    request, channel, ..
                } => {
                    debug!(peer = %peer, agent = %request.agent_pubkey, "Received trust handshake");
                    // Build context from presented credentials (conductor integration fills stubs)
                    let ctx = crate::trust_verification::VerifiedTrustContext {
                        agent_pubkey: request.agent_pubkey.clone(),
                        agent_verified: true,
                        reach_ceiling: "public".to_string(),
                        verified_memberships: vec![],
                        verified_relationships: vec![],
                        verified_attestations: vec![],
                        verified_stewardship: vec![],
                        verified_at: std::time::Instant::now(),
                        ttl: std::time::Duration::from_secs(3600),
                    };
                    let response = trust_protocol::TrustResponse::Verified {
                        reach_ceiling: ctx.reach_ceiling.clone(),
                        ttl_seconds: 3600,
                    };
                    self.peer_trust_cache.insert(peer, ctx).await;
                    let mut swarm = self.swarm.write().await;
                    if let Err(e) = swarm
                        .behaviour_mut()
                        .trust_protocol
                        .send_response(channel, response)
                    {
                        warn!(peer = %peer, error = ?e, "Failed to send trust response");
                    }
                }
                request_response::Message::Response { response, .. } => match response {
                    trust_protocol::TrustResponse::Verified {
                        reach_ceiling,
                        ttl_seconds,
                    } => {
                        debug!(peer = %peer, ceiling = %reach_ceiling, ttl = ttl_seconds, "Trust handshake verified");
                        let ctx = crate::trust_verification::VerifiedTrustContext {
                            agent_pubkey: peer.to_string(),
                            agent_verified: true,
                            reach_ceiling,
                            verified_memberships: vec![],
                            verified_relationships: vec![],
                            verified_attestations: vec![],
                            verified_stewardship: vec![],
                            verified_at: std::time::Instant::now(),
                            ttl: std::time::Duration::from_secs(ttl_seconds),
                        };
                        self.peer_trust_cache.insert(peer, ctx).await;
                    }
                    trust_protocol::TrustResponse::Rejected { reason } => {
                        info!(peer = %peer, reason = %reason, "Trust handshake rejected");
                    }
                    trust_protocol::TrustResponse::Error(msg) => {
                        warn!(peer = %peer, error = %msg, "Trust handshake error");
                    }
                },
            },
            behaviour::ElohimStorageBehaviourEvent::TrustProtocol(
                request_response::Event::OutboundFailure { peer, error, .. },
            ) => {
                debug!(peer = %peer, error = ?error, "Trust handshake outbound failure");
            }
            behaviour::ElohimStorageBehaviourEvent::TrustProtocol(
                request_response::Event::InboundFailure { peer, error, .. },
            ) => {
                debug!(peer = %peer, error = ?error, "Trust handshake inbound failure");
            }
            behaviour::ElohimStorageBehaviourEvent::TrustProtocol(
                request_response::Event::ResponseSent { peer, .. },
            ) => {
                debug!(peer = %peer, "Trust response sent");
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
            behaviour::ElohimStorageBehaviourEvent::Identify(identify::Event::Sent {
                peer_id,
                ..
            }) => {
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
                relay::client::Event::ReservationReqAccepted {
                    relay_peer_id,
                    renewal,
                    ..
                },
            ) => {
                if renewal {
                    debug!(relay = %relay_peer_id, "Relay: reservation renewed");
                } else {
                    info!(relay = %relay_peer_id, "Relay: reservation accepted");
                    self.relay_reservations
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
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
            }) => match result {
                Ok(_) => info!(peer = %remote_peer_id, "DCUtR: direct connection upgraded"),
                Err(ref e) => debug!(peer = %remote_peer_id, error = %e, "DCUtR: upgrade failed"),
            },
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
            ShardRequest::ListContent { reach_filter, offset, limit } => {
                let pool = match self.db_pool.as_ref() {
                    Some(p) => p,
                    None => return ShardResponse::Error("No database pool".to_string()),
                };
                let mut conn = match pool.get() {
                    Ok(c) => c,
                    Err(e) => return ShardResponse::Error(format!("DB connection failed: {}", e)),
                };
                let app_ctx = crate::db::AppContext::default_lamad();
                let query = crate::db::content_diesel::ContentQuery {
                    reach: reach_filter,
                    limit: limit as i64,
                    offset: offset as i64,
                    ..Default::default()
                };
                match crate::db::content_diesel::list_content(&mut conn, &app_ctx, &query) {
                    Ok(items) => {
                        let total = crate::db::content_diesel::count_content(&mut conn, &app_ctx, &query)
                            .unwrap_or(items.len() as i64) as u64;
                        let inventory: Vec<shard_protocol::ContentInventoryItem> = items.iter().map(|cwt| {
                            shard_protocol::ContentInventoryItem {
                                id: cwt.content.id.clone(),
                                title: cwt.content.title.clone(),
                                content_type: cwt.content.content_type.clone(),
                                content_format: cwt.content.content_format.clone(),
                                reach: cwt.content.reach.clone(),
                                blob_cid: cwt.content.blob_cid.clone(),
                                updated_at: cwt.content.updated_at.clone(),
                            }
                        }).collect();
                        let has_more = (offset as u64 + inventory.len() as u64) < total;
                        info!(count = inventory.len(), total = total, "Serving content inventory");
                        ShardResponse::ContentList { items: inventory, total, has_more }
                    }
                    Err(e) => ShardResponse::Error(format!("Content query failed: {}", e)),
                }
            }
            ShardRequest::GetContent { id } => {
                let pool = match self.db_pool.as_ref() {
                    Some(p) => p,
                    None => return ShardResponse::Error("No database pool".to_string()),
                };
                let mut conn = match pool.get() {
                    Ok(c) => c,
                    Err(e) => return ShardResponse::Error(format!("DB connection failed: {}", e)),
                };
                let app_ctx = crate::db::AppContext::default_lamad();
                match crate::db::content_diesel::get_content_with_tags(&mut conn, &app_ctx, &id) {
                    Ok(Some(cwt)) => {
                        debug!(id = %id, "Serving content record to peer");
                        ShardResponse::Content(shard_protocol::ContentRecord {
                            id: cwt.content.id,
                            title: cwt.content.title,
                            description: cwt.content.description,
                            content_type: cwt.content.content_type,
                            content_format: cwt.content.content_format,
                            blob_hash: cwt.content.blob_hash,
                            blob_cid: cwt.content.blob_cid,
                            content_size_bytes: cwt.content.content_size_bytes,
                            metadata_json: cwt.content.metadata_json,
                            reach: cwt.content.reach,
                            created_by: cwt.content.created_by,
                            tags: cwt.tags,
                            content_body: cwt.content.content_body,
                        })
                    }
                    Ok(None) => ShardResponse::ContentNotFound,
                    Err(e) => ShardResponse::Error(format!("Content fetch failed: {}", e)),
                }
            }
        }
    }

    /// Verify that peers still hold their announced shards.
    /// Sends Have requests and marks lost shards.
    async fn verify_shard_locations(&self, swarm: &mut Swarm<ElohimStorageBehaviour>) {
        let pool = match &self.db_pool {
            Some(p) => p,
            None => return,
        };

        let mut conn = match pool.get() {
            Ok(c) => c,
            Err(_) => return,
        };

        use crate::db::diesel_schema::shard_locations;
        use diesel::prelude::*;

        let locations: Vec<crate::db::models::ShardLocationRow> = shard_locations::table
            .filter(shard_locations::status.ne("lost"))
            .limit(100)
            .load(&mut conn)
            .unwrap_or_default();

        if locations.is_empty() {
            return;
        }

        debug!(count = locations.len(), "Verifying shard locations");

        for loc in &locations {
            let peer_id: PeerId = match loc.peer_id.parse() {
                Ok(p) => p,
                Err(_) => continue,
            };

            if !swarm.is_connected(&peer_id) {
                let _ =
                    crate::db::shard_locations::mark_lost(&mut conn, &loc.shard_hash, &loc.peer_id);
                info!(shard = %loc.shard_hash, peer = %loc.peer_id, "Marked lost (peer disconnected)");
                continue;
            }

            let request = ShardRequest::Have {
                hash: loc.shard_hash.clone(),
            };
            let request_id = swarm
                .behaviour_mut()
                .shard_protocol
                .send_request(&peer_id, request);

            self.pending_verifications
                .lock()
                .await
                .insert(request_id, (loc.shard_hash.clone(), loc.peer_id.clone()));
        }
    }

    /// Handle an incoming sync request
    async fn handle_sync_request(&self, request: SyncRequest) -> SyncResponse {
        match request {
            SyncRequest::GetHeads { h_app_id, doc_id } => {
                debug!(h_app_id = %h_app_id, doc_id = %doc_id, "Handling GetHeads request");
                match self.sync_manager.get_heads(&h_app_id, &doc_id).await {
                    Ok(heads) => {
                        // Get change count from doc store
                        let change_count = match self
                            .sync_manager
                            .list_documents(&h_app_id, Some(&doc_id), 0, 1)
                            .await
                        {
                            Ok((docs, _)) => docs.first().map(|d| d.change_count).unwrap_or(0),
                            Err(_) => 0,
                        };
                        SyncResponse::Heads {
                            h_app_id,
                            doc_id,
                            heads,
                            change_count,
                        }
                    }
                    Err(e) => {
                        warn!(h_app_id = %h_app_id, doc_id = %doc_id, error = %e, "Failed to get heads");
                        SyncResponse::Error {
                            message: format!("Failed to get heads: {}", e),
                        }
                    }
                }
            }
            SyncRequest::SyncChanges {
                h_app_id,
                doc_id,
                have_heads,
                bloom_filter: _, // TODO: Use bloom filter for optimization
            } => {
                debug!(h_app_id = %h_app_id, doc_id = %doc_id, have_heads = ?have_heads, "Handling SyncChanges request");
                match self
                    .sync_manager
                    .get_changes_since(&h_app_id, &doc_id, &have_heads)
                    .await
                {
                    Ok((changes, new_heads)) => {
                        info!(h_app_id = %h_app_id, doc_id = %doc_id, changes_count = changes.len(), "Sending changes");
                        SyncResponse::Changes {
                            h_app_id,
                            doc_id,
                            changes,
                            has_more: false, // TODO: Implement pagination
                            new_heads,
                        }
                    }
                    Err(e) => {
                        warn!(h_app_id = %h_app_id, doc_id = %doc_id, error = %e, "Failed to get changes");
                        SyncResponse::Error {
                            message: format!("Failed to get changes: {}", e),
                        }
                    }
                }
            }
            SyncRequest::GetChanges {
                h_app_id,
                doc_id,
                change_hashes,
            } => {
                debug!(h_app_id = %h_app_id, doc_id = %doc_id, change_hashes = ?change_hashes, "Handling GetChanges request");
                // For now, return all changes since empty heads (full sync)
                // TODO: Implement selective change fetching by hash
                match self
                    .sync_manager
                    .get_changes_since(&h_app_id, &doc_id, &[])
                    .await
                {
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
                            h_app_id,
                            doc_id,
                            changes: changes_with_hashes,
                            not_found: vec![],
                        }
                    }
                    Err(e) => {
                        warn!(h_app_id = %h_app_id, doc_id = %doc_id, error = %e, "Failed to get requested changes");
                        SyncResponse::Error {
                            message: format!("Failed to get changes: {}", e),
                        }
                    }
                }
            }
            SyncRequest::AnnounceChange {
                h_app_id,
                doc_id,
                change_hash: _,
                change_data,
            } => {
                debug!(h_app_id = %h_app_id, doc_id = %doc_id, "Handling AnnounceChange request");
                if let Some(data) = change_data {
                    match self
                        .sync_manager
                        .apply_changes(&h_app_id, &doc_id, vec![data])
                        .await
                    {
                        Ok(_) => {
                            info!(h_app_id = %h_app_id, doc_id = %doc_id, "Applied announced change");
                            SyncResponse::ChangeAck {
                                h_app_id,
                                doc_id,
                                was_new: true,
                            }
                        }
                        Err(e) => {
                            warn!(h_app_id = %h_app_id, doc_id = %doc_id, error = %e, "Failed to apply change");
                            SyncResponse::Error {
                                message: format!("Failed to apply change: {}", e),
                            }
                        }
                    }
                } else {
                    // Just an announcement, we'd need to request the change
                    SyncResponse::ChangeAck {
                        h_app_id,
                        doc_id,
                        was_new: false,
                    }
                }
            }
            SyncRequest::ListDocuments {
                h_app_id,
                prefix,
                offset,
                limit,
            } => {
                debug!(h_app_id = %h_app_id, prefix = ?prefix, offset = offset, limit = limit, "Handling ListDocuments request");
                match self
                    .sync_manager
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
                    Err(e) => {
                        warn!(h_app_id = %h_app_id, error = %e, "Failed to list documents");
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
            SyncResponse::DocumentList {
                h_app_id,
                documents,
                total,
                ..
            } => {
                debug!(
                    peer = %peer, h_app_id = %h_app_id, doc_count = documents.len(),
                    total = total, "Received document list from peer"
                );

                // Compare with local documents and request changes for diverged ones
                for remote_doc in &documents {
                    match self
                        .sync_manager
                        .get_heads(&h_app_id, &remote_doc.doc_id)
                        .await
                    {
                        Ok(local_heads) => {
                            if local_heads != remote_doc.heads {
                                // Heads differ — request changes from this peer
                                let sync_request = SyncRequest::SyncChanges {
                                    h_app_id: h_app_id.clone(),
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
                                h_app_id: h_app_id.clone(),
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
            SyncResponse::Changes {
                h_app_id,
                doc_id,
                changes,
                new_heads,
                ..
            } => {
                if changes.is_empty() {
                    debug!(peer = %peer, h_app_id = %h_app_id, doc_id = %doc_id, "No new changes from peer");
                    return;
                }
                info!(
                    peer = %peer, h_app_id = %h_app_id, doc_id = %doc_id,
                    change_count = changes.len(), "Applying changes from peer"
                );
                if let Err(e) = self
                    .sync_manager
                    .apply_changes(&h_app_id, &doc_id, changes)
                    .await
                {
                    warn!(
                        peer = %peer, h_app_id = %h_app_id, doc_id = %doc_id,
                        error = %e, "Failed to apply sync changes"
                    );
                } else {
                    debug!(
                        peer = %peer, doc_id = %doc_id,
                        new_heads = ?new_heads, "Changes applied successfully"
                    );
                }
            }
            SyncResponse::Heads {
                h_app_id,
                doc_id,
                heads,
                ..
            } => {
                // Compare with local heads and request changes if different
                match self.sync_manager.get_heads(&h_app_id, &doc_id).await {
                    Ok(local_heads) if local_heads != heads => {
                        let sync_request = SyncRequest::SyncChanges {
                            h_app_id: h_app_id.clone(),
                            doc_id: doc_id.clone(),
                            have_heads: local_heads,
                            bloom_filter: None,
                        };
                        let mut swarm = self.swarm.write().await;
                        swarm
                            .behaviour_mut()
                            .sync_protocol
                            .send_request(&peer, sync_request);
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

    /// Look up content from local DB and encode as an EPR Head (MessagePack).
    /// Returns None if content not found or DB not available.
    fn resolve_epr_head_locally(&self, id: &str) -> Option<Vec<u8>> {
        let pool = self.db_pool.as_ref()?;
        let mut conn = pool.get().ok()?;
        let app_ctx = crate::db::AppContext::default_lamad();
        match crate::db::content_diesel::get_content_with_tags(&mut conn, &app_ctx, id) {
            Ok(Some(content_with_tags)) => {
                let content = &content_with_tags.content;
                let head = crate::epr_codec::EprHead {
                    version: 1,
                    id: content.id.clone(),
                    content: content.blob_cid.clone().unwrap_or_default(),
                    lamad: crate::epr_codec::EprLamadContext {
                        title: content.title.clone(),
                        content_type: content.content_type.clone(),
                        description: content.description.clone(),
                        content_format: Some(content.content_format.clone()),
                        tags: content_with_tags.tags.clone(),
                    },
                    shefa: {
                        match crate::db::stewardship_allocations::get_allocations_for_content(
                            &mut conn,
                            &app_ctx,
                            &content.id,
                        ) {
                            Ok(allocations) if !allocations.is_empty() => {
                                crate::epr_codec::EprShefaContext {
                                    stewards: allocations
                                        .iter()
                                        .map(|a| a.steward_presence_id.clone())
                                        .collect(),
                                    allocations: allocations
                                        .iter()
                                        .map(|a| a.allocation_ratio as f64)
                                        .collect(),
                                }
                            }
                            _ => crate::epr_codec::EprShefaContext {
                                stewards: vec![],
                                allocations: vec![],
                            },
                        }
                    },
                    qahal: {
                        let mut attestation_requirements = Vec::new();
                        if let Ok(atts) =
                            crate::db::content_attestations::query_attestations_for_content(
                                &mut conn,
                                &content.id,
                            )
                        {
                            for att in &atts {
                                if att.is_revoked == 0 {
                                    let req = if let Some(ref evidence) = att.evidence {
                                        format!("{}:{}", att.attestation_type, evidence)
                                    } else {
                                        att.attestation_type.clone()
                                    };
                                    attestation_requirements.push(req);
                                }
                            }
                        }
                        crate::epr_codec::EprQahalContext {
                            reach: Some(content.reach.clone()),
                            layer: None,
                            attestation_requirements,
                        }
                    },
                    relationships: vec![],
                    author: content.created_by.clone(),
                    updated: Some(content.updated_at.clone()),
                };
                match rmp_serde::to_vec(&head) {
                    Ok(bytes) => Some(bytes),
                    Err(e) => {
                        warn!(id = %id, error = %e, "Failed to encode EPR Head");
                        None
                    }
                }
            }
            Ok(None) => {
                debug!(id = %id, "Content not found for EPR resolve");
                None
            }
            Err(e) => {
                warn!(id = %id, error = %e, "DB error resolving EPR Head");
                None
            }
        }
    }

    /// Resolve an agent pubkey to a DB connection, app context, and Human record.
    fn resolve_agent(
        &self,
        agent_pubkey: Option<&str>,
    ) -> Result<
        (
            crate::db::PooledConn,
            crate::db::AppContext,
            crate::db::models::Human,
        ),
        String,
    > {
        let agent_key = agent_pubkey
            .ok_or_else(|| "Agent identity required for restricted content".to_string())?;
        let pool = self
            .db_pool
            .as_ref()
            .ok_or_else(|| "Database not available for authorization".to_string())?;
        let mut conn = pool
            .get()
            .map_err(|e| format!("DB connection failed: {}", e))?;
        let app_ctx = crate::db::AppContext::default_lamad();
        let human = crate::db::humans::get_human_by_agent_key(&mut conn, agent_key)
            .map_err(|e| format!("Agent lookup failed: {}", e))?
            .ok_or_else(|| "Unknown agent — no human identity found".to_string())?;
        Ok((conn, app_ctx, human))
    }

    /// Get human IDs of all active stewards for a content item.
    /// Maps: content_id → stewardship_allocations → contributor_presences → steward_id (human).
    fn get_steward_human_ids(
        &self,
        conn: &mut diesel::SqliteConnection,
        ctx: &crate::db::AppContext,
        content_id: &str,
    ) -> Result<Vec<String>, String> {
        use crate::db::diesel_schema::contributor_presences;
        use diesel::prelude::*;

        let allocations =
            crate::db::stewardship_allocations::get_allocations_for_content(conn, ctx, content_id)
                .map_err(|e| format!("Allocation lookup failed: {}", e))?;

        if allocations.is_empty() {
            return Ok(Vec::new());
        }

        let presence_ids: Vec<&str> = allocations
            .iter()
            .map(|a| a.steward_presence_id.as_str())
            .collect();

        let presences: Vec<crate::db::models::ContributorPresence> = contributor_presences::table
            .filter(contributor_presences::id.eq_any(&presence_ids))
            .filter(contributor_presences::h_app_id.eq(ctx.h_app_id()))
            .load(conn)
            .map_err(|e| format!("Presence lookup failed: {}", e))?;

        Ok(presences.into_iter().filter_map(|p| p.steward_id).collect())
    }

    /// Check if a requesting agent is authorized to access content at the given reach level.
    /// Returns Ok(()) if authorized, Err(reason) if denied.
    ///
    /// Reach tiers (lowest to highest):
    /// - commons/public: no check
    /// - community: consented collective membership
    /// - familiar: shared collective with a content steward
    /// - trusted: relationship with steward at intimacy >= trusted
    /// - intimate: mutual intimate relationship with steward (both consents)
    /// - self/private: agent is the content creator
    ///
    /// Fast path: if the peer has a cached trust context with a reach ceiling
    /// at or above the requested tier, and the tier is community or below,
    /// skip DB lookups entirely (ambient authorization).
    fn check_reach_authorization(
        &self,
        reach: &str,
        agent_pubkey: Option<&str>,
        content_id: &str,
    ) -> Result<(), String> {
        // Fast path: check cached peer trust context (ambient authorization)
        if let Some(ctx) = self.peer_trust_cache.try_get_by_agent(agent_pubkey) {
            let reach_idx = reach_level_index(reach);
            let ceiling_idx = reach_level_index(&ctx.reach_ceiling);
            // For community and below, ambient ceiling is sufficient — no content-specific check needed
            if ceiling_idx >= reach_idx && reach_idx <= reach_level_index("community") {
                return Ok(());
            }
            // For familiar+ tiers, fall through — need content-specific steward match
        }

        match reach {
            "commons" | "public" => Ok(()),

            "community" => {
                let (mut conn, app_ctx, human) = self.resolve_agent(agent_pubkey)?;
                let participations = crate::db::collectives::get_participations_for_human(
                    &mut conn, &app_ctx, &human.id,
                )
                .map_err(|e| format!("Participation lookup failed: {}", e))?;

                if participations
                    .iter()
                    .any(|p| p.consent_state == "consented")
                {
                    Ok(())
                } else {
                    Err("No consented collective membership".to_string())
                }
            }

            "familiar" => {
                let (mut conn, app_ctx, human) = self.resolve_agent(agent_pubkey)?;
                let steward_human_ids =
                    self.get_steward_human_ids(&mut conn, &app_ctx, content_id)?;

                let participations = crate::db::collectives::get_participations_for_human(
                    &mut conn, &app_ctx, &human.id,
                )
                .map_err(|e| format!("Participation lookup failed: {}", e))?;

                for participation in &participations {
                    if participation.consent_state != "consented" {
                        continue;
                    }
                    let members = crate::db::collectives::get_participants_of_collective(
                        &mut conn,
                        &app_ctx,
                        &participation.collective_id,
                    )
                    .map_err(|e| format!("Members lookup failed: {}", e))?;

                    if members
                        .iter()
                        .any(|m| steward_human_ids.contains(&m.human_id))
                    {
                        return Ok(());
                    }
                }

                Err("No shared collective with content steward".to_string())
            }

            "trusted" => {
                let (mut conn, app_ctx, human) = self.resolve_agent(agent_pubkey)?;
                let steward_human_ids =
                    self.get_steward_human_ids(&mut conn, &app_ctx, content_id)?;

                let relationships = crate::db::human_relationships::get_relationships_for_human(
                    &mut conn, &app_ctx, &human.id,
                )
                .map_err(|e| format!("Relationship lookup failed: {}", e))?;

                let trusted_idx = crate::db::models::intimacy_levels::index_of("trusted")
                    .ok_or_else(|| "Invalid intimacy level config".to_string())?;

                for rel in &relationships {
                    let other_id = if rel.party_a_id == human.id {
                        &rel.party_b_id
                    } else {
                        &rel.party_a_id
                    };
                    if steward_human_ids.contains(other_id) {
                        if let Some(rel_idx) =
                            crate::db::models::intimacy_levels::index_of(&rel.intimacy_level)
                        {
                            if rel_idx >= trusted_idx {
                                return Ok(());
                            }
                        }
                    }
                }

                Err("No trusted relationship with content steward".to_string())
            }

            "intimate" => {
                let (mut conn, app_ctx, human) = self.resolve_agent(agent_pubkey)?;
                let steward_human_ids =
                    self.get_steward_human_ids(&mut conn, &app_ctx, content_id)?;

                let relationships = crate::db::human_relationships::get_relationships_for_human(
                    &mut conn, &app_ctx, &human.id,
                )
                .map_err(|e| format!("Relationship lookup failed: {}", e))?;

                for rel in &relationships {
                    let other_id = if rel.party_a_id == human.id {
                        &rel.party_b_id
                    } else {
                        &rel.party_a_id
                    };
                    if steward_human_ids.contains(other_id)
                        && rel.intimacy_level == "intimate"
                        && rel.consent_given_by_a == 1
                        && rel.consent_given_by_b == 1
                    {
                        return Ok(());
                    }
                }

                Err("No mutual intimate relationship with content steward".to_string())
            }

            "self" | "private" => {
                let agent_key = agent_pubkey
                    .ok_or_else(|| "Agent identity required for private content".to_string())?;
                let pool = self
                    .db_pool
                    .as_ref()
                    .ok_or_else(|| "Database not available".to_string())?;
                let mut conn = pool
                    .get()
                    .map_err(|e| format!("DB connection failed: {}", e))?;
                let app_ctx = crate::db::AppContext::default_lamad();

                let content = crate::db::content_diesel::get_content_with_tags(
                    &mut conn, &app_ctx, content_id,
                )
                .map_err(|e| format!("Content lookup failed: {}", e))?
                .ok_or_else(|| "Content not found".to_string())?;

                if content.content.created_by.as_deref() == Some(agent_key) {
                    Ok(())
                } else {
                    Err("Content is private — only the creator can access it".to_string())
                }
            }

            _ => Err(format!("Unknown reach level: {}", reach)),
        }
    }

    /// Handle an incoming EPR request from a peer
    async fn handle_epr_request(&self, request: EprRequest) -> EprResponse {
        match request {
            EprRequest::Resolve { id, agent_pubkey } => {
                debug!(id = %id, "Handling EPR Resolve request");

                // Check reach authorization before serving
                if let Some(ref pool) = self.db_pool {
                    if let Ok(mut conn) = pool.get() {
                        let app_ctx = crate::db::AppContext::default_lamad();
                        if let Ok(Some(content_with_tags)) =
                            crate::db::content_diesel::get_content_with_tags(
                                &mut conn, &app_ctx, &id,
                            )
                        {
                            let reach = &content_with_tags.content.reach;
                            if let Err(reason) =
                                self.check_reach_authorization(reach, agent_pubkey.as_deref(), &id)
                            {
                                info!(id = %id, reach = %reach, reason = %reason, "EPR access denied");
                                return EprResponse::AccessDenied {
                                    required_reach: reach.clone(),
                                    reason,
                                };
                            }

                            // Policy enforcement: check device policy ceiling
                            if let (Some(ref enforcement), Some(ref agent)) =
                                (&self.policy_enforcement, &agent_pubkey)
                            {
                                let reach_level_num = match reach.as_str() {
                                    "commons" | "public" => 0u8,
                                    "community" => 1,
                                    "familiar" => 2,
                                    "trusted" => 3,
                                    "intimate" => 4,
                                    "self" | "private" => 5,
                                    _ => 0,
                                };
                                let content_meta = crate::db::policy_cache::ContentMetadata {
                                    hash: id.clone(),
                                    categories: content_with_tags.tags.clone(),
                                    age_rating: None,
                                    reach_level: Some(reach_level_num),
                                };
                                match enforcement.can_serve(agent, &content_meta) {
                                    Ok(crate::db::policy_cache::PolicyDecision::Block {
                                        reason,
                                    }) => {
                                        info!(id = %id, reason = %reason, "P2P content blocked by policy");
                                        return EprResponse::AccessDenied {
                                            required_reach: reach.clone(),
                                            reason,
                                        };
                                    }
                                    Ok(crate::db::policy_cache::PolicyDecision::Allow) => {}
                                    Err(_) => {} // Policy lookup failure is non-blocking
                                }
                            }

                            // Layer 2: Attestation gate (mirrors HTTP path)
                            if let Some(ref agent_key) = agent_pubkey {
                                let attestations =
                                    crate::db::content_attestations::query_attestations_for_content(
                                        &mut conn, &id,
                                    );
                                if let Ok(atts) = attestations {
                                    let prereq_atts: Vec<_> = atts
                                        .iter()
                                        .filter(|a| {
                                            a.attestation_type == "prerequisite-mastery"
                                                && a.is_revoked == 0
                                        })
                                        .collect();

                                    if !prereq_atts.is_empty() {
                                        let human = crate::db::humans::get_human_by_agent_key(
                                            &mut conn, agent_key,
                                        );
                                        if let Ok(Some(human)) = human {
                                            for att in &prereq_atts {
                                                let prereq_content_id = att
                                                    .evidence
                                                    .as_deref()
                                                    .unwrap_or(&att.content_id);
                                                let mastery =
                                                    crate::db::content_mastery::get_mastery_for_content(
                                                        &mut conn,
                                                        &app_ctx,
                                                        &human.id,
                                                        prereq_content_id,
                                                    );
                                                match mastery {
                                                    Ok(Some(m))
                                                        if m.mastery_level != "not_started" => {}
                                                    _ => {
                                                        info!(id = %id, "P2P attestation gate: prerequisite mastery required");
                                                        return EprResponse::AccessDenied {
                                                            required_reach: reach.clone(),
                                                            reason: "Prerequisite mastery required"
                                                                .to_string(),
                                                        };
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Authorized — serve the EPR Head
                match self.resolve_epr_head_locally(&id) {
                    Some(bytes) => {
                        info!(id = %id, size = bytes.len(), "Serving EPR Head");
                        EprResponse::Head(bytes)
                    }
                    None => EprResponse::NotFound,
                }
            }
            EprRequest::Announce { head } => {
                debug!(size = head.len(), "Handling EPR Announce request");
                // Decode and validate the EPR Head
                match rmp_serde::from_slice::<crate::epr_codec::EprHead>(&head) {
                    Ok(epr_head) => {
                        // Store in local Kademlia
                        let key = RecordKey::new(&format!("epr:{}", epr_head.id));
                        let record = Record {
                            key,
                            value: head,
                            publisher: None,
                            expires: None,
                        };
                        let mut swarm = self.swarm.write().await;
                        match swarm
                            .behaviour_mut()
                            .kademlia
                            .put_record(record, libp2p::kad::Quorum::One)
                        {
                            Ok(_) => {
                                info!(id = %epr_head.id, "Stored announced EPR Head");
                                EprResponse::Announced {
                                    accepted: true,
                                    reason: None,
                                }
                            }
                            Err(e) => EprResponse::Announced {
                                accepted: false,
                                reason: Some(format!("Kademlia put failed: {:?}", e)),
                            },
                        }
                    }
                    Err(e) => EprResponse::Announced {
                        accepted: false,
                        reason: Some(format!("Invalid EPR Head format: {}", e)),
                    },
                }
            }
            EprRequest::ResolveBatch { ids } => {
                debug!(count = ids.len(), "Handling EPR ResolveBatch request");
                let mut results = Vec::with_capacity(ids.len());
                for id in &ids {
                    match self.resolve_epr_head_locally(id) {
                        Some(bytes) => results.push(bytes),
                        None => results.push(vec![]),
                    }
                }
                EprResponse::HeadBatch(results)
            }
            EprRequest::GetDocument { id } => {
                debug!(id = %id, "EPR GetDocument not yet implemented");
                EprResponse::Error("GetDocument not yet implemented".to_string())
            }
            EprRequest::QueryDelivery { blob_hash } => {
                debug!(blob_hash = %blob_hash, "Handling EPR QueryDelivery request");

                let warm = match &self.extraction_cache {
                    Some(cache) => {
                        let hashes = cache.ready_content_hashes().await;
                        hashes.contains(&blob_hash)
                    }
                    None => false,
                };

                let (serves_extracted, cache_tier) = match &self.extraction_cache {
                    Some(_) => (warm, "extraction".to_string()),
                    None => (false, "blob-only".to_string()),
                };

                EprResponse::DeliveryInfo {
                    serves_extracted,
                    serves_compressed: true, // all nodes can serve raw blobs
                    cache_tier,
                    warm,
                }
            }
        }
    }

    /// Handle an outbound EPR response from a peer.
    /// Delivers to pending resolve callers and caches in local Kademlia.
    async fn handle_epr_response(
        &self,
        peer: PeerId,
        request_id: request_response::OutboundRequestId,
        response: EprResponse,
    ) {
        // Check if there's a pending resolve waiting for this response
        let pending = self.pending_epr_resolves.lock().await.remove(&request_id);

        match response {
            EprResponse::Head(data) => {
                // Decode and validate the EPR Head before caching
                match rmp_serde::from_slice::<crate::epr_codec::EprHead>(&data) {
                    Ok(head) => {
                        // Validate ID matches what was requested (anti-poisoning)
                        if let Some((ref requested_id, _)) = pending {
                            if head.id != *requested_id {
                                warn!(
                                    peer = %peer,
                                    requested = %requested_id,
                                    received = %head.id,
                                    "EPR Head ID mismatch — peer returned wrong content, ignoring"
                                );
                                if let Some((_, tx)) = pending {
                                    let _ = tx.send(None);
                                }
                                return;
                            }
                        }

                        info!(
                            peer = %peer, id = %head.id,
                            "Received EPR Head from peer, caching locally"
                        );
                        let key = RecordKey::new(&format!("epr:{}", head.id));
                        let record = Record {
                            key,
                            value: data.clone(),
                            publisher: None,
                            expires: None,
                        };
                        let mut swarm = self.swarm.write().await;
                        let _ = swarm
                            .behaviour_mut()
                            .kademlia
                            .put_record(record, libp2p::kad::Quorum::One);
                    }
                    Err(e) => {
                        warn!(peer = %peer, error = %e, "Failed to decode EPR Head response");
                        if let Some((_, tx)) = pending {
                            let _ = tx.send(None);
                        }
                        return;
                    }
                }
                // Deliver to waiting caller
                if let Some((_, tx)) = pending {
                    let _ = tx.send(Some(data));
                }
            }
            EprResponse::NotFound => {
                debug!(peer = %peer, request_id = ?request_id, "EPR Head not found on peer");
                if let Some((_, tx)) = pending {
                    let _ = tx.send(None);
                }
            }
            EprResponse::Error(msg) => {
                warn!(peer = %peer, error = %msg, "EPR error from peer");
                if let Some((_, tx)) = pending {
                    let _ = tx.send(None);
                }
            }
            _ => {
                debug!(peer = %peer, request_id = ?request_id, "Unhandled EPR response type");
                if let Some((_, tx)) = pending {
                    let _ = tx.send(None);
                }
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

    /// Create a Send+Sync handle for HttpServer to query P2P status and send commands
    pub fn handle(&self) -> P2PHandle {
        P2PHandle {
            status_rx: self.status_tx.subscribe(),
            command_tx: self.command_tx.clone(),
            agent_pubkey: self.identity.agent_pubkey().to_string(),
            delivery_peers: Arc::clone(&self.delivery_peers),
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
                h_app_id: "elohim".to_string(),
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
        let listen_addresses: Vec<String> = swarm.listeners().map(|a| a.to_string()).collect();
        let bootstrap_nodes: Vec<String> = self.config.bootstrap_nodes.clone();
        let sync_documents = self.sync_manager.count_documents("_all").await.unwrap_or(0) as usize;
        let nat_status = self.nat_status.read().await.clone();
        let relay_reservations = self
            .relay_reservations
            .load(std::sync::atomic::Ordering::Relaxed);

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
            replication: replication::ReplicationStatus::default(),
        };
        let _ = self.status_tx.send(status);
    }
}
