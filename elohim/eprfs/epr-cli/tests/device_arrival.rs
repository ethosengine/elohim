//! Native arrival and trust acceptance: no live key, roster, launcher, or other worktree is touched.
use serde_json::{json, Value};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
    time::{Duration, Instant},
};

fn epr(base: &Path, args: &[&str]) -> Output {
    epr_with_env(base, args, &[])
}

fn epr_with_env(base: &Path, args: &[&str], extra_env: &[(&str, &str)]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_epr"));
    command
        .args(args)
        .current_dir(base.join("repo"))
        .env("HOME", base)
        .env("XDG_CONFIG_HOME", base.join("config"))
        .env("ELOHIM_DEVICE_KEY_FILE", base.join("device/seed"))
        .env("EPR_BIN_TOOLS", base.join("registry.json"))
        .env("BIN_STORE", base.join("store"))
        .env("CARGO_TARGET_POOL_ROOT", base.join("cargo-pool"))
        .env("EPR_UNDER_TEST", "1")
        .env_remove("EPR_WOULD_TIMEOUT_MS")
        .env("EPR_PROC_ROOT", base.join("proc"))
        .env_remove("ELOHIM_SESSION_OWNER_PID")
        .env_remove("EPR_SESSION_LAZY")
        .env("EPR_STORE", base.join("store/epr"))
        .env("BRIT_STORE", base.join("store/brit"))
        .env("EPR_BIN", env!("CARGO_BIN_EXE_epr"))
        .env("BRIT_BIN", env!("CARGO_BIN_EXE_epr"))
        .env("CLAUDE_CONFIG_DIR", base.join("berth-config"))
        // An arrival remembers its sysroot here, never in this host's real state directory.
        .env("ELOHIM_STATE_DIR", base.join("state"))
        .env("BIN_STORE_SKIP_TOOLS", "epr,brit")
        .env("ELOHIM_SESSION_ID", "arrival-test")
        .env("PATH", "/usr/local/bin:/usr/bin:/bin")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_COUNT", "0")
        // `git -c safe.directory=*` reaches children through this; the fixture owns its trust.
        .env_remove("GIT_CONFIG_PARAMETERS")
        .env("GIT_TEST_ASSUME_DIFFERENT_OWNER", "1");
    for (name, value) in extra_env {
        command.env(name, value);
    }
    command.output().unwrap()
}

const PUSH_CHANGED_FILES: &str =
    include_str!("../../../../genesis/orchestrator/push-changed-files.sh");

