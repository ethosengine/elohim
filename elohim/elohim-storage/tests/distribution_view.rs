//! Integration tests for `services::distribution_view` — both
//! `compose_distribution_summary` (T23) and `compose_distribution_details` (T24).
//!
//! Uses the shared `elohim_storage::test_util::test_pool` in-memory SQLite pattern.
//! All tests are async (tokio) because the compose functions are async.

use diesel::prelude::*;
use diesel::RunQueryDsl;
use elohim_storage::db::diesel_schema::{
    content, peer_blob_inventory, peer_identity_bindings, placement_gaps, rea_commitments,
};
use elohim_storage::db::models::{
    NewPeerBlobInventoryRow, NewPeerIdentityBindingRow, NewPlacementGap, NewReaCommitment,
};
use elohim_storage::services::distribution_view::{
    compose_distribution_details, compose_distribution_summary, replica_health_for,
    replica_target_for, DistributionContext,
};
use elohim_storage::test_util::test_pool;
use elohim_storage::views::{
    DeviceArchetype, DiversityHint, MyRole, PlacementGapKind, ReachClass, ReplicaHealth,
};

// ============================================================================
// Pure function tests
// ============================================================================

#[test]
fn replica_health_thresholds() {
    // 14/14 = 1.0 >= 0.85 → Healthy
    assert_eq!(replica_health_for(14, 14), ReplicaHealth::Healthy);
    // 12/14 ≈ 0.857 >= 0.85 → Healthy
    assert_eq!(replica_health_for(12, 14), ReplicaHealth::Healthy);
    // 7/14 = 0.5 >= 0.50 → AtRisk
    assert_eq!(replica_health_for(7, 14), ReplicaHealth::AtRisk);
    // 3/14 ≈ 0.214 < 0.50 → Critical
    assert_eq!(replica_health_for(3, 14), ReplicaHealth::Critical);
    // 0/0 — vacuous → Healthy
    assert_eq!(replica_health_for(0, 0), ReplicaHealth::Healthy);
}

#[test]
fn replica_target_increases_with_reach() {
    let private = replica_target_for(&ReachClass::Private);
    let household = replica_target_for(&ReachClass::Household);
    let public = replica_target_for(&ReachClass::Public);
    assert!(
        private < household,
        "Private target ({private}) must be < Household target ({household})"
    );
    assert!(
        household < public,
        "Household target ({household}) must be < Public target ({public})"
    );
}

// ============================================================================
// Integration helpers
// ============================================================================

fn seed_inventory(conn: &mut SqliteConnection, peer_id: &str, blob_hash: &str) {
    diesel::insert_or_ignore_into(peer_blob_inventory::table)
        .values(&NewPeerBlobInventoryRow {
            peer_id: peer_id.to_string(),
            blob_hash: blob_hash.to_string(),
            last_seen_at: "2026-04-30T12:00:00Z".to_string(),
            source: "gossip-snapshot".to_string(),
            sequence: 1,
            blake3_hash: None,
        })
        .execute(conn)
        .expect("seed inventory");
}

fn seed_content_with_reach(conn: &mut SqliteConnection, blob_hash: &str, reach: &str) {
    diesel::insert_or_ignore_into(content::table)
        .values((
            content::id.eq(format!("content-{blob_hash}")),
            content::h_app_id.eq("lamad"),
            content::title.eq("Test content"),
            content::content_type.eq("concept"),
            content::content_format.eq("markdown"),
            content::blob_hash.eq(blob_hash),
            content::reach.eq(reach),
            content::validation_status.eq("valid"),
            content::created_at.eq("2026-04-30T00:00:00Z"),
            content::updated_at.eq("2026-04-30T00:00:00Z"),
        ))
        .execute(conn)
        .expect("seed content");
}

fn seed_binding(conn: &mut SqliteConnection, peer_id: &str, agent_cid: &str) {
    diesel::insert_or_ignore_into(peer_identity_bindings::table)
        .values(&NewPeerIdentityBindingRow {
            peer_id: peer_id.to_string(),
            agent_cid: agent_cid.to_string(),
            dht_anchor_hash: format!("anchor-{peer_id}"),
            valid_from: "2026-04-01T00:00:00Z".to_string(),
            valid_until: None,
            observed_at: "2026-04-01T00:00:00Z".to_string(),
            source: "dht".to_string(),
            device_archetype: "desktop".to_string(),
            superseded_by: None,
        })
        .execute(conn)
        .expect("seed binding");
}

