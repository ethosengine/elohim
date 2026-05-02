//! Shared blob-fetch helper. Used by both:
//! - HTTP blob handler (`GET /blob/{hash}`) on local miss
//! - Custody reconciliation controller (T16) for own-commitment kicks
//!
//! Strategy:
//! 1. Look up candidate peers in `peer_blob_inventory`, ordered by evidence
//!    strength (fetch-success first, then by recency).
//! 2. Filter to currently-connected peers.
//! 3. Race candidates in parallel batches of `fetch_blob_parallelism` (default 3).
//!    First reply that returns `Ok(bytes)` AND verifies the content hash wins.
//!    Pending replies in the batch are dropped; failed batch advances to the next.
//! 4. On verified success: persist locally, record fetch-success in
//!    `peer_blob_inventory`, emit `serve-blob` REA event.
//!
//! Hash verification: sha256-hex matches the requested hash (case-insensitive).
//!
//! # Stage 1 note
//! `P2PCommand::FetchBlob` is a Stage 1 stub that always returns
//! `Err("FetchBlob not yet implemented; Stage 1 placeholder")`.  That means
//! `race_fetch` will always yield `Miss` in a live swarm until Stage 2 wires
//! the command to a real request-response protocol.  The helper's control
//! flow, hash verification, persistence, and serve-blob emission are verified
//! by unit tests without requiring a running swarm.

use crate::blob_store::BlobStore;
use crate::config::Config;
use crate::db::diesel_schema::economic_events;
use crate::db::models::NewEconomicEvent;
use crate::db::peer_blob_inventory::record_fetch_success;
use crate::error::StorageError;
use chrono::Utc;
use diesel::Connection;
use diesel::RunQueryDsl;
use diesel::SqliteConnection;
use futures::stream::{FuturesUnordered, StreamExt};
use sha2::{Digest, Sha256};
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};

/// Outcome of a race-fetch.
#[derive(Debug)]
pub enum FetchOutcome {
    /// Bytes fetched and verified.
    Hit { bytes: Vec<u8>, source_peer: String },
    /// All candidates exhausted; no peer served verified bytes.
    Miss,
    /// Inventory had no candidates to try (either empty or none connected).
    NoCandidates,
}

/// Race a fetch across the candidates known to host the blob.
/// Returns the verified bytes on first hit, or Miss/NoCandidates if no peer served.
///
/// `cmd_tx` is the swarm command channel.  `is_connected` filters the
/// candidate list to peers actively connected at call time.
///
/// Candidates are processed in batches of `parallelism`.  All peers in a
/// batch are attempted concurrently; the first that returns verified bytes
/// wins and remaining in-flight requests are dropped.  If the whole batch
/// fails, the next batch is tried.
pub async fn race_fetch(
    blob_hash: &str,
    candidates: Vec<String>,
    cmd_tx: &mpsc::Sender<crate::p2p::P2PCommand>,
    is_connected: impl Fn(&str) -> bool,
    parallelism: usize,
    per_peer_timeout: Duration,
) -> FetchOutcome {
    let connected: Vec<String> = candidates.into_iter().filter(|p| is_connected(p)).collect();

    if connected.is_empty() {
        return FetchOutcome::NoCandidates;
    }

    let mut iter = connected.into_iter();
    loop {
        let batch: Vec<String> = iter.by_ref().take(parallelism).collect();
        if batch.is_empty() {
            return FetchOutcome::Miss;
        }

        // Spawn a per-peer fetch task into a FuturesUnordered for
        // first-responder semantics: as soon as any peer returns verified
        // bytes we return Hit and stop awaiting the rest of the batch.
        //
        // Note: `tokio::spawn` detaches the task — dropping the JoinHandle
        // does NOT abort it. The spawned future runs to completion (bounded
        // by the per_peer_timeout in `tokio::time::timeout`). At Stage 2 with
        // real network I/O, replace `tokio::spawn` with unboxed futures
        // inside `FuturesUnordered` so drop = cancel for true cooperative
        // cancellation.
        let mut in_flight: FuturesUnordered<_> = FuturesUnordered::new();
        for peer_id_str in batch {
            let Ok(peer_id) = peer_id_str.parse::<libp2p::PeerId>() else {
                continue;
            };
            let cmd_tx = cmd_tx.clone();
            let hash = blob_hash.to_string();
            let timeout = per_peer_timeout;
            let peer_label = peer_id_str.clone();
            in_flight.push(tokio::spawn(async move {
                let (reply_tx, reply_rx) = oneshot::channel();
                if cmd_tx
                    .send(crate::p2p::P2PCommand::FetchBlob {
                        peer_id,
                        hash: hash.clone(),
                        reply: reply_tx,
                    })
                    .await
                    .is_err()
                {
                    return (peer_label, Err("swarm channel closed".to_string()));
                }
                match tokio::time::timeout(timeout, reply_rx).await {
                    Ok(Ok(Ok(bytes))) => (peer_label, Ok(bytes)),
                    Ok(Ok(Err(e))) => (peer_label, Err(e)),
                    Ok(Err(_)) => (peer_label, Err("oneshot dropped".to_string())),
                    Err(_) => (peer_label, Err("timeout".to_string())),
                }
            }));
        }

        // Drain in completion order; first verified hit wins. Mismatches and
        // errors are skipped — keep polling the remaining futures.
        while let Some(joined) = in_flight.next().await {
            if let Ok((peer, Ok(bytes))) = joined {
                if verify_blob_hash(&bytes, blob_hash) {
                    // Returning here drops `in_flight`, which stops awaiting
                    // the remaining JoinHandles. The detached tokio tasks
                    // continue running to completion (bounded by
                    // per_peer_timeout) — see Stage 2 note on the spawn
                    // block above.
                    return FetchOutcome::Hit {
                        bytes,
                        source_peer: peer,
                    };
                }
            }
        }
        // All in batch failed (timeout, error, or hash mismatch); try next batch.
    }
}

