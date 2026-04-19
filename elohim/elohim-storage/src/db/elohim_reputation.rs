//! Reputation aggregation query for an elohim agent.
//!
//! Computes a multi-dimensional reputation profile by walking the outcome graph:
//!   GateDecisionAttestation (phase=elohim-active, within window)
//!     -> GateDecisionChallenge (challenged_decision_cid = decision_id)
//!       -> ChallengeOutcome (challenge_cid = challenge_id)
//!
//! Source of truth: mishpat DNA DHT. The numbers here are computed from the
//! read-optimised SQLite projections of those entries. If the projections and
//! the DHT disagree, the DHT wins — a reputation rebuild re-reads from the DHT.
//!
//! Design principles:
//! - Phase filter is mandatory: dev-context decisions carry no reputation weight.
//! - No scalar score is produced. Dimensions are raw counts; weighting is deferred.
//! - In-memory accumulation is used for histograms because the working set is
//!   small (an elohim's decisions over a 90-day window, typically < 1000 rows).

use std::collections::HashMap;

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

// ---------------------------------------------------------------------------
// Private row types for raw SQL queries
// ---------------------------------------------------------------------------

#[derive(QueryableByName, Debug)]
struct DecisionRow {
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub decision_id: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub elohim_substance_cid: String,
    /// Populated by ORDER BY decided_at DESC so the first row is the most-recent substance.
    /// Field value is not used in Rust — the ordering is what matters.
    #[allow(dead_code)]
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub decided_at: String,
}

#[derive(QueryableByName, Debug)]
struct ChallengeRow {
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub challenge_id: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub challenged_decision_cid: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub grounds: String,
}

#[derive(QueryableByName, Debug)]
struct OutcomeRow {
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub challenge_cid: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub verdict: String,
}

// ---------------------------------------------------------------------------
// Public input / output types
// ---------------------------------------------------------------------------

/// Parameters for a reputation aggregation query.
pub struct ReputationQuery {
    pub app_id: String,
    pub elohim_id: String,
    /// ISO 8601 string — decisions with decided_at >= this value are included.
    pub window_start: String,
    /// ISO 8601 string — decisions with decided_at <= this value are included.
    pub window_end: String,
}

/// Raw aggregation result, pre-conversion to the HTTP view.
///
/// All counts are `i64` to match SQLite INTEGER affinity and avoid cast noise
/// at the view boundary.
pub struct ReputationResult {
    /// CID of the most-recently-used substance in the window (None if no decisions).
    pub current_substance_cid: Option<String>,
    /// Count of elohim-active decisions in the window.
    pub total_decisions: i64,
    /// Count of distinct decisions that attracted at least one challenge.
    pub challenged_count: i64,
    /// Count of ChallengeOutcomes with verdict=upheld.
    pub upheld_count: i64,
    /// Count of ChallengeOutcomes with verdict=dismissed.
    pub dismissed_count: i64,
    /// Count of ChallengeOutcomes with verdict=superseded.
    pub superseded_count: i64,
    /// Count of challenges without a ChallengeOutcome yet.
    pub pending_count: i64,
    /// Histogram: grounds -> count.
    pub challenges_by_grounds: HashMap<String, i64>,
    /// Histogram: verdict -> count.
    pub outcomes_by_verdict: HashMap<String, i64>,
}

// ---------------------------------------------------------------------------
// Query implementation
// ---------------------------------------------------------------------------

