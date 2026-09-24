//! `epr flow walk <path>` and `epr flow status` — read the sidecar `FlowStore` and render
//! the developer value chain: lineage (walk_back) and frontier (walk_forward + the
//! directly-scoped, still-open intents/commitments a change here leaves on the plate).

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Duration, Utc};
use cid::Cid;
use elohim_epr_rea::{
    Commitment, CommitmentState, FlowRecord, FlowStore, FlowWalk, Frontier, Intent, Lineage,
    Process, SidecarFlowStore,
};
use serde::Serialize;

use super::cluster_state;
use super::edges::{edge_verdict, governor_label, EdgeIndex, IndexedEdge, Verdict};
use super::registers;
use super::report::{self, BoundOutcome};
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
    /// The habit this commitment is accounted to (its `habit:` slot), when `--serves` named one.
    /// Absent — not null — on an unaccounted commitment, so a payload with nothing to say about
    /// evidence stays byte-identical to the one before these columns existed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub habit: Option<String>,
    /// The highest rung of the evidence ladder the habit's proof is green at, and when.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tier: Option<TierMark>,
    /// What the habit's path currently costs: the `cost:`-slot bounds whose `concern:` is it.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub cost: Vec<CostMark>,
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
    /// Present when the target is a habit atom (`*.habit.md`): the habit's tier, cost, and every
    /// commitment accounted to it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub habit: Option<HabitView>,
    /// Compiler-format findings (`ERROR dangling-proof …`, `WARN orphan …`,
    /// `ERROR stale-claim …`) about the target's proof and the claims standing on it.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<Diagnostic>,
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
        habit: habit_slot(c).map(ToString::to_string),
        tier: None,
        cost: Vec::new(),
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
    let store = SidecarFlowStore::open(root)?;
    let records = store.records()?;
    let mut result = walk_with_records(root, rel_path, &records)?;
    attach_evidence(root, &records, &mut result, Utc::now())?;
    Ok(result)
}

/// Read-only adapter: all existing walk algorithms see the caller's single snapshot.
struct Snapshot<'a>(&'a [(Cid, FlowRecord)]);
impl FlowStore for Snapshot<'_> {
    fn append(&mut self, _record: FlowRecord) -> elohim_epr_rea::Result<Cid> {
        Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "read-only flow snapshot",
        )
        .into())
    }
    fn records(&self) -> elohim_epr_rea::Result<Vec<(Cid, FlowRecord)>> {
        Ok(self.0.to_vec())
    }
}

pub(crate) fn walk_with_records(
    root: &Path,
    rel_path: &str,
    records: &[(Cid, FlowRecord)],
) -> FlowResult<WalkResult> {
    let abs = if Path::new(rel_path).is_absolute() {
        std::path::PathBuf::from(rel_path)
    } else {
        root.join(rel_path)
    };
    let target = rel_to_root(root, &abs);
    let cid =
        body_cid_of_file(&abs).ok_or_else(|| FlowError::UnknownResource(rel_path.to_string()))?;

    let store = Snapshot(records);
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
        habit: None,
        diagnostics: Vec::new(),
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
                    "      ✗ {} [{}]{}",
                    c.commitment.label,
                    c.classified_as.join(", "),
                    evidence_suffix(c)
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

        if let Some(habit) = &self.habit {
            habit.render();
        }
        if !self.diagnostics.is_empty() {
            println!();
            for d in &self.diagnostics {
                println!("{}", d.line());
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
    /// Compiler-format findings over every declared habit and every standing claim.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<Diagnostic>,
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

    let mut evidence = Evidence::new(root);
    for view in &mut unfulfilled {
        evidence.mark(view)?;
    }
    let habits = registers::read_habits(root).unwrap_or_default();
    let declared: BTreeSet<String> = habits.iter().map(|h| h.id.clone()).collect();
    let mut diagnostics = Vec::new();
    for habit in &habits {
        let checks = HabitChecks {
            id: habit.id.clone(),
            checks: habit.checks.clone(),
        };
        diagnostics.extend(dangling_proofs(root, &checks));
        diagnostics.extend(orphan_proofs(&checks, evidence.features()));
    }
    diagnostics.extend(claim_diagnostics(
        &labels,
        &records,
        None,
        &declared,
        Utc::now(),
    ));

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
        diagnostics,
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
                    "    ✗ {} [{}]{}",
                    c.commitment.label,
                    c.classified_as.join(", "),
                    evidence_suffix(c)
                );
            }
        }
        for d in &self.diagnostics {
            println!("{}", d.line());
        }
    }
}

