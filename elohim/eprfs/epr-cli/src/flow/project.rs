//! `epr flow project` — derive FlowRecords from the repository filesystem and append the
//! new ones (deduped by CID) to the sidecar `FlowStore`, alongside a `labels.json` index
//! (spec §4, §5). Deterministic + idempotent: identity is the canonical body CID and all
//! timestamps come from git — never `now()`.

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use cid::Cid;
use elohim_epr_rea::{
    atom_cid, AgentRef, Commitment, CommitmentState, FlowEvent, FlowRecord, FlowStore, Intent,
    Magnitude, PinnedRef, Process, ProcessSpec, ReaVerb, ResourceSpec, SidecarFlowStore,
};
use serde::{Deserialize, Serialize};

use super::registry::{Recipe, Registry};
use super::{
    body_cid, body_cid_of_file, cite_path, parse_frontmatter, producing_commit, rel_to_root,
    repo_agent, repo_scope_atom, FlowResult, Labels, REPO_AGENT,
};

/// Stages whose artifacts are content-addressed doc/scenario resources.
const RESOURCE_STAGES: &[&str] = &[
    "manifesto",
    "epic",
    "architecture-seed",
    "spec",
    "plan",
    "scenario",
];

#[derive(Debug, Default, Serialize)]
pub struct KindCount {
    pub new: usize,
    pub present: usize,
}

#[derive(Debug, Serialize)]
pub struct ProjectSummary {
    pub recipes: usize,
    pub counts: BTreeMap<String, KindCount>,
    pub resources_labeled: usize,
    pub unresolvable_cites: usize,
    pub sidecar: String,
    pub labels_path: String,
}

impl ProjectSummary {
    pub fn render(&self, _root: &Path) {
        println!("epr flow project — {} recipe(s)", self.recipes);
        for kind in ["spec", "process", "event", "intent", "commitment"] {
            if let Some(c) = self.counts.get(kind) {
                println!(
                    "  {kind:11} {:>4} new  {:>4} already-present",
                    c.new, c.present
                );
            }
        }
        println!("  resources labeled: {}", self.resources_labeled);
        println!(
            "  unresolvable cite/refines locators: {}",
            self.unresolvable_cites
        );
        println!("  sidecar: {}", self.sidecar);
        println!("  labels:  {}", self.labels_path);
    }
}

/// A gap-items decompose file: proto-intents scoped to one source spec/plan.
#[derive(Debug, Deserialize)]
struct GapFile {
    doc: String,
    #[serde(default)]
    items: Vec<GapItem>,
}

#[derive(Debug, Deserialize)]
struct GapItem {
    #[serde(default)]
    id: String,
    #[serde(default = "default_state")]
    state: String,
}

fn default_state() -> String {
    "OPEN".to_string()
}

/// A record staged for append, plus the label to give its CID.
struct Staged {
    record: FlowRecord,
    label: Option<(String, String)>,
}

/// Accumulates derived records + labels while tracking unresolvable locators.
#[derive(Default)]
struct Derivation {
    staged: Vec<Staged>,
    labels: Labels,
    unresolvable: usize,
}

impl Derivation {
    fn label(&mut self, cid: &Cid, value: impl Into<String>) {
        self.labels.insert(cid.to_string(), value.into());
    }

    fn stage_record(&mut self, record: FlowRecord, cid: Cid, label: impl Into<String>) {
        self.staged.push(Staged {
            record,
            label: Some((cid.to_string(), label.into())),
        });
    }
}

pub fn project(root: &Path, recipes: &Path) -> FlowResult<ProjectSummary> {
    let registry = Registry::load(recipes)?;
    let mut deriv = Derivation::default();
    let repo_scope = repo_scope_atom()?;
    deriv.label(&repo_scope, REPO_AGENT);

    for recipe in &registry.recipes {
        derive_recipe(root, recipe, &repo_scope, &mut deriv)?;
    }

    // Dedupe against the existing sidecar, then append the new records.
    let mut store = SidecarFlowStore::open(root)?;
    let existing: HashSet<Cid> = store.records()?.into_iter().map(|(c, _)| c).collect();
    let mut seen: HashSet<Cid> = HashSet::new();
    let mut counts: BTreeMap<String, KindCount> = BTreeMap::new();

    for staged in deriv.staged {
        let cid = staged.record.cid()?;
        let kind = kind_name(&staged.record).to_string();
        let entry = counts.entry(kind).or_default();
        if existing.contains(&cid) || seen.contains(&cid) {
            entry.present += 1;
        } else {
            store.append(staged.record)?;
            seen.insert(cid);
            entry.new += 1;
        }
        if let Some((k, v)) = staged.label {
            deriv.labels.insert(k, v);
        }
    }

    // labels.json is an operational index — overwritten whole each run.
    let labels_path = root.join(".eprfs").join("status").join("labels.json");
    std::fs::write(&labels_path, serde_json::to_string_pretty(&deriv.labels)?)?;

    Ok(ProjectSummary {
        recipes: registry.recipes.len(),
        resources_labeled: deriv.labels.len(),
        counts,
        unresolvable_cites: deriv.unresolvable,
        sidecar: store.log_path().display().to_string(),
        labels_path: labels_path.display().to_string(),
    })
}

