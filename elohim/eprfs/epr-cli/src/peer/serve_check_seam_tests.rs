//! `peer serve --check` with its exercise supplied through the crate-private seam: the check is
//! run in a child of this test binary, re-executed with a fixture device's environment, so the
//! released `epr` and library callers have no way to skip or fake the exercise.
use super::{serve_check_with, HeadTable, Options, PeerKey};
use crate::actor::{device_endpoint, witness};
use crate::device_key::{self, DeviceKey};
use crate::exercise::{Exercised, Failure};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Output};

/// The exercise outcome the child injects (`trap`, `wrong` or `real`) and its `peer serve`
/// arguments as JSON. Read only by this test binary.
const OUTCOME_ENV: &str = "EPR_CLI_TEST_SERVE_CHECK_OUTCOME";
const ARGS_ENV: &str = "EPR_CLI_TEST_SERVE_CHECK_ARGS";
const CHILD: &str = "peer::serve_check_seam_tests::child";

/// The child half; run on its own (no child environment) it does nothing.
#[test]
fn child() {
    let Ok(outcome) = std::env::var(OUTCOME_ENV) else {
        return;
    };
    let args: Vec<String> = serde_json::from_str(&std::env::var(ARGS_ENV).unwrap()).unwrap();
    let options = Options::parse(&args).unwrap();
    let trap = || -> Result<Exercised, Failure> { std::process::abort() };
    let wrong = || {
        Err(Failure {
            step: "sqlite",
            detail: "an injected wrong answer".into(),
        })
    };
    let exercise: &dyn Fn() -> Result<Exercised, Failure> = match outcome.as_str() {
        "trap" => &trap,
        "wrong" => &wrong,
        _ => &crate::exercise::run,
    };
    let code = serve_check_with(&options, exercise).unwrap();
    std::process::exit(if code == ExitCode::SUCCESS { 0 } else { 1 });
}

/// A device: its key file, config home (its peer key lives there) and checkout.
struct Fixture {
    key_file: PathBuf,
    config: tempfile::TempDir,
    checkout: tempfile::TempDir,
}

impl Fixture {
    fn new(keys: &Path, name: &str) -> (Fixture, DeviceKey) {
        let key_file = keys.join(name).join("ed25519.seed");
        let key = DeviceKey::load_or_generate(&key_file).unwrap();
        let checkout = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(checkout.path().join(".epr-meta")).unwrap();
        std::fs::write(
            checkout.path().join(".epr-meta/repository.yaml"),
            "version: 1\nagent: repo:ethosengine/elohim\n",
        )
        .unwrap();
        for args in [
            &["init", "-q"][..],
            &["add", "-A"],
            &["commit", "-q", "-m", "fixture"],
        ] {
            let out = crate::process::build_command("git", args, checkout.path(), &[])
                .env("GIT_AUTHOR_NAME", "Fixture Author")
                .env("GIT_AUTHOR_EMAIL", "author@example.test")
                .env("GIT_COMMITTER_NAME", "Fixture Author")
                .env("GIT_COMMITTER_EMAIL", "author@example.test")
                .output()
                .unwrap();
            assert!(out.status.success(), "git {args:?}");
        }
        let fixture = Fixture {
            key_file,
            config: tempfile::tempdir().unwrap(),
            checkout,
        };
        (fixture, key)
    }

    fn peer_dir(&self) -> PathBuf {
        self.config.path().join("epr").join("peer")
    }

