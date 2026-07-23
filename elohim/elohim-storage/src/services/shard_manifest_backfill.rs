//! One-shot startup pass: shard-manifest backfill for legacy content.
//!
//! Content seeded BEFORE the distribute-at-ingest path existed has a `blob_hash`
//! (and bytes in the blob store) but no `shard_manifests` row. Without a manifest
//! the resilience snapshot (`services::household_resilience::snapshot`) reads that
//! content as never-distributed (`distributionState: "unmeasured"`) and
//! `shard_locations` never populates → "Stewarding collectives: 0".
//!
//! This backfill closes that gap without a re-seed:
//!   1. Find content rows with a non-empty `blob_hash` and NO matching
//!      `shard_manifests` row (for the given `h_app_id`).
//!   2. For each: load the blob, encode + persist a manifest via the shared
//!      `db::shard_manifests::record_manifest_from_bytes` helper (the same one
//!      the `POST /db/content` create path uses — single source of truth).
//!   3. When a p2p handle is available, also `distribute_shards` so shards fan
//!      out to peers (that's what writes `shard_locations` on receiving peers
//!      and lights "stewarding collectives").
//!
//! The pass is bounded (batched with a small inter-item delay — RS-encoding is
//! CPU, distribution is network) and tolerant: a content row whose `blob_hash`
//! is set but whose bytes are absent locally is skipped-and-warned, never fatal.
//!
//! Category C (operational): manifests are local projection state rebuilt from
//! the blob store, not DHT-notarized. Running this twice is idempotent — content
//! that gained a manifest on the first pass is no longer a candidate on the next.

use std::time::Duration;

use diesel::prelude::*;

use crate::blob_store::BlobStore;
use crate::db::DbPool;
use crate::StorageError;

/// A content row eligible for manifest backfill: has a blob, lacks a manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackfillCandidate {
    pub content_id: String,
    pub blob_hash: String,
    pub blob_cid: Option<String>,
    pub content_format: String,
    pub reach: String,
}

/// Tuning for a single backfill sweep.
#[derive(Debug, Clone)]
pub struct BackfillConfig {
    /// Items to process before yielding for `batch_pause`.
    pub batch_size: usize,
    /// Pause between items (RS-encode CPU + distribution network breathing room).
    pub item_delay: Duration,
    /// Pause after each batch.
    pub batch_pause: Duration,
}

impl Default for BackfillConfig {
    fn default() -> Self {
        Self {
            batch_size: 25,
            item_delay: Duration::from_millis(50),
            batch_pause: Duration::from_millis(500),
        }
    }
}

/// Outcome counters for a sweep (returned for logging/tests).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct BackfillReport {
    /// Candidates selected (had blob, lacked manifest).
    pub candidates: usize,
    /// Manifests successfully recorded.
    pub recorded: usize,
    /// Candidates skipped because the blob bytes were absent locally.
    pub missing_blob: usize,
    /// Candidates that errored during encode/persist (non-fatal, logged).
    pub failed: usize,
    /// Part B: content with a manifest but ZERO shard_locations selected for a
    /// re-distribution attempt (measured-but-never-distributed remediation).
    pub redistribute_candidates: usize,
    /// Part B: re-distribution ATTEMPTS made (not necessarily placements — a
    /// still-unresolvable peer set leaves shards unplaced; honest attempt count).
    pub redistribute_attempts: usize,
}

/// Part B cap: the most re-distribution attempts a single boot sweep will make.
/// Bounds the boot-time distribution work when the push-side identity fix
/// (slice 2) first lands and a backlog of dark content becomes distributable.
/// Excess candidates are picked up on subsequent boots (idempotent — a content
/// that gains shard_locations is no longer a candidate).
const REDISTRIBUTE_CAP_PER_BOOT: usize = 50;

