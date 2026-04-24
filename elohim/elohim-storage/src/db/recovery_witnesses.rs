//! Projection CRUD for recovery witness accumulation under an IntimateQuorum
//! recovery request. Idempotent on `dht_anchor_hash`.
//!
//! Source of truth: DHT (imagodei HumanityWitness entry, linked via
//! RecoveryRequestToHumanityWitness). This table is rebuildable from signal replay.

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use crate::db::diesel_schema::recovery_witnesses;
use crate::db::models::{NewRecoveryWitnessRow, RecoveryWitnessRow};
use crate::error::StorageError;

/// Insert-or-replace a witness row. Signal projection is idempotent.
pub fn upsert_recovery_witness(
    conn: &mut SqliteConnection,
    row: NewRecoveryWitnessRow,
) -> Result<(), StorageError> {
    diesel::insert_into(recovery_witnesses::table)
        .values(&row)
        .on_conflict(recovery_witnesses::dht_anchor_hash)
        .do_update()
        .set((
            recovery_witnesses::recovery_request_hash.eq(&row.recovery_request_hash),
            recovery_witnesses::witness_agent_id.eq(&row.witness_agent_id),
            recovery_witnesses::human_id.eq(&row.human_id),
            recovery_witnesses::note.eq(&row.note),
            recovery_witnesses::submitted_at.eq(&row.submitted_at),
        ))
        .execute(conn)
        .map(|_| ())
        .map_err(|e| StorageError::Internal(format!("Failed to upsert recovery_witness: {e}")))
}

/// Count witnesses submitted for a given recovery request (for UI "2 of 3" rendering).
pub fn count_witnesses_for_request(
    conn: &mut SqliteConnection,
    request_hash: &str,
) -> Result<i64, StorageError> {
    recovery_witnesses::table
        .filter(recovery_witnesses::recovery_request_hash.eq(request_hash))
        .count()
        .get_result(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to count recovery_witnesses: {e}")))
}

/// List all witnesses for a request (ordered by submitted_at).
pub fn list_witnesses_for_request(
    conn: &mut SqliteConnection,
    request_hash: &str,
) -> Result<Vec<RecoveryWitnessRow>, StorageError> {
    recovery_witnesses::table
        .filter(recovery_witnesses::recovery_request_hash.eq(request_hash))
        .order(recovery_witnesses::submitted_at.asc())
        .select(RecoveryWitnessRow::as_select())
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to list recovery_witnesses: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use diesel::connection::SimpleConnection;

    fn test_conn() -> SqliteConnection {
        let mut conn = SqliteConnection::establish(":memory:").unwrap();
        conn.batch_execute(include_str!(
            "../../migrations/2026-04-24-000000_recovery_witnesses/up.sql"
        ))
        .unwrap();
        conn
    }

    #[test]
    fn upsert_is_idempotent() {
        let mut conn = test_conn();
        let row = NewRecoveryWitnessRow {
            dht_anchor_hash: "W1".into(),
            recovery_request_hash: "R1".into(),
            witness_agent_id: "A1".into(),
            human_id: "H1".into(),
            note: Some("hi".into()),
            submitted_at: "2026-04-24T00:00:00Z".into(),
        };
        upsert_recovery_witness(&mut conn, row.clone()).unwrap();
        upsert_recovery_witness(&mut conn, row).unwrap();
        assert_eq!(count_witnesses_for_request(&mut conn, "R1").unwrap(), 1);
    }

    #[test]
    fn lists_in_submitted_order() {
        let mut conn = test_conn();
        for (w, t) in [("W1", "2026-04-24T00:00:02Z"), ("W2", "2026-04-24T00:00:01Z")] {
            upsert_recovery_witness(
                &mut conn,
                NewRecoveryWitnessRow {
                    dht_anchor_hash: w.into(),
                    recovery_request_hash: "R1".into(),
                    witness_agent_id: "A".into(),
                    human_id: "H".into(),
                    note: None,
                    submitted_at: t.into(),
                },
            )
            .unwrap();
        }
        let rows = list_witnesses_for_request(&mut conn, "R1").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].dht_anchor_hash, "W2"); // earlier
        assert_eq!(rows[1].dht_anchor_hash, "W1"); // later
    }
}
