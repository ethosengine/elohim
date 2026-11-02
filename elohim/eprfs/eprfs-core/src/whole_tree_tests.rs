use crate::{
    test_support::{file, CountingStorage},
    BlobCid, FetchPolicy, KeyInput, ProjectedRoot, TreeEntry, TreeEntryKind, TreeError, TreeLimits,
    TreePath, View,
};

#[tokio::test]
async fn whole_tree_projection_checks_bytes_and_preserves_identity() {
    let mut store = CountingStorage::new();
    store.put_leaf(b"one");
    let root = store.put_tree(vec![file(b"file", b"one")]);
    let projected = ProjectedRoot::verify_whole_tree(
        &store,
        &root,
        &View::whole(),
        FetchPolicy::LocalOnly,
        &TreeLimits::default(),
    )
    .await
    .unwrap();
    assert_eq!(projected.cid(), &root);
    assert_eq!(
        store.fetched(),
        vec![root.clone(), BlobCid::compute_raw(b"one")]
    );
    let key = KeyInput::Projected(projected);
    assert_eq!(key.cid(), &root);
    assert!(matches!(key, KeyInput::Projected(_)));
}

#[tokio::test]
async fn empty_and_narrow_views_refuse_before_fetch() {
    let store = CountingStorage::new();
    for view in [
        View::from_tree_paths([]),
        View::from_tree_paths([TreePath::from_slash_bytes(b"src").unwrap()]),
    ] {
        let error = ProjectedRoot::verify_whole_tree(
            &store,
            &BlobCid::compute(b"missing"),
            &view,
            FetchPolicy::LocalOnly,
            &TreeLimits::default(),
        )
        .await
        .unwrap_err();
        assert!(matches!(error, TreeError::InvalidView(_)));
    }
    assert!(store.fetched().is_empty());
}

#[tokio::test]
async fn whole_tree_projection_rejects_incomplete_corrupt_external_and_over_budget() {
    let mut store = CountingStorage::new();
    let root = store.put_tree(vec![file(b"file", b"missing")]);
    let error = ProjectedRoot::verify_whole_tree(
        &store,
        &root,
        &View::whole(),
        FetchPolicy::LocalOnly,
        &TreeLimits::default(),
    )
    .await
    .unwrap_err();
    assert!(matches!(error, TreeError::Incomplete { .. }));
    store.blobs.insert(
        BlobCid::compute_raw(b"missing"),
        bytes::Bytes::from_static(b"corrupt"),
    );
    let error = ProjectedRoot::verify_whole_tree(
        &store,
        &root,
        &View::whole(),
        FetchPolicy::LocalOnly,
        &TreeLimits::default(),
    )
    .await
    .unwrap_err();
    assert!(matches!(error, TreeError::Integrity(_)));
    let external = store.put_tree(vec![TreeEntry {
        name: b"external".to_vec(),
        kind: TreeEntryKind::External {
            kind: "opaque".into(),
            reference: vec![42],
        },
    }]);
    let error = ProjectedRoot::verify_whole_tree(
        &store,
        &external,
        &View::whole(),
        FetchPolicy::LocalOnly,
        &TreeLimits::default(),
    )
    .await
    .unwrap_err();
    assert!(matches!(error, TreeError::InvalidView(_)));
    let empty = store.put_tree(vec![]);
    let error = ProjectedRoot::verify_whole_tree(
        &store,
        &empty,
        &View::whole(),
        FetchPolicy::LocalOnly,
        &TreeLimits {
            max_total_bytes: 1,
            ..TreeLimits::default()
        },
    )
    .await
    .unwrap_err();
    assert!(matches!(error, TreeError::Budget(_)));
    assert!(ProjectedRoot::verify_whole_tree(
        &store,
        &BlobCid::compute_raw(b"raw is not a tree"),
        &View::whole(),
        FetchPolicy::LocalOnly,
        &TreeLimits::default(),
    )
    .await
    .is_err());
}
