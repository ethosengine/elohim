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
        "epr":{"bins":["epr"],"candidateEnv":{"epr":["EPR_BIN"]}},
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
            "peer daemon"
        ]
    );
    assert_eq!(checks[2]["state"], "refused");
    assert_eq!(checks[4]["state"], "degraded");
    assert_eq!(checks[5]["state"], "degraded");
    assert_eq!(checks[6]["state"], "degraded");
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
    let current = eprfs_core::BlobCid::compute_raw(b"current").to_string();
    let older = eprfs_core::BlobCid::compute_raw(b"older").to_string();
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
    let (_, rows) = checks(base);
    let peer = rows.iter().find(|r| r["name"] == "peer daemon").unwrap();
    assert_eq!(peer["state"], "degraded");
    assert!(peer["cure"].as_str().unwrap().contains("restart"));
    write(&current, start);
    assert_eq!(checks(base).1.last().unwrap()["state"], "ok");
    write(&older, start + 1);
    assert_eq!(checks(base).1.last().unwrap()["state"], "ok");
}
