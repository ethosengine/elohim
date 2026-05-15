//! Recovery Protocol Phase 2 — M4 projection CRUD for key_revocations.
//!
//! Source of truth: DHT (elohim DNA Content entries with content_type
//! `governance-action:key-revocation` + `attestation:revocation-vote`
//! children). These rows are read-optimized projections populated via the
//! ElohimContentSignal dispatcher (T10/T12 wiring) for cross-DNA Content.
//!
//! Insert pattern: upsert (INSERT OR REPLACE) on `id` primary key so
//! replayed signals are idempotent. `dht_anchor_hash` carries provenance
//! back to the canonical DHT entry as a `BLOB` (Vec<u8>).

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use crate::db::diesel_schema::key_revocations;
use crate::db::models::KeyRevocationRow;
use crate::error::StorageError;

// ============================================================================
// KeyRevocation projection
// ============================================================================

/// Upsert a key revocation projection row.
///
/// Idempotent: replaying the same signal produces the same row.
/// Conflict on `id` (primary key) replaces all mutable columns.
pub fn upsert(conn: &mut SqliteConnection, row: &KeyRevocationRow) -> Result<(), StorageError> {
    diesel::insert_into(key_revocations::table)
        .values(row)
        .on_conflict(key_revocations::id)
        .do_update()
        .set(row)
        .execute(conn)
        .map(|_| ())
        .map_err(|e| StorageError::Internal(format!("Failed to upsert key_revocation: {e}")))
}

/// Mark a revocation as effective and set effective_at.
///
/// Called by the projector after the row exists and the threshold has been
/// observed. Idempotent: calling with the same values is safe.
pub fn set_effective(
    conn: &mut SqliteConnection,
    target_id: &str,
    new_effective_at: &str,
    new_updated_at: &str,
) -> Result<(), StorageError> {
    let rows = diesel::update(key_revocations::table.filter(key_revocations::id.eq(target_id)))
        .set((
            key_revocations::threshold_reached.eq(1),
            key_revocations::effective_at.eq(Some(new_effective_at.to_string())),
            key_revocations::updated_at.eq(new_updated_at.to_string()),
        ))
        .execute(conn)
        .map_err(|e| {
            StorageError::Internal(format!("Failed to set key_revocation effective: {e}"))
        })?;

    if rows == 0 {
        // Effective gossip arrived before we had projected the Requested signal
        // (DHT partition recovery, fresh boot, replay-out-of-order). M5 elohim-defender
        // gates need to know about this peer's revocation; surface the gap loudly.
        tracing::warn!(
            target: "imagodei.revocation_observed",
            revocation_id = %target_id,
            "set_effective: no projection row exists; revocation may be invisible to local gates until requested signal replays"
        );
    }
    Ok(())
}

