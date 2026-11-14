//! K11's standalone boundary: real Git, lock, metadata, compiler, Brit and isolated stores.
#![cfg(feature = "peer")]
use elohim_epr_cli::device_key::{DeviceKey, DEVICE_KEY_ENV};
use elohim_epr_rea::{FlowStore, ReaVerb, SidecarFlowStore};
use eprfs_core::{BlobCid, FetchPolicy, MethodDescriptor};
use std::path::PathBuf;
use std::process::{Command, Output};
use tempfile::TempDir;

#[path = "support/artifact_elf.rs"]
mod fixture;
#[path = "support/toolchain_pin.rs"]
mod toolchain_pin;
static CARGO: std::sync::Mutex<()> = std::sync::Mutex::new(());

const REACH: &str = "reach: pending, not enforced";
/// A bin-store alias (`<store>/bin/<name>` linking to `bin-launch`) resolves its pin from the
/// caller's session and config, which this fixture deliberately isolates, so the launcher refuses
/// under the fixture's environment. Use the store's current real binary
/// (`<store>/<tool>/current/<name>`) instead; any other path is used as given.
fn real_brit(found: PathBuf) -> PathBuf {
    let launcher = std::fs::canonicalize(&found).ok();
    if launcher.as_ref().and_then(|p| p.file_name()) != Some("bin-launch".as_ref()) {
        return found;
    }
    let Some(store) = found
        .read_link()
        .ok()
        .and_then(|alias| alias.parent()?.parent().map(PathBuf::from))
    else {
        return found;
    };
    std::fs::read_dir(&store)
        .into_iter()
        .flatten()
        .flatten()
        .map(|tool| tool.path().join("current").join("brit-build-ref"))
        .find(|candidate| candidate.is_file())
        .unwrap_or(found)
}
/// The command environment is cleared and its PATH holds only the toolchain, so a bare name
/// would not resolve there: find brit-build-ref on the caller's PATH up front.
fn brit_build_ref() -> Option<PathBuf> {
    std::env::var_os("BRIT_BUILD_REF_BIN")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| {
            std::env::var_os("PATH").and_then(|paths| {
                std::env::split_paths(&paths)
                    .map(|dir| dir.join("brit-build-ref"))
                    .find(|candidate| candidate.is_file())
            })
        })
        .map(real_brit)
}
/// A test that records through Brit or reads its signer needs a real `brit-build-ref`. A device
/// has one; CI's nix shell does not, so such a test prints `SKIP:` there. Refusal tests that
/// stop before Brit runs do not call this and still run everywhere.
fn brit_or_skip() -> bool {
    let found = brit_build_ref().is_some_and(|path| path.is_file());
    if !found {
        eprintln!("SKIP: no brit-build-ref file on BRIT_BUILD_REF_BIN or PATH");
    }
    found
}
fn text(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}
fn success(out: Output) -> Output {
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    out
}

