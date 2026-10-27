//! The fold: another device's crossing events, read from its tier and appended to this device's
//! own flow store with provenance `via:<device>` (spec §3.3).
//!
//! Fetching asks the device's tier for the diff between the root last folded from it and the new
//! one, and lists the new root's `channel/events` when the diff cannot be answered (the device no
//! longer holds the old root, or this is the first fold). Every leaf is checked three ways before
//! it is folded: the tier client checks its bytes hash to its CID and that it is at the path
//! asked; here its name must be that CID, and its bytes must be a FlowEvent whose canonical
//! encoding they are. Appending is idempotent by event CID: a replay or a duplicate announce
//! appends nothing.

use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use cid::Cid;
use elohim_epr_rea::model::canonical_bytes;
use elohim_epr_rea::{atom_cid, FlowEvent, FlowRecord, FlowStore, SidecarFlowStore};
use eprfs_core::{BlobCid, TreeEntryKind, TreeLimits, TreePath, View};
use iroh::endpoint::{Connection, VarInt};
use iroh::{Endpoint, NodeAddr};

use crate::allow::{via_device, CrossingAllowList, VIA_SLOT};
use crate::channel::{CHANNEL_EVENTS, CHANNEL_NACKS};
use crate::error::PeerError;
use crate::gate::SharedAdmission;
use crate::head_table::{NackLeaf, MAX_NACK_CIDS};
use crate::lease_view::{LeaseEvent, LeaseKind};
use crate::tier::{tier_request, tier_request_capped, TierAnswer, TierRequest, TIER_ALPN};

/// The most events one fold reads. A larger change is folded across successive heads: the fold
/// reports itself incomplete and the head is not recorded, so the next one resumes from the same
/// base and skips what was already folded.
pub const MAX_FOLD_EVENTS: usize = 1024;

/// The largest event one fold accepts, in bytes. A berth record is well under a kilobyte; a
/// larger leaf is refused before its bytes are read past this, and leaves the fold incomplete.
pub const MAX_EVENT_BYTES: usize = 64 * 1024;

/// The most event bytes one fold reads before it stops (and reports itself incomplete), so an
/// admitted device that misbehaves costs this device bounded memory per fold.
pub const MAX_FOLD_BYTES: usize = 8 * 1024 * 1024;

/// What one read answer may carry beyond the event's own bytes: its path, CIDs and framing.
const READ_ANSWER_OVERHEAD: usize = 4096;

/// How much one fold reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FoldLimits {
    /// New events read per fold ([`MAX_FOLD_EVENTS`]).
    pub max_events: usize,
    /// The largest event accepted ([`MAX_EVENT_BYTES`]); a larger one leaves the fold incomplete.
    pub max_event_bytes: usize,
    /// Event bytes read per fold ([`MAX_FOLD_BYTES`]); the event that crosses it is the last.
    pub max_fold_bytes: usize,
}

impl Default for FoldLimits {
    fn default() -> FoldLimits {
        FoldLimits {
            max_events: MAX_FOLD_EVENTS,
            max_event_bytes: MAX_EVENT_BYTES,
            max_fold_bytes: MAX_FOLD_BYTES,
        }
    }
}

/// One crossing event as another device's channel tree holds it.
#[derive(Debug, Clone, PartialEq)]
pub struct ChannelEvent {
    /// The event's own atom CID: its name in the tree (its leaf is the same digest, raw codec).
    pub cid: Cid,
    pub event: FlowEvent,
}

/// One channel leaf this device could address but would not fold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Quarantined {
    pub cid: Cid,
    pub path: String,
    pub reason: String,
    bytes: Option<Vec<u8>>,
}

