use super::*;
use eprfs_core::{DerivationKey, KeyInput};
use eprfs_peer::{FetchDone, FetchFor, FetchRequest};
use std::time::{Duration, Instant};

fn record_manifest(owner: &str, at: &str, commit: &str) -> BlobCid {
    BlobCid::compute(format!("{owner}:{at}:{commit}").as_bytes())
}

fn input_record(key: &DerivationKey, owner: &str, at: &str, commit: &str) -> FlowEvent {
    let mut event = produce_record(
        &eprfs_peer::key_cid(key).unwrap().to_string(),
        &BlobCid::compute_bytes(b"artifact"),
        &record_manifest(owner, at, commit).to_string(),
        None,
    );
    event.receiver = AgentRef(owner.into());
    event.classified_as.extend([
        format!("env:method={}", key.method),
        format!("env:input={}", key.input.cid()),
    ]);
    event.occurred_at = at.into();
    event
        .classified_as
        .retain(|slot| !slot.starts_with("env:commit="));
    event.classified_as.push(format!("env:commit={commit}"));
    event
}

fn append_local(device: &Device, key: &DerivationKey) {
    SidecarFlowStore::open(device.root())
        .unwrap()
        .append(FlowRecord::Event(input_record(
            key,
            &device.key.did_key(),
            "2026-11-08T10:00:00Z",
            "source",
        )))
        .unwrap();
}

fn fold_inputs(device: &Device, owner: &str, events: Vec<FlowEvent>) {
    let mut channel: Vec<_> = events
        .into_iter()
        .map(|event| eprfs_peer::ChannelEvent {
            cid: atom_cid(&event).unwrap(),
            event,
        })
        .collect();
    channel.sort_by_key(|row| row.cid.to_string());
    let outcome = eprfs_peer::fold_events(
        &SidecarFlowStore::open(device.root()).unwrap(),
        owner,
        &channel,
        &CrossingAllowList::new([("artifact-produce@1", ["produce"])]),
    )
    .unwrap();
    assert_eq!(outcome.appended as usize, channel.len());
}

fn store_lines(path: &Path, input: &BlobCid, lines: &str) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        let store = eprfs_peer::open_payload_store(path).unwrap();
        eprfs_peer::put_input_document(
            store.as_ref(),
            input,
            eprfs_peer::Bytes::from(lines.to_owned()),
        )
        .await
        .unwrap();
    });
}

