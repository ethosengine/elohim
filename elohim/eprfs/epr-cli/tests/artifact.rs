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
use eprfs_peer::{BlobMount, CrossingAllowList};
use tempfile::TempDir;

#[path = "support/artifact_elf.rs"]
mod fixture;
use fixture::{artifact_elf, wait_for_stub};

#[path = "support/artifact_native_libraries.rs"]
mod native_libraries;

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
        tools: [
            ("toolchain".to_string(), "1.96.1".to_string()),
            ("rustc".to_string(), fixture::RUSTC_IDENTITY.to_string()),
        ]
        .into(),
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
    std::fs::create_dir_all(dir).unwrap();
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
    wait_for_stub(&stub);
    std::fs::remove_file(&log).unwrap();
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
    put_and_mount(false, false).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn artifact_overridden_flags_store_the_keyed_method_and_mount() {
    put_and_mount(true, true).await;
}

async fn put_and_mount(overridden: bool, with_requires: bool) {
    let keys = TempDir::new().unwrap();
    let a = Device::new(&keys, "a");
    let scratch = TempDir::new().unwrap();
    let (method, mut descriptor) = method_file(scratch.path(), &["crate/src", "README.md"]);
    if overridden {
        descriptor.non_path_inputs.insert(
            "CARGO_ENCODED_RUSTFLAGS".into(),
            "-Clink-arg=-fuse-ld=lld".into(),
        );
        descriptor
            .non_path_inputs
            .insert("RUSTFLAGS".into(), "-Clinker=/nonexistent/ignored".into());
        std::fs::write(&method, serde_json::to_vec(&descriptor).unwrap()).unwrap();
    }
    let command = || {
        let mut command = a.command();
        command.envs(descriptor.non_path_inputs.iter());
        command
    };
    let mut effective = descriptor.clone();
    if overridden {
        effective.non_path_inputs.remove("RUSTFLAGS");
        assert_ne!(effective.cid().unwrap(), descriptor.cid().unwrap());
    }
    let method_cid = effective.cid().unwrap();
    let (stub, log) = brit_stub(scratch.path());
    let bytes = scratch.path().join("elohim-storage");
    let mut artifact = artifact_elf(b"Linker: LLD 22.1.2\0");
    artifact.extend((0..700_000u32).map(|i| (i * 31 % 251) as u8));
    std::fs::write(&bytes, &artifact).unwrap();
    let repo = a.root().to_str().unwrap().to_string();
    let method = method.to_str().unwrap().to_string();

    let out = command()
        .args([
            "artifact", "key", "--method", &method, "--repo", &repo, "--json",
        ])
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", text(&out));
    let key: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let input = key["input"].as_str().unwrap().to_string();
    let key_cid = key["keyCid"].as_str().unwrap().to_string();
    assert_eq!(key["basis"], "basis-key:git-tree");
    assert_eq!(key["method"], method_cid.to_string());
    assert!(
        key_cid.starts_with("bafyrei"),
        "the key is a DAG-CBOR CID: {key_cid}"
    );

    let requires_file = scratch.path().join("requires.json");
    std::fs::write(&requires_file, r#"[ {"source":"declared", "predicate":{"value":["avx2"], "kind":"subset"}, "axis":"isa.x86"} ]"#).unwrap();
    let mut expected = serde_json::json!([
        {"axis":host_contract::names::axis::ARCH,"predicate":{"kind":"equal","value":"x86_64"},"source":"extracted"},
        {"axis":host_contract::names::axis::OS,"predicate":{"kind":"equal","value":"linux"},"source":"extracted"}
    ]);
    if with_requires {
        expected.as_array_mut().unwrap().insert(1, serde_json::json!({"axis":host_contract::names::axis::ISA_X86,"predicate":{"kind":"subset","value":["avx2"]},"source":"declared"}));
    }
    expected.sort_all_objects();
    let canonical_requires = serde_json::to_vec(&expected).unwrap();
    let requires_cid = BlobCid::compute_bytes(&canonical_requires);
    let put = |input: &str| {
        let mut command = command();
        command.env("BRIT_BUILD_REF_BIN", &stub).args([
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
        ]);
        if with_requires {
            command.args(["--requires", requires_file.to_str().unwrap()]);
        }
        command.output().unwrap()
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
        format!("--manifest {method_cid}"),
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
        format!("env:family={}", effective.family_cid().unwrap()),
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

    assert_eq!(
        produce
            .classified_as
            .iter()
            .find_map(|s| s.strip_prefix("env:requires=")),
        Some(requires_cid.to_string()).as_deref()
    );

    // It crosses under the repository's own allow-list …
    let policy =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../genesis/agentic/pool-policy.json");
    let allow = CrossingAllowList::from_policy_file(&policy).unwrap();
    assert!(allow.admits(produce), "artifact-produce@1 produce crosses");

    // … and it is this device's local custody: the serve-blob mount indexes it from this event.
    // Opened as every epr process opens it: by the directory's format marker (artifact-store E4),
    // so a blob-store directory is never read as a flat directory store.
    let store = elohim_epr_cli::payload::open_payload_store(
        &a.payload(),
        elohim_epr_cli::payload::Access::Shared,
    )
    .await
    .unwrap()
    .store;
    let stored = store
        .fetch_blob(&method_cid, eprfs_core::FetchPolicy::LocalOnly)
        .await
        .unwrap();
    assert_eq!(BlobCid::compute(&stored.bytes).to_string(), key["method"]);
    assert_eq!(stored.cid, effective.cid().unwrap());
    {
        let stored = store
            .fetch_blob(&requires_cid, eprfs_core::FetchPolicy::LocalOnly)
            .await
            .unwrap();
        assert_eq!(stored.bytes.as_ref(), canonical_requires);
        assert_eq!(BlobCid::compute_bytes(&stored.bytes), requires_cid);
    }
    let evidence_cid = produce
        .classified_as
        .iter()
        .find_map(|s| s.strip_prefix("env:requires-evidence="))
        .unwrap();
    let evidence_cid = BlobCid::parse(evidence_cid).unwrap();
    let stored = store
        .fetch_blob(&evidence_cid, eprfs_core::FetchPolicy::LocalOnly)
        .await
        .unwrap();
    assert_eq!(BlobCid::compute_bytes(&stored.bytes), evidence_cid);
    let evidence: serde_json::Value = serde_json::from_slice(&stored.bytes).unwrap();
    assert_eq!(evidence[host_contract::names::axis::ARCH]["e_machine"], 62);
    assert_eq!(
        evidence[host_contract::names::axis::ISA_X86]["instructions"],
        1
    );
    let (mount, _served) = BlobMount::new(store, did.clone());
    let indexed: Vec<_> = SidecarFlowStore::open(a.root()).unwrap().events().unwrap();
    let unindexed = mount.rebuild_from_flow(&indexed).await;
    assert!(unindexed.is_empty(), "{unindexed:?}");
    let held = mount
        .held(&BlobCid::parse(&output).unwrap())
        .expect("indexed as custody");
    assert_eq!(held.manifest.to_string(), manifest);

    // A readable ELF with unknown OS must fail extraction before even a brit attestation.
    let calls_before = std::fs::read(&log).unwrap();
    let mut unknown_os = artifact.clone();
    unknown_os[7] = 0; // Generic System V, with neither interpreter nor ABI tag.
    std::fs::write(&bytes, &unknown_os).unwrap();
    let out = put(&input);
    assert_eq!(out.status.code(), Some(3), "{}", text(&out));
    assert!(
        text(&out).contains("Requires extraction") && text(&out).contains("os unknown"),
        "{}",
        text(&out)
    );
    assert!(
        text(&out).contains(bytes.to_str().unwrap()),
        "{}",
        text(&out)
    );
    assert_eq!(std::fs::read(&log).unwrap(), calls_before);
    assert!(mount.held(&BlobCid::compute_bytes(&unknown_os)).is_none());
    std::fs::write(&bytes, &artifact).unwrap();

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
        authority: eprfs_peer::FetchAuthority::Output {
            cid: BlobCid::compute(b"\xa1\x61k\x61v"),
        },
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
            authority: eprfs_peer::FetchAuthority::Output {
                cid: BlobCid::compute(b"\xa1\x61k\x61w"),
            },
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

/// An executable `cc` in `dir` that answers `--version` with `version`.
fn fake_cc(dir: &Path, version: &str) {
    fake_tool(dir, "cc", &format!(
        "case \"$1\" in\n--version) echo '{version}' ;;\n-E) echo '#define __SSE2__ 1' ;;\n-print-file-name=*) echo \"${{1#*=}}\" ;;\nesac\n"
    ));
}

#[test]
fn changing_the_path_resolved_cc_moves_the_key() {
    let keys = TempDir::new().unwrap();
    let a = Device::new(&keys, "a");
    let scratch = TempDir::new().unwrap();
    let tools = TempDir::new().unwrap();
    let first = tools.path().join("one");
    let second = tools.path().join("two");
    fake_cc(&first, "cc (Fixture) 14.3.1");
    fake_cc(&second, "cc (Fixture) 14.3.1");
    let path_one = format!("{}:/bin", first.display());
    let path_two = format!("{}:/bin", second.display());
    let (method_one, _) = method_file_with(
        &scratch.path().join("m1"),
        &["crate/src"],
        &[("PATH", path_one.as_str())],
    );
    let (method_two, _) = method_file_with(
        &scratch.path().join("m2"),
        &["crate/src"],
        &[("PATH", path_one.as_str())],
    );
    let before = key_of(&a, &method_one, &[]);
    assert_eq!(
        before,
        key_of(&a, &method_two, &[]),
        "one PATH, one cc: one key"
    );
    // The same cc answering another version moves the key.
    fake_cc(&first, "cc (Fixture) 15.0.0");
    let upgraded = key_of(&a, &method_one, &[]);
    assert_ne!(before, upgraded, "the resolved cc's --version is keyed");
    // A recorded PATH resolving another cc moves it too, even when it answers alike.
    let (method_three, _) = method_file_with(
        &scratch.path().join("m3"),
        &["crate/src"],
        &[("PATH", path_two.as_str())],
    );
    fake_cc(&second, "cc (Fixture) 15.0.0");
    assert_ne!(
        upgraded,
        key_of(&a, &method_three, &[]),
        "the resolved cc's path is keyed"
    );
}

#[test]
fn a_served_daemon_without_a_store_record_refuses_put() {
    let keys = TempDir::new().unwrap();
    let a = Device::new(&keys, "a");
    let scratch = TempDir::new().unwrap();
    let (method, _) = method_file(scratch.path(), &["crate/src"]);
    let (stub, _log) = brit_stub(scratch.path());
    let bytes = scratch.path().join("elohim-storage");
    std::fs::write(&bytes, artifact_elf(b"Linker: LLD 22.1.2\0")).unwrap();
    let repo = a.root().to_str().unwrap().to_string();
    let method = method.to_str().unwrap().to_string();
    let input = {
        let out = a.epr(&[
            "artifact", "key", "--method", &method, "--repo", &repo, "--print", "input",
        ]);
        assert!(out.status.success(), "{}", text(&out));
        String::from_utf8(out.stdout).unwrap().trim().to_string()
    };
    let put = || {
        a.command()
            .env("BRIT_BUILD_REF_BIN", &stub)
            .args([
                "artifact",
                "put",
                "--method",
                &method,
                "--input",
                &input,
                "--bytes",
                bytes.to_str().unwrap(),
                "--project",
                "elohim-storage",
                "--cross",
                "--repo",
                &repo,
            ])
            .output()
            .unwrap()
    };
    // This device's daemon is serving (its lock is held) and recorded no payload store.
    let peer_dir = a.config.path().join("epr").join("peer");
    let _serving = elohim_epr_cli::peer::ServeLock::acquire(&peer_dir).unwrap();
    let marker = peer_dir.join(elohim_epr_cli::peer::PAYLOAD_STORE_RECORD);
    let out = put();
    assert_eq!(out.status.code(), Some(3), "{}", text(&out));
    assert!(text(&out).contains("unrecorded:"), "{}", text(&out));
    assert!(
        text(&out).contains("payload-store record"),
        "{}",
        text(&out)
    );
    // An empty record refuses alike; a record naming another store refuses.
    std::fs::write(&marker, "").unwrap();
    assert_eq!(put().status.code(), Some(3));
    std::fs::write(&marker, "/somewhere/else\n").unwrap();
    let out = put();
    assert_eq!(out.status.code(), Some(3), "{}", text(&out));
    assert!(
        text(&out).contains("serves the payload store /somewhere/else"),
        "{}",
        text(&out)
    );
    // The record naming this put's own store lets it through.
    let ours = std::fs::canonicalize(a.payload()).unwrap();
    std::fs::write(&marker, format!("{}\n", ours.display())).unwrap();
    let out = put();
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(
        events(a.root())
            .iter()
            .filter(|e| e.action == ReaVerb::Produce)
            .count(),
        1,
        "only the put that knew its store recorded"
    );
}

fn fake_tool(dir: &Path, name: &str, body: &str) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::create_dir_all(dir).unwrap();
    let file = dir.join(name);
    std::fs::write(&file, format!("#!/bin/sh\n{body}\n")).unwrap();
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o755)).unwrap();
    wait_for_stub(&file);
    // A fixture toolchain must also answer the key-time std native-library probe.
    if name != "rustc" && !dir.join("rustc").exists() {
        fake_tool(
            dir,
            "rustc",
            &format!("if [ \"$1\" = -vV ]; then printf '%s' '{}'; else echo 'note: native-static-libs: -lgcc_s -lc' >&2; fi", fixture::RUSTC_IDENTITY),
        );
    }
}

