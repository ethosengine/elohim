//! Release retention: a peer keeps the bytes of the latest releases of each
//! channel it follows and lets older ones go.
//!
//! The window is counted from the release ledger (`db::release_ledger`), which
//! the adoption controller writes each time it verifies a release. Bytes the
//! ledger does not name are never touched, so a release this peer never
//! verified is outside this pass entirely.
//!
//! What is never released, whatever the window says:
//!
//! - the newest release of a channel, and any release the controller reports as
//!   resolved or applied;
//! - a blob any content row names (what this peer serves);
//! - a blob a kept release also names;
//! - a blob a live custody commitment still obliges someone to hold, because
//!   the custody pass would fetch it straight back.
//!
//! Letting a blob go removes it from every place this peer keeps it: the
//! extraction cache, the blob store (with the shards no other blob shares),
//! the iroh store, and this peer's own record of holding it. The files go
//! first and the rows after, so an interrupted pass leaves rows the next pass
//! finishes, never files that nothing names.
//!
//! Only `app-bundle` releases are released today. Other classes are recorded in
//! the ledger and left alone.

use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use elohim_cache_core::extraction::ExtractionCache;

use super::{state, ArtifactClass, ReleaseManifest};
use crate::blob_store::BlobStore;
use crate::db::release_ledger::{self, LedgerArtifact, LedgerRelease};
use crate::db::DbPool;
use crate::error::StorageError;

/// How often the pass runs.
pub const PASS_INTERVAL_SECS: u64 = 300;

/// bounded-work: how many past-window releases one pass examines. A release a
/// pass cannot finish stays in the ledger, so the pass rotates through them
/// instead of re-examining the same oldest rows forever.
pub const MAX_PAST_WINDOW_PER_PASS: usize = 64;

/// A manifest's artifacts in the shape the ledger records.
pub fn ledger_artifacts(manifest: &ReleaseManifest) -> Vec<LedgerArtifact> {
    manifest
        .artifacts
        .iter()
        .map(|a| LedgerArtifact {
            sha256: a.sha256.to_ascii_lowercase(),
            blob_cid: a.blob_cid.clone(),
            bytes: a.bytes,
            filename: a.filename.clone(),
        })
        .collect()
}

/// The ledger split into what the window keeps and what is past it.
#[derive(Debug, Default)]
pub struct WindowSplit<'a> {
    pub kept: Vec<&'a LedgerRelease>,
    /// Oldest first.
    pub past: Vec<&'a LedgerRelease>,
}

/// Split `releases` (newest first within each channel) at `depth` per channel.
///
/// A release stays kept when it is among its channel's newest `depth`, when the
/// controller names it as resolved or applied (`in_use`), or when its class is
/// not one this pass releases.
pub fn split_window<'a>(
    releases: &'a [LedgerRelease],
    depth: usize,
    in_use: &HashSet<String>,
) -> WindowSplit<'a> {
    let depth = depth.max(1);
    let mut split = WindowSplit::default();
    let mut rank: BTreeMap<&str, usize> = BTreeMap::new();
    for release in releases {
        let position = rank.entry(release.channel_id.as_str()).or_insert(0);
        let in_window = *position < depth;
        *position += 1;
        if in_window
            || in_use.contains(&release.release_cid)
            || release.artifact_class != ArtifactClass::AppBundle.label()
        {
            split.kept.push(release);
        } else {
            split.past.push(release);
        }
    }
    split.past.sort_by_key(|r| r.seq);
    split
}

/// Why a blob of a past-window release is still held.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BlobHold {
    /// A kept release names the same bytes.
    KeptRelease,
    /// A content row names it: this peer serves it.
    Served,
    /// A live custody commitment names it.
    Pledged,
}

impl BlobHold {
    pub const ALL: [BlobHold; 3] = [BlobHold::KeptRelease, BlobHold::Served, BlobHold::Pledged];

    pub fn label(self) -> &'static str {
        match self {
            BlobHold::KeptRelease => "kept_release",
            BlobHold::Served => "served",
            BlobHold::Pledged => "pledged",
        }
    }
}

