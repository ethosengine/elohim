//! Holders, fetch choice and family listing over recorded Produce events: lookup by output and
//! by key, the plurality rule with its divergence record, install refusals and `artifact family`.
use super::*;

#[test]
fn holders_by_output_and_key_share_distinct_counts_and_advisory_warning() {
    let (_keys, a, b) = household();
    let key = BlobCid::compute(b"holder-key").to_string();
    let other_key = BlobCid::compute(b"other-holder-key").to_string();
    let output = BlobCid::compute_bytes(b"holder-output");
    let manifest = BlobCid::compute(b"manifest").to_string();
    let mut store = SidecarFlowStore::open(b.root()).unwrap();
    let row = produce_record(&key, &output, &manifest, Some(&a.key.did_key()));
    store.append(FlowRecord::Event(row.clone())).unwrap();
    let mut repeated = row;
    repeated.occurred_at = "2026-11-09T10:00:00Z".into();
    store.append(FlowRecord::Event(repeated)).unwrap();
    let query = |flag: &str, value: &str, json: bool| {
        let mut args = vec![
            "artifact",
            "holders",
            flag,
            value,
            "--repo",
            b.root().to_str().unwrap(),
        ];
        if json {
            args.push("--json");
        }
        b.epr(&args)
    };
    let by_key = query("--key", &key, true);
    let by_output = query("--output", &output.to_string(), true);
    assert!(by_key.status.success(), "{}", text(&by_key));
    assert!(by_output.status.success(), "{}", text(&by_output));
    let one: serde_json::Value = serde_json::from_slice(&by_key.stdout).unwrap();
    assert_eq!(
        one,
        serde_json::from_slice::<serde_json::Value>(&by_output.stdout).unwrap()
    );
    assert_eq!(one["holders"].as_array().unwrap().len(), 1);
    assert_eq!(one["belowMin"], true);
    let warning = query("--output", &output.to_string(), false);
    assert!(warning.status.success());
    assert_eq!(
        String::from_utf8(warning.stderr).unwrap().trim(),
        format!("warning: 1 holder(s) of {output}, fewer than 2")
    );
    // Local, uncrossed Produce is custody too; another key by the same device is not another holder.
    let mut local = produce_record(&key, &output, &manifest, None);
    local.receiver = AgentRef(b.key.did_key());
    local
        .classified_as
        .retain(|s| !s.starts_with("measure:") && !s.starts_with("reason:"));
    store.append(FlowRecord::Event(local)).unwrap();
    store
        .append(FlowRecord::Event(produce_record(
            &other_key,
            &output,
            &manifest,
            Some(&a.key.did_key()),
        )))
        .unwrap();
    let two = query("--output", &output.to_string(), true);
    let two: serde_json::Value = serde_json::from_slice(&two.stdout).unwrap();
    assert_eq!(two["holders"].as_array().unwrap().len(), 2);
    assert_eq!(two["belowMin"], false);
    for holder in two["holders"].as_array().unwrap() {
        assert_eq!(holder["self"], holder["device"] == b.key.did_key());
    }
    let marked = query("--output", &output.to_string(), false);
    assert!(String::from_utf8_lossy(&marked.stdout)
        .contains(&format!("{} {output} self", b.key.did_key())));
    let by_key = query("--key", &key, true);
    assert_eq!(
        two,
        serde_json::from_slice::<serde_json::Value>(&by_key.stdout).unwrap()
    );
    let quiet = query("--output", &output.to_string(), false);
    assert!(quiet.status.success());
    assert!(!String::from_utf8_lossy(&quiet.stderr).contains("warning:"));
    let higher = b.epr(&[
        "artifact",
        "holders",
        "--output",
        &output.to_string(),
        "--min",
        "3",
        "--repo",
        b.root().to_str().unwrap(),
    ]);
    assert!(higher.status.success());
    assert!(String::from_utf8_lossy(&higher.stderr).contains("fewer than 3"));
    // Later production under the same key retains A's older output in either lookup.
    let newer = BlobCid::compute_bytes(b"newer-holder-output");
    store
        .append(FlowRecord::Event(produce_record(
            &key,
            &newer,
            &manifest,
            Some(&a.key.did_key()),
        )))
        .unwrap();
    let retained = query("--output", &output.to_string(), true);
    assert!(retained.status.success());
    assert_eq!(
        two,
        serde_json::from_slice::<serde_json::Value>(&retained.stdout).unwrap()
    );
    let by_key = query("--key", &key, true);
    let by_key: serde_json::Value = serde_json::from_slice(&by_key.stdout).unwrap();
    assert_eq!(by_key["holders"].as_array().unwrap().len(), 3);
    let absent = BlobCid::compute_bytes(b"absent");
    let zero = query("--output", &absent.to_string(), true);
    assert!(zero.status.success());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&zero.stdout).unwrap()["outputs"][0]["count"],
        0
    );
    assert!(!b
        .epr(&[
            "artifact",
            "holders",
            "--key",
            &key,
            "--output",
            &output.to_string()
        ])
        .status
        .success());
    assert!(!b
        .epr(&["artifact", "holders", "--output", "invalid-cid"])
        .status
        .success());
}

