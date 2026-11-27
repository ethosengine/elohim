//! `FlowStore` — the polymorphic persistence seam. One model, three depths:
//! sidecar (`.eprfs/status/`, offline floor — implemented here), diesel projection and
//! DHT rails (implemented in their own crates against this trait).

use std::collections::HashSet;
#[cfg(feature = "sidecar")]
use std::ffi::OsStr;
#[cfg(feature = "sidecar")]
use std::fs::{self, OpenOptions};
#[cfg(feature = "sidecar")]
use std::io::Write;
#[cfg(feature = "sidecar")]
use std::path::{Path, PathBuf};
#[cfg(feature = "sidecar")]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(feature = "sidecar")]
use std::time::{SystemTime, UNIX_EPOCH};

use cid::Cid;
use elohim_epr::algedonic::AlgedonicEvidence;
#[cfg(feature = "sidecar")]
use multihash_codetable::{Code, MultihashDigest};
use serde::{Deserialize, Serialize};

#[cfg(feature = "sidecar")]
use crate::error::FabricError;
use crate::error::Result;
use crate::fold::fulfillment;
use crate::model::{
    atom_cid, edge_fp, Commitment, CommitmentState, DepEdge, FlowEvent, Intent, Process,
    ProcessSpec,
};

/// One appended fabric record. Append-only everywhere; corrections are new records.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum FlowRecord {
    Intent(Intent),
    Commitment(Commitment),
    Event(FlowEvent),
    Process(Process),
    Spec(ProcessSpec),
    Edge(DepEdge),
}

impl FlowRecord {
    /// The record's identity: the atom CID of the PAYLOAD (the envelope enum is a
    /// storage detail and never participates in identity).
    pub fn cid(&self) -> Result<Cid> {
        match self {
            FlowRecord::Intent(i) => atom_cid(i),
            FlowRecord::Commitment(c) => atom_cid(c),
            FlowRecord::Event(e) => atom_cid(e),
            FlowRecord::Process(p) => atom_cid(p),
            FlowRecord::Spec(s) => atom_cid(s),
            FlowRecord::Edge(e) => atom_cid(e),
        }
    }
}

pub trait FlowStore {
    /// Append a record; returns the record's atom CID.
    fn append(&mut self, record: FlowRecord) -> Result<Cid>;

    /// Append one logical batch. Stores without batch mechanics preserve the call order; the
    /// sidecar override additionally makes the whole batch one reader-visible prefix transition.
    fn append_batch(&mut self, records: Vec<FlowRecord>) -> Result<Vec<Cid>> {
        records
            .into_iter()
            .map(|record| self.append(record))
            .collect()
    }

    /// All records in append order, with their CIDs.
    fn records(&self) -> Result<Vec<(Cid, FlowRecord)>>;

    fn events(&self) -> Result<Vec<(Cid, FlowEvent)>> {
        Ok(self
            .records()?
            .into_iter()
            .filter_map(|(cid, record)| match record {
                FlowRecord::Event(e) => Some((cid, e)),
                _ => None,
            })
            .collect())
    }

    fn commitments(&self) -> Result<Vec<(Cid, Commitment)>> {
        Ok(self
            .records()?
            .into_iter()
            .filter_map(|(cid, record)| match record {
                FlowRecord::Commitment(c) => Some((cid, c)),
                _ => None,
            })
            .collect())
    }

    fn processes(&self) -> Result<Vec<(Cid, Process)>> {
        Ok(self
            .records()?
            .into_iter()
            .filter_map(|(cid, record)| match record {
                FlowRecord::Process(p) => Some((cid, p)),
                _ => None,
            })
            .collect())
    }

    /// `DepEdge` records collapsed to latest-per-`(from, to)`: the winner is the highest
    /// `sealed_at`, ties broken by later file/append order (the reseal/hold semantics —
    /// a later record always wins a same-timestamp tie because it is appended after).
    /// `records()` is already in append order, so a plain forward fold with `>=` on
    /// `sealed_at` gives exactly that: a strictly-later timestamp always wins, and an
    /// equal timestamp is won by whichever copy is folded in last (the later one).
    ///
    /// Read-path invariant validation: each candidate is re-checked against
    /// [`DepEdge::validate`] before it may win a slot. A record that fails validation (the
    /// `sealed_cid.is_some() ⇔ CiteSeal` invariant broken — a hand-built struct literal, a
    /// tampered sidecar line) is SKIPPED, not surfaced as a read error — one poisoned record
    /// must not fail the whole `edges()` call. This crate stays deliberately dependency-light
    /// (no `log`/`tracing`, see `Cargo.toml`), so the skip carries no runtime-visible signal
    /// beyond this doc comment; a caller that needs to enumerate rejects can walk `records()`
    /// and call `DepEdge::validate` itself.
    fn edges(&self) -> Result<Vec<(Cid, DepEdge)>> {
        let mut latest: Vec<(String, Cid, DepEdge)> = Vec::new();
        for (cid, record) in self.records()? {
            let FlowRecord::Edge(edge) = record else {
                continue;
            };
            if edge.validate().is_err() {
                continue;
            }
            let key = edge_fp(&edge.from, &edge.to);
            match latest.iter_mut().find(|(k, _, _)| *k == key) {
                Some(slot) => {
                    if edge.sealed_at >= slot.2.sealed_at {
                        slot.1 = cid;
                        slot.2 = edge;
                    }
                }
                None => latest.push((key, cid, edge)),
            }
        }
        Ok(latest
            .into_iter()
            .map(|(_, cid, edge)| (cid, edge))
            .collect())
    }

    /// Commitments in `scope` not yet discharged: state is Proposed/Active AND no
    /// appended event's `fulfills` names them.
    fn unfulfilled_in_scope(&self, scope: &Cid) -> Result<Vec<(Cid, Commitment)>> {
        let discharged: HashSet<Cid> = self
            .events()?
            .into_iter()
            .flat_map(|(_, e)| e.fulfills)
            .collect();
        Ok(self
            .commitments()?
            .into_iter()
            .filter(|(cid, c)| {
                &c.in_scope_of == scope
                    && matches!(c.state, CommitmentState::Proposed | CommitmentState::Active)
                    && !discharged.contains(cid)
            })
            .collect())
    }

    /// Open pain across the store: for every OPEN commitment (Proposed/Active) that declared a
    /// [`crate::model::Bound`], the algedonic evidence its folded stock has crossed — keyed by
    /// the commitment's CID, which IS that evidence's `bound_ref`.
    ///
    /// Silence has three honest sources here and none of them is an entry: a promise that
    /// declared no ceiling, a stock still inside the band, and a promise no longer open (the
    /// same Proposed/Active rule [`Self::unfulfilled_in_scope`] applies — a revoked or fulfilled
    /// promise cannot be in *open* pain). Nothing is emitted from this projection; deciding
    /// whether an entry becomes a signal is the caller's, under
    /// `elohim_epr::algedonic::should_emit`.
    fn open_pain(&self) -> Result<Vec<(Cid, AlgedonicEvidence)>> {
        let events: Vec<FlowEvent> = self.events()?.into_iter().map(|(_, e)| e).collect();
        Ok(self
            .commitments()?
            .into_iter()
            .filter(|(_, c)| matches!(c.state, CommitmentState::Proposed | CommitmentState::Active))
            .filter_map(|(cid, c)| fulfillment(&cid, &c, &events).pain.map(|pain| (cid, pain)))
            .collect())
    }
}

/// In-memory store — tests and short-lived walks.
#[derive(Debug, Default)]
pub struct MemoryFlowStore {
    records: Vec<(Cid, FlowRecord)>,
}

impl MemoryFlowStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl FlowStore for MemoryFlowStore {
    fn append(&mut self, record: FlowRecord) -> Result<Cid> {
        let cid = record.cid()?;
        self.records.push((cid, record));
        Ok(cid)
    }

    fn records(&self) -> Result<Vec<(Cid, FlowRecord)>> {
        Ok(self.records.clone())
    }
}

/// The offline floor: an append-only JSONL log under `<root>/.eprfs/status/flows.jsonl`.
/// Each line carries the record and its dag-cbor atom CID; CIDs are re-verified on read
/// (a tampered line is an integrity error, not silent drift).
///
/// Behind the default-on `sidecar` feature, which is the crate's ONLY `std::fs` surface.
/// A consumer that must stay filesystem-free — `elohim-ark-core`, whose purity boundary is a
/// test over its own dependency graph — takes this crate with `default-features = false` and
/// keeps the model, fold, and stock layers.
#[cfg(feature = "sidecar")]
#[derive(Debug)]
pub struct SidecarFlowStore {
    log_path: PathBuf,
}

#[cfg(feature = "sidecar")]
const VERIFIED_PREFIX_CACHE_SUBDIRECTORY: &str = "elohim/flow-verified-prefix";
#[cfg(feature = "sidecar")]
const VERIFIED_PREFIX_CACHE_READERS: usize = 4;
#[cfg(feature = "sidecar")]
const VERIFIED_PREFIX_CACHE_FILES: usize = 64;
#[cfg(feature = "sidecar")]
const VERIFIED_PREFIX_CACHE_EXTENSION: &str = ".json";

#[cfg(feature = "sidecar")]
static CACHE_FALLBACK_REPORTED: AtomicBool = AtomicBool::new(false);
#[cfg(feature = "sidecar")]
static CACHE_WRITE_FAILURE_REPORTED: AtomicBool = AtomicBool::new(false);

#[cfg(feature = "sidecar")]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReaderIdentity {
    executable_path: PathBuf,
    executable_len: u64,
    executable_modified_before_epoch: bool,
    executable_modified_secs: u64,
    executable_modified_nanos: u32,
    reader_version: String,
    log_path: PathBuf,
}

