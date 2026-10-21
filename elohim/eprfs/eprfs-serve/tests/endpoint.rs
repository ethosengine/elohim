//! The serving contract (interfaces spec §A.5): reach once per batch, membership per item,
//! forwarding through the upstream chain, and every answer stamped with the endpoint's id.

use std::sync::Arc;

use bytes::Bytes;
use eprfs_core::{
    encode_tree, BatchAnswer, BatchItem, BatchOptions, BlobCid, BlobPresence, CollectSink,
    EprfsStorage, FetchPolicy, ItemAbsence, KeyCustody, OverBudget, ProximityOrder, ReachReason,
    Requester, TierChain, TierClass, TierId, TierRecord, TreeEntry, TreeEntryKind, TreeError,
    TreeLimits, TreeNode, TreePath, View,
};
use eprfs_serve::testing::{
    assert_outer_tier_contract, record, AllowAll, ClosedGate, CountingTier, ListGate, MapConnector,
    OpenGate, OuterTierSetup,
};
use eprfs_serve::TierEndpoint;

fn tp(path: &str) -> TreePath {
    TreePath::from_slash_bytes(path.as_bytes()).unwrap()
}

fn put_tree(tier: &CountingTier, entries: Vec<TreeEntry>) -> BlobCid {
    let (cid, bytes) =
        encode_tree(&TreeNode::new(entries).unwrap(), &TreeLimits::default()).unwrap();
    tier.insert_at(cid.clone(), Bytes::from(bytes));
    cid
}

fn file(name: &str, cid: &BlobCid) -> TreeEntry {
    TreeEntry {
        name: name.as_bytes().to_vec(),
        kind: TreeEntryKind::File {
            blob: cid.clone().into(),
            executable: false,
        },
    }
}

fn dir(name: &str, cid: &BlobCid) -> TreeEntry {
    TreeEntry {
        name: name.as_bytes().to_vec(),
        kind: TreeEntryKind::Directory {
            tree: cid.clone().into(),
        },
    }
}

/// root → app/{index.html, help.html}, held whole by `tier`.
struct App {
    root: BlobCid,
    app: BlobCid,
    index: BlobCid,
    help: BlobCid,
}

fn app(tier: &CountingTier) -> App {
    let index = tier.insert(b"<html>index</html>");
    let help = tier.insert(b"<html>help</html>");
    let app = put_tree(
        tier,
        vec![file("help.html", &help), file("index.html", &index)],
    );
    let root = put_tree(tier, vec![dir("app", &app)]);
    App {
        root,
        app,
        index,
        help,
    }
}

fn hub_id() -> TierId {
    TierId::agent("uhCAkhub")
}

fn matthew() -> Requester {
    Requester::authenticated("uhCAkmatthew", KeyCustody::DeviceHeld)
}

fn stranger() -> Requester {
    Requester::authenticated("uhCAkstranger", KeyCustody::DoorwayHeld)
}

fn household_gate(root: &BlobCid) -> ListGate {
    let mut gate = ListGate::default();
    gate.readers
        .insert(root.clone(), vec!["uhCAkmatthew".into()]);
    gate
}

fn fetch() -> BatchOptions {
    FetchPolicy::FetchIfMissing.into()
}

async fn serve(
    endpoint: &TierEndpoint<ListGate>,
    who: &Requester,
    root: &BlobCid,
    items: &[BatchItem],
) -> CollectSink {
    let mut sink = CollectSink::default();
    endpoint
        .serve_batch(who, root, items, &fetch(), &mut sink)
        .await
        .unwrap();
    sink
}

fn absence(answer: &BatchAnswer) -> &ItemAbsence {
    match answer {
        BatchAnswer::Absent(why) => &why.why,
        other => panic!("expected an absence, got {other:?}"),
    }
}

/// A requester's view of a remote tier: a tier that calls its endpoint as `who`, as the HTTP
/// batch route would after authenticating the requester.
struct ViaEndpoint(Arc<TierEndpoint<ListGate>>, TierId, Requester);

#[async_trait::async_trait]
impl EprfsStorage for ViaEndpoint {
    async fn resolve_epr(
        &self,
        r: &eprfs_core::EprRef,
    ) -> eprfs_core::Result<eprfs_core::EprRecord> {
        Err(eprfs_core::EprfsError::EprNotFound(r.clone()))
    }
    async fn has_blob(&self, _: &BlobCid) -> eprfs_core::Result<BlobPresence> {
        Ok(BlobPresence::Unknown)
    }
    async fn fetch_blob(
        &self,
        cid: &BlobCid,
        _: FetchPolicy,
    ) -> eprfs_core::Result<eprfs_core::BlobHandle> {
        Err(eprfs_core::EprfsError::RootRequired(cid.clone()))
    }
    async fn put_blob(&self, _: Bytes) -> eprfs_core::Result<BlobCid> {
        Err(eprfs_core::EprfsError::Storage("read-only".into()))
    }
    async fn publish_attestation(
        &self,
        _: eprfs_core::AttestationDraft,
    ) -> eprfs_core::Result<eprfs_core::EprRef> {
        Err(eprfs_core::EprfsError::Storage("read-only".into()))
    }
    fn tier_id(&self) -> TierId {
        self.1.clone()
    }
    async fn fetch_blobs(
        &self,
        root: &BlobCid,
        items: &[BatchItem],
        opts: &BatchOptions,
        sink: &mut (dyn eprfs_core::BlobSink + Send),
    ) -> eprfs_core::Result<eprfs_core::BatchReport> {
        self.0.serve_batch(&self.2, root, items, opts, sink).await
    }
}

