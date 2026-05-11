//! CRUD + queries for the governance_actions projection table.
//! Source of truth: Holochain DHT (Content entries with content_type LIKE 'governance-action:%').
//! This table is a read-optimised projection populated by post-commit signals;
//! treat rows as a cache — the DHT entry is authoritative.

use diesel::prelude::*;

use crate::db::diesel_schema::governance_actions;
use crate::db::models::GovernanceActionRow;
use crate::error::StorageError;

/// Upsert a governance action projection row.
///
/// Idempotent: replaying the same signal produces the same row.
/// Conflict on `id` (CID — primary key) replaces all columns.
pub fn upsert(
    conn: &mut SqliteConnection,
    row: &GovernanceActionRow,
) -> Result<(), StorageError> {
    diesel::insert_into(governance_actions::table)
        .values(row)
        .on_conflict(governance_actions::id)
        .do_update()
        .set(row)
        .execute(conn)
        .map(|_| ())
        .map_err(|e| StorageError::Internal(format!("Failed to upsert governance_action: {e}")))
}

/// Fetch a single governance action by CID.
pub fn get_by_id(
    conn: &mut SqliteConnection,
    id: &str,
) -> Result<Option<GovernanceActionRow>, StorageError> {
    governance_actions::table
        .filter(governance_actions::id.eq(id))
        .first::<GovernanceActionRow>(conn)
        .optional()
        .map_err(|e| {
            StorageError::Internal(format!("Failed to fetch governance_action {id}: {e}"))
        })
}

/// List governance actions for a subject CID.
pub fn list_by_subject(
    conn: &mut SqliteConnection,
    subject_cid: &str,
) -> Result<Vec<GovernanceActionRow>, StorageError> {
    governance_actions::table
        .filter(governance_actions::subject_cid.eq(subject_cid))
        .order_by(governance_actions::created_at.desc())
        .load::<GovernanceActionRow>(conn)
        .map_err(|e| {
            StorageError::Internal(format!(
                "Failed to list governance_actions for subject {subject_cid}: {e}"
            ))
        })
}

/// List governance actions that are still open (closes_at > now).
///
/// `now_rfc3339` should be an RFC 3339 timestamp string, e.g. from
/// `chrono::Utc::now().to_rfc3339()`.
pub fn list_open(
    conn: &mut SqliteConnection,
    now_rfc3339: &str,
) -> Result<Vec<GovernanceActionRow>, StorageError> {
    governance_actions::table
        .filter(governance_actions::closes_at.gt(now_rfc3339))
        .order_by(governance_actions::closes_at.asc())
        .load::<GovernanceActionRow>(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to list open governance_actions: {e}")))
}

/// Delete a governance action projection row by CID.
///
/// Note: deletion from the projection does NOT invalidate the DHT entry.
pub fn delete_by_id(
    conn: &mut SqliteConnection,
    id: &str,
) -> Result<usize, StorageError> {
    diesel::delete(governance_actions::table.filter(governance_actions::id.eq(id)))
        .execute(conn)
        .map_err(|e| {
            StorageError::Internal(format!("Failed to delete governance_action {id}: {e}"))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use diesel::prelude::*;
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

    const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

    fn setup() -> SqliteConnection {
        let mut conn = SqliteConnection::establish(":memory:").expect("in-memory DB");
        conn.run_pending_migrations(MIGRATIONS).expect("migrations");
        conn
    }

    fn make_row(id: &str, subject: &str, closes_at: &str) -> GovernanceActionRow {
        GovernanceActionRow {
            id: id.to_string(),
            dht_anchor_hash: b"fakehash0000000000000000000000000".to_vec(),
            governance_kind: "governance-action:recovery".to_string(),
            subject_cid: subject.to_string(),
            proposer_cid: "proposer-001".to_string(),
            threshold_json: r#"{"m":3}"#.to_string(),
            eligibility_predicate_json: None,
            ballot_format: "approve-reject".to_string(),
            closes_at: closes_at.to_string(),
            parameters_json: None,
            title: "Recovery proposal".to_string(),
            description: None,
            created_at: "2026-05-12T10:00:00Z".to_string(),
        }
    }

    #[test]
    fn insert_and_get() {
        let mut conn = setup();
        let row = make_row("gov-001", "subject-001", "2099-01-01T00:00:00Z");
        upsert(&mut conn, &row).unwrap();
        let fetched = get_by_id(&mut conn, "gov-001").unwrap().unwrap();
        assert_eq!(fetched.id, "gov-001");
        assert_eq!(fetched.governance_kind, "governance-action:recovery");
    }

    #[test]
    fn upsert_is_idempotent() {
        let mut conn = setup();
        let row = make_row("gov-002", "subject-002", "2099-01-01T00:00:00Z");
        upsert(&mut conn, &row).unwrap();
        upsert(&mut conn, &row).unwrap();
        let rows = list_by_subject(&mut conn, "subject-002").unwrap();
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn list_open_filters_by_closes_at() {
        let mut conn = setup();
        upsert(&mut conn, &make_row("gov-003", "subj-003", "2099-01-01T00:00:00Z")).unwrap();
        upsert(&mut conn, &make_row("gov-004", "subj-004", "2020-01-01T00:00:00Z")).unwrap();

        // Only gov-003 should be "open" relative to now (2026)
        let open = list_open(&mut conn, "2026-05-12T00:00:00Z").unwrap();
        assert_eq!(open.len(), 1);
        assert_eq!(open[0].id, "gov-003");
    }

    #[test]
    fn delete_by_id_removes_row() {
        let mut conn = setup();
        let row = make_row("gov-005", "subj-005", "2099-01-01T00:00:00Z");
        upsert(&mut conn, &row).unwrap();
        let deleted = delete_by_id(&mut conn, "gov-005").unwrap();
        assert_eq!(deleted, 1);
        assert!(get_by_id(&mut conn, "gov-005").unwrap().is_none());
    }
}
