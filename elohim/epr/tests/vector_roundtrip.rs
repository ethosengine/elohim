use elohim_epr::Epr;
use std::fs;
use std::path::PathBuf;

#[test]
fn all_vectors_verify_under_their_public_key() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/vectors/signed_eprs.json");
    let raw = fs::read_to_string(&path)
        .expect("run: cargo run -p elohim-epr --example gen_vectors first");
    let vectors: Vec<serde_json::Value> = serde_json::from_str(&raw).unwrap();

    for v in &vectors {
        let envelope = serde_json::from_value(v["envelope"].clone()).unwrap();
        let payload_hex = v["payload_hex"].as_str().unwrap();
        let payload = hex::decode(payload_hex).unwrap();
        let pk_hex = v["public_key_hex"].as_str().unwrap();
        let pk_bytes = hex::decode(pk_hex).unwrap();
        let mut pk = [0u8; 32];
        pk.copy_from_slice(&pk_bytes);

        let epr = Epr { envelope, payload };
        epr.verify_with_key(&pk).expect("vector must verify");
    }
}
