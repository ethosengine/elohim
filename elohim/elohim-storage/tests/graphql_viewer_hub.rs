//! Integration tests for the `viewer.hub` GraphQL resolver (Epic A, Task A2).
//!
//! Uses `test_pool()` + `Federator::new(P2PHandle::for_testing())` — the
//! `for_testing()` stub returns `TransportError` for every federation call,
//! so all peers come back Offline / AllOffline. That is expected and tested.
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
// Test (a): no bindings → devices empty, freshness Live, totals zero
// ---------------------------------------------------------------------------

#[tokio::test]
async fn hub_no_bindings_returns_empty_devices_and_live_freshness() {
    let schema = make_schema();

    let query = r#"
        query {
            viewer(agentCid: "agent_hub_no_bindings") {
                agentCid
                hub {
                    agentCid
                    devices {
                        peerId
                        archetype
                        online
                    }
                    totals {
                        storageUsedBytes
                        storageTotalBytes
                        externalCommittedBytes
                        reciprocityNetBytes
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
    assert_eq!(viewer["agentCid"], "agent_hub_no_bindings");

    let hub = &viewer["hub"];
    assert_eq!(hub["agentCid"], "agent_hub_no_bindings");

    let devices = hub["devices"].as_array().expect("devices array");
    assert!(devices.is_empty(), "expected no devices, got {:?}", devices);

    // Empty devices → aggregator returns Live freshness.
    // async-graphql serializes enum variants as SCREAMING_SNAKE_CASE by default.
    assert_eq!(hub["freshness"]["state"], "LIVE");

    // Totals all zero
    assert_eq!(hub["totals"]["storageUsedBytes"], "0");
    assert_eq!(hub["totals"]["storageTotalBytes"], "0");
    assert_eq!(hub["totals"]["externalCommittedBytes"], "0");
    assert_eq!(hub["totals"]["reciprocityNetBytes"], "0");
}

// ---------------------------------------------------------------------------
// Test (b): one binding archetype=Workstation → device present, archetype correct
// ---------------------------------------------------------------------------

#[tokio::test]
async fn hub_one_binding_archetype_projected_correctly() {
    let pool = test_pool();
    let peer_id = libp2p::PeerId::random().to_string();
    seed_binding(&pool, &peer_id, "agent_hub_workstation", "desktop");

    let engine = open_test_engine();
    let federator = Arc::new(Federator::new(P2PHandle::for_testing()));
    let schema = build_schema(engine, pool, federator);

    let query = r#"
        query {
            viewer(agentCid: "agent_hub_workstation") {
                hub {
                    devices {
                        peerId
                        archetype
                        online
                        freshness {
                            state
                        }
                    }
                    totals {
                        storageUsedBytes
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
    let hub = &data["viewer"]["hub"];

    let devices = hub["devices"].as_array().expect("devices array");
    assert_eq!(devices.len(), 1, "expected exactly one device");

    let device = &devices[0];
    assert_eq!(device["peerId"], peer_id.as_str());
    // for_testing() stub returns TransportError → peer offline
    assert_eq!(device["online"], false);
    // Archetype from binding row.
    // async-graphql serializes enum variants as SCREAMING_SNAKE_CASE by default.
    assert_eq!(device["archetype"], "DESKTOP");
    // Device freshness: Offline (because for_testing stub transport error → Offline)
    assert_eq!(device["freshness"]["state"], "OFFLINE");

    // Hub-level freshness: AllOffline (all peers offline)
    assert_eq!(hub["freshness"]["state"], "ALL_OFFLINE");
}

// ---------------------------------------------------------------------------
// Test (c): malformed GraphQL → errors array non-empty, no panic
// ---------------------------------------------------------------------------

#[tokio::test]
async fn hub_malformed_query_returns_errors_array() {
    let schema = make_schema();

    // Missing closing brace — should be a parse error from async-graphql.
    let bad_query = r#"
        query {
            viewer(agentCid: "x") {
                hub {
                    devices {
                        peerId
                    }
                    totals {
                        storageUsedBytes
                        THIS_FIELD_DOES_NOT_EXIST
    "#;

    let resp = schema.execute(bad_query).await;
    assert!(
        !resp.errors.is_empty(),
        "expected errors for malformed query, got none"
    );
    // The response data should be null / empty (not a panic)
    // async-graphql returns serde_json::Value::Null when there's a parse error
    // so just confirm we got back an error without panicking.
}
