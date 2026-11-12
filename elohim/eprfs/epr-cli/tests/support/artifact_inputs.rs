use super::*;
use eprfs_core::{DerivationKey, KeyInput};
use eprfs_peer::{FetchDone, FetchFor, FetchRequest};
use std::time::{Duration, Instant};

fn append_input_record(device: &Device, key: &DerivationKey, owner: &str, local: bool) {
    let mut event = produce_record(
        &eprfs_peer::key_cid(key).unwrap().to_string(),
        &BlobCid::compute_bytes(b"artifact"),
        &BlobCid::compute(b"manifest").to_string(),
        (!local).then_some(owner),
    );
    event.receiver = AgentRef(owner.into());
    event.classified_as.extend([
        format!("env:method={}", key.method),
        format!("env:input={}", key.input.cid()),
    ]);
    SidecarFlowStore::open(device.root())
        .unwrap()
        .append(FlowRecord::Event(event))
        .unwrap();
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
fn inputs_show_and_diff_use_verified_documents_and_only_print_differing_lines() {
    let (_keys, a, b) = household();
    let scratch = TempDir::new().unwrap();
    let (method, descriptor) = method_file(scratch.path(), &["crate"]);
    let computed = elohim_epr_cli::artifact::closure_key(b.root(), &descriptor).unwrap();
    let local = computed.input_lines.unwrap();
    store_lines(&b.payload(), &computed.input, &local);
    append_input_record(&b, &computed.key, &b.key.did_key(), true);
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
    for (lines, exit, expected) in [
        (
            changed.as_str(),
            1,
            format!("-{removed}\n+zz-sibling:changed\n"),
        ),
        (local.as_str(), 0, String::new()),
    ] {
        let remote = DerivationKey {
            method: computed.method.clone(),
            input: KeyInput::Raw(BlobCid::compute_bytes(lines.as_bytes())),
        };
        append_input_record(&b, &remote, &a.key.did_key(), false);
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
        worker.join().unwrap();
        assert_eq!(diff.status.code(), Some(exit), "{}", text(&diff));
        assert_eq!(String::from_utf8(diff.stdout).unwrap(), expected);
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
