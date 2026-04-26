//! CRUD for the `portal_hosts` table.
//!
//! Source of truth: Holochain DHT (imagodei PortalHost entry — M5).
//! This table is a Category A projection rebuildable from signal replay.
//!
//! ## Primary operations
//!
//! - `upsert` — called by the ReconcileController when a PortalHostRegistered signal
//!   arrives; idempotent on `dht_anchor_hash`.
//! - `list_for_human` — called by the portal host resolution path to enumerate all
//!   registered doorways for a human contributor.
//! - `delete_by_anchor_hash` — called when a PortalHostRevoked signal removes a
//!   registration from the DHT.
//! - `update_last_reachable` — operational enrichment; called by the reachability
//!   probe after a successful HTTP health check. Does NOT touch DHT truth.

use crate::db::diesel_schema::portal_hosts;
use crate::db::models::{NewPortalHostRow, PortalHostRow};
use crate::error::StorageError;
use diesel::prelude::*;

/// Upsert a portal host row.
///
/// Uses INSERT OR REPLACE semantics keyed on `dht_anchor_hash` (UNIQUE constraint).
/// If the same DHT entry is observed again (e.g. replayed from signal log), the row
/// is replaced with the latest values. `last_reachable_at` is NOT carried over —
/// callers should call `update_last_reachable` separately if they need to preserve it.
pub fn upsert(conn: &mut SqliteConnection, row: &NewPortalHostRow) -> Result<(), StorageError> {
    diesel::replace_into(portal_hosts::table)
        .values(row)
        .execute(conn)
        .map(|_| ())
        .map_err(|e| StorageError::Database(format!("portal_hosts upsert: {e}")))
}

/// Delete a portal host row by its DHT anchor hash.
///
/// Returns the number of rows deleted (0 if the anchor was not present).
pub fn delete_by_anchor_hash(
    conn: &mut SqliteConnection,
    anchor: &str,
) -> Result<usize, StorageError> {
    use portal_hosts::dsl;

    diesel::delete(dsl::portal_hosts.filter(dsl::dht_anchor_hash.eq(anchor)))
        .execute(conn)
        .map_err(|e| StorageError::Database(format!("portal_hosts delete: {e}")))
}

/// List all portal hosts registered for a given human, most recently added first.
///
/// Returns an empty Vec if the human has no registered portal hosts.
pub fn list_for_human(
    conn: &mut SqliteConnection,
    human_id: &str,
) -> Result<Vec<PortalHostRow>, StorageError> {
    use portal_hosts::dsl;

    dsl::portal_hosts
        .filter(dsl::human_id.eq(human_id))
        .order(dsl::added_at.desc())
        .load::<PortalHostRow>(conn)
        .map_err(|e| StorageError::Database(format!("portal_hosts list_for_human: {e}")))
}

