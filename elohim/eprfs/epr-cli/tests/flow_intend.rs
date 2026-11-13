#![cfg(feature = "peer")]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use elohim_epr_cli::flow::{project, walk};
use elohim_epr_rea::{atom_cid, FlowEvent, FlowRecord, FlowStore, SidecarFlowStore};
use eprfs_peer::{fold_events, ChannelEvent, CrossingAllowList};
use tempfile::TempDir;

const CHANGE: &str = "I0123456789abcdef0123456789abcdef01234567";
const OTHER_CHANGE: &str = "I1123456789abcdef0123456789abcdef01234567";
const TREE: &str = "0123456789abcdef0123456789abcdef01234567";
const BASE: &str = "1123456789abcdef0123456789abcdef01234567";

struct Fixture {
    root: TempDir,
    config: TempDir,
    proc_root: TempDir,
    measures: PathBuf,
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
        run_git(root.path(), &["config", "user.name", "Landing Test"]);
        run_git(
            root.path(),
            &["config", "user.email", "landing@example.test"],
        );
        std::fs::write(root.path().join("README.md"), "fixture\n").unwrap();
        run_git(
            root.path(),
            &["add", "README.md", ".epr-meta/repository.yaml"],
        );
        run_git(root.path(), &["commit", "-m", "fixture"]);
        let measures = root.path().join("measures.yaml");
        std::fs::write(
            &measures,
            "measures:\n  - id: dev-landing\n    version: 1\n    unit: landings\n",
        )
        .unwrap();
        Self {
            root,
            config: tempfile::tempdir().unwrap(),
            proc_root: tempfile::tempdir().unwrap(),
            measures,
        }
    }

    fn epr(&self, tail: &[&str]) -> Output {
        self.epr_at(self.root.path(), tail)
    }

    fn epr_at(&self, root: &Path, tail: &[&str]) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_epr"));
        command
            .args(["flow", "intend", "--root"])
            .arg(root)
            .arg("--measures")
            .arg(&self.measures)
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

    fn raise(&self, change: &str) -> serde_json::Value {
        success_json(self.raise_output(change))
    }

    fn raise_output(&self, change: &str) -> Output {
        self.epr(&[
            "--change",
            change,
            "--tree",
            TREE,
            "--base",
            BASE,
            "--class",
            "low",
            "--closure",
            "eprfs=abcdef",
            "--predicted-base",
            BASE,
            "--chain-len",
            "2",
            "--session",
            "landing-test",
        ])
    }

    fn records(&self) -> Vec<(cid::Cid, FlowRecord)> {
        SidecarFlowStore::open(self.root.path())
            .unwrap()
            .records()
            .unwrap()
    }
}

fn run_git(root: &Path, args: &[&str]) {
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
}

fn success_json(output: Output) -> serde_json::Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn event_kind(record: &FlowRecord) -> Option<&str> {
    let FlowRecord::Event(event) = record else {
        return None;
    };
    event
        .classified_as
        .first()
        .and_then(|slot| slot.strip_prefix("dev-landing:"))
}

fn event(record: &FlowRecord) -> &FlowEvent {
    let FlowRecord::Event(event) = record else {
        panic!("expected event");
    };
    event
}

fn env<'a>(event: &'a FlowEvent, key: &str) -> Option<&'a str> {
    let prefix = format!("env:{key}=");
    event
        .classified_as
        .iter()
        .find_map(|slot| slot.strip_prefix(&prefix))
}

fn landing_allow() -> CrossingAllowList {
    CrossingAllowList::new([("dev-landing@1", ["intent", "landed", "withdraw", "dismiss"])])
}

