use ark_core::declaration::{FollowMode, ManifestAct};
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
