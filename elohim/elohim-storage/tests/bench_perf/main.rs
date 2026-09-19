//! Consolidated integration-test binary — step 0b of the elohim-storage
//! crate decomposition (genesis/docs/superpowers/specs/
//! 2026-09-19-storage-crate-decomposition-design.md §4 row 0b).
//!
//! Each module below was its own `tests/*.rs` integration-test binary and
//! therefore its own link of the whole lib. Nothing about what is tested
//! changed: the crate-root-only `#![cfg(...)]` gates are hoisted onto the
//! `mod` declarations here, verbatim.

#[cfg(all(feature = "p2p", feature = "p2p-iroh"))]
mod bench_blob_perf;
#[cfg(all(feature = "p2p", feature = "p2p-iroh"))]
mod bench_blob_stress_10k;
#[cfg(all(feature = "p2p", feature = "p2p-iroh"))]
mod bench_epr_atom_perf;
#[cfg(all(feature = "p2p", feature = "p2p-iroh"))]
mod bench_epr_perf;
#[cfg(all(feature = "p2p", feature = "p2p-iroh"))]
mod bench_gossip_perf;
#[cfg(all(feature = "p2p", feature = "p2p-iroh"))]
mod bench_identity_handshake_perf;
#[cfg(all(feature = "p2p", feature = "p2p-iroh"))]
mod bench_shard_perf;
#[cfg(all(feature = "p2p", feature = "p2p-iroh"))]
mod bench_sync_perf;
#[cfg(all(feature = "p2p", feature = "p2p-iroh"))]
mod bench_trust_perf;
#[cfg(all(feature = "p2p", feature = "p2p-iroh"))]
mod bench_view_fed_perf;
