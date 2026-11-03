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

fn adopted(
    root: &Path,
    scope: Option<&str>,
    guide: &str,
) -> elohim_rust_style::discipline::Discipline {
    use elohim_rust_style::{discipline, repository::POLICY};
    let (mut closure, objects) = discipline::bundle(guide.as_bytes()).unwrap();
    for (path, bytes) in objects {
        let path = root.join(".epr-meta/rust-style").join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }
    for object in [&mut closure.policy, &mut closure.guide, &mut closure.method] {
        object.path = format!(".epr-meta/rust-style/{}", object.path);
    }
    let mut policy: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join(POLICY)).unwrap()).unwrap();
    policy["discipline"] = serde_json::to_value(&closure).unwrap();
    if let Some(scope) = scope {
        policy["scope_roots"] = serde_json::json!([scope]);
    }
    write(
        root,
        POLICY,
        &serde_json::to_string_pretty(&policy).unwrap(),
    );
    closure
}

#[test]
fn addressed_discipline_is_offline_scoped_and_never_auto_upgrades() {
    use elohim_rust_style::{discipline, repository::POLICY};
    let (first, _) = fixture();
    let (second, _) = fixture();
    let one = adopted(first.path(), Some("crate/src"), "Shared reviewed guide.");
    let two = adopted(second.path(), Some("crate"), "Shared reviewed guide.");
    assert_eq!(one, two, "local scope must not change addressed meaning");
    let first_base = commit(first.path());
    let second_base = commit(second.path());
    let held = fs::read(first.path().join(POLICY)).unwrap();
    let (_, successor) = discipline::bundle(b"Unadopted successor guide.").unwrap();
    for (path, bytes) in successor {
        let path = first.path().join(".epr-meta/rust-style").join(path);
        fs::write(path, bytes).unwrap();
    }
    write(
        first.path(),
        "crate/other.rs",
        "#[allow(unused)] fn outside_scope() {}",
    );
    write(
        second.path(),
        "crate/other.rs",
        "#[allow(unused)] fn inside_scope() {}",
    );
    let first_report = run(first.path(), &["--base", &first_base]);
    assert!(
        first_report.status.success(),
        "{}",
        String::from_utf8_lossy(&first_report.stderr)
    );
    assert_eq!(
        run(second.path(), &["--base", &second_base]).status.code(),
        Some(1)
    );
    assert_eq!(fs::read(first.path().join(POLICY)).unwrap(), held);
    assert_eq!(
        fs::read(first.path().join(&one.policy.path)).unwrap(),
        fs::read(second.path().join(&two.policy.path)).unwrap()
    );
    // A verified closure is required before the edit-time source path as well.
    let good = elohim_rust_style::repository::prospective(
        first.path(),
        "crate/src/lib.rs",
        "pub fn clean() {}",
    );
    assert!(good.unwrap().is_empty());
}

#[test]
fn committed_closure_cannot_borrow_worktree_repair() {
    use elohim_rust_style::repository::POLICY;
    let (dir, base) = fixture();
    let closure = adopted(dir.path(), None, "Reviewed guide.");
    let valid = commit(dir.path());
    let guide = fs::read(dir.path().join(&closure.guide.path)).unwrap();
    write(dir.path(), &closure.guide.path, "corrupted committed guide");
    let corrupt = commit(dir.path());
    fs::write(dir.path().join(&closure.guide.path), &guide).unwrap();
    assert_eq!(
        run(dir.path(), &["--base", &base, "--head", &corrupt])
            .status
            .code(),
        Some(2)
    );
    assert!(run(dir.path(), &["--base", &base, "--head", &valid])
        .status
        .success());
    let mut policy: serde_json::Value =
        serde_json::from_slice(&fs::read(dir.path().join(POLICY)).unwrap()).unwrap();
    policy["discipline"]["guide"]["cid"] = "malformed".into();
    write(dir.path(), POLICY, &serde_json::to_string(&policy).unwrap());
    let bad_policy = commit(dir.path());
    adopted(dir.path(), None, "Reviewed guide.");
    assert_eq!(
        run(dir.path(), &["--base", &base, "--head", &bad_policy])
            .status
            .code(),
        Some(2)
    );
    assert!(run(dir.path(), &["--base", &base]).status.success());
}

