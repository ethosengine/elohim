//! The channel tree: a projection of this device's crossing events and nack leaves, and the store
//! the tier ALPN serves them from (spec §3.3).
//!
//! Events sit at `channel/events/<event-cid>`; each leaf is one FlowEvent's canonical dag-cbor
//! bytes, held under the raw codec eprfs-core requires of leaves, so a leaf's digest is the digest
//! of the event's own atom CID, which names it. Nacks sit at `channel/nacks/<author>` and name the
//! event CIDs this device refused from that author. The tree is rebuilt from the flow store, the
//! allow-list and private nack state, never edited: Ephemeral (C), with those records as its source.
//!
//! [`ChannelStore`] is the only store `epr peer serve` hands the tier. It holds channel trees and
//! nothing else — never the repository's eprfs store, never recall, session or memory stores — so
//! an admitted device can read only what crossed (carry ruling 5). It keeps the last few roots so
//! a peer can diff from the root it last folded; an older base is answered as unknown and the peer
//! lists the new root instead.

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::sync::RwLock;

use async_trait::async_trait;
use bytes::Bytes;
use cid::Cid;
use elohim_epr_rea::model::canonical_bytes;
use elohim_epr_rea::FlowStore;
use eprfs_core::{
    encode_tree, AttestationDraft, BlobCid, BlobHandle, BlobPresence, EprRecord, EprRef,
    EprfsError, EprfsStorage, FetchPolicy, TierId, TreeEntry, TreeEntryKind, TreeLimits, TreeNode,
};

use crate::allow::CrossingAllowList;
use crate::error::PeerError;
use crate::head_table::NackLeaf;

/// Where the events sit in a channel tree.
pub const CHANNEL_EVENTS: &str = "channel/events";

/// Where this device tells each author which records it did not accept.
pub const CHANNEL_NACKS: &str = "channel/nacks";

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
    events: usize,
    /// How many flow events of the store the tree was projected from, crossing or not.
    read: u64,
    /// The CID of the last of them.
    last: Option<Cid>,
}

impl ChannelTree {
    pub fn root(&self) -> &BlobCid {
        &self.root
    }

    /// How many events crossed.
    pub fn events(&self) -> usize {
        self.events
    }

    /// How many flow events of the store it was projected from, crossing or not: the store is
    /// append-only, so the tree covers exactly its first `read` events.
    pub fn read(&self) -> u64 {
        self.read
    }

    /// The CID of the last flow event it was projected from, `None` for an empty store: with
    /// [`Self::read`], the store's identity at that position.
    pub fn last(&self) -> Option<&Cid> {
        self.last.as_ref()
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
    project_channel_with_nacks(store, allow, &BTreeMap::new())
}

/// [`project_channel`] with this device's nack leaves. A leaf is present only for an author with
/// a non-empty refusal set, and its canonical JSON changes only when that set changes.
pub fn project_channel_with_nacks<S>(
    store: &S,
    allow: &CrossingAllowList,
    nacks: &BTreeMap<String, NackLeaf>,
) -> Result<ChannelTree, PeerError>
where
    S: FlowStore + ?Sized,
{
    let tree_error = |reason: String| PeerError::ChannelTree { reason };
    let mut leaves = Vec::new();
    let mut entries = Vec::new();
    let mut seen = HashSet::new();
    let events = store.events().map_err(PeerError::FlowStore)?;
    let read = u64::try_from(events.len()).unwrap_or(u64::MAX);
    let last = events.last().map(|(cid, _)| *cid);
    for (cid, event) in events {
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
    let mut nodes = Vec::with_capacity(4);
    let events = leaves.len();
    let events_node = node(entries, &mut nodes)?;
    let mut nack_entries = Vec::new();
    for (device, nack) in nacks {
        if nack.refused_total == 0 {
            continue;
        }
        let bytes = serde_json::to_vec(nack).map_err(|e| tree_error(e.to_string()))?;
        let leaf = BlobCid::compute_raw(&bytes);
        nack_entries.push(TreeEntry {
            name: device.as_bytes().to_vec(),
            kind: TreeEntryKind::File {
                blob: leaf.clone().into(),
                executable: false,
            },
        });
        leaves.push((leaf, Bytes::from(bytes)));
    }
    let nacks_node = node(nack_entries, &mut nodes)?;
    let channel = node(
        vec![
            TreeEntry {
                name: b"events".to_vec(),
                kind: TreeEntryKind::Directory {
                    tree: events_node.into(),
                },
            },
            TreeEntry {
                name: b"nacks".to_vec(),
                kind: TreeEntryKind::Directory {
                    tree: nacks_node.into(),
                },
            },
        ],
        &mut nodes,
    )?;
    let child = node(
        vec![TreeEntry {
            name: b"channel".to_vec(),
            kind: TreeEntryKind::Directory {
                tree: channel.into(),
            },
        }],
        &mut nodes,
    )?;
    Ok(ChannelTree {
        root: child,
        nodes,
        leaves,
        events,
        read,
        last,
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
