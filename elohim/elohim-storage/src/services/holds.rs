//! Holds: why this peer holds each blob in its blob store, and the letting go
//! of the ones it brought here itself that nothing names any more.
//!
//! The rule (`genesis/docs/superpowers/specs/2026-10-06-device-footprint-residency-carrying-capacity-design.md`
//! §4): bytes a peer brought here by its own act, and that nothing names any
//! more, are its own to let go. Bytes another peer placed here are that peer's
//! to withdraw. Bytes with no record of how they arrived are reported and never
//! let go by this pass.
//!
//! "Named" means one of: a kept release names it, a content row names it (what
//! this peer serves; an item pin holds through its content row), a live
//! `custody-blob` commitment names it, or it is a shard of a blob that is
//! itself held.
//!
//! How a blob arrived is read from `db::blob_arrivals`, which every storing
//! path writes. For blobs that arrived before that record existed, two older
//! local rows stand in: a self-custody inventory row or a `blob:` manifest
//! (this peer put it), and a `serve-blob` event naming this node as receiver
//! (this peer fetched it). A blob with none of these is unrecorded.
//!
//! The pass runs after the release window pass, on the same tick
//! ([`super::release_adoption::retention::spawn`]). Its account is the `holds`
//! key of node-local `GET /admin/adoption` and the `elohim_node_holds_*`
//! gauges. It never leaves the node: what a peer holds says what its person
//! uses.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::atomic::Ordering;
use std::sync::Mutex;

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use super::release_adoption::retention::{split_window, RetentionSweeper};
use crate::blob_store::BlobStore;
use crate::db::blob_arrivals::{self, Arrival, ArrivalVia, UnnamedWatch};
use crate::db::diesel_schema::{
    content, economic_events, peer_blob_inventory, rea_commitments, shard_manifests,
};
use crate::db::release_ledger::{self, LedgerArtifact};
use crate::error::StorageError;

/// bounded-work: how many unnamed blobs one pass looks at closely (the
/// authoritative per-blob read, the watch count, and letting go). The rest are
/// reported and reached on later passes: the starting point rotates.
pub const MAX_UNNAMED_PER_PASS: usize = 64;

/// How many unnamed blobs the report lists one by one.
const MAX_LISTED: usize = 100;

/// Why a blob is held, strongest reason first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HoldReason {
    /// A release inside the window names it.
    KeptRelease,
    /// A content row names it: this peer serves it.
    Served,
    /// A live custody commitment names it.
    Pledged,
    /// It is a shard of a blob that is itself held.
    PartOf,
    /// Another peer placed it here and has not withdrawn it.
    Placed,
    /// The adoption controller is pulling it for a release it is taking up.
    Arriving,
    /// This peer brought it here itself and nothing above names it.
    OwnUnnamed,
    /// Nothing above names it and no record says how it arrived.
    Unrecorded,
}

impl HoldReason {
    pub const ALL: [HoldReason; 8] = [
        HoldReason::KeptRelease,
        HoldReason::Served,
        HoldReason::Pledged,
        HoldReason::PartOf,
        HoldReason::Placed,
        HoldReason::Arriving,
        HoldReason::OwnUnnamed,
        HoldReason::Unrecorded,
    ];

    pub fn label(self) -> &'static str {
        match self {
            HoldReason::KeptRelease => "kept_release",
            HoldReason::Served => "served",
            HoldReason::Pledged => "pledged",
            HoldReason::PartOf => "part_of",
            HoldReason::Placed => "placed",
            HoldReason::Arriving => "arriving",
            HoldReason::OwnUnnamed => "own_unnamed",
            HoldReason::Unrecorded => "unrecorded",
        }
    }
}

/// What is known about one blob. Every field is a fact read from this peer's
/// own tables; none is inferred from the file.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BlobFacts {
    pub in_kept_release: bool,
    pub served: bool,
    pub pledged: bool,
    /// A shard of a blob that is itself held for one of the reasons above.
    pub part_of_held: bool,
    /// A placement by another peer that it has not withdrawn.
    pub placed: bool,
    /// Pulled for a release the adoption controller is taking up right now.
    pub arriving: bool,
    /// A record that makes it this peer's to let go: it put it, fetched it or
    /// pulled it, or another peer placed it and has since withdrawn.
    pub own_arrival: bool,
}

/// Why a blob is held. The order is the order an operator would act on:
/// what this peer answers for first, what it merely has last.
pub fn hold_reason(facts: &BlobFacts) -> HoldReason {
    if facts.in_kept_release {
        HoldReason::KeptRelease
    } else if facts.served {
        HoldReason::Served
    } else if facts.pledged {
        HoldReason::Pledged
    } else if facts.part_of_held {
        HoldReason::PartOf
    } else if facts.placed {
        HoldReason::Placed
    } else if facts.arriving {
        HoldReason::Arriving
    } else if facts.own_arrival {
        HoldReason::OwnUnnamed
    } else {
        HoldReason::Unrecorded
    }
}

/// The declared bounds on letting a blob go.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Seconds since arrival before a blob nothing names may go.
    pub min_age_secs: u64,
    /// Consecutive passes it must have read as unnamed.
    pub min_passes: u64,
}

/// Whether a blob may be let go. Only a blob this peer brought here itself,
/// unnamed on enough consecutive passes and old enough. An age that could not
/// be read is no age: the blob stays.
pub fn may_let_go(reason: HoldReason, age_secs: Option<u64>, passes: u64, limits: Limits) -> bool {
    reason == HoldReason::OwnUnnamed
        && passes >= limits.min_passes.max(2)
        && age_secs.is_some_and(|age| age >= limits.min_age_secs)
}

/// Blobs and bytes held for one reason.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize)]
pub struct Tally {
    pub blobs: usize,
    pub bytes: u64,
}

/// One blob that reads as this peer's own and named by nothing.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnnamedBlob {
    pub blob_hash: String,
    pub bytes: u64,
    /// How it arrived, as recorded.
    pub arrived_via: Vec<&'static str>,
    pub arrived_at: Option<String>,
    /// Consecutive passes it has read as unnamed. 0 until a pass has looked
    /// at it closely.
    pub passes: u64,
}

/// One pass's account of the blob store.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HoldsReport {
    pub at_unix_secs: i64,
    pub min_age_secs: u64,
    pub min_passes: u64,
    /// The blob store by reason.
    pub blobs: BTreeMap<&'static str, Tally>,
    /// The blobs that read as own and unnamed, largest first, at most
    /// `MAX_LISTED`.
    pub own_unnamed: Vec<UnnamedBlob>,
    /// Unnamed blobs this pass did not look at closely.
    pub unexamined: usize,
    /// Blobs this pass let go, and the blob-store bytes that freed.
    pub released: Vec<String>,
    pub bytes_released: u64,
    /// Blobs a step failed on. They are tried again next pass.
    pub failures: usize,
}