fn derive_recipe(
    root: &Path,
    recipe: &Recipe,
    repo_scope: &Cid,
    deriv: &mut Derivation,
) -> FlowResult<()> {
    // Knowledge plane: the ProcessSpec atom (paths excluded from the hash).
    let spec: ProcessSpec = recipe.to_process_spec();
    let spec_cid = atom_cid(&spec)?;
    deriv.stage_record(
        FlowRecord::Spec(spec),
        spec_cid,
        format!("recipe:{}@{}", recipe.id, recipe.version),
    );

    // Resource labels for every doc/scenario artifact under the resource stages.
    for stage_name in RESOURCE_STAGES {
        let Some(stage) = recipe.stage(stage_name) else {
            continue;
        };
        for (rel, abs) in stage_files(root, &stage.paths) {
            if let Some(cid) = body_cid_of_file(&abs) {
                deriv.label(&cid, rel);
            }
        }
    }

    // Plan plane, part 1: spec/plan docs with an `id:` → Process instance + Produce event.
    for stage_name in ["spec", "plan"] {
        if let Some(stage) = recipe.stage(stage_name) {
            for (rel, abs) in stage_files(root, &stage.paths) {
                derive_process_doc(root, recipe, &rel, &abs, repo_scope, deriv)?;
            }
        }
    }

    // Plan plane, part 2: gap-items → Intents (+ Commitments for CLAIMED).
    if let Some(stage) = recipe.stage("intent") {
        for (_, abs) in stage_files(root, &stage.paths) {
            derive_gap_items(root, &abs, deriv)?;
        }
    }

    // Observation frontier: each scenario → an unfulfilled a2o commitment.
    if let Some(stage) = recipe.stage("scenario") {
        for (rel, abs) in stage_files(root, &stage.paths) {
            derive_scenario(&rel, &abs, repo_scope, deriv)?;
        }
    }

    Ok(())
}

fn derive_process_doc(
    root: &Path,
    recipe: &Recipe,
    rel: &str,
    abs: &Path,
    repo_scope: &Cid,
    deriv: &mut Derivation,
) -> FlowResult<()> {
    let Ok(text) = std::fs::read_to_string(abs) else {
        return Ok(());
    };
    let fm = parse_frontmatter(&text);
    if fm.get("id").is_none() {
        return Ok(());
    }
    let own_cid = body_cid(&text);
    deriv.label(&own_cid, rel);

    let mut inputs: Vec<Cid> = Vec::new();

    // Resolved cites (entries carrying an existing `path:` locator).
    for entry in fm.list("cites") {
        match cite_path(entry) {
            Some(path) if root.join(&path).exists() => {
                if let Some(cid) = body_cid_of_file(&root.join(&path)) {
                    push_unique(&mut inputs, cid);
                    deriv.label(&cid, path);
                } else {
                    deriv.unresolvable += 1;
                }
            }
            _ => deriv.unresolvable += 1,
        }
    }

    // `refines:` targets are also inputs; the first resolvable one is the scope.
    let mut scope: Option<Cid> = None;
    for target in fm.list("refines") {
        let target_abs = root.join(target);
        if let Some(cid) = body_cid_of_file(&target_abs) {
            push_unique(&mut inputs, cid);
            deriv.label(&cid, target.clone());
            if scope.is_none() {
                scope = Some(cid);
            }
        } else {
            deriv.unresolvable += 1;
        }
    }
    let in_scope_of = scope.unwrap_or(*repo_scope);

    let process = Process {
        spec: PinnedRef {
            id: recipe.id.clone(),
            version: recipe.version,
        },
        in_scope_of,
        inputs,
        outputs: vec![own_cid],
    };
    let process_cid = atom_cid(&process)?;
    deriv.stage_record(
        FlowRecord::Process(process),
        process_cid,
        format!("process:{rel}"),
    );

    // The Produce event — provenance is the commit that added the file.
    let (provider, occurred_at) = producing_commit(root, rel)
        .map(|(email, ts)| (AgentRef(email), ts))
        .unwrap_or_else(|| (repo_agent(), String::new()));
    let event = FlowEvent {
        action: ReaVerb::Produce,
        provider,
        receiver: repo_agent(),
        resource: own_cid,
        quantity: Magnitude::Count {
            value: 1.0,
            unit: "artifact".to_string(),
        },
        process: Some(process_cid),
        in_scope_of,
        fulfills: Vec::new(),
        satisfies: Vec::new(),
        occurred_at,
    };
    let event_cid = atom_cid(&event)?;
    deriv.stage_record(
        FlowRecord::Event(event),
        event_cid,
        format!("produce:{rel}"),
    );
    Ok(())
}

