#![cfg(target_os = "linux")]

use eprfs_core::{load_verified_tree, BlobCid, FetchPolicy, TreeEntryKind, TreeLimits};
use eprfs_local::capture_tree;
use eprfs_storage::DirectoryStorage;

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
