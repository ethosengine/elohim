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
//!   `commitment_latest_event` below) — is a `Dismiss` → **regression re-commitment**: append a
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

use std::cmp::Ordering;
use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use chrono::DateTime;
use cid::Cid;
use elohim_epr_rea::{
    AgentRef, CommitmentState, FlowEvent, FlowRecord, FlowStore, Magnitude, ReaVerb,
    SidecarFlowStore,
};
use serde::{Deserialize, Serialize};

use super::{body_cid_of_file, repo_agent, repo_scope_atom, FlowError, FlowResult};

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

    let mut store = SidecarFlowStore::open(root)?;
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

/// A comparable key for an `occurred_at` (RFC3339) string: parses when possible, falls back
/// to raw-string comparison on failure — the exact fallback `saga-status.py`'s `_sort_key`
/// uses (still monotonic for same-precision UTC `Z` timestamps, per its own comment). Cross-
/// variant comparisons (one side parsed, the other raw because it failed to parse) fall back
/// to comparing the two ORIGINAL strings — never panics, never picks an arbitrary ordering.
#[derive(Debug, Clone, PartialEq, Eq)]
enum OccurredAtKey {
    Parsed(DateTime<chrono::FixedOffset>),
    Raw(String),
}

impl OccurredAtKey {
    fn parse(occurred_at: &str) -> Self {
        match DateTime::parse_from_rfc3339(occurred_at) {
            Ok(dt) => OccurredAtKey::Parsed(dt),
            Err(_) => OccurredAtKey::Raw(occurred_at.to_string()),
        }
    }

    fn raw(&self) -> String {
        match self {
            OccurredAtKey::Parsed(dt) => dt.to_rfc3339(),
            OccurredAtKey::Raw(s) => s.clone(),
        }
    }
}

impl PartialOrd for OccurredAtKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OccurredAtKey {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (OccurredAtKey::Parsed(a), OccurredAtKey::Parsed(b)) => a.cmp(b),
            (OccurredAtKey::Raw(a), OccurredAtKey::Raw(b)) => a.cmp(b),
            // Mixed parse success — one side failed to parse. Fall back to comparing the
            // original strings rather than guessing an ordering across variants.
            (a, b) => a.raw().cmp(&b.raw()),
        }
    }
}

/// Is `a` (an `occurred_at`/`generated_at` RFC3339 string) STRICTLY newer than `b`, by the
/// same comparison rule as `OccurredAtKey`?
fn is_strictly_newer(a: &str, b: &str) -> bool {
    OccurredAtKey::parse(a) > OccurredAtKey::parse(b)
}

/// The `(action, occurred_at)` of the LATEST event associated with a commitment, ordered by
/// `occurred_at` timestamp and tie-broken by sidecar append order — mirroring
/// `saga-status.py`'s `index_flow_state`/`_sort_key` EXACTLY (same two-pass association scan,
/// same tagged-list-then-stable-sort shape), so the two tools can never disagree on "latest".
/// Append order ALONE (the pre-fix behavior) diverges under replay/backfill: a delayed report
/// can append after a chronologically newer event and would then read as "latest" by position
/// even though it isn't by time (module doc above) — that's the bug this closes.
///
/// An event is "associated" the same way `saga-status.py` associates one: a `Produce` naming
/// `commit_cid` directly in its `fulfills`, or a `Dismiss` whose `resource` matches the
/// resource FIRST learned (in append order) from such a `Produce` (Dismiss events carry an
/// empty `fulfills` — see the module doc). Returns `None` if the commitment has no associated
/// events at all (never called on an undischarged commitment in practice, since callers gate
/// on the `discharged` set first).
fn commitment_latest_event(
    records: &[(Cid, FlowRecord)],
    commit_cid: &Cid,
) -> Option<(ReaVerb, String)> {
    let mut resource: Option<Cid> = None;
    let mut produce_ts: Vec<String> = Vec::new();
    for (_, record) in records {
        let FlowRecord::Event(e) = record else {
            continue;
        };
        if e.action == ReaVerb::Produce && e.fulfills.contains(commit_cid) {
            if resource.is_none() {
                resource = Some(e.resource);
            }
            produce_ts.push(e.occurred_at.clone());
        }
    }

    let mut dismiss_ts: Vec<String> = Vec::new();
    if let Some(resource) = resource {
        for (_, record) in records {
            let FlowRecord::Event(e) = record else {
                continue;
            };
            if e.action == ReaVerb::Dismiss && e.resource == resource {
                dismiss_ts.push(e.occurred_at.clone());
            }
        }
    }

    if produce_ts.is_empty() {
        return None;
    }

    // Tagged in the same order saga-status builds it: all Produce timestamps (in the order
    // encountered), THEN all Dismiss timestamps (in the order encountered) — a stable sort by
    // time then preserves that relative order among exact ties.
    let mut tagged: Vec<(OccurredAtKey, ReaVerb, String)> = produce_ts
        .into_iter()
        .map(|t| (OccurredAtKey::parse(&t), ReaVerb::Produce, t))
        .chain(
            dismiss_ts
                .into_iter()
                .map(|t| (OccurredAtKey::parse(&t), ReaVerb::Dismiss, t)),
        )
        .collect();
    tagged.sort_by(|a, b| a.0.cmp(&b.0));

    tagged
        .pop()
        .map(|(_, verb, occurred_at)| (verb, occurred_at))
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
