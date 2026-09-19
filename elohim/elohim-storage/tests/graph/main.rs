//! Consolidated integration-test binary — step 0b of the elohim-storage
//! crate decomposition (genesis/docs/superpowers/specs/
//! 2026-09-19-storage-crate-decomposition-design.md §4 row 0b).
//!
//! Each module below was its own `tests/*.rs` integration-test binary and
//! therefore its own link of the whole lib. Nothing about what is tested
//! changed: the crate-root-only `#![cfg(...)]` gates are hoisted onto the
//! `mod` declarations here, verbatim.

#[cfg(feature = "graph-native")]
mod graph_backfill;
#[cfg(feature = "graph-native")]
mod graph_engine_smoke;
#[cfg(feature = "graph-native")]
mod graph_indexes;
#[cfg(feature = "graph-native")]
mod graph_primitives;
#[cfg(feature = "graph-native")]
mod graph_projector;
#[cfg(feature = "graph-native")]
mod graphql_codegen;
#[cfg(feature = "graph-native")]
mod graphql_demonstration_queries;
#[cfg(feature = "graph-native")]
mod graphql_endpoint;
#[cfg(feature = "graph-native")]
mod graphql_federation_spec;
#[cfg(feature = "graph-native")]
mod graphql_viewer_hub;
#[cfg(feature = "graph-native")]
mod graphql_viewer_peers;
#[cfg(feature = "graph-native")]
mod graphql_viewer_reciprocity;
#[cfg(feature = "graph-native")]
mod lamad_manifest_registration;
#[cfg(feature = "graph-native")]
mod manifest_graph_validator;
#[cfg(feature = "graph-native")]
mod projection_fanout;
#[cfg(feature = "graph-native")]
mod shefa_manifest_registration;
