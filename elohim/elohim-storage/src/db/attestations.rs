//! CRUD + queries for the unified attestations projection table.
//! Source of truth: Holochain DHT (Content entries with content_type LIKE 'attestation:%').
//! This table is a read-optimised projection populated by post-commit signals;
//! treat rows as a cache — the DHT entry is authoritative.

use diesel::prelude::*;

use crate::db::diesel_schema::attestations;
use crate::db::models::AttestationRow;
use crate::error::StorageError;

/// Upsert an attestation projection row.
///
/// Idempotent: replaying the same signal produces the same row.
/// Conflict on `id` (CID — primary key) replaces all columns.
pub fn upsert(conn: &mut SqliteConnection, row: &AttestationRow) -> Result<(), StorageError> {
    diesel::insert_into(attestations::table)
        .values(row)
        .on_conflict(attestations::id)
        .do_update()
        .set(row)
        .execute(conn)
        .map(|_| ())
        .map_err(|e| StorageError::Internal(format!("Failed to upsert attestation: {e}")))
}

/// Fetch a single attestation by CID.
pub fn get_by_id(
    conn: &mut SqliteConnection,
    id: &str,
) -> Result<Option<AttestationRow>, StorageError> {
    attestations::table
        .filter(attestations::id.eq(id))
        .first::<AttestationRow>(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Failed to fetch attestation {id}: {e}")))
}

/// List attestations for a subject CID, optionally filtered by kind.
pub fn list_by_subject(
    conn: &mut SqliteConnection,
    subject_cid: &str,
    kind_filter: Option<&str>,
) -> Result<Vec<AttestationRow>, StorageError> {
    let mut q = attestations::table
        .filter(attestations::subject_cid.eq(subject_cid))
        .into_boxed();
    if let Some(kind) = kind_filter {
        q = q.filter(attestations::attestation_kind.eq(kind));
    }
    q.order_by(attestations::created_at.desc())
        .load::<AttestationRow>(conn)
        .map_err(|e| {
            StorageError::Internal(format!(
                "Failed to list attestations for subject {subject_cid}: {e}"
            ))
        })
}

/// List vote-children of a governance action (votes submitted against a parent CID).
pub fn list_by_parent_governance_action(
    conn: &mut SqliteConnection,
    parent_cid: &str,
) -> Result<Vec<AttestationRow>, StorageError> {
    attestations::table
        .filter(attestations::parent_governance_action_cid.eq(parent_cid))
        .order_by(attestations::created_at.asc())
        .load::<AttestationRow>(conn)
        .map_err(|e| {
            StorageError::Internal(format!(
                "Failed to list attestation children of {parent_cid}: {e}"
            ))
        })
}

/// Count vote-children of a governance action (parent CID).
///
/// Used by the recovery-flow projector to recompute `current_votes` after each
/// vote-child upsert. Because the underlying `attestations` row is keyed on
/// `id` (the vote's content CID), redelivering the same signal idempotently
/// replaces the same row — the count never double-counts.
pub fn count_by_parent_governance_action_cid(
    conn: &mut SqliteConnection,
    parent_cid: &str,
) -> Result<i64, StorageError> {
    attestations::table
        .filter(attestations::parent_governance_action_cid.eq(parent_cid))
        .count()
        .get_result::<i64>(conn)
        .map_err(|e| {
            StorageError::Internal(format!(
                "Failed to count attestation children of {parent_cid}: {e}"
            ))
        })
}

/// List all attestations issued by a given issuer CID.
pub fn list_by_issuer(
    conn: &mut SqliteConnection,
    issuer_cid: &str,
) -> Result<Vec<AttestationRow>, StorageError> {
    attestations::table
        .filter(attestations::issuer_cid.eq(issuer_cid))
        .order_by(attestations::created_at.desc())
        .load::<AttestationRow>(conn)
        .map_err(|e| {
            StorageError::Internal(format!(
                "Failed to list attestations by issuer {issuer_cid}: {e}"
            ))
        })
}

/// Delete an attestation projection row by CID.
///
/// Note: deletion from the projection does NOT revoke the DHT entry.
/// Use `upsert` with a `revoked_at` timestamp to represent revocation.
pub fn delete_by_id(conn: &mut SqliteConnection, id: &str) -> Result<usize, StorageError> {
    diesel::delete(attestations::table.filter(attestations::id.eq(id)))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to delete attestation {id}: {e}")))
}

// ===========================================================================
// attestation:gate-decision projection (Phase-2a consolidation)
//
// The legacy per-app gate-decision projection table was dropped in migration
// 2026-05-12-100300. Gate decisions now project into THIS unified table as
// `attestation:gate-decision`, exactly mirroring the elohim-DNA bridge
// `create_gate_decision_attestation` (mishpat/zomes/mishpat/src/lib.rs ~1426):
//
//   subject_cid / issuer_cid = elohim_id (the deciding agent)
//   subject_kind             = "agent"
//   proof_class              = "audit"
//   proof_evidence_json      = { "class": "audit", "reasoning_json": ... }
//   evidence_json            = the full gate-decision field bag
//   manifest_ref             = "mishpat"
//   created_at               = decided_at
//
// The structured fields the old readers filtered on (gate_name, phase,
// elohim_substance_cid, decided_at, request_ref_json, decision,
// gate_process_cid, universal_band_cid) live inside `evidence_json` — see the
// design doc §3.3 (per-subtype evidence_json shape). Because this projection
// stays small (one row per gate evaluation), the gate-name / phase readers below
// load by kind (indexed) and filter the JSON field in Rust; the reputation query
// in `elohim_reputation.rs` uses SQLite `json_extract` in its raw-SQL window join.
// ===========================================================================

/// `attestation_kind` discriminator for gate-decision attestations.
pub const GATE_DECISION_KIND: &str = "attestation:gate-decision";

/// Project a `GateDecisionAttestationEntry` signal into a unified `AttestationRow`.
///
/// `action_hash` is the base64 ActionHash string from the signal envelope; it is
/// stored as the UTF-8 bytes of that string in the BLOB `dht_anchor_hash` column
/// so the legacy `GateDecisionAttestationView.dht_anchor_hash` (a base64 String)
/// reconstructs losslessly on read-back.
pub fn attestation_row_from_gate_decision(
    entry: &crate::signals::GateDecisionAttestationEntry,
    action_hash: &str,
) -> crate::db::models::AttestationRow {
    let evidence_json = serde_json::json!({
        "decision_id": entry.decision_id,
        "phase": entry.phase,
        "elohim_id": entry.elohim_id,
        "elohim_substance_cid": entry.elohim_substance_cid,
        "gate_name": entry.gate_name,
        "gate_process_cid": entry.gate_process_cid,
        "request_ref_json": entry.request_ref_json,
        "decision": entry.decision,
        "context_summary_cid": entry.context_summary_cid,
        "decided_at": entry.decided_at,
        "universal_band_cid": entry.universal_band_cid,
    })
    .to_string();

    let proof_evidence_json = serde_json::json!({
        "class": "audit",
        "reasoning_json": entry.reasoning_json,
    })
    .to_string();

    crate::db::models::AttestationRow {
        id: entry.decision_id.clone(),
        dht_anchor_hash: action_hash.as_bytes().to_vec(),
        attestation_kind: GATE_DECISION_KIND.to_string(),
        subject_cid: entry.elohim_id.clone(),
        subject_kind: "agent".to_string(),
        issuer_cid: entry.elohim_id.clone(),
        parent_governance_action_cid: None,
        vote_value: None,
        vote_weight: None,
        proof_class: "audit".to_string(),
        proof_evidence_json,
        evidence_json,
        expires_at: None,
        supersedes_cid: None,
        revocation_reason: None,
        revoked_at: None,
        // created_at = decided_at: a gate decision's issuance time IS its decision time.
        created_at: entry.decided_at.clone(),
        manifest_ref: "mishpat".to_string(),
        title: format!("Gate decision: {} by {}", entry.gate_name, entry.elohim_id),
        description: Some(format!(
            "Phase: {} — decision: {}",
            entry.phase, entry.decision
        )),
    }
}

/// Fetch a single gate-decision attestation by its decision CID (the row `id`).
pub fn gate_decision_find_by_id(
    conn: &mut SqliteConnection,
    decision_id: &str,
) -> Result<Option<AttestationRow>, StorageError> {
    attestations::table
        .filter(attestations::id.eq(decision_id))
        .filter(attestations::attestation_kind.eq(GATE_DECISION_KIND))
        .first::<AttestationRow>(conn)
        .optional()
        .map_err(|e| {
            StorageError::Internal(format!("Failed to fetch gate decision {decision_id}: {e}"))
        })
}

/// List gate-decision attestations issued by a given elohim agent (issuer_cid),
/// newest decision first.
pub fn gate_decision_find_by_elohim(
    conn: &mut SqliteConnection,
    elohim_id: &str,
) -> Result<Vec<AttestationRow>, StorageError> {
    attestations::table
        .filter(attestations::attestation_kind.eq(GATE_DECISION_KIND))
        .filter(attestations::issuer_cid.eq(elohim_id))
        .order_by(attestations::created_at.desc())
        .load::<AttestationRow>(conn)
        .map_err(|e| {
            StorageError::Internal(format!(
                "Failed to list gate decisions for elohim {elohim_id}: {e}"
            ))
        })
}

/// Read a string field out of a gate-decision row's `evidence_json`.
fn evidence_field(row: &AttestationRow, key: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(&row.evidence_json)
        .ok()
        .and_then(|v| v.get(key).and_then(|x| x.as_str()).map(str::to_string))
}

/// List gate-decision attestations produced by a named gate, newest first.
///
/// `gate_name` lives in `evidence_json`. The projection stays small (one row per
/// gate evaluation), so we load by kind (indexed) and filter the JSON field in
/// Rust rather than reaching for a `json_extract` SQL fragment.
pub fn gate_decision_find_by_gate(
    conn: &mut SqliteConnection,
    gate_name: &str,
) -> Result<Vec<AttestationRow>, StorageError> {
    let rows = attestations::table
        .filter(attestations::attestation_kind.eq(GATE_DECISION_KIND))
        .order_by(attestations::created_at.desc())
        .load::<AttestationRow>(conn)
        .map_err(|e| {
            StorageError::Internal(format!(
                "Failed to list gate decisions for gate {gate_name}: {e}"
            ))
        })?;
    Ok(rows
        .into_iter()
        .filter(|r| evidence_field(r, "gate_name").as_deref() == Some(gate_name))
        .collect())
}

/// List gate-decision attestations from a given deployment phase, newest first.
///
/// `phase` lives in `evidence_json` — filtered in Rust (see `find_by_gate` for
/// why the JSON field is not a `json_extract` SQL filter).
pub fn gate_decision_find_by_phase(
    conn: &mut SqliteConnection,
    phase: &str,
) -> Result<Vec<AttestationRow>, StorageError> {
    let rows = attestations::table
        .filter(attestations::attestation_kind.eq(GATE_DECISION_KIND))
        .order_by(attestations::created_at.desc())
        .load::<AttestationRow>(conn)
        .map_err(|e| {
            StorageError::Internal(format!(
                "Failed to list gate decisions for phase {phase}: {e}"
            ))
        })?;
    Ok(rows
        .into_iter()
        .filter(|r| evidence_field(r, "phase").as_deref() == Some(phase))
        .collect())
}

/// List all gate-decision attestations, newest first, up to `limit`.
pub fn gate_decision_list_all(
    conn: &mut SqliteConnection,
    limit: i64,
) -> Result<Vec<AttestationRow>, StorageError> {
    attestations::table
        .filter(attestations::attestation_kind.eq(GATE_DECISION_KIND))
        .order_by(attestations::created_at.desc())
        .limit(limit)
        .load::<AttestationRow>(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to list gate decisions: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

    const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

    fn setup() -> SqliteConnection {
        let mut conn = SqliteConnection::establish(":memory:").expect("in-memory DB");
        conn.run_pending_migrations(MIGRATIONS).expect("migrations");
        conn
    }

    fn make_row(id: &str, subject: &str, kind: &str) -> AttestationRow {
        AttestationRow {
            id: id.to_string(),
            dht_anchor_hash: b"fakehash0000000000000000000000000".to_vec(),
            attestation_kind: kind.to_string(),
            subject_cid: subject.to_string(),
            subject_kind: "human".to_string(),
            issuer_cid: "issuer-001".to_string(),
            parent_governance_action_cid: None,
            vote_value: None,
            vote_weight: None,
            proof_class: "witness".to_string(),
            proof_evidence_json: "{}".to_string(),
            evidence_json: "{}".to_string(),
            expires_at: None,
            supersedes_cid: None,
            revocation_reason: None,
            revoked_at: None,
            created_at: "2026-05-12T10:00:00Z".to_string(),
            manifest_ref: "imagodei".to_string(),
            title: "Test attestation".to_string(),
            description: None,
        }
    }

    #[test]
    fn insert_and_get() {
        let mut conn = setup();
        let row = make_row("cid-001", "subject-001", "attestation:humanness");
        upsert(&mut conn, &row).unwrap();
        let fetched = get_by_id(&mut conn, "cid-001").unwrap().unwrap();
        assert_eq!(fetched.id, "cid-001");
        assert_eq!(fetched.attestation_kind, "attestation:humanness");
    }

    #[test]
    fn upsert_is_idempotent() {
        let mut conn = setup();
        let row = make_row("cid-002", "subject-002", "attestation:humanness");
        upsert(&mut conn, &row).unwrap();
        upsert(&mut conn, &row).unwrap(); // second upsert should not error
        let rows = list_by_subject(&mut conn, "subject-002", None).unwrap();
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn list_by_subject_with_kind_filter() {
        let mut conn = setup();
        upsert(
            &mut conn,
            &make_row("cid-003", "subject-003", "attestation:humanness"),
        )
        .unwrap();
        upsert(
            &mut conn,
            &make_row("cid-004", "subject-003", "attestation:mastery"),
        )
        .unwrap();

        let all = list_by_subject(&mut conn, "subject-003", None).unwrap();
        assert_eq!(all.len(), 2);

        let filtered =
            list_by_subject(&mut conn, "subject-003", Some("attestation:humanness")).unwrap();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].attestation_kind, "attestation:humanness");
    }

    #[test]
    fn vote_children_list_by_parent() {
        let mut conn = setup();
        let mut vote = make_row("cid-005", "subject-005", "attestation:governance-vote");
        vote.parent_governance_action_cid = Some("gov-action-001".to_string());
        vote.vote_value = Some("approve".to_string());
        upsert(&mut conn, &vote).unwrap();

        let children =
            super::list_by_parent_governance_action(&mut conn, "gov-action-001").unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].vote_value.as_deref(), Some("approve"));
    }

    #[test]
    fn count_by_parent_governance_action_cid_dedupes_on_upsert() {
        let mut conn = setup();
        let mut vote = make_row("vote-A", "subj-A", "attestation:recovery-approval");
        vote.parent_governance_action_cid = Some("gov-parent-X".to_string());
        upsert(&mut conn, &vote).unwrap();
        // Re-upserting the same id must not bump the count.
        upsert(&mut conn, &vote).unwrap();
        upsert(&mut conn, &vote).unwrap();
        assert_eq!(
            count_by_parent_governance_action_cid(&mut conn, "gov-parent-X").unwrap(),
            1
        );

        // A distinct vote increments the count.
        let mut vote2 = make_row("vote-B", "subj-B", "attestation:recovery-approval");
        vote2.parent_governance_action_cid = Some("gov-parent-X".to_string());
        upsert(&mut conn, &vote2).unwrap();
        assert_eq!(
            count_by_parent_governance_action_cid(&mut conn, "gov-parent-X").unwrap(),
            2
        );
    }

    #[test]
    fn delete_by_id_removes_row() {
        let mut conn = setup();
        let row = make_row("cid-006", "subject-006", "attestation:humanness");
        upsert(&mut conn, &row).unwrap();
        let deleted = delete_by_id(&mut conn, "cid-006").unwrap();
        assert_eq!(deleted, 1);
        assert!(get_by_id(&mut conn, "cid-006").unwrap().is_none());
    }
}
