//! HTTP handler contract tests for `/db/gate-decisions[/{cid}]` (Phase-2a).
//!
//! These tests exercise the query functions and view conversion that back the two
//! HTTP handlers (`handle_gate_decisions_list`, `handle_gate_decision_by_id`).
//! They do not spin up a full HTTP server — see the note below.
//!
//! ## Phase-2a consolidation
//!
//! Gate decisions were migrated off the dropped per-app gate-decision projection
//! table onto the unified `attestations` table as `attestation:gate-decision`
//! (mirroring the elohim-DNA bridge `create_gate_decision_attestation`). The
//! structured fields (phase, gate_name, decided_at, …) live in `evidence_json`;
//! `reasoning_json` lives under `proof_evidence_json` (proof_class=audit). The
//! unified table is CID-global — there is no app_id scope anymore. The query
//! layer (`db::attestations::gate_decision_*`) is also unit-tested in
//! `src/db/attestations.rs` and `src/signals.rs`.
//!
//! ## Testing strategy
//!
//! 1. **by_id hit** — gate_decision_find_by_id returns a row; view conversion correct.
//! 2. **by_id miss** — gate_decision_find_by_id returns None; handler would 404.
//! 3. **list by_elohim** — filter by issuer_cid (the deciding agent).
//! 4. **list by_gate** — filter by gate_name (json_extract from evidence_json).
//! 5. **list by_phase** — filter by phase (json_extract from evidence_json).
//! 6. **list unfiltered** — returns all rows up to limit, ordered by created_at desc.
//! 7. **limit clamping** — raw_limit clamps at 500.
//! 8. **view field mapping** — all fields of GateDecisionAttestationView round-trip.
//! 9. **kind isolation** — a non-gate-decision attestation is never returned.
//!
//! ## Why no live HTTP test?
//!
//! No SweetConductor-based harness exists in this crate's `tests/` directory (see
//! `peer_status_e2e.rs` for the precedent explanation). Standing up the full
//! `ContentServer` requires a conductor connection and a Diesel pool — out of
//! scope for `cargo test`.

use diesel::connection::SimpleConnection;
use diesel::sqlite::SqliteConnection;
use diesel::Connection;

use elohim_storage::db::attestations::{
    attestation_row_from_gate_decision, gate_decision_find_by_elohim, gate_decision_find_by_gate,
    gate_decision_find_by_id, gate_decision_find_by_phase, gate_decision_list_all, upsert,
};
use elohim_storage::db::models::AttestationRow;
use elohim_storage::signals::GateDecisionAttestationEntry;
use elohim_storage::views_convert::qahal::gate_decision_view_from_attestation;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn setup_db() -> SqliteConnection {
    let mut conn =
        SqliteConnection::establish(":memory:").expect("Failed to create in-memory SQLite");

    conn.batch_execute(
        r#"
        CREATE TABLE attestations (
            id TEXT PRIMARY KEY,
            dht_anchor_hash BLOB NOT NULL,
            attestation_kind TEXT NOT NULL,
            subject_cid TEXT NOT NULL,
            subject_kind TEXT NOT NULL,
            issuer_cid TEXT NOT NULL,
            parent_governance_action_cid TEXT,
            vote_value TEXT,
            vote_weight TEXT,
            proof_class TEXT NOT NULL,
            proof_evidence_json TEXT NOT NULL,
            evidence_json TEXT NOT NULL,
            expires_at TEXT,
            supersedes_cid TEXT,
            revocation_reason TEXT,
            revoked_at TEXT,
            created_at TEXT NOT NULL,
            manifest_ref TEXT NOT NULL,
            title TEXT NOT NULL,
            description TEXT
        );
        CREATE INDEX attestations_kind ON attestations(attestation_kind);
        CREATE INDEX attestations_issuer ON attestations(issuer_cid);
        "#,
    )
    .expect("Failed to create test schema");

    conn
}

/// Build a gate-decision `AttestationRow` via the production projection helper,
/// so the test exercises the same mapping the live writer uses.
fn make_row(
    decision_id: &str,
    gate: &str,
    phase: &str,
    elohim_id: &str,
    decision: &str,
    decided_at: &str,
) -> AttestationRow {
    let entry = GateDecisionAttestationEntry {
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
    };
    attestation_row_from_gate_decision(&entry, "uhCkkDEF")
}

// ---------------------------------------------------------------------------
// Test 1 — by_id hit: gate_decision_find_by_id returns a row; view correct
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

    let found = gate_decision_find_by_id(&mut conn, "bafyDEC1")
        .unwrap()
        .expect("row must be present");

    // Simulate what the handler does: row → view
    let view = gate_decision_view_from_attestation(found);

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
// Test 2 — by_id miss: gate_decision_find_by_id returns None (handler 404s)
// ---------------------------------------------------------------------------

