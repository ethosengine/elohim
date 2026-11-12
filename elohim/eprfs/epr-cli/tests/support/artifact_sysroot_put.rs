//! `epr artifact sysroot put`: the recorded floor fetched by output CID under explicit authority,
//! the reserved archive method, and the bounded archive read.
use super::*;

#[test]
fn artifact_sysroot_put_then_output_fetch_uses_explicit_cid_authority() {
    sysroot_put_then_output_fetch(false);
}

#[test]
fn artifact_sysroot_put_records_family_and_canonical_requires() {
    sysroot_put_then_output_fetch(true);
}

fn sysroot_put_then_output_fetch(with_requires: bool) {
    use eprfs_peer::{FetchAuthority, FetchDone, FetchFor, FetchRequest};
    use std::time::{Duration, Instant};
    let (_keys, a, b) = household();
    let scratch = TempDir::new().unwrap();
    let input = scratch.path().join("input");
    std::fs::create_dir(&input).unwrap();
    std::fs::write(input.join("header.h"), b"/* content-pinned header */").unwrap();
    let archive = scratch.path().join("sysroot.eprsys");
    let packed = a.epr(&[
        "artifact",
        "sysroot",
        "pack",
        "--from",
        input.to_str().unwrap(),
        "--triple",
        "x86_64-unknown-linux-gnu",
        "--out",
        archive.to_str().unwrap(),
        "--json",
    ]);
    assert!(packed.status.success(), "{}", text(&packed));
    let bytes = std::fs::read(&archive).unwrap();
    let output = BlobCid::compute_bytes(&bytes);
    let (stub, log) = brit_stub(scratch.path());
    let requires_file = scratch.path().join("requires.json");
    let canonical_requires = br#"[{"axis":"isa.x86","predicate":{"kind":"equal","value":"x86-64-v3"},"source":"declared"}]"#;
    std::fs::write(
        &requires_file,
        br#"[ { "source": "declared", "predicate": { "value": "x86-64-v3", "kind": "equal" }, "axis": "isa.x86" } ]"#,
    )
    .unwrap();
    let requires_cid = BlobCid::compute_bytes(canonical_requires);
    let mut command = a.command();
    command.env("BRIT_BUILD_REF_BIN", &stub).args([
        "artifact",
        "sysroot",
        "put",
        "--archive",
        archive.to_str().unwrap(),
        "--repo",
        a.root().to_str().unwrap(),
        "--cross",
        "--json",
    ]);
    if with_requires {
        command.arg("--requires").arg(&requires_file);
    }
    let put = command.output().unwrap();
    assert!(put.status.success(), "{}", text(&put));
    assert!(String::from_utf8_lossy(&put.stderr).starts_with("reach: pending, not enforced\n"));
    let recorded: serde_json::Value = serde_json::from_slice(&put.stdout).unwrap();
    assert_eq!(recorded["output"], output.to_string());
    let verified = elohim_epr_cli::artifact::sysroot::verify(&bytes).unwrap();
    assert_eq!(recorded["input"], verified.cid().unwrap().to_string());
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let family = runtime.block_on(async {
        let store = eprfs_peer::open_payload_store(&a.payload()).unwrap();
        let held = store
            .fetch_blob(&verified.cid().unwrap(), eprfs_core::FetchPolicy::LocalOnly)
            .await
            .unwrap();
        assert_eq!(held.bytes.as_ref(), verified.canonical_bytes().unwrap());
        let method_cid = BlobCid::parse(recorded["method"].as_str().unwrap()).unwrap();
        let method_bytes = store
            .fetch_blob(&method_cid, eprfs_core::FetchPolicy::LocalOnly)
            .await
            .unwrap();
        let method: MethodDescriptor =
            serde::Deserialize::deserialize(elohim_epr::cbor::decode(&method_bytes.bytes).unwrap())
                .unwrap();
        assert_eq!(method.name, "sysroot-pack");
        assert_eq!(method.cid().unwrap(), method_cid);
        if with_requires {
            let held = store
                .fetch_blob(&requires_cid, eprfs_core::FetchPolicy::LocalOnly)
                .await
                .unwrap();
            assert_eq!(held.bytes.as_ref(), canonical_requires);
        }
        method.family_cid().unwrap()
    });
    assert!(std::fs::read_to_string(log)
        .unwrap()
        .contains("--step sysroot-pack:sysroot"));
    let manifest = BlobCid::parse(recorded["manifest"].as_str().unwrap()).unwrap();
    let produces: Vec<_> = events(a.root())
        .into_iter()
        .filter(|e| e.action == ReaVerb::Produce)
        .collect();
    assert_eq!(produces.len(), 1);
    let gaps: Vec<_> = produces[0]
        .classified_as
        .iter()
        .filter(|s| s.starts_with("env:gaps="))
        .map(String::as_str)
        .collect();
    assert_eq!(gaps, ["env:gaps=reach"]);

    assert!(produces[0]
        .classified_as
        .contains(&format!("env:family={family}")));
    let requires_slots: Vec<_> = produces[0]
        .classified_as
        .iter()
        .filter(|slot| slot.starts_with("env:requires="))
        .cloned()
        .collect();
    assert_eq!(
        requires_slots,
        if with_requires {
            vec![format!("env:requires={requires_cid}")]
        } else {
            Vec::<String>::new()
        }
    );
    assert!(produces[0]
        .classified_as
        .contains(&"basis-key:sysroot-manifest".into()));
    let mut crossed = produces[0].clone();
    crossed
        .classified_as
        .push(format!("via:{}", a.key.did_key()));
    SidecarFlowStore::open(b.root())
        .unwrap()
        .append(FlowRecord::Event(crossed))
        .unwrap();
    let peer_dir = b.config.path().join("epr/peer");
    let _lock = elohim_epr_cli::peer::ServeLock::acquire(&peer_dir).unwrap();
    let queue = eprfs_peer::fetch_queue_dir(&peer_dir);
    eprfs_peer::ensure_queue_dir(&queue).unwrap();
    let expected = output.clone();
    let holder = a.key.did_key();
    let expected_bytes = bytes.clone();
    let worker = std::thread::spawn(move || {
        // This fixture tests CLI/custody/queue integration. The peer suite separately exercises
        // real tier method verification, bounded transfer, whole-blob verification and install.
        let started = Instant::now();
        let (path, request) = loop {
            let found = std::fs::read_dir(&queue)
                .unwrap()
                .filter_map(Result::ok)
                .find_map(|entry| {
                    let path = entry.path();
                    if path.extension().and_then(|x| x.to_str()) != Some("json") {
                        return None;
                    }
                    let request: FetchRequest =
                        serde_json::from_slice(&std::fs::read(&path).ok()?).ok()?;
                    if path != eprfs_peer::fetch_request_path(&queue, &request.stem()) {
                        return None;
                    }
                    Some((path, request))
                });
            if let Some(found) = found {
                break found;
            }
            assert!(started.elapsed() < Duration::from_secs(30));
            std::thread::sleep(Duration::from_millis(20));
        };
        assert_eq!(
            request.authority,
            FetchAuthority::Output {
                cid: expected.clone()
            }
        );
        request.authority.validate(&request.output).unwrap();
        assert_eq!(request.output, expected);
        assert_eq!(request.holders.len(), 1);
        assert_eq!(request.holders[0].device, holder);
        assert_eq!(request.holders[0].manifest, manifest);
        assert_eq!(BlobCid::compute_bytes(&expected_bytes), request.output);
        let stage = request.dest.with_extension("stage");
        std::fs::write(&stage, expected_bytes).unwrap();
        std::fs::rename(stage, &request.dest).unwrap();
        std::fs::remove_file(path).unwrap();
        eprfs_peer::write_queue_file(
            &eprfs_peer::fetch_done_path(&queue, &request.stem()),
            &FetchDone::Installed {
                for_request: FetchFor::of(&request),
                manifest,
                manifest_from: holder.clone(),
                fetched_from: vec![holder],
                chunks_fetched: 1,
                chunks_resumed: 0,
                bytes_fetched: bytes.len() as u64,
                wasted: 0,
                seconds: 0.01,
            },
        )
        .unwrap();
    });
    let dest = scratch.path().join("fetched.eprsys");
    let fetch = |cid: &BlobCid| {
        b.epr(&[
            "artifact",
            "fetch",
            "--output",
            &cid.to_string(),
            "--dest",
            dest.to_str().unwrap(),
            "--repo",
            b.root().to_str().unwrap(),
            "--json",
        ])
    };
    let result = fetch(&output);
    worker.join().unwrap();
    assert!(result.status.success(), "{}", text(&result));
    let answer: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert!(answer["keyCid"].is_null());
    assert_eq!(answer["authority"]["kind"], "output");
    assert_eq!(
        BlobCid::compute_bytes(&std::fs::read(&dest).unwrap()),
        output
    );
    let missing = fetch(&BlobCid::compute_bytes(b"unheld"));
    assert_eq!(missing.status.code(), Some(3));
    assert!(
        text(&missing).contains("expected-output-unavailable"),
        "{}",
        text(&missing)
    );
}