struct Foreign {
    dir: TempDir,
    root: PathBuf,
    method: PathBuf,
    binary: PathBuf,
    path: String,
    brit: PathBuf,
    pin: String,
    via_rustup: bool,
}
impl Foreign {
    /// `None` (after a `SKIP:` line) when no real compiler at the pin is available.
    fn try_new() -> Option<Self> {
        let toolchain = toolchain_pin::real_toolchain_or_skip()?;
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("project");
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"hello\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .unwrap();
        std::fs::write(
            root.join("src/main.rs"),
            "fn main() { println!(\"hello\"); }\n",
        )
        .unwrap();
        let cargo = toolchain.cargo;
        // Exercise real rustup proxies offline where rustup exists (a device); installed
        // toolchains live outside our isolated HOME. Without rustup (CI's nix shell) the PATH
        // toolchain is the pin itself, so its directory is the whole PATH.
        let proxy = PathBuf::from("/opt/rust/cargo/bin");
        let path = format!(
            "{}:/usr/bin:/bin",
            if toolchain.via_rustup && proxy.join("rustup").is_file() {
                proxy
            } else {
                cargo.parent().unwrap().to_path_buf()
            }
            .display()
        );
        let brit = brit_build_ref().unwrap_or_else(|| PathBuf::from("brit-build-ref"));
        let fixture = Self {
            pin: toolchain.pin,
            via_rustup: toolchain.via_rustup,
            method: dir.path().join("method.json"),
            binary: dir.path().join("target/debug/hello"),
            root,
            dir,
            path,
            brit,
        };
        fixture.git(&["init", "-q"]);
        success(
            fixture
                .cargo()
                .args(["generate-lockfile", "--offline"])
                .output()
                .unwrap(),
        );
        fixture.commit();
        // A fixed test seed belongs only to this repository, not the developer's Brit identity.
        let signer = fixture.root.join(".git/brit");
        std::fs::create_dir_all(&signer).unwrap();
        std::fs::write(signer.join("agent-key"), [42u8; 32]).unwrap();
        DeviceKey::load_or_generate(&fixture.dir.path().join("device.seed")).unwrap();
        Some(fixture)
    }
    fn command(&self, program: impl AsRef<std::ffi::OsStr>) -> Command {
        let mut c = Command::new(program);
        c.env_clear()
            .current_dir(&self.root)
            .env("PATH", &self.path)
            .env("RUSTUP_TOOLCHAIN", &self.pin)
            .env("RUSTUP_AUTO_INSTALL", "0")
            .env("RUSTFLAGS", "")
            .env("HOME", self.dir.path().join("home"))
            .env("CARGO_HOME", self.dir.path().join("cargo-home"))
            .env("CARGO_TARGET_DIR", self.dir.path().join("target"))
            .env("XDG_CONFIG_HOME", self.dir.path().join("config"))
            .env("XDG_DATA_HOME", self.dir.path().join("data"))
            .env("EPR_PAYLOAD_STORE", self.payload())
            .env("EPR_STORE", self.dir.path().join("store"))
            .env("BRIT_BUILD_REF_BIN", &self.brit)
            .env(DEVICE_KEY_ENV, self.dir.path().join("device.seed"));
        if self.via_rustup {
            c.env("RUSTUP_HOME", "/opt/rust/rustup");
        }
        if let Some(tmp) = std::env::var_os("TMPDIR") {
            c.env("TMPDIR", tmp);
        }
        c
    }
    fn cargo(&self) -> Command {
        self.command("cargo")
    }
    fn git(&self, args: &[&str]) {
        let mut c = self.command("git");
        success(
            c.args([
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.test",
                "-c",
                "core.hooksPath=/dev/null",
            ])
            .args(args)
            .output()
            .unwrap(),
        );
    }
    fn commit(&self) {
        self.git(&["add", "Cargo.toml", "Cargo.lock", "src/main.rs"]);
        self.git(&["commit", "-qm", "standalone fixture"]);
    }
    fn epr(&self) -> Command {
        let mut c = self.command(env!("CARGO_BIN_EXE_epr"));
        c.arg("artifact");
        c
    }
    fn describe(&self) -> Output {
        self.epr()
            .args([
                "describe",
                "--env",
                "RUSTUP_AUTO_INSTALL=0",
                "--project",
                "hello",
                "--bin",
                "hello",
                "--rustflags",
                "",
                "--out",
            ])
            .arg(&self.method)
            .output()
            .unwrap()
    }
    fn descriptor(&self) -> MethodDescriptor {
        serde_json::from_slice(&std::fs::read(&self.method).unwrap()).unwrap()
    }
    fn save(&self, d: &MethodDescriptor) {
        std::fs::write(&self.method, serde_json::to_vec(d).unwrap()).unwrap();
    }
    fn input(&self) -> String {
        let out = success(
            self.epr()
                .args(["key", "--method"])
                .arg(&self.method)
                .args(["--print", "input"])
                .output()
                .unwrap(),
        );
        String::from_utf8(out.stdout).unwrap().trim().into()
    }
    fn put_command(&self, input: &str, repository: bool) -> Command {
        let mut c = self.epr();
        c.args(["put", "--method"])
            .arg(&self.method)
            .args(["--input", input, "--bytes"])
            .arg(&self.binary)
            .args(["--project", "hello", "--json"]);
        if repository {
            c.args(["--repository", "example/hello"]);
        }
        c
    }
    fn put(&self, input: &str, repository: bool) -> Output {
        self.put_command(input, repository).output().unwrap()
    }
    fn payload(&self) -> PathBuf {
        self.dir.path().join("payload")
    }
    fn build(&self) {
        success(
            self.cargo()
                .args(["build", "--locked", "--offline"])
                .output()
                .unwrap(),
        );
    }
    fn absent(&self, descriptor: &MethodDescriptor, input: &str) {
        let notes = success(
            self.command("git")
                .args(["for-each-ref", "refs/notes/brit/build"])
                .output()
                .unwrap(),
        );
        assert!(notes.stdout.is_empty(), "no attestation");
        assert!(
            !self.payload().exists(),
            "preflight did not even open the payload store"
        );
        assert!(!self.root.join(".eprfs/flows").exists());
        let flow = SidecarFlowStore::open(&self.root).unwrap();
        assert!(flow.events().unwrap().is_empty(), "reopened flow store");
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let store = elohim_epr_cli::payload::open_payload_store(
                &self.payload(),
                elohim_epr_cli::payload::Access::Shared,
            )
            .await
            .unwrap()
            .store;
            for cid in [
                descriptor.cid().unwrap(),
                BlobCid::parse(input).unwrap(),
                BlobCid::compute_bytes(&std::fs::read(&self.binary).unwrap()),
            ] {
                assert!(
                    store
                        .fetch_blob(&cid, FetchPolicy::LocalOnly)
                        .await
                        .is_err(),
                    "reopened payload store has {cid}"
                );
            }
        });
    }
}