#[test]
fn intend_raises_one_intent_with_the_change_id_as_its_subject() {
    let fixture = Fixture::new();
    let result = fixture.raise(CHANGE);
    assert_eq!(result["mode"], "raise");
    assert_eq!(result["appended"], true);
    assert_eq!(result["dseq"], 1);
    let records = fixture.records();
    assert_eq!(records.len(), 2);
    let FlowRecord::Intent(atom) = &records[0].1 else {
        panic!("first record is the intent atom");
    };
    assert_eq!(
        atom.resource_spec.classified_as,
        vec![
            "dev-landing:intent".to_string(),
            CHANGE.to_string(),
            format!("tree:{TREE}"),
            format!("base:{BASE}"),
            "class:low".to_string(),
            "closure:eprfs=abcdef".to_string(),
        ]
    );
    assert!(!atom
        .resource_spec
        .classified_as
        .iter()
        .any(|slot| slot.starts_with("at:")));
    assert_eq!(event_kind(&records[1].1), Some("raise"));
}

#[test]
fn intend_is_idempotent_on_the_same_live_closure() {
    let fixture = Fixture::new();
    let first = fixture.raise(CHANGE);
    let second = fixture.raise(CHANGE);
    assert_eq!(first["intent"], second["intent"]);
    assert_eq!(second["appended"], false);
    assert_eq!(second["dseq"], 1);
    assert_eq!(fixture.records().len(), 2);
}

#[test]
fn an_identical_hand_made_live_atom_gets_its_missing_raise_event() {
    let source = Fixture::new();
    source.raise(CHANGE);
    let atom = source.records()[0].1.clone();
    let fixture = Fixture::new();
    SidecarFlowStore::open(fixture.root.path())
        .unwrap()
        .append(atom)
        .unwrap();
    let repaired = fixture.raise(CHANGE);
    assert_eq!(repaired["appended"], true);
    assert_eq!(repaired["dseq"], 1);
    let records = fixture.records();
    assert_eq!(records.len(), 2);
    assert_eq!(event_kind(&records[1].1), Some("raise"));
}

#[test]
fn a_re_raise_after_withdraw_or_dismiss_mints_a_live_successor() {
    for mode in ["--withdraw", "--dismiss"] {
        let fixture = Fixture::new();
        let first = fixture.raise(CHANGE);
        let intent = first["intent"].as_str().unwrap();
        let tail = if mode == "--dismiss" {
            vec![
                mode,
                intent,
                "--capsule",
                "key_predicted=a",
                "--capsule",
                "key_dev=b",
                "--capsule",
                "project=eprfs",
                "--capsule",
                "chain=c",
                "--capsule",
                "heads=d",
                "--capsule",
                "now=e",
            ]
        } else {
            vec![mode, intent]
        };
        success_json(fixture.epr(&tail));
        let next = fixture.raise(CHANGE);
        assert_eq!(next["appended"], true);
        assert_ne!(next["intent"], first["intent"]);
        assert_eq!(next["superseded"], first["intent"]);
        let lines = std::fs::read_to_string(fixture.root.path().join(".eprfs/status/flows.jsonl"))
            .unwrap()
            .lines()
            .count();
        let third = fixture.raise(CHANGE);
        assert_eq!(third["appended"], false);
        assert_eq!(third["intent"], next["intent"]);
        assert_eq!(third["dseq"], next["dseq"]);
        assert_eq!(
            std::fs::read_to_string(fixture.root.path().join(".eprfs/status/flows.jsonl"))
                .unwrap()
                .lines()
                .count(),
            lines
        );
    }
}

#[test]
fn a_withdrawn_successor_is_superseded_by_the_lineage_head() {
    let fixture = Fixture::new();
    let first = fixture.raise(CHANGE);
    success_json(fixture.epr(&["--withdraw", first["intent"].as_str().unwrap()]));
    let successor = fixture.raise(CHANGE);
    success_json(fixture.epr(&["--withdraw", successor["intent"].as_str().unwrap()]));
    let next = fixture.raise(CHANGE);
    assert_eq!(next["superseded"], successor["intent"]);
    assert_ne!(next["superseded"], first["intent"]);
}

