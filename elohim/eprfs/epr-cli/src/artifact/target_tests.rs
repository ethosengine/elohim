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
fn artifact_c_family_names_every_spelling_the_cc_crate_reads() {
    for name in [
        "CFLAGS",
        "CXXFLAGS",
        "CPPFLAGS",
        "ASMFLAGS",
        "CFLAGS_x86_64_unknown_linux_gnu",
        "CFLAGS_x86_64-unknown-linux-gnu",
        "CXXFLAGS_x86_64-unknown-linux-gnu",
        "HOST_CFLAGS",
        "TARGET_CXXFLAGS",
        "CC_x86_64_unknown_linux_gnu",
        "CXX_x86_64-unknown-linux-gnu",
        "HOST_CC",
        "TARGET_CC",
        "HOST_CXX",
        "CRATE_CC_NO_DEFAULTS",
        "CRATE_CC_NO_DEFAULTS_x86_64_unknown_linux_gnu",
    ] {
        assert!(is_c_family_var(name), "{name}");
        assert!(is_build_var(name), "{name}");
    }
    // Plain CC/CXX stay native toolchain inputs; linker and look-alike names are not C flags.
    for name in [
        "CC",
        "CXX",
        "AR",
        "LDFLAGS",
        "CFLAGSX",
        "HOST_CC_x",
        "CCACHE_DIR",
        "TARGET",
    ] {
        assert!(!is_c_family_var(name), "{name}");
    }
}

/// Ruling 4 (L2a rework): the archiver and ranlib selectors and their flags are C tools too, in
/// every spelling cc 1.11.1 reads (`get_base_archiver_variant` and `envflags` over `target_envs`,
/// lib.rs 4725/4774/4788/5452), and the switches cc reads by name (`CC_FORCE_DISABLE`,
/// `CC_ENABLE_DEBUG_OUTPUT`) are recorded like any other C-family input.
#[test]
fn artifact_c_family_names_the_archiver_and_ranlib_in_every_spelling() {
    for name in [
        "HOST_AR",
        "TARGET_AR",
        "AR_x86_64_unknown_linux_gnu",
        "AR_x86_64-unknown-linux-gnu",
        "ARFLAGS",
        "HOST_ARFLAGS",
        "TARGET_ARFLAGS",
        "ARFLAGS_x86_64_unknown_linux_gnu",
        "ARFLAGS_x86_64-unknown-linux-gnu",
        "RANLIB",
        "HOST_RANLIB",
        "TARGET_RANLIB",
        "RANLIB_x86_64_unknown_linux_gnu",
        "RANLIB_x86_64-unknown-linux-gnu",
        "RANLIBFLAGS",
        "TARGET_RANLIBFLAGS",
        "RANLIBFLAGS_x86_64_unknown_linux_gnu",
        "NVCC",
        "HOST_NVCC",
        "CXXSTDLIB",
        "CXXSTDLIB_x86_64_unknown_linux_gnu",
        "TARGET_CXXSTDLIB_STATIC",
        "CC_FORCE_DISABLE",
        "CC_ENABLE_DEBUG_OUTPUT",
    ] {
        assert!(is_c_family_var(name), "{name}");
        assert!(is_build_var(name), "{name}");
    }
    for name in ["ARCH", "RANLIBX", "HOST_ARX", "TARGET_ARCH"] {
        assert!(!is_c_family_var(name), "{name}");
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

#[test]
fn artifact_requires_is_opaque_array_with_canonical_object_order() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("requires.json");
    // No predicate interpretation here, including future shapes and array ordering.
    std::fs::write(&path, r#"[ {"z":[2,1],"a":{"z":"true","a":"é"}}, 1 ]"#).unwrap();
    let (cid, bytes) = read_requires(&path).unwrap();
    assert_eq!(
        bytes.as_ref(),
        "[{\"a\":{\"a\":\"é\",\"z\":\"true\"},\"z\":[2,1]},1]".as_bytes()
    );
    std::fs::write(&path, &bytes).unwrap();
    assert_eq!(read_requires(&path).unwrap().0, cid);
    for invalid in ["{}", "null", "true", "[broken"] {
        std::fs::write(&path, invalid).unwrap();
        assert!(read_requires(&path).is_err());
    }
    std::fs::write(&path, "[]").unwrap();
    assert_eq!(read_requires(&path).unwrap().1.as_ref(), b"[]");
}