#[test]
fn artifact_foreign_describe_put_records_complete_local_artifact() {
    let _cargo = CARGO.lock().unwrap_or_else(|e| e.into_inner());
    if !brit_or_skip() {
        return;
    }
    let Some(f) = Foreign::try_new() else { return };
    f.build();
    let out = success(f.describe());
    assert!(text(&out).contains(REACH));
    let d = f.descriptor();
    assert_eq!(d.name, "mesh-build");
    assert_eq!(d.non_path_inputs["PATH"], f.path);
    let paths: Vec<_> = d
        .view
        .iter()
        .map(|p| p.to_slash_string().unwrap())
        .collect();
    assert!(paths.contains(&"src/main.rs".into()) && paths.contains(&"Cargo.lock".into()));
    assert!(!paths.iter().any(String::is_empty));
    let input = f.input();
    let out = success(f.put(&input, true));
    assert!(text(&out).contains(REACH));
    let events = SidecarFlowStore::open(&f.root).unwrap().events().unwrap();
    let produces: Vec<_> = events
        .iter()
        .map(|(_, e)| e)
        .filter(|e| e.action == ReaVerb::Produce)
        .collect();
    assert_eq!(produces.len(), 1);
    let p = produces[0];
    let requires = p
        .classified_as
        .iter()
        .find_map(|s| s.strip_prefix("env:requires="))
        .unwrap();
    assert!(p.classified_as.iter().any(|s| s.starts_with("env:family=")));
    assert!(!p.classified_as.iter().any(|s| s.starts_with("env:reach=")));
    assert!(p
        .classified_as
        .contains(&"env:gaps=build-inputs.compiler-inputs-unattested,reach".to_string()));
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let store = elohim_epr_cli::payload::open_payload_store(
            &f.payload(),
            elohim_epr_cli::payload::Access::Shared,
        )
        .await
        .unwrap()
        .store;
        let input_cid = BlobCid::parse(&input).unwrap();
        let doc = store
            .fetch_blob(&input_cid, FetchPolicy::LocalOnly)
            .await
            .unwrap();
        assert_eq!(BlobCid::compute_bytes(&doc.bytes), input_cid);
        assert!(String::from_utf8_lossy(&doc.bytes).contains("link:"));
        let doc = store
            .fetch_blob(&BlobCid::parse(requires).unwrap(), FetchPolicy::LocalOnly)
            .await
            .unwrap();
        let requires: serde_json::Value = serde_json::from_slice(&doc.bytes).unwrap();
        assert!(requires
            .as_array()
            .unwrap()
            .iter()
            .all(|c| c["source"] == "extracted"));
    });
}

