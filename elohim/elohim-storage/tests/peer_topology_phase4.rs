//! Phase 4 T11 — peer_topology_view resilience_cliffs from substrate.

use diesel::prelude::*;
use elohim_storage::db::models::{NewPeerBlobInventoryRow, NewPeerIdentityBindingRow};
use elohim_storage::p2p::P2PHandle;
use elohim_storage::services::federator::Federator;
use elohim_storage::services::peer_topology_view::aggregate_peer_topology_view;
use elohim_storage::test_util::test_pool;

fn insert_content_with_creator(
    conn: &mut diesel::SqliteConnection,
    blob_hash: &str,
    created_by: &str,
) {
    use elohim_storage::db::diesel_schema::content;
    diesel::insert_or_ignore_into(content::table)
        .values((
            content::id.eq(format!("content-{blob_hash}")),
            content::h_app_id.eq("lamad"),
            content::title.eq("Test content"),
            content::content_type.eq("concept"),
            content::content_format.eq("markdown"),
            content::blob_hash.eq(blob_hash),
            content::reach.eq("private"),
            content::validation_status.eq("valid"),
            content::created_by.eq(created_by),
            content::created_at.eq("2026-05-11T00:00:00Z"),
            content::updated_at.eq("2026-05-11T00:00:00Z"),
        ))
        .execute(conn)
        .expect("insert content");
}

fn insert_inventory(conn: &mut diesel::SqliteConnection, blob_hash: &str, peer_id: &str) {
    use elohim_storage::db::diesel_schema::peer_blob_inventory;
    diesel::insert_or_ignore_into(peer_blob_inventory::table)
        .values(&NewPeerBlobInventoryRow {
            peer_id: peer_id.to_string(),
            blob_hash: blob_hash.to_string(),
            last_seen_at: "2026-05-11T12:00:00Z".to_string(),
            source: "gossip-snapshot".to_string(),
            sequence: 1,
            blake3_hash: None,
        })
        .execute(conn)
        .expect("insert inventory");
}

fn insert_binding(conn: &mut diesel::SqliteConnection, agent_cid: &str, peer_id: &str) {
    use elohim_storage::db::diesel_schema::peer_identity_bindings;
    diesel::insert_or_ignore_into(peer_identity_bindings::table)
        .values(&NewPeerIdentityBindingRow {
            peer_id: peer_id.to_string(),
            agent_cid: agent_cid.to_string(),
            dht_anchor_hash: format!("anchor-{peer_id}"),
            valid_from: "2026-01-01T00:00:00Z".to_string(),
            valid_until: None,
            observed_at: "2026-01-01T00:00:00Z".to_string(),
            source: "dht".to_string(),
            device_archetype: "desktop".to_string(),
            superseded_by: None,
        })
        .execute(conn)
        .expect("insert binding");
}

#[tokio::test]
async fn peer_topology_emits_resilience_cliff_when_sole_replica() {
    let pool = test_pool();
    let federator = Federator::new(P2PHandle::for_testing());

    {
        let mut conn = pool.get().expect("conn");
        // matthew has a binding
        insert_binding(&mut conn, "agent-matthew-pt4", "12D3KooWmatthew-pt4");
        // jessica has a binding
        insert_binding(&mut conn, "agent-jessica-pt4", "12D3KooWjessica-pt4");
        // cidA is authored by matthew and has only ONE replica (held by jessica's peer)
        insert_content_with_creator(&mut conn, "sha256-cidA-pt4", "agent-matthew-pt4");
        insert_inventory(&mut conn, "sha256-cidA-pt4", "12D3KooWjessica-pt4");
    }

    let view = aggregate_peer_topology_view(&pool, &federator, "agent-matthew-pt4")
        .await
        .expect("aggregate");

    assert!(
        !view.resilience_cliffs.is_empty(),
        "expected at least one resilience cliff (cidA has sole replica jessica)"
    );
    let cliff = &view.resilience_cliffs[0];
    assert_eq!(cliff.sole_replica_cid_count, 1);
}

#[tokio::test]
async fn peer_topology_no_cliffs_when_multiple_replicas() {
    let pool = test_pool();
    let federator = Federator::new(P2PHandle::for_testing());

    {
        let mut conn = pool.get().expect("conn");
        insert_binding(&mut conn, "agent-matthew-pt4b", "12D3KooWmatthew-pt4b");
        insert_binding(&mut conn, "agent-jessica-pt4b", "12D3KooWjessica-pt4b");
        insert_binding(&mut conn, "agent-adam-pt4b", "12D3KooWadam-pt4b");
        // cidB has 2 replicas — no cliff
        insert_content_with_creator(&mut conn, "sha256-cidB-pt4", "agent-matthew-pt4b");
        insert_inventory(&mut conn, "sha256-cidB-pt4", "12D3KooWjessica-pt4b");
        insert_inventory(&mut conn, "sha256-cidB-pt4", "12D3KooWadam-pt4b");
    }

    let view = aggregate_peer_topology_view(&pool, &federator, "agent-matthew-pt4b")
        .await
        .expect("aggregate");

    assert!(
        view.resilience_cliffs.is_empty(),
        "2+ replicas should produce no cliffs"
    );
}
