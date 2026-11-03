#![cfg(target_os = "linux")]

use eprfs_core::{load_verified_tree, BlobCid, FetchPolicy, TreeEntryKind, TreeLimits};
use eprfs_local::capture_tree;
use eprfs_storage::DirectoryStorage;

#[test]
fn restore_error_enum_remains_exhaustively_matchable() {
    // Existing downstream exhaustive matches must still compile in this patch.
    fn existing_consumer(error: eprfs_local::ExactRestoreError) -> &'static str {
        match error {
            eprfs_local::ExactRestoreError::Tree(_) => "tree",
            eprfs_local::ExactRestoreError::Refused(_) => "refused",
            eprfs_local::ExactRestoreError::Io { .. } => "io",
        }
    }
    let error = eprfs_local::ExactRestoreError::Refused("preflight".into());
    assert!(error.residue().is_none());
    assert_eq!(existing_consumer(error), "refused");
}

#[tokio::test]
async fn captures_without_git_and_keeps_candidate_a_after_source_becomes_b() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    std::fs::create_dir(&source).unwrap();
    std::fs::write(source.join("main.rs"), b"candidate A").unwrap();
    let store = temp.path().join("objects");
    let captured = capture_tree(&source, &store, &["main.rs".into()], &TreeLimits::default())
        .await
        .unwrap();
    std::fs::write(source.join("main.rs"), b"candidate B").unwrap();
    let tree = load_verified_tree(
        &DirectoryStorage::open(&store).unwrap(),
        &captured.root,
        FetchPolicy::LocalOnly,
        &TreeLimits::default(),
    )
    .await
    .unwrap();
    assert!(!captured.published);
    assert_eq!(captured.files, 1);
    match &tree.root_node().entries[0].kind {
        TreeEntryKind::File { blob, executable } => {
            assert_eq!(blob.as_blob_cid(), &BlobCid::compute_raw(b"candidate A"));
            assert_eq!(
                tree.leaf(blob.as_blob_cid()).unwrap().as_ref(),
                b"candidate A"
            );
            assert!(!executable);
        }
        other => panic!("expected file, got {other:?}"),
    }
    assert_eq!(
        std::fs::read(source.join("main.rs")).unwrap(),
        b"candidate B"
    );
}

#[tokio::test]
async fn durable_capture_restore_recapture_keeps_the_tree_v1_golden() {
    use std::os::unix::{ffi::OsStrExt, fs::PermissionsExt};
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    std::fs::create_dir(&source).unwrap();
    let name = std::ffi::OsStr::from_bytes(b"\xff");
    std::fs::write(source.join(name), b"hello world").unwrap();
    std::fs::set_permissions(source.join(name), std::fs::Permissions::from_mode(0o644)).unwrap();
    let store_path = temp.path().join("objects");
    let captured = capture_tree(&source, &store_path, &[name.into()], &TreeLimits::default())
        .await
        .unwrap();
    // Independent Tree v1 vector from core's hand-written DAG-CBOR test; no
    // expected CID computed by the capture/restore path under test.
    assert_eq!(
        captured.root.to_string(),
        "bafyreihedxqhkuhibpz2uhoknm5ov2p4ubsjr5csvg4tdajfdapsyotrou"
    );
    assert!(!captured.published);
    std::fs::write(source.join(name), b"edited B").unwrap();
    let restored = temp.path().join("restored");
    let report = eprfs_local::restore_exact_tree(
        &DirectoryStorage::open(&store_path).unwrap(),
        &captured.root,
        &restored,
        FetchPolicy::LocalOnly,
        &TreeLimits::default(),
        &eprfs_host::HostProfile::linux_native(),
    )
    .await
    .unwrap();
    assert_eq!(report.files, 1);
    assert_eq!(report.bytes_written, 11);
    assert_eq!(std::fs::read(restored.join(name)).unwrap(), b"hello world");
    assert_eq!(std::fs::read(source.join(name)).unwrap(), b"edited B");
    let recaptured = capture_tree(
        &restored,
        &store_path,
        &[name.into()],
        &TreeLimits::default(),
    )
    .await
    .unwrap();
    assert_eq!(recaptured.root, captured.root);
}

#[tokio::test]
async fn restore_preserves_executable_content_without_executing_it() {
    use std::os::unix::fs::PermissionsExt;
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    std::fs::create_dir(&source).unwrap();
    let marker = temp.path().join("executed");
    let script = format!("#!/bin/sh\nprintf executed > '{}'\n", marker.display());
    std::fs::write(source.join("run"), &script).unwrap();
    std::fs::set_permissions(source.join("run"), std::fs::Permissions::from_mode(0o755)).unwrap();
    let store = temp.path().join("objects");
    let captured = capture_tree(&source, &store, &["run".into()], &TreeLimits::default())
        .await
        .unwrap();
    let restored = temp.path().join("restored");
    eprfs_local::restore_exact_tree(
        &DirectoryStorage::open(store).unwrap(),
        &captured.root,
        &restored,
        FetchPolicy::LocalOnly,
        &TreeLimits::default(),
        &eprfs_host::HostProfile::linux_native(),
    )
    .await
    .unwrap();
    assert_eq!(
        std::fs::read(restored.join("run")).unwrap(),
        script.as_bytes()
    );
    assert!(!marker.exists());
    assert_eq!(
        std::fs::metadata(restored.join("run"))
            .unwrap()
            .permissions()
            .mode()
            & 0o111,
        0o111
    );
    assert_eq!(std::fs::read_dir(&restored).unwrap().count(), 1);
    assert_eq!(std::fs::read_dir(&source).unwrap().count(), 1);
}
