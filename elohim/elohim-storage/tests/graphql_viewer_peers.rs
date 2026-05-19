//! Integration tests for the `viewer.peers` GraphQL resolver (Epic A, Task A3).
//!
//! Uses `test_pool()` + `Federator::new(P2PHandle::for_testing())` — the
//! `for_testing()` stub returns `TransportError` for every federation call,
//! so peers come back AllOffline when bindings exist, Live when none.
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
// Helper: seed a peer-identity binding row
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

// ---------------------------------------------------------------------------
// Build a test schema with the real test_pool + for_testing federator
// ---------------------------------------------------------------------------

fn open_test_engine() -> Arc<GraphEngine> {
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = tmp.path().join("graph.db");
    // Leak the TempDir so the sled db file isn't removed while the engine is open.
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
// Test (a): empty bindings → edges empty, reciprocationCount 0,
//           resilienceCliffs empty, freshness LIVE
// ---------------------------------------------------------------------------

#[tokio::test]
async fn peers_no_bindings_returns_empty_edges_and_live_freshness() {
    let schema = make_schema();

    let query = r#"
        query {
            viewer(agentCid: "agent_peers_no_bindings") {
                agentCid
                peers {
                    agentCid
                    edges {
                        householdId
                        online
                    }
                    reciprocationCount
                    resilienceCliffs {
                        householdId
                        soleReplicaCidCount
                    }
                    freshness {
                        state
                    }
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
    assert_eq!(viewer["agentCid"], "agent_peers_no_bindings");

    let peers = &viewer["peers"];
    assert_eq!(peers["agentCid"], "agent_peers_no_bindings");

    let edges = peers["edges"].as_array().expect("edges array");
    assert!(edges.is_empty(), "expected no edges, got {:?}", edges);

    assert_eq!(peers["reciprocationCount"], 0);

    let cliffs = peers["resilienceCliffs"]
        .as_array()
        .expect("resilienceCliffs array");
    assert!(
        cliffs.is_empty(),
        "expected no resilience cliffs, got {:?}",
        cliffs
    );

    // Empty bindings → aggregator returns Live freshness.
    assert_eq!(peers["freshness"]["state"], "LIVE");
}

// ---------------------------------------------------------------------------
// Test (b): binding seeded → edge fields projected, nullable Option lands null
// ---------------------------------------------------------------------------

#[tokio::test]
async fn peers_with_binding_returns_edge_fields_and_alloffline_freshness() {
    let pool = test_pool();
    let peer_id = libp2p::PeerId::random().to_string();
    seed_binding(&pool, &peer_id, "agent_peers_one_binding", "node");

    let engine = open_test_engine();
    let federator = Arc::new(Federator::new(P2PHandle::for_testing()));
    let schema = build_schema(engine, pool, federator);

    // Query all fields including nullable ones to ensure they resolve correctly.
    let query = r#"
        query {
            viewer(agentCid: "agent_peers_one_binding") {
                peers {
                    agentCid
                    edges {
                        householdId
                        displayName
                        online
                        lastSyncSec
                        myCidsHostedByThem
                        theirCidsHostedByMe
                        netDiff
                        isCriticalForMe
                        iAmCriticalForThem
                    }
                    reciprocationCount
                    resilienceCliffs {
                        householdId
                        soleReplicaCidCount
                    }
                    freshness {
                        state
                        staleSinceMs
                    }
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
    let peers = &data["viewer"]["peers"];

    assert_eq!(peers["agentCid"], "agent_peers_one_binding");

    // for_testing() stub returns TransportError → no live slices arrive →
    // edges is empty (no peer returned data), freshness is AllOffline.
    let edges = peers["edges"].as_array().expect("edges array");
    assert!(
        edges.is_empty(),
        "for_testing stub returns no slices → edges should be empty"
    );

    // reciprocationCount = 0 (no live edges)
    assert_eq!(peers["reciprocationCount"], 0);

    // Freshness: AllOffline (all peers offline via for_testing stub)
    assert_eq!(peers["freshness"]["state"], "ALL_OFFLINE");

    // staleSinceMs is null when state is AllOffline (not Stale)
    assert!(
        peers["freshness"]["staleSinceMs"].is_null(),
        "staleSinceMs should be null for AllOffline state"
    );
}

// ---------------------------------------------------------------------------
// Test (c): malformed query → errors array non-empty, no panic
// ---------------------------------------------------------------------------

#[tokio::test]
async fn peers_malformed_query_returns_errors_array() {
    let schema = make_schema();

    // Field that doesn't exist on PeerTopology — should be a validation error.
    let bad_query = r#"
        query {
            viewer(agentCid: "x") {
                peers {
                    edges {
                        householdId
                    }
                    THIS_FIELD_DOES_NOT_EXIST_ON_PEER_TOPOLOGY
    "#;

    let resp = schema.execute(bad_query).await;
    assert!(
        !resp.errors.is_empty(),
        "expected errors for malformed query, got none"
    );
    // Confirm we got back an error without panicking.
}
