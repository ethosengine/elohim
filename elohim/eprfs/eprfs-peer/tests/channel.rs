//! The channel tree: a device projects only the crossing allow-list's events into
//! `channel/events/<event-cid>` (spec §3.3; review focus item 5).

mod support;

use std::collections::{BTreeMap, BTreeSet};

use eprfs_core::{list_dir, read_path, BatchOptions, TreeLimits, TreePath};
use eprfs_peer::{
    project_channel, project_channel_with_nacks, ChannelStore, CrossingAllowList, NackLeaf,
    PeerError, CHANNEL_EVENTS, CHANNEL_NACKS,
};
use support::{berth_event, event_with, policy, Checkout, MEASURE};

fn tp(path: &str) -> TreePath {
    TreePath::from_slash_bytes(path.as_bytes()).expect("path")
}

fn names(listing: &eprfs_core::DirListing) -> BTreeSet<String> {
    listing
        .entries
        .iter()
        .map(|e| String::from_utf8(e.name.clone()).expect("utf8 name"))
        .collect()
}

#[tokio::test]
async fn channel_tree_holds_only_allowed_kinds() {
    let checkout = Checkout::new();
    let moor = checkout.append(berth_event("moor", "cargo", "s1", "1.0"));
    let claim = checkout.append(berth_event("claim", "dev-integration", "s1", "2.0"));
    let say = checkout.append(berth_event("note", "-", "s1", "3.0"));
    // Berth kinds the spec does not list stay local.
    let refuse = checkout.append(berth_event("refuse", "cargo", "s2", "4.0"));
    let overrun = checkout.append(berth_event("overrun", "cargo", "s2", "5.0"));
    // Another measure's events never cross, whatever their kind: recall, memory, anything else.
    let recall = checkout.append(event_with("recall-session@1", "claim", vec![]));
    let memory = checkout.append(event_with("memory-dump@1", "note", vec![]));
    // An event folded from another device never re-crosses from this one.
    let mut folded = berth_event("claim", "alpha-fleet", "s9", "6.0");
    folded.classified_as.push("via:did:key:zOther".into());
    let folded = checkout.append(folded);
    // An event naming the measure twice, or no kind, is not a berth record.
    let mut doubled = berth_event("claim", "cargo", "s3", "7.0");
    doubled.classified_as.push(format!("measure:{MEASURE}"));
    let doubled = checkout.append(doubled);

    let allow = CrossingAllowList::from_policy(&policy()).expect("policy");
    let tree = project_channel(&checkout.store(), &allow).expect("projects");
    assert_eq!(tree.events(), 3);

    let store = ChannelStore::default();
    store.install(&tree);
    let opts = BatchOptions::default();
    let limits = TreeLimits::default();
    let listing = list_dir(&store, tree.root(), &tp(CHANNEL_EVENTS), &opts, &limits)
        .await
        .expect("lists channel/events");
    let crossed = names(&listing);
    let expected: BTreeSet<String> = [moor, claim, say].iter().map(|c| c.to_string()).collect();
    assert_eq!(crossed, expected);
    for local in [refuse, overrun, recall, memory, folded, doubled] {
        assert!(!crossed.contains(&local.to_string()), "{local} stays local");
    }

    // Each leaf is the event's canonical dag-cbor bytes, addressed by the event's own CID.
    let (_, event) = checkout
        .events()
        .into_iter()
        .find(|(cid, _)| *cid == claim)
        .expect("claim");
    let leaf = read_path(
        &store,
        tree.root(),
        &tp(&format!("{CHANNEL_EVENTS}/{claim}")),
        &opts,
        &limits,
    )
    .await
    .expect("reads the claim");
    assert_eq!(
        leaf.bytes.as_ref(),
        elohim_epr_rea::model::canonical_bytes(&event)
            .expect("bytes")
            .as_slice()
    );
    // Leaves are raw-codec blobs; the digest is the event CID's.
    assert_eq!(leaf.cid.as_cid().hash(), claim.hash());
}

