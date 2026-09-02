//! `FlowStore` — the polymorphic persistence seam. One model, three depths:
//! sidecar (`.eprfs/status/`, offline floor — implemented here), diesel projection and
//! DHT rails (implemented in their own crates against this trait).

use std::collections::HashSet;
#[cfg(feature = "sidecar")]
use std::fs;
#[cfg(feature = "sidecar")]
use std::io::Write as _;
#[cfg(feature = "sidecar")]
use std::path::{Path, PathBuf};

use cid::Cid;
use elohim_epr::algedonic::AlgedonicEvidence;
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

#[cfg(feature = "sidecar")]
#[derive(Serialize, Deserialize)]
struct SidecarLine {
    cid: String,
    record: FlowRecord,
}

#[cfg(feature = "sidecar")]
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
            }))
            .unwrap();

        let edges = store.edges().unwrap();
        assert_eq!(edges.len(), 2);
        assert!(edges.contains(&(e1_cid, e1)));
        assert!(edges.contains(&(e2_cid, e2)));
    }
}
