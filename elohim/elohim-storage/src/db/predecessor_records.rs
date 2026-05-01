//! CRUD for the `predecessor_records` table.
//!
//! Operational: Category C — durable local store for trust-compute gradient
//! back-propagation records received via libp2p gossip. The sealed_blob holds
//! the dryoc-encrypted 2-of-2 predecessor payload; T11 lights up the crypto.
//! Durable disk persistence ensures peer-restart does not truncate back-prop chains.
//!
//! ## Primary operations
//!
//! - `insert_predecessor` — idempotent receive; ON CONFLICT DO NOTHING so
//!   duplicate gossip deliveries of the same `(target_cid, predecessor_peer_id)`
//!   pair are silently ignored rather than erroring.
//! - `list_predecessors_for_cid` — returns all predecessor rows for a given CID,
//!   ordered by `received_at` ascending; multiple peers may each contribute a
//!   predecessor record.
//! - `delete_for_cid` — removes all rows matching `target_cid`; used on retraction.

use crate::db::diesel_schema::predecessor_records;
use diesel::prelude::*;

// ============================================================================
// Row types
// ============================================================================

/// Queryable model for reading predecessor_records rows (SELECT).
#[derive(Debug, Clone, Queryable, Selectable, PartialEq)]
#[diesel(table_name = predecessor_records)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct PredecessorRecordRow {
    /// Auto-increment surrogate key — not meaningful outside this node.
    pub id: i32,
    /// CID of the content whose predecessor this record describes.
    pub target_cid: String,
    /// libp2p PeerId of the peer asserting this predecessor relationship.
    pub predecessor_peer_id: String,
    /// RFC 3339 UTC timestamp of when this node received the record.
    pub received_at: String,
    /// dryoc-encrypted 2-of-2 predecessor payload (T11 crypto).
    pub sealed_blob: Vec<u8>,
}

/// Insertable model for predecessor_records rows (INSERT with ON CONFLICT DO NOTHING).
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = predecessor_records)]
pub struct NewPredecessorRecord {
    pub target_cid: String,
    pub predecessor_peer_id: String,
    pub received_at: String,
    pub sealed_blob: Vec<u8>,
}

// ============================================================================
// Queries
// ============================================================================

/// Insert a predecessor record, ignoring duplicate `(target_cid, predecessor_peer_id)`.
///
/// Returns `Ok(1)` on new insert, `Ok(0)` when the unique constraint fires
/// (idempotent gossip delivery). Never returns an error for constraint violations.
pub fn insert_predecessor(
    conn: &mut SqliteConnection,
    target_cid: &str,
    predecessor_peer_id: &str,
    received_at: &str,
    sealed_blob: Vec<u8>,
) -> QueryResult<usize> {
    let row = NewPredecessorRecord {
        target_cid: target_cid.to_string(),
        predecessor_peer_id: predecessor_peer_id.to_string(),
        received_at: received_at.to_string(),
        sealed_blob,
    };

    diesel::insert_into(predecessor_records::table)
        .values(&row)
        .on_conflict_do_nothing()
        .execute(conn)
}

/// Return all predecessor records for the given `target_cid`.
///
/// Multiple peers may each hold a predecessor relationship to the same CID,
/// so this may return more than one row.
///
/// Rows are ordered by `received_at` ascending so callers (e.g. T12 back-prop
/// service) observe a stable chronological traversal order.
pub fn list_predecessors_for_cid(
    conn: &mut SqliteConnection,
    target_cid: &str,
) -> QueryResult<Vec<PredecessorRecordRow>> {
    use predecessor_records::dsl;

    dsl::predecessor_records
        .filter(dsl::target_cid.eq(target_cid))
        .order_by(dsl::received_at.asc())
        .load::<PredecessorRecordRow>(conn)
}

