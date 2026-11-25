use ark_core::declaration::{FollowMode, ManifestAct};
use ark_core::{BuildRole, BuildRoleDeclaration};
use elohim_device_runtime::manifest_store::{self, ManifestStore, Projections};
use std::fs;

fn fixture() -> (tempfile::TempDir, ManifestStore) {
    let dir = tempfile::tempdir().unwrap();
    let store = ManifestStore::workspace(
        dir.path(),
        Projections {
            runtime_config: Some(dir.path().join("runtime-config.toml")),
            bin_tools: Some(dir.path().join("bin/bin-tools.json")),
        },
    )
    .unwrap();
    (dir, store)
}
fn follow(channel: &str) -> ManifestAct {
    ManifestAct::Follow {
        channel: channel.into(),
        mode: Some(FollowMode::Observe),
        pin: None,
    }
}
#[test]
fn offline_chain_replay_and_base_recovery() {
    let (dir, store) = fixture();
    let base = store.read().unwrap();
    let first = store
        .act(follow("runtime:coordinators:elohim:workspace"))
        .unwrap();
    assert_eq!(first.supersedes, Some(base.cid().unwrap()));
    let file = dir.path().join("runtime-config.toml");
    let expected = fs::read_to_string(&file).unwrap();
    fs::remove_file(&file).unwrap();
    assert_eq!(store.apply().unwrap(), first);
    assert_eq!(fs::read_to_string(&file).unwrap(), expected);
    fs::remove_dir_all(dir.path().join("manifest")).unwrap();
    assert_eq!(store.read().unwrap(), base);
}
#[test]
fn malformed_head_never_falls_back_and_failed_acts_do_not_move_it() {
    let (dir, store) = fixture();
    let first = store.act(follow("bin:epr")).unwrap();
    assert!(store.act(follow("../escape")).is_err());
    assert_eq!(store.read().unwrap(), first);
    fs::write(dir.path().join("manifest/head"), "../../escape").unwrap();
    assert!(store.read().is_err());
}
#[test]
fn concurrent_acts_do_not_lose_a_channel() {
    let (_dir, store) = fixture();
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..8)
            .map(|i| {
                let store = &store;
                scope.spawn(move || store.act(follow(&format!("bin:tool{i}"))).unwrap())
            })
            .collect();
        for handle in handles {
            handle.join().unwrap();
        }
    });
    assert_eq!(store.read().unwrap().follows.len(), 10);
}
#[test]
fn config_release_is_replayable_and_neighbour_settings_survive_follow() {
    let (dir, store) = fixture();
    store
        .config_release(
            "CONTEST_BACKOFF_SECONDS = 17\nELOHIM_RELEASE_CHANNELS = \"bin:epr=observe\"\n",
        )
        .unwrap();
    store.act(follow("bin:brit")).unwrap();
    fs::remove_file(dir.path().join("runtime-config.toml")).unwrap();
    store.apply().unwrap();
    let result = fs::read_to_string(dir.path().join("runtime-config.toml")).unwrap();
    assert!(result.contains("CONTEST_BACKOFF_SECONDS = 17"));
    assert!(result.contains("bin:epr=observe,bin:brit=observe"));
}
#[test]
fn new_outputs_match_view_schemas() {
    let (dir, store) = fixture();
    let head = store.read().unwrap();
    let views = [
        (
            "runtime-declaration",
            serde_json::to_value(manifest_store::view(&head).unwrap()).unwrap(),
        ),
        (
            "bin-tools",
            serde_json::to_value(manifest_store::registry(&head).unwrap()).unwrap(),
        ),
        ("runtime-manifest", serde_json::to_value(&head).unwrap()),
        (
            "runtime-observation",
            serde_json::to_value(
                manifest_store::observe(&head, dir.path(), &Default::default()).unwrap(),
            )
            .unwrap(),
        ),
    ];
    for (name, value) in views {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!("../../sdk/schemas/v1/views/{name}.schema.json"));
        let schema: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        assert!(
            validator.is_valid(&value),
            "{name}: {:?}",
            validator
                .iter_errors(&value)
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn an_explicit_changed_base_supersedes_private_head_and_preserves_settings() {
    let (dir, store) = fixture();
    store.apply().unwrap();
    let old = store.act(follow("bin:local")).unwrap();
    let mut base = manifest_store::workspace_base().unwrap();
    base.follows[0].mode = FollowMode::Canary;
    let deployed = ManifestStore::new(
        dir.path(),
        base,
        Projections {
            runtime_config: Some(dir.path().join("runtime-config.toml")),
            bin_tools: None,
        },
    )
    .unwrap();
    let next = deployed.apply().unwrap();
    assert_eq!(
        next.supersedes.as_deref(),
        Some(old.cid().unwrap().as_str())
    );
    assert_eq!(next.follows[0].mode, FollowMode::Canary);
    assert_eq!(deployed.apply().unwrap(), next);
}

#[test]
fn fleet_route_reads_saved_base_and_never_guesses_workspace_defaults() {
    if std::env::var_os("M_ISOLATION_CHILD").is_none() {
        let ambient = tempfile::tempdir().unwrap();
        let watched = ambient.path().join("ambient.toml");
        fs::write(&watched, "DO NOT TOUCH").unwrap();
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "fleet_route_reads_saved_base_and_never_guesses_workspace_defaults",
            ])
            .env("M_ISOLATION_CHILD", "1")
            .env("ELOHIM_RUNTIME_MANIFEST_PATH", ambient.path())
            .env("ELOHIM_RUNTIME_CONFIG_PATH", &watched)
            .status()
            .unwrap();
        assert!(status.success());
        assert_eq!(fs::read_to_string(watched).unwrap(), "DO NOT TOUCH");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("runtime-config.toml");
    fs::write(&config, "# an existing fleet file\n").unwrap();
    assert_eq!(
        manifest_store::for_runtime_config_with_base(&config, None)
            .unwrap_err()
            .kind,
        "manifest-base-absent"
    );
    let mut base = manifest_store::workspace_base().unwrap();
    base.tools.clear();
    base.follows = vec![ark_core::declaration::Follow {
        channel: "runtime:coordinators:elohim:workspace".into(),
        mode: FollowMode::Canary,
        pin: None,
    }];
    fs::write(
        dir.path().join("runtime-manifest.json"),
        serde_json::to_vec(&base).unwrap(),
    )
    .unwrap();
    let store = manifest_store::for_runtime_config_with_base(&config, None).unwrap();
    let next = store.act(follow("runtime:app-bundle:alpha:dev")).unwrap();
    assert!(next.tools.is_empty());
    assert_eq!(next.follows[0], base.follows[0]);
}

#[test]
fn boot_seed_never_overrides_private_follows_or_a_base_pin() {
    let (dir, _store) = fixture();
    let mut base = manifest_store::workspace_base().unwrap();
    base.follows[0].pin =
        Some("bafkreihdwdcefgh4dqkjv67uzcmw7ojee6xedzdetojuzjevtenxquvyku".into());
    let store = ManifestStore::new(
        dir.path(),
        base.clone(),
        Projections {
            runtime_config: Some(dir.path().join("runtime-config.toml")),
            bin_tools: None,
        },
    )
    .unwrap();
    store
        .seed_settings(
            "ELOHIM_RELEASE_CHANNELS = \"bin:epr=observe\"\nCONTEST_BACKOFF_SECONDS = 17\n",
        )
        .unwrap();
    assert_eq!(store.apply().unwrap(), base);
    let head = store.act(follow("bin:local")).unwrap();
    fs::remove_file(dir.path().join("runtime-config.toml")).unwrap();
    store
        .seed_settings(
            "ELOHIM_RELEASE_CHANNELS = \"bin:epr=observe\"\nCONTEST_BACKOFF_SECONDS = 17\n",
        )
        .unwrap();
    assert_eq!(store.apply().unwrap(), head);
    store
        .seed_settings(
            "ELOHIM_RELEASE_CHANNELS = \"bin:brit=apply\"\nCONTEST_BACKOFF_SECONDS = 99\n",
        )
        .unwrap();
    let changed = store.apply().unwrap();
    assert_eq!(changed.follows, head.follows);
    assert_eq!(changed.supersedes, Some(head.cid().unwrap()));
    assert!(fs::read_to_string(dir.path().join("runtime-config.toml"))
        .unwrap()
        .contains("CONTEST_BACKOFF_SECONDS = 99"));
}

#[test]
fn legacy_mode_and_config_inputs_preserve_pins_until_channel_removal() {
    let (_dir, store) = fixture();
    let pin = "bafkreihdwdcefgh4dqkjv67uzcmw7ojee6xedzdetojuzjevtenxquvyku";
    store
        .act(ManifestAct::Follow {
            channel: "bin:epr".into(),
            mode: Some(FollowMode::Apply),
            pin: Some(pin.into()),
        })
        .unwrap();
    let head = store
        .follow_mode("bin:epr".into(), Some(FollowMode::Canary))
        .unwrap();
    assert_eq!(head.follows[0].pin.as_deref(), Some(pin));
    let head = store.follow_list("bin:epr=observe").unwrap();
    assert_eq!(head.follows[0].pin.as_deref(), Some(pin));
    let head = store
        .config_release("ELOHIM_RELEASE_CHANNELS = \"bin:epr=apply\"\n")
        .unwrap();
    assert_eq!(head.follows[0].pin.as_deref(), Some(pin));
    assert!(store
        .follow_mode("bin:epr".into(), None)
        .unwrap()
        .follows
        .is_empty());
}

#[test]
fn changed_seed_and_base_adopt_new_follows_together() {
    let (dir, store) = fixture();
    store
        .seed_settings("CONTEST_BACKOFF_SECONDS = 17\n")
        .unwrap();
    store.apply().unwrap();
    store.act(follow("bin:local")).unwrap();
    let mut base = manifest_store::workspace_base().unwrap();
    base.follows[0].mode = FollowMode::Canary;
    let next = ManifestStore::new(
        dir.path(),
        base.clone(),
        Projections {
            runtime_config: Some(dir.path().join("runtime-config.toml")),
            bin_tools: None,
        },
    )
    .unwrap();
    next.seed_settings("CONTEST_BACKOFF_SECONDS = 99\n")
        .unwrap();
    assert_eq!(next.apply().unwrap().follows, base.follows);
    let projected = fs::read_to_string(dir.path().join("runtime-config.toml")).unwrap();
    assert!(projected.contains("bin:epr=canary"));
    assert!(projected.contains("CONTEST_BACKOFF_SECONDS = 99"));
}

#[test]
fn every_act_reconciles_a_changed_base_before_saving_it() {
    let (dir, store) = fixture();
    store.act(follow("bin:old")).unwrap();
    let mut base = manifest_store::workspace_base().unwrap();
    base.follows[0].mode = FollowMode::Canary;
    let next = ManifestStore::new(
        dir.path(),
        base,
        Projections {
            runtime_config: None,
            bin_tools: None,
        },
    )
    .unwrap();
    let head = next.act(follow("bin:new")).unwrap();
    assert_eq!(head.follows[0].mode, FollowMode::Canary);
    assert!(!head.follows.iter().any(|f| f.channel == "bin:old"));
    assert!(head.follows.iter().any(|f| f.channel == "bin:new"));
}

#[test]
fn projections_refuse_a_second_chain_and_storage_resolves_the_owner() {
    let (dir, store) = fixture();
    store.act(follow("bin:private")).unwrap();
    for projection in ["runtime-config.toml", "bin/bin-tools.json"] {
        let path = dir.path().join(projection);
        let before = fs::read(&path).unwrap();
        let other = dir.path().join("other");
        let projections = if projection.ends_with("json") {
            Projections {
                runtime_config: None,
                bin_tools: Some(path.clone()),
            }
        } else {
            Projections {
                runtime_config: Some(path.clone()),
                bin_tools: None,
            }
        };
        let rival = ManifestStore::workspace(&other, projections).unwrap();
        let error = rival.act(follow("bin:wrong")).unwrap_err();
        assert_eq!(error.kind, "projection-owner-conflict");
        assert!(error
            .to_string()
            .contains(dir.path().join("manifest").to_str().unwrap()));
        assert_eq!(fs::read(path).unwrap(), before);
        assert!(!other.join("manifest/head").exists());
    }
    let config = dir.path().join("runtime-config.toml");
    let resolved = manifest_store::for_runtime_config_with_base(&config, None).unwrap();
    assert_eq!(resolved.read().unwrap(), store.read().unwrap());
    resolved.act(follow("bin:storage")).unwrap();
    assert_eq!(resolved.read().unwrap(), store.read().unwrap());
}

#[test]
fn storage_boot_repairs_external_projection_from_owner_without_manifest_override() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("external/runtime.toml");
    let store = ManifestStore::workspace(
        dir.path().join("device"),
        Projections {
            runtime_config: Some(config.clone()),
            bin_tools: None,
        },
    )
    .unwrap();
    store.act(follow("bin:repair")).unwrap();
    let expected = fs::read(&config).unwrap();
    fs::write(&config, "ELOHIM_RELEASE_CHANNELS = \"bin:stale=observe\"\n").unwrap();
    manifest_store::repair_runtime_config(&config, None, None).unwrap();
    assert_eq!(fs::read(&config).unwrap(), expected);
    fs::remove_file(format!("{}.owner", config.display())).unwrap();
    // Pre-marker chains beside their watched config also repair on boot.
    let local = dir.path().join("device/runtime.toml");
    manifest_store::repair_runtime_config(&local, None, None).unwrap();
    assert_eq!(fs::read(local).unwrap(), expected);
}