/// A tier holding `setup.held`, served by a `TierEndpoint` that admits matthew to the root, and
/// read over the loopback as matthew, or as a stranger when the setup asks for a requester
/// outside reach. With `upstream`, the endpoint forwards through a chain over
/// its own store and an empty hub.
fn endpoint_tier(setup: OuterTierSetup, upstream: bool) -> Arc<dyn EprfsStorage> {
    let laptop_id = TierId::agent("uhCAkouter-laptop");
    let store = Arc::new(CountingTier::new(laptop_id.clone()));
    for (cid, bytes) in setup.held {
        store.insert_at(cid, bytes);
    }
    let chain = upstream.then(|| {
        TierChain::new(
            store.clone(),
            vec![record(&hub_id(), TierClass::Hub)],
            Arc::new(ProximityOrder),
            &MapConnector::default().with(hub_id(), Arc::new(CountingTier::new(hub_id()))),
            Arc::new(AllowAll),
            Arc::new(OpenGate),
            Requester::authenticated("uhCAkouter-laptop", KeyCustody::DeviceHeld),
        )
    });
    let endpoint = TierEndpoint::new(
        laptop_id.clone(),
        store,
        household_gate(&setup.root),
        chain,
        TreeLimits::default(),
    )
    .unwrap();
    let who = if setup.requester_admitted {
        matthew()
    } else {
        stranger()
    };
    Arc::new(ViaEndpoint(Arc::new(endpoint), laptop_id, who))
}

/// The outer-tier contract against the serving side read over a loopback: an endpoint answering
/// from its own store, and one forwarding through its upstream chain. The store is a
/// `CountingTier`, which does not verify, so the endpoint's own check is what keeps the tampered
/// object from being served.
#[tokio::test]
async fn an_endpoint_served_tier_passes_the_outer_tier_contract() {
    assert_outer_tier_contract(&|setup| endpoint_tier(setup, false)).await;
    assert_outer_tier_contract(&|setup| endpoint_tier(setup, true)).await;
}

/// The harness bites: a tier that serves what it holds without verifying it hands the tampered
/// object's bytes to the sink.
#[tokio::test]
#[should_panic(expected = "outer contract 2")]
async fn a_tier_serving_unverified_bytes_fails_the_outer_tier_contract() {
    assert_outer_tier_contract(&|setup| {
        let tier = Arc::new(CountingTier::new(TierId::agent("uhCAkunverified")));
        for (cid, bytes) in setup.held {
            tier.insert_at(cid, bytes);
        }
        tier as Arc<dyn EprfsStorage>
    })
    .await;
}

#[tokio::test]
async fn a_stranger_under_an_intimate_root_is_refused_the_same_whether_or_not_the_object_exists() {
    let hub = Arc::new(CountingTier::new(hub_id()));
    let a = app(&hub);
    let endpoint = TierEndpoint::new(
        hub_id(),
        hub.clone(),
        household_gate(&a.root),
        None,
        TreeLimits::default(),
    )
    .unwrap();
    let invented = BlobCid::compute_raw(b"never stored");
    let sink = serve(
        &endpoint,
        &stranger(),
        &a.root,
        &[
            BatchItem::new(a.index.clone(), tp("app/index.html")),
            BatchItem::new(invented, tp("app/nope")),
        ],
    )
    .await;
    assert_eq!(sink.answers.len(), 2);
    let first = absence(&sink.answers[0].1).clone();
    assert_eq!(
        &first,
        absence(&sink.answers[1].1),
        "the same refusal for held and missing"
    );
    assert!(matches!(
        first,
        ItemAbsence::Refused { ref by, ref refusal } if *by == hub_id() && refusal.reason == ReachReason::OutsideStanding
    ));
    assert!(
        hub.calls().batches.is_empty() && hub.calls().fetch_blob == 0,
        "no byte read before reach"
    );
    let counters = endpoint.counters();
    assert_eq!((counters.batches, counters.refused_batches), (1, 1));
}

#[tokio::test]
async fn an_unadmitted_requester_cannot_distinguish_private_and_unknown_roots() {
    let hub = Arc::new(CountingTier::new(hub_id()));
    let a = app(&hub);
    let unknown = BlobCid::compute(b"unknown root");
    let endpoint = TierEndpoint::new(
        hub_id(),
        hub.clone(),
        household_gate(&a.root),
        None,
        TreeLimits::default(),
    )
    .unwrap();
    let item = BatchItem::new(a.index.clone(), tp("app/index.html"));
    let mut expected = None;
    for root in [&a.root, &unknown] {
        let sink = serve(&endpoint, &stranger(), root, std::slice::from_ref(&item)).await;
        let BatchAnswer::Absent(answer) = &sink.answers[0].1 else {
            panic!("an unadmitted requester must receive a refusal");
        };
        assert!(matches!(
            &answer.why,
            ItemAbsence::Refused { refusal, .. }
                if refusal.reason == ReachReason::OutsideStanding && refusal.needs_standing.is_none()
        ));
        if let Some(previous) = &expected {
            assert_eq!(previous, answer);
        }
        expected = Some(answer.clone());
        let read = endpoint
            .serve_read(&stranger(), root, &item.path, &fetch())
            .await;
        let list = endpoint
            .serve_list(&stranger(), root, &item.path, &fetch())
            .await;
        let diff = endpoint
            .serve_diff(&stranger(), root, &a.root, &View::whole(), &fetch())
            .await;
        for error in [read.unwrap_err(), list.unwrap_err(), diff.unwrap_err()] {
            match error {
                TreeError::Unanswered { why, .. } => assert_eq!(why.as_ref(), answer),
                other => panic!("expected a uniform reach refusal, got {other:?}"),
            }
        }
        assert_eq!(
            endpoint
                .presence(&stranger(), root, &item.path, &item.cid)
                .await
                .unwrap(),
            BlobPresence::Missing
        );
    }
    assert!(hub.calls().batches.is_empty() && hub.calls().fetch_blob == 0);
}

