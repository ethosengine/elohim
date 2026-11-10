//! The shipped CLI and hooks share the a2o fixture, including real flow claims and closure.
fn run_fixture_stage(base: &std::path::Path, stage: &str) {
    // gate-reads: genesis/agentic/session_envelope_test.py
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let result = std::process::Command::new("python3")
        .arg(root.join("genesis/agentic/session_envelope_test.py"))
        .args(["--stage", stage, "--base"])
        .arg(base)
        .env("EPR_BIN", env!("CARGO_BIN_EXE_epr"))
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn runtime_child_resumes_without_inheriting_authority() {
    // gate-reads: genesis/agentic/session_envelope_test.py
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let result = std::process::Command::new("python3")
        .arg(root.join("genesis/agentic/session_envelope_test.py"))
        .arg("ResumeEnvelopeTest.test_runtime_child_resumes_without_inheriting_authority")
        .env("EPR_BIN", env!("CARGO_BIN_EXE_epr"))
        .output()
        .expect("native resume fixture starts");
    assert!(
        result.status.success(),
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn session_envelope_native_facets_and_hook_pin_regressions() {
    // gate-reads: genesis/agentic/session_envelope_test.py
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let result = std::process::Command::new("python3")
        .arg(root.join("genesis/agentic/session_envelope_test.py"))
        .env("EPR_BIN", env!("CARGO_BIN_EXE_epr"))
        .output()
        .expect("python fixture starts");
    assert!(
        result.status.success(),
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn native_session_views_match_the_wire_schema() {
    // gate-reads: elohim/sdk/schemas/v1/views
    // gate-reads: elohim/sdk/schemas/v1/views
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let fixture = tempfile::tempdir().unwrap();
    for stage in [
        "setup",
        "acquire",
        "joined",
        "close_claude",
        "collect",
        "resume_schema",
    ] {
        run_fixture_stage(fixture.path(), stage);
    }
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../../../sdk/schemas/v1/views/session-envelope.schema.json"
    ))
    .unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    for runtime in ["claude", "codex", "successor"] {
        let view: serde_json::Value = serde_json::from_slice(
            &std::fs::read(fixture.path().join(format!("view-{runtime}.json"))).unwrap(),
        )
        .unwrap();
        validator.validate(&view).unwrap();
        if runtime == "successor" {
            assert_eq!(view["envelope"]["generation"], 2);
            assert!(view["envelope"]["predecessor"]["recordCid"].is_string());
            assert!(view["envelope"]["predecessor"]["covenantSeal"].is_string());
            let mut malformed = view.clone();
            malformed["envelope"]["predecessor"]["recordCid"] = serde_json::json!(42);
            assert!(validator.validate(&malformed).is_err());
        }
    }
    for (verb, output) in [
        ("open", "open-claude.json"),
        ("open", "open-successor.json"),
        ("close", "close.json"),
        ("gc", "gc.json"),
    ] {
        let contract: serde_json::Value = serde_json::from_slice(
            &std::fs::read(root.join(format!(
                "elohim/sdk/schemas/v1/views/session-envelope-{verb}.schema.json"
            )))
            .unwrap(),
        )
        .unwrap();
        let validator = jsonschema::options()
            .with_resource(
                "epr:schema:view:session-envelope",
                jsonschema::Resource::from_contents(schema.clone()).unwrap(),
            )
            .build(&contract)
            .unwrap();
        let actual: serde_json::Value =
            serde_json::from_slice(&std::fs::read(fixture.path().join(output)).unwrap()).unwrap();
        validator.validate(&actual).unwrap();
    }
}

#[test]
fn gc_continues_after_one_dead_owner_record_cannot_close() {
    let fixture = tempfile::tempdir().unwrap();
    run_fixture_stage(fixture.path(), "setup");
    run_fixture_stage(fixture.path(), "acquire");

    let sessions = fixture.path().join("config-home/elohim/device/sessions");
    let failed_session = "claude-code-4101-77";
    let closed_session = "codex-4102-77";
    let failed_path = sessions.join(format!("{failed_session}.json"));
    let mut failed_record: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&failed_path).unwrap()).unwrap();
    let non_repository = fixture.path().join("not-a-repository");
    std::fs::create_dir(&non_repository).unwrap();
    failed_record["repository"] = serde_json::json!(non_repository);
    std::fs::write(&failed_path, serde_json::to_vec(&failed_record).unwrap()).unwrap();
    std::fs::remove_dir_all(fixture.path().join("proc/4101")).unwrap();
    std::fs::remove_dir_all(fixture.path().join("proc/4102")).unwrap();

    let root = fixture.path().join("repo");
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_epr"));
    command
        .args(["session", "gc", "--json"])
        .current_dir(&root)
        .env("EPR_UNDER_TEST", "1")
        .env(
            "EPR_AGENT_RUNTIMES",
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../../genesis/agentic/agent-runtimes.json"),
        )
        .env("BIN_STORE", fixture.path().join("stores"))
        .env("EPR_PROC_ROOT", fixture.path().join("proc"))
        .env("XDG_CONFIG_HOME", fixture.path().join("config-home"))
        .env("CLAUDE_PROJECT_DIR", &root)
        .env("CLAUDE_CONFIG_DIR", fixture.path().join("config"))
        .env("BERTH_DIR", fixture.path().join("berth"))
        .env("BERTH_UNDER_TEST", "1")
        .env("BRIT_BUILD_REF_BIN", fixture.path().join("brit-stub"))
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1");
    let output = command.output().unwrap();

    assert!(!output.status.success());
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(result["closed"]
        .as_array()
        .unwrap()
        .iter()
        .any(|record| record["identity"]["session"] == closed_session));
    assert!(result["failed"].as_array().unwrap().iter().any(|failure| {
        failure["session"] == failed_session
            && failure["error"]
                .as_str()
                .is_some_and(|error| !error.is_empty())
    }));
    let closed_record: serde_json::Value = serde_json::from_slice(
        &std::fs::read(sessions.join(format!("{closed_session}.json"))).unwrap(),
    )
    .unwrap();
    assert!(closed_record["closedAt"].is_string());
}

// ---------------------------------------------------------------------------
// Direct CLI paths: retire, restart successor, re-offer, holder arming. These build their
// fixture with the shared Python `setup` stage but never call `berth verify-released`: every
// envelope here has no bound berth store, so closure needs no berth facet.
// ---------------------------------------------------------------------------

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const CLAUDE_ID: &str = "claude-code-4101-77";

// The brit covenant stub the covenant tests use: clauses as JSON files per session namespace.
const COVENANT_STUB: &str = r#"#!/usr/bin/env python3
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
    return os.path.join(store, session.replace("/", "__") + ".json")
def load(session):
    try:
        return json.load(open(path(session)))
    except FileNotFoundError:
        return []
if verb == "put":
    rows = load(opts["session"])
    now = datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%S.%f+00:00")
    clause = {"kind": opts["kind"], "session": opts["session"], "party": opts["party"],
        "quote": opts["quote"], "payload": json.loads(opts.get("payload-json", "{}")),
        "validFrom": now, "validUntil": opts.get("valid-until"), "supersedes": None,
        "agentId": "00", "signature": "00"}
    cid = "bafy" + hashlib.sha256((json.dumps(clause, sort_keys=True) + str(len(rows))).encode()).hexdigest()[:40]
    rows.append({"cid": cid, "clause": clause})
    json.dump(rows, open(path(opts["session"]), "w"))
    print(json.dumps({"cid": cid, "clause": clause}) if opts.get("output-json") else cid)
elif verb == "list":
    print(json.dumps(load(opts["session"])))
elif verb == "show":
    sys.exit(1)
else:
    sys.exit(2)
"#;

struct Cli {
    base: tempfile::TempDir,
}

impl Cli {
    fn new() -> Self {
        let base = tempfile::tempdir().unwrap();
        run_fixture_stage(base.path(), "setup");
        let stub = base.path().join("brit-stub");
        std::fs::write(&stub, COVENANT_STUB).unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();
        // The collective and its stewards, as the covenant tests carry them.
        let repo = base.path().join("repo");
        std::fs::write(
            repo.join(".epr-meta/collective.json"),
            include_str!("../../../../.epr-meta/collective.json"),
        )
        .unwrap();
        std::fs::create_dir_all(repo.join(".eprfs/status")).unwrap();
        std::fs::write(
            repo.join(".eprfs/status/affiliations.jsonl"),
            include_str!("../../../../.eprfs/status/affiliations.jsonl"),
        )
        .unwrap();
        Self { base }
    }
    fn sessions(&self) -> PathBuf {
        self.base.path().join("config-home/elohim/device/sessions")
    }
    fn proc(&self, pid: u32) -> PathBuf {
        self.base.path().join(format!("proc/{pid}"))
    }
    /// Run `epr` as a child of the fake runtime process `owner`, with the fixture's isolation.
    fn epr(&self, owner: u32, args: &[&str], extra: &[(&str, &str)]) -> Output {
        let base = self.base.path();
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let mut command = Command::new("python3");
        command
            .arg(base.join("exec.py"))
            .arg(env!("CARGO_BIN_EXE_epr"))
            .args(args)
            .current_dir(base.join("repo"))
            .env_clear()
            .env("PATH", std::env::var("PATH").unwrap_or_default())
            .env("HOME", std::env::var("HOME").unwrap_or_default())
            .env("TMPDIR", std::env::var("TMPDIR").unwrap_or_default())
            .env("FIXTURE_OWNER", owner.to_string())
            .env("EPR_UNDER_TEST", "1")
            .env(
                "EPR_AGENT_RUNTIMES",
                root.join("genesis/agentic/agent-runtimes.json"),
            )
            .env("BIN_STORE", base.join("stores"))
            .env("EPR_PROC_ROOT", base.join("proc"))
            .env("XDG_CONFIG_HOME", base.join("config-home"))
            .env("CLAUDE_PROJECT_DIR", base.join("repo"))
            .env("CLAUDE_CONFIG_DIR", base.join("config"))
            .env("BERTH_DIR", base.join("berth"))
            .env("BERTH_UNDER_TEST", "1")
            .env("BRIT_BUILD_REF_BIN", base.join("brit-stub"))
            .env("COVENANT_STUB_STORE", base.join("covenant-store"))
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_AUTHOR_NAME", "Fixture")
            .env("GIT_COMMITTER_NAME", "Fixture")
            .env("GIT_AUTHOR_EMAIL", "fixture@example.test")
            .env("GIT_COMMITTER_EMAIL", "fixture@example.test");
        for (key, value) in extra {
            command.env(key, value);
        }
        command.output().unwrap()
    }
    fn ok(&self, owner: u32, args: &[&str], extra: &[(&str, &str)]) -> serde_json::Value {
        let out = self.epr(owner, args, extra);
        assert!(
            out.status.success(),
            "{args:?}: {}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        serde_json::from_slice(&out.stdout).unwrap_or(serde_json::Value::Null)
    }
    fn refused(&self, owner: u32, args: &[&str], reason: &str) {
        let out = self.epr(owner, args, &[]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            !out.status.success() && stderr.contains(reason),
            "{args:?} expected `{reason}`: {}{stderr}",
            String::from_utf8_lossy(&out.stdout)
        );
    }
    fn record(&self) -> serde_json::Value {
        serde_json::from_slice(
            &std::fs::read(self.sessions().join(format!("{CLAUDE_ID}.json"))).unwrap(),
        )
        .unwrap()
    }
    fn record_cid(&self) -> String {
        let bytes = std::fs::read(self.sessions().join(format!("{CLAUDE_ID}.json"))).unwrap();
        eprfs_core::BlobCid::compute_raw(&bytes).to_string()
    }
    /// The resumed runtime: a new owner process that exports the same session id.
    fn successor_env() -> Vec<(&'static str, &'static str)> {
        vec![("FIXTURE_OWNER", "4201"), ("CLAUDE_SESSION_ID", CLAUDE_ID)]
    }
    fn successor_process(&self) {
        let mut fields = vec!["0"; 20];
        fields[0] = "S";
        fields[19] = "88";
        std::fs::create_dir_all(self.proc(4201)).unwrap();
        std::fs::write(
            self.proc(4201).join("stat"),
            format!("4201 (claude) {}", fields.join(" ")),
        )
        .unwrap();
        std::fs::write(self.proc(4201).join("environ"), "").unwrap();
    }
    fn retire_args<'a>(
        &self,
        generation: &'a str,
        record: &'a str,
        party: &'a str,
        quote: &'a str,
    ) -> Vec<&'a str> {
        vec![
            "session",
            "retire",
            "--session",
            CLAUDE_ID,
            "--expect-generation",
            generation,
            "--expect-record",
            record,
            "--party",
            party,
            "--quote",
            quote,
            "--admit-successor",
        ]
    }
}

