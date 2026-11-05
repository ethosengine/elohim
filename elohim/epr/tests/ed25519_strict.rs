//! Parity with the TypeScript verifier on Ed25519's edge cases: `tests/vectors/ed25519_strict.json`
//! (derived by `elohim/sdk/epr-ts/tests/strict-vectors.ts`) holds small-order, mixed-torsion and
//! non-canonical signatures. ed25519-dalek's `verify_strict` — and so `SignatureEnvelope::check`
//! under scheme 0 — must answer each exactly as the vector says; the epr-ts `verifyEd25519` answers
//! the same file the same way, so the two languages admit one set of signatures.

use ed25519_dalek::{Signature as EdSig, VerifyingKey};
use elohim_epr::signature::{
    signing_transcript, Multikey, SignatureCheck, SignatureEnvelope, ED25519_SIGNATURE_LEN,
};
use serde_json::Value;

const DOMAIN: &[u8] = b"elohim:test:v0:";

fn unhex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
        .collect()
}

fn vectors() -> Vec<Value> {
    serde_json::from_str::<Value>(include_str!("vectors/ed25519_strict.json"))
        .unwrap()
        .as_array()
        .unwrap()
        .clone()
}

#[test]
fn verify_strict_answers_every_shared_vector() {
    let vectors = vectors();
    assert_eq!(vectors.len(), 8);
    for v in &vectors {
        let label = v["label"].as_str().unwrap();
        let accept = match v["strict"].as_str().unwrap() {
            "accept" => true,
            "reject" => false,
            other => panic!("{label}: unknown expectation {other}"),
        };
        let pk: [u8; 32] = unhex(v["public_key_hex"].as_str().unwrap())
            .try_into()
            .unwrap();
        let body = unhex(v["body_hex"].as_str().unwrap());
        let sig: [u8; ED25519_SIGNATURE_LEN] = unhex(v["signature_hex"].as_str().unwrap())
            .try_into()
            .unwrap();

        // The transcript the vector signs is this crate's transcript v1.
        let transcript =
            signing_transcript(DOMAIN, 0, 0, Multikey::ed25519(&pk).as_bytes(), &body).unwrap();
        assert_eq!(
            transcript,
            unhex(v["transcript_hex"].as_str().unwrap()),
            "{label}"
        );

        // ed25519-dalek directly …
        let strict = VerifyingKey::from_bytes(&pk)
            .map(|vk| {
                vk.verify_strict(&transcript, &EdSig::from_bytes(&sig))
                    .is_ok()
            })
            .unwrap_or(false);
        assert_eq!(strict, accept, "{label}: verify_strict");

        // … and the envelope every signed object carries.
        let check = SignatureEnvelope::ed25519(&pk, sig).check(DOMAIN, 0, &body);
        let expected = if accept {
            SignatureCheck::Verified
        } else {
            SignatureCheck::Invalid
        };
        assert_eq!(check, expected, "{label}: SignatureEnvelope::check");
    }
}

/// The review's vector by name: `A = B`, `R = (0, −1)` (`ecff…7f`), `S = SHA512(R ‖ A ‖ M) mod L`.
/// A cofactored verifier admits it; `verify_strict` must not.
#[test]
fn small_order_r_review_vector_is_rejected() {
    let v = vectors()
        .into_iter()
        .find(|v| v["label"] == "small-order-r-cofactored-only")
        .unwrap();
    assert_eq!(
        &v["signature_hex"].as_str().unwrap()[..64],
        "ecffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f"
    );
    assert_eq!(
        v["public_key_hex"].as_str().unwrap(),
        "5866666666666666666666666666666666666666666666666666666666666666",
        "A is the standard base point"
    );
    assert_eq!(v["strict"], "reject");
}