#[tokio::test]
async fn a_member_is_served_and_a_cid_not_at_its_path_is_not_under_root_even_when_public() {
    let hub = Arc::new(CountingTier::new(hub_id()));
    let a = app(&hub);
    let mut gate = ListGate::default();
    gate.public.push(a.root.clone());
    let endpoint =
        TierEndpoint::new(hub_id(), hub.clone(), gate, None, TreeLimits::default()).unwrap();
    let sink = serve(
        &endpoint,
        &stranger(),
        &a.root,
        &[
            BatchItem::new(a.index.clone(), tp("app/index.html")),
            BatchItem::new(a.help.clone(), tp("app/index.html")),
            BatchItem::new(a.app.clone(), tp("app")),
            BatchItem::new(a.root.clone(), TreePath::root()),
        ],
    )
    .await;
    let arrived: Vec<&BatchItem> = sink
        .answers
        .iter()
        .filter(|(_, a)| matches!(a, BatchAnswer::Arrived { .. }))
        .map(|(i, _)| i)
        .collect();
    assert_eq!(arrived.len(), 3);
    for (item, answer) in &sink.answers {
        match answer {
            BatchAnswer::Arrived { from, .. } => {
                assert_eq!(from, &hub_id(), "stamped with the endpoint's id")
            }
            BatchAnswer::Absent(why) => {
                assert_eq!(item.cid, a.help);
                assert!(matches!(
                    &why.why,
                    ItemAbsence::Refused { refusal, .. } if refusal.reason == ReachReason::NotUnderRoot
                ));
            }
        }
    }
    assert_eq!(endpoint.counters().not_under_root, 1);
}

#[tokio::test]
async fn a_cold_forwarding_tier_resolves_the_spine_through_its_upstream_and_serves() {
    // The laptop holds nothing; the hub upstream holds the app. The laptop's chain own store is
    // the laptop's store.
    let laptop_id = TierId::agent("uhCAklaptop");
    let laptop = Arc::new(CountingTier::new(laptop_id.clone()));
    let hub = Arc::new(CountingTier::new(hub_id()));
    let a = app(&hub);
    let upstream = TierChain::new(
        laptop.clone(),
        vec![record(&hub_id(), TierClass::Hub)],
        Arc::new(ProximityOrder),
        &MapConnector::default().with(hub_id(), hub.clone()),
        Arc::new(AllowAll),
        Arc::new(OpenGate),
        Requester::authenticated("uhCAklaptop", KeyCustody::DeviceHeld),
    );
    let endpoint = TierEndpoint::new(
        laptop_id.clone(),
        laptop.clone(),
        household_gate(&a.root),
        Some(upstream),
        TreeLimits::default(),
    )
    .unwrap();
    let opts = BatchOptions {
        via: vec![hub_id()],
        ..fetch()
    };
    let mut sink = CollectSink::default();
    endpoint
        .serve_batch(
            &matthew(),
            &a.root,
            &[BatchItem::new(a.index.clone(), tp("app/index.html"))],
            &opts,
            &mut sink,
        )
        .await
        .unwrap();
    match &sink.answers[..] {
        [(_, BatchAnswer::Arrived { bytes, from })] => {
            assert_eq!(bytes.as_ref(), b"<html>index</html>");
            assert_eq!(from, &laptop_id);
        }
        other => panic!("expected the leaf, got {other:?}"),
    }
    // The spine and the leaf were written back to the laptop: it absorbs the repeat.
    assert!(laptop.holds(&a.root) && laptop.holds(&a.app) && laptop.holds(&a.index));

    // With no upstream and the spine not held, the item is MembershipUnknown, not final.
    let bare = Arc::new(CountingTier::new(laptop_id.clone()));
    let endpoint = TierEndpoint::new(
        laptop_id,
        bare,
        household_gate(&a.root),
        None,
        TreeLimits::default(),
    )
    .unwrap();
    let sink = serve(
        &endpoint,
        &matthew(),
        &a.root,
        &[BatchItem::new(a.index.clone(), tp("app/index.html"))],
    )
    .await;
    assert!(matches!(
        absence(&sink.answers[0].1),
        ItemAbsence::MembershipUnknown { .. }
    ));
}

#[tokio::test]
async fn over_budget_upstream_names_its_rule_and_no_answer_carries_own() {
    let laptop_id = TierId::agent("uhCAklaptop");
    let laptop = Arc::new(CountingTier::new(laptop_id.clone()));
    let hub = Arc::new(CountingTier::new(hub_id()));
    let a = app(&hub);
    // The laptop holds the spine but not the leaf.
    for cid in [&a.root, &a.app] {
        let handle = hub.fetch_blob(cid, FetchPolicy::LocalOnly).await.unwrap();
        laptop.insert_at(cid.clone(), handle.bytes);
    }
    let upstream = TierChain::new(
        laptop.clone(),
        vec![record(&hub_id(), TierClass::Hub)],
        Arc::new(ProximityOrder),
        &MapConnector::default().with(hub_id(), hub.clone()),
        Arc::new(AllowAll),
        Arc::new(ClosedGate(OverBudget {
            retry_after_ms: 3000,
            retry_at_epoch_secs: 1_700_000_003,
            rule: "household-forward-per-requester".into(),
        })),
        Requester::authenticated("uhCAklaptop", KeyCustody::DeviceHeld),
    );
    let endpoint = TierEndpoint::new(
        laptop_id.clone(),
        laptop,
        household_gate(&a.root),
        Some(upstream),
        TreeLimits::default(),
    )
    .unwrap();
    let opts = BatchOptions {
        via: vec![hub_id()],
        ..fetch()
    };
    let mut sink = CollectSink::default();
    endpoint
        .serve_batch(
            &matthew(),
            &a.root,
            &[
                BatchItem::new(a.index.clone(), tp("app/index.html")),
                BatchItem::new(a.help.clone(), tp("app/help.html")),
            ],
            &opts,
            &mut sink,
        )
        .await
        .unwrap();
    for (_, answer) in &sink.answers {
        let BatchAnswer::Absent(why) = answer else {
            panic!("over budget serves nothing")
        };
        assert!(
            matches!(&why.why, ItemAbsence::OverBudget { rule, by } if rule == "household-forward-per-requester" && *by == laptop_id)
        );
        // Every answer serializes: none carries the indexical own.
        serde_json::to_string(&why.why).expect("no answer carries own");
    }
    assert!(hub.calls().batches.is_empty());
}

