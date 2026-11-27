//! CLI contract for build-role gating at the final local-build fallback.
#![cfg(feature = "peer")]

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use bytes::Bytes;
use elohim_device_runtime::bin_store::{IngestOptions, Registry, Store};
use elohim_epr_cli::device_key::{self, DeviceKey};
use elohim_epr_rea::{
    atom_cid, AgentRef, FlowStore, ParticipantRow, SidecarFlowStore, SidecarRoster,
};
use eprfs_core::{
    BlobCid, DerivationBasis, DerivationKey, DerivationRecord, KeyInput, MethodDescriptor, Witness,
};
use serde_json::json;
use std::{
    fs,
    io::{BufRead, BufReader},
    os::unix::fs::{symlink, PermissionsExt},
    path::{Path, PathBuf},
    process::{Child, Command, Output, Stdio},
    time::Duration,
};

struct Fixture {
    temp: tempfile::TempDir,
    registry: Registry,
    head: String,
}

impl Fixture {
    fn new(with_old_release: bool) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path();
        fs::create_dir_all(base.join("repo/elohim/eprfs")).unwrap();
        fs::create_dir_all(base.join("config/elohim/device")).unwrap();
        fs::create_dir_all(base.join("cargo-pool")).unwrap();
        fs::create_dir_all(base.join("fake-bin")).unwrap();
        git(base, &["init", "-q"]);
        fs::write(base.join("repo/README.md"), "old\n").unwrap();
        git(base, &["add", "README.md"]);
        git(base, &["commit", "-qm", "old"]);
        let old = git_text(base, &["rev-parse", "HEAD"]);

        let registry: Registry = serde_json::from_value(json!({
            "version": 1,
            "tools": {
                "epr": {
                    "bins": ["epr"],
                    "candidateEnv": {},
                    "source": {
                        "kind": "repository",
                        "publishedRef": ["refs/remotes/origin/dev"]
                    }
                }
            }
        }))
        .unwrap();
        fs::write(
            base.join("registry.json"),
            serde_json::to_vec(&registry).unwrap(),
        )
        .unwrap();

        if with_old_release {
            ingest_and_make_current(base, &registry, &old, "old", "fixture:old");
        }
        fs::write(base.join("repo/README.md"), "new\n").unwrap();
        git(base, &["add", "README.md"]);
        git(base, &["commit", "-qm", "new"]);
        let head = git_text(base, &["rev-parse", "HEAD"]);
        git(base, &["update-ref", "refs/remotes/origin/dev", &head]);

        let cargo = base.join("fake-bin/cargo");
        fs::write(
            &cargo,
            format!(
                "#!/bin/sh\nprintf 'called\\n' >> '{}'\nexit 7\n",
                base.join("cargo.log").display()
            ),
        )
        .unwrap();
        fs::set_permissions(&cargo, fs::Permissions::from_mode(0o755)).unwrap();

        Self {
            temp,
            registry,
            head,
        }
    }

    fn base(&self) -> &Path {
        self.temp.path()
    }

    fn command(&self, args: &[&str]) -> Output {
        let base = self.base();
        Command::new(env!("CARGO_BIN_EXE_epr"))
            .args(args)
            .current_dir(base.join("repo"))
            .env("XDG_CONFIG_HOME", base.join("config"))
            .env("ELOHIM_STATE_DIR", base.join("state"))
            .env("EPR_BIN_TOOLS", base.join("registry.json"))
            .env("BIN_STORE", base.join("store"))
            .env("EPR_STORE", base.join("store/epr"))
            .env("EPR_PAYLOAD_STORE", base.join("payload"))
            .env("CARGO_TARGET_POOL_ROOT", base.join("cargo-pool"))
            .env("CARGO_TARGET_DIR", base.join("target"))
            .env("CARGO_NET_OFFLINE", "true")
            .env(
                "PATH",
                format!("{}:/usr/bin:/bin", base.join("fake-bin").display()),
            )
            .env_remove("ELOHIM_SESSION_ID")
            .env_remove("CLAUDE_CODE_SESSION_ID")
            .env_remove("CLAUDE_SESSION_ID")
            .output()
            .unwrap()
    }

    fn declare_follow(&self) {
        let output = self.command(&["device", "follow", "bin:epr", "--mode", "apply"]);
        assert!(output.status.success(), "{}", text(&output));
    }

    fn follow(&self) -> Output {
        self.command(&[
            "bin",
            "follow",
            "--tool",
            "epr",
            "--repo",
            self.base().join("repo").to_str().unwrap(),
        ])
    }

    fn role(&self, value: serde_json::Value) {
        fs::write(
            self.base().join("config/elohim/device/build-role.json"),
            serde_json::to_vec(&value).unwrap(),
        )
        .unwrap();
    }

    fn cargo_ran(&self) -> bool {
        self.base().join("cargo.log").exists()
    }
}