#[cfg(feature = "sidecar")]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VerifiedPrefixCache {
    reader: ReaderIdentity,
    prefix_len: u64,
    digest: Vec<u8>,
    record_count: u64,
}

#[cfg(feature = "sidecar")]
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VerifiedPrefixCacheMap {
    // Oldest write first. Replacing a reader moves it to the end, so eviction is deterministic
    // without trusting wall-clock ordering between processes. Each entry stays opaque here so a
    // reader replacing its own entry preserves every other reader's bytes, including fields from
    // a cache schema this build predates.
    entries: Vec<String>,
}

#[cfg(feature = "sidecar")]
struct CachedPrefix {
    prefix_len: u64,
    records: Vec<(u64, Cid, FlowRecord)>,
}

#[cfg(feature = "sidecar")]
enum CacheLookup {
    Hit(CachedPrefix),
    Miss,
    Mismatch(&'static str),
}

/// One immutable flow-log prefix with each decoded record's JSONL line-start offset.
#[cfg(feature = "sidecar")]
#[derive(Debug)]
pub struct FlowRecordPrefix {
    pub end_offset: u64,
    pub records: Vec<(u64, Cid, FlowRecord)>,
}

#[cfg(feature = "sidecar")]
impl SidecarFlowStore {
    /// Open (creating directories/log as needed) the sidecar under `root`.
    pub fn open(root: &Path) -> Result<Self> {
        let log_path = if root.join(".eprfs/status/feedback/state.json").exists() {
            // Once feedback can reclaim, a lost reference log must not be silently
            // recreated as an empty inventory by an unrelated flow operation.
            let path = root.join(".eprfs/status/flows.jsonl");
            std::fs::File::open(&path)?;
            path
        } else {
            crate::sidecar::open_log(root, "flows.jsonl")?
        };
        Ok(Self { log_path })
    }

    pub fn log_path(&self) -> &Path {
        &self.log_path
    }

    /// The byte offset immediately after a complete, immutable prefix, captured under the log's
    /// shared lock. Every record in that prefix starts below this offset.
    pub fn end_offset(&self) -> Result<u64> {
        crate::sidecar::flow_length(&self.log_path)
    }

    /// Every record in one captured prefix, with its byte offset at the start of its JSONL line,
    /// and the offset immediately after the prefix.
    pub fn records_with_offsets(&self) -> Result<FlowRecordPrefix> {
        let contents = crate::sidecar::flow_contents(&self.log_path)?;
        let end_offset = u64::try_from(contents.len())
            .map_err(|_| FabricError::Decode("flow log is too large for offsets".into()))?;
        Ok(FlowRecordPrefix {
            end_offset,
            records: decode_flow_records_with_cache(&self.log_path, &contents)?,
        })
    }

    /// Retain feedback coordination and reference-admission state across an operation.
    /// Flow-log locks are deliberately per read or append batch: a reader captures a whole prefix,
    /// and each batch validates the tail once and syncs once before releasing its exclusive lock.
    /// Appends are not rolled back if a subsequent append or check fails. No recovery rewrites
    /// damaged bytes.
    pub fn transaction(&self) -> Result<SidecarFlowTransaction> {
        let root = self
            .log_path
            .parent()
            .and_then(Path::parent)
            .and_then(Path::parent)
            .ok_or_else(|| FabricError::InvalidFeedback("invalid flow store root".into()))?;
        let guard = crate::feedback_store::coordination(root)?;
        Ok(SidecarFlowTransaction {
            feedback_guard: Some(guard),
            feedback_seen: None,
            root: root.to_path_buf(),
            log_path: self.log_path.clone(),
        })
    }
}

/// A process-exclusive view of the existing flow store, implementing the same store seam.
#[cfg(feature = "sidecar")]
pub struct SidecarFlowTransaction {
    feedback_guard: Option<crate::sidecar::LockedLog>,
    feedback_seen: Option<HashSet<Cid>>,
    root: PathBuf,
    log_path: PathBuf,
}

#[cfg(feature = "sidecar")]
impl SidecarFlowTransaction {
    /// Inspect evidence while retaining the same lock through reference admission.
    pub fn feedback_records(
        &mut self,
        policy: crate::feedback::Policy,
    ) -> Result<Vec<crate::feedback::Entry>> {
        if self.feedback_guard.is_none() {
            return Err(FabricError::InvalidFeedback(
                "reference admission requires exclusive coordination".into(),
            ));
        }
        let entries =
            crate::feedback_store::FeedbackStore::new(&self.root, policy)?.read_locked()?;
        self.feedback_seen = Some(entries.iter().map(|entry| entry.cid).collect());
        Ok(entries)
    }
}

#[cfg(feature = "sidecar")]
#[derive(Serialize, Deserialize)]
struct SidecarLine {
    cid: String,
    record: FlowRecord,
}

#[cfg(feature = "sidecar")]
impl FlowStore for SidecarFlowStore {
    fn append(&mut self, record: FlowRecord) -> Result<Cid> {
        self.transaction()?.append(record)
    }

    fn records(&self) -> Result<Vec<(Cid, FlowRecord)>> {
        let contents = crate::sidecar::flow_contents(&self.log_path)?;
        Ok(decode_flow_records_with_cache(&self.log_path, &contents)?
            .into_iter()
            .map(|(_, cid, record)| (cid, record))
            .collect())
    }
}

#[cfg(feature = "sidecar")]
impl FlowStore for SidecarFlowTransaction {
    fn append(&mut self, record: FlowRecord) -> Result<Cid> {
        self.append_batch(vec![record]).map(|mut cids| {
            cids.pop()
                .expect("a one-record sidecar batch returns one CID")
        })
    }

    fn append_batch(&mut self, records: Vec<FlowRecord>) -> Result<Vec<Cid>> {
        for record in &records {
            let FlowRecord::Intent(intent) = record else {
                continue;
            };
            if intent
                .resource_spec
                .classified_as
                .iter()
                .any(|tag| tag == "feedback:repair")
                && !self
                    .feedback_seen
                    .as_ref()
                    .is_some_and(|ids| ids.contains(&intent.in_scope_of))
            {
                return Err(FabricError::InvalidFeedback(
                    "feedback reference admission requires retained evidence read in this transaction".into()));
            }
        }
        let encoded: Vec<(Cid, String)> = records
            .into_iter()
            .map(|record| {
                let cid = record.cid()?;
                let line = serde_json::to_string(&SidecarLine {
                    cid: cid.to_string(),
                    record,
                })
                .map_err(|e| FabricError::Encode(e.to_string()))?;
                Ok((cid, line))
            })
            .collect::<Result<_>>()?;
        // Only the tail can be adjacent to this append. Verify it under the same exclusive lock;
        // older records are still fully verified by readers after they release their brief lock.
        let mut log = crate::sidecar::LockedLog::exclusive(&self.log_path)?;
        if let Some(tail) = log.prepare_append(&self.root, &self.log_path)? {
            decode_flow_records(&(tail + "\n"))?;
        }
        log.append_batch(encoded.iter().map(|(_, line)| line.clone()))?;
        Ok(encoded.into_iter().map(|(cid, _)| cid).collect())
    }

