//! Native arrival and trust acceptance: no live key, roster, launcher, or other worktree is touched.
use serde_json::{json, Value};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

fn epr(base: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_epr"))
        .args(args)
        .current_dir(base.join("repo"))
        .env("HOME", base)
        .env("XDG_CONFIG_HOME", base.join("config"))
        .env("ELOHIM_DEVICE_KEY_FILE", base.join("device/seed"))
        .env("EPR_BIN_TOOLS", base.join("registry.json"))
        .env("BIN_STORE", base.join("store"))
        .env("EPR_UNDER_TEST", "1")
        .env("EPR_PROC_ROOT", base.join("proc"))
        .env_remove("ELOHIM_SESSION_OWNER_PID")
        .env_remove("EPR_SESSION_LAZY")
        .env("EPR_STORE", base.join("store/epr"))
        .env("BRIT_STORE", base.join("store/brit"))
        .env("EPR_BIN", env!("CARGO_BIN_EXE_epr"))
        .env("BRIT_BIN", env!("CARGO_BIN_EXE_epr"))
        .env("CLAUDE_CONFIG_DIR", base.join("berth-config"))
        .env("BIN_STORE_SKIP_TOOLS", "epr,brit")
        .env("ELOHIM_SESSION_ID", "arrival-test")
        .env("PATH", "/usr/bin:/bin")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_COUNT", "0")
        // `git -c safe.directory=*` reaches children through this; the fixture owns its trust.
        .env_remove("GIT_CONFIG_PARAMETERS")
        .env("GIT_TEST_ASSUME_DIFFERENT_OWNER", "1")
        .output()
        .unwrap()
}
fn fixture() -> tempfile::TempDir {
    let temp = tempfile::tempdir().unwrap();
    fs::create_dir(temp.path().join("repo")).unwrap();
    assert!(Command::new("git")
        .args(["init", "-q"])
        .current_dir(temp.path().join("repo"))
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .status()
        .unwrap()
        .success());
    let registry = json!({"version":1,"tools":{
        "epr":{"bins":["epr"],"candidateEnv":{"epr":["EPR_BIN"]},
            "source":{"publishedRef":["refs/heads/dev","refs/remotes/origin/dev"]}},
        "brit":{"bins":["brit"],"candidateEnv":{"brit":["BRIT_BIN"]}}
    }});
    fs::write(temp.path().join("registry.json"), registry.to_string()).unwrap();
    temp
}
fn checks(base: &Path) -> (Output, Vec<Value>) {
    let output = epr(base, &["device", "status", "--json"]);
    let checks = serde_json::from_slice(&output.stdout).unwrap();
    (output, checks)
}
#[test]
fn declared_trust_cures_ownership_for_exact_checkout_only() {
    let temp = fixture();
    let base = temp.path();
    let (_, before) = checks(base);
    assert_eq!(before[7]["state"], "refused");
    let result = epr(base, &["device", "trust", "."]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(epr(base, &["device", "trust", "."]).status.success());
    let trusted = fs::read_to_string(base.join("device/trusted-checkouts")).unwrap();
    assert_eq!(
        trusted.lines().collect::<Vec<_>>(),
        [base.join("repo").canonicalize().unwrap().to_str().unwrap()]
    );
    assert!(
        !base.join("device/seed").exists(),
        "trust must never mint a key"
    );
    let (_, after) = checks(base);
    assert_eq!(after[7]["state"], "ok");
    fs::create_dir(base.join("repo/nested")).unwrap();
    assert!(Command::new("git")
        .args(["init", "-q"])
        .current_dir(base.join("repo/nested"))
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .status()
        .unwrap()
        .success());
    let output = epr(base, &["device", "status", "--json", "--root", "nested"]);
    let nested: Vec<Value> = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        nested[7]["state"], "refused",
        "parent trust must not confer nested trust"
    );
}
#[test]
fn status_keeps_dependency_order_and_does_not_mint_a_key() {
    let temp = fixture();
    let (output, checks) = checks(temp.path());
    assert_eq!(output.status.code(), Some(1));
    let names: Vec<_> = checks.iter().map(|c| c["name"].as_str().unwrap()).collect();
    assert_eq!(
        names,
        [
            "epr binary",
            "brit binary",
            "device key",
            "roster bound",
            "embed model",
            "fold attested",
            "berth moored",
            "repository trusted",
            "peer daemon",
            "sibling exchange",
            "device supervisor",
            "rust toolchain",
            "git hooks",
            "brit trust",
            "object packs",
            "device layout",
            "next step"
        ]
    );
    // No serve lock is held: siblings read this device as silent.
    assert_eq!(checks[8]["state"], "degraded");
    assert!(checks[8]["detail"].as_str().unwrap().contains("silent"));
    // No device stands here, so there is no sibling to exchange with; never a refusal.
    assert_eq!(checks[9]["state"], "degraded");
    assert!(checks[9]["detail"]
        .as_str()
        .unwrap()
        .contains("stands on no roster"));
    // The fixture pins no toolchain and declares no governed hooks.
    assert_eq!(checks[11]["state"], "ok");
    assert_eq!(checks[12]["state"], "ok");
    assert_eq!(checks[2]["state"], "refused");
    assert_eq!(checks[4]["state"], "degraded");
    assert_eq!(checks[5]["state"], "degraded");
    assert_eq!(checks[6]["state"], "degraded");
    // The committed base declares no process: nothing to supervise is not a lack.
    assert_eq!(checks[10]["state"], "ok");
    assert!(checks[10]["cure"].is_null());
    // The last line repeats the first owed ceremony cure, and never refuses.
    let next = checks.last().unwrap();
    assert_ne!(next["state"], "refused");
    let owed = [
        "epr binary",
        "device key",
        "roster bound",
        "peer daemon",
        "sibling exchange",
    ]
    .iter()
    .filter_map(|name| checks.iter().find(|c| c["name"] == *name))
    .find(|c| c["state"] != "ok" && !c["cure"].is_null())
    .unwrap();
    assert_eq!(next["detail"], owed["name"]);
    assert_eq!(next["cure"], owed["cure"]);
    assert!(!temp.path().join("device/seed").exists());
}
#[test]
fn arrive_bad_registry_fails_open_but_strict_refuses() {
    let temp = fixture();
    fs::write(temp.path().join("registry.json"), "invalid").unwrap();
    let open = epr(temp.path(), &["device", "arrive"]);
    assert!(open.status.success());
    assert!(String::from_utf8_lossy(&open.stderr).contains("continuing"));
    let strict = epr(temp.path(), &["device", "arrive", "--strict"]);
    assert_eq!(strict.status.code(), Some(1));
}
#[test]
fn detached_arrival_closes_parent_pipes_and_returns_summary() {
    let temp = fixture();
    let start = std::time::Instant::now();
    let result = epr(temp.path(), &["device", "arrive"]);
    assert!(result.status.success());
    assert!(start.elapsed() < std::time::Duration::from_secs(10));
    assert!(String::from_utf8_lossy(&result.stdout).contains("repository trusted:"));
    // Only skipped fixture tools exist: no cargo process or live tool installation can start.
}

