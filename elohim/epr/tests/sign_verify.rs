use elohim_epr::proof::{sign, verify, AgentKeypair};

#[test]
fn keypair_generates_different_keys() {
    let mut rng = rand::thread_rng();
    let kp1 = AgentKeypair::generate(&mut rng);
    let kp2 = AgentKeypair::generate(&mut rng);
    assert_ne!(kp1.public_key_bytes(), kp2.public_key_bytes());
}

#[test]
fn sign_and_verify_roundtrip() {
    let mut rng = rand::thread_rng();
    let kp = AgentKeypair::generate(&mut rng);
    let message = b"the quick brown fox";
    let sig = sign(&kp, message);
    assert!(verify(&kp.public_key_bytes(), message, &sig));
}

#[test]
fn verify_rejects_tampered_message() {
    let mut rng = rand::thread_rng();
    let kp = AgentKeypair::generate(&mut rng);
    let sig = sign(&kp, b"original");
    assert!(!verify(&kp.public_key_bytes(), b"tampered", &sig));
}

#[test]
fn verify_rejects_wrong_key() {
    let mut rng = rand::thread_rng();
    let kp1 = AgentKeypair::generate(&mut rng);
    let kp2 = AgentKeypair::generate(&mut rng);
    let sig = sign(&kp1, b"message");
    assert!(!verify(&kp2.public_key_bytes(), b"message", &sig));
}

#[test]
fn rfc8032_test_vector_1() {
    // RFC 8032 §7.1 Test 1 — the canonical test vector.
    let secret_hex = "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";
    let public_hex = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";
    let msg: &[u8] = b"";
    let expected_sig_hex = concat!(
        "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e06522490155",
        "5fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b"
    );

    let secret = hex::decode(secret_hex).unwrap();
    let kp = AgentKeypair::from_secret(&secret).unwrap();
    assert_eq!(hex::encode(kp.public_key_bytes()), public_hex);
    let sig = sign(&kp, msg);
    assert_eq!(hex::encode(&sig), expected_sig_hex);
    assert!(verify(&kp.public_key_bytes(), msg, &sig));
}