fn derive_gap_items(root: &Path, abs: &Path, deriv: &mut Derivation) -> FlowResult<()> {
    let Ok(text) = std::fs::read_to_string(abs) else {
        return Ok(());
    };
    let gap: GapFile = match serde_json::from_str(&text) {
        Ok(g) => g,
        Err(_) => return Ok(()), // not a gap-items file shape; skip quietly
    };
    let source_abs = root.join(&gap.doc);
    let Some(scope_cid) = body_cid_of_file(&source_abs) else {
        deriv.unresolvable += 1;
        return Ok(());
    };
    deriv.label(&scope_cid, gap.doc.clone());

    for item in &gap.items {
        let classified_as = vec![
            format!("gap:{}", item.state.to_lowercase()),
            item.id.clone(),
        ];
        let resource_spec = ResourceSpec {
            classified_as: classified_as.clone(),
            quantity: None,
        };
        let intent = Intent {
            action: ReaVerb::Produce,
            resource_spec: resource_spec.clone(),
            in_scope_of: scope_cid,
            raised_by: AgentRef("tool:decompose".to_string()),
        };
        let intent_cid = atom_cid(&intent)?;
        deriv.stage_record(
            FlowRecord::Intent(intent),
            intent_cid,
            format!("intent:{}", item.id),
        );

        if item.state.eq_ignore_ascii_case("CLAIMED") {
            let commitment = Commitment {
                action: ReaVerb::Produce,
                provider: AgentRef("tool:decompose-claim".to_string()),
                receiver: repo_agent(),
                resource_spec,
                in_scope_of: scope_cid,
                valid_from: None,
                valid_until: None,
                state: CommitmentState::Active,
                satisfies: vec![intent_cid],
                bound: None,
            };
            let commitment_cid = atom_cid(&commitment)?;
            deriv.stage_record(
                FlowRecord::Commitment(commitment),
                commitment_cid,
                format!("commitment:claim:{}", item.id),
            );
        }
    }
    Ok(())
}

fn derive_scenario(
    rel: &str,
    abs: &Path,
    repo_scope: &Cid,
    deriv: &mut Derivation,
) -> FlowResult<()> {
    let Ok(text) = std::fs::read_to_string(abs) else {
        return Ok(());
    };
    let resource = body_cid(&text);
    deriv.label(&resource, rel);
    let commitment = Commitment {
        action: ReaVerb::Produce,
        provider: AgentRef("tool:a2o".to_string()),
        receiver: repo_agent(),
        resource_spec: ResourceSpec {
            classified_as: vec!["a2o:scenario-green".to_string(), rel.to_string()],
            quantity: None,
        },
        in_scope_of: *repo_scope,
        valid_from: None,
        valid_until: None,
        state: CommitmentState::Active,
        satisfies: Vec::new(),
        bound: None,
    };
    let commitment_cid = atom_cid(&commitment)?;
    deriv.stage_record(
        FlowRecord::Commitment(commitment),
        commitment_cid,
        format!("commitment:scenario:{rel}"),
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Expand a stage's repo-relative globs into `(rel, abs)` file pairs (deduped, files only).
fn stage_files(root: &Path, patterns: &[String]) -> Vec<(String, PathBuf)> {
    let mut seen: HashSet<PathBuf> = HashSet::new();
    let mut out = Vec::new();
    for pattern in patterns {
        let full = root.join(pattern);
        let Some(pat) = full.to_str() else { continue };
        let Ok(paths) = glob::glob(pat) else { continue };
        for entry in paths.flatten() {
            if entry.is_file() && seen.insert(entry.clone()) {
                out.push((rel_to_root(root, &entry), entry));
            }
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn push_unique(list: &mut Vec<Cid>, cid: Cid) {
    if !list.contains(&cid) {
        list.push(cid);
    }
}

fn kind_name(record: &FlowRecord) -> &'static str {
    match record {
        FlowRecord::Spec(_) => "spec",
        FlowRecord::Process(_) => "process",
        FlowRecord::Event(_) => "event",
        FlowRecord::Intent(_) => "intent",
        FlowRecord::Commitment(_) => "commitment",
        // `project` never stages an Edge (those come from the seal verbs), but the match
        // must stay exhaustive over `FlowRecord`.
        FlowRecord::Edge(_) => "edge",
    }
}
