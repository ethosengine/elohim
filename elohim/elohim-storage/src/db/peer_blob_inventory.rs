//! CRUD for `peer_blob_inventory` and `peer_inventory_cursor`.
//!
//! Source of truth: libp2p gossipsub messages on topic 'elohim/inventory/blob'.
//! Category C operational projection rebuildable from gossip replay.
//! Manifest counterpart: rea_commitments(action='custody-blob').
//!
//! ## Sequence semantics
//!
//! - Snapshots are accepted regardless of sequence (recovery path from
//!   sequence-manipulation attacks). The receiver updates its high-watermark
//!   to the snapshot's sequence.
//! - Deltas check `sequence == stored_max + 1`. Gaps queue a snapshot-request.
//!   Replays drop silently.

use crate::db::diesel_schema::{peer_blob_inventory, peer_inventory_cursor};
use crate::db::models::{NewPeerBlobInventoryRow, NewPeerInventoryCursorRow, PeerBlobInventoryRow};
use crate::error::StorageError;
use diesel::prelude::*;

/// Apply a full snapshot for a peer. Replaces all existing entries for this
/// peer with the new set; entries not in the new set are deleted.
/// Snapshots are accepted regardless of sequence.
pub fn apply_snapshot(
    conn: &mut SqliteConnection,
    peer_id: &str,
    hashes: &[String],
    sequence: i64,
    snapshot_at: &str,
) -> Result<(), StorageError> {
    conn.transaction(|conn| {
        // Delete all existing entries for this peer.
        diesel::delete(peer_blob_inventory::table.filter(peer_blob_inventory::peer_id.eq(peer_id)))
            .execute(conn)?;

        // Insert the new set as a single batch.
        let rows: Vec<NewPeerBlobInventoryRow> = hashes
            .iter()
            .map(|hash| NewPeerBlobInventoryRow {
                peer_id: peer_id.to_string(),
                blob_hash: hash.clone(),
                last_seen_at: snapshot_at.to_string(),
                source: "gossip-snapshot".to_string(),
                sequence,
                blake3_hash: None,
            })
            .collect();

        if !rows.is_empty() {
            diesel::insert_into(peer_blob_inventory::table)
                .values(&rows)
                .execute(conn)?;
        }

        // Update cursor. Snapshots always advance the cursor to their sequence.
        upsert_cursor(conn, peer_id, sequence, snapshot_at)?;

        Ok::<(), diesel::result::Error>(())
    })
    .map_err(|e| StorageError::Database(format!("apply_snapshot: {e}")))
}

/// Apply a delta for a peer.
///
/// Returns:
/// - `Ok(DeltaApplyOutcome::Applied)` — delta was applied and cursor advanced.
/// - `Ok(DeltaApplyOutcome::Replay)` — sequence ≤ stored max; drop silently.
/// - `Ok(DeltaApplyOutcome::Gap { expected, received })` — sequence skipped;
///   caller should request a fresh snapshot from the source peer.
/// - `Err(_)` — actual database failure.
pub fn apply_delta(
    conn: &mut SqliteConnection,
    peer_id: &str,
    added: &[String],
    removed: &[String],
    sequence: i64,
    emitted_at: &str,
) -> Result<DeltaApplyOutcome, StorageError> {
    conn.transaction(|conn| {
        let stored_max = read_cursor_sequence(conn, peer_id)?;

        match stored_max {
            Some(max) if sequence <= max => {
                // Replay; drop silently.
                Ok::<DeltaApplyOutcome, diesel::result::Error>(DeltaApplyOutcome::Replay)
            }
            Some(max) if sequence != max + 1 => {
                // Gap detected; do not apply. Caller will request a snapshot.
                Ok::<DeltaApplyOutcome, diesel::result::Error>(DeltaApplyOutcome::Gap {
                    expected: max + 1,
                    received: sequence,
                })
            }
            // Either fresh peer (None) — accept as initial — or sequence == max + 1.
            _ => {
                for hash in added {
                    let row = NewPeerBlobInventoryRow {
                        peer_id: peer_id.to_string(),
                        blob_hash: hash.clone(),
                        last_seen_at: emitted_at.to_string(),
                        source: "gossip-delta".to_string(),
                        sequence,
                        blake3_hash: None,
                    };
                    diesel::replace_into(peer_blob_inventory::table)
                        .values(&row)
                        .execute(conn)?;
                }
                for hash in removed {
                    diesel::delete(
                        peer_blob_inventory::table
                            .filter(peer_blob_inventory::peer_id.eq(peer_id))
                            .filter(peer_blob_inventory::blob_hash.eq(hash)),
                    )
                    .execute(conn)?;
                }
                upsert_cursor(conn, peer_id, sequence, emitted_at)?;
                Ok(DeltaApplyOutcome::Applied)
            }
        }
    })
    .map_err(|e| StorageError::Database(format!("apply_delta: {e}")))
}

