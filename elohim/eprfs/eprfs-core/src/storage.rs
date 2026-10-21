use async_trait::async_trait;
use bytes::Bytes;
use seam_contracts::Unanswered;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::address::{BlobCid, EprRef};
use crate::attestation::AttestationDraft;
use crate::error::EprfsError;
use crate::error::Result;
use crate::reach::ReachLevel;
use crate::tier::{ItemAbsence, TierId};
use crate::tree::{TreeLimits, TreeResult};
use crate::walk::{DirListing, ReadLeaf, TreeChange, TreePath, View};

/// Minimal EPR record shape needed by projection code.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EprRecord {
    pub reference: EprRef,
    pub content_type: String,
    pub payload: Value,
}

/// Local/remote availability of a blob.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BlobPresence {
    Local,
    Remote,
    Missing,
    Unknown,
}

/// Fetch behavior requested by a projection/materialization operation.
///
/// `Copy` and `Default` are additive (A12): `LocalOnly` is the safe default, because a read that
/// never leaves the device cannot leak its read pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FetchPolicy {
    #[default]
    LocalOnly,
    FetchIfMissing,
}

/// Existing, unchanged: the bytes a store returns, owned and reference-counted, with the CID they
/// were asked for. A store does not promise that `bytes` verify against `cid`: the chain and the
/// default [`EprfsStorage::fetch_blobs`] check `h.cid == item.cid && item.cid.verifies(&h.bytes)`.
#[derive(Debug, Clone, PartialEq)]
pub struct BlobHandle {
    pub cid: BlobCid,
    pub bytes: Bytes,
}

/// One object named by its place under a root (spec §12.3: a serving tier checks membership from
/// its own nodes, so every request names the path).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchItem {
    pub cid: BlobCid,
    pub path: TreePath,
}

impl BatchItem {
    pub fn new(cid: BlobCid, path: TreePath) -> Self {
        Self { cid, path }
    }
}

/// How a batch is asked.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BatchOptions {
    pub policy: FetchPolicy,
    /// The rest of the requester's tier list. A forwarding tier forwards a miss only to these, in
    /// order (spec §12.3), and reports every tier it asked in `ItemAbsence::NotHeld.tried`.
    pub via: Vec<TierId>,
    /// The root's reach as `Reach::openness()`, when the host knows it (the head plane records it
    /// beside the root). The requester-side chain hands it to its `AskPolicy`, so a private root's
    /// miss never leaves the tiers its reach admits. Never sent on the wire: a serving tier judges
    /// reach from its own records. `None` is unknown, which an `AskPolicy` treats narrowly.
    ///
    /// Revision of interfaces spec §A.1: `EprfsStorage::fetch_blobs` carried no root reach, so the
    /// chain's own reads could not give `AskPolicy::may_ask` its third argument.
    pub root_reach: Option<ReachLevel>,
}

impl From<FetchPolicy> for BatchOptions {
    fn from(policy: FetchPolicy) -> Self {
        Self {
            policy,
            ..Self::default()
        }
    }
}

#[async_trait]
pub trait EprfsStorage: Send + Sync {
    async fn resolve_epr(&self, reference: &EprRef) -> Result<EprRecord>;

    async fn has_blob(&self, cid: &BlobCid) -> Result<BlobPresence>;

    async fn fetch_blob(&self, cid: &BlobCid, policy: FetchPolicy) -> Result<BlobHandle>;

    async fn put_blob(&self, bytes: Bytes) -> Result<BlobCid>;

    /// Store bytes under an explicitly selected raw or DAG-CBOR CID. Adapters
    /// must verify the CID before persisting. This is byte custody only.
    async fn put_blob_verified(&self, _cid: &BlobCid, _bytes: Bytes) -> Result<()> {
        Err(EprfsError::Storage(
            "explicit-CID put is unsupported by this adapter".into(),
        ))
    }

    async fn publish_attestation(&self, draft: AttestationDraft) -> Result<EprRef>;

    /// The identity this store stamps on what it answers (A2). A leaf store is `TierId::own()`,
    /// which is indexical and never serialized; a serving adapter returns its agent key.
    fn tier_id(&self) -> TierId {
        TierId::own()
    }

    /// One object named by its place under a root. A leaf store ignores `root`, `path` and `via`;
    /// a serving or forwarding tier uses them for its reach and membership checks. The walk calls
    /// this for every node and leaf it fetches, so every fetch carries its root (A8).
    async fn fetch_at(
        &self,
        root: &BlobCid,
        item: &BatchItem,
        opts: &BatchOptions,
    ) -> Result<BlobHandle> {
        let _ = root;
        self.fetch_blob(&item.cid, opts.policy).await
    }

