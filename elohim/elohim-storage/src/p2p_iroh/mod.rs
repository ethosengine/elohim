//! Parallel iroh-based P2P stack — staged cutover from libp2p.
//!
//! Sibling to [`crate::p2p`]; gated by the `p2p-iroh` Cargo feature. The two
//! stacks are mutually exclusive at runtime — selected by
//! [`crate::config::TransportBackend`] at startup — but compile additively so
//! the parity test harness can exercise them in one binary.
//!
//! Phase 2 (current scope): blob plane via iroh-blobs.
//! Phases 3+: gossip, sync, shard, EPR, view federation, identity, discovery.
//!
//! See `genesis/docs/superpowers/plans/2026-05-07-iroh-parallel-stack.md`.
//!
//! Pinned to `iroh = "=0.92"` + `iroh-blobs = "=0.94"`. Custom-ALPN handlers in
//! later phases will reuse the existing wire types from [`crate::p2p`]
//! (`BlobFetchRequest`, `SyncRequest`, etc.) — already `pub`, no relocation
//! required. Cutover removes one transport, never two divergent message
//! schemas.

pub mod auth;
pub mod auth_backends;
mod blob_store;
pub mod codec;
mod config;
pub mod dual_publish;
mod endpoint;
pub mod epr;
pub mod epr_atom_backend;
pub mod epr_backend;
mod gossip;
mod identity;
pub mod multi_stack_fixture;
mod node;
pub mod observation_backend;
pub mod parity_harness;
pub mod peer_map;
pub mod shard;
pub mod shard_backend;
pub mod sync;
pub mod sync_backend;
pub mod view_fed;
pub mod view_fed_backend;

pub use auth::{
    IdentityHandshakeBackend, IrohIdentityHandshakeClient, IrohIdentityHandshakeProtocol,
    IrohTrustClient, IrohTrustProtocol, TrustBackend, IDENTITY_HANDSHAKE_ALPN, TRUST_ALPN,
};
pub use auth_backends::{IdentityHandshakeServiceBackend, TrustServiceBackend};
pub use blob_store::IrohBlobStore;
pub use config::{DiscoveryResolverConfig, DiscoveryResolverKind, IrohConfig};
pub use dual_publish::{classify_topic, DualGossipPublisher, IrohGossipPublisher, TopicTransports};
pub use endpoint::{build_endpoint, BuildEndpointError};
pub use epr::{
    EprAtomBackend, EprBackend, IrohEprAtomClient, IrohEprAtomProtocol, IrohEprClient,
    IrohEprProtocol, EPR_ALPN, EPR_ATOM_ALPN,
};
pub use epr_atom_backend::EprAtomServiceBackend;
pub use epr_backend::EprServiceBackend;
pub use gossip::{GossipEvent, IrohGossip};
pub use identity::load_or_generate as load_or_generate_secret_key;
pub use node::{AlpnRegistration, IrohNode};
pub use shard::{IrohShardClient, IrohShardProtocol, ShardBackend, SHARD_ALPN};
pub use shard_backend::ShardServiceBackend;
pub use sync::{IrohSyncClient, IrohSyncProtocol, SyncBackend, SYNC_ALPN};
pub use sync_backend::SyncManagerBackend;
pub use view_fed::{
    IrohViewFederationClient, IrohViewFederationProtocol, ViewFederationBackend, VIEW_FED_ALPN,
};
pub use view_fed_backend::ViewFedServiceBackend;

// Re-export the iroh-blobs Hash type so callers don't have to depend on
// iroh-blobs directly when they live behind this module's API.
pub use iroh_blobs::Hash as BlobHash;
