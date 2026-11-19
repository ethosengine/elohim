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
#[path = "support/toolchain_pin.rs"]
mod toolchain_pin;

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
        // harness's own cargo or C-family variables reach it.
        for (name, _) in std::env::vars() {
            if name.starts_with("CARGO_")
                || name.starts_with("RUST")
                || elohim_epr_cli::artifact::NATIVE_BUILD_ENV.contains(&name.as_str())
                || elohim_epr_cli::artifact::is_c_family_var(&name)
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
    // No declared target, so no C level was checked and no marker is written.
    assert!(
        isa_c_slots(produce).is_empty(),
        "{:?}",
        produce.classified_as
    );
    // Row 12: the statement crosses with the Produce, byte for byte, so a sibling can judge
    // the output before fetching it.
    {
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
        let inline = produce
            .classified_as
            .iter()
            .find_map(|s| s.strip_prefix("env:requires-doc="))
            .expect("the requirement document crosses inline");
        let decoded = URL_SAFE_NO_PAD.decode(inline).unwrap();
        assert_eq!(decoded.as_slice(), canonical_requires);
        assert_eq!(BlobCid::compute_bytes(&decoded), requires_cid);
    }

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

/// The descriptor inputs of a build declared at the x86-64 target, its C flags recorded as the
/// gate exports them (pool-policy.json declared_targets).
const DECLARED_X86_64: [(&str, &str); 3] = [
    ("target.isa", "x86-64"),
    ("target.cflags", "-march=x86-64"),
    ("CFLAGS_x86_64_unknown_linux_gnu", "-march=x86-64"),
];

/// One build script of the build put is handed, as cargo and cc leave it: its crates.io crate,
/// the `running:` lines cc logged (`$OUT` is its OUT_DIR) and the objects it left there.
#[derive(Clone)]
struct BuildScript {
    package: &'static str,
    running: Vec<String>,
    objects: &'static [&'static str],
}

/// A gcc compile as cc 1.11.1 logs it, `flags` before the output, the source last.
fn cc_compile(flags: &str, source: &str) -> String {
    let flags: String = flags
        .split_whitespace()
        .map(|f| format!(" {f:?}"))
        .collect();
    format!("\"gcc\" \"-O2\" \"-fPIC\" \"-m64\"{flags} \"-o\" \"$OUT/x.o\" \"-c\" {source:?}")
}

/// A crate's build script with these compile commands, cc's family probe and its archive.
fn script(package: &'static str, compiles: &[String]) -> BuildScript {
    let mut running = vec!["\"gcc\" \"-E\" \"$OUT/detect_compiler_family.c\"".to_string()];
    running.extend(compiles.iter().cloned());
    running.push("\"ar\" \"cqD\" \"$OUT/libx.a\" \"$OUT/x.o\"".into());
    BuildScript {
        package,
        running,
        objects: &["x.o", "libx.a"],
    }
}

/// ring's build with its curve25519.c compiled at `flags`.
fn ring_at(flags: &str) -> BuildScript {
    script(
        "ring",
        &[cc_compile(flags, "crypto/curve25519/curve25519.c")],
    )
}

/// The build messages a put is handed.
#[derive(Clone)]
enum Messages {
    /// No `--build-messages` at all.
    Absent,
    /// The build of these bytes.
    Of(Vec<BuildScript>),
    /// A build whose bin executable is other bytes.
    OtherBytes(Vec<BuildScript>),
}

/// Write `messages` for the bin `bin` built at `exe` into `dir`, as cargo's
/// `--message-format=json` names them, each script's log in `run/stdout` beside its OUT_DIR.
fn build_messages(dir: &Path, bin: &str, exe: &Path, messages: &Messages) -> Option<PathBuf> {
    let (scripts, exe) = match messages {
        Messages::Absent => return None,
        _ if !dir.exists() => {
            std::fs::create_dir_all(dir).unwrap();
            return build_messages(dir, bin, exe, messages);
        }
        Messages::Of(scripts) => (scripts, exe.to_path_buf()),
        Messages::OtherBytes(scripts) => {
            let other = dir.join("other-exe");
            std::fs::write(&other, b"other bytes").unwrap();
            (scripts, other)
        }
    };
    let mut lines = vec![serde_json::json!({
        "reason": "compiler-artifact",
        "package_id": format!("path+file:///repo/crate#{bin}@0.1.0"),
        "target": {"kind": ["bin"], "name": bin},
        "executable": exe,
        "fresh": true,
    })
    .to_string()];
    for (n, script) in scripts.iter().enumerate() {
        let unit = dir
            .join("build")
            .join(script.package)
            .join(format!("{n:04}"));
        let out = unit.join("out");
        std::fs::create_dir_all(&out).unwrap();
        std::fs::create_dir_all(unit.join("run")).unwrap();
        let log: String = script
            .running
            .iter()
            .map(|line| {
                format!(
                    "running: {}\n",
                    line.replace("$OUT", &out.display().to_string())
                )
            })
            .collect();
        // As cargo leaves it: the log stamped with the script's start, then what the run wrote.
        std::fs::write(unit.join("run/stdout"), log).unwrap();
        std::fs::File::options()
            .write(true)
            .open(unit.join("run/stdout"))
            .unwrap()
            .set_modified(std::time::SystemTime::now() - std::time::Duration::from_secs(2))
            .unwrap();
        for object in script.objects {
            std::fs::write(out.join(object), b"obj").unwrap();
        }
        lines.push(
            serde_json::json!({
                "reason": "build-script-executed",
                "package_id": format!(
                    "registry+https://github.com/rust-lang/crates.io-index#{}@1.0.0",
                    script.package
                ),
                "out_dir": out,
            })
            .to_string(),
        );
    }
    let path = dir.join("build-messages.json");
    std::fs::write(&path, lines.join("\n") + "\n").unwrap();
    Some(path)
}

