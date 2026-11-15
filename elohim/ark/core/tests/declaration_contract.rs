use ark_core::{declaration::*, RuntimeManifest};

fn base() -> RuntimeManifest {
    RuntimeManifest::from_json(include_str!(
        "../../../../genesis/orchestrator/manifests/runtime/workspace-device.manifest.json"
    ))
    .unwrap()
}
#[test]
fn declaration_grammar_and_supersession_preserve_neighbours() {
    let parsed = parse_followed_channels(
        " bin:epr ;bin:brit=apply,bin:epr=observe\nbin:epr=apply,=apply,bin:x=oops",
    );
    assert_eq!(parsed.follows.len(), 2);
    assert_eq!(parsed.refused.len(), 3);
    let first = base();
    let next = supersede(
        &first,
        ManifestAct::Follow {
            channel: "bin:epr".into(),
            mode: Some(FollowMode::Canary),
            pin: None,
        },
    )
    .unwrap();
    assert_eq!(next.supersedes, Some(first.cid().unwrap()));
    assert_eq!(next.follows[1], first.follows[1]);
    assert!(render_runtime_config(&next).contains("bin:epr=canary,bin:brit=apply"));
    let stopped = supersede(
        &supersede(
            &next,
            ManifestAct::Run {
                tool: "epr".into(),
                stop: false,
                launch: RunLaunch::default(),
            },
        )
        .unwrap(),
        ManifestAct::Run {
            tool: "epr".into(),
            stop: true,
            launch: RunLaunch::default(),
        },
    )
    .unwrap();
    assert!(stopped.processes.is_empty());
    assert!(next.tools["epr"].bins.contains(&"bin-launch".to_owned()));
}
#[test]
fn authored_channels_and_pins_refuse_ambiguous_input() {
    for channel in [
        "",
        "ch",
        "bin:../epr",
        "bin:epr=apply",
        "runtime:a:b",
        "runtime:a:b:c:d",
        "bin:a\n",
    ] {
        assert!(
            supersede(
                &base(),
                ManifestAct::Follow {
                    channel: channel.into(),
                    mode: Some(FollowMode::Apply),
                    pin: None
                }
            )
            .is_err(),
            "{channel}"
        );
    }
    assert!(supersede(
        &base(),
        ManifestAct::Follow {
            channel: "bin:epr".into(),
            mode: Some(FollowMode::Apply),
            pin: Some("not-a-cid".into())
        }
    )
    .is_err());
}
#[test]
fn all_committed_manifest_cids_survive_added_fields() {
    let fixtures: serde_json::Value =
        serde_json::from_str(include_str!("manifest-cids.json")).unwrap();
    for fixture in fixtures.as_array().unwrap() {
        let manifest = RuntimeManifest::from_json(&fixture["manifest"].to_string()).unwrap();
        assert_eq!(
            manifest.cid().unwrap(),
            fixture["cid"],
            "{}",
            fixture["path"]
        );
        let wire = serde_json::to_value(manifest).unwrap();
        assert!(wire.get("follows").is_none());
        assert!(wire.get("tools").is_none());
    }
}
#[test]
fn release_pin_requires_the_content_identity_even_when_the_head_exists() {
    let mut manifest = base();
    let pin = manifest.cid().unwrap();
    manifest.follows.push(Follow {
        channel: "runtime:app-bundle:alpha:dev".into(),
        mode: FollowMode::Apply,
        pin: Some(pin.clone()),
    });
    assert!(require_release_pin(&manifest, "runtime:app-bundle:alpha:dev", None).is_err());
    assert!(
        require_release_pin(&manifest, "runtime:app-bundle:alpha:dev", Some("different")).is_err()
    );
    assert!(require_release_pin(&manifest, "runtime:app-bundle:alpha:dev", Some(&pin)).is_ok());
}

