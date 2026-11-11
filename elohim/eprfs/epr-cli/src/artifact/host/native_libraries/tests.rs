//! Key sensitivity and refusal at the rustc native-library / link-driver seam.

use super::*;
use crate::artifact::{closure_key, host, PATH_INPUT};
use eprfs_core::TreePath;

fn stub(dir: &Path, name: &str, body: &str) {
    let file = dir.join(name);
    std::fs::write(
        &file,
        format!("#!/bin/sh\nif [ \"$1\" = --version ]; then exit 0; fi\n{body}\n"),
    )
    .unwrap();
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o755)).unwrap();
    host::fixture::wait_for_stub(&file);
}

fn descriptor(path: &str) -> MethodDescriptor {
    MethodDescriptor {
        name: "mesh-build".into(),
        tools: [
            ("rustc".into(), "host: x86_64-unknown-linux-gnu".into()),
            ("toolchain".into(), "1.96.1".into()),
        ]
        .into(),
        flags: vec![],
        params: None,
        exec: None,
        step: None,
        embedder: None,
        view: vec![TreePath::from_slash_bytes(b"main.rs").unwrap()],
        non_path_inputs: [(PATH_INPUT.into(), path.into())].into(),
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
test -z "$HOME" || exit 4
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
    let mut descriptor = descriptor(&std::env::var("PATH").unwrap());
    let rustc = Command::new("rustc")
        .args(["+1.96.1", "-vV"])
        .output()
        .unwrap();
    assert!(rustc.status.success());
    descriptor
        .tools
        .insert("rustc".into(), String::from_utf8(rustc.stdout).unwrap());
    for name in ["RUSTUP_HOME", "CARGO_HOME", "HOME"] {
        if let Ok(value) = std::env::var(name) {
            descriptor.non_path_inputs.insert(name.into(), value);
        }
    }
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
