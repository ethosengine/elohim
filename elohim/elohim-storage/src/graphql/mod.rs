//! GraphQL surface — Apollo Federation v2 subgraph for the graph-native projection.
//!
//! Exposes `POST /api/v1/graphql` via a hand-rolled hyper handler (this codebase uses
//! hyper directly, not axum, so async-graphql-axum is not available). The handler:
//!   1. Reads the request body to bytes.
//!   2. Deserializes as `async_graphql::Request`.
//!   3. Executes against the schema built from `server::build_schema`.
//!   4. Serializes the `async_graphql::Response` and wraps it in `Response<Full<Bytes>>`.
//!
//! All items are gated under `#[cfg(feature = "graph-native")]` — the outer `lib.rs`
//! declaration already gates `pub mod graphql` on the feature, but individual items
//! carry their own cfg in case this module is ever partially re-exported.

#[cfg(feature = "graph-native")]
pub mod codegen;
#[cfg(feature = "graph-native")]
pub mod resolvers;
#[cfg(feature = "graph-native")]
pub mod server;
