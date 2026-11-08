//! `epr artifact` and `epr peer artifacts` (artifact-store design §3): a recorded build is one
//! custody Produce that both stays this device's local custody and crosses as the
//! `artifact-produce@1` record; a sibling reads only admitted siblings' records for its key; and a
//! fetch with no daemon is a typed gap that names its cure.
//!
//! Every key is a temp file and every child gets its own `XDG_CONFIG_HOME`, payload store and
//! (stub) `brit-build-ref`, so no test touches this workspace's real device, peer key or notes.
#![cfg(feature = "peer")]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use elohim_epr_cli::actor::{
    device_authorize, device_bind, device_endpoint, device_enroll, witness,
};
use elohim_epr_cli::device_key::{self, DeviceKey};
use elohim_epr_rea::{
    atom_cid, AgentRef, FlowEvent, FlowRecord, FlowStore, Magnitude, ReaVerb, SidecarFlowStore,
};
use eprfs_core::{BlobCid, MethodDescriptor, TreePath};
use eprfs_peer::{open_payload_store, BlobMount, CrossingAllowList};
use tempfile::TempDir;

const HANDLE: &str = "matthew";
const AGENT_ID: &str = "4f5e3d2c1b0a99887766554433221100ffeeddccbbaa00998877665544332211";

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

fn checkout() -> TempDir {
    let dir = TempDir::new().unwrap();
    std::fs::create_dir_all(dir.path().join(".epr-meta")).unwrap();
    std::fs::write(
        dir.path().join(".epr-meta/repository.yaml"),
        "version: 1\nagent: repo:ethosengine/elohim\n",
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join("crate/src")).unwrap();
    std::fs::write(dir.path().join("crate/src/main.rs"), "fn main() {}\n").unwrap();
    std::fs::write(dir.path().join("README.md"), "fixture\n").unwrap();
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["add", "-A"]);
    git(dir.path(), &["commit", "-q", "-m", "fixture"]);
    dir
}

/// One device: its device key file, its config home (peer key, fetch queue), payload store and
/// checkout.
struct Device {
    key_file: PathBuf,
    key: DeviceKey,
    config: TempDir,
    checkout: TempDir,
}

impl Device {
    fn new(keys: &TempDir, name: &str) -> Device {
        let key_file = keys.path().join(name).join("ed25519.seed");
        let key = DeviceKey::load_or_generate(&key_file).unwrap();
        Device {
            key_file,
            key,
            config: TempDir::new().unwrap(),
            checkout: checkout(),
        }
    }

    fn payload(&self) -> PathBuf {
        self.config.path().join("payload")
    }

    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_epr"));
        // The build environment `epr artifact key|put` judges is this child's: none of the test
        // harness's own cargo variables reach it.
        for (name, _) in std::env::vars() {
            if name.starts_with("CARGO_")
                || name.starts_with("RUST")
                || elohim_epr_cli::artifact::NATIVE_BUILD_ENV.contains(&name.as_str())
            {
                command.env_remove(name);
            }
        }
        command
            .env(device_key::DEVICE_KEY_ENV, &self.key_file)
            .env("XDG_CONFIG_HOME", self.config.path())
            .env("EPR_STORE", self.config.path().join("epr-store"))
            .env("EPR_PAYLOAD_STORE", self.payload())
            .env("EPR_BIN", env!("CARGO_BIN_EXE_epr"))
            .env_remove("EPR_PEER_RELAY_URL")
            .env_remove("CLAUDE_CODE_SESSION_ID")
            .env_remove("CLAUDE_SESSION_ID")
            .env_remove("ELOHIM_SESSION_ID");
        command
    }

    fn epr(&self, args: &[&str]) -> Output {
        self.command().args(args).output().expect("epr runs")
    }

    fn root(&self) -> &Path {
        self.checkout.path()
    }

    fn node_id(&self) -> String {
        let out = self.epr(&["peer", "id"]);
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout).unwrap().trim().to_string()
    }
}

