use super::*;
use crate::bin_store::{CommandOutput, IngestOptions, Registry, Runner};
use std::{
    fs,
    sync::{Arc, Mutex},
};

#[derive(Default)]
struct Capture {
    calls: Mutex<Vec<CommandSpec>>,
    outcome: Mutex<Option<&'static str>>,
}
impl Runner for Capture {
    fn run(&self, spec: &CommandSpec) -> Result<CommandOutput> {
        self.calls.lock().unwrap().push(spec.clone());
        let stdout = if spec.args == ["--version"] {
            json!({"cid":files::file_cid(Path::new(&spec.program))?,"version":"fixture"})
                .to_string()
        } else {
            let pair = spec.args.contains(&"--brit".into());
            json!({"checkId":"private-feedback-loop", "outcome":self.outcome.lock().unwrap().unwrap_or("passed"),
                "summary":"fixture", "observed":{"mode":if pair {"pair"}else{"recorder-only"},"pairQualified":pair}}).to_string()
        };
        Ok(CommandOutput {
            success: true,
            stdout,
            stderr: String::new(),
        })
    }
}

struct Fixture {
    tmp: tempfile::TempDir,
    epr: Store,
    brit: Store,
    epr_cid: String,
    brit_cid: String,
    runner: Arc<Capture>,
}
impl Fixture {
    fn new() -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let registry: Registry = serde_json::from_value(json!({"version":1,"tools":{
            "epr":{"bins":["epr"]},"brit":{"bins":["brit"]}}}))
        .unwrap();
        let runner = Arc::new(Capture::default());
        let epr = Store::new(tmp.path().join("epr"), "epr", &registry)
            .unwrap()
            .with_runner(runner.clone());
        let brit = Store::new(tmp.path().join("brit"), "brit", &registry)
            .unwrap()
            .with_runner(runner.clone());
        let mut cids = Vec::new();
        for store in [&epr, &brit] {
            let input = tmp.path().join(format!("{}-input", store.tool));
            files::atomic_bytes(
                &input,
                format!("#!/bin/sh\n# {}\nexit 0\n", store.tool).as_bytes(),
                0o755,
            )
            .unwrap();
            let release = store
                .ingest(
                    &input,
                    &IngestOptions {
                        source_commit: Some("fixture".into()),
                        tree_clean: Some(true),
                        ..Default::default()
                    },
                )
                .unwrap();
            files::atomic_json(
                &store.root.join("current.json"),
                &json!({"cid":release.cid,"commit":"fixture"}),
            )
            .unwrap();
            cids.push(release.cid);
        }
        fs::create_dir_all(tmp.path().join(".epr-meta")).unwrap();
        fs::write(tmp.path().join(".epr-meta/feedback-policy.json"), "{}").unwrap();
        Self {
            tmp,
            epr,
            brit,
            epr_cid: cids[0].clone(),
            brit_cid: cids[1].clone(),
            runner,
        }
    }
}

#[test]
fn absent_policy_is_explicit_and_cannot_remove_an_established_obligation() {
    let f = Fixture::new();
    fs::remove_file(f.tmp.path().join(".epr-meta/feedback-policy.json")).unwrap();
    let health = f
        .epr
        .feedback_health(&f.epr_cid, f.tmp.path())
        .unwrap()
        .unwrap();
    assert_eq!(health.witness["outcome"], "skipped");
    assert_eq!(health.witness["observed"]["pairQualified"], false);
    assert!(health.witness["summary"]
        .as_str()
        .unwrap()
        .contains("policy is absent"));
    let mut record = f.epr.record(&f.epr_cid).unwrap();
    record
        .extra
        .insert("feedbackQualification".into(), json!({"outcome":"passed"}));
    f.epr.save(&record).unwrap();
    assert_eq!(
        health.revalidate(&f.epr, f.tmp.path()).unwrap_err().kind,
        "health-feedback-policy-missing"
    );
    assert_eq!(
        f.epr
            .feedback_health(&f.epr_cid, f.tmp.path())
            .err()
            .unwrap()
            .kind,
        "health-feedback-policy-missing"
    );
}

#[test]
fn first_recorder_arrives_without_claiming_pair_readiness() {
    let f = Fixture::new();
    let health = f
        .epr
        .feedback_health_with(&f.epr_cid, f.tmp.path(), f.brit.clone())
        .unwrap();
    assert_eq!(health.witness["observed"]["mode"], "recorder-only");
    assert_eq!(health.witness["observed"]["pairQualified"], false);
    assert_eq!(health.witness["counterpartCid"], f.brit_cid);
    let _locks = health.locks(&f.epr).unwrap();
    health.revalidate(&f.epr, f.tmp.path()).unwrap();
}

#[test]
fn producer_requires_pair_and_next_recorder_requalifies_that_producer() {
    let f = Fixture::new();
    let health = f
        .brit
        .feedback_health_with(&f.brit_cid, f.tmp.path(), f.epr.clone())
        .unwrap();
    assert_eq!(health.witness["observed"]["mode"], "pair");
    let mut record = f.brit.record(&f.brit_cid).unwrap();
    record
        .extra
        .insert("feedbackQualification".into(), health.witness);
    f.brit.save(&record).unwrap();
    let health = f
        .epr
        .feedback_health_with(&f.epr_cid, f.tmp.path(), f.brit.clone())
        .unwrap();
    assert_eq!(health.witness["observed"]["pairQualified"], true);
    let calls = f.runner.calls.lock().unwrap();
    let command = calls.last().unwrap();
    assert_eq!(
        command.program,
        f.epr
            .artifact(&f.epr_cid)
            .unwrap()
            .join("epr")
            .to_string_lossy()
    );
    assert!(command.args.contains(
        &f.brit
            .artifact(&f.brit_cid)
            .unwrap()
            .join("brit")
            .to_string_lossy()
            .into_owned()
    ));
}

#[test]
fn skipped_or_failed_witness_cannot_qualify_even_with_zero_command_exit() {
    let f = Fixture::new();
    for outcome in ["skipped", "failed"] {
        *f.runner.outcome.lock().unwrap() = Some(outcome);
        let result = f
            .brit
            .feedback_health_with(&f.brit_cid, f.tmp.path(), f.epr.clone());
        assert_eq!(result.err().unwrap().kind, "health-feedback-failed");
    }
}

#[test]
fn policy_and_counterpart_changes_invalidate_the_prepared_witness() {
    let f = Fixture::new();
    let health = f
        .brit
        .feedback_health_with(&f.brit_cid, f.tmp.path(), f.epr.clone())
        .unwrap();
    fs::write(
        f.tmp.path().join(".epr-meta/feedback-policy.json"),
        "{\"changed\":true}",
    )
    .unwrap();
    assert_eq!(
        health.revalidate(&f.brit, f.tmp.path()).unwrap_err().kind,
        "feedback-policy-moved"
    );
    fs::write(f.tmp.path().join(".epr-meta/feedback-policy.json"), "{}").unwrap();
    files::atomic_json(
        &f.epr.root.join("current.json"),
        &json!({"cid":f.brit_cid,"commit":"other"}),
    )
    .unwrap();
    assert_eq!(
        health.revalidate(&f.brit, f.tmp.path()).unwrap_err().kind,
        "feedback-peer-moved"
    );
}
