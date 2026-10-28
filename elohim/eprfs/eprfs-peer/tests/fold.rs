//! The fold over real iroh endpoints: device A asks device B's tier for B's channel tree and
//! appends B's crossing events to its own flow store, `via` B, once (spec §3.3).

mod support;

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use cid::Cid;
use elohim_epr_rea::{atom_cid, model::canonical_bytes, FlowEvent, SidecarRoster};
use eprfs_core::{
    encode_tree, AttestationDraft, BlobCid, BlobHandle, BlobPresence, DirListing, EprRecord,
    EprRef, EprfsError, EprfsStorage, FetchPolicy, LeafKind, ReadLeaf, TreeChange, TreeEntry,
    TreeEntryKind, TreeLimits, TreeNode, TreePath,
};
use eprfs_peer::{
    bind, channel_tier_id, fetch_channel, fold_events, fold_root, fold_root_planned, folded_from,
    project_channel, tier_call, via_device, ChannelEvent, ChannelHead, ChannelStore, ChannelTree,
    CrossingAllowList, FoldLimits, FoldPlan, FoldReader, FoldTarget, HeadTable, NackLeaf,
    PeerConfig, PeerDaemon, PeerError, Rules, TierAnswer, TierLimits, TierProtocol, TierRequest,
    CHANNEL_EVENTS, TIER_ALPN,
};
use futures_lite::StreamExt;
use iroh::endpoint::Connection;
use iroh::protocol::{AcceptError, ProtocolHandler, Router};
use iroh::{Endpoint, NodeAddr};
use support::{berth_event, household, loopback, policy, Checkout, Household, HANDLE};

const WITHIN: Duration = Duration::from_secs(20);

#[derive(Debug)]
struct RawTreeStore {
    blobs: HashMap<BlobCid, Bytes>,
}

#[derive(Debug, Clone)]
struct TestChannel {
    root: BlobCid,
    listings: HashMap<String, DirListing>,
    reads: HashMap<String, ReadLeaf>,
}

impl TestChannel {
    fn answer(&self, request: &TierRequest) -> TierAnswer {
        match request {
            TierRequest::List { path, .. } => self
                .listings
                .get(&path.to_string())
                .cloned()
                .map(TierAnswer::List)
                .unwrap_or_else(|| TierAnswer::Failed {
                    reason: format!("no listing for {path}"),
                }),
            TierRequest::Read { path, .. } => self
                .reads
                .get(&path.to_string())
                .cloned()
                .map(TierAnswer::Read)
                .unwrap_or_else(|| TierAnswer::Failed {
                    reason: format!("no leaf for {path}"),
                }),
            TierRequest::Diff { .. } => TierAnswer::Failed {
                reason: "no diff".into(),
            },
        }
    }

    fn listing(&self, path: &str) -> DirListing {
        self.listings[path].clone()
    }

    fn read(&self, path: &str) -> ReadLeaf {
        self.reads[path].clone()
    }

    fn event_kind(&self, cid: &Cid) -> TreeEntryKind {
        self.listing(CHANNEL_EVENTS)
            .entries
            .into_iter()
            .find(|entry| entry.name.as_slice() == cid.to_string().as_bytes())
            .expect("event entry")
            .kind
    }
}

fn test_channel(events: Vec<FlowEvent>, nacks: Vec<(String, NackLeaf)>) -> TestChannel {
    let limits = TreeLimits::default();
    let mut reads = HashMap::new();
    let mut event_entries = Vec::new();
    for event in events {
        let cid = atom_cid(&event).expect("event cid");
        let bytes = canonical_bytes(&event).expect("event bytes");
        let leaf = BlobCid::compute_raw(&bytes);
        let path = TreePath::from_slash_bytes(format!("{CHANNEL_EVENTS}/{cid}").as_bytes())
            .expect("event path");
        reads.insert(
            path.to_string(),
            ReadLeaf::new(
                path,
                leaf.clone(),
                LeafKind::File { executable: false },
                Bytes::from(bytes),
            ),
        );
        event_entries.push(TreeEntry {
            name: cid.to_string().into_bytes(),
            kind: TreeEntryKind::File {
                blob: leaf.into(),
                executable: false,
            },
        });
    }

    let mut nack_entries = Vec::new();
    for (device, nack) in nacks {
        let bytes = serde_json::to_vec(&nack).expect("nack bytes");
        let leaf = BlobCid::compute_raw(&bytes);
        let path = TreePath::from_slash_bytes(format!("channel/nacks/{device}").as_bytes())
            .expect("nack path");
        reads.insert(
            path.to_string(),
            ReadLeaf::new(
                path,
                leaf.clone(),
                LeafKind::File { executable: false },
                Bytes::from(bytes),
            ),
        );
        nack_entries.push(TreeEntry {
            name: device.into_bytes(),
            kind: TreeEntryKind::File {
                blob: leaf.into(),
                executable: false,
            },
        });
    }

    let listing = |path: TreePath, entries: Vec<TreeEntry>| {
        let node = TreeNode::new(entries.clone()).expect("tree");
        let (cid, _) = encode_tree(&node, &limits).expect("tree cid");
        (cid.clone(), DirListing::new(path, cid, entries))
    };
    let events_path = TreePath::from_slash_bytes(CHANNEL_EVENTS.as_bytes()).expect("events path");
    let (events_cid, events_listing) = listing(events_path, event_entries);
    let nacks_path = TreePath::from_slash_bytes(b"channel/nacks").expect("nacks path");
    let (nacks_cid, nacks_listing) = listing(nacks_path, nack_entries);
    let channel_entries = vec![
        TreeEntry {
            name: b"events".to_vec(),
            kind: TreeEntryKind::Directory {
                tree: events_cid.into(),
            },
        },
        TreeEntry {
            name: b"nacks".to_vec(),
            kind: TreeEntryKind::Directory {
                tree: nacks_cid.into(),
            },
        },
    ];
    let channel_path = TreePath::from_slash_bytes(b"channel").expect("channel path");
    let (channel_cid, channel_listing) = listing(channel_path, channel_entries);
    let root_entries = vec![TreeEntry {
        name: b"channel".to_vec(),
        kind: TreeEntryKind::Directory {
            tree: channel_cid.into(),
        },
    }];
    let (root, root_listing) = listing(TreePath::root(), root_entries);
    let listings = [root_listing, channel_listing, events_listing, nacks_listing]
        .into_iter()
        .map(|listing| (listing.path.to_string(), listing))
        .collect();
    TestChannel {
        root,
        listings,
        reads,
    }
}