/// Persist verified bytes to the local blob store, record fetch-success in
/// `peer_blob_inventory`, and emit a `serve-blob` REA event.
///
/// **Ordering contract — persist filesystem first, then SQL.**
/// A crash between filesystem and SQL leaves a benign orphan blob on disk;
/// the parity sweep (T18, `/api/v1/diagnostics/inventory-parity`) reconciles
/// by triggering a re-fetch on the next gossip round. The opposite order
/// (SQL first) would create the worse failure mode: an inventory row
/// claiming we host a blob whose bytes we never managed to write.
///
/// **Atomicity contract — the two SQL writes are wrapped in a single
/// transaction.** `record_fetch_success` and the `serve-blob` event insert
/// run inside one `conn.transaction(|txn| ...)` block so a crash or
/// constraint-violation between them leaves no half-state. (SQLite via
/// Diesel emits a SAVEPOINT for the inner `record_fetch_success` call,
/// which already opens its own transaction; nested transactions on
/// SqliteConnection are supported and roll back together on outer error.)
/// The T18 parity sweep can then assume: if a `peer_blob_inventory` row
/// with `source='fetch-success'` exists for `(self, hash)`, the matching
/// `serve-blob` REA event also exists.
///
/// Sequence inside this function:
/// 1. `blob_store.store(bytes).await` — write bytes to local filesystem.
/// 2. `conn.transaction(|txn| { record_fetch_success(txn, ...); insert serve-blob event })`.
pub async fn finalize_fetch_success(
    conn: &mut SqliteConnection,
    blob_hash: &str,
    source_peer: &str,
    bytes: &[u8],
    self_cid: &str,
    blob_store: &BlobStore,
) -> Result<(), StorageError> {
    // Step 1: persist to filesystem first. On error, return without writing
    // any SQL — leaving inventory clean.
    blob_store.store(bytes).await?;

    let now_iso = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let event_id = uuid::Uuid::new_v4().to_string();
    // TODO(stewardship-precision): `economic_events.resource_quantity_value`
    // is f32 in the schema, which loses precision for blobs >16 MB.
    // Migrating to f64 (or fixed-point) requires a Diesel migration; tracked
    // separately from this T20 review pass.
    let qty_value = bytes.len() as f32;

    // Step 2 + 3 (atomic): record fetch-success AND emit the serve-blob REA
    // event in a single transaction so a crash between them cannot leave a
    // peer_blob_inventory row without its matching event (or vice versa).
    //
    // REA event direction: a remote peer SERVED the bytes to us, so the remote
    // peer is the provider (source_peer) and we are the receiver (self_cid).
    // Note: T16's custody-blob events use the opposite convention (custodian
    // is provider, content steward is receiver). Cross-event analytics joining
    // on these fields must be aware of the per-action semantics.
    conn.transaction::<(), StorageError, _>(|txn| {
        record_fetch_success(txn, source_peer, blob_hash, &now_iso)?;

        let new_event = NewEconomicEvent {
            id: &event_id,
            h_app_id: "elohim",
            action: "serve-blob",
            provider: source_peer,
            receiver: self_cid,
            resource_conforms_to: None,
            resource_inventoried_as: Some(blob_hash),
            resource_classified_as_json: None,
            resource_quantity_value: Some(qty_value),
            resource_quantity_unit: Some("bytes"),
            effort_quantity_value: None,
            effort_quantity_unit: None,
            has_point_in_time: &now_iso,
            has_duration: None,
            input_of: None,
            // The matching custody-blob commitment hash is not always known at this layer.
            output_of: None,
            lamad_event_type: None,
            content_id: None,
            contributor_presence_id: None,
            path_id: None,
            triggered_by: None,
            state: "completed",
            note: None,
            metadata_json: None,
            dht_anchor_hash: None,
            at_location: None,
            verified_at: None,
        };
        diesel::insert_into(economic_events::table)
            .values(&new_event)
            .execute(txn)
            .map_err(|e| StorageError::Database(format!("insert serve-blob event: {e}")))?;
        Ok(())
    })
}

