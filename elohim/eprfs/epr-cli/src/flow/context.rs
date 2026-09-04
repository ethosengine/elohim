//! `epr flow context <path|cid>` — the one screen a fresh agent otherwise re-derives by hand
//! (spec §6, `genesis/docs/superpowers/specs/2026-09-05-valueflow-authoring-surface-design.md`).
//!
//! "What is open on this atom, what seals it, what governs it, what has been said about it" is
//! a question every session asks and every session answers by grepping. Each of its parts was
//! already derivable; none of them was in one place, and the cost of that was paid once per
//! agent per atom. This verb is the assembly, not a new derivation: `walk` supplies the scoped
//! intents, the undischarged commitments and the sealed edges; [`super::read`] supplies the
//! notes and the latest-event rule; `explain`'s authority and cascade supply the governance
//! line. Nothing here re-derives anything one of those already owns.
//!
//! **`walk`'s `Produce` filter is NOT widened.** Notes are `Cite` events and are invisible to
//! `walk` by construction, which is a stability promise its JSON consumers hold. Section 4 goes
//! to the store directly instead.
//!
//! **A bare content address skips sections 5 through 8** and says so in one line. Seals,
//! habits, gates and governance are all properties of a FILE IN THE TREE; an address has no
//! path, and printing an empty section without saying why reads as "there are none".

use std::path::Path;

use cid::Cid;
use elohim_epr_rea::{CommitmentState, FlowRecord, FlowStore, SidecarFlowStore};
use serde::Serialize;

use super::read::{commitment_latest_event, notes_across, NoteSource, NoteView};
use super::registers::{self, GateView, HabitEntry};
use super::walk::{self, EdgeView};
use super::{body_cid_of_file, confine_under, rel_to_root, short_cid, FlowError, FlowResult};

/// How many notes a context prints when the caller does not say.
pub const DEFAULT_NOTES: usize = 5;

/// How many rows any one list prints before it truncates with an explicit "and N more".
const RENDER_ROWS: usize = 3;

/// Section 1 — what this atom IS.
#[derive(Debug, Serialize)]
pub struct Identity {
    /// Repo-relative path, absent when the target was named by address alone.
    pub path: Option<String>,
    pub cid: String,
    /// The operational labels the sidecar carries for this address.
    pub labels: Vec<String>,
}

/// Section 2 — an open intent scoped by this atom.
#[derive(Debug, Serialize)]
pub struct ContextIntent {
    pub cid: String,
    /// The intent's slot-1 subject: its gap id, when it has one.
    pub gap_id: Option<String>,
    /// The intent's slot-0 tag: `gap:open`, `gap:claimed`, …
    pub state: Option<String>,
    pub raised_by: String,
}

/// The latest event associated with a commitment, by the one shared ordering rule.
#[derive(Debug, Serialize)]
pub struct LatestEvent {
    pub action: String,
    pub occurred_at: String,
}

/// Section 3 — an undischarged commitment on this atom.
#[derive(Debug, Serialize)]
pub struct ContextCommitment {
    pub cid: String,
    pub provider: String,
    pub state: String,
    pub gap_id: Option<String>,
    pub brief: Option<String>,
    pub habit: Option<String>,
    pub latest_event: Option<LatestEvent>,
}

/// Section 5 — the sealed contract edges this atom depends on, and what a change here breaks.
#[derive(Debug, Serialize)]
pub struct Seals {
    pub edges: Vec<EdgeView>,
    pub stale_downstream: usize,
}

/// Section 6 — a habit that covers this atom. A habit is the STANDARD the work is accounted to
/// (spec §3), so what a reader needs from it is where it stands and what would prove it.
#[derive(Debug, Serialize)]
pub struct HabitView {
    pub id: String,
    pub status: String,
    pub active: bool,
    pub first_check: Option<String>,
    /// Where the coverage claim came from: the generated register, or a `.epr-meta` declaration
    /// in an ancestor directory (which names the file).
    pub source: String,
}

