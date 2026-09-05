//! `epr flow fulfill <report.json>` — the a2o-verdict → REA-fulfillment emitter (spec §5
//! joint 5, slice-3's a2o leg,
//! `genesis/docs/superpowers/specs/2026-07-18-epr-rea-valueflow-fabric-design.md`).
//!
//! Reads a sprint-report's `summary.byConcern` rollup and, for each concern's distinct
//! `scenarios[].surface` (a repo-relative feature-file path under `genesis/a2o`), resolves
//! the ONE open `a2o:scenario-green` `Commitment` the `project` command minted for that
//! feature file (`project.rs::derive_scenario`), then:
//!
//! - **all-green** (`failed==0 && pending==0 && passed>0`) and not yet discharged → append a
//!   `Produce` `FlowEvent` fulfilling the commitment (unit `green-run`).
//! - **all-green**, discharged, and the LATEST event associated with the commitment — ordered
//!   by `occurred_at` (RFC3339), tie-broken by sidecar append order, mirroring
//!   `.claude/scripts/saga-status.py`'s `index_flow_state`/`_sort_key` EXACTLY (see
//!   `read::commitment_latest_event`) — is a `Dismiss` → **regression re-commitment**: append a
//!   fresh `Produce` `FlowEvent` (unit `green-run`) re-fulfilling it, counted `refulfilled` —
//!   distinct from `fulfilled_new` (a first-ever fulfillment) and `already_fulfilled`
//!   (steady-state, nothing to do). Without this, a chapter that regresses once (a red run
//!   after a prior green) stays "regressed" forever even after CI goes green again, because the
//!   ordinary `discharged` check only asks "was there ever a Produce", not "is the LATEST event
//!   (by time) a Produce" — saga-status reads the same sidecar and derives its `regressed`
//!   state from that same latest-BY-TIME-event question, so the two tools must agree on
//!   "latest". Ordering by *append* order alone (the pre-fix behavior) diverges from
//!   saga-status under replay/backfill: a delayed green report can append AFTER a chronologically
//!   newer Dismiss, making append-order "latest" say Produce while time-order (saga-status'
//!   truth) still correctly says Dismiss — exactly the bug this fix closes.
//! - This regression re-commitment is additionally gated on **freshness**: the incoming
//!   report's `generatedAt` must be STRICTLY NEWER (same timestamp-comparison rule) than the
//!   latest Dismiss's `occurredAt`. A backfilled OLD green report must never re-produce over a
//!   chronologically newer regression still standing — that's counted `skipped_stale_recovery`,
//!   not `refulfilled`.
//! - **all-green**, discharged, and the latest associated event (by time) is a `Produce` (no
//!   intervening regression, or a regression already re-committed) → no-op, counted
//!   `already_fulfilled`.
//! - **red** (`failed>0`) and discharged → append a `Dismiss` `FlowEvent` (unit `red-run`,
//!   `fulfills` empty) — a regression on a previously-green chapter.
//! - **red** and never discharged → no-op, counted `skipped_red` (nothing to reverse).
//! - **neither green nor red** (still `pending`) → no-op, counted `skipped_pending`.
//!
//! Identity is the atom CID exactly as everywhere else in this crate: an event's CID is
//! fully determined by its fields (including `occurred_at`, sourced from the report's own
//! `generatedAt` — never a wall-clock read), so re-running the SAME report is naturally a
//! no-op. For a first-ever fulfillment this is the `discharged`-set check before an event is
//! even constructed; for a regression re-commitment, re-running the SAME recovery report
//! finds the LATEST event is now the recovery `Produce` itself (not a `Dismiss` anymore), so
//! the second run falls into the ordinary `already_fulfilled` no-op path without ever
//! re-examining the atom-CID dedupe — the state-machine already advanced. `--dry-run` runs the
//! full matching pass and skips only the append.

use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use cid::Cid;
use elohim_epr_rea::{
    AgentRef, CommitmentState, FlowEvent, FlowRecord, FlowStore, Magnitude, ReaVerb,
    SidecarFlowStore,
};
use serde::{Deserialize, Serialize};