/// What [`fetch_channel`] read.
#[derive(Debug, Clone, Default)]
pub struct Fetched {
    pub events: Vec<ChannelEvent>,
    /// False when the fold stopped short of every new event: past the event count, at an event
    /// over the per-event cap, or past the byte budget ([`FoldLimits`]).
    pub complete: bool,
    /// Events refused for exceeding the per-event cap (at most one: the fold stops there).
    pub oversized: u64,
    /// Whether the fold stopped at the byte budget.
    pub over_budget: bool,
    /// Leaves skipped without stopping the rest of the fold.
    pub quarantined: Vec<Quarantined>,
    /// The peer's nack leaf for this device, from the root being folded.
    pub peer_nack: NackLeaf,
}

/// What [`fold_events`] did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FoldOutcome {
    /// Events appended to this device's flow store.
    pub appended: u64,
    /// Events already folded (the same event CID via the same device).
    pub already: u64,
    /// Events this device's own allow-list does not admit; never appended.
    pub refused: u64,
    /// Refused event CIDs, for this device's nack leaf for their author.
    pub refused_cids: Vec<Cid>,
}

/// Read the events of the channel tree at `root` on an open tier connection that `known` does not
/// already hold (the original CIDs [`folded_from`] returns for that device). `since` is the root
/// last folded from that device, the diff's base.
///
/// # Errors
///
/// The [`tier_request`] errors; [`PeerError::FoldUnanswered`] when the tier does not answer for
/// `root`; [`PeerError::FoldIntegrity`] when the tree does not hold events at their own CIDs.
pub async fn fetch_channel(
    connection: &Connection,
    since: Option<&BlobCid>,
    root: &BlobCid,
    known: &HashSet<Cid>,
    tree: &TreeLimits,
    limits: &FoldLimits,
) -> Result<Fetched, PeerError> {
    fetch_channel_for(connection, since, root, known, tree, limits, None).await
}

async fn fetch_channel_for(
    connection: &Connection,
    since: Option<&BlobCid>,
    root: &BlobCid,
    known: &HashSet<Cid>,
    tree: &TreeLimits,
    limits: &FoldLimits,
    reader: Option<&str>,
) -> Result<Fetched, PeerError> {
    let events_dir = TreePath::from_slash_bytes(CHANNEL_EVENTS.as_bytes())
        .map_err(|e| integrity(e.to_string()))?;
    let offered = match since {
        Some(old) if old == root => Vec::new(),
        Some(old) => match diff(connection, old, root, &events_dir, tree).await? {
            Some(added) => added,
            None => list(connection, root, &events_dir, tree).await?,
        },
        None => list(connection, root, &events_dir, tree).await?,
    };
    let mut fresh = Vec::new();
    for (name, leaf) in offered {
        let cid = Cid::try_from(name.as_str())
            .map_err(|e| integrity(format!("entry `{name}` is not a CID: {e}")))?;
        // The leaf holds the event's bytes under the raw codec: one digest, two codecs.
        if leaf.as_cid().hash() != cid.hash() {
            return Err(integrity(format!("entry {cid} holds leaf {leaf}")));
        }
        if !known.contains(&cid) {
            fresh.push((cid, leaf));
        }
    }
    let mut fetched = Fetched {
        complete: fresh.len() <= limits.max_events,
        ..Fetched::default()
    };
    fresh.truncate(limits.max_events);
    // The answer frame is capped too, so an oversized leaf is refused before its bytes are read.
    let answer_cap = u32::try_from(limits.max_event_bytes.saturating_add(READ_ANSWER_OVERHEAD))
        .unwrap_or(u32::MAX);
    let mut read_bytes = 0usize;
    for (cid, leaf) in fresh {
        if read_bytes >= limits.max_fold_bytes {
            fetched.over_budget = true;
            fetched.complete = false;
            break;
        }
        let path = events_dir.join(cid.to_string().as_bytes());
        let request = TierRequest::Read {
            root: root.clone(),
            path: path.clone(),
        };
        let read = match tier_request_capped(connection, &request, tree, answer_cap).await {
            Ok(TierAnswer::Read(read)) if read.bytes.len() <= limits.max_event_bytes => read,
            Ok(TierAnswer::Read(_)) | Err(PeerError::TierFrameTooLarge { .. }) => {
                fetched.oversized += 1;
                fetched.quarantined.push(Quarantined {
                    cid,
                    path: path.to_string(),
                    reason: "unread-oversize".into(),
                    bytes: None,
                });
                continue;
            }
            Ok(other) => return Err(unanswered(format!("event {cid}"), &other)),
            Err(e) => return Err(e),
        };
        if read.cid != leaf {
            return Err(integrity(format!("event {cid} was served as {}", read.cid)));
        }
        if BlobCid::compute_raw(&read.bytes) != read.cid {
            return Err(integrity(format!("event {cid} bytes fail {}", read.cid)));
        }
        read_bytes = read_bytes.saturating_add(read.bytes.len());
        match decode_event(&cid, &read.bytes) {
            Ok(event) => fetched.events.push(ChannelEvent { cid, event }),
            Err(reason) => fetched.quarantined.push(Quarantined {
                cid,
                path: path.to_string(),
                reason,
                bytes: Some(read.bytes.to_vec()),
            }),
        }
    }
    if let Some(reader) = reader {
        fetched.peer_nack = read_nack(connection, root, reader, tree).await?;
    }
    Ok(fetched)
}