/// Delete all predecessor records for the given `target_cid`.
///
/// Returns the count of deleted rows. Used on EPR retraction.
pub fn delete_for_cid(conn: &mut SqliteConnection, target_cid: &str) -> QueryResult<usize> {
    use predecessor_records::dsl;

    diesel::delete(dsl::predecessor_records.filter(dsl::target_cid.eq(target_cid))).execute(conn)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{run_migrations, DbPool};
    use diesel::r2d2::{ConnectionManager, Pool};

    fn test_pool() -> DbPool {
        let url = format!(
            "file:pred_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4().as_simple()
        );
        let pool = Pool::builder()
            .max_size(1)
            .build(ConnectionManager::<SqliteConnection>::new(&url))
            .expect("pool");
        run_migrations(&pool).expect("migrations");
        pool
    }

    #[test]
    fn inserts_and_retrieves_one() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        let n = insert_predecessor(
            &mut conn,
            "bafyreicid1",
            "12D3KooWPeer1",
            "2026-05-01T03:00:00Z",
            vec![0x01, 0x02, 0x03],
        )
        .expect("insert");
        assert_eq!(n, 1, "first insert should affect 1 row");

        let rows = list_predecessors_for_cid(&mut conn, "bafyreicid1").expect("get");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].target_cid, "bafyreicid1");
        assert_eq!(rows[0].predecessor_peer_id, "12D3KooWPeer1");
        assert_eq!(rows[0].received_at, "2026-05-01T03:00:00Z");
        assert_eq!(rows[0].sealed_blob, vec![0x01, 0x02, 0x03]);
    }

    #[test]
    fn idempotent_insert_via_unique_constraint() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        insert_predecessor(
            &mut conn,
            "bafyreicid2",
            "12D3KooWPeer2",
            "2026-05-01T03:00:00Z",
            vec![0xAA],
        )
        .expect("first insert");

        // Identical (target_cid, predecessor_peer_id) — must be a no-op, not an error.
        let n = insert_predecessor(
            &mut conn,
            "bafyreicid2",
            "12D3KooWPeer2",
            "2026-05-01T04:00:00Z", // different timestamp, still same pair
            vec![0xBB],
        )
        .expect("second insert should not error");

        assert_eq!(n, 0, "duplicate insert should affect 0 rows");

        let rows = list_predecessors_for_cid(&mut conn, "bafyreicid2").expect("get");
        assert_eq!(
            rows.len(),
            1,
            "only one row must exist after idempotent insert"
        );
        // Original blob is preserved, not overwritten.
        assert_eq!(rows[0].sealed_blob, vec![0xAA]);
    }

    #[test]
    fn multiple_predecessors_for_same_cid() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        insert_predecessor(
            &mut conn,
            "bafyreicid3",
            "12D3KooWPeerA",
            "2026-05-01T03:00:00Z",
            vec![0x01],
        )
        .expect("insert peer A");

        insert_predecessor(
            &mut conn,
            "bafyreicid3",
            "12D3KooWPeerB",
            "2026-05-01T03:01:00Z",
            vec![0x02],
        )
        .expect("insert peer B");

        let rows = list_predecessors_for_cid(&mut conn, "bafyreicid3").expect("get");
        assert_eq!(rows.len(), 2, "two distinct peers = two rows");

        let peer_ids: Vec<&str> = rows
            .iter()
            .map(|r| r.predecessor_peer_id.as_str())
            .collect();
        assert!(peer_ids.contains(&"12D3KooWPeerA"));
        assert!(peer_ids.contains(&"12D3KooWPeerB"));
    }

    #[test]
    fn delete_for_cid_removes_all() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        insert_predecessor(
            &mut conn,
            "bafyreicid4",
            "12D3KooWPeerX",
            "2026-05-01T03:00:00Z",
            vec![0x10],
        )
        .expect("insert X");

        insert_predecessor(
            &mut conn,
            "bafyreicid4",
            "12D3KooWPeerY",
            "2026-05-01T03:00:00Z",
            vec![0x20],
        )
        .expect("insert Y");

        let deleted = delete_for_cid(&mut conn, "bafyreicid4").expect("delete");
        assert_eq!(deleted, 2, "two rows should be deleted");

        let rows = list_predecessors_for_cid(&mut conn, "bafyreicid4").expect("get after delete");
        assert!(rows.is_empty(), "no rows should remain after delete");
    }
}
