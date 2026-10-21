//! The tier contract harness and its doubles (interfaces spec §A.4; feature `testing`).
//!
//! [`assert_tier_contract`] runs seam A's chain contract (items 1–10 and 12) against any
//! requester-side composition a factory builds from a [`TierContractSetup`]: eprfs-core's
//! `TierChain`, or a host's own composition. [`assert_outer_tier_contract`] runs an outer tier's
//! own obligations against the tier itself (an HTTP, blob-store or peer-transport tier, or a
//! loopback over a `TierEndpoint`). [`assert_leaf_contract`] runs item 11 and the default batch
//! behaviour against a leaf store. Each panics with the contract item's number on a violation.
//! elohim-storage runs the same harness as a dev-dependency (rail R-E3).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use bytes::Bytes;
use eprfs_core::{
    AskPolicy, AttestationDraft, BatchAnswer, BatchItem, BatchOptions, BatchReport, BlobCid,
    BlobHandle, BlobPresence, BlobSink, CollectSink, EprRecord, EprRef, EprfsError, EprfsStorage,
    FetchPolicy, ForwardGate, ForwardPermit, ItemAbsence, KeyCustody, OverBudget, ProximityOrder,
    ReachGate, ReachLevel, ReachReason, ReachRefusal, Requester, Result, SinkControl, TierClass,
    TierConnector, TierId, TierOrder, TierRecord, TierSource, TreePath,
};
use seam_contracts::{ComeBack, Meanwhile, Unanswered};

/// How a [`CountingTier`] answers.
#[derive(Debug, Clone)]
pub enum Behaviour {
    /// Serve what it holds; everything else is `NotHeld { tried: [] }`.
    Serve,
    /// Answer every item absent with this answer.
    AnswerAll(Unanswered<ItemAbsence>),
    /// Answer every item with bytes that do not match its address.
    Corrupt,
    /// Fail every batch as a whole.
    Fail,
    /// Serve what it holds and answer every miss `NotHeld` naming these tiers as tried too, as a
    /// forwarding tier reports the tiers it asked.
    NotHeldNaming(Vec<TierId>),
    /// Serve what it holds, and also answer a CID nobody asked for.
    ServeWithExtra(BlobCid, Bytes),
}

/// What a [`CountingTier`] was asked.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TierCalls {
    /// One entry per `fetch_blobs` call: its items and its options' `via`.
    pub batches: Vec<(Vec<BatchItem>, Vec<TierId>)>,
    pub fetch_blob: usize,
    pub has_blob: usize,
    pub puts: usize,
}

impl TierCalls {
    /// Every batch item's CID, in order.
    pub fn asked_cids(&self) -> Vec<BlobCid> {
        self.batches
            .iter()
            .flat_map(|(items, _)| items.iter().map(|i| i.cid.clone()))
            .collect()
    }
}

/// An in-memory tier that counts every call and answers by its [`Behaviour`]. It neither verifies
/// nor judges reach: the chain and the endpoint do. It honours a sink's `Stop`.
pub struct CountingTier {
    id: TierId,
    blobs: Mutex<HashMap<BlobCid, Bytes>>,
    behaviour: Mutex<Behaviour>,
    refuse_puts: Mutex<bool>,
    calls: Mutex<TierCalls>,
}

impl CountingTier {
    pub fn new(id: TierId) -> Self {
        Self {
            id,
            blobs: Mutex::new(HashMap::new()),
            behaviour: Mutex::new(Behaviour::Serve),
            refuse_puts: Mutex::new(false),
            calls: Mutex::new(TierCalls::default()),
        }
    }

    /// Hold raw bytes; returns their raw CID.
    pub fn insert(&self, bytes: &[u8]) -> BlobCid {
        let cid = BlobCid::compute_raw(bytes);
        self.insert_at(cid.clone(), Bytes::copy_from_slice(bytes));
        cid
    }

    /// Hold bytes under a CID the caller chose (a tree node's DAG-CBOR CID, or a wrong one).
    pub fn insert_at(&self, cid: BlobCid, bytes: Bytes) {
        self.blobs.lock().unwrap().insert(cid, bytes);
    }

    pub fn holds(&self, cid: &BlobCid) -> bool {
        self.blobs.lock().unwrap().contains_key(cid)
    }

    pub fn behave(&self, behaviour: Behaviour) {
        *self.behaviour.lock().unwrap() = behaviour;
    }

    pub fn refuse_puts(&self, refuse: bool) {
        *self.refuse_puts.lock().unwrap() = refuse;
    }

    pub fn calls(&self) -> TierCalls {
        self.calls.lock().unwrap().clone()
    }

    pub fn reset(&self) {
        *self.calls.lock().unwrap() = TierCalls::default();
    }

    fn get(&self, cid: &BlobCid) -> Option<Bytes> {
        self.blobs.lock().unwrap().get(cid).cloned()
    }
}

#[async_trait]
impl EprfsStorage for CountingTier {
    async fn resolve_epr(&self, reference: &EprRef) -> Result<EprRecord> {
        Err(EprfsError::EprNotFound(reference.clone()))
    }

    async fn has_blob(&self, cid: &BlobCid) -> Result<BlobPresence> {
        self.calls.lock().unwrap().has_blob += 1;
        Ok(if self.holds(cid) {
            BlobPresence::Local
        } else {
            BlobPresence::Missing
        })
    }