#[test]
fn an_identical_raise_never_returns_a_superseded_atom() {
    let fixture = Fixture::new();
    let first = fixture.raise(CHANGE);
    let replacement = success_json(fixture.epr(&[
        "--change",
        CHANGE,
        "--tree",
        BASE,
        "--base",
        TREE,
        "--class",
        "low",
        "--supersedes",
        first["intent"].as_str().unwrap(),
    ]));
    let refused = fixture.raise_output(CHANGE);
    assert!(!refused.status.success());
    let stderr = String::from_utf8_lossy(&refused.stderr);
    assert!(
        stderr.contains(replacement["intent"].as_str().unwrap()),
        "{stderr}"
    );
}

#[test]
fn closure_order_does_not_change_semantic_identity() {
    let fixture = Fixture::new();
    let first = success_json(fixture.epr(&[
        "--change",
        CHANGE,
        "--tree",
        TREE,
        "--base",
        BASE,
        "--class",
        "low",
        "--closure",
        "z=bbbb",
        "--closure",
        "a=aaaa",
    ]));
    let second = success_json(fixture.epr(&[
        "--change",
        CHANGE,
        "--tree",
        TREE,
        "--base",
        BASE,
        "--class",
        "low",
        "--closure",
        "a=aaaa",
        "--closure",
        "z=bbbb",
    ]));
    assert_eq!(first["intent"], second["intent"]);
    assert_eq!(second["appended"], false);
}

#[test]
fn intend_refuses_malformed_change_tree_class_before_any_append() {
    let fixture = Fixture::new();
    for tail in [
        vec![
            "--change", "bad", "--tree", TREE, "--base", BASE, "--class", "low",
        ],
        vec![
            "--change", CHANGE, "--tree", "bad", "--base", BASE, "--class", "low",
        ],
        vec![
            "--change", CHANGE, "--tree", TREE, "--base", BASE, "--class", "evidence",
        ],
    ] {
        assert!(!fixture.epr(&tail).status.success());
    }
    assert!(!fixture
        .root
        .path()
        .join(".eprfs/status/flows.jsonl")
        .exists());
}

