//! `FlowStore` — the polymorphic persistence seam. One model, three depths:
//! sidecar (`.eprfs/status/`, offline floor — implemented here), diesel projection and
//! DHT rails (implemented in their own crates against this trait).

use std::collections::HashSet;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use cid::Cid;
use serde::{Deserialize, Serialize};

use crate::error::{FabricError, Result};
use crate::model::{
    atom_cid, Commitment, CommitmentState, FlowEvent, Intent, Process, ProcessSpec,
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
        }
    }
}

pub trait FlowStore {
    /// Append a record; returns the record's atom CID.
    fn append(&mut self, record: FlowRecord) -> Result<Cid>;

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
#[derive(Debug)]
pub struct SidecarFlowStore {
    log_path: PathBuf,
}

impl SidecarFlowStore {
    /// Open (creating directories/log as needed) the sidecar under `root`.
    pub fn open(root: &Path) -> Result<Self> {
        let dir = root.join(".eprfs").join("status");
        fs::create_dir_all(&dir)?;
        let log_path = dir.join("flows.jsonl");
        if !log_path.exists() {
            fs::File::create(&log_path)?;
        }
        Ok(Self { log_path })
    }

    pub fn log_path(&self) -> &Path {
        &self.log_path
    }
}

#[derive(Serialize, Deserialize)]
struct SidecarLine {
    cid: String,
    record: FlowRecord,
}

impl FlowStore for SidecarFlowStore {
    fn append(&mut self, record: FlowRecord) -> Result<Cid> {
        let cid = record.cid()?;
        let line = serde_json::to_string(&SidecarLine {
            cid: cid.to_string(),
            record,
        })
        .map_err(|e| FabricError::Encode(e.to_string()))?;
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)?;
        writeln!(file, "{line}")?;
        Ok(cid)
    }

    fn records(&self) -> Result<Vec<(Cid, FlowRecord)>> {
        let contents = fs::read_to_string(&self.log_path)?;
        let mut records = Vec::new();
        for line in contents.lines().filter(|l| !l.trim().is_empty()) {
            let parsed: SidecarLine =
                serde_json::from_str(line).map_err(|e| FabricError::Decode(e.to_string()))?;
            let computed = parsed.record.cid()?;
            if computed.to_string() != parsed.cid {
                return Err(FabricError::Integrity {
                    stored: parsed.cid,
                    computed: computed.to_string(),
                });
            }
            records.push((computed, parsed.record));
        }
        Ok(records)
    }
}
