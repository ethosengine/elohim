//! The serving side of an eprfs tier (interfaces spec §A.5).
//!
//! A [`TierEndpoint`] answers another party's batch, read, list and diff requests over one store.
//! It judges the requester's reach once per root per batch (R1), checks that every requested
//! object really lies at its path under that root (membership, spec §12.3), forwards a miss
//! through its upstream [`TierChain`] when it has one, and stamps its own identity on every
//! answer, so no answer carries the indexical `own` (A2).
//!
//! It is not itself an `EprfsStorage` (B5): it is the door a transport adapter (the HTTP batch
//! route, a request-response protocol) calls with the requester the transport authenticated.
//!
//! The `ReasonLabel` faces for eprfs-core's reasons live here too ([`labels`]), and, behind the
//! `testing` feature, the tier contract harness and its counting doubles ([`testing`]).

// eprfs-core's `TreeError` is 128 bytes (an inline CID plus a path); see eprfs-core's lib.rs.
#![allow(
    clippy::result_large_err,
    reason = "Serving preserves eprfs-core TreeError's inline CID and path without a second error representation"
)]

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use eprfs_core::{
    check_membership, BatchItem, BatchOptions, BatchReport, BlobCid, BlobPresence, BlobSink,
    DirListing, EprfsError, EprfsStorage, FetchPolicy, ItemAbsence, OnBehalf, ReachGate,
    ReachLevel, ReachReason, ReachRefusal, ReadLeaf, Requester, Result, SinkControl, TierChain,
    TierId, TreeChange, TreeError, TreeLimits, TreePath, TreeResult, View,
};

pub mod labels;
#[cfg(feature = "testing")]
pub mod testing;

pub use labels::{DerivationUnansweredLabel, ItemAbsenceLabel, ReachReasonLabel};

/// What an endpoint has answered. Counters keep apart what the answers deliberately do not: a
/// refused batch and a missing object look the same to a requester outside reach.
#[derive(Debug, Default)]
pub struct ServeCounters {
    batches: AtomicU64,
    refused_batches: AtomicU64,
    items_served: AtomicU64,
    items_absent: AtomicU64,
    not_under_root: AtomicU64,
    membership_unknown: AtomicU64,
    forwarded: AtomicU64,
    oversized_batches: AtomicU64,
}

/// A snapshot of [`ServeCounters`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ServeCountersSnapshot {
    pub batches: u64,
    pub refused_batches: u64,
    pub items_served: u64,
    pub items_absent: u64,
    pub not_under_root: u64,
    pub membership_unknown: u64,
    /// Batches whose misses went upstream.
    pub forwarded: u64,
    /// Batches refused whole for exceeding `max_pending_entries` items.
    pub oversized_batches: u64,
}

impl ServeCounters {
    fn add(counter: &AtomicU64, n: u64) {
        counter.fetch_add(n, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> ServeCountersSnapshot {
        let read = |c: &AtomicU64| c.load(Ordering::Relaxed);
        ServeCountersSnapshot {
            batches: read(&self.batches),
            refused_batches: read(&self.refused_batches),
            items_served: read(&self.items_served),
            items_absent: read(&self.items_absent),
            not_under_root: read(&self.not_under_root),
            membership_unknown: read(&self.membership_unknown),
            forwarded: read(&self.forwarded),
            oversized_batches: read(&self.oversized_batches),
        }
    }
}

/// The serving door of one tier.
pub struct TierEndpoint<G> {
    /// Its agent key; stamps every answer (A2).
    self_id: TierId,
    /// The same store the upstream chain's own store is (A4).
    store: Arc<dyn EprfsStorage>,
    gate: G,
    upstream: Option<TierChain>,
    limits: TreeLimits,
    counters: ServeCounters,
}

/// Compare two trait objects by the data they point at, not their vtables.
fn same_store(a: &Arc<dyn EprfsStorage>, b: &Arc<dyn EprfsStorage>) -> bool {
    std::ptr::eq(Arc::as_ptr(a) as *const (), Arc::as_ptr(b) as *const ())
}

/// Restamps another sink's answers with the endpoint's identity: the requester dialled this tier,
/// so every arrival is from it, and any indexical `own` in an absence is this tier.
///
/// With `verify`, bytes are checked against their address before they leave: the endpoint's own
/// store answers through its own `fetch_blobs`, which an adapter may override without verifying,
/// and a tier never serves bytes that fail their address (contract 6, the outer-tier contract).
/// An upstream chain has already verified every byte it delivers, so its answers are not hashed
/// twice. After the requester's `Stop`, nothing more passes, whatever the store behind does.
struct Restamp<'s> {
    inner: &'s mut (dyn BlobSink + Send),
    self_id: &'s TierId,
    verify: bool,
    report: BatchReport,
}

impl BlobSink for Restamp<'_> {
    fn arrived(&mut self, item: &BatchItem, bytes: bytes::Bytes, _from: &TierId) -> SinkControl {
        // Once the requester said `Stop`, nothing more is served, whether or not the store or
        // chain behind honours it.
        if self.report.stopped {
            return SinkControl::Stop;
        }
        if self.verify && !item.cid.verifies(&bytes) {
            self.report.absent += 1;
            self.inner
                .absent(item, ItemAbsence::integrity(self.self_id));
            return SinkControl::Continue;
        }
        self.report.arrived += 1;
        self.report.bytes += bytes.len() as u64;
        let control = self.inner.arrived(item, bytes, self.self_id);
        if control == SinkControl::Stop {
            self.report.stopped = true;
        }
        control
    }