/// Verify that `bytes` has the sha256 hex digest matching `expected_hex`.
/// Comparison is case-insensitive (both sides lowercased).
///
/// Accepts both raw hex (`"a7ffc6f8..."`) and the canonical `"sha256-<hex>"`
/// form used at the HTTP boundary. The `sha256-` prefix is stripped before
/// comparison; without this, callers that normalize to the prefixed form
/// (see `http.rs` `handle_get_blob`) would always see a hash mismatch and
/// silently return `FetchOutcome::Miss`.
pub fn verify_blob_hash(bytes: &[u8], expected_hex: &str) -> bool {
    let hex_part = expected_hex.strip_prefix("sha256-").unwrap_or(expected_hex);
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let actual_hex = hex::encode(hasher.finalize());
    actual_hex == hex_part.to_lowercase()
}

/// Extract race-fetch parameters from the runtime `Config`.
pub fn fetch_params_from_config(config: &Config) -> (usize, Duration) {
    (
        config.fetch_blob_parallelism,
        Duration::from_secs(config.fetch_blob_timeout_seconds),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn known_blob() -> (Vec<u8>, String) {
        let bytes = b"hello world".to_vec();
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let hex = hex::encode(hasher.finalize());
        (bytes, hex)
    }

    #[test]
    fn verify_blob_hash_accepts_match() {
        let (bytes, hex) = known_blob();
        assert!(verify_blob_hash(&bytes, &hex));
    }

    #[test]
    fn verify_blob_hash_rejects_mismatch() {
        let (bytes, _) = known_blob();
        assert!(!verify_blob_hash(&bytes, &"a".repeat(64)));
    }

    #[test]
    fn verify_blob_hash_handles_uppercase_expected() {
        let (bytes, hex) = known_blob();
        let upper = hex.to_uppercase();
        assert!(verify_blob_hash(&bytes, &upper));
    }

    /// T19 review Fix #1 regression: the call site in `http.rs::race_fetch`
    /// passes the canonical `"sha256-<64hex>"` form (set by
    /// `handle_get_blob`'s `normalized_hash`), not the raw hex. Prior to
    /// the fix, `verify_blob_hash` compared raw-hex against prefixed-hex
    /// and always returned false, causing every Stage 2 race-fetch with
    /// real bytes to silently degrade to `FetchOutcome::Miss`.
    #[test]
    fn verify_blob_hash_handles_sha256_prefix() {
        let (bytes, hex) = known_blob();
        let prefixed = format!("sha256-{}", hex);
        assert!(
            verify_blob_hash(&bytes, &prefixed),
            "verify_blob_hash must accept canonical 'sha256-<hex>' form"
        );
    }

    /// T19 Fix #2 regression: race-fetch must use FuturesUnordered so that
    /// the FIRST future to finish wins, regardless of how late other futures
    /// might complete. Prior implementation iterated `for handle in handles`
    /// which awaited them in spawn-order — a slow first peer would make the
    /// whole batch wait on it before noticing a fast second peer.
    ///
    /// This test models the await semantics directly: we spawn three tasks
    /// with staggered completion, push them into FuturesUnordered, drain via
    /// `.next().await`, and assert the FIRST one off the stream is the
    /// fastest one — proving "first responder wins" wiring is in place.
    #[tokio::test]
    async fn race_fetch_first_responder_wins() {
        use std::time::Duration;

        let mut fu: FuturesUnordered<_> = FuturesUnordered::new();
        // Slow peer (~400 ms).
        fu.push(tokio::spawn(async {
            tokio::time::sleep(Duration::from_millis(400)).await;
            "slow"
        }));
        // Medium peer (~200 ms).
        fu.push(tokio::spawn(async {
            tokio::time::sleep(Duration::from_millis(200)).await;
            "medium"
        }));
        // Fast peer (~20 ms).
        fu.push(tokio::spawn(async {
            tokio::time::sleep(Duration::from_millis(20)).await;
            "fast"
        }));

        let first = fu.next().await.expect("at least one future").unwrap();
        assert_eq!(
            first, "fast",
            "FuturesUnordered must yield the fastest future first; \
             sequential .await would yield 'slow' (spawn-order head)"
        );
    }

    /// T20 contract test: `finalize_fetch_success` must persist the bytes
    /// to the BlobStore FIRST, then write the `peer_blob_inventory` row
    /// (source = `fetch-success`), then emit exactly one `serve-blob`
    /// REA `economic_events` row. The persist-first ordering is what
    /// makes the T18 parity sweep correct: a crash mid-finalize leaves
    /// a benign orphan blob on disk that the sweep reconciles via
    /// re-fetch — never an inventory row pointing at bytes we don't
    /// actually have.
    #[tokio::test]
    async fn finalize_persists_bytes_then_writes_sql() {
        use crate::blob_store::BlobStore;
        use crate::db::peer_blob_inventory::lookup_hosts;
        use crate::test_util::test_pool;

        let blob_store = BlobStore::new_memory();
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        let bytes = b"finalize-test-payload".to_vec();
        let hash = {
            let mut h = Sha256::new();
            h.update(&bytes);
            format!("sha256-{}", hex::encode(h.finalize()))
        };

        finalize_fetch_success(
            &mut conn,
            &hash,
            "peer_X",
            &bytes,
            "self_cid_Y",
            &blob_store,
        )
        .await
        .expect("finalize ok");

        // Filesystem first: BlobStore filenames are canonical
        // `sha256-<hex>`, so probe with the prefixed form.
        assert!(
            blob_store.exists(&hash).await,
            "blob persisted to filesystem before SQL writes"
        );

        // SQL second: peer_blob_inventory row written with source='fetch-success'.
        // Use a fresh_after cutoff well in the past so the lookup window
        // includes our just-written row.
        let rows = lookup_hosts(&mut conn, &hash, "2020-01-01T00:00:00Z").unwrap();
        assert!(
            rows.iter()
                .any(|r| r.peer_id == "peer_X" && r.source == "fetch-success"),
            "peer_blob_inventory row written with source='fetch-success'; \
             got rows: {rows:?}"
        );

        // SQL third: exactly one serve-blob REA event present, scoped to
        // this blob hash.
        use crate::db::diesel_schema::economic_events::dsl as ee;
        use diesel::prelude::*;
        let count: i64 = ee::economic_events
            .filter(ee::action.eq("serve-blob"))
            .filter(ee::resource_inventoried_as.eq(&hash))
            .count()
            .get_result(&mut conn)
            .unwrap();
        assert_eq!(count, 1, "exactly one serve-blob event");
    }

    /// T20 review Fix #2 regression: the two SQL writes inside
    /// `finalize_fetch_success` (record_fetch_success + serve-blob event
    /// insert) MUST be wrapped in a single transaction so a failure in the
    /// serve-blob insert rolls back the peer_blob_inventory row that was
    /// just written. Otherwise a crash between them leaves an inventory row
    /// with `source='fetch-success'` that the T18 parity sweep cannot
    /// detect (because the row says we host the blob and the filesystem
    /// agrees) but no matching REA event exists.
    ///
    /// We induce a SQL failure cheaply by dropping the `economic_events`
    /// table BEFORE calling finalize. The first SQL statement
    /// (record_fetch_success on peer_blob_inventory) succeeds, but the
    /// second (insert into economic_events) fails because the table is
    /// missing. With the transaction wrap in place, the inventory row must
    /// also be rolled back.
    #[tokio::test]
    async fn finalize_rolls_back_inventory_on_event_insert_failure() {
        use crate::blob_store::BlobStore;
        use crate::db::peer_blob_inventory::lookup_hosts;
        use crate::test_util::test_pool;
        use diesel::RunQueryDsl;

        let blob_store = BlobStore::new_memory();
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        let bytes = b"rollback-test-payload".to_vec();
        let hash = {
            let mut h = Sha256::new();
            h.update(&bytes);
            format!("sha256-{}", hex::encode(h.finalize()))
        };

        // Force the second SQL write to fail by dropping its target table.
        diesel::sql_query("DROP TABLE economic_events;")
            .execute(&mut conn)
            .expect("drop economic_events");

        let result = finalize_fetch_success(
            &mut conn,
            &hash,
            "peer_R",
            &bytes,
            "self_cid_R",
            &blob_store,
        )
        .await;
        assert!(
            result.is_err(),
            "finalize must fail when economic_events insert fails"
        );

        // The filesystem write happened first (and is intentionally NOT
        // rolled back; the parity sweep handles orphan blobs).
        assert!(
            blob_store.exists(&hash).await,
            "blob persisted to filesystem before SQL transaction"
        );

        // Critical assertion: the peer_blob_inventory row must NOT exist,
        // because the wrapping transaction rolled it back when the
        // serve-blob insert failed.
        let rows = lookup_hosts(&mut conn, &hash, "2020-01-01T00:00:00Z").unwrap();
        assert!(
            !rows.iter().any(|r| r.peer_id == "peer_R"),
            "peer_blob_inventory row must be rolled back when serve-blob \
             insert fails; got rows: {rows:?}"
        );
    }
}
