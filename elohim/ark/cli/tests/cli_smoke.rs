use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Child, Command, Output, Stdio},
    thread,
    time::{Duration, Instant},
};

use ark_core::RuntimeManifest;
use nix::{
    sys::signal::{kill, Signal},
    unistd::Pid,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

const ARK: &str = env!("CARGO_BIN_EXE_ark");
const SHELL: &str = "/bin/sh";

fn shell_sha256() -> String {
    let bytes = fs::read(SHELL).expect("read /bin/sh");
    format!("{:x}", Sha256::digest(bytes))
}

fn manifest_json(process: &str) -> Value {
    json!({
        "schema": 1,
        "kind": "runtime-manifest",
        "reach": "trusted",
        "processes": [{
            "name": process,
            "kind": "native",
            "artifact": {"pinned": {"sha256": shell_sha256()}},
            "argv": ["{artifact}", "-c", "echo booted; exec sleep 300"],
            "readiness": [{
                "stdout_line": {"contains": "booted", "patience_ms": 5000}
            }],
            "policy": {
                "shutdown": {"signal": 2, "grace_ms": 1000},
                "backoff": {"min_s": 0, "max_s": 0, "steps": 1}
            }
        }]
    })
}

fn manifest_cid(manifest: &Value) -> String {
    RuntimeManifest::from_json(&manifest.to_string())
        .expect("valid test manifest")
        .cid()
        .expect("addressable test manifest")
}

fn write_declarations(root: &Path, manifest: &Value, berth_cid: &str) -> (PathBuf, PathBuf) {
    let manifest_path = root.join("manifest.json");
    let berth_path = root.join("berth.json");
    fs::write(
        &manifest_path,
        serde_json::to_vec(manifest).expect("encode manifest"),
    )
    .expect("write manifest");
    fs::write(
        &berth_path,
        serde_json::to_vec(&json!({
            "manifest": berth_cid,
            "data_root": root,
            "artifacts": {"child": SHELL}
        }))
        .expect("encode berth"),
    )
    .expect("write berth");
    (manifest_path, berth_path)
}

fn ark(args: &[&str]) -> Output {
    Command::new(ARK).args(args).output().expect("run ark")
}

fn wait_until<T>(what: &str, timeout: Duration, mut probe: impl FnMut() -> Option<T>) -> T {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(value) = probe() {
            return value;
        }
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        thread::sleep(Duration::from_millis(50));
    }
}

fn run_until_ready_then_stop(log_level: Option<&str>) -> String {
    let root = tempfile::tempdir().expect("temp data root");
    let manifest = manifest_json("child");
    let cid = manifest_cid(&manifest);
    let (manifest_path, berth_path) = write_declarations(root.path(), &manifest, &cid);
    let mut command = Command::new(ARK);
    command.env_remove("ARK_LOG");
    command.args([
        "run",
        "--manifest",
        manifest_path.to_str().unwrap(),
        "--berth",
        berth_path.to_str().unwrap(),
    ]);
    if let Some(level) = log_level {
        command.args(["--log-level", level]);
    }
    let child = command
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn ark run");
    let mut ark_process = ArkProcess(child);

    wait_until("ready child passport", Duration::from_secs(5), || {
        let bytes = fs::read(root.path().join("ark/passport.json")).ok()?;
        let passport: Value = serde_json::from_slice(&bytes).ok()?;
        passport["processes"][0]["ready"].as_bool()?.then_some(())
    });
    kill(ark_process.pid(), Signal::SIGTERM).expect("SIGTERM ark");
    let status = wait_until("ark to stop cleanly", Duration::from_secs(5), || {
        ark_process.0.try_wait().expect("wait ark")
    });
    assert_eq!(status.code(), Some(0));

    let mut stderr = String::new();
    ark_process
        .0
        .stderr
        .take()
        .expect("piped ark stderr")
        .read_to_string(&mut stderr)
        .expect("read ark stderr");
    stderr
}

struct ArkProcess(Child);

impl ArkProcess {
    fn pid(&self) -> Pid {
        Pid::from_raw(self.0.id() as i32)
    }
}

impl Drop for ArkProcess {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = kill(self.pid(), Signal::SIGTERM);
            for _ in 0..40 {
                if self.0.try_wait().ok().flatten().is_some() {
                    return;
                }
                thread::sleep(Duration::from_millis(50));
            }
            let _ = kill(self.pid(), Signal::SIGKILL);
            let _ = self.0.wait();
        }
    }
}