use super::note::{named_identity, non_empty, resolve_attribution, NoteActor};
use super::read::{commitment_latest_event, is_strictly_newer, OccurredAtKey};
use super::{
    body_cid_of_file, confine_under, head_commit_provenance, rel_to_root, repo_agent,
    repo_scope_atom, FlowError, FlowResult,
};

/// The synthetic CI agent standing in for the a2o run that observed the verdict.
const CI_AGENT: &str = "ci:dataplane";

// ---------------------------------------------------------------------------
// Sprint-report shape (only the fields `fulfill` reads — schema:
// genesis/a2o/schemas/sprint-report.schema.json)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SprintReport {
    generated_at: String,
    run_id: String,
    summary: Summary,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Summary {
    #[serde(default)]
    by_concern: BTreeMap<String, ConcernRollup>,
}

#[derive(Debug, Deserialize)]
struct ConcernRollup {
    #[serde(default)]
    passed: u32,
    #[serde(default)]
    failed: u32,
    #[serde(default)]
    pending: u32,
    #[serde(default)]
    scenarios: Vec<ConcernScenario>,
}

#[derive(Debug, Deserialize)]
struct ConcernScenario {
    surface: String,
}

// ---------------------------------------------------------------------------
// Options + summary
// ---------------------------------------------------------------------------

pub struct FulfillOptions {
    pub dry_run: bool,
    /// Joined with a scenario's `surface` (`{prefix}/{surface}`) to resolve the commitment
    /// path a report's feature-file surface refers to. Defaults to `genesis/a2o` — cucumber
    /// surfaces are repo-relative under that directory.
    pub surface_prefix: String,
}

