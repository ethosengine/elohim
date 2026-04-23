//! Unit tests for /elohim/epr-atom/1.0.0 wire codec.
//!
//! These validate the transient wire types — no persistent source of truth
//! is exercised. Mirrors the discipline of `p2p/epr_protocol.rs` tests.

use elohim_storage::p2p::{EprAtomRequest, EprAtomResponse, MAX_BATCH_CIDS};

fn encode<V: serde::Serialize>(v: &V) -> Vec<u8> {
    let mut buf = Vec::new();
    ciborium::ser::into_writer(v, &mut buf).expect("encode");
    buf
}

fn decode<V: serde::de::DeserializeOwned>(bytes: &[u8]) -> V {
    ciborium::de::from_reader(bytes).expect("decode")
}

#[test]
fn request_fetch_roundtrip() {
    let r = EprAtomRequest::Fetch {
        cid: "bafkreibmzonpj42xk5vxltpl2h3mj5qnxmvprsnwkl3uml7yzhbpqu7c4a".into(),
    };
    let bytes = encode(&r);
    match decode::<EprAtomRequest>(&bytes) {
        EprAtomRequest::Fetch { cid } => assert!(cid.starts_with("bafk")),
        _ => panic!("wrong variant"),
    }
}

#[test]
fn request_announce_roundtrip() {
    let body = vec![0xA1, 0x63, 0x66, 0x6F, 0x6F];
    let r = EprAtomRequest::Announce {
        envelope_bytes: body.clone(),
    };
    let bytes = encode(&r);
    match decode::<EprAtomRequest>(&bytes) {
        EprAtomRequest::Announce { envelope_bytes } => assert_eq!(envelope_bytes, body),
        _ => panic!("wrong variant"),
    }
}

#[test]
fn request_fetch_batch_roundtrip() {
    let r = EprAtomRequest::FetchBatch {
        cids: vec!["a".into(), "b".into(), "c".into()],
    };
    let bytes = encode(&r);
    match decode::<EprAtomRequest>(&bytes) {
        EprAtomRequest::FetchBatch { cids } => assert_eq!(cids.len(), 3),
        _ => panic!("wrong variant"),
    }
}

#[test]
fn response_atom_roundtrip() {
    let body = vec![0x01, 0x02, 0x03];
    let r = EprAtomResponse::Atom {
        envelope_bytes: body.clone(),
    };
    let bytes = encode(&r);
    match decode::<EprAtomResponse>(&bytes) {
        EprAtomResponse::Atom { envelope_bytes } => assert_eq!(envelope_bytes, body),
        _ => panic!("wrong variant"),
    }
}

#[test]
fn response_atom_batch_preserves_none_slots() {
    let r = EprAtomResponse::AtomBatch {
        atoms: vec![Some(vec![0x01]), None, Some(vec![0x03])],
    };
    let bytes = encode(&r);
    match decode::<EprAtomResponse>(&bytes) {
        EprAtomResponse::AtomBatch { atoms } => {
            assert_eq!(atoms.len(), 3);
            assert_eq!(atoms[0].as_deref(), Some(&[0x01][..]));
            assert!(atoms[1].is_none());
            assert_eq!(atoms[2].as_deref(), Some(&[0x03][..]));
        }
        _ => panic!("wrong variant"),
    }
}

#[test]
fn response_announced_roundtrip() {
    let r = EprAtomResponse::Announced {
        accepted: false,
        reason: Some("signature verification failed".into()),
    };
    let bytes = encode(&r);
    match decode::<EprAtomResponse>(&bytes) {
        EprAtomResponse::Announced { accepted, reason } => {
            assert!(!accepted);
            assert_eq!(reason.as_deref(), Some("signature verification failed"));
        }
        _ => panic!("wrong variant"),
    }
}

#[test]
fn response_announced_accepted_no_reason_roundtrip() {
    let r = EprAtomResponse::Announced {
        accepted: true,
        reason: None,
    };
    let bytes = encode(&r);
    match decode::<EprAtomResponse>(&bytes) {
        EprAtomResponse::Announced { accepted, reason } => {
            assert!(accepted);
            assert!(reason.is_none());
        }
        _ => panic!("wrong variant"),
    }
}

#[test]
fn response_not_found_roundtrip() {
    let r = EprAtomResponse::NotFound;
    let bytes = encode(&r);
    assert!(matches!(
        decode::<EprAtomResponse>(&bytes),
        EprAtomResponse::NotFound
    ));
}

