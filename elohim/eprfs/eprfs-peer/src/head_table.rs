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
//!   "rules": { "digest", "policy", "loadedAt", "pending" },
//!   "devices": {
//!     "<did>": { "seq", "root", "at", "folded": {"<did>": seq}, "folded_at", "events_folded", "nack" }
//!   },
//!   "early_acked": { "<announcing-did>": { "<acknowledged-did>": seq } },
//!   "nacks": { "<author-did>": { "refused", "quarantined" } },
//!   "dropped": { "unadmitted", "forged", "rollback", "replayed", "malformed" },
//!   "crossing_drop_groups": {
//!     "<rules-digest>": {
//!       "<author-did>": { "<measure>": { "<kind>": { "count", "first_seen", "last_seen" } } }
//!     }
//!   },
//!   "crossing_drop_overflows": {
//!     "<rules-digest>": { "count", "first_seen", "last_seen", "devices": { "<author-did>": { "count", "first_seen", "last_seen" } } }
//!   },
//!   "evicted_totals": { "<author-did>": { "count", "last_seen" } },
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
//! A device's optional `coverage` is in that SAME JSON object and atomic file replacement as its
//! head. It names the complete fold's flow-log prefix: a sender event whose JSONL line starts below
//! `log_offset` is covered. Older epr builds ignore this unknown head-table field; no new flow
//! record or field is written to `flows.jsonl`.
//! `early_acked` is deliberately separate from `devices`: a verified, admitted, forward head can
//! acknowledge this device before its content fold finishes, but cannot advance rollback order,
//! root, coverage, freshness, nacks, or the acknowledgement this device sends back. Its values
//! rise monotonically unless that announcing head's content is later refused as invalid.
//! `dropped` counts heads refused: from a device not admitted, failing their signature, with a
//! lower `seq` than the last folded (rollback), the same `seq` again (replay), or not decoding.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use eprfs_core::BlobCid;
use serde::{Deserialize, Serialize};

use crate::error::PeerError;
use crate::head::ChannelHead;
use crate::rules::RulesState;

/// The table's format version.
pub const HEAD_TABLE_VERSION: u32 = 1;

/// How many of this device's own announces [`HeadTable::own_log`] keeps.
pub const OWN_LOG_LEN: usize = 1024;

/// The most refused CIDs one nack leaf lists. The total remains exact when the list overflows.
pub const MAX_NACK_CIDS: usize = 64;

/// The most rules generations whose receiver-side crossing refusals remain in the head table.
pub const MAX_CROSSING_DROP_RULE_GROUPS: usize = 3;

/// The most distinct device/measure/kind refusal rows persisted for one rules digest.
pub const MAX_CROSSING_DROP_ROWS_PER_RULE_GROUP: usize = 32;

/// One nack leaf as it crosses at `channel/nacks/<author>`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NackLeaf {
    #[serde(default)]
    pub refused: Vec<String>,
    #[serde(default)]
    pub quarantined: Vec<String>,
    #[serde(default)]
    pub refused_total: u64,
}

impl NackLeaf {
    pub fn contains(&self, cid: &str) -> bool {
        self.refused.iter().any(|listed| listed == cid)
            || self.quarantined.iter().any(|listed| listed == cid)
    }

    pub fn overflowed(&self) -> bool {
        self.refused_total
            > u64::try_from(self.refused.len().saturating_add(self.quarantined.len()))
                .unwrap_or(u64::MAX)
    }
}

/// This device's complete, private nack sets. Projection caps their public leaf.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NackState {
    #[serde(default)]
    pub refused: BTreeSet<String>,
    #[serde(default)]
    pub quarantined: BTreeSet<String>,
}