/// Update the denormalized current_votes count on a key_revocation row.
///
/// Called by the projector after observing a `attestation:revocation-vote`
/// child of the revocation parent.
pub fn update_current_votes(
    conn: &mut SqliteConnection,
    target_id: &str,
    new_current_votes: i32,
    new_updated_at: &str,
) -> Result<(), StorageError> {
    diesel::update(key_revocations::table.filter(key_revocations::id.eq(target_id)))
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

/// Fetch a key revocation by its coordinator-assigned id (primary key).
pub fn get_by_id(
    conn: &mut SqliteConnection,
    revocation_id: &str,
) -> Result<Option<KeyRevocationRow>, StorageError> {
    key_revocations::table
        .filter(key_revocations::id.eq(revocation_id))
        .select(KeyRevocationRow::as_select())
        .first::<KeyRevocationRow>(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Failed to fetch key_revocation by id: {e}")))
}

/// List all key revocations for a human (most recent first).
pub fn list_for_human(
    conn: &mut SqliteConnection,
    subject_human_id: &str,
) -> Result<Vec<KeyRevocationRow>, StorageError> {
    key_revocations::table
        .filter(key_revocations::subject_human_id.eq(subject_human_id))
        .order(key_revocations::created_at.desc())
        .select(KeyRevocationRow::as_select())
        .load(conn)
        .map_err(|e| {
            StorageError::Internal(format!("Failed to list key_revocations for human: {e}"))
        })
}

/// List all pending key revocations (threshold_reached = 0).
pub fn list_pending(
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
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

    const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

    fn test_conn() -> SqliteConnection {
        let mut conn = SqliteConnection::establish(":memory:").expect("in-memory DB");
        conn.run_pending_migrations(MIGRATIONS).expect("migrations");
        conn
    }

    fn sample_row(id: &str, threshold: i32) -> KeyRevocationRow {
        KeyRevocationRow {
            id: id.to_string(),
            dht_anchor_hash: format!("anchor-{id}").into_bytes(),
            subject_human_id: "human-matthew".into(),
            revoked_key: "uhCAkTEST".into(),
            trigger_type: "voluntary".into(),
            reason: "compromised".into(),
            initiated_by_cid: "human-matthew".into(),
            required_votes: 1,
            current_votes: threshold,
            threshold_reached: threshold,
            effective_at: if threshold == 1 {
                Some("2026-05-15T00:00:00Z".into())
            } else {
                None
            },
            derived_compromise_at: None,
            created_at: "2026-05-15T00:00:00Z".into(),
            updated_at: "2026-05-15T00:00:00Z".into(),
        }
    }

    #[test]
    fn upsert_is_idempotent() {
        let mut conn = test_conn();
        let row = sample_row("rev-R1", 1);
        upsert(&mut conn, &row).unwrap();
        upsert(&mut conn, &row).unwrap();
        let fetched = get_by_id(&mut conn, "rev-R1").unwrap();
        assert!(fetched.is_some());
        let fetched = fetched.unwrap();
        assert_eq!(fetched.subject_human_id, "human-matthew");
        assert_eq!(fetched.initiated_by_cid, "human-matthew");
    }

    #[test]
    fn set_effective_marks_threshold_reached() {
        let mut conn = test_conn();
        upsert(&mut conn, &sample_row("rev-R2", 0)).unwrap();
        set_effective(
            &mut conn,
            "rev-R2",
            "2026-05-15T01:00:00Z",
            "2026-05-15T01:00:00Z",
        )
        .unwrap();
        let row = get_by_id(&mut conn, "rev-R2").unwrap().unwrap();
        assert_eq!(row.threshold_reached, 1);
        assert_eq!(row.effective_at.as_deref(), Some("2026-05-15T01:00:00Z"));
    }

    #[test]
    fn update_current_votes_increments() {
        let mut conn = test_conn();
        upsert(&mut conn, &sample_row("rev-R3", 0)).unwrap();
        update_current_votes(&mut conn, "rev-R3", 2, "2026-05-15T01:00:00Z").unwrap();
        let row = get_by_id(&mut conn, "rev-R3").unwrap().unwrap();
        assert_eq!(row.current_votes, 2);
    }

    #[test]
    fn list_pending_excludes_effective() {
        let mut conn = test_conn();
        upsert(&mut conn, &sample_row("rev-R4", 0)).unwrap(); // pending
        upsert(&mut conn, &sample_row("rev-R5", 1)).unwrap(); // effective
        let pending = list_pending(&mut conn).unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, "rev-R4");
    }

    #[test]
    fn list_for_human_returns_only_matching_subject() {
        let mut conn = test_conn();
        upsert(&mut conn, &sample_row("rev-R6", 0)).unwrap();
        let mut other = sample_row("rev-R7", 0);
        other.subject_human_id = "human-jessica".into();
        upsert(&mut conn, &other).unwrap();
        let matthews = list_for_human(&mut conn, "human-matthew").unwrap();
        assert_eq!(matthews.len(), 1);
        assert_eq!(matthews[0].id, "rev-R6");
    }
}