    /// `peer serve --check` for this device in a child of this test binary, the exercise
    /// outcome `outcome` injected. The running executable is this test binary, so it is the
    /// exact `EPR_BIN` candidate the exec floor admits.
    fn check(&self, root: &Path, policy: &Path, outcome: &str) -> Output {
        let me = std::env::current_exe().unwrap();
        let args = [
            "--check",
            "--root",
            root.to_str().unwrap(),
            "--policy",
            policy.to_str().unwrap(),
            "--json",
        ];
        Command::new(&me)
            .env(device_key::DEVICE_KEY_ENV, &self.key_file)
            .env("XDG_CONFIG_HOME", self.config.path())
            .env("EPR_STORE", self.config.path().join("epr-store"))
            .env("EPR_PAYLOAD_STORE", self.config.path().join("payload"))
            .env("EPR_BIN", &me)
            .env_remove("EPR_PEER_RELAY_URL")
            .env_remove("EPR_PEER_RELAYS")
            .env_remove("CLAUDE_CODE_SESSION_ID")
            .env_remove("CLAUDE_SESSION_ID")
            .env_remove("ELOHIM_SESSION_ID")
            .env(OUTCOME_ENV, outcome)
            .env(ARGS_ENV, serde_json::to_string(&args).unwrap())
            .args(["--exact", CHILD, "--nocapture", "--test-threads", "1", "-q"])
            .output()
            .unwrap()
    }
}

/// The pretty-printed JSON object in a child's stdout, among the test harness's own lines.
fn child_json(stdout: &[u8]) -> serde_json::Value {
    let text = String::from_utf8_lossy(stdout);
    let start = text.find("\n{\n").map(|at| at + 1).expect("a JSON object");
    let end = text[start..].find("\n}").expect("its end") + start + 2;
    serde_json::from_str(&text[start..end]).unwrap()
}

#[test]
fn serve_check_runs_its_exercise_first_and_a_returned_failure_is_only_a_note() {
    use std::os::unix::process::ExitStatusExt;
    let keys = tempfile::tempdir().unwrap();
    let (device, key) = Fixture::new(keys.path(), "a");
    let root = device.checkout.path();
    witness(
        root,
        "human:matthew",
        "agent:orchestrator@claude-opus-5-5",
        "s1",
        "operator of this stewarded device; present in this session",
        false,
        &key,
    )
    .unwrap();
    let node = PeerKey::load_or_generate(&device.peer_dir())
        .unwrap()
        .node_id()
        .to_string();
    device_endpoint(root, None, &node, &key).unwrap();
    HeadTable::default().save(&device.peer_dir()).unwrap();
    let policy = keys.path().join("pool-policy.json");
    std::fs::write(
        &policy,
        r#"{"berth": {"channel": {"crosses": {"dev-berth-events@1": ["claim", "release"]}}}}"#,
    )
    .unwrap();

    // A trap in the exercise kills the check by signal even on a device it would refuse as
    // not-a-device: the exercise runs before every other step, and death needs no handling.
    let (stranger, _) = Fixture::new(keys.path(), "off-roster");
    let trapped = stranger.check(root, &policy, "trap");
    assert_eq!(
        trapped.status.signal(),
        Some(libc::SIGABRT),
        "{:?} {}",
        trapped.status,
        String::from_utf8_lossy(&trapped.stderr)
    );
    // The same device without the trap is refused as not-a-device, exit 1 from the child.
    let refused = stranger.check(root, &policy, "real");
    assert_eq!(refused.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&refused.stderr).contains("not-a-device")
            || String::from_utf8_lossy(&refused.stdout).contains("not-a-device"),
        "{}",
        String::from_utf8_lossy(&refused.stdout)
    );

    // A returned failure is a note: the check still passes.
    let noted = device.check(root, &policy, "wrong");
    let stderr = String::from_utf8_lossy(&noted.stderr);
    assert!(noted.status.success(), "{stderr}");
    assert!(
        stderr.contains("note exercise-failed: sqlite: an injected wrong answer"),
        "{stderr}"
    );
    let result = child_json(&noted.stdout);
    assert_eq!(result["ok"], true, "{result}");
    assert!(
        result["notes"][0]
            .as_str()
            .is_some_and(|note| note.starts_with("exercise-failed: sqlite")),
        "{result}"
    );
    // The real exercise passes here and leaves no note.
    let clean = device.check(root, &policy, "real");
    assert!(
        clean.status.success(),
        "{}",
        String::from_utf8_lossy(&clean.stderr)
    );
    let result = child_json(&clean.stdout);
    assert_eq!(result["ok"], true, "{result}");
    assert!(result.get("notes").is_none(), "{result}");
}