/// Put one recordable mesh build with `inputs` recorded (and set), handed `messages` as the
/// evidence of its C level, under the fixture's declared target record. The put's output and
/// every Produce recorded.
fn declared_put(inputs: &[(&str, &str)], messages: &Messages) -> (Output, Vec<FlowEvent>) {
    declared_put_under(inputs, messages, Some(&declared_policy(X86_64_RECORD)))
}

/// The fixture's declared target record, as genesis/agentic/pool-policy.json writes it: the
/// fixture's triple (its rustc's host) for its workspace (`crate`, the directory of its lock).
const X86_64_RECORD: &str = r#""x86_64-unknown-linux-gnu": {"isa": "x86-64", "cflags": "-march=x86-64", "env": {"CC_ENABLE_DEBUG_OUTPUT": "1"}, "workspaces": ["crate"]}"#;

fn declared_policy(records: &str) -> String {
    format!("{{\"declared_targets\": {{\"_comment\": \"fixture\", {records}}}}}\n")
}

/// [`declared_put`] with `policy` committed as the checkout's genesis/agentic/pool-policy.json
/// (none when `None`).
fn declared_put_under(
    inputs: &[(&str, &str)],
    messages: &Messages,
    policy: Option<&str>,
) -> (Output, Vec<FlowEvent>) {
    let keys = TempDir::new().unwrap();
    let a = Device::new(&keys, "a");
    let scratch = TempDir::new().unwrap();
    let (method, mut descriptor) =
        method_file_with(scratch.path(), &["crate/src", "README.md"], inputs);
    let bytes = scratch.path().join("elohim-storage");
    std::fs::write(&bytes, artifact_elf(b"Linker: LLD 22.1.2\0")).unwrap();
    recordable_mesh_fixture(a.root(), scratch.path(), &method, &mut descriptor);
    if let Some(policy) = policy {
        std::fs::create_dir_all(a.root().join("genesis/agentic")).unwrap();
        std::fs::write(a.root().join("genesis/agentic/pool-policy.json"), policy).unwrap();
        git(a.root(), &["add", "genesis/agentic/pool-policy.json"]);
        git(a.root(), &["commit", "-qm", "the declared target record"]);
    }
    let evidence = build_messages(
        &scratch.path().join("target"),
        "elohim-storage",
        &bytes,
        messages,
    );
    let (stub, _log) = brit_stub(scratch.path());
    let command = || {
        let mut command = a.command();
        command.envs(descriptor.non_path_inputs.iter());
        command
    };
    let out = command()
        .args(["artifact", "key", "--method"])
        .arg(&method)
        .args(["--print", "input", "--repo"])
        .arg(a.root())
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", text(&out));
    let input = String::from_utf8(out.stdout).unwrap().trim().to_string();
    let mut put = command();
    put.env("BRIT_BUILD_REF_BIN", &stub)
        .args(["artifact", "put", "--method"])
        .arg(&method)
        .args(["--input", &input, "--bytes"])
        .arg(&bytes)
        .args(["--project", "elohim-storage", "--cross", "--json", "--repo"])
        .arg(a.root());
    if let Some(evidence) = evidence {
        put.arg("--build-messages").arg(evidence);
    }
    let out = put.output().unwrap();
    let produces = events(a.root())
        .into_iter()
        .filter(|e| e.action == ReaVerb::Produce)
        .collect();
    (out, produces)
}

fn isa_c_slots(produce: &FlowEvent) -> Vec<&str> {
    produce
        .classified_as
        .iter()
        .filter(|slot| slot.starts_with("env:isa-c"))
        .map(String::as_str)
        .collect()
}

/// Row 29.2, as reworked (ruling 1): a build declared at x86-64 whose recorded C flags are the
/// declared ones and whose every cc compile carries exactly the declared -march is recorded with
/// one `env:isa-c=x86-64`, the marker a sibling draws by (L2b).
#[test]
fn a_declared_target_realized_in_its_c_is_recorded_with_one_isa_c_slot() {
    let sqlite = script(
        "libsqlite3-sys",
        &[cc_compile(
            "-march=x86-64 -mtune=generic -mno-red-zone",
            "sqlite3.c",
        )],
    );
    let built = Messages::Of(vec![ring_at("-march=x86-64"), sqlite]);
    let (out, produces) = declared_put(&DECLARED_X86_64, &built);
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(produces.len(), 1, "{}", text(&out));
    assert_eq!(isa_c_slots(&produces[0]), ["env:isa-c=x86-64"]);
    assert!(
        text(&out).contains("isa-c: ring: 1 compile(s) at -march=x86-64"),
        "{}",
        text(&out)
    );
    // The underscore CXXFLAGS_<triple> spelling of the declared flags is the declared one too; the
    // dashed spelling is refused under a declared target (see the refusal test).
    let mut with_cxx = DECLARED_X86_64.to_vec();
    with_cxx.push(("CXXFLAGS_x86_64_unknown_linux_gnu", "-march=x86-64"));
    let (out, produces) = declared_put(&with_cxx, &built);
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(isa_c_slots(&produces[0]), ["env:isa-c=x86-64"]);
    // Ruling 4: cc's command log, the gate's declared diagnostic switch, recorded and allowed.
    with_cxx.push(("CC_ENABLE_DEBUG_OUTPUT", "1"));
    let (out, produces) = declared_put(&with_cxx, &built);
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(isa_c_slots(&produces[0]), ["env:isa-c=x86-64"]);
}

