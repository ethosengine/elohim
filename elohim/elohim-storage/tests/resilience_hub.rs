//! DB-layer tests for `GET /api/v1/resilience/{content_id}/hub` (Task C2).
//!
//! ## Testing strategy
//!
//! Consistent with the codebase pattern (api_placement_gaps.rs,
//! peer_statuses_route.rs): tests exercise the DB layer directly — the same
//! code path the handler delegates to — giving equivalent confidence that
//! the handler returns correct JSON when wired through HTTP.
//!
//! `spawn_test_server` does not exist in `test_util.rs`; HTTP-server variants
//! are stubbed with `#[ignore]` pointing at Task 17 (A2O coverage), per the
//! established codebase convention.
//!
//! ## Hub classification
//!
//! - Peer with `household_id` in `humans` → `dwelling`
//! - Peer with `collective_participations` row but no `household_id` → `collective`
//! - Peer with no binding / no human row → `computed`

use elohim_storage::db::models::{
    NewCollectiveParticipation, NewPeerBlobInventoryRow, NewPeerIdentityBindingRow,
};
use elohim_storage::services::resilience;
use elohim_storage::test_util::test_pool;
use elohim_views::{HubKind, ResilienceHubView};

// ---------------------------------------------------------------------------
// Seed helpers
// ---------------------------------------------------------------------------

fn now_ts() -> String {
    "2026-05-20T12:00:00Z".to_string()
}

fn insert_inventory(conn: &mut elohim_storage::db::PooledConn, peer_id: &str, blob_hash: &str) {
    use diesel::prelude::*;
    use elohim_storage::db::diesel_schema::peer_blob_inventory;
    let row = NewPeerBlobInventoryRow {
        peer_id: peer_id.to_string(),
        blob_hash: blob_hash.to_string(),
        last_seen_at: now_ts(),
        source: "gossip-snapshot".to_string(),
        sequence: 1,
        blake3_hash: None,
    };
    diesel::replace_into(peer_blob_inventory::table)
        .values(&row)
        .execute(conn)
        .expect("insert peer_blob_inventory");
}

fn insert_binding(conn: &mut elohim_storage::db::PooledConn, peer_id: &str, agent_cid: &str) {
    use diesel::prelude::*;
    use elohim_storage::db::diesel_schema::peer_identity_bindings;
    let row = NewPeerIdentityBindingRow {
        peer_id: peer_id.to_string(),
        agent_cid: agent_cid.to_string(),
        dht_anchor_hash: format!("dht-{peer_id}"),
        valid_from: now_ts(),
        valid_until: None,
        observed_at: now_ts(),
        source: "dht".to_string(),
        device_archetype: "node".to_string(),
        superseded_by: None,
    };
    diesel::replace_into(peer_identity_bindings::table)
        .values(&row)
        .execute(conn)
        .expect("insert peer_identity_bindings");
}

fn insert_human(
    conn: &mut elohim_storage::db::PooledConn,
    human_id: &str,
    agent_pub_key: Option<&str>,
    household_id: Option<&str>,
) {
    use diesel::prelude::*;
    use elohim_storage::db::diesel_schema::humans;
    // Minimal insert using a raw tuple approach via SQL, since NewHuman may
    // have many required fields. Use the existing human helpers if available,
    // otherwise fall back to raw diesel.
    diesel::sql_query(format!(
        "INSERT OR REPLACE INTO humans \
         (id, display_name, affinities, profile_reach, h_app_id, created_at, updated_at, \
          agent_pub_key, household_id) \
         VALUES ('{}', 'Test {}', '[]', 'commons', 'lamad', '{}', '{}', {}, {})",
        human_id,
        human_id,
        now_ts(),
        now_ts(),
        agent_pub_key
            .map(|k| format!("'{k}'"))
            .unwrap_or_else(|| "NULL".to_string()),
        household_id
            .map(|h| format!("'{h}'"))
            .unwrap_or_else(|| "NULL".to_string()),
    ))
    .execute(conn)
    .expect("insert human");
    let _ = humans::table; // suppress unused import warning
}

fn insert_collective(conn: &mut elohim_storage::db::PooledConn, collective_id: &str) {
    use diesel::prelude::*;
    // Insert a minimal collective row to satisfy the FK from collective_participations.
    diesel::sql_query(format!(
        "INSERT OR REPLACE INTO collectives \
         (id, h_app_id, name, governance_layer, reach, created_at, updated_at) \
         VALUES ('{}', 'lamad', 'Test Collective {}', 'community', 'community', '{}', '{}')",
        collective_id,
        collective_id,
        now_ts(),
        now_ts(),
    ))
    .execute(conn)
    .expect("insert collective");
}