/// The habit-as-scope render, used when the target IS a habit atom. A habit's own screen is not
/// the same screen as a file's: what is open ON it is the work accounted TO it.
#[derive(Debug, Serialize)]
pub struct HabitScope {
    pub id: String,
    pub status: String,
    pub active: bool,
    pub checks: Vec<String>,
    /// The specs and plans the register says name this habit.
    pub refs: Vec<String>,
    /// Undischarged commitments carrying this habit's `habit:<id>` slot, wherever they are
    /// scoped — the work accounted to the standard.
    pub open_commitments: Vec<ContextCommitment>,
}

/// Section 8 — the governance already computed by `epr explain`, reused rather than duplicated.
#[derive(Debug, Serialize)]
pub struct Governance {
    /// The capability package that owns this path, when one does.
    pub authority: Option<String>,
    /// How many `.epr-meta` manifests apply.
    pub cascade_depth: usize,
    /// How many inline rules those manifests put in force.
    pub rule_count: usize,
}

/// The whole screen, in section order. Every section is PRESENT under `--json` — absent ones as
/// an explicit null or an empty collection — because a key that disappears makes "no seals" and
/// "seals not computed" the same reading, and they are not the same fact.
#[derive(Debug, Serialize)]
pub struct ContextResult {
    pub identity: Identity,
    pub intents: Vec<ContextIntent>,
    pub commitments: Vec<ContextCommitment>,
    pub notes: Vec<NoteView>,
    pub seals: Option<Seals>,
    pub habits: Vec<HabitView>,
    pub gate: Option<GateView>,
    /// Present only when the target is itself a `.habit.md` atom.
    pub habit_scope: Option<HabitScope>,
    pub governance: Option<Governance>,
    /// One line saying why the path-only sections are absent, when they are.
    pub scope_note: Option<String>,
}

/// `epr flow context <path|cid>` with the default note window.
///
/// The plan's named entry point: tests call the library the way `tests/flow_edges.rs` already
/// calls `walk`.
pub fn context(root: &Path, target: &str) -> FlowResult<ContextResult> {
    context_with(root, target, DEFAULT_NOTES)
}

/// `context`, with the `--notes N` window the CLI passes through.
pub fn context_with(root: &Path, target: &str, notes: usize) -> FlowResult<ContextResult> {
    let store = SidecarFlowStore::open(root)?;
    let records = store.records()?;
    let (cid, path) = resolve_target(root, target, &records)?;

    let labels = load_label(root, &cid);

    // Sections 2 and 3 come from `walk` itself when there is a path to walk — its scoped-intents
    // and unfulfilled-commitments derivations are the ones every other reader already trusts. A
    // bare address has no path, so the same two predicates are applied to the records directly.
    let walked = match &path {
        Some(rel) => Some(walk::walk(root, rel)?),
        None => None,
    };

    let (intents, commitments) = match &walked {
        Some(result) => (
            result
                .frontier
                .scoped_intents
                .iter()
                .map(|view| ContextIntent {
                    cid: view.intent.cid.clone(),
                    state: view.classified_as.first().cloned(),
                    gap_id: view.classified_as.get(1).cloned(),
                    raised_by: view.raised_by.clone(),
                })
                .collect(),
            result
                .frontier
                .unfulfilled_commitments
                .iter()
                .map(|view| {
                    commitment_row(
                        &view.commitment.cid,
                        &view.provider,
                        &view.state,
                        &view.classified_as,
                        &records,
                    )
                })
                .collect(),
        ),
        None => scoped_from_records(&cid, &records),
    };

    // Section 4. The atom's own notes, PLUS the notes recorded on the commitments scoped by it.
    // A review seat records its verdict ON the commitment it reviewed, so a screen that showed
    // only the atom's own notes would miss exactly the record the seat left — which is what the
    // first dogfood run found. Discharged commitments are included too: the window decides what
    // fits, not the state of the promise, because a verdict does not stop being the verdict when
    // the work it judged is done.
    let mut sources = vec![NoteSource {
        resource: cid,
        via: None,
    }];
    for (record_cid, record) in &records {
        if let FlowRecord::Commitment(commitment) = record {
            if commitment.in_scope_of == cid {
                sources.push(NoteSource {
                    resource: *record_cid,
                    via: Some(
                        commitment
                            .resource_spec
                            .classified_as
                            .get(1)
                            .cloned()
                            .unwrap_or_else(|| short_cid(record_cid)),
                    ),
                });
            }
        }
    }
    let notes = notes_across(&records, &sources, notes);

    let seals = walked.as_ref().map(|result| Seals {
        edges: result.edges.outgoing.clone(),
        stale_downstream: result.frontier.stale_edges.len(),
    });

    // Sections 6 and 7. The register is a GENERATED projection and is read as data; a missing
    // one is an honest empty section rather than a refusal, because `context` must still answer
    // for a repository that carries no habit register at all.
    let register = registers::read_habits(root).unwrap_or_default();
    let habits = path
        .as_deref()
        .map(|rel| habits_for(root, rel, &register))
        .unwrap_or_default();
    let gate = path
        .as_deref()
        .and_then(|rel| registers::gate_for_path(root, rel));
    let habit_scope = path
        .as_deref()
        .and_then(|rel| habit_scope_for(rel, &register, &records));

    let governance = path.as_deref().and_then(|rel| governance_for(root, rel));

    let scope_note = path.is_none().then(|| {
        "target named by content address: seals, habits, gate and governance are properties of a \
         file in the tree and are not computed here"
            .to_string()
    });

    Ok(ContextResult {
        identity: Identity {
            path,
            cid: cid.to_string(),
            labels,
        },
        intents,
        commitments,
        notes,
        seals,
        habits,
        gate,
        habit_scope,
        governance,
        scope_note,
    })
}

