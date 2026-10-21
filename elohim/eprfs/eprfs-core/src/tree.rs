//! Immutable, artifact-neutral, content-addressed filesystem trees.
//!
//! A tree is a private byte value. Its CID proves bytes, not publication,
//! standing, or the election of any release head.

use std::collections::HashMap;

use bytes::Bytes;
use seam_contracts::Unanswered;
use serde::{Deserialize, Serialize};

use crate::tier::ItemAbsence;
use crate::walk::TreePath;
use crate::{BatchItem, BatchOptions, BlobCid, BlobLink, EprfsStorage, FetchPolicy};

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

/// Non-exhaustive: the walk's remote tiers add refusals without breaking a
/// caller. Marking it, and the path variants slice 1 added, is a breaking
/// change from 0.1.2, so the next publication of this crate is 0.2.0.
/// `Clone`, so a spine node's failure shared by many paths is copied whole, never re-typed.
#[derive(Debug, Clone, thiserror::Error)]
#[non_exhaustive]
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
    #[error("path not found under root: {path}")]
    PathNotFound { path: TreePath },
    #[error("path is not a directory: {path}")]
    NotADirectory { path: TreePath },
    #[error("path is not a file or symlink: {path}")]
    NotALeaf { path: TreePath },
    #[error("path crosses an external boundary at {path} ({kind})")]
    ExternalBoundary { path: TreePath, kind: String },
    #[error("invalid view: {0}")]
    InvalidView(String),
    /// A tier did not answer for the object at `path`, with ruling 7's three parts. A single
    /// answer (read, list, diff) returns it in place of `Incomplete` for a node or leaf it needs
    /// and no tier gave (B1).
    /// Boxed so every `TreeResult` stays small (clippy `result_large_err`).
    #[error("unanswered at {path} ({cid}): {}", why.why.kind())]
    Unanswered {
        cid: BlobCid,
        path: TreePath,
        why: Box<Unanswered<ItemAbsence>>,
    },
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

