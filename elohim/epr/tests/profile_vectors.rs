//! `elohim-epr` consumes the addressing kernel; it does not re-derive it. These tests pin that
//! the constructors reachable through `elohim_epr::cid` produce the kernel's shared vectors
//! (`crates/elohim-address/vectors/address-vectors.json`), so the citation and record
//! constructors keep their pre-crossing output and the byte constructor is BLAKE3.

// gate-reads: crates/elohim-address/vectors/address-vectors.json

use elohim_epr::cid::{
    compute_bytes_cid, compute_cid, compute_raw_cid, verify_cid_any, ByteCid, Verify,
};
use serde_json::Value;

fn vectors() -> Value {
    serde_json::from_str(include_str!(
        "../../../crates/elohim-address/vectors/address-vectors.json"
    ))
    .unwrap()
}

#[test]
fn byte_cids_match_oracle() {
    let v = vectors();
    for row in v["positive"].as_array().unwrap() {
        let input = row["input"].as_str().unwrap().as_bytes();
        assert_eq!(
            compute_bytes_cid(input).to_string(),
            row["byteCid"].as_str().unwrap()
        );
        // The citation and record constructors are unchanged by the crossing.
        assert_eq!(
            compute_raw_cid(input).to_string(),
            row["citeCid"].as_str().unwrap()
        );
        assert_eq!(
            compute_cid(input).to_string(),
            row["recordCid"].as_str().unwrap()
        );
    }
}

#[test]
fn byte_surface_refuses_every_negative_vector() {
    let v = vectors();
    for row in v["negative"].as_array().unwrap() {
        let input = row["input"].as_str().unwrap();
        let err = ByteCid::parse(input).unwrap_err();
        assert_eq!(err.check(), row["check"].as_str().unwrap(), "{input}");
    }
}

#[test]
fn verifies_dispatches_on_the_declared_profile() {
    let bytes = b"hello world";
    for cid in [
        *compute_bytes_cid(bytes).as_cid(),
        compute_raw_cid(bytes),
        compute_cid(bytes),
    ] {
        assert_eq!(verify_cid_any(&cid, bytes), Verify::Verified);
    }
    // A citation CID never verifies payload bytes under the byte profile, and vice versa:
    // equal inputs, different digests, no cross-profile equality.
    assert_ne!(
        compute_bytes_cid(bytes).as_cid().hash().digest(),
        compute_raw_cid(bytes).hash().digest()
    );
}