fn git(base: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args([
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
        ])
        .args(args)
        .current_dir(base.join("repo"))
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", text(&output));
}

fn git_text(base: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(base.join("repo"))
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", text(&output));
    String::from_utf8(output.stdout).unwrap().trim().into()
}

fn ingest(base: &Path, registry: &Registry, commit: &str, label: &str, gate: &str) -> String {
    let source = base.join(label);
    fs::create_dir(&source).unwrap();
    fs::copy(env!("CARGO_BIN_EXE_epr"), source.join("epr")).unwrap();
    let store = Store::new(base.join("store/epr"), "epr", registry).unwrap();
    store
        .ingest(
            &source,
            &IngestOptions {
                source_commit: Some(commit.into()),
                tree_clean: Some(true),
                gate_run: Some(gate.into()),
                payload_formats: Some("blob-store-v1,directory-v1".into()),
                ..Default::default()
            },
        )
        .unwrap()
        .cid
}

fn ingest_and_make_current(
    base: &Path,
    registry: &Registry,
    commit: &str,
    label: &str,
    gate: &str,
) -> String {
    let cid = ingest(base, registry, commit, label, gate);
    symlink(&cid, base.join("store/epr/current")).unwrap();
    fs::write(
        base.join("store/epr/current.json"),
        json!({"cid": cid, "commit": commit}).to_string(),
    )
    .unwrap();
    cid
}

fn text(output: &Output) -> String {
    format!(
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

struct Serving {
    child: Child,
    bound_socket: String,
}

impl Serving {
    fn start(mut command: Command) -> Self {
        let mut child = command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("peer serve starts");
        let stdout = child.stdout.take().expect("serve stdout is piped");
        let (send, receive) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut line = String::new();
            let read = BufReader::new(stdout).read_line(&mut line).map(|_| line);
            let _ = send.send(read);
        });
        let line = receive
            .recv_timeout(Duration::from_secs(10))
            .expect("peer serve announces readiness within 10 seconds")
            .expect("peer serve readiness line is readable");
        if line.is_empty() {
            let output = child.wait_with_output().expect("failed serve can be read");
            panic!("peer serve exited before readiness: {}", text(&output));
        }
        assert!(
            line.starts_with(elohim_epr_cli::peer::SERVING_LINE),
            "peer serve readiness line: {line:?}"
        );
        let bound_socket = line
            .split_once(" on ")
            .and_then(|(_, rest)| rest.split_once(" for ").map(|(sockets, _)| sockets))
            .and_then(|sockets| sockets.split(',').next())
            .expect("SERVING_LINE carries the bound socket")
            .trim()
            .to_string();
        Self {
            child,
            bound_socket,
        }
    }

    fn bound_socket(&self) -> &str {
        &self.bound_socket
    }
}

impl Drop for Serving {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn device_command(
    key_file: &Path,
    config: &Path,
    bin_store: &Path,
    payload: &Path,
    registry: &Path,
) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_epr"));
    command
        .env(device_key::DEVICE_KEY_ENV, key_file)
        .env("XDG_CONFIG_HOME", config)
        .env("ELOHIM_STATE_DIR", config.join("state"))
        .env("EPR_STORE", bin_store.join("epr"))
        .env("EPR_PAYLOAD_STORE", payload)
        .env("EPR_BIN", env!("CARGO_BIN_EXE_epr"))
        .env("EPR_BIN_TOOLS", registry)
        .env("BIN_STORE", bin_store)
        .env_remove("EPR_PEER_RELAY_URL")
        .env_remove("EPR_PEER_RELAYS")
        .env_remove("ELOHIM_SESSION_ID")
        .env_remove("CLAUDE_CODE_SESSION_ID")
        .env_remove("CLAUDE_SESSION_ID");
    command
}

