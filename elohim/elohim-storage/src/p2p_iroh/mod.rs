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

// Submodules added in subsequent tasks.