#[test]
fn retire_refuses_a_wrong_generation_record_party_quote_and_a_live_owner() {
    let cli = Cli::new();
    let opened = cli.ok(4101, &["session", "open", "--json"], &[]);
    assert_eq!(opened["generation"], 1);
    let cid = cli.record_cid();
    // The owner (fake pid 4101) is alive in this very namespace for every refusal below.
    let words = "the steward retires this stranded session";
    cli.refused(
        4102,
        &cli.retire_args("2", &cid, "human:matthew", words),
        "envelope-generation-mismatch",
    );
    cli.refused(
        4102,
        &cli.retire_args("1", "bafkr4iwrongrecord", "human:matthew", words),
        "envelope-record-mismatch",
    );
    cli.refused(
        4102,
        &cli.retire_args("1", &cid, "human:adam", words),
        "retire-party-refused",
    );
    cli.refused(
        4102,
        &cli.retire_args("1", &cid, "human:matthew", "   "),
        "retire-quote-blank",
    );
    cli.refused(
        4102,
        &cli.retire_args("1", &cid, "human:matthew", words),
        "envelope-owner-live",
    );
    // Nothing was written by any refusal.
    assert_eq!(cli.record_cid(), cid);
    assert!(!cli.sessions().join(format!("{CLAUDE_ID}.gen")).exists());
}

