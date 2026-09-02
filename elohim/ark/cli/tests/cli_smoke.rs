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
