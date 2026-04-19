//! Projection of the notarized GateDecisionChallenge DHT entry (mishpat DNA).
//!
//! Writes here come exclusively from the post-commit signal projection in
//! `src/signals.rs` (`MishpatSignal::GateDecisionChallengeCreated`). Source of truth
//! is the mishpat DNA DHT. If this projection and the DHT disagree, the DHT
//! wins — rebuild from `get_gate_challenge_by_id` / query the mishpat DNA
//! directly rather than mutating this table.
//!
//! Column conventions:
//! - CIDs / ActionHashes: TEXT (base64 / CID string). No binary blobs cross
//!   the wire; `dht_anchor_hash` is the same shape as the signal payload.
//! - `evidence_refs`: comma-separated CID list stored as raw TEXT; parsing
//!   happens at the view boundary if needed.

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use crate::db::diesel_schema::gate_decision_challenges;

// ---------------------------------------------------------------------------
// Row model
// ---------------------------------------------------------------------------

/// Row-shaped projection of a GateDecisionChallenge DHT entry.
#[derive(Debug, Clone, Queryable, Insertable, AsChangeset, Identifiable)]
#[diesel(table_name = gate_decision_challenges, primary_key(app_id, challenge_id))]
pub struct GateDecisionChallengeRow {
    pub app_id: String,
    pub challenge_id: String,
    pub challenged_decision_cid: String,
    pub challenger_id: String,
    pub grounds: String,
    pub summary: String,
    pub evidence_refs: String,
    pub filed_at: String,
    pub reach: String,
    pub dht_anchor_hash: String,
    pub created_at: String,
    pub updated_at: String,
}

// ---------------------------------------------------------------------------
// Writes
// ---------------------------------------------------------------------------

/// Insert or replace a gate decision challenge row.
///
/// Uses `INSERT OR REPLACE` semantics so that a re-delivered signal is
/// idempotent. The primary key is `(app_id, challenge_id)` — `challenge_id`
/// is a CID so collisions indicate a true duplicate, not a logical conflict.
pub fn upsert(conn: &mut SqliteConnection, row: &GateDecisionChallengeRow) -> QueryResult<usize> {
    diesel::insert_into(gate_decision_challenges::table)
        .values(row)
        .on_conflict((
            gate_decision_challenges::app_id,
            gate_decision_challenges::challenge_id,
        ))
        .do_update()
        .set(row)
        .execute(conn)
}

// ---------------------------------------------------------------------------
// Reads
// ---------------------------------------------------------------------------

/// Look up a challenge by its CID (challenge_id) within an app scope.
pub fn find_by_id(
    conn: &mut SqliteConnection,
    app_id: &str,
    challenge_id: &str,
) -> QueryResult<Option<GateDecisionChallengeRow>> {
    gate_decision_challenges::table
        .filter(gate_decision_challenges::app_id.eq(app_id))
        .filter(gate_decision_challenges::challenge_id.eq(challenge_id))
        .first::<GateDecisionChallengeRow>(conn)
        .optional()
}

/// List all challenges filed by a given challenger agent within an app scope.
pub fn find_by_challenger(
    conn: &mut SqliteConnection,
    app_id: &str,
    challenger_id: &str,
) -> QueryResult<Vec<GateDecisionChallengeRow>> {
    gate_decision_challenges::table
        .filter(gate_decision_challenges::app_id.eq(app_id))
        .filter(gate_decision_challenges::challenger_id.eq(challenger_id))
        .order(gate_decision_challenges::filed_at.desc())
        .load::<GateDecisionChallengeRow>(conn)
}

