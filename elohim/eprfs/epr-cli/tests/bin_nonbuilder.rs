//! CLI contract for build-role gating at the final local-build fallback.
#![cfg(feature = "peer")]

use elohim_device_runtime::bin_store::{IngestOptions, Registry, Store};
use serde_json::json;
use std::{
    fs,
    os::unix::fs::{symlink, PermissionsExt},
    path::Path,
    process::{Command, Output},
};

struct Fixture {
    temp: tempfile::TempDir,
    registry: Registry,
    head: String,
}

impl Fixture {
    fn new(with_old_release: bool) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path();
        fs::create_dir_all(base.join("repo/elohim/eprfs")).unwrap();
        fs::create_dir_all(base.join("config/elohim/device")).unwrap();
        fs::create_dir_all(base.join("cargo-pool")).unwrap();
        fs::create_dir_all(base.join("fake-bin")).unwrap();
        git(base, &["init", "-q"]);
        fs::write(base.join("repo/README.md"), "old\n").unwrap();
        git(base, &["add", "README.md"]);
        git(base, &["commit", "-qm", "old"]);
        let old = git_text(base, &["rev-parse", "HEAD"]);

        let registry: Registry = serde_json::from_value(json!({
            "version": 1,
            "tools": {
                "epr": {
                    "bins": ["epr"],
                    "candidateEnv": {},
                    "source": {
                        "kind": "repository",
                        "publishedRef": ["refs/remotes/origin/dev"]
                    }
                }
            }
        }))
        .unwrap();
        fs::write(
            base.join("registry.json"),
            serde_json::to_vec(&registry).unwrap(),
        )
        .unwrap();

        if with_old_release {
            ingest_and_make_current(base, &registry, &old, "old", "fixture:old");
        }
        fs::write(base.join("repo/README.md"), "new\n").unwrap();
        git(base, &["add", "README.md"]);
        git(base, &["commit", "-qm", "new"]);
        let head = git_text(base, &["rev-parse", "HEAD"]);
        git(base, &["update-ref", "refs/remotes/origin/dev", &head]);

        let cargo = base.join("fake-bin/cargo");
        fs::write(
            &cargo,
            format!(
                "#!/bin/sh\nprintf 'called\\n' >> '{}'\nexit 7\n",
                base.join("cargo.log").display()
            ),
        )
        .unwrap();
        fs::set_permissions(&cargo, fs::Permissions::from_mode(0o755)).unwrap();

        Self {
            temp,
            registry,
            head,
        }
    }

    fn base(&self) -> &Path {
        self.temp.path()
    }

    fn command(&self, args: &[&str]) -> Output {
        let base = self.base();
        Command::new(env!("CARGO_BIN_EXE_epr"))
            .args(args)
            .current_dir(base.join("repo"))
            .env("XDG_CONFIG_HOME", base.join("config"))
            .env("ELOHIM_STATE_DIR", base.join("state"))
            .env("EPR_BIN_TOOLS", base.join("registry.json"))
            .env("BIN_STORE", base.join("store"))
            .env("EPR_STORE", base.join("store/epr"))
            .env("EPR_PAYLOAD_STORE", base.join("payload"))
            .env("CARGO_TARGET_POOL_ROOT", base.join("cargo-pool"))
            .env("CARGO_TARGET_DIR", base.join("target"))
            .env("CARGO_NET_OFFLINE", "true")
            .env(
                "PATH",
                format!("{}:/usr/bin:/bin", base.join("fake-bin").display()),
            )
            .env_remove("ELOHIM_SESSION_ID")
            .env_remove("CLAUDE_CODE_SESSION_ID")
            .env_remove("CLAUDE_SESSION_ID")
            .output()
            .unwrap()
    }

    fn declare_follow(&self) {
        let output = self.command(&["device", "follow", "bin:epr", "--mode", "apply"]);
        assert!(output.status.success(), "{}", text(&output));
    }

    fn follow(&self) -> Output {
        self.command(&[
            "bin",
            "follow",
            "--tool",
            "epr",
            "--repo",
            self.base().join("repo").to_str().unwrap(),
        ])
    }

    fn role(&self, value: serde_json::Value) {
        fs::write(
            self.base().join("config/elohim/device/build-role.json"),
            serde_json::to_vec(&value).unwrap(),
        )
        .unwrap();
    }

    fn cargo_ran(&self) -> bool {
        self.base().join("cargo.log").exists()
    }
}

fn git(base: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args([
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
        ])
        .args(args)
        .current_dir(base.join("repo"))
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", text(&output));
}

