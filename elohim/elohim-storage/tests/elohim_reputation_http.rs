//! End-to-end reputation aggregation tests (Task 11.3, Phase 11).
//!
//! These tests exercise the full reputation computation pipeline:
//!   `db::elohim_reputation::compute` -> `ElohimReputationProfileView::from_result`
//!
//! They use in-memory SQLite with realistic seeded rows across the three
//! tables that constitute the outcome graph:
//!   - gate_decision_attestations
//!   - gate_decision_challenges
//!   - challenge_outcomes
//!
//! ## Why no live HTTP server?
//!
//! Standing up the full `ContentServer` requires a conductor connection and a
//! Diesel pool. The route handler (`handle_elohim_reputation`) is thin: it
//! parses query params, calls `compute`, and calls `from_result`. Both of
//! those functions are tested directly here. The HTTP layer is covered by
//! the pattern established in `gate_decisions_http.rs`.
//!
//! ## What these tests cover
//!
//! 1. Fresh elohim — no decisions — all zeros, currentSubstanceCid is null.
//! 2. Decisions without challenges — totalDecisions > 0, challengedCount = 0.
//! 3. One upheld challenge — all three verdict buckets exercised.
//! 4. Multiple grounds — histogram buckets correctly.
//! 5. Phase filter — dev-context decisions excluded.
//! 6. Window filter — decisions outside window excluded.
//! 7. from_result conversion — camelCase JSON shape matches ElohimReputationProfileView.
//! 8. Pending count — challenge with no outcome counted as pending.

