//! `epr flow context <dir> --participate` — the participation read, cut 1
//! (`genesis/docs/superpowers/specs/2026-11-22-participation-read-design.md`, P6–P8, P14–P16).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use cid::Cid;
use elohim_epr_cli::flow::participate::{self, Probes};
use elohim_epr_cli::flow::{body_cid_of_file, repository_scope};
use elohim_epr_rea::{
    AgentRef, Bound, Commitment, CommitmentState, FlowEvent, FlowRecord, FlowStore, Intent,
    Magnitude, ReaVerb, ResourceSpec, SidecarFlowStore,
};
use serde_json::Value;
use tempfile::TempDir;

const EPR: &str = env!("CARGO_BIN_EXE_epr");
const LIMIT: usize = participate::DEFAULT_LIMIT;
const EXCLUDED: &str = "Excluded (counts of in-scope records this caller can see)";

fn write(root: &Path, rel: &str, contents: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, contents).unwrap();
}

fn git(root: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(root)
        .env("GIT_AUTHOR_NAME", "fixture")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.invalid")
        .env("GIT_COMMITTER_NAME", "fixture")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.invalid")
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?}");
}

fn habit(id: &str, status: &str) -> String {
    format!(
        "---\nepr-habit-version: 1\nid: {id}\ninvariant: Works repeatedly\nstatus: {status}\n\
         checks: [just test]\nretire-when: Superseded by a stronger invariant\n---\n"
    )
}

fn spec(slots: &[&str]) -> ResourceSpec {
    ResourceSpec {
        classified_as: slots.iter().map(|s| s.to_string()).collect(),
        quantity: None,
    }
}

fn station(scope: Cid, state: &str, id: &str) -> FlowRecord {
    FlowRecord::Intent(Intent {
        action: ReaVerb::Produce,
        resource_spec: spec(&[state, id]),
        in_scope_of: scope,
        raised_by: AgentRef("tool:decompose".into()),
    })
}

fn commitment(scope: Cid, id: &str, provider: &str, bound: Option<Bound>) -> FlowRecord {
    FlowRecord::Commitment(Commitment {
        action: ReaVerb::Produce,
        provider: AgentRef(provider.into()),
        receiver: AgentRef("repo:fixture".into()),
        resource_spec: spec(&["gap:claimed", id]),
        in_scope_of: scope,
        valid_from: None,
        valid_until: None,
        state: CommitmentState::Active,
        satisfies: vec![],
        bound,
    })
}

fn event(resource: Cid, scope: Cid, unit: &str, slots: &[&str], fulfills: Vec<Cid>) -> FlowRecord {
    FlowRecord::Event(FlowEvent {
        action: ReaVerb::Cite,
        provider: AgentRef("agent:fixture@model".into()),
        receiver: AgentRef("repo:fixture".into()),
        resource,
        quantity: Magnitude::Count {
            value: 1.0,
            unit: unit.into(),
        },
        process: None,
        in_scope_of: scope,
        fulfills,
        satisfies: vec![],
        occurred_at: "2026-11-22T12:00:00Z".into(),
        classified_as: slots.iter().map(|s| s.to_string()).collect(),
    })
}

