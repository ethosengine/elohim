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

use super::edges::{edge_verdict, governor_label, EdgeIndex, IndexedEdge, Verdict};
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
    /// Downstream cite-seal edges that reference the target and have gone stale — the work
    /// a change here leaves open, surfaced from the seal-aware edge index.
    pub stale_edges: Vec<EdgeView>,
}

/// One edge in the seal-aware walk's Edges section — `verdict · governor · desc`.
#[derive(Debug, Clone, Serialize)]
pub struct EdgeView {
    pub from: String,
    pub to: String,
    pub verdict: String,
    pub governor: String,
    pub desc: Option<String>,
}

fn edge_view(edge: &IndexedEdge, verdict: &Verdict) -> EdgeView {
    EdgeView {
        from: edge.from.clone(),
        to: edge.to.clone(),
        verdict: verdict.word().to_string(),
        governor: governor_label(&edge.governor),
        desc: edge.desc.clone(),
    }
}

/// The target's sealed edges, both directions (spec §2 one-graph index).
#[derive(Debug, Serialize)]
pub struct EdgeSection {
    /// What the target depends on (edges `from` the target).
    pub outgoing: Vec<EdgeView>,
    /// Who depends on the target (edges `to` the target).
    pub incoming: Vec<EdgeView>,
}

#[derive(Debug, Serialize)]
pub struct WalkResult {
    pub target: String,
    pub target_cid: String,
    pub lineage: LineageView,
    pub frontier: FrontierView,
    pub edges: EdgeSection,
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

    // Seal-aware edge surfaces (spec §2): the one-graph index, both directions.
    let index = EdgeIndex::build(root, &store)?;
    let outgoing: Vec<(EdgeView, Verdict)> = index
        .outgoing(&target)
        .map(|e| {
            let verdict = edge_verdict(root, e);
            (edge_view(e, &verdict), verdict)
        })
        .collect();
    let incoming: Vec<(EdgeView, Verdict)> = index
        .incoming(&target)
        .map(|e| {
            let verdict = edge_verdict(root, e);
            (edge_view(e, &verdict), verdict)
        })
        .collect();
    // walk-forward: the dependents that reference the target and have gone stale.
    let stale_edges: Vec<EdgeView> = incoming
        .iter()
        .filter(|(_, v)| matches!(v, Verdict::Stale))
        .map(|(view, _)| view.clone())
        .collect();

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
        stale_edges,
    };

    let edges = EdgeSection {
        outgoing: outgoing.into_iter().map(|(v, _)| v).collect(),
        incoming: incoming.into_iter().map(|(v, _)| v).collect(),
    };

    Ok(WalkResult {
        target,
        target_cid: cid.to_string(),
        lineage: lineage_view,
        frontier: frontier_view,
        edges,
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

        println!("\n  EDGES (sealed contract edges touching this artifact)");
        if self.edges.outgoing.is_empty() && self.edges.incoming.is_empty() {
            println!("    (no sealed edges reference this artifact)");
        }
        if !self.edges.outgoing.is_empty() {
            println!("    outgoing (what this depends on):");
            for e in &self.edges.outgoing {
                print_edge_line(e);
            }
        }
        if !self.edges.incoming.is_empty() {
            println!("    incoming (who depends on this):");
            for e in &self.edges.incoming {
                print_edge_line(e);
            }
        }
    }
}

fn print_edge_line(e: &EdgeView) {
    let desc = e.desc.as_deref().unwrap_or("");
    println!("      {} · {} · {}", e.verdict, e.governor, desc);
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
    /// Sealed-edge health, partitioned by verdict (spec §2 one-graph index).
    pub edges_sealed: usize,
    pub edges_governed: usize,
    pub edges_stale: usize,
    pub edges_held: usize,
    pub edges_dangling: usize,
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

    // Seal-aware edge totals over the one-graph index, partitioned by verdict.
    let index = EdgeIndex::build(root, &store)?;
    let (mut sealed, mut governed, mut stale, mut held, mut dangling) = (0, 0, 0, 0, 0);
    for edge in &index.edges {
        match edge_verdict(root, edge) {
            Verdict::Ok => sealed += 1,
            Verdict::Governed(_) => governed += 1,
            Verdict::Stale => stale += 1,
            Verdict::Held(_) => held += 1,
            Verdict::Dangling => dangling += 1,
        }
    }

    Ok(StatusResult {
        resources_labeled: labels.len(),
        events,
        intents,
        commitments_active,
        unfulfilled_total,
        top_unfulfilled: unfulfilled,
        edges_sealed: sealed,
        edges_governed: governed,
        edges_stale: stale,
        edges_held: held,
        edges_dangling: dangling,
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
        println!(
            "  edges: {} sealed · {} governed · {} stale · {} held · {} dangling",
            self.edges_sealed,
            self.edges_governed,
            self.edges_stale,
            self.edges_held,
            self.edges_dangling
        );
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