#[test]
fn artifact_executable_put_refuses_reserved_archive_method_before_recording() {
    let keys = TempDir::new().unwrap();
    let device = Device::new(&keys, "reserved");
    let scratch = TempDir::new().unwrap();
    let (method, mut descriptor) = method_file(scratch.path(), &["crate/src"]);
    descriptor.name = "sysroot-pack".into();
    descriptor.step = Some("sysroot-pack:elohim-storage".into());
    let rustc = Command::new("rustc").arg("-vV").output().unwrap();
    assert!(rustc.status.success());
    let rustc = String::from_utf8(rustc.stdout).unwrap();
    let llvm = rustc
        .lines()
        .find_map(|line| line.strip_prefix("LLVM version: "))
        .unwrap()
        .to_owned();
    descriptor.tools.insert("rustc".into(), rustc);
    std::fs::write(&method, serde_json::to_vec(&descriptor).unwrap()).unwrap();
    let key = elohim_epr_cli::artifact::closure_key(device.root(), &descriptor).unwrap();
    let (stub, log) = brit_stub(scratch.path());
    let bytes = scratch.path().join("binary");
    std::fs::write(
        &bytes,
        artifact_elf(format!("Linker: LLD {llvm}\0").as_bytes()),
    )
    .unwrap();
    let out = device
        .command()
        .env("BRIT_BUILD_REF_BIN", &stub)
        .args([
            "artifact",
            "put",
            "--method",
            method.to_str().unwrap(),
            "--input",
            &key.input.to_string(),
            "--bytes",
            bytes.to_str().unwrap(),
            "--project",
            "elohim-storage",
            "--repo",
            device.root().to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3), "{}", text(&out));
    assert!(
        text(&out).contains("method/validator mismatch"),
        "{}",
        text(&out)
    );
    assert!(!log.exists());
    assert!(events(device.root()).is_empty());
}

#[test]
fn artifact_sysroot_put_bounds_initial_archive_read() {
    let scratch = TempDir::new().unwrap();
    let archive = scratch.path().join("oversize.eprsys");
    std::fs::File::create(&archive)
        .unwrap()
        .set_len(elohim_epr_cli::artifact::sysroot::limits::MAX_ARCHIVE_BYTES + 1)
        .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_epr"))
        .args([
            "artifact",
            "sysroot",
            "put",
            "--archive",
            archive.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3), "{}", text(&out));
    assert!(
        text(&out).contains("archive read byte limit"),
        "{}",
        text(&out)
    );
}
