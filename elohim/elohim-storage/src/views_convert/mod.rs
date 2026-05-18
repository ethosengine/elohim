//! Wire → View conversion helpers.
//!
//! Storage-internal conversion functions live here, organized by domain.
//! These functions touch DB types (Diesel models) and therefore cannot
//! live in elohim-views. They produce View types (defined in elohim-views)
//! at the HTTP API boundary.

pub mod epr;
pub mod imagodei;
pub mod infrastructure;
pub mod inputs;
pub mod lamad;
pub mod qahal;
pub mod shared;
pub mod shefa;

// shared::* is empty by design today (kept as a stub for future cross-domain helpers).
// Re-export still preserved so consumers can `use crate::views_convert::shared::*;` later
// without diff churn — allow keeps clippy -D warnings happy under WASM rustflags.
#[allow(unused_imports)]
pub use shared::*;