/// One commitment row, with its positional slots read back out and its latest event resolved by
/// the one shared ordering rule.
fn commitment_row(
    cid: &str,
    provider: &str,
    state: &str,
    classified_as: &[String],
    records: &[(Cid, FlowRecord)],
) -> ContextCommitment {
    let latest = cid
        .parse::<Cid>()
        .ok()
        .and_then(|parsed| commitment_latest_event(records, &parsed))
        .map(|(action, occurred_at)| LatestEvent {
            action: format!("{action:?}"),
            occurred_at,
        });
    ContextCommitment {
        cid: cid.to_string(),
        provider: provider.to_string(),
        state: state.to_string(),
        gap_id: classified_as.get(1).cloned(),
        brief: slot(classified_as, "brief:"),
        habit: slot(classified_as, "habit:"),
        latest_event: latest,
    }
}

fn slot(classified_as: &[String], prefix: &str) -> Option<String> {
    classified_as
        .iter()
        .find_map(|s| s.strip_prefix(prefix).map(str::to_string))
}

/// Sections 2 and 3 for a bare-address target, by `walk`'s own two predicates: an intent scoped
/// by the atom, and a Proposed/Active commitment scoped by it that no event has discharged.
fn scoped_from_records(
    cid: &Cid,
    records: &[(Cid, FlowRecord)],
) -> (Vec<ContextIntent>, Vec<ContextCommitment>) {
    let discharged: std::collections::HashSet<Cid> = records
        .iter()
        .filter_map(|(_, record)| match record {
            FlowRecord::Event(e) => Some(e.fulfills.clone()),
            _ => None,
        })
        .flatten()
        .collect();

    let mut intents = Vec::new();
    let mut commitments = Vec::new();
    for (record_cid, record) in records {
        match record {
            FlowRecord::Intent(i) if &i.in_scope_of == cid => intents.push(ContextIntent {
                cid: record_cid.to_string(),
                state: i.resource_spec.classified_as.first().cloned(),
                gap_id: i.resource_spec.classified_as.get(1).cloned(),
                raised_by: i.raised_by.0.clone(),
            }),
            FlowRecord::Commitment(c)
                if &c.in_scope_of == cid
                    && matches!(c.state, CommitmentState::Proposed | CommitmentState::Active)
                    && !discharged.contains(record_cid) =>
            {
                commitments.push(commitment_row(
                    &record_cid.to_string(),
                    &c.provider.0,
                    &format!("{:?}", c.state),
                    &c.resource_spec.classified_as,
                    records,
                ));
            }
            _ => {}
        }
    }
    (intents, commitments)
}

