//! Kernel properties that are not single vectors: the byte-CID prefix is fixed by the header,
//! the verifier fails closed on any profile it does not know, and the short fingerprint exists
//! only for sha2 digests.

use cid::Cid;
use elohim_address::{
    compute_bytes_cid, compute_raw_sha2_cid, short_fingerprint, verify_cid_any, AddressError,
    ByteCid, Verify, BYTE_CID_LEN, BYTE_CID_PREFIX, BYTE_CODEC, BYTE_DIGEST_LEN, BYTE_MULTIHASH,
};

/// xorshift64* — the same frozen generator the chunker vectors use; no `rand` in the lockfile.
fn xorshift(state: &mut u64) -> u64 {
    *state ^= *state >> 12;
    *state ^= *state << 25;
    *state ^= *state >> 27;
    state.wrapping_mul(0x2545_F491_4F6C_DD1D)
}

#[test]
fn byte_cid_prefix_is_header_determined() {
    let mut state = 0x454c_4f48_494d_u64;
    for i in 0..10_000usize {
        let len = (xorshift(&mut state) % 97) as usize + (i % 3);
        let input: Vec<u8> = (0..len).map(|_| xorshift(&mut state) as u8).collect();
        let cid = compute_bytes_cid(&input);
        let text = cid.to_string();
        assert!(text.starts_with(BYTE_CID_PREFIX), "{text}");
        assert_eq!(text.len(), BYTE_CID_LEN);
        assert_eq!(ByteCid::parse(&text).unwrap(), cid);
    }
}

#[test]
fn verifies_dispatches_on_codec_and_multihash() {
    let bytes = b"elohim";
    let byte = compute_bytes_cid(bytes);
    assert_eq!(byte.as_cid().codec(), BYTE_CODEC);
    assert_eq!(byte.as_cid().hash().code(), BYTE_MULTIHASH);
    assert_eq!(byte.as_cid().hash().size() as usize, BYTE_DIGEST_LEN);
    // A sha2 CID carrying the blake3 digest (same length, wrong algorithm) never verifies.
    let mislabelled = Cid::new_v1(
        0x55,
        cid::multihash::Multihash::<64>::wrap(0x12, byte.digest()).unwrap(),
    );
    assert_eq!(verify_cid_any(&mislabelled, bytes), Verify::Mismatch);
    assert_eq!(verify_cid_any(byte.as_cid(), bytes), Verify::Verified);
}

#[test]
fn verifies_fails_closed_on_unknown_pair() {
    let digest = [7u8; 32];
    for (codec, code, len) in [
        (0x70u64, 0x1eu64, 32usize), // dag-pb over blake3
        (0x71, 0x1e, 32),            // dag-cbor over blake3: not a profile this epoch
        (0x55, 0x13, 64),            // sha2-512
        (0x55, 0x1e, 16),            // truncated blake3
        (0x55, 0xb220, 32),          // blake2b-256
    ] {
        let mh = cid::multihash::Multihash::<64>::wrap(code, &vec![7u8; len]).unwrap();
        let claimed = Cid::new_v1(codec, mh);
        assert_eq!(
            verify_cid_any(&claimed, &digest),
            Verify::UnsupportedProfile {
                codec,
                multihash: code,
                len
            }
        );
    }
    // CIDv0 is dag-pb sha2: never a profile here, even with a matching digest.
    let v0 = Cid::new_v0(cid::multihash::Multihash::<64>::wrap(0x12, &[0u8; 32]).unwrap()).unwrap();
    assert!(matches!(
        verify_cid_any(&v0, b""),
        Verify::UnsupportedProfile { codec: 0x70, .. }
    ));
}

#[test]
fn short_fingerprint_refuses_non_sha2_digests() {
    let byte = compute_bytes_cid(b"abc");
    assert!(matches!(
        short_fingerprint(byte.as_cid()),
        Err(AddressError::NotSha2 { multihash: 0x1e })
    ));
    // The cite fingerprint of the empty body, pinned against sha256sum.
    assert_eq!(
        short_fingerprint(&compute_raw_sha2_cid(b"")).unwrap(),
        "sha256:e3b0c44298fc1c14"
    );
}

#[test]
fn display_and_from_str_round_trip() {
    let cid = compute_bytes_cid(b"round trip");
    let text = cid.to_string();
    assert_eq!(text.parse::<ByteCid>().unwrap(), cid);
    assert_eq!(ByteCid::from_digest(*cid.digest()), cid);
}
