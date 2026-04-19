//! Projection of the notarized ChallengeOutcome DHT entry (mishpat DNA).
//!
//! Writes here come exclusively from the post-commit signal projection in
//! `src/signals.rs` (`MishpatSignal::ChallengeOutcomeCreated`). Source of truth
//! is the mishpat DNA DHT. If this projection and the DHT disagree, the DHT
//! wins — rebuild from `get_challenge_outcome_by_id` / query the mishpat DNA
//! directly rather than mutating this table.
//!
//! Column conventions:
//! - JSON blobs: TEXT (raw serialized), surfaced as `serde_json::Value` at the
//!   view boundary.
//! - CIDs / ActionHashes: TEXT (base64 / CID string). No binary blobs cross
//!   the wire; `dht_anchor_hash` is the same shape as the signal payload.
//! - `reviewer_consensus`: comma-separated AgentPubKey list stored as raw TEXT.

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use crate::db::diesel_schema::challenge_outcomes;

// ---------------------------------------------------------------------------
// Row model
// ---------------------------------------------------------------------------

/// Row-shaped projection of a ChallengeOutcome DHT entry.
///
/// JSON fields (`reasoning_json`, `indemnification_actions_json`) are stored
/// as raw TEXT and converted to `serde_json::Value` at the view boundary —
/// never in this module.
#[derive(Debug, Clone, Queryable, Insertable, AsChangeset, Identifiable)]
#[diesel(table_name = challenge_outcomes, primary_key(app_id, outcome_id))]
pub struct ChallengeOutcomeRow {
    pub app_id: String,
    pub outcome_id: String,
    pub challenge_cid: String,
    pub verdict: String,
    pub reviewer_consensus: String,
    pub reasoning_json: String,
    pub decided_at: String,
    pub indemnification_actions_json: String,
    pub dht_anchor_hash: String,
    pub created_at: String,
    pub updated_at: String,
}

// ---------------------------------------------------------------------------
// Writes
// ---------------------------------------------------------------------------

/// Insert or replace a challenge outcome row.
///
/// Uses `INSERT OR REPLACE` semantics so that a re-delivered signal is
/// idempotent. The primary key is `(app_id, outcome_id)` — `outcome_id`
/// is a CID so collisions indicate a true duplicate, not a logical conflict.
pub fn upsert(conn: &mut SqliteConnection, row: &ChallengeOutcomeRow) -> QueryResult<usize> {
    diesel::insert_into(challenge_outcomes::table)
        .values(row)
        .on_conflict((
            challenge_outcomes::app_id,
            challenge_outcomes::outcome_id,
        ))
        .do_update()
        .set(row)
        .execute(conn)
}

// ---------------------------------------------------------------------------
// Reads
// ---------------------------------------------------------------------------

/// Look up an outcome by its CID (outcome_id) within an app scope.
pub fn find_by_id(
    conn: &mut SqliteConnection,
    app_id: &str,
    outcome_id: &str,
) -> QueryResult<Option<ChallengeOutcomeRow>> {
    challenge_outcomes::table
        .filter(challenge_outcomes::app_id.eq(app_id))
        .filter(challenge_outcomes::outcome_id.eq(outcome_id))
        .first::<ChallengeOutcomeRow>(conn)
        .optional()
}

/// Look up the outcome that closes a given challenge CID within an app scope.
///
/// Returns at most one result — each challenge has exactly one closing outcome.
pub fn find_by_challenge(
    conn: &mut SqliteConnection,
    app_id: &str,
    challenge_cid: &str,
) -> QueryResult<Option<ChallengeOutcomeRow>> {
    challenge_outcomes::table
        .filter(challenge_outcomes::app_id.eq(app_id))
        .filter(challenge_outcomes::challenge_cid.eq(challenge_cid))
        .first::<ChallengeOutcomeRow>(conn)
        .optional()
}

