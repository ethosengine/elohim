//! Standalone governance uses native declarations, without borrowing a parent
//! package tree; removing an existing package capability cannot bypass reach.
use std::{fs, path::Path, process::Output};

use elohim_epr_cli::{
    authority::{PACKAGE_ROOT, PACKAGE_VERIFIER},
    process,
};
use tempfile::TempDir;

fn write(root: &Path, path: &str, text: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    fs::write(path, text).expect("write fixture");
}

fn command(root: &Path, program: &str, args: &[&str]) -> Output {
    process::build_command(program, args, root, &[])
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env(
            "GIT_CONFIG_GLOBAL",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
        )
        .env_remove("GIT_CONFIG_COUNT")
        .env_remove("GIT_CONFIG_PARAMETERS")
        .env_remove("GIT_CONFIG")
        .env("EPR_TEST_BIN", env!("CARGO_BIN_EXE_epr"))
        .env("GIT_AUTHOR_NAME", "Fixture")
        .env("GIT_COMMITTER_NAME", "Fixture")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.invalid")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.invalid")
        .output()
        .expect("spawn fixture command")
}

fn git(root: &Path, args: &[&str]) -> String {
    let output = command(root, "git", args);
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("git output")
        .trim()
        .into()
}

fn epr(root: &Path, args: &[&str]) -> Output {
    command(root, env!("CARGO_BIN_EXE_epr"), args)
}

fn hook(root: &Path, dir: &str) {
    write(root, &format!("{dir}/pre-push"), "#!/bin/sh\nexit 0\n");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(
            root.join(dir).join("pre-push"),
            fs::Permissions::from_mode(0o755),
        )
        .expect("executable");
    }
}

fn repo() -> TempDir {
    let dir = tempfile::tempdir().expect("fixture");
    let root = dir.path();
    git(root, &["init", "-q"]);
    write(root, "AGENTS.md", "Native repository governance.\n");
    write(root, "CLAUDE.md", "Native repository governance.\n");
    write(
        root,
        ".epr-meta/manifest.md",
        "---\nepr-meta-version: 1\nid: standalone\nroot: true\n---\n",
    );
    hook(root, ".githooks");
    git(
        root,
        &[
            "add",
            "AGENTS.md",
            "CLAUDE.md",
            ".epr-meta/manifest.md",
            ".githooks/pre-push",
        ],
    );
    git(root, &["commit", "-q", "-m", "standalone floor"]);
    dir
}

