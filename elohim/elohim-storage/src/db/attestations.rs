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
pub fn upsert(
    conn: &mut SqliteConnection,
    row: &AttestationRow,
) -> Result<(), StorageError> {
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
pub fn delete_by_id(
    conn: &mut SqliteConnection,
    id: &str,
) -> Result<usize, StorageError> {
    diesel::delete(attestations::table.filter(attestations::id.eq(id)))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to delete attestation {id}: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use diesel::prelude::*;
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

    const MIGRATIONS: EmbeddedMigrations =
        embed_migrations!("migrations");

    fn setup() -> SqliteConnection {
        let mut conn = SqliteConnection::establish(":memory:").expect("in-memory DB");
        conn.run_pending_migrations(MIGRATIONS)
            .expect("migrations");
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
        upsert(&mut conn, &make_row("cid-003", "subject-003", "attestation:humanness")).unwrap();
        upsert(&mut conn, &make_row("cid-004", "subject-003", "attestation:mastery")).unwrap();

        let all = list_by_subject(&mut conn, "subject-003", None).unwrap();
        assert_eq!(all.len(), 2);

        let filtered = list_by_subject(&mut conn, "subject-003", Some("attestation:humanness")).unwrap();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].attestation_kind, "attestation:humanness");
    }

    #[test]
    fn list_by_parent_governance_action() {
        let mut conn = setup();
        let mut vote = make_row("cid-005", "subject-005", "attestation:governance-vote");
        vote.parent_governance_action_cid = Some("gov-action-001".to_string());
        vote.vote_value = Some("approve".to_string());
        upsert(&mut conn, &vote).unwrap();

        let children = list_by_parent_governance_action(&mut conn, "gov-action-001").unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].vote_value.as_deref(), Some("approve"));
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
