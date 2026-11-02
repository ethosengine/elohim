use super::*;

fn failure() -> std::io::Error {
    std::io::Error::other("injected durability failure")
}

#[test]
fn new_and_existing_root_sync_failures_refuse_and_preserve_root() {
    for existing in [false, true] {
        for failed in [StoreStep::SyncRoot, StoreStep::SyncParent] {
            let temp = tempfile::tempdir().unwrap();
            let root = temp.path().join("store");
            if existing {
                DirectoryStorage::create(&root).unwrap();
            }
            let error = DirectoryStorage::create_with_hook(&root, &mut |step, _| {
                if step == failed {
                    Err(failure())
                } else {
                    Ok(())
                }
            })
            .unwrap_err();
            assert!(error.to_string().contains("injected durability failure"));
            assert!(root.is_dir());
            DirectoryStorage::create(&root).unwrap();
        }
    }
}

#[test]
fn concurrent_root_creator_still_synchronizes_root_and_parent() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("store");
    let mut seen = Vec::new();
    DirectoryStorage::create_with_hook(&root, &mut |step, path| {
        seen.push((step, path.to_path_buf()));
        if step == StoreStep::CreateRoot {
            DirectoryStorage::create(&root).unwrap();
        }
        Ok(())
    })
    .unwrap();
    assert_eq!(
        seen,
        vec![
            (StoreStep::CreateRoot, root.clone()),
            (StoreStep::SyncRoot, root),
            (StoreStep::SyncParent, temp.path().to_path_buf()),
        ]
    );
}

#[test]
fn new_existing_and_concurrent_objects_sync_the_actual_inode_before_directories() {
    for state in ["new", "existing", "concurrent"] {
        let temp = tempfile::tempdir().unwrap();
        let store = DirectoryStorage::create(temp.path().join("store")).unwrap();
        let bytes = Bytes::from_static(b"durable candidate");
        let cid = BlobCid::compute_raw(&bytes);
        let object = store.object_path(&cid);
        if state == "existing" {
            store.put_object(&cid, bytes.clone()).unwrap();
        }
        let mut seen = Vec::new();
        store
            .put_object_with_hook(&cid, bytes.clone(), &mut |step, path| {
                seen.push((step, path.to_path_buf()));
                if step == StoreStep::PublishObject && state == "concurrent" {
                    // This competing object intentionally has not been synchronized.
                    let mut winner = tempfile::NamedTempFile::new_in(&store.root).unwrap();
                    winner.write_all(&bytes).unwrap();
                    winner.persist_noclobber(&object).unwrap();
                }
                Ok(())
            })
            .unwrap();
        let steps: Vec<_> = seen.iter().map(|(step, _)| *step).collect();
        let mut expected = if state == "existing" {
            vec![]
        } else {
            vec![
                StoreStep::WriteObject,
                StoreStep::SyncObject,
                StoreStep::PublishObject,
            ]
        };
        if state != "new" {
            expected.push(StoreStep::SyncObject);
        }
        expected.extend([StoreStep::SyncRoot, StoreStep::SyncParent]);
        assert_eq!(steps, expected, "{state}");
        if state != "new" {
            let last_sync = seen
                .iter()
                .rev()
                .find(|(s, _)| *s == StoreStep::SyncObject)
                .unwrap();
            assert_eq!(last_sync.1, object, "must sync winner inode");
        }
        assert_eq!(store.read_object(&cid).unwrap(), bytes);
        assert_eq!(std::fs::read_dir(&store.root).unwrap().count(), 1);
    }
}

#[test]
fn every_object_write_sync_and_promotion_failure_refuses_without_losing_candidates() {
    for state in ["new", "existing", "concurrent"] {
        let steps = if state == "existing" {
            vec![
                StoreStep::SyncObject,
                StoreStep::SyncRoot,
                StoreStep::SyncParent,
            ]
        } else {
            vec![
                StoreStep::WriteObject,
                StoreStep::SyncObject,
                StoreStep::PublishObject,
                StoreStep::SyncRoot,
                StoreStep::SyncParent,
            ]
        };
        for failed in steps {
            let temp = tempfile::tempdir().unwrap();
            let store = DirectoryStorage::create(temp.path().join("store")).unwrap();
            let prior = Bytes::from_static(b"prior candidate");
            let prior_cid = BlobCid::compute_raw(&prior);
            store.put_object(&prior_cid, prior.clone()).unwrap();
            let bytes = Bytes::from_static(b"new candidate");
            let cid = BlobCid::compute_raw(&bytes);
            if state == "existing" {
                store.put_object(&cid, bytes.clone()).unwrap();
            }
            let error = store
                .put_object_with_hook(&cid, bytes.clone(), &mut |step, _| {
                    if step == failed {
                        return Err(failure());
                    }
                    if step == StoreStep::PublishObject && state == "concurrent" {
                        store.put_object(&cid, bytes.clone()).unwrap();
                    }
                    Ok(())
                })
                .unwrap_err();
            assert!(error.to_string().contains("injected durability failure"));
            assert_eq!(store.read_object(&prior_cid).unwrap(), prior);
            if state != "existing"
                && matches!(
                    failed,
                    StoreStep::WriteObject | StoreStep::SyncObject | StoreStep::PublishObject
                )
            {
                assert!(error.to_string().contains("incomplete object retained at"));
                assert_eq!(std::fs::read_dir(&store.root).unwrap().count(), 2);
                assert!(!store.object_path(&cid).exists());
            } else {
                assert_eq!(store.read_object(&cid).unwrap(), bytes);
            }
            // An interrupted write did not establish success. An explicit retry
            // must do its own verification and synchronization.
            store.put_object(&cid, bytes.clone()).unwrap();
            assert_eq!(store.read_object(&cid).unwrap(), bytes);
        }
    }
}

