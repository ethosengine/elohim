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

// ---- FB-1: capture under a declared claim, and the capture-gap spool drain ----

const CLAIM: &str = "ferry/capability-absent:list.since";
/// The session and invocation variables capture reads; each claim-path case sets its own.
const SESSION_ENV: [&str; 9] = [
    "EPR_FEEDBACK_CLAIM",
    "EPR_FEEDBACK_EMITTER",
    "EPR_FEEDBACK_INVOCATION",
    "ELOHIM_SESSION_ID",
    "CLAUDE_SESSION_ID",
    "CLAUDE_CODE_SESSION_ID",
    "CODEX_THREAD_ID",
    "CODEX_SESSION_ID",
    "XDG_STATE_HOME",
];

fn claim_fixture() -> tempfile::TempDir {
    let dir = fixture();
    std::fs::write(dir.path().join(".epr-meta/feedback-policy.json"),json!({
        "id":"fixture@1","authority":"human:fixture","participant":"human:fixture",
        "producers":["human","genesis","brit","brit-guard","ferry"],
        "automaticCapture":true,"deliberateEvidence":true,"maxBytes":1000000,"maxRecordBytes":65536,
        "response":"repair","sharing":"separately-authorized"
    }).to_string()).expect("policy");
    dir
}

fn run_env(root: &std::path::Path, args: &[&str], env: &[(&str, &str)]) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_epr"));
    command
        .arg("feedback")
        .args(args)
        .arg("--root")
        .arg(root)
        .arg("--json")
        .current_dir(root)
        .env("EPR_SESSION_LAZY", "1");
    for name in SESSION_ENV {
        command.env_remove(name);
    }
    for (name, value) in env {
        command.env(name, value);
    }
    command.output().expect("epr")
}

fn claim_capture(
    root: &std::path::Path,
    outcome: &str,
    env: &[(&str, &str)],
) -> std::process::Output {
    let mut all = vec![("EPR_FEEDBACK_CLAIM", CLAIM)];
    all.extend_from_slice(env);
    run_env(
        root,
        &[
            "capture",
            "--producer",
            "ferry",
            "--operation",
            "list-since",
            "--outcome",
            outcome,
        ],
        &all,
    )
}

fn occurrences(root: &std::path::Path) -> Vec<Value> {
    value(run_env(root, &["list"], &[]))["experiences"]
        .as_array()
        .expect("experiences")
        .iter()
        .map(|row| row["occurrence"].clone())
        .collect()
}

#[test]
fn claim_capture_stores_claim_item_session_run_emitter_and_no_duration() {
    let dir = claim_fixture();
    value(run_env(
        dir.path(),
        &[
            "capture",
            "--producer",
            "ferry",
            "--operation",
            "list-since",
            "--outcome",
            "refused",
            "--elapsed-ms",
            "40",
            "--invocation",
            "ignored",
        ],
        &[
            ("EPR_FEEDBACK_CLAIM", CLAIM),
            ("EPR_FEEDBACK_EMITTER", "unrecorded:1+exe.000000000001"),
            ("ELOHIM_SESSION_ID", "fixture-session"),
        ],
    ));
    // The legacy path in the same store is unchanged.
    value(run_env(
        dir.path(),
        &[
            "capture",
            "--producer",
            "brit",
            "--operation",
            "status",
            "--outcome",
            "failed",
            "--invocation",
            "legacy-1",
            "--elapsed-ms",
            "7",
        ],
        &[("ELOHIM_SESSION_ID", "fixture-session")],
    ));
    let rows = occurrences(dir.path());
    assert_eq!(rows.len(), 2);
    let claim = rows
        .iter()
        .find(|o| o["subject"] == CLAIM)
        .expect("claim record");
    assert_eq!(
        claim["source"],
        json!({"producer":"ferry",
            "run":"session-d6440983c454c2e5999fdb66bbe9cf5f89a8f5847d6c9578ef14afd26eee122c",
            "item":"ferry/capability-absent:list.since refused unrecorded:1+exe.000000000001"})
    );
    assert_eq!(claim["outcome"], "refused");
    assert_eq!(claim["participant"], "human:fixture");
    assert_eq!(
        claim["detail"],
        json!({"emitter":"unrecorded:1+exe.000000000001"})
    );
    assert!(claim["durationMs"].is_null());
    assert!(claim["message"].is_null());
    let legacy = rows
        .iter()
        .find(|o| o["subject"] == "brit:status")
        .expect("legacy record");
    assert_eq!(
        legacy["source"],
        json!({"producer":"brit","run":"legacy-1","item":"status"})
    );
    assert_eq!(legacy["durationMs"], 7);
    assert!(legacy["detail"].is_null());
}

