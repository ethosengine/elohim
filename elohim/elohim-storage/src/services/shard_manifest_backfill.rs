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
//!   2. For each: persist a manifest via the shared, shape-aware
//!      `db::shard_manifests::record_manifest_for_blob` helper (the same one
//!      the `POST /db/content` create path uses — single source of truth). It
//!      reuses an already-known manifest for the blob's hash when one exists
//!      (the common case for a `"chunked"`/`"rs-*"` blob whose composite is
//!      never stored under its own hash — only its shards under theirs), and
//!      otherwise falls back to loading the blob and encoding fresh.
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

/// A content row eligible for manifest backfill: has a blob, lacks a manifest —
/// or holds a manifest that has DIVERGED from the content's current blob.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackfillCandidate {
    pub content_id: String,
    /// `content.blob_hash` — the bytes the content points at RIGHT NOW.
    pub blob_hash: String,
    pub blob_cid: Option<String>,
    pub content_format: String,
    pub reach: String,
    /// `shard_manifests.blob_hash` — the bytes the STORED manifest was stamped
    /// from. `None` when no manifest exists (Part A). `Some(h)` with
    /// `h != blob_hash` is the divergence this sweep re-stamps.
    pub manifest_blob_hash: Option<String>,
}

impl BackfillCandidate {
    /// The stored manifest was stamped from bytes the content no longer points at.
    ///
    /// This is the `stewardingCollectives: 0` root cause. `distribute_shards`
    /// (`p2p/mod.rs`) recomputes a FRESH manifest from the bytes handed to it and
    /// writes `shard_locations` under those NEW shard hashes — but it never
    /// reconciles the stored `shard_manifests` row. Every per-content reader
    /// (`db::shard_locations::get_locations_for_content`,
    /// `household_resilience::load_holder_relation`) keys off the STORED manifest,
    /// so after a blob rotation the join is guaranteed-empty even though
    /// distribution SUCCEEDED and holder rows exist. The card reads
    /// `distributionState: "measured"` with zero stewards — measured yet dark.
    pub fn is_manifest_divergent(&self) -> bool {
        self.manifest_blob_hash
            .as_deref()
            .is_some_and(|stamped| stamped != self.blob_hash)
    }
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
    /// Part B: candidates selected because their stored manifest had DIVERGED
    /// from `content.blob_hash` (blob rotation). Subset of `redistribute_candidates`.
    pub divergent_manifests: usize,
    /// Part B: divergent manifests actually RE-STAMPED from the current local bytes.
    pub manifests_restamped: usize,
    /// Part B: divergent manifests that could NOT be re-stamped because the bytes
    /// for `content.blob_hash` are absent locally — the `bytes-not-local` gap arm
    /// stays authoritative for these (never a re-stamp from bytes we don't hold).
    pub restamp_bytes_missing: usize,
    /// Part B: orphaned `shard_locations` rows pruned after re-stamping (keyed
    /// under a superseded manifest's shard hashes, named by no manifest any more).
    pub orphan_locations_pruned: usize,
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
                // Part A selects on the ABSENCE of a manifest, by construction.
                manifest_blob_hash: None,
            })
        })
        .collect();

    Ok(candidates)
}

