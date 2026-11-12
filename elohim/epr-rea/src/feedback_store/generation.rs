//! Two-slot, bounded replacement transactions. State activation is the sole commit point.
//! Every acknowledged generation is immutable. Recovery verifies the active hash before
//! retiring anything; uncommitted partial replacements are discarded only with a valid journal.
use super::{decode, invalid, retention, Entry, FabricError, FeedbackStore, LockedLog, Result};
use crate::{atom_cid, feedback_capacity::Capacity, store::decode_flow_records};
use chrono::Utc;
use cid::Cid;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    collections::BTreeSet,
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

const SENTINEL: &str = "feedback-format-v2: legacy writers must stop; use epr feedback\n";
// Existing algedonic state has at most two open keys; fixed identity strings and timestamps
// fit in this conservative transaction reservation, including its temporary replacement.
const SIGNAL_RESERVE: u64 = 16_384;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Checkpoint {
    version: u32,
    cid: String,
    active: u8,
    digest: String,
    bytes: u64,
    rejected_capture: u64,
    receipts: Vec<serde_json::Value>,
}
impl Checkpoint {
    fn address(&self) -> Result<String> {
        let mut value =
            serde_json::to_value(self).map_err(|e| FabricError::Encode(e.to_string()))?;
        value
            .as_object_mut()
            .ok_or_else(|| FabricError::Encode("checkpoint is not an object".into()))?
            .remove("cid");
        Ok(atom_cid(&value)?.to_string())
    }
    fn verify(&self) -> Result<()> {
        if self.version != 2 || self.active > 1 || self.cid != self.address()? {
            return invalid("unsupported or corrupt feedback checkpoint");
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Transaction {
    version: u32,
    old: Option<Checkpoint>,
    next: Checkpoint,
    legacy_digest: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapacityReport {
    pub policy: Capacity,
    pub retained_bytes: u64,
    pub total_managed_file_bytes: u64,
    pub control_bytes: u64,
    pub reclaimable_bytes: u64,
    pub protected_bytes: u64,
    pub selected_bytes: u64,
    pub rejected_capture: u64,
    pub at_capacity: bool,
    pub observation: Option<crate::AlgedonicEvidence>,
    pub waiting: Vec<String>,
    pub recent_receipts: Vec<serde_json::Value>,
    pub signal_state: Option<serde_json::Value>,
    pub history: &'static str,
}

fn dir(store: &FeedbackStore) -> PathBuf {
    store.root.join(".eprfs/status/feedback")
}
fn slot(d: &Path, active: u8) -> PathBuf {
    d.join(format!("generation-{active}.jsonl"))
}
fn capacity(store: &FeedbackStore) -> Result<&Capacity> {
    store.policy.capacity.as_ref().ok_or_else(|| {
        FabricError::InvalidFeedback("version-pinned capacity binding required".into())
    })
}
fn json_bytes<T: Serialize>(v: &T) -> Result<Vec<u8>> {
    serde_json::to_vec(v).map_err(|e| FabricError::Encode(e.to_string()))
}
fn parse<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    serde_json::from_slice(bytes).map_err(|e| FabricError::Decode(e.to_string()))
}
fn digest(bytes: &[u8]) -> Result<String> {
    Ok(atom_cid(&bytes)?.to_string())
}
fn write_sync(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut f = OpenOptions::new().create_new(true).write(true).open(path)?;
    #[cfg(test)]
    if FAIL_AT.with(|s| {
        s.borrow().as_deref()
            == Some(&format!(
                "partial:{}",
                path.file_name().unwrap().to_string_lossy()
            ))
    }) {
        f.write_all(&bytes[..bytes.len() / 2])?;
        f.sync_all()?;
        return Err(std::io::Error::from(std::io::ErrorKind::StorageFull).into());
    }
    f.write_all(bytes)?;
    f.sync_all()?;
    Ok(())
}
fn sync(d: &Path) -> Result<()> {
    File::open(d)?.sync_all()?;
    Ok(())
}
fn remove(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}
fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>> {
    let meta = fs::symlink_metadata(path)?;
    if !meta.is_file() || meta.len() > limit {
        return invalid("feedback file is not regular or exceeds declared allowance");
    }
    Ok(fs::read(path)?)
}
fn checkpoint(store: &FeedbackStore) -> Result<Checkpoint> {
    let c = capacity(store)?;
    if !dir(store).join("state.json").exists() {
        return invalid("capacity format not initialized; run migrate --legacy-writers-quiesced after stopping old writers");
    }
    let state: Checkpoint = parse(&read_bounded(
        &dir(store).join("state.json"),
        c.parameters.control_bytes,
    )?)?;
    state.verify()?;
    Ok(state)
}
fn contents(store: &FeedbackStore, state: &Checkpoint) -> Result<String> {
    let bytes = read_bounded(
        &slot(&dir(store), state.active),
        capacity(store)?.parameters.active_bytes,
    )?;
    if state.bytes != bytes.len() as u64 || state.digest != digest(&bytes)? {
        return invalid("feedback generation integrity mismatch; no reclamation permitted");
    }
    let text = String::from_utf8(bytes).map_err(|e| FabricError::Decode(e.to_string()))?;
    if !text.is_empty() && !text.ends_with('\n') {
        return invalid("unterminated feedback generation");
    }
    decode(&text)?;
    Ok(text)
}
pub(super) fn read(store: &FeedbackStore) -> Result<Vec<Entry>> {
    decode(&contents(store, &checkpoint(store)?)?)
}

fn accounting(store: &FeedbackStore) -> Result<(u64, u64)> {
    let c = capacity(store)?;
    let mut total = 0u64;
    let mut control = 0u64;
    let d = dir(store);
    if d.exists() {
        for file in fs::read_dir(&d)? {
            let file = file?;
            let name = file.file_name();
            let name = name.to_string_lossy();
            let m = fs::symlink_metadata(file.path())?;
            if !m.is_file() {
                return invalid("unknown feedback-owned filesystem entry; accounting unmeasured");
            }
            if !([
                "generation-0.jsonl",
                "generation-1.jsonl",
                "state.json",
                "state.next",
                "transaction.json",
                "transaction.next",
                "legacy.next",
                "signal.json",
                "signal.json.lock",
            ]
            .contains(&name.as_ref())
                || (name.starts_with("signal.json.") && name.ends_with(".tmp")))
            {
                return invalid("unknown feedback-owned file; explicit review required");
            }
            if name == "signal.json" {
                let value: serde_json::Value =
                    parse(&read_bounded(&file.path(), SIGNAL_RESERVE / 2)?)?;
                if value["bound_ref"].as_str().is_none() || !value["open"].is_array() {
                    return invalid("capacity signal state corrupt; measurement unknown");
                }
            }
            total = total
                .checked_add(m.len())
                .ok_or_else(|| FabricError::InvalidFeedback("accounting overflow".into()))?;
            if !name.starts_with("generation-") {
                control += m.len();
            }
        }
    }
    for name in ["feedback.lock", "feedback.jsonl"] {
        let path = store.root.join(".eprfs/status").join(name);
        match fs::symlink_metadata(path) {
            Ok(m) if m.is_file() => {
                total += m.len();
                if name == "feedback.lock" || m.len() == SENTINEL.len() as u64 {
                    control += m.len();
                }
            }
            Ok(_) => return invalid("feedback legacy/control path is not a regular file"),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
    }
    if total > c.parameters.total_bytes || control > c.parameters.control_bytes {
        return invalid(
            "managed feedback footprint exceeds resolved capacity; no implicit enlargement",
        );
    }
    Ok((total, control))
}
fn flow_refs(store: &FeedbackStore) -> Result<BTreeSet<Cid>> {
    let path = store.root.join(".eprfs/status/flows.jsonl");
    let log = LockedLog::shared(&path).map_err(|e| {
        FabricError::InvalidFeedback(format!("native reference information unavailable: {e}"))
    })?;
    let records = decode_flow_records(&log.contents()?)?;
    let mut refs = BTreeSet::new();
    for (cid, record) in records {
        refs.insert(cid);
        retention::references(
            &serde_json::to_value(record).map_err(|e| FabricError::Encode(e.to_string()))?,
            &mut refs,
        );
    }
    Ok(refs)
}
fn selection(
    store: &FeedbackStore,
    text: &str,
    incoming: &BTreeSet<Cid>,
    reserved_bytes: u64,
) -> Result<retention::Selection> {
    retention::select(
        &decode(text)?,
        &text.split_inclusive('\n').collect::<Vec<_>>(),
        capacity(store)?,
        &flow_refs(store)?,
        Utc::now(),
        incoming,
        reserved_bytes,
    )
}

impl FeedbackStore {
    /// Read-only: pending transitions are reported, never retired by inspection.
    pub fn capacity(&self) -> Result<CapacityReport> {
        let _guard = self.coordinate(false)?;
        report(self)
    }
    pub fn reclaim(&self, dry_run: bool) -> Result<CapacityReport> {
        if dry_run {
            return self.capacity();
        }
        let _guard = self.coordinate(true)?;
        recover(self)?;
        observe(self)?;
        let state = checkpoint(self)?;
        let text = contents(self, &state)?;
        let chosen = selection(self, &text, &BTreeSet::new(), 0)?;
        if !chosen.selected.is_empty() {
            let kept: String = text
                .split_inclusive('\n')
                .enumerate()
                .filter(|(i, _)| !chosen.selected.contains(i))
                .map(|(_, s)| s)
                .collect();
            let mut next = state.clone();
            receipt(self, &mut next, &text, &kept, chosen.selected.len())?;
            commit(self, Some(state), next, kept.as_bytes(), None)?;
        }
        observe(self)?;
        report(self)
    }
    /// Explicit format cutover. Caller must stop old binaries; a poison-pill legacy log makes
    /// their next append refuse. Old atoms are copied byte-for-byte, then the old copy is removed.
    pub fn migrate(&self, legacy_writers_quiesced: bool) -> Result<CapacityReport> {
        if !legacy_writers_quiesced {
            return invalid("migration requires --legacy-writers-quiesced");
        }
        let _guard = self.coordinate(true)?;
        let c = capacity(self)?;
        if dir(self).join("state.json").exists() {
            recover(self)?;
            return report(self);
        }
        fs::create_dir_all(dir(self))?;
        recover(self)?;
        let legacy = crate::sidecar::open_log(&self.root, "feedback.jsonl")?;
        let log = LockedLog::exclusive(&legacy)?;
        let text = log.contents_bounded(c.parameters.active_bytes)?;
        decode(&text)?;
        accounting(self)?;
        let references = self.root.join(".eprfs/status/flows.jsonl");
        if !references.exists() {
            if !text.is_empty() {
                return invalid("legacy evidence has no native reference log; restore reference information before migration");
            }
            // A new, empty feedback store has no retained evidence to reference. Establish
            // its empty native log once; absence after cutover is unknown, never an empty set.
            crate::sidecar::open_log(&self.root, "flows.jsonl")?;
            File::open(&references)?.sync_all()?;
            sync(&self.root.join(".eprfs/status"))?;
        }
        flow_refs(self)?;
        let state = Checkpoint {
            version: 2,
            cid: String::new(),
            active: 0,
            digest: String::new(),
            bytes: 0,
            rejected_capture: 0,
            receipts: vec![],
        };
        commit(
            self,
            None,
            state,
            text.as_bytes(),
            Some(digest(text.as_bytes())?),
        )?;
        drop(log);
        report(self)
    }
}

fn report(store: &FeedbackStore) -> Result<CapacityReport> {
    let state = checkpoint(store)?;
    let text = contents(store, &state)?;
    let (total, control) = accounting(store)?;
    let chosen = selection(store, &text, &BTreeSet::new(), 0)?;
    let c = capacity(store)?;
    let mut waiting = chosen.waiting;
    if dir(store).join("transaction.json").exists() {
        waiting.push("interrupted transaction awaits next mutating command".into());
    }
    if state.bytes > c.target() && chosen.selected_bytes == 0 {
        waiting.push("insufficient eligible evidence; pressure retained".into());
    }
    Ok(CapacityReport {
        policy: c.clone(), retained_bytes: state.bytes, total_managed_file_bytes: total,
        control_bytes: control, reclaimable_bytes: chosen.reclaimable_bytes, protected_bytes: chosen.protected_bytes,
        selected_bytes: chosen.selected_bytes, rejected_capture: state.rejected_capture,
        at_capacity: state.bytes >= c.parameters.active_bytes,
        observation: crate::fold::level_evidence(&c.policy_cid.parse().map_err(|_| FabricError::InvalidFeedback("invalid policy CID".into()))?, &c.bound()?, state.bytes as f64),
        waiting, recent_receipts: state.receipts,
        signal_state: if dir(store).join("signal.json").exists() {
            Some(parse(&read_bounded(&dir(store).join("signal.json"), SIGNAL_RESERVE/2)?)?)
        } else { None },
        history: "Deduplication and exact lookup cover retained records only. Reclaimed records are permanently unknown; receipts are a bounded recent window, not a tombstone ledger.",
    })
}

fn receipt(
    store: &FeedbackStore,
    state: &mut Checkpoint,
    before: &str,
    after: &str,
    count: usize,
) -> Result<()> {
    let c = capacity(store)?;
    state.receipts.push(
        json!({"policyCid":c.policy_cid,"at":Utc::now().to_rfc3339(),
        "beforeBytes":before.len(),"afterBytes":after.len(),"removedRecords":count,
        "beforeDigest":digest(before.as_bytes())?,"afterDigest":digest(after.as_bytes())?}),
    );
    let drop = state
        .receipts
        .len()
        .saturating_sub(c.parameters.recent_receipts);
    state.receipts.drain(..drop);
    Ok(())
}

pub(super) fn append(store: &FeedbackStore, entry: Entry, line: String) -> Result<Cid> {
    if !dir(store).join("state.json").exists() {
        return invalid(
            "feedback capacity format requires explicit migrate --legacy-writers-quiesced",
        );
    }
    recover(store)?;
    let state = checkpoint(store)?;
    let text = contents(store, &state)?;
    let entries = decode(&text)?;
    for old in &entries {
        if old.cid == entry.cid {
            return Ok(old.cid);
        }
        if let (super::Feedback::Occurrence(a), super::Feedback::Occurrence(b)) =
            (&old.record, &entry.record)
        {
            if a.source == b.source {
                let mut b = b.clone();
                b.observed_at.clone_from(&a.observed_at);
                if *a == b {
                    if entry.protected == Some(true) && old.protected != Some(true) {
                        return invalid("retry cannot change retained protection; author a protection evidence link");
                    }
                    return Ok(old.cid);
                }
                return invalid("source identity already retained with different evidence");
            }
        }
    }
    super::validate_links(&entries, &entry.record)?;
    let c = capacity(store)?;
    let mut next = state.clone();
    if line.len() as u64 + 1 > c.parameters.record_bytes {
        next.rejected_capture = next.rejected_capture.saturating_add(1);
        commit(store, Some(state), next, text.as_bytes(), None)?;
        return invalid("feedback record retention limit; retained evidence preserved");
    }
    let mut kept = text.clone();
    let mut incoming = BTreeSet::new();
    retention::references(
        &serde_json::to_value(&entry.record).map_err(|e| FabricError::Encode(e.to_string()))?,
        &mut incoming,
    );
    if c.bound()?.approached_by(text.len() as f64)
        || text.len() as u64 + line.len() as u64 + 1 > c.parameters.active_bytes
    {
        let chosen = selection(store, &text, &incoming, line.len() as u64 + 1)?;
        if !chosen.selected.is_empty() {
            kept = text
                .split_inclusive('\n')
                .enumerate()
                .filter(|(i, _)| !chosen.selected.contains(i))
                .map(|(_, s)| s)
                .collect();
            receipt(store, &mut next, &text, &kept, chosen.selected.len())?;
        }
    }
    // Exactly fitting writes remain admissible even though Bound signals at capacity.
    if kept.len() as u64 + line.len() as u64 + 1 > c.parameters.active_bytes {
        next.rejected_capture = next.rejected_capture.saturating_add(1);
        commit(store, Some(state), next, kept.as_bytes(), None)?;
        observe(store)?;
        return invalid("feedback store retention limit; protected evidence preserved");
    }
    kept.push_str(&line);
    kept.push('\n');
    decode(&kept)?;
    commit(store, Some(state), next, kept.as_bytes(), None)?;
    // Capture has committed. Signal failure is visible at capacity inspection, but must not
    // turn an acknowledged producer result into failure or recursively capture itself.
    if let Err(e) = observe(store) {
        eprintln!("feedback capacity signal unmeasured: {e}");
    }
    Ok(entry.cid)
}

fn commit(
    store: &FeedbackStore,
    old: Option<Checkpoint>,
    mut next: Checkpoint,
    bytes: &[u8],
    legacy_digest: Option<String>,
) -> Result<()> {
    let d = dir(store);
    let c = capacity(store)?;
    next.active = old.as_ref().map_or(0, |s| 1 - s.active);
    next.bytes = bytes.len() as u64;
    next.digest = digest(bytes)?;
    if next.bytes > c.parameters.active_bytes || next.bytes > c.parameters.replacement_bytes {
        return invalid("replacement generation exceeds declared reserve");
    }
    let obsolete = next
        .receipts
        .len()
        .saturating_sub(c.parameters.recent_receipts);
    next.receipts.drain(..obsolete);
    next.cid = next.address()?;
    let txn = Transaction {
        version: 2,
        old,
        next: next.clone(),
        legacy_digest,
    };
    let journal = json_bytes(&txn)?;
    let state = json_bytes(&next)?;
    let (total, control) = accounting(store)?;
    let metadata =
        journal.len() as u64 + state.len() as u64 + SENTINEL.len() as u64 + SIGNAL_RESERVE;
    if control + metadata > c.parameters.control_bytes
        || total + metadata + bytes.len() as u64 > c.parameters.total_bytes
    {
        return invalid("insufficient declared transaction reserve; evidence preserved");
    }
    write_sync(&d.join("transaction.next"), &journal)?;
    fs::rename(d.join("transaction.next"), d.join("transaction.json"))?;
    sync(&d)?;
    fault("journal")?;
    write_sync(&slot(&d, next.active), bytes)?;
    fault("generation")?;
    write_sync(&d.join("state.next"), &state)?;
    fault("checkpoint")?;
    fs::rename(d.join("state.next"), d.join("state.json"))?;
    fault("activate")?;
    sync(&d)?;
    fault("directory")?;
    finish(store, &txn)?;
    Ok(())
}

fn recover(store: &FeedbackStore) -> Result<()> {
    let d = dir(store);
    let c = capacity(store)?;
    accounting(store)?;
    let path = d.join("transaction.json");
    if !path.exists() {
        if d.join("state.json").exists() {
            contents(store, &checkpoint(store)?)?;
        } else {
            let legacy = store.root.join(".eprfs/status/feedback.jsonl");
            if legacy.exists() {
                decode(
                    std::str::from_utf8(&read_bounded(&legacy, c.parameters.active_bytes)?)
                        .map_err(|e| FabricError::Decode(e.to_string()))?,
                )?;
            }
        }
        // This journal was never activated; no generation has been touched yet.
        remove(&d.join("transaction.next"))?;
        if d.exists() {
            for file in fs::read_dir(&d)? {
                let file = file?;
                let name = file.file_name();
                let name = name.to_string_lossy();
                if name.starts_with("signal.json.") && name.ends_with(".tmp") {
                    remove(&file.path())?;
                }
            }
        }
        return Ok(());
    }
    let txn: Transaction = parse(&read_bounded(&path, c.parameters.control_bytes)?)?;
    if txn.version != 2
        || txn.next.active > 1
        || txn
            .old
            .as_ref()
            .is_some_and(|s| s.active > 1 || s.active == txn.next.active)
    {
        return invalid("unsupported or corrupt feedback transaction");
    }
    txn.next.verify()?;
    if let Some(old) = &txn.old {
        old.verify()?;
    }
    let current = if d.join("state.json").exists() {
        Some(checkpoint(store)?)
    } else {
        None
    };
    if current.as_ref().is_some_and(|s| s.cid == txn.next.cid) {
        contents(store, &txn.next)?;
        finish(store, &txn)?;
    } else {
        match (&current, &txn.old) {
            (Some(a), Some(b)) if a.cid == b.cid => {
                contents(store, a)?;
            }
            (None, None) => {
                let legacy = read_bounded(
                    &store.root.join(".eprfs/status/feedback.jsonl"),
                    c.parameters.active_bytes,
                )?;
                if Some(digest(&legacy)?) != txn.legacy_digest {
                    return invalid("legacy migration source changed");
                }
                decode(
                    std::str::from_utf8(&legacy).map_err(|e| FabricError::Decode(e.to_string()))?,
                )?;
            }
            _ => return invalid("feedback transaction does not join active checkpoint"),
        }
        remove(&slot(&d, txn.next.active))?;
        remove(&d.join("state.next"))?;
        remove(&path)?;
        sync(&d)?;
    }
    Ok(())
}
fn finish(store: &FeedbackStore, txn: &Transaction) -> Result<()> {
    let d = dir(store);
    contents(store, &txn.next)?;
    if let Some(old) = &txn.old {
        if slot(&d, old.active).exists() {
            contents(store, old)?;
            remove(&slot(&d, old.active))?;
        }
    }
    if let Some(expected) = &txn.legacy_digest {
        let path = store.root.join(".eprfs/status/feedback.jsonl");
        let legacy = fs::read(&path)?;
        if legacy != SENTINEL.as_bytes() {
            if digest(&legacy)? != *expected {
                return invalid("legacy source changed during cutover");
            }
            let pending = d.join("legacy.next");
            if pending.exists() {
                remove(&pending)?;
            }
            write_sync(&pending, SENTINEL.as_bytes())?;
            fs::rename(pending, path)?;
            sync(&store.root.join(".eprfs/status"))?;
        }
    }
    sync(&d)?;
    fault("retire")?;
    remove(&d.join("transaction.json"))?;
    sync(&d)?;
    Ok(())
}

fn observe(store: &FeedbackStore) -> Result<()> {
    use elohim_epr::algedonic::{AlgedonicLatch, Hysteresis, LatchConfig, LatchSense, Reading};
    let c = capacity(store)?;
    let d = dir(store);
    let state = checkpoint(store)?;
    let (total, control) = accounting(store)?;
    if total + SIGNAL_RESERVE > c.parameters.total_bytes
        || control + SIGNAL_RESERVE > c.parameters.control_bytes
    {
        return invalid("capacity signal control reserve unavailable");
    }
    let path = d.join("signal.json");
    // A changed policy is a different bound. Preserve old identities in receipts/atoms;
    // the bounded current signal state is rebuilt for the newly selected bound.
    if path.exists() {
        let value: serde_json::Value = parse(&read_bounded(&path, SIGNAL_RESERVE / 2)?)?;
        if value["bound_ref"].as_str() != Some(&c.policy_cid) {
            remove(&path)?;
        }
    }
    let map = |e: elohim_epr::EprError| FabricError::InvalidFeedback(e.to_string());
    let mut latch = AlgedonicLatch::open(
        LatchConfig {
            declarer: "local-feedback".into(),
            target: c.policy_cid.clone(),
            bound_ref: c.policy_cid.clone(),
            limit: c.parameters.active_bytes as f64,
            sense: LatchSense::Ceiling,
            hysteresis: Hysteresis {
                threshold_pct: f64::from(c.parameters.trigger_percent),
                reset_pct: f64::from(c.parameters.reset_percent),
                dwell_s: c.parameters.dwell_seconds,
            },
        },
        path,
    )
    .map_err(map)?;
    let now = Utc::now();
    let reading = Reading::Closed {
        cause_ref: &state.digest,
    };
    latch
        .observe(state.bytes as f64, now, reading)
        .map_err(map)?;
    // The native latch requires a fresh witnessed reading after reopen, including zero dwell.
    latch
        .observe(state.bytes as f64, now, reading)
        .map_err(map)?;
    if let Some(warning) = latch.take_durability_warning() {
        return invalid(&warning);
    }
    Ok(())
}

#[cfg(not(test))]
fn fault(_stage: &str) -> Result<()> {
    Ok(())
}
#[cfg(test)]
thread_local! { static FAIL_AT: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) }; }
#[cfg(test)]
fn fault(stage: &str) -> Result<()> {
    if FAIL_AT.with(|f| f.borrow().as_deref() == Some(stage)) {
        return Err(std::io::Error::from(std::io::ErrorKind::StorageFull).into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::feedback::{Feedback, Occurrence, Outcome, Policy, Source};

    fn setup() -> (tempfile::TempDir, FeedbackStore) {
        let root = tempfile::tempdir().unwrap();
        let mut policy:Policy=parse(br#"{"id":"fixture@2","authority":"operator-test","participant":null,"producers":["brit"],"automaticCapture":true,"deliberateEvidence":true,"response":"report","sharing":"separately-authorized"}"#).unwrap();
        let p = json!({"totalBytes":180000,"activeBytes":50000,"replacementBytes":50000,"controlBytes":80000,
            "recordBytes":10000,"triggerPercent":85,"resetPercent":70,"routineDays":7,"resolvedDays":30,
            "recentReceipts":2,"dwellSeconds":0});
        policy.capacity=Some(Capacity::resolve("fixture@1".into(),json!({"id":"fixture","version":1,
            "established_by":"operator-test","binding":"binding-local", "permits":["reclaim-eligible-feedback"], "parameter-bounds":{"routineDays":{"min":7},"resolvedDays":{"min":30},"controlBytes":{"min":65536},"recentReceipts":{"min":1,"max":32}},"why":"test","scope":{"path":".eprfs/status/feedback"},
            "measure":p,"permitted-local-parameters":[]}),json!({})).unwrap());
        let store = FeedbackStore::new(root.path(), policy).unwrap();
        store.migrate(true).unwrap();
        (root, store)
    }
    fn record(run: &str) -> Feedback {
        Feedback::Occurrence(Occurrence {
            source: Source {
                producer: "brit".into(),
                run: run.into(),
                item: "status".into(),
            },
            observed_at: "2020-01-01T00:00:00Z".into(),
            participant: None,
            subject: "status".into(),
            outcome: Outcome::Passed,
            count: 1,
            duration_ms: None,
            impact: None,
            message: None,
            detail: json!(null),
        })
    }
    #[test]
    fn feedback_crash_and_disk_full_at_every_transition_preserve_acknowledged_survivors() {
        for stage in [
            "journal",
            "generation",
            "checkpoint",
            "activate",
            "directory",
            "retire",
            "partial:transaction.next",
            "partial:generation-0.jsonl",
            "partial:state.next",
        ] {
            let (_root, store) = setup();
            let protected = store
                .append_protected(record("protected"), true, true)
                .unwrap();
            let before = store.read().unwrap();
            FAIL_AT.with(|s| *s.borrow_mut() = Some(stage.into()));
            let result = store.append(record("interrupted"), false);
            FAIL_AT.with(|s| *s.borrow_mut() = None);
            assert!(result.is_err(), "fault {stage} reached");
            assert!(
                accounting(&store).unwrap().0 <= 180000,
                "transaction reserve {stage}"
            );
            store.reclaim(false).unwrap();
            let entries = store.read().unwrap();
            assert!(
                entries.iter().any(|e| e.cid == protected),
                "acknowledged survivor {stage}"
            );
            assert_eq!(&entries[0], &before[0]);
            store.append(record("after-restart"), false).unwrap();
            assert!(!dir(&store).join("transaction.json").exists());
            assert!(!dir(&store).join("transaction.next").exists());
        }
    }
    #[test]
    fn feedback_compaction_faults_keep_protected_records_and_recover_receipts() {
        for stage in [
            "journal",
            "generation",
            "checkpoint",
            "activate",
            "directory",
            "retire",
        ] {
            let (_root, store) = setup();
            let id = store
                .append_protected(record("protected"), true, true)
                .unwrap();
            for i in 0..100 {
                if store.capacity().unwrap().retained_bytes >= 42500 {
                    break;
                }
                store.append(record(&format!("old-{i}")), false).unwrap();
            }
            assert!(store.capacity().unwrap().selected_bytes > 0);
            FAIL_AT.with(|s| *s.borrow_mut() = Some(stage.into()));
            assert!(store.reclaim(false).is_err());
            FAIL_AT.with(|s| *s.borrow_mut() = None);
            assert!(accounting(&store).unwrap().0 <= 180000);
            let after = store.reclaim(false).unwrap();
            assert!(after.retained_bytes <= 35000);
            assert!(store.read().unwrap().iter().any(|e| e.cid == id));
            assert!(after.recent_receipts.len() <= 2);
        }
    }
}