    fn absent(&mut self, item: &BatchItem, mut why: seam_contracts::Unanswered<ItemAbsence>) {
        if self.report.stopped {
            return;
        }
        self.report.absent += 1;
        why.why.rewrite_own(self.self_id);
        self.inner.absent(item, why);
    }
}

impl<G: ReachGate> TierEndpoint<G> {
    /// Refuses an indexical `self_id` (an endpoint must stamp a real id) and an upstream chain
    /// whose own store is not `store` (the chain would write back somewhere this endpoint does not
    /// serve from).
    pub fn new(
        self_id: TierId,
        store: Arc<dyn EprfsStorage>,
        gate: G,
        upstream: Option<TierChain>,
        limits: TreeLimits,
    ) -> Result<Self> {
        if self_id.is_own() {
            return Err(EprfsError::Storage(
                "a serving endpoint stamps its own id; `own` is indexical".into(),
            ));
        }
        if let Some(chain) = &upstream {
            if !same_store(chain.own(), &store) {
                return Err(EprfsError::Storage(
                    "the upstream chain's own store must be the endpoint's store".into(),
                ));
            }
        }
        Ok(Self {
            self_id,
            store,
            gate,
            upstream,
            limits,
            counters: ServeCounters::default(),
        })
    }

    pub fn self_id(&self) -> &TierId {
        &self.self_id
    }

    pub fn counters(&self) -> ServeCountersSnapshot {
        self.counters.snapshot()
    }

    /// The options this tier serves a request under: the requester's policy and `via`, and the
    /// root's reach from this tier's own records (`ReachGate::root_level`), never from the wire.
    fn serving_opts(&self, root: &BlobCid, opts: &BatchOptions) -> BatchOptions {
        BatchOptions {
            policy: opts.policy,
            via: opts.via.clone(),
            root_reach: self.gate.root_level(root),
        }
    }

