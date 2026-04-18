//! Projection of the notarized GateDecisionAttestation DHT entry (mishpat DNA).
//!
//! Writes here come exclusively from the post-commit signal projection in
//! `src/signals.rs` (`MishpatSignal::GateDecisionCreated`). Source of truth
//! is the mishpat DNA DHT. If this projection and the DHT disagree, the DHT
//! wins — rebuild from `get_gate_decision_by_id` / query the mishpat DNA
//! directly rather than mutating this table.
//!
//! Column conventions:
//! - JSON blobs: TEXT (raw serialized), surfaced as `serde_json::Value` at the
//!   view boundary.
//! - ActionHashes / CIDs: TEXT (base64 / CID string). No binary blobs cross
//!   the wire; `dht_anchor_hash` is the same shape as the signal payload.

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use crate::db::diesel_schema::gate_decision_attestations;

// ---------------------------------------------------------------------------
// Row model
// ---------------------------------------------------------------------------

/// Row-shaped projection of a GateDecisionAttestation DHT entry.
///
/// All JSON fields are stored as raw TEXT and converted to `serde_json::Value`
/// at the view boundary — never in this module.
#[derive(Debug, Clone, Queryable, Insertable, AsChangeset, Identifiable)]
#[diesel(table_name = gate_decision_attestations, primary_key(app_id, decision_id))]
pub struct GateDecisionAttestationRow {
    pub app_id: String,
    pub decision_id: String,
    pub phase: String,
    pub elohim_id: String,
    pub elohim_substance_cid: String,
    pub gate_name: String,
    pub gate_process_cid: String,
    pub request_ref_json: String,
    pub decision: String,
    pub reasoning_json: String,
    pub context_summary_cid: String,
    pub decided_at: String,
    pub universal_band_cid: String,
    pub dht_anchor_hash: String,
    pub created_at: String,
    pub updated_at: String,
}

// ---------------------------------------------------------------------------
// Writes
// ---------------------------------------------------------------------------

/// Insert or replace a gate decision attestation row.
///
/// Uses `INSERT OR REPLACE` (equivalent to `ON CONFLICT DO UPDATE SET ALL`)
/// so that a re-delivered signal is idempotent. The primary key is
/// `(app_id, decision_id)` — `decision_id` is a CID so collisions indicate
/// a true duplicate, not a logical conflict.
pub fn upsert(conn: &mut SqliteConnection, row: &GateDecisionAttestationRow) -> QueryResult<usize> {
    diesel::insert_into(gate_decision_attestations::table)
        .values(row)
        .on_conflict((
            gate_decision_attestations::app_id,
            gate_decision_attestations::decision_id,
        ))
        .do_update()
        .set(row)
        .execute(conn)
}

// ---------------------------------------------------------------------------
// Reads
// ---------------------------------------------------------------------------

/// Look up a decision by its CID (decision_id) within an app scope.
pub fn find_by_id(
    conn: &mut SqliteConnection,
    app_id: &str,
    decision_id: &str,
) -> QueryResult<Option<GateDecisionAttestationRow>> {
    gate_decision_attestations::table
        .filter(gate_decision_attestations::app_id.eq(app_id))
        .filter(gate_decision_attestations::decision_id.eq(decision_id))
        .first::<GateDecisionAttestationRow>(conn)
        .optional()
}

/// List all decisions made by a given elohim agent within an app scope.
pub fn find_by_elohim(
    conn: &mut SqliteConnection,
    app_id: &str,
    elohim_id: &str,
) -> QueryResult<Vec<GateDecisionAttestationRow>> {
    gate_decision_attestations::table
        .filter(gate_decision_attestations::app_id.eq(app_id))
        .filter(gate_decision_attestations::elohim_id.eq(elohim_id))
        .order(gate_decision_attestations::decided_at.desc())
        .load::<GateDecisionAttestationRow>(conn)
}

/// List all decisions produced by a named gate within an app scope.
pub fn find_by_gate(
    conn: &mut SqliteConnection,
    app_id: &str,
    gate_name: &str,
) -> QueryResult<Vec<GateDecisionAttestationRow>> {
    gate_decision_attestations::table
        .filter(gate_decision_attestations::app_id.eq(app_id))
        .filter(gate_decision_attestations::gate_name.eq(gate_name))
        .order(gate_decision_attestations::decided_at.desc())
        .load::<GateDecisionAttestationRow>(conn)
}

