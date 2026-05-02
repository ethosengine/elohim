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

pub mod adapters;
pub mod attention_tending;
pub mod behaviour;
pub mod blob_fetch;
pub mod dedup;
pub mod epr_atom_protocol;
pub mod epr_protocol;
pub mod fanout;
pub mod feedback_signal;
pub mod identity_binding_gossip;
pub mod identity_handshake;
pub mod identity_map;
pub mod inventory_broadcaster;
pub mod inventory_gossip;
pub mod kad_store;
pub mod reach_authorization;
pub mod recovery_invitation;
pub mod recovery_revocation;
pub mod replication;
pub mod shard_protocol;
pub mod sync_protocol;
pub mod topics;
pub mod trust_cache;
pub mod trust_protocol;

// D.3: re-export topic helpers so callers have a single import surface.
pub use topics::{topic_for, TOPIC_IDENTITY_BINDING, TOPIC_INTEGRITY_REVOCATION};
// D.6: re-export DedupLru so callers can access it via the p2p module surface.
pub use dedup::DedupLru;
// D.4: re-export reach authorization types for callers (author-side earning +
// receiver-side pre-authorization). The DB-backed resolver functions
// (signer_is_known_agent, node_has_embodied_responsibility) are intentionally
// NOT re-exported — they are internal to FederatedEprStore::put and the
// Phase 3+ subscription wiring respectively.
pub use reach_authorization::{
    classify_pre_authorization, classify_reach_authorization, PreAuthorizationDecision,
    ReachAuthDecision,
};

use futures::StreamExt;
use libp2p::kad::{store::RecordStore, Record, RecordKey};
use libp2p::{
    autonat, dcutr, identify, kad, mdns,
    multiaddr::Protocol,
    noise, ping, relay, request_response,
    swarm::{Swarm, SwarmEvent},
    tcp, yamux, Multiaddr, PeerId, SwarmBuilder,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, oneshot, RwLock};
use tracing::{debug, error, info, warn};
use ts_rs::TS;

use crate::db::DbPool;

// ---------------------------------------------------------------------------
// EPR atom Kademlia constants
// ---------------------------------------------------------------------------

/// Kademlia key prefix for EPR atom provider records.
///
/// MUST stay consistent across `KadStartProviding` (D.4) and `KadGetProviders`
/// (D.7). Both arms format the key as `"{EPR_ATOM_KAD_KEY_PREFIX}:{cid}"`.
/// A single constant prevents silent key-space divergence from a typo.
pub(crate) const EPR_ATOM_KAD_KEY_PREFIX: &str = "epr-atom";

/// Timeout for Kademlia `get_providers` queries before falling back to
/// local-only results.  epr_store.rs imports this constant so the value is
/// expressed once and can be tuned from a single location.
pub(crate) const KAD_GET_PROVIDERS_TIMEOUT: Duration = Duration::from_secs(5);

/// Format a Kademlia key for an EPR atom CID.
#[inline]
pub(crate) fn kad_key_for_atom(cid: &str) -> String {
    format!("{EPR_ATOM_KAD_KEY_PREFIX}:{cid}")
}

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

/// D.7: Map of pending Kademlia `get_providers` queries.
/// QueryId → (accumulated PeerIds, reply sender).
/// Populated when `KadGetProviders` command is processed; resolved (and removed)
/// when the query reaches `step.last == true` via `OutboundQueryProgressed`.
type PendingKadGetProvidersMap = Arc<
    tokio::sync::Mutex<
        std::collections::HashMap<kad::QueryId, (Vec<PeerId>, oneshot::Sender<Vec<PeerId>>)>,
    >,
>;

/// Map of pending shard verification requests: outbound request ID → (shard_hash, peer_id)
type PendingVerificationMap = Arc<
    tokio::sync::Mutex<
        std::collections::HashMap<request_response::OutboundRequestId, (String, String)>,
    >,
>;

/// Map of pending replication GetContent requests: outbound request ID → content_id.
/// Used to clean up replication state when requests fail at the transport level.
type PendingReplicationFetchMap =
    Arc<tokio::sync::Mutex<std::collections::HashMap<request_response::OutboundRequestId, String>>>;

/// Map of pending blob-pull requests (issued after a replication GetContent returns a
/// non-empty blob_hash): outbound request ID → (content_id, blob_hash).
///
/// When the `ShardResponse::Data` arrives the handler stores the bytes into the local
/// BlobStore and emits a tracing event.  Kept separate from `pending_shard_fetches` (which
/// delivers bytes to a caller via oneshot) because blob pulls are fire-and-store — there is
/// no caller waiting on a channel.
type PendingBlobPullMap = Arc<
    tokio::sync::Mutex<
        std::collections::HashMap<request_response::OutboundRequestId, (String, String)>,
    >,
>;

/// P3.4: Map of pending EPR atom fetch requests issued by FederatedEprStore::fetch.
/// outbound request ID → reply oneshot (delivers Some(envelope_bytes) on success, None on failure).
type PendingEprAtomFetchMap = Arc<
    tokio::sync::Mutex<
        std::collections::HashMap<
            request_response::OutboundRequestId,
            oneshot::Sender<Option<Vec<u8>>>,
        >,
    >,
>;

/// Ordered queue of content IDs discovered as replication gaps, awaiting dispatch.
/// Populated by discover() on each ListContent response; drained by drain_gap_queue()
/// at the 5-second dispatch interval, bounded by MAX_REPLICATION_INFLIGHT.
type ReplicationGapQueue = Arc<tokio::sync::Mutex<std::collections::VecDeque<String>>>;

use dashmap::DashMap;

use crate::blob_store::BlobStore;
use crate::error::StorageError;
use crate::identity::NodeIdentity;

// =============================================================================
// Gossipsub topic constants (centralised here to prevent drift)
// =============================================================================

/// Gossipsub topic for recovery invitation fan-out (M3).
/// Subscriber set: the human's intimate recovery circle.
pub const RECOVERY_INVITATION_TOPIC: &str = "recovery.invitation";

/// Gossipsub topic for key revocation fan-out (M4).
/// Subscriber set: emergency contacts, specialist-elohim watchers, security dashboards.
/// Distinct from RECOVERY_INVITATION_TOPIC — subscriber sets differ and
/// revocation semantics differ from invitation (see spec §7.1 decision #6).
pub const RECOVERY_REVOCATION_TOPIC: &str = "recovery.revocation";

/// Convert microseconds-since-epoch to an ISO-8601 UTC string.
/// Returns `None` if the timestamp is out of range.
/// Used by the inventory gossip receive arm (T14).
fn micros_to_iso(micros: i64) -> Option<String> {
    chrono::DateTime::<chrono::Utc>::from_timestamp_micros(micros)
        .map(|dt| dt.format("%Y-%m-%dT%H:%M:%SZ").to_string())
}

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
pub use epr_atom_protocol::{
    verify_incoming_epr, EprAtomCodec, EprAtomProtocol, EprAtomRequest, EprAtomResponse,
    VerifyError, EPR_ATOM_PROTOCOL_ID, MAX_BATCH_CIDS,
    MAX_REQUEST_SIZE as EPR_ATOM_MAX_REQUEST_SIZE, MAX_RESPONSE_SIZE as EPR_ATOM_MAX_RESPONSE_SIZE,
};
pub use epr_protocol::{EprCodec, EprProtocol, EprRequest, EprResponse};
pub use fanout::{channels_for_reach, FanoutChannel};
pub use identity_map::{
    CallerIdentity, HolochainBackedPeerIdentityMap, PeerIdentityMap, StubIdentityMap,
};
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
    /// Identity-driven replication state
    replication_state: replication::ReplicationState,
    /// Maps in-flight replication GetContent request IDs to content IDs.
    /// Used to clean up replication state when requests fail.
    pending_replication_fetches: PendingReplicationFetchMap,
    /// Ordered queue of content IDs awaiting replication dispatch.
    /// drain_gap_queue() consumes from this at a rate bounded by
    /// MAX_REPLICATION_INFLIGHT, decoupling discovery from dispatch.
    gap_queue: ReplicationGapQueue,
    /// Extraction cache for delivery capability advertisement
    extraction_cache: Option<Arc<ExtractionCache>>,
    /// Discovered peers with delivery capabilities (populated from mDNS + identify)
    delivery_peers: Arc<DashMap<String, DeliveryPeer>>,
    /// Cached identify info per peer (populated from identify::Event::Received)
    identify_cache: Arc<DashMap<String, CachedIdentifyInfo>>,
    /// Per-peer runtime metrics (direction, last-seen, RTT)
    peer_metrics: Arc<DashMap<String, PeerMetrics>>,
    /// Backpressure flag: when true, sync/replication cycles are skipped.
    /// Set by bulk write operations (account import, content bulk) to prevent
    /// P2P sync from competing for memory during heavy writes.
    sync_paused: Arc<AtomicBool>,
    /// PeerId → agent CID mapping for reach enforcement.
    /// Backed by `HolochainBackedPeerIdentityMap` once a db_pool is attached
    /// (via `with_db_pool`). Falls back to `StubIdentityMap` (always-Anonymous)
    /// when no pool is present (e.g. unit tests without a DB).
    identity_map: Arc<dyn identity_map::PeerIdentityMap>,
    /// D.6: bounded LRU dedup cache for inbound EPR atom receive paths.
    /// Drops duplicates from gossipsub + Kad + direct-notify redundancy.
    /// In-memory only — restart clears the cache (acceptable; DB writes are
    /// idempotent, so duplicate processing on restart is a non-issue).
    dedup: Arc<dedup::DedupLru>,
    /// D.7: pending Kademlia `get_providers` queries.
    /// Populated by `handle_command(KadGetProviders)`, resolved when
    /// `OutboundQueryProgressed { result: QueryResult::GetProviders(..), step.last }` fires.
    pending_kad_get_providers: PendingKadGetProvidersMap,
    /// Pending blob-pull requests issued after replication GetContent returns a
    /// non-empty blob_hash.  Keyed by the OutboundRequestId of the follow-up
    /// ShardRequest::Get so the Data handler can store bytes without a caller channel.
    pending_blob_pulls: PendingBlobPullMap,
    /// P3.4: pending EPR atom fetch requests from FederatedEprStore::fetch.
    /// Populated by `handle_command(FetchEprAtomFromPeer)`, resolved in
    /// `handle_epr_atom_response` on success/failure/timeout.
    pending_epr_atom_fetches: PendingEprAtomFetchMap,
    /// T16: custody reconciliation counters — incremented by reconcile_pass.
    pub reconciliation_metrics: std::sync::Arc<ReconciliationMetrics>,
    /// T18: shared cache of last gossiped inventory hashes.
    /// Stage 2 broadcaster timer will write here; the parity diagnostic HTTP
    /// endpoint reads via `P2PHandle::last_gossiped_inventory()`.
    /// Both sides share this Arc — initialized in `new()`, cloned into `handle()`.
    pub last_gossiped: Arc<std::sync::RwLock<Vec<String>>>,
}

/// Cached identify protocol info for a connected peer.
#[derive(Debug, Clone)]
struct CachedIdentifyInfo {
    agent_version: String,
    protocols: Vec<String>,
    listen_addrs: Vec<String>,
}

/// Per-peer runtime metrics tracked from swarm events.
struct PeerMetrics {
    /// Whether this peer currently has an active libp2p connection.
    /// Set to true on ConnectionEstablished; false on ConnectionClosed
    /// (entry is also removed on disconnect, so false entries are transient).
    is_connected: bool,
    /// Connection direction: "inbound" or "outbound"
    direction: &'static str,
    /// Unix epoch millis of last peer activity
    last_seen_ms: u64,
    /// Ring buffer of RTT samples from ping (max 8)
    rtt_samples: std::collections::VecDeque<Duration>,
}

/// Atomic counters for custody reconciliation — incremented each pass.
#[derive(Debug, Default)]
pub struct ReconciliationMetrics {
    pub reconcile_passes_total: std::sync::atomic::AtomicU64,
    pub kicks_fired_total: std::sync::atomic::AtomicU64,
    pub placement_gaps_emitted_total: std::sync::atomic::AtomicU64,
}

/// Snapshot copy of `ReconciliationMetrics` for external callers.
#[derive(Debug, Clone, Copy)]
pub struct ReconciliationMetricsSnapshot {
    pub reconcile_passes_total: u64,
    pub kicks_fired_total: u64,
    pub placement_gaps_emitted_total: u64,
}

/// Current unix epoch in milliseconds.
fn now_unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Compute median RTT from a ring buffer of samples.
fn median_rtt(samples: &std::collections::VecDeque<Duration>) -> Option<f64> {
    if samples.is_empty() {
        return None;
    }
    let mut sorted: Vec<f64> = samples.iter().map(|d| d.as_secs_f64() * 1000.0).collect();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = sorted.len() / 2;
    if sorted.len().is_multiple_of(2) {
        Some((sorted[mid - 1] + sorted[mid]) / 2.0)
    } else {
        Some(sorted[mid])
    }
}

/// Drain queue observability. Exposed via `P2PStatusInfo` so other peers can
/// judge how busy/overloaded this node is and potentially route around it
/// — not just for the local seeder's drain-complete check.
#[derive(Debug, Clone, Serialize, Default, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct DrainStatusInfo {
    /// Total rows in the local content projection (scoped to lamad app).
    pub total: i32,
    /// Rows that have been successfully published to the libp2p Kad DHT.
    pub published: i32,
    /// Rows not yet drained. When this is 0 and stable, drain is caught up.
    pub pending: i32,
}

/// P2P node status for observability.
///
/// Wire format governed by: `elohim/sdk/schemas/v1/views/p2p-status-view.schema.json`
/// Schema contract test: `tests/schema_contract.rs::p2p_status_view_matches_schema`
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct P2PStatusInfo {
    pub peer_id: String,
    pub listen_addresses: Vec<String>,
    #[ts(type = "number")]
    pub connected_peers: usize,
    pub bootstrap_nodes: Vec<String>,
    #[ts(type = "number")]
    pub sync_documents: usize,
    /// NAT status detected by autonat: "Unknown", "Public", "Private"
    pub nat_status: String,
    /// Number of active relay reservations
    #[ts(type = "number")]
    pub relay_reservations: usize,
    /// Addresses announced to the network
    pub announce_addresses: Vec<String>,
    /// Relay mode this node is running in
    pub relay_mode: String,
    /// Replication progress for identity-driven content sync
    pub replication: replication::ReplicationStatus,
    /// Drain queue state — None when the DB pool or query is unavailable.
    /// Consumers should treat None as "data not available" (e.g., wait or
    /// avoid using this peer as a load signal), NOT as "caught up".
    pub drain: Option<DrainStatusInfo>,
    /// True when sync/replication is paused for backpressure (bulk write in progress).
    pub sync_paused: bool,
    /// D.7 dedup LRU: number of unique CIDs currently in the dedup window.
    #[ts(type = "number")]
    pub dedup_unique_len: usize,
    /// D.7 dedup LRU: cumulative insert calls (new + duplicate).
    /// Ratio `(dedup_total_seen - dedup_unique_len) / dedup_total_seen` approximates duplication rate.
    #[ts(type = "number")]
    pub dedup_total_seen: usize,
}

/// Per-peer detail from libp2p Swarm state.
///
/// Wire format governed by: `elohim/sdk/schemas/v1/views/peer-info-view.schema.json`
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct PeerInfoView {
    pub peer_id: String,
    pub multiaddrs: Vec<String>,
    pub protocols: Vec<String>,
    pub agent_version: String,
    pub direction: String,
    /// Tier 3 — populated in follow-up sprint
    pub rtt_ms: Option<f64>,
    /// Tier 3 — populated in follow-up sprint
    #[ts(type = "number | null")]
    pub last_seen_ms: Option<u64>,
    /// Tier 3 — populated in follow-up sprint
    pub remote_nat_status: Option<String>,
    /// Tier 3 — populated in follow-up sprint
    #[ts(type = "number | null")]
    pub bandwidth_in: Option<u64>,
    /// Tier 3 — populated in follow-up sprint
    #[ts(type = "number | null")]
    pub bandwidth_out: Option<u64>,
}