// ---------------------------------------------------------------------------
// evidence: tier, cost, and compiler-format diagnostics
// (evidence-ladder spec §5, increment 3 — "the one command")
// ---------------------------------------------------------------------------

/// The claim tag `epr flow claim` writes into slot 0 of every claim commitment.
const CLAIM_TAG: &str = "gap:claimed";
/// The `--serves` accounting slot `epr flow claim` writes.
const HABIT_SLOT_PREFIX: &str = "habit:";
/// An active claim older than this with no fulfilling event is a claim nobody is working.
///
/// Two weeks: one sprint plus its review. A claim is a promise dated by the tree it was made
/// against, and a promise that outlives the sprint that made it is the "left off" the walker
/// exists to surface.
pub const STALE_CLAIM_DAYS: i64 = 14;

/// The a2o receipt directory, relative to the repository root.
const REPORTS_REL: &str = "genesis/a2o/reports";
/// The a2o feature tree, relative to the repository root.
const FEATURES_REL: &str = "genesis/a2o/features";
/// Act I's lane contract — what the household mesh (T2) can promise.
const HOUSEHOLD_LANE_REL: &str = "genesis/manifests/cluster-state.act1-household.yaml";
/// The live substrate declaration — what a local run against the deployed fleet (T3) can reach.
const LIVE_LANE_REL: &str = "genesis/manifests/cluster-state.yaml";

/// The evidence ladder's rungs, low to high (evidence-ladder spec §3).
pub const RUNGS: [(&str, &str); 5] = [
    ("T0", "compiler"),
    ("T1", "in-process"),
    ("T2", "household"),
    ("T3", "live-local"),
    ("T4", "ci"),
];

/// The rungs this walker does not yet read receipts for. Gate runs (T0) and in-process suites
/// (T1) leave no content-addressed receipt a walker can join to a concern, so the tier column
/// says so rather than implying they were read and found wanting.
const UNREAD_RUNGS: [&str; 2] = ["T0", "T1"];

fn rung_label(rung: &str) -> &'static str {
    RUNGS
        .iter()
        .find(|(r, _)| *r == rung)
        .map(|(_, l)| *l)
        .unwrap_or("unknown")
}

fn rung_index(rung: &str) -> usize {
    RUNGS.iter().position(|(r, _)| *r == rung).unwrap_or(0)
}

/// A commitment's habit, read from its `habit:<id>` slot.
fn habit_slot(c: &Commitment) -> Option<&str> {
    c.resource_spec
        .classified_as
        .iter()
        .find_map(|slot| slot.strip_prefix(HABIT_SLOT_PREFIX))
        .filter(|id| !id.is_empty())
}

/// The compact tier a commitment carries: the habit's highest green rung and when it went green.
///
/// Present on every habit-accounted commitment. `rung: null` is a READING — no rung's newest
/// receipt is green — and `runnable_from` still says where the proof could first go green.
#[derive(Debug, Clone, Serialize)]
pub struct TierMark {
    pub rung: Option<String>,
    pub label: Option<String>,
    pub when: Option<String>,
    pub runnable_from: Option<String>,
}

/// One cost reading in compact form — the `cost:`-slot bound's latest outcome.
#[derive(Debug, Clone, Serialize)]
pub struct CostMark {
    pub measure: String,
    pub outcome: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observed: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hard: Option<f64>,
}

impl CostMark {
    fn of(outcome: &BoundOutcome) -> Self {
        Self {
            measure: outcome.measure.clone(),
            outcome: outcome.outcome.as_str().to_string(),
            observed: outcome.observed,
            unit: outcome.unit.clone(),
            hard: outcome.watermarks.hard,
        }
    }
}

/// The newest a2o receipt reading for a concern at one rung.
#[derive(Debug, Clone, Serialize)]
pub struct RungReading {
    pub rung: String,
    pub label: String,
    /// Every scenario of the concern passed in this receipt (and at least one ran).
    pub green: bool,
    pub passed: u64,
    pub failed: u64,
    /// The receipt's `generatedAt`.
    pub when: String,
    /// Repository-relative path of the receipt.
    pub receipt: String,
}

