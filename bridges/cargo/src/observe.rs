//! The two operations: `observe` appends the reliance records, `verify` compares them with the
//! store and never writes.
//!
//! The refuse-without-mutation shape of `bridges/jenkins/src/observe.rs`: inputs are parsed and
//! translated BEFORE the sidecar is opened for writing, and the one write is a single
//! transaction that appends only records whose CID is not already there. So a rerun over
//! unchanged metadata leaves `.eprfs/status/flows.jsonl` byte-identical.
//!
//! **Time is the first observation's, not the commit's.** Every event CID includes its
//! `occurred_at`. If the time came from the lockfile's commit, a rebase or squash would move it
//! and re-append every record. So when the store already holds this workspace's Process for this
//! exact lockfile (same `lockfile:<CID>` slot), its time is reused and the caller's
//! `occurred_at` only dates a lockfile seen for the first time.
//!
//! **The offer document is persisted beside the records** at its repository path under the
//! store root, so the text an approval is of can be retrieved, not only its Intent.
//!
//! Unlike Jenkins' observations, reliance records are not gated on the offer being Active: they
//! are rebuilt from the lockfile (operational, class C), the offer is minted Proposed beside
//! them, and its standing is a Steward's to change.

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use cid::Cid;
use elohim_epr_rea::{FlowRecord, FlowStore, SidecarFlowStore};
use serde::Serialize;

use crate::metadata::{parse_lockfile, parse_metadata};
use crate::translate::{lockfile_slot, translate, Config, Refusal, Reliance, Report, RECIPE_ID};

const FLOWS: &str = ".eprfs/status/flows.jsonl";

/// Why an operation refused, and the exit code it carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BridgeError {
    /// Malformed or unknown input (exit 65). Nothing was written.
    Malformed(String),
    /// The sidecar could not be read or written (exit 74).
    Io(String),
}

impl BridgeError {
    pub fn exit_code(&self) -> u8 {
        match self {
            BridgeError::Malformed(_) => 65,
            BridgeError::Io(_) => 74,
        }
    }
}

impl std::fmt::Display for BridgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BridgeError::Malformed(m) => {
                write!(f, "refused (malformed input, nothing written): {m}")
            }
            BridgeError::Io(m) => write!(f, "sidecar: {m}"),
        }
    }
}
impl std::error::Error for BridgeError {}

impl From<Refusal> for BridgeError {
    fn from(r: Refusal) -> Self {
        BridgeError::Malformed(r.0)
    }
}

pub type Result<T> = std::result::Result<T, BridgeError>;

/// The inputs one run reads: cargo's metadata JSON and the workspace's lockfile.
#[derive(Debug, Clone)]
pub struct Inputs {
    pub metadata: String,
    pub lockfile: Vec<u8>,
}

/// Parse and translate with the config's `occurred_at` as given. Pure with respect to the
/// sidecar; [`observe`] and [`verify`] first reuse a stored first-observation time.
pub fn reliance(inputs: &Inputs, config: &Config) -> Result<Reliance> {
    let metadata = parse_metadata(&inputs.metadata)?;
    let lock_text = std::str::from_utf8(&inputs.lockfile)
        .map_err(|e| BridgeError::Malformed(format!("Cargo.lock is not UTF-8: {e}")))?;
    let checksums = parse_lockfile(lock_text)?;
    Ok(translate(&metadata, &checksums, &inputs.lockfile, config)?)
}

/// Every record in the store under `root`, or none when it has no store yet. Never creates one.
pub fn stored_records(root: &Path) -> Result<Vec<(Cid, FlowRecord)>> {
    if !root.join(FLOWS).is_file() {
        return Ok(Vec::new());
    }
    SidecarFlowStore::open(root)
        .and_then(|s| s.records())
        .map_err(|e| BridgeError::Io(e.to_string()))
}

/// The time this workspace's lockfile (these exact bytes) was first observed in `records`: the
/// `occurred_at` of the first input of the earliest-appended matching Process.
pub fn first_observed_at(
    records: &[(Cid, FlowRecord)],
    workspace: &str,
    lockfile_bytes: &[u8],
) -> Option<String> {
    let lockfile = lockfile_slot(lockfile_bytes);
    let workspace_slot = format!("workspace:{workspace}");
    let events: BTreeMap<Cid, &str> = records
        .iter()
        .filter_map(|(cid, r)| match r {
            FlowRecord::Event(e) => Some((*cid, e.occurred_at.as_str())),
            _ => None,
        })
        .collect();
    records.iter().find_map(|(_, r)| match r {
        FlowRecord::Process(p)
            if p.spec.id == RECIPE_ID
                && p.classified_as.contains(&lockfile)
                && p.classified_as.contains(&workspace_slot) =>
        {
            p.inputs
                .first()
                .and_then(|input| events.get(input))
                .map(|t| t.to_string())
        }
        _ => None,
    })
}

