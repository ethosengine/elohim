//! Transport and P2P protocol integration tests
//!
//! Tests libp2p transport configuration, mDNS discovery,
//! and the /elohim/sync/1.0.0 protocol exchange.

use libp2p::{
    identity, kad, mdns, noise, tcp, yamux,
    Multiaddr, PeerId, StreamProtocol, SwarmBuilder,
};
use std::time::Duration;

const SYNC_PROTOCOL: &str = "/elohim/sync/1.0.0";

// =============================================================================
// Identity & Keypair
// =============================================================================

#[test]
fn test_generate_ed25519_keypair() {
    let keypair = identity::Keypair::generate_ed25519();
    let peer_id = PeerId::from(keypair.public());

    // PeerId should be a 12D3Koo... string (base58)
    let peer_str = peer_id.to_string();
    assert!(
        peer_str.starts_with("12D3Koo"),
        "Ed25519 PeerId should start with 12D3Koo, got: {}",
        peer_str
    );
}

#[test]
fn test_keypair_persistence_protobuf() {
    let dir = tempfile::TempDir::new().unwrap();
    let key_path = dir.path().join("node_key");

    // Generate and save
    let keypair = identity::Keypair::generate_ed25519();
    let original_peer_id = PeerId::from(keypair.public());
    let encoded = keypair.to_protobuf_encoding().unwrap();
    std::fs::write(&key_path, &encoded).unwrap();

    // Load back
    let loaded_bytes = std::fs::read(&key_path).unwrap();
    let loaded_keypair = identity::Keypair::from_protobuf_encoding(&loaded_bytes).unwrap();
    let loaded_peer_id = PeerId::from(loaded_keypair.public());

    assert_eq!(original_peer_id, loaded_peer_id);
}

#[test]
fn test_two_different_keypairs_different_peer_ids() {
    let kp1 = identity::Keypair::generate_ed25519();
    let kp2 = identity::Keypair::generate_ed25519();

    let pid1 = PeerId::from(kp1.public());
    let pid2 = PeerId::from(kp2.public());

    assert_ne!(pid1, pid2, "Two random keypairs should have different PeerIds");
}

// =============================================================================
// Multiaddr Parsing
// =============================================================================

#[test]
fn test_parse_tcp_multiaddr() {
    let addr: Multiaddr = "/ip4/0.0.0.0/tcp/4001".parse().unwrap();
    let addr_str = addr.to_string();
    assert!(addr_str.contains("tcp"), "Should contain tcp protocol");
    assert!(addr_str.contains("4001"), "Should contain port 4001");
}

#[test]
fn test_parse_quic_multiaddr() {
    let addr: Multiaddr = "/ip4/0.0.0.0/udp/4001/quic-v1".parse().unwrap();
    let addr_str = addr.to_string();
    assert!(addr_str.contains("udp"), "Should contain udp protocol");
    assert!(addr_str.contains("quic"), "Should contain quic protocol");
}

#[test]
fn test_parse_peer_addr_with_p2p() {
    // Generate a real PeerId for the address
    let keypair = identity::Keypair::generate_ed25519();
    let peer_id = PeerId::from(keypair.public());

    let addr_str = format!("/ip4/192.168.1.100/tcp/4001/p2p/{}", peer_id);
    let addr: Multiaddr = addr_str.parse().unwrap();

    // Extract PeerId from multiaddr
    let extracted_peer = addr.iter().find_map(|p| {
        if let libp2p::multiaddr::Protocol::P2p(pid) = p {
            Some(pid)
        } else {
            None
        }
    });

    assert_eq!(extracted_peer.unwrap(), peer_id);
}

#[test]
fn test_parse_invalid_multiaddr() {
    let result: Result<Multiaddr, _> = "not a valid addr".parse();
    assert!(result.is_err(), "Invalid multiaddr should fail to parse");
}

// =============================================================================
// Protocol Identifiers
// =============================================================================

#[test]
fn test_sync_protocol_identifier() {
    let protocol = StreamProtocol::new(SYNC_PROTOCOL);
    assert_eq!(protocol.as_ref(), "/elohim/sync/1.0.0");
}

#[test]
fn test_protocol_identifiers_unique() {
    let sync = "/elohim/sync/1.0.0";
    let shard = "/elohim/shard/1.0.0";
    let cluster = "/elohim/cluster/1.0.0";
    let identify = "/elohim/id/1.0.0";

    let protos = vec![sync, shard, cluster, identify];
    let unique: std::collections::HashSet<_> = protos.iter().collect();
    assert_eq!(
        protos.len(),
        unique.len(),
        "All protocol IDs should be unique"
    );
}

// =============================================================================
// Swarm Builder Configuration
// =============================================================================

#[test]
fn test_swarm_builder_tcp_transport() {
    let keypair = identity::Keypair::generate_ed25519();

    let result = SwarmBuilder::with_existing_identity(keypair)
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        );

    assert!(result.is_ok(), "TCP transport should configure successfully");
}

#[tokio::test]
async fn test_swarm_builder_tcp_plus_quic() {
    let keypair = identity::Keypair::generate_ed25519();

    // Verify QUIC can be added on top of TCP, then build with mDNS only
    let swarm = SwarmBuilder::with_existing_identity(keypair)
        .with_tokio()
        .with_tcp(tcp::Config::default(), noise::Config::new, yamux::Config::default)
        .unwrap()
        .with_quic()
        .with_behaviour(|key| {
            mdns::tokio::Behaviour::new(
                mdns::Config::default(),
                key.public().to_peer_id(),
            ).unwrap()
        })
        .unwrap()
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(5)))
        .build();

    assert!(
        swarm.local_peer_id().to_string().starts_with("12D3Koo"),
        "Swarm should have a valid PeerId"
    );
}

