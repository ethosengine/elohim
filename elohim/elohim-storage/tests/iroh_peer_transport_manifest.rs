//! Phase 12 integration — full CRUD + select_transport across three planes.

#![cfg(feature = "p2p-iroh")]

use elohim_storage::db::{init_pool_from_dir, run_migrations};
use elohim_storage::p2p_iroh::peer_map::{
    self, IrohTransportProfile, Libp2pTransportProfile, PeerTransportManifest, Plane,
    TransportChoice,
};
use tempfile::tempdir;

fn hub_manifest() -> PeerTransportManifest {
    PeerTransportManifest {
        agent_cid: "bafyrei...hub".to_string(),
        libp2p: Some(Libp2pTransportProfile {
            peer_id: "12D3KooWHub".to_string(),
            addrs: vec!["/ip4/10.0.0.1/tcp/4001".to_string()],
            supports: vec!["blob".into(), "gossip".into(), "trust".into()],
        }),
        iroh: Some(IrohTransportProfile {
            node_id: "node-hub".to_string(),
            relays: vec![],
            supports: vec!["blob".into(), "sync".into(), "epr".into()],
        }),
        discovery: vec!["pkarr".into(), "kademlia".into()],
        capability_level: 5,
        last_observed: 1746878400,
    }
}

#[test]
fn full_crud_persists_and_reads_back() {
    let dir = tempdir().unwrap();
    let pool = init_pool_from_dir(dir.path()).expect("pool");
    run_migrations(&pool).expect("migrations");
    let mut conn = pool.get().expect("conn");

    let agent = "bafyrei...crud";
    peer_map::record_libp2p_observation(
        &mut conn,
        agent,
        "12D3KooWCrud",
        &["/ip4/10.0.0.1/tcp/4001".to_string()],
        &[Plane::Blob, Plane::Gossip, Plane::Trust],
        1746878400,
    )
    .unwrap();
    peer_map::record_iroh_observation(
        &mut conn,
        agent,
        "node-crud",
        &["https://relay.iroh.network".to_string()],
        &[Plane::Blob, Plane::Sync, Plane::Epr],
        1746878401,
    )
    .unwrap();
    peer_map::record_capability(&mut conn, agent, 4).unwrap();
    peer_map::record_discovery(&mut conn, agent, &["pkarr", "mdns"]).unwrap();

    let m = peer_map::lookup_by_agent_cid(&mut conn, agent).unwrap().unwrap();
    assert_eq!(m.capability_level, 4);
    assert_eq!(m.libp2p.as_ref().unwrap().supports.len(), 3);
    assert_eq!(m.iroh.as_ref().unwrap().supports.len(), 3);

    let m2 = peer_map::lookup_by_libp2p_peer_id(&mut conn, "12D3KooWCrud").unwrap().unwrap();
    assert_eq!(m2.agent_cid, agent);
    let m3 = peer_map::lookup_by_iroh_node_id(&mut conn, "node-crud").unwrap().unwrap();
    assert_eq!(m3.agent_cid, agent);
}

#[test]
fn select_transport_blob_prefers_iroh() {
    let self_m = hub_manifest();
    let peer_m = hub_manifest();
    let choice = peer_map::select_transport(&self_m, &peer_m, Plane::Blob).unwrap();
    assert_eq!(choice, TransportChoice::Iroh);
}

#[test]
fn select_transport_trust_only_libp2p_shared_falls_back() {
    // Both peers have iroh, but neither lists "trust" in iroh.supports;
    // both list it in libp2p.supports.
    let self_m = hub_manifest();
    let peer_m = hub_manifest();
    let choice = peer_map::select_transport(&self_m, &peer_m, Plane::Trust).unwrap();
    assert_eq!(choice, TransportChoice::Libp2p);
}

#[test]
fn select_transport_sync_iroh_only_picks_iroh() {
    let self_m = hub_manifest();
    let peer_m = hub_manifest();
    let choice = peer_map::select_transport(&self_m, &peer_m, Plane::Sync).unwrap();
    assert_eq!(choice, TransportChoice::Iroh);
}