    async fn fetch_blob(&self, cid: &BlobCid, _policy: FetchPolicy) -> Result<BlobHandle> {
        self.calls.lock().unwrap().fetch_blob += 1;
        self.get(cid)
            .map(|bytes| BlobHandle {
                cid: cid.clone(),
                bytes,
            })
            .ok_or_else(|| EprfsError::BlobNotFound(cid.clone()))
    }

    async fn put_blob(&self, bytes: Bytes) -> Result<BlobCid> {
        let cid = BlobCid::compute(&bytes);
        self.put_blob_verified(&cid, bytes).await?;
        Ok(cid)
    }

    async fn put_blob_verified(&self, cid: &BlobCid, bytes: Bytes) -> Result<()> {
        self.calls.lock().unwrap().puts += 1;
        if *self.refuse_puts.lock().unwrap() {
            return Err(EprfsError::Storage("this tier refuses writes".into()));
        }
        if !cid.verifies(&bytes) {
            return Err(EprfsError::Storage(format!(
                "blob integrity verification failed for {cid}"
            )));
        }
        self.insert_at(cid.clone(), bytes);
        Ok(())
    }

    async fn publish_attestation(&self, _draft: AttestationDraft) -> Result<EprRef> {
        Err(EprfsError::Storage(
            "a counting tier publishes nothing".into(),
        ))
    }

    fn tier_id(&self) -> TierId {
        self.id.clone()
    }

    async fn fetch_at(
        &self,
        root: &BlobCid,
        item: &BatchItem,
        opts: &BatchOptions,
    ) -> Result<BlobHandle> {
        let mut collected = CollectSink::default();
        self.fetch_blobs(root, std::slice::from_ref(item), opts, &mut collected)
            .await?;
        match collected.answers.into_iter().next() {
            Some((_, BatchAnswer::Arrived { bytes, .. })) => Ok(BlobHandle {
                cid: item.cid.clone(),
                bytes,
            }),
            Some((_, BatchAnswer::Absent(why))) => Err(EprfsError::Unanswered {
                cid: item.cid.clone(),
                why: Box::new(why),
            }),
            None => Err(EprfsError::BlobNotFound(item.cid.clone())),
        }
    }

    async fn fetch_blobs(
        &self,
        _root: &BlobCid,
        items: &[BatchItem],
        opts: &BatchOptions,
        sink: &mut (dyn BlobSink + Send),
    ) -> Result<BatchReport> {
        self.calls
            .lock()
            .unwrap()
            .batches
            .push((items.to_vec(), opts.via.clone()));
        let behaviour = self.behaviour.lock().unwrap().clone();
        let mut report = BatchReport::default();
        if let Behaviour::Fail = behaviour {
            return Err(EprfsError::Storage("this tier failed as a whole".into()));
        }
        if let Behaviour::ServeWithExtra(cid, bytes) = &behaviour {
            if sink.arrived(
                &BatchItem::new(cid.clone(), TreePath::root()),
                bytes.clone(),
                &self.id,
            ) == SinkControl::Stop
            {
                report.stopped = true;
                return Ok(report);
            }
        }
        for item in items {
            match &behaviour {
                Behaviour::AnswerAll(why) => {
                    report.absent += 1;
                    sink.absent(item, why.clone());
                }
                Behaviour::Corrupt => {
                    report.arrived += 1;
                    if sink.arrived(item, Bytes::from_static(b"not the bytes"), &self.id)
                        == SinkControl::Stop
                    {
                        report.stopped = true;
                        return Ok(report);
                    }
                }
                _ => match self.get(&item.cid) {
                    Some(bytes) => {
                        report.arrived += 1;
                        if sink.arrived(item, bytes, &self.id) == SinkControl::Stop {
                            report.stopped = true;
                            return Ok(report);
                        }
                    }
                    None => {
                        report.absent += 1;
                        let tried = match &behaviour {
                            Behaviour::NotHeldNaming(tried) => tried.clone(),
                            _ => Vec::new(),
                        };
                        sink.absent(item, ItemAbsence::not_held(tried));
                    }
                },
            }
        }
        Ok(report)
    }
}

/// A gate that admits everything.
#[derive(Debug, Clone, Copy, Default)]
pub struct OpenGate;

impl ForwardGate for OpenGate {
    fn admit(
        &self,
        _on_behalf: &Requester,
        _to: &TierId,
        _bytes_hint: Option<u64>,
    ) -> std::result::Result<ForwardPermit, OverBudget> {
        Ok(ForwardPermit::new(()))
    }
}

/// A gate whose budget is spent.
#[derive(Debug, Clone)]
pub struct ClosedGate(pub OverBudget);

impl ForwardGate for ClosedGate {
    fn admit(
        &self,
        _on_behalf: &Requester,
        _to: &TierId,
        _bytes_hint: Option<u64>,
    ) -> std::result::Result<ForwardPermit, OverBudget> {
        Err(self.0.clone())
    }
}

/// Asks every tier.
#[derive(Debug, Clone, Copy, Default)]
pub struct AllowAll;

impl AskPolicy for AllowAll {
    fn may_ask(&self, _: &TierRecord, _: &BlobCid, _: Option<ReachLevel>) -> bool {
        true
    }
}

/// Asks no tier in `denied`.
#[derive(Debug, Clone, Default)]
pub struct DenyTiers(pub Vec<TierId>);

impl AskPolicy for DenyTiers {
    fn may_ask(&self, tier: &TierRecord, _: &BlobCid, _: Option<ReachLevel>) -> bool {
        !self.0.contains(&tier.id)
    }
}

