//! Real exec tests: no mock resolver, PATH store layout or shell launcher.
use elohim_device_runtime::bin_store::{
    file_cid, CommandOutput, CommandSpec, IngestOptions, Refusal, Registry, Runner, Store,
};
use serde_json::json;
use std::{
    fs,
    os::unix::{ffi::OsStringExt, fs::symlink},
    path::Path,
    process::Command,
    sync::Arc,
};
struct Identity;
impl Runner for Identity {
    fn run(&self, c: &CommandSpec) -> Result<CommandOutput, Refusal> {
        let stdout = if Path::new(&c.program)
            .file_name()
            .is_some_and(|n| n == "brit")
        {
            "brit fixture\n".into()
        } else {
            json!({"cid":file_cid(Path::new(&c.program))?,"version":"fixture"}).to_string()
        };
        Ok(CommandOutput {
            success: true,
            stdout,
            stderr: String::new(),
        })
    }
}
fn setup(tool: &str, binary: &Path) -> (tempfile::TempDir, Store, std::path::PathBuf, String) {
    let tmp = tempfile::tempdir().unwrap();
    let registry:Registry=serde_json::from_value(json!({"version":1,"tools":{tool:{"bins":[tool],"aliases":if tool=="brit" {json!({"rakia":"brit"})}else{json!({})},"longRunning":[["-c"]],"candidateEnv":{tool:[format!("{}_BIN",tool.to_uppercase())]}}}})).unwrap();
    let store = Store::new(tmp.path().join("store").join(tool), tool, &registry)
        .unwrap()
        .with_runner(Arc::new(Identity));
    let release = store
        .ingest(
            binary,
            &IngestOptions {
                source_commit: Some("fixture".into()),
                tree_clean: Some(true),
                ..Default::default()
            },
        )
        .unwrap();
    symlink(&release.cid, store.root().join("current")).unwrap();
    let bin = tmp.path().join("launch");
    fs::create_dir_all(&bin).unwrap();
    fs::copy(env!("CARGO_BIN_EXE_bin-launch"), bin.join("bin-launch")).unwrap();
    symlink("bin-launch", bin.join(tool)).unwrap();
    if tool == "brit" {
        symlink("bin-launch", bin.join("rakia")).unwrap();
    }
    fs::write(
        bin.join("bin-tools.json"),
        serde_json::to_vec(&registry).unwrap(),
    )
    .unwrap();
    (tmp, store, bin, release.cid)
}
#[test]
fn exec_keeps_pid_current_exe_and_daemon_lease_outside_checkout() {
    let (tmp, store, bin, cid) = setup("epr", Path::new("/bin/bash"));
    let child = Command::new(bin.join("epr"))
        .args(["-c", "/usr/bin/readlink /proc/$$/exe; :"])
        .env_clear()
        .env("BIN_STORE", store.root().parent().unwrap())
        .env("ELOHIM_SESSION_ID", "caller")
        .current_dir(tmp.path())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let pid = child.id();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        store.root().join(&cid).join("epr").to_str().unwrap()
    );
    let leases: Vec<_> = fs::read_dir(store.root().join("sessions"))
        .unwrap()
        .map(|p| p.unwrap().path())
        .collect();
    assert_eq!(leases.len(), 1);
    let pin: serde_json::Value = serde_json::from_slice(&fs::read(&leases[0]).unwrap()).unwrap();
    assert_eq!(pin["ownerPid"], pid);
    assert_eq!(pin["cid"], cid);
    assert!(pin["session"].as_str().unwrap().starts_with("daemon-"));
    assert!(store.root().join(cid).join(".used").exists());
}
#[test]
fn alias_argv_zero_and_non_utf8_arguments_survive_exec() {
    let (tmp, store, bin, _) = setup("brit", Path::new("/bin/bash"));
    let output = Command::new(bin.join("rakia"))
        .args(["-c", "printf '%s' \"$0\""])
        .env_clear()
        .env("BIN_STORE", store.root().parent().unwrap())
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, b"rakia");
    let raw = std::ffi::OsString::from_vec(vec![b'x', 0xff]);
    let output = Command::new(bin.join("rakia"))
        .args(["-c", "printf '%s' \"$1\"", "arg0"])
        .arg(raw)
        .env_clear()
        .env("BIN_STORE", store.root().parent().unwrap())
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, [b'x', 0xff]);
}
#[test]
fn two_processes_publish_one_pin_using_bin_store_override() {
    let (tmp, store, bin, cid) = setup("epr", Path::new("/bin/echo"));
    let mut children = Vec::new();
    for _ in 0..2 {
        children.push(
            Command::new(bin.join("epr"))
                .arg("hello")
                .env_clear()
                .env("BIN_STORE", store.root().parent().unwrap())
                .env("ELOHIM_SESSION_ID", "same")
                .current_dir(tmp.path())
                .stdout(std::process::Stdio::null())
                .spawn()
                .unwrap(),
        );
    }
    for mut child in children {
        assert!(child.wait().unwrap().success());
    }
    let value: serde_json::Value =
        serde_json::from_slice(&fs::read(store.root().join("sessions/same")).unwrap()).unwrap();
    assert_eq!(value["cid"], cid);
    assert_eq!(
        fs::read_dir(store.root().join("sessions")).unwrap().count(),
        1
    );
}