#[test]
fn undeclared_measure_refuses_before_any_append() {
    let fixture = Fixture::new();
    std::fs::write(&fixture.measures, "measures: []\n").unwrap();
    let output = fixture.epr(&[
        "--change", CHANGE, "--tree", TREE, "--base", BASE, "--class", "low",
    ]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unknown measure"));
}

#[test]
fn a_second_live_intent_for_one_change_needs_supersedes() {
    let fixture = Fixture::new();
    let first = fixture.raise(CHANGE);
    let output = fixture.epr(&[
        "--change", CHANGE, "--tree", BASE, "--base", TREE, "--class", "low",
    ]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains(first["intent"].as_str().unwrap()));
    let first_cid = first["intent"].as_str().unwrap();
    let replacement = success_json(fixture.epr(&[
        "--change",
        CHANGE,
        "--tree",
        BASE,
        "--base",
        TREE,
        "--class",
        "low",
        "--supersedes",
        first_cid,
    ]));
    assert_eq!(replacement["appended"], true);
    let other = fixture.raise(OTHER_CHANGE);
    let wrong_change = fixture.epr(&[
        "--change",
        CHANGE,
        "--tree",
        TREE,
        "--base",
        TREE,
        "--class",
        "low",
        "--supersedes",
        other["intent"].as_str().unwrap(),
    ]);
    assert!(!wrong_change.status.success());
    assert!(String::from_utf8_lossy(&wrong_change.stderr).contains(OTHER_CHANGE));
}

#[test]
fn verified_writes_one_crossing_intent_cite() {
    let fixture = Fixture::new();
    let raised = fixture.raise(CHANGE);
    let intent = raised["intent"].as_str().unwrap();
    let verified = success_json(fixture.epr(&["--verified", intent, "--gate-s", "9"]));
    assert_eq!(verified["dseq"], 1);
    let records = fixture.records();
    let raised_event = event(&records[1].1);
    let verified_event = event(&records[2].1);
    assert_eq!(event_kind(&records[2].1), Some("intent"));
    assert_eq!(verified_event.classified_as[1], CHANGE);
    assert_eq!(env(verified_event, "at"), env(raised_event, "at"));
    assert_eq!(env(verified_event, "gate_s"), Some("9"));
    assert!(landing_allow().admits(verified_event));
    assert!(!landing_allow().admits(raised_event));
    let replay = success_json(fixture.epr(&["--verified", intent, "--gate-s", "9"]));
    assert_eq!(replay["appended"], false);
}

#[test]
fn land_writes_produce_satisfying_the_intent() {
    let fixture = Fixture::new();
    let raised = fixture.raise(CHANGE);
    let intent = raised["intent"].as_str().unwrap();
    success_json(fixture.epr(&[
        "--land",
        intent,
        "--tip",
        TREE,
        "--commit",
        BASE,
        "--landed-as",
        "moved",
        "--regated-s",
        "12",
    ]));
    let records = fixture.records();
    let FlowRecord::Event(event) = &records.last().unwrap().1 else {
        panic!("landing is an event");
    };
    assert_eq!(event.satisfies, [intent.parse().unwrap()]);
    assert!(event.fulfills.is_empty());
    assert_eq!(event.classified_as[1], CHANGE);
    assert_eq!(env(event, "landed_as"), Some("moved"));
    assert_eq!(env(event, "regated_s"), Some("12"));
    assert_eq!(env(event, "tip"), Some(TREE));
    assert_eq!(env(event, "commit"), Some(BASE));
    assert!(landing_allow().admits(event));
    assert_eq!(event_kind(&records.last().unwrap().1), Some("landed"));
}

#[test]
fn withdraw_and_dismiss_carry_their_env_and_cross() {
    let fixture = Fixture::new();
    let raised = fixture.raise(CHANGE);
    let intent = raised["intent"].as_str().unwrap();
    success_json(fixture.epr(&["--withdraw", intent, "--why", "interrupted"]));
    let second = Fixture::new();
    let raised = second.raise(CHANGE);
    let intent = raised["intent"].as_str().unwrap();
    success_json(second.epr(&[
        "--dismiss",
        intent,
        "--capsule",
        "key_predicted=a,b",
        "--capsule",
        "key_dev=b",
        "--capsule",
        "project=eprfs",
        "--capsule",
        "chain=c",
        "--capsule",
        "heads=d",
        "--capsule",
        "now=e",
    ]));
    assert!(fixture
        .records()
        .iter()
        .any(|(_, row)| event_kind(row) == Some("withdraw")));
    let withdraw = fixture.records();
    let withdraw = event(&withdraw.last().unwrap().1);
    assert_eq!(withdraw.classified_as[1], CHANGE);
    assert_eq!(env(withdraw, "why"), Some("interrupted"));
    assert!(landing_allow().admits(withdraw));
    assert!(second
        .records()
        .iter()
        .any(|(_, row)| event_kind(row) == Some("dismiss")));
    let dismissed = second.records();
    let dismissed = event(&dismissed.last().unwrap().1);
    assert_eq!(dismissed.classified_as[1], CHANGE);
    for (key, value) in [
        ("key_predicted", "a,b"),
        ("key_dev", "b"),
        ("project", "eprfs"),
        ("chain", "c"),
        ("heads", "d"),
        ("now", "e"),
    ] {
        assert_eq!(env(dismissed, key), Some(value));
    }
    assert!(landing_allow().admits(dismissed));
    let refused = second.epr(&["--dismiss", intent, "--capsule", "at=reserved"]);
    assert!(!refused.status.success());
}

#[test]
fn dseq_is_the_authors_ordinal_and_ignores_folded_rows() {
    let fixture = Fixture::new();
    let first = fixture.raise(CHANGE);
    let records = fixture.records();
    let mut folded = event(&records[1].1).clone();
    for slot in &mut folded.classified_as {
        if slot.starts_with("env:dseq=") {
            *slot = "env:dseq=99".to_string();
        }
    }
    folded.classified_as.push("via:did:key:sibling".to_string());
    SidecarFlowStore::open(fixture.root.path())
        .unwrap()
        .append(FlowRecord::Event(folded))
        .unwrap();
    success_json(fixture.epr(&["--withdraw", first["intent"].as_str().unwrap()]));
    let next = fixture.raise(CHANGE);
    assert_eq!(next["dseq"], 3);
}

#[test]
fn landing_records_leave_the_plan_frontier_unchanged() {
    let fixture = Fixture::new();
    let recipes = fixture.root.path().join("recipes.yaml");
    std::fs::create_dir_all(fixture.root.path().join("docs")).unwrap();
    std::fs::create_dir_all(fixture.root.path().join("gap")).unwrap();
    std::fs::write(
        &recipes,
        "version: 1\nrecipes:\n  - id: landing-parity\n    version: 1\n    description: parity\n    stages:\n      - name: spec\n        artifactKind: 'doc:spec'\n        paths: ['docs/*.md']\n      - name: intent\n        artifactKind: 'gap-item'\n        paths: ['gap/*.json']\n    edges:\n      - { from: spec, to: intent, meaningful: true, validators: [decompose] }\n",
    )
    .unwrap();
    std::fs::write(
        fixture.root.path().join("docs/spec.md"),
        "---\nid: landing-parity\ntitle: Landing parity\n---\n\nParity fixture.\n",
    )
    .unwrap();
    std::fs::write(
        fixture.root.path().join("gap/spec.json"),
        r#"{"doc":"docs/spec.md","slug":"landing-parity","items":[{"id":"landing-parity#1","state":"CLAIMED"}]}"#,
    )
    .unwrap();
    project::project(fixture.root.path(), &recipes).unwrap();
    let before = walk::status(fixture.root.path()).unwrap();
    assert!(before.unfulfilled_total > 0);
    let raised = fixture.raise(CHANGE);
    let after_raise = walk::status(fixture.root.path()).unwrap();
    assert_eq!(after_raise.intents, before.intents + 1);
    assert_eq!(after_raise.events, before.events + 1);
    success_json(fixture.epr(&["--verified", raised["intent"].as_str().unwrap()]));
    success_json(fixture.epr(&[
        "--land",
        raised["intent"].as_str().unwrap(),
        "--tip",
        TREE,
        "--commit",
        BASE,
        "--landed-as",
        "ff",
    ]));
    success_json(fixture.epr(&["--withdraw", raised["intent"].as_str().unwrap()]));
    success_json(fixture.epr(&[
        "--dismiss",
        raised["intent"].as_str().unwrap(),
        "--capsule",
        "key_predicted=a",
        "--capsule",
        "key_dev=b",
        "--capsule",
        "project=eprfs",
        "--capsule",
        "chain=c",
        "--capsule",
        "heads=d",
        "--capsule",
        "now=e",
    ]));
    let after = walk::status(fixture.root.path()).unwrap();
    assert_eq!(after.events, before.events + 5);
    assert_eq!(after.intents, before.intents + 1);
    assert_eq!(after.unfulfilled_total, before.unfulfilled_total);
    assert_eq!(after.commitments_active, before.commitments_active);
    assert_eq!(
        serde_json::to_value(&after.top_unfulfilled).unwrap(),
        serde_json::to_value(&before.top_unfulfilled).unwrap()
    );
}

#[test]
fn every_mode_refuses_a_root_other_than_the_served_channel_store() {
    let fixture = Fixture::new();
    let other = tempfile::tempdir().unwrap();
    let peer_dir = fixture.config.path().join("epr/peer");
    let table = eprfs_peer::HeadTable {
        store: Some(eprfs_peer::StorePlace {
            root: fixture.root.path().canonicalize().unwrap(),
            log: fixture.root.path().join(".eprfs/status/flows.jsonl"),
        }),
        ..eprfs_peer::HeadTable::default()
    };
    std::fs::create_dir_all(fixture.root.path().join(".eprfs/status")).unwrap();
    std::fs::write(fixture.root.path().join(".eprfs/status/flows.jsonl"), "").unwrap();
    table.save(&peer_dir).unwrap();
    let raised = fixture.raise(CHANGE);
    let intent = raised["intent"].as_str().unwrap();
    for tail in [
        vec![
            "--change", CHANGE, "--tree", TREE, "--base", BASE, "--class", "low",
        ],
        vec!["--verified", intent],
        vec![
            "--land",
            intent,
            "--tip",
            TREE,
            "--commit",
            BASE,
            "--landed-as",
            "ff",
        ],
        vec!["--withdraw", intent],
        vec![
            "--dismiss",
            intent,
            "--capsule",
            "key_predicted=a",
            "--capsule",
            "key_dev=b",
            "--capsule",
            "project=eprfs",
            "--capsule",
            "chain=c",
            "--capsule",
            "heads=d",
            "--capsule",
            "now=e",
        ],
    ] {
        let before = std::fs::read_dir(other.path()).unwrap().count();
        let output = fixture.epr_at(other.path(), &tail);
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("pass --root"));
        assert_eq!(std::fs::read_dir(other.path()).unwrap().count(), before);
    }
    std::fs::write(peer_dir.join("heads.json"), "not json").unwrap();
    let corrupt = fixture.raise_output(OTHER_CHANGE);
    assert!(!corrupt.status.success());
    assert!(String::from_utf8_lossy(&corrupt.stderr).contains("unreadable or does not parse"));
    let gone = fixture.root.path().join("gone-flows.jsonl");
    eprfs_peer::HeadTable {
        store: Some(eprfs_peer::StorePlace {
            root: fixture.root.path().to_path_buf(),
            log: gone.clone(),
        }),
        ..eprfs_peer::HeadTable::default()
    }
    .save(&peer_dir)
    .unwrap();
    let missing = fixture.raise_output(OTHER_CHANGE);
    assert!(!missing.status.success());
    assert!(String::from_utf8_lossy(&missing.stderr).contains("is gone"));
    assert!(!gone.exists());
}

