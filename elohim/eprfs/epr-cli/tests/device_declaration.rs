//! Device commands cannot borrow the invoking process's runtime file or registry authority.
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

fn epr(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_epr"))
        .arg("device")
        .args(args)
        .arg("--root")
        .arg(root.join("device"))
        .env_remove("EPR_SESSION_LAZY")
        .env("ELOHIM_SESSION_ID", "declaration-fixture")
        .env("ELOHIM_DEVICE_KEY_FILE", root.join("key"))
        .env("EPR_UNDER_TEST", "1")
        .env("EPR_PROC_ROOT", root.join("proc"))
        .env("BIN_STORE", root.join("bins"))
        .env("EPR_BIN_TOOLS", root.join("invalid-registry"))
        .env("ELOHIM_RUNTIME_CONFIG_PATH", root.join("ambient.toml"))
        .env("ELOHIM_RUNTIME_MANIFEST_PATH", root.join("invalid-base"))
        .output()
        .unwrap()
}
fn value(output: Output) -> serde_json::Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn build_role_uses_the_private_manifest_and_names_carrier_switch_authority() {
    let root = tempfile::tempdir().unwrap();
    let missing_until = epr(root.path(), &["build-role", "builder"]);
    assert!(!missing_until.status.success());
    assert!(String::from_utf8_lossy(&missing_until.stderr).contains("needs an until"));
    assert!(!root.path().join("device/manifest/head").exists());

    let builder = value(epr(
        root.path(),
        &["build-role", "builder", "--until", "2099-01-01T00:00:00Z"],
    ));
    assert_eq!(builder["buildRole"]["role"], "builder");
    assert_eq!(builder["buildRole"]["until"], "2099-01-01T00:00:00Z");

    let duplicate_carrier_expiry = epr(
        root.path(),
        &["build-role", "carrier", "--until", "2099-01-01T00:00:00Z"],
    );
    assert!(!duplicate_carrier_expiry.status.success());
    assert!(String::from_utf8_lossy(&duplicate_carrier_expiry.stderr).contains("carry switch"));
    assert_eq!(value(epr(root.path(), &["show"])), builder);

    let neither = value(epr(root.path(), &["build-role", "neither"]));
    assert_eq!(neither["buildRole"]["role"], "neither");
    assert!(neither["buildRole"]["until"].is_null());
    let manifest: serde_json::Value =
        serde_json::from_slice(&epr(root.path(), &["show", "--render", "manifest"]).stdout)
            .unwrap();
    assert_eq!(manifest["build_role"]["role"], "neither");
    assert!(root.path().join("device/manifest/head").is_file());
    assert!(root
        .path()
        .read_dir()
        .unwrap()
        .all(|entry| entry.unwrap().file_name() == "device"));
}
#[test]
fn explicit_root_retains_its_base_and_refused_options_never_mutate() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("ambient.toml"), "untouched").unwrap();
    fs::write(root.path().join("invalid-registry"), "invalid").unwrap();
    fs::write(root.path().join("invalid-base"), "invalid").unwrap();
    let mut base = elohim_device_runtime::manifest_store::workspace_base().unwrap();
    base.follows[0].mode = ark_core::declaration::FollowMode::Observe;
    let path = root.path().join("fleet-base.json");
    fs::write(&path, serde_json::to_vec(&base).unwrap()).unwrap();
    let first = value(epr(
        root.path(),
        &["apply", "--base", path.to_str().unwrap()],
    ));
    assert_eq!(value(epr(root.path(), &["apply"])), first);
    let head = value(epr(root.path(), &["follow", "bin:epr", "--mode", "canary"]));
    assert!(!epr(
        root.path(),
        &["follow", "bin:brit", "--mode", "apply", "--render", "typo"]
    )
    .status
    .success());
    assert_eq!(value(epr(root.path(), &["show"])), head);
    assert_eq!(
        fs::read_to_string(root.path().join("ambient.toml")).unwrap(),
        "untouched"
    );
    assert!(!root.path().join("bins").exists());
}

#[test]
fn fresh_cli_recovers_the_committed_base_after_projection_failure() {
    recover_default_adapter(false);
}

#[test]
fn fresh_storage_boot_recovers_the_committed_base_after_projection_failure() {
    recover_default_adapter(true);
}