/// List all outcomes with a given verdict within an app scope.
pub fn find_by_verdict(
    conn: &mut SqliteConnection,
    app_id: &str,
    verdict: &str,
) -> QueryResult<Vec<ChallengeOutcomeRow>> {
    challenge_outcomes::table
        .filter(challenge_outcomes::app_id.eq(app_id))
        .filter(challenge_outcomes::verdict.eq(verdict))
        .order(challenge_outcomes::decided_at.desc())
        .load::<ChallengeOutcomeRow>(conn)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use diesel::connection::SimpleConnection;

    fn setup_test_db() -> SqliteConnection {
        let mut conn =
            SqliteConnection::establish(":memory:").expect("Failed to create in-memory database");

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
            CREATE INDEX idx_challenge_outcomes_challenge ON challenge_outcomes(challenge_cid);
            CREATE INDEX idx_challenge_outcomes_verdict ON challenge_outcomes(verdict);
            CREATE INDEX idx_challenge_outcomes_decided_at ON challenge_outcomes(decided_at);
            "#,
        )
        .expect("Failed to create test table");

        conn
    }

    fn make_row(
        outcome_id: &str,
        challenge_cid: &str,
        verdict: &str,
    ) -> ChallengeOutcomeRow {
        ChallengeOutcomeRow {
            app_id: "test-app".into(),
            outcome_id: outcome_id.into(),
            challenge_cid: challenge_cid.into(),
            verdict: verdict.into(),
            reviewer_consensus: "uhCAkREVIEWER1,uhCAkREVIEWER2".into(),
            reasoning_json: r#"{"summary":"Evidence reviewed","steps":[]}"#.into(),
            decided_at: "2026-04-20T10:00:00Z".into(),
            indemnification_actions_json: "[]".into(),
            dht_anchor_hash: "uhCkkOUTCOME".into(),
            created_at: "2026-04-20T10:00:00".into(),
            updated_at: "2026-04-20T10:00:00".into(),
        }
    }

    #[test]
    fn upsert_then_find_by_id() {
        let mut conn = setup_test_db();
        let row = make_row("bafyOUT1", "bafyCHAL1", "upheld");
        upsert(&mut conn, &row).unwrap();

        let found = find_by_id(&mut conn, "test-app", "bafyOUT1")
            .unwrap()
            .unwrap();
        assert_eq!(found.verdict, "upheld");
        assert_eq!(found.challenge_cid, "bafyCHAL1");
        assert_eq!(found.dht_anchor_hash, "uhCkkOUTCOME");
    }

    #[test]
    fn upsert_is_idempotent() {
        let mut conn = setup_test_db();
        let row = make_row("bafyOUT1", "bafyCHAL1", "upheld");
        upsert(&mut conn, &row).unwrap();
        // Re-deliver the same signal — should not fail or duplicate
        upsert(&mut conn, &row).unwrap();

        let rows = find_by_verdict(&mut conn, "test-app", "upheld").unwrap();
        assert_eq!(rows.len(), 1, "Re-delivered signal must not duplicate the row");
    }

    #[test]
    fn find_by_challenge_returns_outcome_for_that_challenge() {
        let mut conn = setup_test_db();
        let row_a = make_row("bafyOUT1", "bafyCHAL1", "upheld");
        let row_b = make_row("bafyOUT2", "bafyCHAL2", "dismissed");

        upsert(&mut conn, &row_a).unwrap();
        upsert(&mut conn, &row_b).unwrap();

        let found = find_by_challenge(&mut conn, "test-app", "bafyCHAL1")
            .unwrap()
            .unwrap();
        assert_eq!(found.outcome_id, "bafyOUT1");
        assert_eq!(found.verdict, "upheld");
    }

    #[test]
    fn find_by_verdict_filters_correctly() {
        let mut conn = setup_test_db();
        upsert(&mut conn, &make_row("bafyOUT1", "bafyCHAL1", "upheld")).unwrap();
        upsert(&mut conn, &make_row("bafyOUT2", "bafyCHAL2", "dismissed")).unwrap();
        upsert(&mut conn, &make_row("bafyOUT3", "bafyCHAL3", "upheld")).unwrap();

        let upheld = find_by_verdict(&mut conn, "test-app", "upheld").unwrap();
        assert_eq!(upheld.len(), 2);
        assert!(upheld.iter().all(|r| r.verdict == "upheld"));

        let dismissed = find_by_verdict(&mut conn, "test-app", "dismissed").unwrap();
        assert_eq!(dismissed.len(), 1);
        assert_eq!(dismissed[0].outcome_id, "bafyOUT2");
    }

    #[test]
    fn find_by_challenge_returns_none_for_missing() {
        let mut conn = setup_test_db();
        assert!(find_by_challenge(&mut conn, "test-app", "missing-cid")
            .unwrap()
            .is_none());
    }

    #[test]
    fn find_by_id_returns_none_for_missing() {
        let mut conn = setup_test_db();
        assert!(find_by_id(&mut conn, "test-app", "missing-cid")
            .unwrap()
            .is_none());
    }
}
