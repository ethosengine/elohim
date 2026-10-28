//! Membership proofs for channel directories fetched through the existing tier protocol.
//!
//! Tier answers prove their own bytes and path. This module joins those answers from the signed
//! root down to a requested directory, so a self-consistent tree from another root is never folded.

use std::collections::{HashSet, VecDeque};
use std::sync::{Mutex, OnceLock};

use eprfs_core::{
    encode_tree, BlobCid, DirListing, TreeChange, TreeEntry, TreeEntryKind, TreeLimits, TreeNode,
    TreePath,
};
use iroh::endpoint::Connection;

use crate::error::PeerError;
use crate::tier::{tier_request, TierAnswer, TierRequest};

/// The number of verified events nodes retained for diff recomputation.
const VERIFIED_EVENTS_CACHE_LEN: usize = 64;

/// An events directory whose entries were joined to `root` through a CID-checked spine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VerifiedEventsNode {
    /// The channel root these entries were proven under.
    pub(crate) root: BlobCid,
    /// The CID of the events directory node.
    pub(crate) cid: BlobCid,
    /// The directory's canonical entries.
    pub(crate) entries: Vec<TreeEntry>,
}

/// The result of applying a supported diff to a verified events node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RecomputedEventsNode {
    /// The recomputed events node, keyed by the new root it represents.
    pub(crate) node: VerifiedEventsNode,
    /// File entries added by the diff.
    pub(crate) added: Vec<TreeEntry>,
}

/// Verify every directory from `root` to `path`, without listing the directory at `path`, and
/// return the CID the verified parent names for it. The root path itself is listed and checked.
///
/// # Errors
///
/// Returns the tier request errors, [`PeerError::FoldUnanswered`] for a non-list answer, or
/// [`PeerError::FoldIntegrity`] when a listing is not the node named by its verified parent.
pub(crate) async fn verify_spine(
    connection: &Connection,
    root: &BlobCid,
    path: &TreePath,
    tree: &TreeLimits,
) -> Result<BlobCid, PeerError> {
    let mut expected = root.clone();
    let mut parent = TreePath::root();
    let components = path.components();
    if components.is_empty() {
        verified_listing(connection, root, &parent, &expected, tree).await?;
        return Ok(expected);
    }

    for (index, component) in components.iter().enumerate() {
        let listing = verified_listing(connection, root, &parent, &expected, tree).await?;
        let entry = listing
            .entries
            .iter()
            .find(|entry| entry.name.as_slice() == component.as_slice())
            .ok_or_else(|| {
                integrity(format!(
                    "verified directory {} has no `{}` child",
                    parent,
                    String::from_utf8_lossy(component)
                ))
            })?;
        let TreeEntryKind::Directory { tree: child } = &entry.kind else {
            return Err(integrity(format!(
                "verified path {}/{} is not a directory",
                parent,
                String::from_utf8_lossy(component)
            )));
        };
        expected = child.as_blob_cid().clone();
        if index + 1 == components.len() {
            return Ok(expected);
        }
        parent = parent.join(component);
    }

    Ok(expected)
}

/// List `path` and require the self-consistent answer to have the CID its verified parent names.
///
/// # Errors
///
/// Returns the tier request errors, [`PeerError::FoldUnanswered`] for a non-list answer, or
/// [`PeerError::FoldIntegrity`] when the listing CID differs from `expected`.
pub(crate) async fn verified_listing(
    connection: &Connection,
    root: &BlobCid,
    path: &TreePath,
    expected: &BlobCid,
    tree: &TreeLimits,
) -> Result<DirListing, PeerError> {
    let request = TierRequest::List {
        root: root.clone(),
        path: path.clone(),
    };
    let listing = match tier_request(connection, &request, tree).await? {
        TierAnswer::List(listing) => listing,
        other => return Err(unanswered(format!("listing {path} under {root}"), &other)),
    };
    if listing.cid != *expected {
        return Err(integrity(format!(
            "listing {path} is {} but its verified parent names {expected}",
            listing.cid
        )));
    }
    Ok(listing)
}

/// Apply `changes` under `events_dir` to `previous`, re-encode the resulting node, and return it
/// only when every change is a direct file addition or removal consistent with the prior node.
/// The returned node is keyed by `new_root`, never by whichever head was most recently observed.
pub(crate) fn recompute_events_node(
    previous: &VerifiedEventsNode,
    new_root: &BlobCid,
    changes: &[TreeChange],
    events_dir: &TreePath,
    tree: &TreeLimits,
) -> Option<RecomputedEventsNode> {
    let mut entries = previous.entries.clone();
    let mut added = Vec::new();
    let mut changed_names = HashSet::new();
    for change in changes {
        let parts = change.path.components();
        let direct_child = parts.len() == events_dir.components().len() + 1
            && parts.starts_with(events_dir.components());
        if !direct_child {
            return None;
        }
        let name = parts.last().cloned().unwrap_or_default();
        if !changed_names.insert(name.clone()) {
            return None;
        }
        let position = entries.iter().position(|entry| entry.name == name);
        match (&change.old, &change.new, position) {
            (Some(old @ TreeEntryKind::File { .. }), None, Some(at))
                if &entries[at].kind == old =>
            {
                entries.remove(at);
            }
            (None, Some(kind @ TreeEntryKind::File { .. }), None) => {
                let entry = TreeEntry {
                    name,
                    kind: kind.clone(),
                };
                entries.push(entry.clone());
                added.push(entry);
            }
            _ => return None,
        }
    }
    let node = TreeNode::new(entries).ok()?;
    let (cid, _) = encode_tree(&node, tree).ok()?;
    Some(RecomputedEventsNode {
        node: VerifiedEventsNode {
            root: new_root.clone(),
            cid,
            entries: node.entries,
        },
        added,
    })
}

/// Return the verified events node cached for exactly `root`.
pub(crate) fn cached_events_node(root: &BlobCid) -> Option<VerifiedEventsNode> {
    verified_events_cache()
        .lock()
        .ok()?
        .iter()
        .find(|node| node.root == *root)
        .cloned()
}

/// Retain `node` for later diff recomputation, evicting the oldest distinct root at the bound.
pub(crate) fn remember_events_node(node: VerifiedEventsNode) {
    let Ok(mut cache) = verified_events_cache().lock() else {
        return;
    };
    if let Some(at) = cache.iter().position(|held| held.root == node.root) {
        cache.remove(at);
    }
    cache.push_back(node);
    while cache.len() > VERIFIED_EVENTS_CACHE_LEN {
        cache.pop_front();
    }
}

fn verified_events_cache() -> &'static Mutex<VecDeque<VerifiedEventsNode>> {
    static CACHE: OnceLock<Mutex<VecDeque<VerifiedEventsNode>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(VecDeque::new()))
}

fn integrity(reason: String) -> PeerError {
    PeerError::FoldIntegrity { reason }
}

fn unanswered(what: String, answer: &TierAnswer) -> PeerError {
    let why = match answer {
        TierAnswer::Failed { reason } => format!(" ({reason})"),
        TierAnswer::Unanswered { .. } => " (unanswered)".to_string(),
        _ => " (an answer of another kind)".to_string(),
    };
    PeerError::FoldUnanswered {
        what: format!("{what}{why}"),
    }
}
