use serde_json::{json, Value};
use std::io::Write;
use std::process::{Command, Stdio};

fn fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("fixture");
    std::fs::create_dir(dir.path().join(".epr-meta")).expect("policy directory");
    std::fs::write(dir.path().join(".epr-meta/feedback-policy.json"),json!({
        "id":"fixture@1","authority":"human:fixture","participant":"human:fixture","producers":["human","genesis","brit","brit-guard"],
        "automaticCapture":true,"deliberateEvidence":true,"maxBytes":1000000,"maxRecordBytes":65536,
        "response":"repair","sharing":"separately-authorized"
    }).to_string()).expect("policy");
    dir
}

fn run(root: &std::path::Path, args: &[&str], input: Option<Value>) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_epr"))
        .arg("feedback")
        .args(args)
        .arg("--root")
        .arg(root)
        .arg("--json")
        .current_dir(root)
        .env("EPR_SESSION_LAZY", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("epr");
    if let Some(input) = input {
        child
            .stdin
            .take()
            .expect("stdin")
            .write_all(input.to_string().as_bytes())
            .expect("write");
    }
    child.wait_with_output().expect("result")
}
fn value(output: std::process::Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("JSON")
}

#[test]
fn feedback_cli_is_offline_private_and_preserves_source_counts() {
    let dir = fixture();
    let mut report = json!({"runId":"run-1","generatedAt":"2026-11-08T15:00:00Z","env":{"sut":"sut-1","secret":"ENV_SECRET"},
        "findings":[{"fingerprint":"failure-1","source":"console-error","severity":"error","occurrences":9,
            "message":"SECRET_OUTPUT","firstSeenUrl":"https://private.invalid","screenshotPath":"/private/screenshot",
            "suggestedObjective":"PRIVATE_OBJECTIVE"}],
        "summary":{"byConcern":{"retained":{"passed":1,"failed":0,"pending":1,"skipped":1}}},
        "declared":{"concerns":["retained","absent"],"notMeasured":["absent"]}});
    let first = value(run(dir.path(), &["ingest-genesis"], Some(report.clone())));
    report["generatedAt"] = json!("2026-11-09T15:00:00Z");
    let repeated = value(run(dir.path(), &["ingest-genesis"], Some(report)));
    assert_eq!(first["records"], repeated["records"]);
    let list = value(run(dir.path(), &["list"], None));
    let experiences = list["experiences"].as_array().expect("experiences");
    assert_eq!(experiences.len(), 4);
    assert!(experiences[0]["cid"]
        .as_str()
        .expect("copyable address")
        .parse::<cid::Cid>()
        .is_ok());
    assert_eq!(experiences[0]["occurrence"]["count"], 9);
    assert!(experiences
        .iter()
        .any(|v| v["occurrence"]["outcome"] == "skipped"));
    assert!(experiences
        .iter()
        .any(|v| v["occurrence"]["outcome"] == "unmeasured"));
    let retained =
        std::fs::read_to_string(dir.path().join(".eprfs/status/feedback.jsonl")).expect("evidence");
    for private in [
        "SECRET_OUTPUT",
        "private.invalid",
        "/private/screenshot",
        "ENV_SECRET",
        "PRIVATE_OBJECTIVE",
    ] {
        assert!(!retained.contains(private));
    }
    assert!(
        !dir.path().join(".eprfs/status/flows.jsonl").exists(),
        "ingestion never fulfils or proposes"
    );
    assert!(!dir.path().join(".git").exists());
}