impl NackState {
    pub fn leaf(&self) -> NackLeaf {
        let mut left = MAX_NACK_CIDS;
        let refused: Vec<String> = self.refused.iter().take(left).cloned().collect();
        left = left.saturating_sub(refused.len());
        let quarantined = self.quarantined.iter().take(left).cloned().collect();
        NackLeaf {
            refused,
            quarantined,
            refused_total: u64::try_from(self.refused.len().saturating_add(self.quarantined.len()))
                .unwrap_or(u64::MAX),
        }
    }
}

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
    /// The rules the daemon loaded, so `epr peer status` and berth read them from the daemon,
    /// never from a session's worktree.
    #[serde(default)]
    pub rules: Option<RulesState>,
    /// The last head folded from each other device, by did:key.
    pub devices: BTreeMap<String, DeviceHead>,
    /// Acknowledgement vectors accepted from verified forward heads before their content fold.
    #[serde(default)]
    pub early_acked: BTreeMap<String, BTreeMap<String, u64>>,
    /// CIDs this device refused from each author, projected into this device's channel tree.
    #[serde(default)]
    pub nacks: BTreeMap<String, NackState>,
    pub dropped: DroppedHeads,
    /// Receiver-side event refusals, by loaded rules digest, sending device, measure and kind.
    #[serde(default)]
    pub crossing_drop_groups: CrossingDropGroups,
    /// Refusals whose distinct row would exceed the per-rules row bound. The aggregate feeds
    /// status; the per-device counters preserve exact attribution if the rules group is evicted.
    #[serde(default)]
    pub crossing_drop_overflows: BTreeMap<String, CrossingDropOverflow>,
    /// Running refusal totals from rules groups no longer retained, by sending device.
    #[serde(default)]
    pub evicted_totals: BTreeMap<String, EvictedCrossingDropTotal>,
    /// The ungrouped field written before refusals carried the loaded rules digest. Loading folds
    /// it into the `unknown` history group; saving never writes this field again.
    #[serde(default, rename = "crossing_drops", skip_serializing)]
    pub legacy_crossing_drops: CrossingDrops,
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
    /// Connections refused per node id (64 lowercase hex) the roster names: a contested device's
    /// new ones, and the open ones a reload closed.
    #[serde(default)]
    pub refused_connections: BTreeMap<String, u64>,
    /// Connections refused from node ids the roster does not name, in one total.
    #[serde(default)]
    pub refused_strangers: u64,
}

impl Default for HeadTable {
    fn default() -> HeadTable {
        HeadTable {
            version: HEAD_TABLE_VERSION,
            own: None,
            own_log: Vec::new(),
            store: None,
            rules: None,
            devices: BTreeMap::new(),
            early_acked: BTreeMap::new(),
            nacks: BTreeMap::new(),
            dropped: DroppedHeads::default(),
            crossing_drop_groups: BTreeMap::new(),
            crossing_drop_overflows: BTreeMap::new(),
            evicted_totals: BTreeMap::new(),
            legacy_crossing_drops: BTreeMap::new(),
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
    /// This peer's nack leaf for this device, read from the same signed channel root.
    #[serde(default)]
    pub nack: NackLeaf,
    /// The complete prefix this head covered. Missing or malformed coverage deliberately reads as
    /// `None`, preserving the pre-coverage reader behavior instead of refusing the head table.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_coverage"
    )]
    pub coverage: Option<FoldCoverage>,
}

/// The sender events covered by one completed [`DeviceHead`] fold.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoldCoverage {
    /// Accepted events from this sender below [`Self::log_offset`].
    pub events: u64,
    /// The byte offset immediately after the captured `flows.jsonl` prefix. A record belongs to
    /// the prefix when its line-start offset is strictly less than this value.
    pub log_offset: u64,
}

fn deserialize_coverage<'de, D>(deserializer: D) -> Result<Option<FoldCoverage>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(serde_json::from_value(value).ok())
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

/// Receiver-side allow-list refusals, grouped as device -> measure -> kind -> count and times.
pub type CrossingDrops = BTreeMap<String, BTreeMap<String, BTreeMap<String, CrossingDrop>>>;

/// Receiver-side allow-list refusals grouped first by the digest of the rules that refused them.
pub type CrossingDropGroups = BTreeMap<String, CrossingDrops>;

/// One receiver-side allow-list refusal counter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrossingDrop {
    pub count: u64,
    /// This device's clock at the first refusal, unix milliseconds.
    pub first_seen: u64,
    /// This device's clock at the latest refusal, unix milliseconds.
    pub last_seen: u64,
}

/// Refusals that could not receive their own persisted row in one rules-digest group.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrossingDropOverflow {
    pub count: u64,
    /// This device's clock at the first overflowed refusal, unix milliseconds.
    pub first_seen: u64,
    /// This device's clock at the latest overflowed refusal, unix milliseconds.
    pub last_seen: u64,
    /// Exact sender accounting retained without persisting another measure/kind row.
    pub devices: BTreeMap<String, CrossingDrop>,
}

/// Refusals retained after their rules-digest group has been evicted.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvictedCrossingDropTotal {
    pub count: u64,
    /// This device's clock at the latest refusal in an evicted group, unix milliseconds.
    pub last_seen: u64,
}

impl HeadTable {
    /// The table's file name inside the peer state dir.
    pub const FILE_NAME: &'static str = "heads.json";