/// A blob a pass let go, kept for the account after that pass is no longer
/// the last one.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LetGo {
    pub blob_hash: String,
    pub at_unix_secs: i64,
}

/// How many let-go blobs the account remembers.
const MAX_RECENT: usize = 100;

static LAST_PASS: Mutex<Option<HoldsReport>> = Mutex::new(None);
static RECENTLY_LET_GO: Mutex<Vec<LetGo>> = Mutex::new(Vec::new());

/// The last pass, for `GET /admin/adoption`, with the blobs let go by this
/// process's recent passes (newest last). `null` before the first pass.
pub fn report_json() -> serde_json::Value {
    let Some(report) = LAST_PASS.lock().unwrap().clone() else {
        return serde_json::Value::Null;
    };
    let mut body = serde_json::to_value(&report).unwrap_or(serde_json::Value::Null);
    if let Some(map) = body.as_object_mut() {
        map.insert(
            "recentlyLetGo".to_string(),
            serde_json::to_value(&*RECENTLY_LET_GO.lock().unwrap())
                .unwrap_or(serde_json::Value::Null),
        );
    }
    body
}

/// Publish a pass: the admin report and the gauges.
pub fn publish(report: &HoldsReport) {
    for (reason, tally) in &report.blobs {
        crate::metrics::set_node_holds("blobs", reason, tally.blobs, tally.bytes);
    }
    crate::metrics::add_node_holds_released(report.released.len(), report.bytes_released);
    crate::metrics::add_release_retention_failures(report.failures);
    {
        let mut recent = RECENTLY_LET_GO.lock().unwrap();
        recent.extend(report.released.iter().map(|blob_hash| LetGo {
            blob_hash: blob_hash.clone(),
            at_unix_secs: report.at_unix_secs,
        }));
        let overflow = recent.len().saturating_sub(MAX_RECENT);
        recent.drain(..overflow);
    }
    *LAST_PASS.lock().unwrap() = Some(report.clone());
}

/// Everything the pass reads from the database, in one blocking task.
struct Tables {
    kept: HashSet<String>,
    /// Hashes and CIDs content rows name in their own columns.
    served: HashSet<String>,
    pledged: HashSet<String>,
    /// `(blob, its shards)` for every manifest.
    manifests: Vec<(String, Vec<String>)>,
    arrivals: HashMap<String, Vec<Arrival>>,
    /// Older evidence of this peer's own act: hash to when.
    own_evidence: HashMap<String, (ArrivalVia, String)>,
    watch: HashMap<String, UnnamedWatch>,
}

fn read_tables(
    conn: &mut SqliteConnection,
    depth: usize,
    in_use: &HashSet<String>,
    self_cid: Option<&str>,
) -> Result<Tables, StorageError> {
    let db =
        |what: &str, e: diesel::result::Error| StorageError::Database(format!("holds {what}: {e}"));

    let releases = release_ledger::list_releases(conn)?;
    let kept = split_window(&releases, depth, in_use)
        .kept
        .iter()
        .flat_map(|r| r.artifacts.iter().map(LedgerArtifact::blob_hash))
        .collect();

    let mut served = HashSet::new();
    let rows: Vec<(Option<String>, Option<String>, Option<String>)> = content::table
        .select((
            content::blob_hash,
            content::server_blob_hash,
            content::blob_cid,
        ))
        .filter(
            content::blob_hash
                .is_not_null()
                .or(content::server_blob_hash.is_not_null())
                .or(content::blob_cid.is_not_null()),
        )
        .load(conn)
        .map_err(|e| db("content rows", e))?;
    for (blob, server, cid) in rows {
        served.extend([blob, server, cid].into_iter().flatten());
    }

    let pledged_rows: Vec<Option<String>> = rea_commitments::table
        .filter(rea_commitments::action.eq("custody-blob"))
        .filter(rea_commitments::state.ne_all(crate::db::models::commitment_withdrawn_states::ALL))
        .select(rea_commitments::resource_classified_as)
        .load(conn)
        .map_err(|e| db("custody commitments", e))?;
    // A commitment names its blob bare or as a JSON list, and as `sha256-<hex>`,
    // bare hex, a CID, or a CID behind a `sha256-` prefix. Each is brought to
    // the blob store's own spelling; one that cannot be read is left to the
    // per-blob read, which matches the digest anywhere in the field.
    let pledged = pledged_rows
        .iter()
        .flat_map(|raw| crate::db::rea_commitments::classifications_of(raw.as_deref()))
        .filter_map(|named| store_spelling(&named))
        .collect();

    let manifest_rows: Vec<(String, String)> = shard_manifests::table
        .select((
            shard_manifests::blob_hash,
            shard_manifests::shard_hashes_json,
        ))
        .load(conn)
        .map_err(|e| db("manifests", e))?;
    let mut own_evidence: HashMap<String, (ArrivalVia, String)> = HashMap::new();
    let manifest_ids: Vec<(String, String, String)> = shard_manifests::table
        .filter(shard_manifests::content_id.like("blob:%"))
        .select((
            shard_manifests::blob_hash,
            shard_manifests::content_id,
            shard_manifests::created_at,
        ))
        .load(conn)
        .map_err(|e| db("put manifests", e))?;
    for (blob_hash, _, created_at) in manifest_ids {
        own_evidence.insert(blob_hash, (ArrivalVia::SelfPut, created_at));
    }
    let manifests = manifest_rows
        .into_iter()
        .map(|(blob, json)| (blob, serde_json::from_str(&json).unwrap_or_default()))
        .collect();

    // Match on `source`, never on `peer_id`: a peer that has not learned its
    // own id writes these rows under a placeholder.
    let self_custody: Vec<(String, String)> = peer_blob_inventory::table
        .filter(peer_blob_inventory::source.eq("self-custody"))
        .select((
            peer_blob_inventory::blob_hash,
            peer_blob_inventory::last_seen_at,
        ))
        .load(conn)
        .map_err(|e| db("self-custody rows", e))?;
    for (blob_hash, at) in self_custody {
        own_evidence.insert(blob_hash, (ArrivalVia::SelfPut, at));
    }
    // A fetch this node made: the event names it as receiver. Another peer's
    // fetch event can be projected here too, so the receiver has to match.
    if let Some(self_cid) = self_cid {
        let fetched: Vec<(Option<String>, String)> = economic_events::table
            .filter(economic_events::action.eq("serve-blob"))
            .filter(economic_events::receiver.eq(self_cid))
            .select((
                economic_events::resource_inventoried_as,
                economic_events::has_point_in_time,
            ))
            .load(conn)
            .map_err(|e| db("fetch events", e))?;
        for (blob_hash, at) in fetched {
            let Some(blob_hash) = blob_hash else { continue };
            let newer = own_evidence
                .get(&blob_hash)
                .is_none_or(|(_, seen)| *seen < at);
            if newer {
                own_evidence.insert(blob_hash, (ArrivalVia::SelfFetch, at));
            }
        }
    }

    Ok(Tables {
        kept,
        served,
        pledged,
        manifests,
        arrivals: blob_arrivals::arrivals_by_hash(conn)?,
        own_evidence,
        watch: blob_arrivals::watched(conn)?,
    })
}

