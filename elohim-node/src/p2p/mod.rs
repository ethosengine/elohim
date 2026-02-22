//! P2P networking layer - libp2p transport
//!
//! Handles:
//! - Multi-transport (QUIC + TCP with Noise/Yamux)
//! - Protocol handlers (sync codec, request-response)
//! - Peer discovery (mDNS, Kademlia)
//! - NAT traversal (future)

pub mod nat;
pub mod protocols;
pub mod transport;

pub use transport::build_swarm;