#[test]
fn secret_named_environment_is_refused_on_import_and_every_act() {
    for key in [
        "API_TOKEN",
        "secret",
        "Db_Password",
        "PASSPHRASE",
        "CREDENTIAL",
        "PRIVATE_KEY",
        "API_KEY",
    ] {
        let (dir, store) = fixture();
        let mut base = ark_core::declaration::supersede(
            &manifest_store::workspace_base().unwrap(),
            ManifestAct::Run {
                tool: "epr".into(),
                stop: false,
                launch: Default::default(),
            },
        )
        .unwrap();
        base.processes[0]
            .env
            .insert(key.into(), "literal-value".into());
        let error = ManifestStore::new(
            dir.path(),
            base.clone(),
            Projections {
                runtime_config: None,
                bin_tools: None,
            },
        )
        .unwrap_err();
        assert!(error.to_string().contains(key));
        assert!(error
            .to_string()
            .contains("secrets are referenced by name only"));
        // A legacy chain cannot smuggle a secret through a subsequent act or replay.
        fs::create_dir_all(dir.path().join("manifest")).unwrap();
        let cid = base.cid().unwrap();
        fs::write(
            dir.path().join(format!("manifest/{cid}.cbor")),
            base.canonical_bytes().unwrap(),
        )
        .unwrap();
        let record = serde_json::json!({"cid": cid, "base": store.read().unwrap().cid().unwrap()})
            .to_string();
        fs::write(dir.path().join("manifest/head"), &record).unwrap();
        assert!(store
            .act(follow("bin:epr"))
            .unwrap_err()
            .to_string()
            .contains(key));
        assert!(store.apply().unwrap_err().to_string().contains(key));
        assert_eq!(
            fs::read_to_string(dir.path().join("manifest/head")).unwrap(),
            record
        );
    }
}

