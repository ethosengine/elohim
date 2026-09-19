//! Consolidated integration-test binary — step 0b of the elohim-storage
//! crate decomposition (genesis/docs/superpowers/specs/
//! 2026-09-19-storage-crate-decomposition-design.md §4 row 0b).
//!
//! Each module below was its own `tests/*.rs` integration-test binary and
//! therefore its own link of the whole lib. Nothing about what is tested
//! changed: the crate-root-only `#![cfg(...)]` gates are hoisted onto the
//! `mod` declarations here, verbatim.

#[path = "../harness/mod.rs"]
mod harness;
#[path = "../harness_d8/mod.rs"]
mod harness_d8;

mod aunt_and_rage_bait_integration;
mod back_prop_record_predecessor_announce_e2e;
mod epr_atom_federation_d8;
mod epr_atom_federation_integration;
mod epr_atom_protocol_unit;
mod epr_head_centralization;
mod epr_ingest_integration;
mod epr_landing_atom_cid_roundtrip;
mod epr_reach_enforcement;
mod manifest_resolver_integration;