#[test]
fn artifact_descriptor_without_path_preserves_legacy_key() {
    let repo = checkout();
    let scratch = TempDir::new().unwrap();
    let (_, mut descriptor) = method_file(scratch.path(), &["crate/src"]);
    descriptor.tools.insert(
        "rustc".into(),
        "rustc fixture\nhost: x86_64-unknown-linux-gnu\nLLVM version: 22.1.2\n".into(),
    );
    let key = elohim_epr_cli::artifact::closure_key(repo.path(), &descriptor).unwrap();
    // Captured at dev 6b8006579. This synthetic descriptor has no recorded PATH and
    // probes no host tools: this golden covers legacy descriptors, not link inputs.
    assert_eq!(
        key.key_cid.to_string(),
        "bafyreiccytmiybrzfdgsbctsti6jq3pmtflee4iadyezcxfaeng6d2wsvm"
    );
}

#[test]
fn artifact_unused_system_ld_does_not_move_key() {
    let repo = checkout();
    let tools = TempDir::new().unwrap();
    fake_cc(tools.path(), "cc fixture");
    fake_tool(tools.path(), "ld", "echo 'GNU ld 1'");
    let (_, first) = method_file_with(
        tools.path(),
        &["crate/src"],
        &[("PATH", tools.path().to_str().unwrap())],
    );
    let second = first.clone();
    let before = elohim_epr_cli::artifact::closure_key(repo.path(), &first).unwrap();
    fake_tool(tools.path(), "ld", "echo 'GNU ld 2'");
    let after = elohim_epr_cli::artifact::closure_key(repo.path(), &second).unwrap();
    assert_eq!(before.key_cid, after.key_cid);
}

#[test]
fn artifact_explicit_driver_keys_default_ld_unless_bare_lld_selected() {
    use elohim_epr_cli::artifact::{closure_key, ArtifactError};
    let repo = checkout();
    let tools = TempDir::new().unwrap();
    fake_cc(tools.path(), "gcc fixture");
    let gcc = tools.path().join("gcc");
    std::fs::copy(tools.path().join("cc"), &gcc).unwrap();
    wait_for_stub(&gcc);
    let (_, mut descriptor) = method_file_with(
        tools.path(),
        &["crate/src"],
        &[
            ("PATH", tools.path().to_str().unwrap()),
            ("RUSTFLAGS", &format!("-Clinker={}", gcc.display())),
        ],
    );
    let error = closure_key(repo.path(), &descriptor).unwrap_err();
    assert!(matches!(error, ArtifactError::Gap(_)));
    assert!(
        error
            .to_string()
            .contains("external-linker=ld does not resolve under recorded PATH"),
        "{error}"
    );
    fake_tool(tools.path(), "ld", "echo 'GNU ld 2.41'");
    let before = closure_key(repo.path(), &descriptor).unwrap();
    fake_tool(tools.path(), "ld", "echo 'GNU ld 2.42'");
    assert_ne!(
        before.key_cid,
        closure_key(repo.path(), &descriptor).unwrap().key_cid
    );

    descriptor.non_path_inputs.insert(
        "RUSTFLAGS".into(),
        format!("-Clinker={} -Clink-arg=-fuse-ld=lld", gcc.display()),
    );
    let bundled = closure_key(repo.path(), &descriptor).unwrap();
    std::fs::remove_file(tools.path().join("ld")).unwrap();
    assert_eq!(
        bundled.key_cid,
        closure_key(repo.path(), &descriptor).unwrap().key_cid
    );
}

#[test]
fn artifact_put_explicit_driver_without_gnu_ld_witness_refuses_before_recording() {
    let keys = TempDir::new().unwrap();
    let device = Device::new(&keys, "gnu-ld");
    let scratch = TempDir::new().unwrap();
    let tools = TempDir::new().unwrap();
    fake_cc(tools.path(), "gcc fixture");
    fake_tool(tools.path(), "ld", "echo 'GNU ld (GNU Binutils) 2.41'");
    // CLI plumbing uses git, but the recorded build PATH contains only the fixture tools.
    let build_path = tools.path().to_str().unwrap();
    let flags = format!("-Clinker={}/cc", build_path);
    let (method, descriptor) = method_file_with(
        scratch.path(),
        &["crate/src"],
        &[("PATH", build_path), ("RUSTFLAGS", &flags)],
    );
    let key = elohim_epr_cli::artifact::closure_key(device.root(), &descriptor).unwrap();
    let (stub, log) = brit_stub(scratch.path());
    let bytes = scratch.path().join("binary");
    // GNU ld preserves compiler comments but emits no linker identity.
    std::fs::write(&bytes, artifact_elf(b"GCC: (GNU) 14.3.1\0")).unwrap();
    let out = device
        .command()
        .env("BRIT_BUILD_REF_BIN", &stub)
        .env(
            "PATH",
            format!("{build_path}:{}", std::env::var("PATH").unwrap()),
        )
        .env("RUSTFLAGS", &flags)
        .args([
            "artifact",
            "put",
            "--method",
            method.to_str().unwrap(),
            "--input",
            &key.input.to_string(),
            "--bytes",
            bytes.to_str().unwrap(),
            "--project",
            "elohim-storage",
            "--repo",
            device.root().to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3), "{}", text(&out));
    assert!(
        text(&out)
            .contains("an external GNU ld leaves no linker witness; this build cannot be recorded"),
        "{}",
        text(&out)
    );
    assert!(!log.exists(), "refused before any brit call");
    assert!(events(device.root()).is_empty());
}

#[test]
fn artifact_link_input_bytes_move_key() {
    let repo = checkout();
    let tools = TempDir::new().unwrap();
    let input = tools.path().join("Scrt1.o");
    std::fs::write(&input, b"glibc 2.39-128").unwrap();
    fake_tool(tools.path(), "cc", &format!(
        "case \"$1\" in\n--version) echo 'cc fixture' ;;\n-E) echo '#define __SSE2__ 1' ;;\n-print-file-name=Scrt1.o) echo '{}' ;;\n-print-file-name=*) echo \"${{1#*=}}\" ;;\nesac",
        input.display()
    ));
    let (_, descriptor) = method_file_with(
        tools.path(),
        &["crate/src"],
        &[("PATH", tools.path().to_str().unwrap())],
    );
    let before = elohim_epr_cli::artifact::closure_key(repo.path(), &descriptor).unwrap();
    std::fs::write(&input, b"glibc 2.39-145").unwrap();
    let after = elohim_epr_cli::artifact::closure_key(repo.path(), &descriptor).unwrap();
    assert_ne!(before.key_cid, after.key_cid);
}

#[test]
fn artifact_cc_target_defaults_move_key() {
    let repo = checkout();
    let tools = TempDir::new().unwrap();
    let cc = |march| {
        fake_tool(tools.path(), "cc", &format!(
        "case \"$1\" in\n--version) echo 'cc fixture' ;;\n-E) echo '#define {march} 1' ;;\n-print-file-name=*) echo \"${{1#*=}}\" ;;\nesac"
    ))
    };
    cc("__SSE2__");
    let (_, descriptor) = method_file_with(
        tools.path(),
        &["crate/src"],
        &[("PATH", tools.path().to_str().unwrap())],
    );
    let before = elohim_epr_cli::artifact::closure_key(repo.path(), &descriptor).unwrap();
    cc("__AVX2__");
    let after = elohim_epr_cli::artifact::closure_key(repo.path(), &descriptor).unwrap();
    assert_ne!(before.key_cid, after.key_cid);
}

