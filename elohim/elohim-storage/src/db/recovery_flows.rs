//! Recovery Protocol Phase 2 — M4 projection CRUD for `recovery_flows`.
//!
//! Source of truth: DHT (elohim DNA Content entries with content_type
//! `governance-action:recovery-request` or `governance-action:identity-freeze`).
//! These rows are read-optimized state-machine projections populated via the
//! `RecoveryFlowProjector` (Task 8/9) consuming ElohimContentSignal events.
//!
//! Insert pattern: upsert (INSERT OR REPLACE) on `id` primary key so replayed
//! signals are idempotent. `dht_anchor_hash` carries provenance back to the
//! canonical DHT entry as a `BLOB` (Vec<u8>).
//!
//! State machine (column `state`): `Open` → `Quorum` → `Effective` → `Closed`.

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use crate::db::diesel_schema::recovery_flows;
use crate::db::models::RecoveryFlowRow;
use crate::error::StorageError;

// ============================================================================
// RecoveryFlow projection
// ============================================================================

/// Upsert a recovery flow projection row.
///
/// Idempotent: replaying the same signal produces the same row.
/// Conflict on `id` (primary key) replaces all mutable columns.
pub fn upsert(conn: &mut SqliteConnection, row: &RecoveryFlowRow) -> Result<(), StorageError> {
    diesel::insert_into(recovery_flows::table)
        .values(row)
        .on_conflict(recovery_flows::id)
        .do_update()
        .set(row)
        .execute(conn)
        .map(|_| ())
        .map_err(|e| StorageError::Internal(format!("Failed to upsert recovery_flow: {e}")))
}

