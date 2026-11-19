//! Describe's declared target is recorded once and changes only the variant method identity.
#![cfg(all(feature = "peer", unix))]

use std::os::unix::fs::PermissionsExt;
use std::process::{Command, Output};

use eprfs_core::{BlobCid, MethodDescriptor};
use tempfile::TempDir;

struct Fixture(TempDir);

impl Fixture {
    fn new() -> Self {
        let fixture = Self(TempDir::new().unwrap());
        let root = fixture.0.path();
        std::fs::create_dir(root.join("crate")).unwrap();
        std::fs::create_dir(root.join("bin")).unwrap();
        std::fs::write(
            root.join("manifest.json"),
            r#"{"steps":{"build":{"inputs":{"sources":["crate/src/**"]}}}}"#,
        )
        .unwrap();
        std::fs::create_dir_all(root.join("crate/src")).unwrap();
        std::fs::write(
            root.join("crate/Cargo.toml"),
            "[package]\nname='doorway'\nversion='0.1.0'\n",
        )
        .unwrap();
        std::fs::write(root.join("crate/src/main.rs"), "fn main() {}\n").unwrap();
        let metadata = serde_json::json!({"packages": [{"source": null, "manifest_path": root.join("crate/Cargo.toml"), "targets": [{"name": "doorway", "kind": ["bin"], "src_path": root.join("crate/src/main.rs")}]}]});
        let cargo = format!("case \"$1\" in -V) echo 'cargo fixture';; metadata) echo '{metadata}';; *) exit 1;; esac");
        // Tool responses are fixed; describe still exercises its real argument parser, metadata
        // reader and descriptor writer without launching a nested Cargo process in cargo test.
        for (name, body) in [
            (
                "rustc",
                "printf 'rustc fixture\nhost: x86_64-unknown-linux-gnu\nLLVM version: 22.1.2\n'",
            ),
            ("cargo", cargo.as_str()),
        ] {
            let path = root.join("bin").join(name);
            std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        std::fs::write(root.join("crate/Cargo.lock"), "version = 4\n").unwrap();
        for args in [
            vec!["init", "-q"],
            vec!["add", "crate", "manifest.json"],
            vec![
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.test",
                "commit",
                "-qm",
                "fixture",
            ],
        ] {
            let out = elohim_epr_cli::process::build_command("git", &args, root, &[])
                .output()
                .unwrap();
            assert!(
                out.status.success(),
                "{}",
                String::from_utf8_lossy(&out.stderr)
            );
        }
        fixture
    }

    fn describe(&self, extra: &[&str]) -> Output {
        self.describe_env(extra, &[])
    }

    /// [`Fixture::describe`] with `env` set in describe's otherwise cleared environment.
    fn describe_env(&self, extra: &[&str], env: &[(&str, &str)]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_epr"))
            .env_clear()
            .envs(env.iter().copied())
            .env(
                "PATH",
                format!("{}:/bin", self.0.path().join("bin").display()),
            )
            .env("CARGO_HOME", self.0.path().join("cargo-home"))
            .env("XDG_CONFIG_HOME", self.0.path().join("config"))
            .args([
                "artifact",
                "describe",
                "--project",
                "doorway",
                "--bin",
                "doorway",
                "--crate",
                "crate",
                "--sources",
                "manifest.json#build",
                "--rustflags",
                "",
                "--repo",
            ])
            .arg(self.0.path())
            .args(extra)
            .output()
            .unwrap()
    }

    fn descriptor(&self, args: &[&str]) -> MethodDescriptor {
        let out = self.describe(args);
        assert!(
            out.status.success(),
            "{} {}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        serde_json::from_slice(&out.stdout).unwrap()
    }
}

#[test]
fn artifact_describe_variants_share_only_family() {
    let fixture = Fixture::new();
    let v2 = fixture.descriptor(&["--target-isa", "x86-64-v2"]);
    let v3 = fixture.descriptor(&["--target-isa", "x86-64-v3"]);
    assert_eq!(v2.non_path_inputs["target.isa"], "x86-64-v2");
    assert_eq!(v2.non_path_inputs["RUSTFLAGS"], "");
    assert_ne!(v2.cid().unwrap(), v3.cid().unwrap());
    assert_eq!(v2.family_cid().unwrap(), v3.family_cid().unwrap());
    assert_eq!(
        v2.family_cid().unwrap(),
        fixture.descriptor(&[]).cid().unwrap()
    );
    let path = fixture.0.path().join("method.json");
    let out = fixture.describe(&["--target-isa", "x86-64-v2", "--out", path.to_str().unwrap()]);
    assert!(out.status.success());
    let written: MethodDescriptor = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(written, v2);
    assert_eq!(
        String::from_utf8(out.stdout).unwrap().trim(),
        v2.cid().unwrap().to_string()
    );
}

#[test]
fn artifact_describe_refuses_every_target_cpu_spelling_only_with_declared_isa() {
    let fixture = Fixture::new();
    for prefix in ["-C", "-C ", "-C=", "--codegen ", "--codegen="] {
        for key in ["target-cpu", "target_cpu"] {
            let flags = format!("{prefix}{key}=native");
            let path = fixture.0.path().join("refused.json");
            let out = fixture.describe(&[
                "--target-isa",
                "x86-64-v2",
                "--rustflags",
                &flags,
                "--out",
                path.to_str().unwrap(),
            ]);
            assert_eq!(out.status.code(), Some(3), "{flags}");
            let reason = String::from_utf8(out.stdout).unwrap();
            assert!(
                reason.contains("target.isa") && reason.contains("target-cpu"),
                "{reason}"
            );
            assert!(!path.exists());
            let host_bound = fixture.descriptor(&["--rustflags", &flags]);
            assert_eq!(host_bound.non_path_inputs["RUSTFLAGS"], flags);
            assert_eq!(host_bound.family_cid().unwrap(), host_bound.cid().unwrap());
        }
    }
}

/// `--target-cflags=-march=x86-64` is the producer's spelling: the inline `=` value is the slot's
/// whole value and swallows no following argument, and the spaced form still takes a value that
/// starts with `-`.
#[test]
fn artifact_describe_takes_a_value_inline_or_spaced() {
    let fixture = Fixture::new();
    for args in [
        &["--target-cflags=-march=x86-64", "--target-isa", "x86-64"][..],
        &["--target-cflags", "-march=x86-64", "--target-isa=x86-64"][..],
        &["--target-isa=x86-64", "--target-cflags=-march=x86-64"][..],
    ] {
        let descriptor = fixture.descriptor(args);
        assert_eq!(
            descriptor.non_path_inputs["target.cflags"], "-march=x86-64",
            "{args:?}"
        );
        assert_eq!(
            descriptor.non_path_inputs["target.isa"], "x86-64",
            "{args:?}"
        );
        assert!(
            descriptor.non_path_inputs.keys().all(|k| !k.contains('=')),
            "{args:?}: {:?}",
            descriptor.non_path_inputs
        );
    }
    let inline = fixture.descriptor(&["--env=EXTRA=1", "--rustflags=--cfg=fixture"]);
    assert_eq!(inline.non_path_inputs["EXTRA"], "1");
    assert_eq!(inline.non_path_inputs["RUSTFLAGS"], "--cfg=fixture");
    let out = fixture.describe(&["--json=yes"]);
    assert_ne!(out.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&out.stderr).contains("--json"));
}

#[test]
fn artifact_describe_validates_all_target_slots_and_env_extension() {
    let fixture = Fixture::new();
    let cid = BlobCid::compute(b"sysroot").to_string();
    let descriptor = fixture.descriptor(&[
        "--target-arch",
        "x86_64",
        "--target-isa",
        "haswell",
        "--target-os",
        "linux",
        "--target-libc",
        "glibc",
        "--target-cflags",
        "-march=x86-64-v2",
        "--target-sysroot",
        &cid,
    ]);
    for (name, value) in [
        ("arch", "x86_64"),
        ("isa", "haswell"),
        ("os", "linux"),
        ("libc", "glibc"),
        ("cflags", "-march=x86-64-v2"),
        ("sysroot", cid.as_str()),
    ] {
        assert_eq!(descriptor.non_path_inputs[&format!("target.{name}")], value);
    }
    for args in [
        vec!["--target-unknown", "value"],
        vec!["--env", "target.unknown=value"],
        vec!["--target-sysroot", "/sysroot"],
        vec!["--target-isa", ""],
        vec!["--target-isa", "v2", "--target-isa", "v3"],
        vec!["--target-isa", "v2", "--env", "target.isa=v3"],
    ] {
        let out = fixture.describe(&args);
        assert_eq!(out.status.code(), Some(3), "{args:?}");
        assert!(String::from_utf8(out.stdout).unwrap().contains("target."));
    }
}

/// Describe records every C-family variable it sees as a build input, as it records the native
/// toolchain (row 29.2), and leaves the rest of the environment out.
#[test]
fn artifact_describe_records_the_c_family_it_sees() {
    let fixture = Fixture::new();
    let family = [
        ("CFLAGS", "-O2"),
        ("CXXFLAGS", "-O2"),
        ("CPPFLAGS", "-DX=1"),
        ("CFLAGS_x86_64_unknown_linux_gnu", "-march=x86-64"),
        ("CXXFLAGS_x86_64_unknown_linux_gnu", "-march=x86-64"),
        ("CFLAGS_x86_64-unknown-linux-gnu", "-march=x86-64"),
        ("CXXFLAGS_x86_64-unknown-linux-gnu", "-march=x86-64"),
        ("HOST_CFLAGS", "-O1"),
        ("TARGET_CFLAGS", "-O1"),
        ("HOST_CXXFLAGS", "-O1"),
        ("TARGET_CXXFLAGS", "-O1"),
        ("CC_x86_64_unknown_linux_gnu", "gcc"),
        ("CC_x86_64-unknown-linux-gnu", "gcc"),
        ("HOST_CC", "gcc"),
        ("TARGET_CC", "gcc"),
        ("CRATE_CC_NO_DEFAULTS", "1"),
    ];
    let env = [&family[..], &[("LDFLAGS", "-s"), ("UNRELATED", "1")]].concat();
    let out = fixture.describe_env(&[], &env);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let descriptor: MethodDescriptor = serde_json::from_slice(&out.stdout).unwrap();
    for (name, value) in family {
        assert_eq!(
            descriptor.non_path_inputs.get(name).map(String::as_str),
            Some(value),
            "{name}"
        );
    }
    for name in ["LDFLAGS", "UNRELATED"] {
        assert!(!descriptor.non_path_inputs.contains_key(name), "{name}");
    }
}
