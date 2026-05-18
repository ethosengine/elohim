//! Wire → View conversion helpers.
//!
//! Storage-internal conversion functions live here, organized by domain.
//! These functions touch DB types (Diesel models) and therefore cannot
//! live in elohim-views. They produce View types (defined in elohim-views)
//! at the HTTP API boundary.

pub mod shared;
pub mod lamad;
pub mod shefa;
pub mod qahal;
pub mod imagodei;
pub mod infrastructure;
pub mod epr;
pub mod inputs;

pub use shared::*;
