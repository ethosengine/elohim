//! CID derivation for EPR canonical bytes.
//!
//! CIDv1, codec = 0x71 (dag-cbor), multihash = 0x12 (sha2-256).

use cid::Cid;
use multihash_codetable::{Code, MultihashDigest};

/// Codec byte for dag-cbor per the IPLD multicodec table.
const DAG_CBOR_CODEC: u64 = 0x71;

/// Compute CIDv1(dag-cbor, sha2-256) over the given canonical bytes.
pub fn compute_cid(canonical_bytes: &[u8]) -> Cid {
    let mh = Code::Sha2_256.digest(canonical_bytes);
    Cid::new_v1(DAG_CBOR_CODEC, mh)
}

/// Verify that a given CID matches the hash of the given canonical bytes.
pub fn verify_cid(claimed: &Cid, canonical_bytes: &[u8]) -> bool {
    compute_cid(canonical_bytes) == *claimed
}