/// Select content rows that have a blob but no shard manifest for `h_app_id`.
///
/// A row qualifies iff `content.blob_hash` is non-null AND non-empty AND there
/// is no `shard_manifests` row for `(content.id, h_app_id)`.
///
/// Pure query — no blob I/O, no encoding — so it is cheap to unit-test.
pub fn select_candidates(
    conn: &mut SqliteConnection,
    h_app_id: &str,
) -> Result<Vec<BackfillCandidate>, StorageError> {
    use crate::db::diesel_schema::{content, shard_manifests};

    // Tuple shape of the candidate projection:
    // (content_id, blob_hash, blob_cid, content_format, reach).
    type CandidateRow = (String, Option<String>, Option<String>, String, String);

    // Left-join content → its manifest for this h_app_id; keep rows where the
    // manifest side is NULL (no manifest) and the blob_hash is present.
    let rows: Vec<CandidateRow> = content::table
        .left_join(
            shard_manifests::table.on(shard_manifests::content_id
                .eq(content::id)
                .and(shard_manifests::h_app_id.eq(h_app_id))),
        )
        .filter(content::h_app_id.eq(h_app_id))
        .filter(content::blob_hash.is_not_null())
        .filter(shard_manifests::content_id.is_null())
        .select((
            content::id,
            content::blob_hash,
            content::blob_cid,
            content::content_format,
            content::reach,
        ))
        .load::<CandidateRow>(conn)
        .map_err(StorageError::from)?;

    let candidates = rows
        .into_iter()
        .filter_map(|(content_id, blob_hash, blob_cid, content_format, reach)| {
            // Defensive: SQL filtered NULL, but an empty-string blob_hash is also
            // "no blob" and must not produce a bogus manifest.
            let blob_hash = blob_hash?;
            if blob_hash.trim().is_empty() {
                return None;
            }
            Some(BackfillCandidate {
                content_id,
                blob_hash,
                blob_cid,
                content_format,
                reach,
            })
        })
        .collect();

    Ok(candidates)
}

/// Part B — select content that HAS a manifest but whose shards have ZERO
/// `shard_locations` rows: measured (manifest present) yet never-distributed (no
/// holder rows). This is the exact dark shape the push-side identity fix (slice 2)
/// unblocks — on the next boot after the fix these re-attempt distribution instead
/// of needing a manual per-content re-seed. It is measurement-driven REMEDIATION,
/// NOT a stewardship claim: distribution still has to succeed + be acked to write
/// `shard_locations`.
///
/// Output is capped at `cap` (see [`REDISTRIBUTE_CAP_PER_BOOT`]). The per-content
/// `shard_locations` existence check is one small query per manifest content — an
/// alpha-scale corpus, and the cap bounds the *result*; the scan stops once `cap`
/// candidates are found.
///
/// A blob present-but-absent-locally is NOT excluded here (the byte load happens
/// at distribution time in `distribute_one`, which skips-and-returns on absence).
pub fn select_redistribution_candidates(
    conn: &mut SqliteConnection,
    h_app_id: &str,
    cap: usize,
) -> Result<Vec<BackfillCandidate>, StorageError> {
    use crate::db::diesel_schema::{content, shard_locations, shard_manifests};

    if cap == 0 {
        return Ok(Vec::new());
    }

    // (content_id, blob_hash, blob_cid, content_format, reach, shard_hashes_json)
    type Row = (
        String,
        Option<String>,
        Option<String>,
        String,
        String,
        String,
    );

    let rows: Vec<Row> = content::table
        .inner_join(
            shard_manifests::table.on(shard_manifests::content_id
                .eq(content::id)
                .and(shard_manifests::h_app_id.eq(h_app_id))),
        )
        .filter(content::h_app_id.eq(h_app_id))
        .filter(content::blob_hash.is_not_null())
        .select((
            content::id,
            content::blob_hash,
            content::blob_cid,
            content::content_format,
            content::reach,
            shard_manifests::shard_hashes_json,
        ))
        .load::<Row>(conn)
        .map_err(StorageError::from)?;

    let mut out: Vec<BackfillCandidate> = Vec::new();
    for (content_id, blob_hash, blob_cid, content_format, reach, shard_hashes_json) in rows {
        if out.len() >= cap {
            break;
        }
        let Some(blob_hash) = blob_hash else { continue };
        if blob_hash.trim().is_empty() {
            continue;
        }
        // The manifest's shard hashes are the join key into shard_locations.
        let shard_hashes: Vec<String> =
            serde_json::from_str(&shard_hashes_json).unwrap_or_default();
        if shard_hashes.is_empty() {
            continue;
        }

        // Does ANY shard_location exist for these shards under this app? A single
        // present row means distribution already reached at least one peer — leave
        // partially-placed content alone (only fully-dark content re-attempts).
        let location_count: i64 = shard_locations::table
            .filter(shard_locations::h_app_id.eq(h_app_id))
            .filter(shard_locations::shard_hash.eq_any(&shard_hashes))
            .count()
            .get_result(conn)
            .map_err(StorageError::from)?;

        if location_count == 0 {
            out.push(BackfillCandidate {
                content_id,
                blob_hash,
                blob_cid,
                content_format,
                reach,
            });
        }
    }

    Ok(out)
}