type StubAnswer = dyn Fn(&TierRequest) -> TierAnswer + Send + Sync;

#[derive(Clone)]
struct StubTier {
    answer: Arc<StubAnswer>,
    requests: Arc<Mutex<Vec<TierRequest>>>,
}

impl std::fmt::Debug for StubTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StubTier").finish_non_exhaustive()
    }
}

impl ProtocolHandler for StubTier {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        while let Ok((mut send, mut recv)) = connection.accept_bi().await {
            let Ok(frame) = recv.read_to_end(16 * 1024 * 1024).await else {
                continue;
            };
            if frame.len() < 4 {
                continue;
            }
            let Ok(request) = serde_ipld_dagcbor::from_slice::<TierRequest>(&frame[4..]) else {
                continue;
            };
            self.requests
                .lock()
                .expect("request log")
                .push(request.clone());
            let answer = (self.answer)(&request);
            let body = serde_ipld_dagcbor::to_vec(&answer).expect("answer frame");
            let len = u32::try_from(body.len()).expect("small answer");
            let mut frame = len.to_be_bytes().to_vec();
            frame.extend_from_slice(&body);
            let _ = send.write_all(&frame).await;
            let _ = send.finish();
        }
        Ok(())
    }
}

struct MountedStub {
    router: Router,
    addr: NodeAddr,
}

struct MountedGossipStub {
    router: Router,
    endpoint: Endpoint,
    gossip: iroh_gossip::net::Gossip,
    addr: NodeAddr,
}

async fn mount_stub(key: &eprfs_peer::PeerKey, tier: StubTier) -> MountedStub {
    let endpoint = bind(key, None, vec![TIER_ALPN.to_vec()])
        .await
        .expect("bind stub");
    let addr = loopback(&endpoint);
    let router = Router::builder(endpoint).accept(TIER_ALPN, tier).spawn();
    MountedStub { router, addr }
}

async fn mount_gossip_stub(key: &eprfs_peer::PeerKey, tier: StubTier) -> MountedGossipStub {
    let endpoint = bind(
        key,
        None,
        vec![TIER_ALPN.to_vec(), iroh_gossip::ALPN.to_vec()],
    )
    .await
    .expect("bind gossip stub");
    let addr = loopback(&endpoint);
    let gossip = iroh_gossip::net::Gossip::builder().spawn(endpoint.clone());
    let router = Router::builder(endpoint.clone())
        .accept(TIER_ALPN, tier)
        .accept(iroh_gossip::ALPN, gossip.clone())
        .spawn();
    MountedGossipStub {
        router,
        endpoint,
        gossip,
        addr,
    }
}

#[async_trait]
impl EprfsStorage for RawTreeStore {
    async fn resolve_epr(&self, reference: &EprRef) -> eprfs_core::Result<EprRecord> {
        Err(EprfsError::EprNotFound(reference.clone()))
    }

