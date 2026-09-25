//! The first gift: drift — the network's declared recipe compared with what the consumer did.
//!
//! Self-report is only a claim; the network's understanding of the edge is DECLARED compared with
//! OBSERVED, and the gap is the signal. Two readings:
//!
//! - **Recipe drift.** Stages Jenkins ran that the recipe does not declare (`extra stage`),
//!   declared stages Jenkins never reported (`missing stage`), and declared stages observed out of
//!   the declared order (`reordered`). A declared stage Jenkins skipped (`NOT_EXECUTED`) is named
//!   as `not executed`, information rather than drift.
//! - **Masking.** A stage that ran and, per the recipe's `exercises:` binding, exercises a
//!   capability the offer's `heldCapabilities` does not disclose reads `undisclosed capability` —
//!   not merely `extra stage`. Disclosed capabilities no executed stage exercised are listed as
//!   `disclosed, not exercised` (information).
//!
//! Pure. Jenkins may ignore every line for free.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::translate::{ObservedStage, StageResult};

/// One declared stage, as drift reads the recipe: its name and its `exercises:` binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaredStage {
    pub name: String,
    pub exercises: Vec<String>,
}

/// One line of drift.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Finding {
    ExtraStage {
        stage: String,
    },
    MissingStage {
        stage: String,
    },
    Reordered {
        stage: String,
        ran_before: String,
    },
    UndisclosedCapability {
        capability: String,
        stage: String,
    },
    NotExecuted {
        stage: String,
    },
    /// Reported FAILED/ABORTED, but it started only after an earlier stage had already failed and
    /// ended: a cascade skip (wfapi gives it the same status and error as the real failure), so it
    /// never ran and exercised nothing.
    SkippedAfterFailure {
        stage: String,
        after: String,
    },
    DisclosedNotExercised {
        capability: String,
    },
}

impl Finding {
    /// Drift proper, as opposed to information.
    pub fn is_drift(&self) -> bool {
        !matches!(
            self,
            Finding::NotExecuted { .. }
                | Finding::SkippedAfterFailure { .. }
                | Finding::DisclosedNotExercised { .. }
        )
    }