fn peer_id(
    key_file: &Path,
    config: &Path,
    bin_store: &Path,
    payload: &Path,
    registry: &Path,
) -> String {
    let output = device_command(key_file, config, bin_store, payload, registry)
        .args(["peer", "id"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", text(&output));
    String::from_utf8(output.stdout).unwrap().trim().into()
}

fn init_repo(root: &Path, contents: &str) {
    fs::create_dir_all(root.join("elohim/eprfs")).unwrap();
    fs::create_dir_all(root.join(".epr-meta")).unwrap();
    fs::create_dir_all(root.join("genesis/agentic")).unwrap();
    git_at(root, &["init", "-q"]);
    fs::write(root.join("README.md"), contents).unwrap();
    fs::write(root.join(".gitignore"), ".eprfs/\n").unwrap();
    fs::write(
        root.join(".epr-meta/repository.yaml"),
        "version: 1\nagent: repo:ethosengine/elohim\n",
    )
    .unwrap();
    fs::write(
        root.join("genesis/agentic/pool-policy.json"),
        r#"{"berth":{"channel":{"crosses":{"artifact-produce@1":["produce"]}},"artifacts":{"serves":{}}}}"#,
    )
    .unwrap();
    git_at(
        root,
        &[
            "add",
            "README.md",
            ".gitignore",
            ".epr-meta/repository.yaml",
            "genesis/agentic/pool-policy.json",
        ],
    );
    git_at(root, &["commit", "-qm", "fixture"]);
}

fn git_at(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args([
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
        ])
        .args(args)
        .current_dir(root)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", text(&output));
}

fn git_text_at(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", text(&output));
    String::from_utf8(output.stdout).unwrap().trim().into()
}

fn signature(row: &ParticipantRow, key: &DeviceKey) -> String {
    key.sign(&row.signing_message().unwrap())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Which bytes the sibling offers in `sibling_draw`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Offered {
    /// The `epr` and `bin-launch` this test run built.
    Built,
    /// The same, with the offered `epr`'s ELF interpreter rewritten to a path no host has.
    UnmetLoader,
}

/// What the draw owes on this host.
enum Owed {
    Drawn,
    /// Refused before either binary runs, naming one of these loaders, which this host's probe does
    /// not observe. The draw judges the offered binaries in its own order (bin-launch sorts first)
    /// and names the first unmet one, so the test accepts any of them rather than assume an order.
    RefusedForLoader(Vec<String>),
}

/// The byte offset and length (without its NUL) of an ELF64 little-endian file's PT_INTERP path.
fn elf_interpreter(bytes: &[u8]) -> Option<(usize, usize)> {
    const PT_INTERP: u32 = 3;
    if bytes.len() < 64 || &bytes[..4] != b"\x7fELF" || bytes[4] != 2 || bytes[5] != 1 {
        return None;
    }
    let u16_at = |at: usize| u16::from_le_bytes(bytes[at..at + 2].try_into().unwrap()) as usize;
    let u64_at = |at: usize| u64::from_le_bytes(bytes[at..at + 8].try_into().unwrap()) as usize;
    let (phoff, phentsize, phnum) = (u64_at(0x20), u16_at(0x36), u16_at(0x38));
    (0..phnum).find_map(|i| {
        let header = phoff + i * phentsize;
        let kind = u32::from_le_bytes(bytes[header..header + 4].try_into().unwrap());
        (kind == PT_INTERP).then(|| (u64_at(header + 8), u64_at(header + 32).saturating_sub(1)))
    })
}

fn interpreter_of(file: &Path) -> Option<String> {
    let bytes = fs::read(file).unwrap();
    let (at, len) = elf_interpreter(&bytes)?;
    Some(String::from_utf8(bytes[at..at + len].to_vec()).unwrap())
}

/// The loaders this host's own probe observes: the set the draw judges a staged binary against.
fn observed_loaders() -> std::collections::BTreeSet<String> {
    let host = ark_supervisor::host::observe_host();
    match host.observed(&host_contract::AxisId(
        host_contract::names::axis::ABI_LOADER.into(),
    )) {
        Some(host_contract::AxisValue::Names(names)) => names.clone(),
        _ => Default::default(),
    }
}

/// Rewrite the last character of `file`'s ELF interpreter so it names a loader that exists nowhere,
/// keeping its length (the bytes still parse; they are refused before they would ever run).
fn set_absent_loader(file: &Path) -> String {
    let mut bytes = fs::read(file).unwrap();
    let (at, len) = elf_interpreter(&bytes).expect("the built epr is a dynamic ELF");
    bytes[at + len - 1] = if bytes[at + len - 1] == b'9' {
        b'8'
    } else {
        b'9'
    };
    let absent = String::from_utf8(bytes[at..at + len].to_vec()).unwrap();
    assert!(
        !Path::new(&absent).exists(),
        "{absent} must not exist on this host"
    );
    fs::write(file, bytes).unwrap();
    absent
}

/// A declared non-builder with no release for HEAD draws the one its sibling serves, and never
/// runs cargo; whether the draw is admitted is what the drawn bytes require of THIS host.
#[test]
fn declared_non_builder_draws_an_absent_sibling_release_without_cargo() {
    sibling_draw(Offered::Built);
}

/// The opposite case, on every host: bytes whose loader this host lacks are refused before either
/// binary runs, naming the loader, with nothing ingested, the installed head kept and no cargo.
#[test]
fn declared_non_builder_refuses_a_sibling_release_whose_loader_this_host_lacks() {
    sibling_draw(Offered::UnmetLoader);
}

fn sibling_draw(case: Offered) {
    const HANDLE: &str = "matthew";

    let temp = tempfile::tempdir().unwrap();
    let base = temp.path();
    let reader_root = base.join("reader-repo");
    let sibling_root = base.join("sibling-repo");
    init_repo(&reader_root, "old\n");
    init_repo(&sibling_root, "sibling\n");

    let reader_config = base.join("reader-config");
    let sibling_config = base.join("sibling-config");
    let reader_bins = base.join("reader-bins");
    let sibling_bins = base.join("sibling-bins");
    let reader_payload = base.join("reader-payload");
    let sibling_payload = base.join("sibling-payload");
    let reader_key_file = base.join("keys/reader/ed25519.seed");
    let sibling_key_file = base.join("keys/sibling/ed25519.seed");
    let reader_key = DeviceKey::load_or_generate(&reader_key_file).unwrap();
    let sibling_key = DeviceKey::load_or_generate(&sibling_key_file).unwrap();
    let registry: Registry = serde_json::from_value(json!({
        "version": 1,
        "tools": {
            "epr": {
                "bins": ["epr", "bin-launch"],
                "candidateEnv": {},
                "source": {
                    "kind": "repository",
                    "publishedRef": ["refs/remotes/origin/dev"]
                }
            }
        }
    }))
    .unwrap();
    let registry_file = base.join("registry.json");
    fs::write(&registry_file, serde_json::to_vec(&registry).unwrap()).unwrap();

    let reader_node = peer_id(
        &reader_key_file,
        &reader_config,
        &reader_bins,
        &reader_payload,
        &registry_file,
    );
    let sibling_node = peer_id(
        &sibling_key_file,
        &sibling_config,
        &sibling_bins,
        &sibling_payload,
        &registry_file,
    );
    let reader_did = reader_key.did_key();
    let sibling_did = sibling_key.did_key();
    let roster_store = SidecarRoster::open(&reader_root, HANDLE).unwrap();
    let mut genesis = ParticipantRow::Genesis {
        handle: HANDLE.into(),
        chain_root: reader_did.clone(),
        record_cid: atom_cid(&"fixture founding witness").unwrap().to_string(),
        signer: reader_did.clone(),
        signature: String::new(),
    };
    let signed = signature(&genesis, &reader_key);
    if let ParticipantRow::Genesis { signature, .. } = &mut genesis {
        *signature = signed;
    }
    roster_store.append(genesis).unwrap();
    let mut binding = ParticipantRow::Binding {
        handle: HANDLE.into(),
        chain_root: reader_did.clone(),
        controller: sibling_did.clone(),
        authorized_by: reader_did.clone(),
        sig_a: String::new(),
        sig_b: String::new(),
        nonce: "fixture-binding".into(),
    };
    let sig_a = signature(&binding, &reader_key);
    let sig_b = signature(&binding, &sibling_key);
    if let ParticipantRow::Binding {
        sig_a: a, sig_b: b, ..
    } = &mut binding
    {
        *a = sig_a;
        *b = sig_b;
    }
    roster_store.append(binding).unwrap();
    for (device, node_id, key, nonce) in [
        (&reader_did, &reader_node, &reader_key, "reader-endpoint"),
        (
            &sibling_did,
            &sibling_node,
            &sibling_key,
            "sibling-endpoint",
        ),
    ] {
        let mut endpoint = ParticipantRow::PeerEndpoint {
            handle: HANDLE.into(),
            chain_root: reader_did.clone(),
            device: device.clone(),
            node_id: node_id.clone(),
            seq: 1,
            nonce: nonce.into(),
            signature: String::new(),
        };
        let signed = signature(&endpoint, key);
        if let ParticipantRow::PeerEndpoint { signature, .. } = &mut endpoint {
            *signature = signed;
        }
        roster_store.append(endpoint).unwrap();
    }
    let roster = format!(".eprfs/status/participants/{HANDLE}.jsonl");
    fs::create_dir_all(sibling_root.join(".eprfs/status/participants")).unwrap();
    fs::copy(reader_root.join(&roster), sibling_root.join(&roster)).unwrap();

    let store = Store::new(reader_bins.join("epr"), "epr", &registry).unwrap();
    let old = git_text_at(&reader_root, &["rev-parse", "HEAD"]);
    let old_release = base.join("old-release");
    fs::create_dir(&old_release).unwrap();
    fs::copy(env!("CARGO_BIN_EXE_epr"), old_release.join("epr")).unwrap();
    fs::copy("/bin/true", old_release.join("bin-launch")).unwrap();
    let old_cid = store
        .ingest(
            &old_release.join("epr"),
            &IngestOptions {
                source_commit: Some(old),
                tree_clean: Some(true),
                gate_run: Some("fixture:old".into()),
                payload_formats: Some("blob-store-v1,directory-v1".into()),
                ..Default::default()
            },
        )
        .unwrap()
        .cid;
    symlink(&old_cid, store.root().join("current")).unwrap();
    fs::write(
        store.root().join("current.json"),
        json!({"cid": old_cid, "commit": git_text_at(&reader_root, &["rev-parse", "HEAD"])})
            .to_string(),
    )
    .unwrap();
    fs::write(reader_root.join("README.md"), "new\n").unwrap();
    git_at(&reader_root, &["add", "README.md"]);
    git_at(&reader_root, &["commit", "-qm", "new"]);
    let head = git_text_at(&reader_root, &["rev-parse", "HEAD"]);
    git_at(
        &reader_root,
        &["update-ref", "refs/remotes/origin/dev", &head],
    );

    let offered = base.join("offered-release");
    fs::create_dir(&offered).unwrap();
    let epr = PathBuf::from(env!("CARGO_BIN_EXE_epr"));
    let bin_launch = epr.with_file_name("bin-launch");
    assert!(bin_launch.is_file(), "build bin-launch before this test");
    fs::copy(&epr, offered.join("epr")).unwrap();
    fs::copy(&bin_launch, offered.join("bin-launch")).unwrap();
    let loaders = observed_loaders();
    let owed = match case {
        // Both offered binaries get an absent loader: whichever the draw judges first is unmet.
        Offered::UnmetLoader => Owed::RefusedForLoader(
            ["epr", "bin-launch"]
                .into_iter()
                .map(|bin| set_absent_loader(&offered.join(bin)))
                .collect(),
        ),
        Offered::Built => {
            // The draw re-extracts what the bytes require and judges it on this host, so the
            // outcome follows how this run's binaries were linked: a build against this host's
            // standard loader is drawn; a build whose loader the probe does not observe (CI's
            // nix-shell build names a /nix/store loader, which the probe's fixed list never
            // holds) is owed the refusal, and the test says which it exercised.
            let unobserved: Vec<String> = ["epr", "bin-launch"]
                .into_iter()
                .filter_map(|bin| interpreter_of(&offered.join(bin)))
                .filter(|loader| !loaders.contains(loader))
                .collect();
            if unobserved.is_empty() {
                Owed::Drawn
            } else {
                eprintln!(
                    "this host's probe does not observe {unobserved:?}, the loader(s) this run's \
                     binaries were linked against (observed: {loaders:?}); asserting the refusal \
                     the draw owes here instead of the draw"
                );
                Owed::RefusedForLoader(unobserved)
            }
        }
    };
    let release = store.release_cid_of(&offered).unwrap();
    assert!(
        !store.root().join(&release).exists(),
        "the sibling release is absent before follow"
    );
    assert!(
        !store.holds_verified_for(&head),
        "no verified release for HEAD is held before follow"
    );

    let policy = reader_root.join("genesis/agentic/pool-policy.json");
    let requires = serde_json::to_vec(&json!([
        {"axis":"arch","predicate":{"kind":"equal","value":std::env::consts::ARCH},"source":"extracted"},
        {"axis":"os","predicate":{"kind":"equal","value":"linux"},"source":"extracted"}
    ]))
    .unwrap();
    let requires_cid = BlobCid::compute_bytes(&requires);
    let descriptor = MethodDescriptor {
        name: "release-bin".into(),
        tools: Default::default(),
        flags: Vec::new(),
        params: None,
        exec: None,
        step: None,
        embedder: None,
        view: Vec::new(),
        non_path_inputs: Default::default(),
        deterministic: true,
    };
    let method = descriptor.cid().unwrap();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        let payload = elohim_epr_cli::payload::open_payload_store(
            &sibling_payload,
            elohim_epr_cli::payload::Access::Shared,
        )
        .await
        .unwrap()
        .store;
        eprfs_peer::put_method_record(payload.as_ref(), &descriptor)
            .await
            .unwrap();
        let mut flow = SidecarFlowStore::open(&sibling_root).unwrap();
        for bin in ["epr", "bin-launch"] {
            let bytes = fs::read(offered.join(bin)).unwrap();
            let input = BlobCid::compute_raw(bin.as_bytes());
            let key = DerivationKey {
                method: method.clone(),
                input: KeyInput::Raw(input),
            };
            let output = BlobCid::compute_bytes(&bytes);
            let record = DerivationRecord {
                key: key.clone(),
                output: output.clone(),
                computed_by: sibling_key.did_key(),
                basis: DerivationBasis::Computed,
                deterministic: true,
                witness: Witness::Signature {
                    sig: "fixture-signature".into(),
                },
            };
            let mut extra = vec![
                "env:reach=self".into(),
                "basis-key:git-tree".into(),
                format!("env:key={}", eprfs_peer::key_cid(&key).unwrap()),
                format!("env:method={method}"),
                format!("env:output={output}"),
                format!("env:bytes={}", bytes.len()),
                format!("env:commit={head}"),
                format!("env:release={release}"),
                format!("env:release-bin={bin}"),
                format!("env:requires={requires_cid}"),
                format!("env:requires-doc={}", URL_SAFE_NO_PAD.encode(&requires)),
                "env:isa-c=x86-64".into(),
            ];
            if bin == "epr" {
                extra.push("env:payload-formats=blob-store-v1,directory-v1".into());
            }
            eprfs_peer::put_local_blob_with(
                payload.as_ref(),
                &mut flow,
                record,
                Bytes::from(bytes),
                &eprfs_peer::LocalBlobContext {
                    custodian: AgentRef(sibling_key.did_key()),
                    in_scope_of: atom_cid(&"repo-scope".to_string()).unwrap(),
                    occurred_at: "2026-11-26T10:00:00Z".into(),
                    crossing: Some(("artifact-produce@1".into(), "produce".into())),
                },
                extra,
            )
            .await
            .unwrap();
        }
    });

    let mut sibling_events: Vec<eprfs_peer::ChannelEvent> = SidecarFlowStore::open(&sibling_root)
        .unwrap()
        .events()
        .unwrap()
        .into_iter()
        .map(|(_, event)| eprfs_peer::ChannelEvent {
            cid: atom_cid(&event).unwrap(),
            event,
        })
        .collect();
    sibling_events.sort_by_key(|row| row.cid.to_string());
    let folded = eprfs_peer::fold_events(
        &SidecarFlowStore::open(&reader_root).unwrap(),
        &sibling_key.did_key(),
        &sibling_events,
        &eprfs_peer::CrossingAllowList::new([("artifact-produce@1", ["produce"])]),
    )
    .unwrap();
    assert_eq!(folded.appended, 2);

    let mut sibling_serve = device_command(
        &sibling_key_file,
        &sibling_config,
        &sibling_bins,
        &sibling_payload,
        &registry_file,
    );
    sibling_serve.args([
        "peer",
        "serve",
        "--root",
        sibling_root.to_str().unwrap(),
        "--policy",
        policy.to_str().unwrap(),
        "--listen",
        "127.0.0.1:0",
    ]);
    let sibling_serve = Serving::start(sibling_serve);
    let mut reader_serve = device_command(
        &reader_key_file,
        &reader_config,
        &reader_bins,
        &reader_payload,
        &registry_file,
    );
    reader_serve.args([
        "peer",
        "serve",
        "--root",
        reader_root.to_str().unwrap(),
        "--policy",
        policy.to_str().unwrap(),
        "--listen",
        "127.0.0.1:0",
        "--peer",
        &format!("{sibling_node}@{}", sibling_serve.bound_socket()),
    ]);
    let _reader_serve = Serving::start(reader_serve);

    let reader_device = reader_key_file.parent().unwrap();
    fs::create_dir_all(reader_device).unwrap();
    fs::write(
        reader_device.join("build-role.json"),
        r#"{"version":1,"role":"carrier"}"#,
    )
    .unwrap();
    let fake_bin = base.join("fake-bin");
    fs::create_dir(&fake_bin).unwrap();
    let cargo = fake_bin.join("cargo");
    fs::write(
        &cargo,
        format!(
            "#!/bin/sh\nprintf 'called\\n' >> '{}'\nexit 7\n",
            base.join("cargo.log").display()
        ),
    )
    .unwrap();
    fs::set_permissions(&cargo, fs::Permissions::from_mode(0o755)).unwrap();
    let reader = || {
        let mut command = device_command(
            &reader_key_file,
            &reader_config,
            &reader_bins,
            &reader_payload,
            &registry_file,
        );
        command
            .current_dir(&reader_root)
            .env("CARGO", &cargo)
            .env("CARGO_TARGET_DIR", base.join("target"))
            .env("CARGO_NET_OFFLINE", "true")
            .env("PATH", format!("{}:/usr/bin:/bin", fake_bin.display()));
        command
    };
    let declared = reader()
        .args(["device", "follow", "bin:epr", "--mode", "apply"])
        .output()
        .unwrap();
    assert!(declared.status.success(), "{}", text(&declared));
    let output = reader()
        .args([
            "bin",
            "follow",
            "--tool",
            "epr",
            "--repo",
            reader_root.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    let installed = || {
        serde_json::from_slice::<serde_json::Value>(
            &fs::read(store.root().join("current.json")).unwrap(),
        )
        .unwrap()["cid"]
            .clone()
    };
    match owed {
        Owed::Drawn => {
            assert!(output.status.success(), "{}", text(&output));
            let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(
                result["sibling"]["device"],
                sibling_key.did_key(),
                "{}",
                text(&output)
            );
            assert_eq!(result["sibling"]["release"], release, "{}", text(&output));
            assert_eq!(installed(), release, "{}", text(&output));
            assert!(store.holds_verified_for(&head));
        }
        Owed::RefusedForLoader(loaders) => {
            assert!(!output.status.success(), "{}", text(&output));
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(stderr.contains("abi.loader unmet"), "{}", text(&output));
            assert!(
                loaders
                    .iter()
                    .any(|loader| stderr.contains(loader.as_str())),
                "names one of {loaders:?}: {}",
                text(&output)
            );
            assert!(stderr.contains("nothing was ingested"), "{}", text(&output));
            assert_eq!(
                installed(),
                old_cid,
                "the installed head is kept: {}",
                text(&output)
            );
            assert!(!store.holds_verified_for(&head));
            assert!(
                !store.root().join(&release).exists(),
                "the refused release is never stored"
            );
        }
    }
    assert!(
        !base.join("cargo.log").exists(),
        "a declared non-builder never invokes cargo"
    );
}

#[test]
fn declared_non_builder_with_a_usable_sibling_release_follows_without_cargo() {
    let fixture = Fixture::new(true);
    let candidate = ingest(
        fixture.base(),
        &fixture.registry,
        &fixture.head,
        "sibling-candidate",
        "sibling:did:key:z6MkBuilder",
    );
    fixture.role(json!({"version": 1, "role": "carrier"}));
    fixture.declare_follow();

    let output = fixture.follow();

    assert!(output.status.success(), "{}", text(&output));
    assert!(String::from_utf8_lossy(&output.stdout).contains(&candidate));
    assert!(!fixture.cargo_ran(), "a held sibling release is used first");
}

#[test]
fn missing_build_role_builds_with_one_warning() {
    let fixture = Fixture::new(true);
    fixture.declare_follow();

    let output = fixture.follow();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(output.status.code(), Some(2), "{}", text(&output));
    assert_eq!(stderr.matches("warning: build-role").count(), 1, "{stderr}");
    assert!(
        stderr.contains("build-role missing; building as before"),
        "{stderr}"
    );
    assert!(
        fixture.cargo_ran(),
        "unknown role preserves the build fallback"
    );
}

#[test]
fn expired_builder_builds_with_one_warning() {
    let fixture = Fixture::new(true);
    fixture.role(json!({
        "version": 1,
        "role": "builder",
        "until": "2000-01-01T00:00:00Z"
    }));
    fixture.declare_follow();

    let output = fixture.follow();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(output.status.code(), Some(2), "{}", text(&output));
    assert_eq!(stderr.matches("warning: build-role").count(), 1, "{stderr}");
    assert!(stderr.contains("builder expired 2000-01-01T00:00:00Z"));
    assert!(
        fixture.cargo_ran(),
        "expired builder preserves the build fallback"
    );
}

#[test]
fn declared_non_builder_without_a_release_refuses_with_attempts_cure_and_exit_three() {
    let fixture = Fixture::new(true);
    fixture.role(json!({"version": 1, "role": "neither"}));
    fixture.declare_follow();

    let output = fixture.follow();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(output.status.code(), Some(3), "{}", text(&output));
    for expected in [
        "verified current unavailable",
        "no verified stored candidate",
        "no local closure supplied",
        "sibling attempts:",
        "epr device build-role builder --until <RFC3339>",
        "epr bin follow --tool epr",
    ] {
        assert!(stderr.contains(expected), "missing {expected:?}: {stderr}");
    }
    assert!(
        !fixture.cargo_ran(),
        "a declared non-builder never invokes cargo"
    );
}

#[test]
fn bootstrap_refuses_for_a_declared_non_builder_without_calling_cargo() {
    let fixture = Fixture::new(false);
    fixture.role(json!({"version": 1, "role": "neither"}));

    let output = fixture.command(&[
        "bin",
        "bootstrap",
        "--tool",
        "epr",
        "--repo",
        fixture.base().join("repo").to_str().unwrap(),
    ]);

    assert_eq!(output.status.code(), Some(3), "{}", text(&output));
    assert!(String::from_utf8_lossy(&output.stderr).contains("REFUSED not-a-builder"));
    assert!(!fixture.cargo_ran(), "bootstrap must honor the role gate");
}
