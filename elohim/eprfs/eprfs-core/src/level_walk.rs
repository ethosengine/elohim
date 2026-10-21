//! The level-batched walk (interfaces spec §B.1): a sans-I/O state machine with the depth-first
//! walk's view matching, budget and verification, and no storage reference.
//!
//! [`walk`](crate::walk::walk) fetches one node per round trip. A [`LevelWalk`] yields a whole
//! tree level's wanted nodes at once, so [`walk_batched`] costs one
//! [`EprfsStorage::fetch_blobs`] per level chunk: depth + 1 round trips for the spine, against
//! one per node. Its result equals [`walk_collect`](crate::walk::walk_collect)'s on every view:
//! the same node set (re-sorted to pre-order), the same absences and boundaries in the same
//! order, and the same errors, with one bounded difference: the depth-first walk refuses a
//! stack of more than `max_pending_entries` queued directories (`"pending entries"`), and the
//! level walk, which must hold a whole level, refuses instead when its queued frames' retained
//! size passes `max_total_bytes` (`"pending path bytes"`, B13).

use std::collections::{HashMap, HashSet};

use bytes::Bytes;
use seam_contracts::Unanswered;

use crate::address::BlobCid;
use crate::reach::{ReachReason, ReachRefusal};
use crate::storage::{
    Awaiting, BatchItem, BatchOptions, BlobPresence, BlobSink, EprfsStorage, SinkControl,
};
use crate::tier::{ItemAbsence, TierId};
use crate::tree::{
    absence_error, accept_node, check_root_codec, Budget, TreeError, TreeLimits, TreeNode,
    TreeResult,
};
use crate::walk::{
    descend, expand, find, link_of, Frame, TreePath, View, WalkItem, WalkStats, WalkedNode,
    WalkedView,
};

/// Whether a level walk has more to fetch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LevelOutcome {
    /// Another chunk is wanted: call [`LevelWalk::wanted`] again.
    Next,
    /// The walk is finished: call [`LevelWalk::finish`].
    Done,
}

/// A view-scoped walk advanced one tree level chunk at a time by its caller.
pub struct LevelWalk<'a> {
    root: BlobCid,
    budget: Budget<'a>,
    /// What the gathered result keeps, charged as each node or absence is kept (`"result path
    /// bytes"`), as `walk_collect` does: the bound holds while the walk runs, not after it.
    retained: Budget<'a>,
    /// Frames of the current level not yet taken into a chunk.
    level: Vec<Frame<'a>>,
    /// The frames of the chunk being fetched.
    chunk: Vec<Frame<'a>>,
    wanted: Vec<BatchItem>,
    /// `wanted`'s CIDs, so an answer is matched without scanning the chunk.
    wanted_cids: HashSet<BlobCid>,
    offered: HashMap<BlobCid, (TreeNode, usize)>,
    absent: HashMap<BlobCid, Unanswered<ItemAbsence>>,
    next: Vec<Frame<'a>>,
    cache: HashMap<BlobCid, (TreeNode, usize)>,
    nodes: Vec<WalkedNode>,
    /// Boundaries and absences, keyed by the path of the node that emitted them and arrival order,
    /// so `finish` restores the depth-first walk's order.
    trailing: Vec<(TreePath, usize, WalkItem)>,
    seq: usize,
    stats: WalkStats,
}

impl<'a> LevelWalk<'a> {
    /// Refuses a non-DAG-CBOR root and any view path longer than `max_path_bytes` before any
    /// fetch, as the depth-first walk does.
    pub fn start(root: &BlobCid, view: &'a View, limits: &'a TreeLimits) -> TreeResult<Self> {
        check_root_codec(root)?;
        view.check_path_bytes(limits)?;
        let mut walk = Self {
            root: root.clone(),
            budget: Budget::new(limits),
            retained: Budget::new(limits),
            level: Frame::root(root, view).into_iter().collect(),
            chunk: Vec::new(),
            wanted: Vec::new(),
            wanted_cids: HashSet::new(),
            offered: HashMap::new(),
            absent: HashMap::new(),
            next: Vec::new(),
            cache: HashMap::new(),
            nodes: Vec::new(),
            trailing: Vec::new(),
            seq: 0,
            stats: WalkStats::default(),
        };
        walk.take_chunk();
        Ok(walk)
    }

    /// The root this walk is under: every `fetch_blobs` for it names this root.
    pub fn root(&self) -> &BlobCid {
        &self.root
    }

    /// This level's frontier, deduplicated by CID and without nodes already held from an earlier
    /// level, in chunks of at most `max_pending_entries` frames: one level may take several
    /// `fetch_blobs` calls, and a level wider than `max_pending_entries` directories does not
    /// refuse where the depth-first walk passes (B13).
    pub fn wanted(&self) -> &[BatchItem] {
        &self.wanted
    }

