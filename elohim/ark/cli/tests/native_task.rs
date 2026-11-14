//! Native fixture has no shell, helper executable or network. Its synchronization files let
//! the operator edit B while the private restored A is executing.
#![cfg(target_os = "linux")]

use eprfs_core::{BlobCid, FetchPolicy, TreeLimits};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Child, Command, Output, Stdio},
    thread,
    time::{Duration, Instant},
};

fn ark() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ark"))
}

struct Fixture {
    temp: tempfile::TempDir,
    source: PathBuf,
    store: PathBuf,
    root: BlobCid,
    artifact: PathBuf,
    request: Value,
}

impl Fixture {
    fn new(mode: &str) -> Self {
        let temp = tempfile::tempdir().expect("fixture directory");
        let source = temp.path().join("source");
        let store = temp.path().join("store");
        fs::create_dir(&source).expect("source");
        fs::write(source.join("value"), b"A captured\n").expect("source A");
        let root = futures::executor::block_on(eprfs_local::capture_tree(
            &source,
            &store,
            &["value".into()],
            &TreeLimits::default(),
        ))
        .expect("capture")
        .root;
        let artifact = temp.path().join("fixture");
        fs::copy(std::env::current_exe().expect("test executable"), &artifact)
            .expect("fixture artifact");
        let digest = hex::encode(Sha256::digest(fs::read(&artifact).expect("artifact bytes")));
        let request = json!({
            "method": {"name":"brit-gate:native-fixture", "step":"native-fixture", "deterministic":false},
            "child": {"artifact":{"pinned":{"sha256":digest}},
                "argv":["{artifact}", "--exact", "native_fixture_process", "--nocapture"],
                "env":{"PATH":"", "HOME":"{data_root}/home", "FIXTURE_MODE":mode,
                    "INPUT":"{input}", "OUTPUT":"{output}", "RUST_TEST_THREADS":"1"}, "envScrub":true},
            "outputs":["answer"]
        });
        Self {
            temp,
            source,
            store,
            root,
            artifact,
            request,
        }
    }

    fn command(&self, label: &str) -> Command {
        let request = self.temp.path().join(format!("{label}.json"));
        fs::write(
            &request,
            serde_json::to_vec(&self.request).expect("request JSON"),
        )
        .expect("write request");
        let mut cmd = ark();
        cmd.args(["task", "run", "--root", &self.root.to_string()]);
        cmd.arg("--store")
            .arg(&self.store)
            .arg("--request")
            .arg(request);
        cmd.arg("--artifact")
            .arg(&self.artifact)
            .arg("--run-dir")
            .arg(self.run(label));
        cmd.env("SHOULD_NOT_SURVIVE", "private ambient value");
        cmd.current_dir(self.temp.path());
        cmd
    }

    fn run(&self, label: &str) -> PathBuf {
        self.temp.path().join(label)
    }

    fn refusal(&self, label: &str, expected: &str) {
        let output = self.command(label).output().expect("run refusal");
        assert!(!output.status.success(), "unexpected success: {:?}", output);
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(expected),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            !self.run(label).join("output/started").exists(),
            "preflight spawned child"
        );
    }
}

fn result(output: Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let record: Value =
        serde_json::from_slice(&output.stdout).expect("one existing TaskOutcome JSON");
    assert_task_record_has_no_provides(&record);
    record
}

// All successful native-task scenarios exercise the record boundary as well as
// the outcome. Provides belongs to the envelope, never a task's transferable effect.
fn assert_task_record_has_no_provides(value: &Value) {
    match value {
        Value::Object(fields) => {
            assert!(!fields.contains_key("provides"));
            for nested in fields.values() {
                assert_task_record_has_no_provides(nested);
            }
        }
        Value::Array(values) => {
            for nested in values {
                assert_task_record_has_no_provides(nested);
            }
        }
        _ => {}
    }
}