fn insert_collective_participation(
    conn: &mut elohim_storage::db::PooledConn,
    human_id: &str,
    collective_id: &str,
) {
    use diesel::prelude::*;
    // Ensure the parent collective exists (FK requirement).
    insert_collective(conn, collective_id);

    let row = NewCollectiveParticipation {
        id: &format!("cp-{human_id}-{collective_id}"),
        h_app_id: "lamad",
        collective_id,
        human_id,
        intimacy_level: "member",
        role_context: None,
        governance_weight: 1.0,
        consent_state: "active",
        metadata_json: None,
    };
    use elohim_storage::db::diesel_schema::collective_participations;
    diesel::replace_into(collective_participations::table)
        .values(&row)
        .execute(conn)
        .expect("insert collective_participation");
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Empty hubs[] when content_id has no inventory records.
#[test]
fn resilience_hub_endpoint_returns_empty_for_unknown_content() {
    let pool = test_pool();
    let ctx = elohim_storage::db::AppContext {
        h_app_id: "lamad".to_string(),
        local_libp2p_peer_id: None,
    };

    let hubs = resilience::hub_summary(&pool, &ctx, "sha256-unknown-blob-hash")
        .expect("hub_summary should not error on unknown content");

    assert!(
        hubs.is_empty(),
        "expected empty hubs for unknown content, got {:#?}",
        hubs
    );

    // Verify wire-format round-trip via ResilienceHubView
    let view = ResilienceHubView {
        content_id: "sha256-unknown-blob-hash".to_string(),
        hubs,
    };
    let json = serde_json::to_string(&view).unwrap();
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(v["contentId"], "sha256-unknown-blob-hash");
    assert!(
        v["hubs"].as_array().unwrap().is_empty(),
        "hubs must be empty array in JSON"
    );
}

/// Single peer, no identity binding → computed hub-of-one.
#[test]
fn resilience_hub_endpoint_handles_single_device_hub() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let ctx = elohim_storage::db::AppContext {
        h_app_id: "lamad".to_string(),
        local_libp2p_peer_id: None,
    };

    let blob_hash = "sha256-single-device-test";
    let peer_id = "12D3KooWSingleDevice";

    // Inventory entry — no binding, no human row
    insert_inventory(&mut conn, peer_id, blob_hash);
    drop(conn);

    let hubs = resilience::hub_summary(&pool, &ctx, blob_hash)
        .expect("hub_summary should not error");

    assert_eq!(hubs.len(), 1, "expected exactly one hub");
    assert_eq!(hubs[0].hub_id, peer_id, "hub_id should be peer_id");
    assert_eq!(
        hubs[0].kind,
        HubKind::Computed,
        "kind must be computed when no membership is known"
    );
    assert_eq!(hubs[0].replica_count, 1, "replica_count should be 1");

    // Wire-format verification
    let json = serde_json::to_string(&hubs[0]).unwrap();
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(v["kind"], "computed", "kind must serialise as snake_case");
    assert_eq!(v["hubId"], peer_id, "hubId must be camelCase");
    assert_eq!(v["replicaCount"], 1, "replicaCount must be camelCase");
    // lastVerifiedSeconds may or may not be present — not required for computed
}

/// Two peers with mixed kinds: one Dwelling, one Computed.
/// Verifies polymorphic shape is returned correctly.
#[test]
fn resilience_hub_endpoint_returns_polymorphic_hubs() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let ctx = elohim_storage::db::AppContext {
        h_app_id: "lamad".to_string(),
        local_libp2p_peer_id: None,
    };

    let blob_hash = "sha256-polymorphic-hub-test";

    // Peer A: has identity binding → human with household_id → Dwelling
    let peer_a = "12D3KooWPeerAlice";
    let agent_a = "uhCAkAlicePubKey";
    let human_a = "human-alice";
    let household_a = "household-alpha";

    insert_inventory(&mut conn, peer_a, blob_hash);
    insert_binding(&mut conn, peer_a, agent_a);
    insert_human(&mut conn, human_a, Some(agent_a), Some(household_a));

    // Peer B: no identity binding → Computed
    let peer_b = "12D3KooWPeerBob";
    insert_inventory(&mut conn, peer_b, blob_hash);

    drop(conn);

    let hubs = resilience::hub_summary(&pool, &ctx, blob_hash)
        .expect("hub_summary should not error");

    // We expect two distinct hub entries: one Dwelling (household-alpha) and one Computed (peer_b)
    assert_eq!(hubs.len(), 2, "expected 2 hubs (dwelling + computed)");

    let dwelling = hubs.iter().find(|h| h.kind == HubKind::Dwelling);
    let computed = hubs.iter().find(|h| h.kind == HubKind::Computed);

    assert!(
        dwelling.is_some(),
        "expected at least one dwelling hub; got: {:#?}",
        hubs
    );
    assert!(
        computed.is_some(),
        "expected at least one computed hub; got: {:#?}",
        hubs
    );

    let d = dwelling.unwrap();
    assert_eq!(d.hub_id, household_a, "dwelling hub_id must be household_id");
    assert_eq!(d.replica_count, 1);

    let c = computed.unwrap();
    assert_eq!(c.hub_id, peer_b, "computed hub_id must be peer_id");
    assert_eq!(c.replica_count, 1);

    // Wire-format: mixed kinds in hubs[]
    let view = ResilienceHubView {
        content_id: blob_hash.to_string(),
        hubs: hubs.clone(),
    };
    let json = serde_json::to_value(&view).unwrap();
    let kinds: Vec<&str> = json["hubs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|h| h["kind"].as_str().unwrap())
        .collect();
    assert!(
        kinds.contains(&"dwelling"),
        "hubs[] must contain dwelling kind"
    );
    assert!(
        kinds.contains(&"computed"),
        "hubs[] must contain computed kind"
    );
}