    /// Bytes for a wanted node: verified against its link, size-checked, charged and decoded.
    /// An item the level did not want, or one already answered, is ignored (A13).
    pub fn offer(&mut self, item: &BatchItem, bytes: &Bytes) -> TreeResult<()> {
        if !self.is_open(&item.cid) {
            return Ok(());
        }
        let (node, len) = accept_node(&mut self.budget, &item.cid, bytes)??;
        self.stats.nodes_fetched += 1;
        self.stats.node_bytes += len;
        self.offered.insert(item.cid.clone(), (node, len));
        Ok(())
    }

    /// A wanted node a tier did not give. Integrity, and a storage failure of the own store, are
    /// errors, as in the depth-first walk; only `NotHeld`, `Refused`, `OverBudget`, `Unreachable`
    /// from another tier and `MembershipUnknown` stay absences (B4).
    pub fn absent(&mut self, item: &BatchItem, why: Unanswered<ItemAbsence>) -> TreeResult<()> {
        if !self.is_open(&item.cid) {
            return Ok(());
        }
        if let Some(error) = absence_error(&item.cid, &why.why) {
            return Err(error);
        }
        self.absent.insert(item.cid.clone(), why);
        Ok(())
    }

    /// The driver names the tier that left this chunk's open items unanswered (the sink returned
    /// `Stop`, a batch was cut short, the tier failed as a whole): each becomes
    /// `Unreachable { by }` through [`LevelWalk::absent`]'s rule, so it is `Withheld`, never
    /// `NotHeld` (B3), unless `by` is the own store, whose failure is an error (B4). The walk
    /// cannot know which tier failed, so the driver says; nothing here invents a tier id.
    pub fn unanswered(&mut self, by: &TierId) -> TreeResult<()> {
        let open: Vec<BatchItem> = self
            .wanted
            .iter()
            .filter(|w| self.is_open(&w.cid))
            .cloned()
            .collect();
        for item in open {
            self.absent(&item, ItemAbsence::unreachable(by))?;
        }
        Ok(())
    }

    fn is_open(&self, cid: &BlobCid) -> bool {
        self.wanted_cids.contains(cid)
            && !self.offered.contains_key(cid)
            && !self.absent.contains_key(cid)
    }

    /// Visit every frame of the chunk. Every wanted item must have been offered, named absent,
    /// or named unanswered by its tier ([`LevelWalk::unanswered`]); an item the driver left
    /// unanswered refuses, because the walk cannot name the tier that failed and `own` would say
    /// the own store failed (B3, B4).
    pub fn advance(&mut self) -> TreeResult<LevelOutcome> {
        if let Some(open) = self.wanted.iter().find(|w| self.is_open(&w.cid)) {
            return Err(TreeError::Invalid(format!(
                "the driver left {} unanswered; name its tier with `LevelWalk::unanswered`",
                open.cid
            )));
        }
        let chunk = std::mem::take(&mut self.chunk);
        for frame in chunk {
            self.visit(frame)?;
        }
        self.wanted.clear();
        self.wanted_cids.clear();
        self.offered.clear();
        self.absent.clear();
        if self.level.is_empty() {
            self.level = std::mem::take(&mut self.next);
        }
        if self.level.is_empty() {
            return Ok(LevelOutcome::Done);
        }
        self.take_chunk();
        Ok(LevelOutcome::Next)
    }

    fn visit(&mut self, frame: Frame<'a>) -> TreeResult<()> {
        self.budget.pop_pending(frame.charge);
        self.budget.check_depth(frame.depth)?;
        let node = if let Some((node, len)) = self.offered.remove(&frame.cid) {
            self.cache.insert(frame.cid.clone(), (node.clone(), len));
            node
        } else if let Some((node, len)) = self.cache.get(&frame.cid) {
            self.budget.charge_reused(*len)?;
            self.stats.nodes_reused += 1;
            node.clone()
        } else {
            // `advance` refused any wanted item left open, so an un-offered frame is named.
            let why = self.absent.get(&frame.cid).cloned().ok_or_else(|| {
                TreeError::Invalid(format!("{} was neither offered nor named", frame.cid))
            })?;
            let error = TreeError::Unanswered {
                cid: frame.cid.clone(),
                path: frame.path.clone(),
                why: Box::new(why),
            };
            let key = frame.path.clone();
            for item in frame.absences(error)? {
                self.push_trailing(key.clone(), item)?;
            }
            return Ok(());
        };
        self.stats.entries_visited += node.entries.len();
        let expanded = expand(&mut self.budget, frame, node, None)?;
        let key = expanded.node.path.clone();
        self.retained.charge_result(key.byte_len())?;
        self.nodes.push(expanded.node);
        for item in expanded.trailing {
            self.push_trailing(key.clone(), item)?;
        }
        self.next.extend(expanded.children);
        Ok(())
    }

    fn push_trailing(&mut self, key: TreePath, item: WalkItem) -> TreeResult<()> {
        self.retained.charge_result(item.retained_bytes())?;
        self.trailing.push((key, self.seq, item));
        self.seq += 1;
        Ok(())
    }

