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
//! Five sibling names, `<live>` being the service path (`sync.sled`):
//!
//! | name | meaning |
//! |------|---------|
//! | `<live>.reclaim-tmp` | the fresh copy being built. Verified once `<live>.reclaim-old` exists. |
//! | `<live>.reclaim-old` | the original, complete only while `.reclaim-complete` is present. |
//! | `<live>.reclaim-rejected` | a fresh copy that failed to open, being discarded. |
//! | `<live>.reclaim-attempts` | unfinished-attempt count (a file). |
//! | `<live>.reclaim-lock` | held (`flock`) for the whole open; never deleted. |
//!
//! The swap is two renames, `live → old` then `tmp → live`, and the first only
//! happens after the copy verified. So on boot ([`recover_leftovers`]):
//!
//! - no live, old and tmp present: the crash fell between the renames, tmp is
//!   verified, finish the swap (`tmp → live`);
//! - no live, old present, no tmp: finish an interrupted rollback (`old → live`)
//!   only if old has its completeness marker;
//! - tmp present beside live: the copy never reached the swap, delete it;
//! - rejected present: delete it;
//! - live and old present: open live; on success delete old, on corruption roll
//!   back only if old has its completeness marker (`live → rejected`, `old → live`).
//!   Other open errors leave live in place. An unmarked old is stale and deleted.
//!
//! The marker is fsynced into the original after verification, before the first
//! rename. Deleting old removes and fsyncs the marker first, so a partial deletion
//! can never be mistaken for a complete original on the next boot.
//!
//! Every step moves toward exactly one intact sled at the live path, none
//! deletes the only intact copy, and none is undone by a later rule, so a crash
//! anywhere resolves in one boot. The live path is never opened (which would
//! create an empty database) while it is missing beside an original.
//!
//! Those rules assume one opener at a time: a second boot reading the names
//! mid-swap would see a missing live path or a copy it takes for garbage. So
//! the whole open, recovery through swap, holds an exclusive lock on
//! `<live>.reclaim-lock`, and a second opener is refused rather than queued
//! (it could not open the sled the first one serves anyway).
//!
//! Deleting a discarded copy is never needed for an intact live sled, so a
//! failure to delete one is logged and the boot serves without reclaiming
//! until it is gone. An open error without a safe rollback, or a rename that
//! cannot be made, stops a boot. A rollback first frees the rejected name.
//!
//! An attempt is counted from the moment it starts reading the live trees,
//! and the count is cleared only when a copy is promoted (or the sled turns
//! out not to need one). A reclaim that is killed, runs out of time or memory,
//! or rolls back stays counted; after [`DEFAULT_MAX_UNFINISHED_ATTEMPTS`] the
//! node stops trying and says so, rather than paying the cost every boot.
//!
//! # Bounds
//!
//! The readiness probe waits 120 s and this runs before the HTTP listener, so
//! everything before the swap (measuring, copying, verifying, fsyncing the
//! copy) checks [`DEFAULT_BUDGET`] between entries and files. Past it the copy
//! is discarded and the node serves from the original sled. A single file's
//! or directory's fsync is not interruptible, so the overshoot is at most one fsync.
//!
//! Outside the budget, by design: the directory walk that decides whether to
//! try (a `stat` per file), the swap's two renames, opening the promoted copy,
//! and deleting the original or a discarded copy. Each is either cheap or
//! resumed where it stopped by the boot rules (a deletion interrupted midway
//! has removed what it removed), so none can repeat its whole cost forever.
//!
//! Progress is logged every few seconds. Free space under the needed headroom
//! refuses before any write. A panic inside sled's export/import (it panics on
//! I/O errors) is caught and treated as an aborted attempt.

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

