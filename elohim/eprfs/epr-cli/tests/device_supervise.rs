//! Story: ark runs the device's declared manifest.
//!
//! A person declares a resident with `epr device run`, asks `epr device supervise` to run it,
//! and the device keeps it: a killed child is restarted with its death witnessed, a moved
//! head reloads ark under the new CID with the incarnation carried, and `--stop` leaves
//! nothing running.
//!
//! This needs a built `ark`, which lives in the `elohim` cargo workspace, not this one. It
//! runs only when `ARK_BIN` names one (for example the pool slot
//! `/projects/.cargo-target-pool/family/dev/elohim/dev/debug/ark` after `just gate ark`);
//! otherwise it says so and passes without asserting anything. The eprfs gate does not set
//! `ARK_BIN`, so this story is exercised by hand until a gate builds both workspaces.
use elohim_device_runtime::{
    bin_store::{CommandOutput, CommandSpec, IngestOptions, Refusal, Runner, Store},
    manifest_store,
};
use serde_json::{json, Value};
use std::{
    fs,
    os::unix::fs::{symlink, PermissionsExt},
    path::{Path, PathBuf},
    process::{Command, Output},
    thread,
    time::{Duration, Instant},
};

const READY: &str = "fixture-ready";

struct Fixture {
    _temp: tempfile::TempDir,
    base: PathBuf,
    repo: PathBuf,
    ark: PathBuf,
}

impl Fixture {
    fn epr(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_epr"))
            .args(args)
            .current_dir(&self.repo)
            .env("HOME", &self.base)
            .env("XDG_CONFIG_HOME", self.base.join("config"))
            .env("ELOHIM_DEVICE_KEY_FILE", self.base.join("device/seed"))
            .env("BIN_STORE", self.base.join("bins"))
            .env("ARK_BIN", &self.ark)
            .env("EPR_BERTH_BIN", "")
            .env("EPR_PROC_ROOT", self.base.join("proc"))
            .env("ELOHIM_SESSION_ID", "supervise-fixture")
            .env("CLAUDE_CONFIG_DIR", self.base.join("berth-config"))
            .env("PATH", "/usr/bin:/bin")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env_remove("EPR_SESSION_LAZY")
            .env_remove("EPR_BIN_TOOLS")
            .env_remove("EPR_STORE")
            .env_remove("BRIT_STORE")
            .env_remove("ELOHIM_RUNTIME_CONFIG_PATH")
            .env_remove("ELOHIM_RUNTIME_MANIFEST_PATH")
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .output()
            .expect("run epr")
    }

