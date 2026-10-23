//! Test doubles and tree builders shared by the tree and walk tests.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use bytes::Bytes;
use seam_contracts::Unanswered;

use crate::walk::TreePath;
use crate::{
    encode_tree, AskPolicy, AttestationDraft, BatchItem, BatchOptions, BatchReport, BlobCid,
    BlobHandle, BlobPresence, BlobSink, EprRecord, EprRef, EprfsError, EprfsStorage, FetchPolicy,
    ForwardGate, ForwardPermit, ItemAbsence, OverBudget, ReachLevel, Requester, TierConnector,
    TierId, TierRecord, TreeEntry, TreeEntryKind, TreeLimits, TreeNode,
};

/// In-memory storage that logs every `fetch_blob` CID, so a test can assert
/// exactly which objects a traversal asked for. Verified writes land in a
/// separate map, so a chain's write-back is visible without `&mut`.
#[derive(Default)]
pub(crate) struct CountingStorage {
    pub blobs: HashMap<BlobCid, Bytes>,
    written: Mutex<HashMap<BlobCid, Bytes>>,
    fetched: Mutex<Vec<BlobCid>>,
    id: Option<TierId>,
    /// A CID whose read fails as the disk would (`EprfsError::Storage`), not as a miss.
    pub fail: Option<BlobCid>,
}

impl CountingStorage {
    pub fn new() -> Self {
        Self::default()
    }

    /// The same store answering as a tier with `id`.
    pub fn with_id(mut self, id: TierId) -> Self {
        self.id = Some(id);
        self
    }

    #[allow(
        dead_code,
        reason = "Shared counting fixture is compiled by test modules that do not all inspect recorded writes"
    )]
    pub fn written(&self) -> Vec<BlobCid> {
        let mut cids: Vec<BlobCid> = self.written.lock().unwrap().keys().cloned().collect();
        cids.sort();
        cids
    }

    pub fn put_leaf(&mut self, bytes: &[u8]) -> BlobCid {
        let cid = BlobCid::compute_raw(bytes);
        self.blobs
            .insert(cid.clone(), Bytes::copy_from_slice(bytes));
        cid
    }

    pub fn put_tree(&mut self, entries: Vec<TreeEntry>) -> BlobCid {
        let node = TreeNode::new(entries).unwrap();
        let (cid, bytes) = encode_tree(&node, &TreeLimits::default()).unwrap();
        self.blobs.insert(cid.clone(), Bytes::from(bytes));
        cid
    }

    pub fn remove(&mut self, cid: &BlobCid) {
        self.blobs.remove(cid);
    }

    pub fn tamper(&mut self, cid: &BlobCid, bytes: Bytes) {
        self.blobs.insert(cid.clone(), bytes);
    }

    pub fn fetched(&self) -> Vec<BlobCid> {
        self.fetched.lock().unwrap().clone()
    }

    pub fn reset(&self) {
        self.fetched.lock().unwrap().clear();
    }
}

#[async_trait::async_trait]
impl EprfsStorage for CountingStorage {
    async fn resolve_epr(&self, _: &EprRef) -> crate::Result<EprRecord> {
        unreachable!()
    }

    async fn has_blob(&self, cid: &BlobCid) -> crate::Result<BlobPresence> {
        Ok(
            if self.blobs.contains_key(cid) || self.written.lock().unwrap().contains_key(cid) {
                BlobPresence::Local
            } else {
                BlobPresence::Missing
            },
        )
    }

    async fn fetch_blob(&self, cid: &BlobCid, _: FetchPolicy) -> crate::Result<BlobHandle> {
        self.fetched.lock().unwrap().push(cid.clone());
        if self.fail.as_ref() == Some(cid) {
            return Err(EprfsError::Storage(format!("disk read failed for {cid}")));
        }
        self.blobs
            .get(cid)
            .cloned()
            .or_else(|| self.written.lock().unwrap().get(cid).cloned())
            .map(|bytes| BlobHandle {
                cid: cid.clone(),
                bytes,
            })
            .ok_or_else(|| EprfsError::BlobNotFound(cid.clone()))
    }

    async fn put_blob(&self, _: Bytes) -> crate::Result<BlobCid> {
        unreachable!()
    }

    async fn put_blob_verified(&self, cid: &BlobCid, bytes: Bytes) -> crate::Result<()> {
        if !cid.verifies(&bytes) {
            return Err(EprfsError::Storage(format!("integrity failed for {cid}")));
        }
        self.written.lock().unwrap().insert(cid.clone(), bytes);
        Ok(())
    }

    fn tier_id(&self) -> TierId {
        self.id.clone().unwrap_or_else(TierId::own)
    }