/// Where a habit's proof stands on the evidence ladder.
#[derive(Debug, Clone, Serialize)]
pub struct TierView {
    /// The highest rung whose NEWEST receipt is green; `None` when no rung reads green.
    pub highest_green: Option<RungReading>,
    /// The newest reading at every rung that has one, low to high — a red above a green is the
    /// ladder's "a T4 red must be reproduced lower" signal, so it is shown, not folded away.
    pub rungs: Vec<RungReading>,
    /// The a2o feature files that carry `@concern:<habit>`.
    pub scenarios: Vec<String>,
    /// The `@requires:` capabilities those files declare.
    pub requires: Vec<String>,
    /// The lowest rung any of those files can run at, from its `@requires:` tags read against the
    /// household lane contract and the live substrate declaration. `None` with no scenario.
    pub runnable_from: Option<String>,
    /// Rungs this walker does not read receipts for (see [`UNREAD_RUNGS`]).
    pub unread: Vec<String>,
}

impl TierView {
    fn mark(&self) -> TierMark {
        let green = self.highest_green.as_ref();
        TierMark {
            rung: green.map(|r| r.rung.clone()),
            label: green.map(|r| r.label.clone()),
            when: green.map(|r| r.when.clone()),
            runnable_from: self.runnable_from.clone(),
        }
    }
}

/// A build attestation noted on HEAD (`refs/notes/brit/build/<step>`, brit's build refs).
#[derive(Debug, Clone, Serialize)]
pub struct AttestedBuild {
    pub step: String,
    pub build_duration_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub built_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success: Option<bool>,
}

/// What a habit's delivery path costs.
#[derive(Debug, Clone, Serialize)]
pub struct CostView {
    /// The `cost:`-slot bounds whose `concern:` names this habit, evaluated exactly as the
    /// SessionStart headline evaluates them.
    pub bounds: Vec<BoundOutcome>,
    /// Build attestations noted on the walked tree's HEAD, when any exist.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub attested_builds: Vec<AttestedBuild>,
    /// Their summed wall clock, when any exist.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attested_build_ms: Option<u64>,
}

/// A habit atom, walked: its standing, its evidence, its price, and the work accounted to it.
#[derive(Debug, Serialize)]
pub struct HabitView {
    pub id: String,
    pub status: String,
    pub tier: TierView,
    pub cost: CostView,
    /// Every commitment whose `habit:` slot names this habit, with its fulfilment.
    pub serving: Vec<ServingView>,
}

#[derive(Debug, Serialize)]
pub struct ServingView {
    #[serde(flatten)]
    pub commitment: CommitmentView,
    pub fulfilled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid_from: Option<String>,
}

impl HabitView {
    fn render(&self) {
        println!("\n  HABIT {} [{}]", self.id, self.status);
        match &self.tier.highest_green {
            Some(r) => println!(
                "    tier: {} {} green at {} ({})",
                r.rung, r.label, r.when, r.receipt
            ),
            None => println!("    tier: no rung reads green"),
        }
        for r in &self.tier.rungs {
            println!(
                "      {} {:<10} {} · {} passed · {} failed · {}",
                r.rung,
                r.label,
                if r.green { "green" } else { "red" },
                r.passed,
                r.failed,
                r.when
            );
        }
        if let Some(from) = &self.tier.runnable_from {
            println!(
                "    runnable from {} ({} scenario file(s); @requires: {})",
                from,
                self.tier.scenarios.len(),
                if self.tier.requires.is_empty() {
                    "none".to_string()
                } else {
                    self.tier.requires.join(", ")
                }
            );
        }
        println!("    unread rungs: {}", self.tier.unread.join(", "));
        if self.cost.bounds.is_empty() {
            println!("    cost: no cost bound names this habit as its concern");
        }
        for b in &self.cost.bounds {
            println!(
                "    cost: {} {} — {}",
                b.measure,
                b.outcome.as_str(),
                b.summary
            );
        }
        if let Some(ms) = self.cost.attested_build_ms {
            println!(
                "    cost: {} attested build(s) on HEAD, {:.1} min",
                self.cost.attested_builds.len(),
                ms as f64 / 60_000.0
            );
        }
        if !self.serving.is_empty() {
            println!("    serving commitments ({}):", self.serving.len());
            for s in &self.serving {
                println!(
                    "      {} {} [{}]",
                    if s.fulfilled { "✓" } else { "✗" },
                    s.commitment.commitment.label,
                    s.commitment.classified_as.join(", ")
                );
            }
        }
    }
}

