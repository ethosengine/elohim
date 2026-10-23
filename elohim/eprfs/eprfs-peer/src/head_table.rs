//! The head table: what this device knows of every device's channel head, persisted as JSON under
//! the peer state dir (`<dir>/heads.json`) so `epr peer status` and the steward lease view (T6)
//! read what the daemon folded.
//!
//! Field names are stable; T6 reads them:
//!
//! ```json
//! {
//!   "version": 1,
//!   "own": { "device", "root", "seq", "at", "folded": {"<did>": seq}, "signature" },
//!   "own_log": [ { "seq", "root", "at", "events", "last" } ],
//!   "store": { "root", "log" },
//!   "devices": {
//!     "<did>": { "seq", "root", "at", "folded": {"<did>": seq}, "folded_at", "events_folded" }
//!   },
//!   "dropped": { "unadmitted", "forged", "rollback", "replayed", "malformed" },
//!   "channel": { "projection_failures", "last_projection_error", "fold_failures",
//!                "oversized_events", "over_budget_folds" }
//! }
//! ```
//!
//! `own` is this device's last announced head (so `seq` keeps increasing across restarts).
//! `own_log` holds the own heads that announced a new root — the last [`OWN_LOG_LEN`], oldest
//! first; a periodic re-announce of an unchanged root adds none — each with `events`, how many
//! flow events of the channel store the root was projected from, and `last`, the CID of the last
//! of them. The steward lease view (T6) reads it to find the first own head that carried a
//! claim: a claim written as the store's `n`th event is in every head whose `events` is at least
//! `n`, but only while the store holds `last` at position `events` — an entry counted over a
//! store since reset or replaced confirms nothing. `store` names that channel store: the one
//! flow store `epr peer serve` projects, recorded when it starts, which every reader of the
//! steward channel reads (and berth writes its steward records to). A file written before these
//! fields existed loads with an empty log and no store.
//! `devices` holds, per other device, the last head folded: its `seq`, `root`, `at` (that
//! device's clock, display only) and `folded` (its acknowledgement vector), with `folded_at`
//! (this device's clock, unix milliseconds) and the running count of events appended from it.
//! `dropped` counts heads refused: from a device not admitted, failing their signature, with a
//! lower `seq` than the last folded (rollback), the same `seq` again (replay), or not decoding.

use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use eprfs_core::BlobCid;
use serde::{Deserialize, Serialize};

use crate::error::PeerError;
use crate::head::ChannelHead;

/// The table's format version.
pub const HEAD_TABLE_VERSION: u32 = 1;

/// How many of this device's own announces [`HeadTable::own_log`] keeps.
pub const OWN_LOG_LEN: usize = 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeadTable {
    pub version: u32,
    /// This device's last announced head.
    pub own: Option<ChannelHead>,
    /// The own heads that announced a new root, the last [`OWN_LOG_LEN`], oldest first.
    #[serde(default)]
    pub own_log: Vec<OwnAnnounce>,
    /// The flow store this device's channel is projected from, once `epr peer serve` has run.
    #[serde(default)]
    pub store: Option<StorePlace>,
    /// The last head folded from each other device, by did:key.
    pub devices: BTreeMap<String, DeviceHead>,
    pub dropped: DroppedHeads,
    /// The daemon's own channel health: projection and fold failures.
    #[serde(default)]
    pub channel: ChannelCounters,
}

/// What went wrong on this device's side of the channel, so `epr peer status` can say it rather
/// than only the daemon's log.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelCounters {
    /// Projections of the channel tree that failed. While failing, the last good root is still
    /// announced and nothing new crosses.
    pub projection_failures: u64,
    /// Why the latest projection failed; cleared by the next one that succeeds.
    pub last_projection_error: Option<String>,
    /// Folds that failed (unreachable, unanswered, or a tree that does not hold its events at
    /// their CIDs); each is retried on the device's next head.
    pub fold_failures: u64,
    /// Events refused for exceeding the per-event cap.
    pub oversized_events: u64,
    /// Folds stopped at their byte budget.
    pub over_budget_folds: u64,
}