/// A checkout with in-scope records under `plans/` and, when `outside`, records under `other/`.
fn fixture(outside: bool) -> TempDir {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(
        root,
        ".epr-meta/repository.yaml",
        "version: 1\nagent: repo:fixture/participate\n",
    );
    write(root, "plans/epic.md", "# Epic\n\n- [ ] one\n- [x] two\n");
    write(
        root,
        "plans/.epr-meta/plans-red.habit.md",
        &habit("plans-red", "red"),
    );
    write(
        root,
        "plans/.epr-meta/plans-green.habit.md",
        &habit("plans-green", "green"),
    );
    write(root, "other/far.md", "# Far away\n");
    write(
        root,
        "other/.epr-meta/far-red.habit.md",
        &habit("far-red", "red"),
    );
    // The device's own capacity and presence answers, read-only, as berth prints them.
    fake_berth(
        &root.join("genesis/agentic/bin/berth"),
        &status_print(STATUS_JSON),
    );

    let epic = body_cid_of_file(&root.join("plans/epic.md")).unwrap();
    let far = body_cid_of_file(&root.join("other/far.md")).unwrap();
    let repo = repository_scope(root).unwrap();
    let mut labels = BTreeMap::new();
    labels.insert(epic.to_string(), "plans/epic.md".to_string());
    if outside {
        labels.insert(far.to_string(), "other/far.md".to_string());
    }

    let mut store = SidecarFlowStore::open(root).unwrap();
    store
        .append(station(epic, "gap:open", "plans__epic#1"))
        .unwrap();
    store
        .append(station(epic, "gap:claimed", "plans__epic#2"))
        .unwrap();
    store
        .append(commitment(
            epic,
            "plans__epic#2",
            "tool:decompose-claim",
            None,
        ))
        .unwrap();
    let bounded = commitment(
        epic,
        "plans__epic#3",
        "agent:owner@model",
        Some(Bound::new(987_654.0, "widgets".into(), 85.0).unwrap()),
    );
    store.append(bounded).unwrap();
    let done = store
        .append(commitment(epic, "plans__epic#4", "agent:done@model", None))
        .unwrap();
    store
        .append(event(
            done,
            epic,
            "run-note-not",
            &["run:done", "plans/epic.md"],
            vec![done],
        ))
        .unwrap();
    // A live landing intent, scoped to the repository, with gate seconds on its verify event.
    let landing = store
        .append(FlowRecord::Intent(Intent {
            action: ReaVerb::Produce,
            resource_spec: spec(&[
                "dev-landing:intent",
                "Ifixture",
                "tree:abc",
                "base:def",
                "class:low",
            ]),
            in_scope_of: repo,
            raised_by: AgentRef("agent:lander@model".into()),
        }))
        .unwrap();
    store
        .append(event(
            landing,
            repo,
            "dev-landing",
            &[
                "dev-landing:intent",
                "Ifixture",
                "reason:intent",
                "measure:dev-landing@1",
                "value:1",
                "env:gate_s=42",
            ],
            vec![],
        ))
        .unwrap();
    // A broadcast note: testimony on the epic, naming its steward.
    store
        .append(event(
            epic,
            epic,
            "run-note",
            &[
                "run:ruling",
                "plans/epic.md",
                "reason:keep it small",
                "steward:human:matthew",
            ],
            vec![],
        ))
        .unwrap();
    if outside {
        store
            .append(station(far, "gap:open", "other__far#1"))
            .unwrap();
        store
            .append(commitment(far, "other__far#1", "agent:far@model", None))
            .unwrap();
        store
            .append(event(
                far,
                far,
                "run-note",
                &["run:ruling", "other/far.md", "reason:elsewhere"],
                vec![],
            ))
            .unwrap();
    }
    drop(store);
    write(
        root,
        ".eprfs/status/labels.json",
        &serde_json::to_string(&labels).unwrap(),
    );
    git(root, &["init", "-q"]);
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "-m", "fixture"]);
    dir
}

const ROOM_JSON: &str = r#"{"resource":"cargo","verdict":"admit","reason":"2 seats, 0 build(s) running","axis":null,"owner":null}"#;

/// `berth status --json` in its published contract: two live local sessions (one declaring a
/// path under `plans/`, one elsewhere), one sibling session declaring `plans/**`, a silent device
/// that has acknowledged nothing, and an exchanging one.
const STATUS_JSON: &str = r#"{"device":"did:key:z6MkThisDevice000000000000000000000","moorings":[{"session":"aaaaaaaa-1111-2222","runtime":"claude","model":"opus","task":"plans lane","writes":["plans/epic.md"],"seenS":12,"declaredS":30,"observedS":null},{"session":"bbbbbbbb-3333-4444","runtime":"codex","model":"sol","task":"far lane","writes":["other/far.md"],"seenS":5,"declaredS":null,"observedS":null}],"siblings":[{"device":"did:key:z6MkSiblingDevice00000000000000abcd","session":"cccccccc-5555-6666","runtime":"codex","task":"sibling lane","writes":["plans/**"],"seenS":300}],"devices":[{"device":"did:key:z6MkgHjuLaptopNeverHeard000000000xMbu","headAgeS":null,"silent":true,"unacked":{"reason":"never","gap":null}},{"device":"did:key:z6MkSiblingDevice00000000000000abcd","headAgeS":3,"silent":false,"unacked":null}],"unavailable":[]}"#;