/// Record a successful direct fetch. Promotes the entry to source='fetch-success'
/// (the strongest evidence). Does NOT touch the cursor — fetch-success is
/// an out-of-band evidence path, not a gossip arrival.
pub fn record_fetch_success(
    conn: &mut SqliteConnection,
    peer_id: &str,
    blob_hash: &str,
    observed_at: &str,
) -> Result<(), StorageError> {
    conn.transaction(|conn| {
        let existing_seq: Option<i64> = peer_blob_inventory::table
            .filter(peer_blob_inventory::peer_id.eq(peer_id))
            .filter(peer_blob_inventory::blob_hash.eq(blob_hash))
            .select(peer_blob_inventory::sequence)
            .first::<i64>(conn)
            .optional()?;

        let row = NewPeerBlobInventoryRow {
            peer_id: peer_id.to_string(),
            blob_hash: blob_hash.to_string(),
            last_seen_at: observed_at.to_string(),
            source: "fetch-success".to_string(),
            sequence: existing_seq.unwrap_or(0),
            blake3_hash: None,
        };
        diesel::replace_into(peer_blob_inventory::table)
            .values(&row)
            .execute(conn)?;
        Ok::<(), diesel::result::Error>(())
    })
    .map_err(|e| StorageError::Database(format!("record_fetch_success: {e}")))
}

/// Look up the set of peers known to host a blob, ordered by evidence
/// strength (fetch-success first, then by recency).
pub fn lookup_hosts(
    conn: &mut SqliteConnection,
    blob_hash: &str,
    fresh_after: &str,
) -> Result<Vec<PeerBlobInventoryRow>, StorageError> {
    use peer_blob_inventory::dsl;

    dsl::peer_blob_inventory
        .filter(dsl::blob_hash.eq(blob_hash))
        .filter(dsl::last_seen_at.gt(fresh_after))
        .order((
            // SQLite sorts strings lexicographically; 'fetch-success' < 'gossip-*'
            // alphabetically, but we want fetch-success first. Use a CASE expression
            // via order_by would require diesel sql_query; simpler: order by a
            // computed boolean. For now, fetch in two passes and merge.
            dsl::last_seen_at.desc(),
        ))
        .load::<PeerBlobInventoryRow>(conn)
        .map(|rows| {
            // Stable partition: fetch-success first, then the rest in last_seen_at desc order.
            let mut fetch_success: Vec<_> = rows
                .iter()
                .filter(|r| r.source == "fetch-success")
                .cloned()
                .collect();
            let rest: Vec<_> = rows
                .into_iter()
                .filter(|r| r.source != "fetch-success")
                .collect();
            fetch_success.extend(rest);
            fetch_success
        })
        .map_err(|e| StorageError::Database(format!("lookup_hosts: {e}")))
}

/// Outcome of `apply_delta`. Used by the caller to decide whether to request
/// a snapshot from the source peer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeltaApplyOutcome {
    Applied,
    Replay,
    Gap { expected: i64, received: i64 },
}

fn upsert_cursor(
    conn: &mut SqliteConnection,
    peer_id: &str,
    sequence: i64,
    last_updated: &str,
) -> Result<(), diesel::result::Error> {
    let row = NewPeerInventoryCursorRow {
        peer_id: peer_id.to_string(),
        last_sequence: sequence,
        last_updated: last_updated.to_string(),
    };
    diesel::replace_into(peer_inventory_cursor::table)
        .values(&row)
        .execute(conn)
        .map(|_| ())
}

