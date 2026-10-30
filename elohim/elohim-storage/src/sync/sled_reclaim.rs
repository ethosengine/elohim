//! Boot-time reclaim of the sync sled's leaked disk.
//!
//! sled 0.34 keeps a large value in its own blob file and does not remove the
//! superseded file when the value is overwritten. Before the write-side cure
//! (`DocStore::save_if_changed`), every redundant rewrite of a multi-megabyte
//! Automerge document left a dead copy on disk: about 1 GB/h on the alpha fleet,
//! 20-32 GB of garbage per pod, and disk-quota crashloops. Stopping new garbage
//! does not return the old; sled has no online compaction that finds it.
//!
//! This module returns it offline, before the database is opened for service:
//! every tree is exported into a fresh sibling directory, verified entry for
//! entry, fsynced, and swapped in by rename. The fresh directory holds only the
//! live values.
//!
//! # When
//!
//! Only when [`RECLAIM_ENV`] is `on`, and the directory is far above what it holds: more than
//! [`DEFAULT_MIN_DISK_BYTES`] on disk AND more than [`DEFAULT_MIN_RATIO`] times
//! the live logical bytes. A healthy sled sits at roughly 1-2x its logical
//! bytes (segment fragmentation plus the open log), so 4x is never normal
//! amplification; and below 2 GiB a rewrite costs a boot more than the disk it
//! returns. Both bounds are on [`ReclaimPolicy`].
//!
//! # Crash safety
//!
//! Four sibling names, `<live>` being the service path (`sync.sled`):
//!
//! | name | meaning |
//! |------|---------|
//! | `<live>.reclaim-tmp` | the fresh copy being built. Verified once `<live>.reclaim-old` exists. |
//! | `<live>.reclaim-old` | the original, kept until the fresh copy opens. |
//! | `<live>.reclaim-rejected` | a fresh copy that failed to open, being discarded. |
//! | `<live>.reclaim-attempts` | unfinished-attempt count (a file). |
//!
//! The swap is two renames, `live → old` then `tmp → live`, and the first only
//! happens after the copy verified. So on boot ([`recover_leftovers`]):
//!
//! - no live, old and tmp present: the crash fell between the renames, tmp is
//!   verified, finish the swap (`tmp → live`);
//! - no live, old present, no tmp: a rollback was interrupted, finish it
//!   (`old → live`);
//! - tmp present beside live: the copy never reached the swap, delete it;
//! - rejected present: delete it;
//! - live and old present: open live; on success delete old, on failure roll
//!   back (`live → rejected`, `old → live`).
//!
//! Every step moves toward exactly one intact sled at the live path, none
//! deletes the only intact copy, and none is undone by a later rule, so a crash
//! anywhere resolves in one boot. An attempt that never finishes (killed, out of
//! memory, out of time) is counted; after [`DEFAULT_MAX_UNFINISHED_ATTEMPTS`]
//! the node stops trying and says so, rather than paying the cost every boot.
//!
//! # Bounds
//!
//! The readiness probe waits 120 s and this runs before the HTTP listener, so
//! the whole attempt is bounded by [`DEFAULT_BUDGET`]. Past it the copy is
//! discarded and the node serves from the original sled. Progress is logged
//! every few seconds. Free space under the needed headroom refuses before any
//! write. A panic inside sled's export/import (it panics on I/O errors) is
//! caught and treated as an aborted attempt.

use std::collections::BTreeMap;
use std::io;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use tracing::{info, warn};

use crate::error::StorageError;
use crate::services::store_footprint;

/// Environment switch. The reclaim runs only when this is `on` (also `1`, `true`,
/// `yes`); unset or anything else leaves it off. Off by default since 2026-10-29:
/// review found crash- and concurrency-safety gaps in the swap that are still open,
/// and the write-side cure (`DocStore::save_if_changed`) already stops new garbage.
pub const RECLAIM_ENV: &str = "ELOHIM_SLED_RECLAIM";

/// Page cache the sync sled is opened with for service.
pub const SYNC_SLED_CACHE_BYTES: u64 = 64 * 1024 * 1024;

/// Directory size below which a reclaim never runs (2 GiB).
pub const DEFAULT_MIN_DISK_BYTES: u64 = 2 * 1024 * 1024 * 1024;

/// On-disk to live-logical ratio above which a reclaim runs.
pub const DEFAULT_MIN_RATIO: u64 = 4;

/// Wall-clock bound on one attempt. Half the 120 s readiness window, leaving
/// the other half for the rest of boot.
pub const DEFAULT_BUDGET: Duration = Duration::from_secs(60);

/// Unfinished attempts after which the node stops trying.
pub const DEFAULT_MAX_UNFINISHED_ATTEMPTS: u32 = 2;

/// Fixed headroom on top of twice the logical bytes: the fresh copy is about
/// the logical size plus sled's log and segment slack, and the rest of the
/// node keeps writing while it is built.
const HEADROOM_FIXED_BYTES: u64 = 512 * 1024 * 1024;

