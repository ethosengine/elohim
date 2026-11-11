use super::*;

#[test]
fn artifact_every_recorded_build_variable_has_exactly_one_class() {
    let mut names = std::collections::BTreeSet::new();
    for (name, _) in BUILD_INPUT_CLASSES {
        assert!(names.insert(name), "duplicate classification: {name}");
    }
    for name in NATIVE_BUILD_ENV
        .into_iter()
        .chain(BENIGN_BUILD_ENV)
        .chain(["RUSTFLAGS", "RUSTUP_TOOLCHAIN"])
    {
        let rows: Vec<_> = BUILD_INPUT_CLASSES
            .iter()
            .filter(|(key, _)| *key == name)
            .collect();
        assert_eq!(rows.len(), 1, "{name} must be explicitly classified once");
        assert_eq!(build_input_class(name), BuildInputClass::Method);
    }
    for name in [
        "CARGO_ENCODED_RUSTFLAGS",
        "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS",
        "CARGO_BUILD_RUSTFLAGS",
        "CUSTOM_ENV",
        "target",
        "targeted.isa",
    ] {
        assert_eq!(build_input_class(name), BuildInputClass::Method);
    }
    for name in ["arch", "isa", "os", "libc", "cflags", "sysroot"] {
        assert_eq!(
            build_input_class(&format!("target.{name}")),
            BuildInputClass::Target
        );
    }
}

#[test]
fn artifact_loaded_target_inputs_refuse_unknown_names_and_invalid_sysroot() {
    for (name, value) in [
        ("target.unknown", "value"),
        ("target.sysroot", "/a/path"),
        ("target.isa", " "),
    ] {
        let error = validate_target_inputs(&[(name.into(), value.into())].into()).unwrap_err();
        assert!(matches!(error, ArtifactError::Gap(_)));
        assert!(error.to_string().contains(name));
    }
    let sysroot = BlobCid::compute(b"reserved sysroot").to_string();
    assert!(validate_target_inputs(&[("target.sysroot".into(), sysroot)].into()).is_ok());
}

#[test]
fn artifact_recorded_flag_sources_cannot_repeat_declared_isa() {
    for name in [
        "RUSTFLAGS",
        "CARGO_BUILD_RUSTFLAGS",
        "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS",
        "CARGO_ENCODED_RUSTFLAGS",
    ] {
        let separator = if name == "CARGO_ENCODED_RUSTFLAGS" {
            "\x1f"
        } else {
            " "
        };
        let inputs = [
            ("target.isa".into(), "x86-64-v2".into()),
            (
                name.into(),
                format!("--codegen{separator}target_cpu=native"),
            ),
        ]
        .into();
        assert!(validate_target_inputs(&inputs)
            .unwrap_err()
            .to_string()
            .contains(name));
    }
}

#[test]
fn artifact_effective_config_cannot_repeat_declared_isa() {
    let mut inputs = [("target.isa".into(), "x86-64-v2".into())].into();
    let config = "[build]\nrustflags=['--codegen=target_cpu=native']"
        .parse()
        .unwrap();
    assert!(
        host::resolve(&mut inputs, &config, "x86_64-unknown-linux-gnu")
            .unwrap_err()
            .to_string()
            .contains("target-cpu")
    );
}