#[test]
fn secret_named_settings_never_enter_seed_or_config_release_replay() {
    let (dir, store) = fixture();
    let payload = "Api_Token = \"literal-value\"\n";
    assert!(store
        .seed_settings(payload)
        .unwrap_err()
        .to_string()
        .contains("Api_Token"));
    assert!(store
        .config_release(payload)
        .unwrap_err()
        .to_string()
        .contains("Api_Token"));
    assert!(!dir.path().join("runtime-config-seed.toml").exists());
    assert!(!dir.path().join("manifest/head").exists());
}

#[test]
fn mode_only_acts_retain_the_new_bases_pin() {
    for legacy_list in [false, true] {
        let (dir, store) = fixture();
        store.apply().unwrap();
        store.act(follow("bin:epr")).unwrap();
        let mut base = manifest_store::workspace_base().unwrap();
        let pin = "bafkreihdwdcefgh4dqkjv67uzcmw7ojee6xedzdetojuzjevtenxquvyku";
        base.follows[0].pin = Some(pin.into());
        let deployed = ManifestStore::new(
            dir.path(),
            base,
            Projections {
                runtime_config: None,
                bin_tools: None,
            },
        )
        .unwrap();
        let head = if legacy_list {
            deployed.follow_list("bin:epr=observe")
        } else {
            deployed.follow_mode("bin:epr".into(), Some(FollowMode::Observe))
        }
        .unwrap();
        assert_eq!(head.follows[0].pin.as_deref(), Some(pin));
    }
}

