//! Uniform admission, actual compiler inputs, recording order and the proxy path.
use super::*;

const REGISTRY: &str = "/opt/rust/cargo/registry";
const REGISTRY_CRATE: &str = "cache/index.crates.io-1949cf8c6b5b557f/cfg-if-1.0.4.crate";

/// These tests copy one installed crate from the device's Cargo registry; without it they `SKIP:`.
fn registry_or_skip() -> bool {
    let found = PathBuf::from(REGISTRY).join(REGISTRY_CRATE).is_file();
    if !found {
        eprintln!("SKIP: {REGISTRY}/{REGISTRY_CRATE} is not installed");
    }
    found
}

fn copy_registry(f: &Foreign) {
    // Copy just one already-installed registry package and its sparse-index record. No shared
    // Cargo-home writes, network, seed identities or dependency on the Elohim manifest.
    let registry = "index.crates.io-1949cf8c6b5b557f";
    for rel in [
        format!("index/{registry}/config.json"),
        format!("index/{registry}/.cache/cf/g-/cfg-if"),
        format!("cache/{registry}/cfg-if-1.0.4.crate"),
    ] {
        let dest = f.dir.path().join("cargo-home/registry").join(&rel);
        std::fs::create_dir_all(dest.parent().unwrap()).unwrap();
        std::fs::copy(PathBuf::from(REGISTRY).join(rel), dest).unwrap();
    }
}

fn add_dependencies(f: &Foreign) {
    copy_registry(f);
    std::fs::create_dir_all(f.root.join("mac/src")).unwrap();
    std::fs::write(
        f.root.join("mac/Cargo.toml"),
        "[package]\nname='mac'\nversion='0.1.0'\n[lib]\nproc-macro=true\n",
    )
    .unwrap();
    std::fs::write(f.root.join("mac/src/lib.rs"), "extern crate proc_macro;\n").unwrap();
    std::fs::write(f.root.join("build.rs"), "fn main() {}\n").unwrap();
    std::fs::write(f.root.join("Cargo.toml"), "[package]\nname='hello'\nversion='0.1.0'\nedition='2021'\n[dependencies]\ncfg-if='=1.0.4'\nmac={path='mac'}\n").unwrap();
    success(
        f.cargo()
            .args(["generate-lockfile", "--offline"])
            .output()
            .unwrap(),
    );
    f.git(&["add", "mac", "build.rs"]);
    f.commit();
}

fn gaps(f: &Foreign) -> String {
    let events = SidecarFlowStore::open(&f.root).unwrap().events().unwrap();
    let p: Vec<_> = events
        .iter()
        .filter(|(_, e)| e.action == ReaVerb::Produce)
        .collect();
    assert_eq!(p.len(), 1);
    let slots: Vec<_> = p[0]
        .1
        .classified_as
        .iter()
        .filter(|s| s.starts_with("env:gaps="))
        .collect();
    assert_eq!(slots.len(), 1);
    slots[0].to_string()
}

#[test]
fn artifact_foreign_dependencies_record_sorted_declared_gaps_through_real_proxies() {
    let _cargo = CARGO.lock().unwrap_or_else(|e| e.into_inner());
    if !brit_or_skip() || !registry_or_skip() {
        return;
    }
    let Some(f) = Foreign::try_new() else { return };
    add_dependencies(&f);
    f.build();
    let out = success(f.describe());
    for gap in [
        "build-inputs.build-scripts",
        "build-inputs.dependency-sources",
        "build-inputs.compiler-inputs-unattested",
        "reach",
    ] {
        assert!(text(&out).contains(&format!("{gap}: pending, not enforced")));
    }
    let d = f.descriptor();
    assert_eq!(d.tools["toolchain"], f.pin);
    assert!(!d.non_path_inputs.contains_key("RUSTUP_TOOLCHAIN"));
    if f.via_rustup && std::path::Path::new("/opt/rust/cargo/bin/rustup").exists() {
        assert!(d.non_path_inputs["PATH"].starts_with("/opt/rust/cargo/bin:"));
    }
    let input = f.input();
    let out = success(f.put(&input, true));
    assert!(text(&out).contains("build-inputs.build-scripts: pending, not enforced"));
    assert_eq!(
        gaps(&f),
        "env:gaps=build-inputs.build-scripts,build-inputs.compiler-inputs-unattested,build-inputs.dependency-sources,reach"
    );
}