/// Resolve the target to `(address, optional repo-relative path)`.
///
/// A CID must already be an atom this sidecar knows — the same membership check `note` makes,
/// and for the same reason: a well-formed address for something never recorded would render an
/// empty screen that reads as "nothing is open here".
fn resolve_target(
    root: &Path,
    target: &str,
    records: &[(Cid, FlowRecord)],
) -> FlowResult<(Cid, Option<String>)> {
    if let Ok(cid) = target.parse::<Cid>() {
        let known = records.iter().any(|(record_cid, record)| {
            record_cid == &cid
                || match record {
                    FlowRecord::Event(e) => e.resource == cid || e.in_scope_of == cid,
                    FlowRecord::Commitment(c) => c.in_scope_of == cid,
                    FlowRecord::Intent(i) => i.in_scope_of == cid,
                    FlowRecord::Process(p) => {
                        p.in_scope_of == cid || p.inputs.contains(&cid) || p.outputs.contains(&cid)
                    }
                    FlowRecord::Spec(_) | FlowRecord::Edge(_) => false,
                }
        });
        if known {
            return Ok((cid, None));
        }
        return Err(FlowError::UnknownResource(format!(
            "{target} (a well-formed CID, but no record in this sidecar mints or names it)"
        )));
    }

    let canonical_root = std::fs::canonicalize(root).map_err(|source| FlowError::Read {
        path: root.to_path_buf(),
        source,
    })?;
    let abs = if Path::new(target).is_absolute() {
        std::path::PathBuf::from(target)
    } else {
        canonical_root.join(target)
    };
    let confined = confine_under(&canonical_root, &abs)?;
    let cid = body_cid_of_file(&confined)
        .ok_or_else(|| FlowError::UnknownResource(target.to_string()))?;
    Ok((cid, Some(rel_to_root(&canonical_root, &confined))))
}

fn load_label(root: &Path, cid: &Cid) -> Vec<String> {
    let path = root.join(".eprfs").join("status").join("labels.json");
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    let Ok(labels) = serde_json::from_str::<super::Labels>(&text) else {
        return Vec::new();
    };
    labels.get(&cid.to_string()).cloned().into_iter().collect()
}

/// The habits covering `rel`, in the spec's order: the register's own coverage claim first
/// (any entry whose `checks:` or `refs:` mention the path), then any habit DECLARED in the
/// nearest ancestor `.epr-meta` directory as `<id>.habit.md`.
///
/// The declaration arm reads only the file NAME and takes status, active and checks from the
/// register, which is that atom's own projection. Parsing the atom's frontmatter here would put
/// a second reader of the habit declaration in the tree, and two readers of one declaration is
/// how a projection acquires a rival.
fn habits_for(root: &Path, rel: &str, register: &[HabitEntry]) -> Vec<HabitView> {
    let mut views: Vec<HabitView> = registers::habits_covering(register, rel)
        .into_iter()
        .map(|habit| habit_view(&habit, "register".to_string()))
        .collect();

    let mut dir = std::path::PathBuf::from(rel);
    if !dir.pop() {
        dir = std::path::PathBuf::new();
    }
    loop {
        let meta_rel = dir.join(".epr-meta");
        if let Ok(entries) = std::fs::read_dir(root.join(&meta_rel)) {
            let mut declared: Vec<String> = entries
                .filter_map(|entry| entry.ok())
                .filter_map(|entry| {
                    let name = entry.file_name().to_string_lossy().to_string();
                    name.strip_suffix(".habit.md").map(str::to_string)
                })
                .collect();
            declared.sort();
            if !declared.is_empty() {
                for id in declared {
                    if views.iter().any(|view| view.id == id) {
                        continue;
                    }
                    let source = meta_rel
                        .join(format!("{id}.habit.md"))
                        .to_string_lossy()
                        .replace('\\', "/");
                    match register.iter().find(|habit| habit.id == id) {
                        Some(habit) => views.push(habit_view(habit, source)),
                        None => views.push(HabitView {
                            id,
                            status: "unprojected".to_string(),
                            active: false,
                            first_check: None,
                            source,
                        }),
                    }
                }
                // The NEAREST ancestor that declares habits owns the declaration arm; a further
                // ancestor's habits govern a wider concern than this atom.
                break;
            }
        }
        if !dir.pop() {
            break;
        }
    }
    views
}

