//! The two operations: `observe` appends the reliance records, `verify` compares them with the
//! store and never writes.
//!
//! The refuse-without-mutation shape of `bridges/jenkins/src/observe.rs`: inputs are parsed and
//! translated BEFORE the sidecar is opened for writing, and the one write is a single
//! transaction that appends only records whose CID is not already there. So a rerun over
//! unchanged metadata leaves `.eprfs/status/flows.jsonl` byte-identical.
//!
//! Unlike Jenkins' observations, reliance records are not gated on the offer being Active: they
//! are rebuilt from the lockfile (operational, class C), the offer is minted Proposed beside
//! them, and its standing is a Steward's to change. Whether a cite record should wait for an
//! approved offer is recorded as an open question in the reliance-as-cite spec's follow-ons.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use cid::Cid;
use elohim_epr_rea::{FlowRecord, FlowStore, SidecarFlowStore};
use serde::Serialize;

use crate::metadata::{parse_lockfile, parse_metadata};
use crate::translate::{translate, Config, Refusal, Reliance, Report};

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

/// Parse and translate. Pure with respect to the sidecar.
pub fn reliance(inputs: &Inputs, config: &Config) -> Result<Reliance> {
    let metadata = parse_metadata(&inputs.metadata)?;
    let lock_text = std::str::from_utf8(&inputs.lockfile)
        .map_err(|e| BridgeError::Malformed(format!("Cargo.lock is not UTF-8: {e}")))?;
    let checksums = parse_lockfile(lock_text)?;
    Ok(translate(&metadata, &checksums, &inputs.lockfile, config)?)
}

/// What `observe` did.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObserveOutcome {
    pub offer: String,
    pub process: String,
    pub appended: usize,
    pub present: usize,
    pub report: Report,
}

fn sidecar_cids(root: &Path) -> Result<HashSet<Cid>> {
    if !root.join(FLOWS).is_file() {
        return Ok(HashSet::new());
    }
    SidecarFlowStore::open(root)
        .and_then(|s| s.records())
        .map(|records| records.into_iter().map(|(c, _)| c).collect())
        .map_err(|e| BridgeError::Io(e.to_string()))
}

fn record_cid(record: &FlowRecord) -> Result<Cid> {
    record
        .cid()
        .map_err(|e| BridgeError::Malformed(e.to_string()))
}

/// Translate, then append the records the store lacks — idempotently, deduped by CID under one
/// sidecar transaction.
pub fn observe(root: &Path, inputs: &Inputs, config: &Config) -> Result<ObserveOutcome> {
    let reliance = reliance(inputs, config)?;
    let store = SidecarFlowStore::open(root).map_err(|e| BridgeError::Io(e.to_string()))?;
    let mut tx = store
        .transaction()
        .map_err(|e| BridgeError::Io(e.to_string()))?;
    let mut existing: HashSet<Cid> = tx
        .records()
        .map_err(|e| BridgeError::Io(e.to_string()))?
        .into_iter()
        .map(|(c, _)| c)
        .collect();
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
    Ok(ObserveOutcome {
        offer: reliance.offer_cid.to_string(),
        process: reliance.process_cid.to_string(),
        appended,
        present,
        report: reliance.report,
    })
}

/// What `verify` found: the records the store would gain from an `observe` now.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifyOutcome {
    pub workspace: String,
    pub lockfile: String,
    pub missing: usize,
    pub total: usize,
}

impl VerifyOutcome {
    pub fn is_fresh(&self) -> bool {
        self.missing == 0
    }

    /// The rung-one notice: a stale record is named, with the command that regenerates it.
    pub fn notice(&self) -> String {
        if self.is_fresh() {
            format!(
                "reliance record for workspace `{}` is fresh ({} records)",
                self.workspace, self.total
            )
        } else {
            format!(
                "notice: the reliance record for workspace `{}` is stale — {} of {} records \
                 derived from {} are not in {FLOWS}; regenerate with `cargo-bridge observe` \
                 (proceeding records nothing; RELIANCE=strict refuses)",
                self.workspace, self.missing, self.total, self.lockfile
            )
        }
    }
}

/// Compare the would-be records with the store. Never writes.
pub fn verify(root: &Path, inputs: &Inputs, config: &Config) -> Result<VerifyOutcome> {
    let reliance = reliance(inputs, config)?;
    let existing = sidecar_cids(root)?;
    let records = reliance.records();
    let mut missing = 0;
    for record in &records {
        if !existing.contains(&record_cid(record)?) {
            missing += 1;
        }
    }
    Ok(VerifyOutcome {
        workspace: config.workspace.clone(),
        lockfile: elohim_epr::cid::compute_raw_cid(&inputs.lockfile).to_string(),
        missing,
        total: records.len(),
    })
}

/// Write the workspace's offer document under `root` (its rendered text, byte for byte).
pub fn write_offer(root: &Path, reliance: &Reliance) -> Result<PathBuf> {
    let path = root.join(reliance.offer.path());
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| BridgeError::Io(e.to_string()))?;
    }
    std::fs::write(&path, reliance.offer.text()).map_err(|e| BridgeError::Io(e.to_string()))?;
    Ok(path)
}
