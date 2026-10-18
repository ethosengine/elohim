//! One deterministic candidate classifier, called only through the existing validator seam.
use eprfs_meta::{ValidatorOutcome, ValidatorRequest};
use serde_json::{json, Value};

use super::declarations::{self, evidence, identity, matches};

pub const REFERENCE: &str = "epr:validator-native-seam-guidance";

pub fn classify(request: &ValidatorRequest<'_>) -> ValidatorOutcome {
    // Pin even invalid inputs: two malformed revisions must not share a review association.
    let args = &request.rule.parameters["arguments"];
    let inputs: Vec<_> = args["declarations"]
        .as_array()
        .into_iter()
        .flatten()
        .chain(std::iter::once(&args["algorithm"]))
        .take(17)
        .filter_map(Value::as_str)
        .map(|path| {
            json!({"path": path, "sourceId": declarations::read(request.repo_root, path)
            .ok().map(|raw| identity(raw.as_bytes()))})
        })
        .collect();
    let result = classify_inner(request).map(|mut evidence| {
        evidence["inputIdentities"] = json!(inputs);
        evidence
    });
    match result {
        Ok(evidence) if evidence["candidates"].as_array().is_some_and(|a| a.is_empty())
            && evidence["coverageGaps"].as_array().is_some_and(|a| a.is_empty()) => ValidatorOutcome::Pass,
        Ok(evidence) => ValidatorOutcome::Classified {
            reason: "Advisory seam candidates; placement requires active-agent review, not keyword authority".into(), evidence,
        },
        Err(gap) => ValidatorOutcome::Classified {
            reason: format!("Seam guidance coverage gap: {gap}"),
            evidence: json!({"candidates": [], "coverageGaps": [gap], "inputIdentities": inputs}),
        },
    }
}

fn classify_inner(request: &ValidatorRequest<'_>) -> Result<Value, String> {
    let algorithm = request.rule.parameters["arguments"]["algorithm"]
        .as_str()
        .ok_or("missing classifier algorithm reference")?;
    let method = super::method::load(request.repo_root, algorithm)?;
    let refs = request.rule.parameters["arguments"]["declarations"]
        .as_array()
        .ok_or("missing declaration references")?;
    if refs.len() > 16 {
        return Err("too many declaration references".into());
    }
    let post = request
        .write
        .content
        .as_deref()
        .ok_or("proposed content unavailable")?;
    if post.len() > declarations::MAX_BYTES {
        return Err("proposed content exceeds size limit".into());
    }
    declarations::local_path(request.repo_root, &request.write.path)?;
    let proposed_id = identity(post.as_bytes());
    let prior = request.write.prior_content.as_deref().unwrap_or("");
    let old_lines: std::collections::HashSet<_> = prior.lines().collect();
    let mut candidates = Vec::new();
    let mut gaps = Vec::new();
    let mut sources = Vec::new();
    for reference in refs {
        let rel = reference
            .as_str()
            .ok_or("declaration reference is not a path")?;
        let (d, declaration_id) = match declarations::load(request.repo_root, rel) {
            Ok(d) => d,
            Err(e) => {
                gaps.push(e);
                continue;
            }
        };
        sources.push(json!({"path": rel, "sourceId": declaration_id}));
        if !matches(&d.applies_to, &request.write.path)
            || matches(&d.exclusions, &request.write.path)
        {
            continue;
        }
        let mut spans = Vec::new();
        let mut byte = 0;
        let mut block_comment = false;
        for (index, line) in post.split_inclusive('\n').enumerate() {
            let trimmed = line.trim_start();
            let comment = block_comment
                || trimmed.starts_with("//")
                || trimmed.starts_with('#')
                || trimmed.starts_with("/*")
                || trimmed.starts_with('*');
            if trimmed.starts_with("/*") {
                block_comment = true;
            }
            if line.contains("*/") {
                block_comment = false;
            }
            if !comment && !old_lines.contains(line.trim_end_matches('\n')) {
                for hint in &d.detection {
                    if let Some(offset) = line.find(hint) {
                        spans.push(json!({"path": request.write.path, "sourceId": proposed_id,
                            "startByte": byte + offset, "endByte": byte + offset + hint.len(),
                            "line": index + 1, "text": hint, "context": line.chars().take(512).collect::<String>()}));
                    }
                }
            }
            byte += line.len();
        }
        if spans.is_empty() {
            continue;
        }
        let mut contracts = Vec::new();
        let mut reuse = Vec::new();
        for (definitions, out) in [
            (&d.authoritative_contracts, &mut contracts),
            (&d.reusable_interfaces, &mut reuse),
        ] {
            for source in definitions {
                match evidence(request.repo_root, source) {
                    Ok(e) => out.push(e),
                    Err(e) => gaps.push(e),
                }
            }
        }
        let owner = matches(&d.owner, &request.write.path);
        let consumer = matches(&d.allowed_consumers, &request.write.path);
        let composition = consumer && d.composition.iter().any(|s| post.contains(s));
        // A composition hint is evidence to review, never immunity for a mixed implementation.
        candidates.push(json!({"declaration": rel, "declarationId": declaration_id, "id": d.id,
            "responsibility": d.responsibility, "owner": d.owner, "allowedConsumer": consumer,
            "kind": if owner { "owner-implementation" } else if composition { "composition-candidate" } else { "placement-candidate" },
            "reviewed": false, "uncertainty": "lexical hints cannot establish authority, exclusivity or architectural correctness",
            "evidence": spans, "contracts": contracts, "reuseCandidates": reuse}));
    }
    Ok(
        json!({"candidates": candidates, "coverageGaps": gaps, "declarations": sources, "method": method}),
    )
}
