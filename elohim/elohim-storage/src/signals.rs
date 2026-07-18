//! Signal-based P2P blob transfer
//!
//! Couples blob storage to Holochain DHT by using Holochain signals for
//! coordination. The DNA network handles discovery, this handles data transfer.
//!
//! ## Protocol
//!
//! 1. **Request**: DNA emits `BlobRequest { hash, requester_agent }`
//! 2. **Announce**: Nodes that have the blob emit `BlobAnnounce { hash, chunks, size }`
//! 3. **Transfer**: Direct P2P transfer negotiated between nodes
//!
//! ## Signal Types
//!
//! Sent via Holochain DNA signal mechanism:
//! - `blob_request` - "I need this blob"
//! - `blob_announce` - "I have this blob"
//! - `blob_chunk` - "Here's a chunk" (for small chunks only)
//! - `blob_transfer_init` - "Let's do a direct transfer"

use crate::blob_store::BlobStore;
use crate::error::StorageError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Signal types for blob coordination
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum BlobSignal {
    /// Request a blob by hash
    Request { hash: String, requester: String },

    /// Announce having a blob
    Announce {
        hash: String,
        size_bytes: u64,
        chunk_count: u32,
        provider: String,
    },

    /// Small chunk delivery (for blobs < 256KB, fits in signal)
    ChunkDelivery {
        hash: String,
        chunk_index: u32,
        total_chunks: u32,
        data: Vec<u8>,
    },

    /// Initiate direct transfer for large blobs
    TransferInit {
        hash: String,
        size_bytes: u64,
        transfer_id: String,
        /// Direct connection info (e.g., WebRTC offer, or TCP address if on same network)
        connection_info: String,
    },
}

/// Maximum size for inline chunk delivery via signal (256KB)
const MAX_SIGNAL_CHUNK_SIZE: usize = 256 * 1024;

/// Pending blob requests
#[allow(dead_code)]
struct PendingRequest {
    hash: String,
    requester: String,
    received_chunks: HashMap<u32, Vec<u8>>,
    total_chunks: Option<u32>,
    created_at: std::time::Instant,
}

/// Signal handler for blob P2P coordination
pub struct SignalHandler {
    store: Arc<BlobStore>,
    pending_requests: RwLock<HashMap<String, PendingRequest>>,
    my_agent_id: String,
}

impl SignalHandler {
    pub fn new(store: Arc<BlobStore>, my_agent_id: String) -> Self {
        Self {
            store,
            pending_requests: RwLock::new(HashMap::new()),
            my_agent_id,
        }
    }

    /// Handle incoming signal from Holochain
    pub async fn handle_signal(
        &self,
        signal: BlobSignal,
    ) -> Result<Option<BlobSignal>, StorageError> {
        match signal {
            BlobSignal::Request { hash, requester } => self.handle_request(&hash, &requester).await,
            BlobSignal::Announce {
                hash,
                size_bytes,
                chunk_count,
                provider,
            } => {
                self.handle_announce(&hash, size_bytes, chunk_count, &provider)
                    .await
            }
            BlobSignal::ChunkDelivery {
                hash,
                chunk_index,
                total_chunks,
                data,
            } => {
                self.handle_chunk_delivery(&hash, chunk_index, total_chunks, data)
                    .await
            }
            BlobSignal::TransferInit {
                hash,
                size_bytes,
                transfer_id,
                connection_info,
            } => {
                self.handle_transfer_init(&hash, size_bytes, &transfer_id, &connection_info)
                    .await
            }
        }
    }

    /// Handle blob request - check if we have it and can provide
    async fn handle_request(
        &self,
        hash: &str,
        requester: &str,
    ) -> Result<Option<BlobSignal>, StorageError> {
        if !self.store.exists(hash).await {
            debug!(hash = %hash, "Blob request received but we don't have it");
            return Ok(None);
        }

        let size_bytes = self.store.size(hash).await?;

        info!(hash = %hash, requester = %requester, size = size_bytes, "Announcing blob availability");

        // If small enough, could send directly via chunks
        // For now, just announce availability
        Ok(Some(BlobSignal::Announce {
            hash: hash.to_string(),
            size_bytes,
            chunk_count: size_bytes.div_ceil(MAX_SIGNAL_CHUNK_SIZE as u64) as u32,
            provider: self.my_agent_id.clone(),
        }))
    }

    /// Handle blob announcement - decide if we want to request it
    async fn handle_announce(
        &self,
        hash: &str,
        size_bytes: u64,
        chunk_count: u32,
        provider: &str,
    ) -> Result<Option<BlobSignal>, StorageError> {
        // Check if we have a pending request for this blob
        let mut pending = self.pending_requests.write().await;

        if let Some(request) = pending.get_mut(hash) {
            request.total_chunks = Some(chunk_count);

            info!(
                hash = %hash,
                provider = %provider,
                size = size_bytes,
                chunks = chunk_count,
                "Provider announced for pending request"
            );

            // For small blobs, request chunk delivery via signals
            if size_bytes <= MAX_SIGNAL_CHUNK_SIZE as u64 * 4 {
                // Request will be handled by provider sending ChunkDelivery signals
                return Ok(None);
            }

            // For large blobs, we'd negotiate a direct transfer
            // This would involve WebRTC or direct TCP connection
            warn!(hash = %hash, "Large blob transfer not yet implemented");
        }

        Ok(None)
    }

    /// Handle incoming chunk
    async fn handle_chunk_delivery(
        &self,
        hash: &str,
        chunk_index: u32,
        total_chunks: u32,
        data: Vec<u8>,
    ) -> Result<Option<BlobSignal>, StorageError> {
        let mut pending = self.pending_requests.write().await;

        if let Some(request) = pending.get_mut(hash) {
            request.received_chunks.insert(chunk_index, data);
            request.total_chunks = Some(total_chunks);

            // Check if we have all chunks
            if request.received_chunks.len() as u32 == total_chunks {
                info!(hash = %hash, chunks = total_chunks, "All chunks received, reassembling");

                // Reassemble and store
                let mut full_data = Vec::new();
                for i in 0..total_chunks {
                    if let Some(chunk) = request.received_chunks.get(&i) {
                        full_data.extend_from_slice(chunk);
                    } else {
                        return Err(StorageError::ChunkMissing {
                            hash: hash.to_string(),
                            index: i,
                        });
                    }
                }

                // Verify hash
                let computed_hash = BlobStore::compute_hash(&full_data);
                if computed_hash != hash {
                    return Err(StorageError::HashMismatch {
                        expected: hash.to_string(),
                        actual: computed_hash,
                    });
                }

                // Store
                self.store.store(&full_data).await?;
                pending.remove(hash);

                info!(hash = %hash, size = full_data.len(), "Blob received and stored");
            }
        } else {
            debug!(hash = %hash, "Received chunk for unknown request");
        }

        Ok(None)
    }

    /// Handle direct transfer initiation
    async fn handle_transfer_init(
        &self,
        hash: &str,
        size_bytes: u64,
        transfer_id: &str,
        _connection_info: &str,
    ) -> Result<Option<BlobSignal>, StorageError> {
        // TODO: Implement WebRTC or direct TCP transfer for large blobs
        warn!(
            hash = %hash,
            size = size_bytes,
            transfer_id = %transfer_id,
            "Direct transfer requested but not yet implemented"
        );

        Ok(None)
    }

    /// Request a blob from the network
    pub async fn request_blob(&self, hash: &str) -> Result<(), StorageError> {
        // Check if we already have it
        if self.store.exists(hash).await {
            debug!(hash = %hash, "Already have blob, skipping request");
            return Ok(());
        }

        // Add to pending requests
        {
            let mut pending = self.pending_requests.write().await;
            pending.insert(
                hash.to_string(),
                PendingRequest {
                    hash: hash.to_string(),
                    requester: self.my_agent_id.clone(),
                    received_chunks: HashMap::new(),
                    total_chunks: None,
                    created_at: std::time::Instant::now(),
                },
            );
        }

        info!(hash = %hash, "Requesting blob from network");

        // The actual signal emission would happen through Holochain client
        // Caller should emit BlobSignal::Request through the conductor

        Ok(())
    }

    /// Provide a blob to a requester
    pub async fn provide_blob(
        &self,
        hash: &str,
        _requester: &str,
    ) -> Result<Vec<BlobSignal>, StorageError> {
        let data = self.store.get(hash).await?;
        let size = data.len();

        if size <= MAX_SIGNAL_CHUNK_SIZE {
            // Send as single chunk
            Ok(vec![BlobSignal::ChunkDelivery {
                hash: hash.to_string(),
                chunk_index: 0,
                total_chunks: 1,
                data,
            }])
        } else if size <= MAX_SIGNAL_CHUNK_SIZE * 10 {
            // Send as multiple chunks via signals
            let chunk_count = size.div_ceil(MAX_SIGNAL_CHUNK_SIZE);
            let mut signals = Vec::with_capacity(chunk_count);

            for (i, chunk) in data.chunks(MAX_SIGNAL_CHUNK_SIZE).enumerate() {
                signals.push(BlobSignal::ChunkDelivery {
                    hash: hash.to_string(),
                    chunk_index: i as u32,
                    total_chunks: chunk_count as u32,
                    data: chunk.to_vec(),
                });
            }

            Ok(signals)
        } else {
            // Too large for signal-based transfer, need direct connection
            Ok(vec![BlobSignal::TransferInit {
                hash: hash.to_string(),
                size_bytes: size as u64,
                transfer_id: uuid::Uuid::new_v4().to_string(),
                connection_info: "TODO: WebRTC or TCP".to_string(),
            }])
        }
    }

    /// Clean up old pending requests
    pub async fn cleanup_stale_requests(&self, max_age_secs: u64) {
        let mut pending = self.pending_requests.write().await;
        let now = std::time::Instant::now();

        pending.retain(|hash, request| {
            let age = now.duration_since(request.created_at).as_secs();
            if age > max_age_secs {
                warn!(hash = %hash, age_secs = age, "Cleaning up stale blob request");
                false
            } else {
                true
            }
        });
    }
}

// =============================================================================
// InfrastructureSignal — projection of infrastructure DNA post-commit signals
// =============================================================================
//
// Mirrors the DNA-side `InfrastructureSignal` enum (see
// `elohim/holochain/dna/infrastructure/zomes/infrastructure/src/lib.rs`).
// Fields that are `AgentPubKey` or `ActionHash` on the DNA side serialize as
// base64 `String`s over the wire — the storage mirror declares them as
// `String` so the projection layer needs no integrity-crate dependency.
//
// Serde tagging (`tag = "type", content = "payload"`) MUST match the DNA
// side exactly; variant names MUST match exactly.

/// A HoloHash-bearing signal field (AgentPubKey / ActionHash), normalized to
/// the canonical holochain base64 form (`u` + base64url-no-pad over the raw
/// 39 bytes).
///
/// WHY THIS EXISTS (2026-06-12 peer-statuses root cause): the conductor
/// encodes app signals with MessagePack (`ExternIO`), and `holo_hash` types
/// serialize there as raw BYTE ARRAYS — not base64 strings. The old mirror
/// declared these fields `String`, so every real `PeerStatusRecorded` signal
/// failed to decode and was dropped at `debug!` level: heartbeat publishes
/// succeeded, `post_commit` emitted, and `peer_statuses` stayed empty,
/// silently. (The old `rmp → serde_json::Value` pre-pass died even earlier:
/// `serde_json::Value` has no byte-array representation.) This type accepts
/// bytes, byte-seq, or string and normalizes — JSON-context producers (tests,
/// fixtures) keep working.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct HoloHashB64(pub String);

impl From<&str> for HoloHashB64 {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for HoloHashB64 {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl HoloHashB64 {
    fn from_raw_bytes(bytes: &[u8]) -> Self {
        use base64::engine::general_purpose::URL_SAFE_NO_PAD;
        use base64::Engine;
        Self(format!("u{}", URL_SAFE_NO_PAD.encode(bytes)))
    }

    /// The canonical base64 string this hash normalizes to. Used by projection
    /// handlers that thread the anchor through `&str` APIs (e.g. the REA
    /// `upsert_with_anchor` family, which takes `Option<&str>`).
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for HoloHashB64 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for HoloHashB64 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct V;
        impl<'de> serde::de::Visitor<'de> for V {
            type Value = HoloHashB64;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a holo-hash as bytes or base64 string")
            }

            fn visit_bytes<E: serde::de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
                Ok(HoloHashB64::from_raw_bytes(v))
            }

            fn visit_byte_buf<E: serde::de::Error>(self, v: Vec<u8>) -> Result<Self::Value, E> {
                Ok(HoloHashB64::from_raw_bytes(&v))
            }

            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
                Ok(HoloHashB64(v.to_string()))
            }

            fn visit_string<E: serde::de::Error>(self, v: String) -> Result<Self::Value, E> {
                Ok(HoloHashB64(v))
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let mut bytes = Vec::with_capacity(seq.size_hint().unwrap_or(39));
                while let Some(b) = seq.next_element::<u8>()? {
                    bytes.push(b);
                }
                Ok(HoloHashB64::from_raw_bytes(&bytes))
            }
        }
        deserializer.deserialize_any(V)
    }
}

/// Storage-side mirror of the DNA `InfrastructureSignal` enum.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum InfrastructureSignal {
    /// PeerStatus DHT entry was recorded — project into SQLite.
    PeerStatusRecorded {
        peer_id: HoloHashB64,
        status: String,
        general_pool_member: bool,
        accepting_stewardship_reserves: bool,
        archetype_class: Option<String>,
        timestamp: i64,
        action_hash: HoloHashB64,
    },
}

/// Mirror-variant names: a decode failure on one of these tags is a REAL
/// projection miss (loud); any other tag is a foreign signal sharing the app
/// interface (quiet).
const INFRA_MIRROR_VARIANTS: &[&str] = &["PeerStatusRecorded"];

/// Cumulative count of REAL infrastructure decode misses this process.
static INFRA_DECODE_MISSES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Cumulative REAL decode misses (mirror-variant tag present but the typed
/// decode failed). Exposed for status surfaces / tests.
pub fn infrastructure_decode_miss_count() -> u64 {
    INFRA_DECODE_MISSES.load(std::sync::atomic::Ordering::Relaxed)
}

/// Outcome classification for a non-decodable app-signal payload.
#[derive(Debug)]
pub enum SignalDecodeMiss {
    /// The payload carries a different signal family's tag (lamad content,
    /// REA, …) — expected on a shared app interface, not an error.
    ForeignSignal { type_tag: String },
    /// The tag IS one of ours but the payload failed the typed decode —
    /// a real wire-shape regression. LOUD.
    InfraShapeMismatch { type_tag: String, error: String },
    /// As `InfraShapeMismatch`, but for a `MishpatSignal` mirror variant — a
    /// real mishpat wire-shape regression. LOUD.
    MishpatShapeMismatch { type_tag: String, error: String },
    /// As `InfraShapeMismatch`, but for a `ReaProjectionSignal` mirror variant
    /// — a real REA wire-shape regression. LOUD.
    ReaShapeMismatch { type_tag: String, error: String },
    /// As `InfraShapeMismatch`, but for the elohim-content `ContentCommitted`
    /// mirror variant — a real content wire-shape regression. LOUD.
    ElohimContentShapeMismatch { type_tag: String, error: String },
    /// Not even the tag could be read.
    Undecodable { error: String },
}

/// Decode a conductor app-signal payload (MessagePack `ExternIO` bytes) into
/// the typed mirror, classifying failures so the subscriber can be loud about
/// real misses without spamming on foreign signals.
pub fn decode_infrastructure_signal(
    bytes: &[u8],
) -> Result<InfrastructureSignal, SignalDecodeMiss> {
    match rmp_serde::from_slice::<InfrastructureSignal>(bytes) {
        Ok(signal) => Ok(signal),
        Err(typed_err) => {
            /// Tag-only peek: serde skips the payload via IgnoredAny (which,
            /// unlike `serde_json::Value`, tolerates msgpack byte arrays).
            #[derive(Deserialize)]
            struct TagOnly {
                #[serde(rename = "type")]
                type_tag: String,
            }
            match rmp_serde::from_slice::<TagOnly>(bytes) {
                Ok(tag) if INFRA_MIRROR_VARIANTS.contains(&tag.type_tag.as_str()) => {
                    INFRA_DECODE_MISSES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    Err(SignalDecodeMiss::InfraShapeMismatch {
                        type_tag: tag.type_tag,
                        error: typed_err.to_string(),
                    })
                }
                Ok(tag) => Err(SignalDecodeMiss::ForeignSignal {
                    type_tag: tag.type_tag,
                }),
                Err(tag_err) => Err(SignalDecodeMiss::Undecodable {
                    error: format!("typed: {typed_err}; tag peek: {tag_err}"),
                }),
            }
        }
    }
}

/// Dispatch an `InfrastructureSignal` into the SQLite projection.
///
/// New variants add a match arm here and a call into the corresponding
/// `db::*` module. Keep this function thin — the per-entity projection
/// logic belongs next to its Diesel model.
pub fn handle_signal(
    conn: &mut diesel::sqlite::SqliteConnection,
    signal: InfrastructureSignal,
) -> Result<(), StorageError> {
    match signal {
        InfrastructureSignal::PeerStatusRecorded {
            peer_id,
            status,
            general_pool_member,
            accepting_stewardship_reserves,
            archetype_class,
            timestamp,
            action_hash,
        } => {
            let row = crate::db::peer_statuses::PeerStatusRow {
                peer_id: peer_id.0,
                status,
                general_pool_member: general_pool_member as i32,
                accepting_stewardship_reserves: accepting_stewardship_reserves as i32,
                archetype_class,
                timestamp,
                dht_anchor_hash: action_hash.0,
                updated_at: chrono::Utc::now().timestamp_micros(),
            };
            crate::db::peer_statuses::upsert(conn, &row)?;
            Ok(())
        }
    }
}

// =============================================================================
// MishpatSignal — projection of mishpat DNA post-commit signals
// =============================================================================
//
// Mirrors the DNA-side `MishpatSignal` enum (mishpat DNA, `mishpat` zome).
// Fields that are `AgentPubKey`/`ActionHash`/`EntryHash` on the DNA side
// serialize on the conductor wire (MessagePack `ExternIO`) as raw 39-BYTE
// ARRAYS, NOT base64 strings — so they MUST be mirrored as `HoloHashB64`
// (not `String`). The old `String` mirror + the `rmp → serde_json::Value`
// pre-pass dropped every real signal at `debug!` level: the exact dark-surface
// class fixed for `InfrastructureSignal` in d33b0e1f5 (2026-06-12). The
// embedded `Commitment` / entry sub-structs are all-`String` on the DNA side
// (verified field-by-field against `mishpat_integrity`), so they need no
// HoloHashB64 treatment.
//
// Serde tagging (`tag = "type", content = "payload"`) MUST match the DNA
// side exactly. The entry sub-struct uses snake_case to match the DNA wire
// format (HDK serialises Rust structs as snake_case by default).

/// Storage-side mirror of the GateDecisionAttestation entry fields as they
/// arrive in the signal payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateDecisionAttestationEntry {
    pub decision_id: String,
    pub phase: String,
    pub elohim_id: String,
    pub elohim_substance_cid: String,
    pub gate_name: String,
    pub gate_process_cid: String,
    pub request_ref_json: String,
    pub decision: String,
    pub reasoning_json: String,
    pub context_summary_cid: String,
    pub decided_at: String,
    pub universal_band_cid: String,
}