#[test]
fn missing_or_malformed_method_refuses_before_source_evaluation() {
    use elohim_rust_style::repository::{self, POLICY};
    let (dir, base) = fixture();
    let closure = adopted(dir.path(), None, "Reviewed guide.");
    let method = fs::read(dir.path().join(&closure.method.path)).unwrap();
    fs::remove_file(dir.path().join(&closure.method.path)).unwrap();
    assert_eq!(run(dir.path(), &["--base", &base]).status.code(), Some(2));
    assert!(repository::prospective(dir.path(), "crate/src/lib.rs", "pub fn clean() {}").is_err());
    fs::write(dir.path().join(&closure.method.path), b"{}").unwrap();
    assert_eq!(run(dir.path(), &["--base", &base]).status.code(), Some(2));
    fs::write(dir.path().join(&closure.method.path), method).unwrap();
    assert!(run(dir.path(), &["--base", &base]).status.success());
    let mut policy: serde_json::Value =
        serde_json::from_slice(&fs::read(dir.path().join(POLICY)).unwrap()).unwrap();
    policy["discipline"]["method"]["path"] = "../outside.json".into();
    write(dir.path(), POLICY, &serde_json::to_string(&policy).unwrap());
    assert_eq!(run(dir.path(), &["--base", &base]).status.code(), Some(2));
}

#[test]
fn additive_addressing_preserves_debt_and_adopted_content_cannot_be_removed() {
    use elohim_rust_style::repository::POLICY;
    let (dir, _) = fixture();
    write(
        dir.path(),
        "crate/src/lib.rs",
        "#[allow(dead_code)] fn debt() {}",
    );
    let base = commit(dir.path());
    adopted(dir.path(), None, "Reviewed guide.");
    assert!(
        run(dir.path(), &["--base", &base]).status.success(),
        "addressing alone is not a source touch"
    );
    write(
        dir.path(),
        "crate/src/lib.rs",
        "#[allow(dead_code)] fn debt() {} // touched",
    );
    assert_eq!(run(dir.path(), &["--base", &base]).status.code(), Some(1));
    write(
        dir.path(),
        "crate/src/lib.rs",
        "#[allow(dead_code)] fn debt() {}",
    );
    let adopted_base = commit(dir.path());
    let mut policy: serde_json::Value =
        serde_json::from_slice(&fs::read(dir.path().join(POLICY)).unwrap()).unwrap();
    policy.as_object_mut().unwrap().remove("discipline");
    write(dir.path(), POLICY, &serde_json::to_string(&policy).unwrap());
    assert_eq!(
        run(dir.path(), &["--base", &adopted_base]).status.code(),
        Some(1)
    );
}

#[test]
fn validly_rehashed_unknown_method_is_not_executable_authority() {
    use elohim_rust_style::repository::POLICY;
    use eprfs_core::BlobCid;
    use eprfs_meta::portable_policy::PortablePolicy;
    let (dir, base) = fixture();
    let closure = adopted(dir.path(), None, "Reviewed guide.");
    let method = b"{\"format\":\"foreign-method\"}";
    fs::write(dir.path().join(&closure.method.path), method).unwrap();
    let cid = BlobCid::compute_raw(method).to_string();
    let mut artifact: PortablePolicy =
        serde_json::from_slice(&fs::read(dir.path().join(&closure.policy.path)).unwrap()).unwrap();
    artifact
        .semantics
        .insert("method".into(), cid.clone().into());
    let policy_bytes = artifact.canonical_bytes().unwrap();
    fs::write(dir.path().join(&closure.policy.path), &policy_bytes).unwrap();
    let mut local: serde_json::Value =
        serde_json::from_slice(&fs::read(dir.path().join(POLICY)).unwrap()).unwrap();
    local["discipline"]["method"]["cid"] = cid.into();
    local["discipline"]["policy"]["cid"] = BlobCid::compute_raw(&policy_bytes).to_string().into();
    write(dir.path(), POLICY, &serde_json::to_string(&local).unwrap());
    let output = run(dir.path(), &["--base", &base]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("installed checker"));
}

#[test]
fn committed_mode_cannot_select_worktree_prospective_input() {
    let (dir, base) = fixture();
    let output = run(
        dir.path(),
        &[
            "--base",
            &base,
            "--head",
            &base,
            "--path",
            "crate/src/lib.rs",
            "--stdin",
        ],
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("working-tree prospective"));
}

#[cfg(unix)]
#[test]
fn committed_policy_symlink_is_not_authored_policy_even_if_target_text_is_json() {
    use elohim_rust_style::repository::POLICY;
    let (dir, base) = fixture();
    adopted(dir.path(), None, "Reviewed guide.");
    let policy = fs::read_to_string(dir.path().join(POLICY)).unwrap();
    fs::remove_file(dir.path().join(POLICY)).unwrap();
    std::os::unix::fs::symlink(policy, dir.path().join(POLICY)).unwrap();
    let linked = commit(dir.path());
    assert_eq!(
        run(dir.path(), &["--base", &base, "--head", &linked])
            .status
            .code(),
        Some(2)
    );
}
