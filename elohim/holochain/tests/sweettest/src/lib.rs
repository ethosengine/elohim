//! Elohim Sweettest — shared helpers for integration tests across all 5 DNAs.
//!
//! Layout:
//! - `common::conductors` — conductor spin-up helpers (single/multi-agent).
//! - `common::fixtures`   — shared test data (agent identities, network seeds, happ paths).
//! - `common::mirrors`    — DHT sync/propagation wait helpers.
//!
//! Per-DNA tests live at `src/tests/<dna>.rs` and are driven by the
//! `[[test]]` targets declared in `Cargo.toml`.
//!
//! See `README.md` for how to add a new test.

pub mod common;

pub use common::conductors;
pub use common::fixtures;
pub use common::mirrors;