/// Storage-side mirror of the GateDecisionChallenge entry fields as they
/// arrive in the signal payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateDecisionChallengeEntry {
    pub challenge_id: String,
    pub challenged_decision_cid: String,
    pub challenger_id: String,
    pub grounds: String,
    pub summary: String,
    pub evidence_refs: String,
    pub filed_at: String,
    pub reach: String,
}

/// Storage-side mirror of the ChallengeOutcome entry fields as they
/// arrive in the signal payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengeOutcomeEntry {
    pub outcome_id: String,
    pub challenge_cid: String,
    pub verdict: String,
    pub reviewer_consensus: String,
    pub reasoning_json: String,
    pub decided_at: String,
    pub indemnification_actions_json: String,
}

/// Storage-side mirror of the DNA `MishpatSignal` enum.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum MishpatSignal {
    /// GateDecisionAttestation DHT entry was recorded — project into SQLite.
    ///
    /// NOTE (Stage C): the mishpat DNA no longer emits this variant — the
    /// GateDecisionAttestation entry type moved to the elohim DNA. The mirror
    /// arm is retained for back-compat dispatch/tests; the live wire only
    /// carries `CommitmentCommitted` and `ChallengeOutcomeCreated`.
    GateDecisionCreated {
        action_hash: HoloHashB64,
        entry_hash: HoloHashB64,
        author: HoloHashB64,
        entry: GateDecisionAttestationEntry,
    },
    /// GateDecisionChallenge DHT entry was recorded — project into SQLite.
    ///
    /// NOTE (Stage C): no longer emitted by the mishpat DNA (entry type moved
    /// to elohim DNA); mirror arm retained for back-compat.
    GateDecisionChallengeCreated {
        action_hash: HoloHashB64,
        entry_hash: HoloHashB64,
        author: HoloHashB64,
        entry: GateDecisionChallengeEntry,
    },
    /// ChallengeOutcome DHT entry was recorded — project into SQLite.
    ChallengeOutcomeCreated {
        action_hash: HoloHashB64,
        entry_hash: HoloHashB64,
        author: HoloHashB64,
        entry: ChallengeOutcomeEntry,
    },
    /// Commitment DHT entry was recorded — project into mishpat_commitments (Slice-2a T5).
    ///
    /// Emitted by the mishpat coordinator `post_commit` hook for each committed
    /// `Commitment` entry. `entry_hash` is the content-addressed identity used
    /// as `cid` in the projection row; `action_hash` becomes `dht_anchor_hash`.
    CommitmentCommitted {
        action_hash: HoloHashB64,
        entry_hash: HoloHashB64,
        author: HoloHashB64,
        commitment: crate::mishpat_projection::CommitmentPayload,
    },
}

/// Mirror-variant names: a decode failure on one of these tags is a REAL
/// projection miss (loud); any other tag is a foreign signal sharing the app
/// interface (quiet).
///
/// The live mishpat DNA only emits `CommitmentCommitted` and
/// `ChallengeOutcomeCreated` (the gate-decision variants moved to the elohim
/// DNA in Stage C); the two retained mirror tags are listed so a back-compat
/// replay still classifies as a real miss rather than a foreign signal.
const MISHPAT_MIRROR_VARIANTS: &[&str] = &[
    "CommitmentCommitted",
    "ChallengeOutcomeCreated",
    "GateDecisionCreated",
    "GateDecisionChallengeCreated",
];

/// Cumulative count of REAL mishpat decode misses this process.
static MISHPAT_DECODE_MISSES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Cumulative REAL mishpat decode misses (mirror-variant tag present but the
/// typed decode failed). Exposed for status surfaces / tests.
pub fn mishpat_decode_miss_count() -> u64 {
    MISHPAT_DECODE_MISSES.load(std::sync::atomic::Ordering::Relaxed)
}

/// Decode a conductor app-signal payload (MessagePack `ExternIO` bytes) into
/// the typed `MishpatSignal` mirror, classifying failures so the subscriber
/// can be loud about real misses without spamming on foreign signals.
///
/// WHY (the 2026-06-12 dark-`CommitmentCommitted` root cause): the conductor
/// encodes signals with MessagePack, where `ActionHash`/`EntryHash`/
/// `AgentPubKey` serialize as raw 39-BYTE ARRAYS. The old subscriber pre-
/// decoded into `serde_json::Value` (no byte-array representation) and the
/// mirror declared those fields `String`, so EVERY real `CommitmentCommitted`
/// signal failed decode and was dropped at `debug!` — the Epic B provide
/// projection (4626f820b) was dead code in production. Identical class to the
/// `InfrastructureSignal` fix in d33b0e1f5.
pub fn decode_mishpat_signal(bytes: &[u8]) -> Result<MishpatSignal, SignalDecodeMiss> {
    match rmp_serde::from_slice::<MishpatSignal>(bytes) {
        Ok(signal) => Ok(signal),
        Err(typed_err) => {
            // Tag-only peek: serde skips the payload via IgnoredAny (which,
            // unlike `serde_json::Value`, tolerates msgpack byte arrays).
            #[derive(Deserialize)]
            struct TagOnly {
                #[serde(rename = "type")]
                type_tag: String,
            }
            match rmp_serde::from_slice::<TagOnly>(bytes) {
                Ok(tag) if MISHPAT_MIRROR_VARIANTS.contains(&tag.type_tag.as_str()) => {
                    MISHPAT_DECODE_MISSES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    Err(SignalDecodeMiss::MishpatShapeMismatch {
                        type_tag: tag.type_tag,
                        error: typed_err.to_string(),
                    })
                }
                Ok(tag) => Err(SignalDecodeMiss::ForeignSignal {
                    type_tag: tag.type_tag,
                }),
                Err(tag_err) => Err(SignalDecodeMiss::Undecodable {
                    error: format!("typed: {typed_err}; tag peek: {tag_err}"),
                }),
            }
        }
    }
}

/// Dispatch a `MishpatSignal` into the SQLite projection.
///
/// New variants add a match arm here and a call into the corresponding
/// `db::*` module. Keep this function thin — the per-entity projection
/// logic belongs next to its Diesel model.
///
/// ## Doorway signal-absorption verdict (Task 4.3)
///
/// Doorway's `ProjectionEngine` (`doorway/doorway-service/src/projection/engine.rs`)
/// is type-agnostic — it processes `ProjectionSignal` (doc_type, id, data, search_tokens,
/// invalidates, ttl_secs) from the Holochain conductor app WebSocket. It has zero
/// knowledge of "GateDecisionAttestation" or any other specific entry type.
///
/// The `MishpatSignal` enum is elohim-storage's own internal relay format. This
/// function writes gate decisions directly into SQLite. Doorway's projection cache
/// (MongoDB) is a separate, parallel pathway for DHT content reads — it does NOT
/// need extending to absorb mishpat signals. No doorway code changes are required.
pub fn handle_mishpat_signal(
    conn: &mut diesel::sqlite::SqliteConnection,
    app_id: &str,
    signal: MishpatSignal,
) -> Result<(), StorageError> {
    match signal {
        MishpatSignal::GateDecisionCreated {
            action_hash,
            entry_hash: _,
            author: _,
            entry,
        } => {
            // Phase-2a consolidation: gate decisions project into the UNIFIED
            // `attestations` table as `attestation:gate-decision`, mirroring the
            // elohim-DNA bridge `create_gate_decision_attestation`
            // (mishpat/zomes/mishpat/src/lib.rs ~1426). The legacy per-app
            // gate-decision projection table was dropped in migration
            // 2026-05-12-100300. The full gate-decision shape (phase, gate_name,
            // gate_process_cid, request_ref_json, decision, decided_at,
            // elohim_substance_cid, universal_band_cid) lives in `evidence_json`;
            // `reasoning_json` lives under `proof_evidence_json` (proof_class=audit).
            let row =
                crate::db::attestations::attestation_row_from_gate_decision(&entry, &action_hash.0);
            crate::db::attestations::upsert(conn, &row)?;
            Ok(())
        }
        MishpatSignal::GateDecisionChallengeCreated {
            action_hash,
            entry_hash: _,
            author: _,
            entry,
        } => {
            let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
            let row = crate::db::gate_decision_challenges::GateDecisionChallengeRow {
                app_id: app_id.to_string(),
                challenge_id: entry.challenge_id,
                challenged_decision_cid: entry.challenged_decision_cid,
                challenger_id: entry.challenger_id,
                grounds: entry.grounds,
                summary: entry.summary,
                evidence_refs: entry.evidence_refs,
                filed_at: entry.filed_at,
                reach: entry.reach,
                dht_anchor_hash: action_hash.0,
                created_at: now.clone(),
                updated_at: now,
            };
            crate::db::gate_decision_challenges::upsert(conn, &row)?;
            Ok(())
        }
        MishpatSignal::ChallengeOutcomeCreated {
            action_hash,
            entry_hash: _,
            author: _,
            entry,
        } => {
            let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
            let row = crate::db::challenge_outcomes::ChallengeOutcomeRow {
                app_id: app_id.to_string(),
                outcome_id: entry.outcome_id,
                challenge_cid: entry.challenge_cid,
                verdict: entry.verdict,
                reviewer_consensus: entry.reviewer_consensus,
                reasoning_json: entry.reasoning_json,
                decided_at: entry.decided_at,
                indemnification_actions_json: entry.indemnification_actions_json,
                dht_anchor_hash: action_hash.0,
                created_at: now.clone(),
                updated_at: now,
            };
            crate::db::challenge_outcomes::upsert(conn, &row)?;
            Ok(())
        }
        MishpatSignal::CommitmentCommitted {
            action_hash,
            entry_hash,
            author: _,
            commitment,
        } => {
            // Delegate to the dedicated projection handler in mishpat_projection.
            // That module owns the payload-parse logic. The router returns a
            // CommitmentProjection: most actions Upsert a new row; a
            // revokes-commitment action Revokes a target row (sets revoked_at).
            // We stay synchronous and conn-oriented (matching the other arms),
            // avoiding an async boundary.
            match crate::mishpat_projection::parse_commitment_payload(
                &commitment.action,
                &commitment.payload_json,
                entry_hash.as_str(),
                action_hash.as_str(),
            ) {
                Ok(crate::mishpat_projection::CommitmentProjection::Upsert(new_row)) => {
                    // Epic B side-projection: a notarized `replicates-commons`
                    // CONTENT commitment is a standing offer to provide content
                    // at commons reach. The resilience snapshot + PeerSelection
                    // read that as a `rea_commitments` provide / `content:<reach>`
                    // row, which the production path never wrote (only test_util
                    // did). Derive it from this DHT-anchored event BEFORE the
                    // mishpat_commitments write consumes `new_row`.
                    if let Some(p) = crate::mishpat_projection::provide_projection_for(&new_row) {
                        match crate::db::rea_commitments::record_provide_from_content_commitment(
                            conn,
                            app_id,
                            &p.provider,
                            &p.reach,
                            p.dht_anchor_hash.as_deref(),
                        ) {
                            Ok(inserted) => {
                                if inserted {
                                    tracing::info!(
                                        provider = %p.provider,
                                        reach = %p.reach,
                                        action_hash = %action_hash,
                                        "handle_mishpat_signal: replicates-commons → active provide row (rea_commitments)"
                                    );
                                }
                            }
                            Err(e) => {
                                // Non-fatal: the authoritative mishpat_commitments
                                // projection below must still land. A failed
                                // provide side-projection self-heals on the next
                                // commitment for the same provider/reach (idempotent).
                                tracing::warn!(
                                    error = %e,
                                    provider = %p.provider,
                                    "handle_mishpat_signal: provide side-projection failed (non-fatal; retries on next commitment)"
                                );
                            }
                        }
                    }
                    crate::db::mishpat_commitments::upsert_with_anchor(conn, new_row)
                        .map_err(|e| StorageError::Database(e.to_string()))?;
                    tracing::info!(
                        cid = %entry_hash,
                        action_hash = %action_hash,
                        "handle_mishpat_signal: CommitmentCommitted projected → mishpat_commitments"
                    );
                }
                Ok(crate::mishpat_projection::CommitmentProjection::UpsertLens(new_lens)) => {
                    // author-lens projects into the A-class `lenses` table (NOT
                    // mishpat_commitments). Anchor-preserving upsert; cid = entry_hash.
                    crate::db::lenses::upsert_with_anchor(conn, new_lens)
                        .map_err(|e| StorageError::Database(e.to_string()))?;
                    tracing::info!(
                        cid = %entry_hash,
                        action_hash = %action_hash,
                        "handle_mishpat_signal: CommitmentCommitted projected → lenses"
                    );
                }
                Ok(crate::mishpat_projection::CommitmentProjection::UpsertIdentityHead(
                    new_head,
                )) => {
                    // binds-identity projects into the A-class `identity_heads` table
                    // (NOT mishpat_commitments). Anchor-preserving upsert; cid =
                    // entry_hash. Feeds the did:elohim head resolver's controllers +
                    // lineage (Wave C1).
                    crate::db::identity_heads::upsert_with_anchor(conn, new_head)
                        .map_err(|e| StorageError::Database(e.to_string()))?;
                    tracing::info!(
                        cid = %entry_hash,
                        action_hash = %action_hash,
                        "handle_mishpat_signal: CommitmentCommitted projected → identity_heads"
                    );
                }
                Ok(crate::mishpat_projection::CommitmentProjection::Revoke {
                    target_cid,
                    signed_at,
                }) => {
                    // A revoke target is either a `mishpat_commitments` row OR an
                    // A-class lens (distinct tables over the same CID space) — set
                    // both; each no-ops (0 rows) when the CID lives in the other
                    // table. Without the lenses call a revoke targeting a lens
                    // silently matched 0 rows and the lens stayed live in the market.
                    let affected_commitments = crate::db::mishpat_commitments::set_revoked_at(
                        conn,
                        &target_cid,
                        &signed_at,
                    )
                    .map_err(|e| StorageError::Database(e.to_string()))?;
                    let affected_lenses =
                        crate::db::lenses::set_revoked_at(conn, &target_cid, &signed_at)
                            .map_err(|e| StorageError::Database(e.to_string()))?;
                    // A revoke may also target an identity_heads row (same CID space,
                    // distinct table) — no-ops (0 rows) when the CID lives elsewhere.
                    let affected_identity_heads =
                        crate::db::identity_heads::set_revoked_at(conn, &target_cid, &signed_at)
                            .map_err(|e| StorageError::Database(e.to_string()))?;
                    tracing::info!(
                        target_cid = %target_cid,
                        action_hash = %action_hash,
                        affected_commitments,
                        affected_lenses,
                        affected_identity_heads,
                        "handle_mishpat_signal: revokes-commitment projected → set_revoked_at (commitments + lenses + identity_heads)"
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        action_hash = %action_hash,
                        "handle_mishpat_signal: CommitmentCommitted payload parse failed — skipped"
                    );
                }
            }
            Ok(())
        }
    }
}

// =============================================================================
// RecoveryV2Signal — projection of imagodei DNA post-commit signals
// =============================================================================
//
// Mirrors the DNA-side `RecoveryV2Signal` enum (imagodei coordinator zome).
// Fields that are `AgentPubKey` or `ActionHash` on the DNA side serialize as
// base64 `String`s over the wire. `Timestamp` serializes as microseconds i64.
// `RecoveryAuthorityKind` serializes as a serde-internally-tagged enum with
// PascalCase variant names (no rename_all on the DNA side).
//
// Serde tag must match the DNA side exactly: `tag = "type"` (internally tagged,
// no `content` wrapper — the coordinator uses `#[serde(tag = "type")]`).

/// Storage-side mirror of the RecoveryRequest fields as they arrive in the signal.
/// `proposed_authority` is stored as two columns: `kind` (discriminator string)
/// and `json` (variant-specific payload). This mirrors the view layer's
/// `proposed_authority_kind` + `proposed_authority_json` split.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryRequestPayload {
    pub human_agent_pubkey: String,
    pub new_agent_pubkey: String,
    pub hosting_doorway_pubkey: String,
    pub proposed_authority: serde_json::Value,
    pub request_nonce: Vec<u8>,
    pub human_id: Option<String>,
    pub required_witness_count: u32,
    /// Holochain Timestamp — serializes as microseconds i64.
    pub created_at: serde_json::Value,
}

/// Storage-side mirror of the HumanityWitness fields as they arrive in the
/// IntimateWitnessSubmitted signal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanityWitnessPayload {
    pub human_id: String,
    pub witness_agent_id: String,
    pub attestation_type: String,
    pub note: Option<String>,
    /// Holochain Timestamp — serializes as microseconds i64.
    pub issued_at: serde_json::Value,
    pub revoked_at: Option<serde_json::Value>,
}

/// Storage-side mirror of the KeyRotation fields as they arrive in the signal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyRotationPayload {
    pub human_agent_pubkey: String,
    pub new_agent_pubkey: String,
    pub superseded_agent_pubkey: String,
    pub recovery_request_hash: String,
    pub authority: serde_json::Value,
    /// Holochain Timestamp — serializes as microseconds i64.
    pub rotated_at: serde_json::Value,
}