/// One compiler-format finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Diagnostic {
    /// `ERROR` | `WARN`.
    pub severity: String,
    /// `dangling-proof` | `orphan` | `stale-claim`.
    pub code: String,
    /// What the finding is about — a path, a gap id, or a commitment label.
    pub subject: String,
    pub message: String,
}

impl Diagnostic {
    fn new(severity: &str, code: &str, subject: impl Into<String>, message: String) -> Self {
        Self {
            severity: severity.to_string(),
            code: code.to_string(),
            subject: subject.into(),
            message,
        }
    }

    /// `ERROR dangling-proof <subject>: <message>` — one line, grep-able by severity and code.
    pub fn line(&self) -> String {
        format!(
            "{} {} {}: {}",
            self.severity, self.code, self.subject, self.message
        )
    }
}

fn evidence_suffix(c: &CommitmentView) -> String {
    let mut out = String::new();
    if let Some(t) = &c.tier {
        match (&t.rung, &t.label, &t.when) {
            (Some(rung), Some(label), Some(when)) => {
                out.push_str(&format!(" · tier {rung} {label} green {when}"));
            }
            _ => out.push_str(&format!(
                " · tier none green (runnable from {})",
                t.runnable_from.as_deref().unwrap_or("no scenario")
            )),
        }
    }
    for m in &c.cost {
        let name = m.measure.split('@').next().unwrap_or_default();
        match (m.observed, &m.unit) {
            (Some(v), Some(u)) => out.push_str(&format!(" · {name} {v} {u} {}", m.outcome)),
            _ => out.push_str(&format!(" · {name} {}", m.outcome)),
        }
    }
    out
}

/// One a2o feature file's tags.
#[derive(Debug, Clone)]
struct FeatureTags {
    rel: String,
    concerns: BTreeSet<String>,
    requires: BTreeSet<String>,
}

/// The lazily-built readings a walk joins its commitments to. Nothing is read until a habit asks.
pub(crate) struct Evidence {
    root: PathBuf,
    receipts: Option<BTreeMap<String, BTreeMap<String, RungReading>>>,
    features: Option<Vec<FeatureTags>>,
    costs: Option<BTreeMap<String, Vec<BoundOutcome>>>,
    tiers: BTreeMap<String, TierView>,
}

impl Evidence {
    pub(crate) fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            receipts: None,
            features: None,
            costs: None,
            tiers: BTreeMap::new(),
        }
    }

    fn features(&mut self) -> &[FeatureTags] {
        if self.features.is_none() {
            self.features = Some(read_features(&self.root));
        }
        self.features.as_deref().unwrap_or_default()
    }

    fn tier(&mut self, habit: &str) -> TierView {
        if let Some(t) = self.tiers.get(habit) {
            return t.clone();
        }
        if self.receipts.is_none() {
            self.receipts = Some(read_receipts(&self.root));
        }
        let rungs: Vec<RungReading> = self
            .receipts
            .as_ref()
            .and_then(|r| r.get(habit))
            .map(|by_rung| {
                let mut v: Vec<RungReading> = by_rung.values().cloned().collect();
                v.sort_by_key(|r| rung_index(&r.rung));
                v
            })
            .unwrap_or_default();
        let highest_green = rungs.iter().rev().find(|r| r.green).cloned();
        let root = self.root.clone();
        let files: Vec<FeatureTags> = self
            .features()
            .iter()
            .filter(|f| f.concerns.contains(habit))
            .cloned()
            .collect();
        let requires: BTreeSet<String> = files.iter().flat_map(|f| f.requires.clone()).collect();
        let runnable_from = lowest_runnable(&root, &files);
        let view = TierView {
            highest_green,
            rungs,
            scenarios: files.iter().map(|f| f.rel.clone()).collect(),
            requires: requires.into_iter().collect(),
            runnable_from,
            unread: UNREAD_RUNGS.iter().map(ToString::to_string).collect(),
        };
        self.tiers.insert(habit.to_string(), view.clone());
        view
    }

    fn cost(&mut self, habit: &str) -> FlowResult<Vec<BoundOutcome>> {
        if self.costs.is_none() {
            self.costs = Some(report::cost_by_concern(&self.root)?);
        }
        Ok(self
            .costs
            .as_ref()
            .and_then(|c| c.get(habit))
            .cloned()
            .unwrap_or_default())
    }

    /// Fill a commitment's tier and cost from its habit. A commitment accounted to no habit is
    /// left untouched: there is no concern to read evidence for.
    pub(crate) fn mark(&mut self, view: &mut CommitmentView) -> FlowResult<()> {
        let Some(habit) = view.habit.clone() else {
            return Ok(());
        };
        view.tier = Some(self.tier(&habit).mark());
        view.cost = self.cost(&habit)?.iter().map(CostMark::of).collect();
        Ok(())
    }
}