/// Removed and fsynced before any original files can be deleted.
const ORIGINAL_COMPLETE: &str = ".reclaim-complete";

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
    /// Opens the live path for service ([`open_at`]).
    pub open: fn(&Path, u64) -> sled::Result<sled::Db>,
    /// Deletes a file or directory tree, a missing one included.
    pub remove: fn(&Path) -> io::Result<()>,
    /// Fsyncs a copy directory within the attempt's deadline.
    pub sync_dir: fn(&Path, Instant) -> io::Result<()>,
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
            open: open_at,
            remove: remove_path,
            sync_dir: |p, _| sync_dir(p),
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
    pub lock: PathBuf,
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
            lock: sibling("reclaim-lock"),
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

fn mark_original_complete(original: &Path) -> io::Result<()> {
    std::fs::File::create(original.join(ORIGINAL_COMPLETE))?.sync_all()?;
    sync_dir(original)
}

fn remove_original(paths: &ReclaimPaths, remove: fn(&Path) -> io::Result<()>) -> io::Result<()> {
    remove_path(&paths.old.join(ORIGINAL_COMPLETE))?;
    sync_dir(&paths.old)?;
    remove(&paths.old)
}

/// fsync every file and directory under `root`, including `root` itself.
/// `Ok(false)` when `deadline` passed first (checked around directory fsyncs too).
fn sync_tree(
    root: &Path,
    deadline: Instant,
    sync_directory: fn(&Path, Instant) -> io::Result<()>,
) -> io::Result<bool> {
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir)? {
            if Instant::now() > deadline {
                return Ok(false);
            }
            let path = entry?.path();
            let meta = std::fs::symlink_metadata(&path)?;
            if meta.is_dir() {
                pending.push(path);
            } else if meta.is_file() {
                std::fs::File::open(&path)?.sync_all()?;
            }
        }
        if Instant::now() > deadline {
            return Ok(false);
        }
        sync_directory(&dir, deadline)?;
        if Instant::now() > deadline {
            return Ok(false);
        }
    }
    Ok(Instant::now() <= deadline)
}

/// Take `<live>.reclaim-lock` for the whole open. Refused, not awaited, when
/// another process holds it.
fn lock_open(paths: &ReclaimPaths) -> Result<std::fs::File, StorageError> {
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&paths.lock)?;
    match file.try_lock() {
        Ok(()) => Ok(file),
        Err(std::fs::TryLockError::WouldBlock) => Err(StorageError::Database(format!(
            "another process is opening {} (it holds {}); not touching it",
            paths.live.display(),
            paths.lock.display()
        ))),
        Err(std::fs::TryLockError::Error(e)) => Err(e.into()),
    }
}

/// Resolve whatever an interrupted reclaim left beside `paths.live`, so at
/// most `live` and `old` remain (see the module docs for the rules). `old` is
/// handled by [`open_live`], which must open `live` to decide.
///
/// Errors when an interrupted rename cannot be finished safely. A discarded
/// copy that `remove` cannot delete is logged and left; the caller sees it
/// still exists and does not reclaim this boot.
pub fn recover_leftovers(
    paths: &ReclaimPaths,
    remove: fn(&Path) -> io::Result<()>,
) -> io::Result<()> {
    let parent = paths.parent();
    if !paths.live.exists() && paths.old.exists() {
        if paths.tmp.exists() {
            warn!(
                tmp = %paths.tmp.display(),
                "sync sled reclaim: finishing an interrupted swap (the copy was verified before the swap began)"
            );
            rename_synced(&paths.tmp, &paths.live, &parent)?;
        } else if paths.old.join(ORIGINAL_COMPLETE).try_exists()? {
            warn!(
                old = %paths.old.display(),
                "sync sled reclaim: finishing an interrupted rollback to the original"
            );
            rename_synced(&paths.old, &paths.live, &parent)?;
        } else {
            return Err(io::Error::other(format!(
                "{} is missing and {} has no completeness marker; not restoring a possibly partial original",
                paths.live.display(),
                paths.old.display()
            )));
        }
    }
    for (path, what) in [
        (&paths.tmp, "a copy that never reached the swap"),
        (&paths.rejected, "a copy that did not open"),
    ] {
        if path.exists() {
            warn!(path = %path.display(), "sync sled reclaim: discarding {what}");
            if let Err(e) = remove(path) {
                warn!(
                    path = %path.display(),
                    error = %e,
                    "sync sled reclaim: could not delete it; serving the live sled without reclaiming"
                );
            }
        }
    }
    Ok(())
}

