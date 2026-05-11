//! Phase 4 T1 — peer_identity_bindings device_archetype + superseded_by column tests.
//!
//! Migration 2026-05-01-120000_peer_identity_bindings_archetype_superseded already
//! applied these columns. These tests confirm the projection works end-to-end with
//! the full Diesel model.

use chrono::Utc;
use elohim_storage::db::models::NewPeerIdentityBindingRow;
use elohim_storage::db::peer_identity_bindings;
use elohim_storage::test_util::test_pool;

fn now_iso() -> String {
    Utc::now().to_rfc3339()
}

fn binding(
    peer_id: &str,
    agent_cid: &str,
    dht_anchor_hash: &str,
    device_archetype: &str,
    superseded_by: Option<&str>,
) -> NewPeerIdentityBindingRow {
    NewPeerIdentityBindingRow {
        peer_id: peer_id.to_string(),
        agent_cid: agent_cid.to_string(),
        dht_anchor_hash: dht_anchor_hash.to_string(),
        valid_from: "2026-01-01T00:00:00Z".to_string(),
        valid_until: None,
        observed_at: now_iso(),
        source: "dht".to_string(),
        device_archetype: device_archetype.to_string(),
        superseded_by: superseded_by.map(|s| s.to_string()),
    }
}

#[test]
fn binding_carries_device_archetype() {
    let pool = test_pool();
    let mut conn = pool.get().expect("conn");

    peer_identity_bindings::upsert(
        &mut conn,
        &binding(
            "12D3KooWtest1",
            "agent-matthew-desktop",
            "u-action-hash-1",
            "desktop",
            None,
        ),
    )
    .expect("insert");

    let active = peer_identity_bindings::list_active_for_agent(
        &mut conn,
        "agent-matthew-desktop",
        &now_iso(),
    )
    .expect("list");
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].device_archetype, "desktop");
    assert_eq!(active[0].superseded_by, None);
}

#[test]
fn superseded_binding_is_excluded_from_list_active() {
    let pool = test_pool();
    let mut conn = pool.get().expect("conn");

    // Old binding, superseded_by pointing at new anchor hash.
    peer_identity_bindings::upsert(
        &mut conn,
        &binding(
            "12D3KooWold",
            "agent-matthew-desktop",
            "u-action-hash-old",
            "desktop",
            Some("u-action-hash-new"),
        ),
    )
    .expect("insert old");

    // New binding, not superseded.
    peer_identity_bindings::upsert(
        &mut conn,
        &binding(
            "12D3KooWnew",
            "agent-matthew-desktop",
            "u-action-hash-new",
            "desktop",
            None,
        ),
    )
    .expect("insert new");

    let active = peer_identity_bindings::list_active_for_agent(
        &mut conn,
        "agent-matthew-desktop",
        &now_iso(),
    )
    .expect("list");
    assert_eq!(active.len(), 1, "superseded binding must be excluded");
    assert_eq!(active[0].peer_id, "12D3KooWnew");
}