    fn ok(&self, args: &[&str]) -> String {
        let output = self.epr(args);
        assert!(
            output.status.success(),
            "epr {args:?}: {}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).into_owned()
    }

    fn status(&self) -> Value {
        serde_json::from_str(&self.ok(&["device", "supervise", "--status", "--json"])).unwrap()
    }

    fn head(&self) -> String {
        let view: Value = serde_json::from_str(&self.ok(&["device", "show"])).unwrap();
        view["manifestCid"].as_str().unwrap().to_owned()
    }

    fn supervisor_line(&self) -> Value {
        let output = self.epr(&["device", "status", "--json"]);
        let rows: Vec<Value> = serde_json::from_slice(&output.stdout).unwrap();
        rows.into_iter()
            .find(|row| row["name"] == "device supervisor")
            .expect("a device supervisor line")
    }

    fn data_root(&self) -> PathBuf {
        self.repo.join(".eprfs/status/ark/device")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        // Never leave an ark behind a failed assertion.
        let _ = self.epr(&["device", "supervise", "--stop"]);
    }
}

/// Ingest resolves a tool's version by running it; the fixture answers for it.
struct IdentityRunner;
impl Runner for IdentityRunner {
    fn run(&self, command: &CommandSpec) -> Result<CommandOutput, Refusal> {
        Ok(CommandOutput {
            success: true,
            stdout: json!({
                "cid": elohim_device_runtime::bin_store::file_cid(Path::new(&command.program))?,
                "version": "fixture"
            })
            .to_string(),
            stderr: String::new(),
        })
    }
}

fn fixture() -> Option<Fixture> {
    let Some(ark) = std::env::var_os("ARK_BIN").filter(|v| !v.is_empty()) else {
        eprintln!("device_supervise: ARK_BIN is unset; the ark story is not exercised");
        return None;
    };
    Some(fixture_with(PathBuf::from(ark)))
}

fn fixture_with(ark: PathBuf) -> Fixture {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().to_owned();
    let repo = base.join("repo");
    fs::create_dir_all(&repo).unwrap();
    assert!(Command::new("git")
        .args(["init", "-q"])
        .current_dir(&repo)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .status()
        .unwrap()
        .success());
    // A real executable, so the identity rung reads the same bytes it was declared with: a
    // `#!` script would run as its interpreter and the rung would rightly refuse it. The
    // resident is declared as the workspace base's own `epr` tool, because the bin store
    // ingests only the tools it knows how to identify; its bytes here are a copy of bash.
    let source = base.join("source");
    fs::create_dir_all(&source).unwrap();
    for bin in ["epr", "bin-launch"] {
        fs::copy("/bin/bash", source.join(bin)).unwrap();
        fs::set_permissions(source.join(bin), fs::Permissions::from_mode(0o755)).unwrap();
    }
    let registry = manifest_store::registry(&manifest_store::workspace_base().unwrap()).unwrap();
    let store = Store::new(base.join("bins/epr"), "epr", &registry)
        .unwrap()
        .with_runner(std::sync::Arc::new(IdentityRunner));
    let release = store
        .ingest(
            &source.join("epr"),
            &IngestOptions {
                source_commit: Some("a".repeat(40)),
                tree_clean: Some(true),
                gate_run: Some("fixture".into()),
                ..Default::default()
            },
        )
        .unwrap();
    symlink(&release.cid, store.root().join("current")).unwrap();
    fs::write(
        store.root().join("current.json"),
        json!({ "cid": release.cid }).to_string(),
    )
    .unwrap();
    Fixture {
        _temp: temp,
        base,
        repo,
        ark,
    }
}

fn wait_for(what: &str, mut probe: impl FnMut() -> Option<Value>) -> Value {
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        if let Some(value) = probe() {
            return value;
        }
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        thread::sleep(Duration::from_millis(200));
    }
}

fn script(tag: &str) -> String {
    format!("trap 'exit 0' INT TERM; echo {READY} {tag}; while :; do sleep 1; done")
}

fn alive(pid: u64) -> bool {
    fs::read_to_string(format!("/proc/{pid}/stat"))
        .ok()
        .and_then(|stat| {
            let rest = stat.get(stat.rfind(')')? + 2..)?.to_owned();
            rest.chars().next()
        })
        .is_some_and(|state| state != 'Z')
}