/// Storage-side mirror of the DNA `RecoveryV2Signal` enum.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RecoveryV2Signal {
    RecoveryRequestCreated {
        action_hash: String,
        request: RecoveryRequestPayload,
    },
    IntimateWitnessSubmitted {
        action_hash: String,
        request_hash: String,
        witness: HumanityWitnessPayload,
        witness_agent_id: String,
    },
    KeyRotationCommitted {
        action_hash: String,
        rotation: KeyRotationPayload,
    },
    // M4: fast-path revocation signals. Field order and names match DNA side exactly.
    KeyRevocationRequested {
        id: String,
        human_id: String,
        revoked_key: String,
        reason: String,
        trigger_type: String,
        initiated_by: String,
        required_votes: u32,
        current_votes: u32,
        threshold_reached: bool,
        effective_at: Option<String>,
        created_at: String,
    },
    RevocationVoteSubmitted {
        id: String,
        revocation_id: String,
        steward_id: String,
        approved: bool,
        attestation: String,
        voted_at: String,
        current_votes: u32,
        required_votes: u32,
        threshold_now_reached: bool,
    },
    KeyRevocationEffective {
        revocation_id: String,
        revoked_key: String,
        human_id: String,
        effective_at: String,
        triggering_vote_id: Option<String>,
    },
}

// =============================================================================
// T18: KeyRevocationEnvelope — EPR-shape signal mirror
// =============================================================================
//
// Storage-side mirror of the imagodei coordinator's `DnaSignal::KeyRevocation`
// EPR envelope. Used by `holochain_app_signal::try_decode_and_translate`
// (Step 2c) to decode and signature-verify the new envelope before translating
// into the existing `DnaSignal::KeyRevocation(KeyRevocationSignal)`.
//
// These types MUST remain structurally identical to the zome-side
// `KeyRevocationEnvelope` / `KeyRevocationMetadata` in
// `imagodei/zomes/imagodei/src/lib.rs`. The `canonical_envelope_bytes` function
// below MUST match the zome-side implementation exactly.
//
// Do NOT merge into the zome crate — `elohim-storage` cannot depend on
// Holochain DNA crates (wasm32 incompatibility). Duplication is intentional
// and load-bearing; the comment is the contract.

/// Storage-side mirror of `imagodei::KeyRevocationEnvelope`.
///
/// Deserialized from `DnaSignal::KeyRevocation` wire payloads emitted by the
/// imagodei coordinator zome (T18+). The `type` discriminator on the wire is
/// `"keyRevocation"` (serde tag from `imagodei::DnaSignal`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyRevocationEnvelope {
    pub attestation_kind: String,
    pub subject_cid: String,
    pub issuer: String,
    pub issued_at: String,
    pub signature: String,
    pub metadata: KeyRevocationMetadata,
    #[serde(default)]
    pub relay_chain: Vec<serde_json::Value>,
}

/// Storage-side mirror of `imagodei::KeyRevocationMetadata`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyRevocationMetadata {
    pub revocation_id: String,
    pub revoked_pubkey: String,
    pub agent_cid: String,
    pub compromise_at: String,
    pub effective_at: String,
    pub triggering_revocation_id: Option<String>,
    pub supersedes_cid: Option<String>,
}

/// Top-level discriminator wrapper for `imagodei::DnaSignal`.
///
/// The zome emits `DnaSignal::KeyRevocation(envelope)` with serde tag `"type"`
/// and camelCase variant name `"keyRevocation"`. We only decode the
/// `KeyRevocation` variant here; other variants remain unrecognised and fall
/// through to Step 3 in `try_decode_and_translate`.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ImagodeiDnaSignal {
    KeyRevocation(KeyRevocationEnvelope),
    // Other variants (KeyRotation, AgentPeerBinding migrations) not yet decoded
    // here; they remain on the RecoveryV2Signal / ImagodeiSignal path.
}

// =============================================================================
// ImagodeiSignal — mirror of the DNA-side enum (A.11 signal stream)
// =============================================================================
//
// Wire format: the imagodei coordinator emits via `emit_signal(ImagodeiSignal::...)`
// which serializes with `#[serde(tag = "type", content = "payload")]`.
//
// The `AgentPeerBindingCreated` variant is the one consumed by
// `HolochainAppSignalStream` (Task A.11) to drive `DnaSignal::AgentPeerBinding`.
// Other variants (HumanCommitted, etc.) are decoded but not yet consumed by the
// reconcile controller; they are silently skipped after decode.
//
// Holochain Timestamp fields (valid_from, valid_until) serialize as microseconds
// i64 from the DNA side. We store them as `i64` and convert to DateTime<Utc> in
// the translator.

/// Storage-side mirror of the `AgentPeerBinding` entry as it arrives in the
/// `AgentPeerBindingCreated` signal payload.
///
/// Mirrors `imagodei_integrity::AgentPeerBinding`. `valid_from` / `valid_until`
/// are Holochain Timestamps (microseconds i64). `device_archetype` is a
/// snake_case string (`node`, `desktop`, `mobile`, `steward`).
/// `signature` is present on the wire but not needed by the translator;
/// `superseded_by` is extracted by `translate_imagodei`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPeerBindingPayload {
    pub peer_id: String,
    pub agent_cid: String,
    /// Holochain Timestamp (microseconds i64).
    pub valid_from: i64,
    /// Holochain Timestamp (microseconds i64). `None` = no declared expiry.
    pub valid_until: Option<i64>,
    /// snake_case device archetype string.
    pub device_archetype: String,
    // signature is present on the wire but unused by the translator.
    // superseded_by is extracted by translate_imagodei.
    #[serde(default)]
    pub signature: Vec<u8>,
    #[serde(default)]
    pub superseded_by: Option<serde_json::Value>,
}

/// Storage-side mirror of the imagodei DNA `ImagodeiSignal` enum.
///
/// The DNA side uses `#[serde(tag = "type", content = "payload")]`. This mirror
/// uses the same structure. Only `AgentPeerBindingCreated` is consumed by
/// `HolochainAppSignalStream`; other variants are decoded and silently skipped.
///
/// `action_hash` in each variant is an `ActionHash` on the DNA side, which
/// serializes as a base64 `String` over the app-signal wire.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum ImagodeiSignal {
    HumanCommitted {
        action_hash: String,
        // remaining fields unused — not decoded
        #[serde(flatten)]
        _rest: serde_json::Value,
    },
    AgentCommitted {
        action_hash: String,
        #[serde(flatten)]
        _rest: serde_json::Value,
    },
    RelationshipCommitted {
        action_hash: String,
        #[serde(flatten)]
        _rest: serde_json::Value,
    },
    AttestationCommitted {
        action_hash: String,
        #[serde(flatten)]
        _rest: serde_json::Value,
    },
    /// Consumed by `HolochainAppSignalStream` → `DnaSignal::AgentPeerBinding`.
    AgentPeerBindingCreated {
        action_hash: String,
        binding: AgentPeerBindingPayload,
    },
    /// Emitted by the imagodei coordinator `post_commit` when a `Collective`
    /// entry is committed (Wave 2 T4 — prioritizer epic).
    ///
    /// Consumed by `translate_imagodei` → `DnaSignal::CollectiveProjected` →
    /// `ReconcileController::on_collective_projected` → upsert `collectives`
    /// row with `collective_cid` = `collective:{action_hash}`.
    ///
    /// Ordering guarantee: the DNA emits `CollectiveCommitted` before
    /// `MembershipCommitted` in the same batch so the storage projector can
    /// upsert the Collective row before its memberships arrive.
    CollectiveCommitted {
        action_hash: String,
        entry_hash: String,
        collective: CollectivePayload,
        author: String,
    },
    /// Emitted by the imagodei coordinator `post_commit` when a `Membership`
    /// entry is committed (Wave 2 T4 — prioritizer epic).
    ///
    /// Consumed by `translate_imagodei` → `DnaSignal::MembershipProjected` →
    /// `ReconcileController::on_membership_projected` → upsert
    /// `collective_participations` keyed by `dht_anchor_hash = action_hash`.
    MembershipCommitted {
        action_hash: String,
        entry_hash: String,
        membership: MembershipPayload,
        author: String,
    },
}

// =============================================================================
// CollectivePayload / MembershipPayload
// =============================================================================
//
// Storage-side mirrors of the imagodei integrity `Collective` and `Membership`
// entry structs. Field names and types must match the DNA exactly — serde
// deserialises the signal payload over the wire and a name mismatch silently
// drops the signal.
//
// DNA source of truth: `imagodei_integrity::qahal`
// (`elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/qahal.rs`).

/// Mirror of `imagodei_integrity::Collective`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectivePayload {
    pub founder_agent_cid: String,
    pub charter: String,
    pub display_name: String,
    pub created_at_block_height: u64,
    pub salt: String,
    /// Non-null when this Collective was instantiated from a `CollabAgreement`.
    #[serde(default)]
    pub anchor_agreement_cid: Option<String>,
}

/// Mirror of `imagodei_integrity::MemberKind`.
///
/// Serializes as the enum variant name (e.g. `"Person"`, `"Collective"`,
/// `"ElohimAgent"`) matching the HDK `#[derive(Serialize, Deserialize)]` default.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MemberKindPayload {
    Person,
    Collective,
    ElohimAgent,
}

impl MemberKindPayload {
    /// Map to the storage `member_kind` string used in `collective_participations`.
    pub fn as_db_str(&self) -> &'static str {
        match self {
            MemberKindPayload::Person => "person",
            MemberKindPayload::Collective => "collective",
            MemberKindPayload::ElohimAgent => "elohim_agent",
        }
    }
}

/// Mirror of `imagodei_integrity::MembershipRole`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MembershipRolePayload {
    Steward,
    Contributor,
    Observer,
}

impl MembershipRolePayload {
    /// Map to the storage `role_context` string.
    pub fn as_db_str(&self) -> &'static str {
        match self {
            MembershipRolePayload::Steward => "steward",
            MembershipRolePayload::Contributor => "contributor",
            MembershipRolePayload::Observer => "observer",
        }
    }
}

/// Mirror of `imagodei_integrity::Membership`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MembershipPayload {
    pub member_cid: String,
    pub member_kind: MemberKindPayload,
    pub collective_cid: String,
    pub role: MembershipRolePayload,
    #[serde(default)]
    pub sponsor_cid: Option<String>,
    pub joined_at_block_height: u64,
    /// Set when the membership has been cleanly withdrawn.
    #[serde(default)]
    pub withdrawn_at_block_height: Option<u64>,
}

// =============================================================================
// M4 outbound reconciliation signal (P1 — imagodei.revocation_observed)
// =============================================================================

/// Outbound event emitted by the storage reconciliation controller after each
/// revocation projection write. Downstream consumers (M5 elohim defender,
/// Phase 4 GraphQL subscriptions) subscribe to `imagodei.revocation_observed`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ImagodeiReconciledEvent {
    RevocationObserved {
        revocation_id: String,
        revoked_key: String,
        human_id: String,
        /// "pending" | "effective-on-create" | "effective"
        status: String,
        observed_at: String,
    },
}

/// Emit a reconciled event to the `imagodei.revocation_observed` log target.
///
/// M4 implementation: best-effort log line. M5 wires this to a broadcast channel
/// for elohim-defender subscribers. The log target is the seam future consumers
/// will hook into.
fn emit_reconciled_signal(event: ImagodeiReconciledEvent) {
    tracing::info!(target: "imagodei.revocation_observed", ?event, "reconciled event");
}

// =============================================================================
// M4 mesh wire struct (RecoveryRevocationMessage)
// =============================================================================

// The canonical type is defined in p2p::recovery_revocation; re-export here so
// callers of signals.rs don't need to know the p2p module path.
pub use crate::p2p::recovery_revocation::RecoveryRevocationMessage;

/// Extract the authority kind discriminator from a `RecoveryAuthorityKind`
/// or `RecoveryAuthority` serde_json::Value (internally-tagged or plain variant).
fn extract_authority_kind(v: &serde_json::Value) -> String {
    // Internally-tagged: {"type": "IntimateQuorum", ...} or just "IntimateQuorum"
    if let Some(t) = v.get("type").and_then(|t| t.as_str()) {
        return t.to_string();
    }
    // Plain string variant (unit variants serialize as string)
    if let Some(s) = v.as_str() {
        return s.to_string();
    }
    "unknown".to_string()
}

