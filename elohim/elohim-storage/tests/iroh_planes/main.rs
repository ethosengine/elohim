//! Consolidated integration-test binary — step 0b of the elohim-storage
//! crate decomposition (genesis/docs/superpowers/specs/
//! 2026-09-19-storage-crate-decomposition-design.md §4 row 0b).
//!
//! Each module below was its own `tests/*.rs` integration-test binary and
//! therefore its own link of the whole lib. Nothing about what is tested
//! changed: the crate-root-only `#![cfg(...)]` gates are hoisted onto the
//! `mod` declarations here, verbatim.

#[cfg(feature = "p2p-iroh")]
mod iroh_auth_parity;
#[cfg(feature = "p2p-iroh")]
mod iroh_auth_real_backend;
#[cfg(feature = "p2p-iroh")]
mod iroh_blob_roundtrip;
#[cfg(feature = "p2p-iroh")]
mod iroh_custom_alpn_echo;
#[cfg(feature = "p2p-iroh")]
mod iroh_epr_announce_accepted;
#[cfg(feature = "p2p-iroh")]
mod iroh_epr_atom_real_backend;
#[cfg(feature = "p2p-iroh")]
mod iroh_epr_parity;
#[cfg(feature = "p2p-iroh")]
mod iroh_epr_real_backend;
#[cfg(feature = "p2p-iroh")]
mod iroh_gossip_byte_parity;
#[cfg(feature = "p2p-iroh")]
mod iroh_gossip_cross_stack_e2e;
#[cfg(feature = "p2p-iroh")]
mod iroh_gossip_dual_publish_conductor_agent_info;
#[cfg(feature = "p2p-iroh")]
mod iroh_gossip_dual_publish_identity_binding;
#[cfg(feature = "p2p-iroh")]
mod iroh_gossip_dual_publish_inventory;
#[cfg(feature = "p2p-iroh")]
mod iroh_gossip_dual_publish_recovery;
#[cfg(feature = "p2p-iroh")]
mod iroh_gossip_parity;
#[cfg(all(feature = "p2p", feature = "p2p-iroh"))]
mod iroh_inventory_fetch_parity;
#[cfg(feature = "p2p-iroh")]
mod iroh_node_lifecycle;
#[cfg(feature = "p2p-iroh")]
mod iroh_peer_map;
#[cfg(feature = "p2p-iroh")]
mod iroh_peer_transport_manifest;
#[cfg(feature = "p2p-iroh")]
mod iroh_peer_transport_manifest_migration;
#[cfg(feature = "p2p-iroh")]
mod iroh_pkarr_e2e;
#[cfg(feature = "p2p-iroh")]
mod iroh_recovery_cross_stack;
#[cfg(feature = "p2p-iroh")]
mod iroh_shard_parity;
#[cfg(feature = "p2p-iroh")]
mod iroh_shard_real_backend;
#[cfg(feature = "p2p-iroh")]
mod iroh_sync_announce;
#[cfg(feature = "p2p-iroh")]
mod iroh_sync_driver;
#[cfg(feature = "p2p-iroh")]
mod iroh_sync_parity;
#[cfg(feature = "p2p-iroh")]
mod iroh_sync_real_backend;
#[cfg(feature = "p2p-iroh")]
mod iroh_view_fed_parity;
#[cfg(feature = "p2p-iroh")]
mod iroh_view_fed_real_backend;
