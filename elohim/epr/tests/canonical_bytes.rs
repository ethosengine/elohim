use chrono::{TimeZone, Utc};
use cid::Cid;
use elohim_epr::{cid::compute_cid, Coupling, Envelope, EprKind, Reach, Signature};

fn test_cid(b: u8) -> Cid {
    compute_cid(&[b])
}

fn env() -> Envelope {
    Envelope {
        cid: test_cid(0),
        kind: EprKind::Content,
        schema_ref: test_cid(1),
        schema_key: "concept".into(),
        reach: Reach::Commons,
        coupling: Coupling {
            knowledge: Some(test_cid(2)),
            value: Some(test_cid(3)),
            governance: Some(test_cid(4)),
        },
        claims: vec![test_cid(5)],
        supersedes: None,
        superseded_by: None,
        issued_at: Utc.with_ymd_and_hms(2026, 4, 21, 12, 0, 0).unwrap(),
        proof: Signature::ed25519(test_cid(6), vec![0u8; 64]),
    }
}

#[test]
fn canonical_bytes_excludes_cid_proof_superseded_by() {
    let a = env();
    let mut b = env();

    // Different cid, proof, superseded_by — canonical bytes must still match
    b.cid = test_cid(42);
    b.proof = Signature::ed25519(test_cid(7), vec![1u8; 64]);
    b.superseded_by = Some(test_cid(99));

    let payload = b"hello";
    let ba = a.canonical_bytes(payload).unwrap();
    let bb = b.canonical_bytes(payload).unwrap();
    assert_eq!(ba, bb, "cid/proof/supersededBy must not affect canonical bytes");
}

#[test]
fn canonical_bytes_changes_when_schema_key_changes() {
    let a = env();
    let mut b = env();
    b.schema_key = "lesson".into();

    let payload = b"hello";
    assert_ne!(
        a.canonical_bytes(payload).unwrap(),
        b.canonical_bytes(payload).unwrap()
    );
}

#[test]
fn canonical_bytes_changes_when_payload_changes() {
    let a = env();
    assert_ne!(
        a.canonical_bytes(b"foo").unwrap(),
        a.canonical_bytes(b"bar").unwrap()
    );
}

#[test]
fn canonical_bytes_includes_supersedes_when_present() {
    let mut a = env();
    a.supersedes = Some(test_cid(77));
    let b = env(); // supersedes: None

    // With vs without supersedes must differ
    assert_ne!(
        a.canonical_bytes(b"hello").unwrap(),
        b.canonical_bytes(b"hello").unwrap()
    );
}

#[test]
fn canonical_bytes_deterministic() {
    let a = env();
    let ba = a.canonical_bytes(b"stable").unwrap();
    let bb = a.canonical_bytes(b"stable").unwrap();
    assert_eq!(ba, bb);
}

#[test]
fn canonical_bytes_golden_vector() {
    // Golden vector: this exact fixture + payload MUST produce these exact bytes.
    // If a library upgrade changes the encoding, this test fails loudly instead of
    // letting Rust↔TS interop break silently at verify time.
    //
    // To regenerate (after a deliberate, approved wire-format change):
    //   1. Run `cd elohim && cargo test -p elohim-epr --test canonical_bytes golden -- --nocapture`
    //   2. Confirm the printed hex matches what TS also produces (interop re-verify)
    //   3. Update the EXPECTED_HEX constant below
    //   4. Bump the crate's wire-format version
    let e = env();
    let bytes = e.canonical_bytes(b"golden").unwrap();
    let hex_encoded = hex::encode(&bytes);
    // Uncomment during regeneration:
    // println!("canonical bytes = {hex_encoded}");

    // This value was captured on initial generation. If it changes, interop breaks.
    // See `elohim/sdk/epr-ts/tests/interop.test.ts` for the TS-side counterpart.
    // First time running: expect this to fail — capture the actual hex from the
    // assertion output, verify it structurally (manual CBOR decode), and paste it here.
    const EXPECTED_HEX: &str = "a8646b696e6467436f6e74656e7465726561636867636f6d6d6f6e7366636c61696d7381d82a58250001711220e77b9a9ae9e30b0dbdb6f510a264ef9de781501d7b6b92ae89eb059c5ab743db677061796c6f616446676f6c64656e68636f75706c696e67a36576616c7565d82a58250001711220084fed08b978af4d7d196a7446a86b58009e636b611db16211b65a9aadff29c5696b6e6f776c65646765d82a58250001711220dbc1b4c900ffe48d575b5da5c638040125f65db0fe3e24494b76ea986457d9866a676f7665726e616e6365d82a58250001711220e52d9c508c502347344d8c07ad91cbd6068afc75ff6292f062a09ca381c89e7168697373756564417474323032362d30342d32315431323a30303a30305a69736368656d614b657967636f6e6365707469736368656d61526566d82a582500017112204bf5122f344554c53bde2ebb8cd2b7e3d1600ad631c385a5d7cce23c7785459a";
    if EXPECTED_HEX == "__PLACEHOLDER__" {
        // Bootstrap mode: print the actual value so it can be pasted in.
        panic!(
            "golden vector placeholder — actual hex is:\n{hex_encoded}\n\nPaste this into EXPECTED_HEX and re-run."
        );
    }
    assert_eq!(
        hex_encoded,
        EXPECTED_HEX,
        "canonical_bytes encoding drifted — investigate before updating EXPECTED_HEX"
    );
}

#[test]
fn kind_canonical_strings_for_all_variants() {
    // Every EprKind variant must serialize to its exact canonical string.
    // Copy-paste errors (e.g., "Observation" -> "Obsrevation") compile fine but
    // break TS interop — this test catches them.
    let mut e = env();
    for (kind, expected_substring) in [
        (EprKind::Content, "\"kind\":\"Content\""),
        (EprKind::Agent, "\"kind\":\"Agent\""),
        (EprKind::Manifest, "\"kind\":\"Manifest\""),
        (EprKind::Claim, "\"kind\":\"Claim\""),
        (EprKind::Observation, "\"kind\":\"Observation\""),
        (EprKind::EconomicEvent, "\"kind\":\"EconomicEvent\""),
        (EprKind::Commitment, "\"kind\":\"Commitment\""),
        (EprKind::Attestation, "\"kind\":\"Attestation\""),
        (EprKind::Delegation, "\"kind\":\"Delegation\""),
    ] {
        e.kind = kind;
        // We test via JSON round-trip of the envelope — serde uses the canonical
        // pascal case for EprKind (no rename_all).
        let json = serde_json::to_string(&e).unwrap();
        assert!(
            json.contains(expected_substring),
            "kind {:?} did not produce expected substring {}: got {}",
            kind,
            expected_substring,
            json
        );
    }
}

#[test]
fn reach_canonical_strings_for_all_variants() {
    let mut e = env();
    for (reach, expected_substring) in [
        (Reach::Commons, "\"reach\":\"commons\""),
        (Reach::Community, "\"reach\":\"community\""),
        (Reach::Collective, "\"reach\":\"collective\""),
        (Reach::Steward, "\"reach\":\"steward\""),
        (Reach::Private, "\"reach\":\"private\""),
    ] {
        e.reach = reach;
        let json = serde_json::to_string(&e).unwrap();
        assert!(
            json.contains(expected_substring),
            "reach {:?} did not produce expected substring {}: got {}",
            reach,
            expected_substring,
            json
        );
    }
}
