//! Integration tests for the `viewer.reciprocity` GraphQL resolver
//! (L6 plan: 2026-05-19-viewer-symmetry-reciprocity-qahal-substrate.md).
//!
//! Wraps `services::reciprocity_view::aggregate_reciprocity_view` — pure SQL,
//! no federation. Cold-cache reads use empty `connected_peers`, which gives
//! `online: Some(false)` semantics on every row (per Phase 4 T10).
//!
//! Schema executes in-process via `schema.execute(...)` — no HTTP layer needed.

#![cfg(feature = "graph-native")]

use std::sync::Arc;

use elohim_storage::db::models::NewPeerIdentityBindingRow;
use elohim_storage::db::peer_identity_bindings;
use elohim_storage::graph::{engine::GraphEngine, schema::apply_core_schema};
use elohim_storage::graphql::server::build_schema;
use elohim_storage::services::federator::Federator;
use elohim_storage::test_util::test_pool;
use elohim_storage::P2PHandle;

// ---------------------------------------------------------------------------
// Helpers (mirrors graphql_viewer_peers.rs)
// ---------------------------------------------------------------------------

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

fn open_test_engine() -> Arc<GraphEngine> {
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = tmp.path().join("graph.db");
    std::mem::forget(tmp);
    let engine = Arc::new(GraphEngine::open(&path).expect("open graph engine"));
    apply_core_schema(&engine).expect("apply core schema");
    engine
}

fn make_schema() -> elohim_storage::graphql::server::AppSchema {
    let pool = test_pool();
    let engine = open_test_engine();
    let federator = Arc::new(Federator::new(P2PHandle::for_testing()));
    build_schema(engine, pool, federator)
}

// ---------------------------------------------------------------------------
// Test (a): no bindings → empty inflow/outflow + zero net/capacity
// ---------------------------------------------------------------------------

#[tokio::test]
async fn reciprocity_no_bindings_returns_empty_ledger() {
    let schema = make_schema();

    let query = r#"
        query {
            viewer(agentCid: "agent_reciprocity_empty") {
                agentCid
                reciprocity {
                    agentCid
                    inflow {
                        counterpartyHouseholdId
                        committedBytes
                        deliveredBytes
                        honoredPercent
                    }
                    outflow {
                        counterpartyHouseholdId
                        committedBytes
                        deliveredBytes
                    }
                    netHostedBytes
                    capacityAvailableBytes
                }
            }
        }
    "#;

    let resp = schema.execute(query).await;
    assert!(
        resp.errors.is_empty(),
        "unexpected errors: {:?}",
        resp.errors
    );

    let data = resp.data.into_json().expect("data");
    let viewer = &data["viewer"];
    assert_eq!(viewer["agentCid"], "agent_reciprocity_empty");

    let reciprocity = &viewer["reciprocity"];
    assert_eq!(reciprocity["agentCid"], "agent_reciprocity_empty");
    assert!(
        reciprocity["inflow"]
            .as_array()
            .expect("inflow array")
            .is_empty(),
        "expected empty inflow"
    );
    assert!(
        reciprocity["outflow"]
            .as_array()
            .expect("outflow array")
            .is_empty(),
        "expected empty outflow"
    );
    // u64/i64 surface as stringified for JS precision safety.
    assert_eq!(reciprocity["netHostedBytes"], "0");
    assert_eq!(reciprocity["capacityAvailableBytes"], "0");
}

// ---------------------------------------------------------------------------
// Test (b): binding seeded, no REA commitments → empty ledger + zero totals
// (Without rea_commitments rows the aggregator returns empty arrays even when
// bindings exist; that's the same legitimate cold-state we cover with the
// HTTP handler.)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn reciprocity_with_binding_no_commitments_returns_empty_ledger() {
    let pool = test_pool();
    let peer_id = libp2p::PeerId::random().to_string();
    seed_binding(&pool, &peer_id, "agent_reciprocity_binding", "node");

    let engine = open_test_engine();
    let federator = Arc::new(Federator::new(P2PHandle::for_testing()));
    let schema = build_schema(engine, pool, federator);

    let query = r#"
        query {
            viewer(agentCid: "agent_reciprocity_binding") {
                reciprocity {
                    agentCid
                    inflow {
                        counterpartyHouseholdId
                        displayName
                        committedBytes
                        deliveredBytes
                        honoredPercent
                        online
                    }
                    outflow {
                        counterpartyHouseholdId
                        committedBytes
                        deliveredBytes
                    }
                    netHostedBytes
                    capacityAvailableBytes
                }
            }
        }
    "#;

    let resp = schema.execute(query).await;
    assert!(
        resp.errors.is_empty(),
        "unexpected errors: {:?}",
        resp.errors
    );

    let data = resp.data.into_json().expect("data");
    let reciprocity = &data["viewer"]["reciprocity"];
    assert_eq!(reciprocity["agentCid"], "agent_reciprocity_binding");
    assert!(
        reciprocity["inflow"].as_array().expect("inflow").is_empty(),
        "no commitments seeded — inflow must be empty"
    );
    assert!(
        reciprocity["outflow"]
            .as_array()
            .expect("outflow")
            .is_empty(),
        "no commitments seeded — outflow must be empty"
    );
    assert_eq!(reciprocity["netHostedBytes"], "0");
}

// ---------------------------------------------------------------------------
// Test (c): malformed query → errors array non-empty, no panic
// ---------------------------------------------------------------------------

#[tokio::test]
async fn reciprocity_malformed_query_returns_errors_array() {
    let schema = make_schema();

    // Field that doesn't exist on ReciprocityViewGql — should be a validation error.
    let bad_query = r#"
        query {
            viewer(agentCid: "x") {
                reciprocity {
                    inflow {
                        counterpartyHouseholdId
                    }
                    THIS_FIELD_DOES_NOT_EXIST_ON_RECIPROCITY_VIEW
    "#;

    let resp = schema.execute(bad_query).await;
    assert!(
        !resp.errors.is_empty(),
        "expected errors for malformed query, got none"
    );
}