/// Compute a reputation profile for the given elohim agent and time window.
///
/// Returns `QueryResult<ReputationResult>` — callers handle the `Err` case
/// as a 500 at the HTTP layer; a well-formed empty result (all zeros) is always
/// returned when the window contains no matching decisions.
pub fn compute(conn: &mut SqliteConnection, q: &ReputationQuery) -> QueryResult<ReputationResult> {
    // ------------------------------------------------------------------
    // Step 1: Load all elohim-active decisions in the window.
    //
    // We use raw SQL here because Diesel's type-checked DSL becomes
    // cumbersome for the multi-table join pattern that follows.
    // ------------------------------------------------------------------
    let decisions: Vec<DecisionRow> = diesel::sql_query(
        "SELECT decision_id, elohim_substance_cid, decided_at \
         FROM gate_decision_attestations \
         WHERE app_id = ? \
           AND elohim_id = ? \
           AND phase = 'elohim-active' \
           AND decided_at >= ? \
           AND decided_at <= ? \
         ORDER BY decided_at DESC",
    )
    .bind::<diesel::sql_types::Text, _>(&q.app_id)
    .bind::<diesel::sql_types::Text, _>(&q.elohim_id)
    .bind::<diesel::sql_types::Text, _>(&q.window_start)
    .bind::<diesel::sql_types::Text, _>(&q.window_end)
    .load(conn)?;

    let total_decisions = decisions.len() as i64;
    // Most-recent substance: decisions are ordered decided_at DESC so the first row is newest.
    let current_substance_cid = decisions.first().map(|r| r.elohim_substance_cid.clone());

    if decisions.is_empty() {
        return Ok(ReputationResult {
            current_substance_cid: None,
            total_decisions: 0,
            challenged_count: 0,
            upheld_count: 0,
            dismissed_count: 0,
            superseded_count: 0,
            pending_count: 0,
            challenges_by_grounds: HashMap::new(),
            outcomes_by_verdict: HashMap::new(),
        });
    }

    let decision_ids: Vec<String> = decisions.into_iter().map(|r| r.decision_id).collect();

    // ------------------------------------------------------------------
    // Step 2: Load all challenges whose challenged_decision_cid is one of
    // the decisions we just found.
    //
    // Diesel does not natively bind Vec<String> as an IN list in raw SQL,
    // so we iterate with one query per decision_id. For typical window sizes
    // (tens to low hundreds of decisions) this is fine.
    // ------------------------------------------------------------------
    let mut all_challenge_ids: Vec<String> = Vec::new();
    let mut challenged_decision_set: std::collections::HashSet<String> =
        std::collections::HashSet::new();
    let mut challenges_by_grounds: HashMap<String, i64> = HashMap::new();

    for decision_id in &decision_ids {
        let challenges: Vec<ChallengeRow> = diesel::sql_query(
            "SELECT challenge_id, challenged_decision_cid, grounds \
             FROM gate_decision_challenges \
             WHERE app_id = ? AND challenged_decision_cid = ?",
        )
        .bind::<diesel::sql_types::Text, _>(&q.app_id)
        .bind::<diesel::sql_types::Text, _>(decision_id)
        .load(conn)?;

        for c in challenges {
            challenged_decision_set.insert(c.challenged_decision_cid.clone());
            *challenges_by_grounds.entry(c.grounds.clone()).or_insert(0) += 1;
            all_challenge_ids.push(c.challenge_id);
        }
    }

    let challenged_count = challenged_decision_set.len() as i64;

    // ------------------------------------------------------------------
    // Step 3: Load ChallengeOutcomes for all challenge_ids found above.
    // ------------------------------------------------------------------
    let mut outcomes_by_verdict: HashMap<String, i64> = HashMap::new();
    let mut resolved_challenge_set: std::collections::HashSet<String> =
        std::collections::HashSet::new();

    for challenge_id in &all_challenge_ids {
        let outcomes: Vec<OutcomeRow> = diesel::sql_query(
            "SELECT challenge_cid, verdict \
             FROM challenge_outcomes \
             WHERE app_id = ? AND challenge_cid = ?",
        )
        .bind::<diesel::sql_types::Text, _>(&q.app_id)
        .bind::<diesel::sql_types::Text, _>(challenge_id)
        .load(conn)?;

        for o in outcomes {
            *outcomes_by_verdict.entry(o.verdict.clone()).or_insert(0) += 1;
            resolved_challenge_set.insert(o.challenge_cid);
        }
    }

    let upheld_count = outcomes_by_verdict.get("upheld").copied().unwrap_or(0);
    let dismissed_count = outcomes_by_verdict.get("dismissed").copied().unwrap_or(0);
    let superseded_count = outcomes_by_verdict.get("superseded").copied().unwrap_or(0);

    // Pending = challenges that have no outcome yet.
    let pending_count = all_challenge_ids
        .iter()
        .filter(|id| !resolved_challenge_set.contains(*id))
        .count() as i64;

    Ok(ReputationResult {
        current_substance_cid,
        total_decisions,
        challenged_count,
        upheld_count,
        dismissed_count,
        superseded_count,
        pending_count,
        challenges_by_grounds,
        outcomes_by_verdict,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use diesel::connection::SimpleConnection;

    fn setup_db() -> SqliteConnection {
        let mut conn =
            SqliteConnection::establish(":memory:").expect("in-memory SQLite");

        conn.batch_execute(
            r#"
            CREATE TABLE gate_decision_attestations (
                app_id TEXT NOT NULL,
                decision_id TEXT NOT NULL,
                phase TEXT NOT NULL,
                elohim_id TEXT NOT NULL,
                elohim_substance_cid TEXT NOT NULL,
                gate_name TEXT NOT NULL,
                gate_process_cid TEXT NOT NULL,
                request_ref_json TEXT NOT NULL,
                decision TEXT NOT NULL,
                reasoning_json TEXT NOT NULL,
                context_summary_cid TEXT NOT NULL,
                decided_at TEXT NOT NULL,
                universal_band_cid TEXT NOT NULL,
                dht_anchor_hash TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                PRIMARY KEY (app_id, decision_id)
            );
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
            "#,
        )
        .expect("schema setup");

        conn
    }

    fn insert_decision(
        conn: &mut SqliteConnection,
        decision_id: &str,
        phase: &str,
        elohim_id: &str,
        substance_cid: &str,
        decided_at: &str,
    ) {
        diesel::sql_query(
            "INSERT INTO gate_decision_attestations \
             (app_id, decision_id, phase, elohim_id, elohim_substance_cid, gate_name, \
              gate_process_cid, request_ref_json, decision, reasoning_json, \
              context_summary_cid, decided_at, universal_band_cid, dht_anchor_hash) \
             VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
        )
        .bind::<diesel::sql_types::Text, _>("test-app")
        .bind::<diesel::sql_types::Text, _>(decision_id)
        .bind::<diesel::sql_types::Text, _>(phase)
        .bind::<diesel::sql_types::Text, _>(elohim_id)
        .bind::<diesel::sql_types::Text, _>(substance_cid)
        .bind::<diesel::sql_types::Text, _>("gate-a")
        .bind::<diesel::sql_types::Text, _>("bafyPROC")
        .bind::<diesel::sql_types::Text, _>(r#"{}"#)
        .bind::<diesel::sql_types::Text, _>("allow")
        .bind::<diesel::sql_types::Text, _>(r#"{}"#)
        .bind::<diesel::sql_types::Text, _>("bafyCTX")
        .bind::<diesel::sql_types::Text, _>(decided_at)
        .bind::<diesel::sql_types::Text, _>("bafyUNIV")
        .bind::<diesel::sql_types::Text, _>("uhCkkANCHOR")
        .execute(conn)
        .expect("insert decision");
    }

    fn insert_challenge(
        conn: &mut SqliteConnection,
        challenge_id: &str,
        challenged_decision_cid: &str,
        grounds: &str,
    ) {
        diesel::sql_query(
            "INSERT INTO gate_decision_challenges \
             (app_id, challenge_id, challenged_decision_cid, challenger_id, grounds, \
              summary, evidence_refs, filed_at, reach, dht_anchor_hash) \
             VALUES (?,?,?,?,?,?,?,?,?,?)",
        )
        .bind::<diesel::sql_types::Text, _>("test-app")
        .bind::<diesel::sql_types::Text, _>(challenge_id)
        .bind::<diesel::sql_types::Text, _>(challenged_decision_cid)
        .bind::<diesel::sql_types::Text, _>("uhCAkCHALLENGER")
        .bind::<diesel::sql_types::Text, _>(grounds)
        .bind::<diesel::sql_types::Text, _>("summary text")
        .bind::<diesel::sql_types::Text, _>("")
        .bind::<diesel::sql_types::Text, _>("2026-04-19T10:00:00Z")
        .bind::<diesel::sql_types::Text, _>("community")
        .bind::<diesel::sql_types::Text, _>("uhCkkCHAL")
        .execute(conn)
        .expect("insert challenge");
    }

    fn insert_outcome(
        conn: &mut SqliteConnection,
        outcome_id: &str,
        challenge_cid: &str,
        verdict: &str,
    ) {
        diesel::sql_query(
            "INSERT INTO challenge_outcomes \
             (app_id, outcome_id, challenge_cid, verdict, reviewer_consensus, \
              reasoning_json, decided_at, indemnification_actions_json, dht_anchor_hash) \
             VALUES (?,?,?,?,?,?,?,?,?)",
        )
        .bind::<diesel::sql_types::Text, _>("test-app")
        .bind::<diesel::sql_types::Text, _>(outcome_id)
        .bind::<diesel::sql_types::Text, _>(challenge_cid)
        .bind::<diesel::sql_types::Text, _>(verdict)
        .bind::<diesel::sql_types::Text, _>("uhCAkREVIEWER")
        .bind::<diesel::sql_types::Text, _>(r#"{}"#)
        .bind::<diesel::sql_types::Text, _>("2026-04-20T10:00:00Z")
        .bind::<diesel::sql_types::Text, _>("[]")
        .bind::<diesel::sql_types::Text, _>("uhCkkOUT")
        .execute(conn)
        .expect("insert outcome");
    }

    fn default_query() -> ReputationQuery {
        ReputationQuery {
            app_id: "test-app".into(),
            elohim_id: "uhCAkELOHIM".into(),
            window_start: "2026-01-01T00:00:00Z".into(),
            window_end: "2026-12-31T23:59:59Z".into(),
        }
    }

    // -------------------------------------------------------------------------
    // Test 1: Empty result — no decisions in window -> everything zero
    // -------------------------------------------------------------------------
    #[test]
    fn empty_result_when_no_decisions() {
        let mut conn = setup_db();
        let result = compute(&mut conn, &default_query()).unwrap();

        assert_eq!(result.total_decisions, 0);
        assert_eq!(result.challenged_count, 0);
        assert_eq!(result.upheld_count, 0);
        assert_eq!(result.dismissed_count, 0);
        assert_eq!(result.superseded_count, 0);
        assert_eq!(result.pending_count, 0);
        assert!(result.current_substance_cid.is_none());
        assert!(result.challenges_by_grounds.is_empty());
        assert!(result.outcomes_by_verdict.is_empty());
    }

    // -------------------------------------------------------------------------
    // Test 2: Decisions without challenges -> challenged/upheld/dismissed all zero
    // -------------------------------------------------------------------------
    #[test]
    fn decisions_without_challenges_gives_zero_challenge_counts() {
        let mut conn = setup_db();
        insert_decision(
            &mut conn,
            "bafyDEC1",
            "elohim-active",
            "uhCAkELOHIM",
            "bafySUB1",
            "2026-04-10T12:00:00Z",
        );
        insert_decision(
            &mut conn,
            "bafyDEC2",
            "elohim-active",
            "uhCAkELOHIM",
            "bafySUB2",
            "2026-04-15T12:00:00Z",
        );

        let result = compute(&mut conn, &default_query()).unwrap();

        assert_eq!(result.total_decisions, 2);
        assert_eq!(result.challenged_count, 0);
        assert_eq!(result.upheld_count, 0);
        assert_eq!(result.dismissed_count, 0);
        assert_eq!(result.superseded_count, 0);
        assert_eq!(result.pending_count, 0);
        // Most-recent substance should be from bafyDEC2 (decided_at DESC)
        assert_eq!(result.current_substance_cid.as_deref(), Some("bafySUB2"));
    }

    // -------------------------------------------------------------------------
    // Test 3: One upheld challenge -> all three categories present
    // -------------------------------------------------------------------------
    #[test]
    fn one_upheld_challenge_populates_all_counts() {
        let mut conn = setup_db();
        insert_decision(
            &mut conn,
            "bafyDEC1",
            "elohim-active",
            "uhCAkELOHIM",
            "bafySUB1",
            "2026-04-10T12:00:00Z",
        );
        insert_challenge(&mut conn, "bafyCHAL1", "bafyDEC1", "safety");
        insert_outcome(&mut conn, "bafyOUT1", "bafyCHAL1", "upheld");

        let result = compute(&mut conn, &default_query()).unwrap();

        assert_eq!(result.total_decisions, 1);
        assert_eq!(result.challenged_count, 1);
        assert_eq!(result.upheld_count, 1);
        assert_eq!(result.dismissed_count, 0);
        assert_eq!(result.superseded_count, 0);
        assert_eq!(result.pending_count, 0);
        assert_eq!(result.challenges_by_grounds.get("safety").copied(), Some(1));
        assert_eq!(result.outcomes_by_verdict.get("upheld").copied(), Some(1));
    }

    // -------------------------------------------------------------------------
    // Test 4: Multiple challenges with different grounds -> histogram buckets correctly
    // -------------------------------------------------------------------------
    #[test]
    fn multiple_challenges_different_grounds_histogram_correct() {
        let mut conn = setup_db();
        insert_decision(
            &mut conn,
            "bafyDEC1",
            "elohim-active",
            "uhCAkELOHIM",
            "bafySUB1",
            "2026-04-10T12:00:00Z",
        );
        insert_decision(
            &mut conn,
            "bafyDEC2",
            "elohim-active",
            "uhCAkELOHIM",
            "bafySUB2",
            "2026-04-11T12:00:00Z",
        );
        insert_challenge(&mut conn, "bafyCHAL1", "bafyDEC1", "safety");
        insert_challenge(&mut conn, "bafyCHAL2", "bafyDEC1", "policy");
        insert_challenge(&mut conn, "bafyCHAL3", "bafyDEC2", "safety");
        insert_outcome(&mut conn, "bafyOUT1", "bafyCHAL1", "upheld");
        insert_outcome(&mut conn, "bafyOUT2", "bafyCHAL2", "dismissed");
        // bafyCHAL3 has no outcome -> pending

        let result = compute(&mut conn, &default_query()).unwrap();

        assert_eq!(result.total_decisions, 2);
        assert_eq!(result.challenged_count, 2);
        assert_eq!(result.upheld_count, 1);
        assert_eq!(result.dismissed_count, 1);
        assert_eq!(result.pending_count, 1);
        assert_eq!(result.challenges_by_grounds.get("safety").copied(), Some(2));
        assert_eq!(result.challenges_by_grounds.get("policy").copied(), Some(1));
        assert_eq!(result.outcomes_by_verdict.get("upheld").copied(), Some(1));
        assert_eq!(result.outcomes_by_verdict.get("dismissed").copied(), Some(1));
    }

    // -------------------------------------------------------------------------
    // Test 5: Phase filter — dev-context decisions excluded from total_decisions
    // -------------------------------------------------------------------------
    #[test]
    fn dev_context_decisions_excluded_from_total() {
        let mut conn = setup_db();
        insert_decision(
            &mut conn,
            "bafyDEV1",
            "dev-context",
            "uhCAkELOHIM",
            "bafySUBDEV",
            "2026-04-10T12:00:00Z",
        );
        insert_decision(
            &mut conn,
            "bafyACTIVE1",
            "elohim-active",
            "uhCAkELOHIM",
            "bafySUBACT",
            "2026-04-11T12:00:00Z",
        );

        let result = compute(&mut conn, &default_query()).unwrap();

        assert_eq!(result.total_decisions, 1);
        assert_eq!(
            result.current_substance_cid.as_deref(),
            Some("bafySUBACT")
        );
    }

    // -------------------------------------------------------------------------
    // Test 6: Window filter — decisions outside window excluded
    // -------------------------------------------------------------------------
    #[test]
    fn decisions_outside_window_excluded() {
        let mut conn = setup_db();
        insert_decision(
            &mut conn,
            "bafyBEFORE",
            "elohim-active",
            "uhCAkELOHIM",
            "bafySUB_OLD",
            "2025-12-31T23:59:59Z",
        );
        insert_decision(
            &mut conn,
            "bafyINWINDOW",
            "elohim-active",
            "uhCAkELOHIM",
            "bafySUB_NEW",
            "2026-04-10T12:00:00Z",
        );
        insert_decision(
            &mut conn,
            "bafyAFTER",
            "elohim-active",
            "uhCAkELOHIM",
            "bafySUB_FUTURE",
            "2027-01-01T00:00:00Z",
        );

        let result = compute(&mut conn, &default_query()).unwrap();

        assert_eq!(result.total_decisions, 1);
        assert_eq!(
            result.current_substance_cid.as_deref(),
            Some("bafySUB_NEW")
        );
    }
}