fn status_print(json: &str) -> String {
    format!("printf '%s\\n' '{json}'")
}

/// A berth that answers `room cargo --json` and runs `status` for `status --json`.
fn fake_berth(path: &Path, status: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(
        path,
        format!(
            "#!/bin/sh\n# status-json@1\ncase \"$1\" in\n  room) printf '%s\\n' '{ROOM_JSON}' ;;\n  status) {status} ;;\nesac\n"
        ),
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
}

fn probes(root: &Path) -> Probes {
    Probes {
        berth: Some(root.join("genesis/agentic/bin/berth")),
        device_key: Some(root.join("no-device-key")),
    }
}

/// Every path under `root` with its bytes (directories as `None`), for byte-identity checks.
fn tree(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    let mut out = BTreeMap::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            let rel = path.strip_prefix(root).unwrap().to_path_buf();
            if entry.file_type().unwrap().is_dir() {
                out.insert(rel, None);
                stack.push(path);
            } else {
                out.insert(rel, Some(std::fs::read(&path).unwrap()));
            }
        }
    }
    out
}

fn run_epr(root: &Path, config: &Path, args: &[&str]) -> std::process::Output {
    Command::new(EPR)
        .args(args)
        .arg("--root")
        .arg(root)
        .current_dir(root)
        .env_remove("BERTH_BIN")
        .env_remove("EPR_BIN")
        .env_remove("ELOHIM_DEVICE_KEY_FILE")
        .env("XDG_CONFIG_HOME", config.join("xdg-config"))
        .env("HOME", config.join("home"))
        .output()
        .unwrap()
}

fn group<'a>(json: &'a Value, source: &str) -> &'a Vec<Value> {
    json["groups"]
        .as_array()
        .unwrap()
        .iter()
        .find(|g| g["source"] == source)
        .unwrap_or_else(|| panic!("no group {source}: {json:#}"))["items"]
        .as_array()
        .unwrap()
}

fn as_json(result: &participate::ParticipationResult) -> Value {
    serde_json::to_value(result).unwrap()
}

// ── P15: the read opens nothing that creates ────────────────────────────────────────────────

#[test]
fn participate_against_no_stores_leaves_the_directory_byte_identical() {
    let dir = TempDir::new().unwrap();
    let config = TempDir::new().unwrap();
    let before = tree(dir.path());
    let config_before = tree(config.path());
    for json in [false, true] {
        let mut args = vec!["flow", "context", ".", "--participate"];
        if json {
            args.push("--json");
        }
        let output = run_epr(dir.path(), config.path(), &args);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{stdout}\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        if json {
            let value: Value = serde_json::from_str(&stdout).unwrap();
            assert_eq!(value["outcome"], "no visible candidate established");
        } else {
            let mut lines = stdout.lines();
            assert_eq!(
                lines.next(),
                Some("Participation: this device's own records, for its steward's session; completeness unknown")
            );
            assert!(
                stdout.contains("\nNo visible candidate established\n"),
                "{stdout}"
            );
            assert!(
                stdout.contains("\nSources: moorings unavailable (berth not found at ")
                    && stdout.contains(
                        "; stations unavailable (no flow store at .eprfs/status/flows.jsonl)"
                    ),
                "{stdout}"
            );
            assert!(stdout.contains("Covenant scope: unknown (no non-initialising reader)"));
            assert!(
                stdout.contains(&format!("\n{EXCLUDED}: none\n")),
                "{stdout}"
            );
        }
    }
    assert_eq!(tree(dir.path()), before, "no file or directory created");
    assert_eq!(
        tree(config.path()),
        config_before,
        "no key or config created"
    );
}

