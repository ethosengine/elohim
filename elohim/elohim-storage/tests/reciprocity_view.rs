//! Integration tests for `services::reciprocity_view::aggregate_reciprocity_view`.
//!
//! Uses the shared `elohim_storage::test_util::test_pool` in-memory SQLite
//! pattern. All tests are async (tokio) because the aggregator is async.

use diesel::prelude::*;
use diesel::RunQueryDsl;
use elohim_storage::db::diesel_schema::{economic_events, peer_identity_bindings, rea_commitments};
use elohim_storage::db::models::{NewEconomicEvent, NewPeerIdentityBindingRow, NewReaCommitment};
use elohim_storage::services::reciprocity_view::aggregate_reciprocity_view;
use elohim_storage::test_util::test_pool;

// ============================================================================
// Seed helpers
// ============================================================================

fn seed_rea_commitment(
    pool: &elohim_storage::db::DbPool,
    id: &str,
    action: &str,
    provider: &str,
    receiver: &str,
    qty: f32,
) {
    let mut conn = pool.get().unwrap();
    diesel::insert_or_ignore_into(rea_commitments::table)
        .values(&NewReaCommitment {
            id,
            h_app_id: "lamad",
            action,
            provider,
            receiver,
            resource_conforms_to: None,
            resource_classified_as: None,
            resource_quantity_value: Some(qty),
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

fn seed_economic_event(
    pool: &elohim_storage::db::DbPool,
    id: &str,
    action: &str,
    provider: &str,
    receiver: &str,
    qty: f32,
) {
    let mut conn = pool.get().unwrap();
    diesel::insert_or_ignore_into(economic_events::table)
        .values(&NewEconomicEvent {
            id,
            h_app_id: "lamad",
            action,
            provider,
            receiver,
            resource_conforms_to: None,
            resource_inventoried_as: None,
            resource_classified_as_json: None,
            resource_quantity_value: Some(qty),
            resource_quantity_unit: Some("bytes"),
            effort_quantity_value: None,
            effort_quantity_unit: None,
            has_point_in_time: "2026-05-01T00:00:00Z",
            has_duration: None,
            input_of: None,
            output_of: None,
            lamad_event_type: None,
            content_id: None,
            contributor_presence_id: None,
            path_id: None,
            triggered_by: None,
            state: "active",
            note: None,
            metadata_json: None,
            dht_anchor_hash: None,
            at_location: None,
            verified_at: None,
        })
        .execute(&mut conn)
        .expect("seed economic_event");
}

fn seed_binding_with_agent(
    pool: &elohim_storage::db::DbPool,
    peer_id: &str,
    agent_cid: &str,
    archetype: &str,
) {
    let mut conn = pool.get().unwrap();
    diesel::insert_or_ignore_into(peer_identity_bindings::table)
        .values(&NewPeerIdentityBindingRow {
            peer_id: peer_id.to_string(),
            agent_cid: agent_cid.to_string(),
            dht_anchor_hash: format!("anchor-{peer_id}"),
            valid_from: "2026-04-01T00:00:00Z".to_string(),
            valid_until: None,
            observed_at: "2026-04-01T00:00:00Z".to_string(),
            source: "dht".to_string(),
            device_archetype: archetype.to_string(),
            superseded_by: None,
        })
        .execute(&mut conn)
        .expect("seed binding");
}

fn load_bindings(
    pool: &elohim_storage::db::DbPool,
    agent_cid: &str,
) -> Vec<elohim_storage::db::models::PeerIdentityBindingRow> {
    let mut conn = pool.get().unwrap();
    use elohim_storage::db::diesel_schema::peer_identity_bindings::dsl as pib;
    pib::peer_identity_bindings
        .filter(pib::agent_cid.eq(agent_cid))
        .load::<elohim_storage::db::models::PeerIdentityBindingRow>(&mut conn)
        .unwrap()
}

// ============================================================================
// Tests
// ============================================================================

/// T1: empty bindings → empty inflow + outflow, net == 0
#[tokio::test]
async fn reciprocity_empty_when_no_bindings() {
    let pool = test_pool();
    let bindings = vec![];

    let view = aggregate_reciprocity_view(&pool, "agent-alice", &bindings, &std::collections::HashSet::new())
        .await
        .expect("aggregate should succeed");

    assert!(view.inflow.is_empty(), "inflow must be empty");
    assert!(view.outflow.is_empty(), "outflow must be empty");
    assert_eq!(view.net_hosted_bytes, 0, "net must be 0");
    assert_eq!(view.agent_cid, "agent-alice");
}

/// T2: one outflow commitment, no delivery events → committed > 0, delivered == 0,
/// honored_percent == 0.0
#[tokio::test]
async fn reciprocity_outflow_committed_no_delivered_zero_percent() {
    let pool = test_pool();
    let agent_cid = "agent-t2";
    let my_peer = "peer-t2-a";
    let remote_peer = "peer-t2-remote";

    seed_binding_with_agent(&pool, my_peer, agent_cid, "node");
    seed_rea_commitment(
        &pool,
        "cmt-t2-1",
        "custody-blob",
        my_peer,
        remote_peer,
        5_000_000_000.0,
    );
    // No economic_events seeded.

    let bindings = load_bindings(&pool, agent_cid);
    let view = aggregate_reciprocity_view(&pool, agent_cid, &bindings, &std::collections::HashSet::new())
        .await
        .expect("aggregate should succeed");

    assert_eq!(view.outflow.len(), 1, "one outflow counterparty");
    assert!(view.inflow.is_empty(), "no inflow");

    let row = &view.outflow[0];
    assert_eq!(row.counterparty_household_id, remote_peer);
    assert!(row.committed_bytes > 0, "committed_bytes must be > 0");
    assert_eq!(row.delivered_bytes, 0, "no delivery → 0");
    assert_eq!(row.honored_percent, 0.0, "honored_percent must be 0.0");
}

/// T3: outflow commitment 14 GB, delivery event 11.2 GB → honored_percent ≈ 80.0
#[tokio::test]
async fn reciprocity_outflow_partially_delivered() {
    let pool = test_pool();
    let agent_cid = "agent-t3";
    let my_peer = "peer-t3-a";
    let remote_peer = "peer-t3-remote";

    seed_binding_with_agent(&pool, my_peer, agent_cid, "node");
    seed_rea_commitment(
        &pool,
        "cmt-t3-1",
        "custody-blob",
        my_peer,
        remote_peer,
        14_000_000_000.0,
    );
    seed_economic_event(
        &pool,
        "ev-t3-1",
        "serve-blob",
        my_peer,
        remote_peer,
        11_200_000_000.0,
    );

    let bindings = load_bindings(&pool, agent_cid);
    let view = aggregate_reciprocity_view(&pool, agent_cid, &bindings, &std::collections::HashSet::new())
        .await
        .expect("aggregate should succeed");

    assert_eq!(view.outflow.len(), 1);
    let row = &view.outflow[0];
    assert_eq!(row.committed_bytes, 14_000_000_000u64);
    assert_eq!(row.delivered_bytes, 11_200_000_000u64);
    let diff = (row.honored_percent - 80.0_f64).abs();
    assert!(
        diff < 0.5,
        "honored_percent must be ≈ 80.0, got {}",
        row.honored_percent
    );
}

/// T4: symmetric inflow case — receiver IN my_peers, committed 10 GB, delivered 8 GB → 80%
#[tokio::test]
async fn reciprocity_inflow_partially_delivered() {
    let pool = test_pool();
    let agent_cid = "agent-t4";
    let my_peer = "peer-t4-a";
    let remote_peer = "peer-t4-remote";

    seed_binding_with_agent(&pool, my_peer, agent_cid, "node");
    // Inflow: remote_peer committed to host MY bytes (receiver = my_peer)
    seed_rea_commitment(
        &pool,
        "cmt-t4-1",
        "custody-blob",
        remote_peer,
        my_peer,
        10_000_000_000.0,
    );
    seed_economic_event(
        &pool,
        "ev-t4-1",
        "serve-blob",
        remote_peer,
        my_peer,
        8_000_000_000.0,
    );

    let bindings = load_bindings(&pool, agent_cid);
    let view = aggregate_reciprocity_view(&pool, agent_cid, &bindings, &std::collections::HashSet::new())
        .await
        .expect("aggregate should succeed");

    assert!(view.outflow.is_empty(), "no outflow");
    assert_eq!(view.inflow.len(), 1, "one inflow counterparty");

    let row = &view.inflow[0];
    assert_eq!(row.counterparty_household_id, remote_peer);
    assert_eq!(row.committed_bytes, 10_000_000_000u64);
    assert_eq!(row.delivered_bytes, 8_000_000_000u64);
    let diff = (row.honored_percent - 80.0_f64).abs();
    assert!(
        diff < 0.5,
        "honored_percent must be ≈ 80.0, got {}",
        row.honored_percent
    );
}

/// T5: over-delivery — committed 3 GB, delivered 3.1 GB → honored_percent > 100.0
#[tokio::test]
async fn reciprocity_over_delivered_flagged() {
    let pool = test_pool();
    let agent_cid = "agent-t5";
    let my_peer = "peer-t5-a";
    let remote_peer = "peer-t5-remote";

    seed_binding_with_agent(&pool, my_peer, agent_cid, "node");
    seed_rea_commitment(
        &pool,
        "cmt-t5-1",
        "custody-blob",
        my_peer,
        remote_peer,
        3_000_000_000.0,
    );
    seed_economic_event(
        &pool,
        "ev-t5-1",
        "serve-blob",
        my_peer,
        remote_peer,
        3_100_000_000.0,
    );

    let bindings = load_bindings(&pool, agent_cid);
    let view = aggregate_reciprocity_view(&pool, agent_cid, &bindings, &std::collections::HashSet::new())
        .await
        .expect("aggregate should succeed");

    assert_eq!(view.outflow.len(), 1);
    let row = &view.outflow[0];
    assert!(
        row.honored_percent > 100.0,
        "over-delivery must produce honored_percent > 100.0, got {}",
        row.honored_percent
    );
}

/// T6: two commitments to the same counterparty — bytes sum correctly
#[tokio::test]
async fn reciprocity_aggregates_multiple_commitments_to_same_counterparty() {
    let pool = test_pool();
    let agent_cid = "agent-t6";
    let my_peer = "peer-t6-a";
    let remote_peer = "peer-t6-remote";

    seed_binding_with_agent(&pool, my_peer, agent_cid, "node");
    seed_rea_commitment(
        &pool,
        "cmt-t6-1",
        "custody-blob",
        my_peer,
        remote_peer,
        4_000_000_000.0,
    );
    seed_rea_commitment(
        &pool,
        "cmt-t6-2",
        "custody-blob",
        my_peer,
        remote_peer,
        6_000_000_000.0,
    );
    seed_economic_event(
        &pool,
        "ev-t6-1",
        "serve-blob",
        my_peer,
        remote_peer,
        5_000_000_000.0,
    );

    let bindings = load_bindings(&pool, agent_cid);
    let view = aggregate_reciprocity_view(&pool, agent_cid, &bindings, &std::collections::HashSet::new())
        .await
        .expect("aggregate should succeed");

    assert_eq!(view.outflow.len(), 1, "still one counterparty");
    let row = &view.outflow[0];
    // 4 GB + 6 GB = 10 GB committed
    assert_eq!(
        row.committed_bytes, 10_000_000_000u64,
        "committed must be sum of both commitments"
    );
    assert_eq!(row.delivered_bytes, 5_000_000_000u64);
    let diff = (row.honored_percent - 50.0_f64).abs();
    assert!(
        diff < 0.5,
        "honored_percent must be ≈ 50.0, got {}",
        row.honored_percent
    );
}

/// T7: seed both directions — inflow.len() and outflow.len() are independent
#[tokio::test]
async fn reciprocity_separates_inflow_and_outflow() {
    let pool = test_pool();
    let agent_cid = "agent-t7";
    let my_peer = "peer-t7-me";
    let remote_a = "peer-t7-remote-a";
    let remote_b = "peer-t7-remote-b";

    seed_binding_with_agent(&pool, my_peer, agent_cid, "node");

    // Outflow: I (my_peer) committed to host remote_a's bytes
    seed_rea_commitment(
        &pool,
        "cmt-t7-out",
        "custody-blob",
        my_peer,
        remote_a,
        2_000_000_000.0,
    );

    // Inflow: remote_b committed to host my bytes
    seed_rea_commitment(
        &pool,
        "cmt-t7-in",
        "custody-blob",
        remote_b,
        my_peer,
        3_000_000_000.0,
    );

    let bindings = load_bindings(&pool, agent_cid);
    let view = aggregate_reciprocity_view(&pool, agent_cid, &bindings, &std::collections::HashSet::new())
        .await
        .expect("aggregate should succeed");

    assert_eq!(view.outflow.len(), 1, "one outflow counterparty");
    assert_eq!(view.inflow.len(), 1, "one inflow counterparty");
    assert_eq!(view.outflow[0].counterparty_household_id, remote_a);
    assert_eq!(view.inflow[0].counterparty_household_id, remote_b);
}

/// T8: net_hosted_bytes == inflow_delivered - outflow_delivered
#[tokio::test]
async fn reciprocity_net_hosted_bytes_is_inflow_minus_outflow_delivered() {
    let pool = test_pool();
    let agent_cid = "agent-t8";
    let my_peer = "peer-t8-me";
    let remote_a = "peer-t8-remote-a"; // outflow counterparty
    let remote_b = "peer-t8-remote-b"; // inflow counterparty

    seed_binding_with_agent(&pool, my_peer, agent_cid, "node");

    // Use values ≤ 2^24 (16_777_216) to stay within f32's exact-integer range,
    // avoiding rounding drift when SQLite returns the SUM as f32.
    // outflow_delivered = 6_000_000 bytes; inflow_delivered = 9_000_000 bytes.
    seed_rea_commitment(
        &pool,
        "cmt-t8-out",
        "custody-blob",
        my_peer,
        remote_a,
        8_000_000.0,
    );
    seed_economic_event(
        &pool,
        "ev-t8-out",
        "serve-blob",
        my_peer,
        remote_a,
        6_000_000.0, // outflow_delivered = 6 MB
    );

    // Inflow commitment + delivery (receiver = my_peer)
    seed_rea_commitment(
        &pool,
        "cmt-t8-in",
        "custody-blob",
        remote_b,
        my_peer,
        10_000_000.0,
    );
    seed_economic_event(
        &pool,
        "ev-t8-in",
        "serve-blob",
        remote_b,
        my_peer,
        9_000_000.0, // inflow_delivered = 9 MB
    );

    let bindings = load_bindings(&pool, agent_cid);
    let view = aggregate_reciprocity_view(&pool, agent_cid, &bindings, &std::collections::HashSet::new())
        .await
        .expect("aggregate should succeed");

    // net = inflow_delivered (9 MB) - outflow_delivered (6 MB) = 3 MB
    let expected_net: i64 = 9_000_000 - 6_000_000;
    assert_eq!(
        view.net_hosted_bytes, expected_net,
        "net_hosted_bytes must equal inflow_delivered minus outflow_delivered"
    );
}