#[test]
fn ark_keeps_the_declared_resident_and_reloads_a_moved_head() {
    let Some(fx) = fixture() else { return };

    // Nothing declared yet: there is nothing to supervise, and that is not a lack.
    assert!(fx
        .ok(&["device", "supervise"])
        .contains("declares no process"));

    // The person declares a resident: offline, one act on the chain.
    let first = script("v1");
    fx.ok(&[
        "device",
        "run",
        "epr",
        "--ready-line",
        READY,
        "--restart",
        "permanent",
        "--",
        "-c",
        &first,
    ]);
    let head1 = fx.head();
    assert_eq!(fx.supervisor_line()["state"], "degraded");

    assert!(fx.ok(&["device", "supervise"]).contains("started pid"));
    let ready = wait_for("the resident to become ready", || {
        let status = fx.status();
        (status["processes"][0]["ready"] == true).then_some(status)
    });
    assert_eq!(ready["manifestCid"], head1);
    assert_eq!(ready["current"], true);
    let incarnation1 = ready["incarnation"].as_u64().unwrap();
    let child1 = ready["processes"][0]["pid"].as_u64().unwrap();
    assert_eq!(fx.supervisor_line()["state"], "ok");
    // A second supervise at the same head starts nothing.
    assert!(fx.ok(&["device", "supervise"]).contains("current manifest"));

    // Kill the resident: ark restarts it and witnesses the death.
    assert!(Command::new("kill")
        .args(["-9", &child1.to_string()])
        .status()
        .unwrap()
        .success());
    let restarted = wait_for("ark to restart the resident", || {
        let status = fx.status();
        let pid = status["processes"][0]["pid"].as_u64()?;
        (status["processes"][0]["ready"] == true && pid != child1 && status["witnesses"] != 0)
            .then_some(status)
    });
    let witnesses = Command::new(&fx.ark)
        .args(["witness", "ls", "--berth"])
        .arg(fx.data_root().join("berth.json"))
        .output()
        .unwrap();
    assert!(witnesses.status.success());
    let rows: Vec<Value> = serde_json::from_slice(&witnesses.stdout).unwrap();
    let death = rows
        .iter()
        .find(|row| row["pid"] == child1)
        .expect("a witness for the killed resident");
    assert_eq!(death["exit"]["signal"], 9);
    assert_eq!(restarted["manifestCid"], head1);

    // Supersede the declaration: the running ark is now stale until supervise reloads it.
    let second = script("v2");
    fx.ok(&[
        "device",
        "run",
        "epr",
        "--ready-line",
        READY,
        "--",
        "-c",
        &second,
    ]);
    let head2 = fx.head();
    assert_ne!(head1, head2);
    assert_eq!(fx.status()["current"], false);
    assert_eq!(fx.supervisor_line()["state"], "degraded");
    let ark1 = fx.status()["pid"].as_u64().unwrap();
    assert!(fx.ok(&["device", "supervise"]).contains("reloading"));
    let reloaded = wait_for("the reload under the new head", || {
        let status = fx.status();
        (status["manifestCid"] == head2.as_str() && status["processes"][0]["ready"] == true)
            .then_some(status)
    });
    assert_eq!(reloaded["current"], true);
    assert!(reloaded["incarnation"].as_u64().unwrap() > incarnation1);
    assert_ne!(reloaded["pid"].as_u64().unwrap(), ark1);
    assert!(
        !alive(ark1),
        "the stale ark stopped before the new one started"
    );
    assert_eq!(fx.supervisor_line()["state"], "ok");

    // Stop: nothing of the device ark is left running.
    let ark2 = reloaded["pid"].as_u64().unwrap();
    let child2 = reloaded["processes"][0]["pid"].as_u64().unwrap();
    assert!(fx
        .ok(&["device", "supervise", "--stop"])
        .contains("stopped"));
    let stopped = fx.status();
    assert_eq!(stopped["running"], false);
    assert!(!alive(ark2));
    wait_for("the resident to be gone", || {
        (!alive(child2)).then_some(Value::Null)
    });
    assert_eq!(fx.supervisor_line()["state"], "degraded");
}

/// No ark is needed to declare: the run act writes argv, the readiness ladder and the
/// restart policy offline, and refuses a stop that carries launch detail.
#[test]
fn run_declares_args_readiness_and_restart_offline() {
    let fx = fixture_with(PathBuf::from("/nonexistent/ark"));
    fx.ok(&[
        "device",
        "run",
        "epr",
        "--ready-line",
        READY,
        "--restart",
        "transient",
        "--",
        "-c",
        "echo --not-a-flag",
    ]);
    let manifest: Value =
        serde_json::from_str(&fx.ok(&["device", "show", "--render", "manifest"])).unwrap();
    let child = &manifest["processes"][0];
    assert_eq!(
        child["argv"],
        json!(["{artifact}", "-c", "echo --not-a-flag"])
    );
    assert_eq!(child["readiness"][0]["stdout_line"]["contains"], READY);
    assert!(child["readiness"][1]["executable_identity"].is_object());
    assert_eq!(child["policy"]["restart"], "transient");
    let stop_with_args = fx.epr(&["device", "run", "epr", "--stop", "--", "x"]);
    assert!(!stop_with_args.status.success());
    let bad_restart = fx.epr(&["device", "run", "epr", "--restart", "sometimes"]);
    assert!(!bad_restart.status.success());
    let follow_with_args = fx.epr(&["device", "follow", "bin:epr", "--ready-line", "x"]);
    assert!(!follow_with_args.status.success());
    // With no ark anywhere, supervise refuses in one line naming how to build one.
    let refused = fx.epr(&["device", "supervise"]);
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("ARK_BIN"));
    assert_eq!(fx.status()["running"], false);
    fx.ok(&["device", "run", "epr", "--stop"]);
    assert_eq!(
        serde_json::from_str::<Value>(&fx.ok(&["device", "show"])).unwrap()["processes"],
        json!([])
    );
}