fn habit_view(habit: &HabitEntry, source: String) -> HabitView {
    HabitView {
        id: habit.id.clone(),
        status: habit.status.clone(),
        active: habit.active,
        first_check: habit.checks.first().cloned(),
        source,
    }
}

/// When the target IS a habit atom, the habit renders as a SCOPE: what it holds itself to, and
/// what work is accounted to it.
fn habit_scope_for(
    rel: &str,
    register: &[HabitEntry],
    records: &[(Cid, FlowRecord)],
) -> Option<HabitScope> {
    let file = rel.rsplit('/').next()?;
    let id = file.strip_suffix(".habit.md")?.to_string();
    let habit = register.iter().find(|habit| habit.id == id);
    let slot = format!("habit:{id}");

    let discharged: std::collections::HashSet<Cid> = records
        .iter()
        .filter_map(|(_, record)| match record {
            FlowRecord::Event(e) => Some(e.fulfills.clone()),
            _ => None,
        })
        .flatten()
        .collect();

    let open_commitments = records
        .iter()
        .filter_map(|(cid, record)| match record {
            FlowRecord::Commitment(c)
                if matches!(c.state, CommitmentState::Proposed | CommitmentState::Active)
                    && !discharged.contains(cid)
                    && c.resource_spec.classified_as.iter().any(|s| s == &slot) =>
            {
                Some(commitment_row(
                    &cid.to_string(),
                    &c.provider.0,
                    &format!("{:?}", c.state),
                    &c.resource_spec.classified_as,
                    records,
                ))
            }
            _ => None,
        })
        .collect();

    Some(HabitScope {
        id,
        status: habit
            .map(|h| h.status.clone())
            .unwrap_or_else(|| "unprojected".to_string()),
        active: habit.is_some_and(|h| h.active),
        checks: habit.map(|h| h.checks.clone()).unwrap_or_default(),
        refs: habit.map(|h| h.refs.clone()).unwrap_or_default(),
        open_commitments,
    })
}

/// Section 8, best effort. An unreadable authority index or cascade yields `None` — the honest
/// absence — rather than an error: a governance line is an enrichment of the screen, and an
/// enrichment that can veto its subject is a dependency in the wrong direction.
fn governance_for(root: &Path, rel: &str) -> Option<Governance> {
    let resolution = eprfs_meta::resolve_path(root, root.join(rel)).ok()?;
    let authority = crate::authority::AuthorityIndex::load(root)
        .ok()
        .and_then(|index| index.find(Path::new(rel)).map(|p| p.package_path.clone()));
    Some(Governance {
        authority,
        cascade_depth: resolution.records.len(),
        rule_count: resolution.effective_rules.len(),
    })
}