/// How often progress is logged while copying.
const PROGRESS_EVERY: Duration = Duration::from_secs(5);

/// When and how a reclaim runs.
#[derive(Debug, Clone)]
pub struct ReclaimPolicy {
    pub enabled: bool,
    pub min_disk_bytes: u64,
    pub min_ratio: u64,
    pub budget: Duration,
    pub max_unfinished_attempts: u32,
    /// Available bytes on the filesystem holding a path.
    pub free_space: fn(&Path) -> io::Result<u64>,
}

impl Default for ReclaimPolicy {
    fn default() -> Self {
        Self {
            enabled: true,
            min_disk_bytes: DEFAULT_MIN_DISK_BYTES,
            min_ratio: DEFAULT_MIN_RATIO,
            budget: DEFAULT_BUDGET,
            max_unfinished_attempts: DEFAULT_MAX_UNFINISHED_ATTEMPTS,
            free_space: |p| fs4::available_space(p),
        }
    }
}

impl ReclaimPolicy {
    /// The defaults, with [`RECLAIM_ENV`] read once. Call at boot only.
    pub fn from_env() -> Self {
        let raw = std::env::var(RECLAIM_ENV).ok();
        Self {
            enabled: enabled_from(raw.as_deref()),
            ..Self::default()
        }
    }
}

fn enabled_from(raw: Option<&str>) -> bool {
    matches!(
        raw.map(|v| v.trim().to_ascii_lowercase()).as_deref(),
        Some("on" | "1" | "true" | "yes")
    )
}

/// What the attempt did. `as_str` is the metric label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReclaimOutcome {
    Disabled,
    /// The directory is under `min_disk_bytes`.
    Small,
    /// The directory is within `min_ratio` of its logical bytes.
    NotBloated,
    Reclaimed,
    RefusedFreeSpace,
    /// `max_unfinished_attempts` earlier attempts never finished.
    RefusedAttempts,
    AbortedBudget,
    /// The copy did not match the original.
    AbortedMismatch,
    /// An I/O or sled error, or a panic inside sled.
    AbortedError,
}

impl ReclaimOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::Small => "small",
            Self::NotBloated => "not_bloated",
            Self::Reclaimed => "reclaimed",
            Self::RefusedFreeSpace => "refused_free_space",
            Self::RefusedAttempts => "refused_attempts",
            Self::AbortedBudget => "aborted_budget",
            Self::AbortedMismatch => "aborted_mismatch",
            Self::AbortedError => "aborted_error",
        }
    }

    /// Every label, for zero-initialising the counter.
    pub const ALL: [Self; 9] = [
        Self::Disabled,
        Self::Small,
        Self::NotBloated,
        Self::Reclaimed,
        Self::RefusedFreeSpace,
        Self::RefusedAttempts,
        Self::AbortedBudget,
        Self::AbortedMismatch,
        Self::AbortedError,
    ];
}

/// The result of one boot's reclaim decision.
#[derive(Debug, Clone)]
pub struct ReclaimReport {
    pub outcome: ReclaimOutcome,
    /// Allocated bytes of the live directory before (0 when not measured).
    pub disk_before: u64,
    /// Allocated bytes of the live directory after (equal to before unless
    /// reclaimed).
    pub disk_after: u64,
    /// Live key + value bytes across every tree (0 when not measured).
    pub logical: u64,
    pub elapsed: Duration,
    /// Why, for the log line.
    pub detail: String,
}

/// The sibling paths of one sled directory.
#[derive(Debug, Clone)]
pub struct ReclaimPaths {
    pub live: PathBuf,
    pub tmp: PathBuf,
    pub old: PathBuf,
    pub rejected: PathBuf,
    pub attempts: PathBuf,
}

impl ReclaimPaths {
    pub fn new(live: &Path) -> Self {
        let name = live
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "sync.sled".to_string());
        let sibling = |suffix: &str| live.with_file_name(format!("{name}.{suffix}"));
        Self {
            live: live.to_path_buf(),
            tmp: sibling("reclaim-tmp"),
            old: sibling("reclaim-old"),
            rejected: sibling("reclaim-rejected"),
            attempts: sibling("reclaim-attempts"),
        }
    }

    fn parent(&self) -> PathBuf {
        self.live
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."))
    }
}

/// Open a sync sled at `path` for service with the service config.
pub fn open_at(path: &Path, cache_capacity: u64) -> sled::Result<sled::Db> {
    sled::Config::new()
        .path(path)
        .cache_capacity(cache_capacity)
        .mode(sled::Mode::HighThroughput)
        .open()
}

/// Allocated bytes of every file under `dir`.
pub fn dir_bytes(dir: &Path) -> u64 {
    store_footprint::walk(dir, store_footprint::classify_storage)
        .values()
        .map(|t| t.allocated_bytes)
        .sum()
}

