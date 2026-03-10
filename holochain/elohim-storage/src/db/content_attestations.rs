//! Content attestation CRUD operations using Diesel

use diesel::prelude::*;

use super::diesel_schema::content_attestations;
use super::models::{current_timestamp, ContentAttestation, NewContentAttestation};
use crate::error::StorageError;

/// Insert a new content attestation
pub fn create_attestation(
    conn: &mut SqliteConnection,
    new: &NewContentAttestation,
) -> Result<ContentAttestation, StorageError> {
    diesel::insert_into(content_attestations::table)
        .values(new)
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Insert failed: {}", e)))?;

    content_attestations::table
        .filter(content_attestations::id.eq(new.id))
        .first(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Revoke an attestation by setting is_revoked = 1
pub fn revoke_attestation(
    conn: &mut SqliteConnection,
    id: &str,
) -> Result<ContentAttestation, StorageError> {
    let now = current_timestamp();
    diesel::update(content_attestations::table.filter(content_attestations::id.eq(id)))
        .set((
            content_attestations::is_revoked.eq(1),
            content_attestations::updated_at.eq(&now),
        ))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Update failed: {}", e)))?;

    content_attestations::table
        .filter(content_attestations::id.eq(id))
        .first(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Get attestation by ID
pub fn get_attestation(
    conn: &mut SqliteConnection,
    id: &str,
) -> Result<Option<ContentAttestation>, StorageError> {
    content_attestations::table
        .filter(content_attestations::id.eq(id))
        .first(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Query attestations for a specific content item
pub fn query_attestations_for_content(
    conn: &mut SqliteConnection,
    content_id: &str,
) -> Result<Vec<ContentAttestation>, StorageError> {
    content_attestations::table
        .filter(content_attestations::content_id.eq(content_id))
        .order(content_attestations::created_at.desc())
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Query attestations by attestor presence ID
pub fn query_attestations_by_attestor(
    conn: &mut SqliteConnection,
    attestor_id: &str,
) -> Result<Vec<ContentAttestation>, StorageError> {
    content_attestations::table
        .filter(content_attestations::attestor_presence_id.eq(attestor_id))
        .order(content_attestations::created_at.desc())
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Query attestations for multiple content IDs
pub fn query_attestations_by_content_ids(
    conn: &mut SqliteConnection,
    content_ids: &[String],
) -> Result<Vec<ContentAttestation>, StorageError> {
    content_attestations::table
        .filter(content_attestations::content_id.eq_any(content_ids))
        .order(content_attestations::created_at.desc())
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}