/// Peer with collective participation but no household_id → Collective kind.
#[test]
fn resilience_hub_peer_with_collective_participation_is_collective() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let ctx = elohim_storage::db::AppContext {
        h_app_id: "lamad".to_string(),
        local_libp2p_peer_id: None,
    };

    let blob_hash = "sha256-collective-hub-test";
    let peer_c = "12D3KooWPeerCarol";
    let agent_c = "uhCAkCarolPubKey";
    let human_c = "human-carol";
    let collective_c = "collective-dawn-runners";

    insert_inventory(&mut conn, peer_c, blob_hash);
    insert_binding(&mut conn, peer_c, agent_c);
    // No household_id (None)
    insert_human(&mut conn, human_c, Some(agent_c), None);
    insert_collective_participation(&mut conn, human_c, collective_c);
    drop(conn);

    let hubs = resilience::hub_summary(&pool, &ctx, blob_hash)
        .expect("hub_summary should not error");

    assert_eq!(hubs.len(), 1, "expected exactly one hub");
    assert_eq!(hubs[0].kind, HubKind::Collective, "kind must be collective");
    assert_eq!(
        hubs[0].hub_id, collective_c,
        "hub_id must be collective_id"
    );
    assert_eq!(hubs[0].replica_count, 1);

    // Wire-format
    let json = serde_json::to_string(&hubs[0]).unwrap();
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(v["kind"], "collective");
}

/// Multiple peers in the same household are grouped into one Dwelling hub with
/// replica_count = number of peers.
#[test]
fn resilience_hub_same_household_grouped_with_summed_replica_count() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let ctx = elohim_storage::db::AppContext {
        h_app_id: "lamad".to_string(),
        local_libp2p_peer_id: None,
    };

    let blob_hash = "sha256-household-group-test";
    let household = "household-beta";

    for i in 0..3u8 {
        let peer_id = format!("12D3KooWHousehold{i}");
        let agent_cid = format!("uhCAkHouseholdAgent{i}");
        let human_id = format!("human-household-{i}");

        insert_inventory(&mut conn, &peer_id, blob_hash);
        insert_binding(&mut conn, &peer_id, &agent_cid);
        insert_human(&mut conn, &human_id, Some(&agent_cid), Some(household));
    }
    drop(conn);

    let hubs = resilience::hub_summary(&pool, &ctx, blob_hash)
        .expect("hub_summary should not error");

    assert_eq!(hubs.len(), 1, "all three peers in same household → one hub");
    assert_eq!(hubs[0].kind, HubKind::Dwelling);
    assert_eq!(hubs[0].hub_id, household);
    assert_eq!(
        hubs[0].replica_count, 3,
        "replica_count must sum across grouped peers"
    );
}

// ---------------------------------------------------------------------------
// HTTP-server variants — ignored until Task 17 adds spawn_test_server.
// ---------------------------------------------------------------------------

/// Ignored: `spawn_test_server` does not exist yet.
/// Task 17 (A2O) will add the hyper test-server harness and un-ignore this.
#[tokio::test]
#[ignore = "spawn_test_server not yet implemented — see Task 17"]
async fn resilience_hub_endpoint_returns_polymorphic_hubs_http() {
    unimplemented!("spawn_test_server not yet available")
}

/// Ignored: `spawn_test_server` does not exist yet.
/// Task 17 (A2O) will add the hyper test-server harness and un-ignore this.
#[tokio::test]
#[ignore = "spawn_test_server not yet implemented — see Task 17"]
async fn resilience_hub_endpoint_handles_single_device_hub_http() {
    unimplemented!("spawn_test_server not yet available")
}

/// Ignored: `spawn_test_server` does not exist yet.
/// Task 17 (A2O) will add the hyper test-server harness and un-ignore this.
#[tokio::test]
#[ignore = "spawn_test_server not yet implemented — see Task 17"]
async fn resilience_hub_endpoint_returns_empty_for_unknown_content_http() {
    unimplemented!("spawn_test_server not yet available")
}