#[test]
fn artifact_mesh_named_foreign_dependencies_use_the_same_rule() {
    let _cargo = CARGO.lock().unwrap_or_else(|e| e.into_inner());
    if !brit_or_skip() || !registry_or_skip() {
        return;
    }
    let Some(f) = Foreign::try_new() else { return };
    add_dependencies(&f);
    // Reproduce every old recognition fact without being the Elohim repository.
    let crate_dir = "elohim/elohim-storage";
    std::fs::create_dir_all(f.root.join(crate_dir)).unwrap();
    for name in ["Cargo.toml", "Cargo.lock", "src", "mac", "build.rs"] {
        f.git(&["mv", name, &format!("{crate_dir}/{name}")]);
    }
    std::fs::create_dir_all(f.root.join(".epr-meta")).unwrap();
    std::fs::write(
        f.root.join(".epr-meta/repository.yaml"),
        "version: 1\nagent: repo:ethosengine/elohim\n",
    )
    .unwrap();
    std::fs::create_dir_all(f.root.join("elohim/holochain")).unwrap();
    std::fs::write(
        f.root.join("elohim/holochain/build-manifest.json"),
        r#"{"steps":{"cargo-build-storage":{"inputs":{"sources":["elohim/elohim-storage/**"]}}}}"#,
    )
    .unwrap();
    f.git(&[
        "add",
        ".epr-meta/repository.yaml",
        "elohim/holochain/build-manifest.json",
    ]);
    f.git(&["commit", "-qm", "copied recognition facts"]);
    success(
        f.cargo()
            .current_dir(f.root.join(crate_dir))
            .args(["build", "--locked", "--offline"])
            .output()
            .unwrap(),
    );
    success(
        f.epr()
            .args([
                "describe",
                "--env",
                "RUSTUP_AUTO_INSTALL=0",
                "--project",
                "elohim-storage",
                "--bin",
                "hello",
                "--crate",
                crate_dir,
                "--rustflags",
                "",
                "--out",
            ])
            .arg(&f.method)
            .output()
            .unwrap(),
    );
    let input = f.input();
    success(
        f.put_command(&input, false)
            .args(["--project", "elohim-storage"])
            .output()
            .unwrap(),
    );
    assert_eq!(
        gaps(&f),
        "env:gaps=build-inputs.build-scripts,build-inputs.compiler-inputs-unattested,build-inputs.dependency-sources,reach"
    );
}

#[test]
fn artifact_untracked_entrypoint_refuses_describe_and_put() {
    let _cargo = CARGO.lock().unwrap_or_else(|e| e.into_inner());
    let Some(f) = Foreign::try_new() else { return };
    success(f.describe());
    let d = f.descriptor();
    let input = f.input();
    f.git(&["rm", "--cached", "src/main.rs"]);
    f.git(&["commit", "-qm", "untrack entrypoint"]);
    f.build();
    for out in [f.describe(), f.put(&input, true)] {
        assert_eq!(out.status.code(), Some(3), "{}", text(&out));
        assert!(
            text(&out).contains("compiler/source input not committed in view: src/main.rs"),
            "{}",
            text(&out)
        );
    }
    f.absent(&d, &input);
}

#[test]
fn artifact_ignored_include_is_found_in_cargo_dep_info() {
    let _cargo = CARGO.lock().unwrap_or_else(|e| e.into_inner());
    let Some(f) = Foreign::try_new() else { return };
    std::fs::write(f.root.join("secret.txt"), "not in captured view").unwrap();
    std::fs::write(f.root.join(".gitignore"), "/secret.txt\n").unwrap();
    std::fs::write(
        f.root.join("src/main.rs"),
        "fn main() { println!(\"{}\", include_str!(\"../secret.txt\")); }\n",
    )
    .unwrap();
    f.git(&["add", ".gitignore"]);
    f.commit();
    f.build();
    success(f.describe());
    let input = f.input();
    let out = f.put(&input, true);
    assert_eq!(out.status.code(), Some(3), "{}", text(&out));
    assert!(text(&out).contains("compiler/source input not committed in view: secret.txt"));
    f.absent(&f.descriptor(), &input);
}

#[test]
fn artifact_missing_dep_info_refuses() {
    let _cargo = CARGO.lock().unwrap_or_else(|e| e.into_inner());
    let Some(f) = Foreign::try_new() else { return };
    f.build();
    success(f.describe());
    let input = f.input();
    std::fs::remove_file(f.binary.with_extension("d")).unwrap();
    let out = f.put(&input, true);
    assert_eq!(out.status.code(), Some(3), "{}", text(&out));
    assert!(text(&out).contains("missing build-inputs: compiler inputs unverifiable (no dep-info)"));
    f.absent(&f.descriptor(), &input);
}

