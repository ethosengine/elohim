//! Recovery Protocol Phase 2 — M4 projection CRUD for revocation_votes.
//!
//! Source of truth: DHT (imagodei RevocationVote entries). These rows are
//! read-optimized projections populated via RecoveryV2Signal::RevocationVoteSubmitted.
//!
//! Votes are immutable once committed to the DHT; the projection reflects this
//! with insert-or-ignore semantics on (revocation_id, steward_id).

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use crate::db::diesel_schema::revocation_votes;
use crate::db::models::{InsertRevocationVoteRow, RevocationVoteRow};
use crate::error::StorageError;

// ============================================================================
// RevocationVote projection
// ============================================================================

/// Insert a revocation vote projection row.
///
/// Idempotent: a second vote from the same steward on the same revocation is
/// a bug upstream (the DHT coordinator rejects duplicate votes). At the
/// projection layer we silently ignore the duplicate so signal replay is safe.
pub fn insert_revocation_vote(
    conn: &mut SqliteConnection,
    row: InsertRevocationVoteRow,
) -> Result<(), StorageError> {
    diesel::insert_into(revocation_votes::table)
        .values(&row)
        .on_conflict((
            revocation_votes::revocation_id,
            revocation_votes::steward_id,
        ))
        .do_nothing()
        .execute(conn)
        .map(|_| ())
        .map_err(|e| StorageError::Internal(format!("Failed to insert revocation_vote: {e}")))
}

/// Count approved votes for a given revocation (identified by its DHT anchor hash).
///
/// Used by RevocationVoteSubmitted handler to recompute the denormalized
/// `current_votes` on the key_revocations row.
pub fn count_approved_votes_for_revocation(
    conn: &mut SqliteConnection,
    revocation_dht_anchor_hash: &str,
) -> Result<i64, StorageError> {
    revocation_votes::table
        .filter(revocation_votes::revocation_dht_anchor_hash.eq(revocation_dht_anchor_hash))
        .filter(revocation_votes::approved.eq(1))
        .count()
        .get_result(conn)
        .map_err(|e| {
            StorageError::Internal(format!("Failed to count approved revocation_votes: {e}"))
        })
}

/// List all votes for a revocation (identified by revocation_id string).
///
/// Used for audit / UI rendering of "N of M" vote progress.
pub fn list_votes_for_revocation(
    conn: &mut SqliteConnection,
    revocation_id: &str,
) -> Result<Vec<RevocationVoteRow>, StorageError> {
    revocation_votes::table
        .filter(revocation_votes::revocation_id.eq(revocation_id))
        .order(revocation_votes::voted_at.asc())
        .select(RevocationVoteRow::as_select())
        .load(conn)
        .map_err(|e| {
            StorageError::Internal(format!(
                "Failed to list revocation_votes for revocation: {e}"
            ))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use diesel::connection::SimpleConnection;

    fn test_conn() -> SqliteConnection {
        let mut conn = SqliteConnection::establish(":memory:").unwrap();
        conn.batch_execute(include_str!(
            "../../migrations/2026-04-24-020000_revocation_votes/up.sql"
        ))
        .unwrap();
        conn
    }

    fn sample_vote(dht_hash: &str, steward: &str, approved: i32) -> InsertRevocationVoteRow {
        InsertRevocationVoteRow {
            dht_anchor_hash: dht_hash.into(),
            id: format!("vote-{dht_hash}"),
            revocation_dht_anchor_hash: "REV_ANCHOR".into(),
            revocation_id: "rev-jessica-2026-04-24".into(),
            steward_id: steward.into(),
            approved,
            attestation: "Verified identity.".into(),
            voted_at: "2026-04-24T00:00:00Z".into(),
        }
    }

    #[test]
    fn insert_is_idempotent_on_duplicate_steward() {
        let mut conn = test_conn();
        let row = sample_vote("V1", "human-timothy", 1);
        insert_revocation_vote(&mut conn, row.clone()).unwrap();
        insert_revocation_vote(&mut conn, row).unwrap(); // duplicate — no-op
        let votes = list_votes_for_revocation(&mut conn, "rev-jessica-2026-04-24").unwrap();
        assert_eq!(votes.len(), 1);
    }

    #[test]
    fn count_approved_excludes_rejections() {
        let mut conn = test_conn();
        insert_revocation_vote(&mut conn, sample_vote("V2", "human-timothy", 1)).unwrap();
        insert_revocation_vote(&mut conn, sample_vote("V3", "human-sarah", 0)).unwrap();
        let count = count_approved_votes_for_revocation(&mut conn, "REV_ANCHOR").unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn list_votes_in_submission_order() {
        let mut conn = test_conn();
        let mut v2 = sample_vote("V4", "human-timothy", 1);
        v2.voted_at = "2026-04-24T00:00:02Z".into();
        let mut v1 = sample_vote("V5", "human-sarah", 0);
        v1.voted_at = "2026-04-24T00:00:01Z".into();
        insert_revocation_vote(&mut conn, v2).unwrap();
        insert_revocation_vote(&mut conn, v1).unwrap();
        let votes = list_votes_for_revocation(&mut conn, "rev-jessica-2026-04-24").unwrap();
        assert_eq!(votes.len(), 2);
        assert_eq!(votes[0].dht_anchor_hash, "V5"); // earlier
        assert_eq!(votes[1].dht_anchor_hash, "V4"); // later
    }
}
