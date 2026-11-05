use super::*;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::PermissionsExt;

async fn verified() -> VerifiedTree {
    let (store, root) = super::tests::fixture().await;
    load_verified_tree(
        &store,
        &root,
        FetchPolicy::LocalOnly,
        &TreeLimits::default(),
    )
    .await
    .unwrap()
}

fn assert_contents(path: &Path) {
    assert_eq!(
        std::fs::metadata(path).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert_eq!(
        std::fs::read(
            path.join("deep")
                .join(std::ffi::OsStr::from_bytes(b"run\xff"))
        )
        .unwrap(),
        b"\0binary\xff\n"
    );
    assert_eq!(
        std::fs::read_link(path.join("deep/link"))
            .unwrap()
            .as_os_str()
            .as_bytes(),
        b"../\xff-target"
    );
}

#[tokio::test]
async fn sync_order_is_file_then_bottom_up_directories_then_promotion_then_parent() {
    let tree = verified().await;
    let parent = tempfile::tempdir().unwrap();
    let destination = parent.path().join("restored");
    let mut seen = Vec::new();
    stage_and_promote_with_hook(
        &tree,
        &destination,
        parent.path(),
        || Ok(()),
        &mut |step, path| {
            if step != RestoreStep::SyncDestinationParent {
                assert!(!destination.exists());
            }
            if step == RestoreStep::SyncDestinationParent {
                assert_contents(&destination);
            }
            seen.push((step, path.to_path_buf()));
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(
        seen.iter().map(|(step, _)| *step).collect::<Vec<_>>(),
        vec![
            RestoreStep::SyncStageParent,
            RestoreStep::WriteFile,
            RestoreStep::SyncFile,
            RestoreStep::SyncDirectory,
            RestoreStep::SyncDirectory,
            RestoreStep::Promote,
            RestoreStep::SyncDestinationParent,
        ]
    );
    assert_eq!(seen[2].1.file_name().unwrap().as_bytes(), b"run\xff");
    assert_eq!(seen[3].1.file_name().unwrap(), "deep");
    assert_eq!(seen[3].1.parent().unwrap(), seen[4].1);
    assert_eq!(seen[6].1, parent.path());
    assert_eq!(std::fs::read_dir(parent.path()).unwrap().count(), 1);
    assert_contents(&destination);
}

#[tokio::test]
async fn every_write_sync_and_promotion_failure_reports_retained_location() {
    let tree = verified().await;
    // Both directory syncs are independent required boundaries.
    for fail_at in 0..7 {
        let parent = tempfile::tempdir().unwrap();
        let source = parent.path().join("source");
        std::fs::write(&source, b"later editable B").unwrap();
        let prior = parent.path().join("prior-candidate");
        std::fs::write(&prior, b"prior candidate").unwrap();
        let destination = parent.path().join("restored");
        let mut step_number = 0;
        let error = stage_and_promote_with_hook(
            &tree,
            &destination,
            parent.path(),
            || Ok(()),
            &mut |_, path| {
                let current = step_number;
                step_number += 1;
                if current == fail_at {
                    return Err(io_error(
                        path,
                        std::io::Error::other("injected sync/write failure"),
                    ));
                }
                Ok(())
            },
        )
        .unwrap_err();
        assert!(error.to_string().contains("injected sync/write failure"));
        let residue = error.residue().expect("missing residue report");
        let retained_path = &residue.retained_path;
        assert_eq!(residue.promoted, fail_at == 6);
        assert_eq!(destination.exists(), residue.promoted);
        assert!(retained_path.is_dir());
        assert_eq!(std::fs::read(&source).unwrap(), b"later editable B");
        assert_eq!(std::fs::read(&prior).unwrap(), b"prior candidate");
        if fail_at >= 3 {
            assert_contents(retained_path);
        }
        // Explicit retries choose another fresh destination and leave residue.
        let retry = parent.path().join("retry");
        stage_and_promote(&tree, &retry, parent.path(), || Ok(())).unwrap();
        assert_contents(&retry);
        assert!(retained_path.is_dir());
        assert_eq!(std::fs::read_dir(parent.path()).unwrap().count(), 4);
    }
}

#[tokio::test]
async fn panic_does_not_clean_up_uncertain_stage() {
    let tree = verified().await;
    let parent = tempfile::tempdir().unwrap();
    let destination = parent.path().join("restored");
    let panic = std::panic::catch_unwind(|| {
        let _ = stage_and_promote(&tree, &destination, parent.path(), || {
            panic!("interruption")
        });
    });
    assert!(panic.is_err());
    assert!(!destination.exists());
    let entries: Vec<_> = std::fs::read_dir(parent.path())
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(entries.len(), 1);
    assert_contents(&entries[0]);
}

#[tokio::test]
async fn oversized_raw_object_refuses_before_staging_even_with_raised_tree_budget() {
    use bytes::Bytes;
    use eprfs_core::{encode_tree, BlobLink, TreeEntry, TreeNode};
    let store = eprfs_storage::MemoryStorage::default();
    let bytes = Bytes::from(vec![0; 64 * 1024 * 1024 + 1]);
    let cid = BlobCid::compute_bytes(&bytes);
    store.insert_blob(cid.clone(), bytes).await;
    let node = TreeNode::new(vec![TreeEntry {
        name: b"large".to_vec(),
        kind: TreeEntryKind::File {
            blob: BlobLink::from(cid),
            executable: false,
        },
    }])
    .unwrap();
    let limits = TreeLimits {
        max_total_bytes: usize::MAX,
        ..TreeLimits::default()
    };
    let (root, encoded) = encode_tree(&node, &limits).unwrap();
    store.insert_blob(root.clone(), Bytes::from(encoded)).await;
    let parent = tempfile::tempdir().unwrap();
    let error = restore_exact_tree(
        &store,
        &root,
        parent.path().join("absent"),
        FetchPolicy::LocalOnly,
        &limits,
        &HostProfile::linux_native(),
    )
    .await
    .unwrap_err();
    assert!(error
        .to_string()
        .contains("restore object byte budget exceeded"));
    assert!(error.residue().is_none());
    assert_eq!(std::fs::read_dir(parent.path()).unwrap().count(), 0);
}

#[tokio::test]
async fn process_interruption_preserves_stage_or_promoted_tree() {
    const CHILD: &str = "EPRFS_RESTORE_DURABILITY_CHILD";
    if let Some(parent) = std::env::var_os(CHILD) {
        let parent = PathBuf::from(parent);
        let tree = verified().await;
        let after = std::env::var_os("EPRFS_RESTORE_AFTER_PROMOTION").is_some();
        let stop = if after {
            RestoreStep::SyncDestinationParent
        } else {
            RestoreStep::Promote
        };
        let _ = stage_and_promote_with_hook(
            &tree,
            &parent.join("restored"),
            &parent,
            || Ok(()),
            &mut |step, _| {
                if step == stop {
                    std::process::exit(73);
                }
                Ok(())
            },
        );
        panic!("child did not interrupt");
    }
    for after in [false, true] {
        let parent = tempfile::tempdir().unwrap();
        let mut child = std::process::Command::new(std::env::current_exe().unwrap());
        child.args(["--exact", "exact_tree::durability_tests::process_interruption_preserves_stage_or_promoted_tree", "--nocapture"])
            .env(CHILD, parent.path()).env_remove("EPRFS_RESTORE_AFTER_PROMOTION");
        if after {
            child.env("EPRFS_RESTORE_AFTER_PROMOTION", "1");
        }
        assert_eq!(child.status().unwrap().code(), Some(73));
        let entries: Vec<_> = std::fs::read_dir(parent.path())
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        assert_eq!(entries.len(), 1);
        assert_eq!(parent.path().join("restored").exists(), after);
        assert_contents(&entries[0]);
    }
}
