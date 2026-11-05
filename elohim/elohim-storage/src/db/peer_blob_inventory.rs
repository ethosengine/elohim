//! CRUD for `peer_blob_inventory` and `peer_inventory_cursor`.
//!
//! Source of truth: signed inventory envelopes on topic `elohim/inventory/blob/2`
//! (`p2p::inventory_gossip`) and negentropy-v1 range reconciliation
//! (`p2p::inventory_recon`). Category C operational projection, rebuildable by recon.
//! Manifest counterpart: rea_commitments(action='custody-blob'). Every `blob_cid` is
//! the byte CID text of an address the caller already admitted by full decode.
//!
//! ## Sequence semantics
//!
//! - A delta applies only when `sequence == stored_max + 1` (or the peer is new).
//!   A replay (`sequence <= stored_max`) is dropped; a gap is reported so the caller
//!   runs a recon session with that peer.
//! - [`rebase_after_recon`] is the ONLY path that moves a cursor to an arbitrary
//!   sequence: a completed recon proves the projected set equals the holder's set at
//!   the responder's sequence.

use crate::db::diesel_schema::{peer_blob_inventory, peer_inventory_cursor};
use crate::db::models::{NewPeerBlobInventoryRow, NewPeerInventoryCursorRow, PeerBlobInventoryRow};
use crate::error::StorageError;
use diesel::prelude::*;

/// Apply a signed inventory delta from `peer_id`.
///
/// Returns:
/// - `Ok(DeltaApplyOutcome::Applied)` — delta was applied and cursor advanced.
/// - `Ok(DeltaApplyOutcome::Replay)` — sequence ≤ stored max; dropped.
/// - `Ok(DeltaApplyOutcome::Gap { expected, received })` — sequence skipped; nothing
///   applied, the caller reconciles with the source peer.
/// - `Err(_)` — actual database failure.
pub fn apply_delta(
    conn: &mut SqliteConnection,
    peer_id: &str,
    added: &[String],
    removed: &[String],
    sequence: i64,
    emitted_at: &str,
) -> Result<DeltaApplyOutcome, StorageError> {
    super::read_then_write_transaction(conn, |conn| {
        let stored_max = read_cursor_sequence(conn, peer_id)?;
        match stored_max {
            Some(max) if sequence <= max => {
                Ok::<DeltaApplyOutcome, diesel::result::Error>(DeltaApplyOutcome::Replay)
            }
            Some(max) if sequence != max + 1 => Ok(DeltaApplyOutcome::Gap {
                expected: max + 1,
                received: sequence,
            }),
            _ => {
                write_set_change(
                    conn,
                    peer_id,
                    added,
                    removed,
                    "gossip-delta",
                    sequence,
                    emitted_at,
                )?;
                upsert_cursor(conn, peer_id, sequence, emitted_at)?;
                Ok(DeltaApplyOutcome::Applied)
            }
        }
    })
    .map_err(|e| StorageError::Database(format!("apply_delta: {e}")))
}

/// Project a completed recon session with `peer_id`: `add` are the byte CIDs the peer
/// holds that this projection lacked, `remove` the ones it listed that the peer does not
/// hold. The cursor is re-based to the responder's `sequence` — the one path that moves
/// a cursor to an arbitrary sequence, because a converged recon proves the set.
pub fn rebase_after_recon(
    conn: &mut SqliteConnection,
    peer_id: &str,
    add: &[String],
    remove: &[String],
    sequence: i64,
    when: &str,
) -> Result<(), StorageError> {
    super::read_then_write_transaction(conn, |conn| {
        write_set_change(conn, peer_id, add, remove, "recon", sequence, when)?;
        upsert_cursor(conn, peer_id, sequence, when)?;
        Ok::<(), diesel::result::Error>(())
    })
    .map_err(|e| StorageError::Database(format!("rebase_after_recon: {e}")))
}

