//! `epr flow intend --carry` and `--carry-decision`, through the binary: the claim is written as a
//! native commitment plus one crossing `Cite` on the intent atom, a replay never moves its bound,
//! and a device standing alone always decides to run.
#![cfg(feature = "peer")]

use std::path::Path;
use std::process::{Command, Output};

use elohim_epr_rea::{
    AgentRef, CommitmentState, FlowEvent, FlowRecord, FlowStore, Magnitude, ReaVerb,
    SidecarFlowStore,
};
use tempfile::TempDir;

const CHANGE: &str = "I0123456789abcdef0123456789abcdef01234567";
const TREE: &str = "0123456789abcdef0123456789abcdef01234567";
const BASE: &str = "1123456789abcdef0123456789abcdef01234567";

struct Fixture {
    root: TempDir,
    config: TempDir,
    proc_root: TempDir,
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join(".epr-meta")).unwrap();
        std::fs::write(
            root.path().join(".epr-meta/repository.yaml"),
            "version: 1\nagent: repo:test/landing\n",
        )
        .unwrap();
        run_git(root.path(), &["init"]);
        run_git(root.path(), &["config", "user.name", "Carry Test"]);
        run_git(root.path(), &["config", "user.email", "carry@example.test"]);
        std::fs::write(root.path().join("README.md"), "fixture\n").unwrap();
        run_git(
            root.path(),
            &["add", "README.md", ".epr-meta/repository.yaml"],
        );
        run_git(root.path(), &["commit", "-m", "fixture"]);
        std::fs::write(
            root.path().join("measures.yaml"),
            "measures:\n  - id: dev-landing\n    version: 1\n    unit: landings\n",
        )
        .unwrap();
        Self {
            root,
            config: tempfile::tempdir().unwrap(),
            proc_root: tempfile::tempdir().unwrap(),
        }
    }

    fn epr(&self, tail: &[&str]) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_epr"));
        command
            .args(["flow", "intend", "--root"])
            .arg(self.root.path())
            .arg("--json")
            .args(tail)
            .env("XDG_CONFIG_HOME", self.config.path())
            .env("EPR_PROC_ROOT", self.proc_root.path())
            .env("EPR_STORE", self.root.path().join(".eprfs"))
            .env(
                "ELOHIM_DEVICE_KEY_FILE",
                self.config.path().join("missing-device-key"),
            )
            .env_remove("EPR_PEER_RELAY_URL")
            .env_remove("EPR_PEER_RELAYS")
            .env_remove("CLAUDE_CODE_SESSION_ID")
            .env_remove("CLAUDE_SESSION_ID")
            .env_remove("ELOHIM_SESSION_ID");
        command.output().unwrap()
    }

    fn measures(&self) -> String {
        self.root.path().join("measures.yaml").display().to_string()
    }

    fn raise(&self) -> serde_json::Value {
        let measures = self.measures();
        success_json(self.epr(&[
            "--measures",
            &measures,
            "--change",
            CHANGE,
            "--tree",
            TREE,
            "--base",
            BASE,
            "--class",
            "low",
            "--closure",
            "eprfs=abcdef",
        ]))
    }

    /// Post this device's own misses event on `intent` for eprfs=abcdef, as the hand-over does,
    /// and return its CID.
    fn post_miss(&self, intent: &str) -> String {
        self.post_miss_for(intent, TREE, "refs/heads/self/carry-test")
    }

    fn post_miss_for(&self, intent: &str, commit: &str, git_ref: &str) -> String {
        let record = FlowRecord::Event(FlowEvent {
            action: ReaVerb::Cite,
            provider: AgentRef("human:test".to_string()),
            receiver: AgentRef("repo:test/landing".to_string()),
            resource: intent.parse().unwrap(),
            quantity: Magnitude::Count {
                value: 1.0,
                unit: "dev-landing".to_string(),
            },
            process: None,
            in_scope_of: intent.parse().unwrap(),
            fulfills: Vec::new(),
            satisfies: Vec::new(),
            occurred_at: "2026-11-23T00:00:00Z".to_string(),
            classified_as: vec![
                "dev-landing:misses".to_string(),
                CHANGE.to_string(),
                "reason:misses".to_string(),
                "measure:dev-landing@1".to_string(),
                "value:1".to_string(),
                format!("env:change={CHANGE}"),
                format!("env:commit={commit}"),
                format!("env:ref={git_ref}"),
                "env:miss=eprfs=abcdef".to_string(),
            ],
        });
        let mut store = SidecarFlowStore::open(self.root.path()).unwrap();
        store.append(record).unwrap().to_string()
    }

    fn carry(&self, intent: &str, miss: &str, valid_for: &str) -> Output {
        let measures = self.measures();
        self.epr(&[
            "--measures",
            &measures,
            "--carry",
            intent,
            "--project",
            "eprfs",
            "--closure",
            "abcdef",
            "--miss",
            miss,
            "--valid-for",
            valid_for,
        ])
    }

    fn records(&self) -> Vec<(cid::Cid, FlowRecord)> {
        SidecarFlowStore::open(self.root.path())
            .unwrap()
            .records()
            .unwrap()
    }
}

fn run_git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_string()
}

