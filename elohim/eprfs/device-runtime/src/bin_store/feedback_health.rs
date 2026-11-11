//! Installed feedback qualification at the existing promotion boundary. The witness
//! is private release evidence, never permission to accept or publish a repair.
use super::{files, CommandSpec, Refusal, Result, Store};
use serde_json::{json, Value};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};

pub(super) struct FeedbackHealth {
    pub witness: Value,
    peer: Option<Store>,
    peer_snapshot: Value,
    policy: Vec<(PathBuf, Option<String>)>,
}

fn snapshot(peer: &Store) -> Value {
    let cid = peer.head("current");
    let qualification = cid
        .as_deref()
        .and_then(|id| peer.record(id))
        .and_then(|r| r.extra.get("feedbackQualification").cloned());
    json!({"cid":cid,"metadata":files::read_json::<Value>(&peer.root.join("current.json")),"qualification":qualification})
}

fn policies(repo: &Path) -> Result<Vec<(PathBuf, Option<String>)>> {
    [
        ".epr-meta/feedback-policy.json",
        ".epr-meta/feedback-capacity.json",
        ".claude/epr-meta/policies.yaml",
    ]
    .iter()
    .map(|name| {
        let path = repo.join(name);
        let cid = if path.exists() {
            Some(files::file_cid(&path)?)
        } else {
            None
        };
        Ok((path, cid))
    })
    .collect()
}

impl FeedbackHealth {
    // Both tools use the same order: simultaneous EPR/Brit promotion cannot deadlock
    // or change the qualified counterpart between revalidation and head publication.
    pub fn locks(&self, own: &Store) -> Result<Vec<files::Lock>> {
        let mut roots = vec![own.root.clone()];
        if let Some(peer) = &self.peer {
            roots.push(peer.root.clone());
        }
        roots.sort();
        roots.dedup();
        roots
            .iter()
            .map(|root| files::Lock::acquire(root, files::MUTATION_WAIT))
            .collect()
    }

    pub fn revalidate(&self, own: &Store, repo: &Path) -> Result<()> {
        if self.witness["outcome"] == "skipped"
            && snapshot(own)["qualification"]["outcome"] == "passed"
        {
            return Err(Refusal::new("health-feedback-policy-missing",
                "feedback became qualified during inspection; restore its contextual policy before promotion"));
        }
        if self
            .peer
            .as_ref()
            .is_some_and(|peer| snapshot(peer) != self.peer_snapshot)
        {
            return Err(Refusal::new(
                "feedback-peer-moved",
                "installed counterpart changed during qualification; qualify the new pair",
            ));
        }
        if policies(repo)? != self.policy {
            return Err(Refusal::new(
                "feedback-policy-moved",
                "contextual policy changed during qualification; qualify again",
            ));
        }
        Ok(())
    }
}

impl Store {
    pub(super) fn feedback_health(&self, cid: &str, repo: &Path) -> Result<Option<FeedbackHealth>> {
        if !matches!(self.tool.as_str(), "epr" | "brit") {
            return Ok(None);
        }
        if !repo.join(".epr-meta/feedback-policy.json").is_file() {
            if snapshot(self)["qualification"]["outcome"] == "passed" {
                return Err(Refusal::new("health-feedback-policy-missing",
                    "previously qualified feedback requires its contextual policy; restore .epr-meta/feedback-policy.json"));
            }
            return Ok(Some(FeedbackHealth {
                witness: json!({"checkId":"private-feedback-loop", "outcome":"skipped",
                    "summary":"Contextual feedback policy is absent; install .epr-meta/feedback-policy.json before qualifying feedback.",
                    "candidateCid":cid,"observed":{"mode":"unmeasured","pairQualified":false}}),
                peer: None,
                peer_snapshot: Value::Null,
                policy: policies(repo)?,
            }));
        }
        let other = if self.tool == "epr" { "brit" } else { "epr" };
        let peer = Store::from_environment(other, &self.registry)?;
        self.feedback_health_with(cid, repo, peer).map(Some)
    }

    fn feedback_health_with(&self, cid: &str, repo: &Path, peer: Store) -> Result<FeedbackHealth> {
        let peer_snapshot = snapshot(&peer);
        let policy = policies(repo)?;
        // A pre-contract Brit has no pair witness. EPR can arrive first with recorder
        // qualification, explicitly leaving pair readiness unestablished. Once Brit
        // qualifies, every later promotion of either tool must prove the whole pair.
        let pair = self.tool == "brit"
            || (peer_snapshot["qualification"]["outcome"] == "passed"
                && peer_snapshot["qualification"]["observed"]["mode"] == "pair"
                && peer_snapshot["qualification"]["observed"]["pairQualified"] == true);
        let candidate = self.artifact(cid)?.join(&self.tool);
        let (recorder, producer) = if self.tool == "brit" {
            (peer.current_artifact()?.1, Some(candidate))
        } else {
            (
                candidate,
                if pair {
                    Some(peer.current_artifact()?.1)
                } else {
                    None
                },
            )
        };
        let mut args = vec!["feedback".to_owned(), "qualify".to_owned()];
        if let Some(brit) = producer {
            args.extend(["--brit".into(), brit.to_string_lossy().into_owned()]);
        } else {
            args.push("--recorder-only".into());
        }
        args.extend([
            "--root".into(),
            repo.to_string_lossy().into_owned(),
            "--json".into(),
        ]);
        let mut spec = CommandSpec::new(recorder.to_string_lossy(), args);
        spec.cwd = Some(repo.into());
        spec.timeout = Duration::from_secs(120);
        let result = self
            .run(&spec)
            .map_err(|e| Refusal::new("health-feedback-failed", e.detail))?;
        let mut witness: Value = serde_json::from_str(&result.stdout)
            .map_err(|_| Refusal::new("health-feedback-failed", "recorder did not return a qualification witness; install a compatible EPR recorder"))?;
        let mode = if pair { "pair" } else { "recorder-only" };
        if !result.success
            || witness["checkId"] != "private-feedback-loop"
            || witness["outcome"] != "passed"
            || witness["observed"]["mode"] != mode
            || witness["observed"]["pairQualified"] != pair
        {
            return Err(Refusal::new(
                "health-feedback-failed",
                format!(
                    "{mode} qualification did not pass: {}",
                    witness["summary"].as_str().unwrap_or("invalid witness")
                ),
            ));
        }
        witness["candidateCid"] = json!(cid);
        witness["counterpartCid"] = peer_snapshot["cid"].clone();
        witness["policyInputs"] = json!(policy);
        Ok(FeedbackHealth {
            witness,
            peer: Some(peer),
            peer_snapshot,
            policy,
        })
    }
}

#[cfg(test)]
mod tests;