/// The host-side `ReachStandingAsk` rule (interfaces spec §A.6) is eprfs-core's one
/// [`eprfs_core::OpennessAsk`]; re-exported so the harness and its callers name the same rule.
pub use eprfs_core::OpennessAsk;

/// A reach gate over two lists: public roots, and roots each named reader may read. An unknown
/// root refuses `NoRecord`; a known root the requester may not read refuses `OutsideStanding`.
#[derive(Debug, Clone, Default)]
pub struct ListGate {
    pub public: Vec<BlobCid>,
    pub readers: HashMap<BlobCid, Vec<String>>,
}

impl ReachGate for ListGate {
    fn admit_root(&self, who: &Requester, root: &BlobCid) -> std::result::Result<(), ReachRefusal> {
        if self.public.contains(root) {
            return Ok(());
        }
        match self.readers.get(root) {
            None => Err(ReachRefusal::new(ReachReason::NoRecord, None)),
            Some(readers) if who.agent().is_some_and(|a| readers.iter().any(|r| r == a)) => Ok(()),
            Some(_) => Err(ReachRefusal::new(
                ReachReason::OutsideStanding,
                Some("household".into()),
            )),
        }
    }

    fn admit_object(
        &self,
        who: &Requester,
        cid: &BlobCid,
    ) -> std::result::Result<(), ReachRefusal> {
        self.admit_root(who, cid)
    }
}

/// Connects records to tiers registered by id.
#[derive(Default, Clone)]
pub struct MapConnector(pub HashMap<TierId, Arc<dyn EprfsStorage>>);

impl MapConnector {
    pub fn with(mut self, id: TierId, tier: Arc<dyn EprfsStorage>) -> Self {
        self.0.insert(id, tier);
        self
    }
}

impl TierConnector for MapConnector {
    fn connect(&self, record: &TierRecord) -> Option<Arc<dyn EprfsStorage>> {
        self.0.get(&record.id).cloned()
    }
}

/// A held-record tier record of `class`.
pub fn record(id: &TierId, class: TierClass) -> TierRecord {
    TierRecord {
        id: id.clone(),
        class,
        sources: vec![TierSource::HeldRecord {
            record: "roster".into(),
        }],
        endpoint: None,
        standing: None,
        proximity: None,
    }
}

/// What a factory composes into a requester-side chain.
pub struct TierContractSetup {
    pub own: Arc<dyn EprfsStorage>,
    /// In the order the chain must ask them (distinct classes, narrowest first).
    pub records: Vec<TierRecord>,
    /// The order the chain applies per request.
    pub order: Arc<dyn TierOrder>,
    pub connector: Arc<dyn TierConnector>,
    pub ask: Arc<dyn AskPolicy>,
    pub gate: Arc<dyn ForwardGate>,
    pub who: Requester,
}

/// Builds the storage under test from a setup.
pub type ChainFactory<'f> = &'f (dyn Fn(TierContractSetup) -> Arc<dyn EprfsStorage> + Sync);

struct Fixture {
    own: Arc<CountingTier>,
    tiers: Vec<(TierId, Arc<CountingTier>)>,
    chain: Arc<dyn EprfsStorage>,
    root: BlobCid,
}

const CLASSES: [TierClass; 3] = [
    TierClass::PersonDevice,
    TierClass::Household,
    TierClass::Hub,
];

fn fixture(
    make: ChainFactory<'_>,
    tiers: usize,
    ask: Arc<dyn AskPolicy>,
    gate: Arc<dyn ForwardGate>,
) -> Fixture {
    let own = Arc::new(CountingTier::new(TierId::agent("uhCAkcontract-own")));
    let root = BlobCid::compute(b"contract-root");
    let mut connector = MapConnector::default();
    let mut records = Vec::new();
    let mut made = Vec::new();
    for (i, class) in CLASSES.iter().take(tiers).enumerate() {
        let id = TierId::agent(format!("uhCAkcontract-tier-{i}"));
        let tier = Arc::new(CountingTier::new(id.clone()));
        connector = connector.with(id.clone(), tier.clone());
        records.push(record(&id, *class));
        made.push((id, tier));
    }
    let chain = make(TierContractSetup {
        own: own.clone(),
        records,
        order: Arc::new(ProximityOrder),
        connector: Arc::new(connector),
        ask,
        gate,
        who: Requester::authenticated("uhCAkcontract-requester", KeyCustody::DeviceHeld),
    });
    Fixture {
        own,
        tiers: made,
        chain,
        root,
    }
}

fn item(cid: &BlobCid, path: &str) -> BatchItem {
    BatchItem::new(
        cid.clone(),
        TreePath::from_slash_bytes(path.as_bytes()).expect("a valid contract path"),
    )
}

fn fetch_opts() -> BatchOptions {
    FetchPolicy::FetchIfMissing.into()
}

async fn ask(fx: &Fixture, items: &[BatchItem], opts: &BatchOptions) -> CollectSink {
    let mut sink = CollectSink::default();
    fx.chain
        .fetch_blobs(&fx.root, items, opts, &mut sink)
        .await
        .expect("a chain answers absences through the sink, never as an error");
    sink
}

/// Contract 12, applied to every absence the harness sees.
fn three_parts(item_no: u8, why: &Unanswered<ItemAbsence>) {
    if let Meanwhile::Nothing { because } = &why.meanwhile {
        assert!(
            !because.is_empty(),
            "contract {item_no}/12: meanwhile says nothing without a because"
        );
    }
    if let ComeBack::OnEvent { event } = &why.come_back {
        assert!(
            !event.is_empty(),
            "contract {item_no}/12: come-back names no event"
        );
    }
}

