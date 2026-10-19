//! `epr covenant`, end to end through the built `epr` binary against a STUB `brit-build-ref`.
//!
//! The stub is a small script written into a tempdir that stores clauses as JSON files and
//! implements `covenant put|list|show` per the brit contract; `BRIT_BUILD_REF_BIN` selects it on the
//! CHILD process only. Standing is the same read `epr actor current` makes: the session's human
//! claim. The collective and its Stewards are the REAL ones (`human:matthew` stands, `human:adam`
//! is a fixture Steward), so the steward predicate under test is the live one.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use elohim_epr_cli::actor;
use elohim_epr_cli::device_key::DEVICE_KEY_ENV;
use serde_json::{json, Value};
use tempfile::TempDir;

const SESSION: &str = "covenant-session";
const MATTHEW: &str = "human:matthew";
const ADAM: &str = "human:adam";
const RULE: &str = "story-review";
const MANIFEST: &str = "area/.epr-meta";
const FLOOR: &str = "one considered fresh read";

const STUB: &str = r#"#!/usr/bin/env python3
import datetime, hashlib, json, os, sys

args = sys.argv[1:]
if args and args[0] == "--repo":
    args = args[2:]
assert args[0] == "covenant", args
verb, rest = args[1], args[2:]
opts, k = {}, 0
while k < len(rest):
    if rest[k] == "--output-json":
        opts["output-json"] = True
        k += 1
    else:
        opts[rest[k][2:]] = rest[k + 1]
        k += 2
store = os.environ["COVENANT_STUB_STORE"]
os.makedirs(store, exist_ok=True)

def path(session):
    return os.path.join(store, session + ".json")

def load(session):
    try:
        return json.load(open(path(session)))
    except FileNotFoundError:
        return []

if verb == "put":
    rows = load(opts["session"])
    now = datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%S.%f+00:00")
    clause = {
        "kind": opts["kind"], "session": opts["session"], "party": opts["party"],
        "quote": opts["quote"], "payload": json.loads(opts.get("payload-json", "{}")),
        "validFrom": now, "validUntil": opts.get("valid-until"), "supersedes": None,
        "agentId": "00", "signature": "00",
    }
    cid = "bafy" + hashlib.sha256((json.dumps(clause, sort_keys=True) + str(len(rows))).encode()).hexdigest()[:40]
    rows.append({"cid": cid, "clause": clause})
    json.dump(rows, open(path(opts["session"]), "w"))
    print(json.dumps({"cid": cid, "clause": clause}) if opts.get("output-json") else cid)
elif verb == "list":
    print(json.dumps(load(opts["session"])))
elif verb == "show":
    for name in os.listdir(store):
        for row in json.load(open(os.path.join(store, name))):
            if row["cid"] == opts["cid"]:
                print(json.dumps(row))
                sys.exit(0)
    sys.exit(1)
else:
    sys.exit(2)
"#;