    fn records(&self) -> Result<Vec<(Cid, FlowRecord)>> {
        let contents = crate::sidecar::flow_contents(&self.log_path)?;
        Ok(decode_flow_records_with_cache(&self.log_path, &contents)?
            .into_iter()
            .map(|(_, cid, record)| (cid, record))
            .collect())
    }
}

/// Decode and verify an already captured append-order log without filesystem access.
#[cfg(feature = "sidecar")]
pub fn decode_flow_records(contents: &str) -> Result<Vec<(Cid, FlowRecord)>> {
    Ok(decode_flow_records_with_offsets(contents)?
        .into_iter()
        .map(|(_, cid, record)| (cid, record))
        .collect())
}

#[cfg(feature = "sidecar")]
fn decode_flow_records_with_offsets(contents: &str) -> Result<Vec<(u64, Cid, FlowRecord)>> {
    decode_flow_record_segment(contents, 0, true)
}

#[cfg(feature = "sidecar")]
fn decode_flow_records_with_cache(
    log_path: &Path,
    contents: &str,
) -> Result<Vec<(u64, Cid, FlowRecord)>> {
    #[cfg(not(test))]
    let cache_directory = verified_prefix_cache_directory();
    #[cfg(test)]
    let cache_directory = None::<PathBuf>;
    let reader = cache_directory
        .as_ref()
        .and_then(|_| reader_identity(log_path).ok());
    decode_flow_records_with_reader(
        log_path,
        contents,
        cache_directory.as_deref(),
        reader.as_ref(),
        report_cache_fallback_once,
        report_cache_write_failure_once,
    )
}

#[cfg(feature = "sidecar")]
fn decode_flow_records_with_reader(
    log_path: &Path,
    contents: &str,
    cache_directory: Option<&Path>,
    reader: Option<&ReaderIdentity>,
    report_fallback: impl Fn(&str),
    report_write_failure: impl Fn(&FabricError),
) -> Result<Vec<(u64, Cid, FlowRecord)>> {
    let (Some(cache_directory), Some(reader)) = (cache_directory, reader) else {
        return decode_flow_records_with_offsets(contents);
    };
    match cached_prefix(cache_directory, contents, reader) {
        CacheLookup::Hit(CachedPrefix {
            prefix_len,
            mut records,
        }) => {
            let prefix_len_usize = usize::try_from(prefix_len)
                .map_err(|_| FabricError::Decode("cached prefix is too large".into()))?;
            let tail = contents
                .get(prefix_len_usize..)
                .ok_or_else(|| FabricError::Decode("cached prefix is not UTF-8 aligned".into()))?;
            records.extend(decode_flow_record_segment(tail, prefix_len, true)?);
            if prefix_len_usize < contents.len() {
                advance_verified_prefix_cache(
                    log_path,
                    cache_directory,
                    contents,
                    records.len(),
                    reader,
                    &report_write_failure,
                );
            }
            Ok(records)
        }
        CacheLookup::Miss => {
            let records = decode_flow_records_with_offsets(contents)?;
            advance_verified_prefix_cache(
                log_path,
                cache_directory,
                contents,
                records.len(),
                reader,
                &report_write_failure,
            );
            Ok(records)
        }
        CacheLookup::Mismatch(reason) => {
            report_fallback(reason);
            let records = decode_flow_records_with_offsets(contents)?;
            advance_verified_prefix_cache(
                log_path,
                cache_directory,
                contents,
                records.len(),
                reader,
                &report_write_failure,
            );
            Ok(records)
        }
    }
}

#[cfg(feature = "sidecar")]
fn cached_prefix(cache_directory: &Path, contents: &str, reader: &ReaderIdentity) -> CacheLookup {
    let path = verified_prefix_cache_path(cache_directory, &reader.log_path);
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return CacheLookup::Miss,
        Err(_) => return CacheLookup::Mismatch("unreadable cache"),
    };
    let cache_map: VerifiedPrefixCacheMap = match serde_json::from_slice(&bytes) {
        Ok(cache_map) => cache_map,
        Err(_) => return CacheLookup::Mismatch("corrupt cache"),
    };
    let mut matching = None;
    for encoded in &cache_map.entries {
        let cache: VerifiedPrefixCache = match serde_json::from_str(encoded) {
            Ok(cache) => cache,
            Err(_) => return CacheLookup::Mismatch("corrupt cache"),
        };
        if &cache.reader == reader {
            matching = Some(cache);
        }
    }
    let Some(cache) = matching else {
        return CacheLookup::Miss;
    };
    let Ok(prefix_len) = usize::try_from(cache.prefix_len) else {
        return CacheLookup::Mismatch("corrupt prefix length");
    };
    let Some(prefix_bytes) = contents.as_bytes().get(..prefix_len) else {
        return CacheLookup::Mismatch("log is shorter than cached prefix");
    };
    if !prefix_bytes.is_empty() && !prefix_bytes.ends_with(b"\n") {
        return CacheLookup::Mismatch("cached prefix is not record-aligned");
    }
    if sha2_digest(prefix_bytes) != cache.digest {
        return CacheLookup::Mismatch("prefix digest mismatch");
    }
    let Ok(prefix) = std::str::from_utf8(prefix_bytes) else {
        return CacheLookup::Mismatch("cached prefix is not UTF-8 aligned");
    };
    let records = match decode_flow_record_segment(prefix, 0, false) {
        Ok(records) => records,
        Err(_) => return CacheLookup::Mismatch("cached prefix decode failed"),
    };
    if u64::try_from(records.len()).ok() != Some(cache.record_count) {
        return CacheLookup::Mismatch("cached record count mismatch");
    }
    CacheLookup::Hit(CachedPrefix {
        prefix_len: cache.prefix_len,
        records,
    })
}

#[cfg(feature = "sidecar")]
fn advance_verified_prefix_cache(
    log_path: &Path,
    cache_directory: &Path,
    contents: &str,
    record_count: usize,
    reader: &ReaderIdentity,
    report_write_failure: impl Fn(&FabricError),
) {
    if let Err(error) =
        write_verified_prefix_cache(log_path, cache_directory, contents, record_count, reader)
    {
        report_write_failure(&error);
    }
}

#[cfg(feature = "sidecar")]
fn write_verified_prefix_cache(
    log_path: &Path,
    cache_directory: &Path,
    contents: &str,
    record_count: usize,
    reader: &ReaderIdentity,
) -> Result<bool> {
    let prefix_len = u64::try_from(contents.len())
        .map_err(|_| FabricError::Encode("flow log is too large for cache".into()))?;
    let record_count = u64::try_from(record_count)
        .map_err(|_| FabricError::Encode("flow record count is too large for cache".into()))?;
    let candidate = VerifiedPrefixCache {
        reader: reader.clone(),
        prefix_len,
        digest: sha2_digest(contents.as_bytes()),
        record_count,
    };
    let cache_path = verified_prefix_cache_path(cache_directory, &reader.log_path);

    // Serialize cache writers with appenders on the log's own lock, but never wait for this
    // optimisation. Parsing and hashing stay outside the critical section.
    let _log_guard = match crate::sidecar::LockedLog::try_exclusive(log_path) {
        Ok(guard) => guard,
        Err(FabricError::Io(error)) if error.kind() == std::io::ErrorKind::WouldBlock => {
            return Ok(false)
        }
        Err(error) => return Err(error),
    };
    let current_log_len = fs::metadata(log_path)?.len();
    fs::create_dir_all(cache_directory)?;
    let mut cache_map = match fs::read(&cache_path) {
        Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_default(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            VerifiedPrefixCacheMap::default()
        }
        Err(error) => return Err(error.into()),
    };
    let mut decoded_entries = Vec::with_capacity(cache_map.entries.len());
    for encoded in std::mem::take(&mut cache_map.entries) {
        let Ok(entry) = serde_json::from_str::<VerifiedPrefixCache>(&encoded) else {
            decoded_entries.clear();
            break;
        };
        decoded_entries.push((encoded, entry));
    }
    if let Some((_, existing)) = decoded_entries
        .iter()
        .find(|(_, entry)| entry.reader == candidate.reader)
    {
        let same_prefix = existing.prefix_len == candidate.prefix_len
            && existing.digest == candidate.digest
            && existing.record_count == candidate.record_count;
        if same_prefix {
            return Ok(true);
        }
        // A reader that captured an older prefix must not regress its own entry after another
        // process advanced it. A genuinely shortened/replaced log may replace the stale entry.
        if existing.prefix_len > candidate.prefix_len && current_log_len > candidate.prefix_len {
            return Ok(true);
        }
    }
    cache_map.entries = decoded_entries
        .into_iter()
        .filter(|(_, entry)| entry.reader != candidate.reader)
        .map(|(encoded, _)| encoded)
        .collect();
    cache_map.entries.push(
        serde_json::to_string(&candidate)
            .map_err(|error| FabricError::Encode(error.to_string()))?,
    );
    while cache_map.entries.len() > VERIFIED_PREFIX_CACHE_READERS {
        cache_map.entries.remove(0);
    }

    let encoded =
        serde_json::to_vec(&cache_map).map_err(|error| FabricError::Encode(error.to_string()))?;
    let temp_path = cache_path.with_extension("json.tmp");
    let mut temp = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&temp_path)?;
    temp.write_all(&encoded)?;
    temp.sync_all()?;
    fs::rename(temp_path, cache_path)?;
    prune_verified_prefix_cache_directory(cache_directory);
    Ok(true)
}

#[cfg(feature = "sidecar")]
fn reader_identity(log_path: &Path) -> std::io::Result<ReaderIdentity> {
    let log_path = fs::canonicalize(log_path)?;
    let executable_path = fs::canonicalize(std::env::current_exe()?)?;
    let metadata = fs::metadata(&executable_path)?;
    let modified = metadata.modified()?;
    let (executable_modified_before_epoch, modified) = match modified.duration_since(UNIX_EPOCH) {
        Ok(duration) => (false, duration),
        Err(error) => (true, error.duration()),
    };
    Ok(ReaderIdentity {
        executable_path,
        executable_len: metadata.len(),
        executable_modified_before_epoch,
        executable_modified_secs: modified.as_secs(),
        executable_modified_nanos: modified.subsec_nanos(),
        reader_version: env!("CARGO_PKG_VERSION").to_string(),
        log_path,
    })
}

#[cfg(feature = "sidecar")]
fn report_cache_fallback_once(reason: &str) {
    if !CACHE_FALLBACK_REPORTED.swap(true, Ordering::Relaxed) {
        eprintln!("flow verified-prefix cache fallback: {reason}");
    }
}

#[cfg(feature = "sidecar")]
fn report_cache_write_failure_once(error: &FabricError) {
    if !CACHE_WRITE_FAILURE_REPORTED.swap(true, Ordering::Relaxed) {
        eprintln!("flow verified-prefix cache not advanced: {error}");
    }
}

#[cfg(feature = "sidecar")]
fn verified_prefix_cache_directory() -> Option<PathBuf> {
    verified_prefix_cache_directory_from(
        std::env::var_os("XDG_CACHE_HOME").as_deref(),
        std::env::var_os("HOME").as_deref(),
    )
}

#[cfg(feature = "sidecar")]
fn verified_prefix_cache_directory_from(
    xdg_cache_home: Option<&OsStr>,
    home: Option<&OsStr>,
) -> Option<PathBuf> {
    let base = xdg_cache_home
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| {
            home.map(PathBuf::from)
                .filter(|path| path.is_absolute())
                .map(|path| path.join(".cache"))
        })?;
    Some(base.join(VERIFIED_PREFIX_CACHE_SUBDIRECTORY))
}

#[cfg(feature = "sidecar")]
fn verified_prefix_cache_path(cache_directory: &Path, canonical_log_path: &Path) -> PathBuf {
    let digest = sha2_digest(canonical_log_path.as_os_str().as_encoded_bytes());
    let mut name = String::with_capacity(digest.len() * 2 + VERIFIED_PREFIX_CACHE_EXTENSION.len());
    for byte in digest {
        use std::fmt::Write as _;
        write!(name, "{byte:02x}").expect("writing to a String cannot fail");
    }
    name.push_str(VERIFIED_PREFIX_CACHE_EXTENSION);
    cache_directory.join(name)
}

