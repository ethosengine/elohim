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

pub mod acquisition;
pub mod acquisition_dispatch; // pull-leg dispatch planning across libp2p + iroh (row 13)
pub mod adapters;
pub mod attention_tending;
pub mod behaviour;
pub mod binding_cross_signature;
pub mod binding_mint;
pub mod binding_proof_wire;
pub mod blob_fetch;
pub mod blob_protocol;
pub mod blob_swarm;
pub mod conductor_agent_info_gossip;
pub mod custody_announce;
pub mod dedup;
pub mod epr_atom_protocol;
pub mod epr_protocol;
pub mod fanout;
pub mod feedback_signal;
pub mod gossip_dispatch;
pub mod head_record_client; // adopt-before-author — fetch a peer's head Record over view-federation
pub mod identity_binding_gossip;
pub mod identity_handshake;
pub mod identity_map;
pub mod inventory_broadcaster;
pub mod inventory_gossip;
pub mod kad_store;
pub mod observation_gossip;
pub mod participations_reconcile; // cross-peer membership arm of the projection reconcile
pub mod projection_ack_handler; // Phase 4 T4 — ack-projection side-projection writer
pub mod projection_reconcile; // P1 reconciliation stream — REA commitments converge from own conductor
pub mod reach_authorization;
pub mod reconcile_rails;
pub mod recovery_invitation;
pub mod recovery_revocation;
pub mod recovery_rotation;
pub mod replication;
pub mod replication_schedule;
pub mod revocation_attestation_message;
pub mod salvage_gossip;
pub mod shamir_transport;
pub mod shard_protocol;
pub mod sync_gate;
pub mod sync_protocol;
pub mod sync_round;
pub mod topics;
pub mod transport_manifest_gossip;
pub mod trust_cache;
pub mod trust_protocol;
pub mod view_federation;

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
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, oneshot, RwLock};
use tracing::{debug, info, warn};
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

/// Page cursors for the client side of `ListDocuments` sync rounds: request ID →
/// the `offset` that page was requested at. When a `DocumentList` response
/// reports `has_more`, the next page is requested from the same peer at
/// `offset + page_len` (`sync_protocol::next_doc_list_offset`), so a DocStore
/// past one page (`SYNC_LIST_PAGE_LIMIT` docs) is never silently unsyncable —
/// without this, every doc past the first page sat outside every sync round
/// forever (the corpus back-fill projects the WHOLE content table, so a modest
/// corpus crosses one page easily). Entries are removed on response AND on
/// `OutboundFailure`, bounding the map by in-flight requests.
type DocListCursorMap =
    Arc<tokio::sync::Mutex<std::collections::HashMap<request_response::OutboundRequestId, u32>>>;

/// Per-peer bounded `SyncChanges` schedulers, one round each.
///
/// The `DocumentList` arm used to send EVERY divergent document's `SyncChanges`
/// request in one pass; they all ride one connection, so yamux refused the
/// overflow (`Io(max sub-streams reached)`) and nothing retried — measured
/// 2026-08-24 as 196 -> 55 -> 0 failures across three 60s rounds while a wiped
/// DocStore took 181s to refill (iroh, one request in flight, took 63s). The
/// window is per-peer because the sub-stream budget is per-connection, and
/// per-round because `initiate_sync_round` drops the map: whatever a round did
/// not reach is simply re-enumerated by the next one.
type SyncFetchWindowMap =
    Arc<tokio::sync::Mutex<std::collections::HashMap<PeerId, sync_round::FetchWindow>>>;

/// In-flight windowed `SyncChanges` requests: request ID → (peer, doc id, namespace).
///
/// The response and failure arms are keyed by request ID, not document, so the
/// window can only free the right slot through this map. The namespace rides
/// along because the failure arm carries no response body to read it from, and
/// re-issuing the next queued document needs it. Entries are removed on
/// response AND on `OutboundFailure`, and the map is dropped at round start, so
/// it stays bounded by genuinely in-flight requests.
type SyncFetchInflightMap = Arc<
    tokio::sync::Mutex<
        std::collections::HashMap<request_response::OutboundRequestId, (PeerId, String, String)>,
    >,
>;

/// Page size for `ListDocuments` sync-round enumeration — shared by the round
/// opener (`initiate_sync_round`, page 0) and the `DocumentList` follow-up
/// (next pages), so the cursor arithmetic and the request size can't drift.
/// Defined in `sync_round` so the round planner and the wire agree by
/// construction (see that module's standing-red note).
const SYNC_LIST_PAGE_LIMIT: u32 = sync_round::SYNC_LIST_PAGE_LIMIT;

/// Client-side page cursors for in-flight `ListContent` replication chains:
/// request ID → (peer, offset that page was requested at). The `ListContent`
/// analogue of [`DocListCursorMap`]. A `ContentList` response with `has_more`
/// requests the next page at `offset + items.len()` from the SAME peer
/// (`sync_protocol::next_doc_list_offset`), so content past the first page is
/// never silently unsyncable — the pre-fix cycle hard-limited 5000 rows and
/// never honored `has_more`, so a corpus over the page size left its tail
/// permanently undiscovered by this arm. Entries are removed on the `ContentList`
/// response AND on `OutboundFailure`, bounding the map by in-flight chains; the
/// removed `(peer, _)` is what tells the [`replication_schedule::ReplicationScheduler`]
/// which peer's chain just completed or failed.
/// In-flight ListContent pages: `request_id -> (peer, offset, sent_at)`.
///
/// `sent_at` was added 2026-08-20 to time the REQUESTER half of the inventory
/// round trip. This map already spans exactly the interval we need — inserted at
/// send, removed on the matching `ContentList` response — so the instant rides
/// the cursor rather than needing a second parallel map that could drift out of
/// sync with it.
///
/// Paired with `ConvergenceAtom::InventoryServe` (the responder's local read
/// cost), this yields the composition residual for the hop:
/// `round_trip - remote_serve = wire + queue`. Neither number alone can
/// distinguish "the peer is slow to answer" from "the link is slow" — which is
/// the exact ambiguity that made a 1387% read-pool saturation look like a
/// network problem for a day.
type ListContentCursorMap = Arc<
    tokio::sync::Mutex<
        std::collections::HashMap<
            request_response::OutboundRequestId,
            (PeerId, u32, std::time::Instant),
        >,
    >,
>;

/// Page size for `ListContent` replication-discovery enumeration. Dropped from
/// the pre-fix 5000 so a single page fits a slow WAN link inside libp2p's fixed
/// 30s request timeout — an oversized page was the trigger for the timeout storm
/// this scheduling arm exists to prevent. Paired with the `has_more` cursor
/// (`ListContentCursorMap`), a larger corpus is enumerated across pages rather
/// than in one over-large, timeout-prone response.
const REPLICATION_LIST_PAGE_LIMIT: u32 = 1000;

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

/// T21: Map of in-flight blob fetch requests on the `/elohim/blob/1.0.0` channel.
///
/// Resolved when the blob-protocol `Message::Response` or `OutboundFailure`
/// event matches the `request_id`; the oneshot sender delivers the result back
/// to the caller (typically the race-fetch helper in `p2p::blob_fetch`).
///
/// Distinct from `PendingShardMap` because the value type is
/// `Result<Vec<u8>, String>` rather than `Option<Vec<u8>>` — the explicit-peer
/// fetch path surfaces precise failure reasons (NotFound, codec error, transport
/// failure) for parallel-batch first-responder logic.
type PendingBlobFetchMap = Arc<
    tokio::sync::Mutex<
        std::collections::HashMap<
            request_response::OutboundRequestId,
            oneshot::Sender<Result<crate::p2p::blob_fetch::BlobFetchReply, String>>,
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

/// Round-robin peer index for an acquisition dispatch, offset by a per-drain
/// `rotation` so successive RETRIES of the same item walk different peers.
///
/// `position` is the item's index within the drained batch; because the acquisition
/// queue is rebuilt each 60s reconcile from `list_active_pins` in stable DB order,
/// that position is stable across retries. Without the rotation term an item is
/// therefore asked of exactly one peer for its whole retry budget — provider
/// resolution that never resolves past the first candidate.
///
/// Returns `0` for an empty peer set (callers peer-gate before dispatch; the guard
/// keeps this total so it can be unit-tested without a swarm).
fn rotated_peer_index(rotation: usize, position: usize, peer_count: usize) -> usize {
    if peer_count == 0 {
        return 0;
    }
    rotation.wrapping_add(position) % peer_count
}

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

/// F-T19: Map of pending view-federation requests on `/elohim/view-federation/1.0.0`.
/// Resolved when the behaviour event delivers `Message::Response` or `OutboundFailure`.
/// Mirrors `PendingBlobFetchMap` with typed result error.
type PendingViewFederationMap = Arc<
    tokio::sync::Mutex<
        std::collections::HashMap<
            request_response::OutboundRequestId,
            oneshot::Sender<Result<crate::views::ViewFederationResponse, FederationError>>,
        >,
    >,
>;

/// T20: Map of pending Shamir share requests on `/elohim/shamir-share/1.0.0`.
/// Resolved when the behaviour event delivers `Message::Response` or `OutboundFailure`.
/// `Err(String)` carries the failure reason so the `ShareAssembler` can log + skip.
type PendingShamirShareMap = Arc<
    tokio::sync::Mutex<
        std::collections::HashMap<
            request_response::OutboundRequestId,
            oneshot::Sender<Result<crate::p2p::shamir_transport::ShamirShareResponse, String>>,
        >,
    >,
>;

/// F-T19: Error type for the requester-side view-federation dispatch path.
///
/// Returned by `P2PHandle::view_federate` when the peer does not respond in time,
/// the swarm channel is closed, or a transport-level failure occurs.
///
/// `HubId` can ride alongside this enum in a future sprint without a wire break
/// (it lives entirely in the caller layer, not on the wire).
#[derive(Debug, thiserror::Error)]
pub enum FederationError {
    /// The peer did not respond before the deadline expired — either the
    /// caller-supplied timeout in `P2PHandle::view_federate` or libp2p's
    /// internal request_timeout configured on the behaviour.
    #[error("federation timeout")]
    Timeout,
    /// The P2P command channel was closed — swarm task is gone.
    #[error("swarm channel closed")]
    SwarmGone,
    /// Outbound transport failure — the local peer could not reach the remote,
    /// the connection closed mid-flight, the remote did not support
    /// `/elohim/view-federation/1.0.0`, or an I/O error occurred. Distinct from
    /// `Timeout` (peer reachable but slow) and from a future application-level
    /// rejection variant.
    #[error("transport error")]
    TransportError,
}

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
pub(crate) fn micros_to_iso(micros: i64) -> Option<String> {
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
    /// Capability strings exactly as the peer self-declared them on its
    /// libp2p Identify `agent_version` (`caps=` token — see
    /// `parse_capabilities_from_agent_version`), e.g. `serves_compressed`,
    /// `serves_extracted`, `cache_tier:extraction`. Kept verbatim for
    /// consumers that still string-match; the typed projections below are
    /// what a client should read.
    pub capabilities: Vec<String>,
    /// Typed projection of `capabilities`: this peer can hand a client ONE
    /// file out of an app bundle (the storage-extraction delivery layer).
    /// `false` until the peer's Identify handshake has declared it — a peer
    /// that never said either reads as "cannot", which is the safe default
    /// for a client choosing where to fetch from (it falls back to the whole
    /// blob, which every peer serves).
    #[serde(default)]
    pub serves_extracted: bool,
    /// Typed projection of `capabilities`: this peer serves the raw blob.
    /// Every elohim-storage peer can, so this is `true` from discovery.
    #[serde(default)]
    pub serves_compressed: bool,
    /// Typed projection of the `cache_tier:<tier>` capability — the highest
    /// delivery layer this peer keeps warm: `projection`, `extraction` or
    /// `blob-only`. `blob-only` until the peer declares otherwise.
    #[serde(default = "DeliveryPeer::default_cache_tier")]
    pub cache_tier: String,
    /// When this peer was last seen (unix ms)
    pub last_seen: u64,
    /// HTTP port for direct file serving (default 8090)
    pub http_port: u16,
    /// Owning household hub id for this peer, when an active identity binding
    /// resolves it (`peer_id → agent_cid → household → collective`). `None` when
    /// the peer has no active binding or the bound agent has no household.
    ///
    /// Discovery (mDNS/identify) cannot populate this — it requires a DB read —
    /// so the gossip-construction path leaves it `None` and the
    /// `/api/v1/peers/delivery` HTTP handler enriches it at request time
    /// (soft-fail: any error degrades to `None`, never a 500).
    #[serde(default)]
    pub household_id: Option<String>,
    /// Distinct active provide-commitment reach tags this peer PROVIDES.
    ///
    /// Derived (request-time, in the HTTP handler) from `rea_commitments` rows
    /// where `provider = peer_id` and the commitment is active. The string set
    /// is the commitment's *reach scope* so the a2o resilience precondition can
    /// test `commitments.includes("commons")` (see
    /// `genesis/a2o/steps/resilience.steps.ts`). Decision: custody-blob
    /// commitments carry no explicit reach column — the seeder
    /// (`buildCustodyCommitmentBody`) stores only `action`,
    /// `resource_classified_as = sha256-…`, and a note — so a custody-of-a-blob
    /// commitment maps to the `"commons"` reach (the resilience scenarios ingest
    /// commons-reach content and the custody mesh hosts it). When a commitment
    /// DOES carry an explicit reach (via `in_scope_of`, e.g. a `reach:<class>`
    /// scope), that class is used verbatim instead. Empty when the peer provides
    /// no active commitments. See the handler for the full derivation.
    #[serde(default)]
    pub commitments: Vec<String>,
}

impl DeliveryPeer {
    /// The tier a peer is assumed to hold before it declares one.
    pub(crate) fn default_cache_tier() -> String {
        "blob-only".to_string()
    }

    /// The capability set a freshly discovered peer is credited with before
    /// its Identify handshake lands: raw-blob service only. Every
    /// elohim-storage node serves raw blobs (`epr_service::handle_query_delivery`
    /// answers `serves_compressed: true` unconditionally), so this is a floor,
    /// never a guess about extraction.
    pub(crate) fn discovery_floor_capabilities() -> Vec<String> {
        vec!["serves_compressed".to_string()]
    }

    /// Replace this row's capability facts with a peer's self-declared set
    /// (typed fields are derived from the strings, so the two can never
    /// disagree). The `warm:<hash>` vocabulary is accepted and kept verbatim
    /// in `capabilities`; nothing on the Identify path carries it today.
    pub(crate) fn apply_capabilities(&mut self, caps: &[String]) {
        self.capabilities = caps.to_vec();
        self.serves_extracted = caps.iter().any(|c| c == "serves_extracted");
        self.serves_compressed = caps.iter().any(|c| c == "serves_compressed");
        self.cache_tier = caps
            .iter()
            .find_map(|c| c.strip_prefix("cache_tier:"))
            .filter(|tier| !tier.is_empty())
            .map(str::to_string)
            .unwrap_or_else(Self::default_cache_tier);
    }
}
use crate::sync::{DocStore, StreamTracker, SyncManager};
use elohim_cache_core::extraction::ExtractionCache;

pub use behaviour::{ElohimStorageBehaviour, RelayMode};
pub use blob_protocol::{
    BlobCodec, BlobFetchRequest, BlobFetchResponse, BlobProtocol, BLOB_PROTOCOL_ID,
    DEFAULT_MAX_RESPONSE_SIZE as BLOB_DEFAULT_MAX_RESPONSE_SIZE,
    HARD_MAX_RESPONSE_SIZE as BLOB_HARD_MAX_RESPONSE_SIZE,
};
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
pub use view_federation::{
    ViewFederationCodec, ViewFederationProtocol, MAX_PAYLOAD as VIEW_FEDERATION_MAX_PAYLOAD,
    VIEW_FEDERATION_PROTOCOL_ID,
};

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
    /// T21: maximum response size for the `/elohim/blob/1.0.0` request-response
    /// protocol. Default 16 MiB (matches `MAX_INLINE_SIZE` in `blob_store.rs`);
    /// hard-capped at 64 MiB by the codec regardless of this value.
    pub max_blob_response_size: usize,
    /// T22: device archetype this peer reports as (e.g. `"node"`, `"desktop"`,
    /// `"mobile"`, `"steward"`). Drives the default cadence for the inventory
    /// broadcaster timer; the canonical source is `Config::device_archetype`
    /// (see `config.rs::inventory_broadcast_seconds_default`).
    pub device_archetype: Option<String>,
    /// T22: explicit inventory-broadcast cadence override in seconds. `Some(0)`
    /// disables broadcasting entirely; `None` falls back to the archetype
    /// default (mobile → disabled, node/steward → 60s, desktop → 300s).
    pub inventory_broadcast_seconds: Option<u64>,
    /// T23: CID of this peer's steward, threaded from `Config::self_cid`.
    /// `None` disables the custody reconcile sweep — without a self CID the
    /// reconcile pass cannot tell whether a custody-blob commitment is "own"
    /// (kick on miss) or "other" (emit placement-gap), and any guess would
    /// emit spurious events.
    pub self_cid: Option<String>,
    /// Identity-coherence: this node's boot-time conductor cell-key `agent_cid`
    /// (`uhCAk…`) SNAPSHOT, when the conductor was connected at P2P construction.
    /// It is the stable FALLBACK candidate for the custody reconcile pass's
    /// self-matching; each pass ALSO re-resolves the active-session arm from its
    /// DB connection (`reconcile::custody::resolve_self_agent_cid`) so the read
    /// side stays in lockstep with the salvage author's per-tick write identity.
    /// A `custody-blob` row whose `provider`/`receiver` is this node in EITHER
    /// namespace (transport `self_cid` OR the resolved `agent_cid`) counts as
    /// "own" — salvage authors `provider = agent_cid`, so without this the node's
    /// own salvage rows would never trigger their fetch. `None` (conductor not
    /// connected at boot AND no session) degrades to transport-only matching (the
    /// pre-existing behavior, no regression).
    pub self_cellkey_agent_cid: Option<String>,
    /// T23: periodic custody reconcile cadence in seconds. `Some(0)` disables
    /// the timer arm (event-driven sweeps via `ConnectionEstablished` still
    /// fire when the trigger is enabled). `None` falls back to the
    /// `Config::custody_sweep_seconds` default (120s).
    pub custody_sweep_seconds: Option<u64>,
    /// T23: how long a custody commitment can be unhonored before
    /// placement-gap fires. Threaded from `Config::placement_grace_seconds`.
    pub placement_grace_seconds: u64,
    /// T23: minimum time between repeated placement-gap events for the same
    /// commitment. Threaded from `Config::placement_gap_cooldown_seconds`.
    pub placement_gap_cooldown_seconds: u64,
    /// T23: TTL for `peer_blob_inventory` entries before they're considered
    /// stale during reconcile. Threaded from `Config::inventory_freshness_seconds`.
    pub inventory_freshness_seconds: u64,
    /// T23: per-peer timeout (seconds) for the kicked race-fetch.
    /// Threaded from `Config::fetch_blob_timeout_seconds`.
    pub fetch_blob_timeout_seconds: u64,
    /// T23: parallelism bound for the kicked race-fetch.
    /// Threaded from `Config::fetch_blob_parallelism`.
    pub fetch_blob_parallelism: usize,
    /// P3 salvage: opt-in consent gate. Default false — a node is never
    /// conscripted (imago-dei floor). When true AND the archetype is always-on
    /// (`node`/`steward`), this peer advertises spare capacity and the salvage
    /// pass may author custody-blob commitments naming self.
    /// Threaded from `Config::salvage_capacity_enabled`.
    pub salvage_capacity_enabled: bool,
    /// P3 salvage: desired distinct-holder count per blob (default 2 = the
    /// `min_replicas_for_eviction` value). Threaded from
    /// `Config::salvage_target_replicas`.
    pub salvage_target_replicas: usize,
    /// P3 salvage: cadence (seconds) for the capacity-ad broadcast and the
    /// salvage recheck sweep. Threaded from `Config::salvage_recheck_seconds`.
    pub salvage_recheck_seconds: u64,
    /// Sync round cadence in seconds, threaded from `Config::sync_interval_secs`.
    /// `None` (and `Some(0)`) fall back to `sync_round::DEFAULT_ROUND_INTERVAL`.
    ///
    /// Before this field the round tick was a hardcoded `Duration::from_secs(60)`
    /// in `run()` while `Config::sync_interval_secs` existed, was documented, and
    /// was read by nothing — the one cadence knob for the plane was inert.
    pub sync_interval_secs: Option<u64>,
    /// Bounded `SyncChanges` fetch window per peer per round, threaded from
    /// `Config::sync_fetch_window`. `None` (and `Some(0)`) fall back to
    /// `sync_round::DEFAULT_FETCH_WINDOW` — a literal zero would issue nothing,
    /// forever, with no error (same reasoning as `sync_interval_secs`).
    pub sync_fetch_window: Option<usize>,
    /// Acquisition + provide reconcile cadence in seconds, threaded from
    /// `ACQUISITION_RECONCILE_SECS` (main.rs). `None`/`Some(0)` fall back to
    /// [`acquisition::DEFAULT_RECONCILE_SECS`] via [`acquisition::reconcile_cadence`].
    pub acquisition_reconcile_secs: Option<u64>,
    /// This node's own HTTP port (`Config::http_port`), advertised to peers
    /// via the libp2p Identify `agent_version` suffix (see
    /// `behaviour::ElohimStorageBehaviour::new`) so the receiving side can
    /// populate `DeliveryPeer::http_port` with the peer's REAL port instead
    /// of a fleet-wide 8090 guess. See
    /// `genesis/data/timeline/backlog/delivery-peer-row-local-port-and-untyped-capabilities.md`.
    pub http_port: u16,
    /// Whether THIS node serves individual extracted app files (it runs an
    /// `ExtractionCache` — `Config::extraction_cache.enabled`). Advertised to
    /// peers on the same Identify `agent_version` as `http_port` so their
    /// `DeliveryPeer` row for this node carries `serves_extracted` /
    /// `cache_tier` as declared facts, not a permanent `false` (backlog §4).
    pub serves_extracted: bool,
    /// libp2p ping cadence (seconds). A connection whose ping fails is CLOSED
    /// (see the `Ping` arm in `handle_behaviour_event`) so a peer that stops
    /// answering — paused, wedged, gone dark without a FIN — leaves the
    /// connected set within `ping_interval + ping_timeout` instead of sitting
    /// in it until the 300 s idle timeout. Defaults are libp2p's (15 s / 20 s).
    pub ping_interval_secs: u64,
    /// libp2p ping timeout (seconds); see `ping_interval_secs`.
    pub ping_timeout_secs: u64,
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
            max_blob_response_size: BLOB_DEFAULT_MAX_RESPONSE_SIZE,
            device_archetype: None,
            inventory_broadcast_seconds: None,
            self_cid: None,
            self_cellkey_agent_cid: None,
            // T23 defaults mirror `config.rs::default_*`. Production callers
            // override via `main.rs::p2p_config` from top-level Config.
            custody_sweep_seconds: None,
            placement_grace_seconds: 300,
            placement_gap_cooldown_seconds: 1800,
            inventory_freshness_seconds: 600,
            fetch_blob_timeout_seconds: 5,
            fetch_blob_parallelism: 3,
            salvage_capacity_enabled: false,
            salvage_target_replicas: 2,
            salvage_recheck_seconds: 300,
            sync_interval_secs: None,
            sync_fetch_window: None,
            acquisition_reconcile_secs: None,
            http_port: DEFAULT_HTTP_PORT,
            serves_extracted: false,
            ping_interval_secs: DEFAULT_PING_INTERVAL_SECS,
            ping_timeout_secs: DEFAULT_PING_TIMEOUT_SECS,
        }
    }
}

/// libp2p's own ping defaults, restated so the config knob has a named floor.
pub const DEFAULT_PING_INTERVAL_SECS: u64 = 15;
pub const DEFAULT_PING_TIMEOUT_SECS: u64 = 20;

/// Cadence of the LAN re-dial arm: every tick, each mDNS-discovered peer that
/// is neither connected nor being dialed is dialed again on its last-known
/// multiaddrs. mDNS itself re-queries only every ~5 min and Kademlia's
/// periodic bootstrap is on the same order, so without this arm a household
/// link dropped by a failed ping (peer paused, wedged, rebooted) stays down
/// for minutes after the peer is back.
pub(crate) const LAN_REDIAL_INTERVAL_SECS: u64 = 10;

/// Concurrent inbound view-federation slice-builds permitted per node.
///
/// Moving the inbound arm off the swarm event loop (F-A(ii)) removed the
/// ACCIDENTAL serialization that used to bound conductor load: previously the
/// loop could only build one slice at a time — which is exactly what made it
/// wedge, but also what kept `get_record_for_action` fan-out at one. Unbounded
/// spawning would trade a head-of-line block for a conductor stampede on the
/// first reconnect storm, where every peer's opening ask lands at once.
///
/// 8 is deliberately small: the only view kind that touches the conductor is
/// `ContentHeadRecord`, each ask is bounded by
/// [`view_federation::HEAD_RECORD_CONDUCTOR_TIMEOUT`] (5s), and a per-node
/// ceiling of 8 in-flight conductor reads sits well inside the read-pool
/// headroom a saturated conductor still has. Excess asks QUEUE rather than shed:
/// the requester's own deadline is the shed, and a permit frees within the
/// responder budget.
static VIEW_FEDERATION_INBOUND_LIMIT: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(8);

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
    /// Conductor bridge for view-federation kinds that must ask this peer's own
    /// conductor — currently `ContentHeadRecord` (adopt-before-author's
    /// declare-carries-Record source half). Absent ⇒ that view kind serves the
    /// head hash without the proving bytes, never an error.
    hc_registry: Option<Arc<crate::hc_client_registry::HcClientRegistry>>,
    /// Policy enforcement for content filtering on P2P path
    policy_enforcement: Option<Arc<crate::db::policy_cache::PolicyEnforcement>>,
    /// Per-connection trust context cache for ambient authorization
    peer_trust_cache: trust_cache::PeerTrustCache,
    /// Pending EPR resolve requests awaiting responses from peers
    pending_epr_resolves: PendingEprMap,
    /// Pending shard fetch requests awaiting responses from peers
    pending_shard_fetches: PendingShardMap,
    /// T21: explicit-peer blob fetch responses awaiting delivery on the
    /// `/elohim/blob/1.0.0` request-response channel.
    pending_blob_fetches: PendingBlobFetchMap,
    /// Pending shard push requests awaiting acknowledgment
    pending_shard_pushes: PendingShardPushMap,
    /// Pending shard verification (Have) requests
    pending_verifications: PendingVerificationMap,
    /// Page cursors for in-flight ListDocuments sync-round requests — see
    /// `DocListCursorMap` for the silent-tail rationale.
    doc_list_cursors: DocListCursorMap,
    /// Per-peer bounded `SyncChanges` fetch window for the current sync round —
    /// see [`SyncFetchWindowMap`] for the measured unbounded-fan-out defect.
    sync_fetch_windows: SyncFetchWindowMap,
    /// Request ID → (peer, doc id, namespace) for windowed `SyncChanges`
    /// requests — see [`SyncFetchInflightMap`].
    sync_fetch_inflight: SyncFetchInflightMap,
    /// Identity-driven replication state
    replication_state: replication::ReplicationState,
    /// Per-peer `ListContent` scheduling: in-flight guard + exponential backoff.
    /// Stops a slow/timing-out peer from re-arming a fixed-cost timeout storm on
    /// every 60s tick (see `replication_schedule`).
    replication_scheduler: replication_schedule::ReplicationScheduler,
    /// Client-side page cursors for in-flight `ListContent` discovery chains —
    /// see [`ListContentCursorMap`]. Drives `has_more` pagination and tells the
    /// scheduler which peer's chain completed/failed.
    list_content_cursors: ListContentCursorMap,
    /// Maps in-flight replication GetContent request IDs to content IDs.
    /// Used to clean up replication state when requests fail.
    pending_replication_fetches: PendingReplicationFetchMap,
    /// Ordered queue of content IDs awaiting replication dispatch.
    /// drain_gap_queue() consumes from this at a rate bounded by
    /// MAX_REPLICATION_INFLIGHT, decoupling discovery from dispatch.
    gap_queue: ReplicationGapQueue,
    /// Acquisition stream state (spec §4) — sibling of replication_state.
    acquisition: acquisition::AcquisitionState,
    /// P1 projection-reconcile stream status, surfaced on `/p2p/status`. The
    /// actual sweep runs in main.rs composition scope (it needs the lamad
    /// HcClient author seam); the node holds a clone of the shared status only
    /// so `refresh_status` can publish it. `None` when the reconcile task was
    /// not spawned (missing HcClient / pool / disabled).
    projection_reconcile: Option<projection_reconcile::ProjectionReconcileState>,
    /// Provide-loop + re-anchor-backfill observability (Workstream D). The boot
    /// path (self_cid derive + loop spawn) and the re-anchor backfill run in
    /// main.rs composition scope; the node holds a clone of the shared status
    /// only so `refresh_status` can publish it on `/p2p/status`. `None` when not
    /// wired (e.g. a P2PNode constructed without the builder).
    provide_loop: Option<crate::services::provide_loop_status::ProvideLoopState>,
    /// Provide-loop reconciler (Slice 2b): caught-up commons pins → notarized
    /// replicates-commons Commitments. Logical-key dedup survives restart; the
    /// in-memory latch is a per-process optimisation only.
    provide_reconciler: crate::services::provide_reconcile::ProvideReconciler,
    /// content ids dispatched for acquisition, keyed by request id
    pending_acquisition_fetches: PendingReplicationFetchMap,
    /// acquisition gap queue (priority-ordered at enqueue time)
    acquisition_queue: ReplicationGapQueue,
    /// Round-robin rotation offset for acquisition dispatch, bumped once per
    /// drain. Without it, `peers[position % peers.len()]` pins every RETRY of a
    /// given item to the SAME peer — see [`rotated_peer_index`].
    acquisition_rotation: Arc<std::sync::atomic::AtomicUsize>,
    /// GetContent fetches in flight on the iroh plane (the libp2p ones are the
    /// `pending_acquisition_fetches` map); both count against the dispatch budget.
    acquisition_iroh_in_flight: Arc<std::sync::atomic::AtomicUsize>,
    /// Extraction cache for delivery capability advertisement
    extraction_cache: Option<Arc<ExtractionCache>>,
    /// T7 (head-plane trust-gradient program): process-lifetime
    /// verification-memo store, threaded into the per-request
    /// [`Self::epr_service`] snapshot via [`Self::with_memo_store`] so the
    /// libp2p EPR read path shares the SAME store instance as the HTTP and
    /// iroh paths. `None` (default) preserves today's behavior exactly —
    /// nothing consults it yet (`TrustGradient::inert()` still passes
    /// `memo: None`; T9 wires consumption).
    memo_store: Option<Arc<dyn crate::trust::VerificationMemoStore>>,
    /// Content graph engine (graph-native feature). Threaded into the
    /// per-request [`Self::epr_service`] snapshot so the libp2p read path
    /// enforces the prerequisite-mastery gate (via `PREREQUISITE` edges) and
    /// enriches EPR-head display — identical to the iroh + HTTP paths.
    /// `None` (no graph) → prerequisite gate is a no-op (allow).
    #[cfg(feature = "graph-native")]
    graph_engine: Option<Arc<crate::graph::engine::GraphEngine>>,
    /// Discovered peers with delivery capabilities (populated from mDNS + identify)
    delivery_peers: Arc<DashMap<String, DeliveryPeer>>,
    /// Cached identify info per peer (populated from identify::Event::Received)
    identify_cache: Arc<DashMap<String, CachedIdentifyInfo>>,
    /// Per-peer runtime metrics (direction, last-seen, RTT)
    peer_metrics: Arc<DashMap<String, PeerMetrics>>,
    /// T24 persistent peering: bootstrap addr (canonical multiaddr string) →
    /// PeerId learned from a successful outbound dial. Lets the bootstrap
    /// retry arm redial INDIVIDUAL dropped links — the connected==0-only
    /// retry left matthew↔jessica partitioned for pod lifetimes while each
    /// held 10-11 unrelated connections (genesis #1119).
    bootstrap_peer_ids: Arc<DashMap<String, PeerId>>,
    /// Unified sync gate: when its verdict is suppressed, sync/replication
    /// cycles are skipped. Composes the EXISTING backpressure sources (account
    /// import, content bulk, drain backlog) with the operator mode and the
    /// declared network class — one reason-carrying verdict, no second pause
    /// plane. See [`sync_gate::SyncGate`].
    sync_gate: Arc<sync_gate::SyncGate>,
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
    /// F-T19: pending view-federation requests on `/elohim/view-federation/1.0.0`.
    /// Populated by `handle_command(ViewFederate)`, resolved in the behaviour
    /// event arm when `Message::Response` or `OutboundFailure` fires.
    pending_view_federations: PendingViewFederationMap,
    /// T20: pending Shamir share requests on `/elohim/shamir-share/1.0.0`.
    /// Populated by `handle_command(RequestShamirShare)`, resolved in the behaviour
    /// event arm when `Message::Response` or `OutboundFailure` fires.
    pending_shamir_shares: PendingShamirShareMap,
    /// T16: custody reconciliation counters — incremented by reconcile_pass.
    pub reconciliation_metrics: std::sync::Arc<ReconciliationMetrics>,
    /// T18: shared cache of last gossiped inventory hashes.
    /// Stage 2 broadcaster timer will write here; the parity diagnostic HTTP
    /// endpoint reads via `P2PHandle::last_gossiped_inventory()`.
    /// Both sides share this Arc — initialized in `new()`, cloned into `handle()`.
    pub last_gossiped: Arc<std::sync::RwLock<Vec<String>>>,
    /// T22: monotonic per-peer sequence allocator for outgoing
    /// `BlobInventorySnapshot` messages. Initialized at 0 in `new()`; the
    /// run-loop broadcaster tick allocates the next value before each
    /// publish via `inventory_broadcaster::build_snapshot`.
    inventory_seq: inventory_broadcaster::SequenceAllocator,
    /// P3 salvage: monotonic sequence allocator for outgoing
    /// `SalvageCapacityAd` messages. Initialized at 0 in `new()`.
    salvage_seq: inventory_broadcaster::SequenceAllocator,
    /// Plan 4: dual-publish gossip publisher.
    ///
    /// Defaults to a `LibP2PGossipPublisher` using the node's own command channel.
    /// Replaced by `DualGossipPublisher` via `with_gossip_publisher()` when the
    /// iroh stack is running — fans out to both libp2p and iroh simultaneously.
    gossip_publisher: Arc<dyn crate::services::gossip_flood::GossipPublisher>,
    /// W2A / T22: sealing public keys for `services::back_prop::record_predecessor`.
    ///
    /// Injected via `with_sealing_keys()` when the 2-of-2 sealing infra is
    /// available (same keys the HTTP fan-out layer uses). When `None`, the
    /// `record_predecessor` call in `handle_epr_atom_request` is silently skipped
    /// (best-effort — the Announce response is never affected).
    sealing_keys: Option<Arc<crate::api::epr::SealingKeyPair>>,
    /// Step-zero substrate gossip: bounded mpsc sender for inbound
    /// `ConductorAgentInfo` envelopes received on the
    /// `CONDUCTOR_AGENT_INFO_TOPIC`. `None` when the feature flag
    /// (`ENABLE_CONDUCTOR_AGENT_INFO_GOSSIP`) is off — gossip edge silently
    /// drops in that case. When `Some`, the subscriber worker (spawned in
    /// `main.rs`) drains the receiver end via
    /// `conductor_agent_info_gossip::spawn_agent_info_subscriber_worker`.
    agent_info_inbound_tx:
        Option<mpsc::Sender<crate::p2p::conductor_agent_info_gossip::ConductorAgentInfo>>,
    /// Dual mode: where verified `elohim/transport/manifest` announcements go
    /// (the iroh peer book). `None` on libp2p-only nodes — the durable row is
    /// still projected, there is just no iroh plane to dial from.
    transport_manifest_sink:
        Option<Arc<dyn crate::p2p::gossip_dispatch::TransportManifestSink + Send + Sync>>,
    /// Wave 3: bounds the number of concurrently in-flight commitment-driven
    /// blob fetches spawned from the inventory gossip receive arm. Sized at
    /// `fetch_blob_parallelism * 4` (same as `RaceFetchKicker`). Drop-on-saturation
    /// (try_acquire) prevents fetch storms on dense gossip. Shared across the
    /// snapshot + delta arms so saturation from one does not allow an unbounded
    /// second arm.
    commitment_fetch_semaphore: Arc<tokio::sync::Semaphore>,
}

