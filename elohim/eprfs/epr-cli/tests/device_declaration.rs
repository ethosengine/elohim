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