impl Default for FulfillOptions {
    fn default() -> Self {
        Self {
            dry_run: false,
            surface_prefix: "genesis/a2o".to_string(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct FulfillSummary {
    pub run_id: String,
    pub generated_at: String,
    pub dry_run: bool,
    pub fulfilled_new: usize,
    pub already_fulfilled: usize,
    /// A regression re-commitment: the commitment was discharged, then regressed (its
    /// LATEST-BY-TIME associated event was a `Dismiss`), and this all-green report re-fulfills
    /// it with a fresh `Produce` — distinct from a first-ever `fulfilled_new`.
    pub refulfilled: usize,
    /// A regressed commitment's LATEST-BY-TIME event is a `Dismiss`, but this all-green
    /// report's `generatedAt` is NOT strictly newer than that Dismiss's `occurredAt` — a
    /// backfilled/delayed old green report arriving after a chronologically newer regression.
    /// The newer regression stands; no recovery `Produce` is emitted.
    pub skipped_stale_recovery: usize,
    pub skipped_red: usize,
    pub skipped_pending: usize,
    pub regressions_dismissed: usize,
    pub unmatched_surfaces: Vec<String>,
}

impl FulfillSummary {
    pub fn render(&self) {
        println!(
            "epr flow fulfill — run {} @ {}",
            self.run_id, self.generated_at
        );
        if self.dry_run {
            println!("  (dry run — no records appended)");
        }
        println!("  fulfilled (new):       {}", self.fulfilled_new);
        println!("  re-fulfilled (recovery): {}", self.refulfilled);
        println!("  already fulfilled:     {}", self.already_fulfilled);
        println!(
            "  skipped (stale recovery): {}",
            self.skipped_stale_recovery
        );
        println!("  skipped (red):         {}", self.skipped_red);
        println!("  skipped (pending):     {}", self.skipped_pending);
        println!("  regressions dismissed: {}", self.regressions_dismissed);
        println!("  unmatched surfaces:    {}", self.unmatched_surfaces.len());
        if !self.unmatched_surfaces.is_empty() {
            for s in &self.unmatched_surfaces {
                println!("    ? {s}");
            }
            println!(
                "    hint: run `epr flow project` first to mint commitments for every feature"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// fulfill
// ---------------------------------------------------------------------------

pub fn fulfill(
    root: &Path,
    report_path: &Path,
    opts: &FulfillOptions,
) -> FlowResult<FulfillSummary> {
    let text = std::fs::read_to_string(report_path).map_err(|source| FlowError::Read {
        path: report_path.to_path_buf(),
        source,
    })?;
    let report: SprintReport = serde_json::from_str(&text)?;

    let mut store = SidecarFlowStore::open(root)?.transaction()?;
    let records = store.records()?;

    // Index open scenario commitments: `classified_as == ["a2o:scenario-green", <path>]`,
    // state Active. `classified_as[1]` is the repo-relative feature-file path
    // (`project.rs::derive_scenario`).
    let mut commitments: Vec<(Cid, String)> = Vec::new();
    for (cid, record) in &records {
        let FlowRecord::Commitment(c) = record else {
            continue;
        };
        if c.state != CommitmentState::Active {
            continue;
        }
        let classified = &c.resource_spec.classified_as;
        if classified.first().map(String::as_str) != Some("a2o:scenario-green") {
            continue;
        }
        if let Some(path) = classified.get(1) {
            commitments.push((*cid, path.clone()));
        }
    }

    // The discharged set — exactly the walk.rs:170-179 derivation: any event whose
    // `fulfills` names the commitment. `Dismiss` events carry an empty `fulfills`, so only
    // a prior `Produce` fulfillment ever discharges a commitment here.
    let discharged: HashSet<Cid> = records
        .iter()
        .filter_map(|(_, record)| match record {
            FlowRecord::Event(e) => Some(e.fulfills.clone()),
            _ => None,
        })
        .flatten()
        .collect();

    let existing_cids: HashSet<Cid> = records.iter().map(|(cid, _)| *cid).collect();
    let repo_scope = repo_scope_atom()?;

    let mut summary = FulfillSummary {
        run_id: report.run_id.clone(),
        generated_at: report.generated_at.clone(),
        dry_run: opts.dry_run,
        fulfilled_new: 0,
        already_fulfilled: 0,
        refulfilled: 0,
        skipped_stale_recovery: 0,
        skipped_red: 0,
        skipped_pending: 0,
        regressions_dismissed: 0,
        unmatched_surfaces: Vec::new(),
    };

    let mut to_append: Vec<FlowRecord> = Vec::new();
    let mut staged_cids: HashSet<Cid> = HashSet::new();
    let prefix = opts.surface_prefix.trim_end_matches('/');

    for (concern, rollup) in &report.summary.by_concern {
        let all_green = rollup.failed == 0 && rollup.pending == 0 && rollup.passed > 0;
        let is_red = rollup.failed > 0;

        let mut surfaces: Vec<&str> = rollup
            .scenarios
            .iter()
            .map(|s| s.surface.as_str())
            .collect();
        surfaces.sort_unstable();
        surfaces.dedup();

        for surface in surfaces {
            let joined = format!("{prefix}/{surface}");
            let candidates: Vec<&(Cid, String)> = commitments
                .iter()
                .filter(|(_, path)| path.ends_with(&joined))
                .collect();

            let (commit_cid, commit_path) = match candidates.len() {
                0 => {
                    summary.unmatched_surfaces.push(surface.to_string());
                    continue;
                }
                1 => candidates[0],
                _ => {
                    let list: Vec<&str> =
                        candidates.iter().map(|(_, path)| path.as_str()).collect();
                    return Err(FlowError::InvalidArguments(format!(
                        "ambiguous surface `{surface}` (concern `{concern}`) matches {} \
                         commitments — never guess: {}",
                        list.len(),
                        list.join(", ")
                    )));
                }
            };

            if all_green {
                if discharged.contains(commit_cid) {
                    match commitment_latest_event(&records, commit_cid) {
                        Some((ReaVerb::Dismiss, dismiss_at)) => {
                            // Regression re-commitment: discharged, then regressed (the
                            // LATEST-BY-TIME associated event is a Dismiss) — but only if
                            // THIS report is actually newer than that regression. A
                            // backfilled/delayed old green report must not paper over a
                            // chronologically newer Dismiss (module doc above).
                            if is_strictly_newer(&report.generated_at, &dismiss_at) {
                                let resource = body_cid_of_file(&root.join(commit_path))
                                    .ok_or_else(|| {
                                        FlowError::UnknownResource(commit_path.clone())
                                    })?;
                                let event = FlowEvent {
                                    action: ReaVerb::Produce,
                                    provider: AgentRef(CI_AGENT.to_string()),
                                    receiver: repo_agent(),
                                    resource,
                                    quantity: Magnitude::Count {
                                        value: 1.0,
                                        unit: "green-run".to_string(),
                                    },
                                    process: None,
                                    in_scope_of: repo_scope,
                                    fulfills: vec![*commit_cid],
                                    satisfies: Vec::new(),
                                    classified_as: Vec::new(),
                                    occurred_at: report.generated_at.clone(),
                                };
                                stage_or_count(
                                    event,
                                    &existing_cids,
                                    &mut staged_cids,
                                    &mut to_append,
                                    &mut summary.refulfilled,
                                    &mut summary.already_fulfilled,
                                )?;
                            } else {
                                summary.skipped_stale_recovery += 1;
                            }
                        }
                        _ => {
                            summary.already_fulfilled += 1;
                        }
                    }
                    continue;
                }
                let resource = body_cid_of_file(&root.join(commit_path))
                    .ok_or_else(|| FlowError::UnknownResource(commit_path.clone()))?;
                let event = FlowEvent {
                    action: ReaVerb::Produce,
                    provider: AgentRef(CI_AGENT.to_string()),
                    receiver: repo_agent(),
                    resource,
                    quantity: Magnitude::Count {
                        value: 1.0,
                        unit: "green-run".to_string(),
                    },
                    process: None,
                    in_scope_of: repo_scope,
                    fulfills: vec![*commit_cid],
                    satisfies: Vec::new(),
                    classified_as: Vec::new(),
                    occurred_at: report.generated_at.clone(),
                };
                stage_or_count(
                    event,
                    &existing_cids,
                    &mut staged_cids,
                    &mut to_append,
                    &mut summary.fulfilled_new,
                    &mut summary.already_fulfilled,
                )?;
            } else if is_red {
                if !discharged.contains(commit_cid) {
                    summary.skipped_red += 1;
                    continue;
                }
                let resource = body_cid_of_file(&root.join(commit_path))
                    .ok_or_else(|| FlowError::UnknownResource(commit_path.clone()))?;
                let event = FlowEvent {
                    action: ReaVerb::Dismiss,
                    provider: AgentRef(CI_AGENT.to_string()),
                    receiver: repo_agent(),
                    resource,
                    quantity: Magnitude::Count {
                        value: 1.0,
                        unit: "red-run".to_string(),
                    },
                    process: None,
                    in_scope_of: repo_scope,
                    fulfills: Vec::new(),
                    satisfies: Vec::new(),
                    classified_as: Vec::new(),
                    occurred_at: report.generated_at.clone(),
                };
                // A Dismiss that duplicates one already in the sidecar (same report,
                // already dismissed) is a true no-op — it does not recount the regression.
                let mut discard = 0usize;
                stage_or_count(
                    event,
                    &existing_cids,
                    &mut staged_cids,
                    &mut to_append,
                    &mut summary.regressions_dismissed,
                    &mut discard,
                )?;
            } else {
                summary.skipped_pending += 1;
            }
        }
    }

    if !opts.dry_run {
        for record in to_append {
            store.append(record)?;
        }
    }

    Ok(summary)
}

/// Stage a freshly-built event for append unless its atom CID is already present (either in
/// the sidecar from a prior run, or already staged earlier in THIS run) — the idempotency
/// mechanism shared with `project`'s dedup-by-CID.
fn stage_or_count(
    event: FlowEvent,
    existing_cids: &HashSet<Cid>,
    staged_cids: &mut HashSet<Cid>,
    to_append: &mut Vec<FlowRecord>,
    new_counter: &mut usize,
    present_counter: &mut usize,
) -> FlowResult<()> {
    let record = FlowRecord::Event(event);
    let cid = record.cid()?;
    if existing_cids.contains(&cid) || staged_cids.contains(&cid) {
        *present_counter += 1;
    } else {
        staged_cids.insert(cid);
        to_append.push(record);
        *new_counter += 1;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// The task-report arm: `epr flow fulfill --on <commitment|gap-id> --report … --status …`
//
// A SECOND ARM ON ONE VERB, not a second verb. The positional sprint-report path above is
// untouched — its records, its counters and its CIDs are byte-identical — because a2o's
// scenario-green discharge is a CI observation and this one is an authored delivery, and the
// two must stay distinguishable in the ledger while meaning the same thing to the drain: an
// event whose `fulfills` names a commitment.
//
// The status gate is the load-bearing part. `NEEDS_CONTEXT`, `BLOCKED` and `HOLD` are REFUSED
// rather than recorded as a weaker discharge, because the dev-system-equilibrium habit's
// outflow classifier keys on `fulfills` — a non-discharging status that emitted the field would
// read as a drain that never happened, which is exactly the over-claim that habit's invariant
// names. The refusal names `note --kind observation` as the record those three actually want.
// ---------------------------------------------------------------------------

/// The unit a task-report discharge is counted in — distinct from `green-run` so a CI verdict
/// and an authored delivery can never be folded together by a unit-keyed stock.
const TASK_REPORT_UNIT: &str = "task-report";

const REPORT_SLOT_PREFIX: &str = "report:";
const EVIDENCE_SLOT_PREFIX: &str = "evidence:";
const COMMIT_SLOT_PREFIX: &str = "commit:";

/// The two statuses that DISCHARGE a commitment.
const DISCHARGING: [&str; 2] = ["DONE", "DONE_WITH_CONCERNS"];

/// The three that do not, and that this verb refuses by name.
const NON_DISCHARGING: [&str; 3] = ["NEEDS_CONTEXT", "BLOCKED", "HOLD"];

/// What the caller asked for, already split out of argv.
pub struct FulfillOnRequest<'a> {
    pub on: &'a str,
    pub report: &'a str,
    pub status: &'a str,
    pub commits: &'a [String],
    pub actor: &'a NoteActor,
}

/// The machine-facing result of one task-report fulfilment.
#[derive(Debug, Serialize)]
pub struct FulfillOnOutcome {
    pub commitment: String,
    /// The commitment's subject slot — the gap id, when it has one.
    pub gap_id: String,
    pub status: String,
    /// The report's repo-relative path.
    pub report: String,
    /// The report's canonical body address — the evidence, carried by reference.
    pub evidence: String,
    pub commits: Vec<String>,
    pub provider: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steward: Option<String>,
    pub occurred_at: String,
    /// The event's atom address, absent when nothing was minted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub record_cid: Option<String>,
    pub appended: bool,
    /// `true` when the commitment was ALREADY discharged, so this call appended nothing.
    pub already_fulfilled: bool,
}

impl FulfillOnOutcome {
    pub fn render(&self) {
        println!("fulfill {} → {}", self.status, self.gap_id);
        println!("        report: {} ({})", self.report, self.evidence);
        if !self.commits.is_empty() {
            println!("        commits: {}", self.commits.join(", "));
        }
        println!("        by: {}", self.provider);
        if let Some(steward) = &self.steward {
            println!("        steward: {steward}");
        }
        if self.already_fulfilled {
            println!("        (already discharged — no-op)");
        } else if !self.appended {
            println!("        (already recorded — no-op)");
        }
    }
}

/// Normalize and gate `--status`.
///
/// Case and hyphens are forgiven (`done-with-concerns` is the same declaration as
/// `DONE_WITH_CONCERNS`); the VOCABULARY is not. An unknown status is refused naming all five,
/// because a status this verb silently accepted would classify a delivery by a word nothing
/// downstream reads.
fn gate_status(raw: &str) -> FlowResult<String> {
    let normalized = raw.trim().to_ascii_uppercase().replace('-', "_");
    if DISCHARGING.contains(&normalized.as_str()) {
        return Ok(normalized);
    }
    if NON_DISCHARGING.contains(&normalized.as_str()) {
        return Err(FlowError::InvalidArguments(format!(
            "status `{normalized}` does not discharge a commitment — only {} do. \
             Record it instead as `epr flow note --on <gap-id> --kind observation --reason '…'`; \
             a non-discharging status that emitted `fulfills` would read as a drain that never \
             happened",
            DISCHARGING.join(" and ")
        )));
    }
    Err(FlowError::InvalidArguments(format!(
        "unknown --status `{raw}` — the report vocabulary is closed: {}|{}",
        DISCHARGING.join("|"),
        NON_DISCHARGING.join("|")
    )))
}

/// `epr flow fulfill --on <commitment-cid|gap-id> --report <path> --status <STATUS>`.
///
/// Two phases, the idiom this family uses everywhere: Phase 1 resolves and refuses, Phase 2
/// appends exactly one record or none.
pub fn fulfill_on(root: &Path, request: &FulfillOnRequest) -> FlowResult<FulfillOnOutcome> {
    let on = non_empty(request.on, "--on")?;
    let report_rel = non_empty(request.report, "--report")?;
    let status = gate_status(request.status)?;
    let named = named_identity(request.actor.as_ref.as_deref())?;

    let mut store = SidecarFlowStore::open(root)?.transaction()?;
    let records = store.records()?;
    let (commit_cid, commitment) = resolve_commitment(on, &records)?;

    let canonical_root = std::fs::canonicalize(root).map_err(|source| FlowError::Read {
        path: root.to_path_buf(),
        source,
    })?;
    let report_abs = if Path::new(report_rel).is_absolute() {
        std::path::PathBuf::from(report_rel)
    } else {
        canonical_root.join(report_rel)
    };
    let report_abs = confine_under(&canonical_root, &report_abs)?;
    let evidence = body_cid_of_file(&report_abs).ok_or_else(|| {
        FlowError::UnknownResource(format!(
            "{report_rel} (named by --report, but it cannot be read) — \
             the evidence is carried by address, so the report must exist to have one"
        ))
    })?;
    let report_label = rel_to_root(&canonical_root, &report_abs);

    let (author, occurred_at) = head_commit_provenance(root).ok_or_else(|| {
        FlowError::InvalidArguments(format!(
            "cannot date a fulfilment in `{}`: git has no HEAD commit to date it against",
            root.display()
        ))
    })?;
    let attribution = resolve_attribution(root, named, request.actor.session.as_deref(), &author);

    let gap_id = commitment
        .resource_spec
        .classified_as
        .get(1)
        .cloned()
        .unwrap_or_else(|| commit_cid.to_string());

    // Already discharged? The crate's one derivation of it — any event whose `fulfills` names
    // the commitment — so this verb, `claim`, `walk` and the stock fold never disagree.
    let already_fulfilled = records.iter().any(|(_, record)| match record {
        FlowRecord::Event(e) => e.fulfills.contains(&commit_cid),
        _ => false,
    });

    let mut classified_as = Vec::with_capacity(4 + request.commits.len());
    classified_as.push(format!("{REPORT_SLOT_PREFIX}{status}"));
    classified_as.push(gap_id.clone());
    classified_as.push(format!("{EVIDENCE_SLOT_PREFIX}{evidence}"));
    let mut commits = Vec::with_capacity(request.commits.len());
    for sha in request.commits {
        let sha = non_empty(sha, "--commit")?.to_string();
        classified_as.push(format!("{COMMIT_SLOT_PREFIX}{sha}"));
        commits.push(sha);
    }
    attribution.append_slots(&mut classified_as);

    let event = FlowEvent {
        action: ReaVerb::Produce,
        provider: AgentRef(attribution.provider(&author)),
        receiver: repo_agent(),
        resource: evidence,
        quantity: Magnitude::Count {
            value: 1.0,
            unit: TASK_REPORT_UNIT.to_string(),
        },
        process: None,
        // The commitment's OWN scope, copied: the delivery is accounted to the container that
        // raised the promise, not to a scope this verb invented.
        in_scope_of: commitment.in_scope_of,
        fulfills: vec![commit_cid],
        satisfies: Vec::new(),
        classified_as,
        occurred_at: occurred_at.clone(),
    };

    let record = FlowRecord::Event(event);
    let record_cid = record.cid()?;
    let already_recorded = records.iter().any(|(cid, _)| cid == &record_cid);
    let appended = !already_fulfilled && !already_recorded;

    let outcome = FulfillOnOutcome {
        commitment: commit_cid.to_string(),
        gap_id,
        status,
        report: report_label,
        evidence: evidence.to_string(),
        commits,
        provider: attribution.provider(&author),
        steward: attribution.steward.clone(),
        occurred_at,
        record_cid: appended.then(|| record_cid.to_string()),
        appended,
        already_fulfilled,
    };

    if appended {
        store.append(record)?;
    }
    Ok(outcome)
}

/// Resolve `--on` to `(commitment cid, commitment)`: an address this sidecar holds, or a gap id.
///
/// The gap-id arm picks the NEWEST active commitment carrying that id — newest by `valid_from`
/// with sidecar append order as the tie-break, the same ordering rule
/// [`crate::flow::read::commitment_latest_event`] holds. A superseded claim and the claim that
/// took it over both carry the id; discharging the older one would leave the live promise open.
fn resolve_commitment(
    on: &str,
    records: &[(Cid, FlowRecord)],
) -> FlowResult<(Cid, elohim_epr_rea::Commitment)> {
    if let Ok(cid) = on.parse::<Cid>() {
        for (record_cid, record) in records {
            if record_cid == &cid {
                return match record {
                    FlowRecord::Commitment(c) => Ok((cid, c.clone())),
                    _ => Err(FlowError::InvalidArguments(format!(
                        "{on} is not a Commitment in this sidecar — a fulfilment discharges a \
                         promise, and only a commitment is one"
                    ))),
                };
            }
        }
        return Err(FlowError::UnknownResource(format!(
            "{on} (a well-formed CID, but no record in this sidecar mints it)"
        )));
    }

    let mut candidates: Vec<(usize, Cid, &elohim_epr_rea::Commitment)> = records
        .iter()
        .enumerate()
        .filter_map(|(index, (cid, record))| match record {
            FlowRecord::Commitment(c)
                if c.state == CommitmentState::Active
                    && c.resource_spec.classified_as.iter().any(|s| s == on) =>
            {
                Some((index, *cid, c))
            }
            _ => None,
        })
        .collect();
    candidates.sort_by(|a, b| {
        let left = OccurredAtKey::parse(a.2.valid_from.as_deref().unwrap_or_default());
        let right = OccurredAtKey::parse(b.2.valid_from.as_deref().unwrap_or_default());
        left.cmp(&right).then_with(|| a.0.cmp(&b.0))
    });
    match candidates.last() {
        Some((_, cid, commitment)) => Ok((*cid, (*commitment).clone())),
        None => Err(FlowError::UnknownResource(format!(
            "{on} names no active commitment — claim it first with \
             `epr flow claim --on {on} --as agent:<role>@<model>`"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elohim_epr_rea::atom_cid;

    /// Pinned so canonical dag-cbor encoding of the fulfillment `FlowEvent` can never
    /// silently drift (mirrors `elohim_epr_rea::model::tests::depedge_cid_is_stable`).
    #[test]
    fn fulfillment_event_cid_is_stable() {
        let resource = atom_cid(&"golden-feature-body".to_string()).expect("cid");
        let commitment = atom_cid(&"golden-commitment".to_string()).expect("cid");
        let repo_scope = atom_cid(&"golden-repo-scope".to_string()).expect("cid");
        let event = FlowEvent {
            action: ReaVerb::Produce,
            provider: AgentRef(CI_AGENT.to_string()),
            receiver: AgentRef("repo:ethosengine/elohim".to_string()),
            resource,
            quantity: Magnitude::Count {
                value: 1.0,
                unit: "green-run".to_string(),
            },
            process: None,
            in_scope_of: repo_scope,
            fulfills: vec![commitment],
            satisfies: Vec::new(),
            classified_as: Vec::new(),
            occurred_at: "2026-07-25T00:00:00Z".to_string(),
        };
        let cid = atom_cid(&event).expect("cid");
        assert_eq!(
            cid.to_string(),
            "bafyreigjk7tsn6vczrzkwhetpvb5lkdh5jrw5fvhd4uu4e6rx32u5bj2ze"
        );
    }
}