/// Cached identify protocol info for a connected peer.
#[derive(Debug, Clone)]
struct CachedIdentifyInfo {
    agent_version: String,
    protocols: Vec<String>,
    listen_addrs: Vec<String>,
    /// The peer's REAL HTTP port, parsed from its self-declared
    /// `agent_version` suffix by `parse_http_port_from_agent_version`.
    /// Already defaulted to `DEFAULT_HTTP_PORT` at insertion time when the
    /// suffix is absent/unparseable — never `None` here.
    http_port: u16,
    /// The peer's self-declared delivery capabilities (`caps=` token on the
    /// same `agent_version`), parsed by `parse_capabilities_from_agent_version`.
    /// `None` for a peer that predates the token — its delivery row keeps
    /// the discovery floor rather than inventing a declaration.
    capabilities: Option<Vec<String>>,
}

/// Per-peer runtime metrics tracked from swarm events.
///
/// Visibility is `pub` (rather than module-private) so the T23 custody
/// reconcile sweep adapter (`reconcile::custody_sweep::RaceFetchKicker`) can
/// hold an `Arc<DashMap<String, PeerMetrics>>` and snapshot connected peers
/// at kick time. The fields stay module-private — external callers should
/// route through `is_connected` / `connected_peers` accessors.
pub struct PeerMetrics {
    /// Whether this peer currently has an active libp2p connection.
    /// Set to true on ConnectionEstablished; false on ConnectionClosed
    /// (entry is also removed on disconnect, so false entries are transient).
    pub(crate) is_connected: bool,
    /// Connection direction: "inbound" or "outbound"
    pub(crate) direction: &'static str,
    /// Unix epoch millis of last peer activity
    pub(crate) last_seen_ms: u64,
    /// Ring buffer of RTT samples from ping (max 8)
    pub(crate) rtt_samples: std::collections::VecDeque<Duration>,
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
    /// Acquisition pull-queue rollup — None until the first local active-pin ×
    /// presence reconcile completes. Some(total=0) means that reconcile ran and
    /// observed no active desired items. Consumers treat None as "keep waiting",
    /// NEVER as caught up (spec §4.3).
    pub pull: Option<acquisition::PullStatusInfo>,
    /// P1 projection-reconcile stream status (REA commitments converge from own
    /// conductor; peers as discovery). None when the reconcile task is not
    /// running (missing lamad HcClient / pool / disabled).
    pub projection_reconcile: Option<projection_reconcile::ProjectionReconcileStatus>,
    /// True when the unified SyncGate holds sync for ANY reason (operator
    /// pause, wifi-only on cellular, import/bulk backpressure, drain backlog).
    pub sync_paused: bool,
    /// Operator/device sync mode from the unified SyncGate: "sync" |
    /// "paused" | "wifi-only". Sticky across restart (SQLite-persisted
    /// operational state — `db::sync_mode`).
    pub sync_mode: String,
    /// Device-declared network class: "wifi" | "cellular" | "unknown".
    /// "unknown" is honest and permissive — it never suppresses under
    /// wifi-only; the device/steward layer declares it via the API.
    pub network_class: String,
    /// Reasons currently holding sync (kebab-case, e.g. "operator-paused",
    /// "drain-backlog"). Empty exactly when `sync_paused` is false.
    pub sync_reasons: Vec<String>,
    /// D.7 dedup LRU: number of unique CIDs currently in the dedup window.
    #[ts(type = "number")]
    pub dedup_unique_len: usize,
    /// D.7 dedup LRU: cumulative insert calls (new + duplicate).
    /// Ratio `(dedup_total_seen - dedup_unique_len) / dedup_total_seen` approximates duplication rate.
    #[ts(type = "number")]
    pub dedup_total_seen: usize,
    /// T23 custody reconcile: total reconcile passes run since startup.
    /// Lets CI assert "a custody reconcile pass ran" without log scraping.
    #[ts(type = "number")]
    pub reconcile_passes_total: u64,
    /// T23 custody reconcile: total fetch kicks fired across all passes.
    /// A non-zero value confirms the reconciler emitted fetch requests for gaps.
    #[ts(type = "number")]
    pub kicks_fired_total: u64,
    /// T23 custody reconcile: total placement gaps emitted across all passes.
    #[ts(type = "number")]
    pub placement_gaps_emitted_total: u64,
    /// Workstream D provide-loop + re-anchor-backfill observability. `None`
    /// before the holder is wired (e.g. a P2PNode constructed without the
    /// builder). When present, `active=false` / `selfCidSource="unset"` is the
    /// dark-resilience-card signal: the provide-loop never spawned.
    pub provide_loop: Option<crate::services::provide_loop_status::ProvideLoopStatus>,
    /// The co-resident iroh node's `NodeId` (64-char hex), when this node runs
    /// the iroh transport stack alongside libp2p. `None` on a libp2p-only node
    /// (dual-stack boot is mode-exclusive today), so the field is additive and
    /// omitted from the wire when absent — an old client reading a new payload
    /// simply doesn't see it, a new client reading an old payload gets `null`.
    /// Set at the `main.rs` dual block from `iroh_n.node_id()`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub iroh_node_id: Option<String>,
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

// The attribution-admissibility decision for `AgentPeerBinding` (habit
// `identity-cross-signed`) lives with the codec that can actually verify it —
// `binding_proof_wire`. Re-exported here because that is where callers and the
// standing red (`tests/binding_attribution_refuses_sentinel.rs`) already reach
// for it.
pub use binding_proof_wire::{
    binding_admissible_for_attribution, binding_admissible_for_attribution_proof,
};

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
    /// Tell every connected peer that this node just authored a change, so they
    /// learn about it by being told rather than by waiting up to a full round
    /// interval for their next poll (spine node `sync-scale-honesty`).
    ///
    /// Fire-and-forget: there is no reply channel because the 60s round remains
    /// the reconciliation backstop — a dropped or failed announce costs latency,
    /// never correctness.
    AnnounceLocalChange { doc_id: String, change_hash: String },
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
    /// Step-zero substrate gossip — broadcast a Holochain conductor's agent_info JSON
    /// to peer pods so their embedded conductors learn about us regardless of
    /// signal_url. Producer: `conductor_agent_info_gossip::publish_once`.
    /// See `genesis/docs/superpowers/specs/2026-05-28-conductor-agent-info-substrate-gossip-design.md`.
    PublishConductorAgentInfo(crate::p2p::conductor_agent_info_gossip::ConductorAgentInfo),
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
    /// Transport-neutral inventory work queued by either gossip receive plane.
    /// The event loop drains this bounded command channel and invokes the one
    /// commitment scorer / libp2p byte-fetch path. The iroh receive task never
    /// dials bytes itself.
    InventoryAdvertisement {
        source_peer_id: String,
        hints: Vec<crate::p2p::inventory_gossip::BlobHint>,
        hashes: Vec<String>,
    },
    /// T17: fetch a blob from a specific peer. Used by the race-fetch helper
    /// (`p2p::blob_fetch::race_fetch`) for GET-time fallback and custody-driven kicks.
    ///
    /// # Stage 2 (T21 onward)
    /// In a live swarm this issues a `/elohim/blob/1.0.0` request-response exchange
    /// to the named peer (see `p2p/blob_protocol.rs`). The reply oneshot is delivered
    /// when the response arrives, the outbound times out, or the connection fails —
    /// all three cases produce a deterministic
    /// `Result<blob_fetch::BlobFetchReply, String>` the helper can act on. The
    /// error string in the failure path encodes the `OutboundFailure` variant
    /// ("timeout", "connection_closed", "dial_failure", "unsupported_protocols",
    /// or "io_error: …") so the consumer can classify it.
    ///
    /// # Q3 (minutes-quiesce W1.2)
    /// `BlobFetchReply::Manifest` carries a durable `ShardManifest` when the
    /// peer had no direct bytes for a sharded composite hash but resolved a
    /// manifest for it instead — see `blob_protocol::BlobFetchResponse::manifest_only`.
    ///
    /// The `for_testing()` stub still returns `Err("FetchBlob not yet implemented;
    /// Stage 1 placeholder")` because no real swarm runs in unit tests.
    FetchBlob {
        peer_id: libp2p::PeerId,
        hash: String,
        reply: tokio::sync::oneshot::Sender<Result<crate::p2p::blob_fetch::BlobFetchReply, String>>,
    },
    /// T23 review fix #2: trigger a custody reconcile pass from the
    /// `ConnectionEstablished` swarm event, decoupling the reconcile work
    /// from the swarm event-loop task. Without this, the event loop awaits
    /// `run_custody_reconcile()` inline (which runs sync diesel queries),
    /// head-of-line-blocking inventory snapshots and identity handshakes
    /// from peers that just connected during a reconnect burst.
    ///
    /// Sent via `command_tx.try_send` (non-blocking) so a saturated queue
    /// drops the trigger and the timer-tick cadence picks up the slack.
    /// Fire-and-forget; no reply.
    TriggerCustodyReconcile,
    /// F-T19: issue a `/elohim/view-federation/1.0.0` request to `peer` for the
    /// given `request`, then deliver the result via `respond`.
    ///
    /// The in-flight entry is stored in `pending_view_federations` keyed by the
    /// `OutboundRequestId` returned by `send_request`. On `Message::Response`
    /// or `OutboundFailure` the event handler removes and resolves it.
    ///
    /// Callers drive the deadline via `P2PHandle::view_federate(…, timeout)`.
    ViewFederate {
        peer: libp2p::PeerId,
        request: crate::views::ViewFederationRequest,
        respond: oneshot::Sender<Result<crate::views::ViewFederationResponse, FederationError>>,
    },
    /// T20: issue a `/elohim/shamir-share/1.0.0` request to `peer` for a
    /// Shamir share belonging to `recovery_governance_action_cid`.
    ///
    /// The in-flight oneshot is stored in `pending_shamir_shares` keyed by the
    /// `OutboundRequestId` returned by `send_request`. On `Message::Response`
    /// or `OutboundFailure` the event arm removes and resolves it.
    ///
    /// Callers come via [`LibP2PShareTransport::request_share`].
    RequestShamirShare {
        peer: libp2p::PeerId,
        request: crate::p2p::shamir_transport::ShamirShareRequest,
        respond: oneshot::Sender<Result<crate::p2p::shamir_transport::ShamirShareResponse, String>>,
    },
    /// F-A(ii): deliver a view-federation response whose slice was built OFF the
    /// swarm event loop.
    ///
    /// The inbound arm hands the slice-build to a spawned task so a conductor-
    /// touching view kind (`ContentHeadRecord`) can never wedge the event loop
    /// (see that arm for the incident). `Swarm` is not `Sync`, so the spawned
    /// task cannot hold the swarm handle to answer with — it routes the finished
    /// response back through the command channel instead, and THIS arm performs
    /// the synchronous `send_response` with the `&mut Swarm` the command loop
    /// already owns.
    ///
    /// Dropping this command without sending makes libp2p emit
    /// `InboundFailure::ResponseOmission` here and an `OutboundFailure` at the
    /// requester — the same honest degradation a signing failure produces.
    SendViewFederationResponse {
        peer: libp2p::PeerId,
        channel: libp2p::request_response::ResponseChannel<crate::views::ViewFederationResponse>,
        response: Box<crate::views::ViewFederationResponse>,
    },
}

/// RAII guard that releases its sync-suppression source when dropped.
/// Created by `P2PHandle::pause_sync()` — ensures sync always resumes
/// even if the bulk write panics or returns early. Counter-backed in the
/// gate, so overlapping operations release only when the LAST one ends.
pub struct SyncPauseGuard {
    gate: Arc<sync_gate::SyncGate>,
    source: sync_gate::SyncSuppressReason,
}

impl Drop for SyncPauseGuard {
    fn drop(&mut self) {
        self.gate.end_source(self.source);
        info!(
            source = self.source.as_str(),
            "P2P sync backpressure source released"
        );
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
    /// Shared unified sync gate — bulk write handlers register suppression
    /// sources, HTTP handlers set mode/network, the event loop reads verdicts.
    sync_gate: Arc<sync_gate::SyncGate>,
    /// Last gossiped inventory snapshot hashes.
    /// Stage 1: initialized empty. Stage 2 broadcaster timer populates via
    /// `set_last_gossiped_inventory`. The diagnostic endpoint reads this to
    /// compute filesystem parity without needing live P2P connectivity.
    last_gossiped: Arc<std::sync::RwLock<Vec<String>>>,
    /// Shared acquisition state — cloned from the node's `AcquisitionState`
    /// (which is Arc-backed internally). Used by `acquisition_per_pin()` to
    /// serve GET /api/v1/pins without requiring a p2p command round-trip.
    acquisition: acquisition::AcquisitionState,
    /// Local pantry, for the self-stewardship bytes probe in
    /// [`P2PHandle::distribute_shards`]. `None` on stub/test handles (self
    /// stewardship is then simply not recorded — never fabricated).
    blob_store: Option<Arc<BlobStore>>,
    /// Lamad/content_store conductor bridge used only for notarized automatic
    /// self-custody authoring after verified self-placement. `None` on test
    /// handles and while the bridge is offline; absence skips authoring rather
    /// than creating an unanchored local promise.
    hc_lamad: Option<Arc<crate::hc_client::HcClient>>,
}

impl P2PHandle {
    /// Get the latest P2P status snapshot
    pub fn status(&self) -> P2PStatusInfo {
        self.status_rx.borrow().clone()
    }

    /// Per-pin acquisition pull progress for GET /api/v1/pins enrichment.
    /// Returns the live per-pin status vector from the shared AcquisitionState.
    /// Empty when no pins have been reconciled yet (e.g. immediately after startup).
    pub async fn acquisition_per_pin(&self) -> Vec<acquisition::PinPullStatus> {
        self.acquisition.per_pin().await
    }

    /// Per-EPR acquisition pull progress for GET /api/v1/pins/{eprId}/pull.
    /// `pin_heads` is the pin_id→head_ref map the HTTP handler reads from the
    /// local acquisition_pins table. Returns None when no active tracker
    /// belongs to `epr_id` (handler maps None → 404).
    pub async fn acquisition_per_epr(
        &self,
        epr_id: &str,
        pin_heads: &std::collections::HashMap<i32, String>,
    ) -> Option<elohim_views::acquisition::EprPullStatusView> {
        self.acquisition.per_epr(epr_id, pin_heads).await
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
    /// Returns a guard that automatically releases the source when dropped.
    /// `source` should be `ImportInProgress` or `BulkWriteInProgress` — the
    /// two counter-backed internal suppression sources of the unified gate.
    pub fn pause_sync(
        &self,
        source: sync_gate::SyncSuppressReason,
        reason: &str,
    ) -> SyncPauseGuard {
        self.sync_gate.begin_source(source);
        info!(source = source.as_str(), reason = %reason, "P2P sync paused for backpressure");
        SyncPauseGuard {
            gate: Arc::clone(&self.sync_gate),
            source,
        }
    }

    /// The unified sync gate shared with the P2P event loop. HTTP handlers use
    /// this to read/set the operator mode and declared network class and to
    /// serve live verdicts on `/p2p/status` and `/p2p/sync-mode`.
    pub fn sync_gate(&self) -> &Arc<sync_gate::SyncGate> {
        &self.sync_gate
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
                    P2PCommand::AnnounceLocalChange { .. } => {} // fire-and-forget
                    P2PCommand::PublishRecoveryInvitation(_) => {} // fire-and-forget
                    P2PCommand::PublishIdentityBinding(_) => {} // fire-and-forget
                    P2PCommand::PublishRecoveryRevocation(_) => {} // fire-and-forget
                    P2PCommand::PublishConductorAgentInfo(_) => {} // fire-and-forget
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
                    P2PCommand::InventoryAdvertisement { .. } => {} // fire-and-forget
                    P2PCommand::FetchBlob { reply, .. } => {
                        // T17 Stage-1 stub: no swarm in test.
                        let _ =
                            reply
                                .send(Err("FetchBlob not yet implemented; Stage 1 placeholder"
                                    .to_string()));
                    }
                    P2PCommand::TriggerCustodyReconcile => {} // T23: no swarm in test, no-op
                    P2PCommand::ViewFederate { respond, .. } => {
                        // F-T19 stub: no swarm in test — signal TransportError so callers
                        // get a deterministic non-panic result from for_testing() handles.
                        let _ = respond.send(Err(FederationError::TransportError));
                    }
                    P2PCommand::RequestShamirShare { respond, .. } => {
                        // T20 stub: no swarm in test — signal dial failure.
                        let _ = respond.send(Err(
                            "stub: no P2P swarm in test (RequestShamirShare)".to_string(),
                        ));
                    }
                    // F-A(ii) stub: no swarm in test. Dropping the channel is the
                    // same honest degradation the live path takes when signing
                    // fails — the requester sees an OutboundFailure.
                    P2PCommand::SendViewFederationResponse { .. } => {}
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
            pull: None,
            projection_reconcile: None,
            sync_paused: false,
            sync_mode: sync_gate::SyncMode::Sync.as_str().to_string(),
            network_class: sync_gate::NetworkClass::Unknown.as_str().to_string(),
            sync_reasons: vec![],
            dedup_unique_len: 0,
            dedup_total_seen: 0,
            reconcile_passes_total: 0,
            kicks_fired_total: 0,
            placement_gaps_emitted_total: 0,
            provide_loop: None,
            iroh_node_id: None,
        };
        let (status_tx, status_rx) = watch::channel(initial_status);
        // Keep sender alive so the receiver never sees "sender dropped"
        std::mem::forget(status_tx);
        P2PHandle {
            status_rx,
            command_tx,
            agent_pubkey: "stub-agent".to_string(),
            delivery_peers: Arc::new(DashMap::new()),
            sync_gate: Arc::new(sync_gate::SyncGate::new()),
            last_gossiped: Arc::new(std::sync::RwLock::new(Vec::new())),
            // Stub state — NOT shared with any P2PNode; acquisition_per_pin() always empty here.
            acquisition: acquisition::AcquisitionState::new(),
            blob_store: None,
            hc_lamad: None,
        }
    }

    /// Test-only: replace the stub command channel with one the caller owns, and
    /// set the advertised `agent_pubkey`.
    ///
    /// Returns the receiver so a test can assert which commands were (or were
    /// NOT) sent — e.g. that a SELF-selected shard never triggers a `PushShard`.
    /// Intended for test utilities only — not for production use.
    #[doc(hidden)]
    pub fn for_testing_capturing_commands(
        agent_pubkey: String,
    ) -> (Self, mpsc::Receiver<P2PCommand>) {
        let (command_tx, command_rx) = mpsc::channel(64);
        let mut handle = Self::for_testing();
        handle.command_tx = command_tx;
        handle.agent_pubkey = agent_pubkey;
        (handle, command_rx)
    }

    /// Test-only: attach a local pantry so the self-stewardship bytes probe in
    /// [`Self::distribute_shards`] has something to read.
    #[doc(hidden)]
    pub fn with_blob_store_for_testing(mut self, blob_store: Arc<BlobStore>) -> Self {
        self.blob_store = Some(blob_store);
        self
    }