    fn take_chunk(&mut self) {
        let size = self.budget.limits().max_pending_entries.max(1);
        let take = size.min(self.level.len());
        self.chunk = self.level.drain(..take).collect();
        let mut seen = HashSet::new();
        self.wanted = self
            .chunk
            .iter()
            .filter(|frame| !self.cache.contains_key(&frame.cid))
            .filter(|frame| seen.insert(frame.cid.clone()))
            .map(Frame::item)
            .collect();
        self.wanted_cids = self.wanted.iter().map(|w| w.cid.clone()).collect();
    }

    /// The gathered walk, nodes re-sorted to pre-order, absences and boundaries in the
    /// depth-first walk's order.
    pub fn finish(mut self) -> WalkedView {
        self.nodes.sort_by(|a, b| a.path.cmp(&b.path));
        self.trailing
            .sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
        let mut boundaries = Vec::new();
        let mut absent = Vec::new();
        for (_, _, item) in self.trailing {
            match item {
                boundary @ WalkItem::Boundary { .. } => boundaries.push(boundary),
                other => absent.push(other),
            }
        }
        WalkedView {
            root: self.root,
            nodes: self.nodes,
            boundaries,
            absent,
            stats: self.stats,
        }
    }
}

/// One `fetch_blobs(root, …)` per tree level chunk, over any tier. It re-sends the items the sink
/// did not see once, as the resumable contract allows, before `advance`. Like `walk_collect`, what
/// each kept node and absence retains is charged against `max_total_bytes` (`"result path
/// bytes"`) as it is kept.
pub async fn walk_batched<S: EprfsStorage + ?Sized>(
    storage: &S,
    root: &BlobCid,
    view: &View,
    opts: &BatchOptions,
    limits: &TreeLimits,
) -> TreeResult<WalkedView> {
    let mut walk = LevelWalk::start(root, view, limits)?;
    loop {
        let mut unseen = walk.wanted().to_vec();
        for _attempt in 0..2 {
            if unseen.is_empty() {
                break;
            }
            // Each answer is offered to the walk on arrival (verified, charged and decoded), so
            // no level is buffered whole. A tier failing as a whole leaves its items unseen; they
            // are re-sent once, then named unanswered by this tier (B3).
            let mut sink = LevelSink {
                walk: &mut walk,
                awaiting: Awaiting::new(unseen.iter().map(|item| &item.cid)),
                error: None,
            };
            let _ = storage.fetch_blobs(root, &unseen, opts, &mut sink).await;
            let LevelSink {
                awaiting, error, ..
            } = sink;
            if let Some(error) = error {
                return Err(error);
            }
            unseen.retain(|item| awaiting.awaits(&item.cid));
        }
        // What this tier never answered is its absence: `Withheld { Unreachable }` from a remote
        // tier, an error from the own store or a chain over it (B3, B4), as in `walk`.
        walk.unanswered(&storage.tier_id())?;
        if walk.advance()? == LevelOutcome::Done {
            break;
        }
    }
    Ok(walk.finish())
}