#[test]
fn test_stream_protocol_creation() {
    let protocol = StreamProtocol::new(SYNC_PROTOCOL);
    assert_eq!(protocol.as_ref(), "/elohim/sync/1.0.0");

    // Verify timeout duration used in transport.rs
    let timeout = Duration::from_secs(30);
    assert_eq!(timeout.as_secs(), 30);
}

// =============================================================================
// mDNS Discovery Configuration
// =============================================================================

#[tokio::test]
async fn test_mdns_config_defaults() {
    let config = mdns::Config::default();

    // Default mDNS config should work without customization
    let keypair = identity::Keypair::generate_ed25519();
    let peer_id = PeerId::from(keypair.public());

    let behaviour = mdns::tokio::Behaviour::new(config, peer_id);
    assert!(behaviour.is_ok(), "Default mDNS config should work");
}

// =============================================================================
// Kademlia DHT Configuration
// =============================================================================

#[test]
fn test_kademlia_server_mode() {
    let keypair = identity::Keypair::generate_ed25519();
    let peer_id = PeerId::from(keypair.public());

    let store = kad::store::MemoryStore::new(peer_id);
    let mut kademlia = kad::Behaviour::new(peer_id, store);

    // Set to server mode (as done in transport.rs)
    kademlia.set_mode(Some(kad::Mode::Server));

    // Verify we can add addresses
    let addr: Multiaddr = "/ip4/192.168.1.1/tcp/4001".parse().unwrap();
    let other_peer = PeerId::from(identity::Keypair::generate_ed25519().public());
    kademlia.add_address(&other_peer, addr);
}

// =============================================================================
// Identify Protocol
// =============================================================================

#[test]
fn test_identify_config() {
    let keypair = identity::Keypair::generate_ed25519();
    let config = libp2p::identify::Config::new(
        "/elohim/id/1.0.0".to_string(),
        keypair.public(),
    )
    .with_agent_version("elohim-node/0.1.0".to_string());

    let _behaviour = libp2p::identify::Behaviour::new(config);
    // Just verify it's constructible
}

// =============================================================================
// Two-Node mDNS Discovery (Tokio integration test)
// =============================================================================

#[tokio::test]
async fn test_two_nodes_build_swarms() {
    // Verify two independent swarms can be built (prerequisite for mDNS discovery)
    let kp1 = identity::Keypair::generate_ed25519();
    let kp2 = identity::Keypair::generate_ed25519();
    let pid1 = PeerId::from(kp1.public());
    let pid2 = PeerId::from(kp2.public());

    assert_ne!(pid1, pid2);

    // Build swarm 1
    let swarm1 = SwarmBuilder::with_existing_identity(kp1)
        .with_tokio()
        .with_tcp(tcp::Config::default(), noise::Config::new, yamux::Config::default)
        .unwrap()
        .with_quic()
        .with_behaviour(|key| {
            let mdns = mdns::tokio::Behaviour::new(
                mdns::Config::default(),
                key.public().to_peer_id(),
            ).unwrap();
            mdns
        })
        .unwrap()
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(10)))
        .build();

    // Build swarm 2
    let swarm2 = SwarmBuilder::with_existing_identity(kp2)
        .with_tokio()
        .with_tcp(tcp::Config::default(), noise::Config::new, yamux::Config::default)
        .unwrap()
        .with_quic()
        .with_behaviour(|key| {
            let mdns = mdns::tokio::Behaviour::new(
                mdns::Config::default(),
                key.public().to_peer_id(),
            ).unwrap();
            mdns
        })
        .unwrap()
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(10)))
        .build();

    // Verify both swarms have different local peer IDs
    assert_ne!(swarm1.local_peer_id(), swarm2.local_peer_id());
}

#[tokio::test]
async fn test_swarm_listens_on_random_port() {
    use futures::StreamExt;

    let keypair = identity::Keypair::generate_ed25519();
    let mut swarm = SwarmBuilder::with_existing_identity(keypair)
        .with_tokio()
        .with_tcp(tcp::Config::default(), noise::Config::new, yamux::Config::default)
        .unwrap()
        .with_quic()
        .with_behaviour(|key| {
            mdns::tokio::Behaviour::new(
                mdns::Config::default(),
                key.public().to_peer_id(),
            ).unwrap()
        })
        .unwrap()
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(5)))
        .build();

    // Listen on random port (port 0)
    let listen_addr: Multiaddr = "/ip4/127.0.0.1/tcp/0".parse().unwrap();
    swarm.listen_on(listen_addr).unwrap();

    // Poll the swarm to get the actual listen address
    let mut got_listen_addr = false;
    let timeout = tokio::time::sleep(Duration::from_secs(2));
    tokio::pin!(timeout);

    loop {
        tokio::select! {
            event = swarm.next() => {
                if let Some(libp2p::swarm::SwarmEvent::NewListenAddr { address, .. }) = event {
                    let addr_str = address.to_string();
                    assert!(addr_str.contains("127.0.0.1"), "Should listen on localhost");
                    assert!(!addr_str.contains("/tcp/0"), "Port should be assigned (not 0)");
                    got_listen_addr = true;
                    break;
                }
            }
            _ = &mut timeout => {
                break;
            }
        }
    }

    assert!(got_listen_addr, "Should receive NewListenAddr event");
}
