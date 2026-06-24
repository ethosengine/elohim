//! CRUD for `salvage_capacity`.
//!
//! Source of truth: salvage-capacity gossip (`SalvageCapacityAd` on topic
//! 'elohim/storage/salvage'). Category C operational projection rebuildable
//! from gossip replay.
//!
//! agent_cid-keyed by construction — the candidate pool the Phase-3 placement
//! XOR metric ranks over never crosses identity namespaces (the all-zeros
//! incident). The candidate pool = FRESH, opted-in entries ([`list_fresh`],
//! TTL-aged like inventory).

use crate::db::diesel_schema::salvage_capacity;
use crate::db::models::{NewSalvageCapacityRow, SalvageCapacityRow};
use crate::error::StorageError;
use diesel::prelude::*;

/// Apply (upsert) a salvage-capacity advertisement for an agent. Replaces any
/// prior row for the same `agent_cid` (PK), so the newest advertisement wins.
/// Mirrors `peer_blob_inventory`'s `replace_into` upsert.
pub fn apply_capacity(
    conn: &mut SqliteConnection,
    agent_cid: &str,
    spare_bytes: i64,
    archetype: &str,
    observed_at: &str,
    seq: i64,
) -> Result<(), StorageError> {
    let row = NewSalvageCapacityRow {
        agent_cid: agent_cid.to_string(),
        spare_bytes,
        archetype: archetype.to_string(),
        last_seen_at: observed_at.to_string(),
        seq,
    };
    diesel::replace_into(salvage_capacity::table)
        .values(&row)
        .execute(conn)
        .map(|_| ())
        .map_err(|e| StorageError::Database(format!("apply_capacity: {e}")))
}

/// Lookup the capacity row for a specific agent (None if never advertised).
pub fn lookup_capacity(
    conn: &mut SqliteConnection,
    agent_cid: &str,
) -> Result<Option<SalvageCapacityRow>, StorageError> {
    salvage_capacity::table
        .find(agent_cid)
        .select(SalvageCapacityRow::as_select())
        .first::<SalvageCapacityRow>(conn)
        .optional()
        .map_err(|e| StorageError::Database(format!("lookup_capacity: {e}")))
}

/// Return all capacity rows whose `last_seen_at` is strictly newer than
/// `fresh_after` (an ISO-8601 cutoff). This is the salvage candidate pool:
/// stale entries (peers that stopped advertising) age out and are excluded.
/// P3-6 maps the returned rows into `PlacementCandidate`s.
pub fn list_fresh(
    conn: &mut SqliteConnection,
    fresh_after: &str,
) -> Result<Vec<SalvageCapacityRow>, StorageError> {
    salvage_capacity::table
        .filter(salvage_capacity::last_seen_at.gt(fresh_after))
        .select(SalvageCapacityRow::as_select())
        .load::<SalvageCapacityRow>(conn)
        .map_err(|e| StorageError::Database(format!("list_fresh: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{run_migrations, DbPool};
    use diesel::r2d2::{ConnectionManager, Pool};

    fn test_pool() -> DbPool {
        let url = format!(
            "file:salvage_cap_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4().as_simple()
        );
        let pool = Pool::builder()
            .max_size(1)
            .build(ConnectionManager::<SqliteConnection>::new(&url))
            .expect("pool");
        run_migrations(&pool).expect("migrations");
        pool
    }

    /// ISO-8601 timestamp `offset_secs` relative to now (negative = in the past).
    fn iso_offset(offset_secs: i64) -> String {
        (chrono::Utc::now() + chrono::Duration::seconds(offset_secs))
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string()
    }

    #[test]
    fn apply_then_lookup_roundtrips() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        apply_capacity(&mut conn, "uhCAk-self", 4096, "node", &iso_offset(0), 1).unwrap();

        let row = lookup_capacity(&mut conn, "uhCAk-self").unwrap().unwrap();
        assert_eq!(row.agent_cid, "uhCAk-self");
        assert_eq!(row.spare_bytes, 4096);
        assert_eq!(row.archetype, "node");
        assert_eq!(row.seq, 1);
        assert!(lookup_capacity(&mut conn, "uhCAk-absent")
            .unwrap()
            .is_none());
    }

    #[test]
    fn apply_is_upsert_newest_wins() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        apply_capacity(&mut conn, "uhCAk-self", 1000, "node", &iso_offset(-10), 1).unwrap();
        apply_capacity(&mut conn, "uhCAk-self", 9000, "steward", &iso_offset(0), 2).unwrap();

        let row = lookup_capacity(&mut conn, "uhCAk-self").unwrap().unwrap();
        assert_eq!(
            row.spare_bytes, 9000,
            "second advertisement replaces the first"
        );
        assert_eq!(row.archetype, "steward");
        assert_eq!(row.seq, 2);
    }

    /// `list_fresh` returns recently-advertised rows and excludes stale ones —
    /// the candidate-pool freshness gate the salvage pass relies on.
    #[test]
    fn list_fresh_includes_fresh_excludes_stale() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        // Fresh entry (last_seen now).
        apply_capacity(&mut conn, "uhCAk-fresh", 2048, "node", &iso_offset(0), 1).unwrap();
        // Stale entry (last_seen well in the past).
        apply_capacity(
            &mut conn,
            "uhCAk-stale",
            2048,
            "node",
            &iso_offset(-3600),
            1,
        )
        .unwrap();

        // Cutoff = 10 minutes ago: fresh in, stale out.
        let cutoff = iso_offset(-600);
        let rows = list_fresh(&mut conn, &cutoff).unwrap();

        let cids: Vec<&str> = rows.iter().map(|r| r.agent_cid.as_str()).collect();
        assert!(cids.contains(&"uhCAk-fresh"), "fresh row must be included");
        assert!(!cids.contains(&"uhCAk-stale"), "stale row must be excluded");
        assert_eq!(rows.len(), 1);
    }
}