impl ContextResult {
    /// The human screen. Capped at 40 lines by construction: every list truncates at
    /// [`RENDER_ROWS`] with an explicit "and N more" line, because a list that silently stops is
    /// a list that lies about its length.
    pub fn render(&self) {
        let target = self
            .identity
            .path
            .clone()
            .unwrap_or_else(|| short_cid_str(&self.identity.cid));
        println!("epr flow context — {target}");
        println!("  cid: {}", self.identity.cid);
        if let Some(label) = self.identity.labels.first() {
            println!("  label: {label}");
        }
        if let Some(note) = &self.scope_note {
            println!("  note: {note}");
        }

        println!("\n  INTENTS ({} open here)", self.intents.len());
        for intent in self.intents.iter().take(RENDER_ROWS) {
            println!(
                "    ◇ {} [{}]",
                intent.gap_id.as_deref().unwrap_or("(no gap id)"),
                intent.state.as_deref().unwrap_or("?")
            );
        }
        print_more(self.intents.len());

        println!("\n  COMMITMENTS ({} undischarged)", self.commitments.len());
        for commitment in self.commitments.iter().take(RENDER_ROWS) {
            println!(
                "    ✗ {} — {} [{}]",
                commitment.gap_id.as_deref().unwrap_or("(no gap id)"),
                commitment.provider,
                commitment.state
            );
            if let Some(latest) = &commitment.latest_event {
                println!(
                    "        latest: {} at {}",
                    latest.action, latest.occurred_at
                );
            }
        }
        print_more(self.commitments.len());

        println!("\n  NOTES ({} shown, newest first)", self.notes.len());
        for note in &self.notes {
            let verdict = note
                .verdict
                .as_ref()
                .map(|v| format!(" [{v}]"))
                .unwrap_or_default();
            let via = note
                .via
                .as_ref()
                .map(|v| format!(" via {v}"))
                .unwrap_or_default();
            println!(
                "    • {}{}{} — {}",
                note.kind,
                verdict,
                via,
                note.reason.as_deref().unwrap_or("")
            );
        }

        match &self.seals {
            Some(seals) => {
                println!(
                    "\n  SEALS ({} outgoing · {} stale downstream)",
                    seals.edges.len(),
                    seals.stale_downstream
                );
                for edge in seals.edges.iter().take(RENDER_ROWS) {
                    println!("    {} · {} · {}", edge.verdict, edge.governor, edge.to);
                }
                print_more(seals.edges.len());
            }
            None => println!("\n  SEALS (not computed — no path)"),
        }

        if let Some(scope) = &self.habit_scope {
            println!(
                "\n  HABIT SCOPE — {} [{}{}] · {} check(s) · {} open commitment(s)",
                scope.id,
                scope.status,
                if scope.active { ", active" } else { "" },
                scope.checks.len(),
                scope.open_commitments.len()
            );
            for check in scope.checks.iter().take(RENDER_ROWS) {
                println!("    check: {check}");
            }
        } else {
            match self.habits.first() {
                Some(habit) => println!(
                    "\n  HABIT — {} [{}{}] ({})",
                    habit.id,
                    habit.status,
                    if habit.active { ", active" } else { "" },
                    habit.source
                ),
                None => println!("\n  HABIT (no habit in the register names this path)"),
            }
        }

        match &self.gate {
            // A tie names every candidate. The reader picks; this render never does.
            Some(gate) if !gate.ambiguous.is_empty() => {
                let names: Vec<String> = gate
                    .ambiguous
                    .iter()
                    .map(|name| format!("just gate {name}"))
                    .collect();
                println!("\n  GATE — ambiguous: {}", names.join(" | "));
            }
            Some(gate) => {
                println!(
                    "\n  GATE — {}",
                    gate.command.as_deref().unwrap_or("(unnamed)")
                );
                if let Some(dir) = &gate.target_dir {
                    println!(
                        "    CARGO_TARGET_DIR={dir}  RUSTFLAGS=\"{}\"",
                        gate.rustflags.as_deref().unwrap_or("")
                    );
                }
            }
            None => println!("\n  GATE (no gate project covers this path)"),
        }

        match &self.governance {
            Some(g) => println!(
                "\n  GOVERNANCE — {} manifest(s), {} rule(s), authority: {}",
                g.cascade_depth,
                g.rule_count,
                g.authority.as_deref().unwrap_or("unmanaged")
            ),
            None => println!("\n  GOVERNANCE (not computed — no path)"),
        }
    }
}

fn print_more(total: usize) {
    if total > RENDER_ROWS {
        println!("    … and {} more", total - RENDER_ROWS);
    }
}

fn short_cid_str(cid: &str) -> String {
    match cid.parse::<Cid>() {
        Ok(parsed) => short_cid(&parsed),
        Err(_) => cid.to_string(),
    }
}