#[test]
fn claim_capture_keeps_one_record_per_session_and_outcome() {
    let dir = claim_fixture();
    let session = [("ELOHIM_SESSION_ID", "fixture-session")];
    let first = value(claim_capture(dir.path(), "refused", &session));
    for _ in 1..20 {
        assert_eq!(first, value(claim_capture(dir.path(), "refused", &session)));
    }
    assert_eq!(occurrences(dir.path()).len(), 1);
    value(claim_capture(dir.path(), "failed", &session));
    assert_eq!(occurrences(dir.path()).len(), 2);
    value(claim_capture(
        dir.path(),
        "refused",
        &[("ELOHIM_SESSION_ID", "fixture-session-2")],
    ));
    let rows = occurrences(dir.path());
    assert_eq!(rows.len(), 3);
    assert!(rows.iter().all(|o| o["count"] == 1));
    // An invalid emitter is dropped, not refused, and still matches its session's record.
    value(claim_capture(
        dir.path(),
        "refused",
        &[
            ("ELOHIM_SESSION_ID", "fixture-session"),
            ("EPR_FEEDBACK_EMITTER", "ferry 1"),
        ],
    ));
    assert_eq!(occurrences(dir.path()).len(), 3);
}

#[test]
fn claim_capture_accepts_any_operation_word_that_legacy_capture_refuses() {
    let dir = claim_fixture();
    assert!(!run_env(
        dir.path(),
        &[
            "capture",
            "--producer",
            "brit",
            "--operation",
            "list-since",
            "--outcome",
            "failed"
        ],
        &[],
    )
    .status
    .success());
    assert!(occurrences(dir.path()).is_empty());
    value(claim_capture(
        dir.path(),
        "refused",
        &[("ELOHIM_SESSION_ID", "s")],
    ));
    assert_eq!(occurrences(dir.path()).len(), 1);
    // A producer outside the policy's list is still refused by the store.
    let output = run_env(
        dir.path(),
        &[
            "capture",
            "--producer",
            "ferry-2",
            "--operation",
            "x",
            "--outcome",
            "refused",
        ],
        &[("EPR_FEEDBACK_CLAIM", CLAIM)],
    );
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("invalid occurrence or producer outside policy"));
}

#[test]
fn malformed_claim_ids_are_refused_and_store_nothing() {
    let dir = claim_fixture();
    let long = format!("ferry/{}", "n".repeat(123));
    assert_eq!(long.len(), 129);
    for claim in [
        "Ferry/capability-absent",
        "ferry/capability/absent",
        "ferry/list since",
        long.as_str(),
    ] {
        let output = run_env(
            dir.path(),
            &[
                "capture",
                "--producer",
                "ferry",
                "--operation",
                "list",
                "--outcome",
                "refused",
            ],
            &[("EPR_FEEDBACK_CLAIM", claim), ("ELOHIM_SESSION_ID", "s")],
        );
        assert!(!output.status.success(), "{claim}");
        assert!(String::from_utf8_lossy(&output.stderr).contains("EPR_FEEDBACK_CLAIM"));
    }
    assert!(!dir.path().join(".eprfs/status/feedback.jsonl").exists());
}

