//! Canonical registry-row bytes shared with native `.epr-meta` evaluation.
//!
//! Keeping the implementation in `eprfs-meta` makes policy-pin verification and canon lifting
//! consume one byte codec. The public re-export remains here for existing CLI callers and parity
//! tests.

pub use eprfs_meta::{canonical_body, canonical_json, hex_lower, CanonError};