fn install_push_changed_files(base: &Path) {
    let orchestrator = base.join("repo/genesis/orchestrator");
    fs::create_dir_all(&orchestrator).unwrap();
    fs::write(
        orchestrator.join("push-changed-files.sh"),
        PUSH_CHANGED_FILES,
    )
    .unwrap();
}
fn fixture() -> tempfile::TempDir {
    let temp = tempfile::tempdir().unwrap();
    fs::create_dir(temp.path().join("repo")).unwrap();
    fs::create_dir(temp.path().join("cargo-pool")).unwrap();
    fs::create_dir(temp.path().join("proc")).unwrap();
    fs::write(
        temp.path().join("proc/cpuinfo"),
        "processor: 0\nprocessor: 1\n",
    )
    .unwrap();
    fs::write(temp.path().join("proc/meminfo"), "MemTotal: 4096 kB\n").unwrap();
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
            "peer rules",
            "sibling exchange",
            "device supervisor",
            "rust toolchain",
            "sysroot",
            "git hooks",
            "brit trust",
            "object packs",
            "device layout",
            "device capacity",
            "next step"
        ]
    );
    // A status read runs no probe: with nothing recorded, the sysroot is not yet observed.
    assert_eq!(checks[13]["state"], "unknown");
    assert!(checks[13]["detail"]
        .as_str()
        .unwrap()
        .starts_with("not yet observed; an arrival records it"));
    assert_eq!(checks[13]["cure"], "epr device arrive");
    // No serve lock is held: siblings read this device as silent.
    assert_eq!(checks[8]["state"], "degraded");
    assert!(checks[8]["detail"].as_str().unwrap().contains("silent"));
    assert_eq!(checks[9]["state"], "unknown");
    // No device stands here, so there is no sibling to exchange with; never a refusal.
    assert_eq!(checks[10]["state"], "degraded");
    assert!(checks[10]["detail"]
        .as_str()
        .unwrap()
        .contains("stands on no roster"));
    // The fixture pins no toolchain and declares no governed hooks.
    assert_eq!(checks[12]["state"], "ok");
    assert_eq!(checks[14]["state"], "ok");
    assert_eq!(checks[2]["state"], "refused");
    assert_eq!(checks[4]["state"], "degraded");
    assert_eq!(checks[5]["state"], "degraded");
    assert_eq!(checks[6]["state"], "degraded");
    // The committed base declares no process: nothing to supervise is not a lack.
    assert_eq!(checks[11]["state"], "ok");
    assert!(checks[11]["cure"].is_null());
    let capacity = &checks[18];
    assert_eq!(capacity["state"], "ok");
    assert_eq!(capacity["capacity"]["cargoPoolPresent"], true);
    assert!(capacity["capacity"]["cargoPoolFreeBytes"].is_u64());
    assert!(capacity["capacity"]["cores"].is_u64());
    assert!(capacity["capacity"]["memoryBytes"].is_u64());
    assert!(capacity["detail"]
        .as_str()
        .unwrap()
        .contains("pool present"));
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
fn only_an_explicit_non_builder_makes_a_missing_pin_ok() {
    let temp = fixture();
    let base = temp.path();
    first_device(base);
    let policy = base.join("repo/genesis/agentic/pool-policy.json");
    fs::create_dir_all(policy.parent().unwrap()).unwrap();
    fs::write(
        policy,
        r#"{"cargo_env_overrides":{"*":{"RUSTUP_TOOLCHAIN":"99.0.0"}}}"#,
    )
    .unwrap();
    let rust = |base: &Path| {
        checks(base)
            .1
            .into_iter()
            .find(|check| check["name"] == "rust toolchain")
            .unwrap()
    };
    assert_eq!(rust(base)["state"], "degraded", "undeclared stays degraded");
    let builder = epr(
        base,
        &[
            "device",
            "build-role",
            "builder",
            "--until",
            "2099-01-01T00:00:00Z",
        ],
    );
    assert!(
        builder.status.success(),
        "{}",
        String::from_utf8_lossy(&builder.stderr)
    );
    assert_eq!(
        rust(base)["state"],
        "degraded",
        "builder still needs the pin"
    );
    let neither = epr(base, &["device", "build-role", "neither"]);
    assert!(
        neither.status.success(),
        "{}",
        String::from_utf8_lossy(&neither.stderr)
    );
    let rust = rust(base);
    assert_eq!(rust["state"], "ok");
    assert!(rust["detail"].as_str().unwrap().contains("does not build"));
    assert!(base.join("device/build-role.json").is_file());
    assert!(
        !base.join("device/manifest/head").exists(),
        "a build role never enters the runtime manifest chain"
    );
}

#[test]
fn would_uses_the_pre_push_range_and_reports_selection_only() {
    let temp = fixture();
    let base = temp.path();
    first_device(base);
    let orchestrator = base.join("repo/genesis/orchestrator");
    install_push_changed_files(base);
    fs::write(
        orchestrator.join("gate-runner.mjs"),
        "process.stdin.resume(); process.stdin.on('end', () => { console.log('alpha'); console.log('beta'); });\n",
    )
    .unwrap();
    fs::write(base.join("repo/changed.txt"), "selected\n").unwrap();
    fixture_git(
        base,
        &[
            "add",
            "genesis/orchestrator/gate-runner.mjs",
            "genesis/orchestrator/push-changed-files.sh",
            "changed.txt",
        ],
    );
    fixture_git(base, &["commit", "-q", "-m", "fixture gate selection"]);
    let commit = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(base.join("repo"))
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap();
    assert!(commit.status.success());
    let commit = String::from_utf8(commit.stdout).unwrap();
    let report = epr(
        base,
        &[
            "device",
            "status",
            "--would",
            commit.trim(),
            "--target",
            "HEAD~1",
        ],
    );
    let text = String::from_utf8(report.stdout).unwrap();
    assert!(text.contains("FORECAST"), "{text}");
    assert!(
        text.contains("selection only; reuse, attestation and hand-over can change what runs"),
        "{text}"
    );
    assert!(text.contains("selected gates:\n  alpha\n  beta"), "{text}");
    assert!(
        !text.contains("run here") && !text.contains("leave as miss"),
        "the runner provides selection, not role-based execution placement"
    );

    assert!(epr(base, &["device", "build-role", "neither"])
        .status
        .success());
    let report = epr(
        base,
        &[
            "device",
            "status",
            "--would",
            commit.trim(),
            "--target",
            "HEAD~1",
        ],
    );
    let text = String::from_utf8(report.stdout).unwrap();
    assert!(text.contains("selected gates:\n  alpha\n  beta"), "{text}");
}

