use cid::Cid;
use elohim_epr::Signature;

fn test_cid(b: u8) -> Cid {
    elohim_epr::cid::compute_cid(&[b])
}

#[test]
fn signature_constructs() {
    let signer = test_cid(99);
    let sig = Signature::ed25519(signer, vec![0u8; 64]);
    assert_eq!(sig.signer, test_cid(99));
    assert_eq!(sig.algorithm, "ed25519");
    assert_eq!(sig.signature.len(), 64);
}

#[test]
fn signature_rejects_wrong_length() {
    let signer = test_cid(99);
    // Ed25519 signatures are exactly 64 bytes
    assert!(Signature::ed25519_checked(signer, vec![0u8; 63]).is_err());
    assert!(Signature::ed25519_checked(test_cid(99), vec![0u8; 65]).is_err());
    assert!(Signature::ed25519_checked(test_cid(99), vec![0u8; 64]).is_ok());
}
