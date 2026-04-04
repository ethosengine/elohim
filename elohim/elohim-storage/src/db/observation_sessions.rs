//! Observation session CRUD operations using Diesel
//!
//! Diagnostic observation sessions are Category C (operational): ephemeral per-node,
//! not shared via DHT. Sessions collect structured log entries from multiple origins
//! during a bounded TTL window and are closed with a report content ID.

use diesel::prelude::*;
use uuid::Uuid;

use super::diesel_schema::{observation_entries, observation_sessions};
use super::models::{NewObservationEntry, NewObservationSession, ObservationEntry, ObservationSession};
use crate::error::StorageError;

// ============================================================================
// Session Lifecycle
// ============================================================================

/// Begin a new observation session with a generated UUID.
/// Returns the created session row.
pub fn begin_session(
    conn: &mut SqliteConnection,
    source: &str,
    ttl_seconds: i32,
    metadata_json: Option<&str>,
) -> Result<ObservationSession, StorageError> {
    let id = Uuid::new_v4().to_string();

    let new_session = NewObservationSession {
        id: &id,
        ttl_seconds,
        source,
        metadata_json,
    };

    diesel::insert_into(observation_sessions::table)
        .values(&new_session)
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Insert observation session failed: {}", e)))?;

    get_session(conn, &id)?
        .ok_or_else(|| StorageError::Internal("Failed to retrieve created observation session".into()))
}

/// Get an observation session by ID. Returns None if not found.
pub fn get_session(
    conn: &mut SqliteConnection,
    session_id: &str,
) -> Result<Option<ObservationSession>, StorageError> {
    observation_sessions::table
        .filter(observation_sessions::id.eq(session_id))
        .first(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Query observation session failed: {}", e)))
}

/// Returns true if the session exists and has not been ended.
pub fn is_session_active(
    conn: &mut SqliteConnection,
    session_id: &str,
) -> Result<bool, StorageError> {
    let session = get_session(conn, session_id)?;
    Ok(session.map(|s| s.ended_at.is_none()).unwrap_or(false))
}

/// Close a session by setting ended_at to now and storing the report content ID.
pub fn close_session(
    conn: &mut SqliteConnection,
    session_id: &str,
    report_content_id: Option<&str>,
) -> Result<(), StorageError> {
    let ended_at = chrono::Utc::now().to_rfc3339();

    diesel::update(observation_sessions::table.filter(observation_sessions::id.eq(session_id)))
        .set((
            observation_sessions::ended_at.eq(Some(&ended_at)),
            observation_sessions::report_content_id.eq(report_content_id),
        ))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Close observation session failed: {}", e)))?;

    Ok(())
}

// ============================================================================
// Entry Operations
// ============================================================================

/// Append a single entry to an observation session.
#[allow(clippy::too_many_arguments)]
pub fn append_entry(
    conn: &mut SqliteConnection,
    session_id: &str,
    origin: &str,
    category: &str,
    severity: &str,
    method: Option<&str>,
    path: Option<&str>,
    status_code: Option<i32>,
    message: &str,
    context_json: Option<&str>,
) -> Result<(), StorageError> {
    let timestamp = chrono::Utc::now().to_rfc3339();

    let new_entry = NewObservationEntry {
        session_id,
        origin,
        category,
        severity,
        method,
        path,
        status_code,
        message,
        context_json,
    };

    // We need to set timestamp separately since it has a DB default but we want explicit control
    diesel::insert_into(observation_entries::table)
        .values((
            &new_entry,
            observation_entries::timestamp.eq(&timestamp),
        ))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Insert observation entry failed: {}", e)))?;

    Ok(())
}

/// Get all entries for a session, ordered by timestamp ascending.
pub fn get_entries(
    conn: &mut SqliteConnection,
    session_id: &str,
) -> Result<Vec<ObservationEntry>, StorageError> {
    observation_entries::table
        .filter(observation_entries::session_id.eq(session_id))
        .order(observation_entries::timestamp.asc())
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query observation entries failed: {}", e)))
}

/// Delete all entries for a session. Returns the count of deleted rows.
pub fn purge_entries(
    conn: &mut SqliteConnection,
    session_id: &str,
) -> Result<usize, StorageError> {
    diesel::delete(
        observation_entries::table.filter(observation_entries::session_id.eq(session_id)),
    )
    .execute(conn)
    .map_err(|e| StorageError::Internal(format!("Purge observation entries failed: {}", e)))
}
