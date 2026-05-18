//! Wire-shape Rust types for the Elohim Protocol storage API.
//!
//! These types use `#[derive(TS)]` + `#[ts(export, export_to = "...")]` to
//! generate camelCase TypeScript interfaces at
//! `elohim/sdk/storage-client-ts/src/generated/`. Consumers depend on this
//! crate (directly or via the `elohim-sdk` facade) to get a stable wire
//! contract without pulling the heavy `elohim-storage` implementation
//! (diesel, axum, libp2p, conductor).
//!
//! # Boundary rules
//!
//! - Types here MUST be wire-shape (camelCase via `#[serde(rename_all = "camelCase")]`)
//! - Types here MUST NOT depend on storage-internal types (Diesel models,
//!   internal error types, P2P transport details)
//! - Conversion functions (Wire→View `From` impls touching DB types) live in
//!   `elohim-storage`, NOT here
//!
//! See `genesis/docs/plans/2026-05-18-sdk-boundary-clarification.md` for the
//! migration history.

pub mod shared;
pub mod lamad;
pub mod shefa;
pub mod qahal;
pub mod imagodei;
pub mod infrastructure;
pub mod epr;
pub mod inputs;

// Re-export all domain types at the crate root for convenience
pub use shared::*;
pub use lamad::*;
pub use shefa::*;
pub use qahal::*;
pub use imagodei::*;
pub use infrastructure::*;
pub use epr::*;