#[test]
fn claim_session_key_matches_brits_three_forms() {
    let dir = claim_fixture();
    let run_of = |env: &[(&str, &str)], outcome: &str| {
        value(claim_capture(dir.path(), outcome, env));
        occurrences(dir.path())
            .into_iter()
            .find(|o| o["outcome"] == outcome)
            .expect("record")["source"]["run"]
            .as_str()
            .unwrap()
            .to_owned()
    };
    assert_eq!(
        run_of(
            &[("CLAUDE_CODE_SESSION_ID", " fixture-session ")],
            "refused"
        ),
        "session-d6440983c454c2e5999fdb66bbe9cf5f89a8f5847d6c9578ef14afd26eee122c"
    );
    assert_eq!(
        run_of(
            &[
                ("ELOHIM_SESSION_ID", "fixture-session"),
                ("EPR_FEEDBACK_INVOCATION", "probe-1")
            ],
            "failed"
        ),
        "invocation-dab5a6d218b53c9e03bf36bfe5f0105f761b5c1bc7d0b8695ae523898703ccaa"
    );
    assert_eq!(
        run_of(&[("EPR_FEEDBACK_INVOCATION", "probe-1")], "skipped"),
        "invocation-9c3fa6d6cb7d09cb9668dbcbebcc555ff711eac65f2ca4d2fb096e436acb5a1b"
    );
    let before = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        / 86_400;
    let day = run_of(&[], "unsupported");
    let after = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        / 86_400;
    assert!(
        day == format!("day-{before}") || day == format!("day-{after}"),
        "{day}"
    );
}

fn spool_file(at: &str, session: &str, pad: usize) -> String {
    json!({"v":1,"claim":"epr/recorder-unavailable:timeout","cause":"timeout","at":at,
        "emitter":"unrecorded:0.3.6+exe.0123456789ab","session":session,
        "recorder":{"path":"/secret/recorder","size":12,"mtime":null},
        "stderr":format!("SECRET_STDERR{}", "x".repeat(pad))})
    .to_string()
}

#[test]
fn drain_spool_records_each_valid_gap_once_and_keeps_the_invalid() {
    let dir = claim_fixture();
    let spool = dir.path().join("spool");
    std::fs::create_dir(&spool).unwrap();
    let session = "a".repeat(64);
    let valid = spool_file("2026-11-27T10:00:00Z", &session, 0);
    std::fs::write(spool.join("00000000000000000000000000000001.json"), &valid).unwrap();
    let invalid = [
        (
            "oversized.json",
            spool_file("2026-11-27T10:00:00Z", &session, 1024),
        ),
        ("malformed.json", "{\"v\":1,".to_owned()),
        ("bad-time.json", spool_file("27/11/2026", &session, 0)),
        (
            "bad-session.json",
            spool_file("2026-11-27T10:00:00Z", &"A".repeat(64), 0),
        ),
    ];
    for (name, body) in &invalid {
        std::fs::write(spool.join(name), body).unwrap();
    }
    std::fs::write(spool.join("note.txt"), "untouched").unwrap();
    let spool_arg = spool.to_str().unwrap();
    let first = value(run_env(
        dir.path(),
        &["drain-spool", "--dir", spool_arg],
        &[],
    ));
    assert_eq!(first["drained"], 1);
    assert_eq!(first["skipped"], 4);
    assert_eq!(first["other"], 1);
    assert_eq!(first["unremoved"], json!([]));
    assert_eq!(first["dir"], spool_arg);
    assert!(!spool.join("00000000000000000000000000000001.json").exists());
    for (name, _) in &invalid {
        assert!(spool.join(name).exists(), "{name} is kept");
    }
    let rows = occurrences(dir.path());
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0],
        json!({"source":{"producer":"brit","run":format!("spool-{session}"),
                "item":"epr/recorder-unavailable:timeout failed unrecorded:0.3.6+exe.0123456789ab"},
            "observedAt":"2026-11-27T10:00:00Z","participant":null,
            "subject":"epr/recorder-unavailable:timeout","outcome":"failed","count":1,
            "durationMs":null,"impact":null,"message":null,"detail":{"emitter":"unrecorded:0.3.6+exe.0123456789ab"}})
    );
    // The same gap noted again (another file, same session and cause) is already held.
    std::fs::write(spool.join("00000000000000000000000000000002.json"), &valid).unwrap();
    // The default spool is read from XDG_STATE_HOME when no --dir is given.
    let state = dir.path().join("state");
    std::fs::create_dir_all(state.join("brit")).unwrap();
    std::fs::rename(&spool, state.join("brit/feedback-spool")).unwrap();
    let second = value(run_env(
        dir.path(),
        &["drain-spool"],
        &[("XDG_STATE_HOME", state.to_str().unwrap())],
    ));
    assert_eq!(second["drained"], 1);
    assert_eq!(second["skipped"], 4);
    assert_eq!(second["other"], 1);
    assert_eq!(second["unremoved"], json!([]));
    assert!(!state
        .join("brit/feedback-spool/00000000000000000000000000000002.json")
        .exists());
    assert_eq!(occurrences(dir.path()).len(), 1);
    let retained =
        std::fs::read_to_string(dir.path().join(".eprfs/status/feedback.jsonl")).expect("store");
    assert!(!retained.contains("SECRET_STDERR"));
    assert!(!retained.contains("/secret/recorder"));
}