// Each case starts from an independently committed foreign project, then reopens both stores.
fn refusal(
    change: impl FnOnce(&Foreign, &mut MethodDescriptor),
    expected: &[&str],
    repository: bool,
) {
    let _cargo = CARGO.lock().unwrap_or_else(|e| e.into_inner());
    let Some(f) = Foreign::try_new() else { return };
    // Refusal checks need a real readable ELF, but don't compile the same binary seven times.
    std::fs::create_dir_all(f.binary.parent().unwrap()).unwrap();
    let llvm = fixture::RUSTC_IDENTITY
        .lines()
        .find_map(|l| l.strip_prefix("LLVM version: "))
        .unwrap();
    std::fs::write(
        &f.binary,
        fixture::artifact_elf(format!("Linker: LLD {llvm}\0").as_bytes()),
    )
    .unwrap();
    success(f.describe());
    let mut d = f.descriptor();
    let input = f.input();
    change(&f, &mut d);
    f.save(&d);
    let out = f.put(&input, repository);
    assert_eq!(out.status.code(), Some(3), "{}", text(&out));
    for item in expected {
        assert!(text(&out).contains(item), "{item}: {}", text(&out));
    }
    assert!(text(&out).contains(REACH));
    f.absent(&d, &input);
}
#[test]
fn artifact_foreign_missing_lock_records_nothing() {
    refusal(
        |f, _| {
            std::fs::remove_file(f.root.join("Cargo.lock")).unwrap();
        },
        &["missing recipe.source-lock: Cargo.lock"],
        true,
    );
}
#[test]
fn artifact_foreign_missing_commit_records_nothing() {
    refusal(
        |f, _| {
            std::fs::remove_dir_all(f.root.join(".git")).unwrap();
            f.git(&["init", "-q"]);
        },
        &["missing recipe.source-commit"],
        true,
    );
}
#[test]
fn artifact_foreign_uncommitted_build_script_refuses() {
    refusal(
        |f, _| {
            std::fs::write(f.root.join("build.rs"), "fn main() {}\n").unwrap();
        },
        &[
            "compiler/source input not committed in view: build.rs",
            "build-inputs.build-scripts: pending, not enforced",
        ],
        true,
    );
}
#[test]
fn artifact_foreign_unlocked_registry_dependency_refuses() {
    refusal(
        |f, _| {
            std::fs::write(
                f.root.join("Cargo.toml"),
                "[package]\nname=\"hello\"\nversion=\"0.1.0\"\n[dependencies]\nserde=\"1\"\n",
            )
            .unwrap();
        },
        &["missing build-inputs: dependency metadata unverifiable"],
        true,
    );
}
#[test]
fn artifact_foreign_missing_path_records_nothing() {
    refusal(
        |_, d| {
            d.non_path_inputs.remove("PATH");
        },
        &["missing build-inputs: compiler/linker inputs incomplete"],
        true,
    );
}
#[test]
fn artifact_foreign_missing_repository_records_nothing() {
    refusal(
        |_, _| {},
        &["missing attribution: repository identity (.epr-meta/repository.yaml or --repository)"],
        false,
    );
}
#[test]
fn artifact_foreign_noncargo_lists_all_unmet_items_and_records_nothing() {
    refusal(
        |_, d| {
            d.name = "foreign-shell".into();
            d.step = Some("foreign-shell:hello".into());
        },
        &[
            "missing recipe:",
            "missing recipe.view:",
            "missing recipe.source-lock:",
            "missing recipe.source-commit:",
            "missing build-inputs:",
            "missing family:",
            "missing Requires:",
            "missing dispatch-evidence:",
            "missing declared-target:",
            "missing attribution: repository identity",
        ],
        false,
    );
}
#[test]
fn artifact_foreign_missing_signer_and_device_are_aggregated() {
    if !brit_or_skip() {
        return;
    }
    refusal(
        |f, _| {
            std::fs::remove_file(f.root.join(".git/brit/agent-key")).unwrap();
            std::fs::remove_file(f.dir.path().join("device.seed")).unwrap();
        },
        &[
            "missing attribution: brit signer",
            "missing attribution: device key",
        ],
        true,
    );
}

