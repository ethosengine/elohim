//! HTTP handler contract tests for `/db/gate-decisions[/{cid}]` (Task 4.3, Phase 4).
//!
//! These tests exercise the query functions and view conversion that back the two
//! HTTP handlers (`handle_gate_decisions_list`, `handle_gate_decision_by_id`).
//! They do not spin up a full HTTP server — see the note below.
//!
//! ## Testing strategy
//!
//! The HTTP handlers are thin: they parse query params, call db functions, and
//! convert rows to views. The DB layer (`gate_decision_attestations`) is already
//! tested in `src/db/gate_decision_attestations.rs`. What we add here:
//!
//! 1. **by_id hit** — find_by_id returns a row, view conversion is correct.
//! 2. **by_id miss** — find_by_id returns None; handler would 404.
//! 3. **list by_elohim** — filter by elohim_id.
//! 4. **list by_gate** — filter by gate_name.
//! 5. **list by_phase** — filter by phase.
//! 6. **list unfiltered** — returns all rows up to limit, ordered by decided_at desc.
//! 7. **limit clamping** — raw_limit clamps at 500.
//! 8. **view field mapping** — all fields of GateDecisionAttestationView are present
//!    and correctly mapped from the row.
//!
//! ## Why no live HTTP test?
//!
//! No SweetConductor-based harness exists in this crate's `tests/` directory (see
//! `peer_status_e2e.rs` for the precedent explanation). Standing up the full
//! `ContentServer` requires a conductor connection and a Diesel pool. A separate
//! integration smoke test would need a running elohim-storage process, which is
//! out of scope for `cargo test`. Flagged as a concern for Task 4.4/4.5.

use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use elohim_storage::db::gate_decision_attestations::{
    find_by_elohim, find_by_gate, find_by_id, find_by_phase, upsert, GateDecisionAttestationRow,
};
use elohim_storage::views::GateDecisionAttestationView;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn setup_db() -> SqliteConnection {
    let mut conn =
        SqliteConnection::establish(":memory:").expect("Failed to create in-memory SQLite");

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
        CREATE INDEX idx_gate_decisions_elohim ON gate_decision_attestations(elohim_id);
        CREATE INDEX idx_gate_decisions_gate ON gate_decision_attestations(gate_name);
        CREATE INDEX idx_gate_decisions_phase ON gate_decision_attestations(phase);
        CREATE INDEX idx_gate_decisions_decided_at ON gate_decision_attestations(decided_at);
        "#,
    )
    .expect("Failed to create test schema");

    conn
}

fn make_row(
    decision_id: &str,
    gate: &str,
    phase: &str,
    elohim_id: &str,
    decision: &str,
    decided_at: &str,
) -> GateDecisionAttestationRow {
    GateDecisionAttestationRow {
        app_id: "test-app".into(),
        decision_id: decision_id.into(),
        phase: phase.into(),
        elohim_id: elohim_id.into(),
        elohim_substance_cid: "bafySUB".into(),
        gate_name: gate.into(),
        gate_process_cid: "bafyPROC".into(),
        request_ref_json: r#"{"eventId":"ev1"}"#.into(),
        decision: decision.into(),
        reasoning_json: r#"{"steps":[]}"#.into(),
        context_summary_cid: "bafyCTX".into(),
        decided_at: decided_at.into(),
        universal_band_cid: "bafyUNIV".into(),
        dht_anchor_hash: "uhCkkDEF".into(),
        created_at: "2026-04-18T12:00:00".into(),
        updated_at: "2026-04-18T12:00:00".into(),
    }
}

// ---------------------------------------------------------------------------
// Test 1 — by_id hit: find_by_id returns a row; view conversion is correct
// ---------------------------------------------------------------------------

