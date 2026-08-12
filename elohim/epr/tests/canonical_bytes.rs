use chrono::{TimeZone, Utc};
use cid::Cid;
use elohim_epr::measure::*;
use elohim_epr::{cbor, cid::compute_cid, Coupling, Envelope, EprKind, Reach, Signature};

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
    assert_eq!(
        ba, bb,
        "cid/proof/supersededBy must not affect canonical bytes"
    );
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
        hex_encoded, EXPECTED_HEX,
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
        (Reach::Private, "\"reach\":\"private\""),
        (Reach::SelfScope, "\"reach\":\"self\""),
        (Reach::Intimate, "\"reach\":\"intimate\""),
        (Reach::Trusted, "\"reach\":\"trusted\""),
        (Reach::Familiar, "\"reach\":\"familiar\""),
        (Reach::Community, "\"reach\":\"community\""),
        (Reach::Public, "\"reach\":\"public\""),
        (Reach::Commons, "\"reach\":\"commons\""),
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

// --- L2: confidence rides inside the canonical bytes (dag-cbor, not JSON) ---
//
// L2 is a claim about the content HASH: `cid.rs::compute_cid` hashes exactly
// the dag-cbor bytes `cbor.rs` produces, so the assertion must run over that
// path. A JSON assertion would prove nothing about the address itself — JSON
// is not this crate's canonical-bytes contract.
//
// This does NOT exercise a typed `Quantity -> canonical bytes` entry point,
// because none exists yet in this crate's own API (`cbor::encode` takes only
// `&Ipld`, and every other canonical-bytes producer — `envelope.rs`,
// `witness.rs` — hand-builds a `BTreeMap<String, Ipld>` first). What this
// proves is narrower and still load-bearing: `serde_ipld_dagcbor`'s typed
// serializer, invoked directly on `Quantity`, produces bytes that are
// independently canonical by the crate's own `cbor::decode_strict` check —
// and confidence is inside them. Wiring a typed entry point remains open
// (see the spec's "What slice 2 inherits" section).

fn quantity(confidence: Confidence) -> Quantity {
    Quantity {
        value: 12.0,
        kind: MeasureKind::Level,
        confidence,
    }
}

#[test]
fn quantity_canonical_bytes_are_dag_cbor_and_pass_the_crates_own_canonicality_check() {
    let q = quantity(Confidence::estimated(
        Interval::new(8.0, 16.0),
        "3-week sample",
    ));
    let bytes = serde_ipld_dagcbor::to_vec(&q).expect("Quantity encodes to dag-cbor");

    // decode_strict re-encodes the decoded Ipld and requires a byte-identical
    // match against the input — this is the crate's own definition of
    // "canonical." If the typed serializer ever drifted from that definition
    // (e.g. producing non-shortest-form integers or unsorted map keys), this
    // is where it would be caught.
    cbor::decode_strict(&bytes).expect("typed Quantity encoding is canonical dag-cbor");
}

#[test]
fn quantity_canonical_bytes_round_trip_through_dag_cbor() {
    let q = quantity(Confidence::witnessed(
        Interval::exact(12.0),
        "wc -c on disk",
    ));
    let bytes = serde_ipld_dagcbor::to_vec(&q).unwrap();
    let round_tripped: Quantity = serde_ipld_dagcbor::from_slice(&bytes).unwrap();
    assert_eq!(round_tripped, q, "dag-cbor round trip must be lossless");
}

#[test]
fn confidence_is_inside_the_canonical_bytes() {
    // L2's real property: if two quantities differ ONLY in their confidence
    // interval, their CANONICAL (dag-cbor, content-addressed) bytes must
    // differ. If they collided, the interval would be a detachable sibling —
    // narrowable after the fact without moving the quantity's own CID. Treat
    // a failure here as a design bug in the ontology, not a test bug.
    let wide = quantity(Confidence::estimated(
        Interval::new(8.0, 16.0),
        "3-week sample",
    ));
    let narrow = quantity(Confidence::estimated(
        Interval::new(11.0, 13.0),
        "3-week sample",
    ));

    let wide_bytes = serde_ipld_dagcbor::to_vec(&wide).unwrap();
    let narrow_bytes = serde_ipld_dagcbor::to_vec(&narrow).unwrap();

    // Both sides of the comparison must themselves be canonical, or the
    // inequality below would prove nothing about the address space L2
    // actually governs.
    cbor::decode_strict(&wide_bytes).expect("wide-interval encoding is canonical");
    cbor::decode_strict(&narrow_bytes).expect("narrow-interval encoding is canonical");

    assert_ne!(
        wide_bytes, narrow_bytes,
        "confidence must be inside the canonical bytes, never a detachable sibling"
    );

    // And therefore the CIDs the crate would mint over these bytes differ too
    // — this is the property L2 is actually named for.
    assert_ne!(
        compute_cid(&wide_bytes),
        compute_cid(&narrow_bytes),
        "quantities differing only in confidence must mint different CIDs"
    );
}

