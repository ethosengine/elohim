//! The channel tree: a projection of this device's crossing events, and the store the tier ALPN
//! serves it from (spec §3.3).
//!
//! The tree is `channel/events/<event-cid>`; each leaf is one FlowEvent's canonical dag-cbor
//! bytes, held under the raw codec eprfs-core requires of leaves, so a leaf's digest is the digest
//! of the event's own atom CID, which names it. It is rebuilt from the flow store and the
//! allow-list, never edited: Ephemeral (C), with the flow store as its source of truth.
//!
//! [`ChannelStore`] is the only store `epr peer serve` hands the tier. It holds channel trees and
//! nothing else — never the repository's eprfs store, never recall, session or memory stores — so
//! an admitted device can read only what crossed (carry ruling 5). It keeps the last few roots so
//! a peer can diff from the root it last folded; an older base is answered as unknown and the peer
//! lists the new root instead.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::RwLock;

use async_trait::async_trait;
use bytes::Bytes;
use elohim_epr_rea::model::canonical_bytes;
use elohim_epr_rea::FlowStore;
use eprfs_core::{
    encode_tree, AttestationDraft, BlobCid, BlobHandle, BlobPresence, EprRecord, EprRef,
    EprfsError, EprfsStorage, FetchPolicy, TierId, TreeEntry, TreeEntryKind, TreeLimits, TreeNode,
};

use crate::allow::CrossingAllowList;
use crate::error::PeerError;

/// Where the events sit in a channel tree.
pub const CHANNEL_EVENTS: &str = "channel/events";

/// How many recent roots a [`ChannelStore`] keeps by default.
pub const RETAINED_ROOTS: usize = 16;

/// The identity the channel's tier stamps on what it answers, strangers' refusals included. It
/// names no device and no person: a constant for the protocol, never the device's did:key or the
/// lineage's chain root (T4 review, minor 1).
pub fn channel_tier_id() -> TierId {
    TierId::agent("elohim/steward-channel/tier")
}

/// One projection of a device's crossing events: its root and every node and leaf under it.
#[derive(Debug, Clone)]
pub struct ChannelTree {
    root: BlobCid,
    nodes: Vec<(BlobCid, Bytes)>,
    leaves: Vec<(BlobCid, Bytes)>,
}

impl ChannelTree {
    pub fn root(&self) -> &BlobCid {
        &self.root
    }

    /// How many events crossed.
    pub fn events(&self) -> usize {
        self.leaves.len()
    }

    fn blobs(&self) -> impl Iterator<Item = &(BlobCid, Bytes)> {
        self.nodes.iter().chain(&self.leaves)
    }
}

/// Project the crossing events of `store` (the device's SidecarFlowStore) into a channel tree.
/// Only events `allow` admits are placed in it: an event of any other measure or kind, and every
/// event folded from another device, stays local, whatever else the store holds.
///
/// # Errors
///
/// [`PeerError::FlowStore`] when the store cannot be read (a tampered line is an integrity error,
/// never skipped); [`PeerError::ChannelTree`] when an event's bytes do not hash to its stored CID
/// or the tree exceeds the tree limits.
pub fn project_channel<S>(store: &S, allow: &CrossingAllowList) -> Result<ChannelTree, PeerError>
where
    S: FlowStore + ?Sized,
{
    let tree_error = |reason: String| PeerError::ChannelTree { reason };
    let mut leaves = Vec::new();
    let mut entries = Vec::new();
    let mut seen = HashSet::new();
    for (cid, event) in store.events().map_err(PeerError::FlowStore)? {
        if !allow.admits(&event) || !seen.insert(cid) {
            continue;
        }
        let bytes = canonical_bytes(&event).map_err(PeerError::FlowStore)?;
        if BlobCid::compute(&bytes).as_cid() != &cid {
            return Err(tree_error(format!(
                "event {cid} does not re-encode to its own CID"
            )));
        }
        // A tree leaf is raw bytes (eprfs-core requires the raw codec); its digest is the event
        // CID's digest, and the entry's name is the event CID itself.
        let leaf = BlobCid::compute_raw(&bytes);
        entries.push(TreeEntry {
            name: cid.to_string().into_bytes(),
            kind: TreeEntryKind::File {
                blob: leaf.clone().into(),
                executable: false,
            },
        });
        leaves.push((leaf, Bytes::from(bytes)));
    }
    let mut nodes = Vec::with_capacity(3);
    let mut child = node(entries, &mut nodes)?;
    // channel/events, then channel, then the root: each a directory of the one below it.
    for name in ["events", "channel"] {
        child = node(
            vec![TreeEntry {
                name: name.as_bytes().to_vec(),
                kind: TreeEntryKind::Directory { tree: child.into() },
            }],
            &mut nodes,
        )?;
    }
    Ok(ChannelTree {
        root: child,
        nodes,
        leaves,
    })
}