/// Many paths under one root, resolved together (A3): level by level, the spine nodes the paths
/// share are fetched in one `fetch_blobs` per chunk of at most `max_pending_entries` nodes and
/// decoded once, under one [`Budget`] for the whole call, with the walk's verifier and name
/// matcher. It is the membership check a serving tier runs for a batch, so the serving side makes
/// no tree decision of its own.
///
/// Each path's answer is the link at it (the root's CID for the root path, else its entry's
/// object, a directory's tree included), or the error the single-path resolver behind
/// [`crate::read_path`] gives it: `PathNotFound`, `NotADirectory`, `ExternalBoundary`, an
/// `Unanswered` spine node, `Integrity` or `Invalid` for a spine node that failed its address or
/// its decoding, `Budget("path bytes")` for a path past `max_path_bytes`, and `Storage` when the
/// own store failed (B4). The outer `Err` is the call failing as a whole: a root that is not a
/// DAG-CBOR tree CID, or the shared budget spent.
pub async fn resolve_links<S: EprfsStorage + ?Sized>(
    storage: &S,
    root: &BlobCid,
    paths: &[TreePath],
    opts: &BatchOptions,
    limits: &TreeLimits,
) -> TreeResult<Vec<TreeResult<BlobCid>>> {
    check_root_codec(root)?;
    let mut budget = Budget::new(limits);
    let mut out: Vec<Option<TreeResult<BlobCid>>> = paths.iter().map(|_| None).collect();
    // (path index, the node to read next, the components already consumed)
    let mut pending: Vec<(usize, BlobCid, usize)> = Vec::new();
    for (i, path) in paths.iter().enumerate() {
        if path.byte_len() > limits.max_path_bytes {
            out[i] = Some(Err(TreeError::Budget("path bytes")));
        } else if path.is_root() {
            out[i] = Some(Ok(root.clone()));
        } else {
            pending.push((i, root.clone(), 0));
        }
    }
    let prefix = |path: &TreePath, depth: usize| -> TreeResult<TreePath> {
        TreePath::from_components(path.components()[..depth].to_vec())
    };
    let me = storage.tier_id();
    while !pending.is_empty() {
        let mut wanted: Vec<BatchItem> = Vec::new();
        let mut seen = HashSet::new();
        for (i, cid, depth) in &pending {
            budget.check_depth(*depth)?;
            if seen.insert(cid.clone()) {
                wanted.push(BatchItem::new(cid.clone(), prefix(&paths[*i], *depth)?));
            }
        }
        let mut nodes: HashMap<BlobCid, TreeResult<TreeNode>> = HashMap::new();
        for chunk in wanted.chunks(limits.max_pending_entries.max(1)) {
            // Each spine node is verified, charged and decoded on arrival, so no chunk's bytes
            // are buffered whole. A tier failing as a whole leaves its items unanswered, named
            // below.
            let mut sink = SpineSink {
                nodes: &mut nodes,
                budget: &mut budget,
                awaiting: Awaiting::new(chunk.iter().map(|item| &item.cid)),
                error: None,
            };
            let _ = storage.fetch_blobs(root, chunk, opts, &mut sink).await;
            if let Some(error) = sink.error.take() {
                return Err(error);
            }
            for item in chunk {
                if !nodes.contains_key(&item.cid) {
                    let why = ItemAbsence::unreachable(&me);
                    let error = absence_error(&item.cid, &why.why).unwrap_or_else(|| {
                        TreeError::Unanswered {
                            cid: item.cid.clone(),
                            path: item.path.clone(),
                            why: Box::new(why),
                        }
                    });
                    nodes.insert(item.cid.clone(), Err(error));
                }
            }
        }
        let mut next = Vec::new();
        for (i, cid, depth) in pending {
            let path = &paths[i];
            let node = match &nodes[&cid] {
                Ok(node) => node,
                Err(error) => {
                    out[i] = Some(Err(error.clone()));
                    continue;
                }
            };
            let components = path.components();
            let last = depth + 1 == components.len();
            let Some(entry) = find(node, &components[depth]) else {
                out[i] = Some(Err(TreeError::PathNotFound { path: path.clone() }));
                continue;
            };
            let here = prefix(path, depth + 1)?;
            // The single-path resolver's own steps (`descend`, `link_of`), so each path gets the
            // answer `read_path`'s resolver gives it.
            if last {
                out[i] = Some(link_of(entry, here));
                continue;
            }
            match descend(entry, here) {
                Ok(child) => {
                    budget.check_child_depth(depth)?;
                    next.push((i, child, depth + 1));
                }
                Err(error) => out[i] = Some(Err(error)),
            }
        }
        pending = next;
    }
    Ok(out
        .into_iter()
        .map(|answer| answer.unwrap_or_else(|| Err(TreeError::Invalid("unresolved path".into()))))
        .collect())
}

/// What one requested object's path under a root proves about its membership there, read from
/// [`resolve_links`]'s answer for that path. It is the tree decision a serving tier would
/// otherwise make for itself (spec §1.1: no adapter makes a reach decision of its own), so it lives
/// here, beside the resolver whose errors it reads, and its match over [`TreeError`] is exhaustive
/// inside this crate: a variant added later must be classified here before it compiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Membership {
    /// The path leads to the object.
    Member,
    /// The path is proved, in nodes this tier holds and verified, not to lead to the object:
    /// it leads elsewhere, or is absent, or passes through a leaf or an external boundary. Final
    /// for the requester (`Refused { NotUnderRoot }`).
    NotUnderRoot,
    /// Membership is not established: a spine node is not held or was not answered, the check's
    /// budget was spent, or the root is no tree. Not final; the requester's chain advances.
    Unknown,
    /// A spine node failed its address or its decoding.
    Integrity,
    /// This tier's own store failed while reading the spine.
    OwnFailed,
}

impl Membership {
    /// The verdict for an object `cid` whose path resolved to `link`.
    pub fn of(cid: &BlobCid, link: &TreeResult<BlobCid>) -> Self {
        match link {
            Ok(linked) if linked == cid => Membership::Member,
            Ok(_) => Membership::NotUnderRoot,
            // The resolver's proofs, as its seam-registry row names them.
            Err(
                TreeError::PathNotFound { .. }
                | TreeError::NotADirectory { .. }
                | TreeError::ExternalBoundary { .. },
            ) => Membership::NotUnderRoot,
            Err(TreeError::Integrity(_) | TreeError::Invalid(_)) => Membership::Integrity,
            Err(TreeError::Storage { .. }) => Membership::OwnFailed,
            // `resolve_links` answers a directory with its tree's link and never says `NotALeaf`;
            // a resolver that did would not have proved the object absent.
            Err(
                TreeError::NotALeaf { .. }
                | TreeError::Unanswered { .. }
                | TreeError::Incomplete { .. }
                | TreeError::Budget(_)
                | TreeError::InvalidView(_),
            ) => Membership::Unknown,
        }
    }