#[tokio::test]
async fn serve_diff_refuses_a_stranger_when_either_root_is_beyond_reach() {
    let hub = Arc::new(CountingTier::new(hub_id()));
    let old = app(&hub);
    let fresh = hub.insert(b"<html>new index</html>");
    let new_app = put_tree(
        &hub,
        vec![file("help.html", &old.help), file("index.html", &fresh)],
    );
    let new_root = put_tree(&hub, vec![dir("app", &new_app)]);
    let mut gate = household_gate(&old.root);
    gate.public.push(new_root.clone());
    let endpoint =
        TierEndpoint::new(hub_id(), hub.clone(), gate, None, TreeLimits::default()).unwrap();
    match endpoint
        .serve_diff(&stranger(), &old.root, &new_root, &View::whole(), &fetch())
        .await
    {
        Err(TreeError::Unanswered { why, .. }) => {
            assert!(matches!(why.why, ItemAbsence::Refused { .. }))
        }
        other => panic!("expected the whole diff refused, got {other:?}"),
    }
    let changes = endpoint
        .serve_diff(&matthew(), &old.root, &new_root, &View::whole(), &fetch())
        .await
        .unwrap_or_else(|e| panic!("matthew reads both: {e}"));
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].path, tp("app/index.html"));
    // Read and list are judged the same way.
    assert!(endpoint
        .serve_read(&stranger(), &old.root, &tp("app/index.html"), &fetch())
        .await
        .is_err());
    let leaf = endpoint
        .serve_read(&matthew(), &old.root, &tp("app/index.html"), &fetch())
        .await
        .unwrap();
    assert_eq!(leaf.bytes.as_ref(), b"<html>index</html>");
    let listing = endpoint
        .serve_list(&stranger(), &new_root, &tp("app"), &fetch())
        .await
        .unwrap();
    assert_eq!(listing.entries.len(), 2);
    // Presence for a party outside the root's reach is Missing.
    assert_eq!(
        endpoint
            .presence(&stranger(), &old.root, &tp("app/index.html"), &old.index)
            .await
            .unwrap(),
        BlobPresence::Missing
    );
    assert_eq!(
        endpoint
            .presence(&matthew(), &old.root, &tp("app/index.html"), &old.index)
            .await
            .unwrap(),
        BlobPresence::Local
    );
}

#[tokio::test]
async fn new_refuses_an_indexical_id_and_an_upstream_over_another_store() {
    let store = Arc::new(CountingTier::new(hub_id()));
    assert!(TierEndpoint::new(
        TierId::own(),
        store.clone(),
        ListGate::default(),
        None,
        TreeLimits::default()
    )
    .is_err());
    let other = Arc::new(CountingTier::new(hub_id()));
    let chain = TierChain::new(
        other,
        vec![],
        Arc::new(ProximityOrder),
        &MapConnector::default(),
        Arc::new(AllowAll),
        Arc::new(OpenGate),
        Requester::anonymous(),
    );
    assert!(TierEndpoint::new(
        hub_id(),
        store,
        ListGate::default(),
        Some(chain),
        TreeLimits::default()
    )
    .is_err());
}

