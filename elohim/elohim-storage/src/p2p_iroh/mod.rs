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

mod blob_store;
pub mod codec;
mod config;
mod endpoint;
pub mod epr;
mod gossip;
mod identity;
mod node;
pub mod parity_harness;
pub mod sync;

pub use blob_store::IrohBlobStore;
pub use config::IrohConfig;
pub use endpoint::{build_endpoint, BuildEndpointError};
pub use epr::{
    EprAtomBackend, EprBackend, IrohEprAtomClient, IrohEprAtomProtocol, IrohEprClient,
    IrohEprProtocol, EPR_ALPN, EPR_ATOM_ALPN,
};
pub use gossip::{GossipEvent, IrohGossip};
pub use identity::load_or_generate as load_or_generate_secret_key;
pub use node::{AlpnRegistration, IrohNode};
pub use sync::{IrohSyncClient, IrohSyncProtocol, SyncBackend, SYNC_ALPN};

// Re-export the iroh-blobs Hash type so callers don't have to depend on
// iroh-blobs directly when they live behind this module's API.
pub use iroh_blobs::Hash as BlobHash;
