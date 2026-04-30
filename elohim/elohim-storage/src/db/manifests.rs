//! CRUD for the `manifests` projection table — Phase 3 P3.2.
//!
//! Source of truth: Holochain DHT (Manifest entry — content_store_integrity zome).
//! This table is a Category C operational projection rebuildable from signal replay.
//!
//! ## Primary operations
//!
//! - `insert_manifest` — upsert by CID; bumps `revision` on conflict.
//! - `fetch_manifest_by_cid` — primary-key lookup.
//! - `fetch_manifests_by_pillar` — registry refresh helper for ManifestRegistry (Task 5).
//! - `fetch_manifests_by_kind` — registry filter by manifest_kind (e.g., "pillar-projection").

use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::db::diesel_schema::manifests;
use crate::error::StorageError;

/// Row model for the `manifests` table (Queryable + Insertable in one struct,
/// matching the pattern used by other simple projection tables that don't need
/// a separate `New*Row` insert shape).
#[derive(Queryable, Selectable, Insertable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = manifests)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct ManifestRow {
    pub cid: String,
    pub manifest_kind: String,
    pub pillar: Option<String>,
    pub payload_json: String,
    pub schema_ref: Option<String>,
    pub signer_pubkey: Vec<u8>,
    pub created_at: String,
    pub verified_at: Option<String>,
    pub revision: i32,
}

/// Insert (or upsert) a manifest row.
///
/// On `cid` conflict, `payload_json` and `verified_at` are updated and the
/// `revision` is incremented by 1. This matches Phase 3's "manifest mutations
/// project to the latest local snapshot, with revision counter visible to
/// the registry" semantics.
pub fn insert_manifest(conn: &mut SqliteConnection, row: &ManifestRow) -> Result<(), StorageError> {
    diesel::insert_into(manifests::table)
        .values(row)
        .on_conflict(manifests::cid)
        .do_update()
        .set((
            manifests::payload_json.eq(&row.payload_json),
            manifests::verified_at.eq(&row.verified_at),
            manifests::revision.eq(manifests::revision + 1),
        ))
        .execute(conn)
        .map(|_| ())
        .map_err(|e| StorageError::Database(format!("manifests insert: {e}")))
}

/// Fetch a single manifest by CID. Returns None if not present.
pub fn fetch_manifest_by_cid(
    conn: &mut SqliteConnection,
    cid: &str,
) -> Result<Option<ManifestRow>, StorageError> {
    manifests::table
        .find(cid)
        .first::<ManifestRow>(conn)
        .optional()
        .map_err(|e| StorageError::Database(format!("manifests fetch by cid: {e}")))
}

/// Fetch all manifests for a given pillar (e.g., "lamad").
pub fn fetch_manifests_by_pillar(
    conn: &mut SqliteConnection,
    pillar: &str,
) -> Result<Vec<ManifestRow>, StorageError> {
    manifests::table
        .filter(manifests::pillar.eq(pillar))
        .load::<ManifestRow>(conn)
        .map_err(|e| StorageError::Database(format!("manifests fetch by pillar: {e}")))
}

/// Fetch all manifests of a given manifest_kind (e.g., "pillar-projection").
pub fn fetch_manifests_by_kind(
    conn: &mut SqliteConnection,
    manifest_kind: &str,
) -> Result<Vec<ManifestRow>, StorageError> {
    manifests::table
        .filter(manifests::manifest_kind.eq(manifest_kind))
        .load::<ManifestRow>(conn)
        .map_err(|e| StorageError::Database(format!("manifests fetch by kind: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::test_pool;

    fn fake_manifest_row(cid: &str, pillar: Option<&str>) -> ManifestRow {
        ManifestRow {
            cid: cid.to_string(),
            manifest_kind: "pillar-projection".to_string(),
            pillar: pillar.map(String::from),
            payload_json: r#"{"version":1}"#.to_string(),
            schema_ref: None,
            signer_pubkey: vec![0u8; 32],
            created_at: "2026-04-30T00:00:00Z".to_string(),
            verified_at: None,
            revision: 1,
        }
    }

    #[test]
    fn insert_and_fetch_by_cid() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let row = fake_manifest_row("test-cid-1", Some("lamad"));
        insert_manifest(&mut conn, &row).unwrap();
        let fetched = fetch_manifest_by_cid(&mut conn, "test-cid-1")
            .unwrap()
            .unwrap();
        assert_eq!(fetched.cid, "test-cid-1");
        assert_eq!(fetched.pillar, Some("lamad".to_string()));
    }

    #[test]
    fn fetch_by_pillar_returns_matches() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        insert_manifest(&mut conn, &fake_manifest_row("c1", Some("lamad"))).unwrap();
        insert_manifest(&mut conn, &fake_manifest_row("c2", Some("shefa"))).unwrap();
        insert_manifest(&mut conn, &fake_manifest_row("c3", Some("lamad"))).unwrap();
        let lamad = fetch_manifests_by_pillar(&mut conn, "lamad").unwrap();
        assert_eq!(lamad.len(), 2);
    }

    #[test]
    fn upsert_increments_revision() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let mut row = fake_manifest_row("upsert-cid", Some("lamad"));
        insert_manifest(&mut conn, &row).unwrap();
        row.payload_json = r#"{"version":2}"#.to_string();
        insert_manifest(&mut conn, &row).unwrap();
        let fetched = fetch_manifest_by_cid(&mut conn, "upsert-cid")
            .unwrap()
            .unwrap();
        assert_eq!(fetched.revision, 2);
        assert_eq!(fetched.payload_json, r#"{"version":2}"#);
    }

    #[test]
    fn fetch_missing_returns_none() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let result = fetch_manifest_by_cid(&mut conn, "nonexistent").unwrap();
        assert!(result.is_none());
    }
}