/// List all challenges against a specific gate decision CID within an app scope.
pub fn find_by_challenged_decision(
    conn: &mut SqliteConnection,
    app_id: &str,
    challenged_decision_cid: &str,
) -> QueryResult<Vec<GateDecisionChallengeRow>> {
    gate_decision_challenges::table
        .filter(gate_decision_challenges::app_id.eq(app_id))
        .filter(gate_decision_challenges::challenged_decision_cid.eq(challenged_decision_cid))
        .order(gate_decision_challenges::filed_at.desc())
        .load::<GateDecisionChallengeRow>(conn)
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
            CREATE INDEX idx_gate_challenges_challenger ON gate_decision_challenges(challenger_id);
            CREATE INDEX idx_gate_challenges_decision ON gate_decision_challenges(challenged_decision_cid);
            CREATE INDEX idx_gate_challenges_grounds ON gate_decision_challenges(grounds);
            CREATE INDEX idx_gate_challenges_filed_at ON gate_decision_challenges(filed_at);
            "#,
        )
        .expect("Failed to create test table");

        conn
    }

    fn make_row(
        challenge_id: &str,
        challenged_decision_cid: &str,
        challenger_id: &str,
        grounds: &str,
    ) -> GateDecisionChallengeRow {
        GateDecisionChallengeRow {
            app_id: "test-app".into(),
            challenge_id: challenge_id.into(),
            challenged_decision_cid: challenged_decision_cid.into(),
            challenger_id: challenger_id.into(),
            grounds: grounds.into(),
            summary: "Decision appears to violate constitutional principles".into(),
            evidence_refs: "bafybeievidence1,bafybeievidence2".into(),
            filed_at: "2026-04-19T10:00:00Z".into(),
            reach: "community".into(),
            dht_anchor_hash: "uhCkkCHAL".into(),
            created_at: "2026-04-19T10:00:00".into(),
            updated_at: "2026-04-19T10:00:00".into(),
        }
    }

    #[test]
    fn upsert_then_find_by_id() {
        let mut conn = setup_test_db();
        let row = make_row("bafyCHAL1", "bafyDEC1", "uhCAkCHALLENGER", "constitutional");
        upsert(&mut conn, &row).unwrap();

        let found = find_by_id(&mut conn, "test-app", "bafyCHAL1")
            .unwrap()
            .unwrap();
        assert_eq!(found.grounds, "constitutional");
        assert_eq!(found.challenged_decision_cid, "bafyDEC1");
        assert_eq!(found.dht_anchor_hash, "uhCkkCHAL");
    }

    #[test]
    fn upsert_is_idempotent() {
        let mut conn = setup_test_db();
        let row = make_row("bafyCHAL1", "bafyDEC1", "uhCAkCHALLENGER", "safety");
        upsert(&mut conn, &row).unwrap();
        // Re-deliver the same signal — should not fail or duplicate
        upsert(&mut conn, &row).unwrap();

        let rows = find_by_challenger(&mut conn, "test-app", "uhCAkCHALLENGER").unwrap();
        assert_eq!(
            rows.len(),
            1,
            "Re-delivered signal must not duplicate the row"
        );
    }

    #[test]
    fn find_by_challenger_returns_only_that_agents_challenges() {
        let mut conn = setup_test_db();
        let row_a = make_row("bafyCHAL1", "bafyDEC1", "uhCAkAGENT1", "constitutional");
        let row_b = make_row("bafyCHAL2", "bafyDEC2", "uhCAkAGENT2", "safety");

        upsert(&mut conn, &row_a).unwrap();
        upsert(&mut conn, &row_b).unwrap();

        let challenges = find_by_challenger(&mut conn, "test-app", "uhCAkAGENT1").unwrap();
        assert_eq!(challenges.len(), 1);
        assert_eq!(challenges[0].challenge_id, "bafyCHAL1");
    }

    #[test]
    fn find_by_challenged_decision_returns_all_challenges_for_that_decision() {
        let mut conn = setup_test_db();
        let row_a = make_row("bafyCHAL1", "bafyDEC1", "uhCAkAGENT1", "constitutional");
        let row_b = make_row("bafyCHAL2", "bafyDEC1", "uhCAkAGENT2", "safety");
        let row_c = make_row("bafyCHAL3", "bafyDEC2", "uhCAkAGENT3", "policy");

        upsert(&mut conn, &row_a).unwrap();
        upsert(&mut conn, &row_b).unwrap();
        upsert(&mut conn, &row_c).unwrap();

        let challenges = find_by_challenged_decision(&mut conn, "test-app", "bafyDEC1").unwrap();
        assert_eq!(challenges.len(), 2);
        assert!(challenges
            .iter()
            .all(|c| c.challenged_decision_cid == "bafyDEC1"));
    }

    #[test]
    fn find_by_id_returns_none_for_missing() {
        let mut conn = setup_test_db();
        assert!(find_by_id(&mut conn, "test-app", "missing-cid")
            .unwrap()
            .is_none());
    }
}
