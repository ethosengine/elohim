//! Validate association and cited bytes, never the semantic correctness of an agent's judgment.
use serde_json::{json, Value};

pub fn validate(report: &Value, response: &Value) -> Value {
    let invalid = |why| json!({"status": "invalid", "reason": why});
    if response["proposalId"] != report["proposalId"] {
        return invalid("stale proposal or evidence identity");
    }
    if !["keep", "route", "split", "abstain"].contains(&response["decision"].as_str().unwrap_or(""))
    {
        return invalid("unsupported review decision");
    }
    if response["rationale"]
        .as_str()
        .is_none_or(|s| s.trim().is_empty())
        || response["limitations"]
            .as_str()
            .is_none_or(|s| s.trim().is_empty())
    {
        return invalid("rationale and limitations required");
    }
    let Some(citations) = response["citations"].as_array() else {
        return invalid("citations array required");
    };
    if citations.is_empty() && response["decision"] != "abstain" {
        return invalid("non-abstaining review needs evidence");
    }
    let sources: Vec<&Value> = report["candidates"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|c| {
            ["evidence", "contracts", "reuseCandidates"]
                .into_iter()
                .flat_map(move |k| c[k].as_array().into_iter().flatten())
        })
        .collect();
    if citations.iter().any(|c| !sources.contains(&c)) {
        return invalid("citation is not exact freshly verified evidence");
    }
    json!({"status": "validated", "decision": response["decision"], "rationale": response["rationale"],
        "limitations": response["limitations"], "citations": citations, "semanticCorrectness": "not-certified"})
}