/// Ruling 1(e, f): bin-launch is marked by the same rule from its own build messages; a build
/// whose only cc-built crates are named exceptions (blake3's CPUID-dispatched assembly, a
/// stacker that compiles nothing) and whose other scripts compile nothing is recorded.
#[test]
fn a_declared_target_whose_build_compiles_only_exception_crates_is_recorded() {
    let blake3 = script(
        "blake3",
        &[cc_compile(
            "-march=x86-64 -mavx512f -mavx512vl",
            "c/blake3_avx512_x86-64_unix.S",
        )],
    );
    let stacker = BuildScript {
        package: "stacker",
        running: Vec::new(),
        objects: &[],
    };
    let serde = BuildScript {
        package: "serde",
        running: Vec::new(),
        objects: &[],
    };
    let (out, produces) = declared_put(
        &DECLARED_X86_64,
        &Messages::Of(vec![blake3, stacker, serde]),
    );
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(isa_c_slots(&produces[0]), ["env:isa-c=x86-64"]);
    assert!(
        text(&out).contains("isa-c: blake3 excepted"),
        "{}",
        text(&out)
    );
}

/// One refused declared-target put: why, the recorded inputs, the build messages, and the
/// reason the refusal must name.
type Refusal<'a> = (&'a str, &'a [(&'a str, &'a str)], Messages, &'a str);

/// Every way the declared C level is not what the build or the record shows refuses with exit 3
/// `unrecorded: <reason>`, and records nothing, so no marker.
#[test]
fn a_declared_target_its_c_does_not_realize_refuses_and_records_no_marker() {
    let v1 = || Messages::Of(vec![ring_at("-march=x86-64")]);
    let mut other_cflags = DECLARED_X86_64.to_vec();
    other_cflags[2] = ("CFLAGS_x86_64_unknown_linux_gnu", "-march=x86-64-v2");
    let mut unrecorded_cflags = DECLARED_X86_64.to_vec();
    unrecorded_cflags.pop();
    let mut extra_family = DECLARED_X86_64.to_vec();
    extra_family.push(("CFLAGS", "-O2"));
    let mut dashed = DECLARED_X86_64.to_vec();
    dashed.push(("CFLAGS_x86_64-unknown-linux-gnu", "-march=x86-64"));
    let mut other_cxx = DECLARED_X86_64.to_vec();
    other_cxx.push(("CXXFLAGS_x86_64_unknown_linux_gnu", "-march=x86-64-v3"));
    let mut isa_mismatch = DECLARED_X86_64.to_vec();
    isa_mismatch[0] = ("target.isa", "x86-64-v2");
    let mut no_isa = DECLARED_X86_64.to_vec();
    no_isa.remove(0);
    let sysroot = BlobCid::compute(b"sysroot").to_string();
    let mut with_sysroot = DECLARED_X86_64.to_vec();
    with_sysroot.push(("target.sysroot", &sysroot));
    let only_isa = [("target.isa", "x86-64")];
    // Ruling 2: the declaration and the record agree on an extra flag; still refused.
    let agreeing_extra = [
        ("target.isa", "x86-64"),
        ("target.cflags", "-march=x86-64 -mavx2"),
        ("CFLAGS_x86_64_unknown_linux_gnu", "-march=x86-64 -mavx2"),
    ];
    let baseline_and_above = script(
        "ring",
        &[
            cc_compile("-march=x86-64", "crypto/curve25519/curve25519.c"),
            cc_compile("-march=x86-64 -mavx2", "crypto/fipsmodule/ec/p256.c"),
        ],
    );
    let silent_sqlite = BuildScript {
        package: "libsqlite3-sys",
        running: vec!["\"ar\" \"cqD\" \"$OUT/libsqlite3.a\" \"$OUT/x.o\"".into()],
        objects: &["x.o", "libsqlite3.a"],
    };
    let cases: [Refusal; 19] = [
        (
            "v3 C",
            &DECLARED_X86_64[..],
            Messages::Of(vec![ring_at("-march=x86-64-v3")]),
            "-march=x86-64-v3",
        ),
        (
            "an extra ISA flag",
            &DECLARED_X86_64[..],
            Messages::Of(vec![ring_at("-march=x86-64 -mavx2")]),
            "-mavx2",
        ),
        (
            "no -march",
            &DECLARED_X86_64[..],
            Messages::Of(vec![ring_at("-O2")]),
            "no -march",
        ),
        (
            "one compile at the baseline and one above it",
            &DECLARED_X86_64[..],
            Messages::Of(vec![baseline_and_above]),
            "ring's compile of crypto/fipsmodule/ec/p256.c carries -mavx2",
        ),
        (
            "objects with no logged compile",
            &DECLARED_X86_64[..],
            Messages::Of(vec![ring_at("-march=x86-64"), silent_sqlite]),
            "no evidence: libsqlite3-sys's build script left",
        ),
        (
            "no build messages",
            &DECLARED_X86_64[..],
            Messages::Absent,
            "needs --build-messages",
        ),
        (
            "messages of other bytes",
            &DECLARED_X86_64[..],
            Messages::OtherBytes(vec![ring_at("-march=x86-64")]),
            "is not --bytes",
        ),
        (
            "recorded flags differ",
            &other_cflags[..],
            v1(),
            "CFLAGS_x86_64_unknown_linux_gnu",
        ),
        (
            "declared flags unrecorded",
            &unrecorded_cflags[..],
            v1(),
            "CFLAGS_x86_64_unknown_linux_gnu",
        ),
        (
            "another C variable recorded",
            &extra_family[..],
            v1(),
            "C-family variable CFLAGS;",
        ),
        (
            "the dashed spelling recorded",
            &dashed[..],
            v1(),
            "CFLAGS_x86_64-unknown-linux-gnu",
        ),
        (
            "CXXFLAGS differ",
            &other_cxx[..],
            v1(),
            "CXXFLAGS_x86_64_unknown_linux_gnu",
        ),
        (
            "cflags level is not the isa",
            &isa_mismatch[..],
            v1(),
            "not target.isa x86-64-v2",
        ),
        (
            "cflags without isa",
            &no_isa[..],
            v1(),
            "declares no target.isa",
        ),
        (
            "isa without cflags",
            &only_isa[..],
            v1(),
            "declares no target.cflags",
        ),
        (
            "a declared sysroot",
            &with_sysroot[..],
            v1(),
            "target.sysroot",
        ),
        (
            "declaration and record agree on an extra flag",
            &agreeing_extra[..],
            v1(),
            "is not exactly -march=x86-64",
        ),
        (
            "an exception crate compiling C",
            &DECLARED_X86_64[..],
            Messages::Of(vec![script(
                "blake3",
                &[cc_compile("-march=x86-64 -mavx512f", "c/blake3_avx512.c")],
            )]),
            "blake3's exception covers its CPUID-dispatched assembly",
        ),
        (
            "a second crate above the baseline",
            &DECLARED_X86_64[..],
            Messages::Of(vec![
                ring_at("-march=x86-64"),
                script("zstd-sys", &[cc_compile("-march=x86-64-v3", "zstd.c")]),
            ]),
            "zstd-sys's compile of zstd.c carries -march=x86-64-v3",
        ),
    ];
    for (why, inputs, messages, reason) in cases {
        let (out, produces) = declared_put(inputs, &messages);
        assert_eq!(out.status.code(), Some(3), "{why}: {}", text(&out));
        let said = String::from_utf8_lossy(&out.stdout);
        assert!(said.contains("unrecorded:"), "{why}: {said}");
        assert!(said.contains(reason), "{why}: {reason} in {said}");
        assert!(produces.is_empty(), "{why}: {produces:?}");
    }
}

