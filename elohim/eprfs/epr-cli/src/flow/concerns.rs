//! Read-only, edge-paged concern projection over the existing merged native graph.
//! Drift is a reason to investigate, never a substantive conflict or permission to reseal.
use std::collections::BTreeMap;
use std::fmt::Write;
use std::path::Path;

use elohim_epr_rea::{Governor, MemoryFlowStore, SidecarFlowStore};
use serde::Serialize;

use super::edges::{self, EdgeIndex, EdgePlane, SealForm, Verdict};
use super::{confine_under, rel_to_root, FlowError, FlowResult};

#[derive(Debug, Serialize)]
pub struct EdgeSlot {
    pub plane: EdgePlane,
    pub from: String,
    pub to: String,
    pub governor: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ConcernEdge {
    pub slot: EdgeSlot,
    pub verdict: String,
    pub sealed_evidence: Option<String>,
    pub current_evidence: Option<String>,
    pub current_fingerprint: Option<String>,
    pub source_readable: bool,
    pub consumer_readable: bool,
    pub consumer_evidence: Option<String>,
    /// Open native context on this path to obtain exact assertion/intent/review CIDs.
    /// A path is a locator, not an assertion identity or evidence of acceptance.
    pub assertion_context: String,
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct ConcernGroup {
    pub source: String,
    pub edges: Vec<ConcernEdge>,
}

#[derive(Debug, Default, Serialize)]
pub struct Counts {
    pub total_edges: usize,
    pub stale: usize,
    pub dangling: usize,
    pub held: usize,
    pub governed: usize,
    pub ok: usize,
    /// Sidecar slots WITHDRAWN by `epr flow retract` and therefore absent from `total_edges`.
    ///
    /// Counted so a withdrawal can never read as an absence. The whole point of the retraction
    /// verb is that a slot stops being offered as a concern; the whole point of this number is
    /// that the reader can still see how many stopped, and `omissions` says on what ground.
    pub retracted: usize,
    pub selected_edges: usize,
    pub groups: usize,
}

#[derive(Debug, Serialize)]
pub struct Page {
    pub offset: usize,
    pub limit: usize,
    pub returned_edges: usize,
    pub omitted_edges: usize,
    pub next_offset: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct Concerns {
    pub scope: String,
    pub scope_kind: String,
    pub selection_rule: String,
    pub order: String,
    pub counts: Counts,
    pub page: Page,
    pub groups: Vec<ConcernGroup>,
    pub omissions: Vec<String>,
}

pub fn concerns(root: &Path, target: &str, offset: usize, limit: usize) -> FlowResult<Concerns> {
    concerns_with(root, target, offset, limit, false)
}

/// Include all native states only when explicitly requested, for exact-slot resumption.
pub fn concerns_with(
    root: &Path,
    target: &str,
    offset: usize,
    limit: usize,
    all_states: bool,
) -> FlowResult<Concerns> {
    if target.parse::<cid::Cid>().is_ok() {
        return Err(FlowError::InvalidArguments(
            "--concerns needs a repository path; use ordinary context for a content address".into(),
        ));
    }
    if !(1..=100).contains(&limit) {
        return Err(FlowError::InvalidArguments(
            "--limit must be between 1 and 100 edges".into(),
        ));
    }
    let root = std::fs::canonicalize(root).map_err(|source| FlowError::Read {
        path: root.to_path_buf(),
        source,
    })?;
    let path = confine_under(&root, &root.join(target))?;
    let directory = path.is_dir();
    let scope = rel_to_root(&root, &path);
    let scope = if scope.is_empty() { ".".into() } else { scope };
    let sidecar_present = root.join(".eprfs/status/flows.jsonl").exists();
    let index = if sidecar_present {
        EdgeIndex::build(&root, &SidecarFlowStore::open(&root)?)?
    } else {
        // SidecarFlowStore::open initializes a missing store. A concern read must not.
        EdgeIndex::build(&root, &MemoryFlowStore::default())?
    };
    let mut omissions = vec![
        "Only registry-declared sealed doc citations and native sidecar edges are indexed; unsealed references and undeclared document families are outside this view.".into(),
        "Counts cover incident edges in the declared scope, not all repository assertions. Paging bounds output, not graph scan or source bytes.".into(),
        "Each page recomputes the current graph; offsets are not snapshot identities. Restart paging after changing source or native records.".into(),
        "No semantic ranking or substantive repair/conflict judgment is inferred. Open each consumer's native context for assertion, review and acceptance records; it may have none.".into(),
    ];
    if !sidecar_present {
        omissions
            .push("Native sidecar absent: no native edge records observed; doc plane only.".into());
    }
    match edges::load_registry(&root) {
        None => omissions.push(
            "Recipe registry missing or unreadable: doc plane unavailable; sidecar edges only."
                .into(),
        ),
        Some(registry) => {
            let unreadable = edges::doc_corpus(&root, &registry)
                .iter()
                .filter(|(_, abs)| std::fs::read_to_string(abs).is_err())
                .count();
            if unreadable > 0 {
                omissions.push(format!(
                    "{unreadable} registry doc sources unreadable and omitted from the doc plane."
                ));
            }
        }
    }
    let in_scope =
        |p: &str| scope == "." || p == scope || (directory && p.starts_with(&format!("{scope}/")));
    let mut counts = Counts::default();
    // Withdrawn slots are NOT in `index.edges`, so they can never be selected as a concern. They
    // are counted and named here instead, because a concern page that quietly shrank would make
    // `epr flow retract` a way to hide drift rather than to record a decision about it.
    let withdrawn_in_scope: Vec<&edges::WithdrawnEdge> = index
        .withdrawn
        .iter()
        .filter(|w| in_scope(&w.from) || in_scope(&w.to))
        .collect();
    counts.retracted = withdrawn_in_scope.len();
    if !withdrawn_in_scope.is_empty() {
        omissions.push(format!(
            "{} sidecar slot(s) withdrawn by retraction and excluded from this view: {}. The \
             records remain in the append-only sidecar; `epr flow context <record-cid>` reads \
             each withdrawal and its ground.",
            withdrawn_in_scope.len(),
            withdrawn_in_scope
                .iter()
                .map(|w| format!("{} → {} ({})", w.from, w.to, w.reason))
                .collect::<Vec<_>>()
                .join("; ")
        ));
    }
    let mut selected = Vec::new();
    let mut source_evidence = BTreeMap::new();
    for edge in index
        .edges
        .iter()
        .filter(|e| in_scope(&e.from) || in_scope(&e.to))
    {
        counts.total_edges += 1;
        // One source read per upstream per projection: grouping shares investigation, and
        // the displayed current pin is exactly the evidence used for this verdict.
        let current =
            if all_states || (matches!(edge.governor, Governor::CiteSeal) && edge.held.is_none()) {
                source_evidence
                    .entry(edge.to.clone())
                    .or_insert_with(|| edges::recompute_upstream(&root, edge))
                    .clone()
            } else {
                None
            };
        let verdict = edges::derive_verdict(edge, current.as_ref());
        match verdict {
            Verdict::Stale => counts.stale += 1,
            Verdict::Dangling => counts.dangling += 1,
            Verdict::Held(_) => counts.held += 1,
            Verdict::Governed(_) => counts.governed += 1,
            Verdict::Ok => counts.ok += 1,
        }
        if all_states || matches!(verdict, Verdict::Stale | Verdict::Dangling) {
            selected.push((edge, verdict, current));
        }
    }
    selected.sort_by_key(|(e, _, _)| {
        (
            &e.to,
            &e.from,
            governor_label(e),
            format!("{:?}", e.plane),
            &e.desc,
        )
    });
    counts.selected_edges = selected.len();
    counts.groups = selected
        .iter()
        .map(|(e, _, _)| &e.to)
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    let mut groups: BTreeMap<String, Vec<ConcernEdge>> = BTreeMap::new();
    for (edge, verdict, current) in selected.into_iter().skip(offset).take(limit) {
        let sealed_evidence = match &edge.seal {
            SealForm::Short(s) => Some(s.clone()),
            SealForm::Full(c) => Some(c.to_string()),
            SealForm::None => None,
        };
        let consumer = super::body_cid_of_file(&root.join(&edge.from));
        groups.entry(edge.to.clone()).or_default().push(ConcernEdge {
            slot: EdgeSlot { plane: edge.plane, from: edge.from.clone(), to: edge.to.clone(), governor: governor_label(edge), description: edge.desc.clone() },
            verdict: verdict.word().into(),
            sealed_evidence,
            current_evidence: current.as_ref().map(|c| c.as_cid().to_string()),
            current_fingerprint: current.as_ref().map(|c| c.short_fingerprint()),
            source_readable: current.is_some(),
            consumer_readable: consumer.is_some(),
            consumer_evidence: consumer.map(|cid| cid.to_string()),
            assertion_context: edge.from.clone(),
            reason: match verdict {
                Verdict::Stale => "Sealed upstream differs from current source; substantive relationship is unreviewed.".into(),
                Verdict::Ok => "Current source matches the seal; substantive acceptance remains a separate judgment.".into(),
                Verdict::Held(reason) => format!("Declared held edge: {reason}"),
                Verdict::Governed(governor) => format!("Conformance belongs to {governor}; not a cite-seal acceptance judgment."),
                Verdict::Dangling => "Seal or readable upstream evidence is missing; no substantive judgment is established.".into(),
            },
        });
    }
    let returned_edges = groups.values().map(Vec::len).sum();
    let next = offset.saturating_add(returned_edges);
    let page = Page {
        offset,
        limit,
        returned_edges,
        omitted_edges: counts.selected_edges.saturating_sub(returned_edges),
        next_offset: (next < counts.selected_edges).then_some(next),
    };
    Ok(Concerns {
        scope, scope_kind: if directory { "directory" } else { "file" }.into(),
        selection_rule: if all_states {
            "All incident native edge states explicitly selected for exact-slot inspection; group by shared upstream source."
        } else {
            "Incident edges with native stale or dangling verdict; group by shared upstream source. Held/governed/ok counted but excluded."
        }.into(),
        order: "upstream path, consumer path, governor, plane, description; ascending; pages may split a source group".into(),
        counts, page, groups: groups.into_iter().map(|(source, edges)| ConcernGroup { source, edges }).collect(), omissions,
    })
}

fn governor_label(edge: &edges::IndexedEdge) -> String {
    edges::governor_label(&edge.governor)
}

impl Concerns {
    pub fn render_text(&self) -> String {
        let mut text = format!("epr flow context — concerns in {}\n{}\nOrder: {}\nScope: {} indexed edges; {} stale, {} dangling, {} held, {} governed, {} ok; {} retracted (withdrawn, not indexed); {} source groups\nPage: {} edges shown, {} omitted; next offset: {:?}\n", self.scope, self.selection_rule, self.order, self.counts.total_edges, self.counts.stale, self.counts.dangling, self.counts.held, self.counts.governed, self.counts.ok, self.counts.retracted, self.counts.groups, self.page.returned_edges, self.page.omitted_edges, self.page.next_offset);
        for group in &self.groups {
            let _ = writeln!(text, "\nSource: {}", group.source);
            for edge in &group.edges {
                let _ = writeln!(text, "  {} [{} / {}] {}\n    {}\n    sealed: {:?}; current: {:?}\n    open context: {}", edge.slot.from, edge.verdict, edge.slot.governor, edge.slot.description.as_deref().unwrap_or("purpose unknown"), edge.reason, edge.sealed_evidence, edge.current_evidence, edge.assertion_context);
            }
        }
        for omission in &self.omissions {
            let _ = writeln!(text, "Limit: {omission}");
        }
        text
    }
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// `--corrections` — unresolved corrections by IDENTITY, never by date
// ───────────────────────────────────────────────────────────────────────────────────────────────

/// The kinds of note that may discharge a correction.
///
/// `run:failed-approach` is deliberately absent: an approach that did not work closes nothing, and
/// admitting it would let "we tried and stopped" read as "resolved" — the same inference, one layer
/// down, that the date heuristic made at the filename level.
const CLOSURE_KINDS: [&str; 4] = [
    "run:observation",
    "run:ruling",
    "run:verdict",
    "run:correction",
];

/// One `run:correction` record, addressed by its own CID.
#[derive(Debug, Serialize)]
pub struct Correction {
    /// The record's atom CID — the identity a closure must name.
    pub cid: String,
    /// The subject label the note was written against (`classified_as[1]`).
    pub target: String,
    /// The resource the note refers to; a closure must refer to the SAME one.
    pub resource: String,
    pub occurred_at: String,
    /// `occurred_at`'s date, for display only. It is never consulted to decide resolution.
    pub date: String,
    pub reason: String,
    /// Whether the authored body opens with `STALE`, the kit's surface convention.
    pub stale: bool,
    /// The exact native command that would close this correction.
    pub closure_template: Vec<String>,
}

/// One closure act, named by the note that made it.
#[derive(Debug, Serialize)]
pub struct Closure {
    pub correction_cid: String,
    /// The CID of the note that carried `closes:`.
    pub closed_by: String,
    pub kind: String,
    pub occurred_at: String,
    pub reason: String,
}

#[derive(Debug, Default, Serialize)]
pub struct CorrectionCounts {
    pub corrections: usize,
    pub unresolved: usize,
    pub resolved: usize,
    pub stale_unresolved: usize,
    pub omitted_by_since: usize,
    pub surfaces: usize,
}

#[derive(Debug, Serialize)]
pub struct Corrections {
    pub since: Option<String>,
    pub counts: CorrectionCounts,
    /// Unresolved rows grouped by the surface they were written against.
    pub surfaces: BTreeMap<String, Vec<Correction>>,
    /// Closures observed, keyed by the correction CID each one discharges.
    pub resolved: BTreeMap<String, Vec<Closure>>,
    pub issues: Vec<String>,
    pub selection_rule: String,
    pub omissions: Vec<String>,
}

/// Read unresolved corrections from the flow plane by identity.
///
/// The whole method is one sentence: a `run:correction` is unresolved unless some LATER note whose
/// SUBJECT LABEL is the same carries `closes:<its exact CID>`. The label, not the resource CID: a
/// repair changes the bytes the correction pinned, so resource equality would admit only no-op
/// closures (see the label check below). Nothing here reads a filename, a chronicle
/// date or a ceremony's timestamp, because none of those can say WHICH correction was absorbed —
/// which is how the previous reading both hid live corrections and retired resolved ones.
///
/// `--since` is an explicit presentation filter over the display list. It never closes anything,
/// and the count it suppressed is reported rather than silently dropped.
pub fn corrections(root: &Path, since: Option<&str>) -> FlowResult<Corrections> {
    use elohim_epr_rea::{FlowRecord, FlowStore, Magnitude};

    if let Some(value) = since {
        let valid = value.len() == 10
            && value.as_bytes()[4] == b'-'
            && value.as_bytes()[7] == b'-'
            && value
                .split('-')
                .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()));
        if !valid {
            return Err(FlowError::InvalidArguments(
                "--since must be an ISO YYYY-MM-DD date".into(),
            ));
        }
    }
    let mut issues = Vec::new();
    let sidecar = root.join(".eprfs/status/flows.jsonl");
    let records = if sidecar.exists() {
        SidecarFlowStore::open(root)?.records()?
    } else {
        // Asking a question must not create a store.
        issues.push("correction ledger is missing".to_string());
        Vec::new()
    };

    let is_note = |event: &elohim_epr_rea::FlowEvent| matches!(&event.quantity, Magnitude::Count { unit, .. } if unit == super::note::NOTE_UNIT);
    let slot = |event: &elohim_epr_rea::FlowEvent, prefix: &str| {
        event
            .classified_as
            .iter()
            .find_map(|value| value.strip_prefix(prefix).map(str::to_string))
    };

    let mut found: Vec<Correction> = Vec::new();
    let mut positions: BTreeMap<String, usize> = BTreeMap::new();
    for (index, (cid, record)) in records.iter().enumerate() {
        let FlowRecord::Event(event) = record else {
            continue;
        };
        if !is_note(event)
            || event.classified_as.first().map(String::as_str) != Some("run:correction")
        {
            continue;
        }
        // A correction that CARRIES `closes:` is a closure act, not an open concern of its own.
        // Without this, every closure written as `--kind correction` would arrive as a fresh
        // unresolved correction needing its own closure — a treadmill where the list can never
        // reach zero however much work is actually discharged.
        if event
            .classified_as
            .iter()
            .any(|slot| slot.starts_with(super::note::CLOSES_SLOT_PREFIX))
        {
            continue;
        }
        let Some(target) = event.classified_as.get(1) else {
            issues.push(format!(
                "ledger record {index}: correction names no target slot"
            ));
            continue;
        };
        let Some(reason) = slot(event, super::note::REASON_SLOT_PREFIX) else {
            issues.push(format!(
                "ledger record {index}: correction carries no authored reason"
            ));
            continue;
        };
        let cid = cid.to_string();
        if positions.contains_key(&cid) {
            // Identity is the atom CID, so a repeated one is the SAME record, never a second event.
            continue;
        }
        positions.insert(cid.clone(), index);
        found.push(Correction {
            closure_template: vec![
                "epr".into(),
                "flow".into(),
                "note".into(),
                "--on".into(),
                target.clone(),
                "--kind".into(),
                "correction".into(),
                "--closes".into(),
                cid.clone(),
                "--reason".into(),
                "<the observed repair and the evidence that shows it>".into(),
            ],
            date: event.occurred_at.chars().take(10).collect(),
            stale: reason.trim_start().starts_with("STALE"),
            cid,
            target: target.clone(),
            resource: event.resource.to_string(),
            occurred_at: event.occurred_at.clone(),
            reason,
        });
    }

    let by_cid: BTreeMap<&str, &Correction> =
        found.iter().map(|row| (row.cid.as_str(), row)).collect();
    let mut resolved: BTreeMap<String, Vec<Closure>> = BTreeMap::new();
    for (index, (cid, record)) in records.iter().enumerate() {
        let FlowRecord::Event(event) = record else {
            continue;
        };
        if !is_note(event) {
            continue;
        }
        let Some(kind) = event.classified_as.first() else {
            continue;
        };
        if !CLOSURE_KINDS.contains(&kind.as_str()) {
            continue;
        }
        let Some(closed) = slot(event, super::note::CLOSES_SLOT_PREFIX) else {
            continue;
        };
        let Some(correction) = by_cid.get(closed.as_str()) else {
            issues.push(format!(
                "ledger record {index}: closure names unknown correction {closed}"
            ));
            continue;
        };
        // Identity is matched on the SUBJECT LABEL, not on the resource CID.
        //
        // This is the one place the distinction is load-bearing. A correction pins the bytes it was
        // written against; a repair CHANGES those bytes, so the closure's resource CID is expected
        // to differ. Requiring resource equality would make every genuine repair unable to close
        // the correction it repaired, while leaving only no-op closures admissible. The exact
        // correction is already named by `closes:<cid>`; the label check is what stops a closure
        // for one surface being read as a closure on another.
        if event.classified_as.get(1).map(String::as_str) != Some(correction.target.as_str()) {
            issues.push(format!(
                "ledger record {index}: closure for {closed} names a different subject"
            ));
            continue;
        }
        if positions.get(&closed).is_some_and(|at| index <= *at) {
            issues.push(format!(
                "ledger record {index}: closure for {closed} is not later than the correction"
            ));
            continue;
        }
        resolved.entry(closed).or_default().push(Closure {
            correction_cid: correction.cid.clone(),
            closed_by: cid.to_string(),
            kind: kind.clone(),
            occurred_at: event.occurred_at.clone(),
            reason: slot(event, super::note::REASON_SLOT_PREFIX).unwrap_or_default(),
        });
    }

    let mut counts = CorrectionCounts {
        corrections: found.len(),
        resolved: resolved.len(),
        ..CorrectionCounts::default()
    };
    let mut surfaces: BTreeMap<String, Vec<Correction>> = BTreeMap::new();
    for row in found {
        if resolved.contains_key(&row.cid) {
            continue;
        }
        if since.is_some_and(|value| row.date.as_str() < value) {
            counts.omitted_by_since += 1;
            continue;
        }
        counts.unresolved += 1;
        if row.stale {
            counts.stale_unresolved += 1;
        }
        surfaces.entry(row.target.clone()).or_default().push(row);
    }
    counts.surfaces = surfaces.len();
    Ok(Corrections {
        since: since.map(str::to_string),
        counts,
        surfaces,
        resolved,
        issues,
        selection_rule: "Every run:correction record with no LATER observation/ruling/verdict/correction note carrying closes:<its exact CID> under the same subject label. The label, not the resource CID: a repair changes the bytes the correction pinned.".into(),
        omissions: vec![
            "Closure is an explicit act by identity. A ceremony that ran afterwards, a dated chronicle entry and a passing test close nothing.".into(),
            "Only the local flow plane is read; corrections recorded on another peer's chain are outside this view.".into(),
            "--since filters the DISPLAY list only; suppressed rows are counted, never resolved.".into(),
        ],
    })
}

impl Corrections {
    pub fn render_text(&self) -> String {
        let mut text = format!(
            "Unresolved corrections: {} ({} STALE); resolved by explicit closure: {}\n{}\n",
            self.counts.unresolved,
            self.counts.stale_unresolved,
            self.counts.resolved,
            self.selection_rule
        );
        for (target, rows) in &self.surfaces {
            let _ = writeln!(text, "\n{target}");
            for row in rows {
                let _ = writeln!(text, "  {} [{}] {}", row.cid, row.date, row.reason);
                let _ = writeln!(
                    text,
                    "    close it: {}",
                    row.closure_template
                        .iter()
                        .map(|word| if word.contains(' ') || word.contains('<') {
                            format!("'{word}'")
                        } else {
                            word.clone()
                        })
                        .collect::<Vec<_>>()
                        .join(" ")
                );
            }
        }
        if self.counts.omitted_by_since > 0 {
            let _ = writeln!(
                text,
                "\nExplicit --since omitted {} unresolved corrections",
                self.counts.omitted_by_since
            );
        }
        for omission in &self.omissions {
            let _ = writeln!(text, "Limit: {omission}");
        }
        for issue in &self.issues {
            let _ = writeln!(text, "REVALIDATE: {issue}");
        }
        text
    }
}