#[test]
fn participate_against_a_populated_fixture_changes_nothing() {
    let dir = fixture(true);
    let config = TempDir::new().unwrap();
    let before = tree(dir.path());
    let output = run_epr(
        dir.path(),
        config.path(),
        &["flow", "context", ".", "--participate", "--json"],
    );
    assert_eq!(output.status.code(), Some(0));
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["outcome"], "candidates");
    // The capacity answer came from the device's own read-only berth.
    assert_eq!(value["capacity"]["verdict"], "admit");
    let text = run_epr(
        dir.path(),
        config.path(),
        &["flow", "context", "plans", "--participate"],
    );
    assert_eq!(text.status.code(), Some(0));
    assert_eq!(tree(dir.path()), before, "the read wrote nothing");
}

// ── P14: one caller; nothing outside the scope moves a count; unreadable is unavailable ──

#[test]
fn records_outside_the_directory_move_nothing_printed() {
    let inside = fixture(false);
    let both = fixture(true);
    let a = participate::participate_with(inside.path(), "plans", LIMIT, &probes(inside.path()))
        .unwrap();
    let b =
        participate::participate_with(both.path(), "plans", LIMIT, &probes(both.path())).unwrap();
    assert_eq!(a.render_text(), b.render_text());
    assert_eq!(as_json(&a), as_json(&b));
    assert!(!b.render_text().contains("other__far"));
    assert!(!b.render_text().contains("far-red"));
}