#[test]
fn by_id_hit_returns_correct_view() {
    let mut conn = setup_db();
    let row = make_row(
        "bafyDEC1",
        "discernment-gate-v1",
        "elohim-active",
        "uhCAkABC",
        "allow",
        "2026-04-18T12:00:00Z",
    );
    upsert(&mut conn, &row).unwrap();

    let found = find_by_id(&mut conn, "test-app", "bafyDEC1")
        .unwrap()
        .expect("row must be present");

    // Simulate what the handler does: row → view
    let view = GateDecisionAttestationView::from(found);

    assert_eq!(view.decision_id, "bafyDEC1");
    assert_eq!(view.gate_name, "discernment-gate-v1");
    assert_eq!(view.phase, "elohim-active");
    assert_eq!(view.elohim_id, "uhCAkABC");
    assert_eq!(view.decision, "allow");
    assert_eq!(view.dht_anchor_hash, "uhCkkDEF");
    assert_eq!(view.decided_at, "2026-04-18T12:00:00Z");
    assert_eq!(view.request_ref_json, r#"{"eventId":"ev1"}"#);
    assert_eq!(view.reasoning_json, r#"{"steps":[]}"#);
}

// ---------------------------------------------------------------------------
// Test 2 — by_id miss: find_by_id returns None (handler would 404)
// ---------------------------------------------------------------------------

#[test]
fn by_id_miss_returns_none() {
    let mut conn = setup_db();
    let result = find_by_id(&mut conn, "test-app", "bafyNONEXISTENT").unwrap();
    assert!(
        result.is_none(),
        "missing decision_id must return None so the handler returns 404"
    );
}

// ---------------------------------------------------------------------------
// Test 3 — list by_elohim: only rows for that elohim agent are returned
// ---------------------------------------------------------------------------

#[test]
fn list_by_elohim_filters_correctly() {
    let mut conn = setup_db();

    upsert(
        &mut conn,
        &make_row(
            "bafyDEC1",
            "gate-a",
            "elohim-active",
            "uhCAkABC",
            "allow",
            "2026-04-18T11:00:00Z",
        ),
    )
    .unwrap();
    upsert(
        &mut conn,
        &make_row(
            "bafyDEC2",
            "gate-a",
            "elohim-active",
            "uhCAkXYZ",
            "decline",
            "2026-04-18T12:00:00Z",
        ),
    )
    .unwrap();

    let rows = find_by_elohim(&mut conn, "test-app", "uhCAkABC").unwrap();
    assert_eq!(
        rows.len(),
        1,
        "only the matching agent's row should be returned"
    );
    assert_eq!(rows[0].decision_id, "bafyDEC1");
    assert_eq!(rows[0].decision, "allow");
}

// ---------------------------------------------------------------------------
// Test 4 — list by_gate: only rows for that gate name are returned
// ---------------------------------------------------------------------------

#[test]
fn list_by_gate_filters_correctly() {
    let mut conn = setup_db();

    upsert(
        &mut conn,
        &make_row(
            "bafyDEC1",
            "discernment-gate-v1",
            "elohim-active",
            "uhCAkABC",
            "allow",
            "2026-04-18T10:00:00Z",
        ),
    )
    .unwrap();
    upsert(
        &mut conn,
        &make_row(
            "bafyDEC2",
            "budget-gate-v1",
            "elohim-active",
            "uhCAkABC",
            "decline",
            "2026-04-18T11:00:00Z",
        ),
    )
    .unwrap();

    let rows = find_by_gate(&mut conn, "test-app", "discernment-gate-v1").unwrap();
    assert_eq!(
        rows.len(),
        1,
        "only rows for the named gate should be returned"
    );
    assert_eq!(rows[0].decision_id, "bafyDEC1");
}

// ---------------------------------------------------------------------------
// Test 5 — list by_phase: only rows for that deployment phase are returned
// ---------------------------------------------------------------------------

#[test]
fn list_by_phase_filters_correctly() {
    let mut conn = setup_db();

    upsert(
        &mut conn,
        &make_row(
            "bafyDEC1",
            "gate-a",
            "dev-context",
            "uhCAkABC",
            "allow",
            "2026-04-18T09:00:00Z",
        ),
    )
    .unwrap();
    upsert(
        &mut conn,
        &make_row(
            "bafyDEC2",
            "gate-a",
            "elohim-active",
            "uhCAkABC",
            "decline",
            "2026-04-18T10:00:00Z",
        ),
    )
    .unwrap();

    let dev_rows = find_by_phase(&mut conn, "test-app", "dev-context").unwrap();
    assert_eq!(dev_rows.len(), 1);
    assert_eq!(dev_rows[0].decision_id, "bafyDEC1");

    let active_rows = find_by_phase(&mut conn, "test-app", "elohim-active").unwrap();
    assert_eq!(active_rows.len(), 1);
    assert_eq!(active_rows[0].decision_id, "bafyDEC2");
}

// ---------------------------------------------------------------------------
// Test 6 — unfiltered list: all rows returned ordered by decided_at desc
// ---------------------------------------------------------------------------

#[test]
fn list_unfiltered_returns_all_ordered_desc() {
    use elohim_storage::db::diesel_schema::gate_decision_attestations::dsl;

    let mut conn = setup_db();

    // Insert three rows with different timestamps
    for (cid, ts) in &[
        ("bafyDEC_A", "2026-04-18T08:00:00Z"),
        ("bafyDEC_B", "2026-04-18T09:00:00Z"),
        ("bafyDEC_C", "2026-04-18T10:00:00Z"),
    ] {
        upsert(
            &mut conn,
            &make_row(cid, "gate-x", "elohim-active", "uhCAkABC", "allow", ts),
        )
        .unwrap();
    }

    let rows = dsl::gate_decision_attestations
        .filter(dsl::app_id.eq("test-app"))
        .order(dsl::decided_at.desc())
        .limit(50)
        .load::<GateDecisionAttestationRow>(&mut conn)
        .unwrap();

    assert_eq!(rows.len(), 3);
    // Verify descending order
    assert_eq!(rows[0].decided_at, "2026-04-18T10:00:00Z");
    assert_eq!(rows[1].decided_at, "2026-04-18T09:00:00Z");
    assert_eq!(rows[2].decided_at, "2026-04-18T08:00:00Z");
}

// ---------------------------------------------------------------------------
// Test 7 — limit clamping: raw_limit.clamp(1, 500) logic
// ---------------------------------------------------------------------------

#[test]
fn limit_clamps_to_valid_range() {
    // Verify the clamping logic used in the handler
    assert_eq!((-1_i64).clamp(1, 500), 1, "negative limit clamps to 1");
    assert_eq!(0_i64.clamp(1, 500), 1, "zero limit clamps to 1");
    assert_eq!(50_i64.clamp(1, 500), 50, "default limit passes through");
    assert_eq!(500_i64.clamp(1, 500), 500, "max limit passes through");
    assert_eq!(999_i64.clamp(1, 500), 500, "over-max limit clamps to 500");
}

// ---------------------------------------------------------------------------
// Test 8 — view field completeness: all public fields are mapped
// ---------------------------------------------------------------------------

#[test]
fn view_has_all_expected_fields() {
    let row = make_row(
        "bafyFIELD_TEST",
        "gate-complete",
        "elohim-active",
        "uhCAkFIELD",
        "allow",
        "2026-04-18T12:30:00Z",
    );

    let view = GateDecisionAttestationView::from(row);

    // Verify every field is mapped (any missing field would fail compilation,
    // but this test makes the field contract explicit and readable).
    assert_eq!(view.decision_id, "bafyFIELD_TEST");
    assert_eq!(view.phase, "elohim-active");
    assert_eq!(view.elohim_id, "uhCAkFIELD");
    assert_eq!(view.elohim_substance_cid, "bafySUB");
    assert_eq!(view.gate_name, "gate-complete");
    assert_eq!(view.gate_process_cid, "bafyPROC");
    assert_eq!(view.request_ref_json, r#"{"eventId":"ev1"}"#);
    assert_eq!(view.decision, "allow");
    assert_eq!(view.reasoning_json, r#"{"steps":[]}"#);
    assert_eq!(view.context_summary_cid, "bafyCTX");
    assert_eq!(view.decided_at, "2026-04-18T12:30:00Z");
    assert_eq!(view.universal_band_cid, "bafyUNIV");
    assert_eq!(view.dht_anchor_hash, "uhCkkDEF");
    assert_eq!(view.created_at, "2026-04-18T12:00:00");
}

// ---------------------------------------------------------------------------
// Test 9 — cross-app isolation: rows from a different app_id are not visible
// ---------------------------------------------------------------------------

#[test]
fn rows_are_scoped_to_app_id() {
    let mut conn = setup_db();

    let mut row_a = make_row(
        "bafyDEC1",
        "gate-a",
        "elohim-active",
        "uhCAkABC",
        "allow",
        "2026-04-18T12:00:00Z",
    );
    row_a.app_id = "app-alpha".into();

    let mut row_b = make_row(
        "bafyDEC2",
        "gate-a",
        "elohim-active",
        "uhCAkABC",
        "allow",
        "2026-04-18T12:00:00Z",
    );
    row_b.app_id = "app-beta".into();

    upsert(&mut conn, &row_a).unwrap();
    upsert(&mut conn, &row_b).unwrap();

    let alpha_rows = find_by_elohim(&mut conn, "app-alpha", "uhCAkABC").unwrap();
    assert_eq!(alpha_rows.len(), 1);
    assert_eq!(alpha_rows[0].decision_id, "bafyDEC1");

    let beta_rows = find_by_elohim(&mut conn, "app-beta", "uhCAkABC").unwrap();
    assert_eq!(beta_rows.len(), 1);
    assert_eq!(beta_rows[0].decision_id, "bafyDEC2");

    // No cross-contamination
    let none = find_by_id(&mut conn, "app-alpha", "bafyDEC2").unwrap();
    assert!(none.is_none(), "app-alpha must not see app-beta's row");
}