#[test]
fn hash_matches_sha256sum() {
    let file = tempfile::NamedTempFile::new().expect("temp file");
    fs::write(file.path(), b"the ark hashes these exact bytes\n").expect("write temp file");
    let expected = format!(
        "{:x}",
        Sha256::digest(b"the ark hashes these exact bytes\n")
    );

    let output = ark(&["hash", file.path().to_str().expect("utf-8 path")]);

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), expected);
}

#[test]
fn run_refuses_mismatched_berth_manifest_with_65() {
    let root = tempfile::tempdir().expect("temp data root");
    let manifest = manifest_json("child");
    let actual = manifest_cid(&manifest);
    let other = manifest_json("other-child");
    let declared = manifest_cid(&other);
    let (manifest_path, berth_path) = write_declarations(root.path(), &manifest, &declared);

    let output = ark(&[
        "run",
        "--manifest",
        manifest_path.to_str().unwrap(),
        "--berth",
        berth_path.to_str().unwrap(),
    ]);

    assert_eq!(output.status.code(), Some(65));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains(&declared), "{stderr}");
    assert!(stderr.contains(&actual), "{stderr}");
}

#[test]
fn witness_ls_on_empty_spool_prints_empty_array() {
    let root = tempfile::tempdir().expect("temp data root");
    let manifest = manifest_json("child");
    let cid = manifest_cid(&manifest);
    let (_, berth_path) = write_declarations(root.path(), &manifest, &cid);

    let output = ark(&["witness", "ls", "--berth", berth_path.to_str().unwrap()]);

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        json!([])
    );
}

#[test]
fn witness_show_invalid_cid_is_spool_failure_67() {
    let root = tempfile::tempdir().expect("temp data root");
    let manifest = manifest_json("child");
    let cid = manifest_cid(&manifest);
    let (_, berth_path) = write_declarations(root.path(), &manifest, &cid);

    let output = ark(&[
        "witness",
        "show",
        "--berth",
        berth_path.to_str().unwrap(),
        "../passport",
    ]);

    assert_eq!(output.status.code(), Some(67));
}

