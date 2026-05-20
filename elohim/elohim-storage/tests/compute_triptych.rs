//! TDD tests for `services::reciprocity_view::aggregate_stewarded_bytes_by_peer`.
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