fn git_text(base: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(base.join("repo"))
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", text(&output));
    String::from_utf8(output.stdout).unwrap().trim().into()
}

fn ingest(base: &Path, registry: &Registry, commit: &str, label: &str, gate: &str) -> String {
    let source = base.join(label);
    fs::create_dir(&source).unwrap();
    fs::copy(env!("CARGO_BIN_EXE_epr"), source.join("epr")).unwrap();
    let store = Store::new(base.join("store/epr"), "epr", registry).unwrap();
    store
        .ingest(
            &source,
            &IngestOptions {
                source_commit: Some(commit.into()),
                tree_clean: Some(true),
                gate_run: Some(gate.into()),
                payload_formats: Some("blob-store-v1,directory-v1".into()),
                ..Default::default()
            },
        )
        .unwrap()
        .cid
}

fn ingest_and_make_current(
    base: &Path,
    registry: &Registry,
    commit: &str,
    label: &str,
    gate: &str,
) -> String {
    let cid = ingest(base, registry, commit, label, gate);
    symlink(&cid, base.join("store/epr/current")).unwrap();
    fs::write(
        base.join("store/epr/current.json"),
        json!({"cid": cid, "commit": commit}).to_string(),
    )
    .unwrap();
    cid
}

fn text(output: &Output) -> String {
    format!(
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn declared_non_builder_with_a_usable_sibling_release_follows_without_cargo() {
    let fixture = Fixture::new(true);
    let candidate = ingest(
        fixture.base(),
        &fixture.registry,
        &fixture.head,
        "sibling-candidate",
        "sibling:did:key:z6MkBuilder",
    );
    fixture.role(json!({"version": 1, "role": "carrier"}));
    fixture.declare_follow();

    let output = fixture.follow();

    assert!(output.status.success(), "{}", text(&output));
    assert!(String::from_utf8_lossy(&output.stdout).contains(&candidate));
    assert!(!fixture.cargo_ran(), "a held sibling release is used first");
}

#[test]
fn missing_build_role_builds_with_one_warning() {
    let fixture = Fixture::new(true);
    fixture.declare_follow();

    let output = fixture.follow();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(output.status.code(), Some(2), "{}", text(&output));
    assert_eq!(stderr.matches("warning: build-role").count(), 1, "{stderr}");
    assert!(
        stderr.contains("build-role missing; building as before"),
        "{stderr}"
    );
    assert!(
        fixture.cargo_ran(),
        "unknown role preserves the build fallback"
    );
}

#[test]
fn expired_builder_builds_with_one_warning() {
    let fixture = Fixture::new(true);
    fixture.role(json!({
        "version": 1,
        "role": "builder",
        "until": "2000-01-01T00:00:00Z"
    }));
    fixture.declare_follow();

    let output = fixture.follow();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(output.status.code(), Some(2), "{}", text(&output));
    assert_eq!(stderr.matches("warning: build-role").count(), 1, "{stderr}");
    assert!(stderr.contains("builder expired 2000-01-01T00:00:00Z"));
    assert!(
        fixture.cargo_ran(),
        "expired builder preserves the build fallback"
    );
}

#[test]
fn declared_non_builder_without_a_release_refuses_with_attempts_cure_and_exit_three() {
    let fixture = Fixture::new(true);
    fixture.role(json!({"version": 1, "role": "neither"}));
    fixture.declare_follow();

    let output = fixture.follow();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(output.status.code(), Some(3), "{}", text(&output));
    for expected in [
        "verified current unavailable",
        "no verified stored candidate",
        "no local closure supplied",
        "sibling attempts:",
        "epr device build-role builder --until <RFC3339>",
        "epr bin follow --tool epr",
    ] {
        assert!(stderr.contains(expected), "missing {expected:?}: {stderr}");
    }
    assert!(
        !fixture.cargo_ran(),
        "a declared non-builder never invokes cargo"
    );
}

#[test]
fn bootstrap_refuses_for_a_declared_non_builder_without_calling_cargo() {
    let fixture = Fixture::new(false);
    fixture.role(json!({"version": 1, "role": "neither"}));

    let output = fixture.command(&[
        "bin",
        "bootstrap",
        "--tool",
        "epr",
        "--repo",
        fixture.base().join("repo").to_str().unwrap(),
    ]);

    assert_eq!(output.status.code(), Some(3), "{}", text(&output));
    assert!(String::from_utf8_lossy(&output.stderr).contains("REFUSED not-a-builder"));
    assert!(!fixture.cargo_ran(), "bootstrap must honor the role gate");
}