pub(crate) fn valid_name(name: &[u8]) -> bool {
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

/// The one rule every walk applies to a tier's named absence (B4), in one place so the
/// depth-first walk, its queries and [`crate::LevelWalk::absent`] cannot disagree: bytes that
/// failed their address are [`TreeError::Integrity`], and a failure of the own store (the only
/// party that can name the indexical `own`, which never crosses the wire) is
/// [`TreeError::Storage`]. Every other absence stays an absence (`None`), and the walk continues.
pub(crate) fn absence_error(cid: &BlobCid, why: &ItemAbsence) -> Option<TreeError> {
    match why {
        ItemAbsence::Integrity { .. } => Some(TreeError::Integrity(cid.clone())),
        ItemAbsence::Unreachable { tier } if tier.is_own() => Some(TreeError::Storage {
            cid: cid.clone(),
            reason: "the own store failed to read the object".into(),
        }),
        _ => None,
    }
}

/// Fetch one object named by its place under `root`, verified against its address. Every node
/// and leaf fetch carries its root and path, so a forwarding tier can check reach and membership
/// (A8). A tier's named absence becomes [`TreeError::Unanswered`], except what
/// [`absence_error`] makes an error; a leaf store's miss stays [`TreeError::Incomplete`]; a
/// stored object that fails its address ([`crate::EprfsError::Corrupt`]) is
/// [`TreeError::Integrity`]; any other failure is [`TreeError::Storage`].
pub(crate) async fn fetch_checked<S: EprfsStorage + ?Sized>(
    storage: &S,
    root: &BlobCid,
    item: &BatchItem,
    opts: &BatchOptions,
) -> TreeResult<Bytes> {
    let bytes = fetch_unverified(storage, root, item, opts).await?;
    if !item.cid.verifies(&bytes) {
        return Err(TreeError::Integrity(item.cid.clone()));
    }
    Ok(bytes)
}

/// [`fetch_checked`] without the final hash: the error mapping and the handle's CID check. Its
/// one other caller, [`fetch_node`], verifies through [`accept_node`].
async fn fetch_unverified<S: EprfsStorage + ?Sized>(
    storage: &S,
    root: &BlobCid,
    item: &BatchItem,
    opts: &BatchOptions,
) -> TreeResult<Bytes> {
    let cid = &item.cid;
    let handle = storage
        .fetch_at(root, item, opts)
        .await
        .map_err(|error| match error {
            crate::EprfsError::BlobNotFound(_) | crate::EprfsError::BlobNotLocal(_) => {
                TreeError::Incomplete {
                    cid: cid.clone(),
                    reason: error.to_string(),
                }
            }
            crate::EprfsError::Corrupt(_) => TreeError::Integrity(cid.clone()),
            crate::EprfsError::Unanswered { why, .. } => absence_error(cid, &why.why)
                .unwrap_or_else(|| TreeError::Unanswered {
                    cid: cid.clone(),
                    path: item.path.clone(),
                    why,
                }),
            _ => TreeError::Storage {
                cid: cid.clone(),
                reason: error.to_string(),
            },
        })?;
    if &handle.cid != cid {
        return Err(TreeError::Integrity(cid.clone()));
    }
    Ok(handle.bytes)
}

/// The one node-acceptance rule every traversal shares (the depth-first walk, the level walk and
/// the membership resolver): verify `bytes` against `cid`, refuse a node over `max_node_bytes`,
/// charge the bytes, decode canonically. The outer `Err` is the shared budget spent, which ends
/// the traversal; the inner `Err` is this node failing (`Integrity`, `Budget("encoded node
/// bytes")`, `Invalid`), which a caller resolving many paths gives to each path through it.
/// Entries are charged by the caller, where it expands the node.
pub(crate) fn accept_node(
    budget: &mut Budget<'_>,
    cid: &BlobCid,
    bytes: &[u8],
) -> TreeResult<TreeResult<(TreeNode, usize)>> {
    if !cid.verifies(bytes) {
        return Ok(Err(TreeError::Integrity(cid.clone())));
    }
    if bytes.len() > budget.limits().max_node_bytes {
        return Ok(Err(TreeError::Budget("encoded node bytes")));
    }
    budget.charge_fetched(bytes.len())?;
    Ok(decode_tree(bytes, budget.limits()).map(|node| (node, bytes.len())))
}

/// Traversal accounting shared by the closure loader and the view-scoped walk,
/// read, list and diff. Labels are stable: callers and tests match on them.
pub(crate) struct Budget<'l> {
    limits: &'l TreeLimits,
    total_entries: usize,
    total_bytes: usize,
    pending_path_bytes: usize,
    external_path_bytes: usize,
    result_bytes: usize,
}

impl<'l> Budget<'l> {
    pub(crate) fn new(limits: &'l TreeLimits) -> Self {
        Self {
            limits,
            total_entries: 0,
            total_bytes: 0,
            pending_path_bytes: 0,
            external_path_bytes: 0,
            result_bytes: 0,
        }
    }