#[test]
fn artifact_foreign_missing_brit_records_nothing() {
    let _cargo = CARGO.lock().unwrap_or_else(|e| e.into_inner());
    let Some(f) = Foreign::try_new() else { return };
    success(f.describe());
    let d = f.descriptor();
    let input = f.input();
    std::fs::create_dir_all(f.binary.parent().unwrap()).unwrap();
    std::fs::write(&f.binary, fixture::artifact_elf(b"Linker: LLD 22.1.2\0")).unwrap();
    let out = f
        .put_command(&input, true)
        .env("BRIT_BUILD_REF_BIN", f.dir.path().join("absent-brit"))
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3), "{}", text(&out));
    assert!(text(&out).contains("missing attribution: brit-build-ref"));
    f.absent(&d, &input);
}

#[test]
fn artifact_foreign_unusable_signer_records_nothing() {
    use std::os::unix::fs::PermissionsExt;
    let _cargo = CARGO.lock().unwrap_or_else(|e| e.into_inner());
    let Some(f) = Foreign::try_new() else { return };
    success(f.describe());
    let d = f.descriptor();
    let input = f.input();
    std::fs::create_dir_all(f.binary.parent().unwrap()).unwrap();
    std::fs::write(&f.binary, fixture::artifact_elf(b"Linker: LLD 22.1.2\0")).unwrap();
    let brit = f.dir.path().join("broken-brit");
    std::fs::write(
        &brit,
        "#!/bin/sh\n[ \"$1\" = --version ] && exit 0\necho invalid-json\n",
    )
    .unwrap();
    std::fs::set_permissions(&brit, std::fs::Permissions::from_mode(0o755)).unwrap();
    fixture::wait_for_stub(&brit);
    let out = f
        .put_command(&input, true)
        .env("BRIT_BUILD_REF_BIN", &brit)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3), "{}", text(&out));
    assert!(text(&out).contains("missing attribution: brit signer"));
    f.absent(&d, &input);
}

#[test]
fn artifact_foreign_proc_macro_records_nothing() {
    refusal(
        |f, _| {
            std::fs::write(f.root.join("src/lib.rs"), "").unwrap();
            std::fs::write(
                f.root.join("Cargo.toml"),
                "[package]\nname=\"hello\"\nversion=\"0.1.0\"\n[lib]\nproc-macro=true\n",
            )
            .unwrap();
        },
        &["missing build-inputs: compiler/source input not committed in view: src/lib.rs"],
        true,
    );
}

#[test]
fn artifact_foreign_all_independent_failures_are_reported_together() {
    refusal(
        |f, d| {
            std::fs::remove_file(f.root.join("Cargo.lock")).unwrap();
            std::fs::write(f.root.join("build.rs"), "fn main() {}\n").unwrap();
            std::fs::write(
                f.root.join("Cargo.toml"),
                "[package]\nname=\"hello\"\nversion=\"0.1.0\"\n[dependencies]\nserde=\"1\"\n",
            )
            .unwrap();
            d.non_path_inputs.remove("PATH");
            d.non_path_inputs
                .insert("target.isa".into(), "x86-64-v2".into());
            std::fs::remove_file(f.dir.path().join("device.seed")).unwrap();
        },
        &[
            "missing recipe.source-lock",
            "missing build-inputs: dependency metadata",
            "missing build-inputs: compiler/linker",
            "missing declared-target",
            "missing attribution: repository identity",
            "missing attribution: device key",
        ],
        false,
    );
}

#[path = "support/artifact_round2.rs"]
mod round2;

#[path = "support/artifact_round3.rs"]
mod round3;
