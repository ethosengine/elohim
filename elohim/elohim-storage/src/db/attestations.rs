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

// ===========================================================================
// attestation:statement-vote projection (Phase-2a consolidation)
//
// The legacy `statement_votes` table was dropped in migration 2026-05-12-100300.
// Polis-style agree/disagree/pass votes on an OpinionStatement now project into
// THIS unified table as `attestation:statement-vote`, mirroring the elohim-DNA
// bridge `create_statement_vote` (mishpat/zomes/mishpat/src/lib.rs ~1324):
//
//   subject_cid                  = statement_id (the OpinionStatement voted on)
//   subject_kind                 = "governance-action"
//   issuer_cid                   = the voter (human_id)
//   parent_governance_action_cid = statement_id (this is a child vote, per §4)
//   vote_value                   = the vote ("agree" | "disagree" | "pass")
//   proof_class                  = "witness"
//   proof_evidence_json          = { "class": "witness" }
//   evidence_json                = { statement_id, voter_id, vote }
//   manifest_ref                 = "mishpat"
//
// Mapping NOTE (schema drift, honestly recorded — NOT a silent guess): the
// `statement-vote-metadata.schema.json` describes `{ statement_cid, polis_axis }`
// under `summary_metric`, but the live DNA bridge writes the flat bag above with
// the vote in the real `vote_value` column. Storage is the DHT's projection, so
// it mirrors what the conductor emits (the DNA bridge), exactly as the
// gate-decision migration mirrored its bridge over the schema's enum shape.
//
// Latest-wins (Polis semantics) is FREE: the row `id` is deterministic
// (`sv-{statement_id}-{human_id}`), so a voter re-voting produces the same `id`
// and `upsert`'s `on_conflict(id).do_update()` replaces the prior vote in place —
// equivalent to the old delete-then-insert, no explicit delete needed.
// ===========================================================================

/// `attestation_kind` discriminator for statement-vote attestations.
pub const STATEMENT_VOTE_KIND: &str = "attestation:statement-vote";

/// Deterministic, latest-wins row id for a (statement, voter) vote.
pub fn statement_vote_id(statement_id: &str, human_id: &str) -> String {
    format!("sv-{statement_id}-{human_id}")
}

/// Project a statement vote into a unified `AttestationRow`.
///
/// `dht_anchor_hash` is a NOT NULL BLOB; statement votes have no DHT anchor on a
/// direct storage write, so we store empty bytes and project them back to `None`
/// on read (`statement_vote_from_attestation`) — preserving the legacy
/// `StatementVote.dht_anchor_hash: Option<String> = None` wire shape.
pub fn attestation_row_from_statement_vote(
    statement_id: &str,
    human_id: &str,
    vote: &str,
    created_at: &str,
) -> AttestationRow {
    let evidence_json = serde_json::json!({
        "statement_id": statement_id,
        "voter_id": human_id,
        "vote": vote,
    })
    .to_string();

    AttestationRow {
        id: statement_vote_id(statement_id, human_id),
        dht_anchor_hash: Vec::new(),
        attestation_kind: STATEMENT_VOTE_KIND.to_string(),
        subject_cid: statement_id.to_string(),
        subject_kind: "governance-action".to_string(),
        issuer_cid: human_id.to_string(),
        parent_governance_action_cid: Some(statement_id.to_string()),
        vote_value: Some(vote.to_string()),
        vote_weight: None,
        proof_class: "witness".to_string(),
        proof_evidence_json: serde_json::json!({ "class": "witness" }).to_string(),
        evidence_json,
        expires_at: None,
        supersedes_cid: None,
        revocation_reason: None,
        revoked_at: None,
        created_at: created_at.to_string(),
        manifest_ref: "mishpat".to_string(),
        title: format!("Statement vote on {statement_id} by {human_id}"),
        description: Some(format!("Vote: {vote}")),
    }
}

/// Project a statement-vote `AttestationRow` back into the `StatementVote` domain
/// struct the clustering + view-converter consumers expect.
pub fn statement_vote_from_attestation(row: AttestationRow) -> crate::db::models::StatementVote {
    // The empty-BLOB sentinel projects back to None (no DHT anchor).
    let dht_anchor_hash = if row.dht_anchor_hash.is_empty() {
        None
    } else {
        String::from_utf8(row.dht_anchor_hash).ok()
    };
    crate::db::models::StatementVote {
        id: row.id,
        statement_id: row.subject_cid,
        // issuer_cid IS the voter (human_id).
        human_id: row.issuer_cid,
        vote: row.vote_value.unwrap_or_default(),
        created_at: row.created_at,
        dht_anchor_hash,
    }
}

/// Upsert a statement vote (latest-wins on the deterministic id).
pub fn upsert_statement_vote(
    conn: &mut SqliteConnection,
    statement_id: &str,
    human_id: &str,
    vote: &str,
    created_at: &str,
) -> Result<crate::db::models::StatementVote, StorageError> {
    let row = attestation_row_from_statement_vote(statement_id, human_id, vote, created_at);
    upsert(conn, &row)?;
    Ok(statement_vote_from_attestation(row))
}