fn only_absence(item_no: u8, sink: &CollectSink) -> Unanswered<ItemAbsence> {
    assert_eq!(
        sink.answers.len(),
        1,
        "contract {item_no}: one answer per item"
    );
    match &sink.answers[0].1 {
        BatchAnswer::Absent(why) => {
            three_parts(item_no, why);
            why.clone()
        }
        other => panic!("contract {item_no}: expected an absence, got {other:?}"),
    }
}

fn arrived(item_no: u8, sink: &CollectSink) -> Vec<(BatchItem, Bytes, TierId)> {
    sink.answers
        .iter()
        .map(|(item, answer)| match answer {
            BatchAnswer::Arrived { bytes, from } => (item.clone(), bytes.clone(), from.clone()),
            BatchAnswer::Absent(why) => {
                panic!("contract {item_no}: expected {item:?} to arrive, got {why:?}")
            }
        })
        .collect()
}

fn no_has_blob(item_no: u8, fx: &Fixture) {
    assert_eq!(
        fx.own.calls().has_blob,
        0,
        "contract {item_no}: the own store was asked has_blob"
    );
    for (id, tier) in &fx.tiers {
        assert_eq!(
            tier.calls().has_blob,
            0,
            "contract {item_no}: {id} was asked has_blob"
        );
    }
}