const MANIFEST_TEXT: &str = "---
epr-meta-version: 1
id: area-governance
covers: subtree
rules:
  - id: story-review
    class: inject
    when: { write: \"*.feature\" }
    parameters:
      review-profile: story
      variance:
        floor: one considered fresh read
        requires: steward-of-record
        offered-when: the session's declared purpose is authoring these stories
  - id: plain-inject
    class: inject
    when: { write: \"*.md\" }
    parameters:
      review-profile: plain
  - id: ask-rule
    class: ask
    when: { write: \"*.sql\" }
    parameters:
      variance:
        floor: ask once
---
";

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

fn write(root: &Path, rel: &str, text: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

struct Fixture {
    repo: TempDir,
    tools: TempDir,
}

impl Fixture {
    fn new() -> Self {
        let repo = TempDir::new().unwrap();
        let root = repo.path();
        write(
            root,
            ".epr-meta/repository.yaml",
            "version: 1\nagent: repo:ethosengine/elohim\n",
        );
        write(
            root,
            ".epr-meta/collective.json",
            include_str!("../../../../.epr-meta/collective.json"),
        );
        write(
            root,
            ".eprfs/status/affiliations.jsonl",
            include_str!("../../../../.eprfs/status/affiliations.jsonl"),
        );
        write(root, MANIFEST, MANIFEST_TEXT);
        write(root, "area/features/a.feature", "Feature: a\n");
        git(root, &["init", "-q"]);
        git(root, &["add", "-A"]);
        git(root, &["commit", "-q", "-m", "fixture"]);

        let tools = TempDir::new().unwrap();
        let stub = tools.path().join("brit-build-ref");
        std::fs::write(&stub, STUB).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        Self { repo, tools }
    }

    fn root(&self) -> &Path {
        self.repo.path()
    }

    fn stand(&self, human: &str) {
        actor::claim(self.root(), human, SESSION).expect("claim");
    }

    fn cmd(&self, verb: &str, extra: &[&str]) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_epr"));
        command
            .arg("covenant")
            .arg(verb)
            .args(["--repo", self.root().to_str().unwrap()])
            .args(["--session", SESSION])
            .args(extra)
            .arg("--json")
            .env(
                "BRIT_BUILD_REF_BIN",
                self.tools.path().join("brit-build-ref"),
            )
            .env("COVENANT_STUB_STORE", self.tools.path().join("store"))
            .env(
                DEVICE_KEY_ENV,
                self.tools.path().join("no-device/ed25519.seed"),
            )
            .env_remove("CLAUDE_CODE_SESSION_ID")
            .env_remove("CLAUDE_SESSION_ID")
            .env_remove("ELOHIM_SESSION_ID");
        command
    }

    fn run(&self, verb: &str, extra: &[&str]) -> Output {
        self.cmd(verb, extra).output().expect("epr runs")
    }

    fn json(&self, verb: &str, extra: &[&str]) -> Value {
        let out = self.run(verb, extra);
        assert!(
            out.status.success(),
            "epr covenant {verb} {extra:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        serde_json::from_slice(&out.stdout).expect("one JSON object")
    }

    fn refusal(&self, verb: &str, extra: &[&str]) -> String {
        let out = self.run(verb, extra);
        assert!(
            !out.status.success(),
            "expected a refusal for {extra:?}, got {}",
            String::from_utf8_lossy(&out.stdout)
        );
        assert!(out.stdout.is_empty(), "a refusal prints nothing on stdout");
        String::from_utf8_lossy(&out.stderr).to_string()
    }

    fn stored(&self) -> usize {
        let path = self
            .tools
            .path()
            .join("store")
            .join(format!("{SESSION}.json"));
        std::fs::read_to_string(path)
            .map(|t| serde_json::from_str::<Vec<Value>>(&t).unwrap().len())
            .unwrap_or(0)
    }

    fn affirm(&self) -> String {
        self.record(&[
            "--kind",
            "affirmsStewardOfRecord",
            "--party",
            MATTHEW,
            "--quote",
            "yes, I stand",
        ])
    }

    fn record(&self, extra: &[&str]) -> String {
        self.json("record", extra)["cid"]
            .as_str()
            .unwrap()
            .to_string()
    }

    fn vary(&self, extra: &[&str]) -> String {
        let mut args = vec![
            "--kind",
            "variesRule",
            "--party",
            MATTHEW,
            "--quote",
            "authoring the stories is the work",
            "--rule",
            RULE,
            "--paths",
            "area/features/**/*.feature",
        ];
        args.extend(extra);
        self.record(&args)
    }

    fn check(&self, path: &str) -> Value {
        self.json("check", &["--rule", RULE, "--path", path])
    }
}

// ── record: refusals ────────────────────────────────────────────────────────────────────────────

#[test]
fn record_refuses_a_party_with_no_standing() {
    let f = Fixture::new();
    // Nobody claimed this session and the device is unwitnessed.
    let why = f.refusal(
        "record",
        &[
            "--kind",
            "affirmsStewardOfRecord",
            "--party",
            MATTHEW,
            "--quote",
            "yes",
        ],
    );
    assert!(why.contains("no standing"), "{why}");
    // A claim by a different human does not stand for matthew.
    f.stand(ADAM);
    let why = f.refusal(
        "record",
        &[
            "--kind",
            "affirmsStewardOfRecord",
            "--party",
            MATTHEW,
            "--quote",
            "yes",
        ],
    );
    assert!(why.contains("no standing"), "{why}");
    // An agent is never a party.
    let why = f.refusal(
        "record",
        &[
            "--kind",
            "affirmsStewardOfRecord",
            "--party",
            "agent:x@y",
            "--quote",
            "yes",
        ],
    );
    assert!(why.contains("human"), "{why}");
    assert_eq!(f.stored(), 0, "a refusal writes nothing");
}

#[test]
fn record_refuses_an_affirmation_from_a_fixture_steward() {
    let f = Fixture::new();
    f.stand(ADAM);
    let why = f.refusal(
        "record",
        &[
            "--kind",
            "affirmsStewardOfRecord",
            "--party",
            ADAM,
            "--quote",
            "yes",
        ],
    );
    assert!(why.contains("FIXTURE"), "{why}");
    assert_eq!(f.stored(), 0);
}

#[test]
fn record_refuses_anything_before_an_affirmation() {
    let f = Fixture::new();
    f.stand(MATTHEW);
    let why = f.refusal(
        "record",
        &[
            "--kind",
            "variesRule",
            "--party",
            MATTHEW,
            "--quote",
            "q",
            "--rule",
            RULE,
            "--paths",
            "area/x/*.feature",
        ],
    );
    assert!(why.contains("affirmsStewardOfRecord"), "{why}");
    let why = f.refusal(
        "record",
        &[
            "--kind",
            "declaresPurpose",
            "--party",
            MATTHEW,
            "--quote",
            "q",
            "--purpose",
            "p",
        ],
    );
    assert!(why.contains("affirmsStewardOfRecord"), "{why}");
    assert_eq!(f.stored(), 0);
}

#[test]
fn record_refuses_a_variance_the_rule_does_not_declare() {
    let f = Fixture::new();
    f.stand(MATTHEW);
    f.affirm();
    let base = [
        "--kind",
        "variesRule",
        "--party",
        MATTHEW,
        "--quote",
        "q",
        "--paths",
        "area/x/*.feature",
    ];
    let mut no_variance = base.to_vec();
    no_variance.extend(["--rule", "plain-inject"]);
    let why = f.refusal("record", &no_variance);
    assert!(why.contains("declares no parameters.variance"), "{why}");

    let mut ask = base.to_vec();
    ask.extend(["--rule", "ask-rule"]);
    let why = f.refusal("record", &ask);
    assert!(why.contains("only inject and dispatch"), "{why}");

    let mut missing = base.to_vec();
    missing.extend(["--rule", "no-such-rule"]);
    let why = f.refusal("record", &missing);
    assert!(why.contains("no .epr-meta manifest"), "{why}");
    assert_eq!(f.stored(), 1, "only the affirmation was written");
}

#[test]
fn record_refuses_paths_outside_the_manifest_directory() {
    let f = Fixture::new();
    f.stand(MATTHEW);
    f.affirm();
    for outside in [
        "other/features/*.feature",
        "*.feature",
        "area/../other/*.feature",
        "area",
    ] {
        let why = f.refusal(
            "record",
            &[
                "--kind",
                "variesRule",
                "--party",
                MATTHEW,
                "--quote",
                "q",
                "--rule",
                RULE,
                "--paths",
                outside,
            ],
        );
        assert!(why.contains("falls outside"), "{outside}: {why}");
    }
    assert_eq!(f.stored(), 1);
}

#[test]
fn record_refuses_to_revoke_another_partys_clause_or_a_stranger() {
    let f = Fixture::new();
    f.stand(MATTHEW);
    let affirmed = f.affirm();
    // Plant a clause by another party straight into the stub's store.
    let store = f.tools.path().join("store").join(format!("{SESSION}.json"));
    let mut rows: Vec<Value> =
        serde_json::from_str(&std::fs::read_to_string(&store).unwrap()).unwrap();
    rows.push(json!({"cid": "bafyother", "clause": {
        "kind": "declaresPurpose", "session": SESSION, "party": ADAM, "quote": "q",
        "payload": {"purpose": "p", "writeSet": []}, "validFrom": "2026-10-19T00:00:00.000000+00:00",
        "validUntil": null, "supersedes": null, "agentId": "00", "signature": "00"}}));
    std::fs::write(&store, serde_json::to_string(&rows).unwrap()).unwrap();

    let why = f.refusal(
        "record",
        &[
            "--kind",
            "revokesClause",
            "--party",
            MATTHEW,
            "--quote",
            "q",
            "--revokes",
            "bafyother",
        ],
    );
    assert!(why.contains("belongs to"), "{why}");
    let why = f.refusal(
        "record",
        &[
            "--kind",
            "revokesClause",
            "--party",
            MATTHEW,
            "--quote",
            "q",
            "--revokes",
            "bafynothere",
        ],
    );
    assert!(why.contains("not in this session"), "{why}");
    assert!(!affirmed.is_empty());
}

// ── record: success ─────────────────────────────────────────────────────────────────────────────

#[test]
fn record_succeeds_for_every_kind_and_builds_the_variance_payload_itself() {
    let f = Fixture::new();
    f.stand(MATTHEW);

    let affirmed = f.json(
        "record",
        &[
            "--kind",
            "affirmsStewardOfRecord",
            "--party",
            MATTHEW,
            "--quote",
            "yes, I stand",
        ],
    );
    assert_eq!(affirmed["status"], "recorded");
    assert_eq!(affirmed["clause"]["kind"], "affirmsStewardOfRecord");
    assert_eq!(affirmed["clause"]["payload"], json!({}));
    println!("record affirms: {affirmed}");

    let purpose = f.json(
        "record",
        &[
            "--kind",
            "declaresPurpose",
            "--party",
            MATTHEW,
            "--quote",
            "write the stories",
            "--purpose",
            "author the area stories",
            "--write-set",
            "area/features/**/*.feature",
            "area/notes/*.md",
        ],
    );
    assert_eq!(
        purpose["clause"]["payload"],
        json!({"purpose": "author the area stories", "writeSet": ["area/features/**/*.feature", "area/notes/*.md"]})
    );
    println!("record purpose: {purpose}");

    let varied = f.json(
        "record",
        &[
            "--kind",
            "variesRule",
            "--party",
            MATTHEW,
            "--quote",
            "the stories are the work",
            "--rule",
            RULE,
            "--paths",
            "area/features/**/*.feature",
            "--valid-until",
            "2099-01-01T00:00:00Z",
        ],
    );
    let payload = &varied["clause"]["payload"];
    assert_eq!(payload["rule"], RULE);
    assert_eq!(payload["manifestPath"], MANIFEST);
    assert_eq!(payload["floor"], FLOOR);
    assert_eq!(payload["paths"], json!(["area/features/**/*.feature"]));
    let expected = elohim_epr::cid::compute_raw_cid(MANIFEST_TEXT.as_bytes()).to_string();
    assert_eq!(
        payload["manifestCid"], expected,
        "raw-codec cid of the manifest bytes"
    );
    assert_eq!(varied["clause"]["validUntil"], "2099-01-01T00:00:00Z");
    println!("record varies: {varied}");

    let varied_cid = varied["cid"].as_str().unwrap();
    let revoked = f.json(
        "record",
        &[
            "--kind",
            "revokesClause",
            "--party",
            MATTHEW,
            "--quote",
            "never mind",
            "--revokes",
            varied_cid,
        ],
    );
    assert_eq!(revoked["clause"]["payload"], json!({"revokes": varied_cid}));
    println!("record revokes: {revoked}");
    assert_eq!(f.stored(), 4);
}

// ── check ───────────────────────────────────────────────────────────────────────────────────────

#[test]
fn check_holds_a_variance_on_its_paths_only() {
    let f = Fixture::new();
    f.stand(MATTHEW);
    f.affirm();
    let clause = f.vary(&[]);

    let held = f.check("area/features/a.feature");
    assert_eq!(held["status"], "held", "{held}");
    assert_eq!(held["variable"], true);
    assert_eq!(held["clause"], clause);
    assert_eq!(held["floor"], FLOOR);
    assert_eq!(held["party"], MATTHEW);
    println!("check held: {held}");

    let off = f.check("area/other/b.feature");
    assert_eq!(off["status"], "not-held");
    assert_eq!(off["variable"], true);
    assert_eq!(off["clause"], Value::Null);
    assert!(
        off["reason"]
            .as_str()
            .unwrap()
            .contains("outside the paths"),
        "{off}"
    );
    println!("check not-held (off path): {off}");

    // An absolute path under the repository reads the same.
    let abs = f.check(f.root().join("area/features/a.feature").to_str().unwrap());
    assert_eq!(abs["status"], "held", "{abs}");
}

#[test]
fn check_is_not_held_after_revocation() {
    let f = Fixture::new();
    f.stand(MATTHEW);
    f.affirm();
    let clause = f.vary(&[]);
    assert_eq!(f.check("area/features/a.feature")["status"], "held");
    f.record(&[
        "--kind",
        "revokesClause",
        "--party",
        MATTHEW,
        "--quote",
        "withdrawn",
        "--revokes",
        &clause,
    ]);
    let after = f.check("area/features/a.feature");
    assert_eq!(after["status"], "not-held");
    assert!(
        after["reason"].as_str().unwrap().contains("revoked"),
        "{after}"
    );
    println!("check not-held (revoked): {after}");
}

#[test]
fn check_is_not_held_after_the_rule_is_edited() {
    let f = Fixture::new();
    f.stand(MATTHEW);
    f.affirm();
    f.vary(&[]);
    assert_eq!(f.check("area/features/a.feature")["status"], "held");
    let mut edited = std::fs::read_to_string(f.root().join(MANIFEST)).unwrap();
    edited.push_str("# touched\n");
    std::fs::write(f.root().join(MANIFEST), edited).unwrap();
    let after = f.check("area/features/a.feature");
    assert_eq!(after["status"], "not-held");
    assert!(
        after["reason"]
            .as_str()
            .unwrap()
            .contains("rule edited since the clause"),
        "{after}"
    );
    println!("check not-held (edited): {after}");
}

#[test]
fn check_is_not_held_past_valid_until() {
    let f = Fixture::new();
    f.stand(MATTHEW);
    f.affirm();
    f.vary(&["--valid-until", "2020-01-01T00:00:00Z"]);
    let after = f.check("area/features/a.feature");
    assert_eq!(after["status"], "not-held");
    assert!(
        after["reason"].as_str().unwrap().contains("expired"),
        "{after}"
    );
}

#[test]
fn check_is_not_held_once_the_party_loses_standing_or_the_affirmation_is_revoked() {
    let f = Fixture::new();
    f.stand(MATTHEW);
    let affirmation = f.affirm();
    f.vary(&[]);
    f.record(&[
        "--kind",
        "revokesClause",
        "--party",
        MATTHEW,
        "--quote",
        "no longer",
        "--revokes",
        &affirmation,
    ]);
    let after = f.check("area/features/a.feature");
    assert_eq!(after["status"], "not-held");
    assert!(
        after["reason"].as_str().unwrap().contains("affirmation"),
        "{after}"
    );

    // A fresh session claimed by someone else cannot read the clause as honoured either.
    let g = Fixture::new();
    g.stand(MATTHEW);
    g.affirm();
    g.vary(&[]);
    g.stand(ADAM);
    let after = g.check("area/features/a.feature");
    assert_eq!(after["status"], "not-held", "{after}");
}

#[test]
fn check_reports_variable_false_for_a_rule_without_a_variance() {
    let f = Fixture::new();
    f.stand(MATTHEW);
    f.affirm();
    let plain = f.json(
        "check",
        &["--rule", "plain-inject", "--path", "area/notes/a.md"],
    );
    assert_eq!(plain["status"], "not-held");
    assert_eq!(plain["variable"], false);
    assert_eq!(plain["clause"], Value::Null);
    assert!(plain["reason"]
        .as_str()
        .unwrap()
        .contains("declares no parameters.variance"));
    println!("check variable:false: {plain}");
    let unknown = f.json("check", &["--rule", "no-such-rule", "--path", "area/x"]);
    assert_eq!(unknown["variable"], false);
    // A variable rule with no clause at all is variable:true, not-held.
    let bare = Fixture::new();
    bare.stand(MATTHEW);
    let none = bare.check("area/features/a.feature");
    assert_eq!(
        (none["status"].as_str(), none["variable"].as_bool()),
        (Some("not-held"), Some(true))
    );
}

// ── offer ───────────────────────────────────────────────────────────────────────────────────────

#[test]
fn offer_empties_pending_as_clauses_are_recorded_and_lists_the_variable_rule() {
    let f = Fixture::new();
    f.stand(MATTHEW);

    let offer = f.json("offer", &[]);
    assert_eq!(offer["session"], SESSION);
    assert_eq!(offer["standing"], MATTHEW);
    let kinds: Vec<&str> = offer["pending"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["kind"].as_str().unwrap())
        .collect();
    assert_eq!(kinds, ["affirmsStewardOfRecord", "declaresPurpose"]);
    assert!(offer["pending"][0]["question"].as_str().unwrap().len() > 10);
    let variable = offer["variable"].as_array().unwrap();
    assert_eq!(variable.len(), 1, "{offer}");
    assert_eq!(variable[0]["rule"], RULE);
    assert_eq!(variable[0]["manifest"], MANIFEST);
    assert_eq!(variable[0]["scope"], json!({"write": "*.feature"}));
    assert_eq!(variable[0]["floor"], FLOOR);
    assert!(variable[0]["offeredWhen"]
        .as_str()
        .unwrap()
        .contains("authoring"));
    assert_eq!(variable[0]["requires"], "steward-of-record");
    assert_eq!(offer["held"], json!([]));
    println!("offer (fresh): {offer}");

    f.affirm();
    let offer = f.json("offer", &[]);
    assert_eq!(offer["pending"].as_array().unwrap().len(), 1);
    assert_eq!(offer["pending"][0]["kind"], "declaresPurpose");
    assert_eq!(offer["held"].as_array().unwrap().len(), 1);

    f.record(&[
        "--kind",
        "declaresPurpose",
        "--party",
        MATTHEW,
        "--quote",
        "q",
        "--purpose",
        "stories",
    ]);
    let offer = f.json("offer", &[]);
    assert_eq!(offer["pending"], json!([]));
    assert_eq!(
        offer["variable"].as_array().unwrap().len(),
        1,
        "unheld rule still offered"
    );

    f.vary(&[]);
    let offer = f.json("offer", &[]);
    assert_eq!(
        offer["variable"],
        json!([]),
        "a held rule is not offered again"
    );
    assert_eq!(offer["held"].as_array().unwrap().len(), 3);
    println!("offer (all held): {offer}");
}

// ── show ────────────────────────────────────────────────────────────────────────────────────────

#[test]
fn show_carries_honoured_flags() {
    let f = Fixture::new();
    f.stand(MATTHEW);
    f.affirm();
    f.record(&[
        "--kind",
        "declaresPurpose",
        "--party",
        MATTHEW,
        "--quote",
        "q",
        "--purpose",
        "stories",
        "--write-set",
        "area/features/**/*.feature",
    ]);
    let kept = f.vary(&[]);
    let dropped = f.vary(&["--valid-until", "2020-01-01T00:00:00Z"]);

    let shown = f.json("show", &[]);
    assert_eq!(shown["steward"], MATTHEW);
    assert_eq!(
        shown["purpose"],
        json!({"purpose": "stories", "writeSet": ["area/features/**/*.feature"]})
    );
    let variances = shown["variances"].as_array().unwrap();
    assert_eq!(variances.len(), 1, "only honoured clauses: {shown}");
    assert_eq!(variances[0]["clause"], kept);
    assert_eq!(variances[0]["rule"], RULE);
    assert_eq!(variances[0]["paths"], json!(["area/features/**/*.feature"]));
    assert_eq!(variances[0]["floor"], FLOOR);
    assert_eq!(variances[0]["party"], MATTHEW);
    assert_eq!(variances[0]["validUntil"], Value::Null);

    let clauses = shown["clauses"].as_array().unwrap();
    assert_eq!(clauses.len(), 4);
    let expired = clauses.iter().find(|c| c["cid"] == dropped).unwrap();
    assert_eq!(expired["honoured"], false);
    assert!(expired["reason"].as_str().unwrap().contains("expired"));
    assert!(clauses.iter().filter(|c| c["honoured"] == true).count() == 3);
    println!("show: {shown}");
}

// ── the brit binary ─────────────────────────────────────────────────────────────────────────────

#[test]
fn every_command_exits_non_zero_when_brit_is_absent() {
    let f = Fixture::new();
    f.stand(MATTHEW);
    let missing: PathBuf = f.tools.path().join("not-brit");
    // The ambient PATH, less any directory that carries a real `brit-build-ref`.
    let ambient = std::env::var_os("PATH").unwrap_or_default();
    let without_brit = std::env::join_paths(
        std::env::split_paths(&ambient).filter(|dir| !dir.join("brit-build-ref").is_file()),
    )
    .unwrap();
    for (verb, extra) in [
        ("offer", vec![]),
        ("show", vec![]),
        (
            "check",
            vec!["--rule", RULE, "--path", "area/features/a.feature"],
        ),
        (
            "record",
            vec![
                "--kind",
                "affirmsStewardOfRecord",
                "--party",
                MATTHEW,
                "--quote",
                "yes",
            ],
        ),
    ] {
        // An explicit override that names nothing.
        let out = f
            .cmd(verb, &extra)
            .env("BRIT_BUILD_REF_BIN", &missing)
            .output()
            .unwrap();
        assert!(
            !out.status.success(),
            "{verb}: {}",
            String::from_utf8_lossy(&out.stdout)
        );
        let err = String::from_utf8_lossy(&out.stderr);
        assert_eq!(err.trim().lines().count(), 1, "{verb}: one line: {err}");
        assert!(err.contains("brit-build-ref not found"), "{verb}: {err}");
        // No override and no brit on PATH.
        let out = f
            .cmd(verb, &extra)
            .env_remove("BRIT_BUILD_REF_BIN")
            .env("PATH", &without_brit)
            .output()
            .unwrap();
        assert!(!out.status.success(), "{verb}");
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(err.contains("brit-build-ref not found"), "{verb}: {err}");
    }
    assert_eq!(f.stored(), 0);
}