    /// What a serving tier `by` answers for an item with this verdict, `None` for a member, which
    /// is served. Exhaustive here, so a verdict added later is named before the crate compiles
    /// and no adapter needs a wildcard: `NotUnderRoot` is final (`Refused { NotUnderRoot }`),
    /// `Unknown` is `MembershipUnknown` (the requester's chain advances), `Integrity` is the
    /// tier's own `Integrity` statement, and `OwnFailed` is `Unreachable`.
    pub fn absence(self, by: &TierId) -> Option<Unanswered<ItemAbsence>> {
        match self {
            Membership::Member => None,
            Membership::NotUnderRoot => Some(ItemAbsence::refused(
                by,
                ReachRefusal::new(ReachReason::NotUnderRoot, None),
            )),
            Membership::Unknown => Some(ItemAbsence::membership_unknown(by)),
            Membership::Integrity => Some(ItemAbsence::integrity(by)),
            Membership::OwnFailed => Some(ItemAbsence::unreachable(by)),
        }
    }

    /// What a presence question about the object may say, `None` for a member, whose presence is
    /// the store's to answer. A path proved not to lead to the object is `Missing`; anything that
    /// leaves membership unproved (unknown, a spine that failed its address, the store failing)
    /// is `Unknown`, so a presence answer never says more about an object than its membership
    /// proved (spec §12.3 rule 2).
    pub fn presence(self) -> Option<BlobPresence> {
        match self {
            Membership::Member => None,
            Membership::NotUnderRoot => Some(BlobPresence::Missing),
            Membership::Unknown | Membership::Integrity | Membership::OwnFailed => {
                Some(BlobPresence::Unknown)
            }
        }
    }
}

/// The membership check a serving tier runs for one batch: [`resolve_links`] over every item's
/// path under one budget, each answer read by [`Membership::of`]. A check that fails as a whole
/// (a root that is no DAG-CBOR tree, the shared budget spent) establishes no item's membership.
pub async fn check_membership<S: EprfsStorage + ?Sized>(
    storage: &S,
    root: &BlobCid,
    items: &[BatchItem],
    opts: &BatchOptions,
    limits: &TreeLimits,
) -> Vec<Membership> {
    let paths: Vec<TreePath> = items.iter().map(|item| item.path.clone()).collect();
    match resolve_links(storage, root, &paths, opts, limits).await {
        Ok(links) => items
            .iter()
            .zip(&links)
            .map(|(item, link)| Membership::of(&item.cid, link))
            .collect(),
        Err(_) => vec![Membership::Unknown; items.len()],
    }
}

/// [`walk_batched`]'s sink: each answer goes to the [`LevelWalk`] as it arrives, once per wanted
/// CID; anything else is dropped before it is kept. A walk error stops the tier.
struct LevelSink<'w, 'a> {
    walk: &'w mut LevelWalk<'a>,
    awaiting: Awaiting,
    error: Option<TreeError>,
}

impl BlobSink for LevelSink<'_, '_> {
    fn arrived(&mut self, item: &BatchItem, bytes: Bytes, _from: &TierId) -> SinkControl {
        if self.error.is_some() {
            return SinkControl::Stop;
        }
        if self.awaiting.claim(&item.cid).is_some() {
            if let Err(error) = self.walk.offer(item, &bytes) {
                self.error = Some(error);
                return SinkControl::Stop;
            }
        }
        self.awaiting.control()
    }

    fn absent(&mut self, item: &BatchItem, why: Unanswered<ItemAbsence>) {
        if self.error.is_some() || self.awaiting.claim(&item.cid).is_none() {
            return;
        }
        if let Err(error) = self.walk.absent(item, why) {
            self.error = Some(error);
        }
    }
}

/// [`resolve_links`]'s sink: one verified, size-checked, charged and decoded node per wanted CID,
/// kept as it arrives; anything else is dropped before it is kept. The shared budget running out
/// stops the tier and fails the call.
struct SpineSink<'n, 'l> {
    nodes: &'n mut HashMap<BlobCid, TreeResult<TreeNode>>,
    budget: &'n mut Budget<'l>,
    awaiting: Awaiting,
    error: Option<TreeError>,
}

impl SpineSink<'_, '_> {
    /// The shared acceptance rule, then the node's entries charged: the outer `Err` is the
    /// shared budget spent, the inner one this node failing for every path through it.
    fn node(&mut self, item: &BatchItem, bytes: &Bytes) -> TreeResult<TreeResult<TreeNode>> {
        let node = accept_node(self.budget, &item.cid, bytes)?;
        if let Ok((node, _)) = &node {
            self.budget.charge_entries(node.entries.len())?;
        }
        Ok(node.map(|(node, _)| node))
    }
}

impl BlobSink for SpineSink<'_, '_> {
    fn arrived(&mut self, item: &BatchItem, bytes: Bytes, _from: &TierId) -> SinkControl {
        if self.error.is_some() {
            return SinkControl::Stop;
        }
        if self.awaiting.claim(&item.cid).is_some() {
            match self.node(item, &bytes) {
                Ok(node) => {
                    self.nodes.insert(item.cid.clone(), node);
                }
                Err(error) => {
                    self.error = Some(error);
                    return SinkControl::Stop;
                }
            }
        }
        self.awaiting.control()
    }