#[test]
fn inputs_diff_selects_newest_same_commit_in_cid_fold_order_and_prints_provenance() {
    let (_keys, a, b) = household();
    let scratch = TempDir::new().unwrap();
    let (method, descriptor) = method_file(scratch.path(), &["crate"]);
    let computed = elohim_epr_cli::artifact::closure_key(b.root(), &descriptor).unwrap();
    let local = computed.input_lines.unwrap();
    store_lines(&b.payload(), &computed.input, &local);
    append_local(&b, &computed.key);
    // Append order is deliberately opposite occurred_at order.
    SidecarFlowStore::open(b.root())
        .unwrap()
        .append(FlowRecord::Event(input_record(
            &computed.key,
            &b.key.did_key(),
            "2026-11-08T11:00:00Z",
            "newer-source",
        )))
        .unwrap();
    SidecarFlowStore::open(b.root())
        .unwrap()
        .append(FlowRecord::Event(input_record(
            &computed.key,
            &b.key.did_key(),
            "2026-11-08T09:00:00Z",
            "oldest-source",
        )))
        .unwrap();

    // Equal occurred_at instants use ascending commit/manifest/output, not append order.
    SidecarFlowStore::open(b.root())
        .unwrap()
        .append(FlowRecord::Event(input_record(
            &computed.key,
            &b.key.did_key(),
            "2026-11-08T13:00:00+02:00",
            "z-source",
        )))
        .unwrap();

    for subject in [method.to_str().unwrap(), &computed.key_cid.to_string()] {
        let shown = b.epr(&[
            "artifact",
            "inputs",
            "show",
            subject,
            "--repo",
            b.root().to_str().unwrap(),
        ]);
        assert!(shown.status.success(), "{}", text(&shown));
        assert_eq!(shown.stdout, local.as_bytes());
    }
    let absent = b.epr(&[
        "artifact",
        "inputs",
        "show",
        &BlobCid::compute(b"absent").to_string(),
        "--repo",
        b.root().to_str().unwrap(),
    ]);
    assert_eq!(absent.status.code(), Some(3));
    let peer_dir = b.config.path().join("epr/peer");
    let _lock = elohim_epr_cli::peer::ServeLock::acquire(&peer_dir).unwrap();
    let queue = eprfs_peer::fetch_queue_dir(&peer_dir);
    eprfs_peer::ensure_queue_dir(&queue).unwrap();
    // One removed and one added line: exactly two lines of output, with no context.
    let removed = local.lines().next().unwrap();
    let changed = local.replacen(removed, "zz-sibling:changed", 1);
    for (index, (lines, exit, expected)) in [
        (
            changed.as_str(),
            1,
            format!("-{removed}\n+zz-sibling:changed\n"),
        ),
        (local.as_str(), 0, String::new()),
        (local.as_str(), 0, String::new()),
    ]
    .into_iter()
    .enumerate()
    {
        let remote = DerivationKey {
            method: computed.method.clone(),
            input: KeyInput::Raw(BlobCid::compute_bytes(lines.as_bytes())),
        };
        let at = format!("2026-11-08T12:00:0{index}Z");
        let commit = if index == 2 {
            "literal-other-source"
        } else if index == 1 {
            "newer-source"
        } else {
            "source"
        };
        let mut newest = input_record(&remote, &a.key.did_key(), &at, commit);
        // Find an older event sorting AFTER the newest one, as an initial channel sync
        // does. The older timestamp even sorts lexically later, because of its offset.
        let mut older = None;
        for nonce in 0..1000 {
            newest
                .classified_as
                .retain(|slot| !slot.starts_with("env:fixtureNonce="));
            newest
                .classified_as
                .push(format!("env:fixtureNonce={nonce}"));
            let newest_cid = atom_cid(&newest).unwrap().to_string();
            let other = DerivationKey {
                method: computed.method.clone(),
                input: KeyInput::Raw(BlobCid::compute_bytes(
                    format!("older {index} {nonce}").as_bytes(),
                )),
            };
            let event = input_record(
                &other,
                &a.key.did_key(),
                "2026-11-08T13:00:00+02:00",
                "source",
            );
            if atom_cid(&event).unwrap().to_string() > newest_cid {
                older = Some(event);
                break;
            }
        }
        let other_source = input_record(
            &computed.key,
            &a.key.did_key(),
            "2026-11-09T00:00:00Z",
            &format!("other-source-{index}"),
        );
        fold_inputs(
            &b,
            &a.key.did_key(),
            vec![newest, older.expect("CID order inversion"), other_source],
        );
        if index == 2 {
            fold_inputs(
                &b,
                &a.key.did_key(),
                vec![input_record(
                    &remote,
                    &a.key.did_key(),
                    "2026-11-10T00:00:00Z",
                    commit,
                )],
            );
        }
        let expected_manifest = record_manifest(
            &a.key.did_key(),
            if index == 2 {
                "2026-11-10T00:00:00Z"
            } else {
                &at
            },
            commit,
        );
        let requested_manifest = expected_manifest.clone();
        let (queue, payload, remote_lines, did, remote_key) = (
            queue.clone(),
            b.payload(),
            lines.to_owned(),
            a.key.did_key(),
            remote.clone(),
        );
        let worker = std::thread::spawn(move || {
            let started = Instant::now();
            let (path, request) = loop {
                let found = std::fs::read_dir(&queue)
                    .unwrap()
                    .filter_map(Result::ok)
                    .find_map(|entry| {
                        let path = entry.path();
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
                assert!(started.elapsed() < Duration::from_secs(30));
                std::thread::sleep(Duration::from_millis(20));
            };
            assert!(request.inputs_only);
            assert_eq!(request.authority.key(), Some(&remote_key));
            assert_eq!(request.holders.len(), 1);
            assert_eq!(request.holders[0].device, did);
            assert_eq!(request.holders[0].manifest, requested_manifest);
            assert!(request.dest.as_os_str().is_empty());
            std::fs::remove_file(path).unwrap();
            store_lines(&payload, remote_key.input.cid(), &remote_lines);
            eprfs_peer::write_queue_file(
                &eprfs_peer::fetch_done_path(&queue, &request.stem()),
                &FetchDone::InputDocument {
                    for_request: FetchFor::of(&request),
                    cid: remote_key.input.cid().clone(),
                },
            )
            .unwrap();
        });
        let literal = computed.key_cid.to_string();
        let subject = if index == 2 {
            &literal
        } else {
            method.to_str().unwrap()
        };
        let diff = b.epr(&[
            "artifact",
            "inputs",
            "diff",
            subject,
            "--device",
            &a.key.did_key(),
            "--repo",
            b.root().to_str().unwrap(),
        ]);
        worker.join().unwrap();
        assert_eq!(diff.status.code(), Some(exit), "{}", text(&diff));
        assert_eq!(String::from_utf8_lossy(&diff.stdout), expected);
        let stderr = String::from_utf8_lossy(&diff.stderr);
        for line in [
            format!("inputs diff method={}", computed.method),
            format!(
                "this device: device={} key={} input={} commit={} manifest={}",
                b.key.did_key(),
                computed.key_cid,
                computed.input,
                if index == 0 { "source" } else { "newer-source" },
                record_manifest(
                    &b.key.did_key(),
                    if index == 0 {
                        "2026-11-08T10:00:00Z"
                    } else {
                        "2026-11-08T11:00:00Z"
                    },
                    if index == 0 { "source" } else { "newer-source" }
                )
            ),
            format!(
                "sibling: device={} key={} input={} commit={commit} manifest={}",
                a.key.did_key(),
                eprfs_peer::key_cid(&remote).unwrap(),
                remote.input.cid(),
                expected_manifest
            ),
        ] {
            assert!(stderr.lines().any(|actual| actual == line), "{stderr}");
        }
    }
    let refused = b.epr(&[
        "artifact",
        "inputs",
        "diff",
        method.to_str().unwrap(),
        "--device",
        "did:key:stranger",
        "--repo",
        b.root().to_str().unwrap(),
    ]);
    assert_eq!(refused.status.code(), Some(3));
}

#[test]
fn inputs_diff_without_same_commit_refuses_with_five_candidates_and_literal_key_hint() {
    let (_keys, a, b) = household();
    let scratch = TempDir::new().unwrap();
    let (method, descriptor) = method_file(scratch.path(), &["crate"]);
    let computed = elohim_epr_cli::artifact::closure_key(b.root(), &descriptor).unwrap();
    store_lines(
        &b.payload(),
        &computed.input,
        computed.input_lines.as_ref().unwrap(),
    );
    append_local(&b, &computed.key);
    SidecarFlowStore::open(b.root())
        .unwrap()
        .append(FlowRecord::Event(input_record(
            &computed.key,
            &b.key.did_key(),
            "2026-11-08T11:00:00Z",
            "newer-source",
        )))
        .unwrap();

    let mut events = Vec::new();
    let mut expected = Vec::new();
    for i in 0..7 {
        let remote = DerivationKey {
            method: computed.method.clone(),
            input: KeyInput::Raw(BlobCid::compute_bytes(format!("other {i}").as_bytes())),
        };
        let commit = format!("other-commit-{i}");
        let at = format!("2026-11-09T00:00:0{i}Z");
        expected.push(format!(
            "input={} commit={commit} time={at}",
            remote.input.cid()
        ));
        events.push(input_record(&remote, &a.key.did_key(), &at, &commit));
    }
    fold_inputs(&b, &a.key.did_key(), events);
    let diff = b.epr(&[
        "artifact",
        "inputs",
        "diff",
        method.to_str().unwrap(),
        "--device",
        &a.key.did_key(),
        "--repo",
        b.root().to_str().unwrap(),
    ]);
    assert_eq!(diff.status.code(), Some(3), "{}", text(&diff));
    assert!(diff.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&diff.stderr);
    assert!(
        stderr.contains("no same-source sibling record for commit newer-source"),
        "{stderr}"
    );
    assert!(stderr.contains("pass a literal key"), "{stderr}");
    expected.sort();
    assert_eq!(
        stderr
            .lines()
            .filter(|line| line.starts_with("input="))
            .collect::<Vec<_>>(),
        expected
            .iter()
            .take(5)
            .map(String::as_str)
            .collect::<Vec<_>>()
    );
}

#[test]
fn inputs_diff_equal_produce_times_with_different_inputs_refuses_both_candidates() {
    let (_keys, a, b) = household();
    let scratch = TempDir::new().unwrap();
    let (method, descriptor) = method_file(scratch.path(), &["crate"]);
    let computed = elohim_epr_cli::artifact::closure_key(b.root(), &descriptor).unwrap();
    store_lines(
        &b.payload(),
        &computed.input,
        computed.input_lines.as_ref().unwrap(),
    );
    append_local(&b, &computed.key);
    let remote = DerivationKey {
        method: computed.method.clone(),
        input: KeyInput::Raw(BlobCid::compute_bytes(b"different input")),
    };
    fold_inputs(
        &b,
        &a.key.did_key(),
        vec![
            input_record(
                &computed.key,
                &a.key.did_key(),
                "2026-11-08T12:00:00Z",
                "source",
            ),
            input_record(
                &remote,
                &a.key.did_key(),
                "2026-11-08T14:00:00+02:00",
                "source",
            ),
        ],
    );
    let diff = b.epr(&[
        "artifact",
        "inputs",
        "diff",
        method.to_str().unwrap(),
        "--device",
        &a.key.did_key(),
        "--repo",
        b.root().to_str().unwrap(),
    ]);
    assert_eq!(diff.status.code(), Some(3), "{}", text(&diff));
    assert!(diff.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&diff.stderr);
    assert!(
        stderr.contains("ambiguous newest same-source Produce records"),
        "{stderr}"
    );
    for (key, at) in [
        (&computed.key, "2026-11-08T12:00:00Z"),
        (&remote, "2026-11-08T14:00:00+02:00"),
    ] {
        assert!(
            stderr.contains(&format!(
                "input={} commit=source time={at}",
                key.input.cid()
            )),
            "{stderr}"
        );
    }
}
