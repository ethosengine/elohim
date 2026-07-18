//! `epr flow walk <path>` and `epr flow status` — read the sidecar `FlowStore` and render
//! the developer value chain: lineage (walk_back) and frontier (walk_forward + the
//! directly-scoped, still-open intents/commitments a change here leaves on the plate).

use std::collections::HashSet;
use std::path::Path;

use cid::Cid;
use elohim_epr_rea::{
    Commitment, CommitmentState, FlowRecord, FlowStore, FlowWalk, Frontier, Intent, Lineage,
    Process, SidecarFlowStore,
};
use serde::Serialize;

use super::{body_cid_of_file, rel_to_root, short_cid, FlowError, FlowResult, Labels};

/// A resolved CID reference: the address plus its operational label.
#[derive(Debug, Serialize)]
pub struct Ref {
    pub cid: String,
    pub label: String,
}

#[derive(Debug, Serialize)]
pub struct EventView {
    pub provider: String,
    pub occurred_at: String,
    pub resource: Ref,
}

#[derive(Debug, Serialize)]
pub struct ProcessView {
    pub spec: String,
    pub in_scope_of: Ref,
    pub inputs: Vec<Ref>,
    pub outputs: Vec<Ref>,
}

#[derive(Debug, Serialize)]
pub struct CommitmentView {
    pub commitment: Ref,
    pub provider: String,
    pub state: String,
    pub classified_as: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct IntentView {
    pub intent: Ref,
    pub raised_by: String,
    pub classified_as: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct LineageView {
    pub producing_events: Vec<EventView>,
    pub processes: Vec<ProcessView>,
    pub inputs: Vec<Ref>,
    pub commitments: Vec<Ref>,
    pub intents: Vec<Ref>,
}

#[derive(Debug, Serialize)]
pub struct FrontierView {
    pub dependents: Vec<ProcessView>,
    pub outputs: Vec<Ref>,
    pub unfulfilled_commitments: Vec<CommitmentView>,
    pub scoped_intents: Vec<IntentView>,
}

#[derive(Debug, Serialize)]
pub struct WalkResult {
    pub target: String,
    pub target_cid: String,
    pub lineage: LineageView,
    pub frontier: FrontierView,
}

fn resolve(labels: &Labels, cid: &Cid) -> Ref {
    let key = cid.to_string();
    let label = labels.get(&key).cloned().unwrap_or_else(|| short_cid(cid));
    Ref { cid: key, label }
}

fn load_labels(root: &Path) -> Labels {
    let path = root.join(".eprfs").join("status").join("labels.json");
    std::fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn process_view(labels: &Labels, process: &Process) -> ProcessView {
    ProcessView {
        spec: format!("{}@{}", process.spec.id, process.spec.version),
        in_scope_of: resolve(labels, &process.in_scope_of),
        inputs: process.inputs.iter().map(|c| resolve(labels, c)).collect(),
        outputs: process.outputs.iter().map(|c| resolve(labels, c)).collect(),
    }
}

fn commitment_view(labels: &Labels, cid: &Cid, c: &Commitment) -> CommitmentView {
    CommitmentView {
        commitment: resolve(labels, cid),
        provider: c.provider.0.clone(),
        state: format!("{:?}", c.state),
        classified_as: c.resource_spec.classified_as.clone(),
    }
}

fn intent_view(labels: &Labels, cid: &Cid, i: &Intent) -> IntentView {
    IntentView {
        intent: resolve(labels, cid),
        raised_by: i.raised_by.0.clone(),
        classified_as: i.resource_spec.classified_as.clone(),
    }
}

pub fn walk(root: &Path, rel_path: &str) -> FlowResult<WalkResult> {
    let abs = if Path::new(rel_path).is_absolute() {
        std::path::PathBuf::from(rel_path)
    } else {
        root.join(rel_path)
    };
    let target = rel_to_root(root, &abs);
    let cid =
        body_cid_of_file(&abs).ok_or_else(|| FlowError::UnknownResource(rel_path.to_string()))?;

    let store = SidecarFlowStore::open(root)?;
    let labels = load_labels(root);

    let lineage: Lineage = store.walk_back(&cid)?;
    let frontier: Frontier = store.walk_forward(&cid)?;

    // Directly-scoped, still-open intents/commitments: a change at `cid` leaves these on
    // the plate even when no downstream Process yet consumes it.
    let mut scoped_intents = Vec::new();
    let discharged: HashSet<Cid> = store
        .events()?
        .into_iter()
        .flat_map(|(_, e)| e.fulfills)
        .collect();
    let mut scoped_unfulfilled: Vec<CommitmentView> = frontier
        .unfulfilled
        .iter()
        .map(|(c, commitment)| commitment_view(&labels, c, commitment))
        .collect();
    let mut seen_commitments: HashSet<Cid> = frontier.unfulfilled.iter().map(|(c, _)| *c).collect();

    for (rcid, record) in store.records()? {
        match record {
            FlowRecord::Intent(i) if i.in_scope_of == cid => {
                scoped_intents.push(intent_view(&labels, &rcid, &i));
            }
            FlowRecord::Commitment(c)
                if c.in_scope_of == cid
                    && matches!(c.state, CommitmentState::Proposed | CommitmentState::Active)
                    && !discharged.contains(&rcid)
                    && seen_commitments.insert(rcid) =>
            {
                scoped_unfulfilled.push(commitment_view(&labels, &rcid, &c));
            }
            _ => {}
        }
    }

    let lineage_view = LineageView {
        producing_events: lineage
            .producing_events
            .iter()
            .map(|(_, e)| EventView {
                provider: e.provider.0.clone(),
                occurred_at: e.occurred_at.clone(),
                resource: resolve(&labels, &e.resource),
            })
            .collect(),
        processes: lineage
            .processes
            .iter()
            .map(|(_, p)| process_view(&labels, p))
            .collect(),
        inputs: lineage.inputs.iter().map(|c| resolve(&labels, c)).collect(),
        commitments: lineage
            .commitments
            .iter()
            .map(|c| resolve(&labels, c))
            .collect(),
        intents: lineage
            .intents
            .iter()
            .map(|c| resolve(&labels, c))
            .collect(),
    };

    let frontier_view = FrontierView {
        dependents: frontier
            .dependents
            .iter()
            .map(|(_, p)| process_view(&labels, p))
            .collect(),
        outputs: frontier
            .outputs
            .iter()
            .map(|c| resolve(&labels, c))
            .collect(),
        unfulfilled_commitments: scoped_unfulfilled,
        scoped_intents,
    };

    Ok(WalkResult {
        target,
        target_cid: cid.to_string(),
        lineage: lineage_view,
        frontier: frontier_view,
    })
}

impl WalkResult {
    pub fn render(&self) {
        println!("epr flow walk — {}", self.target);
        println!("  resource cid: {}", self.target_cid);
        println!("\n  LINEAGE (walk back — how this came to be)");
        if self.lineage.producing_events.is_empty() {
            println!("    (no producing events recorded)");
        }
        for e in &self.lineage.producing_events {
            let when = if e.occurred_at.is_empty() {
                "unknown".to_string()
            } else {
                e.occurred_at.clone()
            };
            println!("    produced by {} at {}", e.provider, when);
        }
        for p in &self.lineage.processes {
            println!("    process {} — {} input(s):", p.spec, p.inputs.len());
            for input in &p.inputs {
                println!("      ← {}", input.label);
            }
        }
        if !self.lineage.commitments.is_empty() {
            println!("    fulfills commitment(s):");
            for c in &self.lineage.commitments {
                println!("      • {}", c.label);
            }
        }
        if !self.lineage.intents.is_empty() {
            println!("    satisfies intent(s):");
            for i in &self.lineage.intents {
                println!("      • {}", i.label);
            }
        }

        println!("\n  FRONTIER (walk forward — what a change here leaves open)");
        for p in &self.frontier.dependents {
            println!(
                "    feeds process {} → {} output(s)",
                p.spec,
                p.outputs.len()
            );
            for out in &p.outputs {
                println!("      → {}", out.label);
            }
        }
        if !self.frontier.scoped_intents.is_empty() {
            println!(
                "    open intents scoped here ({}):",
                self.frontier.scoped_intents.len()
            );
            for i in &self.frontier.scoped_intents {
                println!(
                    "      ◇ {} [{}]",
                    i.intent.label,
                    i.classified_as.join(", ")
                );
            }
        }
        if !self.frontier.unfulfilled_commitments.is_empty() {
            println!(
                "    unfulfilled commitments ({}):",
                self.frontier.unfulfilled_commitments.len()
            );
            for c in &self.frontier.unfulfilled_commitments {
                println!(
                    "      ✗ {} [{}]",
                    c.commitment.label,
                    c.classified_as.join(", ")
                );
            }
        }
        if self.frontier.dependents.is_empty()
            && self.frontier.scoped_intents.is_empty()
            && self.frontier.unfulfilled_commitments.is_empty()
        {
            println!("    (frontier clear — nothing downstream is open)");
        }
    }
}

// ---------------------------------------------------------------------------
// status
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct StatusResult {
    pub resources_labeled: usize,
    pub events: usize,
    pub intents: usize,
    pub commitments_active: usize,
    pub unfulfilled_total: usize,
    pub top_unfulfilled: Vec<CommitmentView>,
}

pub fn status(root: &Path) -> FlowResult<StatusResult> {
    let store = SidecarFlowStore::open(root)?;
    let labels = load_labels(root);
    let records = store.records()?;

    let events = records
        .iter()
        .filter(|(_, r)| matches!(r, FlowRecord::Event(_)))
        .count();
    let intents = records
        .iter()
        .filter(|(_, r)| matches!(r, FlowRecord::Intent(_)))
        .count();
    let commitments_active = records
        .iter()
        .filter(
            |(_, r)| matches!(r, FlowRecord::Commitment(c) if c.state == CommitmentState::Active),
        )
        .count();

    // Unfulfilled across every distinct scope any commitment declares.
    let scopes: HashSet<Cid> = records
        .iter()
        .filter_map(|(_, r)| match r {
            FlowRecord::Commitment(c) => Some(c.in_scope_of),
            _ => None,
        })
        .collect();
    let mut seen: HashSet<Cid> = HashSet::new();
    let mut unfulfilled: Vec<CommitmentView> = Vec::new();
    for scope in &scopes {
        for (cid, commitment) in store.unfulfilled_in_scope(scope)? {
            if seen.insert(cid) {
                unfulfilled.push(commitment_view(&labels, &cid, &commitment));
            }
        }
    }
    let unfulfilled_total = unfulfilled.len();
    unfulfilled.truncate(10);

    Ok(StatusResult {
        resources_labeled: labels.len(),
        events,
        intents,
        commitments_active,
        unfulfilled_total,
        top_unfulfilled: unfulfilled,
    })
}

impl StatusResult {
    pub fn render(&self) {
        println!("epr flow status");
        println!("  resources labeled:   {}", self.resources_labeled);
        println!("  flow events:         {}", self.events);
        println!("  intents:             {}", self.intents);
        println!("  active commitments:  {}", self.commitments_active);
        println!("  unfulfilled (total): {}", self.unfulfilled_total);
        if !self.top_unfulfilled.is_empty() {
            println!("  top unfulfilled:");
            for c in &self.top_unfulfilled {
                println!(
                    "    ✗ {} [{}]",
                    c.commitment.label,
                    c.classified_as.join(", ")
                );
            }
        }
    }
}
