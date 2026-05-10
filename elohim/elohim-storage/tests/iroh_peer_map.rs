//! Phase 12 acceptance — peer transport manifest API + Phase 10
//! back-compat shims.

#![cfg(feature = "p2p-iroh")]

use elohim_storage::db::{init_pool_from_dir, run_migrations};
use elohim_storage::p2p_iroh::peer_map;
use tempfile::tempdir;

fn setup() -> (
    tempfile::TempDir,
    diesel::r2d2::PooledConnection<
        diesel::r2d2::ConnectionManager<diesel::SqliteConnection>,
    >,
) {
    let dir = tempdir().unwrap();
    let pool = init_pool_from_dir(dir.path()).expect("pool");
    run_migrations(&pool).expect("migrations");
    let conn = pool.get().expect("conn");
    (dir, conn)
}

#[test]
fn back_compat_libp2p_then_iroh_converge_for_same_agent() {
    let (_dir, mut conn) = setup();
    let agent = "bafyrei...agent-1";
    peer_map::record_libp2p(&mut conn, agent, "12D3Koo...A", "2026-05-08T05:00:00Z").unwrap();
    peer_map::record_iroh(&mut conn, agent, "node-id-A", "2026-05-08T05:01:00Z").unwrap();

    let nid = peer_map::iroh_for_libp2p(&mut conn, "12D3Koo...A").unwrap();
    assert_eq!(nid.as_deref(), Some("node-id-A"));

    let pid = peer_map::libp2p_for_iroh(&mut conn, "node-id-A").unwrap();
    assert_eq!(pid.as_deref(), Some("12D3Koo...A"));
}

#[test]
fn back_compat_iroh_only_observation_returns_no_libp2p() {
    let (_dir, mut conn) = setup();
    let agent = "bafyrei...agent-2";
    peer_map::record_iroh(&mut conn, agent, "node-id-B", "2026-05-08T05:00:00Z").unwrap();
    let pid = peer_map::libp2p_for_iroh(&mut conn, "node-id-B").unwrap();
    assert_eq!(pid, None);
}

#[test]
fn back_compat_libp2p_only_observation_returns_no_iroh() {
    let (_dir, mut conn) = setup();
    let agent = "bafyrei...agent-3";
    peer_map::record_libp2p(&mut conn, agent, "12D3Koo...C", "2026-05-08T05:00:00Z").unwrap();
    let nid = peer_map::iroh_for_libp2p(&mut conn, "12D3Koo...C").unwrap();
    assert_eq!(nid, None);
}

#[test]
fn back_compat_unknown_peer_id_resolves_to_none() {
    let (_dir, mut conn) = setup();
    assert_eq!(peer_map::iroh_for_libp2p(&mut conn, "12D3Koo...nope").unwrap(), None);
    assert_eq!(peer_map::libp2p_for_iroh(&mut conn, "node-id-nope").unwrap(), None);
}

#[test]
fn extended_api_full_manifest_roundtrip() {
    let (_dir, mut conn) = setup();
    let agent = "bafyrei...agent-full";
    peer_map::record_libp2p_observation(
        &mut conn,
        agent,
        "12D3KooWFull",
        &["/ip4/10.0.0.1/tcp/4001".to_string()],
        &[peer_map::Plane::Blob, peer_map::Plane::Gossip],
        1746878400,
    )
    .unwrap();
    peer_map::record_iroh_observation(
        &mut conn,
        agent,
        "node-id-full",
        &["https://relay.iroh.network".to_string()],
        &[peer_map::Plane::Blob, peer_map::Plane::Sync, peer_map::Plane::Epr],
        1746878401,
    )
    .unwrap();
    peer_map::record_capability(&mut conn, agent, 4).unwrap();
    peer_map::record_discovery(&mut conn, agent, &["pkarr", "mdns"]).unwrap();

    let m = peer_map::lookup_by_agent_cid(&mut conn, agent).unwrap().unwrap();
    assert_eq!(m.capability_level, 4);
    assert_eq!(m.discovery, vec!["pkarr".to_string(), "mdns".to_string()]);
    assert!(m.libp2p.is_some());
    assert!(m.iroh.is_some());
    assert_eq!(m.iroh.as_ref().unwrap().supports.len(), 3);
}