fn success_json(output: Output) -> serde_json::Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn a_carry_writes_a_commitment_and_one_crossing_cite_on_the_intent() {
    let fixture = Fixture::new();
    let raised = fixture.raise();
    let intent = raised["intent"].as_str().unwrap().to_string();
    let miss = fixture.post_miss(&intent);
    let before = fixture.records().len();

    let receipt = success_json(fixture.carry(&intent, &miss, "600"));
    assert_eq!(receipt["appended"], true);
    let records = fixture.records();
    assert_eq!(records.len(), before + 2, "one commitment and one event");

    let commitment = records
        .iter()
        .find_map(|(cid, record)| match record {
            FlowRecord::Commitment(c) if cid.to_string() == receipt["commitment"] => Some(c),
            _ => None,
        })
        .expect("the receipt's commitment is in the store");
    assert_eq!(commitment.action, ReaVerb::Produce);
    assert_eq!(commitment.state, CommitmentState::Active);
    assert_eq!(
        commitment
            .satisfies
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        vec![intent.clone()]
    );
    assert!(commitment.valid_until.is_some());

    let event = records
        .iter()
        .find_map(|(cid, record)| match record {
            FlowRecord::Event(e) if cid.to_string() == receipt["event"] => Some(e),
            _ => None,
        })
        .expect("the receipt's event is in the store");
    assert_eq!(event.action, ReaVerb::Cite);
    assert_eq!(event.resource.to_string(), intent);
    let slots = &event.classified_as;
    assert_eq!(slots[0], "dev-landing:carry");
    assert_eq!(slots[1], CHANGE);
    for expected in [
        "reason:carry".to_string(),
        "measure:dev-landing@1".to_string(),
        "value:1".to_string(),
        "env:project=eprfs".to_string(),
        "env:closure=abcdef".to_string(),
        format!("env:miss={miss}"),
        format!("env:commitment={}", receipt["commitment"].as_str().unwrap()),
        format!("env:valid_until={}", receipt["validUntil"]),
    ] {
        assert!(slots.contains(&expected), "missing {expected} in {slots:?}");
    }
    assert!(
        !slots.iter().any(|slot| slot.contains("Commitment")),
        "no serialized commitment rides the event"
    );
}

#[test]
fn a_replay_of_the_same_carry_appends_nothing_and_keeps_its_bound() {
    let fixture = Fixture::new();
    let raised = fixture.raise();
    let intent = raised["intent"].as_str().unwrap().to_string();
    let miss = fixture.post_miss(&intent);
    let first = success_json(fixture.carry(&intent, &miss, "600"));
    let count = fixture.records().len();
    let replay = success_json(fixture.carry(&intent, &miss, "7200"));
    assert_eq!(replay["appended"], false);
    assert_eq!(replay["validUntil"], first["validUntil"]);
    assert_eq!(replay["commitment"], first["commitment"]);
    assert_eq!(fixture.records().len(), count);
}

#[test]
fn a_carry_refuses_a_zero_bound_or_a_malformed_miss_before_any_append() {
    let fixture = Fixture::new();
    let raised = fixture.raise();
    let intent = raised["intent"].as_str().unwrap().to_string();
    let miss = fixture.post_miss(&intent);
    let count = fixture.records().len();
    assert!(!fixture.carry(&intent, &miss, "0").status.success());
    // Past the ceiling a claim is refused: it would hold other carriers off for longer than any gate.
    assert!(!fixture.carry(&intent, &miss, "90000").status.success());
    assert!(!fixture.carry(&intent, "not-a-cid", "60").status.success());
    assert!(!fixture.carry("not-a-cid", &miss, "60").status.success());
    // The raise event is no misses event: no open miss is named by it.
    let raise_event = raised["event"].as_str().unwrap().to_string();
    assert!(!fixture.carry(&intent, &raise_event, "60").status.success());
    assert_eq!(fixture.records().len(), count);
}

#[test]
fn a_carry_refuses_a_landed_miss_before_appending_any_record() {
    let fixture = Fixture::new();
    let raised = fixture.raise();
    let intent = raised["intent"].as_str().unwrap().to_string();
    let commit = run_git(fixture.root.path(), &["rev-parse", "HEAD"]);
    run_git(
        fixture.root.path(),
        &["update-ref", "refs/remotes/origin/dev", &commit],
    );
    let miss = fixture.post_miss_for(&intent, &commit, "refs/heads/dev");
    let count = fixture.records().len();

    let refused = fixture.carry(&intent, &miss, "600");

    assert!(!refused.status.success());
    assert!(
        String::from_utf8_lossy(&refused.stderr)
            .contains("the miss is settled: its commit landed on refs/remotes/origin/dev"),
        "{}",
        String::from_utf8_lossy(&refused.stderr)
    );
    assert_eq!(fixture.records().len(), count);
}

#[test]
fn a_carry_on_an_unknown_landing_still_appends_as_before() {
    let fixture = Fixture::new();
    let raised = fixture.raise();
    let intent = raised["intent"].as_str().unwrap().to_string();
    let miss = fixture.post_miss(&intent);
    let count = fixture.records().len();

    let receipt = success_json(fixture.carry(&intent, &miss, "600"));

    assert_eq!(receipt["appended"], true);
    assert_eq!(fixture.records().len(), count + 2);
}

#[test]
fn a_device_standing_alone_decides_to_run_in_both_roles() {
    let fixture = Fixture::new();
    let raised = fixture.raise();
    let intent = raised["intent"].as_str().unwrap().to_string();
    let miss = fixture.post_miss(&intent);
    success_json(fixture.carry(&intent, &miss, "600"));
    let ask = |role: &str| {
        success_json(fixture.epr(&[
            "--carry-decision",
            &intent,
            "--project",
            "eprfs",
            "--closure",
            "abcdef",
            "--role",
            role,
            "--max-wait-s",
            "600",
        ]))
    };
    assert_eq!(ask("carrier")["decision"], "run");
    assert_eq!(ask("lander")["decision"], "run-here");
    let bad = fixture.epr(&[
        "--carry-decision",
        &intent,
        "--project",
        "eprfs",
        "--closure",
        "abcdef",
        "--role",
        "steward",
    ]);
    assert!(!bad.status.success());
}