/// Record a successful direct fetch. Promotes the entry to source='fetch-success'
/// (the strongest evidence). Does NOT touch the cursor — fetch-success is an
/// out-of-band evidence path, not a gossip arrival.
pub fn record_fetch_success(
    conn: &mut SqliteConnection,
    peer_id: &str,
    blob_cid: &str,
    observed_at: &str,
) -> Result<(), StorageError> {
    super::read_then_write_transaction(conn, |conn| {
        let existing_seq: Option<i64> = peer_blob_inventory::table
            .filter(peer_blob_inventory::peer_id.eq(peer_id))
            .filter(peer_blob_inventory::blob_cid.eq(blob_cid))
            .select(peer_blob_inventory::sequence)
            .first::<i64>(conn)
            .optional()?;

        let row = NewPeerBlobInventoryRow {
            peer_id: peer_id.to_string(),
            blob_cid: blob_cid.to_string(),
            last_seen_at: observed_at.to_string(),
            source: "fetch-success".to_string(),
            sequence: existing_seq.unwrap_or(0),
        };
        diesel::replace_into(peer_blob_inventory::table)
            .values(&row)
            .execute(conn)?;
        Ok::<(), diesel::result::Error>(())
    })
    .map_err(|e| StorageError::Database(format!("record_fetch_success: {e}")))
}

/// Stamp a row recording that *this* peer holds `blob_cid` locally (the blob store
/// verified the bytes when it stored them). Idempotent; does NOT touch the cursor —
/// self-custody is local evidence, not a gossip arrival.
pub fn record_self_custody(
    conn: &mut SqliteConnection,
    peer_id: &str,
    blob_cid: &str,
    observed_at: &str,
) -> Result<(), StorageError> {
    let row = NewPeerBlobInventoryRow {
        peer_id: peer_id.to_string(),
        blob_cid: blob_cid.to_string(),
        last_seen_at: observed_at.to_string(),
        source: "self-custody".to_string(),
        sequence: 0,
    };
    diesel::replace_into(peer_blob_inventory::table)
        .values(&row)
        .execute(conn)
        .map(|_| ())
        .map_err(|e| StorageError::Database(format!("record_self_custody: {e}")))
}

/// The peers known to host a blob, ordered by evidence strength (fetch-success first,
/// then by recency).
pub fn lookup_hosts(
    conn: &mut SqliteConnection,
    blob_cid: &str,
    fresh_after: &str,
) -> Result<Vec<PeerBlobInventoryRow>, StorageError> {
    use peer_blob_inventory::dsl;

    dsl::peer_blob_inventory
        .filter(dsl::blob_cid.eq(blob_cid))
        .filter(dsl::last_seen_at.gt(fresh_after))
        .order(dsl::last_seen_at.desc())
        .select(PeerBlobInventoryRow::as_select())
        .load::<PeerBlobInventoryRow>(conn)
        .map(|rows| {
            let (mut fetch_success, rest): (Vec<_>, Vec<_>) =
                rows.into_iter().partition(|r| r.source == "fetch-success");
            fetch_success.extend(rest);
            fetch_success
        })
        .map_err(|e| StorageError::Database(format!("lookup_hosts: {e}")))
}

/// The byte CIDs `peer_id` is projected to hold — the initiator's set for a recon
/// session with that peer.
pub fn cids_for_peer(
    conn: &mut SqliteConnection,
    peer_id: &str,
) -> Result<Vec<String>, StorageError> {
    peer_blob_inventory::table
        .filter(peer_blob_inventory::peer_id.eq(peer_id))
        .select(peer_blob_inventory::blob_cid)
        .load::<String>(conn)
        .map_err(|e| StorageError::Database(format!("cids_for_peer: {e}")))
}

/// Outcome of [`apply_delta`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeltaApplyOutcome {
    Applied,
    Replay,
    Gap { expected: i64, received: i64 },
}