/// The interfaces spec's worked example (§A.6): Matthew's phone reads index.html of a household
/// app whose release root is intimate. The phone's chain asks the laptop (household) with via =
/// [hub]; the laptop holds the spine but not the leaf, admits the phone, checks membership and
/// forwards to the hub, which hits. Doorway-named peers are never asked for the intimate root.
#[tokio::test]
async fn worked_example_phone_reads_through_the_laptop_and_the_hub() {
    use eprfs_core::{ReachLevel, TierSource};
    use eprfs_serve::testing::OpennessAsk;

    let hub = Arc::new(CountingTier::new(hub_id()));
    let a = app(&hub);
    let laptop_id = TierId::agent("uhCAkjessica-laptop");
    let laptop = Arc::new(CountingTier::new(laptop_id.clone()));
    for cid in [&a.root, &a.app] {
        let handle = hub.fetch_blob(cid, FetchPolicy::LocalOnly).await.unwrap();
        laptop.insert_at(cid.clone(), handle.bytes);
    }
    let laptop_upstream = TierChain::new(
        laptop.clone(),
        vec![record(&hub_id(), TierClass::Hub)],
        Arc::new(ProximityOrder),
        &MapConnector::default().with(hub_id(), hub.clone()),
        Arc::new(AllowAll),
        Arc::new(OpenGate),
        Requester::authenticated("uhCAkjessica-laptop", KeyCustody::DeviceHeld),
    );
    let laptop_endpoint = Arc::new(
        TierEndpoint::new(
            laptop_id.clone(),
            laptop.clone(),
            household_gate(&a.root),
            Some(laptop_upstream),
            TreeLimits::default(),
        )
        .unwrap(),
    );

    // The phone holds the spine; its chain knows the laptop, the hub and a doorway-named peer.
    let phone = Arc::new(CountingTier::new(TierId::agent("uhCAkmatthew")));
    for cid in [&a.root, &a.app] {
        let handle = hub.fetch_blob(cid, FetchPolicy::LocalOnly).await.unwrap();
        phone.insert_at(cid.clone(), handle.bytes);
    }
    let peer_id = TierId::agent("uhCAkdelivery-peer");
    let peer = Arc::new(CountingTier::new(peer_id.clone()));
    let mut peer_record = record(&peer_id, TierClass::DoorwayPeer);
    peer_record.sources = vec![TierSource::Doorway {
        doorway: TierId::agent("uhCAkdoorway-w1"),
    }];
    let mut laptop_record = record(&laptop_id, TierClass::Household);
    laptop_record.standing = Some(ReachLevel(2));
    let mut hub_record = record(&hub_id(), TierClass::Hub);
    hub_record.standing = Some(ReachLevel(2));
    let phone_chain = TierChain::new(
        phone.clone(),
        vec![peer_record, hub_record, laptop_record],
        Arc::new(ProximityOrder),
        &MapConnector::default()
            .with(
                laptop_id.clone(),
                Arc::new(ViaEndpoint(
                    laptop_endpoint.clone(),
                    laptop_id.clone(),
                    matthew(),
                )),
            )
            .with(hub_id(), hub.clone())
            .with(peer_id.clone(), peer.clone()),
        Arc::new(OpennessAsk {
            household: ReachLevel(2),
            public: ReachLevel(4),
        }),
        Arc::new(OpenGate),
        matthew(),
    );
    let opts = BatchOptions {
        root_reach: Some(ReachLevel(2)),
        ..fetch()
    };
    let leaf = phone_chain
        .query_read_path(
            &a.root,
            &tp("app/index.html"),
            &opts,
            &TreeLimits::default(),
        )
        .await
        .unwrap();
    assert_eq!(leaf.bytes.as_ref(), b"<html>index</html>");
    // The laptop was asked with via = [hub]; it forwarded once; the hub hit once.
    assert_eq!(laptop_endpoint.counters().forwarded, 1);
    let laptop_calls: Vec<Vec<TierId>> = hub
        .calls()
        .batches
        .iter()
        .map(|(_, via)| via.clone())
        .collect();
    assert_eq!(
        laptop_calls,
        vec![Vec::<TierId>::new()],
        "the hub was asked once, by the laptop"
    );
    assert!(
        peer.calls().batches.is_empty(),
        "a doorway-named public peer is never asked for an intimate root"
    );
    let stats = phone_chain.stats();
    assert_eq!(stats.per_tier[&laptop_id].hits, 1);
    assert!(phone.holds(&a.index), "the phone wrote the leaf back");
}

/// §A.5 `presence`: an object reachable only through a root `who` may not read is `Missing`,
/// even when `who` names a root it may read. Membership is proved from what the tier holds.
#[tokio::test]
async fn presence_never_confirms_an_object_outside_the_admitted_root() {
    let hub = Arc::new(CountingTier::new(hub_id()));
    let public = app(&hub);
    let secret = hub.insert(b"held only under the intimate root");
    let intimate = put_tree(&hub, vec![file("diary.txt", &secret)]);
    let mut gate = household_gate(&intimate);
    gate.public.push(public.root.clone());
    let endpoint =
        TierEndpoint::new(hub_id(), hub.clone(), gate, None, TreeLimits::default()).unwrap();
    for path in ["app/index.html", "diary.txt", ""] {
        assert_eq!(
            endpoint
                .presence(&stranger(), &public.root, &tp(path), &secret)
                .await
                .unwrap(),
            BlobPresence::Missing,
            "the stranger names the public root and the secret's CID at {path:?}"
        );
    }
    assert_eq!(
        endpoint
            .presence(&stranger(), &intimate, &tp("diary.txt"), &secret)
            .await
            .unwrap(),
        BlobPresence::Missing
    );
    assert_eq!(
        endpoint
            .presence(&matthew(), &intimate, &tp("diary.txt"), &secret)
            .await
            .unwrap(),
        BlobPresence::Local
    );
    // A spine this tier does not hold says nothing either way.
    let bare = Arc::new(CountingTier::new(hub_id()));
    let mut gate = ListGate::default();
    gate.public.push(public.root.clone());
    let cold = TierEndpoint::new(hub_id(), bare, gate, None, TreeLimits::default()).unwrap();
    assert_eq!(
        cold.presence(
            &stranger(),
            &public.root,
            &tp("app/index.html"),
            &public.index
        )
        .await
        .unwrap(),
        BlobPresence::Unknown
    );
}

/// A batch past `max_pending_entries` is not answered (the requester's chain reads it as
/// unreachable and advances), and no byte or spine node is read for it.
#[tokio::test]
async fn a_batch_past_the_pending_entries_bound_is_not_answered() {
    let hub = Arc::new(CountingTier::new(hub_id()));
    let a = app(&hub);
    let mut gate = ListGate::default();
    gate.public.push(a.root.clone());
    let limits = TreeLimits {
        max_pending_entries: 2,
        ..TreeLimits::default()
    };
    let endpoint = TierEndpoint::new(hub_id(), hub.clone(), gate, None, limits).unwrap();
    let items: Vec<BatchItem> = (0..3)
        .map(|_| BatchItem::new(a.index.clone(), tp("app/index.html")))
        .collect();
    let mut sink = CollectSink::default();
    assert!(endpoint
        .serve_batch(&stranger(), &a.root, &items, &fetch(), &mut sink)
        .await
        .is_err());
    assert!(sink.answers.is_empty());
    assert!(hub.calls().batches.is_empty());
    assert_eq!(endpoint.counters().oversized_batches, 1);
}