impl Default for HeadTable {
    fn default() -> HeadTable {
        HeadTable {
            version: HEAD_TABLE_VERSION,
            own: None,
            own_log: Vec::new(),
            store: None,
            devices: BTreeMap::new(),
            dropped: DroppedHeads::default(),
            channel: ChannelCounters::default(),
        }
    }
}

/// One of this device's own announces, as the steward lease view needs it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnAnnounce {
    pub seq: u64,
    pub root: BlobCid,
    /// This device's clock when it announced, unix milliseconds.
    pub at: u64,
    /// How many flow events of the channel store `root` was projected from. The store is
    /// append-only, so the head covers exactly its first `events` events.
    pub events: u64,
    /// The CID of the last of them (`None` when `events` is 0): the store's identity at that
    /// position. An entry whose `last` is not the store's `events`th event counted another store.
    #[serde(default)]
    pub last: Option<String>,
}

/// Where this device's channel store is: the checkout `epr peer serve` projects, and its flow
/// log, both canonical paths.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorePlace {
    pub root: PathBuf,
    pub log: PathBuf,
}

/// The last head folded from one device.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceHead {
    pub seq: u64,
    pub root: BlobCid,
    /// The device's own clock when it announced, unix milliseconds. Display only.
    pub at: u64,
    /// Its acknowledgement vector: for each device, the highest seq it has folded from it.
    pub folded: BTreeMap<String, u64>,
    /// This device's clock when the head was folded, unix milliseconds.
    pub folded_at: u64,
    /// Events appended from this device so far.
    pub events_folded: u64,
}

/// Heads refused, by reason. Refused heads are never folded and never re-announced.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DroppedHeads {
    pub unadmitted: u64,
    pub forged: u64,
    pub rollback: u64,
    pub replayed: u64,
    pub malformed: u64,
}

impl HeadTable {
    /// The table's file name inside the peer state dir.
    pub const FILE_NAME: &'static str = "heads.json";

    /// The table persisted in `dir`, or an empty one when there is none yet.
    ///
    /// # Errors
    ///
    /// [`PeerError::HeadTableIo`] when it cannot be read; [`PeerError::HeadTableInvalid`] when it
    /// does not parse. A table that does not parse is never reset: its own `seq` is what keeps
    /// this device's heads increasing.
    pub fn load(dir: &Path) -> Result<HeadTable, PeerError> {
        let path = dir.join(Self::FILE_NAME);
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(HeadTable::default()),
            Err(source) => return Err(PeerError::HeadTableIo { path, source }),
        };
        serde_json::from_str(&text).map_err(|e| PeerError::HeadTableInvalid {
            path,
            reason: e.to_string(),
        })
    }

    /// Persist the table in `dir`, replacing the previous one atomically (a reader sees the old
    /// table or the new one, never half of either).
    ///
    /// # Errors
    ///
    /// [`PeerError::HeadTableIo`] when the directory or the file cannot be written.
    pub fn save(&self, dir: &Path) -> Result<(), PeerError> {
        let path = dir.join(Self::FILE_NAME);
        let io_error = |path: &Path| {
            let path = path.to_path_buf();
            move |source| PeerError::HeadTableIo { path, source }
        };
        fs::create_dir_all(dir).map_err(io_error(dir))?;
        let text = serde_json::to_string_pretty(self).map_err(|e| PeerError::HeadTableInvalid {
            path: path.clone(),
            reason: e.to_string(),
        })?;
        let tmp = dir.join(format!(".{}.{}.tmp", Self::FILE_NAME, std::process::id()));
        let written = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&tmp)
            .and_then(|mut file| {
                file.write_all(text.as_bytes())?;
                file.write_all(b"\n")?;
                file.sync_all()
            })
            .and_then(|()| fs::rename(&tmp, &path))
            .and_then(|()| File::open(dir)?.sync_all());
        if written.is_err() {
            // Best effort: the write error is the one worth reporting.
            let _ = fs::remove_file(&tmp);
        }
        written.map_err(io_error(&path))
    }
}