#[test]
fn fetch_plurality_queues_only_chosen_holders_and_records_divergence_once() {
    use eprfs_peer::{FetchDone, FetchFor, FetchRequest};
    use std::time::{Duration, Instant};
    let (keys, a, b) = household();
    let c = Device::new(&keys, "c");
    let d = Device::new(&keys, "d");
    for device in [&c, &d] {
        let request = device_enroll(a.root(), HANDLE, &device.key).unwrap();
        let authorization =
            device_authorize(a.root(), &serde_json::to_string(&request).unwrap(), &a.key).unwrap();
        device_bind(
            a.root(),
            &serde_json::to_string(&authorization).unwrap(),
            &device.key,
        )
        .unwrap();
        device_endpoint(a.root(), None, &device.node_id(), &device.key).unwrap();
    }
    let roster = format!(".eprfs/status/participants/{HANDLE}.jsonl");
    std::fs::copy(a.root().join(&roster), b.root().join(&roster)).unwrap();
    let scratch = TempDir::new().unwrap();
    let (method, descriptor) = method_file(scratch.path(), &["crate"]);
    let key_result = b
        .command()
        .envs(descriptor.non_path_inputs.iter())
        .args([
            "artifact",
            "key",
            "--method",
            method.to_str().unwrap(),
            "--repo",
            b.root().to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();
    assert!(key_result.status.success(), "{}", text(&key_result));
    let key = serde_json::from_slice::<serde_json::Value>(&key_result.stdout).unwrap()["keyCid"]
        .as_str()
        .unwrap()
        .to_string();
    let x = BlobCid::compute_bytes(b"majority");
    let y = BlobCid::compute_bytes(b"minority");
    let manifest = BlobCid::compute(b"manifest");
    let mut store = SidecarFlowStore::open(b.root()).unwrap();
    for (device, output) in [(&a, &x), (&c, &x), (&d, &y)] {
        store
            .append(FlowRecord::Event(produce_record(
                &key,
                output,
                &manifest.to_string(),
                Some(&device.key.did_key()),
            )))
            .unwrap();
    }
    let peer_dir = b.config.path().join("epr/peer");
    let _lock = elohim_epr_cli::peer::ServeLock::acquire(&peer_dir).unwrap();
    let queue = eprfs_peer::fetch_queue_dir(&peer_dir);
    eprfs_peer::ensure_queue_dir(&queue).unwrap();
    let expected_devices: std::collections::BTreeSet<_> = [a.key.did_key(), c.key.did_key()].into();
    let own_devices: std::collections::BTreeSet<_> = [a.key.did_key(), d.key.did_key()].into();
    let expected_requests = [
        (x.clone(), expected_devices.clone(), None),
        (x.clone(), expected_devices.clone(), None),
        (x.clone(), expected_devices, Some(x.clone())),
        (y.clone(), own_devices, None),
    ];
    let later_manifest = manifest.to_string();
    let worker = std::thread::spawn(move || {
        // Exercise the CLI/daemon queue boundary. The peer transport test independently moves
        // actual bytes from this same selection, including proving the divergent holder is idle.
        for (selected, expected_devices, expected_output) in expected_requests {
            let started = Instant::now();
            let (path, request) = loop {
                let found = std::fs::read_dir(&queue)
                    .unwrap()
                    .filter_map(Result::ok)
                    .find_map(|entry| {
                        let path = entry.path();
                        // Only a renamed `<stem>.json` is a queued request. The CLI writes it
                        // through a sibling `<stem>.tmp-…` and renames; claiming (and deleting)
                        // the temporary raced that rename and failed the fetch with ENOENT
                        // whenever the worker scanned between the write and the rename.
                        if path.extension().and_then(|e| e.to_str()) != Some("json") {
                            return None;
                        }
                        let request =
                            serde_json::from_slice::<FetchRequest>(&std::fs::read(&path).ok()?)
                                .ok()?;
                        Some((path, request))
                    });
                if let Some(found) = found {
                    break found;
                }
                assert!(
                    started.elapsed() < Duration::from_secs(30),
                    "request queued"
                );
                std::thread::sleep(Duration::from_millis(20));
            };
            assert_eq!(request.output, selected);
            assert_eq!(request.expect_output, expected_output);
            assert_eq!(
                request
                    .holders
                    .iter()
                    .map(|h| h.device.clone())
                    .collect::<std::collections::BTreeSet<_>>(),
                expected_devices
            );
            std::fs::remove_file(path).unwrap();
            eprfs_peer::write_queue_file(
                &eprfs_peer::fetch_done_path(&queue, &request.stem()),
                &FetchDone::Installed {
                    for_request: FetchFor::of(&request),
                    manifest: manifest.clone(),
                    manifest_from: request.holders[0].device.clone(),
                    fetched_from: vec![request.holders[0].device.clone()],
                    chunks_fetched: 1,
                    chunks_resumed: 0,
                    bytes_fetched: 8,
                    wasted: 0,
                    seconds: 0.01,
                },
            )
            .unwrap();
        }
    });
    for rule in ["plurality", "plurality", "expected", "own"] {
        if rule == "expected" {
            // A's later Y does not erase its X custody: --expect-output X still queues A.
            store
                .append(FlowRecord::Event(produce_record(
                    &key,
                    &y,
                    &later_manifest,
                    Some(&a.key.did_key()),
                )))
                .unwrap();
        }
        if rule == "own" {
            let mut own = produce_record(
                &key,
                &y,
                &BlobCid::compute(b"own-manifest").to_string(),
                None,
            );
            own.receiver = AgentRef(b.key.did_key());
            own.classified_as
                .retain(|slot| !slot.starts_with("measure:") && !slot.starts_with("reason:"));
            store.append(FlowRecord::Event(own)).unwrap();
        }
        let mut command = b.command();
        command.envs(descriptor.non_path_inputs.iter()).args([
            "artifact",
            "fetch",
            "--method",
            method.to_str().unwrap(),
            "--dest",
            scratch.path().join("binary").to_str().unwrap(),
            "--repo",
            b.root().to_str().unwrap(),
        ]);
        if rule == "expected" {
            command.args(["--expect-output", &x.to_string()]);
        }
        let out = command.output().unwrap();
        assert!(out.status.success(), "{}", text(&out));
        let said = String::from_utf8_lossy(&out.stdout);
        assert_eq!(
            said.lines()
                .filter(|line| line.starts_with("divergence:"))
                .count(),
            1
        );
        assert!(said.contains(&format!("rule {rule}")), "{said}");
        let observations: Vec<_> = events(b.root())
            .into_iter()
            .filter(|e| e.classified_as.contains(&"env:kind=divergence".into()))
            .collect();
        assert_eq!(observations.len(), 1);
        let observation = &observations[0];
        assert_eq!(observation.action, ReaVerb::Cite);
        let mut outputs = [x.to_string(), y.to_string()];
        outputs.sort();
        for tag in [
            format!("env:key={key}"),
            format!("env:outputs={}", outputs.join(",")),
            format!("env:chosen={x}"),
            "env:rule=plurality".into(),
        ] {
            assert!(observation.classified_as.contains(&tag), "{tag}");
        }
    }
    worker.join().unwrap();
}

#[test]
fn daemon_install_refusal_records_and_prints_no_divergence() {
    use std::time::{Duration, Instant};
    let (_keys, a, b) = household();
    let scratch = TempDir::new().unwrap();
    let (method, descriptor) = method_file(scratch.path(), &["crate"]);
    let result = b
        .command()
        .envs(descriptor.non_path_inputs.iter())
        .args([
            "artifact",
            "key",
            "--method",
            method.to_str().unwrap(),
            "--repo",
            b.root().to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();
    assert!(result.status.success(), "{}", text(&result));
    let key = serde_json::from_slice::<serde_json::Value>(&result.stdout).unwrap()["keyCid"]
        .as_str()
        .unwrap()
        .to_string();
    let x = BlobCid::compute_bytes(b"remote-output");
    let y = BlobCid::compute_bytes(b"local-only-output");
    let manifest = BlobCid::compute(b"manifest").to_string();
    let mut store = SidecarFlowStore::open(b.root()).unwrap();
    let mut own = produce_record(&key, &y, &manifest, None);
    own.receiver = AgentRef(b.key.did_key());
    own.classified_as
        .retain(|s| !s.starts_with("measure:") && !s.starts_with("reason:"));
    store.append(FlowRecord::Event(own)).unwrap();

    let policy = scratch.path().join("policy.json");
    std::fs::write(
        &policy,
        r#"{"berth":{"channel":{"crosses":{"artifact-produce@1":["produce"]}}}}"#,
    )
    .unwrap();
    let allowed = scratch.path().join("allowed");
    std::fs::create_dir(&allowed).unwrap();
    let log = scratch.path().join("daemon.log");
    // Only this test's child is stopped, including on an assertion failure.
    struct Daemon(std::process::Child);
    impl Drop for Daemon {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let mut daemon = Daemon(
        b.command()
            .args([
                "peer",
                "serve",
                "--root",
                b.root().to_str().unwrap(),
                "--policy",
                policy.to_str().unwrap(),
                "--listen",
                "127.0.0.1:0",
                "--install-root",
                allowed.to_str().unwrap(),
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::fs::File::create(&log).unwrap())
            .spawn()
            .unwrap(),
    );
    let peer_dir = b.config.path().join("epr/peer");
    let started = Instant::now();
    loop {
        assert!(
            daemon.0.try_wait().unwrap().is_none(),
            "{}",
            std::fs::read_to_string(&log).unwrap()
        );
        if eprfs_peer::queue_dir_is_private(&eprfs_peer::fetch_queue_dir(&peer_dir))
            && elohim_epr_cli::peer::ServeLock::held(&peer_dir) == Some(true)
        {
            break;
        }
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "daemon ready: {}",
            std::fs::read_to_string(&log).unwrap()
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    let dest = scratch.path().join("outside");
    let fetch = || {
        b.command()
            .envs(descriptor.non_path_inputs.iter())
            .args([
                "artifact",
                "fetch",
                "--method",
                method.to_str().unwrap(),
                "--repo",
                b.root().to_str().unwrap(),
                "--dest",
                dest.to_str().unwrap(),
                "--stall",
                "1",
                "--total",
                "1",
            ])
            .output()
            .unwrap()
    };
    let local_only = fetch();
    assert!(!local_only.status.success());
    assert!(
        text(&local_only).contains("local-only-custody"),
        "{}",
        text(&local_only)
    );
    assert!(text(&local_only).contains(&y.to_string()));
    // Rule (b) cannot select local-only Y. X is now the remote plurality, but installation refuses.
    store
        .append(FlowRecord::Event(produce_record(
            &key,
            &x,
            &manifest,
            Some(&a.key.did_key()),
        )))
        .unwrap();
    let refused = fetch();
    assert!(!refused.status.success());
    assert!(
        text(&refused).contains("install-refused"),
        "{}",
        text(&refused)
    );
    assert!(
        text(&refused).contains("outside this daemon's install roots"),
        "{}",
        text(&refused)
    );
    assert!(!text(&refused).contains("divergence:"));
    assert!(!dest.exists());
    assert_eq!(
        events(b.root())
            .iter()
            .filter(|e| e.classified_as.contains(&"env:kind=divergence".into()))
            .count(),
        0
    );
}

#[test]
fn artifact_family_lists_v2_v3_keys_requires_and_self_without_selection() {
    let (_keys, a, b) = household();
    let scratch = TempDir::new().unwrap();
    let (_, base) = method_file(scratch.path(), &["crate/src"]);
    let input = eprfs_core::KeyInput::Raw(BlobCid::compute_bytes(b"source"));
    let mut expected = std::collections::BTreeMap::new();
    let mut store = SidecarFlowStore::open(b.root()).unwrap();
    let mut family = None;
    // K2 refuses declared target.* at put until the build applies it (K6). Construct Produce
    // records at the fold boundary to prove family grouping without claiming such builds ran.
    for isa in ["x86-64-v2", "x86-64-v3"] {
        let mut descriptor = base.clone();
        descriptor
            .non_path_inputs
            .insert("target.isa".into(), isa.into());
        let this_family = descriptor.family_cid().unwrap();
        assert_eq!(family.get_or_insert(this_family.clone()), &this_family);
        let key = eprfs_peer::key_cid(&eprfs_core::DerivationKey {
            method: descriptor.cid().unwrap(),
            input: input.clone(),
        })
        .unwrap()
        .to_string();
        let output = BlobCid::compute_bytes(isa.as_bytes());
        let requires = BlobCid::compute_bytes(
            &serde_json::to_vec(&serde_json::json!([{
                "axis": "isa.x86", "predicate": {"kind": "equal", "value": isa},
                "source": "declared"
            }]))
            .unwrap(),
        )
        .to_string();
        let mut row = produce_record(
            &key,
            &output,
            &BlobCid::compute(b"manifest").to_string(),
            Some(&a.key.did_key()),
        );
        row.classified_as.extend([
            format!("env:family={this_family}"),
            format!("env:requires={requires}"),
        ]);
        store.append(FlowRecord::Event(row.clone())).unwrap();
        row.classified_as.retain(|slot| !slot.starts_with("via:"));
        row.receiver = AgentRef(b.key.did_key());
        store.append(FlowRecord::Event(row)).unwrap();
        expected.insert(key, (output.to_string(), requires));
    }
    assert_eq!(expected.len(), 2);
    let family = family.unwrap().to_string();
    let out = b.epr(&[
        "artifact",
        "holders",
        "--family",
        &family,
        "--repo",
        b.root().to_str().unwrap(),
        "--json",
    ]);
    assert!(out.status.success(), "{}", text(&out));
    let result: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let variants = result["variants"].as_array().unwrap();
    assert_eq!(variants.len(), 2);
    for variant in variants {
        let (output, requires) = &expected[variant["key"].as_str().unwrap()];
        assert_eq!(variant["outputs"], serde_json::json!([output]));
        assert_eq!(variant["requires"]["Unanimous"], *requires);
        assert!(variant["claims"]
            .as_array()
            .unwrap()
            .iter()
            .all(|claim| claim["availability"] == "unavailable"));
        let holders = variant["holders"].as_array().unwrap();
        assert_eq!(holders.len(), 2);
        assert_eq!(holders.iter().filter(|h| h["self"] == true).count(), 1);
        for holder in holders {
            assert_eq!(holder["self"], holder["device"] == b.key.did_key());
            assert_eq!(holder["output"], *output);
            assert_eq!(holder["requires"]["Unanimous"], *requires);
        }
    }
    let out = b.epr(&[
        "artifact",
        "holders",
        "--family",
        &family,
        "--repo",
        b.root().to_str().unwrap(),
    ]);
    assert!(out.status.success(), "{}", text(&out));
    assert!(text(&out).contains(" self"));
    for (key, (_, requires)) in expected {
        assert!(text(&out).contains(&format!(
            "key {key} requires Unanimous({requires} (unavailable))"
        )));
    }
}

#[test]
fn artifact_family_preserves_conflicts_older_holders_and_local_availability() {
    let (_keys, a, b) = household();
    let family = BlobCid::compute(b"family").to_string();
    let key = BlobCid::compute(b"key").to_string();
    let output = BlobCid::compute_bytes(b"output");
    let first_bytes = eprfs_peer::Bytes::from_static(b"[1]");
    let first = BlobCid::compute_bytes(&first_bytes);
    let second = BlobCid::compute_bytes(b"[2]");
    let mut store = SidecarFlowStore::open(b.root()).unwrap();
    let mut row = produce_record(
        &key,
        &output,
        &BlobCid::compute(b"manifest").to_string(),
        Some(&a.key.did_key()),
    );
    // B's own pre-K3 custody has neither family nor Requires. A later tags the key.
    let mut older = row.clone();
    older.classified_as.retain(|slot| !slot.starts_with("via:"));
    older.receiver = AgentRef(b.key.did_key());
    store.append(FlowRecord::Event(older)).unwrap();
    row.classified_as.extend([
        format!("env:family={family}"),
        format!("env:requires={first}"),
    ]);
    store.append(FlowRecord::Event(row.clone())).unwrap();
    row.classified_as
        .retain(|slot| !slot.starts_with("env:requires=") && !slot.starts_with("env:family="));
    row.classified_as.push(format!("env:requires={second}"));
    store.append(FlowRecord::Event(row)).unwrap();

    // Only the first contract is present on the querying device. No peer retrieval is involved.
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let store = elohim_epr_cli::payload::open_payload_store(
            &b.payload(),
            elohim_epr_cli::payload::Access::Shared,
        )
        .await
        .unwrap()
        .store;
        store.put_blob_verified(&first, first_bytes).await.unwrap();
    });
    let args = [
        "artifact",
        "holders",
        "--family",
        &family,
        "--repo",
        b.root().to_str().unwrap(),
    ];
    let out = b.epr(&[args.as_slice(), &["--json"]].concat());
    assert!(out.status.success(), "{}", text(&out));
    let result: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let variant = &result["variants"][0];
    let conflicts = variant["requires"]["Conflict"].as_array().unwrap();
    assert_eq!(conflicts.len(), 3);
    assert!(conflicts.contains(&serde_json::json!(first)));
    assert!(conflicts.contains(&serde_json::json!(second)));
    assert!(conflicts.contains(&serde_json::Value::Null));
    let holders = variant["holders"].as_array().unwrap();
    assert_eq!(holders.len(), 2);
    let self_holder = holders
        .iter()
        .find(|h| h["device"] == b.key.did_key())
        .unwrap();
    assert_eq!(self_holder["requires"], "Unclaimed");
    assert_eq!(self_holder["self"], true);
    let remote_holder = holders
        .iter()
        .find(|h| h["device"] == a.key.did_key())
        .unwrap();
    assert_eq!(
        remote_holder["requires"]["Conflict"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let claims = variant["claims"].as_array().unwrap();
    assert_eq!(claims.len(), 3);
    for (cid, status) in [(&first, "local"), (&second, "unavailable")] {
        assert!(claims
            .iter()
            .any(|c| c["requires"] == cid.to_string() && c["availability"] == status));
    }
    let by_key = b.epr(&[
        "artifact",
        "holders",
        "--key",
        &key,
        "--repo",
        b.root().to_str().unwrap(),
        "--json",
    ]);
    assert!(by_key.status.success(), "{}", text(&by_key));
    let by_key: serde_json::Value = serde_json::from_slice(&by_key.stdout).unwrap();
    assert_eq!(by_key["holders"].as_array().unwrap().len(), holders.len());
    for holder in holders {
        assert!(by_key["holders"]
            .as_array()
            .unwrap()
            .iter()
            .any(|h| h["device"] == holder["device"]
                && h["output"] == holder["output"]
                && h["self"] == holder["self"]));
    }
    let out = b.epr(&args);
    assert!(out.status.success(), "{}", text(&out));
    let printed = text(&out);
    assert!(printed.contains("Conflict{"));
    assert!(printed.contains(&format!("{first} (local)")));
    assert!(printed.contains(&format!("{second} (unavailable)")));
    assert!(printed.contains(&format!(
        "{} {} requires Unclaimed self",
        b.key.did_key(),
        output
    )));
}