#[test]
fn artifact_recorded_linker_version_moves_key() {
    let repo = checkout();
    let tools = TempDir::new().unwrap();
    fake_cc(tools.path(), "cc fixture");
    for (variable, value, tool) in [
        ("RUSTFLAGS", "-C linker=ld", "ld"),
        ("RUSTFLAGS", "-Clinker=ld", "ld"),
        ("CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER", "ld", "ld"),
        ("RUSTFLAGS", "-C link-arg=-fuse-ld=bfd", "ld.bfd"),
        ("CARGO_ENCODED_RUSTFLAGS", "-C\u{1f}linker=ld", "ld"),
    ] {
        fake_tool(tools.path(), tool, "case \"$1\" in\n--version) echo 'GNU ld 1' ;;\n-E) echo '#define __SSE2__ 1' ;;\n-print-file-name=*) echo \"${1#*=}\" ;;\nesac");
        let (_, descriptor) = method_file_with(
            tools.path(),
            &["crate/src"],
            &[("PATH", tools.path().to_str().unwrap()), (variable, value)],
        );
        let before = elohim_epr_cli::artifact::closure_key(repo.path(), &descriptor).unwrap();
        fake_tool(tools.path(), tool, "case \"$1\" in\n--version) echo 'GNU ld 2' ;;\n-E) echo '#define __SSE2__ 1' ;;\n-print-file-name=*) echo \"${1#*=}\" ;;\nesac");
        let after = elohim_epr_cli::artifact::closure_key(repo.path(), &descriptor).unwrap();
        assert_ne!(before.key_cid, after.key_cid, "{variable}={value}");
    }
}

#[test]
fn artifact_put_checks_elf_linker_before_recording() {
    let keys = TempDir::new().unwrap();
    let device = Device::new(&keys, "linker");
    let scratch = TempDir::new().unwrap();
    let (method, mut descriptor) = method_file(scratch.path(), &["crate/src"]);
    let rustc = Command::new("rustc").arg("-vV").output().unwrap();
    assert!(rustc.status.success());
    let rustc = String::from_utf8(rustc.stdout).unwrap();
    let llvm = rustc
        .lines()
        .find_map(|line| line.strip_prefix("LLVM version: "))
        .unwrap()
        .to_string();
    descriptor.tools.insert("rustc".into(), rustc);
    std::fs::write(&method, serde_json::to_vec(&descriptor).unwrap()).unwrap();
    let key = elohim_epr_cli::artifact::closure_key(device.root(), &descriptor).unwrap();
    let (stub, log) = brit_stub(scratch.path());
    let bytes = scratch.path().join("binary");
    for comment in [
        "Linker: GNU ld 2.41\0",
        "Linker: LLD 0.0.0\0",
        "GCC: 14.3.1\0",
    ] {
        std::fs::write(&bytes, artifact_elf(comment.as_bytes())).unwrap();
        let out = device
            .command()
            .env("BRIT_BUILD_REF_BIN", &stub)
            .args([
                "artifact",
                "put",
                "--method",
                method.to_str().unwrap(),
                "--input",
                &key.input.to_string(),
                "--bytes",
                bytes.to_str().unwrap(),
                "--project",
                "elohim-storage",
                "--repo",
                device.root().to_str().unwrap(),
            ])
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(3), "{}", text(&out));
        {
            assert!(!log.exists(), "refused before any brit call");
            assert!(events(device.root()).is_empty());
            assert!(text(&out).contains("Linker"), "{}", text(&out));
            if comment.starts_with("Linker:") {
                assert!(
                    text(&out).contains(&format!("LLD {llvm}")),
                    "{}",
                    text(&out)
                );
                assert!(
                    text(&out).contains(comment.trim_end_matches('\0')),
                    "{}",
                    text(&out)
                );
            }
        }
    }
}

#[test]
fn artifact_put_refuses_a_declared_target_no_build_applies_yet() {
    let keys = TempDir::new().unwrap();
    let device = Device::new(&keys, "declared-target");
    let scratch = TempDir::new().unwrap();
    let (method, mut descriptor) = method_file(scratch.path(), &["crate/src"]);
    descriptor
        .non_path_inputs
        .insert("target.isa".into(), "x86-64-v3".into());
    std::fs::write(&method, serde_json::to_vec(&descriptor).unwrap()).unwrap();
    let (stub, log) = brit_stub(scratch.path());
    let bytes = scratch.path().join("binary");
    std::fs::write(&bytes, artifact_elf(b"Linker: LLD 22.1.2\0")).unwrap();
    let out = device
        .command()
        .env("BRIT_BUILD_REF_BIN", &stub)
        .args([
            "artifact",
            "put",
            "--method",
            method.to_str().unwrap(),
            "--input",
            "bafkreiaaaa",
            "--bytes",
            bytes.to_str().unwrap(),
            "--project",
            "elohim-storage",
            "--repo",
            device.root().to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3), "{}", text(&out));
    assert!(text(&out).contains("target.isa"), "{}", text(&out));
    assert!(text(&out).contains("K6"), "{}", text(&out));
    assert!(!log.exists(), "refused before any brit call");
    assert!(events(device.root()).is_empty());
}

fn fixture_driver(dir: &Path, name: &str, version: &str, macros: &str, input: &Path) {
    fake_tool(dir, name, &format!(
        "case \"$1\" in\n--version) echo '{version}' ;;\n-E) test \"$*\" = '-E -dM -x c -' || exit 1; read ignored && exit 1; printf '%s\\n' '{macros}' ;;\n-print-file-name=Scrt1.o) echo '{}' ;;\n-print-file-name=*) echo \"${{1#*=}}\" ;;\n*) exit 1 ;;\nesac", input.display()
    ));
}

#[test]
fn artifact_link_driver_is_independent_of_cc() {
    use elohim_epr_cli::artifact::closure_key;
    let repo = checkout();
    let tools = TempDir::new().unwrap();
    fake_tool(tools.path(), "ld", "echo 'GNU ld 2.41'");
    let input = tools.path().join("startup.o");
    std::fs::write(&input, "startup one").unwrap();
    fixture_driver(
        tools.path(),
        "cc",
        "driver one",
        "#define __SSE2__ 1",
        &input,
    );
    // A build-script compiler must never receive link-input probes.
    fake_tool(tools.path(), "build-cc", "case \"$1\" in\n--version) echo 'build cc 1' ;;\n-E) echo '#define __SSE2__ 1' ;;\n*) exit 1 ;;\nesac");
    let (_, mut descriptor) = method_file_with(
        tools.path(),
        &["crate/src"],
        &[("PATH", tools.path().to_str().unwrap()), ("CC", "build-cc")],
    );
    let before = closure_key(repo.path(), &descriptor).unwrap();
    std::fs::write(&input, "startup two").unwrap();
    assert_ne!(
        before.key_cid,
        closure_key(repo.path(), &descriptor).unwrap().key_cid
    );
    let before = closure_key(repo.path(), &descriptor).unwrap();
    fake_tool(tools.path(), "build-cc", "case \"$1\" in\n--version) echo 'build cc 2' ;;\n-E) echo '#define __AVX2__ 1' ;;\n*) exit 1 ;;\nesac");
    assert_ne!(
        before.key_cid,
        closure_key(repo.path(), &descriptor).unwrap().key_cid
    );
    fixture_driver(
        tools.path(),
        "selected-cc",
        "selected driver",
        "#define __BMI2__ 1",
        &input,
    );
    descriptor.non_path_inputs.remove("RUSTFLAGS");
    for (variable, flags) in [
        ("RUSTFLAGS", "-C linker=selected-cc"),
        ("CARGO_ENCODED_RUSTFLAGS", "-Clinker=selected-cc"),
        (
            "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS",
            "-C linker=selected-cc",
        ),
        (
            "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER",
            "selected-cc",
        ),
    ] {
        descriptor
            .non_path_inputs
            .insert(variable.into(), flags.into());
        let before = closure_key(repo.path(), &descriptor).unwrap();
        fake_cc(tools.path(), "unused cc changed");
        assert_eq!(
            before.key_cid,
            closure_key(repo.path(), &descriptor).unwrap().key_cid
        );
        std::fs::write(&input, flags).unwrap();
        assert_ne!(
            before.key_cid,
            closure_key(repo.path(), &descriptor).unwrap().key_cid
        );
        descriptor.non_path_inputs.remove(variable);
    }
}

#[test]
fn artifact_unsupported_linker_selections_refuse_before_probing() {
    use elohim_epr_cli::artifact::{closure_key, ArtifactError};
    let repo = checkout();
    let tools = TempDir::new().unwrap();
    let (_, mut descriptor) = method_file_with(tools.path(), &["crate/src"], &[]);
    descriptor.non_path_inputs.remove("RUSTFLAGS");
    descriptor
        .flags
        .push("--target=i686-unknown-linux-gnu".into());
    for variable in [
        "RUSTFLAGS",
        "CARGO_ENCODED_RUSTFLAGS",
        "CARGO_TARGET_I686_UNKNOWN_LINUX_GNU_RUSTFLAGS",
    ] {
        for flag in [
            "-Clink-self-contained=-linker",
            "-C link-self-contained=no",
            "-Zlinker-features=-lld",
            "-Z linker-features=-lld",
            "-Clinker-flavor=gcc",
            "-C linker-flavor=gnu",
        ] {
            let value = if variable == "CARGO_ENCODED_RUSTFLAGS" {
                flag.replace(' ', "\u{1f}")
            } else {
                flag.into()
            };
            descriptor.non_path_inputs.insert(variable.into(), value);
            let error = closure_key(repo.path(), &descriptor).unwrap_err();
            assert!(matches!(error, ArtifactError::Gap(_)));
            assert!(
                error
                    .to_string()
                    .contains("does not model that linker selection yet"),
                "{error}"
            );
            assert!(
                error
                    .to_string()
                    .contains(flag.split_whitespace().last().unwrap()),
                "{error}"
            );
            descriptor.non_path_inputs.remove(variable);
        }
    }
    descriptor
        .non_path_inputs
        .insert("crate".into(), "crate".into());
    let config = repo.path().join("crate/.cargo/config.toml");
    std::fs::create_dir_all(config.parent().unwrap()).unwrap();
    for setting in [
        "linker = 'gcc'",
        "rustflags = ['-C', 'link-self-contained=-linker']",
        "rustflags = '-Zlinker-features=-lld'",
        "rustflags = ['-Clinker-flavor=gcc']",
        "rustflags = ['-Clinker=gcc']",
        "rustflags = ['-C', 'link-arg=-fuse-ld=bfd']",
    ] {
        std::fs::write(&config, format!("[target.'cfg(unix)']\n{setting}\n")).unwrap();
        let error = closure_key(repo.path(), &descriptor).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("does not model that linker selection yet"),
            "{error}"
        );
        assert!(error.to_string().contains("target.cfg(unix)"), "{error}");
    }
    std::fs::write(
        config,
        "[target.'cfg(unix)']\nrustflags = ['-C', 'opt-level=2']\n",
    )
    .unwrap();
    assert!(closure_key(repo.path(), &descriptor)
        .unwrap_err()
        .to_string()
        .contains("cfg(unix)"));
}

