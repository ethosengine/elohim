//! The fold over real iroh endpoints: device A asks device B's tier for B's channel tree and
//! appends B's crossing events to its own flow store, `via` B, once (spec §3.3).

mod support;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use cid::Cid;
use elohim_epr_rea::{atom_cid, model::canonical_bytes};
use eprfs_core::{
    encode_tree, AttestationDraft, BlobCid, BlobHandle, BlobPresence, EprRecord, EprRef,
    EprfsError, EprfsStorage, FetchPolicy, TreeEntry, TreeEntryKind, TreeLimits, TreeNode,
    TreePath,
};
use eprfs_peer::{
    bind, channel_tier_id, fetch_channel, fold_events, fold_root, folded_from, project_channel,
    tier_call, via_device, ChannelEvent, ChannelStore, ChannelTree, CrossingAllowList, FoldLimits,
    FoldReader, FoldTarget, TierAnswer, TierLimits, TierProtocol, TierRequest, CHANNEL_EVENTS,
    TIER_ALPN,
};
use iroh::endpoint::Connection;
use iroh::protocol::Router;
use iroh::{Endpoint, NodeAddr};
use support::{berth_event, household, loopback, policy, Checkout, Household};

const WITHIN: Duration = Duration::from_secs(20);

#[derive(Debug)]
struct RawTreeStore {
    blobs: HashMap<BlobCid, Bytes>,
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