#[test]
fn an_unreadable_source_is_unavailable_never_an_error_exit() {
    let dir = fixture(false);
    let log = dir.path().join(".eprfs/status/flows.jsonl");
    let text = std::fs::read_to_string(&log).unwrap();
    // Tamper with the first line's stored CID: an integrity error for every flow-backed source.
    let tampered = text.replacen("\"cid\":\"b", "\"cid\":\"x", 1);
    assert_ne!(tampered, text);
    std::fs::write(&log, tampered).unwrap();
    let config = TempDir::new().unwrap();
    let output = run_epr(
        dir.path(),
        config.path(),
        &["flow", "context", ".", "--participate", "--json"],
    );
    assert_eq!(output.status.code(), Some(0));
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    let stations = value["sources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["source"] == "stations")
        .unwrap();
    assert_eq!(stations["status"], "unavailable");
    assert!(stations["reason"]
        .as_str()
        .unwrap()
        .contains("could not be decoded"));
    // The habit reader is a separate source and still answers.
    assert!(group(&value, "habits")
        .iter()
        .any(|h| h["ref"] == "plans-red"));
}

// ── P6: each source's own meaning of open ───────────────────────────────────────────────────

#[test]
fn each_source_keeps_its_own_meaning_of_open() {
    let dir = fixture(false);
    let result =
        participate::participate_with(dir.path(), ".", LIMIT, &probes(dir.path())).unwrap();
    let json = as_json(&result);

    let stations = group(&json, "stations");
    assert_eq!(stations.len(), 1, "only the unticked station is open");
    assert!(stations[0]["state"]
        .as_str()
        .unwrap()
        .starts_with("open — unticked intent"));
    let excluded = json["excluded"].as_array().unwrap();
    assert!(excluded.iter().any(|e| e["source"] == "stations"
        && e["count"] == 1
        && e["reason"]
            .as_str()
            .unwrap()
            .starts_with("ticked: claimed, not fulfilled")));

    // The ticked station's claim commitment is undischarged and owned: shown with its owner.
    let commitments = group(&json, "commitments");
    let claim = commitments
        .iter()
        .find(|c| c["state"] == "undischarged (active); owned by tool:decompose-claim")
        .expect("the owned commitment is shown, never hidden");
    assert!(claim["serves"]
        .as_array()
        .unwrap()
        .iter()
        .any(|s| s == "plans__epic#2"));
    assert!(excluded
        .iter()
        .any(|e| e["source"] == "commitments" && e["reason"] == "discharged by a recorded event"));

    let landing = group(&json, "landing intents");
    assert_eq!(landing.len(), 1);
    assert!(landing[0]["state"].as_str().unwrap().starts_with("live"));
    assert_eq!(landing[0]["worth"][0]["unit"], "gate seconds");
    assert_eq!(landing[0]["worth"][0]["value"], "42");

    let notes = group(&json, "notes");
    let ruling = notes
        .iter()
        .find(|n| n["state"] == "testimony (run:ruling) — not claimable")
        .unwrap();
    assert_eq!(ruling["taking"], "nothing: testimony is not claimable");
    // A one-member role set stays a set; an unresolved one is unknown with its reason.
    assert_eq!(
        ruling["roles"]["stewards"],
        serde_json::json!(["human:matthew"])
    );
    assert!(ruling["roles"]["gifters"]["unknown"].is_string());
    let text = result.render_text();
    assert!(
        text.contains("For whom: stewards [human:matthew]; gifters unknown ("),
        "{text}"
    );
    assert!(text.contains("  Person: ready for unknown — reachability undeclared\n"));

    let habits = group(&json, "habits");
    assert!(habits.iter().any(|h| h["ref"] == "plans-red"));
    assert!(habits.iter().any(|h| h["ref"] == "far-red"));
    assert!(!habits.iter().any(|h| h["ref"] == "plans-green"));
    assert!(excluded
        .iter()
        .any(|e| e["source"] == "habits" && e["reason"] == "green"));

    // Landing intents are scoped to the repository: a subdirectory holds none.
    let plans =
        participate::participate_with(dir.path(), "plans", LIMIT, &probes(dir.path())).unwrap();
    assert!(group(&as_json(&plans), "landing intents").is_empty());
}

// ── P7 / P8: a bound is never worth; source order; no score ─────────────────────────────────

fn numbers(value: &Value, found: &mut Vec<String>, at: &str) {
    match value {
        Value::Number(n) => found.push(format!("{at}={n}")),
        Value::Array(items) => {
            for (i, item) in items.iter().enumerate() {
                numbers(item, found, &format!("{at}[{i}]"));
            }
        }
        Value::Object(map) => {
            for (key, item) in map {
                numbers(item, found, &format!("{at}.{key}"));
            }
        }
        _ => {}
    }
}

#[test]
fn a_bound_is_never_worth_order_is_source_order_and_there_is_no_score() {
    let dir = fixture(false);
    let result =
        participate::participate_with(dir.path(), "plans", LIMIT, &probes(dir.path())).unwrap();
    let json = as_json(&result);
    let commitments = group(&json, "commitments");
    let order: Vec<&str> = commitments
        .iter()
        .map(|c| c["serves"][1].as_str().unwrap())
        .collect();
    assert_eq!(
        order,
        ["plans__epic#2", "plans__epic#3"],
        "append order kept"
    );
    let bounded = &commitments[1];
    assert!(bounded["worth"]["unknown"].is_string(), "{bounded:#}");
    let item_text = serde_json::to_string(bounded).unwrap();
    assert!(!item_text.contains("987654"), "{item_text}");
    assert!(!item_text.contains("widgets"), "{item_text}");
    let text = result.render_text();
    assert!(
        !text.contains("987654") && !text.contains("widgets"),
        "{text}"
    );

    let mut found = Vec::new();
    numbers(&json["groups"], &mut found, "groups");
    assert!(found.is_empty(), "items carry no numeric field: {found:?}");
    let raw = serde_json::to_string(&json).unwrap();
    for word in ["score", "rank", "merit", "priority"] {
        assert!(!raw.contains(&format!("\"{word}\"")), "{word} in {raw}");
    }
}

// ── P16: exactly the keys it lists ──────────────────────────────────────────────────────────

#[test]
fn json_keys_are_exactly_p16s() {
    let dir = fixture(true);
    let result =
        participate::participate_with(dir.path(), ".", LIMIT, &probes(dir.path())).unwrap();
    let json = as_json(&result);
    let keys = |value: &Value| {
        let mut keys: Vec<String> = value.as_object().unwrap().keys().cloned().collect();
        keys.sort();
        keys
    };
    let mut top = vec![
        "scope",
        "completeness",
        "sources",
        "capacity",
        "grant",
        "covenant",
        "groups",
        "excluded",
        "outcome",
    ];
    top.sort_unstable();
    assert_eq!(keys(&json), top);
    let mut item_keys = vec![
        "ref",
        "state",
        "serves",
        "roles",
        "standing",
        "worth",
        "testimony",
        "taking",
        "fit",
        "person",
        "agent",
    ];
    item_keys.sort_unstable();
    let mut seen = 0;
    for group in json["groups"].as_array().unwrap() {
        assert_eq!(keys(group), ["items", "source"]);
        for item in group["items"].as_array().unwrap() {
            assert_eq!(keys(item), item_keys);
            seen += 1;
        }
    }
    assert!(seen > 0);
    // An unknown carries its reason.
    assert!(json["covenant"]["unknown"] == "no non-initialising reader");
    assert!(json["completeness"]["unknown"].is_string());
    assert!(json["grant"]["unknown"].is_string());
}

// ── the plain read is untouched ─────────────────────────────────────────────────────────────

#[test]
fn plain_context_is_unchanged_and_participate_refuses_its_windows() {
    let dir = fixture(false);
    let config = TempDir::new().unwrap();
    let plain = run_epr(
        dir.path(),
        config.path(),
        &["flow", "context", "plans/epic.md", "--json"],
    );
    assert_eq!(plain.status.code(), Some(0));
    let value: Value = serde_json::from_slice(&plain.stdout).unwrap();
    let mut keys: Vec<&str> = value
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    let mut expected = vec![
        "identity",
        "repository_agent",
        "repository_identity_issue",
        "reconciliation",
        "actionable",
        "habit_read_issue",
        "intents",
        "commitments",
        "notes",
        "seals",
        "habits",
        "gate",
        "habit_scope",
        "governance",
        "scope_note",
    ];
    expected.sort_unstable();
    assert_eq!(keys, expected);
    let text = run_epr(
        dir.path(),
        config.path(),
        &["flow", "context", "plans/epic.md"],
    );
    let stdout = String::from_utf8_lossy(&text.stdout);
    assert!(
        stdout.starts_with("epr flow context — plans/epic.md\n"),
        "{stdout}"
    );
    assert!(!stdout.contains("Participation:"));

    let refused = run_epr(
        dir.path(),
        config.path(),
        &["flow", "context", ".", "--participate", "--notes", "3"],
    );
    assert!(
        !refused.status.success(),
        "a usage error is the one non-zero exit"
    );
    let not_a_dir = run_epr(
        dir.path(),
        config.path(),
        &["flow", "context", "plans/epic.md", "--participate"],
    );
    assert!(!not_a_dir.status.success());
}

#[test]
fn the_item_cap_keeps_source_order_and_counts_only_what_is_seen() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(
        root,
        ".epr-meta/repository.yaml",
        "version: 1\nagent: repo:fixture/participate\n",
    );
    write(root, "plans/epic.md", "# Epic\n");
    let epic = body_cid_of_file(&root.join("plans/epic.md")).unwrap();
    let mut store = SidecarFlowStore::open(root).unwrap();
    for i in 0..45 {
        store
            .append(station(epic, "gap:open", &format!("plans__epic#{i}")))
            .unwrap();
    }
    for i in 0..3 {
        store
            .append(commitment(
                epic,
                &format!("plans__epic#c{i}"),
                "agent:o@m",
                None,
            ))
            .unwrap();
    }
    drop(store);
    write(
        root,
        ".eprfs/status/labels.json",
        &serde_json::json!({ epic.to_string(): "plans/epic.md" }).to_string(),
    );
    // Per source: stations cannot crowd out the commitments behind them.
    let result = participate::participate_with(root, "plans", LIMIT, &probes(root)).unwrap();
    let json = as_json(&result);
    let stations = group(&json, "stations");
    assert_eq!(stations.len(), 8);
    assert_eq!(stations[0]["ref"], "plans__epic#0");
    assert_eq!(stations[7]["ref"], "plans__epic#7");
    assert_eq!(group(&json, "commitments").len(), 3);
    let text = result.render_text();
    assert!(text.contains("  … 37 more in stations\n"), "{text}");
    assert!(!text.contains("more in commitments"), "{text}");
    let wide = participate::participate_with(root, "plans", 40, &probes(root)).unwrap();
    let wide_json = as_json(&wide);
    assert_eq!(group(&wide_json, "stations").len(), 40);
    assert!(wide.render_text().contains("  … 5 more in stations\n"));
    for bad in [0, participate::MAX_LIMIT + 1] {
        assert!(participate::participate_with(root, "plans", bad, &probes(root)).is_err());
    }
    let config = TempDir::new().unwrap();
    let cli = run_epr(
        root,
        config.path(),
        &[
            "flow",
            "context",
            "plans",
            "--participate",
            "--limit",
            "2",
            "--json",
        ],
    );
    assert_eq!(cli.status.code(), Some(0));
    let cli_json: Value = serde_json::from_slice(&cli.stdout).unwrap();
    assert_eq!(group(&cli_json, "stations").len(), 2);
    let refused = run_epr(
        root,
        config.path(),
        &[
            "flow",
            "context",
            "plans",
            "--participate",
            "--limit",
            "201",
        ],
    );
    assert!(!refused.status.success());
}

