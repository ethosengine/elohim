//! The device payload store's format marker and the one-store rule (artifact-store E4).
#![cfg(feature = "peer")]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use bytes::Bytes;
use elohim_epr_cli::payload::{
    detect, open_payload_store, select_format, Access, Detected, PayloadFormat, PayloadStoreError,
    MARKER_FILE,
};
use eprfs_core::FetchPolicy;

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap()
}

/// A private parent directory and the payload root inside it (not yet created).
fn layout() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    let root = dir.path().join("payload");
    (dir, root)
}

fn private_mkdir(path: &Path) {
    std::fs::create_dir(path).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
}

/// Every file under `dir` (relative path → bytes) and every directory (→ empty).
fn snapshot(dir: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    fn walk(base: &Path, dir: &Path, out: &mut BTreeMap<PathBuf, Option<Vec<u8>>>) {
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            let relative = path.strip_prefix(base).unwrap().to_path_buf();
            if path.is_dir() {
                out.insert(relative, None);
                walk(base, &path, out);
            } else {
                out.insert(relative, Some(std::fs::read(&path).unwrap()));
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(dir, dir, &mut out);
    out
}

fn deterministic(seed: u64, len: usize) -> Vec<u8> {
    let mut state = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
    let mut out = Vec::with_capacity(len + 8);
    while out.len() < len {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        out.extend_from_slice(&state.wrapping_mul(0x2545_F491_4F6C_DD1D).to_le_bytes());
    }
    out.truncate(len);
    out
}

#[test]
fn an_unmarked_directory_opens_as_directory_v1_unchanged() {
    let rt = runtime();
    let (_dir, root) = layout();
    // Every device's payload directory before the marker: eprfs-peer's directory store.
    let small = Bytes::from_static(b"an object from before the marker");
    let big = Bytes::from(deterministic(3, 300_000));
    let (small_cid, big_cid) = rt.block_on(async {
        let legacy = eprfs_peer::open_payload_store(&root).unwrap();
        (
            legacy.put_blob(small.clone()).await.unwrap(),
            legacy.put_blob(big.clone()).await.unwrap(),
        )
    });
    let before = snapshot(&root);
    assert_eq!(detect(&root).unwrap(), Detected::UnmarkedDirectory);

    for access in [Access::Owner, Access::Shared] {
        let opened = rt.block_on(open_payload_store(&root, access)).unwrap();
        assert_eq!(opened.format, PayloadFormat::DirectoryV1, "{access:?}");
        for (cid, bytes) in [(&small_cid, &small), (&big_cid, &big)] {
            let got = rt
                .block_on(opened.store.fetch_blob(cid, FetchPolicy::LocalOnly))
                .unwrap();
            assert_eq!(&got.bytes, bytes);
        }
    }
    assert_eq!(snapshot(&root), before, "opening wrote to the directory");
    assert!(
        !root.join(MARKER_FILE).exists(),
        "an existing directory is never stamped"
    );

    // A directory holding only the directory store's own staging residue is legacy too.
    let (_dir, residue) = layout();
    private_mkdir(&residue);
    std::fs::write(residue.join(".tmpA1b2C3"), b"half a write").unwrap();
    let before = snapshot(&residue);
    let opened = rt
        .block_on(open_payload_store(&residue, Access::Owner))
        .unwrap();
    assert_eq!(opened.format, PayloadFormat::DirectoryV1);
    assert_eq!(snapshot(&residue), before);
}

#[test]
fn a_payload_store_of_another_format_is_refused_not_duplicated() {
    let rt = runtime();
    let mut foreign = vec!["directory-v9", "a-store-from-the-future"];
    if !cfg!(feature = "blob-store") {
        // Built without the blob store, a blob-store directory is another format.
        foreign.push(PayloadFormat::BlobStoreV1.marker());
    }
    for marker in foreign {
        let (dir, root) = layout();
        private_mkdir(&root);
        std::fs::write(root.join(MARKER_FILE), format!("{marker}\n")).unwrap();
        std::fs::write(root.join("held"), b"something the other format keeps").unwrap();
        let (before, parent_before) = (snapshot(&root), snapshot(dir.path()));
        for access in [Access::Owner, Access::Shared] {
            match rt.block_on(open_payload_store(&root, access)) {
                Err(PayloadStoreError::Format {
                    found, supported, ..
                }) => {
                    assert_eq!(found, marker);
                    assert_eq!(supported, PayloadFormat::supported());
                }
                Err(other) => panic!("{marker}: refused for the wrong reason: {other}"),
                Ok(_) => panic!("{marker}: a foreign payload store was opened"),
            }
        }
        assert_eq!(snapshot(&root), before, "{marker}: the store was touched");
        assert_eq!(
            snapshot(dir.path()),
            parent_before,
            "{marker}: a second store was created beside it"
        );
    }

    // Unmarked layouts that are not a legacy directory store are refused the same way.
    for (case, dirs, files) in [
        (
            "blob-store layout",
            &["blobs", "chunks", "index"][..],
            &[][..],
        ),
        ("mixed", &["blobs"][..], &["<a flat object>"][..]),
        ("unrecognisable", &[][..], &["notes.txt"][..]),
    ] {
        let (dir, root) = layout();
        private_mkdir(&root);
        for name in dirs {
            std::fs::create_dir(root.join(name)).unwrap();
        }
        for name in files {
            let name = if *name == "<a flat object>" {
                elohim_epr::address::compute_bytes_cid(b"x").to_string()
            } else {
                (*name).to_string()
            };
            std::fs::write(root.join(name), b"x").unwrap();
        }
        let (before, parent_before) = (snapshot(&root), snapshot(dir.path()));
        assert!(
            matches!(
                rt.block_on(open_payload_store(&root, Access::Owner)),
                Err(PayloadStoreError::Layout { .. })
            ),
            "{case}"
        );
        assert_eq!(snapshot(&root), before, "{case}");
        assert_eq!(snapshot(dir.path()), parent_before, "{case}");
    }
}

#[test]
fn concurrent_first_opens_agree_on_one_format() {
    for _ in 0..20 {
        let (_dir, root) = layout();
        let winners: Vec<String> = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..8)
                .map(|i| {
                    let root = root.clone();
                    scope.spawn(move || {
                        let wanted = if i % 2 == 0 {
                            PayloadFormat::BlobStoreV1
                        } else {
                            PayloadFormat::DirectoryV1
                        };
                        select_format(&root, wanted).unwrap()
                    })
                })
                .collect();
            handles.into_iter().map(|h| h.join().unwrap()).collect()
        });
        assert!(
            winners.windows(2).all(|pair| pair[0] == pair[1]),
            "{winners:?}"
        );
        let names: Vec<String> = std::fs::read_dir(&root)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, vec![MARKER_FILE.to_string()], "no staging residue");
        assert_eq!(detect(&root).unwrap(), Detected::Marked(winners[0].clone()));
    }
}

