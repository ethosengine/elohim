//! Consolidated integration-test binary — step 0b of the elohim-storage
//! crate decomposition (genesis/docs/superpowers/specs/
//! 2026-09-19-storage-crate-decomposition-design.md §4 row 0b).
//!
//! Each module below was its own `tests/*.rs` integration-test binary and
//! therefore its own link of the whole lib. Nothing about what is tested
//! changed: the crate-root-only `#![cfg(...)]` gates are hoisted onto the
//! `mod` declarations here, verbatim.

mod acquisition_pins_http;
mod api_observations_test;
mod api_placement_gaps;
mod db_content_list_tags;
mod db_humans_http_route;
mod elohim_reputation_http;
mod forwarder_integration;
mod gate_decisions_http;
mod operation_authorization;
mod operator_verbs;
mod peer_statuses_route;
mod qahal_http_contract;
mod rea_commitment_camelcase_input;
mod rea_commitments_http_route;
mod reach_vocabulary_contract;
mod schema_contract_recovery_v2;
mod serve_routing;
mod ssr_direct;
mod startup_wiring;
#[cfg(not(feature = "graph-native"))]
mod thin_build_smoke;
