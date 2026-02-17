//! P2P networking layer - libp2p transport
//!
//! Handles:
//! - Multi-transport (QUIC + TCP with Noise/Yamux)
//! - Protocol handlers (sync codec, request-response)
//! - Peer discovery (mDNS, Kademlia)
//! - NAT traversal (future)

pub mod transport;
pub mod protocols;
pub mod nat;

pub use transport::{build_swarm, ElohimSwarm, SwarmEvent};
pub use protocols::{SyncCodec, SYNC_PROTOCOL, SHARD_PROTOCOL, CLUSTER_PROTOCOL};