#[test]
fn run_with_error_log_level_prints_no_state_lines() {
    let stderr = run_until_ready_then_stop(Some("error"));

    assert!(!stderr.contains(r#""ark":"state""#), "{stderr}");
}

#[test]
fn run_with_default_log_level_prints_state_lines() {
    let stderr = run_until_ready_then_stop(None);

    assert!(
        stderr.contains(r#"{"ark":"log_level","level":"info"}"#),
        "{stderr}"
    );
    assert!(stderr.contains(r#""ark":"state""#), "{stderr}");
}

#[test]
fn run_then_kill_child_then_witness_ls_shows_one() {
    let root = tempfile::tempdir().expect("temp data root");
    let manifest = manifest_json("child");
    let cid = manifest_cid(&manifest);
    let (manifest_path, berth_path) = write_declarations(root.path(), &manifest, &cid);

    let child = Command::new(ARK)
        .args([
            "run",
            "--manifest",
            manifest_path.to_str().unwrap(),
            "--berth",
            berth_path.to_str().unwrap(),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn ark run");
    let mut ark_process = ArkProcess(child);

    let child_pid = wait_until("ready child passport", Duration::from_secs(5), || {
        let bytes = fs::read(root.path().join("ark/passport.json")).ok()?;
        let passport: Value = serde_json::from_slice(&bytes).ok()?;
        let process = passport.get("processes")?.as_array()?.first()?;
        process.get("ready")?.as_bool()?.then(|| {
            process
                .get("pid")
                .and_then(Value::as_u64)
                .expect("ready child pid") as u32
        })
    });
    kill(Pid::from_raw(child_pid as i32), Signal::SIGKILL).expect("SIGKILL child");

    let row = wait_until("one paired witness row", Duration::from_secs(5), || {
        let output = ark(&["witness", "ls", "--berth", berth_path.to_str().unwrap()]);
        if !output.status.success() {
            return None;
        }
        let rows: Vec<Value> = serde_json::from_slice(&output.stdout).ok()?;
        (rows.len() == 1
            && rows[0]
                .get("write_ahead_cid")
                .and_then(Value::as_str)
                .is_some()
            && rows[0].get("verdict_cid").and_then(Value::as_str).is_some())
        .then(|| rows.into_iter().next().unwrap())
    });
    assert_eq!(row["process"], "child");
    assert_eq!(row["pid"], child_pid);
    assert_eq!(row["exit"]["class"], "signaled");
    assert_eq!(row["exit"]["signal"], 9);
    assert!(row["verdict"]["restart"].is_object(), "{row}");

    kill(ark_process.pid(), Signal::SIGTERM).expect("SIGTERM ark");
    let status = wait_until("ark to stop cleanly", Duration::from_secs(5), || {
        ark_process.0.try_wait().expect("wait ark")
    });
    assert_eq!(status.code(), Some(0));
}

#[test]
fn bin_channel_runs_verified_device_current_and_passport_observes_it() {
    use elohim_device_runtime::{
        bin_store::{IngestOptions, Store},
        manifest_store,
    };
    use std::os::unix::fs::{symlink, PermissionsExt};
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    fs::create_dir(&source).unwrap();
    for bin in ["epr", "bin-launch"] {
        let path = source.join(bin);
        fs::write(&path, b"#!/bin/sh\necho booted\nexec sleep 300\n").unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }
    let base = manifest_store::workspace_base().unwrap();
    let registry = manifest_store::registry(&base).unwrap();
    let bins = root.path().join("bins");
    struct IdentityRunner;
    impl elohim_device_runtime::bin_store::Runner for IdentityRunner {
        fn run(
            &self,
            command: &elohim_device_runtime::bin_store::CommandSpec,
        ) -> Result<
            elohim_device_runtime::bin_store::CommandOutput,
            elohim_device_runtime::bin_store::Refusal,
        > {
            Ok(elohim_device_runtime::bin_store::CommandOutput { success: true,
                stdout: json!({"cid":elohim_device_runtime::bin_store::file_cid(Path::new(&command.program))?,"version":"fixture"}).to_string(), stderr: String::new() })
        }
    }
    let store = Store::new(bins.join("epr"), "epr", &registry)
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
        serde_json::to_vec(&json!({"cid":release.cid})).unwrap(),
    )
    .unwrap();
    manifest_store::write_registry(&bins.join("bin/bin-tools.json"), &registry).unwrap();
    let mut manifest = manifest_json("child");
    manifest["processes"][0]["artifact"] = json!({"channel":{"channel_id":"bin:epr"}});
    manifest["follows"] = serde_json::to_value(&base.follows).unwrap();
    manifest["tools"] = serde_json::to_value(&base.tools).unwrap();
    let cid = manifest_cid(&manifest);
    let (manifest_path, berth_path) = write_declarations(root.path(), &manifest, &cid);
    let child = Command::new(ARK)
        .args(["run", "--manifest"])
        .arg(manifest_path)
        .arg("--berth")
        .arg(berth_path)
        .env_remove("EPR_STORE")
        .env_remove("BRIT_STORE")
        .env("BIN_STORE", &bins)
        .env("EPR_BIN_TOOLS", bins.join("bin/bin-tools.json"))
        .env("EPR_PROC_ROOT", root.path().join("proc"))
        .env("ELOHIM_DEVICE_KEY_FILE", root.path().join("key"))
        .env("ELOHIM_SESSION_ID", "manifest-fixture")
        .env_remove("EPR_SESSION_LAZY")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut process = ArkProcess(child);
    let passport = wait_until("channel child", Duration::from_secs(10), || {
        let bytes = fs::read(root.path().join("ark/passport.json")).ok()?;
        let value: Value = serde_json::from_slice(&bytes).ok()?;
        value["processes"][0]["ready"].as_bool()?.then_some(value)
    });
    assert_eq!(passport["tools"][1]["tool"], "epr");
    assert_eq!(passport["tools"][1]["appliedRelease"], release.cid);
    assert_eq!(passport["follows"][0]["appliedRelease"], release.cid);
    assert_eq!(passport["manifest"], cid);
    assert_eq!(
        passport["processes"][0]["artifact_path"],
        store
            .root()
            .join(&release.cid)
            .join("epr")
            .to_str()
            .unwrap()
    );
    kill(process.pid(), Signal::SIGTERM).unwrap();
    let status = wait_until("ark stopped", Duration::from_secs(5), || {
        process.0.try_wait().unwrap()
    });
    assert!(status.success());
}

// ---- ark task: a device's own gate as a placement Task ---------------------------------------------

/// A task fixture: a child script, a closure material, a fake `brit-build-ref` answering `rows`,
/// and the spec `ark task spec` built for them, all under `root`.
struct TaskFixture {
    root: PathBuf,
    spec: PathBuf,
    material: PathBuf,
    child: PathBuf,
    status: PathBuf,
    brit: PathBuf,
}

/// How the fake `brit-build-ref` answers `signer --json` (ark passes `--repo <repo> signer
/// --json`, so the subcommand is `$3`): as the daily-driver brit, with this workspace's agent
/// id, or as the pinned brit that has no `signer` at all (clap's usage error, exit 2), under
/// which ark falls back to the reuse context's own `selfId`.
#[derive(Clone, Copy)]
enum FakeSigner {
    Answers(&'static str),
    Unsupported,
}

fn task_fixture(root: &Path, method: &str, script: &str, rows: &Value) -> TaskFixture {
    task_fixture_with(root, method, script, rows, FakeSigner::Answers("me"))
}

fn task_fixture_with(
    root: &Path,
    method: &str,
    script: &str,
    rows: &Value,
    signer: FakeSigner,
) -> TaskFixture {
    let child = root.join("child.sh");
    fs::write(&child, format!("#!/bin/sh\n{script}\n")).unwrap();
    let material = root.join("material.json");
    fs::write(&material, r#"{"project":"fixture","v":3}"#).unwrap();
    let brit = root.join("brit-build-ref");
    let signer_case = match signer {
        FakeSigner::Answers(id) => format!("printf '%s\\n' '{{\"agentId\":\"{id}\"}}'"),
        FakeSigner::Unsupported => "printf '%s\\n' \"error: unrecognized subcommand 'signer'\" \
                                   \"\" 'Usage: brit-build-ref [OPTIONS] <COMMAND>' \"\" \
                                   \"For more information, try '--help'.\" >&2; exit 2"
            .to_string(),
    };
    fs::write(
        &brit,
        format!(
            "#!/bin/sh\ncase \"$3\" in\n  signer) {signer_case} ;;\n  *) cat <<'ROWS'\n{rows}\nROWS\n  ;;\nesac\n"
        ),
    )
    .unwrap();
    for path in [&child, &brit] {
        Command::new("chmod").arg("+x").arg(path).status().unwrap();
    }
    let sha = format!("{:x}", Sha256::digest(fs::read(&child).unwrap()));
    let request = root.join("request.json");
    fs::write(
        &request,
        json!({
            "method": {"name": method, "step": "gate:fixture", "deterministic": true},
            "child": {"artifact": {"pinned": {"sha256": sha}}, "argv": ["child.sh"]},
        })
        .to_string(),
    )
    .unwrap();
    let made = ark(&[
        "task",
        "spec",
        "--request",
        request.to_str().unwrap(),
        "--material",
        material.to_str().unwrap(),
    ]);
    assert!(
        made.status.success(),
        "{}",
        String::from_utf8_lossy(&made.stderr)
    );
    let made: Value = serde_json::from_slice(&made.stdout).unwrap();
    for cid in ["method", "input"] {
        assert!(made["derivation"][cid]
            .as_str()
            .unwrap()
            .starts_with("bafyrei"));
    }
    let spec = root.join("spec.json");
    fs::write(&spec, made["spec"].to_string()).unwrap();
    TaskFixture {
        root: root.to_path_buf(),
        spec,
        material,
        child,
        status: root.join("status.jsonl"),
        brit,
    }
}

impl TaskFixture {
    fn command(&self, reuse: Option<&Value>) -> Command {
        let mut command = Command::new(ARK);
        command
            .args(["task", "run", "--spec"])
            .arg(&self.spec)
            .arg("--material")
            .arg(&self.material)
            .arg("--artifact")
            .arg(&self.child)
            .arg("--status")
            .arg(&self.status)
            .arg("--repo")
            .arg(&self.root)
            .args(["--device", "me"])
            .env("BRIT_BUILD_REF_BIN", &self.brit);
        if let Some(reuse) = reuse {
            let path = self.root.join("reuse.json");
            fs::write(&path, reuse.to_string()).unwrap();
            command.arg("--reuse").arg(path);
        }
        command
    }

    fn statuses(&self) -> Vec<Value> {
        fs::read_to_string(&self.status)
            .unwrap_or_default()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }
}

/// A miss runs the child: its bytes pass through unchanged, the exit is the child's, and the
/// status goes queued then answered (failed, not a hit) with a local completion.
#[test]
fn task_run_passes_the_child_through_and_answers_its_exit() {
    let root = tempfile::tempdir().unwrap();
    let task = task_fixture(
        root.path(),
        "brit-gate:fixture",
        "printf 'out line\\npartial'; echo 'err line' >&2; exit 3",
        &json!([]),
    );
    let out = task.command(None).output().unwrap();
    assert_eq!(out.status.code(), Some(3));
    assert_eq!(out.stdout, b"out line\npartial");
    assert!(String::from_utf8_lossy(&out.stderr).contains("err line"));
    let statuses = task.statuses();
    assert_eq!(statuses[0], json!({"status": "queued"}));
    let answer = &statuses[1]["value"];
    assert_eq!(statuses[1]["status"], "answered");
    assert_eq!(
        (answer["outcome"].as_str(), answer["hit"].as_bool()),
        (Some("failed"), Some(false))
    );
    assert_eq!(
        answer["completion"],
        json!({"by": "local", "device": "me", "at": answer["completion"]["at"]})
    );
    assert_eq!(answer["record"], Value::Null);
}

/// A green attestation by this workspace for an unchanged tree answers the task: no child runs.
/// The same attestation under report-only mode is not reused.
#[test]
fn task_run_reuses_an_admitted_attestation_and_runs_nothing() {
    let root = tempfile::tempdir().unwrap();
    let git = |args: &[&str]| {
        let out = Command::new("git")
            .arg("-C")
            .arg(root.path())
            .args(args)
            .output()
            .unwrap();
        assert!(out.status.success(), "git {args:?}");
        String::from_utf8(out.stdout).unwrap().trim().to_string()
    };
    git(&["init", "-q"]);
    git(&[
        "-c",
        "user.name=t",
        "-c",
        "user.email=t@t",
        "commit",
        "-q",
        "--allow-empty",
        "-m",
        "a",
    ]);
    let head = git(&["rev-parse", "HEAD"]);
    let output = elohim_epr::cid::compute_cid(b"verdict").to_bytes();
    let rows = json!([{"commit": head, "attestation": {
        "agentId": "me", "builtAt": "2026-10-26T10:00:00Z", "success": true,
        "outputCid": output, "signature": "sig",
    }}]);
    let task = task_fixture(root.path(), "brit-gate:fixture", "touch ran; exit 0", &rows);
    let reuse =
        json!({"mode": "reuse", "enabled": true, "selfId": "me", "dirty": false, "commit": head});
    let out = task.command(Some(&reuse)).output().unwrap();
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!root.path().join("ran").exists(), "a hit runs no child");
    let statuses = task.statuses();
    assert_eq!(statuses[1]["value"]["hit"], true);
    assert_eq!(statuses[1]["value"]["outcome"], "done");
    assert_eq!(statuses[1]["value"]["record"]["computedBy"], "me");

    fs::remove_file(&task.status).unwrap();
    let report_only =
        json!({"mode": null, "enabled": true, "selfId": "me", "dirty": false, "commit": head});
    let out = task.command(Some(&report_only)).output().unwrap();
    assert_eq!(out.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&out.stderr).contains("not reused: report-only"));
    assert!(root.path().join("ran").exists());
    assert_eq!(task.statuses()[1]["value"]["hit"], false);
}

/// Under the pinned brit, which has no `signer` subcommand, the reuse context's own `selfId` is
/// the workspace key: the same attestation is still reused and runs nothing.
#[test]
fn task_run_reuses_under_a_brit_without_signer() {
    let root = tempfile::tempdir().unwrap();
    let git = |args: &[&str]| {
        let out = Command::new("git")
            .arg("-C")
            .arg(root.path())
            .args(args)
            .output()
            .unwrap();
        assert!(out.status.success(), "git {args:?}");
        String::from_utf8(out.stdout).unwrap().trim().to_string()
    };
    git(&["init", "-q"]);
    git(&[
        "-c",
        "user.name=t",
        "-c",
        "user.email=t@t",
        "commit",
        "-q",
        "--allow-empty",
        "-m",
        "a",
    ]);
    let head = git(&["rev-parse", "HEAD"]);
    let output = elohim_epr::cid::compute_cid(b"verdict").to_bytes();
    let rows = json!([{"commit": head, "attestation": {
        "agentId": "me", "builtAt": "2026-10-26T10:00:00Z", "success": true,
        "outputCid": output, "signature": "sig",
    }}]);
    let task = task_fixture_with(
        root.path(),
        "brit-gate:fixture",
        "touch ran; exit 0",
        &rows,
        FakeSigner::Unsupported,
    );
    let reuse =
        json!({"mode": "reuse", "enabled": true, "selfId": "me", "dirty": false, "commit": head});
    let out = task.command(Some(&reuse)).output().unwrap();
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        !root.path().join("ran").exists(),
        "a hit under the old brit runs no child"
    );
    assert_eq!(task.statuses()[1]["value"]["hit"], true);
    assert_eq!(task.statuses()[1]["value"]["record"]["computedBy"], "me");
}

/// SIGTERM to ark mid-run reaches the child and its own children, and the answer is unanswered
/// with ruling 7's three parts, never answered.
#[test]
fn task_run_interrupted_is_unanswered_with_three_parts() {
    let root = tempfile::tempdir().unwrap();
    let task = task_fixture(
        root.path(),
        "brit-gate:fixture",
        "echo started; (sleep 77; touch grandchild) & sleep 78",
        &json!([]),
    );
    let mut child: Child = task.command(None).stdout(Stdio::piped()).spawn().unwrap();
    let mut stdout = child.stdout.take().unwrap();
    let mut first = [0u8; 8];
    stdout.read_exact(&mut first).unwrap();
    assert_eq!(&first, b"started\n");
    kill(Pid::from_raw(child.id() as i32), Signal::SIGTERM).unwrap();
    let status = child.wait().unwrap();
    assert_eq!(status.code(), Some(143));
    let statuses = task.statuses();
    let last = statuses.last().unwrap();
    assert_eq!(last["status"], "unanswered");
    for part in ["why", "comeBack", "meanwhile"] {
        assert!(!last["value"][part].is_null(), "{part}");
    }
    assert!(!statuses.iter().any(|s| s["status"] == "answered"));
    thread::sleep(Duration::from_millis(300));
    assert!(!root.path().join("grandchild").exists());
}

/// Only a device's own task runs here; a measure stage is refused before anything is written.
#[test]
fn task_run_refuses_a_task_that_is_not_the_devices_own() {
    let root = tempfile::tempdir().unwrap();
    let task = task_fixture(root.path(), "a2o-stage:x", "exit 0", &json!([]));
    let out = task.command(None).output().unwrap();
    assert_eq!(out.status.code(), Some(65));
    assert!(task.statuses().is_empty());
}

/// ark is the subreaper of a task's tree, so it reaps the orphans that exit while the task runs;
/// none is left a zombie holding a pid until ark exits.
#[test]
fn task_run_reaps_orphans_while_the_task_runs() {
    let root = tempfile::tempdir().unwrap();
    let task = task_fixture(
        root.path(),
        "brit-gate:fixture",
        "for i in 1 2 3 4 5; do (sleep 0.1 &); done; echo spawned; sleep 3",
        &json!([]),
    );
    let mut child: Child = task.command(None).stdout(Stdio::piped()).spawn().unwrap();
    let mut stdout = child.stdout.take().unwrap();
    let mut first = [0u8; 8];
    stdout.read_exact(&mut first).unwrap();
    thread::sleep(Duration::from_millis(1500));
    let ark = child.id().to_string();
    let zombies = fs::read_dir("/proc")
        .unwrap()
        .flatten()
        .filter_map(|entry| fs::read_to_string(entry.path().join("stat")).ok())
        .filter(|stat| {
            let fields: Vec<&str> = stat
                .rsplit_once(')')
                .map_or(vec![], |(_, rest)| rest.split_whitespace().collect());
            fields.first() == Some(&"Z") && fields.get(1) == Some(&ark.as_str())
        })
        .count();
    assert_eq!(zombies, 0, "orphans left unreaped under ark");
    assert_eq!(child.wait().unwrap().code(), Some(0));
}