/// List all statement votes cast against a single statement, oldest first.
pub fn statement_votes_for_statement(
    conn: &mut SqliteConnection,
    statement_id: &str,
) -> Result<Vec<crate::db::models::StatementVote>, StorageError> {
    let rows = attestations::table
        .filter(attestations::attestation_kind.eq(STATEMENT_VOTE_KIND))
        .filter(attestations::subject_cid.eq(statement_id))
        .order_by(attestations::created_at.asc())
        .load::<AttestationRow>(conn)
        .map_err(|e| {
            StorageError::Internal(format!(
                "Failed to list statement votes for {statement_id}: {e}"
            ))
        })?;
    Ok(rows
        .into_iter()
        .map(statement_vote_from_attestation)
        .collect())
}

/// List all statement votes cast against any of the given statement CIDs,
/// oldest first (for entity-wide clustering).
pub fn statement_votes_for_statements(
    conn: &mut SqliteConnection,
    statement_ids: &[String],
) -> Result<Vec<crate::db::models::StatementVote>, StorageError> {
    let rows = attestations::table
        .filter(attestations::attestation_kind.eq(STATEMENT_VOTE_KIND))
        .filter(attestations::subject_cid.eq_any(statement_ids))
        .order_by(attestations::created_at.asc())
        .load::<AttestationRow>(conn)
        .map_err(|e| {
            StorageError::Internal(format!("Failed to list statement votes for entity: {e}"))
        })?;
    Ok(rows
        .into_iter()
        .map(statement_vote_from_attestation)
        .collect())
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

    // -- attestation:statement-vote (Phase-2a) -------------------------------

    #[test]
    fn statement_vote_round_trips_through_attestations() {
        let mut conn = setup();
        // Write a vote via the repointed writer (projects into unified attestations).
        let projected = upsert_statement_vote(
            &mut conn,
            "stmt-1",
            "human-A",
            "agree",
            "2026-05-12T10:00:00Z",
        )
        .unwrap();
        assert_eq!(projected.id, "sv-stmt-1-human-A");
        assert_eq!(projected.statement_id, "stmt-1");
        assert_eq!(projected.human_id, "human-A");
        assert_eq!(projected.vote, "agree");
        // No DHT anchor on a direct write → None (empty-BLOB sentinel projected back).
        assert_eq!(projected.dht_anchor_hash, None);

        // The raw row mirrors the DNA bridge mapping.
        let row = get_by_id(&mut conn, "sv-stmt-1-human-A").unwrap().unwrap();
        assert_eq!(row.attestation_kind, STATEMENT_VOTE_KIND);
        assert_eq!(row.subject_cid, "stmt-1");
        assert_eq!(row.subject_kind, "governance-action");
        assert_eq!(row.issuer_cid, "human-A");
        assert_eq!(row.parent_governance_action_cid.as_deref(), Some("stmt-1"));
        assert_eq!(row.vote_value.as_deref(), Some("agree"));
        assert_eq!(row.proof_class, "witness");
        assert_eq!(row.manifest_ref, "mishpat");
        assert!(row.dht_anchor_hash.is_empty());

        // Read back via the repointed reader.
        let votes = statement_votes_for_statement(&mut conn, "stmt-1").unwrap();
        assert_eq!(votes.len(), 1);
        assert_eq!(votes[0].vote, "agree");
        assert_eq!(votes[0].dht_anchor_hash, None);
    }

    #[test]
    fn statement_vote_is_latest_wins() {
        let mut conn = setup();
        // Same (statement, voter) votes agree, then changes to disagree.
        upsert_statement_vote(
            &mut conn,
            "stmt-2",
            "human-B",
            "agree",
            "2026-05-12T10:00:00Z",
        )
        .unwrap();
        upsert_statement_vote(
            &mut conn,
            "stmt-2",
            "human-B",
            "disagree",
            "2026-05-12T10:05:00Z",
        )
        .unwrap();

        // Deterministic id → exactly one row survives, holding the latest vote.
        let votes = statement_votes_for_statement(&mut conn, "stmt-2").unwrap();
        assert_eq!(
            votes.len(),
            1,
            "latest-wins: one row per (statement, voter)"
        );
        assert_eq!(votes[0].vote, "disagree");
        assert_eq!(votes[0].id, "sv-stmt-2-human-B");
    }

    #[test]
    fn statement_votes_for_statements_spans_an_entity() {
        let mut conn = setup();
        upsert_statement_vote(
            &mut conn,
            "stmt-3",
            "human-C",
            "agree",
            "2026-05-12T10:00:00Z",
        )
        .unwrap();
        upsert_statement_vote(
            &mut conn,
            "stmt-4",
            "human-C",
            "pass",
            "2026-05-12T10:01:00Z",
        )
        .unwrap();
        // A vote on an unrelated statement must NOT be returned.
        upsert_statement_vote(
            &mut conn,
            "stmt-X",
            "human-D",
            "disagree",
            "2026-05-12T10:02:00Z",
        )
        .unwrap();

        let ids = vec!["stmt-3".to_string(), "stmt-4".to_string()];
        let votes = statement_votes_for_statements(&mut conn, &ids).unwrap();
        assert_eq!(votes.len(), 2);
        assert!(votes
            .iter()
            .all(|v| v.statement_id == "stmt-3" || v.statement_id == "stmt-4"));
    }
}
