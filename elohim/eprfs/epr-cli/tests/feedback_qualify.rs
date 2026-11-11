use elohim_epr_rea::feedback_capacity::Capacity;
use serde_json::{json, Value};
use std::{
    os::unix::fs::PermissionsExt,
    path::Path,
    process::{Command, Output},
};

fn fixture() -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("fixture");
    std::fs::create_dir(root.path().join(".epr-meta")).expect("policy directory");
    let parameters = json!({"totalBytes":180000,"activeBytes":50000,"replacementBytes":50000,
        "controlBytes":80000,"recordBytes":12000,"triggerPercent":85,"resetPercent":70,
        "routineDays":7,"resolvedDays":30,"recentReceipts":2,"dwellSeconds":0});
    let precedent = json!({"id":"fixture-retention","version":1,"binding":"binding-local",
        "permits":["reclaim-eligible-feedback"],"parameter-bounds":{"routineDays":{"min":7},
        "resolvedDays":{"min":30},"controlBytes":{"min":65536},"recentReceipts":{"min":1,"max":32}},
        "established_by":"operator-fixture","why":"bounded private evidence","supersedes":null,
        "scope":{"path":".eprfs/status/feedback"},"measure":parameters,"permitted-local-parameters":[]});
    let capacity =
        Capacity::resolve("fixture-retention@1".into(), precedent, json!({})).expect("capacity");
    std::fs::write(root.path().join(".epr-meta/feedback-policy.json"), json!({
        "id":"fixture@1","authority":"human:fixture","participant":null,
        "producers":["human","genesis","brit","brit-guard"],"automaticCapture":true,
        "deliberateEvidence":true,"response":"repair","sharing":"separately-authorized","capacity":capacity
    }).to_string()).expect("policy");
    root
}

fn run(root: &Path, args: &[&str]) -> (Output, Value) {
    let output = Command::new(env!("CARGO_BIN_EXE_epr"))
        .args(["feedback", "qualify"])
        .args(args)
        .arg("--root")
        .arg(root)
        .arg("--json")
        .env("EPR_BIN", "/wrong-epr")
        .env("BRIT_BIN", "/wrong-brit")
        .env("EPR_PIN_CID", "wrong-pin")
        .env("BRIT_PIN_CID", "wrong-pin")
        .output()
        .expect("qualification");
    let value = serde_json::from_slice(&output.stdout).expect("structured witness");
    (output, value)
}

fn producer(root: &Path) -> std::path::PathBuf {
    let script = root.join("fixture-brit");
    std::fs::write(&script, r#"#!/bin/sh
case "$1" in --version) code=0; outcome=passed;; *) code=2; outcome=refused;; esac
epr feedback capture --producer brit --operation log --outcome "$outcome" --invocation "$EPR_FEEDBACK_INVOCATION" --root "$EPR_FEEDBACK_ROOT" --json >/dev/null 2>&1 || echo 'feedback capture gap' >&2
exit "$code"
"#).expect("producer");
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o700)).expect("executable");
    script
}

#[test]
fn recorder_only_is_explicit_and_leaves_the_real_store_untouched() {
    let root = fixture();
    std::fs::create_dir_all(root.path().join(".eprfs/status")).expect("private source");
    let source = root.path().join(".eprfs/status/feedback.jsonl");
    std::fs::write(&source, "untouched-private-evidence\n").expect("private source");
    let (output, witness) = run(root.path(), &["--recorder-only"]);
    assert!(output.status.success(), "{}", witness);
    assert_eq!(witness["checkId"], "private-feedback-loop");
    assert_eq!(witness["outcome"], "passed");
    assert_eq!(witness["observed"]["mode"], "recorder-only");
    assert_eq!(witness["observed"]["pairQualified"], false);
    assert_eq!(witness["observed"]["admitted"], false);
    assert_eq!(
        std::fs::read_to_string(source).expect("preserved"),
        "untouched-private-evidence\n"
    );
    assert!(!root.path().join(".eprfs/status/flows.jsonl").exists());
}

#[test]
fn pair_qualification_exercises_capture_proposal_and_recorder_failure() {
    let root = fixture();
    let brit = producer(root.path());
    let (output, witness) = run(root.path(), &["--brit", brit.to_str().expect("path")]);
    assert!(output.status.success(), "{witness}");
    assert_eq!(witness["observed"]["mode"], "pair");
    assert_eq!(witness["observed"]["pairQualified"], true);
    assert_eq!(witness["observed"]["recorderFailurePreservesExit"], true);
    assert_eq!(
        witness["observed"]["occurrences"]
            .as_array()
            .expect("occurrences")
            .len(),
        2
    );
    assert!(!root.path().join(".eprfs").exists());
}

#[test]
fn unavailable_prerequisites_are_unmeasured_and_never_green() {
    let empty = tempfile::tempdir().expect("fixture");
    let (output, witness) = run(empty.path(), &["--recorder-only"]);
    assert!(!output.status.success());
    assert_eq!(witness["outcome"], "skipped");
    assert_eq!(witness["observed"]["pairQualified"], false);
    let root = fixture();
    let (output, witness) = run(root.path(), &["--brit", "/absent-brit"]);
    assert!(!output.status.success());
    assert_eq!(witness["outcome"], "skipped");
}

#[test]
fn incompatible_producer_or_invalid_mode_cannot_qualify() {
    let root = fixture();
    for args in [
        vec!["--brit", "/bin/true"],
        vec!["--recorder-only", "--brit", "/bin/true"],
        vec![],
    ] {
        let (output, witness) = run(root.path(), &args);
        assert!(!output.status.success());
        assert_eq!(witness["outcome"], "failed");
        assert_eq!(witness["observed"]["pairQualified"], false);
    }
}

#[test]
fn disabled_capture_is_a_failed_contract_not_a_qualified_recorder() {
    let root = fixture();
    let path = root.path().join(".epr-meta/feedback-policy.json");
    let mut policy: Value =
        serde_json::from_slice(&std::fs::read(&path).expect("policy")).expect("json");
    policy["automaticCapture"] = json!(false);
    std::fs::write(path, policy.to_string()).expect("policy");
    let (output, witness) = run(root.path(), &["--recorder-only"]);
    assert!(!output.status.success());
    assert_eq!(witness["outcome"], "failed");
}

#[test]
fn another_producers_records_do_not_qualify_brit() {
    let root = fixture();
    let brit = producer(root.path());
    let script = std::fs::read_to_string(&brit).expect("producer");
    std::fs::write(
        &brit,
        script.replace("--producer brit ", "--producer brit-guard "),
    )
    .expect("different producer");
    let (output, witness) = run(root.path(), &["--brit", brit.to_str().expect("path")]);
    assert!(!output.status.success());
    assert_eq!(witness["outcome"], "failed");
    assert_eq!(witness["observed"]["pairQualified"], false);
}

#[test]
fn bounded_command_deadline_includes_descendant_pipe_drain() {
    let mut command = Command::new("sh");
    command.args(["-c", "sleep 30 & exit 0"]);
    let start = std::time::Instant::now();
    let output = elohim_epr_cli::flow::memory::recall::bounded_command(command, 64, 0.1, None)
        .expect("bounded child");
    assert!(output.error.is_some());
    assert!(start.elapsed().as_secs_f64() < 2.0);
}