/// Seam A's chain contract (interfaces spec §A.4), items 1–10 and 12, against the composition
/// `make` builds. Panics naming the item on a violation.
pub async fn assert_tier_contract(make: ChainFactory<'_>) {
    let allow: Arc<dyn AskPolicy> = Arc::new(AllowAll);
    let open: Arc<dyn ForwardGate> = Arc::new(OpenGate);

    // 1. Own store first, without has_blob: a hit makes zero outer calls.
    {
        let fx = fixture(make, 2, allow.clone(), open.clone());
        let cid = fx.own.insert(b"own hit");
        let sink = ask(&fx, &[item(&cid, "a")], &fetch_opts()).await;
        assert_eq!(
            arrived(1, &sink).len(),
            1,
            "contract 1: the own hit arrives"
        );
        for (id, tier) in &fx.tiers {
            assert!(
                tier.calls().batches.is_empty(),
                "contract 1: {id} was asked on an own hit"
            );
        }
        no_has_blob(1, &fx);
    }

    // 2. LocalOnly stays local: absences at once, zero outer calls.
    {
        let fx = fixture(make, 2, allow.clone(), open.clone());
        let held_outside = fx.tiers[0].1.insert(b"held outside");
        let sink = ask(
            &fx,
            &[item(&held_outside, "a")],
            &FetchPolicy::LocalOnly.into(),
        )
        .await;
        let why = only_absence(2, &sink);
        assert!(
            matches!(&why.why, ItemAbsence::NotHeld { tried } if tried.is_empty()),
            "contract 2: a local-only miss is NotHeld naming no tier, got {why:?}"
        );
        for (id, tier) in &fx.tiers {
            assert!(
                tier.calls().batches.is_empty(),
                "contract 2: {id} was dialled under LocalOnly"
            );
        }
    }

    // 3. A pathless read never leaves the device, and says so.
    {
        let fx = fixture(make, 1, allow.clone(), open.clone());
        let elsewhere = fx.tiers[0].1.insert(b"pathless");
        match fx.chain.fetch_blob(&elsewhere, FetchPolicy::FetchIfMissing).await {
            Err(EprfsError::RootRequired(cid)) => assert_eq!(cid, elsewhere),
            other => panic!("contract 3: a pathless fetch past the own store must be RootRequired, got {other:?}"),
        }
        assert_eq!(
            fx.chain.has_blob(&elsewhere).await.ok(),
            Some(BlobPresence::Unknown),
            "contract 3: an own miss with outer tiers is Unknown, never Missing"
        );
        assert!(
            fx.tiers[0].1.calls().batches.is_empty(),
            "contract 3: a pathless read dialled out"
        );
        assert_eq!(
            fx.tiers[0].1.calls().fetch_blob,
            0,
            "contract 3: a pathless read dialled out"
        );
    }

    // 4. Ask before asking: a tier the policy refuses sees zero calls.
    {
        let denied = TierId::agent("uhCAkcontract-tier-0");
        let fx = fixture(make, 2, Arc::new(DenyTiers(vec![denied])), open.clone());
        let cid = fx.tiers[1].1.insert(b"second tier");
        let sink = ask(&fx, &[item(&cid, "a")], &fetch_opts()).await;
        assert_eq!(arrived(4, &sink).len(), 1);
        assert!(
            fx.tiers[0].1.calls().batches.is_empty(),
            "contract 4: a refused tier was asked"
        );
        // 4b. The root's reach reaches the policy: a public-standing tier is never asked for an
        //     intimate root; a household-standing tier is asked for a public root.
        let levels = OpennessAsk {
            household: ReachLevel(2),
            public: ReachLevel(4),
        };
        let fx = fixture(make, 3, Arc::new(levels), open.clone());
        let cid = fx.tiers[2].1.insert(b"on the hub");
        let intimate = BatchOptions {
            root_reach: Some(ReachLevel(1)),
            ..fetch_opts()
        };
        let sink = ask(&fx, &[item(&cid, "a")], &intimate).await;
        only_absence(4, &sink);
        assert!(
            fx.tiers[2].1.calls().batches.is_empty(),
            "contract 4b: a public-standing tier was asked for an intimate root"
        );
        let public = BatchOptions {
            root_reach: Some(ReachLevel(4)),
            ..fetch_opts()
        };
        let sink = ask(&fx, &[item(&cid, "a")], &public).await;
        assert_eq!(
            arrived(4, &sink).len(),
            1,
            "contract 4b: a public root may ask every tier"
        );
    }

    // 5. Strict tiering.
    {
        // The first tier is asked first, with the rest as `via`; a tier it names in NotHeld.tried
        // is never re-asked.
        let fx = fixture(make, 3, allow.clone(), open.clone());
        let (a, b, c) = (&fx.tiers[0], &fx.tiers[1], &fx.tiers[2]);
        a.1.behave(Behaviour::NotHeldNaming(vec![b.0.clone()]));
        let cid = c.1.insert(b"third tier");
        b.1.insert(b"third tier");
        let sink = ask(&fx, &[item(&cid, "a")], &fetch_opts()).await;
        let got = arrived(5, &sink);
        assert_eq!(
            got[0].2, c.0,
            "contract 5: bytes come from the tier that held them"
        );
        let a_calls = a.1.calls();
        assert_eq!(a_calls.batches.len(), 1);
        assert_eq!(
            a_calls.batches[0].1,
            vec![b.0.clone(), c.0.clone()],
            "contract 5: via is the rest of the list"
        );
        assert!(
            b.1.calls().batches.is_empty(),
            "contract 5: a tier named in NotHeld.tried was re-asked"
        );

        // Refused and OverBudget are final for the item.
        for why in [
            ItemAbsence::refused(
                &TierId::agent("x"),
                ReachRefusal::new(ReachReason::OutsideStanding, None),
            ),
            ItemAbsence::over_budget(
                &TierId::agent("x"),
                &OverBudget {
                    retry_after_ms: 1000,
                    retry_at_epoch_secs: 42,
                    rule: "r".into(),
                },
            ),
        ] {
            let fx = fixture(make, 2, allow.clone(), open.clone());
            fx.tiers[0].1.behave(Behaviour::AnswerAll(why.clone()));
            let cid = fx.tiers[1].1.insert(b"never reached");
            let sink = ask(&fx, &[item(&cid, "a")], &fetch_opts()).await;
            assert_eq!(
                only_absence(5, &sink).why,
                why.why,
                "contract 5: a final answer is passed on"
            );
            assert!(
                fx.tiers[1].1.calls().batches.is_empty(),
                "contract 5: a final answer was routed around"
            );
        }

        // Unreachable and MembershipUnknown advance.
        for behaviour in [
            Behaviour::Fail,
            Behaviour::AnswerAll(ItemAbsence::membership_unknown(&TierId::agent("x"))),
        ] {
            let fx = fixture(make, 2, allow.clone(), open.clone());
            fx.tiers[0].1.behave(behaviour);
            let cid = fx.tiers[1].1.insert(b"second");
            let sink = ask(&fx, &[item(&cid, "a")], &fetch_opts()).await;
            assert_eq!(
                arrived(5, &sink).len(),
                1,
                "contract 5: an unestablished absence advances"
            );
        }

        // Nothing that is not observed absent reads as NotHeld.
        let fx = fixture(make, 2, allow.clone(), open.clone());
        fx.tiers[0].1.behave(Behaviour::Fail);
        let cid = BlobCid::compute_raw(b"nowhere");
        let sink = ask(&fx, &[item(&cid, "a")], &fetch_opts()).await;
        let why = only_absence(5, &sink);
        assert!(
            matches!(why.why, ItemAbsence::Unreachable { .. }),
            "contract 5: an unreachable tier left absence unestablished, got {why:?}"
        );
    }

    // 6. Verify at every hop: bad bytes are Integrity, counted, never stored.
    {
        let fx = fixture(make, 2, allow.clone(), open.clone());
        fx.tiers[0].1.behave(Behaviour::Corrupt);
        let cid = fx.tiers[1].1.insert(b"good bytes");
        let sink = ask(&fx, &[item(&cid, "a")], &fetch_opts()).await;
        let got = arrived(6, &sink);
        assert_eq!(
            got[0].1.as_ref(),
            b"good bytes",
            "contract 6: only verified bytes reach the sink"
        );
        assert_eq!(got[0].2, fx.tiers[1].0);

        let fx = fixture(make, 1, allow.clone(), open.clone());
        fx.tiers[0].1.behave(Behaviour::Corrupt);
        let cid = BlobCid::compute_raw(b"wanted");
        let sink = ask(&fx, &[item(&cid, "a")], &fetch_opts()).await;
        let why = only_absence(6, &sink);
        assert_eq!(
            why.why,
            ItemAbsence::Integrity {
                from: fx.tiers[0].0.clone()
            },
            "contract 6"
        );
        assert!(!fx.own.holds(&cid), "contract 6: bad bytes were stored");
    }

    // 7. Store before return: forwarded bytes are in the own store before the batch returns, so a
    //    second read is an own hit; a failed write-back still delivers. (The chain streams, so
    //    the caller's sink sees the bytes as they arrive and the write-back follows the tier's
    //    answer: revision of §A.4 contract 7, which put the write before `sink.arrived`.)
    {
        let fx = fixture(make, 1, allow.clone(), open.clone());
        let cid = fx.tiers[0].1.insert(b"forwarded");
        ask(&fx, &[item(&cid, "a")], &fetch_opts()).await;
        assert!(
            fx.own.holds(&cid),
            "contract 7: forwarded bytes were not written back"
        );
        fx.tiers[0].1.reset();
        let sink = ask(&fx, &[item(&cid, "a")], &fetch_opts()).await;
        assert_eq!(arrived(7, &sink).len(), 1);
        assert!(
            fx.tiers[0].1.calls().batches.is_empty(),
            "contract 7: a second read was not an own hit"
        );

        let fx = fixture(make, 1, allow.clone(), open.clone());
        fx.own.refuse_puts(true);
        let cid = fx.tiers[0].1.insert(b"cannot cache");
        let sink = ask(&fx, &[item(&cid, "a")], &fetch_opts()).await;
        assert_eq!(
            arrived(7, &sink).len(),
            1,
            "contract 7: a failed write-back withheld the read"
        );
    }

    // 8. Bounded upstream: past the gate the item is OverBudget with its come-back time.
    {
        let budget = OverBudget {
            retry_after_ms: 5000,
            retry_at_epoch_secs: 1_700_000_005,
            rule: "phone-forward".into(),
        };
        let fx = fixture(make, 1, allow.clone(), Arc::new(ClosedGate(budget.clone())));
        let cid = fx.tiers[0].1.insert(b"over budget");
        let sink = ask(&fx, &[item(&cid, "a")], &fetch_opts()).await;
        let why = only_absence(8, &sink);
        assert!(
            matches!(&why.why, ItemAbsence::OverBudget { rule, .. } if *rule == budget.rule),
            "contract 8: over budget names its rule, got {why:?}"
        );
        assert_eq!(
            why.come_back,
            ComeBack::At {
                epoch_secs: 1_700_000_005
            },
            "contract 8"
        );
        assert!(
            fx.tiers[0].1.calls().batches.is_empty(),
            "contract 8: a tier was dialled past the gate"
        );
    }

    // 9. Named failure: tried lists every tier asked, in order, by the dialled ids; no answer
    //    carries another party's `own`.
    {
        let fx = fixture(make, 3, allow.clone(), open.clone());
        fx.tiers[0]
            .1
            .behave(Behaviour::AnswerAll(ItemAbsence::not_held(vec![
                TierId::own(),
            ])));
        let cid = BlobCid::compute_raw(b"held by nobody");
        let sink = ask(&fx, &[item(&cid, "a")], &fetch_opts()).await;
        let why = only_absence(9, &sink);
        let ids: Vec<TierId> = fx.tiers.iter().map(|(id, _)| id.clone()).collect();
        assert_eq!(
            why.why,
            ItemAbsence::NotHeld { tried: ids },
            "contract 9: tried in order, own rewritten"
        );
    }

    // 10. Batch equivalence: each item answered once; bytes fetched once per CID and fanned out;
    //     unrequested CIDs dropped.
    {
        let fx = fixture(make, 1, allow.clone(), open.clone());
        let x = fx.tiers[0].1.insert(b"x");
        let y = BlobCid::compute_raw(b"y");
        let extra = BlobCid::compute_raw(b"extra");
        fx.tiers[0].1.behave(Behaviour::ServeWithExtra(
            extra.clone(),
            Bytes::from_static(b"extra"),
        ));
        let items = [item(&x, "p"), item(&x, "q"), item(&y, "r")];
        let sink = ask(&fx, &items, &fetch_opts()).await;
        assert_eq!(
            sink.answers.len(),
            3,
            "contract 10: each item answered exactly once"
        );
        let mut paths: Vec<String> = sink
            .answers
            .iter()
            .map(|(i, _)| i.path.to_string())
            .collect();
        paths.sort();
        assert_eq!(paths, vec!["p", "q", "r"]);
        assert!(
            sink.answers.iter().all(|(i, _)| i.cid != extra),
            "contract 10: an unrequested CID reached the sink"
        );
        let asked = fx.tiers[0].1.calls().asked_cids();
        assert_eq!(
            asked.iter().filter(|c| **c == x).count(),
            1,
            "contract 10: a CID was fetched twice"
        );
        assert!(
            !fx.own.holds(&extra),
            "contract 10: an unrequested CID was stored"
        );
    }
}