/// Every `*.feature` under the a2o tree with its `@concern:` and `@requires:` tags.
///
/// Tags are read only from tag lines (a line whose first non-blank character is `@`), so prose
/// that mentions a tag in a comment or a step is not read as one.
fn read_features(root: &Path) -> Vec<FeatureTags> {
    let pattern = root.join(FEATURES_REL).join("**").join("*.feature");
    let Ok(paths) = glob::glob(&pattern.to_string_lossy()) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for path in paths.flatten() {
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let mut concerns = BTreeSet::new();
        let mut requires = BTreeSet::new();
        for line in text.lines() {
            let line = line.trim_start();
            if !line.starts_with('@') {
                continue;
            }
            for token in line.split_whitespace() {
                if let Some(c) = token.strip_prefix("@concern:") {
                    if !c.is_empty() {
                        concerns.insert(c.to_string());
                    }
                } else if let Some(r) = token.strip_prefix("@requires:") {
                    let r = r.trim_end_matches(|ch: char| !(ch.is_alphanumeric() || ch == '-'));
                    if !r.is_empty() {
                        requires.insert(r.to_string());
                    }
                }
            }
        }
        if concerns.is_empty() {
            continue;
        }
        out.push(FeatureTags {
            rel: rel_to_root(root, &path),
            concerns,
            requires,
        });
    }
    out.sort_by(|a, b| a.rel.cmp(&b.rel));
    out
}

/// The lowest rung ANY of a concern's scenario files can run at.
///
/// A file whose `@requires:` capabilities the household lane contract does not withhold runs on
/// the household (T2): a cap Act I does not declare is a fixture precondition there, not a gate —
/// the same inert-cap rule the a2o runtime applies. A file the household cannot run but the live
/// substrate can is T3 (local cucumber against deployed doorways). Anything else waits for a CI
/// banking run (T4).
fn lowest_runnable(root: &Path, files: &[FeatureTags]) -> Option<String> {
    if files.is_empty() {
        return None;
    }
    let household = cluster_state::load(&root.join(HOUSEHOLD_LANE_REL));
    let live = cluster_state::load(&root.join(LIVE_LANE_REL));
    let withheld = |state: &cluster_state::ClusterState, caps: &BTreeSet<String>| {
        let declared = state.declared_names();
        let available = state.available_names();
        caps.iter()
            .any(|c| declared.contains(c) && !available.contains(c))
    };
    files
        .iter()
        .map(|f| {
            if !withheld(&household, &f.requires) {
                "T2"
            } else if !withheld(&live, &f.requires) {
                "T3"
            } else {
                "T4"
            }
        })
        .min_by_key(|r| rung_index(r))
        .map(ToString::to_string)
}

/// The rung a sprint report was measured at, or `None` when its lane names no rung.
///
/// A report that carries CI provenance (`ci: true` or a `buildUrl`, top level or in `env`) is
/// banked CI evidence (T4) whatever lane it ran on. Otherwise the lane decides: `household` is the
/// local mesh (T2), `alpha-fleet` a local run against deployed doorways (T3). An `unknown` lane is
/// evidence of no rung and is not read.
fn receipt_rung(report: &serde_json::Value) -> Option<&'static str> {
    let env = report.get("env");
    let ci = |v: Option<&serde_json::Value>| {
        v.is_some_and(|v| {
            v.get("ci").and_then(serde_json::Value::as_bool) == Some(true)
                || v.get("buildUrl")
                    .and_then(serde_json::Value::as_str)
                    .is_some()
        })
    };
    if ci(Some(report)) || ci(env) {
        return Some("T4");
    }
    match env
        .and_then(|e| e.get("lane"))
        .and_then(serde_json::Value::as_str)
    {
        Some("household") => Some("T2"),
        Some("alpha-fleet") => Some("T3"),
        _ => None,
    }
}