    async fn publish_attestation(&self, _: AttestationDraft) -> crate::Result<EprRef> {
        unreachable!()
    }
}

pub(crate) fn file(name: &[u8], content: &[u8]) -> TreeEntry {
    TreeEntry {
        name: name.to_vec(),
        kind: TreeEntryKind::File {
            blob: BlobCid::compute_raw(content).into(),
            executable: false,
        },
    }
}

pub(crate) fn dir(name: &[u8], tree: BlobCid) -> TreeEntry {
    TreeEntry {
        name: name.to_vec(),
        kind: TreeEntryKind::Directory { tree: tree.into() },
    }
}

pub(crate) fn external(name: &[u8]) -> TreeEntry {
    TreeEntry {
        name: name.to_vec(),
        kind: TreeEntryKind::External {
            kind: "opaque".into(),
            reference: vec![0, 0xff],
        },
    }
}

pub(crate) fn tp(path: &str) -> TreePath {
    TreePath::from_slash_bytes(path.as_bytes()).unwrap()
}

/// The content of the file at `a{i}/b{j}/f{k:03}`; unique per path, so no two
/// directories share a CID.
pub(crate) fn ten_thousand_content(i: usize, j: usize, k: usize) -> Vec<u8> {
    format!("a{i}/b{j}/f{k:03}").into_bytes()
}

/// root -> a0..a9 -> b0..b9 -> f000..f099: 10,000 file entries (10,110 entries
/// in all) and 111 tree nodes. Returns the root and every tree node's CID by path.
pub(crate) fn ten_thousand(
    store: &mut CountingStorage,
    with_leaves: bool,
) -> (BlobCid, HashMap<TreePath, BlobCid>) {
    let mut nodes = HashMap::new();
    let mut a_entries = Vec::new();
    for i in 0..10 {
        let mut b_entries = Vec::new();
        for j in 0..10 {
            let mut files = Vec::new();
            for k in 0..100 {
                let content = ten_thousand_content(i, j, k);
                if with_leaves {
                    store.put_leaf(&content);
                }
                files.push(file(format!("f{k:03}").as_bytes(), &content));
            }
            let b = store.put_tree(files);
            nodes.insert(tp(&format!("a{i}/b{j}")), b.clone());
            b_entries.push(dir(format!("b{j}").as_bytes(), b));
        }
        let a = store.put_tree(b_entries);
        nodes.insert(tp(&format!("a{i}")), a.clone());
        a_entries.push(dir(format!("a{i}").as_bytes(), a));
    }
    let root = store.put_tree(a_entries);
    nodes.insert(TreePath::root(), root.clone());
    (root, nodes)
}

/// A store wrapper that records every `fetch_blobs` call's items, so a test
/// can count round trips.
pub(crate) struct Batches<S> {
    pub inner: S,
    calls: Mutex<Vec<Vec<BatchItem>>>,
}

impl<S> Batches<S> {
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            calls: Mutex::new(Vec::new()),
        }
    }

    pub fn calls(&self) -> Vec<Vec<BatchItem>> {
        self.calls.lock().unwrap().clone()
    }
}

#[async_trait::async_trait]
impl<S: EprfsStorage> EprfsStorage for Batches<S> {
    async fn resolve_epr(&self, r: &EprRef) -> crate::Result<EprRecord> {
        self.inner.resolve_epr(r).await
    }
    async fn has_blob(&self, cid: &BlobCid) -> crate::Result<BlobPresence> {
        self.inner.has_blob(cid).await
    }
    async fn fetch_blob(&self, cid: &BlobCid, policy: FetchPolicy) -> crate::Result<BlobHandle> {
        self.inner.fetch_blob(cid, policy).await
    }
    async fn put_blob(&self, bytes: Bytes) -> crate::Result<BlobCid> {
        self.inner.put_blob(bytes).await
    }
    async fn put_blob_verified(&self, cid: &BlobCid, bytes: Bytes) -> crate::Result<()> {
        self.inner.put_blob_verified(cid, bytes).await
    }
    async fn publish_attestation(&self, d: AttestationDraft) -> crate::Result<EprRef> {
        self.inner.publish_attestation(d).await
    }
    fn tier_id(&self) -> TierId {
        self.inner.tier_id()
    }
    async fn fetch_at(
        &self,
        root: &BlobCid,
        item: &BatchItem,
        opts: &BatchOptions,
    ) -> crate::Result<BlobHandle> {
        self.inner.fetch_at(root, item, opts).await
    }
    async fn fetch_blobs(
        &self,
        root: &BlobCid,
        items: &[BatchItem],
        opts: &BatchOptions,
        sink: &mut (dyn BlobSink + Send),
    ) -> crate::Result<BatchReport> {
        self.calls.lock().unwrap().push(items.to_vec());
        self.inner.fetch_blobs(root, items, opts, sink).await
    }
}