#[test]
fn artifact_bare_lld_uses_rustc_but_external_flavors_and_paths_are_keyed() {
    use elohim_epr_cli::artifact::closure_key;
    let repo = checkout();
    let tools = TempDir::new().unwrap();
    fake_cc(tools.path(), "cc fixture");
    let (_, mut descriptor) = method_file_with(
        tools.path(),
        &["crate/src"],
        &[
            ("PATH", tools.path().to_str().unwrap()),
            ("RUSTFLAGS", "-Clinker=cc -C link-arg=-fuse-ld=lld"),
        ],
    );
    let before = closure_key(repo.path(), &descriptor).unwrap();
    fake_tool(tools.path(), "ld.lld", "echo 'LLD unrelated 999'");
    assert_eq!(
        before.key_cid,
        closure_key(repo.path(), &descriptor).unwrap().key_cid
    );
    for flavor in ["bfd", "gold", "mold"] {
        descriptor.non_path_inputs.insert(
            "RUSTFLAGS".into(),
            format!("-Clinker=cc -C link-arg=-fuse-ld={flavor}"),
        );
        assert!(closure_key(repo.path(), &descriptor)
            .unwrap_err()
            .to_string()
            .contains("does not resolve"));
        fake_tool(tools.path(), &format!("ld.{flavor}"), "echo 'linker 1'");
        closure_key(repo.path(), &descriptor).unwrap();
    }
    let linker = tools.path().join("external-lld");
    descriptor.non_path_inputs.insert(
        "RUSTFLAGS".into(),
        format!("-Clinker=cc -C link-arg=-fuse-ld={}", linker.display()),
    );
    fake_tool(tools.path(), "external-lld", "echo 'LLD 1'");
    let before = closure_key(repo.path(), &descriptor).unwrap();
    fake_tool(tools.path(), "external-lld", "echo 'LLD 2'");
    assert_ne!(
        before.key_cid,
        closure_key(repo.path(), &descriptor).unwrap().key_cid
    );
}

#[test]
fn artifact_linker_script_referents_move_key_and_missing_or_deep_inputs_refuse() {
    use elohim_epr_cli::artifact::closure_key;
    let repo = checkout();
    let tools = TempDir::new().unwrap();
    let script = tools.path().join("input.so");
    let nested = tools.path().join("nested.so");
    let library = tools.path().join("library.so.1");
    std::fs::write(
        &script,
        format!(
            "/* GNU ld script /not/an/input */\nGROUP ( AS_NEEDED ( {} ) )",
            nested.display()
        ),
    )
    .unwrap();
    std::fs::write(&nested, format!("INPUT ( {} )", library.display())).unwrap();
    std::fs::write(&library, b"library one").unwrap();
    fixture_driver(
        tools.path(),
        "cc",
        "cc fixture",
        "#define __SSE2__ 1",
        &script,
    );
    let (_, descriptor) = method_file_with(
        tools.path(),
        &["crate/src"],
        &[("PATH", tools.path().to_str().unwrap())],
    );
    let before = closure_key(repo.path(), &descriptor).unwrap();
    std::fs::write(&library, b"library two").unwrap();
    assert_ne!(
        before.key_cid,
        closure_key(repo.path(), &descriptor).unwrap().key_cid
    );
    std::fs::remove_file(&library).unwrap();
    assert!(closure_key(repo.path(), &descriptor)
        .unwrap_err()
        .to_string()
        .contains("does not resolve"));
    std::fs::write(&library, format!("INPUT ( {} )", script.display())).unwrap();
    assert!(closure_key(repo.path(), &descriptor)
        .unwrap_err()
        .to_string()
        .contains("depth 4"));
}

#[test]
fn artifact_macro_order_is_stable_and_target_default_changes_move_key() {
    use elohim_epr_cli::artifact::closure_key;
    let repo = checkout();
    let tools = TempDir::new().unwrap();
    let input = tools.path().join("startup.o");
    std::fs::write(&input, "startup").unwrap();
    let macros = "#define __SSE2__ 1\n#define __BMI2__ 1";
    fixture_driver(tools.path(), "cc", "clang fixture", macros, &input);
    let (_, descriptor) = method_file_with(
        tools.path(),
        &["crate/src"],
        &[("PATH", tools.path().to_str().unwrap())],
    );
    let before = closure_key(repo.path(), &descriptor).unwrap();
    fixture_driver(
        tools.path(),
        "cc",
        "clang fixture",
        "#define __BMI2__ 1\n#define __SSE2__ 1",
        &input,
    );
    assert_eq!(
        before.key_cid,
        closure_key(repo.path(), &descriptor).unwrap().key_cid
    );
    fixture_driver(
        tools.path(),
        "cc",
        "clang fixture",
        "#define __AVX2__ 1",
        &input,
    );
    assert_ne!(
        before.key_cid,
        closure_key(repo.path(), &descriptor).unwrap().key_cid
    );
}

#[test]
fn artifact_native_build_env_excludes_unpassed_c_flags() {
    for name in ["CFLAGS", "CXXFLAGS", "LDFLAGS"] {
        assert!(!elohim_epr_cli::artifact::NATIVE_BUILD_ENV.contains(&name));
    }
}

#[test]
fn artifact_real_gcc_accepts_rustc_bundled_lld() {
    let Ok(cc) = Command::new("cc").arg("--version").output() else {
        eprintln!("SKIP: real cc is unavailable");
        return;
    };
    if !cc.status.success() || !text(&cc).contains("Free Software Foundation") {
        eprintln!("SKIP: real cc on PATH is not gcc");
        return;
    }
    let Ok(rustc) = Command::new("rustup")
        .args(["run", "1.96.1", "rustc", "-vV"])
        .output()
    else {
        eprintln!("SKIP: rustup / rustc 1.96.1 is unavailable");
        return;
    };
    if !rustc.status.success() {
        eprintln!("SKIP: rustc 1.96.1 is not installed");
        return;
    }
    let repo = checkout();
    let scratch = TempDir::new().unwrap();
    let (_, mut descriptor) = method_file_with(
        scratch.path(),
        &["crate/src"],
        &[
            ("PATH", &std::env::var("PATH").unwrap()),
            ("RUSTFLAGS", "-C link-arg=-fuse-ld=lld"),
            ("RUSTUP_TOOLCHAIN", "1.96.1"),
        ],
    );
    descriptor
        .tools
        .insert("rustc".into(), String::from_utf8(rustc.stdout).unwrap());
    elohim_epr_cli::artifact::closure_key(repo.path(), &descriptor).unwrap();
    eprintln!("real gcc + rustc 1.96.1 bundled-lld key: PASS");
}

#[test]
fn artifact_real_describe_keys_with_process_homes_and_home_location_is_not_an_input() {
    let repo = checkout();
    let scratch = TempDir::new().unwrap();
    let tools = scratch.path().join("bin");
    std::fs::create_dir_all(&tools).unwrap();
    // Keep Cargo's metadata response synthetic: describe runs its actual parser/writer and
    // the real rustc, without nesting another cargo command inside the test's cargo process.
    fake_tool(&tools, "cargo", "case \"$1\" in -V|--version) echo 'cargo fixture';; metadata) echo '{\"packages\":[]}';; *) exit 1;; esac");
    // fake_tool supplies rustc for other fixtures; this test must resolve the real proxy.
    std::fs::remove_file(tools.join("rustc")).unwrap();
    std::fs::write(
        repo.path().join("crate/Cargo.toml"),
        "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    std::fs::write(
        repo.path().join("manifest.json"),
        r#"{"steps":{"build":{"inputs":{"sources":["crate/src/**"]}}}}"#,
    )
    .unwrap();
    git(repo.path(), &["add", "crate/Cargo.toml", "manifest.json"]);
    git(repo.path(), &["commit", "-qm", "describe fixture"]);

    let build_path = std::env::var("PATH").unwrap();
    let mut describe = Command::new(env!("CARGO_BIN_EXE_epr"));
    describe
        .env_clear()
        .env("PATH", format!("{}:{build_path}", tools.display()))
        .env("RUSTUP_TOOLCHAIN", "1.96.1")
        .env("RUSTUP_AUTO_INSTALL", "0")
        .env("XDG_CONFIG_HOME", scratch.path().join("config"));
    for name in ["RUSTUP_HOME", "CARGO_HOME", "HOME", "TMPDIR"] {
        if let Some(value) = std::env::var_os(name) {
            describe.env(name, value);
        }
    }
    // mesh_describe_method passes configured build variables and PATH via --env. Homes are
    // inherited by the process only, exactly as they are for its later key/fetch/put calls.
    describe
        .args([
            "artifact",
            "describe",
            "--project",
            "doorway",
            "--bin",
            "fixture",
            "--crate",
            "crate",
            "--sources",
            "manifest.json#build",
            "--rustflags",
            "",
            "--env",
            "CC=cc",
            "--env",
            "RUSTUP_TOOLCHAIN=1.96.1",
            "--env",
        ])
        .arg(format!("PATH={build_path}"))
        .arg("--repo")
        .arg(repo.path());
    let out = describe.output().unwrap();
    assert!(out.status.success(), "{}", text(&out));
    let descriptor: MethodDescriptor = serde_json::from_slice(&out.stdout).unwrap();
    for name in ["RUSTUP_HOME", "CARGO_HOME", "HOME"] {
        assert!(!descriptor.non_path_inputs.contains_key(name), "{name}");
    }
    assert_eq!(descriptor.non_path_inputs["PATH"], build_path);
    assert_eq!(descriptor.non_path_inputs["CC"], "cc");
    assert!(descriptor.tools["rustc"].contains("release: 1.96.1"));
    let method = scratch.path().join("method.json");
    std::fs::write(&method, &out.stdout).unwrap();
    let baseline = elohim_epr_cli::artifact::closure_key(repo.path(), &descriptor).unwrap();

    // Isolated children vary HOME without mutating the parallel test process's environment.
    // RUSTUP_HOME continues to locate the same installed compiler in both cases.
    let rustup_home = std::env::var_os("RUSTUP_HOME").expect("device has RUSTUP_HOME");
    for home in ["home-a", "home-b"] {
        let home = scratch.path().join(home);
        std::fs::create_dir(&home).unwrap();
        let mut key_command = Command::new(env!("CARGO_BIN_EXE_epr"));
        key_command
            .env_clear()
            .env("PATH", &build_path)
            .env("CC", "cc")
            .env("RUSTFLAGS", "")
            .env("RUSTUP_TOOLCHAIN", "1.96.1")
            .env("HOME", &home)
            .env("RUSTUP_HOME", &rustup_home)
            .env("XDG_CONFIG_HOME", scratch.path().join("config"));
        for name in ["CARGO_HOME", "TMPDIR"] {
            if let Some(value) = std::env::var_os(name) {
                key_command.env(name, value);
            }
        }
        let out = key_command
            .args(["artifact", "key", "--json", "--repo"])
            .arg(repo.path())
            .arg("--method")
            .arg(&method)
            .output()
            .unwrap();
        assert!(out.status.success(), "{}", text(&out));
        let key: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(key["keyCid"], baseline.key_cid.to_string());
        assert_eq!(
            std::fs::read_dir(home).unwrap().count(),
            0,
            "proxy must not write in HOME"
        );
    }
}

