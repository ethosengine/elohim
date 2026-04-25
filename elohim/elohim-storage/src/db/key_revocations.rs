//! Recovery Protocol Phase 2 — M4 projection CRUD for key_revocations.
//!
//! Source of truth: DHT (imagodei KeyRevocation entries). These rows are
//! read-optimized projections populated via RecoveryV2Signal events emitted
//! by the imagodei coordinator zome.
//!
//! Insert pattern: upsert (INSERT OR REPLACE) on dht_anchor_hash primary key
//! so replayed signals are idempotent.

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use crate::db::diesel_schema::key_revocations;
use crate::db::models::{KeyRevocationRow, UpsertKeyRevocationRow};
use crate::error::StorageError;

// ============================================================================
// KeyRevocation projection
// ============================================================================

/// Upsert a key revocation projection row.
///
/// Idempotent: replaying the same signal produces the same row.
/// Conflict on `dht_anchor_hash` (primary key) replaces all mutable columns.
pub fn upsert_key_revocation(
    conn: &mut SqliteConnection,
    row: UpsertKeyRevocationRow,
) -> Result<(), StorageError> {
    diesel::insert_into(key_revocations::table)
        .values(&row)
        .on_conflict(key_revocations::dht_anchor_hash)
        .do_update()
        .set((
            key_revocations::id.eq(&row.id),
            key_revocations::human_id.eq(&row.human_id),
            key_revocations::revoked_key.eq(&row.revoked_key),
            key_revocations::reason.eq(&row.reason),
            key_revocations::trigger_type.eq(&row.trigger_type),
            key_revocations::initiated_by.eq(&row.initiated_by),
            key_revocations::required_votes.eq(&row.required_votes),
            key_revocations::current_votes.eq(&row.current_votes),
            key_revocations::threshold_reached.eq(&row.threshold_reached),
            key_revocations::effective_at.eq(&row.effective_at),
            key_revocations::created_at.eq(&row.created_at),
            key_revocations::updated_at.eq(&row.updated_at),
        ))
        .execute(conn)
        .map(|_| ())
        .map_err(|e| StorageError::Internal(format!("Failed to upsert key_revocation: {e}")))
}

/// Mark a revocation as effective and set effective_at.
///
/// Called by KeyRevocationEffective handler after the projection row exists.
/// Idempotent: calling with the same values is safe.
pub fn set_key_revocation_effective(
    conn: &mut SqliteConnection,
    target_dht_anchor_hash: &str,
    new_effective_at: &str,
    new_updated_at: &str,
) -> Result<(), StorageError> {
    let rows = diesel::update(
        key_revocations::table.filter(key_revocations::dht_anchor_hash.eq(target_dht_anchor_hash)),
    )
    .set((
        key_revocations::threshold_reached.eq(1),
        key_revocations::effective_at.eq(Some(new_effective_at.to_string())),
        key_revocations::updated_at.eq(new_updated_at.to_string()),
    ))
    .execute(conn)
    .map_err(|e| StorageError::Internal(format!("Failed to set key_revocation effective: {e}")))?;

    if rows == 0 {
        // Effective gossip arrived before we had projected the Requested signal
        // (DHT partition recovery, fresh boot, replay-out-of-order). M5 elohim-defender
        // gates need to know about this peer's revocation; surface the gap loudly.
        tracing::warn!(
            target: "imagodei.revocation_observed",
            dht_anchor_hash = %target_dht_anchor_hash,
            "set_key_revocation_effective: no projection row exists; revocation may be invisible to local gates until requested signal replays"
        );
    }
    Ok(())
}

/// Update the denormalized current_votes count on a key_revocation row.
///
/// Called by RevocationVoteSubmitted handler after inserting a vote row.
pub fn update_current_votes(
    conn: &mut SqliteConnection,
    target_dht_anchor_hash: &str,
    new_current_votes: i32,
    new_updated_at: &str,
) -> Result<(), StorageError> {
    diesel::update(
        key_revocations::table.filter(key_revocations::dht_anchor_hash.eq(target_dht_anchor_hash)),
    )
    .set((
        key_revocations::current_votes.eq(new_current_votes),
        key_revocations::updated_at.eq(new_updated_at.to_string()),
    ))
    .execute(conn)
    .map(|_| ())
    .map_err(|e| {
        StorageError::Internal(format!(
            "Failed to update current_votes on key_revocation: {e}"
        ))
    })
}

