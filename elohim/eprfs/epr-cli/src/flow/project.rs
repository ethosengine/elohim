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

    // Observation plane, the OTHER half: artifacts that LEFT a stage → Consume events.
    derive_absorption(root, recipe, repo_scope, deriv)?;

    Ok(())
}

/// Artifacts that left a resource stage → `Consume` events. The sink half of the projection.
///
/// **This function exists because the projection had no sink at all.** Every event minted above
/// is `ReaVerb::Produce`; `Consume` is in the vocabulary and was never emitted, so the
/// repository's own ValueFlows model of its own development was source-side only. Meadows:
/// natural capital is used unsustainably if sources decline **or sinks fill** — and a capacity
/// declaration that names only a source is half a model. Concretely, it meant the doc corpus had
/// an inflow and no modelled outflow, so its turnover time was unbounded *by construction*
/// rather than by observation, and nothing could tell those two apart.
///
/// # What counts as absorption, and the distinction that needs the recipe
///
/// A deletion is absorption. A rename is the interesting case, and it is where the pre-existing
/// `git log --diff-filter=DR` heuristic in the session-start instrument is simply wrong: it
/// counts a move to `held/` (real absorption — the doc left the plate) and an in-place rename
/// (not absorption at all — same doc, same stage, new filename) as the same event. Nothing in
/// git distinguishes them, which is why that instrument had to keep its estimate wide.
///
/// The recipe does distinguish them. A rename is absorption **iff its destination no longer
/// matches any resource stage's `paths:` globs** — the artifact left the value chain rather than
/// moving inside it. That is placement knowledge, it lives in the registry's binding plane, and
/// it turns a wide estimate into a witnessed count. This is the recipe doing real work rather
/// than being decoration on a folder layout.
///
/// # Identity
///
/// The resource CID is the canonical body CID **as of the parent of the removing commit** — the
/// last revision at which the artifact existed. That is the same identity its `Produce` event
/// carries whenever the body was unchanged between authoring and removal, so the two events fold
/// against one resource. When the body was edited in between they are different CIDs, which is
/// correct and not a bug: content addressing means the thing that was removed is the version
/// that was there, and a stock counting `produce - consume` over a *versioned* corpus is a
/// question this projection does not yet answer. Recorded as a known limit rather than papered
/// over — see the level's own basis in `elohim_epr_rea::stock`.
fn derive_absorption(
    root: &Path,
    recipe: &Recipe,
    repo_scope: &Cid,
    deriv: &mut Derivation,
) -> FlowResult<()> {
    let patterns = resource_stage_patterns(recipe);
    if patterns.is_empty() {
        return Ok(());
    }
    let mut pathspecs: Vec<String> = Vec::new();
    for stage_name in RESOURCE_STAGES {
        if let Some(stage) = recipe.stage(stage_name) {
            pathspecs.extend(stage.paths.iter().cloned());
        }
    }

    for removal in git_removals(root, &pathspecs) {
        // A rename INSIDE the value chain is not absorption. This is the discrimination the
        // git-only heuristic cannot make and the recipe can.
        if let Some(dest) = &removal.moved_to {
            if patterns.iter().any(|p| p.matches_with(dest, SEGMENT_WISE)) {
                continue;
            }
        }
        let Some(text) = git_show(root, &format!("{}^:{}", removal.commit, removal.path)) else {
            continue; // root commit, or the path was not in the parent tree — no identity to mint
        };
        let resource = body_cid(&text);
        deriv.label(&resource, removal.path.clone());
        let event = FlowEvent {
            action: ReaVerb::Consume,
            provider: AgentRef(removal.author.clone()),
            receiver: repo_agent(),
            resource,
            quantity: Magnitude::Count {
                value: 1.0,
                unit: "artifact".to_string(),
            },
            // No Process: absorption is not a recipe stage transition. A doc leaving the plate
            // is the chain ENDING for that artifact, and inventing a process to hang it on would
            // assert a stage that never ran.
            process: None,
            in_scope_of: *repo_scope,
            fulfills: Vec::new(),
            satisfies: Vec::new(),
            classified_as: Vec::new(),
            occurred_at: removal.occurred_at.clone(),
        };
        let event_cid = atom_cid(&event)?;
        deriv.stage_record(
            FlowRecord::Event(event),
            event_cid,
            format!("consume:{}", removal.path),
        );
    }
    Ok(())
}

