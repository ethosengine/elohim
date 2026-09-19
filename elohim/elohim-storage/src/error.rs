//! Error types for elohim-storage
//!
//! Re-export shim. `StorageError` and its four `From` conversions moved verbatim
//! to the `elohim-error` crate — step 1 of the storage decomposition
//! (`genesis/docs/superpowers/specs/2026-09-19-storage-crate-decomposition-design.md`
//! §4 row 1). This module keeps `crate::error::StorageError` alive so the ~244
//! call sites that name it never changed; `lib.rs` re-exports it as
//! `crate::StorageError` exactly as before.
//!
//! New code should import from `elohim_error::` directly. A new variant, and any
//! new `From<ForeignError>`, belongs in that crate — the orphan rule leaves no
//! choice — or is reduced here to an existing `String` variant.

pub use elohim_error::*;