#[test]
fn stale_allow_list_refuses_the_row_and_the_current_one_folds_it() {
    let fixture = Fixture::new();
    let raised = fixture.raise(CHANGE);
    success_json(fixture.epr(&["--verified", raised["intent"].as_str().unwrap()]));
    let records = fixture.records();
    let crossing = event(&records[2].1).clone();
    let channel = ChannelEvent {
        cid: atom_cid(&crossing).unwrap(),
        event: crossing,
    };
    let stale_root = tempfile::tempdir().unwrap();
    let stale = fold_events(
        &SidecarFlowStore::open(stale_root.path()).unwrap(),
        "did:key:sibling",
        std::slice::from_ref(&channel),
        &CrossingAllowList::new([("gate-verdict@1", ["verdict"])]),
    )
    .unwrap();
    assert_eq!(stale.refused, 1);
    let current_root = tempfile::tempdir().unwrap();
    let current_store = SidecarFlowStore::open(current_root.path()).unwrap();
    let current = fold_events(
        &current_store,
        "did:key:sibling",
        &[channel],
        &landing_allow(),
    )
    .unwrap();
    assert_eq!(current.appended, 1);
    assert!(event(&current_store.records().unwrap()[0].1)
        .classified_as
        .iter()
        .any(|slot| slot == "via:did:key:sibling"));
}
