//! Phase 4 T13 — end-to-end projector ack → topology surfaces integration test.
//!
//! Closure test: coordinator emits ack-projection EconomicEvent →
//! projection_events row appears → distribution_view returns
//! projector_count + diversity_hint + projector_identities
//! + recent_projection_events.
//!
//! Source of truth: DHT EconomicEvent entry (Category A). All assertions
//! here verify the operational (Category C) projection is correctly derived.

use diesel::prelude::*;
use elohim_storage::db::models::NewPeerBlobInventoryRow;
use elohim_storage::db::models::NewPeerIdentityBindingRow;
use elohim_storage::p2p::projection_ack_handler::handle_projection_ack;
use elohim_storage::services::distribution_view::{
    compose_distribution_details, compose_distribution_summary, DistributionContext,
};
use elohim_storage::test_util::test_pool;

fn insert_human(conn: &mut diesel::SqliteConnection, id: &str, name: &str) {
    use elohim_storage::db::diesel_schema::humans::dsl as h;
    diesel::insert_or_ignore_into(h::humans)
        .values((
            h::id.eq(id),
            h::display_name.eq(name),
            h::affinities.eq("[]"),
            h::profile_reach.eq("private"),
            h::h_app_id.eq("lamad"),
            h::created_at.eq("2026-05-11T00:00:00Z"),
            h::updated_at.eq("2026-05-11T00:00:00Z"),
        ))
        .execute(conn)
        .expect("insert human");
}

fn insert_binding(
    conn: &mut diesel::SqliteConnection,
    agent_cid: &str,
    peer_id: &str,
    archetype: &str,
) {
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
            device_archetype: archetype.to_string(),
            superseded_by: None,
        })
        .execute(conn)
        .expect("insert binding");
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

#[tokio::test]
async fn end_to_end_projector_ack_surfaces_in_distribution_view() {
    let pool = test_pool();

    // Seed: humans + bindings with mixed archetypes + 3 replicas
    {
        let mut conn = pool.get().expect("conn");
        insert_human(&mut conn, "agent-matthew-e2e", "Matthew");
        insert_human(&mut conn, "agent-doorway-e2e", "Doorway 1");
        insert_binding(&mut conn, "agent-replica-a", "12D3KooWa-e2e", "desktop");
        insert_binding(&mut conn, "agent-replica-b", "12D3KooWb-e2e", "mobile");
        insert_binding(&mut conn, "agent-replica-c", "12D3KooWc-e2e", "steward");
        insert_inventory(&mut conn, "sha256-blobX-e2e", "12D3KooWa-e2e");
        insert_inventory(&mut conn, "sha256-blobX-e2e", "12D3KooWb-e2e");
        insert_inventory(&mut conn, "sha256-blobX-e2e", "12D3KooWc-e2e");
    }

    // Doorway acks projection.
    handle_projection_ack(
        &pool,
        "ack-projection",
        "agent-doorway-e2e",
        "sha256-blobX-e2e",
        "u-action-X-e2e",
        "2026-05-11T12:00:00Z",
    )
    .await
    .unwrap();

    // Compose Visitor distribution summary.
    let summary =
        compose_distribution_summary(&pool, "sha256-blobX-e2e", DistributionContext::Visitor)
            .await
            .unwrap();

    assert_eq!(summary.replica_count, 3, "3 replicas in inventory");
    assert_eq!(summary.projector_count, 1, "1 distinct doorway acked");

    // diversity_hint should reflect the 3 archetypes (desktop/mobile/steward).
    match &summary.diversity_hint {
        elohim_storage::views::DiversityHint::HouseholdArchetypes(archetypes) => {
            assert!(
                archetypes.len() >= 2,
                "expected ≥2 distinct archetypes, got {:?}",
                archetypes
            );
        }
        elohim_storage::views::DiversityHint::None => {
            panic!("expected HouseholdArchetypes diversity, got None");
        }
        other => {
            panic!("unexpected diversity hint variant: {:?}", other);
        }
    }

    // Compose details — projector_identities should reflect the acking doorway.
    let details = compose_distribution_details(
        &pool,
        "sha256-blobX-e2e",
        DistributionContext::Visitor,
    )
    .await
    .unwrap();

    assert_eq!(
        details.projector_identities.len(),
        1,
        "1 projector identity"
    );
    assert_eq!(
        details.projector_identities[0].doorway_hostname,
        "agent-doorway-e2e"
    );
    assert_eq!(
        details.recent_projection_events.len(),
        1,
        "1 recent projection event"
    );
}