#[test]
fn feedback_cli_contextual_policy_refuses_missing_and_filters_capture() {
    let empty = tempfile::tempdir().expect("fixture");
    assert!(!run(
        empty.path(),
        &[
            "capture",
            "--producer",
            "brit",
            "--operation",
            "status",
            "--outcome",
            "failed"
        ],
        None
    )
    .status
    .success());
    assert!(!empty.path().join(".eprfs").exists());
    let dir = fixture();
    assert!(!run(
        dir.path(),
        &[
            "capture",
            "--producer",
            "brit",
            "--operation",
            "https://secret.invalid",
            "--outcome",
            "failed"
        ],
        None
    )
    .status
    .success());
    value(run(
        dir.path(),
        &[
            "capture",
            "--producer",
            "brit",
            "--operation",
            "bridge",
            "--outcome",
            "bridge-declared",
            "--invocation",
            "one",
        ],
        None,
    ));
    value(run(
        dir.path(),
        &[
            "capture",
            "--producer",
            "brit-guard",
            "--operation",
            "guard",
            "--outcome",
            "fallback-declared",
            "--invocation",
            "one",
        ],
        None,
    ));
    value(run(
        dir.path(),
        &[
            "capture",
            "--producer",
            "brit-guard",
            "--operation",
            "guard",
            "--outcome",
            "unsupported",
            "--invocation",
            "two",
        ],
        None,
    ));
    let list = value(run(dir.path(), &["list"], None));
    let rows = list["experiences"].as_array().expect("experiences");
    assert_eq!(rows.len(), 3);
    assert!(rows.iter().all(|v| v["occurrence"]["outcome"] != "failed"));
    let triage = value(run(dir.path(), &["triage", "--response", "repair"], None));
    assert!(triage["rows"]
        .as_array()
        .expect("rows")
        .iter()
        .all(|r| r["admitted"] == false && !r["waiting"].as_array().expect("waiting").is_empty()));
}

#[test]
fn feedback_cli_human_proposal_is_not_admitted_repair() {
    let dir = fixture();
    let record = json!({"kind":"occurrence","source":{"producer":"human","run":"conversation-one","item":"need"},
        "observedAt":"2026-11-08T15:00:00.000Z","participant":"human:fixture","subject":"history","outcome":"reported",
        "count":1,"durationMs":null,"impact":null,"message":"I need a bounded history view","detail":null});
    let receipt = value(run(dir.path(), &["record"], Some(record)));
    let cid = receipt["cid"].as_str().expect("CID");
    let proposal = value(run(
        dir.path(),
        &[
            "propose",
            "--id",
            cid,
            "--as",
            "human:fixture",
            "--objective",
            "Bound history",
        ],
        None,
    ));
    assert_eq!(proposal["admitted"], false);
    let repeated = value(run(
        dir.path(),
        &[
            "propose",
            "--id",
            cid,
            "--as",
            "human:fixture",
            "--objective",
            "Bound history",
        ],
        None,
    ));
    assert_eq!(proposal, repeated);
    assert!(!run(
        dir.path(),
        &[
            "propose",
            "--id",
            cid,
            "--as",
            "human:fixture",
            "--objective",
            "Duplicate repair"
        ],
        None
    )
    .status
    .success());
    assert_eq!(
        std::fs::read_to_string(dir.path().join(".eprfs/status/flows.jsonl"))
            .expect("flows")
            .lines()
            .count(),
        1
    );
}

#[test]
fn feedback_cli_concurrent_processes_and_empty_report() {
    let dir = fixture();
    let mut children = vec![];
    for n in 0..8 {
        children.push(
            Command::new(env!("CARGO_BIN_EXE_epr"))
                .args([
                    "feedback",
                    "capture",
                    "--producer",
                    "brit",
                    "--operation",
                    "status",
                    "--outcome",
                    "passed",
                    "--invocation",
                ])
                .arg(format!("invocation-{n}"))
                .arg("--root")
                .arg(dir.path())
                .arg("--json")
                .stdout(Stdio::null())
                .spawn()
                .expect("writer"),
        );
    }
    for mut child in children {
        assert!(child.wait().expect("writer exit").success());
    }
    assert_eq!(
        value(run(dir.path(), &["list"], None))["experiences"]
            .as_array()
            .expect("experiences")
            .len(),
        8
    );
    let report = json!({"runId":"empty","generatedAt":"2026-11-08T15:00:00Z","findings":[],"summary":{"byConcern":{},"scenarios":{"total":0,"passed":0,"failed":0,"skipped":0,"pending":0}}});
    value(run(dir.path(), &["ingest-genesis"], Some(report)));
    let rows = value(run(dir.path(), &["list"], None));
    let last = rows["experiences"]
        .as_array()
        .expect("rows")
        .last()
        .expect("empty run retained");
    assert_eq!(last["occurrence"]["outcome"], "unmeasured");
    assert!(last["occurrence"]["detail"]["sut"].is_null());
}

