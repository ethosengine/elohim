//! Pure translation layer — substrate EPR-REA types → VF-GraphQL wire shapes.
//!
//! Each module provides free functions that accept bridge-local thin input
//! structs (not elohim-views types, which live in the elohim workspace).
//! Callers in elohim-storage project `elohim-views` fields into these thin
//! structs before calling translate functions.
//!
//! Every translate function also writes a `TranslationPoint` observation to
//! the learning ledger (fire-and-forget via `tokio::task::spawn_blocking`,
//! same as `schema::economic_event::resolve`) so the M5 upstream-contribution
//! report has coverage data.
//!
//! Reference:
//! `genesis/docs/superpowers/specs/2026-05-20-wave3-valueflows-hrea-interop-design.md`
//! §4.1 (crate structure) and §4.2 (learning ledger schema).

pub mod agent;