/// Encode + persist the manifest for one candidate. Returns `Ok(true)` if a
/// manifest was recorded, `Ok(false)` if the blob bytes were absent locally
/// (skip-and-warn), or `Err` on encode/persist failure (logged by the caller).
async fn record_one(
    pool: &DbPool,
    blob_store: &BlobStore,
    candidate: &BackfillCandidate,
    h_app_id: &str,
) -> Result<bool, StorageError> {
    let data = match blob_store.get(&candidate.blob_hash).await {
        Ok(d) => d,
        Err(_) => {
            tracing::warn!(
                content_id = %candidate.content_id,
                blob_hash = %candidate.blob_hash,
                "shard_manifest_backfill: blob bytes absent locally; skipping"
            );
            return Ok(false);
        }
    };

    let mut conn = pool
        .get()
        .map_err(|e| StorageError::Internal(e.to_string()))?;

    crate::db::shard_manifests::record_manifest_from_bytes(
        &mut conn,
        &candidate.content_id,
        h_app_id,
        &candidate.blob_hash,
        candidate.blob_cid.as_deref(),
        &data,
        &candidate.content_format,
        &candidate.reach,
    )?;

    Ok(true)
}

/// Trigger shard distribution for one candidate (p2p feature only).
///
/// Loads the blob (already verified present during `record_one`, but re-loads
/// because distribution is a separate, network-bound step) and fans shards out
/// to peers. Best-effort: failures are logged, never propagated.
#[cfg(feature = "p2p")]
async fn distribute_one(
    handle: &crate::p2p::P2PHandle,
    pool: &DbPool,
    blob_store: &BlobStore,
    candidate: &BackfillCandidate,
    h_app_id: &str,
) {
    let data = match blob_store.get(&candidate.blob_hash).await {
        Ok(d) => d,
        Err(_) => return,
    };
    match handle
        .distribute_shards(&candidate.content_id, &data, pool, h_app_id)
        .await
    {
        Ok(n) => tracing::info!(
            content_id = %candidate.content_id,
            shards = n,
            "shard_manifest_backfill: distribution complete"
        ),
        Err(e) => tracing::warn!(
            content_id = %candidate.content_id,
            error = %e,
            "shard_manifest_backfill: distribution failed"
        ),
    }
}