/// A tier that sends whatever bytes it holds without checking them against their address, as a
/// tampering or broken remote would: unlike the default `fetch_blobs`, which turns a held copy
/// that fails its address into an `Integrity` statement, it puts the bad bytes on the wire, so
/// the requester's own verification is what catches them.
pub(crate) struct Unverified(pub CountingStorage);

#[async_trait::async_trait]
impl EprfsStorage for Unverified {
    async fn resolve_epr(&self, _: &EprRef) -> crate::Result<EprRecord> {
        unreachable!()
    }

    async fn has_blob(&self, cid: &BlobCid) -> crate::Result<BlobPresence> {
        self.0.has_blob(cid).await
    }

    async fn fetch_blob(&self, cid: &BlobCid, policy: FetchPolicy) -> crate::Result<BlobHandle> {
        self.0.fetch_blob(cid, policy).await
    }

    async fn put_blob(&self, _: Bytes) -> crate::Result<BlobCid> {
        unreachable!()
    }

    async fn put_blob_verified(&self, cid: &BlobCid, bytes: Bytes) -> crate::Result<()> {
        self.0.put_blob_verified(cid, bytes).await
    }

    fn tier_id(&self) -> TierId {
        self.0.tier_id()
    }

    async fn publish_attestation(&self, _: AttestationDraft) -> crate::Result<EprRef> {
        unreachable!()
    }

    async fn fetch_blobs(
        &self,
        _root: &BlobCid,
        items: &[BatchItem],
        _opts: &BatchOptions,
        sink: &mut (dyn BlobSink + Send),
    ) -> crate::Result<BatchReport> {
        let me = self.tier_id();
        let mut report = BatchReport::default();
        for item in items {
            match self.0.fetch_blob(&item.cid, FetchPolicy::LocalOnly).await {
                Ok(handle) => {
                    report.arrived += 1;
                    if sink.arrived(item, handle.bytes, &me) == crate::SinkControl::Stop {
                        report.stopped = true;
                        break;
                    }
                }
                Err(error) => {
                    report.absent += 1;
                    sink.absent(item, ItemAbsence::from_error(&error, &me));
                }
            }
        }
        Ok(report)
    }
}

/// A tier that answers every item absent with one fixed answer.
pub(crate) struct Answering {
    pub id: TierId,
    pub why: Unanswered<ItemAbsence>,
}

#[async_trait::async_trait]
impl EprfsStorage for Answering {
    async fn resolve_epr(&self, _: &EprRef) -> crate::Result<EprRecord> {
        unreachable!()
    }
    async fn has_blob(&self, _: &BlobCid) -> crate::Result<BlobPresence> {
        Ok(BlobPresence::Unknown)
    }
    async fn fetch_blob(&self, cid: &BlobCid, _: FetchPolicy) -> crate::Result<BlobHandle> {
        Err(EprfsError::Unanswered {
            cid: cid.clone(),
            why: Box::new(self.why.clone()),
        })
    }
    async fn put_blob(&self, _: Bytes) -> crate::Result<BlobCid> {
        unreachable!()
    }
    async fn publish_attestation(&self, _: AttestationDraft) -> crate::Result<EprRef> {
        unreachable!()
    }
    fn tier_id(&self) -> TierId {
        self.id.clone()
    }
}

/// A misbehaving remote tier: it serves what it holds, but before each item it sends `junk`
/// answers for CIDs nobody asked for, and it repeats each real answer `repeats` extra times. It
/// counts every sink call it makes and returns on the first `Stop`, as a streaming tier does.
pub(crate) struct Flood {
    pub id: TierId,
    pub blobs: HashMap<BlobCid, Bytes>,
    pub junk: usize,
    pub repeats: usize,
    calls: Mutex<usize>,
    stopped: Mutex<bool>,
}

impl Flood {
    /// Holding raw leaves; returns their CIDs in order.
    pub fn new(id: TierId, held: &[&[u8]], junk: usize, repeats: usize) -> (Self, Vec<BlobCid>) {
        let mut blobs = HashMap::new();
        let mut cids = Vec::new();
        for bytes in held {
            let cid = BlobCid::compute_raw(bytes);
            blobs.insert(cid.clone(), Bytes::copy_from_slice(bytes));
            cids.push(cid);
        }
        (Self::holding(id, blobs, junk, repeats), cids)
    }

