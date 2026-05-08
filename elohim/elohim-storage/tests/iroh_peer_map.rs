//! Phase 10 acceptance — cross-stack peer identity mapping.
//!
//! Verifies that observations from both transports for the same
//! `agent_cid` converge into one row, and that `iroh_for_libp2p` /
//! `libp2p_for_iroh` resolve correctly across the bridge.

#![cfg(feature = "p2p-iroh")]

use elohim_storage::db::{init_pool_from_dir, run_migrations};
use elohim_storage::p2p_iroh::peer_map;
use tempfile::tempdir;

#[test]
fn libp2p_then_iroh_converge_for_same_agent() {
    let dir = tempdir().unwrap();
    let pool = init_pool_from_dir(dir.path()).expect("pool");
    run_migrations(&pool).expect("migrations");
    let mut conn = pool.get().expect("conn");

    let agent = "bafyrei...agent-1";
    peer_map::record_libp2p(&mut conn, agent, "12D3Koo...A", "2026-05-08T05:00:00Z").unwrap();
    peer_map::record_iroh(&mut conn, agent, "node-id-A", "2026-05-08T05:01:00Z").unwrap();

    // Resolve in either direction.
    let nid = peer_map::iroh_for_libp2p(&mut conn, "12D3Koo...A").unwrap();
    assert_eq!(nid.as_deref(), Some("node-id-A"));

    let pid = peer_map::libp2p_for_iroh(&mut conn, "node-id-A").unwrap();
    assert_eq!(pid.as_deref(), Some("12D3Koo...A"));
}

#[test]
fn iroh_only_observation_returns_no_libp2p() {
    let dir = tempdir().unwrap();
    let pool = init_pool_from_dir(dir.path()).expect("pool");
    run_migrations(&pool).expect("migrations");
    let mut conn = pool.get().expect("conn");

    let agent = "bafyrei...agent-2";
    peer_map::record_iroh(&mut conn, agent, "node-id-B", "2026-05-08T05:00:00Z").unwrap();

    let pid = peer_map::libp2p_for_iroh(&mut conn, "node-id-B").unwrap();
    assert_eq!(pid, None);
}

#[test]
fn libp2p_only_observation_returns_no_iroh() {
    let dir = tempdir().unwrap();
    let pool = init_pool_from_dir(dir.path()).expect("pool");
    run_migrations(&pool).expect("migrations");
    let mut conn = pool.get().expect("conn");

    let agent = "bafyrei...agent-3";
    peer_map::record_libp2p(&mut conn, agent, "12D3Koo...C", "2026-05-08T05:00:00Z").unwrap();

    let nid = peer_map::iroh_for_libp2p(&mut conn, "12D3Koo...C").unwrap();
    assert_eq!(nid, None);
}

#[test]
fn unknown_peer_id_resolves_to_none() {
    let dir = tempdir().unwrap();
    let pool = init_pool_from_dir(dir.path()).expect("pool");
    run_migrations(&pool).expect("migrations");
    let mut conn = pool.get().expect("conn");

    assert_eq!(
        peer_map::iroh_for_libp2p(&mut conn, "12D3Koo...nope").unwrap(),
        None
    );
    assert_eq!(
        peer_map::libp2p_for_iroh(&mut conn, "node-id-nope").unwrap(),
        None
    );
}