/// A spine node this tier holds corrupt is an integrity failure for the items under it, not a
/// `MembershipUnknown` that hides it.
#[tokio::test]
async fn a_corrupt_held_spine_node_is_integrity_not_membership_unknown() {
    let hub = Arc::new(CountingTier::new(hub_id()));
    let a = app(&hub);
    hub.insert_at(a.app.clone(), Bytes::from_static(b"not the app node"));
    let mut gate = ListGate::default();
    gate.public.push(a.root.clone());
    let endpoint =
        TierEndpoint::new(hub_id(), hub.clone(), gate, None, TreeLimits::default()).unwrap();
    let sink = serve(
        &endpoint,
        &stranger(),
        &a.root,
        &[BatchItem::new(a.index.clone(), tp("app/index.html"))],
    )
    .await;
    assert!(matches!(
        absence(&sink.answers[0].1),
        ItemAbsence::Integrity { from } if *from == hub_id()
    ));
}

/// A4 on the depth-0 routes: a read's miss goes upstream under the inbound requester's budget
/// and only to the requester's `via`, never under the serving tier's own standing.
#[tokio::test]
async fn depth_zero_reads_forward_on_the_requesters_behalf_and_only_to_its_via() {
    struct OnlyFor(&'static str);
    impl eprfs_core::ForwardGate for OnlyFor {
        fn admit(
            &self,
            on_behalf: &Requester,
            _: &TierId,
            _: Option<u64>,
        ) -> std::result::Result<eprfs_core::ForwardPermit, OverBudget> {
            if on_behalf.agent() == Some(self.0) {
                Ok(eprfs_core::ForwardPermit::new(()))
            } else {
                Err(OverBudget {
                    retry_after_ms: 1000,
                    retry_at_epoch_secs: 1_700_000_001,
                    rule: "per-requester".into(),
                })
            }
        }
    }
    let laptop_id = TierId::agent("uhCAklaptop");
    let hub = Arc::new(CountingTier::new(hub_id()));
    let a = app(&hub);
    let endpoint_for = |only: &'static str| {
        let laptop = Arc::new(CountingTier::new(laptop_id.clone()));
        let upstream = TierChain::new(
            laptop.clone(),
            vec![record(&hub_id(), TierClass::Hub)],
            Arc::new(ProximityOrder),
            &MapConnector::default().with(hub_id(), hub.clone()),
            Arc::new(AllowAll),
            Arc::new(OnlyFor(only)),
            // The laptop's own standing, which the gate would admit.
            Requester::authenticated("uhCAklaptop", KeyCustody::DeviceHeld),
        );
        TierEndpoint::new(
            laptop_id.clone(),
            laptop,
            household_gate(&a.root),
            Some(upstream),
            TreeLimits::default(),
        )
        .unwrap()
    };
    let via_hub = BatchOptions {
        via: vec![hub_id()],
        ..fetch()
    };
    // The gate admits the laptop itself but not matthew: the read is over budget for matthew.
    match endpoint_for("uhCAklaptop")
        .serve_read(&matthew(), &a.root, &tp("app/index.html"), &via_hub)
        .await
    {
        Err(TreeError::Unanswered { why, .. }) => assert!(
            matches!(&why.why, ItemAbsence::OverBudget { rule, .. } if rule == "per-requester")
        ),
        other => panic!("expected over budget for matthew, got {other:?}"),
    }
    assert!(hub.calls().batches.is_empty());
    // Admitted for matthew and handed the hub as `via`, the read is served through it.
    let leaf = endpoint_for("uhCAkmatthew")
        .serve_read(&matthew(), &a.root, &tp("app/index.html"), &via_hub)
        .await
        .unwrap();
    assert_eq!(leaf.bytes.as_ref(), b"<html>index</html>");
    // Without the hub in `via`, the hub is never asked.
    hub.reset();
    assert!(endpoint_for("uhCAkmatthew")
        .serve_list(&matthew(), &a.root, &tp("app"), &fetch())
        .await
        .is_err());
    assert!(hub.calls().batches.is_empty());
}

/// A tier that verifies what it serves but ignores the path and the requester: it serves any held
/// CID at any path, to anyone. It must fail the outer-tier contract on membership (contract 9).
struct PathBlind(Arc<CountingTier>);

#[async_trait::async_trait]
impl EprfsStorage for PathBlind {
    async fn resolve_epr(
        &self,
        r: &eprfs_core::EprRef,
    ) -> eprfs_core::Result<eprfs_core::EprRecord> {
        self.0.resolve_epr(r).await
    }
    async fn has_blob(&self, cid: &BlobCid) -> eprfs_core::Result<BlobPresence> {
        self.0.has_blob(cid).await
    }
    async fn fetch_blob(
        &self,
        cid: &BlobCid,
        policy: FetchPolicy,
    ) -> eprfs_core::Result<eprfs_core::BlobHandle> {
        self.0.fetch_blob(cid, policy).await
    }
    async fn put_blob(&self, bytes: Bytes) -> eprfs_core::Result<BlobCid> {
        self.0.put_blob(bytes).await
    }
    async fn publish_attestation(
        &self,
        d: eprfs_core::AttestationDraft,
    ) -> eprfs_core::Result<eprfs_core::EprRef> {
        self.0.publish_attestation(d).await
    }
    fn tier_id(&self) -> TierId {
        self.0.tier_id()
    }
    async fn fetch_blobs(
        &self,
        _root: &BlobCid,
        items: &[BatchItem],
        _opts: &BatchOptions,
        sink: &mut (dyn eprfs_core::BlobSink + Send),
    ) -> eprfs_core::Result<eprfs_core::BatchReport> {
        let me = self.0.tier_id();
        for item in items {
            match self.0.fetch_blob(&item.cid, FetchPolicy::LocalOnly).await {
                Ok(handle) if item.cid.verifies(&handle.bytes) => {
                    if sink.arrived(item, handle.bytes, &me) == eprfs_core::SinkControl::Stop {
                        break;
                    }
                }
                Ok(_) => sink.absent(item, ItemAbsence::integrity(&me)),
                Err(_) => sink.absent(item, ItemAbsence::not_held(vec![])),
            }
        }
        Ok(eprfs_core::BatchReport::default())
    }
}

