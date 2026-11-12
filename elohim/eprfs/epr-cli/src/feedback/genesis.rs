//! Genesis adapter. Raw messages, scenario paths, URLs, screenshots and environment values
//! are excluded from automatic retention. Their original evidence is content-addressed here;
//! an author may attach richer evidence deliberately. No fulfilment or publication call exists.
use super::Result;
use elohim_epr_rea::{
    atom_cid,
    feedback::{Feedback, Occurrence, Outcome, Policy, Source},
    feedback_store::FeedbackStore,
};
use serde_json::{json, Value};

fn identity(value: &Value) -> Result<String> {
    Ok(atom_cid(value)?.to_string())
}
fn count(value: &Value, key: &str) -> Result<u64> {
    value[key]
        .as_u64()
        .ok_or_else(|| format!("invalid Genesis count: {key}").into())
}

fn measured_duration(rollup: &Value) -> Option<u64> {
    let scenarios = rollup["scenarios"].as_array()?;
    if scenarios.is_empty() {
        return None;
    }
    scenarios.iter().try_fold(0u64, |sum, scenario| {
        sum.checked_add(scenario["durationMs"].as_u64()?)
    })
}

pub(super) fn ingest(store: &FeedbackStore, policy: &Policy, report: &Value) -> Result<Value> {
    let run = report["runId"]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or("Genesis runId required")?;
    let at = report["generatedAt"]
        .as_str()
        .ok_or("Genesis generatedAt required")?;
    let at = chrono::DateTime::parse_from_rfc3339(at)?
        .to_utc()
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let findings = report["findings"]
        .as_array()
        .ok_or("Genesis findings required")?;
    let concerns = report["summary"]["byConcern"]
        .as_object()
        .ok_or("Genesis summary.byConcern required")?;
    let mut canonical = report.clone();
    canonical
        .as_object_mut()
        .ok_or("Genesis report must be an object")?
        .remove("generatedAt");
    let provenance = identity(&canonical)?;
    let source_run = identity(&json!(run))?;
    let sut = report["env"]["sut"]
        .as_str()
        .filter(|value| !value.is_empty() && *value != "unknown")
        .map(|value| identity(&json!(value)))
        .transpose()?;
    let mut records = vec![];
    for finding in findings {
        let fingerprint = finding["fingerprint"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or("Genesis finding fingerprint required")?;
        let fingerprint = identity(&json!(fingerprint))?;
        let source = finding["source"]
            .as_str()
            .ok_or("Genesis finding source required")?;
        if ![
            "console-error",
            "page-error",
            "failed-request",
            "scenario-failure",
            "pending-step",
            "coverage-gap",
            "visual-regression",
        ]
        .contains(&source)
        {
            return Err("unknown Genesis finding source".into());
        }
        let severity = finding["severity"]
            .as_str()
            .ok_or("Genesis finding severity required")?;
        if !["error", "warning", "info"].contains(&severity) {
            return Err("unknown Genesis severity".into());
        }
        let occurrences = count(finding, "occurrences")?;
        if occurrences == 0 {
            return Err("Genesis finding count must be positive".into());
        }
        records.push(Occurrence {source:Source {producer:"genesis".into(),run:source_run.clone(),item:format!("finding:{fingerprint}")},
            observed_at:at.clone(),participant:None,subject:format!("genesis:finding:{fingerprint}"),
            outcome:Outcome::Reported,count:occurrences,duration_ms:None,impact:Some(severity.into()),message:None,
            detail:json!({"adapter":"genesis-finding@1","source":source,"sut":sut,"provenance":provenance,"aggregate":true,
                "suggestedObjective":"not retained automatically; proposal requires contextual judgment"})});
    }
    // Include declared-but-absent concerns as unmeasured, keeping the denominator distinct.
    let mut names: Vec<String> = concerns.keys().cloned().collect();
    if let Some(declared) = report["declared"]["concerns"].as_array() {
        for name in declared {
            let name = name.as_str().ok_or("invalid declared concern")?;
            if !names.iter().any(|n| n == name) {
                names.push(name.into());
            }
        }
    }
    for name in names {
        let rollup = concerns.get(&name);
        let (passed, failed, pending, skipped) = match rollup {
            Some(r) => (
                count(r, "passed")?,
                count(r, "failed")?,
                count(r, "pending")?,
                count(r, "skipped")?,
            ),
            None => (0, 0, 0, 0),
        };
        if skipped > pending {
            return Err("Genesis skipped must remain a subset of pending".into());
        }
        let outcome = if failed > 0 {
            Outcome::Failed
        } else if pending > 0 {
            if pending == skipped {
                Outcome::Skipped
            } else {
                Outcome::Unmeasured
            }
        } else if passed > 0 {
            Outcome::Passed
        } else {
            Outcome::Unmeasured
        };
        let name = identity(&json!(name))?;
        records.push(Occurrence {source:Source {producer:"genesis".into(),run:source_run.clone(),item:format!("concern:{name}")},
            observed_at:at.clone(),participant:None,subject:format!("genesis:concern:{name}"),outcome,count:1,duration_ms:rollup.and_then(measured_duration),
            impact:None,message:None,detail:json!({"adapter":"genesis-concern@1","sut":sut,"provenance":provenance,
                "passed":passed,"failed":failed,"pending":pending,"skipped":skipped,"presentInRun":rollup.is_some(),
                "declaredCoverageKnown":report["declared"].is_object() && report["declared"]["unreadable"].is_null(),
                "declaredCoverage":identity(&report["declared"])?})});
    }
    let summary = &report["summary"]["scenarios"];
    let run_outcome = if summary["failed"].as_u64().unwrap_or(0) > 0 {
        Outcome::Failed
    } else if summary["skipped"].as_u64().unwrap_or(0) > 0 {
        Outcome::Skipped
    } else if summary["total"]
        .as_u64()
        .is_some_and(|total| total > 0 && summary["passed"].as_u64() == Some(total))
    {
        Outcome::Passed
    } else {
        Outcome::Unmeasured
    };
    // Even an empty selection is a retained run, never a disappearing or passing experience.
    records.push(Occurrence {source:Source {producer:"genesis".into(),run:source_run.clone(),item:"run".into()},
        observed_at:at,participant:None,subject:"genesis:run".into(),outcome:run_outcome,count:1,duration_ms:summary["durationMs"].as_u64(),impact:None,message:None,
        detail:json!({"adapter":"genesis-run@1","sut":sut,"provenance":provenance,
            "total":summary["total"].as_u64(),"passed":summary["passed"].as_u64(),"failed":summary["failed"].as_u64(),"skipped":summary["skipped"].as_u64(),"pending":summary["pending"].as_u64(),
            "declaredCoverage":identity(&report["declared"])?})});
    // Validate the whole input before the first append. If storage later fills, successful
    // prefixes remain and retry is idempotent; no report verdict is manufactured from capture.
    let mut cids = vec![];
    for record in records {
        cids.push(
            store
                .append(Feedback::Occurrence(record), false)?
                .to_string(),
        );
    }
    Ok(
        json!({"sourceRun":source_run,"records":cids,"policy":policy.id,"published":false,"fulfilled":false}),
    )
}
