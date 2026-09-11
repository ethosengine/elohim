//! Acceptance-aware local projection. Historical production is evidence, never an
//! instruction to hide an assertion. This reader does not change REA stock discharge.
use std::collections::HashSet;
use std::path::Path;

use cid::Cid;
use elohim_epr_rea::{Commitment, FlowEvent, FlowRecord, Magnitude, ReaVerb};
use serde::Serialize;
use sha2::{Digest, Sha256};

use super::{acceptance, body_cid_of_file, confine_under, Labels};

#[derive(Debug, Serialize)]
pub struct Reconciliation {
    pub version: &'static str,
    pub evaluator: serde_json::Value,
    pub scope_cid: String,
    pub record_set_fingerprint: String,
    pub assertions: Vec<Assertion>,
    pub historical_candidates: Vec<HistoricalCandidate>,
}

#[derive(Debug, Serialize)]
pub struct Assertion {
    pub intent: Option<String>,
    pub gap_id: Option<String>,
    pub authored_state: Option<String>,
    pub issues: Vec<String>,
    pub commitments: Vec<CommitmentEvidence>,
    pub acceptance: String,
    pub remaining_action: String,
}

#[derive(Debug, Serialize)]
pub struct CommitmentEvidence {
    pub commitment: String,
    pub fulfillments: Vec<String>,
    pub technical_reviews: Vec<String>,
    pub acceptance_records: Vec<String>,
    pub unrecognized_records: Vec<String>,
    pub acceptance: String,
    pub issues: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct HistoricalCandidate {
    pub record: String,
    pub scope_cid: String,
    pub label: String,
    pub linkage: &'static str,
}

fn slot<'a>(event: &'a FlowEvent, prefix: &str) -> Option<&'a str> {
    acceptance::slot(event, prefix)
}

fn tag(event: &FlowEvent, value: &str) -> bool {
    event.classified_as.first().is_some_and(|s| s == value)
}

fn source_issues(root: &Path, scope: &Cid, path: Option<&str>) -> Vec<String> {
    let Some(path) = path else {
        return vec![
            "current source unestablished: no resolvable source path or label for this scope"
                .into(),
        ];
    };
    let canonical = std::fs::canonicalize(root).ok();
    let actual = canonical
        .as_ref()
        .and_then(|base| confine_under(base, &base.join(path)).ok())
        .and_then(|p| body_cid_of_file(&p));
    if actual.as_ref() == Some(scope) {
        Vec::new()
    } else {
        vec![format!(
            "source missing or revised: {path}; historical scope requires revalidation"
        )]
    }
}

/// All decisions use this borrowed append-order snapshot. Labels supply candidate
/// discovery only; they never establish continuity between different content addresses.
pub fn reconcile(
    root: &Path,
    scope: &Cid,
    path: Option<&str>,
    records: &[(Cid, FlowRecord)],
) -> Reconciliation {
    let labels: Labels = std::fs::read_to_string(root.join(".eprfs/status/labels.json"))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let issues = source_issues(
        root,
        scope,
        path.or_else(|| {
            labels
                .get(&scope.to_string())
                .map(String::as_str)
                .filter(|label| !label.starts_with("repo:"))
        }),
    );
    let mut assertions = Vec::new();
    let mut linked = HashSet::new();
    for (id, record) in records {
        let FlowRecord::Intent(intent) = record else {
            continue;
        };
        if intent.in_scope_of != *scope {
            continue;
        }
        let mut commitments = Vec::new();
        for (cid, record) in records {
            if let FlowRecord::Commitment(c) = record {
                if c.satisfies.contains(id) {
                    linked.insert(*cid);
                    let mut own_issues = issues.clone();
                    if c.in_scope_of != *scope {
                        own_issues.push("commitment scope differs from intent scope".into());
                    }
                    commitments.push(commitment_evidence(root, cid, c, records, own_issues));
                }
            }
        }
        assertions.push(assertion(
            Some(id.to_string()),
            &intent.resource_spec.classified_as,
            commitments,
        ));
    }
    for (cid, record) in records {
        if let FlowRecord::Commitment(c) = record {
            if c.in_scope_of == *scope && !linked.contains(cid) {
                assertions.push(assertion(
                    None,
                    &c.resource_spec.classified_as,
                    vec![commitment_evidence(root, cid, c, records, issues.clone())],
                ));
            }
        }
    }
    if !issues.is_empty() {
        for assertion in &mut assertions {
            assertion.issues = issues.clone();
            if assertion.acceptance != "contested" {
                assertion.acceptance = "revalidation-required".into();
                assertion.remaining_action =
                    "inspect missing or revised source before continuing".into();
            }
        }
    }
    assertions.sort_by_key(|a| {
        (
            a.acceptance == "accepted",
            a.intent
                .clone()
                .unwrap_or_else(|| a.commitments[0].commitment.clone()),
        )
    });
    let mut historical_candidates = Vec::new();
    if let Some(path) = path {
        for (cid, record) in records {
            let old_scope = match record {
                FlowRecord::Intent(i) => i.in_scope_of,
                FlowRecord::Commitment(c) => c.in_scope_of,
                _ => continue,
            };
            if old_scope != *scope
                && labels
                    .get(&old_scope.to_string())
                    .is_some_and(|label| label == path)
            {
                historical_candidates.push(HistoricalCandidate {
                    record: cid.to_string(),
                    scope_cid: old_scope.to_string(),
                    label: path.into(),
                    linkage: "unverified historical label match",
                });
            }
        }
    }
    historical_candidates.sort_by(|a, b| a.record.cmp(&b.record));
    let mut hasher = Sha256::new();
    for (cid, _) in records {
        hasher.update(cid.to_string());
        hasher.update(b"\n");
    }
    Reconciliation {
        version: "acceptance-reconciliation-v1",
        evaluator: crate::govern::evaluator_identity(),
        scope_cid: scope.to_string(),
        record_set_fingerprint: format!("sha256:{:x}", hasher.finalize()),
        assertions,
        historical_candidates,
    }
}