#[test]
fn a_marker_without_its_layout_opens_and_builds_the_layout() {
    // A first open that died between selecting the format and building the store.
    let rt = runtime();
    let (_dir, root) = layout();
    let format = PayloadFormat::for_new_store();
    assert_eq!(select_format(&root, format).unwrap(), format.marker());
    let opened = rt
        .block_on(open_payload_store(&root, Access::Owner))
        .unwrap();
    assert_eq!(opened.format, format);
    let bytes = Bytes::from_static(b"stored after the interrupted first open");
    let cid = rt.block_on(opened.store.put_blob(bytes.clone())).unwrap();
    let got = rt
        .block_on(opened.store.fetch_blob(&cid, FetchPolicy::LocalOnly))
        .unwrap();
    assert_eq!(got.bytes, bytes);
    assert_eq!(
        detect(&root).unwrap(),
        Detected::Marked(format.marker().into())
    );
}

#[cfg(feature = "blob-store")]
#[test]
fn the_same_cid_is_never_held_twice_on_one_device_by_the_two_paths() {
    // The two real paths onto one device root: `epr artifact put`'s custody path (the payload
    // opener as artifact.rs opens it, then `put_local_blob_with`, which stores each chunk as its
    // own object plus a manifest) and storage's marker-gated tier (a whole-payload put, which
    // the store chunks itself). Both orders, a multi-chunk payload.
    for cli_first in [true, false] {
        the_two_paths_hold_one_copy(cli_first);
    }
}

