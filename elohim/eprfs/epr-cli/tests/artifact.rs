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
    git(
        dir.path(),
        &[
            "add",
            ".epr-meta/repository.yaml",
            "crate/src/main.rs",
            "README.md",
        ],
    );
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
            .env_remove("EPR_PEER_RELAYS")
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

/// A stub `brit-build-ref`: `signer --json` names a signer, `build put` succeeds; each write is
/// logged one line per invocation; read-only signer probes do not mutate this log.
fn brit_stub(dir: &Path) -> (PathBuf, PathBuf) {
    let log = dir.join("brit.log");
    let stub = dir.join("brit-build-ref");
    std::fs::write(
        &stub,
        format!(
            "#!/bin/sh\ncase \"$*\" in\n  *signer*) echo '{{\"agentId\":\"{AGENT_ID}\"}}' ;;\n  *) echo \"$*\" >> '{}'; echo '{{}}' ;;\nesac\n",
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
async fn artifact_mesh_fixture_passes_preflight_and_records_one_produce() {
    put_and_mount(false, false).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn artifact_overridden_flags_store_the_keyed_method_and_mount() {
    put_and_mount(true, true).await;
}

fn recordable_mesh_fixture(
    root: &Path,
    scratch: &Path,
    method: &Path,
    descriptor: &mut MethodDescriptor,
) {
    std::fs::write(
        root.join("crate/Cargo.toml"),
        "[package]\nname = \"elohim-storage\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    std::fs::write(
        root.join("crate/Cargo.lock"),
        "version = 4\n[[package]]\nname = \"elohim-storage\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    git(root, &["add", "crate/Cargo.toml", "crate/Cargo.lock"]);
    git(root, &["commit", "-qm", "cargo closure"]);
    for bin in ["elohim-storage", "artifact"] {
        std::fs::write(
            scratch.join(format!("{bin}.d")),
            format!(
                "{}: {}\n",
                scratch.join(bin).display(),
                root.join("crate/src/main.rs").display()
            ),
        )
        .unwrap();
    }
    let tools = scratch.join("tools");
    fake_cc(&tools, "cc fixture");
    fake_tool(
        &tools,
        "cargo",
        &format!(
            r#"echo '{{"packages":[{{"manifest_path":"{}/crate/Cargo.toml","source":null,"targets":[{{"name":"elohim-storage","kind":["bin"],"src_path":"{}/crate/src/main.rs"}}]}}]}}'"#,
            root.display(),
            root.display()
        ),
    );
    descriptor
        .non_path_inputs
        .insert("crate".into(), "crate".into());
    descriptor
        .non_path_inputs
        .insert("PATH".into(), format!("{}:/bin", tools.display()));
    for path in ["crate/Cargo.toml", "crate/Cargo.lock"] {
        descriptor
            .view
            .push(TreePath::from_slash_bytes(path.as_bytes()).unwrap());
    }
    std::fs::write(method, serde_json::to_vec(descriptor).unwrap()).unwrap();
}

async fn put_and_mount(overridden: bool, with_requires: bool) {
    let keys = TempDir::new().unwrap();
    let a = Device::new(&keys, "a");
    let scratch = TempDir::new().unwrap();
    let (method, mut descriptor) = method_file(scratch.path(), &["crate/src", "README.md"]);
    let bytes = scratch.path().join("elohim-storage");
    let mut artifact = artifact_elf(b"Linker: LLD 22.1.2\0");
    artifact.extend((0..700_000u32).map(|i| (i * 31 % 251) as u8));
    std::fs::write(&bytes, &artifact).unwrap();
    recordable_mesh_fixture(a.root(), scratch.path(), &method, &mut descriptor);
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
    // The document is the exact byte-profile input, not a reconstructed diagnostic summary.
    let shown = command()
        .args(["artifact", "inputs", "show", &method, "--repo", &repo])
        .output()
        .unwrap();
    assert!(shown.status.success(), "{}", text(&shown));
    assert_eq!(BlobCid::compute_bytes(&shown.stdout).to_string(), input);
    let expected_lines = elohim_epr_cli::artifact::closure_key(a.root(), &descriptor)
        .unwrap()
        .input_lines
        .unwrap();
    assert_eq!(shown.stdout, expected_lines.as_bytes());

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
    let events_before =
        serde_json::to_value(SidecarFlowStore::open(a.root()).unwrap().events().unwrap()).unwrap();
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
    let reopened = elohim_epr_cli::payload::open_payload_store(
        &a.payload(),
        elohim_epr_cli::payload::Access::Shared,
    )
    .await
    .unwrap()
    .store;
    let rejected_cid = BlobCid::compute_bytes(&unknown_os);
    let absence = reopened
        .fetch_blob(&rejected_cid, eprfs_core::FetchPolicy::LocalOnly)
        .await;
    assert!(
        matches!(absence,
            Err(eprfs_core::EprfsError::BlobNotFound(ref cid) | eprfs_core::EprfsError::BlobNotLocal(ref cid))
                if cid == &rejected_cid
        ),
        "expected local payload absence: {absence:?}"
    );
    let after = SidecarFlowStore::open(a.root()).unwrap().events().unwrap();
    assert_eq!(serde_json::to_value(&after).unwrap(), events_before);
    let (fresh_mount, _) = BlobMount::new(reopened, did.clone());
    assert!(fresh_mount.rebuild_from_flow(&after).await.is_empty());
    assert!(fresh_mount
        .held(&BlobCid::compute_bytes(&unknown_os))
        .is_none());
    std::fs::write(&bytes, &artifact).unwrap();

    // A closure that moved since the build started is never recorded.
    let out = put("bafkr4ihkr4ld3m4gqkjf4reryxsy2s5tkbxprqkow6fin2iiyvreuzzab4");
    assert_eq!(out.status.code(), Some(3), "{}", text(&out));
    assert!(text(&out).contains("the closure moved"), "{}", text(&out));
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
        inputs_only: false,
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
            inputs_only: true,
            ..ours.clone()
        },
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
        "case \"$1\" in\n--version) echo '{version}' ;;\n-E) echo '#define __SSE2__ 1' ;;\n-print-file-name=*) exec /usr/bin/cc \"$1\" ;;\nesac\n"
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
    let (method, mut descriptor) = method_file(scratch.path(), &["crate/src"]);
    let bytes = scratch.path().join("elohim-storage");
    std::fs::write(&bytes, artifact_elf(b"Linker: LLD 22.1.2\0")).unwrap();
    recordable_mesh_fixture(a.root(), scratch.path(), &method, &mut descriptor);
    let (stub, _log) = brit_stub(scratch.path());
    let repo = a.root().to_str().unwrap().to_string();
    let method = method.to_str().unwrap().to_string();
    let input = {
        let out = a
            .command()
            .envs(&descriptor.non_path_inputs)
            .args([
                "artifact", "key", "--method", &method, "--repo", &repo, "--print", "input",
            ])
            .output()
            .unwrap();
        assert!(out.status.success(), "{}", text(&out));
        String::from_utf8(out.stdout).unwrap().trim().to_string()
    };
    let put = || {
        a.command()
            .envs(&descriptor.non_path_inputs)
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
    assert!(
        !a.payload().exists(),
        "refused preflight never opens the store"
    );
    std::fs::create_dir_all(a.payload()).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(a.payload(), std::fs::Permissions::from_mode(0o700)).unwrap();
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

#[path = "support/artifact_link_key.rs"]
mod link_key;

#[path = "support/artifact_holders.rs"]
mod holders;

#[path = "support/artifact_sysroot_put.rs"]
mod sysroot_put;

#[path = "support/artifact_inputs.rs"]
mod inputs;

#[path = "support/artifact_preflight.rs"]
mod preflight;