/// `*` stops at a path separator, which is NOT the `glob` crate's default and is load-bearing
/// here rather than a style preference.
///
/// With the default (`require_literal_separator: false`) a `*` crosses `/`, so the stage glob
/// `docs/*.md` matches `docs/held/moved.md` — and a move into `held/` would be classified as a
/// rename INSIDE the value chain and silently dropped. That is the single most common real
/// absorption event in this repo (`scope-reconcile --apply` git-mv's docs to `held/` when a
/// capability goes down), so the default would have made the outflow read near-zero while
/// looking like it worked. Caught by the fixture below, which is why the fixture builds all
/// three removal shapes instead of just a delete.
const SEGMENT_WISE: glob::MatchOptions = glob::MatchOptions {
    case_sensitive: true,
    require_literal_separator: true,
    require_literal_leading_dot: false,
};

/// Compiled `paths:` globs across every resource stage — "still on the plate".
fn resource_stage_patterns(recipe: &Recipe) -> Vec<glob::Pattern> {
    let mut out = Vec::new();
    for stage_name in RESOURCE_STAGES {
        let Some(stage) = recipe.stage(stage_name) else {
            continue;
        };
        for p in &stage.paths {
            if let Ok(pat) = glob::Pattern::new(p) {
                out.push(pat);
            }
        }
    }
    out
}

/// One artifact leaving a stage: deleted, or renamed to `moved_to`.
struct Removal {
    commit: String,
    author: String,
    occurred_at: String,
    path: String,
    moved_to: Option<String>,
}

/// `git log --diff-filter=DR --name-status` over the resource-stage pathspecs.
///
/// Deterministic and history-derived like every other timestamp on this path — never `now()`.
/// A git failure yields an empty list rather than an error: absorption we cannot see is honest
/// absence (the stock's outflow reads zero and its turnover reads *unknown*), whereas failing
/// the whole projection because history is unreadable would take the source side down with it.
fn git_removals(root: &Path, pathspecs: &[String]) -> Vec<Removal> {
    const REC: char = '\u{1}';
    let mut args: Vec<String> = vec![
        "log".into(),
        "--diff-filter=DR".into(),
        "--name-status".into(),
        "-M".into(),
        format!("--format={REC}%H%x1f%ae%x1f%aI"),
        "--".into(),
    ];
    args.extend(pathspecs.iter().cloned());
    let argv: Vec<&str> = args.iter().map(String::as_str).collect();
    let Ok(out) = crate::process::build_command("git", &argv, root, &[]).output() else {
        return Vec::new();
    };
    if !out.status.success() {
        return Vec::new();
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let (mut commit, mut author, mut ts) = (String::new(), String::new(), String::new());
    let mut removals = Vec::new();
    for line in text.lines() {
        if let Some(header) = line.strip_prefix(REC) {
            let mut parts = header.split('\u{1f}');
            commit = parts.next().unwrap_or_default().to_string();
            author = parts.next().unwrap_or_default().to_string();
            ts = parts.next().unwrap_or_default().to_string();
            continue;
        }
        let mut cols = line.split('\t');
        let Some(status) = cols.next() else { continue };
        let Some(path) = cols.next() else { continue };
        let moved_to = cols.next().map(str::to_string);
        let is_removal = status.starts_with('D') || status.starts_with('R');
        if !is_removal || commit.is_empty() {
            continue;
        }
        removals.push(Removal {
            commit: commit.clone(),
            author: author.clone(),
            occurred_at: ts.clone(),
            path: path.to_string(),
            moved_to,
        });
    }
    removals
}

/// `git show <rev>:<path>` — the artifact's bytes at the revision before it left.
fn git_show(root: &Path, spec: &str) -> Option<String> {
    let out = crate::process::build_command("git", &["show", spec], root, &[])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
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
        classified_as: Vec::new(),
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
