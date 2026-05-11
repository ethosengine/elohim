//! Phase 4 T7 — tests for device_capacity::available_bytes_for.

use diesel::prelude::*;
use elohim_storage::db::models::NewPeerIdentityBindingRow;
use elohim_storage::services::device_capacity::{available_bytes_for, override_total_for_test};
use elohim_storage::test_util::test_pool;

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

fn insert_commitment(conn: &mut diesel::SqliteConnection, provider: &str, qty: f32) {
    use elohim_storage::db::diesel_schema::rea_commitments;
    diesel::insert_or_ignore_into(rea_commitments::table)
        .values((
            rea_commitments::id.eq(format!("cmt-{provider}-{}", uuid::Uuid::new_v4())),
            rea_commitments::h_app_id.eq("lamad"),
            rea_commitments::action.eq("custody-blob"),
            rea_commitments::provider.eq(provider),
            rea_commitments::receiver.eq("agent-other"),
            rea_commitments::resource_quantity_value.eq(qty),
            rea_commitments::resource_quantity_unit.eq("bytes"),
            rea_commitments::state.eq("active"),
            rea_commitments::finished.eq(0i32),
        ))
        .execute(conn)
        .expect("insert commitment");
}

#[tokio::test]
async fn available_equals_total_minus_committed() {
    let pool = test_pool();
    {
        let mut conn = pool.get().expect("conn");
        insert_binding(&mut conn, "human-matthew-dc7", "12D3KooWdc7");
        insert_commitment(&mut conn, "12D3KooWdc7", 300_000.0);
    }

    override_total_for_test("human-matthew-dc7", 1_000_000);
    let available = available_bytes_for(&pool, "human-matthew-dc7").await;
    assert_eq!(available, 700_000);
}

#[tokio::test]
async fn available_returns_zero_when_no_bindings() {
    let pool = test_pool();
    // No bindings, no commitments → committed = 0, total = 0 (no override) → available = 0.
    let available = available_bytes_for(&pool, "agent-nobody-dc7").await;
    assert_eq!(available, 0);
}