#[test]
fn response_error_roundtrip() {
    let r = EprAtomResponse::Error {
        message: "bad".into(),
    };
    let bytes = encode(&r);
    match decode::<EprAtomResponse>(&bytes) {
        EprAtomResponse::Error { message } => assert_eq!(message, "bad"),
        _ => panic!("wrong variant"),
    }
}

#[test]
fn batch_size_constant_matches_spec() {
    assert_eq!(MAX_BATCH_CIDS, 128);
}

/// Golden vector stability — if these change, the protocol version must bump.
/// The fixture describes the transient wire format only; no persistent source
/// of truth is involved.
#[test]
fn golden_vectors_stable() {
    let fixture_str = std::fs::read_to_string("tests/vectors/epr_atom_messages.json")
        .expect("fixture missing");
    let fixture: serde_json::Value = serde_json::from_str(&fixture_str).expect("fixture parse");
    let v = &fixture["vectors"];

    // Pairs of (Rust value, fixture key)
    let pairs: Vec<(Vec<u8>, &str)> = vec![
        (
            encode(&EprAtomRequest::Fetch { cid: "bafkreiabc".into() }),
            "request_fetch",
        ),
        (
            encode(&EprAtomRequest::Announce {
                envelope_bytes: vec![0x01, 0x02, 0x03],
            }),
            "request_announce",
        ),
        (
            encode(&EprAtomRequest::FetchBatch {
                cids: vec!["a".into(), "b".into()],
            }),
            "request_fetch_batch",
        ),
        (
            encode(&EprAtomResponse::Atom {
                envelope_bytes: vec![0x01, 0x02, 0x03],
            }),
            "response_atom",
        ),
        (
            encode(&EprAtomResponse::AtomBatch {
                atoms: vec![Some(vec![0x01]), None, Some(vec![0x03])],
            }),
            "response_atom_batch",
        ),
        (
            encode(&EprAtomResponse::Announced {
                accepted: true,
                reason: None,
            }),
            "response_announced_true",
        ),
        (
            encode(&EprAtomResponse::Announced {
                accepted: false,
                reason: Some("bad sig".into()),
            }),
            "response_announced_false",
        ),
        (encode(&EprAtomResponse::NotFound), "response_not_found"),
        (
            encode(&EprAtomResponse::Error {
                message: "batch too large".into(),
            }),
            "response_error",
        ),
    ];

    for (bytes, key) in pairs {
        let actual = hex::encode(&bytes);
        let expected = v[key]["cbor_hex"].as_str().unwrap_or_else(|| {
            panic!("fixture missing cbor_hex for {}", key)
        });
        assert_eq!(
            actual, expected,
            "{}: golden vector drift — if the protocol truly changed, bump the version in the fixture",
            key
        );
    }
}

/// Existing variants should be decodable after new ones are added.
/// Uses tag-based discrimination, so variant order in source doesn't matter.
#[test]
fn all_variants_roundtrip_byte_identical() {
    let requests: Vec<EprAtomRequest> = vec![
        EprAtomRequest::Fetch { cid: "bafkrei_a".into() },
        EprAtomRequest::Announce {
            envelope_bytes: vec![0x01, 0x02],
        },
        EprAtomRequest::FetchBatch {
            cids: vec!["x".into(), "y".into()],
        },
    ];
    for r in &requests {
        let bytes = encode(r);
        let decoded: EprAtomRequest = decode(&bytes);
        let re_encoded = encode(&decoded);
        assert_eq!(bytes, re_encoded, "roundtrip drifted: {:?}", r);
    }

    let responses: Vec<EprAtomResponse> = vec![
        EprAtomResponse::Atom {
            envelope_bytes: vec![0x01],
        },
        EprAtomResponse::AtomBatch {
            atoms: vec![Some(vec![0x01]), None],
        },
        EprAtomResponse::Announced {
            accepted: true,
            reason: None,
        },
        EprAtomResponse::NotFound,
        EprAtomResponse::Error {
            message: "err".into(),
        },
    ];
    for r in &responses {
        let bytes = encode(r);
        let decoded: EprAtomResponse = decode(&bytes);
        let re_encoded = encode(&decoded);
        assert_eq!(bytes, re_encoded, "roundtrip drifted: {:?}", r);
    }
}