#[test]
fn artifact_codegen_long_form_keys_driver() {
    use elohim_epr_cli::artifact::closure_key;
    let repo = checkout();
    let tools = TempDir::new().unwrap();
    fake_tool(tools.path(), "ld", "echo 'GNU ld 2.41'");
    fake_cc(tools.path(), "build cc");
    let (_, mut descriptor) = method_file_with(
        tools.path(),
        &["crate/src"],
        &[("PATH", tools.path().to_str().unwrap())],
    );
    for flag in ["--codegen linker=chosen", "--codegen=linker=chosen"] {
        descriptor
            .non_path_inputs
            .insert("RUSTFLAGS".into(), flag.into());
        assert!(closure_key(repo.path(), &descriptor)
            .unwrap_err()
            .to_string()
            .contains("chosen"));
        fixture_driver(
            tools.path(),
            "chosen",
            "driver 1",
            "#define X 1",
            &tools.path().join("input"),
        );
        std::fs::write(tools.path().join("input"), "one").unwrap();
        let before = closure_key(repo.path(), &descriptor).unwrap();
        std::fs::write(tools.path().join("input"), "two").unwrap();
        assert_ne!(
            before.key_cid,
            closure_key(repo.path(), &descriptor).unwrap().key_cid
        );
        std::fs::remove_file(tools.path().join("chosen")).unwrap();
    }
}

#[test]
fn artifact_codegen_long_form_refuses_self_contained() {
    use elohim_epr_cli::artifact::closure_key;
    let repo = checkout();
    let tools = TempDir::new().unwrap();
    let (_, mut descriptor) = method_file(tools.path(), &["crate/src"]);
    for flag in [
        "--codegen link-self-contained=-linker",
        "--codegen=link-self-contained=-linker",
    ] {
        descriptor
            .non_path_inputs
            .insert("RUSTFLAGS".into(), flag.into());
        assert!(closure_key(repo.path(), &descriptor)
            .unwrap_err()
            .to_string()
            .contains("link-self-contained"));
    }
}

#[test]
fn artifact_build_rustflags_refuse_unmodelled_selection() {
    use elohim_epr_cli::artifact::closure_key;
    let repo = checkout();
    let tools = TempDir::new().unwrap();
    let (_, mut descriptor) = method_file_with(tools.path(), &["crate/src"], &[("crate", "crate")]);
    descriptor.non_path_inputs.remove("RUSTFLAGS");
    std::fs::create_dir_all(repo.path().join("crate/.cargo")).unwrap();
    std::fs::write(
        repo.path().join("crate/.cargo/config.toml"),
        "[build]\nrustflags = ['--codegen=link-self-contained=-linker']\n",
    )
    .unwrap();
    assert!(closure_key(repo.path(), &descriptor)
        .unwrap_err()
        .to_string()
        .contains("link-self-contained"));
    // Even an empty present higher-priority source makes this config flag inactive.
    descriptor
        .non_path_inputs
        .insert("RUSTFLAGS".into(), String::new());
    closure_key(repo.path(), &descriptor).unwrap();
}

#[test]
fn artifact_last_bundled_lld_clears_external_tool() {
    use elohim_epr_cli::artifact::closure_key;
    let repo = checkout();
    let tools = TempDir::new().unwrap();
    fake_cc(tools.path(), "cc fixture");
    let (_, descriptor) = method_file_with(
        tools.path(),
        &["crate/src"],
        &[
            ("PATH", tools.path().to_str().unwrap()),
            (
                "RUSTFLAGS",
                "-C link-arg=-fuse-ld=bfd -C link-arg=-fuse-ld=lld",
            ),
        ],
    );
    let before = closure_key(repo.path(), &descriptor).unwrap();
    fake_tool(tools.path(), "ld.bfd", "echo 'GNU ld 999'");
    assert_eq!(
        before.key_cid,
        closure_key(repo.path(), &descriptor).unwrap().key_cid
    );
}

#[test]
fn artifact_encoded_flags_ignore_overridden_rustflags_in_key() {
    use elohim_epr_cli::artifact::closure_key;
    let repo = checkout();
    let tools = TempDir::new().unwrap();
    fake_cc(tools.path(), "cc fixture");
    let (_, mut descriptor) = method_file_with(
        tools.path(),
        &["crate/src"],
        &[
            ("PATH", tools.path().to_str().unwrap()),
            ("CARGO_ENCODED_RUSTFLAGS", "-C\u{1f}link-arg=-fuse-ld=lld"),
            ("RUSTFLAGS", "--codegen=linker=/nonexistent/ignored"),
        ],
    );
    let before = closure_key(repo.path(), &descriptor).unwrap();
    descriptor.non_path_inputs.remove("RUSTFLAGS");
    assert_eq!(
        before.key_cid,
        closure_key(repo.path(), &descriptor).unwrap().key_cid
    );
}

#[test]
fn artifact_other_target_linker_does_not_affect_key() {
    use elohim_epr_cli::artifact::closure_key;
    let repo = checkout();
    let tools = TempDir::new().unwrap();
    let (_, mut descriptor) = method_file_with(
        tools.path(),
        &["crate/src"],
        &[(
            "CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER",
            "/nonexistent/ignored",
        )],
    );
    descriptor
        .flags
        .push("--target=x86_64-unknown-linux-gnu".into());
    let before = closure_key(repo.path(), &descriptor).unwrap();
    descriptor
        .non_path_inputs
        .remove("CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER");
    assert_eq!(
        before.key_cid,
        closure_key(repo.path(), &descriptor).unwrap().key_cid
    );
}

#[test]
fn artifact_explicit_driver_still_keys_implicit_cc_defaults_and_version() {
    use elohim_epr_cli::artifact::closure_key;
    let repo = checkout();
    let tools = TempDir::new().unwrap();
    fake_tool(tools.path(), "ld", "echo 'GNU ld 2.41'");
    let input = tools.path().join("startup.o");
    std::fs::write(&input, "startup").unwrap();
    fixture_driver(
        tools.path(),
        "driver",
        "driver 1",
        "#define DRIVER 1",
        &input,
    );
    let (_, descriptor) = method_file_with(
        tools.path(),
        &["crate/src"],
        &[
            ("PATH", tools.path().to_str().unwrap()),
            ("RUSTFLAGS", "-Clinker=driver"),
        ],
    );
    assert!(!descriptor.non_path_inputs.contains_key("CC"));
    let compiler = |version, macros| {
        fake_tool(
            tools.path(),
            "cc",
            &format!(
        "case \"$1\" in\n--version) echo '{version}' ;;\n-E) echo '{macros}' ;;\n*) exit 1 ;;\nesac"
    ),
        )
    };
    compiler("cc 1", "#define DEFAULT 1");
    let before = closure_key(repo.path(), &descriptor).unwrap();
    compiler("cc 1", "#define DEFAULT 2");
    let defaults_changed = closure_key(repo.path(), &descriptor).unwrap();
    assert_ne!(before.key_cid, defaults_changed.key_cid);
    compiler("cc 2", "#define DEFAULT 2");
    assert_ne!(
        defaults_changed.key_cid,
        closure_key(repo.path(), &descriptor).unwrap().key_cid
    );
}

#[test]
fn artifact_underscore_link_args_key_external_path() {
    use elohim_epr_cli::artifact::closure_key;
    let repo = checkout();
    let tools = TempDir::new().unwrap();
    fake_cc(tools.path(), "cc fixture");
    let external = tools.path().join("external-ld");
    let (_, mut descriptor) = method_file_with(
        tools.path(),
        &["crate/src"],
        &[("PATH", tools.path().to_str().unwrap())],
    );
    for flags in [
        format!("-Clink_arg=-fuse-ld={}", external.display()),
        format!(
            "--codegen=link_args=-fuse-ld=lld -fuse-ld={}",
            external.display()
        ),
        format!(
            "-Clink-arg=-fuse-ld=lld\x1f-C\x1flink_args=-fuse-ld=bfd -fuse-ld={}",
            external.display()
        ),
    ] {
        descriptor
            .non_path_inputs
            .insert("CARGO_ENCODED_RUSTFLAGS".into(), flags);
        fake_tool(tools.path(), "external-ld", "echo 'GNU ld 1'");
        let before = closure_key(repo.path(), &descriptor).unwrap();
        fake_tool(tools.path(), "external-ld", "echo 'GNU ld 2'");
        assert_ne!(
            before.key_cid,
            closure_key(repo.path(), &descriptor).unwrap().key_cid
        );
        std::fs::remove_file(&external).unwrap();
        assert!(closure_key(repo.path(), &descriptor)
            .unwrap_err()
            .to_string()
            .contains(external.to_str().unwrap()));
    }
}

