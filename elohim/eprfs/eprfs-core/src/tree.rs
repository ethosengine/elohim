//! Immutable, artifact-neutral, content-addressed filesystem trees.
//!
//! A tree is a private byte value. Its CID proves bytes, not publication,
//! standing, or the election of any release head.

use std::collections::HashMap;

use bytes::Bytes;
use serde::{Deserialize, Serialize};

use crate::{BlobCid, BlobLink, EprfsStorage, FetchPolicy};

pub const TREE_VERSION: u8 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreeNode {
    pub version: u8,
    pub entries: Vec<TreeEntry>,
}

impl TreeNode {
    /// Sort by raw name bytes and reject malformed or duplicate names.
    pub fn new(mut entries: Vec<TreeEntry>) -> TreeResult<Self> {
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        let node = Self {
            version: TREE_VERSION,
            entries,
        };
        validate_node(&node, &TreeLimits::default())?;
        Ok(node)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreeEntry {
    #[serde(with = "serde_bytes")]
    pub name: Vec<u8>,
    pub kind: TreeEntryKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub enum TreeEntryKind {
    File {
        blob: BlobLink,
        executable: bool,
    },
    Directory {
        tree: BlobLink,
    },
    Symlink {
        target: BlobLink,
    },
    External {
        kind: String,
        #[serde(with = "serde_bytes")]
        reference: Vec<u8>,
    },
}

/// Limits apply before decoding a node and throughout a verified closure walk.
/// A storage adapter must separately bound its allocation and fetch duration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TreeLimits {
    pub max_node_bytes: usize,
    pub max_entries_per_node: usize,
    pub max_depth: usize,
    pub max_total_entries: usize,
    pub max_total_bytes: usize,
    pub max_path_bytes: usize,
    pub max_pending_entries: usize,
}

impl Default for TreeLimits {
    fn default() -> Self {
        Self {
            max_node_bytes: 4 * 1024 * 1024,
            max_entries_per_node: 100_000,
            max_depth: 128,
            max_total_entries: 1_000_000,
            max_total_bytes: 1024 * 1024 * 1024,
            max_path_bytes: 4096,
            max_pending_entries: 4096,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TreeError {
    #[error("invalid tree: {0}")]
    Invalid(String),
    #[error("tree resource budget exceeded: {0}")]
    Budget(&'static str),
    #[error("tree closure incomplete at {cid}: {reason}")]
    Incomplete { cid: BlobCid, reason: String },
    #[error("tree integrity failure at {0}")]
    Integrity(BlobCid),
    #[error("tree storage failure at {cid}: {reason}")]
    Storage { cid: BlobCid, reason: String },
}

pub type TreeResult<T> = std::result::Result<T, TreeError>;

/// Fully verified bytes needed by an exact local restore or artifact adapter.
/// External boundaries are reported in their containing nodes and are never
/// claimed fetched by this loader.
#[derive(Debug, Clone)]
pub struct VerifiedTree {
    pub root: BlobCid,
    pub nodes: HashMap<BlobCid, TreeNode>,
    pub leaves: HashMap<BlobCid, Bytes>,
    pub external_paths: Vec<Vec<Vec<u8>>>,
}

impl VerifiedTree {
    pub fn root_node(&self) -> &TreeNode {
        &self.nodes[&self.root]
    }

    pub fn leaf(&self, cid: &BlobCid) -> Option<&Bytes> {
        self.leaves.get(cid)
    }

    pub fn external_count(&self) -> usize {
        self.external_paths.len()
    }
}

fn valid_name(name: &[u8]) -> bool {
    !name.is_empty() && name != b"." && name != b".." && !name.contains(&0) && !name.contains(&b'/')
}

fn validate_node(node: &TreeNode, limits: &TreeLimits) -> TreeResult<()> {
    if node.version != TREE_VERSION {
        return Err(TreeError::Invalid(format!(
            "unsupported tree version {}",
            node.version
        )));
    }
    if node.entries.len() > limits.max_entries_per_node {
        return Err(TreeError::Budget("entries per node"));
    }
    let mut last: Option<&[u8]> = None;
    for entry in &node.entries {
        if !valid_name(&entry.name) {
            return Err(TreeError::Invalid(
                "invalid single-component byte name".into(),
            ));
        }
        if last.is_some_and(|previous| previous >= entry.name.as_slice()) {
            return Err(TreeError::Invalid(
                "names must be unique and byte-sorted".into(),
            ));
        }
        last = Some(&entry.name);
        match &entry.kind {
            TreeEntryKind::File { blob, .. } | TreeEntryKind::Symlink { target: blob } => {
                if blob.as_blob_cid().as_cid().codec() != 0x55 {
                    return Err(TreeError::Invalid("leaf must use raw CID codec".into()));
                }
            }
            TreeEntryKind::Directory { tree } => {
                if tree.as_blob_cid().as_cid().codec() != 0x71 {
                    return Err(TreeError::Invalid(
                        "directory must use DAG-CBOR CID codec".into(),
                    ));
                }
            }
            TreeEntryKind::External { kind, reference } => {
                if kind.is_empty() || reference.is_empty() {
                    return Err(TreeError::Invalid(
                        "external boundary needs kind and reference".into(),
                    ));
                }
            }
        }
    }
    Ok(())
}

pub fn encode_tree(node: &TreeNode, limits: &TreeLimits) -> TreeResult<(BlobCid, Vec<u8>)> {
    validate_node(node, limits)?;
    let bytes = serde_ipld_dagcbor::to_vec(node)
        .map_err(|error| TreeError::Invalid(format!("DAG-CBOR encoding: {error}")))?;
    if bytes.len() > limits.max_node_bytes || bytes.len() > limits.max_total_bytes {
        return Err(TreeError::Budget("encoded node bytes"));
    }
    Ok((BlobCid::compute(&bytes), bytes))
}

pub fn decode_tree(bytes: &[u8], limits: &TreeLimits) -> TreeResult<TreeNode> {
    if bytes.len() > limits.max_node_bytes || bytes.len() > limits.max_total_bytes {
        return Err(TreeError::Budget("encoded node bytes"));
    }
    let node: TreeNode = serde_ipld_dagcbor::from_slice(bytes)
        .map_err(|error| TreeError::Invalid(format!("DAG-CBOR decoding: {error}")))?;
    validate_node(&node, limits)?;
    let canonical = serde_ipld_dagcbor::to_vec(&node)
        .map_err(|error| TreeError::Invalid(format!("DAG-CBOR encoding: {error}")))?;
    if canonical != bytes {
        return Err(TreeError::Invalid("noncanonical DAG-CBOR tree".into()));
    }
    Ok(node)
}

async fn fetch_checked<S: EprfsStorage>(
    storage: &S,
    cid: &BlobCid,
    policy: FetchPolicy,
) -> TreeResult<Bytes> {
    let handle = storage
        .fetch_blob(cid, policy)
        .await
        .map_err(|error| match error {
            crate::EprfsError::BlobNotFound(_) | crate::EprfsError::BlobNotLocal(_) => {
                TreeError::Incomplete {
                    cid: cid.clone(),
                    reason: error.to_string(),
                }
            }
            _ => TreeError::Storage {
                cid: cid.clone(),
                reason: error.to_string(),
            },
        })?;
    if &handle.cid != cid || !cid.verifies(&handle.bytes) {
        return Err(TreeError::Integrity(cid.clone()));
    }
    Ok(handle.bytes)
}

/// Fetch and verify every directory and raw leaf reachable from `root`.
/// Repeated links share bytes; every path still counts toward traversal limits.
pub async fn load_verified_tree<S: EprfsStorage>(
    storage: &S,
    root: &BlobCid,
    policy: FetchPolicy,
    limits: &TreeLimits,
) -> TreeResult<VerifiedTree> {
    if root.as_cid().codec() != 0x71 {
        return Err(TreeError::Invalid(
            "root must use DAG-CBOR CID codec".into(),
        ));
    }
    let mut result = VerifiedTree {
        root: root.clone(),
        nodes: HashMap::new(),
        leaves: HashMap::new(),
        external_paths: Vec::new(),
    };
    let mut pending = vec![(
        root.clone(),
        0usize,
        Vec::<BlobCid>::new(),
        Vec::<Vec<u8>>::new(),
        0usize,
    )];
    let mut pending_path_bytes = 0usize;
    let mut external_path_bytes = 0usize;
    let mut total_entries = 0usize;
    let mut total_bytes = 0usize;
    let mut node_sizes: HashMap<BlobCid, usize> = HashMap::new();
    while let Some((cid, depth, ancestors, path, path_bytes)) = pending.pop() {
        pending_path_bytes -= path_bytes;
        if depth > limits.max_depth {
            return Err(TreeError::Budget("tree depth"));
        }
        if ancestors.contains(&cid) {
            return Err(TreeError::Invalid("directory link cycle".into()));
        }
        let node = if let Some(node) = result.nodes.get(&cid) {
            total_bytes = total_bytes
                .checked_add(node_sizes[&cid])
                .ok_or(TreeError::Budget("expanded bytes"))?;
            if total_bytes > limits.max_total_bytes {
                return Err(TreeError::Budget("expanded bytes"));
            }
            node.clone()
        } else {
            let bytes = fetch_checked(storage, &cid, policy.clone()).await?;
            if bytes.len() > limits.max_node_bytes {
                return Err(TreeError::Budget("encoded node bytes"));
            }
            total_bytes = total_bytes
                .checked_add(bytes.len())
                .ok_or(TreeError::Budget("total bytes"))?;
            if total_bytes > limits.max_total_bytes {
                return Err(TreeError::Budget("total bytes"));
            }
            let node = decode_tree(&bytes, limits)?;
            node_sizes.insert(cid.clone(), bytes.len());
            result.nodes.insert(cid.clone(), node.clone());
            node
        };
        total_entries = total_entries
            .checked_add(node.entries.len())
            .ok_or(TreeError::Budget("total entries"))?;
        if total_entries > limits.max_total_entries {
            return Err(TreeError::Budget("total entries"));
        }
        let mut lineage = ancestors;
        lineage.push(cid);
        for entry in &node.entries {
            let child_path_bytes = path_bytes
                .checked_add(entry.name.len())
                .ok_or(TreeError::Budget("path bytes"))?;
            if child_path_bytes > limits.max_path_bytes {
                return Err(TreeError::Budget("path bytes"));
            }
            let mut child_path = path.clone();
            child_path.push(entry.name.clone());
            match &entry.kind {
                TreeEntryKind::Directory { tree } => {
                    if depth >= limits.max_depth {
                        return Err(TreeError::Budget("tree depth"));
                    }
                    if pending.len() >= limits.max_pending_entries {
                        return Err(TreeError::Budget("pending entries"));
                    }
                    pending_path_bytes = pending_path_bytes
                        .checked_add(child_path_bytes)
                        .ok_or(TreeError::Budget("pending path bytes"))?;
                    if pending_path_bytes > limits.max_total_bytes {
                        return Err(TreeError::Budget("pending path bytes"));
                    }
                    pending.push((
                        tree.as_blob_cid().clone(),
                        depth + 1,
                        lineage.clone(),
                        child_path,
                        child_path_bytes,
                    ));
                }
                TreeEntryKind::File { blob, .. } | TreeEntryKind::Symlink { target: blob } => {
                    let leaf_cid = blob.as_blob_cid();
                    if let Some(bytes) = result.leaves.get(leaf_cid) {
                        total_bytes = total_bytes
                            .checked_add(bytes.len())
                            .ok_or(TreeError::Budget("expanded bytes"))?;
                        if total_bytes > limits.max_total_bytes {
                            return Err(TreeError::Budget("expanded bytes"));
                        }
                    } else {
                        let bytes = fetch_checked(storage, leaf_cid, policy.clone()).await?;
                        total_bytes = total_bytes
                            .checked_add(bytes.len())
                            .ok_or(TreeError::Budget("total bytes"))?;
                        if total_bytes > limits.max_total_bytes {
                            return Err(TreeError::Budget("total bytes"));
                        }
                        result.leaves.insert(leaf_cid.clone(), bytes);
                    }
                }
                TreeEntryKind::External { .. } => {
                    external_path_bytes = external_path_bytes
                        .checked_add(child_path_bytes)
                        .ok_or(TreeError::Budget("external path bytes"))?;
                    if external_path_bytes > limits.max_total_bytes {
                        return Err(TreeError::Budget("external path bytes"));
                    }
                    result.external_paths.push(child_path);
                }
            }
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(name: &[u8], content: &[u8]) -> TreeEntry {
        TreeEntry {
            name: name.to_vec(),
            kind: TreeEntryKind::File {
                blob: BlobCid::compute_raw(content).into(),
                executable: false,
            },
        }
    }

    #[test]
    fn canonical_tree_sorts_raw_names_and_round_trips() {
        let a = file(b"a", b"a");
        let z = file(b"z", b"z");
        let first = TreeNode::new(vec![z.clone(), a.clone()]).unwrap();
        let second = TreeNode::new(vec![a, z]).unwrap();
        assert_eq!(first, second);
        let (cid, bytes) = encode_tree(&first, &TreeLimits::default()).unwrap();
        assert_eq!(cid, BlobCid::compute(&bytes));
        assert_eq!(decode_tree(&bytes, &TreeLimits::default()).unwrap(), first);
    }

    #[test]
    fn empty_tree_has_frozen_canonical_bytes() {
        // Canonical DAG-CBOR map: entries: [], version: 1. Independent byte vector.
        let (cid, bytes) =
            encode_tree(&TreeNode::new(vec![]).unwrap(), &TreeLimits::default()).unwrap();
        assert_eq!(
            cid.to_string(),
            "bafyreia7fco57n2u6rg7m4hym6hx7jdg3c7ga7bimukcoz5cxhizsvbble"
        );
        assert_eq!(
            bytes,
            [
                0xa2, 0x67, b'e', b'n', b't', b'r', b'i', b'e', b's', 0x80, 0x67, b'v', b'e', b'r',
                b's', b'i', b'o', b'n', 0x01
            ]
        );
    }

    #[test]
    fn raw_file_with_non_utf8_name_has_frozen_tag_42_tree_bytes_and_cid() {
        // Independently assembled DAG-CBOR: one File entry, byte name ff, and
        // the fixed raw hello-world CID in an IPLD tag-42 link position.
        let mut expected = vec![
            0xa2, 0x67, b'e', b'n', b't', b'r', b'i', b'e', b's', 0x81, 0xa2, 0x64, b'k', b'i',
            b'n', b'd', 0xa1, 0x64, b'f', b'i', b'l', b'e', 0xa2, 0x64, b'b', b'l', b'o', b'b',
        ];
        expected.extend_from_slice(&[
            0xd8, 0x2a, 0x58, 0x25, 0x00, 0x01, 0x55, 0x12, 0x20, 0xb9, 0x4d, 0x27, 0xb9, 0x93,
            0x4d, 0x3e, 0x08, 0xa5, 0x2e, 0x52, 0xd7, 0xda, 0x7d, 0xab, 0xfa, 0xc4, 0x84, 0xef,
            0xe3, 0x7a, 0x53, 0x80, 0xee, 0x90, 0x88, 0xf7, 0xac, 0xe2, 0xef, 0xcd, 0xe9,
        ]);
        expected.extend_from_slice(&[
            0x6a, b'e', b'x', b'e', b'c', b'u', b't', b'a', b'b', b'l', b'e', 0xf4, 0x64, b'n',
            b'a', b'm', b'e', 0x41, 0xff, 0x67, b'v', b'e', b'r', b's', b'i', b'o', b'n', 0x01,
        ]);
        let leaf =
            BlobCid::parse("bafkreifzjut3te2nhyekklss27nh3k72ysco7y32koao5eei66wof36n5e").unwrap();
        let node = TreeNode::new(vec![TreeEntry {
            name: vec![0xff],
            kind: TreeEntryKind::File {
                blob: leaf.into(),
                executable: false,
            },
        }])
        .unwrap();
        let (cid, bytes) = encode_tree(&node, &TreeLimits::default()).unwrap();
        assert_eq!(bytes, expected);
        assert_eq!(
            cid.to_string(),
            "bafyreihedxqhkuhibpz2uhoknm5ov2p4ubsjr5csvg4tdajfdapsyotrou"
        );
        assert_eq!(
            decode_tree(&expected, &TreeLimits::default()).unwrap(),
            node
        );
    }

    #[test]
    fn decode_refuses_unknown_version_field_and_noncanonical_order() {
        let (_, canonical) =
            encode_tree(&TreeNode::new(vec![]).unwrap(), &TreeLimits::default()).unwrap();
        let mut unsupported_version = canonical.clone();
        *unsupported_version.last_mut().unwrap() = 2;
        assert!(matches!(
            decode_tree(&unsupported_version, &TreeLimits::default()),
            Err(TreeError::Invalid(_))
        ));

        // The same map with reversed key order is valid CBOR but not canonical.
        let reversed = [
            0xa2, 0x67, b'v', b'e', b'r', b's', b'i', b'o', b'n', 0x01, 0x67, b'e', b'n', b't',
            b'r', b'i', b'e', b's', 0x80,
        ];
        assert!(matches!(
            decode_tree(&reversed, &TreeLimits::default()),
            Err(TreeError::Invalid(_))
        ));

        let unknown = [
            0xa3, 0x67, b'e', b'n', b't', b'r', b'i', b'e', b's', 0x80, 0x65, b'e', b'x', b't',
            b'r', b'a', 0x00, 0x67, b'v', b'e', b'r', b's', b'i', b'o', b'n', 0x01,
        ];
        assert!(matches!(
            decode_tree(&unknown, &TreeLimits::default()),
            Err(TreeError::Invalid(_))
        ));
    }

    #[test]
    fn rejects_invalid_names_and_duplicates() {
        for name in [
            b"".as_slice(),
            b".".as_slice(),
            b"..".as_slice(),
            b"a/b".as_slice(),
            b"a\0b".as_slice(),
        ] {
            assert!(TreeNode::new(vec![file(name, b"x")]).is_err());
        }
        assert!(TreeNode::new(vec![file(b"x", b"a"), file(b"x", b"b")]).is_err());
        assert!(TreeNode::new(vec![file(b"\xff", b"x")]).is_ok());
    }

    #[test]
    fn rejects_wrong_leaf_codec_and_unbounded_node() {
        let mut node = TreeNode::new(vec![file(b"x", b"x")]).unwrap();
        node.entries[0].kind = TreeEntryKind::File {
            blob: BlobCid::compute(b"x").into(),
            executable: false,
        };
        assert!(matches!(
            encode_tree(&node, &TreeLimits::default()),
            Err(TreeError::Invalid(_))
        ));
        let limits = TreeLimits {
            max_node_bytes: 1,
            ..TreeLimits::default()
        };
        assert!(matches!(
            encode_tree(&TreeNode::new(vec![]).unwrap(), &limits),
            Err(TreeError::Budget(_))
        ));
    }

    #[tokio::test]
    async fn closure_verifies_leaf_and_reports_external_boundary() {
        use crate::{AttestationDraft, BlobHandle, BlobPresence, EprRecord, EprRef, EprfsError};
        struct TestStorage(HashMap<BlobCid, Bytes>);
        #[async_trait::async_trait]
        impl EprfsStorage for TestStorage {
            async fn resolve_epr(&self, _: &EprRef) -> crate::Result<EprRecord> {
                unreachable!()
            }
            async fn has_blob(&self, cid: &BlobCid) -> crate::Result<BlobPresence> {
                Ok(if self.0.contains_key(cid) {
                    BlobPresence::Local
                } else {
                    BlobPresence::Missing
                })
            }
            async fn fetch_blob(&self, cid: &BlobCid, _: FetchPolicy) -> crate::Result<BlobHandle> {
                self.0
                    .get(cid)
                    .cloned()
                    .map(|bytes| BlobHandle {
                        cid: cid.clone(),
                        bytes,
                    })
                    .ok_or_else(|| EprfsError::BlobNotFound(cid.clone()))
            }
            async fn put_blob(&self, _: Bytes) -> crate::Result<BlobCid> {
                unreachable!()
            }
            async fn publish_attestation(&self, _: AttestationDraft) -> crate::Result<EprRef> {
                unreachable!()
            }
        }
        let mut store = TestStorage(HashMap::new());
        let leaf = Bytes::from_static(b"hello");
        let leaf_cid = BlobCid::compute_raw(&leaf);
        store.0.insert(leaf_cid.clone(), leaf.clone());
        let node = TreeNode::new(vec![
            file(b"hello", &leaf),
            TreeEntry {
                name: b"vendor".to_vec(),
                kind: TreeEntryKind::External {
                    kind: "opaque".into(),
                    reference: vec![0, 0xff],
                },
            },
        ])
        .unwrap();
        let (root, encoded) = encode_tree(&node, &TreeLimits::default()).unwrap();
        store.0.insert(root.clone(), Bytes::from(encoded));
        let loaded = load_verified_tree(
            &store,
            &root,
            FetchPolicy::LocalOnly,
            &TreeLimits::default(),
        )
        .await
        .unwrap();
        assert_eq!(loaded.root_node(), &node);
        assert_eq!(loaded.leaf(&leaf_cid), Some(&leaf));
        assert_eq!(loaded.external_paths, vec![vec![b"vendor".to_vec()]]);
    }

    #[tokio::test]
    async fn repeated_leaf_links_consume_expanded_byte_budget() {
        use crate::{AttestationDraft, BlobHandle, BlobPresence, EprRecord, EprRef, EprfsError};
        struct TestStorage(HashMap<BlobCid, Bytes>);
        #[async_trait::async_trait]
        impl EprfsStorage for TestStorage {
            async fn resolve_epr(&self, _: &EprRef) -> crate::Result<EprRecord> {
                unreachable!()
            }
            async fn has_blob(&self, cid: &BlobCid) -> crate::Result<BlobPresence> {
                Ok(if self.0.contains_key(cid) {
                    BlobPresence::Local
                } else {
                    BlobPresence::Missing
                })
            }
            async fn fetch_blob(&self, cid: &BlobCid, _: FetchPolicy) -> crate::Result<BlobHandle> {
                self.0
                    .get(cid)
                    .cloned()
                    .map(|bytes| BlobHandle {
                        cid: cid.clone(),
                        bytes,
                    })
                    .ok_or_else(|| EprfsError::BlobNotFound(cid.clone()))
            }
            async fn put_blob(&self, _: Bytes) -> crate::Result<BlobCid> {
                unreachable!()
            }
            async fn publish_attestation(&self, _: AttestationDraft) -> crate::Result<EprRef> {
                unreachable!()
            }
        }
        let leaf = Bytes::from(vec![42; 100]);
        let leaf_cid = BlobCid::compute_raw(&leaf);
        let node = TreeNode::new(vec![file(b"a", &leaf), file(b"b", &leaf)]).unwrap();
        let (root, encoded) = encode_tree(&node, &TreeLimits::default()).unwrap();
        let mut store = TestStorage(HashMap::new());
        store.0.insert(leaf_cid, leaf);
        let node_len = encoded.len();
        store.0.insert(root.clone(), Bytes::from(encoded));
        let limits = TreeLimits {
            max_total_bytes: node_len + 150,
            ..TreeLimits::default()
        };
        assert!(matches!(
            load_verified_tree(&store, &root, FetchPolicy::LocalOnly, &limits).await,
            Err(TreeError::Budget("expanded bytes"))
        ));
    }

    #[tokio::test]
    async fn long_directory_frontier_refuses_before_descendant_fetch() {
        use crate::{AttestationDraft, BlobHandle, BlobPresence, EprRecord, EprRef, EprfsError};
        struct RootOnly(HashMap<BlobCid, Bytes>);
        #[async_trait::async_trait]
        impl EprfsStorage for RootOnly {
            async fn resolve_epr(&self, _: &EprRef) -> crate::Result<EprRecord> {
                unreachable!()
            }
            async fn has_blob(&self, _: &BlobCid) -> crate::Result<BlobPresence> {
                unreachable!()
            }
            async fn fetch_blob(&self, cid: &BlobCid, _: FetchPolicy) -> crate::Result<BlobHandle> {
                self.0
                    .get(cid)
                    .cloned()
                    .map(|bytes| BlobHandle {
                        cid: cid.clone(),
                        bytes,
                    })
                    .ok_or_else(|| EprfsError::BlobNotFound(cid.clone()))
            }
            async fn put_blob(&self, _: Bytes) -> crate::Result<BlobCid> {
                unreachable!()
            }
            async fn publish_attestation(&self, _: AttestationDraft) -> crate::Result<EprRef> {
                unreachable!()
            }
        }
        let child = TreeNode::new(vec![]).unwrap();
        let (child_cid, _) = encode_tree(&child, &TreeLimits::default()).unwrap();
        let root_node = TreeNode::new(vec![TreeEntry {
            name: vec![b'x'; 80],
            kind: TreeEntryKind::Directory {
                tree: child_cid.into(),
            },
        }])
        .unwrap();
        let (root, encoded) = encode_tree(&root_node, &TreeLimits::default()).unwrap();
        let store = RootOnly(HashMap::from([(root.clone(), Bytes::from(encoded))]));
        let limits = TreeLimits {
            max_path_bytes: 40,
            ..TreeLimits::default()
        };
        assert!(matches!(
            load_verified_tree(&store, &root, FetchPolicy::LocalOnly, &limits).await,
            Err(TreeError::Budget("path bytes"))
        ));
    }
}