#[cfg(feature = "sidecar")]
fn is_verified_prefix_cache_file(name: &OsStr) -> bool {
    let Some(name) = name.to_str() else {
        return false;
    };
    let Some(stem) = name.strip_suffix(VERIFIED_PREFIX_CACHE_EXTENSION) else {
        return false;
    };
    stem.len() == 64
        && stem
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(feature = "sidecar")]
fn prune_verified_prefix_cache_directory(cache_directory: &Path) {
    let Ok(entries) = fs::read_dir(cache_directory) else {
        return;
    };
    let mut cache_files = entries
        .filter_map(std::result::Result::ok)
        .filter(|entry| is_verified_prefix_cache_file(&entry.file_name()))
        .filter_map(|entry| {
            let metadata = entry.metadata().ok()?;
            metadata.is_file().then(|| {
                (
                    metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                    entry.file_name(),
                    entry.path(),
                )
            })
        })
        .collect::<Vec<_>>();
    if cache_files.len() <= VERIFIED_PREFIX_CACHE_FILES {
        return;
    }
    cache_files.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));
    let remove_count = cache_files.len() - VERIFIED_PREFIX_CACHE_FILES;
    for (_, _, path) in cache_files.into_iter().take(remove_count) {
        let _ = fs::remove_file(path);
    }
}

#[cfg(feature = "sidecar")]
fn sha2_digest(bytes: &[u8]) -> Vec<u8> {
    Code::Sha2_256.digest(bytes).digest().to_vec()
}

#[cfg(feature = "sidecar")]
fn decode_flow_record_segment(
    contents: &str,
    starting_offset: u64,
    verify_cids: bool,
) -> Result<Vec<(u64, Cid, FlowRecord)>> {
    if !contents.is_empty() && !contents.ends_with('\n') {
        return Err(FabricError::Decode("unterminated record log".into()));
    }
    let mut records = Vec::new();
    let mut offset = starting_offset;
    for terminated in contents.split_inclusive('\n') {
        let line = terminated.strip_suffix('\n').unwrap_or(terminated);
        let line = line.strip_suffix('\r').unwrap_or(line);
        if line.trim().is_empty() {
            offset = offset.saturating_add(u64::try_from(terminated.len()).unwrap_or(u64::MAX));
            continue;
        }
        let parsed: SidecarLine =
            serde_json::from_str(line).map_err(|e| FabricError::Decode(e.to_string()))?;
        let cid = if verify_cids {
            let computed = parsed.record.cid()?;
            if computed.to_string() == parsed.cid {
                computed
            } else {
                // A line from a NEWER writer (an additive field this binary predates) decodes
                // with that field dropped and so re-hashes differently. Say so, naming the
                // fields and this reader's version, instead of calling it tampering.
                let unknown = unknown_fields(line, &parsed.record);
                if !unknown.is_empty() {
                    return Err(FabricError::NewerWriter {
                        stored: parsed.cid,
                        fields: unknown.join(", "),
                        reader: env!("CARGO_PKG_VERSION"),
                    });
                }
                return Err(FabricError::Integrity {
                    stored: parsed.cid,
                    computed: computed.to_string(),
                });
            }
        } else {
            parsed
                .cid
                .parse::<Cid>()
                .map_err(|error| FabricError::Decode(error.to_string()))?
        };
        records.push((offset, cid, parsed.record));
        offset = offset.saturating_add(u64::try_from(terminated.len()).unwrap_or(u64::MAX));
    }
    Ok(records)
}