/// Ruling 3 (L2a rework): put does not trust the descriptor's declaration alone. Without a
/// pool-policy.json record at HEAD that declares the method's own triple, for the bin's
/// workspace, with the same isa and cflags, the put refuses and records nothing.
#[test]
fn a_declared_target_the_record_at_head_does_not_declare_refuses() {
    let record = |triple: &str, isa: &str, workspace: &str| {
        declared_policy(&format!(
            r#""{triple}": {{"isa": "{isa}", "cflags": "-march={isa}", "workspaces": ["{workspace}"]}}"#
        ))
    };
    for (why, policy, reason) in [
        ("no record at HEAD", None, "does not read at HEAD"),
        (
            "the record's isa differs",
            Some(record("x86_64-unknown-linux-gnu", "x86-64-v2", "crate")),
            "x86-64-v2\\\", and the method declares x86-64",
        ),
        (
            "only a same-architecture neighbour is declared",
            Some(record("x86_64-unknown-linux-musl", "x86-64", "crate")),
            "declares no target \\\"x86_64-unknown-linux-gnu\\\"",
        ),
        (
            "another workspace",
            Some(record("x86_64-unknown-linux-gnu", "x86-64", "elohim/eprfs")),
            "not the bin's workspace crate",
        ),
    ] {
        let messages = Messages::Of(vec![ring_at("-march=x86-64")]);
        let (out, produces) = declared_put_under(&DECLARED_X86_64, &messages, policy.as_deref());
        assert_eq!(out.status.code(), Some(3), "{why}: {}", text(&out));
        let said = String::from_utf8_lossy(&out.stdout);
        assert!(said.contains(reason), "{why}: {reason} in {said}");
        assert!(produces.is_empty(), "{why}: {produces:?}");
    }
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
fn a_produce_stamped_with_a_release_names_it_to_siblings() {
    let (_keys, a, b) = household();
    let a_did = a.key.did_key();
    let key = "bafyreigq2n5xqgbtzbb6ce2ebobfqvqtjqdmmqkn3mmdyx5n6k4hgemrzu";
    let output = BlobCid::compute_bytes(b"the epr binary");
    let manifest = BlobCid::compute(b"\xa0").to_string();
    let release = "bafyreiaww7ivxkoouk2ojpwpqtm4pm7hxlvlzl3u7bqhrgnxcdhbg7mwmu";
    let mut stamped = produce_record(key, &output, &manifest, Some(&a_did));
    stamped.classified_as.extend([
        format!("env:release={release}"),
        "env:release-bin=epr".into(),
    ]);
    let mut store = SidecarFlowStore::open(b.root()).unwrap();
    store.append(FlowRecord::Event(stamped)).unwrap();
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
    assert_eq!(rows[0]["release"], release, "{rows}");
    assert_eq!(rows[0]["releaseBin"], "epr", "{rows}");
}

/// Device B follows epr where its sibling A recorded a landed release for B's HEAD: the offer is
/// found through B's standing and folded custody, and the draw reaches B's own `epr peer serve`.
/// With no daemon up the draw is a typed gap and the follow goes on to its own path.
#[test]
fn a_follow_under_the_knob_draws_a_siblings_release_for_head_before_building() {
    let (_keys, a, b) = household();
    let head = String::from_utf8(
        elohim_epr_cli::process::build_command("git", &["rev-parse", "HEAD"], b.root(), &[])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap()
    .trim()
    .to_string();
    let release = "bafyreiaww7ivxkoouk2ojpwpqtm4pm7hxlvlzl3u7bqhrgnxcdhbg7mwmu";
    let manifest = BlobCid::compute(b"\xa0").to_string();
    let key = "bafyreigq2n5xqgbtzbb6ce2ebobfqvqtjqdmmqkn3mmdyx5n6k4hgemrzu";
    let stamped = |bin: &str, commit: &str| {
        let output = BlobCid::compute_bytes(format!("{commit} {bin}").as_bytes());
        let mut event = produce_record(key, &output, &manifest, Some(&a.key.did_key()));
        event
            .classified_as
            .retain(|slot| !slot.starts_with("env:commit="));
        event.classified_as.extend([
            format!("env:commit={commit}"),
            format!("env:release={release}"),
            format!("env:release-bin={bin}"),
        ]);
        event
    };
    let follow = |b: &Device| {
        let bins = b.config.path().join("bins");
        let run = |args: &[&str]| {
            b.command()
                .args(args)
                .env("BIN_STORE", &bins)
                .env("EPR_FOLLOW_SIBLING", "1")
                .output()
                .unwrap()
        };
        let declared = run(&["device", "follow", "bin:epr", "--mode", "apply"]);
        assert!(declared.status.success(), "{}", text(&declared));
        run(&[
            "bin",
            "--tool",
            "epr",
            "follow",
            "--repo",
            b.root().to_str().unwrap(),
        ])
    };
    let mut store = SidecarFlowStore::open(b.root()).unwrap();
    let nothing_staged = |b: &Device| {
        !b.config
            .path()
            .join("bins/incoming")
            .read_dir()
            .is_ok_and(|mut d| d.next().is_some())
    };

    let stderr_of = |b: &Device| String::from_utf8_lossy(&follow(b).stderr).into_owned();

    // Row 14: B's HEAD is on no published ref here (as with a stale origin/dev). Nothing is
    // read or drawn, the line names each ref where it stands and the fetch cure, and the follow
    // goes on to its own path.
    for bin in ["epr", "bin-launch"] {
        store
            .append(FlowRecord::Event(stamped(bin, "0123456789abcdef")))
            .unwrap();
    }
    let line = stderr_of(&b);
    assert!(
        line.contains("follow: nothing drawn from a sibling (unpublished: "),
        "{line}"
    );
    assert!(line.contains("refs/remotes/origin/dev absent"), "{line}");
    assert!(line.contains("brit fetch -r origin"), "{line}");
    assert!(!line.contains("drawing epr from a sibling first"), "{line}");
    assert!(!line.contains("fetch gap"), "nothing is fetched: {line}");
    assert!(nothing_staged(&b));
    let published = elohim_epr_cli::process::build_command(
        "git",
        &["update-ref", "refs/remotes/origin/dev", &head],
        b.root(),
        &[],
    )
    .output()
    .unwrap();
    assert!(published.status.success(), "{}", text(&published));

    // Published, and only another commit's release is on record: no offer, said once, then the
    // follow's own path.
    let line = stderr_of(&b);
    assert!(
        line.contains("follow: drawing epr from a sibling first: "),
        "{line}"
    );
    assert!(line.contains("no-offer"), "{line}");

    // A's release for B's HEAD, recorded before env:requires-doc existed (as a51dacfc8's puts
    // were: the CID, not the document): row 12 reads it as unmeasured, fetches nothing, and the
    // follow goes on.
    for bin in ["epr", "bin-launch"] {
        let mut event = stamped(bin, &head);
        event
            .classified_as
            .push(format!("env:requires={}", BlobCid::compute_bytes(b"[]")));
        store.append(FlowRecord::Event(event)).unwrap();
    }
    let said = stderr_of(&b);
    assert!(
        said.contains("follow: drawing epr from a sibling first: "),
        "{said}"
    );
    assert!(said.contains("offer predates requires-doc"), "{said}");
    assert!(!said.contains("fetch gap"), "nothing is fetched: {said}");
    assert!(nothing_staged(&b));

    // The same release, re-recorded with a statement this host cannot meet.
    let statement = |glibc: u32| {
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
        use host_contract::{names::axis, AxisId, Clause, ClauseSource, Predicate, Requires};
        let mut requires = Requires::default();
        for (name, predicate) in [
            (axis::ARCH, Predicate::Equal(std::env::consts::ARCH.into())),
            (axis::OS, Predicate::Equal("linux".into())),
            (
                axis::ABI_LIBC,
                Predicate::AtLeast(host_contract::Version {
                    line: "glibc".into(),
                    components: vec![2, glibc],
                }),
            ),
        ] {
            requires
                .insert(Clause {
                    axis: AxisId(name.into()),
                    predicate,
                    source: ClauseSource::Extracted,
                    narrowing: None,
                })
                .unwrap();
        }
        let bytes = serde_json::to_vec(&requires).unwrap();
        [
            format!("env:requires={}", BlobCid::compute_bytes(&bytes)),
            format!("env:requires-doc={}", URL_SAFE_NO_PAD.encode(&bytes)),
            // As a producer that checked its C at the x86_64 baseline records it (row 29.2).
            "env:isa-c=x86-64".to_owned(),
        ]
    };
    let stating = |store: &mut SidecarFlowStore, glibc: u32| {
        for bin in ["epr", "bin-launch"] {
            let mut event = stamped(bin, &head);
            event.classified_as.extend(statement(glibc));
            store.append(FlowRecord::Event(event)).unwrap();
        }
    };
    stating(&mut store, 9999);
    let said = stderr_of(&b);
    assert!(said.contains("requirement-unmet: "), "{said}");
    assert!(
        said.contains("abi.libc"),
        "names the failed requirement: {said}"
    );
    assert!(!said.contains("fetch gap"), "nothing is fetched: {said}");
    assert!(nothing_staged(&b));

    // A statement this host meets (any glibc this test runs on is past 2.0): the draw goes to
    // B's daemon, which is not serving.
    stating(&mut store, 0);
    let said = stderr_of(&b);
    eprintln!("{said}");
    assert!(!said.contains("no-offer"), "{said}");
    assert!(!said.contains("requirement"), "{said}");
    assert!(said.contains("fetch gap: daemon-down"), "{said}");
    assert!(
        !b.config
            .path()
            .join("bins/incoming")
            .read_dir()
            .is_ok_and(|mut d| d.next().is_some()),
        "a draw that installed nothing leaves nothing in incoming"
    );
}

#[test]
fn put_takes_a_release_only_with_the_bin_it_names() {
    let keys = TempDir::new().unwrap();
    let a = Device::new(&keys, "a");
    for args in [
        &["--release", "bafyrei-release"][..],
        &["--release-bin", "epr"][..],
        &["--release", "has space", "--release-bin", "epr"][..],
        &["--release", "bafyrei-release", "--release-bin", "a=b"][..],
    ] {
        let mut argv = vec!["artifact", "put", "--project", "eprfs", "--input", "x"];
        argv.extend_from_slice(args);
        let out = a.epr(&argv);
        assert!(!out.status.success(), "{args:?}: {}", text(&out));
        assert!(text(&out).contains("--release"), "{args:?}: {}", text(&out));
    }
}

/// A workspace restart wipes git's global `safe.directory`, so the brit-build-ref put spawns runs
/// git under only this device's declared checkout trust (`epr device trust`). It reaches brit as
/// `GIT_CONFIG_*` variables, the way the covenant and walk spawns pass it.
#[test]
fn put_hands_brit_the_declared_checkout_trust() {
    let keys = TempDir::new().unwrap();
    let a = Device::new(&keys, "a");
    let scratch = TempDir::new().unwrap();
    let (method, _) = method_file(scratch.path(), &["crate/src"]);
    let root = a.root().canonicalize().unwrap();
    // A brit that, like git on a checkout it does not own, refuses unless trust is declared.
    let stub = scratch.path().join("brit-build-ref");
    std::fs::write(
        &stub,
        format!(
            "#!/bin/sh\n[ \"$1\" = --version ] && exit 0\n\
             if [ \"$GIT_CONFIG_KEY_0\" = safe.directory ] && [ \"$GIT_CONFIG_VALUE_0\" = '{}' ]; \
             then echo '{{\"agentId\":\"{AGENT_ID}\"}}'; \
             else echo 'fatal: detected dubious ownership' >&2; exit 128; fi\n",
            root.display()
        ),
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();
    wait_for_stub(&stub);
    let put = || {
        a.command()
            .env("BRIT_BUILD_REF_BIN", &stub)
            .env_remove("GIT_CONFIG_COUNT")
            .args(["artifact", "put", "--method"])
            .arg(&method)
            .args(["--input", "x", "--project", "elohim-storage", "--repo"])
            .arg(&root)
            .output()
            .unwrap()
    };
    let untrusted = put();
    assert_eq!(untrusted.status.code(), Some(3), "{}", text(&untrusted));
    assert!(
        text(&untrusted).contains("missing attribution: brit signer"),
        "{}",
        text(&untrusted)
    );
    std::fs::write(
        a.key_file.parent().unwrap().join("trusted-checkouts"),
        format!("{}\n", root.display()),
    )
    .unwrap();
    let trusted = put();
    assert!(
        !text(&trusted).contains("missing attribution: brit signer"),
        "{}",
        text(&trusted)
    );
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

/// The C family is a build input (row 29.2: a v3-default gcc put AVX2 into ring's C): every
/// spelling the cc crate reads, set and not recorded with that value, refuses the key and put.
#[test]
fn an_unrecorded_c_family_var_refuses_the_key() {
    let keys = TempDir::new().unwrap();
    let a = Device::new(&keys, "a");
    let scratch = TempDir::new().unwrap();
    let declared = "CFLAGS_x86_64_unknown_linux_gnu";
    let (method, _) = method_file_with(
        scratch.path(),
        &["crate/src"],
        &[(declared, "-march=x86-64")],
    );
    key_of(&a, &method, &[(declared, "-march=x86-64")]);
    for (name, value) in [
        ("CFLAGS", "-march=native"),
        ("CXXFLAGS", "-O3"),
        ("CPPFLAGS", "-DX=1"),
        ("CFLAGS_x86_64-unknown-linux-gnu", "-march=x86-64-v3"),
        ("CXXFLAGS_x86_64_unknown_linux_gnu", "-march=x86-64-v3"),
        ("HOST_CFLAGS", "-mavx2"),
        ("TARGET_CFLAGS", "-mavx2"),
        ("TARGET_CXXFLAGS", "-mavx2"),
        ("CC_x86_64_unknown_linux_gnu", "clang"),
        ("HOST_CC", "clang"),
        ("TARGET_CC", "clang"),
        ("CRATE_CC_NO_DEFAULTS", "1"),
        // Ruling 4 (L2a rework): the archiver, ranlib and cc's own switches are C tools too.
        ("HOST_AR", "llvm-ar"),
        ("TARGET_AR", "llvm-ar"),
        ("AR_x86_64_unknown_linux_gnu", "llvm-ar"),
        ("AR_x86_64-unknown-linux-gnu", "llvm-ar"),
        ("ARFLAGS", "-X"),
        ("TARGET_ARFLAGS", "-X"),
        ("RANLIB", "llvm-ranlib"),
        ("RANLIBFLAGS", "-D"),
        ("CC_FORCE_DISABLE", "1"),
        // Recorded like the rest, though allowed under a declared target once recorded.
        ("CC_ENABLE_DEBUG_OUTPUT", "1"),
    ] {
        let out = key_with(&a, &method, &[(declared, "-march=x86-64"), (name, value)]);
        assert_eq!(out.status.code(), Some(3), "{name}: {}", text(&out));
        assert!(text(&out).contains(name), "{name}: {}", text(&out));
    }
    // The recorded one set otherwise, or not set, refuses too.
    let out = key_with(&a, &method, &[(declared, "-march=x86-64-v3")]);
    assert_eq!(out.status.code(), Some(3), "{}", text(&out));
    let out = key_with(&a, &method, &[]);
    assert_eq!(out.status.code(), Some(3), "{}", text(&out));
    assert!(text(&out).contains(declared), "{}", text(&out));
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

/// A checkout with a JavaScript package (`pkg/package.json`) and a ts-rs crate
/// (`crate/Cargo.toml`), both inside the view `crate` + `pkg`, ignoring `dist/` and `bindings/`
/// the way the workspace's root `.gitignore` does.
fn generated_outputs_checkout(a: &Device) {
    std::fs::write(
        a.root().join(".gitignore"),
        "dist/\nbindings/\nout/\n*.generated\n",
    )
    .unwrap();
    std::fs::write(
        a.root().join("crate/Cargo.toml"),
        "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\n\n[dependencies]\nts-rs = \"10\"\n",
    )
    .unwrap();
    std::fs::create_dir_all(a.root().join("pkg/src")).unwrap();
    std::fs::write(a.root().join("pkg/package.json"), "{\"name\":\"pkg\"}\n").unwrap();
    std::fs::write(a.root().join("pkg/src/index.ts"), "export {};\n").unwrap();
    git(
        a.root(),
        &[
            "add",
            ".gitignore",
            "crate/Cargo.toml",
            "pkg/package.json",
            "pkg/src/index.ts",
        ],
    );
    git(
        a.root(),
        &["commit", "-q", "-m", "a package and a ts-rs crate"],
    );
}

fn write_ignored(a: &Device, path: &str) {
    let path = a.root().join(path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, "generated\n").unwrap();
}

#[test]
fn generated_outputs_beside_their_manifests_key_as_a_clean_checkout() {
    let keys = TempDir::new().unwrap();
    let a = Device::new(&keys, "a");
    let scratch = TempDir::new().unwrap();
    generated_outputs_checkout(&a);
    let (method, _) = method_file(scratch.path(), &["crate", "pkg"]);
    let clean = key_of(&a, &method, &[]);
    // A package's dist beside its package.json and a ts-rs crate's bindings export directory.
    write_ignored(&a, "pkg/dist/index.js");
    write_ignored(&a, "crate/bindings/Thing.ts");
    // An ignored directory outside the view whose path merely shares a string prefix with a view
    // path: git lists `crate-extra/out/` for the pathspec `crate`, and it is no input here.
    std::fs::create_dir_all(a.root().join("crate-extra")).unwrap();
    std::fs::write(a.root().join("crate-extra/tracked.txt"), "t\n").unwrap();
    git(a.root(), &["add", "crate-extra/tracked.txt"]);
    git(
        a.root(),
        &["commit", "-q", "-m", "a tracked sibling directory"],
    );
    write_ignored(&a, "crate-extra/out/y.generated");
    let after = key_with(&a, &method, &[]);
    assert!(after.status.success(), "{}", text(&after));
    assert!(
        text(&after).contains("left out ignored generated outputs")
            && text(&after).contains("pkg/dist"),
        "{}",
        text(&after)
    );
    // The sibling commit moved HEAD but nothing inside the view: the key is the clean one.
    assert_eq!(
        String::from_utf8(after.stdout).unwrap().trim(),
        clean,
        "excluded generated outputs never enter the key"
    );
}

#[test]
fn an_ignored_file_outside_the_generated_shapes_still_refuses() {
    let keys = TempDir::new().unwrap();
    let a = Device::new(&keys, "a");
    let scratch = TempDir::new().unwrap();
    generated_outputs_checkout(&a);
    let (method, _) = method_file(scratch.path(), &["crate", "pkg"]);
    write_ignored(&a, "pkg/dist/index.js");
    // A manifest that mentions ts-rs only in a comment is no ts-rs crate.
    std::fs::write(
        a.root().join("pkg/Cargo.toml"),
        "# no ts-rs here\n[package]\nname = \"pkg\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    git(a.root(), &["add", "pkg/Cargo.toml"]);
    git(
        a.root(),
        &["commit", "-q", "-m", "a manifest naming ts-rs in a comment"],
    );
    for stray in [
        // A generated file beside sources.
        "crate/src/schema.generated",
        // A `dist` with no package.json beside it.
        "crate/src/dist/bundle.js",
        // A `bindings` directory of a crate that depends on no ts-rs.
        "pkg/bindings/x.ts",
    ] {
        write_ignored(&a, stray);
        let out = key_with(&a, &method, &[]);
        assert_eq!(out.status.code(), Some(3), "{stray}: {}", text(&out));
        assert!(
            text(&out).contains("ignored files"),
            "{stray}: {}",
            text(&out)
        );
        std::fs::remove_file(a.root().join(stray)).unwrap();
    }
}

#[test]
fn a_tracked_source_naming_a_generated_output_refuses() {
    let keys = TempDir::new().unwrap();
    let a = Device::new(&keys, "a");
    let scratch = TempDir::new().unwrap();
    generated_outputs_checkout(&a);
    let (method, _) = method_file(scratch.path(), &["crate", "pkg"]);
    write_ignored(&a, "pkg/dist/index.js");
    key_of(&a, &method, &[]);
    // A crate embedding the package's bundle: the build reads the ignored bytes.
    std::fs::write(
        a.root().join("crate/src/embed.rs"),
        "pub const APP: &[u8] = include_bytes!(\"../../pkg/dist/index.js\");\n",
    )
    .unwrap();
    git(a.root(), &["add", "crate/src/embed.rs"]);
    git(a.root(), &["commit", "-q", "-m", "embed the bundle"]);
    let out = key_with(&a, &method, &[]);
    assert_eq!(out.status.code(), Some(3), "{}", text(&out));
    assert!(
        text(&out).contains("pkg/dist") && text(&out).contains("crate/src/embed.rs"),
        "{}",
        text(&out)
    );
    // A build script that only names the leaf joins the path at build time; it refuses too.
    git(a.root(), &["rm", "-q", "crate/src/embed.rs"]);
    std::fs::write(
        a.root().join("crate/build.rs"),
        "fn main() { let _ = std::path::Path::new(\"..\").join(\"pkg\").join(\"dist\"); }\n",
    )
    .unwrap();
    git(a.root(), &["add", "crate/build.rs"]);
    git(
        a.root(),
        &["commit", "-q", "-m", "a build script reading the bundle"],
    );
    let out = key_with(&a, &method, &[]);
    assert_eq!(out.status.code(), Some(3), "{}", text(&out));
    assert!(text(&out).contains("crate/build.rs"), "{}", text(&out));
    // A build script named by `package.build` is build-time code too, whatever its file name.
    git(a.root(), &["rm", "-q", "crate/build.rs"]);
    std::fs::create_dir_all(a.root().join("crate/tools")).unwrap();
    std::fs::write(
        a.root().join("crate/tools/gen.rs"),
        "fn main() { let _ = [\"..\", \"pkg\", \"dist\"].join(\"/\"); }\n",
    )
    .unwrap();
    std::fs::write(
        a.root().join("crate/Cargo.toml"),
        "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\nbuild = \"tools/gen.rs\"\n\n\
         [dependencies]\nts-rs = \"10\"\n",
    )
    .unwrap();
    git(a.root(), &["add", "crate/tools/gen.rs", "crate/Cargo.toml"]);
    git(
        a.root(),
        &["commit", "-q", "-m", "a build script by another name"],
    );
    let out = key_with(&a, &method, &[]);
    assert_eq!(out.status.code(), Some(3), "{}", text(&out));
    assert!(text(&out).contains("crate/tools/gen.rs"), "{}", text(&out));
}

#[test]
fn a_directory_walking_embed_of_a_generated_outputs_package_refuses() {
    for (file, source, names) in [
        // include_dir! of the whole package: no `dist` literal anywhere in the file.
        (
            "crate/src/web.rs",
            "pub static WEB: include_dir::Dir = \
             include_dir::include_dir!(\"$CARGO_MANIFEST_DIR/../pkg\");\n",
            "pkg/dist",
        ),
        // rust-embed's folder attribute resolves a bare `.` against the crate root, whose
        // ignored bindings/ it would embed.
        (
            "crate/src/assets.rs",
            "#[derive(rust_embed::RustEmbed)]\n#[folder = \".\"]\npub struct Assets;\n",
            "crate/bindings",
        ),
    ] {
        let keys = TempDir::new().unwrap();
        let a = Device::new(&keys, "a");
        let scratch = TempDir::new().unwrap();
        generated_outputs_checkout(&a);
        let (method, _) = method_file(scratch.path(), &["crate", "pkg"]);
        write_ignored(&a, "pkg/dist/index.js");
        write_ignored(&a, "crate/bindings/Thing.ts");
        key_of(&a, &method, &[]);
        std::fs::write(a.root().join(file), source).unwrap();
        git(a.root(), &["add", file]);
        git(a.root(), &["commit", "-q", "-m", "embed a directory"]);
        let out = key_with(&a, &method, &[]);
        assert_eq!(out.status.code(), Some(3), "{file}: {}", text(&out));
        assert!(
            text(&out).contains(names) && text(&out).contains(file),
            "{file}: {}",
            text(&out)
        );
    }
}

#[test]
fn a_tracked_symlink_aliasing_a_generated_output_refuses() {
    let keys = TempDir::new().unwrap();
    let a = Device::new(&keys, "a");
    let scratch = TempDir::new().unwrap();
    generated_outputs_checkout(&a);
    let (method, _) = method_file(scratch.path(), &["crate", "pkg"]);
    write_ignored(&a, "pkg/dist/index.js");
    key_of(&a, &method, &[]);
    // Inside the view, so refuse_escaping_symlinks passes it; it reads the ignored bundle.
    std::os::unix::fs::symlink("../../pkg/dist", a.root().join("crate/src/app")).unwrap();
    git(a.root(), &["add", "crate/src/app"]);
    git(a.root(), &["commit", "-q", "-m", "an aliasing link"]);
    let out = key_with(&a, &method, &[]);
    assert_eq!(out.status.code(), Some(3), "{}", text(&out));
    assert!(
        text(&out).contains("symlink crate/src/app") && text(&out).contains("pkg/dist"),
        "{}",
        text(&out)
    );
}

#[test]
fn a_submodule_in_the_view_is_read_for_names_of_a_generated_output() {
    let keys = TempDir::new().unwrap();
    let a = Device::new(&keys, "a");
    let scratch = TempDir::new().unwrap();
    generated_outputs_checkout(&a);
    // An upstream repository whose source embeds the superproject package's bundle.
    let upstream = TempDir::new().unwrap();
    std::fs::create_dir_all(upstream.path().join("src")).unwrap();
    std::fs::write(upstream.path().join("src/lib.rs"), "pub fn f() {}\n").unwrap();
    git(upstream.path(), &["init", "-q"]);
    git(upstream.path(), &["add", "src/lib.rs"]);
    git(upstream.path(), &["commit", "-q", "-m", "upstream"]);
    git(
        a.root(),
        &[
            "-c",
            "protocol.file.allow=always",
            "submodule",
            "add",
            "-q",
            upstream.path().to_str().unwrap(),
            "sub",
        ],
    );
    git(a.root(), &["commit", "-q", "-m", "a submodule"]);
    // The view reaches into the submodule, as a mesh view reaches `elohim/rakia/schemas`.
    let (method, _) = method_file(scratch.path(), &["crate", "pkg", "sub/src"]);
    write_ignored(&a, "pkg/dist/index.js");
    key_of(&a, &method, &[]);
    std::fs::write(
        a.root().join("sub/src/lib.rs"),
        "pub const APP: &str = include_str!(\"../../pkg/dist/index.js\");\n",
    )
    .unwrap();
    git(&a.root().join("sub"), &["commit", "-q", "-am", "embed"]);
    git(a.root(), &["commit", "-q", "-am", "move the submodule"]);
    let out = key_with(&a, &method, &[]);
    assert_eq!(out.status.code(), Some(3), "{}", text(&out));
    assert!(text(&out).contains("sub/src/lib.rs"), "{}", text(&out));
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

#[path = "support/artifact_requires.rs"]
mod requires;

#[path = "support/artifact_sysroot_put.rs"]
mod sysroot_put;

#[path = "support/artifact_reach.rs"]
mod reach;

#[path = "support/artifact_inputs.rs"]
mod inputs;

#[path = "support/artifact_preflight.rs"]
mod preflight;