fn seed_rea_commitment(
    conn: &mut SqliteConnection,
    id: &str,
    provider: &str,
    receiver: &str,
    qty: Option<f32>,
) {
    diesel::insert_or_ignore_into(rea_commitments::table)
        .values(&NewReaCommitment {
            id,
            h_app_id: "lamad",
            action: "custody-blob",
            provider,
            receiver,
            resource_conforms_to: None,
            resource_classified_as: None,
            resource_quantity_value: qty,
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
        .execute(conn)
        .expect("seed rea_commitment");
}

// ============================================================================
// Integration tests
// ============================================================================

#[tokio::test]
async fn summary_visitor_no_my_role() {
    let pool = test_pool();
    {
        let mut conn = pool.get().unwrap();
        let hash = "hash_pub";
        seed_content_with_reach(&mut conn, hash, "public");
        seed_inventory(&mut conn, "peer-A", hash);
        seed_inventory(&mut conn, "peer-B", hash);
        seed_inventory(&mut conn, "peer-C", hash);
    }

    let summary = compose_distribution_summary(&pool, "hash_pub", DistributionContext::Visitor)
        .await
        .expect("compose should succeed");

    assert_eq!(summary.replica_count, 3);
    assert_eq!(summary.reach_class, ReachClass::Public);
    assert!(summary.my_role.is_none(), "Visitor must have no my_role");
    assert!(
        summary.reciprocity_hint.is_none(),
        "Visitor must have no reciprocity_hint"
    );
    assert_eq!(summary.projector_count, 0);
    assert_eq!(summary.diversity_hint, DiversityHint::None);
}

#[tokio::test]
async fn summary_steward_replica_role() {
    let pool = test_pool();
    let agent_cid = "agent-matthew";
    let my_peer = "peer_M_desktop";
    {
        let mut conn = pool.get().unwrap();
        let hash = "hash_x";
        seed_content_with_reach(&mut conn, hash, "household");
        // Seed 3 replicas; one is ours
        seed_inventory(&mut conn, my_peer, hash);
        seed_inventory(&mut conn, "peer-other-1", hash);
        seed_inventory(&mut conn, "peer-other-2", hash);
        seed_binding(&mut conn, my_peer, agent_cid);
    }

    let pool_ref = &pool;
    let bindings: Vec<_> = {
        let mut conn = pool_ref.get().unwrap();
        use elohim_storage::db::diesel_schema::peer_identity_bindings::dsl as pib;
        pib::peer_identity_bindings
            .filter(pib::agent_cid.eq(agent_cid))
            .load::<elohim_storage::db::models::PeerIdentityBindingRow>(&mut conn)
            .unwrap()
    };

    let summary = compose_distribution_summary(
        pool_ref,
        "hash_x",
        DistributionContext::Steward {
            agent_cid,
            bindings: &bindings,
        },
    )
    .await
    .expect("compose should succeed");

    // 3 replicas, one is mine, no projector data → Replica (not SoleReplica)
    assert_eq!(summary.my_role, Some(MyRole::Replica));
}

#[tokio::test]
async fn summary_steward_sole_replica_when_only_replica() {
    let pool = test_pool();
    let agent_cid = "agent-sole";
    let my_peer = "peer_M_only";
    {
        let mut conn = pool.get().unwrap();
        let hash = "hash_y";
        seed_content_with_reach(&mut conn, hash, "intimate");
        seed_inventory(&mut conn, my_peer, hash);
        seed_binding(&mut conn, my_peer, agent_cid);
    }

    let bindings: Vec<_> = {
        let mut conn = pool.get().unwrap();
        use elohim_storage::db::diesel_schema::peer_identity_bindings::dsl as pib;
        pib::peer_identity_bindings
            .filter(pib::agent_cid.eq(agent_cid))
            .load::<elohim_storage::db::models::PeerIdentityBindingRow>(&mut conn)
            .unwrap()
    };

    let summary = compose_distribution_summary(
        &pool,
        "hash_y",
        DistributionContext::Steward {
            agent_cid,
            bindings: &bindings,
        },
    )
    .await
    .expect("compose should succeed");

    assert_eq!(summary.my_role, Some(MyRole::SoleReplica));
}

#[tokio::test]
async fn summary_steward_not_hosting_when_no_overlap() {
    let pool = test_pool();
    let agent_cid = "agent-ghost";
    let my_peer = "peer_ghost";
    {
        let mut conn = pool.get().unwrap();
        let hash = "hash_z";
        seed_content_with_reach(&mut conn, hash, "community");
        seed_inventory(&mut conn, "peer-remote-1", hash);
        seed_inventory(&mut conn, "peer-remote-2", hash);
        // my_peer is NOT in the inventory for hash_z
        seed_binding(&mut conn, my_peer, agent_cid);
    }

    let bindings: Vec<_> = {
        let mut conn = pool.get().unwrap();
        use elohim_storage::db::diesel_schema::peer_identity_bindings::dsl as pib;
        pib::peer_identity_bindings
            .filter(pib::agent_cid.eq(agent_cid))
            .load::<elohim_storage::db::models::PeerIdentityBindingRow>(&mut conn)
            .unwrap()
    };

    let summary = compose_distribution_summary(
        &pool,
        "hash_z",
        DistributionContext::Steward {
            agent_cid,
            bindings: &bindings,
        },
    )
    .await
    .expect("compose should succeed");

    assert_eq!(summary.my_role, Some(MyRole::NotHosting));
}

#[tokio::test]
async fn summary_unknown_blob_defaults_to_private_zero_replicas() {
    let pool = test_pool();
    // No content row, no inventory rows for this hash.

    let summary =
        compose_distribution_summary(&pool, "hash_nonexistent_xyz", DistributionContext::Visitor)
            .await
            .expect("compose should succeed even for unknown blob");

    assert_eq!(summary.replica_count, 0, "zero replicas for unknown blob");
    assert_eq!(
        summary.reach_class,
        ReachClass::Private,
        "default to Private"
    );
    // Private target = 2, count = 0 → ratio 0.0 < 0.5 → Critical
    assert_eq!(
        summary.replica_health,
        ReplicaHealth::Critical,
        "0/2 < 0.5 → Critical"
    );
    assert_eq!(summary.replica_target, 2);
}

#[tokio::test]
async fn reciprocity_hint_steward_outflow_only() {
    let pool = test_pool();
    let agent_cid = "agent-outflow";
    let my_peer = "peer_M_desktop_outflow";
    {
        let mut conn = pool.get().unwrap();
        // Seed commitments: provider = my_peer, total outflow = 150.0
        seed_rea_commitment(&mut conn, "cmt-out-1", my_peer, "peer-remote", Some(100.0));
        seed_rea_commitment(&mut conn, "cmt-out-2", my_peer, "peer-remote", Some(50.0));
        // No inflow commitments where receiver = my_peer
        seed_binding(&mut conn, my_peer, agent_cid);
    }

    let bindings: Vec<_> = {
        let mut conn = pool.get().unwrap();
        use elohim_storage::db::diesel_schema::peer_identity_bindings::dsl as pib;
        pib::peer_identity_bindings
            .filter(pib::agent_cid.eq(agent_cid))
            .load::<elohim_storage::db::models::PeerIdentityBindingRow>(&mut conn)
            .unwrap()
    };

    let summary = compose_distribution_summary(
        &pool,
        "hash_any_blob_outflow",
        DistributionContext::Steward {
            agent_cid,
            bindings: &bindings,
        },
    )
    .await
    .expect("compose should succeed");

    // outflow = 150.0, inflow = 0.0 → reciprocity_hint = Some(150)
    assert_eq!(
        summary.reciprocity_hint,
        Some(150),
        "outflow-only reciprocity should be 150"
    );
}

// ============================================================================
// T24 seeding helpers
// ============================================================================

fn seed_placement_gap(conn: &mut SqliteConnection, id: &str, shard_hash: &str) {
    diesel::insert_or_ignore_into(placement_gaps::table)
        .values(&NewPlacementGap {
            id,
            content_id: &format!("content-{shard_hash}"),
            shard_hash,
            h_app_id: "lamad",
            requested_steward_count: 6,
            achieved_steward_count: 2,
            contract_coverage: 0.33,
            gap_kind: "under-replicated",
            first_seen_at: "2026-05-01T00:00:00Z",
            last_seen_at: "2026-05-02T00:00:00Z",
        })
        .execute(conn)
        .expect("seed placement_gap");
}

fn seed_rea_commitment_for_blob(
    conn: &mut SqliteConnection,
    id: &str,
    blob_hash: &str,
    provider: &str,
    receiver: &str,
) {
    diesel::insert_or_ignore_into(rea_commitments::table)
        .values(&NewReaCommitment {
            id,
            h_app_id: "lamad",
            action: "custody-blob",
            provider,
            receiver,
            resource_conforms_to: None,
            resource_classified_as: Some(blob_hash),
            resource_quantity_value: Some(1024.0),
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
        .execute(conn)
        .expect("seed rea_commitment_for_blob");
}

fn seed_binding_with_archetype(
    conn: &mut SqliteConnection,
    peer_id: &str,
    agent_cid: &str,
    archetype: &str,
) {
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
        .execute(conn)
        .expect("seed binding with archetype");
}

fn load_bindings_for_agent(
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
// T24 integration tests — compose_distribution_details
// ============================================================================

#[tokio::test]
async fn details_includes_full_replica_peers() {
    let pool = test_pool();
    let hash = "hash_details_5peers";
    {
        let mut conn = pool.get().unwrap();
        seed_content_with_reach(&mut conn, hash, "neighborhood");
        for i in 0..5 {
            seed_inventory(&mut conn, &format!("peer-det-{i}"), hash);
        }
    }

    let details = compose_distribution_details(&pool, hash, DistributionContext::Visitor)
        .await
        .expect("compose_distribution_details should succeed");

    assert_eq!(
        details.summary.replica_count, 5,
        "summary replica_count == 5"
    );
    assert_eq!(details.replica_peers.len(), 5, "replica_peers.len() == 5");
    // All archetypes must be valid DeviceArchetype variants (no binding rows → Node)
    for peer in &details.replica_peers {
        assert_eq!(
            peer.device_archetype,
            DeviceArchetype::Node,
            "no binding → Node default"
        );
    }
}

#[tokio::test]
async fn details_replica_peers_pick_up_archetype_from_bindings() {
    let pool = test_pool();
    let hash = "hash_archetype_pickup";
    let agent_cid = "agent-archetype-test";
    {
        let mut conn = pool.get().unwrap();
        seed_content_with_reach(&mut conn, hash, "household");
        // Seed peers with different archetypes via bindings
        seed_inventory(&mut conn, "peer-arch-desktop", hash);
        seed_inventory(&mut conn, "peer-arch-mobile", hash);
        seed_inventory(&mut conn, "peer-arch-node", hash);
        seed_binding_with_archetype(&mut conn, "peer-arch-desktop", agent_cid, "desktop");
        seed_binding_with_archetype(&mut conn, "peer-arch-mobile", agent_cid, "mobile");
        seed_binding_with_archetype(&mut conn, "peer-arch-node", agent_cid, "node");
    }

    let details = compose_distribution_details(&pool, hash, DistributionContext::Visitor)
        .await
        .expect("compose should succeed");

    assert_eq!(details.replica_peers.len(), 3);

    let by_id: std::collections::HashMap<_, _> = details
        .replica_peers
        .iter()
        .map(|p| (p.peer_id.as_str(), &p.device_archetype))
        .collect();

    assert_eq!(by_id["peer-arch-desktop"], &DeviceArchetype::Desktop);
    assert_eq!(by_id["peer-arch-mobile"], &DeviceArchetype::Mobile);
    assert_eq!(by_id["peer-arch-node"], &DeviceArchetype::Node);
}

#[tokio::test]
async fn details_replica_peers_default_to_node_without_binding() {
    let pool = test_pool();
    let hash = "hash_no_binding_node_default";
    {
        let mut conn = pool.get().unwrap();
        seed_content_with_reach(&mut conn, hash, "private");
        seed_inventory(&mut conn, "peer-unbound-1", hash);
        seed_inventory(&mut conn, "peer-unbound-2", hash);
        // No peer_identity_bindings rows for these peers
    }

    let details = compose_distribution_details(&pool, hash, DistributionContext::Visitor)
        .await
        .expect("compose should succeed");

    assert_eq!(details.replica_peers.len(), 2);
    for peer in &details.replica_peers {
        assert_eq!(
            peer.device_archetype,
            DeviceArchetype::Node,
            "peers without binding row must default to Node"
        );
    }
}

#[tokio::test]
async fn details_visitor_no_commitment_references() {
    let pool = test_pool();
    let hash = "hash_visitor_no_refs";
    {
        let mut conn = pool.get().unwrap();
        seed_content_with_reach(&mut conn, hash, "community");
        seed_inventory(&mut conn, "peer-x", hash);
    }

    let details = compose_distribution_details(&pool, hash, DistributionContext::Visitor)
        .await
        .expect("compose should succeed");

    assert!(
        details.commitment_references.is_none(),
        "Visitor context must yield None commitment_references"
    );
}

#[tokio::test]
async fn details_steward_includes_commitment_references() {
    let pool = test_pool();
    let hash = "hash_test";
    let agent_cid = "agent-steward-refs";
    let my_peer = "peer_M_desktop";
    {
        let mut conn = pool.get().unwrap();
        seed_content_with_reach(&mut conn, hash, "household");
        seed_inventory(&mut conn, my_peer, hash);
        seed_binding(&mut conn, my_peer, agent_cid);
        seed_rea_commitment_for_blob(&mut conn, "commitment_id", hash, my_peer, "peer-other");
    }

    let bindings = load_bindings_for_agent(&pool, agent_cid);
    let details = compose_distribution_details(
        &pool,
        hash,
        DistributionContext::Steward {
            agent_cid,
            bindings: &bindings,
        },
    )
    .await
    .expect("compose should succeed");

    assert_eq!(
        details.commitment_references,
        Some(vec!["commitment_id".to_string()]),
        "steward must see their commitment IDs"
    );
}

/// C4: `load_placement_gaps_for` now emits real `HubDiversity` gap rows via
/// `hub_summary`. Content replicated to only one peer (one computed hub) falls
/// below the C4 target of 2 hubs → a gap row must be present.
#[tokio::test]
async fn details_includes_placement_gaps_for_blob() {
    let pool = test_pool();
    let hash = "hash_gap_test";
    {
        let mut conn = pool.get().unwrap();
        seed_content_with_reach(&mut conn, hash, "collective");
        // One peer, no identity binding → one computed hub → below target of 2.
        seed_inventory(&mut conn, "peer-gap-1", hash);
        seed_placement_gap(&mut conn, "gap-001", hash);
    }

    let details = compose_distribution_details(&pool, hash, DistributionContext::Visitor)
        .await
        .expect("compose should succeed");

    // C4: gap row emitted because observed hubs (1) < target (2).
    let hub_gap = details
        .placement_gaps
        .iter()
        .find(|g| matches!(g.kind, PlacementGapKind::HubDiversity));
    assert!(
        hub_gap.is_some(),
        "expected a HubDiversity gap for single-hub content; gaps: {:#?}",
        details.placement_gaps
    );
    let gap = hub_gap.unwrap();
    assert_eq!(gap.content_id, hash);
    assert!(
        gap.shortfall.observed < gap.shortfall.target,
        "observed ({}) must be less than target ({})",
        gap.shortfall.observed,
        gap.shortfall.target
    );
}

#[tokio::test]
async fn details_projector_identities_stubbed_empty() {
    let pool = test_pool();
    let hash = "hash_projector_stub";
    {
        let mut conn = pool.get().unwrap();
        seed_content_with_reach(&mut conn, hash, "private");
        seed_inventory(&mut conn, "peer-proj-1", hash);
    }

    let details = compose_distribution_details(&pool, hash, DistributionContext::Visitor)
        .await
        .expect("compose should succeed");

    assert!(
        details.projector_identities.is_empty(),
        "projector_identities must be empty when no ack-projection events have been recorded"
    );
}

#[tokio::test]
async fn details_recent_projection_events_stubbed_empty() {
    let pool = test_pool();
    let hash = "hash_proj_events_stub";
    {
        let mut conn = pool.get().unwrap();
        seed_content_with_reach(&mut conn, hash, "private");
        seed_inventory(&mut conn, "peer-pev-1", hash);
    }

    let details = compose_distribution_details(&pool, hash, DistributionContext::Visitor)
        .await
        .expect("compose should succeed");

    assert!(
        details.recent_projection_events.is_empty(),
        "recent_projection_events must be empty when no ack-projection events have been recorded"
    );
}

// ============================================================================
// C4 hub-diversity gap helpers
// ============================================================================

fn seed_human_with_household(
    conn: &mut SqliteConnection,
    human_id: &str,
    agent_pub_key: &str,
    household_id: &str,
) {
    // Uses raw SQL to avoid depending on NewHuman's full required-field list.
    diesel::sql_query(format!(
        "INSERT OR REPLACE INTO humans \
         (id, display_name, affinities, profile_reach, h_app_id, created_at, updated_at, \
          agent_pub_key, household_id) \
         VALUES ('{human_id}', 'Test {human_id}', '[]', 'commons', 'lamad', \
                 '2026-05-20T12:00:00Z', '2026-05-20T12:00:00Z', \
                 '{agent_pub_key}', '{household_id}')",
    ))
    .execute(conn)
    .expect("seed human with household");
}

/// Seed a single peer into `peer_blob_inventory` with no identity binding.
/// The peer will be classified as a single `Computed` hub → 1 hub < 2 target → gap emitted.
async fn seed_content_with_single_hub(pool: &elohim_storage::db::DbPool, blob_hash: &str) {
    let mut conn = pool.get().unwrap();
    seed_content_with_reach(&mut conn, blob_hash, "community");
    // One peer, no binding → hub_summary returns one Computed hub.
    seed_inventory(&mut conn, "peer-single-hub-1", blob_hash);
}

/// Seed content replicated across `hub_count` distinct households.
/// Each household contributes one peer. hub_count must be >= 1.
async fn seed_content_with_multi_hub(
    pool: &elohim_storage::db::DbPool,
    blob_hash: &str,
    hub_count: usize,
) {
    let mut conn = pool.get().unwrap();
    seed_content_with_reach(&mut conn, blob_hash, "community");
    for i in 0..hub_count {
        let peer_id = format!("peer-multi-hub-{blob_hash}-{i}");
        let agent_cid = format!("agent-multi-hub-{blob_hash}-{i}");
        let human_id = format!("human-multi-hub-{blob_hash}-{i}");
        let household_id = format!("household-multi-hub-{blob_hash}-{i}");
        seed_inventory(&mut conn, &peer_id, blob_hash);
        seed_binding(&mut conn, &peer_id, &agent_cid);
        seed_human_with_household(&mut conn, &human_id, &agent_cid, &household_id);
    }
}

// ============================================================================
// C4 hub-diversity gap tests
// ============================================================================

/// A content item replicated to a single peer (one computed hub) is below the
/// C4 floor of 2 hubs. A `HubDiversity` gap row must be emitted with
/// `observed < target` and `content_id` matching the blob hash.
#[tokio::test]
async fn distribution_details_emits_hub_diversity_gap_when_single_hub() {
    let pool = test_pool();
    let blob_hash = "hash-c4-single-hub-lonely";

    seed_content_with_single_hub(&pool, blob_hash).await;

    let details = compose_distribution_details(&pool, blob_hash, DistributionContext::Visitor)
        .await
        .expect("compose_distribution_details should succeed");

    let hub_diversity = details
        .placement_gaps
        .iter()
        .find(|g| matches!(g.kind, PlacementGapKind::HubDiversity))
        .expect("expected a HubDiversity gap for single-hub content");

    assert!(
        hub_diversity.shortfall.observed < hub_diversity.shortfall.target,
        "observed ({}) must be less than target ({}) for a real gap",
        hub_diversity.shortfall.observed,
        hub_diversity.shortfall.target
    );
    assert_eq!(
        hub_diversity.content_id, blob_hash,
        "content_id must match the queried blob hash"
    );
    assert!(
        hub_diversity.remediation.is_some(),
        "remediation hint must be present"
    );
}

/// A content item replicated across 3 distinct households meets the C4 hub floor.
/// No `HubDiversity` gap row should be emitted.
#[tokio::test]
async fn distribution_details_no_gap_when_multi_hub() {
    let pool = test_pool();
    let blob_hash = "hash-c4-multi-hub-spread";

    seed_content_with_multi_hub(&pool, blob_hash, 3).await;

    let details = compose_distribution_details(&pool, blob_hash, DistributionContext::Visitor)
        .await
        .expect("compose_distribution_details should succeed");

    let hub_diversity = details
        .placement_gaps
        .iter()
        .find(|g| matches!(g.kind, PlacementGapKind::HubDiversity));

    assert!(
        hub_diversity.is_none(),
        "expected no HubDiversity gap when content is replicated across 3 hubs; gaps: {:#?}",
        details.placement_gaps
    );
}
