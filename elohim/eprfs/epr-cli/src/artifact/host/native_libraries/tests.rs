//! Key sensitivity and refusal at the rustc native-library / link-driver seam.

use super::*;
use crate::artifact::{closure_key, host, PATH_INPUT};
use eprfs_core::TreePath;

#[path = "../../../../tests/support/toolchain_pin.rs"]
mod toolchain_pin;

fn stub(dir: &Path, name: &str, body: &str) {
    let identity = if name == "rustc" {
        format!(
            "if [ \"$1\" = -vV ]; then printf '%s' '{}'; exit 0; fi\n",
            host::fixture::RUSTC_IDENTITY
        )
    } else {
        String::new()
    };
    let file = dir.join(name);
    std::fs::write(
        &file,
        format!("#!/bin/sh\nif [ \"$1\" = --version ]; then exit 0; fi\n{identity}{body}\n"),
    )
    .unwrap();
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o755)).unwrap();
    host::fixture::wait_for_stub(&file);
}

fn descriptor(path: &str) -> MethodDescriptor {
    MethodDescriptor {
        name: "mesh-build".into(),
        tools: [
            ("rustc".into(), host::fixture::RUSTC_IDENTITY.into()),
            ("toolchain".into(), "1.96.1".into()),
        ]
        .into(),
        flags: vec![],
        params: None,
        exec: None,
        step: None,
        embedder: None,
        view: vec![TreePath::from_slash_bytes(b"main.rs").unwrap()],
        non_path_inputs: [
            (PATH_INPUT.into(), path.into()),
            ("RUSTUP_TOOLCHAIN".into(), "1.96.1".into()),
        ]
        .into(),
        deterministic: false,
    }
}

fn lines(descriptor: &MethodDescriptor) -> ArtifactResult<String> {
    let selections = host::resolve(
        &mut descriptor.non_path_inputs.clone(),
        &toml::Value::Table(Default::default()),
        host::target(descriptor),
    )?;
    host::tool_lines(descriptor, &selections)
}

