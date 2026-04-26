//! elohim-agent defender specialist — M5 stub.
//!
//! See: genesis/docs/superpowers/specs/2026-04-25-recovery-protocol-phase-2-m5-...md §11
//!
//! Stage 3 evolution: defender role attestation reuses the existing imagodei
//! `Attestation` entry type. NO new entry type ever needed for defender role.

pub mod attestation;
pub mod detection;
pub mod manifest;
pub mod role_marker;

pub use attestation::*;
pub use detection::*;
pub use manifest::*;
pub use role_marker::*;
