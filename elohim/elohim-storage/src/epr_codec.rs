//! EPR Head IPLD codec — re-exported from `elohim-epr`.
//!
//! The codec lives in [`elohim_epr::head`]: the head is a protocol primitive,
//! and the crate that addresses EPR atoms is the one home for the dag-cbor CID
//! construction both share. This module keeps storage's `epr_codec::` paths
//! resolving; new code should name `elohim_epr::head` directly.

pub use elohim_epr::head::*;