/// Fetch a recovery flow by its coordinator-assigned id (primary key).
pub fn get_by_id(
    conn: &mut SqliteConnection,
    flow_id: &str,
) -> Result<Option<RecoveryFlowRow>, StorageError> {
    recovery_flows::table
        .filter(recovery_flows::id.eq(flow_id))
        .select(RecoveryFlowRow::as_select())
        .first::<RecoveryFlowRow>(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Failed to fetch recovery_flow by id: {e}")))
}

/// List recovery flows by state-machine state (e.g. "Open", "Quorum", "Effective", "Closed").
///
/// Most recent first by `created_at`.
pub fn list_by_state(
    conn: &mut SqliteConnection,
    state: &str,
) -> Result<Vec<RecoveryFlowRow>, StorageError> {
    recovery_flows::table
        .filter(recovery_flows::state.eq(state))
        .order(recovery_flows::created_at.desc())
        .select(RecoveryFlowRow::as_select())
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to list recovery_flows by state: {e}")))
}

/// List all recovery flows for a subject human (most recent first).
pub fn list_for_subject(
    conn: &mut SqliteConnection,
    subject_human_id: &str,
) -> Result<Vec<RecoveryFlowRow>, StorageError> {
    recovery_flows::table
        .filter(recovery_flows::subject_human_id.eq(subject_human_id))
        .order(recovery_flows::created_at.desc())
        .select(RecoveryFlowRow::as_select())
        .load(conn)
        .map_err(|e| {
            StorageError::Internal(format!("Failed to list recovery_flows for subject: {e}"))
        })
}

/// Transition a recovery flow to a new state. Updates `state`, optional
/// `effective_at`, `threshold_reached`, and `updated_at`.
///
/// Idempotent: callers that observe the same projection event multiple times
/// will land on the same row state.
pub fn transition_state(
    conn: &mut SqliteConnection,
    target_id: &str,
    new_state: &str,
    new_threshold_reached: i32,
    new_effective_at: Option<&str>,
    new_updated_at: &str,
) -> Result<(), StorageError> {
    let rows = diesel::update(recovery_flows::table.filter(recovery_flows::id.eq(target_id)))
        .set((
            recovery_flows::state.eq(new_state.to_string()),
            recovery_flows::threshold_reached.eq(new_threshold_reached),
            recovery_flows::effective_at.eq(new_effective_at.map(|s| s.to_string())),
            recovery_flows::updated_at.eq(new_updated_at.to_string()),
        ))
        .execute(conn)
        .map_err(|e| {
            StorageError::Internal(format!("Failed to transition recovery_flow state: {e}"))
        })?;

    if rows == 0 {
        tracing::warn!(
            target: "recovery.flow_observed",
            flow_id = %target_id,
            new_state = %new_state,
            "transition_state: no projection row exists; state change may be invisible until parent signal replays"
        );
    }
    Ok(())
}

/// Update the denormalized `current_votes` count on a recovery_flow row.
///
/// Called by the projector after observing an attestation child of the
/// recovery_flow parent.
pub fn update_current_votes(
    conn: &mut SqliteConnection,
    target_id: &str,
    new_current_votes: i32,
    new_updated_at: &str,
) -> Result<(), StorageError> {
    diesel::update(recovery_flows::table.filter(recovery_flows::id.eq(target_id)))
        .set((
            recovery_flows::current_votes.eq(new_current_votes),
            recovery_flows::updated_at.eq(new_updated_at.to_string()),
        ))
        .execute(conn)
        .map(|_| ())
        .map_err(|e| {
            StorageError::Internal(format!(
                "Failed to update current_votes on recovery_flow: {e}"
            ))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

    const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

    fn test_conn() -> SqliteConnection {
        let mut conn = SqliteConnection::establish(":memory:").expect("in-memory DB");
        conn.run_pending_migrations(MIGRATIONS).expect("migrations");
        conn
    }

    fn sample_row(id: &str, state: &str) -> RecoveryFlowRow {
        RecoveryFlowRow {
            id: id.to_string(),
            dht_anchor_hash: format!("anchor-{id}").into_bytes(),
            flow_kind: "recovery-request".into(),
            subject_human_id: "human-matthew".into(),
            initiated_by_cid: "human-jessica".into(),
            state: state.to_string(),
            required_votes: 3,
            current_votes: 0,
            threshold_reached: 0,
            effective_at: None,
            closes_at: "2099-01-01T00:00:00Z".into(),
            metadata_json: "{}".into(),
            created_at: "2026-05-15T00:00:00Z".into(),
            updated_at: "2026-05-15T00:00:00Z".into(),
        }
    }

    #[test]
    fn upsert_is_idempotent() {
        let mut conn = test_conn();
        let row = sample_row("flow-F1", "Open");
        upsert(&mut conn, &row).unwrap();
        upsert(&mut conn, &row).unwrap();
        let fetched = get_by_id(&mut conn, "flow-F1").unwrap().unwrap();
        assert_eq!(fetched.state, "Open");
        assert_eq!(fetched.flow_kind, "recovery-request");
    }

    #[test]
    fn list_by_state_filters_correctly() {
        let mut conn = test_conn();
        upsert(&mut conn, &sample_row("flow-F2", "Open")).unwrap();
        upsert(&mut conn, &sample_row("flow-F3", "Effective")).unwrap();
        upsert(&mut conn, &sample_row("flow-F4", "Open")).unwrap();
        let opens = list_by_state(&mut conn, "Open").unwrap();
        assert_eq!(opens.len(), 2);
    }

    #[test]
    fn transition_state_updates_row() {
        let mut conn = test_conn();
        upsert(&mut conn, &sample_row("flow-F5", "Open")).unwrap();
        transition_state(
            &mut conn,
            "flow-F5",
            "Effective",
            1,
            Some("2026-05-15T01:00:00Z"),
            "2026-05-15T01:00:00Z",
        )
        .unwrap();
        let row = get_by_id(&mut conn, "flow-F5").unwrap().unwrap();
        assert_eq!(row.state, "Effective");
        assert_eq!(row.threshold_reached, 1);
        assert_eq!(row.effective_at.as_deref(), Some("2026-05-15T01:00:00Z"));
    }

    #[test]
    fn update_current_votes_increments() {
        let mut conn = test_conn();
        upsert(&mut conn, &sample_row("flow-F6", "Open")).unwrap();
        update_current_votes(&mut conn, "flow-F6", 2, "2026-05-15T01:00:00Z").unwrap();
        let row = get_by_id(&mut conn, "flow-F6").unwrap().unwrap();
        assert_eq!(row.current_votes, 2);
    }

    #[test]
    fn list_for_subject_filters_correctly() {
        let mut conn = test_conn();
        upsert(&mut conn, &sample_row("flow-F7", "Open")).unwrap();
        let mut other = sample_row("flow-F8", "Open");
        other.subject_human_id = "human-jessica".into();
        upsert(&mut conn, &other).unwrap();
        let matthews = list_for_subject(&mut conn, "human-matthew").unwrap();
        assert_eq!(matthews.len(), 1);
        assert_eq!(matthews[0].id, "flow-F7");
    }
}
