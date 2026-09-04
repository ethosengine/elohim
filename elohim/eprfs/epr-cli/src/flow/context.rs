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

use super::read::{commitment_latest_event, notes_on, NoteView};
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

    let notes = notes_on(&records, &cid, notes);

    let seals = walked.as_ref().map(|result| Seals {
        edges: result.edges.outgoing.clone(),
        stale_downstream: result.frontier.stale_edges.len(),
    });

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

/// Section 8, best effort. An unreadable authority index or cascade yields `None` — the honest
/// absence — rather than an error: a governance line is an enrichment of the screen, and an
/// enrichment that can veto its subject is a dependency in the wrong direction.
fn governance_for(root: &Path, rel: &str) -> Option<Governance> {
    let resolution = eprfs_meta::resolve_path(root, &root.join(rel)).ok()?;
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
            println!(
                "    • {}{} — {}",
                note.kind,
                verdict,
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