#[test]
fn retire_closes_without_a_berth_store_and_the_successor_is_generation_two_with_re_offer() {
    let cli = Cli::new();
    cli.ok(4101, &["session", "open", "--json"], &[]);
    // Generation one's steward affirms; that sealed view is what the successor is offered.
    cli.ok(
        4101,
        &[
            "covenant",
            "record",
            "--kind",
            "affirmsStewardOfRecord",
            "--party",
            "human:matthew",
            "--quote",
            "yes, I stand",
            "--json",
        ],
        &[],
    );
    let cid = cli.record_cid();
    std::fs::remove_dir_all(cli.proc(4101)).unwrap();
    let closed = cli.ok(
        4102,
        &cli.retire_args("1", &cid, "human:matthew", "the container is gone"),
        &[],
    );
    assert!(closed["closedAt"].is_string() && closed["covenantSeal"].is_string());
    assert!(closed["retirement"].is_string() && closed["releaseReceipt"].is_string());
    let directory = cli.sessions().join(format!("{CLAUDE_ID}.gen"));
    let retirement: serde_json::Value = serde_json::from_slice(
        &std::fs::read(directory.join(format!(
            "1.retirement.{}.json",
            closed["retirement"].as_str().unwrap()
        )))
        .unwrap(),
    )
    .unwrap();
    assert_eq!(retirement["party"], "human:matthew");
    assert_eq!(retirement["recordCid"], cid);
    assert_eq!(retirement["admitSuccessor"], true);
    assert_eq!(retirement["observedLiveness"], "dead");
    let receipt: serde_json::Value =
        serde_json::from_slice(&std::fs::read(directory.join("1.release.json")).unwrap()).unwrap();
    assert!(receipt["berthStore"].is_null());
    assert_eq!(receipt["verifier"], "berth verify-released");

    // The resumed runtime, a RuntimeAncestor, is admitted as generation two.
    cli.successor_process();
    let env = Cli::successor_env();
    let opened = cli.ok(4201, &["session", "open", "--json"], &env);
    assert_eq!(opened["generation"], 2);
    assert_eq!(
        opened["predecessor"]["covenantSeal"],
        closed["covenantSeal"]
    );
    let claim = cli.ok(
        4201,
        &[
            "actor",
            "claim",
            "--as",
            "human:matthew",
            "--session",
            CLAUDE_ID,
            "--json",
        ],
        &env,
    );
    assert_eq!(claim["session"], format!("{CLAUDE_ID}/generations/2"));

    // The re-offer shows generation one's held clauses as history and leaves pending pending.
    let offer = cli.ok(4201, &["covenant", "offer", "--json"], &env);
    assert_eq!(offer["predecessor"]["generation"], 1);
    assert_eq!(offer["predecessor"]["sealCid"], closed["covenantSeal"]);
    let history = offer["predecessor"]["clauses"].as_array().unwrap();
    assert!(history
        .iter()
        .any(|c| c["kind"] == "affirmsStewardOfRecord" && c["cid"].is_string()));
    let pending: Vec<_> = offer["pending"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["kind"].clone())
        .collect();
    assert!(
        pending.contains(&serde_json::json!("affirmsStewardOfRecord")),
        "{offer}"
    );
    assert!(
        pending.contains(&serde_json::json!("declaresPurpose")),
        "{offer}"
    );
    assert!(offer["held"].as_array().unwrap().is_empty());
}