#[test]
fn build_role_refuses_an_arbitrary_root_and_reads_expiry_at_status_time() {
    let temp = fixture();
    let base = temp.path();
    first_device(base);
    let foreign = tempfile::tempdir().unwrap();
    let refused = epr(
        base,
        &[
            "device",
            "build-role",
            "neither",
            "--root",
            foreign.path().to_str().unwrap(),
        ],
    );
    assert!(!refused.status.success());
    assert!(!foreign.path().join("build-role.json").exists());

    let expired = epr(
        base,
        &[
            "device",
            "build-role",
            "builder",
            "--until",
            "2000-01-01T00:00:00Z",
        ],
    );
    assert!(expired.status.success());
    let capacity = line(base, "device capacity");
    assert!(capacity["detail"]
        .as_str()
        .unwrap()
        .contains("role builder, expired 2000-01-01T00:00:00Z: declare again"));
}

#[test]
fn would_timeout_is_reported_without_changing_status_exit() {
    let temp = fixture();
    let base = temp.path();
    first_device(base);
    let orchestrator = base.join("repo/genesis/orchestrator");
    install_push_changed_files(base);
    fs::write(
        orchestrator.join("gate-runner.mjs"),
        "setTimeout(() => console.log('too-late'), 10000);\n",
    )
    .unwrap();
    fs::write(base.join("repo/changed.txt"), "selected\n").unwrap();
    fixture_git(
        base,
        &[
            "add",
            "genesis/orchestrator/gate-runner.mjs",
            "genesis/orchestrator/push-changed-files.sh",
            "changed.txt",
        ],
    );
    fixture_git(base, &["commit", "-q", "-m", "fixture slow selection"]);
    let ordinary = epr(base, &["device", "status"]);
    let forecast = epr_with_env(
        base,
        &["device", "status", "--would", "HEAD", "--target", "HEAD~1"],
        &[("EPR_WOULD_TIMEOUT_MS", "100")],
    );
    assert_eq!(forecast.status.code(), ordinary.status.code());
    assert!(String::from_utf8_lossy(&forecast.stdout)
        .contains("timed out after 10s while running gate selection"));
}

#[test]
fn would_input_stall_is_reported_within_deadline_without_changing_status_exit() {
    let temp = fixture();
    let base = temp.path();
    first_device(base);
    let orchestrator = base.join("repo/genesis/orchestrator");
    install_push_changed_files(base);
    fs::write(
        orchestrator.join("gate-runner.mjs"),
        "console.log('too-late');\n",
    )
    .unwrap();
    fs::write(base.join("repo/changed.txt"), "selected\n").unwrap();
    fixture_git(
        base,
        &[
            "add",
            "genesis/orchestrator/gate-runner.mjs",
            "genesis/orchestrator/push-changed-files.sh",
            "changed.txt",
        ],
    );
    fixture_git(
        base,
        &["commit", "-q", "-m", "fixture stalled selection input"],
    );
    let ordinary_started = Instant::now();
    let ordinary = epr(base, &["device", "status"]);
    let ordinary_elapsed = ordinary_started.elapsed();
    let started = Instant::now();
    let forecast = epr_with_env(
        base,
        &["device", "status", "--would", "HEAD", "--target", "HEAD~1"],
        &[
            ("EPR_WOULD_TIMEOUT_MS", "100"),
            ("EPR_WOULD_STALL_INPUT", "1"),
        ],
    );
    let forecast_elapsed = started.elapsed();
    let forecast_overhead = forecast_elapsed.saturating_sub(ordinary_elapsed);
    assert!(
        forecast_overhead < Duration::from_secs(2),
        "forecast added {forecast_overhead:?}: ordinary {ordinary_elapsed:?}, forecast \
         {forecast_elapsed:?}"
    );
    assert_eq!(forecast.status.code(), ordinary.status.code());
    assert!(String::from_utf8_lossy(&forecast.stdout)
        .contains("timed out after 10s while running gate selection"));
}

