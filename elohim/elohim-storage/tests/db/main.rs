//! Consolidated integration-test binary — step 0b of the elohim-storage
//! crate decomposition (genesis/docs/superpowers/specs/
//! 2026-09-19-storage-crate-decomposition-design.md §4 row 0b).
//!
//! Each module below was its own `tests/*.rs` integration-test binary and
//! therefore its own link of the whole lib. Nothing about what is tested
//! changed: the crate-root-only `#![cfg(...)]` gates are hoisted onto the
//! `mod` declarations here, verbatim.

mod account_import_idempotency;
mod household_backfill;
mod human_household_create_bridge;
mod imagodei_lookup;
mod manifest_registry_layer1;
mod migration_content_reach_canonicalize;
mod peer_bindings_phase4_columns;
mod phase4_projector_topology_integration;
mod projection_events_writer;
mod signal_emit_round_trip;
