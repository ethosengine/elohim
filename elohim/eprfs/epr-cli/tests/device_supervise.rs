//! Story: ark runs the device's declared manifest.
//!
//! A person declares a resident with `epr device run`, asks `epr device supervise` to run it,
//! and the device keeps it: a killed child is restarted with its death witnessed, a moved
//! head reloads ark under the new CID with the incarnation carried, and `--stop` leaves
//! nothing running.
//!
//! A binary landed under an unchanged manifest reaches a running child the same way: supervise
//! sees the child's executing bytes differ from its channel's current and restarts ark. And an
//! `epr peer serve` the person started by hand keeps its serve lock until they hand it over:
//! supervise refuses beside it, and `--handover` interrupts it (SIGINT only) before ark starts
//! the declared one.
//!
//! The ark stories need a built `ark`, which lives in the `elohim` cargo workspace, not this
//! one. They run only when `ARK_BIN` names one (for example the pool slot
//! `/projects/.cargo-target-pool/family/dev/elohim/dev/debug/ark` after `just gate elohim-ark`);
//! otherwise they say so and pass without asserting anything. The eprfs gate does not set
//! `ARK_BIN`, so run it as `ARK_BIN=<ark> just gate eprfs`. The refusals before ark is reached
//! run everywhere a `python3` is on PATH (it stands in for a hand-started daemon holding the
//! serve lock).
use elohim_device_runtime::{
    bin_store::{CommandOutput, CommandSpec, IngestOptions, Refusal, Runner, Store},
    manifest_store,
};
use serde_json::{json, Value};
use std::{
    ffi::OsStr,
    fs,
    io::{BufRead, BufReader},
    os::unix::fs::{symlink, PermissionsExt},
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Output, Stdio},
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
        self.epr_with(args, &[])
    }

    fn epr_with(&self, args: &[&str], extra: &[(&str, &OsStr)]) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_epr"));
        command
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
            .env_remove("GIT_WORK_TREE");
        for (key, value) in extra {
            command.env(key, value);
        }
        command.output().expect("run epr")
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

    fn store(&self) -> Store {
        let registry =
            manifest_store::registry(&manifest_store::workspace_base().unwrap()).unwrap();
        Store::new(self.base.join("bins/epr"), "epr", &registry)
            .unwrap()
            .with_runner(std::sync::Arc::new(IdentityRunner))
    }

    /// The current `bin:epr` release and the file ark executes for it.
    fn current(&self) -> (String, PathBuf) {
        self.store().current_artifact().unwrap()
    }

    /// A daemon lease naming `pid`, as `epr peer serve` records its own.
    fn lease(&self, pid: u32) {
        let cid = self.current().0;
        assert!(self
            .store()
            .daemon_lease(&cid, pid, Path::new("/proc"))
            .unwrap()
            .is_some());
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
    let fx = Fixture {
        _temp: temp,
        base,
        repo,
        ark,
    };
    land(&fx, "source", b"");
    fx
}

/// Ingest a copy of bash (with `tail` appended, so each landing is new bytes that still run)
/// and make it the verified current, as `epr bin land` would.
fn land(fx: &Fixture, dir: &str, tail: &[u8]) -> String {
    let source = fx.base.join(dir);
    fs::create_dir_all(&source).unwrap();
    for bin in ["epr", "bin-launch"] {
        let mut bytes = fs::read("/bin/bash").unwrap();
        bytes.extend_from_slice(tail);
        fs::write(source.join(bin), bytes).unwrap();
        fs::set_permissions(source.join(bin), fs::Permissions::from_mode(0o755)).unwrap();
    }
    let store = fx.store();
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
    let link = store.root().join("current");
    let previous = fs::read_link(&link).ok();
    let _ = fs::remove_file(&link);
    symlink(&release.cid, &link).unwrap();
    let mut record = json!({ "cid": release.cid });
    if let Some(previous) = previous {
        record["previous"] = json!(previous.to_string_lossy());
    }
    fs::write(store.root().join("current.json"), record.to_string()).unwrap();
    release.cid
}