#[test]
fn drain_spool_requires_an_absolute_state_or_home_directory() {
    let dir = claim_fixture();
    let output = run_env(dir.path(), &["drain-spool"], &[("HOME", "relative/home")]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains(
        "no spool directory: set XDG_STATE_HOME or HOME to an absolute path, or pass --dir"
    ));
}

#[test]
fn records_written_by_the_claim_path_and_the_drain_read_with_the_existing_reader() {
    let dir = claim_fixture();
    value(claim_capture(
        dir.path(),
        "refused",
        &[
            ("ELOHIM_SESSION_ID", "s"),
            ("EPR_FEEDBACK_EMITTER", "unrecorded:1+exe.000000000001"),
        ],
    ));
    let spool = dir.path().join("spool");
    std::fs::create_dir(&spool).unwrap();
    std::fs::write(
        spool.join("g.json"),
        spool_file("2026-11-27T10:00:00Z", &"b".repeat(64), 0),
    )
    .unwrap();
    value(run_env(
        dir.path(),
        &["drain-spool", "--dir", spool.to_str().unwrap()],
        &[],
    ));
    let policy: elohim_epr_rea::feedback::Policy = serde_json::from_slice(
        &std::fs::read(dir.path().join(".epr-meta/feedback-policy.json")).unwrap(),
    )
    .unwrap();
    let entries = elohim_epr_rea::feedback_store::FeedbackStore::new(dir.path(), policy)
        .unwrap()
        .read()
        .unwrap();
    assert_eq!(entries.len(), 2);
    for line in std::fs::read_to_string(dir.path().join(".eprfs/status/feedback.jsonl"))
        .unwrap()
        .lines()
    {
        let entry: Value = serde_json::from_str(line).unwrap();
        // The record's own shape, with unknown fields denied: no field was added.
        let _: elohim_epr_rea::feedback::Feedback =
            serde_json::from_value(entry["record"].clone()).expect("known fields only");
    }
    for entry in &entries {
        value(run_env(
            dir.path(),
            &["show", "--id", &entry.cid.to_string()],
            &[],
        ));
    }
}

// ---- FB-1 review fixes (B1, B2, B4b, B7b) ----

fn spool_note(at: &str, session: &str, emitter: &str) -> String {
    json!({"v":1,"claim":"epr/recorder-unavailable:timeout","cause":"timeout","at":at,
        "emitter":emitter,"session":session,
        "recorder":{"path":null,"size":null,"mtime":null},"stderr":null})
    .to_string()
}

#[test]
fn dir_option_is_refused_outside_drain_spool() {
    let dir = claim_fixture();
    for args in [
        &[
            "capture",
            "--producer",
            "brit",
            "--operation",
            "status",
            "--outcome",
            "failed",
            "--invocation",
            "x",
            "--dir",
            "anything",
        ][..],
        &["list", "--dir", "anything"][..],
    ] {
        let output = run_env(dir.path(), args, &[]);
        assert!(!output.status.success(), "{args:?}");
        assert!(String::from_utf8_lossy(&output.stderr).contains("unknown feedback option"));
    }
    assert!(!dir.path().join(".eprfs/status/feedback.jsonl").exists());
}