    /// Many objects under one root (R6): out of order and resumable (re-send the items the sink
    /// did not see). Each object is verified before the sink sees it; absences are named to the
    /// sink, never raised as errors. `Err` is reserved for the tier failing as a whole.
    ///
    /// Each requested item is answered exactly once; bytes are fetched once per CID and fanned out
    /// to every item carrying that CID (A13).
    async fn fetch_blobs(
        &self,
        root: &BlobCid,
        items: &[BatchItem],
        opts: &BatchOptions,
        sink: &mut (dyn BlobSink + Send),
    ) -> Result<BatchReport> {
        let me = self.tier_id();
        let mut report = BatchReport::default();
        for (cid, group) in group_by_cid(items) {
            match self.fetch_at(root, group[0], opts).await {
                Ok(h) if h.cid == *cid && cid.verifies(&h.bytes) => {
                    for item in group {
                        report.arrived += 1;
                        report.bytes += h.bytes.len() as u64;
                        if sink.arrived(item, h.bytes.clone(), &me) == SinkControl::Stop {
                            report.stopped = true;
                            return Ok(report);
                        }
                    }
                }
                Ok(_) => {
                    for item in group {
                        report.absent += 1;
                        sink.absent(item, ItemAbsence::integrity(&me));
                    }
                }
                Err(e) => {
                    for item in group {
                        report.absent += 1;
                        sink.absent(item, ItemAbsence::from_error(&e, &me));
                    }
                }
            }
        }
        Ok(report)
    }

    /// Depth-0 query (B5), defaulted to the free function. A tier that can answer in one round
    /// trip overrides it.
    async fn query_read_path(
        &self,
        root: &BlobCid,
        path: &TreePath,
        opts: &BatchOptions,
        limits: &TreeLimits,
    ) -> TreeResult<ReadLeaf> {
        crate::walk::read_path(self, root, path, opts, limits).await
    }

    /// Depth-0 query (B5), defaulted to the free function.
    async fn query_list_dir(
        &self,
        root: &BlobCid,
        path: &TreePath,
        opts: &BatchOptions,
        limits: &TreeLimits,
    ) -> TreeResult<DirListing> {
        crate::walk::list_dir(self, root, path, opts, limits).await
    }

    /// Depth-0 query (B5), defaulted to the free function.
    async fn query_diff(
        &self,
        old: &BlobCid,
        new: &BlobCid,
        view: &View,
        opts: &BatchOptions,
        limits: &TreeLimits,
    ) -> TreeResult<Vec<TreeChange>> {
        crate::walk::diff_trees(self, old, new, view, opts, limits).await
    }
}

/// Groups items by CID: groups in the order each CID first occurs, items inside a group in input
/// order. A CID appearing at several paths is one group, so its bytes are fetched once.
pub(crate) fn group_by_cid(items: &[BatchItem]) -> Vec<(&BlobCid, Vec<&BatchItem>)> {
    let mut groups: Vec<(&BlobCid, Vec<&BatchItem>)> = Vec::new();
    let mut index: std::collections::HashMap<&BlobCid, usize> = std::collections::HashMap::new();
    for item in items {
        match index.get(&item.cid) {
            Some(&at) => groups[at].1.push(item),
            None => {
                index.insert(&item.cid, groups.len());
                groups.push((&item.cid, vec![item]));
            }
        }
    }
    groups
}

/// The answers one batch still waits for, as every asker of another tier's `fetch_blobs` keeps
/// them (contract 10, A13). Each requested CID is claimed once, by its position in the request,
/// through a hash index rather than a scan of the request. An answer for a CID nobody asked for,
/// or a second answer for one already claimed, is dropped on arrival, before its bytes are kept
/// or verified. The asker tells the tier to stop once nothing is wanted, or once the tier has
/// sent more unwanted answers than it was asked for, so a misbehaving tier cannot make an asker
/// hold or scan more than the batch it requested.
pub(crate) struct Awaiting {
    index: std::collections::HashMap<BlobCid, usize>,
    unwanted: usize,
    limit: usize,
}

impl Awaiting {
    /// One entry per distinct CID, at the position it first occurs in `cids`.
    pub(crate) fn new<'c>(cids: impl IntoIterator<Item = &'c BlobCid>) -> Self {
        let mut index = std::collections::HashMap::new();
        for (at, cid) in cids.into_iter().enumerate() {
            index.entry(cid.clone()).or_insert(at);
        }
        let limit = index.len();
        Self {
            index,
            unwanted: 0,
            limit,
        }
    }

    /// The requested position of `cid` the first time it is answered; `None`, counted as
    /// unwanted, for an unrequested or already-answered CID.
    pub(crate) fn claim(&mut self, cid: &BlobCid) -> Option<usize> {
        let claimed = self.index.remove(cid);
        if claimed.is_none() {
            self.unwanted += 1;
        }
        claimed
    }

    /// Whether `cid` is still awaited, without claiming it.
    pub(crate) fn awaits(&self, cid: &BlobCid) -> bool {
        self.index.contains_key(cid)
    }

    /// `Stop` once every requested CID is answered or the tier has overrun its batch.
    pub(crate) fn control(&self) -> SinkControl {
        if self.index.is_empty() || self.unwanted > self.limit {
            SinkControl::Stop
        } else {
            SinkControl::Continue
        }
    }

    /// The requested positions never answered, in no particular order.
    pub(crate) fn remaining(&self) -> impl Iterator<Item = usize> + '_ {
        self.index.values().copied()
    }
}

