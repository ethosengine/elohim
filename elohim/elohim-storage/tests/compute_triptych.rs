//! TDD tests for `services::reciprocity_view::aggregate_stewarded_bytes_by_peer`
//! and `views::ComputeTriptych` wire-format struct.
//!
//! A1 substrate seed: verifies that the helper SUMs custody-blob REA
//! commitments per peer (where that peer is the provider) to compute
//! "stewarded bytes" — bytes a peer has committed to host for others.
//!
//! Source-of-truth lineage: Category-C operational projection over Category-A
//! notarized REA Commitments (`rea_commitments` table).

use diesel::RunQueryDsl;
use elohim_storage::db::diesel_schema::rea_commitments;
use elohim_storage::db::models::NewReaCommitment;
use elohim_storage::services::reciprocity_view::aggregate_stewarded_bytes_by_peer;
use elohim_storage::test_util::test_pool;
use elohim_views::ComputeTriptych;

// A4 imports — only compiled when graph-native feature is present.
#[cfg(feature = "graph-native")]
use {
    elohim_storage::db::models::NewPeerIdentityBindingRow,
    elohim_storage::db::peer_identity_bindings,
    elohim_storage::graph::{engine::GraphEngine, schema::apply_core_schema},
    elohim_storage::graphql::server::build_schema,
    elohim_storage::services::federator::Federator,
    elohim_storage::P2PHandle,
    std::sync::Arc,
};

// ============================================================================
// Seed helper
// ============================================================================

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

// ============================================================================
// Tests
// ============================================================================

#[tokio::test]
async fn stewarded_bytes_sums_custody_blob_commitments_by_peer() {
    let pool = test_pool();
    // peer_M_laptop: 1_000_000 + 500_000 + 300_000 = 1_800_000
    seed_custody_commitment(&pool, "c1", "peer_M_laptop", "agent_T", 1_000_000);
    seed_custody_commitment(&pool, "c2", "peer_M_laptop", "agent_J", 500_000);
    seed_custody_commitment(&pool, "c3", "peer_M_phone", "agent_T", 200_000);
    seed_custody_commitment(&pool, "c4", "peer_M_laptop", "agent_T2", 300_000);

    let peers = vec!["peer_M_laptop".to_string(), "peer_M_phone".to_string()];
    let result = aggregate_stewarded_bytes_by_peer(&pool, &peers)
        .await
        .expect("aggregate");

    assert_eq!(result.get("peer_M_laptop"), Some(&1_800_000_u64));
    assert_eq!(result.get("peer_M_phone"), Some(&200_000_u64));
}

#[tokio::test]
async fn stewarded_bytes_empty_when_no_commitments() {
    let pool = test_pool();
    let peers = vec!["peer_lonely".to_string()];
    let result = aggregate_stewarded_bytes_by_peer(&pool, &peers)
        .await
        .expect("aggregate");
    // Either empty or zero — both are acceptable semantics for no rows found.
    assert!(result.is_empty() || result.get("peer_lonely").copied() == Some(0));
}