    pub fn line(&self) -> String {
        match self {
            Finding::ExtraStage { stage } => format!("extra stage: {stage} (ran; not declared)"),
            Finding::MissingStage { stage } => {
                format!("missing stage: {stage} (declared; not reported)")
            }
            Finding::Reordered { stage, ran_before } => {
                format!("reordered: {stage} ran before {ran_before}, which it is declared after")
            }
            Finding::UndisclosedCapability { capability, stage } => format!(
                "undisclosed capability: {capability} (exercised by {stage}; not in the offer's \
                 heldCapabilities)"
            ),
            Finding::NotExecuted { stage } => format!("not executed: {stage}"),
            Finding::SkippedAfterFailure { stage, after } => format!(
                "skipped after failure: {stage} (reported failed; started after {after} had \
                 already failed and ended, so it never ran)"
            ),
            Finding::DisclosedNotExercised { capability } => {
                format!("disclosed, not exercised: {capability}")
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DriftReport {
    pub findings: Vec<Finding>,
}

impl DriftReport {
    pub fn drift_count(&self) -> usize {
        self.findings.iter().filter(|f| f.is_drift()).count()
    }
    pub fn is_clean(&self) -> bool {
        self.drift_count() == 0
    }
}

/// Compare observed stages (start order) with the declared recipe and the offer's disclosure.
pub fn drift(
    declared: &[DeclaredStage],
    observed: &[ObservedStage],
    held: &[String],
) -> DriftReport {
    let order: BTreeMap<&str, usize> = declared
        .iter()
        .enumerate()
        .map(|(i, s)| (s.name.as_str(), i))
        .collect();
    let seen: BTreeSet<&str> = observed.iter().map(|s| s.name.as_str()).collect();
    let held: BTreeSet<&str> = held.iter().map(String::as_str).collect();
    let mut findings = Vec::new();

    for stage in observed {
        if !order.contains_key(stage.name.as_str()) {
            findings.push(Finding::ExtraStage {
                stage: stage.name.clone(),
            });
        }
    }
    for stage in declared {
        if !seen.contains(stage.name.as_str()) {
            findings.push(Finding::MissingStage {
                stage: stage.name.clone(),
            });
        }
    }
    // A FAILED/ABORTED stage that starts at or after an earlier genuine failure has ended is a
    // cascade skip, not a run: wfapi reports it with the same status and error. A parallel sibling
    // that started before the failure ended genuinely ran, and still counts.
    let mut skipped: BTreeMap<&str, &str> = BTreeMap::new();
    let mut failure: Option<(i64, &str)> = None;
    for stage in observed {
        if !matches!(stage.result, StageResult::Failed | StageResult::Aborted) {
            continue;
        }
        match failure {
            Some((ended, by)) if stage.start_millis >= ended => {
                skipped.insert(stage.name.as_str(), by);
            }
            _ => {
                let ends = stage.start_millis + stage.duration_millis;
                if failure.is_none_or(|(ended, _)| ends < ended) {
                    failure = Some((ends, stage.name.as_str()));
                }
            }
        }
    }
    // Declared stages that ran, in the order they started: each must follow the one before it.
    let ran: Vec<&ObservedStage> = observed
        .iter()
        .filter(|s| {
            order.contains_key(s.name.as_str())
                && s.result != StageResult::NotExecuted
                && !skipped.contains_key(s.name.as_str())
        })
        .collect();
    for pair in ran.windows(2) {
        if order[pair[1].name.as_str()] < order[pair[0].name.as_str()] {
            findings.push(Finding::Reordered {
                stage: pair[0].name.clone(),
                ran_before: pair[1].name.clone(),
            });
        }
    }
    let mut exercised = BTreeSet::new();
    for stage in &ran {
        let decl = declared
            .iter()
            .find(|d| d.name == stage.name)
            .expect("ran is filtered to declared stages");
        for capability in &decl.exercises {
            exercised.insert(capability.as_str());
            if !held.contains(capability.as_str()) {
                findings.push(Finding::UndisclosedCapability {
                    capability: capability.clone(),
                    stage: stage.name.clone(),
                });
            }
        }
    }
    for stage in observed {
        if stage.result == StageResult::NotExecuted && order.contains_key(stage.name.as_str()) {
            findings.push(Finding::NotExecuted {
                stage: stage.name.clone(),
            });
        }
    }
    for stage in observed {
        if let Some(after) = skipped.get(stage.name.as_str()) {
            findings.push(Finding::SkippedAfterFailure {
                stage: stage.name.clone(),
                after: after.to_string(),
            });
        }
    }
    for capability in &held {
        if !exercised.contains(capability) {
            findings.push(Finding::DisclosedNotExercised {
                capability: capability.to_string(),
            });
        }
    }
    DriftReport { findings }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn declared(name: &str, exercises: &[&str]) -> DeclaredStage {
        DeclaredStage {
            name: name.into(),
            exercises: exercises.iter().map(|s| s.to_string()).collect(),
        }
    }
    fn ran(name: &str, at: i64, result: StageResult) -> ObservedStage {
        ObservedStage {
            name: name.into(),
            result,
            start_millis: at,
            duration_millis: 1,
        }
    }

    fn at(name: &str, start: i64, dur: i64, result: StageResult) -> ObservedStage {
        ObservedStage {
            name: name.into(),
            result,
            start_millis: start,
            duration_millis: dur,
        }
    }

    #[test]
    fn a_stage_skipped_after_a_failure_exercises_nothing_but_a_parallel_sibling_does() {
        let recipe = [
            declared("gate-a", &["probe:a"]),
            declared("gate-b", &["probe:b"]),
            declared("build", &[]),
            declared("deploy", &["deploy:kube"]),
        ];
        // gate-a fails over [0, 100); gate-b runs in parallel from 10 and fails too (genuine);
        // build fails at 200 and deploy is reported failed at 205 after 55 ms: cascade skips.
        let observed = [
            at("gate-a", 0, 100, StageResult::Failed),
            at("gate-b", 10, 50, StageResult::Failed),
            at("build", 200, 60, StageResult::Failed),
            at("deploy", 205, 55, StageResult::Failed),
        ];
        let report = drift(
            &recipe,
            &observed,
            &["probe:a".into(), "deploy:kube".into()],
        );
        // deploy never ran: no undisclosed or exercised deploy capability.
        assert!(report.findings.contains(&Finding::DisclosedNotExercised {
            capability: "deploy:kube".into()
        }));
        assert!(report.findings.contains(&Finding::SkippedAfterFailure {
            stage: "deploy".into(),
            after: "gate-b".into()
        }));
        assert!(report.findings.contains(&Finding::SkippedAfterFailure {
            stage: "build".into(),
            after: "gate-b".into()
        }));
        // gate-b started before gate-a's failure ended: it ran, and its capability is undisclosed.
        assert!(report.findings.contains(&Finding::UndisclosedCapability {
            capability: "probe:b".into(),
            stage: "gate-b".into()
        }));
        assert!(!report.findings.iter().any(|f| matches!(
            f,
            Finding::UndisclosedCapability { capability, .. } if capability == "deploy:kube"
        )));
    }

    #[test]
    fn names_extra_missing_reordered_and_masked_capabilities() {
        let recipe = [
            declared("build", &[]),
            declared("push", &["registry:push"]),
            declared("deploy", &["deploy:kube"]),
        ];
        let observed = [
            ran("push", 1, StageResult::Success),
            ran("build", 2, StageResult::Success),
            ran("sneak", 3, StageResult::Success),
        ];
        let report = drift(
            &recipe,
            &observed,
            &["registry:push".into(), "unused:x".into()],
        );
        assert!(report.findings.contains(&Finding::ExtraStage {
            stage: "sneak".into()
        }));
        assert!(report.findings.contains(&Finding::MissingStage {
            stage: "deploy".into()
        }));
        assert!(report.findings.contains(&Finding::Reordered {
            stage: "push".into(),
            ran_before: "build".into()
        }));
        assert!(report.findings.contains(&Finding::DisclosedNotExercised {
            capability: "unused:x".into()
        }));
        assert!(!report.is_clean());

        // A deploy that RUNS while deploy credentials are undisclosed is masking, named as such.
        let observed = [
            ran("build", 1, StageResult::Success),
            ran("push", 2, StageResult::Success),
            ran("deploy", 3, StageResult::Failed),
        ];
        let report = drift(&recipe, &observed, &["registry:push".into()]);
        assert_eq!(
            report.findings,
            vec![Finding::UndisclosedCapability {
                capability: "deploy:kube".into(),
                stage: "deploy".into()
            }]
        );
    }

    #[test]
    fn a_skipped_stage_is_information_not_drift() {
        let recipe = [declared("build", &[]), declared("deploy", &["deploy:kube"])];
        let observed = [
            ran("build", 1, StageResult::Success),
            ran("deploy", 2, StageResult::NotExecuted),
        ];
        let report = drift(&recipe, &observed, &[]);
        assert!(report.is_clean(), "{:?}", report.findings);
        assert_eq!(
            report.findings,
            vec![Finding::NotExecuted {
                stage: "deploy".into()
            }]
        );
    }
}