fn success(output: Output) -> String {
    assert!(
        output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("epr output")
}

#[test]
fn standalone_setup_doctor_and_agent_doc_reach_need_no_parent_projection() {
    let dir = repo();
    let root = dir.path();
    success(epr(root, &["setup"]));
    assert_eq!(
        git(root, &["config", "--local", "--get", "core.hooksPath"]),
        ".githooks"
    );
    let doctor = success(epr(root, &["--json", "doctor"]));
    assert!(doctor.contains("governance.native-root"));
    assert!(!doctor.contains("governance.package-root"));
    let base = git(root, &["rev-parse", "HEAD"]);
    write(root, "AGENTS.md", "Changed native governance prose.\n");
    git(root, &["add", "AGENTS.md"]);
    git(root, &["commit", "-q", "-m", "native agent doc"]);
    let ready = success(epr(root, &["--json", "ready", "--target", &base]));
    assert!(
        !ready.contains("package.verify"),
        "unmanaged agent docs are not package projections"
    );
}

#[test]
fn configured_hook_paths_are_preserved_and_invalid_paths_are_not_rewritten() {
    for absolute in [false, true] {
        let dir = repo();
        let root = dir.path();
        hook(root, "custom hooks");
        let path = if absolute {
            root.join("custom hooks")
                .to_str()
                .expect("fixture path")
                .to_owned()
        } else {
            "custom hooks".into()
        };
        git(root, &["config", "core.hooksPath", &path]);
        success(epr(root, &["setup"]));
        assert_eq!(git(root, &["config", "--get", "core.hooksPath"]), path);
        for invalid in ["missing-hooks", ""] {
            git(root, &["config", "core.hooksPath", invalid]);
            assert!(!epr(root, &["setup"]).status.success());
            assert_eq!(git(root, &["config", "--get", "core.hooksPath"]), invalid);
        }
    }
}

#[test]
fn native_floor_still_requires_governing_files_and_active_hooks() {
    for missing in [
        "AGENTS.md",
        "CLAUDE.md",
        ".epr-meta/manifest.md",
        ".githooks/pre-push",
    ] {
        let dir = repo();
        let root = dir.path();
        success(epr(root, &["setup"]));
        fs::remove_file(root.join(missing)).expect("remove required fixture");
        assert!(
            !epr(root, &["doctor"]).status.success(),
            "missing {missing}"
        );
        assert!(
            !epr(root, &["ready", "--target", "HEAD"]).status.success(),
            "missing {missing} must block reach"
        );
    }
}

#[test]
fn malformed_or_nonroot_manifests_and_missing_declared_policies_refuse() {
    for manifest in [
        "---\nepr-meta-version: 1\nroot: [\n---\n",
        "---\nepr-meta-version: 2\nroot: true\n---\n",
        "---\nepr-meta-version: 1\nroot: false\n---\n",
        "---\nepr-meta-version: 1\nroot: true\nrules:\n  - id: missing\n    policy: absent-policy@1\n---\n",
    ] {
        let dir = repo();
        success(epr(dir.path(), &["setup"]));
        write(dir.path(), ".epr-meta/manifest.md", manifest);
        assert!(!epr(dir.path(), &["doctor"]).status.success(), "invalid declaration: {manifest}");
        assert!(!epr(dir.path(), &["ready", "--target", "HEAD"]).status.success());
    }
}

#[test]
fn native_deny_rules_still_block_committed_reach() {
    let dir = repo();
    let root = dir.path();
    write(root, ".epr-meta/manifest.md", "---\nepr-meta-version: 1\nid: native\nroot: true\nrules:\n  - id: identified\n    class: deny\n    when: {write: '*.md', new: true}\n    require-frontmatter: [id]\n---\n");
    git(root, &["add", ".epr-meta/manifest.md"]);
    git(root, &["commit", "-q", "-m", "declare rule"]);
    success(epr(root, &["setup"]));
    let base = git(root, &["rev-parse", "HEAD"]);
    write(root, "new.md", "no identity\n");
    git(root, &["add", "new.md"]);
    git(root, &["commit", "-q", "-m", "candidate"]);
    let output = epr(root, &["ready", "--target", &base]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("governance.rule.identified"));
    // Exercise the gate through a real push as well: Git exports GIT_DIR to the
    // hook, and the native process adapter must keep the sender authoritative.
    write(
        root,
        ".githooks/pre-push",
        "#!/bin/sh\nexec \"$EPR_TEST_BIN\" ready --target HEAD~1\n",
    );
    let remote = tempfile::tempdir().expect("remote");
    git(remote.path(), &["init", "--bare", "-q"]);
    let output = command(
        root,
        "git",
        &[
            "push",
            remote.path().to_str().expect("remote path"),
            "HEAD:refs/heads/main",
        ],
    );
    assert!(
        !output.status.success(),
        "native deny gate must reject the real push"
    );
    assert!(
        git(remote.path(), &["for-each-ref"]).is_empty(),
        "rejected push preserves remote"
    );
}

fn declare_packages(root: &Path) {
    write(
        root,
        &format!("{PACKAGE_ROOT}/fixture.json"),
        r#"{"kind":"AgentDocPackage","metadata":{"id":"fixture","master":"package"},"projections":{"codex":{"path":"AGENTS.md"}}}"#,
    );
    write(root, PACKAGE_VERIFIER, "process.exit(0);\n");
    git(root, &["add", PACKAGE_ROOT, PACKAGE_VERIFIER]);
    git(root, &["commit", "-q", "-m", "declare package capability"]);
}

#[test]
fn deleting_package_tree_and_verifier_cannot_downgrade_head_or_base_capability() {
    let dir = repo();
    let root = dir.path();
    declare_packages(root);
    success(epr(root, &["setup"]));
    let base = git(root, &["rev-parse", "HEAD"]);
    fs::remove_dir_all(root.join(PACKAGE_ROOT)).expect("remove packages");
    fs::remove_file(root.join(PACKAGE_VERIFIER)).expect("remove verifier");
    let doctor = epr(root, &["doctor"]);
    assert!(
        !doctor.status.success(),
        "tracked HEAD protects against local deletion"
    );
    git(root, &["add", "--update", PACKAGE_ROOT, PACKAGE_VERIFIER]);
    git(root, &["commit", "-q", "-m", "remove capability"]);
    let ready = epr(root, &["ready", "--target", &base]);
    assert!(
        !ready.status.success(),
        "merge-base protects committed capability removal"
    );
    assert!(String::from_utf8_lossy(&ready.stdout).contains("governance.package-root"));
}

#[test]
fn declared_package_capability_requires_both_components_and_valid_json() {
    for missing in [PACKAGE_ROOT, PACKAGE_VERIFIER] {
        let dir = repo();
        let root = dir.path();
        declare_packages(root);
        success(epr(root, &["setup"]));
        if root.join(missing).is_dir() {
            fs::remove_dir_all(root.join(missing)).expect("remove tree");
        } else {
            fs::remove_file(root.join(missing)).expect("remove script");
        }
        assert!(!epr(root, &["doctor"]).status.success());
    }
    let dir = repo();
    declare_packages(dir.path());
    success(epr(dir.path(), &["setup"]));
    write(
        dir.path(),
        &format!("{PACKAGE_ROOT}/fixture.json"),
        "{ broken",
    );
    assert!(!epr(dir.path(), &["doctor"]).status.success());
}

#[cfg(unix)]
#[test]
fn nonexecutable_gate_is_not_activated() {
    use std::os::unix::fs::PermissionsExt;
    let dir = repo();
    fs::set_permissions(
        dir.path().join(".githooks/pre-push"),
        fs::Permissions::from_mode(0o644),
    )
    .expect("chmod");
    assert!(!epr(dir.path(), &["setup"]).status.success());
    assert!(!command(
        dir.path(),
        "git",
        &["config", "--local", "--get", "core.hooksPath"]
    )
    .status
    .success());
}
