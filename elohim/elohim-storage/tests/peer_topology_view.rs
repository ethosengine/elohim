//! T26 integration tests for peer_topology_view::aggregate_peer_topology_view
//! and build_local_slice.
//!
//! Multi-peer integration tests that need real federation responders are
//! deferred to Jenkins per project memory `feedback_shift_measure_jenkins`.
//! These tests exercise the aggregator's classification logic against the
//! P2PHandle::for_testing() stub (which always returns TransportError →
//! all peers Offline → AllOffline freshness).

use elohim_storage::db::models::NewPeerIdentityBindingRow;
use elohim_storage::db::peer_identity_bindings;
use elohim_storage::services::federator::Federator;
use elohim_storage::services::peer_topology_view::{
    aggregate_peer_topology_view, build_local_slice,
};
use elohim_storage::test_util::test_pool;
use elohim_storage::views::FreshnessState;
use elohim_storage::P2PHandle;

fn seed_binding(pool: &elohim_storage::db::DbPool, peer_id: &str, agent_cid: &str) {
    let mut conn = pool.get().expect("conn");
    let row = NewPeerIdentityBindingRow {
        peer_id: peer_id.to_string(),
        agent_cid: agent_cid.to_string(),
        dht_anchor_hash: format!("anchor-{peer_id}"),
        valid_from: "2026-01-01T00:00:00Z".to_string(),
        valid_until: None,
        observed_at: "2026-04-24T00:00:00Z".to_string(),
        source: "dht".to_string(),
        device_archetype: "node".to_string(),
        superseded_by: None,
    };
    peer_identity_bindings::upsert(&mut conn, &row).expect("upsert binding");
}

/// Test 1: unknown agent → empty edges, Live freshness, reciprocation_count 0.
#[tokio::test]
async fn peer_topology_no_bindings_returns_empty_live() {
    let pool = test_pool();
    let federator = Federator::new(P2PHandle::for_testing());

    let view = aggregate_peer_topology_view(&pool, &federator, "agent_visitor_unknown")
        .await
        .expect("aggregate");

    assert_eq!(view.agent_cid, "agent_visitor_unknown");
    assert!(view.edges.is_empty());
    assert_eq!(view.reciprocation_count, 0);
    assert!(view.resilience_cliffs.is_empty());
    assert_eq!(view.freshness.state, FreshnessState::Live);
}

/// Test 2: 3 bindings, for_testing stub returns TransportError → all offline →
/// AllOffline freshness, edges empty (no slices arrived).
#[tokio::test]
async fn peer_topology_three_bindings_all_offline_via_for_testing() {
    let pool = test_pool();

    seed_binding(&pool, &libp2p::PeerId::random().to_string(), "agent_T26_A");
    seed_binding(&pool, &libp2p::PeerId::random().to_string(), "agent_T26_A");
    seed_binding(&pool, &libp2p::PeerId::random().to_string(), "agent_T26_A");

    let federator = Federator::new(P2PHandle::for_testing());
    let view = aggregate_peer_topology_view(&pool, &federator, "agent_T26_A")
        .await
        .expect("aggregate");

    // No slices arrived — edges are empty.
    assert!(view.edges.is_empty());
    // All peers were offline → AllOffline freshness.
    assert_eq!(view.freshness.state, FreshnessState::AllOffline);
}

/// Test 3: explicit freshness-rollup assertion — AllOffline when no peer returned Live.
#[tokio::test]
async fn peer_topology_freshness_rollup_alloffline_when_no_live() {
    let pool = test_pool();

    seed_binding(&pool, &libp2p::PeerId::random().to_string(), "agent_T26_B");
    seed_binding(&pool, &libp2p::PeerId::random().to_string(), "agent_T26_B");

    let federator = Federator::new(P2PHandle::for_testing());
    let view = aggregate_peer_topology_view(&pool, &federator, "agent_T26_B")
        .await
        .expect("aggregate");

    assert_eq!(
        view.freshness.state,
        FreshnessState::AllOffline,
        "freshness must be AllOffline when no peer slice arrived"
    );
}

/// Test 4: reciprocation_count is 0 when no live edges.
#[tokio::test]
async fn peer_topology_reciprocation_count_zero_when_no_live_edges() {
    let pool = test_pool();

    seed_binding(&pool, &libp2p::PeerId::random().to_string(), "agent_T26_C");

    let federator = Federator::new(P2PHandle::for_testing());
    let view = aggregate_peer_topology_view(&pool, &federator, "agent_T26_C")
        .await
        .expect("aggregate");

    assert_eq!(
        view.reciprocation_count, 0,
        "reciprocation_count must be 0 when no live edges returned"
    );
}

/// Test 5: build_local_slice returns a JSON object with the expected key
/// `connected_peer_households` pointing to an empty array.
#[tokio::test]
async fn build_local_slice_returns_connected_peer_households_key() {
    let pool = test_pool();
    let v = build_local_slice(&pool, &[]).await;

    assert!(
        v.get("connected_peer_households").is_some(),
        "missing key: connected_peer_households"
    );
    let arr = v["connected_peer_households"]
        .as_array()
        .expect("connected_peer_households must be an array");
    assert!(
        arr.is_empty(),
        "connected_peer_households must be empty in the stub"
    );
}