fn checkout(dir: &Path) {
    std::fs::write(dir.join("main.rs"), "fn main() {}\n").unwrap();
    for args in [
        vec!["init", "-q"],
        vec!["add", "main.rs"],
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
        let output = Command::new("git")
            .args(args)
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn artifact_native_library_note_controls_keyed_bytes_and_archive_fallback() {
    let repo = tempfile::tempdir().unwrap();
    checkout(repo.path());
    let tools = tempfile::tempdir().unwrap();
    let library = tools.path().join("libfoo.so");
    std::fs::write(&library, b"first library").unwrap();
    stub(tools.path(), "cc", &format!(
        "case \"$1\" in\n--version) echo 'gcc fixture' ;;\n-E) echo '#define __SSE2__ 1' ;;\n-print-file-name=libfoo.so) echo '{}' ;;\n-print-file-name=*) echo \"${{1#*=}}\" ;;\nesac", library.display()
    ));
    stub(
        tools.path(),
        "rustc",
        "echo 'note: native-static-libs: -lfoo -lc' >&2",
    );
    let descriptor = descriptor(tools.path().to_str().unwrap());
    let before = closure_key(repo.path(), &descriptor).unwrap();
    assert!(lines(&descriptor).unwrap().contains("link:libfoo.so\t"));
    assert!(lines(&descriptor).unwrap().contains("link:libc\t-\n"));
    std::fs::write(&library, b"second library").unwrap();
    assert_ne!(
        before.key_cid,
        closure_key(repo.path(), &descriptor).unwrap().key_cid
    );
    stub(
        tools.path(),
        "rustc",
        "echo 'note: native-static-libs: -lc' >&2",
    );
    assert!(!lines(&descriptor).unwrap().contains("link:libfoo"));
    let without_foo = closure_key(repo.path(), &descriptor).unwrap();
    std::fs::write(&library, b"unread library").unwrap();
    assert_eq!(
        without_foo.key_cid,
        closure_key(repo.path(), &descriptor).unwrap().key_cid
    );

    // The same declared library falls back to a static archive only when .so is unresolved.
    let archive = tools.path().join("libfoo.a");
    std::fs::rename(&library, &archive).unwrap();
    stub(tools.path(), "cc", &format!(
        "case \"$1\" in\n--version) echo 'gcc fixture' ;;\n-E) echo '#define __SSE2__ 1' ;;\n-print-file-name=libfoo.a) echo '{}' ;;\n-print-file-name=*) echo \"${{1#*=}}\" ;;\nesac", archive.display()
    ));
    stub(
        tools.path(),
        "rustc",
        "echo 'note: native-static-libs: -lfoo -lc' >&2",
    );
    let linked = lines(&descriptor).unwrap();
    assert!(linked.contains("link:libfoo.a\t"));
    assert!(!linked.contains("link:libfoo.so\t"));
    let before = closure_key(repo.path(), &descriptor).unwrap();
    std::fs::write(&archive, b"changed archive").unwrap();
    assert_ne!(
        before.key_cid,
        closure_key(repo.path(), &descriptor).unwrap().key_cid
    );
    // Script referents use the same fail-closed reader as startup objects.
    std::fs::write(&archive, format!("INPUT ( {} )", library.display())).unwrap();
    assert!(
        matches!(closure_key(repo.path(), &descriptor), Err(ArtifactError::Gap(message)) if message.contains("does not resolve"))
    );
}

#[test]
fn artifact_native_library_missing_or_malformed_note_refuses_with_gap() {
    let repo = tempfile::tempdir().unwrap();
    checkout(repo.path());
    let tools = tempfile::tempdir().unwrap();
    stub(
        tools.path(),
        "cc",
        "case \"$1\" in\n-print-file-name=*) echo \"${1#*=}\" ;;\nesac",
    );
    let descriptor = descriptor(tools.path().to_str().unwrap());
    for note in [
        "",
        "note: native-static-libs: ",
        "note: native-static-libs: -l",
        "note: native-static-libs: -lfoo garbage",
        "note: native-static-libs: -lfoo\nnote: native-static-libs: -lc",
    ] {
        stub(tools.path(), "rustc", &format!("echo '{note}' >&2"));
        assert!(
            matches!(closure_key(repo.path(), &descriptor), Err(ArtifactError::Gap(message)) if message.contains("native-static-libs")),
            "{note:?}"
        );
    }
}

#[test]
fn artifact_native_library_probe_uses_recorded_environment_target_and_private_scratch() {
    let tools = tempfile::tempdir().unwrap();
    let marker = tools.path().join("scratch-path");
    stub(
        tools.path(),
        "rustc",
        &format!(
            r#"
test "$RUSTUP_TOOLCHAIN" = fixture-toolchain || exit 1
test "$RECORDED_INPUT" = recorded || exit 2
test "$LANG" = C || exit 3
test "$RUSTUP_AUTO_INSTALL" = 0 || exit 4
test -z "$UNRECORDED_INPUT" || exit 9
test "$TMPDIR" = "$PWD" || exit 5
test "$(/usr/bin/stat -c %a "$PWD")" = 700 || exit 8
test "$*" = '--print native-static-libs --crate-type staticlib --target fixture-target -o probe.a -' || exit 6
read ignored && exit 7
echo "$PWD" > '{}'
echo archive > probe.a
echo 'note: native-static-libs: -lfoo -lc' >&2
"#,
            marker.display()
        ),
    );
    let mut descriptor = descriptor(tools.path().to_str().unwrap());
    descriptor
        .tools
        .insert("toolchain".into(), "fixture-toolchain".into());
    descriptor.flags = vec!["--target=fixture-target".into()];
    descriptor
        .non_path_inputs
        .insert("RECORDED_INPUT".into(), "recorded".into());
    descriptor
        .non_path_inputs
        .insert("TMPDIR".into(), "/not/the/process/tmpdir".into());
    assert_eq!(
        probe(&descriptor, tools.path().to_str().unwrap()).unwrap(),
        ["c".into(), "foo".into()].into()
    );
    let scratch = std::fs::read_to_string(marker).unwrap();
    let scratch = Path::new(scratch.trim());
    assert_eq!(scratch.parent().unwrap(), std::env::temp_dir());
    assert!(!scratch.exists(), "the private probe directory is removed");
}

#[test]
fn artifact_real_toolchain_native_link_lines() {
    let Some(real) = toolchain_pin::real_toolchain_or_skip() else {
        return;
    };
    let mut descriptor = descriptor(&std::env::var("PATH").unwrap());
    // The recorded pin selects the compiler when PATH's rustc is a rustup proxy.
    descriptor
        .tools
        .insert("toolchain".into(), real.pin.clone());
    descriptor
        .non_path_inputs
        .insert("RUSTUP_TOOLCHAIN".into(), real.pin);
    descriptor.tools.insert("rustc".into(), real.identity);
    let linked = lines(&descriptor).unwrap();
    for name in ["m", "pthread", "dl", "rt", "util", "c", "gcc_s"] {
        assert!(
            linked
                .lines()
                .any(|line| line.starts_with(&format!("link:lib{name}.so\t"))
                    || line.starts_with(&format!("link:lib{name}.a\t"))),
            "missing {name}: {linked}"
        );
    }
    for line in linked.lines().filter(|line| line.starts_with("link:")) {
        eprintln!("{}", line.split('\t').next().unwrap());
    }
}

#[test]
fn artifact_native_library_requires_full_matching_identity_before_probing() {
    let repo = tempfile::tempdir().unwrap();
    checkout(repo.path());
    let tools = tempfile::tempdir().unwrap();
    let path = tools.path().to_str().unwrap();
    let mut descriptor = descriptor(path);
    stub(
        tools.path(),
        "cc",
        "case \"$1\" in -print-file-name=*) echo \"${1#*=}\" ;; esac",
    );
    // PATH selects another compiler which ignores the pinned RUSTUP_TOOLCHAIN. No library
    // response is needed: the identity mismatch must stop the second invocation.
    for actual in [
        host::fixture::RUSTC_IDENTITY.replace("release: 1.96.1", "release: 1.99.1"),
        host::fixture::RUSTC_IDENTITY.replace("fixture-hash", "other-hash"),
        host::fixture::RUSTC_IDENTITY.replace("22.1.2", "23.0.0"),
    ] {
        let rustc = tools.path().join("rustc");
        std::fs::write(&rustc, format!(
            "#!/bin/sh\n[ \"$1\" = --version ] && exit 0\n[ \"$1\" = -vV ] || exit 99\nprintf '%s' '{actual}'\n"
        )).unwrap();
        std::fs::set_permissions(&rustc, std::fs::Permissions::from_mode(0o755)).unwrap();
        host::fixture::wait_for_stub(&rustc);
        let Err(ArtifactError::Gap(message)) = closure_key(repo.path(), &descriptor) else {
            panic!("compiler mismatch must be a typed Gap");
        };
        assert!(message.contains("rustc identity mismatch"), "{message}");
        for identity in [host::fixture::RUSTC_IDENTITY, &actual] {
            for line in identity
                .lines()
                .filter(|line| line.starts_with("release: ") || line.starts_with("commit-hash: "))
            {
                assert!(message.contains(line), "{message}");
            }
        }
    }
    for incomplete in [None, Some("rustc fixture\nhost: x86_64-unknown-linux-gnu")] {
        descriptor.tools.remove("rustc");
        if let Some(identity) = incomplete {
            descriptor.tools.insert("rustc".into(), identity.into());
        }
        assert!(
            matches!(closure_key(repo.path(), &descriptor), Err(ArtifactError::Gap(message))
            if message.contains("full rustc -vV identity; describe again"))
        );
    }
}

#[test]
fn artifact_native_library_missing_toolchain_refuses_without_installing() {
    // The subject is the rustup-proxy path, so it is built hermetically: a `rustc` that is a
    // link to a `rustup` stub (what `is_rustup_proxy` recognises, whatever compiler this machine
    // has) and a rustup home that exists but holds no toolchain at all. Nothing is installed.
    let tools = tempfile::tempdir().unwrap();
    stub(tools.path(), "rustup", "exit 99");
    let rustc = tools.path().join("rustc");
    std::os::unix::fs::symlink(tools.path().join("rustup"), &rustc).unwrap();
    let home = tempfile::tempdir().unwrap();
    std::fs::create_dir(home.path().join("toolchains")).unwrap();
    let mut descriptor = descriptor(tools.path().to_str().unwrap());
    descriptor
        .tools
        .insert("toolchain".into(), "9.99.99".into());
    descriptor
        .non_path_inputs
        .insert("RUSTUP_TOOLCHAIN".into(), "9.99.99".into());
    let Err(ArtifactError::Gap(message)) = proxy_compiler(
        std::fs::canonicalize(&rustc).unwrap(),
        &descriptor,
        Some(home.path().to_path_buf()),
    ) else {
        panic!("uninstalled toolchain must refuse");
    };
    assert!(message.contains("not installed"), "{message}");
    assert!(!message.contains("syncing channel"), "{message}");
    assert!(!message.contains("downloading"), "{message}");
    assert_eq!(
        home.path().join("toolchains").read_dir().unwrap().count(),
        0,
        "refusal must not install"
    );
}

#[test]
fn artifact_native_library_location_vars_stay_inside_scratch_unless_they_exist() {
    let scratch = tempfile::tempdir().unwrap();
    let existing = tempfile::tempdir().unwrap();
    let outside = existing.path().join("never-created");
    let existing_path = existing.path().to_path_buf();
    let outside_path = outside.clone();
    let env: std::collections::BTreeMap<_, _> = location_env(scratch.path(), |name| match name {
        "RUSTUP_HOME" => Some(outside_path.clone().into_os_string()),
        "CARGO_HOME" => Some(existing_path.clone().into_os_string()),
        "HOME" => Some("/home/somebody".into()),
        _ => None,
    })
    .into_iter()
    .collect();
    // A nonexistent RUSTUP_HOME is never handed to the child: a disguised proxy could create it.
    let rustup_home = std::path::PathBuf::from(&env["RUSTUP_HOME"]);
    assert!(rustup_home.starts_with(scratch.path()), "{rustup_home:?}");
    assert_ne!(rustup_home, outside);
    // An existing home passes through unchanged; HOME passes through when set.
    assert_eq!(
        std::path::PathBuf::from(&env["CARGO_HOME"]),
        existing.path()
    );
    assert_eq!(env["HOME"], std::ffi::OsString::from("/home/somebody"));
    // Unset homes also point inside scratch; unset HOME stays unset.
    let unset: std::collections::BTreeMap<_, _> =
        location_env(scratch.path(), |_| None).into_iter().collect();
    assert!(std::path::PathBuf::from(&unset["RUSTUP_HOME"]).starts_with(scratch.path()));
    assert!(std::path::PathBuf::from(&unset["CARGO_HOME"]).starts_with(scratch.path()));
    assert!(!unset.contains_key("HOME"));
    assert!(!outside.exists());
}