struct Running(Option<Child>);
impl Drop for Running {
    fn drop(&mut self) {
        if let Some(child) = &mut self.0 {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn ready(child: &mut Child, file: &Path) {
    let until = Instant::now() + Duration::from_secs(20);
    while !file.exists() {
        assert!(
            child.try_wait().expect("poll").is_none(),
            "child exited before synchronization"
        );
        assert!(Instant::now() < until, "fixture synchronization timeout");
        thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn native_fixture_process() {
    let Ok(mode) = std::env::var("FIXTURE_MODE") else {
        return;
    };
    let input = PathBuf::from(std::env::var_os("INPUT").expect("input"));
    let output = PathBuf::from(std::env::var_os("OUTPUT").expect("output"));
    assert_eq!(std::env::var("PATH").expect("explicit PATH"), "");
    assert!(std::env::var_os("SHOULD_NOT_SURVIVE").is_none());
    assert_eq!(
        PathBuf::from(std::env::var_os("HOME").expect("HOME")),
        output.parent().expect("run").join("home")
    );
    fs::write(output.join("started"), b"started").expect("start marker");
    match mode.as_str() {
        "sync" | "interrupt" => {
            fs::write(output.join("ready"), b"ready").expect("ready");
            let until = Instant::now() + Duration::from_secs(20);
            while !output.join("continue").exists() {
                assert!(Instant::now() < until, "no continuation");
                thread::sleep(Duration::from_millis(10));
            }
        }
        "add" => fs::create_dir(input.join("unlisted-empty-directory")).expect("add directory"),
        "change" => fs::write(input.join("value"), b"changed").expect("change input"),
        "delete" => fs::remove_file(input.join("value")).expect("delete input"),
        "mode" => fs::set_permissions(input.join("value"), fs::Permissions::from_mode(0o755))
            .expect("change mode"),
        "special-mode" => {
            fs::set_permissions(input.join("value"), fs::Permissions::from_mode(0o4644))
                .expect("change special permission bits");
        }
        "symlink" => {
            fs::remove_file(input.join("value")).expect("unlink");
            std::os::unix::fs::symlink("/etc/passwd", input.join("value")).expect("symlink");
        }
        "output-link" => {
            std::os::unix::fs::symlink("/etc", output.join("escape")).expect("output symlink");
        }
        "descendant" => {
            // SAFETY: the child uses only async-signal-safe libc operations and _exit after fork.
            let pid = unsafe { nix::libc::fork() };
            assert!(pid >= 0);
            if pid == 0 {
                // SAFETY: no Rust runtime activity in the forked child; deliberately retain pipes.
                unsafe {
                    nix::libc::setsid();
                    loop {
                        nix::libc::pause();
                    }
                }
            }
            fs::write(output.join("descendant"), pid.to_string()).expect("descendant pid");
        }
        "missing" => return,
        "directory-output" => {
            fs::create_dir(output.join("nested")).expect("output directory");
            fs::write(output.join("nested/value"), b"selected nested output")
                .expect("nested output");
        }
        "fail" => std::process::exit(7),
        _ => {}
    }
    fs::write(
        output.join("answer"),
        fs::read(input.join("value")).unwrap_or_default(),
    )
    .expect("answer");
}

#[test]
fn native_fixed_candidate_and_identity() {
    let mut fixture = Fixture::new("sync");
    let mut running = Running(Some(
        fixture
            .command("run")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn Ark"),
    ));
    ready(
        running.0.as_mut().expect("running"),
        &fixture.run("run").join("output/ready"),
    );
    fs::write(fixture.source.join("value"), b"B edited while A ran\n").expect("independent B");
    // Original executable replacement cannot replace Ark's retained verified program.
    let replacement = fixture.temp.path().join("replacement");
    fs::write(&replacement, b"wrong executable").expect("replacement");
    fs::rename(replacement, &fixture.artifact).expect("replace original artifact");
    fs::write(fixture.run("run").join("output/continue"), b"continue").expect("continue");
    let outcome = result(
        running
            .0
            .take()
            .expect("child")
            .wait_with_output()
            .expect("finish"),
    );
    assert_eq!(outcome["key"]["input"]["cid"], fixture.root.to_string());
    assert_eq!(outcome["key"]["input"]["form"], "projected");
    assert!(outcome["record"].is_null());
    assert_eq!(outcome["hit"], false);
    let root = BlobCid::parse(outcome["output"].as_str().expect("output root")).expect("CID");
    let storage = eprfs_storage::DirectoryStorage::open(&fixture.store).expect("store");
    let restored = fixture.temp.path().join("restored-output");
    futures::executor::block_on(eprfs_local::restore_exact_tree(
        &storage,
        &root,
        &restored,
        FetchPolicy::LocalOnly,
        &TreeLimits::default(),
        &eprfs_host::HostProfile::current_platform(),
    ))
    .expect("restore output");
    assert_eq!(
        fs::read(restored.join("answer")).expect("answer"),
        b"A captured\n"
    );
    assert_eq!(
        fs::read_dir(&restored).expect("output inventory").count(),
        1
    );
    assert_eq!(
        fs::read(fixture.source.join("value")).expect("B"),
        b"B edited while A ran\n"
    );
    let b = futures::executor::block_on(eprfs_local::capture_tree(
        &fixture.source,
        &fixture.store,
        &["value".into()],
        &TreeLimits::default(),
    ))
    .expect("capture B");
    assert_ne!(fixture.root, b.root);
    for dir in [
        fixture.run("run"),
        fixture.run("run").join("input"),
        fixture.run("run").join("home"),
        fixture.run("run").join("output"),
    ] {
        assert_eq!(
            fs::metadata(dir)
                .expect("private directory")
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
    }
    let original: Value = serde_json::from_slice(
        &fs::read(fixture.run("run").join("spec.json")).expect("original spec"),
    )
    .expect("spec");
    assert_eq!(original["child"]["argv"], fixture.request["child"]["argv"]);
    fixture.request["child"]["env"]["FIXTURE_MODE"] = json!("plain");
    fs::copy(
        std::env::current_exe().expect("fixture executable"),
        &fixture.artifact,
    )
    .expect("restore artifact");
    let changed = result(
        fixture
            .command("changed-method")
            .output()
            .expect("changed method"),
    );
    assert_ne!(outcome["key"]["method"], changed["key"]["method"]);
    fixture.root = b.root;
    let b_result = result(fixture.command("b-run").output().expect("B run"));
    assert_ne!(outcome["key"]["input"], b_result["key"]["input"]);
}

#[test]
fn native_method_identity_changes_with_each_declared_execution_input() {
    let mut fixture = Fixture::new("plain");
    let original = fixture.request.clone();
    let baseline = result(
        fixture
            .command("identity-baseline")
            .output()
            .expect("baseline"),
    );
    for (label, request) in [
        ("argv", {
            let mut value = original.clone();
            value["child"]["argv"]
                .as_array_mut()
                .expect("argv")
                .push(json!("--test-threads=1"));
            value
        }),
        ("env", {
            let mut value = original.clone();
            value["child"]["env"]["DECLARED_VALUE"] = json!("new explicit value");
            value
        }),
        ("outputs", {
            let mut value = original.clone();
            value["outputs"] = json!(["answer", "started"]);
            value
        }),
    ] {
        fixture.request = request;
        let changed = result(
            fixture
                .command(label)
                .output()
                .expect("changed declaration"),
        );
        assert_eq!(changed["key"]["input"], baseline["key"]["input"]);
        assert_ne!(
            changed["key"]["method"], baseline["key"]["method"],
            "{label}"
        );
        assert!(changed["record"].is_null());
        assert_eq!(changed["hit"], false);
    }
    // Linux permits trailing data in an ELF executable. Change its actual bytes,
    // keep its behavior, and explicitly pin the new digest rather than a pathname.
    fixture.request = original;
    let mut bytes = fs::read(&fixture.artifact).expect("executable");
    bytes.extend_from_slice(b"distinct pinned artifact");
    fs::write(&fixture.artifact, &bytes).expect("new artifact bytes");
    fixture.request["child"]["artifact"]["pinned"]["sha256"] =
        json!(hex::encode(Sha256::digest(&bytes)));
    let changed = result(fixture.command("artifact").output().expect("new artifact"));
    assert_eq!(changed["key"]["input"], baseline["key"]["input"]);
    assert_ne!(changed["key"]["method"], baseline["key"]["method"]);
    assert!(changed["record"].is_null());
}

#[test]
fn native_preflight_refusals_spawn_nothing() {
    let mut f = Fixture::new("plain");
    let good = f.request.clone();
    f.request["child"]["artifact"]["pinned"]["sha256"] = json!("0".repeat(64));
    f.refusal("digest", "digest mismatch");
    f.request = good.clone();
    f.request["child"]["argv"][0] = json!("{unknown}");
    f.refusal("placeholder", "unknown native placeholder");
    f.request = good.clone();
    f.request["method"]["name"] = json!("a2o-stage:native");
    f.refusal("scope", "OwnEnvelope");
    f.request = good.clone();
    f.request["child"]["env"]
        .as_object_mut()
        .expect("env")
        .remove("PATH");
    f.refusal("path", "explicit PATH");
    f.request = good.clone();
    f.request["child"]["env"]["HOME"] = json!("/tmp");
    f.refusal("home", "private home");
    f.request = good;
    for flag in ["--repo", "--reuse", "--data-root", "--material"] {
        let output = f
            .command("mixed")
            .arg(flag)
            .arg("anything")
            .output()
            .expect("mixed options");
        assert!(!output.status.success());
        assert!(!f.run("mixed").exists());
    }
    fs::create_dir(f.run("exists")).expect("existing run");
    fs::write(f.run("exists").join("sentinel"), b"preserve").expect("sentinel");
    f.refusal("exists", "exists");
    assert_eq!(
        fs::read(f.run("exists").join("sentinel")).expect("sentinel"),
        b"preserve"
    );
    let missing = eprfs_core::encode_tree(
        &eprfs_core::TreeNode::new(Vec::new()).expect("empty tree"),
        &TreeLimits::default(),
    )
    .expect("encode")
    .0;
    f.root = missing;
    let output = f.command("missing-tree").output().expect("missing closure");
    assert!(!output.status.success());
    assert!(!f.run("missing-tree").exists());
}

#[test]
fn native_mutation_missing_output_and_descendants_refuse() {
    for mode in [
        "add",
        "change",
        "delete",
        "mode",
        "special-mode",
        "symlink",
        "missing",
        "descendant",
    ] {
        let f = Fixture::new(mode);
        let output = f.command("run").output().expect("adversarial fixture");
        assert!(
            !output.status.success(),
            "{mode}: {}",
            String::from_utf8_lossy(&output.stdout)
        );
        let statuses = fs::read_to_string(f.run("run").join("status.jsonl")).expect("statuses");
        assert!(!statuses.contains("\"answered\""), "{mode}: {statuses}");
        if mode == "descendant" {
            let pid = fs::read_to_string(f.run("run").join("output/descendant")).expect("pid");
            assert!(
                !Path::new("/proc").join(pid).exists(),
                "surviving descendant"
            );
        }
    }
}

#[test]
fn native_interrupt_is_unanswered() {
    let f = Fixture::new("interrupt");
    let mut running = Running(Some(
        f.command("run")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn"),
    ));
    let child = running.0.as_mut().expect("child");
    ready(child, &f.run("run").join("output/ready"));
    nix::sys::signal::kill(
        nix::unistd::Pid::from_raw(child.id() as i32),
        nix::sys::signal::Signal::SIGTERM,
    )
    .expect("interrupt");
    let output = running
        .0
        .take()
        .expect("child")
        .wait_with_output()
        .expect("reaped");
    assert_eq!(output.status.code(), Some(143));
    assert!(output.stdout.is_empty());
    let status = fs::read_to_string(f.run("run").join("status.jsonl")).expect("status");
    assert!(status.contains("unanswered"));
    assert!(!status.contains("\"answered\""));
}

#[test]
fn native_spec_rejects_narrow_view_and_preserves_method_identity() {
    let f = Fixture::new("plain");
    let request = f.temp.path().join("request.json");
    fs::write(&request, serde_json::to_vec(&f.request).expect("request")).expect("request file");
    let output = ark()
        .args(["task", "spec", "--root", &f.root.to_string()])
        .arg("--store")
        .arg(&f.store)
        .arg("--request")
        .arg(&request)
        .output()
        .expect("spec command");
    let declared = result(output);
    let original: ark_core::TaskSpec =
        serde_json::from_value(declared["spec"].clone()).expect("checked spec");
    let path = f.temp.path().join("spec.json");
    fs::write(&path, serde_json::to_vec(&original).expect("original JSON")).expect("spec file");
    let run = |label: &str| {
        ark()
            .args(["task", "run"])
            .arg("--spec")
            .arg(&path)
            .arg("--store")
            .arg(&f.store)
            .arg("--artifact")
            .arg(&f.artifact)
            .arg("--run-dir")
            .arg(f.run(label))
            .output()
            .expect("run spec")
    };
    let outcome = result(run("whole"));
    assert_eq!(outcome["key"]["method"], declared["derivation"]["method"]);
    let mut method = original.method().clone();
    method.view = eprfs_core::View::new([eprfs_core::ProjectionPath::new("value").expect("path")])
        .expect("view")
        .paths()
        .cloned()
        .collect();
    let narrowed = ark_core::TaskSpec::new(
        method,
        f.root.clone(),
        None,
        original.child().clone(),
        None,
        original.outputs().to_vec(),
    )
    .expect("valid narrowed task for other carriers");
    fs::write(&path, serde_json::to_vec(&narrowed).expect("narrow JSON")).expect("narrow spec");
    let refused = run("narrow");
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("whole view"));
    assert!(!f.run("narrow").exists());
}

#[test]
fn native_corrupt_or_missing_leaf_and_output_escape_cannot_succeed() {
    use eprfs_core::EprfsStorage;
    for corrupt in [false, true] {
        let f = Fixture::new("plain");
        let storage = eprfs_storage::DirectoryStorage::open(&f.store).expect("store");
        let tree = futures::executor::block_on(eprfs_core::load_verified_tree(
            &storage,
            &f.root,
            FetchPolicy::LocalOnly,
            &TreeLimits::default(),
        ))
        .expect("tree");
        let leaf = tree.leaves.keys().next().expect("leaf");
        let expected =
            futures::executor::block_on(storage.fetch_blob(leaf, FetchPolicy::LocalOnly))
                .expect("leaf bytes");
        // DirectoryStorage's layout is an implementation detail: locate by exact retained bytes.
        fn find(root: &Path, bytes: &[u8]) -> Option<PathBuf> {
            for entry in fs::read_dir(root).ok()? {
                let path = entry.ok()?.path();
                if path.is_dir() {
                    if let Some(found) = find(&path, bytes) {
                        return Some(found);
                    }
                } else if fs::read(&path).ok().as_deref() == Some(bytes) {
                    return Some(path);
                }
            }
            None
        }
        let file = find(&f.store, &expected.bytes).expect("stored leaf");
        if corrupt {
            fs::write(file, b"corrupt bytes").expect("corrupt fixture");
        } else {
            fs::remove_file(file).expect("missing fixture");
        }
        let output = f.command("run").output().expect("closure refusal");
        assert!(!output.status.success());
        assert!(!f.run("run").exists());
    }
    let mut f = Fixture::new("output-link");
    f.request["outputs"] = json!(["escape/passwd"]);
    let output = f.command("escape").output().expect("output escape");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("missing declared output"));
    assert!(!fs::read_to_string(f.run("escape").join("status.jsonl"))
        .expect("status")
        .contains("\"answered\""));
}

#[test]
fn native_declared_directory_captures_only_its_leaves_and_failed_exit_has_no_output() {
    let mut f = Fixture::new("plain");
    f.request["child"]["env"]["FIXTURE_MODE"] = json!("directory-output");
    f.request["outputs"] = json!(["nested"]);
    let output = result(f.command("directory").output().expect("directory outputs"));
    let storage = eprfs_storage::DirectoryStorage::open(&f.store).expect("store");
    let tree = futures::executor::block_on(eprfs_core::load_verified_tree(
        &storage,
        &BlobCid::parse(output["output"].as_str().expect("root")).expect("CID"),
        FetchPolicy::LocalOnly,
        &TreeLimits::default(),
    ))
    .expect("output closure");
    assert_eq!(tree.root_node().entries.len(), 1);
    assert_eq!(tree.root_node().entries[0].name, b"nested");
    let f = Fixture::new("fail");
    let output = f.command("failed").output().expect("failed exit");
    assert_eq!(output.status.code(), Some(7));
    let outcome: Value = serde_json::from_slice(&output.stdout).expect("failed TaskOutcome");
    assert!(outcome["output"].is_null());
    assert!(outcome["record"].is_null());
}