// ── presence: `berth status --json` ─────────────────────────────────────────────────────────

#[test]
fn presence_lists_live_sessions_declaring_writes_in_scope_first() {
    let dir = fixture(false);
    let result =
        participate::participate_with(dir.path(), "plans", LIMIT, &probes(dir.path())).unwrap();
    let json = as_json(&result);
    assert_eq!(
        json["groups"][0]["source"], "presence",
        "presence is listed first"
    );
    let presence = group(&json, "presence");
    let refs: Vec<&str> = presence
        .iter()
        .map(|i| i["ref"].as_str().unwrap())
        .collect();
    assert_eq!(
        refs,
        [
            "session aaaaaaaa",
            "session cccccccc on did:key:z6MkSibl…abcd"
        ],
        "the far-lane session declares nothing under plans/"
    );
    let local = &presence[0];
    assert!(local["state"]
        .as_str()
        .unwrap()
        .contains("task: plans lane; declared in scope: plans/epic.md; seen 12s ago"));
    assert_eq!(local["agent"], "running (seen 12s ago)");
    assert_eq!(
        local["serves"]["notApplicable"],
        "not applicable (presence is a signal, not claimable work)"
    );
    assert_eq!(
        local["worth"]["notApplicable"],
        local["serves"]["notApplicable"]
    );
    assert_eq!(local["taking"], local["serves"]["notApplicable"]);
    // A session is never a person's presence.
    assert_eq!(
        local["person"]["readyFor"]["unknown"],
        "reachability undeclared"
    );
    assert_eq!(presence[1]["agent"], "running (seen 5m ago)");

    // Every other item names the sessions running, never a person.
    let station = &group(&json, "stations")[0];
    assert_eq!(
        station["agent"],
        "running: 2 session(s) on this device, 1 on siblings"
    );
    let sources = json["sources"].as_array().unwrap();
    for name in ["moorings", "sibling moorings"] {
        let source = sources.iter().find(|s| s["source"] == name).unwrap();
        assert_eq!(source["status"], "available", "{source:#}");
    }
    let text = result.render_text();
    assert!(
        text.contains("\n  device did:key:z6MkgHju…xMbu: silent; has acknowledged none of this device's records\n"),
        "{text}"
    );
    assert!(
        !text.contains("device did:key:z6MkSibl…abcd:"),
        "an exchanging device is not named"
    );
    assert!(text.contains("  Serves: not applicable (presence is a signal, not claimable work)\n"));
    assert!(text.contains("  Agent: running (seen 12s ago)\n"));
    let person_lines = text.lines().filter(|l| l.starts_with("  Person:")).count();
    let ready = text
        .lines()
        .filter(|l| *l == "  Person: ready for unknown — reachability undeclared")
        .count();
    assert_eq!(person_lines, ready);
}