#[test]
fn claim_capture_drops_an_emitter_that_is_not_a_release_identity() {
    let dir = claim_fixture();
    value(claim_capture(
        dir.path(),
        "refused",
        &[
            ("ELOHIM_SESSION_ID", "s"),
            ("EPR_FEEDBACK_EMITTER", "/home/alice/secret"),
        ],
    ));
    let rows = occurrences(dir.path());
    assert_eq!(rows[0]["source"]["item"], format!("{CLAIM} refused"));
    assert!(rows[0]["detail"].is_null());
    let retained =
        std::fs::read_to_string(dir.path().join(".eprfs/status/feedback.jsonl")).expect("store");
    assert!(!retained.contains("alice"));
}

#[test]
fn drain_spool_keeps_distinct_emitters_of_one_session_apart() {
    let dir = claim_fixture();
    let spool = dir.path().join("spool");
    std::fs::create_dir(&spool).unwrap();
    let session = "d".repeat(64);
    std::fs::write(
        spool.join("a.json"),
        spool_note(
            "2026-11-27T10:00:00Z",
            &session,
            "unrecorded:0.3.5+exe.0123456789ab",
        ),
    )
    .unwrap();
    std::fs::write(
        spool.join("b.json"),
        spool_note(
            "2026-11-27T10:01:00Z",
            &session,
            "unrecorded:0.3.6+exe.0123456789ab",
        ),
    )
    .unwrap();
    let spool_arg = spool.to_str().unwrap();
    let drained = value(run_env(
        dir.path(),
        &["drain-spool", "--dir", spool_arg],
        &[],
    ));
    assert_eq!(drained["drained"], 2);
    assert_eq!(occurrences(dir.path()).len(), 2);
    assert!(!spool.join("a.json").exists() && !spool.join("b.json").exists());
    // The same session, cause and emitter at another time is the same record.
    for (name, at) in [
        ("c.json", "2026-11-27T11:00:00Z"),
        ("d.json", "2026-11-27T12:00:00Z"),
    ] {
        std::fs::write(
            spool.join(name),
            spool_note(at, &"e".repeat(64), "unrecorded:0.3.6+exe.0123456789ab"),
        )
        .unwrap();
    }
    let drained = value(run_env(
        dir.path(),
        &["drain-spool", "--dir", spool_arg],
        &[],
    ));
    assert_eq!(drained["drained"], 2);
    assert_eq!(occurrences(dir.path()).len(), 3);
    assert!(!spool.join("c.json").exists() && !spool.join("d.json").exists());
}

#[test]
fn drain_spool_reads_at_most_256_notes_and_says_so() {
    let dir = claim_fixture();
    let spool = dir.path().join("spool");
    std::fs::create_dir(&spool).unwrap();
    for n in 0..258 {
        std::fs::write(
            spool.join(format!("{n:032x}.json")),
            spool_note(
                "2026-11-27T10:00:00Z",
                &format!("{n:064x}"),
                "unrecorded:1+exe.0",
            ),
        )
        .unwrap();
    }
    let spool_arg = spool.to_str().unwrap();
    let first = value(run_env(
        dir.path(),
        &["drain-spool", "--dir", spool_arg],
        &[],
    ));
    assert_eq!(first["drained"], 256);
    assert_eq!(first["read"], "256 or more");
    assert_eq!(std::fs::read_dir(&spool).unwrap().count(), 2);
    let second = value(run_env(
        dir.path(),
        &["drain-spool", "--dir", spool_arg],
        &[],
    ));
    assert_eq!(second["drained"], 2);
    assert_eq!(second["read"], 2);
    assert_eq!(occurrences(dir.path()).len(), 258);
}

#[test]
fn trailing_option_requires_value_and_names_the_option() {
    let dir = fixture();
    for option in ["--dir", "--root"] {
        let output = Command::new(env!("CARGO_BIN_EXE_epr"))
            .args(["feedback", "drain-spool", option])
            .current_dir(dir.path())
            .env("EPR_SESSION_LAZY", "1")
            .output()
            .expect("epr");
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr)
            .contains(&format!("option requires value: {option}")));
    }
}
