//! Generated artifacts for the epr crate.
//!
//! Files in this module are emitted from protocol JSON schemas by
//! `pnpm run schema:codegen:rs`. They live here (rather than in
//! elohim-storage) because epr is upstream of storage and cannot import
//! storage's generated enums without a circular dependency.
//!
//! DO NOT EDIT the generated submodules by hand.

mod reach_ordinal;

pub use reach_ordinal::*;