#[test]
fn artifact_required_library_placeholder_refuses() {
    use std::os::unix::fs::PermissionsExt;
    let _cargo = CARGO.lock().unwrap_or_else(|e| e.into_inner());
    let Some(mut f) = Foreign::try_new() else {
        return;
    };
    f.build();
    let tools = f.dir.path().join("tools");
    std::fs::create_dir(&tools).unwrap();
    let cc = tools.join("cc");
    std::fs::write(&cc, "#!/bin/sh\ncase \"$1\" in\n-print-file-name=libc.so|-print-file-name=libc.a) echo \"${1#*=}\" ;;\n*) exec /usr/bin/cc \"$@\" ;;\nesac\n").unwrap();
    std::fs::set_permissions(&cc, std::fs::Permissions::from_mode(0o755)).unwrap();
    fixture::wait_for_stub(&cc);
    f.path = format!("{}:{}", tools.display(), f.path);
    success(f.describe());
    let input = f.input();
    let out = f.put(&input, true);
    assert_eq!(out.status.code(), Some(3), "{}", text(&out));
    assert!(
        text(&out).contains("required link input unresolved: libc"),
        "{}",
        text(&out)
    );
    f.absent(&f.descriptor(), &input);
}

#[test]
fn artifact_destination_failures_leave_no_attestation_or_produce() {
    let _cargo = CARGO.lock().unwrap_or_else(|e| e.into_inner());
    for failure in ["payload-file", "payload-format", "flow"] {
        let Some(f) = Foreign::try_new() else { return };
        f.build();
        success(f.describe());
        let input = f.input();
        let log = f.root.join(".eprfs/status/flows.jsonl");
        match failure {
            "payload-file" => std::fs::write(f.payload(), "invalid layout").unwrap(),
            "payload-format" => {
                std::fs::create_dir_all(f.payload()).unwrap();
                std::fs::write(
                    f.payload().join(elohim_epr_cli::payload::MARKER_FILE),
                    "unsupported-v99",
                )
                .unwrap();
            }
            _ => {
                std::fs::create_dir_all(log.parent().unwrap()).unwrap();
                std::fs::write(&log, "broken flow log\n").unwrap();
            }
        }
        let out = f.put(&input, true);
        assert_eq!(out.status.code(), Some(3), "{failure}: {}", text(&out));
        // Preserve and check the rejected bytes, then repair only this fixture's invalid
        // layout so both stores can be reopened and inspected for leaked records/documents.
        match failure {
            "payload-file" => {
                assert_eq!(
                    std::fs::read_to_string(f.payload()).unwrap(),
                    "invalid layout"
                );
                std::fs::remove_file(f.payload()).unwrap();
            }
            "payload-format" => {
                assert_eq!(
                    std::fs::read_to_string(f.payload().join(elohim_epr_cli::payload::MARKER_FILE))
                        .unwrap(),
                    "unsupported-v99"
                );
                std::fs::remove_dir_all(f.payload()).unwrap();
            }
            _ => {
                assert_eq!(std::fs::read_to_string(&log).unwrap(), "broken flow log\n");
                std::fs::remove_file(&log).unwrap();
            }
        }
        f.absent(&f.descriptor(), &input);
    }
}

#[test]
fn artifact_pending_banner_precedes_option_errors() {
    let _cargo = CARGO.lock().unwrap_or_else(|e| e.into_inner());
    let Some(f) = Foreign::try_new() else { return };
    for verb in ["describe", "put"] {
        let out = f.epr().args([verb, "--unknown", "value"]).output().unwrap();
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.starts_with("reach: pending, not enforced\n"),
            "{stderr}"
        );
        assert!(!out.status.success());
    }
}

#[test]
fn artifact_unpinned_dependencies_are_aggregated_before_resolution() {
    refusal(
        |f, _| {
            std::fs::write(f.root.join("Cargo.lock"), "version=4\n[[package]]\nname='unpinned-registry'\nversion='1.0.0'\nsource='registry+https://github.com/rust-lang/crates.io-index'\n[[package]]\nname='unpinned-git'\nversion='1.0.0'\nsource='git+https://example.test/repo'\n").unwrap();
            f.git(&["add", "Cargo.lock"]);
            f.git(&["commit", "-qm", "invalid pins"]);
        },
        &[
            "registry dependency unpinned-registry has no lockfile checksum",
            "git dependency unpinned-git has no pinned commit",
        ],
        true,
    );
}

#[test]
fn artifact_compiler_input_outside_repository_refuses() {
    let _cargo = CARGO.lock().unwrap_or_else(|e| e.into_inner());
    let Some(f) = Foreign::try_new() else { return };
    let external = f.dir.path().join("external.txt");
    std::fs::write(&external, "outside").unwrap();
    std::fs::write(
        f.root.join("src/main.rs"),
        format!(
            "fn main() {{ println!(\"{{}}\", include_str!({:?})); }}\n",
            external
        ),
    )
    .unwrap();
    f.commit();
    f.build();
    success(f.describe());
    let input = f.input();
    let out = f.put(&input, true);
    assert_eq!(out.status.code(), Some(3), "{}", text(&out));
    assert!(text(&out).contains("compiler input outside captured view"));
    f.absent(&f.descriptor(), &input);
}