fn read_cursor_sequence(
    conn: &mut SqliteConnection,
    peer_id: &str,
) -> Result<Option<i64>, diesel::result::Error> {
    peer_inventory_cursor::table
        .filter(peer_inventory_cursor::peer_id.eq(peer_id))
        .select(peer_inventory_cursor::last_sequence)
        .first::<i64>(conn)
        .optional()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{run_migrations, DbPool};
    use diesel::r2d2::{ConnectionManager, Pool};

    fn test_pool() -> DbPool {
        let url = format!(
            "file:pbi_test_{}?mode=memory&cache=shared",
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
    fn snapshot_replaces_set() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        apply_snapshot(
            &mut conn,
            "peer_A",
            &["h1".into(), "h2".into()],
            1,
            "2026-05-02T00:00:00Z",
        )
        .unwrap();
        apply_snapshot(
            &mut conn,
            "peer_A",
            &["h2".into(), "h3".into()],
            2,
            "2026-05-02T00:01:00Z",
        )
        .unwrap();

        let rows = lookup_hosts(&mut conn, "h1", "2026-05-01T00:00:00Z").unwrap();
        assert!(rows.is_empty(), "h1 should be gone after second snapshot");

        let rows = lookup_hosts(&mut conn, "h2", "2026-05-01T00:00:00Z").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].peer_id, "peer_A");
        assert_eq!(rows[0].sequence, 2);
    }

    #[test]
    fn delta_applied_when_in_order() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        // Initial snapshot establishes sequence 1.
        apply_snapshot(
            &mut conn,
            "peer_B",
            &["h1".into()],
            1,
            "2026-05-02T00:00:00Z",
        )
        .unwrap();

        let outcome = apply_delta(
            &mut conn,
            "peer_B",
            &["h2".into()],
            &[],
            2,
            "2026-05-02T00:00:30Z",
        )
        .unwrap();
        assert_eq!(outcome, DeltaApplyOutcome::Applied);

        let rows = lookup_hosts(&mut conn, "h2", "2026-05-01T00:00:00Z").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].peer_id, "peer_B");
    }

    #[test]
    fn delta_gap_returns_gap_outcome() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        apply_snapshot(
            &mut conn,
            "peer_C",
            &["h1".into()],
            1,
            "2026-05-02T00:00:00Z",
        )
        .unwrap();

        // Skip to sequence 5 — gap.
        let outcome = apply_delta(
            &mut conn,
            "peer_C",
            &["h2".into()],
            &[],
            5,
            "2026-05-02T00:00:30Z",
        )
        .unwrap();
        assert_eq!(
            outcome,
            DeltaApplyOutcome::Gap {
                expected: 2,
                received: 5
            }
        );

        // h2 should NOT be persisted.
        let rows = lookup_hosts(&mut conn, "h2", "2026-05-01T00:00:00Z").unwrap();
        assert!(rows.is_empty(), "delta with gap must not persist");
    }

    #[test]
    fn delta_replay_drops_silently() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        apply_snapshot(
            &mut conn,
            "peer_D",
            &["h1".into()],
            5,
            "2026-05-02T00:00:00Z",
        )
        .unwrap();

        // Replay an old delta with sequence 3.
        let outcome = apply_delta(
            &mut conn,
            "peer_D",
            &["h2".into()],
            &[],
            3,
            "2026-05-02T00:00:30Z",
        )
        .unwrap();
        assert_eq!(outcome, DeltaApplyOutcome::Replay);

        let rows = lookup_hosts(&mut conn, "h2", "2026-05-01T00:00:00Z").unwrap();
        assert!(rows.is_empty(), "replay must not write");
    }

    #[test]
    fn record_fetch_success_promotes_source() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        apply_snapshot(
            &mut conn,
            "peer_E",
            &["h1".into()],
            1,
            "2026-05-02T00:00:00Z",
        )
        .unwrap();

        record_fetch_success(&mut conn, "peer_E", "h1", "2026-05-02T00:01:00Z").unwrap();

        let rows = lookup_hosts(&mut conn, "h1", "2026-05-01T00:00:00Z").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].source, "fetch-success");
        assert_eq!(rows[0].last_seen_at, "2026-05-02T00:01:00Z");
    }

    #[test]
    fn lookup_hosts_orders_fetch_success_first() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        // Two peers gossip the same blob.
        apply_snapshot(
            &mut conn,
            "peer_F",
            &["h1".into()],
            1,
            "2026-05-02T00:00:00Z",
        )
        .unwrap();
        apply_snapshot(
            &mut conn,
            "peer_G",
            &["h1".into()],
            1,
            "2026-05-02T00:01:00Z",
        )
        .unwrap();

        // peer_F got promoted to fetch-success.
        record_fetch_success(&mut conn, "peer_F", "h1", "2026-05-02T00:00:30Z").unwrap();

        let rows = lookup_hosts(&mut conn, "h1", "2026-05-01T00:00:00Z").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[0].peer_id, "peer_F",
            "fetch-success peer must come first"
        );
        assert_eq!(rows[0].source, "fetch-success");
        assert_eq!(rows[1].peer_id, "peer_G");
    }

    #[test]
    fn lookup_hosts_filters_stale() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        apply_snapshot(
            &mut conn,
            "peer_H",
            &["h1".into()],
            1,
            "2026-05-01T00:00:00Z",
        )
        .unwrap();

        // Fresh-after threshold beyond the snapshot timestamp.
        let rows = lookup_hosts(&mut conn, "h1", "2026-05-02T00:00:00Z").unwrap();
        assert!(rows.is_empty(), "stale entries must not appear");
    }
}