/// Whether a past-window release's blob may be let go. `None` means release it.
pub fn blob_hold(in_kept_release: bool, content_rows: i64, live_pledges: i64) -> Option<BlobHold> {
    if in_kept_release {
        Some(BlobHold::KeptRelease)
    } else if content_rows > 0 {
        Some(BlobHold::Served)
    } else if live_pledges > 0 {
        Some(BlobHold::Pledged)
    } else {
        None
    }
}

/// One channel's standing after a pass.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelRetention {
    /// Releases the ledger still names, newest first.
    pub held: Vec<String>,
    /// Of those, how many are past the window and held for a reason.
    pub past_window_held: usize,
    /// Releases this pass let go.
    pub released: Vec<String>,
}

/// What one pass did.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PassReport {
    pub at_unix_secs: i64,
    pub depth: usize,
    pub channels: BTreeMap<String, ChannelRetention>,
    /// Past-window blobs still held, by reason.
    pub blobs_held: BTreeMap<&'static str, usize>,
    pub blob_bytes_released: u64,
    pub staging_bytes_released: u64,
    /// Blobs a step failed on. They are tried again next pass.
    pub failures: usize,
    /// Past-window releases this pass did not reach.
    pub unexamined: usize,
}

static LAST_PASS: Mutex<Option<PassReport>> = Mutex::new(None);

/// The last pass, for `GET /admin/adoption`. `null` before the first pass.
pub fn report_json() -> serde_json::Value {
    serde_json::to_value(&*LAST_PASS.lock().unwrap()).unwrap_or(serde_json::Value::Null)
}

/// The retention pass and the stores it lets bytes go from.
pub struct RetentionSweeper {
    db: DbPool,
    blobs: Arc<BlobStore>,
    staging_root: PathBuf,
    extraction: Option<Arc<ExtractionCache>>,
    #[cfg(feature = "p2p-iroh")]
    iroh: Option<Arc<crate::p2p_iroh::IrohBlobStore>>,
    cursor: AtomicUsize,
}

impl RetentionSweeper {
    pub fn new(db: DbPool, blobs: Arc<BlobStore>, staging_root: impl Into<PathBuf>) -> Self {
        Self {
            db,
            blobs,
            staging_root: staging_root.into(),
            extraction: None,
            #[cfg(feature = "p2p-iroh")]
            iroh: None,
            cursor: AtomicUsize::new(0),
        }
    }

    pub fn with_extraction_cache(mut self, cache: Arc<ExtractionCache>) -> Self {
        self.extraction = Some(cache);
        self
    }

    #[cfg(feature = "p2p-iroh")]
    pub fn with_iroh_store(mut self, store: Arc<crate::p2p_iroh::IrohBlobStore>) -> Self {
        self.iroh = Some(store);
        self
    }