/// A declared env as the run wrote it, less the device dirs the test host's own shell carries
/// (`device run` carries HOME and the XDG homes it finds; the fixture sets HOME and
/// XDG_CONFIG_HOME, the host may set the rest).
fn declared_env(env: &Value, fx: &Fixture) -> Value {
    let mut env = env.clone();
    let map = env.as_object_mut().expect("env is an object");
    for key in ["XDG_DATA_HOME", "XDG_CACHE_HOME", "XDG_STATE_HOME"] {
        map.remove(key);
    }
    assert_eq!(map.remove("HOME"), Some(json!(fx.base)), "HOME is carried");
    assert_eq!(
        map.remove("XDG_CONFIG_HOME"),
        Some(json!(fx.base.join("config"))),
        "XDG_CONFIG_HOME is carried"
    );
    env
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

/// A named run act declares a second resident of one tool: `peer` runs `bin:epr` beside any
/// default `epr`, with its declared environment kept in the stored manifest. The printed
/// view follows the runtime-declaration schema, which carries no environment.
#[test]
fn run_declares_a_named_process_with_its_environment_offline() {
    let fx = fixture_with(PathBuf::from("/nonexistent/ark"));
    let view: Value = serde_json::from_str(&fx.ok(&[
        "device",
        "run",
        "epr",
        "--name",
        "peer",
        "--env",
        "A=1",
        "--env",
        "B=2",
        "--ready-line",
        "epr peer: serving",
        "--",
        "peer",
        "serve",
        "--root",
        "/x",
    ]))
    .unwrap();
    let process = &view["processes"][0];
    assert_eq!(process["name"], "peer");
    assert_eq!(process["channelId"], "bin:epr");
    assert_eq!(
        process["argv"],
        json!(["{artifact}", "peer", "serve", "--root", "/x"])
    );
    let manifest: Value =
        serde_json::from_str(&fx.ok(&["device", "show", "--render", "manifest"])).unwrap();
    let child = &manifest["processes"][0];
    assert_eq!(
        declared_env(&child["env"], &fx),
        json!({"A": "1", "B": "2"})
    );
    assert_eq!(
        child["readiness"][0]["stdout_line"]["contains"],
        "epr peer: serving"
    );
    // The default `epr` coexists; stopping by name removes only `peer`.
    fx.ok(&["device", "run", "epr"]);
    let names = |view: &Value| -> Vec<String> {
        view["processes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p["name"].as_str().unwrap().to_owned())
            .collect()
    };
    let shown: Value = serde_json::from_str(&fx.ok(&["device", "show"])).unwrap();
    assert_eq!(names(&shown), ["peer", "epr"]);
    let stopped: Value =
        serde_json::from_str(&fx.ok(&["device", "run", "epr", "--name", "peer", "--stop"]))
            .unwrap();
    assert_eq!(names(&stopped), ["epr"]);
    // Refusals: a secret-named key, a malformed entry, a bad name, launch flags off `run`.
    for refused in [
        &[
            "device",
            "run",
            "epr",
            "--name",
            "peer",
            "--env",
            "EPR_TOKEN=x",
        ][..],
        &["device", "run", "epr", "--env", "NOEQUALS"],
        &["device", "run", "epr", "--env", "A=1", "--env", "A=2"],
        &["device", "run", "epr", "--name", "a/b"],
        &[
            "device", "run", "epr", "--name", "peer", "--env", "A=1", "--stop",
        ],
        &["device", "follow", "bin:epr", "--name", "peer"],
        &["device", "follow", "bin:epr", "--env", "A=1"],
    ] {
        assert!(!fx.epr(refused).status.success(), "{refused:?}");
    }
}

/// `--env` splits at the first `=` only, keeps an empty value, and refuses at declaration what
/// spawn would refuse later: an unresolvable `{…}` placeholder, loader keys, a duplicate
/// `--name`, and a name already held by another tool's process.
#[test]
fn run_env_entries_keep_their_text_and_refuse_what_spawn_cannot_honour_offline() {
    let fx = fixture_with(PathBuf::from("/nonexistent/ark"));
    fx.ok(&[
        "device",
        "run",
        "epr",
        "--name",
        "peer",
        "--env",
        "A=b=c",
        "--env",
        "E=",
        "--env",
        "ROOT={data_root}/peer",
    ]);
    let manifest: Value =
        serde_json::from_str(&fx.ok(&["device", "show", "--render", "manifest"])).unwrap();
    assert_eq!(
        declared_env(&manifest["processes"][0]["env"], &fx),
        json!({"A": "b=c", "E": "", "ROOT": "{data_root}/peer"})
    );
    fx.ok(&["device", "run", "epr"]);
    for refused in [
        &["device", "run", "epr", "--name", "peer", "--env", "X={foo}"][..],
        &["device", "run", "epr", "--name", "peer", "--env", "X=a{b"],
        &[
            "device",
            "run",
            "epr",
            "--name",
            "peer",
            "--env",
            "LD_PRELOAD=/x.so",
        ],
        &[
            "device",
            "run",
            "epr",
            "--name",
            "peer",
            "--env",
            "DYLD_INSERT_LIBRARIES=/x",
        ],
        &[
            "device",
            "run",
            "epr",
            "--name",
            "peer",
            "--env",
            "GCONV_PATH=/x",
        ],
        &["device", "run", "epr", "--name", "peer", "--name", "other"],
        &["device", "run", "brit", "--name", "epr"],
        &["device", "run", "brit", "--name", "epr", "--stop"],
    ] {
        let output = fx.epr(refused);
        assert!(!output.status.success(), "{refused:?}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("{foo}") && !stderr.contains("x.so"),
            "{stderr}"
        );
    }
    // Every refusal left the declaration as it was.
    let manifest: Value =
        serde_json::from_str(&fx.ok(&["device", "show", "--render", "manifest"])).unwrap();
    assert_eq!(manifest["processes"].as_array().unwrap().len(), 2);
    assert_eq!(manifest["processes"][0]["env"]["A"], "b=c");
}

/// A hand-started `epr peer serve` stand-in: `python3` invoked through a link named `epr` as
/// `epr peer serve --root <root>`, so its command line reads as the daemon's. It takes the
/// serve lock under the fixture's `XDG_CONFIG_HOME` and exits 0 on SIGINT (1 on SIGTERM, so a
/// handover that terminated it would show).
struct Holder {
    child: Child,
}

fn python3() -> Option<PathBuf> {
    let found = std::env::var_os("PATH").and_then(|path| {
        std::env::split_paths(&path)
            .map(|dir| dir.join("python3"))
            .find(|p| p.is_file())
    });
    let Some(found) = found else {
        eprintln!("device_supervise: no python3 on PATH; the serve-lock story is not exercised");
        return None;
    };
    // A nix `python3` is a wrapper that re-executes the interpreter under the interpreter's own
    // name, so a symlink to it never shows `epr` in /proc/<pid>/cmdline (eprfs dev #160). The
    // holder links to the interpreter the wrapper ends in; it imports the standard library only.
    let real = Command::new(&found)
        .args(["-c", "import os; print(os.readlink('/proc/self/exe'))"])
        .output()
        .ok()
        .filter(|out| out.status.success())
        .map(|out| PathBuf::from(String::from_utf8_lossy(&out.stdout).trim()))
        .filter(|path| path.is_file());
    Some(real.unwrap_or(found))
}

impl Holder {
    /// `epr peer serve --root <root>`, run from its own directory.
    fn start(fx: &Fixture, root: &Path) -> Option<Holder> {
        let tail = ["--root".to_owned(), root.display().to_string()];
        Holder::start_in(fx, &fx.base.join("holder"), &tail)
    }

    /// `epr peer serve <tail…>` with working directory `dir`.
    fn start_in(fx: &Fixture, dir: &Path, tail: &[String]) -> Option<Holder> {
        let python = python3()?;
        let dir = dir.to_owned();
        fs::create_dir_all(&dir).unwrap();
        let link = dir.join("epr");
        let _ = fs::remove_file(&link);
        symlink(python, &link).unwrap();
        fs::write(
            dir.join("peer"),
            "import fcntl, os, signal, sys, time\n\
             path = os.path.join(os.environ['XDG_CONFIG_HOME'], 'epr', 'peer', 'serve.lock')\n\
             os.makedirs(os.path.dirname(path), exist_ok=True)\n\
             lock = open(path, 'a')\n\
             fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)\n\
             signal.signal(signal.SIGINT, lambda *_: sys.exit(0))\n\
             signal.signal(signal.SIGTERM, lambda *_: sys.exit(1))\n\
             print('held', flush=True)\n\
             while True:\n    time.sleep(0.1)\n",
        )
        .unwrap();
        let mut child = Command::new(&link)
            .args(["peer", "serve"])
            .args(tail)
            .current_dir(&dir)
            .env_clear()
            .env("XDG_CONFIG_HOME", fx.base.join("config"))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let mut line = String::new();
        BufReader::new(child.stdout.take().unwrap())
            .read_line(&mut line)
            .unwrap();
        assert_eq!(line.trim(), "held", "the holder took the serve lock");
        Some(Holder { child })
    }

    fn pid(&self) -> u32 {
        self.child.id()
    }

    fn running(&mut self) -> bool {
        self.child.try_wait().unwrap().is_none()
    }

    fn exit(&mut self) -> ExitStatus {
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                return status;
            }
            assert!(Instant::now() < deadline, "the holder never exited");
            thread::sleep(Duration::from_millis(50));
        }
    }
}