#[test]
fn by_id_miss_returns_none() {
    let mut conn = setup_db();
    let result = gate_decision_find_by_id(&mut conn, "bafyNONEXISTENT").unwrap();
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

    let rows = gate_decision_find_by_elohim(&mut conn, "uhCAkABC").unwrap();
    assert_eq!(
        rows.len(),
        1,
        "only the matching agent's row should be returned"
    );
    assert_eq!(rows[0].id, "bafyDEC1");
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

    let rows = gate_decision_find_by_gate(&mut conn, "discernment-gate-v1").unwrap();
    assert_eq!(
        rows.len(),
        1,
        "only rows for the named gate should be returned"
    );
    assert_eq!(rows[0].id, "bafyDEC1");
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

    let dev_rows = gate_decision_find_by_phase(&mut conn, "dev-context").unwrap();
    assert_eq!(dev_rows.len(), 1);
    assert_eq!(dev_rows[0].id, "bafyDEC1");

    let active_rows = gate_decision_find_by_phase(&mut conn, "elohim-active").unwrap();
    assert_eq!(active_rows.len(), 1);
    assert_eq!(active_rows[0].id, "bafyDEC2");
}

// ---------------------------------------------------------------------------
// Test 6 — unfiltered list: all rows returned ordered by created_at desc
// ---------------------------------------------------------------------------

#[test]
fn list_unfiltered_returns_all_ordered_desc() {
    let mut conn = setup_db();

    // created_at = decided_at in the projection, so timestamp ordering holds.
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

    let rows = gate_decision_list_all(&mut conn, 50).unwrap();

    assert_eq!(rows.len(), 3);
    // Verify descending order by created_at (= decided_at).
    assert_eq!(rows[0].created_at, "2026-04-18T10:00:00Z");
    assert_eq!(rows[1].created_at, "2026-04-18T09:00:00Z");
    assert_eq!(rows[2].created_at, "2026-04-18T08:00:00Z");
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
// Test 8 — view field completeness: all public fields round-trip
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

    let view = gate_decision_view_from_attestation(row);

    // Verify every field round-trips through evidence_json / proof_evidence_json.
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
    // created_at = decided_at in the projection.
    assert_eq!(view.created_at, "2026-04-18T12:30:00Z");
}

// ---------------------------------------------------------------------------
// Test 9 — kind isolation: a non-gate-decision attestation is never returned
//
// Replaces the old per-app_id isolation test: the unified table is CID-global,
// so the meaningful isolation guarantee is now by attestation_kind.
// ---------------------------------------------------------------------------

#[test]
fn non_gate_decision_attestations_are_excluded() {
    let mut conn = setup_db();

    // A real gate-decision row.
    upsert(
        &mut conn,
        &make_row(
            "bafyDEC1",
            "gate-a",
            "elohim-active",
            "uhCAkABC",
            "allow",
            "2026-04-18T12:00:00Z",
        ),
    )
    .unwrap();

    // A sibling attestation of a DIFFERENT kind, same issuer.
    let other = AttestationRow {
        id: "bafyOTHER".into(),
        dht_anchor_hash: b"uhCkkOTHER".to_vec(),
        attestation_kind: "attestation:humanness".into(),
        subject_cid: "uhCAkABC".into(),
        subject_kind: "agent".into(),
        issuer_cid: "uhCAkABC".into(),
        parent_governance_action_cid: None,
        vote_value: None,
        vote_weight: None,
        proof_class: "witness".into(),
        proof_evidence_json: "{}".into(),
        evidence_json: r#"{"phase":"elohim-active","gate_name":"gate-a"}"#.into(),
        expires_at: None,
        supersedes_cid: None,
        revocation_reason: None,
        revoked_at: None,
        created_at: "2026-04-18T12:00:00Z".into(),
        manifest_ref: "imagodei".into(),
        title: "Humanness".into(),
        description: None,
    };
    upsert(&mut conn, &other).unwrap();

    // None of the gate-decision readers should surface the humanness row,
    // even though it shares issuer / phase / gate_name strings in evidence_json.
    let by_elohim = gate_decision_find_by_elohim(&mut conn, "uhCAkABC").unwrap();
    assert_eq!(by_elohim.len(), 1);
    assert_eq!(by_elohim[0].id, "bafyDEC1");

    let by_gate = gate_decision_find_by_gate(&mut conn, "gate-a").unwrap();
    assert_eq!(by_gate.len(), 1);
    assert_eq!(by_gate[0].id, "bafyDEC1");

    let by_phase = gate_decision_find_by_phase(&mut conn, "elohim-active").unwrap();
    assert_eq!(by_phase.len(), 1);
    assert_eq!(by_phase[0].id, "bafyDEC1");

    let all = gate_decision_list_all(&mut conn, 50).unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].id, "bafyDEC1");

    // The humanness row is not addressable as a gate decision by id.
    assert!(gate_decision_find_by_id(&mut conn, "bafyOTHER")
        .unwrap()
        .is_none());
}