/// Part B — select content that HAS a manifest but is not truthfully readable
/// through it. Two admission arms, in priority order:
///
/// 1. **DIVERGENT** (`shard_manifests.blob_hash != content.blob_hash`) — the blob
///    rotated (an SSR bundle rebuild) and the stored manifest still names the OLD
///    bytes' shard hashes. Every per-content reader joins through that manifest, so
///    the holder join is guaranteed-empty *no matter how well distribution went*.
///    These are admitted REGARDLESS of location count: a divergent manifest with
///    plenty of holder rows under the new hash is exactly the
///    `stewardingCollectives: 0` case, and re-stamping alone lights it.
/// 2. **DARK** — manifest agrees with the content blob, but its shards have ZERO
///    `shard_locations` rows: measured yet never-distributed. Measurement-driven
///    REMEDIATION, not a stewardship claim — distribution still has to succeed and
///    be acked to write `shard_locations`.
///
/// Output is capped at `cap` (see [`REDISTRIBUTE_CAP_PER_BOOT`]) with **divergent
/// first**, so a rotated blob cannot lose the per-boot slot lottery to healthy-but-
/// dark rows in a corpus of thousands. Within a bucket, order is the query's.
///
/// The per-content `shard_locations` existence check runs only for the DARK arm and
/// only while that bucket has room, so the expensive per-row query is bounded by
/// `cap` even though the scan itself sees every manifest row (divergence is a free
/// in-memory comparison on already-loaded columns).
///
/// A blob present-but-absent-locally is NOT excluded here — the byte load happens in
/// `run_once`/`distribute_one`, which record an honest `bytes-not-local` gap instead.
pub fn select_redistribution_candidates(
    conn: &mut SqliteConnection,
    h_app_id: &str,
    cap: usize,
) -> Result<Vec<BackfillCandidate>, StorageError> {
    use crate::db::diesel_schema::{content, shard_locations, shard_manifests};

    if cap == 0 {
        return Ok(Vec::new());
    }

    // (content_id, blob_hash, blob_cid, content_format, reach, shard_hashes_json,
    //  manifest_blob_hash)
    type Row = (
        String,
        Option<String>,
        Option<String>,
        String,
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
            shard_manifests::blob_hash,
        ))
        .load::<Row>(conn)
        .map_err(StorageError::from)?;

    let mut divergent: Vec<BackfillCandidate> = Vec::new();
    let mut dark: Vec<BackfillCandidate> = Vec::new();

    for (
        content_id,
        blob_hash,
        blob_cid,
        content_format,
        reach,
        shard_hashes_json,
        manifest_blob_hash,
    ) in rows
    {
        // Both buckets full — nothing further can enter the capped result.
        if divergent.len() >= cap && dark.len() >= cap {
            break;
        }
        let Some(blob_hash) = blob_hash else { continue };
        if blob_hash.trim().is_empty() {
            continue;
        }

        let candidate = BackfillCandidate {
            content_id,
            blob_hash,
            blob_cid,
            content_format,
            reach,
            manifest_blob_hash: Some(manifest_blob_hash),
        };

        // Arm 1 — divergent manifest. Admitted on the divergence alone; the
        // location count is irrelevant because the STORED shard hashes are the
        // wrong join key either way.
        if candidate.is_manifest_divergent() {
            if divergent.len() < cap {
                divergent.push(candidate);
            }
            continue;
        }

        // Arm 2 — dark (manifest agrees, zero holders). Skip the per-row count
        // query once this bucket is full: it can no longer affect the result.
        if dark.len() >= cap {
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
            dark.push(candidate);
        }
    }

    // Divergent first: a rotated blob must not lose the per-boot cap to healthy rows.
    let mut out = divergent;
    out.extend(dark);
    out.truncate(cap);
    Ok(out)
}

/// Encode + persist the manifest for one candidate. Returns `Ok(true)` if a
/// manifest was recorded, `Ok(false)` if neither the blob bytes NOR an
/// existing manifest were found locally (skip-and-warn), or `Err` on
/// encode/persist failure (logged by the caller).
///
/// Shape-aware: routes through `db::shard_manifests::record_manifest_for_blob`,
/// which reuses an already-known manifest for `blob_hash` (the common case for
/// a `"chunked"`/`"rs-*"` blob — e.g. the landing bundle — whose composite is
/// never stored under its own hash, only its shards under theirs) before
/// falling back to a bytes read. A bare `blob_store.get(blob_hash)` here would
/// silently miss every such blob even though the manifest, and every shard
/// byte it names, is already on disk.
async fn record_one(
    pool: &DbPool,
    blob_store: &BlobStore,
    candidate: &BackfillCandidate,
    h_app_id: &str,
) -> Result<bool, StorageError> {
    let mut conn = pool
        .get()
        .map_err(|e| StorageError::Internal(e.to_string()))?;

    match crate::db::shard_manifests::record_manifest_for_blob(
        &mut conn,
        blob_store,
        &candidate.content_id,
        h_app_id,
        &candidate.blob_hash,
        candidate.blob_cid.as_deref(),
        &candidate.content_format,
        &candidate.reach,
    )
    .await
    {
        Ok(_) => Ok(true),
        Err(StorageError::NotFound(_)) => {
            tracing::warn!(
                content_id = %candidate.content_id,
                blob_hash = %candidate.blob_hash,
                "shard_manifest_backfill: blob bytes absent locally and no manifest known; skipping"
            );
            Ok(false)
        }
        Err(e) => Err(e),
    }
}