/// A blob address in any of its spellings, as the blob store spells it.
fn store_spelling(named: &str) -> Option<String> {
    let named = named.trim();
    BlobStore::parse_content_address(named)
        .or_else(|_| {
            BlobStore::parse_content_address(named.strip_prefix("sha256-").unwrap_or(named))
        })
        .or_else(|_| BlobStore::parse_content_address(&format!("sha256-{named}")))
        .ok()
        .map(|hex| format!("sha256-{}", hex.to_ascii_lowercase()))
}

/// Unix seconds of a recorded time, or `None` when it cannot be read. Rows
/// written by different paths spell time as RFC 3339 or as SQLite's
/// `YYYY-MM-DD HH:MM:SS` (UTC).
fn parse_time(at: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(at)
        .map(|t| t.timestamp())
        .ok()
        .or_else(|| {
            chrono::NaiveDateTime::parse_from_str(at, "%Y-%m-%d %H:%M:%S")
                .ok()
                .map(|t| t.and_utc().timestamp())
        })
}

/// Seconds since the NEWEST of a blob's arrivals, or `None` when any of them
/// cannot be read or lies in the future: an age that is not known is no age.
fn age_secs(arrived: &[(&'static str, String)], now_unix: i64) -> Option<u64> {
    let mut newest: Option<i64> = None;
    for (_, at) in arrived {
        let at = parse_time(at)?;
        newest = Some(newest.map_or(at, |n| n.max(at)));
    }
    u64::try_from(now_unix - newest?).ok()
}

/// What the tables say about one address, whether or not a file by that name
/// is on disk: a blob stored only as shards is still a blob this peer holds.
struct Standing {
    reason: HoldReason,
    /// The records that make it this peer's to let go: how it arrived, when.
    own: Vec<(&'static str, String)>,
}

fn standing(tables: &Tables, in_use: &HashSet<String>, hash: &str) -> Standing {
    let cid = hash
        .strip_prefix("sha256-")
        .and_then(|hex| BlobStore::hash_to_cid(hex).ok())
        .map(|cid| cid.to_string());
    let arrivals = tables.arrivals.get(hash).map(Vec::as_slice).unwrap_or(&[]);
    // This peer's own acts, and placements the placing peer has since
    // withdrawn: those bytes are nobody else's any more.
    let mut own: Vec<(&'static str, String)> = arrivals
        .iter()
        .filter(|a| a.via.is_own_act() || a.withdrawn_at.is_some())
        .map(|a| (a.via.label(), a.arrived_at.clone()))
        .collect();
    if let Some((via, at)) = tables.own_evidence.get(hash) {
        own.push((via.label(), at.clone()));
    }
    own.sort_by_key(|(_, at)| parse_time(at));
    let facts = BlobFacts {
        in_kept_release: tables.kept.contains(hash),
        served: tables.served.contains(hash)
            || cid.as_ref().is_some_and(|cid| tables.served.contains(cid)),
        pledged: tables.pledged.contains(hash),
        part_of_held: false,
        placed: arrivals.iter().any(Arrival::is_live_placement),
        arriving: arrivals.iter().any(|a| {
            a.via == ArrivalVia::Adoption
                && a.named_for.as_ref().is_some_and(|cid| in_use.contains(cid))
        }),
        own_arrival: !own.is_empty(),
    };
    Standing {
        reason: hold_reason(&facts),
        own,
    }
}

fn is_held(reason: HoldReason) -> bool {
    !matches!(reason, HoldReason::OwnUnnamed | HoldReason::Unrecorded)
}

/// The files on disk that make up one blob: its own file when it has one, and
/// the shards its manifest names.
fn members_of<'a>(
    blob: &str,
    on_disk: &'a BTreeMap<String, u64>,
    shards_of: &HashMap<&'a str, Vec<&'a str>>,
) -> Vec<&'a str> {
    let own_file = on_disk.get_key_value(blob).map(|(hash, _)| hash.as_str());
    own_file
        .into_iter()
        .chain(
            shards_of
                .get(blob)
                .into_iter()
                .flatten()
                .copied()
                .filter(|shard| on_disk.contains_key(*shard)),
        )
        .collect()
}

/// Run one holds pass.
pub async fn run_pass(
    sweeper: &RetentionSweeper,
    depth: usize,
    in_use: &HashSet<String>,
    limits: Limits,
) -> Result<HoldsReport, StorageError> {
    let now_unix = super::release_adoption::state::now_unix();
    let tables = {
        let in_use = in_use.clone();
        let self_cid = sweeper.self_cid().map(str::to_string);
        sweeper
            .db(move |conn| read_tables(conn, depth, &in_use, self_cid.as_deref()))
            .await?
    };

    let hashes = {
        let blobs = sweeper.blobs().clone();
        tokio::task::spawn_blocking(move || blobs.list_hashes())
            .await
            .map_err(|e| StorageError::Internal(format!("holds: listing blobs: {e}")))??
    };
    let mut on_disk: BTreeMap<String, u64> = BTreeMap::new();
    for hash in hashes {
        let bytes = sweeper.blobs().size(&hash).await.unwrap_or(0);
        on_disk.insert(hash, bytes);
    }

    // The shards each blob's manifest names, and the blobs each shard is a
    // part of. A blob over the single-shard size is on disk only as shards.
    let mut shards_of: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut owners: HashMap<&str, Vec<&str>> = HashMap::new();
    for (blob, shards) in &tables.manifests {
        for shard in shards {
            if shard != blob {
                shards_of
                    .entry(blob.as_str())
                    .or_default()
                    .push(shard.as_str());
                owners
                    .entry(shard.as_str())
                    .or_default()
                    .push(blob.as_str());
            }
        }
    }

    // What the tables say about every file on disk and about every blob with a
    // shard on disk.
    let mut standings: HashMap<String, Standing> = HashMap::new();
    for hash in on_disk.keys() {
        standings
            .entry(hash.clone())
            .or_insert_with(|| standing(&tables, in_use, hash));
        for owner in owners.get(hash.as_str()).into_iter().flatten() {
            standings
                .entry((*owner).to_string())
                .or_insert_with(|| standing(&tables, in_use, owner));
        }
    }

    // Each file's reason: its own when it is held in its own right; otherwise
    // it takes its standing from the blobs it is a shard of.
    let mut file_reason: BTreeMap<&str, HoldReason> = BTreeMap::new();
    for hash in on_disk.keys() {
        let own_reason = standings[hash].reason;
        let owner_reasons: Vec<HoldReason> = owners
            .get(hash.as_str())
            .into_iter()
            .flatten()
            .filter_map(|owner| standings.get(*owner).map(|s| s.reason))
            .collect();
        let reason = if is_held(own_reason) {
            own_reason
        } else if owner_reasons.iter().any(|r| is_held(*r)) {
            HoldReason::PartOf
        } else if own_reason == HoldReason::OwnUnnamed
            || owner_reasons.contains(&HoldReason::OwnUnnamed)
        {
            HoldReason::OwnUnnamed
        } else {
            HoldReason::Unrecorded
        };
        file_reason.insert(hash.as_str(), reason);
    }
    // Files held in their own right. Letting a blob go never takes one of
    // these with it, even when the blob's manifest names it.
    let held_in_own_right: HashSet<String> = on_disk
        .keys()
        .filter(|hash| is_held(standings[*hash].reason))
        .cloned()
        .collect();

    let mut report = HoldsReport {
        at_unix_secs: now_unix,
        min_age_secs: limits.min_age_secs,
        min_passes: limits.min_passes.max(2),
        ..HoldsReport::default()
    };

    // What may be let go is a whole blob: a file that is no other blob's
    // shard, or a blob on disk only as shards. A shard goes with its blob.
    let mut candidates: Vec<String> = standings
        .iter()
        .filter(|(hash, standing)| {
            standing.reason == HoldReason::OwnUnnamed
                && !owners.contains_key(hash.as_str())
                && !members_of(hash, &on_disk, &shards_of).is_empty()
        })
        .map(|(hash, _)| hash.clone())
        .collect();
    candidates.sort();

    // The close look, at a bounded slice of them.
    let start = if candidates.len() > MAX_UNNAMED_PER_PASS {
        sweeper
            .holds_cursor
            .fetch_add(MAX_UNNAMED_PER_PASS, Ordering::Relaxed)
            % candidates.len()
    } else {
        0
    };
    report.unexamined = candidates.len().saturating_sub(MAX_UNNAMED_PER_PASS);
    let examined: Vec<String> = candidates
        .iter()
        .cycle()
        .skip(start)
        .take(candidates.len().min(MAX_UNNAMED_PER_PASS))
        .cloned()
        .collect();

    let mut passes_now: HashMap<String, u64> = HashMap::new();
    // Blobs the close look found to be named after all, and why.
    let mut named_after_all: HashMap<String, HoldReason> = HashMap::new();
    for hash in &examined {
        // The authoritative read: the digest anywhere in any content row, in
        // any live pledge, or in another row that shows it, counts as named.
        let facts = {
            let hash = hash.clone();
            sweeper
                .db(move |conn| {
                    let sha256 = hash.strip_prefix("sha256-").unwrap_or(&hash).to_string();
                    let artifact = LedgerArtifact {
                        blob_cid: BlobStore::hash_to_cid(&sha256)
                            .map(|cid| cid.to_string())
                            .unwrap_or_default(),
                        sha256,
                        bytes: 0,
                        filename: String::new(),
                    };
                    Ok((
                        release_ledger::content_rows_naming(conn, &artifact)?
                            + release_ledger::other_rows_naming(conn, &hash)?,
                        release_ledger::live_custody_pledges_naming(conn, &hash)?,
                    ))
                })
                .await
        };
        let (rows, pledges) = match facts {
            Ok(facts) => facts,
            Err(e) => {
                // Not knowing whether a blob is named is not permission to
                // let it go.
                tracing::warn!(blob = %hash, error = %e, "holds: could not read whether a blob is named; kept");
                report.failures += 1;
                continue;
            }
        };
        if rows > 0 || pledges > 0 {
            named_after_all.insert(
                hash.clone(),
                if rows > 0 {
                    HoldReason::Served
                } else {
                    HoldReason::Pledged
                },
            );
            continue;
        }

        let watch = {
            let hash = hash.clone();
            sweeper
                .db(move |conn| blob_arrivals::note_unnamed(conn, &hash))
                .await
        };
        let passes = match watch {
            Ok(watch) => u64::try_from(watch.passes).unwrap_or(0),
            Err(e) => {
                tracing::warn!(blob = %hash, error = %e, "holds: could not count an unnamed pass; kept");
                report.failures += 1;
                continue;
            }
        };
        passes_now.insert(hash.clone(), passes);

        let seen_before = !sweeper.seen_unnamed.lock().unwrap().insert(hash.clone());
        let own = &standings[hash].own;
        let age = age_secs(own, now_unix);
        if !seen_before || !may_let_go(HoldReason::OwnUnnamed, age, passes, limits) {
            continue;
        }

        // From the last look to the delete, nothing may arrive: a put or a
        // fetch of these bytes either lands before, and the watch it clears
        // stops this, or lands after, and stores them again.
        let gate = blob_arrivals::arrival_gate().write().await;
        let stands = {
            let hash = hash.clone();
            let min_passes = limits.min_passes.max(2);
            sweeper
                .db(move |conn| blob_arrivals::watch_stands(conn, &hash, min_passes))
                .await
        };
        if !matches!(stands, Ok(true)) {
            drop(gate);
            continue;
        }
        let released = sweeper.release_hash(hash, &held_in_own_right).await;
        drop(gate);
        match released {
            Ok(bytes) => {
                tracing::info!(
                    blob = %hash,
                    bytes,
                    passes,
                    arrived_via = ?own.iter().map(|(via, _)| *via).collect::<Vec<_>>(),
                    "holds: a blob this peer brought here and nothing names was let go"
                );
                report.released.push(hash.clone());
                report.bytes_released += bytes;
                sweeper.seen_unnamed.lock().unwrap().remove(hash);
            }
            Err(e) => {
                tracing::warn!(blob = %hash, error = %e, "holds: could not let a blob go; trying again next pass");
                report.failures += 1;
            }
        }
    }

    // A blob that is named again, or gone, starts its count over.
    let still_unnamed: HashSet<&String> = candidates
        .iter()
        .filter(|hash| !named_after_all.contains_key(*hash) && !report.released.contains(*hash))
        .collect();
    let stale: Vec<String> = tables
        .watch
        .keys()
        .filter(|hash| !still_unnamed.contains(*hash))
        .cloned()
        .collect();
    if !stale.is_empty() {
        {
            let mut seen_unnamed = sweeper.seen_unnamed.lock().unwrap();
            for hash in &stale {
                seen_unnamed.remove(hash);
            }
        }
        if let Err(e) = sweeper
            .db(move |conn| blob_arrivals::clear_watch(conn, &stale))
            .await
        {
            tracing::warn!(error = %e, "holds: watch rows of named blobs not cleared");
            report.failures += 1;
        }
    }

    // The account, file by file. A blob the close look found named carries
    // that reason, and so do its shards.
    for reason in HoldReason::ALL {
        report.blobs.insert(reason.label(), Tally::default());
    }
    let let_go: HashSet<&str> = report
        .released
        .iter()
        .flat_map(|hash| members_of(hash, &on_disk, &shards_of))
        .filter(|hash| !held_in_own_right.contains(*hash))
        .collect();
    for (hash, bytes) in &on_disk {
        if let_go.contains(hash.as_str()) {
            continue;
        }
        let mut reason = file_reason[hash.as_str()];
        if reason == HoldReason::OwnUnnamed {
            if let Some(found) = named_after_all.get(hash) {
                reason = *found;
            } else if owners
                .get(hash.as_str())
                .into_iter()
                .flatten()
                .any(|owner| named_after_all.contains_key(*owner))
            {
                reason = HoldReason::PartOf;
            }
        }
        let tally = report.blobs.entry(reason.label()).or_default();
        tally.blobs += 1;
        tally.bytes += bytes;
    }
    let mut listed: Vec<&String> = still_unnamed.into_iter().collect();
    listed.sort();
    for hash in listed {
        let own = &standings[hash].own;
        report.own_unnamed.push(UnnamedBlob {
            blob_hash: hash.clone(),
            bytes: members_of(hash, &on_disk, &shards_of)
                .iter()
                .map(|file| on_disk[*file])
                .sum(),
            arrived_via: own.iter().map(|(via, _)| *via).collect(),
            arrived_at: own.last().map(|(_, at)| at.clone()),
            passes: passes_now.get(hash).copied().unwrap_or_else(|| {
                tables
                    .watch
                    .get(hash)
                    .map_or(0, |w| u64::try_from(w.passes).unwrap_or(0))
            }),
        });
    }
    report
        .own_unnamed
        .sort_by(|a, b| b.bytes.cmp(&a.bytes).then(a.blob_hash.cmp(&b.blob_hash)));
    report.own_unnamed.truncate(MAX_LISTED);
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIMITS: Limits = Limits {
        min_age_secs: 60,
        min_passes: 2,
    };

    #[test]
    fn a_blob_is_held_for_the_strongest_reason_that_applies() {
        let all = BlobFacts {
            in_kept_release: true,
            served: true,
            pledged: true,
            part_of_held: true,
            placed: true,
            arriving: true,
            own_arrival: true,
        };
        assert_eq!(hold_reason(&all), HoldReason::KeptRelease);
        assert_eq!(
            hold_reason(&BlobFacts {
                in_kept_release: false,
                ..all
            }),
            HoldReason::Served
        );
        assert_eq!(
            hold_reason(&BlobFacts {
                placed: true,
                own_arrival: true,
                ..BlobFacts::default()
            }),
            HoldReason::Placed
        );
        assert_eq!(
            hold_reason(&BlobFacts {
                own_arrival: true,
                ..BlobFacts::default()
            }),
            HoldReason::OwnUnnamed
        );
        assert_eq!(hold_reason(&BlobFacts::default()), HoldReason::Unrecorded);
    }

    #[test]
    fn only_a_blob_this_peer_brought_here_may_be_let_go() {
        for reason in HoldReason::ALL {
            assert_eq!(
                may_let_go(reason, Some(3600), 5, LIMITS),
                reason == HoldReason::OwnUnnamed,
                "{reason:?}"
            );
        }
    }

    #[test]
    fn an_unnamed_blob_stays_until_it_is_old_enough_and_seen_twice() {
        let own = HoldReason::OwnUnnamed;
        assert!(may_let_go(own, Some(60), 2, LIMITS));
        assert!(!may_let_go(own, Some(59), 2, LIMITS));
        assert!(!may_let_go(own, Some(3600), 1, LIMITS));
        // An age that cannot be read is no age.
        assert!(!may_let_go(own, None, 9, LIMITS));
        // One pass alone never lets a blob go, whatever is declared.
        let lax = Limits {
            min_age_secs: 0,
            min_passes: 0,
        };
        assert!(!may_let_go(own, Some(3600), 1, lax));
        assert!(may_let_go(own, Some(0), 2, lax));
    }

    // ── the pass against a real blob store and database ─────────────────────

    use std::sync::Arc;

    use crate::db::DbPool;

    const OLD: &str = "2026-01-01T00:00:00Z";
    /// No age and the fewest passes the rule allows: what is left is the rule.
    const SOON: Limits = Limits {
        min_age_secs: 0,
        min_passes: 2,
    };

    struct Peer {
        dir: tempfile::TempDir,
        pool: DbPool,
        blobs: Arc<BlobStore>,
        sweeper: RetentionSweeper,
    }

    async fn peer() -> Peer {
        let dir = tempfile::tempdir().unwrap();
        let pool = crate::db::init_pool_from_dir(dir.path()).unwrap();
        let blobs = Arc::new(BlobStore::new(dir.path().join("blobs")).await.unwrap());
        let sweeper = RetentionSweeper::new(
            pool.clone(),
            blobs.clone(),
            dir.path().join("release-adoption"),
        )
        .with_self_cid(Some("12D3KooWSelf".to_string()));
        Peer {
            dir,
            pool,
            blobs,
            sweeper,
        }
    }

    impl Peer {
        /// The same peer after a restart: the same stores, a new process.
        fn restarted(&self) -> RetentionSweeper {
            RetentionSweeper::new(
                self.pool.clone(),
                self.blobs.clone(),
                self.dir.path().join("release-adoption"),
            )
            .with_self_cid(Some("12D3KooWSelf".to_string()))
        }

        /// Bytes in the blob store with nothing to say how they got there.
        async fn bare(&self, bytes: &[u8]) -> String {
            self.blobs.store(bytes).await.unwrap().hash
        }

        /// Bytes that arrived `via`, long enough ago to be past any age.
        async fn arrived(&self, bytes: &[u8], via: ArrivalVia, placed_by: Option<&str>) -> String {
            let hash = self.bare(bytes).await;
            let mut conn = self.pool.get().unwrap();
            blob_arrivals::record_arrival(&mut conn, &hash, via, placed_by, None).unwrap();
            self.arrived_at(&hash, OLD);
            hash
        }

        fn arrived_at(&self, hash: &str, at: &str) {
            let mut conn = self.pool.get().unwrap();
            diesel::sql_query("UPDATE blob_arrivals SET arrived_at = ? WHERE blob_hash = ?")
                .bind::<diesel::sql_types::Text, _>(at)
                .bind::<diesel::sql_types::Text, _>(hash)
                .execute(&mut conn)
                .unwrap();
        }

        /// A content row naming `blob_hash` in its own column, or only inside
        /// its metadata.
        fn content_row(&self, id: &str, blob_hash: Option<&str>, metadata_json: Option<String>) {
            let mut conn = self.pool.get().unwrap();
            crate::db::content_diesel::create_content(
                &mut conn,
                &crate::db::AppContext::default_lamad(),
                crate::db::content_diesel::CreateContentInput {
                    id: id.to_string(),
                    title: id.to_string(),
                    description: None,
                    content_type: "app".to_string(),
                    content_format: "html5-app".to_string(),
                    blob_hash: blob_hash.map(str::to_string),
                    blob_cid: None,
                    content_size_bytes: None,
                    metadata_json,
                    reach: "commons".to_string(),
                    created_by: None,
                    tags: Vec::new(),
                    content_body: None,
                    dht_anchor_hash: None,
                },
            )
            .unwrap();
        }

        fn drop_content_row(&self, id: &str) {
            let mut conn = self.pool.get().unwrap();
            diesel::sql_query("DELETE FROM content WHERE id = ?")
                .bind::<diesel::sql_types::Text, _>(id)
                .execute(&mut conn)
                .unwrap();
        }

        async fn pass(&self, limits: Limits) -> HoldsReport {
            run_pass(&self.sweeper, 10, &HashSet::new(), limits)
                .await
                .unwrap()
        }

        async fn holds(&self, hash: &str) -> bool {
            self.blobs.exists(hash).await
        }
    }

    fn tally(report: &HoldsReport, reason: HoldReason) -> usize {
        report.blobs[reason.label()].blobs
    }

    #[tokio::test]
    async fn a_blob_nothing_records_is_reported_and_never_let_go() {
        let peer = peer().await;
        let hash = peer.bare(b"bytes nobody recorded").await;
        for _ in 0..4 {
            let report = peer.pass(SOON).await;
            assert_eq!(tally(&report, HoldReason::Unrecorded), 1);
            assert!(report.released.is_empty());
            assert!(report.own_unnamed.is_empty());
        }
        assert!(peer.holds(&hash).await);
    }

    #[tokio::test]
    async fn a_blob_this_peer_put_and_nothing_names_is_let_go_on_its_second_pass_past_the_age() {
        let peer = peer().await;
        let hash = peer
            .arrived(b"an earlier build", ArrivalVia::SelfPut, None)
            .await;

        let first = peer.pass(SOON).await;
        assert_eq!(tally(&first, HoldReason::OwnUnnamed), 1);
        assert_eq!(first.own_unnamed[0].blob_hash, hash);
        assert_eq!(first.own_unnamed[0].arrived_via, vec!["self-put"]);
        assert_eq!(first.own_unnamed[0].passes, 1);
        assert!(first.released.is_empty(), "one pass alone lets nothing go");
        assert!(peer.holds(&hash).await);

        let second = peer.pass(SOON).await;
        assert_eq!(second.released, vec![hash.clone()]);
        assert_eq!(second.bytes_released, b"an earlier build".len() as u64);
        assert_eq!(tally(&second, HoldReason::OwnUnnamed), 0);
        assert!(!peer.holds(&hash).await);

        // Its records went with it: nothing is left naming bytes that are gone.
        let mut conn = peer.pool.get().unwrap();
        assert!(blob_arrivals::arrivals_by_hash(&mut conn)
            .unwrap()
            .is_empty());
        assert!(blob_arrivals::watched(&mut conn).unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_blob_younger_than_the_age_stays_however_many_passes_see_it() {
        let peer = peer().await;
        let hash = peer.bare(b"just arrived").await;
        let mut conn = peer.pool.get().unwrap();
        blob_arrivals::record_arrival(&mut conn, &hash, ArrivalVia::SelfFetch, None, None).unwrap();
        drop(conn);
        let day = Limits {
            min_age_secs: 86_400,
            min_passes: 2,
        };
        for _ in 0..4 {
            assert!(peer.pass(day).await.released.is_empty());
        }
        assert!(peer.holds(&hash).await);
    }

    #[tokio::test]
    async fn a_blob_named_again_between_passes_starts_its_count_over() {
        let peer = peer().await;
        let hash = peer
            .arrived(b"a build that is served again", ArrivalVia::SelfPut, None)
            .await;
        assert_eq!(peer.pass(SOON).await.own_unnamed[0].passes, 1);

        peer.content_row("app-one", Some(&hash), None);
        let named = peer.pass(SOON).await;
        assert_eq!(tally(&named, HoldReason::Served), 1);
        assert!(named.released.is_empty());

        peer.drop_content_row("app-one");
        let unnamed_again = peer.pass(SOON).await;
        assert_eq!(unnamed_again.own_unnamed[0].passes, 1);
        assert!(unnamed_again.released.is_empty());
        assert!(peer.holds(&hash).await);

        assert_eq!(peer.pass(SOON).await.released, vec![hash]);
    }

    #[tokio::test]
    async fn a_blob_named_only_inside_a_content_rows_metadata_is_not_let_go() {
        let peer = peer().await;
        let hash = peer
            .arrived(
                b"named by a manifest in metadata",
                ArrivalVia::SelfFetch,
                None,
            )
            .await;
        let hex = hash.trim_start_matches("sha256-");
        peer.content_row(
            "channel-row",
            None,
            Some(format!(r#"{{"artifacts":[{{"sha256":"{hex}"}}]}}"#)),
        );
        for _ in 0..3 {
            let report = peer.pass(SOON).await;
            assert!(report.released.is_empty());
            assert_eq!(tally(&report, HoldReason::Served), 1);
        }
        assert!(peer.holds(&hash).await);
    }

    #[tokio::test]
    async fn a_pushed_shard_is_held_until_the_peer_that_placed_it_withdraws() {
        let peer = peer().await;
        let hash = peer
            .arrived(
                b"another peer's shard",
                ArrivalVia::Placed,
                Some("uhCAkPlacer"),
            )
            .await;
        for _ in 0..3 {
            let report = peer.pass(SOON).await;
            assert_eq!(tally(&report, HoldReason::Placed), 1);
            assert!(report.released.is_empty());
        }
        assert!(peer.holds(&hash).await);

        let mut conn = peer.pool.get().unwrap();
        assert!(blob_arrivals::withdraw_placement(&mut conn, &hash, "uhCAkPlacer").unwrap());
        drop(conn);
        let after = peer.pass(SOON).await;
        assert_eq!(after.own_unnamed[0].arrived_via, vec!["placed"]);
        assert!(after.released.is_empty());
        assert_eq!(peer.pass(SOON).await.released, vec![hash]);
    }

    #[tokio::test]
    async fn a_blob_this_peer_also_put_stays_while_another_peers_placement_stands() {
        let peer = peer().await;
        let hash = peer
            .arrived(b"put here and placed here", ArrivalVia::SelfPut, None)
            .await;
        let mut conn = peer.pool.get().unwrap();
        blob_arrivals::record_arrival(
            &mut conn,
            &hash,
            ArrivalVia::Placed,
            Some("uhCAkPlacer"),
            None,
        )
        .unwrap();
        drop(conn);
        for _ in 0..3 {
            assert!(peer.pass(SOON).await.released.is_empty());
        }
        assert!(peer.holds(&hash).await);
    }

    #[tokio::test]
    async fn a_blob_under_a_live_pledge_is_held_however_the_pledge_spells_it() {
        let peer = peer().await;
        let mut hashes = Vec::new();
        for (i, bytes) in [
            &b"pledged, plain"[..],
            b"pledged, in a list",
            b"pledged, bare hex",
            b"pledged, as a cid",
            b"pledged, cid behind a prefix",
        ]
        .into_iter()
        .enumerate()
        {
            let hash = peer.arrived(bytes, ArrivalVia::SelfFetch, None).await;
            let hex = hash.trim_start_matches("sha256-").to_string();
            let cid = BlobStore::hash_to_cid(&hex).unwrap().to_string();
            let spelled = match i {
                0 => hash.clone(),
                1 => format!("[\"{hash}\"]"),
                2 => hex.to_ascii_uppercase(),
                3 => cid,
                _ => format!("sha256-{cid}"),
            };
            let mut conn = peer.pool.get().unwrap();
            diesel::sql_query(
                "INSERT INTO rea_commitments (id, h_app_id, action, provider, receiver, \
                 resource_classified_as, state) VALUES (?, 'lamad', 'custody-blob', 'me', \
                 'me', ?, 'active')",
            )
            .bind::<diesel::sql_types::Text, _>(format!("pledge-{i}"))
            .bind::<diesel::sql_types::Text, _>(spelled)
            .execute(&mut conn)
            .unwrap();
            hashes.push(hash);
        }
        for _ in 0..3 {
            let report = peer.pass(SOON).await;
            assert!(report.released.is_empty(), "{:?}", report.released);
            assert_eq!(tally(&report, HoldReason::Pledged), hashes.len());
            assert_eq!(tally(&report, HoldReason::OwnUnnamed), 0);
        }
        for hash in &hashes {
            assert!(
                peer.holds(hash).await,
                "{hash} was let go under a live pledge"
            );
        }

        // Once the pledges are withdrawn the blobs are this peer's own again.
        let mut conn = peer.pool.get().unwrap();
        diesel::sql_query("UPDATE rea_commitments SET state = 'superseded'")
            .execute(&mut conn)
            .unwrap();
        drop(conn);
        peer.pass(SOON).await;
        let mut released = peer.pass(SOON).await.released;
        released.sort();
        hashes.sort();
        assert_eq!(released, hashes);
    }

    #[tokio::test]
    async fn a_blob_put_again_while_it_waits_to_be_let_go_starts_over() {
        let peer = peer().await;
        let hash = peer
            .arrived(b"put, forgotten, and put again", ArrivalVia::SelfPut, None)
            .await;
        assert_eq!(peer.pass(SOON).await.own_unnamed[0].passes, 1);

        // Someone hands the same bytes over again: a fresh arrival. Without
        // this the next pass would be the blob's second and would let it go.
        let mut conn = peer.pool.get().unwrap();
        blob_arrivals::record_arrival(&mut conn, &hash, ArrivalVia::SelfPut, None, None).unwrap();
        drop(conn);

        let after = peer.pass(SOON).await;
        assert!(
            after.released.is_empty(),
            "a blob just put again was let go"
        );
        assert_eq!(after.own_unnamed[0].passes, 1, "its count must start over");
        assert!(peer.holds(&hash).await);

        // And its age is counted from the new arrival, not the first.
        let day = Limits {
            min_age_secs: 86_400,
            min_passes: 2,
        };
        for _ in 0..3 {
            assert!(peer.pass(day).await.released.is_empty());
        }
        assert!(peer.holds(&hash).await);
    }

    /// A blob over the single-shard size: on disk only as its shards, named
    /// by a manifest, with the arrival recorded under the blob's own hash.
    async fn sharded(peer: &Peer, blob: &str, shards: &[&[u8]]) -> Vec<String> {
        let mut hashes = Vec::new();
        for shard in shards {
            hashes.push(peer.bare(shard).await);
        }
        let mut conn = peer.pool.get().unwrap();
        diesel::sql_query(
            "INSERT INTO shard_manifests (content_id, h_app_id, blob_hash, blob_cid, encoding, \
             data_shard_count, parity_shard_count, shard_hashes_json, total_size_bytes, \
             shard_size_bytes, mime_type, reach, created_at) VALUES (?, 'lamad', ?, NULL, \
             'rs-4-7', 2, 0, ?, 1, 1, 'application/zip', 'commons', '2026-01-01T00:00:00Z')",
        )
        .bind::<diesel::sql_types::Text, _>(format!("row-for-{blob}"))
        .bind::<diesel::sql_types::Text, _>(blob)
        .bind::<diesel::sql_types::Text, _>(serde_json::to_string(&hashes).unwrap())
        .execute(&mut conn)
        .unwrap();
        blob_arrivals::record_arrival(&mut conn, blob, ArrivalVia::SelfPut, None, None).unwrap();
        drop(conn);
        peer.arrived_at(blob, OLD);
        hashes
    }

    #[tokio::test]
    async fn a_blob_held_only_as_shards_is_let_go_whole() {
        let peer = peer().await;
        let blob = format!("sha256-{}", "a".repeat(64));
        let shards = sharded(&peer, &blob, &[b"shard one", b"shard two"]).await;

        let first = peer.pass(SOON).await;
        assert_eq!(tally(&first, HoldReason::OwnUnnamed), 2);
        assert_eq!(tally(&first, HoldReason::Unrecorded), 0);
        assert_eq!(first.own_unnamed.len(), 1);
        assert_eq!(first.own_unnamed[0].blob_hash, blob);
        assert_eq!(
            first.own_unnamed[0].bytes,
            (b"shard one".len() + b"shard two".len()) as u64
        );

        assert_eq!(peer.pass(SOON).await.released, vec![blob]);
        for shard in &shards {
            assert!(!peer.holds(shard).await);
        }
    }

    #[tokio::test]
    async fn a_shard_another_peer_placed_stays_when_the_blob_it_is_part_of_goes() {
        let peer = peer().await;
        let blob = format!("sha256-{}", "b".repeat(64));
        let shards = sharded(
            &peer,
            &blob,
            &[b"this peer's shard", b"a shard a peer placed"],
        )
        .await;
        let mut conn = peer.pool.get().unwrap();
        blob_arrivals::record_arrival(
            &mut conn,
            &shards[1],
            ArrivalVia::Placed,
            Some("uhCAkPlacer"),
            None,
        )
        .unwrap();
        drop(conn);

        peer.pass(SOON).await;
        let second = peer.pass(SOON).await;
        assert_eq!(second.released, vec![blob]);
        assert!(!peer.holds(&shards[0]).await);
        assert!(
            peer.holds(&shards[1]).await,
            "bytes another peer placed are that peer's to withdraw"
        );
        let after = peer.pass(SOON).await;
        assert_eq!(tally(&after, HoldReason::Placed), 1);
        let mut conn = peer.pool.get().unwrap();
        assert!(
            blob_arrivals::arrivals_by_hash(&mut conn).unwrap()[&shards[1]][0].is_live_placement()
        );
    }

    #[tokio::test]
    async fn a_shard_of_a_held_blob_is_held_as_part_of_it() {
        let peer = peer().await;
        let blob = peer
            .arrived(b"a blob this peer serves", ArrivalVia::SelfPut, None)
            .await;
        let shard = peer.bare(b"one shard of it").await;
        peer.content_row("app-two", Some(&blob), None);
        let mut conn = peer.pool.get().unwrap();
        diesel::sql_query(
            "INSERT INTO shard_manifests (content_id, h_app_id, blob_hash, blob_cid, encoding, \
             data_shard_count, parity_shard_count, shard_hashes_json, total_size_bytes, \
             shard_size_bytes, mime_type, reach, created_at) VALUES ('app-two', 'lamad', ?, \
             NULL, 'rs', 1, 0, ?, 1, 1, 'application/zip', 'commons', '2026-01-01T00:00:00Z')",
        )
        .bind::<diesel::sql_types::Text, _>(&blob)
        .bind::<diesel::sql_types::Text, _>(format!("[\"{shard}\"]"))
        .execute(&mut conn)
        .unwrap();
        drop(conn);

        let report = peer.pass(SOON).await;
        assert_eq!(tally(&report, HoldReason::Served), 1);
        assert_eq!(tally(&report, HoldReason::PartOf), 1);
        assert_eq!(tally(&report, HoldReason::Unrecorded), 0);
    }

    #[tokio::test]
    async fn letting_a_blob_go_leaves_other_peers_location_rows() {
        let peer = peer().await;
        let hash = peer
            .arrived(b"a blob with holders elsewhere", ArrivalVia::SelfPut, None)
            .await;
        let mut conn = peer.pool.get().unwrap();
        for (holder, status) in [
            (
                "uhCAkSelf",
                crate::services::self_stewardship::SELF_HELD_STATUS,
            ),
            ("uhCAkOther", "announced"),
        ] {
            crate::db::shard_locations::upsert_location(
                &mut conn,
                &crate::db::models::NewShardLocation {
                    shard_hash: &hash,
                    peer_id: holder,
                    h_app_id: "lamad",
                    status,
                },
            )
            .unwrap();
        }
        drop(conn);

        peer.pass(SOON).await;
        assert_eq!(peer.pass(SOON).await.released, vec![hash.clone()]);

        #[derive(QueryableByName)]
        struct Holder {
            #[diesel(sql_type = diesel::sql_types::Text)]
            peer_id: String,
        }
        let mut conn = peer.pool.get().unwrap();
        let left: Vec<Holder> =
            diesel::sql_query("SELECT peer_id FROM shard_locations WHERE shard_hash = ?")
                .bind::<diesel::sql_types::Text, _>(&hash)
                .load(&mut conn)
                .unwrap();
        let left: Vec<&str> = left.iter().map(|h| h.peer_id.as_str()).collect();
        assert_eq!(left, vec!["uhCAkOther"]);
    }

    #[tokio::test]
    async fn a_restart_never_lets_a_blob_go_on_its_first_look() {
        let peer = peer().await;
        let hash = peer
            .arrived(b"watched across a restart", ArrivalVia::SelfPut, None)
            .await;
        peer.pass(SOON).await;

        // The watch row survives; the new process has not itself seen the
        // blob unnamed, so its first pass counts and its second lets go.
        let restarted = peer.restarted();
        let first = run_pass(&restarted, 10, &HashSet::new(), SOON)
            .await
            .unwrap();
        assert_eq!(first.own_unnamed[0].passes, 2);
        assert!(first.released.is_empty());
        assert!(peer.holds(&hash).await);
        let second = run_pass(&restarted, 10, &HashSet::new(), SOON)
            .await
            .unwrap();
        assert_eq!(second.released, vec![hash]);
    }

    #[cfg(feature = "p2p")]
    #[tokio::test]
    async fn a_fetch_this_node_made_before_arrivals_were_recorded_is_its_own_act() {
        let peer = peer().await;
        let mine = b"fetched by this node".to_vec();
        let theirs = b"fetched by another node".to_vec();
        let mut conn = peer.pool.get().unwrap();
        for (bytes, receiver) in [(&mine, "12D3KooWSelf"), (&theirs, "12D3KooWOther")] {
            crate::p2p::blob_fetch::finalize_fetch_success(
                &mut conn,
                &BlobStore::compute_hash(bytes),
                "12D3KooWSource",
                bytes,
                receiver,
                &peer.blobs,
            )
            .await
            .unwrap();
        }
        // As it was before arrivals were recorded: only the fetch events.
        diesel::sql_query("DELETE FROM blob_arrivals")
            .execute(&mut conn)
            .unwrap();
        diesel::sql_query("UPDATE economic_events SET has_point_in_time = ?")
            .bind::<diesel::sql_types::Text, _>(OLD)
            .execute(&mut conn)
            .unwrap();
        drop(conn);

        let report = peer.pass(SOON).await;
        assert_eq!(tally(&report, HoldReason::OwnUnnamed), 1);
        assert_eq!(
            report.own_unnamed[0].blob_hash,
            BlobStore::compute_hash(&mine)
        );
        assert_eq!(report.own_unnamed[0].arrived_via, vec!["self-fetch"]);
        // The other node's fetch event says nothing about how OUR copy got
        // here.
        assert_eq!(tally(&report, HoldReason::Unrecorded), 1);
    }

    #[test]
    fn an_arrival_time_in_the_future_or_unreadable_gives_no_age() {
        let at = parse_time("2026-10-06T00:00:00Z").unwrap();
        let one = |when: &str| vec![("self-put", when.to_string())];
        assert_eq!(age_secs(&one("2026-10-06T00:00:00Z"), at + 60), Some(60));
        assert_eq!(age_secs(&one("2026-10-06 00:00:00"), at + 60), Some(60));
        assert_eq!(age_secs(&one("2026-10-06T00:02:00Z"), at + 60), None);
        assert_eq!(age_secs(&one("yesterday"), at + 60), None);
        // The newest arrival is the one that counts, and one unreadable time
        // among several leaves the age unknown.
        let two = vec![
            ("self-put", "2026-10-01T00:00:00Z".to_string()),
            ("self-fetch", "2026-10-06T00:00:00Z".to_string()),
        ];
        assert_eq!(age_secs(&two, at + 60), Some(60));
        let mixed = vec![
            ("self-put", "2026-10-01T00:00:00Z".to_string()),
            ("self-fetch", "some time ago".to_string()),
        ];
        assert_eq!(age_secs(&mixed, at + 60), None);
        assert_eq!(age_secs(&[], at + 60), None);
    }
}