    async fn db<T, F>(&self, work: F) -> Result<T, StorageError>
    where
        T: Send + 'static,
        F: FnOnce(&mut diesel::sqlite::SqliteConnection) -> Result<T, StorageError>
            + Send
            + 'static,
    {
        let pool = self.db.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|e| StorageError::Database(format!("release retention: {e}")))?;
            work(&mut conn)
        })
        .await
        .map_err(|e| StorageError::Internal(format!("release retention task: {e}")))?
    }

    /// Run one pass at `depth`, keeping every release in `in_use`.
    pub async fn run_pass(
        &self,
        depth: usize,
        in_use: &HashSet<String>,
    ) -> Result<PassReport, StorageError> {
        let releases = self.db(release_ledger::list_releases).await?;
        let split = split_window(&releases, depth, in_use);
        let kept_blobs: HashSet<&str> = split
            .kept
            .iter()
            .flat_map(|r| r.artifacts.iter().map(|a| a.sha256.as_str()))
            .collect();

        let mut report = PassReport {
            at_unix_secs: state::now_unix(),
            depth: depth.max(1),
            ..PassReport::default()
        };
        for hold in BlobHold::ALL {
            report.blobs_held.insert(hold.label(), 0);
        }

        // Rotate the starting point so releases a pass cannot finish do not
        // starve the ones behind them.
        let start = if split.past.len() > MAX_PAST_WINDOW_PER_PASS {
            self.cursor
                .fetch_add(MAX_PAST_WINDOW_PER_PASS, Ordering::Relaxed)
                % split.past.len()
        } else {
            0
        };
        report.unexamined = split.past.len().saturating_sub(MAX_PAST_WINDOW_PER_PASS);
        let examined: Vec<&LedgerRelease> = split
            .past
            .iter()
            .cycle()
            .skip(start)
            .take(split.past.len().min(MAX_PAST_WINDOW_PER_PASS))
            .copied()
            .collect();

        let mut released_seqs: HashSet<i32> = HashSet::new();
        let mut held_past: HashSet<i32> = HashSet::new();
        for release in examined {
            report.staging_bytes_released += self.remove_staging(&release.release_cid).await;
            let mut finished = true;
            for artifact in &release.artifacts {
                let in_kept = kept_blobs.contains(artifact.sha256.as_str());
                let facts = {
                    let artifact = artifact.clone();
                    self.db(move |conn| {
                        Ok((
                            release_ledger::content_rows_naming(conn, &artifact)?,
                            release_ledger::live_custody_pledges_naming(
                                conn,
                                &artifact.blob_hash(),
                            )?,
                        ))
                    })
                    .await
                };
                let (content_rows, pledges) = match facts {
                    Ok(facts) => facts,
                    Err(e) => {
                        // Not knowing whether a blob is in use is not
                        // permission to release it.
                        tracing::warn!(
                            channel = %release.channel_id,
                            blob = %artifact.blob_hash(),
                            error = %e,
                            "release retention: could not read whether a blob is in use; kept"
                        );
                        report.failures += 1;
                        finished = false;
                        continue;
                    }
                };
                match blob_hold(in_kept, content_rows, pledges) {
                    // A kept release owns these bytes now; this one is done
                    // with them.
                    Some(BlobHold::KeptRelease) => {
                        *report
                            .blobs_held
                            .entry(BlobHold::KeptRelease.label())
                            .or_default() += 1;
                    }
                    Some(hold) => {
                        *report.blobs_held.entry(hold.label()).or_default() += 1;
                        finished = false;
                    }
                    None => match self.release_blob(artifact).await {
                        Ok(bytes) => report.blob_bytes_released += bytes,
                        Err(e) => {
                            tracing::warn!(
                                channel = %release.channel_id,
                                release_cid = %release.release_cid,
                                blob = %artifact.blob_hash(),
                                error = %e,
                                "release retention: could not let a blob go; trying again next pass"
                            );
                            report.failures += 1;
                            finished = false;
                        }
                    },
                }
            }
            if finished {
                let seq = release.seq;
                match self
                    .db(move |conn| release_ledger::remove_release(conn, seq))
                    .await
                {
                    Ok(()) => {
                        released_seqs.insert(seq);
                        tracing::info!(
                            channel = %release.channel_id,
                            release_cid = %release.release_cid,
                            first_seen_at = %release.first_seen_at,
                            depth = report.depth,
                            "release retention: a release past the window was let go"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "release retention: ledger row not removed");
                        report.failures += 1;
                        held_past.insert(seq);
                    }
                }
            } else {
                held_past.insert(release.seq);
            }
        }
        // Past-window releases this pass did not reach are still held.
        for release in &split.past {
            if !released_seqs.contains(&release.seq) {
                held_past.insert(release.seq);
            }
        }

        for release in &releases {
            let channel = report
                .channels
                .entry(release.channel_id.clone())
                .or_default();
            if released_seqs.contains(&release.seq) {
                channel.released.push(release.release_cid.clone());
            } else {
                channel.held.push(release.release_cid.clone());
                if held_past.contains(&release.seq) {
                    channel.past_window_held += 1;
                }
            }
        }
        Ok(report)
    }

    /// Remove a past-window release's staged copy. Returns the bytes removed.
    async fn remove_staging(&self, release_cid: &str) -> u64 {
        let dir = self
            .staging_root
            .join(super::watch::sanitize_segment(release_cid));
        let mut bytes = 0u64;
        let Ok(mut entries) = tokio::fs::read_dir(&dir).await else {
            return 0;
        };
        while let Ok(Some(entry)) = entries.next_entry().await {
            if let Ok(meta) = entry.metadata().await {
                bytes += meta.len();
            }
        }
        match tokio::fs::remove_dir_all(&dir).await {
            Ok(()) => bytes,
            Err(_) => 0,
        }
    }

    /// Let one blob go from every store. Returns the blob-store bytes removed.
    async fn release_blob(&self, artifact: &LedgerArtifact) -> Result<u64, StorageError> {
        let blob_hash = artifact.blob_hash();

        // The extraction first: it is what a request would be served from.
        if let Some(cache) = self.extraction.as_ref() {
            for spelling in [
                blob_hash.as_str(),
                artifact.sha256.as_str(),
                artifact.blob_cid.as_str(),
            ] {
                cache
                    .evict_blob(spelling)
                    .await
                    .map_err(|e| StorageError::Internal(format!("extraction evict: {e}")))?;
            }
        }

        let plan = {
            let blob_hash = blob_hash.clone();
            self.db(move |conn| release_ledger::plan_forget_blob(conn, &blob_hash))
                .await?
        };

        let mut bytes = 0u64;
        for file in &plan.files {
            if !self.blobs.exists(file).await {
                continue;
            }
            bytes += self.blobs.size(file).await.unwrap_or(0);
            self.blobs.delete(file).await?;
            if self.blobs.exists(file).await {
                return Err(StorageError::Internal(format!(
                    "blob file {file} is still present after delete"
                )));
            }
        }

        #[cfg(feature = "p2p-iroh")]
        if let (Some(store), Some(alias)) = (self.iroh.as_ref(), plan.blake3.as_deref()) {
            let hex = alias.strip_prefix("blake3-").unwrap_or(alias);
            let hash: iroh_blobs::Hash = hex
                .parse()
                .map_err(|e| StorageError::Internal(format!("blake3 alias {alias}: {e}")))?;
            store
                .forget(hash)
                .await
                .map_err(|e| StorageError::Internal(format!("iroh forget {alias}: {e}")))?;
        }

        self.db(move |conn| release_ledger::forget_blob_rows(conn, &plan))
            .await?;
        Ok(bytes)
    }
}