/// concern → rung → the NEWEST receipt reading, from every `sprint-report*.json` under the a2o
/// reports tree (the receipts `build-sprint-report.ts` writes, `summary.byConcern` per concern).
fn read_receipts(root: &Path) -> BTreeMap<String, BTreeMap<String, RungReading>> {
    let mut out: BTreeMap<String, BTreeMap<String, RungReading>> = BTreeMap::new();
    let base = root.join(REPORTS_REL);
    let mut paths: Vec<PathBuf> = Vec::new();
    for pattern in [
        "sprint-report*.json",
        "*/sprint-report*.json",
        "*/*/sprint-report*.json",
    ] {
        if let Ok(found) = glob::glob(&base.join(pattern).to_string_lossy()) {
            paths.extend(found.flatten());
        }
    }
    for path in paths {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(report) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
            continue;
        };
        let Some(rung) = receipt_rung(&report) else {
            continue;
        };
        let when = report
            .get("generatedAt")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_string();
        if when.is_empty() {
            continue;
        }
        let Some(by_concern) = report
            .get("summary")
            .and_then(|s| s.get("byConcern"))
            .and_then(serde_json::Value::as_object)
        else {
            continue;
        };
        let rel = rel_to_root(root, &path);
        for (concern, counts) in by_concern {
            let n = |k: &str| {
                counts
                    .get(k)
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0)
            };
            let (passed, failed) = (n("passed"), n("failed"));
            if passed + failed == 0 {
                continue;
            }
            let reading = RungReading {
                rung: rung.to_string(),
                label: rung_label(rung).to_string(),
                green: failed == 0 && passed > 0,
                passed,
                failed,
                when: when.clone(),
                receipt: rel.clone(),
            };
            let slot = out.entry(concern.clone()).or_default();
            // ISO-8601 UTC stamps order lexically; the receipt path breaks a same-instant tie so
            // the reading is a function of the tree, not of glob order.
            let newer = slot.get(rung).is_none_or(|cur| {
                (reading.when.as_str(), reading.receipt.as_str())
                    > (cur.when.as_str(), cur.receipt.as_str())
            });
            if newer {
                slot.insert(rung.to_string(), reading);
            }
        }
    }
    out
}

/// Build attestations noted on HEAD under `refs/notes/brit/build/`.
///
/// brit writes one git note per step per commit (`BritRefManager::put_build_ref`), its payload
/// the camelCase `BuildAttestationContentNode`. Read-only: `git for-each-ref` then `git notes
/// show`. No note, no git, or an unparseable payload is simply no attestation.
fn attested_builds(root: &Path) -> Vec<AttestedBuild> {
    let run = |args: &[&str]| -> Option<String> {
        let out = crate::process::build_command("git", args, root, &[])
            .output()
            .ok()?;
        out.status
            .success()
            .then(|| String::from_utf8_lossy(&out.stdout).to_string())
    };
    let Some(refs) = run(&[
        "for-each-ref",
        "--format=%(refname)",
        "refs/notes/brit/build/",
    ]) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for refname in refs.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let Some(body) = run(&["notes", "--ref", refname, "show", "HEAD"]) else {
            continue;
        };
        let Ok(v) = serde_json::from_str::<serde_json::Value>(body.trim()) else {
            continue;
        };
        let Some(ms) = v
            .get("buildDurationMs")
            .or_else(|| v.get("build_duration_ms"))
            .and_then(serde_json::Value::as_u64)
        else {
            continue;
        };
        out.push(AttestedBuild {
            step: v
                .get("stepName")
                .and_then(serde_json::Value::as_str)
                .map(ToString::to_string)
                .unwrap_or_else(|| {
                    refname
                        .trim_start_matches("refs/notes/brit/build/")
                        .to_string()
                }),
            build_duration_ms: ms,
            built_at: v
                .get("builtAt")
                .and_then(serde_json::Value::as_str)
                .map(ToString::to_string),
            success: v.get("success").and_then(serde_json::Value::as_bool),
        });
    }
    out
}

/// A habit's id and checks — from its atom's frontmatter on a walk, from the register on status.
struct HabitChecks {
    id: String,
    checks: Vec<String>,
}

