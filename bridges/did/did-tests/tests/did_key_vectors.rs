//! `did:key` codec vectors and offline resolution.
//!
//! Vectors are of two kinds:
//!
//! 1. **Real Holochain agent keys** — the reverse path recomputes the 4 DHT
//!    location bytes; a byte-identical round-trip against a real key proves our
//!    blake2b loc algorithm matches the `holo_hash` reference implementation.
//!    The first key is `holo_hash`'s own reference test vector; every key here
//!    is a valid agent-key encoding (prefix `0x842024`, blake2b-128 XOR-fold
//!    loc) whose stored loc our algorithm reproduces exactly.
//!
//! 2. **A did:key vector built from a published ed25519 public key** (RFC 8032
//!    §7.1 Test 1 public key). The expected `z6Mk…` string is *constructed from
//!    first principles* here — multicodec `0xed01` + base58btc — rather than
//!    copied from the did:key spec's own suite, and is asserted self-consistent
//!    (encode == fixture, and decode recovers the exact pubkey bytes).

use did_bridge::{
    agent_cid_to_core32, agent_cid_to_did_key, core32_to_did_key, did_key_to_agent_cid,
    did_key_to_core32, DidKeyResolver, DidResolver,
};
use did_types::{Did, VerificationRelationship};

/// (agent_cid, expected did:key) — real Holochain agent keys.
const DID_KEY_VECTORS: &[(&str, &str)] = &[
    (
        "uhCAk39SDf7rynCg5bYgzroGaOJKGKrloI1o57Xao6S-U5KNZ0dUH",
        "did:key:z6MkuWzukKSaEVxe76gbFYrnW7jUUftksarjkrjUwKdEp8Lr",
    ),
    (
        "uhCAkwfTgZ5eDJwI6ZV5vGt-kg8cVgXvcf35XKj6HnMv4PBH8noYB",
        "did:key:z6MksWPJRcZMjF6dzHt7XUp8FsN7GbV6gRmtRtAGRfaijs4U",
    ),
];

/// A larger set of real agent keys for the loc round-trip (agent_cid only).
const AGENT_KEYS: &[&str] = &[
    "uhCAk39SDf7rynCg5bYgzroGaOJKGKrloI1o57Xao6S-U5KNZ0dUH",
    "uhCAk71wNXTv7lstvi4PfUr_JDvxLucF9WzUgWPNIEZIoPGMF4b_o",
    "uhCAkJ9p-IlfMpeP_HeygQt2jqHDXu4-YRAQezq3L0m9nz3wCa0Mh",
    "uhCAkJCuynkgVdMn_bzZ2ZYaVfygkn0WCuzfFspczxFnZM1QAyXoo",
    "uhCAkQHMlYam1PRiYJCzAwQ0AUxIMwOoOvxgXS67N_YPOMj-fGx6X",
    "uhCAkwfTgZ5eDJwI6ZV5vGt-kg8cVgXvcf35XKj6HnMv4PBH8noYB",
];

#[test]
fn did_key_vectors_encode_exactly() {
    for (agent_cid, did_key) in DID_KEY_VECTORS {
        assert_eq!(&agent_cid_to_did_key(agent_cid).unwrap(), did_key);
        assert_eq!(&did_key_to_agent_cid(did_key).unwrap(), agent_cid);
    }
}

#[test]
fn agent_keys_loc_recompute_byte_identical() {
    // Reconstructing the agent_cid from its core (recomputing loc) must equal the
    // real key — proves the loc algorithm matches the holo_hash reference.
    for agent_cid in AGENT_KEYS {
        let core = agent_cid_to_core32(agent_cid).unwrap();
        assert_eq!(
            &did_key_to_agent_cid(&core32_to_did_key(&core)).unwrap(),
            agent_cid,
            "loc recompute diverged for {agent_cid}"
        );
    }
}

#[test]
fn rfc8032_pubkey_did_key_is_self_consistent() {
    // RFC 8032 §7.1 Test 1 public key.
    let pubkey = hex_to_32("d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a");
    // Constructed from first principles (multicodec 0xed01 + base58btc).
    const EXPECTED: &str = "did:key:z6MktwupdmLXVVqTzCw4i46r4uGyosGXRnR3XjN4Zq7oMMsw";
    assert_eq!(core32_to_did_key(&pubkey), EXPECTED);
    // Decoding recovers the exact published pubkey bytes.
    assert_eq!(did_key_to_core32(EXPECTED).unwrap(), pubkey);
}

#[tokio::test]
async fn did_key_resolves_offline_with_five_relationships() {
    let resolver = DidKeyResolver::new();
    let did = Did::parse("did:key:z6MkuWzukKSaEVxe76gbFYrnW7jUUftksarjkrjUwKdEp8Lr").unwrap();
    let result = resolver.resolve(&did).await.unwrap();
    let doc = result.did_document.unwrap();

    assert_eq!(doc.verification_method.as_ref().unwrap().len(), 1);
    let refs = [
        &doc.authentication,
        &doc.assertion_method,
        &doc.key_agreement,
        &doc.capability_invocation,
        &doc.capability_delegation,
    ];
    for r in refs {
        let entries = r.as_ref().expect("relationship present");
        assert!(matches!(entries[0], VerificationRelationship::Reference(_)));
    }

    // Assembled documents carry the DID 1.1 context.
    let json = serde_json::to_value(&doc).unwrap();
    assert_eq!(json["@context"][0], "https://www.w3.org/ns/did/v1.1");
}

fn hex_to_32(s: &str) -> [u8; 32] {
    let bytes: Vec<u8> = (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect();
    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes);
    out
}
