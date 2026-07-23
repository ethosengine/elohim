//! Integration tests for the upgraded `distribute_shards` — contract-aware
//! peer selection + placement_gaps recording.
//!
//! These tests require the `p2p` feature because they exercise `P2PHandle`.

#![cfg(feature = "p2p")]

use elohim_storage::db::placement_gaps;
use elohim_storage::test_util::{spawn_p2p_with_peers, test_pool};

/// Regression: selection SUCCESS must not fabricate a green placement.
///
/// A small blob → single "none"-encoded shard → `desired_count == 1`. One
/// ACCEPTING peer makes `PeerSelection` return `Ok` (`gap_kind_opt == None`).
/// But that peer carries no `peer_transport_manifest` / `peer_identity_bindings`
/// row, so `distribute_shards` cannot resolve its `agent_cid` to a dialable
/// libp2p id and SKIPS the push (`distributed == 0`). The old code cleared
/// `placement_gaps` on selection-success alone — an empty gap set beside zero
/// real placements (fabricated green). The fix records a gap whenever the pushes
/// behind a full selection did not land.
#[tokio::test]
async fn distribute_records_gap_when_selection_ok_but_zero_pushed() {
    let pool = test_pool();
    let harness = spawn_p2p_with_peers(
        pool.clone(),
        &[("agent-alpha-1", "home-alpha", "accepting")], // selection succeeds…
    )
    .await;

    // …but the peer has no transport binding, so the push is skipped.
    let blob = vec![7u8; 4096]; // <= single_shard_max → 1 shard, desired_count == 1
    let distributed = harness
        .p2p
        .distribute_shards("content-zero-push", &blob, &pool, "lamad")
        .await
        .unwrap();
    assert_eq!(
        distributed, 0,
        "unresolvable peer → zero shards actually pushed despite selection success"
    );

    let mut conn = pool.get().unwrap();
    let gaps = placement_gaps::list_gaps(
        &mut conn,
        "lamad",
        placement_gaps::GapQuery {
            content_id: Some("content-zero-push".into()),
            ..Default::default()
        },
    )
    .unwrap();

    assert!(
        !gaps.is_empty(),
        "zero pushed shards must leave/record a placement gap, not a fabricated-green empty set"
    );
    // gap_kind must stay within the placement-gap-view schema enum.
    assert!(
        gaps.iter()
            .all(|g| g.achieved_steward_count == 0 && g.gap_kind == "peers-unavailable"),
        "the recorded gap must reflect the zero-push reality: {:?}",
        gaps.iter()
            .map(|g| (&g.gap_kind, g.achieved_steward_count))
            .collect::<Vec<_>>()
    );
}

/// Seeds three peers across two households and calls `distribute_shards`.
///
/// **This test is `#[ignore]`'d.**
///
/// The push-side identity bridge is now fixed (slice 2): `distribute_shards`
/// resolves each selected `agent_cid` → libp2p `PeerId` via
/// `services::transport_resolve` BEFORE dialing, and skips peers with no
/// transport binding. But two things still keep this test from running under the
/// stub harness:
///   1. The stub `P2PHandle` returns `Err("stub: no P2P swarm in test")` from
///      `push_shard`, so `shard_locations` is never populated even for a
///      resolvable peer — the household-diversity assertion cannot be verified.
///   2. The seeded peers carry non-`uhCAk` agent keys and NO `peer_transport_manifest`
///      / `peer_identity_bindings` rows, so the new resolver returns `None` and
///      they are skipped (distributed == 0) by design.
///
/// A live re-enable therefore needs a `spawn_live_p2p_cluster` fixture that (a)
/// runs a real libp2p swarm and (b) seeds transport bindings for the peers so the
/// resolver can dial them.
///
/// TODO(task-17): implement live P2P integration harness + transport-binding seed,
/// then re-enable.
#[tokio::test]
#[ignore = "Requires live libp2p swarm + seeded transport bindings — stub handle cannot push, and unbound peers are (correctly) skipped by the slice-2 resolver. See Task 17."]
async fn distribute_picks_diverse_households() {
    let pool = test_pool();
    let harness = spawn_p2p_with_peers(
        pool.clone(),
        &[
            ("agent-alpha-1", "home-alpha", "accepting"),
            ("agent-alpha-2", "home-alpha", "accepting"),
            ("agent-beta-1", "home-beta", "accepting"),
        ],
    )
    .await;

    let blob = vec![42u8; 4096];
    let distributed = harness
        .p2p
        .distribute_shards("content-x", &blob, &pool, "lamad")
        .await
        .unwrap();
    assert!(
        distributed > 0,
        "expected at least one shard distributed, got {distributed}"
    );

    // Verify shard_locations has ≥ 2 distinct households represented.
    let mut conn = pool.get().unwrap();
    let locations = elohim_storage::db::shard_locations::get_locations_for_content(
        &mut conn,
        "lamad",
        "content-x",
    )
    .unwrap();
    let households: std::collections::HashSet<String> = locations
        .iter()
        .filter_map(|l| {
            elohim_storage::db::humans::get_human_by_agent_key(&mut conn, &l.peer_id)
                .ok()
                .flatten()
                .and_then(|h| h.household_id)
        })
        .collect();
    assert!(
        households.len() >= 2,
        "expected ≥ 2 households in shard_locations, got {households:?}"
    );
}

/// Seeds a peer whose REA commitment exists but is *leaving* (not accepting),
/// so `PeerSelection` returns `Short(peers-unavailable)` regardless of blob
/// size.  `distribute_shards` must then write at least one `placement_gaps`
/// row with `gap_kind = "peers-unavailable"`.
///
/// Using a "leaving" peer rather than an "accepting" peer with a tiny blob
/// ensures the gap path is exercised independently of blob-size / shard-count
/// arithmetic.  This test does NOT require an actual libp2p swarm.
#[tokio::test]
async fn distribute_records_gap_when_households_are_short() {
    let pool = test_pool();
    let harness = spawn_p2p_with_peers(
        pool.clone(),
        &[("agent-alpha-1", "home-alpha", "leaving")], // committed but not accepting
    )
    .await;

    let blob = vec![42u8; 4096];
    // distribute_shards returns Ok even on short placement (gaps recorded separately)
    let _ = harness
        .p2p
        .distribute_shards("content-short", &blob, &pool, "lamad")
        .await
        .unwrap();

    let mut conn = pool.get().unwrap();
    let gaps = placement_gaps::list_gaps(
        &mut conn,
        "lamad",
        placement_gaps::GapQuery {
            content_id: Some("content-short".into()),
            ..Default::default()
        },
    )
    .unwrap();

    assert!(
        !gaps.is_empty(),
        "expected placement_gaps rows for content-short but got none"
    );
    assert!(
        gaps.iter().any(|g| {
            g.gap_kind == "under-committed"
                || g.gap_kind == "contracts-short"
                || g.gap_kind == "peers-unavailable"
        }),
        "expected a recognized gap_kind, got: {:?}",
        gaps.iter().map(|g| &g.gap_kind).collect::<Vec<_>>()
    );
}
