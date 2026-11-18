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

fn store_method(path: &Path, descriptor: &MethodDescriptor) -> Vec<u8> {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        let store = eprfs_peer::open_payload_store(path).unwrap();
        let cid = eprfs_peer::put_method_record(store.as_ref(), descriptor)
            .await
            .unwrap();
        store
            .fetch_blob(&cid, eprfs_core::FetchPolicy::LocalOnly)
            .await
            .unwrap()
            .bytes
            .to_vec()
    })
}

fn fixture_cid(seed: &str) -> String {
    BlobCid::compute_bytes(seed.as_bytes()).to_string()
}

fn libm() -> String {
    format!(
        "link:libm.so\t/usr/lib64/libm.so\t{}\n",
        fixture_cid("libm")
    )
}

fn libc() -> String {
    format!(
        "link:libc.so\t/usr/lib64/libc.so\t{}\n",
        fixture_cid("libc")
    )
}

/// Two synthetic recordings of one crate: the sibling's PATH lists the same directories in
/// another order, its rustc reports another LLVM version, its crt1.o resolves elsewhere with equal
/// content (unresolved: a path can reach DT_NEEDED), and its libm/libc probe lines swap places.
fn two_recordings() -> (MethodDescriptor, String, MethodDescriptor, String) {
    let scratch = TempDir::new().unwrap();
    let (_, mut this) = method_file_with(
        scratch.path(),
        &["crate"],
        &[("PATH", "/opt/rust/bin:/usr/bin:/bin")],
    );
    this.tools.insert("cargo".into(), "cargo 1.96.1".into());
    let mut sibling = this.clone();
    sibling
        .non_path_inputs
        .insert("PATH".into(), "/usr/bin:/opt/rust/bin:/bin".into());
    sibling.tools.insert(
        "rustc".into(),
        fixture::RUSTC_IDENTITY.replace("LLVM version: 22.1.2", "LLVM version: 22.1.3"),
    );
    let source = "crate\t040000 tree 1111111111111111111111111111111111111111\n";
    let tool = format!(
        "tool:link-driver\t/usr/bin/gcc\t{}\n",
        fixture_cid("version")
    );
    let this_lines = format!(
        "{source}{tool}link:crt1.o\t/usr/lib64/crt1.o\t{}\n{}{}",
        fixture_cid("crt1"),
        libm(),
        libc()
    );
    let sibling_lines = format!(
        "{source}{tool}link:crt1.o\t/lib/x86_64-linux-gnu/crt1.o\t{}\n{}{}",
        fixture_cid("crt1"),
        libc(),
        libm()
    );
    (this, this_lines, sibling, sibling_lines)
}

fn key_of(descriptor: &MethodDescriptor, lines: &str) -> DerivationKey {
    DerivationKey {
        method: descriptor.cid().unwrap(),
        input: KeyInput::Raw(BlobCid::compute_bytes(lines.as_bytes())),
    }
}

