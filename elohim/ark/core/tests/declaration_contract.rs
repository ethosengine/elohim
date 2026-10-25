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
            },
        )
        .unwrap(),
        ManifestAct::Run {
            tool: "epr".into(),
            stop: true,
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
