//! Exercise actual Git objects and the CLI, not only in-memory comparisons.
use std::{fs, path::Path, process::Command};

fn git(root: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim().into()
}
fn write(root: &Path, path: &str, text: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}
fn commit(root: &Path) -> String {
    git(root, &["add", "crate", ".epr-meta"]);
    git(
        root,
        &[
            "-c",
            "user.name=Style fixture",
            "-c",
            "user.email=style@example.invalid",
            "commit",
            "-qm",
            "fixture",
        ],
    );
    git(root, &["rev-parse", "HEAD"])
}
fn fixture() -> (tempfile::TempDir, String) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    git(root, &["init", "-q"]);
    write(
        root,
        "crate/Cargo.toml",
        "[package]\nname='fixture'\nversion='0.1.0'\n",
    );
    write(root, "crate/src/lib.rs", "pub fn old() {}\n");
    write(
        root,
        ".epr-meta/rust-style-policy.json",
        r#"{"version":1,"rules":["RS-GOV-04"],"external_roots":[]}"#,
    );
    let base = commit(root);
    (dir, base)
}
fn run(root: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_epr-rust-style"))
        .arg("--root")
        .arg(root)
        .arg("--json")
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn committed_violation_cannot_be_hidden_by_a_dirty_fix() {
    let (dir, base) = fixture();
    let root = dir.path();
    write(root, "crate/src/lib.rs", "#[allow(dead_code)] fn new() {}");
    let bad = commit(root);
    write(root, "crate/src/lib.rs", "pub fn new() {}");
    assert_eq!(
        run(root, &["--base", &base, "--head", &bad]).status.code(),
        Some(1)
    );
    assert!(run(root, &["--base", &base]).status.success());
    // A second pushed ref is independently measured, not hidden behind HEAD.
    assert!(run(root, &["--base", &base, "--head", &base])
        .status
        .success());
}

#[test]
fn untracked_sources_are_checked_and_missing_history_is_unmeasured() {
    let (dir, base) = fixture();
    write(
        dir.path(),
        "crate/src/new.rs",
        "#[allow(unused)] fn new() {}",
    );
    assert_eq!(run(dir.path(), &["--base", &base]).status.code(), Some(1));
    assert_eq!(
        run(dir.path(), &["--base", "refs/heads/missing"])
            .status
            .code(),
        Some(2)
    );
}

#[test]
fn repeat_scans_do_not_demand_repeated_paydown() {
    let (dir, _) = fixture();
    let root = dir.path();
    write(
        root,
        "crate/src/lib.rs",
        "#[allow(dead_code)] fn one() {} #[allow(dead_code)] fn two() {}",
    );
    let base = commit(root);
    write(
        root,
        "crate/src/lib.rs",
        "pub fn one() {} #[allow(dead_code)] fn two() {}",
    );
    assert!(run(root, &["--base", &base]).status.success());
    assert!(run(root, &["--base", &base]).status.success());
    let fixed = commit(root);
    write(
        root,
        "crate/src/lib.rs",
        "pub fn one() {} #[allow(dead_code)] fn two() {} // another change",
    );
    assert_eq!(run(root, &["--base", &fixed]).status.code(), Some(1));
}

#[test]
fn strict_module_requires_zero_while_siblings_can_retain_debt() {
    let (dir, _) = fixture();
    let root = dir.path();
    write(
        root,
        "crate/src/legacy.rs",
        "#[allow(dead_code)] fn legacy() {}",
    );
    let base = commit(root);
    write(root, "crate/src/refactor/.epr-meta", "---\nepr-meta-version: 1\nid: strict-module\ncovers: subtree\nrules:\n  - id: rust-style-strict\n    policy: rust-style-strict@1\n---\n");
    write(
        root,
        "crate/src/refactor/mod.rs",
        "#[allow(dead_code)] fn bad() {}",
    );
    let output = run(root, &["--base", &base]);
    assert_eq!(output.status.code(), Some(1));
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["crates"]["crate"]["strict_files"], 1);
    write(root, "crate/src/refactor/mod.rs", "pub fn clean() {}");
    // The owning crate still owes its ordinary one-finding paydown.
    assert_eq!(run(root, &["--base", &base]).status.code(), Some(1));
    write(root, "crate/src/legacy.rs", "pub fn legacy() {}");
    assert!(run(root, &["--base", &base]).status.success());
}

#[test]
fn compiler_refuses_stale_expectations_with_warning_denial() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("lib.rs");
    fs::write(&path, "#[expect(dead_code, reason = \"fixture proving expectation freshness\")] pub fn public() {}").unwrap();
    let output = Command::new("rustc")
        .args(["--crate-type", "lib", "-D", "warnings", "--out-dir"])
        .arg(dir.path())
        .arg(&path)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unfulfilled_lint_expectations"));
}