async fn read_nack(
    connection: &Connection,
    root: &BlobCid,
    reader: &str,
    tree: &TreeLimits,
) -> Result<NackLeaf, PeerError> {
    // A peer that has refused nothing of ours writes no leaf, and the tier answers a missing path
    // only as a failure string. Read the leaf only where the listings show it, so absence means
    // "no nacks" and every real failure still aborts the fold.
    if !lists_child(connection, root, "channel", "nacks", tree).await?
        || !lists_child(connection, root, CHANNEL_NACKS, reader, tree).await?
    {
        return Ok(NackLeaf::default());
    }
    let path = TreePath::from_slash_bytes(format!("{CHANNEL_NACKS}/{reader}").as_bytes())
        .map_err(|e| integrity(e.to_string()))?;
    let request = TierRequest::Read {
        root: root.clone(),
        path,
    };
    match tier_request(connection, &request, tree).await? {
        TierAnswer::Read(read) => {
            if BlobCid::compute_raw(&read.bytes) != read.cid {
                return Err(integrity("nack bytes fail their CID".into()));
            }
            let nack: NackLeaf = serde_json::from_slice(&read.bytes)
                .map_err(|e| integrity(format!("nack leaf is not valid: {e}")))?;
            let listed = nack.refused.len().saturating_add(nack.quarantined.len());
            if listed > MAX_NACK_CIDS
                || nack.refused_total < u64::try_from(listed).unwrap_or(u64::MAX)
            {
                return Err(integrity("nack leaf exceeds its declared bounds".into()));
            }
            Ok(nack)
        }
        TierAnswer::Unanswered { .. } => Ok(NackLeaf::default()),
        other => Err(unanswered(format!("nack for {reader}"), &other)),
    }
}

/// Whether the directory at `dir` under `root` lists an entry named `name`.
async fn lists_child(
    connection: &Connection,
    root: &BlobCid,
    dir: &str,
    name: &str,
    tree: &TreeLimits,
) -> Result<bool, PeerError> {
    let path = TreePath::from_slash_bytes(dir.as_bytes()).map_err(|e| integrity(e.to_string()))?;
    let request = TierRequest::List {
        root: root.clone(),
        path,
    };
    match tier_request(connection, &request, tree).await? {
        TierAnswer::List(listing) => Ok(listing
            .entries
            .iter()
            .any(|entry| entry.name.as_slice() == name.as_bytes())),
        other => Err(unanswered(format!("listing {dir} under {root}"), &other)),
    }
}

