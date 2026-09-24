//! The standing-human attribution arm, end to end through the built `epr` binary: a device whose
//! human was witnessed attributes a hook-shaped note (no `--as`, no `--session`) to that human,
//! with `source:claim-signed` and the handle — never the email — in the note's slots.
//!
//! The device key is a temp file named on the CHILD process only (`ELOHIM_DEVICE_KEY_FILE`), so
//! this exercises the same env resolution a real session uses without touching this device's real
//! key or mutating the environment other tests share. The command-level tests live in
//! `actor_witness.rs`.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use elohim_epr_cli::actor::witness;
use elohim_epr_cli::device_key::{self, DeviceKey};
use tempfile::TempDir;

const SUBJECT: &str = "human:matthew";
const WITNESS: &str = "agent:orchestrator@claude-fable-5-1";
const SESSION: &str = "witness-session";
const BASIS: &str = "operator of this stewarded device; present in this session";

fn git(root: &Path, args: &[&str]) {
    let out = elohim_epr_cli::process::build_command("git", args, root, &[])
        .env("GIT_AUTHOR_NAME", "Fixture Author")
        .env("GIT_AUTHOR_EMAIL", "author@example.test")
        .env("GIT_COMMITTER_NAME", "Fixture Author")
        .env("GIT_COMMITTER_EMAIL", "author@example.test")
        .output()
        .expect("git runs");
    assert!(out.status.success(), "git {args:?} failed");
}

fn fixture() -> TempDir {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("README.md"), "fixture\n").unwrap();
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["add", "-A"]);
    git(dir.path(), &["commit", "-q", "-m", "fixture"]);
    dir
}

fn device(keys: &TempDir, name: &str) -> (PathBuf, DeviceKey) {
    let path = keys.path().join(name).join("ed25519.seed");
    let key = DeviceKey::load_or_generate(&path).unwrap();
    (path, key)
}

fn epr(root: &Path, key_file: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_epr"))
        .args(args)
        .arg("--root")
        .arg(root)
        .env(device_key::DEVICE_KEY_ENV, key_file)
        .env_remove("CLAUDE_CODE_SESSION_ID")
        .env_remove("CLAUDE_SESSION_ID")
        .env_remove("ELOHIM_SESSION_ID")
        .output()
        .expect("epr runs")
}

#[test]
fn note_attribution_uses_standing_human_with_claim_signed_source() {
    let dir = fixture();
    let keys = TempDir::new().unwrap();
    let root = dir.path();
    let (key_file, key) = device(&keys, "a");
    let witnessed = witness(root, SUBJECT, WITNESS, SESSION, BASIS, false, &key).unwrap();

    // A hook-shaped note: no --as, no --session.
    let out = epr(
        root,
        &key_file,
        &[
            "flow",
            "note",
            "--on",
            "README.md",
            "--kind",
            "observation",
            "--reason",
            "hook-emitted",
            "--json",
        ],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["actor"], SUBJECT);
    assert_eq!(json["steward"], SUBJECT);
    assert_eq!(json["actor_claim"], witnessed.record_cid);

    let flows = std::fs::read_to_string(root.join(".eprfs/status/flows.jsonl")).unwrap();
    let line: serde_json::Value = serde_json::from_str(flows.lines().last().unwrap()).unwrap();
    let slots: Vec<String> = serde_json::from_value(line["record"]["classifiedAs"].clone())
        .unwrap_or_else(|_| panic!("classifiedAs slots: {line}"));
    assert!(
        slots.contains(&"source:claim-signed".to_string()),
        "{slots:?}"
    );
    assert_eq!(
        slots.last().unwrap(),
        &format!("steward:{SUBJECT}"),
        "{slots:?}"
    );
    assert!(
        !slots.iter().any(|s| s.contains("example.test")),
        "no email anywhere in the slots: {slots:?}"
    );

    // An unwitnessed device keeps the old arm: the commit author, no source slot.
    let bare = fixture();
    let out = epr(
        bare.path(),
        &key_file,
        &[
            "flow",
            "note",
            "--on",
            "README.md",
            "--kind",
            "observation",
            "--reason",
            "hook-emitted",
            "--json",
        ],
    );
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["actor"], serde_json::Value::Null);
    let flows = std::fs::read_to_string(bare.path().join(".eprfs/status/flows.jsonl")).unwrap();
    assert!(!flows.contains("claim-signed"), "{flows}");
}

/// `epr` with the session variables set on the child exactly as given (the harness's own removed).
fn epr_with_env(root: &Path, key_file: &Path, args: &[&str], envs: &[(&str, &str)]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_epr"));
    command
        .args(args)
        .arg("--root")
        .arg(root)
        .env(device_key::DEVICE_KEY_ENV, key_file)
        .env_remove("CLAUDE_CODE_SESSION_ID")
        .env_remove("CLAUDE_SESSION_ID")
        .env_remove("ELOHIM_SESSION_ID");
    for (key, value) in envs {
        command.env(key, value);
    }
    command.output().expect("epr runs")
}

fn last_slots(root: &Path) -> Vec<String> {
    let flows = std::fs::read_to_string(root.join(".eprfs/status/flows.jsonl")).unwrap();
    let line: serde_json::Value = serde_json::from_str(flows.lines().last().unwrap()).unwrap();
    serde_json::from_value(line["record"]["classifiedAs"].clone()).unwrap()
}

const NOTE: [&str; 9] = [
    "flow",
    "note",
    "--on",
    "README.md",
    "--kind",
    "observation",
    "--reason",
    "hook-emitted",
    "--json",
];

#[test]
fn w4_env_session_stamps_source_session_env() {
    let dir = fixture();
    let keys = TempDir::new().unwrap();
    let root = dir.path();
    let (key_file, _) = device(&keys, "a");
    elohim_epr_cli::actor::claim(root, "agent:implementer@opus-5.5", "sub-session").unwrap();
    elohim_epr_cli::actor::claim(root, "agent:orchestrator@fable-5", "harness-session").unwrap();

    // The subagent's own ELOHIM_SESSION_ID beats the harness's CLAUDE_CODE_SESSION_ID, and the
    // inference is stamped on the record.
    let out = epr_with_env(
        root,
        &key_file,
        &NOTE,
        &[
            ("ELOHIM_SESSION_ID", "sub-session"),
            ("CLAUDE_CODE_SESSION_ID", "harness-session"),
        ],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["actor"], "agent:implementer@opus-5.5");
    let slots = last_slots(root);
    assert!(
        slots.contains(&"source:session-env".to_string()),
        "{slots:?}"
    );
    assert_eq!(
        slots.last().unwrap(),
        "steward:repo:ethosengine/elohim",
        "no standing human: the collective stewards, never the email: {slots:?}"
    );

    // Named with --session, the same claim attributes and nothing is stamped.
    let named = [
        "flow",
        "note",
        "--on",
        "README.md",
        "--kind",
        "observation",
        "--reason",
        "named",
        "--session",
        "sub-session",
        "--json",
    ];
    let out = epr_with_env(
        root,
        &key_file,
        &named,
        &[("CLAUDE_CODE_SESSION_ID", "harness-session")],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["actor"], "agent:implementer@opus-5.5");
    assert!(
        !last_slots(root).iter().any(|s| s.starts_with("source:")),
        "{:?}",
        last_slots(root)
    );
}
