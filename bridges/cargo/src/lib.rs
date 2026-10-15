//! # cargo-bridge — cargo observed, never in control
//!
//! The first implementation of reliance as `cite`
//! (`genesis/docs/superpowers/specs/2026-10-15-reliance-as-cite-shared-provenance-primitives-design.md`):
//! a Rust workspace's resolved dependencies become `Cite` inputs on its build `Process`, each
//! upstream project named as an unclaimed presence in a governed offer's disclosure. The records
//! are the shared ones — the same `FlowEvent{Cite}`, `Process`, `DepEdge` and offer a co-op
//! grocer's supplier bridge or a garden network's seed-library bridge would write. Nothing here
//! is code-specific beyond reading cargo's own files.
//!
//! - [`metadata`] reads `cargo metadata --format-version 1 --locked` and `Cargo.lock`
//!   (never `authors`: no person is derived from package metadata);
//! - [`translate`] is pure: metadata → offer, cite events, Process, edges;
//! - [`observe`] appends idempotently, or verifies and prints a rung-one notice when stale.

pub mod metadata;
pub mod observe;
pub mod translate;

pub use observe::{observe, reliance, verify, BridgeError, Inputs, ObserveOutcome, VerifyOutcome};
pub use translate::{presence_slug, translate, Config, Origin, Refusal, Reliance, Report};