/// Item 11 and the default batch behaviour against a leaf store that holds `held` and not
/// `missing`: a leaf treats `FetchIfMissing` as `LocalOnly`, answers each batch item once, and
/// names a miss `NotHeld`.
pub async fn assert_leaf_contract(store: &dyn EprfsStorage, held: &BlobCid, missing: &BlobCid) {
    let local = store.fetch_blob(missing, FetchPolicy::LocalOnly).await;
    let fetch = store.fetch_blob(missing, FetchPolicy::FetchIfMissing).await;
    assert!(
        local.is_err() && fetch.is_err(),
        "contract 11: a leaf store fetched a missing blob"
    );
    assert!(
        matches!(
            fetch,
            Err(EprfsError::BlobNotFound(_) | EprfsError::BlobNotLocal(_))
        ),
        "contract 11: a leaf miss is not-found under either policy, got {fetch:?}"
    );
    let root = BlobCid::compute(b"leaf-contract-root");
    let items = [item(held, "a"), item(missing, "b"), item(held, "c")];
    let mut sink = CollectSink::default();
    store
        .fetch_blobs(
            &root,
            &items,
            &FetchPolicy::FetchIfMissing.into(),
            &mut sink,
        )
        .await
        .expect("a leaf store answers absences through the sink");
    assert_eq!(
        sink.answers.len(),
        3,
        "contract 10/11: each item answered once"
    );
    for (answered, answer) in &sink.answers {
        match answer {
            BatchAnswer::Arrived { bytes, .. } => {
                assert_eq!(&answered.cid, held);
                assert!(
                    held.verifies(bytes),
                    "contract 6: unverified bytes reached the sink"
                );
            }
            BatchAnswer::Absent(why) => {
                assert_eq!(&answered.cid, missing);
                assert!(
                    matches!(why.why, ItemAbsence::NotHeld { .. }),
                    "contract 11: {why:?}"
                );
                three_parts(11, why);
            }
        }
    }
}