#[test]
fn a_head_without_a_recorded_base_is_refused() {
    let (dir, store) = fixture();
    let head = store.act(follow("bin:private")).unwrap();
    for text in [
        head.cid().unwrap(),
        serde_json::json!({"cid": head.cid().unwrap()}).to_string(),
    ] {
        fs::write(dir.path().join("manifest/head"), text).unwrap();
        assert!(store.read().is_err());
        assert!(store.apply().is_err());
    }
}

#[test]
fn storage_boot_repairs_a_deleted_chain_and_a_headless_chain() {
    for marker in [true, false] {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("device");
        let config = if marker {
            dir.path().join("external/runtime.toml")
        } else {
            root.join("runtime.toml")
        };
        let store = ManifestStore::workspace(
            &root,
            Projections {
                runtime_config: Some(config.clone()),
                bin_tools: None,
            },
        )
        .unwrap();
        store.act(follow("bin:private")).unwrap();
        fs::remove_dir_all(root.join("manifest")).unwrap();
        if !marker {
            fs::remove_file(format!("{}.owner", config.display())).unwrap();
            fs::create_dir(root.join("manifest")).unwrap();
        }
        manifest_store::boot_runtime_config(Some(&config), None, None).unwrap();
        let text = fs::read_to_string(&config).unwrap();
        assert!(text.contains("bin:epr=apply"));
        assert!(!text.contains("bin:private"));
        assert!(!root.join("manifest/head").exists());
    }
    assert!(
        manifest_store::boot_runtime_config(None, Some(std::path::Path::new("base")), None)
            .is_err()
    );
    assert!(manifest_store::boot_runtime_config(None, None, None).is_ok());
}

