use elohim_epr::cid::compute_cid;

#[test]
fn cid_from_empty_cbor() {
    // Empty CBOR map: 0xa0
    let bytes = vec![0xa0u8];
    let cid = compute_cid(&bytes);
    let s = cid.to_string();
    // CIDv1 base32 + dag-cbor codec + sha256 multihash begins with "bafyrei"
    assert!(s.starts_with("bafyrei"), "got {s}");
}

#[test]
fn cid_stable_across_calls() {
    let bytes = vec![0x01, 0x02, 0x03, 0x04];
    assert_eq!(compute_cid(&bytes), compute_cid(&bytes));
}

#[test]
fn cid_differs_for_different_bytes() {
    let a = compute_cid(&[0x01]);
    let b = compute_cid(&[0x02]);
    assert_ne!(a, b);
}

#[test]
fn verify_cid_matches() {
    use elohim_epr::cid::verify_cid;
    let bytes = vec![0xaa, 0xbb, 0xcc];
    let cid = compute_cid(&bytes);
    assert!(verify_cid(&cid, &bytes));
    assert!(!verify_cid(&cid, &[0xaa, 0xbb]));
}
