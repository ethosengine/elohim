//! T25 integration tests for cluster_view::aggregate_my_cluster_view.
//!
//! Multi-peer integration tests that need real federation responders are
//! deferred to Jenkins per project memory `feedback_shift_measure_jenkins`.
//! These tests exercise the aggregator's classification logic against the
//! P2PHandle::for_testing() stub (which always returns TransportError →
//! all peers Offline → AllOffline freshness).

use elohim_storage::db::models::NewPeerIdentityBindingRow;
use elohim_storage::db::peer_identity_bindings;
use elohim_storage::services::cluster_view::{aggregate_my_cluster_view, build_local_slice};
use elohim_storage::services::federator::Federator;
use elohim_storage::test_util::test_pool;
use elohim_storage::views::{DeviceArchetype, FreshnessState};
use elohim_storage::P2PHandle;

fn seed_binding(
    pool: &elohim_storage::db::DbPool,
    peer_id: &str,
    agent_cid: &str,
    archetype: &str,
) {
    let mut conn = pool.get().expect("conn");
    let row = NewPeerIdentityBindingRow {
        peer_id: peer_id.to_string(),
        agent_cid: agent_cid.to_string(),
        dht_anchor_hash: format!("anchor-{peer_id}"),
        valid_from: "2026-01-01T00:00:00Z".to_string(),
        valid_until: None,
        observed_at: "2026-04-24T00:00:00Z".to_string(),
        source: "dht".to_string(),
        device_archetype: archetype.to_string(),
        superseded_by: None,
    };
    peer_identity_bindings::upsert(&mut conn, &row).expect("upsert binding");
}

#[tokio::test]
async fn cluster_view_no_bindings_returns_empty_devices_and_live() {
    let pool = test_pool();
    let federator = Federator::new(P2PHandle::for_testing());
    let view = aggregate_my_cluster_view(&pool, &federator, "agent_unknown")
        .await
        .expect("aggregate");

    assert_eq!(view.agent_cid, "agent_unknown");
    assert!(view.devices.is_empty());
    assert_eq!(view.freshness.state, FreshnessState::Live);
    assert_eq!(view.totals.storage_used_bytes, 0);
}

#[tokio::test]
async fn cluster_view_three_bindings_all_offline_via_for_testing_stub() {
    // for_testing() returns TransportError for view_federate, so all three
    // peers come back Offline → freshness = AllOffline.
    let pool = test_pool();
    seed_binding(
        &pool,
        &libp2p::PeerId::random().to_string(),
        "agent_M",
        "desktop",
    );
    seed_binding(
        &pool,
        &libp2p::PeerId::random().to_string(),
        "agent_M",
        "node",
    );
    seed_binding(
        &pool,
        &libp2p::PeerId::random().to_string(),
        "agent_M",
        "mobile",
    );

    let federator = Federator::new(P2PHandle::for_testing());
    let view = aggregate_my_cluster_view(&pool, &federator, "agent_M")
        .await
        .expect("aggregate");

    assert_eq!(view.devices.len(), 3);
    assert!(view.devices.iter().all(|d| !d.online));
    assert!(view
        .devices
        .iter()
        .all(|d| d.freshness.state == FreshnessState::Offline));
    assert_eq!(view.freshness.state, FreshnessState::AllOffline);
}

#[tokio::test]
async fn cluster_view_archetype_picked_up_from_binding() {
    let pool = test_pool();
    let peer_a = libp2p::PeerId::random().to_string();
    let peer_b = libp2p::PeerId::random().to_string();
    seed_binding(&pool, &peer_a, "agent_M2", "desktop");
    seed_binding(&pool, &peer_b, "agent_M2", "mobile");

    let federator = Federator::new(P2PHandle::for_testing());
    let view = aggregate_my_cluster_view(&pool, &federator, "agent_M2")
        .await
        .expect("aggregate");

    let summary_a = view
        .devices
        .iter()
        .find(|d| d.peer_id == peer_a)
        .expect("peer A");
    let summary_b = view
        .devices
        .iter()
        .find(|d| d.peer_id == peer_b)
        .expect("peer B");
    assert_eq!(summary_a.archetype, DeviceArchetype::Desktop);
    assert_eq!(summary_b.archetype, DeviceArchetype::Mobile);
}

#[tokio::test]
async fn cluster_view_external_committed_bytes_zero_when_no_commitments() {
    let pool = test_pool();
    seed_binding(
        &pool,
        &libp2p::PeerId::random().to_string(),
        "agent_M3",
        "desktop",
    );
    let federator = Federator::new(P2PHandle::for_testing());
    let view = aggregate_my_cluster_view(&pool, &federator, "agent_M3")
        .await
        .unwrap();
    assert_eq!(view.totals.external_committed_bytes, 0);
}

#[tokio::test]
async fn build_local_slice_returns_expected_payload_keys() {
    let pool = test_pool();
    let v = build_local_slice(&pool).await;
    for key in &[
        "display_name",
        "storage_used_bytes",
        "storage_total_bytes",
        "hosting_count",
        "projecting_count",
        "beacon_age_ms",
    ] {
        assert!(v.get(*key).is_some(), "missing key: {key}");
    }
}