/// Devices A (the chain root) and B (bound), each with an endpoint row, the roster carried to B.
fn household() -> (TempDir, Device, Device) {
    let keys = TempDir::new().unwrap();
    let a = Device::new(&keys, "a");
    let b = Device::new(&keys, "b");
    let root = a.root();
    witness(
        root,
        "human:matthew",
        "agent:orchestrator@claude-opus-5-5",
        "s1",
        "operator of this stewarded device; present in this session",
        false,
        &a.key,
    )
    .unwrap();
    let request = device_enroll(root, HANDLE, &b.key).unwrap();
    let authorization =
        device_authorize(root, &serde_json::to_string(&request).unwrap(), &a.key).unwrap();
    device_bind(
        root,
        &serde_json::to_string(&authorization).unwrap(),
        &b.key,
    )
    .unwrap();
    device_endpoint(root, None, &a.node_id(), &a.key).unwrap();
    device_endpoint(root, None, &b.node_id(), &b.key).unwrap();
    let rel = format!(".eprfs/status/participants/{HANDLE}.jsonl");
    std::fs::create_dir_all(b.root().join(".eprfs/status/participants")).unwrap();
    std::fs::copy(root.join(&rel), b.root().join(&rel)).unwrap();
    (keys, a, b)
}

/// A `mesh-build` descriptor whose view is `paths`, written outside the checkout.
fn method_file(dir: &Path, paths: &[&str]) -> (PathBuf, MethodDescriptor) {
    method_file_with(dir, paths, &[])
}

/// [`method_file`] with extra non-path inputs.
fn method_file_with(
    dir: &Path,
    paths: &[&str],
    inputs: &[(&str, &str)],
) -> (PathBuf, MethodDescriptor) {
    let mut non_path_inputs: std::collections::BTreeMap<String, String> =
        [("RUSTFLAGS".to_string(), String::new())].into();
    for (name, value) in inputs {
        non_path_inputs.insert((*name).to_string(), (*value).to_string());
    }
    let descriptor = MethodDescriptor {
        name: "mesh-build".into(),
        tools: [("toolchain".to_string(), "1.96.1".to_string())].into(),
        flags: vec!["--bin=elohim-storage".into(), "--profile=dev".into()],
        params: None,
        exec: None,
        step: Some("mesh-build:elohim-storage".into()),
        embedder: None,
        view: paths
            .iter()
            .map(|p| TreePath::from_slash_bytes(p.as_bytes()).unwrap())
            .collect(),
        non_path_inputs,
        deterministic: false,
    };
    let path = dir.join("method.json");
    std::fs::write(&path, serde_json::to_string_pretty(&descriptor).unwrap()).unwrap();
    (path, descriptor)
}