/// Fetch a key revocation by its DHT anchor hash.
pub fn get_key_revocation_by_anchor(
    conn: &mut SqliteConnection,
    dht_anchor_hash: &str,
) -> Result<Option<KeyRevocationRow>, StorageError> {
    key_revocations::table
        .filter(key_revocations::dht_anchor_hash.eq(dht_anchor_hash))
        .first::<KeyRevocationRow>(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Failed to fetch key_revocation: {e}")))
}

/// Fetch a key revocation by its coordinator-assigned id.
pub fn get_key_revocation_by_id(
    conn: &mut SqliteConnection,
    revocation_id: &str,
) -> Result<Option<KeyRevocationRow>, StorageError> {
    key_revocations::table
        .filter(key_revocations::id.eq(revocation_id))
        .first::<KeyRevocationRow>(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Failed to fetch key_revocation by id: {e}")))
}

/// List all key revocations for a human (most recent first).
pub fn list_revocations_for_human(
    conn: &mut SqliteConnection,
    human_id: &str,
) -> Result<Vec<KeyRevocationRow>, StorageError> {
    key_revocations::table
        .filter(key_revocations::human_id.eq(human_id))
        .order(key_revocations::created_at.desc())
        .select(KeyRevocationRow::as_select())
        .load(conn)
        .map_err(|e| {
            StorageError::Internal(format!("Failed to list key_revocations for human: {e}"))
        })
}

/// List all pending key revocations (threshold_reached = 0).
pub fn list_pending_revocations(
    conn: &mut SqliteConnection,
) -> Result<Vec<KeyRevocationRow>, StorageError> {
    key_revocations::table
        .filter(key_revocations::threshold_reached.eq(0))
        .order(key_revocations::created_at.desc())
        .select(KeyRevocationRow::as_select())
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to list pending key_revocations: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use diesel::connection::SimpleConnection;

    fn test_conn() -> SqliteConnection {
        let mut conn = SqliteConnection::establish(":memory:").unwrap();
        conn.batch_execute(include_str!(
            "../../migrations/2026-04-24-010000_key_revocations/up.sql"
        ))
        .unwrap();
        conn
    }

    fn sample_row(dht_hash: &str, threshold: i32) -> UpsertKeyRevocationRow {
        UpsertKeyRevocationRow {
            dht_anchor_hash: dht_hash.into(),
            id: format!("rev-{dht_hash}"),
            human_id: "human-matthew".into(),
            revoked_key: "uhCAkTEST".into(),
            reason: "compromised".into(),
            trigger_type: "voluntary".into(),
            initiated_by: "human-matthew".into(),
            required_votes: 1,
            current_votes: threshold,
            threshold_reached: threshold,
            effective_at: if threshold == 1 {
                Some("2026-04-24T00:00:00Z".into())
            } else {
                None
            },
            created_at: "2026-04-24T00:00:00Z".into(),
            updated_at: "2026-04-24T00:00:00Z".into(),
        }
    }

    #[test]
    fn upsert_is_idempotent() {
        let mut conn = test_conn();
        let row = sample_row("R1", 1);
        upsert_key_revocation(&mut conn, row.clone()).unwrap();
        upsert_key_revocation(&mut conn, row).unwrap();
        let fetched = get_key_revocation_by_anchor(&mut conn, "R1").unwrap();
        assert!(fetched.is_some());
    }

    #[test]
    fn set_effective_marks_threshold_reached() {
        let mut conn = test_conn();
        upsert_key_revocation(&mut conn, sample_row("R2", 0)).unwrap();
        set_key_revocation_effective(
            &mut conn,
            "R2",
            "2026-04-24T01:00:00Z",
            "2026-04-24T01:00:00Z",
        )
        .unwrap();
        let row = get_key_revocation_by_anchor(&mut conn, "R2")
            .unwrap()
            .unwrap();
        assert_eq!(row.threshold_reached, 1);
        assert_eq!(row.effective_at.as_deref(), Some("2026-04-24T01:00:00Z"));
    }

    #[test]
    fn update_current_votes_increments() {
        let mut conn = test_conn();
        upsert_key_revocation(&mut conn, sample_row("R3", 0)).unwrap();
        update_current_votes(&mut conn, "R3", 2, "2026-04-24T01:00:00Z").unwrap();
        let row = get_key_revocation_by_anchor(&mut conn, "R3")
            .unwrap()
            .unwrap();
        assert_eq!(row.current_votes, 2);
    }

    #[test]
    fn list_pending_excludes_effective() {
        let mut conn = test_conn();
        upsert_key_revocation(&mut conn, sample_row("R4", 0)).unwrap(); // pending
        upsert_key_revocation(&mut conn, sample_row("R5", 1)).unwrap(); // effective
        let pending = list_pending_revocations(&mut conn).unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].dht_anchor_hash, "R4");
    }
}
