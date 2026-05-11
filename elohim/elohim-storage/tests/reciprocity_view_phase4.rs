//! Phase 4 T10 — reciprocity_view Phase 4 helper integration.
//!
//! Verifies:
//! 1. display_name is resolved from the humans table per counterparty row.
//! 2. online is set based on the connected_peers snapshot.
//! 3. capacity_available_bytes reflects device_capacity::available_bytes_for.

use std::collections::HashSet;

use diesel::prelude::*;
use elohim_storage::db::models::NewPeerIdentityBindingRow;
use elohim_storage::db::peer_identity_bindings;
use elohim_storage::services::device_capacity::override_total_for_test;
use elohim_storage::services::reciprocity_view::aggregate_reciprocity_view;
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

fn insert_commitment(
    conn: &mut diesel::SqliteConnection,
    provider: &str,
    receiver: &str,
    qty: f32,
) {
    use elohim_storage::db::diesel_schema::rea_commitments;
    diesel::insert_or_ignore_into(rea_commitments::table)
        .values((
            rea_commitments::id.eq(format!(
                "cmt-{provider}-{receiver}-{}",
                uuid::Uuid::new_v4()
            )),
            rea_commitments::h_app_id.eq("lamad"),
            rea_commitments::action.eq("custody-blob"),
            rea_commitments::provider.eq(provider),
            rea_commitments::receiver.eq(receiver),
            rea_commitments::resource_quantity_value.eq(qty),
            rea_commitments::resource_quantity_unit.eq("bytes"),
            rea_commitments::state.eq("active"),
            rea_commitments::finished.eq(0i32),
        ))
        .execute(conn)
        .expect("insert commitment");
}

#[tokio::test]
async fn reciprocity_view_resolves_display_name_and_online() {
    let pool = test_pool();

    {
        let mut conn = pool.get().expect("conn");
        insert_human(&mut conn, "agent-jessica-rv4", "Jessica");
        insert_binding(
            &mut conn,
            "agent-jessica-rv4",
            "12D3KooWjessica-rv4",
            "desktop",
        );
        insert_binding(
            &mut conn,
            "agent-matthew-rv4",
            "12D3KooWmatthew-rv4",
            "desktop",
        );
        // matthew commits to provide to jessica's peer
        insert_commitment(
            &mut conn,
            "12D3KooWmatthew-rv4",
            "agent-jessica-rv4",
            100_000.0,
        );
    }

    let bindings = {
        let mut conn = pool.get().expect("conn");
        peer_identity_bindings::list_active_for_agent(
            &mut conn,
            "agent-matthew-rv4",
            &chrono::Utc::now().to_rfc3339(),
        )
        .expect("bindings")
    };

    let connected: HashSet<String> = ["12D3KooWjessica-rv4".into()].into();

    let view = aggregate_reciprocity_view(&pool, "agent-matthew-rv4", &bindings, &connected)
        .await
        .expect("aggregate");

    assert!(!view.outflow.is_empty(), "expected outflow row to jessica");
    let row = &view.outflow[0];
    assert_eq!(
        row.display_name,
        Some("Jessica".to_string()),
        "display_name should be resolved"
    );
    assert_eq!(row.online, Some(true), "jessica's peer is in connected set");
}

#[tokio::test]
async fn reciprocity_capacity_returns_total_minus_committed() {
    let pool = test_pool();

    {
        let mut conn = pool.get().expect("conn");
        insert_binding(
            &mut conn,
            "agent-matthew-cap",
            "12D3KooWmatthew-cap",
            "desktop",
        );
        insert_commitment(&mut conn, "12D3KooWmatthew-cap", "agent-other", 250_000.0);
    }

    override_total_for_test("agent-matthew-cap", 1_000_000);

    let bindings = {
        let mut conn = pool.get().expect("conn");
        peer_identity_bindings::list_active_for_agent(
            &mut conn,
            "agent-matthew-cap",
            &chrono::Utc::now().to_rfc3339(),
        )
        .unwrap()
    };

    let view = aggregate_reciprocity_view(&pool, "agent-matthew-cap", &bindings, &HashSet::new())
        .await
        .unwrap();

    assert_eq!(view.capacity_available_bytes, 750_000);
}
