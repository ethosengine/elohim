//! T25 integration tests for cluster_view::aggregate_my_cluster_view.
//!
//! Multi-peer integration tests that need real federation responders are
//! deferred to Jenkins per project memory `feedback_shift_measure_jenkins`.
//! These tests exercise the aggregator's classification logic against the
//! P2PHandle::for_testing() stub (which always returns TransportError →
//! all peers Offline → AllOffline freshness).

use diesel::RunQueryDsl;
use elohim_storage::db::diesel_schema::rea_commitments;
use elohim_storage::db::models::{NewPeerIdentityBindingRow, NewReaCommitment};
use elohim_storage::db::peer_identity_bindings;
use elohim_storage::services::cluster_view::{aggregate_my_cluster_view, build_local_slice};
use elohim_storage::services::federator::Federator;
use elohim_storage::test_util::test_pool;
use elohim_storage::views::{DeviceArchetype, FreshnessState};
use elohim_storage::P2PHandle;

/// Seed a custody-blob REA commitment: `provider` commits to host `bytes` on behalf of `receiver`.
fn seed_custody_commitment(
    pool: &elohim_storage::db::DbPool,
    id: &str,
    provider: &str,
    receiver: &str,
    bytes: u64,
) {
    let mut conn = pool.get().unwrap();
    diesel::insert_or_ignore_into(rea_commitments::table)
        .values(&NewReaCommitment {
            id,
            h_app_id: "lamad",
            action: "custody-blob",
            provider,
            receiver,
            resource_conforms_to: None,
            resource_classified_as: None,
            resource_quantity_value: Some(bytes as f32),
            resource_quantity_unit: Some("bytes"),
            effort_quantity_value: None,
            effort_quantity_unit: None,
            has_beginning: None,
            has_end: None,
            due: None,
            clause_of: None,
            in_scope_of: None,
            medium_of_exchange_id: None,
            state: "active",
            finished: 0,
            note: None,
            metadata_json: None,
            dht_anchor_hash: None,
        })
        .execute(&mut conn)
        .expect("seed rea_commitment");
}

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

// ============================================================================
// A3: ComputeTriptych population in aggregate_my_cluster_view
// ============================================================================

#[tokio::test]
async fn cluster_view_includes_compute_triptych_stewarded_bytes() {
    // Seeds a binding that associates peer_M_laptop with agent_M_ct.
    // Seeds a custody-blob commitment where peer_M_laptop is the provider.
    // Asserts that the resulting DeviceSummary.compute.stewarded reflects the SUM.
    //
    // Note: free/used are derived from storage_total_bytes/storage_used_bytes which
    // come from the federated slice payload. With P2PHandle::for_testing() all peers
    // return Offline (no slice), so those fields will be None — only stewarded is
    // verifiable in this fixture.
    let pool = test_pool();

    let peer_id = "peer_ct_laptop_A3".to_string();
    seed_binding(&pool, &peer_id, "agent_M_ct", "desktop");
    seed_custody_commitment(&pool, "ct-c1", &peer_id, "agent_T_ct", 750_000);
    seed_custody_commitment(&pool, "ct-c2", &peer_id, "agent_J_ct", 250_000);

    let federator = Federator::new(P2PHandle::for_testing());
    let view = aggregate_my_cluster_view(&pool, &federator, "agent_M_ct")
        .await
        .expect("aggregate");

    let laptop = view
        .devices
        .iter()
        .find(|d| d.peer_id == peer_id)
        .expect("peer_ct_laptop_A3 present in cluster view");

    let compute = laptop.compute.as_ref().expect("compute triptych populated");
    assert_eq!(
        compute.stewarded,
        Some(1_000_000),
        "stewarded should reflect SUM of custody-blob commitments for this peer (750_000 + 250_000)"
    );
    // With for_testing() the peer is offline and no slice is returned, so
    // storage bytes are None — free and used will also be None.
    assert!(
        compute.free.is_none(),
        "free is None when storage bytes unavailable (offline peer)"
    );
    assert!(
        compute.used.is_none(),
        "used is None when storage bytes unavailable (offline peer)"
    );
}