#[test]
fn default_base_is_verified_and_never_falls_back_to_the_projection() {
    use elohim_device_runtime::manifest_store::{workspace_base, ManifestStore, Projections};
    let dir = tempfile::tempdir().unwrap();
    let projections = Projections {
        runtime_config: None,
        bin_tools: None,
    };
    let store = ManifestStore::workspace(dir.path(), projections.clone()).unwrap();
    store.follow_list("bin:test=observe").unwrap();
    let path = dir.path().join(format!(
        "manifest/{}.cbor",
        workspace_base().unwrap().cid().unwrap()
    ));
    let mut different = workspace_base().unwrap();
    different.follows.clear();
    std::fs::write(&path, different.canonical_bytes().unwrap()).unwrap();
    let error = ManifestStore::from_source(dir.path(), None, projections.clone()).unwrap_err();
    assert!(
        error.to_string().contains("manifest-cid-mismatch"),
        "{error}"
    );
    std::fs::remove_file(path).unwrap();
    assert!(ManifestStore::from_source(dir.path(), None, projections).is_err());
}

#[test]
fn a_default_handle_opened_before_an_upgrade_adopts_the_new_recorded_base() {
    use elohim_device_runtime::manifest_store::{workspace_base, ManifestStore, Projections};
    let dir = tempfile::tempdir().unwrap();
    let projections = Projections {
        runtime_config: None,
        bin_tools: None,
    };
    let a = ManifestStore::workspace(dir.path(), projections.clone()).unwrap();
    a.follow_list("bin:old=observe").unwrap();
    let default = ManifestStore::from_source(dir.path(), None, projections.clone()).unwrap();
    let mut base = workspace_base().unwrap();
    base.follows[0].mode = ark_core::declaration::FollowMode::Canary;
    let b = ManifestStore::new(dir.path(), base.clone(), projections).unwrap();
    let committed = b.follow_list("bin:new=observe").unwrap();
    assert_eq!(default.apply().unwrap(), committed);
    default.follow_list("bin:next=observe").unwrap();
    let record: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.path().join("manifest/head")).unwrap()).unwrap();
    assert_eq!(record["base"], base.cid().unwrap());
    assert_eq!(
        ark_core::RuntimeManifest::from_json(
            &std::fs::read_to_string(dir.path().join("runtime-manifest.json")).unwrap(),
        )
        .unwrap(),
        base,
    );
}

#[test]
fn a_headless_default_refreshes_before_apply_and_its_first_act() {
    use elohim_device_runtime::manifest_store::{workspace_base, ManifestStore, Projections};
    for apply_first in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let projections = Projections {
            runtime_config: None,
            bin_tools: None,
        };
        let a = ManifestStore::workspace(dir.path(), projections.clone()).unwrap();
        a.apply().unwrap();
        let default = ManifestStore::from_source(dir.path(), None, projections.clone()).unwrap();
        let mut base = workspace_base().unwrap();
        base.follows[0].mode = ark_core::declaration::FollowMode::Canary;
        let source = dir.path().join("base-b.json");
        std::fs::write(&source, serde_json::to_vec(&base).unwrap()).unwrap();
        ManifestStore::from_source(dir.path(), Some(&source), projections)
            .unwrap()
            .apply()
            .unwrap();
        assert!(!dir.path().join("manifest/head").exists());
        if apply_first {
            assert_eq!(default.apply().unwrap(), base);
            assert!(!dir.path().join("manifest/head").exists());
        }
        let first = default.act(follow("bin:next")).unwrap();
        assert_eq!(first.supersedes, Some(base.cid().unwrap()));
        assert_eq!(
            first.follows[0].mode,
            ark_core::declaration::FollowMode::Canary
        );
        let saved = std::fs::read_to_string(dir.path().join("runtime-manifest.json")).unwrap();
        assert_eq!(ark_core::RuntimeManifest::from_json(&saved).unwrap(), base);
        let record: serde_json::Value =
            serde_json::from_slice(&std::fs::read(dir.path().join("manifest/head")).unwrap())
                .unwrap();
        assert_eq!(record["base"], base.cid().unwrap());
    }
}

