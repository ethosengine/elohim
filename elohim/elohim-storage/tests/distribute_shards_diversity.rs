//! Integration tests for the upgraded `distribute_shards` — contract-aware
//! peer selection + placement_gaps recording.
//!
//! These tests require the `p2p` feature because they exercise `P2PHandle`.

#![cfg(feature = "p2p")]

use elohim_storage::db::placement_gaps;
use elohim_storage::test_util::{spawn_p2p_with_peers, test_pool};

/// Seeds three peers across two households and calls `distribute_shards`.
///
/// **This test is `#[ignore]`'d.**
///
/// With the stub P2PHandle, `push_shard` always returns
/// `Err("stub: no P2P swarm in test")`, so `shard_locations` is never
/// populated and the household-diversity assertion on shard locations cannot
/// be verified.  Full live coverage — including verifying that at least two
/// distinct households receive shards — requires a running libp2p swarm.
///
/// TODO(task-17): implement live P2P integration harness and re-enable this
/// test with a `spawn_live_p2p_cluster` fixture.
#[tokio::test]
#[ignore = "Requires live libp2p swarm — stub handle cannot push shards to shard_locations. See Task 17."]
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
    assert!(distributed > 0, "expected at least one shard distributed, got {distributed}");

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
        &[("agent-alpha-1", "home-alpha", "leaving")],  // committed but not accepting
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