/// Where a batch's answers go, one call per requested item.
pub trait BlobSink {
    /// Called only with bytes that verified against `item.cid`. `Bytes` passes through without a
    /// copy.
    fn arrived(&mut self, item: &BatchItem, bytes: Bytes, from: &TierId) -> SinkControl;
    /// R7: every absence carries its three parts.
    fn absent(&mut self, item: &BatchItem, why: Unanswered<ItemAbsence>);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SinkControl {
    Continue,
    Stop,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BatchReport {
    pub arrived: usize,
    pub absent: usize,
    pub bytes: u64,
    pub stopped: bool,
}

/// One item's answer, as a [`CollectSink`] keeps it.
#[derive(Debug, Clone, PartialEq)]
pub enum BatchAnswer {
    Arrived { bytes: Bytes, from: TierId },
    Absent(Unanswered<ItemAbsence>),
}

/// A sink that keeps every answer in arrival order. The walk and the chain use it to turn a batch
/// back into per-item results; it is public because every tier adapter needs the same thing.
#[derive(Debug, Default)]
pub struct CollectSink {
    pub answers: Vec<(BatchItem, BatchAnswer)>,
}

impl BlobSink for CollectSink {
    fn arrived(&mut self, item: &BatchItem, bytes: Bytes, from: &TierId) -> SinkControl {
        self.answers.push((
            item.clone(),
            BatchAnswer::Arrived {
                bytes,
                from: from.clone(),
            },
        ));
        SinkControl::Continue
    }

    fn absent(&mut self, item: &BatchItem, why: Unanswered<ItemAbsence>) {
        self.answers.push((item.clone(), BatchAnswer::Absent(why)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{tp, CountingStorage};

    #[tokio::test]
    async fn default_fetch_blobs_answers_each_item_once_and_fetches_each_cid_once() {
        let mut store = CountingStorage::new();
        let held = store.put_leaf(b"held");
        let missing = BlobCid::compute_raw(b"missing");
        let root = BlobCid::compute(b"root");
        let items = vec![
            BatchItem::new(held.clone(), tp("a")),
            BatchItem::new(missing.clone(), tp("b")),
            BatchItem::new(held.clone(), tp("c")),
        ];
        let mut sink = CollectSink::default();
        let report = store
            .fetch_blobs(&root, &items, &BatchOptions::default(), &mut sink)
            .await
            .unwrap();
        assert_eq!(store.fetched(), vec![held.clone(), missing.clone()]);
        assert_eq!(report.arrived, 2);
        assert_eq!(report.absent, 1);
        let answered: Vec<_> = sink.answers.iter().map(|(i, _)| i.path.clone()).collect();
        assert_eq!(answered, vec![tp("a"), tp("c"), tp("b")]);
        match &sink.answers[2].1 {
            BatchAnswer::Absent(u) => {
                assert!(matches!(u.why, ItemAbsence::NotHeld { .. }));
            }
            other => panic!("expected an absence, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn default_fetch_blobs_names_bad_bytes_integrity() {
        let mut store = CountingStorage::new();
        let cid = store.put_leaf(b"good");
        store.tamper(&cid, Bytes::from_static(b"evil"));
        let mut sink = CollectSink::default();
        store
            .fetch_blobs(
                &BlobCid::compute(b"root"),
                &[BatchItem::new(cid, tp("a"))],
                &BatchOptions::default(),
                &mut sink,
            )
            .await
            .unwrap();
        assert!(matches!(
            &sink.answers[0].1,
            BatchAnswer::Absent(u) if matches!(u.why, ItemAbsence::Integrity { .. })
        ));
    }

    #[test]
    fn fetch_policy_defaults_to_local_only_and_converts() {
        assert_eq!(FetchPolicy::default(), FetchPolicy::LocalOnly);
        let opts: BatchOptions = FetchPolicy::FetchIfMissing.into();
        assert_eq!(opts.policy, FetchPolicy::FetchIfMissing);
        assert!(opts.via.is_empty());
        assert_eq!(opts.root_reach, None);
    }
}
