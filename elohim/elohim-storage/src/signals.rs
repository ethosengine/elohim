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

/// Storage-side mirror of the DNA `InfrastructureSignal` enum.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum InfrastructureSignal {
    /// PeerStatus DHT entry was recorded — project into SQLite.
    PeerStatusRecorded {
        peer_id: String,
        status: String,
        general_pool_member: bool,
        accepting_stewardship_reserves: bool,
        archetype_class: Option<String>,
        timestamp: i64,
        action_hash: String,
    },
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
                peer_id,
                status,
                general_pool_member: general_pool_member as i32,
                accepting_stewardship_reserves: accepting_stewardship_reserves as i32,
                archetype_class,
                timestamp,
                dht_anchor_hash: action_hash,
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
// Mirrors the DNA-side `MishpatSignal` enum (mishpat DNA, gate_decision zome).
// Fields that are `AgentPubKey` or `ActionHash` on the DNA side serialize as
// base64 `String`s over the wire — no integrity-crate dependency needed here.
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
    GateDecisionCreated {
        action_hash: String,
        entry_hash: String,
        author: String,
        entry: GateDecisionAttestationEntry,
    },
    /// GateDecisionChallenge DHT entry was recorded — project into SQLite.
    GateDecisionChallengeCreated {
        action_hash: String,
        entry_hash: String,
        author: String,
        entry: GateDecisionChallengeEntry,
    },
    /// ChallengeOutcome DHT entry was recorded — project into SQLite.
    ChallengeOutcomeCreated {
        action_hash: String,
        entry_hash: String,
        author: String,
        entry: ChallengeOutcomeEntry,
    },
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
            let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
            let row = crate::db::gate_decision_attestations::GateDecisionAttestationRow {
                app_id: app_id.to_string(),
                decision_id: entry.decision_id,
                phase: entry.phase,
                elohim_id: entry.elohim_id,
                elohim_substance_cid: entry.elohim_substance_cid,
                gate_name: entry.gate_name,
                gate_process_cid: entry.gate_process_cid,
                request_ref_json: entry.request_ref_json,
                decision: entry.decision,
                reasoning_json: entry.reasoning_json,
                context_summary_cid: entry.context_summary_cid,
                decided_at: entry.decided_at,
                universal_band_cid: entry.universal_band_cid,
                dht_anchor_hash: action_hash,
                created_at: now.clone(),
                updated_at: now,
            };
            crate::db::gate_decision_attestations::upsert(conn, &row)?;
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
                dht_anchor_hash: action_hash,
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
                dht_anchor_hash: action_hash,
                created_at: now.clone(),
                updated_at: now,
            };
            crate::db::challenge_outcomes::upsert(conn, &row)?;
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
    KeyRotationCommitted {
        action_hash: String,
        rotation: KeyRotationPayload,
    },
}

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
fn timestamp_to_iso(v: &serde_json::Value) -> String {
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

/// Dispatch a `RecoveryV2Signal` into the SQLite projection.
///
/// New variants add a match arm here. Keep this function thin — per-entity
/// projection logic belongs in `db::recovery_requests`.
pub fn handle_recovery_v2_signal(
    conn: &mut diesel::sqlite::SqliteConnection,
    signal: RecoveryV2Signal,
) -> Result<(), StorageError> {
    match signal {
        RecoveryV2Signal::RecoveryRequestCreated { action_hash, request } => {
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
        RecoveryV2Signal::KeyRotationCommitted { action_hash, rotation } => {
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
    }
}

#[cfg(test)]
mod mishpat_signal_tests {
    use super::*;
    use diesel::connection::SimpleConnection;
    use diesel::prelude::*;
    use diesel::sqlite::SqliteConnection;

    fn setup_test_conn() -> SqliteConnection {
        let mut conn =
            SqliteConnection::establish(":memory:").expect("Failed to create in-memory SQLite");

        conn.batch_execute(
            r#"
            CREATE TABLE gate_decision_attestations (
                app_id TEXT NOT NULL,
                decision_id TEXT NOT NULL,
                phase TEXT NOT NULL,
                elohim_id TEXT NOT NULL,
                elohim_substance_cid TEXT NOT NULL,
                gate_name TEXT NOT NULL,
                gate_process_cid TEXT NOT NULL,
                request_ref_json TEXT NOT NULL,
                decision TEXT NOT NULL,
                reasoning_json TEXT NOT NULL,
                context_summary_cid TEXT NOT NULL,
                decided_at TEXT NOT NULL,
                universal_band_cid TEXT NOT NULL,
                dht_anchor_hash TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                PRIMARY KEY (app_id, decision_id)
            );
            "#,
        )
        .expect("Failed to create test table");

        conn
    }

    fn make_signal(decision_id: &str, decision: &str, phase: &str) -> MishpatSignal {
        MishpatSignal::GateDecisionCreated {
            action_hash: "uhCkkDEFABC".to_string(),
            entry_hash: "uhCEkDEFABC".to_string(),
            author: "uhCAkABCDEF".to_string(),
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

    #[test]
    fn gate_decision_created_projects_row() {
        let mut conn = setup_test_conn();
        let signal = make_signal("bafyDec001", "allow", "elohim-active");
        handle_mishpat_signal(&mut conn, "test-app", signal).unwrap();

        let row =
            crate::db::gate_decision_attestations::find_by_id(&mut conn, "test-app", "bafyDec001")
                .unwrap()
                .expect("Row must be present after signal");

        assert_eq!(row.decision, "allow");
        assert_eq!(row.phase, "elohim-active");
        assert_eq!(row.dht_anchor_hash, "uhCkkDEFABC");
        assert_eq!(row.gate_name, "discernment-gate-v1-mechanical");
    }

    #[test]
    fn gate_decision_created_is_idempotent() {
        let mut conn = setup_test_conn();
        let signal = make_signal("bafyDec002", "decline", "dev-context");
        handle_mishpat_signal(&mut conn, "test-app", signal.clone()).unwrap();
        handle_mishpat_signal(&mut conn, "test-app", signal).unwrap();

        let rows = crate::db::gate_decision_attestations::find_by_phase(
            &mut conn,
            "test-app",
            "dev-context",
        )
        .unwrap();
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
                assert_eq!(action_hash, "uhCkkABC");
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
            action_hash: "uhCkkCHALABC".to_string(),
            entry_hash: "uhCEkCHALABC".to_string(),
            author: "uhCAkCHALLENGER".to_string(),
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
                assert_eq!(action_hash, "uhCkkCHAL");
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
            action_hash: "uhCkkOUTABC".to_string(),
            entry_hash: "uhCEkOUTABC".to_string(),
            author: "uhCAkREVIEWER".to_string(),
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
                assert_eq!(action_hash, "uhCkkOUT");
                assert_eq!(entry.verdict, "upheld");
                assert_eq!(entry.challenge_cid, "bafyChal");
            }
            _ => panic!("Expected ChallengeOutcomeCreated variant"),
        }
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
                assert_eq!(peer_id, "uhCAkABC");
                assert_eq!(status, "online");
            }
        }
    }
}