/// How [`open_live`] found the live path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveOpen {
    /// No original beside it.
    Plain,
    /// A reclaimed copy opened and replaces the original (deleted now, or by
    /// the next boot if that failed).
    Promoted,
    /// The reclaimed copy was corrupt; the complete original is live again.
    RolledBack,
}

/// Open `live` for service. When `old` sits beside it, `live` is a fresh
/// reclaimed copy: delete `old` once `live` opens, or roll back on corruption
/// only while `old` still has its completeness marker.
pub fn open_live(
    paths: &ReclaimPaths,
    cache_capacity: u64,
    policy: &ReclaimPolicy,
) -> Result<(sled::Db, LiveOpen), StorageError> {
    let open_err =
        |e: sled::Error| StorageError::Database(format!("Failed to open sync.sled: {e}"));
    if !paths.live.exists() && paths.old.exists() {
        // Opening would create an empty database here, and the rule below
        // would then delete the original as superseded.
        return Err(StorageError::Database(format!(
            "{} is missing beside {}; the reclaim leftovers are unresolved",
            paths.live.display(),
            paths.old.display()
        )));
    }
    match (policy.open)(&paths.live, cache_capacity) {
        Ok(db) if !paths.old.exists() => Ok((db, LiveOpen::Plain)),
        Ok(db) => {
            if let Err(e) = remove_original(paths, policy.remove) {
                // Retried next boot by this same rule; the live copy serves.
                warn!(old = %paths.old.display(), error = %e, "sync sled reclaim: could not delete the original yet");
            }
            Ok((db, LiveOpen::Promoted))
        }
        Err(e) if paths.old.exists() => {
            if !paths.old.join(ORIGINAL_COMPLETE).try_exists()? {
                warn!(
                    old = %paths.old.display(),
                    error = %e,
                    "sync sled reclaim: the original may be partially deleted; keeping live and discarding the stale original"
                );
                if let Err(e) = remove_original(paths, policy.remove) {
                    warn!(old = %paths.old.display(), error = %e, "sync sled reclaim: could not delete the stale original yet");
                }
                return Err(open_err(e));
            }
            if !matches!(e, sled::Error::Corruption { .. }) {
                return Err(open_err(e));
            }
            warn!(
                error = %e,
                "sync sled reclaim: the reclaimed copy is corrupt; rolling back to the complete original"
            );
            let parent = paths.parent();
            // A rename cannot replace a non-empty directory.
            (policy.remove)(&paths.rejected)?;
            rename_synced(&paths.live, &paths.rejected, &parent)?;
            rename_synced(&paths.old, &paths.live, &parent)?;
            if let Err(e) = (policy.remove)(&paths.rejected) {
                warn!(rejected = %paths.rejected.display(), error = %e, "sync sled reclaim: could not delete the rejected copy; next boot will");
            }
            let db = (policy.open)(&paths.live, cache_capacity).map_err(open_err)?;
            Ok((db, LiveOpen::RolledBack))
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

/// Put the count back to `n` (a refusal is not an attempt).
fn restore_attempts(paths: &ReclaimPaths, n: u32) {
    if n == 0 {
        clear_attempts(paths);
    } else if let Err(e) = write_attempts(&paths.attempts, n) {
        warn!(error = %e, "sync sled reclaim: could not restore the attempt count");
    }
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
    // Held to the end of this function; the served sled holds its own lock.
    let _lock = lock_open(&paths)?;
    recover_leftovers(&paths, policy.remove)?;
    let (db, _) = open_live(&paths, cache_capacity, policy)?;
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

    if let Some(left) = [&paths.tmp, &paths.old, &paths.rejected]
        .into_iter()
        .find(|p| p.exists())
    {
        return Ok((
            db,
            report(
                ReclaimOutcome::AbortedError,
                disk_before,
                disk_before,
                0,
                format!(
                    "{} could not be deleted; not reclaiming until it is",
                    left.display()
                ),
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
                0,
                format!(
                    "{unfinished} earlier attempts never finished; delete {} to try again",
                    paths.attempts.display()
                ),
            ),
        ));
    }

    // From here an attempt is in flight: count it until it finishes.
    if let Err(e) = write_attempts(&paths.attempts, unfinished + 1) {
        // Without the count a crash mid-attempt could repeat every boot.
        return Ok((
            db,
            report(
                ReclaimOutcome::AbortedError,
                disk_before,
                disk_before,
                0,
                format!("could not record the attempt: {e}"),
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

    let needed = logical
        .saturating_mul(2)
        .saturating_add(HEADROOM_FIXED_BYTES);
    let available = (policy.free_space)(&paths.parent()).unwrap_or(0);
    if available < needed {
        restore_attempts(&paths, unfinished);
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
        if let Err(e) = (policy.remove)(&paths.tmp) {
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
    match sync_tree(&paths.tmp, deadline, policy.sync_dir) {
        Ok(true) => {}
        Ok(false) => {
            return abort(
                db,
                ReclaimOutcome::AbortedBudget,
                "budget spent fsyncing the copy".into(),
            )
        }
        Err(e) => {
            return abort(
                db,
                ReclaimOutcome::AbortedError,
                format!("fsync of the copy failed: {e}"),
            )
        }
    }
    if let Err(e) = mark_original_complete(&paths.live) {
        return abort(
            db,
            ReclaimOutcome::AbortedError,
            format!("could not mark the original complete: {e}"),
        );
    }
    if Instant::now() > deadline {
        return abort(
            db,
            ReclaimOutcome::AbortedBudget,
            "budget spent marking the original complete".into(),
        );
    }

    // The swap. Close the original first so nothing writes to it mid-rename.
    drop(db);
    let parent = paths.parent();
    let swapped = rename_synced(&paths.live, &paths.old, &parent)
        .and_then(|()| rename_synced(&paths.tmp, &paths.live, &parent));
    let swap_err = swapped.err();
    if let Some(e) = &swap_err {
        // Resolve whichever rename failed by the boot rules, then serve.
        warn!(error = %e, "sync sled reclaim: the swap failed; resolving it");
        recover_leftovers(&paths, policy.remove)?;
    }
    let (db, opened) = open_live(&paths, cache_capacity, policy)?;
    let disk_after = dir_bytes(&paths.live);
    // Which copy serves decides the outcome, not which names remain: a
    // rollback leaves no original beside live either.
    let (outcome, detail) = match (opened, &swap_err) {
        (LiveOpen::RolledBack, _) => (
            ReclaimOutcome::AbortedError,
            "the reclaimed copy did not open; rolled back".to_string(),
        ),
        // Only the first rename can fail and leave the original live.
        (LiveOpen::Plain, Some(e)) => (
            ReclaimOutcome::AbortedError,
            format!("swap failed: {e}; serving the original"),
        ),
        (LiveOpen::Promoted | LiveOpen::Plain, None) => {
            (ReclaimOutcome::Reclaimed, "swapped".to_string())
        }
        (LiveOpen::Promoted, Some(e)) => (
            ReclaimOutcome::Reclaimed,
            format!("swapped after finishing a failed rename: {e}"),
        ),
    };
    if outcome == ReclaimOutcome::Reclaimed {
        clear_attempts(&paths);
    }
    // Otherwise the attempt stays counted.
    Ok((
        db,
        report(outcome, disk_before, disk_after, logical, detail),
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
    fn a_non_corruption_open_error_does_not_roll_back() {
        let dir = tempfile::tempdir().unwrap();
        let paths = ReclaimPaths::new(&dir.path().join("sync.sled"));
        marked_sled(&paths.old, "original");
        mark_original_complete(&paths.old).unwrap();
        // A live path sled cannot open as a database.
        std::fs::write(&paths.live, b"not a sled").unwrap();
        assert!(open_sync_sled(&paths.live, SYNC_SLED_CACHE_BYTES, &off()).is_err());
        assert_eq!(std::fs::read(&paths.live).unwrap(), b"not a sled");
        assert!(paths.old.join(ORIGINAL_COMPLETE).is_file());
        assert!(!paths.rejected.exists());
        let db = open_at(&paths.old, SYNC_SLED_CACHE_BYTES).unwrap();
        assert_eq!(db.get(b"who").unwrap().unwrap().as_ref(), b"original");
    }

    #[test]
    fn a_partial_original_is_not_restored_after_an_open_error() {
        let dir = tempfile::tempdir().unwrap();
        let paths = ReclaimPaths::new(&dir.path().join("sync.sled"));
        marked_sled(&paths.live, "intact-live");
        std::fs::create_dir(&paths.old).unwrap();
        std::fs::write(paths.old.join("remnant"), b"partially deleted").unwrap();
        let failing = ReclaimPolicy {
            open: |_, _| {
                Err(sled::Error::Io(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "refused by the test",
                )))
            },
            remove: |path| {
                assert_eq!(path.file_name().unwrap(), "sync.sled.reclaim-old");
                remove_path(path)
            },
            ..off()
        };
        let err = open_sync_sled(&paths.live, SYNC_SLED_CACHE_BYTES, &failing).unwrap_err();
        assert!(err.to_string().contains("refused by the test"));
        only_live_remains(&paths);
        assert_eq!(served_marker(&paths.live), "intact-live");
    }

    #[test]
    fn a_corrupt_copy_rolls_back_to_a_complete_original() {
        let dir = tempfile::tempdir().unwrap();
        let paths = ReclaimPaths::new(&dir.path().join("sync.sled"));
        marked_sled(&paths.old, "original");
        mark_original_complete(&paths.old).unwrap();
        std::fs::write(&paths.live, b"corrupt copy").unwrap();
        let corrupt = ReclaimPolicy {
            open: |path, cache| {
                if path.is_file() {
                    Err(sled::Error::Corruption { at: None, bt: () })
                } else {
                    open_at(path, cache)
                }
            },
            ..off()
        };
        let (db, opened) = open_live(&paths, SYNC_SLED_CACHE_BYTES, &corrupt).unwrap();
        assert_eq!(opened, LiveOpen::RolledBack);
        assert_eq!(db.get(b"who").unwrap().unwrap().as_ref(), b"original");
        only_live_remains(&paths);
    }

    #[test]
    fn a_partial_original_is_not_restored_even_on_corruption() {
        let dir = tempfile::tempdir().unwrap();
        let paths = ReclaimPaths::new(&dir.path().join("sync.sled"));
        marked_sled(&paths.live, "intact-live");
        std::fs::create_dir(&paths.old).unwrap();
        std::fs::write(paths.old.join("remnant"), b"partially deleted").unwrap();
        let corrupt = ReclaimPolicy {
            open: |_, _| Err(sled::Error::Corruption { at: None, bt: () }),
            ..off()
        };
        assert!(open_sync_sled(&paths.live, SYNC_SLED_CACHE_BYTES, &corrupt).is_err());
        only_live_remains(&paths);
        assert_eq!(served_marker(&paths.live), "intact-live");
    }

    #[test]
    fn deleting_the_original_invalidates_its_marker_first() {
        let dir = tempfile::tempdir().unwrap();
        let paths = ReclaimPaths::new(&dir.path().join("sync.sled"));
        marked_sled(&paths.old, "original");
        mark_original_complete(&paths.old).unwrap();
        marked_sled(&paths.live, "verified-copy");
        let partial_remove = ReclaimPolicy {
            remove: |path| {
                assert!(!path.join(ORIGINAL_COMPLETE).exists());
                std::fs::remove_file(path.join("conf"))?;
                Err(io::Error::other("deletion interrupted by the test"))
            },
            ..off()
        };
        let (db, opened) = open_live(&paths, SYNC_SLED_CACHE_BYTES, &partial_remove).unwrap();
        assert_eq!(opened, LiveOpen::Promoted);
        assert_eq!(db.get(b"who").unwrap().unwrap().as_ref(), b"verified-copy");
        assert!(paths.old.is_dir());
        assert!(!paths.old.join(ORIGINAL_COMPLETE).exists());
        assert!(!paths.old.join("conf").exists());
        drop(db);

        let failing = ReclaimPolicy {
            open: |_, _| Err(sled::Error::Io(io::Error::other("I/O failure by the test"))),
            ..off()
        };
        assert!(open_sync_sled(&paths.live, SYNC_SLED_CACHE_BYTES, &failing).is_err());
        only_live_remains(&paths);
        assert_eq!(served_marker(&paths.live), "verified-copy");
    }

    #[test]
    fn crash_during_a_rollback_finishes_it() {
        let dir = tempfile::tempdir().unwrap();
        let paths = ReclaimPaths::new(&dir.path().join("sync.sled"));
        // After `live → rejected`, before `old → live`.
        marked_sled(&paths.old, "original");
        mark_original_complete(&paths.old).unwrap();
        std::fs::write(&paths.rejected, b"not a sled").unwrap();
        assert_eq!(served_marker(&paths.live), "original");
        only_live_remains(&paths);

        // After `old → live`, before the rejected copy is deleted.
        marked_sled(&paths.rejected, "bad-copy");
        assert_eq!(served_marker(&paths.live), "original");
        only_live_remains(&paths);
    }

    #[test]
    fn a_partial_original_cannot_fill_a_missing_live_path() {
        let dir = tempfile::tempdir().unwrap();
        let paths = ReclaimPaths::new(&dir.path().join("sync.sled"));
        std::fs::create_dir(&paths.old).unwrap();
        std::fs::write(paths.old.join("remnant"), b"partially deleted").unwrap();
        marked_sled(&paths.rejected, "retained-copy");
        assert!(recover_leftovers(&paths, remove_path).is_err());
        assert!(!paths.live.exists());
        assert!(paths.old.join("remnant").exists());
        assert!(paths.rejected.is_dir());
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

    /// Hold `<live>.reclaim-lock` the way another booting process would.
    fn hold_lock(paths: &ReclaimPaths) -> std::fs::File {
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&paths.lock)
            .unwrap();
        file.try_lock().unwrap();
        file
    }

    #[test]
    fn a_second_opener_is_refused_while_one_holds_the_lock() {
        let dir = tempfile::tempdir().unwrap();
        let paths = ReclaimPaths::new(&dir.path().join("sync.sled"));
        marked_sled(&paths.live, "original");
        // Another boot is mid-reclaim: its copy sits beside the original.
        marked_sled(&paths.tmp, "copy-in-progress");
        let held = hold_lock(&paths);
        let second = open_sync_sled(&paths.live, SYNC_SLED_CACHE_BYTES, &policy());
        assert!(second.is_err(), "the second opener is refused");
        assert!(paths.tmp.is_dir(), "the other boot's copy is not touched");
        drop(held);
        assert_eq!(served_marker(&paths.live), "original");
        only_live_remains(&paths);
    }

    #[test]
    fn the_live_path_is_never_created_beside_an_original() {
        let dir = tempfile::tempdir().unwrap();
        let paths = ReclaimPaths::new(&dir.path().join("sync.sled"));
        // Mid-swap: `live → old` done, the copy not yet renamed in.
        marked_sled(&paths.old, "original");
        assert!(open_live(&paths, SYNC_SLED_CACHE_BYTES, &off()).is_err());
        assert!(!paths.live.exists(), "no empty database at the live path");
        let db = open_at(&paths.old, SYNC_SLED_CACHE_BYTES).unwrap();
        assert_eq!(db.get(b"who").unwrap().unwrap().as_ref(), b"original");
    }

    /// A remover that cannot delete the copies a reclaim discards.
    fn stuck_remove(path: &Path) -> io::Result<()> {
        let name = path.to_string_lossy();
        if name.ends_with("reclaim-tmp") || name.ends_with("reclaim-rejected") {
            return Err(io::Error::other("refused by the test"));
        }
        remove_path(path)
    }

    #[test]
    fn a_leftover_that_cannot_be_deleted_does_not_stop_the_boot() {
        let dir = tempfile::tempdir().unwrap();
        let paths = ReclaimPaths::new(&dir.path().join("sync.sled"));
        marked_sled(&paths.live, "original");
        marked_sled(&paths.tmp, "partial-copy");
        std::fs::write(&paths.rejected, b"not a sled").unwrap();

        let disabled = ReclaimPolicy {
            remove: stuck_remove,
            ..off()
        };
        let (db, r) = open_sync_sled(&paths.live, SYNC_SLED_CACHE_BYTES, &disabled)
            .expect("the switch serves the intact original");
        assert_eq!(r.outcome, ReclaimOutcome::Disabled);
        assert_eq!(db.get(b"who").unwrap().unwrap().as_ref(), b"original");
        drop(db);

        let enabled = ReclaimPolicy {
            remove: stuck_remove,
            ..policy()
        };
        let (db, r) = open_sync_sled(&paths.live, SYNC_SLED_CACHE_BYTES, &enabled)
            .expect("an undeletable leftover never stops the boot");
        assert_eq!(r.outcome, ReclaimOutcome::AbortedError, "{r:?}");
        assert!(r.detail.contains("reclaim-"), "the log names it: {r:?}");
        assert_eq!(db.get(b"who").unwrap().unwrap().as_ref(), b"original");
        assert!(!paths.attempts.exists(), "nothing was attempted");
        drop(db);

        // Once the leftovers can go, they go.
        assert_eq!(served_marker(&paths.live), "original");
        only_live_remains(&paths);
    }

    static PROMOTED_OPENS: AtomicU64 = AtomicU64::new(0);

    /// Every boot opens the live path three times on a failed promotion:
    /// the original, the promoted copy (refused here), the rolled-back
    /// original.
    fn refuse_the_promoted_copy(path: &Path, cache: u64) -> sled::Result<sled::Db> {
        if PROMOTED_OPENS.fetch_add(1, Ordering::SeqCst) % 3 == 1 {
            return Err(sled::Error::Corruption { at: None, bt: () });
        }
        open_at(path, cache)
    }

    #[tokio::test]
    async fn a_rolled_back_reclaim_is_not_reported_as_reclaimed() {
        let dir = tempfile::tempdir().unwrap();
        let live = dir.path().join("sync.sled");
        leaky_sled(&live).await;
        let before = {
            let (db, _) = open_sync_sled(&live, SYNC_SLED_CACHE_BYTES, &off()).unwrap();
            contents(&db)
        };
        let paths = ReclaimPaths::new(&live);
        let failing = ReclaimPolicy {
            open: refuse_the_promoted_copy,
            ..policy()
        };
        for attempt in 1..=DEFAULT_MAX_UNFINISHED_ATTEMPTS {
            let (db, r) = open_sync_sled(&live, SYNC_SLED_CACHE_BYTES, &failing).unwrap();
            assert_eq!(r.outcome, ReclaimOutcome::AbortedError, "{r:?}");
            assert_eq!(contents(&db), before, "the original serves");
            assert_eq!(read_attempts(&paths.attempts), attempt, "still unfinished");
            drop(db);
            only_live_remains(&paths);
        }
        let (_db, r) = open_sync_sled(&live, SYNC_SLED_CACHE_BYTES, &failing).unwrap();
        assert_eq!(r.outcome, ReclaimOutcome::RefusedAttempts, "{r:?}");
    }

    #[tokio::test]
    async fn a_budget_abort_counts_as_an_unfinished_attempt() {
        let dir = tempfile::tempdir().unwrap();
        let live = dir.path().join("sync.sled");
        leaky_sled(&live).await;
        let paths = ReclaimPaths::new(&live);
        let hurried = ReclaimPolicy {
            budget: Duration::ZERO,
            ..policy()
        };
        for attempt in 1..=DEFAULT_MAX_UNFINISHED_ATTEMPTS {
            let (db, r) = open_sync_sled(&live, SYNC_SLED_CACHE_BYTES, &hurried).unwrap();
            assert_eq!(r.outcome, ReclaimOutcome::AbortedBudget, "{r:?}");
            assert_eq!(read_attempts(&paths.attempts), attempt);
            drop(db);
        }
        let (_db, r) = open_sync_sled(&live, SYNC_SLED_CACHE_BYTES, &hurried).unwrap();
        assert_eq!(r.outcome, ReclaimOutcome::RefusedAttempts, "{r:?}");
    }

    #[test]
    fn the_fsync_of_the_copy_stops_at_the_deadline() {
        let dir = tempfile::tempdir().unwrap();
        let copy = dir.path().join("copy");
        marked_sled(&copy, "copy");
        let spent = Instant::now() - Duration::from_secs(1);
        assert!(
            !sync_tree(&copy, spent, policy().sync_dir).unwrap(),
            "past the deadline it stops"
        );
        let ample = Instant::now() + Duration::from_secs(60);
        assert!(sync_tree(&copy, ample, policy().sync_dir).unwrap());
    }

    #[test]
    fn a_deadline_during_the_final_directory_fsync_aborts_the_swap() {
        let dir = tempfile::tempdir().unwrap();
        let paths = ReclaimPaths::new(&dir.path().join("sync.sled"));
        marked_sled(&paths.live, "original");
        std::fs::write(paths.live.join("original-only"), b"not copied by sled").unwrap();
        let hurried = ReclaimPolicy {
            min_ratio: 0,
            budget: Duration::from_secs(2),
            sync_dir: |path, deadline| {
                sync_dir(path)?;
                // The copy has only its root and an empty blobs directory,
                // which is visited last by sync_tree.
                if path.file_name().unwrap() == "blobs" {
                    assert!(std::fs::read_dir(path)?.next().is_none());
                    assert!(Instant::now() <= deadline);
                    std::thread::sleep(
                        deadline.saturating_duration_since(Instant::now())
                            + Duration::from_millis(1),
                    );
                    std::fs::write(path.join("deadline-spent"), b"")?;
                }
                Ok(())
            },
            remove: |path| {
                assert_eq!(path.file_name().unwrap(), "sync.sled.reclaim-tmp");
                assert!(path.join("blobs/deadline-spent").exists());
                remove_path(path)
            },
            ..policy()
        };
        let (db, report) = open_sync_sled(&paths.live, SYNC_SLED_CACHE_BYTES, &hurried).unwrap();
        assert_eq!(report.outcome, ReclaimOutcome::AbortedBudget, "{report:?}");
        assert_eq!(report.detail, "budget spent fsyncing the copy");
        assert_eq!(db.get(b"who").unwrap().unwrap().as_ref(), b"original");
        assert_eq!(
            std::fs::read(paths.live.join("original-only")).unwrap(),
            b"not copied by sled"
        );
        assert!(!paths.live.join(ORIGINAL_COMPLETE).exists());
        only_live_remains(&paths);
    }

    #[test]
    fn recovery_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let paths = ReclaimPaths::new(&dir.path().join("sync.sled"));
        marked_sled(&paths.old, "original");
        marked_sled(&paths.tmp, "verified-copy");
        recover_leftovers(&paths, remove_path).unwrap();
        recover_leftovers(&paths, remove_path).unwrap();
        assert_eq!(served_marker(&paths.live), "verified-copy");
        assert_eq!(served_marker(&paths.live), "verified-copy");
        only_live_remains(&paths);
    }
}