/// Paginated list of connected peers.
///
/// Wire format governed by: `elohim/sdk/schemas/v1/views/peer-list-view.schema.json`
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct PeerListView {
    pub peers: Vec<PeerInfoView>,
    #[ts(type = "number")]
    pub total: usize,
}

/// Commands sent from HTTP handlers to the P2P event loop.
pub enum P2PCommand {
    /// Publish an EPR Head to Kademlia DHT.
    ///
    /// Currently unused: the drain loop (`drain_publish_queue`) is the sole
    /// publisher of EPR Heads and calls `put_record` directly on the swarm.
    /// Retained as part of the P2PHandle abstraction for future use.
    #[allow(dead_code)]
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
    /// List connected peers with identify info
    ListPeers {
        reply: oneshot::Sender<Vec<PeerInfoView>>,
    },
    /// Publish a RecoveryInvitation to the `recovery.invitation` gossipsub topic.
    /// Sent by the projection layer after a successful `RecoveryRequestCreated`
    /// upsert. The swarm event loop calls `gossipsub.publish(topic, bytes)`.
    /// Best-effort — publish failure does NOT revert the DB projection; the DHT
    /// remains the source of truth and subscribers can rediscover via signal replay.
    PublishRecoveryInvitation(crate::p2p::recovery_invitation::RecoveryInvitation),
    /// Publish an IdentityBindingGossip to the `elohim/identity/binding` gossipsub topic.
    ///
    /// Sent by `ReconcileController::on_agent_peer_binding` (Task A.10) when a local
    /// `AgentPeerBinding` DHT signal is received. Propagates the binding to all
    /// subscribed peers so they can update their `peer_identity_bindings` projection
    /// without waiting for a connection-time handshake.
    ///
    /// Best-effort — publish failure does NOT block the controller; the DHT entry
    /// remains the source of truth and peers can reconstruct via signal replay or
    /// the next connection-time handshake (A.9).
    PublishIdentityBinding(crate::p2p::identity_binding_gossip::IdentityBindingGossip),
    /// Publish a RecoveryRevocationMessage to the `recovery.revocation` gossipsub topic.
    /// Sent by the projection layer after `KeyRevocationRequested` or
    /// `KeyRevocationEffective` signals. Best-effort — subscriber discovery is
    /// eventual; publish failure does not affect projection correctness.
    PublishRecoveryRevocation(crate::p2p::recovery_revocation::RecoveryRevocationMessage),
    /// Advertise that this node holds the EPR atom with the given CID by issuing
    /// `kademlia.start_providing(...)`. Triggered by `FederatedEprStore::put`
    /// when the fanout policy includes a Kad/KadLight channel for the EPR's reach.
    /// Best-effort: failure to send (channel closed) or to start providing (DHT
    /// rejection) is logged but does not affect the local put.
    ///
    /// Key prefix `epr-atom:{cid}` — distinct from `epr:{id}` used for EPR Head
    /// put_record to clearly demarcate the atom federation track.
    KadStartProviding { cid: String },
    /// D.7: query Kademlia DHT for peers providing the given CID.
    ///
    /// The reply channel receives the accumulated list of PeerIds once the query
    /// reaches `step.last == true` (or empty Vec on timeout / no providers).
    /// Key `epr-atom:{cid}` — matches the key used by D.2's `KadStartProviding`.
    KadGetProviders {
        cid: String,
        reply: oneshot::Sender<Vec<PeerId>>,
    },
    /// Publish an EPR atom announce to a gossipsub topic. Triggered by
    /// `FederatedEprStore::put` when the fanout policy includes a Gossip channel.
    ///
    /// `topic` is the fully-qualified gossipsub topic name built by
    /// `p2p::topics::topic_for`. `payload` is a MessagePack-encoded CID string
    /// (announce-only; receivers fetch the full atom via the EPR atom protocol
    /// if they want the payload).
    ///
    /// Best-effort: failure is logged but does NOT block the local put.
    PublishEprAnnounce { topic: String, payload: Vec<u8> },
    /// P3.4: send `EprAtomRequest::Fetch { cid }` to a specific peer and reply
    /// with the raw envelope bytes on success, or None if the peer responds
    /// NotFound / Error or the request fails at the transport level.
    ///
    /// Composed at the FederatedEprStore level: one command per provider in
    /// arrival order; the store blocks on the reply with a per-peer timeout
    /// and tries the next provider on failure.
    FetchEprAtomFromPeer {
        peer_id: PeerId,
        cid: String,
        reply: oneshot::Sender<Option<Vec<u8>>>,
    },
    /// D.5: direct-notify integrity event to a list of peers via
    /// `/elohim/epr-atom/1.0.0` request-response. Used by
    /// `ReconcileController::on_key_revocation` to send the revocation payload
    /// directly to peers known to have been bound to the revoked agent.
    ///
    /// Best-effort: per-peer send failures are logged but do not block the
    /// controller loop. Peers that miss the direct-notify will still learn of
    /// the revocation via the gossipsub and Kademlia channels (D.2/D.3).
    DirectNotifyIntegrity {
        peer_ids: Vec<PeerId>,
        kind: String,
        payload_bytes: Vec<u8>,
    },
    /// Phase 3.5 — Light Up the Graph: send raw bytes directly to a single peer.
    ///
    /// Used by [`crate::p2p::adapters::LibP2POutboundSink`] (back-prop) for
    /// one-hop predecessor walks. Routes via `IntegrityNotify { kind:
    /// "feedback-signal", .. }` on the existing `/elohim/epr-atom/1.0.0`
    /// request-response protocol — best-effort, fire-and-forget at the command
    /// level.
    SendDirect { peer: PeerId, payload: Vec<u8> },
    /// Phase 3.5 — Light Up the Graph: publish raw bytes to a gossipsub topic.
    ///
    /// Used by [`crate::p2p::adapters::LibP2PGossipPublisher`] (gossip-flood)
    /// for content-reach broadcasts. Best-effort — publish failure (e.g. no
    /// peers subscribed) is logged and does not block the caller.
    GossipPublish { topic: String, payload: Vec<u8> },
    /// T14: request a fresh `BlobInventorySnapshot` from the named peer.
    /// Issued by the projection writer when it detects a sequence gap.
    /// Stage 1 placeholder — just logs and drops; Stage 2 will route this
    /// as a libp2p request-response message. The next periodic snapshot from
    /// the source peer will close the gap naturally in the interim.
    SnapshotRequest { peer_id: libp2p::PeerId },
    /// T17: fetch a blob from a specific peer. Used by the race-fetch helper
    /// (`p2p::blob_fetch::race_fetch`) for GET-time fallback and custody-driven kicks.
    ///
    /// Stage 1 stub — sends `Err("FetchBlob not yet implemented; Stage 1 placeholder")`
    /// via the reply oneshot. The race-fetch helper's control flow, hash verification,
    /// persistence, and serve-blob emission are all exercised by unit tests; the actual
    /// shard-protocol wiring is Stage 2 work (requires a dedicated blob-fetch request-
    /// response channel distinct from the shard channel, which picks any peer not a
    /// specific one).
    FetchBlob {
        peer_id: libp2p::PeerId,
        hash: String,
        reply: tokio::sync::oneshot::Sender<Result<Vec<u8>, String>>,
    },
}

/// RAII guard that resumes P2P sync when dropped.
/// Created by `P2PHandle::pause_sync()` — ensures sync always resumes
/// even if the bulk write panics or returns early.
pub struct SyncPauseGuard {
    flag: Arc<AtomicBool>,
}

impl Drop for SyncPauseGuard {
    fn drop(&mut self) {
        self.flag.store(false, Ordering::Release);
        info!("P2P sync resumed (backpressure released)");
    }
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
    /// Shared backpressure flag — set by bulk write handlers, read by event loop
    sync_paused: Arc<AtomicBool>,
    /// Last gossiped inventory snapshot hashes.
    /// Stage 1: initialized empty. Stage 2 broadcaster timer populates via
    /// `set_last_gossiped_inventory`. The diagnostic endpoint reads this to
    /// compute filesystem parity without needing live P2P connectivity.
    last_gossiped: Arc<std::sync::RwLock<Vec<String>>>,
}

impl P2PHandle {
    /// Get the latest P2P status snapshot
    pub fn status(&self) -> P2PStatusInfo {
        self.status_rx.borrow().clone()
    }

    /// Return the local libp2p PeerId as a base58-encoded string.
    ///
    /// Used by `HttpServer` to populate `AppContext::local_libp2p_peer_id` so
    /// that `FederatedEprStore` can dedup self-reports from the providers list.
    pub fn local_peer_id(&self) -> String {
        self.status_rx.borrow().peer_id.clone()
    }

    /// Return a clone of the P2P command sender.
    ///
    /// Used by `ReconcileController::with_swarm_tx` (Task A.10) to wire the
    /// controller's gossip-publish path. The controller holds the `Sender`
    /// and calls `send(P2PCommand::PublishIdentityBinding(...))` when an
    /// `AgentPeerBinding` DHT signal arrives.
    pub fn command_sender(&self) -> mpsc::Sender<P2PCommand> {
        self.command_tx.clone()
    }

    /// Pause sync/replication cycles for backpressure during bulk writes.
    /// Returns a guard that automatically resumes sync when dropped.
    pub fn pause_sync(&self, reason: &str) -> SyncPauseGuard {
        self.sync_paused.store(true, Ordering::Release);
        info!(reason = %reason, "P2P sync paused for backpressure");
        SyncPauseGuard {
            flag: Arc::clone(&self.sync_paused),
        }
    }

    /// Construct a minimal `P2PHandle` for unit/integration tests.
    ///
    /// The returned handle has an empty delivery-peer registry and a stub
    /// command channel — all `push_shard` / `fetch_shard` / `resolve_epr`
    /// calls will return `Err("stub: no P2P swarm in test")`.  This is
    /// intentional: tests that exercise placement_gaps recording do not need
    /// live P2P connectivity; they only need the DB-side selector to run.
    ///
    /// Tests that require actual shard delivery (e.g. household-diversity
    /// verification via shard_locations) must use a live harness — see
    /// Task 17 (live integration coverage).
    ///
    /// Intended for test utilities only — not for production use.
    #[doc(hidden)]
    pub fn for_testing() -> Self {
        use tokio::sync::{mpsc, watch};

        let (command_tx, mut command_rx) = mpsc::channel::<P2PCommand>(32);
        // Spawn a task that drains commands and responds with stub errors,
        // preventing the channel from blocking callers.
        tokio::spawn(async move {
            while let Some(cmd) = command_rx.recv().await {
                match cmd {
                    P2PCommand::PushShard { reply, .. } => {
                        let _ = reply.send(Err("stub: no P2P swarm in test".to_string()));
                    }
                    P2PCommand::FetchShard { reply, .. } => {
                        let _ = reply.send(None);
                    }
                    P2PCommand::ResolveEpr { reply, .. } => {
                        let _ = reply.send(None);
                    }
                    P2PCommand::ListPeers { reply } => {
                        let _ = reply.send(vec![]);
                    }
                    P2PCommand::PublishEprHead { .. } => {} // fire-and-forget
                    P2PCommand::PublishRecoveryInvitation(_) => {} // fire-and-forget
                    P2PCommand::PublishIdentityBinding(_) => {} // fire-and-forget
                    P2PCommand::PublishRecoveryRevocation(_) => {} // fire-and-forget
                    P2PCommand::KadStartProviding { .. } => {} // fire-and-forget
                    P2PCommand::PublishEprAnnounce { .. } => {} // fire-and-forget
                    P2PCommand::DirectNotifyIntegrity { .. } => {} // fire-and-forget (D.5 best-effort)
                    P2PCommand::SendDirect { .. } => {} // fire-and-forget (T14 back-prop stub)
                    P2PCommand::GossipPublish { .. } => {} // fire-and-forget (T14 gossip-flood stub)
                    P2PCommand::KadGetProviders { reply, .. } => {
                        // D.7 stub: no swarm in test, always return empty.
                        let _ = reply.send(vec![]);
                    }
                    P2PCommand::FetchEprAtomFromPeer { reply, .. } => {
                        // P3.4 stub: no swarm in test, always miss.
                        let _ = reply.send(None);
                    }
                    P2PCommand::SnapshotRequest { .. } => {} // T14 Stage-1 placeholder
                    P2PCommand::FetchBlob { reply, .. } => {
                        // T17 Stage-1 stub: no swarm in test.
                        let _ =
                            reply
                                .send(Err("FetchBlob not yet implemented; Stage 1 placeholder"
                                    .to_string()));
                    }
                }
            }
        });
        let initial_status = P2PStatusInfo {
            peer_id: "stub-peer".to_string(),
            listen_addresses: vec![],
            connected_peers: 0,
            bootstrap_nodes: vec![],
            sync_documents: 0,
            nat_status: "unknown".to_string(),
            relay_reservations: 0,
            announce_addresses: vec![],
            relay_mode: "client".to_string(),
            replication: crate::p2p::replication::ReplicationStatus::default(),
            drain: None,
            sync_paused: false,
            dedup_unique_len: 0,
            dedup_total_seen: 0,
        };
        let (status_tx, status_rx) = watch::channel(initial_status);
        // Keep sender alive so the receiver never sees "sender dropped"
        std::mem::forget(status_tx);
        P2PHandle {
            status_rx,
            command_tx,
            agent_pubkey: "stub-agent".to_string(),
            delivery_peers: Arc::new(DashMap::new()),
            sync_paused: Arc::new(AtomicBool::new(false)),
            last_gossiped: Arc::new(std::sync::RwLock::new(Vec::new())),
        }
    }