fn recover_default_adapter(storage: bool) {
    use elohim_device_runtime::manifest_store::{boot_runtime_config, workspace_base};
    let root = tempfile::tempdir().unwrap();
    value(epr(root.path(), &["follow", "bin:old"]));
    let device = root.path().join("device");
    let saved = device.join("runtime-manifest.json");
    let saved_a = fs::read(&saved).unwrap();
    let mut base = workspace_base().unwrap();
    base.follows[0].mode = ark_core::declaration::FollowMode::Canary;
    let explicit = root.path().join("base-b.json");
    fs::write(&explicit, serde_json::to_vec(&base).unwrap()).unwrap();
    // Fail the first projection rename after the act commits, even as root.
    fs::remove_file(&saved).unwrap();
    fs::create_dir(&saved).unwrap();
    let failed = epr(
        root.path(),
        &["follow", "bin:new", "--base", explicit.to_str().unwrap()],
    );
    assert!(!failed.status.success());
    let head = fs::read(device.join("manifest/head")).unwrap();
    let record: serde_json::Value = serde_json::from_slice(&head).unwrap();
    assert_eq!(record["base"], base.cid().unwrap());
    assert!(device
        .join(format!("manifest/{}.cbor", base.cid().unwrap()))
        .is_file());
    fs::remove_dir(&saved).unwrap();
    fs::write(&saved, &saved_a).unwrap();
    if storage {
        boot_runtime_config(Some(&device.join("runtime-config.toml")), None, None).unwrap();
    } else {
        value(epr(root.path(), &["apply"]));
    }
    assert_eq!(fs::read(device.join("manifest/head")).unwrap(), head);
    assert_eq!(
        ark_core::RuntimeManifest::from_json(&fs::read_to_string(&saved).unwrap()).unwrap(),
        base
    );
    let config = fs::read_to_string(device.join("runtime-config.toml")).unwrap();
    assert!(config.contains("bin:new=observe"), "{config}");
    assert!(config.contains("bin:epr=canary"), "{config}");
    assert!(
        !config.contains("bin:old"),
        "explicit B replaced A's private follows"
    );
}

#[test]
fn an_open_headless_default_keeps_a_later_explicit_cli_base() {
    use elohim_device_runtime::manifest_store::{workspace_base, ManifestStore, Projections};
    let root = tempfile::tempdir().unwrap();
    value(epr(root.path(), &["apply"]));
    let device = root.path().join("device");
    let default = ManifestStore::from_source(
        &device,
        None,
        Projections {
            runtime_config: Some(device.join("runtime-config.toml")),
            bin_tools: Some(device.join("bin/bin-tools.json")),
        },
    )
    .unwrap();
    let mut base = workspace_base().unwrap();
    base.follows[0].mode = ark_core::declaration::FollowMode::Canary;
    let source = root.path().join("base-b.json");
    fs::write(&source, serde_json::to_vec(&base).unwrap()).unwrap();
    value(epr(
        root.path(),
        &["apply", "--base", source.to_str().unwrap()],
    ));
    assert!(!device.join("manifest/head").exists());
    assert_eq!(default.apply().unwrap(), base);
    assert!(!device.join("manifest/head").exists());
    let act = default.follow_list("bin:next=observe").unwrap();
    assert_eq!(act.supersedes, Some(base.cid().unwrap()));
    assert_eq!(
        ark_core::RuntimeManifest::from_json(
            &fs::read_to_string(device.join("runtime-manifest.json")).unwrap()
        )
        .unwrap(),
        base
    );
}

fn snapshot(dir: &Path) -> std::collections::BTreeMap<std::path::PathBuf, Vec<u8>> {
    let mut files = std::collections::BTreeMap::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(next) = stack.pop() {
        for entry in fs::read_dir(&next).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path.clone());
                files.insert(path, Vec::new());
            } else {
                files.insert(path.clone(), fs::read(&path).unwrap());
            }
        }
    }
    files
}