use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use elohim_storage::db::elohim_reputation::{compute, ReputationQuery};
use elohim_storage::ElohimReputationProfileView;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn setup_db() -> SqliteConnection {
    let mut conn = SqliteConnection::establish(":memory:").expect("in-memory SQLite");

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
    .bind::<diesel::sql_types::Text, _>("discernment-gate-v1")
    .bind::<diesel::sql_types::Text, _>("bafyPROC")
    .bind::<diesel::sql_types::Text, _>(r#"{"eventId":"ev-001"}"#)
    .bind::<diesel::sql_types::Text, _>("allow")
    .bind::<diesel::sql_types::Text, _>(r#"{"steps":[]}"#)
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
    .bind::<diesel::sql_types::Text, _>("Evidence-backed challenge")
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
    .bind::<diesel::sql_types::Text, _>("uhCAkREVIEWER1,uhCAkREVIEWER2")
    .bind::<diesel::sql_types::Text, _>(r#"{"summary":"reviewed"}"#)
    .bind::<diesel::sql_types::Text, _>("2026-04-20T10:00:00Z")
    .bind::<diesel::sql_types::Text, _>("[]")
    .bind::<diesel::sql_types::Text, _>("uhCkkOUT")
    .execute(conn)
    .expect("insert outcome");
}

const ELOHIM: &str = "uhCAkELOHIM01234567890123456789012345678901234567890123456789";
const WIN_START: &str = "2026-01-01T00:00:00Z";
const WIN_END: &str = "2026-12-31T23:59:59Z";

fn default_query() -> ReputationQuery {
    ReputationQuery {
        app_id: "test-app".into(),
        elohim_id: ELOHIM.into(),
        window_start: WIN_START.into(),
        window_end: WIN_END.into(),
    }
}

/// Convert a `ReputationResult` to a `ElohimReputationProfileView` the same way
/// the HTTP handler does.
fn to_view(conn: &mut SqliteConnection) -> ElohimReputationProfileView {
    let result = compute(conn, &default_query()).expect("compute must succeed");
    ElohimReputationProfileView::from_result(
        ELOHIM.to_string(),
        WIN_START.to_string(),
        WIN_END.to_string(),
        result,
    )
}

// ---------------------------------------------------------------------------
// Test 1: Fresh elohim — no decisions in window
// ---------------------------------------------------------------------------

#[test]
fn fresh_elohim_all_zeros_no_substance() {
    let mut conn = setup_db();
    let view = to_view(&mut conn);

    assert_eq!(view.elohim_id, ELOHIM);
    assert_eq!(view.window_start, WIN_START);
    assert_eq!(view.window_end, WIN_END);
    assert!(view.current_substance_cid.is_none());
    assert_eq!(view.total_decisions, 0);
    assert_eq!(view.challenged_count, 0);
    assert_eq!(view.upheld_count, 0);
    assert_eq!(view.dismissed_count, 0);
    assert_eq!(view.superseded_count, 0);
    assert_eq!(view.pending_count, 0);

    // Empty histograms should be JSON objects (not null/array)
    assert!(view.challenges_by_grounds.0.is_object());
    assert!(view.outcomes_by_verdict.0.is_object());
    assert_eq!(view.challenges_by_grounds.0.as_object().unwrap().len(), 0);
    assert_eq!(view.outcomes_by_verdict.0.as_object().unwrap().len(), 0);
}

// ---------------------------------------------------------------------------
// Test 2: Decisions without challenges
// ---------------------------------------------------------------------------

#[test]
fn decisions_without_challenges_zero_challenge_counts() {
    let mut conn = setup_db();
    insert_decision(
        &mut conn,
        "bafyDEC1",
        "elohim-active",
        ELOHIM,
        "bafySUB1",
        "2026-03-01T12:00:00Z",
    );
    insert_decision(
        &mut conn,
        "bafyDEC2",
        "elohim-active",
        ELOHIM,
        "bafySUB2",
        "2026-04-01T12:00:00Z",
    );

    let view = to_view(&mut conn);

    assert_eq!(view.total_decisions, 2);
    assert_eq!(view.challenged_count, 0);
    assert_eq!(view.upheld_count, 0);
    assert_eq!(view.dismissed_count, 0);
    assert_eq!(view.pending_count, 0);
    // Most recent substance is from bafyDEC2 (latest decided_at)
    assert_eq!(view.current_substance_cid.as_deref(), Some("bafySUB2"));
}

// ---------------------------------------------------------------------------
// Test 3: One upheld challenge — all verdict buckets populated
// ---------------------------------------------------------------------------

#[test]
fn one_upheld_challenge_all_verdict_buckets_populated() {
    let mut conn = setup_db();
    insert_decision(
        &mut conn,
        "bafyDEC1",
        "elohim-active",
        ELOHIM,
        "bafySUB1",
        "2026-03-15T12:00:00Z",
    );
    insert_challenge(&mut conn, "bafyCHAL1", "bafyDEC1", "safety");
    insert_outcome(&mut conn, "bafyOUT1", "bafyCHAL1", "upheld");

    let view = to_view(&mut conn);

    assert_eq!(view.total_decisions, 1);
    assert_eq!(view.challenged_count, 1);
    assert_eq!(view.upheld_count, 1);
    assert_eq!(view.dismissed_count, 0);
    assert_eq!(view.superseded_count, 0);
    assert_eq!(view.pending_count, 0);

    let grounds = view.challenges_by_grounds.0.as_object().unwrap();
    assert_eq!(grounds.get("safety").and_then(|v| v.as_i64()), Some(1));

    let verdicts = view.outcomes_by_verdict.0.as_object().unwrap();
    assert_eq!(verdicts.get("upheld").and_then(|v| v.as_i64()), Some(1));
}

// ---------------------------------------------------------------------------
// Test 4: Multiple grounds — histogram buckets correctly
// ---------------------------------------------------------------------------

#[test]
fn multiple_grounds_histogram_buckets_correctly() {
    let mut conn = setup_db();
    insert_decision(
        &mut conn,
        "bafyDEC1",
        "elohim-active",
        ELOHIM,
        "bafySUB1",
        "2026-03-10T12:00:00Z",
    );
    insert_decision(
        &mut conn,
        "bafyDEC2",
        "elohim-active",
        ELOHIM,
        "bafySUB2",
        "2026-03-11T12:00:00Z",
    );
    insert_challenge(&mut conn, "bafyCHAL1", "bafyDEC1", "safety");
    insert_challenge(&mut conn, "bafyCHAL2", "bafyDEC1", "constitutional");
    insert_challenge(&mut conn, "bafyCHAL3", "bafyDEC2", "safety");
    insert_outcome(&mut conn, "bafyOUT1", "bafyCHAL1", "upheld");
    insert_outcome(&mut conn, "bafyOUT2", "bafyCHAL2", "dismissed");
    insert_outcome(&mut conn, "bafyOUT3", "bafyCHAL3", "superseded");

    let view = to_view(&mut conn);

    assert_eq!(view.total_decisions, 2);
    assert_eq!(view.challenged_count, 2);
    assert_eq!(view.upheld_count, 1);
    assert_eq!(view.dismissed_count, 1);
    assert_eq!(view.superseded_count, 1);
    assert_eq!(view.pending_count, 0);

    let grounds = view.challenges_by_grounds.0.as_object().unwrap();
    assert_eq!(grounds.get("safety").and_then(|v| v.as_i64()), Some(2));
    assert_eq!(
        grounds.get("constitutional").and_then(|v| v.as_i64()),
        Some(1)
    );

    let verdicts = view.outcomes_by_verdict.0.as_object().unwrap();
    assert_eq!(verdicts.get("upheld").and_then(|v| v.as_i64()), Some(1));
    assert_eq!(verdicts.get("dismissed").and_then(|v| v.as_i64()), Some(1));
    assert_eq!(verdicts.get("superseded").and_then(|v| v.as_i64()), Some(1));
}

// ---------------------------------------------------------------------------
// Test 5: Phase filter — dev-context decisions excluded
// ---------------------------------------------------------------------------

#[test]
fn dev_context_decisions_excluded_from_view() {
    let mut conn = setup_db();
    insert_decision(
        &mut conn,
        "bafyDEV1",
        "dev-context",
        ELOHIM,
        "bafySUBDEV",
        "2026-02-01T12:00:00Z",
    );
    insert_decision(
        &mut conn,
        "bafyACT1",
        "elohim-active",
        ELOHIM,
        "bafySUBACT",
        "2026-02-02T12:00:00Z",
    );

    let view = to_view(&mut conn);

    assert_eq!(
        view.total_decisions, 1,
        "dev-context decision must not contribute to totalDecisions"
    );
    assert_eq!(
        view.current_substance_cid.as_deref(),
        Some("bafySUBACT"),
        "currentSubstanceCid must come from elohim-active decision"
    );
}

// ---------------------------------------------------------------------------
// Test 6: Window filter — decisions outside window excluded
// ---------------------------------------------------------------------------

#[test]
fn decisions_outside_window_excluded_from_view() {
    let mut conn = setup_db();
    // Before the window
    insert_decision(
        &mut conn,
        "bafyOLD",
        "elohim-active",
        ELOHIM,
        "bafySUBOLD",
        "2025-12-31T23:59:59Z",
    );
    // Inside the window
    insert_decision(
        &mut conn,
        "bafyNEW",
        "elohim-active",
        ELOHIM,
        "bafySUBNEW",
        "2026-06-15T12:00:00Z",
    );
    // After the window
    insert_decision(
        &mut conn,
        "bafyFUTURE",
        "elohim-active",
        ELOHIM,
        "bafySUBFUT",
        "2027-01-01T00:00:00Z",
    );

    let view = to_view(&mut conn);

    assert_eq!(
        view.total_decisions, 1,
        "only the in-window decision should count"
    );
    assert_eq!(view.current_substance_cid.as_deref(), Some("bafySUBNEW"));
}

// ---------------------------------------------------------------------------
// Test 7: Pending count — challenge without outcome is pending
// ---------------------------------------------------------------------------

#[test]
fn challenge_without_outcome_counted_as_pending() {
    let mut conn = setup_db();
    insert_decision(
        &mut conn,
        "bafyDEC1",
        "elohim-active",
        ELOHIM,
        "bafySUB1",
        "2026-03-01T12:00:00Z",
    );
    insert_challenge(&mut conn, "bafyCHAL1", "bafyDEC1", "policy");
    // No outcome inserted — challenge is pending

    let view = to_view(&mut conn);

    assert_eq!(view.challenged_count, 1);
    assert_eq!(view.pending_count, 1);
    assert_eq!(view.upheld_count, 0);
    assert_eq!(view.dismissed_count, 0);
}

// ---------------------------------------------------------------------------
// Test 8: Serialized JSON shape — verify camelCase field names on the wire
// ---------------------------------------------------------------------------

#[test]
fn view_serializes_to_camel_case_json() {
    let mut conn = setup_db();
    insert_decision(
        &mut conn,
        "bafyDEC1",
        "elohim-active",
        ELOHIM,
        "bafySUB1",
        "2026-03-01T12:00:00Z",
    );

    let view = to_view(&mut conn);
    let json = serde_json::to_value(&view).expect("serialize");

    // Check camelCase field names on the wire
    assert!(json.get("elohimId").is_some(), "elohimId missing");
    assert!(json.get("windowStart").is_some(), "windowStart missing");
    assert!(json.get("windowEnd").is_some(), "windowEnd missing");
    assert!(
        json.get("currentSubstanceCid").is_some(),
        "currentSubstanceCid missing"
    );
    assert!(
        json.get("totalDecisions").is_some(),
        "totalDecisions missing"
    );
    assert!(
        json.get("challengedCount").is_some(),
        "challengedCount missing"
    );
    assert!(json.get("upheldCount").is_some(), "upheldCount missing");
    assert!(
        json.get("dismissedCount").is_some(),
        "dismissedCount missing"
    );
    assert!(
        json.get("supersededCount").is_some(),
        "supersededCount missing"
    );
    assert!(json.get("pendingCount").is_some(), "pendingCount missing");
    assert!(
        json.get("challengesByGrounds").is_some(),
        "challengesByGrounds missing"
    );
    assert!(
        json.get("outcomesByVerdict").is_some(),
        "outcomesByVerdict missing"
    );

    // No snake_case leaked
    assert!(json.get("elohim_id").is_none(), "snake_case must not leak");
    assert!(
        json.get("total_decisions").is_none(),
        "snake_case must not leak"
    );
}