fn node(entries: Vec<TreeEntry>, nodes: &mut Vec<(BlobCid, Bytes)>) -> Result<BlobCid, PeerError> {
    let tree_error = |e: eprfs_core::TreeError| PeerError::ChannelTree {
        reason: e.to_string(),
    };
    let node = TreeNode::new(entries).map_err(tree_error)?;
    let (cid, bytes) = encode_tree(&node, &TreeLimits::default()).map_err(tree_error)?;
    nodes.push((cid.clone(), Bytes::from(bytes)));
    Ok(cid)
}

/// The store a device's tier serves: its recent channel trees and nothing else.
///
/// Written only by [`ChannelStore::install`]; the storage seam's own writes are refused, so
/// nothing reaches what is served except a projection.
#[derive(Debug)]
pub struct ChannelStore {
    retain: usize,
    held: RwLock<Held>,
}

#[derive(Debug, Default)]
struct Held {
    blobs: HashMap<BlobCid, Bytes>,
    /// Retained roots, oldest first, each with every blob under it.
    roots: VecDeque<(BlobCid, Vec<BlobCid>)>,
}

impl Default for ChannelStore {
    fn default() -> ChannelStore {
        ChannelStore::new(RETAINED_ROOTS)
    }
}

impl ChannelStore {
    /// A store keeping the last `retain` roots installed (at least one).
    pub fn new(retain: usize) -> ChannelStore {
        ChannelStore {
            retain: retain.max(1),
            held: RwLock::new(Held::default()),
        }
    }

    /// Serve `tree`. The oldest root past the retained count is dropped with every blob no
    /// retained root still holds.
    pub fn install(&self, tree: &ChannelTree) {
        let Ok(mut held) = self.held.write() else {
            // A poisoned lock serves nothing new; the tier keeps answering what it held.
            return;
        };
        if let Some(at) = held.roots.iter().position(|(root, _)| root == &tree.root) {
            if let Some(again) = held.roots.remove(at) {
                held.roots.push_back(again);
            }
            return;
        }
        for (cid, bytes) in tree.blobs() {
            held.blobs.insert(cid.clone(), bytes.clone());
        }
        let under = tree.blobs().map(|(cid, _)| cid.clone()).collect();
        held.roots.push_back((tree.root.clone(), under));
        if held.roots.len() <= self.retain {
            return;
        }
        held.roots.pop_front();
        let kept: HashSet<BlobCid> = held
            .roots
            .iter()
            .flat_map(|(_, under)| under.iter().cloned())
            .collect();
        held.blobs.retain(|cid, _| kept.contains(cid));
    }

    /// The roots served now, oldest first.
    pub fn roots(&self) -> Vec<BlobCid> {
        self.held
            .read()
            .map(|held| held.roots.iter().map(|(root, _)| root.clone()).collect())
            .unwrap_or_default()
    }

    fn get(&self, cid: &BlobCid) -> Option<Bytes> {
        self.held.read().ok()?.blobs.get(cid).cloned()
    }
}

#[async_trait]
impl EprfsStorage for ChannelStore {
    async fn resolve_epr(&self, reference: &EprRef) -> eprfs_core::Result<EprRecord> {
        Err(EprfsError::EprNotFound(reference.clone()))
    }

    async fn has_blob(&self, cid: &BlobCid) -> eprfs_core::Result<BlobPresence> {
        Ok(if self.get(cid).is_some() {
            BlobPresence::Local
        } else {
            BlobPresence::Missing
        })
    }

    async fn fetch_blob(
        &self,
        cid: &BlobCid,
        _policy: FetchPolicy,
    ) -> eprfs_core::Result<BlobHandle> {
        let bytes = self
            .get(cid)
            .ok_or_else(|| EprfsError::BlobNotFound(cid.clone()))?;
        Ok(BlobHandle {
            cid: cid.clone(),
            bytes,
        })
    }

    async fn put_blob(&self, _bytes: Bytes) -> eprfs_core::Result<BlobCid> {
        Err(EprfsError::Storage(
            "the channel store holds projections only".into(),
        ))
    }

    async fn publish_attestation(&self, _draft: AttestationDraft) -> eprfs_core::Result<EprRef> {
        Err(EprfsError::Storage(
            "the channel store publishes nothing".into(),
        ))
    }
}
