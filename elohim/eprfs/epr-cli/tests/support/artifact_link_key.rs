//! The link half of the derivation key: which linker, driver, link inputs and compiler defaults
//! move the key, and which unmodelled selections refuse before anything is recorded.
use super::*;

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
    assert!(!log.exists(), "refused before any brit write");
    assert!(events(device.root()).is_empty());
}

#[test]
fn artifact_link_input_bytes_move_key() {
    let repo = checkout();
    let tools = TempDir::new().unwrap();
    let input = tools.path().join("Scrt1.o");
    std::fs::write(&input, b"glibc 2.39-128").unwrap();
    fake_tool(tools.path(), "cc", &format!(
        "case \"$1\" in\n--version) echo 'cc fixture' ;;\n-E) echo '#define __SSE2__ 1' ;;\n-print-file-name=Scrt1.o) echo '{}' ;;\n-print-file-name=*) exec /usr/bin/cc \"$1\" ;;\nesac",
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
        "case \"$1\" in\n--version) echo 'cc fixture' ;;\n-E) echo '#define {march} 1' ;;\n-print-file-name=*) exec /usr/bin/cc \"$1\" ;;\nesac"
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
            assert!(!log.exists(), "refused before any brit write");
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
    assert!(!log.exists(), "refused before any brit write");
    assert!(events(device.root()).is_empty());
}

fn fixture_driver(dir: &Path, name: &str, version: &str, macros: &str, input: &Path) {
    fake_tool(dir, name, &format!(
        "case \"$1\" in\n--version) echo '{version}' ;;\n-E) test \"$*\" = '-E -dM -x c -' || exit 1; read ignored && exit 1; printf '%s\\n' '{macros}' ;;\n-print-file-name=Scrt1.o) echo '{}' ;;\n-print-file-name=*) exec /usr/bin/cc \"$1\" ;;\n*) exit 1 ;;\nesac", input.display()
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
    let Some(real) = toolchain_pin::real_toolchain_or_skip() else {
        return;
    };
    let repo = checkout();
    let scratch = TempDir::new().unwrap();
    let (_, mut descriptor) = method_file_with(
        scratch.path(),
        &["crate/src"],
        &[
            ("PATH", &std::env::var("PATH").unwrap()),
            ("RUSTFLAGS", "-C link-arg=-fuse-ld=lld"),
            ("RUSTUP_TOOLCHAIN", &real.pin),
        ],
    );
    descriptor
        .tools
        .insert("toolchain".into(), real.pin.clone());
    descriptor.tools.insert("rustc".into(), real.identity);
    elohim_epr_cli::artifact::closure_key(repo.path(), &descriptor).unwrap();
    eprintln!("real gcc + rustc {} bundled-lld key: PASS", real.pin);
}

#[test]
fn artifact_real_describe_keys_with_process_homes_and_home_location_is_not_an_input() {
    let Some(real) = toolchain_pin::real_toolchain_or_skip() else {
        return;
    };
    // A rustup device must name its home explicitly: the test varies HOME, which would otherwise
    // relocate `HOME/.rustup`. Without rustup (CI's nix shell) the PATH compiler needs no home.
    let rustup_home = std::env::var_os("RUSTUP_HOME");
    if real.via_rustup && rustup_home.is_none() {
        eprintln!("SKIP: rustup device without RUSTUP_HOME");
        return;
    }
    let repo = checkout();
    let scratch = TempDir::new().unwrap();
    let tools = scratch.path().join("bin");
    std::fs::create_dir_all(&tools).unwrap();
    // Keep Cargo's metadata response synthetic: describe runs its actual parser/writer and
    // the real rustc, without nesting another cargo command inside the test's cargo process.
    let metadata = serde_json::json!({"packages": [{"source": null, "manifest_path": repo.path().join("crate/Cargo.toml"), "targets": [{"name": "fixture", "kind": ["bin"], "src_path": repo.path().join("crate/src/main.rs")}]}]});
    fake_tool(&tools, "cargo", &format!("case \"$1\" in -V|--version) echo 'cargo fixture';; metadata) echo '{metadata}';; *) exit 1;; esac"));
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
    std::fs::write(repo.path().join("crate/Cargo.lock"), "version = 4\n").unwrap();
    git(
        repo.path(),
        &[
            "add",
            "crate/Cargo.toml",
            "crate/Cargo.lock",
            "manifest.json",
        ],
    );
    git(repo.path(), &["commit", "-qm", "describe fixture"]);

    let build_path = std::env::var("PATH").unwrap();
    let mut describe = Command::new(env!("CARGO_BIN_EXE_epr"));
    describe
        .env_clear()
        .env("PATH", format!("{}:{build_path}", tools.display()))
        .env("RUSTUP_TOOLCHAIN", &real.pin)
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
        ])
        .arg(format!("RUSTUP_TOOLCHAIN={}", real.pin))
        .arg("--env")
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
    assert!(descriptor.tools["rustc"].contains(&format!("release: {}", real.pin)));
    let method = scratch.path().join("method.json");
    std::fs::write(&method, &out.stdout).unwrap();
    let baseline = elohim_epr_cli::artifact::closure_key(repo.path(), &descriptor).unwrap();

    // Isolated children vary HOME without mutating the parallel test process's environment.
    // RUSTUP_HOME continues to locate the same installed compiler in both cases.
    for home in ["home-a", "home-b"] {
        let home = scratch.path().join(home);
        std::fs::create_dir(&home).unwrap();
        let mut key_command = Command::new(env!("CARGO_BIN_EXE_epr"));
        key_command
            .env_clear()
            .env("PATH", &build_path)
            .env("CC", "cc")
            .env("RUSTFLAGS", "")
            .env("RUSTUP_TOOLCHAIN", &real.pin)
            .env("HOME", &home)
            .env("XDG_CONFIG_HOME", scratch.path().join("config"));
        if let Some(rustup_home) = &rustup_home {
            key_command.env("RUSTUP_HOME", rustup_home);
        }
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