    /// Construct a `P2PHandle` from explicit parts for fine-grained unit tests.
    ///
    /// Unlike `for_testing()`, the caller supplies the command channel so it can
    /// control what happens when a command is received (e.g. never reply to test
    /// timeout behaviour, drop the receiver to test swarm-gone behaviour).
    ///
    /// The status receiver, command sender, and agent pubkey must be consistent.
    /// Intended for test utilities only — not for production use.
    #[doc(hidden)]
    pub fn from_parts_for_testing(
        status_rx: tokio::sync::watch::Receiver<P2PStatusInfo>,
        command_tx: mpsc::Sender<P2PCommand>,
        agent_pubkey: String,
    ) -> Self {
        P2PHandle {
            status_rx,
            command_tx,
            agent_pubkey,
            delivery_peers: Arc::new(DashMap::new()),
            sync_gate: Arc::new(sync_gate::SyncGate::new()),
            last_gossiped: Arc::new(std::sync::RwLock::new(Vec::new())),
            // Stub state — NOT shared with any P2PNode; acquisition_per_pin() always empty here.
            acquisition: acquisition::AcquisitionState::new(),
            blob_store: None,
            hc_lamad: None,
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

    /// This node's agent public key (CID). Used by the projection-reconcile
    /// stream to populate the `agent_cid` field of outbound federation requests.
    pub fn agent_pubkey(&self) -> &str {
        &self.agent_pubkey
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

    /// F-T19: Issue a `/elohim/view-federation/1.0.0` request to `peer` and await the response.
    ///
    /// Sends `P2PCommand::ViewFederate` to the swarm event loop, which calls
    /// `behaviour.view_federation.send_request` and stores the oneshot in
    /// `pending_view_federations`. The event loop resolves the oneshot when
    /// `Message::Response` or `OutboundFailure` arrives.
    ///
    /// `timeout` is the caller-supplied deadline; the event loop does **not** impose
    /// its own deadline so callers can tune per-context (aggregator vs. one-shot lookup).
    ///
    /// # Errors
    /// - `FederationError::Timeout` — peer did not respond within `timeout`.
    /// - `FederationError::SwarmGone` — command channel closed (swarm task exited).
    /// - `FederationError::TransportError` — transport-level failure (connection closed, dial failure, unsupported protocol, I/O error).
    pub async fn view_federate(
        &self,
        peer: libp2p::PeerId,
        request: crate::views::ViewFederationRequest,
        timeout: Duration,
    ) -> Result<crate::views::ViewFederationResponse, FederationError> {
        let (tx, rx) = oneshot::channel();
        self.command_tx
            .send(P2PCommand::ViewFederate {
                peer,
                request,
                respond: tx,
            })
            .await
            .map_err(|_| FederationError::SwarmGone)?;
        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(Ok(response))) => Ok(response),
            Ok(Ok(Err(e))) => Err(e),
            Ok(Err(_)) => Err(FederationError::SwarmGone),
            // Caller's deadline elapsed before swarm replied. The Receiver is dropped here,
            // but the libp2p OutboundFailure event will eventually fire and clear the
            // pending_view_federations entry — the orphan window is bounded by the
            // libp2p request_timeout. Safe to drop: send-on-closed-receiver is a no-op.
            Err(_) => Err(FederationError::Timeout),
        }
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

    /// Broadcast a single custody claim on the custody-announcement gossip plane
    /// (`elohim/storage/custody`). Called on-change immediately after a
    /// `shard_locations` row this node can verify is written — self-held recording
    /// and push-ack placement in [`P2PHandle::distribute_shards`].
    ///
    /// Best-effort: routed through `P2PCommand::GossipPublish` (the libp2p
    /// gossipsub plane — the same path content-reach broadcasts take via
    /// `LibP2PGossipPublisher`). A closed/full channel is logged and never reverts
    /// the local projection — the DB is this node's operational truth, and the
    /// catch-up reconcile arm backfills peers that missed the gossip.
    ///
    /// KNOWN Dual-mode gap: this reaches only libp2p (the handle holds the command
    /// channel, not the `DualGossipPublisher`), so an iroh-only peer converges via
    /// the catch-up arm rather than this hot announce — same asymmetry class as the
    /// inventory active-fetch path documented in `p2p::gossip_dispatch`.
    ///
    /// `holder_agent_cid` and `status` are exactly the values just written to
    /// `shard_locations`; the receiver projects them as a weaker `peer-announced`
    /// row that never overwrites a locally-witnessed one.
    async fn announce_custody(
        &self,
        shard_hash: &str,
        holder_agent_cid: &str,
        h_app_id: &str,
        status: &str,
    ) {
        use crate::p2p::custody_announce::{CustodyAnnouncement, CUSTODY_ANNOUNCE_TOPIC};

        let ann = CustodyAnnouncement {
            shard_hash: shard_hash.to_string(),
            holder_agent_cid: holder_agent_cid.to_string(),
            h_app_id: h_app_id.to_string(),
            status: status.to_string(),
            announced_at_ms: chrono::Utc::now().timestamp_millis(),
            signature: vec![0x00], // Stage 1 structural non-empty
        };
        let bytes = match ann.to_bytes() {
            Ok(b) => b,
            Err(e) => {
                warn!(
                    target: "elohim_storage::custody",
                    error = %e,
                    "failed to serialize custody announcement"
                );
                return;
            }
        };
        if let Err(e) = self
            .command_tx
            .send(P2PCommand::GossipPublish {
                topic: CUSTODY_ANNOUNCE_TOPIC.to_string(),
                payload: bytes,
            })
            .await
        {
            debug!(
                target: "elohim_storage::custody",
                error = %e,
                "custody announcement publish failed (P2P command channel closed)"
            );
            return;
        }
        crate::metrics::inc_custody_announce("sent");
        debug!(
            target: "elohim_storage::custody",
            shard_hash = %shard_hash,
            holder = %holder_agent_cid,
            status = %status,
            "queued custody announcement for gossip"
        );
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

        // The location rows below are custody evidence, so write the exact
        // manifest they describe before attempting a placement. Without this
        // projection the custody-facing fold cannot resolve a placed shard
        // back to the blob named by a custody-blob commitment. Refuse the
        // placement rather than creating an unsearchable location row.
        {
            let mut conn = pool
                .get()
                .map_err(|e| format!("shard manifest DB connection: {e}"))?;
            crate::db::shard_manifests::record_generated_manifest(
                &mut conn, content_id, h_app_id, &manifest,
            )
            .map_err(|e| format!("shard manifest projection: {e}"))?;
        }

        let total_shards = shards.len();

        // Run the contract-aware diverse selector. The liveness staleness window
        // (honesty guard: a fanned-in "online" from a peer that has since gone dark
        // must not count as accepting forever) is read here at the wiring point,
        // default 900s; `0` disables it. Self rows heartbeat every 60s, so a 900s
        // window never dims a live pod.
        let staleness_secs = std::env::var("PEER_STATUS_STALENESS_SECS")
            .ok()
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(900);
        let sel = crate::services::peer_selection::PeerSelection::new(pool.clone())
            .with_staleness_secs(staleness_secs);
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
        let mut self_placement_recorded = false;
        let now = chrono::Utc::now().to_rfc3339();

        // Pre-resolve each SELECTED peer's `agent_cid` → dialable libp2p `PeerId`
        // ONCE. `peer_selection` yields `agent_cid`s (`uhCAk…`, == `agent_pub_key`);
        // `push_shard` must dial a libp2p `PeerId`. An `agent_cid` NEVER parses as
        // a `PeerId`, so without this every push errored and `shard_locations` was
        // never populated by distribution. Resolution is transport-layer-only: the
        // dial target is translated, but `shard_locations.peer_id` below stays the
        // `agent_cid` (the resilience card's join key). A peer with no known
        // transport binding is dropped from dialing (skip + metric), never a hard
        // error. See `services::transport_resolve`.
        let dial_ids: std::collections::HashMap<String, String> = {
            let mut map = std::collections::HashMap::new();
            if let Ok(mut conn) = pool.get() {
                for p in &selected {
                    if map.contains_key(&p.peer_id) {
                        continue;
                    }
                    if let Some(libp2p_id) =
                        crate::services::transport_resolve::resolve_agent_cid_to_libp2p(
                            &mut conn, &p.peer_id,
                        )
                    {
                        map.insert(p.peer_id.clone(), libp2p_id);
                    }
                }
            }
            map
        };

        // Self-stewardship seam. `peer_statuses` is structurally SELF-ONLY today
        // (author-local rows; cross-agent aggregation deferred in the
        // infrastructure zome), so `peer_selection` can select THIS node. Dialing
        // yourself is wrong — you have no transport binding to yourself, so the
        // selection landed on the "unresolvable" arm and every shard was skipped
        // (`elohim_shard_push_peer_unresolved_total` climbing on exactly the pod
        // that selected itself).
        //
        // Identity: resolved through the EXISTING self-identity seam
        // (`reconcile::custody::resolve_self_agent_cid` — active session first,
        // then this handle's boot agent key), which returns an `agent_cid`
        // (`uhCAk…`) or `None`. The comparison below is agent_cid == agent_cid:
        // `SelectedPeer::peer_id` is an agent_cid (it is `humans.agent_pub_key`).
        // NEVER compared against a libp2p/iroh transport id.
        //
        // The local pantry snapshot is taken ONCE (same idiom as the custody
        // reconcile pass) and is the verify-before-record gate: a self-held
        // location row is written only for shards whose bytes are demonstrably
        // present locally. See `services::self_stewardship`.
        let self_agent_cid: Option<String> = pool.get().ok().and_then(|mut conn| {
            crate::reconcile::custody::resolve_self_agent_cid(
                &mut conn,
                Some(self.agent_pubkey.as_str()),
            )
        });
        let self_is_selected = self_agent_cid
            .as_deref()
            .is_some_and(|me| selected.iter().any(|p| p.peer_id == me));
        let local_pantry: Option<crate::reconcile::custody::BlobStoreSnapshot> = if self_is_selected
        {
            match self.blob_store.as_ref() {
                Some(store) => {
                    match crate::reconcile::custody::BlobStoreSnapshot::from_store(store) {
                        Ok(snap) => Some(snap),
                        Err(e) => {
                            tracing::warn!(
                                content_id,
                                error = %e,
                                "self-stewardship: local pantry snapshot failed; \
                                 self-held locations not recorded this pass"
                            );
                            None
                        }
                    }
                }
                None => {
                    tracing::debug!(
                        content_id,
                        "self-stewardship: no local pantry on this handle; \
                             self-held locations not recorded"
                    );
                    None
                }
            }
        } else {
            None
        };

        let placement_plan = crate::sharding::plan_shard_placement_slots(&manifest, selected.len());
        if manifest.encoding.starts_with("rs-") {
            let data_households: std::collections::HashSet<String> = placement_plan
                .iter()
                .filter(|slot| slot.role == crate::sharding::ShardRole::Data)
                .map(|slot| {
                    let peer = &selected[slot.holder_index];
                    peer.household_id
                        .clone()
                        .unwrap_or_else(|| format!("unknown:{}", peer.peer_id))
                })
                .collect();
            let parity_shards = placement_plan
                .iter()
                .filter(|slot| slot.role == crate::sharding::ShardRole::Parity)
                .count();
            tracing::info!(
                content_id,
                data_shards_diverse_households = data_households.len(),
                parity_shards,
                "parity-aware shard placement planned"
            );
        }

        for slot in placement_plan {
            let i = slot.shard_index;
            let shard_data = &shards[i];
            let hash = &manifest.shard_hashes[i];
            let peer = &selected[slot.holder_index];

            // SELF-selected: record what we can verify, never dial ourselves.
            if self_agent_cid.as_deref() == Some(peer.peer_id.as_str()) {
                let Some(ref pantry) = local_pantry else {
                    continue;
                };
                let Ok(mut conn) = pool.get() else { continue };
                match crate::services::self_stewardship::record_self_held_shard(
                    &mut conn,
                    pantry,
                    hash,
                    &peer.peer_id,
                    h_app_id,
                ) {
                    Ok(crate::services::self_stewardship::SelfHeldOutcome::Recorded) => {
                        tracing::info!(
                            content_id,
                            shard_index = i,
                            peer = %peer.peer_id,
                            "Shard recorded as self-held (bytes verified in local pantry; no push)"
                        );
                        // Cross-pod convergence: announce this self-held custody
                        // claim so peers (and other doorways) can project it.
                        self.announce_custody(
                            hash,
                            &peer.peer_id,
                            h_app_id,
                            crate::services::self_stewardship::SELF_HELD_STATUS,
                        )
                        .await;
                        self_placement_recorded = true;
                        distributed += 1;
                    }
                    Ok(outcome) => {
                        tracing::debug!(
                            content_id,
                            shard_index = i,
                            peer = %peer.peer_id,
                            ?outcome,
                            "Self-held shard NOT recorded (gap left honestly visible)"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            content_id,
                            shard_index = i,
                            error = %e,
                            "Self-held shard recording failed"
                        );
                    }
                }
                continue;
            }

            // Resolve the dial target. An unresolvable peer (no transport binding
            // yet) is skipped with a log + metric — the shard stays unplaced this
            // pass, exactly as a push failure does today (no hard error).
            let Some(dial_id) = dial_ids.get(&peer.peer_id) else {
                crate::metrics::inc_shard_push_peer_unresolved();
                tracing::warn!(
                    content_id,
                    shard_index = i,
                    peer = %peer.peer_id,
                    "Shard push skipped: agent_cid has no known libp2p transport binding \
                     (peer_transport_manifest + peer_identity_bindings both miss)"
                );
                continue;
            };

            match self.push_shard(dial_id, hash, shard_data.clone()).await {
                Ok(()) => {
                    tracing::info!(
                        content_id,
                        shard_index = i,
                        peer = %peer.peer_id,
                        dial = %dial_id,
                        household = ?peer.household_id,
                        "Shard distributed"
                    );
                    if let Ok(mut conn) = pool.get() {
                        let location = crate::db::models::NewShardLocation {
                            shard_hash: hash,
                            // Store the AGENT key, not the transport dial id — the
                            // resilience join is agent_cid == agent_cid.
                            peer_id: &peer.peer_id,
                            h_app_id,
                            status: "announced",
                        };
                        let _ = crate::db::shard_locations::upsert_location(&mut conn, &location);
                    }
                    // Cross-pod convergence: announce this push-acked placement so
                    // peers project the holder into their own shard_locations.
                    self.announce_custody(hash, &peer.peer_id, h_app_id, "announced")
                        .await;
                    distributed += 1;
                }
                Err(e) => {
                    tracing::warn!(
                        content_id,
                        shard_index = i,
                        peer = %peer.peer_id,
                        dial = %dial_id,
                        error = %e,
                        "Shard push failed"
                    );
                }
            }
        }

        // First-commitment producer: verified self-placement is the consented
        // self-stewardship seam. Author ONE active custody-blob promise for the
        // exact generated manifest address. The method already holds the full
        // `blob_data`, so this node can truthfully pledge the blob even when the
        // placement selector assigned only a subset of its shards to self.
        //
        // Conductor-required: an offline bridge leaves the commitment absent
        // and a later distribution/backfill retries; it never creates a local
        // SQLite-only promise. Failure is non-fatal to the evidence plane—the
        // manifest + shard_locations remain truthful and the gap stays visible.
        if self_placement_recorded {
            match (self_agent_cid.as_deref(), self.hc_lamad.as_ref()) {
                (Some(provider), Some(hc)) => match pool.get() {
                    Ok(mut conn) => {
                        let ctx = crate::db::AppContext::new(h_app_id);
                        match crate::services::rea_commitment_service::ReaCommitmentService::ensure_active_self_custody(
                            &mut conn,
                            &ctx,
                            &manifest.blob_hash,
                            manifest.total_size,
                            provider,
                            Some(content_id),
                            hc,
                        )
                        .await
                        {
                            Ok(outcome) => tracing::info!(
                                content_id,
                                blob_hash = %manifest.blob_hash,
                                ?outcome,
                                "self-stewardship: active custody-blob commitment ensured"
                            ),
                            Err(e) => tracing::warn!(
                                content_id,
                                blob_hash = %manifest.blob_hash,
                                error = %e,
                                "self-stewardship: custody commitment authoring failed; evidence remains and a later pass retries"
                            ),
                        }
                    }
                    Err(e) => tracing::warn!(
                        content_id,
                        error = %e,
                        "self-stewardship: DB pool unavailable for custody commitment authoring"
                    ),
                },
                (Some(_), None) => tracing::warn!(
                    content_id,
                    blob_hash = %manifest.blob_hash,
                    "self-stewardship: lamad conductor bridge offline; skipping unanchored custody commitment"
                ),
                (None, _) => {}
            }
        }

        // Placement-gap disposition reflects BOTH peer SELECTION and PUSH reality.
        // A selection that returned `Ok` does NOT mean bytes moved: the
        // unresolvable-peer skip path (no transport binding) and push errors can
        // leave `distributed < total_shards` — even ZERO pushed — while
        // `gap_kind_opt` is `None`. Clearing gaps on selection-success alone
        // fabricates a green placement (observed on alpha: an empty `placementGaps`
        // beside zero real placements). Gaps therefore clear ONLY when selection was
        // full AND every shard's push landed; any push shortfall records/leaves the
        // gap instead.
        //
        // `gap_kind` must stay within the `placement-gap-view.schema.json` enum
        // (`under-committed` | `contracts-short` | `peers-unavailable`), which lives
        // outside this crate. A push shortfall on a full selection is surfaced as
        // `peers-unavailable` — the selected peer could not be reached to receive the
        // push (missing transport binding / push failure), the closest honest fit.
        let gap_to_record: Option<(&'static str, i32, i32)> = match gap_kind_opt {
            // Selection came up short — report the selection shortfall as before.
            Some(gap_kind) => Some((gap_kind, achieved, requested)),
            // Selection met the target, but not every shard's push landed.
            None if distributed < total_shards => {
                Some(("peers-unavailable", distributed as i32, total_shards as i32))
            }
            // Full selection AND all shards pushed — nothing to record.
            None => None,
        };

        if let Some((gap_kind, gap_achieved, gap_requested)) = gap_to_record {
            // One row per shard hash so the shefa signal reflects per-shard reality.
            if let Ok(mut conn) = pool.get() {
                let coverage = if gap_requested == 0 {
                    0.0_f32
                } else {
                    gap_achieved as f32 / gap_requested as f32
                };
                for hash in &manifest.shard_hashes {
                    let id = uuid::Uuid::new_v4().to_string();
                    let gap = crate::db::models::NewPlacementGap {
                        id: &id,
                        content_id,
                        shard_hash: hash,
                        h_app_id,
                        requested_steward_count: gap_requested,
                        achieved_steward_count: gap_achieved,
                        contract_coverage: coverage,
                        gap_kind,
                        first_seen_at: &now,
                        last_seen_at: &now,
                    };
                    let _ = crate::db::placement_gaps::upsert_gap(&mut conn, &gap);
                }
            }
        } else {
            // Full placement backed by successful pushes — clear any stale gaps.
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
/// Fleet-wide default HTTP port, used until a peer's REAL port is learned.
///
/// A libp2p multiaddr carries only the transport (TCP/QUIC) port for the P2P
/// swarm itself — it has no bearing on the peer's separate HTTP server port,
/// so it can never be parsed out of one (the function that used to pretend
/// otherwise, `extract_http_port`, always returned this constant regardless
/// of its argument). The real value travels via the Identify `agent_version`
/// suffix instead — see `parse_http_port_from_agent_version` and
/// `behaviour::ElohimStorageBehaviour::new`. This constant is only the
/// fallback for a peer whose Identify handshake hasn't completed yet, or
/// who hasn't upgraded to advertise its port.
pub(crate) const DEFAULT_HTTP_PORT: u16 = 8090;

/// Parse the self-declared HTTP port a peer appends to its libp2p Identify
/// `agent_version` string (`"<user_agent> http_port=<port>"` — see
/// `behaviour::ElohimStorageBehaviour::new`, the sole encoder).
///
/// Returns `None` when the suffix is absent (an older peer that hasn't
/// upgraded) or malformed; callers fall back to `DEFAULT_HTTP_PORT`, so a
/// mixed-version fleet degrades to today's behavior rather than erroring.
fn parse_http_port_from_agent_version(agent_version: &str) -> Option<u16> {
    agent_version
        .split_whitespace()
        .find_map(|token| token.strip_prefix("http_port="))
        .and_then(|port_str| port_str.parse::<u16>().ok())
        // Port 0 is never a servable HTTP port ("any port" bind syntax at
        // best) — treat it as absent so callers fall back to the default
        // instead of projecting an undialable row.
        .filter(|port| *port != 0)
}

/// Parse the self-declared delivery capabilities a peer appends to its libp2p
/// Identify `agent_version` (`"<user_agent> http_port=<port> caps=<a,b,c>"` —
/// see `behaviour::ElohimStorageBehaviour::new`, the sole encoder; the
/// vocabulary is `identity::DeliveryCapabilities::to_capability_strings`).
///
/// Returns `None` when the token is absent (a peer that predates it) so the
/// caller keeps the discovery floor instead of inventing a declaration. An
/// empty `caps=` is a declaration of nothing and yields `Some(vec![])`.
fn parse_capabilities_from_agent_version(agent_version: &str) -> Option<Vec<String>> {
    agent_version
        .split_whitespace()
        .find_map(|token| token.strip_prefix("caps="))
        .map(|list| {
            list.split(',')
                .map(str::trim)
                .filter(|c| !c.is_empty())
                .map(str::to_string)
                .collect()
        })
}

/// Decide a delivery-peer row's `http_port` on an mDNS (re)discovery.
///
/// `ConnectionClosed` clears `identify_cache`, so a fast mDNS rediscovery can
/// observe `cached = None` for a peer whose REAL port was already learned on
/// the previous connection and is still on the existing row. Resetting to
/// `DEFAULT_HTTP_PORT` there regressed a correct row until Identify
/// re-completed — instead, prefer the cached (freshest) self-declaration,
/// then the row's existing value, and only for a never-identified,
/// never-seen peer fall back to the default.
fn resolve_discovery_http_port(cached: Option<u16>, existing: Option<u16>) -> u16 {
    cached.or(existing).unwrap_or(DEFAULT_HTTP_PORT)
}

#[cfg(test)]
mod http_port_advertisement_tests {
    //! Regression coverage for
    //! `genesis/data/timeline/backlog/delivery-peer-row-local-port-and-untyped-capabilities.md`
    //! part 1: `httpPort` used to be a hardcoded 8090 for every peer row
    //! (`extract_http_port` ignored its argument). These tests pin the
    //! replacement mechanism: the peer's real port travels via the Identify
    //! `agent_version` suffix and projects onto the `DeliveryPeer` HTTP row
    //! as `httpPort`; an unparseable/absent suffix (an un-upgraded peer)
    //! degrades to `DEFAULT_HTTP_PORT` rather than erroring.
    use super::{
        parse_http_port_from_agent_version, resolve_discovery_http_port, DeliveryPeer,
        DEFAULT_HTTP_PORT,
    };

    #[test]
    fn parses_http_port_suffix_from_agent_version() {
        let agent_version = "elohim-storage/1.2.3+abc1234 http_port=8091";
        assert_eq!(
            parse_http_port_from_agent_version(agent_version),
            Some(8091)
        );
    }

    #[test]
    fn missing_suffix_returns_none_and_falls_back_to_default() {
        // An older peer that hasn't upgraded to advertise its port.
        let agent_version = "elohim-storage/1.2.3+abc1234";
        assert_eq!(parse_http_port_from_agent_version(agent_version), None);
        assert_eq!(
            parse_http_port_from_agent_version(agent_version).unwrap_or(DEFAULT_HTTP_PORT),
            DEFAULT_HTTP_PORT
        );
    }

    #[test]
    fn malformed_suffix_returns_none() {
        let agent_version = "elohim-storage/1.2.3 http_port=not-a-port";
        assert_eq!(parse_http_port_from_agent_version(agent_version), None);
    }

    /// Port 0 is never a servable HTTP port — a `http_port=0` suffix is
    /// treated as absent (→ default fallback), not projected onto peer rows.
    #[test]
    fn zero_port_suffix_is_rejected_as_absent() {
        let agent_version = "elohim-storage/1.2.3+abc1234 http_port=0";
        assert_eq!(parse_http_port_from_agent_version(agent_version), None);
        assert_eq!(
            parse_http_port_from_agent_version(agent_version).unwrap_or(DEFAULT_HTTP_PORT),
            DEFAULT_HTTP_PORT
        );
    }

    /// The transient regression this pins: `ConnectionClosed` clears
    /// `identify_cache`, so a fast mDNS rediscovery sees `cached = None` for
    /// a peer whose real port (e.g. jessica's :8091) is already on the
    /// existing `delivery_peers` row. The row must KEEP that port rather
    /// than resetting to `DEFAULT_HTTP_PORT` until Identify re-completes.
    #[test]
    fn rediscovery_without_identify_cache_keeps_existing_row_port() {
        assert_eq!(resolve_discovery_http_port(None, Some(8091)), 8091);
    }

    /// A fresh Identify declaration wins over whatever the row held.
    #[test]
    fn rediscovery_prefers_cached_identify_port_over_existing_row() {
        assert_eq!(resolve_discovery_http_port(Some(8092), Some(8091)), 8092);
    }

    /// A never-identified, never-seen peer starts at the default.
    #[test]
    fn first_discovery_without_identify_defaults() {
        assert_eq!(resolve_discovery_http_port(None, None), DEFAULT_HTTP_PORT);
    }

    /// The `caps=` token rides the same string as `http_port=`; neither may
    /// swallow the other (the old `rsplit_once("http_port=")` parse would
    /// have read `"8091 caps=…"` as the port and defaulted to 8090).
    #[test]
    fn http_port_and_caps_tokens_coexist_on_one_agent_version() {
        let agent_version =
            "elohim-storage/1.2.3+abc1234 http_port=8091 caps=serves_compressed,serves_extracted,cache_tier:extraction";
        assert_eq!(
            parse_http_port_from_agent_version(agent_version),
            Some(8091)
        );
        assert_eq!(
            super::parse_capabilities_from_agent_version(agent_version).as_deref(),
            Some(
                &[
                    "serves_compressed".to_string(),
                    "serves_extracted".to_string(),
                    "cache_tier:extraction".to_string()
                ][..]
            )
        );
    }

    /// A peer that predates the token declares nothing — `None`, never an
    /// invented empty set — so its row keeps the discovery floor.
    #[test]
    fn absent_caps_token_is_none_and_empty_token_is_empty() {
        assert_eq!(
            super::parse_capabilities_from_agent_version("elohim-storage/1.2.3 http_port=8090"),
            None
        );
        assert_eq!(
            super::parse_capabilities_from_agent_version("elohim-storage/1.2.3 caps="),
            Some(vec![])
        );
    }

    /// The typed row fields are derived from the declared strings, so the
    /// HTTP consumer reads `servesExtracted: true` / `cacheTier: "extraction"`
    /// for a peer that said so — and the floor for one that has not.
    #[test]
    fn apply_capabilities_projects_typed_fields() {
        let mut row = sample_peer(8090);
        row.apply_capabilities(&DeliveryPeer::discovery_floor_capabilities());
        assert!(!row.serves_extracted);
        assert!(row.serves_compressed);
        assert_eq!(row.cache_tier, "blob-only");

        row.apply_capabilities(&[
            "serves_extracted".to_string(),
            "serves_compressed".to_string(),
            "cache_tier:extraction".to_string(),
        ]);
        assert!(row.serves_extracted);
        assert!(row.serves_compressed);
        assert_eq!(row.cache_tier, "extraction");
        let json = serde_json::to_value(&row).unwrap();
        assert_eq!(json["servesExtracted"], true);
        assert_eq!(json["servesCompressed"], true);
        assert_eq!(json["cacheTier"], "extraction");
        assert!(json["capabilities"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c == "serves_extracted"));
    }

    fn sample_peer(http_port: u16) -> DeliveryPeer {
        DeliveryPeer {
            peer_id: "12D3KooWExamplePeer".to_string(),
            multiaddrs: vec!["/ip4/192.168.1.42/tcp/4001".to_string()],
            network: "lan".to_string(),
            capabilities: vec!["serves_compressed".to_string()],
            serves_extracted: false,
            serves_compressed: true,
            cache_tier: DeliveryPeer::default_cache_tier(),
            last_seen: 1_700_000_000_000,
            http_port,
            household_id: None,
            commitments: Vec::new(),
        }
    }

    /// A received peer record whose Identify agent_version carried
    /// `http_port=8091` projects `httpPort: 8091` onto the HTTP row —
    /// the specific regression named in the backlog (matthew :8090,
    /// jessica :8091, james :8092 all read back 8090 before this fix).
    #[test]
    fn peer_record_with_real_http_port_projects_onto_delivery_row() {
        let parsed =
            parse_http_port_from_agent_version("elohim-storage/1.2.3+abc1234 http_port=8091")
                .unwrap_or(DEFAULT_HTTP_PORT);
        assert_eq!(parsed, 8091);

        let peer = sample_peer(parsed);
        let json = serde_json::to_value(&peer).expect("DeliveryPeer serializes");
        assert_eq!(json["httpPort"], serde_json::json!(8091));
    }

    /// A peer record missing the http_port suffix (un-upgraded peer, or no
    /// Identify handshake yet) defaults to `DEFAULT_HTTP_PORT` (8090) —
    /// fleet behavior is unchanged until peers upgrade.
    #[test]
    fn peer_record_missing_http_port_defaults_to_8090_on_delivery_row() {
        let parsed = parse_http_port_from_agent_version("elohim-storage/1.2.3+abc1234")
            .unwrap_or(DEFAULT_HTTP_PORT);
        assert_eq!(parsed, DEFAULT_HTTP_PORT);
        assert_eq!(parsed, 8090);

        let peer = sample_peer(parsed);
        let json = serde_json::to_value(&peer).expect("DeliveryPeer serializes");
        assert_eq!(json["httpPort"], serde_json::json!(8090));
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
            // 300s (was 60s): pod↔pod links idle between ~60s inventory cadences
            // raced the 60s timeout (genesis #1119 partition class). Persistent
            // peering redials dropped links; the longer timeout reduces flap.
            .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(300)))
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
            pull: None,
            projection_reconcile: None,
            sync_paused: false,
            sync_mode: sync_gate::SyncMode::Sync.as_str().to_string(),
            network_class: sync_gate::NetworkClass::Unknown.as_str().to_string(),
            sync_reasons: vec![],
            dedup_unique_len: 0,
            dedup_total_seen: 0,
            reconcile_passes_total: 0,
            kicks_fired_total: 0,
            placement_gaps_emitted_total: 0,
            provide_loop: None,
            iroh_node_id: None,
        };
        let (status_tx, _) = tokio::sync::watch::channel(initial_status);

        // Default to always-Anonymous until a pool is attached via with_db_pool().
        // with_db_pool() replaces this with HolochainBackedPeerIdentityMap.
        let identity_map: Arc<dyn identity_map::PeerIdentityMap> =
            Arc::new(identity_map::StubIdentityMap::new());

        info!(peer_id = %peer_id, relay_mode = %config.relay_mode, "Created P2P node with NAT traversal");

        // Default gossip publisher: libp2p-only using the command channel.
        // Replaced by DualGossipPublisher via with_gossip_publisher() when the iroh stack runs.
        let default_gossip_publisher: Arc<dyn crate::services::gossip_flood::GossipPublisher> =
            Arc::new(crate::p2p::adapters::LibP2PGossipPublisher::new(
                command_tx.clone(),
            ));

        // Wave 3: size the commitment-fetch semaphore before `config` is moved into Self.
        let commitment_fetch_concurrency = config.fetch_blob_parallelism.max(1) * 4;

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
            hc_registry: None,
            policy_enforcement: None,
            peer_trust_cache: trust_cache::PeerTrustCache::new(),
            pending_epr_resolves: Arc::new(tokio::sync::Mutex::new(
                std::collections::HashMap::new(),
            )),
            pending_shard_fetches: Arc::new(tokio::sync::Mutex::new(
                std::collections::HashMap::new(),
            )),
            pending_blob_fetches: Arc::new(tokio::sync::Mutex::new(
                std::collections::HashMap::new(),
            )),
            pending_shard_pushes: Arc::new(tokio::sync::Mutex::new(
                std::collections::HashMap::new(),
            )),
            pending_verifications: Arc::new(tokio::sync::Mutex::new(
                std::collections::HashMap::new(),
            )),
            doc_list_cursors: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
            sync_fetch_windows: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
            sync_fetch_inflight: Arc::new(
                tokio::sync::Mutex::new(std::collections::HashMap::new()),
            ),
            replication_state: replication::ReplicationState::new(),
            replication_scheduler: replication_schedule::ReplicationScheduler::new(),
            list_content_cursors: Arc::new(tokio::sync::Mutex::new(
                std::collections::HashMap::new(),
            )),
            pending_replication_fetches: Arc::new(tokio::sync::Mutex::new(
                std::collections::HashMap::new(),
            )),
            gap_queue: Arc::new(tokio::sync::Mutex::new(std::collections::VecDeque::new())),
            acquisition: acquisition::AcquisitionState::new(),
            projection_reconcile: None,
            provide_loop: None,
            provide_reconciler: crate::services::provide_reconcile::ProvideReconciler::new(),
            pending_acquisition_fetches: Arc::new(tokio::sync::Mutex::new(
                std::collections::HashMap::new(),
            )),
            acquisition_queue: Arc::new(tokio::sync::Mutex::new(std::collections::VecDeque::new())),
            acquisition_rotation: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            acquisition_iroh_in_flight: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            extraction_cache: None,
            memo_store: None,
            #[cfg(feature = "graph-native")]
            graph_engine: None,
            delivery_peers: Arc::new(DashMap::new()),
            identify_cache: Arc::new(DashMap::new()),
            peer_metrics: Arc::new(DashMap::new()),
            bootstrap_peer_ids: Arc::new(DashMap::new()),
            sync_gate: Arc::new(sync_gate::SyncGate::new()),
            identity_map,
            dedup: Arc::new(dedup::DedupLru::new()),
            pending_kad_get_providers: Arc::new(tokio::sync::Mutex::new(
                std::collections::HashMap::new(),
            )),
            pending_blob_pulls: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
            pending_epr_atom_fetches: Arc::new(tokio::sync::Mutex::new(
                std::collections::HashMap::new(),
            )),
            pending_view_federations: Arc::new(tokio::sync::Mutex::new(
                std::collections::HashMap::new(),
            )),
            pending_shamir_shares: Arc::new(tokio::sync::Mutex::new(
                std::collections::HashMap::new(),
            )),
            reconciliation_metrics: std::sync::Arc::new(ReconciliationMetrics::default()),
            last_gossiped: Arc::new(std::sync::RwLock::new(Vec::new())),
            inventory_seq: inventory_broadcaster::SequenceAllocator::new(0),
            salvage_seq: inventory_broadcaster::SequenceAllocator::new(0),
            gossip_publisher: default_gossip_publisher,
            sealing_keys: None,
            agent_info_inbound_tx: None,
            transport_manifest_sink: None,
            // Wave 3: commitment-driven fetch semaphore. Sized at parallelism * 4
            // matching RaceFetchKicker (12 with default parallelism=3). Built once at
            // node construction; cloned (Arc) into each spawned task in the receive arm.
            commitment_fetch_semaphore: Arc::new(tokio::sync::Semaphore::new(
                commitment_fetch_concurrency,
            )),
        })
    }

    /// Attach the content graph engine so the libp2p EPR read path enforces the
    /// prerequisite-mastery gate (via `PREREQUISITE` edges) and enriches
    /// EPR-head display. Threaded into the per-request [`Self::epr_service`]
    /// snapshot. `None` → prerequisite gate is a no-op (allow).
    #[cfg(feature = "graph-native")]
    pub fn with_graph_engine(
        mut self,
        graph_engine: Option<Arc<crate::graph::engine::GraphEngine>>,
    ) -> Self {
        self.graph_engine = graph_engine;
        self
    }

