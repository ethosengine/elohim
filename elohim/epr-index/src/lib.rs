//! The pure parts of an EPR index, shared by every runtime that folds or answers one.
//!
//! An index is a measure (`elohim_epr_rea::IndexMeasure`): a declared procedure whose every
//! custodian proves its own completion. The recall executor folds the repository under one; the
//! storage peer folds its content projection under another. What they share lives here — the
//! declared chunk rule ([`chunk`]), a question's terms and the FTS5 match expression ([`terms`]),
//! ranking ([`rank`]), rank fusion ([`fuse`]), the embedding boundary ([`embedder`]), the fold
//! attestation ([`attest`]) and the reader an answer is shaped for ([`reader`]).
//!
//! What does NOT live here: walking a tree, reading git, deciding which units a fold covers,
//! reading a recipe's contract, or any wire type. A unit is an opaque `unit_id` — a path to the
//! executor, a content id to storage — and this crate never interprets it.
//!
//! **One SQLite per binary.** `rusqlite` is a dependency without features; the binary that links
//! this crate chooses its SQLite (post-station-4 sprint, ruling R-S1).
pub mod attest;
pub mod chunk;
pub mod embedder;
pub mod error;
pub mod fuse;
pub mod rank;
pub mod reader;
pub mod terms;

pub use error::{IndexError, Result};