/// The harness bites on membership: a tier that verifies bytes but serves a held CID at any path
/// hands `one` over at a path that leads to `two`.
#[tokio::test]
#[should_panic(expected = "outer contract 9")]
async fn a_tier_serving_a_cid_at_any_path_fails_the_outer_tier_contract() {
    assert_outer_tier_contract(&|setup| {
        let tier = Arc::new(CountingTier::new(TierId::agent("uhCAkpath-blind")));
        for (cid, bytes) in setup.held {
            tier.insert_at(cid, bytes);
        }
        Arc::new(PathBlind(tier)) as Arc<dyn EprfsStorage>
    })
    .await;
}

/// The harness bites on reach: an endpoint whose gate admits everyone passes contracts 1-9 and
/// serves the requester the setup put outside reach.
#[tokio::test]
#[should_panic(expected = "outer contract 10")]
async fn a_tier_serving_a_requester_outside_reach_fails_the_outer_tier_contract() {
    assert_outer_tier_contract(&|setup| {
        let id = TierId::agent("uhCAkreach-blind");
        let store = Arc::new(CountingTier::new(id.clone()));
        for (cid, bytes) in setup.held {
            store.insert_at(cid, bytes);
        }
        let mut gate = household_gate(&setup.root);
        gate.readers
            .get_mut(&setup.root)
            .unwrap()
            .push("uhCAkstranger".into());
        let endpoint =
            TierEndpoint::new(id.clone(), store, gate, None, TreeLimits::default()).unwrap();
        let who = if setup.requester_admitted {
            matthew()
        } else {
            stranger()
        };
        Arc::new(ViaEndpoint(Arc::new(endpoint), id, who)) as Arc<dyn EprfsStorage>
    })
    .await;
}

/// A root holding `n` distinct leaves `f00000 …`, all held by `tier`.
fn wide(tier: &CountingTier, n: usize) -> (BlobCid, Vec<BatchItem>) {
    let mut entries = Vec::new();
    let mut items = Vec::new();
    for i in 0..n {
        let name = format!("f{i:05}");
        let cid = tier.insert(format!("leaf {i}").as_bytes());
        entries.push(file(&name, &cid));
        items.push(BatchItem::new(cid, tp(&name)));
    }
    (put_tree(tier, entries), items)
}

fn phone_chain(
    phone: Arc<CountingTier>,
    outer: Vec<(TierRecord, Arc<dyn EprfsStorage>)>,
) -> TierChain {
    let mut connect = MapConnector::default();
    let mut records = Vec::new();
    for (record, tier) in outer {
        connect = connect.with(record.id.clone(), tier);
        records.push(record);
    }
    TierChain::new(
        phone,
        records,
        Arc::new(ProximityOrder),
        &connect,
        Arc::new(AllowAll),
        Arc::new(OpenGate),
        matthew(),
    )
}

/// One more leaf than a serving tier's per-request bound, asked through a requester's chain at
/// the defaults: the chain asks in rounds the endpoint accepts, so every leaf arrives and no batch
/// is refused as oversized. The chain's bound and the endpoint's are one default.
#[tokio::test]
async fn a_chain_asks_an_endpoint_in_rounds_its_bound_admits() {
    let limits = TreeLimits::default();
    assert_eq!(
        eprfs_core::ChainLimits::default().max_request_items,
        limits.max_pending_entries
    );
    let laptop_id = TierId::agent("uhCAklaptop");
    let laptop = Arc::new(CountingTier::new(laptop_id.clone()));
    let n = limits.max_pending_entries + 1;
    let (root, items) = wide(&laptop, n);
    let endpoint = Arc::new(
        TierEndpoint::new(
            laptop_id.clone(),
            laptop.clone(),
            household_gate(&root),
            None,
            limits,
        )
        .unwrap(),
    );
    let phone = Arc::new(CountingTier::new(TierId::agent("uhCAkphone")));
    let chain = phone_chain(
        phone.clone(),
        vec![(
            record(&laptop_id, TierClass::Household),
            Arc::new(ViaEndpoint(endpoint.clone(), laptop_id.clone(), matthew())),
        )],
    );
    let mut sink = CollectSink::default();
    chain
        .fetch_blobs(&root, &items, &fetch(), &mut sink)
        .await
        .unwrap();
    let arrived = sink
        .answers
        .iter()
        .filter(|(_, a)| matches!(a, BatchAnswer::Arrived { .. }))
        .count();
    assert_eq!(arrived, n, "every leaf arrives");
    let counters = endpoint.counters();
    assert_eq!(counters.oversized_batches, 0);
    assert_eq!(counters.batches, 2, "two rounds: the bound, then one");
}