#[test]
fn peer_daemon_status_compares_live_lease_with_current() {
    use std::os::unix::{fs::symlink, fs::PermissionsExt};
    let temp = fixture();
    let base = temp.path();
    let store = base.join("store/epr");
    let current = eprfs_core::BlobCid::compute_bytes(b"current").to_string();
    let older = eprfs_core::BlobCid::compute_bytes(b"older").to_string();
    fs::create_dir_all(store.join(&current)).unwrap();
    fs::create_dir_all(store.join("sessions")).unwrap();
    let executable = store.join(&current).join("epr");
    fs::write(&executable, "fixture").unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
    symlink(&current, store.join("current")).unwrap();
    let pid = 42;
    let start = 77;
    fs::create_dir_all(base.join("proc/42")).unwrap();
    let mut fields = vec!["0".to_owned(); 20];
    fields[0] = "S".into();
    fields[1] = "1".into();
    fields[19] = start.to_string();
    fs::write(
        base.join("proc/42/stat"),
        format!("42 (epr) {}", fields.join(" ")),
    )
    .unwrap();
    let lease = store.join("sessions/daemon-fixture");
    let write = |cid: &str, start: u64| {
        fs::write(
            &lease,
            json!({"cid":cid,"runningCid":cid,"session":"daemon-fixture",
            "pinnedAt":"2026-10-24T00:00:00Z","ownerPid":pid,"ownerStartTime":start.to_string()})
            .to_string(),
        )
        .unwrap();
    };
    write(&older, start);
    // A live daemon holds its serve lock; liveness is the lock, never a recorded pid.
    let _serving = elohim_epr_cli::peer::ServeLock::acquire(&base.join("config/epr/peer")).unwrap();
    let (_, rows) = checks(base);
    let peer = rows.iter().find(|r| r["name"] == "peer daemon").unwrap();
    assert_eq!(peer["state"], "degraded");
    assert!(peer["cure"].as_str().unwrap().contains("restart"));
    let peer_state = |base: &Path| {
        checks(base)
            .1
            .into_iter()
            .find(|r| r["name"] == "peer daemon")
            .unwrap()["state"]
            .clone()
    };
    write(&current, start);
    assert_eq!(peer_state(base), "ok");
    write(&older, start + 1);
    assert_eq!(peer_state(base), "ok");
}