    /// Count one distinct event refused by this device's crossing allow-list.
    pub fn record_crossing_drop(
        &mut self,
        rules_digest: &str,
        device: &str,
        measure: &str,
        kind: &str,
        at: u64,
    ) {
        let rows = self
            .crossing_drop_groups
            .entry(rules_digest.to_owned())
            .or_default();
        if let Some(drop) = rows
            .get_mut(device)
            .and_then(|measures| measures.get_mut(measure))
            .and_then(|kinds| kinds.get_mut(kind))
        {
            count_crossing_drop(drop, 1, at, at);
        } else if crossing_drop_row_count(rows) < MAX_CROSSING_DROP_ROWS_PER_RULE_GROUP {
            rows.entry(device.to_owned())
                .or_default()
                .entry(measure.to_owned())
                .or_default()
                .insert(
                    kind.to_owned(),
                    CrossingDrop {
                        count: 1,
                        first_seen: at,
                        last_seen: at,
                    },
                );
        } else {
            let overflow = self
                .crossing_drop_overflows
                .entry(rules_digest.to_owned())
                .or_default();
            count_overflow(overflow, device, 1, at, at);
        }
        self.trim_crossing_drop_groups();
    }

    /// The persisted receiver-side refusal rows for one rules digest, in deterministic order.
    pub fn crossing_drop_entries(
        &self,
        rules_digest: &str,
    ) -> impl Iterator<Item = (&str, &str, &str, &CrossingDrop)> {
        self.crossing_drop_groups
            .get(rules_digest)
            .into_iter()
            .flat_map(|rows| {
                rows.iter().flat_map(|(device, measures)| {
                    measures.iter().flat_map(move |(measure, kinds)| {
                        kinds.iter().map(move |(kind, drop)| {
                            (device.as_str(), measure.as_str(), kind.as_str(), drop)
                        })
                    })
                })
            })
    }

    /// The bounded rules groups in deterministic digest order.
    pub fn crossing_drop_rule_groups(&self) -> impl Iterator<Item = (&str, &CrossingDrops)> {
        self.crossing_drop_groups
            .iter()
            .map(|(digest, rows)| (digest.as_str(), rows))
    }

    /// The aggregate for distinct rows suppressed by one rules digest's persisted-row bound.
    pub fn crossing_drop_overflow(&self, rules_digest: &str) -> Option<&CrossingDropOverflow> {
        self.crossing_drop_overflows.get(rules_digest)
    }

    /// Running refusal totals from evicted rules generations, in sending-device order.
    pub fn evicted_crossing_drop_totals(
        &self,
    ) -> impl Iterator<Item = (&str, &EvictedCrossingDropTotal)> {
        self.evicted_totals
            .iter()
            .map(|(device, total)| (device.as_str(), total))
    }

    fn trim_crossing_drop_groups(&mut self) {
        while self.crossing_drop_groups.len() > MAX_CROSSING_DROP_RULE_GROUPS {
            let Some(oldest) = self
                .crossing_drop_groups
                .iter()
                .min_by_key(|(digest, rows)| {
                    (
                        crossing_drop_latest(rows).max(
                            self.crossing_drop_overflows
                                .get(*digest)
                                .map_or(0, |overflow| overflow.last_seen),
                        ),
                        digest.as_str(),
                    )
                })
                .map(|(digest, _)| digest.clone())
            else {
                break;
            };
            if let Some(rows) = self.crossing_drop_groups.remove(&oldest) {
                self.remember_evicted_rows(rows);
            }
            if let Some(overflow) = self.crossing_drop_overflows.remove(&oldest) {
                self.remember_evicted_overflow(overflow);
            }
        }
    }

    fn normalize_crossing_drop_rows(&mut self) {
        for (digest, rows) in &mut self.crossing_drop_groups {
            let overflow = self
                .crossing_drop_overflows
                .entry(digest.clone())
                .or_default();
            while crossing_drop_row_count(rows) > MAX_CROSSING_DROP_ROWS_PER_RULE_GROUP {
                let Some((device, measure, kind, drop)) = pop_last_crossing_drop(rows) else {
                    break;
                };
                count_overflow(
                    overflow,
                    &device,
                    drop.count,
                    drop.first_seen,
                    drop.last_seen,
                );
                remove_empty_crossing_drop_maps(rows, &device, &measure, &kind);
            }
        }
        self.crossing_drop_overflows.retain(|digest, overflow| {
            self.crossing_drop_groups.contains_key(digest) && overflow.count > 0
        });
    }

    fn remember_evicted_rows(&mut self, rows: CrossingDrops) {
        for (device, measures) in rows {
            for kinds in measures.into_values() {
                for drop in kinds.into_values() {
                    self.remember_evicted(&device, drop.count, drop.last_seen);
                }
            }
        }
    }