#[test]
fn holders_by_output_and_key_share_distinct_counts_and_advisory_warning() {
    let (_keys, a, b) = household();
    let key = BlobCid::compute(b"holder-key").to_string();
    let other_key = BlobCid::compute(b"other-holder-key").to_string();
    let output = BlobCid::compute_bytes(b"holder-output");
    let manifest = BlobCid::compute(b"manifest").to_string();
    let mut store = SidecarFlowStore::open(b.root()).unwrap();
    let row = produce_record(&key, &output, &manifest, Some(&a.key.did_key()));
    store.append(FlowRecord::Event(row.clone())).unwrap();
    let mut repeated = row;
    repeated.occurred_at = "2026-11-09T10:00:00Z".into();
    store.append(FlowRecord::Event(repeated)).unwrap();
    let query = |flag: &str, value: &str, json: bool| {
        let mut args = vec![
            "artifact",
            "holders",
            flag,
            value,
            "--repo",
            b.root().to_str().unwrap(),
        ];
        if json {
            args.push("--json");
        }
        b.epr(&args)
    };
    let by_key = query("--key", &key, true);
    let by_output = query("--output", &output.to_string(), true);
    assert!(by_key.status.success(), "{}", text(&by_key));
    assert!(by_output.status.success(), "{}", text(&by_output));
    let one: serde_json::Value = serde_json::from_slice(&by_key.stdout).unwrap();
    assert_eq!(
        one,
        serde_json::from_slice::<serde_json::Value>(&by_output.stdout).unwrap()
    );
    assert_eq!(one["holders"].as_array().unwrap().len(), 1);
    assert_eq!(one["belowMin"], true);
    let warning = query("--output", &output.to_string(), false);
    assert!(warning.status.success());
    assert_eq!(
        String::from_utf8(warning.stderr).unwrap().trim(),
        format!("warning: 1 holder(s) of {output}, fewer than 2")
    );
    // Local, uncrossed Produce is custody too; another key by the same device is not another holder.
    let mut local = produce_record(&key, &output, &manifest, None);
    local.receiver = AgentRef(b.key.did_key());
    local
        .classified_as
        .retain(|s| !s.starts_with("measure:") && !s.starts_with("reason:"));
    store.append(FlowRecord::Event(local)).unwrap();
    store
        .append(FlowRecord::Event(produce_record(
            &other_key,
            &output,
            &manifest,
            Some(&a.key.did_key()),
        )))
        .unwrap();
    let two = query("--output", &output.to_string(), true);
    let two: serde_json::Value = serde_json::from_slice(&two.stdout).unwrap();
    assert_eq!(two["holders"].as_array().unwrap().len(), 2);
    assert_eq!(two["belowMin"], false);
    for holder in two["holders"].as_array().unwrap() {
        assert_eq!(holder["self"], holder["device"] == b.key.did_key());
    }
    let marked = query("--output", &output.to_string(), false);
    assert!(String::from_utf8_lossy(&marked.stdout)
        .contains(&format!("{} {output} self", b.key.did_key())));
    let by_key = query("--key", &key, true);
    assert_eq!(
        two,
        serde_json::from_slice::<serde_json::Value>(&by_key.stdout).unwrap()
    );
    let quiet = query("--output", &output.to_string(), false);
    assert!(quiet.status.success());
    assert!(!String::from_utf8_lossy(&quiet.stderr).contains("warning:"));
    let higher = b.epr(&[
        "artifact",
        "holders",
        "--output",
        &output.to_string(),
        "--min",
        "3",
        "--repo",
        b.root().to_str().unwrap(),
    ]);
    assert!(higher.status.success());
    assert!(String::from_utf8_lossy(&higher.stderr).contains("fewer than 3"));
    // Later production under the same key retains A's older output in either lookup.
    let newer = BlobCid::compute_bytes(b"newer-holder-output");
    store
        .append(FlowRecord::Event(produce_record(
            &key,
            &newer,
            &manifest,
            Some(&a.key.did_key()),
        )))
        .unwrap();
    let retained = query("--output", &output.to_string(), true);
    assert!(retained.status.success());
    assert_eq!(
        two,
        serde_json::from_slice::<serde_json::Value>(&retained.stdout).unwrap()
    );
    let by_key = query("--key", &key, true);
    let by_key: serde_json::Value = serde_json::from_slice(&by_key.stdout).unwrap();
    assert_eq!(by_key["holders"].as_array().unwrap().len(), 3);
    let absent = BlobCid::compute_bytes(b"absent");
    let zero = query("--output", &absent.to_string(), true);
    assert!(zero.status.success());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&zero.stdout).unwrap()["outputs"][0]["count"],
        0
    );
    assert!(!b
        .epr(&[
            "artifact",
            "holders",
            "--key",
            &key,
            "--output",
            &output.to_string()
        ])
        .status
        .success());
    assert!(!b
        .epr(&["artifact", "holders", "--output", "invalid-cid"])
        .status
        .success());
}