fn remove_path(path: &Path) -> io::Result<()> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.is_dir() => std::fs::remove_dir_all(path),
        Ok(_) => std::fs::remove_file(path),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

fn sync_dir(dir: &Path) -> io::Result<()> {
    std::fs::File::open(dir)?.sync_all()
}

fn rename_synced(from: &Path, to: &Path, parent: &Path) -> io::Result<()> {
    std::fs::rename(from, to)?;
    sync_dir(parent)
}

/// fsync every file and directory under `root`, then `root` itself.
fn sync_tree(root: &Path) -> io::Result<()> {
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir)? {
            let path = entry?.path();
            let meta = std::fs::symlink_metadata(&path)?;
            if meta.is_dir() {
                pending.push(path);
            } else if meta.is_file() {
                std::fs::File::open(&path)?.sync_all()?;
            }
        }
        sync_dir(&dir)?;
    }
    Ok(())
}

/// Resolve whatever an interrupted reclaim left beside `paths.live`, so at
/// most `live` and `old` remain (see the module docs for the rules). `old` is
/// handled by [`open_live`], which must open `live` to decide.
pub fn recover_leftovers(paths: &ReclaimPaths) -> io::Result<()> {
    let parent = paths.parent();
    if !paths.live.exists() && paths.old.exists() {
        if paths.tmp.exists() {
            warn!(
                tmp = %paths.tmp.display(),
                "sync sled reclaim: finishing an interrupted swap (the copy was verified before the swap began)"
            );
            rename_synced(&paths.tmp, &paths.live, &parent)?;
        } else {
            warn!(
                old = %paths.old.display(),
                "sync sled reclaim: finishing an interrupted rollback to the original"
            );
            rename_synced(&paths.old, &paths.live, &parent)?;
        }
    }
    if paths.tmp.exists() {
        warn!(
            tmp = %paths.tmp.display(),
            "sync sled reclaim: discarding a copy that never reached the swap"
        );
        remove_path(&paths.tmp)?;
    }
    if paths.rejected.exists() {
        remove_path(&paths.rejected)?;
    }
    Ok(())
}

fn is_lock_contention(e: &sled::Error) -> bool {
    e.to_string().contains("could not acquire lock")
}

/// Open `live` for service. When `old` sits beside it, `live` is a fresh
/// reclaimed copy: delete `old` once `live` opens, or roll back to `old` if it
/// does not.
pub fn open_live(paths: &ReclaimPaths, cache_capacity: u64) -> Result<sled::Db, StorageError> {
    let open_err =
        |e: sled::Error| StorageError::Database(format!("Failed to open sync.sled: {e}"));
    match open_at(&paths.live, cache_capacity) {
        Ok(db) => {
            if paths.old.exists() {
                if let Err(e) = remove_path(&paths.old) {
                    // Retried next boot by this same rule; the live copy serves.
                    warn!(old = %paths.old.display(), error = %e, "sync sled reclaim: could not delete the original yet");
                }
            }
            Ok(db)
        }
        Err(e) if paths.old.exists() && !is_lock_contention(&e) => {
            warn!(
                error = %e,
                "sync sled reclaim: the reclaimed copy did not open; rolling back to the original"
            );
            let parent = paths.parent();
            rename_synced(&paths.live, &paths.rejected, &parent)?;
            rename_synced(&paths.old, &paths.live, &parent)?;
            remove_path(&paths.rejected)?;
            open_at(&paths.live, cache_capacity).map_err(open_err)
        }
        Err(e) => Err(open_err(e)),
    }
}

/// Live contents of one tree. Two sleds hold the same data iff their
/// per-tree digests are equal.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TreeDigest {
    pub entries: u64,
    /// Key + value bytes.
    pub bytes: u64,
    /// BLAKE3 over every length-prefixed key and value, in key order.
    pub hash: [u8; 32],
}

/// Why a digest or copy stopped.
#[derive(Debug)]
enum Stop {
    Budget,
    Sled(sled::Error),
}

impl From<sled::Error> for Stop {
    fn from(e: sled::Error) -> Self {
        Self::Sled(e)
    }
}

/// Digest every tree, stopping at `deadline`.
fn digest(db: &sled::Db, deadline: Instant) -> Result<BTreeMap<Vec<u8>, TreeDigest>, Stop> {
    let mut out = BTreeMap::new();
    for name in db.tree_names() {
        let tree = db.open_tree(&name)?;
        let mut hasher = blake3::Hasher::new();
        let mut d = TreeDigest::default();
        for kv in tree.iter() {
            if Instant::now() > deadline {
                return Err(Stop::Budget);
            }
            let (k, v) = kv?;
            hasher.update(&(k.len() as u64).to_le_bytes());
            hasher.update(&k);
            hasher.update(&(v.len() as u64).to_le_bytes());
            hasher.update(&v);
            d.entries += 1;
            d.bytes += (k.len() + v.len()) as u64;
        }
        d.hash = *hasher.finalize().as_bytes();
        out.insert(name.to_vec(), d);
    }
    Ok(out)
}