fn assertion(
    intent: Option<String>,
    slots: &[String],
    mut commitments: Vec<CommitmentEvidence>,
) -> Assertion {
    commitments.sort_by(|a, b| a.commitment.cmp(&b.commitment));
    let acceptance = [
        "contested",
        "revalidation-required",
        "changes-requested",
        "acceptance-unestablished",
    ]
    .into_iter()
    .find(|state| commitments.iter().any(|c| c.acceptance == *state))
    .unwrap_or(if commitments.is_empty() {
        "acceptance-unestablished"
    } else {
        "accepted"
    });
    let remaining_action = match acceptance {
        "accepted" => "none within the accepted evidence scope",
        "contested" => "resolve conflicting unsuperseded decisions explicitly",
        "revalidation-required" => {
            "inspect changed or contrary evidence and obtain a fresh acceptance"
        }
        "changes-requested" => "address the acceptor's requested changes",
        _ if commitments.is_empty() => "claim and implement the assertion, then review and accept",
        _ => "complete missing fulfillment, technical review, or independent acceptance",
    };
    Assertion {
        intent,
        gap_id: slots.get(1).cloned(),
        authored_state: slots.first().cloned(),
        issues: Vec::new(),
        commitments,
        acceptance: acceptance.into(),
        remaining_action: remaining_action.into(),
    }
}