#[test]
fn fetch_plurality_queues_only_chosen_holders_and_records_divergence_once() {
    use eprfs_peer::{FetchDone, FetchFor, FetchRequest};
    use std::time::{Duration, Instant};
    let (keys, a, b) = household();
    let c = Device::new(&keys, "c");
    let d = Device::new(&keys, "d");
    for device in [&c, &d] {
        let request = device_enroll(a.root(), HANDLE, &device.key).unwrap();
        let authorization =
            device_authorize(a.root(), &serde_json::to_string(&request).unwrap(), &a.key).unwrap();
        device_bind(
            a.root(),
            &serde_json::to_string(&authorization).unwrap(),
            &device.key,
        )
        .unwrap();
        device_endpoint(a.root(), None, &device.node_id(), &device.key).unwrap();
    }
    let roster = format!(".eprfs/status/participants/{HANDLE}.jsonl");
    std::fs::copy(a.root().join(&roster), b.root().join(&roster)).unwrap();
    let scratch = TempDir::new().unwrap();
    let (method, descriptor) = method_file(scratch.path(), &["crate"]);
    let key_result = b
        .command()
        .envs(descriptor.non_path_inputs.iter())
        .args([
            "artifact",
            "key",
            "--method",
            method.to_str().unwrap(),
            "--repo",
            b.root().to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();
    assert!(key_result.status.success(), "{}", text(&key_result));
    let key = serde_json::from_slice::<serde_json::Value>(&key_result.stdout).unwrap()["keyCid"]
        .as_str()
        .unwrap()
        .to_string();
    let x = BlobCid::compute_bytes(b"majority");
    let y = BlobCid::compute_bytes(b"minority");
    let manifest = BlobCid::compute(b"manifest");
    let mut store = SidecarFlowStore::open(b.root()).unwrap();
    for (device, output) in [(&a, &x), (&c, &x), (&d, &y)] {
        store
            .append(FlowRecord::Event(produce_record(
                &key,
                output,
                &manifest.to_string(),
                Some(&device.key.did_key()),
            )))
            .unwrap();
    }
    let peer_dir = b.config.path().join("epr/peer");
    let _lock = elohim_epr_cli::peer::ServeLock::acquire(&peer_dir).unwrap();
    let queue = eprfs_peer::fetch_queue_dir(&peer_dir);
    eprfs_peer::ensure_queue_dir(&queue).unwrap();
    let expected_devices: std::collections::BTreeSet<_> = [a.key.did_key(), c.key.did_key()].into();
    let own_devices: std::collections::BTreeSet<_> = [a.key.did_key(), d.key.did_key()].into();
    let expected_requests = [
        (x.clone(), expected_devices.clone(), None),
        (x.clone(), expected_devices.clone(), None),
        (x.clone(), expected_devices, Some(x.clone())),
        (y.clone(), own_devices, None),
    ];
    let later_manifest = manifest.to_string();
    let worker = std::thread::spawn(move || {
        // Exercise the CLI/daemon queue boundary. The peer transport test independently moves
        // actual bytes from this same selection, including proving the divergent holder is idle.
        for (selected, expected_devices, expected_output) in expected_requests {
            let started = Instant::now();
            let (path, request) = loop {
                let found = std::fs::read_dir(&queue)
                    .unwrap()
                    .filter_map(Result::ok)
                    .find_map(|entry| {
                        let path = entry.path();
                        // Only a renamed `<stem>.json` is a queued request. The CLI writes it
                        // through a sibling `<stem>.tmp-…` and renames; claiming (and deleting)
                        // the temporary raced that rename and failed the fetch with ENOENT
                        // whenever the worker scanned between the write and the rename.
                        if path.extension().and_then(|e| e.to_str()) != Some("json") {
                            return None;
                        }
                        let request =
                            serde_json::from_slice::<FetchRequest>(&std::fs::read(&path).ok()?)
                                .ok()?;
                        Some((path, request))
                    });
                if let Some(found) = found {
                    break found;
                }
                assert!(
                    started.elapsed() < Duration::from_secs(30),
                    "request queued"
                );
                std::thread::sleep(Duration::from_millis(20));
            };
            assert_eq!(request.output, selected);
            assert_eq!(request.expect_output, expected_output);
            assert_eq!(
                request
                    .holders
                    .iter()
                    .map(|h| h.device.clone())
                    .collect::<std::collections::BTreeSet<_>>(),
                expected_devices
            );
            std::fs::remove_file(path).unwrap();
            eprfs_peer::write_queue_file(
                &eprfs_peer::fetch_done_path(&queue, &request.stem()),
                &FetchDone::Installed {
                    for_request: FetchFor::of(&request),
                    manifest: manifest.clone(),
                    manifest_from: request.holders[0].device.clone(),
                    fetched_from: vec![request.holders[0].device.clone()],
                    chunks_fetched: 1,
                    chunks_resumed: 0,
                    bytes_fetched: 8,
                    wasted: 0,
                    seconds: 0.01,
                },
            )
            .unwrap();
        }
    });
    for rule in ["plurality", "plurality", "expected", "own"] {
        if rule == "expected" {
            // A's later Y does not erase its X custody: --expect-output X still queues A.
            store
                .append(FlowRecord::Event(produce_record(
                    &key,
                    &y,
                    &later_manifest,
                    Some(&a.key.did_key()),
                )))
                .unwrap();
        }
        if rule == "own" {
            let mut own = produce_record(
                &key,
                &y,
                &BlobCid::compute(b"own-manifest").to_string(),
                None,
            );
            own.receiver = AgentRef(b.key.did_key());
            own.classified_as
                .retain(|slot| !slot.starts_with("measure:") && !slot.starts_with("reason:"));
            store.append(FlowRecord::Event(own)).unwrap();
        }
        let mut command = b.command();
        command.envs(descriptor.non_path_inputs.iter()).args([
            "artifact",
            "fetch",
            "--method",
            method.to_str().unwrap(),
            "--dest",
            scratch.path().join("binary").to_str().unwrap(),
            "--repo",
            b.root().to_str().unwrap(),
        ]);
        if rule == "expected" {
            command.args(["--expect-output", &x.to_string()]);
        }
        let out = command.output().unwrap();
        assert!(out.status.success(), "{}", text(&out));
        let said = String::from_utf8_lossy(&out.stdout);
        assert_eq!(
            said.lines()
                .filter(|line| line.starts_with("divergence:"))
                .count(),
            1
        );
        assert!(said.contains(&format!("rule {rule}")), "{said}");
        let observations: Vec<_> = events(b.root())
            .into_iter()
            .filter(|e| e.classified_as.contains(&"env:kind=divergence".into()))
            .collect();
        assert_eq!(observations.len(), 1);
        let observation = &observations[0];
        assert_eq!(observation.action, ReaVerb::Cite);
        let mut outputs = [x.to_string(), y.to_string()];
        outputs.sort();
        for tag in [
            format!("env:key={key}"),
            format!("env:outputs={}", outputs.join(",")),
            format!("env:chosen={x}"),
            "env:rule=plurality".into(),
        ] {
            assert!(observation.classified_as.contains(&tag), "{tag}");
        }
    }
    worker.join().unwrap();
}

#[test]
fn daemon_install_refusal_records_and_prints_no_divergence() {
    use std::time::{Duration, Instant};
    let (_keys, a, b) = household();
    let scratch = TempDir::new().unwrap();
    let (method, descriptor) = method_file(scratch.path(), &["crate"]);
    let result = b
        .command()
        .envs(descriptor.non_path_inputs.iter())
        .args([
            "artifact",
            "key",
            "--method",
            method.to_str().unwrap(),
            "--repo",
            b.root().to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();
    assert!(result.status.success(), "{}", text(&result));
    let key = serde_json::from_slice::<serde_json::Value>(&result.stdout).unwrap()["keyCid"]
        .as_str()
        .unwrap()
        .to_string();
    let x = BlobCid::compute_bytes(b"remote-output");
    let y = BlobCid::compute_bytes(b"local-only-output");
    let manifest = BlobCid::compute(b"manifest").to_string();
    let mut store = SidecarFlowStore::open(b.root()).unwrap();
    let mut own = produce_record(&key, &y, &manifest, None);
    own.receiver = AgentRef(b.key.did_key());
    own.classified_as
        .retain(|s| !s.starts_with("measure:") && !s.starts_with("reason:"));
    store.append(FlowRecord::Event(own)).unwrap();

    let policy = scratch.path().join("policy.json");
    std::fs::write(
        &policy,
        r#"{"berth":{"channel":{"crosses":{"artifact-produce@1":["produce"]}}}}"#,
    )
    .unwrap();
    let allowed = scratch.path().join("allowed");
    std::fs::create_dir(&allowed).unwrap();
    let log = scratch.path().join("daemon.log");
    // Only this test's child is stopped, including on an assertion failure.
    struct Daemon(std::process::Child);
    impl Drop for Daemon {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let mut daemon = Daemon(
        b.command()
            .args([
                "peer",
                "serve",
                "--root",
                b.root().to_str().unwrap(),
                "--policy",
                policy.to_str().unwrap(),
                "--listen",
                "127.0.0.1:0",
                "--install-root",
                allowed.to_str().unwrap(),
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::fs::File::create(&log).unwrap())
            .spawn()
            .unwrap(),
    );
    let peer_dir = b.config.path().join("epr/peer");
    let started = Instant::now();
    loop {
        assert!(
            daemon.0.try_wait().unwrap().is_none(),
            "{}",
            std::fs::read_to_string(&log).unwrap()
        );
        if eprfs_peer::queue_dir_is_private(&eprfs_peer::fetch_queue_dir(&peer_dir))
            && elohim_epr_cli::peer::ServeLock::held(&peer_dir) == Some(true)
        {
            break;
        }
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "daemon ready: {}",
            std::fs::read_to_string(&log).unwrap()
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    let dest = scratch.path().join("outside");
    let fetch = || {
        b.command()
            .envs(descriptor.non_path_inputs.iter())
            .args([
                "artifact",
                "fetch",
                "--method",
                method.to_str().unwrap(),
                "--repo",
                b.root().to_str().unwrap(),
                "--dest",
                dest.to_str().unwrap(),
                "--stall",
                "1",
                "--total",
                "1",
            ])
            .output()
            .unwrap()
    };
    let local_only = fetch();
    assert!(!local_only.status.success());
    assert!(
        text(&local_only).contains("local-only-custody"),
        "{}",
        text(&local_only)
    );
    assert!(text(&local_only).contains(&y.to_string()));
    // Rule (b) cannot select local-only Y. X is now the remote plurality, but installation refuses.
    store
        .append(FlowRecord::Event(produce_record(
            &key,
            &x,
            &manifest,
            Some(&a.key.did_key()),
        )))
        .unwrap();
    let refused = fetch();
    assert!(!refused.status.success());
    assert!(
        text(&refused).contains("install-refused"),
        "{}",
        text(&refused)
    );
    assert!(
        text(&refused).contains("outside this daemon's install roots"),
        "{}",
        text(&refused)
    );
    assert!(!text(&refused).contains("divergence:"));
    assert!(!dest.exists());
    assert_eq!(
        events(b.root())
            .iter()
            .filter(|e| e.classified_as.contains(&"env:kind=divergence".into()))
            .count(),
        0
    );
}

#[test]
fn artifact_family_lists_v2_v3_keys_requires_and_self_without_selection() {
    let (_keys, a, b) = household();
    let scratch = TempDir::new().unwrap();
    let (_, base) = method_file(scratch.path(), &["crate/src"]);
    let input = eprfs_core::KeyInput::Raw(BlobCid::compute_bytes(b"source"));
    let mut expected = std::collections::BTreeMap::new();
    let mut store = SidecarFlowStore::open(b.root()).unwrap();
    let mut family = None;
    // K2 refuses declared target.* at put until the build applies it (K6). Construct Produce
    // records at the fold boundary to prove family grouping without claiming such builds ran.
    for isa in ["x86-64-v2", "x86-64-v3"] {
        let mut descriptor = base.clone();
        descriptor
            .non_path_inputs
            .insert("target.isa".into(), isa.into());
        let this_family = descriptor.family_cid().unwrap();
        assert_eq!(family.get_or_insert(this_family.clone()), &this_family);
        let key = eprfs_peer::key_cid(&eprfs_core::DerivationKey {
            method: descriptor.cid().unwrap(),
            input: input.clone(),
        })
        .unwrap()
        .to_string();
        let output = BlobCid::compute_bytes(isa.as_bytes());
        let requires = BlobCid::compute_bytes(
            &serde_json::to_vec(&serde_json::json!([{
                "axis": "isa.x86", "predicate": {"kind": "equal", "value": isa},
                "source": "declared"
            }]))
            .unwrap(),
        )
        .to_string();
        let mut row = produce_record(
            &key,
            &output,
            &BlobCid::compute(b"manifest").to_string(),
            Some(&a.key.did_key()),
        );
        row.classified_as.extend([
            format!("env:family={this_family}"),
            format!("env:requires={requires}"),
        ]);
        store.append(FlowRecord::Event(row.clone())).unwrap();
        row.classified_as.retain(|slot| !slot.starts_with("via:"));
        row.receiver = AgentRef(b.key.did_key());
        store.append(FlowRecord::Event(row)).unwrap();
        expected.insert(key, (output.to_string(), requires));
    }
    assert_eq!(expected.len(), 2);
    let family = family.unwrap().to_string();
    let out = b.epr(&[
        "artifact",
        "holders",
        "--family",
        &family,
        "--repo",
        b.root().to_str().unwrap(),
        "--json",
    ]);
    assert!(out.status.success(), "{}", text(&out));
    let result: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let variants = result["variants"].as_array().unwrap();
    assert_eq!(variants.len(), 2);
    for variant in variants {
        let (output, requires) = &expected[variant["key"].as_str().unwrap()];
        assert_eq!(variant["outputs"], serde_json::json!([output]));
        assert_eq!(variant["requires"]["Unanimous"], *requires);
        assert!(variant["claims"]
            .as_array()
            .unwrap()
            .iter()
            .all(|claim| claim["availability"] == "unavailable"));
        let holders = variant["holders"].as_array().unwrap();
        assert_eq!(holders.len(), 2);
        assert_eq!(holders.iter().filter(|h| h["self"] == true).count(), 1);
        for holder in holders {
            assert_eq!(holder["self"], holder["device"] == b.key.did_key());
            assert_eq!(holder["output"], *output);
            assert_eq!(holder["requires"]["Unanimous"], *requires);
        }
    }
    let out = b.epr(&[
        "artifact",
        "holders",
        "--family",
        &family,
        "--repo",
        b.root().to_str().unwrap(),
    ]);
    assert!(out.status.success(), "{}", text(&out));
    assert!(text(&out).contains(" self"));
    for (key, (_, requires)) in expected {
        assert!(text(&out).contains(&format!(
            "key {key} requires Unanimous({requires} (unavailable))"
        )));
    }
}

#[test]
fn artifact_family_preserves_conflicts_older_holders_and_local_availability() {
    let (_keys, a, b) = household();
    let family = BlobCid::compute(b"family").to_string();
    let key = BlobCid::compute(b"key").to_string();
    let output = BlobCid::compute_bytes(b"output");
    let first_bytes = eprfs_peer::Bytes::from_static(b"[1]");
    let first = BlobCid::compute_bytes(&first_bytes);
    let second = BlobCid::compute_bytes(b"[2]");
    let mut store = SidecarFlowStore::open(b.root()).unwrap();
    let mut row = produce_record(
        &key,
        &output,
        &BlobCid::compute(b"manifest").to_string(),
        Some(&a.key.did_key()),
    );
    // B's own pre-K3 custody has neither family nor Requires. A later tags the key.
    let mut older = row.clone();
    older.classified_as.retain(|slot| !slot.starts_with("via:"));
    older.receiver = AgentRef(b.key.did_key());
    store.append(FlowRecord::Event(older)).unwrap();
    row.classified_as.extend([
        format!("env:family={family}"),
        format!("env:requires={first}"),
    ]);
    store.append(FlowRecord::Event(row.clone())).unwrap();
    row.classified_as
        .retain(|slot| !slot.starts_with("env:requires=") && !slot.starts_with("env:family="));
    row.classified_as.push(format!("env:requires={second}"));
    store.append(FlowRecord::Event(row)).unwrap();

    // Only the first contract is present on the querying device. No peer retrieval is involved.
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let store = elohim_epr_cli::payload::open_payload_store(
            &b.payload(),
            elohim_epr_cli::payload::Access::Shared,
        )
        .await
        .unwrap()
        .store;
        store.put_blob_verified(&first, first_bytes).await.unwrap();
    });
    let args = [
        "artifact",
        "holders",
        "--family",
        &family,
        "--repo",
        b.root().to_str().unwrap(),
    ];
    let out = b.epr(&[args.as_slice(), &["--json"]].concat());
    assert!(out.status.success(), "{}", text(&out));
    let result: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let variant = &result["variants"][0];
    let conflicts = variant["requires"]["Conflict"].as_array().unwrap();
    assert_eq!(conflicts.len(), 3);
    assert!(conflicts.contains(&serde_json::json!(first)));
    assert!(conflicts.contains(&serde_json::json!(second)));
    assert!(conflicts.contains(&serde_json::Value::Null));
    let holders = variant["holders"].as_array().unwrap();
    assert_eq!(holders.len(), 2);
    let self_holder = holders
        .iter()
        .find(|h| h["device"] == b.key.did_key())
        .unwrap();
    assert_eq!(self_holder["requires"], "Unclaimed");
    assert_eq!(self_holder["self"], true);
    let remote_holder = holders
        .iter()
        .find(|h| h["device"] == a.key.did_key())
        .unwrap();
    assert_eq!(
        remote_holder["requires"]["Conflict"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let claims = variant["claims"].as_array().unwrap();
    assert_eq!(claims.len(), 3);
    for (cid, status) in [(&first, "local"), (&second, "unavailable")] {
        assert!(claims
            .iter()
            .any(|c| c["requires"] == cid.to_string() && c["availability"] == status));
    }
    let by_key = b.epr(&[
        "artifact",
        "holders",
        "--key",
        &key,
        "--repo",
        b.root().to_str().unwrap(),
        "--json",
    ]);
    assert!(by_key.status.success(), "{}", text(&by_key));
    let by_key: serde_json::Value = serde_json::from_slice(&by_key.stdout).unwrap();
    assert_eq!(by_key["holders"].as_array().unwrap().len(), holders.len());
    for holder in holders {
        assert!(by_key["holders"]
            .as_array()
            .unwrap()
            .iter()
            .any(|h| h["device"] == holder["device"]
                && h["output"] == holder["output"]
                && h["self"] == holder["self"]));
    }
    let out = b.epr(&args);
    assert!(out.status.success(), "{}", text(&out));
    let printed = text(&out);
    assert!(printed.contains("Conflict{"));
    assert!(printed.contains(&format!("{first} (local)")));
    assert!(printed.contains(&format!("{second} (unavailable)")));
    assert!(printed.contains(&format!(
        "{} {} requires Unclaimed self",
        b.key.did_key(),
        output
    )));
}

#[test]
fn artifact_sysroot_put_then_output_fetch_uses_explicit_cid_authority() {
    sysroot_put_then_output_fetch(false);
}

#[test]
fn artifact_sysroot_put_records_family_and_canonical_requires() {
    sysroot_put_then_output_fetch(true);
}

fn sysroot_put_then_output_fetch(with_requires: bool) {
    use eprfs_peer::{FetchAuthority, FetchDone, FetchFor, FetchRequest};
    use std::time::{Duration, Instant};
    let (_keys, a, b) = household();
    let scratch = TempDir::new().unwrap();
    let input = scratch.path().join("input");
    std::fs::create_dir(&input).unwrap();
    std::fs::write(input.join("header.h"), b"/* content-pinned header */").unwrap();
    let archive = scratch.path().join("sysroot.eprsys");
    let packed = a.epr(&[
        "artifact",
        "sysroot",
        "pack",
        "--from",
        input.to_str().unwrap(),
        "--triple",
        "x86_64-unknown-linux-gnu",
        "--out",
        archive.to_str().unwrap(),
        "--json",
    ]);
    assert!(packed.status.success(), "{}", text(&packed));
    let bytes = std::fs::read(&archive).unwrap();
    let output = BlobCid::compute_bytes(&bytes);
    let (stub, log) = brit_stub(scratch.path());
    let requires_file = scratch.path().join("requires.json");
    let canonical_requires = br#"[{"axis":"isa.x86","predicate":{"kind":"equal","value":"x86-64-v3"},"source":"declared"}]"#;
    std::fs::write(
        &requires_file,
        br#"[ { "source": "declared", "predicate": { "value": "x86-64-v3", "kind": "equal" }, "axis": "isa.x86" } ]"#,
    )
    .unwrap();
    let requires_cid = BlobCid::compute_bytes(canonical_requires);
    let mut command = a.command();
    command.env("BRIT_BUILD_REF_BIN", &stub).args([
        "artifact",
        "sysroot",
        "put",
        "--archive",
        archive.to_str().unwrap(),
        "--repo",
        a.root().to_str().unwrap(),
        "--cross",
        "--json",
    ]);
    if with_requires {
        command.arg("--requires").arg(&requires_file);
    }
    let put = command.output().unwrap();
    assert!(put.status.success(), "{}", text(&put));
    let recorded: serde_json::Value = serde_json::from_slice(&put.stdout).unwrap();
    assert_eq!(recorded["output"], output.to_string());
    let verified = elohim_epr_cli::artifact::sysroot::verify(&bytes).unwrap();
    assert_eq!(recorded["input"], verified.cid().unwrap().to_string());
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let family = runtime.block_on(async {
        let store = eprfs_peer::open_payload_store(&a.payload()).unwrap();
        let held = store
            .fetch_blob(&verified.cid().unwrap(), eprfs_core::FetchPolicy::LocalOnly)
            .await
            .unwrap();
        assert_eq!(held.bytes.as_ref(), verified.canonical_bytes().unwrap());
        let method_cid = BlobCid::parse(recorded["method"].as_str().unwrap()).unwrap();
        let method_bytes = store
            .fetch_blob(&method_cid, eprfs_core::FetchPolicy::LocalOnly)
            .await
            .unwrap();
        let method: MethodDescriptor =
            serde::Deserialize::deserialize(elohim_epr::cbor::decode(&method_bytes.bytes).unwrap())
                .unwrap();
        assert_eq!(method.name, "sysroot-pack");
        assert_eq!(method.cid().unwrap(), method_cid);
        if with_requires {
            let held = store
                .fetch_blob(&requires_cid, eprfs_core::FetchPolicy::LocalOnly)
                .await
                .unwrap();
            assert_eq!(held.bytes.as_ref(), canonical_requires);
        }
        method.family_cid().unwrap()
    });
    assert!(std::fs::read_to_string(log)
        .unwrap()
        .contains("--step sysroot-pack:sysroot"));
    let manifest = BlobCid::parse(recorded["manifest"].as_str().unwrap()).unwrap();
    let produces: Vec<_> = events(a.root())
        .into_iter()
        .filter(|e| e.action == ReaVerb::Produce)
        .collect();
    assert_eq!(produces.len(), 1);
    assert!(produces[0]
        .classified_as
        .contains(&format!("env:family={family}")));
    let requires_slots: Vec<_> = produces[0]
        .classified_as
        .iter()
        .filter(|slot| slot.starts_with("env:requires="))
        .cloned()
        .collect();
    assert_eq!(
        requires_slots,
        if with_requires {
            vec![format!("env:requires={requires_cid}")]
        } else {
            Vec::<String>::new()
        }
    );
    assert!(produces[0]
        .classified_as
        .contains(&"basis-key:sysroot-manifest".into()));
    let mut crossed = produces[0].clone();
    crossed
        .classified_as
        .push(format!("via:{}", a.key.did_key()));
    SidecarFlowStore::open(b.root())
        .unwrap()
        .append(FlowRecord::Event(crossed))
        .unwrap();
    let peer_dir = b.config.path().join("epr/peer");
    let _lock = elohim_epr_cli::peer::ServeLock::acquire(&peer_dir).unwrap();
    let queue = eprfs_peer::fetch_queue_dir(&peer_dir);
    eprfs_peer::ensure_queue_dir(&queue).unwrap();
    let expected = output.clone();
    let holder = a.key.did_key();
    let expected_bytes = bytes.clone();
    let worker = std::thread::spawn(move || {
        // This fixture tests CLI/custody/queue integration. The peer suite separately exercises
        // real tier method verification, bounded transfer, whole-blob verification and install.
        let started = Instant::now();
        let (path, request) = loop {
            let found = std::fs::read_dir(&queue)
                .unwrap()
                .filter_map(Result::ok)
                .find_map(|entry| {
                    let path = entry.path();
                    if path.extension().and_then(|x| x.to_str()) != Some("json") {
                        return None;
                    }
                    let request: FetchRequest =
                        serde_json::from_slice(&std::fs::read(&path).ok()?).ok()?;
                    if path != eprfs_peer::fetch_request_path(&queue, &request.stem()) {
                        return None;
                    }
                    Some((path, request))
                });
            if let Some(found) = found {
                break found;
            }
            assert!(started.elapsed() < Duration::from_secs(30));
            std::thread::sleep(Duration::from_millis(20));
        };
        assert_eq!(
            request.authority,
            FetchAuthority::Output {
                cid: expected.clone()
            }
        );
        request.authority.validate(&request.output).unwrap();
        assert_eq!(request.output, expected);
        assert_eq!(request.holders.len(), 1);
        assert_eq!(request.holders[0].device, holder);
        assert_eq!(request.holders[0].manifest, manifest);
        assert_eq!(BlobCid::compute_bytes(&expected_bytes), request.output);
        let stage = request.dest.with_extension("stage");
        std::fs::write(&stage, expected_bytes).unwrap();
        std::fs::rename(stage, &request.dest).unwrap();
        std::fs::remove_file(path).unwrap();
        eprfs_peer::write_queue_file(
            &eprfs_peer::fetch_done_path(&queue, &request.stem()),
            &FetchDone::Installed {
                for_request: FetchFor::of(&request),
                manifest,
                manifest_from: holder.clone(),
                fetched_from: vec![holder],
                chunks_fetched: 1,
                chunks_resumed: 0,
                bytes_fetched: bytes.len() as u64,
                wasted: 0,
                seconds: 0.01,
            },
        )
        .unwrap();
    });
    let dest = scratch.path().join("fetched.eprsys");
    let fetch = |cid: &BlobCid| {
        b.epr(&[
            "artifact",
            "fetch",
            "--output",
            &cid.to_string(),
            "--dest",
            dest.to_str().unwrap(),
            "--repo",
            b.root().to_str().unwrap(),
            "--json",
        ])
    };
    let result = fetch(&output);
    worker.join().unwrap();
    assert!(result.status.success(), "{}", text(&result));
    let answer: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert!(answer["keyCid"].is_null());
    assert_eq!(answer["authority"]["kind"], "output");
    assert_eq!(
        BlobCid::compute_bytes(&std::fs::read(&dest).unwrap()),
        output
    );
    let missing = fetch(&BlobCid::compute_bytes(b"unheld"));
    assert_eq!(missing.status.code(), Some(3));
    assert!(
        text(&missing).contains("expected-output-unavailable"),
        "{}",
        text(&missing)
    );
}

#[test]
fn artifact_executable_put_refuses_reserved_archive_method_before_recording() {
    let keys = TempDir::new().unwrap();
    let device = Device::new(&keys, "reserved");
    let scratch = TempDir::new().unwrap();
    let (method, mut descriptor) = method_file(scratch.path(), &["crate/src"]);
    descriptor.name = "sysroot-pack".into();
    descriptor.step = Some("sysroot-pack:elohim-storage".into());
    let rustc = Command::new("rustc").arg("-vV").output().unwrap();
    assert!(rustc.status.success());
    let rustc = String::from_utf8(rustc.stdout).unwrap();
    let llvm = rustc
        .lines()
        .find_map(|line| line.strip_prefix("LLVM version: "))
        .unwrap()
        .to_owned();
    descriptor.tools.insert("rustc".into(), rustc);
    std::fs::write(&method, serde_json::to_vec(&descriptor).unwrap()).unwrap();
    let key = elohim_epr_cli::artifact::closure_key(device.root(), &descriptor).unwrap();
    let (stub, log) = brit_stub(scratch.path());
    let bytes = scratch.path().join("binary");
    std::fs::write(
        &bytes,
        artifact_elf(format!("Linker: LLD {llvm}\0").as_bytes()),
    )
    .unwrap();
    let out = device
        .command()
        .env("BRIT_BUILD_REF_BIN", &stub)
        .args([
            "artifact",
            "put",
            "--method",
            method.to_str().unwrap(),
            "--input",
            &key.input.to_string(),
            "--bytes",
            bytes.to_str().unwrap(),
            "--project",
            "elohim-storage",
            "--repo",
            device.root().to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3), "{}", text(&out));
    assert!(
        text(&out).contains("method/validator mismatch"),
        "{}",
        text(&out)
    );
    assert!(!log.exists());
    assert!(events(device.root()).is_empty());
}

#[test]
fn artifact_sysroot_put_bounds_initial_archive_read() {
    let scratch = TempDir::new().unwrap();
    let archive = scratch.path().join("oversize.eprsys");
    std::fs::File::create(&archive)
        .unwrap()
        .set_len(elohim_epr_cli::artifact::sysroot::limits::MAX_ARCHIVE_BYTES + 1)
        .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_epr"))
        .args([
            "artifact",
            "sysroot",
            "put",
            "--archive",
            archive.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3), "{}", text(&out));
    assert!(
        text(&out).contains("archive read byte limit"),
        "{}",
        text(&out)
    );
}