/// What the outer-tier contract hands a factory: one DAG-CBOR tree and every object the tier
/// under test is to hold.
///
/// The tree is `root → {a, b, c, d, e}`: `a` and `c` are the same leaf (one object at two paths),
/// `d` another leaf, `b` a leaf in the tree that the tier does not hold, and `e` a leaf held under
/// its CID with bytes that do not match it. `held` carries the root's node and the leaves of `a`,
/// `c`, `d` and `e`; the factory may refuse to keep the tampered pair (a verifying store will).
pub struct OuterTierSetup {
    pub root: BlobCid,
    pub held: Vec<(BlobCid, Bytes)>,
    /// Whom the tier serves: `true`, a requester the tier admits to `root`; `false`, one outside
    /// the root's reach, to whom the tier must refuse every item alike.
    pub requester_admitted: bool,
}

/// Builds the outer tier under test from what it is to hold and whom it serves (a loopback over
/// a `TierEndpoint`, say, calls it as an admitted requester or as a stranger).
pub type OuterTierFactory<'f> = &'f (dyn Fn(OuterTierSetup) -> Arc<dyn EprfsStorage> + Sync);

/// A sink that says `Stop` on the first arrival and counts what comes after.
#[derive(Default)]
struct StopFirst {
    arrivals: usize,
    after_stop: usize,
}

impl BlobSink for StopFirst {
    fn arrived(&mut self, _item: &BatchItem, _bytes: Bytes, _from: &TierId) -> SinkControl {
        self.arrivals += 1;
        if self.arrivals > 1 {
            self.after_stop += 1;
        }
        SinkControl::Stop
    }

    fn absent(&mut self, _item: &BatchItem, _why: Unanswered<ItemAbsence>) {}
}