/// Plain git in the fixture repository, as its owner (no assumed foreign ownership).
fn fixture_git(base: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args([
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
        ])
        .args(args)
        .current_dir(base.join("repo"))
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
/// The fixture's device becomes its person's first device: the chain root of a roster here.
fn first_device(base: &Path) {
    assert!(epr(base, &["device", "trust", "."]).status.success());
    fs::write(base.join("repo/README.md"), "# fixture\n").unwrap();
    fixture_git(base, &["add", "README.md"]);
    fixture_git(base, &["commit", "-q", "-m", "before the roster"]);
    let witnessed = epr(
        base,
        &[
            "actor",
            "witness",
            "--subject",
            "human:matthew",
            "--as",
            "agent:reader@fixture",
            "--session",
            "arrival-test",
            "--basis",
            "the person is present at the fixture",
            "--json",
        ],
    );
    assert!(
        witnessed.status.success(),
        "{}",
        String::from_utf8_lossy(&witnessed.stderr)
    );
}
fn line(base: &Path, name: &str) -> Value {
    checks(base)
        .1
        .into_iter()
        .find(|c| c["name"] == name)
        .unwrap()
}

/// The laptop's case: bound in a local commit and never published reads as local only, never
/// as arrived; publication is read from the last fetch of the declared remote-tracking ref.
#[test]
fn roster_bound_names_whether_the_shared_branch_carries_the_row() {
    let temp = fixture();
    let base = temp.path();
    first_device(base);
    let never = line(base, "roster bound");
    assert_eq!(never["state"], "unknown", "{never}");
    assert!(never["detail"]
        .as_str()
        .unwrap()
        .contains("origin/dev has never been fetched"));
    assert_eq!(never["cure"], "fetch the integration branch");
    // The shared branch as last fetched: the commit before the roster existed.
    fixture_git(base, &["update-ref", "refs/remotes/origin/dev", "HEAD"]);
    let uncommitted = line(base, "roster bound");
    assert_eq!(uncommitted["state"], "degraded", "{uncommitted}");
    assert!(uncommitted["detail"]
        .as_str()
        .unwrap()
        .contains("matthew's roster row for this device is local only (in the working tree)"));
    fixture_git(base, &["add", ".eprfs/status/participants/matthew.jsonl"]);
    fixture_git(base, &["commit", "-q", "-m", "matthew's roster"]);
    let committed = line(base, "roster bound");
    assert_eq!(committed["state"], "degraded");
    let detail = committed["detail"].as_str().unwrap();
    assert!(detail.contains("(in a local commit), not on origin/dev (as of the last fetch)"));
    assert!(committed["cure"]
        .as_str()
        .unwrap()
        .starts_with("publish the roster rows through the normal gate"));
    let (output, rows) = checks(base);
    assert_eq!(rows.last().unwrap()["name"], "next step");
    // Local only is degraded, so the exit stays the refusals' to set.
    assert!(rows.iter().all(|r| r["state"] != "refused") == output.status.success());
    fixture_git(base, &["update-ref", "refs/remotes/origin/dev", "HEAD"]);
    let published = line(base, "roster bound");
    assert_eq!(published["state"], "ok", "{published}");
    assert!(published["detail"]
        .as_str()
        .unwrap()
        .ends_with("published on origin/dev (as of the last fetch)"));
    // One device: no sibling, said plainly, and no exchange is claimed.
    let sibling = line(base, "sibling exchange");
    assert_eq!(sibling["state"], "degraded");
    assert_eq!(
        sibling["detail"],
        "this device has no sibling in the roster"
    );
}

/// Every file under the fixture, with its bytes: a status run must leave it identical.
fn tree(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut files = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(next) = stack.pop() {
        for entry in fs::read_dir(&next).unwrap() {
            let path = entry.unwrap().path();
            let meta = fs::symlink_metadata(&path).unwrap();
            let name = path.strip_prefix(dir).unwrap().display().to_string();
            if meta.is_dir() {
                files.push((format!("{name}/"), Vec::new()));
                stack.push(path);
            } else {
                files.push((name, fs::read(&path).unwrap_or_default()));
            }
        }
    }
    files.sort();
    files
}

#[test]
fn status_is_a_read_that_leaves_the_fixture_byte_identical() {
    let temp = fixture();
    let base = temp.path();
    first_device(base);
    fixture_git(base, &["update-ref", "refs/remotes/origin/dev", "HEAD"]);
    let before = tree(base);
    let (_, rows) = checks(base);
    assert_eq!(rows.last().unwrap()["name"], "next step");
    let text = epr(base, &["device", "status"]);
    assert!(!text.stdout.is_empty());
    let after = tree(base);
    let changed: Vec<_> = before
        .iter()
        .zip(after.iter())
        .filter(|(b, a)| b != a)
        .map(|(b, _)| b.0.clone())
        .collect();
    assert_eq!(
        before.len(),
        after.len(),
        "a status run created or removed a file"
    );
    assert!(changed.is_empty(), "a status run changed {changed:?}");
}