    pub(crate) fn limits(&self) -> &'l TreeLimits {
        self.limits
    }

    /// A visited node's depth.
    pub(crate) fn check_depth(&self, depth: usize) -> TreeResult<()> {
        if depth > self.limits.max_depth {
            return Err(TreeError::Budget("tree depth"));
        }
        Ok(())
    }

    /// A directory child about to be queued below a node at `depth`.
    pub(crate) fn check_child_depth(&self, depth: usize) -> TreeResult<()> {
        if depth >= self.limits.max_depth {
            return Err(TreeError::Budget("tree depth"));
        }
        Ok(())
    }

    /// Bytes fetched from storage for the first time in this traversal.
    pub(crate) fn charge_fetched(&mut self, len: usize) -> TreeResult<()> {
        self.charge_bytes(len, "total bytes")
    }

    /// Bytes already held, reached again by another path.
    pub(crate) fn charge_reused(&mut self, len: usize) -> TreeResult<()> {
        self.charge_bytes(len, "expanded bytes")
    }

    fn charge_bytes(&mut self, len: usize, label: &'static str) -> TreeResult<()> {
        self.total_bytes = self
            .total_bytes
            .checked_add(len)
            .ok_or(TreeError::Budget(label))?;
        if self.total_bytes > self.limits.max_total_bytes {
            return Err(TreeError::Budget(label));
        }
        Ok(())
    }

    pub(crate) fn charge_entries(&mut self, count: usize) -> TreeResult<()> {
        self.total_entries = self
            .total_entries
            .checked_add(count)
            .ok_or(TreeError::Budget("total entries"))?;
        if self.total_entries > self.limits.max_total_entries {
            return Err(TreeError::Budget("total entries"));
        }
        Ok(())
    }

    pub(crate) fn child_path_bytes(&self, parent: usize, name_len: usize) -> TreeResult<usize> {
        let child = parent
            .checked_add(name_len)
            .ok_or(TreeError::Budget("path bytes"))?;
        if child > self.limits.max_path_bytes {
            return Err(TreeError::Budget("path bytes"));
        }
        Ok(child)
    }

    /// Admit one more queued directory whose path is `path_bytes` long.
    pub(crate) fn push_pending(&mut self, pending_len: usize, path_bytes: usize) -> TreeResult<()> {
        if pending_len >= self.limits.max_pending_entries {
            return Err(TreeError::Budget("pending entries"));
        }
        self.pending_path_bytes = self
            .pending_path_bytes
            .checked_add(path_bytes)
            .ok_or(TreeError::Budget("pending path bytes"))?;
        if self.pending_path_bytes > self.limits.max_total_bytes {
            return Err(TreeError::Budget("pending path bytes"));
        }
        Ok(())
    }

    /// Admit one more queued directory's path bytes without the count limit: the
    /// level-batched walk is bounded by its level chunks instead (B13).
    pub(crate) fn push_pending_bytes(&mut self, path_bytes: usize) -> TreeResult<()> {
        self.pending_path_bytes = self
            .pending_path_bytes
            .checked_add(path_bytes)
            .ok_or(TreeError::Budget("pending path bytes"))?;
        if self.pending_path_bytes > self.limits.max_total_bytes {
            return Err(TreeError::Budget("pending path bytes"));
        }
        Ok(())
    }

    pub(crate) fn pop_pending(&mut self, path_bytes: usize) {
        self.pending_path_bytes -= path_bytes;
    }

    /// Bytes a gathered answer retains that no fetched node already accounts
    /// for: the path of each node or change it keeps. A shared subtree costs
    /// its node bytes once per path, but every path it is kept under is new.
    pub(crate) fn charge_result(&mut self, len: usize) -> TreeResult<()> {
        self.result_bytes = self
            .result_bytes
            .checked_add(len)
            .ok_or(TreeError::Budget("result path bytes"))?;
        if self.result_bytes > self.limits.max_total_bytes {
            return Err(TreeError::Budget("result path bytes"));
        }
        Ok(())
    }

    pub(crate) fn charge_external_path(&mut self, path_bytes: usize) -> TreeResult<()> {
        self.external_path_bytes = self
            .external_path_bytes
            .checked_add(path_bytes)
            .ok_or(TreeError::Budget("external path bytes"))?;
        if self.external_path_bytes > self.limits.max_total_bytes {
            return Err(TreeError::Budget("external path bytes"));
        }
        Ok(())
    }
}

/// Fetch one directory node, verify it against `cid`, charge its bytes and
/// decode it canonically. Returns the node and its encoded length.
pub(crate) async fn fetch_node<S: EprfsStorage + ?Sized>(
    storage: &S,
    root: &BlobCid,
    item: &BatchItem,
    opts: &BatchOptions,
    budget: &mut Budget<'_>,
) -> TreeResult<(TreeNode, usize)> {
    let bytes = fetch_unverified(storage, root, item, opts).await?;
    accept_node(budget, &item.cid, &bytes)?
}

pub(crate) fn check_root_codec(root: &BlobCid) -> TreeResult<()> {
    if root.as_cid().codec() != 0x71 {
        return Err(TreeError::Invalid(
            "root must use DAG-CBOR CID codec".into(),
        ));
    }
    Ok(())
}

