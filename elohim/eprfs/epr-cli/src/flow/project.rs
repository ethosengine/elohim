//! `epr flow project` — derive FlowRecords from the repository filesystem and append the
//! new ones (deduped by CID) to the sidecar `FlowStore`, alongside a `labels.json` index
//! (spec §4, §5). Deterministic + idempotent: identity is the canonical body CID and all
//! timestamps come from git — never `now()`.

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use cid::Cid;
use elohim_epr_rea::{
    atom_cid, AgentRef, Bound, Commitment, CommitmentState, FlowEvent, FlowRecord, FlowStore,
    Intent, Magnitude, PinnedRef, Process, ProcessSpec, ReaVerb, ResourceSpec, SidecarFlowStore,
};
use serde::{Deserialize, Serialize};

use super::registry::{Recipe, Registry};
use super::{
    body_cid, body_cid_of_file, cite_path, parse_frontmatter, producing_commit, rel_to_root,
    repo_agent, repo_scope_atom, FlowResult, Labels, REPO_AGENT,
};

/// What the WIP fence's ceiling is denominated in. Shared with the stock that is judged against
/// it, so the declaration and the measurement can never name two different things.
pub const WIP_FENCE_UNIT: &str = "active-habit";

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

    // The STANDARD plane. Not a recipe stage: the habit register is one file at one path and is
    // read whether or not any recipe names it, because the WIP fence is a property of the
    // repository's attention rather than of any one value chain.
    derive_wip_fence(root, &mut deriv)?;

    // Dedupe against the existing sidecar, then append the new records.
    let sidecar = SidecarFlowStore::open(root)?;
    let mut store = sidecar.transaction()?;
    let recorded = store.records()?;
    let existing: HashSet<Cid> = recorded.iter().map(|(cid, _)| *cid).collect();
    let existing_events: HashSet<EventKey> = recorded
        .iter()
        .filter_map(|(_, record)| match record {
            FlowRecord::Event(event) => Some(event_key(event)),
            _ => None,
        })
        .collect();
    let mut seen: HashSet<Cid> = HashSet::new();
    let mut counts: BTreeMap<String, KindCount> = BTreeMap::new();

    for staged in deriv.staged {
        let cid = staged.record.cid()?;
        let kind = kind_name(&staged.record).to_string();
        // Two questions, not one. "Have I already got this record?" is the CID; "have I already
        // got this ACT?" is the key. They diverge whenever the vocabulary an event carries grows
        // — the same production, re-derived, addresses differently.
        let already_present = existing.contains(&cid)
            || seen.contains(&cid)
            || matches!(&staged.record, FlowRecord::Event(event)
                if existing_events.contains(&event_key(event)));
        let entry = counts.entry(kind).or_default();
        if already_present {
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
        sidecar: sidecar.log_path().display().to_string(),
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
            ts = super::normalize_git_timestamp(parts.next().unwrap_or_default());
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
    //
    // `provider` stays the SIGNING author even when the commit names co-authors. The commit is
    // the steward's signed envelope: they are the one answerable for what entered the tree, and
    // moving the provider to a named collaborator would transfer that answerability to someone
    // who never signed anything. Plurality is additive — it lands in `classified_as` beside the
    // steward, never in place of them.
    let (provider, occurred_at, classified_as) = match producing_commit(root, rel) {
        Some(p) => (
            AgentRef(p.author),
            p.occurred_at,
            co_author_slots(&p.co_authors),
        ),
        None => (repo_agent(), String::new(), Vec::new()),
    };
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
        classified_as,
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

// ---------------------------------------------------------------------------
// Plural authorship — `Co-Authored-By` trailers → `classified_as` slots
// ---------------------------------------------------------------------------

/// Prefix on every slot naming someone the signing author worked with.
///
/// Prefixed like the other authored slots in this vocabulary so a reader can tell a *person*
/// from a tag at a glance, and so a fold that wants only the roster can select it by prefix
/// without knowing how many slots precede it.
const CO_AUTHOR_SLOT_PREFIX: &str = "co-author:";

/// Raw `Co-Authored-By` values → the sorted, deduped slots a `Produce` event carries.
///
/// Sorted because the slot list is part of the event's content address and git preserves the
/// order the author happened to type; two commits naming the same three collaborators in
/// different orders describe the same collaboration and must address the same. Deduped because a
/// repeated trailer, or two spellings that normalize to one identity, is one collaborator named
/// twice — not two.
///
/// An empty roster yields `Vec::new()`, which `FlowEvent::classified_as` skips when serializing.
/// That is what keeps every trailer-less commit's event byte-identical to what it was before this
/// vocabulary existed: the sidecar re-verifies each stored CID on read, so a re-encoding here
/// would turn the whole append-only log into an integrity error.
fn co_author_slots(raw: &[String]) -> Vec<String> {
    let mut slots: Vec<String> = raw
        .iter()
        .filter_map(|value| normalize_co_author(value))
        .map(|id| format!("{CO_AUTHOR_SLOT_PREFIX}{id}"))
        .collect();
    slots.sort();
    slots.dedup();
    slots
}

/// One `Name <email>` trailer value → the vocabulary term naming that collaborator, or `None`.
///
/// # This is vocabulary, not identity resolution
///
/// The domain arms are a **heuristic naming convention**, deliberately shallow. They say how this
/// repository has spelled its collaborators in commit trailers; they do not claim to have
/// resolved anyone to a durable identity, and nothing downstream may treat them as if they had.
/// The fallback is what makes that safe: an address from a domain this function has never heard
/// of becomes its own lowercased self, which misattributes nobody — it just declines to
/// interpret. Real resolution (`ContributorPresence`, the DID bridge) is a graduation-time
/// concern with witnesses and consent behind it, and will never be reachable by string-matching
/// an email domain here.
///
/// # The `agent:` form here is NOT an actor-plane ref
///
/// `agent:<name-slug>` carries no `@` half, so `parse_agent_ref` — which requires
/// `agent:<role>@<model>` — refuses it by construction. That structural disjointness is the
/// point: a trailer is an assertion the *committer* makes about who helped, while an actor claim
/// is an assertion an agent makes about *itself* in flight. Joining the two vocabularies by
/// string equality would let either one silently stand in for the other, so they are shaped so
/// that no comparison can succeed by accident.
///
/// # Failure is per-value
///
/// `None` for anything unparseable, never an error. A projection derives thousands of records
/// from history nobody can go back and fix; one malformed trailer dropping its own slot is
/// containable, and one malformed trailer failing the whole projection is not.
///
/// Kept pure and small on purpose: commit-trailer grammar has a native home coming in brit, and
/// this family migrates there intact. Git's `%(trailers)` already does the RFC-822-shaped work —
/// all that belongs here is normalizing the values it hands back.
fn normalize_co_author(raw: &str) -> Option<String> {
    let raw = raw.trim();
    let open = raw.find('<')?;
    let close = raw.rfind('>')?;
    if close < open {
        return None;
    }
    let email = raw[open + 1..close].trim().to_lowercase();
    if email.is_empty() {
        return None;
    }
    let name = &raw[..open];
    match email.as_str() {
        "noreply@anthropic.com" => Some(format!("agent:{}", name_slug(name)?)),
        "noreply@ethosengine.com" => Some(format!("collective:{}", name_slug(name)?)),
        // An unrecognised domain is a person we decline to classify, addressed by the only
        // identifier the trailer actually established.
        _ => Some(email),
    }
}

/// A display name → the slug half of a vocabulary term.
///
/// Restricted to `[a-z0-9-]` — the same character class the actor plane's role segment uses — so
/// a reader comparing an `agent:` term against an actor ref is comparing like with like rather
/// than squinting at two different alphabets. Every run of anything else collapses to a single
/// separator; leading and trailing separators are trimmed. A name with nothing left after that
/// yields `None`, because a term whose subject is the empty string names everyone.
fn name_slug(name: &str) -> Option<String> {
    let mut slug = String::new();
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let trimmed = slug.trim_matches('-');
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// The WIP fence, minted as ONE bounded `Commitment` (spec §3 and §12).
///
/// The covenant says max two habits may be `active` at once. That was a number a human
/// remembered; here it becomes a PROMISE with a declared ceiling, so attention allocation is
/// measured by the same fabric that measures everything else rather than by recollection.
///
/// Its scope is the covenant atom itself — the document that declares the fence — so the
/// commitment hangs off the thing it is accountable to. Nothing mints if either the covenant or
/// the register is absent: a fence declared over a register that does not exist would be a
/// promise about nothing.
///
/// `sense` and `source` are deliberately `None`. `Ceiling` and `Declared` ARE the v1 defaults,
/// and `Bound::validate` refuses their redundant explicit spellings, because one meaning with
/// two encodings gives one promise two content addresses.
fn derive_wip_fence(root: &Path, deriv: &mut Derivation) -> FlowResult<()> {
    let covenant_rel = ".epr-meta/habits-covenant.md";
    let Some(scope_cid) = body_cid_of_file(&root.join(covenant_rel)) else {
        return Ok(());
    };
    // Reading the register is the precondition, not an input to the promise: the fence's LIMIT
    // is declared by the covenant, and the register is what will be counted against it.
    if super::registers::read_habits(root).is_err() {
        return Ok(());
    }
    deriv.label(&scope_cid, covenant_rel);

    // "Max 2 active" is the covenant's sentence; 3.0 is its encoding. `Bound::breached_by` for
    // a ceiling is `stock >= limit`, so the limit is the FIRST FORBIDDEN level, not the last
    // permitted one — a limit of 2.0 would refuse the very state the covenant allows, and did,
    // live, on a register with exactly two active habits.
    let bound = Bound {
        limit: 3.0,
        unit: WIP_FENCE_UNIT.to_string(),
        threshold_pct: 50.0,
        sense: None,
        source: None,
    };
    bound.validate()?;

    let commitment = Commitment {
        action: ReaVerb::Produce,
        provider: AgentRef("tool:habits-register".to_string()),
        receiver: repo_agent(),
        resource_spec: ResourceSpec {
            classified_as: vec![
                "register:wip-fence".to_string(),
                "habit:attention".to_string(),
            ],
            quantity: None,
        },
        in_scope_of: scope_cid,
        valid_from: None,
        valid_until: None,
        state: CommitmentState::Active,
        satisfies: Vec::new(),
        bound: Some(bound),
    };
    let cid = atom_cid(&commitment)?;
    deriv.stage_record(
        FlowRecord::Commitment(commitment),
        cid,
        "commitment:wip-fence".to_string(),
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

/// What makes two events the SAME ACT, independent of what vocabulary they carry: this verb, by
/// this provider, on this resource, in this process, at this instant.
type EventKey = (ReaVerb, AgentRef, Cid, Option<Cid>, String);

/// The identity of an act, as distinct from the identity of a record.
///
/// **Why this exists.** CID dedupe alone answers "is this record already written". It cannot
/// answer "is this act already recorded", and the two stopped being the same question the moment
/// an event grew optional slots. A `Produce` already in the sidecar, re-derived once its commit's
/// co-authors are read, is byte-different and therefore CID-different — so a CID-only guard would
/// append it as a SECOND production of the same artifact, by the same author, at the same
/// instant. Every stock that folds produce-minus-consume would then double-count it, and the
/// corpus would appear to grow on being re-measured.
///
/// **What it deliberately does not do.** It does not update the recorded event. History here is
/// append-only and is not retro-attributed: the roster on an act that was recorded without one
/// is a deliberate migration with its own decision behind it, never a silent side effect of
/// running the projection again. Enrichment therefore reaches newly-minted events only, and an
/// older event keeps saying exactly what it said when it was witnessed.
///
/// `classified_as` is excluded from the key precisely because it is the field that may grow;
/// including it would make the key a restatement of the CID and guard nothing.
fn event_key(event: &FlowEvent) -> EventKey {
    (
        event.action,
        event.provider.clone(),
        event.resource,
        event.process,
        event.occurred_at.clone(),
    )
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The whole normalization contract in one table: the three domain arms, what a slug does to
    /// a name, and every shape that must decline rather than guess.
    #[test]
    fn normalize_co_author_maps_trailers_to_vocabulary_and_declines_the_rest() {
        let cases: &[(&str, Option<&str>)] = &[
            // The two known domains name their collaborators by slug.
            (
                "Claude Fable 5 <noreply@anthropic.com>",
                Some("agent:claude-fable-5"),
            ),
            (
                "Claude Opus 4.6 <noreply@anthropic.com>",
                Some("agent:claude-opus-4-6"),
            ),
            (
                "ethosengine <noreply@ethosengine.com>",
                Some("collective:ethosengine"),
            ),
            // Slug shaping: runs of punctuation and whitespace collapse to one separator, and
            // leading/trailing junk is trimmed rather than carried into the term.
            (
                "  Claude   Sonnet   5  <noreply@anthropic.com>",
                Some("agent:claude-sonnet-5"),
            ),
            (
                "-- Claude (Fable) 5! -- <noreply@anthropic.com>",
                Some("agent:claude-fable-5"),
            ),
            (
                "Ada — Lovelace <noreply@ethosengine.com>",
                Some("collective:ada-lovelace"),
            ),
            // An unknown domain is declined-but-addressed: no interpretation, no misattribution.
            (
                "Matthew Dowell <MBD06b+GitHub@Gmail.com>",
                Some("mbd06b+github@gmail.com"),
            ),
            ("someone@example.test", None), // no brackets — not a trailer shape
            ("Nameless <>", None),          // brackets, nothing inside
            ("Nameless <   >", None),       // whitespace is not an address
            ("", None),                     // empty-after-trim
            ("   ", None),
            (">inverted< <", None), // brackets in the wrong order
            // A known domain with no name has no slug, and a term whose subject is empty names
            // everyone — so it is declined rather than minted as a bare `agent:`.
            ("<noreply@anthropic.com>", None),
            ("!!! <noreply@ethosengine.com>", None),
        ];
        for (raw, expected) in cases {
            assert_eq!(
                normalize_co_author(raw).as_deref(),
                *expected,
                "normalizing {raw:?}"
            );
        }
    }

    #[test]
    fn a_roster_is_sorted_deduped_and_prefixed_so_typing_order_cannot_move_a_cid() {
        let raw: Vec<String> = [
            "ethosengine <noreply@ethosengine.com>",
            "Claude Fable 5 <noreply@anthropic.com>",
            // The same collaborator named twice, once with sloppier spacing.
            "Claude  Fable  5 <noreply@anthropic.com>",
            // One unparseable value must cost only its own slot.
            "not a trailer at all",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        assert_eq!(
            co_author_slots(&raw),
            vec![
                "co-author:agent:claude-fable-5".to_string(),
                "co-author:collective:ethosengine".to_string(),
            ]
        );
    }

    #[test]
    fn an_empty_roster_stays_an_empty_vec_so_a_solo_commit_encodes_as_it_always_did() {
        assert!(co_author_slots(&[]).is_empty());
        assert!(co_author_slots(&["<>".to_string()]).is_empty());
    }

    #[test]
    fn the_minted_agent_term_is_not_a_parseable_actor_ref() {
        // Structural disjointness, asserted rather than assumed: a trailer-derived term is an
        // assertion ABOUT someone, an actor ref is an agent's claim about itself, and no string
        // comparison may ever let one pass for the other.
        let term = normalize_co_author("Claude Fable 5 <noreply@anthropic.com>").unwrap();
        assert!(elohim_epr_rea::parse_agent_ref(&term).is_err());
    }
}