/// List all decisions from a given deployment phase within an app scope.
pub fn find_by_phase(
    conn: &mut SqliteConnection,
    app_id: &str,
    phase: &str,
) -> QueryResult<Vec<GateDecisionAttestationRow>> {
    gate_decision_attestations::table
        .filter(gate_decision_attestations::app_id.eq(app_id))
        .filter(gate_decision_attestations::phase.eq(phase))
        .order(gate_decision_attestations::decided_at.desc())
        .load::<GateDecisionAttestationRow>(conn)
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
            CREATE INDEX idx_gate_decisions_manifest ON gate_decision_attestations(gate_process_cid);
            CREATE INDEX idx_gate_decisions_decided_at ON gate_decision_attestations(decided_at);
            "#,
        )
        .expect("Failed to create test table");

        conn
    }

    fn make_row(
        decision_id: &str,
        gate: &str,
        phase: &str,
        decision: &str,
    ) -> GateDecisionAttestationRow {
        GateDecisionAttestationRow {
            app_id: "test-app".into(),
            decision_id: decision_id.into(),
            phase: phase.into(),
            elohim_id: "uhCAkABC".into(),
            elohim_substance_cid: "bafyABC".into(),
            gate_name: gate.into(),
            gate_process_cid: "bafyPROC".into(),
            request_ref_json: r#"{"eventId":"ev1"}"#.into(),
            decision: decision.into(),
            reasoning_json: r#"{"steps":[]}"#.into(),
            context_summary_cid: "bafyCTX".into(),
            decided_at: "2026-04-18T12:00:00Z".into(),
            universal_band_cid: "bafyUNIV".into(),
            dht_anchor_hash: "uhCkkDEF".into(),
            created_at: "2026-04-18T12:00:00".into(),
            updated_at: "2026-04-18T12:00:00".into(),
        }
    }

    #[test]
    fn upsert_then_find_by_id() {
        let mut conn = setup_test_db();
        let row = make_row("bafyDEC1", "discernment-gate-v1", "elohim-active", "allow");
        upsert(&mut conn, &row).unwrap();

        let found = find_by_id(&mut conn, "test-app", "bafyDEC1")
            .unwrap()
            .unwrap();
        assert_eq!(found.decision, "allow");
        assert_eq!(found.gate_name, "discernment-gate-v1");
        assert_eq!(found.dht_anchor_hash, "uhCkkDEF");
    }

    #[test]
    fn upsert_is_idempotent() {
        let mut conn = setup_test_db();
        let row = make_row("bafyDEC1", "discernment-gate-v1", "elohim-active", "allow");
        upsert(&mut conn, &row).unwrap();
        // Re-deliver the same signal — should not fail or duplicate
        upsert(&mut conn, &row).unwrap();

        let rows = find_by_gate(&mut conn, "test-app", "discernment-gate-v1").unwrap();
        assert_eq!(
            rows.len(),
            1,
            "Re-delivered signal must not duplicate the row"
        );
    }

    #[test]
    fn find_by_elohim_returns_only_that_agents_decisions() {
        let mut conn = setup_test_db();
        let mut row_a = make_row("bafyDEC1", "discernment-gate-v1", "elohim-active", "allow");
        row_a.elohim_id = "uhCAkABC".into();
        let mut row_b = make_row(
            "bafyDEC2",
            "discernment-gate-v1",
            "elohim-active",
            "decline",
        );
        row_b.elohim_id = "uhCAkXYZ".into();

        upsert(&mut conn, &row_a).unwrap();
        upsert(&mut conn, &row_b).unwrap();

        let decisions = find_by_elohim(&mut conn, "test-app", "uhCAkABC").unwrap();
        assert_eq!(decisions.len(), 1);
        assert_eq!(decisions[0].decision_id, "bafyDEC1");
    }

    #[test]
    fn find_by_phase_filters_correctly() {
        let mut conn = setup_test_db();
        upsert(
            &mut conn,
            &make_row("bafyDEC1", "gate-a", "dev-context", "allow"),
        )
        .unwrap();
        upsert(
            &mut conn,
            &make_row("bafyDEC2", "gate-b", "elohim-active", "decline"),
        )
        .unwrap();

        let dev = find_by_phase(&mut conn, "test-app", "dev-context").unwrap();
        assert_eq!(dev.len(), 1);
        assert_eq!(dev[0].decision_id, "bafyDEC1");

        let active = find_by_phase(&mut conn, "test-app", "elohim-active").unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].decision_id, "bafyDEC2");
    }

    #[test]
    fn find_by_id_returns_none_for_missing() {
        let mut conn = setup_test_db();
        assert!(find_by_id(&mut conn, "test-app", "missing-cid")
            .unwrap()
            .is_none());
    }
}
