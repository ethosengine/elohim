//! Consolidated integration-test binary — step 0b of the elohim-storage
//! crate decomposition (genesis/docs/superpowers/specs/
//! 2026-09-19-storage-crate-decomposition-design.md §4 row 0b).
//!
//! Each module below was its own `tests/*.rs` integration-test binary and
//! therefore its own link of the whole lib. Nothing about what is tested
//! changed: the crate-root-only `#![cfg(...)]` gates are hoisted onto the
//! `mod` declarations here, verbatim.

mod chaos_dataplane;
mod federator;
mod happ_manager_install_lineage;
mod happ_manifest_relay_url_compat;
mod p2p_command_view_federate;
mod peer_selection;
mod peer_status_e2e;
mod private_preauth_receive;
mod projection_ack_signal_e2e;
mod projector_invariants;
mod projector_round_trip;
mod projector_signals;
mod recovery_rotation_wire;
mod sync_integration;
mod sync_libp2p_convergence;
#[cfg(feature = "p2p")]
mod sync_scale_honesty;