#[test]
fn inputs_diff_two_keys_classifies_each_method_field_and_ordered_input_line() {
    let (_keys, a, b) = household();
    let (this, this_lines, sibling, sibling_lines) = two_recordings();
    let (this_key, sibling_key) = (key_of(&this, &this_lines), key_of(&sibling, &sibling_lines));
    for (descriptor, lines, key) in [
        (&this, &this_lines, &this_key),
        (&sibling, &sibling_lines, &sibling_key),
    ] {
        store_method(&b.payload(), descriptor);
        store_lines(&b.payload(), key.input.cid(), lines);
    }
    append_local(&b, &this_key);
    fold_inputs(
        &b,
        &a.key.did_key(),
        vec![input_record(
            &sibling_key,
            &a.key.did_key(),
            "2026-11-08T12:00:00Z",
            "source",
        )],
    );
    let (this_cid, sibling_cid) = (
        eprfs_peer::key_cid(&this_key).unwrap().to_string(),
        eprfs_peer::key_cid(&sibling_key).unwrap().to_string(),
    );
    let root = b.root().to_str().unwrap();
    let diff = |extra: &[&str], left: &str, right: &str| {
        let mut args = vec!["artifact", "inputs", "diff", left, right, "--repo", root];
        args.extend_from_slice(extra);
        b.epr(&args)
    };

    let json = diff(&["--json"], &this_cid, &sibling_cid);
    assert_eq!(json.status.code(), Some(1), "{}", text(&json));
    let report: serde_json::Value = serde_json::from_slice(&json.stdout).unwrap();
    assert_eq!(report["a"]["own"], true);
    assert_eq!(report["b"]["own"], false);
    assert_eq!(report["b"]["device"], a.key.did_key());
    assert_eq!(report["identical"], false);
    assert_eq!(report["orderOnly"], false);
    let method = report["method"].as_array().unwrap();
    let field = |name: &str| {
        method
            .iter()
            .find(|delta| delta["field"] == name)
            .unwrap_or_else(|| panic!("no {name} in {method:#?}"))
    };
    let path = field("nonPathInputs.PATH order");
    assert_eq!(path["change"], "order-only");
    assert_eq!(path["class"], "unresolved");
    assert_eq!(path["a"], "/opt/rust/bin:/usr/bin:/bin");
    assert_eq!(path["b"], "/usr/bin:/opt/rust/bin:/bin");
    let llvm = field("tools.rustc[LLVM version]");
    assert_eq!(llvm["class"], "byte-affecting");
    assert_eq!(llvm["a"], "22.1.2");
    assert_eq!(llvm["b"], "22.1.3");
    assert_eq!(method.len(), 2, "{method:#?}");
    let inputs = report["inputs"].as_array().unwrap();
    let crt1 = inputs.iter().find(|d| d["label"] == "link:crt1.o").unwrap();
    assert_eq!(crt1["change"], "changed");
    assert_eq!(crt1["class"], "unresolved");
    assert_eq!(
        crt1["b"]["line"],
        format!(
            "link:crt1.o\t/lib/x86_64-linux-gnu/crt1.o\t{}",
            fixture_cid("crt1")
        )
    );
    let moved: Vec<_> = inputs
        .iter()
        .filter(|d| d["change"] == "order-only")
        .collect();
    assert_eq!(moved.len(), 1, "{inputs:#?}");
    assert_eq!(moved[0]["class"], "unresolved");
    assert_ne!(moved[0]["a"]["position"], moved[0]["b"]["position"]);
    assert_eq!(inputs.len(), 2, "{inputs:#?}");

    let plain = diff(&[], &this_cid, &sibling_cid);
    assert_eq!(plain.status.code(), Some(1), "{}", text(&plain));
    let stdout = String::from_utf8_lossy(&plain.stdout);
    for line in [
        "method tools.rustc[LLVM version] changed byte-affecting: ".to_owned(),
        "  - 22.1.2".to_owned(),
        "  + 22.1.3".to_owned(),
        "method nonPathInputs.PATH order order-only unresolved: ".to_owned(),
        "input link:crt1.o changed unresolved: ".to_owned(),
        format!(
            "  + [3] link:crt1.o\t/lib/x86_64-linux-gnu/crt1.o\t{}",
            fixture_cid("crt1")
        ),
        "summary: method 2 (byte-affecting 1, key-only 0, unresolved 1); inputs 2 \
         (byte-affecting 0, key-only 0, unresolved 2)"
            .to_owned(),
    ] {
        assert!(
            stdout.lines().any(|l| l.starts_with(&line)),
            "{line}\n{stdout}"
        );
    }
    let stderr = String::from_utf8_lossy(&plain.stderr);
    assert!(
        stderr.contains(&format!("b: device={} key={sibling_cid}", a.key.did_key())),
        "{stderr}"
    );

    // An order-only document pair is reported as such.
    let only_order = this_lines.replace(
        &format!("{}{}", libm(), libc()),
        &format!("{}{}", libc(), libm()),
    );
    let reordered = key_of(&this, &only_order);
    store_lines(&b.payload(), reordered.input.cid(), &only_order);
    fold_inputs(
        &b,
        &a.key.did_key(),
        vec![input_record(
            &reordered,
            &a.key.did_key(),
            "2026-11-08T12:30:00Z",
            "source",
        )],
    );
    let reordered_cid = eprfs_peer::key_cid(&reordered).unwrap().to_string();
    let order = diff(&["--json"], &this_cid, &reordered_cid);
    assert_eq!(order.status.code(), Some(1), "{}", text(&order));
    let report: serde_json::Value = serde_json::from_slice(&order.stdout).unwrap();
    assert_eq!(report["orderOnly"], true);
    assert_eq!(report["method"].as_array().unwrap().len(), 0);

    let same = diff(&[], &this_cid, &this_cid);
    assert_eq!(same.status.code(), Some(0), "{}", text(&same));

    // A sibling whose method document is not held refuses, naming it and the cure.
    let mut unheld = sibling.clone();
    unheld.flags.push("--features=unheld".into());
    let unheld_key = key_of(&unheld, &sibling_lines);
    fold_inputs(
        &b,
        &a.key.did_key(),
        vec![input_record(
            &unheld_key,
            &a.key.did_key(),
            "2026-11-08T13:00:00Z",
            "source",
        )],
    );
    let unheld_cid = eprfs_peer::key_cid(&unheld_key).unwrap().to_string();
    let refused = diff(&[], &this_cid, &unheld_cid);
    assert_eq!(refused.status.code(), Some(3), "{}", text(&refused));
    assert!(refused.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&refused.stderr);
    assert!(
        stderr.contains(&format!("b's method document {}", unheld_key.method)),
        "{stderr}"
    );
    assert!(
        stderr.contains("no read path draws a method document"),
        "{stderr}"
    );
    assert!(stderr.contains("--documents DIR"), "{stderr}");

    // --partial still compares what is held, and still exits 3.
    let partial = diff(&["--partial", "--json"], &this_cid, &unheld_cid);
    assert_eq!(partial.status.code(), Some(3), "{}", text(&partial));
    let report: serde_json::Value = serde_json::from_slice(&partial.stdout).unwrap();
    assert!(report["method"].is_null());
    assert_eq!(report["inputs"].as_array().unwrap().len(), 2);
    assert_eq!(report["missing"].as_array().unwrap().len(), 1);

    // A carried copy that fails its CID refuses; the true bytes are accepted.
    let documents = TempDir::new().unwrap();
    let carried = documents.path().join(unheld_key.method.to_string());
    std::fs::write(&carried, b"not the method document").unwrap();
    let docs = documents.path().to_str().unwrap();
    let tampered = diff(&["--documents", docs], &this_cid, &unheld_cid);
    assert_eq!(tampered.status.code(), Some(3), "{}", text(&tampered));
    assert!(
        String::from_utf8_lossy(&tampered.stderr).contains("fails its CID"),
        "{}",
        text(&tampered)
    );
    let elsewhere = TempDir::new().unwrap();
    std::fs::write(
        &carried,
        store_method(&elsewhere.path().join("payload"), &unheld),
    )
    .unwrap();
    let carried_diff = diff(&["--documents", docs, "--json"], &this_cid, &unheld_cid);
    assert_eq!(
        carried_diff.status.code(),
        Some(1),
        "{}",
        text(&carried_diff)
    );
    let report: serde_json::Value = serde_json::from_slice(&carried_diff.stdout).unwrap();
    assert_eq!(report["b"]["methodDocument"], carried.display().to_string());
    assert!(report["method"]
        .as_array()
        .unwrap()
        .iter()
        .any(|d| d["field"] == "flags[--features]" && d["class"] == "byte-affecting"));

    // A carried document is read only as a regular file, never through a symlink or a FIFO,
    // and never past the document cap.
    let refused_carry = |prepare: &dyn Fn(&Path)| {
        let dir = TempDir::new().unwrap();
        prepare(&dir.path().join(unheld_key.method.to_string()));
        let out = diff(
            &["--documents", dir.path().to_str().unwrap()],
            &this_cid,
            &unheld_cid,
        );
        assert_eq!(out.status.code(), Some(3), "{}", text(&out));
        text(&out)
    };
    let symlinked = refused_carry(&|path| std::os::unix::fs::symlink(&carried, path).unwrap());
    assert!(symlinked.contains("is a symbolic link"), "{symlinked}");
    let fifo = refused_carry(&|path| {
        let made = std::process::Command::new("mkfifo")
            .arg(path)
            .status()
            .unwrap();
        assert!(made.success());
    });
    assert!(fifo.contains("is not a regular file"), "{fifo}");
    let oversized = refused_carry(&|path| {
        std::fs::File::create(path)
            .unwrap()
            .set_len(8 * 1024 * 1024 + 1)
            .unwrap();
    });
    assert!(oversized.contains("document cap"), "{oversized}");

    // A draw that fails under --partial is kept as the input gap; the held methods still compare.
    let unfetched_lines = format!("{sibling_lines}extra\t-\n");
    let unfetched = key_of(&sibling, &unfetched_lines);
    fold_inputs(
        &b,
        &a.key.did_key(),
        vec![input_record(
            &unfetched,
            &a.key.did_key(),
            "2026-11-08T14:00:00Z",
            "source",
        )],
    );
    let unfetched_cid = eprfs_peer::key_cid(&unfetched).unwrap().to_string();
    let fetched = diff(
        &["--fetch", "--partial", "--json"],
        &this_cid,
        &unfetched_cid,
    );
    assert_eq!(fetched.status.code(), Some(3), "{}", text(&fetched));
    let report: serde_json::Value = serde_json::from_slice(&fetched.stdout).unwrap();
    assert_eq!(report["method"].as_array().unwrap().len(), 2, "{report:#}");
    assert!(report["inputs"].is_null());
    let gap = report["missing"][0].as_str().unwrap();
    assert!(gap.contains("--fetch did not draw it"), "{gap}");
    let strict = diff(&["--fetch"], &this_cid, &unfetched_cid);
    assert_eq!(strict.status.code(), Some(3), "{}", text(&strict));
    assert!(strict.stdout.is_empty());

    // An empty payload directory is no store: the diagnostic leaves it empty.
    let empty = TempDir::new().unwrap();
    let blank = diff(
        &["--payload-store", empty.path().to_str().unwrap()],
        &this_cid,
        &sibling_cid,
    );
    assert_eq!(blank.status.code(), Some(3), "{}", text(&blank));
    assert_eq!(std::fs::read_dir(empty.path()).unwrap().count(), 0);

    // A key with no record is refused, and --device is not part of the two-key form.
    let absent = BlobCid::compute(b"absent").to_string();
    let none = diff(&[], &this_cid, &absent);
    assert_eq!(none.status.code(), Some(3), "{}", text(&none));
    assert!(String::from_utf8_lossy(&none.stderr).contains("no Produce record for key"));
    let device = diff(&["--device", &a.key.did_key()], &this_cid, &sibling_cid);
    assert_ne!(device.status.code(), Some(0));
    assert!(
        text(&device).contains("takes no --device"),
        "{}",
        text(&device)
    );
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