#[test]
fn feedback_cli_repair_requires_existing_claim_and_local_delivery() {
    let dir = fixture();
    let cid = value(run(
        dir.path(),
        &[
            "capture",
            "--producer",
            "brit",
            "--operation",
            "status",
            "--outcome",
            "failed",
        ],
        None,
    ))["cid"]
        .as_str()
        .expect("cid")
        .to_owned();
    std::fs::write(
        dir.path().join("contract.json"),
        json!({"mode":"delivery"}).to_string(),
    )
    .expect("contract");
    let result = run(
        dir.path(),
        &[
            "repair",
            "--id",
            &cid,
            "--contract",
            "contract.json",
            "--runtime",
            "codex",
        ],
        None,
    );
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("local-delivery"));
    std::fs::write(
        dir.path().join("contract.json"),
        json!({"mode":"local-delivery","commitment":cid}).to_string(),
    )
    .expect("contract");
    let result = run(
        dir.path(),
        &[
            "repair",
            "--id",
            &cid,
            "--contract",
            "contract.json",
            "--runtime",
            "codex",
        ],
        None,
    );
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("no existing development commitments"));
    assert!(!dir.path().join(".claude/shifts").exists());
}

// gate-reads: .claude/epr-meta/policies.yaml
fn capacity_fixture(root: &std::path::Path) {
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    std::fs::create_dir_all(root.join(".claude/epr-meta")).unwrap();
    std::fs::copy(
        repo.join(".claude/epr-meta/policies.yaml"),
        root.join(".claude/epr-meta/policies.yaml"),
    )
    .unwrap();
    std::fs::write(root.join(".epr-meta/feedback-capacity.json"),json!({"policy":"feedback-local-retention@1",
        "params":{"totalBytes":240000,"activeBytes":80000,"replacementBytes":80000,"controlBytes":80000,"recordBytes":20000}}).to_string()).unwrap();
}

#[test]
fn feedback_cli_capacity_cutover_policy_pins_measure_and_producer_compatibility() {
    let dir = fixture();
    capacity_fixture(dir.path());
    let args = [
        "capture",
        "--producer",
        "brit",
        "--operation",
        "status",
        "--outcome",
        "passed",
        "--invocation",
        "run1",
    ];
    assert!(!run(dir.path(), &args, None).status.success());
    assert!(!run(dir.path(), &["migrate"], None).status.success());
    let migrated = value(run(
        dir.path(),
        &["migrate", "--legacy-writers-quiesced"],
        None,
    ));
    assert_eq!(migrated["retainedBytes"], 0);
    let first = value(run(dir.path(), &args, None));
    assert_eq!(first, value(run(dir.path(), &args, None)));
    let status = value(run(dir.path(), &["capacity"], None));
    assert_eq!(status["policy"]["policy"], "feedback-local-retention@1");
    assert!(
        status["totalManagedFileBytes"].as_u64().unwrap()
            > status["retainedBytes"].as_u64().unwrap()
    );
    let measure = value(run(
        dir.path(),
        &["measure", "--measure", "feedback-retained-bytes@1"],
        None,
    ));
    assert_eq!(measure["value"], status["retainedBytes"]);
    let before = std::fs::read(dir.path().join(".eprfs/status/feedback/state.json")).unwrap();
    value(run(dir.path(), &["reclaim", "--dry-run"], None));
    assert_eq!(
        before,
        std::fs::read(dir.path().join(".eprfs/status/feedback/state.json")).unwrap()
    );
    let registry = dir.path().join(".claude/epr-meta/policies.yaml");
    let original = std::fs::read_to_string(&registry).unwrap();
    std::fs::write(
        &registry,
        original.replacen("routineDays: 7", "routineDays: 1", 1),
    )
    .unwrap();
    assert!(
        !run(dir.path(), &["reclaim"], None).status.success(),
        "changed unpinned policy refuses"
    );
    assert_eq!(
        before,
        std::fs::read(dir.path().join(".eprfs/status/feedback/state.json")).unwrap()
    );
}