/// Read a habit atom's frontmatter: `(id, status, checks)`.
fn read_habit_atom(path: &Path) -> Option<(String, String, Vec<String>)> {
    let text = std::fs::read_to_string(path).ok()?;
    let rest = text.strip_prefix("---")?;
    let end = rest.find("\n---")?;
    let front: serde_yaml::Value = serde_yaml::from_str(&rest[..end]).ok()?;
    let id = front.get("id")?.as_str()?.trim().to_string();
    let status = front
        .get("status")
        .and_then(serde_yaml::Value::as_str)
        .unwrap_or_default()
        .to_string();
    let checks = front
        .get("checks")
        .and_then(serde_yaml::Value::as_sequence)
        .map(|seq| {
            seq.iter()
                .filter_map(serde_yaml::Value::as_str)
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default();
    Some((id, status, checks))
}

/// The repository paths a check line names: the path-shaped words inside its backtick spans.
///
/// A word is path-shaped when it holds a `/`, ends in a file extension, and is neither a URL, an
/// absolute or home path, a glob, nor a placeholder. Only backtick spans are read — prose around
/// them is description, and a check's runnable surface is its quoted command.
fn check_paths(check: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (i, span) in check.split('`').enumerate() {
        if i % 2 == 0 {
            continue;
        }
        for word in span.split_whitespace() {
            let word = word.trim_matches(|c: char| matches!(c, '"' | '\'' | ',' | ';' | ')' | '('));
            let word = word.split(':').next().unwrap_or_default();
            if !word.contains('/')
                || word.contains("://")
                || word.starts_with('/')
                || word.starts_with('~')
                || word.starts_with('-')
                || word.contains(['*', '{', '}', '<', '>', '$', '?', '[', ']'])
            {
                continue;
            }
            let has_extension = Path::new(word).extension().is_some_and(|e| {
                !e.is_empty() && e.to_string_lossy().chars().all(char::is_alphanumeric)
            });
            if has_extension {
                out.push(word.to_string());
            }
        }
    }
    out
}

/// `ERROR dangling-proof`: a check names a proof that is not on disk.
///
/// A path is resolved against the repository root and, failing that, against the a2o workspace
/// (`just test mesh features/…` names a feature relative to it).
fn dangling_proofs(root: &Path, habit: &HabitChecks) -> Vec<Diagnostic> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for (n, check) in habit.checks.iter().enumerate() {
        for path in check_paths(check) {
            if !seen.insert(path.clone()) {
                continue;
            }
            let exists = root.join(&path).exists() || root.join("genesis/a2o").join(&path).exists();
            if !exists {
                out.push(Diagnostic::new(
                    "ERROR",
                    "dangling-proof",
                    path.clone(),
                    format!(
                        "habit `{}` check {} names it, and it is not on disk",
                        habit.id,
                        n + 1
                    ),
                ));
            }
        }
    }
    out
}

/// `WARN orphan`: a scenario carries `@concern:<habit>`, and no check of that habit reaches it —
/// neither by path nor by the `@concern:<habit>` tag expression a scoped `just test mesh` takes.
/// The proof exists; the claim does not name it, so a green there moves nothing.
fn orphan_proofs(habit: &HabitChecks, features: &[FeatureTags]) -> Vec<Diagnostic> {
    let tag = format!("@concern:{}", habit.id);
    if habit.checks.iter().any(|c| c.contains(&tag)) {
        return Vec::new();
    }
    features
        .iter()
        .filter(|f| f.concerns.contains(&habit.id))
        .filter(|f| {
            let short = f.rel.trim_start_matches("genesis/a2o/");
            !habit
                .checks
                .iter()
                .any(|c| c.contains(&f.rel) || c.contains(short))
        })
        .map(|f| {
            Diagnostic::new(
                "WARN",
                "orphan",
                f.rel.clone(),
                format!(
                    "carries @concern:{} and no check of that habit names it",
                    habit.id
                ),
            )
        })
        .collect()
}

/// Claim findings over the sidecar, optionally restricted to one habit.
///
/// `ERROR stale-claim`: an ACTIVE claim (`gap:claimed`) no event fulfils, promised more than
/// [`STALE_CLAIM_DAYS`] before `now`. `WARN orphan`: a claim accounted to a habit the register no
/// longer declares — work whose standard is gone.
fn claim_diagnostics(
    labels: &Labels,
    records: &[(Cid, FlowRecord)],
    habit: Option<&str>,
    declared: &BTreeSet<String>,
    now: DateTime<Utc>,
) -> Vec<Diagnostic> {
    let discharged: HashSet<Cid> = records
        .iter()
        .filter_map(|(_, r)| match r {
            FlowRecord::Event(e) => Some(e.fulfills.clone()),
            _ => None,
        })
        .flatten()
        .collect();
    let mut out = Vec::new();
    for (cid, record) in records {
        let FlowRecord::Commitment(c) = record else {
            continue;
        };
        if c.resource_spec.classified_as.first().map(String::as_str) != Some(CLAIM_TAG) {
            continue;
        }
        let served = habit_slot(c);
        if habit.is_some() && served != habit {
            continue;
        }
        let gap = c
            .resource_spec
            .classified_as
            .get(1)
            .cloned()
            .unwrap_or_else(|| resolve(labels, cid).label);
        if let Some(h) = served {
            if !declared.is_empty() && !declared.contains(h) {
                out.push(Diagnostic::new(
                    "WARN",
                    "orphan",
                    gap.clone(),
                    format!("claim serves habit `{h}`, which the register no longer declares"),
                ));
            }
        }
        if c.state != CommitmentState::Active || discharged.contains(cid) {
            continue;
        }
        let Some(since) = c
            .valid_from
            .as_deref()
            .and_then(|v| DateTime::parse_from_rfc3339(v).ok())
        else {
            continue;
        };
        let age = now.signed_duration_since(since.with_timezone(&Utc));
        if age > Duration::days(STALE_CLAIM_DAYS) {
            out.push(Diagnostic::new(
                "ERROR",
                "stale-claim",
                gap,
                format!(
                    "claimed by {} on {}, {} days ago, and nothing fulfils it (bound {} days)",
                    c.provider.0,
                    since.format("%Y-%m-%d"),
                    age.num_days(),
                    STALE_CLAIM_DAYS
                ),
            ));
        }
    }
    out
}

/// Join the walk's commitments to their habits' evidence and, when the target is a habit atom,
/// build its [`HabitView`] and diagnostics. Nothing is read when there is nothing to join.
pub(crate) fn attach_evidence(
    root: &Path,
    records: &[(Cid, FlowRecord)],
    result: &mut WalkResult,
    now: DateTime<Utc>,
) -> FlowResult<()> {
    let labels = load_labels(root);
    let mut evidence = Evidence::new(root);
    for view in &mut result.frontier.unfulfilled_commitments {
        evidence.mark(view)?;
    }

    let atom = if result.target.ends_with(".habit.md") {
        read_habit_atom(&root.join(&result.target))
    } else {
        None
    };
    let Some((id, status, checks)) = atom else {
        // Not a habit: the only finding a plain walk owns is a stale claim on its own frontier.
        let frontier: BTreeSet<String> = result
            .frontier
            .unfulfilled_commitments
            .iter()
            .map(|c| c.commitment.cid.clone())
            .collect();
        let scoped: Vec<(Cid, FlowRecord)> = records
            .iter()
            .filter(|(cid, _)| frontier.contains(&cid.to_string()))
            .cloned()
            .collect();
        result.diagnostics = claim_diagnostics(&labels, &scoped, None, &BTreeSet::new(), now);
        return Ok(());
    };

    let discharged: HashSet<Cid> = records
        .iter()
        .filter_map(|(_, r)| match r {
            FlowRecord::Event(e) => Some(e.fulfills.clone()),
            _ => None,
        })
        .flatten()
        .collect();
    let mut serving = Vec::new();
    for (cid, record) in records {
        let FlowRecord::Commitment(c) = record else {
            continue;
        };
        if habit_slot(c) != Some(id.as_str()) {
            continue;
        }
        let mut view = commitment_view(&labels, cid, c);
        evidence.mark(&mut view)?;
        serving.push(ServingView {
            commitment: view,
            fulfilled: discharged.contains(cid) || matches!(c.state, CommitmentState::Fulfilled),
            valid_from: c.valid_from.clone(),
        });
    }

    let tier = evidence.tier(&id);
    let bounds = evidence.cost(&id)?;
    let attested = attested_builds(root);
    let attested_build_ms =
        (!attested.is_empty()).then(|| attested.iter().map(|a| a.build_duration_ms).sum());
    let habit_checks = HabitChecks {
        id: id.clone(),
        checks,
    };
    let declared: BTreeSet<String> = registers::read_habits(root)
        .map(|hs| hs.into_iter().map(|h| h.id).collect())
        .unwrap_or_default();
    let mut diagnostics = dangling_proofs(root, &habit_checks);
    diagnostics.extend(orphan_proofs(&habit_checks, evidence.features()));
    diagnostics.extend(claim_diagnostics(
        &labels,
        records,
        Some(&id),
        &declared,
        now,
    ));

    result.habit = Some(HabitView {
        id,
        status,
        tier,
        cost: CostView {
            bounds,
            attested_builds: attested,
            attested_build_ms,
        },
        serving,
    });
    result.diagnostics = diagnostics;
    Ok(())
}