#[test]
fn prepare_berth_prints_its_generation() {
    let cli = Cli::new();
    let store = cli.base.path().join("berth-store");
    std::fs::create_dir_all(&store).unwrap();
    let printed = cli.ok(
        4101,
        &[
            "session",
            "prepare-berth",
            "--store",
            store.to_str().unwrap(),
        ],
        &[],
    );
    assert_eq!(printed, serde_json::json!({"generation": 1}));
}

// Holders of THIS fixture only: parallel fixtures share the session id, so match the
// holder's own XDG_CONFIG_HOME too.
fn holder_pids(session: &str, config_home: &Path) -> Vec<u32> {
    let needle = format!("session\0hold\0--session\0{session}\0");
    let marker = format!("XDG_CONFIG_HOME={}\0", config_home.display());
    std::fs::read_dir("/proc")
        .unwrap()
        .filter_map(|e| e.ok()?.file_name().to_str()?.parse::<u32>().ok())
        .filter(|pid| {
            std::fs::read(format!("/proc/{pid}/cmdline"))
                .is_ok_and(|c| String::from_utf8_lossy(&c).contains(&needle))
                && std::fs::read(format!("/proc/{pid}/environ"))
                    .is_ok_and(|e| String::from_utf8_lossy(&e).contains(&marker))
        })
        .collect()
}

