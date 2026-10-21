//! The view-scoped walk and its queries over the real `DirectoryStorage`,
//! `FetchPolicy::LocalOnly`, on a store that holds only what a view needs.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use bytes::Bytes;
use eprfs_core::{
    encode_tree, list_dir, read_path, walk_batched, walk_collect, AbsentReason, BlobCid,
    EprfsStorage, FetchPolicy, ItemAbsence, TreeEntry, TreeEntryKind, TreeError, TreeLimits,
    TreeNode, TreePath, View, WalkItem,
};
use eprfs_storage::DirectoryStorage;

const LOCAL: FetchPolicy = FetchPolicy::LocalOnly;

fn tp(path: &str) -> TreePath {
    TreePath::from_slash_bytes(path.as_bytes()).unwrap()
}

fn content(i: usize, j: usize, k: usize) -> Vec<u8> {
    format!("a{i}/b{j}/f{k:03}").into_bytes()
}

struct Built {
    root: BlobCid,
    nodes: HashMap<TreePath, BlobCid>,
    leaves: HashMap<TreePath, BlobCid>,
}

/// root -> a0..a9 -> b0..b9 -> f000..f099. Every tree node is written; a leaf
/// is written only when `keep_leaf` admits its path, which leaves the store in
/// the same state as writing every leaf and removing the rest.
async fn build(store: &DirectoryStorage, keep_leaf: impl Fn(&TreePath) -> bool) -> Built {
    let limits = TreeLimits::default();
    let mut nodes = HashMap::new();
    let mut leaves = HashMap::new();
    let mut a_entries = Vec::new();
    for i in 0..10 {
        let mut b_entries = Vec::new();
        for j in 0..10 {
            let mut files = Vec::new();
            for k in 0..100 {
                let bytes = content(i, j, k);
                let cid = BlobCid::compute_raw(&bytes);
                let path = tp(&format!("a{i}/b{j}/f{k:03}"));
                if keep_leaf(&path) {
                    store
                        .put_blob_verified(&cid, Bytes::from(bytes))
                        .await
                        .unwrap();
                }
                leaves.insert(path, cid.clone());
                files.push(TreeEntry {
                    name: format!("f{k:03}").into_bytes(),
                    kind: TreeEntryKind::File {
                        blob: cid.into(),
                        executable: false,
                    },
                });
            }
            let b = put_tree(store, files, &limits).await;
            nodes.insert(tp(&format!("a{i}/b{j}")), b.clone());
            b_entries.push(dir(&format!("b{j}"), b));
        }
        let a = put_tree(store, b_entries, &limits).await;
        nodes.insert(tp(&format!("a{i}")), a.clone());
        a_entries.push(dir(&format!("a{i}"), a));
    }
    let root = put_tree(store, a_entries, &limits).await;
    nodes.insert(TreePath::root(), root.clone());
    Built {
        root,
        nodes,
        leaves,
    }
}

fn dir(name: &str, tree: BlobCid) -> TreeEntry {
    TreeEntry {
        name: name.as_bytes().to_vec(),
        kind: TreeEntryKind::Directory { tree: tree.into() },
    }
}

async fn put_tree(
    store: &DirectoryStorage,
    entries: Vec<TreeEntry>,
    limits: &TreeLimits,
) -> BlobCid {
    let (cid, bytes) = encode_tree(&TreeNode::new(entries).unwrap(), limits).unwrap();
    store
        .put_blob_verified(&cid, Bytes::from(bytes))
        .await
        .unwrap();
    cid
}

fn object(root: &Path, cid: &BlobCid) -> std::path::PathBuf {
    root.join(cid.to_string())
}

fn three_path_view() -> View {
    View::from_tree_paths([tp("a1/b2/f003"), tp("a4/b4/f099"), tp("a9/b0")])
}

const SPINE: [&str; 7] = ["", "a1", "a1/b2", "a4", "a4/b4", "a9", "a9/b0"];