/// Record an honest, idempotent placement gap for a redistribution candidate whose
/// blob bytes are NOT local — this node holds a manifest but cannot source the
/// bytes, so it cannot be the distributor. One row per manifest shard, upserted by
/// `(content_id, shard_hash, h_app_id, gap_kind)` exactly like `distribute_shards`'
/// gap writer (so `first_seen_at` is preserved and `last_seen_at` bumps on re-run).
///
/// Returns the number of gap rows written (0 when the manifest is missing or has no
/// shards — never an error path that would abort the sweep).
///
/// gap_kind — WHY `peers-unavailable`, not `bytes-not-local`: the placement-gap
/// `gap_kind` is a CLOSED enum at the wire schema (`placement-gap-view.schema.json`:
/// `under-committed | contracts-short | peers-unavailable`), generated into TS.
/// Adding a `bytes-not-local` kind is schema churn crossing the wire, so we reuse
/// the closest existing kind — `peers-unavailable` — which is also the exact
/// fallback `distribute_shards` already uses for a push shortfall on a full
/// selection ("no steward can hold these bytes"). The PRECISE cause lives on the
/// `elohim_shard_redistribute_bytes_missing_total` metric instead, so no diagnostic
/// fidelity is lost.
#[cfg(feature = "p2p")]
fn record_bytes_not_local_gap(
    conn: &mut SqliteConnection,
    content_id: &str,
    h_app_id: &str,
) -> Result<usize, StorageError> {
    let Some(manifest) = crate::db::shard_manifests::get_manifest(conn, h_app_id, content_id)?
    else {
        return Ok(0);
    };
    let shard_hashes: Vec<String> =
        serde_json::from_str(&manifest.shard_hashes_json).unwrap_or_default();
    if shard_hashes.is_empty() {
        return Ok(0);
    }

    let now = chrono::Utc::now().to_rfc3339();
    // Desired steward count mirrors `distribute_shards` (one steward per shard);
    // achieved is 0 because distribution never began (bytes could not be sourced).
    let requested = i32::try_from(shard_hashes.len()).unwrap_or(i32::MAX);
    let mut written = 0usize;
    for hash in &shard_hashes {
        let id = uuid::Uuid::new_v4().to_string();
        let gap = crate::db::models::NewPlacementGap {
            id: &id,
            content_id,
            shard_hash: hash,
            h_app_id,
            requested_steward_count: requested,
            achieved_steward_count: 0,
            contract_coverage: 0.0,
            gap_kind: "peers-unavailable",
            first_seen_at: &now,
            last_seen_at: &now,
        };
        crate::db::placement_gaps::upsert_gap(conn, &gap)?;
        written += 1;
    }
    Ok(written)
}