fn write_set_change(
    conn: &mut SqliteConnection,
    peer_id: &str,
    added: &[String],
    removed: &[String],
    source: &str,
    sequence: i64,
    at: &str,
) -> Result<(), diesel::result::Error> {
    for cid in added {
        let row = NewPeerBlobInventoryRow {
            peer_id: peer_id.to_string(),
            blob_cid: cid.clone(),
            last_seen_at: at.to_string(),
            source: source.to_string(),
            sequence,
        };
        diesel::replace_into(peer_blob_inventory::table)
            .values(&row)
            .execute(conn)?;
    }
    for cid in removed {
        diesel::delete(
            peer_blob_inventory::table
                .filter(peer_blob_inventory::peer_id.eq(peer_id))
                .filter(peer_blob_inventory::blob_cid.eq(cid)),
        )
        .execute(conn)?;
    }
    Ok(())
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
    use crate::test_util::test_pool;
    use elohim_epr::address::compute_bytes_cid;

    fn cid(label: &str) -> String {
        compute_bytes_cid(label.as_bytes()).to_string()
    }

    const T0: &str = "2026-11-06T00:00:00Z";
    const T1: &str = "2026-11-06T00:00:30Z";
    const LONG_AGO: &str = "2026-11-01T00:00:00Z";

    #[test]
    fn delta_applied_when_in_order() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let first = apply_delta(&mut conn, "peer_B", &[cid("h1")], &[], 1, T0).unwrap();
        assert_eq!(first, DeltaApplyOutcome::Applied);
        let next = apply_delta(&mut conn, "peer_B", &[cid("h2")], &[cid("h1")], 2, T1).unwrap();
        assert_eq!(next, DeltaApplyOutcome::Applied);

        assert_eq!(
            lookup_hosts(&mut conn, &cid("h2"), LONG_AGO).unwrap().len(),
            1
        );
        assert!(lookup_hosts(&mut conn, &cid("h1"), LONG_AGO)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn delta_gap_returns_gap_and_writes_nothing() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        apply_delta(&mut conn, "peer_C", &[cid("h1")], &[], 1, T0).unwrap();
        let outcome = apply_delta(&mut conn, "peer_C", &[cid("h2")], &[], 5, T1).unwrap();
        assert_eq!(
            outcome,
            DeltaApplyOutcome::Gap {
                expected: 2,
                received: 5
            }
        );
        assert!(lookup_hosts(&mut conn, &cid("h2"), LONG_AGO)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn replayed_delta_drops_and_never_moves_the_cursor() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        for seq in 1..=5 {
            apply_delta(
                &mut conn,
                "peer_D",
                &[cid(&format!("h{seq}"))],
                &[],
                seq,
                T0,
            )
            .unwrap();
        }
        let outcome = apply_delta(&mut conn, "peer_D", &[cid("late")], &[], 3, T1).unwrap();
        assert_eq!(outcome, DeltaApplyOutcome::Replay);
        assert!(lookup_hosts(&mut conn, &cid("late"), LONG_AGO)
            .unwrap()
            .is_empty());
        assert_eq!(read_cursor_sequence(&mut conn, "peer_D").unwrap(), Some(5));
    }

    #[test]
    fn recon_rebases_the_cursor_and_projects_the_set() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        apply_delta(&mut conn, "peer_R", &[cid("stale")], &[], 900, T0).unwrap();

        rebase_after_recon(&mut conn, "peer_R", &[cid("held")], &[cid("stale")], 3, T1).unwrap();
        assert_eq!(read_cursor_sequence(&mut conn, "peer_R").unwrap(), Some(3));
        assert_eq!(
            cids_for_peer(&mut conn, "peer_R").unwrap(),
            vec![cid("held")]
        );
        // The next in-order delta after the re-base applies.
        let next = apply_delta(&mut conn, "peer_R", &[cid("new")], &[], 4, T1).unwrap();
        assert_eq!(next, DeltaApplyOutcome::Applied);
        let host = lookup_hosts(&mut conn, &cid("held"), LONG_AGO).unwrap();
        assert_eq!(host[0].source, "recon");
    }

    #[test]
    fn lookup_hosts_orders_fetch_success_first_and_filters_stale() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let blob = cid("shared");
        apply_delta(&mut conn, "older", std::slice::from_ref(&blob), &[], 1, T0).unwrap();
        apply_delta(&mut conn, "newer", std::slice::from_ref(&blob), &[], 1, T1).unwrap();
        record_fetch_success(&mut conn, "older", &blob, T0).unwrap();

        let rows = lookup_hosts(&mut conn, &blob, LONG_AGO).unwrap();
        assert_eq!(rows[0].peer_id, "older");
        assert_eq!(rows[0].source, "fetch-success");
        assert_eq!(rows[1].peer_id, "newer");
        assert_eq!(lookup_hosts(&mut conn, &blob, T0).unwrap().len(), 1);
    }

    #[test]
    fn record_self_custody_is_idempotent() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let blob = cid("mine");
        record_self_custody(&mut conn, "self", &blob, T0).unwrap();
        record_self_custody(&mut conn, "self", &blob, T1).unwrap();
        let rows = lookup_hosts(&mut conn, &blob, LONG_AGO).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].source, "self-custody");
    }
}