/// A stub `brit-build-ref`: `signer --json` names a signer, `build put` succeeds; every call is
/// logged one line per invocation.
fn brit_stub(dir: &Path) -> (PathBuf, PathBuf) {
    let log = dir.join("brit.log");
    let stub = dir.join("brit-build-ref");
    std::fs::write(
        &stub,
        format!(
            "#!/bin/sh\necho \"$*\" >> '{}'\ncase \"$*\" in\n  *signer*) echo '{{\"agentId\":\"{AGENT_ID}\"}}' ;;\n  *) echo '{{}}' ;;\nesac\n",
            log.display()
        ),
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();
    (stub, log)
}

fn text(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

fn events(root: &Path) -> Vec<FlowEvent> {
    if !root.join(".eprfs/status/flows.jsonl").is_file() {
        return Vec::new();
    }
    SidecarFlowStore::open(root)
        .unwrap()
        .events()
        .unwrap()
        .into_iter()
        .map(|(_, event)| event)
        .collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn put_records_one_produce_that_both_stays_local_custody_and_crosses() {
    let keys = TempDir::new().unwrap();
    let a = Device::new(&keys, "a");
    let scratch = TempDir::new().unwrap();
    let (method, descriptor) = method_file(scratch.path(), &["crate/src", "README.md"]);
    let (stub, log) = brit_stub(scratch.path());
    let bytes = scratch.path().join("elohim-storage");
    let artifact: Vec<u8> = (0..700_000u32).map(|i| (i * 31 % 251) as u8).collect();
    std::fs::write(&bytes, &artifact).unwrap();
    let repo = a.root().to_str().unwrap().to_string();
    let method = method.to_str().unwrap().to_string();

    let out = a.epr(&[
        "artifact", "key", "--method", &method, "--repo", &repo, "--json",
    ]);
    assert!(out.status.success(), "{}", text(&out));
    let key: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let input = key["input"].as_str().unwrap().to_string();
    let key_cid = key["keyCid"].as_str().unwrap().to_string();
    assert_eq!(key["basis"], "basis-key:git-tree");
    assert_eq!(key["method"], descriptor.cid().unwrap().to_string());
    assert!(
        key_cid.starts_with("bafyrei"),
        "the key is a DAG-CBOR CID: {key_cid}"
    );

    let put = |input: &str| {
        a.command()
            .env("BRIT_BUILD_REF_BIN", &stub)
            .args([
                "artifact",
                "put",
                "--method",
                &method,
                "--input",
                input,
                "--bytes",
                bytes.to_str().unwrap(),
                "--project",
                "elohim-storage",
                "--duration-ms",
                "1234",
                "--cross",
                "--repo",
                &repo,
                "--json",
            ])
            .output()
            .unwrap()
    };
    let out = put(&input);
    assert!(out.status.success(), "{}", text(&out));
    let recorded: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let output = BlobCid::compute_bytes(&artifact).to_string();
    assert_eq!(recorded["output"], output);
    assert_eq!(recorded["key"], key_cid);
    let manifest = recorded["manifest"].as_str().unwrap().to_string();

    // brit's attestation is keyed by the DerivationKey CID, never a sha256 hex.
    let calls = std::fs::read_to_string(&log).unwrap();
    let build = calls
        .lines()
        .find(|line| line.contains("build put"))
        .expect("build put");
    for part in [
        "--step mesh-build:elohim-storage".to_string(),
        format!("--inputs-hash {key_cid}"),
        format!("--output {output}"),
        format!("--manifest {}", descriptor.cid().unwrap()),
        "--duration-ms 1234".to_string(),
    ] {
        assert!(build.contains(&part), "{part} in {build}");
    }

    // Exactly one Produce: this device's custody, carrying the crossing record's fields.
    let all = events(a.root());
    let produces: Vec<&FlowEvent> = all
        .iter()
        .filter(|e| e.action == ReaVerb::Produce)
        .collect();
    assert_eq!(produces.len(), 1, "{all:?}");
    let produce = produces[0];
    let did = a.key.did_key();
    assert_eq!(produce.receiver, AgentRef(did.clone()));
    assert_eq!(produce.provider, AgentRef(AGENT_ID.into()));
    assert_eq!(produce.resource.to_string(), output);
    assert_eq!(produce.classified_as[0], "custody-blob");
    assert_eq!(produce.classified_as[1], output);
    for slot in [
        format!("manifest:{manifest}"),
        "copy:self".into(),
        "measure:artifact-produce@1".into(),
        "reason:produce".into(),
        "basis-key:git-tree".into(),
        format!("env:key={key_cid}"),
        format!("env:input={input}"),
        format!("env:output={output}"),
        format!("env:bytes={}", artifact.len()),
        format!("env:agentId={AGENT_ID}"),
        "env:project=elohim-storage".into(),
    ] {
        assert!(
            produce.classified_as.contains(&slot),
            "{slot} in {:?}",
            produce.classified_as
        );
    }

    // It crosses under the repository's own allow-list …
    let policy =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../genesis/agentic/pool-policy.json");
    let allow = CrossingAllowList::from_policy_file(&policy).unwrap();
    assert!(allow.admits(produce), "artifact-produce@1 produce crosses");

    // … and it is this device's local custody: the serve-blob mount indexes it from this event.
    let store = open_payload_store(&a.payload()).unwrap();
    let (mount, _served) = BlobMount::new(store, did.clone());
    let indexed: Vec<_> = SidecarFlowStore::open(a.root()).unwrap().events().unwrap();
    let unindexed = mount.rebuild_from_flow(&indexed).await;
    assert!(unindexed.is_empty(), "{unindexed:?}");
    let held = mount
        .held(&BlobCid::parse(&output).unwrap())
        .expect("indexed as custody");
    assert_eq!(held.manifest.to_string(), manifest);

    // A closure that moved since the build started is never recorded.
    let out = put("bafkr4ihkr4ld3m4gqkjf4reryxsy2s5tkbxprqkow6fin2iiyvreuzzab4");
    assert_eq!(out.status.code(), Some(3), "{}", text(&out));
    assert!(
        text(&out).contains("unrecorded: the closure moved"),
        "{}",
        text(&out)
    );
    // Nor a dirty one: the bytes would not be HEAD's.
    std::fs::write(
        a.root().join("crate/src/main.rs"),
        "fn main() { todo!() }\n",
    )
    .unwrap();
    let out = put(&input);
    assert_eq!(out.status.code(), Some(3), "{}", text(&out));
    assert!(text(&out).contains("unrecorded:"), "{}", text(&out));
    assert!(text(&out).contains("uncommitted changes"), "{}", text(&out));
    assert_eq!(
        events(a.root())
            .iter()
            .filter(|e| e.action == ReaVerb::Produce)
            .count(),
        1,
        "refusals record nothing"
    );
}

fn produce_record(key: &str, output: &BlobCid, manifest: &str, via: Option<&str>) -> FlowEvent {
    let mut classified_as = vec![
        "custody-blob".to_string(),
        output.to_string(),
        format!("manifest:{manifest}"),
        "copy:self".into(),
        "measure:artifact-produce@1".into(),
        "reason:produce".into(),
        "basis-key:git-tree".into(),
        "env:project=elohim-storage".into(),
        format!("env:key={key}"),
        format!("env:output={output}"),
        "env:bytes=4096".into(),
        "env:commit=0123456789abcdef".into(),
        format!("env:agentId={AGENT_ID}"),
    ];
    if let Some(device) = via {
        classified_as.push(format!("via:{device}"));
    }
    FlowEvent {
        action: ReaVerb::Produce,
        provider: AgentRef(AGENT_ID.into()),
        receiver: AgentRef(via.unwrap_or("did:key:zSelf").into()),
        resource: *output.as_cid(),
        quantity: Magnitude::Count {
            value: 4096.0,
            unit: "bytes".into(),
        },
        process: None,
        in_scope_of: atom_cid(&"repo-scope".to_string()).unwrap(),
        fulfills: Vec::new(),
        satisfies: Vec::new(),
        occurred_at: "2026-11-08T10:00:00Z".into(),
        classified_as,
    }
}

#[test]
fn peer_artifacts_lists_only_admitted_siblings_records_for_the_key() {
    let (keys, a, b) = household();
    let a_did = a.key.did_key();
    let stranger = Device::new(&keys, "stranger-artifact");
    let key = "bafyreigq2n5xqgbtzbb6ce2ebobfqvqtjqdmmqkn3mmdyx5n6k4hgemrzu";
    let other_key = "bafyreihdwdcefgh4dqkjv67uzcmw7ojee6xedzdetojuzjevtenxquvyku";
    let output = BlobCid::compute_bytes(b"the storage binary");
    let manifest = BlobCid::compute(b"\xa0").to_string();
    let mut store = SidecarFlowStore::open(b.root()).unwrap();
    for event in [
        produce_record(key, &output, &manifest, Some(&a_did)),
        produce_record(other_key, &output, &manifest, Some(&a_did)),
        produce_record(key, &output, &manifest, None),
        produce_record(key, &output, &manifest, Some(&stranger.key.did_key())),
        produce_record(key, &output, &manifest, Some(&b.key.did_key())),
    ] {
        store.append(FlowRecord::Event(event)).unwrap();
    }
    // A record whose resource and named output disagree is no record.
    let mut inconsistent = produce_record(key, &output, &manifest, Some(&a_did));
    inconsistent.resource = *BlobCid::compute_bytes(b"something else").as_cid();
    store.append(FlowRecord::Event(inconsistent)).unwrap();

    let out = b.epr(&[
        "peer",
        "artifacts",
        "--key",
        key,
        "--json",
        "--root",
        b.root().to_str().unwrap(),
    ]);
    assert!(out.status.success(), "{}", text(&out));
    let rows: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(
        rows,
        serde_json::json!([{
            "device": a_did,
            "output": output.to_string(),
            "manifest": manifest,
            "bytes": 4096,
            "commit": "0123456789abcdef",
            "agentId": AGENT_ID,
            "at": "2026-11-08T10:00:00Z"
        }])
    );
    let out = b.epr(&["peer", "artifacts", "--root", b.root().to_str().unwrap()]);
    assert!(!out.status.success(), "--key is required");
}

#[test]
fn fetch_without_a_daemon_is_daemon_down_with_a_cure() {
    let keys = TempDir::new().unwrap();
    let a = Device::new(&keys, "a");
    let scratch = TempDir::new().unwrap();
    let (method, _) = method_file(scratch.path(), &["crate/src"]);
    let dest = scratch.path().join("mesh-bin/elohim-storage");
    let out = a.epr(&[
        "artifact",
        "fetch",
        "--method",
        method.to_str().unwrap(),
        "--repo",
        a.root().to_str().unwrap(),
        "--dest",
        dest.to_str().unwrap(),
    ]);
    assert_eq!(out.status.code(), Some(3), "{}", text(&out));
    let said = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(said.contains("fetch gap: daemon-down"), "{said}");
    assert!(said.contains("cure: start `epr peer serve`"), "{said}");
    assert!(!dest.exists(), "nothing installed");
    assert!(
        !a.config.path().join("epr/peer/fetch-queue").exists(),
        "nothing queued for a daemon that is not there"
    );
}

/// `epr artifact key --print key` for `method` in `device`'s checkout, with `env` set.
fn key_with(device: &Device, method: &Path, env: &[(&str, &str)]) -> Output {
    let mut command = device.command();
    for (name, value) in env {
        command.env(name, value);
    }
    command
        .args([
            "artifact",
            "key",
            "--method",
            method.to_str().unwrap(),
            "--repo",
            device.root().to_str().unwrap(),
            "--print",
            "key",
        ])
        .output()
        .unwrap()
}

fn key_of(device: &Device, method: &Path, env: &[(&str, &str)]) -> String {
    let out = key_with(device, method, env);
    assert!(out.status.success(), "{}", text(&out));
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

#[test]
fn an_unrecorded_cargo_env_var_refuses_the_key() {
    let keys = TempDir::new().unwrap();
    let a = Device::new(&keys, "a");
    let scratch = TempDir::new().unwrap();
    let (method, _) = method_file_with(scratch.path(), &["crate/src"], &[("CC", "gcc")]);
    // The recorded environment keys.
    key_of(&a, &method, &[("CC", "gcc"), ("CARGO_BUILD_JOBS", "1")]);
    for (name, value) in [
        ("CARGO_ENCODED_RUSTFLAGS", "-Copt-level=3"),
        ("CARGO_PROFILE_DEV_OPT_LEVEL", "3"),
        ("RUSTC_WRAPPER", "sccache"),
        ("RUSTFLAGS", "--cfg other"),
    ] {
        let out = key_with(&a, &method, &[("CC", "gcc"), (name, value)]);
        assert_eq!(out.status.code(), Some(3), "{name}: {}", text(&out));
        assert!(text(&out).contains(name), "{name}: {}", text(&out));
    }
    // A recorded variable that is not set, or set otherwise, refuses too.
    let out = key_with(&a, &method, &[]);
    assert_eq!(out.status.code(), Some(3), "{}", text(&out));
    assert!(text(&out).contains("CC=gcc"), "{}", text(&out));
    let out = key_with(&a, &method, &[("CC", "clang")]);
    assert_eq!(out.status.code(), Some(3), "{}", text(&out));
}

#[test]
fn an_ancestor_cargo_config_moves_the_key() {
    let keys = TempDir::new().unwrap();
    let a = Device::new(&keys, "a");
    let scratch = TempDir::new().unwrap();
    let cargo_home = TempDir::new().unwrap();
    let home = cargo_home.path().to_str().unwrap().to_string();
    let (method, _) = method_file_with(scratch.path(), &["crate/src"], &[("crate", "crate")]);
    let env = [("CARGO_HOME", home.as_str())];
    let first = key_of(&a, &method, &env);
    // An ancestor configuration cargo would read, outside the view and untracked.
    std::fs::create_dir_all(a.root().join(".cargo")).unwrap();
    std::fs::write(
        a.root().join(".cargo/config.toml"),
        "[build]\nrustflags = [\"-Copt-level=3\"]\n",
    )
    .unwrap();
    let second = key_of(&a, &method, &env);
    assert_ne!(
        first, second,
        "an ancestor .cargo/config.toml moves the key"
    );
    // The user-level configuration under CARGO_HOME too.
    std::fs::write(
        cargo_home.path().join("config.toml"),
        "[profile.dev]\nopt-level = 1\n",
    )
    .unwrap();
    let third = key_of(&a, &method, &env);
    assert_ne!(second, third, "$CARGO_HOME/config.toml moves the key");
    // And the same configuration keys alike again.
    assert_eq!(third, key_of(&a, &method, &env));
}

#[test]
fn entry_mode_moves_the_key() {
    use std::os::unix::fs::PermissionsExt;
    let keys = TempDir::new().unwrap();
    let a = Device::new(&keys, "a");
    let scratch = TempDir::new().unwrap();
    std::fs::write(a.root().join("crate/build.sh"), "#!/bin/sh\necho hi\n").unwrap();
    git(a.root(), &["add", "crate/build.sh"]);
    git(a.root(), &["commit", "-q", "-m", "a build script"]);
    let (method, _) = method_file(scratch.path(), &["crate/build.sh"]);
    let before = key_of(&a, &method, &[]);
    std::fs::set_permissions(
        a.root().join("crate/build.sh"),
        std::fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    git(a.root(), &["add", "crate/build.sh"]);
    git(a.root(), &["commit", "-q", "-m", "executable"]);
    let after = key_of(&a, &method, &[]);
    assert_ne!(
        before, after,
        "a selected file's mode is part of what was read"
    );
}

#[test]
fn an_out_of_view_symlink_refuses() {
    let keys = TempDir::new().unwrap();
    let a = Device::new(&keys, "a");
    let scratch = TempDir::new().unwrap();
    let (method, _) = method_file(scratch.path(), &["crate/src"]);
    std::os::unix::fs::symlink("main.rs", a.root().join("crate/src/inside")).unwrap();
    git(a.root(), &["add", "-A"]);
    git(a.root(), &["commit", "-q", "-m", "an inside link"]);
    key_of(&a, &method, &[]);
    std::os::unix::fs::symlink("../../README.md", a.root().join("crate/src/escape")).unwrap();
    git(a.root(), &["add", "-A"]);
    git(a.root(), &["commit", "-q", "-m", "an escaping link"]);
    let out = key_with(&a, &method, &[]);
    assert_eq!(out.status.code(), Some(3), "{}", text(&out));
    assert!(
        text(&out).contains("points outside the method's view"),
        "{}",
        text(&out)
    );
}

#[test]
fn an_index_flag_refuses() {
    let keys = TempDir::new().unwrap();
    let a = Device::new(&keys, "a");
    let scratch = TempDir::new().unwrap();
    let (method, _) = method_file(scratch.path(), &["crate/src"]);
    key_of(&a, &method, &[]);
    for (set, clear) in [
        ("--assume-unchanged", "--no-assume-unchanged"),
        ("--skip-worktree", "--no-skip-worktree"),
    ] {
        git(a.root(), &["update-index", set, "crate/src/main.rs"]);
        let out = key_with(&a, &method, &[]);
        assert_eq!(out.status.code(), Some(3), "{set}: {}", text(&out));
        assert!(text(&out).contains("index flags"), "{set}: {}", text(&out));
        git(a.root(), &["update-index", clear, "crate/src/main.rs"]);
    }
    // An ignored file inside the view refuses as well: the build could read it.
    std::fs::write(a.root().join(".gitignore"), "*.generated\n").unwrap();
    git(a.root(), &["add", ".gitignore"]);
    git(a.root(), &["commit", "-q", "-m", "ignore"]);
    std::fs::write(a.root().join("crate/src/schema.generated"), "x").unwrap();
    let out = key_with(&a, &method, &[]);
    assert_eq!(out.status.code(), Some(3), "{}", text(&out));
    assert!(text(&out).contains("ignored files"), "{}", text(&out));
}

#[test]
fn a_completion_for_another_request_is_never_accepted() {
    use eprfs_peer::{FetchDone, FetchFor, FetchGap};
    let queue = TempDir::new().unwrap();
    let ours = FetchFor {
        nonce: "0123456789abcdef0123456789abcdef".into(),
        key_cid: BlobCid::compute(b"\xa1\x61k\x61v"),
        output: BlobCid::compute_bytes(b"the binary"),
        dest: PathBuf::from("/pool/mesh-bin/elohim-storage"),
    };
    let done_path = eprfs_peer::fetch_done_path(queue.path(), "our-stem");
    let installed = |for_request: FetchFor| FetchDone::Installed {
        for_request,
        manifest: BlobCid::compute(b"\xa0"),
        manifest_from: "did:key:zA".into(),
        fetched_from: vec!["did:key:zA".into()],
        chunks_fetched: 1,
        chunks_resumed: 0,
        bytes_fetched: 10,
        wasted: 0,
        seconds: 0.1,
    };
    let wait = std::time::Duration::from_millis(600);
    // Another request's answer, at our very path, differing in each field: never taken for ours.
    let others = [
        FetchFor {
            nonce: "ffffffffffffffffffffffffffffffff".into(),
            ..ours.clone()
        },
        FetchFor {
            dest: PathBuf::from("/elsewhere/bin"),
            ..ours.clone()
        },
        FetchFor {
            output: BlobCid::compute_bytes(b"another binary"),
            ..ours.clone()
        },
        FetchFor {
            key_cid: BlobCid::compute(b"\xa1\x61k\x61w"),
            ..ours.clone()
        },
    ];
    for other in others {
        eprfs_peer::write_queue_file(&done_path, &installed(other.clone())).unwrap();
        assert!(
            elohim_epr_cli::artifact::wait_for_answer(&done_path, &ours, wait).is_none(),
            "an answer for {other:?} was taken for ours"
        );
    }
    // Ours is.
    eprfs_peer::write_queue_file(
        &done_path,
        &FetchDone::gap(ours.clone(), FetchGap::NoRecord),
    )
    .unwrap();
    let done = elohim_epr_cli::artifact::wait_for_answer(&done_path, &ours, wait).expect("ours");
    assert_eq!(done.for_request(), &ours);
    assert!(!done_path.exists(), "a taken answer is removed");
}