fn read_attempts(path: &Path) -> u32 {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0)
}

fn write_attempts(path: &Path, n: u32) -> io::Result<()> {
    std::fs::write(path, n.to_string())?;
    std::fs::File::open(path)?.sync_all()
}

fn clear_attempts(paths: &ReclaimPaths) {
    if paths.attempts.exists() {
        if let Err(e) = remove_path(&paths.attempts) {
            warn!(error = %e, "sync sled reclaim: could not clear the attempt count");
        }
    }
}

/// Copy every tree of `src` into `dst` with sled's export/import, stopping
/// at `deadline` and logging progress. `Ok(false)` when the deadline cut
/// the copy short.
fn copy_all(src: &sled::Db, dst: &sled::Db, deadline: Instant) -> bool {
    let stopped = AtomicBool::new(false);
    let entries = AtomicU64::new(0);
    let bytes = AtomicU64::new(0);
    let last_log = std::sync::Mutex::new(Instant::now());
    let gate = |kv: &Vec<Vec<u8>>| {
        let now = Instant::now();
        if now > deadline {
            stopped.store(true, Ordering::Relaxed);
            return false;
        }
        let n = entries.fetch_add(1, Ordering::Relaxed) + 1;
        let b = bytes.fetch_add(kv.iter().map(|p| p.len() as u64).sum(), Ordering::Relaxed);
        if let Ok(mut last) = last_log.lock() {
            if now.duration_since(*last) >= PROGRESS_EVERY {
                *last = now;
                info!(entries = n, bytes = b, "sync sled reclaim: copying");
            }
        }
        true
    };
    let gate = &gate;
    let export: Vec<_> = src
        .export()
        .into_iter()
        .map(|(kind, name, iter)| (kind, name, iter.take_while(move |kv| gate(kv))))
        .collect();
    dst.import(export);
    !stopped.load(Ordering::Relaxed)
}

