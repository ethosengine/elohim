//! Advisory seam guidance. No remote inference, publication, or persistence occurs here.
//! Reports are ephemeral: reconstruct from the private proposal and versioned declarations.
pub mod adapters;
pub mod classify;
pub mod declarations;
mod method;
mod review;

use crate::{
    error::{Error, Result},
    repository_validators::ElohimRepositoryValidators,
};
use adapters::Proposal;
use declarations::{identity, MAX_BYTES};
use eprfs_meta::{evaluate_path_with, GovernanceWrite};
use serde_json::{json, Value};
use std::{io::Read, path::Path};

pub fn command(root: &Path, args: &[String]) -> Result<Value> {
    if args.first().map(String::as_str) == Some("capabilities") {
        return Ok(adapters::capabilities_at(
            root,
            args.get(1).map(String::as_str).unwrap_or("unknown"),
        ));
    }
    if !args.is_empty() {
        return Err(Error::InvalidArguments(
            "govern seam [capabilities HARNESS]; JSON request on stdin".into(),
        ));
    }
    let mut raw = String::new();
    std::io::stdin()
        .take((MAX_BYTES * 3 + 1) as u64)
        .read_to_string(&mut raw)
        .map_err(|source| Error::Read {
            path: "<stdin>".into(),
            source,
        })?;
    if raw.len() > MAX_BYTES * 3 {
        return Ok(gap("request exceeds limit"));
    }
    Ok(match serde_json::from_str::<Value>(&raw) {
        Ok(input) => evaluate(root, &input),
        Err(e) => gap(&format!("invalid request JSON: {e}")),
    })
}

fn gap(message: &str) -> Value {
    let message: String = message.chars().take(1000).collect();
    json!({"advisoryOnly": true, "coverageGaps": [message], "candidates": [],
        "packet": format!("Seam guidance coverage gap: {message}")})
}

pub fn evaluate(root: &Path, input: &Value) -> Value {
    let proposal = if input.get("proposal").is_some() {
        serde_json::from_value::<Proposal>(input["proposal"].clone()).map_err(|e| e.to_string())
    } else {
        adapters::normalize(
            root,
            input["harness"].as_str().unwrap_or("unknown"),
            &input["event"],
        )
    };
    let proposal = match proposal.and_then(|p| {
        adapters::validate(root, &p)?;
        Ok(p)
    }) {
        Ok(p) => p,
        Err(e) => return gap(&e),
    };
    let mut methods = Vec::new();
    let mut max_candidates = 3;
    let mut packet_bytes = 8192;
    let mut candidates = Vec::new();
    let mut gaps = Vec::new();
    let mut evaluations = Vec::new();
    for change in &proposal.changes {
        // Deletion and rename source must be covered too. Guidance never authorizes either.
        let targets = if let Some(from) = &change.from {
            vec![from.as_str(), &change.path]
        } else {
            vec![change.path.as_str()]
        };
        for target in targets {
            let write = GovernanceWrite {
                path: target.into(),
                content: if change.from.as_deref() == Some(target) {
                    Some(String::new())
                } else {
                    change.proposed.clone().or_else(|| Some(String::new()))
                },
                prior_content: if change.from.is_some() && target == change.path {
                    None
                } else {
                    change.prior.clone()
                },
                is_new: change.operation == "create",
                is_new_subdir: false,
            };
            match evaluate_path_with(root, root.join(target), &write, &ElohimRepositoryValidators) {
                Ok(evaluation) => {
                    let bound = evaluation.rules.iter().any(|rule| {
                        rule.parameters
                            .as_str()
                            .or_else(|| rule.parameters["ref"].as_str())
                            == Some(classify::REFERENCE)
                    });
                    if !bound {
                        gaps.push(json!(format!("no native seam binding for {target}")));
                    }

                    for verdict in &evaluation.verdicts {
                        if let Some(evidence) = &verdict.evidence {
                            if let Some(method) = evidence.get("method") {
                                max_candidates = max_candidates.min(
                                    method["application"]["maxCandidates"].as_u64().unwrap_or(3)
                                        as usize,
                                );
                                packet_bytes = packet_bytes.min(
                                    method["application"]["packetBytes"]
                                        .as_u64()
                                        .unwrap_or(8192)
                                        as usize,
                                );
                                if !methods.contains(method) {
                                    methods.push(method.clone());
                                }
                            }
                            if let Some(items) = evidence["candidates"].as_array() {
                                candidates.extend(items.iter().cloned());
                            }
                            if let Some(items) = evidence["coverageGaps"].as_array() {
                                gaps.extend(items.iter().cloned());
                            }
                        }
                    }
                    evaluations.push(serde_json::to_value(&evaluation).unwrap_or(Value::Null));
                }
                Err(e) => gaps.push(json!(format!("governance unavailable: {e}"))),
            }
        }
    }
    // Includes session/repository, every proposed byte, resolved governance and source identities.
    let binding = json!({"version": 1, "proposal": proposal, "evaluations": evaluations});
    let proposal_id = identity(
        serde_json::to_vec(&binding)
            .expect("JSON values serialize")
            .as_slice(),
    );
    let total = candidates.len();
    candidates.truncate(max_candidates);
    if total > max_candidates {
        gaps.push(json!(format!(
            "{} candidates omitted; use bounded recall",
            total - max_candidates
        )));
    }
    let mut report = json!({"advisoryOnly": true, "proposalId": proposal_id, "methods": methods, "candidates": candidates,
        "coverageGaps": gaps, "review": {"status": "not-reviewed"},
        "limitations": ["heuristic candidates are not placement rulings", "comments and examples are filtered heuristically", "no interception of arbitrary shell writes"]});
    if let Some(response) = input.get("review") {
        report["review"] = review::validate(&report, response);
    }
    let instructions = "Advisory seam review in the active agent: inspect proposed evidence and verified sources; return keep, route, split or abstain with proposalId, citations (exact source evidence objects), rationale and limitations. Native validation checks binding and citations, not judgment correctness. Use epr flow memory recall for bounded supplementary discovery; re-read sources. No automatic contribution or publication. ";
    let mut packet_report = report.clone();
    // Shrink whole candidates, never truncate JSON or split a Unicode codepoint.
    while serde_json::to_vec(&packet_report).unwrap().len() + instructions.len() > packet_bytes {
        if packet_report["candidates"]
            .as_array()
            .is_some_and(|a| !a.is_empty())
        {
            packet_report["candidates"].as_array_mut().unwrap().pop();
            packet_report["packetOmissions"] = json!("large candidate evidence omitted; retrieve via explicit native interface and bounded recall");
        } else {
            packet_report = json!({"proposalId": proposal_id, "coverageGaps": ["report exceeds packet budget; retrieve through explicit native interface"]});
            break;
        }
    }
    report["packet"] = if (report["review"]["status"] == "validated"
        && report["coverageGaps"].as_array().unwrap().is_empty())
        || (report["candidates"].as_array().unwrap().is_empty()
            && report["coverageGaps"].as_array().unwrap().is_empty())
    {
        json!("")
    } else {
        json!(format!(
            "{instructions}{}",
            serde_json::to_string(&packet_report).unwrap()
        ))
    };
    report
}