fn commitment_evidence(
    root: &Path,
    id: &Cid,
    commitment: &Commitment,
    records: &[(Cid, FlowRecord)],
    mut issues: Vec<String>,
) -> CommitmentEvidence {
    let mut fulfillments = Vec::new();
    let mut produced = Vec::new();
    let mut technical_reviews = Vec::new();
    let mut unrecognized_records = Vec::new();
    let mut decisions = Vec::new();
    for (index, (cid, record)) in records.iter().enumerate() {
        let FlowRecord::Event(e) = record else {
            continue;
        };
        if e.action == ReaVerb::Produce && e.fulfills.contains(id) {
            fulfillments.push(cid.to_string());
            produced.push((index, e.resource));
        }
        if e.resource != *id {
            continue;
        }
        let purpose = slot(e, "purpose:");
        let acceptance_metadata = [
            "appointment:",
            "fulfillment:",
            "review:",
            "report-path:",
            "report-cid:",
            "scope-cid:",
            "supersedes:",
        ]
        .iter()
        .any(|prefix| slot(e, prefix).is_some());
        if purpose.is_some_and(|purpose| purpose != "acceptance")
            || (purpose.is_none() && acceptance_metadata)
        {
            unrecognized_records.push(cid.to_string());
            issues.push(format!(
                "unrecognized acceptance metadata requires revalidation: {cid}"
            ));
        } else if tag(e, "run:verdict") && purpose.is_none() {
            technical_reviews.push(cid.to_string());
        }
        if purpose == Some("acceptance") {
            let error = acceptance::validate_record(root, records, e)
                .err()
                .map(|err| err.to_string());
            decisions.push((index, *cid, e, error));
        }
    }
    let superseded: HashSet<String> = decisions
        .iter()
        .filter(|(_, _, event, _)| acceptance::validate_structure(root, records, event).is_ok())
        .filter_map(|(_, _, e, _)| slot(e, "supersedes:").map(str::to_string))
        .collect();
    let leaves: Vec<_> = decisions
        .iter()
        .filter(|(_, cid, _, _)| !superseded.contains(&cid.to_string()))
        .collect();
    if commitment
        .resource_spec
        .classified_as
        .first()
        .is_some_and(|s| s == "a2o:scenario-green")
    {
        if let Some(path) = commitment.resource_spec.classified_as.get(1) {
            // Validate the evidence the live decision actually names. Historical
            // scenario revisions remain facts, not permanent poison for a fresh acceptance.
            let selected: Vec<Cid> = if leaves.is_empty() {
                produced
                    .last()
                    .map(|(_, resource)| vec![*resource])
                    .unwrap_or_default()
            } else {
                leaves
                    .iter()
                    .filter_map(|(_, _, e, _)| slot(e, "fulfillment:"))
                    .filter_map(|pin| records.iter().find(|(cid, _)| cid.to_string() == pin))
                    .filter_map(|(_, r)| match r {
                        FlowRecord::Event(e) => Some(e.resource),
                        _ => None,
                    })
                    .collect()
            };
            for resource in selected {
                issues.extend(source_issues(root, &resource, Some(path)));
            }
        }
    }
    let mut status = "acceptance-unestablished";
    if !leaves.is_empty() {
        status = if leaves.len() > 1 {
            "contested"
        } else {
            let (_, _, decision, error) = leaves[0];
            if error.is_some() {
                "revalidation-required"
            } else {
                if slot(decision, "verdict:") == Some("changes-requested") {
                    "changes-requested"
                } else {
                    "accepted"
                }
            }
        };
    }
    // A decision can arrive after new work while still naming an old receipt.
    // Check the explicitly examined production, not merely the note's arrival.
    if let Some((latest_index, _)) = produced.last() {
        let latest_id = records[*latest_index].0.to_string();
        for (_, _, decision, _) in &leaves {
            if slot(decision, "fulfillment:") != Some(latest_id.as_str()) {
                issues.push(format!(
                    "newer unexamined fulfillment requires revalidation: {latest_id}"
                ));
            }
        }
    }
    // No acceptance may hide a later red observation; legacy fulfilled work must
    // expose regression too. Append order witnesses arrival, not adjudication.
    let baseline = leaves
        .iter()
        .map(|(index, _, _, _)| *index)
        .max()
        .or_else(|| produced.iter().map(|(index, _)| *index).max());
    if let Some(baseline) = baseline {
        for (later_id, later) in records.iter().skip(baseline + 1) {
            if let FlowRecord::Event(e) = later {
                let concerns = e.resource == *id
                    || e.resource == commitment.in_scope_of
                    || produced.iter().any(|(_, resource)| *resource == e.resource);
                let red_run = e.action == ReaVerb::Dismiss
                    && matches!(&e.quantity, Magnitude::Count { unit, .. } if unit == "red-run");
                let contrary = red_run
                    || tag(e, "run:correction")
                    || tag(e, "a2o:scenario-red")
                    || (tag(e, "run:verdict")
                        && slot(e, "purpose:") != Some("acceptance")
                        && slot(e, "verdict:") == Some("changes-requested"));
                let new_production = e.action == ReaVerb::Produce && e.fulfills.contains(id);
                if (concerns && contrary) || new_production {
                    issues.push(format!(
                        "subsequent evidence requires revalidation: {later_id}"
                    ));
                }
            }
        }
    }
    for (_, cid, _, error) in &leaves {
        if let Some(error) = error {
            issues.push(format!("acceptance {cid}: {error}"));
        }
    }
    if !issues.is_empty() && status != "contested" {
        status = "revalidation-required";
    }
    fulfillments.sort();
    technical_reviews.sort();
    unrecognized_records.sort();
    let mut acceptance_records: Vec<_> = decisions
        .iter()
        .map(|(_, cid, _, _)| cid.to_string())
        .collect();
    acceptance_records.sort();
    CommitmentEvidence {
        commitment: id.to_string(),
        fulfillments,
        technical_reviews,
        acceptance_records,
        unrecognized_records,
        acceptance: status.into(),
        issues,
    }
}

impl Reconciliation {
    /// Text and JSON share the same decisions; this only limits presentation.
    pub fn render_text(&self, limit: usize) -> String {
        use std::fmt::Write;
        let mut text = format!(
            "\n  RECONCILIATION ({} assertions; unresolved first, CID order)\n",
            self.assertions.len()
        );
        for assertion in self.assertions.iter().take(limit) {
            let _ = writeln!(
                text,
                "    {} [{}] — {}",
                assertion
                    .gap_id
                    .as_deref()
                    .unwrap_or("(unlabelled assertion)"),
                assertion.acceptance,
                assertion.remaining_action
            );
            for issue in &assertion.issues {
                let _ = writeln!(text, "      {issue}");
            }
            for c in &assertion.commitments {
                let _ = writeln!(
                    text,
                    "      {}: fulfillment {} · technical review {} · acceptance {}",
                    c.commitment,
                    c.fulfillments.len(),
                    c.technical_reviews.len(),
                    c.acceptance_records.len()
                );
                for issue in &c.issues {
                    let _ = writeln!(text, "      {issue}");
                }
            }
        }
        if self.assertions.len() > limit {
            let _ = writeln!(
                text,
                "    … and {} more assertions (--json for all)",
                self.assertions.len() - limit
            );
        }
        let _ = writeln!(
            text,
            "  HISTORICAL CANDIDATES ({}; linkage unverified)",
            self.historical_candidates.len()
        );
        for candidate in self.historical_candidates.iter().take(limit) {
            let _ = writeln!(text, "    {} — {}", candidate.record, candidate.label);
        }
        if self.historical_candidates.len() > limit {
            let _ = writeln!(
                text,
                "    … and {} more historical candidates (--json for all)",
                self.historical_candidates.len() - limit
            );
        }
        text
    }
}