#[tokio::test]
async fn directory_storage_walk_three_path_view_over_sparse_store() {
    let dir = tempfile::tempdir().unwrap();
    let objects = dir.path().join("objects");
    let store = DirectoryStorage::create(&objects).unwrap();
    let view = three_path_view();
    let built = build(&store, |path| {
        matches!(view.coverage(path), eprfs_core::Coverage::InView)
    })
    .await;

    // Remove every tree node outside the view's spine and subtrees.
    let keep: HashSet<&BlobCid> = SPINE.iter().map(|p| &built.nodes[&tp(p)]).collect();
    for cid in built.nodes.values().filter(|cid| !keep.contains(cid)) {
        std::fs::remove_file(object(&objects, cid)).unwrap();
    }
    let held = std::fs::read_dir(&objects).unwrap().count();
    assert_eq!(held, 7 + 102, "7 tree nodes and the 102 leaves in view");

    let walked = walk_collect(
        &store,
        &built.root,
        &view,
        &LOCAL.into(),
        &TreeLimits::default(),
    )
    .await
    .unwrap();
    assert_eq!(
        walked
            .nodes
            .iter()
            .map(|n| n.path.clone())
            .collect::<Vec<_>>(),
        SPINE.map(tp).to_vec()
    );
    assert!(walked.absent.is_empty());
    assert_eq!(walked.stats.nodes_fetched, 7);
    assert_eq!(walked.stats.entries_visited, 340);
    let in_view: HashSet<TreePath> = walked.leaves_in_view().map(|(path, _)| path).collect();
    assert_eq!(in_view.len(), 102);

    let leaf = read_path(
        &store,
        &built.root,
        &tp("a4/b4/f099"),
        &LOCAL.into(),
        &TreeLimits::default(),
    )
    .await
    .unwrap();
    assert_eq!(leaf.bytes.as_ref(), content(4, 4, 99).as_slice());
    assert_eq!(leaf.cid, built.leaves[&tp("a4/b4/f099")]);
    let listing = list_dir(
        &store,
        &built.root,
        &tp("a9/b0"),
        &LOCAL.into(),
        &TreeLimits::default(),
    )
    .await
    .unwrap();
    assert_eq!(listing.entries.len(), 100);

    // A path whose spine is not held: a single answer cannot be partial.
    match read_path(
        &store,
        &built.root,
        &tp("a2/b0/f000"),
        &LOCAL.into(),
        &TreeLimits::default(),
    )
    .await
    {
        // B1: the missing node's path, with ruling 7's three parts and no tier asked.
        Err(TreeError::Unanswered { cid, path, why }) => {
            assert_eq!(cid, built.nodes[&tp("a2")]);
            assert_eq!(path, tp("a2"));
            assert!(matches!(&why.why, ItemAbsence::NotHeld { tried } if tried.is_empty()));
        }
        other => panic!("expected Unanswered, got {other:?}"),
    }

    // The walk names the same absence and continues with the rest of the view.
    let wider = View::from_tree_paths([tp("a1/b2/f003"), tp("a2/b0")]);
    let walked = walk_collect(
        &store,
        &built.root,
        &wider,
        &LOCAL.into(),
        &TreeLimits::default(),
    )
    .await
    .unwrap();
    assert_eq!(walked.nodes.len(), 3);
    // The walk types are non-exhaustive, so a downstream caller matches them
    // with `..` rather than building them.
    match walked.absent.as_slice() {
        [WalkItem::Absent {
            view_path,
            reason: AbsentReason::NotHeld {
                missing_at, cid, ..
            },
            ..
        }] => {
            assert_eq!(view_path, &tp("a2/b0"));
            assert_eq!(missing_at, &tp("a2"));
            assert_eq!(cid, &built.nodes[&tp("a2")]);
        }
        other => panic!("expected one NotHeld absence, got {other:?}"),
    }

    // The level-batched walk over the same store gives the same answer.
    let batched = walk_batched(
        &store,
        &built.root,
        &wider,
        &LOCAL.into(),
        &TreeLimits::default(),
    )
    .await
    .unwrap();
    assert_eq!(
        batched
            .nodes
            .iter()
            .map(|n| n.path.clone())
            .collect::<Vec<_>>(),
        walked
            .nodes
            .iter()
            .map(|n| n.path.clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(batched.absent, walked.absent);
    assert_eq!(batched.stats, walked.stats);
}

#[tokio::test]
async fn directory_storage_corrupt_spine_object_refuses() {
    let dir = tempfile::tempdir().unwrap();
    let objects = dir.path().join("objects");
    let store = DirectoryStorage::create(&objects).unwrap();
    let built = build(&store, |_| false).await;
    // Overwriting keeps the object's owner-only mode, so only the bytes are wrong.
    std::fs::write(object(&objects, &built.nodes[&tp("a1")]), b"corrupt").unwrap();
    match walk_collect(
        &store,
        &built.root,
        &three_path_view(),
        &LOCAL.into(),
        &TreeLimits::default(),
    )
    .await
    {
        Err(TreeError::Integrity(cid)) => assert_eq!(cid, built.nodes[&tp("a1")]),
        other => panic!("expected an integrity refusal, got {other:?}"),
    }
}

/// The level-batched walk refuses a corrupt spine object with the same error as the depth-first
/// walk. DirectoryStorage reports it as the typed `EprfsError::Corrupt`, which `fetch_checked`
/// reads as `TreeError::Integrity` and `ItemAbsence::from_error` as `Integrity` (A17): neither
/// walk degrades the corrupt object to an absence, and they agree (B.1 level equivalence).
#[tokio::test]
async fn directory_storage_corrupt_spine_object_refuses_the_batched_walk() {
    let dir = tempfile::tempdir().unwrap();
    let objects = dir.path().join("objects");
    let store = DirectoryStorage::create(&objects).unwrap();
    let built = build(&store, |_| false).await;
    std::fs::write(object(&objects, &built.nodes[&tp("a1")]), b"corrupt").unwrap();
    match walk_batched(
        &store,
        &built.root,
        &three_path_view(),
        &LOCAL.into(),
        &TreeLimits::default(),
    )
    .await
    {
        Err(TreeError::Integrity(cid)) => assert_eq!(cid, built.nodes[&tp("a1")]),
        other => panic!("expected an integrity refusal, got {other:?}"),
    }
}