#[test]
fn a_berth_without_the_contract_leaves_presence_unavailable_and_the_read_answers() {
    let fakes = TempDir::new().unwrap();
    let cases = [
        (
            "garbage",
            "printf 'BERTH (this workspace as one blade)\\n'".to_string(),
            "printed no answer in its contract",
        ),
        (
            "older",
            "echo 'berth: unknown flag --json' >&2; exit 2".to_string(),
            "exited 2: berth: unknown flag --json",
        ),
        (
            "slow",
            "exec sleep 10".to_string(),
            "did not answer within 5 s",
        ),
    ];
    let dir = fixture(false);
    for (name, body, reason) in cases {
        let berth = fakes.path().join(name);
        fake_berth(&berth, &body);
        let probes = Probes {
            berth: Some(berth),
            device_key: Some(dir.path().join("no-device-key")),
        };
        let started = std::time::Instant::now();
        let result = participate::participate_with(dir.path(), "plans", LIMIT, &probes).unwrap();
        assert!(
            started.elapsed() < std::time::Duration::from_secs(9),
            "{name}: capped"
        );
        let json = as_json(&result);
        for source in ["moorings", "sibling moorings"] {
            let view = json["sources"]
                .as_array()
                .unwrap()
                .iter()
                .find(|s| s["source"] == source)
                .unwrap();
            assert_eq!(view["status"], "unavailable", "{name}");
            assert!(
                view["reason"].as_str().unwrap().contains(reason),
                "{name}: {view:#}"
            );
        }
        assert!(
            !json["groups"]
                .as_array()
                .unwrap()
                .iter()
                .any(|g| g["source"] == "presence"),
            "{name}"
        );
        let station = &group(&json, "stations")[0];
        assert!(
            station["agent"]["unknown"]
                .as_str()
                .unwrap()
                .contains(reason),
            "{name}"
        );
        // Capacity still answers from the same berth.
        assert_eq!(json["capacity"]["verdict"], "admit", "{name}");
    }
}

/// P15 reaches the tools the read spawns: a berth that does not declare the read-only presence
/// contract builds its store on any `status` call, so it is read as text and never run for it.
#[test]
fn a_berth_without_the_presence_contract_is_never_run_for_status() {
    let dir = fixture(false);
    let fakes = TempDir::new().unwrap();
    let berth = fakes.path().join("berth-old");
    let witness = fakes.path().join("status-ran");
    std::fs::write(
        &berth,
        format!(
            "#!/bin/sh\ncase \"$1\" in\n  room) printf '%s\\n' '{ROOM_JSON}' ;;\n  status) : > '{}' ;;\nesac\n",
            witness.display()
        ),
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&berth, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let probes = Probes {
        berth: Some(berth),
        device_key: Some(dir.path().join("no-device-key")),
    };
    let result = participate::participate_with(dir.path(), "plans", LIMIT, &probes).unwrap();
    let json = as_json(&result);
    let moorings = json["sources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["source"] == "moorings")
        .unwrap();
    assert_eq!(moorings["status"], "unavailable");
    assert!(
        moorings["reason"]
            .as_str()
            .unwrap()
            .contains("status-json@1"),
        "{moorings:#}"
    );
    assert!(!witness.exists(), "the older berth's status was run");
}