    /// 0. a batch past `max_pending_entries` items is this tier not answering it (`Err`, read by
    ///    the requester as `Unreachable`, so its chain advances): a requester's chain asks in
    ///    rounds of `ChainLimits::max_request_items`, whose default is this bound's default, and
    ///    the bound keeps one request from spending the tier;
    /// 1. `gate.admit_root(who, root)`, once per batch (R1), or every item is refused by the same
    ///    name, whether or not the object exists;
    /// 2. membership for the whole batch through eprfs-core's [`check_membership`]: spine nodes
    ///    the paths share are fetched once per level and decoded once (A3), through `upstream` on
    ///    the requester's behalf when this tier does not hold them, and each verdict is
    ///    eprfs-core's [`eprfs_core::Membership::of`], and its answer is eprfs-core's
    ///    [`eprfs_core::Membership::absence`]: this tier makes no mapping of its own;
    /// 3. the own store, else `upstream.forward_blobs(who, root, …, opts)` (restricted to
    ///    `opts.via`), verified and written back by the chain, then served.
    pub async fn serve_batch(
        &self,
        who: &Requester,
        root: &BlobCid,
        items: &[BatchItem],
        opts: &BatchOptions,
        sink: &mut (dyn BlobSink + Send),
    ) -> Result<BatchReport> {
        ServeCounters::add(&self.counters.batches, 1);
        if items.len() > self.limits.max_pending_entries {
            ServeCounters::add(&self.counters.oversized_batches, 1);
            return Err(EprfsError::Storage(format!(
                "a batch of {} items exceeds this tier's {} per request",
                items.len(),
                self.limits.max_pending_entries
            )));
        }
        let mut report = BatchReport::default();
        if self.gate.admit_root(who, root).is_err() {
            ServeCounters::add(&self.counters.refused_batches, 1);
            let why = self.root_refusal();
            for item in items {
                report.absent += 1;
                sink.absent(item, why.clone());
            }
            ServeCounters::add(&self.counters.items_absent, items.len() as u64);
            return Ok(report);
        }
        let opts = self.serving_opts(root, opts);

        let verdicts = {
            let target = self.target(who, &opts);
            check_membership(target.storage(), root, items, &target.opts, &self.limits).await
        };
        let mut members = Vec::new();
        for (item, verdict) in items.iter().zip(verdicts) {
            // eprfs-core names the answer for every verdict; this tier only counts and sends it.
            let Some(absence) = verdict.absence(&self.self_id) else {
                members.push(item.clone());
                continue;
            };
            match &absence.why {
                ItemAbsence::Refused { .. } => ServeCounters::add(&self.counters.not_under_root, 1),
                ItemAbsence::MembershipUnknown { .. } => {
                    ServeCounters::add(&self.counters.membership_unknown, 1)
                }
                _ => {}
            }
            report.absent += 1;
            sink.absent(item, absence);
        }

        let mut restamp = Restamp {
            inner: sink,
            self_id: &self.self_id,
            verify: self.upstream.is_none(),
            report: BatchReport::default(),
        };
        match &self.upstream {
            // The chain tries this same store first, then the requester's remaining tiers.
            Some(chain) => {
                ServeCounters::add(&self.counters.forwarded, 1);
                chain
                    .forward_blobs(who, root, &members, &opts, &mut restamp)
                    .await?;
            }
            None => {
                let local = BatchOptions {
                    policy: FetchPolicy::LocalOnly,
                    ..BatchOptions::default()
                };
                self.store
                    .fetch_blobs(root, &members, &local, &mut restamp)
                    .await?;
            }
        }
        let served = restamp.report;
        ServeCounters::add(&self.counters.items_served, served.arrived as u64);
        ServeCounters::add(
            &self.counters.items_absent,
            (report.absent + served.absent) as u64,
        );
        report.arrived += served.arrived;
        report.absent += served.absent;
        report.bytes += served.bytes;
        report.stopped |= served.stopped;
        Ok(report)
    }

