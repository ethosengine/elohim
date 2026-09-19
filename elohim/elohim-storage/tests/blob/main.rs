//! Consolidated integration-test binary — step 0b of the elohim-storage
//! crate decomposition (genesis/docs/superpowers/specs/
//! 2026-09-19-storage-crate-decomposition-design.md §4 row 0b).
//!
//! Each module below was its own `tests/*.rs` integration-test binary and
//! therefore its own link of the whole lib. Nothing about what is tested
//! changed: the crate-root-only `#![cfg(...)]` gates are hoisted onto the
//! `mod` declarations here, verbatim.

mod acquisition_pull_e2e;
#[cfg(feature = "p2p-iroh")]
mod blob_backend_dispatch;
mod blob_ingest_manifest_projection;
mod chunked_blob_local_read_unification;
mod chunked_blob_manifest_durability;
#[cfg(feature = "p2p-iroh")]
mod compute_payload_native;
mod compute_triptych;
#[cfg(feature = "p2p")]
mod distribute_shards_diversity;
mod inventory_writer_smoke;
#[cfg(feature = "p2p-iroh")]
mod put_blob_dual_write;
mod rs_blob_erasure_coded_put;
#[cfg(feature = "p2p-iroh")]
mod seed_e2e_dual_address;
#[cfg(feature = "p2p")]
mod shard_frame_budget;