/// The `(name, leaf)` pairs the diff from `old` to `new` adds under `channel/events`, or `None`
/// when the diff is not answered or reports anything but added files there (a whole directory
/// added, a kind changed), which a listing answers instead.
async fn diff(
    connection: &Connection,
    old: &BlobCid,
    new: &BlobCid,
    events_dir: &TreePath,
    tree: &TreeLimits,
) -> Result<Option<Vec<(String, BlobCid)>>, PeerError> {
    let request = TierRequest::Diff {
        old: old.clone(),
        new: new.clone(),
        view: View::from_tree_paths([events_dir.clone()]),
    };
    let TierAnswer::Diff(changes) = tier_request(connection, &request, tree).await? else {
        return Ok(None);
    };
    let mut added = Vec::new();
    for change in changes {
        let parts = change.path.components();
        let under_events = parts.len() == events_dir.components().len() + 1
            && parts.starts_with(events_dir.components());
        match (&change.old, &change.new) {
            // An event that left the tree (a narrower allow-list) is not unfolded.
            (Some(TreeEntryKind::File { .. }), None) if under_events => {}
            (None, Some(TreeEntryKind::File { blob, .. })) if under_events => {
                let Some(name) = parts.last().and_then(|n| String::from_utf8(n.clone()).ok())
                else {
                    return Ok(None);
                };
                added.push((name, blob.as_blob_cid().clone()));
            }
            _ => return Ok(None),
        }
    }
    Ok(Some(added))
}

/// Every `(name, leaf)` under `channel/events` at `root`.
async fn list(
    connection: &Connection,
    root: &BlobCid,
    events_dir: &TreePath,
    tree: &TreeLimits,
) -> Result<Vec<(String, BlobCid)>, PeerError> {
    let request = TierRequest::List {
        root: root.clone(),
        path: events_dir.clone(),
    };
    let listing = match tier_request(connection, &request, tree).await? {
        TierAnswer::List(listing) => listing,
        other => return Err(unanswered(format!("root {root}"), &other)),
    };
    listing
        .entries
        .into_iter()
        .map(|entry| {
            let name = String::from_utf8(entry.name)
                .map_err(|_| integrity("an entry name is not UTF-8".into()))?;
            match entry.kind {
                TreeEntryKind::File { blob, .. } => Ok((name, blob.as_blob_cid().clone())),
                _ => Err(integrity(format!("entry `{name}` is not an event file"))),
            }
        })
        .collect()
}

/// The FlowEvent `bytes` hold, refused unless they are its canonical encoding and its atom CID is
/// `cid`.
fn decode_event(cid: &Cid, bytes: &[u8]) -> Result<FlowEvent, String> {
    let event: FlowEvent =
        serde_ipld_dagcbor::from_slice(bytes).map_err(|_| "decode".to_string())?;
    let canonical = canonical_bytes(&event).map_err(|_| "canonical".to_string())?;
    if canonical != bytes || atom_cid(&event).map_err(|_| "canonical".to_string())? != *cid {
        return Err("canonical".into());
    }
    Ok(event)
}

/// Append `events`, read from `device`'s channel, to this device's flow store, each with a
/// `via:<device>` slot, under one transaction ([`SidecarFlowStore::transaction`]). An event this
/// device's own `allow` does not admit is refused; an event already folded via `device` is not
/// appended again.
///
/// # Errors
///
/// [`PeerError::FlowStore`] when the store cannot be read or appended to;
/// [`PeerError::FoldIntegrity`] when an event is not at its stated CID.
pub fn fold_events(
    store: &SidecarFlowStore,
    device: &str,
    events: &[ChannelEvent],
    allow: &CrossingAllowList,
) -> Result<FoldOutcome, PeerError> {
    fold_events_admitted(store, device, events, allow, None)
}

