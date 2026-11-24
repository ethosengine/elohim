//! Device-local primitives: process identity and content-addressed executable custody.
#![warn(missing_docs)]
pub mod bin_store;
pub mod commit_attribution;
pub mod envelope;
pub mod flock;
#[cfg(all(test, unix))]
mod fork_test;
pub mod layout;
pub mod manifest_store;
pub mod session;
pub mod session_stasis;
