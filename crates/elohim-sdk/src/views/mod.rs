//! View types exposed at the elohim-sdk boundary.
//!
//! These are the wire-shape (camelCase, JSON-parsed) types that HTTP
//! consumers and SDK clients depend on. The ts-rs codegen pipeline reads
//! the per-domain modules and produces TypeScript at
//! `elohim/sdk/storage-client-ts/src/generated/`.
//!
//! Boundary rule: types declared in this module MUST NOT pull in
//! `elohim-storage` types as fields. Anything that needs a storage-internal
//! type lives in `elohim-storage` and is converted at the API edge.

pub mod epr;
pub mod imagodei;
pub mod infrastructure;
pub mod inputs;
pub mod lamad;
pub mod qahal;
pub mod shefa;
pub mod shared;

// Convenience re-exports of the most commonly used types.
pub use shared::*;