#[cfg(feature = "blob-store")]
fn the_two_paths_hold_one_copy(cli_first: bool) {
    use elohim_blob_store::tier::{compose_shared_payload_tier, PayloadTier};
    use elohim_epr::address::compute_bytes_cid;
    use elohim_epr::profile::{chunk_bytes, ValidatedProfile};
    use elohim_epr_rea::{atom_cid, AgentRef, MemoryFlowStore};
    use eprfs_core::{
        BlobCid, DerivationBasis, DerivationKey, DerivationRecord, EprfsStorage, KeyInput, Witness,
    };
    use eprfs_peer::{put_local_blob_with, LocalBlobContext};

    let rt = runtime();
    let (dir, root) = layout();
    let cli = rt
        .block_on(open_payload_store(&root, Access::Shared))
        .unwrap();
    assert_eq!(cli.format, PayloadFormat::BlobStoreV1);
    let storage = match rt.block_on(compose_shared_payload_tier(&root)).unwrap() {
        PayloadTier::Composed(tier) => tier,
        PayloadTier::NotBlobStore { marker } => panic!("not composed: {marker:?}"),
    };

    let data = deterministic(if cli_first { 41 } else { 42 }, 9 * 1024 * 1024);
    let output = BlobCid::compute_bytes(&data);
    let record = DerivationRecord {
        key: DerivationKey {
            method: BlobCid::compute_raw(b"mesh-build"),
            input: KeyInput::Raw(BlobCid::compute_raw(b"a source closure")),
        },
        output: output.clone(),
        computed_by: "did:key:zDeviceA".into(),
        basis: DerivationBasis::Computed,
        deterministic: false,
        witness: Witness::Signature {
            sig: "fixture-signature".into(),
        },
    };
    let context = LocalBlobContext {
        custodian: AgentRef("did:key:zDeviceA".into()),
        in_scope_of: atom_cid(&"artifact-scope").unwrap(),
        occurred_at: "2026-11-09T12:00:00Z".into(),
        crossing: None,
    };
    rt.block_on(async {
        let cli_put = || async {
            let mut flow = MemoryFlowStore::new();
            put_local_blob_with(
                cli.store.as_ref(),
                &mut flow,
                record.clone(),
                Bytes::from(data.clone()),
                &context,
                Vec::new(),
            )
            .await
            .expect("the custody path puts")
        };
        let whole = || async {
            let cid = storage
                .put_blob(Bytes::from(data.clone()))
                .await
                .expect("the tier puts");
            assert_eq!(cid, output);
        };
        let put = if cli_first {
            let put = cli_put().await;
            whole().await;
            put
        } else {
            whole().await;
            cli_put().await
        };
        // Every chunk the manifest names, and the whole artifact, read verified through both.
        assert!(put.manifest.chunks.len() > 1, "the fixture is multi-chunk");
        for chunk in &put.manifest.chunks {
            for reader in [cli.store.as_ref(), &storage as &dyn EprfsStorage] {
                let got = reader
                    .fetch_blob(&chunk.cid, FetchPolicy::LocalOnly)
                    .await
                    .unwrap();
                assert!(chunk.cid.verifies(&got.bytes));
            }
        }
        for reader in [cli.store.as_ref(), &storage as &dyn EprfsStorage] {
            let got = reader
                .fetch_blob(&output, FetchPolicy::LocalOnly)
                .await
                .unwrap();
            assert_eq!(&got.bytes[..], &data[..]);
        }
    });

    // Physical files carrying each byte CID's bytes, across blobs/ and chunks/ (and every other
    // place under the device directory).
    let files = snapshot(dir.path());
    let carriers = |cid: &str| -> Vec<&PathBuf> {
        files
            .iter()
            .filter(|(path, bytes)| {
                path.file_name().and_then(|n| n.to_str()) == Some(cid)
                    && bytes
                        .as_ref()
                        .is_some_and(|b| compute_bytes_cid(b).to_string() == cid)
            })
            .map(|(path, _)| path)
            .collect()
    };
    assert!(
        !files
            .keys()
            .any(|path| path.iter().any(|part| part == "eprfs-tier")),
        "no second (dedicated) store was opened"
    );
    let mut carried = 0usize;
    for (offset, len) in chunk_bytes(&ValidatedProfile::EPOCH_0, &data) {
        let chunk = compute_bytes_cid(&data[offset..offset + len]).to_string();
        let held = carriers(&chunk);
        assert_eq!(
            held.len(),
            1,
            "cli_first={cli_first}: chunk {chunk} held {held:?}"
        );
        assert!(
            held[0].starts_with("payload/blobs") || held[0].starts_with("payload/chunks"),
            "{held:?}"
        );
        carried += len;
    }
    assert!(
        carriers(&output.to_string()).is_empty(),
        "the whole artifact is held as its chunks, not as a second whole copy"
    );
    let byte_files: usize = files
        .iter()
        .filter(|(path, _)| path.starts_with("payload/blobs") || path.starts_with("payload/chunks"))
        .filter_map(|(path, bytes)| {
            let name = path.file_name()?.to_str()?;
            let bytes = bytes.as_ref()?;
            (compute_bytes_cid(bytes).to_string() == name).then_some(bytes.len())
        })
        .sum();
    assert_eq!(
        byte_files, carried,
        "cli_first={cli_first}: every payload byte is on disk once"
    );
}

#[test]
fn racing_first_opens_through_the_opener_all_open_one_format() {
    let rt = runtime();
    for round in 0..50u64 {
        let (_dir, root) = layout();
        private_mkdir(&root);
        let barrier = std::sync::Barrier::new(6);
        let formats: Vec<Result<PayloadFormat, String>> = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..6u64)
                .map(|i| {
                    let (root, barrier, rt) = (&root, &barrier, &rt);
                    scope.spawn(move || {
                        barrier.wait();
                        // Stagger the arrivals across the winner's marker publication.
                        std::thread::sleep(std::time::Duration::from_micros((round + i) % 7 * 150));
                        rt.block_on(open_payload_store(root, Access::Shared))
                            .map(|opened| opened.format)
                            .map_err(|error| error.to_string())
                    })
                })
                .collect();
            handles.into_iter().map(|h| h.join().unwrap()).collect()
        });
        for format in &formats {
            assert_eq!(
                format.as_ref().ok(),
                Some(&PayloadFormat::for_new_store()),
                "round {round}: {formats:?}"
            );
        }
    }
}
