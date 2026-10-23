//! The fold over real iroh endpoints: device A asks device B's tier for B's channel tree and
//! appends B's crossing events to its own flow store, `via` B, once (spec §3.3).

mod support;

use std::sync::Arc;
use std::time::Duration;

use eprfs_core::{TreeLimits, TreePath};
use eprfs_peer::{
    bind, channel_tier_id, fetch_channel, fold_events, folded_from, project_channel, tier_call,
    via_device, ChannelStore, ChannelTree, CrossingAllowList, TierAnswer, TierLimits, TierProtocol,
    TierRequest, CHANNEL_EVENTS, TIER_ALPN,
};
use iroh::endpoint::Connection;
use iroh::protocol::Router;
use iroh::{Endpoint, NodeAddr};
use support::{berth_event, household, loopback, policy, Checkout, Household};

const WITHIN: Duration = Duration::from_secs(20);

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
    let fetched = fetch_channel(&connection, None, first.root(), &known, &limits)
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
    let refetch = fetch_channel(&connection, None, first.root(), &known, &limits)
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
    )
    .await
    .expect("fetches");
    let outcome =
        fold_events(&a_checkout.store(), &h.b.did, &fetched.events, &allow()).expect("folds");
    assert_eq!((outcome.appended, outcome.refused), (1, 1));

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