/// Open the sync sled at `path` for service, reclaiming its leaked disk first
/// when `policy` says so. Blocking: call from `spawn_blocking` (or use
/// [`open_sync_sled_async`]). Never returns without a served sled unless the
/// sled itself cannot be opened; every reclaim failure serves the original.
pub fn open_sync_sled(
    path: &Path,
    cache_capacity: u64,
    policy: &ReclaimPolicy,
) -> Result<(sled::Db, ReclaimReport), StorageError> {
    let started = Instant::now();
    let paths = ReclaimPaths::new(path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    recover_leftovers(&paths)?;
    let db = open_live(&paths, cache_capacity)?;
    let report = |outcome: ReclaimOutcome,
                  disk_before: u64,
                  disk_after: u64,
                  logical: u64,
                  detail: String|
     -> ReclaimReport {
        ReclaimReport {
            outcome,
            disk_before,
            disk_after,
            logical,
            elapsed: started.elapsed(),
            detail,
        }
    };
    if !policy.enabled {
        return Ok((
            db,
            report(
                ReclaimOutcome::Disabled,
                0,
                0,
                0,
                format!("{RECLAIM_ENV} is not on"),
            ),
        ));
    }

    let disk_before = dir_bytes(&paths.live);
    if disk_before <= policy.min_disk_bytes {
        clear_attempts(&paths);
        return Ok((
            db,
            report(
                ReclaimOutcome::Small,
                disk_before,
                disk_before,
                0,
                "under the size floor".into(),
            ),
        ));
    }

    let deadline = started + policy.budget;
    let before = match digest(&db, deadline) {
        Ok(d) => d,
        Err(Stop::Budget) => {
            return Ok((
                db,
                report(
                    ReclaimOutcome::AbortedBudget,
                    disk_before,
                    disk_before,
                    0,
                    "budget spent measuring the live bytes".into(),
                ),
            ))
        }
        Err(Stop::Sled(e)) => {
            return Ok((
                db,
                report(
                    ReclaimOutcome::AbortedError,
                    disk_before,
                    disk_before,
                    0,
                    e.to_string(),
                ),
            ))
        }
    };
    let logical: u64 = before.values().map(|d| d.bytes).sum();
    if disk_before <= logical.saturating_mul(policy.min_ratio) {
        clear_attempts(&paths);
        return Ok((
            db,
            report(
                ReclaimOutcome::NotBloated,
                disk_before,
                disk_before,
                logical,
                format!("within {}x of the live bytes", policy.min_ratio),
            ),
        ));
    }

    let unfinished = read_attempts(&paths.attempts);
    if unfinished >= policy.max_unfinished_attempts {
        return Ok((
            db,
            report(
                ReclaimOutcome::RefusedAttempts,
                disk_before,
                disk_before,
                logical,
                format!(
                    "{unfinished} earlier attempts never finished; delete {} to try again",
                    paths.attempts.display()
                ),
            ),
        ));
    }

    let needed = logical
        .saturating_mul(2)
        .saturating_add(HEADROOM_FIXED_BYTES);
    let available = (policy.free_space)(&paths.parent()).unwrap_or(0);
    if available < needed {
        return Ok((
            db,
            report(
                ReclaimOutcome::RefusedFreeSpace,
                disk_before,
                disk_before,
                logical,
                format!("{available} bytes free, {needed} needed"),
            ),
        ));
    }

    // From here an attempt is in flight: count it until it finishes.
    if let Err(e) = write_attempts(&paths.attempts, unfinished + 1) {
        // Without the count a crash mid-copy could repeat every boot.
        return Ok((
            db,
            report(
                ReclaimOutcome::AbortedError,
                disk_before,
                disk_before,
                logical,
                format!("could not record the attempt: {e}"),
            ),
        ));
    }
    info!(
        disk_before,
        logical,
        trees = before.len(),
        "sync sled reclaim: copying the live trees into a fresh sled"
    );

    let abort = |db: sled::Db,
                 outcome: ReclaimOutcome,
                 detail: String|
     -> Result<(sled::Db, ReclaimReport), StorageError> {
        if let Err(e) = remove_path(&paths.tmp) {
            warn!(error = %e, "sync sled reclaim: could not delete the discarded copy; next boot will");
        }
        Ok((
            db,
            report(outcome, disk_before, disk_before, logical, detail),
        ))
    };

    let built = catch_unwind(AssertUnwindSafe(
        || -> Result<Result<(), ReclaimOutcome>, sled::Error> {
            let fresh = open_at(&paths.tmp, cache_capacity)?;
            if !copy_all(&db, &fresh, deadline) {
                return Ok(Err(ReclaimOutcome::AbortedBudget));
            }
            fresh.flush()?;
            let after = match digest(&fresh, deadline) {
                Ok(d) => d,
                Err(Stop::Budget) => return Ok(Err(ReclaimOutcome::AbortedBudget)),
                Err(Stop::Sled(e)) => return Err(e),
            };
            if after != before {
                return Ok(Err(ReclaimOutcome::AbortedMismatch));
            }
            Ok(Ok(()))
        },
    ));
    match built {
        Ok(Ok(Ok(()))) => {}
        Ok(Ok(Err(outcome))) => return abort(db, outcome, "copy discarded".into()),
        Ok(Err(e)) => return abort(db, ReclaimOutcome::AbortedError, e.to_string()),
        Err(_) => {
            return abort(
                db,
                ReclaimOutcome::AbortedError,
                "sled panicked during the copy".into(),
            )
        }
    }
    if let Err(e) = sync_tree(&paths.tmp) {
        return abort(
            db,
            ReclaimOutcome::AbortedError,
            format!("fsync of the copy failed: {e}"),
        );
    }

    // The swap. Close the original first so nothing writes to it mid-rename.
    drop(db);
    let parent = paths.parent();
    let swapped = rename_synced(&paths.live, &paths.old, &parent)
        .and_then(|()| rename_synced(&paths.tmp, &paths.live, &parent));
    if let Err(e) = swapped {
        // Resolve whichever rename failed by the boot rules, then serve.
        warn!(error = %e, "sync sled reclaim: the swap failed; resolving it");
        recover_leftovers(&paths)?;
        let db = open_live(&paths, cache_capacity)?;
        let disk_after = dir_bytes(&paths.live);
        return Ok((
            db,
            report(
                ReclaimOutcome::AbortedError,
                disk_before,
                disk_after,
                logical,
                format!("swap failed: {e}"),
            ),
        ));
    }
    let db = open_live(&paths, cache_capacity)?;
    let disk_after = dir_bytes(&paths.live);
    if paths.old.exists() {
        // open_live rolled back: the original is live again.
        return Ok((
            db,
            report(
                ReclaimOutcome::AbortedError,
                disk_before,
                disk_after,
                logical,
                "the reclaimed copy did not open; rolled back".into(),
            ),
        ));
    }
    clear_attempts(&paths);
    Ok((
        db,
        report(
            ReclaimOutcome::Reclaimed,
            disk_before,
            disk_after,
            logical,
            "swapped".into(),
        ),
    ))
}

/// [`open_sync_sled`] on the blocking pool, with the outcome logged and
/// recorded (`elohim_sync_sled_reclaim_total`, `elohim_sync_sled_reclaim_bytes`).
pub async fn open_sync_sled_async(
    path: PathBuf,
    cache_capacity: u64,
    policy: ReclaimPolicy,
) -> Result<sled::Db, StorageError> {
    let (db, report) =
        tokio::task::spawn_blocking(move || open_sync_sled(&path, cache_capacity, &policy))
            .await
            .map_err(|e| StorageError::Database(format!("sync sled open task failed: {e}")))??;
    log_report(&report);
    crate::metrics::record_sync_sled_reclaim(
        report.outcome.as_str(),
        report.disk_before,
        report.disk_after,
        report.logical,
        report.elapsed,
    );
    Ok(db)
}

fn log_report(r: &ReclaimReport) {
    let secs = r.elapsed.as_secs_f64();
    match r.outcome {
        ReclaimOutcome::Reclaimed => info!(
            outcome = r.outcome.as_str(),
            disk_before = r.disk_before,
            disk_after = r.disk_after,
            logical = r.logical,
            secs,
            "sync sled reclaim: reclaimed {} bytes",
            r.disk_before.saturating_sub(r.disk_after)
        ),
        ReclaimOutcome::Disabled | ReclaimOutcome::Small | ReclaimOutcome::NotBloated => info!(
            outcome = r.outcome.as_str(),
            disk_before = r.disk_before,
            disk_after = r.disk_after,
            logical = r.logical,
            secs,
            detail = %r.detail,
            "sync sled reclaim: nothing to do"
        ),
        _ => warn!(
            outcome = r.outcome.as_str(),
            disk_before = r.disk_before,
            disk_after = r.disk_after,
            logical = r.logical,
            secs,
            detail = %r.detail,
            "sync sled reclaim: not reclaimed; serving the original"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::DocStore;
    use automerge::transaction::Transactable;
    use automerge::Automerge;

    /// Deterministic incompressible bytes (Automerge deflates its columns).
    fn noise(len: usize, seed: u64) -> Vec<u8> {
        let mut x = seed | 1;
        (0..len)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                x as u8
            })
            .collect()
    }

    fn big_doc(len: usize, seed: u64) -> Automerge {
        let mut doc = Automerge::new();
        doc.transact::<_, _, automerge::AutomergeError>(|tx| {
            tx.put(
                automerge::ROOT,
                "body",
                automerge::ScalarValue::Bytes(noise(len, seed)),
            )?;
            Ok(())
        })
        .unwrap();
        doc
    }

    fn policy() -> ReclaimPolicy {
        ReclaimPolicy {
            min_disk_bytes: 0,
            free_space: |_| Ok(u64::MAX),
            ..ReclaimPolicy::default()
        }
    }

    fn off() -> ReclaimPolicy {
        ReclaimPolicy {
            enabled: false,
            ..policy()
        }
    }

    /// Every key and value of one tree, in key order.
    type Entries = Vec<(Vec<u8>, Vec<u8>)>;

    /// Every key and value of every tree.
    fn contents(db: &sled::Db) -> BTreeMap<Vec<u8>, Entries> {
        db.tree_names()
            .into_iter()
            .map(|name| {
                let tree = db.open_tree(&name).unwrap();
                let kvs = tree
                    .iter()
                    .map(|kv| {
                        let (k, v) = kv.unwrap();
                        (k.to_vec(), v.to_vec())
                    })
                    .collect();
                (name.to_vec(), kvs)
            })
            .collect()
    }

    /// Build a sled that leaks the way the fleet's did: one large document
    /// redelivered many times through the always-save path, plus small docs
    /// and a second tree. Returns the docs' stored bytes.
    async fn leaky_sled(live: &Path) -> BTreeMap<String, Vec<u8>> {
        let (db, _) = open_sync_sled(live, SYNC_SLED_CACHE_BYTES, &off()).unwrap();
        let store = DocStore::from_db(db.clone()).unwrap();
        // 256 KiB is far past sled's inline-value bound, so each save is a
        // fresh blob file; Automerge's save is slow in a debug build, so
        // the count stays small. 24 dead copies is still far past 4x.
        let manifesto = big_doc(256 * 1024, 7);
        for _ in 0..24 {
            store
                .save("elohim", "node:manifesto", &manifesto)
                .await
                .unwrap();
        }
        for i in 0..5u64 {
            store
                .save("elohim", &format!("node:small-{i}"), &big_doc(64, i + 11))
                .await
                .unwrap();
        }
        let kad = db.open_tree("kademlia_records").unwrap();
        kad.insert(b"record-a", b"alpha".to_vec()).unwrap();
        kad.insert(b"record-b", noise(4096, 3)).unwrap();
        db.flush().unwrap();
        let mut docs = BTreeMap::new();
        for id in ["node:manifesto", "node:small-0", "node:small-4"] {
            docs.insert(
                id.to_string(),
                store.get("elohim", id).await.unwrap().unwrap().data,
            );
        }
        docs
    }

    #[tokio::test]
    async fn a_leaking_sled_is_reclaimed_and_keeps_every_byte() {
        let dir = tempfile::tempdir().unwrap();
        let live = dir.path().join("sync.sled");
        let docs = leaky_sled(&live).await;
        let before_contents = {
            let (db, _) = open_sync_sled(&live, SYNC_SLED_CACHE_BYTES, &off()).unwrap();
            contents(&db)
        };
        let disk_before = dir_bytes(&live);

        let (db, report) = open_sync_sled(&live, SYNC_SLED_CACHE_BYTES, &policy()).unwrap();
        assert_eq!(report.outcome, ReclaimOutcome::Reclaimed, "{report:?}");
        eprintln!("reclaim: {report:?}");
        assert_eq!(report.disk_before, disk_before);
        assert!(
            report.disk_after * 4 < report.disk_before,
            "the leaked copies are gone: {report:?}"
        );
        assert_eq!(report.disk_after, dir_bytes(&live));
        assert_eq!(
            contents(&db),
            before_contents,
            "every tree holds the same entries"
        );

        let store = DocStore::from_db(db.clone()).unwrap();
        for (id, data) in &docs {
            let got = store.get("elohim", id).await.unwrap().unwrap();
            assert_eq!(&got.data, data, "{id} bytes are identical");
        }
        let paths = ReclaimPaths::new(&live);
        for leftover in [&paths.tmp, &paths.old, &paths.rejected, &paths.attempts] {
            assert!(!leftover.exists(), "{} is gone", leftover.display());
        }

        // The next boot finds nothing to do: a reclaim never repeats itself.
        drop(store);
        drop(db);
        let (_db, again) = open_sync_sled(&live, SYNC_SLED_CACHE_BYTES, &policy()).unwrap();
        assert_eq!(again.outcome, ReclaimOutcome::NotBloated, "{again:?}");
    }

    #[tokio::test]
    async fn the_bounds_and_the_switch_leave_the_sled_alone() {
        let dir = tempfile::tempdir().unwrap();
        let live = dir.path().join("sync.sled");
        leaky_sled(&live).await;
        let disk = dir_bytes(&live);

        let (db, r) = open_sync_sled(&live, SYNC_SLED_CACHE_BYTES, &off()).unwrap();
        assert_eq!(r.outcome, ReclaimOutcome::Disabled);
        drop(db);

        let floor = ReclaimPolicy {
            min_disk_bytes: disk * 2,
            ..policy()
        };
        let (db, r) = open_sync_sled(&live, SYNC_SLED_CACHE_BYTES, &floor).unwrap();
        assert_eq!(r.outcome, ReclaimOutcome::Small);
        drop(db);

        let lenient = ReclaimPolicy {
            min_ratio: 10_000,
            ..policy()
        };
        let (db, r) = open_sync_sled(&live, SYNC_SLED_CACHE_BYTES, &lenient).unwrap();
        assert_eq!(r.outcome, ReclaimOutcome::NotBloated);
        drop(db);

        let full = ReclaimPolicy {
            free_space: |_| Ok(1024),
            ..policy()
        };
        let (db, r) = open_sync_sled(&live, SYNC_SLED_CACHE_BYTES, &full).unwrap();
        assert_eq!(r.outcome, ReclaimOutcome::RefusedFreeSpace, "{r:?}");
        assert!(!ReclaimPaths::new(&live).tmp.exists());
        assert!(
            !ReclaimPaths::new(&live).attempts.exists(),
            "a refusal is not an attempt"
        );
        drop(db);

        let hurried = ReclaimPolicy {
            budget: Duration::ZERO,
            ..policy()
        };
        let (db, r) = open_sync_sled(&live, SYNC_SLED_CACHE_BYTES, &hurried).unwrap();
        assert_eq!(r.outcome, ReclaimOutcome::AbortedBudget, "{r:?}");
        assert!(!ReclaimPaths::new(&live).tmp.exists());
        drop(db);

        // Nothing above touched the sled: it is still there to reclaim.
        let (_db, r) = open_sync_sled(&live, SYNC_SLED_CACHE_BYTES, &policy()).unwrap();
        assert_eq!(r.outcome, ReclaimOutcome::Reclaimed, "{r:?}");
        assert!(!enabled_from(None), "off unless asked for");
        assert!(!enabled_from(Some("")));
        assert!(!enabled_from(Some("auto")));
        assert!(enabled_from(Some("on")));
        assert!(enabled_from(Some(" ON ")));
        assert!(enabled_from(Some("1")));
        assert!(!enabled_from(Some("off")));
        assert!(!enabled_from(Some("0")));
    }

    #[tokio::test]
    async fn unfinished_attempts_stop_retrying() {
        let dir = tempfile::tempdir().unwrap();
        let live = dir.path().join("sync.sled");
        leaky_sled(&live).await;
        let paths = ReclaimPaths::new(&live);
        write_attempts(&paths.attempts, DEFAULT_MAX_UNFINISHED_ATTEMPTS).unwrap();
        let (db, r) = open_sync_sled(&live, SYNC_SLED_CACHE_BYTES, &policy()).unwrap();
        assert_eq!(r.outcome, ReclaimOutcome::RefusedAttempts, "{r:?}");
        assert!(
            r.detail.contains("reclaim-attempts"),
            "the log names the file to delete"
        );
        drop(db);

        // One unfinished attempt (a crash mid-copy) is retried, and success clears it.
        write_attempts(&paths.attempts, 1).unwrap();
        let (_db, r) = open_sync_sled(&live, SYNC_SLED_CACHE_BYTES, &policy()).unwrap();
        assert_eq!(r.outcome, ReclaimOutcome::Reclaimed, "{r:?}");
        assert!(!paths.attempts.exists());
    }

    /// A small sled at `path` holding `marker` under key `who`.
    fn marked_sled(path: &Path, marker: &str) {
        let db = open_at(path, SYNC_SLED_CACHE_BYTES).unwrap();
        db.insert(b"who", marker.as_bytes()).unwrap();
        db.flush().unwrap();
    }

    fn served_marker(live: &Path) -> String {
        let (db, _) = open_sync_sled(live, SYNC_SLED_CACHE_BYTES, &off()).unwrap();
        String::from_utf8(db.get(b"who").unwrap().unwrap().to_vec()).unwrap()
    }

    fn only_live_remains(paths: &ReclaimPaths) {
        assert!(paths.live.is_dir());
        for leftover in [&paths.tmp, &paths.old, &paths.rejected] {
            assert!(!leftover.exists(), "{} is gone", leftover.display());
        }
    }

    #[test]
    fn crash_while_copying_keeps_the_original() {
        let dir = tempfile::tempdir().unwrap();
        let paths = ReclaimPaths::new(&dir.path().join("sync.sled"));
        marked_sled(&paths.live, "original");
        marked_sled(&paths.tmp, "partial-copy");
        assert_eq!(served_marker(&paths.live), "original");
        only_live_remains(&paths);
    }

    #[test]
    fn crash_between_the_renames_finishes_the_swap() {
        let dir = tempfile::tempdir().unwrap();
        let paths = ReclaimPaths::new(&dir.path().join("sync.sled"));
        marked_sled(&paths.old, "original");
        marked_sled(&paths.tmp, "verified-copy");
        assert_eq!(served_marker(&paths.live), "verified-copy");
        only_live_remains(&paths);
    }

    #[test]
    fn crash_after_the_swap_deletes_the_original_once_the_copy_opens() {
        let dir = tempfile::tempdir().unwrap();
        let paths = ReclaimPaths::new(&dir.path().join("sync.sled"));
        marked_sled(&paths.old, "original");
        marked_sled(&paths.live, "verified-copy");
        assert_eq!(served_marker(&paths.live), "verified-copy");
        only_live_remains(&paths);
    }

    #[test]
    fn a_copy_that_does_not_open_rolls_back_to_the_original() {
        let dir = tempfile::tempdir().unwrap();
        let paths = ReclaimPaths::new(&dir.path().join("sync.sled"));
        marked_sled(&paths.old, "original");
        // A live path sled cannot open as a database.
        std::fs::write(&paths.live, b"not a sled").unwrap();
        assert_eq!(served_marker(&paths.live), "original");
        only_live_remains(&paths);
    }

    #[test]
    fn crash_during_a_rollback_finishes_it() {
        let dir = tempfile::tempdir().unwrap();
        let paths = ReclaimPaths::new(&dir.path().join("sync.sled"));
        // After `live → rejected`, before `old → live`.
        marked_sled(&paths.old, "original");
        std::fs::write(&paths.rejected, b"not a sled").unwrap();
        assert_eq!(served_marker(&paths.live), "original");
        only_live_remains(&paths);

        // After `old → live`, before the rejected copy is deleted.
        marked_sled(&paths.rejected, "bad-copy");
        assert_eq!(served_marker(&paths.live), "original");
        only_live_remains(&paths);
    }

    #[test]
    fn a_stray_copy_with_no_original_is_discarded() {
        let dir = tempfile::tempdir().unwrap();
        let paths = ReclaimPaths::new(&dir.path().join("sync.sled"));
        marked_sled(&paths.tmp, "unverified");
        let (db, _) = open_sync_sled(&paths.live, SYNC_SLED_CACHE_BYTES, &off()).unwrap();
        assert!(
            db.get(b"who").unwrap().is_none(),
            "an unverified copy is never promoted"
        );
        drop(db);
        only_live_remains(&paths);
    }

    #[test]
    fn recovery_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let paths = ReclaimPaths::new(&dir.path().join("sync.sled"));
        marked_sled(&paths.old, "original");
        marked_sled(&paths.tmp, "verified-copy");
        recover_leftovers(&paths).unwrap();
        recover_leftovers(&paths).unwrap();
        assert_eq!(served_marker(&paths.live), "verified-copy");
        assert_eq!(served_marker(&paths.live), "verified-copy");
        only_live_remains(&paths);
    }
}