/// An outer tier's own obligations, as a requester's chain sees them over the wire: the
/// contract a remote tier adapter (`HttpTier`, a blob-store tier, a peer-transport tier, or a
/// loopback over a `TierEndpoint`) runs. [`assert_tier_contract`] runs a requester's
/// composition and [`assert_leaf_contract`] a leaf store; this runs the tier itself. It panics
/// naming the obligation:
///
/// 1. every batch item is answered exactly once, a CID at two paths once per path;
/// 2. bytes reach the sink only when they verify against the item's CID, so the tampered object
///    is `Integrity` (or `NotHeld`, when the tier refused to keep it), never an arrival;
/// 3. every arrival is stamped with the tier's own `tier_id`, which is never `own`;
/// 4. `own` never crosses the wire: every absence serializes (`TierId::own` refuses to);
/// 5. every absence carries its three parts (contract 12);
/// 6. what the tier holds arrives, and the object it does not hold is `NotHeld`;
/// 7. `NotHeld.tried` names only the tier itself and the tiers it was handed as `via`;
/// 8. after the sink says `Stop`, no further bytes arrive;
/// 9. a held object asked at a path that leads elsewhere is `Refused { NotUnderRoot }`, never
///    an arrival: a tier serves an object only where the root links it (spec §12.3);
/// 10. to a requester outside the root's reach, every item, held or not, gets the same
///     `Refused`, and nothing arrives.
pub async fn assert_outer_tier_contract(make: OuterTierFactory<'_>) {
    let limits = eprfs_core::TreeLimits::default();
    let one = Bytes::from_static(b"outer-contract-one");
    let two = Bytes::from_static(b"outer-contract-two");
    let one_cid = BlobCid::compute_raw(&one);
    let two_cid = BlobCid::compute_raw(&two);
    let missing_cid = BlobCid::compute_raw(b"outer-contract-not-held");
    let tampered_cid = BlobCid::compute_raw(b"outer-contract-real");
    let leaf = |name: &str, cid: &BlobCid| eprfs_core::TreeEntry {
        name: name.as_bytes().to_vec(),
        kind: eprfs_core::TreeEntryKind::File {
            blob: cid.clone().into(),
            executable: false,
        },
    };
    let node = eprfs_core::TreeNode::new(vec![
        leaf("a", &one_cid),
        leaf("b", &missing_cid),
        leaf("c", &one_cid),
        leaf("d", &two_cid),
        leaf("e", &tampered_cid),
    ])
    .expect("a valid contract tree");
    let (root, root_bytes) = eprfs_core::encode_tree(&node, &limits).expect("an encodable tree");
    let held = vec![
        (root.clone(), Bytes::from(root_bytes)),
        (one_cid.clone(), one.clone()),
        (two_cid.clone(), two.clone()),
        (
            tampered_cid.clone(),
            Bytes::from_static(b"outer-contract-evil"),
        ),
    ];
    let tier = make(OuterTierSetup {
        root: root.clone(),
        held: held.clone(),
        requester_admitted: true,
    });
    let me = tier.tier_id();
    assert!(
        !me.is_own(),
        "outer contract 3: an outer tier's id is `own`, which never crosses the wire"
    );
    let items = [
        item(&one_cid, "a"),
        item(&missing_cid, "b"),
        item(&one_cid, "c"),
        item(&two_cid, "d"),
        item(&tampered_cid, "e"),
    ];
    let delegate = TierId::agent("uhCAkouter-contract-delegate");
    for via in [Vec::new(), vec![delegate.clone()]] {
        let opts = BatchOptions {
            via: via.clone(),
            ..fetch_opts()
        };
        let mut sink = CollectSink::default();
        let _ = tier.fetch_blobs(&root, &items, &opts, &mut sink).await;

        let mut answered: Vec<(BlobCid, TreePath)> = sink
            .answers
            .iter()
            .map(|(i, _)| (i.cid.clone(), i.path.clone()))
            .collect();
        let mut asked: Vec<(BlobCid, TreePath)> = items
            .iter()
            .map(|i| (i.cid.clone(), i.path.clone()))
            .collect();
        answered.sort_by(|a, b| a.1.cmp(&b.1));
        asked.sort_by(|a, b| a.1.cmp(&b.1));
        assert_eq!(
            answered, asked,
            "outer contract 1: each batch item is answered exactly once"
        );

        for (answered, answer) in &sink.answers {
            let path = answered.path.to_slash_string().unwrap_or_default();
            match answer {
                BatchAnswer::Arrived { bytes, from } => {
                    assert!(
                        answered.cid.verifies(bytes),
                        "outer contract 2: bytes for {path} failed their address and reached the sink"
                    );
                    assert_eq!(
                        from, &me,
                        "outer contract 3: an arrival for {path} is not stamped with the tier's id"
                    );
                    assert!(
                        [one_cid.clone(), two_cid.clone()].contains(&answered.cid),
                        "outer contract 6: {path} arrived, but the tier does not hold it"
                    );
                }
                BatchAnswer::Absent(why) => {
                    assert!(
                        serde_json::to_string(why).is_ok(),
                        "outer contract 4: the absence for {path} carries `own`: {why:?}"
                    );
                    three_parts(5, why);
                    if let ItemAbsence::NotHeld { tried } = &why.why {
                        for id in tried {
                            assert!(
                                id == &me || via.contains(id),
                                "outer contract 7: NotHeld for {path} names {id}, neither the tier nor its via"
                            );
                        }
                    }
                    if answered.cid == missing_cid {
                        assert!(
                            matches!(why.why, ItemAbsence::NotHeld { .. }),
                            "outer contract 6: the object the tier does not hold is not NotHeld: {why:?}"
                        );
                    } else if answered.cid == tampered_cid {
                        assert!(
                            matches!(
                                why.why,
                                ItemAbsence::Integrity { .. } | ItemAbsence::NotHeld { .. }
                            ),
                            "outer contract 2: the tampered object is neither Integrity nor NotHeld: {why:?}"
                        );
                    } else {
                        panic!(
                            "outer contract 6: {path}, which the tier holds, was absent: {why:?}"
                        );
                    }
                }
            }
        }
    }

    let mut stop = StopFirst::default();
    let _ = tier
        .fetch_blobs(&root, &items, &fetch_opts(), &mut stop)
        .await;
    assert_eq!(
        stop.after_stop, 0,
        "outer contract 8: bytes kept arriving after the sink said Stop"
    );

    // 9. A held object at a path that leads to another object.
    let elsewhere = [item(&one_cid, "d")];
    let mut sink = CollectSink::default();
    let _ = tier
        .fetch_blobs(&root, &elsewhere, &fetch_opts(), &mut sink)
        .await;
    match sink.answers.as_slice() {
        [(_, BatchAnswer::Absent(why))] => assert!(
            matches!(
                &why.why,
                ItemAbsence::Refused { refusal, .. } if refusal.reason == ReachReason::NotUnderRoot
            ),
            "outer contract 9: a held object at a path that leads elsewhere is not Refused {{ NotUnderRoot }}: {why:?}"
        ),
        [(_, BatchAnswer::Arrived { .. })] => {
            panic!("outer contract 9: a held object arrived at a path that leads elsewhere")
        }
        other => panic!("outer contract 1: the wrong-path item was answered {} times", other.len()),
    }

    // 10. A requester outside the root's reach.
    let outsider = make(OuterTierSetup {
        root: root.clone(),
        held,
        requester_admitted: false,
    });
    let mut sink = CollectSink::default();
    let _ = outsider
        .fetch_blobs(&root, &items, &fetch_opts(), &mut sink)
        .await;
    assert_eq!(
        sink.answers.len(),
        items.len(),
        "outer contract 1: each batch item is answered exactly once, outside reach too"
    );
    let mut refusals = Vec::new();
    for (answered, answer) in &sink.answers {
        match answer {
            BatchAnswer::Arrived { .. } => panic!(
                "outer contract 10: {} arrived for a requester outside the root's reach",
                answered.path.to_slash_string().unwrap_or_default()
            ),
            BatchAnswer::Absent(why) => {
                assert!(
                    matches!(why.why, ItemAbsence::Refused { .. }),
                    "outer contract 10: an item outside reach is not Refused: {why:?}"
                );
                refusals.push(why.clone());
            }
        }
    }
    assert!(
        refusals.windows(2).all(|pair| pair[0] == pair[1]),
        "outer contract 10: items outside reach were refused differently, held or not: {refusals:?}"
    );
}