    /// The store this tier reads for a requester: its upstream chain bound to the requester's
    /// budget, `via` and the root's reach (`TierChain::on_behalf`), else the store alone,
    /// local only.
    fn target(&self, who: &Requester, opts: &BatchOptions) -> Target<'_> {
        match &self.upstream {
            Some(chain) => Target {
                chain: Some(chain.on_behalf(who, opts)),
                store: self.store.as_ref(),
                opts: FetchPolicy::FetchIfMissing.into(),
            },
            None => Target {
                chain: None,
                store: self.store.as_ref(),
                opts: FetchPolicy::LocalOnly.into(),
            },
        }
    }

    // Gate diagnostics are local. Neither the reason nor a standing label may reveal whether
    // a root exists, or which private group holds it, to someone who was not admitted.
    fn root_refusal(&self) -> seam_contracts::Unanswered<ItemAbsence> {
        ItemAbsence::refused(
            &self.self_id,
            ReachRefusal::new(ReachReason::OutsideStanding, None),
        )
    }

    fn refuse(&self, root: &BlobCid, path: &TreePath) -> TreeError {
        TreeError::Unanswered {
            cid: root.clone(),
            path: path.clone(),
            why: Box::new(self.root_refusal()),
        }
    }

    fn stamp<T>(&self, answer: TreeResult<T>) -> TreeResult<T> {
        answer.map_err(|error| match error {
            TreeError::Unanswered { cid, path, mut why } => {
                why.why.rewrite_own(&self.self_id);
                TreeError::Unanswered { cid, path, why }
            }
            other => other,
        })
    }

    /// A depth-0 read. A miss goes upstream on the requester's behalf, under its budget and only
    /// to its `opts.via`, exactly as a batch does (A4).
    pub async fn serve_read(
        &self,
        who: &Requester,
        root: &BlobCid,
        path: &TreePath,
        opts: &BatchOptions,
    ) -> TreeResult<ReadLeaf> {
        self.gate
            .admit_root(who, root)
            .map_err(|_| self.refuse(root, path))?;
        let target = self.target(who, &self.serving_opts(root, opts));
        self.stamp(
            target
                .storage()
                .query_read_path(root, path, &target.opts, &self.limits)
                .await,
        )
    }

    /// A depth-0 listing, forwarded as [`TierEndpoint::serve_read`] is.
    pub async fn serve_list(
        &self,
        who: &Requester,
        root: &BlobCid,
        path: &TreePath,
        opts: &BatchOptions,
    ) -> TreeResult<DirListing> {
        self.gate
            .admit_root(who, root)
            .map_err(|_| self.refuse(root, path))?;
        let target = self.target(who, &self.serving_opts(root, opts));
        self.stamp(
            target
                .storage()
                .query_list_dir(root, path, &target.opts, &self.limits)
                .await,
        )
    }

    /// Admits BOTH roots and refuses the whole diff if either fails (B6), so a change's `old`
    /// side never leaks a narrower root's names. Forwarded as [`TierEndpoint::serve_read`] is,
    /// under the narrower of the two roots' reach.
    pub async fn serve_diff(
        &self,
        who: &Requester,
        old: &BlobCid,
        new: &BlobCid,
        view: &View,
        opts: &BatchOptions,
    ) -> TreeResult<Vec<TreeChange>> {
        for root in [old, new] {
            self.gate
                .admit_root(who, root)
                .map_err(|_| self.refuse(root, &TreePath::root()))?;
        }
        let mut serving = self.serving_opts(new, opts);
        serving.root_reach = ReachLevel::narrower(self.gate.root_level(old), serving.root_reach);
        let target = self.target(who, &serving);
        self.stamp(
            target
                .storage()
                .query_diff(old, new, view, &target.opts, &self.limits)
                .await,
        )
    }

    /// `has_blob` for another party, for the object at `path` under `root`: an object reachable
    /// only through a root `who` may not read is `Missing` (spec §12.3 rule 2). Membership is
    /// checked from what this tier holds, never forwarded: a path not proved to lead to `cid` is
    /// `Missing`, and a spine this tier does not hold is `Unknown`, so the answer says nothing
    /// about an object outside the admitted root.
    pub async fn presence(
        &self,
        who: &Requester,
        root: &BlobCid,
        path: &TreePath,
        cid: &BlobCid,
    ) -> Result<BlobPresence> {
        if self.gate.admit_root(who, root).is_err() {
            return Ok(BlobPresence::Missing);
        }
        let local: BatchOptions = FetchPolicy::LocalOnly.into();
        let item = BatchItem::new(cid.clone(), path.clone());
        let verdict = check_membership(
            self.store.as_ref(),
            root,
            std::slice::from_ref(&item),
            &local,
            &self.limits,
        )
        .await;
        match verdict.first().map(|v| v.presence()) {
            Some(None) => self.store.has_blob(cid).await,
            Some(Some(presence)) => Ok(presence),
            None => Ok(BlobPresence::Unknown),
        }
    }
}

/// What a serving tier reads for one requester (`TierEndpoint::target`).
struct Target<'a> {
    chain: Option<OnBehalf<'a>>,
    store: &'a dyn EprfsStorage,
    opts: BatchOptions,
}

impl Target<'_> {
    fn storage(&self) -> &dyn EprfsStorage {
        match &self.chain {
            Some(chain) => chain,
            None => self.store,
        }
    }
}