impl Drop for Holder {
    fn drop(&mut self) {
        if self.running() {
            let _ = self.child.kill();
        }
        let _ = self.child.wait();
    }
}

fn declare_peer(fx: &Fixture, env: &[&str]) {
    let root = fx.repo.to_str().unwrap().to_owned();
    declare_peer_with(fx, env, &["--root", &root]);
}

/// Declare `peer` as `peer serve <root_args…>`.
fn declare_peer_with(fx: &Fixture, env: &[&str], root_args: &[&str]) {
    let mut args = vec!["device", "run", "epr", "--name", "peer"];
    for entry in env {
        args.extend(["--env", entry]);
    }
    args.extend(["--ready-line", "epr peer: serving", "--", "peer", "serve"]);
    args.extend(root_args);
    fx.ok(&args);
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

const REAL_PROC: (&str, &str) = ("EPR_PROC_ROOT", "/proc");

/// With no ark anywhere: a hand-started daemon holding the serve lock refuses supervise before
/// ark is even looked for, and nothing is signalled. `--handover` refuses too while any check
/// fails, naming the check, and the daemon keeps running.
#[test]
fn a_serve_lock_held_outside_ark_refuses_and_signals_nothing_offline() {
    let fx = fixture_with(PathBuf::from("/nonexistent/ark"));
    let daemon_cure = || {
        let rows: Vec<Value> =
            serde_json::from_slice(&fx.epr(&["device", "status", "--json"]).stdout).unwrap();
        let row = rows
            .into_iter()
            .find(|r| r["name"] == "peer daemon")
            .unwrap();
        row["cure"].as_str().unwrap_or_default().to_owned()
    };
    // A silent daemon's cure is the devfile command until the head declares one, then the
    // supervisor.
    assert!(daemon_cure().contains("devfile"), "{}", daemon_cure());
    declare_peer(&fx, &[]);
    assert!(
        daemon_cure().starts_with("epr device supervise"),
        "{}",
        daemon_cure()
    );
    let Some(mut holder) = Holder::start(&fx, &fx.repo) else {
        return;
    };
    let proc = [(REAL_PROC.0, OsStr::new(REAL_PROC.1))];

    let refused = fx.epr_with(&["device", "supervise"], &proc);
    assert!(!refused.status.success());
    let text = stderr(&refused);
    assert!(text.contains("REFUSED peer-serve-unowned"), "{text}");
    assert!(text.contains(&format!("pid {}", holder.pid())), "{text}");
    assert!(text.contains("--handover"), "{text}");
    assert!(holder.running(), "an unowned refusal signals nothing");
    assert_eq!(fx.status()["running"], false, "ark was not started");

    // Every check but the lease holds: refused, naming the lease, and nothing is signalled.
    let refused = fx.epr_with(&["device", "supervise", "--handover"], &proc);
    assert!(!refused.status.success());
    let text = stderr(&refused);
    assert!(
        text.contains("REFUSED peer-serve-handover-refused") && text.contains("daemon-lease"),
        "{text}"
    );
    assert!(holder.running());
    assert_eq!(fx.status()["running"], false);
}

/// A daemon serving another root is not the one the head declares: `--handover` refuses it by
/// the root check, though its lease names it, and leaves it running.
#[test]
fn handover_refuses_a_daemon_serving_another_root_offline() {
    let fx = fixture_with(PathBuf::from("/nonexistent/ark"));
    declare_peer(&fx, &[]);
    let elsewhere = fx.base.join("elsewhere");
    fs::create_dir_all(&elsewhere).unwrap();
    let Some(mut holder) = Holder::start(&fx, &elsewhere) else {
        return;
    };
    fx.lease(holder.pid());
    let refused = fx.epr_with(
        &["device", "supervise", "--handover"],
        &[(REAL_PROC.0, OsStr::new(REAL_PROC.1))],
    );
    assert!(!refused.status.success());
    let text = stderr(&refused);
    assert!(
        text.contains("REFUSED peer-serve-handover-refused") && text.contains("same --root"),
        "{text}"
    );
    assert!(holder.running(), "a refused handover signals nothing");
    assert_eq!(fx.status()["running"], false);
}

/// The root check reads `--root <path>` and `--root=<path>` alike, and a daemon that names no
/// root is never matched by its working directory: here every holder runs from the data root,
/// the directory a declared `--root={data_root}` resolves against.
#[test]
fn handover_compares_both_root_forms_and_never_a_working_directory_offline() {
    let fx = fixture_with(PathBuf::from("/nonexistent/ark"));
    let proc = [(REAL_PROC.0, OsStr::new(REAL_PROC.1))];
    let data_root = fx.data_root();
    let repo = fx.repo.display().to_string();
    let attempt = |declared: &[&str], tail: &[String]| -> Option<String> {
        declare_peer_with(&fx, &[], declared);
        let mut holder = Holder::start_in(&fx, &data_root, tail)?;
        fx.lease(holder.pid());
        let refused = fx.epr_with(&["device", "supervise", "--handover"], &proc);
        assert!(!refused.status.success());
        assert!(holder.running(), "a refused handover signals nothing");
        Some(stderr(&refused))
    };
    // A daemon in the data root naming no root, against a declared `--root={data_root}`.
    let Some(text) = attempt(&["--root={data_root}"], &[]) else {
        return;
    };
    assert!(
        text.contains("peer-serve-handover-refused") && text.contains("names no --root"),
        "{text}"
    );
    // Declared `--root={data_root}/peer`, a daemon in the data root serving another root.
    let text = attempt(
        &["--root={data_root}/peer"],
        &["--root".into(), fx.base.display().to_string()],
    )
    .unwrap();
    assert!(text.contains("same --root"), "{text}");
    // A declared child naming no root has nothing to compare: refused, not matched by cwd.
    let text = attempt(&[], &["--root".into(), repo.clone()]).unwrap();
    assert!(text.contains("names no --root to compare"), "{text}");
    // Both spellings of the same root pass the root check; the lease check (passed here) and
    // only the start-tick reconfirmation stand after it, so the handover proceeds to ark.
    for (declared, tail) in [
        (
            format!("--root={repo}"),
            vec!["--root".to_owned(), repo.clone()],
        ),
        ("--root".to_owned(), vec![format!("--root={repo}")]),
    ] {
        let declared: Vec<&str> = if declared == "--root" {
            vec!["--root", &repo]
        } else {
            vec![declared.as_str()]
        };
        let text = attempt(&declared, &tail).unwrap();
        assert!(!text.contains("handover-refused"), "{text}");
        assert!(text.contains("ARK_BIN"), "only ark is missing: {text}");
    }
}

/// The handover: SIGINT to the one confirmed daemon, its lock freed, then ark starts the
/// declared `peer` with its declared environment — `XDG_CONFIG_HOME` (where the child writes
/// what it saw) and `EPR_BIN_TOOLS` as a `{data_root}` template, with no working-directory
/// field: ark's working directory is the data root.
#[test]
fn handover_interrupts_the_unowned_daemon_and_ark_starts_the_declared_one() {
    let Some(fx) = fixture() else { return };
    let config = fx.base.join("config");
    let xdg = format!("XDG_CONFIG_HOME={}", config.display());
    declare_peer(
        &fx,
        &[
            &xdg,
            "EPR_BIN_TOOLS={data_root}/manifest.json",
            "FIXTURE_MARK=declared",
        ],
    );
    // The declared child is bash (the fixture's `epr` bytes) running `peer serve --root …`,
    // which reads the script `peer` from its working directory, the data root.
    fs::create_dir_all(fx.data_root()).unwrap();
    fs::write(
        fx.data_root().join("peer"),
        "trap 'exit 0' INT TERM\nenv > \"$XDG_CONFIG_HOME/child.env\"\n\
         echo \"epr peer: serving $*\"\nwhile :; do sleep 1; done\n",
    )
    .unwrap();
    let Some(mut holder) = Holder::start(&fx, &fx.repo) else {
        return;
    };
    fx.lease(holder.pid());

    let output = fx.epr_with(
        &["device", "supervise", "--handover"],
        &[(REAL_PROC.0, OsStr::new(REAL_PROC.1))],
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "{stdout}\n{}", stderr(&output));
    assert!(
        stdout.contains(&format!("SIGINT to epr peer serve pid {}", holder.pid())),
        "{stdout}"
    );
    assert!(stdout.contains("started pid"), "{stdout}");
    let status = holder.exit();
    assert_eq!(
        status.code(),
        Some(0),
        "the daemon left on SIGINT, never SIGTERM or SIGKILL: {status}"
    );
    let ready = wait_for("the declared peer to serve", || {
        let status = fx.status();
        (status["processes"][0]["ready"] == true).then_some(status)
    });
    assert_eq!(ready["processes"][0]["name"], "peer");

    let seen = fs::read_to_string(config.join("child.env")).unwrap();
    let has = |line: String| seen.lines().any(|l| l == line);
    assert!(has(xdg.clone()), "{seen}");
    assert!(
        has(format!(
            "EPR_BIN_TOOLS={}",
            fx.data_root().join("manifest.json").display()
        )),
        "{seen}"
    );
    assert!(has("FIXTURE_MARK=declared".into()), "{seen}");
    assert!(
        !seen.lines().any(|l| l.starts_with("BIN_STORE=")),
        "ark's scrub keeps undeclared keys out: {seen}"
    );
}

fn start_ticks(pid: u64) -> String {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat")).unwrap();
    let rest = &stat[stat.rfind(')').unwrap() + 2..];
    rest.split_whitespace().nth(19).unwrap().to_owned()
}

/// Same manifest, same bytes: supervise leaves ark alone. Same manifest, a landed current:
/// supervise names the stale child and restarts ark, and the new child executes the current.
#[test]
fn a_child_on_old_bytes_restarts_onto_the_landed_current_and_a_current_one_stays() {
    let Some(fx) = fixture() else { return };
    let resident = script("bytes");
    fx.ok(&[
        "device",
        "run",
        "epr",
        "--ready-line",
        READY,
        "--",
        "-c",
        &resident,
    ]);
    let head = fx.head();
    assert!(fx.ok(&["device", "supervise"]).contains("started pid"));
    let first = wait_for("the resident to become ready", || {
        let status = fx.status();
        (status["processes"][0]["ready"] == true).then_some(status)
    });
    let ark1 = first["pid"].as_u64().unwrap();
    let ticks1 = start_ticks(ark1);
    let child1 = first["processes"][0]["pid"].as_u64().unwrap();

    let again = fx.ok(&["device", "supervise"]);
    assert!(again.contains("at current manifest"), "{again}");
    assert_eq!(fx.status()["pid"].as_u64().unwrap(), ark1);
    assert_eq!(start_ticks(ark1), ticks1, "the same ark, never restarted");
    assert!(alive(child1));

    let old = fx.current().0;
    let new = land(&fx, "source-v2", b"\n# landed v2\n");
    assert_ne!(old, new);
    assert_eq!(fx.head(), head, "a landing leaves the manifest as it was");
    let restarted = fx.ok(&["device", "supervise"]);
    assert!(
        restarted.contains(&format!("epr runs {old}, current is {new}; restarting")),
        "{restarted}"
    );
    let second = wait_for("the resident on the landed bytes", || {
        let status = fx.status();
        let pid = status["processes"][0]["pid"].as_u64()?;
        (status["processes"][0]["ready"] == true && pid != child1).then_some(status)
    });
    assert_eq!(second["manifestCid"], head);
    assert_ne!(second["pid"].as_u64().unwrap(), ark1);
    assert!(!alive(ark1));
    let child2 = second["processes"][0]["pid"].as_u64().unwrap();
    let running =
        elohim_device_runtime::bin_store::file_cid(Path::new(&format!("/proc/{child2}/exe")))
            .unwrap();
    let current = elohim_device_runtime::bin_store::file_cid(&fx.current().1).unwrap();
    assert_eq!(
        running, current,
        "the new child executes the landed current"
    );
    assert!(fx
        .ok(&["device", "supervise"])
        .contains("at current manifest"));
}

/// A launcher candidate key (`EPR_BIN`, or any `candidateEnv` key the registry names) in a
/// `bin:` process's environment would make the child execute a raw candidate and report its file
/// CID instead of the release CID. Declaring one refuses and writes nothing; a manifest that
/// already carries one (here adopted from a base) refuses at supervise and starts nothing; the
/// value is never echoed. Ordinary keys are unaffected.
#[test]
fn a_launcher_candidate_key_is_refused_at_declaration_and_at_supervise_offline() {
    let fx = fixture_with(PathBuf::from("/nonexistent/ark"));
    let secret = "/somewhere/secret-candidate";
    for (tool, key) in [("epr", "EPR_BIN"), ("epr", "BRIT_BIN"), ("brit", "EPR_BIN")] {
        let entry = format!("{key}={secret}");
        let refused = fx.epr(&[
            "device", "run", tool, "--name", "peer", "--env", &entry, "--", "peer", "serve",
        ]);
        assert!(!refused.status.success(), "{tool} {key}");
        let text = stderr(&refused);
        assert!(
            text.contains("REFUSED launcher-env-declared")
                && text.contains(key)
                && text.contains(&format!("bin:{tool}"))
                && text.contains("file CID")
                && text.contains("bin:<tool> channel"),
            "{text}"
        );
        assert!(!text.contains(secret), "the value is never echoed: {text}");
    }
    let shown: Value = serde_json::from_str(&fx.ok(&["device", "show"])).unwrap();
    assert_eq!(shown["processes"], json!([]), "a refusal writes nothing");

    // An ordinary key is unaffected, at declaration and at supervise (which goes on to look
    // for ark).
    fx.ok(&[
        "device", "run", "epr", "--name", "peer", "--env", "A=1", "--", "peer", "serve",
    ]);
    let unaffected = fx.epr(&["device", "supervise"]);
    assert!(
        stderr(&unaffected).contains("ARK_BIN"),
        "{}",
        stderr(&unaffected)
    );

    // A manifest that already carries the key: a base declaring it, adopted by apply.
    let mut base: Value =
        serde_json::from_str(&fx.ok(&["device", "show", "--render", "manifest"])).unwrap();
    base["processes"][0]["env"]["EPR_BIN"] = json!(secret);
    base.as_object_mut().unwrap().remove("supersedes");
    let base_path = fx.base.join("base-with-epr-bin.json");
    fs::write(&base_path, base.to_string()).unwrap();
    fx.ok(&["device", "run", "epr", "--name", "peer", "--stop"]);
    fx.ok(&["device", "apply", "--base", base_path.to_str().unwrap()]);
    let carried: Value =
        serde_json::from_str(&fx.ok(&["device", "show", "--render", "manifest"])).unwrap();
    assert_eq!(carried["processes"][0]["env"]["EPR_BIN"], secret);

    let refused = fx.epr(&["device", "supervise"]);
    assert!(!refused.status.success());
    let text = stderr(&refused);
    assert!(
        text.contains("REFUSED launcher-env-declared")
            && text.contains("EPR_BIN")
            && text.contains("nothing starts"),
        "{text}"
    );
    assert!(!text.contains(secret), "the value is never echoed: {text}");
    assert!(
        !text.contains("ARK_BIN"),
        "refused before ark is looked for: {text}"
    );
    assert_eq!(fx.status()["running"], false, "nothing started");
}

/// The likeliest live state: ark already runs the current head, and a hand-started daemon holds
/// the serve lock, so the declared peer can only wait behind it. A current head is no reason to
/// stand aside: supervise refuses, and `--handover` interrupts the daemon and restarts ark, after
/// which the declared peer takes the lock and serves.
#[test]
fn a_current_ark_does_not_hide_an_unowned_daemon_from_supervise() {
    let Some(fx) = fixture() else { return };
    let Some(python) = python3() else { return };
    let config = fx.base.join("config");
    declare_peer(&fx, &[&format!("XDG_CONFIG_HOME={}", config.display())]);
    // The declared child (bash, the fixture's `epr` bytes) runs a python waiter that takes the
    // serve lock only once `go` exists, retrying while another process holds it.
    let data_root = fx.data_root();
    fs::create_dir_all(&data_root).unwrap();
    fs::write(
        data_root.join("peer"),
        format!(
            "trap 'kill \"$py\" 2>/dev/null; exit 0' INT TERM\n\
             {} \"$PWD/peer.py\" &\npy=$!\nwait \"$py\"\n",
            python.display()
        ),
    )
    .unwrap();
    fs::write(
        data_root.join("peer.py"),
        "import fcntl, os, signal, sys, time\n\
         signal.signal(signal.SIGTERM, lambda *_: sys.exit(0))\n\
         while not os.path.exists('go'):\n    time.sleep(0.05)\n\
         path = os.path.join(os.environ['XDG_CONFIG_HOME'], 'epr', 'peer', 'serve.lock')\n\
         os.makedirs(os.path.dirname(path), exist_ok=True)\n\
         lock = open(path, 'a')\n\
         while True:\n    try:\n        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)\n        break\n    except BlockingIOError:\n        time.sleep(0.1)\n\
         print('epr peer: serving', flush=True)\n\
         while True:\n    time.sleep(0.2)\n",
    )
    .unwrap();
    let proc = [(REAL_PROC.0, OsStr::new(REAL_PROC.1))];
    let started = fx.epr_with(&["device", "supervise"], &proc);
    assert!(started.status.success(), "{}", stderr(&started));
    let ark1 = wait_for("ark at the head", || {
        let status = fx.status();
        status["processes"][0]["pid"].as_u64().map(|_| status)
    })["pid"]
        .as_u64()
        .unwrap();

    // The hand-started daemon takes the lock first; only then may the declared peer try.
    let Some(mut holder) = Holder::start(&fx, &fx.repo) else {
        return;
    };
    fs::write(data_root.join("go"), "").unwrap();
    thread::sleep(Duration::from_millis(500));
    assert_ne!(fx.status()["processes"][0]["ready"], true, "the peer waits");

    let refused = fx.epr_with(&["device", "supervise"], &proc);
    assert!(!refused.status.success(), "a current head hid the holder");
    let text = stderr(&refused);
    assert!(text.contains("REFUSED peer-serve-unowned"), "{text}");
    assert!(holder.running());
    assert_eq!(fx.status()["pid"].as_u64().unwrap(), ark1, "ark untouched");

    fx.lease(holder.pid());
    let output = fx.epr_with(&["device", "supervise", "--handover"], &proc);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "{stdout}\n{}", stderr(&output));
    assert!(stdout.contains("handing over, then restarting"), "{stdout}");
    assert_eq!(holder.exit().code(), Some(0), "SIGINT only");
    let ready = wait_for("the declared peer to serve", || {
        let status = fx.status();
        (status["processes"][0]["ready"] == true).then_some(status)
    });
    assert_ne!(ready["pid"].as_u64().unwrap(), ark1, "ark was restarted");
}