    /// Wire the process-lifetime verification-memo store (T7). Threaded into
    /// the per-request [`Self::epr_service`] snapshot so the libp2p EPR read
    /// path shares the SAME store instance as the HTTP and iroh paths. Call
    /// once at startup with the SAME `Arc` clone threaded to `HttpServer`
    /// and the iroh `EprService`.
    pub fn with_memo_store(
        mut self,
        memo_store: Option<Arc<dyn crate::trust::VerificationMemoStore>>,
    ) -> Self {
        self.memo_store = memo_store;
        self
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

    /// Wire the conductor bridge so this node can SERVE `ContentHeadRecord`
    /// view-federation requests — the bytes a peer needs to declare a canonical
    /// head its own conductor cannot retrieve (adopt-before-author).
    pub fn with_hc_registry(
        mut self,
        registry: Arc<crate::hc_client_registry::HcClientRegistry>,
    ) -> Self {
        self.hc_registry = Some(registry);
        self
    }

    /// Attach the shared P1 projection-reconcile status so `refresh_status`
    /// surfaces it on `/p2p/status`. The same `ProjectionReconcileState` clone
    /// is driven by the reconcile task in main.rs (which holds the lamad
    /// HcClient). Surfacing-only on the node side — the node never sweeps.
    pub fn with_projection_reconcile_state(
        mut self,
        state: projection_reconcile::ProjectionReconcileState,
    ) -> Self {
        self.projection_reconcile = Some(state);
        self
    }

    /// Attach the shared provide-loop / re-anchor-backfill status (Workstream D)
    /// so `refresh_status` surfaces it on `/p2p/status`. The clone is written by
    /// the boot path + re-anchor backfill in main.rs; surfacing-only here.
    pub fn with_provide_loop_state(
        mut self,
        state: crate::services::provide_loop_status::ProvideLoopState,
    ) -> Self {
        self.provide_loop = Some(state);
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

    /// Replace the default `LibP2PGossipPublisher` with a `DualGossipPublisher`
    /// (or any other [`GossipPublisher`] impl). Consuming builder variant.
    ///
    /// Call this when the iroh node is available to enable dual-stack publish
    /// for inventory snapshots. The replacement is consumed by
    /// [`Self::broadcast_inventory_snapshot`].
    pub fn with_gossip_publisher(
        mut self,
        publisher: Arc<dyn crate::services::gossip_flood::GossipPublisher>,
    ) -> Self {
        self.gossip_publisher = publisher;
        self
    }

    /// Inject sealing keys for `back_prop::record_predecessor` on the libp2p
    /// Announce path (W2A / T22) — consuming builder variant.
    ///
    /// When supplied, every successful Content-kind EPR Atom Announce seals and
    /// stores the sender PeerId in `predecessor_records` via
    /// `services::back_prop::record_predecessor`. Errors are logged at warn level
    /// and never surfaced to the wire response (best-effort, non-fatal).
    ///
    /// When absent (`None`), the record step is silently skipped — the Announce
    /// ingest succeeds normally. FeedbackSignal-kind Announces are excluded
    /// regardless: the back-prop graph is for content provenance only.
    pub fn with_sealing_keys(mut self, keys: Arc<crate::api::epr::SealingKeyPair>) -> Self {
        self.sealing_keys = Some(keys);
        self
    }

    /// Inject sealing keys in-place (`&mut self` variant for post-construction wiring).
    ///
    /// Equivalent to `with_sealing_keys` but does not consume `self`. Used in
    /// `main.rs` where `p2p_node` is stored in an `Option<P2PNode>` and the
    /// sealing keys are constructed after the node.
    pub fn set_sealing_keys(&mut self, keys: Arc<crate::api::epr::SealingKeyPair>) {
        self.sealing_keys = Some(keys);
    }

    /// Wire the agent-info inbound mpsc sender for step-zero substrate gossip.
    ///
    /// Called from `main.rs` AFTER constructing the bounded mpsc channel,
    /// only when the `ENABLE_CONDUCTOR_AGENT_INFO_GOSSIP` env flag is on.
    /// The receiver end is handed to
    /// `conductor_agent_info_gossip::spawn_agent_info_subscriber_worker`.
    /// When unset (default), the inbound gossip arm silently drops messages.
    pub fn set_agent_info_inbound_tx(
        &mut self,
        tx: mpsc::Sender<crate::p2p::conductor_agent_info_gossip::ConductorAgentInfo>,
    ) {
        self.agent_info_inbound_tx = Some(tx);
    }

    /// Dual mode: route verified transport-manifest announcements received over
    /// gossipsub into the iroh peer book, so the iroh plane learns who to dial.
    pub fn set_transport_manifest_sink(
        &mut self,
        sink: Arc<dyn crate::p2p::gossip_dispatch::TransportManifestSink + Send + Sync>,
    ) {
        self.transport_manifest_sink = Some(sink);
    }

    /// Replace the gossip publisher in-place (non-consuming `&mut self` variant).
    ///
    /// Used when the caller needs to mutate an already-constructed node — e.g.
    /// after the iroh node is started and its gossip handle becomes available.
    pub fn set_gossip_publisher(
        &mut self,
        publisher: Arc<dyn crate::services::gossip_flood::GossipPublisher>,
    ) {
        self.gossip_publisher = publisher;
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

    /// Clone the node's cross-plane dedup cache Arc.
    ///
    /// In `TransportBackend::Dual` mode the iroh gossip receive loop is handed
    /// this same `Arc<DedupLru>` so a message arriving on both the libp2p and
    /// iroh planes is deduped once (see `gossip_dispatch`).
    pub fn dedup(&self) -> Arc<dedup::DedupLru> {
        self.dedup.clone()
    }

    /// Clone the inbound `ConductorAgentInfo` sender, if the step-zero agent-info
    /// gossip feature is wired. In Dual mode the iroh receive loop forwards
    /// agent-info envelopes into the same bounded channel.
    pub fn agent_info_inbound_tx(
        &self,
    ) -> Option<mpsc::Sender<crate::p2p::conductor_agent_info_gossip::ConductorAgentInfo>> {
        self.agent_info_inbound_tx.clone()
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

    /// T23: Run one custody reconcile pass.
    ///
    /// Builds a [`crate::reconcile::custody::BlobStoreSnapshot`] +
    /// [`crate::reconcile::custody_sweep::RaceFetchKicker`], invokes
    /// `reconcile_pass`, and folds the outcome into
    /// [`ReconciliationMetrics`]. Idempotent and safe to call from both the
    /// timer arm and the `ConnectionEstablished` event arm; cooldowns inside
    /// `reconcile_pass` suppress duplicate placement-gap events.
    ///
    /// Skips silently (with a `debug!` trace) when the prerequisites for a
    /// meaningful sweep are absent: no DB pool attached, no `self_cid`
    /// configured, or the local pantry walk fails.
    ///
    /// **Lock contract:** does NOT acquire the swarm lock. Callers from the
    /// `run()` select! arms must drop their swarm guard before invoking.
    pub(crate) async fn run_custody_reconcile(&self) {
        use crate::reconcile::custody::{reconcile_pass, BlobStoreSnapshot, ReconcileConfig};
        use crate::reconcile::custody_sweep::RaceFetchKicker;
        use std::sync::atomic::Ordering;

        let Some(self_cid) = self.config.self_cid.clone() else {
            debug!(
                target: "elohim_storage::reconcile",
                "T23: self_cid not configured; skipping custody reconcile pass"
            );
            return;
        };

        let pool = match &self.db_pool {
            Some(p) => p.clone(),
            None => {
                debug!(
                    target: "elohim_storage::reconcile",
                    "T23: db_pool not attached; skipping custody reconcile pass"
                );
                return;
            }
        };
        let mut conn = match pool.get() {
            Ok(c) => c,
            Err(e) => {
                warn!(
                    target: "elohim_storage::reconcile",
                    error = %e,
                    "T23: pool exhausted; skipping custody reconcile pass"
                );
                return;
            }
        };

        // Resolve THIS node's agent_cid for self-matching IN LOCKSTEP with the
        // salvage author: re-resolve the active-session arm from `conn` every pass,
        // falling back to the boot-time cell-key snapshot. Frozen-at-boot would
        // reintroduce the orphaned own-row class when a session logs in (or the
        // conductor connects) after P2P boot.
        let self_agent_cid: Option<String> = crate::reconcile::custody::resolve_self_agent_cid(
            &mut conn,
            self.config.self_cellkey_agent_cid.as_deref(),
        );

        let snapshot = match BlobStoreSnapshot::from_store(&self.blob_store) {
            Ok(s) => s,
            Err(e) => {
                warn!(
                    target: "elohim_storage::reconcile",
                    error = %e,
                    "T23: BlobStoreSnapshot build failed; skipping custody reconcile pass"
                );
                return;
            }
        };

        // T23 review fix #4: warn (once per pass) if a misconfigured
        // 0-value will be silently clamped to 1 inside `RaceFetchKicker`.
        // Single warn at construction; per-kick warnings would spam.
        if self.config.fetch_blob_timeout_seconds == 0 {
            warn!(
                target: "elohim_storage::reconcile",
                "T23: fetch_blob_timeout_seconds is 0; clamping to 1s"
            );
        }
        if self.config.fetch_blob_parallelism == 0 {
            warn!(
                target: "elohim_storage::reconcile",
                "T23: fetch_blob_parallelism is 0; clamping to 1"
            );
        }

        // T23 review fix #1: bound concurrent in-flight kicks at
        // parallelism * 4 (12 with default parallelism=3). Derived from
        // existing config so we don't add a new knob. Per-pass scope:
        // each call to `run_custody_reconcile` constructs a fresh
        // semaphore. With ConnectionEstablished now routed through
        // `P2PCommand::TriggerCustodyReconcile` (review fix #2),
        // reconcile passes serialize on the command lane, so per-pass
        // bounding is sufficient.
        let kick_concurrency = self.config.fetch_blob_parallelism.max(1) * 4;

        let kicker = RaceFetchKicker {
            command_tx: self.command_tx.clone(),
            connected_peers: self.peer_metrics.clone(),
            db_pool: pool.clone(),
            blob_store: self.blob_store.clone(),
            self_cid: self_cid.clone(),
            self_agent_cid: self_agent_cid.clone(),
            fetch_blob_parallelism: self.config.fetch_blob_parallelism,
            fetch_blob_timeout_seconds: self.config.fetch_blob_timeout_seconds,
            metrics: self.reconciliation_metrics.clone(),
            kick_semaphore: Arc::new(tokio::sync::Semaphore::new(kick_concurrency)),
        };

        let cfg = ReconcileConfig {
            placement_grace_seconds: self.config.placement_grace_seconds,
            placement_gap_cooldown_seconds: self.config.placement_gap_cooldown_seconds,
            inventory_freshness_seconds: self.config.inventory_freshness_seconds,
        };

        // Connected-peer snapshot for the inventory-blind fallback: when the
        // gossip inventory has no candidates for a missing provider-owned
        // blob, the pass races these instead (capped in reconcile_pass).
        let connected_peers: Vec<String> = self
            .peer_metrics
            .iter()
            .filter_map(|e| {
                if e.value().is_connected {
                    Some(e.key().clone())
                } else {
                    None
                }
            })
            .collect();

        match reconcile_pass(
            &mut conn,
            &self_cid,
            self_agent_cid.as_deref(),
            &snapshot,
            &kicker,
            cfg,
            chrono::Utc::now(),
            &connected_peers,
        ) {
            Ok(outcome) => {
                self.reconciliation_metrics
                    .reconcile_passes_total
                    .fetch_add(1, Ordering::Relaxed);
                self.reconciliation_metrics
                    .placement_gaps_emitted_total
                    .fetch_add(outcome.placement_gaps_emitted as u64, Ordering::Relaxed);
                // kicks_fired_total is incremented inside RaceFetchKicker::kick
                // so the count reflects work scheduled (one increment per kick).
                // INFO, not debug: per-pass evidence must reach Loki — the
                // 2026-06-10 Phase-0 read found zero pass lines in 953k lines
                // and could not tell a dead sweep from a quiet one.
                tracing::info!(
                    target: "elohim_storage::reconcile",
                    snapshot_size = snapshot.len(),
                    commitments_examined = outcome.commitments_examined,
                    kicks_fired = outcome.kicks_fired,
                    fallback_kicks = outcome.fallback_kicks,
                    placement_gaps = outcome.placement_gaps_emitted,
                    invalid_markers = outcome.invalid_markers,
                    connected_peers = connected_peers.len(),
                    "T23: custody reconcile pass completed"
                );
            }
            Err(e) => {
                warn!(
                    target: "elohim_storage::reconcile",
                    error = %e,
                    "T23: reconcile_pass failed"
                );
            }
        }
    }

    /// LAN re-dial arm: dial every mDNS-discovered peer that is neither
    /// connected nor already being dialed, on its last-known multiaddrs.
    /// `PeerCondition::DisconnectedAndNotDialing` makes the swarm the single
    /// judge of "already in flight", so a paused peer whose handshake is
    /// still timing out is not stacked with a second dial every tick.
    fn redial_disconnected_lan_peers(&self, swarm: &mut Swarm<ElohimStorageBehaviour>) {
        use libp2p::swarm::dial_opts::{DialOpts, PeerCondition};
        let candidates: Vec<(PeerId, Vec<Multiaddr>)> = self
            .delivery_peers
            .iter()
            .filter(|entry| entry.value().network == "lan")
            .filter_map(|entry| {
                let peer_id = entry.key().parse::<PeerId>().ok()?;
                if swarm.is_connected(&peer_id) {
                    return None;
                }
                let addrs: Vec<Multiaddr> = entry
                    .value()
                    .multiaddrs
                    .iter()
                    .filter_map(|a| a.parse::<Multiaddr>().ok())
                    .collect();
                (!addrs.is_empty()).then_some((peer_id, addrs))
            })
            .collect();
        for (peer_id, addrs) in candidates {
            let opts = DialOpts::peer_id(peer_id)
                .condition(PeerCondition::DisconnectedAndNotDialing)
                .addresses(addrs)
                .build();
            match swarm.dial(opts) {
                Ok(()) => {
                    info!(peer = %peer_id, "LAN peering: re-dialing discovered peer that dropped")
                }
                Err(libp2p::swarm::DialError::DialPeerConditionFalse(_)) => {}
                Err(e) => debug!(peer = %peer_id, error = %e, "LAN peering: re-dial failed"),
            }
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
        // Cadence comes from Config::sync_interval_secs (threaded via P2PConfig);
        // `round_interval` guards the zero that would panic tokio's ticker.
        let mut sync_interval =
            tokio::time::interval(sync_round::round_interval(self.config.sync_interval_secs));
        let mut verify_interval = tokio::time::interval(Duration::from_secs(300));
        verify_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut replication_interval = tokio::time::interval(Duration::from_secs(60));
        replication_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        // Drain the replication gap queue every 5 seconds during bootstrap.
        // Dispatches up to MAX_REPLICATION_INFLIGHT items per tick; idle
        // (no-ops) when the queue is empty.
        let mut gap_dispatch_interval = tokio::time::interval(Duration::from_secs(5));
        gap_dispatch_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        // Acquisition reconcile: diff active pins against local inventory (spec §4.2).
        // First tick fires at t=0 (parity with gap_dispatch_interval); it
        // no-ops cheaply when no pins are registered yet.
        // Cadence: `ACQUISITION_RECONCILE_SECS` via P2PConfig (default 60s).
        let reconcile_cadence =
            acquisition::reconcile_cadence(self.config.acquisition_reconcile_secs);
        let mut acquisition_reconcile_interval = tokio::time::interval(reconcile_cadence);
        acquisition_reconcile_interval
            .set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        // Provide reconcile: caught-up commons pins → notarized
        // replicates-commons Commitments (spec slice-2b §provide-loop). Same
        // cadence as acquisition; logical-key dedup survives restart.
        let mut provide_reconcile_interval = tokio::time::interval(reconcile_cadence);
        provide_reconcile_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        // Acquisition dispatch: drain the acquisition queue every 5 seconds.
        let mut acquisition_dispatch_interval = tokio::time::interval(Duration::from_secs(5));
        acquisition_dispatch_interval
            .set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
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
        // T22: inventory broadcast cadence — resolved once at run() entry from
        // archetype default + config override. None means "broadcasting
        // disabled" (mobile archetype, or explicit `0` override) — the
        // tokio::select! arm uses `std::future::pending()` so it never fires.
        //
        // TODO(T22-live-reconfig): cadence is resolved once at run() entry. The
        // 4-layer cadence pattern's "sync admin trigger" layer (per
        // project_cadence_archetype_tunable_with_dev_overrides) needs a future
        // P2PCommand::ReconfigureCadence variant that re-resolves and replaces
        // inventory_broadcast_interval here. Acceptable for the demo: archetype
        // is set at boot, no live reconfiguration is required for replicas-grow
        // or placement-gap flows.
        let mut inventory_broadcast_interval: Option<tokio::time::Interval> =
            match inventory_broadcaster::resolved_cadence(
                self.config.device_archetype.as_deref(),
                self.config.inventory_broadcast_seconds,
            ) {
                Some(secs) => {
                    info!(
                        target: "elohim_storage::inventory",
                        secs = secs,
                        archetype = ?self.config.device_archetype,
                        "T22: inventory broadcast enabled"
                    );
                    let mut iv = tokio::time::interval(Duration::from_secs(secs));
                    iv.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                    Some(iv)
                }
                None => {
                    info!(
                        target: "elohim_storage::inventory",
                        archetype = ?self.config.device_archetype,
                        "T22: inventory broadcast disabled (archetype or 0-override)"
                    );
                    None
                }
            };
        // T23: custody reconcile sweep cadence — resolved once at run() entry
        // from `Config::custody_sweep_seconds`. `Some(0)` disables the timer
        // arm (event-driven `ConnectionEstablished` sweeps still fire if
        // `self_cid` is configured); `None` resolves to the 120s default.
        // Sweeps additionally no-op silently when `self_cid` is unset, so the
        // timer is safe to enable unconditionally here.
        let mut custody_sweep_interval: Option<tokio::time::Interval> =
            match self.config.custody_sweep_seconds {
                Some(0) => {
                    info!(
                        target: "elohim_storage::reconcile",
                        "T23: custody sweep timer disabled (0-override)"
                    );
                    None
                }
                resolved => {
                    let secs = resolved.unwrap_or(120);
                    info!(
                        target: "elohim_storage::reconcile",
                        secs = secs,
                        "T23: custody sweep timer enabled"
                    );
                    let mut iv = tokio::time::interval(Duration::from_secs(secs));
                    iv.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                    Some(iv)
                }
            };
        // Track consecutive retry attempts for exponential backoff cap.
        let mut consecutive_empty_ticks: u32 = 0;
        // LAN re-dial arm (see `LAN_REDIAL_INTERVAL_SECS`). First tick is
        // deferred one period so boot-time mDNS discovery gets to dial first.
        let mut lan_redial_interval = tokio::time::interval_at(
            tokio::time::Instant::now() + Duration::from_secs(LAN_REDIAL_INTERVAL_SECS),
            Duration::from_secs(LAN_REDIAL_INTERVAL_SECS),
        );
        lan_redial_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
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
                    if self.sync_gate.suppressed() {
                        debug!("Skipping sync round (backpressure)");
                    } else {
                        self.initiate_sync_round().await;
                    }
                }
                _ = replication_interval.tick() => {
                    drop(swarm);
                    if self.sync_gate.suppressed() {
                        debug!("Skipping replication cycle (backpressure)");
                    } else {
                        self.run_replication_cycle().await;
                    }
                }
                _ = gap_dispatch_interval.tick() => {
                    drop(swarm);
                    if self.sync_gate.suppressed() {
                        debug!("Skipping gap dispatch (backpressure)");
                    } else {
                        self.drain_gap_queue().await;
                    }
                }
                _ = acquisition_reconcile_interval.tick() => {
                    drop(swarm);
                    let outcome = if self.sync_gate.suppressed() {
                        warn!(
                            target: "elohim_storage::acquisition",
                            "acquisition reconcile skipped: sync backpressure is paused"
                        );
                        crate::metrics::AcquisitionReconcileOutcome::SyncPaused
                    } else {
                        self.run_acquisition_reconcile().await
                    };
                    crate::metrics::inc_acquisition_reconcile_outcome(outcome);
                    if outcome == crate::metrics::AcquisitionReconcileOutcome::Completed {
                        // Publish the first honest pull sample immediately;
                        // otherwise the 30s status ticker leaves a just-completed
                        // t=0 reconcile looking null for another half minute.
                        self.refresh_status().await;
                    }
                }
                _ = provide_reconcile_interval.tick() => {
                    drop(swarm);
                    if !self.sync_gate.suppressed() {
                        self.run_provide_reconcile().await;
                    }
                }
                _ = acquisition_dispatch_interval.tick() => {
                    drop(swarm);
                    if !self.sync_gate.suppressed() {
                        self.drain_acquisition_queue().await;
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
                        if pending > 100 {
                            if self.sync_gate.set_drain_backlog(true) {
                                info!(pending, "Sync auto-suppressed: drain backlog > 100");
                            }
                        } else if self.sync_gate.set_drain_backlog(false) {
                            info!(pending, "Sync auto-resumed: drain backlog cleared");
                        }
                    }
                }
                _ = verify_interval.tick() => {
                    self.verify_shard_locations(&mut swarm).await;
                    drop(swarm);
                }
                _ = lan_redial_interval.tick() => {
                    self.redial_disconnected_lan_peers(&mut swarm);
                    drop(swarm);
                }
                _ = bootstrap_retry_interval.tick() => {
                    // T24 persistent peering: redial ANY configured bootstrap
                    // link that is not currently connected — not only when
                    // connected==0. The old gate left a dropped pod↔pod link
                    // dead for the pod's lifetime as long as ANY other peer
                    // (steward clients, other pods) stayed connected: genesis
                    // #1119 found matthew↔jessica partitioned in BOTH
                    // directions while each held 10-11 live connections.
                    if !self.config.bootstrap_nodes.is_empty() {
                        let connected_now: std::collections::HashSet<PeerId> =
                            swarm.connected_peers().copied().collect();
                        let needing = bootstrap_needing_redial(
                            &self.config.bootstrap_nodes,
                            &self.bootstrap_peer_ids,
                            &connected_now,
                        );
                        if needing.is_empty() {
                            if consecutive_empty_ticks > 0 {
                                info!(
                                    connected = connected_now.len(),
                                    prior_attempts = consecutive_empty_ticks,
                                    "Bootstrap links recovered"
                                );
                                consecutive_empty_ticks = 0;
                            }
                        } else {
                            consecutive_empty_ticks = consecutive_empty_ticks.saturating_add(1);
                            // Cap retry frequency: after 10 ticks (~5 min),
                            // slow to every 10th tick (~5-min cadence).
                            let should_retry = consecutive_empty_ticks <= 10
                                || consecutive_empty_ticks.is_multiple_of(10);
                            if should_retry {
                                for addr in needing {
                                    match swarm.dial(addr.clone()) {
                                        Ok(_) => info!(
                                            addr = %addr,
                                            attempt = consecutive_empty_ticks,
                                            "Persistent peering: re-dialing bootstrap link"
                                        ),
                                        Err(e) => debug!(addr = %addr, error = %e, "Persistent peering: re-dial failed"),
                                    }
                                }
                            }
                        }
                    }
                    drop(swarm);
                }
                _ = async {
                    // T22: inventory broadcast tick. When the cadence is None
                    // (mobile archetype / disabled), `pending()` ensures this
                    // arm never fires.
                    if let Some(iv) = &mut inventory_broadcast_interval {
                        iv.tick().await
                    } else {
                        std::future::pending().await
                    }
                } => {
                    drop(swarm);
                    self.broadcast_inventory_snapshot().await;
                    // P3-4: piggyback the salvage-capacity advertisement on the
                    // same broadcast cadence. Self-gates (opt-in + always-on
                    // archetype + self_cid) inside the method — a no-op for
                    // non-salvage nodes, so reusing this tick adds no new timer.
                    self.broadcast_salvage_capacity().await;
                }
                _ = async {
                    // T23: custody reconcile sweep tick. When disabled
                    // (0-override), `pending()` ensures this arm never fires.
                    if let Some(iv) = &mut custody_sweep_interval {
                        iv.tick().await
                    } else {
                        std::future::pending().await
                    }
                } => {
                    drop(swarm);
                    self.run_custody_reconcile().await;
                }
                _ = shutdown.recv() => {
                    info!("P2P node shutting down");
                    break;
                }
            }
        }
    }

    /// T22: Build and publish a `BlobInventorySnapshot` to the inventory
    /// gossip topic.
    ///
    /// **Lock contract:** This method ACQUIRES `self.swarm.write()` internally
    /// for the duration of `gossipsub.publish`. Callers MUST drop any swarm
    /// guard they hold before invoking this method. The interval-tick arm in
    /// `run()` does this via an explicit `drop(swarm)` at the start of the arm
    /// body, matching the pattern used by `initiate_sync_round`,
    /// `run_replication_cycle`, and `drain_publish_queue`.
    ///
    /// Idempotent and safe to call repeatedly. Failures (`list_hashes`,
    /// serialize, publish) log warn at the relevant target and leave the
    /// parity-diagnostic `last_gossiped` record unchanged so the diagnostic
    /// continues to report the previous successful snapshot.
    ///
    /// T22 review fix #1: `list_hashes` failure skips the tick rather than
    /// broadcasting an empty snapshot. `apply_snapshot` on remote peers
    /// evicts all of this peer's per-peer inventory entries on snapshot
    /// arrival — broadcasting empty during a transient I/O blip would
    /// corrupt every remote peer's projection of our custody.
    async fn broadcast_inventory_snapshot(&self) {
        use crate::p2p::inventory_broadcaster::{
            build_bounded_inventory_publications, gather_hints, LocalInventory as _,
            StaticInventory, INVENTORY_GOSSIP_PAYLOAD_BUDGET,
        };
        use crate::p2p::inventory_gossip::INVENTORY_TOPIC;

        // Fetch hashes directly so I/O failure can skip the tick. Must happen
        // before constructing StaticInventory + entering build_snapshot.
        let hashes = match self.blob_store.list_hashes() {
            Ok(h) => h,
            Err(e) => {
                warn!(
                    target: "elohim_storage::inventory",
                    error = %e,
                    "T22: list_hashes failed; skipping inventory snapshot tick \
                     (would have corrupted remote projection)"
                );
                return;
            }
        };

        // Convert raw filenames to validated BlobAddress instances, dropping
        // any non-canonical entries. The production blob store always emits
        // canonical sha256-<hex> names; this filter guards against filesystem
        // contamination (temp files, .DS_Store, etc.).
        use crate::p2p::inventory_gossip::BlobAddress;
        use std::sync::Once;
        static WARN_ONCE_BROADCAST: Once = Once::new();
        let mut dropped_broadcast = 0_usize;
        let blob_addresses: Vec<BlobAddress> = hashes
            .into_iter()
            .filter_map(|s| match BlobAddress::new(s) {
                Ok(a) => Some(a),
                Err(_) => {
                    dropped_broadcast += 1;
                    None
                }
            })
            .collect();
        if dropped_broadcast > 0 {
            WARN_ONCE_BROADCAST.call_once(|| {
                warn!(
                    target: "elohim_storage::inventory",
                    dropped = dropped_broadcast,
                    "broadcast_inventory_snapshot: list_hashes returned non-canonical \
                     filenames; dropped from snapshot (logged once per session)"
                );
            });
        }
        let inventory = StaticInventory::new(blob_addresses);
        let local_peer_id = self.peer_id().to_string();
        let now_micros = chrono::Utc::now().timestamp_micros();

        // Wave 3: gather per-blob hints from the content projection. Failures
        // are non-fatal (gather_hints logs internally and returns empty vec).
        let hints = if let Some(pool) = self.db_pool.as_ref() {
            match pool.get() {
                Ok(mut conn) => {
                    gather_hints(&mut conn, &inventory.current_hashes(), &local_peer_id)
                }
                Err(_) => vec![],
            }
        } else {
            vec![]
        };

        let hashes_for_record: Vec<String> = inventory
            .current_hashes()
            .iter()
            .map(|a| a.as_str().to_string())
            .collect();
        let count = hashes_for_record.len();
        let publications = match build_bounded_inventory_publications(
            &local_peer_id,
            &inventory,
            &self.inventory_seq,
            now_micros,
            hints,
            INVENTORY_GOSSIP_PAYLOAD_BUDGET,
        ) {
            Ok(publications) => publications,
            Err(e) => {
                warn!(
                    target: "elohim_storage::inventory",
                    error = %e,
                    "T22: failed to build bounded inventory publication"
                );
                return;
            }
        };
        let page_count = publications.len();
        let last_sequence = publications.last().map(|p| p.sequence()).unwrap_or(0);

        // Plan 4: every bounded page routes through the same dual publisher, so
        // libp2p and iroh receive byte-identical snapshot+delta sequences.
        for (page_index, publication) in publications.into_iter().enumerate() {
            let bytes = match publication.to_bytes() {
                Ok(bytes) => bytes,
                Err(e) => {
                    warn!(
                        target: "elohim_storage::inventory",
                        error = %e,
                        page_index,
                        "T22: failed to serialize bounded inventory page"
                    );
                    return;
                }
            };
            if let Err(e) = self.gossip_publisher.publish(INVENTORY_TOPIC, bytes) {
                warn!(
                    target: "elohim_storage::inventory",
                    error = %e,
                    page_index,
                    page_count,
                    "T22: bounded inventory dual-publish failed"
                );
                return;
            }
        }

        // T22 review fix #3: delegate the parity-record write to the
        // shared method so poisoned-lock handling stays consistent
        // with `P2PHandle::set_last_gossiped_inventory`.
        self.set_last_gossiped_inventory(hashes_for_record);
        info!(
            target: "elohim_storage::inventory",
            count,
            page_count,
            sequence = last_sequence,
            payload_budget = INVENTORY_GOSSIP_PAYLOAD_BUDGET,
            "T22: published bounded inventory refresh via DualGossipPublisher"
        );
    }

    /// P3-4: Build and publish a [`SalvageCapacityAd`] on the salvage-capacity
    /// gossip topic. Opt-in advertisement of this peer's spare salvage capacity;
    /// receivers project it into `salvage_capacity`, populating the candidate
    /// pool the salvage pass ranks over.
    ///
    /// **Gated**: only runs when this node is opted in
    /// (`salvage_capacity_enabled`) AND is an always-on archetype
    /// (`node` / `steward`) — only always-on peers advertise by default. A node
    /// is never conscripted (the imago-dei floor); silently no-ops otherwise.
    ///
    /// **Identity**: advertises `self_cid` (the agent_cid namespace), NOT the
    /// libp2p peer id — the candidate pool is agent_cid-keyed so the XOR
    /// placement metric never crosses namespaces.
    ///
    /// **Lock contract**: does NOT acquire the swarm lock; publish routes through
    /// the gossip publisher (command channel). Callers from `run()` select! arms
    /// drop their swarm guard first.
    ///
    /// Idempotent and safe to call repeatedly.
    async fn broadcast_salvage_capacity(&self) {
        use crate::p2p::salvage_gossip::{SalvageCapacityAd, SALVAGE_CAPACITY_TOPIC};

        if !self.config.salvage_capacity_enabled {
            return; // opt-in consent gate (default off) — never conscripted
        }
        // Only always-on archetypes advertise by default.
        let archetype = self.config.device_archetype.clone().unwrap_or_default();
        if !matches!(archetype.as_str(), "node" | "steward") {
            return;
        }
        let Some(agent_cid) = self.config.self_cid.clone() else {
            debug!(
                target: "elohim_storage::salvage",
                "P3: self_cid not configured; skipping salvage-capacity advertisement"
            );
            return;
        };

        // P3-4 best-effort spare_bytes: a real capacity probe (fs4 free space −
        // pantry usage) is a follow-on (the candidate seam already carries
        // `spare_bytes` for capacity-weighted strategies). The MVP XOR strategy
        // ignores spare_bytes, so 0 here is harmless for placement; advertise 0
        // until the probe lands.
        let spare_bytes: u64 = 0;
        let seq = self.salvage_seq.next();

        let ad = SalvageCapacityAd {
            agent_cid: agent_cid.clone(),
            spare_bytes,
            archetype: archetype.clone(),
            seq,
            signature: vec![0x00], // Stage 1 structural non-empty
        };

        let bytes = match ad.to_bytes() {
            Ok(b) => b,
            Err(e) => {
                warn!(
                    target: "elohim_storage::salvage",
                    error = %e,
                    "P3: failed to serialize salvage-capacity ad"
                );
                return;
            }
        };

        if let Err(e) = self.gossip_publisher.publish(SALVAGE_CAPACITY_TOPIC, bytes) {
            warn!(
                target: "elohim_storage::salvage",
                error = %e,
                "P3: salvage-capacity publish failed (often: no subscribed peers yet)"
            );
            return;
        }
        debug!(
            target: "elohim_storage::salvage",
            agent_cid = %agent_cid,
            archetype = %archetype,
            seq,
            "P3: published salvage-capacity advertisement"
        );
    }

    /// Wave 3: score advertised blobs against local active `replicates-dwelling`
    /// commitments and enqueue HIGH-priority blobs for bounded fetch.
    ///
    /// Called once per inventory message (snapshot or delta) — active commitments
    /// are queried ONCE here and used to score all hashes in the message.
    ///
    /// Anti-storm: uses `try_acquire_owned` on `commitment_fetch_semaphore` to
    /// drop fetches when the concurrency limit is saturated. The next gossip tick
    /// will retry. Does not block the receive arm (inline sync diesel + spawn).
    ///
    /// The `gap_queue` (content-level replication seam) is deliberately NOT used —
    /// commitment-driven blob fetch is a separate seam at the blob layer.
    fn score_and_enqueue_snapshot(
        &self,
        conn: &mut diesel::SqliteConnection,
        source_peer_id: &str,
        hints: &[crate::p2p::inventory_gossip::BlobHint],
        hashes: &[String],
    ) {
        use crate::p2p::blob_fetch::{finalize_fetch_success, race_fetch, FetchOutcome};
        use crate::p2p::inventory_gossip::BlobHint;
        use crate::services::replication_prioritizer::{
            active_commitments_for_provider, score_advertised_blob, AdvertisedBlob, FetchPriority,
        };
        use std::collections::HashMap;

        let Some(self_cid) = self.config.self_cid.as_deref() else {
            return; // No identity — cannot match commitments
        };

        // Load active commitments once per message.
        let commitments = match active_commitments_for_provider(conn, self_cid) {
            Ok(c) => c,
            Err(e) => {
                debug!(
                    target: "elohim_storage::inventory",
                    error = %e,
                    "score_and_enqueue: failed to load active commitments; skipping"
                );
                return;
            }
        };

        if commitments.is_empty() {
            return; // Fast path: no commitments → nothing to score
        }

        // Build hint map keyed by blob_hash string (O(1) lookup per hash).
        let hint_map: HashMap<&str, &BlobHint> =
            hints.iter().map(|h| (h.address.as_str(), h)).collect();

        let fresh_after = chrono::Utc::now()
            .checked_sub_signed(chrono::Duration::seconds(
                self.config.inventory_freshness_seconds as i64,
            ))
            .unwrap_or_else(chrono::Utc::now)
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();

        for hash in hashes {
            let hint = hint_map.get(hash.as_str());
            let advertised = AdvertisedBlob {
                blob_cid: hash.clone(),
                source_peer_cid: source_peer_id.to_string(),
                blob_size_bytes: hint.and_then(|h| h.size_bytes),
                recipient_hub_id_hint: hint.and_then(|h| h.recipient_hub_id.clone()),
                epr_kind_hint: hint.and_then(|h| h.epr_kind.clone()),
            };

            // Passive gossip replication: commons does not fire (content_id_ctx
            // = None). Only dwelling matches reach High here; the gate admits
            // anything above Skip so the future active-pull path (Medium) works.
            if score_advertised_blob(&advertised, &commitments, None) == FetchPriority::Skip {
                continue;
            }

            // Drop-on-saturation: anti-fetch-storm guard.
            let permit = match self.commitment_fetch_semaphore.clone().try_acquire_owned() {
                Ok(p) => p,
                Err(_) => {
                    debug!(
                        target: "elohim_storage::inventory",
                        hash = %hash,
                        "Wave3: commitment fetch semaphore saturated — dropping; next gossip tick retries"
                    );
                    continue;
                }
            };

            let Some(pool) = self.db_pool.clone() else {
                continue; // No pool — cannot finalize
            };

            let cmd_tx = self.command_tx.clone();
            let blob_store = self.blob_store.clone();
            let peer_metrics = self.peer_metrics.clone();
            let self_cid_owned = self_cid.to_string();
            let hash_owned = hash.clone();
            let parallelism = self.config.fetch_blob_parallelism.max(1);
            let timeout =
                std::time::Duration::from_secs(self.config.fetch_blob_timeout_seconds.max(1));
            // Clone fresh_after per-task so the String is not moved.
            let fresh_after_owned = fresh_after.clone();

            tokio::spawn(async move {
                // Hold permit for lifetime of the task — releases on drop.
                let _permit = permit;

                // Resolve candidates from peer_blob_inventory, then re-order by score.
                //
                // Wave-3: `select_serve_peers` returns score-ordered agent_cids from the
                // agent_cid-native `shard_locations` source. Each is resolved to a libp2p
                // peer id via `peer_transport_manifest`. Resolvable entries form a score-
                // ordered prefix; the original inventory list is appended deduped as fallback.
                // In prod the manifest is empty → prefix empty → inventory list unchanged
                // (today's behavior). No availability regression.
                let candidates = {
                    match pool.get() {
                        Ok(mut c) => {
                            let inventory: Vec<String> =
                                crate::db::peer_blob_inventory::lookup_hosts(
                                    &mut c,
                                    &hash_owned,
                                    &fresh_after_owned,
                                )
                                .unwrap_or_default()
                                .into_iter()
                                .map(|r| r.peer_id)
                                .collect();

                            // Score-order: resolve agent_cids → libp2p ids.
                            // Resolution requires peer_transport_manifest (iroh feature).
                            // Without it the prefix is empty → inventory unchanged (safe degradation).
                            #[cfg(feature = "p2p-iroh")]
                            let score_prefix: Vec<String> =
                                crate::services::serve_routing::select_serve_peers(
                                    &mut c,
                                    &hash_owned,
                                    inventory.len().max(8),
                                )
                                .unwrap_or_default()
                                .into_iter()
                                .filter_map(|cid| {
                                    crate::p2p_iroh::peer_map::lookup_by_agent_cid(&mut c, &cid)
                                        .ok()
                                        .flatten()
                                        .and_then(|m| m.libp2p.map(|l| l.peer_id))
                                })
                                .collect();
                            #[cfg(not(feature = "p2p-iroh"))]
                            let score_prefix: Vec<String> = Vec::new();

                            if score_prefix.is_empty() {
                                inventory
                            } else {
                                let prefix_set: std::collections::HashSet<&str> =
                                    score_prefix.iter().map(String::as_str).collect();
                                let tail: Vec<String> = inventory
                                    .into_iter()
                                    .filter(|p| !prefix_set.contains(p.as_str()))
                                    .collect();
                                let mut merged = score_prefix;
                                merged.extend(tail);
                                merged
                            }
                        }
                        Err(_) => return,
                    }
                };

                // Filter to connected peers at task start.
                let connected_set: std::collections::HashSet<String> = peer_metrics
                    .iter()
                    .filter_map(|e| {
                        if e.value().is_connected {
                            Some(e.key().clone())
                        } else {
                            None
                        }
                    })
                    .collect();
                let is_connected = move |peer: &str| connected_set.contains(peer);

                let outcome = race_fetch(
                    &hash_owned,
                    candidates,
                    &cmd_tx,
                    is_connected,
                    parallelism,
                    timeout,
                )
                .await;

                if let FetchOutcome::Hit { bytes, source_peer } = outcome {
                    if let Ok(mut c) = pool.get() {
                        if let Err(e) = finalize_fetch_success(
                            &mut c,
                            &hash_owned,
                            &source_peer,
                            &bytes,
                            &self_cid_owned,
                            &blob_store,
                        )
                        .await
                        {
                            warn!(
                                target: "elohim_storage::inventory",
                                hash = %hash_owned,
                                error = %e,
                                "Wave3: finalize_fetch_success failed"
                            );
                        }
                    }
                }
            });
        }
    }

    /// T22 review fix #3: write the parity-tracking record from the P2PNode
    /// run-loop side. Mirrors `P2PHandle::set_last_gossiped_inventory` — both
    /// share the same `Arc<RwLock<Vec<String>>>`, so a method on each side
    /// keeps the poisoned-lock warn pattern consistent without round-tripping
    /// through the command channel.
    fn set_last_gossiped_inventory(&self, hashes: Vec<String>) {
        if let Ok(mut guard) = self.last_gossiped.write() {
            *guard = hashes;
        } else {
            warn!(
                target: "elohim_storage::inventory",
                "T22: last_gossiped lock poisoned; parity diagnostic may be stale"
            );
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
            P2PCommand::AnnounceLocalChange {
                doc_id,
                change_hash,
            } => {
                // The send site the standing red names. The `sync_round` planner
                // stays the ONLY constructor of these requests — making it return
                // requests without a caller that sends them would turn the test
                // green while nothing propagates.
                let peers: Vec<PeerId> = swarm.connected_peers().cloned().collect();
                let namespace = crate::sync::projector::PROJECTION_NAMESPACE;
                // The doorbell DELIVERS when the change fits — and it carries THE
                // ANNOUNCED CHANGE, addressed by the hash the command already
                // names, not the doc's whole history. `get_changes_since(.., &[])`
                // is every change since genesis: on a doc with real history it
                // overflows the bound on every local edit (so the eager path
                // silently degrades to pull-on-announce, +1 RTT) after burning a
                // full-doc serialization to find that out. One change is a
                // roughly constant, tiny payload however mature the doc is.
                //
                // `None` (change evicted/compacted, or a read error) degrades to
                // the metadata-only doorbell and the receiver pulls; the 60s round
                // backstops both.
                let change_data = match self
                    .sync_manager
                    .get_change_by_hash(namespace, &doc_id, &change_hash)
                    .await
                {
                    Ok(Some(bytes)) => sync_round::bounded_announce_payload(vec![bytes]),
                    Ok(None) => {
                        debug!(doc_id = %doc_id, change_hash = %change_hash, "Announce: doc no longer holds the announced change, sending doorbell only");
                        None
                    }
                    Err(e) => {
                        debug!(doc_id = %doc_id, error = %e, "Announce: could not load change bytes, sending doorbell only");
                        None
                    }
                };
                let eager_bytes = change_data.as_ref().map(|d| d.len()).unwrap_or(0);
                let announcements = sync_round::announcements_for_local_change_with_data(
                    namespace,
                    &doc_id,
                    &change_hash,
                    &peers,
                    change_data,
                );
                if announcements.is_empty() {
                    debug!(doc_id = %doc_id, "Announce: no connected peers, the round remains the propagation path");
                }
                for (peer_id, request) in announcements {
                    swarm
                        .behaviour_mut()
                        .sync_protocol
                        .send_request(&peer_id, request);
                    crate::metrics::inc_sync_request("announce_change");
                }
                debug!(
                    doc_id = %doc_id, change_hash = %change_hash, peers = peers.len(),
                    eager_bytes = eager_bytes,
                    "Announced local change to connected peers"
                );
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
            // Plan 4: route through DualGossipPublisher for dual-stack publish.
            P2PCommand::PublishRecoveryInvitation(inv) => match inv.to_bytes() {
                Ok(bytes) => {
                    if let Err(e) = self
                        .gossip_publisher
                        .publish(RECOVERY_INVITATION_TOPIC, bytes)
                    {
                        warn!(
                            target: "elohim_storage::recovery",
                            request_hash = %inv.request_hash,
                            error = %e,
                            "PublishRecoveryInvitation dual-publish failed"
                        );
                    } else {
                        info!(
                            target: "elohim_storage::recovery",
                            request_hash = %inv.request_hash,
                            human_id = %inv.human_id,
                            "Published RecoveryInvitation via DualGossipPublisher"
                        );
                    }
                }
                Err(e) => warn!(
                    target: "elohim_storage::recovery",
                    request_hash = %inv.request_hash,
                    error = ?e,
                    "Failed to encode RecoveryInvitation"
                ),
            },
            // A.10: publish identity binding to elohim/identity/binding topic.
            // Triggered by ReconcileController::on_agent_peer_binding when a local
            // AgentPeerBinding DHT signal arrives. Best-effort: publish failure is
            // logged but does not block the controller loop.
            //
            // Plan 4: single dispatch through DualGossipPublisher — fans out
            // byte-identical bytes to both libp2p and iroh simultaneously.
            // Wire format: rmp_serde::to_vec (positional) — UNCHANGED.
            P2PCommand::PublishIdentityBinding(payload) => match payload.to_bytes() {
                Ok(bytes) => {
                    if let Err(e) = self.gossip_publisher.publish(
                        crate::p2p::identity_binding_gossip::IDENTITY_BINDING_TOPIC,
                        bytes,
                    ) {
                        warn!(
                            target: "elohim_storage::identity",
                            peer_id = %payload.peer_id,
                            agent_cid = %payload.agent_cid,
                            error = %e,
                            "PublishIdentityBinding dual-publish failed"
                        );
                    } else {
                        info!(
                            target: "elohim_storage::identity",
                            peer_id = %payload.peer_id,
                            agent_cid = %payload.agent_cid,
                            "Published IdentityBindingGossip via DualGossipPublisher"
                        );
                    }
                }
                Err(e) => warn!(
                    target: "elohim_storage::identity",
                    peer_id = %payload.peer_id,
                    error = ?e,
                    "Failed to encode IdentityBindingGossip"
                ),
            },
            // Plan 4: route through DualGossipPublisher for dual-stack publish.
            P2PCommand::PublishRecoveryRevocation(msg) => match msg.to_bytes() {
                Ok(bytes) => {
                    if let Err(e) = self
                        .gossip_publisher
                        .publish(RECOVERY_REVOCATION_TOPIC, bytes)
                    {
                        warn!(
                            target: "elohim_storage::recovery",
                            revocation_id = %msg.revocation_id,
                            error = %e,
                            "PublishRecoveryRevocation dual-publish failed"
                        );
                    } else {
                        info!(
                            target: "elohim_storage::recovery",
                            revocation_id = %msg.revocation_id,
                            human_id = %msg.human_id,
                            status = %msg.status,
                            "Published RecoveryRevocationMessage via DualGossipPublisher"
                        );
                    }
                }
                Err(e) => warn!(
                    target: "elohim_storage::recovery",
                    revocation_id = %msg.revocation_id,
                    error = ?e,
                    "Failed to encode RecoveryRevocationMessage"
                ),
            },
            // Step-zero substrate gossip: publish a Holochain conductor's
            // agent_info JSON via DualGossipPublisher. Best-effort — publish
            // failure is logged and the next 60s heartbeat retries.
            P2PCommand::PublishConductorAgentInfo(payload) => match payload.to_bytes() {
                Ok(bytes) => {
                    if let Err(e) = self.gossip_publisher.publish(
                        crate::p2p::conductor_agent_info_gossip::CONDUCTOR_AGENT_INFO_TOPIC,
                        bytes,
                    ) {
                        warn!(
                            target: "elohim_storage::agent_info",
                            error = %e,
                            "PublishConductorAgentInfo dual-publish failed"
                        );
                    } else {
                        debug!(
                            target: "elohim_storage::agent_info",
                            published_at = payload.published_at,
                            "conductor agent_info published to substrate"
                        );
                    }
                }
                Err(e) => warn!(
                    target: "elohim_storage::agent_info",
                    error = ?e,
                    "PublishConductorAgentInfo to_bytes failed"
                ),
            },
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
            P2PCommand::InventoryAdvertisement {
                source_peer_id,
                hints,
                hashes,
            } => {
                let Some(pool) = self.db_pool.as_ref() else {
                    warn!(
                        target: "elohim_storage::inventory",
                        peer_id = %source_peer_id,
                        count = hashes.len(),
                        "Queued inventory fetch work dropped: content DB unavailable"
                    );
                    return;
                };
                match pool.get() {
                    Ok(mut conn) => {
                        self.score_and_enqueue_snapshot(&mut conn, &source_peer_id, &hints, &hashes)
                    }
                    Err(error) => warn!(
                        target: "elohim_storage::inventory",
                        peer_id = %source_peer_id,
                        count = hashes.len(),
                        error = %error,
                        "Queued inventory fetch work dropped: DB pool exhausted"
                    ),
                }
            }
            // T21: Stage-2 wiring — issue a real `/elohim/blob/1.0.0` request to
            // the explicit peer_id and stash the reply oneshot in
            // `pending_blob_fetches`. The blob-protocol event handler resolves
            // the entry on `Message::Response` or cleans up on `OutboundFailure`.
            P2PCommand::FetchBlob {
                peer_id,
                hash,
                reply,
            } => {
                let request = BlobFetchRequest { hash: hash.clone() };
                let request_id = swarm
                    .behaviour_mut()
                    .blob_protocol
                    .send_request(&peer_id, request);
                debug!(
                    target: "elohim_storage::blob_fetch",
                    peer_id = %peer_id,
                    hash = %hash,
                    request_id = ?request_id,
                    "T21: blob fetch request sent on /elohim/blob/1.0.0"
                );
                self.pending_blob_fetches
                    .lock()
                    .await
                    .insert(request_id, reply);
            }
            // T23 review fix #2: ConnectionEstablished triggers reconcile via
            // this command instead of awaiting it inline, so swarm events
            // (inventory snapshots, identity handshakes) from a reconnect
            // burst are not head-of-line-blocked by sync diesel queries.
            P2PCommand::TriggerCustodyReconcile => {
                self.run_custody_reconcile().await;
            }
            // F-T19: issue a `/elohim/view-federation/1.0.0` request to the named peer
            // and store the oneshot sender in `pending_view_federations`, keyed by the
            // `OutboundRequestId` returned by `send_request`. The event handler below
            // resolves it when `Message::Response` or `OutboundFailure` arrives.
            P2PCommand::ViewFederate {
                peer,
                request,
                respond,
            } => {
                let request_id = swarm
                    .behaviour_mut()
                    .view_federation
                    .send_request(&peer, request);
                debug!(
                    target: "elohim_storage::view_federation",
                    peer = %peer,
                    request_id = ?request_id,
                    "F-T19: view-federation request sent on /elohim/view-federation/1.0.0"
                );
                self.pending_view_federations
                    .lock()
                    .await
                    .insert(request_id, respond);
            }

            // F-A(ii): deliver a slice built OFF the event loop. `send_response`
            // is synchronous, so this arm never awaits — the whole point of the
            // detour is that the SLOW half (the slice-build, which may call the
            // conductor) already happened in a spawned task.
            P2PCommand::SendViewFederationResponse {
                peer,
                channel,
                response,
            } => {
                if let Err(_response) = swarm
                    .behaviour_mut()
                    .view_federation
                    .send_response(channel, *response)
                {
                    warn!(
                        target: "elohim_storage::view_federation",
                        peer = %peer,
                        "F-T20: failed to send view-federation response (peer disconnected?)"
                    );
                }
            }

            // T20: issue a `/elohim/shamir-share/1.0.0` request to the named peer
            // and store the oneshot sender in `pending_shamir_shares`, keyed by the
            // `OutboundRequestId`. The event arm below resolves it when
            // `Message::Response` or `OutboundFailure` arrives.
            P2PCommand::RequestShamirShare {
                peer,
                request,
                respond,
            } => {
                let request_id = swarm
                    .behaviour_mut()
                    .shamir_share
                    .send_request(&peer, request);
                debug!(
                    target: "elohim_storage::shamir_share",
                    peer = %peer,
                    request_id = ?request_id,
                    "T20: shamir-share request sent on /elohim/shamir-share/1.0.0"
                );
                self.pending_shamir_shares
                    .lock()
                    .await
                    .insert(request_id, respond);
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
                    // Q11 atomic timing budget: ONLY the put_record call, not
                    // the write-lock acquisition above it.
                    let put_record_started = std::time::Instant::now();
                    let result = swarm
                        .behaviour_mut()
                        .kademlia
                        .put_record(record, libp2p::kad::Quorum::One);
                    crate::metrics::observe_atom_duration(
                        crate::metrics::ConvergenceAtom::PutRecord,
                        put_record_started.elapsed(),
                    );
                    result
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

            let page = match crate::db::content_diesel::list_content(
                &mut conn,
                &app_ctx,
                &query,
                crate::db::content_diesel::MinTrust::Invisible,
            ) {
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
                // T24: learn bootstrap-addr → PeerId from outbound dials so the
                // persistent-peering retry knows WHICH configured link dropped.
                if endpoint.is_dialer() {
                    let dialed = endpoint.get_remote_address().to_string();
                    let is_bootstrap = self.config.bootstrap_nodes.iter().any(|cfg| {
                        cfg.parse::<Multiaddr>()
                            .map(|a| a.to_string() == dialed)
                            .unwrap_or(false)
                    });
                    if is_bootstrap {
                        self.bootstrap_peer_ids.insert(dialed.clone(), peer_id);
                        info!(addr = %dialed, peer = %peer_id, "Bootstrap link established (persistent peering tracks it)");
                    }
                }
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
                // T23: trigger a custody reconcile pass on every successful
                // connection so own commitments where this peer is now a
                // candidate get a kick chance immediately, and other peers'
                // commitments to us get re-evaluated against fresh
                // peer_blob_inventory state. No-ops silently when self_cid /
                // db_pool are absent. The placement-gap cooldown inside
                // `reconcile_pass` prevents duplicate events when many peers
                // connect in a short window.
                //
                // T23 review fix #2: dispatch via the command channel
                // instead of awaiting `run_custody_reconcile().await` inline
                // here. The reconcile pass runs sync diesel queries; under
                // a reconnect burst, awaiting inline serializes those
                // queries on the swarm event-loop task and head-of-line-
                // blocks inventory snapshots / identity handshakes from the
                // very peers that just connected. `try_send` is
                // non-blocking — when the queue is full (extreme overload),
                // the trigger is dropped and the timer tick recovers.
                if let Err(e) = self
                    .command_tx
                    .try_send(P2PCommand::TriggerCustodyReconcile)
                {
                    debug!(
                        target: "elohim_storage::reconcile",
                        error = ?e,
                        "T23: failed to queue ConnectionEstablished reconcile trigger; will retry on timer tick"
                    );
                }
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
                            // Err payload is the whole unsent response — summarize, never Debug-dump it.
                            warn!(peer = %peer, response = %e.summary(), "Failed to send shard response");
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
                                    // A proactive quilt-draw IS a serve-transfer (the peer served
                                    // us the bytes), so route it through finalize_quilt_draw to
                                    // leave the REA delivery trail (serve-blob event) AND the
                                    // peer_blob_inventory row — exactly like an on-demand fetch.
                                    // p2p-design-gate 2026-06-18 (see blob_fetch::finalize_quilt_draw).
                                    // Falls back to a bare store only when no db pool is wired.
                                    if let Some(ref pool) = self.db_pool {
                                        match pool.get() {
                                            Ok(mut conn) => {
                                                let self_cid = self
                                                    .config
                                                    .self_cid
                                                    .clone()
                                                    .unwrap_or_default();
                                                match crate::p2p::blob_fetch::finalize_quilt_draw(
                                                    &mut conn,
                                                    &blob_hash,
                                                    &peer.to_string(),
                                                    &data,
                                                    &self_cid,
                                                    &self.blob_store,
                                                )
                                                .await
                                                {
                                                    Ok(true) => info!(
                                                        content_id = %content_id,
                                                        blob_hash = %blob_hash,
                                                        size = data.len(),
                                                        source_peer = %peer,
                                                        "quilt draw: blob stocked + serve-blob delivery event booked"
                                                    ),
                                                    Ok(false) => warn!(
                                                        content_id = %content_id,
                                                        blob_hash = %blob_hash,
                                                        source_peer = %peer,
                                                        "quilt draw: pulled bytes failed hash verification; discarded"
                                                    ),
                                                    Err(e) => warn!(
                                                        content_id = %content_id,
                                                        blob_hash = %blob_hash,
                                                        error = %e,
                                                        "quilt draw: finalize_quilt_draw failed"
                                                    ),
                                                }
                                            }
                                            Err(e) => {
                                                // Pool exhausted: preserve the stock even though we
                                                // cannot book the delivery event this round (a later
                                                // on-demand serve / parity sweep reconciles).
                                                warn!(
                                                    content_id = %content_id,
                                                    blob_hash = %blob_hash,
                                                    error = %e,
                                                    "quilt draw: db pool unavailable; storing blob without delivery event"
                                                );
                                                let _ = self.blob_store.store(&data).await;
                                            }
                                        }
                                    } else {
                                        match self.blob_store.store(&data).await {
                                            Ok(result) => info!(
                                                content_id = %content_id,
                                                blob_hash = %blob_hash,
                                                size = data.len(),
                                                already_existed = result.already_existed,
                                                "quilt draw: blob stocked in pantry after replication (no db pool; delivery event skipped)"
                                            ),
                                            Err(e) => warn!(
                                                content_id = %content_id,
                                                blob_hash = %blob_hash,
                                                error = %e,
                                                "quilt draw: failed to stock blob in pantry"
                                            ),
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
                                        // Reclaim this page's cursor: (peer, offset).
                                        // Its presence identifies this as a scheduled
                                        // ListContent chain (vs an ad-hoc list) and
                                        // tells the scheduler which peer to settle.
                                        let page_cursor = self
                                            .list_content_cursors
                                            .lock()
                                            .await
                                            .remove(&request_id);
                                        // Close the requester half of the round
                                        // trip. Only a cursor-bearing response is
                                        // timed: an ad-hoc list (no cursor) is not
                                        // part of the replication cycle and would
                                        // pollute the series with a different
                                        // population.
                                        if let Some((_, _, sent_at)) = page_cursor.as_ref() {
                                            crate::metrics::observe_atom_duration(
                                                crate::metrics::ConvergenceAtom::InventoryRequest,
                                                sent_at.elapsed(),
                                            );
                                        }
                                        let page_len = items.len();
                                        let remote_ids: Vec<String> =
                                            items.into_iter().map(|i| i.id).collect();
                                        // Primary pin re-admission: a peer
                                        // advertising a RETIRED pin's head_ref is
                                        // fresh evidence its bytes exist, so the
                                        // pin revives here rather than waiting out
                                        // the cooldown. Inventory is metadata, not
                                        // bytes — this re-opens the ASK, it never
                                        // marks anything fetched.
                                        self.readmit_pins_named_by_inventory(&remote_ids);
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

                                        // Follow the has_more page cursor (mirrors the
                                        // ListDocuments continuation): a corpus larger
                                        // than one page is enumerated across pages, so
                                        // its cold tail is no longer invisible to this
                                        // arm. The chain stays in-flight until the final
                                        // page; only then does the peer settle to base
                                        // cadence. A response with no tracked cursor is a
                                        // pre-fix / raced list — handled as before, no
                                        // scheduler transition.
                                        if let Some((cursor_peer, offset, _sent_at)) = page_cursor {
                                            match crate::p2p::sync_protocol::next_doc_list_offset(
                                                offset, page_len, has_more,
                                            ) {
                                                Some(next_offset) => {
                                                    let next_request = ShardRequest::ListContent {
                                                        reach_filter: None,
                                                        offset: next_offset,
                                                        limit: REPLICATION_LIST_PAGE_LIMIT,
                                                    };
                                                    let mut swarm = self.swarm.write().await;
                                                    let next_id = swarm
                                                        .behaviour_mut()
                                                        .shard_protocol
                                                        .send_request(&cursor_peer, next_request);
                                                    drop(swarm);
                                                    // Keep the chain in-flight; record the
                                                    // next page's cursor. No mark_started —
                                                    // the peer is already in flight.
                                                    // Fresh instant per PAGE, not per chain: each
                                                    // page is its own round trip, and carrying the
                                                    // chain's start forward would report page 5 as
                                                    // costing the sum of pages 1-5.
                                                    self.list_content_cursors.lock().await.insert(
                                                        next_id,
                                                        (
                                                            cursor_peer,
                                                            next_offset,
                                                            std::time::Instant::now(),
                                                        ),
                                                    );
                                                    debug!(
                                                        peer = %cursor_peer,
                                                        offset = next_offset,
                                                        request_id = ?next_id,
                                                        "Requested next ListContent page"
                                                    );
                                                }
                                                None => {
                                                    // Chain complete — settle the peer to
                                                    // base cadence (backoff reset).
                                                    self.replication_scheduler
                                                        .mark_success(cursor_peer)
                                                        .await;
                                                }
                                            }
                                        }
                                    }
                                    ShardResponse::Content(record) => {
                                        let content_id = record.id.clone();
                                        debug!(id = %content_id, "Received content record from peer");
                                        self.pending_replication_fetches
                                            .lock()
                                            .await
                                            .remove(&request_id);
                                        self.pending_acquisition_fetches
                                            .lock()
                                            .await
                                            .remove(&request_id);
                                        // Transport-neutral store (shared with the iroh pull
                                        // leg — see `acquire_over_iroh`); the two libp2p-only
                                        // tails (kad head record, quilt-draw blob pull over the
                                        // same libp2p peer) stay here.
                                        let ctx = self.acquisition_ingest_ctx();
                                        if let StoredRecord::Stored { blob_hash, .. } =
                                            store_acquired_record(&ctx, *record).await
                                        {
                                            self.publish_epr_head_record(&content_id).await;
                                            if let Some(ref hash) = blob_hash {
                                                if !hash.is_empty()
                                                    && !self.blob_store.exists(hash).await
                                                {
                                                    let pull_request =
                                                        ShardRequest::Get { hash: hash.clone() };
                                                    let mut swarm = self.swarm.write().await;
                                                    let pull_id = swarm
                                                        .behaviour_mut()
                                                        .shard_protocol
                                                        .send_request(&peer, pull_request);
                                                    drop(swarm);
                                                    self.pending_blob_pulls.lock().await.insert(
                                                        pull_id,
                                                        (content_id.clone(), hash.clone()),
                                                    );
                                                    info!(
                                                        content_id = %content_id,
                                                        blob_hash = %hash,
                                                        source_peer = %peer,
                                                        "quilt draw: pulling blob from peer after content replication"
                                                    );
                                                }
                                            }
                                        }
                                        self.replication_state.update_caught_up().await;
                                    }
                                    ShardResponse::ContentNotFound => {
                                        // Remove from acquisition tracking first (if it was an acquisition fetch)
                                        if let Some(acq_id) = self
                                            .pending_acquisition_fetches
                                            .lock()
                                            .await
                                            .remove(&request_id)
                                        {
                                            crate::metrics::inc_acquisition_outcome("fetch_error");
                                            self.acquisition.mark_failed(&acq_id).await;
                                        }
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
                                    // Every OTHER response variant — notably
                                    // `ShardResponse::Error(_)`, which the responder
                                    // returns for "No database pool" / "DB connection
                                    // failed" / "Content fetch failed" — used to fall
                                    // through here as a bare debug log. That LEAKED the
                                    // in-flight slot: the request id was never removed
                                    // from `pending_{acquisition,replication}_fetches`,
                                    // so `DispatchBudget::available(in_flight)` shrank
                                    // permanently. After MAX_ACQUISITION_INFLIGHT (25)
                                    // such responses the acquisition stream wedges at
                                    // `available == 0` and never dispatches again — with
                                    // no log, no metric, and a tracker that reports
                                    // neither pending nor failed. Release the slot and
                                    // record the failure honestly so the retry budget
                                    // (and the exhaustion counters) can see it.
                                    other => {
                                        let acq_id = self
                                            .pending_acquisition_fetches
                                            .lock()
                                            .await
                                            .remove(&request_id);
                                        let repl_id = self
                                            .pending_replication_fetches
                                            .lock()
                                            .await
                                            .remove(&request_id);
                                        if let Some(ref id) = acq_id {
                                            crate::metrics::inc_acquisition_outcome(
                                                "unexpected_response",
                                            );
                                            self.acquisition.mark_failed(id).await;
                                        }
                                        if let Some(ref id) = repl_id {
                                            self.replication_state.mark_failed(id).await;
                                            self.replication_state.update_caught_up().await;
                                        }
                                        if acq_id.is_some() || repl_id.is_some() {
                                            warn!(
                                                request_id = ?request_id,
                                                content_id = ?acq_id.or(repl_id),
                                                response = %other.summary(),
                                                "Fetch got an unexpected shard response; \
                                                 in-flight slot released and item marked failed"
                                            );
                                        } else {
                                            debug!(request_id = ?request_id, response = ?other, "Received shard response");
                                        }
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
                // Settle a failed ListContent discovery chain: drop its page
                // cursor and back the peer off (interval doubles up to the cap).
                // This is the arm that breaks the timeout storm — a Timeout here
                // no longer lets the next tick re-fire the same request; the peer
                // is deferred until its (now longer) backoff elapses.
                if let Some((cursor_peer, offset, sent_at)) =
                    self.list_content_cursors.lock().await.remove(&request_id)
                {
                    // Record the FAILED round trip at its full elapsed value. A
                    // request that burned the whole 30s timeout is the most
                    // expensive one the replication cycle makes; dropping it
                    // would make inventory_request p95 improve exactly as the
                    // link degrades — coordinated omission, and the timeout
                    // storm this arm exists to break would be invisible in the
                    // very histogram meant to show it.
                    crate::metrics::observe_atom_duration(
                        crate::metrics::ConvergenceAtom::InventoryRequest,
                        sent_at.elapsed(),
                    );
                    debug!(
                        peer = %cursor_peer, offset = offset, error = ?error,
                        elapsed_ms = sent_at.elapsed().as_secs_f64() * 1_000.0,
                        "ListContent chain failed at transport level — backing off peer"
                    );
                    self.replication_scheduler.mark_failure(cursor_peer).await;
                }
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
                // Clean up acquisition state if this was an acquisition fetch
                if let Some(content_id) = self
                    .pending_acquisition_fetches
                    .lock()
                    .await
                    .remove(&request_id)
                {
                    // WARN, not debug: this leg was measured failing 100% on both
                    // genesis peers (alpha 29/29, beta 5/5, fetched=0) while emitting
                    // NOTHING at the deployed level — 1.13M log lines over 6h contained
                    // zero occurrences of this message. A leg that fails every attempt
                    // must not be indistinguishable from an idle healthy one.
                    warn!(content_id = %content_id, error = ?error, "Acquisition fetch failed at transport level");
                    crate::metrics::inc_acquisition_outcome("transport_failure");
                    self.acquisition.mark_failed(&content_id).await;
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
            // T21: /elohim/blob/1.0.0 — explicit-peer blob fetch protocol.
            // Inbound: read local blob_store and respond Found/NotFound/Error.
            // Outbound: resolve the matching pending_blob_fetches entry.
            behaviour::ElohimStorageBehaviourEvent::BlobProtocol(
                request_response::Event::Message { peer, message },
            ) => match message {
                request_response::Message::Request {
                    request, channel, ..
                } => {
                    // T21 review fix #1: validate the requested hash before touching the
                    // filesystem. `BlobStore::blob_path` does not normalize `..` components,
                    // so a crafted `BlobFetchRequest { hash: "../../etc/passwd" }` could
                    // otherwise read arbitrary files outside the blob root. Treat any
                    // invalid content address as `NotFound` — do not leak that the path was
                    // malformed.
                    let response =
                        match crate::blob_store::BlobStore::parse_content_address(&request.hash) {
                            Ok(_) => match self.blob_store.get(&request.hash).await {
                                Ok(bytes) => {
                                    debug!(
                                        target: "elohim_storage::blob_fetch",
                                        peer = %peer,
                                        hash = %request.hash,
                                        size = bytes.len(),
                                        "T21: serving blob to peer"
                                    );
                                    BlobFetchResponse::found(bytes)
                                }
                                Err(StorageError::NotFound(_))
                                | Err(StorageError::BlobNotFound(_)) => {
                                    // Q3 (minutes-quiesce W1.2): no direct bytes under this
                                    // hash — before giving up, consult the durable shard
                                    // manifest projection. This peer runs as a separate
                                    // actor with no access to `HttpServer::manifests`
                                    // (Q2's in-process cache), so it always falls straight
                                    // to `resolve_manifest_by_hash`'s DB read — the same
                                    // fallback `HttpServer::resolve_manifest` uses on its
                                    // own cache miss. `encoding == "none"` manifests name
                                    // the composite itself (no shards to reassemble from),
                                    // so those are not useful to hand back — the responder
                                    // demonstrably has no bytes for that name either.
                                    let manifest = match self.db_pool.as_ref() {
                                        Some(pool) => pool.get().ok().and_then(|mut conn| {
                                            crate::db::shard_manifests::resolve_manifest_by_hash(
                                                &mut conn,
                                                &request.hash,
                                            )
                                        }),
                                        None => None,
                                    };
                                    match manifest {
                                        Some(m) if m.encoding != "none" => {
                                            debug!(
                                                target: "elohim_storage::blob_swarm",
                                                peer = %peer,
                                                hash = %request.hash,
                                                shards = m.shard_hashes.len(),
                                                "Q3: no direct bytes; replying with shard manifest"
                                            );
                                            BlobFetchResponse::manifest_only(m)
                                        }
                                        _ => {
                                            debug!(
                                                target: "elohim_storage::blob_fetch",
                                                peer = %peer,
                                                hash = %request.hash,
                                                "T21: blob not found locally; replying NotFound"
                                            );
                                            BlobFetchResponse::not_found()
                                        }
                                    }
                                }
                                Err(e) => {
                                    warn!(
                                        target: "elohim_storage::blob_fetch",
                                        peer = %peer,
                                        hash = %request.hash,
                                        error = %e,
                                        "T21: blob fetch error; replying Error"
                                    );
                                    BlobFetchResponse::error(e.to_string())
                                }
                            },
                            Err(_) => {
                                warn!(
                                    target: "elohim_storage::blob_fetch",
                                    peer = %peer,
                                    hash = %request.hash,
                                    "T21: rejected blob request with invalid content address"
                                );
                                BlobFetchResponse::not_found()
                            }
                        };
                    let mut swarm = self.swarm.write().await;
                    if let Err(e) = swarm
                        .behaviour_mut()
                        .blob_protocol
                        .send_response(channel, response)
                    {
                        // Err payload is the whole unsent response (Found carries blob bytes) —
                        // summarize, never Debug-dump it.
                        warn!(
                            target: "elohim_storage::blob_fetch",
                            peer = %peer,
                            response = %e.summary(),
                            "T21: failed to send blob response"
                        );
                    }
                }
                request_response::Message::Response {
                    request_id,
                    response,
                } => {
                    if let Some(reply) = self.pending_blob_fetches.lock().await.remove(&request_id)
                    {
                        match response {
                            BlobFetchResponse::Found(bytes) => {
                                debug!(
                                    target: "elohim_storage::blob_fetch",
                                    request_id = ?request_id,
                                    size = bytes.len(),
                                    "T21: blob fetch returned Found"
                                );
                                let _ = reply
                                    .send(Ok(crate::p2p::blob_fetch::BlobFetchReply::Bytes(bytes)));
                            }
                            BlobFetchResponse::NotFound => {
                                debug!(
                                    target: "elohim_storage::blob_fetch",
                                    request_id = ?request_id,
                                    "T21: blob fetch returned NotFound"
                                );
                                let _ = reply.send(Err("not found".to_string()));
                            }
                            BlobFetchResponse::Error(msg) => {
                                debug!(
                                    target: "elohim_storage::blob_fetch",
                                    request_id = ?request_id,
                                    error = %msg,
                                    "T21: blob fetch returned Error"
                                );
                                let _ = reply.send(Err(msg));
                            }
                            // Q3: no direct bytes, but the peer resolved a
                            // durable manifest for this hash — a valid
                            // outcome the caller pivots on (Q4 swarm-fetch),
                            // not a failure.
                            BlobFetchResponse::Manifest(manifest) => {
                                debug!(
                                    target: "elohim_storage::blob_swarm",
                                    request_id = ?request_id,
                                    blob_hash = %manifest.blob_hash,
                                    shards = manifest.shard_hashes.len(),
                                    "Q3: blob fetch returned a shard manifest instead of bytes"
                                );
                                let _ = reply.send(Ok(
                                    crate::p2p::blob_fetch::BlobFetchReply::Manifest(manifest),
                                ));
                            }
                        }
                    } else {
                        debug!(
                            target: "elohim_storage::blob_fetch",
                            request_id = ?request_id,
                            "T21: response with no pending entry (already resolved or unknown)"
                        );
                    }
                }
            },
            behaviour::ElohimStorageBehaviourEvent::BlobProtocol(
                request_response::Event::OutboundFailure {
                    peer,
                    request_id,
                    error,
                },
            ) => {
                warn!(
                    target: "elohim_storage::blob_fetch",
                    peer = %peer,
                    request_id = ?request_id,
                    error = ?error,
                    "T21: outbound blob fetch failed"
                );
                // T21 review fix #4: preserve OutboundFailure variant identity so the
                // race-fetch consumer can distinguish timeout from dial-failure from
                // protocol-mismatch. libp2p-request-response 0.27 (libp2p 0.54) defines
                // exactly five variants: DialFailure, Timeout, ConnectionClosed,
                // UnsupportedProtocols, Io. There is no NotConnected variant.
                if let Some(reply) = self.pending_blob_fetches.lock().await.remove(&request_id) {
                    use libp2p::request_response::OutboundFailure;
                    let err_str = match &error {
                        OutboundFailure::Timeout => "timeout".to_string(),
                        OutboundFailure::ConnectionClosed => "connection_closed".to_string(),
                        OutboundFailure::DialFailure => "dial_failure".to_string(),
                        OutboundFailure::UnsupportedProtocols => {
                            "unsupported_protocols".to_string()
                        }
                        OutboundFailure::Io(_) => format!("io_error: {error}"),
                    };
                    let _ = reply.send(Err(err_str));
                }
            }
            behaviour::ElohimStorageBehaviourEvent::BlobProtocol(
                request_response::Event::InboundFailure {
                    peer,
                    request_id,
                    error,
                },
            ) => {
                debug!(
                    target: "elohim_storage::blob_fetch",
                    peer = %peer,
                    request_id = ?request_id,
                    error = ?error,
                    "T21: inbound blob fetch failed"
                );
            }
            behaviour::ElohimStorageBehaviourEvent::BlobProtocol(
                request_response::Event::ResponseSent { peer, request_id },
            ) => {
                debug!(
                    target: "elohim_storage::blob_fetch",
                    peer = %peer,
                    request_id = ?request_id,
                    "T21: blob response sent"
                );
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

                    // The peer's REAL HTTP port — a multiaddr carries only the
                    // libp2p transport port and has no bearing on it (see
                    // `DEFAULT_HTTP_PORT`'s doc comment). If the Identify
                    // handshake has already completed for this peer, its
                    // self-declared port is cached here. When the cache is
                    // empty (never identified, OR just cleared by
                    // `ConnectionClosed`), an EXISTING row keeps its
                    // previously-learned port rather than regressing to the
                    // default — see `resolve_discovery_http_port`. Only a
                    // never-seen, never-identified peer starts at
                    // `DEFAULT_HTTP_PORT`; the Identify handler refreshes the
                    // row once the handshake lands.
                    let (cached_http_port, cached_caps) = self
                        .identify_cache
                        .get(&key)
                        .map(|c| (Some(c.http_port), c.capabilities.clone()))
                        .unwrap_or((None, None));

                    self.delivery_peers
                        .entry(key.clone())
                        .and_modify(|p| {
                            if !p.multiaddrs.contains(&addr_str) {
                                p.multiaddrs.push(addr_str.clone());
                            }
                            p.last_seen = now_ms;
                            p.network = "lan".to_string();
                            p.http_port =
                                resolve_discovery_http_port(cached_http_port, Some(p.http_port));
                            if let Some(ref caps) = cached_caps {
                                p.apply_capabilities(caps);
                            }
                        })
                        .or_insert_with(|| {
                            let mut row = DeliveryPeer {
                                peer_id: key,
                                multiaddrs: vec![addr_str],
                                network: "lan".to_string(),
                                capabilities: Vec::new(),
                                serves_extracted: false,
                                serves_compressed: false,
                                cache_tier: DeliveryPeer::default_cache_tier(),
                                last_seen: now_ms,
                                http_port: resolve_discovery_http_port(cached_http_port, None),
                                // Enriched at request time by the HTTP handler — the
                                // discovery path has no DB access.
                                household_id: None,
                                commitments: Vec::new(),
                            };
                            // A peer identified before it was discovered keeps its
                            // declaration; otherwise it starts at the floor every
                            // node meets until its Identify handshake lands.
                            row.apply_capabilities(
                                cached_caps
                                    .as_deref()
                                    .unwrap_or(&DeliveryPeer::discovery_floor_capabilities()),
                            );
                            row
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
                    let response = self.handle_sync_request(peer, request).await;
                    let mut swarm = self.swarm.write().await;
                    if let Err(e) = swarm
                        .behaviour_mut()
                        .sync_protocol
                        .send_response(channel, response)
                    {
                        // Err payload is the whole unsent response — summarize, never Debug-dump it.
                        warn!(peer = %peer, response = %e.summary(), "Failed to send sync response");
                    }
                }
                request_response::Message::Response {
                    request_id,
                    response,
                } => {
                    crate::metrics::inc_sync_request_outcome(
                        "ok",
                        self.peer_trust_cache.peer_class_for(&peer),
                    );
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
                // Task #9's surface: persistent per-peer sync-request failures used
                // to exist ONLY as this warn line, so a peer whose sync requests
                // always time out was indistinguishable from a healthy one in
                // Prometheus. The label set stays bounded (result × peer_class,
                // never a raw peer id) — peer identity stays in the log line
                // above, where cardinality is free.
                crate::metrics::inc_sync_request_outcome(
                    sync_round::outbound_failure_label(&error),
                    self.peer_trust_cache.peer_class_for(&peer),
                );
                // Drop any page cursor for the failed request so the map stays
                // bounded by genuinely in-flight requests.
                self.doc_list_cursors.lock().await.remove(&request_id);
                // Free the fetch-window slot. `Io` — the transport REFUSING to
                // carry the request, which is what `max sub-streams reached`
                // is — earns the doc one re-queue inside this round; every other
                // failure is an answer of sorts and waits for the next round.
                //
                // BIND THE REMOVAL FIRST. A `self.…lock().await.remove(..)` used
                // directly as an `if let` SCRUTINEE keeps its `MutexGuard` alive
                // for the whole body (edition 2021 temporary scope), and the
                // body's `settle_sync_fetch` -> `pump_sync_fetch_window` re-takes
                // THIS mutex to record the next request — a non-reentrant
                // `tokio::sync::Mutex`, so the swarm event loop would deadlock
                // outright the moment a round has more divergent docs than the
                // window, which is the only case this change exists for. Same
                // bind-then-match idiom as the `doc_list_cursors` reclaim above.
                let settled = self.sync_fetch_inflight.lock().await.remove(&request_id);
                if let Some((fetch_peer, fetch_doc_id, fetch_h_app_id)) = settled {
                    let outcome = match &error {
                        request_response::OutboundFailure::Io(_) => sync_round::SettleOutcome::Io,
                        _ => sync_round::SettleOutcome::Other,
                    };
                    self.settle_sync_fetch(fetch_peer, &fetch_doc_id, &fetch_h_app_id, outcome)
                        .await;
                }
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
                        // Err payload is the whole unsent response — summarize, never Debug-dump it.
                        warn!(peer = %peer, response = %e.summary(), "Failed to send EPR response");
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
                // Mirror every sibling pending_* map: free the leaked entry on
                // transport failure (this arm previously only logged it), and
                // resolve the caller's oneshot with `None` so it fails fast
                // instead of waiting out its 5s timeout. Without this, every
                // failed peer EPR resolve leaked a (CID, Sender) entry for the
                // process lifetime.
                if let Some((_, reply)) = self.pending_epr_resolves.lock().await.remove(&request_id)
                {
                    let _ = reply.send(None);
                }
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
                        // Err payload is the whole unsent response — summarize, never Debug-dump it.
                        warn!(peer = %peer, response = %e.summary(), "Failed to send EPR atom response");
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

            // F-T19: View federation outbound response/failure dispatch.
            // Message::Response — resolve the pending oneshot with Ok(response).
            // OutboundFailure — map to FederationError and resolve with Err.
            // Message::Request (inbound) — responder side, lands in F-T20; log for now.
            behaviour::ElohimStorageBehaviourEvent::ViewFederation(
                request_response::Event::Message { peer, message },
            ) => match message {
                request_response::Message::Response {
                    request_id,
                    response,
                } => {
                    crate::metrics::inc_view_federation_outbound("ok");
                    if let Some(respond) = self
                        .pending_view_federations
                        .lock()
                        .await
                        .remove(&request_id)
                    {
                        debug!(
                            target: "elohim_storage::view_federation",
                            peer = %peer,
                            request_id = ?request_id,
                            "F-T19: view-federation response received; resolving oneshot"
                        );
                        let _ = respond.send(Ok(response));
                    } else {
                        debug!(
                            target: "elohim_storage::view_federation",
                            peer = %peer,
                            request_id = ?request_id,
                            "F-T19: view-federation response with no pending entry (already resolved or unknown)"
                        );
                    }
                }
                request_response::Message::Request {
                    request, channel, ..
                } => {
                    // F-T20: real responder — build and sign a ViewFederationResponse,
                    // then send_response.
                    //
                    // OFF-LOOP (F-A(ii), 2026-08-01). This used to `.await` the whole
                    // slice-build INLINE in the event-loop arm. The old comment reasoned
                    // about the swarm write LOCK, which was never the hazard: the loop
                    // body itself awaits `handle_event` to completion before it re-polls
                    // the swarm, so ANY await here halts every other swarm event —
                    // inbound requests, outbound response delivery, gossip, identify,
                    // kad. `ContentHeadRecord` makes that fatal because its slice-build
                    // asks the conductor, and `HcClient::call_zome` had no timeout of its
                    // own: one inbound head-record ask wedged the whole loop for up to the
                    // conductor's ~60s websocket deadline, which is longer than every
                    // requester's budget. Fleet-wide, that meant adopt-before-author — the
                    // ONLY path that converges a peer-divergent content head — never
                    // completed a single fetch, while the resulting timeouts also starved
                    // the very inventory asks the next sweep depends on.
                    //
                    // Same cure the ConnectionEstablished arm already applies (see the
                    // `TriggerCustodyReconcile` `try_send` above): get the work off the
                    // event-loop task. EVERY view kind goes through the spawned path —
                    // not just the conductor-touching one — so a future view kind that
                    // grows a conductor call cannot silently re-create the block.
                    //
                    // `Swarm` is not `Sync`, so the task cannot hold the swarm to answer
                    // with: it routes the finished slice back through
                    // `P2PCommand::SendViewFederationResponse`, and the command loop —
                    // which already owns `&mut Swarm` — performs the synchronous
                    // `send_response`. The only swarm touch left on THIS task is the
                    // short peer-set read below.
                    use crate::p2p::view_federation::build_response_slice;
                    debug!(
                        target: "elohim_storage::view_federation",
                        peer = %peer,
                        view_kind = ?request.view_kind,
                        agent_cid = %request.agent_cid,
                        request_id = %request.request_id,
                        "F-T20: received view-federation request"
                    );
                    let identity = self.identity.clone();
                    let db_pool = self.db_pool.clone();
                    let hc_registry = self.hc_registry.clone();
                    let command_tx = self.command_tx.clone();
                    // Read the peer set HERE, not in the task: `Swarm` is not
                    // `Sync`, so `Arc<RwLock<Swarm>>` is not `Send` and cannot
                    // cross a `tokio::spawn`. This read is a short lock hold on
                    // already-in-memory state — it is not the thing that wedged
                    // the loop; the conductor call in the slice-build was.
                    let connected_peers: Vec<libp2p::PeerId> = {
                        let swarm = self.swarm.read().await;
                        swarm.connected_peers().cloned().collect()
                    };
                    tokio::spawn(async move {
                        // Fan-out cap. Moving the slice-build off the event loop
                        // removed the accidental serialization that used to bound
                        // conductor load — under a reconnect storm every peer's
                        // first ask would otherwise become a concurrent
                        // `get_record_for_action`. Acquiring INSIDE the spawn is
                        // load-bearing: waiting for a permit on the event-loop task
                        // would re-create exactly the block this change removes.
                        // Queued (never shed): the requester's own deadline is the
                        // shed, and a permit frees within the 5s responder budget.
                        let _permit = match VIEW_FEDERATION_INBOUND_LIMIT.acquire().await {
                            Ok(p) => p,
                            // Only on close, which never happens for a 'static
                            // semaphore — drop the channel (honest degrade).
                            Err(_) => return,
                        };
                        let local_agent_cid = identity.agent_pubkey().to_string();
                        let local_peer_id = identity.peer_id_string();
                        let inventory_offset = request.inventory_offset;
                        let head_corpus_digest = request.head_corpus_digest;
                        match build_response_slice(
                            request.view_kind,
                            crate::p2p::view_federation::SliceContext {
                                agent_cid: request.agent_cid,
                                request_id: request.request_id,
                                local_agent_cid: &local_agent_cid,
                                local_peer_id,
                                connected_peers: &connected_peers,
                                keypair: identity.keypair(),
                                pool: db_pool.as_ref(),
                                inventory_offset,
                                head_corpus_digest,
                                hc_registry: hc_registry.as_deref(),
                            },
                        )
                        .await
                        {
                            Ok(response) => {
                                // Route the finished slice back to the command
                                // loop, which owns the `&mut Swarm` needed for the
                                // synchronous `send_response` (see
                                // `P2PCommand::SendViewFederationResponse`).
                                if command_tx
                                    .send(P2PCommand::SendViewFederationResponse {
                                        peer,
                                        channel,
                                        response: Box::new(response),
                                    })
                                    .await
                                    .is_err()
                                {
                                    warn!(
                                        target: "elohim_storage::view_federation",
                                        peer = %peer,
                                        "F-T20: command channel closed; dropping view-federation response"
                                    );
                                }
                            }
                            Err(e) => {
                                warn!(
                                    target: "elohim_storage::view_federation",
                                    peer = %peer,
                                    error = %e,
                                    "F-T20: failed to sign view-federation slice; dropping channel"
                                );
                                // Dropping `channel` without calling `send_response` causes
                                // libp2p to emit InboundFailure::ResponseOmission here and an
                                // OutboundFailure at the requester. The view-federation
                                // OutboundFailure arm (below) maps non-Timeout variants to
                                // FederationError::TransportError, which is the correct outcome.
                            }
                        }
                    });
                }
            },
            behaviour::ElohimStorageBehaviourEvent::ViewFederation(
                request_response::Event::OutboundFailure {
                    peer,
                    request_id,
                    error,
                },
            ) => {
                crate::metrics::inc_view_federation_outbound(
                    view_federation_outbound_result_label(&error),
                );
                warn!(
                    target: "elohim_storage::view_federation",
                    peer = %peer,
                    request_id = ?request_id,
                    error = ?error,
                    "F-T19: outbound view-federation request failed"
                );
                if let Some(respond) = self
                    .pending_view_federations
                    .lock()
                    .await
                    .remove(&request_id)
                {
                    use libp2p::request_response::OutboundFailure;
                    let fed_err = match &error {
                        OutboundFailure::Timeout => FederationError::Timeout,
                        OutboundFailure::ConnectionClosed
                        | OutboundFailure::DialFailure
                        | OutboundFailure::UnsupportedProtocols
                        | OutboundFailure::Io(_) => FederationError::TransportError,
                    };
                    let _ = respond.send(Err(fed_err));
                }
            }
            behaviour::ElohimStorageBehaviourEvent::ViewFederation(
                request_response::Event::InboundFailure {
                    peer,
                    request_id,
                    error,
                },
            ) => {
                debug!(
                    target: "elohim_storage::view_federation",
                    peer = %peer,
                    request_id = ?request_id,
                    error = ?error,
                    "F-T19: inbound view-federation request failed"
                );
            }
            behaviour::ElohimStorageBehaviourEvent::ViewFederation(
                request_response::Event::ResponseSent { peer, request_id },
            ) => {
                crate::metrics::inc_view_federation_inbound_served();
                debug!(
                    target: "elohim_storage::view_federation",
                    peer = %peer,
                    request_id = ?request_id,
                    "F-T19: view-federation response sent"
                );
            }

            // === Shamir share protocol (`/elohim/shamir-share/1.0.0`) ===
            //
            // T20: real responder (inbound Request) + requester-side response routing.
            //
            // Architecture note: the DHT carries authorization (recovery-approval
            // attestation); this libp2p layer carries only the share material.
            // The responder NEVER decides who is authorized — it looks up the
            // DB-projected attestation and reflects the DHT verdict.

            // ── Inbound Request (responder side) ────────────────────────────
            behaviour::ElohimStorageBehaviourEvent::ShamirShare(
                request_response::Event::Message {
                    peer,
                    message:
                        request_response::Message::Request {
                            request, channel, ..
                        },
                },
            ) => {
                use crate::db::recovery_approval_gate::{
                    check_share_authorization, ShareAuthDecision,
                };
                use crate::p2p::shamir_transport::ShamirShareResponse;

                let local_agent_cid = self.identity.agent_pubkey().to_string();

                // Authorization gate — must hold a DB connection briefly.
                // All IO happens before acquiring the swarm write lock.
                let auth_result: Result<ShareAuthDecision, crate::error::StorageError> =
                    if let Some(pool) = self.db_pool.as_ref() {
                        pool.get()
                            .map_err(|e| {
                                crate::error::StorageError::Database(format!(
                                    "shamir responder pool.get: {e}"
                                ))
                            })
                            .and_then(|mut conn| {
                                let now_iso =
                                    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
                                check_share_authorization(
                                    &mut conn,
                                    &request.recovery_governance_action_cid,
                                    &request.custodian_cid,
                                    &local_agent_cid,
                                    &now_iso,
                                )
                            })
                    } else {
                        // No DB pool — cannot authorize. Reject.
                        Ok(ShareAuthDecision::NotAuthorized {
                            reason: "no db pool available on responder node".to_string(),
                        })
                    };

                let response: ShamirShareResponse = match auth_result {
                    Ok(ShareAuthDecision::WrongCustodian) => {
                        // Silent drop: request directed at wrong custodian. Do NOT
                        // send an error envelope (prevents custodian enumeration).
                        debug!(
                            target: "elohim_storage::shamir_share",
                            peer = %peer,
                            requested_custodian = %request.custodian_cid,
                            local_agent = %local_agent_cid,
                            "T20: shamir-share request directed at wrong custodian — silently dropped"
                        );
                        // Drop the channel without responding — libp2p emits
                        // InboundFailure::ResponseOmission here and OutboundFailure
                        // at the requester, which the requester routes as dial failure.
                        drop(channel);
                        return;
                    }

                    Ok(ShareAuthDecision::NotAuthorized { reason }) => {
                        // Auditable security event: authorization failed.
                        info!(
                            target: "elohim_storage::shamir_share",
                            peer = %peer,
                            recovery_governance_action_cid = %request.recovery_governance_action_cid,
                            reason = %reason,
                            "T20: shamir-share authorization DENIED — sending error envelope"
                        );
                        ShamirShareResponse::error(reason)
                    }

                    Ok(ShareAuthDecision::Authorized {
                        ref attestation_cid,
                    }) => {
                        // Auditable security event: authorization passed.
                        info!(
                            target: "elohim_storage::shamir_share",
                            peer = %peer,
                            recovery_governance_action_cid = %request.recovery_governance_action_cid,
                            attestation_cid = %attestation_cid,
                            "T21: shamir-share authorization GRANTED — loading share from store"
                        );

                        // T21: load custodian share from the share store.
                        //
                        // The signing key is this node's libp2p keypair (the same
                        // key the custodian used to author the recovery-approval
                        // attestation on the DHT). The canonical signed message is:
                        //
                        //   recovery_governance_action_cid.as_bytes()
                        //   ‖ share_data
                        //   ‖ share_index.to_le_bytes()
                        //
                        // Consistent with `verify_share_response` in shamir_transport.rs.
                        let share_result: Result<ShamirShareResponse, String> = if let Some(pool) =
                            self.db_pool.as_ref()
                        {
                            pool.get()
                                    .map_err(|e| format!("T21: pool.get for share-store: {e}"))
                                    .and_then(|mut conn| {
                                        crate::db::custodian_shares::get_share_for_governance_action(
                                            &mut conn,
                                            &local_agent_cid,
                                            &request.recovery_governance_action_cid,
                                        )
                                        .map_err(|e| {
                                            format!("T21: share-store query error: {e}")
                                        })
                                    })
                                    .and_then(|maybe_share| {
                                        match maybe_share {
                                            None => {
                                                info!(
                                                    target: "elohim_storage::shamir_share",
                                                    custodian_cid = %local_agent_cid,
                                                    recovery_governance_action_cid =
                                                        %request.recovery_governance_action_cid,
                                                    "T21: custodian has no share for this governance action"
                                                );
                                                Ok(ShamirShareResponse::error(
                                                    "custodian has no share for this governance action"
                                                        .to_string(),
                                                ))
                                            }
                                            Some(share) => {
                                                // Build canonical message and sign.
                                                let mut message = Vec::with_capacity(
                                                    request.recovery_governance_action_cid.len()
                                                        + share.share_data.len()
                                                        + 4,
                                                );
                                                message.extend_from_slice(
                                                    request.recovery_governance_action_cid.as_bytes(),
                                                );
                                                message.extend_from_slice(&share.share_data);
                                                message.extend_from_slice(
                                                    &(share.share_index as u32).to_le_bytes(),
                                                );

                                                let signature = self
                                                    .identity
                                                    .keypair()
                                                    .sign(&message)
                                                    .map_err(|e| {
                                                        format!("T21: signing error: {e}")
                                                    })?;

                                                info!(
                                                    target: "elohim_storage::shamir_share",
                                                    peer = %peer,
                                                    share_index = %share.share_index,
                                                    governance_action_cid =
                                                        %request.recovery_governance_action_cid,
                                                    "T21: share loaded and signed — responding to requester"
                                                );

                                                Ok(ShamirShareResponse {
                                                    share_data: share.share_data,
                                                    share_index: share.share_index as u32,
                                                    attestation_cid: attestation_cid.clone(),
                                                    signature,
                                                    error_reason: None,
                                                })
                                            }
                                        }
                                    })
                        } else {
                            Err("T21: no DB pool on responder node".to_string())
                        };

                        match share_result {
                            Ok(resp) => resp,
                            Err(e) => {
                                tracing::error!(
                                    target: "elohim_storage::shamir_share",
                                    peer = %peer,
                                    error = %e,
                                    "T21: share-store or signing error — sending error envelope"
                                );
                                ShamirShareResponse::error(format!("share-store error: {e}"))
                            }
                        }
                    }

                    Err(e) => {
                        tracing::error!(
                            target: "elohim_storage::shamir_share",
                            peer = %peer,
                            error = %e,
                            "T20: shamir-share authorization DB error — sending error envelope"
                        );
                        ShamirShareResponse::error(format!("authorization db error: {e}"))
                    }
                };

                // Send the response (error-envelope or share payload).
                let mut swarm = self.swarm.write().await;
                if let Err(_returned_response) = swarm
                    .behaviour_mut()
                    .shamir_share
                    .send_response(channel, response)
                {
                    debug!(
                        target: "elohim_storage::shamir_share",
                        peer = %peer,
                        "T20: failed to send shamir-share response (peer disconnected?)"
                    );
                }
            }

            // ── Inbound Response (requester side) ──────────────────────────
            behaviour::ElohimStorageBehaviourEvent::ShamirShare(
                request_response::Event::Message {
                    peer,
                    message:
                        request_response::Message::Response {
                            request_id,
                            response,
                        },
                },
            ) => {
                debug!(
                    target: "elohim_storage::shamir_share",
                    peer = %peer,
                    request_id = ?request_id,
                    is_error = %response.is_error(),
                    "T20: shamir-share response received"
                );
                if let Some(respond) = self.pending_shamir_shares.lock().await.remove(&request_id) {
                    let _ = respond.send(Ok(response));
                } else {
                    debug!(
                        target: "elohim_storage::shamir_share",
                        peer = %peer,
                        request_id = ?request_id,
                        "T20: shamir-share response with no pending entry (already resolved or timed out)"
                    );
                }
            }

            // ── Outbound failure (requester side) ─────────────────────────
            behaviour::ElohimStorageBehaviourEvent::ShamirShare(
                request_response::Event::OutboundFailure {
                    peer,
                    request_id,
                    error,
                },
            ) => {
                info!(
                    target: "elohim_storage::shamir_share",
                    peer = %peer,
                    request_id = ?request_id,
                    error = ?error,
                    "T20: shamir-share outbound failure — custodian unreachable or timed out"
                );
                if let Some(respond) = self.pending_shamir_shares.lock().await.remove(&request_id) {
                    use libp2p::request_response::OutboundFailure;
                    let reason = match &error {
                        OutboundFailure::Timeout => "timeout".to_string(),
                        OutboundFailure::ConnectionClosed => "connection_closed".to_string(),
                        OutboundFailure::DialFailure => "dial_failure".to_string(),
                        OutboundFailure::UnsupportedProtocols => {
                            "unsupported_protocols".to_string()
                        }
                        OutboundFailure::Io(e) => format!("io_error: {e}"),
                    };
                    let _ = respond.send(Err(reason));
                }
            }

            // ── Inbound failure (responder side, protocol-level) ──────────
            behaviour::ElohimStorageBehaviourEvent::ShamirShare(
                request_response::Event::InboundFailure {
                    peer,
                    request_id,
                    error,
                },
            ) => {
                debug!(
                    target: "elohim_storage::shamir_share",
                    peer = %peer,
                    request_id = ?request_id,
                    error = ?error,
                    "T20: shamir-share inbound failure (protocol-level — peer closed connection?)"
                );
            }

            // ── ResponseSent (responder-side acknowledgment) ──────────────
            behaviour::ElohimStorageBehaviourEvent::ShamirShare(
                request_response::Event::ResponseSent { peer, request_id },
            ) => {
                debug!(
                    target: "elohim_storage::shamir_share",
                    peer = %peer,
                    request_id = ?request_id,
                    "T20: shamir-share response sent"
                );
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
                let http_port = parse_http_port_from_agent_version(&info.agent_version)
                    .unwrap_or(DEFAULT_HTTP_PORT);
                let capabilities = parse_capabilities_from_agent_version(&info.agent_version);
                // Cache identify info for /p2p/peers endpoint
                self.identify_cache.insert(
                    peer_id.to_string(),
                    CachedIdentifyInfo {
                        agent_version: info.agent_version.clone(),
                        protocols: info.protocols.iter().map(|p| p.to_string()).collect(),
                        listen_addrs: info.listen_addrs.iter().map(|a| a.to_string()).collect(),
                        http_port,
                        capabilities: capabilities.clone(),
                    },
                );
                // mDNS discovery (which populates `delivery_peers`) can fire
                // before this Identify handshake completes and default the
                // row to `DEFAULT_HTTP_PORT` / the discovery-floor
                // capabilities — refresh both now that the peer has declared
                // its real port and what it can serve.
                if let Some(mut p) = self.delivery_peers.get_mut(&peer_id.to_string()) {
                    p.http_port = http_port;
                    if let Some(ref caps) = capabilities {
                        p.apply_capabilities(caps);
                    }
                }
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
                peer,
                connection,
                result: Err(failure),
            }) => {
                // libp2p ≥0.43 no longer closes a connection whose ping
                // fails — it only reports it. Left alone, a peer that has
                // stopped answering (SIGSTOP'd, wedged, or gone dark without
                // a FIN) stays in `connected_peers()` until the 300 s idle
                // timeout, so `/p2p/status` keeps reporting a full floor and
                // the custody fallback keeps racing a peer that cannot
                // answer. Close it; the LAN re-dial arm and the T24 bootstrap
                // retry bring the link back once the peer answers again.
                // `Unsupported` is a protocol mismatch, not silence — leave
                // those connections alone.
                if matches!(failure, ping::Failure::Unsupported) {
                    debug!(peer = %peer, "Ping unsupported by peer; connection left open");
                } else {
                    let mut swarm = self.swarm.write().await;
                    let closed = swarm.close_connection(connection);
                    drop(swarm);
                    info!(
                        peer = %peer,
                        failure = %failure,
                        closed,
                        "Ping failed; closing the connection so the connected set reflects liveness"
                    );
                }
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

            behaviour::ElohimStorageBehaviourEvent::Gossipsub(event) => {
                use libp2p::gossipsub::Event as GossipsubEvent;
                match event {
                    GossipsubEvent::Message {
                        propagation_source,
                        message_id,
                        message,
                    } => {
                        // Transport-neutral dispatch (shared with the iroh
                        // receive loop). libp2p passes `self` as the
                        // `InventoryFetch` hook so the commitment-driven active
                        // fetch + delta-gap snapshot-request stay byte-identical.
                        let source = crate::p2p::gossip_dispatch::GossipSource::Libp2p {
                            peer: propagation_source.to_string(),
                            message_id: format!("{message_id:?}"),
                        };
                        let self_iroh_node_id = self.status_tx.borrow().iroh_node_id.clone();
                        let ctx = crate::p2p::gossip_dispatch::GossipDispatchCtx {
                            db_pool: self.db_pool.as_ref(),
                            dedup: &self.dedup,
                            agent_info_inbound_tx: self.agent_info_inbound_tx.as_ref(),
                            inventory_fetch: Some(self),
                            transport_manifest_sink: self.transport_manifest_sink.as_deref().map(
                                |s| s as &dyn crate::p2p::gossip_dispatch::TransportManifestSink,
                            ),
                            self_iroh_node_id: self_iroh_node_id.as_deref(),
                        };
                        crate::p2p::gossip_dispatch::handle_gossip(
                            &ctx,
                            message.topic.as_str(),
                            &message.data,
                            &source,
                        );
                    }
                    other => debug!(event = ?other, "Gossipsub event"),
                }
            }
        }
    }

    /// Handle an incoming shard request
    /// Snapshot the shard-relevant deps into a transport-neutral
    /// [`crate::shard_service::ShardService`]. Cheap (clones an
    /// `Arc<BlobStore>` and an `Option<DbPool>`); construct per-request.
    pub(crate) fn shard_service(&self) -> crate::shard_service::ShardService {
        crate::shard_service::ShardService::new(self.blob_store.clone(), self.db_pool.clone())
    }

    /// Handle an incoming shard request from a peer.
    ///
    /// Delegates to the transport-neutral
    /// [`crate::shard_service::ShardService`] so the iroh-side
    /// [`crate::p2p_iroh::ShardBackend`] can produce wire-byte-identical
    /// responses.
    async fn handle_shard_request(&self, request: ShardRequest) -> ShardResponse {
        self.shard_service().handle(request).await
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

    /// Handle an incoming sync request.
    ///
    /// `peer` is the sender: the `AnnounceChange` arm needs to be able to answer
    /// a metadata-only doorbell by opening a pull BACK to the announcer, which is
    /// the only way an oversized change propagates eagerly.
    async fn handle_sync_request(&self, peer: PeerId, request: SyncRequest) -> SyncResponse {
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
                change_hash,
                change_data,
            } => {
                debug!(h_app_id = %h_app_id, doc_id = %doc_id, "Handling AnnounceChange request");
                // An oversized payload is refused WITHOUT applying it (the sender
                // bounds its own fan-out; a peer that ignores the bound must not
                // be able to make us allocate past it) and degrades to the same
                // pull a metadata-only doorbell takes.
                let data = change_data.filter(|d| {
                    let ok = d.len() <= sync_round::MAX_ANNOUNCE_PAYLOAD_BYTES;
                    if !ok {
                        warn!(
                            peer = %peer, h_app_id = %h_app_id, doc_id = %doc_id, bytes = d.len(),
                            bound = sync_round::MAX_ANNOUNCE_PAYLOAD_BYTES,
                            "Announced change exceeds the payload bound — refusing the push, pulling instead"
                        );
                    }
                    ok
                });
                if let Some(data) = data {
                    // Same apply path as a PULLED change (`SyncResponse::Changes`
                    // below) — an eager push is never validated less than a pull.
                    match self
                        .sync_manager
                        .apply_changes(&h_app_id, &doc_id, vec![data])
                        .await
                    {
                        Ok(_) => {
                            // DID IT LAND? A single change whose dependencies we
                            // lack is QUEUED by Automerge, not applied — the doc
                            // is untouched and `apply_changes` still returns Ok.
                            // That is the first-contact case (we hold no history
                            // for this doc), and treating it as converged is how
                            // an eager push turns into silent data loss. Ask the
                            // doc whether it now holds the announced change; if
                            // not, fall back to the pull, which carries the deps.
                            let landed = self
                                .sync_manager
                                .get_change_by_hash(&h_app_id, &doc_id, &change_hash)
                                .await
                                .unwrap_or(None)
                                .is_some();
                            if !landed {
                                debug!(
                                    peer = %peer, h_app_id = %h_app_id, doc_id = %doc_id,
                                    "Announced change is missing its dependencies here — pulling the doc instead"
                                );
                                self.pull_announced_doc(peer, &h_app_id, &doc_id).await;
                                return SyncResponse::ChangeAck {
                                    h_app_id,
                                    doc_id,
                                    was_new: false,
                                };
                            }
                            info!(h_app_id = %h_app_id, doc_id = %doc_id, "Applied announced change");
                            // CRDT-heal: reverse-project a converged content doc into
                            // the local SQL row (amber tier). Before moving the strings
                            // into the response.
                            self.heal_content_row(&h_app_id, &doc_id).await;
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
                    // A doorbell with no bytes. Ring it for real: open the SAME
                    // per-doc pull the round uses for a diverged document, at this
                    // one doc, back at the announcer. Without this the ack was the
                    // whole story and the change waited for the next 60s round.
                    self.pull_announced_doc(peer, &h_app_id, &doc_id).await;
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
            SyncRequest::ListDocumentsSince {
                h_app_id,
                prefix,
                corpus_digest,
                limit,
            } => {
                // Digest-first opener. Equal digests ⇒ answer with one hash and
                // enumerate NOTHING; different ⇒ fall through to exactly the
                // ListDocuments behaviour above, so a divergent pair is no worse
                // off than before. Same page window the opener itself uses
                // (offset 0, SYNC_LIST_PAGE_LIMIT) or the two sides would digest
                // different slices and never agree.
                match self
                    .sync_manager
                    .list_documents(&h_app_id, prefix.as_deref(), 0, SYNC_LIST_PAGE_LIMIT)
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
                            debug!(
                                h_app_id = %h_app_id,
                                digest = %ours,
                                docs = local.len(),
                                "ListDocumentsSince: digests match — answering InSync, enumerating nothing"
                            );
                            SyncResponse::InSync {
                                h_app_id,
                                corpus_digest: ours,
                            }
                        } else {
                            debug!(
                                h_app_id = %h_app_id,
                                ours = %ours,
                                theirs = %corpus_digest,
                                "ListDocumentsSince: digests differ — falling back to full enumeration"
                            );
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
                    Err(e) => {
                        warn!(h_app_id = %h_app_id, error = %e, "Failed to list documents for digest compare");
                        SyncResponse::Error {
                            message: format!("Failed to list documents: {}", e),
                        }
                    }
                }
            }
        }
    }

    /// Issue queued `SyncChanges` requests for `peer` until its window is full.
    ///
    /// The ONLY place a windowed `SyncChanges` request is sent. `have_heads` is
    /// read here, at issue time, rather than carried from enumeration: a
    /// document that converged while it waited (the doorbell delivered it, an
    /// earlier page's changes landed) asks from where it actually is instead of
    /// re-fetching from a stale position.
    ///
    /// Never holds the window lock across an await — the swarm write and the
    /// DocStore read both happen with it released.
    async fn pump_sync_fetch_window(&self, peer: PeerId, h_app_id: &str) {
        loop {
            let next = {
                let mut windows = self.sync_fetch_windows.lock().await;
                match windows.get_mut(&peer) {
                    Some(window) => window.next_doc(),
                    None => None,
                }
            };
            let Some(doc_id) = next else {
                break;
            };
            // Err (document absent locally) opens with empty heads — a full
            // sync, exactly the pre-window behaviour for a missing doc.
            let have_heads = self
                .sync_manager
                .get_heads(h_app_id, &doc_id)
                .await
                .unwrap_or_default();
            let sync_request = SyncRequest::SyncChanges {
                h_app_id: h_app_id.to_string(),
                doc_id: doc_id.clone(),
                have_heads,
                bloom_filter: None,
            };
            let req_id = {
                let mut swarm = self.swarm.write().await;
                swarm
                    .behaviour_mut()
                    .sync_protocol
                    .send_request(&peer, sync_request)
            };
            self.sync_fetch_inflight
                .lock()
                .await
                .insert(req_id, (peer, doc_id.clone(), h_app_id.to_string()));
            crate::metrics::inc_sync_request("sync_changes");
            debug!(
                peer = %peer, doc_id = %doc_id, request_id = ?req_id,
                "Requested changes for diverged document (windowed)"
            );
        }
    }

    /// Free the window slot an issued `SyncChanges` request held, then re-fill it.
    ///
    /// Called from BOTH terminal paths — every response shape and every
    /// `OutboundFailure` — because a slot that is never freed is a window that
    /// closes for the rest of the round, which would be strictly worse than the
    /// unbounded fan-out it replaces.
    async fn settle_sync_fetch(
        &self,
        peer: PeerId,
        doc_id: &str,
        h_app_id: &str,
        outcome: sync_round::SettleOutcome,
    ) {
        // Count what the metric NAMES: an actual re-queue. An `Io` settle does
        // not always produce one — the doc may have spent its single per-round
        // re-queue already, or the window may have been retired at a round
        // boundary (no entry, nothing settled). Incrementing on the OUTCOME
        // instead of on the effect published io-settles under a re-queue name.
        let requeued = {
            let mut windows = self.sync_fetch_windows.lock().await;
            match windows.get_mut(&peer) {
                Some(window) => window.settle(doc_id, outcome),
                None => false,
            }
        };
        if requeued {
            crate::metrics::inc_sync_fetch_window_requeued();
        }
        self.pump_sync_fetch_window(peer, h_app_id).await;
    }

    /// Handle an outbound sync response from a peer.
    /// Called when we receive responses to our sync requests (e.g., from initiate_sync_round).
    async fn handle_sync_response(
        &self,
        peer: PeerId,
        request_id: request_response::OutboundRequestId,
        response: SyncResponse,
    ) {
        // Reclaim any page cursor for this request UP FRONT: every response
        // shape (DocumentList, Error, NotFound, …) must clear it, or a peer
        // answering ListDocuments with an Error would strand entries in the
        // cursor map. (OutboundFailure has its own removal in the event arm.)
        let page_offset = self.doc_list_cursors.lock().await.remove(&request_id);
        // Free this request's fetch-window slot BEFORE the match: several arms
        // return early (an empty `Changes` page is the common one), and a slot
        // leaked on an early return would close the window for the rest of the
        // round. An `Error` body is a real answer, just not a useful one — it
        // settles as `Other`, which frees the slot without a retry.
        //
        // The removal is BOUND, never used as the `if let` scrutinee directly:
        // a scrutinee temporary holds its `MutexGuard` across the whole body,
        // and the body re-takes this same non-reentrant mutex through
        // `settle_sync_fetch` -> `pump_sync_fetch_window`. That is a swarm-loop
        // self-deadlock, not a slow path. (Same idiom as `page_offset` above.)
        let settled = self.sync_fetch_inflight.lock().await.remove(&request_id);
        if let Some((fetch_peer, fetch_doc_id, fetch_h_app_id)) = settled {
            let outcome = match &response {
                SyncResponse::Error { .. } => sync_round::SettleOutcome::Other,
                _ => sync_round::SettleOutcome::Ok,
            };
            self.settle_sync_fetch(fetch_peer, &fetch_doc_id, &fetch_h_app_id, outcome)
                .await;
        }
        match response {
            SyncResponse::DocumentList {
                h_app_id,
                documents,
                total,
                has_more,
            } => {
                debug!(
                    peer = %peer, h_app_id = %h_app_id, doc_count = documents.len(),
                    total = total, has_more = has_more, "Received document list from peer"
                );
                // The round's enumeration cost, made visible: on a converged mesh
                // this should trend to zero, and today it tracks corpus size x
                // peers forever (spine node `sync-scale-honesty`).
                crate::metrics::add_sync_docs_enumerated(documents.len() as u64);

                // Follow the page cursor BEFORE processing this page: without
                // this, every doc past the first page sat outside every sync
                // round forever (silent tail — the corpus back-fill projects
                // the whole content table, so one page overflows easily).
                match page_offset {
                    Some(offset) => {
                        if let Some(next_offset) = crate::p2p::sync_protocol::next_doc_list_offset(
                            offset,
                            documents.len(),
                            has_more,
                        ) {
                            let next_request = SyncRequest::ListDocuments {
                                h_app_id: h_app_id.clone(),
                                prefix: None,
                                offset: next_offset,
                                limit: SYNC_LIST_PAGE_LIMIT,
                            };
                            let mut swarm = self.swarm.write().await;
                            let next_id = swarm
                                .behaviour_mut()
                                .sync_protocol
                                .send_request(&peer, next_request);
                            drop(swarm);
                            self.doc_list_cursors
                                .lock()
                                .await
                                .insert(next_id, next_offset);
                            crate::metrics::inc_sync_request("list_documents");
                            debug!(
                                peer = %peer, offset = next_offset, request_id = ?next_id,
                                "Requested next document-list page"
                            );
                        }
                    }
                    None if has_more => {
                        // Untracked page with more remaining (e.g. a restart
                        // raced the response). The next 60s round re-enumerates
                        // from offset 0 — log so a stuck tail is diagnosable.
                        debug!(
                            peer = %peer, total = total,
                            "DocumentList has_more without a tracked page cursor — deferring to next sync round"
                        );
                    }
                    None => {}
                }

                // Compare with local documents and QUEUE the diverged ones. This
                // arm used to send every one of them here, in one pass: on the
                // two-peer recovery harness (504 docs, one wiped DocStore) that
                // was ~500 `SyncChanges` requests onto a single connection, of
                // which yamux refused 196 with `Io(max sub-streams reached)` and
                // nothing retried — three 60s rounds to refill (181s) against
                // iroh's one (63s, one request in flight). The window issues at
                // most `sync_fetch_window` at a time and re-fills as each
                // settles (`handle_sync_response` / the `OutboundFailure` arm).
                //
                // A doc missing locally (`get_heads` Err) opens with empty
                // `have_heads`, exactly as before — the heads are read at ISSUE
                // time in `pump_sync_fetch_window`, so a doc that converged
                // while it waited in the queue asks from where it actually is.
                let mut divergent: Vec<String> = Vec::new();
                for remote_doc in &documents {
                    let diverged = match self
                        .sync_manager
                        .get_heads(&h_app_id, &remote_doc.doc_id)
                        .await
                    {
                        Ok(local_heads) => local_heads != remote_doc.heads,
                        // Document doesn't exist locally — request full sync.
                        Err(_) => true,
                    };
                    if diverged {
                        divergent.push(remote_doc.doc_id.clone());
                    }
                }
                if !divergent.is_empty() {
                    let window_size = sync_round::fetch_window(self.config.sync_fetch_window);
                    {
                        let mut windows = self.sync_fetch_windows.lock().await;
                        let window = windows
                            .entry(peer)
                            .or_insert_with(|| sync_round::FetchWindow::new(window_size));
                        for doc_id in &divergent {
                            window.enqueue(doc_id.clone());
                        }
                    }
                    let divergent = divergent.len();
                    debug!(
                        peer = %peer, h_app_id = %h_app_id, divergent = divergent,
                        "Queued diverged documents into the bounded fetch window"
                    );
                    self.pump_sync_fetch_window(peer, &h_app_id).await;
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
                    // CRDT-heal: reverse-project a converged content doc into the
                    // local SQL row (amber tier) now that the changes landed.
                    self.heal_content_row(&h_app_id, &doc_id).await;
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
                        crate::metrics::inc_sync_request("sync_changes");
                        debug!(peer = %peer, doc_id = %doc_id, "Heads differ, requesting changes");
                    }
                    _ => {
                        debug!(peer = %peer, doc_id = %doc_id, "Heads match, in sync");
                    }
                }
            }
            SyncResponse::ChangeAck {
                h_app_id,
                doc_id,
                was_new,
            } => {
                // The doorbell's answer. `was_new: true` means the peer applied
                // the bytes we pushed — propagation completed in one round-trip
                // instead of waiting for its next 60s round. `false` means the
                // peer already had it, or the announce carried no bytes and the
                // peer is pulling instead; either way nothing is owed here. This
                // arm exists so an eager push is TYPED-handled rather than
                // reading as "Unhandled sync response type" in the catch-all,
                // which is how an inert push path hides.
                debug!(
                    peer = %peer, h_app_id = %h_app_id, doc_id = %doc_id, was_new = was_new,
                    "Announced change acknowledged by peer"
                );
            }
            SyncResponse::Error { message } => {
                warn!(peer = %peer, request_id = ?request_id, error = %message, "Sync error from peer");
            }
            SyncResponse::InSync {
                h_app_id,
                corpus_digest,
            } => {
                // Digest-equal steady state: the peer enumerated nothing and there is
                // nothing to apply. This arm exists so the shortcut is INTENTIONAL and
                // counted — before it, InSync fell into the catch-all and read as
                // "Unhandled sync response type", which is how an inert optimisation
                // hides. The page cursor was already reclaimed at the top of this fn.
                crate::metrics::inc_sync_in_sync();
                debug!(
                    peer = %peer,
                    h_app_id = %h_app_id,
                    digest = %corpus_digest,
                    "Sync round InSync — peer holds the same corpus, nothing enumerated"
                );
            }
            _ => {
                debug!(peer = %peer, request_id = ?request_id, "Unhandled sync response type");
            }
        }
    }

    /// Answer a metadata-only `AnnounceChange` by pulling that ONE doc from the
    /// peer that announced it.
    ///
    /// Reuses the round's per-doc request verbatim — `SyncChanges` carrying our
    /// current heads, exactly the shape `handle_sync_response` sends for a
    /// diverged document — so the response lands in the existing
    /// `SyncResponse::Changes` arm (apply + `heal_content_row`) with no new
    /// protocol message and no second apply path.
    ///
    /// Bounded and lossy, deliberately: one request, no retry, no queue. If the
    /// peer is gone or the request fails, the 60s round is still the backstop.
    async fn pull_announced_doc(&self, peer: PeerId, h_app_id: &str, doc_id: &str) {
        let have_heads = self
            .sync_manager
            .get_heads(h_app_id, doc_id)
            .await
            .unwrap_or_default();
        let request = SyncRequest::SyncChanges {
            h_app_id: h_app_id.to_string(),
            doc_id: doc_id.to_string(),
            have_heads,
            bloom_filter: None,
        };
        let req_id = {
            let mut swarm = self.swarm.write().await;
            swarm
                .behaviour_mut()
                .sync_protocol
                .send_request(&peer, request)
        };
        crate::metrics::inc_sync_request("sync_changes");
        debug!(
            peer = %peer, h_app_id = %h_app_id, doc_id = %doc_id, request_id = ?req_id,
            "Announced change carried no bytes — pulling the doc from the announcer"
        );
    }

    /// Production CRDT-heal leg: after a content-sync doc converges into our
    /// DocStore (via `apply_changes`), re-derive the serving-critical `blob_hash`
    /// into the local SQL `content` row at the amber tier — so SERVING (which
    /// reads SQL, not the DocStore) heals WITHOUT the notary. This is the B1
    /// reverse projector wired into the live consumer; both sync-handler
    /// `apply_changes` call-sites (AnnounceChange + Changes) share this helper.
    ///
    /// Scope: only content-node docs under the "elohim" sync namespace — every
    /// other doc/namespace is a silent no-op.
    ///
    /// Loop-safety (verified): `reverse_project_content_doc` writes via
    /// `content_diesel::update_content` DIRECTLY — NOT `content_service.update` —
    /// so it emits NO `StorageEvent::ContentUpdated`. No `ContentUpdated` means
    /// `spawn_content_projection_listener` never re-projects this row back into
    /// the DocStore, so there is no heal → re-project → converge → heal loop.
    ///
    /// A `None` `db_pool` (a p2p-only node with no SQL projection) skips silently;
    /// a reverse-projection error is logged (warn) but never fails the sync round.
    async fn heal_content_row(&self, h_app_id: &str, doc_id: &str) {
        if h_app_id != crate::sync::projector::PROJECTION_NAMESPACE || !doc_id.starts_with("node:")
        {
            return;
        }
        let Some(pool) = self.db_pool.as_ref() else {
            return;
        };
        match crate::sync::projector::reverse_project_content_doc(&self.sync_manager, pool, doc_id)
            .await
        {
            Ok(true) => {
                debug!("content heal: reverse-projected {doc_id}");
                // Pointer-heal implies bytes-heal: the reverse projection just
                // wrote a blob_hash this peer may not hold. Without an eager
                // fetch, serving discovers the absence lazily PER REQUEST and
                // blocks in heal-on-read (observed live 2026-07-02: adam's
                // healed pointer referenced bytes ~35min behind; every GET /
                // hung >30s until custody replication caught up — backlog
                // heal-pointer-bytes-ordering-blocking-serve).
                self.heal_blob_bytes_if_absent(doc_id).await;
            }
            Ok(false) => {}
            Err(e) => {
                warn!(doc_id = %doc_id, error = %e, "content heal: reverse-projection failed")
            }
        }
    }

    /// Eagerly fetch the bytes behind a just-healed content pointer when the
    /// local store lacks them — the bytes-heal half of the CRDT drift-heal.
    ///
    /// Mirrors the Wave-3 commitment-fetch idiom (`score_and_enqueue_snapshot`):
    /// drop-on-saturation via `commitment_fetch_semaphore` (the next 60s sync
    /// round re-heals and retries), candidates from `peer_blob_inventory`,
    /// connected-filter at task start, `race_fetch` → `finalize_fetch_success`
    /// (which books the serve-blob REA event atomically). No score-prefix
    /// re-ordering here — a heal is a single opportunistic pull, not a
    /// commitment-scored placement; plain freshness-windowed inventory order
    /// is enough and keeps this path free of the transport-manifest seam.
    async fn heal_blob_bytes_if_absent(&self, doc_id: &str) {
        use crate::p2p::blob_fetch::finalize_fetch_success;
        use crate::p2p::blob_swarm::{race_fetch_with_swarm, SwarmFetchParams, SwarmRaceOutcome};

        let Ok(hash) = self
            .sync_manager
            .get_doc_field(
                crate::sync::projector::PROJECTION_NAMESPACE,
                doc_id,
                "blobHash",
            )
            .await
        else {
            return; // no blobHash on the doc — nothing to pull
        };
        if hash.is_empty() || self.blob_store.exists(&hash).await {
            return;
        }
        let Some(self_cid) = self.config.self_cid.clone() else {
            return;
        };
        let Some(pool) = self.db_pool.clone() else {
            return;
        };
        let permit = match self.commitment_fetch_semaphore.clone().try_acquire_owned() {
            Ok(p) => p,
            Err(_) => {
                debug!(
                    doc_id = %doc_id, hash = %hash,
                    "bytes-heal: fetch semaphore saturated — dropping; next sync round retries"
                );
                return;
            }
        };

        let cmd_tx = self.command_tx.clone();
        let blob_store = self.blob_store.clone();
        let peer_metrics = self.peer_metrics.clone();
        let parallelism = self.config.fetch_blob_parallelism.max(1);
        let timeout = std::time::Duration::from_secs(self.config.fetch_blob_timeout_seconds.max(1));
        let doc_id_owned = doc_id.to_string();
        // Q4: same gossip publisher + sequence allocator `broadcast_inventory_snapshot`
        // uses — the composite hash is never on disk under its own name (Q2), so it
        // can NEVER appear in a periodic snapshot's `blob_store.list_hashes()` walk;
        // only an explicit delta on the shared per-peer sequence can advertise it.
        let gossip_publisher = self.gossip_publisher.clone();
        let inventory_seq = self.inventory_seq.clone();
        let local_peer_id = self.peer_id().to_string();
        let fresh_after = chrono::Utc::now()
            .checked_sub_signed(chrono::Duration::seconds(
                self.config.inventory_freshness_seconds as i64,
            ))
            .unwrap_or_else(chrono::Utc::now)
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();

        tokio::spawn(async move {
            let _permit = permit; // held for task lifetime; releases on drop

            let candidates: Vec<String> = match pool.get() {
                Ok(mut c) => {
                    crate::db::peer_blob_inventory::lookup_hosts(&mut c, &hash, &fresh_after)
                        .unwrap_or_default()
                        .into_iter()
                        .map(|r| r.peer_id)
                        .collect()
                }
                Err(_) => return,
            };
            let connected_set: std::collections::HashSet<String> = peer_metrics
                .iter()
                .filter_map(|e| {
                    if e.value().is_connected {
                        Some(e.key().clone())
                    } else {
                        None
                    }
                })
                .collect();

            // Q3/Q4: this is the two-hop pull chain (acquisition GetContent →
            // this blob pull) `epr-acquisition-pull-queue-design` names as
            // the seam Q4's shard-swarm spread extends. A plain race on the
            // composite hash 404s forever when the content's bytes live only
            // as shards on connected peers with no minted composite manifest
            // cache; `race_fetch_with_swarm` accepts a manifest-only reply
            // and completes the pull by racing shards across the holder set.
            let mut conn = match pool.get() {
                Ok(c) => c,
                Err(e) => {
                    warn!(
                        doc_id = %doc_id_owned, hash = %hash, error = %e,
                        "bytes-heal/Q3: pool exhausted, cannot race-fetch"
                    );
                    return;
                }
            };
            let swarm_params = SwarmFetchParams {
                cmd_tx: &cmd_tx,
                connected: &connected_set,
                per_shard_parallelism: parallelism,
                per_peer_timeout: timeout,
                // Same modest fan-out bound as the HTTP GET-miss heal path
                // (`http.rs::get_blob_or_heal`) — see that call site's
                // comment for the `RaceFetchKicker::kick_semaphore` parity.
                total_inflight: parallelism.max(1) * 4,
                self_cid: &self_cid,
                blob_store: &blob_store,
            };
            let outcome =
                race_fetch_with_swarm(&hash, candidates, &mut conn, &fresh_after, &swarm_params)
                    .await;
            drop(conn);

            match outcome {
                SwarmRaceOutcome::Hit { bytes, source_peer } => {
                    // `source_peer == "swarm"` — Q4 reassembled the composite
                    // from independently-fetched shards, each already
                    // persisted + REA-booked to its real source peer inside
                    // `fetch_shards_via_swarm`. Finalizing again here under a
                    // synthetic "swarm" peer would double-book a delivery
                    // that never happened from a single peer.
                    if source_peer == "swarm" {
                        info!(
                            doc_id = %doc_id_owned, hash = %hash,
                            "bytes-heal/Q4: pointer-heal completed via swarm shard-fetch (serve path warm)"
                        );
                        // Q4: advertise the now-servable composite hash
                        // immediately (event-driven `BlobInventoryDelta`,
                        // reusing `inventory_broadcaster::build_delta` +
                        // the SAME gossip publish call
                        // `broadcast_inventory_snapshot` uses) so the holder
                        // set — and aggregate swarm bandwidth — compounds
                        // without waiting for the next periodic snapshot
                        // tick (which would never include it anyway; see
                        // the capture-site comment above).
                        match crate::p2p::inventory_gossip::BlobAddress::new(hash.clone()) {
                            Ok(addr) => {
                                let delta = crate::p2p::inventory_broadcaster::build_delta(
                                    &local_peer_id,
                                    vec![addr],
                                    vec![],
                                    &inventory_seq,
                                    chrono::Utc::now().timestamp_micros(),
                                    vec![],
                                );
                                match delta.to_bytes() {
                                    Ok(bytes) => {
                                        if let Err(e) = gossip_publisher.publish(
                                            crate::p2p::inventory_gossip::INVENTORY_TOPIC,
                                            bytes,
                                        ) {
                                            warn!(
                                                doc_id = %doc_id_owned, hash = %hash, error = %e,
                                                "bytes-heal/Q4: composite delta publish failed (often: no subscribed peers yet)"
                                            );
                                        } else {
                                            debug!(
                                                doc_id = %doc_id_owned, hash = %hash,
                                                "bytes-heal/Q4: advertised swarm-completed composite via inventory delta"
                                            );
                                        }
                                    }
                                    Err(e) => warn!(
                                        doc_id = %doc_id_owned, hash = %hash, error = %e,
                                        "bytes-heal/Q4: failed to serialize composite delta"
                                    ),
                                }
                            }
                            Err(e) => warn!(
                                doc_id = %doc_id_owned, hash = %hash, error = ?e,
                                "bytes-heal/Q4: composite hash is not canonical wire-shaped; skipping delta advertise"
                            ),
                        }
                        return;
                    }
                    if let Ok(mut c) = pool.get() {
                        match finalize_fetch_success(
                            &mut c,
                            &hash,
                            &source_peer,
                            &bytes,
                            &self_cid,
                            &blob_store,
                        )
                        .await
                        {
                            Ok(()) => info!(
                                doc_id = %doc_id_owned, hash = %hash, source = %source_peer,
                                "bytes-heal: pointer-heal completed with bytes (serve path warm)"
                            ),
                            Err(e) => warn!(
                                doc_id = %doc_id_owned, hash = %hash, error = %e,
                                "bytes-heal: finalize failed"
                            ),
                        }
                    }
                }
                SwarmRaceOutcome::ManifestPersistedIncomplete {
                    manifest,
                    missing_shards,
                } => debug!(
                    doc_id = %doc_id_owned, hash = %hash,
                    total_shards = manifest.shard_hashes.len(), missing_shards,
                    "bytes-heal/Q3: manifest persisted but swarm shard-fetch incomplete (next sync round retries)"
                ),
                SwarmRaceOutcome::Reconstructible {
                    landed, missing, ..
                } => {
                    info!(
                        doc_id = %doc_id_owned, hash = %hash, landed, missing,
                        "bytes-heal/Q4: parity-aware swarm reached the reconstructible floor (serve path warm)"
                    );
                    // The composite is now servable even though parity salvage
                    // remains. Advertise that fact immediately, just like the
                    // all-shards swarm completion arm above.
                    match crate::p2p::inventory_gossip::BlobAddress::new(hash.clone()) {
                        Ok(addr) => {
                            let delta = crate::p2p::inventory_broadcaster::build_delta(
                                &local_peer_id,
                                vec![addr],
                                vec![],
                                &inventory_seq,
                                chrono::Utc::now().timestamp_micros(),
                                vec![],
                            );
                            match delta.to_bytes() {
                                Ok(bytes) => {
                                    if let Err(e) = gossip_publisher.publish(
                                        crate::p2p::inventory_gossip::INVENTORY_TOPIC,
                                        bytes,
                                    ) {
                                        warn!(
                                            doc_id = %doc_id_owned, hash = %hash, error = %e,
                                            "bytes-heal/Q4: reconstructible composite delta publish failed"
                                        );
                                    }
                                }
                                Err(e) => warn!(
                                    doc_id = %doc_id_owned, hash = %hash, error = %e,
                                    "bytes-heal/Q4: failed to serialize reconstructible composite delta"
                                ),
                            }
                        }
                        Err(e) => warn!(
                            doc_id = %doc_id_owned, hash = %hash, error = ?e,
                            "bytes-heal/Q4: reconstructible composite hash is not canonical wire-shaped"
                        ),
                    }
                }
                other => debug!(
                    doc_id = %doc_id_owned, hash = %hash, outcome = ?other,
                    "bytes-heal: no bytes pulled this round (next sync round retries)"
                ),
            }
        });
    }

    /// Look up content from local DB and encode as an EPR Head (MessagePack).
    /// Returns None if content not found or DB not available.
    ///
    /// Thin delegate to [`crate::epr_service::EprService`] so the libp2p
    /// drain loop and reconcile path share the same logic the iroh-side
    /// adapter uses. See `epr_service` module docs for the dual-stack
    /// rationale.
    fn resolve_epr_head_locally(&self, id: &str) -> Option<Vec<u8>> {
        self.epr_service().resolve_epr_head_locally(id)
    }

    /// Snapshot the EPR-relevant deps into a transport-neutral
    /// [`crate::epr_service::EprService`]. Cheap (clones `Option<Arc<_>>`
    /// handles); construct per-request rather than keeping a stale field.
    /// Used by [`Self::handle_epr_request`] to delegate read-side variants
    /// and by the iroh-mode adapter to share authorization logic.
    pub(crate) fn epr_service(&self) -> crate::epr_service::EprService {
        let svc = crate::epr_service::EprService::new(
            self.db_pool.clone(),
            self.policy_enforcement.clone(),
            self.extraction_cache.clone(),
            self.peer_trust_cache.clone(),
        );
        #[cfg(feature = "graph-native")]
        let svc = svc.with_graph_engine(self.graph_engine.clone());
        // T7: same process-lifetime memo-store Arc on every per-request
        // snapshot — the store's lifetime is process-wide even though this
        // `EprService` value itself is rebuilt per call.
        svc.with_memo_store(self.memo_store.clone())
    }

    /// Snapshot the EPR-atom-relevant deps into a transport-neutral
    /// [`crate::epr_atom_service::EprAtomService`]. Cheap (clones an
    /// `Option<DbPool>` and an `Arc<DedupLru>`); construct per-request
    /// rather than keeping a stale field. Used by
    /// [`Self::handle_epr_atom_request`] to delegate and by the iroh-mode
    /// adapter to share fetch/announce/dedup logic.
    pub(crate) fn epr_atom_service(&self) -> crate::epr_atom_service::EprAtomService {
        crate::epr_atom_service::EprAtomService::new(self.db_pool.clone(), self.dedup.clone())
    }

    /// Handle an incoming EPR atom federation request from a peer.
    ///
    /// Delegates to the transport-neutral
    /// [`crate::epr_atom_service::EprAtomService`] so the iroh-side
    /// [`crate::p2p_iroh::EprAtomBackend`] can produce wire-byte-identical
    /// responses. The libp2p-specific work (resolving the caller from
    /// libp2p `PeerId` via the `PeerIdentityMap`, formatting the peer
    /// label for tracing) happens here; the service handles the
    /// rest.
    async fn handle_epr_atom_request(
        &self,
        peer: libp2p::PeerId,
        request: EprAtomRequest,
    ) -> EprAtomResponse {
        let caller = self.identity_map.lookup(&peer);

        // W2A / T22: capture (kind, cid) from Announce envelope bytes BEFORE
        // consuming `request` via EprAtomService::handle. Lightweight CBOR
        // decode — only the envelope header, no DB I/O. Returns None if the
        // bytes can't be minimally decoded; record_predecessor is best-effort
        // and we never reject an Announce on account of a back-prop failure.
        let predecessor_target: Option<(elohim_epr::EprKind, String)> =
            if let EprAtomRequest::Announce {
                envelope_bytes: ref bytes,
            } = request
            {
                ciborium::de::from_reader::<elohim_epr::Epr, _>(bytes.as_slice())
                    .ok()
                    .map(|epr| (epr.envelope.kind, epr.envelope.cid.to_string()))
            } else {
                None
            };

        let response = self
            .epr_atom_service()
            .handle(&peer.to_string(), caller, request);

        // W2A / T22: after a successful Content-kind Announce, record the
        // sender PeerId as a predecessor in `predecessor_records`.
        //
        // Constraints:
        //   - Content-kind only — FeedbackSignal and other kinds are excluded;
        //     the back-prop graph is for content provenance, not signal propagation.
        //   - Best-effort + non-fatal — DB / seal errors are logged at warn level
        //     and never bubble to the EprAtomResponse.
        //   - Idempotent — record_predecessor uses ON CONFLICT DO NOTHING (T10).
        //   - Skipped when sealing_keys is None (dev/test without key infra).
        if let EprAtomResponse::Announced { accepted: true, .. } = &response {
            if let Some((elohim_epr::EprKind::Content, ref cid)) = predecessor_target {
                if let Some(ref sealing_keys) = self.sealing_keys {
                    if let Some(ref pool) = self.db_pool {
                        match pool.get() {
                            Ok(mut conn) => {
                                let keys = crate::services::back_prop::SealingPubKeys {
                                    mishpat_pk: &sealing_keys.mishpat_pk,
                                    imagodei_pk: &sealing_keys.imagodei_pk,
                                };
                                if let Err(e) = crate::services::back_prop::record_predecessor(
                                    &mut conn,
                                    cid,
                                    &peer.to_string(),
                                    &keys,
                                ) {
                                    warn!(
                                        target: "epr::back_prop",
                                        cid = %cid,
                                        peer = %peer,
                                        error = ?e,
                                        "W2A: record_predecessor failed (best-effort, non-fatal)"
                                    );
                                }
                            }
                            Err(e) => {
                                warn!(
                                    target: "epr::back_prop",
                                    cid = %cid,
                                    peer = %peer,
                                    error = %e,
                                    "W2A: record_predecessor skipped — db pool exhausted"
                                );
                            }
                        }
                    }
                }
            }
        }

        response
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

    /// Handle an incoming EPR request from a peer.
    ///
    /// Read-side variants (Resolve, ResolveBatch, GetDocument,
    /// QueryDelivery) delegate to the transport-neutral
    /// [`crate::epr_service::EprService`] so the iroh-side
    /// [`crate::p2p_iroh::EprBackend`] can produce wire-byte-identical
    /// responses. The Announce branch stays here because it writes to
    /// libp2p Kademlia (the iroh-side equivalent — pkarr / iroh-gossip
    /// identity-binding — lands later per the complementarity spec's n0
    /// mitigation roadmap).
    async fn handle_epr_request(&self, request: EprRequest) -> EprResponse {
        match request {
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
            other => self
                .epr_service()
                .handle_read(other)
                .await
                .expect("non-Announce EPR variants always produce a response"),
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
            sync_gate: Arc::clone(&self.sync_gate),
            last_gossiped: Arc::clone(&self.last_gossiped),
            // Clone shares the Arc<RwLock<...>> inside AcquisitionState.
            acquisition: self.acquisition.clone(),
            // Self-stewardship needs a LOCAL bytes probe: a self-held
            // shard_locations row is only written after the local pantry is
            // confirmed to hold the shard. See `services::self_stewardship`.
            blob_store: Some(Arc::clone(&self.blob_store)),
            hc_lamad: self
                .hc_registry
                .as_ref()
                .and_then(|registry| registry.lamad_client()),
        }
    }

    /// Initiate a sync round with all connected peers.
    /// Sends ListDocuments requests; responses arrive as SyncProtocol events
    /// and are handled in `handle_behaviour_event`.
    async fn initiate_sync_round(&self) {
        let swarm = self.swarm.read().await;
        let peers: Vec<PeerId> = swarm.connected_peers().cloned().collect();
        drop(swarm);

        // Report, then retire, the previous round's fetch windows — BEFORE the
        // no-peers early return. A window is per-peer per-round: whatever it did
        // not reach is re-enumerated by the round starting now, so carrying it
        // forward would double-ask. Retiring below the early return would leave
        // a disconnected peer's stale window and its in-flight entries resident
        // for as long as the mesh has no peers, so the round after a reconnect
        // would open with `in_flight` already charged against a window whose
        // requests can never settle. The debug line is the round's honest cost —
        // enqueued vs issued vs re-queued vs the tail it never got to.
        {
            let mut windows = self.sync_fetch_windows.lock().await;
            for (window_peer, window) in windows.iter() {
                debug!(
                    peer = %window_peer, window = window.window(),
                    enqueued = window.enqueued(), issued = window.issued(),
                    requeued_io = window.requeued(), unreached = window.pending(),
                    in_flight = window.in_flight(),
                    "Sync fetch window: previous round"
                );
            }
            windows.clear();
        }
        self.sync_fetch_inflight.lock().await.clear();

        if peers.is_empty() {
            debug!("Sync round: no connected peers, skipping");
            return;
        }

        info!(peer_count = peers.len(), "Initiating sync round");
        crate::metrics::inc_sync_round();

        // Single source of truth for the sync-partition namespace: the producer
        // (`sync::projector::project_content_doc`) writes docs under this same
        // const. Referencing it here (not a bare literal) makes producer/consumer
        // drift compile-impossible — a silent content-sync killer otherwise (see
        // PROJECTION_NAMESPACE docs).
        let namespace = crate::sync::projector::PROJECTION_NAMESPACE;

        // What we already hold, read LOCALLY (no network). The opener SHOULD be a
        // function of this so a converged peer can answer with the difference
        // instead of its whole corpus; today `round_opener` ignores it and the
        // round stays a full enumeration — the `sync-scale-honesty` standing red
        // (tests/sync_scale_honesty.rs). This state is threaded now so the cure is
        // a change in one planner body, not a re-plumb of every round.
        let local_state = match self
            .sync_manager
            .list_documents(namespace, None, 0, SYNC_LIST_PAGE_LIMIT)
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
            Err(e) => {
                debug!(error = %e, "Sync round: local corpus read failed, opening blind");
                sync_round::LocalCorpusState::default()
            }
        };

        for peer_id in peers {
            let request = sync_round::round_opener(namespace, &local_state);

            let mut swarm = self.swarm.write().await;
            let request_id = swarm
                .behaviour_mut()
                .sync_protocol
                .send_request(&peer_id, request);
            drop(swarm);
            // Track the page cursor so a has_more response can request the
            // next page (silent-tail guard — see DocListCursorMap).
            self.doc_list_cursors.lock().await.insert(request_id, 0);
            crate::metrics::inc_sync_request("list_documents");
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

        // Bound the scheduler map to the connected set (prunes churned peers),
        // then ask only peers eligible to START a new chain: not already
        // mid-chain (in-flight guard) and past their backoff gate. A slow/
        // timing-out peer is deferred here instead of being re-hammered every
        // tick — the structural fix for the fixed-30s-timeout storm.
        self.replication_scheduler.retain(&peers).await;
        let eligible = self.replication_scheduler.eligible_peers(&peers).await;
        if eligible.is_empty() {
            debug!(
                connected = peers.len(),
                "Replication cycle: all connected peers in-flight or backing off, skipping"
            );
            return;
        }

        // Query each eligible peer for content inventory so we discover content
        // regardless of which peer holds it. The response handler (ContentList
        // branch) deduplicates via replication_state.discover() and follows the
        // has_more page cursor. Page 0 at REPLICATION_LIST_PAGE_LIMIT so the
        // response fits a slow link inside the 30s request timeout.
        for peer in &eligible {
            let request = ShardRequest::ListContent {
                reach_filter: None,
                offset: 0,
                limit: REPLICATION_LIST_PAGE_LIMIT,
            };

            let mut swarm = self.swarm.write().await;
            let request_id = swarm
                .behaviour_mut()
                .shard_protocol
                .send_request(peer, request);
            drop(swarm);

            // Record the page cursor (peer, offset=0) so a has_more response can
            // continue the chain, and mark the peer in-flight so the next tick
            // does not open a second concurrent chain to it.
            self.list_content_cursors
                .lock()
                .await
                .insert(request_id, (*peer, 0, std::time::Instant::now()));
            self.replication_scheduler.mark_started(*peer).await;

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

    /// Acquisition reconcile (spec §4.2): load active pins, resolve item-pin
    /// wants (Slice 1: head_ref IS the item id; cluster pins are rejected at
    /// POST time), diff against local presence, enqueue gaps priority-ordered.
    /// Pure local computation — no network here (R-H).
    async fn run_acquisition_reconcile(&self) -> crate::metrics::AcquisitionReconcileOutcome {
        use crate::metrics::AcquisitionReconcileOutcome;

        let first_pass = self.acquisition.rollup_if_initialized().await.is_none();
        let Some(ref pool) = self.db_pool else {
            warn!(
                target: "elohim_storage::acquisition",
                first_pass,
                "acquisition reconcile stopped before pin census: DB pool is not configured"
            );
            return AcquisitionReconcileOutcome::DbPoolMissing;
        };
        let mut conn = match pool.get() {
            Ok(conn) => conn,
            Err(e) => {
                warn!(
                    target: "elohim_storage::acquisition",
                    error = %e,
                    first_pass,
                    "acquisition reconcile stopped before pin census: DB connection unavailable"
                );
                return AcquisitionReconcileOutcome::DbPoolUnavailable;
            }
        };

        // (0) Cooldown re-admission — the BACKSTOP arm of pin retirement. The
        // primary arm is inventory-driven (a peer advertising a retired pin's
        // head_ref revives it immediately, in the ContentList receive path); this
        // only bounds the worst case where nobody ever advertises the content
        // again, so the substrate re-checks a few times a day rather than never.
        // Retirement is a HOLD, not a delete.
        self.readmit_cooled_down_pins(&mut conn);

        let pins = match crate::db::acquisition_pins::list_active_pins(&mut conn) {
            Ok(p) => p,
            Err(e) => {
                warn!(
                    target: "elohim_storage::acquisition",
                    error = %e,
                    first_pass,
                    "acquisition reconcile stopped: active-pin census failed"
                );
                return AcquisitionReconcileOutcome::PinLoadFailed;
            }
        };
        let active_pin_count = pins.len();
        let pin_wants: Vec<(i32, Vec<String>)> = pins
            .iter()
            .filter(|p| p.kind == "item")
            .map(|p| (p.id, vec![p.head_ref.clone()]))
            .collect();

        let want_ids: Vec<String> = pin_wants
            .iter()
            .flat_map(|(_, ids)| ids.iter().cloned())
            .collect();
        // Slice 1: all pinned content lives in the lamad app — revisit the
        // app-scope here if cross-app pins land.
        let app_ctx = crate::db::AppContext::default_lamad();
        let local_has =
            match crate::db::content_diesel::content_ids_present(&mut conn, &app_ctx, &want_ids) {
                Ok(set) => set,
                Err(e) => {
                    warn!(
                        target: "elohim_storage::acquisition",
                        error = %e,
                        first_pass,
                        active_pins = active_pin_count,
                        "acquisition reconcile stopped: local-presence query failed"
                    );
                    return AcquisitionReconcileOutcome::PresenceQueryFailed;
                }
            };
        // DB work is done; drop the connection before the reconcile await so
        // it isn't held across the acquisition write-lock.
        drop(conn);

        // Peer-breadth budget: the dispatch rotation walks a different provider
        // per retry, so the budget IS the number of providers probed before an
        // item is called unsatisfiable. Sizing it from the live peer count is
        // what makes "exhausted" mean actually-unsatisfiable rather than
        // "we asked 3 of the 6 peers".
        let connected_peers = self.swarm.read().await.connected_peers().count();
        let to_dispatch = self
            .acquisition
            .reconcile(
                pin_wants,
                &local_has,
                acquisition::retry_budget_for_peers(connected_peers),
            )
            .await;
        if !to_dispatch.is_empty() {
            let mut q = self.acquisition_queue.lock().await;
            for id in to_dispatch {
                if !q.contains(&id) {
                    q.push_back(id);
                }
            }
        }

        crate::metrics::set_acquisition_reconcile_completed(active_pin_count);
        if first_pass {
            info!(
                target: "elohim_storage::acquisition",
                active_pins = active_pin_count,
                desired_items = want_ids.len(),
                local_items = local_has.len(),
                "acquisition reconcile first pass completed"
            );
        } else {
            debug!(
                target: "elohim_storage::acquisition",
                active_pins = active_pin_count,
                desired_items = want_ids.len(),
                local_items = local_has.len(),
                "acquisition reconcile completed"
            );
        }

        // Retire pins the fabric cannot satisfy. Held BACK, not deleted: the pin
        // row survives (a person still wants it) and is re-admitted the moment a
        // peer advertises the content or the cooldown elapses. Without this an
        // exhausted pin's tracker keeps `fetched < total` forever and
        // `pull.caughtUp` can never go true (alpha-A: 73 total / 3 fetched / 70
        // failed, caughtUp false for 12h+ with no runtime path to retirement —
        // the only status-flip caller was the HTTP DELETE route).
        self.retire_exhausted_pins().await;
        AcquisitionReconcileOutcome::Completed
    }

    /// Flip pins whose every want exhausted its retry budget to
    /// [`acquisition::PIN_STATUS_RETIRED`], and republish the retired gauge.
    ///
    /// No tracker eviction is written here: once a pin leaves `list_active_pins`
    /// the `trackers.retain` at the top of `AcquisitionState::reconcile` drops it
    /// on the next cycle, removing it from BOTH `total` and `fetched` in the
    /// rollup — which is exactly what lets `caught_up` become reachable again.
    async fn retire_exhausted_pins(&self) {
        let Some(ref pool) = self.db_pool else { return };
        let exhausted = self.acquisition.exhausted_pin_ids().await;
        let Ok(mut conn) = pool.get() else { return };

        let mut retired = 0u64;
        for pin_id in exhausted {
            match crate::db::acquisition_pins::set_pin_status(
                &mut conn,
                pin_id,
                acquisition::PIN_STATUS_RETIRED,
            ) {
                Ok(n) if n > 0 => {
                    retired += 1;
                    info!(
                        target: "elohim_storage::acquisition",
                        pin_id,
                        "acquisition pin RETIRED — no connected peer could supply its bytes; \
                         held back until a peer advertises it again or the cooldown elapses"
                    );
                }
                Ok(_) => {}
                // A rejected status write (e.g. a CHECK-constraint violation)
                // means the pin never actually left `active` — the exact
                // failure mode that made retirement inert until the
                // 2026-07-31-200000_acquisition_pins_admit_retired migration.
                // That must never be invisible again.
                Err(e) => warn!(error = %e, pin_id, "acquisition: pin retirement failed"),
            }
        }
        crate::metrics::inc_acquisition_pin_retirements("exhausted", retired);
        // Materialise the sibling series even when nothing revived this cycle, so
        // "no pin has ever been re-admitted" is distinguishable from "the emitter
        // never ran".
        crate::metrics::inc_acquisition_pin_retirements("readmitted", 0);
        self.publish_retired_pin_gauge(&mut conn);
    }

    /// Backstop re-admission: retired pins whose `updated_at` (the retirement
    /// clock — `set_pin_status` stamps it, so no migration is needed) is older
    /// than [`acquisition::PIN_READMIT_COOLDOWN`] go back to `active`.
    fn readmit_cooled_down_pins(&self, conn: &mut diesel::SqliteConnection) {
        let retired = match crate::db::acquisition_pins::list_pins_by_status(
            conn,
            acquisition::PIN_STATUS_RETIRED,
        ) {
            Ok(p) => p,
            Err(e) => {
                debug!(error = %e, "acquisition: retired-pin load failed");
                return;
            }
        };
        if retired.is_empty() {
            return;
        }
        let now = chrono::Utc::now();
        let mut readmitted = 0u64;
        for pin in &retired {
            if !acquisition::pin_cooled_down(&pin.updated_at, now) {
                continue;
            }
            match crate::db::acquisition_pins::set_pin_status(conn, pin.id, "active") {
                Ok(n) if n > 0 => {
                    readmitted += 1;
                    info!(
                        target: "elohim_storage::acquisition",
                        pin_id = pin.id,
                        head_ref = %pin.head_ref,
                        "acquisition pin RE-ADMITTED after cooldown — retirement is a hold, \
                         never a delete"
                    );
                }
                Ok(_) => {}
                // A rejected status write here would silently strand the pin
                // in `retired` forever — never invisible again.
                Err(e) => warn!(
                    error = %e,
                    pin_id = pin.id,
                    "acquisition: cooldown re-admission write failed"
                ),
            }
        }
        if readmitted > 0 {
            crate::metrics::inc_acquisition_pin_retirements("readmitted", readmitted);
        }
        self.publish_retired_pin_gauge(conn);
    }

    /// Republish `elohim_acquisition_pins_retired` from the DB (the durable
    /// truth), never from an incremented counter — a counter drifts across
    /// restarts and concurrent flips; a recount cannot.
    fn publish_retired_pin_gauge(&self, conn: &mut diesel::SqliteConnection) {
        if let Ok(rows) =
            crate::db::acquisition_pins::list_pins_by_status(conn, acquisition::PIN_STATUS_RETIRED)
        {
            crate::metrics::set_acquisition_pins_retired(rows.len() as u64);
        }
    }

    /// Primary re-admission arm: a peer's content inventory naming a RETIRED
    /// pin's `head_ref` is fresh evidence that the bytes exist somewhere, so the
    /// pin goes straight back to `active` and re-enters the pull queue on the
    /// next reconcile.
    ///
    /// Called from the inventory receive path, where the advertised id set is
    /// already in hand — no extra network, and no polling loop.
    fn readmit_pins_named_by_inventory(&self, remote_ids: &[String]) {
        if remote_ids.is_empty() {
            return;
        }
        let Some(ref pool) = self.db_pool else { return };
        let Ok(mut conn) = pool.get() else { return };
        let retired = match crate::db::acquisition_pins::list_pins_by_status(
            &mut conn,
            acquisition::PIN_STATUS_RETIRED,
        ) {
            Ok(p) if !p.is_empty() => p,
            _ => return,
        };
        let advertised: std::collections::HashSet<&str> =
            remote_ids.iter().map(String::as_str).collect();
        let mut readmitted = 0u64;
        for pin in &retired {
            if !advertised.contains(pin.head_ref.as_str()) {
                continue;
            }
            match crate::db::acquisition_pins::set_pin_status(&mut conn, pin.id, "active") {
                Ok(n) if n > 0 => {
                    readmitted += 1;
                    info!(
                        target: "elohim_storage::acquisition",
                        pin_id = pin.id,
                        head_ref = %pin.head_ref,
                        "acquisition pin RE-ADMITTED — a peer advertised its content again"
                    );
                }
                Ok(_) => {}
                // A rejected status write here would silently strand the pin
                // in `retired` forever — never invisible again.
                Err(e) => warn!(
                    error = %e,
                    pin_id = pin.id,
                    "acquisition: inventory-driven re-admission write failed"
                ),
            }
        }
        if readmitted > 0 {
            crate::metrics::inc_acquisition_pin_retirements("readmitted", readmitted);
            self.publish_retired_pin_gauge(&mut conn);
        }
    }

    /// Provide-loop tick: derive the caught-up commons desired set and run the
    /// reconciler. Author seam wiring (HcClient) is threaded by the conductor
    /// composition; absent it, the pass is a no-op derive (the reconciler's
    /// behaviour is unit-tested in services::provide_reconcile).
    async fn run_provide_reconcile(&self) {
        let Some(ref pool) = self.db_pool else { return };
        let Some(self_cid) = self.config.self_cid.clone() else {
            return;
        };
        let Ok(mut conn) = pool.get() else { return };

        let pins = match crate::db::acquisition_pins::list_active_pins(&mut conn) {
            Ok(p) => p,
            Err(e) => {
                debug!(error = %e, "provide reconcile: pin load failed");
                return;
            }
        };
        drop(conn);

        // caught-up head_refs from the live acquisition rollup.
        let caught_up: std::collections::HashSet<String> = self
            .acquisition
            .per_pin()
            .await
            .into_iter()
            .filter(|s| s.caught_up)
            .filter_map(|s| {
                pins.iter()
                    .find(|p| p.id == s.pin_id)
                    .map(|p| p.head_ref.clone())
            })
            .collect();

        // Provide-eligible head_refs: per-content (head_ref, reach) for present
        // content, classified reach-aware (commons is openly providable;
        // non-commons requires embodied responsibility for the scope). This
        // replaces the old commons-only filter so a non-commons item the node IS
        // eligible to provide can enter the desired set, while a non-commons item
        // the node is NOT eligible for is excluded (a privacy gate, not a no-op).
        use crate::services::provide_reconcile::ProvideEligibility;
        let head_refs: Vec<String> = pins.iter().map(|p| p.head_ref.clone()).collect();
        // Content reach per head_ref — feeds both the eligibility gate and the
        // Stage B reach map on the desired set (observe-only here, but the
        // signature carries it so the desired provides declare the content reach).
        let candidates: Vec<(String, String)> = {
            let Ok(mut conn) = pool.get() else { return };
            let app_ctx = crate::db::AppContext::default_lamad();
            crate::db::content_diesel::content_reaches_for_ids(&mut conn, &app_ctx, &head_refs)
                .unwrap_or_default()
        };
        let eligible: std::collections::HashSet<String> = {
            let resolver = crate::services::provide_reconcile::ClassifierEligibility::new(
                std::sync::Arc::new(pool.clone()),
                "lamad",
            );
            resolver.eligible_head_refs(&candidates)
        };
        let reach_by_head_ref: std::collections::HashMap<String, String> =
            candidates.iter().cloned().collect();

        let desired = crate::services::provide_reconcile::ProvideReconciler::derive_desired(
            &pins,
            &caught_up,
            &eligible,
            &reach_by_head_ref,
        );
        if desired.is_empty() {
            return;
        }

        // Keep the per-key stage latch current against the live DHT projection
        // (restart-safe stage re-derivation). No HcClient author seam is
        // composed on P2PNode yet, so this observe-only pass does not author;
        // the conductor-composed path drives `reconcile_provides` with a live
        // CommitmentAuthor. The diff/dedup behaviour is unit-tested in
        // services::provide_reconcile.
        let needs = match self
            .provide_reconciler
            .observe(pool, &self_cid, &desired)
            .await
        {
            Ok(n) => n,
            Err(e) => {
                debug!(error = %e, "provide reconcile: observe pass failed");
                return;
            }
        };
        debug!(
            target: "elohim_storage::provide",
            self_cid = %self_cid,
            desired = desired.len(),
            needs_commitment = needs,
            "provide reconcile: latched desired set (live author seam pending conductor wiring)"
        );
    }

    /// Acquisition dispatch (spec §4.2): shared-rails budget, round-robin
    /// GetContent across connected peers — same wire request the replication
    /// stream uses; streams differ in WHAT they want, not HOW they fetch.
    ///
    /// # Retry rotation (why `acquisition_rotation` exists)
    ///
    /// A failed item is NOT re-queued immediately: `GapTracker::mark_failed` drops
    /// it from `pending` and the next 60s `run_acquisition_reconcile` re-enqueues it
    /// (retry-on-NEXT-cycle, R-E). That reconcile rebuilds the queue from
    /// `list_active_pins` in stable DB order, so an item lands at the SAME batch
    /// position every cycle. With a bare `peers[position % peers.len()]` the item is
    /// therefore asked of exactly ONE peer on all `MAX_RETRIES` attempts — on a
    /// 6-peer fabric, five of six possible providers are never asked, and the item
    /// exhausts its retry budget having probed 1/6 of the fabric. The rotation
    /// offset advances once per drain so successive retries walk distinct peers.
    async fn drain_acquisition_queue(&self) {
        use acquisition_dispatch::{plan_acquisition_targets, AcquisitionTarget};
        let budget = reconcile_rails::DispatchBudget::new(acquisition::MAX_ACQUISITION_INFLIGHT);
        let peers: Vec<PeerId> = {
            let swarm = self.swarm.read().await;
            swarm.connected_peers().cloned().collect()
        };
        // Both planes feed the target set: libp2p-connected peers plus the iroh peer
        // book (row 13 — the pull leg was libp2p-only, so a pure-iroh mesh never
        // drained and a dual mesh moved no bulk bytes over iroh).
        #[cfg(feature = "p2p-iroh")]
        let targets: Vec<AcquisitionTarget> = {
            let entries = crate::p2p_iroh::iroh_fetch_leg()
                .map(|leg| leg.book().snapshot(Some(&leg.endpoint().node_id())))
                .unwrap_or_default();
            plan_acquisition_targets(&peers, &entries, *acquisition_prefer_iroh())
        };
        #[cfg(not(feature = "p2p-iroh"))]
        let targets: Vec<AcquisitionTarget> =
            plan_acquisition_targets(&peers, *acquisition_prefer_iroh());
        if targets.is_empty() {
            return; // peer-gated (R-E) — on either plane
        }
        let in_flight = self.pending_acquisition_fetches.lock().await.len()
            + self
                .acquisition_iroh_in_flight
                .load(std::sync::atomic::Ordering::Relaxed);
        let available = budget.available(in_flight);
        if available == 0 {
            return;
        }
        let to_dispatch: Vec<String> = {
            let mut queue = self.acquisition_queue.lock().await;
            if queue.is_empty() {
                return;
            }
            let len = queue.len();
            queue.drain(..available.min(len)).collect()
        };
        // Advance once per drain so a retry at a stable batch position walks to a
        // different peer than the attempt before it.
        let rotation = self
            .acquisition_rotation
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let mut skipped = 0usize;
        let mut sent = 0usize;
        for (i, id) in to_dispatch.iter().enumerate() {
            if !self.acquisition.wants(id).await {
                skipped += 1;
                continue;
            }
            let target = &targets[rotated_peer_index(rotation, i, targets.len())];
            crate::metrics::inc_acquisition_dispatch(target.transport());
            match target {
                AcquisitionTarget::Libp2p(peer) => {
                    let request = ShardRequest::GetContent { id: id.clone() };
                    let mut swarm = self.swarm.write().await;
                    let request_id = swarm
                        .behaviour_mut()
                        .shard_protocol
                        .send_request(peer, request);
                    drop(swarm);
                    self.pending_acquisition_fetches
                        .lock()
                        .await
                        .insert(request_id, id.clone());
                }
                #[cfg(feature = "p2p-iroh")]
                AcquisitionTarget::Iroh { label, addr } => {
                    // `Swarm` is not `Sync`, so the task captures only the
                    // transport-neutral ingest context — never `self`.
                    let ctx = self.acquisition_ingest_ctx();
                    let counter = self.acquisition_iroh_in_flight.clone();
                    let id = id.clone();
                    let label = label.clone();
                    let addr = addr.clone();
                    counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    tokio::spawn(async move {
                        acquire_over_iroh(ctx, id, label, addr, PullKind::Pin).await;
                        counter.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
                    });
                }
            }
            sent += 1;
        }
        if skipped > 0 {
            debug!(
                sent,
                skipped, "Acquisition dispatch: skipped no-longer-wanted items"
            );
        }
    }

    /// Snapshot the deps the transport-neutral acquisition ingest needs. Cheap
    /// (Arc/handle clones); built per dispatch so a spawned iroh task never
    /// holds `self` (the swarm is not `Sync`).
    fn acquisition_ingest_ctx(&self) -> AcquisitionIngestCtx {
        AcquisitionIngestCtx {
            db_pool: self.db_pool.clone(),
            replication_state: self.replication_state.clone(),
            acquisition: self.acquisition.clone(),
            blob_store: self.blob_store.clone(),
            self_cid: self.config.self_cid.clone().unwrap_or_default(),
        }
    }

    /// Publish the freshly acquired content's EPR head as a kad record — the
    /// libp2p-only tail of ingest (the iroh plane announces heads on its own
    /// manifest board; an iroh-acquired row is announced there by the announcer).
    async fn publish_epr_head_record(&self, content_id: &str) {
        if let Some(head_bytes) = self.resolve_epr_head_locally(content_id) {
            let key = RecordKey::new(&format!("epr:{}", content_id));
            let dht_record = Record {
                key,
                value: head_bytes,
                publisher: Some(*self.identity.peer_id()),
                expires: None,
            };
            let mut swarm = self.swarm.write().await;
            let _ = swarm
                .behaviour_mut()
                .kademlia
                .put_record(dht_record, libp2p::kad::Quorum::One);
        }
    }

    /// Record the co-resident iroh node's `NodeId` on the status snapshot.
    ///
    /// Wired from the `main.rs` dual block once the iroh node exists. Mutates the
    /// live watch value in place (`send_modify`); `refresh_status` preserves it
    /// across rebuilds by reading the current value back, so the periodic
    /// libp2p-only status rebuild never clobbers it back to `None`.
    pub fn set_iroh_node_id(&self, node_id: String) {
        self.status_tx
            .send_modify(|s| s.iroh_node_id = Some(node_id));
    }

    /// Refresh the status snapshot (called from event loop)
    async fn refresh_status(&self) {
        let swarm = self.swarm.read().await;
        let connected_peers = swarm.connected_peers().count();
        let listen_addresses: Vec<String> = swarm.listeners().map(|a| a.to_string()).collect();
        let bootstrap_nodes: Vec<String> = self.config.bootstrap_nodes.clone();
        // Whole-store count. `count_documents("_all")` scanned the literal
        // "_all:" prefix and read 0 forever (2026-08-24, 5,356 docs on the fleet).
        let sync_documents = self.sync_manager.count_all().await.unwrap_or(0) as usize;
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
        let pull = self.acquisition.rollup_if_initialized().await;
        let projection_reconcile = match &self.projection_reconcile {
            Some(s) => Some(s.status().await),
            None => None,
        };
        let provide_loop = match &self.provide_loop {
            Some(s) => Some(s.status().await),
            None => None,
        };
        let recon = self.reconciliation_metrics();
        // Preserve the iroh NodeId set once by `set_iroh_node_id` (main.rs dual
        // block) — the periodic libp2p-only rebuild must not clobber it to None.
        let iroh_node_id = self.status_tx.borrow().iroh_node_id.clone();
        let sync_verdict = self.sync_gate.verdict();
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
            pull,
            projection_reconcile,
            sync_paused: !sync_verdict.syncing,
            sync_mode: self.sync_gate.mode().as_str().to_string(),
            network_class: self.sync_gate.network().as_str().to_string(),
            sync_reasons: sync_verdict.reason_strings(),
            dedup_unique_len,
            dedup_total_seen,
            reconcile_passes_total: recon.reconcile_passes_total,
            kicks_fired_total: recon.kicks_fired_total,
            placement_gaps_emitted_total: recon.placement_gaps_emitted_total,
            provide_loop,
            iroh_node_id,
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

/// T24 persistent peering: which configured bootstrap addrs lack a live
/// connection? An addr needs redial when its PeerId was never learned (no
/// successful outbound dial yet — covers inbound-only and never-connected
/// links) or its learned PeerId is absent from the connected set. Invalid
/// multiaddr strings are skipped (startup already warns on them).
pub(crate) fn bootstrap_needing_redial(
    configured: &[String],
    learned: &DashMap<String, PeerId>,
    connected: &std::collections::HashSet<PeerId>,
) -> Vec<Multiaddr> {
    configured
        .iter()
        .filter_map(|s| s.parse::<Multiaddr>().ok())
        .filter(|addr| match learned.get(&addr.to_string()) {
            Some(pid) => !connected.contains(pid.value()),
            None => true,
        })
        .collect()
}

/// Prometheus `result` label for a view-federation outbound failure. Mirrors the
/// blob-protocol classification strings; the `io` label collapses the payload
/// variant (matches the `elohim_view_federation_outbound_total` label set).
pub(crate) fn view_federation_outbound_result_label(
    error: &libp2p::request_response::OutboundFailure,
) -> &'static str {
    use libp2p::request_response::OutboundFailure;
    match error {
        OutboundFailure::Timeout => "timeout",
        OutboundFailure::ConnectionClosed => "connection_closed",
        OutboundFailure::DialFailure => "dial_failure",
        OutboundFailure::UnsupportedProtocols => "unsupported_protocols",
        OutboundFailure::Io(_) => "io",
    }
}

#[cfg(test)]
mod view_federation_metric_tests {
    use super::view_federation_outbound_result_label;
    use libp2p::request_response::OutboundFailure;

    #[test]
    fn outbound_result_labels_cover_every_variant() {
        assert_eq!(
            view_federation_outbound_result_label(&OutboundFailure::Timeout),
            "timeout"
        );
        assert_eq!(
            view_federation_outbound_result_label(&OutboundFailure::ConnectionClosed),
            "connection_closed"
        );
        assert_eq!(
            view_federation_outbound_result_label(&OutboundFailure::DialFailure),
            "dial_failure"
        );
        assert_eq!(
            view_federation_outbound_result_label(&OutboundFailure::UnsupportedProtocols),
            "unsupported_protocols"
        );
        assert_eq!(
            view_federation_outbound_result_label(&OutboundFailure::Io(std::io::Error::from(
                std::io::ErrorKind::UnexpectedEof
            ))),
            "io"
        );
    }
}

#[cfg(test)]
mod bootstrap_peering_tests {
    use super::bootstrap_needing_redial;
    use dashmap::DashMap;
    use libp2p::{Multiaddr, PeerId};
    use std::collections::HashSet;

    const ADDR_A: &str = "/dns4/elohim-jessica-alpha-0.svc/tcp/9876";
    const ADDR_B: &str = "/dns4/elohim-pete-alpha-0.svc/tcp/9876";

    fn canonical(addr: &str) -> String {
        addr.parse::<Multiaddr>().unwrap().to_string()
    }

    #[test]
    fn unlearned_addr_needs_redial() {
        let learned = DashMap::new();
        let connected = HashSet::new();
        let needing = bootstrap_needing_redial(&[ADDR_A.to_string()], &learned, &connected);
        assert_eq!(needing.len(), 1);
        assert_eq!(needing[0].to_string(), canonical(ADDR_A));
    }

    #[test]
    fn learned_and_connected_does_not_redial() {
        let pid = PeerId::random();
        let learned = DashMap::new();
        learned.insert(canonical(ADDR_A), pid);
        let connected: HashSet<PeerId> = [pid].into_iter().collect();
        let needing = bootstrap_needing_redial(&[ADDR_A.to_string()], &learned, &connected);
        assert!(needing.is_empty());
    }

    #[test]
    fn learned_but_dropped_link_redials_only_that_addr() {
        // The genesis #1119 shape: one configured link drops while the other
        // stays connected — the dropped one (and ONLY it) must redial even
        // though connected.len() > 0 (the old connected==0 gate never fired).
        let pid_a = PeerId::random();
        let pid_b = PeerId::random();
        let learned = DashMap::new();
        learned.insert(canonical(ADDR_A), pid_a);
        learned.insert(canonical(ADDR_B), pid_b);
        let connected: HashSet<PeerId> = [pid_b].into_iter().collect();
        let needing = bootstrap_needing_redial(
            &[ADDR_A.to_string(), ADDR_B.to_string()],
            &learned,
            &connected,
        );
        assert_eq!(needing.len(), 1);
        assert_eq!(needing[0].to_string(), canonical(ADDR_A));
    }

    #[test]
    fn invalid_multiaddr_is_skipped() {
        let learned = DashMap::new();
        let connected = HashSet::new();
        let needing = bootstrap_needing_redial(
            &["not-a-multiaddr".to_string(), ADDR_A.to_string()],
            &learned,
            &connected,
        );
        assert_eq!(needing.len(), 1);
    }
}

#[cfg(test)]
mod acquisition_rotation_tests {
    use super::rotated_peer_index;

    /// THE DEFECT: with a bare `position % peer_count`, an item at a stable batch
    /// position is asked of the SAME peer on every retry. The acquisition queue is
    /// rebuilt each 60s reconcile from `list_active_pins` in stable DB order, so the
    /// position IS stable — meaning on a 6-peer fabric an item burns its whole
    /// retry budget probing one sixth of the fabric. Live alpha read
    /// `pull:{total:36,fetched:0,failed:36}`.
    ///
    /// With the rotation term, successive drains walk successive peers, so a
    /// 3-retry budget probes 3 DISTINCT peers.
    #[test]
    fn successive_retries_of_a_stable_position_walk_distinct_peers() {
        let peers = 6usize;
        let position = 0usize; // the pathological case: always first in the batch

        // Pre-fix behaviour, for contrast: position % peers is constant.
        assert_eq!(position % peers, 0);

        // Post-fix: drain N (rotation = N) sends that position to a new peer.
        let visited: Vec<usize> = (0..3)
            .map(|rotation| rotated_peer_index(rotation, position, peers))
            .collect();
        assert_eq!(visited, vec![0, 1, 2]);
        let distinct: std::collections::HashSet<usize> = visited.iter().copied().collect();
        assert_eq!(
            distinct.len(),
            3,
            "a 3-retry budget must probe 3 distinct peers, not one peer 3 times"
        );
    }

    /// Within ONE drain the spread across the batch is preserved — the rotation
    /// shifts the whole batch, it does not collapse it onto a single peer.
    #[test]
    fn one_drain_still_spreads_the_batch_across_peers() {
        let peers = 3usize;
        let assigned: Vec<usize> = (0..6)
            .map(|position| rotated_peer_index(7, position, peers))
            .collect();
        assert_eq!(assigned, vec![1, 2, 0, 1, 2, 0]);
    }

    /// Total function: an empty peer set yields 0 rather than panicking on `% 0`.
    /// (Callers peer-gate before dispatch; this keeps the helper unit-testable.)
    #[test]
    fn empty_peer_set_is_total() {
        assert_eq!(rotated_peer_index(9, 4, 0), 0);
    }

    /// The rotation counter is a monotonically-increasing `AtomicUsize`; it must
    /// stay correct across the `usize` wrap rather than panicking in debug builds.
    #[test]
    fn rotation_wraps_without_overflow() {
        assert_eq!(rotated_peer_index(usize::MAX, 1, 4), 0);
        assert_eq!(rotated_peer_index(usize::MAX, 2, 4), 1);
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

// ---------------------------------------------------------------------------
// InventoryFetch — libp2p-command-path inventory hook for the transport-neutral
// gossip dispatch. Both receive planes enter the same bounded command queue;
// the event loop remains the sole owner of scoring and byte-fetch scheduling.
// ---------------------------------------------------------------------------
impl crate::p2p::gossip_dispatch::InventoryFetch for P2PNode {
    fn score_and_enqueue(
        &self,
        _conn: &mut diesel::SqliteConnection,
        source_peer_id: &str,
        hints: &[crate::p2p::inventory_gossip::BlobHint],
        hashes: &[String],
    ) {
        let work = P2PCommand::InventoryAdvertisement {
            source_peer_id: source_peer_id.to_string(),
            hints: hints.to_vec(),
            hashes: hashes.to_vec(),
        };
        if let Err(error) = self.command_tx.try_send(work) {
            warn!(
                target: "elohim_storage::inventory",
                peer_id = %source_peer_id,
                count = hashes.len(),
                error = ?error,
                "libp2p inventory fetch work could not enter the bounded command queue"
            );
        }
    }

    fn request_snapshot_for_gap(&self, peer_id: &str) {
        // Best-effort: parse the peer_id string into a libp2p::PeerId and send
        // the snapshot-request command. A pure iroh NodeId will not parse;
        // the bridge logs that explicit unsupported case.
        if let Ok(pid) = peer_id.parse::<libp2p::PeerId>() {
            let _ = self
                .command_tx
                .try_send(P2PCommand::SnapshotRequest { peer_id: pid });
        }
    }
}

#[cfg(test)]
mod view_federation_offloop_tests {
    //! F-A(ii): the view-federation inbound arm must not do its work ON the
    //! swarm event loop.
    //!
    //! ## The incident these pin (2026-08-01)
    //!
    //! `handle_swarm_event` awaits `handle_behaviour_event` to completion before
    //! the loop re-polls the swarm, so ANY await inside an inbound arm halts
    //! every other swarm event — inbound requests, outbound response delivery,
    //! gossip, identify, kad. The `ContentHeadRecord` view kind makes that fatal
    //! because its slice-build asks the conductor, and `HcClient::call_zome`
    //! carried no timeout of its own: one inbound head-record ask wedged the
    //! whole loop for up to ~60s, which is longer than every requester's budget.
    //! Fleet-wide this meant adopt-before-author — the ONLY path that converges a
    //! peer-divergent content head — never completed a fetch, while the same
    //! wedge starved the inventory asks the next sweep depends on.
    //!
    //! `Swarm` is not `Sync`, so the spawned task cannot hold the swarm to
    //! answer with; the finished slice routes back through
    //! `P2PCommand::SendViewFederationResponse`, which the command loop (which
    //! already owns `&mut Swarm`) delivers synchronously.
    //!
    //! There is no swarm harness in unit tests, so the off-loop property is
    //! pinned structurally, against stable source markers.

    /// The inbound arm's source text, sliced between two stable anchors.
    fn inbound_arm_source() -> &'static str {
        let src = include_str!("mod.rs");
        let start = src
            .find("F-T20: received view-federation request")
            .expect("the inbound view-federation arm's debug! marker must exist");
        let rest = &src[start..];
        let end = rest
            .find("OutboundFailure {")
            .expect("the OutboundFailure arm must follow the inbound arm");
        &rest[..end]
    }

    /// The slice-build must happen inside a spawned task, not inline.
    ///
    /// Regression guard: delete the `tokio::spawn` and await
    /// `build_response_slice` inline (the pre-fix shape) and this fails.
    #[test]
    fn the_inbound_arm_builds_its_slice_off_the_event_loop() {
        let arm = inbound_arm_source();
        let spawn_at = arm
            .find("tokio::spawn(")
            .expect("the inbound view-federation arm must hand its slice-build to a spawned task");
        let build_at = arm
            .find("build_response_slice(")
            .expect("the inbound arm must still build a slice");
        assert!(
            spawn_at < build_at,
            "build_response_slice must be awaited INSIDE the spawned task — awaiting it on the \
             event-loop task halts every other swarm event for the duration of its conductor call"
        );
    }

    /// EVERY view kind takes the spawned path — not just the conductor-touching
    /// one — so a future view kind that grows a conductor call cannot silently
    /// re-create the head-of-line block.
    #[test]
    fn no_view_kind_is_special_cased_back_onto_the_event_loop() {
        let arm = inbound_arm_source();
        assert_eq!(
            arm.matches("build_response_slice(").count(),
            1,
            "exactly ONE slice-build call site keeps every view kind on the spawned path; a \
             second one is a per-view-kind fast path back onto the event loop"
        );
        assert_eq!(
            arm.matches("tokio::spawn(").count(),
            1,
            "one spawn covers the whole arm"
        );
    }

    /// The spawned task must NOT capture the swarm handle: `Swarm` is not
    /// `Sync`, so `Arc<RwLock<Swarm>>` is not `Send` and the spawn would not
    /// compile — but more importantly, re-acquiring the swarm inside the task is
    /// how a future edit would smuggle loop-blocking work back in.
    #[test]
    fn the_spawned_task_answers_through_the_command_loop_not_the_swarm() {
        let arm = inbound_arm_source();
        let spawn_at = arm.find("tokio::spawn(").expect("spawn marker");
        let spawned_body = &arm[spawn_at..];
        assert!(
            spawned_body.contains("P2PCommand::SendViewFederationResponse"),
            "the spawned task must route its finished slice back through the command loop"
        );
        assert!(
            !spawned_body.contains("swarm.write()") && !spawned_body.contains("swarm.read()"),
            "the spawned task must never touch the swarm directly"
        );
    }

    /// LIVENESS ONLY — this does NOT exercise
    /// `P2PCommand::SendViewFederationResponse`.
    ///
    /// Naming it honestly, because the first version of this test implied
    /// coverage it never had. `libp2p::request_response::ResponseChannel` has a
    /// private sender and no public constructor, so the variant cannot be built
    /// outside a live swarm — there is no way to push one through the stub loop
    /// from a unit test. What IS covered elsewhere: the variant is handled in
    /// both `handle_command` and the swarm-less stub drain (a missing arm is a
    /// compile error, `match cmd` being exhaustive), and the spawned task's use
    /// of it is pinned by
    /// `the_spawned_task_answers_through_the_command_loop_not_the_swarm`.
    /// Genuine delivery coverage needs the integration harness.
    #[tokio::test]
    async fn the_swarmless_command_loop_stays_live() {
        let handle = crate::p2p::P2PHandle::for_testing();
        let peers = handle.list_peers().await;
        assert!(
            peers.is_empty(),
            "the swarm-less test handle reports no peers"
        );
    }

    /// The inbound fan-out cap must exist, be acquired INSIDE the spawned task,
    /// and stay small.
    ///
    /// Acquiring outside the spawn would re-create the exact head-of-line block
    /// the spawn removes — a permit wait on the event-loop task is no better
    /// than a conductor call on it.
    #[test]
    fn the_inbound_fanout_cap_is_bounded_and_acquired_inside_the_spawn() {
        use crate::p2p::VIEW_FEDERATION_INBOUND_LIMIT;
        assert!(
            VIEW_FEDERATION_INBOUND_LIMIT.available_permits() > 0
                && VIEW_FEDERATION_INBOUND_LIMIT.available_permits() <= 16,
            "the cap must be small enough to protect a saturated conductor, and non-zero"
        );
        let arm = inbound_arm_source();
        let spawn_at = arm.find("tokio::spawn(").expect("spawn marker");
        let acquire_at = arm
            .find("VIEW_FEDERATION_INBOUND_LIMIT.acquire()")
            .expect("the spawned task must acquire a fan-out permit");
        assert!(
            spawn_at < acquire_at,
            "the permit must be acquired INSIDE the spawn — waiting for one on the event-loop \
             task blocks the loop exactly like the conductor call did"
        );
        let build_at = arm
            .find("build_response_slice(")
            .expect("slice-build marker");
        assert!(
            acquire_at < build_at,
            "the permit must be held ACROSS the slice-build, or it caps nothing"
        );
    }
}

/// Is iroh preferred for the pull leg? Read once (env `ELOHIM_ACQUISITION_IROH`).
fn acquisition_prefer_iroh() -> &'static bool {
    static PREFER: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    PREFER.get_or_init(acquisition_dispatch::prefer_iroh_from_env)
}

/// The deps a content-record ingest needs, independent of the plane the record
/// arrived on. `Clone` + `Send` so the iroh pull task can own one.
#[derive(Clone)]
pub(crate) struct AcquisitionIngestCtx {
    pub(crate) db_pool: Option<DbPool>,
    pub(crate) replication_state: replication::ReplicationState,
    pub(crate) acquisition: acquisition::AcquisitionState,
    pub(crate) blob_store: Arc<BlobStore>,
    pub(crate) self_cid: String,
}

pub(crate) enum StoredRecord {
    Stored {
        #[allow(dead_code)]
        content_id: String,
        blob_hash: Option<String>,
    },
    Failed,
}

/// Store an acquired content record and settle the acquisition/replication
/// trackers exactly as the libp2p arm always has. Transport-neutral: the
/// libp2p `ShardResponse::Content` arm and the iroh pull task both call it.
pub(crate) async fn store_acquired_record(
    ctx: &AcquisitionIngestCtx,
    record: crate::p2p::shard_protocol::ContentRecord,
) -> StoredRecord {
    let content_id = record.id.clone();
    let blob_hash_opt = record.blob_hash.clone();
    let pool = match ctx.db_pool.as_ref() {
        Some(p) => p,
        None => {
            ctx.replication_state.mark_failed(&content_id).await;
            crate::metrics::inc_acquisition_outcome("no_db_pool");
            ctx.acquisition.mark_failed(&content_id).await;
            return StoredRecord::Failed;
        }
    };
    let mut conn = match pool.get() {
        Ok(c) => c,
        Err(_) => {
            ctx.replication_state.mark_failed(&content_id).await;
            crate::metrics::inc_acquisition_outcome("no_db_conn");
            ctx.acquisition.mark_failed(&content_id).await;
            return StoredRecord::Failed;
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
        dht_anchor_hash: None,
    };
    let app_ctx = crate::db::AppContext::default_lamad();
    match crate::db::content_diesel::bulk_create_content(&mut conn, &app_ctx, vec![input]) {
        Ok(result) => {
            if result.inserted > 0 || result.skipped > 0 {
                ctx.replication_state.mark_completed(&content_id).await;
                ctx.acquisition.mark_completed(&content_id).await;
                crate::metrics::inc_acquisition_outcome("fetched");
                StoredRecord::Stored {
                    content_id,
                    blob_hash: blob_hash_opt,
                }
            } else {
                ctx.replication_state.mark_failed(&content_id).await;
                crate::metrics::inc_acquisition_outcome("blob_unavailable");
                ctx.acquisition.mark_failed(&content_id).await;
                StoredRecord::Failed
            }
        }
        Err(e) => {
            warn!(id = %content_id, error = %e, "Failed to store replicated content");
            ctx.replication_state.mark_failed(&content_id).await;
            crate::metrics::inc_acquisition_outcome("store_failed");
            ctx.acquisition.mark_failed(&content_id).await;
            StoredRecord::Failed
        }
    }
}

/// Settle a failed pull on the tracker its queue belongs to. A gap failure keeps
/// `caught_up` honest via `ReplicationState::mark_failed`; a pin failure walks the
/// pin's retry budget via `AcquisitionState::mark_failed` (no-op for untracked ids).
#[cfg(feature = "p2p-iroh")]
async fn settle_pull_failure(ctx: &AcquisitionIngestCtx, content_id: &str, kind: PullKind) {
    match kind {
        PullKind::Gap => {
            ctx.replication_state.mark_failed(content_id).await;
            ctx.replication_state.update_caught_up().await;
        }
        PullKind::Pin => ctx.acquisition.mark_failed(content_id).await,
    }
}

/// Which queue a pull came from — decides which tracker a failure settles
/// (the libp2p arm keys the same decision off its two pending maps).
#[cfg(feature = "p2p-iroh")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PullKind {
    /// A replication gap (peer inventory named an id we lack) — settles `ReplicationState`.
    Gap,
    /// A pin want (acquisition reconcile) — settles `AcquisitionState`.
    Pin,
}

/// The iroh pull leg for one queued content id: `GetContent` over the shard
/// ALPN, then — for a blob-backed row we do not hold — the quilt-draw blob
/// pull over the SAME peer, landing through `finalize_quilt_draw` (verify-first,
/// like the libp2p arm). Bounded: one request per step, each under a timeout.
#[cfg(feature = "p2p-iroh")]
pub(crate) async fn acquire_over_iroh(
    ctx: AcquisitionIngestCtx,
    content_id: String,
    label: String,
    addr: iroh::NodeAddr,
    kind: PullKind,
) {
    use crate::p2p_iroh::IrohShardClient;
    const CONTENT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
    const BLOB_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);

    let Some(leg) = crate::p2p_iroh::iroh_fetch_leg() else {
        // Planned while the leg existed; cannot happen after registration, but the
        // honest disposition is a failed attempt the rotation retries elsewhere.
        crate::metrics::inc_acquisition_outcome("fetch_error");
        settle_pull_failure(&ctx, &content_id, kind).await;
        return;
    };
    let client = IrohShardClient::new(leg.endpoint());
    let request = ShardRequest::GetContent {
        id: content_id.clone(),
    };
    let response =
        tokio::time::timeout(CONTENT_TIMEOUT, client.request(addr.clone(), &request)).await;
    let record = match response {
        Ok(Ok(ShardResponse::Content(record))) => record,
        Ok(Ok(ShardResponse::ContentNotFound)) => {
            crate::metrics::inc_acquisition_outcome("fetch_error");
            settle_pull_failure(&ctx, &content_id, kind).await;
            ctx.replication_state.update_caught_up().await;
            return;
        }
        Ok(Ok(other)) => {
            debug!(id = %content_id, peer = %label, response = ?std::mem::discriminant(&other), "iroh acquisition: unexpected response");
            crate::metrics::inc_acquisition_outcome("fetch_error");
            settle_pull_failure(&ctx, &content_id, kind).await;
            return;
        }
        Ok(Err(e)) => {
            debug!(id = %content_id, peer = %label, error = %e, "iroh acquisition: request failed");
            crate::metrics::inc_acquisition_outcome("fetch_error");
            settle_pull_failure(&ctx, &content_id, kind).await;
            return;
        }
        Err(_) => {
            debug!(id = %content_id, peer = %label, "iroh acquisition: request timed out");
            crate::metrics::inc_acquisition_outcome("fetch_error");
            settle_pull_failure(&ctx, &content_id, kind).await;
            return;
        }
    };
    debug!(id = %content_id, peer = %label, transport = "iroh", "Received content record from peer");
    if let StoredRecord::Stored { blob_hash, .. } = store_acquired_record(&ctx, *record).await {
        if let Some(hash) = blob_hash.filter(|h| !h.is_empty()) {
            if !ctx.blob_store.exists(&hash).await {
                info!(
                    content_id = %content_id,
                    blob_hash = %hash,
                    source_peer = %label,
                    transport = "iroh",
                    "quilt draw: pulling blob from peer after content replication"
                );
                let pull = ShardRequest::Get { hash: hash.clone() };
                match tokio::time::timeout(BLOB_TIMEOUT, client.request(addr, &pull)).await {
                    Ok(Ok(ShardResponse::Data(data))) => {
                        let landed = match ctx.db_pool.as_ref().map(|p| p.get()) {
                            Some(Ok(mut conn)) => {
                                crate::p2p::blob_fetch::finalize_quilt_draw(
                                    &mut conn,
                                    &hash,
                                    &label,
                                    &data,
                                    &ctx.self_cid,
                                    &ctx.blob_store,
                                )
                                .await
                            }
                            _ => ctx.blob_store.store(&data).await.map(|_| true),
                        };
                        match landed {
                            Ok(true) => {
                                crate::metrics::inc_iroh_blob_fetch("ok");
                                info!(content_id = %content_id, blob_hash = %hash, size = data.len(), source_peer = %label, transport = "iroh", "quilt draw: blob stocked + serve-blob delivery event booked");
                            }
                            Ok(false) => {
                                crate::metrics::inc_iroh_blob_fetch("hash_mismatch");
                                warn!(content_id = %content_id, blob_hash = %hash, source_peer = %label, transport = "iroh", "quilt draw: pulled bytes failed hash verification; discarded");
                            }
                            Err(e) => {
                                crate::metrics::inc_iroh_blob_fetch("error");
                                warn!(content_id = %content_id, blob_hash = %hash, error = %e, transport = "iroh", "quilt draw: finalize_quilt_draw failed");
                            }
                        }
                    }
                    Ok(Ok(ShardResponse::NotFound)) => {
                        crate::metrics::inc_iroh_blob_fetch("not_found");
                        warn!(content_id = %content_id, blob_hash = %hash, source_peer = %label, transport = "iroh", "quilt draw: peer served the record but not its blob");
                    }
                    Ok(Ok(_)) | Ok(Err(_)) => {
                        crate::metrics::inc_iroh_blob_fetch("error");
                        warn!(content_id = %content_id, blob_hash = %hash, source_peer = %label, transport = "iroh", "quilt draw: blob pull failed at transport level");
                    }
                    Err(_) => {
                        crate::metrics::inc_iroh_blob_fetch("error");
                        warn!(content_id = %content_id, blob_hash = %hash, source_peer = %label, transport = "iroh", "quilt draw: blob pull timed out");
                    }
                }
            }
        }
    }
    ctx.replication_state.update_caught_up().await;
}
