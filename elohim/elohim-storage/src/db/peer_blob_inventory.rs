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
//!
//! ## Snapshot idempotency (convergence under WAN loss/reorder)
//!
//! `apply_snapshot` dedups on a CONTENT fingerprint stored on the cursor
//! (`last_content_hash`): a snapshot whose blob set equals the last-applied set
//! is a cheap no-op ([`SnapshotApplyOutcome::Deduplicated`]) and the caller skips
//! commitment re-scoring. This collapses gossipsub re-delivery storms and
//! redundant periodic re-snapshots (the publisher bumps `sequence` every tick
//! even when content is unchanged) back toward the design cadence, without
//! weakening the accept-regardless-of-sequence attack-recovery path — a genuinely
//! different snapshot at any sequence has a different fingerprint and still
//! applies.

use crate::db::diesel_schema::{peer_blob_inventory, peer_inventory_cursor};
use crate::db::models::{NewPeerBlobInventoryRow, NewPeerInventoryCursorRow, PeerBlobInventoryRow};
use crate::error::StorageError;
use diesel::prelude::*;

/// Apply a full snapshot for a peer, idempotently.
///
/// When the incoming blob set is byte-identical to the last set already applied
/// for this peer (compared via a content fingerprint stored on the cursor), the
/// snapshot carries no new information: it is a cheap no-op — no delete+reinsert,
/// and, via [`SnapshotApplyOutcome::Deduplicated`], the caller skips commitment
/// re-scoring. Otherwise the peer's entries are fully replaced (entries not in
/// the new set are deleted) and the fingerprint is recorded.
///
/// **Convergence contract.** Under WAN loss/reorder, gossipsub re-floods the same
/// snapshot many times (IHAVE/IWANT gap recovery + relay fan-in) and the publisher
/// re-emits an unchanged inventory every tick with a fresh `sequence`. Both
/// collapse to no-ops here, keeping a full-arc sink flat instead of pegging a core
/// re-applying every echo. See migration
/// `2026-07-06-120000_peer_inventory_cursor_content_hash`.
///
/// **Sequence semantics preserved.** Dedup keys on CONTENT, never on
/// `(peer_id, sequence)`: a genuinely different recovery snapshot at ANY sequence
/// (including a repeated one — the "accepted regardless of sequence" attack-recovery
/// path) has a different fingerprint and still applies.
pub fn apply_snapshot(
    conn: &mut SqliteConnection,
    peer_id: &str,
    hashes: &[String],
    sequence: i64,
    snapshot_at: &str,
) -> Result<SnapshotApplyOutcome, StorageError> {
    let content_hash = content_fingerprint(hashes);

    conn.transaction::<SnapshotApplyOutcome, diesel::result::Error, _>(|conn| {
        // Read the current cursor: sequence high-watermark, the freshness clock
        // (last_updated), and the fingerprint of the last-applied set.
        let existing: Option<(i64, String, Option<String>)> = peer_inventory_cursor::table
            .filter(peer_inventory_cursor::peer_id.eq(peer_id))
            .select((
                peer_inventory_cursor::last_sequence,
                peer_inventory_cursor::last_updated,
                peer_inventory_cursor::last_content_hash,
            ))
            .first::<(i64, String, Option<String>)>(conn)
            .optional()?;

        // ── Idempotent fast-path ────────────────────────────────────────────
        if let Some((last_seq, ref last_updated, Some(ref last_hash))) = existing {
            if *last_hash == content_hash {
                // Identical inventory already applied. Skip the O(N) set churn
                // and (via the outcome) the caller's per-hash commitment scoring.
                let need_seq_advance = sequence > last_seq;
                let need_refresh =
                    secs_between(last_updated, snapshot_at) >= DEDUP_FRESHNESS_REFRESH_SECS;

                if need_refresh {
                    // Cheap, rate-limited freshness touch so lookup_hosts() keeps
                    // returning a still-announced STATIC inventory — one indexed
                    // UPDATE, no set churn, at most once per refresh interval even
                    // under a re-delivery storm.
                    diesel::update(
                        peer_blob_inventory::table.filter(peer_blob_inventory::peer_id.eq(peer_id)),
                    )
                    .set(peer_blob_inventory::last_seen_at.eq(snapshot_at))
                    .execute(conn)?;
                }

                if need_seq_advance || need_refresh {
                    // Keep the delta high-watermark contiguous with the publisher's
                    // shared snapshot/delta counter so a following in-order delta is
                    // not misread as a gap; never move the watermark backward on a
                    // reordered echo. Advance the freshness clock ONLY when we
                    // actually refreshed, so the rate-limit measures from the last
                    // real touch.
                    let new_seq = sequence.max(last_seq);
                    let clock: &str = if need_refresh {
                        snapshot_at
                    } else {
                        last_updated
                    };
                    upsert_cursor(conn, peer_id, new_seq, clock, Some(&content_hash))?;
                }

                return Ok(SnapshotApplyOutcome::Deduplicated);
            }
        }

        // ── Content changed (or first snapshot / delta-advanced cursor) ─────
        // Full replace. Delete all existing entries for this peer.
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

        // Update cursor and record the fingerprint of the applied set. Snapshots
        // always advance the cursor to their sequence (accept-regardless-of-sequence).
        upsert_cursor(conn, peer_id, sequence, snapshot_at, Some(&content_hash))?;

        Ok(SnapshotApplyOutcome::Applied)
    })
    .map_err(|e| StorageError::Database(format!("apply_snapshot: {e}")))
}