/// [`fold_events`], refusing with [`PeerError::FoldRevoked`] — and appending nothing — when
/// `admission` no longer admits `device`. The check is made under the flow store's exclusive
/// lock, immediately before the first append; a poisoned admission admits nobody.
fn fold_events_admitted(
    store: &SidecarFlowStore,
    device: &str,
    events: &[ChannelEvent],
    allow: &CrossingAllowList,
    admission: Option<&SharedAdmission>,
) -> Result<FoldOutcome, PeerError> {
    let mut tx = store.transaction().map_err(PeerError::FlowStore)?;
    let mut held: HashSet<Cid> = tx
        .records()
        .map_err(PeerError::FlowStore)?
        .into_iter()
        .map(|(cid, _)| cid)
        .collect();
    let admitted = admission.is_none_or(|admission| {
        admission
            .read()
            .is_ok_and(|admission| admission.admits_device(device))
    });
    if !admitted {
        return Err(PeerError::FoldRevoked {
            device: device.to_string(),
        });
    }
    let mut outcome = FoldOutcome::default();
    for ChannelEvent { cid, event } in events {
        if atom_cid(event).map_err(PeerError::FlowStore)? != *cid {
            return Err(integrity(format!("event {cid} is not at its own CID")));
        }
        if !allow.admits(event) {
            outcome.refused += 1;
            outcome.refused_cids.push(*cid);
            continue;
        }
        let folded = with_via(event, device);
        let folded_cid = atom_cid(&folded).map_err(PeerError::FlowStore)?;
        if !held.insert(folded_cid) {
            outcome.already += 1;
            continue;
        }
        tx.append(FlowRecord::Event(folded))
            .map_err(PeerError::FlowStore)?;
        outcome.appended += 1;
    }
    Ok(outcome)
}

/// The original CIDs of every event this device has folded via `device`.
///
/// # Errors
///
/// [`PeerError::FlowStore`] when the store cannot be read.
pub fn folded_from<S>(store: &S, device: &str) -> Result<HashSet<Cid>, PeerError>
where
    S: FlowStore + ?Sized,
{
    let mut known = HashSet::new();
    for (_, event) in store.events().map_err(PeerError::FlowStore)? {
        if via_device(&event) == Some(device) {
            known.insert(atom_cid(&without_via(event)).map_err(PeerError::FlowStore)?);
        }
    }
    Ok(known)
}

/// Which tree a fold reads: `root`, announced by `device`, diffed from `since` (the root last
/// folded from that device, if any). With `admission`, the device must still be admitted when the
/// fold appends: the daemon passes the admission it reloads, so a Contest folded while the fold
/// ran leaves nothing of it appended.
#[derive(Debug, Clone, Copy)]
pub struct FoldTarget<'a> {
    pub device: &'a str,
    pub since: Option<&'a BlobCid>,
    pub root: &'a BlobCid,
    pub admission: Option<&'a SharedAdmission>,
    /// The folding device itself: whose nack leaf the fold reads, and where it quarantines.
    pub reader: FoldReader<'a>,
}

/// The folding device itself: whose nack leaf to read from the peer, and where its private
/// quarantine lives.
#[derive(Debug, Clone, Copy)]
pub struct FoldReader<'a> {
    pub device: &'a str,
    pub state_dir: &'a Path,
}

/// What [`fold_root`] did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FoldReport {
    pub outcome: FoldOutcome,
    /// False when the fold stopped short ([`Fetched::complete`]); the head is then not recorded.
    pub complete: bool,
    /// Events refused for exceeding the per-event cap.
    pub oversized: u64,
    /// Whether the fold stopped at its byte budget.
    pub over_budget: bool,
    /// Leaves skipped by this fold.
    pub quarantined: Vec<Quarantined>,
    /// The author's nack leaf for this device at the folded root.
    pub peer_nack: NackLeaf,
}