/// A forwarded `via` that repeats one upstream a thousand times asks it once, even though it
/// answers soft (MembershipUnknown) and so is never struck as tried.
#[tokio::test]
async fn a_repeated_via_asks_each_upstream_once() {
    let laptop_id = TierId::agent("uhCAklaptop");
    let laptop = Arc::new(CountingTier::new(laptop_id.clone()));
    let index = BlobCid::compute_raw(b"<html>index</html>");
    let app_node = put_tree(&laptop, vec![file("index.html", &index)]);
    let root = put_tree(&laptop, vec![dir("app", &app_node)]);
    let hub = Arc::new(CountingTier::new(hub_id()));
    hub.behave(eprfs_serve::testing::Behaviour::AnswerAll(
        ItemAbsence::membership_unknown(&hub_id()),
    ));
    let upstream = TierChain::new(
        laptop.clone(),
        vec![record(&hub_id(), TierClass::Hub)],
        Arc::new(ProximityOrder),
        &MapConnector::default().with(hub_id(), hub.clone()),
        Arc::new(AllowAll),
        Arc::new(OpenGate),
        Requester::authenticated("uhCAklaptop", KeyCustody::DeviceHeld),
    );
    let endpoint = TierEndpoint::new(
        laptop_id,
        laptop.clone(),
        household_gate(&root),
        Some(upstream),
        TreeLimits::default(),
    )
    .unwrap();
    let opts = BatchOptions {
        via: vec![hub_id(); 1000],
        ..fetch()
    };
    let mut sink = CollectSink::default();
    endpoint
        .serve_batch(
            &matthew(),
            &root,
            &[BatchItem::new(index, tp("app/index.html"))],
            &opts,
            &mut sink,
        )
        .await
        .unwrap();
    assert_eq!(hub.calls().batches.len(), 1, "the hub is asked once");
    assert!(matches!(
        absence(&sink.answers[0].1),
        ItemAbsence::MembershipUnknown { .. }
    ));
}

/// A caller's sink that checks, at every arrival, that the chain has already written back every
/// earlier arrival: the chain holds at most its write-back bound plus one object.
struct WrittenBackBefore {
    own: Arc<CountingTier>,
    arrivals: usize,
}

impl eprfs_core::BlobSink for WrittenBackBefore {
    fn arrived(&mut self, _: &BatchItem, _: Bytes, _: &TierId) -> eprfs_core::SinkControl {
        assert_eq!(
            self.own.calls().puts,
            self.arrivals,
            "every earlier arrival is written back before the next is taken"
        );
        self.arrivals += 1;
        eprfs_core::SinkControl::Continue
    }
    fn absent(&mut self, item: &BatchItem, why: seam_contracts::Unanswered<ItemAbsence>) {
        panic!("{:?} absent: {why:?}", item.path);
    }
}

/// With a one-byte write-back bound, each round ends after one verified object: the chain stores
/// it and asks the same tier again for the rest, so no batch is held whole.
#[tokio::test]
async fn the_chain_writes_back_between_rounds_within_its_byte_bound() {
    let hub = Arc::new(CountingTier::new(hub_id()));
    let (root, items) = wide(&hub, 5);
    let phone = Arc::new(CountingTier::new(TierId::agent("uhCAkphone")));
    let chain = phone_chain(
        phone.clone(),
        vec![(record(&hub_id(), TierClass::Hub), hub.clone())],
    )
    .with_limits(eprfs_core::ChainLimits {
        max_request_items: 4096,
        max_write_back_bytes: 1,
    });
    let mut sink = WrittenBackBefore {
        own: phone.clone(),
        arrivals: 0,
    };
    chain
        .fetch_blobs(&root, &items, &fetch(), &mut sink)
        .await
        .unwrap();
    assert_eq!(sink.arrivals, 5);
    let sizes: Vec<usize> = hub.calls().batches.iter().map(|(b, _)| b.len()).collect();
    assert_eq!(sizes, vec![5, 4, 3, 2, 1]);
    assert_eq!(phone.calls().puts, 5);
}

/// Warmth is per root: one long-lived chain asks the tier warm for each root first.
#[tokio::test]
async fn a_chain_orders_its_tiers_for_the_root_asked() {
    let a_id = TierId::agent("uhCAka");
    let b_id = TierId::agent("uhCAkb");
    let a = Arc::new(CountingTier::new(a_id.clone()));
    let b = Arc::new(CountingTier::new(b_id.clone()));
    let blob_a = a.insert(b"one");
    b.insert(b"one");
    let one = BlobCid::compute(b"root one");
    let two = BlobCid::compute(b"root two");
    let warm = |root: &BlobCid| eprfs_core::Proximity {
        network_rank: 0,
        warm_roots: vec![root.clone()],
        serves_extracted: false,
        rtt_ms: None,
        last_seen_ms: None,
    };
    let mut ra = record(&a_id, TierClass::Hub);
    ra.proximity = Some(warm(&one));
    let mut rb = record(&b_id, TierClass::Hub);
    rb.proximity = Some(warm(&two));
    // The own store refuses write-back, so the second read leaves the device too.
    let phone = Arc::new(CountingTier::new(TierId::agent("uhCAkphone")));
    phone.refuse_puts(true);
    let chain = phone_chain(phone, vec![(ra, a.clone()), (rb, b.clone())]);
    let item = [BatchItem::new(blob_a, tp("f"))];
    let mut sink = CollectSink::default();
    chain
        .fetch_blobs(&two, &item, &fetch(), &mut sink)
        .await
        .unwrap();
    assert_eq!(
        (a.calls().batches.len(), b.calls().batches.len()),
        (0, 1),
        "under root two, b (warm for it) is asked first and answers"
    );
    chain
        .fetch_blobs(&one, &item, &fetch(), &mut sink)
        .await
        .unwrap();
    assert_eq!(
        (a.calls().batches.len(), b.calls().batches.len()),
        (1, 1),
        "under root one, the same chain asks a first"
    );
}