/// Convert a Holochain Timestamp serde_json::Value to an ISO 8601 string.
/// HDK Timestamp serializes as microseconds i64 or as `{"secs": i64, "nanos": u32}`.
///
/// Exposed as `pub(crate)` so the reconcile translator (`holochain_app_signal`)
/// can call it directly without duplicating the conversion logic or encoding
/// caller identity in the function name.
pub(crate) fn timestamp_to_iso(v: &serde_json::Value) -> String {
    if let Some(micros) = v.as_i64() {
        let secs = micros / 1_000_000;
        let dt = chrono::DateTime::from_timestamp(secs, 0).unwrap_or_default();
        return dt.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    }
    if let Some(secs) = v.get("secs").and_then(|s| s.as_i64()) {
        let dt = chrono::DateTime::from_timestamp(secs, 0).unwrap_or_default();
        return dt.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    }
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// Extract a `RecoveryInvitation` publish intent from a `RecoveryV2Signal`.
///
/// Returns `Some(RecoveryInvitation)` only for `RecoveryRequestCreated` signals
/// where the coordinator populated `human_id` (post-M3 requests). The production
/// signal subscriber SHOULD send this (if present) via its `P2PCommand` channel
/// alongside calling `handle_recovery_v2_signal` for the SQLite projection:
///
/// ```ignore
/// if let Some(inv) = recovery_invitation_from_signal(&sig) {
///     let _ = p2p_tx.try_send(P2PCommand::PublishRecoveryInvitation(inv));
/// }
/// handle_recovery_v2_signal(&mut conn, sig)?;
/// ```
///
/// Best-effort: dropping the publish intent does not affect projection correctness.
/// The DHT remains the source of truth and peers can rediscover via signal replay.
pub fn recovery_invitation_from_signal(
    signal: &RecoveryV2Signal,
) -> Option<crate::p2p::recovery_invitation::RecoveryInvitation> {
    match signal {
        RecoveryV2Signal::RecoveryRequestCreated {
            action_hash,
            request,
        } => request.human_id.as_ref().map(|hid| {
            crate::p2p::recovery_invitation::RecoveryInvitation {
                request_hash: action_hash.clone(),
                human_id: hid.clone(),
                created_at: timestamp_to_iso(&request.created_at),
            }
        }),
        _ => None,
    }
}

/// Extract a `RecoveryRevocationMessage` publish intent from a `RecoveryV2Signal`.
///
/// Returns `Some(msg)` for `KeyRevocationRequested` and `KeyRevocationEffective`
/// (so other peers can re-ingest revocation state). Returns `None` for
/// `RevocationVoteSubmitted` (votes are DHT-authoritative; they are not gossiped
/// separately per spec §7.1 decision log entry #6) and all other variants.
///
/// The caller SHOULD forward the message via `P2PCommand::PublishRecoveryRevocation`
/// alongside calling `handle_recovery_v2_signal`:
///
/// ```ignore
/// if let Some(msg) = recovery_revocation_from_signal(&sig, &local_peer_id_str) {
///     let _ = p2p_tx.try_send(P2PCommand::PublishRecoveryRevocation(msg));
/// }
/// handle_recovery_v2_signal(&mut conn, sig)?;
/// ```
///
/// `sender_peer_id` is the local node's PeerId as a base58 string.
pub fn recovery_revocation_from_signal(
    signal: &RecoveryV2Signal,
    sender_peer_id: &str,
) -> Option<RecoveryRevocationMessage> {
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    match signal {
        RecoveryV2Signal::KeyRevocationRequested {
            id,
            human_id,
            revoked_key,
            trigger_type,
            reason,
            threshold_reached,
            ..
        } => Some(RecoveryRevocationMessage {
            revocation_id: id.clone(),
            human_id: human_id.clone(),
            revoked_key: revoked_key.clone(),
            trigger_type: trigger_type.clone(),
            reason: reason.clone(),
            status: if *threshold_reached {
                "effective".into()
            } else {
                "pending".into()
            },
            sender_peer_id: sender_peer_id.to_string(),
            sent_at: now,
        }),
        RecoveryV2Signal::KeyRevocationEffective {
            revocation_id,
            revoked_key,
            human_id,
            ..
        } => Some(RecoveryRevocationMessage {
            revocation_id: revocation_id.clone(),
            human_id: human_id.clone(),
            revoked_key: revoked_key.clone(),
            // trigger_type and reason are not carried on the Effective signal;
            // mesh subscribers key by revocation_id which is stable.
            trigger_type: String::new(),
            reason: String::new(),
            status: "effective".into(),
            sender_peer_id: sender_peer_id.to_string(),
            sent_at: now,
        }),
        // RevocationVoteSubmitted: votes stay DHT-side; not gossiped separately.
        // All other variants: not relevant to recovery.revocation topic.
        _ => None,
    }
}

/// Dispatch a `RecoveryV2Signal` into the SQLite projection.
///
/// New variants add a match arm here. Keep this function thin — per-entity
/// projection logic belongs in `db::recovery_requests`.
///
/// The gossipsub publish bridge is intentionally separate — use
/// `recovery_invitation_from_signal` to extract a publish intent and forward it
/// via `P2PCommand::PublishRecoveryInvitation` in the production signal loop.
pub fn handle_recovery_v2_signal(
    conn: &mut diesel::sqlite::SqliteConnection,
    signal: RecoveryV2Signal,
) -> Result<(), StorageError> {
    match signal {
        RecoveryV2Signal::RecoveryRequestCreated {
            action_hash,
            request,
        } => {
            let authority_kind = extract_authority_kind(&request.proposed_authority);
            let authority_json = request.proposed_authority.to_string();
            let created_at = timestamp_to_iso(&request.created_at);

            let row = crate::db::models::NewRecoveryRequestRow {
                dht_anchor_hash: action_hash,
                human_agent_pubkey: request.human_agent_pubkey,
                new_agent_pubkey: request.new_agent_pubkey,
                hosting_doorway_pubkey: request.hosting_doorway_pubkey,
                proposed_authority_kind: authority_kind,
                proposed_authority_json: authority_json,
                request_nonce: request.request_nonce,
                human_id: request.human_id,
                required_witness_count: request.required_witness_count as i32,
                created_at,
            };
            crate::db::recovery_requests::upsert_recovery_request(conn, row)
        }
        RecoveryV2Signal::IntimateWitnessSubmitted {
            action_hash,
            request_hash,
            witness,
            witness_agent_id,
        } => {
            let submitted_at = timestamp_to_iso(&witness.issued_at);
            let row = crate::db::models::NewRecoveryWitnessRow {
                dht_anchor_hash: action_hash,
                recovery_request_hash: request_hash,
                witness_agent_id,
                human_id: witness.human_id,
                note: witness.note,
                submitted_at,
            };
            crate::db::recovery_witnesses::upsert_recovery_witness(conn, row)
        }
        RecoveryV2Signal::KeyRotationCommitted {
            action_hash,
            rotation,
        } => {
            let authority_kind = extract_authority_kind(&rotation.authority);
            let authority_json = rotation.authority.to_string();
            let rotated_at = timestamp_to_iso(&rotation.rotated_at);

            let row = crate::db::models::NewKeyRotationRow {
                dht_anchor_hash: action_hash,
                human_agent_pubkey: rotation.human_agent_pubkey,
                new_agent_pubkey: rotation.new_agent_pubkey,
                superseded_agent_pubkey: rotation.superseded_agent_pubkey,
                recovery_request_hash: rotation.recovery_request_hash,
                authority_kind,
                authority_json,
                rotated_at,
            };
            crate::db::recovery_requests::upsert_key_rotation(conn, row)
        }
        // M4 revocation projection handlers (Phase D.3).
        //
        // Design note: the M4 DNA signals do not include an ActionHash field
        // (unlike RecoveryRequestCreated which carries `action_hash`). The `id`
        // field is the coordinator-assigned revocation UUID. We use `id` as the
        // `dht_anchor_hash` storage key for deduplication; it is stable and unique.
        // A future enrichment pass can backfill the real ActionHash if needed.
        RecoveryV2Signal::KeyRevocationRequested {
            id,
            human_id,
            revoked_key,
            reason,
            trigger_type,
            initiated_by,
            required_votes,
            current_votes,
            threshold_reached,
            effective_at,
            created_at,
        } => {
            // Use id as dht_anchor_hash (see design note above). The new
            // `key_revocations` table stores anchor as BLOB (Vec<u8>); we
            // round-trip the M4 string id through bytes pending T13/T14
            // cross-DNA Content wiring that supplies a real ActionHash.
            let dht_anchor_hash = id.as_bytes().to_vec();
            let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
            let row = crate::db::models::KeyRevocationRow {
                id: id.clone(),
                dht_anchor_hash,
                subject_human_id: human_id.clone(),
                revoked_key: revoked_key.clone(),
                trigger_type: trigger_type.clone(),
                reason: reason.clone(),
                initiated_by_cid: initiated_by,
                required_votes: required_votes as i32,
                current_votes: current_votes as i32,
                threshold_reached: if threshold_reached { 1 } else { 0 },
                effective_at: effective_at.clone(),
                derived_compromise_at: None,
                created_at: created_at.clone(),
                updated_at: created_at.clone(),
            };
            crate::db::key_revocations::upsert(conn, &row)?;

            // P1: Outbound reconciliation signal — imagodei.revocation_observed.
            emit_reconciled_signal(ImagodeiReconciledEvent::RevocationObserved {
                revocation_id: id,
                revoked_key,
                human_id,
                status: if threshold_reached {
                    "effective-on-create".into()
                } else {
                    "pending".into()
                },
                observed_at: now,
            });
            Ok(())
        }
        RecoveryV2Signal::RevocationVoteSubmitted {
            id,
            revocation_id,
            steward_id,
            approved,
            attestation,
            voted_at,
            current_votes: _,
            required_votes: _,
            threshold_now_reached: _,
        } => {
            // The legacy revocation_votes table still keys on `dht_anchor_hash`
            // as String; only `key_revocations` was rewritten in T6/T7. Once
            // T17 aligns the DNA-side signal payloads with the new Content
            // contract, the vote pairing will be sourced via attestation
            // children of the parent key-revocation Content rather than this
            // separate table.
            let revocation_dht_anchor_hash = revocation_id.clone();
            let vote_dht_anchor_hash = id.clone();
            let row = crate::db::models::InsertRevocationVoteRow {
                dht_anchor_hash: vote_dht_anchor_hash,
                id,
                revocation_dht_anchor_hash: revocation_dht_anchor_hash.clone(),
                revocation_id: revocation_id.clone(),
                steward_id,
                approved: if approved { 1 } else { 0 },
                attestation,
                voted_at: voted_at.clone(),
            };
            crate::db::revocation_votes::insert_revocation_vote(conn, row)?;

            // Recompute current_votes from authoritative count in projection.
            // `key_revocations` PK is now `id`; use revocation_id as that key.
            let approved_count = crate::db::revocation_votes::count_approved_votes_for_revocation(
                conn,
                &revocation_dht_anchor_hash,
            )?;
            let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
            crate::db::key_revocations::update_current_votes(
                conn,
                &revocation_id,
                approved_count as i32,
                &now,
            )?;
            Ok(())
        }
        RecoveryV2Signal::KeyRevocationEffective { .. } => {
            // T6 deletion: legacy KeyRevocationEffective is fully superseded by the
            // T18 DnaSignal::KeyRevocation envelope path (see handle_imagodei_dna_signal).
            // Producer side (imagodei coordinator) still emits this variant; M4 deletes
            // the producer emissions + variant as a follow-up. No-op until then.
            Ok(())
        }
    }
}

/// Sweep dependent cached state tied to a revoked key. Eager, bounded,
/// indexed (not a table scan).
///
/// Phase 2B (A.8): clears `verified_at` on every `epr_atom` row where
/// `signer_cid == revoked_key` AND `issued_at > compromise_at` AND the row was
/// previously verified (`verified_at IS NOT NULL`). The sweep is time-bounded —
/// atoms issued strictly before the compromise window are left untouched, as
/// their signatures cannot have been forged by the compromised key.
///
/// The `(signer_cid, issued_at)` index on `epr_atoms` (migration
/// `2026-04-25-000000_verified_at_on_epr_atoms`) makes this a single indexed
/// UPDATE with no table-scan.
///
/// Returns the number of rows updated. Both callers currently ignore the count
/// via `let _ = ...?;` (unit propagation via `?` is a strict subset).
///
/// M5 extension point: add `DELETE FROM peer_identity_bindings WHERE pubkey = ?revoked_key`.
fn sweep_dependent_caches_on_revocation(
    conn: &mut diesel::sqlite::SqliteConnection,
    revoked_key: &str,
    compromise_at: &str,
) -> Result<usize, crate::error::StorageError> {
    use crate::db::diesel_schema::epr_atoms;
    use diesel::prelude::*;

    let updated = diesel::update(
        epr_atoms::table
            .filter(epr_atoms::signer_cid.eq(revoked_key))
            .filter(epr_atoms::issued_at.gt(compromise_at))
            .filter(epr_atoms::verified_at.is_not_null()),
    )
    .set(epr_atoms::verified_at.eq::<Option<String>>(None))
    .execute(conn)
    .map_err(|e| crate::error::StorageError::Database(e.to_string()))?;

    tracing::info!(
        target: "imagodei.revocation_sweep",
        revoked_key = %revoked_key,
        compromise_at = %compromise_at,
        updated = %updated,
        "A.8 EPR-atom revocation sweep complete"
    );
    Ok(updated)
}

// =============================================================================
// T18: ImagodeiDnaSignal dispatcher — EPR envelope consumer
// =============================================================================
//
// `handle_imagodei_dna_signal` is the consumer entry point for the new
// `DnaSignal::KeyRevocation(KeyRevocationEnvelope)` signal emitted alongside
// the legacy `RecoveryV2Signal::KeyRevocationEffective` at all three producer
// sites (back-compat window). Callers decode the raw JSON blob into
// `ImagodeiDnaSignal` first, then call this function.
//
// Signature verification: the canonical bytes are recomputed from the
// deserialized envelope (EXCLUDING signature and relay_chain) using the same
// MessagePack serialiser as the zome side. The issuer's ed25519 public key
// and signature are decoded from base64 and verified via `ed25519-dalek`.
//
// Projection: after verification, the `KeyRevocation` arm calls
// `crate::db::key_revocations::set_effective` using the envelope's
// `metadata.revocation_id` as the projection table PK. The legacy
// `RecoveryV2Signal::KeyRevocationEffective` handler in `handle_recovery_v2_signal`
// is retained as an explicit no-op for match exhaustiveness during the
// producer-side transition (M4 deletes the variant + emissions as a follow-up);
// duplicate delivery is therefore harmless (the envelope path is the single
// projection writer).
//
// **`canonical_envelope_bytes` must match the zome-side implementation in
// `imagodei/zomes/imagodei/src/lib.rs` exactly.**
// See spec at `genesis/docs/content/elohim-protocol/architecture/2026-05-15-dna-signal-as-epr-envelope.md`.

/// Sub-struct for canonical serialization — matches the zome-side
/// `CanonicalEnvelopeCore` / `CanonicalMetadataRef` pair exactly.
///
/// Field order and camelCase rename MUST be kept in sync with the zome.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CanonicalEnvelopeCore<'a> {
    attestation_kind: &'a str,
    subject_cid: &'a str,
    issuer: &'a str,
    issued_at: &'a str,
    metadata: CanonicalMetadataRef<'a>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CanonicalMetadataRef<'a> {
    revocation_id: &'a str,
    revoked_pubkey: &'a str,
    agent_cid: &'a str,
    compromise_at: &'a str,
    effective_at: &'a str,
    triggering_revocation_id: Option<&'a str>,
    supersedes_cid: Option<&'a str>,
}

/// Reproduce the canonical signing bytes for a `KeyRevocationEnvelope`.
///
/// **Must match the zome-side `canonical_envelope_bytes` in
/// `imagodei/zomes/imagodei/src/lib.rs`.**
///
/// Excludes `signature` and `relay_chain`. Serializes via `rmp_serde`
/// struct-map form (field names included, declaration order preserved) —
/// the same serialiser that `holochain_serialized_bytes::encode` uses.
///
/// # Verification recipe
///
/// ```text
/// 1. Deserialize envelope from wire JSON.
/// 2. canonical_bytes = canonical_envelope_bytes(&envelope)
/// 3. issuer_bytes   = base64::STANDARD.decode(&envelope.issuer)       // 32 bytes
/// 4. sig_bytes      = base64::STANDARD.decode(&envelope.signature)    // 64 bytes
/// 5. key   = ed25519_dalek::VerifyingKey::from_bytes(&issuer_bytes)
/// 6. sig   = ed25519_dalek::Signature::from_bytes(&sig_bytes)
/// 7. key.verify_strict(&canonical_bytes, &sig)  →  Ok(()) iff valid
/// ```
fn canonical_envelope_bytes(envelope: &KeyRevocationEnvelope) -> Vec<u8> {
    let core = CanonicalEnvelopeCore {
        attestation_kind: &envelope.attestation_kind,
        subject_cid: &envelope.subject_cid,
        issuer: &envelope.issuer,
        issued_at: &envelope.issued_at,
        metadata: CanonicalMetadataRef {
            revocation_id: &envelope.metadata.revocation_id,
            revoked_pubkey: &envelope.metadata.revoked_pubkey,
            agent_cid: &envelope.metadata.agent_cid,
            compromise_at: &envelope.metadata.compromise_at,
            effective_at: &envelope.metadata.effective_at,
            triggering_revocation_id: envelope.metadata.triggering_revocation_id.as_deref(),
            supersedes_cid: envelope.metadata.supersedes_cid.as_deref(),
        },
    };
    let mut buf = Vec::with_capacity(256);
    let mut se = rmp_serde::encode::Serializer::new(&mut buf).with_struct_map();
    core.serialize(&mut se)
        .expect("canonical_envelope_bytes: encode should never fail on well-formed structs");
    buf
}

/// Verify the issuer signature on a `KeyRevocationEnvelope`.
///
/// Returns `Ok(())` on success, `Err(StorageError::SignatureVerificationFailed)`
/// on invalid signature, or `Err(StorageError::InvalidData(...))` on key/sig
/// decode failure.
///
/// Called by `handle_imagodei_dna_signal` before projection. Consumer-side
/// verification guards against forged or mutated envelopes arriving via the
/// Holochain app-signal stream or gossip relay.
fn verify_envelope_signature(
    envelope: &KeyRevocationEnvelope,
) -> Result<(), crate::error::StorageError> {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    use ed25519_dalek::{Signature as Ed25519Sig, VerifyingKey};

    let issuer_bytes = STANDARD.decode(&envelope.issuer).map_err(|e| {
        crate::error::StorageError::InvalidInput(format!(
            "key_revocation_envelope: issuer base64 decode failed: {e}"
        ))
    })?;
    let sig_bytes = STANDARD.decode(&envelope.signature).map_err(|e| {
        crate::error::StorageError::InvalidInput(format!(
            "key_revocation_envelope: signature base64 decode failed: {e}"
        ))
    })?;

    let key_arr: [u8; 32] = issuer_bytes.try_into().map_err(|_| {
        crate::error::StorageError::InvalidInput(
            "key_revocation_envelope: issuer key is not 32 bytes".into(),
        )
    })?;
    let sig_arr: [u8; 64] = sig_bytes.try_into().map_err(|_| {
        crate::error::StorageError::InvalidInput(
            "key_revocation_envelope: signature is not 64 bytes".into(),
        )
    })?;

    let verifying_key = VerifyingKey::from_bytes(&key_arr).map_err(|e| {
        crate::error::StorageError::InvalidInput(format!(
            "key_revocation_envelope: invalid ed25519 public key: {e}"
        ))
    })?;
    let signature = Ed25519Sig::from_bytes(&sig_arr);

    let canonical = canonical_envelope_bytes(envelope);
    verifying_key
        .verify_strict(&canonical, &signature)
        .map_err(|e| crate::error::StorageError::VerifyFailed {
            reason: format!("key_revocation_envelope: signature invalid: {e}"),
        })
}

/// Dispatch an `ImagodeiDnaSignal` into the SQLite projection.
///
/// T18: only the `KeyRevocation` variant is handled here. Future variants
/// (e.g. `KeyRotation`) extend this match arm.
///
/// **Verification**: the `KeyRevocation` arm verifies the issuer signature
/// before projection. A log warning is emitted on failure and the signal is
/// silently dropped (best-effort — the projection can be recovered from DHT
/// replay on reconnect).
///
/// **Dedup**: uses `metadata.revocation_id` as the projection table PK.
/// The T6-deleted legacy path previously shared this PK; duplicate delivery
/// would have landed the same idempotent `set_effective` call.
pub fn handle_imagodei_dna_signal(
    conn: &mut diesel::sqlite::SqliteConnection,
    signal: ImagodeiDnaSignal,
) -> Result<(), crate::error::StorageError> {
    match signal {
        ImagodeiDnaSignal::KeyRevocation(envelope) => {
            // Verify signature before projecting. On failure: warn + drop.
            // Not a hard error — best-effort; if signature verify fails, the
            // projection row is missed and the operator must replay from DHT.
            if let Err(e) = verify_envelope_signature(&envelope) {
                tracing::warn!(
                    target: "imagodei.dna_signal",
                    revocation_id = %envelope.metadata.revocation_id,
                    error = %e,
                    "T18 KeyRevocation envelope signature verification failed; dropping signal"
                );
                return Ok(());
            }

            let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

            // Project: mark the key_revocations row as effective.
            // The row must already exist (created by handle_recovery_v2_signal's
            // KeyRevocationRequested arm earlier in the same emission batch).
            // If the row is absent (e.g. out-of-order delivery), the set_effective
            // call will silently no-op (no row to update is not an error).
            crate::db::key_revocations::set_effective(
                conn,
                &envelope.metadata.revocation_id,
                &envelope.metadata.effective_at,
                &now,
            )?;

            // Eager cache-invalidation sweep — the single A.8 sweep callsite
            // since the legacy path was retired in T6.
            let _ = sweep_dependent_caches_on_revocation(
                conn,
                &envelope.metadata.revoked_pubkey,
                &envelope.metadata.compromise_at,
            )?;

            // P1: Outbound reconciliation signal.
            emit_reconciled_signal(ImagodeiReconciledEvent::RevocationObserved {
                revocation_id: envelope.metadata.revocation_id.clone(),
                revoked_key: envelope.metadata.revoked_pubkey.clone(),
                human_id: envelope.metadata.agent_cid.clone(),
                status: "effective".into(),
                observed_at: now,
            });

            tracing::info!(
                target: "imagodei.dna_signal",
                revocation_id = %envelope.metadata.revocation_id,
                subject_cid = %envelope.subject_cid,
                issuer = %&envelope.issuer[..8],
                "T18 KeyRevocation envelope projected"
            );

            Ok(())
        }
    }
}

#[cfg(test)]
mod dna_signal_tests {
    use super::*;

    /// Verify that `ImagodeiDnaSignal::KeyRevocation` deserializes correctly
    /// from the wire format emitted by the imagodei coordinator (camelCase,
    /// serde tag = "type").
    #[test]
    fn key_revocation_envelope_deserializes_from_wire_shape() {
        let wire = serde_json::json!({
            "type": "keyRevocation",
            "attestationKind": "attestation:key-revocation-emit",
            "subjectCid": "bafybeicid001",
            "issuer": "dGVzdGlzc3Vlcmlzc3Vl",  // base64 placeholder
            "issuedAt": "2026-05-15T10:00:00.000Z",
            "signature": "c2lnbmF0dXJlcGxhY2Vob2xkZXIwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMA==",
            "metadata": {
                "revocationId": "rev-human-X-12345",
                "revokedPubkey": "dGVzdHJldm9rZWRwdWJrZXkwMDAwMDAwMDAwMDAwMDA=",
                "agentCid": "human-X",
                "compromiseAt": "2026-05-15T09:00:00.000Z",
                "effectiveAt": "2026-05-15T09:00:00.000Z",
                "triggeringRevocationId": null,
                "supersedesCid": null
            },
            "relayChain": []
        });
        let signal: ImagodeiDnaSignal = serde_json::from_value(wire).unwrap();
        match signal {
            ImagodeiDnaSignal::KeyRevocation(env) => {
                assert_eq!(env.attestation_kind, "attestation:key-revocation-emit");
                assert_eq!(env.subject_cid, "bafybeicid001");
                assert_eq!(env.metadata.revocation_id, "rev-human-X-12345");
                assert_eq!(env.metadata.agent_cid, "human-X");
                assert!(env.metadata.triggering_revocation_id.is_none());
                assert!(env.metadata.supersedes_cid.is_none());
                assert!(env.relay_chain.is_empty());
            }
        }
    }

    /// Verify that `canonical_envelope_bytes` is deterministic across two
    /// identical envelopes (no runtime state in the serialiser).
    #[test]
    fn canonical_bytes_are_deterministic() {
        let envelope = make_test_envelope();
        let b1 = canonical_envelope_bytes(&envelope);
        let b2 = canonical_envelope_bytes(&envelope);
        assert_eq!(b1, b2, "canonical bytes must be deterministic");
        assert!(!b1.is_empty());
    }

    /// Verify that changing any covered field produces different canonical bytes.
    #[test]
    fn canonical_bytes_differ_on_field_change() {
        let mut e1 = make_test_envelope();
        let b1 = canonical_envelope_bytes(&e1);
        e1.subject_cid = "bafyDIFFERENT".into();
        let b2 = canonical_envelope_bytes(&e1);
        assert_ne!(b1, b2, "changing subjectCid must change canonical bytes");
    }

    /// Verify that `signature` and `relay_chain` fields are excluded from
    /// canonical bytes (changing them must not change canonical bytes).
    #[test]
    fn canonical_bytes_exclude_signature_and_relay_chain() {
        let mut env = make_test_envelope();
        let b1 = canonical_envelope_bytes(&env);
        env.signature = "CHANGED_SIGNATURE".into();
        env.relay_chain = vec![serde_json::json!({"hop": 1})];
        let b2 = canonical_envelope_bytes(&env);
        assert_eq!(
            b1, b2,
            "signature/relay_chain changes must not affect canonical bytes"
        );
    }

    fn make_test_envelope() -> KeyRevocationEnvelope {
        KeyRevocationEnvelope {
            attestation_kind: "attestation:key-revocation-emit".into(),
            subject_cid: "bafybeicid001".into(),
            issuer: "dGVzdGlzc3VlcisyMzQ1Njc4OTAxMjM0NTY3ODkwMTI=".into(),
            issued_at: "2026-05-15T10:00:00.000Z".into(),
            signature: "placeholder_sig".into(),
            metadata: KeyRevocationMetadata {
                revocation_id: "rev-human-X-12345".into(),
                revoked_pubkey: "dGVzdHJldm9rZWRwdWJrZXkwMDAwMDAwMDAwMDAwMDA=".into(),
                agent_cid: "human-X".into(),
                compromise_at: "2026-05-15T09:00:00.000Z".into(),
                effective_at: "2026-05-15T09:00:00.000Z".into(),
                triggering_revocation_id: None,
                supersedes_cid: None,
            },
            relay_chain: vec![],
        }
    }
}

#[cfg(test)]
mod mishpat_signal_tests {
    use super::*;
    use diesel::connection::SimpleConnection;
    use diesel::prelude::*;
    use diesel::sqlite::SqliteConnection;
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

    const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

    fn setup_test_conn() -> SqliteConnection {
        let mut conn =
            SqliteConnection::establish(":memory:").expect("Failed to create in-memory SQLite");

        // Phase-2a consolidation: gate decisions project into the unified
        // `attestations` table (the legacy per-app gate-decision projection
        // table was dropped). Mirror its schema for the round-trip projection tests.
        conn.batch_execute(
            r#"
            CREATE TABLE attestations (
                id TEXT PRIMARY KEY,
                dht_anchor_hash BLOB NOT NULL,
                attestation_kind TEXT NOT NULL,
                subject_cid TEXT NOT NULL,
                subject_kind TEXT NOT NULL,
                issuer_cid TEXT NOT NULL,
                parent_governance_action_cid TEXT,
                vote_value TEXT,
                vote_weight TEXT,
                proof_class TEXT NOT NULL,
                proof_evidence_json TEXT NOT NULL,
                evidence_json TEXT NOT NULL,
                expires_at TEXT,
                supersedes_cid TEXT,
                revocation_reason TEXT,
                revoked_at TEXT,
                created_at TEXT NOT NULL,
                manifest_ref TEXT NOT NULL,
                title TEXT NOT NULL,
                description TEXT
            );
            "#,
        )
        .expect("Failed to create test table");

        conn
    }

    fn make_signal(decision_id: &str, decision: &str, phase: &str) -> MishpatSignal {
        MishpatSignal::GateDecisionCreated {
            action_hash: "uhCkkDEFABC".into(),
            entry_hash: "uhCEkDEFABC".into(),
            author: "uhCAkABCDEF".into(),
            entry: GateDecisionAttestationEntry {
                decision_id: decision_id.to_string(),
                phase: phase.to_string(),
                elohim_id: "uhCAkABCDEF".to_string(),
                elohim_substance_cid: "bafySubstance".to_string(),
                gate_name: "discernment-gate-v1-mechanical".to_string(),
                gate_process_cid: "bafyProcess".to_string(),
                request_ref_json: r#"{"eventId":"ev1"}"#.to_string(),
                decision: decision.to_string(),
                reasoning_json: r#"{"steps":[]}"#.to_string(),
                context_summary_cid: "bafyCtx".to_string(),
                decided_at: "2026-04-18T12:00:00Z".to_string(),
                universal_band_cid: "bafyBand".to_string(),
            },
        }
    }

    /// Round-trip: write a gate decision via the repointed writer
    /// (`handle_mishpat_signal` → unified `attestations` table) and read it back
    /// via the repointed readers (`db::attestations::gate_decision_*` +
    /// the `GateDecisionAttestationView` reconstruction). Every field of the
    /// legacy wire shape must survive the round-trip through `evidence_json`.
    #[test]
    fn gate_decision_created_projects_row() {
        let mut conn = setup_test_conn();
        let signal = make_signal("bafyDec001", "allow", "elohim-active");
        handle_mishpat_signal(&mut conn, "test-app", signal).unwrap();

        // Read back via the by-id reader on the unified table.
        let row = crate::db::attestations::gate_decision_find_by_id(&mut conn, "bafyDec001")
            .unwrap()
            .expect("Row must be present after signal");
        assert_eq!(row.attestation_kind, "attestation:gate-decision");
        assert_eq!(row.subject_cid, "uhCAkABCDEF");
        assert_eq!(row.issuer_cid, "uhCAkABCDEF");
        assert_eq!(row.proof_class, "audit");

        // Reconstruct the legacy view and assert every structured field survived.
        let view = crate::views_convert::qahal::gate_decision_view_from_attestation(row);
        assert_eq!(view.decision_id, "bafyDec001");
        assert_eq!(view.decision, "allow");
        assert_eq!(view.phase, "elohim-active");
        assert_eq!(view.dht_anchor_hash, "uhCkkDEFABC");
        assert_eq!(view.gate_name, "discernment-gate-v1-mechanical");
        assert_eq!(view.elohim_id, "uhCAkABCDEF");
        assert_eq!(view.elohim_substance_cid, "bafySubstance");
        assert_eq!(view.gate_process_cid, "bafyProcess");
        assert_eq!(view.request_ref_json, r#"{"eventId":"ev1"}"#);
        assert_eq!(view.reasoning_json, r#"{"steps":[]}"#);
        assert_eq!(view.context_summary_cid, "bafyCtx");
        assert_eq!(view.decided_at, "2026-04-18T12:00:00Z");
        assert_eq!(view.universal_band_cid, "bafyBand");

        // The gate / phase readers find it too.
        let by_gate = crate::db::attestations::gate_decision_find_by_gate(
            &mut conn,
            "discernment-gate-v1-mechanical",
        )
        .unwrap();
        assert_eq!(by_gate.len(), 1);
        let by_phase =
            crate::db::attestations::gate_decision_find_by_phase(&mut conn, "elohim-active")
                .unwrap();
        assert_eq!(by_phase.len(), 1);
        let by_elohim =
            crate::db::attestations::gate_decision_find_by_elohim(&mut conn, "uhCAkABCDEF")
                .unwrap();
        assert_eq!(by_elohim.len(), 1);
    }

    #[test]
    fn gate_decision_created_is_idempotent() {
        let mut conn = setup_test_conn();
        let signal = make_signal("bafyDec002", "decline", "dev-context");
        handle_mishpat_signal(&mut conn, "test-app", signal.clone()).unwrap();
        handle_mishpat_signal(&mut conn, "test-app", signal).unwrap();

        let rows =
            crate::db::attestations::gate_decision_find_by_phase(&mut conn, "dev-context").unwrap();
        assert_eq!(
            rows.len(),
            1,
            "Re-delivered signal must not duplicate the row"
        );
    }

    #[test]
    fn serde_tag_matches_dna_wire_format() {
        // Verify `{"type": "GateDecisionCreated", "payload": {...}}` round-trips.
        // Drift here would silently break projection.
        let wire = serde_json::json!({
            "type": "GateDecisionCreated",
            "payload": {
                "action_hash": "uhCkkABC",
                "entry_hash": "uhCEkABC",
                "author": "uhCAkABC",
                "entry": {
                    "decision_id": "bafyDec",
                    "phase": "elohim-active",
                    "elohim_id": "uhCAkABC",
                    "elohim_substance_cid": "bafySub",
                    "gate_name": "discernment-gate-v1",
                    "gate_process_cid": "bafyProc",
                    "request_ref_json": "{}",
                    "decision": "allow",
                    "reasoning_json": "{}",
                    "context_summary_cid": "bafyCtx",
                    "decided_at": "2026-04-18T00:00:00Z",
                    "universal_band_cid": "bafyBand",
                }
            }
        });

        let signal: MishpatSignal = serde_json::from_value(wire).unwrap();
        match signal {
            MishpatSignal::GateDecisionCreated {
                action_hash, entry, ..
            } => {
                assert_eq!(action_hash.0, "uhCkkABC");
                assert_eq!(entry.decision, "allow");
                assert_eq!(entry.phase, "elohim-active");
            }
            _ => panic!("Expected GateDecisionCreated variant"),
        }
    }

    // -------------------------------------------------------------------------
    // GateDecisionChallengeCreated tests
    // -------------------------------------------------------------------------

    fn setup_challenge_conn() -> SqliteConnection {
        let mut conn =
            SqliteConnection::establish(":memory:").expect("Failed to create in-memory SQLite");

        conn.batch_execute(
            r#"
            CREATE TABLE gate_decision_challenges (
                app_id TEXT NOT NULL,
                challenge_id TEXT NOT NULL,
                challenged_decision_cid TEXT NOT NULL,
                challenger_id TEXT NOT NULL,
                grounds TEXT NOT NULL,
                summary TEXT NOT NULL,
                evidence_refs TEXT NOT NULL,
                filed_at TEXT NOT NULL,
                reach TEXT NOT NULL,
                dht_anchor_hash TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                PRIMARY KEY (app_id, challenge_id)
            );
            "#,
        )
        .expect("Failed to create test table");

        conn
    }

    fn make_challenge_signal(challenge_id: &str, grounds: &str) -> MishpatSignal {
        MishpatSignal::GateDecisionChallengeCreated {
            action_hash: "uhCkkCHALABC".into(),
            entry_hash: "uhCEkCHALABC".into(),
            author: "uhCAkCHALLENGER".into(),
            entry: GateDecisionChallengeEntry {
                challenge_id: challenge_id.to_string(),
                challenged_decision_cid: "bafyDec001".to_string(),
                challenger_id: "uhCAkCHALLENGER".to_string(),
                grounds: grounds.to_string(),
                summary: "Decision violated constitutional principle P4".to_string(),
                evidence_refs: "bafyEvidence1".to_string(),
                filed_at: "2026-04-19T10:00:00Z".to_string(),
                reach: "community".to_string(),
            },
        }
    }

    #[test]
    fn gate_decision_challenge_created_projects_row() {
        let mut conn = setup_challenge_conn();
        let signal = make_challenge_signal("bafyChal001", "constitutional");
        handle_mishpat_signal(&mut conn, "test-app", signal).unwrap();

        let row =
            crate::db::gate_decision_challenges::find_by_id(&mut conn, "test-app", "bafyChal001")
                .unwrap()
                .expect("Row must be present after signal");

        assert_eq!(row.grounds, "constitutional");
        assert_eq!(row.challenged_decision_cid, "bafyDec001");
        assert_eq!(row.dht_anchor_hash, "uhCkkCHALABC");
        assert_eq!(row.reach, "community");
    }

    #[test]
    fn gate_decision_challenge_created_is_idempotent() {
        let mut conn = setup_challenge_conn();
        let signal = make_challenge_signal("bafyChal002", "safety");
        handle_mishpat_signal(&mut conn, "test-app", signal.clone()).unwrap();
        handle_mishpat_signal(&mut conn, "test-app", signal).unwrap();

        let rows = crate::db::gate_decision_challenges::find_by_challenger(
            &mut conn,
            "test-app",
            "uhCAkCHALLENGER",
        )
        .unwrap();
        assert_eq!(
            rows.len(),
            1,
            "Re-delivered signal must not duplicate the row"
        );
    }

    #[test]
    fn gate_decision_challenge_serde_tag_matches_dna_wire_format() {
        let wire = serde_json::json!({
            "type": "GateDecisionChallengeCreated",
            "payload": {
                "action_hash": "uhCkkCHAL",
                "entry_hash": "uhCEkCHAL",
                "author": "uhCAkCHAL",
                "entry": {
                    "challenge_id": "bafyChal",
                    "challenged_decision_cid": "bafyDec",
                    "challenger_id": "uhCAkCHAL",
                    "grounds": "constitutional",
                    "summary": "Grievance details",
                    "evidence_refs": "",
                    "filed_at": "2026-04-19T00:00:00Z",
                    "reach": "community",
                }
            }
        });

        let signal: MishpatSignal = serde_json::from_value(wire).unwrap();
        match signal {
            MishpatSignal::GateDecisionChallengeCreated {
                action_hash, entry, ..
            } => {
                assert_eq!(action_hash.0, "uhCkkCHAL");
                assert_eq!(entry.grounds, "constitutional");
                assert_eq!(entry.reach, "community");
            }
            _ => panic!("Expected GateDecisionChallengeCreated variant"),
        }
    }

    // -------------------------------------------------------------------------
    // ChallengeOutcomeCreated tests
    // -------------------------------------------------------------------------

    fn setup_outcome_conn() -> SqliteConnection {
        let mut conn =
            SqliteConnection::establish(":memory:").expect("Failed to create in-memory SQLite");

        conn.batch_execute(
            r#"
            CREATE TABLE challenge_outcomes (
                app_id TEXT NOT NULL,
                outcome_id TEXT NOT NULL,
                challenge_cid TEXT NOT NULL,
                verdict TEXT NOT NULL,
                reviewer_consensus TEXT NOT NULL,
                reasoning_json TEXT NOT NULL,
                decided_at TEXT NOT NULL,
                indemnification_actions_json TEXT NOT NULL,
                dht_anchor_hash TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                PRIMARY KEY (app_id, outcome_id)
            );
            "#,
        )
        .expect("Failed to create test table");

        conn
    }

    fn make_outcome_signal(outcome_id: &str, verdict: &str) -> MishpatSignal {
        MishpatSignal::ChallengeOutcomeCreated {
            action_hash: "uhCkkOUTABC".into(),
            entry_hash: "uhCEkOUTABC".into(),
            author: "uhCAkREVIEWER".into(),
            entry: ChallengeOutcomeEntry {
                outcome_id: outcome_id.to_string(),
                challenge_cid: "bafyChal001".to_string(),
                verdict: verdict.to_string(),
                reviewer_consensus: "uhCAkREVIEWER1,uhCAkREVIEWER2".to_string(),
                reasoning_json: r#"{"summary":"Evidence reviewed","steps":[]}"#.to_string(),
                decided_at: "2026-04-20T10:00:00Z".to_string(),
                indemnification_actions_json: "[]".to_string(),
            },
        }
    }

    #[test]
    fn challenge_outcome_created_projects_row() {
        let mut conn = setup_outcome_conn();
        let signal = make_outcome_signal("bafyOut001", "upheld");
        handle_mishpat_signal(&mut conn, "test-app", signal).unwrap();

        let row = crate::db::challenge_outcomes::find_by_id(&mut conn, "test-app", "bafyOut001")
            .unwrap()
            .expect("Row must be present after signal");

        assert_eq!(row.verdict, "upheld");
        assert_eq!(row.challenge_cid, "bafyChal001");
        assert_eq!(row.dht_anchor_hash, "uhCkkOUTABC");
        assert_eq!(row.reviewer_consensus, "uhCAkREVIEWER1,uhCAkREVIEWER2");
    }

    #[test]
    fn challenge_outcome_created_is_idempotent() {
        let mut conn = setup_outcome_conn();
        let signal = make_outcome_signal("bafyOut002", "dismissed");
        handle_mishpat_signal(&mut conn, "test-app", signal.clone()).unwrap();
        handle_mishpat_signal(&mut conn, "test-app", signal).unwrap();

        let rows =
            crate::db::challenge_outcomes::find_by_verdict(&mut conn, "test-app", "dismissed")
                .unwrap();
        assert_eq!(
            rows.len(),
            1,
            "Re-delivered signal must not duplicate the row"
        );
    }

    #[test]
    fn challenge_outcome_serde_tag_matches_dna_wire_format() {
        let wire = serde_json::json!({
            "type": "ChallengeOutcomeCreated",
            "payload": {
                "action_hash": "uhCkkOUT",
                "entry_hash": "uhCEkOUT",
                "author": "uhCAkOUT",
                "entry": {
                    "outcome_id": "bafyOut",
                    "challenge_cid": "bafyChal",
                    "verdict": "upheld",
                    "reviewer_consensus": "uhCAkR1,uhCAkR2",
                    "reasoning_json": "{}",
                    "decided_at": "2026-04-20T00:00:00Z",
                    "indemnification_actions_json": "[]",
                }
            }
        });

        let signal: MishpatSignal = serde_json::from_value(wire).unwrap();
        match signal {
            MishpatSignal::ChallengeOutcomeCreated {
                action_hash, entry, ..
            } => {
                assert_eq!(action_hash.0, "uhCkkOUT");
                assert_eq!(entry.verdict, "upheld");
                assert_eq!(entry.challenge_cid, "bafyChal");
            }
            _ => panic!("Expected ChallengeOutcomeCreated variant"),
        }
    }

    // -------------------------------------------------------------------------
    // CommitmentCommitted tests (I1 — signal serde boundary coverage)
    // -------------------------------------------------------------------------
    //
    // These tests guard the `#[serde(tag="type", content="payload")]` wire
    // crossing from conductor → storage. Every other MishpatSignal variant has
    // matching coverage; CommitmentCommitted previously had none — a silent drift
    // in field names would have broken the projection without any test failure.

    fn setup_commitments_conn() -> SqliteConnection {
        let mut conn =
            SqliteConnection::establish(":memory:").expect("Failed to create in-memory SQLite");
        conn.run_pending_migrations(MIGRATIONS).expect("migrations");
        conn
    }

    fn make_commitment_signal(
        action_hash: &str,
        entry_hash: &str,
        action: &str,
        scope: &str,
    ) -> MishpatSignal {
        // Build a delegates-compute payload_json with all required fields.
        let payload_json = serde_json::json!({
            "action": action,
            "scope": scope,
            "provider": "agent:matthew-steward",
            "recipient": "agent:deploy-svc",
            "bounds": {
                "epr_scope": ["epr:lamad-spa"],
                "reach_ceiling": "commons",
                "rate_per_hour": 30,
                "rotation_ttl_days": 90
            },
            "valid_from": "2026-05-28T00:00:00Z",
            "valid_until": "2026-08-26T00:00:00Z"
        })
        .to_string();

        MishpatSignal::CommitmentCommitted {
            action_hash: action_hash.into(),
            entry_hash: entry_hash.into(),
            author: "uhCAkAUTHOR".into(),
            commitment: crate::mishpat_projection::CommitmentPayload {
                action: action.to_string(),
                payload_json,
                // epoch-seconds string, not ISO-8601
                signed_at: "1748390400".to_string(),
            },
        }
    }

    /// I1a — serde tag/content shape for CommitmentCommitted must round-trip.
    ///
    /// Verifies the exact JSON wire shape the mishpat zome emits (lib.rs ~:1604):
    /// `{"type":"CommitmentCommitted","payload":{action_hash, entry_hash, commitment:{action, payload_json, signed_at}, author}}`
    #[test]
    fn commitment_committed_serde_tag_matches_dna_wire_format() {
        let payload_json_str = r#"{"action":"delegates-compute","scope":"republish-epr","provider":"agent:alice","recipient":"agent:bob","bounds":{"reach_ceiling":"commons","rate_per_hour":30},"valid_from":"2026-05-28T00:00:00Z","valid_until":"2026-08-26T00:00:00Z"}"#;

        let wire = serde_json::json!({
            "type": "CommitmentCommitted",
            "payload": {
                "action_hash": "uhCkkCOMMIT",
                "entry_hash": "uhCEkCOMMIT",
                "author": "uhCAkCOMMIT",
                "commitment": {
                    "action": "delegates-compute",
                    "payload_json": payload_json_str,
                    "signed_at": "1748390400"
                }
            }
        });

        let signal: MishpatSignal = serde_json::from_value(wire).unwrap();
        match signal {
            MishpatSignal::CommitmentCommitted {
                action_hash,
                entry_hash,
                author,
                commitment,
            } => {
                assert_eq!(action_hash.0, "uhCkkCOMMIT");
                assert_eq!(entry_hash.0, "uhCEkCOMMIT");
                assert_eq!(author.0, "uhCAkCOMMIT");
                assert_eq!(commitment.action, "delegates-compute");
                // epoch-seconds, not ISO-8601
                assert_eq!(commitment.signed_at, "1748390400");
                assert!(!commitment.payload_json.is_empty());
            }
            _ => panic!("Expected CommitmentCommitted variant"),
        }
    }

    /// I1b — CommitmentCommitted signal projects one mishpat_commitments row.
    ///
    /// Verifies the full dispatch path: `handle_mishpat_signal` → inline parse
    /// → `mishpat_commitments::upsert_with_anchor` → row with correct fields.
    #[test]
    fn commitment_committed_projects_row() {
        let mut conn = setup_commitments_conn();
        let signal = make_commitment_signal(
            "uhCkkCOM1",
            "uhCEkCOM1",
            "delegates-compute",
            "republish-epr",
        );

        handle_mishpat_signal(&mut conn, "app", signal).unwrap();

        let row = crate::db::mishpat_commitments::get_by_cid(&mut conn, "uhCEkCOM1")
            .unwrap()
            .expect("Row must be present after CommitmentCommitted signal");

        assert_eq!(row.cid, "uhCEkCOM1");
        assert_eq!(row.action, "delegates-compute");
        assert_eq!(row.scope, "republish-epr");
        assert_eq!(row.provider, "agent:matthew-steward");
        assert_eq!(row.state, "proposed");
        assert_eq!(
            row.dht_anchor_hash.as_deref(),
            Some("uhCkkCOM1"),
            "dht_anchor_hash must equal action_hash"
        );
        // bounds_json must be valid non-empty JSON
        let bounds: serde_json::Value =
            serde_json::from_str(&row.bounds_json).expect("bounds_json must be valid JSON");
        assert_eq!(bounds["reach_ceiling"], "commons");
    }

    /// I1c — CommitmentCommitted is idempotent: re-delivering the signal must
    /// not duplicate the row.
    #[test]
    fn commitment_committed_is_idempotent() {
        let mut conn = setup_commitments_conn();
        let signal = make_commitment_signal(
            "uhCkkCOM2",
            "uhCEkCOM2",
            "delegates-compute",
            "republish-epr",
        );

        handle_mishpat_signal(&mut conn, "app", signal.clone()).unwrap();
        handle_mishpat_signal(&mut conn, "app", signal).unwrap();

        // Row must exist exactly once — get_by_cid returning Some proves exactly one row.
        let row = crate::db::mishpat_commitments::get_by_cid(&mut conn, "uhCEkCOM2")
            .unwrap()
            .expect("Row must exist after two deliveries");
        assert_eq!(
            row.cid, "uhCEkCOM2",
            "Re-delivered CommitmentCommitted must not duplicate the row"
        );
    }

    // -------------------------------------------------------------------------
    // author-lens projection (lens-market S3) — CommitmentCommitted{author-lens}
    // projects a `lenses` A-class row (NOT a mishpat_commitments row).
    // -------------------------------------------------------------------------

    fn make_author_lens_signal(
        action_hash: &str,
        entry_hash: &str,
        governs_epr: &str,
        school: &str,
    ) -> MishpatSignal {
        let payload_json = serde_json::json!({
            "action": "author-lens",
            "governs_epr": governs_epr,
            "school": school,
            "role": "lens",
            "rule": { "predicate": "land_value_uplift > rent_capture" },
            "telos": { "summary": "tax unimproved land value, not labor" },
            "version_parent": serde_json::Value::Null
        })
        .to_string();

        MishpatSignal::CommitmentCommitted {
            action_hash: action_hash.into(),
            entry_hash: entry_hash.into(),
            author: "uhCAkAUTHOR".into(),
            commitment: crate::mishpat_projection::CommitmentPayload {
                action: "author-lens".to_string(),
                payload_json,
                signed_at: "2026-06-27T00:00:00Z".to_string(),
            },
        }
    }

    /// S3a — CommitmentCommitted{author-lens} projects ONE `lenses` row (A-class),
    /// keyed cid = entry_hash, dht_anchor_hash = action_hash. It must NOT land in
    /// mishpat_commitments (author-lens is a lens, not a compute-bounds commitment).
    #[test]
    fn author_lens_signal_projects_lens_row() {
        let mut conn = setup_commitments_conn();
        let signal =
            make_author_lens_signal("uhCkkLENS1", "uhCEkLENS1", "epr:lamad-spa", "georgist");

        handle_mishpat_signal(&mut conn, "app", signal).unwrap();

        let row = crate::db::lenses::get_by_cid(&mut conn, "uhCEkLENS1")
            .unwrap()
            .expect("lenses row must be present after author-lens CommitmentCommitted");
        assert_eq!(row.cid, "uhCEkLENS1", "cid = entry_hash");
        assert_eq!(row.governs_epr, "epr:lamad-spa");
        assert_eq!(row.school, "georgist");
        assert_eq!(row.role, "lens");
        assert_eq!(
            row.dht_anchor_hash.as_deref(),
            Some("uhCkkLENS1"),
            "dht_anchor_hash must equal action_hash (A-class provenance)"
        );
        // rule/telos round-trip as JSON.
        let telos: serde_json::Value =
            serde_json::from_str(&row.telos_json).expect("telos_json must be valid JSON");
        assert_eq!(telos["summary"], "tax unimproved land value, not labor");

        // It must NOT have leaked into mishpat_commitments.
        assert!(
            crate::db::mishpat_commitments::get_by_cid(&mut conn, "uhCEkLENS1")
                .unwrap()
                .is_none(),
            "author-lens must NOT project into mishpat_commitments"
        );
    }

    /// S3b — re-delivery is idempotent + anchor-preserving (one lens row).
    #[test]
    fn author_lens_signal_idempotent() {
        let mut conn = setup_commitments_conn();
        let signal =
            make_author_lens_signal("uhCkkLENS2", "uhCEkLENS2", "epr:lamad-spa", "beerian");

        handle_mishpat_signal(&mut conn, "app", signal.clone()).unwrap();
        handle_mishpat_signal(&mut conn, "app", signal).unwrap();

        let row = crate::db::lenses::get_by_cid(&mut conn, "uhCEkLENS2")
            .unwrap()
            .expect("lens row must exist after two deliveries");
        assert_eq!(row.school, "beerian");
        assert_eq!(row.dht_anchor_hash.as_deref(), Some("uhCkkLENS2"));
    }

    /// S3c — a malformed author-lens payload (missing governs_epr) is warn-skipped:
    /// no lens row, no error bubbles (per-row fail-closed degrade, the EprRouter lesson).
    #[test]
    fn author_lens_malformed_payload_skipped() {
        let mut conn = setup_commitments_conn();
        let payload_json = serde_json::json!({
            "action": "author-lens",
            // governs_epr deliberately absent
            "school": "georgist",
            "role": "lens",
            "rule": { "predicate": "x" },
            "telos": { "summary": "y" }
        })
        .to_string();
        let signal = MishpatSignal::CommitmentCommitted {
            action_hash: "uhCkkBAD".into(),
            entry_hash: "uhCEkBAD".into(),
            author: "uhCAkAUTHOR".into(),
            commitment: crate::mishpat_projection::CommitmentPayload {
                action: "author-lens".to_string(),
                payload_json,
                signed_at: "2026-06-27T00:00:00Z".to_string(),
            },
        };

        // Must NOT error — the handler degrades the row, never the whole signal.
        handle_mishpat_signal(&mut conn, "app", signal).expect("handler must not error on bad row");
        assert!(
            crate::db::lenses::get_by_cid(&mut conn, "uhCEkBAD")
                .unwrap()
                .is_none(),
            "malformed author-lens must not project a lens row"
        );
    }

    // -------------------------------------------------------------------------
    // Wire-conformance: the REAL conductor msgpack wire (the 2026-06-12 fix)
    // -------------------------------------------------------------------------
    //
    // The JSON-context tests above feed `HoloHashB64` base64 STRINGS, which
    // prove serde tag/field shape but say NOTHING about the conductor wire.
    // The conductor emits app signals as MessagePack (`ExternIO`), where
    // `ActionHash`/`EntryHash`/`AgentPubKey` serialize as raw 39-BYTE ARRAYS.
    // The old `String` mirror (+ the `rmp → serde_json::Value` pre-pass before
    // it) failed on every such signal, silently dropping `CommitmentCommitted`
    // and leaving the Epic B provide projection dead in production.

    /// THE root-cause regression test: decode `CommitmentCommitted` from the
    /// REAL conductor wire — `rmp_serde::to_vec_named` over the DNA-side enum
    /// shape with real `holo_hash` types — and assert the holo-hash fields
    /// round-trip to canonical base64 while the all-`String` `Commitment`
    /// payload survives intact.
    #[test]
    fn commitment_committed_decodes_from_conductor_msgpack_wire() {
        use holochain_types::prelude::{ActionHash, AgentPubKey, EntryHash};

        // DNA-side shape: mishpat zome's `MishpatSignal::CommitmentCommitted`
        // with REAL holo_hash types and a `Commitment` struct field-identical
        // to `mishpat_integrity::Commitment` (action / payload_json / signed_at —
        // all plain Strings, verified against the integrity crate).
        #[derive(Serialize)]
        struct DnaCommitment {
            action: String,
            payload_json: String,
            signed_at: String,
        }
        #[derive(Serialize)]
        #[serde(tag = "type", content = "payload")]
        enum DnaMishpatSignal {
            CommitmentCommitted {
                action_hash: ActionHash,
                entry_hash: EntryHash,
                commitment: DnaCommitment,
                author: AgentPubKey,
            },
        }

        let action_hash = ActionHash::from_raw_36(vec![0x11; 36]);
        let entry_hash = EntryHash::from_raw_36(vec![0x22; 36]);
        let author = AgentPubKey::from_raw_36(vec![0x33; 36]);

        let payload_json = r#"{"action":"replicates-commons","scope":"content","provider":"uhCAkPROVIDER","reach_ceiling":"commons"}"#.to_string();
        let dna_signal = DnaMishpatSignal::CommitmentCommitted {
            action_hash: action_hash.clone(),
            entry_hash: entry_hash.clone(),
            commitment: DnaCommitment {
                action: "replicates-commons".into(),
                payload_json: payload_json.clone(),
                signed_at: "1748390400".into(),
            },
            author: author.clone(),
        };

        // emit_signal encodes via ExternIO == holochain_serialized_bytes ==
        // rmp_serde::to_vec_named.
        let wire = rmp_serde::to_vec_named(&dna_signal).expect("encode DNA signal");

        let before = mishpat_decode_miss_count();
        let decoded =
            decode_mishpat_signal(&wire).expect("conductor msgpack wire format must decode");
        match decoded {
            MishpatSignal::CommitmentCommitted {
                action_hash: got_action,
                entry_hash: got_entry,
                author: got_author,
                commitment,
            } => {
                // Normalized to holochain's canonical base64 ("u" + base64url-
                // no-pad over the raw 39 bytes) — matches holo_hash Display.
                assert_eq!(got_action.0, format!("{action_hash}"));
                assert_eq!(got_entry.0, format!("{entry_hash}"));
                assert_eq!(got_author.0, format!("{author}"));
                // The all-String Commitment payload survives the wire intact.
                assert_eq!(commitment.action, "replicates-commons");
                assert_eq!(commitment.payload_json, payload_json);
                assert_eq!(commitment.signed_at, "1748390400");
            }
            _ => panic!("Expected CommitmentCommitted variant"),
        }
        assert_eq!(
            mishpat_decode_miss_count(),
            before,
            "a successful decode must not count as a miss"
        );
    }

    /// End-to-end: a `CommitmentCommitted` decoded from the REAL conductor wire
    /// drives `handle_mishpat_signal` and projects a `mishpat_commitments` row —
    /// proving the wire fix actually reaches the projection the old path
    /// silently starved.
    #[test]
    fn conductor_wire_commitment_projects_row() {
        use holochain_types::prelude::{ActionHash, AgentPubKey, EntryHash};

        #[derive(Serialize)]
        struct DnaCommitment {
            action: String,
            payload_json: String,
            signed_at: String,
        }
        #[derive(Serialize)]
        #[serde(tag = "type", content = "payload")]
        enum DnaMishpatSignal {
            CommitmentCommitted {
                action_hash: ActionHash,
                entry_hash: EntryHash,
                commitment: DnaCommitment,
                author: AgentPubKey,
            },
        }

        let action_hash = ActionHash::from_raw_36(vec![0xA1; 36]);
        let entry_hash = EntryHash::from_raw_36(vec![0xB2; 36]);
        let author = AgentPubKey::from_raw_36(vec![0xC3; 36]);
        let expected_cid = format!("{entry_hash}");

        let payload_json = serde_json::json!({
            "action": "delegates-compute",
            "scope": "republish-epr",
            "provider": "agent:matthew-steward",
            "recipient": "agent:deploy-svc",
            "bounds": { "reach_ceiling": "commons", "rate_per_hour": 30 },
            "valid_from": "2026-05-28T00:00:00Z",
            "valid_until": "2026-08-26T00:00:00Z"
        })
        .to_string();

        let dna_signal = DnaMishpatSignal::CommitmentCommitted {
            action_hash,
            entry_hash,
            commitment: DnaCommitment {
                action: "delegates-compute".into(),
                payload_json,
                signed_at: "1748390400".into(),
            },
            author,
        };
        let wire = rmp_serde::to_vec_named(&dna_signal).expect("encode");
        let signal = decode_mishpat_signal(&wire).expect("decode from wire");

        let mut conn = setup_commitments_conn();
        handle_mishpat_signal(&mut conn, "app", signal).unwrap();

        let row = crate::db::mishpat_commitments::get_by_cid(&mut conn, &expected_cid)
            .unwrap()
            .expect("Row must be present after conductor-wire CommitmentCommitted");
        assert_eq!(row.cid, expected_cid);
        assert_eq!(row.action, "delegates-compute");
        assert_eq!(
            row.dht_anchor_hash.as_deref(),
            Some(format!("{}", ActionHash::from_raw_36(vec![0xA1; 36])).as_str()),
            "dht_anchor_hash must equal the canonical action_hash"
        );
    }

    /// A foreign signal (infrastructure, content, …) shares the app interface —
    /// it must classify quiet, not as a mishpat miss.
    #[test]
    fn foreign_signal_classifies_quiet_for_mishpat() {
        #[derive(Serialize)]
        #[serde(tag = "type", content = "payload")]
        enum OtherSignal {
            PeerStatusRecorded { peer_id: String },
        }
        let wire = rmp_serde::to_vec_named(&OtherSignal::PeerStatusRecorded {
            peer_id: "x".into(),
        })
        .expect("encode");
        match decode_mishpat_signal(&wire) {
            Err(SignalDecodeMiss::ForeignSignal { type_tag }) => {
                assert_eq!(type_tag, "PeerStatusRecorded");
            }
            other => panic!("expected ForeignSignal, got {other:?}"),
        }
    }

    /// A mishpat mirror-variant tag whose payload no longer decodes is a REAL
    /// miss — it must be loud (counted).
    #[test]
    fn shape_mismatch_on_mishpat_mirror_variant_is_counted() {
        #[derive(Serialize)]
        #[serde(tag = "type", content = "payload")]
        enum BrokenShape {
            CommitmentCommitted { unexpected_only_field: bool },
        }
        let wire = rmp_serde::to_vec_named(&BrokenShape::CommitmentCommitted {
            unexpected_only_field: true,
        })
        .expect("encode");
        let before = mishpat_decode_miss_count();
        match decode_mishpat_signal(&wire) {
            Err(SignalDecodeMiss::MishpatShapeMismatch { type_tag, .. }) => {
                assert_eq!(type_tag, "CommitmentCommitted");
            }
            other => panic!("expected MishpatShapeMismatch, got {other:?}"),
        }
        assert_eq!(mishpat_decode_miss_count(), before + 1);
    }
}

#[cfg(test)]
mod peer_status_tests {
    use super::*;
    use crate::db::peer_statuses::get_by_peer;
    use diesel::connection::SimpleConnection;
    use diesel::prelude::*;
    use diesel::sqlite::SqliteConnection;

    fn setup_test_conn() -> SqliteConnection {
        let mut conn =
            SqliteConnection::establish(":memory:").expect("Failed to create in-memory database");

        conn.batch_execute(
            r#"
            CREATE TABLE peer_statuses (
                peer_id TEXT PRIMARY KEY,
                status TEXT NOT NULL,
                general_pool_member INTEGER NOT NULL,
                accepting_stewardship_reserves INTEGER NOT NULL,
                archetype_class TEXT,
                timestamp BIGINT NOT NULL,
                dht_anchor_hash TEXT NOT NULL,
                updated_at BIGINT NOT NULL
            );
            CREATE INDEX idx_peer_statuses_status ON peer_statuses(status);
            CREATE INDEX idx_peer_statuses_pool ON peer_statuses(general_pool_member);
            "#,
        )
        .expect("Failed to create test table");

        conn
    }

    #[test]
    fn peer_status_recorded_upserts_row() {
        let mut conn = setup_test_conn();

        let signal = InfrastructureSignal::PeerStatusRecorded {
            peer_id: "uhCAkABC".into(),
            status: "online".into(),
            general_pool_member: true,
            accepting_stewardship_reserves: false,
            archetype_class: Some("home-nuc".into()),
            timestamp: 1_700_000_000_000_000,
            action_hash: "uhCkkDEF".into(),
        };

        handle_signal(&mut conn, signal).unwrap();

        let row = get_by_peer(&mut conn, "uhCAkABC").unwrap().unwrap();
        assert_eq!(row.status, "online");
        assert_eq!(row.general_pool_member, 1);
        assert_eq!(row.accepting_stewardship_reserves, 0);
        assert_eq!(row.archetype_class.as_deref(), Some("home-nuc"));
        assert_eq!(row.dht_anchor_hash, "uhCkkDEF");
    }

    #[test]
    fn peer_status_recorded_updates_existing_row() {
        let mut conn = setup_test_conn();

        let signal = |status: &str| InfrastructureSignal::PeerStatusRecorded {
            peer_id: "uhCAkABC".into(),
            status: status.into(),
            general_pool_member: true,
            accepting_stewardship_reserves: true,
            archetype_class: None,
            timestamp: 1,
            action_hash: "uhCkkDEF".into(),
        };
        handle_signal(&mut conn, signal("starting")).unwrap();
        handle_signal(&mut conn, signal("online")).unwrap();

        let row = get_by_peer(&mut conn, "uhCAkABC").unwrap().unwrap();
        assert_eq!(
            row.status, "online",
            "upsert should have replaced 'starting'"
        );
    }

    #[test]
    fn serde_tag_matches_dna_wire_format() {
        // Verify the serde(tag, content) shape matches the DNA-side signal
        // exactly: `{"type": "PeerStatusRecorded", "payload": { ... }}`.
        // Deserialization drift here would silently break projection.
        let wire = serde_json::json!({
            "type": "PeerStatusRecorded",
            "payload": {
                "peer_id": "uhCAkABC",
                "status": "online",
                "general_pool_member": true,
                "accepting_stewardship_reserves": false,
                "archetype_class": "home-nuc",
                "timestamp": 1_700_000_000_000_000_i64,
                "action_hash": "uhCkkDEF",
            }
        });
        let signal: InfrastructureSignal = serde_json::from_value(wire).unwrap();
        match signal {
            InfrastructureSignal::PeerStatusRecorded {
                peer_id, status, ..
            } => {
                assert_eq!(peer_id.0, "uhCAkABC");
                assert_eq!(status, "online");
            }
        }
    }

    /// THE 2026-06-12 root-cause regression test: decode the signal from the
    /// REAL conductor wire format — MessagePack (`ExternIO`), where
    /// `AgentPubKey`/`ActionHash` serialize as raw 39-BYTE ARRAYS, not base64
    /// strings. The old `String`-typed mirror (and the `rmp →
    /// serde_json::Value` pre-pass before it) failed on every such signal,
    /// silently emptying `peer_statuses`.
    #[test]
    fn peer_status_signal_decodes_from_conductor_msgpack_wire() {
        use holochain_types::prelude::{ActionHash, AgentPubKey};

        // DNA-side shape: the infrastructure zome's enum with real holo_hash
        // types (see elohim/holochain/dna/infrastructure/.../lib.rs).
        #[derive(Serialize)]
        #[serde(tag = "type", content = "payload")]
        enum DnaInfrastructureSignal {
            PeerStatusRecorded {
                peer_id: AgentPubKey,
                status: String,
                general_pool_member: bool,
                accepting_stewardship_reserves: bool,
                archetype_class: Option<String>,
                timestamp: i64,
                action_hash: ActionHash,
            },
        }

        let peer_id = AgentPubKey::from_raw_36(vec![0x11; 36]);
        let action_hash = ActionHash::from_raw_36(vec![0x22; 36]);
        let dna_signal = DnaInfrastructureSignal::PeerStatusRecorded {
            peer_id: peer_id.clone(),
            status: "online".into(),
            general_pool_member: true,
            accepting_stewardship_reserves: false,
            archetype_class: Some("home-nuc".into()),
            timestamp: 1_700_000_000_000_000,
            action_hash: action_hash.clone(),
        };

        // emit_signal encodes via ExternIO == holochain_serialized_bytes ==
        // rmp_serde::to_vec_named.
        let wire = rmp_serde::to_vec_named(&dna_signal).expect("encode DNA signal");

        let decoded =
            decode_infrastructure_signal(&wire).expect("conductor msgpack wire format must decode");
        match decoded {
            InfrastructureSignal::PeerStatusRecorded {
                peer_id: got_peer,
                status,
                action_hash: got_action,
                ..
            } => {
                assert_eq!(status, "online");
                // Normalized form must match holochain's canonical base64
                // ("u" + base64url-no-pad over the raw 39 bytes).
                assert_eq!(got_peer.0, format!("{peer_id}"));
                assert_eq!(got_action.0, format!("{action_hash}"));
            }
        }
        assert_eq!(
            infrastructure_decode_miss_count(),
            0,
            "a successful decode must not count as a miss"
        );
    }

    /// Foreign signals (lamad content, REA, …) share the app interface — they
    /// must classify quiet, not as infrastructure misses.
    #[test]
    fn foreign_signal_classifies_quiet() {
        #[derive(Serialize)]
        #[serde(tag = "type", content = "payload")]
        enum OtherSignal {
            ContentCommitted { id: String },
        }
        let wire = rmp_serde::to_vec_named(&OtherSignal::ContentCommitted { id: "x".into() })
            .expect("encode");
        match decode_infrastructure_signal(&wire) {
            Err(SignalDecodeMiss::ForeignSignal { type_tag }) => {
                assert_eq!(type_tag, "ContentCommitted");
            }
            other => panic!("expected ForeignSignal, got {other:?}"),
        }
    }

    /// A mirror-variant tag whose payload no longer decodes is a REAL miss —
    /// it must be loud (counted).
    #[test]
    fn shape_mismatch_on_mirror_variant_is_counted() {
        #[derive(Serialize)]
        #[serde(tag = "type", content = "payload")]
        enum BrokenShape {
            PeerStatusRecorded { unexpected_only_field: bool },
        }
        let wire = rmp_serde::to_vec_named(&BrokenShape::PeerStatusRecorded {
            unexpected_only_field: true,
        })
        .expect("encode");
        let before = infrastructure_decode_miss_count();
        match decode_infrastructure_signal(&wire) {
            Err(SignalDecodeMiss::InfraShapeMismatch { type_tag, .. }) => {
                assert_eq!(type_tag, "PeerStatusRecorded");
            }
            other => panic!("expected InfraShapeMismatch, got {other:?}"),
        }
        assert_eq!(infrastructure_decode_miss_count(), before + 1);
    }
}

#[cfg(test)]
mod recovery_v2_signal_tests {
    use super::*;
    use diesel::connection::SimpleConnection;
    use diesel::prelude::*;
    use diesel::sqlite::SqliteConnection;

    /// Create an in-memory connection with only the tables needed for these tests.
    fn setup_test_conn() -> SqliteConnection {
        let mut conn = SqliteConnection::establish(":memory:").expect("in-memory SQLite");
        conn.batch_execute(include_str!(
            "../migrations/2026-04-24-000000_recovery_witnesses/up.sql"
        ))
        .expect("create recovery_witnesses table");
        conn
    }

    #[test]
    fn dispatches_intimate_witness_submitted() {
        let mut conn = setup_test_conn();
        let signal = RecoveryV2Signal::IntimateWitnessSubmitted {
            action_hash: "W1".into(),
            request_hash: "R1".into(),
            witness: HumanityWitnessPayload {
                human_id: "H1".into(),
                witness_agent_id: "A1".into(),
                attestation_type: "intimate_recovery".into(),
                note: Some("recognized".into()),
                issued_at: serde_json::json!(1_700_000_000_000_000_i64),
                revoked_at: None,
            },
            witness_agent_id: "A1".into(),
        };
        handle_recovery_v2_signal(&mut conn, signal).expect("dispatch ok");
        let count =
            crate::db::recovery_witnesses::count_witnesses_for_request(&mut conn, "R1").unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn intimate_witness_submitted_is_idempotent() {
        let mut conn = setup_test_conn();
        let signal = RecoveryV2Signal::IntimateWitnessSubmitted {
            action_hash: "W2".into(),
            request_hash: "R2".into(),
            witness: HumanityWitnessPayload {
                human_id: "H2".into(),
                witness_agent_id: "A2".into(),
                attestation_type: "intimate_recovery".into(),
                note: None,
                issued_at: serde_json::json!(1_700_000_000_000_000_i64),
                revoked_at: None,
            },
            witness_agent_id: "A2".into(),
        };
        handle_recovery_v2_signal(&mut conn, signal.clone()).expect("first dispatch");
        handle_recovery_v2_signal(&mut conn, signal).expect("second dispatch (idempotent)");
        let count =
            crate::db::recovery_witnesses::count_witnesses_for_request(&mut conn, "R2").unwrap();
        assert_eq!(count, 1, "Replayed signal must not duplicate the row");
    }

    #[test]
    fn intimate_witness_submitted_serde_tag_matches_dna() {
        // Verify the internally-tagged enum shape matches the DNA-side RecoveryV2Signal.
        let wire = serde_json::json!({
            "type": "IntimateWitnessSubmitted",
            "action_hash": "uhCkkW1",
            "request_hash": "uhCkkR1",
            "witness": {
                "human_id": "human-123",
                "witness_agent_id": "uhCAkWIT",
                "attestation_type": "intimate_recovery",
                "note": null,
                "issued_at": 1_700_000_000_000_000_i64,
                "revoked_at": null,
            },
            "witness_agent_id": "uhCAkWIT",
        });
        let signal: RecoveryV2Signal = serde_json::from_value(wire).unwrap();
        match signal {
            RecoveryV2Signal::IntimateWitnessSubmitted {
                action_hash,
                request_hash,
                witness,
                witness_agent_id,
            } => {
                assert_eq!(action_hash, "uhCkkW1");
                assert_eq!(request_hash, "uhCkkR1");
                assert_eq!(witness.human_id, "human-123");
                assert_eq!(witness_agent_id, "uhCAkWIT");
            }
            _ => panic!("Expected IntimateWitnessSubmitted variant"),
        }
    }

    #[test]
    fn publish_intent_from_request_created_has_fields() {
        let signal = RecoveryV2Signal::RecoveryRequestCreated {
            action_hash: "uhCkkR1".into(),
            request: RecoveryRequestPayload {
                human_agent_pubkey: "uhCAkHAP".into(),
                new_agent_pubkey: "uhCAkNAP".into(),
                hosting_doorway_pubkey: "uhCAkHDP".into(),
                proposed_authority: serde_json::json!({"type": "IntimateQuorum"}),
                request_nonce: vec![0u8; 16],
                human_id: Some("human-abc".into()),
                required_witness_count: 3,
                created_at: serde_json::json!(1_700_000_000_000_000_i64),
            },
        };
        let inv = recovery_invitation_from_signal(&signal).expect("some");
        assert_eq!(inv.request_hash, "uhCkkR1");
        assert_eq!(inv.human_id, "human-abc");
        assert!(!inv.created_at.is_empty());
    }

    #[test]
    fn publish_intent_is_none_without_human_id() {
        let signal = RecoveryV2Signal::RecoveryRequestCreated {
            action_hash: "uhCkkR2".into(),
            request: RecoveryRequestPayload {
                human_agent_pubkey: "x".into(),
                new_agent_pubkey: "y".into(),
                hosting_doorway_pubkey: "z".into(),
                proposed_authority: serde_json::json!({}),
                request_nonce: vec![],
                human_id: None,
                required_witness_count: 2,
                created_at: serde_json::json!(0_i64),
            },
        };
        assert!(recovery_invitation_from_signal(&signal).is_none());
    }

    #[test]
    fn publish_intent_is_none_for_other_variants() {
        let signal = RecoveryV2Signal::IntimateWitnessSubmitted {
            action_hash: "W".into(),
            request_hash: "R".into(),
            witness: HumanityWitnessPayload {
                human_id: "H".into(),
                witness_agent_id: "A".into(),
                attestation_type: "intimate_recovery".into(),
                note: None,
                issued_at: serde_json::json!(0_i64),
                revoked_at: None,
            },
            witness_agent_id: "A".into(),
        };
        assert!(recovery_invitation_from_signal(&signal).is_none());
    }
}

// =============================================================================
// ElohimContentSignal — post-commit signal for attestation + governance-action
// Content entries from the elohim DNA.
// =============================================================================
//
// These signals are emitted by the elohim coordinator zome after a Content
// entry whose `content_type` starts with `attestation:` or `governance-action:`
// is committed to the source chain.
//
// Field notes:
// - `entry_hash`: ActionHash (base64 String) — stored as Vec<u8> after b64 decode
//   in the projector. This field IS the dht_anchor_hash for these projections.
// - `metadata_json`: JSON-stringified metadata field from the Content entry.
//   The projector unpacks known keys (subject_cid, vote_value, threshold, etc.).
// - `author_id`: AgentPubKey of the committing agent (base64 String on wire).

/// Post-commit signal for a Content entry whose content_type starts with
/// `attestation:` or `governance-action:`.
///
/// Emitted by the elohim coordinator zome. The `AttestationProjector`
/// subscribes to this signal and upserts the appropriate projection table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElohimContentSignal {
    /// CID of the Content entry — used as the projection table primary key.
    pub id: String,
    /// `content_type` field from the Content entry (e.g. `attestation:humanness`,
    /// `governance-action:recovery`).
    pub content_type: String,
    /// ActionHash of the committed DHT entry (base64 String on the wire).
    /// Stored as `dht_anchor_hash` in projection tables.
    pub entry_hash: String,
    /// JSON-encoded metadata field from the Content entry.
    pub metadata_json: String,
    /// AgentPubKey of the committing agent (base64 String).
    pub author_id: Option<String>,
    /// `title` field from the Content entry.
    pub title: String,
    /// `description` field from the Content entry (may be empty).
    pub description: String,
    /// ISO 8601 / RFC 3339 creation timestamp.
    pub created_at: String,
}

// -----------------------------------------------------------------------------
// Conductor-wire decode for the elohim-content projection (2026-06-13).
//
// THE STRUCTURAL TRAP (worse than the plain holo_hash class): the storage
// `ElohimContentSignal` above is a FLAT struct (`id`, `content_type`,
// `entry_hash`, `metadata_json`, `author_id`, `title`, `description`,
// `created_at`) — but the elohim DNA never emits that shape. The only Content
// post-commit signal the DNA emits is the TAGGED-enum variant
// `ProjectionSignal::ContentCommitted { action_hash: ActionHash, entry_hash:
// EntryHash, content: Content, author: AgentPubKey }` (content_store/src/lib.rs
// ~10392/~10639). So the old `subscribe_elohim_content_signals` —
// `rmp → serde_json::Value → from_value::<ElohimContentSignal>` — failed on
// EVERY real signal twice over: (1) the `Value` pre-pass cannot represent the
// holo_hash byte arrays, and (2) even past that, the flat field names never
// matched the `{type, payload:{...}}` envelope. The attestation + recovery
// projections have been dark since wiring.
//
// The honest fix is to decode the REAL wire envelope (`ContentCommitted`) and
// translate into the flat `ElohimContentSignal` the dispatcher/projectors
// consume — filtering to the `attestation:*` / `governance-action:*` content
// this subscriber owns (every other Content type is handled by the REA
// `ContentCommitted` arm and is a quiet skip here). The flat struct stays the
// dispatcher's input contract; only the SUBSCRIBER changes.
//
// `entry_hash` (flat) is sourced from the wire `action_hash` — the codebase-wide
// `dht_anchor_hash` convention (the REA arm threads `action_hash` as the anchor;
// the projectors store `signal.entry_hash` AS `dht_anchor_hash`). `author_id`
// (flat) comes from the Content entry's own `author_id: Option<String>` domain
// field, NOT the wire `author: AgentPubKey` (the committing agent).

/// Storage-side mirror of the DNA `ProjectionSignal::ContentCommitted` wire
/// envelope — the ONLY Content post-commit signal the elohim DNA emits. The
/// embedded `content` reuses `rea_projection::ContentEntry` (the verified
/// field-by-field mirror of the DNA `Content` entry; all-`String`, no holo_hash
/// fields). The three envelope hashes are `HoloHashB64` because they are
/// `ActionHash`/`EntryHash`/`AgentPubKey` on the DNA side (raw byte arrays on
/// the MessagePack wire).
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", content = "payload")]
enum ElohimContentWireSignal {
    ContentCommitted {
        action_hash: HoloHashB64,
        #[serde(default)]
        #[allow(dead_code)]
        entry_hash: Option<HoloHashB64>,
        content: crate::rea_projection::ContentEntry,
        #[serde(default)]
        #[allow(dead_code)]
        author: Option<HoloHashB64>,
    },
}

/// Mirror-variant tags for the elohim-content family. `ContentCommitted` is
/// shared with the REA subscriber's content arm, but on THIS subscriber a
/// decode failure of a `ContentCommitted` envelope is still a real wire-shape
/// regression worth shouting about.
const ELOHIM_CONTENT_MIRROR_VARIANTS: &[&str] = &["ContentCommitted"];

/// Cumulative count of REAL elohim-content decode misses this process.
static ELOHIM_CONTENT_DECODE_MISSES: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(0);

/// Cumulative REAL elohim-content decode misses (a `ContentCommitted` envelope
/// that failed the typed decode). Exposed for status surfaces / tests.
pub fn elohim_content_decode_miss_count() -> u64 {
    ELOHIM_CONTENT_DECODE_MISSES.load(std::sync::atomic::Ordering::Relaxed)
}

/// True for the content types this subscriber owns (attestation + governance
/// action). All other Content entries belong to the REA `ContentCommitted`
/// arm and are a quiet skip here.
fn is_elohim_content_owned(content_type: &str) -> bool {
    content_type.starts_with("attestation:") || content_type.starts_with("governance-action:")
}

/// Decode a conductor app-signal payload into the flat `ElohimContentSignal`
/// the dispatcher consumes, by decoding the REAL `ContentCommitted` wire
/// envelope and translating.
///
/// Returns:
/// - `Ok(Some(signal))` — a `ContentCommitted` for an `attestation:*` /
///   `governance-action:*` entry, translated to the flat shape.
/// - `Ok(None)` — a `ContentCommitted` for some OTHER content type (handled by
///   the REA subscriber; a quiet, expected skip here).
/// - `Err(ElohimContentShapeMismatch)` — a `ContentCommitted` envelope that
///   failed the typed decode (a real wire regression; LOUD).
/// - `Err(ForeignSignal)` / `Err(Undecodable)` — another family / unreadable.
pub fn decode_elohim_content_signal(
    bytes: &[u8],
) -> Result<Option<ElohimContentSignal>, SignalDecodeMiss> {
    match rmp_serde::from_slice::<ElohimContentWireSignal>(bytes) {
        Ok(ElohimContentWireSignal::ContentCommitted {
            action_hash,
            content,
            ..
        }) => {
            if !is_elohim_content_owned(&content.content_type) {
                return Ok(None);
            }
            // Translate the DNA `Content` entry → the flat projector contract.
            // `entry_hash` (flat) = the wire `action_hash` (the dht_anchor_hash
            // convention used everywhere else in this codebase).
            Ok(Some(ElohimContentSignal {
                id: content.id,
                content_type: content.content_type,
                entry_hash: action_hash.0,
                metadata_json: content.metadata_json,
                author_id: content.author_id,
                title: content.title,
                description: content.description,
                created_at: content.created_at,
            }))
        }
        Err(typed_err) => {
            // Tag-only peek: serde skips the payload via IgnoredAny (which,
            // unlike `serde_json::Value`, tolerates msgpack byte arrays).
            #[derive(Deserialize)]
            struct TagOnly {
                #[serde(rename = "type")]
                type_tag: String,
            }
            match rmp_serde::from_slice::<TagOnly>(bytes) {
                Ok(tag) if ELOHIM_CONTENT_MIRROR_VARIANTS.contains(&tag.type_tag.as_str()) => {
                    ELOHIM_CONTENT_DECODE_MISSES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    Err(SignalDecodeMiss::ElohimContentShapeMismatch {
                        type_tag: tag.type_tag,
                        error: typed_err.to_string(),
                    })
                }
                Ok(tag) => Err(SignalDecodeMiss::ForeignSignal {
                    type_tag: tag.type_tag,
                }),
                Err(tag_err) => Err(SignalDecodeMiss::Undecodable {
                    error: format!("typed: {typed_err}; tag peek: {tag_err}"),
                }),
            }
        }
    }
}

// =============================================================================
// ElohimContent wire-conformance tests
// =============================================================================

#[cfg(test)]
mod elohim_content_signal_tests {
    use super::*;
    use holochain_types::prelude::{ActionHash, AgentPubKey, EntryHash};
    use serde::Serialize;

    /// DNA-side `Content` entry shape (the subset relevant to projection),
    /// all-`String`/`Option<String>` — matches content_store_integrity::Content.
    #[derive(Serialize)]
    struct DnaContent {
        id: String,
        content_type: String,
        title: String,
        description: String,
        content: String,
        content_format: String,
        tags: Vec<String>,
        related_node_ids: Vec<String>,
        author_id: Option<String>,
        reach: String,
        trust_score: f64,
        metadata_json: String,
        created_at: String,
        updated_at: String,
        schema_version: u32,
        validation_status: String,
    }

    /// DNA-side envelope: the ONLY Content post-commit signal the elohim DNA
    /// emits — `ProjectionSignal::ContentCommitted { action_hash, entry_hash,
    /// content, author }` with REAL holo_hash types.
    #[derive(Serialize)]
    #[serde(tag = "type", content = "payload")]
    enum DnaProjectionSignal {
        ContentCommitted {
            action_hash: ActionHash,
            entry_hash: EntryHash,
            content: DnaContent,
            author: AgentPubKey,
        },
    }

    fn dna_content(content_type: &str) -> DnaContent {
        DnaContent {
            id: "cid-attestation-001".into(),
            content_type: content_type.into(),
            title: "Humanness attestation".into(),
            description: "I vouch".into(),
            content: "".into(),
            content_format: "markdown".into(),
            tags: vec![],
            related_node_ids: vec![],
            author_id: Some("did:key:zAuthor".into()),
            reach: "local".into(),
            trust_score: 1.0,
            metadata_json: "{\"subject_cid\":\"cid-xyz\"}".into(),
            created_at: "2026-06-13T12:00:00Z".into(),
            updated_at: "2026-06-13T12:00:00Z".into(),
            schema_version: 1,
            validation_status: "valid".into(),
        }
    }

    /// THE 2026-06-13 root-cause regression test (elohim-content arm): the DNA
    /// emits the TAGGED `ContentCommitted` envelope with holo_hash byte arrays;
    /// the old `rmp → serde_json::Value → from_value::<ElohimContentSignal>`
    /// (flat struct) path failed twice over. This decodes the real envelope and
    /// asserts the flat translation lands — including `entry_hash := action_hash`
    /// (the dht_anchor convention) and `author_id` from the Content entry.
    #[test]
    fn elohim_content_signal_decodes_and_translates_from_conductor_msgpack_wire() {
        let action_hash = ActionHash::from_raw_36(vec![0x44; 36]);
        let entry_hash = EntryHash::from_raw_36(vec![0x55; 36]);
        let author = AgentPubKey::from_raw_36(vec![0x66; 36]);
        let dna_signal = DnaProjectionSignal::ContentCommitted {
            action_hash: action_hash.clone(),
            entry_hash,
            content: dna_content("attestation:humanness"),
            author,
        };

        let wire = rmp_serde::to_vec_named(&dna_signal).expect("encode DNA content signal");

        let decoded = decode_elohim_content_signal(&wire)
            .expect("conductor msgpack wire must decode")
            .expect("an attestation:* ContentCommitted must translate to Some(signal)");

        assert_eq!(decoded.id, "cid-attestation-001");
        assert_eq!(decoded.content_type, "attestation:humanness");
        // entry_hash (flat) is the wire action_hash, normalized to canonical b64.
        assert_eq!(decoded.entry_hash, format!("{action_hash}"));
        assert_eq!(decoded.author_id.as_deref(), Some("did:key:zAuthor"));
        assert_eq!(decoded.title, "Humanness attestation");
        assert_eq!(decoded.metadata_json, "{\"subject_cid\":\"cid-xyz\"}");
        assert_eq!(decoded.created_at, "2026-06-13T12:00:00Z");
        // (The miss counter is process-global; exact before/after assertions
        // race under parallel tests, so counting is asserted only in the
        // dedicated `elohim_content_shape_mismatch_is_counted` test.)
    }

    /// A `governance-action:*` ContentCommitted is also owned by this subscriber.
    #[test]
    fn governance_action_content_translates() {
        let dna_signal = DnaProjectionSignal::ContentCommitted {
            action_hash: ActionHash::from_raw_36(vec![0x44; 36]),
            entry_hash: EntryHash::from_raw_36(vec![0x55; 36]),
            content: dna_content("governance-action:recovery-request"),
            author: AgentPubKey::from_raw_36(vec![0x66; 36]),
        };
        let wire = rmp_serde::to_vec_named(&dna_signal).expect("encode");
        let decoded = decode_elohim_content_signal(&wire)
            .expect("must decode")
            .expect("governance-action:* must translate to Some");
        assert_eq!(decoded.content_type, "governance-action:recovery-request");
    }

    /// A ContentCommitted for any OTHER content_type is owned by the REA
    /// subscriber — this subscriber must quietly skip it (`Ok(None)`), NOT
    /// treat it as a miss.
    #[test]
    fn non_attestation_content_is_quiet_skip() {
        let dna_signal = DnaProjectionSignal::ContentCommitted {
            action_hash: ActionHash::from_raw_36(vec![0x44; 36]),
            entry_hash: EntryHash::from_raw_36(vec![0x55; 36]),
            content: dna_content("concept"),
            author: AgentPubKey::from_raw_36(vec![0x66; 36]),
        };
        let wire = rmp_serde::to_vec_named(&dna_signal).expect("encode");
        match decode_elohim_content_signal(&wire) {
            Ok(None) => {}
            other => panic!("expected Ok(None) for non-owned content_type, got {other:?}"),
        }
    }

    /// A signal from another family classifies quiet.
    #[test]
    fn elohim_content_foreign_signal_classifies_quiet() {
        #[derive(Serialize)]
        #[serde(tag = "type", content = "payload")]
        enum OtherSignal {
            PeerStatusRecorded { status: String },
        }
        let wire = rmp_serde::to_vec_named(&OtherSignal::PeerStatusRecorded {
            status: "online".into(),
        })
        .expect("encode");
        match decode_elohim_content_signal(&wire) {
            Err(SignalDecodeMiss::ForeignSignal { type_tag }) => {
                assert_eq!(type_tag, "PeerStatusRecorded");
            }
            other => panic!("expected ForeignSignal, got {other:?}"),
        }
    }

    /// A `ContentCommitted` envelope whose payload no longer decodes is a REAL
    /// miss — loud (counted).
    #[test]
    fn elohim_content_shape_mismatch_is_counted() {
        #[derive(Serialize)]
        #[serde(tag = "type", content = "payload")]
        enum BrokenShape {
            ContentCommitted { unexpected_only_field: bool },
        }
        let wire = rmp_serde::to_vec_named(&BrokenShape::ContentCommitted {
            unexpected_only_field: true,
        })
        .expect("encode");
        // Strictly-increased, not exactly +1: the counter is process-global and
        // parallel tests may also increment it concurrently.
        let before = elohim_content_decode_miss_count();
        match decode_elohim_content_signal(&wire) {
            Err(SignalDecodeMiss::ElohimContentShapeMismatch { type_tag, .. }) => {
                assert_eq!(type_tag, "ContentCommitted");
            }
            other => panic!("expected ElohimContentShapeMismatch, got {other:?}"),
        }
        assert!(elohim_content_decode_miss_count() > before);
    }

    /// JSON-context producers (the legacy fixture shape) keep working —
    /// HoloHashB64 accepts strings, so a string-hash envelope still decodes.
    #[test]
    fn json_string_hashes_still_decode() {
        let wire = serde_json::to_vec(&serde_json::json!({
            "type": "ContentCommitted",
            "payload": {
                "action_hash": "uhCkk-anchor",
                "entry_hash": "uhCEk-entry",
                "content": {
                    "id": "cid-1",
                    "content_type": "attestation:humanness",
                    "title": "t",
                    "description": "d",
                    "content_format": "markdown",
                    "reach": "local",
                    "metadata_json": "{}",
                },
                "author": "uhCAk-author"
            }
        }))
        .expect("encode json");
        // serde_json path: decode the wire enum directly (rmp path is the
        // production one; this asserts the string-accepting fallback).
        let signal: ElohimContentWireSignal =
            serde_json::from_slice(&wire).expect("json string-hash envelope must decode");
        match signal {
            ElohimContentWireSignal::ContentCommitted {
                action_hash,
                content,
                ..
            } => {
                assert_eq!(action_hash.0, "uhCkk-anchor");
                assert_eq!(content.content_type, "attestation:humanness");
            }
        }
    }
}

// =============================================================================
// A.8 sweep tests
// =============================================================================

#[cfg(test)]
mod revocation_sweep_tests {
    use super::*;
    use diesel::connection::SimpleConnection;
    use diesel::prelude::*;
    use diesel::sqlite::SqliteConnection;

    /// Stand up a minimal in-memory SQLite DB with only the `epr_atoms` columns
    /// needed by the sweep. Mirrors the shape of the real migration without
    /// pulling in `embed_migrations!` (which requires the full schema).
    fn setup_epr_atoms_conn() -> SqliteConnection {
        let mut conn =
            SqliteConnection::establish(":memory:").expect("Failed to create in-memory SQLite");

        conn.batch_execute(
            r#"
            CREATE TABLE epr_atoms (
                cid                      TEXT NOT NULL PRIMARY KEY,
                kind                     TEXT NOT NULL,
                schema_ref               TEXT NOT NULL,
                schema_key               TEXT NOT NULL,
                reach                    TEXT NOT NULL,
                issued_at                TEXT NOT NULL,
                signer_cid               TEXT NOT NULL,
                supersedes               TEXT,
                canonical_bytes          BLOB NOT NULL,
                payload_bytes            BLOB NOT NULL,
                proof_bytes              BLOB NOT NULL,
                proof_algorithm          TEXT NOT NULL,
                verified_at              TEXT,
                verified_signer_fingerprint TEXT
            );
            CREATE INDEX idx_epr_atoms_signer_cid_issued_at ON epr_atoms(signer_cid, issued_at);
            "#,
        )
        .expect("Failed to create epr_atoms test table");

        conn
    }

    fn insert_test_atom(
        conn: &mut SqliteConnection,
        cid: &str,
        signer_cid: &str,
        issued_at: &str,
        verified_at: Option<&str>,
    ) {
        use crate::db::diesel_schema::epr_atoms;
        use diesel::prelude::*;

        diesel::insert_into(epr_atoms::table)
            .values((
                epr_atoms::cid.eq(cid),
                epr_atoms::kind.eq("test-kind"),
                epr_atoms::schema_ref.eq("test-schema-ref"),
                epr_atoms::schema_key.eq("test-schema-key"),
                epr_atoms::reach.eq("local"),
                epr_atoms::issued_at.eq(issued_at),
                epr_atoms::signer_cid.eq(signer_cid),
                epr_atoms::supersedes.eq::<Option<String>>(None),
                epr_atoms::canonical_bytes.eq(vec![0u8; 4]),
                epr_atoms::payload_bytes.eq(vec![0u8; 4]),
                epr_atoms::proof_bytes.eq(vec![0u8; 4]),
                epr_atoms::proof_algorithm.eq("ed25519"),
                epr_atoms::verified_at.eq(verified_at),
                epr_atoms::verified_signer_fingerprint.eq::<Option<String>>(None),
            ))
            .execute(conn)
            .expect("Failed to insert test atom");
    }

    fn fetch_verified_at(conn: &mut SqliteConnection, cid: &str) -> Option<String> {
        use crate::db::diesel_schema::epr_atoms;
        use diesel::prelude::*;

        epr_atoms::table
            .filter(epr_atoms::cid.eq(cid))
            .select(epr_atoms::verified_at)
            .first::<Option<String>>(conn)
            .expect("Failed to fetch atom")
    }

    /// Matching atom (same signer, issued after compromise) must have
    /// verified_at cleared. Atom from a different signer must be untouched.
    #[test]
    fn sweep_clears_verified_at_for_matching_atoms() {
        let mut conn = setup_epr_atoms_conn();

        insert_test_atom(
            &mut conn,
            "cid-A",
            "revoked-key-1",
            "2026-01-02T00:00:00Z",
            Some("2026-01-05T00:00:00Z"),
        );
        insert_test_atom(
            &mut conn,
            "cid-B",
            "other-key-99",
            "2026-01-02T00:00:00Z",
            Some("2026-01-05T00:00:00Z"),
        );

        let count = sweep_dependent_caches_on_revocation(
            &mut conn,
            "revoked-key-1",
            "2026-01-01T00:00:00Z",
        )
        .expect("sweep must not error");

        assert_eq!(count, 1, "exactly one row should have been cleared");
        assert!(
            fetch_verified_at(&mut conn, "cid-A").is_none(),
            "cid-A verified_at must be NULL after sweep"
        );
        assert!(
            fetch_verified_at(&mut conn, "cid-B").is_some(),
            "cid-B (different signer) must be unchanged"
        );
    }

    /// Atom whose issued_at is strictly before compromise_at must NOT be cleared.
    #[test]
    fn sweep_leaves_pre_compromise_atoms_alone() {
        let mut conn = setup_epr_atoms_conn();

        insert_test_atom(
            &mut conn,
            "cid-pre",
            "revoked-key-2",
            "2025-12-31T00:00:00Z",
            Some("2026-01-03T00:00:00Z"),
        );

        let count = sweep_dependent_caches_on_revocation(
            &mut conn,
            "revoked-key-2",
            "2026-01-01T00:00:00Z",
        )
        .expect("sweep must not error");

        assert_eq!(count, 0, "pre-compromise atom must not be updated");
        assert!(
            fetch_verified_at(&mut conn, "cid-pre").is_some(),
            "pre-compromise atom verified_at must remain set"
        );
    }

    /// Running the sweep twice must be a no-op on the second run.
    #[test]
    fn sweep_is_idempotent() {
        let mut conn = setup_epr_atoms_conn();

        insert_test_atom(
            &mut conn,
            "cid-idem",
            "revoked-key-3",
            "2026-02-15T00:00:00Z",
            Some("2026-02-20T00:00:00Z"),
        );

        let first = sweep_dependent_caches_on_revocation(
            &mut conn,
            "revoked-key-3",
            "2026-01-01T00:00:00Z",
        )
        .expect("first sweep must not error");
        assert_eq!(first, 1, "first run must clear the row");

        let second = sweep_dependent_caches_on_revocation(
            &mut conn,
            "revoked-key-3",
            "2026-01-01T00:00:00Z",
        )
        .expect("second sweep must not error");
        assert_eq!(second, 0, "second run must be a no-op (already NULL)");

        assert!(
            fetch_verified_at(&mut conn, "cid-idem").is_none(),
            "verified_at must still be NULL after second sweep"
        );
    }

    /// An empty epr_atoms table must return Ok(0).
    #[test]
    fn sweep_returns_zero_on_empty_table() {
        let mut conn = setup_epr_atoms_conn();

        let count =
            sweep_dependent_caches_on_revocation(&mut conn, "any-key", "2026-01-01T00:00:00Z")
                .expect("sweep on empty table must not error");

        assert_eq!(count, 0);
    }
}