/// The config with `occurred_at` replaced by the stored first-observation time, if any.
fn settled(records: &[(Cid, FlowRecord)], inputs: &Inputs, config: &Config) -> Config {
    let mut config = config.clone();
    if let Some(time) = first_observed_at(records, &config.workspace, &inputs.lockfile) {
        config.occurred_at = time;
    }
    config
}

/// What `observe` did.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObserveOutcome {
    pub offer: String,
    pub offer_document: String,
    pub process: String,
    pub occurred_at: String,
    pub appended: usize,
    pub present: usize,
    pub report: Report,
}

fn record_cid(record: &FlowRecord) -> Result<Cid> {
    record
        .cid()
        .map_err(|e| BridgeError::Malformed(e.to_string()))
}

/// Translate, then append the records the store lacks — idempotently, deduped by CID under one
/// sidecar transaction — and persist the offer document (byte-identical on a rerun).
pub fn observe(root: &Path, inputs: &Inputs, config: &Config) -> Result<ObserveOutcome> {
    // 1. Inputs first: a malformed input refuses before anything is opened for writing.
    let first = reliance(inputs, config)?;
    let store = SidecarFlowStore::open(root).map_err(|e| BridgeError::Io(e.to_string()))?;
    let mut tx = store
        .transaction()
        .map_err(|e| BridgeError::Io(e.to_string()))?;
    let records = tx.records().map_err(|e| BridgeError::Io(e.to_string()))?;
    let settled_config = settled(&records, inputs, config);
    let reliance = if settled_config.occurred_at == config.occurred_at {
        first
    } else {
        reliance(inputs, &settled_config)?
    };
    let config = settled_config;
    let mut existing: HashSet<Cid> = records.into_iter().map(|(c, _)| c).collect();
    let (mut appended, mut present) = (0, 0);
    for record in reliance.records() {
        let cid = record_cid(&record)?;
        if !existing.insert(cid) {
            present += 1;
            continue;
        }
        tx.append(record)
            .map_err(|e| BridgeError::Io(e.to_string()))?;
        appended += 1;
    }
    drop(tx);
    let document = write_offer(root, &reliance)?;
    Ok(ObserveOutcome {
        offer: reliance.offer_cid.to_string(),
        offer_document: document.display().to_string(),
        process: reliance.process_cid.to_string(),
        occurred_at: config.occurred_at,
        appended,
        present,
        report: reliance.report,
    })
}

/// What `verify` found: the records (and the offer document) an `observe` would add now.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifyOutcome {
    pub workspace: String,
    pub lockfile: String,
    pub missing: usize,
    pub total: usize,
    pub offer_document_current: bool,
}

impl VerifyOutcome {
    pub fn is_fresh(&self) -> bool {
        self.missing == 0 && self.offer_document_current
    }

    /// The rung-one notice: a stale record is named, with the command that regenerates it.
    pub fn notice(&self) -> String {
        if self.is_fresh() {
            format!(
                "reliance record for workspace `{}` is fresh ({} records)",
                self.workspace, self.total
            )
        } else {
            let document = if self.offer_document_current {
                ""
            } else {
                "; the offer document is missing or differs"
            };
            format!(
                "notice: the reliance record for workspace `{}` is stale — {} of {} records \
                 derived from {} are not in {FLOWS}{document}; regenerate with `cargo-bridge \
                 observe` (proceeding records nothing; RELIANCE=strict refuses)",
                self.workspace, self.missing, self.total, self.lockfile
            )
        }
    }
}

/// Compare the would-be records with the store. Never writes, and never creates the store: a
/// missing store reads as empty.
pub fn verify(root: &Path, inputs: &Inputs, config: &Config) -> Result<VerifyOutcome> {
    let records = stored_records(root)?;
    let config = settled(&records, inputs, config);
    let reliance = reliance(inputs, &config)?;
    let existing: HashSet<Cid> = records.into_iter().map(|(c, _)| c).collect();
    let would = reliance.records();
    let mut missing = 0;
    for record in &would {
        if !existing.contains(&record_cid(record)?) {
            missing += 1;
        }
    }
    let document = root.join(reliance.offer.path());
    let offer_document_current =
        std::fs::read_to_string(&document).is_ok_and(|text| text == reliance.offer.text());
    Ok(VerifyOutcome {
        workspace: config.workspace.clone(),
        lockfile: elohim_epr::cid::compute_raw_cid(&inputs.lockfile).to_string(),
        missing,
        total: would.len(),
        offer_document_current,
    })
}

/// Write the workspace's offer document under `root` (its rendered text, byte for byte).
pub fn write_offer(root: &Path, reliance: &Reliance) -> Result<PathBuf> {
    let path = root.join(reliance.offer.path());
    if std::fs::read_to_string(&path).is_ok_and(|text| text == reliance.offer.text()) {
        return Ok(path);
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| BridgeError::Io(e.to_string()))?;
    }
    std::fs::write(&path, reliance.offer.text()).map_err(|e| BridgeError::Io(e.to_string()))?;
    Ok(path)
}