/// Publish a pass: the admin report and the gauges.
fn publish(report: &PassReport) {
    for (channel, standing) in &report.channels {
        crate::metrics::set_release_retention_channel(
            channel,
            standing
                .held
                .len()
                .saturating_sub(standing.past_window_held),
            standing.past_window_held,
        );
        crate::metrics::add_release_retention_released(channel, standing.released.len());
    }
    for (reason, count) in &report.blobs_held {
        crate::metrics::set_release_retention_blobs_held(reason, *count);
    }
    crate::metrics::add_release_retention_bytes("blobs", report.blob_bytes_released);
    crate::metrics::add_release_retention_bytes("staging", report.staging_bytes_released);
    crate::metrics::add_release_retention_failures(report.failures);
    *LAST_PASS.lock().unwrap() = Some(report.clone());
}

/// Releases the controller reports as resolved or applied, on any channel.
fn in_use_release_cids() -> HashSet<String> {
    let mut in_use = HashSet::new();
    for channel in state::snapshot() {
        if let Some(head) = channel.resolved_head {
            in_use.insert(head.cid);
        }
        if let Some(applied) = channel.applied_release {
            in_use.insert(applied.cid);
        }
    }
    in_use
}

/// Spawn the pass. Its first run waits one interval, so the adoption
/// controller has resolved its channels before anything is judged.
///
/// bounded-work: one pass per [`PASS_INTERVAL_SECS`], missed ticks skipped, at
/// most [`MAX_PAST_WINDOW_PER_PASS`] releases examined per pass.
pub fn spawn(sweeper: RetentionSweeper) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(PASS_INTERVAL_SECS));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        ticker.tick().await;
        loop {
            ticker.tick().await;
            let depth = crate::runtime_config::release_retention_depth();
            match sweeper.run_pass(depth, &in_use_release_cids()).await {
                Ok(report) => publish(&report),
                Err(e) => {
                    tracing::warn!(error = %e, "release retention: pass did not run");
                    crate::metrics::add_release_retention_failures(1);
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::release_ledger::record_release;
    use diesel::prelude::*;

    fn release(seq: i32, channel: &str, cid: &str, blobs: &[&str]) -> LedgerRelease {
        LedgerRelease {
            seq,
            channel_id: channel.to_string(),
            release_cid: cid.to_string(),
            artifact_class: "app-bundle".to_string(),
            artifacts: blobs
                .iter()
                .map(|hex| LedgerArtifact {
                    sha256: hex.to_string(),
                    blob_cid: String::new(),
                    bytes: 1,
                    filename: format!("{hex}.zip"),
                })
                .collect(),
            first_seen_at: String::new(),
        }
    }

    fn cids(releases: &[&LedgerRelease]) -> Vec<String> {
        releases.iter().map(|r| r.release_cid.clone()).collect()
    }

    #[test]
    fn the_window_keeps_the_newest_of_each_channel_and_counts_channels_apart() {
        // Newest first within a channel, as `list_releases` returns them.
        let ledger = vec![
            release(5, "a", "a3", &["x"]),
            release(3, "a", "a2", &["x"]),
            release(1, "a", "a1", &["x"]),
            release(4, "b", "b2", &["x"]),
            release(2, "b", "b1", &["x"]),
        ];
        let split = split_window(&ledger, 2, &HashSet::new());
        assert_eq!(cids(&split.kept), ["a3", "a2", "b2", "b1"]);
        assert_eq!(cids(&split.past), ["a1"]);
    }

    #[test]
    fn a_release_in_use_is_kept_however_old_and_does_not_shrink_the_window() {
        let ledger = vec![
            release(4, "a", "a4", &["x"]),
            release(3, "a", "a3", &["x"]),
            release(2, "a", "a2", &["x"]),
            release(1, "a", "a1", &["x"]),
        ];
        let in_use = HashSet::from(["a1".to_string()]);
        let split = split_window(&ledger, 2, &in_use);
        assert_eq!(cids(&split.kept), ["a4", "a3", "a1"]);
        assert_eq!(cids(&split.past), ["a2"]);
    }

    #[test]
    fn a_depth_of_zero_still_keeps_the_newest_release() {
        let ledger = vec![release(2, "a", "a2", &["x"]), release(1, "a", "a1", &["x"])];
        let split = split_window(&ledger, 0, &HashSet::new());
        assert_eq!(cids(&split.kept), ["a2"]);
    }

    #[test]
    fn a_class_this_pass_does_not_release_is_never_past_the_window() {
        let mut old = release(1, "a", "a1", &["x"]);
        old.artifact_class = "coordinator-bundle".to_string();
        let ledger = vec![
            release(3, "a", "a3", &["x"]),
            release(2, "a", "a2", &["x"]),
            old,
        ];
        let split = split_window(&ledger, 1, &HashSet::new());
        assert_eq!(cids(&split.past), ["a2"]);
        assert!(cids(&split.kept).contains(&"a1".to_string()));
    }

    #[test]
    fn a_blob_is_let_go_only_when_nothing_holds_it() {
        assert_eq!(blob_hold(false, 0, 0), None);
        assert_eq!(blob_hold(true, 0, 0), Some(BlobHold::KeptRelease));
        assert_eq!(blob_hold(false, 1, 0), Some(BlobHold::Served));
        assert_eq!(blob_hold(false, 0, 1), Some(BlobHold::Pledged));
        // Being served outranks a pledge: it is the reason an operator acts on.
        assert_eq!(blob_hold(false, 2, 3), Some(BlobHold::Served));
    }

    // ── the pass against real stores ────────────────────────────────────────

    struct Peer {
        _dir: tempfile::TempDir,
        pool: DbPool,
        blobs: Arc<BlobStore>,
        staging: PathBuf,
        sweeper: RetentionSweeper,
    }

    async fn peer() -> Peer {
        let dir = tempfile::tempdir().unwrap();
        let pool = crate::db::init_pool_from_dir(dir.path()).unwrap();
        let blobs = Arc::new(BlobStore::new(dir.path().join("blobs")).await.unwrap());
        let staging = dir.path().join("release-adoption");
        let sweeper = RetentionSweeper::new(pool.clone(), blobs.clone(), staging.clone());
        Peer {
            _dir: dir,
            pool,
            blobs,
            staging,
            sweeper,
        }
    }

    impl Peer {
        /// Publish one release of `zips` the way a peer comes to hold it: the
        /// bytes in the blob store with a manifest and a self-custody row, a
        /// staged copy, and a ledger row.
        async fn hold_release(
            &self,
            channel: &str,
            cid: &str,
            zips: &[&[u8]],
        ) -> Vec<LedgerArtifact> {
            let mut artifacts = Vec::new();
            for (i, zip) in zips.iter().enumerate() {
                let stored = self.blobs.store(zip).await.unwrap();
                let hex = stored.hash.trim_start_matches("sha256-").to_string();
                let artifact = LedgerArtifact {
                    sha256: hex,
                    blob_cid: stored.cid.to_string(),
                    bytes: zip.len() as u64,
                    filename: format!("app-{i}.zip"),
                };
                let mut conn = self.pool.get().unwrap();
                let hash = artifact.blob_hash();
                diesel::sql_query(
                    "INSERT OR REPLACE INTO shard_manifests (content_id, h_app_id, blob_hash, \
                     blob_cid, encoding, data_shard_count, parity_shard_count, \
                     shard_hashes_json, total_size_bytes, shard_size_bytes, mime_type, reach, \
                     created_at) VALUES (?, 'lamad', ?, ?, 'none', 1, 0, ?, ?, ?, \
                     'application/zip', 'commons', '2026-10-05T00:00:00Z')",
                )
                .bind::<diesel::sql_types::Text, _>(format!("blob:{}", artifact.blob_cid))
                .bind::<diesel::sql_types::Text, _>(&hash)
                .bind::<diesel::sql_types::Text, _>(&artifact.blob_cid)
                .bind::<diesel::sql_types::Text, _>(format!("[\"{hash}\"]"))
                .bind::<diesel::sql_types::BigInt, _>(zip.len() as i64)
                .bind::<diesel::sql_types::BigInt, _>(zip.len() as i64)
                .execute(&mut conn)
                .unwrap();
                crate::db::peer_blob_inventory::record_self_custody(
                    &mut conn,
                    "self",
                    &hash,
                    None,
                    "2026-10-05T00:00:00Z",
                )
                .unwrap();
                let staged = self
                    .staging
                    .join(super::super::watch::sanitize_segment(cid))
                    .join(&artifact.filename);
                tokio::fs::create_dir_all(staged.parent().unwrap())
                    .await
                    .unwrap();
                tokio::fs::write(&staged, zip).await.unwrap();
                artifacts.push(artifact);
            }
            let mut conn = self.pool.get().unwrap();
            assert!(record_release(&mut conn, channel, cid, "app-bundle", &artifacts).unwrap());
            artifacts
        }

        async fn holds(&self, artifact: &LedgerArtifact) -> bool {
            self.blobs.exists(&artifact.blob_hash()).await
        }

        fn staged(&self, cid: &str) -> bool {
            self.staging
                .join(super::super::watch::sanitize_segment(cid))
                .exists()
        }

        fn rows(&self, table: &str, column: &str, value: &str) -> i64 {
            #[derive(QueryableByName)]
            struct N {
                #[diesel(sql_type = diesel::sql_types::BigInt)]
                n: i64,
            }
            let mut conn = self.pool.get().unwrap();
            diesel::sql_query(format!(
                "SELECT COUNT(*) AS n FROM {table} WHERE {column} = ?"
            ))
            .bind::<diesel::sql_types::Text, _>(value)
            .get_result::<N>(&mut conn)
            .unwrap()
            .n
        }
    }

    const CHANNEL: &str = "runtime:app-bundle:test:dev";

    #[tokio::test]
    async fn releases_past_the_window_leave_every_store_and_the_window_stays_whole() {
        let peer = peer().await;
        let r1 = peer
            .hold_release(CHANNEL, "r1", &[b"one-browser", b"one-server"])
            .await;
        let r2 = peer
            .hold_release(CHANNEL, "r2", &[b"two-browser", b"two-server"])
            .await;
        let r3 = peer
            .hold_release(CHANNEL, "r3", &[b"three-browser", b"three-server"])
            .await;

        let report = peer.sweeper.run_pass(2, &HashSet::new()).await.unwrap();

        for artifact in &r1 {
            assert!(
                !peer.holds(artifact).await,
                "a past-window blob is still in the store"
            );
            let hash = artifact.blob_hash();
            assert_eq!(peer.rows("shard_manifests", "blob_hash", &hash), 0);
            assert_eq!(peer.rows("peer_blob_inventory", "blob_hash", &hash), 0);
        }
        assert!(!peer.staged("r1"));
        for artifact in r2.iter().chain(&r3) {
            assert!(
                peer.holds(artifact).await,
                "a blob inside the window was let go"
            );
            assert_eq!(
                peer.rows("shard_manifests", "blob_hash", &artifact.blob_hash()),
                1
            );
        }
        assert!(peer.staged("r2") && peer.staged("r3"));

        let channel = &report.channels[CHANNEL];
        assert_eq!(channel.held, ["r3", "r2"]);
        assert_eq!(channel.released, ["r1"]);
        assert_eq!(channel.past_window_held, 0);
        assert_eq!(report.blob_bytes_released, 21);
        assert_eq!(report.staging_bytes_released, 21);
        assert_eq!(report.failures, 0);

        // A second pass finds nothing left to do.
        let again = peer.sweeper.run_pass(2, &HashSet::new()).await.unwrap();
        assert_eq!(again.channels[CHANNEL].held, ["r3", "r2"]);
        assert!(again.channels[CHANNEL].released.is_empty());
        assert_eq!(again.blob_bytes_released, 0);
    }

    #[tokio::test]
    async fn a_blob_this_peer_serves_is_kept_past_the_window_and_let_go_once_it_is_not() {
        let peer = peer().await;
        let r1 = peer
            .hold_release(CHANNEL, "r1", &[b"served-browser", b"idle-server"])
            .await;
        peer.hold_release(CHANNEL, "r2", &[b"two"]).await;
        // The peer never took r2 up: its app row still serves r1's browser zip.
        {
            let mut conn = peer.pool.get().unwrap();
            diesel::sql_query(
                "INSERT INTO content (id, h_app_id, title, content_type, content_format, \
                 blob_hash) VALUES ('elohim-app', 'lamad', 'App', 'app', 'html5-app', ?)",
            )
            .bind::<diesel::sql_types::Text, _>(r1[0].blob_hash())
            .execute(&mut conn)
            .unwrap();
        }

        let report = peer.sweeper.run_pass(1, &HashSet::new()).await.unwrap();
        assert!(
            peer.holds(&r1[0]).await,
            "the blob this peer serves was deleted"
        );
        assert!(
            !peer.holds(&r1[1]).await,
            "an unserved blob of the same release stayed"
        );
        assert_eq!(report.blobs_held["served"], 1);
        assert_eq!(report.channels[CHANNEL].held, ["r2", "r1"]);
        assert_eq!(report.channels[CHANNEL].past_window_held, 1);

        // The peer takes the newer release up; its row moves off r1's bytes.
        {
            let mut conn = peer.pool.get().unwrap();
            diesel::sql_query(
                "UPDATE content SET blob_hash = 'sha256-other' WHERE id = 'elohim-app'",
            )
            .execute(&mut conn)
            .unwrap();
        }
        let report = peer.sweeper.run_pass(1, &HashSet::new()).await.unwrap();
        assert!(!peer.holds(&r1[0]).await);
        assert_eq!(report.channels[CHANNEL].released, ["r1"]);
    }

    #[tokio::test]
    async fn bytes_a_kept_release_shares_are_kept_and_the_old_release_still_leaves() {
        let peer = peer().await;
        // A revert: r3 is a new release over r1's bytes.
        let r1 = peer.hold_release(CHANNEL, "r1", &[b"same-bytes"]).await;
        let r2 = peer.hold_release(CHANNEL, "r2", &[b"other-bytes"]).await;
        peer.hold_release(CHANNEL, "r3", &[b"same-bytes"]).await;

        let report = peer.sweeper.run_pass(1, &HashSet::new()).await.unwrap();
        assert!(
            peer.holds(&r1[0]).await,
            "bytes the newest release names were deleted"
        );
        assert!(!peer.holds(&r2[0]).await);
        assert_eq!(report.channels[CHANNEL].held, ["r3"]);
        assert_eq!(report.channels[CHANNEL].released, ["r2", "r1"]);
        assert_eq!(report.blobs_held["kept_release"], 1);
    }

    #[tokio::test]
    async fn a_blob_under_a_live_custody_commitment_is_kept_until_it_is_withdrawn() {
        let peer = peer().await;
        let r1 = peer.hold_release(CHANNEL, "r1", &[b"pledged"]).await;
        peer.hold_release(CHANNEL, "r2", &[b"two"]).await;
        {
            let mut conn = peer.pool.get().unwrap();
            diesel::sql_query(
                "INSERT INTO rea_commitments (id, h_app_id, action, provider, receiver, \
                 resource_classified_as, state) VALUES ('c1', 'lamad', 'custody-blob', 'me', \
                 'me', ?, 'active')",
            )
            .bind::<diesel::sql_types::Text, _>(r1[0].blob_hash())
            .execute(&mut conn)
            .unwrap();
        }
        let report = peer.sweeper.run_pass(1, &HashSet::new()).await.unwrap();
        assert!(peer.holds(&r1[0]).await);
        assert_eq!(report.blobs_held["pledged"], 1);

        {
            let mut conn = peer.pool.get().unwrap();
            diesel::sql_query("UPDATE rea_commitments SET state = 'superseded' WHERE id = 'c1'")
                .execute(&mut conn)
                .unwrap();
        }
        peer.sweeper.run_pass(1, &HashSet::new()).await.unwrap();
        assert!(!peer.holds(&r1[0]).await);
    }

    #[tokio::test]
    async fn a_shard_another_blob_names_is_not_deleted_with_the_blob() {
        let peer = peer().await;
        let r1 = peer.hold_release(CHANNEL, "r1", &[b"old-zip"]).await;
        peer.hold_release(CHANNEL, "r2", &[b"new-zip"]).await;
        // Some unrelated blob's manifest names r1's file as one of its shards.
        {
            let mut conn = peer.pool.get().unwrap();
            diesel::sql_query(
                "INSERT INTO shard_manifests (content_id, h_app_id, blob_hash, encoding, \
                 data_shard_count, parity_shard_count, shard_hashes_json, total_size_bytes, \
                 shard_size_bytes, mime_type, reach, created_at) VALUES ('other', 'lamad', \
                 'sha256-other', 'chunked', 1, 0, ?, 7, 7, 'application/zip', 'commons', \
                 '2026-10-05T00:00:00Z')",
            )
            .bind::<diesel::sql_types::Text, _>(format!("[\"{}\"]", r1[0].blob_hash()))
            .execute(&mut conn)
            .unwrap();
        }
        let report = peer.sweeper.run_pass(1, &HashSet::new()).await.unwrap();
        assert!(
            peer.holds(&r1[0]).await,
            "a file another blob is made of was deleted"
        );
        assert_eq!(report.channels[CHANNEL].released, ["r1"]);
    }

    #[tokio::test]
    async fn the_extraction_of_a_released_blob_is_evicted_and_a_kept_one_is_not() {
        use elohim_cache_core::extraction::{DiskBackend, ExtractionCacheConfig};
        let mut peer = peer().await;
        let cache_dir = peer._dir.path().join("cache");
        let cache = Arc::new(ExtractionCache::new(
            Box::new(DiskBackend::new(cache_dir.clone()).await.unwrap()),
            ExtractionCacheConfig {
                cache_dir: cache_dir.clone(),
                ..ExtractionCacheConfig::default()
            },
        ));
        peer.sweeper =
            RetentionSweeper::new(peer.pool.clone(), peer.blobs.clone(), peer.staging.clone())
                .with_extraction_cache(cache.clone());
        let r1 = peer.hold_release(CHANNEL, "r1", &[b"old"]).await;
        let r2 = peer.hold_release(CHANNEL, "r2", &[b"new"]).await;
        // r1 was once served by its address; r2 is served by slug.
        cache
            .put_app(
                &r1[0].blob_cid,
                &r1[0].blob_hash(),
                vec![("index.html".into(), b"old".to_vec())],
            )
            .await
            .unwrap();
        cache
            .put_app(
                "elohim-app",
                &r2[0].blob_hash(),
                vec![("index.html".into(), b"new".to_vec())],
            )
            .await
            .unwrap();

        peer.sweeper.run_pass(1, &HashSet::new()).await.unwrap();
        assert!(
            !cache_dir.join(&r1[0].blob_cid).exists(),
            "a released blob's extraction stayed"
        );
        assert!(cache_dir.join("elohim-app").join("index.html").exists());
    }
}