    async fn has_blob(&self, cid: &BlobCid) -> eprfs_core::Result<BlobPresence> {
        Ok(if self.blobs.contains_key(cid) {
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
            .blobs
            .get(cid)
            .cloned()
            .ok_or_else(|| EprfsError::BlobNotFound(cid.clone()))?;
        Ok(BlobHandle {
            cid: cid.clone(),
            bytes,
        })
    }

    async fn put_blob(&self, _bytes: Bytes) -> eprfs_core::Result<BlobCid> {
        Err(EprfsError::Storage("test store is immutable".into()))
    }

    async fn publish_attestation(&self, _draft: AttestationDraft) -> eprfs_core::Result<EprRef> {
        Err(EprfsError::Storage("test store publishes nothing".into()))
    }
}

fn raw_channel(events: Vec<(Cid, Vec<u8>)>) -> (BlobCid, RawTreeStore) {
    let limits = TreeLimits::default();
    let mut blobs = HashMap::new();
    let mut event_entries = Vec::new();
    for (cid, bytes) in events {
        let leaf = BlobCid::compute_raw(&bytes);
        blobs.insert(leaf.clone(), Bytes::from(bytes));
        event_entries.push(TreeEntry {
            name: cid.to_string().into_bytes(),
            kind: TreeEntryKind::File {
                blob: leaf.into(),
                executable: false,
            },
        });
    }
    let mut node = |entries| {
        let (cid, bytes) =
            encode_tree(&TreeNode::new(entries).expect("tree"), &limits).expect("encodes tree");
        blobs.insert(cid.clone(), Bytes::from(bytes));
        cid
    };
    let events = node(event_entries);
    let nacks = node(vec![]);
    let channel = node(vec![
        TreeEntry {
            name: b"events".to_vec(),
            kind: TreeEntryKind::Directory {
                tree: events.into(),
            },
        },
        TreeEntry {
            name: b"nacks".to_vec(),
            kind: TreeEntryKind::Directory { tree: nacks.into() },
        },
    ]);
    let root = node(vec![TreeEntry {
        name: b"channel".to_vec(),
        kind: TreeEntryKind::Directory {
            tree: channel.into(),
        },
    }]);
    (root, RawTreeStore { blobs })
}

fn allow() -> CrossingAllowList {
    CrossingAllowList::from_policy(&policy()).expect("policy")
}

/// Device B serving its projected channel tree on the tier ALPN, as `epr peer serve` does.
struct ServingB {
    router: Router,
    addr: NodeAddr,
    store: Arc<ChannelStore>,
    checkout: Checkout,
}

impl ServingB {
    fn project(&self, allow: &CrossingAllowList) -> ChannelTree {
        let tree = project_channel(&self.checkout.store(), allow).expect("projects");
        self.store.install(&tree);
        tree
    }
}

async fn serve_b(h: &Household) -> ServingB {
    let store = Arc::new(ChannelStore::default());
    let protocol = TierProtocol::new(
        channel_tier_id(),
        store.clone(),
        h.shared_admission(),
        TierLimits::default(),
    )
    .expect("protocol");
    let endpoint = bind(&h.key_b, None, vec![TIER_ALPN.to_vec()])
        .await
        .expect("bind B");
    let addr = loopback(&endpoint);
    let router = Router::builder(endpoint)
        .accept(TIER_ALPN, protocol)
        .spawn();
    ServingB {
        router,
        addr,
        store,
        checkout: Checkout::new(),
    }
}

async fn connect_a(h: &Household, b: &ServingB) -> (Endpoint, Connection) {
    let a = bind(&h.key_a, None, vec![]).await.expect("bind A");
    let connection = tokio::time::timeout(WITHIN, a.connect(b.addr.clone(), TIER_ALPN))
        .await
        .expect("connects in time")
        .expect("connects");
    (a, connection)
}

#[tokio::test(flavor = "multi_thread")]
async fn fold_is_idempotent_by_event_cid() {
    let h = household();
    let b = serve_b(&h).await;
    b.checkout
        .append(berth_event("claim", "dev-integration", "sb", "1.0"));
    b.checkout.append(berth_event("moor", "-", "sb", "0.5"));
    let first = b.project(&allow());
    let (a, connection) = connect_a(&h, &b).await;
    let a_checkout = Checkout::new();
    let limits = TreeLimits::default();

    let known = folded_from(&a_checkout.store(), &h.b.did).expect("known");
    let fetched = fetch_channel(
        &connection,
        None,
        first.root(),
        &known,
        &limits,
        &FoldLimits::default(),
    )
    .await
    .expect("fetches");
    assert!(fetched.complete);
    assert_eq!(fetched.events.len(), 2);
    let outcome =
        fold_events(&a_checkout.store(), &h.b.did, &fetched.events, &allow()).expect("folds");
    assert_eq!(
        (outcome.appended, outcome.already, outcome.refused),
        (2, 0, 0)
    );

    // Folding the same events again appends nothing.
    let again =
        fold_events(&a_checkout.store(), &h.b.did, &fetched.events, &allow()).expect("folds");
    assert_eq!((again.appended, again.already), (0, 2));
    assert_eq!(a_checkout.events().len(), 2);

    // A refetch of the same root skips every event already folded: nothing is read again.
    let known = folded_from(&a_checkout.store(), &h.b.did).expect("known");
    assert_eq!(known.len(), 2);
    let refetch = fetch_channel(
        &connection,
        None,
        first.root(),
        &known,
        &limits,
        &FoldLimits::default(),
    )
    .await
    .expect("fetches");
    assert!(refetch.events.is_empty());

    // A newer root is diffed against the last one folded: only the new event crosses.
    b.checkout
        .append(berth_event("release", "dev-integration", "sb", "2.0"));
    let second = b.project(&allow());
    let delta = fetch_channel(
        &connection,
        Some(first.root()),
        second.root(),
        &known,
        &limits,
        &FoldLimits::default(),
    )
    .await
    .expect("fetches");
    assert_eq!(delta.events.len(), 1);
    let outcome =
        fold_events(&a_checkout.store(), &h.b.did, &delta.events, &allow()).expect("folds");
    assert_eq!(outcome.appended, 1);
    assert_eq!(a_checkout.events().len(), 3);

    drop(connection);
    a.close().await;
    b.router.shutdown().await.expect("shutdown");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_second_fold_uses_the_held_cid_index_without_a_store_parse() {
    let h = household();
    let b = serve_b(&h).await;
    b.checkout
        .append(berth_event("claim", "dev-integration", "sb", "1.0"));
    let first = b.project(&allow());
    let a = bind(&h.key_a, None, vec![]).await.expect("bind A");
    let a_checkout = Checkout::new();
    let state = tempfile::tempdir().expect("state");
    let first_report = fold_root_planned(
        &a,
        b.addr.clone(),
        FoldTarget {
            device: &h.b.did,
            since: None,
            root: first.root(),
            admission: None,
            reader: FoldReader {
                device: &h.a.did,
                state_dir: state.path(),
            },
        },
        &a_checkout.store(),
        &allow(),
        &TreeLimits::default(),
        FoldPlan::default(),
    )
    .await
    .expect("first fold");
    assert_eq!(first_report.store_parses, 1);

    b.checkout
        .append(berth_event("release", "dev-integration", "sb", "2.0"));
    let second = b.project(&allow());
    let second_report = fold_root_planned(
        &a,
        b.addr.clone(),
        FoldTarget {
            device: &h.b.did,
            since: Some(first.root()),
            root: second.root(),
            admission: None,
            reader: FoldReader {
                device: &h.a.did,
                state_dir: state.path(),
            },
        },
        &a_checkout.store(),
        &allow(),
        &TreeLimits::default(),
        FoldPlan {
            limits: FoldLimits::default(),
            known: Some(&first_report.known),
        },
    )
    .await
    .expect("second fold");
    assert_eq!(second_report.store_parses, 0);
    assert_eq!(second_report.outcome.appended, 1);

    a.close().await;
    b.router.shutdown().await.expect("shutdown");
}

#[tokio::test(flavor = "multi_thread")]
async fn fold_records_via_device_provenance() {
    let h = household();
    let b = serve_b(&h).await;
    let claim_cid = b
        .checkout
        .append(berth_event("claim", "alpha-fleet", "sb", "1.0"));
    let tree = b.project(&allow());
    let (a, connection) = connect_a(&h, &b).await;
    let a_checkout = Checkout::new();

    let fetched = fetch_channel(
        &connection,
        None,
        tree.root(),
        &Default::default(),
        &TreeLimits::default(),
        &FoldLimits::default(),
    )
    .await
    .expect("fetches");
    assert_eq!(fetched.events[0].cid, claim_cid);
    fold_events(&a_checkout.store(), &h.b.did, &fetched.events, &allow()).expect("folds");

    let (_, folded) = a_checkout.events().pop().expect("one folded event");
    assert_eq!(via_device(&folded), Some(h.b.did.as_str()));
    // The authoring record is kept whole: provider, actor claim and every berth field.
    let original = &fetched.events[0].event;
    assert_eq!(folded.provider, original.provider);
    assert!(folded
        .classified_as
        .iter()
        .any(|slot| slot == "actor-claim:bafyreiexampleactorclaim"));
    assert!(folded.classified_as.starts_with(&original.classified_as));
    assert_eq!(folded.classified_as.len(), original.classified_as.len() + 1);
    // It is known as B's event by B's CID.
    let known = folded_from(&a_checkout.store(), &h.b.did).expect("known");
    assert!(known.contains(&claim_cid));
    assert!(folded_from(&a_checkout.store(), &h.a.did)
        .expect("known")
        .is_empty());
    // A's own channel never carries what it folded from B.
    let a_tree = project_channel(&a_checkout.store(), &allow()).expect("projects");
    assert_eq!(a_tree.events(), 0);

    drop(connection);
    a.close().await;
    b.router.shutdown().await.expect("shutdown");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_fold_refuses_kinds_off_its_own_allow_list() {
    let h = household();
    let b = serve_b(&h).await;
    b.checkout
        .append(berth_event("claim", "cargo", "sb", "1.0"));
    b.checkout
        .append(berth_event("refuse", "cargo", "sb", "2.0"));
    // B serves with a looser list than A folds with.
    let loose = CrossingAllowList::from_policy(&serde_json::json!({"berth": {"channel": {
        "crosses": {"dev-berth-events@1": ["claim", "refuse"]}}}}))
    .expect("policy");
    let tree = b.project(&loose);
    let (a, connection) = connect_a(&h, &b).await;
    let a_checkout = Checkout::new();
    let fetched = fetch_channel(
        &connection,
        None,
        tree.root(),
        &Default::default(),
        &TreeLimits::default(),
        &FoldLimits::default(),
    )
    .await
    .expect("fetches");
    let outcome =
        fold_events(&a_checkout.store(), &h.b.did, &fetched.events, &allow()).expect("folds");
    assert_eq!((outcome.appended, outcome.refused), (1, 1));
    assert_eq!(outcome.refused_cids.len(), 1);

    drop(connection);
    a.close().await;
    b.router.shutdown().await.expect("shutdown");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_cid_outside_the_channel_tree_is_unanswered_over_the_tier() {
    let h = household();
    let b = serve_b(&h).await;
    b.checkout
        .append(berth_event("claim", "cargo", "sb", "1.0"));
    let local = b
        .checkout
        .append(berth_event("refuse", "cargo", "sb", "2.0"));
    let tree = b.project(&allow());
    let a = bind(&h.key_a, None, vec![]).await.expect("bind A");

    // The refused record is in B's own flow store, never in what B serves: neither as a path
    // under the channel root nor as a root of its own.
    let limits = TreeLimits::default();
    let ask = |req: TierRequest| {
        let a = a.clone();
        let addr = b.addr.clone();
        async move {
            tokio::time::timeout(WITHIN, tier_call(&a, addr, req, &limits))
                .await
                .expect("answers in time")
                .expect("answers")
        }
    };
    let under_root = ask(TierRequest::Read {
        root: tree.root().clone(),
        path: TreePath::from_slash_bytes(format!("{CHANNEL_EVENTS}/{local}").as_bytes())
            .expect("path"),
    })
    .await;
    assert!(!matches!(under_root, TierAnswer::Read(_)), "{under_root:?}");
    let as_root = ask(TierRequest::List {
        root: eprfs_core::BlobCid::parse(&local.to_string()).expect("cid"),
        path: TreePath::root(),
    })
    .await;
    assert!(
        matches!(as_root, TierAnswer::Unanswered { .. }),
        "{as_root:?}"
    );

    a.close().await;
    b.router.shutdown().await.expect("shutdown");
}

#[tokio::test(flavor = "multi_thread")]
async fn an_oversized_event_is_quarantined_but_an_exhausted_budget_is_incomplete() {
    let h = household();
    let b = serve_b(&h).await;
    b.checkout
        .append(berth_event("claim", "cargo", "sb", "1.0"));
    b.checkout
        .append(berth_event("release", "cargo", "sb", "1.5"));
    // An admitted device's record far past any berth record's size.
    let mut huge = berth_event("note", "-", "sb", "2.0");
    huge.classified_as
        .push(format!("env:text={}", "x".repeat(70 * 1024)));
    let huge = b.checkout.append(huge);
    let tree = b.project(&allow());
    let (a, connection) = connect_a(&h, &b).await;
    let limits = TreeLimits::default();

    let fetched = fetch_channel(
        &connection,
        None,
        tree.root(),
        &Default::default(),
        &limits,
        &FoldLimits::default(),
    )
    .await
    .expect("fetches");
    assert!(
        fetched.complete,
        "fetching continues past an oversized event"
    );
    assert_eq!(fetched.oversized, 1);
    assert_eq!(fetched.quarantined.len(), 1);
    assert_eq!(fetched.quarantined[0].reason, "unread-oversize");
    assert!(fetched.events.len() <= 2);
    assert!(fetched.events.iter().all(|e| e.cid != huge));

    // A budget smaller than one event stops the fold after the first small one.
    let tight = FoldLimits {
        max_fold_bytes: 16,
        ..FoldLimits::default()
    };
    let skip_huge: std::collections::HashSet<cid::Cid> = [huge].into_iter().collect();
    let fetched = fetch_channel(&connection, None, tree.root(), &skip_huge, &limits, &tight)
        .await
        .expect("fetches");
    assert!(!fetched.complete);
    assert!(fetched.over_budget);
    assert_eq!(
        fetched.events.len(),
        1,
        "the event that crossed the budget is kept, no more"
    );

    drop(connection);
    a.close().await;
    b.router.shutdown().await.expect("shutdown");
}

#[tokio::test(flavor = "multi_thread")]
async fn fold_quarantine_and_nack() {
    for reads_nacks in [true, false] {
        let h = household();
        let mut moor = berth_event("moor", "-", "sb", "0.5");
        if reads_nacks {
            moor.classified_as.push("env:reads=nacks/1".into());
        }
        let moor_cid = atom_cid(&moor).expect("moor cid");

        let mut note = berth_event("note", "-", "sb", "1.0");
        note.classified_as
            .push(format!("env:text={}", "n".repeat(100 * 1024)));
        let note_cid = atom_cid(&note).expect("note cid");
        let note_bytes = canonical_bytes(&note).expect("note bytes");

        let mut claim = berth_event("claim", "cargo", "sb", "2.0");
        claim
            .classified_as
            .push(format!("env:text={}", "c".repeat(100 * 1024)));
        let claim_cid = atom_cid(&claim).expect("claim cid");
        let claim_bytes = canonical_bytes(&claim).expect("claim bytes");

        let (low, high) = if note_cid.to_string() < claim_cid.to_string() {
            (note_cid.to_string(), claim_cid.to_string())
        } else {
            (claim_cid.to_string(), note_cid.to_string())
        };
        let (newer_cid, newer_bytes) = (0u64..)
            .find_map(|nonce| {
                let bytes = serde_ipld_dagcbor::to_vec(&serde_json::json!({
                    "schema": 2,
                    "nonce": nonce,
                    "newRequiredField": true
                }))
                .expect("newer event bytes");
                let digest = BlobCid::compute_raw(&bytes).as_cid().hash().to_owned();
                let cid = Cid::new_v1(0x71, digest);
                (low < cid.to_string() && cid.to_string() < high).then_some((cid, bytes))
            })
            .expect("a newer-schema CID between the two oversized leaves");

        let (root, raw) = raw_channel(vec![
            (moor_cid, canonical_bytes(&moor).expect("moor bytes")),
            (note_cid, note_bytes),
            (newer_cid, newer_bytes),
            (claim_cid, claim_bytes),
        ]);
        let protocol = TierProtocol::new(
            channel_tier_id(),
            Arc::new(raw),
            h.shared_admission(),
            TierLimits::default(),
        )
        .expect("protocol");
        let endpoint_b = bind(&h.key_b, None, vec![TIER_ALPN.to_vec()])
            .await
            .expect("bind B");
        let addr_b = loopback(&endpoint_b);
        let router = Router::builder(endpoint_b)
            .accept(TIER_ALPN, protocol)
            .spawn();
        let endpoint_a = bind(&h.key_a, None, vec![]).await.expect("bind A");
        let checkout = Checkout::new();
        fold_events(
            &checkout.store(),
            &h.b.did,
            &[ChannelEvent {
                cid: moor_cid,
                event: moor,
            }],
            &allow(),
        )
        .expect("folds moor first");
        let state = tempfile::tempdir().expect("state dir");
        let report = fold_root(
            &endpoint_a,
            addr_b,
            FoldTarget {
                device: &h.b.did,
                since: None,
                root: &root,
                admission: None,
                reader: FoldReader {
                    device: &h.a.did,
                    state_dir: state.path(),
                },
            },
            &checkout.store(),
            &allow(),
            &TreeLimits::default(),
            &FoldLimits::default(),
        )
        .await
        .expect("faults are quarantined, not returned as a fold error");
        assert_eq!(report.quarantined.len(), 3);
        assert_eq!(report.complete, reads_nacks);
        assert_eq!(report.outcome.appended, 0);
        let quarantine = std::fs::read_to_string(
            state
                .path()
                .join("quarantine")
                .join(format!("{}.jsonl", h.b.did)),
        )
        .expect("private quarantine ledger");
        assert_eq!(quarantine.lines().count(), 3);
        assert_eq!(quarantine.matches("\"bytes\"").count(), 1);

        endpoint_a.close().await;
        router.shutdown().await.expect("shutdown");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_fold_appends_nothing_once_its_device_is_no_longer_admitted() {
    let h = household();
    let b = serve_b(&h).await;
    b.checkout
        .append(berth_event("claim", "cargo", "sb", "1.0"));
    let tree = b.project(&allow());
    let a = bind(&h.key_a, None, vec![]).await.expect("bind A");
    let a_checkout = Checkout::new();
    let state = tempfile::tempdir().expect("state");

    // Admission as A holds it after folding a Contest of B's binding.
    let mut rows = h.rows.clone();
    rows.push(support::contest(&h.a, &h.rows[1]));
    let revoked: eprfs_peer::SharedAdmission = std::sync::Arc::new(std::sync::RwLock::new(
        eprfs_peer::Admission::from_roster(&rows, &h.a.did, &*h.verify),
    ));
    let target = eprfs_peer::FoldTarget {
        device: &h.b.did,
        since: None,
        root: tree.root(),
        admission: Some(&revoked),
        reader: FoldReader {
            device: &h.a.did,
            state_dir: state.path(),
        },
    };
    let folded = eprfs_peer::fold_root(
        &a,
        b.addr.clone(),
        target,
        &a_checkout.store(),
        &allow(),
        &TreeLimits::default(),
        &FoldLimits::default(),
    )
    .await;
    assert!(
        matches!(folded, Err(eprfs_peer::PeerError::FoldRevoked { .. })),
        "{folded:?}"
    );
    assert!(a_checkout.events().is_empty(), "nothing is appended");

    // The same fold under an admission that still admits B appends B's claim.
    let admitted = h.shared_admission();
    let target = eprfs_peer::FoldTarget {
        device: &h.b.did,
        since: None,
        root: tree.root(),
        admission: Some(&admitted),
        reader: FoldReader {
            device: &h.a.did,
            state_dir: state.path(),
        },
    };
    let report = eprfs_peer::fold_root(
        &a,
        b.addr.clone(),
        target,
        &a_checkout.store(),
        &allow(),
        &TreeLimits::default(),
        &FoldLimits::default(),
    )
    .await
    .expect("folds");
    assert_eq!(report.outcome.appended, 1);

    a.close().await;
    b.router.shutdown().await.expect("shutdown");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_listing_whose_cid_is_not_the_requested_root_is_refused() {
    let h = household();
    let foreign = test_channel(
        vec![berth_event("claim", "foreign-root", "sb", "31.0")],
        vec![],
    );
    let signed_root = BlobCid::compute(b"the root named by the signed head");
    assert_ne!(foreign.root, signed_root);
    let requests = Arc::new(Mutex::new(Vec::new()));
    let served = foreign.clone();
    let mounted = mount_stub(
        &h.key_b,
        StubTier {
            answer: Arc::new(move |request| served.answer(request)),
            requests,
        },
    )
    .await;
    let a = bind(&h.key_a, None, vec![]).await.expect("bind A");
    let connection = a
        .connect(mounted.addr.clone(), TIER_ALPN)
        .await
        .expect("connects");

    let fetched = fetch_channel(
        &connection,
        None,
        &signed_root,
        &Default::default(),
        &TreeLimits::default(),
        &FoldLimits::default(),
    )
    .await;
    assert!(matches!(fetched, Err(PeerError::FoldIntegrity { .. })));

    drop(connection);
    a.close().await;
    mounted.router.shutdown().await.expect("shutdown");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_self_consistent_tree_other_than_the_signed_root_is_refused_and_the_head_is_not_recorded()
{
    let h = household();
    let foreign = test_channel(
        vec![berth_event("claim", "foreign-head", "sb", "32.0")],
        vec![],
    );
    let signed_root = BlobCid::compute(b"signed but not served");
    let served = foreign.clone();
    let mounted = mount_gossip_stub(
        &h.key_b,
        StubTier {
            answer: Arc::new(move |request| served.answer(request)),
            requests: Arc::new(Mutex::new(Vec::new())),
        },
    )
    .await;
    let roster_root = Checkout::new();
    let roster = SidecarRoster::open(roster_root.dir.path(), HANDLE).expect("roster");
    for row in &h.rows {
        roster.append(row.clone()).expect("roster row");
    }
    let checkout = Checkout::new();
    let state = tempfile::tempdir().expect("state");
    let signer = Arc::new(h.a.signer());
    let daemon = PeerDaemon::start(PeerConfig {
        key: h.key_a,
        device: h.a.did.clone(),
        chain_root: h.a.did.clone(),
        flow_root: checkout.dir.path().to_path_buf(),
        roster: roster.log_path().to_path_buf(),
        state_dir: state.path().to_path_buf(),
        rules: Rules::from_policy(&policy()).expect("rules"),
        policy: None,
        relay: None,
        listen: None,
        peers: vec![mounted.addr.clone()],
        signer,
        verifier: h.verify.clone(),
        log: Arc::new(|_line: &str| {}),
    })
    .await
    .expect("A daemon starts");
    mounted
        .endpoint
        .add_node_addr(
            NodeAddr::new(daemon.node_id()).with_direct_addresses(daemon.bound_sockets()),
        )
        .expect("A address");
    let (sender, mut receiver) = mounted
        .gossip
        .subscribe(eprfs_peer::topic_for(&h.a.did), vec![daemon.node_id()])
        .await
        .expect("B joins")
        .split();
    let joined = tokio::time::timeout(WITHIN, async {
        loop {
            match receiver.next().await {
                Some(Ok(iroh_gossip::api::Event::NeighborUp(node))) if node == daemon.node_id() => {
                    break true;
                }
                Some(_) => {}
                None => break false,
            }
        }
    })
    .await
    .expect("B sees A as a gossip neighbour");
    assert!(joined, "B's gossip stream stays open after joining A");
    let head = ChannelHead::sign(
        h.b.did.clone(),
        signed_root,
        1,
        1,
        Default::default(),
        &h.b.signer(),
    )
    .expect("head signs");
    sender
        .broadcast(head.to_wire().expect("head wire").into())
        .await
        .expect("head broadcasts");
    tokio::time::timeout(WITHIN, async {
        loop {
            let table = HeadTable::load(state.path()).expect("head table");
            if table.channel.fold_failures > 0 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("the rejected fold is persisted");
    assert!(
        checkout.events().is_empty(),
        "no foreign event was appended"
    );
    let table = HeadTable::load(state.path()).expect("head table");
    assert!(
        !table.devices.contains_key(&h.b.did),
        "a failed fold leaves the peer head unrecorded"
    );

    daemon.shutdown().await;
    mounted.router.shutdown().await.expect("shutdown");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_diff_with_an_invalid_blob_codec_falls_back_to_the_verified_listing() {
    let h = household();
    let first_event = berth_event("claim", "diff-base", "sb", "33.0");
    let second_event = berth_event("release", "diff-base", "sb", "34.0");
    let stranger_event = berth_event("claim", "diff-stranger", "sb", "35.0");
    let first_cid = atom_cid(&first_event).expect("first cid");
    let second_cid = atom_cid(&second_event).expect("second cid");
    let stranger_cid = atom_cid(&stranger_event).expect("stranger cid");
    let old = test_channel(vec![first_event.clone()], vec![]);
    let new = test_channel(vec![first_event, second_event], vec![]);
    let change = TreeChange::new(
        TreePath::from_slash_bytes(format!("{CHANNEL_EVENTS}/{stranger_cid}").as_bytes())
            .expect("stranger path"),
        None,
        Some(TreeEntryKind::File {
            blob: old.root.clone().into(),
            executable: false,
        }),
    );
    let served_old = old.clone();
    let served_new = new.clone();
    let old_root = old.root.clone();
    let new_root = new.root.clone();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let mounted = mount_stub(
        &h.key_b,
        StubTier {
            answer: Arc::new(move |request| match request {
                TierRequest::Diff { old, new, .. } if old == &old_root && new == &new_root => {
                    TierAnswer::Diff(vec![change.clone()])
                }
                TierRequest::List { root, .. } | TierRequest::Read { root, .. }
                    if root == &old_root =>
                {
                    served_old.answer(request)
                }
                TierRequest::List { root, .. } | TierRequest::Read { root, .. }
                    if root == &new_root =>
                {
                    served_new.answer(request)
                }
                _ => TierAnswer::Failed {
                    reason: "unexpected request".into(),
                },
            }),
            requests: requests.clone(),
        },
    )
    .await;
    let a = bind(&h.key_a, None, vec![]).await.expect("bind A");
    let connection = a
        .connect(mounted.addr.clone(), TIER_ALPN)
        .await
        .expect("connects");
    fetch_channel(
        &connection,
        None,
        &old.root,
        &Default::default(),
        &TreeLimits::default(),
        &FoldLimits::default(),
    )
    .await
    .expect("base root verifies");

    let known = HashSet::from([first_cid]);
    let fetched = fetch_channel(
        &connection,
        Some(&old.root),
        &new.root,
        &known,
        &TreeLimits::default(),
        &FoldLimits::default(),
    )
    .await
    .expect("the verified full listing replaces the refused diff");
    assert_eq!(fetched.events.len(), 1);
    assert_eq!(fetched.events[0].cid, second_cid);
    assert!(fetched.events.iter().all(|event| event.cid != stranger_cid));
    assert!(requests.lock().expect("request log").iter().any(|request| {
        matches!(request, TierRequest::List { root, path }
            if root == &new.root && path.to_string() == CHANNEL_EVENTS)
    }));

    drop(connection);
    a.close().await;
    mounted.router.shutdown().await.expect("shutdown");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_diff_adding_an_event_not_in_the_new_events_node_is_refused() {
    let h = household();
    let first_event = berth_event("claim", "foreign-diff-base", "sb", "33.0");
    let second_event = berth_event("release", "foreign-diff-base", "sb", "34.0");
    let stranger_event = berth_event("claim", "foreign-diff-stranger", "sb", "35.0");
    let first_cid = atom_cid(&first_event).expect("first cid");
    let second_cid = atom_cid(&second_event).expect("second cid");
    let stranger_cid = atom_cid(&stranger_event).expect("stranger cid");
    let old = test_channel(vec![first_event.clone()], vec![]);
    let new = test_channel(vec![first_event, second_event], vec![]);
    let stranger = test_channel(vec![stranger_event], vec![]);
    let stranger_path = format!("{CHANNEL_EVENTS}/{stranger_cid}");
    let change = TreeChange::new(
        TreePath::from_slash_bytes(stranger_path.as_bytes()).expect("stranger path"),
        None,
        Some(stranger.event_kind(&stranger_cid)),
    );
    let served_old = old.clone();
    let served_new = new.clone();
    let served_stranger = stranger.clone();
    let old_root = old.root.clone();
    let new_root = new.root.clone();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let mounted = mount_stub(
        &h.key_b,
        StubTier {
            answer: Arc::new(move |request| match request {
                TierRequest::Diff { old, new, .. } if old == &old_root && new == &new_root => {
                    TierAnswer::Diff(vec![change.clone()])
                }
                TierRequest::Read { root, path }
                    if root == &new_root && path.to_string() == stranger_path =>
                {
                    served_stranger.answer(request)
                }
                TierRequest::List { root, .. } | TierRequest::Read { root, .. }
                    if root == &old_root =>
                {
                    served_old.answer(request)
                }
                TierRequest::List { root, .. } | TierRequest::Read { root, .. }
                    if root == &new_root =>
                {
                    served_new.answer(request)
                }
                _ => TierAnswer::Failed {
                    reason: "unexpected request".into(),
                },
            }),
            requests: requests.clone(),
        },
    )
    .await;
    let a = bind(&h.key_a, None, vec![]).await.expect("bind A");
    let connection = a
        .connect(mounted.addr.clone(), TIER_ALPN)
        .await
        .expect("connects");
    fetch_channel(
        &connection,
        None,
        &old.root,
        &Default::default(),
        &TreeLimits::default(),
        &FoldLimits::default(),
    )
    .await
    .expect("base root verifies");

    let known = HashSet::from([first_cid]);
    let fetched = fetch_channel(
        &connection,
        Some(&old.root),
        &new.root,
        &known,
        &TreeLimits::default(),
        &FoldLimits::default(),
    )
    .await
    .expect("the verified full listing replaces the foreign diff");
    assert_eq!(fetched.events.len(), 1);
    assert_eq!(fetched.events[0].cid, second_cid);
    assert!(fetched.events.iter().all(|event| event.cid != stranger_cid));
    assert!(requests.lock().expect("request log").iter().any(|request| {
        matches!(request, TierRequest::List { root, path }
            if root == &new.root && path.to_string() == CHANNEL_EVENTS)
    }));

    drop(connection);
    a.close().await;
    mounted.router.shutdown().await.expect("shutdown");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_diff_that_recomputes_to_the_verified_events_cid_reads_no_full_listing() {
    let h = household();
    let first_event = berth_event("claim", "verified-diff", "sb", "36.0");
    let second_event = berth_event("release", "verified-diff", "sb", "37.0");
    let second_cid = atom_cid(&second_event).expect("second cid");
    let old = test_channel(vec![first_event.clone()], vec![]);
    let new = test_channel(vec![first_event, second_event], vec![]);
    let change = TreeChange::new(
        TreePath::from_slash_bytes(format!("{CHANNEL_EVENTS}/{second_cid}").as_bytes())
            .expect("second path"),
        None,
        Some(new.event_kind(&second_cid)),
    );
    let requests = Arc::new(Mutex::new(Vec::new()));
    let served_old = old.clone();
    let served_new = new.clone();
    let old_root = old.root.clone();
    let new_root = new.root.clone();
    let mounted = mount_stub(
        &h.key_b,
        StubTier {
            answer: Arc::new(move |request| match request {
                TierRequest::Diff { old, new, .. } if old == &old_root && new == &new_root => {
                    TierAnswer::Diff(vec![change.clone()])
                }
                TierRequest::List { root, .. } | TierRequest::Read { root, .. }
                    if root == &old_root =>
                {
                    served_old.answer(request)
                }
                TierRequest::List { root, .. } | TierRequest::Read { root, .. }
                    if root == &new_root =>
                {
                    served_new.answer(request)
                }
                _ => TierAnswer::Failed {
                    reason: "unexpected request".into(),
                },
            }),
            requests: requests.clone(),
        },
    )
    .await;
    let a = bind(&h.key_a, None, vec![]).await.expect("bind A");
    let connection = a
        .connect(mounted.addr.clone(), TIER_ALPN)
        .await
        .expect("connects");
    fetch_channel(
        &connection,
        None,
        &old.root,
        &Default::default(),
        &TreeLimits::default(),
        &FoldLimits::default(),
    )
    .await
    .expect("base root verifies");
    let before = requests.lock().expect("request log").len();

    let fetched = fetch_channel(
        &connection,
        Some(&old.root),
        &new.root,
        &Default::default(),
        &TreeLimits::default(),
        &FoldLimits::default(),
    )
    .await
    .expect("verified diff");
    assert_eq!(fetched.events.len(), 1);
    assert_eq!(fetched.events[0].cid, second_cid);
    {
        let requests = requests.lock().expect("request log");
        let delta_requests = &requests[before..];
        assert!(delta_requests
            .iter()
            .any(|request| matches!(request, TierRequest::Diff { .. })));
        assert!(
            !delta_requests.iter().any(|request| {
                matches!(request, TierRequest::List { path, .. } if path.to_string() == CHANNEL_EVENTS)
            }),
            "a matching diff does not read the full events listing"
        );
    }

    drop(connection);
    a.close().await;
    mounted.router.shutdown().await.expect("shutdown");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_nack_leaf_not_under_the_root_is_refused() {
    let h = household();
    let under_root = NackLeaf {
        refused: vec!["under-root".into()],
        quarantined: vec![],
        refused_total: 1,
    };
    let foreign = NackLeaf {
        refused: vec!["foreign".into()],
        quarantined: vec![],
        refused_total: 1,
    };
    let tree = test_channel(vec![], vec![(h.a.did.clone(), under_root)]);
    let foreign_tree = test_channel(vec![], vec![(h.a.did.clone(), foreign)]);
    let nack_path = format!("channel/nacks/{}", h.a.did);
    let foreign_read = foreign_tree.read(&nack_path);
    let served = tree.clone();
    let requested_nack = nack_path.clone();
    let mounted = mount_stub(
        &h.key_b,
        StubTier {
            answer: Arc::new(move |request| match request {
                TierRequest::Read { path, .. } if path.to_string() == requested_nack => {
                    TierAnswer::Read(foreign_read.clone())
                }
                _ => served.answer(request),
            }),
            requests: Arc::new(Mutex::new(Vec::new())),
        },
    )
    .await;
    let a = bind(&h.key_a, None, vec![]).await.expect("bind A");
    let checkout = Checkout::new();
    let state = tempfile::tempdir().expect("state");

    let folded = fold_root(
        &a,
        mounted.addr.clone(),
        FoldTarget {
            device: &h.b.did,
            since: None,
            root: &tree.root,
            admission: None,
            reader: FoldReader {
                device: &h.a.did,
                state_dir: state.path(),
            },
        },
        &checkout.store(),
        &allow(),
        &TreeLimits::default(),
        &FoldLimits::default(),
    )
    .await;
    assert!(matches!(folded, Err(PeerError::FoldIntegrity { .. })));

    a.close().await;
    mounted.router.shutdown().await.expect("shutdown");
}

#[tokio::test(flavor = "multi_thread")]
async fn an_unmodified_v1_peer_still_folds_and_receives_no_new_request_kind() {
    let h = household();
    let event = berth_event("claim", "v1-peer", "sb", "38.0");
    let cid = atom_cid(&event).expect("event cid");
    let tree = test_channel(vec![event], vec![]);
    let served = tree.clone();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let mounted = mount_stub(
        &h.key_b,
        StubTier {
            answer: Arc::new(move |request| served.answer(request)),
            requests: requests.clone(),
        },
    )
    .await;
    let a = bind(&h.key_a, None, vec![]).await.expect("bind A");
    let connection = a
        .connect(mounted.addr.clone(), TIER_ALPN)
        .await
        .expect("connects");
    let checkout = Checkout::new();

    let fetched = fetch_channel(
        &connection,
        None,
        &tree.root,
        &Default::default(),
        &TreeLimits::default(),
        &FoldLimits::default(),
    )
    .await
    .expect("v1 requests are sufficient");
    let outcome = fold_events(&checkout.store(), &h.b.did, &fetched.events, &allow())
        .expect("v1 event folds");
    assert_eq!(outcome.appended, 1);
    assert_eq!(fetched.events[0].cid, cid);
    assert!(requests.lock().expect("request log").iter().all(|request| {
        matches!(
            request,
            TierRequest::Read { .. } | TierRequest::List { .. } | TierRequest::Diff { .. }
        )
    }));

    drop(connection);
    a.close().await;
    mounted.router.shutdown().await.expect("shutdown");
}
