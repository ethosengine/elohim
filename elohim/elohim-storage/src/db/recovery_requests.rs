//! Recovery Protocol Phase 2 — projection CRUD for recovery_requests and key_rotations.
//!
//! Source of truth: DHT (imagodei DNA). These tables are read-optimized projections
//! populated via RecoveryV2Signal events emitted by the imagodei coordinator zome.
//!
//! Insert pattern: upsert (INSERT OR REPLACE) on dht_anchor_hash primary key so
//! replayed signals are idempotent.

use diesel::prelude::*;

use super::diesel_schema::{key_rotations, recovery_requests};
use super::models::{KeyRotationRow, NewKeyRotationRow, NewRecoveryRequestRow, RecoveryRequestRow};
use crate::error::StorageError;

// ============================================================================
// RecoveryRequest projection
// ============================================================================

/// Upsert a recovery request projection row.
///
/// Idempotent: replaying the same signal produces the same row.
/// Conflict on `dht_anchor_hash` (primary key) replaces all columns.
pub fn upsert_recovery_request(
    conn: &mut SqliteConnection,
    row: NewRecoveryRequestRow,
) -> Result<(), StorageError> {
    diesel::insert_into(recovery_requests::table)
        .values(&row)
        .on_conflict(recovery_requests::dht_anchor_hash)
        .do_update()
        .set((
            recovery_requests::human_agent_pubkey.eq(&row.human_agent_pubkey),
            recovery_requests::new_agent_pubkey.eq(&row.new_agent_pubkey),
            recovery_requests::hosting_doorway_pubkey.eq(&row.hosting_doorway_pubkey),
            recovery_requests::proposed_authority_kind.eq(&row.proposed_authority_kind),
            recovery_requests::proposed_authority_json.eq(&row.proposed_authority_json),
            recovery_requests::request_nonce.eq(&row.request_nonce),
            recovery_requests::human_id.eq(&row.human_id),
            recovery_requests::required_witness_count.eq(&row.required_witness_count),
            recovery_requests::created_at.eq(&row.created_at),
        ))
        .execute(conn)
        .map(|_| ())
        .map_err(|e| StorageError::Internal(format!("Failed to upsert recovery_request: {e}")))
}

/// Fetch a recovery request by its DHT anchor hash.
pub fn get_recovery_request(
    conn: &mut SqliteConnection,
    dht_anchor_hash: &str,
) -> Result<Option<RecoveryRequestRow>, StorageError> {
    recovery_requests::table
        .filter(recovery_requests::dht_anchor_hash.eq(dht_anchor_hash))
        .first::<RecoveryRequestRow>(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Failed to fetch recovery_request: {e}")))
}

/// List recovery requests for a given human agent pubkey.
pub fn list_recovery_requests_for_agent(
    conn: &mut SqliteConnection,
    human_agent_pubkey: &str,
) -> Result<Vec<RecoveryRequestRow>, StorageError> {
    recovery_requests::table
        .filter(recovery_requests::human_agent_pubkey.eq(human_agent_pubkey))
        .order(recovery_requests::created_at.desc())
        .load::<RecoveryRequestRow>(conn)
        .map_err(|e| {
            StorageError::Internal(format!("Failed to list recovery_requests for agent: {e}"))
        })
}

// ============================================================================
// KeyRotation projection
// ============================================================================

/// Upsert a key rotation projection row.
///
/// Idempotent: replaying the same signal produces the same row.
/// Conflict on `dht_anchor_hash` (primary key) replaces all columns.
pub fn upsert_key_rotation(
    conn: &mut SqliteConnection,
    row: NewKeyRotationRow,
) -> Result<(), StorageError> {
    diesel::insert_into(key_rotations::table)
        .values(&row)
        .on_conflict(key_rotations::dht_anchor_hash)
        .do_update()
        .set((
            key_rotations::human_agent_pubkey.eq(&row.human_agent_pubkey),
            key_rotations::new_agent_pubkey.eq(&row.new_agent_pubkey),
            key_rotations::superseded_agent_pubkey.eq(&row.superseded_agent_pubkey),
            key_rotations::recovery_request_hash.eq(&row.recovery_request_hash),
            key_rotations::authority_kind.eq(&row.authority_kind),
            key_rotations::authority_json.eq(&row.authority_json),
            key_rotations::rotated_at.eq(&row.rotated_at),
        ))
        .execute(conn)
        .map(|_| ())
        .map_err(|e| StorageError::Internal(format!("Failed to upsert key_rotation: {e}")))
}

/// Fetch a key rotation by its DHT anchor hash.
pub fn get_key_rotation(
    conn: &mut SqliteConnection,
    dht_anchor_hash: &str,
) -> Result<Option<KeyRotationRow>, StorageError> {
    key_rotations::table
        .filter(key_rotations::dht_anchor_hash.eq(dht_anchor_hash))
        .first::<KeyRotationRow>(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Failed to fetch key_rotation: {e}")))
}

/// List key rotations for a given human agent pubkey (most recent first).
pub fn list_key_rotations_for_agent(
    conn: &mut SqliteConnection,
    human_agent_pubkey: &str,
) -> Result<Vec<KeyRotationRow>, StorageError> {
    key_rotations::table
        .filter(key_rotations::human_agent_pubkey.eq(human_agent_pubkey))
        .order(key_rotations::rotated_at.desc())
        .load::<KeyRotationRow>(conn)
        .map_err(|e| {
            StorageError::Internal(format!("Failed to list key_rotations for agent: {e}"))
        })
}