#[test]
fn would_range_error_is_reported_on_stdout_without_changing_status_exit() {
    let temp = fixture();
    let base = temp.path();
    first_device(base);
    install_push_changed_files(base);
    let ordinary = epr(base, &["device", "status"]);
    let forecast = epr(
        base,
        &[
            "device",
            "status",
            "--would",
            "missing-tip",
            "--target",
            "missing-destination",
        ],
    );
    assert_eq!(forecast.status.code(), ordinary.status.code());
    let text = String::from_utf8_lossy(&forecast.stdout);
    assert!(text.contains("FORECAST"), "{text}");
    assert!(text.contains("capacity:"), "{text}");
    assert!(
        text.contains("changed-file range unreadable (exit 4)"),
        "{text}"
    );
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
#[cfg(feature = "peer")] // the live daemon holds `peer::ServeLock`
fn peer_daemon_status_compares_release_record_cids_not_binary_blob_cids() {
    use std::os::unix::{fs::symlink, fs::PermissionsExt};
    let temp = fixture();
    let base = temp.path();
    let store = base.join("store/epr");
    let current = eprfs_core::BlobCid::compute_bytes(b"current").to_string();
    let older = eprfs_core::BlobCid::compute_bytes(b"older").to_string();
    let current_blob = eprfs_core::BlobCid::compute_bytes(b"current binary blob").to_string();
    let older_blob = eprfs_core::BlobCid::compute_bytes(b"older binary blob").to_string();
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
    let write = |release_cid: &str, blob_cid: &str, start: u64| {
        fs::write(
            &lease,
            json!({"cid":release_cid,"runningCid":blob_cid,"session":"daemon-fixture",
            "pinnedAt":"2026-10-24T00:00:00Z","ownerPid":pid,"ownerStartTime":start.to_string()})
            .to_string(),
        )
        .unwrap();
    };
    write(&older, &older_blob, start);
    // A live daemon holds its serve lock; liveness is the lock, never a recorded pid.
    let _serving = elohim_epr_cli::peer::ServeLock::acquire(&base.join("config/epr/peer")).unwrap();
    let (_, rows) = checks(base);
    let peer = rows.iter().find(|r| r["name"] == "peer daemon").unwrap();
    assert_eq!(peer["state"], "degraded");
    assert!(peer["detail"]
        .as_str()
        .unwrap()
        .contains("release-record CID"));
    assert!(peer["cure"].as_str().unwrap().contains("restart"));
    let peer = |base: &Path| {
        checks(base)
            .1
            .into_iter()
            .find(|r| r["name"] == "peer daemon")
            .unwrap()
    };
    write(&current, &current_blob, start);
    let current_peer = peer(base);
    assert_eq!(current_peer["state"], "ok");
    assert!(current_peer["detail"]
        .as_str()
        .unwrap()
        .contains("running and current release-record CIDs"));
    write(&older, &current_blob, start);
    assert_eq!(
        peer(base)["state"],
        "degraded",
        "a current binary-blob CID cannot make an older release-record CID current"
    );
    write(&older, &older_blob, start + 1);
    assert_eq!(peer(base)["state"], "ok");
}

#[test]
#[cfg(feature = "peer")]
fn explicit_epr_bin_inside_store_reads_peer_daemon_current() {
    use elohim_device_runtime::bin_store::{IngestOptions, Registry, Store};
    use std::os::unix::{fs::symlink, fs::PermissionsExt};

    let temp = fixture();
    let base = temp.path();
    let store_root = base.join("store/epr");
    let registry: Registry = serde_json::from_value(json!({
        "version": 1,
        "tools": {"epr": {"bins": ["epr"], "candidateEnv": {"epr": ["EPR_BIN"]}}}
    }))
    .unwrap();
    let store = Store::new(&store_root, "epr", &registry).unwrap();
    let source = base.join("candidate");
    fs::create_dir(&source).unwrap();
    let source_epr = source.join("epr");
    fs::copy(env!("CARGO_BIN_EXE_epr"), &source_epr).unwrap();
    fs::set_permissions(&source_epr, fs::Permissions::from_mode(0o755)).unwrap();
    let release = store
        .ingest(
            &source,
            &IngestOptions {
                source_commit: Some("a".repeat(40)),
                tree_clean: Some(true),
                gate_run: Some("device-arrival-test".into()),
                ..Default::default()
            },
        )
        .unwrap();
    symlink(&release.cid, store_root.join("current")).unwrap();
    fs::write(
        store_root.join("current.json"),
        json!({"cid": release.cid, "commit": "a".repeat(40)}).to_string(),
    )
    .unwrap();

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
    let executable = store_root.join(&release.cid).join("epr");
    store
        .record_running_daemon(&executable, pid, &base.join("proc"), Some(&executable))
        .unwrap();

    let _serving = elohim_epr_cli::peer::ServeLock::acquire(&base.join("config/epr/peer")).unwrap();
    let peer = line(base, "peer daemon");
    assert_eq!(peer["state"], "ok", "{peer}");
    assert!(peer["detail"].as_str().unwrap().contains(&release.cid));
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