/// Story (slice 2 S2c): the peer daemon is declared on the device's private chain; adopting
/// a committed base (whose processes are empty) previews without writing, keeps `peer`, and
/// refuses rather than silently replacing a `peer` the base declares differently.
#[test]
fn apply_previews_a_base_and_keeps_the_declared_peer() {
    let root = tempfile::tempdir().unwrap();
    let device = root.path().join("device");
    value(epr(root.path(), &["apply"]));
    value(epr(
        root.path(),
        &[
            "run",
            "epr",
            "--name",
            "peer",
            "--env",
            "EPR_PEER_FIXTURE=1",
        ],
    ));
    let mut base = elohim_device_runtime::manifest_store::workspace_base().unwrap();
    base.follows[0].mode = ark_core::declaration::FollowMode::Canary;
    assert!(
        base.processes.is_empty(),
        "the committed base declares no process"
    );
    let committed = root.path().join("committed-base.json");
    fs::write(&committed, serde_json::to_vec(&base).unwrap()).unwrap();
    let committed = committed.to_str().unwrap();

    let before = snapshot(&device);
    let replacing = value(epr(
        root.path(),
        &[
            "apply",
            "--preview",
            "--base",
            committed,
            "--replace-processes",
            "--json",
        ],
    ));
    assert_eq!(
        replacing["processes"]["removed"],
        serde_json::json!(["peer"])
    );
    assert_eq!(replacing["follows"]["changed"][0]["name"], "bin:epr");
    assert_ne!(replacing["currentCid"], replacing["nextCid"]);
    let text = epr(
        root.path(),
        &[
            "apply",
            "--preview",
            "--base",
            committed,
            "--replace-processes",
        ],
    );
    assert!(text.status.success());
    let text = String::from_utf8(text.stdout).unwrap();
    assert!(text.contains("nothing written"), "{text}");
    assert!(text.contains("processes:\n  - peer"), "{text}");
    let carrying = value(epr(
        root.path(),
        &["apply", "--preview", "--base", committed, "--json"],
    ));
    assert_eq!(carrying["processes"]["removed"], serde_json::json!([]));
    assert_eq!(carrying["carried"], serde_json::json!(["peer"]));
    assert_eq!(carrying["conflicts"], serde_json::json!([]));
    assert_eq!(snapshot(&device), before, "a preview writes nothing");

    let output = epr(root.path(), &["apply", "--base", committed]);
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(
        stderr.contains("carried (device-owned, not in base): peer"),
        "{stderr}"
    );
    let applied = value(output);
    assert_eq!(applied["manifestCid"], carrying["nextCid"]);
    assert_eq!(applied["processes"][0]["name"], "peer");

    // A base that names `peer` differently would drop this device's declaration: refused.
    let full: serde_json::Value =
        serde_json::from_slice(&epr(root.path(), &["show", "--render", "manifest"]).stdout)
            .unwrap();
    let mut theirs = full["processes"][0].clone();
    theirs["argv"] = serde_json::json!(["{artifact}", "peer", "serve"]);
    let mut conflicting = serde_json::to_value(&base).unwrap();
    conflicting["follows"][1]["mode"] = serde_json::json!("canary");
    conflicting["processes"] = serde_json::json!([theirs]);
    let conflicting_path = root.path().join("conflicting-base.json");
    fs::write(&conflicting_path, serde_json::to_vec(&conflicting).unwrap()).unwrap();
    let conflicting_path = conflicting_path.to_str().unwrap();
    let before = snapshot(&device);
    let dry = epr(
        root.path(),
        &["apply", "--preview", "--base", conflicting_path, "--json"],
    );
    assert_eq!(dry.status.code(), Some(1), "a conflicting dry run fails");
    let dry: serde_json::Value = serde_json::from_slice(&dry.stdout).unwrap();
    assert_eq!(dry["conflicts"], serde_json::json!(["peer"]));
    let text = epr(
        root.path(),
        &["apply", "--preview", "--base", conflicting_path],
    );
    assert_eq!(text.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&text.stdout).contains("conflicts ("));
    let refused = epr(root.path(), &["apply", "--base", conflicting_path]);
    assert!(!refused.status.success());
    let stderr = String::from_utf8_lossy(&refused.stderr);
    assert!(stderr.contains("manifest-process-conflict"), "{stderr}");
    assert!(stderr.contains("peer"), "{stderr}");
    assert!(stderr.contains("--replace-processes"), "{stderr}");
    assert_eq!(snapshot(&device), before, "a refusal writes nothing");

    let replaced = value(epr(
        root.path(),
        &["apply", "--base", conflicting_path, "--replace-processes"],
    ));
    assert_eq!(
        replaced["processes"][0]["argv"],
        serde_json::json!(["{artifact}", "peer", "serve"])
    );
}

#[test]
fn preview_and_replace_processes_belong_to_their_verbs() {
    let root = tempfile::tempdir().unwrap();
    for args in [
        &["show", "--preview"][..],
        &["follow", "bin:epr", "--preview"],
        &["apply", "--preview", "--render", "manifest"],
        &["apply", "--preview", "--mode", "canary"],
        &["apply", "--preview", "--pin", "bafy"],
        &["apply", "--preview", "--stop"],
        &["apply", "--preview", "--unfollow"],
        &["apply", "--preview", "--channels", "bin:epr=observe"],
        &["show", "--replace-processes"],
    ] {
        assert!(!epr(root.path(), args).status.success(), "{args:?}");
    }
    assert!(!root.path().join("device/manifest/head").exists());
}