fn lock_held(lock: &Path) -> bool {
    let file = std::fs::File::open(lock).unwrap();
    rustix::fs::flock(&file, rustix::fs::FlockOperation::NonBlockingLockShared).is_err()
}

fn wait_for(what: &str, mut done: impl FnMut() -> bool) {
    for _ in 0..200 {
        if done() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    panic!("timed out waiting for {what}");
}

#[test]
fn open_arms_the_holder_and_re_arms_it_once_the_lock_is_not_held() {
    let cli = Cli::new();
    let first = cli.ok(4101, &["session", "open", "--json"], &[]);
    let holder = first["ownerHolder"].clone();
    assert_eq!(holder["generation"], 1);
    let lock = PathBuf::from(holder["lock"].as_str().unwrap());
    assert!(lock_held(&lock), "the spawned holder holds the lock");
    // Free the lock by stopping the real holder process; the owner (fake 4101) lives on.
    for pid in holder_pids(CLAUDE_ID, &cli.base.path().join("config-home")) {
        Command::new("kill").arg(pid.to_string()).status().unwrap();
    }
    wait_for("holder to release", || !lock_held(&lock));
    assert!(cli.proc(4101).exists());
    // The same owner re-opens: the unheld holder is armed again.
    let again = cli.ok(4101, &["session", "open", "--json"], &[]);
    assert_eq!(again["openedAt"], first["openedAt"]);
    assert!(again["ownerHolder"]["armedAt"].is_string());
    wait_for("re-armed holder to hold", || lock_held(&lock));
    assert_eq!(cli.record()["ownerHolder"]["lock"], holder["lock"]);
}

#[test]
fn the_real_holder_releases_when_the_owner_start_changes_or_its_proc_entry_vanishes() {
    for how in ["start-changes", "entry-vanishes"] {
        let cli = Cli::new();
        std::fs::create_dir_all(cli.sessions()).unwrap();
        let lock = cli.sessions().join(format!("{CLAUDE_ID}.gen1.owner.lock"));
        let args = [
            "session",
            "hold",
            "--session",
            CLAUDE_ID,
            "--generation",
            "1",
            "--owner-pid",
            "4101",
            "--owner-start",
            "77",
        ];
        // Run the holder directly (no exec wrapper, which would rewrite the fake proc tree).
        let base = cli.base.path();
        let mut child = Command::new(env!("CARGO_BIN_EXE_epr"))
            .args(args)
            .env_clear()
            .env("EPR_PROC_ROOT", base.join("proc"))
            .env("XDG_CONFIG_HOME", base.join("config-home"))
            .env("HOME", std::env::var("HOME").unwrap_or_default())
            .spawn()
            .unwrap();
        wait_for("holder to hold", || lock.exists() && lock_held(&lock));
        assert!(
            child.try_wait().unwrap().is_none(),
            "{how}: holder must keep holding"
        );
        if how == "start-changes" {
            let stat = cli.proc(4101).join("stat");
            let text = std::fs::read_to_string(&stat).unwrap();
            std::fs::write(&stat, text.replace(" 77", " 78")).unwrap();
        } else {
            std::fs::remove_dir_all(cli.proc(4101)).unwrap();
        }
        wait_for("holder to exit", || child.try_wait().unwrap().is_some());
        assert!(child.wait().unwrap().success(), "{how}");
        assert!(!lock_held(&lock), "{how}: the lock is released");
    }
}

#[test]
fn a_superseding_retirement_keeps_the_record_it_cites() {
    let cli = Cli::new();
    cli.ok(4101, &["session", "open", "--json"], &[]);
    let cid = cli.record_cid();
    std::fs::remove_dir_all(cli.proc(4101)).unwrap();
    let args = |cid: &str, admit: bool| {
        let mut a = cli.retire_args("1", cid, "human:matthew", "the container is gone");
        if !admit {
            a.pop();
        }
        a.into_iter().map(str::to_owned).collect::<Vec<String>>()
    };
    let run = |a: Vec<String>| {
        let refs: Vec<&str> = a.iter().map(String::as_str).collect();
        cli.ok(4102, &refs, &[])
    };
    let first = run(args(&cid, false));
    let first_cid = first["retirement"].as_str().unwrap().to_owned();
    // The record's bytes moved with the first retirement: name its current CID.
    let second = run(args(&cli.record_cid(), true));
    let second_cid = second["retirement"].as_str().unwrap().to_owned();
    assert_ne!(first_cid, second_cid);
    let directory = cli.sessions().join(format!("{CLAUDE_ID}.gen"));
    let read = |cid: &str| -> serde_json::Value {
        serde_json::from_slice(
            &std::fs::read(directory.join(format!("1.retirement.{cid}.json"))).unwrap(),
        )
        .unwrap()
    };
    assert_eq!(read(&first_cid)["admitSuccessor"], false);
    assert_eq!(read(&second_cid)["admitSuccessor"], true);
    assert_eq!(read(&second_cid)["supersedes"], first_cid);
}

#[test]
fn retire_authorizes_by_the_acting_sessions_own_generation_namespace() {
    let cli = Cli::new();
    cli.ok(4101, &["session", "open", "--json"], &[]);
    // A generation-one claim naming someone other than the steward.
    cli.ok(
        4101,
        &[
            "actor",
            "claim",
            "--as",
            "human:adam",
            "--session",
            CLAUDE_ID,
            "--json",
        ],
        &[],
    );
    // The target: another session, stranded.
    let codex_id = "codex-4102-77";
    cli.ok(4102, &["session", "open", "--json"], &[]);
    let target = std::fs::read(cli.sessions().join(format!("{codex_id}.json"))).unwrap();
    let target_cid = eprfs_core::BlobCid::compute_raw(&target).to_string();
    // Generation two of the acting session: its namespace holds no claim.
    std::fs::remove_dir_all(cli.proc(4101)).unwrap();
    cli.ok(
        4102,
        &["session", "gc", "--json"],
        &[("FIXTURE_OWNER", "4102")],
    );
    cli.successor_process();
    let env = Cli::successor_env();
    let opened = cli.ok(4201, &["session", "open", "--json"], &env);
    assert_eq!(opened["generation"], 2);
    std::fs::remove_dir_all(cli.proc(4102)).unwrap();
    // Standing is the device's human (matthew), not generation one's stale claim (adam).
    let args = [
        "session",
        "retire",
        "--session",
        codex_id,
        "--expect-generation",
        "1",
        "--expect-record",
        target_cid.as_str(),
        "--party",
        "human:matthew",
        "--quote",
        "the steward retires the stranded codex session",
        "--admit-successor",
    ];
    let closed = cli.ok(4201, &args, &env);
    assert!(closed["retirement"].is_string());
}
