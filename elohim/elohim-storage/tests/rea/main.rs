//! Consolidated integration-test binary — step 0b of the elohim-storage
//! crate decomposition (genesis/docs/superpowers/specs/
//! 2026-09-19-storage-crate-decomposition-design.md §4 row 0b).
//!
//! Each module below was its own `tests/*.rs` integration-test binary and
//! therefore its own link of the whole lib. Nothing about what is tested
//! changed: the crate-root-only `#![cfg(...)]` gates are hoisted onto the
//! `mod` declarations here, verbatim.

mod attestation_consolidation_integration;
mod binding_attribution_cut;
mod binding_attribution_refuses_sentinel;
mod bounds_validator_integration;
mod graduation_attestation_test;
mod graduation_diversity_test;
mod graduation_evaluator_test;
mod graduation_summary_event_test;
mod mishpat_bounds_gate_chain;
mod provenance_gate_integration;
mod provide_commitment_epic_b;
mod replicates_commons_notarized_gate;
mod seed_delegates_compute;
mod standing_extension_integration;
