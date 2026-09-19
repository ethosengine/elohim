//! Consolidated integration-test binary — step 0b of the elohim-storage
//! crate decomposition (genesis/docs/superpowers/specs/
//! 2026-09-19-storage-crate-decomposition-design.md §4 row 0b).
//!
//! Each module below was its own `tests/*.rs` integration-test binary and
//! therefore its own link of the whole lib. Nothing about what is tested
//! changed: the crate-root-only `#![cfg(...)]` gates are hoisted onto the
//! `mod` declarations here, verbatim.

mod cluster_view;
mod connectivity_helper;
mod device_capacity_helper;
mod distribution_view;
mod distribution_view_extensions_integration;
mod household_resilience;
mod hub_capacity_view_integration;
mod peer_capacity_view_integration;
mod peer_diversity_helper;
mod peer_topology_phase4;
mod peer_topology_view;
mod placement_gaps;
mod reciprocity_view;
mod reciprocity_view_phase4;
mod resilience_hub;
mod resilience_integration;
#[cfg(feature = "graph-native")]
mod views_lamad;
#[cfg(feature = "graph-native")]
mod views_shefa;