/// Update the `last_reachable_at` timestamp for a portal host.
///
/// Operational enrichment — records the last time the reachability probe confirmed
/// the host URL was responsive. This field is NOT in the DHT entry; it is purely
/// local operational state. Returns the number of rows updated (0 if anchor not found).
pub fn update_last_reachable(
    conn: &mut SqliteConnection,
    anchor: &str,
    timestamp_iso: &str,
) -> Result<usize, StorageError> {
    use portal_hosts::dsl;

    diesel::update(dsl::portal_hosts.filter(dsl::dht_anchor_hash.eq(anchor)))
        .set(dsl::last_reachable_at.eq(timestamp_iso))
        .execute(conn)
        .map_err(|e| StorageError::Database(format!("portal_hosts update_last_reachable: {e}")))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::NewPortalHostRow;
    use crate::db::{run_migrations, DbPool};
    use diesel::r2d2::{ConnectionManager, Pool};

    fn test_pool() -> DbPool {
        let url = format!(
            "file:ph_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4().as_simple()
        );
        let pool = Pool::builder()
            .max_size(1)
            .build(ConnectionManager::<SqliteConnection>::new(&url))
            .expect("pool");
        run_migrations(&pool).expect("migrations");
        pool
    }

    fn host(
        human_id: &str,
        host_url: &str,
        label: Option<&str>,
        added_at: &str,
        reach: &str,
        dht_anchor_hash: &str,
    ) -> NewPortalHostRow {
        NewPortalHostRow {
            human_id: human_id.to_string(),
            host_url: host_url.to_string(),
            label: label.map(|s| s.to_string()),
            added_at: added_at.to_string(),
            last_reachable_at: None,
            reach: reach.to_string(),
            dht_anchor_hash: dht_anchor_hash.to_string(),
        }
    }

    #[test]
    fn upsert_and_list_roundtrip() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        let row = host(
            "human-abc",
            "https://doorway.example.com",
            Some("Home doorway"),
            "2026-04-25T00:00:00Z",
            "public",
            "uhCAkPortalAnchor1",
        );
        upsert(&mut conn, &row).expect("upsert");

        let results = list_for_human(&mut conn, "human-abc").expect("list");
        assert_eq!(results.len(), 1);
        let found = &results[0];
        assert_eq!(found.host_url, "https://doorway.example.com");
        assert_eq!(found.label.as_deref(), Some("Home doorway"));
        assert_eq!(found.reach, "public");
        assert_eq!(found.dht_anchor_hash, "uhCAkPortalAnchor1");
        assert!(found.last_reachable_at.is_none());
    }

    #[test]
    fn list_returns_empty_for_unknown_human() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        let results = list_for_human(&mut conn, "human-nobody").expect("list");
        assert!(results.is_empty(), "expected empty list for unknown human");
    }

    #[test]
    fn list_multiple_hosts_ordered_by_added_at_desc() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        upsert(
            &mut conn,
            &host(
                "human-multi",
                "https://old.example.com",
                None,
                "2026-01-01T00:00:00Z",
                "public",
                "anchor-old",
            ),
        )
        .expect("upsert 1");

        upsert(
            &mut conn,
            &host(
                "human-multi",
                "https://new.example.com",
                None,
                "2026-04-25T00:00:00Z",
                "public",
                "anchor-new",
            ),
        )
        .expect("upsert 2");

        let results = list_for_human(&mut conn, "human-multi").expect("list");
        assert_eq!(results.len(), 2);
        // Most recently added first
        assert_eq!(results[0].dht_anchor_hash, "anchor-new");
        assert_eq!(results[1].dht_anchor_hash, "anchor-old");
    }

    #[test]
    fn delete_by_anchor_hash_removes_row() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        let row = host(
            "human-delete",
            "https://doorway.example.com",
            None,
            "2026-04-25T00:00:00Z",
            "private",
            "anchor-to-delete",
        );
        upsert(&mut conn, &row).expect("upsert");

        // Confirm it exists
        let before = list_for_human(&mut conn, "human-delete").expect("list before");
        assert_eq!(before.len(), 1);

        // Delete by anchor
        let deleted = delete_by_anchor_hash(&mut conn, "anchor-to-delete").expect("delete");
        assert_eq!(deleted, 1);

        // Confirm it's gone
        let after = list_for_human(&mut conn, "human-delete").expect("list after");
        assert!(after.is_empty(), "row should be removed after delete");
    }

    #[test]
    fn delete_by_anchor_hash_returns_zero_for_missing() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        let deleted =
            delete_by_anchor_hash(&mut conn, "anchor-nonexistent").expect("delete missing");
        assert_eq!(deleted, 0, "deleting nonexistent anchor should return 0");
    }

    #[test]
    fn upsert_replaces_on_same_dht_anchor_hash() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        let anchor = "anchor-replace";

        let original = host(
            "human-replace",
            "https://v1.example.com",
            Some("v1 label"),
            "2026-04-25T00:00:00Z",
            "public",
            anchor,
        );
        upsert(&mut conn, &original).expect("upsert v1");

        // Same anchor, updated URL — simulates signal replay with corrected data
        let updated = NewPortalHostRow {
            human_id: "human-replace".to_string(),
            host_url: "https://v2.example.com".to_string(),
            label: Some("v2 label".to_string()),
            added_at: "2026-04-25T01:00:00Z".to_string(),
            last_reachable_at: None,
            reach: "public".to_string(),
            dht_anchor_hash: anchor.to_string(),
        };
        upsert(&mut conn, &updated).expect("upsert v2");

        let results = list_for_human(&mut conn, "human-replace").expect("list");
        assert_eq!(results.len(), 1, "replace should not create a second row");
        assert_eq!(results[0].host_url, "https://v2.example.com");
    }

    #[test]
    fn update_last_reachable_sets_timestamp() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        let anchor = "anchor-reachable";
        let row = host(
            "human-reach",
            "https://doorway.example.com",
            None,
            "2026-04-25T00:00:00Z",
            "public",
            anchor,
        );
        upsert(&mut conn, &row).expect("upsert");

        let updated =
            update_last_reachable(&mut conn, anchor, "2026-04-25T12:00:00Z").expect("update");
        assert_eq!(updated, 1);

        let results = list_for_human(&mut conn, "human-reach").expect("list");
        assert_eq!(
            results[0].last_reachable_at.as_deref(),
            Some("2026-04-25T12:00:00Z")
        );
    }

    #[test]
    fn update_last_reachable_returns_zero_for_missing_anchor() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        let updated = update_last_reachable(&mut conn, "no-such-anchor", "2026-04-25T12:00:00Z")
            .expect("update missing");
        assert_eq!(updated, 0);
    }
}