/// Fold `target` from the device whose tier answers at `peer`: dial it, fetch what this device has
/// not folded within `limits`, and append it.
///
/// # Errors
///
/// [`PeerError::TierConnect`] when the device cannot be reached, and the [`fetch_channel`] and
/// [`fold_events`] errors.
pub async fn fold_root(
    endpoint: &Endpoint,
    peer: NodeAddr,
    target: FoldTarget<'_>,
    store: &SidecarFlowStore,
    allow: &CrossingAllowList,
    tree: &TreeLimits,
    fold_limits: &FoldLimits,
) -> Result<FoldReport, PeerError> {
    let known = folded_from(store, target.device)?;
    let connection = endpoint
        .connect(peer, TIER_ALPN)
        .await
        .map_err(|e| PeerError::TierConnect(Box::new(e)))?;
    let fetched = fetch_channel_for(
        &connection,
        target.since,
        target.root,
        &known,
        tree,
        fold_limits,
        Some(target.reader.device),
    )
    .await;
    connection.close(VarInt::from_u32(0), b"");
    let fetched = fetched?;
    write_quarantine(target.reader.state_dir, target.device, &fetched.quarantined)?;
    let outcome = fold_events_admitted(
        store,
        target.device,
        &fetched.events,
        allow,
        target.admission,
    )?;
    let complete = fetched.complete
        && (fetched.quarantined.is_empty() || author_reads_nacks(store, target.device)?);
    Ok(FoldReport {
        outcome,
        complete,
        oversized: fetched.oversized,
        over_budget: fetched.over_budget,
        quarantined: fetched.quarantined,
        peer_nack: fetched.peer_nack,
    })
}

fn author_reads_nacks(store: &SidecarFlowStore, device: &str) -> Result<bool, PeerError> {
    let mut latest = None;
    for (written, (_, event)) in (1u64..).zip(store.events().map_err(PeerError::FlowStore)?) {
        if via_device(&event) != Some(device) {
            continue;
        }
        let Some(moor) =
            LeaseEvent::from_event(&event, written).filter(|event| event.kind == LeaseKind::Moor)
        else {
            continue;
        };
        let order = (moor.dseq.is_some(), moor.dseq, moor.at, written);
        if latest.as_ref().is_none_or(|(prior, _)| order > *prior) {
            latest = Some((order, moor.reads_nacks));
        }
    }
    Ok(latest.is_some_and(|(_, reads)| reads))
}

#[derive(serde::Serialize)]
struct QuarantineLine<'a> {
    cid: String,
    path: &'a str,
    reason: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    bytes: Option<&'a [u8]>,
}

fn write_quarantine(
    state_dir: &Path,
    device: &str,
    quarantined: &[Quarantined],
) -> Result<(), PeerError> {
    if quarantined.is_empty() {
        return Ok(());
    }
    let dir = state_dir.join("quarantine");
    fs::create_dir_all(&dir).map_err(|source| PeerError::HeadTableIo {
        path: dir.clone(),
        source,
    })?;
    let safe: String = device
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || ".:_-".contains(c) {
                c
            } else {
                '_'
            }
        })
        .collect();
    let path = dir.join(format!("{safe}.jsonl"));
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|source| PeerError::HeadTableIo {
            path: path.clone(),
            source,
        })?;
    for entry in quarantined {
        let line = QuarantineLine {
            cid: entry.cid.to_string(),
            path: &entry.path,
            reason: &entry.reason,
            bytes: entry.bytes.as_deref(),
        };
        serde_json::to_writer(&mut file, &line).map_err(|source| PeerError::HeadTableIo {
            path: path.clone(),
            source: std::io::Error::other(source),
        })?;
        file.write_all(b"\n")
            .map_err(|source| PeerError::HeadTableIo {
                path: path.clone(),
                source,
            })?;
    }
    file.sync_data()
        .map_err(|source| PeerError::HeadTableIo { path, source })
}

fn with_via(event: &FlowEvent, device: &str) -> FlowEvent {
    let mut folded = event.clone();
    folded.classified_as.push(format!("{VIA_SLOT}{device}"));
    folded
}

fn without_via(mut event: FlowEvent) -> FlowEvent {
    event
        .classified_as
        .retain(|slot| !slot.starts_with(VIA_SLOT));
    event
}

fn integrity(reason: String) -> PeerError {
    PeerError::FoldIntegrity { reason }
}

fn unanswered(what: String, answer: &TierAnswer) -> PeerError {
    let why = match answer {
        TierAnswer::Failed { reason } => format!(" ({reason})"),
        TierAnswer::Unanswered { .. } => " (unanswered)".to_string(),
        _ => " (an answer of another kind)".to_string(),
    };
    PeerError::FoldUnanswered {
        what: format!("{what}{why}"),
    }
}
