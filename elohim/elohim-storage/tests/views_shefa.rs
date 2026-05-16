//! Task 24 — shefa view builder tests.
//!
//! Seeds a graph with topology edges (MEMBER_OF, OPERATES_DEVICE, RECIPROCATES_WITH,
//! STEWARDS, VALUE_FLOW) and asserts the view builders return expected shapes.
#![cfg(feature = "graph-native")]

use elohim_storage::graph::{engine::GraphEngine, schema::apply_core_schema};
use elohim_storage::graph_views::shefa::{
    cluster, distribution, peer_topology, reciprocity, resilience_snapshot, topology_overview,
};
use cozo::DataValue;

fn open_engine() -> (GraphEngine, tempfile::TempDir) {
    let tmp = tempfile::tempdir().unwrap();
    let engine = GraphEngine::open(&tmp.path().join("graph.db")).unwrap();
    apply_core_schema(&engine).unwrap();
    (engine, tmp)
}

/// Insert a batch of epr_edge rows with [from, to, rel_type].
fn seed_edges(engine: &GraphEngine, rows: &[(&str, &str, &str)]) {
    for (from, to, rel) in rows {
        engine
            .run_script(
                r#"?[from_cid, to_cid, rel_type, asserted_at] <-
                    [[$from, $to, $rel, [1700000000, true]]]
                   :put epr_edge { from_cid, to_cid, rel_type => asserted_at }"#,
                &[
                    ("from", DataValue::from(*from)),
                    ("to", DataValue::from(*to)),
                    ("rel", DataValue::from(*rel)),
                ],
            )
            .unwrap();
    }
}

// ---------------------------------------------------------------------------
// peer_topology tests
// ---------------------------------------------------------------------------

#[test]
fn peer_topology_returns_reciprocation_edges() {
    let (engine, _tmp) = open_engine();
    seed_edges(
        &engine,
        &[
            ("did:test:alice", "did:test:bob", "RECIPROCATES_WITH"),
            ("did:test:alice", "did:test:carol", "RECIPROCATES_WITH"),
        ],
    );

    let view = peer_topology::build(&engine, "did:test:alice").expect("build peer_topology");

    assert_eq!(view.agent_cid, "did:test:alice");
    assert_eq!(view.edges.len(), 2, "two RECIPROCATES_WITH edges");
    assert_eq!(view.reciprocation_count, 2);
    assert!(view.resilience_cliffs.is_empty());
}

#[test]
fn peer_topology_empty_for_agent_with_no_edges() {
    let (engine, _tmp) = open_engine();

    let view = peer_topology::build(&engine, "did:test:nobody").expect("build peer_topology");

    assert!(view.edges.is_empty());
    assert_eq!(view.reciprocation_count, 0);
}

// ---------------------------------------------------------------------------
// reciprocity tests
// ---------------------------------------------------------------------------

#[test]
fn reciprocity_returns_inflow_and_outflow() {
    let (engine, _tmp) = open_engine();
    seed_edges(
        &engine,
        &[
            // Inbound to alice: bob and carol send to alice
            ("did:test:bob", "did:test:alice", "RECIPROCATES_WITH"),
            ("did:test:carol", "did:test:alice", "RECIPROCATES_WITH"),
            // Outbound from alice: alice sends to dave
            ("did:test:alice", "did:test:dave", "RECIPROCATES_WITH"),
        ],
    );

    let view = reciprocity::build(&engine, "did:test:alice").expect("build reciprocity");

    assert_eq!(view.agent_cid, "did:test:alice");
    assert_eq!(view.inflow.len(), 2, "two inbound edges");
    assert_eq!(view.outflow.len(), 1, "one outbound edge");
}

// ---------------------------------------------------------------------------
// cluster (MyClusterView) tests
// ---------------------------------------------------------------------------

#[test]
fn cluster_returns_devices_from_operates_device_edges() {
    let (engine, _tmp) = open_engine();
    seed_edges(
        &engine,
        &[
            ("did:test:alice", "device-1", "OPERATES_DEVICE"),
            ("did:test:alice", "device-2", "OPERATES_DEVICE"),
        ],
    );

    let view = cluster::build(&engine, "did:test:alice").expect("build cluster");

    assert_eq!(view.agent_cid, "did:test:alice");
    assert_eq!(view.devices.len(), 2, "two devices");
}

#[test]
fn cluster_empty_for_agent_with_no_devices() {
    let (engine, _tmp) = open_engine();

    let view = cluster::build(&engine, "did:test:nobody").expect("build cluster");

    assert!(view.devices.is_empty());
}

