//! Projection (not a source of truth) of the notarized PeerStatus DHT entry.
//!
//! Writes here come exclusively from the post-commit signal projection in
//! `src/signals.rs` (Task 9). Source of truth: the infrastructure DNA DHT.
//! If this projection and the DHT disagree, the DHT wins — rebuild the
//! projection from `get_all_current_peer_statuses` rather than mutating
//! this table directly.
//!
//! Column conventions match the rest of the storage layer:
//! - Booleans: SQLite INTEGER (0/1), modeled as `i32` in Rust.
//! - ActionHashes: TEXT (base64), modeled as `String`. No binary blobs cross
//!   the wire; `dht_anchor_hash` is the same shape as the signal payload so
//!   the projector can do `String -> String` with no decoding step.

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

// Reflection of the projection schema — the source of truth remains the DHT entry.
use crate::db::diesel_schema::peer_statuses;

/// Row-shaped projection of a PeerStatus DHT entry.
///
/// Boolean flags are stored as `i32` to match the column type; convert with
/// `value != 0` at the view-layer boundary when surfacing to clients.
#[derive(Debug, Clone, Queryable, Insertable, AsChangeset, Identifiable)]
#[diesel(table_name = peer_statuses, primary_key(peer_id))]
pub struct PeerStatusRow {
    pub peer_id: String,
    pub status: String,
    pub general_pool_member: i32,
    pub accepting_stewardship_reserves: i32,
    pub archetype_class: Option<String>,
    pub timestamp: i64,
    pub dht_anchor_hash: String,
    pub updated_at: i64,
}

/// Insert or update a peer status row, keyed on `peer_id`.
///
/// Conflict resolution is last-writer-wins on `timestamp` enforcement — the
/// signal projector is expected to drop stale signals before calling this.
pub fn upsert(conn: &mut SqliteConnection, row: &PeerStatusRow) -> QueryResult<usize> {
    diesel::insert_into(peer_statuses::table)
        .values(row)
        .on_conflict(peer_statuses::peer_id)
        .do_update()
        .set(row)
        .execute(conn)
}

/// Look up the current projected status for a single peer, if any.
pub fn get_by_peer(
    conn: &mut SqliteConnection,
    peer: &str,
) -> QueryResult<Option<PeerStatusRow>> {
    peer_statuses::table
        .find(peer)
        .first::<PeerStatusRow>(conn)
        .optional()
}

/// List peers currently advertising themselves as general-pool members and
/// either online or degraded. Consumed by the doorway forwarder's
/// candidate-selection step.
pub fn list_pool_members(conn: &mut SqliteConnection) -> QueryResult<Vec<PeerStatusRow>> {
    peer_statuses::table
        .filter(peer_statuses::general_pool_member.eq(1))
        .filter(peer_statuses::status.eq_any(vec!["online", "degraded"]))
        .load::<PeerStatusRow>(conn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use diesel::connection::SimpleConnection;

    fn setup_test_db() -> SqliteConnection {
        let mut conn =
            SqliteConnection::establish(":memory:").expect("Failed to create in-memory database");

        conn.batch_execute(
            r#"
            CREATE TABLE peer_statuses (
                peer_id TEXT PRIMARY KEY,
                status TEXT NOT NULL,
                general_pool_member INTEGER NOT NULL,
                accepting_stewardship_reserves INTEGER NOT NULL,
                archetype_class TEXT,
                timestamp BIGINT NOT NULL,
                dht_anchor_hash TEXT NOT NULL,
                updated_at BIGINT NOT NULL
            );
            CREATE INDEX idx_peer_statuses_status ON peer_statuses(status);
            CREATE INDEX idx_peer_statuses_pool ON peer_statuses(general_pool_member);
            "#,
        )
        .expect("Failed to create test table");

        conn
    }

    #[test]
    fn upsert_then_read_latest() {
        let mut conn = setup_test_db();
        let row = PeerStatusRow {
            peer_id: "uhCAkABC".into(),
            status: "online".into(),
            general_pool_member: 1,
            accepting_stewardship_reserves: 1,
            archetype_class: Some("home-nuc".into()),
            timestamp: 1_700_000_000_000_000,
            dht_anchor_hash: String::new(),
            updated_at: 1_700_000_000_000_000,
        };
        upsert(&mut conn, &row).unwrap();

        let fetched = get_by_peer(&mut conn, "uhCAkABC").unwrap().unwrap();
        assert_eq!(fetched.status, "online");
        assert_eq!(fetched.general_pool_member, 1);
        assert_eq!(fetched.archetype_class.as_deref(), Some("home-nuc"));

        // Upsert again with a newer status — should update, not duplicate.
        let updated = PeerStatusRow {
            status: "degraded".into(),
            timestamp: 1_700_000_000_000_001,
            updated_at: 1_700_000_000_000_001,
            ..row.clone()
        };
        upsert(&mut conn, &updated).unwrap();
        let reread = get_by_peer(&mut conn, "uhCAkABC").unwrap().unwrap();
        assert_eq!(reread.status, "degraded");
        assert_eq!(reread.timestamp, 1_700_000_000_000_001);
    }

    #[test]
    fn get_by_peer_returns_none_for_unknown() {
        let mut conn = setup_test_db();
        assert!(get_by_peer(&mut conn, "missing").unwrap().is_none());
    }

    #[test]
    fn list_pool_members_filters_to_true() {
        let mut conn = setup_test_db();
        let mk = |peer: &str, member: i32, status: &str| PeerStatusRow {
            peer_id: peer.into(),
            status: status.into(),
            general_pool_member: member,
            accepting_stewardship_reserves: 1,
            archetype_class: None,
            timestamp: 1,
            dht_anchor_hash: String::new(),
            updated_at: 1,
        };
        upsert(&mut conn, &mk("adam", 1, "online")).unwrap();
        upsert(&mut conn, &mk("timothy", 0, "online")).unwrap();
        // Pool member but offline state filtered out by status clause.
        upsert(&mut conn, &mk("eve", 1, "leaving")).unwrap();

        let members = list_pool_members(&mut conn).unwrap();
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].peer_id, "adam");
    }
}
