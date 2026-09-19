//! Consolidated integration-test binary — step 0b of the elohim-storage
//! crate decomposition (genesis/docs/superpowers/specs/
//! 2026-09-19-storage-crate-decomposition-design.md §4 row 0b).
//!
//! Each module below was its own `tests/*.rs` integration-test binary and
//! therefore its own link of the whole lib. Nothing about what is tested
//! changed: the crate-root-only `#![cfg(...)]` gates are hoisted onto the
//! `mod` declarations here, verbatim.

mod observation_diversity_view_test;
mod observation_gossip_test;
#[cfg(feature = "p2p-iroh")]
mod observation_iroh_backend_test;
mod observation_log_test;
mod observation_manager_test;
#[cfg(feature = "p2p-iroh")]
mod observation_plane_registration_test;
mod observation_projector_test;
mod observation_view_types_test;
mod observation_wire_test;