/// The keys a stored line's `record` carries that this reader's types do not emit: present when
/// the line was written by a newer writer whose record has a field this binary predates.
#[cfg(feature = "sidecar")]
fn unknown_fields(line: &str, record: &FlowRecord) -> Vec<String> {
    let Ok(raw) = serde_json::from_str::<serde_json::Value>(line) else {
        return Vec::new();
    };
    let Some(raw) = raw.get("record").and_then(|r| r.as_object()) else {
        return Vec::new();
    };
    let Ok(serde_json::Value::Object(known)) = serde_json::to_value(record) else {
        return Vec::new();
    };
    let mut unknown: Vec<String> = raw
        .keys()
        .filter(|k| !known.contains_key(*k))
        .cloned()
        .collect();
    unknown.sort();
    unknown
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{atom_cid, AgentRef, DepEdge, Governor};

    fn agent(name: &str) -> AgentRef {
        AgentRef(format!("uhCAk-test-{name}"))
    }

    /// A CiteSeal edge sealed against a distinct upstream body per `seal_label`, so two
    /// "resealings" of the same `(from, to)` slot carry genuinely different sealed_cids.
    fn edge(from: &str, to: &str, sealed_at: i64, seal_label: &str) -> DepEdge {
        let sealed_cid = atom_cid(&seal_label.to_string()).expect("cid");
        DepEdge::new(
            from.into(),
            to.into(),
            None,
            Governor::CiteSeal,
            Some(sealed_cid),
            agent("claude"),
            sealed_at,
            None,
        )
        .expect("valid edge")
    }

    #[cfg(feature = "sidecar")]
    fn read_cache_map(store: &SidecarFlowStore, cache_directory: &Path) -> VerifiedPrefixCacheMap {
        let reader = reader_identity(store.log_path()).unwrap();
        let path = verified_prefix_cache_path(cache_directory, &reader.log_path);
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
    }

    #[cfg(feature = "sidecar")]
    fn read_cache_entries(
        store: &SidecarFlowStore,
        cache_directory: &Path,
    ) -> Vec<VerifiedPrefixCache> {
        read_cache_map(store, cache_directory)
            .entries
            .into_iter()
            .map(|encoded| serde_json::from_str(&encoded).unwrap())
            .collect()
    }

    #[cfg(feature = "sidecar")]
    fn read_cache(store: &SidecarFlowStore, cache_directory: &Path) -> VerifiedPrefixCache {
        let reader = reader_identity(store.log_path()).unwrap();
        read_cache_entries(store, cache_directory)
            .into_iter()
            .find(|entry| entry.reader == reader)
            .unwrap()
    }

    #[cfg(feature = "sidecar")]
    fn test_reader(label: &str, log_path: &Path) -> ReaderIdentity {
        ReaderIdentity {
            executable_path: PathBuf::from(format!("/test/readers/{label}")),
            executable_len: 100 + u64::try_from(label.len()).unwrap(),
            executable_modified_before_epoch: false,
            executable_modified_secs: 1_700_000_000,
            executable_modified_nanos: u32::try_from(label.len()).unwrap(),
            reader_version: env!("CARGO_PKG_VERSION").to_string(),
            log_path: std::fs::canonicalize(log_path).unwrap(),
        }
    }

    #[cfg(feature = "sidecar")]
    fn records_for_test_reader(
        store: &SidecarFlowStore,
        cache_directory: Option<&Path>,
        reader: Option<&ReaderIdentity>,
        fallback_count: &std::cell::Cell<usize>,
        write_failure_count: &std::cell::Cell<usize>,
    ) -> Result<Vec<(u64, Cid, FlowRecord)>> {
        let contents = crate::sidecar::flow_contents(store.log_path())?;
        decode_flow_records_with_reader(
            store.log_path(),
            &contents,
            cache_directory,
            reader,
            |_| fallback_count.set(fallback_count.get() + 1),
            |_| write_failure_count.set(write_failure_count.get() + 1),
        )
    }

    #[cfg(feature = "sidecar")]
    fn records_for_test_cache(
        store: &SidecarFlowStore,
        cache_directory: &Path,
    ) -> Result<Vec<(Cid, FlowRecord)>> {
        let reader = reader_identity(store.log_path()).unwrap();
        let fallbacks = std::cell::Cell::new(0);
        let write_failures = std::cell::Cell::new(0);
        records_for_test_reader(
            store,
            Some(cache_directory),
            Some(&reader),
            &fallbacks,
            &write_failures,
        )
        .map(|records| {
            records
                .into_iter()
                .map(|(_, cid, record)| (cid, record))
                .collect()
        })
    }

    /// Reads through the cache as this process's reader and returns how many fallbacks the read
    /// reported, so a test can tell a full verification from a trusted cached prefix.
    #[cfg(feature = "sidecar")]
    fn records_and_fallbacks_for_test_cache(
        store: &SidecarFlowStore,
        cache_directory: &Path,
    ) -> (Vec<(Cid, FlowRecord)>, usize) {
        let reader = reader_identity(store.log_path()).unwrap();
        let fallbacks = std::cell::Cell::new(0);
        let write_failures = std::cell::Cell::new(0);
        let records = records_for_test_reader(
            store,
            Some(cache_directory),
            Some(&reader),
            &fallbacks,
            &write_failures,
        )
        .unwrap()
        .into_iter()
        .map(|(_, cid, record)| (cid, record))
        .collect();
        (records, fallbacks.get())
    }

    #[cfg(feature = "sidecar")]
    fn directory_files(path: &Path) -> std::collections::BTreeMap<std::ffi::OsString, Vec<u8>> {
        std::fs::read_dir(path)
            .unwrap()
            .map(|entry| {
                let entry = entry.unwrap();
                (entry.file_name(), std::fs::read(entry.path()).unwrap())
            })
            .collect()
    }

    #[cfg(feature = "sidecar")]
    #[test]
    fn verified_prefix_cache_directory_resolution_obeys_xdg_and_home_fallback() {
        let absolute_xdg = Path::new("/device/cache");
        let absolute_home = Path::new("/device/home");
        assert_eq!(
            verified_prefix_cache_directory_from(
                Some(absolute_xdg.as_os_str()),
                Some(absolute_home.as_os_str()),
            ),
            Some(absolute_xdg.join(VERIFIED_PREFIX_CACHE_SUBDIRECTORY))
        );
        assert_eq!(
            verified_prefix_cache_directory_from(
                Some(OsStr::new("relative-cache")),
                Some(absolute_home.as_os_str()),
            ),
            Some(
                absolute_home
                    .join(".cache")
                    .join(VERIFIED_PREFIX_CACHE_SUBDIRECTORY)
            )
        );
        assert_eq!(
            verified_prefix_cache_directory_from(
                Some(OsStr::new("relative-cache")),
                Some(OsStr::new("relative-home")),
            ),
            None
        );
        assert_eq!(verified_prefix_cache_directory_from(None, None), None);
    }

    #[cfg(feature = "sidecar")]
    #[test]
    fn verified_prefix_cache_cold_and_warm_reads_leave_log_directory_unchanged() {
        let root = tempfile::tempdir().unwrap();
        let cache_home = tempfile::tempdir().unwrap();
        let cache_directory = cache_home.path().join(VERIFIED_PREFIX_CACHE_SUBDIRECTORY);
        let mut store = SidecarFlowStore::open(root.path()).unwrap();
        store
            .append_batch(vec![
                FlowRecord::Edge(edge("a", "b", 1, "first")),
                FlowRecord::Edge(edge("b", "c", 2, "second")),
            ])
            .unwrap();
        let log_directory = store.log_path().parent().unwrap();
        let before = directory_files(log_directory);
        assert!(!cache_directory.exists(), "reads create no cache directory");

        let cold = records_for_test_cache(&store, &cache_directory).unwrap();
        assert_eq!(directory_files(log_directory), before);
        assert!(cache_directory.exists());
        let warm = records_for_test_cache(&store, &cache_directory).unwrap();
        assert_eq!(warm, cold);
        assert_eq!(directory_files(log_directory), before);
        assert!(!cache_directory.starts_with(root.path()));
    }

    #[cfg(feature = "sidecar")]
    #[test]
    fn verified_prefix_cache_without_a_resolved_directory_verifies_and_writes_nothing() {
        let root = tempfile::tempdir().unwrap();
        let untouched = tempfile::tempdir().unwrap();
        let mut store = SidecarFlowStore::open(root.path()).unwrap();
        store
            .append(FlowRecord::Edge(edge("a", "b", 1, "first")))
            .unwrap();
        let reader = test_reader("no-cache", store.log_path());
        let fallbacks = std::cell::Cell::new(0);
        let write_failures = std::cell::Cell::new(0);

        let records =
            records_for_test_reader(&store, None, Some(&reader), &fallbacks, &write_failures)
                .unwrap();

        assert_eq!(records.len(), 1);
        assert_eq!(fallbacks.get(), 0);
        assert_eq!(write_failures.get(), 0);
        assert!(directory_files(untouched.path()).is_empty());
    }

    #[cfg(feature = "sidecar")]
    #[test]
    fn verified_prefix_cache_file_names_and_entries_are_bound_to_the_canonical_log_path() {
        let first_root = tempfile::tempdir().unwrap();
        let second_root = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let mut first = SidecarFlowStore::open(first_root.path()).unwrap();
        let second = SidecarFlowStore::open(second_root.path()).unwrap();
        first
            .append(FlowRecord::Edge(edge("a", "b", 1, "first")))
            .unwrap();
        let contents = std::fs::read_to_string(first.log_path()).unwrap();
        let first_reader = test_reader("same-reader", first.log_path());
        let second_reader = test_reader("same-reader", second.log_path());
        assert_ne!(
            verified_prefix_cache_path(cache.path(), &first_reader.log_path),
            verified_prefix_cache_path(cache.path(), &second_reader.log_path)
        );

        let collision_path = verified_prefix_cache_path(cache.path(), &first_reader.log_path);
        let foreign = VerifiedPrefixCache {
            reader: second_reader,
            prefix_len: u64::try_from(contents.len()).unwrap(),
            digest: sha2_digest(contents.as_bytes()),
            record_count: 1,
        };
        std::fs::write(
            collision_path,
            serde_json::to_vec(&VerifiedPrefixCacheMap {
                entries: vec![serde_json::to_string(&foreign).unwrap()],
            })
            .unwrap(),
        )
        .unwrap();
        assert!(matches!(
            cached_prefix(cache.path(), &contents, &first_reader),
            CacheLookup::Miss
        ));
    }

    #[cfg(feature = "sidecar")]
    #[test]
    fn verified_prefix_cache_directory_keeps_only_the_64_newest_cache_files() {
        let root = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let mut store = SidecarFlowStore::open(root.path()).unwrap();
        store
            .append(FlowRecord::Edge(edge("a", "b", 1, "first")))
            .unwrap();
        for index in 0_u64..65 {
            let path = cache.path().join(format!("{index:064x}.json"));
            let file = std::fs::File::create(path).unwrap();
            file.set_times(
                std::fs::FileTimes::new()
                    .set_modified(UNIX_EPOCH + std::time::Duration::from_secs(index)),
            )
            .unwrap();
        }
        for name in [
            "notes.json",
            "ABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCD.json",
            "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff.tmp",
        ] {
            std::fs::write(cache.path().join(name), b"keep").unwrap();
        }

        records_for_test_cache(&store, cache.path()).unwrap();

        let matching = std::fs::read_dir(cache.path())
            .unwrap()
            .map(std::result::Result::unwrap)
            .filter(|entry| is_verified_prefix_cache_file(&entry.file_name()))
            .count();
        assert_eq!(matching, VERIFIED_PREFIX_CACHE_FILES);
        assert!(!cache.path().join(format!("{:064x}.json", 0)).exists());
        assert!(!cache.path().join(format!("{:064x}.json", 1)).exists());
        assert!(cache.path().join(format!("{:064x}.json", 2)).exists());
        assert!(cache.path().join("notes.json").exists());
        assert!(cache
            .path()
            .join("ABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCD.json")
            .exists());
        assert!(cache
            .path()
            .join("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff.tmp")
            .exists());
    }

    #[cfg(feature = "sidecar")]
    #[test]
    fn verified_prefix_cache_warm_read_matches_cold_read() {
        let dir = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let mut store = SidecarFlowStore::open(dir.path()).unwrap();
        store
            .append_batch(vec![
                FlowRecord::Edge(edge("a", "b", 1, "first")),
                FlowRecord::Edge(edge("b", "c", 2, "second")),
            ])
            .unwrap();

        let reader = reader_identity(store.log_path()).unwrap();
        let cache_path = verified_prefix_cache_path(cache.path(), &reader.log_path);
        assert!(!cache_path.exists());
        let cold = records_for_test_cache(&store, cache.path()).unwrap();
        assert!(cache_path.exists());
        let warm = records_for_test_cache(&store, cache.path()).unwrap();
        assert_eq!(warm, cold);

        let indexed = store.records_with_offsets().unwrap();
        assert_eq!(
            indexed
                .records
                .into_iter()
                .map(|(_, cid, record)| (cid, record))
                .collect::<Vec<_>>(),
            cold
        );
        assert_eq!(store.transaction().unwrap().records().unwrap(), cold);
    }

    #[cfg(feature = "sidecar")]
    #[test]
    fn verified_prefix_cache_middle_record_tamper_still_refuses_integrity() {
        let dir = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let mut store = SidecarFlowStore::open(dir.path()).unwrap();
        store
            .append_batch(vec![
                FlowRecord::Edge(edge("a", "b", 1, "first")),
                FlowRecord::Edge(edge("b", "c", 2, "second")),
                FlowRecord::Edge(edge("c", "d", 3, "third")),
            ])
            .unwrap();
        records_for_test_cache(&store, cache.path()).unwrap();

        let contents = std::fs::read_to_string(store.log_path()).unwrap();
        let mut lines: Vec<String> = contents.lines().map(str::to_owned).collect();
        let mut middle: SidecarLine = serde_json::from_str(&lines[1]).unwrap();
        let FlowRecord::Edge(edge) = &mut middle.record else {
            panic!("fixture must contain an edge");
        };
        edge.sealed_at = 9;
        lines[1] = serde_json::to_string(&middle).unwrap();
        std::fs::write(store.log_path(), lines.join("\n") + "\n").unwrap();

        let error = records_for_test_cache(&store, cache.path()).unwrap_err();
        assert!(matches!(error, FabricError::Integrity { .. }), "{error:?}");
    }

    #[cfg(feature = "sidecar")]
    #[test]
    fn verified_prefix_cache_truncation_and_same_length_replacement_fall_back() {
        let truncated_dir = tempfile::tempdir().unwrap();
        let truncated_cache = tempfile::tempdir().unwrap();
        let mut truncated_store = SidecarFlowStore::open(truncated_dir.path()).unwrap();
        truncated_store
            .append_batch(vec![
                FlowRecord::Edge(edge("a", "b", 1, "first")),
                FlowRecord::Edge(edge("b", "c", 2, "second")),
            ])
            .unwrap();
        let before = records_for_test_cache(&truncated_store, truncated_cache.path()).unwrap();
        let bytes = std::fs::read(truncated_store.log_path()).unwrap();
        let first_end = bytes.iter().position(|byte| *byte == b'\n').unwrap() + 1;
        std::fs::OpenOptions::new()
            .write(true)
            .open(truncated_store.log_path())
            .unwrap()
            .set_len(u64::try_from(first_end).unwrap())
            .unwrap();
        assert_eq!(
            records_for_test_cache(&truncated_store, truncated_cache.path()).unwrap(),
            before[..1]
        );
        assert_eq!(
            read_cache(&truncated_store, truncated_cache.path()).prefix_len,
            u64::try_from(first_end).unwrap()
        );

        let replaced_dir = tempfile::tempdir().unwrap();
        let replaced_cache = tempfile::tempdir().unwrap();
        let mut replaced_store = SidecarFlowStore::open(replaced_dir.path()).unwrap();
        replaced_store
            .append_batch(vec![
                FlowRecord::Edge(edge("a", "b", 1, "first")),
                FlowRecord::Edge(edge("b", "c", 2, "third_")),
            ])
            .unwrap();
        records_for_test_cache(&replaced_store, replaced_cache.path()).unwrap();
        let original = std::fs::read_to_string(replaced_store.log_path()).unwrap();
        let replacement = original
            .lines()
            .enumerate()
            .map(|(index, line)| {
                let mut parsed: SidecarLine = serde_json::from_str(line).unwrap();
                let FlowRecord::Edge(edge) = &mut parsed.record else {
                    panic!("fixture must contain an edge");
                };
                edge.from = if index == 0 { "x" } else { "y" }.into();
                edge.to = if index == 0 { "y" } else { "z" }.into();
                edge.sealed_at += 3;
                parsed.cid = parsed.record.cid().unwrap().to_string();
                serde_json::to_string(&parsed).unwrap()
            })
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        assert_eq!(replacement.len(), original.len());
        let expected = decode_flow_records(&replacement).unwrap();
        std::fs::write(replaced_store.log_path(), &replacement).unwrap();
        // Same length, same record count and valid cids: only the prefix digest tells this log
        // from the one the cache vouched for, so the lookup must refuse the cached prefix and the
        // read must verify every line.
        let reader = reader_identity(replaced_store.log_path()).unwrap();
        assert!(matches!(
            cached_prefix(replaced_cache.path(), &replacement, &reader),
            CacheLookup::Mismatch("prefix digest mismatch")
        ));
        let (records, fallbacks) =
            records_and_fallbacks_for_test_cache(&replaced_store, replaced_cache.path());
        assert_eq!(records, expected);
        assert_eq!(fallbacks, 1);
    }

    #[cfg(feature = "sidecar")]
    #[test]
    fn verified_prefix_cache_edited_prefix_falls_back_to_full_verification() {
        let dir = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let mut store = SidecarFlowStore::open(dir.path()).unwrap();
        store
            .append_batch(vec![
                FlowRecord::Edge(edge("a", "b", 1, "first")),
                FlowRecord::Edge(edge("b", "c", 2, "second")),
                FlowRecord::Edge(edge("c", "d", 3, "third")),
            ])
            .unwrap();
        records_for_test_cache(&store, cache.path()).unwrap();

        let original = std::fs::read_to_string(store.log_path()).unwrap();
        let mut lines: Vec<String> = original.lines().map(str::to_owned).collect();
        let mut middle: SidecarLine = serde_json::from_str(&lines[1]).unwrap();
        let FlowRecord::Edge(edge) = &mut middle.record else {
            panic!("fixture must contain an edge");
        };
        edge.sealed_at = 7;
        middle.cid = middle.record.cid().unwrap().to_string();
        lines[1] = serde_json::to_string(&middle).unwrap();
        let edited = lines.join("\n") + "\n";
        assert_eq!(edited.len(), original.len());
        let expected = decode_flow_records(&edited).unwrap();
        std::fs::write(store.log_path(), &edited).unwrap();

        // The edit kept the line's length and recomputed its cid, so only the prefix digest tells
        // this log from the one the cache vouched for. The lookup must refuse the cached prefix,
        // and the read must then verify every line.
        let reader = reader_identity(store.log_path()).unwrap();
        assert!(matches!(
            cached_prefix(cache.path(), &edited, &reader),
            CacheLookup::Mismatch("prefix digest mismatch")
        ));
        let (records, fallbacks) = records_and_fallbacks_for_test_cache(&store, cache.path());
        assert_eq!(records, expected);
        assert_eq!(fallbacks, 1);
    }

    #[cfg(feature = "sidecar")]
    #[test]
    fn verified_prefix_cache_corrupt_falls_back_and_foreign_reader_is_a_silent_miss() {
        let dir = tempfile::tempdir().unwrap();
        let cache_directory = tempfile::tempdir().unwrap();
        let mut store = SidecarFlowStore::open(dir.path()).unwrap();
        store
            .append(FlowRecord::Edge(edge("a", "b", 1, "first")))
            .unwrap();
        let expected = records_for_test_cache(&store, cache_directory.path()).unwrap();
        let current = reader_identity(store.log_path()).unwrap();
        let cache_path = verified_prefix_cache_path(cache_directory.path(), &current.log_path);

        std::fs::write(&cache_path, b"{not-json").unwrap();
        assert_eq!(
            records_for_test_cache(&store, cache_directory.path()).unwrap(),
            expected
        );
        let mut foreign = read_cache(&store, cache_directory.path());
        foreign.reader.reader_version = "another-reader-version".into();
        std::fs::write(
            &cache_path,
            serde_json::to_vec(&VerifiedPrefixCacheMap {
                entries: vec![serde_json::to_string(&foreign).unwrap()],
            })
            .unwrap(),
        )
        .unwrap();
        let fallback_count = std::cell::Cell::new(0);
        let write_failure_count = std::cell::Cell::new(0);
        assert_eq!(
            records_for_test_reader(
                &store,
                Some(cache_directory.path()),
                Some(&current),
                &fallback_count,
                &write_failure_count,
            )
            .unwrap()
            .into_iter()
            .map(|(_, cid, record)| (cid, record))
            .collect::<Vec<_>>(),
            expected
        );
        assert_eq!(fallback_count.get(), 0);
        let cache = read_cache_entries(&store, cache_directory.path());
        assert_eq!(cache.len(), 2);
        assert!(cache.iter().any(|entry| entry.reader == foreign.reader));
        assert!(cache.iter().any(|entry| entry.reader == current));
    }

    #[cfg(feature = "sidecar")]
    #[test]
    fn verified_prefix_cache_alternating_readers_keep_their_own_warm_entries() {
        let dir = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let mut store = SidecarFlowStore::open(dir.path()).unwrap();
        store
            .append(FlowRecord::Edge(edge("a", "b", 1, "first")))
            .unwrap();
        let a = test_reader("a", store.log_path());
        let b = test_reader("b", store.log_path());
        let fallbacks = std::cell::Cell::new(0);
        let write_failures = std::cell::Cell::new(0);

        records_for_test_reader(
            &store,
            Some(cache.path()),
            Some(&a),
            &fallbacks,
            &write_failures,
        )
        .unwrap();
        records_for_test_reader(
            &store,
            Some(cache.path()),
            Some(&b),
            &fallbacks,
            &write_failures,
        )
        .unwrap();
        records_for_test_reader(
            &store,
            Some(cache.path()),
            Some(&a),
            &fallbacks,
            &write_failures,
        )
        .unwrap();
        records_for_test_reader(
            &store,
            Some(cache.path()),
            Some(&b),
            &fallbacks,
            &write_failures,
        )
        .unwrap();

        assert_eq!(fallbacks.get(), 0);
        assert_eq!(write_failures.get(), 0);
        let contents = std::fs::read_to_string(store.log_path()).unwrap();
        assert!(matches!(
            cached_prefix(cache.path(), &contents, &a),
            CacheLookup::Hit(_)
        ));
        assert!(matches!(
            cached_prefix(cache.path(), &contents, &b),
            CacheLookup::Hit(_)
        ));
        assert_eq!(read_cache_entries(&store, cache.path()).len(), 2);
    }

    #[cfg(feature = "sidecar")]
    #[test]
    fn verified_prefix_cache_replacing_one_reader_preserves_other_entry_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let cache_directory = tempfile::tempdir().unwrap();
        let mut store = SidecarFlowStore::open(dir.path()).unwrap();
        store
            .append(FlowRecord::Edge(edge("a", "b", 1, "first")))
            .unwrap();
        let a = test_reader("a", store.log_path());
        let b = test_reader("b", store.log_path());
        let fallbacks = std::cell::Cell::new(0);
        let write_failures = std::cell::Cell::new(0);
        records_for_test_reader(
            &store,
            Some(cache_directory.path()),
            Some(&a),
            &fallbacks,
            &write_failures,
        )
        .unwrap();
        records_for_test_reader(
            &store,
            Some(cache_directory.path()),
            Some(&b),
            &fallbacks,
            &write_failures,
        )
        .unwrap();

        let cache_path = verified_prefix_cache_path(cache_directory.path(), &a.log_path);
        let mut cache = read_cache_map(&store, cache_directory.path());
        let b_index = cache
            .entries
            .iter()
            .position(|encoded| {
                serde_json::from_str::<VerifiedPrefixCache>(encoded)
                    .is_ok_and(|entry| entry.reader == b)
            })
            .unwrap();
        let b_entry: VerifiedPrefixCache = serde_json::from_str(&cache.entries[b_index]).unwrap();
        let mut future_shape = serde_json::to_value(b_entry).unwrap();
        future_shape["fieldFromFutureCacheSchema"] = serde_json::json!({"kept": true});
        let b_bytes = format!("  {}  ", serde_json::to_string(&future_shape).unwrap());
        cache.entries[b_index] = b_bytes.clone();
        std::fs::write(&cache_path, serde_json::to_vec(&cache).unwrap()).unwrap();

        store
            .append(FlowRecord::Edge(edge("b", "c", 2, "second")))
            .unwrap();
        records_for_test_reader(
            &store,
            Some(cache_directory.path()),
            Some(&a),
            &fallbacks,
            &write_failures,
        )
        .unwrap();

        assert!(read_cache_map(&store, cache_directory.path())
            .entries
            .iter()
            .any(|encoded| encoded == &b_bytes));
    }

    #[cfg(feature = "sidecar")]
    #[test]
    fn verified_prefix_cache_never_trusts_another_reader_identity() {
        let dir = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let store = SidecarFlowStore::open(dir.path()).unwrap();
        let future_record = FlowRecord::Edge(edge("a", "b", 1, "future"));
        let mut raw = serde_json::to_value(SidecarLine {
            cid: future_record.cid().unwrap().to_string(),
            record: future_record,
        })
        .unwrap();
        raw["record"]["fieldFromTheFuture"] = serde_json::json!("x");
        raw["cid"] = serde_json::json!(atom_cid(&"future bytes".to_string()).unwrap().to_string());
        let contents = serde_json::to_string(&raw).unwrap() + "\n";
        std::fs::write(store.log_path(), &contents).unwrap();
        let a = test_reader("new-reader-a", store.log_path());
        let b = test_reader("old-reader-b", store.log_path());
        assert!(
            write_verified_prefix_cache(store.log_path(), cache.path(), &contents, 1, &a).unwrap()
        );

        let fallbacks = std::cell::Cell::new(0);
        let write_failures = std::cell::Cell::new(0);
        let error = records_for_test_reader(
            &store,
            Some(cache.path()),
            Some(&b),
            &fallbacks,
            &write_failures,
        )
        .unwrap_err();

        assert!(
            matches!(error, FabricError::NewerWriter { .. }),
            "{error:?}"
        );
        assert_eq!(fallbacks.get(), 0);
        assert_eq!(read_cache_entries(&store, cache.path()).len(), 1);
        assert_eq!(read_cache_entries(&store, cache.path())[0].reader, a);
    }

    #[cfg(feature = "sidecar")]
    #[test]
    fn verified_prefix_cache_keeps_four_readers_and_evicts_the_oldest_write() {
        let dir = tempfile::tempdir().unwrap();
        let cache_directory = tempfile::tempdir().unwrap();
        let mut store = SidecarFlowStore::open(dir.path()).unwrap();
        store
            .append(FlowRecord::Edge(edge("a", "b", 1, "first")))
            .unwrap();
        let readers: Vec<_> = (0..5)
            .map(|index| test_reader(&format!("reader-{index}"), store.log_path()))
            .collect();
        let fallbacks = std::cell::Cell::new(0);
        let write_failures = std::cell::Cell::new(0);
        for reader in &readers {
            records_for_test_reader(
                &store,
                Some(cache_directory.path()),
                Some(reader),
                &fallbacks,
                &write_failures,
            )
            .unwrap();
        }

        let cache = read_cache_entries(&store, cache_directory.path());
        assert_eq!(cache.len(), VERIFIED_PREFIX_CACHE_READERS);
        assert!(!cache.iter().any(|entry| entry.reader == readers[0]));
        assert_eq!(
            cache
                .iter()
                .map(|entry| entry.reader.clone())
                .collect::<Vec<_>>(),
            readers[1..]
        );
    }

    #[cfg(feature = "sidecar")]
    #[test]
    fn verified_prefix_cache_busy_writer_is_skipped_without_losing_records() {
        let dir = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let mut store = SidecarFlowStore::open(dir.path()).unwrap();
        store
            .append(FlowRecord::Edge(edge("a", "b", 1, "first")))
            .unwrap();
        let contents = crate::sidecar::flow_contents(store.log_path()).unwrap();
        let expected = decode_flow_records(&contents).unwrap();
        let reader = test_reader("busy", store.log_path());
        let fallbacks = std::cell::Cell::new(0);
        let write_failures = std::cell::Cell::new(0);
        let _held = crate::sidecar::LockedLog::exclusive(store.log_path()).unwrap();

        let records = decode_flow_records_with_reader(
            store.log_path(),
            &contents,
            Some(cache.path()),
            Some(&reader),
            |_| fallbacks.set(fallbacks.get() + 1),
            |_| write_failures.set(write_failures.get() + 1),
        )
        .unwrap();

        assert_eq!(
            records
                .into_iter()
                .map(|(_, cid, record)| (cid, record))
                .collect::<Vec<_>>(),
            expected
        );
        assert_eq!(fallbacks.get(), 0);
        assert_eq!(write_failures.get(), 0);
        assert!(!verified_prefix_cache_path(cache.path(), &reader.log_path).exists());
    }

    #[cfg(feature = "sidecar")]
    #[test]
    fn verified_prefix_cache_missing_entry_emits_no_fallback() {
        let dir = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let mut store = SidecarFlowStore::open(dir.path()).unwrap();
        store
            .append(FlowRecord::Edge(edge("a", "b", 1, "first")))
            .unwrap();
        let reader = test_reader("missing", store.log_path());
        let fallbacks = std::cell::Cell::new(0);
        let write_failures = std::cell::Cell::new(0);

        records_for_test_reader(
            &store,
            Some(cache.path()),
            Some(&reader),
            &fallbacks,
            &write_failures,
        )
        .unwrap();

        assert_eq!(fallbacks.get(), 0);
        assert_eq!(write_failures.get(), 0);
    }

    #[cfg(feature = "sidecar")]
    #[test]
    fn verified_prefix_cache_unreadable_executable_identity_bypasses_cache_io() {
        let dir = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let mut store = SidecarFlowStore::open(dir.path()).unwrap();
        store
            .append(FlowRecord::Edge(edge("a", "b", 1, "first")))
            .unwrap();
        let reader = test_reader("unreadable", store.log_path());
        let cache_path = verified_prefix_cache_path(cache.path(), &reader.log_path);
        std::fs::create_dir_all(cache.path()).unwrap();
        let sentinel = b"existing cache bytes are not read or replaced";
        std::fs::write(&cache_path, sentinel).unwrap();
        let fallbacks = std::cell::Cell::new(0);
        let write_failures = std::cell::Cell::new(0);

        let records = records_for_test_reader(
            &store,
            Some(cache.path()),
            None,
            &fallbacks,
            &write_failures,
        )
        .unwrap();

        assert_eq!(records.len(), 1);
        assert_eq!(fallbacks.get(), 0);
        assert_eq!(write_failures.get(), 0);
        assert_eq!(std::fs::read(cache_path).unwrap(), sentinel);
    }

    #[cfg(feature = "sidecar")]
    #[test]
    fn verified_prefix_cache_extends_after_append() {
        let dir = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let mut store = SidecarFlowStore::open(dir.path()).unwrap();
        store
            .append(FlowRecord::Edge(edge("a", "b", 1, "first")))
            .unwrap();
        records_for_test_cache(&store, cache.path()).unwrap();
        let before = read_cache(&store, cache.path());

        store
            .append(FlowRecord::Edge(edge("b", "c", 2, "second")))
            .unwrap();
        assert_eq!(
            records_for_test_cache(&store, cache.path()).unwrap().len(),
            2
        );
        let after = read_cache(&store, cache.path());
        assert!(after.prefix_len > before.prefix_len);
        assert_eq!(after.record_count, 2);
        assert_eq!(
            after.prefix_len,
            std::fs::metadata(store.log_path()).unwrap().len()
        );
    }

    #[cfg(feature = "sidecar")]
    #[test]
    fn verified_prefix_cache_tail_preserves_newer_writer_error() {
        let dir = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let mut store = SidecarFlowStore::open(dir.path()).unwrap();
        store
            .append(FlowRecord::Edge(edge("a", "b", 1, "first")))
            .unwrap();
        records_for_test_cache(&store, cache.path()).unwrap();

        let future_record = FlowRecord::Edge(edge("b", "c", 2, "second"));
        let mut raw = serde_json::to_value(SidecarLine {
            cid: future_record.cid().unwrap().to_string(),
            record: future_record,
        })
        .unwrap();
        raw["record"]["fieldFromTheFuture"] = serde_json::json!("x");
        raw["cid"] = serde_json::json!(atom_cid(&"a different future record".to_string())
            .unwrap()
            .to_string());
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(store.log_path())
            .unwrap();
        writeln!(file, "{}", serde_json::to_string(&raw).unwrap()).unwrap();

        let error = records_for_test_cache(&store, cache.path()).unwrap_err();
        assert!(
            matches!(error, FabricError::NewerWriter { .. }),
            "{error:?}"
        );
    }

    #[cfg(feature = "sidecar")]
    #[test]
    #[ignore = "manual release benchmark against EPR_FLOW_BENCH_LOG"]
    fn benchmark_verified_prefix_cache_cold_and_warm() {
        let path = PathBuf::from(
            std::env::var("EPR_FLOW_BENCH_LOG")
                .expect("set EPR_FLOW_BENCH_LOG to the disposable flows-bench.jsonl copy"),
        );
        assert_eq!(
            path.file_name().and_then(|name| name.to_str()),
            Some("flows-bench.jsonl"),
            "the benchmark refuses to run against the live flows.jsonl"
        );
        let store = SidecarFlowStore { log_path: path };
        let cache_directory = verified_prefix_cache_directory()
            .expect("benchmark needs an absolute XDG_CACHE_HOME or HOME");
        let reader = reader_identity(store.log_path()).unwrap();
        let cache_path = verified_prefix_cache_path(&cache_directory, &reader.log_path);
        let mut cold = Vec::new();
        let mut records = 0;
        for _ in 0..3 {
            match std::fs::remove_file(&cache_path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => panic!("remove disposable benchmark cache: {error}"),
            }
            let started = std::time::Instant::now();
            records = records_for_test_cache(&store, &cache_directory)
                .unwrap()
                .len();
            cold.push(started.elapsed().as_secs_f64() * 1_000.0);
        }
        let mut warm = Vec::new();
        for _ in 0..5 {
            let started = std::time::Instant::now();
            assert_eq!(
                records_for_test_cache(&store, &cache_directory)
                    .unwrap()
                    .len(),
                records
            );
            warm.push(started.elapsed().as_secs_f64() * 1_000.0);
        }
        eprintln!(
            "flow-cache-bench records={records} bytes={} cold_ms={cold:?} warm_ms={warm:?}",
            std::fs::metadata(store.log_path()).unwrap().len()
        );
    }

    /// `DepEdge.to_resource` is additive: sidecar lines written by the `epr` binary BEFORE the
    /// field existed (a cite-seal edge and a compiler edge, produced by `epr flow seal` into a
    /// scratch root and copied verbatim into the fixture) decode with `to_resource: None`,
    /// re-verify against their stored CID, and re-serialize to the identical bytes.
    #[cfg(feature = "sidecar")]
    #[test]
    fn pre_to_resource_sidecar_lines_round_trip_byte_identical() {
        let fixture = include_str!("../tests/fixtures/depedge-before-to-resource.flows.jsonl");
        let mut seen = 0;
        for line in fixture.lines().filter(|l| !l.trim().is_empty()) {
            let parsed: SidecarLine = serde_json::from_str(line).expect("old line decodes");
            let FlowRecord::Edge(edge) = &parsed.record else {
                panic!("fixture holds edges only");
            };
            assert_eq!(edge.to_resource, None);
            assert_eq!(parsed.record.cid().unwrap().to_string(), parsed.cid);
            assert_eq!(serde_json::to_string(&parsed).unwrap(), line);
            seen += 1;
        }
        assert_eq!(seen, 2);
    }

    /// Forward compatibility: a line carrying a field this reader does not know (what an older
    /// binary sees in a `toResource` edge) refuses loudly as `NewerWriter`, naming the field and
    /// this reader's version; a line that is merely tampered stays an `Integrity` error.
    #[cfg(feature = "sidecar")]
    #[test]
    fn a_line_from_a_newer_writer_refuses_naming_the_field_and_the_version() {
        let fixture = include_str!("../tests/fixtures/depedge-before-to-resource.flows.jsonl");
        let line = fixture.lines().next().unwrap();
        let mut raw: serde_json::Value = serde_json::from_str(line).unwrap();
        // The writer hashed a record with a field this reader cannot see.
        raw["record"]["fieldFromTheFuture"] = serde_json::json!("x");
        raw["cid"] = serde_json::json!(atom_cid(&"a different record".to_string())
            .unwrap()
            .to_string());
        let newer = serde_json::to_string(&raw).unwrap();
        let mut tampered: serde_json::Value = serde_json::from_str(line).unwrap();
        tampered["cid"] = raw["cid"].clone();
        let tampered = serde_json::to_string(&tampered).unwrap();

        for (text, expect_newer) in [(newer, true), (tampered, false)] {
            let dir = tempfile::tempdir().unwrap();
            let store = SidecarFlowStore::open(dir.path()).unwrap();
            std::fs::write(store.log_path(), format!("{text}\n")).unwrap();
            let err = store.records().unwrap_err();
            if expect_newer {
                let message = err.to_string();
                assert!(matches!(err, FabricError::NewerWriter { .. }), "{err:?}");
                assert!(message.contains("fieldFromTheFuture"), "{message}");
                assert!(message.contains(env!("CARGO_PKG_VERSION")), "{message}");
            } else {
                assert!(matches!(err, FabricError::Integrity { .. }), "{err:?}");
            }
        }
    }

    #[cfg(feature = "sidecar")]
    #[test]
    fn an_empty_retry_batch_checks_the_visible_tail_before_acknowledging() {
        let dir = tempfile::tempdir().unwrap();
        let store = SidecarFlowStore::open(dir.path()).unwrap();
        std::fs::write(store.log_path(), b"visible-but-not-complete").unwrap();
        let before = std::fs::read(store.log_path()).unwrap();

        let error = store
            .transaction()
            .unwrap()
            .append_batch(Vec::new())
            .expect_err("a dedup retry cannot acknowledge a tail that was never synced whole");

        assert!(matches!(error, FabricError::Decode(_)), "{error:?}");
        assert_eq!(std::fs::read(store.log_path()).unwrap(), before);
    }

    #[cfg(feature = "sidecar")]
    #[test]
    fn indexed_records_name_line_starts_and_the_captured_prefix_end() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = SidecarFlowStore::open(dir.path()).unwrap();
        let first = FlowRecord::Edge(edge("a", "b", 1, "first"));
        let second = FlowRecord::Edge(edge("b", "c", 2, "second"));
        store.append_batch(vec![first, second]).unwrap();

        let bytes = std::fs::read(store.log_path()).unwrap();
        let first_end = bytes.iter().position(|byte| *byte == b'\n').unwrap() + 1;
        let prefix = store.records_with_offsets().unwrap();

        assert_eq!(prefix.records.len(), 2);
        assert_eq!(prefix.records[0].0, 0);
        assert_eq!(prefix.records[1].0, u64::try_from(first_end).unwrap());
        assert_eq!(prefix.end_offset, u64::try_from(bytes.len()).unwrap());
        assert_eq!(store.end_offset().unwrap(), prefix.end_offset);
    }

    #[test]
    fn to_resource_is_carried_and_addresses_a_distinct_record() {
        let base = DepEdge::new(
            "elohim/epr".into(),
            "crates.io:serde@1.0.228".into(),
            None,
            Governor::Compiler("elohim".into()),
            None,
            agent("claude"),
            1,
            None,
        )
        .unwrap();
        let resource = elohim_epr::cid::compute_raw_cid(b"serde-1.0.228.crate");
        let named = base.clone().with_to_resource(resource);
        let json = serde_json::to_string(&named).unwrap();
        assert!(json.contains("\"toResource\""));
        assert!(!serde_json::to_string(&base).unwrap().contains("toResource"));
        let back: DepEdge = serde_json::from_str(&json).unwrap();
        assert_eq!(back, named);
        assert_ne!(atom_cid(&named).unwrap(), atom_cid(&base).unwrap());
    }

    #[test]
    fn edges_round_trips_append_records_and_edges() {
        let mut store = MemoryFlowStore::new();
        let e = edge("app/foo.ts", "spec/bar.md", 100, "bar-v1");
        let cid = store.append(FlowRecord::Edge(e.clone())).unwrap();

        let records = store.records().unwrap();
        assert_eq!(records.len(), 1);
        assert!(matches!(&records[0].1, FlowRecord::Edge(got) if got == &e));

        let edges = store.edges().unwrap();
        assert_eq!(edges, vec![(cid, e)]);
    }

    #[test]
    fn edges_latest_wins_after_a_superseding_reseal() {
        let mut store = MemoryFlowStore::new();
        let older = edge("app/foo.ts", "spec/bar.md", 100, "bar-v1");
        let newer = edge("app/foo.ts", "spec/bar.md", 200, "bar-v2");
        store.append(FlowRecord::Edge(older)).unwrap();
        let newer_cid = store.append(FlowRecord::Edge(newer.clone())).unwrap();

        let edges = store.edges().unwrap();
        assert_eq!(edges, vec![(newer_cid, newer)]);
    }

    #[test]
    fn edges_tie_at_equal_sealed_at_is_won_by_later_append_order() {
        let mut store = MemoryFlowStore::new();
        let first = edge("app/foo.ts", "spec/bar.md", 100, "bar-v1");
        let second = edge("app/foo.ts", "spec/bar.md", 100, "bar-v2");
        store.append(FlowRecord::Edge(first)).unwrap();
        let second_cid = store.append(FlowRecord::Edge(second.clone())).unwrap();

        let edges = store.edges().unwrap();
        assert_eq!(edges, vec![(second_cid, second)]);
    }

    #[test]
    fn edges_a_lower_sealed_at_appended_later_does_not_win() {
        let mut store = MemoryFlowStore::new();
        let higher = edge("app/foo.ts", "spec/bar.md", 200, "bar-v2");
        let lower_but_later_append = edge("app/foo.ts", "spec/bar.md", 50, "bar-v0");
        let higher_cid = store.append(FlowRecord::Edge(higher.clone())).unwrap();
        store
            .append(FlowRecord::Edge(lower_but_later_append))
            .unwrap();

        let edges = store.edges().unwrap();
        assert_eq!(edges, vec![(higher_cid, higher)]);
    }

    #[test]
    fn edges_skips_invalid_records_without_erroring_the_read() {
        let mut store = MemoryFlowStore::new();
        let valid = edge("app/foo.ts", "spec/bar.md", 100, "bar-v1");
        let valid_cid = store.append(FlowRecord::Edge(valid.clone())).unwrap();

        // Hand-construct an invalid edge via the struct literal (fields are `pub`), bypassing
        // `DepEdge::new`'s invariant check entirely — CiteSeal governor with no `sealed_cid`,
        // exactly the invariant `validate()` rejects.
        let invalid = DepEdge {
            from: "app/other.ts".into(),
            to: "spec/other.md".into(),
            to_resource: None,
            desc: None,
            governor: Governor::CiteSeal,
            sealed_cid: None,
            sealed_by: agent("claude"),
            sealed_at: 1,
            status: None,
        };
        assert!(
            invalid.validate().is_err(),
            "fixture must actually violate the invariant"
        );
        store.append(FlowRecord::Edge(invalid)).unwrap();

        let edges = store.edges().unwrap();
        assert_eq!(
            edges,
            vec![(valid_cid, valid)],
            "the invalid record is skipped, not surfaced as a read error"
        );
    }

    #[test]
    fn edges_filters_out_non_edge_records_and_keeps_distinct_slots() {
        let mut store = MemoryFlowStore::new();
        let e1 = edge("app/foo.ts", "spec/bar.md", 1, "bar-v1");
        let e2 = edge("app/baz.ts", "spec/qux.md", 1, "qux-v1");
        let e1_cid = store.append(FlowRecord::Edge(e1.clone())).unwrap();
        let e2_cid = store.append(FlowRecord::Edge(e2.clone())).unwrap();
        store
            .append(FlowRecord::Process(Process {
                spec: crate::model::PinnedRef {
                    id: "elohim-dev-pipeline".into(),
                    version: 1,
                },
                in_scope_of: atom_cid(&"epic".to_string()).unwrap(),
                inputs: vec![],
                outputs: vec![],
                classified_as: vec![],
            }))
            .unwrap();

        let edges = store.edges().unwrap();
        assert_eq!(edges.len(), 2);
        assert!(edges.contains(&(e1_cid, e1)));
        assert!(edges.contains(&(e2_cid, e2)));
    }
}