/// The chain stores manifests, not acts, so an act gains fields without a wire change. What
/// must hold is that an act carrying no launch detail declares exactly the child it always
/// did. The expected child is written out by hand here, independent of `RunLaunch::child`.
#[test]
fn a_run_act_without_launch_detail_keeps_its_historical_manifest_cid() {
    // Freeze the pre-helper input from b05a282be142: today's device tool inventory
    // can evolve without changing the historical RunLaunch wire contract.
    let base = RuntimeManifest::from_json(include_str!("workspace-device-run-launch.json"))
        .expect("historical device manifest decodes");
    let mut historical: serde_json::Value =
        serde_json::to_value(&base).expect("base manifest encodes");
    historical["supersedes"] = serde_json::Value::String(base.cid().unwrap());
    historical["processes"] = serde_json::json!([{
        "name": "epr",
        "artifact": {"channel": {"channel_id": "bin:epr"}},
        "argv": ["{artifact}"]
    }]);
    let historical = RuntimeManifest::from_json(&historical.to_string()).unwrap();
    let next = supersede(
        &base,
        ManifestAct::Run {
            tool: "epr".into(),
            stop: false,
            launch: RunLaunch::default(),
        },
    )
    .unwrap();
    assert_eq!(next, historical);
    assert_eq!(next.cid().unwrap(), historical.cid().unwrap());
    assert_eq!(
        next.cid().unwrap(),
        "bafyreib6r42fyl4pbsbfkedlwow4mq6vmypniasd5lstjx6ldqmi27z5bm",
        "the hand-written historical child's CID, pinned when RunLaunch landed; a change here moves every chain that ran a tool"
    );
}

#[test]
fn run_launch_appends_args_and_declares_the_readiness_ladder_and_restart() {
    use ark_core::manifest::{Probe, Restart};
    let next = supersede(
        &base(),
        ManifestAct::Run {
            tool: "epr".into(),
            stop: false,
            launch: RunLaunch {
                args: vec!["lsp".into(), "serve".into(), "--device".into()],
                ready_line: Some("lsp ready".into()),
                restart: Some(Restart::Transient),
                ..RunLaunch::default()
            },
        },
    )
    .unwrap();
    let child = &next.processes[0];
    assert_eq!(child.argv, ["{artifact}", "lsp", "serve", "--device"]);
    assert_eq!(
        child.readiness,
        [
            Probe::StdoutLine {
                contains: "lsp ready".into(),
                patience_ms: READY_LINE_PATIENCE_MS
            },
            Probe::ExecutableIdentity {
                patience_ms: IDENTITY_PATIENCE_MS
            }
        ]
    );
    assert_eq!(child.policy.restart, Restart::Transient);
    // A second run act replaces the child rather than adding a sibling.
    let again = supersede(
        &next,
        ManifestAct::Run {
            tool: "epr".into(),
            stop: false,
            launch: RunLaunch::default(),
        },
    )
    .unwrap();
    assert_eq!(again.processes.len(), 1);
    assert_eq!(again.processes[0].argv, ["{artifact}"]);
    assert_ne!(again.cid().unwrap(), next.cid().unwrap());
}

#[test]
fn run_launch_refuses_what_it_cannot_honour() {
    let act = |stop, launch| {
        supersede(
            &base(),
            ManifestAct::Run {
                tool: "epr".into(),
                stop,
                launch,
            },
        )
    };
    assert!(act(
        true,
        RunLaunch {
            args: vec!["serve".into()],
            ..RunLaunch::default()
        }
    )
    .is_err());
    assert!(act(
        false,
        RunLaunch {
            ready_line: Some(String::new()),
            ..RunLaunch::default()
        }
    )
    .is_err());
    assert!(act(
        false,
        RunLaunch {
            args: vec!["a\0b".into()],
            ..RunLaunch::default()
        }
    )
    .is_err());
    assert_eq!(
        parse_restart("Transient"),
        Ok(ark_core::manifest::Restart::Transient)
    );
    assert!(parse_restart("sometimes").is_err());
}

fn run(head: &RuntimeManifest, stop: bool, launch: RunLaunch) -> Result<RuntimeManifest, String> {
    supersede(
        head,
        ManifestAct::Run {
            tool: "epr".into(),
            stop,
            launch,
        },
    )
    .map_err(|e| e.to_string())
}
fn named(name: &str) -> RunLaunch {
    RunLaunch {
        name: Some(name.into()),
        ..RunLaunch::default()
    }
}
fn names(manifest: &RuntimeManifest) -> Vec<&str> {
    manifest.processes.iter().map(|p| p.name.as_str()).collect()
}