#[test]
fn losing_writer_cannot_succeed_when_winner_sync_fails() {
    let temp = tempfile::tempdir().unwrap();
    let store = DirectoryStorage::create(temp.path().join("store")).unwrap();
    let bytes = Bytes::from_static(b"same candidate");
    let cid = BlobCid::compute_raw(&bytes);
    let object = store.object_path(&cid);
    let error = store
        .put_object_with_hook(&cid, bytes.clone(), &mut |step, path| {
            if step == StoreStep::PublishObject {
                store.put_object(&cid, bytes.clone()).unwrap();
            }
            if step == StoreStep::SyncObject && path == object {
                return Err(failure());
            }
            Ok(())
        })
        .unwrap_err();
    assert!(error.to_string().contains("incomplete object retained at"));
    assert_eq!(store.read_object(&cid).unwrap(), bytes);
    assert_eq!(std::fs::read_dir(&store.root).unwrap().count(), 2);
}

#[test]
fn actual_parallel_puts_converge_without_temporary_residue() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("store");
    let barrier = std::sync::Barrier::new(8);
    std::thread::scope(|scope| {
        for _ in 0..8 {
            scope.spawn(|| {
                barrier.wait();
                let store = DirectoryStorage::create(&root).unwrap();
                let bytes = Bytes::from_static(b"concurrent");
                store
                    .put_object(&BlobCid::compute_raw(&bytes), bytes)
                    .unwrap();
            });
        }
    });
    assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
    let store = DirectoryStorage::open(&root).unwrap();
    assert_eq!(
        store
            .read_object(&BlobCid::compute_raw(b"concurrent"))
            .unwrap()
            .as_ref(),
        b"concurrent"
    );
}

#[test]
fn object_cap_cannot_be_raised() {
    let temp = tempfile::tempdir().unwrap();
    let store = DirectoryStorage::create(temp.path().join("store"))
        .unwrap()
        .with_max_blob_bytes(usize::MAX);
    let oversized = Bytes::from(vec![0; 64 * 1024 * 1024 + 1]);
    let cid = BlobCid::compute_raw(&oversized);
    let error = store.put_object(&cid, oversized).unwrap_err();
    assert!(error.to_string().contains("exceeds byte limit"));
    assert_eq!(std::fs::read_dir(&store.root).unwrap().count(), 0);
}

#[test]
fn process_interruption_retains_objects_and_allows_verified_retry() {
    const CHILD: &str = "EPRFS_STORE_DURABILITY_CHILD";
    let bytes = Bytes::from_static(b"interrupted candidate");
    let cid = BlobCid::compute_raw(&bytes);
    let prior = Bytes::from_static(b"prior");
    let prior_cid = BlobCid::compute_raw(&prior);
    if let Some(root) = std::env::var_os(CHILD) {
        let store = DirectoryStorage::open(root).unwrap();
        let after = std::env::var_os("EPRFS_STORE_AFTER_PUBLISH").is_some();
        let stop = if after {
            StoreStep::SyncRoot
        } else {
            StoreStep::PublishObject
        };
        let _ = store.put_object_with_hook(&cid, bytes, &mut |step, _| {
            if step == stop {
                std::process::exit(74);
            }
            Ok(())
        });
        panic!("child did not interrupt");
    }
    for after in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("store");
        let store = DirectoryStorage::create(&root).unwrap();
        store.put_object(&prior_cid, prior.clone()).unwrap();
        let mut child = std::process::Command::new(std::env::current_exe().unwrap());
        child
            .args([
                "--exact",
                "durability_tests::process_interruption_retains_objects_and_allows_verified_retry",
                "--nocapture",
            ])
            .env(CHILD, &root)
            .env_remove("EPRFS_STORE_AFTER_PUBLISH");
        if after {
            child.env("EPRFS_STORE_AFTER_PUBLISH", "1");
        }
        assert_eq!(child.status().unwrap().code(), Some(74));
        assert_eq!(store.object_path(&cid).exists(), after);
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 2);
        assert_eq!(store.read_object(&prior_cid).unwrap(), prior);
        store.put_object(&cid, bytes.clone()).unwrap();
        assert_eq!(store.read_object(&cid).unwrap(), bytes);
    }
}