/// Trigger shard distribution for one candidate (p2p feature only).
///
/// Loads the blob (already verified present during `record_one`, but re-loads
/// because distribution is a separate, network-bound step) and fans shards out
/// to peers. Best-effort: failures are logged, never propagated.
///
/// Shape-aware: routes through `db::shard_manifests::read_blob_bytes_for_manifest`,
/// which falls back to reassembling from the candidate's manifest shards when
/// the composite is absent — the shape a `"chunked"`/`"rs-*"` blob actually
/// takes. A bare `blob_store.get(blob_hash)` here reads as "bytes not local"
/// for such a blob even when every shard byte it names is already on disk.
#[cfg(feature = "p2p")]
async fn distribute_one(
    handle: &crate::p2p::P2PHandle,
    pool: &DbPool,
    blob_store: &BlobStore,
    candidate: &BackfillCandidate,
    h_app_id: &str,
) {
    let mut conn = match pool.get() {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(
                content_id = %candidate.content_id,
                error = %e,
                "shard_manifest_backfill: DB connection unavailable for distribution"
            );
            return;
        }
    };

    let data = match crate::db::shard_manifests::read_blob_bytes_for_manifest(
        &mut conn,
        blob_store,
        &candidate.blob_hash,
    )
    .await
    {
        Ok(d) => d,
        Err(_) => {
            // HONESTY (the silent-bail this closes): a redistribution candidate
            // whose bytes are NOT local (composite absent AND no manifest to
            // reassemble from, or a shard genuinely missing) cannot be
            // distributed by this node. Pre-fix this was a bare `return`: no
            // log, no gap row, no metric. That invisibility hid
            // elohim-host-landing's non-distribution for a DAY (the card read
            // `distributionState:"measured"` + `stewarding:0` +
            // `placementGaps:[]` — measured yet dark, with nothing explaining
            // why). Make it visible three ways: a WARN, a precise metric, and
            // an idempotent placement gap the resilience card reads.
            crate::metrics::inc_shard_redistribute_bytes_missing();
            tracing::warn!(
                content_id = %candidate.content_id,
                blob_hash = %candidate.blob_hash,
                h_app_id,
                "shard_manifest_backfill: redistribution candidate bytes absent locally — \
                 cannot distribute; recording bytes-not-local placement gap"
            );
            if let Err(e) = record_bytes_not_local_gap(&mut conn, &candidate.content_id, h_app_id) {
                tracing::warn!(
                    content_id = %candidate.content_id,
                    error = %e,
                    "shard_manifest_backfill: bytes-not-local gap write failed (non-fatal)"
                );
            }
            return;
        }
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

        report.divergent_manifests = redistribute
            .iter()
            .filter(|c| c.is_manifest_divergent())
            .count();

        if !redistribute.is_empty() {
            tracing::info!(
                redistribute_candidates = report.redistribute_candidates,
                divergent_manifests = report.divergent_manifests,
                h_app_id,
                "shard_manifest_backfill: re-attempting distribution for measured-but-dark content"
            );

            // Shard hashes freed by a re-stamp this sweep — pruned at the end once
            // we can prove no manifest names them any more.
            let mut orphan_candidates: std::collections::HashSet<String> =
                std::collections::HashSet::new();

            for (idx, candidate) in redistribute.iter().enumerate() {
                // ── Re-stamp arm (blob rotation) ─────────────────────────────
                // A divergent manifest is the wrong join key for EVERY per-content
                // reader; distributing again without re-stamping just writes more
                // holder rows under a hash the readers never look up. Re-stamp
                // FIRST (level-triggered reconciliation: heal the divergence that
                // already exists, retroactively), then distribute.
                if candidate.is_manifest_divergent() {
                    // Capture the superseded shard hashes BEFORE the overwrite.
                    let superseded: Vec<String> = {
                        match pool.get() {
                            Ok(mut conn) => crate::db::shard_manifests::get_manifest(
                                &mut conn,
                                h_app_id,
                                &candidate.content_id,
                            )
                            .ok()
                            .flatten()
                            .and_then(|m| {
                                serde_json::from_str::<Vec<String>>(&m.shard_hashes_json).ok()
                            })
                            .unwrap_or_default(),
                            Err(_) => Vec::new(),
                        }
                    };

                    // HONESTY: re-stamp ONLY from bytes we actually hold. When the
                    // bytes for `content.blob_hash` are absent locally the stored
                    // manifest is stale but we cannot truthfully replace it — the
                    // `bytes-not-local` gap arm inside `distribute_one` stays
                    // authoritative, and the divergence remains visible.
                    match record_one(pool, blob_store, candidate, h_app_id).await {
                        Ok(true) => {
                            report.manifests_restamped += 1;
                            orphan_candidates.extend(superseded);
                            tracing::info!(
                                content_id = %candidate.content_id,
                                content_blob_hash = %candidate.blob_hash,
                                stale_manifest_blob_hash = ?candidate.manifest_blob_hash,
                                h_app_id,
                                "shard_manifest_backfill: manifest re-stamped after blob rotation \
                                 (stale shard hashes were the guaranteed-empty holder join)"
                            );
                        }
                        Ok(false) => {
                            report.restamp_bytes_missing += 1;
                            tracing::warn!(
                                content_id = %candidate.content_id,
                                content_blob_hash = %candidate.blob_hash,
                                stale_manifest_blob_hash = ?candidate.manifest_blob_hash,
                                h_app_id,
                                "shard_manifest_backfill: manifest DIVERGED from content.blob_hash \
                                 but the current bytes are absent locally — not re-stamping \
                                 (bytes-not-local gap stays authoritative)"
                            );
                        }
                        Err(e) => {
                            report.failed += 1;
                            tracing::warn!(
                                content_id = %candidate.content_id,
                                error = %e,
                                "shard_manifest_backfill: manifest re-stamp failed (non-fatal)"
                            );
                        }
                    }
                }

                distribute_one(handle, pool, blob_store, candidate, h_app_id).await;
                report.redistribute_attempts += 1;

                if (idx + 1) % config.batch_size == 0 {
                    tokio::time::sleep(config.batch_pause).await;
                } else {
                    tokio::time::sleep(config.item_delay).await;
                }
            }

            // Prune holder rows keyed under superseded shard hashes that no
            // manifest names any more (see `db::shard_locations::prune_orphaned_locations`
            // for the content-addressing safety argument). One pass, one query.
            if !orphan_candidates.is_empty() {
                match pool.get() {
                    Ok(mut conn) => match crate::db::shard_locations::prune_orphaned_locations(
                        &mut conn,
                        h_app_id,
                        &orphan_candidates,
                    ) {
                        Ok(n) => {
                            report.orphan_locations_pruned = n;
                            if n > 0 {
                                tracing::info!(
                                    pruned = n,
                                    h_app_id,
                                    "shard_manifest_backfill: pruned orphaned shard_locations rows \
                                     left by superseded manifests"
                                );
                            }
                        }
                        Err(e) => tracing::warn!(
                            error = %e,
                            "shard_manifest_backfill: orphan location prune failed (non-fatal)"
                        ),
                    },
                    Err(e) => tracing::warn!(
                        error = %e,
                        "shard_manifest_backfill: orphan prune skipped (no db conn)"
                    ),
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
        divergent_manifests = report.divergent_manifests,
        manifests_restamped = report.manifests_restamped,
        restamp_bytes_missing = report.restamp_bytes_missing,
        orphan_locations_pruned = report.orphan_locations_pruned,
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

    /// Slice 2 (honesty): the byte-sourcing gap writer emits one gap row per
    /// manifest shard, all `peers-unavailable` with zero achieved stewards, and is
    /// idempotent (upsert by content+shard+kind — a re-run leaves no duplicates).
    #[cfg(feature = "p2p")]
    #[test]
    fn bytes_not_local_gap_is_written_per_shard_and_idempotent() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        insert_content(&mut conn, "dark", Some("sha256-dark"));
        insert_manifest_with_shards(&mut conn, "dark", "sha256-dark", &["s-dark-1", "s-dark-2"]);

        let n = record_bytes_not_local_gap(&mut conn, "dark", "lamad").expect("write gap");
        assert_eq!(n, 2, "one gap row per manifest shard");

        let query = crate::db::placement_gaps::GapQuery {
            content_id: Some("dark".into()),
            ..Default::default()
        };
        let gaps = crate::db::placement_gaps::list_gaps(&mut conn, "lamad", query.clone())
            .expect("list gaps");
        assert_eq!(gaps.len(), 2);
        assert!(
            gaps.iter().all(|g| g.gap_kind == "peers-unavailable"),
            "closed wire enum → reuse peers-unavailable (precise cause on the metric)"
        );
        assert!(gaps.iter().all(|g| g.achieved_steward_count == 0));

        // Idempotent: a second write upserts the SAME rows (no duplicates).
        let n2 = record_bytes_not_local_gap(&mut conn, "dark", "lamad").expect("write gap 2");
        assert_eq!(n2, 2);
        let gaps2 =
            crate::db::placement_gaps::list_gaps(&mut conn, "lamad", query).expect("list gaps 2");
        assert_eq!(
            gaps2.len(),
            2,
            "upsert by content+shard+kind — a re-run must not duplicate rows"
        );
    }

    /// Slice 2 end-to-end: a Part-B redistribution candidate whose bytes are ABSENT
    /// locally must leave an HONEST, visible placement gap — never the old silent
    /// bail (no log, no gap, no metric) that hid non-distribution for a day.
    #[cfg(feature = "p2p")]
    #[tokio::test]
    async fn dark_redistribution_records_gap_instead_of_silent_bail() {
        let pool = test_pool();
        {
            let mut conn = pool.get().expect("conn");
            insert_content(&mut conn, "dark", Some("sha256-dark"));
            insert_manifest_with_shards(&mut conn, "dark", "sha256-dark", &["s-dark-1"]);
        }
        // Blob bytes deliberately ABSENT from this store → distribute_one bails.
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
        assert_eq!(report.redistribute_candidates, 1);
        assert_eq!(report.redistribute_attempts, 1);

        let mut conn = pool.get().expect("conn");
        let gaps = crate::db::placement_gaps::list_gaps(
            &mut conn,
            "lamad",
            crate::db::placement_gaps::GapQuery {
                content_id: Some("dark".into()),
                ..Default::default()
            },
        )
        .expect("list gaps");
        assert_eq!(
            gaps.len(),
            1,
            "dark redistribution must leave an honest, visible gap (not a silent bail)"
        );
        assert_eq!(gaps[0].gap_kind, "peers-unavailable");
        assert_eq!(gaps[0].achieved_steward_count, 0);
    }

    // ────────────────────────────────────────────────────────────────────────
    // Cure 1 — shard-manifest re-stamp on blob rotation.
    //
    // Root cause: `distribute_shards` recomputes a FRESH manifest from the bytes it
    // is handed and writes `shard_locations` under those NEW shard hashes, but never
    // reconciles the STORED `shard_manifests` row. Every per-content reader joins
    // through the stored manifest, so after a blob rotation the holder join is
    // guaranteed-empty even though distribution succeeded — `distributionState:
    // "measured"` with `stewardingCollectives: 0` (live: elohim-host-landing, stored
    // manifest stamped from a 9,972,404-byte SSR bundle while content.blob_hash
    // points at the 10,002,432-byte current one).
    // ────────────────────────────────────────────────────────────────────────

    /// DETECTOR (i): manifest present + DIVERGENT blob_hash → selected for the
    /// re-stamp path, *regardless of how many holder rows exist*.
    ///
    /// This is the leg the pre-fix selector could never reach: its only admission
    /// rule was "zero shard_locations", and a rotated blob whose distribution
    /// SUCCEEDED has holder rows — under the NEW hash. Both the old rule and the
    /// readers looked at the stale hashes, so the content was simultaneously
    /// "already placed" (excluded from remediation) and "unreadable" (empty join).
    #[test]
    fn divergent_manifest_is_selected_even_with_holder_rows_present() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        // Content now points at the NEW bytes; the manifest still names the OLD.
        insert_content(&mut conn, "rotated", Some("sha256-new"));
        insert_manifest_with_shards(&mut conn, "rotated", "sha256-old", &["sha256-old"]);
        // Distribution DID succeed — but the holder row is keyed under the new hash,
        // which the stored manifest does not name.
        seed_location(&mut conn, "sha256-new", "uhCAkHolder");

        let cands = select_redistribution_candidates(&mut conn, "lamad", 50).expect("select");
        let ids: Vec<&str> = cands.iter().map(|c| c.content_id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["rotated"],
            "a divergent manifest must be admitted despite holder rows existing"
        );
        assert!(cands[0].is_manifest_divergent());
        assert_eq!(cands[0].blob_hash, "sha256-new");
        assert_eq!(
            cands[0].manifest_blob_hash.as_deref(),
            Some("sha256-old"),
            "the candidate must carry the stale stamp so the sweep can log the divergence"
        );
    }

    /// DETECTOR (ii): manifest present + MATCHING blob_hash + holders → untouched.
    /// The healthy steady state must never enter the re-stamp path.
    #[test]
    fn matching_manifest_with_holders_is_not_selected() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        insert_content(&mut conn, "healthy", Some("sha256-same"));
        insert_manifest_with_shards(&mut conn, "healthy", "sha256-same", &["s-1"]);
        seed_location(&mut conn, "s-1", "uhCAkHolder");

        let cands = select_redistribution_candidates(&mut conn, "lamad", 50).expect("select");
        assert!(
            cands.is_empty(),
            "an agreeing manifest with holders is healthy, got {cands:?}"
        );
    }

    /// PRIORITY: a divergent manifest must not lose the per-boot cap lottery to
    /// healthy-but-dark rows. Alpha carries ~11.5k content rows against
    /// `REDISTRIBUTE_CAP_PER_BOOT = 50`, so an unordered scan can starve the one
    /// row that actually needs re-stamping for boot after boot.
    #[test]
    fn divergent_candidates_are_prioritised_within_the_cap() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        // Two dark rows FIRST in insertion order, then the divergent one.
        for i in 0..2 {
            let id = format!("dark-{i}");
            let bh = format!("sha256-dark-{i}");
            insert_content(&mut conn, &id, Some(&bh));
            insert_manifest_with_shards(&mut conn, &id, &bh, &[&format!("s-dark-{i}")]);
        }
        insert_content(&mut conn, "rotated", Some("sha256-new"));
        insert_manifest_with_shards(&mut conn, "rotated", "sha256-old", &["sha256-old"]);

        let cands = select_redistribution_candidates(&mut conn, "lamad", 1).expect("select");
        let ids: Vec<&str> = cands.iter().map(|c| c.content_id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["rotated"],
            "the single capped slot must go to the divergent manifest, not a dark row"
        );
    }

    /// END-TO-END re-stamp: the stored manifest is rewritten from the CURRENT local
    /// bytes, so the shard hashes the readers join through finally match the hashes
    /// distribution writes holder rows under.
    #[cfg(feature = "p2p")]
    #[tokio::test]
    async fn run_once_restamps_a_divergent_manifest_from_local_bytes() {
        let pool = test_pool();
        let blob_store = BlobStore::new_memory();

        // The CURRENT bytes — present locally (this node genuinely holds them).
        let current = blob_store
            .store(b"<html>the rebuilt SSR bundle</html>")
            .await
            .expect("store current blob");
        {
            let mut conn = pool.get().expect("conn");
            insert_content(&mut conn, "rotated", Some(&current.hash));
            // Manifest stamped from a PREVIOUS bundle's bytes.
            insert_manifest_with_shards(
                &mut conn,
                "rotated",
                "sha256-previous-bundle",
                &["sha256-previous-bundle"],
            );
        }

        let config = BackfillConfig {
            batch_size: 25,
            item_delay: Duration::from_millis(0),
            batch_pause: Duration::from_millis(0),
        };
        let handle = crate::p2p::P2PHandle::for_testing();
        let report = run_once(&pool, &blob_store, "lamad", &config, Some(handle))
            .await
            .expect("sweep");

        assert_eq!(report.divergent_manifests, 1);
        assert_eq!(
            report.manifests_restamped, 1,
            "the divergence must be healed"
        );
        assert_eq!(report.restamp_bytes_missing, 0);

        let mut conn = pool.get().expect("conn");
        let manifest = crate::db::shard_manifests::get_manifest(&mut conn, "lamad", "rotated")
            .expect("get manifest")
            .expect("manifest present");
        assert_eq!(
            manifest.blob_hash, current.hash,
            "the manifest must now be stamped from the content's CURRENT blob"
        );
        let shard_hashes: Vec<String> =
            serde_json::from_str(&manifest.shard_hashes_json).expect("decode shard hashes");
        // Small payload → encoding "none" → the single shard IS the blob.
        assert_eq!(
            shard_hashes,
            vec![current.hash.clone()],
            "the join key readers use must be the current bytes' hash"
        );
        assert!(
            !shard_hashes.contains(&"sha256-previous-bundle".to_string()),
            "the superseded shard hash must be gone from the manifest"
        );
    }

    /// HONESTY: a divergent manifest whose CURRENT bytes are absent locally must NOT
    /// be re-stamped — this node cannot truthfully restate a manifest from bytes it
    /// does not hold. The `bytes-not-local` gap arm stays authoritative and the
    /// stale manifest stays visibly stale.
    #[cfg(feature = "p2p")]
    #[tokio::test]
    async fn divergent_manifest_without_local_bytes_is_not_restamped() {
        let pool = test_pool();
        {
            let mut conn = pool.get().expect("conn");
            insert_content(&mut conn, "rotated", Some("sha256-new-not-held"));
            insert_manifest_with_shards(&mut conn, "rotated", "sha256-old", &["sha256-old"]);
        }
        // Blob store deliberately EMPTY — the current bytes are not local.
        let tmp = tempfile::tempdir().expect("tempdir");
        let blob_store = BlobStore::new(tmp.path()).await.expect("blob store");

        let config = BackfillConfig {
            batch_size: 25,
            item_delay: Duration::from_millis(0),
            batch_pause: Duration::from_millis(0),
        };
        let handle = crate::p2p::P2PHandle::for_testing();
        let report = run_once(&pool, &blob_store, "lamad", &config, Some(handle))
            .await
            .expect("sweep");

        assert_eq!(report.divergent_manifests, 1);
        assert_eq!(
            report.manifests_restamped, 0,
            "never re-stamp from bytes this node does not hold"
        );
        assert_eq!(report.restamp_bytes_missing, 1);
        assert_eq!(
            report.orphan_locations_pruned, 0,
            "no re-stamp means nothing was superseded, so nothing may be pruned"
        );

        let mut conn = pool.get().expect("conn");
        let manifest = crate::db::shard_manifests::get_manifest(&mut conn, "lamad", "rotated")
            .expect("get manifest")
            .expect("manifest present");
        assert_eq!(
            manifest.blob_hash, "sha256-old",
            "the stale manifest must be left exactly as it was"
        );

        // The gap arm remains authoritative and visible.
        let gaps = crate::db::placement_gaps::list_gaps(
            &mut conn,
            "lamad",
            crate::db::placement_gaps::GapQuery {
                content_id: Some("rotated".into()),
                ..Default::default()
            },
        )
        .expect("list gaps");
        assert_eq!(
            gaps.len(),
            1,
            "bytes-not-local must still leave an honest gap"
        );
        assert_eq!(gaps[0].gap_kind, "peers-unavailable");
    }

    /// ORPHAN PRUNE: holder rows keyed under a superseded manifest's shard hashes
    /// are removed once no manifest names them — but a hash still named by ANOTHER
    /// manifest survives (shard hashes are content-addressed; byte-identical content
    /// legitimately shares them, and with `encoding = "none"` the shard hash IS the
    /// blob hash).
    #[cfg(feature = "p2p")]
    #[tokio::test]
    async fn restamp_prunes_orphans_but_spares_shared_shard_hashes() {
        let pool = test_pool();
        let blob_store = BlobStore::new_memory();
        let current = blob_store
            .store(b"<html>rebuilt</html>")
            .await
            .expect("store");

        {
            let mut conn = pool.get().expect("conn");
            insert_content(&mut conn, "rotated", Some(&current.hash));
            insert_manifest_with_shards(
                &mut conn,
                "rotated",
                "sha256-old",
                &["s-orphan", "s-kept"],
            );
            // A holder row under each of the OLD manifest's shard hashes.
            seed_location(&mut conn, "s-orphan", "uhCAkHolder");
            seed_location(&mut conn, "s-kept", "uhCAkHolder");
            // A DIFFERENT content whose manifest also names `s-kept` — that shard
            // hash is still live and its holder row must survive the prune.
            insert_content(&mut conn, "sibling", Some("sha256-sibling"));
            insert_manifest_with_shards(&mut conn, "sibling", "sha256-sibling", &["s-kept"]);
        }

        let config = BackfillConfig {
            batch_size: 25,
            item_delay: Duration::from_millis(0),
            batch_pause: Duration::from_millis(0),
        };
        let handle = crate::p2p::P2PHandle::for_testing();
        let report = run_once(&pool, &blob_store, "lamad", &config, Some(handle))
            .await
            .expect("sweep");
        assert_eq!(report.manifests_restamped, 1);
        assert_eq!(
            report.orphan_locations_pruned, 1,
            "only the truly-unreferenced shard hash may be pruned"
        );

        let mut conn = pool.get().expect("conn");
        assert!(
            crate::db::shard_locations::get_locations_for_shard(&mut conn, "s-orphan")
                .expect("query")
                .is_empty(),
            "the orphaned holder row must be gone"
        );
        assert_eq!(
            crate::db::shard_locations::get_locations_for_shard(&mut conn, "s-kept")
                .expect("query")
                .len(),
            1,
            "a shard hash another manifest still names must NOT be pruned"
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

    /// The mesh-measured defect this closes: a landing-bundle-shaped blob
    /// (`"chunked"` encoding, ~16MB+, composite NEVER stored under its own
    /// hash — only its shards under theirs) already got a manifest at ingest
    /// time (`put_blob_bytes`, filed under a `blob:{cid}` projection id). A
    /// content row pointing at that same `blob_hash` — the `POST /db/content`
    /// create for the bundle's slug — is a Part A backfill candidate (no
    /// `(content_id, h_app_id)` manifest row yet). The NEXT backfill sweep
    /// must pick it up and reuse the real, already-stored shard hashes rather
    /// than skip-and-warn on the (genuinely absent) composite.
    #[tokio::test]
    async fn run_once_picks_up_chunk_stored_blob_with_no_composite_on_next_run() {
        let pool = test_pool();
        let blob_store = BlobStore::new_memory();

        // Shards already on disk under THEIR OWN hashes — exactly what
        // `put_blob_bytes` leaves for a `"chunked"`-encoded blob.
        let shard_a = blob_store
            .store(b"landing bundle shard one---")
            .await
            .expect("store shard a");
        let shard_b = blob_store
            .store(b"landing bundle shard two---")
            .await
            .expect("store shard b");
        let mut composite = Vec::new();
        composite.extend_from_slice(b"landing bundle shard one---");
        composite.extend_from_slice(b"landing bundle shard two---");
        let composite_hash = crate::blob_store::BlobStore::compute_hash(&composite);

        let manifest = crate::sharding::ShardManifest {
            blob_cid: format!("bafkrei-{composite_hash}"),
            blob_hash: composite_hash.clone(),
            total_size: composite.len() as u64,
            mime_type: "html5-app".to_string(),
            encoding: "chunked".to_string(),
            data_shards: 2,
            total_shards: 2,
            shard_size: shard_a.size_bytes,
            shard_hashes: vec![shard_a.hash.clone(), shard_b.hash.clone()],
            reach: "commons".to_string(),
            author_id: None,
            created_at: "2026-08-21T00:00:00Z".to_string(),
            verified_at: None,
        };

        {
            let mut conn = pool.get().expect("conn");
            // Ingest-time manifest projection, filed under `blob:{cid}` — the
            // same call `put_blob_bytes` makes, and NOT under the content id
            // used below.
            crate::db::shard_manifests::record_generated_manifest(
                &mut conn,
                &format!("blob:{}", manifest.blob_cid),
                "lamad",
                &manifest,
            )
            .expect("seed ingest-time manifest");
            insert_content(&mut conn, "elohim-host-landing", Some(&composite_hash));
        }

        // Confirms the shape under test: the composite is genuinely absent.
        assert!(
            blob_store.get(&composite_hash).await.is_err(),
            "a chunked blob's composite must never be stored under its own hash"
        );

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

        assert_eq!(report.candidates, 1);
        assert_eq!(
            report.recorded, 1,
            "the chunk-stored blob must gain a manifest, not be skipped as missing"
        );
        assert_eq!(report.missing_blob, 0);
        assert_eq!(report.failed, 0);

        let mut conn = pool.get().expect("conn");
        let row =
            crate::db::shard_manifests::get_manifest(&mut conn, "lamad", "elohim-host-landing")
                .expect("get manifest")
                .expect("manifest must be persisted for the content id");
        assert_eq!(row.encoding, "chunked");
        let shard_hashes: Vec<String> =
            serde_json::from_str(&row.shard_hashes_json).expect("decode shard hashes");
        assert_eq!(
            shard_hashes,
            vec![shard_a.hash, shard_b.hash],
            "shard hashes must be the REAL, already-stored chunk hashes"
        );
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