    fn absent(&mut self, item: &BatchItem, why: Unanswered<ItemAbsence>) {
        if self.error.is_some() || self.awaiting.claim(&item.cid).is_none() {
            return;
        }
        let error = absence_error(&item.cid, &why.why).unwrap_or_else(|| TreeError::Unanswered {
            cid: item.cid.clone(),
            path: item.path.clone(),
            why: Box::new(why),
        });
        self.nodes.insert(item.cid.clone(), Err(error));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{dir, external, file, ten_thousand, tp, Batches, CountingStorage};
    use crate::walk::walk_collect;
    use crate::FetchPolicy;

    fn local() -> BatchOptions {
        FetchPolicy::LocalOnly.into()
    }

    /// Equal node sets (with coverage), absences, boundaries, stats, or equal error kinds.
    async fn assert_equivalent(
        store: &CountingStorage,
        root: &BlobCid,
        view: &View,
        limits: &TreeLimits,
    ) {
        let depth_first = walk_collect(store, root, view, &local(), limits).await;
        let batched = walk_batched(store, root, view, &local(), limits).await;
        match (depth_first, batched) {
            (Ok(a), Ok(b)) => {
                let shape = |w: &WalkedView| {
                    w.nodes
                        .iter()
                        .map(|n| {
                            (
                                n.path.clone(),
                                n.cid.clone(),
                                n.coverage,
                                n.entry_coverage.clone(),
                            )
                        })
                        .collect::<Vec<_>>()
                };
                assert_eq!(shape(&a), shape(&b), "nodes differ for {view:?}");
                assert_eq!(a.absent, b.absent, "absences differ for {view:?}");
                assert_eq!(a.boundaries, b.boundaries, "boundaries differ for {view:?}");
                assert_eq!(a.stats, b.stats, "stats differ for {view:?}");
            }
            (Err(a), Err(b)) => assert_eq!(
                std::mem::discriminant(&a),
                std::mem::discriminant(&b),
                "errors differ for {view:?}: {a:?} vs {b:?}"
            ),
            (a, b) => panic!("one walk failed for {view:?}: {a:?} vs {b:?}"),
        }
    }

    fn views() -> Vec<View> {
        vec![
            View::whole(),
            View::from_tree_paths([]),
            View::from_tree_paths([tp("a1/b2/f003"), tp("a4/b4/f099"), tp("a9/b0")]),
            View::from_tree_paths([tp("a1/nope"), tp("a1/b2/zzz"), tp("zz")]),
            View::from_tree_paths([tp("a1/b2/f003/x")]),
            View::from_tree_paths([tp("a3"), tp("a4/b4")]),
            View::from_tree_paths([tp("a0/b0"), tp("a0/b1/f000"), tp("a5")]),
        ]
    }

    #[tokio::test]
    async fn level_equivalence_on_the_slice_one_views() {
        let mut store = CountingStorage::new();
        let (root, nodes) = ten_thousand(&mut store, false);
        for view in views() {
            assert_equivalent(&store, &root, &view, &TreeLimits::default()).await;
        }
        // Missing subtrees are named alike.
        store.remove(&nodes[&tp("a3")]);
        store.remove(&nodes[&tp("a4/b4")]);
        for view in views() {
            assert_equivalent(&store, &root, &view, &TreeLimits::default()).await;
        }
        // A tampered node is an integrity error in both.
        let mut tampered = CountingStorage::new();
        let (root, nodes) = ten_thousand(&mut tampered, false);
        tampered.tamper(&nodes[&tp("a9")], Bytes::from_static(b"\xa0"));
        assert_equivalent(&tampered, &root, &View::whole(), &TreeLimits::default()).await;
        // A budget refusal in both.
        let tight = TreeLimits {
            max_total_entries: 500,
            ..TreeLimits::default()
        };
        let mut store = CountingStorage::new();
        let (root, _) = ten_thousand(&mut store, false);
        assert_equivalent(&store, &root, &View::whole(), &tight).await;
    }

    #[tokio::test]
    async fn level_equivalence_with_boundaries_shared_subtrees_and_leaves() {
        let mut store = CountingStorage::new();
        let shared = store.put_tree(vec![file(b"x", b"shared")]);
        let mid = store.put_tree(vec![
            dir(b"s1", shared.clone()),
            dir(b"s2", shared.clone()),
            external(b"vendor"),
            file(b"leaf", b"leaf"),
        ]);
        let root = store.put_tree(vec![
            dir(b"m", mid.clone()),
            dir(b"n", mid),
            dir(b"o", shared),
        ]);
        for view in [
            View::whole(),
            View::from_tree_paths([tp("m/vendor/deep"), tp("n/s2")]),
            View::from_tree_paths([tp("m/leaf/x"), tp("o"), tp("n/nope")]),
        ] {
            assert_equivalent(&store, &root, &view, &TreeLimits::default()).await;
        }
    }

    #[tokio::test]
    async fn level_batching_costs_one_fetch_blobs_per_level_for_the_headline_view() {
        let mut inner = CountingStorage::new();
        let (root, nodes) = ten_thousand(&mut inner, true);
        let store = Batches::new(inner);
        let view = View::from_tree_paths([tp("a1/b2/f003"), tp("a4/b4/f099"), tp("a9/b0")]);
        let walked = walk_batched(&store, &root, &view, &local(), &TreeLimits::default())
            .await
            .unwrap();
        let calls = store.calls();
        assert_eq!(calls.len(), 3, "one call per level: root, a*, a*/b*");
        let wanted: Vec<BlobCid> = calls.iter().flatten().map(|i| i.cid.clone()).collect();
        assert_eq!(wanted.len(), 7);
        let mut expected: Vec<BlobCid> = ["", "a1", "a1/b2", "a4", "a4/b4", "a9", "a9/b0"]
            .iter()
            .map(|p| nodes[&tp(p)].clone())
            .collect();
        let mut got = wanted.clone();
        expected.sort();
        got.sort();
        assert_eq!(got, expected);
        assert_eq!(walked.nodes.len(), 7);
        // Every batch names its item's path under the root, for membership.
        assert!(calls[1].iter().all(|i| i.path.components().len() == 1));
    }

    #[tokio::test]
    async fn order_invariance_within_a_level() {
        let mut store = CountingStorage::new();
        let (root, _) = ten_thousand(&mut store, false);
        let view = View::from_tree_paths([tp("a1"), tp("a7/b3"), tp("a2/b9/f001")]);
        let limits = TreeLimits::default();
        let forward = walk_batched(&store, &root, &view, &local(), &limits)
            .await
            .unwrap();

        let mut walk = LevelWalk::start(&root, &view, &limits).unwrap();
        loop {
            let mut wanted = walk.wanted().to_vec();
            wanted.reverse();
            for item in wanted {
                let handle = store
                    .fetch_blob(&item.cid, FetchPolicy::LocalOnly)
                    .await
                    .unwrap();
                walk.offer(&item, &handle.bytes).unwrap();
            }
            if walk.advance().unwrap() == LevelOutcome::Done {
                break;
            }
        }
        let reversed = walk.finish();
        let paths = |w: &WalkedView| w.nodes.iter().map(|n| n.path.clone()).collect::<Vec<_>>();
        assert_eq!(paths(&forward), paths(&reversed));
        assert_eq!(forward.stats, reversed.stats);
    }

    #[tokio::test]
    async fn a_level_one_wider_than_max_pending_entries_is_chunked_not_refused() {
        let mut inner = CountingStorage::new();
        let leaf_dir = |store: &mut CountingStorage, n: u8| {
            store.put_tree(vec![file(&[b'f', n + b'0'], &[n])])
        };
        let mut p1 = Vec::new();
        for n in 0..3u8 {
            p1.push(dir(&[b'd', n + b'0'], leaf_dir(&mut inner, n)));
        }
        let mut p2 = Vec::new();
        for n in 3..5u8 {
            p2.push(dir(&[b'd', n + b'0'], leaf_dir(&mut inner, n)));
        }
        let p1 = inner.put_tree(p1);
        let p2 = inner.put_tree(p2);
        let root = inner.put_tree(vec![dir(b"p1", p1), dir(b"p2", p2)]);
        let limits = TreeLimits {
            max_pending_entries: 4,
            ..TreeLimits::default()
        };
        let store = Batches::new(inner);
        let walked = walk_batched(&store, &root, &View::whole(), &local(), &limits)
            .await
            .unwrap();
        assert_eq!(walked.nodes.len(), 8);
        let sizes: Vec<usize> = store.calls().iter().map(Vec::len).collect();
        assert_eq!(
            sizes,
            vec![1, 2, 4, 1],
            "the five-wide level takes two chunks"
        );
        assert_equivalent(&store.inner, &root, &View::whole(), &limits).await;
    }

    #[tokio::test]
    async fn an_item_never_answered_is_withheld_unreachable_never_not_held() {
        let mut store = CountingStorage::new();
        let child = store.put_tree(vec![file(b"x", b"x")]);
        let root = store.put_tree(vec![dir(b"c", child.clone())]);
        let view = View::whole();
        let limits = TreeLimits::default();
        let mut walk = LevelWalk::start(&root, &view, &limits).unwrap();
        let first = walk.wanted().to_vec();
        let bytes = store
            .fetch_blob(&first[0].cid, FetchPolicy::LocalOnly)
            .await
            .unwrap()
            .bytes;
        walk.offer(&first[0], &bytes).unwrap();
        assert_eq!(walk.advance().unwrap(), LevelOutcome::Next);
        // The child level is never offered: the tier failed as a whole. Left unnamed, the walk
        // refuses rather than invent a tier; named, a remote tier's silence is Withheld.
        assert!(matches!(walk.advance(), Err(TreeError::Invalid(_))));
        let hub = TierId::agent("uhCAkhub");
        walk.unanswered(&hub).unwrap();
        assert_eq!(walk.advance().unwrap(), LevelOutcome::Done);
        let walked = walk.finish();
        match walked.absent.as_slice() {
            [WalkItem::Absent {
                reason, unanswered, ..
            }] => {
                assert!(matches!(
                    reason,
                    crate::AbsentReason::Withheld {
                        why: ItemAbsence::Unreachable { tier },
                        ..
                    } if tier == &hub
                ));
                assert!(unanswered.is_some());
            }
            other => panic!("expected one Withheld absence, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn the_own_store_leaving_an_item_unanswered_is_an_error_not_an_absence() {
        let mut store = CountingStorage::new();
        let child = store.put_tree(vec![file(b"x", b"x")]);
        let root = store.put_tree(vec![dir(b"c", child)]);
        let view = View::whole();
        let limits = TreeLimits::default();
        let mut walk = LevelWalk::start(&root, &view, &limits).unwrap();
        assert!(matches!(
            walk.unanswered(&TierId::own()),
            Err(TreeError::Storage { .. })
        ));
    }

    /// Each path `resolve_links` resolves gets exactly the answer the single-path resolver behind
    /// `read_path` and `list_dir` gives it: the same link, or the same error.
    #[tokio::test]
    async fn resolve_links_agrees_with_the_single_path_resolver_on_every_path() {
        use crate::walk::{link_of, resolve_entry, Resolved};
        let mut store = CountingStorage::new();
        let (root, nodes) = ten_thousand(&mut store, false);
        let vendored = store.put_tree(vec![external(b"vendor"), file(b"leaf", b"l")]);
        let root = {
            let mut node = crate::decode_tree(&store.blobs[&root], &TreeLimits::default()).unwrap();
            node.entries.push(dir(b"zz", vendored));
            store.put_tree(node.entries)
        };
        store.remove(&nodes[&tp("a5/b5")]);
        let paths: Vec<TreePath> = [
            "",
            "a1",
            "a1/b2",
            "a1/b2/f003",
            "a1/b2/f003/below",
            "a1/nope",
            "nope",
            "a1/b2/zzz",
            "a5/b5",
            "a5/b5/f001",
            "zz/vendor",
            "zz/vendor/deep",
            "zz/leaf",
            "zz/leaf/x",
            "a9/b9/f099",
        ]
        .iter()
        .map(|p| tp(p))
        .collect();
        let limits = TreeLimits::default();
        let batch = resolve_links(&store, &root, &paths, &local(), &limits)
            .await
            .unwrap();
        for (path, batched) in paths.iter().zip(batch) {
            let mut budget = Budget::new(&limits);
            let single = match resolve_entry(&store, &root, path, &local(), &mut budget).await {
                Ok(Resolved::Root) => Ok(root.clone()),
                Ok(Resolved::Entry(entry)) => link_of(&entry, path.clone()),
                Err(error) => Err(error),
            };
            match (&batched, &single) {
                (Ok(a), Ok(b)) => assert_eq!(a, b, "{path}"),
                (Err(a), Err(b)) => assert_eq!(
                    (std::mem::discriminant(a), a.to_string()),
                    (std::mem::discriminant(b), b.to_string()),
                    "{path}"
                ),
                _ => panic!("{path}: batched {batched:?}, single {single:?}"),
            }
        }
    }

    /// A tier that sends junk and repeats is read through filtering sinks: the level walk and
    /// the membership resolver each keep one answer per wanted CID, drop the rest on arrival, and
    /// stop the tier once the chunk is answered, so neither buffers nor scans what it never asked
    /// for.
    #[tokio::test]
    async fn the_level_walk_and_the_resolver_keep_only_what_they_asked_for() {
        use crate::test_support::Flood;
        let mut source = CountingStorage::new();
        let child = source.put_tree(vec![file(b"x", b"x")]);
        let root = source.put_tree(vec![dir(b"c", child.clone())]);
        let limits = TreeLimits::default();
        let flood = Flood::holding(TierId::agent("uhCAkflood"), source.blobs.clone(), 1, 3);
        let walked = walk_batched(&flood, &root, &View::whole(), &local(), &limits)
            .await
            .unwrap();
        let expected = walk_collect(&source, &root, &View::whole(), &local(), &limits)
            .await
            .unwrap();
        assert_eq!(walked.nodes, expected.nodes);
        assert!(walked.absent.is_empty());
        // Per level: one junk answer, then the node, then Stop before any repeat.
        assert_eq!(flood.calls(), 4);
        assert!(flood.saw_stop());

        let flood = Flood::holding(TierId::agent("uhCAkflood"), source.blobs.clone(), 1, 3);
        let links = resolve_links(&flood, &root, &[tp("c/x"), tp("c")], &local(), &limits)
            .await
            .unwrap();
        assert_eq!(links[0].as_ref().unwrap(), &BlobCid::compute_raw(b"x"));
        assert_eq!(links[1].as_ref().unwrap(), &child);
        assert_eq!(flood.calls(), 4);
    }
}