// Slice 2 S2c: adopting an explicit base keeps the processes this device declared.
fn peer(args: &[&str]) -> ManifestAct {
    ManifestAct::Run {
        tool: "epr".into(),
        stop: false,
        launch: ark_core::declaration::RunLaunch {
            name: Some("peer".into()),
            args: args.iter().map(|a| a.to_string()).collect(),
            ..Default::default()
        },
    }
}
fn deployed(dir: &std::path::Path, base: ark_core::RuntimeManifest) -> ManifestStore {
    ManifestStore::new(
        dir,
        base,
        Projections {
            runtime_config: Some(dir.join("runtime-config.toml")),
            bin_tools: Some(dir.join("bin/bin-tools.json")),
        },
    )
    .unwrap()
}
fn canary_base() -> ark_core::RuntimeManifest {
    let mut base = manifest_store::workspace_base().unwrap();
    base.follows[0].mode = FollowMode::Canary;
    base
}
fn snapshot(dir: &std::path::Path) -> std::collections::BTreeMap<std::path::PathBuf, Vec<u8>> {
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

#[test]
fn adopting_a_base_carries_the_processes_this_device_declared() {
    let (dir, store) = fixture();
    store.apply().unwrap();
    let head = store.act(peer(&["peer", "serve"])).unwrap();
    let base = canary_base();
    let next = deployed(dir.path(), base.clone()).apply().unwrap();
    let mut expected = base;
    expected.supersedes = Some(head.cid().unwrap());
    expected.processes = head.processes.clone();
    assert_eq!(next, expected, "base processes, then the device's own");
    assert_eq!(next.processes[0].name, "peer");
    assert_eq!(
        store.read().unwrap(),
        next,
        "the chain head is the carried CID"
    );
}

#[test]
fn adopting_a_base_keeps_this_devices_build_role() {
    let (dir, store) = fixture();
    store.apply().unwrap();
    let head = store
        .act(ManifestAct::BuildRole(BuildRoleDeclaration {
            role: BuildRole::Neither,
            until: None,
        }))
        .unwrap();
    let mut base = canary_base();
    base.build_role = Some(BuildRoleDeclaration {
        role: BuildRole::Builder,
        until: Some("2099-01-01T00:00:00Z".into()),
    });
    let next = deployed(dir.path(), base).apply().unwrap();
    assert_eq!(next.build_role, head.build_role);
    assert_eq!(next.build_role.unwrap().role, BuildRole::Neither);
}

#[test]
fn a_base_naming_a_device_process_differently_refuses_and_writes_nothing() {
    let (dir, store) = fixture();
    store.apply().unwrap();
    let head = store.act(peer(&["peer", "serve"])).unwrap();
    let mut base = canary_base();
    let mut theirs = head.processes[0].clone();
    theirs.argv.push("--other".into());
    base.processes.push(theirs);
    let before = snapshot(dir.path());
    let adopting = deployed(dir.path(), base);
    let refused = adopting.apply().unwrap_err();
    assert_eq!(refused.kind, "manifest-process-conflict");
    assert!(refused.detail.contains("peer"), "{refused}");
    assert!(refused.detail.contains("--replace-processes"), "{refused}");
    assert!(refused.detail.contains("epr device run"), "{refused}");
    let preview = adopting.preview().unwrap();
    assert_eq!(preview.conflicts, vec!["peer".to_string()]);
    assert_eq!(
        preview.next_cid, preview.current_cid,
        "apply would commit nothing"
    );
    assert!(
        adopting.act(follow("bin:other")).is_err(),
        "every act adopts"
    );
    assert_eq!(snapshot(dir.path()), before);
    assert_eq!(store.read().unwrap(), head);
}

#[test]
fn replace_processes_adopts_the_base_processes_exactly() {
    let (dir, store) = fixture();
    store.apply().unwrap();
    let head = store.act(peer(&["peer", "serve"])).unwrap();
    let base = canary_base();
    let next = deployed(dir.path(), base.clone())
        .replace_processes(true)
        .apply()
        .unwrap();
    let mut expected = base;
    expected.supersedes = Some(head.cid().unwrap());
    assert_eq!(next, expected);
    assert!(next.processes.is_empty(), "peer is gone");
}

#[test]
fn a_base_dropping_no_device_process_adopts_exactly_as_before() {
    // No device process: the adopted manifest is today's `base + supersedes`, byte for byte.
    let (dir, store) = fixture();
    store.apply().unwrap();
    let head = store.act(follow("bin:local")).unwrap();
    let base = canary_base();
    let next = deployed(dir.path(), base.clone()).apply().unwrap();
    let mut today = base.clone();
    today.supersedes = Some(head.cid().unwrap());
    assert_eq!(next.cid().unwrap(), today.cid().unwrap());

    // A process the head's own base declared is the base's: a deployment still removes it.
    let (fleet, scratch) = fixture();
    let declared = scratch.act(peer(&["peer", "serve"])).unwrap().processes;
    let mut with_proxy = manifest_store::workspace_base().unwrap();
    with_proxy.processes = declared;
    let before = deployed(fleet.path(), with_proxy)
        .act(follow("bin:fleet"))
        .unwrap();
    assert_eq!(before.processes.len(), 1);
    let node = deployed(fleet.path(), manifest_store::workspace_base().unwrap());
    let removed = node.act(follow("bin:local")).unwrap();
    assert!(
        removed.processes.is_empty(),
        "the base removed its own process"
    );

    // A device process the new base declares identically has converged; nothing refuses.
    let (dir, store) = fixture();
    store.apply().unwrap();
    let head = store.act(peer(&["peer", "serve"])).unwrap();
    let mut adopting = canary_base();
    adopting.processes = head.processes.clone();
    let next = deployed(dir.path(), adopting.clone()).apply().unwrap();
    let mut today = adopting;
    today.supersedes = Some(head.cid().unwrap());
    assert_eq!(next, today);
}

#[test]
fn preview_names_process_changes_and_writes_nothing() {
    let (dir, store) = fixture();
    store.apply().unwrap();
    let head = store.act(peer(&["peer", "serve"])).unwrap();
    let before = snapshot(dir.path());
    let replacing = deployed(dir.path(), canary_base()).replace_processes(true);
    let preview = replacing.preview().unwrap();
    assert_eq!(preview.current_cid, head.cid().unwrap());
    assert!(preview.changed);
    assert_eq!(preview.processes.removed, vec!["peer".to_string()]);
    assert!(preview.processes.added.is_empty());
    assert_eq!(preview.follows.changed.len(), 1);
    assert_eq!(preview.follows.changed[0].name, "bin:epr");
    assert_eq!(preview.follows.changed[0].fields, vec!["mode".to_string()]);
    let carrying = deployed(dir.path(), canary_base()).preview().unwrap();
    assert_eq!(carrying.processes, Default::default(), "peer is carried");
    // Nothing moved: no head, version, projection, owner marker or lock contents.
    assert_eq!(snapshot(dir.path()), before);
    let applied = deployed(dir.path(), canary_base()).apply().unwrap();
    assert_eq!(carrying.next_cid, applied.cid().unwrap());
    // With nothing to adopt, the preview is the head on both sides.
    let settled = deployed(dir.path(), canary_base()).preview().unwrap();
    assert!(!settled.changed);
    assert_eq!(settled.next_cid, applied.cid().unwrap());
}

#[test]
fn preview_diffs_changed_process_fields_by_wire_name() {
    let mut current = manifest_store::workspace_base().unwrap();
    let (_dir, scratch) = fixture();
    current.processes = scratch.act(peer(&["peer", "serve"])).unwrap().processes;
    let mut next = current.clone();
    next.processes[0].argv.push("--verbose".into());
    next.processes[0]
        .env
        .insert("RUST_LOG".into(), "debug".into());
    next.tools.get_mut("epr").unwrap().bins.push("extra".into());
    let preview = manifest_store::ManifestPreview::between(&current, &next).unwrap();
    assert_eq!(preview.processes.changed[0].name, "peer");
    assert_eq!(
        preview.processes.changed[0].fields,
        vec!["argv".to_string(), "env".to_string()]
    );
    assert_eq!(preview.tools.changed[0].fields, vec!["bins".to_string()]);
}

fn named(name: &str, args: &[&str]) -> ManifestAct {
    ManifestAct::Run {
        tool: "epr".into(),
        stop: false,
        launch: ark_core::declaration::RunLaunch {
            name: Some(name.into()),
            args: args.iter().map(|a| a.to_string()).collect(),
            ..Default::default()
        },
    }
}
// A base declaring `name` with `args`, built from the real run act so it validates.
fn base_with(name: &str, args: &[&str]) -> ark_core::RuntimeManifest {
    let (_dir, scratch) = fixture();
    let mut base = manifest_store::workspace_base().unwrap();
    base.processes = scratch.act(named(name, args)).unwrap().processes;
    base
}

#[test]
fn apply_and_preview_name_the_carried_processes() {
    let (dir, store) = fixture();
    store.apply().unwrap();
    store.act(peer(&["peer", "serve"])).unwrap();
    let preview = deployed(dir.path(), canary_base()).preview().unwrap();
    assert_eq!(preview.carried, vec!["peer".to_string()]);
    assert!(preview.conflicts.is_empty());
    let applied = deployed(dir.path(), canary_base())
        .apply_carrying()
        .unwrap();
    assert_eq!(applied.carried, vec!["peer".to_string()]);
    assert_eq!(applied.manifest.cid().unwrap(), preview.next_cid);
}

#[test]
fn a_device_override_of_a_base_process_is_the_devices_own() {
    // The prior base declares `conductor`; the device re-runs it with its own arguments.
    let prior = base_with("conductor", &["--base-args"]);
    let (dir, _store) = fixture();
    deployed(dir.path(), prior.clone())
        .act(follow("bin:x"))
        .unwrap();
    let head = deployed(dir.path(), prior)
        .act(named("conductor", &["--device-args"]))
        .unwrap();
    // A new base dropping it carries the override.
    let mut dropping = canary_base();
    dropping.processes.clear();
    let carried = deployed(dir.path(), dropping.clone()).preview().unwrap();
    assert_eq!(carried.carried, vec!["conductor".to_string()]);
    // A new base naming it differently (from both the old base and the device) conflicts.
    let mut renamed = base_with("conductor", &["--new-base-args"]);
    renamed.follows[0].mode = FollowMode::Canary;
    let refused = deployed(dir.path(), renamed).apply().unwrap_err();
    assert_eq!(refused.kind, "manifest-process-conflict");
    assert!(refused.detail.contains("conductor"), "{refused}");
    let next = deployed(dir.path(), dropping).apply().unwrap();
    assert_eq!(next.processes, head.processes);
}

#[test]
fn a_carried_process_survives_a_second_adoption_in_order() {
    let (dir, store) = fixture();
    store.apply().unwrap();
    store.act(peer(&["peer", "serve"])).unwrap();
    store.act(named("peer2", &["second"])).unwrap();
    let first = base_with("conductor", &["--base"]);
    let once = deployed(dir.path(), first).apply().unwrap();
    let names = |m: &ark_core::RuntimeManifest| {
        m.processes
            .iter()
            .map(|p| p.name.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(names(&once), ["conductor", "peer", "peer2"]);
    let mut second = base_with("conductor", &["--base"]);
    second.follows[0].mode = FollowMode::Canary;
    let twice = deployed(dir.path(), second).apply().unwrap();
    assert_eq!(names(&twice), ["conductor", "peer", "peer2"]);
    assert_eq!(twice.processes[1..], once.processes[1..]);
    assert_eq!(twice.supersedes, Some(once.cid().unwrap()));
}

#[test]
fn an_unreadable_prior_base_is_a_typed_refusal_with_an_escape() {
    let (dir, store) = fixture();
    store.apply().unwrap();
    let head = store.act(peer(&["peer", "serve"])).unwrap();
    let record: serde_json::Value =
        serde_json::from_slice(&fs::read(dir.path().join("manifest/head")).unwrap()).unwrap();
    let prior = dir.path().join(format!(
        "manifest/{}.cbor",
        record["base"].as_str().unwrap()
    ));
    fs::remove_file(prior).unwrap();
    for refused in [
        deployed(dir.path(), canary_base()).apply().unwrap_err(),
        deployed(dir.path(), canary_base()).preview().unwrap_err(),
    ] {
        assert_eq!(refused.kind, "manifest-base-unreadable", "{refused}");
        assert!(refused.detail.contains("--replace-processes"), "{refused}");
    }
    let replaced = deployed(dir.path(), canary_base())
        .replace_processes(true)
        .apply()
        .unwrap();
    assert_eq!(replaced.supersedes, Some(head.cid().unwrap()));
    assert!(replaced.processes.is_empty());
}

#[test]
fn boot_defers_a_conflicting_adoption_and_keeps_the_previous_head() {
    let (dir, store) = fixture();
    store.apply().unwrap();
    store.act(peer(&["peer", "serve"])).unwrap();
    let head = fs::read(dir.path().join("manifest/head")).unwrap();
    let mut conflicting = base_with("peer", &["other"]);
    conflicting.follows[0].mode = FollowMode::Canary;
    let base = dir.path().join("deployed-base.json");
    fs::write(&base, serde_json::to_vec(&conflicting).unwrap()).unwrap();
    let config = dir.path().join("runtime-config.toml");
    fs::remove_file(&config).unwrap();
    let deferred = manifest_store::boot_runtime_config_reporting(Some(&config), Some(&base), None)
        .unwrap()
        .expect("boot names what it deferred");
    assert_eq!(deferred.kind, "manifest-adoption-deferred");
    assert!(
        deferred.detail.contains("manifest-process-conflict"),
        "{deferred}"
    );
    assert!(deferred.detail.contains("peer"), "{deferred}");
    assert!(
        deferred.detail.contains(&format!(
            "epr device apply --base {} --replace-processes",
            base.display()
        )),
        "{deferred}"
    );
    assert_eq!(fs::read(dir.path().join("manifest/head")).unwrap(), head);
    assert!(
        config.is_file(),
        "the previous head's projection is repaired"
    );
    // The printing form used by storage boot also succeeds.
    manifest_store::boot_runtime_config(Some(&config), Some(&base), None).unwrap();
    assert_eq!(fs::read(dir.path().join("manifest/head")).unwrap(), head);
}

#[test]
fn preview_refuses_another_chains_projection_without_claiming_anything() {
    let owner = tempfile::tempdir().unwrap();
    let shared = owner.path().join("shared/runtime-config.toml");
    let projections = Projections {
        runtime_config: Some(shared.clone()),
        bin_tools: None,
    };
    ManifestStore::workspace(owner.path().join("a"), projections.clone())
        .unwrap()
        .apply()
        .unwrap();
    let other = owner.path().join("b");
    fs::create_dir_all(&other).unwrap();
    let b = ManifestStore::workspace(&other, projections).unwrap();
    let refused = b.preview().unwrap_err();
    assert_eq!(refused.kind, "projection-owner-conflict");
    assert_eq!(
        fs::read_dir(&other).unwrap().count(),
        0,
        "preview created nothing"
    );
    assert_eq!(b.apply().unwrap_err().kind, refused.kind);
}