#[test]
fn run_launch_names_the_process_and_declares_its_environment() {
    let env = std::collections::BTreeMap::from([
        ("EPR_PEER_RELAYS".to_owned(), "x".to_owned()),
        ("XDG_CONFIG_HOME".to_owned(), "y".to_owned()),
    ]);
    let next = run(
        &base(),
        false,
        RunLaunch {
            args: vec!["peer".into(), "serve".into()],
            env: env.clone(),
            ..named("peer")
        },
    )
    .unwrap();
    let child = &next.processes[0];
    assert_eq!(child.name, "peer");
    assert_eq!(
        child.artifact,
        ark_core::manifest::ArtifactRef::Channel {
            channel_id: "bin:epr".into()
        }
    );
    assert_eq!(child.argv, ["{artifact}", "peer", "serve"]);
    assert_eq!(child.env, env);
}

#[test]
fn an_unnamed_run_act_with_no_environment_keeps_the_historical_manifest_cid() {
    // The same pin as `a_run_act_without_launch_detail_keeps_its_historical_manifest_cid`,
    // reached with both new fields spelled out: an absent name and an empty environment
    // add nothing to the child, and naming the tool itself is the same declaration.
    const HISTORICAL: &str = "bafyreib6r42fyl4pbsbfkedlwow4mq6vmypniasd5lstjx6ldqmi27z5bm";
    let base = RuntimeManifest::from_json(include_str!("workspace-device-run-launch.json"))
        .expect("historical device manifest decodes");
    let explicit = RunLaunch {
        args: Vec::new(),
        ready_line: None,
        restart: None,
        name: None,
        env: std::collections::BTreeMap::new(),
    };
    let next = run(&base, false, explicit).unwrap();
    assert_eq!(next.cid().unwrap(), HISTORICAL);
    assert_eq!(
        run(&base, false, named("epr")).unwrap().cid().unwrap(),
        HISTORICAL
    );
}

#[test]
fn named_processes_of_one_tool_coexist_and_are_replaced_and_stopped_by_name() {
    let both = run(
        &run(&base(), false, named("peer")).unwrap(),
        false,
        RunLaunch::default(),
    )
    .unwrap();
    assert_eq!(names(&both), ["peer", "epr"]);
    // Re-declaring `peer` replaces only `peer`.
    let redeclared = run(
        &both,
        false,
        RunLaunch {
            args: vec!["peer".into(), "serve".into()],
            ..named("peer")
        },
    )
    .unwrap();
    assert_eq!(names(&redeclared), ["epr", "peer"]);
    let epr = redeclared
        .processes
        .iter()
        .find(|p| p.name == "epr")
        .unwrap();
    assert_eq!(epr.argv, ["{artifact}"]);
    let peer = redeclared
        .processes
        .iter()
        .find(|p| p.name == "peer")
        .unwrap();
    assert_eq!(peer.argv, ["{artifact}", "peer", "serve"]);
    // A stop by name removes only that process; the default stop removes only `epr`.
    let stopped = run(&redeclared, true, named("peer")).unwrap();
    assert_eq!(names(&stopped), ["epr"]);
    let stopped = run(&redeclared, true, RunLaunch::default()).unwrap();
    assert_eq!(names(&stopped), ["peer"]);
    // A stop still refuses launch detail other than the name.
    let env = std::collections::BTreeMap::from([("A".to_owned(), "1".to_owned())]);
    assert!(run(
        &redeclared,
        true,
        RunLaunch {
            env,
            ..named("peer")
        }
    )
    .is_err());
}

#[test]
fn run_launch_refuses_secret_or_malformed_environment_and_names() {
    let with_env = |key: &str, value: &str| RunLaunch {
        env: std::collections::BTreeMap::from([(key.to_owned(), value.to_owned())]),
        ..named("peer")
    };
    for key in [
        "EPR_PEER_TOKEN",
        "db_password",
        "Signing_Private_Key",
        "API_KEY",
    ] {
        let refused = run(&base(), false, with_env(key, "literal")).unwrap_err();
        assert!(
            refused.contains("secrets are referenced by name only"),
            "{key}: {refused}"
        );
    }
    for (key, value) in [("", "x"), ("A=B", "x"), ("A\0", "x"), ("A", "x\0y")] {
        assert!(
            run(&base(), false, with_env(key, value)).is_err(),
            "{key:?}={value:?}"
        );
    }
    for name in ["", "a/b", "my peer", "peer\t", ".."] {
        assert!(run(&base(), false, named(name)).is_err(), "run {name:?}");
        assert!(run(&base(), true, named(name)).is_err(), "stop {name:?}");
    }
    assert!(secret_env_key("my_secret_dir"));
    assert!(!secret_env_key("XDG_CONFIG_HOME"));
}