/// Fetch and verify every directory and raw leaf reachable from `root`.
/// Repeated links share bytes; every path still counts toward traversal limits.
pub async fn load_verified_tree<S: EprfsStorage + ?Sized>(
    storage: &S,
    root: &BlobCid,
    policy: FetchPolicy,
    limits: &TreeLimits,
) -> TreeResult<VerifiedTree> {
    check_root_codec(root)?;
    let opts = BatchOptions::from(policy);
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
    let mut budget = Budget::new(limits);
    let mut node_sizes: HashMap<BlobCid, usize> = HashMap::new();
    while let Some((cid, depth, ancestors, path, path_bytes)) = pending.pop() {
        budget.pop_pending(path_bytes);
        budget.check_depth(depth)?;
        if ancestors.contains(&cid) {
            return Err(TreeError::Invalid("directory link cycle".into()));
        }
        let node = if let Some(node) = result.nodes.get(&cid) {
            budget.charge_reused(node_sizes[&cid])?;
            node.clone()
        } else {
            let item = BatchItem::new(cid.clone(), tree_path(&path));
            let (node, len) = fetch_node(storage, root, &item, &opts, &mut budget).await?;
            node_sizes.insert(cid.clone(), len);
            result.nodes.insert(cid.clone(), node.clone());
            node
        };
        budget.charge_entries(node.entries.len())?;
        let mut lineage = ancestors;
        lineage.push(cid);
        for entry in &node.entries {
            let child_path_bytes = budget.child_path_bytes(path_bytes, entry.name.len())?;
            let mut child_path = path.clone();
            child_path.push(entry.name.clone());
            match &entry.kind {
                TreeEntryKind::Directory { tree } => {
                    budget.check_child_depth(depth)?;
                    budget.push_pending(pending.len(), child_path_bytes)?;
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
                        budget.charge_reused(bytes.len())?;
                    } else {
                        let item = BatchItem::new(leaf_cid.clone(), tree_path(&child_path));
                        let bytes = fetch_checked(storage, root, &item, &opts).await?;
                        budget.charge_fetched(bytes.len())?;
                        result.leaves.insert(leaf_cid.clone(), bytes);
                    }
                }
                TreeEntryKind::External { .. } => {
                    budget.charge_external_path(child_path_bytes)?;
                    result.external_paths.push(child_path);
                }
            }
        }
    }
    Ok(result)
}

/// The closure loader's raw component path as a [`TreePath`]. Every component was validated by
/// `decode_tree`, so this never needs the checking constructor.
fn tree_path(components: &[Vec<u8>]) -> TreePath {
    TreePath::from_trusted(components.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{dir, file, CountingStorage};

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
        let mut store = CountingStorage::new();
        let leaf = Bytes::from_static(b"hello");
        let leaf_cid = store.put_leaf(&leaf);
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
        let root = store.put_tree(node.entries.clone());
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
        let leaf = Bytes::from(vec![42; 100]);
        let mut store = CountingStorage::new();
        store.put_leaf(&leaf);
        let node = TreeNode::new(vec![file(b"a", &leaf), file(b"b", &leaf)]).unwrap();
        let (_, encoded) = encode_tree(&node, &TreeLimits::default()).unwrap();
        let root = store.put_tree(node.entries);
        let limits = TreeLimits {
            max_total_bytes: encoded.len() + 150,
            ..TreeLimits::default()
        };
        assert!(matches!(
            load_verified_tree(&store, &root, FetchPolicy::LocalOnly, &limits).await,
            Err(TreeError::Budget("expanded bytes"))
        ));
    }

    #[tokio::test]
    async fn long_directory_frontier_refuses_before_descendant_fetch() {
        let (child_cid, _) =
            encode_tree(&TreeNode::new(vec![]).unwrap(), &TreeLimits::default()).unwrap();
        // Only the root is held; the child must never be asked for.
        let mut store = CountingStorage::new();
        let root = store.put_tree(vec![dir(&[b'x'; 80], child_cid)]);
        let limits = TreeLimits {
            max_path_bytes: 40,
            ..TreeLimits::default()
        };
        assert!(matches!(
            load_verified_tree(&store, &root, FetchPolicy::LocalOnly, &limits).await,
            Err(TreeError::Budget("path bytes"))
        ));
        assert_eq!(store.fetched(), vec![root]);
    }
}