#[tokio::test]
async fn stewarded_bytes_ignores_non_custody_blob_actions() {
    let pool = test_pool();
    // Insert a project-blob commitment — must NOT be counted.
    let mut conn = pool.get().unwrap();
    diesel::insert_or_ignore_into(rea_commitments::table)
        .values(&NewReaCommitment {
            id: "non-custody",
            h_app_id: "lamad",
            action: "project-blob",
            provider: "peer_X",
            receiver: "agent_Y",
            resource_conforms_to: None,
            resource_classified_as: None,
            resource_quantity_value: Some(999_999.0),
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
        .expect("seed non-custody");

    let peers = vec!["peer_X".to_string()];
    let result = aggregate_stewarded_bytes_by_peer(&pool, &peers)
        .await
        .expect("aggregate");
    assert!(result.is_empty() || result.get("peer_X").copied() == Some(0));
}

// ============================================================================
// A2: ComputeTriptych wire-format tests
// ============================================================================

#[test]
fn compute_triptych_serializes_camel_case() {
    let triptych = ComputeTriptych {
        free: Some(1_000),
        used: Some(500),
        stewarded: Some(250),
    };
    let json = serde_json::to_value(&triptych).unwrap();
    assert_eq!(json["free"], 1000);
    assert_eq!(json["used"], 500);
    assert_eq!(json["stewarded"], 250);
}

#[test]
fn compute_triptych_allows_null_fields() {
    let triptych = ComputeTriptych {
        free: None,
        used: None,
        stewarded: None,
    };
    let json = serde_json::to_value(&triptych).unwrap();
    assert!(json["free"].is_null());
    assert!(json["used"].is_null());
    assert!(json["stewarded"].is_null());
}

// ============================================================================
// A4: GraphQL resolver — ComputeTriptychGql stringification tests
// ============================================================================

/// Helper: open a temp CozoDB engine and apply the core schema.
#[cfg(feature = "graph-native")]
fn open_test_engine() -> Arc<GraphEngine> {
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = tmp.path().join("graph.db");
    std::mem::forget(tmp); // keep backing file alive for engine lifetime
    let engine = Arc::new(GraphEngine::open(&path).expect("open graph engine"));
    apply_core_schema(&engine).expect("apply core schema");
    engine
}

/// Seed a peer-identity binding so `aggregate_my_cluster_view` has a peer to
/// project.  The binding uses agent_cid `agent_A4` and peer_id `peer_A4_laptop`.
#[cfg(feature = "graph-native")]
fn seed_binding_for_a4(pool: &elohim_storage::db::DbPool) {
    let mut conn = pool.get().expect("conn");
    let row = NewPeerIdentityBindingRow {
        peer_id: "peer_A4_laptop".to_string(),
        agent_cid: "agent_A4".to_string(),
        dht_anchor_hash: "anchor-peer_A4_laptop".to_string(),
        valid_from: "2026-01-01T00:00:00Z".to_string(),
        valid_until: None,
        observed_at: "2026-04-24T00:00:00Z".to_string(),
        source: "dht".to_string(),
        device_archetype: "desktop".to_string(),
        superseded_by: None,
    };
    peer_identity_bindings::upsert(&mut conn, &row).expect("upsert binding");
}

/// Seed a custody-blob commitment for peer_A4_laptop.
#[cfg(feature = "graph-native")]
fn seed_custody_for_a4(pool: &elohim_storage::db::DbPool) {
    let mut conn = pool.get().unwrap();
    diesel::insert_or_ignore_into(rea_commitments::table)
        .values(&NewReaCommitment {
            id: "a4-custody-1",
            h_app_id: "lamad",
            action: "custody-blob",
            provider: "peer_A4_laptop",
            receiver: "agent_T",
            resource_conforms_to: None,
            resource_classified_as: None,
            resource_quantity_value: Some(1_500_000_f32),
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
        .expect("seed custody commitment");
}

/// A4: `viewer.hub.devices[].compute` is exposed via GraphQL and all three
/// fields are stringified for JS Number precision safety.
///
/// The `for_testing()` stub transport returns `TransportError` so the peer is
/// Offline — but the `stewarded` field is populated from the REA ledger (which
/// is a local Diesel query, not a P2P federation call).  `free` and `used` come
/// from the federation payload (unavailable under the stub), so they will be
/// `null` in this scenario.  The critical invariant this test asserts is:
/// **`stewarded` is a JSON string, not a number**.
#[cfg(feature = "graph-native")]
#[tokio::test]
async fn graphql_viewer_hub_returns_compute_triptych() {
    let pool = test_pool();
    seed_binding_for_a4(&pool);
    seed_custody_for_a4(&pool);

    let engine = open_test_engine();
    let federator = Arc::new(Federator::new(P2PHandle::for_testing()));
    let schema = build_schema(engine, pool, federator);

    let query = r#"
        query {
            viewer(agentCid: "agent_A4") {
                hub {
                    devices {
                        peerId
                        compute { free used stewarded }
                    }
                }
            }
        }
    "#;

    let res = schema.execute(query).await;
    assert!(res.errors.is_empty(), "graphql errors: {:?}", res.errors);

    let data = res.data.into_json().unwrap();
    let devices = &data["viewer"]["hub"]["devices"];
    let laptop = devices
        .as_array()
        .expect("devices array")
        .iter()
        .find(|d| d["peerId"] == "peer_A4_laptop")
        .expect("peer_A4_laptop in devices");

    // stewarded bytes come from the REA ledger (local query) — must be a String.
    assert!(
        laptop["compute"]["stewarded"].is_string(),
        "stewarded must be a JSON String for JS precision safety, got: {:?}",
        laptop["compute"]["stewarded"]
    );
    assert_eq!(
        laptop["compute"]["stewarded"], "1500000",
        "stewarded value mismatch"
    );

    // free and used come from federation payload (offline under test stub) — null is correct.
    assert!(
        laptop["compute"]["free"].is_null() || laptop["compute"]["free"].is_string(),
        "free must be String or null, got: {:?}",
        laptop["compute"]["free"]
    );
    assert!(
        laptop["compute"]["used"].is_null() || laptop["compute"]["used"].is_string(),
        "used must be String or null, got: {:?}",
        laptop["compute"]["used"]
    );
}