/// Run one bounded backfill sweep.
///
/// `p2p_handle` is threaded as an opaque generic so this signature is identical
/// with and without the `p2p` feature; the distribution arm is feature-gated
/// internally. Callers without p2p pass `None::<()>`-shaped handle via the
/// `#[cfg]`-selected wrapper in `main.rs`.
pub async fn run_once(
    pool: &DbPool,
    blob_store: &BlobStore,
    h_app_id: &str,
    config: &BackfillConfig,
    #[cfg(feature = "p2p")] p2p_handle: Option<crate::p2p::P2PHandle>,
) -> Result<BackfillReport, StorageError> {
    let candidates = {
        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Internal(e.to_string()))?;
        select_candidates(&mut conn, h_app_id)?
    };

    let mut report = BackfillReport {
        candidates: candidates.len(),
        ..Default::default()
    };

    // Part A — manifest backfill for blob-backed content that has NO manifest.
    //
    // An empty Part A skips only Part A's work; it must NEVER short-circuit the
    // sweep. Part A is the MIGRATION arm (legacy content missing a manifest) and
    // goes permanently empty once a pod has caught up — which is exactly alpha's
    // steady state (`distributionState: "measured"`). Returning early here made
    // Part B (below) unreachable on EVERY boot of a caught-up pod, i.e. the
    // measured-but-dark remediation never ran where it was needed most.
    if candidates.is_empty() {
        tracing::debug!(h_app_id, "shard_manifest_backfill: no candidates");
    } else {
        tracing::info!(
            candidates = report.candidates,
            h_app_id,
            "shard_manifest_backfill: starting sweep"
        );

        for (idx, candidate) in candidates.iter().enumerate() {
            match record_one(pool, blob_store, candidate, h_app_id).await {
                Ok(true) => {
                    report.recorded += 1;

                    // Manifest persisted — now fan shards out so receiving peers
                    // populate shard_locations and light "stewarding collectives".
                    #[cfg(feature = "p2p")]
                    if let Some(ref handle) = p2p_handle {
                        distribute_one(handle, pool, blob_store, candidate, h_app_id).await;
                    }
                }
                Ok(false) => report.missing_blob += 1,
                Err(e) => {
                    report.failed += 1;
                    tracing::warn!(
                        content_id = %candidate.content_id,
                        error = %e,
                        "shard_manifest_backfill: record failed (non-fatal)"
                    );
                }
            }

            // Bound the work: pace per-item, with a longer pause at batch edges.
            if (idx + 1) % config.batch_size == 0 {
                tokio::time::sleep(config.batch_pause).await;
            } else {
                tokio::time::sleep(config.item_delay).await;
            }
        }
    }

    // Part B — measurement-driven re-distribution: content that HAS a manifest
    // but ZERO shard_locations (measured yet never-distributed). This makes the
    // slice-2 push-side identity fix self-deploying on alpha — dark content
    // re-attempts distribution at the next restart instead of a manual re-seed.
    // Bounded by REDISTRIBUTE_CAP_PER_BOOT + the same per-item pacing. Gated on a
    // live p2p handle (no handle = nothing to distribute to).
    #[cfg(feature = "p2p")]
    if let Some(ref handle) = p2p_handle {
        let redistribute = {
            let mut conn = pool
                .get()
                .map_err(|e| StorageError::Internal(e.to_string()))?;
            select_redistribution_candidates(&mut conn, h_app_id, REDISTRIBUTE_CAP_PER_BOOT)?
        };
        report.redistribute_candidates = redistribute.len();

        if !redistribute.is_empty() {
            tracing::info!(
                redistribute_candidates = report.redistribute_candidates,
                h_app_id,
                "shard_manifest_backfill: re-attempting distribution for measured-but-dark content"
            );
            for (idx, candidate) in redistribute.iter().enumerate() {
                distribute_one(handle, pool, blob_store, candidate, h_app_id).await;
                report.redistribute_attempts += 1;

                if (idx + 1) % config.batch_size == 0 {
                    tokio::time::sleep(config.batch_pause).await;
                } else {
                    tokio::time::sleep(config.item_delay).await;
                }
            }
        }
    }

    tracing::info!(
        recorded = report.recorded,
        missing_blob = report.missing_blob,
        failed = report.failed,
        redistribute_candidates = report.redistribute_candidates,
        redistribute_attempts = report.redistribute_attempts,
        "shard_manifest_backfill: sweep complete"
    );

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::{NewContent, NewShardManifest};
    use crate::test_util::test_pool;

    fn insert_content(conn: &mut SqliteConnection, id: &str, blob_hash: Option<&str>) {
        use crate::db::diesel_schema::content;
        diesel::insert_into(content::table)
            .values(&NewContent {
                id,
                h_app_id: "lamad",
                title: id,
                description: None,
                content_type: "concept",
                content_format: "html5-app",
                blob_hash,
                blob_cid: None,
                content_size_bytes: None,
                metadata_json: None,
                reach: "commons",
                created_by: None,
                content_body: None,
                dht_anchor_hash: None,
                server_blob_hash: None,
            })
            .execute(conn)
            .expect("insert content");
    }

    fn insert_manifest(conn: &mut SqliteConnection, content_id: &str, blob_hash: &str) {
        let new_manifest = NewShardManifest {
            content_id,
            h_app_id: "lamad",
            blob_hash,
            blob_cid: None,
            encoding: "none",
            data_shard_count: 1,
            parity_shard_count: 0,
            shard_hashes_json: "[]",
            total_size_bytes: 0,
            shard_size_bytes: 0,
            mime_type: "html5-app",
            reach: "commons",
        };
        crate::db::shard_manifests::upsert_manifest(conn, &new_manifest).expect("insert manifest");
    }

    fn insert_manifest_with_shards(
        conn: &mut SqliteConnection,
        content_id: &str,
        blob_hash: &str,
        shards: &[&str],
    ) {
        let json = serde_json::to_string(shards).expect("encode shard hashes");
        let new_manifest = NewShardManifest {
            content_id,
            h_app_id: "lamad",
            blob_hash,
            blob_cid: None,
            encoding: "rs",
            data_shard_count: shards.len() as i32,
            parity_shard_count: 0,
            shard_hashes_json: &json,
            total_size_bytes: 0,
            shard_size_bytes: 0,
            mime_type: "html5-app",
            reach: "commons",
        };
        crate::db::shard_manifests::upsert_manifest(conn, &new_manifest).expect("insert manifest");
    }

    fn seed_location(conn: &mut SqliteConnection, shard_hash: &str, peer_id: &str) {
        crate::db::shard_locations::upsert_location(
            conn,
            &crate::db::models::NewShardLocation {
                shard_hash,
                peer_id,
                h_app_id: "lamad",
                status: "announced",
            },
        )
        .expect("seed location");
    }

    /// Part B: only content with a manifest AND zero shard_locations is selected
    /// for re-distribution. Partially-placed (≥1 location) and no-manifest content
    /// are excluded.
    #[test]
    fn redistribution_selects_only_manifest_with_zero_locations() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        // dark: manifest, no locations → candidate.
        insert_content(&mut conn, "dark", Some("sha256-dark"));
        insert_manifest_with_shards(&mut conn, "dark", "sha256-dark", &["s-dark-1", "s-dark-2"]);

        // lit: manifest + a location present → NOT a candidate.
        insert_content(&mut conn, "lit", Some("sha256-lit"));
        insert_manifest_with_shards(&mut conn, "lit", "sha256-lit", &["s-lit-1"]);
        seed_location(&mut conn, "s-lit-1", "uhCAkHolder");

        // no-manifest: Part A territory, never a redistribution candidate.
        insert_content(&mut conn, "no-manifest", Some("sha256-nm"));

        // empty-shards manifest: nothing to place → skipped.
        insert_content(&mut conn, "empty-shards", Some("sha256-es"));
        insert_manifest(&mut conn, "empty-shards", "sha256-es");

        let cands = select_redistribution_candidates(&mut conn, "lamad", 50).expect("select");
        let ids: Vec<&str> = cands.iter().map(|c| c.content_id.as_str()).collect();
        assert_eq!(ids, vec!["dark"]);
    }

    /// REGRESSION (the defect this restructure closes): an EMPTY Part A
    /// candidate set must skip only Part A — the sweep still runs Part B.
    ///
    /// Against the previous control flow (`if candidates.is_empty() { return }`)
    /// this fails with `redistribute_candidates == 0`: on a caught-up pod (every
    /// blob-backed content already has a manifest — alpha's steady state) Part A
    /// is always empty, so the measured-but-dark redistribution never ran on any
    /// boot.
    ///
    /// The blob bytes are deliberately ABSENT from the temp blob store, so
    /// `distribute_one` returns at its byte-load guard — the sweep is exercised
    /// end-to-end with zero network work.
    #[cfg(feature = "p2p")]
    #[tokio::test]
    async fn part_b_runs_when_part_a_candidate_set_is_empty() {
        let pool = test_pool();
        {
            let mut conn = pool.get().expect("conn");
            // Manifest present + zero shard_locations → a Part B candidate,
            // and (because the manifest exists) NOT a Part A candidate.
            insert_content(&mut conn, "dark", Some("sha256-dark"));
            insert_manifest_with_shards(&mut conn, "dark", "sha256-dark", &["s-dark-1"]);

            assert!(
                select_candidates(&mut conn, "lamad")
                    .expect("select")
                    .is_empty(),
                "precondition: Part A must be empty for this regression"
            );
        }

        let tmp = tempfile::tempdir().expect("tempdir");
        let blob_store = BlobStore::new(tmp.path()).await.expect("blob store");
        let handle = crate::p2p::P2PHandle::for_testing();

        let config = BackfillConfig {
            batch_size: 25,
            item_delay: Duration::from_millis(0),
            batch_pause: Duration::from_millis(0),
        };

        let report = run_once(&pool, &blob_store, "lamad", &config, Some(handle))
            .await
            .expect("sweep");

        assert_eq!(report.candidates, 0, "Part A is empty");
        assert_eq!(report.recorded, 0, "nothing to record in Part A");
        assert_eq!(
            report.redistribute_candidates, 1,
            "Part B must still select the measured-but-dark content"
        );
        assert_eq!(
            report.redistribute_attempts, 1,
            "Part B must still ATTEMPT redistribution (honest attempt count)"
        );
    }

    /// Part B: the per-boot cap bounds the result.
    #[test]
    fn redistribution_respects_cap() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        for i in 0..3 {
            let id = format!("dark-{i}");
            let bh = format!("sha256-{i}");
            insert_content(&mut conn, &id, Some(&bh));
            insert_manifest_with_shards(&mut conn, &id, &bh, &[&format!("s-{i}")]);
        }
        let cands = select_redistribution_candidates(&mut conn, "lamad", 2).expect("select");
        assert_eq!(cands.len(), 2, "cap bounds the result");

        assert!(
            select_redistribution_candidates(&mut conn, "lamad", 0)
                .expect("select")
                .is_empty(),
            "cap 0 yields nothing"
        );
    }

    #[test]
    fn content_with_blob_and_no_manifest_is_selected() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        insert_content(&mut conn, "has-blob-no-manifest", Some("sha256-aaa"));

        let candidates = select_candidates(&mut conn, "lamad").expect("select");
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].content_id, "has-blob-no-manifest");
        assert_eq!(candidates[0].blob_hash, "sha256-aaa");
        assert_eq!(candidates[0].content_format, "html5-app");
        assert_eq!(candidates[0].reach, "commons");
    }

    #[test]
    fn content_with_existing_manifest_is_not_selected() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        insert_content(&mut conn, "has-blob-and-manifest", Some("sha256-bbb"));
        insert_manifest(&mut conn, "has-blob-and-manifest", "sha256-bbb");

        let candidates = select_candidates(&mut conn, "lamad").expect("select");
        assert!(
            candidates.is_empty(),
            "content with an existing manifest must not be a candidate, got {candidates:?}"
        );
    }

    #[test]
    fn content_without_blob_is_not_selected() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        insert_content(&mut conn, "no-blob", None);

        let candidates = select_candidates(&mut conn, "lamad").expect("select");
        assert!(
            candidates.is_empty(),
            "content without a blob must not be a candidate, got {candidates:?}"
        );
    }

    #[test]
    fn empty_string_blob_hash_is_not_selected() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        // A blob_hash present-but-empty is "no blob" — must not yield a bogus manifest.
        insert_content(&mut conn, "empty-blob-hash", Some(""));

        let candidates = select_candidates(&mut conn, "lamad").expect("select");
        assert!(
            candidates.is_empty(),
            "empty-string blob_hash must not be a candidate, got {candidates:?}"
        );
    }

    #[test]
    fn mixed_population_selects_only_the_gap() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        insert_content(&mut conn, "gap-1", Some("sha256-g1"));
        insert_content(&mut conn, "gap-2", Some("sha256-g2"));
        insert_content(&mut conn, "already-done", Some("sha256-done"));
        insert_manifest(&mut conn, "already-done", "sha256-done");
        insert_content(&mut conn, "no-blob", None);

        let mut candidates = select_candidates(&mut conn, "lamad").expect("select");
        candidates.sort_by(|a, b| a.content_id.cmp(&b.content_id));

        let ids: Vec<&str> = candidates.iter().map(|c| c.content_id.as_str()).collect();
        assert_eq!(ids, vec!["gap-1", "gap-2"]);
    }

    #[test]
    fn other_app_manifest_does_not_satisfy_this_app() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        // Content is lamad; a manifest exists but for a different h_app_id.
        insert_content(&mut conn, "cross-app", Some("sha256-x"));
        let other = NewShardManifest {
            content_id: "cross-app",
            h_app_id: "other-app",
            blob_hash: "sha256-x",
            blob_cid: None,
            encoding: "none",
            data_shard_count: 1,
            parity_shard_count: 0,
            shard_hashes_json: "[]",
            total_size_bytes: 0,
            shard_size_bytes: 0,
            mime_type: "html5-app",
            reach: "commons",
        };
        crate::db::shard_manifests::upsert_manifest(&mut conn, &other).expect("insert other");

        let candidates = select_candidates(&mut conn, "lamad").expect("select");
        assert_eq!(
            candidates.len(),
            1,
            "a manifest under a different h_app_id must not satisfy lamad, got {candidates:?}"
        );
        assert_eq!(candidates[0].content_id, "cross-app");
    }

    #[tokio::test]
    async fn run_once_records_manifest_and_skips_missing_blob() {
        let pool = test_pool();
        let blob_store = BlobStore::new_memory();

        // Candidate 1: blob present → should record.
        let stored = blob_store
            .store(b"<html>backfill me</html>")
            .await
            .expect("store blob");
        {
            let mut conn = pool.get().expect("conn");
            insert_content(&mut conn, "present", Some(&stored.hash));
            // Candidate 2: blob_hash set but bytes absent → skip-and-warn.
            insert_content(&mut conn, "absent", Some("sha256-not-in-store"));
        }

        let config = BackfillConfig {
            batch_size: 10,
            item_delay: Duration::from_millis(0),
            batch_pause: Duration::from_millis(0),
        };

        let report = run_once(
            &pool,
            &blob_store,
            "lamad",
            &config,
            #[cfg(feature = "p2p")]
            None,
        )
        .await
        .expect("run_once");

        assert_eq!(report.candidates, 2);
        assert_eq!(report.recorded, 1);
        assert_eq!(report.missing_blob, 1);
        assert_eq!(report.failed, 0);

        // The present candidate now has a manifest; re-running finds nothing.
        {
            let mut conn = pool.get().expect("conn");
            let remaining = select_candidates(&mut conn, "lamad").expect("select");
            // Only the still-missing-blob candidate remains a candidate.
            let ids: Vec<&str> = remaining.iter().map(|c| c.content_id.as_str()).collect();
            assert_eq!(ids, vec!["absent"]);
        }
    }

    /// Wave 1.2 honesty-boundary proof-gate: the backfill flips the resilience
    /// snapshot's `distribution_state` from `"unmeasured"` to `"measured"` for
    /// content the pod genuinely holds — and does so WITHOUT inflating
    /// `stewarding_collectives` (that count is written only by the real
    /// `distribute_shards`/`shard_locations` path, Wave 1.3). This is the
    /// storage-side closure of the DB-free Wave 1.1 fold proof: it binds the
    /// writer (`run_once`) to the read surface (`household_resilience::snapshot`)
    /// so "distributionState becomes a measurement" is a verified law, not a
    /// wiring assumption.
    #[tokio::test]
    async fn backfill_flips_distribution_state_to_measured_without_claiming_stewards() {
        use crate::db::AppContext;
        use crate::services::household_resilience;

        let pool = test_pool();
        let blob_store = BlobStore::new_memory();
        let ctx = AppContext::default_lamad();

        // Content the pod genuinely holds: bytes in the blob store, a content row
        // pointing at them, but NO shard manifest yet (the pre-backfill dark card).
        let stored = blob_store
            .store(b"<html>a household holds this</html>")
            .await
            .expect("store blob");
        {
            let mut conn = pool.get().expect("conn");
            insert_content(&mut conn, "held-doc", Some(&stored.hash));
        }

        // BEFORE: no manifest → honest non-measurement. The card is dark, and it
        // is honest about being dark (never a fake at-risk verdict).
        let before =
            household_resilience::snapshot(&pool, &ctx, "held-doc", None).expect("snapshot before");
        assert_eq!(
            before.distribution_state, "unmeasured",
            "content with no manifest must read as unmeasured (honest dark card)"
        );
        assert_eq!(
            before.stewarding_collectives, 0,
            "no manifest → empty holder relation → zero stewards"
        );

        // Run the backfill: records a manifest for the held blob (no p2p handle,
        // so it does NOT distribute — shard_locations stays empty by design).
        let config = BackfillConfig {
            batch_size: 10,
            item_delay: Duration::from_millis(0),
            batch_pause: Duration::from_millis(0),
        };
        let report = run_once(
            &pool,
            &blob_store,
            "lamad",
            &config,
            #[cfg(feature = "p2p")]
            None,
        )
        .await
        .expect("run_once");
        assert_eq!(report.recorded, 1, "the held blob must gain a manifest");

        // AFTER: manifest present → the distribution state is now a MEASUREMENT.
        let after =
            household_resilience::snapshot(&pool, &ctx, "held-doc", None).expect("snapshot after");
        assert_eq!(
            after.distribution_state, "measured",
            "a recorded manifest must flip distribution_state to measured"
        );

        // Honesty boundary (plan correction #3): the manifest backfill flips the
        // MEASUREMENT, but it must NOT fabricate stewards. `stewarding_collectives`
        // stays zero because no peer has acknowledged holding a shard
        // (shard_locations is empty until the real distribute_shards path, Wave 1.3).
        assert_eq!(
            after.stewarding_collectives, 0,
            "backfill records a manifest, not a stewardship claim — stewards stay 0 \
             until real distribution populates shard_locations (Wave 1.3)"
        );
    }
}