#[tokio::test]
async fn an_empty_allow_list_projects_an_empty_channel() {
    let checkout = Checkout::new();
    checkout.append(berth_event("claim", "cargo", "s1", "1.0"));
    // Default deny: a policy with no `berth.channel.crosses` lets nothing cross.
    let allow = CrossingAllowList::from_policy(&serde_json::json!({"berth": {}})).expect("policy");
    let tree = project_channel(&checkout.store(), &allow).expect("projects");
    assert_eq!(tree.events(), 0);
    let store = ChannelStore::default();
    store.install(&tree);
    let listing = list_dir(
        &store,
        tree.root(),
        &tp(CHANNEL_EVENTS),
        &BatchOptions::default(),
        &TreeLimits::default(),
    )
    .await
    .expect("the empty events directory is still there");
    assert!(listing.entries.is_empty());
}

#[tokio::test]
async fn nack_leaves_are_additive_and_move_only_with_the_set() {
    let checkout = Checkout::new();
    let allow = CrossingAllowList::from_policy(&policy()).expect("policy");
    let device = "did:key:zAuthor";
    let nack = NackLeaf {
        refused: vec!["bafy-refused".into()],
        quarantined: vec!["bafy-quarantined".into()],
        refused_total: 2,
    };
    let nacks = BTreeMap::from([(device.to_string(), nack.clone())]);
    let first = project_channel_with_nacks(&checkout.store(), &allow, &nacks).expect("projects");
    let again = project_channel_with_nacks(&checkout.store(), &allow, &nacks).expect("projects");
    assert_eq!(
        first.root(),
        again.root(),
        "the same set keeps the leaf stable"
    );

    let store = ChannelStore::default();
    store.install(&first);
    let leaf = read_path(
        &store,
        first.root(),
        &tp(&format!("{CHANNEL_NACKS}/{device}")),
        &BatchOptions::default(),
        &TreeLimits::default(),
    )
    .await
    .expect("reads nack");
    assert_eq!(
        serde_json::from_slice::<NackLeaf>(&leaf.bytes).expect("nack"),
        nack
    );

    let changed = project_channel_with_nacks(
        &checkout.store(),
        &allow,
        &BTreeMap::from([(
            device.to_string(),
            NackLeaf {
                refused_total: 3,
                ..nack
            },
        )]),
    )
    .expect("projects");
    assert_ne!(first.root(), changed.root());
}

#[test]
fn a_malformed_crossing_list_is_refused_not_widened() {
    for bad in [
        serde_json::json!({"berth": {"channel": {"crosses": ["dev-berth-events@1"]}}}),
        serde_json::json!({"berth": {"channel": {"crosses": {MEASURE: "claim"}}}}),
        serde_json::json!({"berth": {"channel": {"crosses": {MEASURE: [7]}}}}),
    ] {
        assert!(
            matches!(
                CrossingAllowList::from_policy(&bad),
                Err(PeerError::Policy { .. })
            ),
            "{bad}"
        );
    }
}

#[tokio::test]
async fn the_store_keeps_a_bounded_number_of_roots() {
    let checkout = Checkout::new();
    let allow = CrossingAllowList::from_policy(&policy()).expect("policy");
    let store = ChannelStore::new(2);
    let mut roots = Vec::new();
    for i in 0..3 {
        checkout.append(berth_event("claim", "cargo", "s1", &format!("{i}.0")));
        let tree = project_channel(&checkout.store(), &allow).expect("projects");
        store.install(&tree);
        roots.push(tree.root().clone());
    }
    let opts = BatchOptions::default();
    let limits = TreeLimits::default();
    let oldest = list_dir(&store, &roots[0], &tp(CHANNEL_EVENTS), &opts, &limits).await;
    assert!(oldest.is_err(), "the oldest root is no longer held");
    for root in &roots[1..] {
        list_dir(&store, root, &tp(CHANNEL_EVENTS), &opts, &limits)
            .await
            .expect("a retained root lists");
    }
}