#[test]
fn adding_an_unknown_reason_moves_no_existing_address() {
    // Q17's hard constraint, and the reason `UnknownReason` does NOT live on `Interval`.
    //
    // `Interval` is hash-bearing: it rides inside `Confidence` inside `Quantity`, and the two
    // tests above are the crate's own proof that those bytes ARE the address. A third field on
    // `Interval` — or a non-skipped field on `Confidence` — would appear in the canonical
    // dag-cbor map of every quantity ever encoded and silently re-address all of them. So the
    // reason rides in the transient `Reasoned` pair through the arithmetic, and reaches the wire
    // only as an `Option` on `Confidence` that is skipped when `None`.
    //
    // The vector below was captured from this exact expression on the commit BEFORE
    // `unknown_reason` existed. It is not a self-consistency check — a regenerated golden would
    // prove nothing — it is the pre-change byte string, and it must still match.
    const PRE_Q17_HEX: &str = "a3646b696e64656c6576656c6576616c7565fb40280000000000006a63\
6f6e666964656e6365a36562617369736d332d7765656b2073616d706c6565636c61696d69657374696d61746564\
68696e74657276616ca2626869fb4030000000000000626c6ffb4020000000000000";
    const PRE_Q17_CID: &str = "bafyreiag43a2arlyomssxfakh3nmqzjewtcifqk6rnfzisnailr4i7bdza";

    let q = quantity(Confidence::estimated(
        Interval::new(8.0, 16.0),
        "3-week sample",
    ));
    let bytes = serde_ipld_dagcbor::to_vec(&q).unwrap();
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(
        hex, PRE_Q17_HEX,
        "adding UnknownReason changed the canonical bytes of a quantity that carries none"
    );
    assert_eq!(compute_cid(&bytes).to_string(), PRE_Q17_CID);

    // And the converse, which is what makes the field worth putting on the wire at all: when a
    // reason IS present it lands inside the canonical bytes, keeps them canonical, and moves the
    // address — because it is strictly more of a claim. That is L2 working as designed.
    //
    // Built as a struct literal on purpose. It is NOT a valid domain value — every constructor
    // upholds "a reason only accompanies an unknown interval" — but it is the only way to
    // exercise the CODEC, because an unknown interval cannot be dag-cbor encoded at all today
    // (see `an_unknown_interval_is_not_encodable_on_either_wire_and_that_predates_q17`). Treat
    // this fixture as a serializer probe, never as an example.
    let with_reason = quantity(Confidence {
        claim: ClaimKind::Estimated,
        interval: Interval::new(8.0, 16.0),
        basis: "3-week sample".into(),
        unknown_reason: Some(UnknownReason::Incommensurable),
    });
    let reasoned_bytes = serde_ipld_dagcbor::to_vec(&with_reason).unwrap();
    cbor::decode_strict(&reasoned_bytes).expect("still canonical dag-cbor with the field present");
    assert_ne!(
        bytes, reasoned_bytes,
        "a stated reason is part of the claim and must be part of the address"
    );
}

#[test]
fn an_unknown_interval_is_not_encodable_on_either_wire_and_that_predates_q17() {
    // Found while wiring Q17, recorded rather than fixed because it is not Q17's defect and the
    // fix is a codec decision this change has no standing to make.
    //
    // dag-cbor (IPLD) admits no non-finite float, so `Interval::unknown()`'s `(-inf, +inf)` —
    // L3's honest absence, the shape the whole ontology routes absence through — cannot be
    // encoded at all. `serde_json` differs but is no better: it emits `null` for a non-finite
    // float and then refuses to read it back.
    //
    // The consequence for Q17 is worth stating plainly: `unknown_reason` is real in process and
    // at the type level, but under the CURRENT codec it can never actually appear in a
    // quantity's canonical bytes, because the only intervals it may accompany are the ones that
    // do not encode. The wire placement above is therefore correct-by-construction and inert
    // until the absence-encoding question is answered (a sentinel? a tagged variant? an
    // `Option<Interval>` at the boundary only?). Nothing here should be read as claiming a
    // working unknown-quantity wire format.
    let unknown = quantity(Confidence::unknown_because(
        UnknownReason::NoObservations,
        "empty window",
    ));
    assert!(
        serde_ipld_dagcbor::to_vec(&unknown).is_err(),
        "if this starts passing, the codec learned non-finite floats — revisit the note above"
    );
    let json = serde_json::to_string(&unknown).unwrap();
    assert!(
        json.contains("null"),
        "non-finite bounds degrade to null: {json}"
    );
    assert!(
        serde_json::from_str::<Quantity>(&json).is_err(),
        "and null does not read back as an f64"
    );
    // The reason itself is perfectly encodable — it is the interval that is not.
    assert_eq!(
        serde_json::to_string(&UnknownReason::NoObservations).unwrap(),
        "\"no-observations\""
    );
}