    /// Get all known delivery peers with their capabilities.
    /// Used by the /api/v1/peers/delivery HTTP endpoint.
    pub fn delivery_peers(&self) -> Vec<DeliveryPeer> {
        self.delivery_peers
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Return the most recently gossiped inventory snapshot hashes, or `None`
    /// if no snapshot has been published yet (Stage 1: always `None`).
    ///
    /// Stage 2 (broadcaster timer) will call `set_last_gossiped_inventory`
    /// after each successful snapshot publish, making this return `Some`.
    pub fn last_gossiped_inventory(&self) -> Option<Vec<String>> {
        // T19 review Fix #4: previously used `.read().ok()?` which silently
        // returned `None` on poison — asymmetric with the setter, which
        // (post-T19) warns. Recover the inner value (Vec<String> has no
        // invariant to violate) and warn so operators can correlate with
        // the panic that poisoned the lock.
        let guard = self.last_gossiped.read().unwrap_or_else(|e| {
            tracing::warn!("last_gossiped_inventory: RwLock poisoned; recovering inner value");
            e.into_inner()
        });
        if guard.is_empty() {
            None
        } else {
            Some(guard.clone())
        }
    }

    /// Record the hashes that were just gossiped in the most recent snapshot.
    ///
    /// Called by the Stage 2 broadcaster timer after a successful
    /// `BlobInventorySnapshot` publish. Not called in Stage 1 (diagnostic
    /// endpoint will report `gossiped_count = 0` until Stage 2 is wired).
    pub fn set_last_gossiped_inventory(&self, hashes: Vec<String>) {
        if let Ok(mut guard) = self.last_gossiped.write() {
            *guard = hashes;
        } else {
            // T19 Fix #4: poisoned-lock case used to silently drop the
            // inventory record, which made gossip-state divergence
            // invisible. Surface it via tracing::warn so operators can
            // correlate with the panic that poisoned the lock.
            tracing::warn!(
                "set_last_gossiped_inventory: RwLock poisoned; inventory record dropped"
            );
        }
    }

    /// List connected peers with identify protocol info.
    /// Used by the /p2p/peers HTTP endpoint.
    pub async fn list_peers(&self) -> Vec<PeerInfoView> {
        let (tx, rx) = oneshot::channel();
        if self
            .command_tx
            .send(P2PCommand::ListPeers { reply: tx })
            .await
            .is_err()
        {
            return vec![];
        }
        rx.await.unwrap_or_default()
    }

    /// Publish an EPR Head to the DHT. Fire-and-forget.
    ///
    /// Currently unused: the drain loop (`drain_publish_queue`) is the sole
    /// publisher of EPR Heads to Kademlia and calls `put_record` directly,
    /// also setting `p2p_published_at` via `mark_published`. This method is
    /// retained as part of the P2PHandle abstraction for future use.
    #[allow(dead_code)]
    pub async fn publish_epr_head(&self, id: String, head_bytes: Vec<u8>) {
        if let Err(e) = self
            .command_tx
            .send(P2PCommand::PublishEprHead { id, head_bytes })
            .await
        {
            warn!(error = %e, "Failed to send PublishEprHead command to P2P loop");
        }
    }

    /// Publish a RecoveryRevocationMessage to the `recovery.revocation` gossipsub topic.
    ///
    /// Called by the production signal subscriber after `handle_recovery_v2_signal`
    /// dispatches a `KeyRevocationRequested` or `KeyRevocationEffective` signal:
    ///
    /// ```ignore
    /// if let Some(msg) = recovery_revocation_from_signal(&sig, &local_peer_id_str) {
    ///     p2p_handle.publish_recovery_revocation(msg).await;
    /// }
    /// handle_recovery_v2_signal(&mut conn, sig)?;
    /// ```
    ///
    /// Best-effort: failure does not affect projection correctness (DHT is truth).
    /// Errors are logged and dropped.
    pub async fn publish_recovery_revocation(
        &self,
        msg: crate::p2p::recovery_revocation::RecoveryRevocationMessage,
    ) {
        if let Err(e) = self
            .command_tx
            .send(P2PCommand::PublishRecoveryRevocation(msg))
            .await
        {
            warn!(
                error = %e,
                "Failed to send PublishRecoveryRevocation command to P2P loop"
            );
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
    ///
    /// Uses the contract-aware diverse selector (`PeerSelection`) to rank peers
    /// by household + archetype diversity before distribution. Falls back to
    /// round-robin over the selected set when there are fewer selected peers
    /// than shards.
    ///
    /// On full placement  → clears any stale `placement_gaps` rows for this content.
    /// On short placement → writes one `placement_gaps` row per shard, so the
    ///   shefa signal reflects per-shard reality.
    ///
    /// Returns the number of shards successfully pushed (0 if no peers were selected).
    pub async fn distribute_shards(
        &self,
        content_id: &str,
        blob_data: &[u8],
        pool: &crate::db::DbPool,
        h_app_id: &str,
    ) -> Result<usize, String> {
        let encoder = crate::sharding::ShardEncoder::new(crate::sharding::ShardConfig::default());
        let manifest = encoder
            .create_manifest(blob_data, "application/octet-stream", "commons")
            .map_err(|e| format!("shard manifest encode: {e}"))?;
        let shards = encoder
            .create_shards(blob_data, &manifest.encoding)
            .map_err(|e| format!("shard data encode: {e}"))?;

        let total_shards = shards.len();

        // Run the contract-aware diverse selector.
        let sel = crate::services::peer_selection::PeerSelection::new(pool.clone());
        let outcome = sel
            .select(&crate::services::peer_selection::SelectionInput {
                h_app_id,
                content_id,
                content_reach: "commons", // TODO(plan-1-followup): derive from content manifest
                desired_count: total_shards,
            })
            .map_err(|e| format!("peer selection: {e}"))?;

        let (selected, gap_kind_opt, achieved, requested) = match outcome {
            crate::services::peer_selection::SelectionOutcome::Ok(peers) => {
                (peers, None, total_shards as i32, total_shards as i32)
            }
            crate::services::peer_selection::SelectionOutcome::Short {
                peers,
                gap_kind,
                achieved,
                requested,
            } => (peers, Some(gap_kind), achieved, requested),
        };

        let mut distributed = 0usize;
        let now = chrono::Utc::now().to_rfc3339();

        for (i, shard_data) in shards.iter().enumerate() {
            let hash = &manifest.shard_hashes[i];
            if selected.is_empty() {
                break;
            }
            let peer = &selected[i % selected.len()];

            match self
                .push_shard(&peer.peer_id, hash, shard_data.clone())
                .await
            {
                Ok(()) => {
                    tracing::info!(
                        content_id,
                        shard_index = i,
                        peer = %peer.peer_id,
                        household = ?peer.household_id,
                        "Shard distributed"
                    );
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
                    tracing::warn!(
                        content_id,
                        shard_index = i,
                        peer = %peer.peer_id,
                        error = %e,
                        "Shard push failed"
                    );
                }
            }
        }

        // Record placement gaps when selection was short — one row per shard hash
        // so the shefa signal reflects per-shard reality.
        if let Some(gap_kind) = gap_kind_opt {
            if let Ok(mut conn) = pool.get() {
                let coverage = if requested == 0 {
                    0.0_f32
                } else {
                    achieved as f32 / requested as f32
                };
                for hash in &manifest.shard_hashes {
                    let id = uuid::Uuid::new_v4().to_string();
                    let gap = crate::db::models::NewPlacementGap {
                        id: &id,
                        content_id,
                        shard_hash: hash,
                        h_app_id,
                        requested_steward_count: requested,
                        achieved_steward_count: achieved,
                        contract_coverage: coverage,
                        gap_kind,
                        first_seen_at: &now,
                        last_seen_at: &now,
                    };
                    let _ = crate::db::placement_gaps::upsert_gap(&mut conn, &gap);
                }
            }
        } else {
            // Full placement — clear any stale gaps for this content.
            if let Ok(mut conn) = pool.get() {
                let _ =
                    crate::db::placement_gaps::clear_for_content(&mut conn, h_app_id, content_id);
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

/// How often the drain loop scans the content table for unpublished rows.
/// Referenced in http.rs comments on POST /db/content so the latency
/// contract stays refactor-safe. First tick is delayed by
/// DRAIN_INTERVAL_STARTUP_DELAY_SECS to give bootstrap time to connect.
pub const DRAIN_INTERVAL_SECS: u64 = 15;

/// Delay of the first drain tick after `run()` enters the event loop.
/// Shorter than the regular interval so a cold-bootstrapped seeder gets
/// its first drain attempt quickly once peers connect.
pub const DRAIN_INTERVAL_STARTUP_DELAY_SECS: u64 = 5;

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
            .with_dns()
            .map_err(|e| StorageError::P2PNetwork(format!("DNS error: {}", e)))?
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
            nat_status: "unknown".to_string(),
            relay_reservations: 0,
            announce_addresses: config.announce_addresses.clone(),
            relay_mode: config.relay_mode.to_string(),
            replication: replication::ReplicationStatus::default(),
            drain: None,
            sync_paused: false,
            dedup_unique_len: 0,
            dedup_total_seen: 0,
        };
        let (status_tx, _) = tokio::sync::watch::channel(initial_status);

        // Default to always-Anonymous until a pool is attached via with_db_pool().
        // with_db_pool() replaces this with HolochainBackedPeerIdentityMap.
        let identity_map: Arc<dyn identity_map::PeerIdentityMap> =
            Arc::new(identity_map::StubIdentityMap::new());

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
            nat_status: Arc::new(RwLock::new("unknown".to_string())),
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
            replication_state: replication::ReplicationState::new(),
            pending_replication_fetches: Arc::new(tokio::sync::Mutex::new(
                std::collections::HashMap::new(),
            )),
            gap_queue: Arc::new(tokio::sync::Mutex::new(std::collections::VecDeque::new())),
            extraction_cache: None,
            delivery_peers: Arc::new(DashMap::new()),
            identify_cache: Arc::new(DashMap::new()),
            peer_metrics: Arc::new(DashMap::new()),
            sync_paused: Arc::new(AtomicBool::new(false)),
            identity_map,
            dedup: Arc::new(dedup::DedupLru::new()),
            pending_kad_get_providers: Arc::new(tokio::sync::Mutex::new(
                std::collections::HashMap::new(),
            )),
            pending_blob_pulls: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
            pending_epr_atom_fetches: Arc::new(tokio::sync::Mutex::new(
                std::collections::HashMap::new(),
            )),
            reconciliation_metrics: std::sync::Arc::new(ReconciliationMetrics::default()),
            last_gossiped: Arc::new(std::sync::RwLock::new(Vec::new())),
        })
    }

    /// Set the database pool for EPR Head construction and peer identity lookup.
    ///
    /// Also replaces the default stub identity map with a
    /// `HolochainBackedPeerIdentityMap` backed by the provided pool, enabling
    /// real PeerId → agent CID resolution from the `peer_identity_bindings` table.
    pub fn with_db_pool(mut self, pool: DbPool) -> Self {
        self.identity_map = Arc::new(identity_map::HolochainBackedPeerIdentityMap::new(
            pool.clone(),
        ));
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

    /// D.6: Return dedup cache stats as `(unique_len, total_seen)`.
    ///
    /// `unique_len` is the current number of unique CIDs in the LRU window.
    /// `total_seen` is the cumulative count of all insert calls (new + duplicate).
    /// The ratio `(total_seen - unique_len) / total_seen` approximates duplication rate.
    ///
    /// Uses `DedupLru::stats()` for an atomic consistent snapshot.
    /// Values are surfaced in `P2PStatusInfo::dedup_unique_len` / `dedup_total_seen`.
    pub fn dedup_stats(&self) -> (usize, usize) {
        self.dedup.stats()
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

    /// Whether the named peer has an active libp2p connection right now.
    /// Backed by the existing `peer_metrics` DashMap (entries are created
    /// on connect and removed on disconnect).
    pub fn is_connected(&self, peer_id: &libp2p::PeerId) -> bool {
        self.peer_metrics
            .get(&peer_id.to_string())
            .map(|m| m.is_connected)
            .unwrap_or(false)
    }

    /// Snapshot of currently connected peers.
    pub fn connected_peers(&self) -> Vec<libp2p::PeerId> {
        self.peer_metrics
            .iter()
            .filter(|m| m.is_connected)
            .filter_map(|m| m.key().parse().ok())
            .collect()
    }

    /// Read-only snapshot of the reconciliation metrics counters.
    pub fn reconciliation_metrics(&self) -> ReconciliationMetricsSnapshot {
        ReconciliationMetricsSnapshot {
            reconcile_passes_total: self
                .reconciliation_metrics
                .reconcile_passes_total
                .load(std::sync::atomic::Ordering::Relaxed),
            kicks_fired_total: self
                .reconciliation_metrics
                .kicks_fired_total
                .load(std::sync::atomic::Ordering::Relaxed),
            placement_gaps_emitted_total: self
                .reconciliation_metrics
                .placement_gaps_emitted_total
                .load(std::sync::atomic::Ordering::Relaxed),
        }
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
        self.hydrate_replication_state().await;

        let mut status_interval = tokio::time::interval(Duration::from_secs(30));
        let mut sync_interval = tokio::time::interval(Duration::from_secs(60));
        let mut verify_interval = tokio::time::interval(Duration::from_secs(300));
        verify_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut replication_interval = tokio::time::interval(Duration::from_secs(60));
        replication_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        // Drain the replication gap queue every 5 seconds during bootstrap.
        // Dispatches up to MAX_REPLICATION_INFLIGHT items per tick; idle
        // (no-ops) when the queue is empty.
        let mut gap_dispatch_interval = tokio::time::interval(Duration::from_secs(5));
        gap_dispatch_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        // Delay first tick by the full retry interval so it doesn't race with the
        // initial dials queued by start(). start() owns t=0 dialing; this loop owns
        // subsequent attempts.
        let mut bootstrap_retry_interval = tokio::time::interval_at(
            tokio::time::Instant::now() + Duration::from_secs(30),
            Duration::from_secs(30),
        );
        bootstrap_retry_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        // Drain unpublished EPR Heads to Kademlia. Delay first tick by
        // DRAIN_INTERVAL_STARTUP_DELAY_SECS to give start() a moment to queue
        // dials before the first drain fires.
        let mut drain_interval = tokio::time::interval_at(
            tokio::time::Instant::now() + Duration::from_secs(DRAIN_INTERVAL_STARTUP_DELAY_SECS),
            Duration::from_secs(DRAIN_INTERVAL_SECS),
        );
        drain_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        // Track consecutive retry attempts for exponential backoff cap.
        let mut consecutive_empty_ticks: u32 = 0;
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
                }
                _ = sync_interval.tick() => {
                    drop(swarm);
                    if self.sync_paused.load(Ordering::Acquire) {
                        debug!("Skipping sync round (backpressure)");
                    } else {
                        self.initiate_sync_round().await;
                    }
                }
                _ = replication_interval.tick() => {
                    drop(swarm);
                    if self.sync_paused.load(Ordering::Acquire) {
                        debug!("Skipping replication cycle (backpressure)");
                    } else {
                        self.run_replication_cycle().await;
                    }
                }
                _ = gap_dispatch_interval.tick() => {
                    drop(swarm);
                    if self.sync_paused.load(Ordering::Acquire) {
                        debug!("Skipping gap dispatch (backpressure)");
                    } else {
                        self.drain_gap_queue().await;
                    }
                }
                _ = drain_interval.tick() => {
                    drop(swarm);
                    let published = self.drain_publish_queue(500).await;
                    // Refresh status after draining so the watch channel reflects
                    // post-drain counts immediately on the 15s drain cadence, not
                    // only on the 30s status cadence.
                    self.refresh_status().await;
                    // Auto-suppress sync while drain has a large backlog.
                    // Sync would add memory pressure for inventory data that's
                    // about to change anyway. The node prioritizes publishing
                    // over synchronizing until it catches up.
                    if published > 0 {
                        let status = self.status_tx.borrow().clone();
                        let pending = status.drain.map(|d| d.pending).unwrap_or(0);
                        if pending > 100 && !self.sync_paused.load(Ordering::Acquire) {
                            self.sync_paused.store(true, Ordering::Release);
                            info!(pending, "Sync auto-suppressed: drain backlog > 100");
                        } else if pending <= 100 && self.sync_paused.load(Ordering::Acquire) {
                            self.sync_paused.store(false, Ordering::Release);
                            info!(pending, "Sync auto-resumed: drain backlog cleared");
                        }
                    }
                }
                _ = verify_interval.tick() => {
                    self.verify_shard_locations(&mut swarm).await;
                    drop(swarm);
                }
                _ = bootstrap_retry_interval.tick() => {
                    let connected = swarm.connected_peers().count();
                    if connected == 0 && !self.config.bootstrap_nodes.is_empty() {
                        consecutive_empty_ticks = consecutive_empty_ticks.saturating_add(1);
                        // Cap the retry frequency: after 10 ticks (~5 minutes of no peers),
                        // slow down to every 5 minutes by skipping ticks.
                        let should_retry = consecutive_empty_ticks <= 10
                            || consecutive_empty_ticks.is_multiple_of(10);
                        if should_retry {
                            info!(
                                attempt = consecutive_empty_ticks,
                                bootstrap_count = self.config.bootstrap_nodes.len(),
                                "Bootstrap retry: no connected peers, re-dialing bootstrap nodes"
                            );
                            for addr_str in &self.config.bootstrap_nodes {
                                match addr_str.parse::<Multiaddr>() {
                                    Ok(addr) => match swarm.dial(addr.clone()) {
                                        Ok(_) => debug!(addr = %addr, "Re-dialed bootstrap"),
                                        Err(e) => debug!(addr = %addr, error = %e, "Re-dial failed"),
                                    },
                                    Err(e) => warn!(
                                        addr = %addr_str,
                                        error = %e,
                                        "Bootstrap retry: invalid multiaddr in config, skipping"
                                    ),
                                }
                            }
                        }
                    } else if connected > 0 && consecutive_empty_ticks > 0 {
                        info!(
                            connected = connected,
                            prior_attempts = consecutive_empty_ticks,
                            "Bootstrap recovered"
                        );
                        consecutive_empty_ticks = 0;
                    }
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
            // PublishEprHead is currently unreachable — the drain loop in
            // drain_publish_queue is the sole publisher of EPR Heads and calls
            // put_record directly on the swarm. This arm is retained as part of
            // the P2PHandle abstraction for potential future imperative publish
            // use; see #[allow(dead_code)] on the variant declaration.
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
            P2PCommand::ListPeers { reply } => {
                let peers: Vec<PeerInfoView> = swarm
                    .connected_peers()
                    .cloned()
                    .collect::<Vec<_>>()
                    .into_iter()
                    .map(|pid| {
                        let pid_str = pid.to_string();
                        let cached = self.identify_cache.get(&pid_str);
                        let metrics = self.peer_metrics.get(&pid_str);
                        PeerInfoView {
                            peer_id: pid_str,
                            multiaddrs: cached
                                .as_ref()
                                .map(|c| c.listen_addrs.clone())
                                .unwrap_or_default(),
                            protocols: cached
                                .as_ref()
                                .map(|c| c.protocols.clone())
                                .unwrap_or_default(),
                            agent_version: cached
                                .as_ref()
                                .map(|c| c.agent_version.clone())
                                .unwrap_or_default(),
                            direction: metrics
                                .as_ref()
                                .map(|m| m.direction.to_string())
                                .unwrap_or_else(|| "unknown".to_string()),
                            rtt_ms: metrics.as_ref().and_then(|m| median_rtt(&m.rtt_samples)),
                            last_seen_ms: metrics.as_ref().map(|m| m.last_seen_ms),
                            remote_nat_status: None,
                            bandwidth_in: None,
                            bandwidth_out: None,
                        }
                    })
                    .collect();
                let _ = reply.send(peers);
            }
            P2PCommand::PublishRecoveryInvitation(inv) => {
                let topic = libp2p::gossipsub::IdentTopic::new(RECOVERY_INVITATION_TOPIC);
                match inv.to_bytes() {
                    Ok(bytes) => match swarm.behaviour_mut().gossipsub.publish(topic, bytes) {
                        Ok(msg_id) => info!(
                            target: "elohim_storage::recovery",
                            request_hash = %inv.request_hash,
                            human_id = %inv.human_id,
                            message_id = ?msg_id,
                            "Published RecoveryInvitation to recovery.invitation"
                        ),
                        Err(e) => warn!(
                            target: "elohim_storage::recovery",
                            request_hash = %inv.request_hash,
                            error = ?e,
                            "gossipsub publish failed (often: no peers subscribed yet)"
                        ),
                    },
                    Err(e) => warn!(
                        target: "elohim_storage::recovery",
                        request_hash = %inv.request_hash,
                        error = ?e,
                        "Failed to encode RecoveryInvitation"
                    ),
                }
            }
            // A.10: publish identity binding to elohim/identity/binding topic.
            // Triggered by ReconcileController::on_agent_peer_binding when a local
            // AgentPeerBinding DHT signal arrives. Best-effort: publish failure is
            // logged but does not block the controller loop.
            P2PCommand::PublishIdentityBinding(payload) => {
                let topic = libp2p::gossipsub::IdentTopic::new(
                    crate::p2p::identity_binding_gossip::IDENTITY_BINDING_TOPIC,
                );
                match payload.to_bytes() {
                    Ok(bytes) => match swarm.behaviour_mut().gossipsub.publish(topic, bytes) {
                        Ok(msg_id) => info!(
                            target: "elohim_storage::identity",
                            peer_id = %payload.peer_id,
                            agent_cid = %payload.agent_cid,
                            message_id = ?msg_id,
                            "Published IdentityBindingGossip to elohim/identity/binding"
                        ),
                        Err(e) => warn!(
                            target: "elohim_storage::identity",
                            peer_id = %payload.peer_id,
                            agent_cid = %payload.agent_cid,
                            error = ?e,
                            "gossipsub publish failed for identity binding (often: no peers subscribed yet)"
                        ),
                    },
                    Err(e) => warn!(
                        target: "elohim_storage::identity",
                        peer_id = %payload.peer_id,
                        error = ?e,
                        "Failed to encode IdentityBindingGossip"
                    ),
                }
            }
            P2PCommand::PublishRecoveryRevocation(msg) => {
                let topic = libp2p::gossipsub::IdentTopic::new(RECOVERY_REVOCATION_TOPIC);
                match msg.to_bytes() {
                    Ok(bytes) => match swarm.behaviour_mut().gossipsub.publish(topic, bytes) {
                        Ok(msg_id) => info!(
                            target: "elohim_storage::recovery",
                            revocation_id = %msg.revocation_id,
                            human_id = %msg.human_id,
                            status = %msg.status,
                            message_id = ?msg_id,
                            "Published RecoveryRevocationMessage to recovery.revocation"
                        ),
                        Err(e) => warn!(
                            target: "elohim_storage::recovery",
                            revocation_id = %msg.revocation_id,
                            error = ?e,
                            "gossipsub publish failed (often: no peers subscribed yet)"
                        ),
                    },
                    Err(e) => warn!(
                        target: "elohim_storage::recovery",
                        revocation_id = %msg.revocation_id,
                        error = ?e,
                        "Failed to encode RecoveryRevocationMessage"
                    ),
                }
            }
            // D.2: announce atom provider record to Kademlia DHT.
            // Key: EPR_ATOM_KAD_KEY_PREFIX + ":" + cid — distinct from `epr:{id}` (EPR Head put_record).
            P2PCommand::KadStartProviding { cid } => {
                let key = RecordKey::new(&kad_key_for_atom(&cid));
                match swarm.behaviour_mut().kademlia.start_providing(key) {
                    Ok(_) => info!(
                        target: "elohim_storage::epr",
                        cid = %cid,
                        "kad_start_providing: advertised atom CID to DHT"
                    ),
                    Err(e) => warn!(
                        target: "elohim_storage::epr",
                        cid = %cid,
                        error = ?e,
                        "kad_start_providing: DHT start_providing failed"
                    ),
                }
            }
            // D.7: query Kademlia DHT for providers of the EPR atom CID.
            // Issues `get_providers` and registers the query in `pending_kad_get_providers`
            // so that `OutboundQueryProgressed { GetProviders }` events can resolve it.
            P2PCommand::KadGetProviders { cid, reply } => {
                let key = RecordKey::new(&kad_key_for_atom(&cid));
                let query_id = swarm.behaviour_mut().kademlia.get_providers(key);
                debug!(
                    target: "elohim_storage::epr",
                    cid = %cid,
                    query_id = ?query_id,
                    "kad_get_providers: issued DHT provider query"
                );
                self.pending_kad_get_providers
                    .lock()
                    .await
                    .insert(query_id, (Vec::new(), reply));
            }
            // D.3: publish EPR atom announce to a reach-scoped gossipsub topic.
            // topic built by p2p::topics::topic_for; payload is msgpack-encoded CID.
            // Best-effort — publish failure (e.g. no peers subscribed) is logged and
            // does not affect the local put or Kad advertisement.
            P2PCommand::PublishEprAnnounce { topic, payload } => {
                let gsub_topic = libp2p::gossipsub::IdentTopic::new(&topic);
                match swarm.behaviour_mut().gossipsub.publish(gsub_topic, payload) {
                    Ok(msg_id) => info!(
                        target: "elohim_storage::epr",
                        topic = %topic,
                        message_id = ?msg_id,
                        "publish_epr_announce: published atom announce to gossipsub"
                    ),
                    Err(e) => warn!(
                        target: "elohim_storage::epr",
                        topic = %topic,
                        error = ?e,
                        "publish_epr_announce: gossipsub publish failed (often: no peers subscribed yet)"
                    ),
                }
            }
            // D.5: send IntegrityNotify to each affected peer via /elohim/epr-atom/1.0.0.
            // Best-effort: per-peer failures are logged but don't block.
            P2PCommand::DirectNotifyIntegrity {
                peer_ids,
                kind,
                payload_bytes,
            } => {
                for peer_id in &peer_ids {
                    let req = EprAtomRequest::IntegrityNotify {
                        kind: kind.clone(),
                        payload_bytes: payload_bytes.clone(),
                    };
                    let _request_id = swarm
                        .behaviour_mut()
                        .epr_atom_protocol
                        .send_request(peer_id, req);
                    info!(
                        target: "elohim_storage::integrity",
                        peer = %peer_id,
                        kind = %kind,
                        "D.5: sent IntegrityNotify to peer"
                    );
                }
                debug!(
                    target: "elohim_storage::integrity",
                    kind = %kind,
                    peer_count = peer_ids.len(),
                    "D.5: DirectNotifyIntegrity dispatched to all peers"
                );
            }
            // P3.4: send EprAtomRequest::Fetch to a specific peer and register the
            // reply sender in `pending_epr_atom_fetches`. The response is delivered
            // in `handle_epr_atom_response`; transport-level failure is delivered in
            // the `OutboundFailure` arm below.
            P2PCommand::FetchEprAtomFromPeer {
                peer_id,
                cid,
                reply,
            } => {
                let req_id = swarm
                    .behaviour_mut()
                    .epr_atom_protocol
                    .send_request(&peer_id, EprAtomRequest::Fetch { cid: cid.clone() });
                debug!(
                    target: "elohim_storage::epr",
                    peer = %peer_id,
                    cid = %cid,
                    request_id = ?req_id,
                    "P3.4: FetchEprAtomFromPeer — sent Fetch request to peer"
                );
                self.pending_epr_atom_fetches
                    .lock()
                    .await
                    .insert(req_id, reply);
            }
            // Phase 3.5 — Light Up the Graph: one-hop back-prop direct send.
            // Routes the FeedbackSignal payload to the predecessor peer via the
            // existing IntegrityNotify request-response channel (epr-atom/1.0.0).
            // Best-effort: request_id is not tracked; the fire-and-forget contract
            // matches back_prop_one_hop's best-effort sink semantics.
            P2PCommand::SendDirect { peer, payload } => {
                let req = EprAtomRequest::IntegrityNotify {
                    kind: "feedback-signal".to_string(),
                    payload_bytes: payload,
                };
                let _request_id = swarm
                    .behaviour_mut()
                    .epr_atom_protocol
                    .send_request(&peer, req);
                debug!(
                    target: "elohim_storage::back_prop",
                    peer = %peer,
                    "SendDirect: forwarded FeedbackSignal to predecessor peer"
                );
            }
            // Phase 3.5 — Light Up the Graph: gossip-flood publish to a reach topic.
            // Mirrors PublishEprAnnounce but with a caller-supplied topic string so
            // flood_feedback can target the content's reach gossipsub topic.
            // Best-effort: publish failure is logged but does not abort the caller.
            P2PCommand::GossipPublish { topic, payload } => {
                let gsub_topic = libp2p::gossipsub::IdentTopic::new(&topic);
                match swarm.behaviour_mut().gossipsub.publish(gsub_topic, payload) {
                    Ok(msg_id) => debug!(
                        target: "elohim_storage::gossip_flood",
                        topic = %topic,
                        message_id = ?msg_id,
                        "GossipPublish: published FeedbackSignal to reach topic"
                    ),
                    Err(e) => warn!(
                        target: "elohim_storage::gossip_flood",
                        topic = %topic,
                        error = ?e,
                        "GossipPublish: gossipsub publish failed (often: no peers subscribed yet)"
                    ),
                }
            }
            // T14: Stage-1 placeholder — log and drop. Stage 2 will route this as a
            // libp2p request-response message to the named peer. The next periodic
            // snapshot from the source peer closes the gap naturally in the interim.
            P2PCommand::SnapshotRequest { peer_id } => {
                debug!(
                    target: "elohim_storage::inventory",
                    peer_id = %peer_id,
                    "SnapshotRequest queued; Stage 1 placeholder — relying on next periodic snapshot"
                );
            }
            // T17: Stage-1 placeholder — send Err so the race-fetch helper
            // can exercise its full control-flow (Miss) path. Stage 2 will wire
            // this to a dedicated blob-fetch request-response channel that targets
            // the explicit peer_id (FetchShard picks any connected peer, not one
            // specific peer — a different shape that requires new infrastructure).
            P2PCommand::FetchBlob {
                peer_id,
                hash,
                reply,
            } => {
                debug!(
                    target: "elohim_storage::blob_fetch",
                    peer_id = %peer_id,
                    hash = %hash,
                    "FetchBlob: Stage 1 placeholder — not yet wired to shard protocol"
                );
                let _ = reply.send(Err(
                    "FetchBlob not yet implemented; Stage 1 placeholder".to_string()
                ));
            }
        }
    }

    /// Drain up to `batch_limit` unpublished content rows by publishing their
    /// EPR Heads to Kademlia. Returns the number of rows successfully marked
    /// as published. Gated on having at least one connected peer — without
    /// peers, `put_record(Quorum::One)` would silently succeed locally without
    /// gossiping, creating phantom "published" state.
    async fn drain_publish_queue(&self, batch_limit: i64) -> usize {
        const SUB_BATCH_SIZE: i64 = 50;

        // Peer-gate: without peers, Kademlia can't gossip. Bail early.
        {
            let swarm = self.swarm.read().await;
            if swarm.connected_peers().count() == 0 {
                debug!("drain_publish_queue: no connected peers, skipping");
                return 0;
            }
        }

        let Some(pool) = self.db_pool.as_ref() else {
            debug!("drain_publish_queue: no DB pool, skipping");
            return 0;
        };

        let app_ctx = crate::db::AppContext::default_lamad();
        let mut published: usize = 0;
        let mut batch_delay = Duration::from_millis(1);

        while (published as i64) < batch_limit {
            // Re-acquire a fresh connection per sub-batch so we don't hold
            // a pool slot across the entire drain cycle.
            let mut conn = match pool.get() {
                Ok(c) => c,
                Err(e) => {
                    warn!(error = %e, "drain_publish_queue: DB connection failed, stopping");
                    break;
                }
            };

            let remaining = batch_limit - published as i64;
            let sub_limit = remaining.min(SUB_BATCH_SIZE);

            let pending_ids = match crate::db::content_diesel::list_unpublished_content_ids(
                &mut conn, &app_ctx, sub_limit,
            ) {
                Ok(ids) => ids,
                Err(e) => {
                    warn!(error = %e, "drain_publish_queue: list_unpublished failed, stopping");
                    break;
                }
            };

            if pending_ids.is_empty() {
                break;
            }

            let sub_batch_start = published;

            for content_id in &pending_ids {
                let Some(head_bytes) = self.resolve_epr_head_locally(content_id) else {
                    // Can't resolve head — skip this row; it'll be retried next tick.
                    warn!(id = %content_id, "drain: EPR head not resolvable, skipping");
                    continue;
                };

                let key = RecordKey::new(&format!("epr:{}", content_id));
                let record = Record {
                    key,
                    value: head_bytes,
                    publisher: Some(*self.identity.peer_id()),
                    expires: None,
                };

                // SAFETY: put_record returns synchronously in libp2p 0.54 — it
                // queues the record in Kademlia's internal state but does not
                // yield. Holding the swarm write lock across this call is
                // bounded and safe. If a future libp2p upgrade makes this
                // await, move the call out of the lock or reacquire per batch.
                let put_result = {
                    let mut swarm = self.swarm.write().await;
                    swarm
                        .behaviour_mut()
                        .kademlia
                        .put_record(record, libp2p::kad::Quorum::One)
                };

                match put_result {
                    Ok(_) => {
                        match crate::db::content_diesel::mark_published(
                            &mut conn, &app_ctx, content_id,
                        ) {
                            Ok(true) => {
                                published += 1;
                            }
                            Ok(false) => {
                                // Row was concurrently deleted — DHT publish succeeded,
                                // nothing to mark. Not counted as an error.
                                info!(id = %content_id, "drain: row gone after publish");
                            }
                            Err(e) => {
                                warn!(id = %content_id, error = %e, "drain: mark_published failed");
                            }
                        }
                        batch_delay =
                            Duration::from_millis((batch_delay.as_millis() as u64 / 2).max(1));
                    }
                    Err(e) => {
                        debug!(id = %content_id, error = ?e, "drain: put_record failed");
                        batch_delay =
                            Duration::from_millis((batch_delay.as_millis() as u64 * 2).min(500));
                    }
                }

                if batch_delay.as_millis() > 1 {
                    tokio::time::sleep(batch_delay).await;
                }
            }

            // If the sub-batch made zero forward progress, break out to avoid
            // spinning forever on rows that always fail (e.g. every row's
            // EPR head is unresolvable). A future drain tick will retry.
            if published == sub_batch_start {
                debug!("drain_publish_queue: sub-batch made no progress, stopping");
                break;
            }

            // Connection drops here (end of loop iteration) before the next
            // sub-batch acquires a fresh one.
            drop(conn);

            // If the sub-batch returned fewer rows than we asked for, the
            // queue is drained for now.
            if (pending_ids.len() as i64) < sub_limit {
                break;
            }
        }

        if published > 0 {
            info!(published, "drain_publish_queue: cycle complete");
        }
        published
    }

    /// One-shot at startup: populate the replication state with the full set
    /// of local content IDs so the replication protocol can distinguish
    /// "already have" from "need to fetch" without re-querying the DB every
    /// tick. Called once from `run()` before the event loop enters its
    /// select loop.
    ///
    /// Paginates through the content table in chunks of HYDRATE_PAGE_SIZE
    /// rows so a node with >100k local rows doesn't silently truncate its
    /// replication set (which would cause unnecessary re-fetches of content
    /// it already has).
    async fn hydrate_replication_state(&self) {
        const HYDRATE_PAGE_SIZE: i64 = 5_000;
        const HYDRATE_HARD_CEILING: usize = 10_000_000; // safety net

        let Some(pool) = self.db_pool.as_ref() else {
            return;
        };
        let app_ctx = crate::db::AppContext::default_lamad();
        let mut ids: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut offset: i64 = 0;

        loop {
            // Re-acquire a fresh connection per page so we don't hold a pool
            // slot across the entire hydration scan.
            let Ok(mut conn) = pool.get() else {
                warn!(
                    offset,
                    "hydrate_replication_state: db pool unavailable, stopping early"
                );
                break;
            };

            // Internal call — require_provenance: false so we see ALL local
            // rows, including ones that haven't been drained yet. The
            // replication state must reflect reality, not the gated view.
            let query = crate::db::content_diesel::ContentQuery {
                limit: HYDRATE_PAGE_SIZE,
                offset,
                ..Default::default()
            };

            let page =
                match crate::db::content_diesel::list_content(&mut conn, &app_ctx, &query, false) {
                    Ok(page) => page,
                    Err(e) => {
                        warn!(
                            error = %e,
                            offset,
                            "hydrate_replication_state: list_content failed, stopping early"
                        );
                        break;
                    }
                };

            let page_len = page.len();
            for item in &page {
                ids.insert(item.content.id.clone());
            }

            if ids.len() >= HYDRATE_HARD_CEILING {
                warn!(
                    count = ids.len(),
                    "hydrate_replication_state: hit hard ceiling {}, stopping (should not happen in practice)",
                    HYDRATE_HARD_CEILING
                );
                break;
            }

            if (page_len as i64) < HYDRATE_PAGE_SIZE {
                // Last page (partial or empty) — done.
                break;
            }

            offset += HYDRATE_PAGE_SIZE;
        }

        tracing::info!(
            count = ids.len(),
            "Loaded local content IDs for replication state"
        );
        self.replication_state.set_local_ids(ids).await;
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
                // Track connection direction and last-seen for /p2p/peers
                let direction = if endpoint.is_dialer() {
                    "outbound"
                } else {
                    "inbound"
                };
                self.peer_metrics
                    .entry(peer_id.to_string())
                    .and_modify(|m| {
                        m.is_connected = true;
                        m.last_seen_ms = now_unix_ms();
                    })
                    .or_insert_with(|| PeerMetrics {
                        is_connected: true,
                        direction,
                        last_seen_ms: now_unix_ms(),
                        rtt_samples: std::collections::VecDeque::with_capacity(8),
                    });
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

                    // Trigger identity handshake (/elohim/identity/handshake/1.0.0)
                    // Category C — session-local projection of AgentPeerBinding DHT state.
                    // The receiver verifies structural integrity + validity window, then
                    // inserts into peer_identity_bindings with source='handshake'.
                    debug!(peer = %peer_id, "Sending identity handshake");
                    let now_iso = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
                    let nonce = {
                        let n = uuid::Uuid::new_v4();
                        base64::Engine::encode(
                            &base64::engine::general_purpose::STANDARD,
                            n.as_bytes(),
                        )
                    };
                    let identity_request = identity_handshake::IdentityHandshakeRequest {
                        binding: identity_handshake::HandshakeBindingPayload {
                            peer_id: self.identity.peer_id().to_base58(),
                            agent_cid: self.identity.agent_pubkey().to_string(),
                            // TODO(A.11): replace with AgentPeerBinding.valid_from from DHT signal
                            // stream when real binding source is wired.
                            valid_from: now_iso.clone(),
                            valid_until: None,
                            device_archetype: "node".to_string(),
                            // Stage 1: structural non-empty sentinel; full Ed25519 sign is Stage 3.
                            signature: base64::Engine::encode(
                                &base64::engine::general_purpose::STANDARD,
                                self.identity.peer_id().to_base58().as_bytes(),
                            ),
                            dht_anchor_hash: None,
                        },
                        timestamp: now_iso,
                        nonce,
                    };
                    swarm
                        .behaviour_mut()
                        .identity_handshake
                        .send_request(&peer_id, identity_request);
                }
                self.refresh_status().await;
            }
            SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
                debug!(peer = %peer_id, cause = ?cause, "Disconnected from peer");
                self.peer_trust_cache.remove(&peer_id).await;
                self.peer_metrics.remove(&peer_id.to_string());
                self.identify_cache.remove(&peer_id.to_string());
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
                        // Check blob-pull requests (fire-and-store, no caller channel).
                        // Must be checked BEFORE pending_shard_fetches so the pull
                        // response is consumed here rather than delivered to a caller.
                        let blob_pull_entry =
                            self.pending_blob_pulls.lock().await.remove(&request_id);
                        if let Some((content_id, blob_hash)) = blob_pull_entry {
                            match response {
                                ShardResponse::Data(data) => {
                                    let blob_store = self.blob_store.clone();
                                    match blob_store.store(&data).await {
                                        Ok(result) => {
                                            info!(
                                                content_id = %content_id,
                                                blob_hash = %blob_hash,
                                                size = data.len(),
                                                already_existed = result.already_existed,
                                                "quilt draw: blob stocked in pantry after replication"
                                            );
                                        }
                                        Err(e) => {
                                            warn!(
                                                content_id = %content_id,
                                                blob_hash = %blob_hash,
                                                error = %e,
                                                "quilt draw: failed to stock blob in pantry"
                                            );
                                        }
                                    }
                                }
                                ShardResponse::NotFound => {
                                    warn!(
                                        content_id = %content_id,
                                        blob_hash = %blob_hash,
                                        "quilt draw: peer does not have blob (NotFound)"
                                    );
                                }
                                _ => {
                                    warn!(
                                        content_id = %content_id,
                                        blob_hash = %blob_hash,
                                        "quilt draw: unexpected response to blob pull"
                                    );
                                }
                            }
                        }
                        // Check pending fetch requests
                        else {
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
                                        let _ =
                                            tx.send(Err("Unexpected response to push".to_string()));
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
                                            ShardResponse::Have(false)
                                            | ShardResponse::NotFound => {
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
                                // Handle replication discovery responses (no pending map entry)
                                match response {
                                    ShardResponse::ContentList {
                                        items,
                                        total,
                                        has_more,
                                    } => {
                                        info!(
                                            count = items.len(),
                                            total = total,
                                            has_more = has_more,
                                            "Received content inventory from peer"
                                        );
                                        let remote_ids: Vec<String> =
                                            items.into_iter().map(|i| i.id).collect();
                                        let new_gaps =
                                            self.replication_state.discover(remote_ids).await;

                                        if new_gaps.is_empty() {
                                            debug!("No new content to replicate");
                                            self.replication_state.update_caught_up().await;
                                        } else {
                                            info!(
                                                gaps = new_gaps.len(),
                                                "Queued content gaps for replication"
                                            );
                                            // Enqueue — drain_gap_queue() dispatches adaptively
                                            // on the 5s interval, bounded by MAX_REPLICATION_INFLIGHT.
                                            self.gap_queue.lock().await.extend(new_gaps);
                                        }
                                    }
                                    ShardResponse::Content(record) => {
                                        let content_id = record.id.clone();
                                        // Capture blob_hash before record is moved into input.
                                        let blob_hash_opt = record.blob_hash.clone();
                                        debug!(id = %content_id, "Received content record from peer");
                                        // Remove the in-flight tracking entry (success path)
                                        self.pending_replication_fetches
                                            .lock()
                                            .await
                                            .remove(&request_id);

                                        let pool = match self.db_pool.as_ref() {
                                            Some(p) => p,
                                            None => {
                                                self.replication_state
                                                    .mark_failed(&content_id)
                                                    .await;
                                                return;
                                            }
                                        };
                                        let mut conn = match pool.get() {
                                            Ok(c) => c,
                                            Err(_) => {
                                                self.replication_state
                                                    .mark_failed(&content_id)
                                                    .await;
                                                return;
                                            }
                                        };

                                        let input = crate::db::content_diesel::CreateContentInput {
                                            id: record.id,
                                            title: record.title,
                                            description: record.description,
                                            content_type: record.content_type,
                                            content_format: record.content_format,
                                            blob_hash: record.blob_hash,
                                            blob_cid: record.blob_cid,
                                            content_size_bytes: record.content_size_bytes,
                                            metadata_json: record.metadata_json,
                                            reach: record.reach,
                                            created_by: record.created_by,
                                            tags: record.tags,
                                            content_body: record.content_body,
                                        };

                                        let app_ctx = crate::db::AppContext::default_lamad();
                                        match crate::db::content_diesel::bulk_create_content(
                                            &mut conn,
                                            &app_ctx,
                                            vec![input],
                                        ) {
                                            Ok(result) => {
                                                if result.inserted > 0 || result.skipped > 0 {
                                                    self.replication_state
                                                        .mark_completed(&content_id)
                                                        .await;

                                                    // Republish EPR Head so other peers can discover from us
                                                    if let Some(head_bytes) =
                                                        self.resolve_epr_head_locally(&content_id)
                                                    {
                                                        let key = RecordKey::new(&format!(
                                                            "epr:{}",
                                                            content_id
                                                        ));
                                                        let dht_record = Record {
                                                            key,
                                                            value: head_bytes,
                                                            publisher: Some(
                                                                *self.identity.peer_id(),
                                                            ),
                                                            expires: None,
                                                        };
                                                        let mut swarm = self.swarm.write().await;
                                                        let _ = swarm
                                                            .behaviour_mut()
                                                            .kademlia
                                                            .put_record(
                                                                dht_record,
                                                                libp2p::kad::Quorum::One,
                                                            );
                                                    }

                                                    // Pull the blob bytes from the same peer that
                                                    // served the content record, if this content
                                                    // has an associated blob and we don't have it.
                                                    //
                                                    // This is the missing link: replication was
                                                    // previously metadata-only.  The quilt draw
                                                    // completes the pantry stock on each peer.
                                                    if let Some(ref hash) = blob_hash_opt {
                                                        if !hash.is_empty()
                                                            && !self.blob_store.exists(hash).await
                                                        {
                                                            let pull_request = ShardRequest::Get {
                                                                hash: hash.clone(),
                                                            };
                                                            let mut swarm =
                                                                self.swarm.write().await;
                                                            let pull_id = swarm
                                                                .behaviour_mut()
                                                                .shard_protocol
                                                                .send_request(&peer, pull_request);
                                                            drop(swarm);
                                                            self.pending_blob_pulls
                                                                .lock()
                                                                .await
                                                                .insert(
                                                                    pull_id,
                                                                    (
                                                                        content_id.clone(),
                                                                        hash.clone(),
                                                                    ),
                                                                );
                                                            info!(
                                                                content_id = %content_id,
                                                                blob_hash = %hash,
                                                                source_peer = %peer,
                                                                "quilt draw: pulling blob from peer after content replication"
                                                            );
                                                        }
                                                    }
                                                } else {
                                                    self.replication_state
                                                        .mark_failed(&content_id)
                                                        .await;
                                                }
                                            }
                                            Err(e) => {
                                                warn!(id = %content_id, error = %e, "Failed to store replicated content");
                                                self.replication_state
                                                    .mark_failed(&content_id)
                                                    .await;
                                            }
                                        }
                                        self.replication_state.update_caught_up().await;
                                    }
                                    ShardResponse::ContentNotFound => {
                                        if let Some(content_id) = self
                                            .pending_replication_fetches
                                            .lock()
                                            .await
                                            .remove(&request_id)
                                        {
                                            debug!(content_id = %content_id, "Content not found on peer, marking failed");
                                            self.replication_state.mark_failed(&content_id).await;
                                            self.replication_state.update_caught_up().await;
                                        } else {
                                            debug!("Content not found on peer (no pending replication request)");
                                        }
                                    }
                                    _ => {
                                        debug!(request_id = ?request_id, response = ?response, "Received shard response");
                                    }
                                }
                            }
                        } // close outer else { // Check pending fetch requests ... }
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
                // Clean up any pending blob pull (fire-and-store, no channel to close)
                if let Some((content_id, blob_hash)) =
                    self.pending_blob_pulls.lock().await.remove(&request_id)
                {
                    warn!(
                        content_id = %content_id,
                        blob_hash = %blob_hash,
                        error = ?error,
                        "quilt draw: blob pull failed at transport level"
                    );
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
                // Clean up replication state if this was a replication fetch
                if let Some(content_id) = self
                    .pending_replication_fetches
                    .lock()
                    .await
                    .remove(&request_id)
                {
                    debug!(content_id = %content_id, error = ?error, "Replication fetch failed at transport level");
                    self.replication_state.mark_failed(&content_id).await;
                    self.replication_state.update_caught_up().await;
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
            // D.7: handle progressive GetProviders results.
            // `FoundProviders` may arrive multiple times for a single query (kad emits
            // one event per batch of discovered providers). Accumulate into
            // `pending_kad_get_providers` until `step.last` fires, then deliver.
            behaviour::ElohimStorageBehaviourEvent::Kademlia(
                kad::Event::OutboundQueryProgressed {
                    id,
                    result:
                        kad::QueryResult::GetProviders(Ok(kad::GetProvidersOk::FoundProviders {
                            providers,
                            ..
                        })),
                    step,
                    ..
                },
            ) => {
                let mut pending = self.pending_kad_get_providers.lock().await;
                if let Some((acc, _)) = pending.get_mut(&id) {
                    acc.extend(providers);
                }
                if step.last {
                    if let Some((acc, tx)) = pending.remove(&id) {
                        debug!(
                            target: "elohim_storage::epr",
                            query_id = ?id,
                            provider_count = acc.len(),
                            "kad_get_providers: query finished with providers (step.last)"
                        );
                        let _ = tx.send(acc);
                    }
                }
            }
            behaviour::ElohimStorageBehaviourEvent::Kademlia(
                kad::Event::OutboundQueryProgressed {
                    id,
                    result:
                        kad::QueryResult::GetProviders(Ok(
                            kad::GetProvidersOk::FinishedWithNoAdditionalRecord { .. },
                        )),
                    step,
                    ..
                },
            ) => {
                if step.last {
                    let mut pending = self.pending_kad_get_providers.lock().await;
                    if let Some((acc, tx)) = pending.remove(&id) {
                        debug!(
                            target: "elohim_storage::epr",
                            query_id = ?id,
                            provider_count = acc.len(),
                            "kad_get_providers: query finished (no additional records, step.last)"
                        );
                        let _ = tx.send(acc);
                    }
                }
            }
            behaviour::ElohimStorageBehaviourEvent::Kademlia(
                kad::Event::OutboundQueryProgressed {
                    id,
                    result: kad::QueryResult::GetProviders(Err(e)),
                    ..
                },
            ) => {
                let mut pending = self.pending_kad_get_providers.lock().await;
                if let Some((_, tx)) = pending.remove(&id) {
                    warn!(
                        target: "elohim_storage::epr",
                        query_id = ?id,
                        error = ?e,
                        "kad_get_providers: query failed — returning empty provider list"
                    );
                    let _ = tx.send(Vec::new());
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

            // === EPR atom federation events (/elohim/epr-atom/1.0.0) ===
            behaviour::ElohimStorageBehaviourEvent::EprAtomProtocol(
                request_response::Event::Message { peer, message },
            ) => match message {
                request_response::Message::Request {
                    request, channel, ..
                } => {
                    debug!(peer = %peer, request = ?request, "Received EPR atom request");
                    let response = self.handle_epr_atom_request(peer, request).await;
                    let mut swarm = self.swarm.write().await;
                    if let Err(e) = swarm
                        .behaviour_mut()
                        .epr_atom_protocol
                        .send_response(channel, response)
                    {
                        warn!(peer = %peer, error = ?e, "Failed to send EPR atom response");
                    }
                }
                request_response::Message::Response {
                    request_id,
                    response,
                } => {
                    self.handle_epr_atom_response(peer, request_id, response)
                        .await;
                }
            },
            behaviour::ElohimStorageBehaviourEvent::EprAtomProtocol(
                request_response::Event::OutboundFailure {
                    peer,
                    request_id,
                    error,
                },
            ) => {
                warn!(peer = %peer, request_id = ?request_id, error = ?error, "Outbound EPR atom request failed");
                // P3.4: deliver None to any FetchEprAtomFromPeer waiter so the store
                // caller doesn't hang waiting on a dead request.
                if let Some(tx) = self
                    .pending_epr_atom_fetches
                    .lock()
                    .await
                    .remove(&request_id)
                {
                    let _ = tx.send(None);
                }
            }
            behaviour::ElohimStorageBehaviourEvent::EprAtomProtocol(
                request_response::Event::InboundFailure {
                    peer,
                    request_id,
                    error,
                },
            ) => {
                warn!(peer = %peer, request_id = ?request_id, error = ?error, "Inbound EPR atom request failed");
            }
            behaviour::ElohimStorageBehaviourEvent::EprAtomProtocol(
                request_response::Event::ResponseSent { peer, request_id },
            ) => {
                debug!(peer = %peer, request_id = ?request_id, "EPR atom response sent");
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

            // === Identity handshake events (/elohim/identity/handshake/1.0.0) ===
            behaviour::ElohimStorageBehaviourEvent::IdentityHandshake(
                request_response::Event::Message { peer, message },
            ) => match message {
                request_response::Message::Request {
                    request, channel, ..
                } => {
                    debug!(
                        peer = %peer,
                        agent_cid = %request.binding.agent_cid,
                        "Received identity handshake"
                    );
                    let now_iso = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
                    let peer_id_str = peer.to_base58();
                    let response = match identity_handshake::verify_handshake_request(
                        &request,
                        &peer_id_str,
                        &now_iso,
                    ) {
                        identity_handshake::VerifyOutcome::Invalid(reason) => {
                            info!(
                                peer = %peer,
                                reason = %reason,
                                "Identity handshake rejected"
                            );
                            identity_handshake::IdentityHandshakeResponse::Rejected { reason }
                        }
                        identity_handshake::VerifyOutcome::Valid => {
                            // Insert into peer_identity_bindings with source='handshake'.
                            // The row construction is centralised in
                            // `binding_row_from_handshake_request` so that the field-flow
                            // contract is exercised by the writer here AND the T03b
                            // regression test in identity_handshake.rs.
                            let row = identity_handshake::binding_row_from_handshake_request(
                                &request,
                                &peer_id_str,
                                &now_iso,
                            );
                            match self.db_pool.as_ref() {
                                Some(pool) => match pool.get() {
                                    Ok(mut conn) => {
                                        // Non-authoritative writer: must NOT clobber any
                                        // `superseded_by` previously written by the
                                        // DHT-arrival path. Use the preserving variant.
                                        match crate::db::peer_identity_bindings::upsert_preserving_supersession(
                                            &mut conn, &row,
                                        ) {
                                            Ok(()) => {
                                                debug!(
                                                    peer = %peer,
                                                    agent_cid = %row.agent_cid,
                                                    "Identity handshake: binding recorded"
                                                );
                                                identity_handshake::IdentityHandshakeResponse::Accepted
                                            }
                                            Err(e) => {
                                                warn!(
                                                    peer = %peer,
                                                    error = %e,
                                                    "Identity handshake: db upsert failed"
                                                );
                                                identity_handshake::IdentityHandshakeResponse::Error(
                                                    format!("db error: {e}"),
                                                )
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        warn!(
                                            peer = %peer,
                                            error = %e,
                                            "Identity handshake: pool exhausted"
                                        );
                                        identity_handshake::IdentityHandshakeResponse::Error(
                                            "pool exhausted".to_string(),
                                        )
                                    }
                                },
                                None => {
                                    // No pool configured — accept but do not persist.
                                    debug!(
                                        peer = %peer,
                                        "Identity handshake: no db_pool configured, skipping persistence"
                                    );
                                    identity_handshake::IdentityHandshakeResponse::Accepted
                                }
                            }
                        }
                    };
                    let mut swarm = self.swarm.write().await;
                    if let Err(e) = swarm
                        .behaviour_mut()
                        .identity_handshake
                        .send_response(channel, response)
                    {
                        warn!(peer = %peer, error = ?e, "Failed to send identity handshake response");
                    }
                }
                request_response::Message::Response { response, .. } => match response {
                    identity_handshake::IdentityHandshakeResponse::Accepted => {
                        debug!(peer = %peer, "Identity handshake accepted by remote");
                    }
                    identity_handshake::IdentityHandshakeResponse::Rejected { reason } => {
                        info!(
                            peer = %peer,
                            reason = %reason,
                            "Remote rejected our identity handshake"
                        );
                    }
                    identity_handshake::IdentityHandshakeResponse::Error(msg) => {
                        warn!(peer = %peer, error = %msg, "Remote error on identity handshake");
                    }
                },
            },
            behaviour::ElohimStorageBehaviourEvent::IdentityHandshake(
                request_response::Event::OutboundFailure { peer, error, .. },
            ) => {
                debug!(
                    peer = %peer,
                    error = ?error,
                    "Identity handshake outbound failure"
                );
            }
            behaviour::ElohimStorageBehaviourEvent::IdentityHandshake(
                request_response::Event::InboundFailure { peer, error, .. },
            ) => {
                debug!(
                    peer = %peer,
                    error = ?error,
                    "Identity handshake inbound failure"
                );
            }
            behaviour::ElohimStorageBehaviourEvent::IdentityHandshake(
                request_response::Event::ResponseSent { peer, .. },
            ) => {
                debug!(peer = %peer, "Identity handshake response sent");
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
                // Cache identify info for /p2p/peers endpoint
                self.identify_cache.insert(
                    peer_id.to_string(),
                    CachedIdentifyInfo {
                        agent_version: info.agent_version.clone(),
                        protocols: info.protocols.iter().map(|p| p.to_string()).collect(),
                        listen_addrs: info.listen_addrs.iter().map(|a| a.to_string()).collect(),
                    },
                );
                if let Some(mut m) = self.peer_metrics.get_mut(&peer_id.to_string()) {
                    m.last_seen_ms = now_unix_ms();
                }
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
                    autonat::NatStatus::Public(_addr) => "public",
                    autonat::NatStatus::Private => "private",
                    autonat::NatStatus::Unknown => "unknown",
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

            behaviour::ElohimStorageBehaviourEvent::Ping(ping::Event {
                peer,
                result: Ok(rtt),
                ..
            }) => {
                let pid = peer.to_string();
                self.peer_metrics
                    .entry(pid)
                    .and_modify(|m| {
                        if m.rtt_samples.len() >= 8 {
                            m.rtt_samples.pop_front();
                        }
                        m.rtt_samples.push_back(rtt);
                        m.last_seen_ms = now_unix_ms();
                    })
                    .or_insert_with(|| {
                        let mut samples = std::collections::VecDeque::with_capacity(8);
                        samples.push_back(rtt);
                        PeerMetrics {
                            is_connected: true,
                            direction: "unknown",
                            last_seen_ms: now_unix_ms(),
                            rtt_samples: samples,
                        }
                    });
            }
            behaviour::ElohimStorageBehaviourEvent::Ping(ping::Event {
                result: Err(_), ..
            }) => {}

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

            behaviour::ElohimStorageBehaviourEvent::Gossipsub(event) => {
                use libp2p::gossipsub::Event as GossipsubEvent;
                match event {
                    GossipsubEvent::Message {
                        propagation_source,
                        message_id,
                        message,
                    } => {
                        if message.topic.as_str() == RECOVERY_INVITATION_TOPIC {
                            match crate::p2p::recovery_invitation::RecoveryInvitation::from_bytes(
                                &message.data,
                            ) {
                                Ok(inv) => info!(
                                    target: "elohim_storage::recovery",
                                    from = %propagation_source,
                                    message_id = ?message_id,
                                    request_hash = %inv.request_hash,
                                    human_id = %inv.human_id,
                                    "Received recovery invitation"
                                ),
                                Err(e) => warn!(
                                    target: "elohim_storage::recovery",
                                    from = %propagation_source,
                                    error = ?e,
                                    "Failed to decode RecoveryInvitation"
                                ),
                            }
                        } else if message.topic.as_str()
                            == crate::p2p::identity_binding_gossip::IDENTITY_BINDING_TOPIC
                        {
                            // A.10: receive an identity binding from a peer and upsert
                            // into peer_identity_bindings with source='gossip'.
                            // Stage 1: structural verification only (non-empty fields).
                            // Stage 2: add Ed25519 verify against resolved pubkey.
                            match crate::p2p::identity_binding_gossip::IdentityBindingGossip::from_bytes(
                                &message.data,
                            ) {
                                Ok(payload) => {
                                    match payload.verify_structural() {
                                        Err(reason) => warn!(
                                            target: "elohim_storage::identity",
                                            from = %propagation_source,
                                            reason = %reason,
                                            "IdentityBindingGossip failed structural verify — dropped"
                                        ),
                                        Ok(()) => {
                                            let now_iso = chrono::Utc::now()
                                                .format("%Y-%m-%dT%H:%M:%SZ")
                                                .to_string();
                                            // Row construction centralised in
                                            // `binding_row_from_gossip` so the field-flow
                                            // contract is exercised by the writer here AND
                                            // the T03b regression test in
                                            // identity_binding_gossip.rs.
                                            let row =
                                                crate::p2p::identity_binding_gossip::binding_row_from_gossip(
                                                    &payload, &now_iso,
                                                );
                                            match self.db_pool.as_ref() {
                                                Some(pool) => {
                                                    match pool.get() {
                                                        Ok(mut conn) => {
                                                            // Non-authoritative writer: must NOT clobber a
                                                            // `superseded_by` previously set by the DHT-arrival
                                                            // path. Use the preserving variant.
                                                            match crate::db::peer_identity_bindings::upsert_preserving_supersession(&mut conn, &row) {
                                                                Ok(()) => info!(
                                                                    target: "elohim_storage::identity",
                                                                    from = %propagation_source,
                                                                    peer_id = %payload.peer_id,
                                                                    agent_cid = %payload.agent_cid,
                                                                    "IdentityBindingGossip upserted with source='gossip'"
                                                                ),
                                                                Err(e) => warn!(
                                                                    target: "elohim_storage::identity",
                                                                    from = %propagation_source,
                                                                    peer_id = %payload.peer_id,
                                                                    error = %e,
                                                                    "IdentityBindingGossip db upsert failed"
                                                                ),
                                                            }
                                                        }
                                                        Err(e) => warn!(
                                                            target: "elohim_storage::identity",
                                                            peer_id = %payload.peer_id,
                                                            error = %e,
                                                            "IdentityBindingGossip: db pool exhausted"
                                                        ),
                                                    }
                                                }
                                                None => debug!(
                                                    target: "elohim_storage::identity",
                                                    peer_id = %payload.peer_id,
                                                    "IdentityBindingGossip: no db_pool configured, skipping persistence"
                                                ),
                                            }
                                            // device_archetype is wired from the gossip wire; superseded_by
                                            // relies on the DHT-arrival path for authoritative supersession.

                                            // NOTE: reconcile signal emission deferred — the controller processes only
                                            // DNA signals in Stage 1. A P2P-received binding reaching the reconcile layer
                                            // (for cache invalidation, etc.) is an A.12 concern.

                                            // TODO(A.12): invalidate the remote agent's pubkey_timeline cache entry here.
                                            // The !Send cache lives in the controller; the receive arm cannot reach it
                                            // without restructuring. Deferred to the full controller signal-flow landing
                                            // in A.12.
                                        }
                                    }
                                }
                                Err(e) => warn!(
                                    target: "elohim_storage::identity",
                                    from = %propagation_source,
                                    error = ?e,
                                    "Failed to decode IdentityBindingGossip"
                                ),
                            }
                        } else if message.topic.as_str() == RECOVERY_REVOCATION_TOPIC {
                            // M4: subscribe/log stub. Active consumer logic lands in M5
                            // (elohim defender + UI). Log is the seam M5 hooks into.
                            // TODO: factor shared body into handle_revocation_message helper
                            // once the event-loop borrow structure allows it (see arm below).
                            match crate::p2p::recovery_revocation::RecoveryRevocationMessage::from_bytes(
                                &message.data,
                            ) {
                                Ok(msg) => {
                                    // D.6 wire point B: dedup on the same synthetic key as
                                    // the direct-notify path (wire point C) so a revocation
                                    // arriving via gossipsub after direct-notify (or vice
                                    // versa) is dropped before any projection work.
                                    // Synthetic dedup key: `KeyRevocation:{revocation_id}` namespace. UUIDs
                                    // don't collide with EPR CIDs (which start with "bafy"). Future integrity
                                    // kinds should use `KeyRotation:{id}` / `AgentPeerBinding:{id}` etc. — the
                                    // namespace prefix prevents cross-kind collisions even if id formats overlap.
                                    let dedup_key =
                                        format!("KeyRevocation:{}", msg.revocation_id);
                                    if !self.dedup.insert(&dedup_key) {
                                        debug!(
                                            target: "elohim_storage::dedup",
                                            from = %propagation_source,
                                            revocation_id = %msg.revocation_id,
                                            "duplicate gossip revocation — dropped"
                                        );
                                    } else {
                                        info!(
                                            target: "recovery.revocation.inbound",
                                            from = %propagation_source,
                                            message_id = ?message_id,
                                            revocation_id = %msg.revocation_id,
                                            human_id = %msg.human_id,
                                            status = %msg.status,
                                            "Received recovery revocation"
                                        );
                                    }
                                }
                                Err(e) => warn!(
                                    target: "recovery.revocation.inbound",
                                    from = %propagation_source,
                                    error = ?e,
                                    "Failed to decode RecoveryRevocationMessage"
                                ),
                            }
                        } else if message.topic.as_str()
                            == crate::p2p::topics::TOPIC_INTEGRITY_REVOCATION
                        {
                            // D.6 Fix 1 (CRITICAL): canonical integrity-revocation topic arm.
                            // TOPIC_INTEGRITY_REVOCATION = "elohim/integrity/revocation" is the
                            // new canonical name. M3/M4 publishers still use RECOVERY_REVOCATION_TOPIC
                            // = "recovery.revocation" (arm above); new publishers (D.5+) use this name.
                            // BOTH arms must route to the same handler with the same dedup key.
                            // TODO: factor into handle_revocation_message helper when event-loop
                            // borrow structure permits; body kept in sync with RECOVERY_REVOCATION_TOPIC arm.
                            match crate::p2p::recovery_revocation::RecoveryRevocationMessage::from_bytes(
                                &message.data,
                            ) {
                                Ok(msg) => {
                                    // Same synthetic dedup key as wire points B and C — cross-channel
                                    // dedup contract from D.6: regardless of which topic name a
                                    // revocation arrives on, KeyRevocation:{revocation_id} is the key.
                                    // See first KeyRevocation: dedup site for namespace rationale.
                                    let dedup_key =
                                        format!("KeyRevocation:{}", msg.revocation_id);
                                    if !self.dedup.insert(&dedup_key) {
                                        debug!(
                                            target: "elohim_storage::dedup",
                                            from = %propagation_source,
                                            revocation_id = %msg.revocation_id,
                                            topic = crate::p2p::topics::TOPIC_INTEGRITY_REVOCATION,
                                            "duplicate integrity/revocation gossip — dropped"
                                        );
                                    } else {
                                        info!(
                                            target: "recovery.revocation.inbound",
                                            from = %propagation_source,
                                            message_id = ?message_id,
                                            revocation_id = %msg.revocation_id,
                                            human_id = %msg.human_id,
                                            status = %msg.status,
                                            topic = crate::p2p::topics::TOPIC_INTEGRITY_REVOCATION,
                                            "Received recovery revocation (canonical topic)"
                                        );
                                    }
                                }
                                Err(e) => warn!(
                                    target: "recovery.revocation.inbound",
                                    from = %propagation_source,
                                    error = ?e,
                                    topic = crate::p2p::topics::TOPIC_INTEGRITY_REVOCATION,
                                    "Failed to decode RecoveryRevocationMessage (canonical topic)"
                                ),
                            }
                        } else if message.topic.as_str()
                            == crate::p2p::inventory_gossip::INVENTORY_TOPIC
                        {
                            // T14: receive blob inventory snapshot or delta from a peer.
                            // Try snapshot first, then delta. We don't have a wire-level
                            // discriminator; distinguishing relies on serde — snapshots have
                            // `hashes` (no `added`/`removed`) and deltas have `added`/`removed`
                            // (no `hashes`). serde will fail one and accept the other.

                            use crate::p2p::inventory_gossip::{
                                BlobInventoryDelta, BlobInventorySnapshot,
                            };

                            if let Ok(snapshot) = BlobInventorySnapshot::from_bytes(&message.data) {
                                if let Err(e) = snapshot.verify_structural() {
                                    warn!(
                                        target: "elohim_storage::inventory",
                                        from = %propagation_source,
                                        error = ?e,
                                        "Inventory snapshot failed structural verify — dropped"
                                    );
                                } else if let Some(pool) = self.db_pool.as_ref() {
                                    match pool.get() {
                                        Ok(mut conn) => {
                                            let now_iso = chrono::Utc::now()
                                                .format("%Y-%m-%dT%H:%M:%SZ")
                                                .to_string();
                                            let when = micros_to_iso(snapshot.snapshot_at)
                                                .unwrap_or(now_iso);
                                            match crate::db::peer_blob_inventory::apply_snapshot(
                                                &mut conn,
                                                &snapshot.peer_id,
                                                &snapshot.hashes,
                                                snapshot.sequence as i64,
                                                &when,
                                            ) {
                                                Ok(()) => debug!(
                                                    target: "elohim_storage::inventory",
                                                    from = %propagation_source,
                                                    peer_id = %snapshot.peer_id,
                                                    count = snapshot.hashes.len(),
                                                    sequence = snapshot.sequence,
                                                    "Inventory snapshot applied"
                                                ),
                                                Err(e) => warn!(
                                                    target: "elohim_storage::inventory",
                                                    from = %propagation_source,
                                                    error = %e,
                                                    "apply_snapshot failed"
                                                ),
                                            }
                                        }
                                        Err(e) => warn!(
                                            target: "elohim_storage::inventory",
                                            error = %e,
                                            "inventory: db pool exhausted"
                                        ),
                                    }
                                }
                            } else if let Ok(delta) = BlobInventoryDelta::from_bytes(&message.data)
                            {
                                if let Err(e) = delta.verify_structural() {
                                    warn!(
                                        target: "elohim_storage::inventory",
                                        from = %propagation_source,
                                        error = ?e,
                                        "Inventory delta failed structural verify — dropped"
                                    );
                                } else if let Some(pool) = self.db_pool.as_ref() {
                                    match pool.get() {
                                        Ok(mut conn) => {
                                            let when = micros_to_iso(delta.emitted_at)
                                                .unwrap_or_else(|| {
                                                    chrono::Utc::now()
                                                        .format("%Y-%m-%dT%H:%M:%SZ")
                                                        .to_string()
                                                });
                                            match crate::db::peer_blob_inventory::apply_delta(
                                                &mut conn,
                                                &delta.peer_id,
                                                &delta.added,
                                                &delta.removed,
                                                delta.sequence as i64,
                                                &when,
                                            ) {
                                                Ok(crate::db::peer_blob_inventory::DeltaApplyOutcome::Applied) => {
                                                    debug!(
                                                        target: "elohim_storage::inventory",
                                                        peer_id = %delta.peer_id,
                                                        sequence = delta.sequence,
                                                        added = delta.added.len(),
                                                        removed = delta.removed.len(),
                                                        "Inventory delta applied"
                                                    );
                                                }
                                                Ok(crate::db::peer_blob_inventory::DeltaApplyOutcome::Replay) => {
                                                    debug!(
                                                        target: "elohim_storage::inventory",
                                                        peer_id = %delta.peer_id,
                                                        sequence = delta.sequence,
                                                        "Inventory delta replay — dropped silently"
                                                    );
                                                }
                                                Ok(crate::db::peer_blob_inventory::DeltaApplyOutcome::Gap {
                                                    expected,
                                                    received,
                                                }) => {
                                                    warn!(
                                                        target: "elohim_storage::inventory",
                                                        peer_id = %delta.peer_id,
                                                        expected,
                                                        received,
                                                        "Inventory delta gap — requesting snapshot"
                                                    );
                                                    // Best-effort: send the snapshot-request command.
                                                    // Parse the peer_id string into a libp2p::PeerId.
                                                    if let Ok(pid) = delta.peer_id.parse::<libp2p::PeerId>() {
                                                        let cmd = P2PCommand::SnapshotRequest { peer_id: pid };
                                                        let _ = self.command_tx.try_send(cmd);
                                                    }
                                                }
                                                Err(e) => warn!(
                                                    target: "elohim_storage::inventory",
                                                    error = %e,
                                                    "apply_delta failed"
                                                ),
                                            }
                                        }
                                        Err(e) => warn!(
                                            target: "elohim_storage::inventory",
                                            error = %e,
                                            "inventory: db pool exhausted"
                                        ),
                                    }
                                }
                            } else {
                                debug!(
                                    target: "elohim_storage::inventory",
                                    from = %propagation_source,
                                    "Inventory message decoded as neither snapshot nor delta — dropped"
                                );
                            }
                        } else if message.topic.as_str().starts_with("elohim/") {
                            // D.6 wire point B (gossipsub EPR announce): per-pillar
                            // reach-scoped topics carry a msgpack-encoded CID string
                            // (announce-only; receivers fetch the full atom via EPR atom
                            // protocol if wanted). Stage 1: decode CID + dedup only.
                            // Full receive-side projection is downstream (Phase 3+).
                            match rmp_serde::from_slice::<String>(&message.data) {
                                Ok(cid) => {
                                    if !self.dedup.insert(&cid) {
                                        debug!(
                                            target: "elohim_storage::dedup",
                                            from = %propagation_source,
                                            topic = %message.topic,
                                            cid = %cid,
                                            "duplicate gossip announce — dropped"
                                        );
                                    } else {
                                        debug!(
                                            target: "elohim_storage::epr",
                                            from = %propagation_source,
                                            topic = %message.topic,
                                            cid = %cid,
                                            "gossip EPR announce (deduped; full fetch deferred to Phase 3+)"
                                        );
                                    }
                                }
                                Err(e) => {
                                    debug!(
                                        target: "elohim_storage::epr",
                                        from = %propagation_source,
                                        topic = %message.topic,
                                        error = %e,
                                        "gossip EPR announce: msgpack decode failed"
                                    );
                                }
                            }
                        } else {
                            debug!(topic = %message.topic, "Gossipsub message on untracked topic");
                        }
                    }
                    other => debug!(event = ?other, "Gossipsub event"),
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
            ShardRequest::ListContent {
                reach_filter,
                offset,
                limit,
            } => {
                // Validate reach_filter against schema-generated constants so
                // unknown strings don't silently return empty results.
                if let Some(ref r) = reach_filter {
                    if !crate::generated_enums::CORE_REACH_LEVELS.contains(&r.as_str()) {
                        return ShardResponse::Error(format!(
                            "Unknown reach level {:?}. Valid values: {:?}",
                            r,
                            crate::generated_enums::CORE_REACH_LEVELS
                        ));
                    }
                }
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
                // P2P shard inventory — internal peer-to-peer protocol, not
                // web2 HTTP. Peers must see all local rows so replication can
                // cover pre-drain content.
                match crate::db::content_diesel::list_content(&mut conn, &app_ctx, &query, false) {
                    Ok(items) => {
                        let total = crate::db::content_diesel::count_content(
                            &mut conn, &app_ctx, &query, false,
                        )
                        .unwrap_or(items.len() as i64) as u64;
                        let inventory: Vec<shard_protocol::ContentInventoryItem> = items
                            .iter()
                            .map(|cwt| shard_protocol::ContentInventoryItem {
                                id: cwt.content.id.clone(),
                                title: cwt.content.title.clone(),
                                content_type: cwt.content.content_type.clone(),
                                content_format: cwt.content.content_format.clone(),
                                reach: cwt.content.reach.clone(),
                                blob_cid: cwt.content.blob_cid.clone(),
                                updated_at: cwt.content.updated_at.clone(),
                            })
                            .collect();
                        let has_more = (offset as u64 + inventory.len() as u64) < total;
                        info!(
                            count = inventory.len(),
                            total = total,
                            "Serving content inventory"
                        );
                        ShardResponse::ContentList {
                            items: inventory,
                            total,
                            has_more,
                        }
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
                match crate::db::content_diesel::get_content_with_tags(
                    &mut conn, &app_ctx, &id, false,
                ) {
                    Ok(Some(cwt)) => {
                        debug!(id = %id, "Serving content record to peer");
                        ShardResponse::Content(Box::new(shard_protocol::ContentRecord {
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
                        }))
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
        // Internal P2P-side resolution; provenance gate off so the drain loop
        // (which also rides this path) can project pre-publish rows.  Pillar
        // enrichment ON: peers receive full stewardship + attestation context.
        match crate::epr_head::derive_epr_head(&mut conn, &app_ctx, id, false, true) {
            Ok(Some(head)) => match rmp_serde::to_vec(&head) {
                Ok(bytes) => Some(bytes),
                Err(e) => {
                    warn!(id = %id, error = %e, "Failed to encode EPR Head");
                    None
                }
            },
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
                    &mut conn, &app_ctx, content_id, false,
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

    /// Handle an incoming EPR atom federation request from a peer.
    ///
    /// Batch C (Tasks 11–14) replaces these stubs with real fetch/announce
    /// logic that reads/writes the `epr_atoms` projection and enforces the
    /// reach gate. During Batch B, the protocol is wired end-to-end but
    /// returns shape-correct placeholders.
    async fn handle_epr_atom_request(
        &self,
        peer: libp2p::PeerId,
        request: EprAtomRequest,
    ) -> EprAtomResponse {
        let caller = self.identity_map.lookup(&peer);

        match request {
            EprAtomRequest::Fetch { cid } => {
                let Some(pool) = self.db_pool.as_ref() else {
                    warn!(cid = %cid, "EPR atom fetch: db pool unavailable");
                    return EprAtomResponse::Error {
                        message: "storage unavailable".to_string(),
                    };
                };
                let mut conn = match pool.get() {
                    Ok(c) => c,
                    Err(e) => {
                        warn!(cid = %cid, error = %e, "EPR atom fetch: db pool exhausted");
                        return EprAtomResponse::Error {
                            message: "storage busy".to_string(),
                        };
                    }
                };

                match crate::services::epr_service::fetch_wire_bytes_by_cid(&mut conn, &cid) {
                    Ok(Some(fetched)) => {
                        // Reach gate: Phase 2c — public atoms served to all; authors
                        // may fetch their own private atoms.
                        if reach_gate_allows(&fetched.reach, &caller, Some(&fetched.signer_cid)) {
                            debug!(
                                cid = %cid,
                                reach = %fetched.reach,
                                bytes = fetched.wire_bytes.len(),
                                "EPR atom fetch served"
                            );
                            EprAtomResponse::Atom {
                                envelope_bytes: fetched.wire_bytes,
                            }
                        } else {
                            // Leak-free: caller cannot distinguish missing from unauthorized.
                            debug!(
                                cid = %cid,
                                reach = %fetched.reach,
                                "EPR atom fetch denied by reach gate"
                            );
                            EprAtomResponse::NotFound
                        }
                    }
                    Ok(None) => EprAtomResponse::NotFound,
                    Err(e) => {
                        warn!(cid = %cid, error = ?e, "EPR atom fetch error");
                        EprAtomResponse::Error {
                            message: "internal error".to_string(),
                        }
                    }
                }
            }
            EprAtomRequest::Announce { envelope_bytes } => {
                // D.6: decode the envelope first (CBOR only — no DB I/O) to
                // extract the CID string cheaply, then dedup-check before
                // committing the pool connection or running ingest.
                let epr: elohim_epr::Epr =
                    match ciborium::de::from_reader(envelope_bytes.as_slice()) {
                        Ok(e) => e,
                        Err(err) => {
                            debug!(
                                bytes = envelope_bytes.len(),
                                reason = %err,
                                "EPR atom announce: cbor decode failed"
                            );
                            return EprAtomResponse::Announced {
                                accepted: false,
                                reason: Some(format!("cbor decode: {err}")),
                            };
                        }
                    };
                let cid_str = epr.envelope.cid.to_string();

                // D.6 wire point A: dedup on CID before DB connection + ingest.
                if !self.dedup.insert(&cid_str) {
                    debug!(
                        target: "elohim_storage::dedup",
                        from = %peer,
                        cid = %cid_str,
                        "duplicate Announce — dropped (no-op)"
                    );
                    // D.6: dedup hit. Respond accepted=false so the sender doesn't treat this
                    // as a fresh acceptance, but include reason so they don't retry. The atom
                    // is already persisted from the original delivery (LocalEprStore::put is
                    // idempotent), so accepted=false here means "no NEW ingestion happened."
                    return EprAtomResponse::Announced {
                        accepted: false,
                        reason: Some("duplicate (already seen)".to_string()),
                    };
                }

                let Some(pool) = self.db_pool.as_ref() else {
                    warn!(
                        bytes = envelope_bytes.len(),
                        "EPR atom announce: db pool unavailable"
                    );
                    return EprAtomResponse::Announced {
                        accepted: false,
                        reason: Some("storage unavailable".to_string()),
                    };
                };
                let mut conn = match pool.get() {
                    Ok(c) => c,
                    Err(e) => {
                        warn!(error = %e, "EPR atom announce: db pool exhausted");
                        return EprAtomResponse::Announced {
                            accepted: false,
                            reason: Some("storage busy".to_string()),
                        };
                    }
                };

                // Call ingest() directly with the already-decoded Epr — avoids
                // a second CBOR decode that ingest_from_wire_bytes would do.
                match crate::services::epr_service::ingest(&mut conn, epr) {
                    Ok(ingested) => {
                        debug!(
                            cid = %ingested.cid,
                            bytes = envelope_bytes.len(),
                            "EPR atom announce accepted"
                        );
                        // TODO(T22-followup): call record_predecessor here so the
                        // libp2p ingest path records the sender for back-prop.
                        //
                        // The sender PeerId is available as `peer.to_string()`.
                        // Wiring requires P2PNode to hold an Arc<SealingKeyPair>
                        // (added via a `with_sealing_keys` builder method, mirroring
                        // the existing `with_db_pool` / `with_policy_enforcement`
                        // pattern). Example call site (once sealing_keys field added):
                        //
                        //   if let Some(keys) = &self.sealing_keys {
                        //       let pub_keys = SealingPubKeys {
                        //           mishpat_pk: &keys.mishpat_pk,
                        //           imagodei_pk: &keys.imagodei_pk,
                        //       };
                        //       if let Err(e) = crate::services::back_prop::record_predecessor(
                        //           &mut conn, &ingested.cid, &peer.to_string(), &pub_keys,
                        //       ) {
                        //           warn!(?e, cid = %ingested.cid, "record_predecessor failed (non-fatal)");
                        //       }
                        //   }
                        //
                        // Without this, back_prop_one_hop finds no predecessors and
                        // returns Ok(vec![]) — correct, just not yet forward-propagating.
                        EprAtomResponse::Announced {
                            accepted: true,
                            reason: None,
                        }
                    }
                    Err(crate::error::StorageError::InvalidInput(msg)) => {
                        debug!(bytes = envelope_bytes.len(), reason = %msg, "EPR atom announce rejected (invalid)");
                        EprAtomResponse::Announced {
                            accepted: false,
                            reason: Some(format!("verification failed: {msg}")),
                        }
                    }
                    Err(e) => {
                        warn!(bytes = envelope_bytes.len(), error = ?e, "EPR atom announce: persistence error");
                        EprAtomResponse::Announced {
                            accepted: false,
                            reason: Some("persistence error".to_string()),
                        }
                    }
                }
            }
            EprAtomRequest::FetchBatch { cids } => {
                use crate::p2p::MAX_BATCH_CIDS;
                if cids.len() > MAX_BATCH_CIDS {
                    debug!(
                        count = cids.len(),
                        max = MAX_BATCH_CIDS,
                        "EPR atom fetch batch rejected — oversized"
                    );
                    return EprAtomResponse::Error {
                        message: format!(
                            "batch too large: {} cids (max {})",
                            cids.len(),
                            MAX_BATCH_CIDS
                        ),
                    };
                }

                let Some(pool) = self.db_pool.as_ref() else {
                    warn!(
                        count = cids.len(),
                        "EPR atom fetch batch: db pool unavailable"
                    );
                    return EprAtomResponse::Error {
                        message: "storage unavailable".to_string(),
                    };
                };
                let mut conn = match pool.get() {
                    Ok(c) => c,
                    Err(e) => {
                        warn!(count = cids.len(), error = %e, "EPR atom fetch batch: db pool exhausted");
                        return EprAtomResponse::Error {
                            message: "storage busy".to_string(),
                        };
                    }
                };

                let mut atoms: Vec<Option<Vec<u8>>> = Vec::with_capacity(cids.len());
                let mut served = 0usize;
                for cid in &cids {
                    let slot =
                        match crate::services::epr_service::fetch_wire_bytes_by_cid(&mut conn, cid)
                        {
                            Ok(Some(fetched))
                                if reach_gate_allows(
                                    &fetched.reach,
                                    &caller,
                                    Some(&fetched.signer_cid),
                                ) =>
                            {
                                served += 1;
                                Some(fetched.wire_bytes)
                            }
                            Ok(_) => None,
                            Err(e) => {
                                warn!(cid = %cid, error = ?e, "EPR atom fetch batch: row error");
                                None
                            }
                        };
                    atoms.push(slot);
                }
                debug!(
                    total = cids.len(),
                    served = served,
                    "EPR atom fetch batch completed"
                );
                EprAtomResponse::AtomBatch { atoms }
            }
            // D.5: direct-notify of an integrity event from a peer.
            // Stage 1: only KeyRevocation is handled; other kinds are accepted
            // but logged as not-yet-handled. Receivers decode by kind and route
            // to the same handler the gossipsub receive path uses.
            EprAtomRequest::IntegrityNotify {
                kind,
                payload_bytes,
            } => {
                match kind.as_str() {
                    "KeyRevocation" => {
                        match crate::p2p::recovery_revocation::RecoveryRevocationMessage::from_bytes(
                            &payload_bytes,
                        ) {
                            Ok(msg) => {
                                // D.6 wire point C: dedup on synthetic KeyRevocation:<id>
                                // key. Same revocation arriving via direct-notify + gossipsub
                                // will not double-process after the first delivery.
                                // See first KeyRevocation: dedup site (wire B) for namespace rationale.
                                let dedup_key = format!("KeyRevocation:{}", msg.revocation_id);
                                if !self.dedup.insert(&dedup_key) {
                                    debug!(
                                        target: "elohim_storage::dedup",
                                        from = %peer,
                                        revocation_id = %msg.revocation_id,
                                        "duplicate KeyRevocation direct-notify — dropped"
                                    );
                                    return EprAtomResponse::IntegrityAck {
                                        received: true,
                                        reason: Some("duplicate".to_string()),
                                    };
                                }
                                info!(
                                    target: "elohim_storage::recovery",
                                    from = %peer,
                                    revocation_id = %msg.revocation_id,
                                    human_id = %msg.human_id,
                                    status = %msg.status,
                                    "D.5: Received KeyRevocation via direct-notify"
                                );
                                // The gossipsub receive path for RECOVERY_REVOCATION_TOPIC
                                // currently logs only. When that path adds projection logic,
                                // factor it into a shared helper and call it from both
                                // direct-notify + gossipsub paths. For now, structural
                                // receive + log is sufficient.
                                EprAtomResponse::IntegrityAck {
                                    received: true,
                                    reason: None,
                                }
                            }
                            Err(e) => {
                                warn!(
                                    target: "elohim_storage::recovery",
                                    from = %peer,
                                    error = %e,
                                    "D.5: Failed to decode RecoveryRevocationMessage from direct-notify"
                                );
                                EprAtomResponse::IntegrityAck {
                                    received: false,
                                    reason: Some(format!("decode failed: {e}")),
                                }
                            }
                        }
                    }
                    _ => {
                        warn!(
                            target: "elohim_storage::integrity",
                            from = %peer,
                            kind = %kind,
                            "D.5: Received IntegrityNotify with unhandled kind — Stage 2/3 will add handlers"
                        );
                        EprAtomResponse::IntegrityAck {
                            received: false,
                            reason: Some(format!("unhandled integrity kind: {kind}")),
                        }
                    }
                }
            }
        }
    }

    /// Handle an incoming EPR atom response to one of our outbound requests.
    async fn handle_epr_atom_response(
        &self,
        peer: libp2p::PeerId,
        request_id: request_response::OutboundRequestId,
        response: EprAtomResponse,
    ) {
        // P3.4: deliver to FederatedEprStore::fetch callers waiting on this request.
        if let Some(tx) = self
            .pending_epr_atom_fetches
            .lock()
            .await
            .remove(&request_id)
        {
            let result = match response {
                EprAtomResponse::Atom { envelope_bytes } => {
                    debug!(
                        target: "elohim_storage::epr",
                        peer = %peer,
                        request_id = ?request_id,
                        bytes = envelope_bytes.len(),
                        "P3.4: EprAtomFetch — Atom received from peer"
                    );
                    Some(envelope_bytes)
                }
                EprAtomResponse::NotFound => {
                    debug!(
                        target: "elohim_storage::epr",
                        peer = %peer,
                        request_id = ?request_id,
                        "P3.4: EprAtomFetch — peer reports NotFound"
                    );
                    None
                }
                EprAtomResponse::Error { message } => {
                    warn!(
                        target: "elohim_storage::epr",
                        peer = %peer,
                        request_id = ?request_id,
                        error = %message,
                        "P3.4: EprAtomFetch — peer responded with Error"
                    );
                    None
                }
                other => {
                    // Announce, Announced, AtomBatch, AtomBatchResponse, IntegrityAck, etc.
                    // are not valid responses to a Fetch request; treat as failure.
                    debug!(
                        target: "elohim_storage::epr",
                        peer = %peer,
                        request_id = ?request_id,
                        response = ?other,
                        "P3.4: EprAtomFetch — unexpected response variant; treating as miss"
                    );
                    None
                }
            };
            let _ = tx.send(result);
        } else {
            // Response for an IntegrityNotify, FetchBatch, or other non-fetch request
            // (no pending entry) — log at debug and ignore.
            debug!(
                peer = %peer,
                request_id = ?request_id,
                response = ?response,
                "EPR atom response received (no pending fetch entry — likely Announce/Batch path)"
            );
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
                                &mut conn, &app_ctx, &id, false,
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
            sync_paused: Arc::clone(&self.sync_paused),
            last_gossiped: Arc::clone(&self.last_gossiped),
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

    /// Run one cycle of replication discovery.
    ///
    /// Sends a ListContent request to discover gaps in this node's content
    /// relative to a connected peer. Gaps are added to the gap_queue and
    /// dispatched adaptively by drain_gap_queue() on the 5s interval.
    ///
    /// Skips if the gap queue is still draining from a previous cycle —
    /// drain fully first, then rediscover to find any remaining items.
    async fn run_replication_cycle(&self) {
        // Don't re-query while the queue is still draining — unnecessary
        // network traffic and discover() would skip pending items anyway.
        if !self.gap_queue.lock().await.is_empty() {
            debug!("Replication cycle: gap queue non-empty, waiting for drain");
            return;
        }

        let swarm = self.swarm.read().await;
        let peers: Vec<PeerId> = swarm.connected_peers().cloned().collect();
        drop(swarm);

        if peers.is_empty() {
            debug!("Replication cycle: no connected peers, skipping");
            return;
        }

        // Query ALL connected peers for content inventory so we discover
        // content regardless of which peer holds it. The response handler
        // (ContentList branch) deduplicates via replication_state.discover().
        for peer in &peers {
            let request = ShardRequest::ListContent {
                reach_filter: None,
                offset: 0,
                limit: 5000,
            };

            let mut swarm = self.swarm.write().await;
            let request_id = swarm
                .behaviour_mut()
                .shard_protocol
                .send_request(peer, request);
            drop(swarm);

            debug!(peer = %peer, request_id = ?request_id, "Sent ListContent for replication discovery");
        }
        // Responses handled in handle_shard_response → ContentList branch
    }

    /// Dispatch replication fetches from the gap queue, bounded by in-flight count.
    ///
    /// Called every 5 seconds from the event loop. Checks how many GetContent
    /// requests are currently in flight and dispatches up to
    /// `MAX_REPLICATION_INFLIGHT - in_flight` more from the gap queue.
    ///
    /// This makes the dispatch rate a natural function of peer response speed:
    /// fast peer → completions arrive quickly → slots free up → more dispatched.
    /// slow peer → slots stay occupied → fewer dispatched per tick.
    /// No sleeps, no blocking — the event loop stays responsive throughout.
    async fn drain_gap_queue(&self) {
        const MAX_REPLICATION_INFLIGHT: usize = 50;

        let peers: Vec<PeerId> = {
            let swarm = self.swarm.read().await;
            swarm.connected_peers().cloned().collect()
        };
        if peers.is_empty() {
            return; // No peer connected — items stay in queue
        }

        let in_flight = self.pending_replication_fetches.lock().await.len();
        let available = MAX_REPLICATION_INFLIGHT.saturating_sub(in_flight);
        if available == 0 {
            return; // At capacity — wait for completions to free slots
        }

        let to_dispatch: Vec<String> = {
            let mut queue = self.gap_queue.lock().await;
            if queue.is_empty() {
                return;
            }
            let len = queue.len();
            queue.drain(..available.min(len)).collect()
        };

        debug!(
            dispatching = to_dispatch.len(),
            in_flight, "Draining replication gap queue"
        );

        // Round-robin fetches across all connected peers so content
        // can be retrieved from whichever peer actually has it.
        for (i, id) in to_dispatch.iter().enumerate() {
            let peer = peers[i % peers.len()];
            let request = ShardRequest::GetContent { id: id.clone() };
            let mut swarm = self.swarm.write().await;
            let request_id = swarm
                .behaviour_mut()
                .shard_protocol
                .send_request(&peer, request);
            drop(swarm);
            self.pending_replication_fetches
                .lock()
                .await
                .insert(request_id, id.clone());
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
        let replication = self.replication_state.status().await;

        // Drain state: operational counts from the content projection. This
        // is a separate DB query per refresh, but status refreshes are only
        // every 15-30 seconds so cost is negligible. On failure we return
        // None so consumers can distinguish "data unavailable" from a real
        // "caught up" reading (pending == 0).
        //
        // TODO(multi-app): scope is hardcoded to lamad. When elohim-storage
        // hosts a second app (e.g. mishpat content), either (a) aggregate
        // across all app contexts, or (b) add an `?appId=` override on
        // /p2p/status, or (c) return a per-app map. The current behaviour
        // is correct for the lamad-only deployments we have today.
        let drain = if let Some(ref pool) = self.db_pool {
            match pool.get() {
                Ok(mut conn) => {
                    let app_ctx = crate::db::AppContext::default_lamad();
                    match crate::db::content_diesel::count_publish_state(&mut conn, &app_ctx) {
                        Ok((total_i64, published_i64)) => {
                            let total: i32 = total_i64.try_into().unwrap_or(i32::MAX);
                            let published: i32 = published_i64.try_into().unwrap_or(i32::MAX);
                            Some(DrainStatusInfo {
                                total,
                                published,
                                pending: total.saturating_sub(published),
                            })
                        }
                        Err(e) => {
                            debug!(error = %e, "refresh_status: count_publish_state failed");
                            None
                        }
                    }
                }
                Err(e) => {
                    debug!(error = %e, "refresh_status: db pool get failed");
                    None
                }
            }
        } else {
            None
        };

        let (dedup_unique_len, dedup_total_seen) = self.dedup.stats();
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
            replication,
            drain,
            sync_paused: self.sync_paused.load(Ordering::Acquire),
            dedup_unique_len,
            dedup_total_seen,
        };
        let _ = self.status_tx.send(status);
    }
}

/// Phase 2c reach gate: allow Commons/Public to any caller; allow the author
/// to fetch their own Private/Community/Familiar/Trusted/Intimate/Self atoms.
/// Unknown reach values deny by default.
///
/// Phase 2b will extend this with relationship + stewardship lookup so that
/// non-author callers with a qualifying relationship can receive the atom.
pub fn reach_gate_allows(
    atom_reach: &str,
    caller: &crate::p2p::CallerIdentity,
    atom_author: Option<&str>,
) -> bool {
    match atom_reach {
        "commons" | "public" => true,
        "community" | "familiar" | "trusted" | "intimate" | "self" | "private" => {
            match (caller, atom_author) {
                (crate::p2p::CallerIdentity::Agent(c), Some(a)) => c == a,
                _ => false,
            }
        }
        _ => false,
    }
}

#[cfg(test)]
mod reach_gate_tests {
    use super::reach_gate_allows;
    use crate::p2p::CallerIdentity;

    #[test]
    fn commons_served_to_anonymous() {
        assert!(reach_gate_allows(
            "commons",
            &CallerIdentity::Anonymous,
            None
        ));
        assert!(reach_gate_allows(
            "public",
            &CallerIdentity::Anonymous,
            Some("anyone")
        ));
    }

    #[test]
    fn private_served_only_to_author() {
        let author = "bafyAUTHOR";
        assert!(reach_gate_allows(
            "private",
            &CallerIdentity::Agent(author.to_string()),
            Some(author),
        ));
        assert!(!reach_gate_allows(
            "private",
            &CallerIdentity::Agent("bafyOTHER".to_string()),
            Some(author),
        ));
        assert!(!reach_gate_allows(
            "private",
            &CallerIdentity::Anonymous,
            Some(author),
        ));
    }

    #[test]
    fn unknown_reach_denies() {
        assert!(!reach_gate_allows(
            "mystery",
            &CallerIdentity::Anonymous,
            None
        ));
    }
}