/// Order-independent content fingerprint of a peer's advertised blob set.
/// SHA-256 over the sorted, de-duplicated, length-prefixed hashes — hex. Two
/// snapshots advertising the same set (in any order) share a fingerprint; an
/// empty set has a stable fingerprint (so re-echoed empty snapshots also dedup).
fn content_fingerprint(hashes: &[String]) -> String {
    use sha2::{Digest, Sha256};
    let mut sorted: Vec<&str> = hashes.iter().map(|s| s.as_str()).collect();
    sorted.sort_unstable();
    sorted.dedup();
    let mut hasher = Sha256::new();
    hasher.update((sorted.len() as u64).to_le_bytes());
    for h in sorted {
        // Length-prefix each element so set boundaries are unambiguous
        // (["ab","c"] and ["a","bc"] must not collide).
        hasher.update((h.len() as u64).to_le_bytes());
        hasher.update(h.as_bytes());
    }
    hex::encode(hasher.finalize())
}

/// Whole seconds from `earlier_iso` to `later_iso` (both `%Y-%m-%dT%H:%M:%SZ`).
/// On any parse failure returns `i64::MAX` — fail toward a freshness refresh,
/// never toward silently letting rows go stale.
fn secs_between(earlier_iso: &str, later_iso: &str) -> i64 {
    use chrono::NaiveDateTime;
    const FMT: &str = "%Y-%m-%dT%H:%M:%SZ";
    match (
        NaiveDateTime::parse_from_str(earlier_iso, FMT),
        NaiveDateTime::parse_from_str(later_iso, FMT),
    ) {
        (Ok(a), Ok(b)) => (b - a).num_seconds(),
        _ => i64::MAX,
    }
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
                // A delta mutates the set, so any snapshot fingerprint recorded on
                // the cursor is now stale — clear it (None) so the next snapshot
                // re-applies and re-establishes the fingerprint rather than being
                // wrongly deduped against a pre-delta set.
                upsert_cursor(conn, peer_id, sequence, emitted_at, None)?;
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

/// Stamp a row recording that *this* peer holds these bytes locally, with
/// both the legacy SHA256 address and (optionally) the BLAKE3 address under
/// which the same bytes were written to `IrohBlobStore`.
///
/// Used by the seeder dual-write path (`handle_put_blob`). Idempotent via
/// `replace_into` on the existing `(peer_id, blob_hash)` PK. Does NOT touch
/// the cursor — self-custody is local evidence, not a gossip arrival.
///
/// When `blake3_hash` is `None` (libp2p-only deployment, or the iroh write
/// failed), the row is still stamped — the SHA256 path succeeded and the peer
/// holds the bytes; the blake3 column stays NULL until backfill or a future
/// re-seed with iroh enabled.
///
/// Category C operational projection — rebuildable from the local `BlobStore`
/// filesystem scan via the `backfill-blake3` binary.
pub fn record_self_custody(
    conn: &mut SqliteConnection,
    peer_id: &str,
    blob_hash: &str,
    blake3_hash: Option<&str>,
    observed_at: &str,
) -> Result<(), StorageError> {
    let row = NewPeerBlobInventoryRow {
        peer_id: peer_id.to_string(),
        blob_hash: blob_hash.to_string(),
        last_seen_at: observed_at.to_string(),
        source: "self-custody".to_string(),
        sequence: 0,
        blake3_hash: blake3_hash.map(|s| s.to_string()),
    };
    diesel::replace_into(peer_blob_inventory::table)
        .values(&row)
        .execute(conn)
        .map(|_| ())
        .map_err(|e| StorageError::Database(format!("record_self_custody: {e}")))
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
        // Explicit select so adding columns to the table (e.g.
        // transport_affinity) does not break this load by column count.
        .select(PeerBlobInventoryRow::as_select())
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

/// Reverse-lookup the BLAKE3 alias for a SHA256-addressed blob. Returns
/// `None` when no row in `peer_blob_inventory` has a populated
/// `blake3_hash` for this `blob_hash`. Used by the HTTP `/blob/{hash}`
/// router to decide whether the iroh path is reachable for a given
/// SHA256 caller-supplied address.
///
/// Picks the first non-NULL `blake3_hash` observed across peers; the
/// schema's content-addressed identity guarantees all rows agree when
/// any agree.
pub fn lookup_blake3_for_sha256(
    conn: &mut SqliteConnection,
    sha256_hash: &str,
) -> Result<Option<String>, StorageError> {
    use peer_blob_inventory::dsl;
    dsl::peer_blob_inventory
        .filter(dsl::blob_hash.eq(sha256_hash))
        .filter(dsl::blake3_hash.is_not_null())
        .select(dsl::blake3_hash)
        .first::<Option<String>>(conn)
        .optional()
        .map(|opt_opt| opt_opt.flatten())
        .map_err(|e| StorageError::Database(format!("lookup_blake3_for_sha256: {e}")))
}

/// Reverse-lookup the SHA256 alias for a BLAKE3-addressed blob. Mirror
/// of [`lookup_blake3_for_sha256`]. Used by the libp2p-fallback path
/// when the caller supplied a `blake3-` prefixed address but the chosen
/// transport is libp2p (no Iroh manifest).
pub fn lookup_sha256_for_blake3(
    conn: &mut SqliteConnection,
    blake3_hash: &str,
) -> Result<Option<String>, StorageError> {
    use peer_blob_inventory::dsl;
    dsl::peer_blob_inventory
        .filter(dsl::blake3_hash.eq(blake3_hash))
        .select(dsl::blob_hash)
        .first::<String>(conn)
        .optional()
        .map_err(|e| StorageError::Database(format!("lookup_sha256_for_blake3: {e}")))
}

/// Look up the per-object `transport_affinity` for a blob addressed by its
/// normalized hash (either `sha256-{hex}` via `blob_hash`, or
/// `blake3-{hex}` via `blake3_hash`). Returns the first non-NULL value
/// observed across peers; content-addressed identity guarantees rows agree.
/// `None` when no row sets it (→ caller maps to `TransportAffinity::Auto`).
pub fn lookup_transport_affinity(
    conn: &mut SqliteConnection,
    normalized_hash: &str,
) -> Result<Option<String>, StorageError> {
    use peer_blob_inventory::dsl;
    let by_column = if normalized_hash.starts_with("blake3-") {
        dsl::peer_blob_inventory
            .filter(dsl::blake3_hash.eq(normalized_hash))
            .filter(dsl::transport_affinity.is_not_null())
            .select(dsl::transport_affinity)
            .first::<Option<String>>(conn)
    } else {
        dsl::peer_blob_inventory
            .filter(dsl::blob_hash.eq(normalized_hash))
            .filter(dsl::transport_affinity.is_not_null())
            .select(dsl::transport_affinity)
            .first::<Option<String>>(conn)
    };
    by_column
        .optional()
        .map(|opt_opt| opt_opt.flatten())
        .map_err(|e| StorageError::Database(format!("lookup_transport_affinity: {e}")))
}

/// Set the per-object `transport_affinity` for all `peer_blob_inventory`
/// rows that address this blob (matching `blob_hash` OR `blake3_hash`).
/// Used by the operator setter route. `None` clears the affinity back to
/// `Auto` (default policy). Returns the number of rows updated.
pub fn set_transport_affinity(
    conn: &mut SqliteConnection,
    normalized_hash: &str,
    affinity: Option<&str>,
) -> Result<usize, StorageError> {
    use peer_blob_inventory::dsl;
    let updated = if normalized_hash.starts_with("blake3-") {
        diesel::update(dsl::peer_blob_inventory.filter(dsl::blake3_hash.eq(normalized_hash)))
            .set(dsl::transport_affinity.eq(affinity))
            .execute(conn)
    } else {
        diesel::update(dsl::peer_blob_inventory.filter(dsl::blob_hash.eq(normalized_hash)))
            .set(dsl::transport_affinity.eq(affinity))
            .execute(conn)
    };
    updated.map_err(|e| StorageError::Database(format!("set_transport_affinity: {e}")))
}

/// Outcome of `apply_snapshot`. The caller uses this to skip the expensive
/// commitment re-scoring (`score_and_enqueue_snapshot`) on a deduplicated echo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotApplyOutcome {
    /// Content changed (or first snapshot): the peer's entries were replaced.
    Applied,
    /// Byte-identical to the last-applied set: no set churn, no re-score needed.
    Deduplicated,
}

/// How stale the freshness clock may get before a deduplicated snapshot triggers
/// a cheap `last_seen_at` touch. Kept well under the inventory freshness window
/// (default 600s) so a still-announced static inventory never falls out of
/// `lookup_hosts`, while a re-delivery storm touches rows at most once per this
/// interval instead of per message.
const DEDUP_FRESHNESS_REFRESH_SECS: i64 = 30;

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
    last_content_hash: Option<&str>,
) -> Result<(), diesel::result::Error> {
    let row = NewPeerInventoryCursorRow {
        peer_id: peer_id.to_string(),
        last_sequence: sequence,
        last_updated: last_updated.to_string(),
        last_content_hash: last_content_hash.map(|s| s.to_string()),
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

    // ── Snapshot idempotency (convergence under WAN re-delivery) ─────────────

    #[test]
    fn content_fingerprint_is_order_independent() {
        // Same set, any order → same fingerprint.
        assert_eq!(
            content_fingerprint(&["a".into(), "b".into()]),
            content_fingerprint(&["b".into(), "a".into()]),
        );
        // Different set → different fingerprint.
        assert_ne!(
            content_fingerprint(&["a".into(), "b".into()]),
            content_fingerprint(&["a".into(), "c".into()]),
        );
        // Length-prefixing guards against set-boundary collisions.
        assert_ne!(
            content_fingerprint(&["ab".into(), "c".into()]),
            content_fingerprint(&["a".into(), "bc".into()]),
        );
    }

    #[test]
    fn snapshot_identical_content_is_deduplicated() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        // First snapshot applies.
        assert_eq!(
            apply_snapshot(
                &mut conn,
                "peer_I",
                &["h1".into(), "h2".into()],
                1,
                "2026-05-02T00:00:00Z"
            )
            .unwrap(),
            SnapshotApplyOutcome::Applied
        );
        // The publisher bumps `sequence` every tick even with unchanged content —
        // an identical set at a NEW sequence must dedup (not re-apply).
        assert_eq!(
            apply_snapshot(
                &mut conn,
                "peer_I",
                &["h1".into(), "h2".into()],
                2,
                "2026-05-02T00:00:10Z"
            )
            .unwrap(),
            SnapshotApplyOutcome::Deduplicated
        );

        // No duplicate rows; the set is intact.
        assert_eq!(
            lookup_hosts(&mut conn, "h1", "2026-05-01T00:00:00Z")
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            lookup_hosts(&mut conn, "h2", "2026-05-01T00:00:00Z")
                .unwrap()
                .len(),
            1
        );
        // Cursor sequence still advanced (delta-ordering stays contiguous).
        assert_eq!(read_cursor_sequence(&mut conn, "peer_I").unwrap(), Some(2));
    }

    #[test]
    fn snapshot_reordered_identical_content_is_deduplicated() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        apply_snapshot(
            &mut conn,
            "peer_O",
            &["h1".into(), "h2".into()],
            1,
            "2026-05-02T00:00:00Z",
        )
        .unwrap();
        // Same set, reordered on the wire → still a no-op.
        assert_eq!(
            apply_snapshot(
                &mut conn,
                "peer_O",
                &["h2".into(), "h1".into()],
                2,
                "2026-05-02T00:00:10Z"
            )
            .unwrap(),
            SnapshotApplyOutcome::Deduplicated
        );
    }

    #[test]
    fn snapshot_different_content_same_sequence_still_applies() {
        // Preserves the deliberate "accepted regardless of sequence" attack-recovery
        // path: a genuinely different recovery snapshot at a REPEATED sequence must
        // still apply, because dedup keys on content, not (peer_id, sequence).
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        apply_snapshot(
            &mut conn,
            "peer_S",
            &["h1".into(), "h2".into()],
            5,
            "2026-05-02T00:00:00Z",
        )
        .unwrap();
        assert_eq!(
            apply_snapshot(
                &mut conn,
                "peer_S",
                &["h3".into(), "h4".into()],
                5,
                "2026-05-02T00:00:10Z"
            )
            .unwrap(),
            SnapshotApplyOutcome::Applied
        );

        assert!(lookup_hosts(&mut conn, "h1", "2026-05-01T00:00:00Z")
            .unwrap()
            .is_empty());
        assert_eq!(
            lookup_hosts(&mut conn, "h3", "2026-05-01T00:00:00Z")
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn snapshot_dedup_advances_cursor_for_delta_ordering() {
        // A deduplicated snapshot must still advance the watermark, or a following
        // in-order delta is misread as a gap and triggers a needless snapshot-request.
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        apply_snapshot(
            &mut conn,
            "peer_DO",
            &["h1".into()],
            1,
            "2026-05-02T00:00:00Z",
        )
        .unwrap();
        assert_eq!(
            apply_snapshot(
                &mut conn,
                "peer_DO",
                &["h1".into()],
                2,
                "2026-05-02T00:00:10Z"
            )
            .unwrap(),
            SnapshotApplyOutcome::Deduplicated
        );
        // Delta at sequence 3 is contiguous with the deduped snapshot@2 → Applied,
        // not Gap. Without the watermark advance the cursor would be stuck at 1.
        let outcome = apply_delta(
            &mut conn,
            "peer_DO",
            &["h2".into()],
            &[],
            3,
            "2026-05-02T00:00:20Z",
        )
        .unwrap();
        assert_eq!(outcome, DeltaApplyOutcome::Applied);
        assert_eq!(
            lookup_hosts(&mut conn, "h2", "2026-05-01T00:00:00Z")
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn snapshot_dedup_refreshes_freshness_after_interval() {
        // A static inventory that only re-snapshots must not fall out of the
        // freshness window: a dedup beyond the refresh interval touches last_seen_at.
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        apply_snapshot(
            &mut conn,
            "peer_F1",
            &["h1".into()],
            1,
            "2026-05-02T00:00:00Z",
        )
        .unwrap();
        // 60s later (> DEDUP_FRESHNESS_REFRESH_SECS=30) → dedup + refresh.
        assert_eq!(
            apply_snapshot(
                &mut conn,
                "peer_F1",
                &["h1".into()],
                2,
                "2026-05-02T00:01:00Z"
            )
            .unwrap(),
            SnapshotApplyOutcome::Deduplicated
        );

        // With a fresh_after between the two timestamps, the row is only returned
        // if last_seen_at was refreshed to the second snapshot's time.
        let rows = lookup_hosts(&mut conn, "h1", "2026-05-02T00:00:30Z").unwrap();
        assert_eq!(
            rows.len(),
            1,
            "refreshed row must survive the freshness filter"
        );
        assert_eq!(rows[0].last_seen_at, "2026-05-02T00:01:00Z");
    }

    #[test]
    fn snapshot_dedup_rate_limits_freshness_touch() {
        // Under a storm the freshness touch is rate-limited: a dedup WITHIN the
        // refresh interval leaves last_seen_at untouched (no per-message row rewrite).
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        apply_snapshot(
            &mut conn,
            "peer_F2",
            &["h1".into()],
            1,
            "2026-05-02T00:00:00Z",
        )
        .unwrap();
        // Refresh at 00:01:00 (60s > 30).
        apply_snapshot(
            &mut conn,
            "peer_F2",
            &["h1".into()],
            2,
            "2026-05-02T00:01:00Z",
        )
        .unwrap();
        // 10s after the last refresh (< 30) → dedup, NO refresh.
        assert_eq!(
            apply_snapshot(
                &mut conn,
                "peer_F2",
                &["h1".into()],
                3,
                "2026-05-02T00:01:10Z"
            )
            .unwrap(),
            SnapshotApplyOutcome::Deduplicated
        );

        let rows = lookup_hosts(&mut conn, "h1", "2026-05-01T00:00:00Z").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0].last_seen_at, "2026-05-02T00:01:00Z",
            "within-interval dedup must not rewrite last_seen_at"
        );
        // Sequence still advances even when freshness is not refreshed.
        assert_eq!(read_cursor_sequence(&mut conn, "peer_F2").unwrap(), Some(3));
    }

    #[test]
    fn snapshot_after_delta_reapplies_not_deduped() {
        // A delta clears the snapshot fingerprint, so the next snapshot re-applies
        // (re-establishing the fingerprint) rather than wrongly deduping against a
        // pre-delta set.
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        apply_snapshot(
            &mut conn,
            "peer_AD",
            &["h1".into()],
            1,
            "2026-05-02T00:00:00Z",
        )
        .unwrap();
        apply_delta(
            &mut conn,
            "peer_AD",
            &["h2".into()],
            &[],
            2,
            "2026-05-02T00:00:10Z",
        )
        .unwrap();
        // Snapshot reflecting the post-delta set must Apply (fingerprint was cleared).
        assert_eq!(
            apply_snapshot(
                &mut conn,
                "peer_AD",
                &["h1".into(), "h2".into()],
                3,
                "2026-05-02T00:00:20Z"
            )
            .unwrap(),
            SnapshotApplyOutcome::Applied
        );
        // And an identical one after it dedups.
        assert_eq!(
            apply_snapshot(
                &mut conn,
                "peer_AD",
                &["h1".into(), "h2".into()],
                4,
                "2026-05-02T00:00:30Z"
            )
            .unwrap(),
            SnapshotApplyOutcome::Deduplicated
        );
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

    #[test]
    fn lookup_blake3_for_sha256_returns_some_when_present() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        diesel::insert_into(peer_blob_inventory::table)
            .values((
                peer_blob_inventory::peer_id.eq("peer-A"),
                peer_blob_inventory::blob_hash.eq("sha256-aaaa"),
                peer_blob_inventory::blake3_hash.eq(Some("blake3-bbbb")),
                peer_blob_inventory::source.eq("gossip-snapshot"),
                peer_blob_inventory::sequence.eq(0i64),
                peer_blob_inventory::last_seen_at.eq("2026-05-10T00:00:00Z"),
            ))
            .execute(&mut conn)
            .unwrap();

        let got = lookup_blake3_for_sha256(&mut conn, "sha256-aaaa").unwrap();
        assert_eq!(got, Some("blake3-bbbb".to_string()));
    }

    #[test]
    fn lookup_blake3_for_sha256_returns_none_when_absent() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let got = lookup_blake3_for_sha256(&mut conn, "sha256-absent").unwrap();
        assert_eq!(got, None);
    }

    #[test]
    fn lookup_sha256_for_blake3_returns_some_when_present() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        diesel::insert_into(peer_blob_inventory::table)
            .values((
                peer_blob_inventory::peer_id.eq("peer-A"),
                peer_blob_inventory::blob_hash.eq("sha256-aaaa"),
                peer_blob_inventory::blake3_hash.eq(Some("blake3-bbbb")),
                peer_blob_inventory::source.eq("gossip-snapshot"),
                peer_blob_inventory::sequence.eq(0i64),
                peer_blob_inventory::last_seen_at.eq("2026-05-10T00:00:00Z"),
            ))
            .execute(&mut conn)
            .unwrap();

        let got = lookup_sha256_for_blake3(&mut conn, "blake3-bbbb").unwrap();
        assert_eq!(got, Some("sha256-aaaa".to_string()));
    }

    // ── record_self_custody — cutover gate #3 ───────────────────────────────

    #[test]
    fn record_self_custody_writes_both_hashes() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        record_self_custody(
            &mut conn,
            "peer_self",
            "sha256-aa00",
            Some("blake3-bb00"),
            "2026-05-10T00:00:00Z",
        )
        .unwrap();

        let rows = lookup_hosts(&mut conn, "sha256-aa00", "2026-05-09T00:00:00Z").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].peer_id, "peer_self");
        assert_eq!(rows[0].source, "self-custody");
        assert_eq!(rows[0].blake3_hash.as_deref(), Some("blake3-bb00"));
    }

    #[test]
    fn record_self_custody_is_idempotent() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        for _ in 0..3 {
            record_self_custody(
                &mut conn,
                "peer_self",
                "sha256-aa00",
                Some("blake3-bb00"),
                "2026-05-10T00:00:00Z",
            )
            .unwrap();
        }
        let rows = lookup_hosts(&mut conn, "sha256-aa00", "2026-05-09T00:00:00Z").unwrap();
        assert_eq!(rows.len(), 1, "re-seed must not duplicate inventory rows");
    }

    #[test]
    fn record_self_custody_accepts_null_blake3() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        // libp2p-only deployment: iroh side unavailable, blake3 is None.
        record_self_custody(
            &mut conn,
            "peer_self",
            "sha256-aa00",
            None,
            "2026-05-10T00:00:00Z",
        )
        .unwrap();

        let rows = lookup_hosts(&mut conn, "sha256-aa00", "2026-05-09T00:00:00Z").unwrap();
        assert_eq!(rows.len(), 1);
        assert!(rows[0].blake3_hash.is_none());
    }
}