    /// Holding exactly `blobs` (a tree's nodes, say).
    pub fn holding(
        id: TierId,
        blobs: HashMap<BlobCid, Bytes>,
        junk: usize,
        repeats: usize,
    ) -> Self {
        Self {
            id,
            blobs,
            junk,
            repeats,
            calls: Mutex::new(0),
            stopped: Mutex::new(false),
        }
    }

    /// Sink calls made, over every batch.
    pub fn calls(&self) -> usize {
        *self.calls.lock().unwrap()
    }

    pub fn saw_stop(&self) -> bool {
        *self.stopped.lock().unwrap()
    }

    fn sent(&self, control: crate::SinkControl) -> bool {
        *self.calls.lock().unwrap() += 1;
        let stop = control == crate::SinkControl::Stop;
        if stop {
            *self.stopped.lock().unwrap() = true;
        }
        stop
    }
}

#[async_trait::async_trait]
impl EprfsStorage for Flood {
    async fn resolve_epr(&self, _: &EprRef) -> crate::Result<EprRecord> {
        unreachable!()
    }
    async fn has_blob(&self, _: &BlobCid) -> crate::Result<BlobPresence> {
        Ok(BlobPresence::Unknown)
    }
    async fn fetch_blob(&self, cid: &BlobCid, _: FetchPolicy) -> crate::Result<BlobHandle> {
        Err(EprfsError::RootRequired(cid.clone()))
    }
    async fn put_blob(&self, _: Bytes) -> crate::Result<BlobCid> {
        unreachable!()
    }
    async fn publish_attestation(&self, _: AttestationDraft) -> crate::Result<EprRef> {
        unreachable!()
    }
    fn tier_id(&self) -> TierId {
        self.id.clone()
    }
    async fn fetch_blobs(
        &self,
        _root: &BlobCid,
        items: &[BatchItem],
        _opts: &BatchOptions,
        sink: &mut (dyn BlobSink + Send),
    ) -> crate::Result<BatchReport> {
        let mut junk_no = 0usize;
        for item in items {
            for _ in 0..self.junk {
                junk_no += 1;
                let bytes = Bytes::from(format!("junk-{junk_no}"));
                let unrequested = BatchItem::new(BlobCid::compute_raw(&bytes), item.path.clone());
                if self.sent(sink.arrived(&unrequested, bytes, &self.id)) {
                    return Ok(BatchReport::default());
                }
            }
            match self.blobs.get(&item.cid) {
                Some(bytes) => {
                    for _ in 0..=self.repeats {
                        if self.sent(sink.arrived(item, bytes.clone(), &self.id)) {
                            return Ok(BatchReport::default());
                        }
                    }
                }
                None => {
                    sink.absent(item, ItemAbsence::not_held(Vec::new()));
                    self.sent(crate::SinkControl::Continue);
                }
            }
        }
        Ok(BatchReport::default())
    }
}

/// A caller's sink that keeps answers and says `Stop` after `after` arrivals.
pub(crate) struct StopAfter {
    pub after: usize,
    pub inner: crate::CollectSink,
}

impl BlobSink for StopAfter {
    fn arrived(&mut self, item: &BatchItem, bytes: Bytes, from: &TierId) -> crate::SinkControl {
        self.inner.arrived(item, bytes, from);
        if self.inner.answers.len() >= self.after {
            crate::SinkControl::Stop
        } else {
            crate::SinkControl::Continue
        }
    }
    fn absent(&mut self, item: &BatchItem, why: Unanswered<ItemAbsence>) {
        self.inner.absent(item, why);
    }
}

pub(crate) struct AllowAll;

impl AskPolicy for AllowAll {
    fn may_ask(&self, _: &TierRecord, _: &BlobCid, _: Option<ReachLevel>) -> bool {
        true
    }
}

pub(crate) struct OpenGate;

impl ForwardGate for OpenGate {
    fn admit(
        &self,
        _: &Requester,
        _: &TierId,
        _: Option<u64>,
    ) -> std::result::Result<ForwardPermit, OverBudget> {
        Ok(ForwardPermit::new(()))
    }
}

/// Connects records to the stores registered under their ids.
#[derive(Default)]
pub(crate) struct Connect(pub HashMap<TierId, Arc<dyn EprfsStorage>>);

impl TierConnector for Connect {
    fn connect(&self, record: &TierRecord) -> Option<Arc<dyn EprfsStorage>> {
        self.0.get(&record.id).cloned()
    }
}

pub(crate) fn household_record(id: &TierId) -> TierRecord {
    TierRecord {
        id: id.clone(),
        class: crate::TierClass::Household,
        sources: vec![crate::TierSource::HeldRecord {
            record: "roster".into(),
        }],
        endpoint: None,
        standing: None,
        proximity: None,
    }
}