// ---------------------------------------------------------------------------
// resilience_snapshot tests
// ---------------------------------------------------------------------------

#[test]
fn resilience_snapshot_counts_steward_nodes() {
    let (engine, _tmp) = open_engine();
    seed_edges(
        &engine,
        &[
            ("content-cid-1", "steward-1", "STEWARDS"),
            ("content-cid-1", "steward-2", "STEWARDS"),
            ("content-cid-1", "steward-3", "STEWARDS"),
        ],
    );

    let view =
        resilience_snapshot::build(&engine, "content-cid-1").expect("build resilience_snapshot");

    assert_eq!(view.content_id, "content-cid-1");
    assert_eq!(view.stewarding_collectives, 3);
    assert_eq!(view.protection_status, "protected");
}

#[test]
fn resilience_snapshot_at_risk_when_no_stewards() {
    let (engine, _tmp) = open_engine();

    let view =
        resilience_snapshot::build(&engine, "content-no-stewards").expect("build resilience_snapshot");

    assert_eq!(view.stewarding_collectives, 0);
    assert_eq!(view.protection_status, "at-risk");
}

// ---------------------------------------------------------------------------
// distribution tests
// ---------------------------------------------------------------------------

#[test]
fn distribution_summary_counts_stewards_as_replicas() {
    let (engine, _tmp) = open_engine();
    // Also seed a qahal row so reach_class is derivable
    engine
        .run_script(
            r#"?[cid, reach, layer, attestation_requirements] <-
                [['content-d1', 'commons', null, []]]
               :put epr_qahal { cid => reach, layer, attestation_requirements }"#,
            &[],
        )
        .unwrap();
    seed_edges(
        &engine,
        &[
            ("content-d1", "steward-a", "STEWARDS"),
            ("content-d1", "steward-b", "STEWARDS"),
            ("content-d1", "steward-c", "STEWARDS"),
            ("content-d1", "steward-d", "STEWARDS"),
            ("content-d1", "steward-e", "STEWARDS"),
        ],
    );

    let view = distribution::build_summary(&engine, "content-d1").expect("build distribution");

    assert_eq!(view.replica_count, 5);
    assert_eq!(view.replica_target, 7);
    // 5 replicas ≥ 3 → Healthy
    assert_eq!(
        view.replica_health,
        elohim_storage::views::ReplicaHealth::Healthy
    );
}

#[test]
fn distribution_details_wraps_summary() {
    let (engine, _tmp) = open_engine();
    engine
        .run_script(
            r#"?[cid, reach, layer, attestation_requirements] <-
                [['content-d2', 'commons', null, []]]
               :put epr_qahal { cid => reach, layer, attestation_requirements }"#,
            &[],
        )
        .unwrap();

    let view = distribution::build_details(&engine, "content-d2").expect("build distribution details");

    assert_eq!(view.summary.replica_count, 0);
    assert!(view.replica_peers.is_empty());
}

// ---------------------------------------------------------------------------
// topology_overview tests
// ---------------------------------------------------------------------------

#[test]
fn topology_overview_returns_households_and_collectives() {
    let (engine, _tmp) = open_engine();
    seed_edges(
        &engine,
        &[
            // alice is in household-1
            ("did:test:alice", "household-1", "MEMBER_OF"),
            // bob is also in household-1
            ("did:test:bob", "household-1", "MEMBER_OF"),
            // alice operates a device
            ("did:test:alice", "device-1", "OPERATES_DEVICE"),
            // carol reciprocates with alice
            ("did:test:carol", "did:test:alice", "RECIPROCATES_WITH"),
        ],
    );

    let view =
        topology_overview::build(&engine, "did:test:alice").expect("build topology_overview");

    assert_eq!(view.contributor_did, "did:test:alice");
    assert_eq!(view.households.len(), 1, "alice is in one household");
    let hh = &view.households[0];
    assert_eq!(hh.household_cid, "household-1");
    // Both alice and bob are members
    assert_eq!(hh.members.len(), 2, "household has 2 members");
    assert_eq!(hh.device_count, 1, "one device operated by a member");

    assert_eq!(view.reciprocity_inbound.len(), 1, "carol reciprocates");
    assert_eq!(view.reciprocity_inbound[0].from_did, "did:test:carol");
}

#[test]
fn topology_overview_empty_for_contributor_with_no_edges() {
    let (engine, _tmp) = open_engine();

    let view =
        topology_overview::build(&engine, "did:test:nobody").expect("build topology_overview");

    assert!(view.households.is_empty());
    assert!(view.collectives.is_empty());
    assert!(view.reciprocity_inbound.is_empty());
}