    fn remember_evicted_overflow(&mut self, overflow: CrossingDropOverflow) {
        for (device, drop) in overflow.devices {
            self.remember_evicted(&device, drop.count, drop.last_seen);
        }
    }

    fn remember_evicted(&mut self, device: &str, count: u64, last_seen: u64) {
        let total = self.evicted_totals.entry(device.to_owned()).or_default();
        total.count = total.count.saturating_add(count);
        total.last_seen = total.last_seen.max(last_seen);
    }

    fn migrate_legacy_crossing_drops(&mut self) {
        let legacy = std::mem::take(&mut self.legacy_crossing_drops);
        if legacy.is_empty() {
            return;
        }
        let unknown = self
            .crossing_drop_groups
            .entry("unknown".to_owned())
            .or_default();
        merge_crossing_drops(unknown, legacy);
        self.normalize_crossing_drop_rows();
        self.trim_crossing_drop_groups();
    }

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
        let mut table: HeadTable =
            serde_json::from_str(&text).map_err(|e| PeerError::HeadTableInvalid {
                path,
                reason: e.to_string(),
            })?;
        table.migrate_legacy_crossing_drops();
        table.normalize_crossing_drop_rows();
        table.trim_crossing_drop_groups();
        Ok(table)
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
        let mut persisted = self.clone();
        persisted.normalize_crossing_drop_rows();
        persisted.trim_crossing_drop_groups();
        let text =
            serde_json::to_string_pretty(&persisted).map_err(|e| PeerError::HeadTableInvalid {
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

fn crossing_drop_row_count(rows: &CrossingDrops) -> usize {
    rows.values()
        .flat_map(BTreeMap::values)
        .map(BTreeMap::len)
        .sum()
}

fn count_crossing_drop(drop: &mut CrossingDrop, count: u64, first_seen: u64, last_seen: u64) {
    drop.count = drop.count.saturating_add(count);
    drop.first_seen = drop.first_seen.min(first_seen);
    drop.last_seen = drop.last_seen.max(last_seen);
}

fn count_overflow(
    overflow: &mut CrossingDropOverflow,
    device: &str,
    count: u64,
    first_seen: u64,
    last_seen: u64,
) {
    if overflow.count == 0 {
        overflow.first_seen = first_seen;
    } else {
        overflow.first_seen = overflow.first_seen.min(first_seen);
    }
    overflow.count = overflow.count.saturating_add(count);
    overflow.last_seen = overflow.last_seen.max(last_seen);
    let device_total = overflow
        .devices
        .entry(device.to_owned())
        .or_insert(CrossingDrop {
            count: 0,
            first_seen,
            last_seen,
        });
    count_crossing_drop(device_total, count, first_seen, last_seen);
}

fn pop_last_crossing_drop(
    rows: &mut CrossingDrops,
) -> Option<(String, String, String, CrossingDrop)> {
    let (device, measures) = rows.last_key_value()?;
    let (measure, kinds) = measures.last_key_value()?;
    let (kind, drop) = kinds.last_key_value()?;
    Some((device.clone(), measure.clone(), kind.clone(), *drop))
}

fn remove_empty_crossing_drop_maps(
    rows: &mut CrossingDrops,
    device: &str,
    measure: &str,
    kind: &str,
) {
    let remove_measure = rows
        .get_mut(device)
        .and_then(|measures| measures.get_mut(measure))
        .is_some_and(|kinds| {
            kinds.remove(kind);
            kinds.is_empty()
        });
    let remove_device = if remove_measure {
        rows.get_mut(device).is_some_and(|measures| {
            measures.remove(measure);
            measures.is_empty()
        })
    } else {
        false
    };
    if remove_device {
        rows.remove(device);
    }
}

fn crossing_drop_latest(rows: &CrossingDrops) -> u64 {
    rows.values()
        .flat_map(BTreeMap::values)
        .flat_map(BTreeMap::values)
        .map(|drop| drop.last_seen)
        .max()
        .unwrap_or(0)
}

fn merge_crossing_drops(target: &mut CrossingDrops, source: CrossingDrops) {
    for (device, measures) in source {
        for (measure, kinds) in measures {
            for (kind, incoming) in kinds {
                let drop = target
                    .entry(device.clone())
                    .or_default()
                    .entry(measure.clone())
                    .or_default()
                    .entry(kind)
                    .or_insert(CrossingDrop {
                        count: 0,
                        first_seen: incoming.first_seen,
                        last_seen: incoming.last_seen,
                    });
                drop.count = drop.count.saturating_add(incoming.count);
                drop.first_seen = drop.first_seen.min(incoming.first_seen);
                drop.last_seen = drop.last_seen.max(incoming.last_seen);
            }
        }
    }
}
